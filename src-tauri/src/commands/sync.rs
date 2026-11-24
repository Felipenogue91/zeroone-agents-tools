//! Tauri commands for the multi-CLI resource model.

use crate::db::resources::ResourceMeta;
use crate::db::Database;
use crate::sync::engine::{self, ApplyOutcome, BackupInfo, Engine, FileChange, ReportItem};
use crate::sync::model::Cli;
use crate::sync::model::Resource;
use crate::sync::occurrences::{self, Occurrence};
use crate::sync::source::SourceSnapshot;
use directories::BaseDirs;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceView {
    #[serde(flatten)]
    pub resource: Resource,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub occurrences: Vec<Occurrence>,
}

pub(crate) fn home_dir() -> Result<PathBuf, String> {
    BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .ok_or_else(|| "Não foi possível descobrir a pasta do usuário".to_string())
}

/// Project folders to inspect: managed ones (sync.json) + the app's list.
pub(crate) fn known_projects(db: &Database, snap: &SourceSnapshot) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = snap.projects.iter().map(|p| p.path.clone()).collect();
    if let Ok(projects) = db.get_all_projects() {
        for p in projects {
            let path = PathBuf::from(&p.path);
            if path.is_dir() && !out.contains(&path) {
                out.push(path);
            }
        }
    }
    out
}

/// Rebuild the occurrence index from disk (read-only scan).
pub(crate) fn refresh_occurrences_in_db(db: &Database) -> Result<usize, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let occ = occurrences::collect(&home, &known_projects(db, &snap));
    db.replace_occurrences(&occ).map_err(|e| e.to_string())?;
    Ok(occ.len())
}

pub(crate) fn build_views(
    resources: Vec<Resource>,
    meta: &[ResourceMeta],
    occ: &[Occurrence],
) -> Vec<ResourceView> {
    resources
        .into_iter()
        .map(|r| {
            let m = meta.iter().find(|m| m.uid == r.uid);
            ResourceView {
                is_favorite: m.map(|m| m.is_favorite).unwrap_or(false),
                tags: m.map(|m| m.tags.clone()).unwrap_or_default(),
                occurrences: occ.iter().filter(|o| o.uid == r.uid).cloned().collect(),
                resource: r,
            }
        })
        .collect()
}

/// Every resource of the source of truth with its CLI target matrix,
/// cached metadata and where it was seen on disk.
#[tauri::command]
pub fn get_sync_resources(
    db: State<'_, Arc<Mutex<Database>>>,
) -> Result<Vec<ResourceView>, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let meta = db.get_resource_meta_all().map_err(|e| e.to_string())?;
    let occ = db.get_occurrences().map_err(|e| e.to_string())?;
    Ok(build_views(snap.resources(), &meta, &occ))
}

#[tauri::command]
pub fn refresh_sync_occurrences(db: State<'_, Arc<Mutex<Database>>>) -> Result<usize, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    refresh_occurrences_in_db(&db)
}

#[tauri::command]
pub fn set_resource_favorite(
    db: State<'_, Arc<Mutex<Database>>>,
    uid: String,
    favorite: bool,
) -> Result<(), String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.set_resource_favorite(&uid, favorite)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_resource_tags(
    db: State<'_, Arc<Mutex<Database>>>,
    uid: String,
    tags: Vec<String>,
) -> Result<(), String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.set_resource_tags(&uid, &tags).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ engine

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeView {
    pub cli: Cli,
    pub scope: crate::sync::model::Scope,
    pub label: String,
    pub path: String,
    /// "create" | "update" | "delete"
    pub action: String,
    pub servers: Vec<String>,
    /// Unified diff with every secret value masked.
    pub diff: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub changes: Vec<ChangeView>,
    pub report: Vec<ReportItem>,
    pub in_sync: bool,
}

pub(crate) const APP_IDENTIFIER: &str = "com.zeroone.agents-tools";

/// `<data dir>/com.zeroone.agents-tools/sync-backups` (same folder Tauri
/// uses for app data on Linux and Windows).
pub(crate) fn backup_root() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|d| d.join(APP_IDENTIFIER).join("sync-backups"))
        .ok_or_else(|| "Não foi possível descobrir a pasta de dados do app".to_string())
}

