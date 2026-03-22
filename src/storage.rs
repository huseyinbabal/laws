pub mod mem {
    use dashmap::DashMap;
    use std::sync::Arc;

    #[derive(Clone)]
    pub struct MemoryStore<V: Clone + Send + Sync + 'static> {
        data: Arc<DashMap<String, V>>,
    }

    impl<V: Clone + Send + Sync + 'static> Default for MemoryStore<V> {
        fn default() -> Self {
            Self {
                data: Arc::new(DashMap::new()),
            }
        }
    }

    impl<V: Clone + Send + Sync + 'static> MemoryStore<V> {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn insert(&self, key: String, value: V) {
            self.data.insert(key, value);
        }

        pub fn get(&self, key: &str) -> Option<V> {
            self.data.get(key).map(|v| v.value().clone())
        }

        pub fn remove(&self, key: &str) -> Option<V> {
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
            self.data.iter().map(|entry| entry.value().clone()).collect()
        }

        pub fn len(&self) -> usize {
            self.data.len()
        }

        pub fn is_empty(&self) -> bool {
            self.data.is_empty()
        }
    }
}
