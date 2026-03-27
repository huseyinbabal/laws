//! Optional SQLite persistence layer.
//!
//! When enabled via `--persist`, every mutating operation writes through to a
//! local SQLite database so that state survives process restarts.
//!
//! The schema is intentionally simple: one table per logical resource type with
//! (service TEXT, key TEXT, data TEXT) where `data` is a JSON blob. This keeps
//! the persistence layer decoupled from individual service structs.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};

/// A thin wrapper around a SQLite connection that provides key-value storage
/// grouped by service table name.
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Open (or create) a SQLite database at `path`.
    /// Creates the parent directory if it doesn't exist.
    pub fn open(path: &str) -> Result<Self, String> {
        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create db directory: {e}"))?;
        }
        let conn = Connection::open(path)
            .map_err(|e| format!("failed to open sqlite db at {path}: {e}"))?;

        // Enable WAL mode for better concurrent read performance.
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("failed to set WAL mode: {e}"))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Delete the database file, effectively wiping all persisted state.
    pub fn reset(path: &str) -> Result<(), String> {
        let p = Path::new(path);
        if p.exists() {
            std::fs::remove_file(p).map_err(|e| format!("failed to remove db at {path}: {e}"))?;
            // Also remove WAL/SHM sidecar files if present.
            let wal = format!("{path}-wal");
            let shm = format!("{path}-shm");
            let _ = std::fs::remove_file(&wal);
            let _ = std::fs::remove_file(&shm);
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Table management
    // ------------------------------------------------------------------

    /// Ensure a table for the given service exists.
    pub fn ensure_table(&self, table: &str) -> Result<(), String> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS [{table}] (
                key   TEXT PRIMARY KEY,
                data  TEXT NOT NULL
            )"
        );
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(&sql, [])
            .map_err(|e| format!("ensure_table({table}): {e}"))?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // CRUD
    // ------------------------------------------------------------------

    /// Insert or replace a key-value pair.
    pub fn put(&self, table: &str, key: &str, data: &str) -> Result<(), String> {
        self.ensure_table(table)?;
        let sql = format!("INSERT OR REPLACE INTO [{table}] (key, data) VALUES (?1, ?2)");
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(&sql, params![key, data])
            .map_err(|e| format!("put({table}, {key}): {e}"))?;
        Ok(())
    }

    /// Get a single value by key.
    pub fn get(&self, table: &str, key: &str) -> Result<Option<String>, String> {
        self.ensure_table(table)?;
        let sql = format!("SELECT data FROM [{table}] WHERE key = ?1");
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("get({table}): {e}"))?;
        let mut rows = stmt
            .query(params![key])
            .map_err(|e| format!("get({table}, {key}): {e}"))?;
        match rows.next().map_err(|e| e.to_string())? {
            Some(row) => {
                let data: String = row.get(0).map_err(|e| e.to_string())?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Remove a key.
    pub fn delete(&self, table: &str, key: &str) -> Result<(), String> {
        self.ensure_table(table)?;
        let sql = format!("DELETE FROM [{table}] WHERE key = ?1");
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(&sql, params![key])
            .map_err(|e| format!("delete({table}, {key}): {e}"))?;
        Ok(())
    }

    /// List all (key, data) pairs in a table.
    pub fn list(&self, table: &str) -> Result<Vec<(String, String)>, String> {
        self.ensure_table(table)?;
        let sql = format!("SELECT key, data FROM [{table}]");
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("list({table}): {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("list({table}): {e}"))?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| e.to_string())?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (SqliteStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = SqliteStore::open(path.to_str().unwrap()).unwrap();
        (store, dir)
    }

    #[test]
    fn put_get_delete() {
        let (store, _dir) = temp_db();
        store.put("test_svc", "key1", r#"{"name":"a"}"#).unwrap();
        let val = store.get("test_svc", "key1").unwrap();
        assert_eq!(val, Some(r#"{"name":"a"}"#.to_string()));

        store.delete("test_svc", "key1").unwrap();
        let val = store.get("test_svc", "key1").unwrap();
        assert_eq!(val, None);
    }

    #[test]
    fn list_returns_all() {
        let (store, _dir) = temp_db();
        store.put("tbl", "k1", "d1").unwrap();
        store.put("tbl", "k2", "d2").unwrap();
        let mut items = store.list("tbl").unwrap();
        items.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], ("k1".to_string(), "d1".to_string()));
    }

    #[test]
    fn upsert_overwrites() {
        let (store, _dir) = temp_db();
        store.put("tbl", "k", "v1").unwrap();
        store.put("tbl", "k", "v2").unwrap();
        let val = store.get("tbl", "k").unwrap();
        assert_eq!(val, Some("v2".to_string()));
    }

    #[test]
    fn reset_wipes_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let path_str = path.to_str().unwrap();
        {
            let store = SqliteStore::open(path_str).unwrap();
            store.put("tbl", "k", "v").unwrap();
        }
        assert!(path.exists());
        SqliteStore::reset(path_str).unwrap();
        assert!(!path.exists());
    }
}
