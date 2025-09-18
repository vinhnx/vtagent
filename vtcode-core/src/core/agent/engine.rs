//! Compaction engine implementation

use crate::core::agent::config::CompactionConfig;
use crate::core::agent::semantic::SemanticAnalyzer;
use crate::core::agent::types::{
    CompactedMessage, CompactionResult, CompactionStatistics, CompactionSuggestion,
    EnhancedMessage, MessagePriority, MessageType, Urgency,
};
use crate::gemini::Content;
use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Main compaction engine
#[derive(Debug)]
pub struct CompactionEngine {
    config: Arc<RwLock<CompactionConfig>>,
    message_history: Arc<RwLock<VecDeque<CompactedMessage>>>,
    enhanced_messages: Arc<RwLock<Vec<EnhancedMessage>>>,
    semantic_analyzer: SemanticAnalyzer,
    last_compaction: Arc<RwLock<u64>>,
    compaction_count: Arc<RwLock<u64>>,
    start_time: Instant,
}

impl CompactionEngine {
    /// Create a new compaction engine
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(CompactionConfig::default())),
            message_history: Arc::new(RwLock::new(VecDeque::new())),
            enhanced_messages: Arc::new(RwLock::new(Vec::new())),
            semantic_analyzer: SemanticAnalyzer::new(),
            last_compaction: Arc::new(RwLock::new(0)),
            compaction_count: Arc::new(RwLock::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Create a new compaction engine with custom configuration
    pub fn with_config(config: CompactionConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            message_history: Arc::new(RwLock::new(VecDeque::new())),
            enhanced_messages: Arc::new(RwLock::new(Vec::new())),
            semantic_analyzer: SemanticAnalyzer::new(),
            last_compaction: Arc::new(RwLock::new(0)),
            compaction_count: Arc::new(RwLock::new(0)),
            start_time: Instant::now(),
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
        let priority = self
            .semantic_analyzer
            .analyze_message_priority(&text_content, &message_type);
        let semantic_tags = self.semantic_analyzer.extract_semantic_tags(&text_content);

        // Create enhanced message
        let enhanced = EnhancedMessage {
            base_message: compacted.clone(),
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

    fn generate_message_summary(
        &self,
        content: &str,
        _message_type: &MessageType,
    ) -> Result<String> {
        if content.len() <= 100 {
            Ok(content.to_string())
        } else {
            Ok(format!("{}...", &content[..100]))
        }
    }

    fn extract_key_information(
        &self,
        content: &str,
        _message_type: &MessageType,
    ) -> Result<Vec<String>> {
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

    /// Get compaction suggestions
    pub async fn get_compaction_suggestions(&self) -> Result<Vec<CompactionSuggestion>> {
        let config = self.config.read().await.clone();
        let history = self.message_history.read().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut suggestions = Vec::new();

        for msg in history.iter() {
            let too_old = now.saturating_sub(msg.timestamp) > config.max_message_age_seconds;
            let over_limit = history.len() > config.max_uncompressed_messages;
            if too_old || over_limit {
                let urgency = if over_limit {
                    Urgency::High
                } else {
                    Urgency::Medium
                };
                suggestions.push(CompactionSuggestion {
                    action: "compact_message".to_string(),
                    urgency,
                    estimated_savings: msg.original_size,
                    reasoning: "message exceeds configured thresholds".to_string(),
                });
            }
        }

        Ok(suggestions)
    }

    /// Get statistics
    pub async fn get_statistics(&self) -> Result<CompactionStatistics> {
        let history = self.message_history.read().await;
        let enhanced = self.enhanced_messages.read().await;
        let total_messages = history.len();
        let total_memory_usage: usize = history.iter().map(|m| m.original_size).sum();
        let average_message_size = if total_messages > 0 {
            total_memory_usage / total_messages
        } else {
            0
        };
        let mut messages_by_priority: HashMap<MessagePriority, usize> = HashMap::new();
        for msg in enhanced.iter() {
            *messages_by_priority
                .entry(msg.priority.clone())
                .or_insert(0) += 1;
        }
        let last_compaction_timestamp = *self.last_compaction.read().await;
        let elapsed_hours = self.start_time.elapsed().as_secs() as f64 / 3600.0;
        let count = *self.compaction_count.read().await as f64;
        let compaction_frequency = if elapsed_hours > 0.0 {
            count / elapsed_hours
        } else {
            0.0
        };

        Ok(CompactionStatistics {
            total_messages,
            messages_by_priority,
            total_memory_usage,
            average_message_size,
            last_compaction_timestamp,
            compaction_frequency,
        })
    }

    /// Check if should compact
    pub async fn should_compact(&self) -> Result<bool> {
        let config = self.config.read().await.clone();
        let history = self.message_history.read().await;
        let total_memory: usize = history.iter().map(|m| m.original_size).sum();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = *self.last_compaction.read().await;

        if history.len() > config.max_uncompressed_messages {
            return Ok(true);
        }
        if total_memory > config.max_memory_mb * 1_000_000 {
            return Ok(true);
        }
        if config.auto_compaction_enabled
            && now.saturating_sub(last) > config.compaction_interval_seconds
        {
            return Ok(true);
        }
        Ok(false)
    }

    /// Compact messages intelligently
    pub async fn compact_messages_intelligently(&self) -> Result<CompactionResult> {
        let start = Instant::now();
        let config = self.config.read().await.clone();
        let mut history = self.message_history.write().await;
        let mut enhanced = self.enhanced_messages.write().await;
        let mut messages_compacted = 0usize;
        let mut original_size = 0usize;

        while history.len() > config.max_uncompressed_messages {
            if let Some(msg) = history.pop_front() {
                original_size += msg.original_size;
                messages_compacted += 1;
                if !enhanced.is_empty() {
                    enhanced.remove(0);
                }
            }
        }

        let processing_time_ms = start.elapsed().as_millis() as u64;
        if messages_compacted > 0 {
            let mut last = self.last_compaction.write().await;
            *last = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let mut count = self.compaction_count.write().await;
            *count += 1;
        }

        Ok(CompactionResult {
            messages_processed: messages_compacted,
            messages_compacted,
            original_size,
            compacted_size: 0,
            compression_ratio: if original_size > 0 { 0.0 } else { 1.0 },
            processing_time_ms,
        })
    }

    /// Compact context
    pub async fn compact_context(
        &self,
        _context_key: &str,
        context_data: &mut std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<CompactionResult> {
        let start = Instant::now();
        let config = self.config.read().await.clone();

        let original_size: usize = context_data
            .values()
            .filter_map(|v| v.as_str().map(|s| s.len()))
            .sum();
        let initial_len = context_data.len();

        context_data.retain(|_, v| {
            v.get("confidence")
                .and_then(|c| c.as_f64())
                .map(|c| c >= config.min_context_confidence)
                .unwrap_or(true)
        });

        let compacted_size: usize = context_data
            .values()
            .filter_map(|v| v.as_str().map(|s| s.len()))
            .sum();
        let messages_compacted = initial_len - context_data.len();

        if messages_compacted > 0 {
            let mut last = self.last_compaction.write().await;
            *last = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let mut count = self.compaction_count.write().await;
            *count += 1;
        }

        Ok(CompactionResult {
            messages_processed: initial_len,
            messages_compacted,
            original_size,
            compacted_size,
            compression_ratio: if original_size > 0 {
                compacted_size as f64 / original_size as f64
            } else {
                1.0
            },
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}

impl Default for CompactionEngine {
    fn default() -> Self {
        Self::new()
    }
}
