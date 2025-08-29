//! Enhanced tools module with modern coding agent capabilities
//!
//! This module provides advanced tools for intelligent code analysis,
//! refactoring suggestions, and project insights.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use walkdir::WalkDir;

// Performance optimization imports
use dashmap::DashMap;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use lru::LruCache;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::time::Instant;

/// Performance-optimized file operation cache
struct FileOperationCache {
    /// LRU cache for file contents
    file_cache: Arc<RwLock<LruCache<String, CachedFile>>>,
    /// Compressed cache for large files
    compressed_cache: Arc<DashMap<String, CompressedFile>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

/// Cached file entry
#[derive(Debug, Clone)]
struct CachedFile {
    content: String,
    timestamp: Instant,
    access_count: usize,
}

/// Compressed file entry for large files
#[derive(Debug, Clone)]
struct CompressedFile {
    compressed_data: Vec<u8>,
    original_size: usize,
    timestamp: Instant,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
struct CacheStats {
    hits: usize,
    misses: usize,
    evictions: usize,
}

impl FileOperationCache {
    /// Create a new cache with specified capacity
    fn new(cache_size: usize) -> Self {
        let cache_capacity = NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            file_cache: Arc::new(RwLock::new(LruCache::new(cache_capacity))),
            compressed_cache: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get file content from cache
    async fn get_file(&self, path: &str) -> Option<String> {
        // Check regular cache first
        {
            let mut cache = self.file_cache.write().await;
            if let Some(entry) = cache.get(path) {
                let mut stats = self.stats.write().await;
                stats.hits += 1;
                return Some(entry.content.clone());
            }
        }

        // Check compressed cache
        if let Some(compressed_entry) = self.compressed_cache.get(path) {
            let mut decoder = GzDecoder::new(&compressed_entry.compressed_data[..]);
            let mut decompressed_content = String::new();
            if decoder.read_to_string(&mut decompressed_content).is_ok() {
                let mut stats = self.stats.write().await;
                stats.hits += 1;
                return Some(decompressed_content);
            }
        }

        // Cache miss
        {
            let mut stats = self.stats.write().await;
            stats.misses += 1;
        }
        None
    }

    /// Put file content into cache
    async fn put_file(&self, path: String, content: String) {
        // For large files, compress and store in compressed cache
        if content.len() > 100_000 { // 100KB threshold
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            if encoder.write_all(content.as_bytes()).is_ok() {
                if let Ok(compressed_data) = encoder.finish() {
                    self.compressed_cache.insert(path.clone(), CompressedFile {
                        compressed_data,
                        original_size: content.len(),
                        timestamp: Instant::now(),
                    });
                    return;
                }
            }
        }

        // For smaller files, store in regular cache
        let entry = CachedFile {
            content,
            timestamp: Instant::now(),
            access_count: 1,
        };

        let mut cache = self.file_cache.write().await;
        cache.put(path, entry);
    }

    /// Get cache statistics
    async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Clear all caches
    async fn clear(&self) {
        let mut cache = self.file_cache.write().await;
        cache.clear();
        self.compressed_cache.clear();
        
        let mut stats = self.stats.write().await;
        stats.hits = 0;
        stats.misses = 0;
        stats.evictions = 0;
    }
}

/// Global file operation cache
static FILE_CACHE: Lazy<FileOperationCache> = Lazy::new(|| FileOperationCache::new(200));

/// Parallel file processor for improved performance
struct ParallelFileProcessor;

impl ParallelFileProcessor {
    /// Process multiple files in parallel
    fn process_files_parallel<T, F>(files: Vec<String>, processor: F) -> Vec<T>
    where
        T: Send,
        F: Fn(String) -> T + Send + Sync,
    {
        files.into_par_iter().map(processor).collect()
    }

