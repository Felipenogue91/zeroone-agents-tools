//! Where each resource was actually seen in the CLI config files.
//!
//! Read-only scan of the generated/target files of every CLI. Comparing
//! occurrences with the source (`SourceSnapshot::resources`) is what the
//! health screen shows as divergences.

use super::model::*;
use super::source::{expand_home, load_json};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub uid: String,
    pub kind: ResourceKind,
    pub name: String,
    pub cli: Cli,
    pub scope: Scope,
    pub file_path: String,
}

/// Paths of every file the engine reads/writes for one machine.
#[derive(Debug, Clone)]
pub struct CliFiles {
    pub claude_json: PathBuf,
    pub codex_global: PathBuf,
    pub agy_global: PathBuf,
    pub agy_skills: PathBuf,
    pub claude_skills: PathBuf,
    pub claude_agents: PathBuf,
}

impl CliFiles {
    pub fn for_home(home: &Path) -> Self {
        CliFiles {
            claude_json: home.join(".claude.json"),
            codex_global: home.join(".codex").join("config.toml"),
            agy_global: home.join(".gemini").join("config").join("mcp_config.json"),
            agy_skills: home.join(".gemini").join("config").join("skills.json"),
            claude_skills: home.join(".claude").join("skills"),
            claude_agents: home.join(".claude").join("agents"),
        }
    }
}

pub const CODEX_PROJECT_REL: &str = ".codex/config.toml";
pub const AGY_PROJECT_REL: &str = ".agents/mcp_config.json";

