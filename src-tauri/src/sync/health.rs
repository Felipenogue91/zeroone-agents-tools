//! Health: divergences between the source of truth and what each CLI file
//! actually contains, with the action that fixes each one.

use super::engine::{FileChange, Plan, ReportItem};
use super::model::{resource_uid, Cli, Resource, ResourceKind, Scope, TargetState};
use super::occurrences::{Occurrence, AGY_PROJECT_REL, CODEX_PROJECT_REL};
use super::secrets::Secrets;
use super::source::{load_json, SourceSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DivergenceKind {
    /// In a CLI file but not in the source.
    OnlyInCli,
    /// In the source (target on) but missing from the CLI file.
    MissingInCli,
    /// Present on both sides with different content.
    Drift,
    /// Claude local-scope copy (`~/.claude.json` projects) that shadows the
    /// project's `.mcp.json` or is only visible to Claude.
    ClaudeLocalScope,
    /// agy does not read `~/.agents/skills`.
    AgySkillsFolder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthAction {
    /// Copy the CLI's entry into the source (global.json / .mcp.json).
    Import,
    /// Regenerate the CLI files from the source (drops extras, adds missing).
    Reapply,
    /// Remove the stale Claude local-scope copy.
    RemoveLocalCopy,
    /// Make agy read `~/.agents/skills`.
    FixAgySkills,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Divergence {
    pub kind: DivergenceKind,
    pub cli: Cli,
    pub scope: Scope,
    pub name: String,
    pub file_path: String,
    pub message: String,
    pub actions: Vec<HealthAction>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub in_sync: bool,
    pub resources: usize,
    pub divergences: Vec<Divergence>,
    pub report: Vec<ReportItem>,
    pub checked_at: String,
}

fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Global => "global".into(),
        Scope::Project { path } => Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone()),
    }
}

