# VTAgent Tools Registry - Complete List

## File Operations Tools

### 1. read_file
- **Description**: Reads content from a file in the workspace
- **Dependencies**: tokio::fs
- **Parameters**: 
  - path (string): Path to the file to read
  - max_bytes (integer, optional): Maximum bytes to read
  - encoding (string, optional): Text encoding (default: utf-8)
  - ast_grep_pattern (string, optional): AST pattern to extract matches

### 2. list_files
- **Description**: Lists files and directories in a given path
- **Dependencies**: walkdir, dashmap
- **Parameters**:
  - path (string): Path to list files from
  - max_items (integer, optional): Maximum number of items to return (default: 1000)
  - include_hidden (boolean, optional): Include hidden files (default: false)
  - ast_grep_pattern (string, optional): AST pattern to filter files

### 3. write_file
- **Description**: Writes content to a file with various modes
- **Dependencies**: tokio::fs
- **Parameters**:
  - path (string): Path to the file to write
  - content (string): Content to write to the file
  - encoding (string, optional): Text encoding (default: utf-8)
  - mode (string, optional): Write mode (overwrite, append, skip_if_exists, patch) (default: overwrite)
  - ast_grep_lint (boolean, optional): Run AST-grep lint analysis after writing (default: false)
  - ast_grep_refactor (boolean, optional): Get refactoring suggestions after writing (default: false)

### 4. edit_file
- **Description**: Edits a file by replacing text
- **Dependencies**: tokio::fs
- **Parameters**:
  - path (string): Path to the file to edit
  - old_string (string): Text to replace
  - new_string (string): Replacement text
  - encoding (string, optional): Text encoding (default: utf-8)
  - ast_grep_lint (boolean, optional): Run AST-grep lint analysis after editing (default: false)
  - ast_grep_refactor (boolean, optional): Get refactoring suggestions after editing (default: false)

### 5. delete_file
- **Description**: Deletes a file in the workspace with confirmation requirement
- **Dependencies**: tokio::fs
- **Parameters**:
  - path (string): Path to the file to delete
  - confirm (boolean): Must be true to confirm deletion (default: false)
  - ast_grep_warn_pattern (string, optional): AST pattern to check for important code before deletion

## Search Tools

### 6. rp_search
- **Description**: Enhanced ripgrep search with debounce and cancellation support
- **Dependencies**: ripgrep (external), tokio::process
- **Parameters**:
  - pattern (string): Search pattern (regex unless 'literal' is true)
  - path (string): Base path to search from (default: ".")
  - case_sensitive (boolean, optional): Enable case-sensitive search (default: true)
  - literal (boolean, optional): Treat pattern as literal text (default: false)
  - glob_pattern (string, optional): Glob pattern to filter files (e.g., '**/*.rs')
  - context_lines (integer, optional): Number of context lines before/after each match (default: 0)
  - include_hidden (boolean, optional): Include hidden files (default: false)
  - max_results (integer, optional): Maximum number of matches to return (default: 1000)

### 7. code_search
- **Description**: Search code using ripgrep-like semantics
- **Dependencies**: ripgrep (external), tokio::process
- **Parameters**:
  - pattern (string): Search pattern (regex)
  - path (string): Base path (file or dir) (default: ".")
  - file_type (string, optional): Limit to extension, e.g. 'rs', 'go'

### 8. codebase_search
- **Description**: High-level search across common source files
- **Dependencies**: ripgrep (external), tokio::process
- **Parameters**:
  - pattern (string): Search pattern
  - path (string): Base path (default: ".")
  - case_sensitive (boolean, optional): (default: true)
  - literal (boolean, optional): (default: false)
  - context_lines (integer, optional): (default: 0)
  - include_hidden (boolean, optional): (default: false)
  - max_results (integer, optional): (default: 1000)

## Terminal/PTY Tools

### 9. run_terminal_cmd
- **Description**: Run a terminal command in the workspace with basic safety checks
- **Dependencies**: tokio::process
- **Parameters**:
  - command (array of strings): Program + args as array
  - working_dir (string, optional): Working directory relative to workspace

### 10. run_pty_cmd
- **Description**: Run a command in a pseudo-terminal (PTY) with full terminal emulation
- **Dependencies**: tokio::process, rexpect
- **Parameters**:
  - command (string): Command to execute in the PTY
  - args (array of strings, optional): Arguments for the command (default: [])
  - working_dir (string, optional): Working directory relative to workspace
  - rows (integer, optional): Terminal rows (default: 24)
  - cols (integer, optional): Terminal columns (default: 80)

### 11. run_pty_cmd_streaming
- **Description**: Run a command in a pseudo-terminal (PTY) with streaming output
- **Dependencies**: tokio::process, rexpect
- **Parameters**:
  - command (string): Command to execute in the PTY
  - args (array of strings, optional): Arguments for the command (default: [])
  - working_dir (string, optional): Working directory relative to workspace
  - rows (integer, optional): Terminal rows (default: 24)
  - cols (integer, optional): Terminal columns (default: 80)