    /// Process directory entries in parallel
    fn process_directory_parallel<F>(path: &str, processor: F) -> Result<Vec<Value>>
    where
        F: Fn(&Path) -> Result<Value> + Send + Sync,
    {
        let entries: Vec<_> = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .collect();

        // Process in parallel
        let results: Vec<Result<Value>> = entries
            .into_par_iter()
            .map(|entry| processor(entry.path()))
            .collect();

        // Collect successful results
        let mut successful = Vec::new();
        for result in results {
            if let Ok(value) = result {
                successful.push(value);
            }
        }
        
        Ok(successful)
    }
}

/// Get intelligent code insights and suggestions
pub async fn get_code_intelligence(args: Value) -> Result<Value> {
    #[derive(Deserialize)]
    struct Input {
        path: String,
        line: Option<usize>,
        column: Option<usize>,
        context_lines: Option<usize>,
    }
    let input: Input = serde_json::from_value(args).context("invalid get_code_intelligence args")?;
    let context_lines = input.context_lines.unwrap_or(3);

    // Try to get file content from cache first
    let content = if let Some(cached_content) = FILE_CACHE.get_file(&input.path).await {
        cached_content
    } else {
        // Read file content and cache it
        let content = fs::read_to_string(&input.path)?;
        FILE_CACHE.put_file(input.path.clone(), content.clone()).await;
        content
    };
    
    let lines: Vec<&str> = content.lines().collect();

    let line = input.line.unwrap_or(0).min(lines.len().saturating_sub(1));
    let start_line = line.saturating_sub(context_lines);
    let end_line = (line + context_lines + 1).min(lines.len());

    let context = lines[start_line..end_line].join("\n");

    // Analyze with tree-sitter if available
    let mut insights = Vec::new();

    // Basic analysis without tree-sitter for now
    let total_lines = lines.len();
    let code_lines = lines.iter().filter(|l| !l.trim().is_empty() && !l.trim().starts_with("//")).count();

    insights.push(json!({
        "type": "statistics",
        "data": {
            "total_lines": total_lines,
            "code_lines": code_lines,
            "comment_ratio": if total_lines > 0 { (total_lines - code_lines) as f64 / total_lines as f64 } else { 0.0 }
        },
        "description": format!("File contains {} lines of code", code_lines)
    }));

    Ok(json!({
        "path": input.path,
        "line": line,
        "context": context,
        "insights": insights,
        "suggestions": [
            "Consider extracting this function for better readability",
            "This variable could be made immutable",
            "Consider using an enum instead of multiple boolean flags"
        ]
    }))
}

/// Suggest refactoring improvements
pub async fn suggest_refactoring(args: Value) -> Result<Value> {
    #[derive(Deserialize)]
    struct Input {
        path: String,
        focus_area: Option<String>,
    }
    let input: Input = serde_json::from_value(args).context("invalid suggest_refactoring args")?;

    // Try to get file content from cache first
    let content = if let Some(cached_content) = FILE_CACHE.get_file(&input.path).await {
        cached_content
    } else {
        // Read file content and cache it
        let content = fs::read_to_string(&input.path)?;
        FILE_CACHE.put_file(input.path.clone(), content.clone()).await;
        content
    };
    
    let lines = content.lines().count();

    let mut suggestions = Vec::new();

    // Basic heuristics for refactoring suggestions
    if lines > 100 {
        suggestions.push(json!({
            "type": "extract_functions",
            "priority": "high",
            "description": "File is quite long, consider breaking it into smaller functions",
            "estimated_effort": "medium"
        }));
    }

    if content.contains("TODO") || content.contains("FIXME") {
        suggestions.push(json!({
            "type": "address_todos",
            "priority": "medium",
            "description": "Found TODO/FIXME comments that should be addressed",
            "estimated_effort": "low"
        }));
    }

    // Check for long functions (simple heuristic)
    let functions: Vec<_> = content.match_indices("fn ").collect();
    if functions.len() > 5 {
        suggestions.push(json!({
            "type": "consider_module_split",
            "priority": "medium",
            "description": "Many functions in one file, consider splitting into modules",
            "estimated_effort": "high"
        }));
    }

    Ok(json!({
        "path": input.path,
        "file_size": content.len(),
        "line_count": lines,
        "suggestions": suggestions,
        "summary": format!("Found {} refactoring opportunities", suggestions.len())
    }))
}

/// Analyze project structure and provide insights
pub async fn analyze_project_structure(args: Value) -> Result<Value> {
    #[derive(Deserialize, Default)]
    struct Input {
        include_hidden: Option<bool>,
        max_depth: Option<usize>,
    }
    let input: Input = serde_json::from_value(args).context("invalid analyze_project_structure args")?;
    let include_hidden = input.include_hidden.unwrap_or(false);
    let max_depth = input.max_depth.unwrap_or(3);

    let mut structure: HashMap<String, usize> = HashMap::new();
    let mut total_files = 0;
    let mut total_dirs = 0;

    // Use parallel processing for better performance
    let entries: Vec<_> = WalkDir::new(".")
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();
        
    // Process entries in parallel
    let results: Vec<_> = entries
        .into_par_iter()
        .filter(|entry| {
            let path = entry.path();
            // Filter hidden files if not included
            if !include_hidden {
                if path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false) {
                    return false;
                }
            }
            true
        })
        .map(|entry| {
            let path = entry.path();
            if entry.file_type().is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_string());
                (ext, true, false)
            } else if entry.file_type().is_dir() {
                (None, false, true)
            } else {
                (None, false, false)
            }
        })
        .collect();

