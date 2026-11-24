//! "Contexto" tab for Codex and agy: what a session put into the model's
//! context, read from what each CLI records on disk.
//!
//! - **Codex** writes `~/.codex/sessions/YYYY/MM/DD/rollout-*-<id>.jsonl`:
//!   one `response_item` per piece sent to the model (system prompt, tool
//!   definitions, skill list, AGENTS.md, deferred MCP namespaces, hook
//!   output, prompts, tool results) and a `token_usage_record` per call.
//! - **agy** keeps one SQLite file per conversation in
//!   `~/.gemini/antigravity-cli/conversations/<id>.db`. Each model call has a
//!   `gen_metadata` row (protobuf) with its token usage; the newest one also
//!   carries the whole request: the system prompt split in named sections,
//!   the tool definitions and every message with its token count.
//!
//! Both formats are internal and change between versions, so everything is
//! lenient: unknown pieces land in the CLI's fixed group, missing ones just
//! leave a group empty. The output is the same `SessionContext` the Claude
//! Code parser returns, so the tab shows the three the same way.

use super::context_config::body_item;
use super::context_inspector::{
    estimate_tokens, human_kb, ContextFolder, ContextGroup, ContextItem, MeasuredUsage,
    SessionContext, SessionRef, TurnStat, IMAGE_CHARS,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn char_len(s: &str) -> usize {
    s.chars().count()
}

fn rfc3339(t: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()
}

fn tilde(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rel) => format!("~/{}", rel.display()),
        Err(_) => path.display().to_string(),
    }
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// Ids come from the UI: keep them plain file-name material.
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn group_folders(rows: Vec<(String, SystemTime)>, limit: usize) -> Vec<ContextFolder> {
    let mut map: BTreeMap<String, (usize, SystemTime)> = BTreeMap::new();
    for (cwd, t) in rows {
        let e = map.entry(cwd).or_insert((0, t));
        e.0 += 1;
        if t > e.1 {
            e.1 = t;
        }
    }
    let mut out: Vec<(SystemTime, ContextFolder)> = map
        .into_iter()
        .map(|(path, (n, t))| {
            (
                t,
                ContextFolder {
                    exists: Path::new(&path).is_dir(),
                    path,
                    sessions: n,
                    last_used: Some(rfc3339(t)),
                },
            )
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter().map(|(_, f)| f).take(limit).collect()
}

/// Group whose total is the sum of its items (some carry exact counts).
fn group(key: &str, title: &str, hint: &str, items: Vec<ContextItem>) -> Option<ContextGroup> {
    if items.is_empty() {
        return None;
    }
    let mut g = ContextGroup::new(key, title, hint, items);
    g.tokens = g.items.iter().map(|i| i.tokens).sum();
    Some(g)
}

/// "Native tools (N)" with the three biggest named, like the Claude side.
fn native_tools_item(tools: &[(String, usize)], source: &str) -> Option<ContextItem> {
    let total: usize = tools.iter().map(|t| t.1).sum();
    if total == 0 {
        return None;
    }
    let mut biggest: Vec<&(String, usize)> = tools.iter().collect();
    biggest.sort_by(|a, b| b.1.cmp(&a.1));
    let top: Vec<String> = biggest
        .iter()
        .take(3)
        .map(|(n, c)| format!("{} ({})", n, human_kb(*c)))
        .collect();
    Some(
        ContextItem::new(
            format!("Ferramentas nativas ({})", tools.len()),
            Some(source.into()),
            total,
        )
        .note(format!("Maiores: {}", top.join(", "))),
    )
}

/// Item whose token count is known exactly (agy records it per message).
fn exact_item(
    label: impl Into<String>,
    source: Option<String>,
    chars: usize,
    tokens: Option<usize>,
) -> ContextItem {
    let mut i = ContextItem::new(label, source, chars);
    if let Some(t) = tokens.filter(|t| *t > 0) {
        i.tokens = t;
    }
    i
}

/// Repeated pieces (hook output on every message), summed by label.
#[derive(Default)]
struct Repeats(BTreeMap<String, (usize, usize, usize)>);

impl Repeats {
    fn add(&mut self, label: &str, chars: usize, tokens: usize) {
        let e = self.0.entry(label.to_string()).or_default();
        e.0 += 1;
        e.1 += chars;
        e.2 += tokens;
    }
    fn items(self) -> Vec<ContextItem> {
        self.0
            .into_iter()
            .map(|(label, (n, chars, tokens))| {
                let mut i = ContextItem::new(label, None, chars);
                i.tokens = tokens.max(i.tokens);
                i.count = Some(n);
                i
            })
            .collect()
    }
}

/// Skill bodies, read from the paths the skill list gave.
fn skill_bodies(skills: &[ContextItem], home: &Path, cli: &str) -> Vec<ContextGroup> {
    let mut items = Vec::new();
    let mut missing = Vec::new();
    for s in skills {
        let path = s.source.as_deref().map(|p| match p.strip_prefix("~/") {
            Some(rel) => home.join(rel),
            None => PathBuf::from(p),
        });
        match path.and_then(|p| body_item(&s.label, &p, home)) {
            Some(i) => items.push(i),
            None if s.source.is_some() => missing.push(s.label.clone()),
            None => {}
        }
    }
    if !missing.is_empty() {
        missing.sort();
        items.push(
            ContextItem::new(
                format!("{} skill(s) sem arquivo no disco", missing.len()),
                None,
                0,
            )
            .note(format!(
                "Embutidas no {} ou apagadas: {}",
                cli,
                missing.join(", ")
            )),
        );
    }
    group(
        "skillBodies",
        "Corpo das skills",
        "Entra quando a skill é usada (você chama ou o modelo decide). Fica até o fim da sessão.",
        items,
    )
    .into_iter()
    .collect()
}

// ─── Message by message (shared) ────────────────────────────────────────────

#[derive(Default, Clone, Copy)]
struct Size {
    chars: usize,
    tokens: usize,
}

impl Size {
    fn est(chars: usize) -> Self {
        Size {
            chars,
            tokens: estimate_tokens(chars),
        }
    }
    fn exact(chars: usize, tokens: Option<usize>) -> Self {
        Size {
            chars,
            tokens: tokens
                .filter(|t| *t > 0)
                .unwrap_or_else(|| estimate_tokens(chars)),
        }
    }
    fn add(&mut self, o: Size) {
        self.chars += o.chars;
        self.tokens += o.tokens;
    }
}

#[derive(Default)]
struct TurnAcc {
    index: usize,
    at: Option<String>,
    prompt: String,
    images: usize,
    message: Size,
    hooks: Size,
    extras: Size,
    reply: Size,
    tools: BTreeMap<String, (usize, Size)>,
    calls: Vec<(u64, u64)>,
    compacted: bool,
}

/// Builds `TurnStat`s the same way the Claude Code side does: the real
/// context after a message is the input of its last call plus that call's
/// output; growth is against the previous message.
struct Turns {
    cli: &'static str,
    turns: Vec<TurnStat>,
    cur: Option<TurnAcc>,
    last_ctx: Option<u64>,
}

impl Turns {
    fn new(cli: &'static str) -> Self {
        Turns {
            cli,
            turns: Vec::new(),
            cur: None,
            last_ctx: None,
        }
    }
    fn open(&mut self, prompt: &str, at: Option<String>, size: Size, images: usize) {
        self.close();
        let shown: String = if prompt.trim().is_empty() {
            "(só imagem)".into()
        } else {
            prompt.trim().chars().take(160).collect()
        };
        let mut message = size;
        message.add(Size::est(images * IMAGE_CHARS));
        self.cur = Some(TurnAcc {
            index: self.turns.len() + 1,
            at,
            prompt: shown,
            images,
            message,
            ..Default::default()
        });
    }
    fn with(&mut self, f: impl FnOnce(&mut TurnAcc)) {
        if let Some(c) = self.cur.as_mut() {
            f(c);
        }
    }
    fn tool(&mut self, name: &str, size: Size) {
        self.with(|c| {
            let e = c.tools.entry(name.to_string()).or_default();
            e.0 += 1;
            e.1.add(size);
        });
    }
    fn call(&mut self, input: u64, output: u64) {
        self.with(|c| {
            if c.calls.last().is_some_and(|(prev, _)| input + 2000 < *prev) {
                c.compacted = true;
            }
            c.calls.push((input, output));
        });
    }

    fn close(&mut self) {
        let Some(t) = self.cur.take() else { return };
        let context_after = t.calls.last().map(|(i, o)| i + o);
        let before = self.last_ctx.or(t.calls.first().map(|c| c.0));
        let growth = match (context_after, before) {
            (Some(a), Some(b)) => Some(a as i64 - b as i64),
            _ => None,
        };
        if context_after.is_some() {
            self.last_ctx = context_after;
        }
        let compacted = t.compacted || growth.is_some_and(|g| g < -2000);
        let output_tokens: u64 = t.calls.iter().map(|c| c.1).sum();
        let tools_size = t.tools.values().fold(Size::default(), |mut a, (_, s)| {
            a.add(*s);
            a
        });
        let part = |label: &str, s: Size| {
            let mut i = ContextItem::new(label, None, s.chars);
            i.tokens = s.tokens;
            i
        };
        let mut parts = vec![
            part("Sua mensagem", t.message),
            part("Resultados de ferramentas", tools_size),
            part("Hooks", t.hooks),
            part(&format!("Avisos do {}", self.cli), t.extras),
        ];
        if t.images > 0 {
            parts[0].note = Some(format!("{} imagem(ns), ~1,6 mil tokens cada", t.images));
        }
        let mut reply = part(&format!("Respostas do {}", self.cli), t.reply);
        if output_tokens > 0 {
            reply.tokens = output_tokens as usize;
            reply.note = Some("Medido: texto, chamadas de ferramenta e raciocínio".into());
        }
        parts.push(reply);
        parts.retain(|p| p.tokens > 0);
        if let Some(g) = growth.filter(|g| *g > 0 && !compacted) {
            let known: usize = parts.iter().map(|p| p.tokens).sum();
            if g as usize > known {
                let mut r = ContextItem::new(
                    format!(
                        "Resto (embalagem do {} e diferença da estimativa)",
                        self.cli
                    ),
                    None,
                    0,
                );
                r.tokens = g as usize - known;
                parts.push(r);
            }
        }
        parts.sort_by(|a, b| b.tokens.cmp(&a.tokens));
        let mut tools: Vec<ContextItem> = t
            .tools
            .into_iter()
            .map(|(name, (n, s))| {
                let mut i = part(&name, s);
                i.count = Some(n);
                i
            })
            .collect();
        tools.sort_by(|a, b| b.tokens.cmp(&a.tokens));
        self.turns.push(TurnStat {
            index: t.index,
            at: t.at,
            prompt: t.prompt,
            kind: "prompt".into(),
            images: t.images,
            context_after,
            growth,
            api_calls: t.calls.len(),
            output_tokens,
            compacted,
            parts,
            tools,
        });
    }

    fn finish(mut self) -> Vec<TurnStat> {
        self.close();
        self.turns
    }
}

/// Short label for an injected hook text: who wrote it, when it says so.
fn hook_label(text: &str) -> String {
    let t = text.trim();
    if let Some(rest) = t.split("injected by a ").nth(1) {
        if let Some(ev) = rest.split_whitespace().next() {
            let first = t
                .lines()
                .skip_while(|l| !l.contains("injected by a "))
                .nth(1)
                .unwrap_or("");
            return format!("Hook {}: {}", ev, clean_line(first));
        }
    }
    clean_line(t.lines().find(|l| !l.trim().is_empty()).unwrap_or(""))
}

fn clean_line(l: &str) -> String {
    let s: String = l
        .trim()
        .trim_start_matches(['>', '#', ' ', '*', '-'])
        .replace("**", "")
        .chars()
        .take(90)
        .collect();
    if s.is_empty() {
        "(texto sem título)".into()
    } else {
        s
    }
}

// ─── Codex ──────────────────────────────────────────────────────────────────

fn codex_sessions_dir(home: &Path) -> PathBuf {
    home.join(".codex").join("sessions")
}

fn codex_files(home: &Path) -> Vec<(SystemTime, PathBuf)> {
    let mut out: Vec<(SystemTime, PathBuf)> = walkdir::WalkDir::new(codex_sessions_dir(home))
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("jsonl")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("rollout-"))
        })
        .filter_map(|p| Some((mtime(&p)?, p)))
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out
}

