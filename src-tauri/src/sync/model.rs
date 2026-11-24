//! Resource model for multi-CLI sync.
//!
//! A *resource* (MCP server, skill, agent, hook) has its own identity
//! (`uid`), one canonical location (the source of truth, usually under
//! `~/.agents` or the project's `.mcp.json`) and a *target matrix*: for each
//! CLI, whether the resource is materialised there.
//!
//! The SQLite database only caches user-facing metadata keyed by `uid`
//! (favorites, tags, UI state) and where each resource was seen
//! (occurrences). It is never the source of a config.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The CLIs kept in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cli {
    Claude,
    Codex,
    Agy,
}

impl Cli {
    pub const ALL: [Cli; 3] = [Cli::Claude, Cli::Codex, Cli::Agy];

    /// Key used in `sync.json` (`overrides`, `pluginProvided`).
    pub fn key(self) -> &'static str {
        match self {
            Cli::Claude => "claude",
            Cli::Codex => "codex",
            Cli::Agy => "agy",
        }
    }

    pub fn from_key(key: &str) -> Option<Cli> {
        match key {
            "claude" | "claude_code" => Some(Cli::Claude),
            "codex" => Some(Cli::Codex),
            "agy" | "antigravity" => Some(Cli::Agy),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Cli::Claude => "Claude Code",
            Cli::Codex => "Codex",
            Cli::Agy => "Antigravity CLI",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceKind {
    Mcp,
    Skill,
    Agent,
    Hook,
}

impl ResourceKind {
    pub fn key(self) -> &'static str {
        match self {
            ResourceKind::Mcp => "mcp",
            ResourceKind::Skill => "skill",
            ResourceKind::Agent => "agent",
            ResourceKind::Hook => "hook",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Scope {
    Global,
    Project { path: String },
}

impl Scope {
    pub fn key(&self) -> String {
        match self {
            Scope::Global => "global".to_string(),
            Scope::Project { path } => format!("project:{}", normalize(path)),
        }
    }

    pub fn project_path(&self) -> Option<&str> {
        match self {
            Scope::Global => None,
            Scope::Project { path } => Some(path),
        }
    }
}

fn normalize(path: &str) -> String {
    let p = path.replace('\\', "/");
    p.trim_end_matches('/').to_string()
}

/// Stable identity: kind + scope + name. Names are case-sensitive (`Neon`
/// and `neon` are different resources), and the same name in two projects
/// gives two resources.
pub fn resource_uid(kind: ResourceKind, scope: &Scope, name: &str) -> String {
    format!("{}:{}:{}", kind.key(), scope.key(), name)
}

/// Whether a resource is materialised in one CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum TargetState {
    /// Generated/linked into the CLI.
    On,
    /// Turned off for this CLI (project `disabledMcpjsonServers`, unlinked skill...).
    Off,
    /// The CLI already gets it from its own plugin (`pluginProvided`).
    ProvidedByPlugin,
    /// The CLI reads the canonical location directly (nothing to generate).
    Native,
    /// The CLI cannot use it (e.g. Codex does not speak SSE).
    Unsupported { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub uid: String,
    pub kind: ResourceKind,
    pub name: String,
    pub scope: Scope,
    /// File or folder that is the source of truth for this resource.
    pub canonical: String,
    /// MCP: config exactly as in the source (`${VAR}` unresolved, unknown
    /// fields kept). Skills/agents: `None`.
    pub config: Option<Value>,
    /// MCP transport (`stdio`, `http`, `sse`).
    pub transport: Option<String>,
    pub description: Option<String>,
    pub targets: BTreeMap<Cli, TargetState>,
    /// Effective per-CLI field overrides (`overrides["*"]` + scope).
    pub overrides: BTreeMap<Cli, Value>,
    /// Names of `${VAR}` references (never their values).
    pub secret_refs: Vec<String>,
    /// Literal values that look like secrets (header/env names), reported so
    /// they can be moved to `secrets.env`. Only the key names.
    pub literal_secret_keys: Vec<String>,
}

pub fn var_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?::-([^}]*))?\}").unwrap())
}