### 12. create_pty_session
- **Description**: Create a new PTY session for running interactive terminal commands
- **Dependencies**: tokio::process, rexpect
- **Parameters**:
  - session_id (string): Unique identifier for the PTY session
  - command (string): Command to execute in the PTY
  - args (array of strings, optional): Arguments for the command (default: [])
  - working_dir (string, optional): Working directory relative to workspace
  - rows (integer, optional): Terminal rows (default: 24)
  - cols (integer, optional): Terminal columns (default: 80)

### 13. list_pty_sessions
- **Description**: List all active PTY sessions
- **Dependencies**: None
- **Parameters**: None

### 14. close_pty_session
- **Description**: Close a PTY session
- **Dependencies**: None
- **Parameters**:
  - session_id (string): Unique identifier for the PTY session to close

## AST-grep Tools

### 15. ast_grep_search
- **Description**: Advanced syntax-aware code search using AST patterns
- **Dependencies**: tree-sitter
- **Parameters**:
  - pattern (string): AST pattern to search for (e.g., 'console.log($msg)')
  - path (string): File or directory path to search in (default: ".")
  - language (string, optional): Programming language (auto-detected if not specified)
  - context_lines (integer, optional): Number of context lines to show around matches (default: 2)
  - max_results (integer, optional): Maximum number of results to return (default: 100)

### 16. ast_grep_transform
- **Description**: Transform code using AST-based pattern matching and replacement
- **Dependencies**: tree-sitter
- **Parameters**:
  - pattern (string): AST pattern to match (e.g., 'console.log($msg)')
  - replacement (string): Replacement pattern (e.g., '// console.log($msg)')
  - path (string): File or directory path to transform (default: ".")
  - language (string, optional): Programming language (auto-detected if not specified)
  - preview_only (boolean, optional): Show preview without applying changes (default: true)

### 17. ast_grep_lint
- **Description**: Lint code using AST-based rules to find potential issues and anti-patterns
- **Dependencies**: tree-sitter
- **Parameters**:
  - path (string): File or directory path to lint (default: ".")
  - language (string, optional): Programming language (auto-detected if not specified)
  - severity_filter (string, optional): Minimum severity to report (default: "warning")

### 18. ast_grep_refactor
- **Description**: Get intelligent refactoring suggestions using common code patterns and best practices
- **Dependencies**: tree-sitter
- **Parameters**:
  - path (string): File or directory path to analyze for refactoring opportunities (default: ".")
  - language (string, optional): Programming language (auto-detected if not specified)
  - refactor_type (string, optional): Type of refactoring to suggest (default: "all")

## Advanced Search Tools

### 19. fuzzy_search
- **Description**: Advanced fuzzy text search that finds approximate matches across files
- **Dependencies**: ripgrep (external)
- **Parameters**:
  - query (string): Search query to match approximately
  - path (string): Directory path to search in (default: ".")
  - max_results (integer, optional): Maximum number of results to return (default: 50)
  - threshold (number, optional): Similarity threshold (0.0 to 1.0) (default: 0.6)
  - case_sensitive (boolean, optional): Whether search should be case sensitive (default: false)

### 20. similarity_search
- **Description**: Find files with similar content structure, imports, functions, or patterns
- **Dependencies**: ripgrep (external)
- **Parameters**:
  - reference_file (string): Path to the reference file to find similar files to
  - search_path (string): Directory path to search in (default: ".")
  - max_results (integer, optional): Maximum number of results to return (default: 20)
  - content_type (string, optional): Type of similarity to search for (default: "all")

### 21. multi_pattern_search
- **Description**: Search using multiple patterns with boolean logic (AND, OR, NOT)
- **Dependencies**: ripgrep (external)
- **Parameters**:
  - patterns (array of strings): List of search patterns
  - logic (string, optional): Boolean logic to apply (default: "AND")
  - path (string): Directory path to search in (default: ".")
  - max_results (integer, optional): Maximum number of results to return (default: 100)
  - context_lines (integer, optional): Number of context lines around matches (default: 2)

### 22. extract_text_patterns
- **Description**: Extract and categorize specific text patterns like URLs, emails, TODOs, credentials, etc.
- **Dependencies**: regex
- **Parameters**:
  - path (string): Directory path to search in (default: ".")
  - pattern_types (array of strings): Types of patterns to extract (e.g., "urls", "emails", "todos")
  - max_results (integer, optional): Maximum number of results to return (default: 200)

## Summary

The VTAgent tools registry includes 22 distinct tools organized into five categories:
1. File Operations (5 tools)
2. Search Tools (3 tools)
3. Terminal/PTY Tools (6 tools)
4. AST-grep Tools (4 tools)
5. Advanced Search Tools (4 tools)

Each tool is designed to work asynchronously with proper error handling and security features. Many tools include optional AST-grep functionality for more intelligent code analysis and manipulation. Several tools depend on external binaries (especially ripgrep) that need to be available in the environment where VTAgent runs.