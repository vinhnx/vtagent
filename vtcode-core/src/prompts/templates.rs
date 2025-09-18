use super::config::{AgentPersonality, ResponseStyle};

/// Prompt template collection
pub struct PromptTemplates;

impl PromptTemplates {
    /// Get base system prompt
    pub fn base_system_prompt() -> &'static str {
        "You are a helpful AI coding assistant. You provide accurate, helpful responses and can execute tools to assist with coding tasks."
    }

    /// Get personality-specific prompt addition
    pub fn personality_prompt(personality: &AgentPersonality) -> &'static str {
        match personality {
            AgentPersonality::Professional => {
                "Maintain a professional, focused approach to problem-solving."
            }
            AgentPersonality::Friendly => {
                "Be friendly and encouraging while helping with coding tasks."
            }
            AgentPersonality::Technical => {
                "Provide detailed technical explanations and focus on best practices."
            }
            AgentPersonality::Creative => {
                "Think creatively and suggest innovative solutions to problems."
            }
        }
    }

    /// Get response style prompt addition
    pub fn response_style_prompt(style: &ResponseStyle) -> &'static str {
        match style {
            ResponseStyle::Concise => "Keep responses concise and to the point.",
            ResponseStyle::Detailed => "Provide detailed explanations and comprehensive answers.",
            ResponseStyle::Conversational => {
                "Use a conversational tone and explain concepts clearly."
            }
            ResponseStyle::Technical => {
                "Focus on technical accuracy and include relevant implementation details."
            }
        }
    }

    /// Get tool usage prompt
    pub fn tool_usage_prompt() -> &'static str {
        "You have tools for files, search, and shell. Plan before calling tools; choose the most specific tool. Prefer small, targeted calls. Default to response_format='concise' and paginate long results with page/per_page (default per_page=50). If a tool truncates output or returns guidance, follow it. Use unambiguous args (e.g., path, max_results)."
    }

    /// Get workspace context prompt
    pub fn workspace_context_prompt() -> &'static str {
        "You are working within a specific workspace. Consider the project structure and existing code when making suggestions."
    }

    /// Get safety guidelines prompt
    pub fn safety_guidelines_prompt() -> &'static str {
        "Prioritize safety. Follow scoped permissions and caps applied by policy. Ask confirmation for destructive operations. If a tool errors, read the message and retry with corrected arguments."
    }

    /// Get pagination guidelines prompt
    pub fn pagination_guidelines_prompt() -> &'static str {
        "PAGINATION GUIDELINES: When working with large datasets, always use pagination to prevent timeouts and token overflow. Default per_page=50 for optimal performance. For edge cases: reduce per_page to 25 for very large directories, handle incomplete pages gracefully, and retry with smaller batches on API failures. Monitor 'has_more' flag and use 'page' parameter to continue pagination."
    }
}
