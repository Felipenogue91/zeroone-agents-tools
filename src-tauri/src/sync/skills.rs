//! Skill materialisation per CLI. The canonical folder is
//! `~/.agents/skills/<name>`:
//! - Claude Code: a symlink in `~/.claude/skills/<name>` (copy fallback);
//! - Antigravity CLI: reads the folder through `~/.gemini/config/skills.json`
//!   entries (`include_only` narrows it to some skills);
//! - Codex: reads `~/.agents/skills` natively (nothing to toggle).

use super::engine::FileChange;
use super::model::{Cli, Scope};
use super::source::{contract_home, expand_home, load_json};
use crate::utils::agents_home::AgentsHome;
use serde_json::{json, Map, Value};
use std::path::Path;

fn is_link(p: &Path) -> bool {
    std::fs::symlink_metadata(p)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Claude (global): link (on) / unlink (off). Never touches the canonical folder.
pub fn set_claude_skill(
    home: &Path,
    agents: &AgentsHome,
    name: &str,
    on: bool,
) -> Result<(), String> {
    set_claude_skill_at(&home.join(".claude"), &agents.skills_dir, name, on, None)
}

/// Claude in one project: `<p>/.claude/skills/<name>` as a relative link to
/// `<p>/.agents/skills/<name>` (works in git and in worktrees).
pub fn set_project_claude_skill(project: &Path, name: &str, on: bool) -> Result<(), String> {
    set_claude_skill_at(
        &project.join(".claude"),
        &project.join(".agents").join("skills"),
        name,
        on,
        Some(Path::new("../../.agents/skills")),
    )
}

/// `claude_dir` = `~/.claude` or `<p>/.claude`. On: restore a parked folder,
/// or link from `shared_dir`. Off: unlink, or park a real folder (never delete).
pub fn set_claude_skill_at(
    claude_dir: &Path,
    shared_dir: &Path,
    name: &str,
    on: bool,
    relative_link: Option<&Path>,
) -> Result<(), String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("nome de skill inválido".into());
    }
    let src = shared_dir.join(name);
    let dest = claude_dir.join("skills").join(name);
    let parked = claude_dir
        .join(crate::services::skill_writer::DISABLED_SKILLS_DIR)
        .join(name);
    if on {
        if dest.exists() || is_link(&dest) {
            return Ok(());
        }
        std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
        if parked.join("SKILL.md").is_file() {
            return std::fs::rename(&parked, &dest).map_err(|e| e.to_string());
        }
        if !src.join("SKILL.md").is_file() {
            return Err(format!("{} não tem SKILL.md", src.display()));
        }
        let target = match relative_link {
            Some(rel) => rel.join(name),
            None => src.clone(),
        };
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&target, &dest);
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&src, &dest);
        if linked.is_err() {
            copy_dir(&src, &dest).map_err(|e| e.to_string())?;
        }
        Ok(())
    } else {
        if is_link(&dest) {
            #[cfg(windows)]
            if std::fs::remove_dir(&dest).is_ok() {
                return Ok(());
            }
            return std::fs::remove_file(&dest).map_err(|e| e.to_string());
        }
        if !dest.exists() {
            return Ok(());
        }
        // A real folder: park it (restorable), never delete.
        std::fs::create_dir_all(parked.parent().unwrap()).map_err(|e| e.to_string())?;
        if parked.exists() {
            return Err(format!(
                "já existe uma cópia estacionada em {}",
                parked.display()
            ));
        }
        std::fs::rename(&dest, &parked).map_err(|e| e.to_string())
    }
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let to = dst.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &to)?;
        } else {
            std::fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

fn shared_skill_names(agents: &AgentsHome) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(&agents.skills_dir) else {
        return Vec::new();
    };
    let mut v: Vec<String> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("SKILL.md").is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    v.sort();
    v
}

