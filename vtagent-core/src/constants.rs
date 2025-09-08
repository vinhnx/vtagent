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
}

/// Default configuration values
pub mod defaults {
    use super::models;

    pub const DEFAULT_MODEL: &str = models::GEMINI_2_5_FLASH;
    pub const DEFAULT_CLI_MODEL: &str = models::GEMINI_2_5_FLASH_LITE;
    pub const DEFAULT_PROVIDER: &str = "gemini";
    pub const DEFAULT_API_KEY_ENV: &str = "GEMINI_API_KEY";
}
