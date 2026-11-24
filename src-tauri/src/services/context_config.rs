//! "Contexto" tab, the config side: hooks configured for Claude Code (to
//! compare with what the session log shows and to try one by hand) and the
//! size of what only loads on demand (skill and subagent bodies).

use super::context_inspector::{estimate_tokens, hook_label, ContextGroup, ContextItem};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

// ─── Plugins ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Plugin {
    /// Name before `@` (the prefix of its skills, `name:skill`).
    name: String,
    root: PathBuf,
}

/// Plugins Claude Code loads in `cwd`: installed for the user (or for this
/// project) and not turned off in `enabledPlugins`.
fn active_plugins(home: &Path, cwd: &Path) -> Vec<Plugin> {
    let claude = home.join(".claude");
    let Some(installed) = read_json(&claude.join("plugins/installed_plugins.json")) else {
        return Vec::new();
    };
    let mut enabled: BTreeMap<String, bool> = BTreeMap::new();
    for f in [
        claude.join("settings.json"),
        cwd.join(".claude/settings.json"),
        cwd.join(".claude/settings.local.json"),
    ] {
        if let Some(map) = read_json(&f).and_then(|v| v.get("enabledPlugins").cloned()) {
            for (k, v) in map.as_object().into_iter().flatten() {
                enabled.insert(k.clone(), v.as_bool().unwrap_or(false));
            }
        }
    }
    let mut out = Vec::new();
    for (key, entries) in installed
        .get("plugins")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if enabled.get(key) != Some(&true) {
            continue;
        }
        for e in entries.as_array().into_iter().flatten() {
            let scope = e.get("scope").and_then(Value::as_str).unwrap_or("");
            let for_here = match e.get("projectPath").and_then(Value::as_str) {
                Some(p) => cwd.starts_with(p),
                None => scope == "user",
            };
            if let (true, Some(root)) = (for_here, e.get("installPath").and_then(Value::as_str)) {
                out.push(Plugin {
                    name: key.split('@').next().unwrap_or(key).to_string(),
                    root: PathBuf::from(root),
                });
                break;
            }
        }
    }
    out
}

// ─── Configured hooks ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredHook {
    /// Stable id to run it later (`origin|event|group|index`).
    pub id: String,
    pub event: String,
    pub matcher: Option<String>,
    pub label: String,
    pub command: String,
    /// Settings file or plugin it comes from.
    pub origin: String,
    pub timeout_secs: Option<u64>,
    /// Folder of the plugin (for `${CLAUDE_PLUGIN_ROOT}`).
    #[serde(skip)]
    pub plugin_root: Option<PathBuf>,
}