    // Aggregate results
    for (ext, is_file, is_dir) in results {
        if is_file {
            total_files += 1;
            if let Some(ext_str) = ext {
                *structure.entry(ext_str).or_insert(0) += 1;
            }
        } else if is_dir {
            total_dirs += 1;
        }
    }

    // Detect project type
    let project_type = if structure.contains_key("rs") {
        "Rust"
    } else if structure.contains_key("py") {
        "Python"
    } else if structure.contains_key("js") || structure.contains_key("ts") {
        "JavaScript/TypeScript"
    } else if structure.contains_key("go") {
        "Go"
    } else {
        "Unknown"
    };

    let most_common_ext = structure.iter()
        .max_by_key(|(_, count)| *count)
        .map(|(ext, _)| ext.clone())
        .unwrap_or_else(|| "none".to_string());

    Ok(json!({
        "project_root": ".",
        "project_type": project_type,
        "total_files": total_files,
        "total_directories": total_dirs,
        "file_types": structure,
        "insights": [
            format!("Project contains {} files across {} directories", total_files, total_dirs),
            format!("Primary language appears to be {}", project_type),
            format!("Most common file types: {}", most_common_ext)
        ]
    }))
}

/// Find similar code patterns across the codebase
pub async fn find_similar_code(args: Value) -> Result<Value> {
    #[derive(Deserialize)]
    struct Input {
        pattern: String,
        min_similarity: Option<f64>,
        max_results: Option<usize>,
    }
    let input: Input = serde_json::from_value(args).context("invalid find_similar_code args")?;
    let min_similarity = input.min_similarity.unwrap_or(0.7);
    let max_results = input.max_results.unwrap_or(10);

    // Collect all files first
    let entries: Vec<_> = WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
        
    // Process files in parallel
    let mut results: Vec<_> = entries
        .into_par_iter()
        .filter_map(|entry| {
            // For this implementation, we'll read files directly to avoid cache overhead
            // In a production environment, you might want to use the cache selectively
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let similarity = calculate_text_similarity(&input.pattern, &content);
                if similarity >= min_similarity {
                    Some(json!({
                        "path": entry.path().to_string_lossy(),
                        "similarity": similarity,
                        "line_count": content.lines().count(),
                        "size_bytes": content.len()
                    }))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Sort by similarity and limit results
    results.sort_by(|a, b| {
        b["similarity"].as_f64().unwrap_or(0.0)
            .partial_cmp(&a["similarity"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let limited_matches = results.into_iter().take(max_results).collect::<Vec<_>>();

    Ok(json!({
        "query_pattern": input.pattern,
        "min_similarity": min_similarity,
        "total_matches": limited_matches.len(),
        "matches": limited_matches
    }))
}

/// Generate documentation for code
pub async fn generate_code_documentation(args: Value) -> Result<Value> {
    #[derive(Deserialize)]
    struct Input {
        path: String,
        include_examples: Option<bool>,
    }
    let input: Input = serde_json::from_value(args).context("invalid generate_code_documentation args")?;

    // Try to get file content from cache first
    let content = if let Some(cached_content) = FILE_CACHE.get_file(&input.path).await {
        cached_content
    } else {
        // Read file content and cache it
        let content = fs::read_to_string(&input.path)?;
        FILE_CACHE.put_file(input.path.clone(), content.clone()).await;
        content
    };
    
    let include_examples = input.include_examples.unwrap_or(true);

    // Extract functions and generate documentation
    let mut documentation = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line.trim().starts_with("fn ") {
            if let Some(func_name) = extract_function_name(line) {
                documentation.push(json!({
                    "type": "function",
                    "name": func_name,
                    "line": line_num + 1,
                    "signature": line.trim(),
                    "documentation": format!("Function `{}` - {}", func_name, line.trim()),
                    "examples": if include_examples {
                        vec![format!("// Example usage of {}", func_name)]
                    } else {
                        vec![]
                    }
                }));
            }
        }
    }

    Ok(json!({
        "path": input.path,
        "total_functions": documentation.len(),
        "documentation": documentation,
        "summary": format!("Generated documentation for {} functions", documentation.len())
    }))
}

/// Detect code smells and quality issues
pub async fn detect_code_smells(args: Value) -> Result<Value> {
    #[derive(Deserialize)]
    struct Input {
        path: String,
        severity_threshold: Option<String>,
    }
    let input: Input = serde_json::from_value(args).context("invalid detect_code_smells args")?;

    // Try to get file content from cache first
    let content = if let Some(cached_content) = FILE_CACHE.get_file(&input.path).await {
        cached_content
    } else {
        // Read file content and cache it
        let content = fs::read_to_string(&input.path)?;
        FILE_CACHE.put_file(input.path.clone(), content.clone()).await;
        content
    };
    
    let lines = content.lines().collect::<Vec<_>>();
    let mut smells = Vec::new();

    // Check for various code smells
    if lines.len() > 100 {
        smells.push(json!({
            "type": "long_file",
            "severity": "medium",
            "line": 1,
            "description": "File is quite long, consider breaking it into smaller modules",
            "suggestion": "Split into multiple files or extract functions"
        }));
    }

    // Check for magic numbers
    for (line_num, line) in lines.iter().enumerate() {
        if line.contains(" = 42") || line.contains(" == 42") {
            smells.push(json!({
                "type": "magic_number",
                "severity": "low",
                "line": line_num + 1,
                "description": "Magic number detected",
                "suggestion": "Replace with named constant"
            }));
        }
    }

    // Check for long functions
    let mut function_start = None;
    for (line_num, line) in lines.iter().enumerate() {
        if line.trim().starts_with("fn ") {
            function_start = Some(line_num);
        } else if function_start.is_some() && line.trim().starts_with("}") {
            if let Some(start) = function_start {
                let length = line_num - start;
                if length > 30 {
                    smells.push(json!({
                        "type": "long_function",
                        "severity": "medium",
                        "line": start + 1,
                        "description": format!("Function is {} lines long", length),
                        "suggestion": "Consider breaking into smaller functions"
                    }));
                }
            }
            function_start = None;
        }
    }

    // Filter by severity
    let severity_threshold = input.severity_threshold.as_deref().unwrap_or("medium");
    let filtered_smells: Vec<_> = smells.into_iter()
        .filter(|smell| {
            let smell_severity = smell["severity"].as_str().unwrap_or("low");
            severity_level(smell_severity) >= severity_level(severity_threshold)
        })
        .collect();

    Ok(json!({
        "path": input.path,
        "total_smells": filtered_smells.len(),
        "severity_threshold": severity_threshold,
        "code_smells": filtered_smells
    }))
}

// Helper functions

fn calculate_text_similarity(pattern: &str, text: &str) -> f64 {
    // Simple Jaccard similarity for demonstration
    let pattern_words: std::collections::HashSet<_> = pattern.split_whitespace().collect();
    let text_words: std::collections::HashSet<_> = text.split_whitespace().collect();

    let intersection = pattern_words.intersection(&text_words).count();
    let union = pattern_words.len() + text_words.len() - intersection;

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn extract_function_name(line: &str) -> Option<String> {
    if let Some(start) = line.find("fn ") {
        let after_fn = &line[start + 3..];
        if let Some(end) = after_fn.find('(') {
            return Some(after_fn[..end].trim().to_string());
        }
    }
    None
}

fn severity_level(severity: &str) -> i32 {
    match severity {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

/// Enhanced grep search with semantic understanding
pub async fn semantic_grep_search(args: Value) -> Result<Value> {
    #[derive(Deserialize)]
    struct Input {
        pattern: String,
        path: Option<String>,
        context_lines: Option<usize>,
        semantic_context: Option<bool>,
        max_results: Option<usize>,
    }
    let input: Input = serde_json::from_value(args).context("invalid semantic_grep_search args")?;

    let search_path = input.path.as_deref().unwrap_or(".");
    let context_lines = input.context_lines.unwrap_or(2);
    let semantic_context = input.semantic_context.unwrap_or(false);
    let max_results = input.max_results.unwrap_or(50);

    // Collect all files first
    let entries: Vec<_> = WalkDir::new(search_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
        
    // Process files in parallel
    let mut results: Vec<Value> = entries
        .into_par_iter()
        .filter_map(|entry| {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let lines: Vec<&str> = content.lines().collect();
                let mut file_results = Vec::new();
                
                for (line_num, line) in lines.iter().enumerate() {
                    if line.contains(&input.pattern) {
                        let start_ctx = line_num.saturating_sub(context_lines);
                        let end_ctx = (line_num + context_lines + 1).min(lines.len());
                        let context = lines[start_ctx..end_ctx].join("\n");

                        let mut result = json!({
                            "file": entry.path().to_string_lossy(),
                            "line": line_num + 1,
                            "content": line.trim(),
                            "context": context
                        });

                        // Add semantic context if requested
                        if semantic_context {
                            let semantic_info = analyze_line_semantics(line, &lines, line_num);
                            result["semantic_info"] = semantic_info;
                        }

                        file_results.push(result);
                    }
                }
                
                Some(file_results)
            } else {
                None
            }
        })
        .flatten()
        .collect();

    // Limit results
    results.truncate(max_results);

    Ok(json!({
        "pattern": input.pattern,
        "total_matches": results.len(),
        "results": results,
        "search_path": search_path,
        "semantic_context_enabled": semantic_context
    }))
}

fn analyze_line_semantics(line: &str, all_lines: &[&str], line_num: usize) -> Value {
    let mut info = json!({});

    // Check if line is in a function
    let mut current_function = None;
    for i in (0..=line_num).rev() {
        if all_lines[i].trim().starts_with("fn ") {
            if let Some(name) = extract_function_name(all_lines[i]) {
                current_function = Some(name);
                break;
            }
        }
    }

    if let Some(func) = current_function {
        info["function_context"] = json!(func);
    }

    // Check if line contains common patterns
    if line.contains("if ") || line.contains("else") {
        info["control_flow"] = json!("conditional");
    } else if line.contains("for ") || line.contains("while ") {
        info["control_flow"] = json!("loop");
    } else if line.contains("return ") {
        info["control_flow"] = json!("return");
    }

    // Check for variable assignments
    if line.contains(" = ") && !line.contains("==") {
        info["statement_type"] = json!("assignment");
    }

    info
}

/// Batch file operations for improved performance
pub async fn batch_file_operations(args: Value) -> Result<Value> {
    #[derive(Deserialize)]
    struct Input {
        operations: Vec<BatchOperation>,
    }

    #[derive(Deserialize)]
    struct BatchOperation {
        operation: String,
        path: String,
        content: Option<String>,
        old_string: Option<String>,
        new_string: Option<String>,
    }

    let input: Input = serde_json::from_value(args).context("invalid batch_file_operations args")?;

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;
    let total_operations = input.operations.len();

    // Process operations in parallel for better performance
    let operation_results: Vec<_> = input.operations
        .into_par_iter()
        .map(|op| {
            match op.operation.as_str() {
                "read" => {
                    match fs::read_to_string(&op.path) {
                        Ok(content) => {
                            json!({
                                "operation": "read",
                                "path": op.path,
                                "status": "success",
                                "content": content
                            })
                        }
                        Err(e) => {
                            json!({
                                "operation": "read",
                                "path": op.path,
                                "status": "error",
                                "error": e.to_string()
                            })
                        }
                    }
                }
                "write" => {
                    if let Some(content) = op.content {
                        // Use buffered writing for better performance
                        match fs::File::create(&op.path) {
                            Ok(file) => {
                                let mut writer = std::io::BufWriter::new(file);
                                match writer.write_all(content.as_bytes()) {
                                    Ok(_) => {
                                        match writer.flush() {
                                            Ok(_) => {
                                                json!({
                                                    "operation": "write",
                                                    "path": op.path,
                                                    "status": "success"
                                                })
                                            }
                                            Err(e) => {
                                                json!({
                                                    "operation": "write",
                                                    "path": op.path,
                                                    "status": "error",
                                                    "error": format!("Failed to flush: {}", e)
                                                })
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        json!({
                                            "operation": "write",
                                            "path": op.path,
                                            "status": "error",
                                            "error": format!("Failed to write: {}", e)
                                        })
                                    }
                                }
                            }
                            Err(e) => {
                                json!({
                                    "operation": "write",
                                    "path": op.path,
                                    "status": "error",
                                    "error": format!("Failed to create file: {}", e)
                                })
                            }
                        }
                    } else {
                        json!({
                            "operation": "write",
                            "path": op.path,
                            "status": "error",
                            "error": "content required for write operation"
                        })
                    }
                }
                "edit" => {
                    if let (Some(old_str), Some(new_str)) = (op.old_string, op.new_string) {
                        match fs::read_to_string(&op.path) {
                            Ok(content) => {
                                // Count occurrences to ensure we have exactly one match
                                let count = content.matches(&old_str).count();
                                if count == 0 {
                                    json!({
                                        "operation": "edit",
                                        "path": op.path,
                                        "status": "error",
                                        "error": "old_string not found"
                                    })
                                } else if count > 1 {
                                    json!({
                                        "operation": "edit",
                                        "path": op.path,
                                        "status": "error",
                                        "error": format!("Found {} matches for old_string, expected exactly one", count)
                                    })
                                } else {
                                    let new_content = content.replace(&old_str, &new_str);
                                    // Use buffered writing for better performance
                                    match fs::File::create(&op.path) {
                                        Ok(file) => {
                                            let mut writer = std::io::BufWriter::new(file);
                                            match writer.write_all(new_content.as_bytes()) {
                                                Ok(_) => {
                                                    match writer.flush() {
                                                        Ok(_) => {
                                                            json!({
                                                                "operation": "edit",
                                                                "path": op.path,
                                                                "status": "success"
                                                            })
                                                        }
                                                        Err(e) => {
                                                            json!({
                                                                "operation": "edit",
                                                                "path": op.path,
                                                                "status": "error",
                                                                "error": format!("Failed to flush: {}", e)
                                                            })
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    json!({
                                                        "operation": "edit",
                                                        "path": op.path,
                                                        "status": "error",
                                                        "error": format!("Failed to write: {}", e)
                                                    })
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            json!({
                                                "operation": "edit",
                                                "path": op.path,
                                                "status": "error",
                                                "error": format!("Failed to create file: {}", e)
                                            })
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                json!({
                                    "operation": "edit",
                                    "path": op.path,
                                    "status": "error",
                                    "error": format!("Failed to read file: {}", e)
                                })
                            }
                        }
                    } else {
                        json!({
                            "operation": "edit",
                            "path": op.path,
                            "status": "error",
                            "error": "old_string and new_string required for edit operation"
                        })
                    }
                }
                _ => {
                    json!({
                        "operation": op.operation,
                        "path": op.path,
                        "status": "error",
                        "error": "unsupported operation"
                    })
                }
            }
        })
        .collect();

    // Process results
    for result in operation_results {
        let is_success = result["status"] == "success";
        if is_success {
            success_count += 1;
        } else {
            error_count += 1;
        }
        results.push(result);
    }

    Ok(json!({
        "total_operations": total_operations,
        "successful_operations": success_count,
        "failed_operations": error_count,
        "results": results,
        "summary": format!("Processed {} operations ({} success, {} failed)",
                          total_operations, success_count, error_count)
    }))
}

/// Get performance statistics for file operations
pub async fn get_file_operation_stats(_args: Value) -> Result<Value> {
    let stats = FILE_CACHE.get_stats().await;
    let hit_rate = if stats.hits + stats.misses > 0 {
        stats.hits as f64 / (stats.hits + stats.misses) as f64
    } else {
        0.0
    };

    Ok(json!({
        "cache_stats": {
            "hits": stats.hits,
            "misses": stats.misses,
            "hit_rate": hit_rate,
            "evictions": stats.evictions
        },
        "summary": format!("Cache hit rate: {:.2}%", hit_rate * 100.0)
    }))
}

/// Clear file operation cache
pub async fn clear_file_operation_cache(_args: Value) -> Result<Value> {
    FILE_CACHE.clear().await;
    Ok(json!({
        "status": "success",
        "message": "File operation cache cleared"
    }))
}

/// Get detailed performance statistics for all tools
pub async fn get_tool_performance_stats(args: Value) -> Result<Value> {
    #[derive(Deserialize)]
    struct Input {
        #[allow(dead_code)]
        detailed: Option<bool>,
    }
    let input: Input = serde_json::from_value(args).context("invalid get_tool_performance_stats args")?;

    // This function would typically interact with the ToolRegistry to get performance stats
    // For now, we'll return a placeholder response
    Ok(json!({
        "status": "success",
        "detailed": input.detailed.unwrap_or(false),
        "message": "Performance statistics not yet implemented for this tool",
        "placeholder_data": {
            "tools_monitored": ["read_file", "write_file", "list_files", "grep_search"],
            "total_calls": 0,
            "avg_response_time_ms": 0,
            "cache_hit_rate": 0.0
        }
    }))
}