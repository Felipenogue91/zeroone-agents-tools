//! Reading the source of truth (`~/.agents` layout + project files).
//!
//! Read-only. Everything is optional: a missing file means "empty", never an
//! error, so users without `~/.agents` get an empty model.

use super::model::*;
use crate::utils::agents_home::AgentsHome;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub fn load_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("[sync] could not parse {}: {}", path.display(), e);
            None
        }
    }
}

/// Expand a leading `~` with the given home (cross-platform).
pub fn expand_home(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\")) {
        return home.join(rest);
    }
    PathBuf::from(path)
}

/// Inverse of [`expand_home`]: store paths under home as `~/...` so
/// `sync.json` stays portable between machines/users.
pub fn contract_home(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.to_string_lossy().replace('\\', "/")),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

/// `per-CLI field overrides`: overrides[scope][cli][server] = fields.
pub type Overrides = BTreeMap<String, BTreeMap<Cli, BTreeMap<String, Map<String, Value>>>>;

/// Parsed `~/.agents/mcp/sync.json`. `raw` keeps the whole document (docs,
/// unknown keys) so it can be written back without losing anything.
#[derive(Debug, Clone, Default)]
pub struct SyncConfig {
    pub projects: Vec<String>,
    pub plugin_provided: BTreeMap<Cli, BTreeSet<String>>,
    pub codex_global_keep: BTreeSet<String>,
    pub overrides: Overrides,
    /// `cliOff[scope][cli] = [names]`: turned off in one CLI only (scope =
    /// "global" or the project folder name). App extension to the format.
    pub cli_off: BTreeMap<String, BTreeMap<Cli, BTreeSet<String>>>,
    pub raw: Value,
}

fn is_doc_key(k: &str) -> bool {
    k.starts_with('_')
}