/// `session_meta` (first line): id and folder.
fn codex_meta(path: &Path) -> Option<(String, String)> {
    let mut line = String::new();
    BufReader::new(File::open(path).ok()?)
        .read_line(&mut line)
        .ok()?;
    let v: Value = serde_json::from_str(&line).ok()?;
    let p = v.get("payload")?;
    Some((
        p.get("id").and_then(Value::as_str)?.to_string(),
        p.get("cwd").and_then(Value::as_str)?.to_string(),
    ))
}

pub fn codex_folders(home: &Path, limit: usize) -> Vec<ContextFolder> {
    let rows = codex_files(home)
        .into_iter()
        .filter_map(|(t, p)| Some((codex_meta(&p)?.1, t)))
        .collect();
    group_folders(rows, limit)
}

/// Thread names Codex keeps in `session_index.jsonl` (last one wins).
fn codex_titles(home: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(f) = File::open(home.join(".codex/session_index.jsonl")) else {
        return out;
    };
    for line in BufReader::new(f).lines().map_while(Result::ok) {
        if let Ok(v) = serde_json::from_str::<Value>(&line) {
            if let (Some(id), Some(name)) = (
                v.get("id").and_then(Value::as_str),
                v.get("thread_name").and_then(Value::as_str),
            ) {
                out.insert(id.to_string(), name.to_string());
            }
        }
    }
    out
}

/// A typed prompt, not one of the wrapped pieces Codex sends as `user`.
fn codex_is_prompt(text: &str) -> bool {
    let t = text.trim_start();
    !(t.starts_with("# AGENTS.md instructions") || t.starts_with('<'))
}

fn codex_first_prompt(path: &Path) -> Option<String> {
    let f = File::open(path).ok()?;
    for line in BufReader::new(f).lines().take(400).map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let p = &v["payload"];
        if v["type"] == "response_item" && p["type"] == "message" && p["role"] == "user" {
            for c in p["content"].as_array().into_iter().flatten() {
                if let Some(t) = c["text"].as_str().filter(|t| codex_is_prompt(t)) {
                    return Some(t.trim().chars().take(100).collect());
                }
            }
        }
    }
    None
}

