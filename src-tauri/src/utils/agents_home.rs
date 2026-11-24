//! Detection of the shared `~/.agents` layout.
//!
//! `~/.agents` is the source of truth for cross-CLI configuration (Claude
//! Code, Codex, Antigravity CLI):
//!
//! ```text
//! ~/.agents/skills/<name>/      canonical skills (linked into each CLI)
//! ~/.agents/mcp/global.json     global MCPs, Claude `.mcp.json` format
//! ~/.agents/mcp/sync.json       managed projects + per-CLI overrides
//! ~/.agents/mcp/secrets.env     values for ${VAR} (never read by the app UI)
//! ~/.agents/bin/mcp-sync        optional generator script
//! ```
//!
//! Everything here is optional: when the directory does not exist the app
//! behaves exactly like before, with no error and no noise.

use directories::BaseDirs;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
#[allow(dead_code)] // sync/secrets paths are used by the sync engine (phase 2+)
pub struct AgentsHome {
    pub root: PathBuf,
    pub skills_dir: PathBuf,
    pub mcp_dir: PathBuf,
    pub global_mcp_file: PathBuf,
    pub sync_file: PathBuf,
    pub secrets_file: PathBuf,
    pub mcp_sync_bin: PathBuf,
}

impl AgentsHome {
    pub fn from_home(home: &Path) -> Self {
        let root = home.join(".agents");
        let mcp_dir = root.join("mcp");
        AgentsHome {
            skills_dir: root.join("skills"),
            global_mcp_file: mcp_dir.join("global.json"),
            sync_file: mcp_dir.join("sync.json"),
            secrets_file: mcp_dir.join("secrets.env"),
            mcp_sync_bin: root.join("bin").join("mcp-sync"),
            mcp_dir,
            root,
        }
    }

    /// MCP configs are owned by `~/.agents/mcp` (and its sync tool). The app
    /// must not write MCP servers into CLI config files directly.
    pub fn mcp_externally_managed(&self) -> bool {
        self.mcp_dir.is_dir()
    }

    pub fn has_mcp_sync(&self) -> bool {
        self.mcp_sync_bin.is_file()
    }
}

pub fn agents_home() -> Option<AgentsHome> {
    BaseDirs::new().map(|b| AgentsHome::from_home(b.home_dir()))
}

/// Message shown when a direct MCP write is refused.
pub const EXTERNAL_MCP_MESSAGE: &str = "Os MCPs desta máquina são gerenciados por ~/.agents/mcp (fonte da verdade). \
O app não grava MCPs direto nas CLIs: edite ~/.agents/mcp/global.json ou o .mcp.json do projeto e rode o mcp-sync.";

/// Returns `Err` with a user-facing message when MCP writes must be refused.
pub fn ensure_mcp_writes_allowed_in(home: &AgentsHome) -> Result<(), String> {
    if home.mcp_externally_managed() {
        log::warn!(
            "[agents_home] MCP write refused: managed by {}",
            home.mcp_dir.display()
        );
        return Err(EXTERNAL_MCP_MESSAGE.to_string());
    }
    Ok(())
}

pub fn ensure_mcp_writes_allowed() -> Result<(), String> {
    match agents_home() {
        Some(h) => ensure_mcp_writes_allowed_in(&h),
        None => Ok(()),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpManagementStatus {
    pub externally_managed: bool,
    pub agents_dir: Option<String>,
    pub global_source: Option<String>,
    pub sync_tool_available: bool,
    pub message: Option<String>,
}

pub fn mcp_management_status_in(home: Option<&AgentsHome>) -> McpManagementStatus {
    match home {
        Some(h) if h.mcp_externally_managed() => McpManagementStatus {
            externally_managed: true,
            agents_dir: Some(h.root.to_string_lossy().to_string()),
            global_source: h
                .global_mcp_file
                .is_file()
                .then(|| h.global_mcp_file.to_string_lossy().to_string()),
            sync_tool_available: h.has_mcp_sync(),
            message: Some(EXTERNAL_MCP_MESSAGE.to_string()),
        },
        Some(h) => McpManagementStatus {
            externally_managed: false,
            agents_dir: h
                .root
                .is_dir()
                .then(|| h.root.to_string_lossy().to_string()),
            global_source: None,
            sync_tool_available: false,
            message: None,
        },
        None => McpManagementStatus {
            externally_managed: false,
            agents_dir: None,
            global_source: None,
            sync_tool_available: false,
            message: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn absent_layout_is_not_managed() {
        let dir = TempDir::new().unwrap();
        let h = AgentsHome::from_home(dir.path());
        assert!(!h.mcp_externally_managed());
        assert!(ensure_mcp_writes_allowed_in(&h).is_ok());
        let st = mcp_management_status_in(Some(&h));
        assert!(!st.externally_managed);
        assert!(st.agents_dir.is_none());
    }

    #[test]
    fn skills_only_layout_is_not_managed() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".agents/skills")).unwrap();
        let h = AgentsHome::from_home(dir.path());
        assert!(!h.mcp_externally_managed());
        assert!(mcp_management_status_in(Some(&h)).agents_dir.is_some());
    }

    #[test]
    fn mcp_dir_means_managed() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".agents/mcp")).unwrap();
        std::fs::write(dir.path().join(".agents/mcp/global.json"), "{}").unwrap();
        let h = AgentsHome::from_home(dir.path());
        assert!(h.mcp_externally_managed());
        let err = ensure_mcp_writes_allowed_in(&h).unwrap_err();
        assert!(err.contains("~/.agents/mcp"));
        let st = mcp_management_status_in(Some(&h));
        assert!(st.externally_managed);
        assert!(st.global_source.is_some());
        assert!(!st.sync_tool_available);
    }
}