pub(crate) fn change_view(eng: &Engine, ch: &FileChange) -> ChangeView {
    ChangeView {
        cli: ch.cli,
        scope: ch.scope.clone(),
        label: ch.label.clone(),
        path: ch.path.to_string_lossy().to_string(),
        action: match (&ch.before, &ch.after) {
            (None, Some(_)) => "create",
            (_, None) => "delete",
            _ => "update",
        }
        .to_string(),
        servers: ch.servers.clone(),
        diff: eng.masked_diff(ch),
    }
}

pub(crate) fn plan_view(eng: &Engine, plan: &engine::Plan) -> PlanView {
    PlanView {
        changes: plan.changes.iter().map(|c| change_view(eng, c)).collect(),
        in_sync: plan.is_clean() && plan.report.iter().all(|r| r.status == "ok"),
        report: plan.report.clone(),
    }
}

/// Preview what a sync would change (nothing is written). `project`: only
/// that managed project; `None`: global + every managed project.
#[tauri::command]
pub fn sync_preview(project: Option<String>) -> Result<PlanView, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let eng = Engine::new(&snap);
    let plan = eng.plan(project.as_deref().map(std::path::Path::new));
    Ok(plan_view(&eng, &plan))
}

/// Apply the sync (re-planned now, each file re-checked) with backup.
#[tauri::command]
pub fn sync_apply(
    db: State<'_, Arc<Mutex<Database>>>,
    project: Option<String>,
) -> Result<ApplyOutcome, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let eng = Engine::new(&snap);
    let plan = eng.plan(project.as_deref().map(std::path::Path::new));
    let out = engine::apply(&plan.changes, &backup_root()?).map_err(|e| e.to_string())?;
    if let Ok(db) = db.lock() {
        let _ = refresh_occurrences_in_db(&db);
    }
    Ok(out)
}

#[tauri::command]
pub fn sync_undo(
    db: State<'_, Arc<Mutex<Database>>>,
    backup_id: String,
) -> Result<Vec<String>, String> {
    let restored = engine::undo(&backup_root()?, &backup_id).map_err(|e| e.to_string())?;
    if let Ok(db) = db.lock() {
        let _ = refresh_occurrences_in_db(&db);
    }
    Ok(restored)
}

#[tauri::command]
pub fn list_sync_backups() -> Result<Vec<BackupInfo>, String> {
    Ok(engine::list_backups(&backup_root()?))
}

/// Turn a shared skill (`~/.agents/skills/<name>`) on/off for one CLI.
#[tauri::command]
pub fn set_skill_target(
    db: State<'_, Arc<Mutex<Database>>>,
    name: String,
    cli: Cli,
    enabled: bool,
) -> Result<Option<ApplyOutcome>, String> {
    let home = home_dir()?;
    let agents = crate::utils::agents_home::AgentsHome::from_home(&home);
    let out = match cli {
        Cli::Claude => {
            crate::sync::skills::set_claude_skill(&home, &agents, &name, enabled)?;
            None
        }
        Cli::Agy => match crate::sync::skills::plan_agy_skill(&home, &agents, &name, enabled) {
            Some(ch) => Some(engine::apply(&[ch], &backup_root()?).map_err(|e| e.to_string())?),
            None => None,
        },
        Cli::Codex => {
            return Err(
                "O Codex lê ~/.agents/skills direto: não dá pra desligar a skill só no Codex."
                    .into(),
            )
        }
    };
    if let Ok(db) = db.lock() {
        let _ = refresh_occurrences_in_db(&db);
    }
    Ok(out)
}

