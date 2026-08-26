//! Optional SQLite persistence layer.
//!
//! When enabled via `--persist`, every mutating operation writes through to a
//! local SQLite database so that state survives process restarts.
//!
//! The schema is intentionally simple: one table per logical resource type with
//! (service TEXT, key TEXT, data TEXT) where `data` is a JSON blob. This keeps
//! the persistence layer decoupled from individual service structs.

use std::path::Path;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use rusqlite::{params, Connection};

/// A thin wrapper around a SQLite connection that provides key-value storage
/// grouped by service table name.
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

/// Validate a table name and return it quoted as a SQL identifier.
///
/// Table names come from code, not user input, but this API is `pub` and the
/// name is interpolated into SQL text, so we refuse anything that isn't a plain
/// identifier (`[A-Za-z_][A-Za-z0-9_]*`) and additionally quote it with
/// standard double quotes. This makes injection via `]`, `"`, `;` etc. impossible.
fn quoted_table(table: &str) -> Result<String, String> {
    let mut chars = table.chars();
    let valid_first = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_');
    let valid_rest = chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid_first || !valid_rest || table.len() > 128 {
        return Err(format!(
            "invalid table name {table:?}: must match [A-Za-z_][A-Za-z0-9_]* (max 128 chars)"
        ));
    }
    Ok(format!("\"{table}\""))
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
        let q = quoted_table(table)?;
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {q} (
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
        let sql = format!(
            "INSERT OR REPLACE INTO {} (key, data) VALUES (?1, ?2)",
            quoted_table(table)?
        );
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(&sql, params![key, data])
            .map_err(|e| format!("put({table}, {key}): {e}"))?;
        Ok(())
    }

    /// Get a single value by key.
    pub fn get(&self, table: &str, key: &str) -> Result<Option<String>, String> {
        self.ensure_table(table)?;
        let sql = format!("SELECT data FROM {} WHERE key = ?1", quoted_table(table)?);
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
        let sql = format!("DELETE FROM {} WHERE key = ?1", quoted_table(table)?);
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(&sql, params![key])
            .map_err(|e| format!("delete({table}, {key}): {e}"))?;
        Ok(())
    }

    /// Delete all rows from a table.
    pub fn delete_all(&self, table: &str) -> Result<(), String> {
        self.ensure_table(table)?;
        let sql = format!("DELETE FROM {}", quoted_table(table)?);
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(&sql, [])
            .map_err(|e| format!("delete_all({table}): {e}"))?;
        Ok(())
    }

    /// List all (key, data) pairs in a table.
    pub fn list(&self, table: &str) -> Result<Vec<(String, String)>, String> {
        self.ensure_table(table)?;
        let sql = format!("SELECT key, data FROM {}", quoted_table(table)?);
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

// ---------------------------------------------------------------------------
// PersistedDashMap — drop-in replacement for DashMap<String, V>
// ---------------------------------------------------------------------------

/// A `DashMap<String, V>` wrapper that optionally persists mutations to SQLite.
///
/// When constructed without a `SqliteStore` (i.e. `--persist` is off), it
/// behaves identically to a plain `DashMap`. When a store is provided, every
/// `insert` / `remove` is written through, and the map is rehydrated on
/// construction.
///
/// This is a **drop-in** replacement: it exposes the same surface as
/// `DashMap` so existing service code compiles without changes beyond
/// swapping the type.
pub struct PersistedDashMap<V: Clone + Send + Sync + 'static> {
    inner: DashMap<String, V>,
    db: Option<Arc<SqliteStore>>,
    table: String,
}

impl<V: Clone + Send + Sync + 'static> std::fmt::Debug for PersistedDashMap<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistedDashMap")
            .field("len", &self.inner.len())
            .field("table", &self.table)
            .finish()
    }
}

impl<V: Clone + Send + Sync + 'static> Default for PersistedDashMap<V> {
    fn default() -> Self {
        Self {
            inner: DashMap::new(),
            db: None,
            table: String::new(),
        }
    }
}