/// Collect `${VAR}` names referenced anywhere in a JSON value.
pub fn collect_var_refs(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            for cap in var_regex().captures_iter(s) {
                let v = cap[1].to_string();
                if !out.contains(&v) {
                    out.push(v);
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|v| collect_var_refs(v, out)),
        Value::Object(m) => m.values().for_each(|v| collect_var_refs(v, out)),
        _ => {}
    }
}

/// Keys that contain "auth"/"key" but hold paths or URLs, not secrets.
pub fn is_non_secret_key(key: &str) -> bool {
    let k = key.to_ascii_uppercase();
    k == "XAUTHORITY"
        || ["_PATH", "_FILE", "_DIR", "_URL", "_HOST", "_PORT"]
            .iter()
            .any(|s| k.ends_with(s))
}

fn looks_secret_key(key: &str) -> bool {
    if is_non_secret_key(key) {
        return false;
    }
    let k = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "key",
        "password",
        "authorization",
        "auth",
        "bearer",
        "cookie",
    ]
    .iter()
    .any(|w| k.contains(w))
}

/// `env`/`headers` keys whose value is a literal (no `${VAR}`) and whose
/// name suggests a secret.
pub fn literal_secret_keys(cfg: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for field in ["env", "headers"] {
        if let Some(map) = cfg.get(field).and_then(|v| v.as_object()) {
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    if !s.is_empty() && !var_regex().is_match(s) && looks_secret_key(k) {
                        out.push(format!("{}.{}", field, k));
                    }
                }
            }
        }
    }
    out
}

/// MCP transport, same rule as mcp-sync: explicit `type`, else http when
/// there is a URL, else stdio.
pub fn mcp_transport(cfg: &Value) -> String {
    match cfg.get("type").and_then(|t| t.as_str()) {
        Some(t @ ("http" | "sse" | "stdio")) => t.to_string(),
        _ => {
            if cfg.get("url").is_some() || cfg.get("serverUrl").is_some() {
                "http".to_string()
            } else {
                "stdio".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn uid_is_case_sensitive_and_scoped() {
        let g = resource_uid(ResourceKind::Mcp, &Scope::Global, "Neon");
        let g2 = resource_uid(ResourceKind::Mcp, &Scope::Global, "neon");
        let p = resource_uid(
            ResourceKind::Mcp,
            &Scope::Project {
                path: "/w/proj/".into(),
            },
            "Neon",
        );
        assert_ne!(g, g2);
        assert_ne!(g, p);
        assert_eq!(p, "mcp:project:/w/proj:Neon");
    }

    #[test]
    fn windows_paths_normalize() {
        let a = Scope::Project {
            path: "C:\\w\\proj".into(),
        };
        assert_eq!(a.key(), "project:C:/w/proj");
    }

    #[test]
    fn collects_var_refs_without_values() {
        let cfg = json!({"headers": {"Authorization": "Bearer ${TOKEN}", "X": "${A:-x}"}, "args": ["${B}"]});
        let mut refs = Vec::new();
        collect_var_refs(&cfg, &mut refs);
        refs.sort();
        assert_eq!(refs, vec!["A", "B", "TOKEN"]);
    }

    #[test]
    fn literal_secret_detection() {
        let cfg = json!({"env": {"API_KEY": "sk-123", "OK": "${API}", "MODE": "dev", "XAUTHORITY": "/run/x", "KEY_FILE": "/k"}, "headers": {"Authorization": "Bearer abc"}});
        let mut keys = literal_secret_keys(&cfg);
        keys.sort();
        assert_eq!(keys, vec!["env.API_KEY", "headers.Authorization"]);
    }

    #[test]
    fn transport_rules() {
        assert_eq!(mcp_transport(&json!({"command": "x"})), "stdio");
        assert_eq!(mcp_transport(&json!({"url": "http://x"})), "http");
        assert_eq!(mcp_transport(&json!({"serverUrl": "http://x"})), "http");
        assert_eq!(mcp_transport(&json!({"type": "sse", "url": "x"})), "sse");
    }
}