/// Read-only cross-check with the reference script when it is installed
/// (`~/.agents/bin/mcp-sync --check --json`).
#[tauri::command]
pub fn run_external_sync_check() -> Result<Option<serde_json::Value>, String> {
    let home = home_dir()?;
    let agents = crate::utils::agents_home::AgentsHome::from_home(&home);
    if !agents.has_mcp_sync() {
        return Ok(None);
    }
    let out = std::process::Command::new(&agents.mcp_sync_bin)
        .args(["--check", "--json"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    serde_json::from_slice(&out.stdout)
        .map(Some)
        .map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ health

/// Divergences between the source and every CLI file, plus the engine
/// report. Read-only.
#[tauri::command]
pub fn get_sync_health(
    db: State<'_, Arc<Mutex<Database>>>,
) -> Result<crate::sync::health::Health, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let projects = {
        let db = db.lock().map_err(|e| e.to_string())?;
        known_projects(&db, &snap)
    };
    let occ = occurrences::collect(&home, &projects);
    if let Ok(db) = db.lock() {
        let _ = db.replace_occurrences(&occ);
    }
    let eng = Engine::new(&snap);
    let plan = eng.plan(None);
    let mut health = crate::sync::health::compute_health(&snap, &snap.resources(), &occ, &plan);
    for d in &mut health.divergences {
        d.message = eng.secrets.mask(&d.message);
    }
    for r in &mut health.report {
        r.summary = eng.secrets.mask(&r.summary);
        r.detail = r.detail.as_ref().map(|d| eng.secrets.mask(d));
    }
    Ok(health)
}

fn parse_scope(project: Option<String>) -> crate::sync::model::Scope {
    match project {
        Some(path) => crate::sync::model::Scope::Project { path },
        None => crate::sync::model::Scope::Global,
    }
}

/// Preview of a health action (masked diff), nothing written.
#[tauri::command]
pub fn health_action_preview(
    action: crate::sync::health::HealthAction,
    cli: Cli,
    project: Option<String>,
    name: String,
    file_path: String,
) -> Result<Vec<ChangeView>, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let eng = Engine::new(&snap);
    let changes = health_changes(&snap, &eng, action, cli, project, &name, &file_path)?;
    Ok(changes.iter().map(|c| change_view(&eng, c)).collect())
}

/// Run a health action (with backup; undo through `sync_undo`).
#[tauri::command]
pub fn health_action_apply(
    db: State<'_, Arc<Mutex<Database>>>,
    action: crate::sync::health::HealthAction,
    cli: Cli,
    project: Option<String>,
    name: String,
    file_path: String,
) -> Result<ApplyOutcome, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let eng = Engine::new(&snap);
    let changes = health_changes(&snap, &eng, action, cli, project, &name, &file_path)?;
    let out = engine::apply(&changes, &backup_root()?).map_err(|e| e.to_string())?;
    if let Ok(db) = db.lock() {
        let _ = refresh_occurrences_in_db(&db);
    }
    Ok(out)
}

fn health_changes(
    snap: &SourceSnapshot,
    eng: &Engine,
    action: crate::sync::health::HealthAction,
    cli: Cli,
    project: Option<String>,
    name: &str,
    file_path: &str,
) -> Result<Vec<FileChange>, String> {
    use crate::sync::health::{self, HealthAction};
    match action {
        HealthAction::Import => {
            let scope = parse_scope(project);
            Ok(vec![health::plan_import(
                snap,
                &eng.secrets,
                cli,
                &scope,
                std::path::Path::new(file_path),
                name,
            )?])
        }
        HealthAction::Reapply => {
            let plan = eng.plan(project.as_deref().map(std::path::Path::new));
            Ok(plan.changes.into_iter().filter(|c| c.cli == cli).collect())
        }
        HealthAction::RemoveLocalCopy => {
            let project = project.ok_or("RemoveLocalCopy precisa do projeto")?;
            Ok(
                health::plan_remove_local(snap, &project, &[name.to_string()])
                    .into_iter()
                    .collect(),
            )
        }
        HealthAction::FixAgySkills => Ok(crate::sync::skills::plan_agy_reads_shared(
            &snap.home,
            &snap.agents,
        )
        .into_iter()
        .collect()),
    }
}

// ------------------------------------------------------------------ projects / editing

/// Project screen data: own servers, inherited globals, worktrees, pending.
#[tauri::command]
pub fn get_project_sync(project: String) -> Result<crate::sync::projects::ProjectSyncView, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let eng = Engine::new(&snap);
    let mut view = crate::sync::projects::project_view(&snap, &eng, std::path::Path::new(&project));
    for r in &mut view.report {
        r.summary = eng.secrets.mask(&r.summary);
        r.detail = r.detail.as_ref().map(|d| eng.secrets.mask(d));
    }
    Ok(view)
}

fn save(
    db: &State<'_, Arc<Mutex<Database>>>,
    changes: Vec<FileChange>,
    scope: &crate::sync::model::Scope,
) -> Result<crate::sync::projects::SaveOutcome, String> {
    let home = home_dir()?;
    let out = crate::sync::projects::save_and_sync(&home, &backup_root()?, changes, scope)?;
    if let Ok(db) = db.lock() {
        let _ = refresh_occurrences_in_db(&db);
    }
    Ok(out)
}

