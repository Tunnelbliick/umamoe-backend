use dashmap::DashMap;
use serde::Serialize;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Maximum number of cache entries before eviction kicks in
const MAX_CACHE_ENTRIES: usize = 1000;

/// Global cache storage
static CACHE: OnceLock<DashMap<String, CacheEntry>> = OnceLock::new();

/// Cache entry with expiration and access tracking
#[derive(Clone)]
struct CacheEntry {
    data: String,
    expires_at: Instant,
    last_accessed: Instant,
    #[allow(dead_code)]
    size_bytes: usize,
}

fn get_cache() -> &'static DashMap<String, CacheEntry> {
    CACHE.get_or_init(|| DashMap::new())
}

/// Get cached data if it exists and hasn't expired
pub fn get<T: for<'de> serde::Deserialize<'de>>(key: &str) -> Option<T> {
    let cache = get_cache();

    if let Some(mut entry) = cache.get_mut(key) {
        // Check if expired
        if Instant::now() < entry.expires_at {
            // Update last accessed time (for LRU tracking)
            entry.last_accessed = Instant::now();

            // Try to deserialize
            if let Ok(data) = serde_json::from_str(&entry.data) {
                return Some(data);
            }
        } else {
            // Remove expired entry
            drop(entry);
            cache.remove(key);
        }
    }

    None
}

/// Set cached data with TTL (time to live)
pub fn set<T: Serialize>(key: &str, data: &T, ttl: Duration) -> Result<(), serde_json::Error> {
    let cache = get_cache();

    // Evict old entries if cache is too large
    if cache.len() >= MAX_CACHE_ENTRIES {
        evict_lru_entries();
    }

    let json_data = serde_json::to_string(data)?;
    let size_bytes = json_data.len();
    let now = Instant::now();

    let entry = CacheEntry {
        data: json_data,
        expires_at: now + ttl,
        last_accessed: now,
        size_bytes,
    };

    cache.insert(key.to_string(), entry);
    Ok(())
}

/// Evict least recently used entries to free up space
/// Removes 20% of entries (sorted by last_accessed time)
fn evict_lru_entries() {
    let cache = get_cache();
    let current_size = cache.len();
    let target_remove = current_size / 5; // Remove 20%

    if target_remove == 0 {
        return;
    }

    // Collect all entries with their access times
    let mut entries: Vec<(String, Instant)> = cache
        .iter()
        .map(|entry| (entry.key().clone(), entry.value().last_accessed))
        .collect();

    // Sort by last accessed (oldest first)
    entries.sort_by_key(|(_, last_accessed)| *last_accessed);

    // Remove the oldest entries
    for (key, _) in entries.iter().take(target_remove) {
        cache.remove(key);
    }

    tracing::info!(
        "🗑️  Cache eviction: removed {} LRU entries (cache size: {} -> {})",
        target_remove,
        current_size,
        cache.len()
    );
}

/// Clear all expired cache entries
#[allow(dead_code)]
pub fn cleanup_expired() {
    let cache = get_cache();
    let now = Instant::now();

    let before_count = cache.len();
    cache.retain(|_, entry| now < entry.expires_at);
    let removed = before_count - cache.len();

    if removed > 0 {
        tracing::info!("🧹 Cleaned up {} expired cache entries", removed);
    }
}

/// Clear specific cache key
#[allow(dead_code)]
pub fn invalidate(key: &str) {
    let cache = get_cache();
    cache.remove(key);
}

/// Clear all cache
#[allow(dead_code)]
pub fn clear_all() {
    let cache = get_cache();
    cache.clear();
}

/// Get cache statistics
#[allow(dead_code)]
pub fn stats() -> CacheStats {
    let cache = get_cache();
    let now = Instant::now();

    let mut total_size = 0;
    let mut expired_count = 0;

    for entry in cache.iter() {
        total_size += entry.value().size_bytes;
        if now >= entry.value().expires_at {
            expired_count += 1;
        }
    }

    CacheStats {
        entry_count: cache.len(),
        total_size_bytes: total_size,
        expired_count,
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_size_bytes: usize,
    pub expired_count: usize,
}