/// agy: planned edit of `skills.json` turning one skill on/off. Returns
/// `None` when nothing changes. Turning one skill off when the entry reads
/// the whole folder converts it to an explicit `include_only` list.
pub fn plan_agy_skill(
    home: &Path,
    agents: &AgentsHome,
    name: &str,
    on: bool,
) -> Option<FileChange> {
    let path = home.join(".gemini").join("config").join("skills.json");
    let before = std::fs::read_to_string(&path).ok();
    let mut doc = load_json(&path).unwrap_or_else(|| json!({"entries": []}));
    let shared = agents.skills_dir.clone();
    let entries = doc
        .as_object_mut()?
        .entry("entries")
        .or_insert_with(|| Value::Array(vec![]));
    let arr = entries.as_array_mut()?;
    let idx = arr.iter().position(|e| {
        e.get("path")
            .and_then(|p| p.as_str())
            .map(|p| {
                let p = expand_home(p, home);
                p == shared
                    || matches!((std::fs::canonicalize(&p), std::fs::canonicalize(&shared)), (Ok(a), Ok(b)) if a == b)
            })
            .unwrap_or(false)
    });
    let all = shared_skill_names(agents);
    match (idx, on) {
        (None, true) => {
            arr.push(json!({"path": contract_home(&shared, home), "include_only": [name]}));
        }
        (None, false) => return None,
        (Some(i), _) => {
            let entry = arr[i].as_object_mut()?;
            let mut list: Vec<String> = match entry.get("include_only").and_then(|l| l.as_array()) {
                Some(l) => l
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect(),
                None if on => return None, // whole folder already read
                None => all.clone(),
            };
            let has = list.iter().any(|n| n == name);
            if on == has {
                return None;
            }
            if on {
                list.push(name.to_string());
                list.sort();
            } else {
                list.retain(|n| n != name);
            }
            if on && all.iter().all(|n| list.contains(n)) {
                entry.remove("include_only");
            } else {
                entry.insert("include_only".into(), json!(list));
            }
        }
    }
    let after = serde_json::to_string_pretty(&doc).ok()? + "\n";
    if before.as_deref() == Some(after.as_str()) {
        return None;
    }
    Some(FileChange {
        cli: Cli::Agy,
        scope: Scope::Global,
        label: "agy skills".into(),
        path,
        before,
        after: Some(after),
        mode: None,
        servers: vec![name.to_string()],
        ensure_ignored: None,
        claude_patch: None,
    })
}

