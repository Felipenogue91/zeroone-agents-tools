//! Secret vault: `~/.agents/mcp/secrets.env` (0600, outside any git repo).
//!
//! The app never returns a value: only names, whether they are defined,
//! referenced or missing. Writes keep comments and other lines intact.
//! Literal secrets found in the app database or in the source files can be
//! moved to the vault and replaced by `${VAR}` references, but only on an
//! explicit, confirmed request.

use super::model::{var_regex, ResourceKind};
use super::secrets::{parse_env, Secrets};
use super::source::SourceSnapshot;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

fn name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").unwrap())
}

pub fn valid_name(name: &str) -> bool {
    name_re().is_match(name)
}

#[cfg(unix)]
fn mode_of(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .ok()
        .map(|m| m.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn mode_of(_path: &Path) -> Option<u32> {
    None
}

#[cfg(unix)]
fn set_0600(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_0600(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretsStatus {
    pub file: String,
    pub exists: bool,
    /// Only the owner can read it (always true where modes do not apply).
    pub permissions_ok: bool,
    pub defined: Vec<String>,
    pub referenced: Vec<String>,
    /// Referenced by some MCP and not defined anywhere.
    pub missing: Vec<String>,
    /// Defined but not referenced by any source MCP.
    pub unused: Vec<String>,
}

pub fn status(snap: &SourceSnapshot) -> SecretsStatus {
    let file = &snap.agents.secrets_file;
    let secrets = Secrets::load(file);
    let defined = secrets.names();
    let mut referenced: BTreeSet<String> = BTreeSet::new();
    for r in snap
        .resources()
        .iter()
        .filter(|r| r.kind == ResourceKind::Mcp)
    {
        referenced.extend(r.secret_refs.iter().cloned());
    }
    // overrides may reference variables too
    let mut ov_refs = Vec::new();
    super::model::collect_var_refs(&snap.sync.raw, &mut ov_refs);
    referenced.extend(ov_refs);
    let missing = referenced
        .iter()
        .filter(|v| !secrets.has(v))
        .cloned()
        .collect();
    let unused = defined
        .iter()
        .filter(|v| !referenced.contains(*v))
        .cloned()
        .collect();
    SecretsStatus {
        file: file.to_string_lossy().to_string(),
        exists: file.is_file(),
        permissions_ok: mode_of(file).map(|m| m & 0o077 == 0).unwrap_or(true),
        defined: defined.into_iter().collect(),
        referenced: referenced.into_iter().collect(),
        missing,
        unused,
    }
}

/// New content of secrets.env with `name` set (or removed with `None`),
/// keeping comments, order and every other line.
pub fn edit_env_text(text: &str, name: &str, value: Option<&str>) -> Result<String, String> {
    if !valid_name(name) {
        return Err("nome de variável inválido (use letras, números e _)".into());
    }
    if let Some(v) = value {
        if v.contains('\n') || v.contains('\r') {
            return Err("o valor não pode ter quebra de linha".into());
        }
    }
    let render = |v: &str| {
        if v.is_empty()
            || v.chars()
                .any(|c| c.is_whitespace() || c == '#' || c == '"' || c == '\'')
        {
            format!("{}='{}'", name, v.replace('\'', "'\\''"))
        } else {
            format!("{}={}", name, v)
        }
    };
    let mut out: Vec<String> = Vec::new();
    let mut done = false;
    for line in text.lines() {
        let t = line.trim_start();
        let t = t.strip_prefix("export ").unwrap_or(t);
        let key = t.split('=').next().unwrap_or("").trim();
        if !t.starts_with('#') && t.contains('=') && key == name {
            if done {
                continue; // drop duplicates
            }
            done = true;
            if let Some(v) = value {
                out.push(render(v));
            }
            continue;
        }
        out.push(line.to_string());
    }
    if !done {
        if let Some(v) = value {
            out.push(render(v));
        }
    }
    let mut s = out.join("\n");
    if !s.is_empty() {
        s.push('\n');
    }
    Ok(s)
}

/// Set or remove one secret (atomic write, forced 0600).
pub fn set_secret(file: &Path, name: &str, value: Option<&str>) -> Result<(), String> {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let new = edit_env_text(&text, name, value)?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if !file.exists() {
        // create with restrictive permissions before any value is written
        std::fs::write(file, "").map_err(|e| e.to_string())?;
        set_0600(file).map_err(|e| e.to_string())?;
    }
    crate::utils::atomic::write_atomic(file, new).map_err(|e| e.to_string())?;
    set_0600(file).map_err(|e| e.to_string())
}

pub fn fix_permissions(file: &Path) -> Result<(), String> {
    if file.exists() {
        set_0600(file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ------------------------------------------------------------------ migration of literals

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralSecret {
    /// "db" (app database) or "source" (global.json / .mcp.json).
    pub location: String,
    pub mcp: String,
    /// "env" | "headers"
    pub field: String,
    pub key: String,
    /// Variable the value would move to.
    pub var: String,
    /// The value already exists in secrets.env under another name.
    pub already_in_vault: Option<String>,
    /// Source file (location = source).
    pub file: Option<String>,
}

fn looks_secret_key(key: &str) -> bool {
    if super::model::is_non_secret_key(key) {
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
        "credential",
    ]
    .iter()
    .any(|w| k.contains(w))
}

/// `MCP_NAME_KEY` in upper snake case.
pub fn suggest_var(mcp: &str, key: &str) -> String {
    let clean = |s: &str| {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
    };
    let key_part = if key.eq_ignore_ascii_case("authorization") {
        "TOKEN".to_string()
    } else {
        clean(key)
    };
    let mut v = format!("{}_{}", clean(mcp), key_part);
    while v.contains("__") {
        v = v.replace("__", "_");
    }
    let v = v.trim_matches('_').to_string();
    if v.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        format!("MCP_{}", v)
    } else {
        v
    }
}

/// Secret part of a value (`Bearer abc` → `abc`).
fn secret_part(v: &str) -> (&str, &str) {
    match v.strip_prefix("Bearer ") {
        Some(rest) => ("Bearer ", rest),
        None => ("", v),
    }
}

fn find_value_name(secrets_text: &str, value: &str) -> Option<String> {
    parse_env(secrets_text)
        .into_iter()
        .find(|(_, v)| v == value)
        .map(|(k, _)| k)
}

/// Literal secret-looking values in env/headers of one config.
fn literals_in(cfg: &Value) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for field in ["env", "headers"] {
        if let Some(map) = cfg.get(field).and_then(|v| v.as_object()) {
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    if !s.is_empty() && !var_regex().is_match(s) && looks_secret_key(k) {
                        out.push((field.to_string(), k.clone(), s.to_string()));
                    }
                }
            }
        }
    }
    out
}

/// Every literal secret in the DB and the source files (names only).
pub fn plan_migration(snap: &SourceSnapshot, db_rows: &[(String, Value)]) -> Vec<LiteralSecret> {
    let vault_text = std::fs::read_to_string(&snap.agents.secrets_file).unwrap_or_default();
    let mut out = Vec::new();
    let mut push = |location: &str, mcp: &str, file: Option<String>, cfg: &Value| {
        for (field, key, value) in literals_in(cfg) {
            let (_, secret) = secret_part(&value);
            out.push(LiteralSecret {
                location: location.into(),
                mcp: mcp.into(),
                var: suggest_var(mcp, &key),
                already_in_vault: find_value_name(&vault_text, secret),
                field,
                key,
                file: file.clone(),
            });
        }
    };
    for (name, cfg) in db_rows {
        push("db", name, None, cfg);
    }
    let global_file = snap.agents.global_mcp_file.to_string_lossy().to_string();
    for (name, cfg) in &snap.global {
        push("source", name, Some(global_file.clone()), cfg);
    }
    for p in &snap.projects {
        let f = p.path.join(".mcp.json").to_string_lossy().to_string();
        for (name, cfg) in &p.servers {
            push("source", name, Some(f.clone()), cfg);
        }
    }
    out
}

/// Replace literal secret values of a config by `${VAR}` and return the
/// (var, value) pairs to store. Values already in the vault reuse its name.
pub fn redact_config(mcp: &str, cfg: &Value, vault_text: &str) -> (Value, Vec<(String, String)>) {
    let mut cfg = cfg.clone();
    let mut to_store = Vec::new();
    for (field, key, value) in literals_in(&cfg) {
        let (prefix, secret) = secret_part(&value);
        let var = find_value_name(vault_text, secret).unwrap_or_else(|| {
            let v = suggest_var(mcp, &key);
            to_store.push((v.clone(), secret.to_string()));
            v
        });
        if let Some(m) = cfg.get_mut(&field).and_then(|m| m.as_object_mut()) {
            m.insert(key, Value::String(format!("{}${{{}}}", prefix, var)));
        }
    }
    (cfg, to_store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn edit_env_keeps_comments_and_order() {
        let t = "# tokens\nA=1\nexport B=2\n\nC='x y'\n";
        let out = edit_env_text(t, "B", Some("new")).unwrap();
        assert_eq!(out, "# tokens\nA=1\nB=new\n\nC='x y'\n");
        let out = edit_env_text(&out, "D", Some("with space")).unwrap();
        assert!(out.ends_with("D='with space'\n"));
        assert_eq!(parse_env(&out)["D"], "with space");
        let out = edit_env_text(&out, "A", None).unwrap();
        assert!(!out.contains("A=1"));
        assert!(edit_env_text(t, "bad-name", Some("x")).is_err());
        assert!(edit_env_text(t, "OK", Some("a\nb")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn set_secret_creates_0600_file() {
        let tmp = TempDir::new().unwrap();
        let f = tmp.path().join(".agents/mcp/secrets.env");
        set_secret(&f, "TOKEN", Some("abc")).unwrap();
        assert_eq!(mode_of(&f), Some(0o600));
        assert_eq!(
            parse_env(&std::fs::read_to_string(&f).unwrap())["TOKEN"],
            "abc"
        );
        set_secret(&f, "TOKEN", None).unwrap();
        assert!(!std::fs::read_to_string(&f).unwrap().contains("TOKEN"));
    }

    #[test]
    fn var_suggestions() {
        assert_eq!(suggest_var("icecat", "Api-Token"), "ICECAT_API_TOKEN");
        assert_eq!(suggest_var("my.mcp", "Authorization"), "MY_MCP_TOKEN");
        assert_eq!(suggest_var("1x", "KEY"), "MCP_1X_KEY");
    }

    #[test]
    fn redact_reuses_vault_names_and_keeps_bearer_prefix() {
        let cfg = json!({"headers": {"Authorization": "Bearer sk-1", "X-Mode": "dev"}, "env": {"API_KEY": "k-2"}});
        let (out, store) = redact_config("svc", &cfg, "EXISTING=k-2\n");
        assert_eq!(out["headers"]["Authorization"], "Bearer ${SVC_TOKEN}");
        assert_eq!(out["headers"]["X-Mode"], "dev");
        assert_eq!(out["env"]["API_KEY"], "${EXISTING}");
        assert_eq!(store, vec![("SVC_TOKEN".to_string(), "sk-1".to_string())]);
    }

    #[test]
    fn status_reports_missing_and_unused_without_values() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        std::fs::create_dir_all(home.join(".agents/mcp")).unwrap();
        std::fs::write(
            home.join(".agents/mcp/global.json"),
            json!({"mcpServers": {"a": {"url": "http://a", "headers": {"Authorization": "Bearer ${A_TOKEN}"}}}}).to_string(),
        )
        .unwrap();
        std::fs::write(
            home.join(".agents/mcp/secrets.env"),
            "UNUSED_ONE=zzz-secret\n",
        )
        .unwrap();
        let snap = SourceSnapshot::load(home);
        let st = status(&snap);
        assert_eq!(st.missing, vec!["A_TOKEN"]);
        assert_eq!(st.unused, vec!["UNUSED_ONE"]);
        assert!(!serde_json::to_string(&st).unwrap().contains("zzz-secret"));
    }
}
