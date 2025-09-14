//! Tool registry and function declarations

use super::apply_patch::Patch;
use super::bash_tool::BashTool;
use super::cache::FILE_CACHE;
use super::command::CommandTool;
use super::file_ops::FileOpsTool;
use super::search::SearchTool;
use super::simple_search::SimpleSearchTool;
use super::traits::Tool;
use crate::config::PtyConfig;
use crate::config::constants::tools;
use crate::config::loader::ConfigManager;
use crate::config::types::CapabilityLevel;
use crate::gemini::FunctionDeclaration;
use crate::tool_policy::{ToolPolicy, ToolPolicyManager};
use crate::tools::ast_grep::AstGrepEngine;
use crate::tools::grep_search::GrepSearchManager;
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Main tool registry that coordinates all tools
#[derive(Clone)]
pub struct ToolRegistry {
    workspace_root: PathBuf,
    search_tool: SearchTool,
    simple_search_tool: SimpleSearchTool,
    bash_tool: BashTool,
    file_ops_tool: FileOpsTool,
    command_tool: CommandTool,
    // Removed stored grep_search (no longer needed as a field)
    ast_grep_engine: Option<Arc<AstGrepEngine>>,
    tool_policy: ToolPolicyManager,
    pty_config: PtyConfig,
    active_pty_sessions: Arc<AtomicUsize>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new(workspace_root: PathBuf) -> Self {
        Self::new_with_config(workspace_root, PtyConfig::default())
    }

