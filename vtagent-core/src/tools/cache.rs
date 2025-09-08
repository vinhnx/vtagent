//! Caching system for tool results

use super::types::{EnhancedCacheEntry, EnhancedCacheStats};
use dashmap::DashMap;
use lru::LruCache;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Global file cache instance
pub static FILE_CACHE: Lazy<FileCache> = Lazy::new(|| FileCache::new(1000));

/// Enhanced file cache with LRU eviction and performance monitoring
pub struct FileCache {
    file_cache: Arc<RwLock<LruCache<String, EnhancedCacheEntry<Value>>>>,
    directory_cache: Arc<RwLock<LruCache<String, EnhancedCacheEntry<Value>>>>,
    stats: Arc<RwLock<EnhancedCacheStats>>,
    max_size_bytes: usize,
    ttl: Duration,
}

impl FileCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            file_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap(),
            ))),
            directory_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(capacity / 2).unwrap(),
            ))),
            stats: Arc::new(RwLock::new(EnhancedCacheStats::default())),
            max_size_bytes: 50 * 1024 * 1024, // 50MB default
            ttl: Duration::from_secs(300),    // 5 minutes default
        }
    }

    /// Get cached file content
    pub async fn get_file(&self, key: &str) -> Option<Value> {
        let mut cache = self.file_cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.get_mut(key) {
            // Check if entry is still valid
            if entry.timestamp.elapsed() < self.ttl {
                entry.access();
                stats.hits += 1;
                return Some(entry.data.clone());
            } else {
                // Entry expired, remove it
                cache.pop(key);
                stats.expired_evictions += 1;
            }
        }

        stats.misses += 1;
        None
    }

    /// Cache file content
    pub async fn put_file(&self, key: String, value: Value) {
        let size_bytes = serde_json::to_string(&value).unwrap_or_default().len();
        let entry = EnhancedCacheEntry::new(value, size_bytes);

        let mut cache = self.file_cache.write().await;
        let mut stats = self.stats.write().await;

        // Check memory limits
        if stats.total_size_bytes + size_bytes > self.max_size_bytes {
            // Evict oldest entries until we have space
            while stats.total_size_bytes + size_bytes > self.max_size_bytes && !cache.is_empty() {
                if let Some((_, old_entry)) = cache.pop_lru() {
                    stats.total_size_bytes =
                        stats.total_size_bytes.saturating_sub(old_entry.size_bytes);
                    stats.memory_evictions += 1;
                }
            }
        }

        cache.put(key, entry);
        stats.entries = cache.len();
        stats.total_size_bytes += size_bytes;
    }

    /// Get cached directory listing
    pub async fn get_directory(&self, key: &str) -> Option<Value> {
        let mut cache = self.directory_cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.get_mut(key) {
            if entry.timestamp.elapsed() < self.ttl {
                entry.access();
                stats.hits += 1;
                return Some(entry.data.clone());
            } else {
                cache.pop(key);
                stats.expired_evictions += 1;
            }
        }

        stats.misses += 1;
        None
    }

    /// Cache directory listing
    pub async fn put_directory(&self, key: String, value: Value) {
        let size_bytes = serde_json::to_string(&value).unwrap_or_default().len();
        let entry = EnhancedCacheEntry::new(value, size_bytes);

        let mut cache = self.directory_cache.write().await;
        let mut stats = self.stats.write().await;

        cache.put(key, entry);
        stats.entries += cache.len();
        stats.total_size_bytes += size_bytes;
    }

    /// Get cache statistics
    pub async fn stats(&self) -> EnhancedCacheStats {
        self.stats.read().await.clone()
    }

    /// Clear all caches
    pub async fn clear(&self) {
        self.file_cache.write().await.clear();
        self.directory_cache.write().await.clear();
        *self.stats.write().await = EnhancedCacheStats::default();
    }
}
