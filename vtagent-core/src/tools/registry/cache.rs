use serde_json::json;

use crate::tools::cache::FILE_CACHE;

use super::ToolRegistry;

impl ToolRegistry {
    pub async fn cache_stats(&self) -> serde_json::Value {
        let stats = FILE_CACHE.stats().await;
        json!({
            "hits": stats.hits,
            "misses": stats.misses,
            "entries": stats.entries,
            "total_size_bytes": stats.total_size_bytes,
            "hit_rate": if stats.hits + stats.misses > 0 {
                stats.hits as f64 / (stats.hits + stats.misses) as f64
            } else { 0.0 }
        })
    }

    pub async fn clear_cache(&self) {
        FILE_CACHE.clear().await;
    }
}