    /// Create a new tool registry with PTY configuration
    pub fn new_with_config(workspace_root: PathBuf, pty_config: PtyConfig) -> Self {
        let grep_search = Arc::new(GrepSearchManager::new(workspace_root.clone()));

        let search_tool = SearchTool::new(workspace_root.clone(), grep_search.clone());
        let simple_search_tool = SimpleSearchTool::new(workspace_root.clone());
        let bash_tool = BashTool::new(workspace_root.clone());
        let file_ops_tool = FileOpsTool::new(workspace_root.clone(), grep_search.clone());
        let command_tool = CommandTool::new(workspace_root.clone());

        // Initialize AST-grep engine
        let ast_grep_engine = match AstGrepEngine::new() {
            Ok(engine) => Some(Arc::new(engine)),
            Err(e) => {
                eprintln!("Warning: Failed to initialize AST-grep engine: {}", e);
                None
            }
        };

        // Initialize policy manager and update available tools
        let mut policy_manager = ToolPolicyManager::new_with_workspace(&workspace_root)
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to initialize tool policy manager: {}", e);
                // Create a fallback that allows all tools
                ToolPolicyManager::new().unwrap()
            });

        // Update available tools in policy manager
        let mut available_tools = vec![
            tools::GREP_SEARCH.to_string(),
            tools::LIST_FILES.to_string(),
            tools::RUN_TERMINAL_CMD.to_string(),
            tools::READ_FILE.to_string(),
            tools::WRITE_FILE.to_string(),
            tools::EDIT_FILE.to_string(),
            tools::BASH.to_string(),
            "apply_patch".to_string(),
        ];

        // Add AST-grep tool if available
        if ast_grep_engine.is_some() {
            available_tools.push(tools::AST_GREP_SEARCH.to_string());
        }

        if let Err(e) = policy_manager.update_available_tools(available_tools) {
            eprintln!("Warning: Failed to update tool policies: {}", e);
        }

        Self {
            workspace_root,
            search_tool,
            simple_search_tool,
            bash_tool,
            file_ops_tool,
            command_tool,
            ast_grep_engine,
            tool_policy: policy_manager,
            pty_config,
            active_pty_sessions: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Set AST-grep engine
    pub fn with_ast_grep(mut self, engine: Arc<AstGrepEngine>) -> Self {
        self.ast_grep_engine = Some(engine);
        self
    }

    /// Get workspace root
    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    /// Initialize async components
    pub async fn initialize_async(&mut self) -> Result<()> {
        // Currently no async initialization needed
        // This method exists for API compatibility
        Ok(())
    }

    /// Execute a tool by name with policy checking
    pub async fn execute_tool(&mut self, name: &str, args: Value) -> Result<Value> {
        // Check tool policy before execution
        if !self.policy_manager_mut().should_execute_tool(name)? {
            return Err(anyhow!("Tool '{}' execution denied by policy", name));
        }

        // Apply optional scoped constraints from policy
        let args = self.apply_policy_constraints(name, args)?;

        // Check PTY session limits for PTY-based tools
        let is_pty_tool = matches!(name, tools::RUN_TERMINAL_CMD | tools::BASH);
        if is_pty_tool {
            self.start_pty_session()?;
        }

        let result = match name {
            tools::GREP_SEARCH => self.search_tool.execute(args).await,
            tools::LIST_FILES => self.file_ops_tool.execute(args).await,
            tools::RUN_TERMINAL_CMD => self.command_tool.execute(args).await,
            tools::READ_FILE => self.file_ops_tool.read_file(args).await,
            tools::WRITE_FILE => self.file_ops_tool.write_file(args).await,
            tools::EDIT_FILE => self.edit_file(args).await,
            tools::AST_GREP_SEARCH => self.execute_ast_grep(args).await,
            tools::SIMPLE_SEARCH => self.simple_search_tool.execute(args).await,
            tools::BASH => self.bash_tool.execute(args).await,
            "apply_patch" => self.execute_apply_patch(args).await,
            _ => Err(anyhow!("Unknown tool: {}", name)),
        };

        // Decrement session count if this was a PTY tool
        if is_pty_tool {
            self.end_pty_session();
        }

        result
    }

    async fn execute_apply_patch(&self, args: Value) -> Result<Value> {
        let input = args
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Error: Missing 'input' string with patch content. Example: apply_patch({{ input: '*** Begin Patch...*** End Patch' }})"))?;
        let patch = Patch::parse(input)?;
        let results = patch.apply(&self.workspace_root).await?;
        Ok(json!({
            "success": true,
            "applied": results,
        }))
    }

    /// Apply optional scoped constraints from tool policy to arguments to improve safety
    fn apply_policy_constraints(&self, name: &str, mut args: Value) -> Result<Value> {
        if let Some(constraints) = self.tool_policy.get_constraints(name).cloned() {
            let obj = args
                .as_object_mut()
                .ok_or_else(|| anyhow!("Error: tool arguments must be an object"))?;

            // Default response_format
            if let Some(fmt) = constraints.default_response_format {
                obj.entry("response_format").or_insert(json!(fmt));
            }

            // Allowed modes
            if let Some(allowed) = constraints.allowed_modes {
                if let Some(mode) = obj.get("mode").and_then(|v| v.as_str()) {
                    if !allowed.iter().any(|m| m == mode) {
                        return Err(anyhow!(format!(
                            "Mode '{}' not allowed by policy for '{}'. Allowed: {}",
                            mode,
                            name,
                            allowed.join(", ")
                        )));
                    }
                }
            }

            // Tool-specific caps
            match name {
                n if n == tools::LIST_FILES => {
                    if let Some(cap) = constraints.max_items_per_call {
                        let requested = obj
                            .get("max_items")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(cap as u64) as usize;
                        if requested > cap {
                            obj.insert("max_items".to_string(), json!(cap));
                            obj.insert(
                                "_policy_note".to_string(),
                                json!(format!("Capped max_items to {} by policy", cap)),
                            );
                        }
                    }
                }
                n if n == tools::GREP_SEARCH => {
                    if let Some(cap) = constraints.max_results_per_call {
                        let requested = obj
                            .get("max_results")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(cap as u64) as usize;
                        if requested > cap {
                            obj.insert("max_results".to_string(), json!(cap));
                            obj.insert(
                                "_policy_note".to_string(),
                                json!(format!("Capped max_results to {} by policy", cap)),
                            );
                        }
                    }
                }
                n if n == tools::READ_FILE => {
                    if let Some(cap) = constraints.max_bytes_per_read {
                        let requested = obj
                            .get("max_bytes")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(cap as u64) as usize;
                        if requested > cap {
                            obj.insert("max_bytes".to_string(), json!(cap));
                            obj.insert(
                                "_policy_note".to_string(),
                                json!(format!("Capped max_bytes to {} by policy", cap)),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(args)
    }

    /// List available tools
    pub fn available_tools(&self) -> Vec<String> {
        let mut tools = vec![
            tools::GREP_SEARCH.to_string(),
            tools::LIST_FILES.to_string(),
            tools::RUN_TERMINAL_CMD.to_string(),
            tools::READ_FILE.to_string(),
            tools::WRITE_FILE.to_string(),
            tools::EDIT_FILE.to_string(),
            "simple_search".to_string(),
            "bash".to_string(),
        ];

        // Add AST-grep tool if available
        if self.ast_grep_engine.is_some() {
            tools.push(tools::AST_GREP_SEARCH.to_string());
        }

        tools
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        match name {
            tools::GREP_SEARCH
            | tools::LIST_FILES
            | tools::RUN_TERMINAL_CMD
            | tools::READ_FILE
            | tools::WRITE_FILE => true,
            tools::AST_GREP_SEARCH => self.ast_grep_engine.is_some(),
            tools::SIMPLE_SEARCH | tools::BASH => true,
            _ => false,
        }
    }

    /// Get tool policy manager (mutable reference)
    pub fn policy_manager_mut(&mut self) -> &mut ToolPolicyManager {
        &mut self.tool_policy
    }

    /// Get tool policy manager (immutable reference)
    pub fn policy_manager(&self) -> &ToolPolicyManager {
        &self.tool_policy
    }

    /// Set policy for a specific tool
    pub fn set_tool_policy(&mut self, tool_name: &str, policy: ToolPolicy) -> Result<()> {
        self.tool_policy.set_policy(tool_name, policy)
    }

    /// Get policy for a specific tool
    pub fn get_tool_policy(&self, tool_name: &str) -> ToolPolicy {
        self.tool_policy.get_policy(tool_name)
    }

    /// Reset all tool policies to prompt
    pub fn reset_tool_policies(&mut self) -> Result<()> {
        self.tool_policy.reset_all_to_prompt()
    }

    /// Allow all tools
    pub fn allow_all_tools(&mut self) -> Result<()> {
        self.tool_policy.allow_all_tools()
    }

    /// Deny all tools
    pub fn deny_all_tools(&mut self) -> Result<()> {
        self.tool_policy.deny_all_tools()
    }

    /// Print tool policy status
    pub fn print_tool_policy_status(&self) {
        self.tool_policy.print_status();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> serde_json::Value {
        let stats = FILE_CACHE.stats().await;
        json!({
            "hits": stats.hits,
            "misses": stats.misses,
            "entries": stats.entries,
            "total_size_bytes": stats.total_size_bytes,
            "hit_rate": if stats.hits + stats.misses > 0 {
                stats.hits as f64 / (stats.hits + stats.misses) as f64
            } else { 0.0 }
        })
    }

    /// Clear all caches
    pub async fn clear_cache(&self) {
        FILE_CACHE.clear().await;
    }

    // Legacy methods for backward compatibility
    pub async fn read_file(&mut self, args: Value) -> Result<Value> {
        self.execute_tool(tools::READ_FILE, args).await
    }

    pub async fn write_file(&mut self, args: Value) -> Result<Value> {
        self.execute_tool(tools::WRITE_FILE, args).await
    }

    pub async fn edit_file(&mut self, args: Value) -> Result<Value> {
        use crate::tools::types::EditInput;

        let input: EditInput = serde_json::from_value(args).context("invalid edit_file args")?;

        // Read the current file content
        let read_args = json!({
            "path": input.path
        });

        let read_result = self.file_ops_tool.read_file(read_args).await?;
        let current_content = read_result["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to read file content"))?;

        // Try multiple matching strategies for better compatibility
        let mut replacement_occurred = false;
        let mut new_content = current_content.to_string();

        // Strategy 1: Exact match (original behavior)
        if current_content.contains(&input.old_str) {
            new_content = current_content.replace(&input.old_str, &input.new_str);
            replacement_occurred = new_content != current_content;
        }

        // Strategy 2: If exact match failed, try with normalized whitespace
        if !replacement_occurred {
            let normalized_content = Self::normalize_whitespace(current_content);
            let normalized_old_str = Self::normalize_whitespace(&input.old_str);

            if normalized_content.contains(&normalized_old_str) {
                // Find the position in original content that corresponds to the normalized match
                // This is a simplified approach - in practice, we'd need more sophisticated diffing
                let old_lines: Vec<&str> = input.old_str.lines().collect();
                let content_lines: Vec<&str> = current_content.lines().collect();

                // Try to find a sequence of lines that match
                for i in 0..=(content_lines.len().saturating_sub(old_lines.len())) {
                    let window = &content_lines[i..i + old_lines.len()];
                    if Self::lines_match(window, &old_lines) {
                        // Found a match, reconstruct the replacement
                        let before = content_lines[..i].join("\n");
                        let after = content_lines[i + old_lines.len()..].join("\n");
                        let replacement_lines: Vec<&str> = input.new_str.lines().collect();

                        new_content =
                            format!("{}\n{}\n{}", before, replacement_lines.join("\n"), after);
                        replacement_occurred = true;
                        break;
                    }
                }
            }
        }

        // If no replacement occurred, provide detailed error
        if !replacement_occurred {
            let content_preview = if current_content.len() > 500 {
                format!(
                    "{}...{}",
                    &current_content[..250],
                    &current_content[current_content.len().saturating_sub(250)..]
                )
            } else {
                current_content.to_string()
            };

            return Err(anyhow!(
                "Could not find text to replace in file.\n\nExpected to replace:\n{}\n\nFile content preview:\n{}",
                input.old_str,
                content_preview
            ));
        }

        // Write the modified content back
        let write_args = json!({
            "path": input.path,
            "content": new_content,
            "mode": "overwrite"
        });

        self.file_ops_tool.write_file(write_args).await
    }

    /// Normalize whitespace for more flexible string matching
    /// This handles common formatting differences like trailing spaces and indentation
    fn normalize_whitespace(s: &str) -> String {
        s.lines()
            .map(|line| {
                // Trim trailing whitespace but preserve leading indentation
                let trimmed = line.trim_end();
                // Only trim leading whitespace if the line is not empty
                if trimmed.is_empty() {
                    trimmed.to_string()
                } else {
                    // Preserve the line as-is but normalize trailing whitespace
                    trimmed.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if lines match, allowing for whitespace differences
    fn lines_match(content_lines: &[&str], expected_lines: &[&str]) -> bool {
        if content_lines.len() != expected_lines.len() {
            return false;
        }

        content_lines
            .iter()
            .zip(expected_lines.iter())
            .all(|(content_line, expected_line)| {
                content_line.trim() == expected_line.trim()
            })
    }

    pub async fn delete_file(&mut self, _args: Value) -> Result<Value> {
        Err(anyhow!("delete_file not yet implemented in modular system"))
    }

    pub async fn rp_search(&mut self, args: Value) -> Result<Value> {
        self.execute_tool(tools::GREP_SEARCH, args).await
    }

    pub async fn list_files(&mut self, args: Value) -> Result<Value> {
        self.execute_tool(tools::LIST_FILES, args).await
    }

    pub async fn run_terminal_cmd(&mut self, args: Value) -> Result<Value> {
        // Enforce command policies before delegating
        let cfg = ConfigManager::load()
            .or_else(|_| ConfigManager::load_from_workspace("."))
            .or_else(|_| ConfigManager::load_from_file("vtagent.toml"))
            .map(|cm| cm.config().clone())
            .unwrap_or_default();

        let mut args = args;
        // Try to extract the command text for policy checking
        let cmd_text = if let Some(cmd_val) = args.get("command") {
            if cmd_val.is_array() {
                cmd_val
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                cmd_val.as_str().unwrap_or("").to_string()
            }
        } else {
            String::new()
        };

        // Deny regex
        for pat in &cfg.commands.deny_regex {
            if Regex::new(pat)
                .ok()
                .map(|re| re.is_match(&cmd_text))
                .unwrap_or(false)
            {
                return Err(anyhow!("Command denied by regex policy: {}", pat));
            }
        }
        // Deny glob (convert basic * to .*)
        for pat in &cfg.commands.deny_glob {
            let re = format!("^{}$", regex::escape(pat).replace(r"\*", ".*"));
            if Regex::new(&re)
                .ok()
                .map(|re| re.is_match(&cmd_text))
                .unwrap_or(false)
            {
                return Err(anyhow!("Command denied by glob policy: {}", pat));
            }
        }
        // Exact deny list
        for d in &cfg.commands.deny_list {
            if cmd_text.starts_with(d) {
                return Err(anyhow!("Command denied by policy: {}", d));
            }
        }

        // Allow: if allow_regex/glob present, require one match
        let mut allow_ok =
            cfg.commands.allow_regex.is_empty() && cfg.commands.allow_glob.is_empty();
        if !allow_ok {
            if cfg.commands.allow_regex.iter().any(|pat| {
                Regex::new(pat)
                    .ok()
                    .map(|re| re.is_match(&cmd_text))
                    .unwrap_or(false)
            }) {
                allow_ok = true;
            }
            if !allow_ok
                && cfg.commands.allow_glob.iter().any(|pat| {
                    let re = format!("^{}$", regex::escape(pat).replace(r"\*", ".*"));
                    Regex::new(&re)
                        .ok()
                        .map(|re| re.is_match(&cmd_text))
                        .unwrap_or(false)
                })
            {
                allow_ok = true;
            }
        }
        if !allow_ok {
            // Fall back to exact allow_list if provided
            if !cfg.commands.allow_list.is_empty() {
                allow_ok = cfg
                    .commands
                    .allow_list
                    .iter()
                    .any(|p| cmd_text.starts_with(p));
            }
        }
        if !allow_ok {
            return Err(anyhow!("Command not allowed by policy"));
        }

        // Clamp working dir by injecting cwd if not set
        if args.get("cwd").is_none() {
            args.as_object_mut().map(|m| {
                m.insert(
                    "cwd".to_string(),
                    json!(self.workspace_root.display().to_string()),
                );
            });
        }

        self.execute_tool(tools::RUN_TERMINAL_CMD, args).await
    }

    /// Execute AST-grep tool
    async fn execute_ast_grep(&self, args: Value) -> Result<Value> {
        let engine = self
            .ast_grep_engine
            .as_ref()
            .ok_or_else(|| anyhow!("AST-grep engine not available"))?;

        let operation = args
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("search");

        let mut out = match operation {
            "search" => {
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .context("'pattern' is required")?;

                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let context_lines = args
                    .get("context_lines")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let max_results = args
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);

                engine
                    .search(pattern, &path, language, context_lines, max_results)
                    .await
            }
            "transform" => {
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .context("'pattern' is required")?;

                let replacement = args
                    .get("replacement")
                    .and_then(|v| v.as_str())
                    .context("'replacement' is required")?;

                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let preview_only = args
                    .get("preview_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let update_all = args
                    .get("update_all")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                engine
                    .transform(
                        pattern,
                        replacement,
                        &path,
                        language,
                        preview_only,
                        update_all,
                    )
                    .await
            }
            "lint" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let severity_filter = args.get("severity_filter").and_then(|v| v.as_str());

                engine.lint(&path, language, severity_filter, None).await
            }
            "refactor" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let refactor_type = args
                    .get("refactor_type")
                    .and_then(|v| v.as_str())
                    .context("'refactor_type' is required")?;

                engine.refactor(&path, language, refactor_type).await
            }
            "custom" => {
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .context("'pattern' is required")?;

                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let rewrite = args.get("rewrite").and_then(|v| v.as_str());
                let context_lines = args
                    .get("context_lines")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let max_results = args
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                let interactive = args
                    .get("interactive")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let update_all = args
                    .get("update_all")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                engine
                    .run_custom(
                        pattern,
                        &path,
                        language,
                        rewrite,
                        context_lines,
                        max_results,
                        interactive,
                        update_all,
                    )
                    .await
            }
            _ => Err(anyhow!("Unknown AST-grep operation: {}", operation)),
        }?;

        // Optional concise transform
        let fmt = args
            .get("response_format")
            .and_then(|v| v.as_str())
            .unwrap_or("concise");
        if fmt.eq_ignore_ascii_case("concise") {
            if let Some(matches) = out.get_mut("matches") {
                let concise = Self::astgrep_to_concise(matches.take());
                out["matches"] = concise;
                out["response_format"] = json!("concise");
            } else if let Some(results) = out.get_mut("results") {
                let concise = Self::astgrep_to_concise(results.take());
                out["results"] = concise;
                out["response_format"] = json!("concise");
            } else if let Some(issues) = out.get_mut("issues") {
                let concise = Self::astgrep_issues_to_concise(issues.take());
                out["issues"] = concise;
                out["response_format"] = json!("concise");
            } else if let Some(suggestions) = out.get_mut("suggestions") {
                let concise = Self::astgrep_changes_to_concise(suggestions.take());
                out["suggestions"] = concise;
                out["response_format"] = json!("concise");
            } else if let Some(changes) = out.get_mut("changes") {
                let concise = Self::astgrep_changes_to_concise(changes.take());
                out["changes"] = concise;
                out["response_format"] = json!("concise");
            }
        } else {
            out["response_format"] = json!("detailed");
        }

        Ok(out)
    }

    /// Normalize a path relative to workspace
    fn normalize_path(&self, path: &str) -> Result<String> {
        let path_buf = PathBuf::from(path);

        // If path is absolute, check if it's within workspace
        if path_buf.is_absolute() {
            if !path_buf.starts_with(&self.workspace_root) {
                return Err(anyhow!(
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

    /// Get PTY configuration
    pub fn pty_config(&self) -> &PtyConfig {
        &self.pty_config
    }

    /// Check if a new PTY session can be started
    pub fn can_start_pty_session(&self) -> bool {
        if !self.pty_config.enabled {
            return false;
        }
        self.active_pty_sessions.load(Ordering::SeqCst) < self.pty_config.max_sessions
    }

    /// Increment active PTY session count
    pub fn start_pty_session(&self) -> Result<()> {
        if !self.can_start_pty_session() {
            return Err(anyhow!(
                "Maximum PTY sessions ({}) exceeded. Current active sessions: {}",
                self.pty_config.max_sessions,
                self.active_pty_sessions.load(Ordering::SeqCst)
            ));
        }
        self.active_pty_sessions.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Decrement active PTY session count
    pub fn end_pty_session(&self) {
        let current = self.active_pty_sessions.load(Ordering::SeqCst);
        if current > 0 {
            self.active_pty_sessions.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// Get current active PTY session count
    pub fn active_pty_sessions(&self) -> usize {
        self.active_pty_sessions.load(Ordering::SeqCst)
    }
}

impl ToolRegistry {
    // Best-effort concise mapping for ast-grep outputs from various operations
    fn astgrep_to_concise(v: Value) -> Value {
        let mut out = Vec::new();
        match v {
            Value::Array(arr) => {
                for item in arr.into_iter() {
                    let mut path = None;
                    let mut line = None;
                    let mut text = None;

                    // Common shapes
                    if let Some(p) = item.get("path").and_then(|p| p.as_str()) {
                        path = Some(p.to_string());
                    }
                    if line.is_none() {
                        line = item
                            .get("range")
                            .and_then(|r| r.get("start"))
                            .and_then(|s| s.get("line"))
                            .and_then(|l| l.as_u64())
                            .or(item
                                .get("start")
                                .and_then(|s| s.get("line"))
                                .and_then(|l| l.as_u64()));
                    }
                    if text.is_none() {
                        text = item
                            .get("text")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                            .or(item
                                .get("lines")
                                .and_then(|l| l.get("text"))
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()))
                            .or(item
                                .get("matched")
                                .and_then(|t| t.as_str())
                                .map(|s| s.to_string()));
                    }

                    out.push(json!({
                        "path": path.unwrap_or_default(),
                        "line_number": line.unwrap_or(0),
                        "text": text.unwrap_or_default(),
                    }));
                }
                Value::Array(out)
            }
            other => other,
        }
    }

    // Map ast-grep lint issues into concise entries
    fn astgrep_issues_to_concise(v: Value) -> Value {
        let mut out = Vec::new();
        match v {
            Value::Array(arr) => {
                for item in arr.into_iter() {
                    let path = item
                        .get("path")
                        .or_else(|| item.get("file"))
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_string();
                    let line = item
                        .get("range")
                        .and_then(|r| r.get("start"))
                        .and_then(|s| s.get("line"))
                        .and_then(|l| l.as_u64())
                        .or(item
                            .get("start")
                            .and_then(|s| s.get("line"))
                            .and_then(|l| l.as_u64()))
                        .or(item.get("line").and_then(|l| l.as_u64()))
                        .unwrap_or(0);
                    let message = item
                        .get("message")
                        .and_then(|m| m.as_str())
                        .or(item.get("text").and_then(|t| t.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let severity = item.get("severity").and_then(|s| s.as_str()).unwrap_or("");
                    let rule = item
                        .get("rule")
                        .or_else(|| item.get("rule_id"))
                        .and_then(|r| r.as_str())
                        .unwrap_or("");
                    out.push(json!({
                        "path": path,
                        "line_number": line,
                        "message": message,
                        "severity": severity,
                        "rule": rule,
                    }));
                }
                Value::Array(out)
            }
            other => other,
        }
    }

    // Map ast-grep refactor/transform suggestions/changes into concise entries
    fn astgrep_changes_to_concise(v: Value) -> Value {
        let mut out = Vec::new();
        match v {
            Value::Array(arr) => {
                for item in arr.into_iter() {
                    let path = item
                        .get("path")
                        .or_else(|| item.get("file"))
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_string();
                    let line = item
                        .get("range")
                        .and_then(|r| r.get("start"))
                        .and_then(|s| s.get("line"))
                        .and_then(|l| l.as_u64())
                        .or(item
                            .get("start")
                            .and_then(|s| s.get("line"))
                            .and_then(|l| l.as_u64()))
                        .or(item.get("line").and_then(|l| l.as_u64()))
                        .unwrap_or(0);
                    // Try different fields to summarize change
                    let before = item
                        .get("text")
                        .and_then(|t| t.as_str())
                        .or(item.get("matched").and_then(|t| t.as_str()))
                        .or(item.get("before").and_then(|t| t.as_str()))
                        .unwrap_or("");
                    let after = item
                        .get("replacement")
                        .and_then(|t| t.as_str())
                        .or(item.get("after").and_then(|t| t.as_str()))
                        .unwrap_or("");
                    let note = if !after.is_empty() {
                        format!("{} -> {}", truncate(before, 80), truncate(after, 80))
                    } else {
                        truncate(before, 120)
                    };
                    out.push(json!({
                        "path": path,
                        "line_number": line,
                        "note": note,
                    }));
                }
                Value::Array(out)
            }
            other => other,
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max {
            break;
        }
        out.push(ch);
    }
    out.push_str("…");
    out
}

/// Build function declarations for all available tools
pub fn build_function_declarations() -> Vec<FunctionDeclaration> {
    vec![
        // Ripgrep search tool
        FunctionDeclaration {
            name: tools::GREP_SEARCH.to_string(),
            description: "code_search: Unified grep-based search. Modes: 'exact' (default), 'fuzzy', 'multi', 'similarity'. Returns concise results by default; set response_format='detailed' for raw ripgrep JSON. Example: grep_search({pattern:'TODO|FIXME', path:'src', max_results:100, response_format:'concise'}).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Search pattern. Example: 'fn \\w+' or 'TODO|FIXME'"},
                    "path": {"type": "string", "description": "Directory path to search in (relative). Default: '.'", "default": "."},
                    "mode": {"type": "string", "description": "Search mode: 'exact' | 'fuzzy' | 'multi' | 'similarity'", "default": "exact"},
                    "max_results": {"type": "integer", "description": "Max results (token efficiency). Default: 100", "default": 100},
                    "case_sensitive": {"type": "boolean", "description": "Case sensitive search. Default: true", "default": true},
                    // Multi-pattern search parameters
                    "patterns": {"type": "array", "items": {"type": "string"}, "description": "For mode='multi'. Example: ['fn \\w+','use \\w+']"},
                    "logic": {"type": "string", "description": "For mode='multi': 'AND' or 'OR'", "default": "AND"},
                    // Fuzzy search parameters
                    "fuzzy_threshold": {"type": "number", "description": "Fuzzy matching threshold (0.0-1.0)", "default": 0.7},
                    // Similarity search parameters
                    "reference_file": {"type": "string", "description": "For mode='similarity': reference file path"},
                    "content_type": {"type": "string", "description": "For mode='similarity': 'structure'|'imports'|'functions'|'all'", "default": "all"},
                    "response_format": {"type": "string", "description": "'concise' (default) or 'detailed' (raw rg JSON)", "default": "concise"}
                },
                "required": ["pattern"]
            }),
        },

        // Consolidated file operations tool
        FunctionDeclaration {
            name: "list_files".to_string(),
            description: "fs_list: File discovery. Modes: 'list' (default), 'recursive', 'find_name', 'find_content'. Supports pagination and response_format for token efficiency. Example: list_files({path:'src', page:1, per_page:50, response_format:'concise'}).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to search from (relative). Example: 'src'"},
                    "mode": {"type": "string", "description": "'list' | 'recursive' | 'find_name' | 'find_content'", "default": "list"},
                    "max_items": {"type": "integer", "description": "Cap total items scanned (token safety). Default: 1000", "default": 1000},
                    "page": {"type": "integer", "description": "Page number (1-based). Default: 1", "default": 1},
                    "per_page": {"type": "integer", "description": "Items per page. Default: 100", "default": 100},
                    "response_format": {"type": "string", "description": "'concise' (default) omits low-signal fields; 'detailed' includes them", "default": "concise"},
                    "include_hidden": {"type": "boolean", "description": "Include hidden files", "default": false},
                    "name_pattern": {"type": "string", "description": "For 'recursive'/'find_name' modes. Example: '*.rs'"},
                    "content_pattern": {"type": "string", "description": "For 'find_content' mode. Example: 'fn main'"},
                    "file_extensions": {"type": "array", "items": {"type": "string"}, "description": "Filter by file extensions"},
                    "case_sensitive": {"type": "boolean", "description": "Case sensitive pattern matching", "default": true},
                    "ast_grep_pattern": {"type": "string", "description": "Optional AST pattern to filter files"}
                },
                "required": ["path"]
            }),
        },

        // File reading tool
        FunctionDeclaration {
            name: tools::READ_FILE.to_string(),
            description: "fs_read: Read file contents. Use max_bytes to limit tokens. Example: read_file({path:'src/main.rs', max_bytes:20000}).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"}
                },
                "required": ["path"]
            }),
        },

        // File writing tool
        FunctionDeclaration {
            name: tools::WRITE_FILE.to_string(),
            description: "fs_write: Write content to a file. Modes: 'overwrite' (default) | 'append' | 'skip_if_exists'. Example: write_file({path:'README.md', content:'Hello', mode:'overwrite'}).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to write to"},
                    "content": {"type": "string", "description": "Content to write to the file"},
                    "mode": {"type": "string", "description": "Write mode: 'overwrite' (default) or 'append'", "default": "overwrite"}
                },
                "required": ["path", "content"]
            }),
        },

        // File editing tool
        FunctionDeclaration {
            name: tools::EDIT_FILE.to_string(),
            description: "fs_edit: Replace specific text in a file. Read the file first to identify exact spans. Example: edit_file({path:'src/lib.rs', old_str:'foo', new_str:'bar'}).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to edit"},
                    "old_str": {"type": "string", "description": "Exact text to replace (must match exactly)"},
                    "new_str": {"type": "string", "description": "New text to replace with"}
                },
                "required": ["path", "old_str", "new_str"]
            }),
        },

        // Consolidated command execution tool
        FunctionDeclaration {
            name: tools::RUN_TERMINAL_CMD.to_string(),
            description: "shell_run: Execute a program with args. Modes: 'terminal' (default), 'pty', 'streaming'. Prefer file tools for edits. Example: run_terminal_cmd({command:['bash','-lc','echo test'], timeout_secs:10, mode:'terminal'}).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "array", "items": {"type": "string"}, "description": "Program + args as array"},
                    "working_dir": {"type": "string", "description": "Working directory relative to workspace"},
                    "timeout_secs": {"type": "integer", "description": "Command timeout in seconds (default: 30)", "default": 30},
                    "mode": {"type": "string", "description": "Execution mode: 'terminal' | 'pty' | 'streaming'", "default": "terminal"},
                    "response_format": {"type": "string", "description": "'concise' (default) or 'detailed'", "default": "concise"}
                },
                "required": ["command"]
            }),
        },

        // AST-grep search and transformation tool
        FunctionDeclaration {
            name: tools::AST_GREP_SEARCH.to_string(),
            description: "code_astgrep: Syntax-aware code search and rewrite using AST-grep. Operations include 'search', 'transform', 'lint', 'refactor', and 'custom'. Provide patterns and optional context_lines/max_results to control output size.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "description": "Operation type: 'search', 'transform', 'lint', 'refactor', 'custom'", "default": "search"},
                    "pattern": {"type": "string", "description": "AST-grep pattern to search for"},
                    "path": {"type": "string", "description": "File or directory path to search in", "default": "."},
                    "language": {"type": "string", "description": "Programming language (auto-detected if not specified)"},
                    "replacement": {"type": "string", "description": "Replacement pattern for transform operations"},
                    "refactor_type": {"type": "string", "description": "Type of refactoring: 'extract_function', 'remove_console_logs', 'simplify_conditions', 'extract_constants', 'modernize_syntax'"},
                    "context_lines": {"type": "integer", "description": "Number of context lines to show", "default": 0},
                    "max_results": {"type": "integer", "description": "Maximum number of results", "default": 100},
                    "preview_only": {"type": "boolean", "description": "Preview changes without applying (transform only)", "default": true},
                    "update_all": {"type": "boolean", "description": "Update all matches (transform only)", "default": false},
                    "interactive": {"type": "boolean", "description": "Interactive mode (custom only)", "default": false},
                    "severity_filter": {"type": "string", "description": "Filter lint results by severity"}
                },
                "required": ["pattern", "path"]
            }),
        },

        // Simple bash-like search tool
        FunctionDeclaration {
            name: tools::SIMPLE_SEARCH.to_string(),
            description: "Simple bash-like search and file operations: grep, find, ls, cat, head, tail, index. Direct file operations without complex abstractions.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Command to execute: 'grep', 'find', 'ls', 'cat', 'head', 'tail', 'index'", "default": "grep"},
                    "pattern": {"type": "string", "description": "Search pattern for grep/find commands"},
                    "file_pattern": {"type": "string", "description": "File pattern filter for grep"},
                    "file_path": {"type": "string", "description": "File path for cat/head/tail commands"},
                    "path": {"type": "string", "description": "Directory path for ls/find/index commands", "default": "."},
                    "start_line": {"type": "integer", "description": "Start line number for cat command"},
                    "end_line": {"type": "integer", "description": "End line number for cat command"},
                    "lines": {"type": "integer", "description": "Number of lines for head/tail commands", "default": 10},
                    "max_results": {"type": "integer", "description": "Maximum results to return", "default": 50},
                    "show_hidden": {"type": "boolean", "description": "Show hidden files for ls command", "default": false}
                },
                "required": []
            }),
        },

        // Bash-like command tool
        FunctionDeclaration {
            name: tools::BASH.to_string(),
            description: "shell_bash: Bash-like commands via PTY (ls, pwd, grep, find, cat, head, tail, mkdir, rm, cp, mv, stat, run). Restricted for safety; prefer 'run_terminal_cmd' and file/search tools when possible.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "bash_command": {"type": "string", "description": "Bash command to execute: 'ls', 'pwd', 'grep', 'find', 'cat', 'head', 'tail', 'mkdir', 'rm', 'cp', 'mv', 'stat', 'run'", "default": "ls"},
                    "path": {"type": "string", "description": "Path for file/directory operations"},
                    "source": {"type": "string", "description": "Source path for cp/mv operations"},
                    "dest": {"type": "string", "description": "Destination path for cp/mv operations"},
                    "pattern": {"type": "string", "description": "Search pattern for grep/find"},
                    "recursive": {"type": "boolean", "description": "Recursive operation", "default": false},
                    "show_hidden": {"type": "boolean", "description": "Show hidden files", "default": false},
                    "parents": {"type": "boolean", "description": "Create parent directories", "default": false},
                    "force": {"type": "boolean", "description": "Force operation", "default": false},
                    "lines": {"type": "integer", "description": "Number of lines for head/tail", "default": 10},
                    "start_line": {"type": "integer", "description": "Start line for cat"},
                    "end_line": {"type": "integer", "description": "End line for cat"},
                    "name_pattern": {"type": "string", "description": "Name pattern for find"},
                    "type_filter": {"type": "string", "description": "Type filter for find (f=file, d=directory)"},
                    "command": {"type": "string", "description": "Command to run for arbitrary execution"},
                    "args": {"type": "array", "items": {"type": "string"}, "description": "Arguments for command execution"}
                },
                "required": []
            }),
        },

        // Apply patch tool (Codex patch format)
        FunctionDeclaration {
            name: "apply_patch".to_string(),
            description: "fs_patch: Apply a Codex-style patch block (*** Begin Patch ... *** End Patch) to the workspace. Use when the assistant provides a patch instead of calling write_file.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string", "description": "Patch content in Codex patch format"}
                },
                "required": ["input"]
            }),
        },
    ]
}