pub fn compute_health(
    snap: &SourceSnapshot,
    resources: &[Resource],
    occ: &[Occurrence],
    plan: &Plan,
) -> Health {
    let mut out: Vec<Divergence> = Vec::new();
    let mut seen: BTreeSet<(Cli, String)> = BTreeSet::new(); // (cli, uid)
    let claude_json = snap.home.join(".claude.json").to_string_lossy().to_string();
    let src_uids: BTreeSet<&str> = resources
        .iter()
        .filter(|r| r.kind == ResourceKind::Mcp)
        .map(|r| r.uid.as_str())
        .collect();

    // 1. Claude local scope (shadows / Claude-only)
    for o in occ
        .iter()
        .filter(|o| o.kind == ResourceKind::Mcp && o.cli == Cli::Claude)
    {
        if o.file_path != claude_json || o.scope == Scope::Global {
            continue;
        }
        let global_uid = resource_uid(ResourceKind::Mcp, &Scope::Global, &o.name);
        let shadows = src_uids.contains(o.uid.as_str()) || src_uids.contains(global_uid.as_str());
        out.push(Divergence {
            kind: DivergenceKind::ClaudeLocalScope,
            cli: Cli::Claude,
            scope: o.scope.clone(),
            name: o.name.clone(),
            file_path: o.file_path.clone(),
            message: if shadows {
                format!(
                    "{}: cópia antiga no escopo local do Claude vence a do projeto ({})",
                    o.name,
                    scope_label(&o.scope)
                )
            } else {
                format!(
                    "{}: só o Claude vê (escopo local de {}); mova pro .mcp.json",
                    o.name,
                    scope_label(&o.scope)
                )
            },
            actions: if shadows {
                vec![HealthAction::RemoveLocalCopy]
            } else {
                vec![HealthAction::Import, HealthAction::RemoveLocalCopy]
            },
        });
    }

    // 2. In a CLI file but not in the source
    for o in occ.iter().filter(|o| o.kind == ResourceKind::Mcp) {
        if o.cli == Cli::Claude && (o.file_path == claude_json && o.scope != Scope::Global) {
            continue; // handled above
        }
        if o.cli == Cli::Claude && o.scope != Scope::Global {
            continue; // project .mcp.json is the source itself
        }
        if src_uids.contains(o.uid.as_str()) {
            continue;
        }
        if o.scope == Scope::Global {
            if o.cli == Cli::Codex && snap.sync.codex_global_keep.contains(&o.name) {
                continue;
            }
        }
        // only report projects the engine manages
        if let Scope::Project { path } = &o.scope {
            if !snap.projects.iter().any(|p| p.path == Path::new(path)) {
                continue;
            }
        }
        seen.insert((o.cli, o.uid.clone()));
        let provided = o.scope == Scope::Global && snap.sync.is_plugin_provided(o.cli, &o.name);
        out.push(Divergence {
            kind: DivergenceKind::OnlyInCli,
            cli: o.cli,
            scope: o.scope.clone(),
            name: o.name.clone(),
            file_path: o.file_path.clone(),
            message: if provided {
                format!(
                    "{}: o {} já recebe por plugin; a cópia no arquivo sobra",
                    o.name,
                    o.cli.label()
                )
            } else {
                format!(
                    "{}: está no {} ({}) mas não na fonte",
                    o.name,
                    o.cli.label(),
                    scope_label(&o.scope)
                )
            },
            actions: if provided {
                vec![HealthAction::Reapply]
            } else {
                vec![HealthAction::Import, HealthAction::Reapply]
            },
        });
    }

    // 3. In the source but missing from a CLI file
    for r in resources.iter().filter(|r| r.kind == ResourceKind::Mcp) {
        for (cli, state) in &r.targets {
            if *state != TargetState::On {
                continue;
            }
            if *cli == Cli::Claude && r.scope != Scope::Global {
                continue; // Claude reads the project .mcp.json directly
            }
            let present = occ.iter().any(|o| {
                o.uid == r.uid
                    && o.cli == *cli
                    && !(o.file_path == claude_json && o.scope != Scope::Global)
            });
            if present {
                continue;
            }
            seen.insert((*cli, r.uid.clone()));
            let file = match (&r.scope, cli) {
                (Scope::Global, Cli::Claude) => claude_json.clone(),
                (Scope::Global, Cli::Codex) => snap
                    .home
                    .join(".codex/config.toml")
                    .to_string_lossy()
                    .to_string(),
                (Scope::Global, Cli::Agy) => snap
                    .home
                    .join(".gemini/config/mcp_config.json")
                    .to_string_lossy()
                    .to_string(),
                (Scope::Project { path }, Cli::Codex) => Path::new(path)
                    .join(CODEX_PROJECT_REL)
                    .to_string_lossy()
                    .to_string(),
                (Scope::Project { path }, _) => Path::new(path)
                    .join(AGY_PROJECT_REL)
                    .to_string_lossy()
                    .to_string(),
            };
            out.push(Divergence {
                kind: DivergenceKind::MissingInCli,
                cli: *cli,
                scope: r.scope.clone(),
                name: r.name.clone(),
                file_path: file,
                message: format!(
                    "{}: está na fonte mas falta no {} ({})",
                    r.name,
                    cli.label(),
                    scope_label(&r.scope)
                ),
                actions: vec![HealthAction::Reapply],
            });
        }
    }

    // 4. Same name on both sides, different content (from the engine plan)
    for ch in &plan.changes {
        for name in &ch.servers {
            let uid = resource_uid(ResourceKind::Mcp, &ch.scope, name);
            if seen.contains(&(ch.cli, uid.clone())) || !src_uids.contains(uid.as_str()) {
                continue;
            }
            let in_file = occ.iter().any(|o| o.uid == uid && o.cli == ch.cli);
            if !in_file {
                continue;
            }
            seen.insert((ch.cli, uid));
            out.push(Divergence {
                kind: DivergenceKind::Drift,
                cli: ch.cli,
                scope: ch.scope.clone(),
                name: name.clone(),
                file_path: ch.path.to_string_lossy().to_string(),
                message: match &ch.scope {
                    Scope::Global => format!(
                        "{}: o {} (global) está diferente da fonte",
                        name,
                        ch.cli.label()
                    ),
                    scope => format!(
                        "{}: o {} ({} · {}) está diferente da fonte",
                        name,
                        ch.cli.label(),
                        scope_label(scope),
                        ch.label
                    ),
                },
                actions: vec![HealthAction::Reapply],
            });
        }
    }

    // 5. agy does not read the shared skills folder
    if super::skills::plan_agy_reads_shared(&snap.home, &snap.agents).is_some()
        && resources.iter().any(|r| {
            r.kind == ResourceKind::Skill && r.targets.get(&Cli::Agy) == Some(&TargetState::Off)
        })
        && !resources.iter().any(|r| {
            r.kind == ResourceKind::Skill && r.targets.get(&Cli::Agy) == Some(&TargetState::Native)
        })
    {
        out.push(Divergence {
            kind: DivergenceKind::AgySkillsFolder,
            cli: Cli::Agy,
            scope: Scope::Global,
            name: "~/.agents/skills".into(),
            file_path: snap
                .home
                .join(".gemini/config/skills.json")
                .to_string_lossy()
                .to_string(),
            message: "O Antigravity CLI não lê ~/.agents/skills".into(),
            actions: vec![HealthAction::FixAgySkills],
        });
    }

    let in_sync =
        out.is_empty() && plan.changes.is_empty() && plan.report.iter().all(|r| r.status == "ok");
    Health {
        in_sync,
        resources: resources.len(),
        divergences: out,
        report: plan.report.clone(),
        checked_at: chrono::Local::now().to_rfc3339(),
    }
}

