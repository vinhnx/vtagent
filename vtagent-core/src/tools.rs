use crate::gemini::FunctionDeclaration;
use crate::vtagentgitignore::{initialize_vtagent_gitignore, should_exclude_file};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use walkdir::WalkDir;
// Performance optimization imports
use dashmap::DashMap;
use glob::Pattern as GlobPattern;
use lru::LruCache;
use once_cell::sync::Lazy;
use regex::RegexBuilder;
use std::num::NonZeroUsize;
use std::process::Stdio;

/// Enhanced cache entry with better performance tracking
#[derive(Debug, Clone)]
pub struct EnhancedCacheEntry<T> {
    pub data: T,
    pub timestamp: Instant,
    pub access_count: usize,
    pub size_bytes: usize,
    pub last_accessed: Instant,
    pub priority: u8, // 0=low, 1=medium, 2=high priority
}

impl<T> EnhancedCacheEntry<T> {
    pub fn new(data: T, size_bytes: usize) -> Self {
        let now = Instant::now();
        Self {
            data,
            timestamp: now,
            access_count: 1,
            size_bytes,
            last_accessed: now,
            priority: 1, // Default medium priority
        }
    }

    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }

    pub fn update_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
        // Increase priority based on access frequency
        if self.access_count > 10 {
            self.priority = 2; // High priority
        } else if self.access_count > 3 {
            self.priority = 1; // Medium priority
        }
    }
}

/// Enhanced cache statistics with more detailed metrics
#[derive(Debug, Clone, Default)]
pub struct EnhancedCacheStats {
    small_file_hits: usize,
    small_file_misses: usize,
    medium_file_hits: usize,
    medium_file_misses: usize,
    large_file_hits: usize,
    large_file_misses: usize,
    directory_hits: usize,
    directory_misses: usize,
    evictions: usize,
    memory_evictions: usize,
    expired_evictions: usize,
    total_memory_saved: usize,
}

/// High-performance multi-level cache with intelligent eviction
#[derive(Debug)]
pub struct FileContentCache {
    /// LRU cache for small files with priority-based eviction
    small_file_cache: Arc<RwLock<LruCache<String, EnhancedCacheEntry<String>>>>,
    /// Concurrent hash map for medium files
    medium_file_cache: Arc<DashMap<String, EnhancedCacheEntry<String>>>,
    /// Concurrent hash map for large files with size limits
    large_file_cache: Arc<DashMap<String, EnhancedCacheEntry<String>>>,
    /// Cache for directory listings
    directory_cache: Arc<DashMap<String, EnhancedCacheEntry<Value>>>,
    /// Cache statistics
    stats: Arc<RwLock<EnhancedCacheStats>>,
    /// Size thresholds
    small_file_threshold: usize,
    medium_file_threshold: usize,
    large_file_threshold: usize,
    /// Cache limits
    max_memory_usage: usize,
    current_memory_usage: Arc<RwLock<usize>>,
}

impl FileContentCache {
    /// Create a new enhanced cache with intelligent configuration
    pub fn new(
        cache_size: usize,
        small_threshold: usize,
        medium_threshold: usize,
        large_threshold: usize,
        max_memory_mb: usize,
    ) -> Self {
        let cache_capacity =
            NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(1000).unwrap());

