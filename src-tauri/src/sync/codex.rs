//! Codex adapter: Claude-format MCP config → `[mcp_servers.X]` tables.
//!
//! Faithful port of mcp-sync (`to_codex`, `toml_tables`, `codex_rewrite`) so
//! both tools generate byte-identical blocks (project `.codex/config.toml`
//! is versioned: no churn between the app and the script).

use super::model::{mcp_transport, var_regex};
use super::secrets::Secrets;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub const MARKER: &str =
    "# --- gerado pelo mcp-sync (~/.agents/bin/mcp-sync): edite o .mcp.json / global.json, não aqui ---";

/// Keys owned by the user/Codex, copied from the existing table.
pub const CODEX_EXTRAS: &[&str] = &[
    "startup_timeout_sec",
    "tool_timeout_sec",
    "enabled_tools",
    "disabled_tools",
    "cwd",
    "required",
    "default_tools_approval_mode",
    "tools",
    "scopes",
    "oauth_resource",
];

fn exact_var() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\$\{([A-Za-z_][A-Za-z0-9_]*)\}$").unwrap())
}

fn bearer_var() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^Bearer \$\{([A-Za-z_][A-Za-z0-9_]*)\}$").unwrap())
}

fn str_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// `tracked = true`: file goes to git, only variable references are allowed
/// (Codex forwards them). `false`: private file, values are resolved.
pub fn to_codex(
    name: &str,
    cfg: &Value,
    disabled: bool,
    old: Option<&Value>,
    tracked: bool,
    secrets: &Secrets,
    warns: &mut Vec<String>,
) -> Option<Map<String, Value>> {
    let mut out = Map::new();
    let mut missing = BTreeSet::new();
    let k = mcp_transport(cfg);
    if k == "sse" {
        warns.push(format!(
            "{}: Codex não fala SSE; ficou de fora do Codex",
            name
        ));
        return None;
    }
    if k == "stdio" {
        out.insert(
            "command".into(),
            cfg.get("command")
                .cloned()
                .unwrap_or(Value::String(String::new())),
        );
        if let Some(args) = cfg
            .get("args")
            .and_then(|a| a.as_array())
            .filter(|a| !a.is_empty())
        {
            let mut args = Value::Array(args.clone());
            let has_var = args
                .as_array()
                .unwrap()
                .iter()
                .any(|a| var_regex().is_match(&str_of(a)));
            if has_var {
                if tracked {
                    warns.push(format!(
                        "{}: ${{VAR}} em args não funciona no Codex; use env",
                        name
                    ));
                } else {
                    args = secrets.resolve(&args, &mut missing);
                }
            }
            out.insert("args".into(), args);
        }
        let mut env = Map::new();
        let mut env_vars = Vec::new();
        if let Some(e) = cfg.get("env").and_then(|e| e.as_object()) {
            for (ek, ev) in e {
                let evs = str_of(ev);
                let exact = exact_var().captures(&evs);
                if tracked && exact.as_ref().map(|m| &m[1] == ek).unwrap_or(false) {
                    env_vars.push(Value::String(ek.clone()));
                } else if var_regex().is_match(&evs) {
                    if tracked {
                        warns.push(format!(
                            "{}: env {} precisa ser exatamente ${{{}}} pro Codex repassar",
                            name, ek, ek
                        ));
                    } else {
                        env.insert(
                            ek.clone(),
                            Value::String(secrets.resolve_str(&evs, &mut missing)),
                        );
                    }
                } else {
                    env.insert(ek.clone(), ev.clone());
                }
            }
        }
        if !env.is_empty() {
            out.insert("env".into(), Value::Object(env));
        }
        if !env_vars.is_empty() {
            out.insert("env_vars".into(), Value::Array(env_vars));
        }
    } else {
        let mut url = cfg
            .get("url")
            .or_else(|| cfg.get("serverUrl"))
            .map(str_of)
            .unwrap_or_default();
        if var_regex().is_match(&url) {
            if tracked {
                warns.push(format!("{}: ${{VAR}} na URL não funciona no Codex", name));
            }
            url = secrets.resolve_str(&url, &mut missing);
        }
        out.insert("url".into(), Value::String(url));
        let mut lit = Map::new();
        let mut envh = Map::new();
        if let Some(h) = cfg.get("headers").and_then(|h| h.as_object()) {
            for (hk, hv) in h {
                let hvs = str_of(hv);
                let m = exact_var().captures(&hvs);
                let b = bearer_var().captures(&hvs);
                if tracked && b.is_some() && hk.eq_ignore_ascii_case("authorization") {
                    out.insert(
                        "bearer_token_env_var".into(),
                        Value::String(b.unwrap()[1].to_string()),
                    );
                } else if tracked && m.is_some() {
                    envh.insert(hk.clone(), Value::String(m.unwrap()[1].to_string()));
                } else if var_regex().is_match(&hvs) {
                    if tracked {
                        warns.push(format!(
                            "{}: header {} precisa ser ${{VAR}} ou 'Bearer ${{VAR}}' pro Codex",
                            name, hk
                        ));
                    } else {
                        lit.insert(
                            hk.clone(),
                            Value::String(secrets.resolve_str(&hvs, &mut missing)),
                        );
                    }
                } else {
                    lit.insert(hk.clone(), hv.clone());
                }
            }
        }
        if !lit.is_empty() {
            out.insert("http_headers".into(), Value::Object(lit));
        }
        if !envh.is_empty() {
            out.insert("env_http_headers".into(), Value::Object(envh));
        }
    }
    if let Some(old) = old.and_then(|o| o.as_object()) {
        for x in CODEX_EXTRAS {
            if let Some(v) = old.get(*x) {
                out.insert((*x).to_string(), v.clone());
            }
        }
    }
    if disabled {
        out.insert("enabled".into(), Value::Bool(false));
    }
    for v in missing {
        warns.push(format!("{}: falta {} no secrets.env", name, v));
    }
    Some(out)
}

