//! Editing the source of truth from the UI ("save" = write the source and
//! regenerate the CLI files right away, as one undoable set).
//!
//! Where each switch lives:
//! - server off in the three CLIs (project): `.claude/settings.json`
//!   `disabledMcpjsonServers` (mcp-sync semantics);
//! - off in Claude only (project): `.claude/settings.local.json`
//!   `disabledMcpjsonServers` (only Claude reads it);
//! - off in Codex/agy only, or in one CLI for a global server:
//!   `sync.json` `cliOff[scope][cli]`;
//! - add/edit/remove a server: `global.json` or `<project>/.mcp.json`;
//! - manage a project: `sync.json` `projects`.

use super::engine::{ApplyOutcome, BackupSet, Engine, FileChange};
use super::jsonedit::{set_disabled, set_key, set_server};
use super::model::{Cli, Scope};
use super::source::{contract_home, SourceSnapshot};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn text_change(
    path: PathBuf,
    before: Option<String>,
    after: String,
    scope: Scope,
    servers: Vec<String>,
) -> FileChange {
    FileChange {
        cli: Cli::Claude,
        scope,
        label: "fonte".into(),
        path,
        before,
        after: Some(after),
        mode: None,
        servers,
        ensure_ignored: None,
        claude_patch: None,
    }
}

const EMPTY_SOURCE: &str = "{\n  \"mcpServers\": {}\n}\n";

fn project_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Add/replace (`Some`) or remove (`None`) a server in the source.
pub fn plan_set_server(
    snap: &SourceSnapshot,
    scope: &Scope,
    name: &str,
    cfg: Option<&Value>,
) -> Result<Vec<FileChange>, String> {
    if name.trim().is_empty() || name.contains(':') {
        return Err("nome de MCP inválido".into());
    }
    let path = match scope {
        Scope::Global => snap.agents.global_mcp_file.clone(),
        Scope::Project { path } => Path::new(path).join(".mcp.json"),
    };
    let before = std::fs::read_to_string(&path).ok();
    if before.is_none() && cfg.is_none() {
        return Ok(vec![]);
    }
    let after = set_server(before.as_deref().unwrap_or(EMPTY_SOURCE), name, cfg)?;
    if before.as_deref() == Some(after.as_str()) {
        return Ok(vec![]);
    }
    Ok(vec![text_change(
        path,
        before,
        after,
        scope.clone(),
        vec![name.into()],
    )])
}

/// Turn a project server off/on in the three CLIs.
pub fn plan_set_enabled(
    project: &Path,
    name: &str,
    enabled: bool,
) -> Result<Vec<FileChange>, String> {
    let path = project.join(".claude").join("settings.json");
    let before = std::fs::read_to_string(&path).ok();
    match set_disabled(before.as_deref(), name, !enabled)? {
        Some(after) => Ok(vec![text_change(
            path,
            before,
            after,
            Scope::Project {
                path: project.to_string_lossy().to_string(),
            },
            vec![name.into()],
        )]),
        None => Ok(vec![]),
    }
}

fn sync_json_text(snap: &SourceSnapshot) -> (Option<String>, String) {
    let before = std::fs::read_to_string(&snap.agents.sync_file).ok();
    let base = before
        .clone()
        .unwrap_or_else(|| "{\n  \"projects\": []\n}\n".to_string());
    (before, base)
}

