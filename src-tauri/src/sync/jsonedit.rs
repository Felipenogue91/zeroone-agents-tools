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

/// Locate the object at `path` (keys from the root): (open, close, entries).
fn locate_at(text: &str, path: &[&str]) -> Result<Option<(usize, usize, Vec<Entry>)>, String> {
    let b = text.as_bytes();
    let mut s = Scan { b, i: 0 };
    s.ws();
    let mut depth = 0usize;
    loop {
        if b.get(s.i) != Some(&b'{') {
            return Err(if depth == 0 {
                "o arquivo não é um objeto JSON".into()
            } else {
                format!("{} não é um objeto", path[depth - 1])
            });
        }
        let open = s.i;
        s.i += 1;
        let mut entries = Vec::new();
        let mut descended = false;
        loop {
            s.ws();
            match b.get(s.i) {
                Some(b'}') => break,
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
            if b.get(s.i) != Some(&b':') {
                return Err("esperava ':'".into());
            }
            s.i += 1;
            s.ws();
            if depth < path.len() && name == path[depth] {
                depth += 1;
                descended = true;
                break;
            }
            let value_start = s.i;
            s.value()?;
            entries.push(Entry {
                name,
                key_start,
                value_start,
                value_end: s.i,
            });
        }
        if descended {
            continue;
        }
        if depth < path.len() {
            return Ok(None); // path missing
        }
        return Ok(Some((open, s.i, entries)));
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

/// Multi-line rendering in the hybrid style: one key/item per line, values
/// inline (used when replacing a value that already spanned lines).
fn expanded(v: &Value, indent: &str, unit: &str) -> String {
    let inner = format!("{}{}", indent, unit);
    match v {
        Value::Object(m) if !m.is_empty() => {
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
        Value::Array(a) if !a.is_empty() => {
            let lines: Vec<String> = a
                .iter()
                .map(|x| format!("{}{}", inner, compact(x)))
                .collect();
            format!("[\n{}\n{}]", lines.join(",\n"), indent)
        }
        other => compact(other),
    }
}

/// File written in the hybrid style (some objects on one line)?
fn file_is_hybrid(text: &str) -> bool {
    text.lines().any(|l| {
        let t = l.trim();
        t.contains(": { \"")
            || t.contains(": {\"")
            || (t.starts_with("\"") && t.contains("\": [\"") && t.ends_with("],"))
    })
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
    set_key(text, &["mcpServers"], name, value)
}

fn value_at<'v>(doc: &'v Value, path: &[&str]) -> Option<&'v Value> {
    path.iter().try_fold(doc, |v, k| v.get(*k))
}

/// Set (`Some`) or remove (`None`) `key` inside the object at `path`,
/// touching only that span and keeping the file's style. Missing parent
/// objects are created.
pub fn set_key(
    text: &str,
    path: &[&str],
    key: &str,
    value: Option<&Value>,
) -> Result<String, String> {
    // validate first: never edit a file we cannot parse
    let doc: Value = serde_json::from_str(text).map_err(|e| format!("JSON inválido: {}", e))?;
    let Some((open, close, entries)) = locate_at(text, path)? else {
        let Some(v) = value else {
            return Ok(text.to_string());
        };
        if path.is_empty() {
            return Err("o arquivo não é um objeto JSON".into());
        }
        // create the missing parent: set parent[last] = { key: value }
        let (parent, last) = path.split_at(path.len() - 1);
        let mut obj = Map::new();
        obj.insert(key.to_string(), v.clone());
        let mut new_parent_val = value_at(&doc, parent).and_then(|p| p.get(last[0])).cloned();
        if let Some(Value::Object(m)) = &mut new_parent_val {
            m.insert(key.to_string(), v.clone());
        } else {
            new_parent_val = Some(Value::Object(obj));
        }
        return set_key(text, parent, last[0], new_parent_val.as_ref());
    };
    let hyb = file_is_hybrid(text) || style(text, open, &entries).2;
    let (indent, unit, _) = style(text, open, &entries);
    let key_json = serde_json::to_string(key).unwrap();
    let existing = entries.iter().position(|e| e.name == key);
    let out = match (existing, value) {
        (Some(i), Some(v)) => {
            let e = &entries[i];
            let old_text = &text[e.value_start..e.value_end];
            if serde_json::from_str::<Value>(old_text).ok().as_ref() == Some(v) {
                return Ok(text.to_string());
            }
            let was_inline = !old_text.contains('\n');
            let rendered = if was_inline && hyb {
                compact(v) // an entry written on one line stays on one line
            } else if hyb {
                expanded(v, &indent, &unit)
            } else {
                pretty_indented(v, &indent)
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
                format!("{}{}", &text[..open + 1], &text[close..])
            } else if i + 1 < entries.len() {
                let next = entries[i + 1].key_start;
                let line_start = text[..e.key_start].rfind('\n').map(|x| x + 1).unwrap_or(0);
                let next_line = text[..next].rfind('\n').map(|x| x + 1).unwrap_or(0);
                let own_line = text[line_start..e.key_start].trim().is_empty();
                let next_own_line =
                    text[next_line..next].trim().is_empty() && next_line > e.key_start;
                if own_line && next_own_line {
                    format!("{}{}", &text[..line_start], &text[next_line..])
                } else {
                    // entries share a line: cut from this key to the next one
                    format!("{}{}", &text[..e.key_start], &text[next..])
                }
            } else {
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
                    key_json,
                    rendered,
                    &text[last.value_end..]
                ),
                None => {
                    let close_indent = line_indent(text, close);
                    format!(
                        "{}\n{}{}: {}\n{}{}",
                        &text[..open + 1],
                        indent,
                        key_json,
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
    let got = value_at(&check, path).and_then(|o| o.get(key));
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

    const SYNC: &str = r#"{
  "_doc": "x",
  "projects": [
    "~/w/a",
    "~/w/b"
  ],
  "pluginProvided": {
    "claude": ["Neon"]
  },
  "codexGlobalKeep": ["node_repl"],
  "overrides": {
    "global": {
      "agy": {
        "ctx": { "args": ["--a"] }
      }
    }
  }
}
"#;

    #[test]
    fn set_key_keeps_multiline_array_style() {
        let out = set_key(
            SYNC,
            &[],
            "projects",
            Some(&json!(["~/w/a", "~/w/b", "~/w/c"])),
        )
        .unwrap();
        assert!(out.contains("  \"projects\": [\n    \"~/w/a\",\n    \"~/w/b\",\n    \"~/w/c\"\n  ],\n  \"pluginProvided\""));
        assert!(out.ends_with(&SYNC[SYNC.find("  \"pluginProvided\"").unwrap()..]));
    }

    #[test]
    fn set_key_creates_nested_parents_inline_in_hybrid_file() {
        let out = set_key(SYNC, &["cliOff", "global"], "codex", Some(&json!(["ctx"]))).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["cliOff"]["global"]["codex"], json!(["ctx"]));
        assert!(out.contains("  \"cliOff\": { \"global\": { \"codex\": [\"ctx\"] } }\n}"));
        // deeper edit inside an existing nested object
        let out2 = set_key(&out, &["cliOff", "global"], "agy", Some(&json!(["x"]))).unwrap();
        let v: Value = serde_json::from_str(&out2).unwrap();
        assert_eq!(v["cliOff"]["global"]["agy"], json!(["x"]));
        assert_eq!(v["cliOff"]["global"]["codex"], json!(["ctx"]));
        // remove
        let out3 = set_key(&out2, &["cliOff", "global"], "codex", None).unwrap();
        let v: Value = serde_json::from_str(&out3).unwrap();
        assert!(v["cliOff"]["global"].get("codex").is_none());
        let untouched = &SYNC[..SYNC.rfind("  }\n}").unwrap() + 3];
        assert!(out3.starts_with(untouched), "{}", out3);
        assert!(
            out3.contains("\"cliOff\": { \"global\": { \"agy\": [\"x\"] } }"),
            "{}",
            out3
        );
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