/// Planned edit making agy read the whole `~/.agents/skills` folder (used by
/// the health screen when the entry is missing).
pub fn plan_agy_reads_shared(home: &Path, agents: &AgentsHome) -> Option<FileChange> {
    if !agents.skills_dir.is_dir() {
        return None;
    }
    let path = home.join(".gemini").join("config").join("skills.json");
    let before = std::fs::read_to_string(&path).ok();
    let mut doc = load_json(&path).unwrap_or_else(|| json!({"entries": []}));
    let obj = doc.as_object_mut()?;
    let arr = obj
        .entry("entries")
        .or_insert_with(|| Value::Array(vec![]))
        .as_array_mut()?;
    let present = arr.iter().any(|e| {
        e.get("path")
            .and_then(|p| p.as_str())
            .map(|p| expand_home(p, home) == agents.skills_dir)
            .unwrap_or(false)
    });
    if present {
        return None;
    }
    let mut entry = Map::new();
    entry.insert(
        "path".into(),
        Value::String(contract_home(&agents.skills_dir, home)),
    );
    arr.insert(0, Value::Object(entry));
    Some(FileChange {
        cli: Cli::Agy,
        scope: Scope::Global,
        label: "agy skills".into(),
        path,
        before,
        after: Some(serde_json::to_string_pretty(&doc).ok()? + "\n"),
        mode: None,
        servers: vec![],
        ensure_ignored: None,
        claude_patch: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn fx() -> (TempDir, PathBuf, AgentsHome) {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().to_path_buf();
        let agents = AgentsHome::from_home(&home);
        for n in ["a", "b", "c"] {
            let d = agents.skills_dir.join(n);
            std::fs::create_dir_all(d.join("scripts")).unwrap();
            std::fs::write(d.join("SKILL.md"), format!("---\nname: {}\n---\n", n)).unwrap();
        }
        (tmp, home, agents)
    }

    #[cfg(unix)]
    #[test]
    fn claude_toggle_links_and_unlinks_only() {
        let (_t, home, agents) = fx();
        set_claude_skill(&home, &agents, "a", true).unwrap();
        let dest = home.join(".claude/skills/a");
        assert!(is_link(&dest));
        set_claude_skill(&home, &agents, "a", true).unwrap();
        set_claude_skill(&home, &agents, "a", false).unwrap();
        assert!(!dest.exists() && !is_link(&dest));
        assert!(agents.skills_dir.join("a/scripts").is_dir());
        assert!(set_claude_skill(&home, &agents, "../x", true).is_err());
    }

    #[test]
    fn project_claude_skill_relative_link_and_park_restore() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("proj");
        let shared = p.join(".agents/skills/s1");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(shared.join("SKILL.md"), "---\nname: s1\n---\n").unwrap();
        set_project_claude_skill(&p, "s1", true).unwrap();
        let link = p.join(".claude/skills/s1");
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            Path::new("../../.agents/skills/s1")
        );
        assert!(link.join("SKILL.md").is_file());
        set_project_claude_skill(&p, "s1", false).unwrap();
        assert!(!link.exists() && shared.exists());
        // claude-only real folder: off parks, on restores
        let only = p.join(".claude/skills/only");
        std::fs::create_dir_all(&only).unwrap();
        std::fs::write(only.join("SKILL.md"), "x").unwrap();
        set_project_claude_skill(&p, "only", false).unwrap();
        assert!(!only.exists());
        let res = crate::sync::source::project_skill_resources(&p);
        let o = res.iter().find(|r| r.name == "only").unwrap();
        assert_eq!(
            o.targets[&crate::sync::model::Cli::Claude],
            crate::sync::model::TargetState::Off
        );
        set_project_claude_skill(&p, "only", true).unwrap();
        assert!(only.join("SKILL.md").is_file());
    }

    #[test]
    fn claude_off_parks_real_folder() {
        let (_t, home, agents) = fx();
        let dest = home.join(".claude/skills/solo");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("SKILL.md"), "x").unwrap();
        set_claude_skill(&home, &agents, "solo", false).unwrap();
        assert!(!dest.exists());
        assert!(home.join(".claude/.skills-disabled/solo/SKILL.md").exists());
    }

    fn apply(ch: FileChange) {
        std::fs::create_dir_all(ch.path.parent().unwrap()).unwrap();
        std::fs::write(&ch.path, ch.after.unwrap()).unwrap();
    }

    #[test]
    fn agy_toggle_converts_to_include_only_and_back() {
        let (_t, home, agents) = fx();
        std::fs::create_dir_all(home.join(".gemini/config")).unwrap();
        std::fs::write(
            home.join(".gemini/config/skills.json"),
            r#"{"entries":[{"path":"~/.agents/skills"},{"path":"~/.codex/skills","include_only":["x"]}]}"#,
        )
        .unwrap();
        assert!(
            plan_agy_skill(&home, &agents, "a", true).is_none(),
            "already on"
        );
        apply(plan_agy_skill(&home, &agents, "b", false).unwrap());
        let doc = load_json(&home.join(".gemini/config/skills.json")).unwrap();
        assert_eq!(doc["entries"][0]["include_only"], json!(["a", "c"]));
        assert_eq!(
            doc["entries"][1]["include_only"],
            json!(["x"]),
            "other entries untouched"
        );
        apply(plan_agy_skill(&home, &agents, "b", true).unwrap());
        let doc = load_json(&home.join(".gemini/config/skills.json")).unwrap();
        assert!(
            doc["entries"][0].get("include_only").is_none(),
            "all on again = whole folder"
        );
    }

    #[test]
    fn agy_reads_shared_adds_entry_once() {
        let (_t, home, agents) = fx();
        let ch = plan_agy_reads_shared(&home, &agents).unwrap();
        apply(ch);
        assert!(plan_agy_reads_shared(&home, &agents).is_none());
        let doc = load_json(&home.join(".gemini/config/skills.json")).unwrap();
        assert_eq!(doc["entries"][0]["path"], "~/.agents/skills");
    }
}
