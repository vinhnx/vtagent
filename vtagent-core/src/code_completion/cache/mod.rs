use crate::code_completion::engine::CompletionSuggestion;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Completion cache for performance optimization
pub struct CompletionCache {
    cache: HashMap<String, CacheEntry>,
    max_entries: usize,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    suggestions: Vec<CompletionSuggestion>,
    created_at: Instant,
    access_count: usize,
}

impl CompletionCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 1000,
            ttl: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Get cached suggestions for a context
    pub fn get(&mut self, context_key: &str) -> Option<Vec<CompletionSuggestion>> {
        if let Some(entry) = self.cache.get_mut(context_key) {
            if entry.created_at.elapsed() < self.ttl {
                entry.access_count += 1;
                return Some(entry.suggestions.clone());
            } else {
                self.cache.remove(context_key);
            }
        }
        None
    }

    /// Cache suggestions for a context
    pub fn put(&mut self, context_key: String, suggestions: Vec<CompletionSuggestion>) {
        // Clean up expired entries
        self.cleanup_expired();

        // Remove least recently used entries if at capacity
        if self.cache.len() >= self.max_entries {
            self.evict_lru();
        }

        let entry = CacheEntry {
            suggestions,
            created_at: Instant::now(),
            access_count: 1,
        };

        self.cache.insert(context_key, entry);
    }

    fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.cache.retain(|_, entry| now.duration_since(entry.created_at) < self.ttl);
    }

    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.cache
            .iter()
            .min_by_key(|(_, entry)| entry.access_count)
            .map(|(key, _)| key.clone())
        {
            self.cache.remove(&lru_key);
        }
    }
}