pub fn codex_sessions(home: &Path, cwd: &Path, limit: usize) -> Vec<SessionRef> {
    let titles = codex_titles(home);
    let want = cwd.to_string_lossy();
    codex_files(home)
        .into_iter()
        .filter_map(|(t, p)| {
            let (id, c) = codex_meta(&p)?;
            (c == want).then_some((t, p, id))
        })
        .take(limit)
        .map(|(t, p, id)| SessionRef {
            title: titles.get(&id).cloned().or_else(|| codex_first_prompt(&p)),
            size_bytes: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
            modified: Some(rfc3339(t)),
            session_id: id,
        })
        .collect()
}

pub fn codex_session_file(home: &Path, id: &str) -> Option<PathBuf> {
    if !safe_id(id) {
        return None;
    }
    let suffix = format!("-{}.jsonl", id);
    codex_files(home)
        .into_iter()
        .map(|(_, p)| p)
        .find(|p| p.to_string_lossy().ends_with(&suffix))
}

fn text_of_output(v: &Value) -> usize {
    match v {
        Value::String(s) => char_len(s),
        Value::Array(a) => a
            .iter()
            .map(|p| match p.get("text").and_then(Value::as_str) {
                Some(t) => char_len(t),
                None if p.get("type").and_then(Value::as_str) == Some("input_image") => IMAGE_CHARS,
                None => 0,
            })
            .sum(),
        Value::Null => 0,
        other => char_len(&other.to_string()),
    }
}

/// Name of the wrapping tag (`<permissions instructions>` → `permissions instructions`).
fn tag_name(text: &str) -> Option<String> {
    let t = text.trim_start().strip_prefix('<')?;
    let end = t.find('>')?;
    let name = &t[..end];
    (!name.is_empty() && name.len() < 60 && !name.contains('\n')).then(|| name.to_string())
}

fn codex_tag_label(tag: &str) -> String {
    match tag {
        "permissions instructions" => "Permissões e sandbox".into(),
        "collaboration_mode" => "Modo de colaboração".into(),
        "multi_agent_role" => "Papel no time de agentes".into(),
        "multi_agent_mode" => "Regra de subagentes".into(),
        "environment_context" => "Ambiente (pasta, shell, data)".into(),
        "apps_instructions" => "Instruções dos apps".into(),
        "plugins_instructions" => "Instruções dos plugins".into(),
        other => format!("<{}>", other),
    }
}

/// Skill list: one item per skill, plus the explanation/roots header.
fn codex_skills(text: &str, home: &Path) -> Vec<ContextItem> {
    let mut roots: HashMap<String, String> = HashMap::new();
    let mut items = Vec::new();
    let mut header = 0;
    for line in text.lines() {
        let len = char_len(line) + 1;
        let l = line.trim_start();
        // - `r0` = `/home/.../skills`
        if let Some(rest) = l.strip_prefix("- `r") {
            if let Some((k, v)) = rest.split_once("` = `") {
                roots.insert(format!("r{}", k), v.trim_end_matches('`').to_string());
                header += len;
                continue;
            }
        }
        if let Some(rest) = l.strip_prefix("- ") {
            if let Some((name, _)) = rest.split_once(": ") {
                if !name.contains(' ') && !name.is_empty() {
                    let source = rest
                        .rsplit_once("(file: ")
                        .map(|(_, f)| f.trim_end_matches(')'))
                        .map(|f| match f.split_once('/') {
                            Some((r, tail)) if roots.contains_key(r) => {
                                tilde(&Path::new(&roots[r]).join(tail), home)
                            }
                            _ => f.to_string(),
                        });
                    items.push(ContextItem::new(name, source, len));
                    continue;
                }
            }
        }
        header += len;
    }
    if header > 0 {
        items.push(ContextItem::new(
            "Explicação da lista e pastas das skills",
            None,
            header,
        ));
    }
    items
}

/// `<tools>` block: deferred MCP namespaces, only name + description.
fn codex_deferred_mcps(text: &str) -> Vec<ContextItem> {
    let mut items = Vec::new();
    let mut header = 0;
    for line in text.lines() {
        let len = char_len(line) + 1;
        match line.trim_start().strip_prefix("- mcp__") {
            Some(rest) => {
                let name = rest.split(':').next().unwrap_or(rest).trim();
                items.push(
                    ContextItem::new(name, Some("config.toml".into()), len)
                        .note("Só o nome e a descrição; as ferramentas entram quando o Codex procura por elas"),
                );
            }
            None => header += len,
        }
    }
    if header > 0 && !items.is_empty() {
        items.push(ContextItem::new("Cabeçalho da lista de MCPs", None, header));
    }
    items
}

fn codex_agents_sources(home: &Path, cwd: Option<&str>) -> Option<String> {
    let mut files = vec![home.join(".codex/AGENTS.md")];
    if let Some(c) = cwd.map(PathBuf::from).filter(|c| c != home) {
        files.push(c.join("AGENTS.md"));
    }
    let found: Vec<String> = files
        .iter()
        .filter(|p| p.is_file())
        .map(|p| tilde(p, home))
        .collect();
    (!found.is_empty()).then(|| found.join(" + "))
}

pub fn parse_codex(path: &Path, home: &Path) -> std::io::Result<SessionContext> {
    let reader = BufReader::new(File::open(path)?);
    Ok(parse_codex_lines(
        path,
        home,
        reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|l| serde_json::from_str::<Value>(&l).ok()),
    ))
}