/// Add/edit (`config`) or remove (`None`) a server in the source, then
/// regenerate the CLIs. `project = None` → global.
#[tauri::command]
pub fn sync_set_server(
    db: State<'_, Arc<Mutex<Database>>>,
    project: Option<String>,
    name: String,
    config: Option<serde_json::Value>,
) -> Result<crate::sync::projects::SaveOutcome, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let scope = parse_scope(project);
    let changes = crate::sync::projects::plan_set_server(&snap, &scope, &name, config.as_ref())?;
    save(&db, changes, &scope)
}

/// Project server on/off in the three CLIs (`disabledMcpjsonServers`).
#[tauri::command]
pub fn sync_set_enabled(
    db: State<'_, Arc<Mutex<Database>>>,
    project: String,
    name: String,
    enabled: bool,
) -> Result<crate::sync::projects::SaveOutcome, String> {
    let changes =
        crate::sync::projects::plan_set_enabled(std::path::Path::new(&project), &name, enabled)?;
    save(
        &db,
        changes,
        &crate::sync::model::Scope::Project { path: project },
    )
}

/// Server on/off in one CLI only.
#[tauri::command]
pub fn sync_set_target(
    db: State<'_, Arc<Mutex<Database>>>,
    project: Option<String>,
    name: String,
    cli: Cli,
    enabled: bool,
) -> Result<crate::sync::projects::SaveOutcome, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let scope = parse_scope(project);
    let changes = crate::sync::projects::plan_set_target(&snap, &scope, &name, cli, enabled)?;
    save(&db, changes, &scope)
}

/// Add/remove a project from `sync.json` (and generate its files).
#[tauri::command]
pub fn sync_manage_project(
    db: State<'_, Arc<Mutex<Database>>>,
    app: tauri::AppHandle,
    project: String,
    managed: bool,
) -> Result<crate::sync::projects::SaveOutcome, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let path = PathBuf::from(&project);
    let changes = crate::sync::projects::plan_manage_project(&snap, &path, managed)?;
    // project scope: sync.json edit + first generation of the project files
    // in the same undo set
    let out = save(
        &db,
        changes,
        &crate::sync::model::Scope::Project { path: project },
    )?;
    start_config_watcher(&app);
    Ok(out)
}

// ------------------------------------------------------------------ watcher

pub struct WatcherState(pub Mutex<Option<crate::sync::watcher::ConfigWatcher>>);

/// (Re)start the config watcher. Emits `sync-changed` with the changed
/// paths; restarts itself when `sync.json` changes (project list).
pub fn start_config_watcher(app: &tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    let Ok(home) = home_dir() else { return };
    let projects = {
        let snap = SourceSnapshot::load(&home);
        let db = app.state::<Arc<Mutex<Database>>>();
        let guard = db.lock();
        match guard {
            Ok(db) => known_projects(&db, &snap),
            Err(_) => snap.projects.iter().map(|p| p.path.clone()).collect(),
        }
    };
    let spec = crate::sync::watcher::watch_spec(&home, &projects);
    let handle = app.clone();
    let started =
        crate::sync::watcher::start(spec, std::time::Duration::from_millis(600), move |paths| {
            let db = handle.state::<Arc<Mutex<Database>>>();
            if let Ok(db) = db.lock() {
                let _ = refresh_occurrences_in_db(&db);
            }
            let list: Vec<String> = paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let _ = handle.emit("sync-changed", &list);
            if paths.iter().any(|p| p.ends_with("sync.json")) {
                let h = handle.clone();
                std::thread::spawn(move || start_config_watcher(&h));
            }
        });
    match started {
        Ok(w) => {
            if let Some(state) = app.try_state::<WatcherState>() {
                if let Ok(mut slot) = state.0.lock() {
                    *slot = Some(w);
                }
            }
        }
        Err(e) => log::warn!("[watcher] não iniciou: {}", e),
    }
}

// ------------------------------------------------------------------ CLI

