//! "Contexto" tab: what a Claude Code session really put into the model's
//! context, read from the session log (`~/.claude/projects/<slug>/<id>.jsonl`).
//!
//! Claude Code logs, before the first reply, one `attachment` record per
//! piece of injected context (instruction files, the skill listing, MCP
//! server instructions, deferred tool names, the subagent listing, hook
//! output...) plus the request shape (system prompt + native tool
//! definitions) and, on the first reply, the real token usage. This module
//! turns that into groups with their source and size, so the tab can show
//! where the tokens go. Sizes are characters; tokens are estimated as
//! chars/4 and the measured total comes from the API usage.
//!
//! The log format is internal to Claude Code and changes between versions:
//! everything here is lenient (unknown records land in "Outros", missing
//! records just leave a group empty, with a warning).

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Rough tokens for a text size (no tokenizer bundled).
pub fn estimate_tokens(chars: usize) -> usize {
    chars.div_ceil(4)
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Folder name Claude Code uses for a project under `~/.claude/projects`:
/// every character that is not ASCII alphanumeric becomes `-`.
pub fn claude_project_slug(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

pub fn claude_projects_dir(home: &Path) -> PathBuf {
    home.join(".claude").join("projects")
}

// ─── Output types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextItem {
    pub label: String,
    /// File, command or plugin the text came from, when known.
    pub source: Option<String>,
    pub chars: usize,
    pub tokens: usize,
    /// Short note shown under the item (e.g. output cut by Claude Code).
    pub note: Option<String>,
    /// Times it ran (per-message items).
    pub count: Option<usize>,
}

impl ContextItem {
    fn new(label: impl Into<String>, source: Option<String>, chars: usize) -> Self {
        ContextItem {
            label: label.into(),
            source,
            chars,
            tokens: estimate_tokens(chars),
            note: None,
            count: None,
        }
    }
    fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextGroup {
    pub key: String,
    pub title: String,
    pub hint: String,
    pub items: Vec<ContextItem>,
    pub chars: usize,
    pub tokens: usize,
}

impl ContextGroup {
    fn new(key: &str, title: &str, hint: &str, mut items: Vec<ContextItem>) -> Self {
        items.sort_by(|a, b| b.chars.cmp(&a.chars));
        let chars = items.iter().map(|i| i.chars).sum();
        ContextGroup {
            key: key.into(),
            title: title.into(),
            hint: hint.into(),
            tokens: estimate_tokens(chars),
            chars,
            items,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MeasuredUsage {
    pub input_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    /// Everything the model read for the first reply.
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionContext {
    pub session_id: String,
    pub file: String,
    pub cwd: Option<String>,
    pub started_at: Option<String>,
    pub claude_version: Option<String>,
    pub model: Option<String>,
    pub title: Option<String>,
    /// Token usage of the first reply (the real number).
    pub measured: Option<MeasuredUsage>,
    /// Loaded before the first reply.
    pub startup: Vec<ContextGroup>,
    pub startup_tokens: usize,
    /// Injected again during the session (hooks, reminders), totals.
    pub per_turn: Vec<ContextGroup>,
    /// Real prompts typed in the session.
    pub prompts: usize,
    /// Loaded only when used (skill/subagent bodies); filled by the caller,
    /// which knows the folders.
    pub on_demand: Vec<ContextGroup>,
    /// How the context grew, message by message.
    pub turns: Vec<TurnStat>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TurnStat {
    /// 1-based position in the session.
    pub index: usize,
    pub at: Option<String>,
    /// Start of what was typed (or a label for automatic messages).
    pub prompt: String,
    /// `prompt` (typed) or `task` (background task notice).
    pub kind: String,
    pub images: usize,
    /// Real context size after this message's last reply (API usage).
    pub context_after: Option<u64>,
    /// Real growth against the previous message (negative after a compaction).
    pub growth: Option<i64>,
    /// Model calls in this message (one per tool round).
    pub api_calls: usize,
    /// Tokens the model wrote (reasoning included).
    pub output_tokens: u64,
    pub compacted: bool,
    /// Estimated split of what was added (chars/4).
    pub parts: Vec<ContextItem>,
    /// Tool results by tool, biggest first.
    pub tools: Vec<ContextItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRef {
    pub session_id: String,
    pub modified: Option<String>,
    pub size_bytes: u64,
    pub title: Option<String>,
}

// ─── Session listing ────────────────────────────────────────────────────────

/// Read the title (`ai-title`, else the first prompt) from the head of a log.
fn session_title(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut first_prompt = None;
    for line in BufReader::new(file).lines().take(400).map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        match v.get("type").and_then(Value::as_str) {
            Some("ai-title") => {
                if let Some(t) = v.get("aiTitle").and_then(Value::as_str) {
                    return Some(t.to_string());
                }
            }
            Some("user") if first_prompt.is_none() => {
                if let Some(t) = prompt_text(&v) {
                    first_prompt = Some(t.chars().take(100).collect());
                }
            }
            _ => {}
        }
    }
    first_prompt
}

/// Most recent sessions of a folder (by modification time).
pub fn list_sessions(home: &Path, cwd: &Path, limit: usize) -> Vec<SessionRef> {
    let dir = claude_projects_dir(home).join(claude_project_slug(cwd));
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf, u64)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("jsonl"))
        .filter_map(|p| {
            let meta = std::fs::metadata(&p).ok()?;
            Some((meta.modified().ok()?, p, meta.len()))
        })
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files
        .into_iter()
        .take(limit)
        .map(|(mtime, path, size)| SessionRef {
            session_id: path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            modified: Some(chrono::DateTime::<chrono::Utc>::from(mtime).to_rfc3339()),
            size_bytes: size,
            title: session_title(&path),
        })
        .collect()
}

impl SessionContext {
    /// Labels of the items of one startup group (skill or agent names).
    pub fn startup_labels(&self, key: &str) -> Vec<String> {
        self.startup
            .iter()
            .filter(|g| g.key == key)
            .flat_map(|g| g.items.iter().map(|i| i.label.clone()))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextFolder {
    pub path: String,
    pub sessions: usize,
    pub last_used: Option<String>,
    /// The folder still exists on disk.
    pub exists: bool,
}

/// `cwd` recorded at the head of a session log.
fn session_cwd(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    BufReader::new(file)
        .lines()
        .take(300)
        .map_while(Result::ok)
        .find_map(|l| {
            serde_json::from_str::<Value>(&l)
                .ok()?
                .get("cwd")?
                .as_str()
                .map(String::from)
        })
}

/// Folders where Claude Code has sessions, most recently used first. The
/// folder name under `~/.claude/projects` is lossy, so the real path comes
/// from the newest log.
pub fn list_folders(home: &Path, limit: usize) -> Vec<ContextFolder> {
    let Ok(entries) = std::fs::read_dir(claude_projects_dir(home)) else {
        return Vec::new();
    };
    let mut out: Vec<(std::time::SystemTime, ContextFolder)> = Vec::new();
    for dir in entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
    {
        let Ok(files) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut logs: Vec<(std::time::SystemTime, PathBuf)> = files
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("jsonl"))
            .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
            .collect();
        if logs.is_empty() {
            continue;
        }
        logs.sort_by(|a, b| b.0.cmp(&a.0));
        let Some(cwd) = logs.iter().take(3).find_map(|(_, p)| session_cwd(p)) else {
            continue;
        };
        // Logs of another folder that happens to share the slug are skipped.
        if claude_project_slug(Path::new(&cwd))
            != dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        {
            continue;
        }
        let newest = logs[0].0;
        out.push((
            newest,
            ContextFolder {
                exists: Path::new(&cwd).is_dir(),
                path: cwd,
                sessions: logs.len(),
                last_used: Some(chrono::DateTime::<chrono::Utc>::from(newest).to_rfc3339()),
            },
        ));
    }
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter().map(|(_, f)| f).take(limit).collect()
}

pub fn session_file(home: &Path, cwd: &Path, session_id: &str) -> Option<PathBuf> {
    // The id comes from the UI: keep it a plain file name.
    if session_id.is_empty()
        || !session_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    let p = claude_projects_dir(home)
        .join(claude_project_slug(cwd))
        .join(format!("{}.jsonl", session_id));
    p.is_file().then_some(p)
}

// ─── Parsing helpers ────────────────────────────────────────────────────────

/// Text of a real typed prompt (not a tool result, not a meta message).
fn prompt_text(v: &Value) -> Option<String> {
    if v.get("isMeta").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let content = v.get("message")?.get("content")?;
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => {
            if parts
                .iter()
                .any(|p| p.get("type").and_then(Value::as_str) == Some("tool_result"))
            {
                return None;
            }
            parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => return None,
    };
    let t = text.trim();
    // Slash-command plumbing and caveats are not typed prompts.
    if t.is_empty() || t.starts_with("<local-command") || t.starts_with("<command-") {
        return None;
    }
    Some(t.to_string())
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn str_list(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn human_kb(chars: usize) -> String {
    format!("{:.1} KB", chars as f64 / 1024.0)
}

/// What a hook put into the context, and a note when Claude Code cut it.
/// `content` is what the model saw for plain-text output; JSON output
/// (`hookSpecificOutput.additionalContext`) is carried in stdout.
fn hook_injected(att: &Value) -> (usize, Option<String>) {
    let content = str_of(att, "content");
    let stdout = str_of(att, "stdout");
    if !content.is_empty() {
        let note = if content.starts_with("<persisted-output>") {
            Some(format!(
                "Saída de {} cortada pelo Claude Code: entra só um trecho e o resto vai para um arquivo",
                human_kb(char_len(stdout))
            ))
        } else {
            None
        };
        return (char_len(content), note);
    }
    if let Ok(json) = serde_json::from_str::<Value>(stdout.trim()) {
        let extra = json
            .pointer("/hookSpecificOutput/additionalContext")
            .and_then(Value::as_str)
            .or_else(|| json.get("systemMessage").and_then(Value::as_str))
            .unwrap_or("");
        return (char_len(extra), None);
    }
    (0, None)
}

/// Short label for a hook command (the script name, not the whole line).
pub fn hook_label(command: &str) -> String {
    let cmd = command.trim();
    // `if [ -x ... ]; then exec X; ...` and quoted paths: take the first path-ish token.
    let token = cmd
        .split(|c: char| c.is_whitespace() || c == ';')
        .map(|t| t.trim_matches(|c| c == '"' || c == '\''))
        .find(|t| t.contains('/') && !t.starts_with('-'))
        .unwrap_or(cmd);
    let name = token.rsplit('/').next().unwrap_or(token);
    let mut extra = "";
    if let Some(i) = cmd.find("--event ") {
        extra = cmd[i + 8..].split_whitespace().next().unwrap_or("");
    }
    if extra.is_empty() {
        name.to_string()
    } else {
        format!("{} ({})", name, extra)
    }
}

/// Skill listing: one `- name: description` entry per skill (descriptions
/// may span lines).
fn split_skill_listing(content: &str) -> Vec<(String, usize)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("- ") {
            let name = rest.split(':').next().unwrap_or(rest).trim();
            // Plugin skills are `plugin:skill: desc`.
            let name = match rest.split_once(": ") {
                Some((n, _)) => n.trim(),
                None => name,
            };
            out.push((name.to_string(), format!("{}\n", line)));
        } else if let Some(last) = out.last_mut() {
            last.1.push_str(line);
            last.1.push('\n');
        }
    }
    out.into_iter().map(|(n, t)| (n, char_len(&t))).collect()
}

/// Server part of a deferred tool name (`mcp__<server>__<tool>`).
fn mcp_server_of(tool: &str) -> Option<&str> {
    let rest = tool.strip_prefix("mcp__")?;
    rest.split("__").next()
}

fn instruction_label(kind: &str) -> &'static str {
    match kind {
        "User" => "Pessoal (global)",
        "Project" => "Projeto",
        "Local" => "Local do projeto",
        "Managed" => "Gerenciado",
        _ => "Instrução",
    }
}

fn attachment_label(kind: &str) -> String {
    match kind {
        "session_context" => "Contexto da sessão (git status, e-mail)".into(),
        "environment" => "Ambiente (pasta, sistema)".into(),
        "model" => "Modelo em uso".into(),
        "date" => "Data de hoje".into(),
        "auto_mode" => "Modo automático".into(),
        "credential_org" => "Organização da conta".into(),
        "remote_session_change" => "Atribuição de commits/PR".into(),
        "task_reminder" => "Lembrete da lista de tarefas".into(),
        "silent_turn_reminder" => "Lembrete de turno silencioso".into(),
        "deferred_tools_record" => "Registro de ferramentas adiadas".into(),
        "edited_text_file" => "Aviso de arquivo editado fora da sessão".into(),
        "queued_command" => "Mensagem enviada na fila".into(),
        other => other.replace('_', " "),
    }
}

/// Attachments that are bookkeeping for the log, not model context.
fn is_log_only(kind: &str) -> bool {
    matches!(kind, "prompt_snapshot")
}

fn text_size_of(v: &Value) -> usize {
    match v {
        Value::String(s) => char_len(s),
        Value::Array(a) => a.iter().map(text_size_of).sum(),
        Value::Object(o) => o.values().map(text_size_of).sum(),
        Value::Number(n) => n.to_string().len(),
        Value::Bool(_) => 4,
        Value::Null => 0,
    }
}

// ─── Message by message ─────────────────────────────────────────────────────

/// Rough size of one image in the context (~1.6k tokens for a screenshot).
const IMAGE_CHARS: usize = 6400;

fn count_images(v: &Value) -> usize {
    match v {
        Value::Array(a) => a.iter().map(count_images).sum(),
        Value::Object(o) => {
            usize::from(o.get("type").and_then(Value::as_str) == Some("image"))
                + o.get("content").map(count_images).unwrap_or(0)
        }
        _ => 0,
    }
}

/// Text size of a tool result's content (strings and text blocks).
fn result_chars(v: &Value) -> usize {
    match v {
        Value::String(s) => char_len(s),
        Value::Array(a) => a.iter().map(result_chars).sum(),
        Value::Object(o) => match o.get("type").and_then(Value::as_str) {
            Some("text") => o
                .get("text")
                .and_then(Value::as_str)
                .map(char_len)
                .unwrap_or(0),
            Some("image") => 0,
            _ => o.get("content").map(result_chars).unwrap_or(0),
        },
        _ => 0,
    }
}

fn usage_input(u: &Value) -> u64 {
    let g = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
    g("input_tokens") + g("cache_creation_input_tokens") + g("cache_read_input_tokens")
}

#[derive(Default)]
struct TurnAcc {
    index: usize,
    at: Option<String>,
    prompt: String,
    kind: String,
    images: usize,
    prompt_chars: usize,
    hook_chars: usize,
    att_chars: usize,
    reply_chars: usize,
    tools: BTreeMap<String, (usize, usize)>,
    api_calls: usize,
    output_tokens: u64,
    first_input: Option<u64>,
    last_input: Option<u64>,
    last_output: u64,
    compacted: bool,
}

#[derive(Default)]
struct TurnTracker {
    turns: Vec<TurnStat>,
    cur: Option<TurnAcc>,
    tool_names: std::collections::HashMap<String, String>,
    seen_msgs: std::collections::HashSet<String>,
    /// Context after the previous message.
    last_ctx: Option<u64>,
}

impl TurnTracker {
    /// A user record that opens a new message: typed text, images, or a
    /// background task notice. Tool results and meta records do not.
    fn opens_turn(v: &Value) -> Option<(String, String, usize)> {
        if v.get("isMeta").and_then(Value::as_bool) == Some(true) {
            return None;
        }
        let content = v.get("message")?.get("content")?;
        if let Value::Array(parts) = content {
            if parts
                .iter()
                .any(|p| p.get("type").and_then(Value::as_str) == Some("tool_result"))
            {
                return None;
            }
        }
        let images = count_images(content);
        let text = prompt_text(v).unwrap_or_default();
        if text.starts_with("<task-notification>") {
            return Some(("Aviso de tarefa em segundo plano".into(), "task".into(), 0));
        }
        if text.is_empty() && images == 0 {
            return None;
        }
        let shown: String = if text.is_empty() {
            "(só imagem)".into()
        } else {
            text.chars().take(160).collect()
        };
        Some((shown, "prompt".into(), images))
    }

    fn feed(&mut self, v: &Value, started: bool) {
        if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            return;
        }
        match str_of(v, "type") {
            "user" => {
                if let Some((prompt, kind, images)) = Self::opens_turn(v) {
                    self.close();
                    let chars = prompt_text(v).map(|t| char_len(&t)).unwrap_or(0);
                    self.cur = Some(TurnAcc {
                        index: self.turns.len() + 1,
                        at: v.get("timestamp").and_then(Value::as_str).map(String::from),
                        prompt,
                        kind,
                        images,
                        prompt_chars: chars + images * IMAGE_CHARS,
                        ..Default::default()
                    });
                    return;
                }
                let Some(cur) = self.cur.as_mut() else { return };
                let Some(Value::Array(parts)) = v.get("message").and_then(|m| m.get("content"))
                else {
                    return;
                };
                for p in parts {
                    if p.get("type").and_then(Value::as_str) != Some("tool_result") {
                        continue;
                    }
                    let id = str_of(p, "tool_use_id");
                    let name = self
                        .tool_names
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| "?".into());
                    let content = p.get("content").unwrap_or(&Value::Null);
                    let chars = result_chars(content) + count_images(content) * IMAGE_CHARS;
                    let e = cur.tools.entry(name).or_default();
                    e.0 += 1;
                    e.1 += chars;
                }
            }
            "assistant" => {
                let Some(msg) = v.get("message") else { return };
                for block in msg
                    .get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    match str_of(block, "type") {
                        "tool_use" => {
                            self.tool_names.insert(
                                str_of(block, "id").to_string(),
                                str_of(block, "name").to_string(),
                            );
                            if let Some(cur) = self.cur.as_mut() {
                                cur.reply_chars += block
                                    .get("input")
                                    .and_then(|i| serde_json::to_string(i).ok())
                                    .map(|s| char_len(&s))
                                    .unwrap_or(0);
                            }
                        }
                        "text" => {
                            if let Some(cur) = self.cur.as_mut() {
                                cur.reply_chars += char_len(str_of(block, "text"));
                            }
                        }
                        _ => {}
                    }
                }
                // One API call is logged as several records (one per block).
                let id = str_of(msg, "id").to_string();
                if id.is_empty() || !self.seen_msgs.insert(id) {
                    return;
                }
                let (Some(cur), Some(u)) = (self.cur.as_mut(), msg.get("usage")) else {
                    return;
                };
                let input = usage_input(u);
                if input == 0 {
                    return;
                }
                if cur.last_input.is_some_and(|prev| input + 2000 < prev) {
                    cur.compacted = true;
                }
                cur.api_calls += 1;
                cur.first_input.get_or_insert(input);
                cur.last_input = Some(input);
                cur.last_output = u.get("output_tokens").and_then(Value::as_u64).unwrap_or(0);
                cur.output_tokens += cur.last_output;
            }
            "attachment" if started => {
                let Some(cur) = self.cur.as_mut() else { return };
                let Some(att) = v.get("attachment") else {
                    return;
                };
                let akind = str_of(att, "type");
                if akind.starts_with("hook_") {
                    if akind != "hook_additional_context" {
                        cur.hook_chars += hook_injected(att).0;
                    }
                } else if !is_log_only(akind) {
                    cur.att_chars += text_size_of(att).saturating_sub(char_len(akind));
                }
            }
            "system" if str_of(v, "subtype") == "compact_boundary" => {
                if let Some(cur) = self.cur.as_mut() {
                    cur.compacted = true;
                }
            }
            _ => {}
        }
    }

    fn close(&mut self) {
        let Some(t) = self.cur.take() else { return };
        let context_after = t.last_input.map(|i| i + t.last_output);
        // The first message is measured from its first call (which already
        // holds everything loaded at start).
        let before = self.last_ctx.or(t.first_input);
        let growth = match (context_after, before) {
            (Some(a), Some(b)) => Some(a as i64 - b as i64),
            _ => None,
        };
        if context_after.is_some() {
            self.last_ctx = context_after;
        }
        let compacted = t.compacted || growth.is_some_and(|g| g < -2000);
        let mut parts = vec![
            ContextItem::new("Sua mensagem", None, t.prompt_chars),
            ContextItem::new(
                "Resultados de ferramentas",
                None,
                t.tools.values().map(|(_, c)| c).sum(),
            ),
            ContextItem::new("Hooks", None, t.hook_chars),
            ContextItem::new("Anexos do Claude Code", None, t.att_chars),
        ];
        if t.images > 0 {
            parts[0].note = Some(format!("{} imagem(ns), ~1,6 mil tokens cada", t.images));
        }
        // The model's own output is measured (reasoning included: it stays
        // in the context while the message runs its tools).
        let mut reply = ContextItem::new("Respostas do Claude", None, t.reply_chars);
        if t.output_tokens > 0 {
            reply.tokens = t.output_tokens as usize;
            reply.note = Some("Medido: texto, chamadas de ferramenta e raciocínio".into());
        }
        parts.push(reply);
        parts.retain(|p| p.tokens > 0);
        if let Some(g) = growth.filter(|g| *g > 0 && !compacted) {
            let known: usize = parts.iter().map(|p| p.tokens).sum();
            let rest = g as usize;
            if rest > known {
                let mut r = ContextItem::new(
                    "Resto (embalagem do Claude Code e diferença da estimativa)",
                    None,
                    0,
                );
                r.tokens = rest - known;
                parts.push(r);
            }
        }
        parts.sort_by(|a, b| b.tokens.cmp(&a.tokens));
        let mut tools: Vec<ContextItem> = t
            .tools
            .into_iter()
            .map(|(name, (n, chars))| {
                let mut i = ContextItem::new(name, None, chars);
                i.count = Some(n);
                i
            })
            .collect();
        tools.sort_by(|a, b| b.chars.cmp(&a.chars));
        self.turns.push(TurnStat {
            index: t.index,
            at: t.at,
            prompt: t.prompt,
            kind: t.kind,
            images: t.images,
            context_after,
            growth,
            api_calls: t.api_calls,
            output_tokens: t.output_tokens,
            compacted,
            parts,
            tools,
        });
    }
}

// ─── Main parser ────────────────────────────────────────────────────────────

#[derive(Default)]
struct HookAgg {
    chars: usize,
    runs: usize,
    injected_runs: usize,
}

pub fn parse_session(path: &Path) -> std::io::Result<SessionContext> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    parse_lines(
        path,
        reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|l| serde_json::from_str::<Value>(&l).ok()),
    )
}

