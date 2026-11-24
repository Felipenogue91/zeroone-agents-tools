//! Sync engine: `read → plan → diff → apply` for Claude Code, Codex and the
//! Antigravity CLI, driven by the `~/.agents` source of truth.
//!
//! Port of `~/.agents/bin/mcp-sync` (the behavioural spec). Differences on
//! purpose:
//! - nothing is written while planning; `apply` re-checks every file and
//!   skips the ones that changed since the preview;
//! - every applied file is backed up first and the whole set can be undone;
//! - Claude user scope is patched in `~/.claude.json` directly (atomic write,
//!   re-read right before writing) instead of shelling out to `claude mcp`.

use super::codex::{codex_current, codex_rewrite, to_codex};
use super::model::{mcp_transport, Cli, Scope};
use super::occurrences::{CliFiles, AGY_PROJECT_REL, CODEX_PROJECT_REL};
use super::secrets::{mask_literal_secrets, Secrets};
use super::source::{load_json, ProjectSource, SourceSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReportItem {
    pub check: String,
    /// "ok" | "warn"
    pub status: String,
    pub summary: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub cli: Cli,
    pub scope: Scope,
    /// Short label (`claude`, `codex`, `agy@main`, `agy@feature-x`...).
    pub label: String,
    pub path: PathBuf,
    /// Current content (None = file does not exist).
    pub before: Option<String>,
    /// New content (None = delete the file).
    pub after: Option<String>,
    /// Permission bits forced on write (agy files hold resolved secrets).
    pub mode: Option<u32>,
    /// Server names that differ.
    pub servers: Vec<String>,
    /// `(repo folder, relative path)` to add to `.git/info/exclude`.
    pub ensure_ignored: Option<(PathBuf, String)>,
    /// `~/.claude.json` is rewritten by Claude Code all the time: apply
    /// re-reads it and only replaces `mcpServers` with this value.
    pub claude_user_servers: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub changes: Vec<FileChange>,
    pub report: Vec<ReportItem>,
}

impl Plan {
    pub fn is_clean(&self) -> bool {
        self.changes.is_empty()
    }
}

// ------------------------------------------------------------------ helpers

fn read_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap() + "\n"
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git_ok(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Main folder + every worktree (git), or just the folder.
pub fn worktrees(main: &Path) -> Vec<PathBuf> {
    match git(main, &["worktree", "list", "--porcelain"]) {
        Some(out) => {
            let v: Vec<PathBuf> = out
                .lines()
                .filter_map(|l| l.strip_prefix("worktree "))
                .map(PathBuf::from)
                .filter(|p| p.is_dir())
                .collect();
            if v.is_empty() {
                vec![main.to_path_buf()]
            } else {
                v
            }
        }
        None => vec![main.to_path_buf()],
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Add `rel` to the repo's `info/exclude` unless git already ignores it.
pub fn ensure_ignored(repo: &Path, rel: &str) {
    if git_ok(repo, &["check-ignore", "-q", rel]) {
        return;
    }
    let Some(common) = git(
        repo,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ) else {
        return;
    };
    let exc = Path::new(&common).join("info").join("exclude");
    if let Some(parent) = exc.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exc)
    {
        let _ = write!(
            f,
            "\n# mcp-sync: gerado localmente, com segredo\n/{}\n",
            rel
        );
    }
}

/// Same normalisation mcp-sync applies before comparing Claude entries.
pub fn claude_norm(cfg: &Value) -> Value {
    let mut c = cfg.clone();
    if mcp_transport(&c) == "stdio" {
        if let Value::Object(m) = &mut c {
            m.entry("type").or_insert(Value::String("stdio".into()));
            m.entry("args").or_insert(Value::Array(vec![]));
            m.entry("env").or_insert(Value::Object(Map::new()));
        }
    }
    c
}

/// Claude config → agy config (value resolved). `None` when a variable is
/// missing (the server is left out of agy, like mcp-sync).
pub fn to_agy(
    name: &str,
    cfg: &Value,
    secrets: &Secrets,
    warns: &mut Vec<String>,
) -> Option<Value> {
    let mut missing = BTreeSet::new();
    let mut out = Map::new();
    if mcp_transport(cfg) == "stdio" {
        out.insert(
            "command".into(),
            cfg.get("command")
                .cloned()
                .unwrap_or(Value::String(String::new())),
        );
        if let Some(a) = cfg
            .get("args")
            .filter(|a| a.as_array().map(|x| !x.is_empty()).unwrap_or(false))
        {
            out.insert("args".into(), secrets.resolve(a, &mut missing));
        }
        if let Some(e) = cfg
            .get("env")
            .filter(|e| e.as_object().map(|x| !x.is_empty()).unwrap_or(false))
        {
            out.insert("env".into(), secrets.resolve(e, &mut missing));
        }
    } else {
        let url = cfg
            .get("url")
            .or_else(|| cfg.get("serverUrl"))
            .cloned()
            .unwrap_or(Value::Null);
        out.insert("serverUrl".into(), secrets.resolve(&url, &mut missing));
        if let Some(h) = cfg
            .get("headers")
            .filter(|h| h.as_object().map(|x| !x.is_empty()).unwrap_or(false))
        {
            out.insert("headers".into(), secrets.resolve(h, &mut missing));
        }
    }
    if !missing.is_empty() {
        warns.push(format!(
            "{}: falta {} no secrets.env (ficou fora do agy)",
            name,
            missing.into_iter().collect::<Vec<_>>().join(", ")
        ));
        return None;
    }
    Some(Value::Object(out))
}

fn agy_servers(
    src: &std::collections::BTreeMap<String, Value>,
    order: &[String],
    disabled: &BTreeSet<String>,
    secrets: &Secrets,
    warns: &mut Vec<String>,
) -> Map<String, Value> {
    let mut out = Map::new();
    for name in order {
        let Some(cfg) = src.get(name) else { continue };
        if disabled.contains(name) {
            continue;
        }
        if let Some(a) = to_agy(name, cfg, secrets, warns) {
            out.insert(name.clone(), a);
        }
    }
    out
}

fn changed_names(cur: &Map<String, Value>, want: &Map<String, Value>, prefix: &str) -> Vec<String> {
    let names: BTreeSet<&String> = cur.keys().chain(want.keys()).collect();
    names
        .into_iter()
        .filter(|n| cur.get(*n) != want.get(*n))
        .map(|n| format!("{}{}", prefix, n))
        .collect()
}

// ------------------------------------------------------------------ engine

pub struct Engine<'a> {
    pub snap: &'a SourceSnapshot,
    pub files: CliFiles,
    pub secrets: Secrets,
}

impl<'a> Engine<'a> {
    pub fn new(snap: &'a SourceSnapshot) -> Self {
        Engine {
            files: CliFiles::for_home(&snap.home),
            secrets: Secrets::load(&snap.agents.secrets_file),
            snap,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_secrets(snap: &'a SourceSnapshot, secrets: Secrets) -> Self {
        Engine {
            files: CliFiles::for_home(&snap.home),
            secrets,
            snap,
        }
    }

    /// Server names of the global source in file order.
    fn global_order(&self) -> Vec<String> {
        load_json(&self.snap.agents.global_mcp_file)
            .and_then(|v| {
                v.get("mcpServers")
                    .and_then(|m| m.as_object())
                    .map(|m| m.keys().cloned().collect())
            })
            .unwrap_or_default()
    }

    fn project_order(path: &Path) -> Vec<String> {
        load_json(&path.join(".mcp.json"))
            .and_then(|v| {
                v.get("mcpServers")
                    .and_then(|m| m.as_object())
                    .map(|m| m.keys().cloned().collect())
            })
            .unwrap_or_default()
    }

    /// Whole plan: global + every managed project (or only one project).
    pub fn plan(&self, only_project: Option<&Path>) -> Plan {
        let mut plan = Plan::default();
        if only_project.is_none() {
            self.plan_global(&mut plan);
        }
        for p in self.snap.sync.project_paths(&self.snap.home) {
            if let Some(only) = only_project {
                if !same_dir(&p, only) {
                    continue;
                }
            }
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !p.is_dir() {
                plan.report.push(ReportItem {
                    check: format!("projeto.{}", name),
                    status: "warn".into(),
                    summary: format!("{} não existe mais", p.display()),
                    detail: None,
                });
                continue;
            }
            self.plan_project(&p, &mut plan, false, None);
        }
        plan
    }

    pub fn plan_global(&self, plan: &mut Plan) {
        let src = &self.snap.global;
        if src.is_empty() {
            plan.report.push(ReportItem {
                check: "global".into(),
                status: "warn".into(),
                summary: format!(
                    "fonte global vazia ou ausente ({})",
                    self.snap.agents.global_mcp_file.display()
                ),
                detail: None,
            });
            return;
        }
        let order = self.global_order();
        let sync = &self.snap.sync;
        let mut warns: Vec<String> = Vec::new();
        let mut changed: Vec<String> = Vec::new();

        // Claude: user scope, resolved value (private file)
        let src_c = sync.with_overrides("global", Cli::Claude, src);
        let mut want_c = Map::new();
        for n in &order {
            let Some(c) = src_c.get(n) else { continue };
            if sync.is_plugin_provided(Cli::Claude, n) {
                continue;
            }
            let mut miss = BTreeSet::new();
            want_c.insert(n.clone(), claude_norm(&self.secrets.resolve(c, &mut miss)));
            for v in miss {
                warns.push(format!("claude/{}: falta {}", n, v));
            }
        }
        let claude_text = read_text(&self.files.claude_json);
        let claude_doc = match &claude_text {
            Some(t) => serde_json::from_str::<Value>(t).ok(),
            None => Some(Value::Object(Map::new())),
        };
        match claude_doc {
            None => warns.push(format!(
                "{} não é JSON válido; Claude ficou de fora",
                self.files.claude_json.display()
            )),
            Some(mut doc) => {
                let cur_c: Map<String, Value> = doc
                    .get("mcpServers")
                    .and_then(|m| m.as_object())
                    .map(|m| m.iter().map(|(k, v)| (k.clone(), claude_norm(v))).collect())
                    .unwrap_or_default();
                let diff = changed_names(&cur_c, &want_c, "claude:");
                if !diff.is_empty() {
                    if let Value::Object(m) = &mut doc {
                        m.insert("mcpServers".into(), Value::Object(want_c.clone()));
                    }
                    plan.changes.push(FileChange {
                        cli: Cli::Claude,
                        scope: Scope::Global,
                        label: "claude".into(),
                        path: self.files.claude_json.clone(),
                        before: claude_text.clone(),
                        after: Some(pretty(&doc)),
                        mode: None,
                        servers: diff
                            .iter()
                            .map(|s| s["claude:".len()..].to_string())
                            .collect(),
                        ensure_ignored: None,
                        claude_user_servers: Some(want_c.clone()),
                    });
                    changed.extend(diff);
                }
            }
        }

        // Codex: ~/.codex/config.toml, resolved value; keep what is not ours
        let codex_text = read_text(&self.files.codex_global);
        match codex_current(codex_text.as_deref()) {
            Err(e) => warns.push(format!(
                "~/.codex/config.toml inválido ({}); Codex ficou de fora",
                e
            )),
            Ok(cur_x) => {
                let keep = &sync.codex_global_keep;
                let src_x = sync.with_overrides("global", Cli::Codex, src);
                let mut want_x: Vec<(String, Map<String, Value>)> = Vec::new();
                for n in &order {
                    let Some(c) = src_x.get(n) else { continue };
                    if sync.is_plugin_provided(Cli::Codex, n) {
                        continue;
                    }
                    if let Some(t) =
                        to_codex(n, c, false, cur_x.get(n), false, &self.secrets, &mut warns)
                    {
                        want_x.push((n.clone(), t));
                    }
                }
                let want_map: Map<String, Value> = want_x
                    .iter()
                    .map(|(n, t)| (n.clone(), Value::Object(t.clone())))
                    .collect();
                let managed_cur: Map<String, Value> = cur_x
                    .iter()
                    .filter(|(n, _)| !keep.contains(*n))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                if managed_cur != want_map {
                    let diff = changed_names(&managed_cur, &want_map, "codex:");
                    let mut drop: BTreeSet<String> = cur_x
                        .keys()
                        .filter(|n| !keep.contains(*n))
                        .cloned()
                        .collect();
                    drop.extend(want_map.keys().cloned());
                    let after =
                        codex_rewrite(codex_text.as_deref().unwrap_or(""), Some(&drop), &want_x);
                    plan.changes.push(FileChange {
                        cli: Cli::Codex,
                        scope: Scope::Global,
                        label: "codex".into(),
                        path: self.files.codex_global.clone(),
                        before: codex_text.clone(),
                        after: Some(after),
                        mode: None,
                        servers: diff
                            .iter()
                            .map(|s| s["codex:".len()..].to_string())
                            .collect(),
                        ensure_ignored: None,
                        claude_user_servers: None,
                    });
                    changed.extend(diff);
                }
            }
        }

        // agy: ~/.gemini/config/mcp_config.json, resolved; off = left out
        let src_a = sync.with_overrides("global", Cli::Agy, src);
        let order_a: Vec<String> = order
            .iter()
            .filter(|n| !sync.is_plugin_provided(Cli::Agy, n))
            .cloned()
            .collect();
        let want_a = agy_servers(
            &src_a,
            &order_a,
            &BTreeSet::new(),
            &self.secrets,
            &mut warns,
        );
        let agy_text = read_text(&self.files.agy_global);
        let mut agy_doc = agy_text
            .as_deref()
            .and_then(|t| serde_json::from_str::<Value>(t).ok())
            .unwrap_or_else(|| Value::Object(Map::new()));
        let cur_a_val = agy_doc.get("mcpServers").cloned();
        if cur_a_val.as_ref() != Some(&Value::Object(want_a.clone())) {
            let cur_a = cur_a_val
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            let diff = changed_names(&cur_a, &want_a, "agy:");
            if let Value::Object(m) = &mut agy_doc {
                m.insert("mcpServers".into(), Value::Object(want_a.clone()));
            }
            plan.changes.push(FileChange {
                cli: Cli::Agy,
                scope: Scope::Global,
                label: "agy".into(),
                path: self.files.agy_global.clone(),
                before: agy_text,
                after: Some(pretty(&agy_doc)),
                mode: Some(0o600),
                servers: diff.iter().map(|s| s["agy:".len()..].to_string()).collect(),
                ensure_ignored: None,
                claude_user_servers: None,
            });
            changed.extend(diff);
        }

        let status = if !warns.is_empty() || !changed.is_empty() {
            "warn"
        } else {
            "ok"
        };
        let summary = format!(
            "global: {} MCPs; {}",
            src.len(),
            if changed.is_empty() {
                "os três iguais".to_string()
            } else {
                format!("fora de sincronia: {}", changed.join(", "))
            }
        );
        plan.report.push(ReportItem {
            check: "global".into(),
            status: status.into(),
            summary,
            detail: (!warns.is_empty()).then(|| warns.join("; ")),
        });
    }

    /// One project (main folder + worktrees). `only_local`: just the agy
    /// file of `only_path` (session hook).
    pub fn plan_project(
        &self,
        main: &Path,
        plan: &mut Plan,
        only_local: bool,
        only_path: Option<&Path>,
    ) {
        let name = main
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let sync = &self.snap.sync;
        let mut warns: Vec<String> = Vec::new();
        let mut changed: Vec<String> = Vec::new();
        let mut notes: Vec<String> = Vec::new();
        let paths = match only_path {
            Some(p) => vec![p.to_path_buf()],
            None => worktrees(main),
        };
        let global_names: BTreeSet<String> = self.snap.global.keys().cloned().collect();

        for p in paths {
            let is_main = same_dir(&p, main);
            let ps = ProjectSource::load(&p);
            let order = Self::project_order(&p);
            let unknown: Vec<&String> = ps
                .disabled
                .iter()
                .filter(|d| !ps.servers.contains_key(*d))
                .collect();
            if !unknown.is_empty() && is_main {
                warns.push(format!(
                    "desligado mas não existe no .mcp.json: {}",
                    unknown
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            let label = if is_main {
                "main".to_string()
            } else {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            };
            let scope = Scope::Project {
                path: main.to_string_lossy().to_string(),
            };

            // agy: local .agents/mcp_config.json in every folder
            let src_a = sync.with_overrides(&name, Cli::Agy, &ps.servers);
            let want = agy_servers(&src_a, &order, &ps.disabled, &self.secrets, &mut warns);
            let f = p.join(AGY_PROJECT_REL);
            let text = read_text(&f);
            let doc = text
                .as_deref()
                .and_then(|t| serde_json::from_str::<Value>(t).ok());
            let cur = doc
                .as_ref()
                .and_then(|d| d.get("mcpServers"))
                .and_then(|m| m.as_object())
                .filter(|m| !m.is_empty())
                .cloned();
            let want_opt = (!want.is_empty()).then(|| want.clone());
            if cur != want_opt {
                let mut obj = Map::new();
                obj.insert("mcpServers".into(), Value::Object(want.clone()));
                let after = want_opt.as_ref().map(|_| pretty(&Value::Object(obj)));
                // delete only a file we could parse (mcp-sync: `elif doc is not None`)
                if after.is_some() || doc.is_some() {
                    plan.changes.push(FileChange {
                        cli: Cli::Agy,
                        scope: scope.clone(),
                        label: format!("agy@{}", label),
                        path: f.clone(),
                        before: text,
                        after: after.clone(),
                        mode: Some(0o600),
                        servers: changed_names(&cur.unwrap_or_default(), &want, ""),
                        ensure_ignored: after.map(|_| (p.clone(), AGY_PROJECT_REL.to_string())),
                        claude_user_servers: None,
                    });
                }
                changed.push(format!("agy@{}", label));
            }

            if only_local || !is_main {
                if !is_main && !only_local {
                    let cx = codex_current(read_text(&p.join(CODEX_PROJECT_REL)).as_deref())
                        .unwrap_or_default();
                    let want_names: BTreeSet<&String> = ps
                        .servers
                        .iter()
                        .filter(|(_, c)| mcp_transport(c) != "sse")
                        .map(|(n, _)| n)
                        .collect();
                    let cur_names: BTreeSet<&String> = cx.keys().collect();
                    if cur_names != want_names {
                        notes.push(label.clone());
                    }
                }
                continue;
            }

            // Codex: project .codex/config.toml (goes to git: references only)
            let cpath = p.join(CODEX_PROJECT_REL);
            let ctext = read_text(&cpath);
            match codex_current(ctext.as_deref()) {
                Err(e) => warns.push(format!(
                    ".codex/config.toml inválido ({}); Codex ficou de fora",
                    e
                )),
                Ok(cur) => {
                    let src_x = sync.with_overrides(&name, Cli::Codex, &ps.servers);
                    let mut want_x: Vec<(String, Map<String, Value>)> = Vec::new();
                    for n in &order {
                        let Some(c) = src_x.get(n) else { continue };
                        if let Some(t) = to_codex(
                            n,
                            c,
                            ps.disabled.contains(n),
                            cur.get(n),
                            true,
                            &self.secrets,
                            &mut warns,
                        ) {
                            want_x.push((n.clone(), t));
                        }
                    }
                    let want_map: Map<String, Value> = want_x
                        .iter()
                        .map(|(n, t)| (n.clone(), Value::Object(t.clone())))
                        .collect();
                    if cur != want_map {
                        let diff = changed_names(&cur, &want_map, "codex:");
                        let ignored_mcp = git_ok(&p, &["check-ignore", "-q", ".mcp.json"]);
                        plan.changes.push(FileChange {
                            cli: Cli::Codex,
                            scope: scope.clone(),
                            label: "codex".into(),
                            path: cpath.clone(),
                            before: ctext.clone(),
                            after: Some(codex_rewrite(
                                ctext.as_deref().unwrap_or(""),
                                None,
                                &want_x,
                            )),
                            mode: None,
                            servers: diff
                                .iter()
                                .map(|s| s["codex:".len()..].to_string())
                                .collect(),
                            ensure_ignored: (ignored_mcp && ctext.is_none())
                                .then(|| (p.clone(), CODEX_PROJECT_REL.to_string())),
                            claude_user_servers: None,
                        });
                        changed.extend(diff);
                    }
                }
            }

            // Claude: local scope only Claude sees
            let local: Vec<String> = load_json(&self.files.claude_json)
                .and_then(|c| {
                    c.get("projects")
                        .and_then(|ps| ps.get(p.to_string_lossy().as_ref()))
                        .and_then(|pr| pr.get("mcpServers"))
                        .and_then(|m| m.as_object())
                        .map(|m| m.keys().cloned().collect())
                })
                .unwrap_or_default();
            let mut dup: Vec<&String> = local
                .iter()
                .filter(|n| ps.servers.contains_key(*n) || global_names.contains(*n))
                .collect();
            dup.sort();
            let mut only: Vec<&String> = local.iter().filter(|n| !dup.contains(n)).collect();
            only.sort();
            if !only.is_empty() {
                warns.push(format!(
                    "só o Claude vê (escopo local): {}; mova pro .mcp.json",
                    only.iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !dup.is_empty() {
                warns.push(format!(
                    "cópia antiga no escopo local do Claude (vence a do projeto): {}",
                    dup.iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        if only_path.is_some() {
            return;
        }
        let mut detail = Vec::new();
        if !warns.is_empty() {
            detail.push(warns.join("; "));
        }
        if !notes.is_empty() {
            detail.push(format!(
                "worktrees com Codex atrás da main (resolve no merge da main): {}",
                notes.join(", ")
            ));
        }
        let status = if !warns.is_empty() || !changed.is_empty() {
            "warn"
        } else {
            "ok"
        };
        let mut summary = format!(
            "{}: {}",
            name,
            if changed.is_empty() {
                "Claude, Codex e agy iguais".to_string()
            } else {
                format!("fora de sincronia: {}", changed.join(", "))
            }
        );
        if let Some(first) = warns.first() {
            summary.push_str(&format!(" · atenção: {}", first));
            if warns.len() > 1 {
                summary.push_str(&format!(" (+{})", warns.len() - 1));
            }
        }
        plan.report.push(ReportItem {
            check: format!("projeto.{}", name),
            status: status.into(),
            summary,
            detail: (!detail.is_empty()).then(|| detail.join(" | ")),
        });
    }

    /// Session hook (`mcp-sync --here`): the agy file of the current folder
    /// when it belongs to a managed project (main folder or worktree).
    pub fn plan_here(&self, cwd: &Path) -> Plan {
        let mut plan = Plan::default();
        let Some(top) = git(cwd, &["rev-parse", "--show-toplevel"]) else {
            return plan;
        };
        let top = PathBuf::from(top);
        let main_root = git(
            &top,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
        .and_then(|c| Path::new(&c).parent().map(Path::to_path_buf))
        .unwrap_or_else(|| top.clone());
        let managed = self
            .snap
            .sync
            .project_paths(&self.snap.home)
            .into_iter()
            .any(|p| same_dir(&p, &main_root));
        if managed {
            self.plan_project(&main_root, &mut plan, true, Some(&top));
        }
        plan
    }

    /// Unified diff of one change, with every secret value hidden.
    pub fn masked_diff(&self, ch: &FileChange) -> String {
        let before = ch.before.as_deref().unwrap_or("");
        let after = ch.after.as_deref().unwrap_or("");
        let path = ch.path.to_string_lossy();
        let diff = similar::TextDiff::from_lines(before, after)
            .unified_diff()
            .context_radius(2)
            .header(&format!("a{}", path), &format!("b{}", path))
            .to_string();
        mask_literal_secrets(&self.secrets.mask(&diff))
    }
}

// ------------------------------------------------------------------ apply / undo

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub path: String,
    pub existed: bool,
    /// File name inside the backup folder (when `existed`).
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutcome {
    pub backup_id: Option<String>,
    pub applied: Vec<String>,
    /// (path, reason)
    pub skipped: Vec<(String, String)>,
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> std::io::Result<()> {
    Ok(())
}

/// Keep the newest `keep` backup sets.
fn prune_backups(root: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(root) else {
        return;
    };
    let mut dirs: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    if dirs.len() > keep {
        for d in &dirs[..dirs.len() - keep] {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

pub const BACKUPS_KEPT: usize = 30;

/// Apply planned changes. Each file is checked against the preview first;
/// a file edited in between is skipped (except `~/.claude.json`, which is
/// re-patched on its fresh content). Every touched file is backed up in
/// `backup_root/<id>/` (0700) so the set can be undone.
pub fn apply(changes: &[FileChange], backup_root: &Path) -> anyhow::Result<ApplyOutcome> {
    let mut outcome = ApplyOutcome::default();
    if changes.is_empty() {
        return Ok(outcome);
    }
    let id = format!(
        "{}-{:03}",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        std::process::id() % 1000
    );
    let dir = backup_root.join(&id);
    std::fs::create_dir_all(&dir)?;
    let _ = set_mode(backup_root, 0o700);
    let _ = set_mode(&dir, 0o700);
    let mut manifest: Vec<BackupEntry> = Vec::new();

    for (i, ch) in changes.iter().enumerate() {
        let current = read_text(&ch.path);
        let after: Option<String> = if let Some(servers) = &ch.claude_user_servers {
            let mut doc = match current.as_deref() {
                Some(t) => match serde_json::from_str::<Value>(t) {
                    Ok(d) => d,
                    Err(_) => {
                        outcome.skipped.push((
                            ch.path.to_string_lossy().to_string(),
                            "JSON inválido".into(),
                        ));
                        continue;
                    }
                },
                None => Value::Object(Map::new()),
            };
            if let Value::Object(m) = &mut doc {
                m.insert("mcpServers".into(), Value::Object(servers.clone()));
            }
            Some(pretty(&doc))
        } else {
            if current != ch.before {
                outcome.skipped.push((
                    ch.path.to_string_lossy().to_string(),
                    "o arquivo mudou desde a prévia; gere a prévia de novo".into(),
                ));
                continue;
            }
            ch.after.clone()
        };

        let entry = match &current {
            Some(text) => {
                let file = format!("{:04}", i);
                let bp = dir.join(&file);
                std::fs::write(&bp, text)?;
                let _ = set_mode(&bp, 0o600);
                BackupEntry {
                    path: ch.path.to_string_lossy().to_string(),
                    existed: true,
                    file: Some(file),
                }
            }
            None => BackupEntry {
                path: ch.path.to_string_lossy().to_string(),
                existed: false,
                file: None,
            },
        };
        manifest.push(entry);

        match after {
            Some(text) => {
                if let Some(parent) = ch.path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if let Some((repo, rel)) = &ch.ensure_ignored {
                    ensure_ignored(repo, rel);
                }
                crate::utils::atomic::write_atomic(&ch.path, &text)?;
                if let Some(mode) = ch.mode {
                    set_mode(&ch.path, mode)?;
                }
            }
            None => {
                if ch.path.exists() {
                    std::fs::remove_file(&ch.path)?;
                }
            }
        }
        outcome.applied.push(ch.path.to_string_lossy().to_string());
    }

    if manifest.is_empty() {
        let _ = std::fs::remove_dir_all(&dir);
    } else {
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest)?,
        )?;
        outcome.backup_id = Some(id);
    }
    prune_backups(backup_root, BACKUPS_KEPT);
    Ok(outcome)
}

/// Restore a backup set (files that did not exist are removed again).
pub fn undo(backup_root: &Path, id: &str) -> anyhow::Result<Vec<String>> {
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        anyhow::bail!("id de backup inválido");
    }
    let dir = backup_root.join(id);
    let manifest: Vec<BackupEntry> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json"))?)?;
    let mut restored = Vec::new();
    for e in manifest.iter().rev() {
        let path = PathBuf::from(&e.path);
        if e.existed {
            let text = std::fs::read_to_string(dir.join(e.file.as_deref().unwrap_or("")))?;
            crate::utils::atomic::write_atomic(&path, text)?;
        } else if path.exists() {
            std::fs::remove_file(&path)?;
        }
        restored.push(e.path.clone());
    }
    Ok(restored)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub id: String,
    pub files: Vec<String>,
}

pub fn list_backups(backup_root: &Path) -> Vec<BackupInfo> {
    let Ok(rd) = std::fs::read_dir(backup_root) else {
        return Vec::new();
    };
    let mut out: Vec<BackupInfo> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let m: Vec<BackupEntry> = serde_json::from_str(
                &std::fs::read_to_string(e.path().join("manifest.json")).ok()?,
            )
            .ok()?;
            Some(BackupInfo {
                id: e.file_name().to_string_lossy().to_string(),
                files: m.into_iter().map(|x| x.path).collect(),
            })
        })
        .collect();
    out.sort_by(|a, b| b.id.cmp(&a.id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tempfile::TempDir;

    struct Fx {
        _tmp: TempDir,
        home: PathBuf,
        proj: PathBuf,
    }

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    fn fixture() -> Fx {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let proj = home.join("w").join("proj");
        write(
            &home.join(".agents/mcp/global.json"),
            &json!({"mcpServers": {
                "ai-memory": {"type": "http", "url": "http://127.0.0.1:9/mcp"},
                "ctx": {"type": "stdio", "command": "npx", "args": ["-y", "ctx"]},
                "icecat": {"type": "http", "url": "https://ice/mcp", "headers": {"Authorization": "Bearer ${ICE_TOKEN}"}},
                "Neon": {"type": "http", "url": "https://mcp.neon.tech/mcp"}
            }})
            .to_string(),
        );
        write(
            &home.join(".agents/mcp/sync.json"),
            &json!({
                "_doc": "fixture",
                "projects": ["~/w/proj"],
                "pluginProvided": {"claude": ["Neon"], "codex": ["Neon"]},
                "codexGlobalKeep": ["node_repl"],
                "overrides": {"global": {"agy": {"ctx": {"args": ["-y", "ctx", "--agy"]}}}}
            })
            .to_string(),
        );
        write(
            &home.join(".agents/mcp/secrets.env"),
            "ICE_TOKEN=ice-secret-123\nSB_REF=ref-secret-456\n",
        );
        write(
            &home.join(".claude.json"),
            &json!({"numStartups": 7, "projects": {}, "mcpServers": {"stale": {"type": "stdio", "command": "x", "args": [], "env": {}}}}).to_string(),
        );
        write(
            &home.join(".codex/config.toml"),
            "model = \"gpt-5\"\n\n[mcp_servers.node_repl]\ncommand = \"node\"\nstartup_timeout_sec = 30\n\n[mcp_servers.ctx]\ncommand = \"old\"\nstartup_timeout_sec = 45\n",
        );
        write(
            &home.join(".gemini/config/mcp_config.json"),
            &json!({"mcpServers": {}}).to_string(),
        );
        write(
            &proj.join(".mcp.json"),
            &json!({"mcpServers": {
                "supabase": {"type": "http", "url": "https://mcp.supabase.com/mcp", "headers": {"Authorization": "Bearer ${SB_REF}"}},
                "local": {"command": "node", "args": ["srv.js"], "env": {"SB_REF": "${SB_REF}"}},
                "off": {"command": "x"}
            }})
            .to_string(),
        );
        write(
            &proj.join(".claude/settings.json"),
            &json!({"disabledMcpjsonServers": ["off"]}).to_string(),
        );
        Fx {
            _tmp: tmp,
            home,
            proj,
        }
    }

    fn engine_secrets(home: &Path) -> Secrets {
        Secrets::from_map(super::super::secrets::parse_env(
            &std::fs::read_to_string(home.join(".agents/mcp/secrets.env")).unwrap(),
        ))
    }

    fn plan_for(home: &Path) -> Plan {
        let snap = SourceSnapshot::load(home);
        Engine::with_secrets(&snap, engine_secrets(home)).plan(None)
    }

    #[test]
    fn global_and_project_plan_apply_is_idempotent() {
        let fx = fixture();
        let plan = plan_for(&fx.home);
        let labels: Vec<_> = plan.changes.iter().map(|c| c.label.clone()).collect();
        assert!(labels.contains(&"claude".to_string()));
        assert!(labels.contains(&"codex".to_string()));
        assert!(labels.contains(&"agy".to_string()));
        assert!(labels.contains(&"agy@main".to_string()));
        assert_eq!(plan.report.len(), 2);

        let backups = fx.home.join("backups");
        let out = apply(&plan.changes, &backups).unwrap();
        assert!(out.skipped.is_empty(), "{:?}", out.skipped);
        assert!(out.backup_id.is_some());

        // Claude: user scope = global minus pluginProvided, other keys kept
        let cj: Value =
            serde_json::from_str(&std::fs::read_to_string(fx.home.join(".claude.json")).unwrap())
                .unwrap();
        assert_eq!(cj["numStartups"], 7);
        let servers = cj["mcpServers"].as_object().unwrap();
        assert!(servers.get("stale").is_none());
        assert!(servers.get("Neon").is_none());
        assert_eq!(
            servers["icecat"]["headers"]["Authorization"],
            "Bearer ice-secret-123"
        );
        assert_eq!(servers["ctx"]["type"], "stdio");

        // Codex global: keep node_repl, keep extras, resolved values
        let cx = std::fs::read_to_string(fx.home.join(".codex/config.toml")).unwrap();
        assert!(cx.starts_with("model = \"gpt-5\""));
        assert!(
            cx.contains("[mcp_servers.node_repl]\ncommand = \"node\"\nstartup_timeout_sec = 30")
        );
        assert!(cx.contains("startup_timeout_sec = 45"), "extras survive");
        assert!(cx.contains(super::super::codex::MARKER));
        assert!(!cx.contains("[mcp_servers.Neon]"));

        // agy global: override applied, 0600
        let agy: Value = serde_json::from_str(
            &std::fs::read_to_string(fx.home.join(".gemini/config/mcp_config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(agy["mcpServers"]["ctx"]["args"][2], "--agy");
        assert_eq!(
            agy["mcpServers"]["ai-memory"]["serverUrl"],
            "http://127.0.0.1:9/mcp"
        );
        assert!(
            agy["mcpServers"].get("Neon").is_some(),
            "agy has no plugin for Neon"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let m = std::fs::metadata(fx.home.join(".gemini/config/mcp_config.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(m & 0o777, 0o600);
        }

        // Project Codex: references only (goes to git), disabled = enabled false
        let pcx = std::fs::read_to_string(fx.proj.join(".codex/config.toml")).unwrap();
        assert!(!pcx.contains("ref-secret-456"));
        assert!(pcx.contains("bearer_token_env_var = \"SB_REF\""));
        assert!(pcx.contains("env_vars = [\"SB_REF\"]"));
        assert!(pcx.contains("[mcp_servers.off]\ncommand = \"x\"\nenabled = false"));

        // Project agy: resolved, disabled left out
        let pagy: Value = serde_json::from_str(
            &std::fs::read_to_string(fx.proj.join(".agents/mcp_config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            pagy["mcpServers"]["supabase"]["headers"]["Authorization"],
            "Bearer ref-secret-456"
        );
        assert!(pagy["mcpServers"].get("off").is_none());

        // Second plan: nothing to do
        let again = plan_for(&fx.home);
        assert!(
            again.is_clean(),
            "{:?}",
            again
                .changes
                .iter()
                .map(|c| (&c.label, &c.servers))
                .collect::<Vec<_>>()
        );
        assert!(
            again.report.iter().all(|r| r.status == "ok"),
            "{:?}",
            again.report
        );
    }

    #[test]
    fn undo_restores_previous_files() {
        let fx = fixture();
        let before_codex = std::fs::read_to_string(fx.home.join(".codex/config.toml")).unwrap();
        let plan = plan_for(&fx.home);
        let backups = fx.home.join("backups");
        let out = apply(&plan.changes, &backups).unwrap();
        let id = out.backup_id.unwrap();
        assert_eq!(list_backups(&backups)[0].id, id);
        undo(&backups, &id).unwrap();
        assert_eq!(
            std::fs::read_to_string(fx.home.join(".codex/config.toml")).unwrap(),
            before_codex
        );
        assert!(
            !fx.proj.join(".agents/mcp_config.json").exists(),
            "created files are removed"
        );
        assert!(undo(&backups, "../x").is_err());
    }

    #[test]
    fn stale_preview_is_skipped_but_claude_json_is_repatched() {
        let fx = fixture();
        let plan = plan_for(&fx.home);
        std::fs::write(fx.home.join(".codex/config.toml"), "model = \"changed\"\n").unwrap();
        // Claude Code rewrote ~/.claude.json meanwhile
        std::fs::write(
            fx.home.join(".claude.json"),
            json!({"numStartups": 8}).to_string(),
        )
        .unwrap();
        let out = apply(&plan.changes, &fx.home.join("b")).unwrap();
        assert_eq!(out.skipped.len(), 1);
        assert!(out.skipped[0].0.ends_with("config.toml"));
        let cj: Value =
            serde_json::from_str(&std::fs::read_to_string(fx.home.join(".claude.json")).unwrap())
                .unwrap();
        assert_eq!(cj["numStartups"], 8);
        assert!(cj["mcpServers"]["ctx"].is_object());
    }

    #[test]
    fn diff_preview_never_shows_secret_values() {
        let fx = fixture();
        let snap = SourceSnapshot::load(&fx.home);
        let eng = Engine::with_secrets(&snap, engine_secrets(&fx.home));
        let plan = eng.plan(None);
        for ch in &plan.changes {
            let d = eng.masked_diff(ch);
            assert!(!d.contains("ice-secret-123"), "{}", d);
            assert!(!d.contains("ref-secret-456"), "{}", d);
        }
        let claude = plan.changes.iter().find(|c| c.label == "claude").unwrap();
        assert!(eng.masked_diff(claude).contains("icecat"));
    }

    #[test]
    fn missing_secret_leaves_server_out_of_agy_with_warning() {
        let fx = fixture();
        let snap = SourceSnapshot::load(&fx.home);
        let eng = Engine::with_secrets(&snap, Secrets::from_map(HashMap::new()));
        let plan = eng.plan(None);
        let agy = plan.changes.iter().find(|c| c.label == "agy").unwrap();
        let after: Value = serde_json::from_str(agy.after.as_deref().unwrap()).unwrap();
        assert!(after["mcpServers"].get("icecat").is_none());
        let global = plan.report.iter().find(|r| r.check == "global").unwrap();
        assert!(global.detail.as_deref().unwrap().contains("ICE_TOKEN"));
    }

    #[test]
    fn worktrees_get_their_own_agy_file() {
        let fx = fixture();
        let ok = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(&fx.proj)
                .args(args)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };
        if !ok(&["init", "-q"]) {
            return; // git not available
        }
        ok(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "i",
        ]);
        let wt = fx.home.join("w").join("proj-feature");
        if !ok(&["worktree", "add", "-q", wt.to_str().unwrap()]) {
            return;
        }
        std::fs::copy(fx.proj.join(".mcp.json"), wt.join(".mcp.json")).unwrap();
        let plan = plan_for(&fx.home);
        assert!(plan.changes.iter().any(|c| c.label == "agy@proj-feature"));
        apply(&plan.changes, &fx.home.join("b")).unwrap();
        assert!(wt.join(".agents/mcp_config.json").exists());
        // generated secret file is excluded from git
        let exclude = std::fs::read_to_string(fx.proj.join(".git/info/exclude")).unwrap();
        assert!(exclude.contains("/.agents/mcp_config.json"));

        // session hook equivalent only touches the current folder
        std::fs::remove_file(wt.join(".agents/mcp_config.json")).unwrap();
        let snap = SourceSnapshot::load(&fx.home);
        let here = Engine::with_secrets(&snap, engine_secrets(&fx.home)).plan_here(&wt);
        assert_eq!(here.changes.len(), 1);
        assert_eq!(here.changes[0].label, "agy@proj-feature");
    }

    /// Parity with the reference implementation: after the Rust engine
    /// applies, `mcp-sync --check` (run with HOME pointing at the fixture)
    /// must report everything in sync. Skipped when the script or python3
    /// is not installed.
    #[test]
    fn parity_with_mcp_sync_reference_script() {
        let Some(real_home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
        else {
            return;
        };
        let script = real_home.join(".agents/bin/mcp-sync");
        if !script.is_file() {
            return;
        }
        let fx = fixture();
        let plan = plan_for(&fx.home);
        apply(&plan.changes, &fx.home.join("b")).unwrap();
        let out = Command::new("python3")
            .arg(&script)
            .args(["--check", "--json"])
            .env("HOME", &fx.home)
            .env_remove("ICE_TOKEN")
            .env_remove("SB_REF")
            .output();
        let Ok(out) = out else { return };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
        for item in &report {
            assert_eq!(item["status"], "ok", "mcp-sync disagrees: {}", item);
        }
        assert_eq!(report.len(), 2);
        eprintln!("parity: mcp-sync --check agrees on {} checks", report.len());
    }
}
