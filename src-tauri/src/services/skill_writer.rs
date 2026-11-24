use crate::db::models::Skill;
use crate::utils::opencode_paths::get_opencode_paths;
use anyhow::Result;
use directories::BaseDirs;
use std::path::{Path, PathBuf};

/// Generate markdown content for an agent skill (.claude/skills/name/SKILL.md)
pub(crate) fn generate_skill_markdown(skill: &Skill) -> String {
    let mut frontmatter = String::from("---\n");

    frontmatter.push_str(&format!("name: {}\n", skill.name));

    if let Some(ref desc) = skill.description {
        if !desc.is_empty() {
            frontmatter.push_str(&format!("description: {}\n", desc));
        }
    }

    if let Some(ref tools) = skill.allowed_tools {
        if !tools.is_empty() {
            frontmatter.push_str(&format!("allowed-tools: {}\n", tools.join(", ")));
        }
    }

    if let Some(ref model) = skill.model {
        if !model.is_empty() {
            frontmatter.push_str(&format!("model: {}\n", model));
        }
    }

    if skill.disable_model_invocation {
        frontmatter.push_str("disable-model-invocation: true\n");
    }

    if let Some(ref context) = skill.context {
        if !context.is_empty() {
            frontmatter.push_str(&format!("context: {}\n", context));
        }
    }
    if let Some(ref agent) = skill.agent {
        if !agent.is_empty() {
            frontmatter.push_str(&format!("agent: {}\n", agent));
        }
    }
    if let Some(ref shell) = skill.shell {
        if !shell.is_empty() {
            frontmatter.push_str(&format!("shell: {}\n", shell));
        }
    }
    if let Some(once) = skill.once {
        if once {
            frontmatter.push_str("once: true\n");
        }
    }
    if let Some(ref paths) = skill.paths {
        if !paths.is_empty() {
            frontmatter.push_str(&format!("paths: {}\n", paths.join(", ")));
        }
    }
    if let Some(ref hooks) = skill.hooks {
        if !hooks.is_empty() {
            frontmatter.push_str(&format!("hooks: {}\n", hooks));
        }
    }
    if let Some(ref effort) = skill.effort {
        if !effort.is_empty() {
            frontmatter.push_str(&format!("effort: {}\n", effort));
        }
    }

    if let Some(ref tags) = skill.tags {
        if !tags.is_empty() {
            frontmatter.push_str(&format!("tags: {}\n", serde_json::to_string(tags).unwrap()));
        }
    }

    frontmatter.push_str("---\n\n");
    format!("{}{}", frontmatter, skill.content)
}

/// Folder (sibling of `.claude/skills`) where toggled-off real skill folders
/// are parked. Claude Code only reads `.claude/skills/*/SKILL.md`, so parked
/// skills are inert, and toggling back on restores the folder intact
/// (scripts/, references/, unknown frontmatter...).
pub(crate) const DISABLED_SKILLS_DIR: &str = ".skills-disabled";

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Remove a symlink (file or directory link) without touching its target.
fn remove_link(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        // Directory symlinks/junctions on Windows are removed with remove_dir.
        if std::fs::remove_dir(path).is_ok() {
            return Ok(());
        }
    }
    std::fs::remove_file(path)
}

fn link_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dst)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(src, dst)
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Find the canonical folder of a skill (the real directory that holds its
/// SKILL.md), excluding `dest` itself. Looks at the shared skills dir
/// (`~/.agents/skills/<name>`) first, then at the skill's recorded source.
pub(crate) fn canonical_skill_source(
    skill: &Skill,
    dest: &Path,
    shared_skills_dir: Option<&Path>,
) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(shared) = shared_skills_dir {
        candidates.push(shared.join(&skill.name));
    }
    if let Some(sp) = skill.source_path.as_deref().filter(|s| !s.is_empty()) {
        let sp = PathBuf::from(sp);
        // source_path may point at the SKILL.md or at the folder
        let dir = if sp.file_name().map(|f| f == "SKILL.md").unwrap_or(false) {
            sp.parent().map(Path::to_path_buf).unwrap_or(sp)
        } else {
            sp
        };
        candidates.push(dir);
    }
    candidates.into_iter().find_map(|c| {
        if !c.join("SKILL.md").is_file() {
            return None;
        }
        // Resolve links: a link is never a source, what it points to is.
        let canon = std::fs::canonicalize(&c).ok()?;
        if same_path(&canon, dest) {
            return None;
        }
        Some(canon)
    })
}