// ------------------------------------------------------------------ import

fn str_field(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(str::to_string)
}

/// Codex/agy/Claude entry → Claude `.mcp.json` format, with resolved secret
/// values turned back into `${VAR}` references.
pub fn to_claude_format(cli: Cli, raw: &Value, secrets: &Secrets) -> Value {
    let mut out = Map::new();
    match cli {
        Cli::Claude => {
            if let Some(m) = raw.as_object() {
                out = m.clone();
            }
        }
        Cli::Agy => {
            if let Some(url) = str_field(raw, "serverUrl").or_else(|| str_field(raw, "url")) {
                out.insert("type".into(), "http".into());
                out.insert("url".into(), url.into());
                if let Some(h) = raw.get("headers") {
                    out.insert("headers".into(), h.clone());
                }
            } else {
                out.insert("type".into(), "stdio".into());
                for k in ["command", "args", "env"] {
                    if let Some(v) = raw.get(k) {
                        out.insert(k.into(), v.clone());
                    }
                }
            }
        }
        Cli::Codex => {
            if let Some(url) = str_field(raw, "url") {
                out.insert("type".into(), "http".into());
                out.insert("url".into(), url.into());
                let mut headers = Map::new();
                if let Some(h) = raw.get("http_headers").and_then(|h| h.as_object()) {
                    headers.extend(h.clone());
                }
                if let Some(h) = raw.get("env_http_headers").and_then(|h| h.as_object()) {
                    for (k, var) in h {
                        if let Some(var) = var.as_str() {
                            headers.insert(k.clone(), format!("${{{}}}", var).into());
                        }
                    }
                }
                if let Some(var) = str_field(raw, "bearer_token_env_var") {
                    headers.insert(
                        "Authorization".into(),
                        format!("Bearer ${{{}}}", var).into(),
                    );
                }
                if !headers.is_empty() {
                    out.insert("headers".into(), Value::Object(headers));
                }
            } else {
                out.insert("type".into(), "stdio".into());
                for k in ["command", "args"] {
                    if let Some(v) = raw.get(k) {
                        out.insert(k.into(), v.clone());
                    }
                }
                let mut env = raw
                    .get("env")
                    .and_then(|e| e.as_object())
                    .cloned()
                    .unwrap_or_default();
                if let Some(vars) = raw.get("env_vars").and_then(|e| e.as_array()) {
                    for var in vars.iter().filter_map(|v| v.as_str()) {
                        env.insert(var.into(), format!("${{{}}}", var).into());
                    }
                }
                if !env.is_empty() {
                    out.insert("env".into(), Value::Object(env));
                }
            }
        }
    }
    secrets.unresolve(&Value::Object(out))
}

/// Raw entry of `name` in a CLI file.
pub fn read_cli_entry(cli: Cli, file: &Path, project: Option<&Path>, name: &str) -> Option<Value> {
    match cli {
        Cli::Codex => {
            let text = std::fs::read_to_string(file).ok()?;
            let cur = super::codex::codex_current(Some(&text)).ok()?;
            cur.get(name).cloned()
        }
        Cli::Agy => load_json(file)?.get("mcpServers")?.get(name).cloned(),
        Cli::Claude => {
            let doc = load_json(file)?;
            match project {
                Some(p) => doc
                    .get("projects")?
                    .get(p.to_string_lossy().as_ref())?
                    .get("mcpServers")?
                    .get(name)
                    .cloned(),
                None => doc.get("mcpServers")?.get(name).cloned(),
            }
        }
    }
}

/// Planned edit adding `name` (from a CLI file) to the source.
pub fn plan_import(
    snap: &SourceSnapshot,
    secrets: &Secrets,
    cli: Cli,
    scope: &Scope,
    file: &Path,
    name: &str,
) -> Result<FileChange, String> {
    let project = scope.project_path().map(PathBuf::from);
    let raw = read_cli_entry(cli, file, project.as_deref(), name)
        .ok_or_else(|| format!("{} não encontrado em {}", name, file.display()))?;
    let cfg = to_claude_format(cli, &raw, secrets);
    let target = match &project {
        Some(p) => p.join(".mcp.json"),
        None => snap.agents.global_mcp_file.clone(),
    };
    let before = std::fs::read_to_string(&target).ok();
    let base = before
        .clone()
        .unwrap_or_else(|| "{\n  \"mcpServers\": {}\n}\n".to_string());
    let after = super::jsonedit::set_server(&base, name, Some(&cfg))?;
    Ok(FileChange {
        cli: Cli::Claude,
        scope: scope.clone(),
        label: "fonte".into(),
        path: target,
        before,
        after: Some(after),
        mode: None,
        servers: vec![name.to_string()],
        ensure_ignored: None,
        claude_patch: None,
    })
}

