pub mod mem {
    use dashmap::DashMap;
    use std::sync::Arc;

    use crate::persistence::SqliteStore;

    #[derive(Clone)]
    pub struct MemoryStore<V: Clone + Send + Sync + 'static> {
        data: Arc<DashMap<String, V>>,
        /// Optional persistence backend. When present, every mutation is
        /// written through to SQLite and the store is rehydrated on creation.
        db: Option<Arc<SqliteStore>>,
        /// Logical table name used as the SQLite table.
        table: String,
    }

    impl<V: Clone + Send + Sync + 'static> Default for MemoryStore<V> {
        fn default() -> Self {
            Self {
                data: Arc::new(DashMap::new()),
                db: None,
                table: String::new(),
            }
        }
    }

    // Core operations that do NOT require Serialize/Deserialize.
    // These work for all existing services unchanged.
    impl<V: Clone + Send + Sync + 'static> MemoryStore<V> {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn insert(&self, key: String, value: V) {
            // Note: persistence write-through happens via `insert_persist`
            // for types that implement Serialize. Plain `insert` always works.
            self.data.insert(key, value);
        }

        pub fn get(&self, key: &str) -> Option<V> {
            self.data.get(key).map(|v| v.value().clone())
        }

        pub fn remove(&self, key: &str) -> Option<V> {
            if let Some(ref db) = self.db {
                let _ = db.delete(&self.table, key);
            }
            self.data.remove(key).map(|(_, v)| v)
        }

        pub fn contains(&self, key: &str) -> bool {
            self.data.contains_key(key)
        }

        pub fn list(&self) -> Vec<(String, V)> {
            self.data
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().clone()))
                .collect()
        }

        pub fn list_values(&self) -> Vec<V> {
            self.data
                .iter()
                .map(|entry| entry.value().clone())
                .collect()
        }

        pub fn len(&self) -> usize {
            self.data.len()
        }

        pub fn is_empty(&self) -> bool {
            self.data.is_empty()
        }
    }

    // Persistence-specific constructor that requires Serialize + DeserializeOwned.
    impl<V: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned + 'static>
        MemoryStore<V>
    {
        /// Insert with persistence write-through. For types that implement
        /// Serialize, this also writes the value to SQLite.
        pub fn insert_persist(&self, key: String, value: V) {
            if let Some(ref db) = self.db {
                if let Ok(json) = serde_json::to_string(&value) {
                    let _ = db.put(&self.table, &key, &json);
                }
            }
            self.data.insert(key, value);
        }

        /// Create a persistence-backed store. All existing rows are loaded
        /// into the in-memory map on construction.
        pub fn with_persistence(table: &str, db: Arc<SqliteStore>) -> Self {
            let data = Arc::new(DashMap::new());

            // Rehydrate from SQLite
            if let Ok(rows) = db.list(table) {
                for (key, json) in rows {
                    if let Ok(val) = serde_json::from_str::<V>(&json) {
                        data.insert(key, val);
                    }
                }
            }

            Self {
                data,
                db: Some(db),
                table: table.to_string(),
            }
        }
    }
}
