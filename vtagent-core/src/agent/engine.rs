//! Compaction engine implementation

use crate::agent::config::CompactionConfig;
use crate::agent::types::{CompactedMessage, EnhancedMessage, MessageType};
use crate::agent::semantic::SemanticAnalyzer;
use crate::gemini::Content;
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main compaction engine
#[derive(Debug)]
pub struct CompactionEngine {
    config: Arc<RwLock<CompactionConfig>>,
    message_history: Arc<RwLock<VecDeque<CompactedMessage>>>,
    enhanced_messages: Arc<RwLock<Vec<EnhancedMessage>>>,
    semantic_analyzer: SemanticAnalyzer,
}

impl CompactionEngine {
    /// Create a new compaction engine
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(CompactionConfig::default())),
            message_history: Arc::new(RwLock::new(VecDeque::new())),
            enhanced_messages: Arc::new(RwLock::new(Vec::new())),
            semantic_analyzer: SemanticAnalyzer::new(),
        }
    }

    /// Add a message to be tracked for compaction
    pub async fn add_message(&self, content: &Content, message_type: MessageType) -> Result<()> {
        // Extract text content
        let text_content = self.extract_text_content(content)?;
        
        // Create compacted message
        let compacted = CompactedMessage {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            message_type: message_type.clone(),
            summary: self.generate_message_summary(&text_content, &message_type)?,
            key_info: self.extract_key_information(&text_content, &message_type)?,
            compression_ratio: 1.0,
            original_size: text_content.len(),
        };

        // Analyze message priority and semantic information
        let priority = self.semantic_analyzer.analyze_message_priority(&text_content, &message_type);
        let semantic_tags = self.semantic_analyzer.extract_semantic_tags(&text_content);

        // Create enhanced message
        let enhanced = EnhancedMessage {
            base_message: compacted,
            priority,
            semantic_tags,
            context_references: Vec::new(),
            conversation_turn: 0,
            related_messages: Vec::new(),
        };

        // Add to histories
        let mut history = self.message_history.write().await;
        history.push_back(compacted);

        let mut enhanced_history = self.enhanced_messages.write().await;
        enhanced_history.push(enhanced);

        Ok(())
    }

    fn extract_text_content(&self, content: &Content) -> Result<String> {
        let mut text = String::new();
        for part in &content.parts {
            if let Some(text_part) = part.as_text() {
                text.push_str(text_part);
                text.push(' ');
            }
        }
        Ok(text.trim().to_string())
    }

    fn generate_message_summary(&self, content: &str, _message_type: &MessageType) -> Result<String> {
        if content.len() <= 100 {
            Ok(content.to_string())
        } else {
            Ok(format!("{}...", &content[..100]))
        }
    }

    fn extract_key_information(&self, content: &str, _message_type: &MessageType) -> Result<Vec<String>> {
        let mut key_info = Vec::new();

        // Simple keyword extraction
        if content.contains("error") {
            key_info.push("error".to_string());
        }
        if content.contains("success") {
            key_info.push("success".to_string());
        }
        if content.contains("function") {
            key_info.push("function".to_string());
        }

        Ok(key_info)
    }
}