/// Hook sources in the order Claude Code merges them.
fn hook_sources(home: &Path, cwd: &Path) -> Vec<(String, Value, Option<PathBuf>)> {
    let mut out = Vec::new();
    let files = [
        (
            home.join(".claude/settings.json"),
            "~/.claude/settings.json".to_string(),
        ),
        (
            cwd.join(".claude/settings.json"),
            ".claude/settings.json (projeto)".to_string(),
        ),
        (
            cwd.join(".claude/settings.local.json"),
            ".claude/settings.local.json (projeto)".to_string(),
        ),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for (path, label) in files {
        // In the home folder, project files are the global ones.
        if !seen.insert(path.clone()) {
            continue;
        }
        if let Some(hooks) = read_json(&path).and_then(|v| v.get("hooks").cloned()) {
            out.push((label, hooks, None));
        }
    }
    for p in active_plugins(home, cwd) {
        if let Some(hooks) =
            read_json(&p.root.join("hooks/hooks.json")).and_then(|v| v.get("hooks").cloned())
        {
            out.push((format!("plugin {}", p.name), hooks, Some(p.root.clone())));
        }
    }
    out
}

pub fn configured_hooks(home: &Path, cwd: &Path) -> Vec<ConfiguredHook> {
    let mut out = Vec::new();
    for (origin, hooks, root) in hook_sources(home, cwd) {
        for (event, groups) in hooks.as_object().into_iter().flatten() {
            for (gi, group) in groups.as_array().into_iter().flatten().enumerate() {
                let matcher = group
                    .get("matcher")
                    .and_then(Value::as_str)
                    .filter(|m| !m.is_empty())
                    .map(String::from);
                for (hi, h) in group
                    .get("hooks")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .enumerate()
                {
                    if h.get("type").and_then(Value::as_str) != Some("command") {
                        continue;
                    }
                    let Some(command) = h.get("command").and_then(Value::as_str) else {
                        continue;
                    };
                    out.push(ConfiguredHook {
                        id: format!("{}|{}|{}|{}", origin, event, gi, hi),
                        event: event.clone(),
                        matcher: matcher.clone(),
                        label: hook_label(command),
                        command: command.to_string(),
                        origin: origin.clone(),
                        timeout_secs: h.get("timeout").and_then(Value::as_u64),
                        plugin_root: root.clone(),
                    });
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookRun {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub elapsed_ms: u64,
    /// What would go into the context (plain stdout, or the JSON's
    /// `additionalContext`).
    pub injected_chars: usize,
    pub injected_tokens: usize,
    pub stdout_chars: usize,
    /// First part of stdout, for a look.
    pub preview: String,
    pub stderr: String,
}

fn kill_tree(pid: u32) {
    #[cfg(unix)]
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{}", pid)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    #[cfg(windows)]
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Events whose plain stdout Claude Code adds to the context.
fn stdout_is_context(event: &str) -> bool {
    matches!(event, "SessionStart" | "UserPromptSubmit")
}

fn injected_from_stdout(event: &str, stdout: &str) -> usize {
    if let Ok(v) = serde_json::from_str::<Value>(stdout.trim()) {
        return v
            .pointer("/hookSpecificOutput/additionalContext")
            .and_then(Value::as_str)
            .map(|s| s.chars().count())
            .unwrap_or(0);
    }
    if stdout_is_context(event) {
        stdout.chars().count()
    } else {
        0
    }
}

/// Run one configured hook the way Claude Code would (stdin JSON, project
/// folder, its timeout). Only on an explicit click: hooks may have effects.
pub fn run_hook(home: &Path, cwd: &Path, hook: &ConfiguredHook) -> Result<HookRun, String> {
    let timeout = Duration::from_secs(hook.timeout_secs.unwrap_or(15).clamp(1, 60));
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&hook.command);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(&hook.command);
        c
    };
    cmd.current_dir(cwd)
        .env("CLAUDE_PROJECT_DIR", cwd)
        .env("HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(root) = &hook.plugin_root {
        cmd.env("CLAUDE_PLUGIN_ROOT", root);
    }
    // Own process group, so a timeout kills the whole tree (a shell's
    // children would otherwise keep the pipes open).
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let input = json!({
        "session_id": "zeroone-agents-tools-preview",
        "transcript_path": "",
        "cwd": cwd,
        "hook_event_name": hook.event,
        "source": "startup",
        "prompt": "(prévia do ZeroOne Agents Tools)",
    });
    let start = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("não deu para rodar: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.to_string().as_bytes());
    }
    // Read pipes on threads so a chatty hook does not block on a full pipe.
    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out_t = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(p) = out_pipe.as_mut() {
            let _ = p.read_to_string(&mut s);
        }
        s
    });
    let err_t = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(p) = err_pipe.as_mut() {
            let _ = p.read_to_string(&mut s);
        }
        s
    });
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > timeout => {
                kill_tree(child.id());
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    };
    let stdout = out_t.join().unwrap_or_default();
    let stderr = err_t.join().unwrap_or_default();
    let injected = injected_from_stdout(&hook.event, &stdout);
    Ok(HookRun {
        exit_code: status.and_then(|s| s.code()),
        timed_out,
        elapsed_ms: start.elapsed().as_millis() as u64,
        injected_chars: injected,
        injected_tokens: estimate_tokens(injected),
        stdout_chars: stdout.chars().count(),
        preview: stdout.chars().take(3000).collect(),
        stderr: stderr.chars().take(1000).collect(),
    })
}

// ─── On demand: skill and subagent bodies ───────────────────────────────────

fn strip_frontmatter(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("---") else {
        return text;
    };
    match rest.find("\n---") {
        Some(i) => rest[i + 4..].trim_start_matches(['\r', '\n']),
        None => text,
    }
}

fn frontmatter_name(text: &str) -> Option<String> {
    let rest = text.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    rest[..end].lines().find_map(|l| {
        l.strip_prefix("name:")
            .map(|n| n.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
    })
}

/// `SKILL.md` files under a folder (a plugin keeps skills at its root or in
/// `skills/`), by folder name and by frontmatter name.
fn skill_files(root: &Path) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    for e in walkdir::WalkDir::new(root)
        .follow_links(true)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
    {
        if e.file_name() != "SKILL.md" {
            continue;
        }
        let path = e.path().to_path_buf();
        if let Some(dir) = path.parent().and_then(|p| p.file_name()) {
            out.entry(dir.to_string_lossy().to_string())
                .or_insert_with(|| path.clone());
        }
        if let Some(n) = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| frontmatter_name(&t))
        {
            out.entry(n).or_insert(path);
        }
    }
    out
}

