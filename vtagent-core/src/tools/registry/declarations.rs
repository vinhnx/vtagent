use std::collections::HashMap;

use crate::config::constants::tools;
use crate::config::types::CapabilityLevel;
use crate::gemini::FunctionDeclaration;
use serde_json::json;

use super::builtins::builtin_tool_registrations;

pub fn build_function_declarations() -> Vec<FunctionDeclaration> {
    vec![
        // Ripgrep search tool
        FunctionDeclaration {
            name: tools::GREP_SEARCH.to_string(),
            description: "Performs advanced code search across the workspace using ripgrep, supporting multiple search modes and patterns. This tool is ideal for finding specific code patterns, function definitions, variable usages, or text matches across multiple files. It should be used when you need to locate code elements, search for TODO comments, find function calls, or identify patterns in the codebase. The tool supports exact matching, fuzzy search, multi-pattern searches with AND/OR logic, and similarity-based searches. Results can be returned in concise format (recommended for most cases) or detailed raw ripgrep JSON format. Always specify a reasonable max_results limit to prevent token overflow.".to_string(),
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
            description: "Explores and lists files and directories in the workspace with multiple discovery modes. This tool is essential for understanding project structure, finding files by name or content, and navigating the codebase. Use this tool when you need to see what files exist in a directory, find files matching specific patterns, or search for files containing certain content. It supports recursive directory traversal, pagination for large directories, and various filtering options. The tool can operate in different modes: 'list' for basic directory contents, 'recursive' for deep directory traversal, 'find_name' for filename-based searches, and 'find_content' for content-based file discovery. PAGINATION BEST PRACTICES: Always use pagination (page and per_page parameters) for large directories to prevent token overflow and timeouts. Default per_page=50 for optimal performance. Monitor the 'has_more' flag and continue with subsequent pages. For very large directories (>1000 items), consider reducing per_page to 25. The concise response format is recommended for most cases as it omits low-value metadata.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to search from (relative). Example: 'src'"},
                    "mode": {"type": "string", "description": "'list' | 'recursive' | 'find_name' | 'find_content'", "default": "list"},
                    "max_items": {"type": "integer", "description": "Cap total items scanned (token safety). Default: 1000", "default": 1000},
                    "page": {"type": "integer", "description": "Page number (1-based). Default: 1", "default": 1},
                    "per_page": {"type": "integer", "description": "Items per page. Default: 50", "default": 50},
                    "response_format": {"type": "string", "description": "'concise' (default) omits low-signal fields; 'detailed' includes them", "default": "concise"},
                    "include_hidden": {"type": "boolean", "description": "Include hidden files", "default": false},
                    "name_pattern": {"type": "string", "description": "Optional pattern for 'recursive'/'find_name' modes. Use '*' or omit for all files. Example: '*.rs'", "default": "*"},
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
            description: "Reads the contents of a specific file from the workspace with intelligent chunking for large files. This tool automatically handles large files by reading the first and last portions when files exceed size thresholds, ensuring efficient token usage while preserving important content. For files larger than 2,000 lines, it reads the first 800 and last 800 lines with a truncation indicator. Use chunk_lines or max_lines parameters to customize the threshold. The tool provides structured logging of chunking operations for debugging.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"},
                    "max_bytes": {"type": "integer", "description": "Maximum bytes to read (optional)", "default": null},
                    "chunk_lines": {"type": "integer", "description": "Line threshold for chunking (optional, default: 2000)", "default": 2000},
                    "max_lines": {"type": "integer", "description": "Alternative parameter for chunk_lines (optional)", "default": null}
                },
                "required": ["path"]
            }),
        },

        // File writing tool
        FunctionDeclaration {
            name: tools::WRITE_FILE.to_string(),
            description: "Creates new files or overwrites existing files with specified content. This tool is essential for creating new source files, configuration files, documentation, or any text-based content. Use this tool when you need to create a new file from scratch, replace an entire file's contents, or append content to an existing file. The tool supports different write modes: 'overwrite' (default) completely replaces the file content, 'append' adds content to the end of the file, and 'skip_if_exists' prevents overwriting existing files. Always ensure you have the correct file path and that the directory exists before writing. This tool cannot create directories automatically - use the terminal command tool for directory creation if needed. The tool validates that the content is properly written and returns success/failure status.".to_string(),
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
            description: "Performs precise text replacements within existing files by finding and replacing exact text matches. This tool is crucial for making targeted code changes, fixing bugs, updating configurations, or modifying documentation. Use this tool when you need to change specific text in a file without affecting the rest of the content. Always read the file first using the read_file tool to identify the exact text to replace, including proper indentation and surrounding context. The old_str parameter must match the existing text exactly, including whitespace and formatting. This tool is preferred over write_file when you only need to modify part of a file, as it preserves the rest of the file's content. Note that this tool performs exact string matching - it cannot handle complex refactoring or pattern-based replacements.".to_string(),
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
            description: "Executes shell commands and external programs in the workspace environment with intelligent output truncation for large command outputs. This tool automatically handles verbose command outputs by truncating to the first and last portions when output exceeds 10,000 lines, ensuring efficient token usage while preserving important information. For commands producing excessive output, it shows the first 5,000 and last 5,000 lines with a truncation indicator. Use this tool for build processes, package managers, test suites, and system operations. Supports 'terminal' (default), 'pty' (interactive), and 'streaming' (long-running) modes. Always specify timeouts and prefer specialized tools for file operations.".to_string(),
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
            description: "Performs advanced syntax-aware code analysis and transformation using AST-grep patterns. This tool excels at structural code searches, automated refactoring, and complex code transformations that require understanding of programming language syntax. Use this tool for finding function definitions, class structures, import statements, or complex code patterns that cannot be easily found with simple text search. It supports multiple operations: 'search' for finding code patterns, 'transform' for automated code changes, 'lint' for code quality checks, 'refactor' for structural improvements, and 'custom' for specialized operations. The tool can work with multiple programming languages and provides context-aware results. Always specify reasonable limits for context_lines and max_results to prevent token overflow. Preview mode is enabled by default for transform operations to allow safe review before applying changes.".to_string(),
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
            description: "Provides simple bash-like file operations and searches for quick, straightforward tasks. This tool offers direct access to common Unix commands like grep, find, ls, cat, head, tail, and file indexing. Use this tool when you need basic file operations without the complexity of advanced search features. It is ideal for quick file content previews, directory listings, or simple pattern matching. The tool supports various commands: 'grep' for text searching, 'find' for file discovery, 'ls' for directory listing, 'cat' for full file reading, 'head'/'tail' for partial file reading, and 'index' for file indexing. This tool is less powerful than specialized search tools but provides fast, intuitive access to common operations. Use appropriate max_results limits to prevent excessive output, especially with recursive operations.".to_string(),
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
            description: "Executes bash-like commands through a pseudo-terminal interface for interactive operations. This tool provides access to common shell commands with enhanced terminal emulation. Use this tool when you need interactive command execution, complex shell pipelines, or commands that require a proper terminal environment. It supports essential commands like 'ls' for directory listing, 'pwd' for current directory, 'grep' for text search, 'find' for file discovery, 'cat'/'head'/'tail' for file content viewing, and file manipulation commands like 'mkdir', 'rm', 'cp', 'mv'. The tool includes safety restrictions and should be used as a complement to specialized tools rather than a replacement. Prefer 'run_terminal_cmd' for non-interactive commands and file/search tools for file operations. Always use appropriate flags and be aware of the recursive and force options which can affect multiple files.".to_string(),
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
            name: tools::APPLY_PATCH.to_string(),
            description: "Applies Codex-style patch blocks to modify multiple files in the workspace. This tool is specialized for applying structured patches that contain changes to multiple files or complex modifications. Use this tool when you receive patch content in the Codex format (marked with '*** Begin Patch' and '*** End Patch') instead of making individual file edits. The tool parses the patch format, validates the changes, and applies them atomically to prevent partial updates. It is particularly useful for applying code review suggestions, automated refactoring changes, or complex multi-file modifications. The tool provides detailed feedback on which files were modified and any issues encountered during application. Always ensure the patch content is complete and properly formatted before using this tool.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string", "description": "Patch content in Codex patch format"}
                },
                "required": ["input"]
            }),
        },

        // Speckit tool for spec-driven development
        FunctionDeclaration {
            name: tools::SPECKIT.to_string(),
            description: "Speckit tool for spec-driven development using GitHub's Spec Kit. This tool enables structured development workflows by transforming natural language specifications into executable code, tests, and documentation. Use this tool to initialize spec-driven projects, create detailed specifications, generate implementation plans, and break down work into actionable tasks. The tool supports commands like 'init' for project setup, '/specify' for creating specifications, '/plan' for technical planning, and '/tasks' for task breakdown. Speckit follows a methodology that inverts traditional development: specifications drive code generation rather than merely guiding it. This tool is particularly valuable for greenfield development, feature exploration, and maintaining specification-driven consistency across projects.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Speckit command to execute: 'init', 'check', '/specify', '/plan', '/tasks'",
                        "enum": ["init", "check", "/specify", "/plan", "/tasks"]
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Arguments for the Speckit command. For 'init': project name or use '--here' flag. For '/specify': specification description. For other commands: optional context arguments."
                    }
                },
                "required": ["command"]
            }),
        },
    ]
}

/// Build function declarations filtered by capability level
pub fn build_function_declarations_for_level(level: CapabilityLevel) -> Vec<FunctionDeclaration> {
    let tool_capabilities: HashMap<&'static str, CapabilityLevel> = builtin_tool_registrations()
        .into_iter()
        .filter(|registration| registration.expose_in_llm())
        .map(|registration| (registration.name(), registration.capability()))
        .collect();

    build_function_declarations()
        .into_iter()
        .filter(|fd| {
            tool_capabilities
                .get(fd.name.as_str())
                .map(|required| level >= *required)
                .unwrap_or(false)
        })
        .collect()
}
