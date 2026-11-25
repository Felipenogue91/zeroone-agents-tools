//! Round-trip tests on sanitized fixtures modeled on real configs
//! (Claude Code 2.1, codex-cli 0.171, agy 1.5): nothing the app does not
//! own may be lost or reordered when the engine writes.

use super::engine::{apply, Engine};
use super::secrets::{parse_env, Secrets};
use super::source::SourceSnapshot;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const CODEX: &str = include_str!("fixtures/codex_config.toml");
const CLAUDE: &str = include_str!("fixtures/claude.json");
const AGY: &str = include_str!("fixtures/agy_mcp_config.json");
const AGY_SKILLS: &str = include_str!("fixtures/agy_skills.json");
const GLOBAL: &str = include_str!("fixtures/global.json");
const SYNC: &str = include_str!("fixtures/sync.json");
const PROJECT_MCP: &str = include_str!("fixtures/project_mcp.json");
const PROJECT_SETTINGS: &str = include_str!("fixtures/project_settings.json");
const SECRETS: &str = include_str!("fixtures/secrets.env");

struct Home {
    _tmp: TempDir,
    home: PathBuf,
    proj: PathBuf,
}

fn put(p: &Path, s: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, s).unwrap();
}

fn home() -> Home {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let proj = home.join("w").join("proj");
    put(&home.join(".codex/config.toml"), CODEX);
    // fixture paths use /home/user/w/proj; point the local scope at the temp project
    put(
        &home.join(".claude.json"),
        &CLAUDE.replace("/home/user/w/proj", &proj.to_string_lossy()),
    );
    put(&home.join(".gemini/config/mcp_config.json"), AGY);
    put(&home.join(".gemini/config/skills.json"), AGY_SKILLS);
    put(&home.join(".agents/mcp/global.json"), GLOBAL);
    put(&home.join(".agents/mcp/sync.json"), SYNC);
    put(&home.join(".agents/mcp/secrets.env"), SECRETS);
    put(&proj.join(".mcp.json"), PROJECT_MCP);
    put(&proj.join(".claude/settings.json"), PROJECT_SETTINGS);
    Home {
        _tmp: tmp,
        home,
        proj,
    }
}

fn secrets() -> Secrets {
    Secrets::from_map(parse_env(SECRETS))
}

fn sync_all(h: &Home) {
    let snap = SourceSnapshot::load(&h.home);
    let eng = Engine::with_secrets(&snap, secrets());
    let plan = eng.plan(None);
    let out = apply(&plan.changes, &h.home.join("bk")).unwrap();
    assert!(out.skipped.is_empty(), "{:?}", out.skipped);
}

fn toml_doc(text: &str) -> toml::Table {
    text.parse().unwrap()
}

#[test]
fn codex_keeps_every_foreign_table_key_and_comment() {
    let h = home();
    sync_all(&h);
    let after = std::fs::read_to_string(h.home.join(".codex/config.toml")).unwrap();
    let (b, a) = (toml_doc(CODEX), toml_doc(&after));
    for (k, v) in &b {
        if k != "mcp_servers" {
            assert_eq!(a.get(k), Some(v), "top-level `{}` changed", k);
        }
    }
    let (bm, am) = (
        b["mcp_servers"].as_table().unwrap(),
        a["mcp_servers"].as_table().unwrap(),
    );
    assert_eq!(
        am.get("node_repl"),
        bm.get("node_repl"),
        "codexGlobalKeep server untouched (incl. subtable)"
    );
    assert_eq!(
        am["ai-memory"]["default_tools_approval_mode"].as_str(),
        Some("approve")
    );
    assert_eq!(
        am["chrome-devtools"]["startup_timeout_sec"].as_integer(),
        Some(60)
    );
    assert_eq!(am["icecat"]["tool_timeout_sec"].as_integer(), Some(120));
    assert!(
        am.get("stale-one").is_none(),
        "removed from the source → removed from Codex"
    );
    assert!(am.get("Neon").is_none(), "pluginProvided");
    assert!(am.get("events").is_none(), "Codex does not speak SSE");
    assert!(after.contains("# Codex CLI config (sanitized fixture"));
    assert!(after.contains("# user-owned server (codexGlobalKeep)"));
    assert!(after.contains("[tools.update_plan]\nenabled = true"));
    assert_eq!(after.matches(super::codex::MARKER).count(), 1);
}