/// Planned removal of stale Claude local-scope copies for one project.
pub fn plan_remove_local(
    snap: &SourceSnapshot,
    project: &str,
    names: &[String],
) -> Option<FileChange> {
    let path = snap.home.join(".claude.json");
    let before = std::fs::read_to_string(&path).ok()?;
    let mut doc: Value = serde_json::from_str(&before).ok()?;
    let patch = super::engine::ClaudePatch::RemoveLocal {
        project: project.to_string(),
        names: names.to_vec(),
    };
    patch.apply_to(&mut doc);
    let after = serde_json::to_string_pretty(&doc).ok()? + "\n";
    Some(FileChange {
        cli: Cli::Claude,
        scope: Scope::Project {
            path: project.to_string(),
        },
        label: "claude (escopo local)".into(),
        path,
        before: Some(before),
        after: Some(after),
        mode: None,
        servers: names.to_vec(),
        ensure_ignored: None,
        claude_patch: Some(patch),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::engine::Engine;
    use crate::sync::occurrences;
    use serde_json::json;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    fn secrets() -> Secrets {
        let mut m = HashMap::new();
        m.insert("TOK".into(), "tok-secret-1".into());
        Secrets::from_map(m)
    }

    fn fx() -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let proj = home.join("w/p");
        write(
            &home.join(".agents/mcp/global.json"),
            "{\n  \"mcpServers\": {\n    \"a\": { \"type\": \"stdio\", \"command\": \"a\" },\n    \"b\": { \"type\": \"http\", \"url\": \"http://b\" }\n  }\n}\n",
        );
        write(
            &home.join(".agents/mcp/sync.json"),
            &json!({"projects": ["~/w/p"], "codexGlobalKeep": ["node_repl"]}).to_string(),
        );
        write(
            &proj.join(".mcp.json"),
            &json!({"mcpServers": {"sb": {"type": "http", "url": "http://sb"}}}).to_string(),
        );
        write(
            &home.join(".claude.json"),
            &json!({
                "mcpServers": {"a": {"type": "stdio", "command": "a", "args": [], "env": {}}},
                "projects": {proj.to_string_lossy(): {"mcpServers": {"sb": {"command": "old"}, "secret-only": {"command": "x"}}}}
            })
            .to_string(),
        );
        write(
            &home.join(".codex/config.toml"),
            "[mcp_servers.a]\ncommand = \"a\"\n[mcp_servers.node_repl]\ncommand = \"n\"\n[mcp_servers.extra]\nurl = \"http://e\"\nbearer_token_env_var = \"TOK\"\n",
        );
        write(
            &home.join(".gemini/config/mcp_config.json"),
            &json!({"mcpServers": {"a": {"command": "DIFFERENT"}, "b": {"serverUrl": "http://b"}, "gone": {"serverUrl": "http://g", "headers": {"Authorization": "Bearer tok-secret-1"}}}}).to_string(),
        );
        (tmp, home, proj)
    }

    fn health(home: &Path) -> Health {
        let snap = SourceSnapshot::load(home);
        let res = snap.resources();
        let projects: Vec<PathBuf> = snap.projects.iter().map(|p| p.path.clone()).collect();
        let occ = occurrences::collect(home, &projects);
        let eng = Engine::with_secrets(&snap, secrets());
        let plan = eng.plan(None);
        compute_health(&snap, &res, &occ, &plan)
    }

    fn find<'a>(
        h: &'a Health,
        kind: DivergenceKind,
        cli: Cli,
        name: &str,
    ) -> Option<&'a Divergence> {
        h.divergences
            .iter()
            .find(|d| d.kind == kind && d.cli == cli && d.name == name)
    }

    #[test]
    fn detects_every_divergence_kind() {
        let (_t, home, _p) = fx();
        let h = health(&home);
        assert!(!h.in_sync);
        // Codex: b missing, extra only in CLI, node_repl kept on purpose
        assert!(find(&h, DivergenceKind::MissingInCli, Cli::Codex, "b").is_some());
        assert!(find(&h, DivergenceKind::OnlyInCli, Cli::Codex, "extra").is_some());
        assert!(find(&h, DivergenceKind::OnlyInCli, Cli::Codex, "node_repl").is_none());
        // Claude: b missing in user scope
        assert!(find(&h, DivergenceKind::MissingInCli, Cli::Claude, "b").is_some());
        // agy: a drifted, gone only in CLI
        assert!(find(&h, DivergenceKind::Drift, Cli::Agy, "a").is_some());
        assert!(find(&h, DivergenceKind::OnlyInCli, Cli::Agy, "gone").is_some());
        // project: agy file missing sb
        assert!(find(&h, DivergenceKind::MissingInCli, Cli::Agy, "sb").is_some());
        // Claude local scope
        let shadow = find(&h, DivergenceKind::ClaudeLocalScope, Cli::Claude, "sb").unwrap();
        assert_eq!(shadow.actions, vec![HealthAction::RemoveLocalCopy]);
        let only = find(
            &h,
            DivergenceKind::ClaudeLocalScope,
            Cli::Claude,
            "secret-only",
        )
        .unwrap();
        assert!(only.actions.contains(&HealthAction::Import));
    }

    #[test]
    fn import_from_codex_keeps_references_and_file_style() {
        let (_t, home, _p) = fx();
        let snap = SourceSnapshot::load(&home);
        let ch = plan_import(
            &snap,
            &secrets(),
            Cli::Codex,
            &Scope::Global,
            &home.join(".codex/config.toml"),
            "extra",
        )
        .unwrap();
        let after = ch.after.unwrap();
        assert!(after.contains("    \"extra\": { \"type\": \"http\", \"url\": \"http://e\", \"headers\": { \"Authorization\": \"Bearer ${TOK}\" } }\n"));
        assert!(after.starts_with(
            "{\n  \"mcpServers\": {\n    \"a\": { \"type\": \"stdio\", \"command\": \"a\" },"
        ));
    }

    #[test]
    fn import_from_agy_turns_resolved_secret_back_into_reference() {
        let (_t, home, _p) = fx();
        let snap = SourceSnapshot::load(&home);
        let ch = plan_import(
            &snap,
            &secrets(),
            Cli::Agy,
            &Scope::Global,
            &home.join(".gemini/config/mcp_config.json"),
            "gone",
        )
        .unwrap();
        let after = ch.after.unwrap();
        assert!(!after.contains("tok-secret-1"));
        assert!(after.contains("Bearer ${TOK}"));
    }

    #[test]
    fn remove_local_copy_patch() {
        let (_t, home, proj) = fx();
        let snap = SourceSnapshot::load(&home);
        let ch = plan_remove_local(&snap, &proj.to_string_lossy(), &["sb".to_string()]).unwrap();
        crate::sync::engine::apply(&[ch], &home.join("bk")).unwrap();
        let cj: Value =
            serde_json::from_str(&std::fs::read_to_string(home.join(".claude.json")).unwrap())
                .unwrap();
        let local = &cj["projects"][proj.to_string_lossy().as_ref()]["mcpServers"];
        assert!(local.get("sb").is_none());
        assert!(local.get("secret-only").is_some());
    }

    #[test]
    fn clean_machine_is_in_sync() {
        let (_t, home, _p) = fx();
        let snap = SourceSnapshot::load(&home);
        let eng = Engine::with_secrets(&snap, secrets());
        // import extras so nothing is "only in CLI", then reapply
        for (cli, file, name) in [
            (Cli::Codex, home.join(".codex/config.toml"), "extra"),
            (
                Cli::Agy,
                home.join(".gemini/config/mcp_config.json"),
                "gone",
            ),
        ] {
            let snap = SourceSnapshot::load(&home);
            let ch = plan_import(&snap, &secrets(), cli, &Scope::Global, &file, name).unwrap();
            std::fs::write(&ch.path, ch.after.unwrap()).unwrap();
        }
        drop(eng);
        let snap = SourceSnapshot::load(&home);
        let eng = Engine::with_secrets(&snap, secrets());
        crate::sync::engine::apply(&eng.plan(None).changes, &home.join("bk")).unwrap();
        // drop the stale local-scope copies
        let mut cj: Value =
            serde_json::from_str(&std::fs::read_to_string(home.join(".claude.json")).unwrap())
                .unwrap();
        cj["projects"] = json!({});
        std::fs::write(home.join(".claude.json"), cj.to_string()).unwrap();
        let h = health(&home);
        assert!(h.divergences.is_empty(), "{:#?}", h.divergences);
        assert!(h.in_sync);
    }
}