fn bare_key() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9_-]+$").unwrap())
}

pub fn toml_key(k: &str) -> String {
    if bare_key().is_match(k) {
        k.to_string()
    } else {
        serde_json::to_string(k).unwrap()
    }
}

pub fn toml_val(v: &Value) -> String {
    match v {
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap(),
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(toml_val).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(m) => format!(
            "{{ {} }}",
            m.iter()
                .map(|(k, x)| format!("{} = {}", toml_key(k), toml_val(x)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Null => "\"\"".to_string(),
    }
}

pub fn toml_tables(servers: &[(String, Map<String, Value>)]) -> String {
    servers
        .iter()
        .map(|(name, cfg)| {
            let mut lines = vec![format!("[mcp_servers.{}]", toml_key(name))];
            lines.extend(
                cfg.iter()
                    .map(|(k, v)| format!("{} = {}", toml_key(k), toml_val(v))),
            );
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn header_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*\[\[?\s*(.+?)\s*\]\]?\s*(#.*)?$").unwrap())
}

fn server_key_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"^mcp_servers\.(?:"([^"]+)"|([A-Za-z0-9_-]+))"#).unwrap())
}

/// (is_header, server name if the header belongs to `[mcp_servers.X...]`)
pub fn header_server(line: &str) -> (bool, Option<String>) {
    let Some(m) = header_re().captures(line) else {
        return (false, None);
    };
    let key = &m[1];
    let srv = server_key_re()
        .captures(key)
        .map(|mm| mm.get(1).or(mm.get(2)).unwrap().as_str().to_string());
    (true, srv)
}

/// Drop `[mcp_servers.X]` tables of `drop` (None = all) and append the
/// generated ones after the marker. Everything else is kept verbatim.
pub fn codex_rewrite(
    text: &str,
    drop: Option<&BTreeSet<String>>,
    servers: &[(String, Map<String, Value>)],
) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in text.lines() {
        if line.trim() == MARKER {
            continue;
        }
        let (is_h, srv) = header_server(line);
        if is_h {
            skipping = match &srv {
                Some(s) => drop.map(|d| d.contains(s)).unwrap_or(true),
                None => false,
            };
        }
        if !skipping {
            out.push(line);
        }
    }
    let mut body = out.join("\n").trim_end().to_string();
    if !servers.is_empty() {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str(MARKER);
        body.push('\n');
        body.push_str(&toml_tables(servers));
    }
    body.push('\n');
    body
}

fn toml_to_json(v: &toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::from(*i),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => Value::Object(
            t.iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect(),
        ),
    }
}

/// Current `[mcp_servers]` of a Codex config as JSON values (comparison is
/// order-insensitive, like mcp-sync's dict equality). Missing file = empty; unparsable file = `Err` (never rewrite
/// a file we could not read).
pub fn codex_current(text: Option<&str>) -> Result<Map<String, Value>, String> {
    let Some(text) = text else {
        return Ok(Map::new());
    };
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    Ok(table
        .get("mcp_servers")
        .and_then(|s| s.as_table())
        .map(|t| {
            t.iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect()
        })
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn secrets() -> Secrets {
        let mut m = HashMap::new();
        m.insert("TOK".into(), "secret-value".into());
        Secrets::from_map(m)
    }

    #[test]
    fn tracked_uses_references_only() {
        let mut w = Vec::new();
        let out = to_codex(
            "sb",
            &json!({"type": "http", "url": "https://x", "headers": {"Authorization": "Bearer ${TOK}", "X-Ref": "${REF}"}}),
            false,
            None,
            true,
            &secrets(),
            &mut w,
        )
        .unwrap();
        assert_eq!(out["bearer_token_env_var"], "TOK");
        assert_eq!(out["env_http_headers"]["X-Ref"], "REF");
        assert!(!serde_json::to_string(&out)
            .unwrap()
            .contains("secret-value"));
        assert!(w.is_empty());
    }

    #[test]
    fn private_resolves_values_and_keeps_extras() {
        let mut w = Vec::new();
        let old = json!({"command": "old", "startup_timeout_sec": 30, "foo": 1});
        let out = to_codex(
            "s",
            &json!({"command": "npx", "args": ["a"], "env": {"K": "${TOK}", "L": "lit"}}),
            false,
            Some(&old),
            false,
            &secrets(),
            &mut w,
        )
        .unwrap();
        assert_eq!(out["env"]["K"], "secret-value");
        assert_eq!(out["startup_timeout_sec"], 30);
        assert!(out.get("foo").is_none(), "only known extras are copied");
        let keys: Vec<_> = out.keys().cloned().collect();
        assert_eq!(keys, vec!["command", "args", "env", "startup_timeout_sec"]);
    }

    #[test]
    fn sse_is_skipped_with_warning() {
        let mut w = Vec::new();
        assert!(to_codex(
            "e",
            &json!({"type": "sse", "url": "u"}),
            false,
            None,
            true,
            &secrets(),
            &mut w
        )
        .is_none());
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn tracked_warns_on_unforwardable_vars() {
        let mut w = Vec::new();
        to_codex(
            "s",
            &json!({"command": "x", "args": ["${A}"], "env": {"K": "pre-${B}"}}),
            false,
            None,
            true,
            &secrets(),
            &mut w,
        );
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn toml_rendering_matches_mcp_sync() {
        let mut m = Map::new();
        m.insert("command".into(), json!("npx"));
        m.insert("args".into(), json!(["-y", "pkg"]));
        m.insert("env".into(), json!({"A": "1", "B C": "2"}));
        m.insert("enabled".into(), json!(false));
        let t = toml_tables(&[("my.server".into(), m)]);
        assert_eq!(
            t,
            "[mcp_servers.\"my.server\"]\ncommand = \"npx\"\nargs = [\"-y\", \"pkg\"]\nenv = { A = \"1\", \"B C\" = \"2\" }\nenabled = false"
        );
    }

    #[test]
    fn rewrite_keeps_foreign_tables_and_moves_marker() {
        let text = format!(
            "model = \"x\"\n\n[mcp_servers.keep]\ncommand = \"k\"\n\n{}\n[mcp_servers.gen]\ncommand = \"old\"\n\n[mcp_servers.gen.env]\nA = \"1\"\n\n[tui]\ntheme = \"d\"\n",
            MARKER
        );
        let mut m = Map::new();
        m.insert("command".into(), json!("new"));
        let drop: BTreeSet<String> = ["gen".to_string()].into();
        let out = codex_rewrite(&text, Some(&drop), &[("gen".into(), m)]);
        assert!(out.contains("[mcp_servers.keep]\ncommand = \"k\""));
        assert!(out.contains("[tui]\ntheme = \"d\""));
        assert!(!out.contains("command = \"old\""));
        assert!(!out.contains("[mcp_servers.gen.env]"));
        assert_eq!(out.matches(MARKER).count(), 1);
        assert!(out.ends_with(&format!(
            "{}\n[mcp_servers.gen]\ncommand = \"new\"\n",
            MARKER
        )));
    }

    #[test]
    fn header_detection() {
        assert_eq!(header_server("[mcp_servers.a]"), (true, Some("a".into())));
        assert_eq!(
            header_server("[mcp_servers.\"a.b\"]  # c"),
            (true, Some("a.b".into()))
        );
        assert_eq!(
            header_server("[mcp_servers.a.env]"),
            (true, Some("a".into()))
        );
        assert_eq!(header_server("[[profiles]]"), (true, None));
        assert_eq!(header_server("key = 1"), (false, None));
    }

    #[test]
    fn current_reads_servers_and_rejects_garbage() {
        let cur = codex_current(Some(
            "[mcp_servers.b]\nurl = \"u\"\n[mcp_servers.a]\ncommand = \"c\"\n",
        ))
        .unwrap();
        let mut keys: Vec<_> = cur.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec!["a", "b"]);
        assert!(codex_current(Some("[[[")).is_err());
        assert!(codex_current(None).unwrap().is_empty());
    }
}