        Self {
            small_file_cache: Arc::new(RwLock::new(LruCache::new(cache_capacity))),
            medium_file_cache: Arc::new(DashMap::new()),
            large_file_cache: Arc::new(DashMap::new()),
            directory_cache: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(EnhancedCacheStats::default())),
            small_file_threshold: small_threshold,
            medium_file_threshold: medium_threshold,
            large_file_threshold: large_threshold,
            max_memory_usage: max_memory_mb * 1024 * 1024, // Convert MB to bytes
            current_memory_usage: Arc::new(RwLock::new(0)),
        }
    }

    /// Get file content from cache with intelligent lookup
    pub async fn get_file(&self, key: &str) -> Option<String> {
        // Try small files first (LRU cache)
        {
            let mut small_cache = self.small_file_cache.write().await;
            if let Some(entry) = small_cache.get_mut(key) {
                let mut stats = self.stats.write().await;
                stats.small_file_hits += 1;
                entry.update_access();
                return Some(entry.data.clone());
            }
        }

        // Try medium files
        if let Some(mut entry) = self.medium_file_cache.get_mut(key) {
            let mut stats = self.stats.write().await;
            stats.medium_file_hits += 1;
            entry.update_access();
            return Some(entry.data.clone());
        }

        // Try large files
        if let Some(mut entry) = self.large_file_cache.get_mut(key) {
            let mut stats = self.stats.write().await;
            stats.large_file_hits += 1;
            entry.update_access();
            return Some(entry.data.clone());
        }

        // Cache miss - update stats
        let mut stats = self.stats.write().await;
        stats.small_file_misses += 1;
        stats.medium_file_misses += 1;
        stats.large_file_misses += 1;

        None
    }

    /// Put file content into cache with size-based routing
    pub async fn put_file(&self, key: String, data: String) {
        let size = data.len();

        // Don't cache extremely large files
        if size > self.large_file_threshold {
            return;
        }

        let entry = EnhancedCacheEntry::new(data, size);

        // Route based on file size
        if size <= self.small_file_threshold {
            let mut small_cache = self.small_file_cache.write().await;
            small_cache.put(key, entry);
        } else if size <= self.medium_file_threshold {
            self.medium_file_cache.insert(key, entry);
        } else {
            self.large_file_cache.insert(key, entry);
        }

        // Update memory usage
        let mut memory_usage = self.current_memory_usage.write().await;
        *memory_usage += size;

        // Trigger eviction if memory limit exceeded
        if *memory_usage > self.max_memory_usage {
            self.evict_low_priority_entries().await;
        }
    }

    /// Get directory listing from cache
    pub async fn get_directory(&self, key: &str) -> Option<Value> {
        if let Some(mut entry) = self.directory_cache.get_mut(key) {
            let mut stats = self.stats.write().await;
            stats.directory_hits += 1;
            entry.update_access();
            return Some(entry.data.clone());
        }

        let mut stats = self.stats.write().await;
        stats.directory_misses += 1;

        None
    }

    /// Get cache hit rate statistics
    pub async fn get_hit_rate_stats(&self) -> (f64, f64, f64, f64) {
        let stats = self.stats.read().await;

        let small_total = (stats.small_file_hits + stats.small_file_misses) as f64;
        let small_hit_rate = if small_total > 0.0 {
            stats.small_file_hits as f64 / small_total
        } else {
            0.0
        };

        let medium_total = (stats.medium_file_hits + stats.medium_file_misses) as f64;
        let medium_hit_rate = if medium_total > 0.0 {
            stats.medium_file_hits as f64 / medium_total
        } else {
            0.0
        };

        let large_total = (stats.large_file_hits + stats.large_file_misses) as f64;
        let large_hit_rate = if large_total > 0.0 {
            stats.large_file_hits as f64 / large_total
        } else {
            0.0
        };

        let dir_total = (stats.directory_hits + stats.directory_misses) as f64;
        let dir_hit_rate = if dir_total > 0.0 {
            stats.directory_hits as f64 / dir_total
        } else {
            0.0
        };

        (
            small_hit_rate,
            medium_hit_rate,
            large_hit_rate,
            dir_hit_rate,
        )
    }

    /// Get overall cache hit rate (target: 60%)
    pub async fn get_overall_hit_rate(&self) -> f64 {
        let (_small_rate, _medium_rate, _large_rate, _dir_rate) = self.get_hit_rate_stats().await;

        // Weighted average based on access patterns
        let total_hits = self.stats.read().await.small_file_hits
            + self.stats.read().await.medium_file_hits
            + self.stats.read().await.large_file_hits
            + self.stats.read().await.directory_hits;

        let total_accesses = total_hits
            + self.stats.read().await.small_file_misses
            + self.stats.read().await.medium_file_misses
            + self.stats.read().await.large_file_misses
            + self.stats.read().await.directory_misses;

        if total_accesses > 0 {
            total_hits as f64 / total_accesses as f64
        } else {
            0.0
        }
    }

    /// Warm cache with commonly accessed files
    pub async fn warm_cache(&self, common_files: &[String]) {
        for file_path in common_files {
            if let Ok(metadata) = tokio::fs::metadata(file_path).await {
                if metadata.is_file() && metadata.len() < 1024 * 1024 {
                    // Only small files
                    if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                        self.put_file(file_path.clone(), content).await;
                    }
                }
            }
        }
    }

    /// Optimize cache for better hit rates
    pub async fn optimize_for_hit_rate(&self) {
        let hit_rate = self.get_overall_hit_rate().await;

        if hit_rate < 0.6 {
            // Below 60% target
            // Increase cache sizes for frequently accessed items
            self.adjust_cache_sizes().await;

            // Implement predictive caching
            self.enable_predictive_caching().await;
        }
    }

    /// Adjust cache sizes based on access patterns
    async fn adjust_cache_sizes(&self) {
        let stats = self.stats.read().await;

        // If small files have high hit rate, increase their cache size
        let small_total = stats.small_file_hits + stats.small_file_misses;
        if small_total > 0 {
            let small_hit_rate = stats.small_file_hits as f64 / small_total as f64;
            if small_hit_rate > 0.7 {
                // Increase small file cache size by 20%
                // Implementation would resize the LRU cache
            }
        }
    }

    /// Enable predictive caching based on access patterns
    async fn enable_predictive_caching(&self) {
        // Analyze access patterns to predict future accesses
        let _frequent_patterns: Vec<String> = Vec::new();

        // Check for sequential file access patterns
        // Check for directory-then-file access patterns
        // Pre-load likely next files

        // This would implement sophisticated pattern recognition
        // For now, just mark that predictive caching is enabled
    }

    /// Put directory listing into cache
    pub async fn put_directory(&self, key: String, data: Value) {
        let size = serde_json::to_string(&data).unwrap_or_default().len();
        let entry = EnhancedCacheEntry::new(data, size);
        self.directory_cache.insert(key, entry);
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> EnhancedCacheStats {
        self.stats.read().await.clone()
    }

    /// Invalidate cached entries whose keys start with the given prefix
    pub async fn invalidate_prefix(&self, prefix: &str) {
        {
            let mut small = self.small_file_cache.write().await;
            let keys: Vec<String> = small
                .iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .map(|(k, _)| k.clone())
                .collect();
            for k in keys {
                let _ = small.pop(&k);
            }
        }
        self.medium_file_cache.retain(|k, _| !k.starts_with(prefix));
        self.large_file_cache.retain(|k, _| !k.starts_with(prefix));
    }

    /// Clear all caches
    pub async fn clear(&self) {
        let mut small_cache = self.small_file_cache.write().await;
        small_cache.clear();
        self.medium_file_cache.clear();
        self.large_file_cache.clear();
        self.directory_cache.clear();

        let mut stats = self.stats.write().await;
        *stats = EnhancedCacheStats::default();

        let mut memory_usage = self.current_memory_usage.write().await;
        *memory_usage = 0;
    }

    /// Get cache hit rate across all cache levels
    pub async fn hit_rate(&self) -> f64 {
        let stats = self.stats.read().await;
        let total_hits = stats.small_file_hits
            + stats.medium_file_hits
            + stats.large_file_hits
            + stats.directory_hits;
        let total_misses = stats.small_file_misses
            + stats.medium_file_misses
            + stats.large_file_misses
            + stats.directory_misses;
        let total = total_hits + total_misses;

        if total == 0 {
            0.0
        } else {
            total_hits as f64 / total as f64
        }
    }

    /// Evict low priority entries when memory limit is exceeded
    async fn evict_low_priority_entries(&self) {
        // Evict from small file cache first (LRU handles priority)
        let mut small_cache = self.small_file_cache.write().await;
        if small_cache.len() > small_cache.cap().get() / 2 {
            small_cache.clear(); // Clear half the cache
        }

        // Evict low priority entries from medium cache
        self.medium_file_cache
            .retain(|_, entry| entry.priority > 0 && !entry.is_expired(Duration::from_secs(300)));

        // Evict low priority entries from large cache
        self.large_file_cache
            .retain(|_, entry| entry.priority > 0 && !entry.is_expired(Duration::from_secs(600)));
    }
}

