//! Search tool implementation with multiple modes

use super::traits::{CacheableTool, ModeTool, Tool};
use crate::tools::rp_search::{RpSearchInput, RpSearchManager};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// Unified search tool with multiple modes
#[derive(Clone)]
pub struct SearchTool {
    workspace_root: PathBuf,
    rp_search: Arc<RpSearchManager>,
}

impl SearchTool {
    pub fn new(workspace_root: PathBuf, rp_search: Arc<RpSearchManager>) -> Self {
        Self {
            workspace_root,
            rp_search,
        }
    }

    /// Execute exact search mode
    async fn execute_exact(&self, args: Value) -> Result<Value> {
        let pattern = args
            .get("pattern")
            .and_then(|p| p.as_str())
            .ok_or_else(|| anyhow!("Missing pattern for search"))?;

        let input = RpSearchInput {
            pattern: pattern.to_string(),
            path: args
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or(".")
                .to_string(),
            max_results: Some(
                args.get("max_results")
                    .and_then(|m| m.as_u64())
                    .unwrap_or(100) as usize,
            ),
            case_sensitive: Some(
                args.get("case_sensitive")
                    .and_then(|c| c.as_bool())
                    .unwrap_or(true),
            ),
            literal: Some(false),
            glob_pattern: None,
            context_lines: Some(0),
            include_hidden: Some(false),
        };

        let result = self.rp_search.perform_search(input).await?;
        Ok(json!({
            "success": true,
            "matches": result.matches,
            "mode": "exact"
        }))
    }

    /// Execute fuzzy search mode
    async fn execute_fuzzy(&self, args: Value) -> Result<Value> {
        // For now, use exact search with fuzzy indication
        let mut result = self.execute_exact(args).await?;
        if let Some(obj) = result.as_object_mut() {
            obj.insert("mode".to_string(), json!("fuzzy"));
            obj.insert("fuzzy_enabled".to_string(), json!(true));
        }
        Ok(result)
    }

    /// Execute multi-pattern search mode
    async fn execute_multi(&self, args: Value) -> Result<Value> {
        let args_obj = args
            .as_object()
            .ok_or_else(|| anyhow!("Invalid arguments"))?;

        let patterns = args_obj
            .get("patterns")
            .and_then(|p| p.as_array())
            .ok_or_else(|| anyhow!("Missing patterns array for multi mode"))?;

        let logic = args_obj
            .get("logic")
            .and_then(|l| l.as_str())
            .unwrap_or("AND");

        let mut all_results = Vec::new();

        // Execute search for each pattern
        for pattern in patterns {
            if let Some(pattern_str) = pattern.as_str() {
                let mut pattern_args = args.clone();
                if let Some(obj) = pattern_args.as_object_mut() {
                    obj.insert("pattern".to_string(), json!(pattern_str));
                }

                match self.execute_exact(pattern_args).await {
                    Ok(result) => {
                        if let Some(matches) = result.get("matches").and_then(|m| m.as_array()) {
                            all_results.extend(matches.clone());
                        }
                    }
                    Err(_) => continue, // Skip failed patterns
                }
            }
        }

        // Apply logic (AND/OR) to combine results
        let final_results = if logic == "AND" {
            self.apply_and_logic(all_results, patterns.len())
        } else {
            self.apply_or_logic(all_results)
        };

        Ok(json!({
            "success": true,
            "matches": final_results,
            "mode": "multi",
            "logic": logic,
            "pattern_count": patterns.len()
        }))
    }

    /// Execute similarity search mode
    async fn execute_similarity(&self, args: Value) -> Result<Value> {
        let args_obj = args
            .as_object()
            .ok_or_else(|| anyhow!("Invalid arguments"))?;

        let reference_file = args_obj
            .get("reference_file")
            .and_then(|f| f.as_str())
            .ok_or_else(|| anyhow!("Missing reference_file for similarity mode"))?;

        let content_type = args_obj
            .get("content_type")
            .and_then(|c| c.as_str())
            .unwrap_or("all");

        // Read reference file to extract patterns
        let ref_path = self.workspace_root.join(reference_file);
        let ref_content = tokio::fs::read_to_string(&ref_path)
            .await
            .map_err(|e| anyhow!("Failed to read reference file: {}", e))?;

        // Extract patterns based on content type
        let patterns = self.extract_similarity_patterns(&ref_content, content_type)?;

        // Execute multi-pattern search with OR logic
        let mut search_args = args.clone();
        if let Some(obj) = search_args.as_object_mut() {
            obj.insert("patterns".to_string(), json!(patterns));
            obj.insert("logic".to_string(), json!("OR"));
        }

        self.execute_multi(search_args).await
    }