/// How a skill should be materialised in a Claude skills folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillMaterialize {
    /// Symlink to the canonical folder (global scope: keeps one source).
    Link,
    /// Full recursive copy (project scope: links to personal paths must not
    /// be committed to a shared repo).
    Copy,
}

/// Enable a skill in `{base_path}/.claude/skills/{name}`.
///
/// Rules (never destructive):
/// - if the destination is already a symlink, nothing is written: writing
///   through it would rewrite the canonical source;
/// - if a previously toggled-off folder is parked, it is restored intact;
/// - if a canonical source exists elsewhere, it is linked (global) or copied
///   (project) — the whole folder, not just a regenerated SKILL.md;
/// - otherwise (app-created skill with no folder anywhere) SKILL.md is
///   generated from the library entry.
pub fn write_skill_file_with(
    base_path: &Path,
    skill: &Skill,
    shared_skills_dir: Option<&Path>,
    mode: SkillMaterialize,
) -> Result<()> {
    let skills_root = base_path.join(".claude").join("skills");
    let skill_dir = skills_root.join(&skill.name);

    if is_symlink(&skill_dir) {
        if skill_dir.exists() {
            // Already enabled through a link: never write through it.
            return Ok(());
        }
        // Dangling link: re-point it only if we know the canonical source.
        match canonical_skill_source(skill, &skill_dir, shared_skills_dir) {
            Some(src) => {
                remove_link(&skill_dir)?;
                link_dir(&src, &skill_dir)?;
                return Ok(());
            }
            None => {
                return Err(anyhow::anyhow!(
                "{} is a broken link and the skill source was not found; refusing to replace it",
                skill_dir.display()
            ))
            }
        }
    }

    // Restore a parked (toggled-off) folder.
    let parked = base_path
        .join(".claude")
        .join(DISABLED_SKILLS_DIR)
        .join(&skill.name);
    if !skill_dir.exists() && (parked.exists() || is_symlink(&parked)) {
        std::fs::create_dir_all(&skills_root)?;
        std::fs::rename(&parked, &skill_dir)?;
        return Ok(());
    }

    if !skill_dir.exists() {
        if let Some(src) = canonical_skill_source(skill, &skill_dir, shared_skills_dir) {
            std::fs::create_dir_all(&skills_root)?;
            match mode {
                SkillMaterialize::Link => {
                    if link_dir(&src, &skill_dir).is_err() {
                        // e.g. Windows without symlink privilege
                        copy_dir_recursive(&src, &skill_dir)?;
                    }
                }
                SkillMaterialize::Copy => copy_dir_recursive(&src, &skill_dir)?,
            }
            return Ok(());
        }
    }

    // Real folder owned by this location (or brand new app-created skill).
    std::fs::create_dir_all(&skill_dir)?;
    let file_path = skill_dir.join("SKILL.md");
    crate::utils::backup::backup_file(&file_path)?;
    let content = generate_skill_markdown(skill);
    crate::utils::atomic::write_atomic(file_path, content)?;

    Ok(())
}

/// Write a skill to the appropriate location
/// Skills go to {base_path}/.claude/skills/{name}/SKILL.md
#[cfg_attr(not(test), allow(dead_code))]
pub fn write_skill_file(base_path: &Path, skill: &Skill) -> Result<()> {
    write_skill_file_with(base_path, skill, None, SkillMaterialize::Copy)
}