pub fn parse_codex_lines(
    path: &Path,
    home: &Path,
    lines: impl Iterator<Item = Value>,
) -> SessionContext {
    let mut ctx = SessionContext {
        cli: "codex".into(),
        session_id: String::new(),
        file: path.to_string_lossy().to_string(),
        cwd: None,
        started_at: None,
        cli_version: None,
        model: None,
        title: None,
        measured: None,
        startup: Vec::new(),
        startup_tokens: 0,
        per_turn: Vec::new(),
        prompts: 0,
        on_demand: Vec::new(),
        turns: Vec::new(),
        warnings: Vec::new(),
    };
    let mut started = false; // after the first model call
    let mut base: Vec<ContextItem> = Vec::new();
    let mut native: Vec<(String, usize)> = Vec::new();
    let mut instructions = Vec::new();
    let mut skills = Vec::new();
    let mut mcps = Vec::new();
    let mut hooks = Vec::new();
    let mut first_prompt = Vec::new();
    let mut repeats = Repeats::default();
    let mut turns = Turns::new("Codex");
    let mut tool_names: HashMap<String, String> = HashMap::new();

    for v in lines {
        let p = &v["payload"];
        let at = v.get("timestamp").and_then(Value::as_str).map(String::from);
        match v["type"].as_str().unwrap_or("") {
            "session_meta" => {
                ctx.session_id = p["id"].as_str().unwrap_or("").to_string();
                ctx.cwd = p["cwd"].as_str().map(String::from);
                ctx.started_at = p["timestamp"].as_str().map(String::from).or(at);
                ctx.cli_version = p["cli_version"].as_str().map(String::from);
            }
            "turn_context" => {
                if ctx.model.is_none() {
                    ctx.model = p["model"].as_str().map(String::from);
                }
            }
            "token_usage_record" => {
                let u = &p["usage"];
                let g = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
                let (input, cached, output) = (
                    g("input_tokens"),
                    g("cached_input_tokens"),
                    g("output_tokens"),
                );
                if input == 0 {
                    continue;
                }
                if ctx.measured.is_none() {
                    ctx.measured = Some(MeasuredUsage {
                        input_tokens: input.saturating_sub(cached),
                        cache_creation_tokens: g("cache_write_input_tokens"),
                        cache_read_tokens: cached,
                        total: input,
                    });
                }
                started = true;
                turns.call(input, output);
            }
            "compacted" => turns.with(|c| c.compacted = true),
            "response_item" => match p["type"].as_str().unwrap_or("") {
                "additional_tools" => {
                    for ns in p["tools"].as_array().into_iter().flatten() {
                        let name = ns["name"].as_str().unwrap_or("?");
                        let size = char_len(&ns.to_string());
                        if started {
                            turns.with(|c| c.extras.add(Size::est(size)));
                            continue;
                        }
                        let n = ns["tools"].as_array().map(|a| a.len()).unwrap_or(1);
                        match name.strip_prefix("mcp__") {
                            Some(server) => mcps.push(
                                ContextItem::new(server, Some("config.toml".into()), size)
                                    .note(format!("{} ferramenta(s) com definição completa", n)),
                            ),
                            None => {
                                for t in ns["tools"].as_array().into_iter().flatten() {
                                    let tn = t["name"].as_str().unwrap_or("?");
                                    let shown = if name == "functions" {
                                        tn.to_string()
                                    } else {
                                        format!("{}.{}", name, tn)
                                    };
                                    native.push((shown, char_len(&t.to_string())));
                                }
                            }
                        }
                    }
                }
                "message" => {
                    let role = p["role"].as_str().unwrap_or("");
                    for c in p["content"].as_array().into_iter().flatten() {
                        let images = usize::from(c["type"] == "input_image");
                        let text = c["text"].as_str().unwrap_or("");
                        let len = char_len(text);
                        if len == 0 && images == 0 {
                            continue;
                        }
                        match role {
                            "developer" => {
                                let item = if text.starts_with("<skills_instructions>") {
                                    if !started {
                                        skills.extend(codex_skills(text, home));
                                        continue;
                                    }
                                    "Lista de skills".to_string()
                                } else if text.starts_with("<tools>") {
                                    if !started {
                                        mcps.extend(codex_deferred_mcps(text));
                                        continue;
                                    }
                                    "Lista de MCPs".to_string()
                                } else if text.starts_with("You are Codex") {
                                    "Prompt do sistema do Codex".to_string()
                                } else if let Some(tag) = tag_name(text) {
                                    codex_tag_label(&tag)
                                } else {
                                    // Anything else from the developer role is
                                    // hook output (SessionStart / UserPromptSubmit).
                                    let label = hook_label(text);
                                    if started || turns.cur.is_some() {
                                        repeats.add(&label, len, estimate_tokens(len));
                                        turns.with(|c| c.hooks.add(Size::est(len)));
                                    } else {
                                        hooks.push(ContextItem::new(label, None, len));
                                    }
                                    continue;
                                };
                                if started {
                                    repeats.add(&item, len, estimate_tokens(len));
                                    turns.with(|c| c.extras.add(Size::est(len)));
                                } else {
                                    base.push(ContextItem::new(item, Some("Codex".into()), len));
                                }
                            }
                            "user" if text.starts_with("# AGENTS.md instructions") => {
                                if started {
                                    turns.with(|c| c.extras.add(Size::est(len)));
                                } else {
                                    instructions.push(ContextItem::new(
                                        "AGENTS.md",
                                        codex_agents_sources(home, ctx.cwd.as_deref()),
                                        len,
                                    ));
                                }
                            }
                            "user" if !text.is_empty() && !codex_is_prompt(text) => {
                                let label = tag_name(text)
                                    .map(|t| codex_tag_label(&t))
                                    .unwrap_or_else(|| "Texto do Codex".into());
                                if started {
                                    turns.with(|c| c.extras.add(Size::est(len)));
                                } else {
                                    base.push(ContextItem::new(label, Some("Codex".into()), len));
                                }
                            }
                            "user" => {
                                // A message may carry text and images in
                                // separate parts: the first part opens it.
                                let joins = turns
                                    .cur
                                    .as_ref()
                                    .is_some_and(|t| t.calls.is_empty() && t.at == at);
                                if joins {
                                    turns.with(|t| {
                                        t.images += images;
                                        t.message.add(Size::est(len + images * IMAGE_CHARS));
                                        if t.prompt == "(só imagem)" && !text.trim().is_empty() {
                                            t.prompt = text.trim().chars().take(160).collect();
                                        }
                                    });
                                } else {
                                    turns.open(text, at.clone(), Size::est(len), images);
                                }
                                if !started {
                                    first_prompt.push(ContextItem::new(
                                        "Sua primeira mensagem",
                                        None,
                                        len + images * IMAGE_CHARS,
                                    ));
                                }
                            }
                            "assistant" => turns.with(|t| t.reply.add(Size::est(len))),
                            _ => {}
                        }
                    }
                }
                "function_call" | "custom_tool_call" => {
                    let name = match (p["namespace"].as_str(), p["name"].as_str()) {
                        (Some(ns), Some(n)) if ns != "functions" => format!("{}.{}", ns, n),
                        (_, Some(n)) => n.to_string(),
                        _ => "?".into(),
                    };
                    if let Some(id) = p["call_id"].as_str() {
                        tool_names.insert(id.to_string(), name);
                    }
                    let args = p
                        .get("arguments")
                        .or_else(|| p.get("input"))
                        .map(text_of_output)
                        .unwrap_or(0);
                    turns.with(|t| t.reply.add(Size::est(args)));
                }
                "function_call_output" | "custom_tool_call_output" => {
                    let name = p["call_id"]
                        .as_str()
                        .and_then(|id| tool_names.get(id))
                        .cloned()
                        .unwrap_or_else(|| "?".into());
                    turns.tool(&name, Size::est(text_of_output(&p["output"])));
                }
                _ => {}
            },
            _ => {}
        }
    }

    let mut first_text = first_prompt;
    first_text.truncate(1);
    ctx.turns = turns.finish();
    ctx.prompts = ctx.turns.len();
    ctx.title = ctx
        .turns
        .first()
        .map(|t| t.prompt.chars().take(100).collect());
    if let Some(i) = native_tools_item(&native, "Codex") {
        base.push(i);
    }
    ctx.startup = [
        group("base", "Codex (fixo)", "Prompt do sistema, permissões, ambiente e ferramentas nativas que vêm com o Codex. Não dá pra mexer por aqui.", base),
        group("instructions", "Instruções (AGENTS.md)", "O AGENTS.md global (~/.codex) e o do projeto, juntos num texto só.", instructions),
        group("skills", "Skills (lista)", "Só o nome e a descrição de cada skill. O corpo entra quando usa.", skills),
        group("mcpTools", "MCPs", "Os MCPs entram adiados: só o nome e a descrição do servidor. As definições das ferramentas entram quando o Codex procura por elas.", mcps),
        group("hooks", "Hooks de início", "Texto que os hooks (SessionStart / UserPromptSubmit) imprimiram antes da 1ª resposta.", hooks),
        group("session", "Sua primeira mensagem", "Conta na 1ª resposta medida.", first_text),
    ]
    .into_iter()
    .flatten()
    .collect();
    ctx.startup_tokens = ctx.startup.iter().map(|g| g.tokens).sum();
    ctx.per_turn = group(
        "hooks",
        "Hooks e avisos repetidos",
        "Texto que o Codex reenviou entre as mensagens (hooks e avisos), somado na sessão.",
        repeats.items(),
    )
    .into_iter()
    .collect();
    ctx.on_demand = skill_bodies(&ctx.startup_items("skills"), home, "Codex");
    if ctx.measured.is_none() {
        ctx.warnings
            .push("Esta sessão não tem resposta registrada; sem o número medido.".into());
    }
    ctx
}

