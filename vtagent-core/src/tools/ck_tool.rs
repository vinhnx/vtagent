//! Ck semantic search tool implementation for VTAgent
//!
//! This module provides a tool interface for the ck (semantic grep) tool,
//! allowing it to be used as a standard agent tool for semantic code search.

use super::traits::Tool;
use crate::config::constants::tools;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Ck semantic search tool that provides meaning-based code search
#[derive(Clone)]
pub struct CkTool {
    /// Workspace root for path resolution
    workspace_root: PathBuf,
}

impl CkTool {
    /// Create a new ck tool
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Get the workspace root
    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    /// Validate and normalize a path relative to workspace
    fn normalize_path(&self, path: &str) -> Result<String> {
        let path_buf = PathBuf::from(path);

        // If path is absolute, check if it's within workspace
        if path_buf.is_absolute() {
            if !path_buf.starts_with(&self.workspace_root) {
                return Err(anyhow::anyhow!(
                    "Path {} is outside workspace root {}",
                    path,
                    self.workspace_root.display()
                ));
            }
            Ok(path.to_string())
        } else {
            // Relative path - resolve relative to workspace
            let resolved = self.workspace_root.join(path);
            Ok(resolved.to_string_lossy().to_string())
        }
    }

    /// Execute semantic search operation
    async fn semantic_search(&self, args: Value) -> Result<Value> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .context("'query' is required for semantic search")?;

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let path = self.normalize_path(path)?;

        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let top_k = args
            .get("top_k")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let full_section = args
            .get("full_section")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let scores = args
            .get("scores")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Build ck command
        let mut cmd = Command::new("ck");
        cmd.arg("--sem").arg(query);
        cmd.arg("--jsonl"); // Use JSONL format for structured output

        if threshold > 0.0 {
            cmd.arg("--threshold").arg(threshold.to_string());
        }

        if let Some(k) = top_k {
            cmd.arg("--topk").arg(k.to_string());
        }

        if full_section {
            cmd.arg("--full-section");
        }

        if scores {
            cmd.arg("--scores");
        }

        cmd.arg(&path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .context("Failed to execute ck semantic search")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("ck command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Vec<Value> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line))
            .collect::<Result<Vec<Value>, _>>()
            .context("Failed to parse ck JSONL output")?;

        Ok(json!({
            "operation": "semantic_search",
            "query": query,
            "path": path,
            "threshold": threshold,
            "top_k": top_k,
            "full_section": full_section,
            "scores": scores,
            "results": results
        }))
    }

    /// Execute hybrid search operation (semantic + keyword)
    async fn hybrid_search(&self, args: Value) -> Result<Value> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .context("'query' is required for hybrid search")?;

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let path = self.normalize_path(path)?;

        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let top_k = args
            .get("top_k")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let scores = args
            .get("scores")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Build ck command
        let mut cmd = Command::new("ck");
        cmd.arg("--hybrid").arg(query);
        cmd.arg("--jsonl");

        if threshold > 0.0 {
            cmd.arg("--threshold").arg(threshold.to_string());
        }

        if let Some(k) = top_k {
            cmd.arg("--topk").arg(k.to_string());
        }

        if scores {
            cmd.arg("--scores");
        }

        cmd.arg(&path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .context("Failed to execute ck hybrid search")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("ck command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Vec<Value> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line))
            .collect::<Result<Vec<Value>, _>>()
            .context("Failed to parse ck JSONL output")?;

        Ok(json!({
            "operation": "hybrid_search",
            "query": query,
            "path": path,
            "threshold": threshold,
            "top_k": top_k,
            "scores": scores,
            "results": results
        }))
    }

    /// Execute regex search operation (traditional grep)
    async fn regex_search(&self, args: Value) -> Result<Value> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .context("'pattern' is required for regex search")?;

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let path = self.normalize_path(path)?;

        let case_insensitive = args
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let line_numbers = args
            .get("line_numbers")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let context_lines = args
            .get("context_lines")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        // Build ck command
        let mut cmd = Command::new("ck");

        if case_insensitive {
            cmd.arg("-i");
        }

        if line_numbers {
            cmd.arg("-n");
        }

        if let Some(context) = context_lines {
            cmd.arg("-C").arg(context.to_string());
        }

        cmd.arg(pattern);
        cmd.arg(&path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .context("Failed to execute ck regex search")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("ck command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        Ok(json!({
            "operation": "regex_search",
            "pattern": pattern,
            "path": path,
            "case_insensitive": case_insensitive,
            "line_numbers": line_numbers,
            "context_lines": context_lines,
            "output": stdout
        }))
    }

