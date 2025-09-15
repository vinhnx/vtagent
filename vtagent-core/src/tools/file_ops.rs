//! File operation tools with composable functionality

use super::traits::{CacheableTool, FileTool, ModeTool, Tool};
use super::types::*;
use crate::tools::grep_search::GrepSearchManager;
use crate::utils::vtagentgitignore::should_exclude_file;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};
use walkdir::WalkDir;

/// File operations tool with multiple modes
#[derive(Clone)]
pub struct FileOpsTool {
    workspace_root: PathBuf,
}

impl FileOpsTool {
    pub fn new(workspace_root: PathBuf, _grep_search: Arc<GrepSearchManager>) -> Self {
        // grep_search was unused; keep param to avoid broad call-site churn
        Self { workspace_root }
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

        let mut all_items = Vec::new();
        if base.is_file() {
            let metadata = tokio::fs::metadata(&base).await
                .with_context(|| format!("Failed to read metadata for file: {}", input.path))?;
            all_items.push(json!({
                "name": base.file_name().unwrap().to_string_lossy(),
                "path": input.path,
                "type": "file",
                "size": metadata.len(),
                "modified": metadata.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs())
            }));
        } else if base.is_dir() {
            let mut entries = tokio::fs::read_dir(&base).await
                .with_context(|| format!("Failed to read directory: {}", input.path))?;
            while let Some(entry) = entries.next_entry().await
                .with_context(|| format!("Failed to read directory entry in: {}", input.path))? {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if !input.include_hidden && name.starts_with('.') {
                    continue;
                }
                if self.should_exclude(&path).await {
                    continue;
                }

                let metadata = entry.metadata().await
                    .with_context(|| format!("Failed to read metadata for: {}", path.display()))?;
                all_items.push(json!({
                    "name": name,
                    "path": path.strip_prefix(&self.workspace_root).unwrap_or(&path).to_string_lossy(),
                    "type": if metadata.is_dir() { "directory" } else { "file" },
                    "size": metadata.len(),
                    "modified": metadata.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs())
                }));
            }
        } else {
            warn!(
                path = %input.path,
                exists = base.exists(),
                is_file = base.is_file(),
                is_dir = base.is_dir(),
                "Path does not exist or is neither file nor directory"
            );
            return Err(anyhow!("Path '{}' does not exist", input.path));
        }

        // Apply max_items cap first for token efficiency
        let capped_total = all_items.len().min(input.max_items);
        let (page, per_page) = (
            input.page.unwrap_or(1).max(1),
            input.per_page.unwrap_or(50).max(1),
        );
        let start = (page - 1).saturating_mul(per_page);
        let end = (start + per_page).min(capped_total);
        let has_more = end < capped_total;

        // Log paging operation details
        info!(
            path = %input.path,
            total_items = all_items.len(),
            capped_total = capped_total,
            page = page,
            per_page = per_page,
            start_index = start,
            end_index = end,
            has_more = has_more,
            "Executing paginated file listing"
        );

        // Validate paging parameters
        if page > 1 && start >= capped_total {
            warn!(
                path = %input.path,
                page = page,
                per_page = per_page,
                total_items = capped_total,
                "Requested page exceeds available data"
            );
        }

        let mut page_items = if start < end {
            all_items[start..end].to_vec()
        } else {
            warn!(
                path = %input.path,
                page = page,
                per_page = per_page,
                start_index = start,
                end_index = end,
                "Empty page result - no items in requested range"
            );
            vec![]
        };

        // Respect response_format
        let concise = input
            .response_format
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("concise"))
            .unwrap_or(true);
        if concise {
            for obj in page_items.iter_mut() {
                if let Some(map) = obj.as_object_mut() {
                    map.remove("modified");
                }
            }
        }

        let guidance = if has_more || capped_total < all_items.len() || all_items.len() > 20 {
            Some(format!(
                "Showing {} of {} items (page {}, per_page {}). Use 'page' and 'per_page' to page through results.",
                page_items.len(),
                capped_total,
                page,
                per_page
            ))
        } else {
            None
        };

        let mut out = json!({
            "success": true,
            "items": page_items,
            "count": page_items.len(),
            "total": capped_total,
            "page": page,
            "per_page": per_page,
            "has_more": has_more,
            "mode": "list",
            "response_format": if concise { "concise" } else { "detailed" }
        });

        if let Some(msg) = guidance {
            out["message"] = json!(msg);
        }
        Ok(out)
    }

    /// Execute recursive file search
    async fn execute_recursive_search(&self, input: &ListInput) -> Result<Value> {
        // Allow recursive listing without pattern by defaulting to "*" (match all)
        let default_pattern = "*".to_string();
        let pattern = input
            .name_pattern
            .as_ref()
            .unwrap_or(&default_pattern);
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

            // Pattern matching - handle "*" as wildcard for all files
            let matches = if pattern == "*" {
                true // Match all files when pattern is "*"
            } else if input.case_sensitive.unwrap_or(true) {
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

        Ok(self.paginate_and_format(items, count, input, "recursive", Some(pattern)))
    }

    /// Execute find by exact name
    async fn execute_find_by_name(&self, input: &ListInput) -> Result<Value> {
        let file_name = input
            .name_pattern
            .as_ref()
            .ok_or_else(|| anyhow!("Error: Missing 'name_pattern'. Example: list_files(path='.', mode='find_name', name_pattern='Cargo.toml')"))?;
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
            "searched_for": file_name,
            "message": "Not found. Consider using mode='recursive' if searching in subdirectories."
        }))
    }

    /// Execute find by content pattern
    async fn execute_find_by_content(&self, input: &ListInput) -> Result<Value> {
        let content_pattern = input
            .content_pattern
            .as_ref()
            .ok_or_else(|| anyhow!("Error: Missing 'content_pattern'. Example: list_files(path='src', mode='find_content', content_pattern='fn main')"))?;

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

        Ok(self.paginate_and_format(items, count, input, "find_content", Some(content_pattern)))
    }

    /// Read file with intelligent path resolution
    pub async fn read_file(&self, args: Value) -> Result<Value> {
        let input: Input = serde_json::from_value(args)
            .context("Error: Invalid 'read_file' arguments. Required: {{ path: string }}. Optional: {{ max_bytes: number }}. Example: read_file({{\"path\": \"src/main.rs\", \"max_bytes\": 20000}})")?;

        // Try to resolve the file path
        let potential_paths = self.resolve_file_path(&input.path)?;

        for candidate_path in &potential_paths {
            if self.should_exclude(candidate_path).await {
                continue;
            }

            if candidate_path.exists() && candidate_path.is_file() {
                // Check if chunking is needed
                let should_chunk = if let Some(max_lines) = input.max_lines {
                    // User specified max_lines threshold
                    self.count_lines_with_tree_sitter(candidate_path).await? > max_lines
                } else if let Some(chunk_lines) = input.chunk_lines {
                    // User specified chunk_lines (legacy parameter)
                    self.count_lines_with_tree_sitter(candidate_path).await? > chunk_lines
                } else {
                    // Use default threshold
                    self.count_lines_with_tree_sitter(candidate_path).await? > crate::config::constants::chunking::MAX_LINES_THRESHOLD
                };

                let (content, truncated, total_lines) = if should_chunk {
                    // Calculate chunk sizes for logging
                    let start_chunk = if let Some(max_lines) = input.max_lines {
                        max_lines / 2
                    } else if let Some(chunk_lines) = input.chunk_lines {
                        chunk_lines / 2
                    } else {
                        crate::config::constants::chunking::CHUNK_START_LINES
                    };
                    let _end_chunk = start_chunk;

                    let result = self.read_file_chunked(candidate_path, &input).await?;
                    // Log chunking operation
                    self.log_chunking_operation(candidate_path, result.1, result.2).await?;
                    result
                } else {
                    let content = if let Some(max_bytes) = input.max_bytes {
                        let mut file_content = tokio::fs::read(candidate_path).await?;
                        if file_content.len() > max_bytes {
                            file_content.truncate(max_bytes);
                        }
                        String::from_utf8_lossy(&file_content).to_string()
                    } else {
                        tokio::fs::read_to_string(candidate_path).await?
                    };
                    (content, false, None)
                };

                let mut result = json!({
                    "success": true,
                    "content": content,
                    "path": candidate_path.strip_prefix(&self.workspace_root).unwrap_or(candidate_path).to_string_lossy(),
                    "metadata": {
                        "size": content.len()
                    }
                });

                if truncated {
                    result["truncated"] = json!(true);
                    result["truncation_reason"] = json!("file_exceeds_line_threshold");
                    if let Some(total) = total_lines {
                        result["total_lines"] = json!(total);
                        let start_chunk = if let Some(max_lines) = input.max_lines {
                            max_lines / 2
                        } else if let Some(chunk_lines) = input.chunk_lines {
                            chunk_lines / 2
                        } else {
                            crate::config::constants::chunking::CHUNK_START_LINES
                        };
                        let end_chunk = start_chunk;
                        result["shown_lines"] = json!(start_chunk + end_chunk);
                    }
                }

                // Log chunking operation
                self.log_chunking_operation(candidate_path, truncated, total_lines).await?;

                return Ok(result);
            }
        }

        Err(anyhow!(
            "Error: File not found: {}. Tried paths: {}. Suggestions: 1) Check the file path and case sensitivity, 2) Use 'list_files' to explore the directory structure, 3) Try case-insensitive search with just the filename. Example: read_file({{\"path\": \"src/main.rs\"}})",
            input.path,
            potential_paths
                .iter()
                .map(|p| p
                    .strip_prefix(&self.workspace_root)
                    .unwrap_or(p)
                    .to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// Write file with various modes and chunking support for large content
    pub async fn write_file(&self, args: Value) -> Result<Value> {
        let input: WriteInput = serde_json::from_value(args)
            .context("Error: Invalid 'write_file' arguments. Required: {{ path: string, content: string }}. Optional: {{ mode: 'overwrite'|'append'|'skip_if_exists' }}. Example: write_file({{\"path\": \"README.md\", \"content\": \"Hello\", \"mode\": \"overwrite\"}})")?;
        let file_path = self.workspace_root.join(&input.path);

        // Check if content needs chunking
        let content_size = input.content.len();
        let should_chunk = content_size > crate::config::constants::chunking::MAX_WRITE_CONTENT_SIZE;

        if should_chunk {
            return self.write_file_chunked(&file_path, &input).await;
        }

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
            _ => {
                return Err(anyhow!(format!(
                    "Error: Unsupported write mode '{}'. Allowed: overwrite, append, skip_if_exists.",
                    input.mode
                )));
            }
        }

        // Log write operation
        self.log_write_operation(&file_path, content_size, false).await?;

        Ok(json!({
            "success": true,
            "path": input.path,
            "mode": input.mode,
            "bytes_written": input.content.len()
        }))
    }

    /// Write large file in chunks for atomicity and memory efficiency
    async fn write_file_chunked(&self, file_path: &Path, input: &WriteInput) -> Result<Value> {
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content_bytes = input.content.as_bytes();
        let chunk_size = crate::config::constants::chunking::WRITE_CHUNK_SIZE;
        let total_size = content_bytes.len();

        match input.mode.as_str() {
            "overwrite" => {
                // Write in chunks for large files
                use tokio::io::AsyncWriteExt;
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(file_path)
                    .await?;

                for chunk in content_bytes.chunks(chunk_size) {
                    file.write_all(chunk).await?;
                }
                file.flush().await?;
            }
            "append" => {
                // Append in chunks
                use tokio::io::AsyncWriteExt;
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)
                    .await?;

                for chunk in content_bytes.chunks(chunk_size) {
                    file.write_all(chunk).await?;
                }
                file.flush().await?;
            }
            "skip_if_exists" => {
                if file_path.exists() {
                    return Ok(json!({
                        "success": true,
                        "skipped": true,
                        "reason": "File already exists"
                    }));
                }
                // Write in chunks for new file
                use tokio::io::AsyncWriteExt;
                let mut file = tokio::fs::File::create(file_path).await?;
                for chunk in content_bytes.chunks(chunk_size) {
                    file.write_all(chunk).await?;
                }
                file.flush().await?;
            }
            _ => {
                return Err(anyhow!(format!(
                    "Error: Unsupported write mode '{}'. Allowed: overwrite, append, skip_if_exists.",
                    input.mode
                )));
            }
        }

        // Log chunked write operation
        self.log_write_operation(file_path, total_size, true).await?;

        Ok(json!({
            "success": true,
            "path": file_path.strip_prefix(&self.workspace_root).unwrap_or(file_path).to_string_lossy(),
            "mode": input.mode,
            "bytes_written": total_size,
            "chunked": true,
            "chunk_size": chunk_size,
            "chunks_written": (total_size + chunk_size - 1) / chunk_size
        }))
    }

    /// Log write operations for debugging
    async fn log_write_operation(&self, file_path: &Path, bytes_written: usize, chunked: bool) -> Result<()> {
        let log_entry = json!({
            "operation": if chunked { "write_file_chunked" } else { "write_file" },
            "file_path": file_path.to_string_lossy(),
            "bytes_written": bytes_written,
            "chunked": chunked,
            "chunk_size": if chunked { Some(crate::config::constants::chunking::WRITE_CHUNK_SIZE) } else { None },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        info!("File write operation: {}", serde_json::to_string(&log_entry)?);
        Ok(())
    }
}

#[async_trait]
impl Tool for FileOpsTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let input: ListInput = serde_json::from_value(args).context(
            "Error: Invalid 'list_files' arguments. Required: {{ path: string }}. Optional: {{ mode, max_items, page, per_page, include_hidden, response_format }}. Example: list_files({{\"path\": \"src\", \"page\": 1, \"per_page\": 50, \"response_format\": \"concise\"}})",
        )?;

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

impl FileOpsTool {
    fn paginate_and_format(
        &self,
        items: Vec<Value>,
        total_count: usize,
        input: &ListInput,
        mode: &str,
        pattern: Option<&String>,
    ) -> Value {
        let (page, per_page) = (
            input.page.unwrap_or(1).max(1),
            input.per_page.unwrap_or(50).max(1),
        );
        let total_capped = total_count.min(input.max_items);
        let start = (page - 1).saturating_mul(per_page);
        let end = (start + per_page).min(total_capped);
        let has_more = end < total_capped;

        // Log pagination operation details
        info!(
            mode = %mode,
            pattern = ?pattern,
            total_items = total_count,
            capped_total = total_capped,
            page = page,
            per_page = per_page,
            start_index = start,
            end_index = end,
            has_more = has_more,
            "Executing paginated search results"
        );

        // Validate pagination parameters
        if page > 1 && start >= total_capped {
            warn!(
                mode = %mode,
                page = page,
                per_page = per_page,
                total_items = total_capped,
                "Requested page exceeds available search results"
            );
        }

        let mut page_items = if start < end {
            items[start..end].to_vec()
        } else {
            warn!(
                mode = %mode,
                page = page,
                per_page = per_page,
                start_index = start,
                end_index = end,
                "Empty page result - no search results in requested range"
            );
            vec![]
        };

        let concise = input
            .response_format
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("concise"))
            .unwrap_or(true);
        if concise {
            for obj in page_items.iter_mut() {
                if let Some(map) = obj.as_object_mut() {
                    map.remove("modified");
                }
            }
        }

        let mut out = json!({
            "success": true,
            "items": page_items,
            "count": page_items.len(),
            "total": total_capped,
            "page": page,
            "per_page": per_page,
            "has_more": has_more,
            "mode": mode,
            "response_format": if concise { "concise" } else { "detailed" }
        });
        if let Some(p) = pattern {
            out["pattern"] = json!(p);
        }
        if has_more || total_capped > 20 {
            out["message"] = json!(format!(
                "Showing {} of {} results. Use 'page' to continue.",
                out["count"].as_u64().unwrap_or(0),
                total_capped
            ));
        }
        out
    }

    /// Count lines in a file using tree-sitter for accurate parsing
    async fn count_lines_with_tree_sitter(&self, file_path: &Path) -> Result<usize> {
        let content = tokio::fs::read_to_string(file_path).await?;
        Ok(content.lines().count())
    }

    /// Read file with chunking (first N + last N lines)
    async fn read_file_chunked(&self, file_path: &Path, input: &Input) -> Result<(String, bool, Option<usize>)> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Use custom chunk sizes if provided, otherwise use defaults
        let start_chunk = if let Some(chunk_lines) = input.chunk_lines {
            chunk_lines / 2
        } else {
            crate::config::constants::chunking::CHUNK_START_LINES
        };
        let end_chunk = if let Some(chunk_lines) = input.chunk_lines {
            chunk_lines / 2
        } else {
            crate::config::constants::chunking::CHUNK_END_LINES
        };

        if total_lines <= start_chunk + end_chunk {
            // File is small enough, return all content
            return Ok((content, false, Some(total_lines)));
        }

        // Create chunked content
        let mut chunked_content = String::new();

        // Add first N lines
        for (i, line) in lines.iter().enumerate().take(start_chunk) {
            if i > 0 {
                chunked_content.push('\n');
            }
            chunked_content.push_str(line);
        }

        // Add truncation indicator
        chunked_content.push_str(&format!(
            "\n\n... [{} lines truncated - showing first {} and last {} lines] ...\n\n",
            total_lines - start_chunk - end_chunk,
            start_chunk,
            end_chunk
        ));

        // Add last N lines
        let start_idx = total_lines.saturating_sub(end_chunk);
        for (i, line) in lines.iter().enumerate().skip(start_idx) {
            if i > start_idx {
                chunked_content.push('\n');
            }
            chunked_content.push_str(line);
        }

        Ok((chunked_content, true, Some(total_lines)))
    }

    /// Log chunking operations for debugging
    async fn log_chunking_operation(&self, file_path: &Path, truncated: bool, total_lines: Option<usize>) -> Result<()> {
        if truncated {
            let log_entry = json!({
                "operation": "read_file_chunked",
                "file_path": file_path.to_string_lossy(),
                "truncated": true,
                "total_lines": total_lines,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            info!("File chunking operation: {}", serde_json::to_string(&log_entry)?);
        }
        Ok(())
    }

    fn resolve_file_path(&self, path: &str) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        // Try exact path first
        paths.push(self.workspace_root.join(path));

        // If it's just a filename, try common directories that exist in most projects
        if !path.contains('/') && !path.contains('\\') {
            // Generic source directories found in most projects
            paths.push(self.workspace_root.join("src").join(path));
            paths.push(self.workspace_root.join("lib").join(path));
            paths.push(self.workspace_root.join("bin").join(path));
            paths.push(self.workspace_root.join("app").join(path));
            paths.push(self.workspace_root.join("source").join(path));
            paths.push(self.workspace_root.join("sources").join(path));
            paths.push(self.workspace_root.join("include").join(path));
            paths.push(self.workspace_root.join("docs").join(path));
            paths.push(self.workspace_root.join("doc").join(path));
            paths.push(self.workspace_root.join("examples").join(path));
            paths.push(self.workspace_root.join("example").join(path));
            paths.push(self.workspace_root.join("tests").join(path));
            paths.push(self.workspace_root.join("test").join(path));
        }

        // Try case-insensitive variants for filenames
        if !path.contains('/') && !path.contains('\\') {
            if let Ok(entries) = std::fs::read_dir(&self.workspace_root) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.to_lowercase() == path.to_lowercase() {
                            paths.push(entry.path());
                        }
                    }
                }
            }
        }

        Ok(paths)
    }
}
