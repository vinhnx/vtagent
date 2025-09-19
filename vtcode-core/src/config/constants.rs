/// Prompt path constants to avoid hardcoding throughout the codebase
pub mod prompts {
    pub const DEFAULT_SYSTEM_PROMPT_PATH: &str = "prompts/system.md";
    pub const CODER_SYSTEM_PROMPT_PATH: &str = "prompts/coder_system.md";
}

/// Model ID constants to sync with docs/models.json
pub mod models {
    // Google/Gemini models
    pub mod google {
        pub const DEFAULT_MODEL: &str = "gemini-2.5-flash-preview-05-20";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "gemini-2.5-flash-preview-05-20",
            "gemini-2.5-pro",
        ];

        // Convenience constants for commonly used models
        pub const GEMINI_2_5_FLASH_PREVIEW: &str = "gemini-2.5-flash-preview-05-20";
        pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";
    }

    // OpenAI models (from docs/models.json)
    pub mod openai {
        pub const DEFAULT_MODEL: &str = "gpt-5";
        pub const SUPPORTED_MODELS: &[&str] = &["gpt-5", "gpt-5-mini", "gpt-5-nano", "codex-mini-latest"];

        // Convenience constants for commonly used models
        pub const GPT_5: &str = "gpt-5";
        pub const GPT_5_MINI: &str = "gpt-5-mini";
        pub const GPT_5_NANO: &str = "gpt-5-nano";
        pub const CODEX_MINI_LATEST: &str = "codex-mini-latest";
        pub const CODEX_MINI: &str = "codex-mini";
    }

    // Anthropic models (from docs/models.json) - Updated for tool use best practices
    pub mod anthropic {
        // Standard model for straightforward tools - Sonnet 4 preferred for most use cases
        pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "claude-opus-4-1-20250805", // Latest: Opus 4.1 (2025-08-05)
            "claude-sonnet-4-20250514", // Latest: Sonnet 4 (2025-05-14)
        ];

        // Convenience constants for commonly used models
        pub const CLAUDE_OPUS_4_1_20250805: &str = "claude-opus-4-1-20250805";
        pub const CLAUDE_SONNET_4_20250514: &str = "claude-sonnet-4-20250514";
    }

    // Backwards compatibility - keep old constants working
    pub const GEMINI_2_5_FLASH_PREVIEW: &str = google::GEMINI_2_5_FLASH_PREVIEW;
    pub const GEMINI_2_5_PRO: &str = google::GEMINI_2_5_PRO;
    pub const GPT_5: &str = openai::GPT_5;
    pub const GPT_5_MINI: &str = openai::GPT_5_MINI;
    pub const GPT_5_NANO: &str = openai::GPT_5_NANO;
    pub const CODEX_MINI: &str = openai::CODEX_MINI;
    pub const CODEX_MINI_LATEST: &str = openai::CODEX_MINI_LATEST;
    pub const CLAUDE_OPUS_4_1_20250805: &str = anthropic::CLAUDE_OPUS_4_1_20250805;
    pub const CLAUDE_SONNET_4_20250514: &str = anthropic::CLAUDE_SONNET_4_20250514;
}

/// Model validation and helper functions
pub mod model_helpers {
    use super::models;

    /// Get supported models for a provider
    pub fn supported_for(provider: &str) -> Option<&'static [&'static str]> {
        match provider {
            "google" | "gemini" => Some(models::google::SUPPORTED_MODELS),
            "openai" => Some(models::openai::SUPPORTED_MODELS),
            "anthropic" => Some(models::anthropic::SUPPORTED_MODELS),
            _ => None,
        }
    }

    /// Get default model for a provider
    pub fn default_for(provider: &str) -> Option<&'static str> {
        match provider {
            "google" | "gemini" => Some(models::google::DEFAULT_MODEL),
            "openai" => Some(models::openai::DEFAULT_MODEL),
            "anthropic" => Some(models::anthropic::DEFAULT_MODEL),
            _ => None,
        }
    }

    /// Validate if a model is supported by a provider
    pub fn is_valid(provider: &str, model: &str) -> bool {
        supported_for(provider)
            .map(|list| list.iter().any(|m| *m == model))
            .unwrap_or(false)
    }
}

/// Default configuration values
pub mod defaults {
    use super::models;

