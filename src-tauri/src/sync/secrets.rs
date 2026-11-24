//! `${VAR}` resolution against `~/.agents/mcp/secrets.env` (then the process
//! environment), same rules as mcp-sync.
//!
//! Values are kept in memory only to generate files that are *outside* any
//! git repo (Claude user scope, Codex global, agy). They are never
//! serialized, logged or sent to the UI: every text shown to the user goes
//! through [`Secrets::mask`].

use super::model::var_regex;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

#[derive(Default, Clone)]
pub struct Secrets {
    values: HashMap<String, String>,
    use_process_env: bool,
}

impl std::fmt::Debug for Secrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print values.
        f.debug_struct("Secrets")
            .field("count", &self.values.len())
            .finish()
    }
}

pub const MASK: &str = "••••••";

/// Parse a dotenv-style file: `KEY=value`, optional `export `, optional
/// matching quotes, `#` comments.
pub fn parse_env(text: &str) -> HashMap<String, String> {
    let mut vals = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let mut v = v.trim();
        if v.len() >= 2 {
            let b = v.as_bytes();
            if b[0] == b[v.len() - 1] && (b[0] == b'\'' || b[0] == b'"') {
                v = &v[1..v.len() - 1];
            }
        }
        vals.insert(k.trim().to_string(), v.to_string());
    }
    vals
}

impl Secrets {
    pub fn load(path: &Path) -> Self {
        let values = std::fs::read_to_string(path)
            .map(|t| parse_env(&t))
            .unwrap_or_default();
        Secrets {
            values,
            use_process_env: true,
        }
    }

    /// For tests: explicit values, no process environment.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn from_map(values: HashMap<String, String>) -> Self {
        Secrets {
            values,
            use_process_env: false,
        }
    }

    pub fn lookup(&self, var: &str) -> Option<String> {
        if let Some(v) = self.values.get(var) {
            return Some(v.clone());
        }
        if self.use_process_env {
            return std::env::var(var).ok();
        }
        None
    }

    pub fn has(&self, var: &str) -> bool {
        self.lookup(var).is_some()
    }

    /// Names defined in secrets.env (never values).
    pub fn names(&self) -> BTreeSet<String> {
        self.values.keys().cloned().collect()
    }

    /// Replace every `${VAR}` / `${VAR:-default}`; unresolved names go to
    /// `missing` and stay as written.
    pub fn resolve_str(&self, s: &str, missing: &mut BTreeSet<String>) -> String {
        var_regex()
            .replace_all(s, |c: &regex::Captures| match self.lookup(&c[1]) {
                Some(v) => v,
                None => match c.get(2) {
                    Some(d) => d.as_str().to_string(),
                    None => {
                        missing.insert(c[1].to_string());
                        c[0].to_string()
                    }
                },
            })
            .to_string()
    }

    pub fn resolve(&self, v: &Value, missing: &mut BTreeSet<String>) -> Value {
        match v {
            Value::String(s) => Value::String(self.resolve_str(s, missing)),
            Value::Array(a) => Value::Array(a.iter().map(|x| self.resolve(x, missing)).collect()),
            Value::Object(m) => Value::Object(
                m.iter()
                    .map(|(k, x)| (k.clone(), self.resolve(x, missing)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// Hide every known secret value in a text (diff previews, reports).
    pub fn mask(&self, text: &str) -> String {
        let mut vals: Vec<&String> = self.values.values().filter(|v| v.len() >= 4).collect();
        // longest first so overlapping values are fully hidden
        vals.sort_by_key(|v| std::cmp::Reverse(v.len()));
        let mut out = text.to_string();
        for v in vals {
            out = out.replace(v.as_str(), MASK);
        }
        out
    }
}

/// Mask values of header/env keys that look like secrets, for texts that
/// may contain literals not present in secrets.env.
pub fn mask_literal_secrets(text: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?i)("?[A-Za-z0-9_-]*(token|secret|key|password|authorization|bearer|cookie)[A-Za-z0-9_-]*"?\s*[:=]\s*")([^"$][^"]*)(")"#)
            .unwrap()
    });
    re.replace_all(text, |c: &regex::Captures| {
        format!("{}{}{}", &c[1], MASK, &c[4])
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn s() -> Secrets {
        let mut m = HashMap::new();
        m.insert("TOKEN".to_string(), "abcd1234".to_string());
        Secrets::from_map(m)
    }

    #[test]
    fn parses_dotenv() {
        let v = parse_env("# c\nexport A=1\nB = \"two words\"\nC='x'\nbad\n");
        assert_eq!(v["A"], "1");
        assert_eq!(v["B"], "two words");
        assert_eq!(v["C"], "x");
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn resolves_defaults_and_missing() {
        let mut miss = BTreeSet::new();
        let out = s().resolve(
            &json!({"h": "Bearer ${TOKEN}", "d": "${NOPE:-dflt}", "m": "${MISSING}"}),
            &mut miss,
        );
        assert_eq!(out["h"], "Bearer abcd1234");
        assert_eq!(out["d"], "dflt");
        assert_eq!(out["m"], "${MISSING}");
        assert_eq!(miss.into_iter().collect::<Vec<_>>(), vec!["MISSING"]);
    }

    #[test]
    fn masks_values() {
        assert_eq!(
            s().mask("Authorization: Bearer abcd1234"),
            format!("Authorization: Bearer {}", MASK)
        );
        assert!(!format!("{:?}", s()).contains("abcd1234"));
    }

    #[test]
    fn masks_literal_secret_like_keys() {
        let t =
            r#""API_KEY": "sk-live", "MODE": "dev", "Authorization": "Bearer x", "ref": "${A}""#;
        let m = mask_literal_secrets(t);
        assert!(!m.contains("sk-live"));
        assert!(!m.contains("Bearer x"));
        assert!(m.contains("\"MODE\": \"dev\""));
    }
}
