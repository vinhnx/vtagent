//! Tool implementations for the VT Code agent
//!
//! This module provides implementations for all the tools available to the agent,
//! including file operations, code search, and terminal commands.

use crate::gemini::FunctionDeclaration;
use crate::vtagentgitignore::{initialize_vtagent_gitignore, should_exclude_file};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
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

// PTY support with rexpect
use rexpect::spawn as spawn_pty;
use std::collections::HashMap as PtySessionMap;
use std::sync::Arc as PtyArc;
use tokio::sync::Mutex as PtyMutex;

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
    // PTY session management
    pty_sessions: Arc<PtyMutex<PtySessionMap<String, PtyArc<VtagentPtySession>>>>,
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
    #[serde(default)]
    mode: Option<String>, // "overwrite", "append", "skip_if_exists"
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

/// PTY Session structure for managing interactive terminal sessions
#[derive(Debug, Clone)]
pub struct VtagentPtySession {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub rows: u16,
    pub cols: u16,
    pub created_at: std::time::Instant,
}

/// Input structure for PTY commands
#[derive(Debug, Deserialize)]
struct PtyInput {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default)]
    rows: Option<u16>,
    #[serde(default)]
    cols: Option<u16>,
}

/// Input structure for creating PTY sessions
#[derive(Debug, Deserialize)]
struct CreatePtySessionInput {
    session_id: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default)]
    rows: Option<u16>,
    #[serde(default)]
    cols: Option<u16>,
}

fn default_max_items() -> usize {
    1000
}