    pub const DEFAULT_MODEL: &str = models::google::GEMINI_2_5_FLASH_PREVIEW;
    pub const DEFAULT_CLI_MODEL: &str = models::google::GEMINI_2_5_FLASH_PREVIEW;
    pub const DEFAULT_PROVIDER: &str = "gemini";
    pub const DEFAULT_API_KEY_ENV: &str = "GEMINI_API_KEY";
    pub const DEFAULT_THEME: &str = "ciapre-dark";
    pub const DEFAULT_MAX_TOOL_LOOPS: usize = 100;
}

/// Message role constants to avoid hardcoding strings
pub mod message_roles {
    pub const SYSTEM: &str = "system";
    pub const USER: &str = "user";
    pub const ASSISTANT: &str = "assistant";
    pub const TOOL: &str = "tool";
}

/// URL constants for API endpoints
pub mod urls {}

/// Tool name constants to avoid hardcoding strings throughout the codebase
pub mod tools {
    pub const GREP_SEARCH: &str = "grep_search";
    pub const LIST_FILES: &str = "list_files";
    pub const RUN_TERMINAL_CMD: &str = "run_terminal_cmd";
    pub const READ_FILE: &str = "read_file";
    pub const WRITE_FILE: &str = "write_file";
    pub const EDIT_FILE: &str = "edit_file";
    pub const DELETE_FILE: &str = "delete_file";
    pub const CREATE_FILE: &str = "create_file";
    pub const AST_GREP_SEARCH: &str = "ast_grep_search";
    pub const SIMPLE_SEARCH: &str = "simple_search";
    pub const BASH: &str = "bash";
    pub const APPLY_PATCH: &str = "apply_patch";
    pub const SRGN: &str = "srgn";

    // Explorer-specific tools
    pub const FILE_METADATA: &str = "file_metadata";
    pub const PROJECT_OVERVIEW: &str = "project_overview";
    pub const TREE_SITTER_ANALYZE: &str = "tree_sitter_analyze";

    // Special wildcard for full access
    pub const WILDCARD_ALL: &str = "*";
}

/// Context window management defaults
pub mod context {
    /// Approximate character count per token when estimating context size
    pub const CHAR_PER_TOKEN_APPROX: usize = 3;

    /// Default maximum context window (in approximate tokens)
    pub const DEFAULT_MAX_TOKENS: usize = 90_000;

    /// Trim target as a percentage of the maximum token budget
    pub const DEFAULT_TRIM_TO_PERCENT: u8 = 80;

    /// Minimum allowed trim percentage (prevents overly aggressive retention)
    pub const MIN_TRIM_RATIO_PERCENT: u8 = 60;

    /// Maximum allowed trim percentage (prevents minimal trimming)
    pub const MAX_TRIM_RATIO_PERCENT: u8 = 90;

    /// Default number of recent turns to preserve verbatim
    pub const DEFAULT_PRESERVE_RECENT_TURNS: usize = 12;

    /// Minimum number of recent turns that must remain after trimming
    pub const MIN_PRESERVE_RECENT_TURNS: usize = 6;

    /// Maximum number of recent turns to keep when aggressively reducing context
    pub const AGGRESSIVE_PRESERVE_RECENT_TURNS: usize = 8;

    /// Maximum number of retry attempts when the provider signals context overflow
    pub const CONTEXT_ERROR_RETRY_LIMIT: usize = 2;
}

/// Chunking constants for large file handling
pub mod chunking {
    /// Maximum lines before triggering chunking for read_file
    pub const MAX_LINES_THRESHOLD: usize = 2_000;

    /// Number of lines to read from start of file when chunking
    pub const CHUNK_START_LINES: usize = 800;

    /// Number of lines to read from end of file when chunking
    pub const CHUNK_END_LINES: usize = 800;

    /// Maximum lines for terminal command output before truncation
    pub const MAX_TERMINAL_OUTPUT_LINES: usize = 3_000;

    /// Number of lines to show from start of terminal output when truncating
    pub const TERMINAL_OUTPUT_START_LINES: usize = 1_000;

    /// Number of lines to show from end of terminal output when truncating
    pub const TERMINAL_OUTPUT_END_LINES: usize = 1_000;

    /// Maximum content size for write_file before chunking (in bytes)
    pub const MAX_WRITE_CONTENT_SIZE: usize = 500_000; // 500KB

    /// Chunk size for write operations (in bytes)
    pub const WRITE_CHUNK_SIZE: usize = 50_000; // 50KB chunks
}
