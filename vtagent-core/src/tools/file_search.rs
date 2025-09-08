//! Recursive file search functionality for VTAgent
//!
//! This module provides utilities for recursively searching files in a project workspace,
//! with support for glob patterns, exclusions, and content searching.

use anyhow::{Context, Result};
use glob::Pattern;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Configuration for file search operations
#[derive(Debug, Clone)]
pub struct FileSearchConfig {
    /// Maximum number of results to return
    pub max_results: usize,
    /// Whether to follow symbolic links
    pub follow_links: bool,
    /// Whether to include hidden files
    pub include_hidden: bool,
    /// File extensions to include (if empty, include all)
    pub include_extensions: HashSet<String>,
    /// File extensions to exclude
    pub exclude_extensions: HashSet<String>,
    /// File names/patterns to exclude
    pub exclude_patterns: Vec<Pattern>,
    /// Maximum file size in bytes (0 = no limit)
    pub max_file_size: u64,
}

impl Default for FileSearchConfig {
    fn default() -> Self {
        Self {
            max_results: 1000,
            follow_links: false,
            include_hidden: false,
            include_extensions: HashSet::new(),
            exclude_extensions: HashSet::new(),
            exclude_patterns: Vec::new(),
            max_file_size: 0,
        }
    }
}

/// Result of a file search operation
#[derive(Debug, Clone)]
pub struct FileSearchResult {
    /// Path to the file
    pub path: PathBuf,
    /// File name
    pub name: String,
    /// File extension
    pub extension: Option<String>,
    /// File size in bytes
    pub size: u64,
    /// Whether the file is a directory
    pub is_dir: bool,
    /// Content matches (if searched for content)
    pub content_matches: Vec<ContentMatch>,
}

/// A match found in file content
#[derive(Debug, Clone)]
pub struct ContentMatch {
    /// Line number (1-based)
    pub line_number: usize,
    /// Content of the line
    pub content: String,
    /// Column where match starts
    pub column: usize,
}

/// File searcher for recursive file operations
pub struct FileSearcher {
    root: PathBuf,
    config: FileSearchConfig,
}

impl FileSearcher {
    /// Create a new file searcher
    pub fn new(root: PathBuf, config: FileSearchConfig) -> Self {
        Self { root, config }
    }

    /// Create a searcher with default configuration
    pub fn with_default_config(root: PathBuf) -> Self {
        Self::new(root, FileSearchConfig::default())
    }

    /// Recursively search for files matching the given pattern
    pub fn search_files(&self, pattern: Option<&str>) -> Result<Vec<FileSearchResult>> {
        let mut results = Vec::new();
        let max_results = self.config.max_results;

        for entry in WalkDir::new(&self.root)
            .follow_links(self.config.follow_links)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();

            // Skip if should be excluded
            if self.should_exclude_path(path)? {
                continue;
            }

            // Check if path matches pattern (if pattern is provided)
            if let Some(pattern_str) = pattern {
                if !pattern_str.is_empty() && !self.path_matches_pattern(path, pattern_str)? {
                    continue;
                }
            }

            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(_) => continue, // Skip files we can't read metadata for
            };

