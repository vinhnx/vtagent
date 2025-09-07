//! Tool implementations for the VT Code agent
//!
//! This module provides implementations for all the tools available to the agent,
//! including file operations, code search, and terminal commands.

use crate::ast_grep::AstGrepEngine;
use crate::file_search::{FileSearchConfig, FileSearcher};

use crate::gemini::FunctionDeclaration;
use crate::rp_search::RpSearchManager;
use crate::vtagentgitignore::{initialize_vtagent_gitignore, should_exclude_file};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use walkdir::WalkDir;
// Performance optimization imports
use dashmap::DashMap;
use lru::LruCache;
use once_cell::sync::Lazy;
use std::num::NonZeroUsize;
// Regex for pattern matching
use regex;

// PTY support with rexpect
// use rexpect::spawn as spawn_pty;
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
    // PTY sessions
    pty_sessions: Arc<PtyMutex<PtySessionMap<String, PtyArc<VtagentPtySession>>>>,
    // RP Search manager for debounce/cancellation logic
    rp_search_manager: Arc<RpSearchManager>,
    // AST-grep engine for syntax-aware code operations
    ast_grep_engine: Option<AstGrepEngine>,
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
    #[serde(default)]
    ast_grep_pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WriteInput {
    path: String,
    content: String,
    #[serde(default)]
    encoding: Option<String>,
    #[serde(default)]
    mode: Option<String>, // "overwrite", "append", "skip_if_exists"
    #[serde(default)]
    ast_grep_lint: Option<bool>,
    #[serde(default)]
    ast_grep_refactor: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct EditInput {
    path: String,
    old_string: String,
    new_string: String,
    #[serde(default)]
    encoding: Option<String>,
    #[serde(default)]
    ast_grep_lint: Option<bool>,
    #[serde(default)]
    ast_grep_refactor: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DeleteInput {
    path: String,
    #[serde(default)]
    confirm: bool,
    #[serde(default)]
    ast_grep_warn_pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListInput {
    path: String,
    #[serde(default = "default_max_items")]
    max_items: usize,
    #[serde(default)]
    include_hidden: bool,
    #[serde(default)]
    ast_grep_pattern: Option<String>,
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
    #[serde(default)]
    file_type: Option<String>,
}

fn default_search_path() -> String {
    ".".to_string()
}

impl ToolRegistry {
    pub fn new(root: PathBuf) -> Self {
        let cargo_toml_path = root.join("Cargo.toml");
        let rp_search_manager = Arc::new(RpSearchManager::new(root.clone()));
        // Initialize AST-grep engine when possible
        let ast_grep_engine = AstGrepEngine::new().ok();

        Self {
            root,
            cargo_toml_path,
            operation_stats: Arc::new(RwLock::new(HashMap::new())),
            max_cache_size: 1000,
            pty_sessions: Arc::new(PtyMutex::new(PtySessionMap::new())),
            rp_search_manager,
            ast_grep_engine,
        }
    }

    /// Intelligently resolve file paths by trying multiple variations
    /// This helps the agent find files even when given imprecise paths
    fn resolve_file_path(&self, input_path: &str) -> Result<Vec<PathBuf>> {
        let mut candidates = Vec::new();

        // 1. Try the exact path as provided (relative to workspace root)
        candidates.push(self.root.join(input_path));

        // 2. If it's just a filename, try common directories
        if !input_path.contains('/') && !input_path.contains('\\') {
            let common_dirs = [
                "src",
                "lib",
                "bin",
                "tests",
                "examples",
                "benches",
                ".", // current directory
            ];

            for dir in &common_dirs {
                candidates.push(self.root.join(dir).join(input_path));
            }

            // Also try recursively searching for the file
            if let Ok(entries) = std::fs::read_dir(&self.root) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        let dir_path = entry.path();
                        candidates.push(dir_path.join(input_path));
                    }
                }
            }
        }

        // 3. If input looks like it's missing a common extension, try adding them
        if !input_path.contains('.') {
            let extensions = ["rs", "toml", "md", "txt", "json", "yaml", "yml", "js", "ts", "py"];
            for ext in &extensions {
                candidates.push(self.root.join(format!("{}.{}", input_path, ext)));

                // Also try in src/ directory
                candidates.push(self.root.join("src").join(format!("{}.{}", input_path, ext)));
            }
        }

        // 4. If it starts with "/" or "\", treat as absolute path
        if input_path.starts_with('/') || input_path.starts_with('\\') {
            candidates.push(PathBuf::from(input_path));
        }

        // 5. Try case-insensitive variations (for common files)
        let lowercase = input_path.to_lowercase();
        if lowercase != input_path {
            candidates.push(self.root.join(&lowercase));
            candidates.push(self.root.join("src").join(&lowercase));
        }

        let uppercase = input_path.to_uppercase();
        if uppercase != input_path {
            candidates.push(self.root.join(&uppercase));
        }

        // Remove duplicates while preserving order (first match wins)
        let mut seen = std::collections::HashSet::new();
        candidates.retain(|path| seen.insert(path.clone()));

        Ok(candidates)
    }

    /// Determine if a search pattern should be treated as literal text
    /// This helps avoid regex parsing errors for common code patterns
    fn should_pattern_be_literal(&self, pattern: &str) -> bool {
        // Patterns that contain common code constructs that are likely meant literally
        let literal_indicators = [
            "(",     // Function calls like "fn main("
            ")",     // Closing parentheses
            "[",     // Array/slice syntax
            "]",     // Array/slice closing
            "{",     // Braces
            "}",     // Closing braces
            "::(",   // Rust method calls
            ".(",    // Method calls
            "->",    // Rust return types
            "=>",    // Rust match arms
            "<",     // Generic brackets (less common but can cause issues)
            ">",     // Closing generic brackets
        ];

        // Common regex metacharacters that are often meant literally in code search
        let regex_metacharacters = [
            "+",     // Could be regex quantifier or arithmetic
            "*",     // Could be regex quantifier or dereference/glob
            "?",     // Could be regex quantifier or Option syntax
            "^",     // Could be regex anchor or XOR
            "$",     // Could be regex anchor or variable
            "|",     // Could be regex alternation or pipe
        ];

        // If the pattern contains parentheses or brackets, it's likely code syntax
        for indicator in &literal_indicators {
            if pattern.contains(indicator) {
                return true;
            }
        }

        // If pattern contains multiple regex metacharacters, likely meant as literal code
        let metachar_count = regex_metacharacters.iter()
            .filter(|&c| pattern.contains(c))
            .count();

        if metachar_count >= 2 {
            return true;
        }

        // If pattern looks like a function signature or similar code construct
        if pattern.contains("fn ") ||
           pattern.contains("struct ") ||
           pattern.contains("impl ") ||
           pattern.contains("pub fn") ||
           pattern.contains("async fn") ||
           pattern.contains("const ") ||
           pattern.contains("let ") ||
           pattern.contains("use ") {
            return true;
        }

        // Otherwise, treat as regex (default ripgrep behavior)
        false
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
            "rp_search" => self.rp_search(args).await,
            "code_search" => self.code_search(args).await,
            "codebase_search" => self.codebase_search(args).await,
            "run_terminal_cmd" => self.run_terminal_cmd(args).await,
            "run_pty_cmd" => self.run_pty_cmd(args).await,
            "run_pty_cmd_streaming" => self.run_pty_cmd_streaming(args).await,
            "create_pty_session" => self.create_pty_session(args).await,
            "list_pty_sessions" => self.list_pty_sessions(args).await,
            "close_pty_session" => self.close_pty_session(args).await,
            "ast_grep_search" => self.ast_grep_search(args).await,
            "ast_grep_transform" => self.ast_grep_transform(args).await,
            "ast_grep_lint" => self.ast_grep_lint(args).await,
            "ast_grep_refactor" => self.ast_grep_refactor(args).await,
            "fuzzy_search" => self.fuzzy_search(args).await,
            "similarity_search" => self.similarity_search(args).await,
            "multi_pattern_search" => self.multi_pattern_search(args).await,
            "extract_text_patterns" => self.extract_text_patterns(args).await,
            "find_config_file" => self.find_config_file(args).await,
            "recursive_file_search" => self.recursive_file_search(args).await,
            "search_files_with_content" => self.search_files_with_content(args).await,
            "find_file_by_name" => self.find_file_by_name(args).await,
            _ => {
                // Check if this might be a common command that should use PTY
                let suggestion = if name.starts_with("git_") {
                    Some(
                        "Did you mean to use 'run_pty_cmd' with 'git' as the command? For example: [TOOL] run_pty_cmd {\"command\": \"git\", \"args\": [\"diff\"]}",
                    )
                } else {
                    match name {
                        "git_diff" | "git_status" | "git_log" | "git_add" | "git_commit"
                        | "git_push" | "git_pull" => Some(
                            "Did you mean to use 'run_pty_cmd' with 'git' as the command? For example: [TOOL] run_pty_cmd {\"command\": \"git\", \"args\": [\"diff\"]}",
                        ),
                        "ls" | "cat" | "grep" | "find" | "ps" | "pwd" | "mkdir" | "rm" | "cp"
                        | "mv" => Some(
                            "Did you mean to use 'run_pty_cmd' for this terminal command? For example: [TOOL] run_pty_cmd {\"command\": \"ls\", \"args\": [\"-la\"]}",
                        ),
                        _ => None,
                    }
                };

                if let Some(suggestion) = suggestion {
                    Err(anyhow!("Unknown tool: {}. {}", name, suggestion))
                } else {
                    Err(anyhow!("Unknown tool: {}", name))
                }
            }
        }
    }

    /// Execute a tool by name with given arguments (alias for execute_tool)
    pub async fn execute(&self, name: &str, args: Value) -> Result<Value> {
        self.execute_tool(name, args).await
    }

    /// Enhanced list_files with intelligent caching and performance monitoring
    async fn list_files(&self, args: Value) -> Result<Value> {
        let input: ListInput = serde_json::from_value(args).context("invalid list_files args")?;
        let base = self.root.join(&input.path);
        let ast_grep_pattern = input.ast_grep_pattern.clone();

        // Create cache key
        let cache_key = format!(
            "list_files:{}:{}:{}:{}",
            base.display(),
            input.max_items,
            input.include_hidden,
            ast_grep_pattern.clone().unwrap_or_default()
        );

        // Try cache first
        if let Some(cached_result) = FILE_CACHE.get_directory(&cache_key).await {
            return Ok(cached_result);
        }

        // Generate fresh result
        let mut result = self
            .generate_directory_listing(&base, input.max_items, input.include_hidden)
            .await?;

        // If ast_grep_pattern is provided, filter files by AST match
        if let Some(_pattern) = ast_grep_pattern {
            let entries = result["files"].as_array_mut().unwrap();
            let mut filtered = Vec::new();
            for entry in entries.iter() {
                if entry["is_dir"].as_bool().unwrap_or(false) {
                    filtered.push(entry.clone());
                    continue;
                }
                let _file_path = self.root.join(entry["path"].as_str().unwrap());
                // TODO: Implement AST-grep pattern filtering when engine is available
                // if self.ast_grep_engine.has_pattern(&file_path, &pattern).await.unwrap_or(false) {
                //     filtered.push(entry.clone());
                // }
                // For now, include all files
                filtered.push(entry.clone());
            }
            result["files"] = json!(filtered);
        }
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

        Ok(json!({ "files": entries }))
    }

    /// Enhanced read_file with intelligent caching and performance monitoring
    async fn read_file(&self, args: Value) -> Result<Value> {
        let input: Input = serde_json::from_value(args).context("invalid read_file args")?;
        let ast_grep_pattern = input.ast_grep_pattern.clone();

        // Intelligent path resolution - try multiple variations
        let potential_paths = self.resolve_file_path(&input.path)?;

        let mut last_error = None;
        let mut resolved_path = None;

        // Try each potential path until we find one that exists
        for candidate_path in &potential_paths {
            // Check if file should be excluded by .vtagentgitignore
            if should_exclude_file(candidate_path).await {
                last_error = Some(anyhow!(
                    "File '{}' is excluded by .vtagentgitignore",
                    candidate_path.display()
                ));
                continue;
            }

            // Check if the file exists and is readable
            if candidate_path.exists() && candidate_path.is_file() {
                resolved_path = Some(candidate_path.clone());
                break;
            } else {
                last_error = Some(anyhow!(
                    "File not found: {}",
                    candidate_path.display()
                ));
            }
        }

        let path = resolved_path.ok_or_else(|| {
            last_error.unwrap_or_else(|| anyhow!(
                "Could not resolve file path '{}'. Tried paths: {}",
                input.path,
                potential_paths.iter()
                    .map(|p| format!("'{}'", p.display()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

        // Create cache key with resolved path
        let cache_key = format!(
            "read_file:{}:{}:{}",
            path.display(),
            input.max_bytes.unwrap_or(usize::MAX),
            ast_grep_pattern.clone().unwrap_or_default()
        );

        // Try cache first
        if let Some(cached_content) = FILE_CACHE.get_file(&cache_key).await {
            if ast_grep_pattern.is_some() {
                // Don't cache ast_grep results, always run fresh
                // (to avoid stale results if pattern changes)
            } else {
                return Ok(json!({ "content": cached_content }));
            }
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

        // If ast_grep_pattern is provided, extract AST matches
        // TODO: Implement AST-grep pattern extraction when engine is available
        let ast_grep_matches: Option<Vec<String>> = None;
        // if let Some(pattern) = ast_grep_pattern {
        //     ast_grep_matches = self.ast_grep_engine.extract_matches(&path, &pattern).await.ok();
        // }

        // Cache the result (only if no ast_grep)
        if ast_grep_pattern.is_none() {
            FILE_CACHE.put_file(cache_key, content.clone()).await;
        }

        // Get file metadata
        let metadata = tokio::fs::metadata(&path).await.context(format!("failed to get metadata for file: {}", path.display()))?;
        let size = metadata.len() as usize;

        Ok(json!({ "content": content, "ast_grep_matches": ast_grep_matches, "metadata": { "size": size } }))
    }

    /// Enhanced write_file with intelligent caching and performance monitoring
    async fn write_file(&self, args: Value) -> Result<Value> {
        let input: WriteInput = serde_json::from_value(args).context("invalid write_file args")?;
        let path = self.root.join(&input.path);
        let _ast_grep_lint = input.ast_grep_lint.unwrap_or(false);
        let _ast_grep_refactor = input.ast_grep_refactor.unwrap_or(false);

        // Check if file should be excluded by .vtagentgitignore
        if should_exclude_file(&path).await {
            return Err(anyhow!(
                "File '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.context(format!(
                "failed to create parent directories: {}",
                parent.display()
            ))?;
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
                    .context(format!(
                        "failed to open file for appending: {}",
                        path.display()
                    ))?;
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
            "patch" => {
                // Validate patch content
                Self::validate_patch_content(&input.content)
                    .map_err(|e| anyhow!("invalid patch format: {}", e))?;

                // Read existing content
                let existing_content = if path.exists() {
                    tokio::fs::read_to_string(&path)
                        .await
                        .context(format!("failed to read existing file: {}", path.display()))?
                } else {
                    String::new()
                };

                // Apply patch
                let new_content =
                    Self::apply_patch_enhanced(&existing_content, &input.content, true)
                        .map_err(|e| anyhow!("patch application failed: {}", e))?;

                // Write the patched content
                tokio::fs::write(&path, &new_content)
                    .await
                    .context(format!("failed to write patched file: {}", path.display()))?;
            }
            _ => {
                return Err(anyhow!(
                    "invalid mode: {}. Must be one of: overwrite, append, skip_if_exists, patch",
                    mode
                ));
            }
        }

        // Invalidate cache for this file
        FILE_CACHE.invalidate_prefix(&input.path).await;

        // Optionally run ast-grep lint/refactor
        let lint_results: Option<Vec<String>> = None;
        let refactor_results: Option<Vec<String>> = None;
        // TODO: Implement AST-grep lint/refactor when engine is available
        // if ast_grep_lint {
        //     lint_results = self.ast_grep_engine.lint_file(&path).await.ok();
        // }
        // if ast_grep_refactor {
        //     refactor_results = self.ast_grep_engine.refactor_file(&path).await.ok();
        // }

        Ok(
            json!({ "success": true, "lint_results": lint_results, "refactor_results": refactor_results }),
        )
    }

    /// Safe text replacement with validation
    fn safe_replace_text(content: &str, old_str: &str, new_str: &str) -> Result<String, ToolError> {
        crate::utils::safe_replace_text(content, old_str, new_str)
    }

    /// Apply a patch to file content using line-based diff approach
    /// This function provides robust patch application with validation
    fn apply_patch(content: &str, patch: &str) -> Result<String, ToolError> {
        // Parse the patch format (simplified unified diff format)
        let _lines: Vec<&str> = content.lines().collect();
        let patch_lines: Vec<&str> = patch.lines().collect();

        let mut result_lines = Vec::new();
        let mut i = 0;

        while i < patch_lines.len() {
            let line = patch_lines[i];

            // Handle patch header lines (we'll skip them for simplicity)
            if line.starts_with("--- ") || line.starts_with("+++ ") {
                i += 1;
                continue;
            }

            // Handle hunk header
            if line.starts_with("@@") {
                i += 1;
                continue;
            }

            // Handle context lines (lines that start with space or are empty)
            if line.is_empty() || line.starts_with(' ') {
                let content_line = line.strip_prefix(' ').unwrap_or(line);
                result_lines.push(content_line.to_string());
                i += 1;
                continue;
            }

            // Handle added lines (lines that start with +)
            if line.starts_with('+') {
                let added_line = line.strip_prefix('+').unwrap_or(&line[1..]);
                result_lines.push(added_line.to_string());
                i += 1;
                continue;
            }

            // Handle removed lines (lines that start with -)
            if line.starts_with('-') {
                // We skip removed lines as they're not added to the result
                i += 1;
                continue;
            }

            // For any other lines, treat as context
            result_lines.push(line.to_string());
            i += 1;
        }

        Ok(result_lines.join("\n"))
    }

    /// Enhanced patch application with validation and error handling
    /// This implements a more robust approach similar to OpenAI Codex
    fn apply_patch_enhanced(
        content: &str,
        patch: &str,
        validation: bool,
    ) -> Result<String, ToolError> {
        // For a production implementation, we would use a proper diff library
        // For now, we'll implement a simplified version

        // Basic validation - check if patch is empty
        if patch.trim().is_empty() {
            return Err(ToolError::InvalidArgument(
                "Patch cannot be empty".to_string(),
            ));
        }

        // Try to apply the patch
        match Self::apply_patch(content, patch) {
            Ok(result) => {
                // Optional validation
                if validation {
                    // Simple validation: check if result is significantly different
                    let original_lines: Vec<&str> = content.lines().collect();
                    let result_lines: Vec<&str> = result.lines().collect();

                    // If result is empty but original wasn't, that's likely an error
                    if result_lines.is_empty() && !original_lines.is_empty() {
                        return Err(ToolError::InvalidArgument(
                            "Patch would result in empty file".to_string(),
                        ));
                    }

                    // Check if the patch actually made changes
                    if content == result {
                        // This might be okay if the patch was meant to make no changes
                        // but let's at least log it
                    }
                }
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    /// Validate patch content for common issues
    fn validate_patch_content(patch: &str) -> Result<(), ToolError> {
        // Check for basic patch format indicators
        let lines: Vec<&str> = patch.lines().collect();

        // A valid patch should have at least some structure
        let has_patch_indicators = lines
            .iter()
            .any(|line| line.starts_with("@@") || line.starts_with("+") || line.starts_with("-"));

        if !has_patch_indicators && lines.len() > 10 {
            // If it's long but has no patch indicators, it might be incorrect
            return Err(ToolError::InvalidArgument(
                "Patch content doesn't appear to be in diff format. Please provide a valid unified diff.".to_string()
            ));
        }

        // Check for common patch format errors
        for line in &lines {
            // Check for malformed hunk headers
            if line.starts_with("@@") {
                if !line.contains("@@") || line.matches("@@").count() < 2 {
                    return Err(ToolError::InvalidArgument(format!(
                        "Malformed hunk header: {}",
                        line
                    )));
                }
            }
        }

        Ok(())
    }

    /// Enhanced edit_file with intelligent caching and performance monitoring
    /// Now supports both exact string replacement and fuzzy matching for more robust editing
    /// Also provides better error messages and proactive search when operations fail
    async fn edit_file(&self, args: Value) -> Result<Value> {
        let input: EditInput = serde_json::from_value(args).context("invalid edit_file args")?;
        let path = self.root.join(&input.path);
        let _ast_grep_lint = input.ast_grep_lint.unwrap_or(false);
        let _ast_grep_refactor = input.ast_grep_refactor.unwrap_or(false);

        // Check if file should be excluded by .vtagentgitignore
        if should_exclude_file(&path).await {
            return Err(anyhow!(
                "File '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        // Read existing content
        let content = tokio::fs::read_to_string(&path)
            .await
            .context(format!("failed to read file: {}", path.display()))?;

        // Apply text replacement
        let new_content = Self::safe_replace_text(&content, &input.old_string, &input.new_string)
            .map_err(|e| anyhow!("text replacement failed: {}", e))?;

        // Write updated content back to file
        tokio::fs::write(&path, &new_content)
            .await
            .context(format!("failed to write file: {}", path.display()))?;

        // Invalidate cache for this file
        FILE_CACHE.invalidate_prefix(&input.path).await;

        // Optionally run ast-grep lint/refactor
        let lint_results: Option<Vec<String>> = None;
        let refactor_results: Option<Vec<String>> = None;
        // TODO: Implement AST-grep lint/refactor when engine is available
        // if ast_grep_lint {
        //     lint_results = self.ast_grep_engine.lint_file(&path).await.ok();
        // }
        // if ast_grep_refactor {
        //     refactor_results = self.ast_grep_engine.refactor_file(&path).await.ok();
        // }

        Ok(
            json!({ "success": true, "lint_results": lint_results, "refactor_results": refactor_results }),
        )
    }

    /// Enhanced delete_file with intelligent caching and performance monitoring
    async fn delete_file(&self, args: Value) -> Result<Value> {
        let input: DeleteInput =
            serde_json::from_value(args).context("invalid delete_file args")?;
        let path = self.root.join(&input.path);
        let ast_grep_warn_pattern = input.ast_grep_warn_pattern.clone();

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

        // If ast_grep_warn_pattern is provided, scan file for matches and warn if found
        let ast_grep_matches: Option<Vec<String>> = None;
        if let Some(_pattern) = ast_grep_warn_pattern {
            // TODO: Implement AST-grep scanning when engine is available
            // ast_grep_matches = self.ast_grep_engine.extract_matches(&path, &pattern).await.ok();
            // if let Some(matches) = &ast_grep_matches {
            //     if !matches.is_empty() && !input.confirm {
            //         return Err(anyhow!(format!(
            //             "File contains important AST pattern matches. Confirm deletion explicitly. Matches: {:?}", matches
            //         )));
            //     }
            // }
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

        Ok(json!({ "success": true, "deleted": true, "ast_grep_matches": ast_grep_matches }))
    }

    /// Enhanced rp_search with debounce/cancellation logic
    /// Uses ripgrep for searching with enhanced features
    async fn rp_search(&self, args: Value) -> Result<Value> {
        let input: RgInput = serde_json::from_value(args).context("invalid rp_search args")?;
        let base_path = self.root.join(&input.path);

        // Check if path should be excluded by .vtagentgitignore
        if should_exclude_file(&base_path).await {
            return Err(anyhow!(
                "Path '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        // Notify the search manager of the new query (for debounce logic)
        self.rp_search_manager.on_user_query(input.pattern.clone());

        // Use ripgrep implementation
        let result = self.rg_search_with_ripgrep(&input).await?;

        Ok(result)
    }

    /// Search using ripgrep
    async fn rg_search_with_ripgrep(&self, input: &RgInput) -> Result<Value> {
        let base_path = self.root.join(&input.path);

        // Determine the working directory - it should be a directory, not a file
        let working_dir = if base_path.is_file() {
            // If base_path is a file, use its parent directory as the working directory
            base_path.parent().unwrap_or(&self.root).to_path_buf()
        } else {
            // If base_path is a directory, use it as the working directory
            base_path.clone()
        };

        // Build ripgrep command
        // Use full path to rg to avoid PATH issues
        let rg_path = env::var("RG_PATH").unwrap_or_else(|_| {
            // Check if rg exists in common locations, otherwise fallback to "rg"
            let common_paths = ["/opt/homebrew/bin/rg", "/usr/local/bin/rg", "/usr/bin/rg"];

            for path in &common_paths {
                if std::fs::metadata(path).is_ok() {
                    return path.to_string();
                }
            }

            // Fallback to just "rg"
            "rg".to_string()
        });
        let mut cmd = tokio::process::Command::new(&rg_path);

        // Smart pattern handling: Auto-detect if pattern should be literal
        let should_use_literal = if input.literal.is_some() {
            // User explicitly specified literal mode
            input.literal.unwrap_or(false)
        } else {
            // Auto-detect: if pattern contains regex special chars that are likely meant literally
            self.should_pattern_be_literal(&input.pattern)
        };

        cmd.arg("--json")
            .arg("--max-count")
            .arg(input.max_results.unwrap_or(1000).to_string())
            .arg("--context")
            .arg(input.context_lines.unwrap_or(0).to_string());

        // Add literal flag before pattern if needed
        if should_use_literal {
            cmd.arg("--fixed-strings");
        }

        cmd.arg(&input.pattern)
            .current_dir(&working_dir);

        // If base_path is a file, we need to pass it as an argument to ripgrep
        if base_path.is_file() {
            // Get the file name relative to the working directory
            if let Ok(file_name) = base_path.strip_prefix(&working_dir) {
                cmd.arg(file_name.to_string_lossy().as_ref());
            }
        }

        // Add case sensitivity flag if specified
        if let Some(case_sensitive) = input.case_sensitive {
            if case_sensitive {
                cmd.arg("--case-sensitive");
            } else {
                cmd.arg("--ignore-case");
            }
        }

        // Add glob pattern if specified
        if let Some(ref glob_pattern) = input.glob_pattern {
            cmd.arg("--glob").arg(glob_pattern);
        }

        // Add file type filter if specified
        if let Some(ref file_type) = input.file_type {
            // Map common abbreviations to ripgrep type names
            let rg_type = match file_type.as_str() {
                "rs" => "rust",
                "py" => "py",
                "js" => "js",
                "ts" => "ts",
                "tsx" => "tsx",
                "go" => "go",
                "java" => "java",
                "cpp" | "cc" | "cxx" => "cpp",
                "c" => "c",
                "h" | "hpp" => "c",
                "html" => "html",
                "css" => "css",
                "json" => "json",
                "yaml" | "yml" => "yaml",
                "toml" => "toml",
                "md" => "md",
                other => other, // Pass through for exact matches
            };
            cmd.arg("--type").arg(rg_type);
        }

        // Add hidden files flag if specified
        if input.include_hidden.unwrap_or(false) {
            cmd.arg("--hidden");
        }

        // Execute ripgrep with timeout
        let output =
            match tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output()).await {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => return Err(anyhow!("failed to execute ripgrep: {}", e)),
                Err(_) => return Err(anyhow!("ripgrep execution timed out")),
            };

        // Check if ripgrep executed successfully
        // Exit code 1 means no matches found, which is not an error
        if !output.status.success() && output.status.code() != Some(1) {
            return Err(anyhow!(
                "ripgrep failed with exit code: {:?}, stderr: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Parse ripgrep JSON output
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut matches = Vec::new();
        let mut file_count = 0;
        let mut current_file = String::new();

        // If no output and exit code is 1, return empty results
        if output_str.is_empty() && output.status.code() == Some(1) {
            return Ok(json!({
                "matches": matches,
                "file_count": file_count,
                "match_count": matches.len()
            }));
        }

        for line in output_str.lines() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                match value.get("type").and_then(|t| t.as_str()) {
                    Some("begin") => {
                        if let Some(data) = value.get("data") {
                            if let Some(path) = data.get("path") {
                                if let Some(text) = path.get("text") {
                                    current_file = text.as_str().unwrap_or("").to_string();
                                    file_count += 1;
                                }
                            }
                        }
                    }
                    Some("match") => {
                        if let Some(data) = value.get("data") {
                            if let Some(line_number) =
                                data.get("line_number").and_then(|n| n.as_u64())
                            {
                                if let Some(submatches) =
                                    data.get("submatches").and_then(|s| s.as_array())
                                {
                                    if let Some(first_match) = submatches.first() {
                                        if let Some(match_text) = first_match
                                            .get("match")
                                            .and_then(|m| m.get("text"))
                                            .and_then(|t| t.as_str())
                                        {
                                            // Get context lines
                                            let context_start = line_number.saturating_sub(
                                                input.context_lines.unwrap_or(0) as u64,
                                            );
                                            // For simplicity, we'll just store the match line and its context as the full content
                                            // A more complete implementation would fetch the actual context lines
                                            let context: Vec<String> = vec![match_text.to_string()];

                                            matches.push(json!({
                                                "path": current_file,
                                                "line": line_number,
                                                "content": match_text,
                                                "context": context,
                                                "context_start": context_start
                                            }));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(json!({
            "matches": matches,
            "file_count": file_count,
            "match_count": matches.len()
        }))
    }

    /// Enhanced code_search with intelligent caching and performance monitoring
    async fn code_search(&self, args: Value) -> Result<Value> {
        let input: RgInput = serde_json::from_value(args).context("invalid code_search args")?;
        self.rp_search(serde_json::to_value(input)?).await
    }

    /// Enhanced codebase_search with intelligent caching and performance monitoring
    async fn codebase_search(&self, args: Value) -> Result<Value> {
        let input: RgInput =
            serde_json::from_value(args).context("invalid codebase_search args")?;
        self.rp_search(serde_json::to_value(input)?).await
    }

    /// Run a terminal command with basic safety checks
    async fn run_terminal_command(&self, args: Value, is_pty: bool) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct TerminalCommandInput {
            command: Vec<String>,
            #[serde(default)]
            working_dir: Option<String>,
            #[serde(default)]
            timeout_secs: Option<u64>,
        }

        let input: TerminalCommandInput =
            serde_json::from_value(args).context("invalid terminal command args")?;

        if input.command.is_empty() {
            return Err(anyhow!("command array cannot be empty"));
        }

        let program = &input.command[0];
        let cmd_args = &input.command[1..];

        // Check for potentially problematic commands and suggest alternatives
        if let Some(suggestion) = self.suggest_better_tool(&input.command) {
            return Err(anyhow!(
                "Command '{}' might be slow or produce large output. Suggestion: {}",
                input.command.join(" "),
                suggestion
            ));
        }

        let mut cmd = tokio::process::Command::new(program);
        cmd.args(cmd_args);

        // Set working directory if provided
        if let Some(ref working_dir) = input.working_dir {
            let working_path = self.root.join(working_dir);
            cmd.current_dir(working_path);
        } else {
            cmd.current_dir(&self.root);
        }

        // For PTY commands, we would use rexpect, but for now we'll use regular process execution
        if is_pty {
            // TODO: Implement proper PTY support using rexpect
            // For now, we'll fall back to regular command execution
            println!(
                "PTY support not fully implemented, falling back to regular command execution"
            );
        }

        // Set up timeout (default 30 seconds, configurable)
        let timeout_duration = std::time::Duration::from_secs(
            input.timeout_secs.unwrap_or(30)
        );

        // Execute command with timeout
        let output = match tokio::time::timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(anyhow!("failed to execute command: {}", e)),
            Err(_) => return Err(anyhow!(
                "command '{}' timed out after {} seconds. Consider using more specific search tools like rp_search instead.",
                input.command.join(" "),
                timeout_duration.as_secs()
            )),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Limit output size to prevent memory issues (max 50KB per stream)
        const MAX_OUTPUT_SIZE: usize = 50 * 1024;
        let stdout_truncated = if stdout.len() > MAX_OUTPUT_SIZE {
            format!("{}... [truncated, {} bytes total]",
                   &stdout[..MAX_OUTPUT_SIZE], stdout.len())
        } else {
            stdout.to_string()
        };

        let stderr_truncated = if stderr.len() > MAX_OUTPUT_SIZE {
            format!("{}... [truncated, {} bytes total]",
                   &stderr[..MAX_OUTPUT_SIZE], stderr.len())
        } else {
            stderr.to_string()
        };

        Ok(json!({
            "success": output.status.success(),
            "code": output.status.code(),
            "stdout": stdout_truncated,
            "stderr": stderr_truncated
        }))
    }

    /// Suggest better tools for common problematic commands
    fn suggest_better_tool(&self, command: &[String]) -> Option<String> {
        if command.is_empty() {
            return None;
        }

        let program = &command[0];

        match program.as_str() {
            "grep" => {
                // Check if it's a recursive grep that could be slow
                if command.contains(&"-r".to_string()) || command.contains(&"-R".to_string()) {
                    Some("Use 'rp_search' tool instead for fast recursive searching with file type filtering".to_string())
                } else {
                    None
                }
            },
            "find" => {
                Some("Use 'list_files' or 'rp_search' tool instead for better performance and filtering".to_string())
            },
            "rg" => {
                Some("Use 'rp_search' tool instead which provides the same ripgrep functionality with better integration".to_string())
            },
            "ag" | "ack" => {
                Some("Use 'rp_search' tool instead for fast searching with file type filtering".to_string())
            },
            _ => None,
        }
    }

    /// Run a terminal command with basic safety checks (legacy)
    async fn run_terminal_cmd(&self, args: Value) -> Result<Value> {
        self.run_terminal_command(args, false).await
    }

    /// Run a command in a pseudo-terminal (PTY) with full terminal emulation
    async fn run_pty_cmd(&self, args: Value) -> Result<Value> {
        self.run_terminal_command(args, true).await
    }

    /// Run a PTY command with streaming output
    async fn run_pty_cmd_streaming(&self, args: Value) -> Result<Value> {
        self.run_terminal_command(args, true).await
    }

    /// Create a new PTY session
    async fn create_pty_session(&self, args: Value) -> Result<Value> {
        let _args = args;
        Ok(json!({ "success": false, "error": "pty sessions not implemented yet" }))
    }

    /// List active PTY sessions
    async fn list_pty_sessions(&self, args: Value) -> Result<Value> {
        let _args = args;
        Ok(json!({ "success": true, "sessions": [] }))
    }

    /// Close a PTY session
    async fn close_pty_session(&self, args: Value) -> Result<Value> {
        let _args = args;
        Ok(json!({ "success": false, "error": "pty sessions not implemented yet" }))
    }

    /// Search using AST-grep patterns
    async fn ast_grep_search(&self, args: Value) -> Result<Value> {
        // If AST-grep engine is available, use it; otherwise fall back to rp_search
        if let Some(ref engine) = self.ast_grep_engine {
            let args_obj = args
                .as_object()
                .ok_or_else(|| anyhow!("Invalid arguments"))?;
            let pattern = args_obj
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing pattern argument"))?;
            let path = args_obj.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let language = args_obj.get("language").and_then(|v| v.as_str());
            let context_lines = args_obj
                .get("context_lines")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let max_results = args_obj
                .get("max_results")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);

            return engine
                .search(pattern, path, language, context_lines, max_results)
                .await;
        }

        // Fall back to regular rp_search if AST-grep is not available
        self.rp_search(args).await
    }

    /// Transform code using AST-grep patterns
    async fn ast_grep_transform(&self, args: Value) -> Result<Value> {
        if let Some(ref engine) = self.ast_grep_engine {
            let args_obj = args
                .as_object()
                .ok_or_else(|| anyhow!("Invalid arguments"))?;
            let pattern = args_obj
                .get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing pattern argument"))?;
            let replacement = args_obj
                .get("replacement")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing replacement argument"))?;
            let path = args_obj.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let language = args_obj.get("language").and_then(|v| v.as_str());
            let preview_only = args_obj
                .get("preview_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let update_all = args_obj
                .get("update_all")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            return engine
                .transform(
                    pattern,
                    replacement,
                    path,
                    language,
                    preview_only,
                    update_all,
                )
                .await;
        }

        Ok(json!({ "success": false, "error": "ast-grep engine not available" }))
    }

    /// Lint code using AST-grep
    async fn ast_grep_lint(&self, args: Value) -> Result<Value> {
        if let Some(ref engine) = self.ast_grep_engine {
            let args_obj = args
                .as_object()
                .ok_or_else(|| anyhow!("Invalid arguments"))?;
            let path = args_obj.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let language = args_obj.get("language").and_then(|v| v.as_str());
            let severity_filter = args_obj.get("severity_filter").and_then(|v| v.as_str());

            return engine.lint(path, language, severity_filter, None).await;
        }

        Ok(json!({ "success": false, "error": "ast-grep engine not available" }))
    }

    /// Refactor code using AST-grep
    async fn ast_grep_refactor(&self, args: Value) -> Result<Value> {
        if let Some(ref engine) = self.ast_grep_engine {
            let args_obj = args
                .as_object()
                .ok_or_else(|| anyhow!("Invalid arguments"))?;
            let path = args_obj.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let language = args_obj.get("language").and_then(|v| v.as_str());
            let refactor_type = args_obj
                .get("refactor_type")
                .and_then(|v| v.as_str())
                .unwrap_or("all");

            return engine.refactor(path, language, refactor_type).await;
        }

        Ok(json!({ "success": false, "error": "ast-grep engine not available" }))
    }

    /// Fuzzy search functionality
    async fn fuzzy_search(&self, args: Value) -> Result<Value> {
        let args_obj = args
            .as_object()
            .ok_or_else(|| anyhow!("Invalid arguments"))?;
        let query = args_obj
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing query argument"))?;
        let path = args_obj.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let max_results = args_obj
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(50);
        let _threshold = args_obj
            .get("threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.6);
        let case_sensitive = args_obj
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // For fuzzy search, we'll use a more relaxed pattern matching
        // This is a simple implementation that uses regex with word boundaries
        // A more sophisticated implementation would use a dedicated fuzzy search library

        // Convert the query to a regex pattern that allows for some flexibility
        let mut regex_pattern = String::new();
        for ch in query.chars() {
            if ch.is_alphanumeric() {
                regex_pattern.push(ch);
                // Allow for optional characters between query characters for fuzzy matching
                regex_pattern.push_str(".*?");
            } else {
                regex_pattern.push_str(&regex::escape(&ch.to_string()));
            }
        }

        // Create ripgrep search input with the fuzzy pattern
        let rg_input = json!({
            "pattern": regex_pattern,
            "path": path,
            "max_results": max_results,
            "case_sensitive": case_sensitive,
        });

        self.rp_search(rg_input).await
    }

    /// Similarity-based search
    async fn similarity_search(&self, args: Value) -> Result<Value> {
        let args_obj = args
            .as_object()
            .ok_or_else(|| anyhow!("Invalid arguments"))?;
        let reference_file = args_obj
            .get("reference_file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing reference_file argument"))?;
        let search_path = args_obj
            .get("search_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let max_results = args_obj
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(20);
        let content_type = args_obj
            .get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        // Read the reference file to extract patterns
        let reference_path = self.root.join(reference_file);
        if !reference_path.exists() {
            return Err(anyhow!("Reference file not found: {}", reference_file));
        }

        let reference_content = tokio::fs::read_to_string(&reference_path)
            .await
            .context(format!("Failed to read reference file: {}", reference_file))?;

        // Extract patterns based on content type
        let patterns = match content_type {
            "structure" => extract_structure_patterns(&reference_content),
            "imports" => extract_import_patterns(&reference_content),
            "functions" => extract_function_patterns(&reference_content),
            _ => extract_all_patterns(&reference_content),
        };

        // Search for files containing similar patterns
        let mut all_matches = Vec::new();

        for pattern in patterns.iter().take(5) {
            // Limit to top 5 patterns
            // Create a search pattern for this extracted pattern
            let search_pattern = format!(".*{}.*", regex::escape(pattern));

            // Create ripgrep search input
            let rg_input = json!({
                "pattern": search_pattern,
                "path": search_path,
                "max_results": max_results / patterns.len().max(1),
            });

            if let Ok(results) = self.rp_search(rg_input).await {
                if let Some(matches) = results.get("matches").and_then(|m| m.as_array()) {
                    for m in matches {
                        all_matches.push(m.clone());
                    }
                }
            }
        }

        // Deduplicate matches by file path
        let mut unique_matches = Vec::new();
        let mut seen_files = std::collections::HashSet::new();

        for m in all_matches {
            if let Some(path) = m.get("path").and_then(|p| p.as_str()) {
                if seen_files.insert(path.to_string()) {
                    unique_matches.push(m);
                }
            }
        }

        Ok(json!({
            "success": true,
            "matches": unique_matches,
            "count": unique_matches.len(),
            "reference_file": reference_file,
            "content_type": content_type
        }))
    }

    /// Multi-pattern search
    async fn multi_pattern_search(&self, args: Value) -> Result<Value> {
        let args_obj = args
            .as_object()
            .ok_or_else(|| anyhow!("Invalid arguments"))?;
        let patterns = args_obj
            .get("patterns")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing patterns argument"))?;
        let logic = args_obj
            .get("logic")
            .and_then(|v| v.as_str())
            .unwrap_or("AND");
        let path = args_obj.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let max_results = args_obj
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100);
        let context_lines = args_obj
            .get("context_lines")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(2);

        // Combine patterns based on logic
        let combined_pattern = match logic {
            "AND" => {
                // For AND logic, we need to find files that contain all patterns
                // This is more complex and requires multiple searches
                let mut all_matches = Vec::new();

                for pattern in patterns {
                    if let Some(pattern_str) = pattern.as_str() {
                        let rg_input = json!({
                            "pattern": pattern_str,
                            "path": path,
                            "max_results": max_results,
                            "context_lines": context_lines,
                        });

                        if let Ok(results) = self.rp_search(rg_input).await {
                            if let Some(matches) = results.get("matches").and_then(|m| m.as_array())
                            {
                                for m in matches {
                                    all_matches.push(m.clone());
                                }
                            }
                        }
                    }
                }

                // Filter matches to only include files that contain all patterns
                let mut file_counts = std::collections::HashMap::new();
                for m in &all_matches {
                    if let Some(file_path) = m.get("path").and_then(|p| p.as_str()) {
                        *file_counts.entry(file_path.to_string()).or_insert(0) += 1;
                    }
                }

                let required_count = patterns.len();
                let filtered_matches: Vec<_> = all_matches
                    .into_iter()
                    .filter(|m| {
                        if let Some(file_path) = m.get("path").and_then(|p| p.as_str()) {
                            file_counts
                                .get(file_path)
                                .map_or(false, |&count| count == required_count)
                        } else {
                            false
                        }
                    })
                    .take(max_results)
                    .collect();

                return Ok(json!({
                    "success": true,
                    "matches": filtered_matches,
                    "count": filtered_matches.len(),
                    "logic": logic
                }));
            }
            "OR" => {
                // For OR logic, create a regex that matches any of the patterns
                let pattern_strings: Vec<_> = patterns.iter().filter_map(|p| p.as_str()).collect();

                if pattern_strings.is_empty() {
                    return Err(anyhow!("No valid patterns provided"));
                }

                format!("({})", pattern_strings.join("|"))
            }
            "NOT" => {
                // For NOT logic, we'll search for the first pattern and exclude matches that contain other patterns
                if patterns.is_empty() {
                    return Err(anyhow!("No patterns provided for NOT logic"));
                }

                if let Some(first_pattern) = patterns.first().and_then(|p| p.as_str()) {
                    first_pattern.to_string()
                } else {
                    return Err(anyhow!("Invalid first pattern for NOT logic"));
                }
            }
            _ => {
                return Err(anyhow!("Invalid logic value: {}", logic));
            }
        };

        // Create ripgrep search input with the combined pattern
        let rg_input = json!({
            "pattern": combined_pattern,
            "path": path,
            "max_results": max_results,
            "context_lines": context_lines,
        });

        self.rp_search(rg_input).await
    }

    /// Extract text patterns from files
    async fn extract_text_patterns(&self, args: Value) -> Result<Value> {
        let args_obj = args
            .as_object()
            .ok_or_else(|| anyhow!("Invalid arguments"))?;
        let path = args_obj.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let pattern_types = args_obj
            .get("pattern_types")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing pattern_types argument"))?;
        let max_results = args_obj
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(200);

        let mut all_patterns = Vec::new();

        // Use ripgrep to search for patterns
        for pattern_type in pattern_types {
            if let Some(pattern_str) = pattern_type.as_str() {
                let pattern = match pattern_str {
                    "urls" => r"https?://[^\\s]+",
                    "emails" => r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}",
                    "todos" => r"TODO:?[^\n]*",
                    "fixmes" => r"FIXME:?[^\n]*",
                    "credentials" => r"(password|passwd|pwd|api_key|secret)\\s*[:=]\\s*[^\\s,;]+",
                    "ip_addresses" => r"\\b(?:[0-9]{1,3}\\.){3}[0-9]{1,3}\\b",
                    "phone_numbers" => {
                        r"(\\+?1[-.\\s]?)?\\(?[0-9]{3}\\)?[-.\\s]?[0-9]{3}[-.\\s]?[0-9]{4}"
                    }
                    "file_paths" => r"[a-zA-Z0-9_\\-/\\\\]+\\.[a-zA-Z0-9]+",
                    _ => continue,
                };

                // Create ripgrep search input
                let rg_input = json!({
                    "pattern": pattern,
                    "path": path,
                    "max_results": max_results,
                });

                if let Ok(results) = self.rp_search(rg_input).await {
                    if let Some(matches) = results.get("matches").and_then(|m| m.as_array()) {
                        for m in matches {
                            all_patterns.push(m.clone());
                        }
                    }
                }
            }
        }

        Ok(json!({
            "success": true,
            "patterns": all_patterns,
            "count": all_patterns.len()
        }))
    }

    /// Find and return the path to the main configuration file (vtagent.toml)
    async fn find_config_file(&self, _args: Value) -> Result<Value> {
        let config_paths = [
            "vtagent.toml",
            ".vtagent.toml",
            "config/vtagent.toml",
            ".config/vtagent.toml",
        ];

        for path_str in &config_paths {
            let path = self.root.join(path_str);
            if path.exists() {
                return Ok(json!({
                    "success": true,
                    "config_path": path_str,
                    "full_path": path.display().to_string()
                }));
            }
        }

        // If not found in standard locations, search for any .toml file with vtagent in the name
        let search_result = self
            .rp_search(json!({
                "pattern": "vtagent.*\\.toml",
                "path": ".",
                "max_results": 5
            }))
            .await?;

        if let Some(matches) = search_result.get("matches").and_then(|m| m.as_array()) {
            if let Some(first_match) = matches.first() {
                if let Some(path) = first_match.get("path").and_then(|p| p.as_str()) {
                    return Ok(json!({
                        "success": true,
                        "config_path": path,
                        "full_path": self.root.join(path).display().to_string()
                    }));
                }
            }
        }

        Ok(json!({
            "success": false,
            "error": "No vtagent.toml configuration file found in standard locations",
            "suggested_locations": config_paths
        }))
    }

    /// Recursively search for files in the workspace
    async fn recursive_file_search(&self, args: Value) -> Result<Value> {
        let args_obj = args.as_object().ok_or_else(|| anyhow!("Invalid arguments"))?;
        let pattern = args_obj
            .get("pattern")
            .and_then(|v| v.as_str());
        let path = args_obj
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let max_results = args_obj
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100);
        let include_hidden = args_obj
            .get("include_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let file_extensions = args_obj
            .get("file_extensions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            });

        // Create file searcher configuration
        let mut config = FileSearchConfig::default();
        config.max_results = max_results;
        config.include_hidden = include_hidden;

        if let Some(extensions) = file_extensions {
            config.include_extensions.extend(extensions);
        }

        let search_path = self.root.join(path);
        let searcher = FileSearcher::new(search_path, config);

        match searcher.search_files(pattern) {
            Ok(results) => Ok(FileSearcher::results_to_json(results)),
            Err(e) => Ok(json!({
                "success": false,
                "error": format!("Failed to search files: {}", e)
            })),
        }
    }

    /// Search for files containing specific content
    async fn search_files_with_content(&self, args: Value) -> Result<Value> {
        let args_obj = args.as_object().ok_or_else(|| anyhow!("Invalid arguments"))?;
        let content_pattern = args_obj
            .get("content_pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing content_pattern argument"))?;
        let file_pattern = args_obj
            .get("file_pattern")
            .and_then(|v| v.as_str());
        let path = args_obj
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let max_results = args_obj
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(50);
        let case_sensitive = args_obj
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Create file searcher configuration
        let mut config = FileSearchConfig::default();
        config.max_results = max_results;

        // Handle case sensitivity by converting to lowercase if needed
        let search_pattern = if case_sensitive {
            content_pattern.to_string()
        } else {
            content_pattern.to_lowercase()
        };

        let search_path = self.root.join(path);
        let searcher = FileSearcher::new(search_path, config);

        match searcher.search_files_with_content(
            &search_pattern,
            file_pattern,
        ) {
            Ok(results) => Ok(FileSearcher::results_to_json(results)),
            Err(e) => Ok(json!({
                "success": false,
                "error": format!("Failed to search files with content: {}", e)
            })),
        }
    }

    /// Find a specific file by name recursively
    async fn find_file_by_name(&self, args: Value) -> Result<Value> {
        let args_obj = args.as_object().ok_or_else(|| anyhow!("Invalid arguments"))?;
        let file_name = args_obj
            .get("file_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing file_name argument"))?;
        let path = args_obj
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let case_sensitive = args_obj
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Create file searcher
        let config = FileSearchConfig::default();
        let search_path = self.root.join(path);
        let searcher = FileSearcher::new(search_path, config);

        match searcher.find_file_by_name(file_name) {
            Ok(Some(found_path)) => {
                // Convert to relative path
                if let Ok(relative_path) = found_path.strip_prefix(&self.root) {
                    Ok(json!({
                        "success": true,
                        "found": true,
                        "path": relative_path.to_string_lossy()
                    }))
                } else {
                    Ok(json!({
                        "success": true,
                        "found": true,
                        "path": found_path.to_string_lossy()
                    }))
                }
            },
            Ok(None) => Ok(json!({
                "success": true,
                "found": false,
                "message": format!("File '{}' not found", file_name)
            })),
            Err(e) => Ok(json!({
                "success": false,
                "error": format!("Failed to find file: {}", e)
            })),
        }
    }
}

// Helper functions for text pattern extraction

// === SEARCH TEXT TOOLS - Simple Implementations ===

// TODO: Implement AST-grep lint/refactor when engine is available
// let lint_results: Option<Vec<String>> = None;
// let refactor_results: Option<Vec<String>> = None;
// if ast_grep_lint {
//     lint_results = self.ast_grep_engine.lint_file(&path).await.ok();
// === SEARCH TEXT TOOLS - Simple Implementations ===

// Helper functions for text pattern extraction
fn extract_import_patterns(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ")
            || trimmed.starts_with("from ")
            || trimmed.starts_with("use ")
            || trimmed.starts_with("#include")
        {
            patterns.push(trimmed.to_string());
        }
    }
    patterns
}

fn extract_function_patterns(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("def ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("fn ")
            || trimmed.contains("function(")
        {
            // Extract function names
            if let Some(func_name) = extract_function_name(trimmed) {
                patterns.push(func_name);
            }
        }
    }
    patterns.into_iter().take(10).collect()
}

fn extract_structure_patterns(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("class ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("enum ")
        {
            if let Some(name) = extract_type_name(trimmed) {
                patterns.push(name);
            }
        }
    }
    patterns.into_iter().take(10).collect()
}

fn extract_all_patterns(content: &str) -> Vec<String> {
    let mut patterns = extract_import_patterns(content);
    patterns.extend(extract_function_patterns(content));
    patterns.extend(extract_structure_patterns(content));
    patterns.into_iter().take(15).collect()
}

fn extract_module_name(line: &str) -> Option<String> {
    // Simple extraction - could be more sophisticated
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() > 1 {
        Some(words[1].trim_matches(&['"', '\'', ';'][..]).to_string())
    } else {
        None
    }
}

fn extract_function_name(line: &str) -> Option<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() > 1 {
        let name = words[1].split('(').next()?;
        Some(name.to_string())
    } else {
        None
    }
}

fn extract_type_name(line: &str) -> Option<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() > 1 {
        Some(words[1].split(':').next()?.split('{').next()?.to_string())
    } else {
        None
    }
}

async fn process_ripgrep_output(stdout: &[u8], search_type: &str) -> Vec<Value> {
    let stdout_str = String::from_utf8_lossy(stdout);
    let mut results = Vec::new();

    for line in stdout_str.lines() {
        if let Ok(result) = serde_json::from_str::<Value>(line) {
            if result["type"] == "match" {
                let data = &result["data"];
                results.push(json!({
                    "file": data["path"]["text"],
                    "line_number": data["line_number"],
                    "content": data["lines"]["text"],
                    "search_type": search_type
                }));
            }
        }
    }

    results
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

/// Build function declarations filtered by capability level
pub fn build_function_declarations_for_level(level: crate::types::CapabilityLevel) -> Vec<FunctionDeclaration> {
    let all_declarations = build_function_declarations();

    match level {
        crate::types::CapabilityLevel::Basic => vec![],
        crate::types::CapabilityLevel::FileReading => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "read_file")
                .collect()
        },
        crate::types::CapabilityLevel::FileListing => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "read_file" || fd.name == "list_files")
                .collect()
        },
        crate::types::CapabilityLevel::Bash => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "read_file" || fd.name == "list_files" || fd.name == "bash")
                .collect()
        },
        crate::types::CapabilityLevel::Editing => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "read_file" || fd.name == "list_files" || fd.name == "bash" || fd.name == "edit_file")
                .collect()
        },
        crate::types::CapabilityLevel::CodeSearch => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "read_file" || fd.name == "list_files" || fd.name == "bash" || fd.name == "edit_file" || fd.name == "code_search")
                .collect()
        },
    }
}
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
            name: "rp_search".to_string(),
            description: "Enhanced ripgrep search with smart literal detection and debounce support. Automatically detects when patterns should be treated as literal text (e.g., 'fn main(' or 'struct MyStruct') to avoid regex parsing errors.".to_string(),
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
                    "max_results": {"type": "integer", "description": "Maximum number of matches to return", "default": 1000},
                    "file_type": {"type": "string", "description": "Filter by file type: 'rs' (Rust), 'py' (Python), 'js' (JavaScript), 'ts' (TypeScript), 'go', 'java', 'cpp', 'c', etc."}
                },
                "required": ["pattern"]
            }),
        },
        FunctionDeclaration {
            name: "list_files".to_string(),
            description: "List files and directories in a given path. Optionally filter by AST pattern matches.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to list files from"},
                    "max_items": {"type": "integer", "description": "Maximum number of items to return", "default": 1000},
                    "include_hidden": {"type": "boolean", "description": "Include hidden files", "default": false},
                    "ast_grep_pattern": {"type": "string", "description": "Optional AST pattern to filter files (only files containing this pattern will be listed)"}
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "read_file".to_string(),
            description: "Read content from a file with intelligent path resolution. Tries multiple path variations if the exact path isn't found (e.g., 'main.rs' will search in 'src/', current directory, etc.). Optionally extract AST pattern matches.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to read (can be relative, filename only, or absolute). System will intelligently resolve the path by trying common locations."},
                    "max_bytes": {"type": "integer", "description": "Maximum bytes to read", "default": null},
                    "encoding": {"type": "string", "description": "Text encoding", "default": "utf-8"},
                    "ast_grep_pattern": {"type": "string", "description": "Optional AST pattern to extract matches from the file content"}
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "write_file".to_string(),
            description: "Write content to a file with various modes. Optionally run AST-grep lint/refactor after writing.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to write"},
                    "content": {"type": "string", "description": "Content to write to the file (or patch in unified diff format for patch mode)"},
                    "encoding": {"type": "string", "description": "Text encoding", "default": "utf-8"},
                    "mode": {"type": "string", "description": "Write mode: overwrite, append, skip_if_exists, or patch", "default": "overwrite", "enum": ["overwrite", "append", "skip_if_exists", "patch"]},
                    "ast_grep_lint": {"type": "boolean", "description": "Run AST-grep lint analysis after writing", "default": false},
                    "ast_grep_refactor": {"type": "boolean", "description": "Get refactoring suggestions after writing", "default": false}
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "edit_file".to_string(),
            description: "Edit a file by replacing text. Optionally run AST-grep lint/refactor after editing.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to edit"},
                    "old_string": {"type": "string", "description": "Text to replace (exact match preferred, fuzzy matching as fallback)"},
                    "new_string": {"type": "string", "description": "Replacement text"},
                    "encoding": {"type": "string", "description": "Text encoding", "default": "utf-8"},
                    "ast_grep_lint": {"type": "boolean", "description": "Run AST-grep lint analysis after editing", "default": false},
                    "ast_grep_refactor": {"type": "boolean", "description": "Get refactoring suggestions after editing", "default": false}
                },
                "required": ["path", "old_string", "new_string"]
            }),
        },
        FunctionDeclaration {
            name: "delete_file".to_string(),
            description: "Delete a file in the workspace. Optionally warn if the file contains important AST patterns.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to delete"},
                    "confirm": {"type": "boolean", "description": "Must be true to confirm deletion", "default": false},
                    "ast_grep_warn_pattern": {"type": "string", "description": "Optional AST pattern to check for important code before deletion (e.g., 'function $name($args) { $$$ }')"}
                },
                "required": ["path", "confirm"]
            }),
        },

        FunctionDeclaration {
            name: "run_terminal_cmd".to_string(),
            description: "Run a terminal command with timeout and safety checks. For search operations, prefer 'rp_search' tool for better performance. Automatically suggests alternatives for slow commands like 'grep -r'.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "array", "items": {"type": "string"}, "description": "Program + args as array"},
                    "working_dir": {"type": "string", "description": "Working directory relative to workspace"},
                    "timeout_secs": {"type": "integer", "description": "Command timeout in seconds (default: 30)", "default": 30}
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
        FunctionDeclaration {
            name: "ast_grep_search".to_string(),
            description: "Advanced syntax-aware code search using AST patterns. More powerful than regex search for structural code queries.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "AST pattern to search for (e.g., 'console.log($msg)' or 'function $name($params) { $$$ }')"},
                    "path": {"type": "string", "description": "File or directory path to search in", "default": "."},
                    "language": {"type": "string", "description": "Programming language (auto-detected if not specified)", "enum": ["rust", "python", "javascript", "typescript", "tsx", "go", "java", "cpp", "c", "html", "css", "json"]},
                    "context_lines": {"type": "integer", "description": "Number of context lines to show around matches", "default": 2},
                    "max_results": {"type": "integer", "description": "Maximum number of results to return", "default": 100}
                },
                "required": ["pattern"]
            }),
        },
        FunctionDeclaration {
            name: "ast_grep_transform".to_string(),
            description: "Transform code using AST-based pattern matching and replacement. Safer than regex for structural code changes.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "AST pattern to match (e.g., 'console.log($msg)')"},
                    "replacement": {"type": "string", "description": "Replacement pattern (e.g., '// console.log($msg)')"},
                    "path": {"type": "string", "description": "File or directory path to transform", "default": "."},
                    "language": {"type": "string", "description": "Programming language (auto-detected if not specified)", "enum": ["rust", "python", "javascript", "typescript", "tsx", "go", "java", "cpp", "c", "html", "css", "json"]},
                    "preview_only": {"type": "boolean", "description": "Show preview without applying changes", "default": true}
                },
                "required": ["pattern", "replacement"]
            }),
        },
        FunctionDeclaration {
            name: "ast_grep_lint".to_string(),
            description: "Lint code using AST-based rules to find potential issues and anti-patterns.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File or directory path to lint", "default": "."},
                    "language": {"type": "string", "description": "Programming language (auto-detected if not specified)", "enum": ["rust", "python", "javascript", "typescript", "tsx", "go", "java", "cpp", "c", "html", "css", "json"]},
                    "severity_filter": {"type": "string", "description": "Minimum severity to report", "default": "warning", "enum": ["error", "warning", "info"]}
                },
                "required": []
            }),
        },
        FunctionDeclaration {
            name: "ast_grep_refactor".to_string(),
            description: "Get intelligent refactoring suggestions using common code patterns and best practices.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File or directory path to analyze for refactoring opportunities", "default": "."},
                    "language": {"type": "string", "description": "Programming language (auto-detected if not specified)", "enum": ["rust", "python", "javascript", "typescript", "tsx", "go", "java", "cpp", "c", "html", "css", "json"]},
                    "refactor_type": {"type": "string", "description": "Type of refactoring to suggest", "enum": ["extract_function", "remove_console_logs", "simplify_conditions", "extract_constants", "modernize_syntax", "all"], "default": "all"}
                },
                "required": []
            }),
        },
        FunctionDeclaration {
            name: "fuzzy_search".to_string(),
            description: "Advanced fuzzy text search that finds approximate matches across files. Good for finding content when exact terms are unknown.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query to match approximately"},
                    "path": {"type": "string", "description": "Directory path to search in", "default": "."},
                    "max_results": {"type": "integer", "description": "Maximum number of results to return", "default": 50},
                    "threshold": {"type": "number", "description": "Similarity threshold (0.0 to 1.0)", "default": 0.6},
                    "case_sensitive": {"type": "boolean", "description": "Whether search should be case sensitive", "default": false}
                },
                "required": ["query"]
            }),
        },
        FunctionDeclaration {
            name: "similarity_search".to_string(),
            description: "Find files with similar content structure, imports, functions, or patterns. Useful for finding related or similar code files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "reference_file": {"type": "string", "description": "Path to the reference file to find similar files to"},
                    "search_path": {"type": "string", "description": "Directory path to search in", "default": "."},
                    "max_results": {"type": "integer", "description": "Maximum number of results to return", "default": 20},
                    "content_type": {"type": "string", "description": "Type of similarity to search for", "enum": ["structure", "imports", "functions", "all"], "default": "all"}
                },
                "required": ["reference_file"]
            }),
        },
        FunctionDeclaration {
            name: "multi_pattern_search".to_string(),
            description: "Search using multiple patterns with boolean logic (AND, OR, NOT). Powerful for complex search requirements.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "patterns": {"type": "array", "items": {"type": "string"}, "description": "List of search patterns"},
                    "logic": {"type": "string", "description": "Boolean logic to apply", "enum": ["AND", "OR", "NOT"], "default": "AND"},
                    "path": {"type": "string", "description": "Directory path to search in", "default": "."},
                    "max_results": {"type": "integer", "description": "Maximum number of results to return", "default": 100},
                    "context_lines": {"type": "integer", "description": "Number of context lines around matches", "default": 2}
                },
                "required": ["patterns"]
            }),
        },
        FunctionDeclaration {
            name: "extract_text_patterns".to_string(),
            description: "Extract and categorize specific text patterns like URLs, emails, TODOs, credentials, etc. Useful for code audits and analysis.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path to search in", "default": "."},
                    "pattern_types": {"type": "array", "items": {"type": "string", "enum": ["urls", "emails", "todos", "fixmes", "credentials", "ip_addresses", "phone_numbers", "file_paths"]}, "description": "Types of patterns to extract"},
                    "max_results": {"type": "integer", "description": "Maximum number of results to return", "default": 200}
                },
                "required": ["pattern_types"]
            }),
        },
        FunctionDeclaration {
            name: "find_config_file".to_string(),
            description: "Find and return the path to the main VTAgent configuration file (vtagent.toml). Use this when user mentions 'config', 'vtconfig', 'settings', or wants to modify configuration.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        FunctionDeclaration {
            name: "recursive_file_search".to_string(),
            description: "Recursively search for files in the workspace by name pattern. More powerful than simple file listing for finding specific files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "File name pattern to search for (supports wildcards)"},
                    "path": {"type": "string", "description": "Directory path to search in", "default": "."},
                    "max_results": {"type": "integer", "description": "Maximum number of results to return", "default": 100},
                    "include_hidden": {"type": "boolean", "description": "Include hidden files and directories", "default": false},
                    "file_extensions": {"type": "array", "items": {"type": "string"}, "description": "Limit search to specific file extensions (e.g., ['rs', 'py', 'js'])"}
                },
                "required": []
            }),
        },
        FunctionDeclaration {
            name: "search_files_with_content".to_string(),
            description: "Search for files containing specific content patterns. Useful for finding files with specific code patterns or text.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "content_pattern": {"type": "string", "description": "Text pattern to search for in file contents"},
                    "file_pattern": {"type": "string", "description": "Optional file name pattern to limit search (e.g., '*.rs')"},
                    "path": {"type": "string", "description": "Directory path to search in", "default": "."},
                    "max_results": {"type": "integer", "description": "Maximum number of results to return", "default": 50},
                    "case_sensitive": {"type": "boolean", "description": "Whether search should be case sensitive", "default": true}
                },
                "required": ["content_pattern"]
            }),
        },
        FunctionDeclaration {
            name: "find_file_by_name".to_string(),
            description: "Find a specific file by exact name match recursively through the workspace.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_name": {"type": "string", "description": "Exact file name to find"},
                    "path": {"type": "string", "description": "Directory path to search in", "default": "."},
                    "case_sensitive": {"type": "boolean", "description": "Whether search should be case sensitive", "default": true}
                },
                "required": ["file_name"]
            }),
        },
        FunctionDeclaration {
            name: "batch_file_operations".to_string(),
            description: "Perform batch operations on multiple files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "operations": {"type": "array", "items": {"type": "object"}, "description": "List of file operations to perform"}
                },
                "required": ["operations"]
            }),
        },
        FunctionDeclaration {
            name: "extract_dependencies".to_string(),
            description: "Extract project dependencies from configuration files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the project directory", "default": "."}
                },
                "required": []
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