/// Build function declarations filtered by capability level
pub fn build_function_declarations_for_level(level: CapabilityLevel) -> Vec<FunctionDeclaration> {
    let all_declarations = build_function_declarations();

    match level {
        CapabilityLevel::Basic => vec![],
        CapabilityLevel::FileReading => all_declarations
            .into_iter()
            .filter(|fd| fd.name == "read_file")
            .collect(),
        CapabilityLevel::FileListing => all_declarations
            .into_iter()
            .filter(|fd| fd.name == tools::LIST_FILES || fd.name == tools::READ_FILE)
            .collect(),
        CapabilityLevel::Bash => all_declarations
            .into_iter()
            .filter(|fd| {
                fd.name == tools::LIST_FILES
                    || fd.name == tools::RUN_TERMINAL_CMD
                    || fd.name == tools::READ_FILE
            })
            .collect(),
        CapabilityLevel::Editing => all_declarations
            .into_iter()
            .filter(|fd| {
                fd.name == tools::LIST_FILES
                    || fd.name == tools::READ_FILE
                    || fd.name == tools::WRITE_FILE
                    || fd.name == tools::EDIT_FILE
                    || fd.name == tools::RUN_TERMINAL_CMD
            })
            .collect(),
        CapabilityLevel::CodeSearch => all_declarations
            .into_iter()
            .filter(|fd| {
                fd.name == tools::LIST_FILES
                    || fd.name == tools::RUN_TERMINAL_CMD
                    || fd.name == tools::GREP_SEARCH
                    || fd.name == tools::READ_FILE
                    || fd.name == tools::WRITE_FILE
                    || fd.name == tools::EDIT_FILE
                    || fd.name == tools::AST_GREP_SEARCH
                    || fd.name == tools::SIMPLE_SEARCH
                    || fd.name == tools::BASH
            })
            .collect(),
    }
}
