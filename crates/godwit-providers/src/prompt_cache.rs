use dashmap::DashMap;
use std::collections::VecDeque;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Entry stored in the prompt cache with TTL information
#[derive(Clone, Debug)]
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Thread-safe prompt cache with TTL and LRU eviction
/// 
/// Default TTL: 3600 seconds (1 hour)
/// Default max size: 10000 entries
pub struct PromptCache<K, V> {
    /// The actual cache storage
    cache: DashMap<K, CacheEntry<V>>,
    /// LRU access order tracking - stores keys in order of access
    lru_order: Arc<Mutex<VecDeque<K>>>,
    /// Maximum number of entries before eviction
    max_size: usize,
    /// Default time-to-live for entries
    default_ttl: Duration,
}

impl<K: Eq + Hash + Clone + Send + Sync + 'static, V: Clone + Send + Sync + 'static> PromptCache<K, V> {
    /// Create a new prompt cache with default settings
    /// - Max size: 10000 entries
    /// - Default TTL: 3600 seconds (1 hour)
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            lru_order: Arc::new(Mutex::new(VecDeque::new())),
            max_size: 10000,
            default_ttl: Duration::from_secs(3600),
        }
    }

    /// Create a new prompt cache with custom settings
    pub fn with_config(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            lru_order: Arc::new(Mutex::new(VecDeque::new())),
            max_size,
            default_ttl,
        }
    }

    /// Insert a value into the cache with the default TTL
    pub fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl);
    }

    /// Insert a value into the cache with a custom TTL
    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let expires_at = Instant::now() + ttl;
        
        // Check if we need to evict before inserting
        if !self.cache.contains_key(&key) && self.cache.len() >= self.max_size {
            self.evict_lru();
        }
        
        // Remove old entry if it exists (to update LRU order)
        self.cache.remove(&key);
        
        // Insert new entry
        self.cache.insert(key.clone(), CacheEntry { value, expires_at });
        
        // Update LRU order
        self.update_lru_access(key);
    }

    /// Get a value from the cache
    /// Returns None if the key doesn't exist or has expired
    pub fn get(&self, key: &K) -> Option<V> {
        let entry = self.cache.get(key)?;
        
        // Check if entry has expired
        if entry.expires_at < Instant::now() {
            // Remove expired entry
            drop(entry);
            self.cache.remove(key);
            return None;
        }
        
        // Update LRU access order
        self.update_lru_access(key.clone());
        
        Some(entry.value.clone())
    }

    /// Check if a key exists in the cache (without updating LRU order)
    pub fn contains_key(&self, key: &K) -> bool {
        if let Some(entry) = self.cache.get(key) {
            if entry.expires_at >= Instant::now() {
                true
            } else {
                // Remove expired entry
                drop(entry);
                self.cache.remove(key);
                false
            }
        } else {
            false
        }
    }

    /// Remove a key from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        self.cache.remove(key).map(|(_, entry)| entry.value)
    }

    /// Get the current number of entries in the cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear all entries from the cache
    pub fn clear(&self) {
        self.cache.clear();
        let mut lru = self.lru_order.lock().unwrap();
        lru.clear();
    }

    /// Evict expired entries from the cache
    pub fn evict_expired(&self) -> usize {
        let now = Instant::now();
        let mut evicted = 0;
        
        // Collect keys to evict (can't modify while iterating)
        let keys_to_evict: Vec<K> = self.cache
            .iter()
            .filter(|entry| entry.value().expires_at < now)
            .map(|entry| entry.key().clone())
            .collect();
        
        for key in keys_to_evict {
            self.cache.remove(&key);
            evicted += 1;
        }
        
        evicted
    }

    /// Evict the least recently used entry
    fn evict_lru(&self) {
        let mut lru = self.lru_order.lock().unwrap();
        while let Some(key) = lru.pop_front() {
            // Only remove if it still exists in cache (might have been removed already)
            if self.cache.contains_key(&key) {
                self.cache.remove(&key);
                break;
            }
        }
    }

    /// Update LRU access order for a key
    fn update_lru_access(&self, key: K) {
        // Remove key from its current position if it exists
        {
            let mut lru = self.lru_order.lock().unwrap();
            if let Some(pos) = lru.iter().position(|k| k == &key) {
                lru.remove(pos);
            }
            lru.push_back(key);
        }
    }
}

