//! "Contexto" tab: what each CLI session loads into the model's context.
//! Claude Code first (read from its session logs); Codex and agy come later.

use crate::services::context_config::{self, ConfiguredHook, HookRun};
use crate::services::context_inspector::{self, ContextFolder, SessionContext, SessionRef};
use std::path::{Path, PathBuf};

fn home() -> Result<PathBuf, String> {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .ok_or_else(|| "pasta pessoal não encontrada".to_string())
}

/// `None` = sessions opened in the home folder (no project).
fn folder(project_path: Option<String>, home: &Path) -> PathBuf {
    project_path
        .filter(|p| !p.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.to_path_buf())
}

/// Folders where Claude Code was used, most recent first.
#[tauri::command]
pub async fn list_context_folders() -> Result<Vec<ContextFolder>, String> {
    tauri::async_runtime::spawn_blocking(move || Ok(context_inspector::list_folders(&home()?, 150)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_context_sessions(
    project_path: Option<String>,
) -> Result<Vec<SessionRef>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        let cwd = folder(project_path, &home);
        Ok(context_inspector::list_sessions(&home, &cwd, 20))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Context of one session; `session_id = None` picks the most recent.
/// `Ok(None)` when the folder has no Claude Code session yet.
#[tauri::command]
pub async fn get_session_context(
    project_path: Option<String>,
    session_id: Option<String>,
) -> Result<Option<SessionContext>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        let cwd = folder(project_path, &home);
        let id = match session_id {
            Some(id) => id,
            None => match context_inspector::list_sessions(&home, &cwd, 1)
                .into_iter()
                .next()
            {
                Some(s) => s.session_id,
                None => return Ok(None),
            },
        };
        let path = context_inspector::session_file(&home, &cwd, &id)
            .ok_or_else(|| format!("sessão {} não encontrada", id))?;
        let mut ctx = context_inspector::parse_session(&path).map_err(|e| e.to_string())?;
        ctx.on_demand = context_config::on_demand(
            &home,
            &cwd,
            &ctx.startup_labels("skills"),
            &ctx.startup_labels("agents"),
        );
        Ok(Some(ctx))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Hooks Claude Code has configured for this folder (settings + plugins).
#[tauri::command]
pub async fn list_context_hooks(
    project_path: Option<String>,
) -> Result<Vec<ConfiguredHook>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        let cwd = folder(project_path, &home);
        Ok(context_config::configured_hooks(&home, &cwd))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Run one configured hook by id (the command is re-read from the config,
/// never taken from the UI). Only on an explicit click.
#[tauri::command]
pub async fn run_context_hook(
    project_path: Option<String>,
    hook_id: String,
) -> Result<HookRun, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        let cwd = folder(project_path, &home);
        let hook = context_config::configured_hooks(&home, &cwd)
            .into_iter()
            .find(|h| h.id == hook_id)
            .ok_or_else(|| "hook não encontrado (a configuração mudou?)".to_string())?;
        context_config::run_hook(&home, &cwd, &hook)
    })
    .await
    .map_err(|e| e.to_string())?
}