    /// Index the workspace for semantic search
    async fn index_workspace(&self, args: Value) -> Result<Value> {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let path = self.normalize_path(path)?;

        let exclude_patterns: Vec<String> = args
            .get("exclude_patterns")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        // Build ck index command
        let mut cmd = Command::new("ck");
        cmd.arg("--index");

        for pattern in &exclude_patterns {
            cmd.arg("--exclude").arg(pattern);
        }

        cmd.arg(&path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await.context("Failed to execute ck index")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("ck index command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        Ok(json!({
            "operation": "index_workspace",
            "path": path,
            "exclude_patterns": exclude_patterns,
            "output": stdout
        }))
    }

    /// Check index status
    async fn index_status(&self, args: Value) -> Result<Value> {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let path = self.normalize_path(path)?;

        let mut cmd = Command::new("ck");
        cmd.arg("--status");
        cmd.arg(&path);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output().await.context("Failed to execute ck status")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("ck status command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        Ok(json!({
            "operation": "index_status",
            "path": path,
            "output": stdout
        }))
    }

    /// Execute combined analysis operation (ck + ast-grep integration)
    async fn analyze_and_search(&self, args: Value) -> Result<Value> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .context("'query' is required for analyze_and_search operation")?;

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let path_normalized = self.normalize_path(path)?;

        let ast_grep_pattern = args.get("ast_grep_pattern").and_then(|v| v.as_str());
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);

        // Step 1: Use ck for semantic discovery
        let mut ck_cmd = Command::new("ck");
        ck_cmd.arg("--sem").arg(query);
        ck_cmd.arg("--jsonl");
        ck_cmd.arg("--topk").arg(max_results.to_string());
        ck_cmd.arg(&path_normalized);
        ck_cmd.stdout(Stdio::piped());
        ck_cmd.stderr(Stdio::piped());

        let ck_output = ck_cmd
            .output()
            .await
            .context("Failed to execute ck semantic search")?;

        if !ck_output.status.success() {
            let stderr = String::from_utf8_lossy(&ck_output.stderr);
            return Err(anyhow::anyhow!("ck command failed: {}", stderr));
        }

        let ck_stdout = String::from_utf8_lossy(&ck_output.stdout);
        let semantic_results: Vec<Value> = ck_stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line))
            .collect::<Result<Vec<Value>, _>>()
            .context("Failed to parse ck JSONL output")?;

        // Step 2: If ast-grep pattern provided, analyze semantic results with ast-grep
        let mut analysis_results = Vec::new();

        if let Some(pattern) = ast_grep_pattern {
            for result in &semantic_results {
                if let Some(file_path) = result.get("path").and_then(|v| v.as_str()) {
                    // Use ast-grep to analyze the file found by ck
                    let ast_result = self.analyze_file_with_ast_grep(file_path, pattern).await?;
                    if let Some(ast_data) = ast_result {
                        analysis_results.push(json!({
                            "semantic_match": result,
                            "ast_analysis": ast_data
                        }));
                    }
                }
            }
        }

        Ok(json!({
            "operation": "analyze_and_search",
            "query": query,
            "path": path_normalized,
            "semantic_results": semantic_results,
            "analysis_results": analysis_results,
            "integrated_analysis": !analysis_results.is_empty()
        }))
    }

    /// Analyze a specific file with ast-grep pattern
    async fn analyze_file_with_ast_grep(
        &self,
        file_path: &str,
        pattern: &str,
    ) -> Result<Option<Value>> {
        // This would integrate with the ast-grep tool
        // For now, return a placeholder structure
        // In a full implementation, this would call the AstGrepTool

        Ok(Some(json!({
            "file": file_path,
            "pattern": pattern,
            "analysis_type": "ast_grep_integration",
            "note": "AST-grep analysis would be performed here"
        })))
    }
}

#[async_trait]
impl Tool for CkTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let operation = args
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("semantic_search");

        match operation {
            "semantic_search" => self.semantic_search(args).await,
            "hybrid_search" => self.hybrid_search(args).await,
            "regex_search" => self.regex_search(args).await,
            "index_workspace" => self.index_workspace(args).await,
            "index_status" => self.index_status(args).await,
            "analyze_and_search" => self.analyze_and_search(args).await,
            _ => Err(anyhow::anyhow!("Unknown ck operation: {}", operation)),
        }
    }

    fn name(&self) -> &'static str {
        tools::CK_SEMANTIC_SEARCH
    }

    fn description(&self) -> &'static str {
        "Semantic code search using ck tool - find code by meaning, not just keywords"
    }

    fn validate_args(&self, args: &Value) -> Result<()> {
        if let Some(operation) = args.get("operation").and_then(|v| v.as_str()) {
            match operation {
                "semantic_search" => {
                    if args.get("query").is_none() {
                        return Err(anyhow::anyhow!(
                            "'query' is required for semantic_search operation"
                        ));
                    }
                }
                "hybrid_search" => {
                    if args.get("query").is_none() {
                        return Err(anyhow::anyhow!(
                            "'query' is required for hybrid_search operation"
                        ));
                    }
                }
                "regex_search" => {
                    if args.get("pattern").is_none() {
                        return Err(anyhow::anyhow!(
                            "'pattern' is required for regex_search operation"
                        ));
                    }
                }
                "analyze_and_search" => {
                    if args.get("query").is_none() {
                        return Err(anyhow::anyhow!(
                            "'query' is required for analyze_and_search operation"
                        ));
                    }
                    // ast_grep_pattern is optional for this operation
                }
                _ => {} // Other operations may have different requirements
            }
        }

        Ok(())
    }
}
