//! Tool registry and function declarations

use super::cache::FILE_CACHE;
use super::command::CommandTool;
use super::file_ops::FileOpsTool;
use super::search::SearchTool;
use super::simple_search::SimpleSearchTool;
use super::bash_tool::BashTool;
use super::traits::Tool;
use super::ast_grep_tool::AstGrepTool;
use crate::config::types::CapabilityLevel;
use crate::config::constants::tools;
use crate::gemini::FunctionDeclaration;
use crate::tool_policy::{ToolPolicy, ToolPolicyManager};
use crate::tools::ast_grep::AstGrepEngine;
use crate::tools::rp_search::RpSearchManager;
use anyhow::{Result, anyhow, Context};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// Main tool registry that coordinates all tools
#[derive(Clone)]
pub struct ToolRegistry {
    workspace_root: PathBuf,
    search_tool: SearchTool,
    simple_search_tool: SimpleSearchTool,
    bash_tool: BashTool,
    file_ops_tool: FileOpsTool,
    command_tool: CommandTool,
    rp_search: Arc<RpSearchManager>,
    ast_grep_engine: Option<Arc<AstGrepEngine>>,
    tool_policy: ToolPolicyManager,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new(workspace_root: PathBuf) -> Self {
        let rp_search = Arc::new(RpSearchManager::new(workspace_root.clone()));

        let search_tool = SearchTool::new(workspace_root.clone(), rp_search.clone());
        let simple_search_tool = SimpleSearchTool::new(workspace_root.clone());
        let bash_tool = BashTool::new(workspace_root.clone());
        let file_ops_tool = FileOpsTool::new(workspace_root.clone(), rp_search.clone());
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
        let mut policy_manager = ToolPolicyManager::new_with_workspace(&workspace_root).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize tool policy manager: {}", e);
            // Create a fallback that allows all tools
            ToolPolicyManager::new().unwrap()
        });

        // Update available tools in policy manager
        let mut available_tools = vec![
            tools::RP_SEARCH.to_string(),
            tools::LIST_FILES.to_string(),
            tools::RUN_TERMINAL_CMD.to_string(),
            tools::READ_FILE.to_string(),
            tools::WRITE_FILE.to_string(),
            tools::EDIT_FILE.to_string(),
            tools::SIMPLE_SEARCH.to_string(),
            tools::BASH.to_string(),
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
            rp_search,
            ast_grep_engine,
            tool_policy: policy_manager,
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
        if !self.policy_manager().should_execute_tool(name)? {
            return Err(anyhow!("Tool '{}' execution denied by policy", name));
        }

        match name {
            tools::RP_SEARCH => self.search_tool.execute(args).await,
            tools::LIST_FILES => self.file_ops_tool.execute(args).await,
            tools::RUN_TERMINAL_CMD => self.command_tool.execute(args).await,
            tools::READ_FILE => self.file_ops_tool.read_file(args).await,
            tools::WRITE_FILE => self.file_ops_tool.write_file(args).await,
            tools::EDIT_FILE => self.edit_file(args).await,
            tools::AST_GREP_SEARCH => self.execute_ast_grep(args).await,
            tools::SIMPLE_SEARCH => self.simple_search_tool.execute(args).await,
            tools::BASH => self.bash_tool.execute(args).await,
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }

    /// List available tools
    pub fn available_tools(&self) -> Vec<String> {
        let mut tools = vec![
            tools::RP_SEARCH.to_string(),
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
            tools::RP_SEARCH | tools::LIST_FILES | tools::RUN_TERMINAL_CMD | tools::READ_FILE | tools::WRITE_FILE => true,
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
        let current_content = read_result["content"].as_str()
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

                        new_content = format!(
                            "{}\n{}\n{}",
                            before,
                            replacement_lines.join("\n"),
                            after
                        );
                        replacement_occurred = true;
                        break;
                    }
                }
            }
        }

        // If no replacement occurred, provide detailed error
        if !replacement_occurred {
            let content_preview = if current_content.len() > 500 {
                format!("{}...{}", &current_content[..250], &current_content[current_content.len().saturating_sub(250)..])
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

        for (content_line, expected_line) in content_lines.iter().zip(expected_lines.iter()) {
            // Trim both lines and compare
            if content_line.trim() != expected_line.trim() {
                return false;
            }
        }

        true
    }

    pub async fn delete_file(&mut self, _args: Value) -> Result<Value> {
        Err(anyhow!("delete_file not yet implemented in modular system"))
    }

    pub async fn rp_search(&mut self, args: Value) -> Result<Value> {
        self.execute_tool(tools::RP_SEARCH, args).await
    }

    pub async fn list_files(&mut self, args: Value) -> Result<Value> {
        self.execute_tool(tools::LIST_FILES, args).await
    }

    pub async fn run_terminal_cmd(&mut self, args: Value) -> Result<Value> {
        self.execute_tool(tools::RUN_TERMINAL_CMD, args).await
    }

    /// Execute AST-grep tool
    async fn execute_ast_grep(&self, args: Value) -> Result<Value> {
        let engine = self.ast_grep_engine.as_ref()
            .ok_or_else(|| anyhow!("AST-grep engine not available"))?;

        let operation = args.get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("search");

        match operation {
            "search" => {
                let pattern = args.get("pattern")
                    .and_then(|v| v.as_str())
                    .context("'pattern' is required")?;

                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let context_lines = args.get("context_lines").and_then(|v| v.as_u64()).map(|v| v as usize);
                let max_results = args.get("max_results").and_then(|v| v.as_u64()).map(|v| v as usize);

                engine.search(pattern, &path, language, context_lines, max_results).await
            }
            "transform" => {
                let pattern = args.get("pattern")
                    .and_then(|v| v.as_str())
                    .context("'pattern' is required")?;

                let replacement = args.get("replacement")
                    .and_then(|v| v.as_str())
                    .context("'replacement' is required")?;

                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let preview_only = args.get("preview_only").and_then(|v| v.as_bool()).unwrap_or(true);
                let update_all = args.get("update_all").and_then(|v| v.as_bool()).unwrap_or(false);

                engine.transform(pattern, replacement, &path, language, preview_only, update_all).await
            }
            "lint" => {
                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let severity_filter = args.get("severity_filter").and_then(|v| v.as_str());

                engine.lint(&path, language, severity_filter, None).await
            }
            "refactor" => {
                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let refactor_type = args.get("refactor_type")
                    .and_then(|v| v.as_str())
                    .context("'refactor_type' is required")?;

                engine.refactor(&path, language, refactor_type).await
            }
            "custom" => {
                let pattern = args.get("pattern")
                    .and_then(|v| v.as_str())
                    .context("'pattern' is required")?;

                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .context("'path' is required")?;

                let path = self.normalize_path(path)?;

                let language = args.get("language").and_then(|v| v.as_str());
                let rewrite = args.get("rewrite").and_then(|v| v.as_str());
                let context_lines = args.get("context_lines").and_then(|v| v.as_u64()).map(|v| v as usize);
                let max_results = args.get("max_results").and_then(|v| v.as_u64()).map(|v| v as usize);
                let interactive = args.get("interactive").and_then(|v| v.as_bool()).unwrap_or(false);
                let update_all = args.get("update_all").and_then(|v| v.as_bool()).unwrap_or(false);

                engine.run_custom(
                    pattern,
                    &path,
                    language,
                    rewrite,
                    context_lines,
                    max_results,
                    interactive,
                    update_all,
                ).await
            }
            _ => Err(anyhow!("Unknown AST-grep operation: {}", operation)),
        }
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
}

/// Build function declarations for all available tools
pub fn build_function_declarations() -> Vec<FunctionDeclaration> {
    vec![
        // Ripgrep search tool
        FunctionDeclaration {
            name: tools::RP_SEARCH.to_string(),
            description: "Enhanced unified search tool with multiple modes: exact (default), fuzzy, multi-pattern, and similarity search. Consolidates all search functionality into one powerful tool.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Search pattern (required for exact/fuzzy modes)"},
                    "path": {"type": "string", "description": "Directory path to search in", "default": "."},
                    "mode": {"type": "string", "description": "Search mode: 'exact' (default), 'fuzzy', 'multi', 'similarity'", "default": "exact"},
                    "max_results": {"type": "integer", "description": "Maximum number of results", "default": 100},
                    "case_sensitive": {"type": "boolean", "description": "Case sensitive search", "default": true},
                    // Multi-pattern search parameters
                    "patterns": {"type": "array", "items": {"type": "string"}, "description": "Multiple patterns for multi mode"},
                    "logic": {"type": "string", "description": "Logic for multi mode: 'AND' or 'OR'", "default": "AND"},
                    // Fuzzy search parameters
                    "fuzzy_threshold": {"type": "number", "description": "Fuzzy matching threshold (0.0-1.0)", "default": 0.7},
                    // Similarity search parameters
                    "reference_file": {"type": "string", "description": "Reference file for similarity mode"},
                    "content_type": {"type": "string", "description": "Content type for similarity: 'structure', 'imports', 'functions', 'all'", "default": "all"}
                },
                "required": ["pattern"]
            }),
        },

        // Consolidated file operations tool
        FunctionDeclaration {
            name: "list_files".to_string(),
            description: "Enhanced file discovery tool with multiple modes: list (default), recursive, find_name, find_content. Consolidates all file discovery functionality.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to search from"},
                    "max_items": {"type": "integer", "description": "Maximum number of items to return", "default": 1000},
                    "include_hidden": {"type": "boolean", "description": "Include hidden files", "default": false},
                    "mode": {"type": "string", "description": "Discovery mode: 'list' (default), 'recursive', 'find_name', 'find_content'", "default": "list"},
                    "name_pattern": {"type": "string", "description": "Pattern for recursive and find_name modes"},
                    "content_pattern": {"type": "string", "description": "Content pattern for find_content mode"},
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
            description: "Read the contents of a file. Use this before editing to understand file structure.".to_string(),
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
            description: "Write content to a file. Can create new files or overwrite existing ones. Use read_file first to understand current content before editing.".to_string(),
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
            description: "Edit a file by replacing specific text. Use read_file first to understand the file structure and find exact text to replace.".to_string(),
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
            description: "Enhanced command execution tool with multiple modes: terminal (default), pty, streaming. Use this for shell commands, not for file editing - use read_file/write_file/edit_file for file operations.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "array", "items": {"type": "string"}, "description": "Program + args as array"},
                    "working_dir": {"type": "string", "description": "Working directory relative to workspace"},
                    "timeout_secs": {"type": "integer", "description": "Command timeout in seconds (default: 30)", "default": 30},
                    "mode": {"type": "string", "description": "Execution mode: 'terminal' (default), 'pty', 'streaming'", "default": "terminal"}
                },
                "required": ["command"]
            }),
        },

        // AST-grep search and transformation tool
        FunctionDeclaration {
            name: tools::AST_GREP_SEARCH.to_string(),
            description: "Advanced syntax-aware code search, transformation, and analysis using AST-grep patterns. Supports multiple operations: search (default), transform, lint, refactor, custom.".to_string(),
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
            description: "Direct bash-like command execution: ls, pwd, grep, find, cat, head, tail, mkdir, rm, cp, mv, stat, run. Acts like a human using bash commands.".to_string(),
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
            .filter(|fd| fd.name == tools::LIST_FILES || fd.name == tools::RUN_TERMINAL_CMD || fd.name == tools::READ_FILE)
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
                || fd.name == tools::RP_SEARCH
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
