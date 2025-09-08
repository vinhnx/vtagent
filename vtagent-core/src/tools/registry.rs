//! Tool registry and function declarations

use super::traits::Tool;
use super::search::SearchTool;
use super::file_ops::FileOpsTool;
use super::command::CommandTool;
use super::cache::FILE_CACHE;
use crate::ast_grep::AstGrepEngine;
use crate::gemini::FunctionDeclaration;
use crate::rp_search::RpSearchManager;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// Main tool registry that coordinates all tools
pub struct ToolRegistry {
    workspace_root: PathBuf,
    search_tool: SearchTool,
    file_ops_tool: FileOpsTool,
    command_tool: CommandTool,
    rp_search: Arc<RpSearchManager>,
    ast_grep_engine: Option<Arc<AstGrepEngine>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new(workspace_root: PathBuf) -> Self {
        let rp_search = Arc::new(RpSearchManager::new(workspace_root.clone()));
        
        let search_tool = SearchTool::new(workspace_root.clone(), rp_search.clone());
        let file_ops_tool = FileOpsTool::new(workspace_root.clone(), rp_search.clone());
        let command_tool = CommandTool::new(workspace_root.clone());

        Self {
            workspace_root,
            search_tool,
            file_ops_tool,
            command_tool,
            rp_search,
            ast_grep_engine: None,
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

    /// Execute a tool by name
    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        match name {
            "rp_search" => self.search_tool.execute(args).await,
            "list_files" => self.file_ops_tool.execute(args).await,
            "run_terminal_cmd" => self.command_tool.execute(args).await,
            "read_file" => self.file_ops_tool.read_file(args).await,
            "write_file" => self.file_ops_tool.write_file(args).await,
            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }

    /// List available tools
    pub fn available_tools(&self) -> Vec<String> {
        vec![
            "rp_search".to_string(),
            "list_files".to_string(),
            "run_terminal_cmd".to_string(),
            "read_file".to_string(),
            "write_file".to_string(),
        ]
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        matches!(name, "rp_search" | "list_files" | "run_terminal_cmd" | "read_file" | "write_file")
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
    pub async fn read_file(&self, args: Value) -> Result<Value> {
        self.file_ops_tool.read_file(args).await
    }

    pub async fn write_file(&self, args: Value) -> Result<Value> {
        self.file_ops_tool.write_file(args).await
    }

    pub async fn edit_file(&self, _args: Value) -> Result<Value> {
        Err(anyhow!("edit_file not yet implemented in modular system"))
    }

    pub async fn delete_file(&self, _args: Value) -> Result<Value> {
        Err(anyhow!("delete_file not yet implemented in modular system"))
    }

    pub async fn rp_search(&self, args: Value) -> Result<Value> {
        self.execute_tool("rp_search", args).await
    }

    pub async fn list_files(&self, args: Value) -> Result<Value> {
        self.execute_tool("list_files", args).await
    }

    pub async fn run_terminal_cmd(&self, args: Value) -> Result<Value> {
        self.execute_tool("run_terminal_cmd", args).await
    }
}

/// Build function declarations for all available tools
pub fn build_function_declarations() -> Vec<FunctionDeclaration> {
    vec![
        // Consolidated search tool
        FunctionDeclaration {
            name: "rp_search".to_string(),
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

        // Consolidated command execution tool
        FunctionDeclaration {
            name: "run_terminal_cmd".to_string(),
            description: "Enhanced command execution tool with multiple modes: terminal (default), pty, streaming. Consolidates all command execution functionality.".to_string(),
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
    ]
}

/// Build function declarations filtered by capability level
pub fn build_function_declarations_for_level(level: crate::types::CapabilityLevel) -> Vec<FunctionDeclaration> {
    let all_declarations = build_function_declarations();

    match level {
        crate::types::CapabilityLevel::Basic => vec![],
        crate::types::CapabilityLevel::FileReading => vec![],
        crate::types::CapabilityLevel::FileListing => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "list_files")
                .collect()
        },
        crate::types::CapabilityLevel::Bash => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "list_files" || fd.name == "run_terminal_cmd")
                .collect()
        },
        crate::types::CapabilityLevel::Editing => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "list_files" || fd.name == "run_terminal_cmd")
                .collect()
        },
        crate::types::CapabilityLevel::CodeSearch => {
            all_declarations.into_iter()
                .filter(|fd| fd.name == "list_files" || fd.name == "run_terminal_cmd" || fd.name == "rp_search")
                .collect()
        },
    }
}