impl<K: Eq + Hash + Clone + Send + Sync + 'static, V: Clone + Send + Sync + 'static> Default for PromptCache<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: DashMap is already Send + Sync, and our Mutex-protected fields are too
unsafe impl<K, V> Send for PromptCache<K, V> where K: Eq + Hash + Clone + Send, V: Clone + Send {}
unsafe impl<K, V> Sync for PromptCache<K, V> where K: Eq + Hash + Clone + Send + Sync, V: Clone + Sync {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cache_insert_and_get() {
        let cache = PromptCache::new();
        cache.insert("key1".to_string(), "value1".to_string());
        
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_hit_on_same_messages() {
        let cache = PromptCache::new();
        let messages = vec!["Hello".to_string(), "World".to_string()];
        let response = "Hi there!".to_string();
        
        // First request - cache miss
        assert_eq!(cache.get(&messages), None);
        
        // Store response
        cache.insert(messages.clone(), response.clone());
        
        // Second request with same messages - cache hit
        assert_eq!(cache.get(&messages), Some(response));
    }

    #[test]
    fn test_cache_ttl_expiration() {
        let cache = PromptCache::with_config(10000, Duration::from_millis(100));
        cache.insert("key1".to_string(), "value1".to_string());
        
        // Should exist immediately
        assert!(cache.contains_key(&"key1".to_string()));
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        
        // Wait for TTL to expire
        thread::sleep(Duration::from_millis(150));
        
        // Should be expired now
        assert!(!cache.contains_key(&"key1".to_string()));
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = PromptCache::with_config(3, Duration::from_secs(3600));
        
        // Insert 3 entries
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        cache.insert("key3".to_string(), "value3".to_string());
        
        assert_eq!(cache.len(), 3);
        
        // Access key1 to make it recently used
        cache.get(&"key1".to_string());
        
        // Insert 4th entry - should evict key2 (least recently used)
        cache.insert("key4".to_string(), "value4".to_string());
        
        assert_eq!(cache.len(), 3);
        assert!(cache.contains_key(&"key1".to_string()));
        assert!(!cache.contains_key(&"key2".to_string()));
        assert!(cache.contains_key(&"key3".to_string()));
        assert!(cache.contains_key(&"key4".to_string()));
    }

    #[test]
    fn test_cache_remove() {
        let cache = PromptCache::new();
        cache.insert("key1".to_string(), "value1".to_string());
        
        assert_eq!(cache.len(), 1);
        
        let removed = cache.remove(&"key1".to_string());
        assert_eq!(removed, Some("value1".to_string()));
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_clear() {
        let cache = PromptCache::new();
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        
        assert_eq!(cache.len(), 2);
        
        cache.clear();
        
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_evict_expired() {
        let cache = PromptCache::with_config(10000, Duration::from_millis(50));
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        
        // Wait for expiration
        thread::sleep(Duration::from_millis(100));
        
        let evicted = cache.evict_expired();
        assert_eq!(evicted, 2);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_custom_ttl() {
        let cache = PromptCache::with_config(10000, Duration::from_secs(3600));
        
        // Insert with short TTL
        cache.insert_with_ttl("key1".to_string(), "value1".to_string(), Duration::from_millis(50));
        
        assert!(cache.contains_key(&"key1".to_string()));
        
        thread::sleep(Duration::from_millis(100));
        
        assert!(!cache.contains_key(&"key1".to_string()));
    }

    #[test]
    fn test_lru_order_updates_on_access() {
        let cache = PromptCache::with_config(3, Duration::from_secs(3600));
        
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        cache.insert("key3".to_string(), "value3".to_string());
        
        // Access key1 to move it to back of LRU queue
        cache.get(&"key1".to_string());
        
        // Insert new entry - should evict key2 (now least recently used)
        cache.insert("key4".to_string(), "value4".to_string());
        
        assert!(!cache.contains_key(&"key2".to_string()));
        assert!(cache.contains_key(&"key1".to_string()));
    }
}
