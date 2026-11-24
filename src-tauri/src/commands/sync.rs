//! Tauri commands for the multi-CLI resource model.

use crate::db::resources::ResourceMeta;
use crate::db::Database;
use crate::sync::model::Resource;
use crate::sync::occurrences::{self, Occurrence};
use crate::sync::source::SourceSnapshot;
use directories::BaseDirs;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceView {
    #[serde(flatten)]
    pub resource: Resource,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub occurrences: Vec<Occurrence>,
}

pub(crate) fn home_dir() -> Result<PathBuf, String> {
    BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .ok_or_else(|| "Não foi possível descobrir a pasta do usuário".to_string())
}

/// Project folders to inspect: managed ones (sync.json) + the app's list.
pub(crate) fn known_projects(db: &Database, snap: &SourceSnapshot) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = snap.projects.iter().map(|p| p.path.clone()).collect();
    if let Ok(projects) = db.get_all_projects() {
        for p in projects {
            let path = PathBuf::from(&p.path);
            if path.is_dir() && !out.contains(&path) {
                out.push(path);
            }
        }
    }
    out
}

/// Rebuild the occurrence index from disk (read-only scan).
pub(crate) fn refresh_occurrences_in_db(db: &Database) -> Result<usize, String> {
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let occ = occurrences::collect(&home, &known_projects(db, &snap));
    db.replace_occurrences(&occ).map_err(|e| e.to_string())?;
    Ok(occ.len())
}

pub(crate) fn build_views(
    resources: Vec<Resource>,
    meta: &[ResourceMeta],
    occ: &[Occurrence],
) -> Vec<ResourceView> {
    resources
        .into_iter()
        .map(|r| {
            let m = meta.iter().find(|m| m.uid == r.uid);
            ResourceView {
                is_favorite: m.map(|m| m.is_favorite).unwrap_or(false),
                tags: m.map(|m| m.tags.clone()).unwrap_or_default(),
                occurrences: occ.iter().filter(|o| o.uid == r.uid).cloned().collect(),
                resource: r,
            }
        })
        .collect()
}

/// Every resource of the source of truth with its CLI target matrix,
/// cached metadata and where it was seen on disk.
#[tauri::command]
pub fn get_sync_resources(
    db: State<'_, Arc<Mutex<Database>>>,
) -> Result<Vec<ResourceView>, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let home = home_dir()?;
    let snap = SourceSnapshot::load(&home);
    let meta = db.get_resource_meta_all().map_err(|e| e.to_string())?;
    let occ = db.get_occurrences().map_err(|e| e.to_string())?;
    Ok(build_views(snap.resources(), &meta, &occ))
}

#[tauri::command]
pub fn refresh_sync_occurrences(db: State<'_, Arc<Mutex<Database>>>) -> Result<usize, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    refresh_occurrences_in_db(&db)
}

#[tauri::command]
pub fn set_resource_favorite(
    db: State<'_, Arc<Mutex<Database>>>,
    uid: String,
    favorite: bool,
) -> Result<(), String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.set_resource_favorite(&uid, favorite)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_resource_tags(
    db: State<'_, Arc<Mutex<Database>>>,
    uid: String,
    tags: Vec<String>,
) -> Result<(), String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    db.set_resource_tags(&uid, &tags).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::model::*;

    #[test]
    fn views_merge_meta_and_occurrences() {
        let r = Resource {
            uid: "mcp:global:a".into(),
            kind: ResourceKind::Mcp,
            name: "a".into(),
            scope: Scope::Global,
            canonical: "/h/.agents/mcp/global.json".into(),
            config: None,
            transport: Some("stdio".into()),
            description: None,
            targets: Default::default(),
            overrides: Default::default(),
            secret_refs: vec![],
            literal_secret_keys: vec![],
        };
        let meta = vec![ResourceMeta {
            uid: "mcp:global:a".into(),
            is_favorite: true,
            tags: vec!["t".into()],
            ui_state: None,
        }];
        let occ = vec![Occurrence {
            uid: "mcp:global:a".into(),
            kind: ResourceKind::Mcp,
            name: "a".into(),
            cli: Cli::Codex,
            scope: Scope::Global,
            file_path: "/h/.codex/config.toml".into(),
        }];
        let v = build_views(vec![r], &meta, &occ);
        assert!(v[0].is_favorite);
        assert_eq!(v[0].occurrences.len(), 1);
        let json = serde_json::to_value(&v[0]).unwrap();
        assert_eq!(json["uid"], "mcp:global:a");
        assert_eq!(json["isFavorite"], true);
    }
}