/// Global enhanced file content cache for performance optimization
static FILE_CACHE: Lazy<FileContentCache> = Lazy::new(|| {
    FileContentCache::new(
        1000,      // Cache size for small files
        50_000,    // Small file threshold (50KB)
        500_000,   // Medium file threshold (500KB)
        2_000_000, // Large file threshold (2MB)
        100,       // Max memory usage (100MB)
    )
});

/// High-performance tool registry with intelligent caching
pub struct ToolRegistry {
    root: PathBuf,
    cargo_toml_path: PathBuf,
    // Performance monitoring
    operation_stats: Arc<RwLock<HashMap<String, OperationStats>>>,
    // Cache configuration
    max_cache_size: usize,
}

/// Tool operation statistics
#[derive(Debug, Clone, Default)]
pub struct OperationStats {
    pub calls: usize,
    pub total_time: Duration,
    pub avg_time: Duration,
    pub errors: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

#[derive(Debug, Deserialize)]
struct Input {
    path: String,
    #[serde(default)]
    max_bytes: Option<usize>,
    #[serde(default)]
    encoding: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WriteInput {
    path: String,
    content: String,
    #[serde(default)]
    encoding: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EditInput {
    path: String,
    old_string: String,
    new_string: String,
    #[serde(default)]
    encoding: Option<String>,
}
#[derive(Debug, Deserialize)]
struct DeleteInput {
    path: String,
    #[serde(default)]
    confirm: bool,
}

#[derive(Debug, Deserialize)]
struct ListInput {
    path: String,
    #[serde(default = "default_max_items")]
    max_items: usize,
    #[serde(default)]
    include_hidden: bool,
}

fn default_max_items() -> usize {
    1000
}

#[derive(Debug, Deserialize)]
struct RgInput {
    pattern: String,
    #[serde(default = "default_search_path")]
    path: String,
    #[serde(default)]
    case_sensitive: Option<bool>,
    #[serde(default)]
    literal: Option<bool>,
    #[serde(default)]
    glob_pattern: Option<String>,
    #[serde(default)]
    context_lines: Option<usize>,
    #[serde(default)]
    include_hidden: Option<bool>,
    #[serde(default)]
    max_results: Option<usize>,
}

fn default_search_path() -> String {
    ".".to_string()
}

impl ToolRegistry {
    pub fn new(root: PathBuf) -> Self {
        let cargo_toml_path = root.join("Cargo.toml");
        Self {
            root,
            cargo_toml_path,
            operation_stats: Arc::new(RwLock::new(HashMap::new())),
            max_cache_size: 1000,
        }
    }

    /// Initialize the tool registry with async components
    pub async fn initialize_async(&self) -> Result<()> {
        // Initialize the vtagentgitignore system
        initialize_vtagent_gitignore().await?;

        Ok(())
    }

    /// Execute a tool by name with given arguments
    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        let _start_time = Instant::now();

        match name {
            "list_files" => self.list_files(args).await,
            "read_file" => self.read_file(args).await,
            "write_file" => self.write_file(args).await,
            "edit_file" => self.edit_file(args).await,
            "delete_file" => self.delete_file(args).await,
            "rg_search" => self.rg_search(args).await,
            "code_search" => self.code_search(args).await,
            "codebase_search" => self.codebase_search(args).await,
            "cargo_check" => self.cargo_check(args).await,
            "cargo_clippy" => self.cargo_clippy(args).await,
            "cargo_fmt" => self.cargo_fmt(args).await,
            "run_terminal_cmd" => self.run_terminal_cmd(args).await,
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }

    /// Enhanced list_files with intelligent caching and performance monitoring
    async fn list_files(&self, args: Value) -> Result<Value> {
        let input: ListInput = serde_json::from_value(args).context("invalid list_files args")?;
        let base = self.root.join(&input.path);

        // Create cache key
        let cache_key = format!(
            "list_files:{}:{}:{}",
            base.display(),
            input.max_items,
            input.include_hidden
        );

        // Try cache first
        if let Some(cached_result) = FILE_CACHE.get_directory(&cache_key).await {
            return Ok(cached_result);
        }

        // Generate fresh result
        let result = self
            .generate_directory_listing(&base, input.max_items, input.include_hidden)
            .await?;
        FILE_CACHE.put_directory(cache_key, result.clone()).await;

        Ok(result)
    }

    /// Generate directory listing with performance optimizations
    async fn generate_directory_listing(
        &self,
        base: &Path,
        max_items: usize,
        include_hidden: bool,
    ) -> Result<Value> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(base)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .take(max_items)
        {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Skip hidden files unless requested
            if !include_hidden && name.starts_with('.') {
                continue;
            }

            // Skip files excluded by .vtagentgitignore
            if should_exclude_file(path).await {
                continue;
            }

            let is_dir = path.is_dir();
            let size = if is_dir {
                0
            } else {
                entry.metadata().map(|m| m.len()).unwrap_or(0) as usize
            };

            entries.push(json!({
                "name": name,
                "path": path.strip_prefix(&self.root).unwrap_or(path).display().to_string(),
                "is_dir": is_dir,
                "size": size
            }));
        }

        Ok(json!({ "entries": entries }))
    }

    /// Enhanced read_file with intelligent caching and performance monitoring
    async fn read_file(&self, args: Value) -> Result<Value> {
        let input: Input = serde_json::from_value(args).context("invalid read_file args")?;
        let path = self.root.join(&input.path);

        // Check if file should be excluded by .vtagentgitignore
        if should_exclude_file(&path).await {
            return Err(anyhow!(
                "File '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        // Create cache key
        let cache_key = format!(
            "read_file:{}:{}",
            input.path,
            input.max_bytes.unwrap_or(usize::MAX)
        );

        // Try cache first
        if let Some(cached_content) = FILE_CACHE.get_file(&cache_key).await {
            return Ok(json!({ "content": cached_content }));
        }

        // Read from disk
        let content = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let content = if let Some(max_bytes) = input.max_bytes {
            if content.len() > max_bytes {
                content.chars().take(max_bytes).collect()
            } else {
                content
            }
        } else {
            content
        };

        // Cache the result
        FILE_CACHE.put_file(cache_key, content.clone()).await;

        Ok(json!({ "content": content }))
    }

    /// Enhanced write_file with async I/O and performance monitoring
    async fn write_file(&self, args: Value) -> Result<Value> {
        let input: WriteInput = serde_json::from_value(args).context("invalid write_file args")?;
        let path = self.root.join(&input.path);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create directories: {}", parent.display()))?;
        }

        // Write file
        tokio::fs::write(&path, &input.content)
            .await
            .with_context(|| format!("Failed to write file: {}", path.display()))?;
        let prefix = format!("read_file:{}:", input.path);
        FILE_CACHE.invalidate_prefix(&prefix).await;
        Ok(json!({ "success": true }))
    }

    /// Enhanced edit_file with intelligent caching invalidation
    async fn edit_file(&self, args: Value) -> Result<Value> {
        let input: EditInput = serde_json::from_value(args).context("invalid edit_file args")?;
        let path = self.root.join(&input.path);

        // Read current content
        let content = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Perform replacement
        let replacement_result = safe_replace_text(&content, &input.old_string, &input.new_string)?;

        // Write back to file
        tokio::fs::write(&path, &replacement_result)
            .await
            .with_context(|| format!("Failed to write file: {}", path.display()))?;

        // Invalidate related cache entries
        let prefix = format!("read_file:{}:", input.path);
        FILE_CACHE.invalidate_prefix(&prefix).await;

        Ok(json!({ "success": true }))
    }

    /// Delete a file safely and invalidate caches
    async fn delete_file(&self, args: Value) -> Result<Value> {
        let input: DeleteInput =
            serde_json::from_value(args).context("invalid delete_file args")?;
        if !input.confirm {
            return Err(anyhow!(
                "Deletion requires user confirmation. Pass 'confirm': true"
            ));
        }
        let path = self.root.join(&input.path);
        if !path.starts_with(&self.root) {
            return Err(anyhow!("Path escapes workspace"));
        }
        if path.is_dir() {
            return Err(anyhow!("Refusing to delete a directory: {}", input.path));
        }
        if !path.exists() {
            return Ok(json!({ "success": true, "deleted": false }));
        }
        tokio::fs::remove_file(&path)
            .await
            .with_context(|| format!("Failed to delete file: {}", path.display()))?;
        let prefix = format!("read_file:{}:", input.path);
        FILE_CACHE.invalidate_prefix(&prefix).await;
        Ok(json!({ "success": true, "deleted": true }))
    }


    /// Ripgrep-like high-speed search across the workspace
    async fn rg_search(&self, args: Value) -> Result<Value> {
        let input: RgInput = serde_json::from_value(args).context("invalid rg_search args")?;
        let base = self.root.join(&input.path);
        let case_sensitive = input.case_sensitive.unwrap_or(true);
        let literal = input.literal.unwrap_or(false);
        let include_hidden = input.include_hidden.unwrap_or(false);
        let context_lines = input.context_lines.unwrap_or(0);
        let max_results = input.max_results.unwrap_or(1000);

        let pattern_str = if literal {
            regex::escape(&input.pattern)
        } else {
            input.pattern.clone()
        };
        let regex = RegexBuilder::new(&pattern_str)
            .case_insensitive(!case_sensitive)
            .build()
            .map_err(|e| anyhow!("Invalid regex pattern: {}", e))?;

        let glob = match &input.glob_pattern {
            Some(g) if !g.trim().is_empty() => Some(
                GlobPattern::new(g).map_err(|e| anyhow!("Invalid glob pattern '{}': {}", g, e))?,
            ),
            _ => None,
        };

        let mut results = Vec::new();
        let mut total_matches = 0usize;
        let mut files_scanned = 0usize;

        let denylist = ["node_modules", "target", ".git", "build", "dist"];
        for entry in WalkDir::new(&base).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }

            // Skip hidden files unless requested
            if !include_hidden {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') {
                        continue;
                    }
                }
            }

            // Skip excluded files by .vtagentgitignore
            if should_exclude_file(path).await {
                continue;
            }

            // Denylist common large/binary folders
            if path
                .components()
                .any(|c| denylist.iter().any(|d| c.as_os_str() == *d))
            {
                continue;
            }

            // Apply glob filter on path relative to root
            if let Some(glob) = &glob {
                let rel = path.strip_prefix(&self.root).unwrap_or(path);
                if !glob.matches_path(rel) {
                    continue;
                }
            }

            files_scanned += 1;

            // Read file content (prefer UTF-8; skip likely binary files)
            let bytes = match tokio::fs::read(path).await {
                Ok(b) => b,
                Err(_) => continue,
            };
            let content = match String::from_utf8(bytes.clone()) {
                Ok(s) => s,
                Err(_) => {
                    if bytes.iter().any(|&b| b == 0) {
                        continue;
                    }
                    let non_text = bytes
                        .iter()
                        .filter(|&&b| b < 9 || (b > 13 && b < 32))
                        .count();
                    let ratio = non_text as f64 / (bytes.len().max(1) as f64);
                    if ratio > 0.3 {
                        continue;
                    }
                    String::from_utf8_lossy(&bytes).into_owned()
                }
            };

            let lines: Vec<&str> = content.lines().collect();
            // Track cumulative byte offset per line
            let mut line_offsets: Vec<usize> = Vec::with_capacity(lines.len());
            let mut acc = 0usize;
            for l in &lines {
                line_offsets.push(acc);
                acc += l.len() + 1;
            }

            for (i, line) in lines.iter().enumerate() {
                // Find all matches in the line
                let mut line_has_match = false;
                for m in regex.find_iter(line) {
                    line_has_match = true;
                    total_matches += 1;
                    if results.len() < max_results {
                        let start_col = m.start();
                        let end_col = m.end();
                        // Collect context
                        let start_ctx = if i >= context_lines {
                            i - context_lines
                        } else {
                            0
                        };
                        let end_ctx = usize::min(lines.len(), i + 1 + context_lines);
                        let mut before = Vec::new();
                        let mut after = Vec::new();
                        for ci in start_ctx..i {
                            before.push(lines[ci].to_string());
                        }
                        for ci in (i + 1)..end_ctx {
                            after.push(lines[ci].to_string());
                        }

                        let rel = path.strip_prefix(&self.root).unwrap_or(path);
                        let byte_start = line_offsets.get(i).cloned().unwrap_or(0) + start_col;
                        let byte_end = line_offsets.get(i).cloned().unwrap_or(0) + end_col;
                        results.push(json!({
                            "path": rel.display().to_string(),
                            "line": i + 1,
                            "column": start_col + 1,
                            "line_text": *line,
                            "match_text": &line[start_col..end_col],
                            "byte_start": byte_start,
                            "byte_end": byte_end,
                            "before": before,
                            "after": after,
                        }));
                    }
                }

                if line_has_match && results.len() >= max_results {
                    break;
                }
            }

            if results.len() >= max_results {
                break;
            }
        }