impl SessionContext {
    fn startup_items(&self, key: &str) -> Vec<ContextItem> {
        self.startup
            .iter()
            .filter(|g| g.key == key)
            .flat_map(|g| g.items.clone())
            .collect()
    }
}

// ─── agy ────────────────────────────────────────────────────────────────────

/// Schema-less protobuf reader: enough to walk agy's records by field number.
mod pb {
    pub enum Val<'a> {
        Int(u64),
        Bytes(&'a [u8]),
        Fixed,
    }

    fn varint(b: &[u8], i: &mut usize) -> Option<u64> {
        let mut r = 0u64;
        for shift in (0..64).step_by(7) {
            let c = *b.get(*i)?;
            *i += 1;
            r |= u64::from(c & 0x7f) << shift;
            if c < 0x80 {
                return Some(r);
            }
        }
        None
    }

    /// Fields of one message; `None` when the bytes are not a message.
    pub fn fields(b: &[u8]) -> Option<Vec<(u32, Val<'_>)>> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            let key = varint(b, &mut i)?;
            let (field, wire) = ((key >> 3) as u32, key & 7);
            if field == 0 {
                return None;
            }
            let v = match wire {
                0 => Val::Int(varint(b, &mut i)?),
                1 => {
                    i = i.checked_add(8).filter(|e| *e <= b.len())?;
                    Val::Fixed
                }
                5 => {
                    i = i.checked_add(4).filter(|e| *e <= b.len())?;
                    Val::Fixed
                }
                2 => {
                    let n = varint(b, &mut i)? as usize;
                    let end = i.checked_add(n).filter(|e| *e <= b.len())?;
                    let s = &b[i..end];
                    i = end;
                    Val::Bytes(s)
                }
                _ => return None,
            };
            out.push((field, v));
        }
        Some(out)
    }

    pub struct Msg<'a>(pub Vec<(u32, Val<'a>)>);

    impl<'a> Msg<'a> {
        pub fn parse(b: &'a [u8]) -> Msg<'a> {
            Msg(fields(b).unwrap_or_default())
        }
        pub fn bytes(&self, f: u32) -> Option<&'a [u8]> {
            self.0.iter().find_map(|(k, v)| match v {
                Val::Bytes(b) if *k == f => Some(*b),
                _ => None,
            })
        }
        pub fn all(&self, f: u32) -> impl Iterator<Item = &'a [u8]> + '_ {
            self.0.iter().filter_map(move |(k, v)| match v {
                Val::Bytes(b) if *k == f => Some(*b),
                _ => None,
            })
        }
        pub fn int(&self, f: u32) -> Option<u64> {
            self.0.iter().find_map(|(k, v)| match v {
                Val::Int(n) if *k == f => Some(*n),
                _ => None,
            })
        }
        pub fn str(&self, f: u32) -> Option<&'a str> {
            std::str::from_utf8(self.bytes(f)?).ok()
        }
        pub fn msg(&self, f: u32) -> Option<Msg<'a>> {
            self.bytes(f).map(Msg::parse)
        }
    }
}

use pb::Msg;

fn agy_dir(home: &Path) -> PathBuf {
    home.join(".gemini/antigravity-cli/conversations")
}

fn open_ro(path: &Path) -> Option<rusqlite::Connection> {
    // `immutable=1`: never touch the CLI's WAL/SHM files.
    let mut uri = String::from("file:");
    for c in path.to_string_lossy().chars() {
        match c {
            '%' => uri.push_str("%25"),
            '?' => uri.push_str("%3f"),
            '#' => uri.push_str("%23"),
            c => uri.push(c),
        }
    }
    uri.push_str("?immutable=1");
    rusqlite::Connection::open_with_flags(
        uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .ok()
}

fn file_uri_path(uri: &str) -> String {
    let raw = uri.strip_prefix("file://").unwrap_or(uri);
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(n) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(n);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    let s = String::from_utf8_lossy(&out).to_string();
    if s.len() > 1 {
        s.trim_end_matches('/').to_string()
    } else {
        s
    }
}

struct AgyMeta {
    cwd: Option<String>,
    created: Option<i64>,
    /// Set on subagent conversations.
    subagent: Option<String>,
}

fn agy_meta(conn: &rusqlite::Connection) -> Option<AgyMeta> {
    let blob: Vec<u8> = conn
        .query_row(
            "select data from trajectory_metadata_blob limit 1",
            [],
            |r| r.get(0),
        )
        .ok()?;
    let m = Msg::parse(&blob);
    let cwd = m
        .msg(1)
        .and_then(|w| w.str(1).map(file_uri_path))
        .or_else(|| m.str(7).map(file_uri_path));
    let created = m.msg(2).and_then(|t| t.int(1)).map(|n| n as i64);
    let subagent = m.str(5).map(|_| {
        m.msg(8)
            .and_then(|s| s.str(2).or(s.str(1)).map(String::from))
            .unwrap_or_else(|| "subagente".into())
    });
    Some(AgyMeta {
        cwd,
        created,
        subagent,
    })
}

fn agy_files(home: &Path) -> Vec<(SystemTime, PathBuf)> {
    let Ok(rd) = std::fs::read_dir(agy_dir(home)) else {
        return Vec::new();
    };
    let mut out: Vec<(SystemTime, PathBuf)> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("db"))
        .filter_map(|p| Some((agy_last_used(&p)?, p)))
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out
}

/// The `.db` mtime lags (WAL); the newest of db/wal is the last activity.
fn agy_last_used(db: &Path) -> Option<SystemTime> {
    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    let wal = PathBuf::from(wal);
    let a = mtime(db)?;
    Some(match std::fs::metadata(&wal) {
        Ok(m) if m.len() > 0 => m.modified().map(|b| a.max(b)).unwrap_or(a),
        _ => a,
    })
}

pub fn agy_folders(home: &Path, limit: usize) -> Vec<ContextFolder> {
    let rows = agy_files(home)
        .into_iter()
        .filter_map(|(t, p)| {
            let m = agy_meta(&open_ro(&p)?)?;
            m.subagent.is_none().then_some(())?;
            Some((m.cwd?, t))
        })
        .collect();
    group_folders(rows, limit)
}

/// First thing typed, from `history.jsonl` (interactive sessions).
fn agy_titles(home: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(f) = File::open(home.join(".gemini/antigravity-cli/history.jsonl")) else {
        return out;
    };
    for line in BufReader::new(f).lines().map_while(Result::ok) {
        if let Ok(v) = serde_json::from_str::<Value>(&line) {
            if let (Some(id), Some(d)) = (v["conversationId"].as_str(), v["display"].as_str()) {
                out.entry(id.to_string())
                    .or_insert_with(|| d.chars().take(100).collect());
            }
        }
    }
    out
}

/// First user input step (type 14 → field 19.2), for non-interactive runs.
fn agy_first_input(conn: &rusqlite::Connection) -> Option<String> {
    let blob: Vec<u8> = conn
        .query_row(
            "select step_payload from steps where step_type = 14 order by idx limit 1",
            [],
            |r| r.get(0),
        )
        .ok()?;
    let t = Msg::parse(&blob)
        .msg(19)?
        .str(2)?
        .trim()
        .chars()
        .take(100)
        .collect::<String>();
    (!t.is_empty()).then_some(t)
}