impl<V: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned + 'static>
    PersistedDashMap<V>
{
    /// Create a map that is optionally backed by SQLite.
    /// When `db` is `None`, behaves like a plain in-memory `DashMap`.
    /// When `Some`, rehydrates from SQLite and writes through on mutations.
    pub fn new(table: &str, db: &Option<Arc<SqliteStore>>) -> Self {
        match db {
            Some(db) => Self::with_persistence(table, db.clone()),
            None => Self::default(),
        }
    }

    /// Create a persisted map that rehydrates from SQLite.
    pub fn with_persistence(table: &str, db: Arc<SqliteStore>) -> Self {
        let inner = DashMap::new();
        if let Ok(rows) = db.list(table) {
            for (key, json) in rows {
                if let Ok(val) = serde_json::from_str::<V>(&json) {
                    inner.insert(key, val);
                }
            }
        }
        Self {
            inner,
            db: Some(db),
            table: table.to_string(),
        }
    }
}

impl<V: Clone + Send + Sync + 'static> PersistedDashMap<V> {
    /// Insert a key-value pair (with optional write-through).
    pub fn insert(&self, key: String, value: V)
    where
        V: serde::Serialize,
    {
        if let Some(ref db) = self.db {
            if let Ok(json) = serde_json::to_string(&value) {
                let _ = db.put(&self.table, &key, &json);
            }
        }
        self.inner.insert(key, value);
    }

    /// Remove by key (with optional write-through).
    pub fn remove(&self, key: &str) -> Option<(String, V)> {
        if let Some(ref db) = self.db {
            let _ = db.delete(&self.table, key);
        }
        self.inner.remove(key)
    }

    pub fn get(&self, key: &str) -> Option<dashmap::mapref::one::Ref<'_, String, V>> {
        self.inner.get(key)
    }

    pub fn get_mut(&self, key: &str) -> Option<dashmap::mapref::one::RefMut<'_, String, V>> {
        self.inner.get_mut(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    pub fn iter(&self) -> dashmap::iter::Iter<'_, String, V> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&self)
    where
        V: serde::Serialize,
    {
        self.inner.clear();
        if let Some(ref db) = self.db {
            let _ = db.delete_all(&self.table);
        }
    }

    pub fn entry(&self, key: String) -> dashmap::Entry<'_, String, V> {
        self.inner.entry(key)
    }

    pub fn iter_mut(&self) -> dashmap::iter::IterMut<'_, String, V> {
        self.inner.iter_mut()
    }

    /// Retain only entries that satisfy the predicate.
    pub fn retain(&self, f: impl FnMut(&String, &mut V) -> bool)
    where
        V: serde::Serialize,
    {
        self.inner.retain(f);
        // Re-persist everything after retain (simplest approach)
        if let Some(ref db) = self.db {
            // Wipe the table and rewrite (retain may remove many)
            let _ = db.delete_all(&self.table);
            for entry in self.inner.iter() {
                if let Ok(json) = serde_json::to_string(entry.value()) {
                    let _ = db.put(&self.table, entry.key(), &json);
                }
            }
        }
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
    fn rejects_malicious_table_names() {
        let (store, _dir) = temp_db();
        for bad in [
            "x]; DROP TABLE users; --",
            "a\"b",
            "with space",
            "semi;colon",
            "",
            "1starts_with_digit",
            "dash-name",
        ] {
            assert!(
                store.ensure_table(bad).is_err(),
                "{bad:?} should be rejected"
            );
            assert!(
                store.put(bad, "k", "v").is_err(),
                "{bad:?} should be rejected"
            );
            assert!(store.get(bad, "k").is_err(), "{bad:?} should be rejected");
            assert!(
                store.delete(bad, "k").is_err(),
                "{bad:?} should be rejected"
            );
            assert!(store.delete_all(bad).is_err(), "{bad:?} should be rejected");
            assert!(store.list(bad).is_err(), "{bad:?} should be rejected");
        }
        // Nothing was created as a side effect.
        let conn = store.conn.lock().unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn accepts_identifier_table_names() {
        let (store, _dir) = temp_db();
        for ok in ["_x", "ecr_repositories", "T1", "a_b_c_123"] {
            store.put(ok, "k", "v").unwrap();
            assert_eq!(store.get(ok, "k").unwrap().as_deref(), Some("v"));
        }
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
