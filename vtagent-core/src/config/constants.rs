/// Prompt path constants to avoid hardcoding throughout the codebase
pub mod prompts {
    pub const DEFAULT_SYSTEM_PROMPT_PATH: &str = "prompts/system.md";
    pub const CODER_SYSTEM_PROMPT_PATH: &str = "prompts/coder_system.md";
}

/// Model ID constants to sync with docs/models.json
pub mod models {
    // Google/Gemini models
    pub mod google {
        pub const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite-preview-06-17";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "gemini-2.5-flash-lite-preview-06-17",
            "gemini-2.5-flash",
            "gemini-2.5-pro",
        ];

        // Convenience constants for commonly used models
        pub const GEMINI_2_5_FLASH_LITE: &str = "gemini-2.5-flash-lite-preview-06-17";
        pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";
        pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";
    }

    // OpenAI models (from docs/models.json)
    pub mod openai {
        pub const DEFAULT_MODEL: &str = "gpt-5";
        pub const SUPPORTED_MODELS: &[&str] = &["gpt-4.1", "gpt-5", "gpt-5-mini"];

        // Convenience constants for commonly used models
        pub const GPT_5: &str = "gpt-5";
        pub const GPT_5_MINI: &str = "gpt-5-mini";
        pub const GPT_4_1: &str = "gpt-4.1";
    }

    // Anthropic models (from docs/models.json) - Updated for tool use best practices
    pub mod anthropic {
        // Standard model for straightforward tools - Sonnet 4 preferred for most use cases
        pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "claude-sonnet-4-20250514", // Standard: Sonnet 4 for straightforward tools
            "claude-opus-4-1-20250805", // Complex: Opus 4.1 for complex workflows (configurable)
            "claude-opus-4-20250514",   // Complex: Alternative Opus model
            "claude-sonnet-3.7-20250219", // Advanced: Sonnet 3.7 with extended thinking
            "claude-haiku-3.5-20241022", // Minimal: Haiku 3.5 for basic tools
            "claude-sonnet-3.5-20241022", // Legacy: Sonnet 3.5 (deprecated but still available)
            "claude-haiku-3-20240307",  // Basic: Haiku 3 for simple tasks
        ];

        // Convenience constants for commonly used models
        pub const CLAUDE_OPUS_4_1_20250805: &str = "claude-opus-4-1-20250805";
        pub const CLAUDE_OPUS_4_20250514: &str = "claude-opus-4-20250514";
        pub const CLAUDE_SONNET_4_20250514: &str = "claude-sonnet-4-20250514";
        pub const CLAUDE_SONNET_3_7_20250219: &str = "claude-sonnet-3.7-20250219";
        pub const CLAUDE_SONNET_3_5_20241022: &str = "claude-sonnet-3.5-20241022";
        pub const CLAUDE_HAIKU_3_5_20241022: &str = "claude-haiku-3.5-20241022";
        pub const CLAUDE_HAIKU_3_20240307: &str = "claude-haiku-3-20240307";
    }

    // Backwards compatibility - keep old constants working
    pub const GEMINI_2_5_FLASH: &str = google::GEMINI_2_5_FLASH;
    pub const GEMINI_2_5_FLASH_LITE: &str = google::GEMINI_2_5_FLASH_LITE;
    pub const GEMINI_2_5_PRO: &str = google::GEMINI_2_5_PRO;
    pub const GPT_5: &str = openai::GPT_5;
    pub const GPT_5_MINI: &str = openai::GPT_5_MINI;
    pub const CLAUDE_OPUS_4_1_20250805: &str = anthropic::CLAUDE_OPUS_4_1_20250805;
    pub const CLAUDE_OPUS_4_20250514: &str = anthropic::CLAUDE_OPUS_4_20250514;
    pub const CLAUDE_SONNET_4_20250514: &str = anthropic::CLAUDE_SONNET_4_20250514;
    pub const CLAUDE_SONNET_3_7_20250219: &str = anthropic::CLAUDE_SONNET_3_7_20250219;
    pub const CLAUDE_SONNET_3_5_20241022: &str = anthropic::CLAUDE_SONNET_3_5_20241022;
    pub const CLAUDE_HAIKU_3_5_20241022: &str = anthropic::CLAUDE_HAIKU_3_5_20241022;
    pub const CLAUDE_HAIKU_3_20240307: &str = anthropic::CLAUDE_HAIKU_3_20240307;
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

    pub const DEFAULT_MODEL: &str = models::google::GEMINI_2_5_FLASH_LITE;
    pub const DEFAULT_CLI_MODEL: &str = models::google::GEMINI_2_5_FLASH;
    pub const DEFAULT_PROVIDER: &str = "gemini";
    pub const DEFAULT_API_KEY_ENV: &str = "GEMINI_API_KEY";
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

    // Explorer-specific tools
    pub const FILE_METADATA: &str = "file_metadata";
    pub const PROJECT_OVERVIEW: &str = "project_overview";
    pub const TREE_SITTER_ANALYZE: &str = "tree_sitter_analyze";

    // Special wildcard for full access
    pub const WILDCARD_ALL: &str = "*";
}
