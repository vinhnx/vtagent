//! LLM error display utilities with enhanced ANSI color support
//!
//! This module provides enhanced error display capabilities for LLM providers
//! using standard console styling for consistent terminal output.

use console::style;

/// Get a styled error message with enhanced coloring
pub fn style_llm_error(message: &str) -> String {
    // Use a rich red color for LLM errors
    style(message).red().to_string()
}

/// Get a styled warning message with enhanced coloring
pub fn style_llm_warning(message: &str) -> String {
    // Use an amber color for LLM warnings
    style(message).yellow().to_string()
}

/// Get a styled success message with enhanced coloring
pub fn style_llm_success(message: &str) -> String {
    // Use a vibrant green color for LLM success messages
    style(message).green().to_string()
}

/// Get a styled provider name with enhanced coloring based on provider type
pub fn style_provider_name(provider: &str) -> String {
    let styled_name = match provider.to_lowercase().as_str() {
        "gemini" => {
            // Deep blue for Gemini
            style(provider).blue()
        }
        "openai" => {
            // Bright orange for OpenAI (using yellow as approximation)
            style(provider).yellow()
        }
        "anthropic" => {
            // Anthropic's brand purple (using magenta as approximation)
            style(provider).magenta()
        }
        _ => {
            // Default styling for other providers
            style(provider).cyan()
        }
    };
    styled_name.to_string()
}

/// Format an LLM error for display with enhanced coloring
pub fn format_llm_error(provider: &str, error: &str) -> String {
    let provider_styled = style_provider_name(provider);
    let error_styled = style_llm_error(error);
    format!("{} {}", provider_styled, error_styled)
}

/// Format an LLM warning for display with enhanced coloring
pub fn format_llm_warning(provider: &str, warning: &str) -> String {
    let provider_styled = style_provider_name(provider);
    let warning_styled = style_llm_warning(warning);
    format!("{} {}", provider_styled, warning_styled)
}

/// Format an LLM success message for display with enhanced coloring
pub fn format_llm_success(provider: &str, message: &str) -> String {
    let provider_styled = style_provider_name(provider);
    let success_styled = style_llm_success(message);
    format!("{} {}", provider_styled, success_styled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_llm_error() {
        let result = style_llm_error("Test error");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_style_llm_warning() {
        let result = style_llm_warning("Test warning");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_style_llm_success() {
        let result = style_llm_success("Test success");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_style_provider_name() {
        let providers = vec!["gemini", "openai", "anthropic", "unknown"];
        for provider in providers {
            let result = style_provider_name(provider);
            assert!(!result.is_empty());
        }
    }

    #[test]
    fn test_format_llm_error() {
        let result = format_llm_error("gemini", "Connection failed");
        assert!(result.contains("gemini"));
        assert!(result.contains("Connection failed"));
    }

    #[test]
    fn test_format_llm_warning() {
        let result = format_llm_warning("openai", "Rate limit approaching");
        assert!(result.contains("openai"));
        assert!(result.contains("Rate limit approaching"));
    }

    #[test]
    fn test_format_llm_success() {
        let result = format_llm_success("anthropic", "Request completed");
        assert!(result.contains("anthropic"));
        assert!(result.contains("Request completed"));
    }
}