pub fn parse_lines(
    path: &Path,
    records: impl Iterator<Item = Value>,
) -> std::io::Result<SessionContext> {
    let session_id = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut ctx = SessionContext {
        session_id,
        file: path.to_string_lossy().to_string(),
        cwd: None,
        started_at: None,
        claude_version: None,
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

    let mut base: Vec<ContextItem> = Vec::new();
    let mut native_tools: Vec<ContextItem> = Vec::new();
    let mut instructions: Vec<ContextItem> = Vec::new();
    let mut skills: Vec<ContextItem> = Vec::new();
    let mut mcp_instr: Vec<ContextItem> = Vec::new();
    let mut mcp_tools: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut mcp_defs: Vec<ContextItem> = Vec::new();
    let mut agents: Vec<ContextItem> = Vec::new();
    let mut start_hooks: Vec<ContextItem> = Vec::new();
    let mut session_bits: BTreeMap<String, usize> = BTreeMap::new();
    let mut first_prompt_chars: Option<usize> = None;
    let mut seen_shape = false;
    let mut seen_skills = false;
    let mut seen_instructions = false;

    let mut turn_hooks: BTreeMap<(String, String), HookAgg> = BTreeMap::new();
    let mut turn_bits: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    let mut started = false; // first main-thread reply seen
    let mut tracker = TurnTracker::default();

    for v in records {
        tracker.feed(&v, started);
        let kind = str_of(&v, "type");
        if ctx.cwd.is_none() {
            if let Some(c) = v.get("cwd").and_then(Value::as_str) {
                ctx.cwd = Some(c.to_string());
            }
        }
        if ctx.started_at.is_none() {
            if let Some(t) = v.get("timestamp").and_then(Value::as_str) {
                ctx.started_at = Some(t.to_string());
            }
        }
        match kind {
            "ai-title" => {
                if let Some(t) = v.get("aiTitle").and_then(Value::as_str) {
                    ctx.title = Some(t.to_string());
                }
            }
            "user" => {
                if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
                    continue;
                }
                if let Some(text) = prompt_text(&v) {
                    ctx.prompts += 1;
                    if !started && first_prompt_chars.is_none() {
                        first_prompt_chars = Some(char_len(&text));
                    }
                }
            }
            "api-request-shape" if !started && !seen_shape => {
                seen_shape = true;
                if ctx.claude_version.is_none() {
                    ctx.claude_version = v.get("version").and_then(Value::as_str).map(String::from);
                }
                if let Some(shape) = v.get("shape") {
                    let system_chars: usize = shape
                        .get("system")
                        .and_then(Value::as_array)
                        .map(|a| a.iter().map(|b| char_len(str_of(b, "text"))).sum())
                        .unwrap_or(0);
                    if system_chars > 0 {
                        base.push(ContextItem::new(
                            "Prompt do sistema do Claude Code",
                            Some("Claude Code".into()),
                            system_chars,
                        ));
                    }
                    for tool in shape
                        .get("tools")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        let name = str_of(tool, "name");
                        let size = serde_json::to_string(tool)
                            .map(|s| char_len(&s))
                            .unwrap_or(0);
                        native_tools.push(ContextItem::new(name, None, size));
                    }
                }
            }
            "assistant" => {
                if v.get("isSidechain").and_then(Value::as_bool) == Some(true) || started {
                    continue;
                }
                started = true;
                let msg = v.get("message");
                ctx.model = msg
                    .and_then(|m| m.get("model"))
                    .and_then(Value::as_str)
                    .map(String::from);
                if ctx.claude_version.is_none() {
                    ctx.claude_version = v.get("version").and_then(Value::as_str).map(String::from);
                }
                if let Some(u) = msg.and_then(|m| m.get("usage")) {
                    let g = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
                    let m = MeasuredUsage {
                        input_tokens: g("input_tokens"),
                        cache_creation_tokens: g("cache_creation_input_tokens"),
                        cache_read_tokens: g("cache_read_input_tokens"),
                        total: 0,
                    };
                    let total = m.input_tokens + m.cache_creation_tokens + m.cache_read_tokens;
                    ctx.measured = Some(MeasuredUsage { total, ..m });
                }
            }
            "attachment" => {
                let Some(att) = v.get("attachment") else {
                    continue;
                };
                let akind = str_of(att, "type");
                let event = str_of(att, "hookEvent");
                if akind == "hook_success"
                    || akind == "hook_error"
                    || akind == "hook_blocking_error"
                {
                    let command = str_of(att, "command").to_string();
                    let (chars, note) = hook_injected(att);
                    if !started && event == "SessionStart" {
                        let mut item = ContextItem::new(hook_label(&command), Some(command), chars);
                        if chars == 0 {
                            item.note = Some("Rodou e não colocou nada no contexto".into());
                        } else if let Some(n) = note {
                            item.note = Some(n);
                        }
                        start_hooks.push(item);
                    } else {
                        let agg = turn_hooks.entry((event.to_string(), command)).or_default();
                        agg.runs += 1;
                        if chars > 0 {
                            agg.injected_runs += 1;
                            agg.chars += chars;
                        }
                    }
                    continue;
                }
                if akind == "hook_additional_context" || is_log_only(akind) {
                    // Counted from the matching hook_success stdout.
                    continue;
                }
                if started {
                    // Later skill/MCP deltas re-list what changed; count them as per-turn.
                    let e = turn_bits.entry(akind.to_string()).or_default();
                    e.0 += 1;
                    e.1 += text_size_of(att) - char_len(akind);
                    continue;
                }
                match akind {
                    "instructions" => {
                        seen_instructions = true;
                        for f in att
                            .get("files")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                        {
                            let p = str_of(f, "path");
                            let name = Path::new(p)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| p.to_string());
                            instructions.push(ContextItem::new(
                                format!("{} · {}", name, instruction_label(str_of(f, "type"))),
                                Some(p.to_string()),
                                char_len(str_of(f, "content")),
                            ));
                        }
                    }
                    "skill_listing" => {
                        seen_skills = true;
                        for (name, chars) in split_skill_listing(str_of(att, "content")) {
                            let source = name.split_once(':').map(|(p, _)| format!("plugin {}", p));
                            skills.push(ContextItem::new(name, source, chars));
                        }
                    }
                    "mcp_instructions_delta" => {
                        let names = str_list(att, "addedNames");
                        let blocks = str_list(att, "addedBlocks");
                        for (i, block) in blocks.iter().enumerate() {
                            let name = names
                                .get(i)
                                .cloned()
                                .unwrap_or_else(|| format!("MCP {}", i + 1));
                            mcp_instr.push(ContextItem::new(name, None, char_len(block)));
                        }
                    }
                    "deferred_tools_delta" => {
                        for line in str_list(att, "addedLines") {
                            let server = mcp_server_of(&line)
                                .map(String::from)
                                .unwrap_or_else(|| "Claude Code (ferramentas próprias)".into());
                            let e = mcp_tools.entry(server).or_default();
                            e.0 += 1;
                            e.1 += char_len(&line) + 1;
                        }
                        for def in att
                            .get("surfacedDefinitions")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                        {
                            let name = str_of(def, "name");
                            let size = def
                                .get("definition")
                                .and_then(|d| serde_json::to_string(d).ok())
                                .map(|s| char_len(&s))
                                .unwrap_or(0);
                            let server = mcp_server_of(name).unwrap_or("?").to_string();
                            mcp_defs.push(ContextItem::new(name, Some(server), size));
                        }
                    }
                    "agent_listing_delta" => {
                        for line in str_list(att, "addedLines") {
                            let name = line
                                .trim_start_matches("- ")
                                .split(':')
                                .next()
                                .unwrap_or("")
                                .to_string();
                            agents.push(ContextItem::new(name, None, char_len(&line) + 1));
                        }
                    }
                    other => {
                        *session_bits.entry(other.to_string()).or_default() +=
                            text_size_of(att).saturating_sub(char_len(other));
                    }
                }
            }
            _ => {}
        }
    }

    if !seen_shape {
        ctx.warnings.push(
            "Esta versão do Claude Code não grava o prompt do sistema no log; a parte fixa do Claude Code fica de fora da estimativa.".into(),
        );
    }
    if !seen_skills && !seen_instructions {
        ctx.warnings.push(
            "O log desta sessão não traz a lista do que foi carregado (versão antiga do Claude Code?). Abra uma sessão nova e atualize.".into(),
        );
    }

    // ── Startup groups ──
    let mut startup = Vec::new();
    let mut claude_items = base;
    let tools_chars: usize = native_tools.iter().map(|t| t.chars).sum();
    if tools_chars > 0 {
        let n = native_tools.len();
        let mut biggest: Vec<&ContextItem> = native_tools.iter().collect();
        biggest.sort_by(|a, b| b.chars.cmp(&a.chars));
        let top: Vec<String> = biggest
            .iter()
            .take(3)
            .map(|t| format!("{} ({})", t.label, human_kb(t.chars)))
            .collect();
        claude_items.push(
            ContextItem::new(
                format!("Ferramentas nativas ({})", n),
                Some("Claude Code".into()),
                tools_chars,
            )
            .note(format!("Maiores: {}", top.join(", "))),
        );
    }
    let push = |v: &mut Vec<ContextGroup>, g: ContextGroup| {
        if !g.items.is_empty() {
            v.push(g);
        }
    };
    push(&mut startup, ContextGroup::new(
        "claude",
        "Claude Code (fixo)",
        "Prompt do sistema e ferramentas nativas que vêm com o Claude Code. Não dá pra mexer por aqui.",
        claude_items,
    ));
    push(
        &mut startup,
        ContextGroup::new(
            "instructions",
            "Instruções (CLAUDE.md / AGENTS.md)",
            "Arquivos de instrução carregados inteiros, incluindo os @imports.",
            instructions,
        ),
    );
    push(
        &mut startup,
        ContextGroup::new(
            "skills",
            "Skills (nome + descrição)",
            "Só a linha de cada skill entra no começo; o corpo entra quando a skill é usada.",
            skills,
        ),
    );
    push(
        &mut startup,
        ContextGroup::new(
            "mcpInstructions",
            "MCPs: instruções do servidor",
            "Texto que cada servidor MCP manda ao conectar. Entra inteiro em toda sessão.",
            mcp_instr,
        ),
    );
    let mut tool_items: Vec<ContextItem> = mcp_tools
        .into_iter()
        .map(|(server, (n, chars))| {
            ContextItem::new(server, None, chars).note(format!("{} ferramenta(s), só o nome", n))
        })
        .collect();
    tool_items.extend(
        mcp_defs
            .into_iter()
            .map(|d| d.note("Definição completa já carregada")),
    );
    push(&mut startup, ContextGroup::new(
        "mcpTools",
        "MCPs: ferramentas",
        "O Claude Code lista só o nome de cada ferramenta; a definição completa entra quando ela é usada.",
        tool_items,
    ));
    push(
        &mut startup,
        ContextGroup::new(
            "agents",
            "Subagentes (nome + descrição)",
            "Lista de subagentes que o Claude pode chamar.",
            agents,
        ),
    );
    push(
        &mut startup,
        ContextGroup::new(
            "hooks",
            "Hooks de início (SessionStart)",
            "Saída dos comandos que rodam ao abrir a sessão.",
            start_hooks,
        ),
    );
    let mut misc: Vec<ContextItem> = session_bits
        .into_iter()
        .filter(|(_, c)| *c > 0)
        .map(|(k, c)| ContextItem::new(attachment_label(&k), None, c))
        .collect();
    if let Some(c) = first_prompt_chars {
        misc.push(ContextItem::new("Sua primeira mensagem", None, c));
    }
    push(
        &mut startup,
        ContextGroup::new(
            "session",
            "Sessão e primeira mensagem",
            "Informações da sessão que o Claude Code anexa, mais o que você digitou.",
            misc,
        ),
    );
    ctx.startup_tokens = startup.iter().map(|g| g.tokens).sum();
    ctx.startup = startup;

    // ── Per-turn groups ──
    let mut hook_items: Vec<ContextItem> = Vec::new();
    for ((event, command), agg) in turn_hooks {
        let mut item = ContextItem::new(
            format!("{} · {}", event, hook_label(&command)),
            Some(command),
            agg.chars,
        );
        item.count = Some(agg.runs);
        item.note = Some(if agg.injected_runs == 0 {
            format!("Rodou {} vez(es) e não colocou nada no contexto", agg.runs)
        } else {
            format!(
                "Colocou texto em {} de {} execuções (média {} por vez)",
                agg.injected_runs,
                agg.runs,
                human_kb(agg.chars / agg.injected_runs.max(1))
            )
        });
        hook_items.push(item);
    }
    let mut per_turn = Vec::new();
    push(&mut per_turn, ContextGroup::new(
        "hooks",
        "Hooks durante a sessão",
        "UserPromptSubmit roda a cada mensagem; PreToolUse/PostToolUse a cada ferramenta. Só pesa se imprimir algo.",
        hook_items,
    ));
    let bit_items: Vec<ContextItem> = turn_bits
        .into_iter()
        .filter(|(_, (_, c))| *c > 0)
        .map(|(k, (n, c))| {
            let mut i = ContextItem::new(attachment_label(&k), None, c);
            i.count = Some(n);
            i
        })
        .collect();
    push(
        &mut per_turn,
        ContextGroup::new(
            "reminders",
            "Anexos do Claude Code durante a sessão",
            "Lembretes e avisos que o próprio Claude Code acrescenta entre as mensagens.",
            bit_items,
        ),
    );
    ctx.per_turn = per_turn;
    tracker.close();
    ctx.turns = tracker.turns;

    Ok(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn att(a: Value) -> Value {
        json!({"type": "attachment", "attachment": a, "cwd": "/p"})
    }

    fn sample() -> Vec<Value> {
        vec![
            att(
                json!({"type": "hook_success", "hookEvent": "SessionStart", "command": "/x/hooks/onde-estamos.sh",
                "content": "<persisted-output>\nresumo", "stdout": "a".repeat(4000)}),
            ),
            att(json!({"type": "hook_success", "hookEvent": "SessionStart",
                "command": "\"$HOME/bin/ai-memory\" hook --event session-start", "content": "",
                "stdout": "{\"hookSpecificOutput\":{\"additionalContext\":\"12345678\"}}"})),
            att(json!({"type": "hook_additional_context", "content": ["12345678"]})),
            json!({"type": "user", "message": {"content": "oi tudo bem"}, "timestamp": "2026-11-24T10:00:00Z"}),
            att(json!({"type": "instructions", "files": [
                {"path": "/h/.claude/CLAUDE.md", "type": "User", "content": "abcd"},
                {"path": "/p/CLAUDE.md", "type": "Project", "content": "abcdefgh"}]})),
            att(
                json!({"type": "skill_listing", "content": "- alpha: faz A\n  continua\n- higgs:gen: gera\n"}),
            ),
            att(
                json!({"type": "mcp_instructions_delta", "addedNames": ["ai-memory"], "addedBlocks": ["## ai-memory\nxx"]}),
            ),
            att(
                json!({"type": "deferred_tools_delta", "addedLines": ["CronCreate", "mcp__ticktick__a", "mcp__ticktick__bb"],
                "surfacedDefinitions": [{"name": "mcp__docs__x", "definition": {"name": "x"}}]}),
            ),
            att(json!({"type": "agent_listing_delta", "addedLines": ["- Explore: busca"]})),
            att(json!({"type": "date", "date": "2026-11-24"})),
            att(json!({"type": "prompt_snapshot", "systemPrompt": ["ignorado"]})),
            json!({"type": "api-request-shape", "version": "2.1.331", "shape": {
                "system": [{"type": "text", "text": "você é o claude"}],
                "tools": [{"name": "Bash", "description": "roda"}]}}),
            json!({"type": "assistant", "message": {"model": "claude-opus-5-5",
                "usage": {"input_tokens": 2, "cache_creation_input_tokens": 100, "cache_read_input_tokens": 50}}}),
            att(
                json!({"type": "hook_success", "hookEvent": "PostToolUse", "command": "/x/ai-memory hook --event post-tool-use", "content": "", "stdout": ""}),
            ),
            att(
                json!({"type": "hook_success", "hookEvent": "UserPromptSubmit", "command": "/x/ctx.sh", "content": "contexto!", "stdout": "contexto!"}),
            ),
            json!({"type": "user", "message": {"content": [{"type": "tool_result", "content": "x"}]}}),
            json!({"type": "user", "message": {"content": "segunda"}}),
            att(json!({"type": "task_reminder", "content": "faça as tarefas"})),
            json!({"type": "ai-title", "aiTitle": "Teste"}),
        ]
    }

    fn group<'a>(ctx: &'a SessionContext, key: &str) -> &'a ContextGroup {
        ctx.startup
            .iter()
            .find(|g| g.key == key)
            .unwrap_or_else(|| panic!("grupo {}", key))
    }

    #[test]
    fn slug_matches_claude_code() {
        assert_eq!(
            claude_project_slug(Path::new("/home/u/.config/Claude/x_y")),
            "-home-u--config-Claude-x-y"
        );
    }

    #[test]
    fn parses_startup_groups() {
        let ctx = parse_lines(Path::new("/l/abc.jsonl"), sample().into_iter()).unwrap();
        assert_eq!(ctx.session_id, "abc");
        assert_eq!(ctx.cwd.as_deref(), Some("/p"));
        assert_eq!(ctx.model.as_deref(), Some("claude-opus-5-5"));
        assert_eq!(ctx.claude_version.as_deref(), Some("2.1.331"));
        assert_eq!(ctx.title.as_deref(), Some("Teste"));
        assert_eq!(ctx.measured.as_ref().unwrap().total, 152);
        assert_eq!(ctx.prompts, 2);

        let instr = group(&ctx, "instructions");
        assert_eq!(instr.chars, 12);
        assert_eq!(instr.items[0].source.as_deref(), Some("/p/CLAUDE.md"));

        let skills = group(&ctx, "skills");
        assert_eq!(skills.items.len(), 2);
        let higgs = skills
            .items
            .iter()
            .find(|i| i.label == "higgs:gen")
            .unwrap();
        assert_eq!(higgs.source.as_deref(), Some("plugin higgs"));
        let alpha = skills.items.iter().find(|i| i.label == "alpha").unwrap();
        assert!(alpha.chars > "- alpha: faz A\n".len());

        let tools = group(&ctx, "mcpTools");
        let tt = tools.items.iter().find(|i| i.label == "ticktick").unwrap();
        assert_eq!(tt.note.as_deref(), Some("2 ferramenta(s), só o nome"));
        assert!(tools
            .items
            .iter()
            .any(|i| i.label == "Claude Code (ferramentas próprias)"));
        assert!(tools.items.iter().any(|i| i.label == "mcp__docs__x"));

        let hooks = group(&ctx, "hooks");
        let onde = hooks
            .items
            .iter()
            .find(|i| i.label == "onde-estamos.sh")
            .unwrap();
        assert!(onde.note.as_deref().unwrap().contains("cortada"));
        let mem = hooks
            .items
            .iter()
            .find(|i| i.label.starts_with("ai-memory"))
            .unwrap();
        assert_eq!(mem.chars, 8);

        let claude = group(&ctx, "claude");
        assert!(claude
            .items
            .iter()
            .any(|i| i.label == "Prompt do sistema do Claude Code"));
        assert!(claude
            .items
            .iter()
            .any(|i| i.label.starts_with("Ferramentas nativas (1)")));

        let session = group(&ctx, "session");
        assert!(session
            .items
            .iter()
            .any(|i| i.label == "Sua primeira mensagem" && i.chars == 11));
        assert!(session.items.iter().any(|i| i.label == "Data de hoje"));
        assert!(!session.items.iter().any(|i| i.label.contains("prompt")));
        assert!(ctx.warnings.is_empty());
    }

    #[test]
    fn parses_per_turn() {
        let ctx = parse_lines(Path::new("/l/abc.jsonl"), sample().into_iter()).unwrap();
        let hooks = ctx.per_turn.iter().find(|g| g.key == "hooks").unwrap();
        let ups = hooks
            .items
            .iter()
            .find(|i| i.label.starts_with("UserPromptSubmit"))
            .unwrap();
        assert_eq!(ups.chars, 9);
        assert_eq!(ups.count, Some(1));
        let post = hooks
            .items
            .iter()
            .find(|i| i.label.starts_with("PostToolUse"))
            .unwrap();
        assert_eq!(post.chars, 0);
        assert!(post.note.as_deref().unwrap().contains("não colocou nada"));
        let rem = ctx.per_turn.iter().find(|g| g.key == "reminders").unwrap();
        assert_eq!(rem.items[0].label, "Lembrete da lista de tarefas");
    }

    #[test]
    fn tracks_growth_message_by_message() {
        let a = |id: &str, input: u64, out: u64, content: Value| {
            json!({"type": "assistant", "message": {"id": id, "content": content,
                "usage": {"input_tokens": 1, "cache_read_input_tokens": input - 1, "output_tokens": out}}})
        };
        let recs = vec![
            att(
                json!({"type": "hook_success", "hookEvent": "SessionStart", "command": "x", "content": "inicio"}),
            ),
            json!({"type": "user", "message": {"content": "lê o arquivo"}, "timestamp": "t1"}),
            att(json!({"type": "skill_listing", "content": "- a: b\n"})),
            a(
                "m1",
                10000,
                50,
                json!([{"type": "tool_use", "id": "tu1", "name": "Read", "input": {"file_path": "/a"}}]),
            ),
            // same call, second block: usage must not count twice
            a("m1", 10000, 50, json!([{"type": "text", "text": "lendo"}])),
            json!({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": "tu1", "content": "x".repeat(8000)}]}}),
            att(
                json!({"type": "hook_success", "hookEvent": "PostToolUse", "command": "y", "content": "nota!"}),
            ),
            a(
                "m2",
                12100,
                100,
                json!([{"type": "text", "text": "pronto"}]),
            ),
            json!({"type": "user", "message": {"content": [{"type": "text", "text": "e agora?"}, {"type": "image", "source": {}}]}}),
            att(json!({"type": "task_reminder", "content": "tarefas"})),
            a("m3", 13000, 20, json!([{"type": "text", "text": "ok"}])),
            json!({"type": "user", "message": {"content": "<task-notification>\n<status>completed</status>"}}),
            json!({"type": "system", "subtype": "compact_boundary"}),
            a("m4", 3000, 10, json!([{"type": "text", "text": "resumo"}])),
            json!({"type": "user", "message": {"content": "interrompida"}}),
        ];
        let ctx = parse_lines(Path::new("/l/t.jsonl"), recs.into_iter()).unwrap();
        assert_eq!(ctx.turns.len(), 4);

        let t1 = &ctx.turns[0];
        assert_eq!(t1.prompt, "lê o arquivo");
        assert_eq!(t1.at.as_deref(), Some("t1"));
        assert_eq!(t1.api_calls, 2);
        assert_eq!(t1.context_after, Some(12200));
        assert_eq!(t1.growth, Some(2200)); // against its first call
        assert_eq!(t1.output_tokens, 150);
        let read = &t1.tools[0];
        assert_eq!(
            (read.label.as_str(), read.count, read.chars),
            ("Read", Some(1), 8000)
        );
        assert!(t1.parts.iter().any(|p| p.label == "Hooks" && p.chars == 5));
        let reply = t1
            .parts
            .iter()
            .find(|p| p.label == "Respostas do Claude")
            .unwrap();
        assert_eq!(reply.tokens, 150);
        let known: usize = t1
            .parts
            .iter()
            .filter(|p| !p.label.starts_with("Resto"))
            .map(|p| p.tokens)
            .sum();
        let rest = t1
            .parts
            .iter()
            .find(|p| p.label.starts_with("Resto"))
            .unwrap();
        assert_eq!(rest.tokens, 2200 - known);
        // the startup skill listing is not part of the first message
        assert!(!t1.parts.iter().any(|p| p.label == "Anexos do Claude Code"));

        let t2 = &ctx.turns[1];
        assert_eq!(t2.images, 1);
        assert_eq!(t2.growth, Some(13020 - 12200));
        let msg = t2.parts.iter().find(|p| p.label == "Sua mensagem").unwrap();
        assert_eq!(msg.chars, "e agora?".len() + IMAGE_CHARS);
        assert!(t2.parts.iter().any(|p| p.label == "Anexos do Claude Code"));

        let t3 = &ctx.turns[2];
        assert_eq!(t3.kind, "task");
        assert!(t3.compacted);
        assert!(t3.growth.unwrap() < 0);

        let t4 = &ctx.turns[3];
        assert_eq!(t4.api_calls, 0);
        assert_eq!(t4.context_after, None);
        assert_eq!(t4.growth, None);
    }

    #[test]
    fn old_log_warns() {
        let recs = vec![json!({"type": "user", "message": {"content": "oi"}})];
        let ctx = parse_lines(Path::new("/l/x.jsonl"), recs.into_iter()).unwrap();
        assert_eq!(ctx.warnings.len(), 2);
        assert!(ctx.startup.iter().all(|g| g.key == "session"));
    }

    #[test]
    fn hook_labels() {
        assert_eq!(
            hook_label("/home/u/.agents/hooks/onde-estamos.sh"),
            "onde-estamos.sh"
        );
        assert_eq!(
            hook_label("\"$HOME/.local/share/ai-memory/native-runner/ai-memory\" --data-dir x hook --event session-start --agent c"),
            "ai-memory (session-start)"
        );
        assert_eq!(
            hook_label("if [ -x '/a/b/repowise-augment' ]; then exec '/a/b/repowise-augment'; fi"),
            "repowise-augment"
        );
        assert_eq!(hook_label("echo oi"), "echo oi");
    }

    #[test]
    fn rejects_bad_session_ids() {
        let home = Path::new("/nao-existe");
        assert!(session_file(home, Path::new("/p"), "../etc/passwd").is_none());
        assert!(session_file(home, Path::new("/p"), "").is_none());
    }

    #[test]
    fn lists_sessions_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let cwd = Path::new("/proj/a");
        let dir = claude_projects_dir(home).join(claude_project_slug(cwd));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("old.jsonl"),
            "{\"type\":\"user\",\"message\":{\"content\":\"primeira\"}}\n",
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(
            dir.join("new.jsonl"),
            "{\"type\":\"ai-title\",\"aiTitle\":\"Nova\"}\n",
        )
        .unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        let list = list_sessions(home, cwd, 10);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].session_id, "new");
        assert_eq!(list[0].title.as_deref(), Some("Nova"));
        assert_eq!(list[1].title.as_deref(), Some("primeira"));
        assert!(session_file(home, cwd, "new").is_some());

        let other = Path::new("/proj/b");
        let dir_b = claude_projects_dir(home).join(claude_project_slug(other));
        std::fs::create_dir_all(&dir_b).unwrap();
        std::fs::write(
            dir_b.join("s.jsonl"),
            "{\"type\":\"user\",\"cwd\":\"/proj/b\"}\n",
        )
        .unwrap();
        // mtime resolution: make the rewrite clearly newer than /proj/b
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(
            dir.join("new.jsonl"),
            "{\"type\":\"user\",\"cwd\":\"/proj/a\"}\n",
        )
        .unwrap();
        let folders = list_folders(home, 10);
        assert_eq!(folders.len(), 2);
        assert_eq!(folders[0].path, "/proj/a");
        assert_eq!(folders[0].sessions, 2);
        assert!(!folders[0].exists);
    }
}