            let file_result = FileSearchResult {
                path: path.to_path_buf(),
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string(),
                extension: path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_string()),
                size: metadata.len(),
                is_dir: metadata.is_dir(),
                content_matches: Vec::new(),
            };

            results.push(file_result);
        }

        Ok(results)
    }

    /// Search for files containing specific content
    pub fn search_files_with_content(
        &self,
        content_pattern: &str,
        file_pattern: Option<&str>,
    ) -> Result<Vec<FileSearchResult>> {
        let mut results = Vec::new();
        let max_results = self.config.max_results;

        for entry in WalkDir::new(&self.root)
            .follow_links(self.config.follow_links)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if results.len() >= max_results {
                break;
            }

            let path = entry.path();

            // Skip if should be excluded
            if self.should_exclude_path(path)? {
                continue;
            }

            // Skip directories for content search
            if entry.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                continue;
            }

            // Check file pattern if specified
            if let Some(pattern_str) = file_pattern {
                if !self.path_matches_pattern(path, pattern_str)? {
                    continue;
                }
            }

            // Search for content in the file
            match self.search_content_in_file(path, content_pattern) {
                Ok(content_matches) => {
                    if !content_matches.is_empty() {
                        let metadata = match entry.metadata() {
                            Ok(meta) => meta,
                            Err(_) => continue,
                        };

                        let file_result = FileSearchResult {
                            path: path.to_path_buf(),
                            name: path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string(),
                            extension: path
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .map(|ext| ext.to_string()),
                            size: metadata.len(),
                            is_dir: metadata.is_dir(),
                            content_matches,
                        };

                        results.push(file_result);
                    }
                }
                Err(_) => {
                    // Skip files we can't read
                    continue;
                }
            }
        }

        Ok(results)
    }

    /// Find a specific file by name (recursively)
    pub fn find_file_by_name(&self, file_name: &str) -> Result<Option<PathBuf>> {
        for entry in WalkDir::new(&self.root)
            .follow_links(self.config.follow_links)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip if should be excluded
            if self.should_exclude_path(path)? {
                continue;
            }

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == file_name {
                    return Ok(Some(path.to_path_buf()));
                }
            }
        }

        Ok(None)
    }

    /// Check if a path should be excluded based on configuration
    fn should_exclude_path(&self, path: &Path) -> Result<bool> {
        let path_str = path.to_string_lossy();

        // Skip hidden files if not included
        if !self.config.include_hidden {
            // Check if any component of the path is hidden (starts with '.')
            for component in path.components() {
                if let std::path::Component::Normal(name) = component {
                    if let Some(name_str) = name.to_str() {
                        if name_str.starts_with('.') {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // Check file extensions
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();

            // Check exclude extensions
            if self.config.exclude_extensions.contains(&ext_lower) {
                return Ok(true);
            }

            // Check include extensions (if specified)
            if !self.config.include_extensions.is_empty()
                && !self.config.include_extensions.contains(&ext_lower)
            {
                return Ok(true);
            }
        }

        // Check exclude patterns
        for pattern in &self.config.exclude_patterns {
            if pattern.matches(&path_str) {
                return Ok(true);
            }
        }

        // Check file size
        if self.config.max_file_size > 0 {
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > self.config.max_file_size {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if a path matches a pattern
    fn path_matches_pattern(&self, path: &Path, pattern: &str) -> Result<bool> {
        // If pattern is empty, match everything
        if pattern.is_empty() {
            return Ok(true);
        }

        // Convert to lowercase for case-insensitive matching
        let path_str = path.to_string_lossy().to_lowercase();
        let pattern_lower = pattern.to_lowercase();

        // Handle wildcard patterns
        if pattern_lower.contains('*') || pattern_lower.contains('?') {
            // Use glob matching for patterns with wildcards
            if let Ok(glob_pattern) = Pattern::new(&format!("*{}*", pattern_lower)) {
                return Ok(glob_pattern.matches(&path_str));
            }
        }

        // Simple substring match for basic patterns
        Ok(path_str.contains(&pattern_lower))
    }

    /// Search for content within a file
    fn search_content_in_file(&self, path: &Path, pattern: &str) -> Result<Vec<ContentMatch>> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let mut matches = Vec::new();
        let pattern_lower = pattern.to_lowercase();

        for (line_num, line) in content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            if line_lower.contains(&pattern_lower) {
                // Find all occurrences in the line
                let mut start = 0;
                while let Some(pos) = line_lower[start..].find(&pattern_lower) {
                    let actual_pos = start + pos;
                    matches.push(ContentMatch {
                        line_number: line_num + 1,
                        content: line.to_string(),
                        column: actual_pos,
                    });
                    start = actual_pos + pattern.len();
                }
            }
        }

        Ok(matches)
    }

    /// Convert search results to JSON format
    pub fn results_to_json(results: Vec<FileSearchResult>) -> Value {
        let json_results: Vec<Value> = results
            .into_iter()
            .map(|result| {
                json!({
                    "path": result.path.to_string_lossy(),
                    "name": result.name,
                    "extension": result.extension,
                    "size": result.size,
                    "is_dir": result.is_dir,
                    "content_matches": result.content_matches.iter().map(|m| json!({
                        "line_number": m.line_number,
                        "content": m.content,
                        "column": m.column,
                    })).collect::<Vec<Value>>()
                })
            })
            .collect();

        json!({
            "success": true,
            "results": json_results,
            "count": json_results.len()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_searcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let searcher = FileSearcher::with_default_config(temp_dir.path().to_path_buf());
        assert_eq!(searcher.root, temp_dir.path());
    }

    #[test]
    fn test_find_file_by_name() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let searcher = FileSearcher::with_default_config(temp_dir.path().to_path_buf());
        let result = searcher.find_file_by_name("test.txt")?;

        assert!(result.is_some());
        assert_eq!(result.unwrap(), test_file);

        Ok(())
    }

    #[test]
    fn test_search_files() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.path().join("file2.rs"), "content2").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("subdir").join("file3.txt"), "content3").unwrap();

        let searcher = FileSearcher::with_default_config(temp_dir.path().to_path_buf());
        let results = searcher.search_files(None)?;

        assert_eq!(results.len(), 4); // 2 files + 1 subdir + 1 file in subdir

        Ok(())
    }
}
