//! Surgical edits of `mcpServers` in hand-formatted JSON source files
//! (`~/.agents/mcp/global.json`, `<project>/.mcp.json`).
//!
//! Re-serialising the whole document would reformat files the user writes
//! by hand. Here only the span of the touched server changes, rendered in
//! the style the file already uses (one-line entries or pretty blocks).

use serde_json::{Map, Value};

#[derive(Debug, Clone)]
struct Entry {
    name: String,
    /// Byte offset of the opening quote of the key.
    key_start: usize,
    value_start: usize,
    /// Exclusive end of the value.
    value_end: usize,
}

struct Scan<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Scan<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && (self.b[self.i] as char).is_whitespace() {
            self.i += 1;
        }
    }
    fn string(&mut self) -> Result<String, String> {
        let start = self.i;
        if self.b.get(self.i) != Some(&b'"') {
            return Err(format!("esperava string na posição {}", self.i));
        }
        self.i += 1;
        while self.i < self.b.len() {
            match self.b[self.i] {
                b'\\' => self.i += 2,
                b'"' => {
                    self.i += 1;
                    let raw =
                        std::str::from_utf8(&self.b[start..self.i]).map_err(|e| e.to_string())?;
                    return serde_json::from_str(raw).map_err(|e| e.to_string());
                }
                _ => self.i += 1,
            }
        }
        Err("string sem fim".into())
    }
    /// Skip any JSON value.
    fn value(&mut self) -> Result<(), String> {
        self.ws();
        match self.b.get(self.i) {
            Some(b'"') => self.string().map(|_| ()),
            Some(b'{') | Some(b'[') => {
                let mut depth = 0i32;
                while self.i < self.b.len() {
                    match self.b[self.i] {
                        b'"' => {
                            self.string()?;
                            continue;
                        }
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                self.i += 1;
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                    self.i += 1;
                }
                Err("objeto sem fim".into())
            }
            Some(_) => {
                while self.i < self.b.len() && !matches!(self.b[self.i], b',' | b'}' | b']') {
                    self.i += 1;
                }
                Ok(())
            }
            None => Err("fim inesperado".into()),
        }
    }
}

/// Locate the `mcpServers` object: (open brace, close brace, entries).
fn locate(text: &str) -> Result<Option<(usize, usize, Vec<Entry>)>, String> {
    let b = text.as_bytes();
    let mut s = Scan { b, i: 0 };
    s.ws();
    if b.get(s.i) != Some(&b'{') {
        return Err("o arquivo não é um objeto JSON".into());
    }
    s.i += 1;
    loop {
        s.ws();
        match b.get(s.i) {
            Some(b'}') => return Ok(None),
            Some(b',') => {
                s.i += 1;
                continue;
            }
            Some(b'"') => {}
            _ => return Err(format!("JSON inesperado na posição {}", s.i)),
        }
        let key = s.string()?;
        s.ws();
        if b.get(s.i) != Some(&b':') {
            return Err("esperava ':'".into());
        }
        s.i += 1;
        s.ws();
        if key != "mcpServers" {
            s.value()?;
            continue;
        }
        if b.get(s.i) != Some(&b'{') {
            return Err("mcpServers não é um objeto".into());
        }
        let open = s.i;
        s.i += 1;
        let mut entries = Vec::new();
        loop {
            s.ws();
            match b.get(s.i) {
                Some(b'}') => return Ok(Some((open, s.i, entries))),
                Some(b',') => {
                    s.i += 1;
                    continue;
                }
                Some(b'"') => {}
                _ => return Err(format!("JSON inesperado na posição {}", s.i)),
            }
            let key_start = s.i;
            let name = s.string()?;
            s.ws();
            s.i += 1; // ':'
            s.ws();
            let value_start = s.i;
            s.value()?;
            entries.push(Entry {
                name,
                key_start,
                value_start,
                value_end: s.i,
            });
        }
    }
}

