use crate::config::constants::defaults;
use serde::{Deserialize, Serialize};

/// Agent-wide configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// AI provider for single agent mode (gemini, openai, anthropic)
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Default model to use
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Maximum number of conversation turns before auto-termination
    #[serde(default = "default_max_conversation_turns")]
    pub max_conversation_turns: usize,

    /// Reasoning effort level for models that support it (low, medium, high)
    /// Applies to: Claude, GPT-5, Gemini, Qwen3, DeepSeek with reasoning capability
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,

    /// Enable an extra self-review pass to refine final responses
    #[serde(default = "default_enable_self_review")]
    pub enable_self_review: bool,

    /// Maximum number of self-review passes
    #[serde(default = "default_max_review_passes")]
    pub max_review_passes: usize,

    /// Enable prompt refinement pass before sending to LLM
    #[serde(default = "default_refine_prompts_enabled")]
    pub refine_prompts_enabled: bool,

    /// Max refinement passes for prompt writing
    #[serde(default = "default_refine_max_passes")]
    pub refine_prompts_max_passes: usize,

    /// Optional model override for the refiner (empty = auto pick efficient sibling)
    #[serde(default)]
    pub refine_prompts_model: String,

    /// Session onboarding and welcome message configuration
    #[serde(default)]
    pub onboarding: AgentOnboardingConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            default_model: default_model(),
            max_conversation_turns: default_max_conversation_turns(),
            reasoning_effort: default_reasoning_effort(),
            enable_self_review: default_enable_self_review(),
            max_review_passes: default_max_review_passes(),
            refine_prompts_enabled: default_refine_prompts_enabled(),
            refine_prompts_max_passes: default_refine_max_passes(),
            refine_prompts_model: String::new(),
            onboarding: AgentOnboardingConfig::default(),
        }
    }
}

fn default_provider() -> String {
    defaults::DEFAULT_PROVIDER.to_string()
}
fn default_model() -> String {
    defaults::DEFAULT_MODEL.to_string()
}
fn default_max_conversation_turns() -> usize {
    150
}
fn default_reasoning_effort() -> String {
    "medium".to_string()
}

fn default_enable_self_review() -> bool {
    false
}

fn default_max_review_passes() -> usize {
    1
}

fn default_refine_prompts_enabled() -> bool {
    true
}

fn default_refine_max_passes() -> usize {
    1
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentOnboardingConfig {
    /// Toggle onboarding message rendering
    #[serde(default = "default_onboarding_enabled")]
    pub enabled: bool,

    /// Introductory text shown at session start
    #[serde(default = "default_intro_text")]
    pub intro_text: String,

    /// Whether to include project overview in onboarding message
    #[serde(default = "default_show_project_overview")]
    pub include_project_overview: bool,

    /// Whether to include language summary in onboarding message
    #[serde(default = "default_show_language_summary")]
    pub include_language_summary: bool,

    /// Whether to include AGENTS.md highlights in onboarding message
    #[serde(default = "default_show_guideline_highlights")]
    pub include_guideline_highlights: bool,

    /// Maximum number of guideline bullets to surface
    #[serde(default = "default_guideline_highlight_limit")]
    pub guideline_highlight_limit: usize,

    /// Tips for collaborating with the agent effectively
    #[serde(default = "default_usage_tips")]
    pub usage_tips: Vec<String>,

    /// Recommended follow-up actions to display
    #[serde(default = "default_recommended_actions")]
    pub recommended_actions: Vec<String>,

    /// Placeholder suggestion for the chat input bar
    #[serde(default = "default_chat_placeholder")]
    pub chat_placeholder: String,
}

impl Default for AgentOnboardingConfig {
    fn default() -> Self {
        Self {
            enabled: default_onboarding_enabled(),
            intro_text: default_intro_text(),
            include_project_overview: default_show_project_overview(),
            include_language_summary: default_show_language_summary(),
            include_guideline_highlights: default_show_guideline_highlights(),
            guideline_highlight_limit: default_guideline_highlight_limit(),
            usage_tips: default_usage_tips(),
            recommended_actions: default_recommended_actions(),
            chat_placeholder: default_chat_placeholder(),
        }
    }
}

fn default_onboarding_enabled() -> bool {
    true
}

fn default_intro_text() -> String {
    "Let's get oriented. I preloaded workspace context so we can move fast.".to_string()
}

fn default_show_project_overview() -> bool {
    true
}

fn default_show_language_summary() -> bool {
    false
}

fn default_show_guideline_highlights() -> bool {
    true
}

fn default_guideline_highlight_limit() -> usize {
    3
}

fn default_usage_tips() -> Vec<String> {
    vec![
        "Describe your current coding goal or ask for a quick status overview.".to_string(),
        "Reference AGENTS.md guidelines when proposing changes.".to_string(),
        "Prefer asking for targeted file reads or diffs before editing.".to_string(),
    ]
}

fn default_recommended_actions() -> Vec<String> {
    vec![
        "Review the highlighted guidelines and share the task you want to tackle.".to_string(),
        "Ask for a workspace tour if you need more context.".to_string(),
    ]
}

fn default_chat_placeholder() -> String {
    "Describe your next coding step (e.g., \"audit router config\")".to_string()
}