/// Disable a skill in `{base_path}/.claude/skills/{name}`.
///
/// - symlink: only the link is removed, the target is never touched;
/// - real folder that *is* the shared canonical source: refused;
/// - other real folder: parked in `.claude/.skills-disabled/` (restorable),
///   never deleted.
pub fn delete_skill_file_with(
    base_path: &Path,
    skill: &Skill,
    shared_skills_dir: Option<&Path>,
) -> Result<()> {
    let skill_dir = base_path.join(".claude").join("skills").join(&skill.name);

    if is_symlink(&skill_dir) {
        remove_link(&skill_dir)?;
        return Ok(());
    }
    if !skill_dir.exists() {
        return Ok(());
    }
    if let Some(shared) = shared_skills_dir {
        if same_path(&skill_dir, &shared.join(&skill.name)) {
            return Err(anyhow::anyhow!(
                "{} is the canonical source of the skill; refusing to remove it",
                skill_dir.display()
            ));
        }
    }

    let parked_root = base_path.join(".claude").join(DISABLED_SKILLS_DIR);
    std::fs::create_dir_all(&parked_root)?;
    let mut parked = parked_root.join(&skill.name);
    if parked.exists() || is_symlink(&parked) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let older = parked_root.join(format!("{}.{}", skill.name, stamp));
        std::fs::rename(&parked, &older)?;
        parked = parked_root.join(&skill.name);
    }
    std::fs::rename(&skill_dir, &parked)?;
    Ok(())
}

/// Delete a skill file from the appropriate location
#[cfg_attr(not(test), allow(dead_code))]
pub fn delete_skill_file(base_path: &Path, skill: &Skill) -> Result<()> {
    delete_skill_file_with(base_path, skill, None)
}

fn shared_skills_dir() -> Option<PathBuf> {
    crate::utils::agents_home::agents_home()
        .map(|h| h.skills_dir)
        .filter(|d| d.is_dir())
}

/// Write a skill to the global Claude config (~/.claude/)
pub fn write_global_skill(skill: &Skill) -> Result<()> {
    let base_dirs =
        BaseDirs::new().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let home = base_dirs.home_dir();
    write_skill_file_with(
        home,
        skill,
        shared_skills_dir().as_deref(),
        SkillMaterialize::Link,
    )
}

/// Delete a skill from the global Claude config (~/.claude/)
pub fn delete_global_skill(skill: &Skill) -> Result<()> {
    let base_dirs =
        BaseDirs::new().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let home = base_dirs.home_dir();
    delete_skill_file_with(home, skill, shared_skills_dir().as_deref())
}

/// Write a skill to a project's Claude config ({project}/.claude/)
pub fn write_project_skill(project_path: &Path, skill: &Skill) -> Result<()> {
    write_skill_file_with(
        project_path,
        skill,
        shared_skills_dir().as_deref(),
        SkillMaterialize::Copy,
    )
}

/// Delete a skill from a project's Claude config ({project}/.claude/)
pub fn delete_project_skill(project_path: &Path, skill: &Skill) -> Result<()> {
    delete_skill_file_with(project_path, skill, shared_skills_dir().as_deref())
}

// ============================================================================
// OpenCode Support
// ============================================================================

/// Write a skill to OpenCode's format
/// Agent skills go to {base_path}/agent/{name}.md (OpenCode uses agent/ not skills/)
pub fn write_skill_file_opencode(base_path: &Path, skill: &Skill) -> Result<()> {
    let agent_dir = base_path.join("agent");
    std::fs::create_dir_all(&agent_dir)?;

    let file_path = agent_dir.join(format!("{}.md", skill.name));
    crate::utils::backup::backup_file(&file_path)?;
    let content = generate_skill_markdown(skill);
    crate::utils::atomic::write_atomic(file_path, content)?;

    Ok(())
}

/// Delete a skill from OpenCode's format
pub fn delete_skill_file_opencode(base_path: &Path, skill: &Skill) -> Result<()> {
    let file_path = base_path.join("agent").join(format!("{}.md", skill.name));
    if file_path.exists() {
        std::fs::remove_file(file_path)?;
    }

    Ok(())
}

/// Write a skill to the global OpenCode config (~/.config/opencode/)
pub fn write_global_skill_opencode(skill: &Skill) -> Result<()> {
    let paths = get_opencode_paths()?;
    write_skill_file_opencode(&paths.config_dir, skill)
}