/// Turn a server off/on in one CLI only.
pub fn plan_set_target(
    snap: &SourceSnapshot,
    scope: &Scope,
    name: &str,
    cli: Cli,
    enabled: bool,
) -> Result<Vec<FileChange>, String> {
    if let (Scope::Project { path }, Cli::Claude) = (scope, cli) {
        // Claude-only switch: settings.local.json (only Claude reads it)
        let file = Path::new(path).join(".claude").join("settings.local.json");
        let before = std::fs::read_to_string(&file).ok();
        return Ok(match set_disabled(before.as_deref(), name, !enabled)? {
            Some(after) => vec![text_change(
                file,
                before,
                after,
                scope.clone(),
                vec![name.into()],
            )],
            None => vec![],
        });
    }
    if matches!(scope, Scope::Global) && snap.sync.is_plugin_provided(cli, name) {
        return Err(format!(
            "O {} já recebe {} por plugin (pluginProvided no sync.json)",
            cli.label(),
            name
        ));
    }
    let scope_name = match scope {
        Scope::Global => "global".to_string(),
        Scope::Project { path } => project_name(Path::new(path)),
    };
    let mut list: BTreeSet<String> = snap
        .sync
        .cli_off
        .get(&scope_name)
        .and_then(|c| c.get(&cli))
        .cloned()
        .unwrap_or_default();
    let changed = if enabled {
        list.remove(name)
    } else {
        list.insert(name.to_string())
    };
    if !changed {
        return Ok(vec![]);
    }
    let (before, base) = sync_json_text(snap);
    let value = (!list.is_empty()).then(|| json!(list.into_iter().collect::<Vec<_>>()));
    let mut after = set_key(&base, &["cliOff", &scope_name], cli.key(), value.as_ref())?;
    // tidy: drop empty containers
    let doc: Value = serde_json::from_str(&after).map_err(|e| e.to_string())?;
    if doc["cliOff"][&scope_name]
        .as_object()
        .map(|m| m.is_empty())
        .unwrap_or(false)
    {
        after = set_key(&after, &["cliOff"], &scope_name, None)?;
    }
    let doc: Value = serde_json::from_str(&after).map_err(|e| e.to_string())?;
    if doc["cliOff"]
        .as_object()
        .map(|m| m.is_empty())
        .unwrap_or(false)
    {
        after = set_key(&after, &[], "cliOff", None)?;
    }
    Ok(vec![text_change(
        snap.agents.sync_file.clone(),
        before,
        after,
        scope.clone(),
        vec![name.into()],
    )])
}

