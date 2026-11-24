use crate::db::Database;
use crate::services::scanner as scanner_service;
use std::sync::{Arc, Mutex};
use tauri::State;

#[tauri::command]
pub fn scan_claude_directory(db: State<'_, Arc<Mutex<Database>>>) -> Result<usize, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    scanner_service::run_full_scan(&db).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    // scanner commands require a full Database instance with tables created.
    // The important logic lives in the service layer (scanner_service), which
    // has its own tests. Here we verify the module compiles correctly.

    #[test]
    fn test_scanner_module_compiles() {
        // Compilation test: ensures imports and function signatures are valid.
        // No runtime behavior to check — the fact that this file compiles is
        // the assertion.
    }
}
