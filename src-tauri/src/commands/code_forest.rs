//! "Code Forest" tab: the forest of projects (separate repo,
//! `~/Workspace/projects/code-forest`) shown inside the tab as an iframe of the
//! forest's local server.
//!
//! The forest only lets this app frame it (`frame-ancestors` lists the app's
//! origins; any other site stays blocked, which guards the prune button).
//! Nothing of the forest is bundled here; the tab only shows up when the repo
//! exists.

use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

const PORT: u16 = 4790;

fn forest_dir() -> Option<PathBuf> {
    let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
    let dir = home.join("Workspace/projects/code-forest");
    dir.join("bin/code-forest").is_file().then_some(dir)
}

fn url() -> String {
    format!("http://127.0.0.1:{}/", PORT)
}

fn is_up() -> bool {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
        .ok()
        .and_then(|c| c.get(url()).send().ok())
        .and_then(|r| r.text().ok())
        .map(|t| t.contains("<title>Code Forest</title>"))
        .unwrap_or(false)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeForestStatus {
    pub installed: bool,
    pub dir: Option<String>,
    pub url: String,
    pub up: bool,
}

#[tauri::command]
pub async fn code_forest_status() -> Result<CodeForestStatus, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let dir = forest_dir();
        CodeForestStatus {
            installed: dir.is_some(),
            up: dir.is_some() && is_up(),
            dir: dir.map(|d| d.to_string_lossy().to_string()),
            url: url(),
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Start the forest server if needed (its own `serve`, detached so it
/// outlives the app, like `code-forest open` does). The tab frames it after.
#[tauri::command]
pub async fn start_code_forest() -> Result<(), String> {
    let dir =
        forest_dir().ok_or("Code Forest não encontrado em ~/Workspace/projects/code-forest")?;
    let up = tauri::async_runtime::spawn_blocking(move || -> Result<bool, String> {
        if is_up() {
            return Ok(true);
        }
        let bin = dir.join("bin/code-forest");
        std::process::Command::new("setsid")
            .arg(&bin)
            .arg("serve")
            .current_dir(&dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("não consegui subir o Code Forest: {}", e))?;
        for _ in 0..60 {
            std::thread::sleep(Duration::from_millis(500));
            if is_up() {
                return Ok(true);
            }
        }
        Ok(false)
    })
    .await
    .map_err(|e| e.to_string())??;
    if !up {
        return Err(
            "O servidor do Code Forest não respondeu em 30 s (veja `code-forest status`).".into(),
        );
    }
    Ok(())
}
