//! Semantic analysis for message content

use crate::agent::types::{MessageType, MessagePriority};

/// Semantic analyzer for message content
#[derive(Debug)]
pub struct SemanticAnalyzer {
    security_keywords: Vec<String>,
    code_keywords: Vec<String>,
    decision_keywords: Vec<String>,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new() -> Self {
        Self {
            security_keywords: vec![
                "password".to_string(),
                "token".to_string(),
                "key".to_string(),
                "secret".to_string(),
                "auth".to_string(),
                "login".to_string(),
                "permission".to_string(),
            ],
            code_keywords: vec![
                "function".to_string(),
                "class".to_string(),
                "struct".to_string(),
                "enum".to_string(),
                "impl".to_string(),
                "trait".to_string(),
                "mod".to_string(),
                "use".to_string(),
            ],
            decision_keywords: vec![
                "decision".to_string(),
                "choose".to_string(),
                "select".to_string(),
                "option".to_string(),
                "alternative".to_string(),
                "recommend".to_string(),
            ],
        }
    }

    /// Analyze message priority based on content and type
    pub fn analyze_message_priority(&self, content: &str, message_type: &MessageType) -> MessagePriority {
        // Security-related content is always critical
        if self.contains_security_keywords(content) {
            return MessagePriority::Critical;
        }

        match message_type {
            MessageType::UserMessage => {
                if self.contains_decision_keywords(content) {
                    MessagePriority::High
                } else if self.contains_code_keywords(content) {
                    MessagePriority::High
                } else {
                    MessagePriority::Medium
                }
            }
            MessageType::AssistantResponse => {
                if self.contains_decision_keywords(content) {
                    MessagePriority::High
                } else {
                    MessagePriority::Medium
                }
            }
            MessageType::ToolCall => {
                if content.contains("error") || content.contains("fail") {
                    MessagePriority::High
                } else {
                    MessagePriority::Low
                }
            }
            MessageType::ToolResponse => {
                if content.contains("error") || content.contains("fail") {
                    MessagePriority::High
                } else {
                    MessagePriority::Low
                }
            }
            MessageType::SystemMessage => MessagePriority::Critical,
        }
    }

    /// Extract semantic tags from message content
    pub fn extract_semantic_tags(&self, content: &str) -> Vec<String> {
        let mut tags = Vec::new();

        if self.contains_security_keywords(content) {
            tags.push("security".to_string());
        }

        if self.contains_code_keywords(content) {
            tags.push("code".to_string());
        }

        if self.contains_decision_keywords(content) {
            tags.push("decision".to_string());
        }

        if content.contains("error") || content.contains("fail") {
            tags.push("error".to_string());
        }

        if content.contains("success") || content.contains("complete") {
            tags.push("success".to_string());
        }

        tags
    }

    fn contains_security_keywords(&self, content: &str) -> bool {
        let content_lower = content.to_lowercase();
        self.security_keywords.iter().any(|keyword| content_lower.contains(keyword))
    }

    fn contains_code_keywords(&self, content: &str) -> bool {
        let content_lower = content.to_lowercase();
        self.code_keywords.iter().any(|keyword| content_lower.contains(keyword))
    }

    fn contains_decision_keywords(&self, content: &str) -> bool {
        let content_lower = content.to_lowercase();
        self.decision_keywords.iter().any(|keyword| content_lower.contains(keyword))
    }
}