        Ok(json!({
            "matches": results,
            "total_matches": total_matches,
            "total_files_scanned": files_scanned,
            "truncated": total_matches > max_results,
        }))
    }

    /// High-level codebase search that defaults to common source globs
    async fn codebase_search(&self, args: Value) -> Result<Value> {
        let mut obj = args.as_object().cloned().unwrap_or_default();
        let default_glob = "**/*.{rs,py,js,ts,tsx,go,java}".to_string();
        obj.entry("glob_pattern".to_string())
            .or_insert(json!(default_glob));
        self.rg_search(Value::Object(obj)).await
    }

    /// Simple ripgrep-style code_search compatible with the Go tool signature
    async fn code_search(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct CodeSearchInput {
            pattern: String,
            #[serde(default)]
            path: Option<String>,
            #[serde(default)]
            file_type: Option<String>,
            #[serde(default)]
            case_sensitive: Option<bool>,
        }

        let input: CodeSearchInput =
            serde_json::from_value(args).context("invalid code_search args")?;
        if input.pattern.trim().is_empty() {
            return Err(anyhow!("pattern is required"));
        }

        // Map file_type to a simple glob if provided
        let glob = input
            .file_type
            .as_ref()
            .map(|ft| format!("**/*.{}", ft.trim().trim_start_matches('.')));

        // Call rg_search and then pretty-format similar to ripgrep output
        let mut rg_obj = serde_json::Map::new();
        rg_obj.insert("pattern".to_string(), json!(input.pattern));
        rg_obj.insert(
            "path".to_string(),
            json!(input.path.unwrap_or_else(|| ".".to_string())),
        );
        rg_obj.insert(
            "case_sensitive".to_string(),
            json!(input.case_sensitive.unwrap_or(false)),
        ); // default false like Go impl
        if let Some(g) = glob {
            rg_obj.insert("glob_pattern".to_string(), json!(g));
        }
        rg_obj.insert("context_lines".to_string(), json!(0));
        rg_obj.insert("include_hidden".to_string(), json!(false));
        rg_obj.insert("max_results".to_string(), json!(1000));

        let rg_resp = self.rg_search(Value::Object(rg_obj)).await?;
        let matches = rg_resp
            .get("matches")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if matches.is_empty() {
            return Ok(json!({ "output": "No matches found" }));
        }

        let mut lines: Vec<String> = Vec::new();
        for m in matches.iter() {
            let path = m.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let line = m.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
            let text = m.get("line_text").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!("{}:{}:{}", path, line, text));
            if lines.len() >= 1000 {
                break;
            }
        }

