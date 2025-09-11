//! File operation tools with composable functionality

use super::traits::{CacheableTool, FileTool, ModeTool, Tool};
use super::types::*;
use crate::tools::rp_search::RpSearchManager;
use crate::utils::vtagentgitignore::should_exclude_file;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

/// File operations tool with multiple modes
#[derive(Clone)]
pub struct FileOpsTool {
    workspace_root: PathBuf,
    rp_search: Arc<RpSearchManager>,
}

impl FileOpsTool {
    pub fn new(workspace_root: PathBuf, rp_search: Arc<RpSearchManager>) -> Self {
        Self {
            workspace_root,
            rp_search,
        }
    }

    /// Execute basic directory listing
    async fn execute_basic_list(&self, input: &ListInput) -> Result<Value> {
        let base = self.workspace_root.join(&input.path);

        if self.should_exclude(&base).await {
            return Err(anyhow!(
                "Path '{}' is excluded by .vtagentgitignore",
                input.path
            ));
        }

        let mut items = Vec::new();
        let mut count = 0;

        if base.is_file() {
            let metadata = tokio::fs::metadata(&base).await?;
            items.push(json!({
                "name": base.file_name().unwrap().to_string_lossy(),
                "path": input.path,
                "type": "file",
                "size": metadata.len(),
                "modified": metadata.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs())
            }));
            count = 1;
        } else if base.is_dir() {
            let mut entries = tokio::fs::read_dir(&base).await?;
            while let Some(entry) = entries.next_entry().await? {
                if count >= input.max_items {
                    break;
                }

                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if !input.include_hidden && name.starts_with('.') {
                    continue;
                }
                if self.should_exclude(&path).await {
                    continue;
                }

                let metadata = entry.metadata().await?;
                items.push(json!({
                    "name": name,
                    "path": path.strip_prefix(&self.workspace_root).unwrap_or(&path).to_string_lossy(),
                    "type": if metadata.is_dir() { "directory" } else { "file" },
                    "size": metadata.len(),
                    "modified": metadata.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs())
                }));
                count += 1;
            }
        }

        Ok(json!({
            "success": true,
            "items": items,
            "count": count,
            "mode": "list"
        }))
    }

    /// Execute recursive file search
    async fn execute_recursive_search(&self, input: &ListInput) -> Result<Value> {
        let pattern = input
            .name_pattern
            .as_ref()
            .ok_or_else(|| anyhow!("name_pattern required for recursive mode"))?;
        let search_path = self.workspace_root.join(&input.path);

        let mut items = Vec::new();
        let mut count = 0;

        for entry in WalkDir::new(&search_path).max_depth(10) {
            if count >= input.max_items {
                break;
            }

            let entry = entry.map_err(|e| anyhow!("Walk error: {}", e))?;
            let path = entry.path();

            if self.should_exclude(path).await {
                continue;
            }

            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if !input.include_hidden && name.starts_with('.') {
                continue;
            }

            // Pattern matching
            let matches = if input.case_sensitive.unwrap_or(true) {
                name.contains(pattern)
            } else {
                name.to_lowercase().contains(&pattern.to_lowercase())
            };

            if matches {
                // Extension filtering
                if let Some(ref extensions) = input.file_extensions {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if !extensions.contains(&ext.to_string()) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                let metadata = entry
                    .metadata()
                    .map_err(|e| anyhow!("Metadata error: {}", e))?;
                items.push(json!({
                    "name": name,
                    "path": path.strip_prefix(&self.workspace_root).unwrap_or(path).to_string_lossy(),
                    "type": if metadata.is_dir() { "directory" } else { "file" },
                    "size": metadata.len(),
                    "depth": entry.depth()
                }));
                count += 1;
            }
        }

        Ok(json!({
            "success": true,
            "items": items,
            "count": count,
            "mode": "recursive",
            "pattern": pattern
        }))
    }

    /// Execute find by exact name
    async fn execute_find_by_name(&self, input: &ListInput) -> Result<Value> {
        let file_name = input
            .name_pattern
            .as_ref()
            .ok_or_else(|| anyhow!("name_pattern required for find_name mode"))?;
        let search_path = self.workspace_root.join(&input.path);

        for entry in WalkDir::new(&search_path).max_depth(10) {
            let entry = entry.map_err(|e| anyhow!("Walk error: {}", e))?;
            let path = entry.path();

            if self.should_exclude(path).await {
                continue;
            }

            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let matches = if input.case_sensitive.unwrap_or(true) {
                name == file_name.as_str()
            } else {
                name.to_lowercase() == file_name.to_lowercase()
            };

            if matches {
                let metadata = entry
                    .metadata()
                    .map_err(|e| anyhow!("Metadata error: {}", e))?;
                return Ok(json!({
                    "success": true,
                    "found": true,
                    "name": name,
                    "path": path.strip_prefix(&self.workspace_root).unwrap_or(path).to_string_lossy(),
                    "type": if metadata.is_dir() { "directory" } else { "file" },
                    "size": metadata.len(),
                    "mode": "find_name"
                }));
            }
        }

        Ok(json!({
            "success": true,
            "found": false,
            "mode": "find_name",
            "searched_for": file_name
        }))
    }

    /// Execute find by content pattern
    async fn execute_find_by_content(&self, input: &ListInput) -> Result<Value> {
        let content_pattern = input
            .content_pattern
            .as_ref()
            .ok_or_else(|| anyhow!("content_pattern required for find_content mode"))?;

        // Simple content search implementation
        let search_path = self.workspace_root.join(&input.path);
        let mut items = Vec::new();
        let mut count = 0;

        for entry in WalkDir::new(&search_path).max_depth(10) {
            if count >= input.max_items {
                break;
            }

            let entry = entry.map_err(|e| anyhow!("Walk error: {}", e))?;
            let path = entry.path();

            if !path.is_file() || self.should_exclude(path).await {
                continue;
            }

            // Read file content and search for pattern
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                let matches = if input.case_sensitive.unwrap_or(true) {
                    content.contains(content_pattern)
                } else {
                    content
                        .to_lowercase()
                        .contains(&content_pattern.to_lowercase())
                };

                if matches {
                    if let Ok(metadata) = tokio::fs::metadata(path).await {
                        items.push(json!({
                            "name": path.file_name().unwrap_or_default().to_string_lossy(),
                            "path": path.strip_prefix(&self.workspace_root).unwrap_or(path).to_string_lossy(),
                            "type": "file",
                            "size": metadata.len(),
                            "pattern_found": true
                        }));
                        count += 1;
                    }
                }
            }
        }

        Ok(json!({
            "success": true,
            "items": items,
            "count": count,
            "mode": "find_content",
            "pattern": content_pattern
        }))
    }

    /// Read file with intelligent path resolution
    pub async fn read_file(&self, args: Value) -> Result<Value> {
        let input: Input = serde_json::from_value(args).context("invalid read_file args")?;

        // Try to resolve the file path
        let potential_paths = self.resolve_file_path(&input.path)?;

        for candidate_path in &potential_paths {
            if self.should_exclude(candidate_path).await {
                continue;
            }

            if candidate_path.exists() && candidate_path.is_file() {
                let content = if let Some(max_bytes) = input.max_bytes {
                    let mut file_content = tokio::fs::read(candidate_path).await?;
                    if file_content.len() > max_bytes {
                        file_content.truncate(max_bytes);
                    }
                    String::from_utf8_lossy(&file_content).to_string()
                } else {
                    tokio::fs::read_to_string(candidate_path).await?
                };

                return Ok(json!({
                    "success": true,
                    "content": content,
                    "path": candidate_path.strip_prefix(&self.workspace_root).unwrap_or(candidate_path).to_string_lossy(),
                    "size": content.len()
                }));
            }
        }

        Err(anyhow!("File not found: {}", input.path))
    }

    /// Write file with various modes
    pub async fn write_file(&self, args: Value) -> Result<Value> {
        let input: WriteInput = serde_json::from_value(args).context("invalid write_file args")?;
        let file_path = self.workspace_root.join(&input.path);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        match input.mode.as_str() {
            "overwrite" => {
                tokio::fs::write(&file_path, &input.content).await?;
            }
            "append" => {
                use tokio::io::AsyncWriteExt;
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&file_path)
                    .await?;
                file.write_all(input.content.as_bytes()).await?;
            }
            "skip_if_exists" => {
                if file_path.exists() {
                    return Ok(json!({
                        "success": true,
                        "skipped": true,
                        "reason": "File already exists"
                    }));
                }
                tokio::fs::write(&file_path, &input.content).await?;
            }
            _ => return Err(anyhow!("Unsupported write mode: {}", input.mode)),
        }

        Ok(json!({
            "success": true,
            "path": input.path,
            "mode": input.mode,
            "bytes_written": input.content.len()
        }))
    }

    /// Resolve file path with intelligent fallbacks
    fn resolve_file_path(&self, path: &str) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        // Try exact path first
        paths.push(self.workspace_root.join(path));

        // If it's just a filename, try common directories
        if !path.contains('/') && !path.contains('\\') {
            paths.push(self.workspace_root.join("src").join(path));
            paths.push(self.workspace_root.join("lib").join(path));
            paths.push(self.workspace_root.join("bin").join(path));
        }

        Ok(paths)
    }
}