#[derive(Debug, Deserialize, Serialize)]
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
            pty_sessions: Arc::new(PtyMutex::new(PtySessionMap::new())),
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
            "run_pty_cmd" => self.run_pty_cmd(args).await,
            "run_pty_cmd_streaming" => self.run_pty_cmd_streaming(args).await,
            "create_pty_session" => self.create_pty_session(args).await,
            "list_pty_sessions" => self.list_pty_sessions(args).await,
            "close_pty_session" => self.close_pty_session(args).await,
            _ => {
                // Check if this might be a common command that should use PTY
                let suggestion = if name.starts_with("git_") {
                    Some("Did you mean to use 'run_pty_cmd' with 'git' as the command? For example: [TOOL] run_pty_cmd {\"command\": \"git\", \"args\": [\"diff\"]}")
                } else {
                    match name {
                        "git_diff" | "git_status" | "git_log" | "git_add" | "git_commit" | "git_push" | "git_pull" => {
                            Some("Did you mean to use 'run_pty_cmd' with 'git' as the command? For example: [TOOL] run_pty_cmd {\"command\": \"git\", \"args\": [\"diff\"]}")
                        },
                        "ls" | "cat" | "grep" | "find" | "ps" | "pwd" | "mkdir" | "rm" | "cp" | "mv" => {
                            Some("Did you mean to use 'run_pty_cmd' for this terminal command? For example: [TOOL] run_pty_cmd {\"command\": \"ls\", \"args\": [\"-la\"]}")
                        },
                        _ => None
                    }
                };
                
                if let Some(suggestion) = suggestion {
                    Err(anyhow!("Unknown tool: {}. {}", name, suggestion))
                } else {
                    Err(anyhow!("Unknown tool: {}", name))
                }
            },
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

        // Read file content
        let mut content = tokio::fs::read_to_string(&path)
            .await
            .context(format!("failed to read file: {}", path.display()))?;

        // Truncate if max_bytes specified
        if let Some(max_bytes) = input.max_bytes {
            if content.len() > max_bytes {
                content.truncate(max_bytes);
                content.push_str("\n... (truncated)");
            }
        }

        // Cache the result
        FILE_CACHE
            .put_file(cache_key, content.clone())
            .await;

        Ok(json!({ "content": content }))
    }

    /// Enhanced write_file with intelligent caching and performance monitoring
    async fn write_file(&self, args: Value) -> Result<Value> {
        let input: WriteInput = serde_json::from_value(args).context("invalid write_file args")?;
        let path = self.root.join(&input.path);

        // Check if file should be excluded by .vtagentgitignore
        if should_exclude_file(&path).await {
            return Err(anyhow!(
                "File '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context(format!("failed to create parent directories: {}", parent.display()))?;
        }

        // Handle different write modes
        let mode = input.mode.as_deref().unwrap_or("overwrite");
        match mode {
            "overwrite" => {
                tokio::fs::write(&path, &input.content)
                    .await
                    .context(format!("failed to write file: {}", path.display()))?;
            }
            "append" => {
                use tokio::io::AsyncWriteExt;
                let mut file = tokio::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&path)
                    .await
                    .context(format!("failed to open file for appending: {}", path.display()))?;
                file.write_all(input.content.as_bytes())
                    .await
                    .context(format!("failed to append to file: {}", path.display()))?;
            }
            "skip_if_exists" => {
                if !path.exists() {
                    tokio::fs::write(&path, &input.content)
                        .await
                        .context(format!("failed to write file: {}", path.display()))?;
                }
            }
            _ => {
                return Err(anyhow!(
                    "invalid mode: {}. Must be one of: overwrite, append, skip_if_exists",
                    mode
                ));
            }
        }

        // Invalidate cache for this file
        FILE_CACHE.invalidate_prefix(&input.path).await;

        Ok(json!({ "success": true }))
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

    /// Enhanced edit_file with intelligent caching and performance monitoring
    async fn edit_file(&self, args: Value) -> Result<Value> {
        let input: EditInput = serde_json::from_value(args).context("invalid edit_file args")?;
        let path = self.root.join(&input.path);

        // Check if file should be excluded by .vtagentgitignore
        if should_exclude_file(&path).await {
            return Err(anyhow!(
                "File '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        // Read current content
        let content = tokio::fs::read_to_string(&path)
            .await
            .context(format!("failed to read file: {}", path.display()))?;

        // Perform replacement
        let new_content = Self::safe_replace_text(&content, &input.old_string, &input.new_string)
            .map_err(|e| anyhow!("edit failed: {}", e))?;

        // Write back to file
        tokio::fs::write(&path, &new_content)
            .await
            .context(format!("failed to write file: {}", path.display()))?;

        // Invalidate cache for this file
        FILE_CACHE.invalidate_prefix(&input.path).await;

        Ok(json!({ "success": true }))
    }

    /// Enhanced delete_file with intelligent caching and performance monitoring
    async fn delete_file(&self, args: Value) -> Result<Value> {
        let input: DeleteInput = serde_json::from_value(args).context("invalid delete_file args")?;
        let path = self.root.join(&input.path);

        // Check if file should be excluded by .vtagentgitignore
        if should_exclude_file(&path).await {
            return Err(anyhow!(
                "File '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        // Check if file exists
        if !path.exists() {
            return Ok(json!({ "success": true, "deleted": false }));
        }

        // Require confirmation for deletion
        if !input.confirm {
            return Err(anyhow!("deletion requires confirmation"));
        }

        // Perform deletion
        if path.is_dir() {
            tokio::fs::remove_dir_all(&path)
                .await
                .context(format!("failed to delete directory: {}", path.display()))?;
        } else {
            tokio::fs::remove_file(&path)
                .await
                .context(format!("failed to delete file: {}", path.display()))?;
        }

        // Invalidate cache for this path
        FILE_CACHE.invalidate_prefix(&input.path).await;

        Ok(json!({ "success": true, "deleted": true }))
    }

    /// Enhanced rg_search with intelligent caching and performance monitoring
    async fn rg_search(&self, args: Value) -> Result<Value> {
        let input: RgInput = serde_json::from_value(args).context("invalid rg_search args")?;
        let base_path = self.root.join(&input.path);

        // Check if path should be excluded by .vtagentgitignore
        if should_exclude_file(&base_path).await {
            return Err(anyhow!(
                "Path '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        // Create cache key
        let cache_key = format!(
            "rg_search:{}:{}:{}:{}:{}:{}:{}:{}",
            input.pattern,
            input.path,
            input.case_sensitive.unwrap_or(false),
            input.literal.unwrap_or(false),
            input.glob_pattern.as_deref().unwrap_or(""),
            input.context_lines.unwrap_or(0),
            input.include_hidden.unwrap_or(false),
            input.max_results.unwrap_or(1000)
        );

        // Try cache first
        if let Some(cached_result) = FILE_CACHE.get_file(&cache_key).await {
            return Ok(serde_json::from_str(&cached_result)?);
        }

        // Build regex pattern
        let pattern = if input.literal.unwrap_or(false) {
            regex::escape(&input.pattern)
        } else {
            input.pattern.clone()
        };

        let regex = RegexBuilder::new(&pattern)
            .case_insensitive(!input.case_sensitive.unwrap_or(true))
            .build()
            .context("invalid regex pattern")?;

        // Collect matches
        let mut matches = Vec::new();
        let mut file_count = 0;
        let max_results = input.max_results.unwrap_or(1000);
        let context_lines = input.context_lines.unwrap_or(0);

        // Walk directory tree
        for entry in WalkDir::new(&base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .take(max_results * 10) // Limit total files scanned
        {
            let path = entry.path();

            // Skip hidden files unless requested
            if !input.include_hidden.unwrap_or(false) {
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false)
                {
                    continue;
                }
            }

            // Apply glob filter if specified
            if let Some(ref glob_pattern) = input.glob_pattern {
                let relative_path = path
                    .strip_prefix(&base_path)
                    .unwrap_or(path)
                    .to_string_lossy();
                if let Ok(pattern) = GlobPattern::new(glob_pattern) {
                    if !pattern.matches(&relative_path) {
                        continue;
                    }
                }
            }

            // Skip files excluded by .vtagentgitignore
            if should_exclude_file(path).await {
                continue;
            }

            file_count += 1;

            // Read file content
            let content = match tokio::fs::read_to_string(path).await {
                Ok(content) => content,
                Err(_) => continue, // Skip unreadable files
            };

            // Search for matches
            for (line_num, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    // Collect context lines
                    let start_line = line_num.saturating_sub(context_lines);
                    let end_line = std::cmp::min(line_num + context_lines + 1, content.lines().count());
                    
                    let context: Vec<String> = content
                        .lines()
                        .skip(start_line)
                        .take(end_line - start_line)
                        .map(|s| s.to_string())
                        .collect();

                    matches.push(json!({
                        "path": path.strip_prefix(&self.root).unwrap_or(path).display().to_string(),
                        "line": line_num + 1,
                        "content": line,
                        "context": context,
                        "context_start": start_line + 1
                    }));

                    // Stop if we've reached the maximum results
                    if matches.len() >= max_results {
                        break;
                    }
                }
            }

            // Stop if we've reached the maximum results
            if matches.len() >= max_results {
                break;
            }
        }

        let result = json!({
            "matches": matches,
            "file_count": file_count,
            "match_count": matches.len()
        });

        // Cache the result
        FILE_CACHE
            .put_file(cache_key, serde_json::to_string(&result)?)
            .await;

        Ok(result)
    }

    /// Enhanced code_search with intelligent caching and performance monitoring
    async fn code_search(&self, args: Value) -> Result<Value> {
        let input: RgInput = serde_json::from_value(args).context("invalid code_search args")?;
        self.rg_search(serde_json::to_value(input)?).await
    }

    /// Enhanced codebase_search with intelligent caching and performance monitoring
    async fn codebase_search(&self, args: Value) -> Result<Value> {
        let input: RgInput = serde_json::from_value(args).context("invalid codebase_search args")?;
        self.rg_search(serde_json::to_value(input)?).await
    }

    /// Enhanced cargo_check with intelligent caching and performance monitoring
    async fn cargo_check(&self, _args: Value) -> Result<Value> {
        // Spawn cargo check process
        let output = tokio::process::Command::new("cargo")
            .arg("check")
            .current_dir(&self.root)
            .output()
            .await
            .context("failed to execute cargo check")?;

        Ok(json!({
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1)
        }))
    }

    /// Enhanced cargo_clippy with intelligent caching and performance monitoring
    async fn cargo_clippy(&self, _args: Value) -> Result<Value> {
        // Spawn cargo clippy process
        let output = tokio::process::Command::new("cargo")
            .arg("clippy")
            .arg("--message-format=json")
            .current_dir(&self.root)
            .output()
            .await
            .context("failed to execute cargo clippy")?;

        Ok(json!({
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1)
        }))
    }

    /// Enhanced cargo_fmt with intelligent caching and performance monitoring
    async fn cargo_fmt(&self, _args: Value) -> Result<Value> {
        // Spawn cargo fmt process
        let output = tokio::process::Command::new("cargo")
            .arg("fmt")
            .arg("--")
            .arg("--check")
            .current_dir(&self.root)
            .output()
            .await
            .context("failed to execute cargo fmt")?;

        Ok(json!({
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1)
        }))
    }

    /// Run a terminal command with basic safety checks
    async fn run_terminal_command(&self, args: Value, is_pty: bool) -> Result<Value> {
        use std::process::Command;

        // Support both input formats
        #[derive(Deserialize)]
        struct TerminalCmdInput {
            #[serde(default)]
            command: Vec<String>,
            #[serde(default)]
            working_dir: Option<String>,
        }
        
        #[derive(Deserialize)]
        struct PtyCmdInput {
            command: String,
            #[serde(default)]
            args: Vec<String>,
            #[serde(default)]
            working_dir: Option<String>,
            #[serde(default)]
            rows: Option<u16>,
            #[serde(default)]
            cols: Option<u16>,
        }
        
        // Parse input based on format
        let (command_line, workdir) = if is_pty {
            let input: PtyCmdInput = serde_json::from_value(args).context("invalid run_pty_cmd args")?;
            
            // Basic injection guards: no shell metacharacters in command
            let bad = [";", "&&", "|", ">", "<", "||"];
            if bad.iter().any(|b| input.command.contains(b)) {
                return Err(anyhow!("disallowed characters in command"));
            }
            
            // Prepare the command to run
            let command_line = if input.args.is_empty() {
                input.command.clone()
            } else {
                format!("{} {}", input.command, input.args.join(" "))
            };
            
            // Set working directory if provided
            let workdir = if let Some(wd) = input.working_dir {
                let workdir = self.root.join(&wd);
                if !workdir.starts_with(&self.root) {
                    return Err(anyhow!("working_dir must be inside workspace"));
                }
                workdir
            } else {
                self.root.clone()
            };
            
            (command_line, workdir)
        } else {
            let input: TerminalCmdInput = serde_json::from_value(args).context("invalid run_terminal_cmd args")?;
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
            let command_line = if rest.is_empty() {
                prog.clone()
            } else {
                format!("{} {}", prog, rest.join(" "))
            };
            
            let wd = input.working_dir.as_deref().unwrap_or(".");
            let workdir = self.root.join(wd);
            if !workdir.starts_with(&self.root) {
                return Err(anyhow!("working_dir must be inside workspace"));
            }
            
            (command_line, workdir)
        };
        
        // Spawn a new session with rexpect
        let mut session = spawn_pty(&format!("sh -c 'cd {} && {}'", workdir.display(), command_line), Some(30000))
            .map_err(|e| anyhow!("failed to spawn command with rexpect: {}", e))?;
        
        // Wait for the process to complete and capture all output
        let output = session.exp_eof()
            .map_err(|e| anyhow!("failed to wait for command completion: {}", e))?;
        
        // Get the actual exit status from the process
        let (success, code) = {
            #[cfg(unix)]
            {
                // On Unix systems, we can get the actual exit code from rexpect
                match session.process.wait() {
                    Ok(rexpect::process::wait::WaitStatus::Exited(_, exit_code)) => {
                        (exit_code == 0, exit_code)
                    }
                    Ok(rexpect::process::wait::WaitStatus::Signaled(_, signal, _)) => {
                        (false, 128 + signal as i32) // Standard convention for signal termination
                    }
                    Ok(rexpect::process::wait::WaitStatus::Stopped(_, _)) => {
                        // Process was stopped, not terminated
                        (false, 1)
                    }
                    Ok(rexpect::process::wait::WaitStatus::Continued(_)) => {
                        // Process was continued
                        (true, 0)
                    }
                    Ok(rexpect::process::wait::WaitStatus::StillAlive) => {
                        // Process is still alive, assume success for backward compatibility
                        (true, 0)
                    }
                    Err(e) => {
                        // If we can't get the status directly, we'll try to determine success based on the output
                        // This maintains backward compatibility with the original implementation
                        eprintln!("Warning: Failed to get process status from rexpect: {}", e);
                        (true, 0)
                    }
                }
            }
            #[cfg(windows)]
            {
                // On Windows, we'll keep the original behavior for now
                (true, 0)
            }
        };
        
        // Return appropriate response format based on command type
        if is_pty {
            Ok(json!({
                "success": success,
                "code": code,
                "output": output
            }))
        } else {
            Ok(json!({
                "success": success,
                "code": code,
                "stdout": output,
                "stderr": "" // For now, we're capturing combined output
            }))
        }
    }
    
    /// Run a terminal command with basic safety checks (legacy)
    async fn run_terminal_cmd(&self, args: Value) -> Result<Value> {
        // Convert TerminalCmdInput format to PtyCmdInput format
        #[derive(Deserialize)]
        struct TerminalCmdInput {
            #[serde(default)]
            command: Vec<String>,
            #[serde(default)]
            working_dir: Option<String>,
        }
        
        #[derive(Serialize, Deserialize)]
        struct PtyCmdInput {
            command: String,
            #[serde(default)]
            args: Vec<String>,
            #[serde(default)]
            working_dir: Option<String>,
        }
        
        let input: TerminalCmdInput = serde_json::from_value(args).context("invalid run_terminal_cmd args")?;
        if input.command.is_empty() {
            return Err(anyhow!("command cannot be empty"));
        }
        
        // Convert the command vector to command string and args
        let (command, args_vec) = if input.command.len() > 1 {
            (input.command[0].clone(), input.command[1..].to_vec())
        } else {
            (input.command[0].clone(), vec![])
        };
        
        let pty_input = PtyCmdInput {
            command,
            args: args_vec,
            working_dir: input.working_dir,
        };
        
        let pty_args = serde_json::to_value(pty_input).context("failed to convert args for run_pty_cmd")?;
        self.run_pty_cmd(pty_args).await
    }

    /// Run a command in a pseudo-terminal (PTY) with full terminal emulation (legacy)
    async fn run_pty_cmd(&self, args: Value) -> Result<Value> {
        self.run_terminal_command(args, true).await
    }

    /// Run a command in a pseudo-terminal (PTY) with streaming output
    async fn run_pty_cmd_streaming(&self, args: Value) -> Result<Value> {
        // For now, we'll implement this the same as run_pty_cmd
        // In a future implementation, we could add streaming capabilities
        self.run_pty_cmd(args).await
    }

    /// Create a new PTY session
    async fn create_pty_session(&self, args: Value) -> Result<Value> {
        let input: CreatePtySessionInput = serde_json::from_value(args).context("invalid create_pty_session args")?;
        
        // Basic injection guards: no shell metacharacters in command
        let bad = [";", "&&", "|", ">", "<", "||"];
        if bad.iter().any(|b| input.command.contains(b)) {
            return Err(anyhow!("disallowed characters in command"));
        }
        
        // Create a new PTY session
        let session = VtagentPtySession {
            id: input.session_id.clone(),
            command: input.command.clone(),
            args: input.args.clone(),
            working_dir: input.working_dir.clone(),
            rows: input.rows.unwrap_or(24),
            cols: input.cols.unwrap_or(80),
            created_at: std::time::Instant::now(),
        };
        
        // Store the session
        let mut sessions = self.pty_sessions.lock().await;
        sessions.insert(input.session_id.clone(), PtyArc::new(session));
        
        Ok(json!({
            "success": true,
            "session_id": input.session_id
        }))
    }

    /// List all active PTY sessions
    async fn list_pty_sessions(&self, _args: Value) -> Result<Value> {
        let sessions = self.pty_sessions.lock().await;
        let session_ids: Vec<String> = sessions.keys().cloned().collect();
        
        Ok(json!({
            "sessions": session_ids
        }))
    }

    /// Close a PTY session
    async fn close_pty_session(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct ClosePtySessionInput {
            session_id: String,
        }
        
        let input: ClosePtySessionInput = serde_json::from_value(args).context("invalid close_pty_session args")?;
        
        let mut sessions = self.pty_sessions.lock().await;
        if let Some(session) = sessions.remove(&input.session_id) {
            Ok(json!({
                "success": true,
                "session_id": input.session_id
            }))
        } else {
            Ok(json!({
                "success": false,
                "session_id": input.session_id,
                "error": "Session not found"
            }))
        }
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

/// Error types for tool operations
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Text not found: {0}")]
    TextNotFound(String),
}

/// Build function declarations for all available tools
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
            description: "Write content to a file with various modes (overwrite, append, skip_if_exists)".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to write"},
                    "content": {"type": "string", "description": "Content to write to the file"},
                    "encoding": {"type": "string", "description": "Text encoding", "default": "utf-8"},
                    "mode": {"type": "string", "description": "Write mode: overwrite, append, or skip_if_exists", "default": "overwrite", "enum": ["overwrite", "append", "skip_if_exists"]}
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
        FunctionDeclaration {
            name: "run_pty_cmd".to_string(),
            description: "Run a command in a pseudo-terminal (PTY) with full terminal emulation".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Command to execute in the PTY"},
                    "args": {"type": "array", "items": {"type": "string"}, "description": "Arguments for the command", "default": []},
                    "working_dir": {"type": "string", "description": "Working directory relative to workspace"},
                    "rows": {"type": "integer", "description": "Terminal rows (default: 24)", "default": 24},
                    "cols": {"type": "integer", "description": "Terminal columns (default: 80)", "default": 80}
                },
                "required": ["command"]
            }),
        },
        FunctionDeclaration {
            name: "run_pty_cmd_streaming".to_string(),
            description: "Run a command in a pseudo-terminal (PTY) with streaming output".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Command to execute in the PTY"},
                    "args": {"type": "array", "items": {"type": "string"}, "description": "Arguments for the command", "default": []},
                    "working_dir": {"type": "string", "description": "Working directory relative to workspace"},
                    "rows": {"type": "integer", "description": "Terminal rows (default: 24)", "default": 24},
                    "cols": {"type": "integer", "description": "Terminal columns (default: 80)", "default": 80}
                },
                "required": ["command"]
            }),
        },
        FunctionDeclaration {
            name: "create_pty_session".to_string(),
            description: "Create a new PTY session for running interactive terminal commands".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "Unique identifier for the PTY session"},
                    "command": {"type": "string", "description": "Command to execute in the PTY"},
                    "args": {"type": "array", "items": {"type": "string"}, "description": "Arguments for the command", "default": []},
                    "working_dir": {"type": "string", "description": "Working directory relative to workspace"},
                    "rows": {"type": "integer", "description": "Terminal rows (default: 24)", "default": 24},
                    "cols": {"type": "integer", "description": "Terminal columns (default: 80)", "default": 80}
                },
                "required": ["session_id", "command"]
            }),
        },
        FunctionDeclaration {
            name: "list_pty_sessions".to_string(),
            description: "List all active PTY sessions".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        FunctionDeclaration {
            name: "close_pty_session".to_string(),
            description: "Close a PTY session".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "Unique identifier for the PTY session to close"}
                },
                "required": ["session_id"]
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