/// Add/remove a project in `sync.json` `projects` (stored as `~/...`).
pub fn plan_manage_project(
    snap: &SourceSnapshot,
    project: &Path,
    managed: bool,
) -> Result<Vec<FileChange>, String> {
    if managed && !project.is_dir() {
        return Err(format!("{} não existe", project.display()));
    }
    let entry = contract_home(project, &snap.home);
    let mut list = snap.sync.projects.clone();
    let present: Vec<usize> = list
        .iter()
        .enumerate()
        .filter(|(_, p)| super::source::expand_home(p, &snap.home) == project)
        .map(|(i, _)| i)
        .collect();
    match (managed, present.is_empty()) {
        (true, true) => list.push(entry),
        (false, false) => {
            for i in present.into_iter().rev() {
                list.remove(i);
            }
        }
        _ => return Ok(vec![]),
    }
    let (before, base) = sync_json_text(snap);
    let after = set_key(&base, &[], "projects", Some(&json!(list)))?;
    Ok(vec![text_change(
        snap.agents.sync_file.clone(),
        before,
        after,
        Scope::Global,
        vec![],
    )])
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOutcome {
    #[serde(flatten)]
    pub apply: ApplyOutcome,
    /// CLIs whose files changed (Claude counts when its source changed).
    pub clis: Vec<Cli>,
    /// The project is not in sync.json: only Claude (which reads the
    /// project's own files) sees the change.
    pub unmanaged_project: bool,
}

/// Write `source_changes`, re-read the source and regenerate the CLI files
/// of `scope` (global, or one project with its worktrees), as one set.
pub fn save_and_sync(
    home: &Path,
    backup_root: &Path,
    source_changes: Vec<FileChange>,
    scope: &Scope,
) -> Result<SaveOutcome, String> {
    let mut set = BackupSet::new(backup_root).map_err(|e| e.to_string())?;
    let mut clis: BTreeSet<Cli> = BTreeSet::new();
    if !source_changes.is_empty() {
        set.apply(&source_changes).map_err(|e| e.to_string())?;
        let touched_claude = source_changes.iter().any(|c| {
            let p = c.path.to_string_lossy();
            p.ends_with(".mcp.json")
                || p.ends_with("settings.json")
                || p.ends_with("settings.local.json")
        });
        if touched_claude && matches!(scope, Scope::Project { .. }) {
            clis.insert(Cli::Claude);
        }
    }
    let snap = SourceSnapshot::load(home);
    let eng = Engine::new(&snap);
    let mut plan = super::engine::Plan::default();
    let mut unmanaged = false;
    match scope {
        Scope::Global => eng.plan_global(&mut plan),
        Scope::Project { path } => {
            let p = PathBuf::from(path);
            let managed = snap
                .sync
                .project_paths(home)
                .iter()
                .any(|m| m == &p || matches!((std::fs::canonicalize(m), std::fs::canonicalize(&p)), (Ok(a), Ok(b)) if a == b));
            if managed {
                eng.plan_project(&p, &mut plan, false, None);
            } else {
                unmanaged = true;
            }
        }
    }
    set.apply(&plan.changes).map_err(|e| e.to_string())?;
    let out = set.finish().map_err(|e| e.to_string())?;
    for ch in &plan.changes {
        if out.applied.iter().any(|a| Path::new(a) == ch.path) {
            clis.insert(ch.cli);
        }
    }
    Ok(SaveOutcome {
        apply: out,
        clis: clis.into_iter().collect(),
        unmanaged_project: unmanaged,
    })
}

// ------------------------------------------------------------------ view

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectServer {
    pub name: String,
    /// Comes from the global source (shown greyed out as "herdado").
    pub inherited: bool,
    /// A project server with the same name replaces the global one.
    pub overrides_global: bool,
    pub transport: String,
    pub targets: std::collections::BTreeMap<Cli, super::model::TargetState>,
    pub secret_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeStatus {
    pub path: String,
    pub label: String,
    pub agy_in_sync: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSyncView {
    pub path: String,
    pub name: String,
    pub managed: bool,
    /// Pending changes per CLI for this project.
    pub pending: Vec<Cli>,
    pub servers: Vec<ProjectServer>,
    pub worktrees: Vec<WorktreeStatus>,
    pub report: Vec<super::engine::ReportItem>,
}

pub fn project_view(snap: &SourceSnapshot, eng: &Engine, project: &Path) -> ProjectSyncView {
    let managed = snap.projects.iter().any(|p| p.path == project);
    let resources = snap.resources();
    let scope_key = Scope::Project {
        path: project.to_string_lossy().to_string(),
    }
    .key();
    let own: Vec<&super::model::Resource> = resources
        .iter()
        .filter(|r| r.kind == super::model::ResourceKind::Mcp && r.scope.key() == scope_key)
        .collect();
    let own_names: BTreeSet<&str> = own.iter().map(|r| r.name.as_str()).collect();
    let mut servers: Vec<ProjectServer> = Vec::new();
    // project not managed: read its own .mcp.json for the list anyway
    let unmanaged_src = (!managed).then(|| super::source::ProjectSource::load(project));
    if let Some(ps) = &unmanaged_src {
        for (name, cfg) in &ps.servers {
            let mut targets = std::collections::BTreeMap::new();
            targets.insert(
                Cli::Claude,
                if ps.disabled.contains(name) || ps.disabled_claude_only.contains(name) {
                    super::model::TargetState::Off
                } else {
                    super::model::TargetState::On
                },
            );
            for cli in [Cli::Codex, Cli::Agy] {
                targets.insert(cli, super::model::TargetState::Off);
            }
            servers.push(ProjectServer {
                name: name.clone(),
                inherited: false,
                overrides_global: snap.global.contains_key(name),
                transport: super::model::mcp_transport(cfg),
                targets,
                secret_refs: vec![],
            });
        }
    }
    for r in &own {
        servers.push(ProjectServer {
            name: r.name.clone(),
            inherited: false,
            overrides_global: snap.global.contains_key(&r.name),
            transport: r.transport.clone().unwrap_or_default(),
            targets: r.targets.clone(),
            secret_refs: r.secret_refs.clone(),
        });
    }
    let local_names: BTreeSet<String> = servers.iter().map(|s| s.name.clone()).collect();
    for r in resources
        .iter()
        .filter(|r| r.kind == super::model::ResourceKind::Mcp && r.scope == Scope::Global)
    {
        if own_names.contains(r.name.as_str()) || local_names.contains(&r.name) {
            continue;
        }
        servers.push(ProjectServer {
            name: r.name.clone(),
            inherited: true,
            overrides_global: false,
            transport: r.transport.clone().unwrap_or_default(),
            targets: r.targets.clone(),
            secret_refs: r.secret_refs.clone(),
        });
    }
    let mut plan = super::engine::Plan::default();
    let mut worktrees = Vec::new();
    if managed {
        eng.plan_project(project, &mut plan, false, None);
        for wt in super::engine::worktrees(project) {
            let label = if wt == project {
                "main".to_string()
            } else {
                project_name(&wt)
            };
            let agy_in_sync = !plan.changes.iter().any(|c| {
                c.cli == Cli::Agy && c.path.starts_with(&wt) && c.label == format!("agy@{}", label)
            });
            worktrees.push(WorktreeStatus {
                path: wt.to_string_lossy().to_string(),
                label,
                agy_in_sync,
            });
        }
    }
    let pending: BTreeSet<Cli> = plan.changes.iter().map(|c| c.cli).collect();
    ProjectSyncView {
        path: project.to_string_lossy().to_string(),
        name: project_name(project),
        managed,
        pending: pending.into_iter().collect(),
        servers,
        worktrees,
        report: plan.report,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::model::TargetState;
    use tempfile::TempDir;

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    fn fx() -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let proj = home.join("w/proj");
        write(
            &home.join(".agents/mcp/global.json"),
            "{\n  \"mcpServers\": {\n    \"ctx\": { \"type\": \"stdio\", \"command\": \"npx\", \"args\": [\"ctx\"] }\n  }\n}\n",
        );
        write(
            &home.join(".agents/mcp/sync.json"),
            "{\n  \"projects\": [\n    \"~/w/proj\"\n  ],\n  \"codexGlobalKeep\": [\"node_repl\"]\n}\n",
        );
        write(&proj.join(".mcp.json"), "{\n  \"mcpServers\": {\n    \"sb\": {\n      \"type\": \"http\",\n      \"url\": \"https://sb\"\n    }\n  }\n}\n");
        write(&home.join(".codex/config.toml"), "model = \"x\"\n");
        (tmp, home, proj)
    }

    fn snap(home: &Path) -> SourceSnapshot {
        SourceSnapshot::load(home)
    }

    #[test]
    fn add_server_to_project_regenerates_codex_and_agy_and_undo_reverts_all() {
        let (_t, home, proj) = fx();
        let scope = Scope::Project {
            path: proj.to_string_lossy().to_string(),
        };
        let ch = plan_set_server(
            &snap(&home),
            &scope,
            "repowise",
            Some(&json!({"command": "repowise", "args": ["mcp"]})),
        )
        .unwrap();
        let out = save_and_sync(&home, &home.join("bk"), ch, &scope).unwrap();
        assert_eq!(out.clis, vec![Cli::Claude, Cli::Codex, Cli::Agy]);
        let codex = std::fs::read_to_string(proj.join(".codex/config.toml")).unwrap();
        assert!(codex.contains("[mcp_servers.repowise]"));
        assert!(proj.join(".agents/mcp_config.json").exists());
        // one undo reverts the source and the generated files
        crate::sync::engine::undo(&home.join("bk"), out.apply.backup_id.as_ref().unwrap()).unwrap();
        assert!(!std::fs::read_to_string(proj.join(".mcp.json"))
            .unwrap()
            .contains("repowise"));
        assert!(!proj.join(".codex/config.toml").exists());
        assert!(!proj.join(".agents/mcp_config.json").exists());
    }

    #[test]
    fn disable_in_project_uses_disabled_mcpjson_servers() {
        let (_t, home, proj) = fx();
        let scope = Scope::Project {
            path: proj.to_string_lossy().to_string(),
        };
        save_and_sync(&home, &home.join("bk"), vec![], &scope).unwrap();
        let ch = plan_set_enabled(&proj, "sb", false).unwrap();
        save_and_sync(&home, &home.join("bk"), ch, &scope).unwrap();
        let st: Value = serde_json::from_str(
            &std::fs::read_to_string(proj.join(".claude/settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(st["disabledMcpjsonServers"], json!(["sb"]));
        assert!(std::fs::read_to_string(proj.join(".codex/config.toml"))
            .unwrap()
            .contains("enabled = false"));
        assert!(
            !proj.join(".agents/mcp_config.json").exists(),
            "agy file removed when nothing is on"
        );
        // .mcp.json keeps the server (it is off, not deleted)
        assert!(std::fs::read_to_string(proj.join(".mcp.json"))
            .unwrap()
            .contains("\"sb\""));
    }

    #[test]
    fn per_cli_switches() {
        let (_t, home, proj) = fx();
        let pscope = Scope::Project {
            path: proj.to_string_lossy().to_string(),
        };
        // Claude only (project) → settings.local.json
        let ch = plan_set_target(&snap(&home), &pscope, "sb", Cli::Claude, false).unwrap();
        assert!(ch[0].path.ends_with(".claude/settings.local.json"));
        save_and_sync(&home, &home.join("bk"), ch, &pscope).unwrap();
        // agy only (project) → sync.json cliOff
        let ch = plan_set_target(&snap(&home), &pscope, "sb", Cli::Agy, false).unwrap();
        save_and_sync(&home, &home.join("bk"), ch, &pscope).unwrap();
        let res = snap(&home).resources();
        let sb = res.iter().find(|r| r.name == "sb").unwrap();
        assert_eq!(sb.targets[&Cli::Claude], TargetState::Off);
        assert_eq!(sb.targets[&Cli::Agy], TargetState::Off);
        assert_eq!(sb.targets[&Cli::Codex], TargetState::On);
        assert!(!proj.join(".agents/mcp_config.json").exists());
        // global, Codex only
        let ch = plan_set_target(&snap(&home), &Scope::Global, "ctx", Cli::Codex, false).unwrap();
        save_and_sync(&home, &home.join("bk"), ch, &Scope::Global).unwrap();
        assert!(!std::fs::read_to_string(home.join(".codex/config.toml"))
            .unwrap()
            .contains("[mcp_servers.ctx]"));
        let cj: Value =
            serde_json::from_str(&std::fs::read_to_string(home.join(".claude.json")).unwrap())
                .unwrap();
        assert!(
            cj["mcpServers"]["ctx"].is_object(),
            "other CLIs still get it"
        );
        // back on: cliOff cleaned up completely
        let ch = plan_set_target(&snap(&home), &Scope::Global, "ctx", Cli::Codex, true).unwrap();
        save_and_sync(&home, &home.join("bk"), ch, &Scope::Global).unwrap();
        let ch = plan_set_target(&snap(&home), &pscope, "sb", Cli::Agy, true).unwrap();
        save_and_sync(&home, &home.join("bk"), ch, &pscope).unwrap();
        let sync = std::fs::read_to_string(home.join(".agents/mcp/sync.json")).unwrap();
        assert!(!sync.contains("cliOff"), "{}", sync);
        assert_eq!(sync, "{\n  \"projects\": [\n    \"~/w/proj\"\n  ],\n  \"codexGlobalKeep\": [\"node_repl\"]\n}\n");
    }

    #[test]
    fn manage_project_and_view_with_inheritance() {
        let (_t, home, _proj) = fx();
        let other = home.join("w/other");
        write(
            &other.join(".mcp.json"),
            "{\"mcpServers\": {\"ctx\": {\"command\": \"local-ctx\"}}}",
        );
        let s = snap(&home);
        let eng = Engine::new(&s);
        let v = project_view(&s, &eng, &other);
        assert!(!v.managed);
        let ctx = v.servers.iter().find(|x| x.name == "ctx").unwrap();
        assert!(ctx.overrides_global && !ctx.inherited);
        // manage it
        let ch = plan_manage_project(&s, &other, true).unwrap();
        save_and_sync(&home, &home.join("bk"), ch, &Scope::Global).unwrap();
        let sync = std::fs::read_to_string(home.join(".agents/mcp/sync.json")).unwrap();
        assert!(
            sync.contains("    \"~/w/proj\",\n    \"~/w/other\"\n  ]"),
            "{}",
            sync
        );
        let s = snap(&home);
        let eng = Engine::new(&s);
        let v = project_view(&s, &eng, &home.join("w/proj"));
        assert!(v.managed);
        let inherited = v.servers.iter().find(|x| x.name == "ctx").unwrap();
        assert!(inherited.inherited);
        assert_eq!(v.worktrees.len(), 1);
        // unmanage
        let ch = plan_manage_project(&s, &other, false).unwrap();
        save_and_sync(&home, &home.join("bk"), ch, &Scope::Global).unwrap();
        assert!(!std::fs::read_to_string(home.join(".agents/mcp/sync.json"))
            .unwrap()
            .contains("other"));
    }

    #[test]
    fn plugin_provided_cannot_be_toggled_and_bad_names_rejected() {
        let (_t, home, _p) = fx();
        write(
            &home.join(".agents/mcp/sync.json"),
            "{\"projects\": [], \"pluginProvided\": {\"codex\": [\"ctx\"]}}",
        );
        assert!(plan_set_target(&snap(&home), &Scope::Global, "ctx", Cli::Codex, true).is_err());
        assert!(plan_set_server(&snap(&home), &Scope::Global, "a:b", Some(&json!({}))).is_err());
    }

    #[test]
    fn manage_with_project_scope_generates_files_in_one_undo_set() {
        let (_t, home, _p) = fx();
        let other = home.join("w/other");
        write(
            &other.join(".mcp.json"),
            "{\"mcpServers\": {\"x\": {\"command\": \"x\"}}}",
        );
        let ch = plan_manage_project(&snap(&home), &other, true).unwrap();
        let scope = Scope::Project {
            path: other.to_string_lossy().to_string(),
        };
        let out = save_and_sync(&home, &home.join("bk"), ch, &scope).unwrap();
        assert!(other.join(".agents/mcp_config.json").exists());
        assert!(other.join(".codex/config.toml").exists());
        crate::sync::engine::undo(&home.join("bk"), out.apply.backup_id.as_ref().unwrap()).unwrap();
        assert!(!other.join(".agents/mcp_config.json").exists());
        assert!(!std::fs::read_to_string(home.join(".agents/mcp/sync.json"))
            .unwrap()
            .contains("other"));
    }
}
