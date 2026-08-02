use dashmap::DashMap;
use std::{fmt::Debug, hash::Hash, sync::Arc};

#[derive(Clone)]
pub struct MemoryCache<K, V> {
    inner: Arc<DashMap<K, V>>,
}

impl<K: Eq + Hash + Debug, V: Clone + Send + Sync> MemoryCache<K, V> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    pub async fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value);
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        self.inner.get(key).map(|entry| entry.clone())
    }

    pub async fn invalidate(&self, key: &K) {
        self.inner.remove(key);
    }
}

impl<K: Eq + Hash + Debug, V: Clone + Send + Sync> Default for MemoryCache<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_stores_and_retrieves() {
        let cache = MemoryCache::new();
        cache.insert("key".to_string(), "value".to_string()).await;
        assert_eq!(
            cache.get(&"key".to_string()).await,
            Some("value".to_string())
        );
    }
}