/// Terminal entry points (no window):
/// - `--sync-here`: session hook, updates the agy file of the current folder;
/// - `--sync-check [--json]`: report only;
/// - `--sync-apply`: apply everything (with backup);
/// - `--sync-health [--json]`: divergences per CLI (read-only).
pub fn run_cli(args: &[String]) -> Option<i32> {
    let first = args.first()?.as_str();
    if !matches!(
        first,
        "--sync-here" | "--sync-check" | "--sync-apply" | "--sync-health"
    ) {
        return None;
    }
    let home = match home_dir() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("zeroone-agents-tools: {}", e);
            return Some(0);
        }
    };
    let snap = SourceSnapshot::load(&home);
    let eng = Engine::new(&snap);
    match first {
        "--sync-here" => {
            // a session hook must never block the session
            let cwd = std::env::current_dir().unwrap_or_else(|_| home.clone());
            let plan = eng.plan_here(&cwd);
            if !plan.changes.is_empty() {
                match backup_root().and_then(|r| engine::apply(&plan.changes, &r).map_err(|e| e.to_string())) {
                    Ok(_) => println!("zeroone-agents-tools: MCPs do projeto atualizados pro agy (vale na próxima sessão do agy)."),
                    Err(e) => eprintln!("zeroone-agents-tools: {}", e),
                }
            }
            Some(0)
        }
        "--sync-health" => {
            let projects: Vec<PathBuf> = snap.projects.iter().map(|p| p.path.clone()).collect();
            let occ = occurrences::collect(&home, &projects);
            let plan = eng.plan(None);
            let h = crate::sync::health::compute_health(&snap, &snap.resources(), &occ, &plan);
            if args.iter().any(|a| a == "--json") {
                println!(
                    "{}",
                    eng.secrets
                        .mask(&serde_json::to_string(&h).unwrap_or_default())
                );
            } else {
                println!(
                    "{} recursos · {}",
                    h.resources,
                    if h.in_sync {
                        "tudo sincronizado".to_string()
                    } else {
                        format!("{} divergências", h.divergences.len())
                    }
                );
                for d in &h.divergences {
                    println!(
                        "  [{:?}] {}  → {:?}",
                        d.kind,
                        eng.secrets.mask(&d.message),
                        d.actions
                    );
                }
            }
            Some(0)
        }
        "--sync-check" => {
            let plan = eng.plan(None);
            if args.iter().any(|a| a == "--json") {
                println!(
                    "{}",
                    serde_json::to_string(&plan.report).unwrap_or_default()
                );
            } else {
                for r in &plan.report {
                    println!(
                        "{} {}",
                        if r.status == "ok" { "✅" } else { "🟡" },
                        r.summary
                    );
                    if let Some(d) = &r.detail {
                        println!("   {}", eng.secrets.mask(d));
                    }
                }
            }
            Some(0)
        }
        _ => {
            let plan = eng.plan(None);
            match backup_root()
                .and_then(|r| engine::apply(&plan.changes, &r).map_err(|e| e.to_string()))
            {
                Ok(out) => {
                    for p in &out.applied {
                        println!("✅ {}", p);
                    }
                    for (p, why) in &out.skipped {
                        println!("🟡 {}: {}", p, why);
                    }
                    if let Some(id) = out.backup_id {
                        println!("backup: {}", id);
                    }
                    Some(0)
                }
                Err(e) => {
                    eprintln!("zeroone-agents-tools: {}", e);
                    Some(1)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::model::*;

    #[test]
    fn views_merge_meta_and_occurrences() {
        let r = Resource {
            uid: "mcp:global:a".into(),
            kind: ResourceKind::Mcp,
            name: "a".into(),
            scope: Scope::Global,
            canonical: "/h/.agents/mcp/global.json".into(),
            config: None,
            transport: Some("stdio".into()),
            description: None,
            targets: Default::default(),
            overrides: Default::default(),
            secret_refs: vec![],
            literal_secret_keys: vec![],
        };
        let meta = vec![ResourceMeta {
            uid: "mcp:global:a".into(),
            is_favorite: true,
            tags: vec!["t".into()],
            ui_state: None,
        }];
        let occ = vec![Occurrence {
            uid: "mcp:global:a".into(),
            kind: ResourceKind::Mcp,
            name: "a".into(),
            cli: Cli::Codex,
            scope: Scope::Global,
            file_path: "/h/.codex/config.toml".into(),
        }];
        let v = build_views(vec![r], &meta, &occ);
        assert!(v[0].is_favorite);
        assert_eq!(v[0].occurrences.len(), 1);
        let json = serde_json::to_value(&v[0]).unwrap();
        assert_eq!(json["uid"], "mcp:global:a");
        assert_eq!(json["isFavorite"], true);
    }
}
