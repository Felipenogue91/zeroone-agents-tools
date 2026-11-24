//! "Contexto" tab: what each CLI session loads into the model's context.
//! Claude Code, Codex and agy, each read from what it records on disk.

use crate::services::context_cli;
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

/// `claude` (default), `codex` or `agy`.
fn cli_of(cli: Option<String>) -> Result<String, String> {
    let cli = cli.unwrap_or_else(|| "claude".into());
    match cli.as_str() {
        "claude" | "codex" | "agy" => Ok(cli),
        other => Err(format!("CLI desconhecida: {}", other)),
    }
}

/// Folders where the CLI was used, most recent first.
#[tauri::command]
pub async fn list_context_folders(cli: Option<String>) -> Result<Vec<ContextFolder>, String> {
    let cli = cli_of(cli)?;
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        Ok(match cli.as_str() {
            "codex" => context_cli::codex_folders(&home, 150),
            "agy" => context_cli::agy_folders(&home, 150),
            _ => context_inspector::list_folders(&home, 150),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn sessions_of(cli: &str, home: &Path, cwd: &Path, limit: usize) -> Vec<SessionRef> {
    match cli {
        "codex" => context_cli::codex_sessions(home, cwd, limit),
        "agy" => context_cli::agy_sessions(home, cwd, limit),
        _ => context_inspector::list_sessions(home, cwd, limit),
    }
}

#[tauri::command]
pub async fn list_context_sessions(
    project_path: Option<String>,
    cli: Option<String>,
) -> Result<Vec<SessionRef>, String> {
    let cli = cli_of(cli)?;
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        let cwd = folder(project_path, &home);
        Ok(sessions_of(&cli, &home, &cwd, 20))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Context of one session; `session_id = None` picks the most recent.
/// `Ok(None)` when the folder has no session of that CLI yet.
#[tauri::command]
pub async fn get_session_context(
    project_path: Option<String>,
    session_id: Option<String>,
    cli: Option<String>,
) -> Result<Option<SessionContext>, String> {
    let cli = cli_of(cli)?;
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        let cwd = folder(project_path, &home);
        let id = match session_id {
            Some(id) => id,
            None => match sessions_of(&cli, &home, &cwd, 1).into_iter().next() {
                Some(s) => s.session_id,
                None => return Ok(None),
            },
        };
        let missing = || format!("sessão {} não encontrada", id);
        match cli.as_str() {
            "codex" => {
                let path = context_cli::codex_session_file(&home, &id).ok_or_else(missing)?;
                let ctx = context_cli::parse_codex(&path, &home).map_err(|e| e.to_string())?;
                Ok(Some(ctx))
            }
            "agy" => {
                let path = context_cli::agy_session_file(&home, &id).ok_or_else(missing)?;
                Ok(Some(context_cli::parse_agy(&path, &home)?))
            }
            _ => {
                let path = context_inspector::session_file(&home, &cwd, &id).ok_or_else(missing)?;
                let mut ctx = context_inspector::parse_session(&path).map_err(|e| e.to_string())?;
                ctx.on_demand = context_config::on_demand(
                    &home,
                    &cwd,
                    &ctx.startup_labels("skills"),
                    &ctx.startup_labels("agents"),
                );
                Ok(Some(ctx))
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Hooks the CLI has configured for this folder.
#[tauri::command]
pub async fn list_context_hooks(
    project_path: Option<String>,
    cli: Option<String>,
) -> Result<Vec<ConfiguredHook>, String> {
    let cli = cli_of(cli)?;
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        let cwd = folder(project_path, &home);
        Ok(context_config::configured_hooks_for(&cli, &home, &cwd))
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
    cli: Option<String>,
) -> Result<HookRun, String> {
    let cli = cli_of(cli)?;
    tauri::async_runtime::spawn_blocking(move || {
        let home = home()?;
        let cwd = folder(project_path, &home);
        let hook = context_config::configured_hooks_for(&cli, &home, &cwd)
            .into_iter()
            .find(|h| h.id == hook_id)
            .ok_or_else(|| "hook não encontrado (a configuração mudou?)".to_string())?;
        context_config::run_hook(&home, &cwd, &hook)
    })
    .await
    .map_err(|e| e.to_string())?
}