#[async_trait]
impl Tool for FileOpsTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let input: ListInput = serde_json::from_value(args).context("invalid list_files args")?;

        let mode_clone = input.mode.clone();
        let mode = mode_clone.as_deref().unwrap_or("list");
        self.execute_mode(mode, serde_json::to_value(input)?).await
    }

    fn name(&self) -> &'static str {
        "list_files"
    }

    fn description(&self) -> &'static str {
        "Enhanced file discovery tool with multiple modes: list (default), recursive, find_name, find_content"
    }
}

#[async_trait]
impl FileTool for FileOpsTool {
    fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    async fn should_exclude(&self, path: &Path) -> bool {
        should_exclude_file(path).await
    }
}

#[async_trait]
impl ModeTool for FileOpsTool {
    fn supported_modes(&self) -> Vec<&'static str> {
        vec!["list", "recursive", "find_name", "find_content"]
    }

    async fn execute_mode(&self, mode: &str, args: Value) -> Result<Value> {
        let input: ListInput = serde_json::from_value(args)?;

        match mode {
            "list" => self.execute_basic_list(&input).await,
            "recursive" => self.execute_recursive_search(&input).await,
            "find_name" => self.execute_find_by_name(&input).await,
            "find_content" => self.execute_find_by_content(&input).await,
            _ => Err(anyhow!("Unsupported file operation mode: {}", mode)),
        }
    }
}

#[async_trait]
impl CacheableTool for FileOpsTool {
    fn cache_key(&self, args: &Value) -> String {
        format!(
            "files:{}:{}",
            args.get("path").and_then(|p| p.as_str()).unwrap_or(""),
            args.get("mode").and_then(|m| m.as_str()).unwrap_or("list")
        )
    }

    fn should_cache(&self, args: &Value) -> bool {
        // Cache list and recursive modes, but not content-based searches
        let mode = args.get("mode").and_then(|m| m.as_str()).unwrap_or("list");
        matches!(mode, "list" | "recursive")
    }

    fn cache_ttl(&self) -> u64 {
        60 // 1 minute for file listings
    }
}
