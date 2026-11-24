//! Cache tables for the multi-CLI resource model.
//!
//! - `resource_meta`: user-facing metadata keyed by resource `uid`
//!   (favorite, tags, UI state). Survives renames of the legacy tables.
//! - `resource_occurrences`: where each resource was seen (CLI, scope, file).
//!
//! Neither table is a source of config: both can be dropped and rebuilt.

use super::schema::Database;
use crate::sync::model::{resource_uid, Cli, ResourceKind, Scope};
use crate::sync::occurrences::Occurrence;
use anyhow::Result;
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMeta {
    pub uid: String,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub ui_state: Option<serde_json::Value>,
}

pub(crate) const RESOURCE_TABLES_SQL: &str = r#"
    CREATE TABLE IF NOT EXISTS resource_meta (
        uid TEXT PRIMARY KEY,
        kind TEXT NOT NULL,
        name TEXT NOT NULL,
        is_favorite INTEGER NOT NULL DEFAULT 0,
        tags TEXT,
        ui_state TEXT,
        updated_at TEXT DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS resource_occurrences (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        uid TEXT NOT NULL,
        kind TEXT NOT NULL,
        name TEXT NOT NULL,
        cli TEXT NOT NULL,
        scope TEXT NOT NULL,
        project_path TEXT,
        file_path TEXT NOT NULL,
        seen_at TEXT DEFAULT CURRENT_TIMESTAMP,
        UNIQUE (uid, cli, file_path)
    );
    CREATE INDEX IF NOT EXISTS idx_resource_occurrences_uid ON resource_occurrences(uid);
"#;

fn kind_from_key(k: &str) -> Option<ResourceKind> {
    match k {
        "mcp" => Some(ResourceKind::Mcp),
        "skill" => Some(ResourceKind::Skill),
        "agent" => Some(ResourceKind::Agent),
        "hook" => Some(ResourceKind::Hook),
        _ => None,
    }
}

impl Database {
    /// Create the cache tables and carry favorites/tags of the legacy
    /// library tables over to `resource_meta` (idempotent: existing meta
    /// rows are never overwritten).
    pub(crate) fn migrate_resource_tables(&self) -> Result<()> {
        self.conn().execute_batch(RESOURCE_TABLES_SQL)?;

        // (legacy table, assignment tables, kind)
        let specs: [(&str, &str, &str, &str, ResourceKind); 3] = [
            (
                "mcps",
                "global_mcps",
                "project_mcps",
                "mcp_id",
                ResourceKind::Mcp,
            ),
            (
                "skills",
                "global_skills",
                "project_skills",
                "skill_id",
                ResourceKind::Skill,
            ),
            (
                "subagents",
                "global_subagents",
                "project_subagents",
                "subagent_id",
                ResourceKind::Agent,
            ),
        ];
        for (table, global_t, project_t, fk, kind) in specs {
            let rows: Vec<(i64, String, bool, Option<String>)> = {
                let sql = format!(
                    "SELECT id, name, COALESCE(is_favorite, 0), tags FROM {} \
                     WHERE COALESCE(is_favorite, 0) != 0 OR (tags IS NOT NULL AND tags NOT IN ('', '[]', 'null'))",
                    table
                );
                let mut st = match self.conn().prepare(&sql) {
                    Ok(st) => st,
                    Err(_) => continue, // legacy table missing in very old DBs
                };
                let mapped = st.query_map([], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get(3)?))
                })?;
                mapped.filter_map(|r| r.ok()).collect()
            };
            for (id, name, fav, tags) in rows {
                let mut scopes: Vec<Scope> = Vec::new();
                let in_global: bool = self
                    .conn()
                    .query_row(
                        &format!("SELECT COUNT(*) > 0 FROM {} WHERE {} = ?", global_t, fk),
                        [id],
                        |r| r.get(0),
                    )
                    .unwrap_or(false);
                if in_global {
                    scopes.push(Scope::Global);
                }
                if let Ok(mut st) = self.conn().prepare(&format!(
                    "SELECT p.path FROM {} a JOIN projects p ON p.id = a.project_id WHERE a.{} = ?",
                    project_t, fk
                )) {
                    let paths: Vec<String> = st
                        .query_map([id], |r| r.get(0))?
                        .filter_map(|r| r.ok())
                        .collect();
                    scopes.extend(paths.into_iter().map(|path| Scope::Project { path }));
                }
                if scopes.is_empty() {
                    // Library-only item: best identity available is global.
                    scopes.push(Scope::Global);
                }
                let tags_json = tags
                    .as_deref()
                    .and_then(|t| serde_json::from_str::<Vec<String>>(t).ok())
                    .map(|t| serde_json::to_string(&t).unwrap());
                for scope in scopes {
                    self.conn().execute(
                        "INSERT OR IGNORE INTO resource_meta (uid, kind, name, is_favorite, tags) VALUES (?, ?, ?, ?, ?)",
                        params![
                            resource_uid(kind, &scope, &name),
                            kind.key(),
                            name,
                            fav as i32,
                            tags_json
                        ],
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn get_resource_meta_all(&self) -> Result<Vec<ResourceMeta>> {
        let mut st = self
            .conn()
            .prepare("SELECT uid, is_favorite, tags, ui_state FROM resource_meta")?;
        let rows = st.query_map([], |r| {
            let tags: Option<String> = r.get(2)?;
            let ui: Option<String> = r.get(3)?;
            Ok(ResourceMeta {
                uid: r.get(0)?,
                is_favorite: r.get::<_, i64>(1)? != 0,
                tags: tags
                    .and_then(|t| serde_json::from_str(&t).ok())
                    .unwrap_or_default(),
                ui_state: ui.and_then(|u| serde_json::from_str(&u).ok()),
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn set_resource_favorite(&self, uid: &str, favorite: bool) -> Result<()> {
        let (kind, name) = split_uid(uid);
        self.conn().execute(
            "INSERT INTO resource_meta (uid, kind, name, is_favorite) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(uid) DO UPDATE SET is_favorite = ?4, updated_at = CURRENT_TIMESTAMP",
            params![uid, kind, name, favorite as i32],
        )?;
        Ok(())
    }

    pub fn set_resource_tags(&self, uid: &str, tags: &[String]) -> Result<()> {
        let (kind, name) = split_uid(uid);
        let json = serde_json::to_string(tags)?;
        self.conn().execute(
            "INSERT INTO resource_meta (uid, kind, name, tags) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(uid) DO UPDATE SET tags = ?4, updated_at = CURRENT_TIMESTAMP",
            params![uid, kind, name, json],
        )?;
        Ok(())
    }

    /// Replace the whole occurrence index (it is a snapshot of the disk).
    pub fn replace_occurrences(&self, occ: &[Occurrence]) -> Result<()> {
        let tx = self.conn().unchecked_transaction()?;
        tx.execute("DELETE FROM resource_occurrences", [])?;
        {
            let mut st = tx.prepare(
                "INSERT OR IGNORE INTO resource_occurrences (uid, kind, name, cli, scope, project_path, file_path)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )?;
            for o in occ {
                st.execute(params![
                    o.uid,
                    o.kind.key(),
                    o.name,
                    o.cli.key(),
                    o.scope.key(),
                    o.scope.project_path(),
                    o.file_path
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_occurrences(&self) -> Result<Vec<Occurrence>> {
        let mut st = self.conn().prepare(
            "SELECT uid, kind, name, cli, project_path, file_path FROM resource_occurrences ORDER BY uid, cli",
        )?;
        let rows = st.query_map([], |r| {
            let kind: String = r.get(1)?;
            let cli: String = r.get(3)?;
            let project: Option<String> = r.get(4)?;
            Ok((
                r.get::<_, String>(0)?,
                kind,
                r.get::<_, String>(2)?,
                cli,
                project,
                r.get::<_, String>(5)?,
            ))
        })?;
        Ok(rows
            .filter_map(|r| r.ok())
            .filter_map(|(uid, kind, name, cli, project, file_path)| {
                Some(Occurrence {
                    uid,
                    kind: kind_from_key(&kind)?,
                    name,
                    cli: Cli::from_key(&cli)?,
                    scope: match project {
                        Some(path) => Scope::Project { path },
                        None => Scope::Global,
                    },
                    file_path,
                })
            })
            .collect())
    }
}

/// `kind:scope...:name` → (kind, name). The name is after the last ':'
/// (names cannot contain ':' in any CLI config format we support).
fn split_uid(uid: &str) -> (String, String) {
    let kind = uid.split(':').next().unwrap_or("").to_string();
    let name = uid.rsplit(':').next().unwrap_or("").to_string();
    (kind, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_carries_favorites_and_tags_by_scope() {
        let db = Database::in_memory().unwrap();
        let c = db.conn();
        c.execute("INSERT INTO mcps (name, type, command, is_favorite, tags) VALUES ('Neon','http','x',1,'[\"db\"]')", []).unwrap();
        c.execute(
            "INSERT INTO mcps (name, type, command) VALUES ('plain','stdio','x')",
            [],
        )
        .unwrap();
        c.execute("INSERT INTO projects (name, path) VALUES ('p','/w/p')", [])
            .unwrap();
        let mcp: i64 = c
            .query_row("SELECT id FROM mcps WHERE name='Neon'", [], |r| r.get(0))
            .unwrap();
        c.execute("INSERT INTO global_mcps (mcp_id) VALUES (?)", [mcp])
            .unwrap();
        c.execute(
            "INSERT INTO project_mcps (project_id, mcp_id) VALUES (1, ?)",
            [mcp],
        )
        .unwrap();
        c.execute(
            "INSERT INTO skills (name, content, is_favorite) VALUES ('sk','body',1)",
            [],
        )
        .unwrap();

        db.migrate_resource_tables().unwrap();
        db.migrate_resource_tables().unwrap(); // idempotent

        let meta = db.get_resource_meta_all().unwrap();
        let get = |uid: &str| meta.iter().find(|m| m.uid == uid).cloned();
        let g = get("mcp:global:Neon").expect("global meta");
        assert!(g.is_favorite);
        assert_eq!(g.tags, vec!["db"]);
        assert!(get("mcp:project:/w/p:Neon").is_some());
        assert!(get("mcp:global:plain").is_none());
        assert!(get("skill:global:sk").unwrap().is_favorite);
    }

    #[test]
    fn favorites_and_tags_upsert() {
        let db = Database::in_memory().unwrap();
        db.set_resource_favorite("mcp:global:a", true).unwrap();
        db.set_resource_tags("mcp:global:a", &["x".into()]).unwrap();
        db.set_resource_favorite("mcp:global:a", false).unwrap();
        let m = db.get_resource_meta_all().unwrap();
        assert_eq!(m.len(), 1);
        assert!(!m[0].is_favorite);
        assert_eq!(m[0].tags, vec!["x"]);
    }

    #[test]
    fn occurrences_roundtrip_and_replace() {
        let db = Database::in_memory().unwrap();
        let o = Occurrence {
            uid: "mcp:project:/w/p:sb".into(),
            kind: ResourceKind::Mcp,
            name: "sb".into(),
            cli: Cli::Codex,
            scope: Scope::Project {
                path: "/w/p".into(),
            },
            file_path: "/w/p/.codex/config.toml".into(),
        };
        db.replace_occurrences(&[o.clone(), o.clone()]).unwrap();
        assert_eq!(db.get_occurrences().unwrap(), vec![o]);
        db.replace_occurrences(&[]).unwrap();
        assert!(db.get_occurrences().unwrap().is_empty());
    }
}