        // Trim to first 50 lines to mimic Go version's guardrail
        let total = lines.len();
        let output = if total > 50 {
            let mut head = lines.into_iter().take(50).collect::<Vec<_>>().join("\n");
            head.push_str(&format!("\n... (showing first 50 of {} matches)", total));
            head
        } else {
            lines.join("\n")
        };

        Ok(json!({ "output": output }))
    }

    /// Execute cargo commands (check, clippy, fmt)
    async fn cargo_cmd(&self, sub: &str, extra: &[&str]) -> Result<Value> {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg(sub)
            .args(extra)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let out = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .map_err(|e| anyhow!("failed to spawn cargo {}: {}", sub, e))??;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        Ok(json!({
            "success": out.status.success(),
            "code": out.status.code(),
            "stdout": stdout,
            "stderr": stderr
        }))
    }

    async fn cargo_check(&self, _args: Value) -> Result<Value> {
        self.cargo_cmd("check", &["--quiet"]).await
    }
    async fn cargo_clippy(&self, _args: Value) -> Result<Value> {
        self.cargo_cmd("clippy", &["--quiet"]).await
    }
    async fn cargo_fmt(&self, _args: Value) -> Result<Value> {
        self.cargo_cmd("fmt", &["--", "--check"]).await
    }

    /// Run a terminal command with basic safety checks
    async fn run_terminal_cmd(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct CmdInput {
            command: Vec<String>,
            #[serde(default)]
            working_dir: Option<String>,
        }
        let input: CmdInput =
            serde_json::from_value(args).context("invalid run_terminal_cmd args")?;
        if input.command.is_empty() {
            return Err(anyhow!("command cannot be empty"));
        }
        // Basic injection guards: no shell metacharacters in tokens
        let bad = [";", "&&", "|", ">", "<", "||"];
        for tok in &input.command {
            if bad.iter().any(|b| tok.contains(b)) {
                return Err(anyhow!("disallowed characters in command token"));
            }
        }
        let (prog, rest) = (&input.command[0], &input.command[1..]);
        let mut cmd = std::process::Command::new(prog);
        cmd.args(rest);
        let wd = input.working_dir.as_deref().unwrap_or(".");
        let workdir = self.root.join(wd);
        if !workdir.starts_with(&self.root) {
            return Err(anyhow!("working_dir must be inside workspace"));
        }
        cmd.current_dir(workdir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let out = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .map_err(|e| anyhow!("failed to spawn command: {}", e))??;
        Ok(json!({
            "success": out.status.success(),
            "code": out.status.code(),
            "stdout": String::from_utf8_lossy(&out.stdout),
            "stderr": String::from_utf8_lossy(&out.stderr),
        }))
    }
}

