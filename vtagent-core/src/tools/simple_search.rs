//! Simple bash-like search tool
//!
//! This tool provides simple, direct search capabilities that act like
//! common bash commands: grep, find, ls, cat, etc.

use super::traits::Tool;
use crate::simple_indexer::{SimpleIndexer, SearchResult};
use crate::config::constants::tools;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;

/// Simple bash-like search tool
#[derive(Clone)]
pub struct SimpleSearchTool {
    indexer: SimpleIndexer,
}

impl SimpleSearchTool {
    /// Create a new simple search tool
    pub fn new(workspace_root: PathBuf) -> Self {
        let mut indexer = SimpleIndexer::new(workspace_root);
        indexer.init().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize indexer: {}", e);
        });

        Self { indexer }
    }

    /// Execute grep-like search
    async fn grep(&self, args: Value) -> Result<Value> {
        let pattern = args.get("pattern")
            .and_then(|v| v.as_str())
            .context("pattern is required for grep")?;

        let file_pattern = args.get("file_pattern").and_then(|v| v.as_str());
        let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

        let results = self.indexer.grep(pattern, file_pattern)?;

        let limited_results: Vec<SearchResult> = results.into_iter()
            .take(max_results)
            .collect();

        let formatted_results = limited_results.iter()
            .map(|r| format!("{}:{}:{}", r.file_path, r.line_number, r.line_content))
            .collect::<Vec<String>>();

        Ok(json!({
            "command": "grep",
            "pattern": pattern,
            "results": formatted_results,
            "count": limited_results.len()
        }))
    }

    /// Execute find-like file search
    async fn find(&self, args: Value) -> Result<Value> {
        let pattern = args.get("pattern")
            .and_then(|v| v.as_str())
            .context("pattern is required for find")?;

        let files = self.indexer.find_files(pattern)?;

        Ok(json!({
            "command": "find",
            "pattern": pattern,
            "files": files,
            "count": files.len()
        }))
    }

    /// Execute ls-like directory listing
    async fn ls(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let show_hidden = args.get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let files = self.indexer.list_files(path, show_hidden)?;

        Ok(json!({
            "command": "ls",
            "path": path,
            "files": files,
            "count": files.len()
        }))
    }

    /// Execute cat-like file content reading
    async fn cat(&self, args: Value) -> Result<Value> {
        let file_path = args.get("file_path")
            .and_then(|v| v.as_str())
            .context("file_path is required for cat")?;

        let start_line = args.get("start_line").and_then(|v| v.as_u64()).map(|v| v as usize);
        let end_line = args.get("end_line").and_then(|v| v.as_u64()).map(|v| v as usize);

        let content = self.indexer.get_file_content(file_path, start_line, end_line)?;

        Ok(json!({
            "command": "cat",
            "file_path": file_path,
            "content": content,
            "start_line": start_line,
            "end_line": end_line
        }))
    }

    /// Execute head-like file preview
    async fn head(&self, args: Value) -> Result<Value> {
        let file_path = args.get("file_path")
            .and_then(|v| v.as_str())
            .context("file_path is required for head")?;

        let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let content = self.indexer.get_file_content(file_path, Some(1), Some(lines))?;

        Ok(json!({
            "command": "head",
            "file_path": file_path,
            "content": content,
            "lines": lines
        }))
    }

    /// Execute tail-like file preview
    async fn tail(&self, args: Value) -> Result<Value> {
        let file_path = args.get("file_path")
            .and_then(|v| v.as_str())
            .context("file_path is required for tail")?;

        let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        // For tail, we need to know the total number of lines first
        let full_content = std::fs::read_to_string(file_path)?;
        let total_lines = full_content.lines().count();

        let start_line = total_lines.saturating_sub(lines) + 1;
        let content = self.indexer.get_file_content(file_path, Some(start_line), Some(total_lines))?;

        Ok(json!({
            "command": "tail",
            "file_path": file_path,
            "content": content,
            "lines": lines
        }))
    }

    /// Index files in directory
    async fn index(&mut self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let path_buf = PathBuf::from(path);
        self.indexer.index_directory(&path_buf)?;

        Ok(json!({
            "command": "index",
            "path": path,
            "status": "completed"
        }))
    }
}

#[async_trait]
impl Tool for SimpleSearchTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("grep");

        match command {
            "grep" => self.grep(args).await,
            "find" => self.find(args).await,
            "ls" => self.ls(args).await,
            "cat" => self.cat(args).await,
            "head" => self.head(args).await,
            "tail" => self.tail(args).await,
            _ => Err(anyhow::anyhow!("Unknown command: {}", command)),
        }
    }

    fn name(&self) -> &'static str {
        tools::SIMPLE_SEARCH
    }

    fn description(&self) -> &'static str {
        "Simple bash-like search and file operations: grep, find, ls, cat, head, tail, index"
    }
}