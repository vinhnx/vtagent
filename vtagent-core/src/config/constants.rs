/// Prompt path constants to avoid hardcoding throughout the codebase
pub mod prompts {
    pub const DEFAULT_SYSTEM_PROMPT_PATH: &str = "prompts/system.md";
}

/// Model ID constants to sync with docs/models.json
pub mod models {
    // Google/Gemini models
    pub mod google {
        pub const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "gemini-2.5-flash-lite",
            "gemini-2.5-flash",
            "gemini-2.5-pro",
        ];

        // Convenience constants for commonly used models
        pub const GEMINI_2_5_FLASH_LITE: &str = "gemini-2.5-flash-lite";
        pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";
        pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";
    }

    // OpenAI models (from docs/models.json)
    pub mod openai {
        pub const DEFAULT_MODEL: &str = "gpt-5";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "gpt-4.1",
            "gpt-5",
            "gpt-5-mini"
        ];

        // Convenience constants for commonly used models
        pub const GPT_5: &str = "gpt-5";
        pub const GPT_5_MINI: &str = "gpt-5-mini";
        pub const GPT_4_1: &str = "gpt-4.1";
    }

    // Anthropic models (from docs/models.json)
    pub mod anthropic {
        pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "claude-opus-4-1-20250805",
            "claude-sonnet-4-20250514",
        ];

        // Convenience constants for commonly used models
        pub const CLAUDE_SONNET_4_20250514: &str = "claude-sonnet-4-20250514";
        pub const CLAUDE_OPUS_4_1_20250805: &str = "claude-opus-4-1-20250805";
    }

    // LMStudio models (from docs/models.json)
    pub mod lmstudio {
        pub const DEFAULT_MODEL: &str = "qwen/qwen3-4b-2507";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "openai/gpt-oss-20b",
            "qwen/qwen3-4b-2507",
            "qwen/qwen3-1.7b",
            "qwen/qwen3-30b-a3b-2507",
            "qwen/qwen3-coder-30b",
        ];

        // Convenience constants for commonly used models
        pub const LMSTUDIO_LOCAL: &str = "local-model"; // Kept for backwards compatibility
        pub const LMSTUDIO_QWEN_30B_A3B_2507: &str = "qwen/qwen3-30b-a3b-2507";
    }

    // OpenRouter models (provider-prefixed - keeping existing for backwards compatibility)
    pub mod openrouter {
        pub const SONOMA_SKY_ALPHA: &str = "openrouter/sonoma-sky-alpha";
        pub const OPENAI_GPT_5: &str = "openai/gpt-5";
        pub const OPENAI_GPT_5_MINI: &str = "openai/gpt-5-mini";
        pub const QWEN_QWEN3_CODER: &str = "qwen/qwen3-coder";
        pub const QWEN_QWEN3_NEXT_80B_A3B_THINKING: &str = "qwen/qwen3-next-80b-a3b-thinking";
        pub const GOOGLE_GEMINI_2_5_PRO: &str = "google/gemini-2.5-pro";
        pub const GOOGLE_GEMINI_2_5_FLASH: &str = "google/gemini-2.5-flash";
        pub const GOOGLE_GEMINI_2_5_FLASH_LITE: &str = "google/gemini-2.5-flash-lite";
        pub const QWEN_QWEN3_30B_A3B_2507: &str = "qwen/qwen3-30b-a3b-2507";
        pub const X_AI_GROK_CODE_FAST_1: &str = "x-ai/grok-code-fast-1";
        pub const DEEPSEEK_DEEPSEEK_CHAT_V3_1: &str = "deepseek/deepseek-chat-v3.1";
        pub const GLM_4_5: &str = "z-ai/glm-4.5";
        pub const QWEN_QWEN3_NEXT_80B_A3B_INSTRUCT: &str = "qwen/qwen3-next-80b-a3b-instruct";
        pub const KIMI_K2_0905: &str = "moonshotai/kimi-k2-0905";
        pub const SUPPORTED_MODELS: &[&str] = &[
            "openai/gpt-5",
            "openai/gpt-5-mini",
            "google/gemini-2.5-pro",
            "google/gemini-2.5-flash",
            "google/gemini-2.5-flash-lite",
            "qwen/qwen3-next-80b-a3b-thinking",
            "qwen/qwen3-next-80b-a3b-instruct",
            "qwen/qwen3-30b-a3b-2507",
            "x-ai/grok-code-fast-1",
            "deepseek/deepseek-chat-v3.1",
            "z-ai/glm-4.5",
            "moonshotai/kimi-k2-0905",
        ];
    }

    // Backwards compatibility - keep old constants working
    pub const GEMINI_2_5_FLASH: &str = google::GEMINI_2_5_FLASH;
    pub const GEMINI_2_5_FLASH_LITE: &str = google::GEMINI_2_5_FLASH_LITE;
    pub const GEMINI_2_5_PRO: &str = google::GEMINI_2_5_PRO;
    pub const GPT_5: &str = openai::GPT_5;
    pub const GPT_5_MINI: &str = openai::GPT_5_MINI;
    pub const CLAUDE_SONNET_4_20250514: &str = anthropic::CLAUDE_SONNET_4_20250514;
    pub const CLAUDE_OPUS_4_1_20250805: &str = anthropic::CLAUDE_OPUS_4_1_20250805;
    pub const LMSTUDIO_LOCAL: &str = lmstudio::LMSTUDIO_LOCAL;
    pub const LMSTUDIO_QWEN_30B_A3B_2507: &str = lmstudio::LMSTUDIO_QWEN_30B_A3B_2507;

    // OpenRouter constants for backwards compatibility
    pub const OPENROUTER_OPENAI_GPT_5: &str = openrouter::OPENAI_GPT_5;
    pub const OPENROUTER_OPENAI_GPT_5_MINI: &str = openrouter::OPENAI_GPT_5_MINI;
    pub const OPENROUTER_GOOGLE_GEMINI_2_5_PRO: &str = openrouter::GOOGLE_GEMINI_2_5_PRO;
    pub const OPENROUTER_GOOGLE_GEMINI_2_5_FLASH: &str = openrouter::GOOGLE_GEMINI_2_5_FLASH;
    pub const OPENROUTER_GOOGLE_GEMINI_2_5_FLASH_LITE: &str =
        openrouter::GOOGLE_GEMINI_2_5_FLASH_LITE;
    pub const OPENROUTER_QWEN_QWEN3_CODER: &str = openrouter::QWEN_QWEN3_CODER;
    pub const OPENROUTER_QWEN_QWEN3_30B_A3B_2507: &str = openrouter::QWEN_QWEN3_30B_A3B_2507;
    pub const OPENROUTER_X_AI_GROK_CODE_FAST_1: &str = openrouter::X_AI_GROK_CODE_FAST_1;
    pub const OPENROUTER_DEEPSEEK_DEEPSEEK_CHAT_V3_1: &str =
        openrouter::DEEPSEEK_DEEPSEEK_CHAT_V3_1;
    pub const OPENROUTER_GLM_4_5: &str = openrouter::GLM_4_5;
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
            "lmstudio" => Some(models::lmstudio::SUPPORTED_MODELS),
            _ => None,
        }
    }

    /// Get default model for a provider
    pub fn default_for(provider: &str) -> Option<&'static str> {
        match provider {
            "google" | "gemini" => Some(models::google::DEFAULT_MODEL),
            "openai" => Some(models::openai::DEFAULT_MODEL),
            "anthropic" => Some(models::anthropic::DEFAULT_MODEL),
            "lmstudio" => Some(models::lmstudio::DEFAULT_MODEL),
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

/// Tool name constants to avoid hardcoding strings throughout the codebase
pub mod tools {
    pub const RP_SEARCH: &str = "rp_search";
    pub const LIST_FILES: &str = "list_files";
    pub const RUN_TERMINAL_CMD: &str = "run_terminal_cmd";
    pub const READ_FILE: &str = "read_file";
    pub const WRITE_FILE: &str = "write_file";
    pub const EDIT_FILE: &str = "edit_file";
    pub const DELETE_FILE: &str = "delete_file";
    pub const CREATE_FILE: &str = "create_file";
    pub const GREP_SEARCH: &str = "grep_search";
    pub const AST_GREP_SEARCH: &str = "ast_grep_search";
}