pub fn json_server_names(path: &Path) -> Vec<String> {
    load_json(path)
        .and_then(|v| v.get("mcpServers").and_then(|m| m.as_object()).cloned())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

pub fn toml_server_names(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    match text.parse::<toml::Table>() {
        Ok(t) => t
            .get("mcp_servers")
            .and_then(|s| s.as_table())
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default(),
        Err(e) => {
            log::warn!("[sync] could not parse {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

fn dir_names(dir: &Path, want_skill_md: bool) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter(|e| !want_skill_md || e.path().join("SKILL.md").is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();
    out.sort();
    out
}

fn push(
    out: &mut Vec<Occurrence>,
    kind: ResourceKind,
    cli: Cli,
    scope: &Scope,
    file: &Path,
    names: impl IntoIterator<Item = String>,
) {
    for name in names {
        out.push(Occurrence {
            uid: resource_uid(kind, scope, &name),
            kind,
            name,
            cli,
            scope: scope.clone(),
            file_path: file.to_string_lossy().to_string(),
        });
    }
}

/// Scan every CLI file for one home and a set of project folders.
pub fn collect(home: &Path, projects: &[PathBuf]) -> Vec<Occurrence> {
    let files = CliFiles::for_home(home);
    let mut out = Vec::new();
    let g = Scope::Global;

    // Claude: user scope and local (per-project) scope in ~/.claude.json
    let claude = load_json(&files.claude_json);
    if let Some(c) = &claude {
        if let Some(m) = c.get("mcpServers").and_then(|m| m.as_object()) {
            push(
                &mut out,
                ResourceKind::Mcp,
                Cli::Claude,
                &g,
                &files.claude_json,
                m.keys().cloned(),
            );
        }
    }
    // Codex / agy global
    push(
        &mut out,
        ResourceKind::Mcp,
        Cli::Codex,
        &g,
        &files.codex_global,
        toml_server_names(&files.codex_global),
    );
    push(
        &mut out,
        ResourceKind::Mcp,
        Cli::Agy,
        &g,
        &files.agy_global,
        json_server_names(&files.agy_global),
    );

    // Skills
    push(
        &mut out,
        ResourceKind::Skill,
        Cli::Claude,
        &g,
        &files.claude_skills,
        dir_names(&files.claude_skills, true),
    );
    let shared = home.join(".agents").join("skills");
    let shared_names = dir_names(&shared, true);
    push(
        &mut out,
        ResourceKind::Skill,
        Cli::Codex,
        &g,
        &shared,
        shared_names.clone(),
    );
    if let Some(doc) = load_json(&files.agy_skills) {
        for e in doc
            .get("entries")
            .and_then(|e| e.as_array())
            .into_iter()
            .flatten()
        {
            let Some(p) = e.get("path").and_then(|p| p.as_str()) else {
                continue;
            };
            let dir = expand_home(p, home);
            let only: Option<BTreeSet<String>> =
                e.get("include_only").and_then(|i| i.as_array()).map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                });
            let names = dir_names(&dir, true)
                .into_iter()
                .filter(|n| only.as_ref().map(|o| o.contains(n)).unwrap_or(true));
            push(&mut out, ResourceKind::Skill, Cli::Agy, &g, &dir, names);
        }
    }
    // Agents
    let agents: Vec<String> = dir_names(&files.claude_agents, false)
        .into_iter()
        .filter_map(|n| n.strip_suffix(".md").map(str::to_string))
        .collect();
    push(
        &mut out,
        ResourceKind::Agent,
        Cli::Claude,
        &g,
        &files.claude_agents,
        agents,
    );

    // Projects
    for p in projects {
        let scope = Scope::Project {
            path: p.to_string_lossy().to_string(),
        };
        let mcp_json = p.join(".mcp.json");
        push(
            &mut out,
            ResourceKind::Mcp,
            Cli::Claude,
            &scope,
            &mcp_json,
            json_server_names(&mcp_json),
        );
        if let Some(local) = claude
            .as_ref()
            .and_then(|c| c.get("projects"))
            .and_then(|ps| ps.get(p.to_string_lossy().as_ref()))
            .and_then(|pr| pr.get("mcpServers"))
            .and_then(|m| m.as_object())
        {
            push(
                &mut out,
                ResourceKind::Mcp,
                Cli::Claude,
                &scope,
                &files.claude_json,
                local.keys().cloned(),
            );
        }
        let codex = p.join(CODEX_PROJECT_REL);
        push(
            &mut out,
            ResourceKind::Mcp,
            Cli::Codex,
            &scope,
            &codex,
            toml_server_names(&codex),
        );
        let agy = p.join(AGY_PROJECT_REL);
        push(
            &mut out,
            ResourceKind::Mcp,
            Cli::Agy,
            &scope,
            &agy,
            json_server_names(&agy),
        );
        let skills = p.join(".claude").join("skills");
        push(
            &mut out,
            ResourceKind::Skill,
            Cli::Claude,
            &scope,
            &skills,
            dir_names(&skills, true),
        );
    }

    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn collects_from_every_cli_file() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let proj = home.join("w/p");
        std::fs::create_dir_all(proj.join(".codex")).unwrap();
        std::fs::create_dir_all(proj.join(".agents")).unwrap();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::create_dir_all(home.join(".gemini/config")).unwrap();
        std::fs::write(
            home.join(".claude.json"),
            json!({"mcpServers": {"a": {}}, "projects": {proj.to_string_lossy(): {"mcpServers": {"local-only": {}}}}}).to_string(),
        )
        .unwrap();
        std::fs::write(home.join(".codex/config.toml"), "model = \"x\"\n[mcp_servers.a]\ncommand = \"a\"\n[mcp_servers.node_repl]\ncommand = \"n\"\n").unwrap();
        std::fs::write(
            home.join(".gemini/config/mcp_config.json"),
            json!({"mcpServers": {"a": {}}}).to_string(),
        )
        .unwrap();
        std::fs::write(
            proj.join(".mcp.json"),
            json!({"mcpServers": {"sb": {}}}).to_string(),
        )
        .unwrap();
        std::fs::write(
            proj.join(".codex/config.toml"),
            "[mcp_servers.sb]\nurl = \"u\"\n",
        )
        .unwrap();
        std::fs::write(
            proj.join(".agents/mcp_config.json"),
            json!({"mcpServers": {"sb": {}}}).to_string(),
        )
        .unwrap();

        let occ = collect(home, &[proj.clone()]);
        let has = |cli: Cli, uid: &str| occ.iter().any(|o| o.cli == cli && o.uid == uid);
        assert!(has(Cli::Claude, "mcp:global:a"));
        assert!(has(Cli::Codex, "mcp:global:a"));
        assert!(has(Cli::Codex, "mcp:global:node_repl"));
        assert!(has(Cli::Agy, "mcp:global:a"));
        let ps = proj.to_string_lossy();
        assert!(has(Cli::Claude, &format!("mcp:project:{}:sb", ps)));
        assert!(has(Cli::Claude, &format!("mcp:project:{}:local-only", ps)));
        assert!(has(Cli::Codex, &format!("mcp:project:{}:sb", ps)));
        assert!(has(Cli::Agy, &format!("mcp:project:{}:sb", ps)));
    }

    #[test]
    fn empty_home_has_no_occurrences() {
        let tmp = TempDir::new().unwrap();
        assert!(collect(tmp.path(), &[]).is_empty());
    }
}
