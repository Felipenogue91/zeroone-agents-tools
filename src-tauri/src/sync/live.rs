//! What each CLI really loads, asked to the CLI itself
//! (`claude mcp list`, `codex mcp list --json`, `agy mcp list`).
//!
//! The config files say what *should* load; this says what the CLI sees.
//! Notes: `codex mcp list` and `agy mcp list` ignore the project layer, so
//! they only answer for the global scope. `claude mcp list` health-checks
//! every server (a few seconds).

use super::model::Cli;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LiveState {
    /// Loaded (and, for Claude, connected).
    Connected,
    /// Loaded but enabled flag is on and we could not check the connection.
    Enabled,
    /// Listed but turned off in the CLI.
    Disabled,
    /// Listed, but the connection failed.
    Failed,
    /// Listed, needs login.
    NeedsAuth,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveServer {
    pub name: String,
    pub state: LiveState,
    /// Raw status text from the CLI.
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveCli {
    pub cli: Cli,
    /// `None` when the CLI could not be asked (not installed, timeout...).
    pub servers: Option<Vec<LiveServer>>,
    pub error: Option<String>,
    pub elapsed_ms: u64,
}

/// PATH with the usual user bin folders (the app may start from a desktop
/// launcher without the shell's PATH).
fn user_path(home: &Path) -> String {
    let mut dirs: Vec<PathBuf> = vec![
        home.join(".local/bin"),
        home.join(".local/share/mise/shims"),
        home.join(".npm-global/bin"),
        home.join(".bun/bin"),
        home.join(".cargo/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ];
    if let Ok(p) = std::env::var("PATH") {
        let mut cur: Vec<PathBuf> = std::env::split_paths(&p).collect();
        cur.append(&mut dirs);
        dirs = cur;
    }
    std::env::join_paths(dirs)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn run(
    home: &Path,
    cwd: &Path,
    prog: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String, String> {
    let mut child = Command::new(prog)
        .args(args)
        .current_dir(cwd)
        .env("PATH", user_path(home))
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{} não encontrado ({})", prog, e))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                return Err(format!("{} demorou mais de {}s", prog, timeout.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// `name: target - ✔ Connected` (name may hold `:` but not `: `).
pub fn parse_claude(text: &str) -> Vec<LiveServer> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(": ") else {
            continue;
        };
        let Some((_, status)) = rest.rsplit_once(" - ") else {
            continue;
        };
        let status = status.trim();
        let low = status.to_lowercase();
        let state = if low.contains("connected") && !low.contains("not connected") {
            LiveState::Connected
        } else if low.contains("auth") {
            LiveState::NeedsAuth
        } else if low.contains("disabled") {
            LiveState::Disabled
        } else {
            LiveState::Failed
        };
        out.push(LiveServer {
            name: name.trim().to_string(),
            state,
            detail: status
                .trim_start_matches(|c: char| !c.is_alphanumeric())
                .to_string(),
        });
    }
    out
}

pub fn parse_codex(text: &str) -> Vec<LiveServer> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![];
    };
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    let name = s.get("name")?.as_str()?.to_string();
                    let enabled = s.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                    let auth = s.get("auth_status").and_then(|e| e.as_str()).unwrap_or("");
                    Some(LiveServer {
                        name,
                        state: if !enabled {
                            LiveState::Disabled
                        } else if auth == "not_logged_in" {
                            LiveState::NeedsAuth
                        } else {
                            LiveState::Enabled
                        },
                        detail: if enabled {
                            "enabled".into()
                        } else {
                            "disabled".into()
                        },
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Table: `NAME  TYPE  STATUS  COMMAND/URL`.
pub fn parse_agy(text: &str) -> Vec<LiveServer> {
    text.lines()
        .filter(|l| !l.starts_with("NAME") && !l.trim().is_empty())
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?.to_string();
            let _kind = it.next()?;
            let status = it.next()?.to_string();
            Some(LiveServer {
                state: if status == "enabled" {
                    LiveState::Enabled
                } else {
                    LiveState::Disabled
                },
                name,
                detail: status,
            })
        })
        .collect()
}

fn ask(home: &Path, cwd: &Path, cli: Cli) -> LiveCli {
    let start = Instant::now();
    let res = match cli {
        Cli::Claude => run(
            home,
            cwd,
            "claude",
            &["mcp", "list"],
            Duration::from_secs(60),
        )
        .map(|t| parse_claude(&t)),
        Cli::Codex => run(
            home,
            cwd,
            "codex",
            &["mcp", "list", "--json"],
            Duration::from_secs(20),
        )
        .map(|t| parse_codex(&t)),
        Cli::Agy => {
            run(home, cwd, "agy", &["mcp", "list"], Duration::from_secs(20)).map(|t| parse_agy(&t))
        }
    };
    let elapsed_ms = start.elapsed().as_millis() as u64;
    match res {
        Ok(servers) => LiveCli {
            cli,
            servers: Some(servers),
            error: None,
            elapsed_ms,
        },
        Err(e) => LiveCli {
            cli,
            servers: None,
            error: Some(e),
            elapsed_ms,
        },
    }
}

/// Ask the three CLIs in parallel. `cwd` = project folder (Claude adds the
/// project's servers) or the home folder for the global view.
pub fn live_status(home: &Path, cwd: &Path) -> BTreeMap<Cli, LiveCli> {
    let handles: Vec<_> = [Cli::Claude, Cli::Codex, Cli::Agy]
        .into_iter()
        .map(|cli| {
            let (h, c) = (home.to_path_buf(), cwd.to_path_buf());
            std::thread::spawn(move || ask(&h, &c, cli))
        })
        .collect();
    handles
        .into_iter()
        .filter_map(|h| h.join().ok())
        .map(|l| (l.cli, l))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude_list() {
        let t = "Checking MCP server health…\n\nclaude.ai Github: https://api.githubcopilot.com/mcp - ✔ Connected\nplugin:neon:neon: https://mcp.neon.tech/mcp (HTTP) - ⚠ Needs authentication\nctx: npx ctx - ✗ Failed to connect\n";
        let s = parse_claude(t);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].name, "claude.ai Github");
        assert_eq!(s[0].state, LiveState::Connected);
        assert_eq!(s[1].name, "plugin:neon:neon");
        assert_eq!(s[1].state, LiveState::NeedsAuth);
        assert_eq!(s[2].state, LiveState::Failed);
        assert_eq!(s[2].detail, "Failed to connect");
    }

    #[test]
    fn parses_codex_json() {
        let t = r#"[{"name":"a","enabled":true,"auth_status":"unsupported"},{"name":"b","enabled":false}]"#;
        let s = parse_codex(t);
        assert_eq!(s[0].state, LiveState::Enabled);
        assert_eq!(s[1].state, LiveState::Disabled);
    }

    #[test]
    fn parses_agy_table() {
        let t = "NAME   TYPE   STATUS    COMMAND/URL\nNeon   http   disabled  https://x\nctx    stdio  enabled   /bin/ctx\nfigma  stdio  disabled  \n";
        let s = parse_agy(t);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].state, LiveState::Disabled);
        assert_eq!(s[1].name, "ctx");
        assert_eq!(s[1].state, LiveState::Enabled);
    }
}
