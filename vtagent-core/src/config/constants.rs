/// Model ID constants to avoid hardcoding throughout the codebase
pub mod models {
    // Gemini models
    pub const GEMINI_2_5_FLASH_LITE: &str = "gemini-2.5-flash-lite";
    pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";
    pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";

    // OpenAI models
    pub const GPT_5: &str = "gpt-5";
    pub const GPT_5_MINI: &str = "gpt-5-mini";

    // Anthropic models
    pub const CLAUDE_SONNET_4_20250514: &str = "claude-sonnet-4-20250514";
    pub const CLAUDE_OPUS_4_1_20250805: &str = "claude-opus-4-1-20250805";

    // OpenRouter models (provider-prefixed)
    pub const OPENROUTER_ANTHROPIC_CLAUDE_3_5_SONNET: &str = "anthropic/claude-3.5-sonnet";
    pub const OPENROUTER_ANTHROPIC_CLAUDE_3_HAIKU: &str = "anthropic/claude-3-haiku";
    pub const OPENROUTER_OPENAI_GPT_5: &str = "openai/gpt-5";
    pub const OPENROUTER_OPENAI_GPT_5_MINI: &str = "openai/gpt-5-mini";
    pub const OPENROUTER_GOOGLE_GEMINI_2_5_PRO: &str = "google/gemini-2.5-pro";
    pub const OPENROUTER_GOOGLE_GEMINI_2_5_FLASH: &str = "google/gemini-2.5-flash";
    pub const OPENROUTER_GOOGLE_GEMINI_2_5_FLASH_LITE: &str = "google/gemini-2.5-flash-lite";
    pub const OPENROUTER_QWEN_QWEN3_CODER: &str = "qwen/qwen3-coder";
    pub const OPENROUTER_X_AI_GROK_CODE_FAST_1: &str = "x-ai/grok-code-fast-1";
    pub const OPENROUTER_DEEPSEEK_DEEPSEEK_CHAT_V3_1: &str = "deepseek/deepseek-chat-v3.1";

    // LMStudio models
    pub const LMSTUDIO_LOCAL: &str = "local-model";
}

/// Default configuration values
pub mod defaults {
    use super::models;

    pub const DEFAULT_MODEL: &str = models::GEMINI_2_5_FLASH;
    pub const DEFAULT_CLI_MODEL: &str = models::GEMINI_2_5_FLASH_LITE;
    pub const DEFAULT_PROVIDER: &str = "gemini";
    pub const DEFAULT_API_KEY_ENV: &str = "GEMINI_API_KEY";
}