fn body_item(label: &str, path: &Path, home: &Path) -> Option<ContextItem> {
    let text = std::fs::read_to_string(path).ok()?;
    let chars = strip_frontmatter(&text).chars().count();
    let shown = match path.strip_prefix(home) {
        Ok(rel) => format!("~/{}", rel.display()),
        Err(_) => path.display().to_string(),
    };
    Some(ContextItem {
        label: label.to_string(),
        source: Some(shown),
        chars,
        tokens: estimate_tokens(chars),
        note: None,
        count: None,
    })
}

fn group(key: &str, title: &str, hint: &str, mut items: Vec<ContextItem>) -> ContextGroup {
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

/// Size of what loads only when used, for the skills and subagents the
/// session listed. Built-in ones have no file and are counted apart.
pub fn on_demand(
    home: &Path,
    cwd: &Path,
    skills: &[String],
    agents: &[String],
) -> Vec<ContextGroup> {
    let mut local = skill_files(&home.join(".claude/skills"));
    if cwd != home {
        for (k, v) in skill_files(&cwd.join(".claude/skills")) {
            local.insert(k, v); // project wins over personal
        }
    }
    let plugins: BTreeMap<String, BTreeMap<String, PathBuf>> = active_plugins(home, cwd)
        .into_iter()
        .map(|p| (p.name.clone(), skill_files(&p.root)))
        .collect();

    let mut items = Vec::new();
    let mut builtin = Vec::new();
    for name in skills {
        let path = match name.split_once(':') {
            Some((plugin, skill)) => plugins.get(plugin).and_then(|m| m.get(skill)),
            None => local.get(name),
        };
        match path.and_then(|p| body_item(name, p, home)) {
            Some(i) => items.push(i),
            None => builtin.push(name.clone()),
        }
    }
    let mut out = Vec::new();
    if !builtin.is_empty() {
        builtin.sort();
        items.push(ContextItem {
            label: format!("{} skill(s) sem arquivo no disco", builtin.len()),
            source: None,
            chars: 0,
            tokens: 0,
            note: Some(format!(
                "Embutidas no Claude Code ou na conta: {}",
                builtin.join(", ")
            )),
            count: None,
        });
    }
    if !items.is_empty() {
        out.push(group(
            "skillBodies",
            "Corpo das skills",
            "Entra quando a skill é usada (você chama ou o Claude decide). Fica até o fim da sessão.",
            items,
        ));
    }

    let mut agent_items = Vec::new();
    for name in agents {
        let candidates = [
            cwd.join(".claude/agents").join(format!("{}.md", name)),
            home.join(".claude/agents").join(format!("{}.md", name)),
        ];
        if let Some(i) = candidates
            .iter()
            .find(|p| p.is_file())
            .and_then(|p| body_item(name, p, home))
        {
            agent_items.push(i);
        }
    }
    if !agent_items.is_empty() {
        out.push(group(
            "agentBodies",
            "Prompt dos subagentes",
            "Vai para a janela do subagente quando ele é chamado, não para a sua.",
            agent_items,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let proj = tmp.path().join("proj");
        let plugin_root = tmp.path().join("cache/hf/1.0");
        write(
            &home.join(".claude/settings.json"),
            &json!({
                "hooks": {
                    "SessionStart": [{"matcher": "startup", "hooks": [{"type": "command", "command": "echo inicio", "timeout": 5}]}],
                    "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "printf '{\"hookSpecificOutput\":{\"additionalContext\":\"abcd\"}}'"}]}],
                    "Stop": [{"hooks": [{"type": "prompt", "prompt": "x"}]}]
                },
                "enabledPlugins": {"hf@mkt": true, "off@mkt": false}
            })
            .to_string(),
        );
        write(
            &proj.join(".claude/settings.local.json"),
            &json!({"hooks": {"PostToolUse": [{"matcher": "Edit", "hooks": [{"type": "command", "command": "/x/fmt.sh"}]}]}}).to_string(),
        );
        write(
            &home.join(".claude/plugins/installed_plugins.json"),
            &json!({"plugins": {
                "hf@mkt": [{"scope": "user", "installPath": plugin_root}],
                "off@mkt": [{"scope": "user", "installPath": "/nao"}]
            }})
            .to_string(),
        );
        write(
            &plugin_root.join("hooks/hooks.json"),
            &json!({"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/s.sh"}]}]}}).to_string(),
        );
        write(
            &plugin_root.join("hf-gen/SKILL.md"),
            "---\nname: hf-gen\n---\n1234567890",
        );
        write(
            &home.join(".claude/skills/minha/SKILL.md"),
            "---\nname: minha\ndescription: d\n---\nabcd",
        );
        write(
            &proj.join(".claude/skills/minha/SKILL.md"),
            "---\nname: minha\n---\nprojeto-vence",
        );
        write(
            &home.join(".claude/agents/revisor.md"),
            "---\nname: revisor\n---\nrevise tudo",
        );
        (tmp, home, proj)
    }

    #[test]
    fn lists_configured_hooks_from_settings_and_plugins() {
        let (_t, home, proj) = fixture();
        let hooks = configured_hooks(&home, &proj);
        let events: Vec<_> = hooks
            .iter()
            .map(|h| (h.event.as_str(), h.origin.as_str()))
            .collect();
        assert!(events.contains(&("SessionStart", "~/.claude/settings.json")));
        assert!(events.contains(&("UserPromptSubmit", "~/.claude/settings.json")));
        assert!(events.contains(&("PostToolUse", ".claude/settings.local.json (projeto)")));
        assert!(events.contains(&("SessionStart", "plugin hf")));
        // prompt-type hooks are not commands
        assert!(!hooks.iter().any(|h| h.event == "Stop"));
        let start = hooks.iter().find(|h| h.command == "echo inicio").unwrap();
        assert_eq!(start.matcher.as_deref(), Some("startup"));
        assert_eq!(start.timeout_secs, Some(5));
        // ids are unique
        let mut ids: Vec<_> = hooks.iter().map(|h| &h.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), hooks.len());
    }

    #[cfg(unix)]
    #[test]
    fn runs_a_hook_and_measures_what_it_injects() {
        let (_t, home, proj) = fixture();
        let hooks = configured_hooks(&home, &proj);
        let start = hooks.iter().find(|h| h.command == "echo inicio").unwrap();
        let r = run_hook(&home, &proj, start).unwrap();
        assert_eq!(r.exit_code, Some(0));
        assert_eq!(r.injected_chars, "inicio\n".len());
        let ups = hooks
            .iter()
            .find(|h| h.event == "UserPromptSubmit")
            .unwrap();
        let r = run_hook(&home, &proj, ups).unwrap();
        assert_eq!(r.injected_chars, 4);
        let post = hooks.iter().find(|h| h.event == "PostToolUse").unwrap();
        let r = run_hook(&home, &proj, post).unwrap();
        assert_ne!(r.exit_code, Some(0)); // /x/fmt.sh does not exist
        assert_eq!(r.injected_chars, 0);
    }

    #[cfg(unix)]
    #[test]
    fn hook_timeout_is_enforced() {
        let (_t, home, proj) = fixture();
        let h = ConfiguredHook {
            id: "t".into(),
            event: "SessionStart".into(),
            matcher: None,
            label: "sleep".into(),
            command: "sleep 5".into(),
            origin: "teste".into(),
            timeout_secs: Some(1),
            plugin_root: None,
        };
        let r = run_hook(&home, &proj, &h).unwrap();
        assert!(r.timed_out);
        assert!(r.elapsed_ms < 4000);
    }

    #[test]
    fn measures_skill_and_agent_bodies() {
        let (_t, home, proj) = fixture();
        let groups = on_demand(
            &home,
            &proj,
            &["minha".into(), "hf:hf-gen".into(), "simplify".into()],
            &["revisor".into(), "Explore".into()],
        );
        let skills = groups.iter().find(|g| g.key == "skillBodies").unwrap();
        let minha = skills.items.iter().find(|i| i.label == "minha").unwrap();
        assert_eq!(minha.chars, "projeto-vence".len());
        let hf = skills
            .items
            .iter()
            .find(|i| i.label == "hf:hf-gen")
            .unwrap();
        assert_eq!(hf.chars, 10);
        let missing = skills
            .items
            .iter()
            .find(|i| i.label.contains("sem arquivo"))
            .unwrap();
        assert!(missing.note.as_deref().unwrap().contains("simplify"));
        let agents = groups.iter().find(|g| g.key == "agentBodies").unwrap();
        assert_eq!(agents.items.len(), 1);
        assert_eq!(agents.items[0].chars, "revise tudo".len());
    }

    #[test]
    fn frontmatter_helpers() {
        assert_eq!(strip_frontmatter("---\na: b\n---\ncorpo"), "corpo");
        assert_eq!(strip_frontmatter("sem frontmatter"), "sem frontmatter");
        assert_eq!(
            frontmatter_name("---\nname: \"x\"\n---\n").as_deref(),
            Some("x")
        );
    }
}