#[test]
fn claude_json_keeps_all_other_keys_in_order() {
    let h = home();
    sync_all(&h);
    let orig: Value =
        serde_json::from_str(&CLAUDE.replace("/home/user/w/proj", &h.proj.to_string_lossy()))
            .unwrap();
    let after: Value =
        serde_json::from_str(&std::fs::read_to_string(h.home.join(".claude.json")).unwrap())
            .unwrap();
    let (o, a) = (orig.as_object().unwrap(), after.as_object().unwrap());
    assert_eq!(
        o.keys().collect::<Vec<_>>(),
        a.keys().collect::<Vec<_>>(),
        "key order preserved"
    );
    for (k, v) in o {
        if k != "mcpServers" {
            assert_eq!(a.get(k), Some(v), "`{}` changed", k);
        }
    }
    let servers = a["mcpServers"].as_object().unwrap();
    assert!(
        servers.get("user-added").is_none(),
        "user scope = source (mcp-sync semantics)"
    );
    assert_eq!(
        servers["icecat"]["headers"]["Api-Token"],
        "fixture-icecat-token"
    );
}

#[test]
fn agy_keeps_unknown_top_level_keys_and_drops_legacy_self_entry() {
    let h = home();
    sync_all(&h);
    let after: Value = serde_json::from_str(
        &std::fs::read_to_string(h.home.join(".gemini/config/mcp_config.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        after["$schema"],
        "https://example.invalid/agy-mcp.schema.json"
    );
    assert_eq!(after["uiState"]["collapsed"][0], "ai-memory");
    assert!(after["mcpServers"]
        .get("claude-code-tool-manager")
        .is_none());
    assert_eq!(
        after["mcpServers"]["chrome-devtools"]["args"][0],
        "--browserUrl=http://127.0.0.1:9222"
    );
    assert!(
        after["mcpServers"]["Neon"].is_object(),
        "agy has no plugin for Neon"
    );
}

#[test]
fn project_files_reference_secrets_only_where_versioned() {
    let h = home();
    sync_all(&h);
    let codex = std::fs::read_to_string(h.proj.join(".codex/config.toml")).unwrap();
    for v in [
        "fixture-meshy-key",
        "fixture-brevo-token",
        "fixture-supabase-token",
    ] {
        assert!(!codex.contains(v), "secret value in versioned file");
    }
    assert!(codex.contains("env_vars = [\"MESHY_API_KEY\"]"));
    assert!(
        codex.contains("project_ref=fixture&read_only=true"),
        "project override applied"
    );
    assert!(codex.contains("[mcp_servers.brevo]") && codex.contains("enabled = false"));
    let agy: Value = serde_json::from_str(
        &std::fs::read_to_string(h.proj.join(".agents/mcp_config.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        agy["mcpServers"]["meshy"]["env"]["MESHY_API_KEY"],
        "fixture-meshy-key"
    );
    assert_eq!(
        agy["mcpServers"]["supabase"]["headers"]["Authorization"],
        "Bearer fixture-supabase-token"
    );
    assert!(
        agy["mcpServers"].get("brevo").is_none(),
        "disabled in the project"
    );
}

#[test]
fn second_run_is_a_no_op() {
    let h = home();
    sync_all(&h);
    let snap = SourceSnapshot::load(&h.home);
    let plan = Engine::with_secrets(&snap, secrets()).plan(None);
    assert!(
        plan.is_clean(),
        "{:?}",
        plan.changes
            .iter()
            .map(|c| (&c.label, &c.servers))
            .collect::<Vec<_>>()
    );
}

#[test]
fn hand_written_sources_survive_edits_byte_for_byte() {
    use super::jsonedit::{set_key, set_server};
    for text in [GLOBAL, PROJECT_MCP] {
        let doc: Value = serde_json::from_str(text).unwrap();
        for (name, v) in doc["mcpServers"].as_object().unwrap() {
            // change then restore: the file must come back identical
            let mut changed = v.clone();
            changed["description"] = Value::String("temp".into());
            let edited = set_server(text, name, Some(&changed)).unwrap();
            assert_ne!(edited, text);
            assert_eq!(
                set_server(&edited, name, Some(v)).unwrap(),
                text,
                "{} not restored",
                name
            );
        }
    }
    let projects: Value = serde_json::from_str::<Value>(SYNC).unwrap()["projects"].clone();
    let with_new = set_key(
        SYNC,
        &[],
        "projects",
        Some(&serde_json::json!(["~/w/proj", "~/w/new"])),
    )
    .unwrap();
    assert_eq!(
        set_key(&with_new, &[], "projects", Some(&projects)).unwrap(),
        SYNC
    );
}

#[test]
fn health_on_fixtures_lists_the_expected_divergences() {
    let h = home();
    let snap = SourceSnapshot::load(&h.home);
    let occ = super::occurrences::collect(&h.home, &[h.proj.clone()]);
    let eng = Engine::with_secrets(&snap, secrets());
    let plan = eng.plan(None);
    let health = super::health::compute_health(&snap, &snap.resources(), &occ, &plan, &eng.secrets);
    let names: Vec<(String, String)> = health
        .divergences
        .iter()
        .map(|d| (format!("{:?}", d.kind), d.name.clone()))
        .collect();
    assert!(names.contains(&("OnlyInCli".into(), "claude-code-tool-manager".into())));
    assert!(names.contains(&("OnlyInCli".into(), "stale-one".into())));
    assert!(names.contains(&("ClaudeLocalScope".into(), "legacy-local".into())));
    assert!(!names.iter().any(|(_, n)| n == "node_repl"));
}

/// Same fixtures through the reference script (when installed).
#[test]
fn reference_script_agrees_on_fixtures() {
    let Some(real_home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()) else {
        return;
    };
    let script = real_home.join(".agents/bin/mcp-sync");
    if !script.is_file() {
        return;
    }
    let h = home();
    sync_all(&h);
    // the local-scope copy is reported by both tools; drop it for a clean comparison
    let mut cj: Value =
        serde_json::from_str(&std::fs::read_to_string(h.home.join(".claude.json")).unwrap())
            .unwrap();
    cj["projects"][h.proj.to_string_lossy().as_ref()]["mcpServers"] = serde_json::json!({});
    std::fs::write(
        h.home.join(".claude.json"),
        serde_json::to_string_pretty(&cj).unwrap(),
    )
    .unwrap();
    let Ok(out) = std::process::Command::new("python3")
        .arg(&script)
        .args(["--check", "--json"])
        .env("HOME", &h.home)
        .output()
    else {
        return;
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    for item in &report {
        // the SSE warning is expected from both tools
        // in sync, and the only warning is the expected SSE one
        let only_sse_warning = item["summary"]
            .as_str()
            .map(|s| s.contains("iguais"))
            .unwrap_or(false)
            && item["detail"]
                .as_str()
                .map(|d| d == "events: Codex não fala SSE; ficou de fora do Codex")
                .unwrap_or(false);
        assert!(
            item["status"] == "ok" || only_sse_warning,
            "mcp-sync disagrees: {}",
            item
        );
        assert!(
            !item["summary"]
                .as_str()
                .unwrap_or("")
                .contains("fora de sincronia"),
            "{}",
            item
        );
    }
}