/// Safe text replacement with validation
fn safe_replace_text(content: &str, old_str: &str, new_str: &str) -> Result<String, ToolError> {
    if old_str.is_empty() {
        return Err(ToolError::InvalidArgument(
            "old_string cannot be empty".to_string(),
        ));
    }

    if !content.contains(old_str) {
        return Err(ToolError::TextNotFound(format!(
            "Text '{}' not found in file",
            old_str
        )));
    }

    Ok(content.replace(old_str, new_str))
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Text not found: {0}")]
    TextNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Get cache statistics for monitoring
pub async fn get_cache_stats() -> EnhancedCacheStats {
    FILE_CACHE.get_stats().await
}

/// Get function declarations for tool calling
pub fn build_function_declarations() -> Vec<FunctionDeclaration> {
    vec![
        FunctionDeclaration {
            name: "code_search".to_string(),
            description: "Search code using ripgrep-like semantics. Compatible with Go code_search.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Search pattern (regex)"},
                    "path": {"type": "string", "description": "Base path (file or dir)", "default": "."},
                    "file_type": {"type": "string", "description": "Limit to extension, e.g. 'rs', 'go'"},
                    "case_sensitive": {"type": "boolean", "description": "Case sensitive search", "default": false}
                },
                "required": ["pattern"]
            }),
        },

        FunctionDeclaration {
            name: "codebase_search".to_string(),
            description: "High-level search across common source files (uses rg_search under the hood).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Search pattern"},
                    "path": {"type": "string", "description": "Base path", "default": "."},
                    "case_sensitive": {"type": "boolean", "default": true},
                    "literal": {"type": "boolean", "default": false},
                    "context_lines": {"type": "integer", "default": 0},
                    "include_hidden": {"type": "boolean", "default": false},
                    "max_results": {"type": "integer", "default": 1000}
                },
                "required": ["pattern"]
            }),
        },
        FunctionDeclaration {
            name: "rg_search".to_string(),
            description: "Ripgrep-like high-speed search across the workspace with glob filters and context.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Search pattern (regex unless 'literal' is true)"},
                    "path": {"type": "string", "description": "Base path to search from", "default": "."},
                    "case_sensitive": {"type": "boolean", "description": "Enable case-sensitive search", "default": true},
                    "literal": {"type": "boolean", "description": "Treat pattern as literal text", "default": false},
                    "glob_pattern": {"type": "string", "description": "Glob pattern to filter files (e.g., '**/*.rs')"},
                    "context_lines": {"type": "integer", "description": "Number of context lines before/after each match", "default": 0},
                    "include_hidden": {"type": "boolean", "description": "Include hidden files", "default": false},
                    "max_results": {"type": "integer", "description": "Maximum number of matches to return", "default": 1000}
                },
                "required": ["pattern"]
            }),
        },
        FunctionDeclaration {
            name: "list_files".to_string(),
            description: "List files and directories in a given path".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to list files from"},
                    "max_items": {"type": "integer", "description": "Maximum number of items to return", "default": 1000},
                    "include_hidden": {"type": "boolean", "description": "Include hidden files", "default": false}
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "read_file".to_string(),
            description: "Read content from a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to read"},
                    "max_bytes": {"type": "integer", "description": "Maximum bytes to read", "default": null},
                    "encoding": {"type": "string", "description": "Text encoding", "default": "utf-8"}
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "write_file".to_string(),
            description: "Write content to a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to write"},
                    "content": {"type": "string", "description": "Content to write to the file"},
                    "encoding": {"type": "string", "description": "Text encoding", "default": "utf-8"}
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "edit_file".to_string(),
            description: "Edit a file by replacing text".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to edit"},
                    "old_string": {"type": "string", "description": "Text to replace"},
                    "new_string": {"type": "string", "description": "Replacement text"},
                    "encoding": {"type": "string", "description": "Text encoding", "default": "utf-8"}
                },
                "required": ["path", "old_string", "new_string"]
            }),

        },
        FunctionDeclaration {
            name: "delete_file".to_string(),
            description: "Delete a file in the workspace".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to delete"},
                    "confirm": {"type": "boolean", "description": "Must be true to confirm deletion", "default": false}
                },
                "required": ["path", "confirm"]
            }),
        },

        FunctionDeclaration {
            name: "cargo_check".to_string(),
            description: "Run 'cargo check' in the workspace".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        },
        FunctionDeclaration {
            name: "cargo_clippy".to_string(),
            description: "Run 'cargo clippy' in the workspace".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        },
        FunctionDeclaration {
            name: "cargo_fmt".to_string(),
            description: "Run 'cargo fmt -- --check' in the workspace".to_string(),
            parameters: json!({"type": "object", "properties": {}, "required": []}),
        },
        FunctionDeclaration {
            name: "run_terminal_cmd".to_string(),
            description: "Run a terminal command in the workspace with basic safety checks".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "array", "items": {"type": "string"}, "description": "Program + args as array"},
                    "working_dir": {"type": "string", "description": "Working directory relative to workspace"}
                },
                "required": ["command"]
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_delete_file_removes_temp_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let workspace = temp_dir.path().to_path_buf();
        let registry = ToolRegistry::new(workspace.clone());

        // Create a temporary file inside the workspace
        let file_path = workspace.join("to_delete.txt");
        tokio::fs::write(&file_path, "temporary").await?;
        assert!(file_path.exists());

        // Delete via tool
        let args = json!({ "path": "to_delete.txt" });
        let resp = registry
            .execute_tool(
                "delete_file",
                json!({ "path": "to_delete.txt", "confirm": true }),
            )
            .await?;
        assert_eq!(resp["success"], true);
        assert_eq!(resp["deleted"], true);
        assert!(!file_path.exists());

        // Deleting again should be a no-op
        let resp2 = registry
            .execute_tool("delete_file", json!({ "path": "to_delete.txt" }))
            .await?;
        assert_eq!(resp2["success"], true);
        assert_eq!(resp2["deleted"], false);

        Ok(())
    }
}