/// One-line rendering: `{ "a": 1, "b": ["x", "y"] }`.
pub fn compact(v: &Value) -> String {
    match v {
        Value::Object(m) if m.is_empty() => "{}".into(),
        Value::Object(m) => format!(
            "{{ {} }}",
            m.iter()
                .map(|(k, x)| format!("{}: {}", serde_json::to_string(k).unwrap(), compact(x)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Array(a) => format!("[{}]", a.iter().map(compact).collect::<Vec<_>>().join(", ")),
        other => serde_json::to_string(other).unwrap(),
    }
}

fn pretty_indented(v: &Value, indent: &str) -> String {
    let p = serde_json::to_string_pretty(v).unwrap();
    p.lines()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 {
                l.to_string()
            } else {
                format!("{}{}", indent, l)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Hybrid style used in hand-written files: inline when short, otherwise
/// one field per line with inline values.
fn hybrid(v: &Value, indent: &str, unit: &str) -> String {
    let c = compact(v);
    if c.len() + indent.len() <= 100 {
        return c;
    }
    match v {
        Value::Object(m) => {
            let inner = format!("{}{}", indent, unit);
            let lines: Vec<String> = m
                .iter()
                .map(|(k, x)| {
                    format!(
                        "{}{}: {}",
                        inner,
                        serde_json::to_string(k).unwrap(),
                        compact(x)
                    )
                })
                .collect();
            format!("{{\n{}\n{}}}", lines.join(",\n"), indent)
        }
        _ => c,
    }
}

fn line_indent(text: &str, pos: usize) -> String {
    let line_start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    text[line_start..pos]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

/// Detect (entry indent, indent unit, prefer hybrid).
fn style(text: &str, open: usize, entries: &[Entry]) -> (String, String, bool) {
    if let Some(e) = entries.first() {
        let ind = line_indent(text, e.key_start);
        let parent = line_indent(text, open);
        let unit = ind
            .strip_prefix(parent.as_str())
            .unwrap_or("  ")
            .to_string();
        let unit = if unit.is_empty() {
            "  ".to_string()
        } else {
            unit
        };
        let any_inline_object = entries.iter().any(|e| {
            let v = &text[e.value_start..e.value_end];
            v.starts_with('{') && !v.contains('\n') && v.len() > 2
        });
        (ind, unit, any_inline_object)
    } else {
        let parent = line_indent(text, open);
        (format!("{}  ", parent), "  ".into(), false)
    }
}

fn render(v: &Value, indent: &str, unit: &str, hybrid_style: bool) -> String {
    if hybrid_style {
        hybrid(v, indent, unit)
    } else {
        pretty_indented(v, indent)
    }
}

/// Set (`Some`) or remove (`None`) one server, touching only its span.
/// Creates `mcpServers` when missing.
pub fn set_server(text: &str, name: &str, value: Option<&Value>) -> Result<String, String> {
    // validate first: never edit a file we cannot parse
    serde_json::from_str::<Value>(text).map_err(|e| format!("JSON inválido: {}", e))?;
    let Some((open, close, entries)) = locate(text)? else {
        let mut doc: Map<String, Value> = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let Some(v) = value else {
            return Ok(text.to_string());
        };
        let mut servers = Map::new();
        servers.insert(name.to_string(), v.clone());
        doc.insert("mcpServers".into(), Value::Object(servers));
        return Ok(serde_json::to_string_pretty(&Value::Object(doc)).unwrap() + "\n");
    };
    let (indent, unit, hyb) = style(text, open, &entries);
    let key = serde_json::to_string(name).unwrap();
    let existing = entries.iter().position(|e| e.name == name);
    let out = match (existing, value) {
        (Some(i), Some(v)) => {
            let e = &entries[i];
            let old_text = &text[e.value_start..e.value_end];
            if serde_json::from_str::<Value>(old_text).ok().as_ref() == Some(v) {
                return Ok(text.to_string());
            }
            let was_inline = !old_text.contains('\n');
            let rendered = if hyb && was_inline {
                // an entry written on one line stays on one line
                compact(v)
            } else {
                render(v, &indent, &unit, hyb)
            };
            format!(
                "{}{}{}",
                &text[..e.value_start],
                rendered,
                &text[e.value_end..]
            )
        }
        (Some(i), None) => {
            let e = &entries[i];
            if entries.len() == 1 {
                // `{ ... }` becomes `{}`
                format!("{}{}", &text[..open + 1], &text[close..])
            } else if i + 1 < entries.len() {
                // remove from key start up to next key start
                let next = entries[i + 1].key_start;
                let line_start = text[..e.key_start]
                    .rfind('\n')
                    .map(|x| x + 1)
                    .unwrap_or(e.key_start);
                let next_line = text[..next].rfind('\n').map(|x| x + 1).unwrap_or(next);
                format!("{}{}", &text[..line_start], &text[next_line..])
            } else {
                // last entry: drop the preceding comma too
                let prev_end = entries[i - 1].value_end;
                format!("{}{}", &text[..prev_end], &text[e.value_end..])
            }
        }
        (None, Some(v)) => {
            let rendered = render(v, &indent, &unit, hyb);
            match entries.last() {
                Some(last) => format!(
                    "{},\n{}{}: {}{}",
                    &text[..last.value_end],
                    indent,
                    key,
                    rendered,
                    &text[last.value_end..]
                ),
                None => {
                    let close_indent = line_indent(text, close);
                    format!(
                        "{}\n{}{}: {}\n{}{}",
                        &text[..open + 1],
                        indent,
                        key,
                        rendered,
                        close_indent,
                        &text[close..]
                    )
                }
            }
        }
        (None, None) => text.to_string(),
    };
    // the result must still be valid JSON with the expected content
    let check: Value =
        serde_json::from_str(&out).map_err(|e| format!("edição gerou JSON inválido: {}", e))?;
    let got = check.get("mcpServers").and_then(|m| m.get(name));
    if got != value {
        return Err("edição não produziu o valor esperado".into());
    }
    Ok(out)
}

/// Set the `disabledMcpjsonServers` list of `.claude/settings.json`
/// (Claude-managed file: standard pretty JSON, other keys kept in order).
pub fn set_disabled(
    text: Option<&str>,
    name: &str,
    disabled: bool,
) -> Result<Option<String>, String> {
    let mut doc: Map<String, Value> = match text {
        Some(t) if !t.trim().is_empty() => {
            serde_json::from_str(t).map_err(|e| format!("JSON inválido: {}", e))?
        }
        _ => Map::new(),
    };
    let list = doc
        .entry("disabledMcpjsonServers")
        .or_insert_with(|| Value::Array(vec![]));
    let arr = list
        .as_array_mut()
        .ok_or("disabledMcpjsonServers não é uma lista")?;
    let has = arr.iter().any(|v| v.as_str() == Some(name));
    if has == disabled {
        return Ok(None);
    }
    if disabled {
        arr.push(Value::String(name.to_string()));
    } else {
        arr.retain(|v| v.as_str() != Some(name));
    }
    if arr.is_empty() {
        doc.remove("disabledMcpjsonServers");
    }
    Ok(Some(
        serde_json::to_string_pretty(&Value::Object(doc)).unwrap() + "\n",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const HYBRID: &str = r#"{
  "mcpServers": {
    "ai-memory": { "type": "http", "url": "http://127.0.0.1:1/mcp" },
    "comfyui": {
      "type": "stdio",
      "command": "node",
      "args": ["/a/very/long/path/to/some/node_modules/comfyui-mcp/dist/index.js", "--full", "--more-flags"],
      "env": { "A": "1" }
    },
    "Neon": { "type": "http", "url": "https://mcp.neon.tech/mcp" }
  }
}
"#;

    const PRETTY: &str = "{\n  \"mcpServers\": {\n    \"supabase\": {\n      \"type\": \"http\",\n      \"url\": \"https://x\"\n    }\n  }\n}\n";

    #[test]
    fn replace_keeps_every_other_byte() {
        let out = set_server(
            HYBRID,
            "ai-memory",
            Some(&json!({"type": "http", "url": "http://127.0.0.1:2/mcp"})),
        )
        .unwrap();
        assert_eq!(out, HYBRID.replace("127.0.0.1:1", "127.0.0.1:2"));
    }

    #[test]
    fn add_in_hybrid_file_is_inline_when_short() {
        let out = set_server(
            HYBRID,
            "ctx",
            Some(&json!({"type": "stdio", "command": "npx", "args": ["ctx"]})),
        )
        .unwrap();
        assert!(out.contains("    \"Neon\": { \"type\": \"http\", \"url\": \"https://mcp.neon.tech/mcp\" },\n    \"ctx\": { \"type\": \"stdio\", \"command\": \"npx\", \"args\": [\"ctx\"] }\n  }"));
        assert!(out.starts_with(&HYBRID[..HYBRID.find("\"Neon\"").unwrap()]));
    }

    #[test]
    fn add_in_pretty_file_is_pretty() {
        let out = set_server(
            PRETTY,
            "repowise",
            Some(&json!({"command": "repowise", "args": ["mcp"]})),
        )
        .unwrap();
        assert_eq!(
            out,
            "{\n  \"mcpServers\": {\n    \"supabase\": {\n      \"type\": \"http\",\n      \"url\": \"https://x\"\n    },\n    \"repowise\": {\n      \"command\": \"repowise\",\n      \"args\": [\n        \"mcp\"\n      ]\n    }\n  }\n}\n"
        );
    }

    #[test]
    fn remove_middle_first_and_last() {
        let mid = set_server(HYBRID, "comfyui", None).unwrap();
        assert!(!mid.contains("comfyui"));
        assert!(mid.contains("\"ai-memory\": { \"type\": \"http\", \"url\": \"http://127.0.0.1:1/mcp\" },\n    \"Neon\""));
        let last = set_server(HYBRID, "Neon", None).unwrap();
        assert!(last.contains("\"env\": { \"A\": \"1\" }\n    }\n  }\n}"));
        let first = set_server(HYBRID, "ai-memory", None).unwrap();
        assert!(first.starts_with("{\n  \"mcpServers\": {\n    \"comfyui\": {"));
        let only = set_server(PRETTY, "supabase", None).unwrap();
        let v: Value = serde_json::from_str(&only).unwrap();
        assert_eq!(v["mcpServers"], json!({}));
    }

    #[test]
    fn add_to_empty_and_missing_object() {
        let out = set_server(
            "{\n  \"mcpServers\": {}\n}\n",
            "a",
            Some(&json!({"command": "x"})),
        )
        .unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["mcpServers"]["a"]["command"], "x");
        let out = set_server("{\"other\": 1}", "a", Some(&json!({"command": "x"}))).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["other"], 1);
        assert_eq!(v["mcpServers"]["a"]["command"], "x");
    }

    #[test]
    fn refuses_invalid_json_and_handles_escapes() {
        assert!(set_server("{ broken", "a", None).is_err());
        let t = "{\"mcpServers\": {\"we\\\"ird\": {\"command\": \"a}b\"}, \"n\": {\"command\": \"c\"}}}";
        let out = set_server(t, "n", None).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["mcpServers"]["we\"ird"]["command"], "a}b");
        assert!(v["mcpServers"].get("n").is_none());
    }

    #[test]
    fn disabled_list_toggle() {
        let t =
            "{\n  \"permissions\": {\"allow\": []},\n  \"disabledMcpjsonServers\": [\"a\"]\n}\n";
        assert!(set_disabled(Some(t), "a", true).unwrap().is_none());
        let on = set_disabled(Some(t), "a", false).unwrap().unwrap();
        let v: Value = serde_json::from_str(&on).unwrap();
        assert!(v.get("disabledMcpjsonServers").is_none());
        assert!(on.find("permissions").is_some());
        let off = set_disabled(None, "b", true).unwrap().unwrap();
        let v: Value = serde_json::from_str(&off).unwrap();
        assert_eq!(v["disabledMcpjsonServers"], json!(["b"]));
    }

    /// Manual check (read-only): rewriting every server of the real source
    /// files with its own value must not change a single byte.
    /// `cargo test real_sources_roundtrip -- --ignored`
    #[test]
    #[ignore]
    fn real_sources_roundtrip() {
        let home = directories::BaseDirs::new()
            .unwrap()
            .home_dir()
            .to_path_buf();
        let snap = crate::sync::source::SourceSnapshot::load(&home);
        let mut files = vec![snap.agents.global_mcp_file.clone()];
        files.extend(snap.projects.iter().map(|p| p.path.join(".mcp.json")));
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else {
                continue;
            };
            let doc: Value = serde_json::from_str(&text).unwrap();
            for (name, v) in doc["mcpServers"].as_object().unwrap() {
                let out = set_server(&text, name, Some(v)).unwrap();
                assert!(out == text, "{} / {} changed", f.display(), name);
            }
            println!("ok {}", f.file_name().unwrap().to_string_lossy());
        }
    }
}