/// Delete a skill from the global OpenCode config
pub fn delete_global_skill_opencode(skill: &Skill) -> Result<()> {
    let paths = get_opencode_paths()?;
    delete_skill_file_opencode(&paths.config_dir, skill)
}

/// Write a skill to a project's OpenCode config ({project}/.opencode/)
pub fn write_project_skill_opencode(project_path: &Path, skill: &Skill) -> Result<()> {
    let opencode_dir = project_path.join(".opencode");
    write_skill_file_opencode(&opencode_dir, skill)
}

/// Delete a skill from a project's OpenCode config
pub fn delete_project_skill_opencode(project_path: &Path, skill: &Skill) -> Result<()> {
    let opencode_dir = project_path.join(".opencode");
    delete_skill_file_opencode(&opencode_dir, skill)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // =========================================================================
    // Helper functions to create test skills
    // =========================================================================

    fn sample_skill() -> Skill {
        Skill {
            id: 1,
            name: "test-agent".to_string(),
            description: Some("An agent skill".to_string()),
            content: "You are a helpful assistant.".to_string(),
            allowed_tools: Some(vec!["Bash".to_string(), "Glob".to_string()]),
            model: Some("opus".to_string()),
            disable_model_invocation: true,
            tags: None,
            source: "manual".to_string(),
            source_path: None,
            is_favorite: false,
            context: None,
            agent: None,
            hooks: None,
            paths: None,
            shell: None,
            once: None,
            effort: None,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        }
    }

    fn sample_minimal_skill() -> Skill {
        Skill {
            id: 2,
            name: "minimal".to_string(),
            description: None,
            content: "Minimal content.".to_string(),
            allowed_tools: None,
            model: None,
            disable_model_invocation: false,
            tags: None,
            source: "manual".to_string(),
            source_path: None,
            is_favorite: false,
            context: None,
            agent: None,
            hooks: None,
            paths: None,
            shell: None,
            once: None,
            effort: None,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        }
    }

    // =========================================================================
    // generate_skill_markdown tests
    // =========================================================================

    #[test]
    fn test_generate_skill_markdown_full() {
        let skill = sample_skill();
        let md = generate_skill_markdown(&skill);

        assert!(md.starts_with("---\n"));
        assert!(md.contains("name: test-agent\n"));
        assert!(md.contains("description: An agent skill\n"));
        assert!(md.contains("allowed-tools: Bash, Glob\n"));
        assert!(md.contains("model: opus\n"));
        assert!(md.contains("disable-model-invocation: true\n"));
        assert!(md.contains("---\n\nYou are a helpful assistant."));
    }

    #[test]
    fn test_generate_skill_markdown_minimal() {
        let skill = sample_minimal_skill();
        let md = generate_skill_markdown(&skill);

        assert!(md.contains("name: minimal\n"));
        assert!(!md.contains("description:"));
        assert!(!md.contains("allowed-tools:"));
        assert!(!md.contains("model:"));
        assert!(!md.contains("disable-model-invocation:"));
    }

    #[test]
    fn test_generate_skill_markdown_always_includes_name() {
        let skill = sample_minimal_skill();
        let md = generate_skill_markdown(&skill);

        assert!(md.contains("name: minimal\n"));
    }

    #[test]
    fn test_generate_skill_markdown_emits_tags_as_json_array() {
        // Mirrors the rule_writer fix: `tags` is read from the DB via
        // `serde_json::from_str`, so if a scanner ever ingests a skill
        // frontmatter the value must be valid JSON, not comma-joined.
        // Also pins against silent-drop: previously `skill.tags` was not
        // written to frontmatter at all.
        let mut skill = sample_minimal_skill();
        skill.tags = Some(vec!["refactor".to_string(), "typescript".to_string()]);

        let md = generate_skill_markdown(&skill);

        assert!(md.contains("tags: [\"refactor\",\"typescript\"]\n"));
    }

    #[test]
    fn test_generate_skill_markdown_omits_empty_tags() {
        let mut skill = sample_minimal_skill();
        skill.tags = Some(vec![]);

        let md = generate_skill_markdown(&skill);

        assert!(!md.contains("tags:"));
    }

    // =========================================================================
    // write_skill_file tests (file system)
    // =========================================================================

    #[test]
    fn test_write_skill_file_creates_correct_path() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();

        write_skill_file(temp_dir.path(), &skill).unwrap();

        let expected_path = temp_dir
            .path()
            .join(".claude")
            .join("skills")
            .join("test-agent")
            .join("SKILL.md");
        assert!(expected_path.exists());
    }

    #[test]
    fn test_write_skill_file_content_matches() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();

        write_skill_file(temp_dir.path(), &skill).unwrap();

        let file_path = temp_dir
            .path()
            .join(".claude")
            .join("skills")
            .join("test-agent")
            .join("SKILL.md");
        let content = std::fs::read_to_string(file_path).unwrap();

        assert!(content.contains("name: test-agent"));
        assert!(content.contains("You are a helpful assistant."));
    }

    // =========================================================================
    // delete_skill_file tests
    // =========================================================================

    #[test]
    fn test_delete_skill_file_removes_directory() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();

        // Write first
        write_skill_file(temp_dir.path(), &skill).unwrap();
        let skill_dir = temp_dir
            .path()
            .join(".claude")
            .join("skills")
            .join("test-agent");
        assert!(skill_dir.exists());

        // Delete
        delete_skill_file(temp_dir.path(), &skill).unwrap();
        assert!(!skill_dir.exists());
    }

    #[test]
    fn test_delete_nonexistent_skill_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();

        // Should not error when file doesn't exist
        let result = delete_skill_file(temp_dir.path(), &skill);
        assert!(result.is_ok());
    }

    // =========================================================================
    // OpenCode format tests
    // =========================================================================

    #[test]
    fn test_write_skill_file_opencode() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();

        write_skill_file_opencode(temp_dir.path(), &skill).unwrap();

        // OpenCode uses "agent" not "skills"
        let expected_path = temp_dir.path().join("agent").join("test-agent.md");
        assert!(expected_path.exists());
    }

    #[test]
    fn test_delete_skill_file_opencode() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();

        write_skill_file_opencode(temp_dir.path(), &skill).unwrap();
        let file_path = temp_dir.path().join("agent").join("test-agent.md");
        assert!(file_path.exists());

        delete_skill_file_opencode(temp_dir.path(), &skill).unwrap();
        assert!(!file_path.exists());
    }

    // =========================================================================
    // Additional coverage: edge cases
    // =========================================================================
    #[test]
    fn test_generate_skill_markdown_empty_description() {
        let mut skill = sample_skill();
        skill.description = Some("".to_string());
        let md = generate_skill_markdown(&skill);
        assert!(!md.contains("description:"));
    }

    #[test]
    fn test_generate_skill_markdown_empty_tools() {
        let mut skill = sample_skill();
        skill.allowed_tools = Some(vec![]);
        let md = generate_skill_markdown(&skill);
        assert!(!md.contains("allowed-tools:"));
    }

    #[test]
    fn test_generate_skill_markdown_empty_model() {
        let mut skill = sample_skill();
        skill.model = Some("".to_string());
        let md = generate_skill_markdown(&skill);
        assert!(!md.contains("model:"));
    }

    #[test]
    fn test_write_project_skill() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();
        write_project_skill(temp_dir.path(), &skill).unwrap();

        let expected = temp_dir
            .path()
            .join(".claude")
            .join("skills")
            .join("test-agent")
            .join("SKILL.md");
        assert!(expected.exists());
    }

    #[test]
    fn test_delete_project_skill() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();
        write_project_skill(temp_dir.path(), &skill).unwrap();
        delete_project_skill(temp_dir.path(), &skill).unwrap();

        let expected = temp_dir
            .path()
            .join(".claude")
            .join("skills")
            .join("test-agent");
        assert!(!expected.exists());
    }

    #[test]
    fn test_write_project_skill_opencode() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();
        write_project_skill_opencode(temp_dir.path(), &skill).unwrap();

        let expected = temp_dir
            .path()
            .join(".opencode")
            .join("agent")
            .join("test-agent.md");
        assert!(expected.exists());
    }

    #[test]
    fn test_delete_project_skill_opencode() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();
        write_project_skill_opencode(temp_dir.path(), &skill).unwrap();
        delete_project_skill_opencode(temp_dir.path(), &skill).unwrap();

        let expected = temp_dir
            .path()
            .join(".opencode")
            .join("agent")
            .join("test-agent.md");
        assert!(!expected.exists());
    }

    #[test]
    fn test_delete_skill_file_opencode_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let skill = sample_skill();
        // Should not error
        let result = delete_skill_file_opencode(temp_dir.path(), &skill);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_skill_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let mut skill = sample_skill();
        write_skill_file(temp_dir.path(), &skill).unwrap();

        // Update content and write again
        skill.content = "Updated content".to_string();
        write_skill_file(temp_dir.path(), &skill).unwrap();

        let file_path = temp_dir
            .path()
            .join(".claude")
            .join("skills")
            .join("test-agent")
            .join("SKILL.md");
        let content = std::fs::read_to_string(file_path).unwrap();
        assert!(content.contains("Updated content"));
    }

    // =========================================================================
    // Shared-skill safety (~/.agents/skills as canonical source)
    // =========================================================================

    fn shared_layout() -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let shared = home.join(".agents").join("skills");
        let src = shared.join("test-agent");
        std::fs::create_dir_all(src.join("scripts")).unwrap();
        std::fs::write(
            src.join("SKILL.md"),
            "---\nname: test-agent\ncustom-field: keep\n---\nORIGINAL\n",
        )
        .unwrap();
        std::fs::write(src.join("scripts").join("run.sh"), "echo hi").unwrap();
        (tmp, home, shared)
    }

    #[cfg(unix)]
    fn assert_link(p: &Path) {
        assert!(
            std::fs::symlink_metadata(p)
                .unwrap()
                .file_type()
                .is_symlink(),
            "{} should be a symlink",
            p.display()
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_global_toggle_links_to_shared_source_and_keeps_it() {
        let (_tmp, home, shared) = shared_layout();
        let skill = sample_skill();
        let dest = home.join(".claude/skills/test-agent");
        let src_md = shared.join("test-agent/SKILL.md");
        let original = std::fs::read_to_string(&src_md).unwrap();

        // on: creates a link, not a regenerated folder
        write_skill_file_with(&home, &skill, Some(&shared), SkillMaterialize::Link).unwrap();
        assert_link(&dest);
        assert!(dest.join("scripts/run.sh").exists());

        // on again: never writes through the link
        write_skill_file_with(&home, &skill, Some(&shared), SkillMaterialize::Link).unwrap();
        assert_link(&dest);
        assert_eq!(std::fs::read_to_string(&src_md).unwrap(), original);

        // off: removes only the link
        delete_skill_file_with(&home, &skill, Some(&shared)).unwrap();
        assert!(std::fs::symlink_metadata(&dest).is_err());
        assert_eq!(std::fs::read_to_string(&src_md).unwrap(), original);
        assert!(shared.join("test-agent/scripts/run.sh").exists());

        // on again: link restored
        write_skill_file_with(&home, &skill, Some(&shared), SkillMaterialize::Link).unwrap();
        assert_link(&dest);
        assert_eq!(std::fs::read_to_string(&src_md).unwrap(), original);
    }

    #[cfg(unix)]
    #[test]
    fn test_existing_user_symlink_is_never_written_through() {
        let (_tmp, home, shared) = shared_layout();
        let skill = sample_skill(); // DB content differs from the source
        let dest = home.join(".claude/skills/test-agent");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(shared.join("test-agent"), &dest).unwrap();
        let src_md = shared.join("test-agent/SKILL.md");
        let original = std::fs::read_to_string(&src_md).unwrap();

        write_skill_file_with(&home, &skill, None, SkillMaterialize::Link).unwrap();
        assert_link(&dest);
        assert_eq!(std::fs::read_to_string(&src_md).unwrap(), original);
        assert!(!shared.join("test-agent/SKILL.md.bak").exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_source_found_via_source_path_when_link_was_removed() {
        let (_tmp, home, shared) = shared_layout();
        let mut skill = sample_skill();
        // scanner recorded the (now removed) link path
        skill.source_path = Some(
            home.join(".claude/skills/test-agent")
                .to_string_lossy()
                .to_string(),
        );
        // shared dir unknown to the writer, but source_path of another copy is known
        let mut skill2 = skill.clone();
        skill2.source_path = Some(shared.join("test-agent").to_string_lossy().to_string());

        write_skill_file_with(&home, &skill2, None, SkillMaterialize::Link).unwrap();
        assert_link(&home.join(".claude/skills/test-agent"));
    }

    #[cfg(unix)]
    #[test]
    fn test_dangling_link_without_source_is_refused_and_kept() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let dest = home.join(".claude/skills/test-agent");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(home.join("nowhere"), &dest).unwrap();

        let err = write_skill_file_with(home, &sample_skill(), None, SkillMaterialize::Link);
        assert!(err.is_err());
        assert_link(&dest);
    }

    #[test]
    fn test_real_folder_toggle_off_is_parked_and_restored_intact() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let dest = base.join(".claude/skills/test-agent");
        std::fs::create_dir_all(dest.join("references")).unwrap();
        std::fs::write(
            dest.join("SKILL.md"),
            "---\nname: test-agent\nx-custom: 1\n---\nBODY",
        )
        .unwrap();
        std::fs::write(dest.join("references/doc.md"), "ref").unwrap();
        let skill = sample_skill();

        delete_skill_file_with(base, &skill, None).unwrap();
        assert!(!dest.exists());
        let parked = base
            .join(".claude")
            .join(DISABLED_SKILLS_DIR)
            .join("test-agent");
        assert!(parked.join("references/doc.md").exists());

        write_skill_file_with(base, &skill, None, SkillMaterialize::Copy).unwrap();
        assert!(dest.join("references/doc.md").exists());
        assert!(std::fs::read_to_string(dest.join("SKILL.md"))
            .unwrap()
            .contains("x-custom: 1"));
        assert!(!parked.exists());
    }

    #[test]
    fn test_parking_twice_keeps_previous_parked_copy() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let dest = base.join(".claude/skills/test-agent");
        let skill = sample_skill();
        for body in ["one", "two"] {
            std::fs::create_dir_all(&dest).unwrap();
            std::fs::write(dest.join("SKILL.md"), body).unwrap();
            delete_skill_file_with(base, &skill, None).unwrap();
        }
        let parked_root = base.join(".claude").join(DISABLED_SKILLS_DIR);
        let n = std::fs::read_dir(&parked_root).unwrap().count();
        assert_eq!(n, 2);
    }

    #[test]
    fn test_refuses_to_remove_canonical_source() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let skills = base.join(".claude/skills");
        std::fs::create_dir_all(skills.join("test-agent")).unwrap();
        std::fs::write(skills.join("test-agent/SKILL.md"), "x").unwrap();

        let err = delete_skill_file_with(base, &sample_skill(), Some(&skills));
        assert!(err.is_err());
        assert!(skills.join("test-agent/SKILL.md").exists());
    }

    #[test]
    fn test_project_scope_copies_whole_folder() {
        let (_tmp, home, shared) = shared_layout();
        let project = home.join("proj");
        std::fs::create_dir_all(&project).unwrap();
        write_skill_file_with(
            &project,
            &sample_skill(),
            Some(&shared),
            SkillMaterialize::Copy,
        )
        .unwrap();
        let dest = project.join(".claude/skills/test-agent");
        assert!(!std::fs::symlink_metadata(&dest)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(dest.join("scripts/run.sh").exists());
        assert!(std::fs::read_to_string(dest.join("SKILL.md"))
            .unwrap()
            .contains("ORIGINAL"));
    }
}