fn string_list(v: Option<&Value>) -> Vec<String> {
    v.and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

impl SyncConfig {
    pub fn parse(raw: Value) -> Self {
        let mut cfg = SyncConfig {
            projects: string_list(raw.get("projects")),
            codex_global_keep: string_list(raw.get("codexGlobalKeep"))
                .into_iter()
                .collect(),
            ..Default::default()
        };
        if let Some(pp) = raw.get("pluginProvided").and_then(|v| v.as_object()) {
            for (k, v) in pp {
                if is_doc_key(k) {
                    continue;
                }
                if let Some(cli) = Cli::from_key(k) {
                    cfg.plugin_provided
                        .insert(cli, string_list(Some(v)).into_iter().collect());
                }
            }
        }
        if let Some(ov) = raw.get("overrides").and_then(|v| v.as_object()) {
            for (scope, clis) in ov {
                if is_doc_key(scope) {
                    continue;
                }
                let Some(clis) = clis.as_object() else {
                    continue;
                };
                let mut per_cli = BTreeMap::new();
                for (cli_key, servers) in clis {
                    let (Some(cli), Some(servers)) = (Cli::from_key(cli_key), servers.as_object())
                    else {
                        continue;
                    };
                    let mut m = BTreeMap::new();
                    for (server, fields) in servers {
                        if is_doc_key(server) {
                            continue;
                        }
                        if let Some(f) = fields.as_object() {
                            m.insert(server.clone(), f.clone());
                        }
                    }
                    per_cli.insert(cli, m);
                }
                cfg.overrides.insert(scope.clone(), per_cli);
            }
        }
        if let Some(off) = raw.get("cliOff").and_then(|v| v.as_object()) {
            for (scope, clis) in off {
                if is_doc_key(scope) {
                    continue;
                }
                let mut per = BTreeMap::new();
                for (k, names) in clis.as_object().into_iter().flatten() {
                    if let Some(cli) = Cli::from_key(k) {
                        per.insert(cli, string_list(Some(names)).into_iter().collect());
                    }
                }
                cfg.cli_off.insert(scope.clone(), per);
            }
        }
        cfg.raw = raw;
        cfg
    }

    pub fn is_cli_off(&self, scope_name: &str, cli: Cli, name: &str) -> bool {
        self.cli_off
            .get(scope_name)
            .and_then(|c| c.get(&cli))
            .map(|s| s.contains(name))
            .unwrap_or(false)
    }

    pub fn load(home: &AgentsHome) -> Self {
        Self::parse(load_json(&home.sync_file).unwrap_or_else(|| Value::Object(Map::new())))
    }

    pub fn project_paths(&self, home: &Path) -> Vec<PathBuf> {
        self.projects.iter().map(|p| expand_home(p, home)).collect()
    }

    pub fn is_plugin_provided(&self, cli: Cli, name: &str) -> bool {
        self.plugin_provided
            .get(&cli)
            .map(|s| s.contains(name))
            .unwrap_or(false)
    }

    /// Merged override fields for one server: `overrides["*"]` then
    /// `overrides[scope_name]` (scope_name = "global" or the project folder name).
    pub fn override_fields(&self, scope_name: &str, cli: Cli, server: &str) -> Map<String, Value> {
        let mut out = Map::new();
        for sc in ["*", scope_name] {
            if let Some(fields) = self
                .overrides
                .get(sc)
                .and_then(|c| c.get(&cli))
                .and_then(|s| s.get(server))
            {
                for (k, v) in fields {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        out
    }

    /// Source servers with per-CLI overrides applied (shallow field merge,
    /// exactly like mcp-sync's `with_overrides`).
    pub fn with_overrides(
        &self,
        scope_name: &str,
        cli: Cli,
        src: &BTreeMap<String, Value>,
    ) -> BTreeMap<String, Value> {
        let mut out = src.clone();
        for (name, cfg) in out.iter_mut() {
            let fields = self.override_fields(scope_name, cli, name);
            if fields.is_empty() {
                continue;
            }
            if let Value::Object(obj) = cfg {
                for (k, v) in fields {
                    obj.insert(k, v);
                }
            }
        }
        out
    }
}

/// `mcpServers` of a Claude-format JSON file (`global.json`, `.mcp.json`).
pub fn read_mcp_servers(path: &Path) -> BTreeMap<String, Value> {
    load_json(path)
        .and_then(|v| v.get("mcpServers").and_then(|m| m.as_object()).cloned())
        .map(|m| m.into_iter().collect())
        .unwrap_or_default()
}

/// `disabledMcpjsonServers` of `<project>/.claude/settings.json` (off in
/// the three CLIs, mcp-sync semantics).
pub fn read_disabled(project: &Path) -> BTreeSet<String> {
    read_disabled_in(&project.join(".claude").join("settings.json"))
}

/// `disabledMcpjsonServers` of `<project>/.claude/settings.local.json`:
/// only Claude reads it, so it means "off in Claude only".
pub fn read_disabled_local(project: &Path) -> BTreeSet<String> {
    read_disabled_in(&project.join(".claude").join("settings.local.json"))
}

fn read_disabled_in(file: &Path) -> BTreeSet<String> {
    let st = load_json(file);
    string_list(st.as_ref().and_then(|s| s.get("disabledMcpjsonServers")))
        .into_iter()
        .collect()
}

#[derive(Debug, Clone)]
pub struct ProjectSource {
    pub path: PathBuf,
    /// Folder name, used as override scope (same as mcp-sync).
    pub name: String,
    pub servers: BTreeMap<String, Value>,
    pub disabled: BTreeSet<String>,
    pub disabled_claude_only: BTreeSet<String>,
}

impl ProjectSource {
    pub fn load(path: &Path) -> Self {
        ProjectSource {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            servers: read_mcp_servers(&path.join(".mcp.json")),
            disabled: read_disabled(path),
            disabled_claude_only: read_disabled_local(path),
            path: path.to_path_buf(),
        }
    }
}

/// Snapshot of the whole source of truth.
#[derive(Debug, Clone)]
pub struct SourceSnapshot {
    pub home: PathBuf,
    pub agents: AgentsHome,
    pub global: BTreeMap<String, Value>,
    pub sync: SyncConfig,
    pub projects: Vec<ProjectSource>,
}

impl SourceSnapshot {
    pub fn load(home: &Path) -> Self {
        let agents = AgentsHome::from_home(home);
        let sync = SyncConfig::load(&agents);
        let projects = sync
            .project_paths(home)
            .into_iter()
            .filter(|p| p.is_dir())
            .map(|p| ProjectSource::load(&p))
            .collect();
        SourceSnapshot {
            home: home.to_path_buf(),
            global: read_mcp_servers(&agents.global_mcp_file),
            agents,
            sync,
            projects,
        }
    }

    /// All resources described by the source, with their target matrix.
    pub fn resources(&self) -> Vec<Resource> {
        let mut out = Vec::new();
        for (name, cfg) in &self.global {
            out.push(self.mcp_resource(name, cfg, &Scope::Global, "global", None));
        }
        for p in &self.projects {
            let scope = Scope::Project {
                path: p.path.to_string_lossy().to_string(),
            };
            for (name, cfg) in &p.servers {
                let mut r = self.mcp_resource(name, cfg, &scope, &p.name, Some(&p.disabled));
                if p.disabled_claude_only.contains(name)
                    && r.targets.get(&Cli::Claude) == Some(&TargetState::On)
                {
                    r.targets.insert(Cli::Claude, TargetState::Off);
                }
                out.push(r);
            }
        }
        out.extend(skill_resources(&self.home, &self.agents));
        out.extend(agent_resources(&self.home));
        out
    }

    fn mcp_resource(
        &self,
        name: &str,
        cfg: &Value,
        scope: &Scope,
        scope_name: &str,
        disabled: Option<&BTreeSet<String>>,
    ) -> Resource {
        let transport = mcp_transport(cfg);
        let is_disabled = disabled.map(|d| d.contains(name)).unwrap_or(false);
        let mut targets = BTreeMap::new();
        for cli in Cli::ALL {
            let state = if is_disabled {
                TargetState::Off
            } else if cli == Cli::Codex && transport == "sse" {
                TargetState::Unsupported {
                    reason: "Codex não fala SSE".to_string(),
                }
            } else if matches!(scope, Scope::Global) && self.sync.is_plugin_provided(cli, name) {
                TargetState::ProvidedByPlugin
            } else if self.sync.is_cli_off(scope_name, cli, name) {
                TargetState::Off
            } else {
                TargetState::On
            };
            targets.insert(cli, state);
        }
        let mut overrides = BTreeMap::new();
        for cli in Cli::ALL {
            let f = self.sync.override_fields(scope_name, cli, name);
            if !f.is_empty() {
                overrides.insert(cli, Value::Object(f));
            }
        }
        let mut secret_refs = Vec::new();
        collect_var_refs(cfg, &mut secret_refs);
        let canonical = match scope {
            Scope::Global => self.agents.global_mcp_file.to_string_lossy().to_string(),
            Scope::Project { path } => Path::new(path)
                .join(".mcp.json")
                .to_string_lossy()
                .to_string(),
        };
        Resource {
            uid: resource_uid(ResourceKind::Mcp, scope, name),
            kind: ResourceKind::Mcp,
            name: name.to_string(),
            scope: scope.clone(),
            canonical,
            description: cfg
                .get("description")
                .and_then(|d| d.as_str())
                .map(str::to_string),
            literal_secret_keys: literal_secret_keys(cfg),
            config: Some(cfg.clone()),
            transport: Some(transport),
            targets,
            overrides,
            secret_refs,
        }
    }
}

/// Whether the Antigravity CLI is configured to read `dir` (via
/// `~/.gemini/config/skills.json` entries) and, if `include_only` is set,
/// whether `name` is in it.
fn agy_reads_skill(home: &Path, dir: &Path, name: &str) -> bool {
    let Some(doc) = load_json(&home.join(".gemini").join("config").join("skills.json")) else {
        return false;
    };
    let Some(entries) = doc.get("entries").and_then(|e| e.as_array()) else {
        return false;
    };
    entries.iter().any(|e| {
        let Some(p) = e.get("path").and_then(|p| p.as_str()) else {
            return false;
        };
        let p = expand_home(p, home);
        let same = p == dir
            || matches!((std::fs::canonicalize(&p), std::fs::canonicalize(dir)), (Ok(a), Ok(b)) if a == b);
        if !same {
            return false;
        }
        match e.get("include_only").and_then(|i| i.as_array()) {
            Some(list) => list.iter().any(|x| x.as_str() == Some(name)),
            None => true,
        }
    })
}

fn skill_description(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("SKILL.md")).ok()?;
    let fm = text.strip_prefix("---")?;
    let end = fm.find("\n---")?;
    fm[..end].lines().find_map(|l| {
        l.strip_prefix("description:")
            .map(|d| d.trim().trim_matches('"').to_string())
            .filter(|d| !d.is_empty())
    })
}

fn is_link(p: &Path) -> bool {
    std::fs::symlink_metadata(p)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Skills: canonical in `~/.agents/skills/<name>`; Claude gets a link in
/// `~/.claude/skills`, agy reads the folder through `skills.json`, Codex reads
/// `~/.agents/skills` natively. Real folders only in `~/.claude/skills` are
/// Claude-only resources.
pub fn skill_resources(home: &Path, agents: &AgentsHome) -> Vec<Resource> {
    let mut out = Vec::new();
    let claude_skills = home.join(".claude").join("skills");
    let mut shared_names = BTreeSet::new();

    if let Ok(rd) = std::fs::read_dir(&agents.skills_dir) {
        let mut dirs: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        dirs.sort();
        for dir in dirs {
            if !dir.join("SKILL.md").is_file() {
                continue;
            }
            let name = dir.file_name().unwrap().to_string_lossy().to_string();
            shared_names.insert(name.clone());
            let claude_dest = claude_skills.join(&name);
            let mut targets = BTreeMap::new();
            targets.insert(
                Cli::Claude,
                if claude_dest.exists() {
                    TargetState::On
                } else {
                    TargetState::Off
                },
            );
            targets.insert(Cli::Codex, TargetState::Native);
            targets.insert(
                Cli::Agy,
                if agy_reads_skill(home, &agents.skills_dir, &name) {
                    TargetState::Native
                } else {
                    TargetState::Off
                },
            );
            out.push(Resource {
                uid: resource_uid(ResourceKind::Skill, &Scope::Global, &name),
                kind: ResourceKind::Skill,
                description: skill_description(&dir),
                canonical: dir.to_string_lossy().to_string(),
                name,
                scope: Scope::Global,
                config: None,
                transport: None,
                targets,
                overrides: BTreeMap::new(),
                secret_refs: Vec::new(),
                literal_secret_keys: Vec::new(),
            });
        }
    }

    if let Ok(rd) = std::fs::read_dir(&claude_skills) {
        let mut dirs: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        dirs.sort();
        for dir in dirs {
            let name = dir.file_name().unwrap().to_string_lossy().to_string();
            if shared_names.contains(&name) || !dir.join("SKILL.md").is_file() {
                continue;
            }
            let canonical = if is_link(&dir) {
                std::fs::canonicalize(&dir).unwrap_or(dir.clone())
            } else {
                dir.clone()
            };
            let mut targets = BTreeMap::new();
            targets.insert(Cli::Claude, TargetState::On);
            targets.insert(Cli::Codex, TargetState::Off);
            targets.insert(Cli::Agy, TargetState::Off);
            out.push(Resource {
                uid: resource_uid(ResourceKind::Skill, &Scope::Global, &name),
                kind: ResourceKind::Skill,
                description: skill_description(&dir),
                canonical: canonical.to_string_lossy().to_string(),
                name,
                scope: Scope::Global,
                config: None,
                transport: None,
                targets,
                overrides: BTreeMap::new(),
                secret_refs: Vec::new(),
                literal_secret_keys: Vec::new(),
            });
        }
    }
    let listed: BTreeSet<String> = out.iter().map(|r| r.name.clone()).collect();
    out.extend(parked_skill_resources(
        &home.join(".claude"),
        &Scope::Global,
        &listed,
    ));
    out
}

fn skill_resource(
    name: String,
    scope: Scope,
    dir: &Path,
    targets: BTreeMap<Cli, TargetState>,
) -> Resource {
    let canonical = if is_link(dir) {
        std::fs::canonicalize(dir).unwrap_or(dir.to_path_buf())
    } else {
        dir.to_path_buf()
    };
    Resource {
        uid: resource_uid(ResourceKind::Skill, &scope, &name),
        kind: ResourceKind::Skill,
        description: skill_description(dir),
        canonical: canonical.to_string_lossy().to_string(),
        name,
        scope,
        config: None,
        transport: None,
        targets,
        overrides: BTreeMap::new(),
        secret_refs: Vec::new(),
        literal_secret_keys: Vec::new(),
    }
}

fn skill_dirs(dir: &Path) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.join("SKILL.md").is_file() {
                out.insert(e.file_name().to_string_lossy().to_string(), p);
            }
        }
    }
    out
}

/// Skills turned off in Claude by parking the real folder
/// (`<.claude>/.skills-disabled/<name>`): still listed, as off.
pub fn parked_skill_resources(
    claude_dir: &Path,
    scope: &Scope,
    skip: &BTreeSet<String>,
) -> Vec<Resource> {
    let parked = claude_dir.join(crate::services::skill_writer::DISABLED_SKILLS_DIR);
    skill_dirs(&parked)
        .into_iter()
        .filter(|(n, _)| !skip.contains(n))
        .map(|(name, dir)| {
            let mut targets = BTreeMap::new();
            targets.insert(Cli::Claude, TargetState::Off);
            for cli in [Cli::Codex, Cli::Agy] {
                targets.insert(
                    cli,
                    TargetState::Unsupported {
                        reason: "Só existe para o Claude (pasta .claude/skills)".into(),
                    },
                );
            }
            skill_resource(name, scope.clone(), &dir, targets)
        })
        .collect()
}

/// Project skills: `<p>/.agents/skills` (Codex and agy read it directly)
/// and `<p>/.claude/skills` (Claude; usually links into `.agents/skills`).
pub fn project_skill_resources(project: &Path) -> Vec<Resource> {
    let scope = Scope::Project {
        path: project.to_string_lossy().to_string(),
    };
    let shared = skill_dirs(&project.join(".agents").join("skills"));
    let claude = skill_dirs(&project.join(".claude").join("skills"));
    let mut names: BTreeSet<String> = shared.keys().cloned().collect();
    names.extend(claude.keys().cloned());
    let mut out: Vec<Resource> = names
        .iter()
        .map(|name| {
            let mut targets = BTreeMap::new();
            targets.insert(
                Cli::Claude,
                if claude.contains_key(name) {
                    TargetState::On
                } else {
                    TargetState::Off
                },
            );
            for cli in [Cli::Codex, Cli::Agy] {
                targets.insert(
                    cli,
                    if shared.contains_key(name) {
                        TargetState::Native
                    } else {
                        TargetState::Unsupported {
                            reason:
                                "Só existe em .claude/skills; o Codex e o agy leem .agents/skills"
                                    .into(),
                        }
                    },
                );
            }
            let dir = shared.get(name).or_else(|| claude.get(name)).unwrap();
            skill_resource(name.clone(), scope.clone(), dir, targets)
        })
        .collect();
    out.extend(parked_skill_resources(
        &project.join(".claude"),
        &scope,
        &names,
    ));
    out
}

/// Sub-agents in `~/.claude/agents/*.md` (Claude only for now).
pub fn agent_resources(home: &Path) -> Vec<Resource> {
    let dir = home.join(".claude").join("agents");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|f| {
            let name = f.file_stem().unwrap().to_string_lossy().to_string();
            let mut targets = BTreeMap::new();
            targets.insert(Cli::Claude, TargetState::On);
            for cli in [Cli::Codex, Cli::Agy] {
                targets.insert(
                    cli,
                    TargetState::Unsupported {
                        reason: "sub-agents do Claude Code ainda não têm equivalente sincronizado"
                            .into(),
                    },
                );
            }
            Resource {
                uid: resource_uid(ResourceKind::Agent, &Scope::Global, &name),
                kind: ResourceKind::Agent,
                canonical: f.to_string_lossy().to_string(),
                name,
                scope: Scope::Global,
                config: None,
                transport: None,
                description: None,
                targets,
                overrides: BTreeMap::new(),
                secret_refs: Vec::new(),
                literal_secret_keys: Vec::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    pub(crate) fn fixture() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let mcp = home.join(".agents/mcp");
        std::fs::create_dir_all(&mcp).unwrap();
        std::fs::write(
            mcp.join("global.json"),
            json!({"mcpServers": {
                "Neon": {"type": "http", "url": "https://mcp.neon.tech/mcp"},
                "ctx": {"type": "stdio", "command": "npx", "args": ["ctx"], "env": {"API_KEY": "literal-123"}},
                "events": {"type": "sse", "url": "http://x/sse", "headers": {"Authorization": "Bearer ${EV_TOKEN}"}}
            }})
            .to_string(),
        )
        .unwrap();
        let proj = home.join("w/proj");
        std::fs::create_dir_all(proj.join(".claude")).unwrap();
        std::fs::write(
            proj.join(".mcp.json"),
            json!({"mcpServers": {
                "supabase": {"type": "http", "url": "https://mcp.supabase.com/mcp?project_ref=${SB_REF}"},
                "off": {"command": "x"}
            }})
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            proj.join(".claude/settings.json"),
            json!({"disabledMcpjsonServers": ["off"]}).to_string(),
        )
        .unwrap();
        std::fs::write(
            mcp.join("sync.json"),
            json!({
                "_doc": "x",
                "projects": ["~/w/proj", "~/w/missing"],
                "pluginProvided": {"_doc": "y", "claude": ["Neon"], "codex": ["Neon"]},
                "codexGlobalKeep": ["node_repl"],
                "overrides": {
                    "global": {"agy": {"ctx": {"args": ["ctx", "--agy"]}}},
                    "*": {"agy": {"supabase": {"headers": {"A": "b"}}}},
                    "proj": {"codex": {"supabase": {"url": "https://codex"}}}
                }
            })
            .to_string(),
        )
        .unwrap();
        // skills
        let sk = home.join(".agents/skills/shared-one");
        std::fs::create_dir_all(&sk).unwrap();
        std::fs::write(
            sk.join("SKILL.md"),
            "---\nname: shared-one\ndescription: Shared skill\n---\nbody",
        )
        .unwrap();
        let sk2 = home.join(".agents/skills/unlinked");
        std::fs::create_dir_all(&sk2).unwrap();
        std::fs::write(sk2.join("SKILL.md"), "---\nname: unlinked\n---\n").unwrap();
        let claude_only = home.join(".claude/skills/claude-only");
        std::fs::create_dir_all(&claude_only).unwrap();
        std::fs::write(
            claude_only.join("SKILL.md"),
            "---\nname: claude-only\n---\n",
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&sk, home.join(".claude/skills/shared-one")).unwrap();
        std::fs::create_dir_all(home.join(".gemini/config")).unwrap();
        std::fs::write(
            home.join(".gemini/config/skills.json"),
            json!({"entries": [{"path": "~/.agents/skills"}]}).to_string(),
        )
        .unwrap();
        std::fs::create_dir_all(home.join(".claude/agents")).unwrap();
        std::fs::write(
            home.join(".claude/agents/reviewer.md"),
            "---\nname: reviewer\n---\n",
        )
        .unwrap();
        (tmp, home)
    }

    #[test]
    fn sync_config_ignores_doc_keys_and_parses_sections() {
        let (_t, home) = fixture();
        let snap = SourceSnapshot::load(&home);
        assert_eq!(snap.sync.projects.len(), 2);
        assert_eq!(
            snap.projects.len(),
            1,
            "missing project folders are skipped"
        );
        assert!(snap.sync.is_plugin_provided(Cli::Claude, "Neon"));
        assert!(!snap.sync.is_plugin_provided(Cli::Agy, "Neon"));
        assert!(snap.sync.codex_global_keep.contains("node_repl"));
        let f = snap.sync.override_fields("proj", Cli::Agy, "supabase");
        assert!(
            f.contains_key("headers"),
            "'*' overrides apply to every scope"
        );
    }

    #[test]
    fn mcp_target_matrix() {
        let (_t, home) = fixture();
        let res = SourceSnapshot::load(&home).resources();
        let get = |uid: &str| res.iter().find(|r| r.uid == uid).unwrap().clone();

        let neon = get("mcp:global:Neon");
        assert_eq!(neon.targets[&Cli::Claude], TargetState::ProvidedByPlugin);
        assert_eq!(neon.targets[&Cli::Agy], TargetState::On);

        let events = get("mcp:global:events");
        assert!(matches!(
            events.targets[&Cli::Codex],
            TargetState::Unsupported { .. }
        ));
        assert_eq!(events.secret_refs, vec!["EV_TOKEN"]);

        let ctx = get("mcp:global:ctx");
        assert_eq!(ctx.literal_secret_keys, vec!["env.API_KEY"]);
        assert!(ctx.overrides.contains_key(&Cli::Agy));

        let proj_path = home.join("w/proj").to_string_lossy().to_string();
        let off = get(&format!("mcp:project:{}:off", proj_path));
        assert!(Cli::ALL.iter().all(|c| off.targets[c] == TargetState::Off));
        let sb = get(&format!("mcp:project:{}:supabase", proj_path));
        assert_eq!(sb.secret_refs, vec!["SB_REF"]);
        assert!(sb.overrides.contains_key(&Cli::Codex));
        assert!(sb.overrides.contains_key(&Cli::Agy));
    }

    #[cfg(unix)]
    #[test]
    fn skill_and_agent_targets() {
        let (_t, home) = fixture();
        let res = SourceSnapshot::load(&home).resources();
        let get = |uid: &str| res.iter().find(|r| r.uid == uid).unwrap().clone();
        let shared = get("skill:global:shared-one");
        assert_eq!(shared.targets[&Cli::Claude], TargetState::On);
        assert_eq!(shared.targets[&Cli::Codex], TargetState::Native);
        assert_eq!(shared.targets[&Cli::Agy], TargetState::Native);
        assert_eq!(shared.description.as_deref(), Some("Shared skill"));
        let unlinked = get("skill:global:unlinked");
        assert_eq!(unlinked.targets[&Cli::Claude], TargetState::Off);
        let only = get("skill:global:claude-only");
        assert_eq!(only.targets[&Cli::Codex], TargetState::Off);
        // a link in ~/.claude/skills is not listed twice
        assert_eq!(res.iter().filter(|r| r.name == "shared-one").count(), 1);
        let agent = get("agent:global:reviewer");
        assert_eq!(agent.targets[&Cli::Claude], TargetState::On);
    }

    #[test]
    fn missing_layout_is_empty_not_error() {
        let tmp = TempDir::new().unwrap();
        let snap = SourceSnapshot::load(tmp.path());
        assert!(snap.global.is_empty());
        assert!(snap.resources().is_empty());
    }

    #[test]
    fn home_expansion_roundtrip() {
        let home = Path::new("/h/u");
        assert_eq!(expand_home("~/w/p", home), PathBuf::from("/h/u/w/p"));
        assert_eq!(contract_home(Path::new("/h/u/w/p"), home), "~/w/p");
        assert_eq!(contract_home(Path::new("/other"), home), "/other");
    }

    /// Manual check against the real machine (read-only, prints names and
    /// targets only, never values): `cargo test real_home_snapshot -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_home_snapshot() {
        let home = directories::BaseDirs::new()
            .unwrap()
            .home_dir()
            .to_path_buf();
        let snap = SourceSnapshot::load(&home);
        for r in snap
            .resources()
            .iter()
            .filter(|r| r.kind == ResourceKind::Mcp)
        {
            let t: Vec<String> = r
                .targets
                .iter()
                .map(|(c, s)| format!("{}={:?}", c.key(), s))
                .collect();
            println!(
                "{} [{}] refs={:?} literal={:?} ov={:?}",
                r.uid.replace(&home.to_string_lossy().to_string(), "~"),
                t.join(" "),
                r.secret_refs,
                r.literal_secret_keys,
                r.overrides.keys().collect::<Vec<_>>()
            );
        }
        let skills = snap
            .resources()
            .iter()
            .filter(|r| r.kind == ResourceKind::Skill)
            .count();
        println!("skills: {}", skills);
    }
}