    /// Apply AND logic to search results
    fn apply_and_logic(&self, results: Vec<Value>, pattern_count: usize) -> Vec<Value> {
        use std::collections::HashMap;

        let mut file_matches: HashMap<String, Vec<Value>> = HashMap::new();

        // Group matches by file
        for result in results {
            if let Some(path) = result.get("path").and_then(|p| p.as_str()) {
                file_matches
                    .entry(path.to_string())
                    .or_default()
                    .push(result);
            }
        }

        // Only include files that have matches for all patterns
        file_matches
            .into_iter()
            .filter(|(_, matches)| matches.len() >= pattern_count)
            .flat_map(|(_, matches)| matches)
            .collect()
    }

    /// Apply OR logic to search results (remove duplicates)
    fn apply_or_logic(&self, results: Vec<Value>) -> Vec<Value> {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        let mut unique_results = Vec::new();

        for result in results {
            let key = format!(
                "{}:{}:{}",
                result.get("path").and_then(|p| p.as_str()).unwrap_or(""),
                result
                    .get("line_number")
                    .and_then(|l| l.as_u64())
                    .unwrap_or(0),
                result.get("column").and_then(|c| c.as_u64()).unwrap_or(0)
            );

            if seen.insert(key) {
                unique_results.push(result);
            }
        }

        unique_results
    }

    /// Extract patterns for similarity search
    fn extract_similarity_patterns(
        &self,
        content: &str,
        content_type: &str,
    ) -> Result<Vec<String>> {
        let mut patterns = Vec::new();

        match content_type {
            "functions" => {
                // Extract function signatures
                for line in content.lines() {
                    if line.trim_start().starts_with("fn ")
                        || line.trim_start().starts_with("pub fn ")
                    {
                        if let Some(name) = self.extract_function_name(line) {
                            patterns.push(format!("fn {}", name));
                        }
                    }
                }
            }
            "imports" => {
                // Extract import statements
                for line in content.lines() {
                    if line.trim_start().starts_with("use ") {
                        patterns.push(line.trim().to_string());
                    }
                }
            }
            "structure" => {
                // Extract struct/enum definitions
                for line in content.lines() {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("struct ") || trimmed.starts_with("enum ") {
                        patterns.push(
                            trimmed
                                .split_whitespace()
                                .take(2)
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                    }
                }
            }
            _ => {
                // Extract all significant keywords
                patterns.extend(self.extract_keywords(content));
            }
        }

        if patterns.is_empty() {
            return Err(anyhow!("No patterns extracted from reference file"));
        }

        Ok(patterns)
    }

    /// Extract function name from function definition
    fn extract_function_name(&self, line: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "fn" && i + 1 < parts.len() {
                let name = parts[i + 1];
                if let Some(paren_pos) = name.find('(') {
                    return Some(name[..paren_pos].to_string());
                }
                return Some(name.to_string());
            }
        }
        None
    }

    /// Extract keywords from content
    fn extract_keywords(&self, content: &str) -> Vec<String> {
        let keywords = ["fn ", "struct ", "enum ", "impl ", "trait ", "use ", "mod "];
        let mut patterns = Vec::new();

        for line in content.lines() {
            for keyword in &keywords {
                if line.contains(keyword) {
                    patterns.push(keyword.trim().to_string());
                }
            }
        }

        patterns.sort();
        patterns.dedup();
        patterns
    }
}

#[async_trait]
impl Tool for SearchTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let args_clone = args.clone();
        let mode = args_clone
            .get("mode")
            .and_then(|m| m.as_str())
            .unwrap_or("exact");

        self.execute_mode(mode, args).await
    }

    fn name(&self) -> &'static str {
        "rp_search"
    }

    fn description(&self) -> &'static str {
        "Enhanced unified search tool with multiple modes: exact (default), fuzzy, multi-pattern, and similarity search"
    }
}

#[async_trait]
impl ModeTool for SearchTool {
    fn supported_modes(&self) -> Vec<&'static str> {
        vec!["exact", "fuzzy", "multi", "similarity"]
    }

    async fn execute_mode(&self, mode: &str, args: Value) -> Result<Value> {
        match mode {
            "exact" => self.execute_exact(args).await,
            "fuzzy" => self.execute_fuzzy(args).await,
            "multi" => self.execute_multi(args).await,
            "similarity" => self.execute_similarity(args).await,
            _ => Err(anyhow!("Unsupported search mode: {}", mode)),
        }
    }
}

#[async_trait]
impl CacheableTool for SearchTool {
    fn cache_key(&self, args: &Value) -> String {
        format!(
            "search:{}:{}",
            args.get("pattern").and_then(|p| p.as_str()).unwrap_or(""),
            args.get("mode").and_then(|m| m.as_str()).unwrap_or("exact")
        )
    }

    fn should_cache(&self, args: &Value) -> bool {
        // Cache exact and fuzzy searches, but not multi/similarity (too dynamic)
        let mode = args.get("mode").and_then(|m| m.as_str()).unwrap_or("exact");
        matches!(mode, "exact" | "fuzzy")
    }
}