pub fn agy_sessions(home: &Path, cwd: &Path, limit: usize) -> Vec<SessionRef> {
    let titles = agy_titles(home);
    let want = cwd.to_string_lossy();
    let mut out = Vec::new();
    for (t, p) in agy_files(home) {
        if out.len() >= limit {
            break;
        }
        let Some(conn) = open_ro(&p) else { continue };
        let Some(m) = agy_meta(&conn) else { continue };
        if m.cwd.as_deref() != Some(&*want) {
            continue;
        }
        let id = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let title = match &m.subagent {
            Some(name) => Some(format!("Subagente: {}", name)),
            None => titles.get(&id).cloned().or_else(|| agy_first_input(&conn)),
        };
        out.push(SessionRef {
            size_bytes: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
            modified: Some(rfc3339(t)),
            session_id: id,
            title,
        });
    }
    out
}

pub fn agy_session_file(home: &Path, id: &str) -> Option<PathBuf> {
    if !safe_id(id) {
        return None;
    }
    let p = agy_dir(home).join(format!("{}.db", id));
    p.is_file().then_some(p)
}

struct AgyGen {
    input: u64,
    cached: u64,
    output: u64,
}

/// `<RULE[source]> ... </RULE[source]>` blocks: one item per rules file.
fn agy_rules(text: &str, home: &Path) -> (Vec<ContextItem>, usize) {
    let mut items = Vec::new();
    let mut rest = text;
    let mut used = 0;
    while let Some(start) = rest.find("<RULE[") {
        let after = &rest[start + 6..];
        let Some(close) = after.find("]>") else { break };
        let source = &after[..close];
        let body_start = start + 6 + close + 2;
        let end_tag = format!("</RULE[{}]>", source);
        let Some(end) = rest[body_start..].find(&end_tag) else {
            break;
        };
        let body = &rest[body_start..body_start + end];
        let len = char_len(body) + char_len(source) * 2 + 17;
        used += len;
        let (label, src) = if source == "user_global" {
            (
                "Regras globais".to_string(),
                Some("~/.gemini/GEMINI.md".to_string()),
            )
        } else {
            let p = Path::new(source);
            (
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| source.into()),
                Some(tilde(p, home)),
            )
        };
        items.push(ContextItem::new(label, src, len));
        rest = &rest[body_start + end + end_tag.len()..];
    }
    (items, char_len(text).saturating_sub(used))
}

/// `- name (path): description` lines of the `<skills>` section.
fn agy_skills(text: &str, home: &Path) -> (Vec<ContextItem>, usize) {
    let mut items = Vec::new();
    let mut header = 0;
    for line in text.lines() {
        let len = char_len(line) + 1;
        let parsed = line.strip_prefix("- ").and_then(|rest| {
            let (name, tail) = rest.split_once(" (")?;
            let (path, _) = tail.split_once("): ")?;
            (!name.contains(' ')).then(|| (name.to_string(), path.to_string()))
        });
        match parsed {
            Some((name, path)) => items.push(ContextItem::new(
                name,
                Some(tilde(Path::new(&path), home)),
                len,
            )),
            None => header += len,
        }
    }
    (items, header)
}

/// `# server` blocks of the `<mcp_servers>` section (tool names only).
fn agy_mcps(text: &str) -> (Vec<ContextItem>, usize) {
    let mut items: Vec<(String, usize, usize)> = Vec::new();
    let mut header = 0;
    let mut in_fence = false;
    for line in text.lines() {
        let len = char_len(line) + 1;
        if line.starts_with("```") {
            in_fence = !in_fence;
            header += len;
            continue;
        }
        match line.strip_prefix("# ") {
            Some(name) if !in_fence && !name.contains('<') => {
                items.push((name.trim().to_string(), len, 0))
            }
            _ => match items.last_mut() {
                Some(cur) if !in_fence && !line.starts_with("</") => {
                    cur.1 += len;
                    if !matches!(line.trim(), "Eager:" | "Lazy:" | "") {
                        cur.2 += 1;
                    }
                }
                _ => header += len,
            },
        }
    }
    let items = items
        .into_iter()
        .map(|(name, chars, tools)| {
            ContextItem::new(name, Some("mcp_config.json".into()), chars)
                .note(format!("{} ferramenta(s) listadas só pelo nome", tools))
        })
        .collect();
    (items, header)
}

pub fn parse_agy(path: &Path, home: &Path) -> Result<SessionContext, String> {
    let conn = open_ro(path).ok_or_else(|| "não deu para abrir a conversa do agy".to_string())?;
    let meta = agy_meta(&conn);
    let mut rows: Vec<(i64, Vec<u8>)> = Vec::new();
    {
        let mut st = conn
            .prepare("select idx, data from gen_metadata order by idx")
            .map_err(|e| e.to_string())?;
        let it = st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?)))
            .map_err(|e| e.to_string())?;
        rows.extend(it.filter_map(Result::ok));
    }
    let id = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut ctx = SessionContext {
        cli: "agy".into(),
        session_id: id,
        file: path.to_string_lossy().to_string(),
        cwd: meta.as_ref().and_then(|m| m.cwd.clone()),
        started_at: meta
            .as_ref()
            .and_then(|m| m.created)
            .and_then(|s| chrono::DateTime::<chrono::Utc>::from_timestamp(s, 0))
            .map(|d| d.to_rfc3339()),
        cli_version: None,
        model: None,
        title: meta
            .as_ref()
            .and_then(|m| m.subagent.clone())
            .map(|n| format!("Subagente: {}", n)),
        measured: None,
        startup: Vec::new(),
        startup_tokens: 0,
        per_turn: Vec::new(),
        prompts: 0,
        on_demand: Vec::new(),
        turns: Vec::new(),
        warnings: Vec::new(),
    };

    let mut gens: Vec<AgyGen> = Vec::new();
    let mut full: Option<&[u8]> = None;
    for (_, data) in &rows {
        let top = Msg::parse(data);
        let Some(req) = top.bytes(1) else { continue };
        let r = Msg::parse(req);
        if let Some(u) = r.msg(4) {
            gens.push(AgyGen {
                input: u.int(2).unwrap_or(0),
                cached: u.int(5).unwrap_or(0),
                output: u.int(3).unwrap_or(0),
            });
        }
        if ctx.model.is_none() {
            ctx.model = r.str(21).or(r.str(19)).map(String::from);
        }
        if r.bytes(1).is_some() {
            full = Some(req); // the newest one with the whole request wins
        }
    }
    if let Some(g) = gens.first() {
        ctx.measured = Some(MeasuredUsage {
            input_tokens: g.input,
            cache_creation_tokens: 0,
            cache_read_tokens: g.cached,
            total: g.input + g.cached,
        });
    }
    let Some(req) = full else {
        ctx.warnings.push(
            "Esta conversa não guardou o pedido completo (conversa vazia ou formato novo do agy); só o número medido aparece.".into(),
        );
        return Ok(ctx);
    };
    let r = Msg::parse(req);

    // ── System prompt, by section ──
    let mut base = Vec::new();
    let mut instructions = Vec::new();
    let mut skills = Vec::new();
    let mut mcps = Vec::new();
    let mut agents = Vec::new();
    let mut seen_sections = 0usize;
    for sec in r.all(16) {
        let s = Msg::parse(sec);
        let (Some(name), Some(text)) = (s.str(1), s.str(2)) else {
            continue;
        };
        seen_sections += char_len(text);
        match name {
            "user_rules" => {
                let (items, header) = agy_rules(text, home);
                instructions.extend(items);
                if header > 0 {
                    instructions.push(ContextItem::new("Cabeçalho das regras", None, header));
                }
            }
            "skills" => {
                let (items, header) = agy_skills(text, home);
                skills.extend(items);
                if header > 0 {
                    skills.push(ContextItem::new(
                        "Explicação da lista de skills",
                        None,
                        header,
                    ));
                }
            }
            "mcp_servers" => {
                let (items, header) = agy_mcps(text);
                mcps.extend(items);
                if header > 0 {
                    mcps.push(ContextItem::new("Explicação dos MCPs", None, header));
                }
            }
            "subagents" => agents.push(ContextItem::new(
                "Instruções de subagentes",
                Some("agy".into()),
                char_len(text),
            )),
            other => base.push(ContextItem::new(
                format!("<{}>", other),
                Some("agy".into()),
                char_len(text),
            )),
        }
    }
    let system_len = r.str(1).map(char_len).unwrap_or(0);
    if seen_sections == 0 && system_len > 0 {
        base.push(ContextItem::new(
            "Prompt do sistema",
            Some("agy".into()),
            system_len,
        ));
    } else if system_len > seen_sections + 200 {
        base.push(ContextItem::new(
            "Resto do prompt do sistema",
            Some("agy".into()),
            system_len - seen_sections,
        ));
    }
    let native: Vec<(String, usize)> = r
        .all(8)
        .map(|t| {
            let m = Msg::parse(t);
            let size = [1, 2, 3]
                .iter()
                .map(|f| m.bytes(*f).map(|b| b.len()).unwrap_or(0))
                .sum();
            (m.str(1).unwrap_or("?").to_string(), size)
        })
        .collect();
    if let Some(i) = native_tools_item(&native, "agy") {
        base.push(i);
    }

    // ── Messages ──
    let mut hooks = Vec::new();
    let mut first = Vec::new();
    let mut repeats = Repeats::default();
    let mut turns = Turns::new("agy");
    let mut tool_names: HashMap<String, String> = HashMap::new();
    let mut replies = 0usize;
    let assistant_total = r.all(2).filter(|m| Msg::parse(m).int(2) == Some(2)).count();
    // One gen per model call, in order. If the conversation was trimmed the
    // request holds only the newest calls: line them up from the end.
    let skip = gens.len().saturating_sub(assistant_total);
    if gens.len().abs_diff(assistant_total) > 2 {
        ctx.warnings.push(format!(
            "O agy registrou {} chamadas e a conversa guardada tem {} respostas; o crescimento por mensagem pode estar desalinhado.",
            gens.len(),
            assistant_total
        ));
    }
    for raw in r.all(2) {
        let m = Msg::parse(raw);
        let text = m.str(3).unwrap_or("");
        let len = char_len(text);
        let tokens = m.int(4).map(|n| n as usize);
        match m.int(2) {
            Some(1) if text.contains("<USER_REQUEST>") => {
                let inner = text
                    .split("<USER_REQUEST>")
                    .nth(1)
                    .and_then(|s| s.split("</USER_REQUEST>").next())
                    .unwrap_or(text)
                    .trim();
                if replies == 0 && turns.cur.is_none() {
                    first.push(exact_item("Sua primeira mensagem", None, len, tokens));
                }
                turns.open(inner, None, Size::exact(len, tokens), 0);
            }
            Some(1) if text.contains("<SYSTEM_MESSAGE>") => {
                // The preamble itself mentions "<SYSTEM_MESSAGE>": the body
                // is after the last opening tag.
                let body = text
                    .rsplit("<SYSTEM_MESSAGE>")
                    .next()
                    .and_then(|s| s.split("</SYSTEM_MESSAGE>").next())
                    .unwrap_or(text);
                let size = Size::exact(len, tokens);
                let b = body.trim_start();
                if b.contains("injected by a ") {
                    let label = hook_label(body);
                    if replies == 0 {
                        hooks.push(exact_item(label, None, len, tokens));
                    } else {
                        repeats.add(&label, size.chars, size.tokens);
                    }
                    turns.with(|t| t.hooks.add(size));
                } else {
                    // Messages from subagents/background tasks and reminders.
                    let label = if b.starts_with("[Message]") {
                        if b.lines().next().unwrap_or("").contains("sender=system") {
                            "Mensagem do sistema".to_string()
                        } else {
                            "Mensagem de subagente ou tarefa".to_string()
                        }
                    } else {
                        clean_line(b.lines().next().unwrap_or(""))
                    };
                    if replies == 0 {
                        base.push(exact_item(label, Some("agy".into()), len, tokens));
                    } else {
                        repeats.add(&label, size.chars, size.tokens);
                    }
                    turns.with(|t| t.extras.add(size));
                }
            }
            Some(1) => {
                let size = Size::exact(len, tokens);
                if replies == 0 {
                    base.push(exact_item("Aviso do agy", Some("agy".into()), len, tokens));
                } else {
                    repeats.add("Aviso do agy", size.chars, size.tokens);
                }
                turns.with(|t| t.extras.add(size));
            }
            Some(2) => {
                if let Some(call) = m.msg(6) {
                    if let (Some(cid), Some(name)) = (call.str(1), call.str(2)) {
                        tool_names.insert(cid.to_string(), name.to_string());
                    }
                }
                let thinking = m.str(11).map(char_len).unwrap_or(0);
                let args = m.msg(6).and_then(|c| c.str(3)).map(char_len).unwrap_or(0);
                turns.with(|t| t.reply.add(Size::est(len + thinking + args)));
                if let Some(g) = replies.checked_add(skip).and_then(|i| gens.get(i)) {
                    turns.call(g.input + g.cached, g.output);
                }
                replies += 1;
            }
            Some(4) => {
                let name = m
                    .str(7)
                    .and_then(|id| tool_names.get(id))
                    .cloned()
                    .unwrap_or_else(|| "?".into());
                turns.tool(&name, Size::exact(len, tokens));
            }
            _ => {}
        }
    }

    ctx.turns = turns.finish();
    ctx.prompts = ctx.turns.len();
    if ctx.title.is_none() {
        ctx.title = ctx
            .turns
            .first()
            .map(|t| t.prompt.chars().take(100).collect());
    }
    first.truncate(1);
    ctx.startup = [
        group("base", "agy (fixo)", "Seções do prompt do sistema e ferramentas nativas que vêm com o agy. Não dá pra mexer por aqui.", base),
        group("instructions", "Regras (GEMINI.md / AGENTS.md)", "Arquivos de regras carregados inteiros: o global e os do projeto.", instructions),
        group("skills", "Skills (lista)", "Só o nome, a pasta e a descrição de cada skill. O corpo entra quando usa.", skills),
        group("mcpTools", "MCPs", "O agy lista só o nome das ferramentas de cada servidor; a definição é lida do disco quando usa.", mcps),
        group("agents", "Subagentes", "Como chamar e criar subagentes.", agents),
        group("hooks", "Hooks de início", "Texto que os hooks injetaram antes da 1ª resposta (SYSTEM_MESSAGE).", hooks),
        group("session", "Sua primeira mensagem", "Conta na 1ª resposta medida.", first),
    ]
    .into_iter()
    .flatten()
    .collect();
    ctx.startup_tokens = ctx.startup.iter().map(|g| g.tokens).sum();
    ctx.per_turn = group(
        "hooks",
        "Hooks e avisos repetidos",
        "Texto que os hooks e o agy acrescentaram entre as mensagens, somado na conversa.",
        repeats.items(),
    )
    .into_iter()
    .collect();
    ctx.on_demand = skill_bodies(&ctx.startup_items("skills"), home, "agy");
    Ok(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn codex_fixture() -> Vec<Value> {
        let msg = |role: &str, text: &str| json!({"type":"response_item","timestamp":"2026-11-24T20:00:00Z","payload":{"type":"message","role":role,"content":[{"type":"input_text","text":text}]}});
        vec![
            json!({"type":"session_meta","payload":{"id":"abc","cwd":"/p","timestamp":"2026-11-24T20:00:00Z","cli_version":"0.171.0"}}),
            json!({"type":"response_item","payload":{"type":"additional_tools","tools":[
                {"type":"namespace","name":"functions","tools":[{"type":"custom","name":"exec","description":"x".repeat(400)}]},
                {"type":"namespace","name":"mcp__cua","tools":[{"type":"function","name":"js"}]}
            ]}}),
            msg("developer", &format!("You are Codex, {}", "a".repeat(4000))),
            msg("developer", "<skills_instructions>\n## Skills\n### Skill roots\n- `r0` = `/h/.agents/skills`\n### Available skills\n- imagegen: Make images. (file: r0/imagegen/SKILL.md)\n- deploy: Ship it. (file: r0/deploy/SKILL.md)\n</skills_instructions>"),
            msg("developer", "<tools>\nDeferred tool namespaces:\n- mcp__ai_memory: Long-term memory.\n- mcp__context7\n</tools>"),
            msg("user", "# AGENTS.md instructions\n\n<INSTRUCTIONS>\nResponder em pt-BR.\n</INSTRUCTIONS>"),
            msg("user", "<environment_context>\n  <cwd>/p</cwd>\n</environment_context>"),
            json!({"type":"turn_context","payload":{"model":"gpt-6.2-sol"}}),
            msg("developer", "> 🧭 **ai-memory: your usual choices**\n> stuff"),
            msg("user", "Rode o date"),
            json!({"type":"response_item","payload":{"type":"custom_tool_call","call_id":"c1","name":"exec","input":"date"}}),
            json!({"type":"token_usage_record","payload":{"usage":{"input_tokens":20000,"cached_input_tokens":0,"output_tokens":50}}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"c1","output":[{"type":"input_text","text":"17:47"}]}}),
            msg("assistant", "17:47"),
            json!({"type":"token_usage_record","payload":{"usage":{"input_tokens":20100,"cached_input_tokens":19000,"output_tokens":5}}}),
            msg("developer", "> 🧭 **ai-memory: your usual choices**\n> stuff"),
            msg("user", "ok?"),
            json!({"type":"token_usage_record","payload":{"usage":{"input_tokens":20400,"cached_input_tokens":20000,"output_tokens":2}}}),
        ]
    }

    #[test]
    fn codex_rollout_becomes_groups_and_turns() {
        let ctx = parse_codex_lines(
            Path::new("/x.jsonl"),
            Path::new("/h"),
            codex_fixture().into_iter(),
        );
        assert_eq!(ctx.cli, "codex");
        assert_eq!(ctx.session_id, "abc");
        assert_eq!(ctx.cli_version.as_deref(), Some("0.171.0"));
        assert_eq!(ctx.model.as_deref(), Some("gpt-6.2-sol"));
        let m = ctx.measured.clone().unwrap();
        assert_eq!(m.total, 20000);
        let keys: Vec<&str> = ctx.startup.iter().map(|g| g.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "base",
                "instructions",
                "skills",
                "mcpTools",
                "hooks",
                "session"
            ]
        );
        let skills = ctx.startup_labels("skills");
        assert!(skills.contains(&"imagegen".to_string()) && skills.contains(&"deploy".to_string()));
        let img = ctx
            .startup
            .iter()
            .find(|g| g.key == "skills")
            .unwrap()
            .items
            .iter()
            .find(|i| i.label == "imagegen")
            .unwrap();
        assert_eq!(
            img.source.as_deref(),
            Some("~/.agents/skills/imagegen/SKILL.md")
        );
        let mcps = ctx.startup_labels("mcpTools");
        assert!(
            mcps.contains(&"ai_memory".to_string())
                && mcps.contains(&"context7".to_string())
                && mcps.contains(&"cua".to_string())
        );
        assert!(ctx
            .startup_labels("base")
            .iter()
            .any(|l| l.starts_with("Ferramentas nativas (1)")));
        assert_eq!(
            ctx.startup_labels("hooks"),
            ["🧭 ai-memory: your usual choices"]
        );
        // two typed messages; the hook before the 2nd is a repeat
        assert_eq!(ctx.turns.len(), 2);
        assert_eq!(ctx.turns[0].api_calls, 2);
        assert_eq!(ctx.turns[0].context_after, Some(20105));
        assert_eq!(ctx.turns[0].tools[0].label, "exec");
        assert_eq!(ctx.turns[1].growth, Some(20402 - 20105));
        assert_eq!(ctx.per_turn[0].items[0].count, Some(1));
    }

    #[test]
    fn protobuf_reader_walks_nested_fields() {
        // field 1 (len) { field 2 varint 7, field 3 "hi" }, field 4 varint 300
        let inner = [0x10, 0x07, 0x1a, 0x02, b'h', b'i'];
        let mut b = vec![0x0a, inner.len() as u8];
        b.extend_from_slice(&inner);
        b.extend_from_slice(&[0x20, 0xac, 0x02]);
        let m = Msg::parse(&b);
        assert_eq!(m.int(4), Some(300));
        let sub = m.msg(1).unwrap();
        assert_eq!(sub.int(2), Some(7));
        assert_eq!(sub.str(3), Some("hi"));
        assert!(pb::fields(&[0x0a, 0x09, 0x01]).is_none());
    }

    #[test]
    fn agy_sections_split_into_items() {
        let rules = "<user_rules>\nheader\n<RULE[user_global]>\nseja breve\n</RULE[user_global]>\n<RULE[/h/p/AGENTS.md]>\nprojeto\n</RULE[/h/p/AGENTS.md]>\n</user_rules>";
        let (items, header) = agy_rules(rules, Path::new("/h"));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "Regras globais");
        assert_eq!(items[1].label, "AGENTS.md");
        assert_eq!(items[1].source.as_deref(), Some("~/p/AGENTS.md"));
        assert!(header > 0);

        let skills =
            "<skills>\nintro\n- deploy (/h/.agents/skills/deploy/SKILL.md): Ship it.\n</skills>";
        let (items, _) = agy_skills(skills, Path::new("/h"));
        assert_eq!(items[0].label, "deploy");
        assert_eq!(
            items[0].source.as_deref(),
            Some("~/.agents/skills/deploy/SKILL.md")
        );

        let mcp = "<mcp_servers>\nformat:\n```\n# <serverName>\nEager:\n<toolName>\n```\n# ai-memory\nLazy:\nmemory_query\nmemory_status\n# ticktick\nLazy:\nlist_tags\n</mcp_servers>";
        let (items, _) = agy_mcps(mcp);
        let names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(names, ["ai-memory", "ticktick"]);
        assert_eq!(
            items[0].note.as_deref(),
            Some("2 ferramenta(s) listadas só pelo nome")
        );
    }

    #[test]
    fn hook_label_names_the_event() {
        let t = "The following system message was injected by a SessionStart hook:\n> 🧭 **ai-memory: your usual choices** (x)";
        assert_eq!(
            hook_label(t),
            "Hook SessionStart: 🧭 ai-memory: your usual choices (x)"
        );
        assert_eq!(file_uri_path("file:///tmp/a%20b/"), "/tmp/a b");
    }
}
