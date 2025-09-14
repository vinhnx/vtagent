use crate::llm::provider::{Message, MessageRole};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Cached prompt entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPrompt {
    pub prompt_hash: String,
    pub original_prompt: String,
    pub optimized_prompt: String,
    pub model_used: String,
    pub tokens_saved: Option<u32>,
    pub quality_score: Option<f64>,
    pub created_at: u64,
    pub last_used: u64,
    pub usage_count: u32,
}

/// Prompt caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCacheConfig {
    pub cache_dir: PathBuf,
    pub max_cache_size: usize,
    pub max_age_days: u64,
    pub enable_auto_cleanup: bool,
    pub min_quality_threshold: f64,
}

impl Default for PromptCacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".vtagent")
                .join("cache")
                .join("prompts"),
            max_cache_size: 1000, // Maximum number of cached prompts
            max_age_days: 30,     // Cache entries older than 30 days are cleaned up
            enable_auto_cleanup: true,
            min_quality_threshold: 0.7, // Minimum quality score to cache
        }
    }
}

/// Prompt caching system
pub struct PromptCache {
    config: PromptCacheConfig,
    cache: HashMap<String, CachedPrompt>,
    dirty: bool,
}

impl PromptCache {
    pub fn new() -> Self {
        Self::with_config(PromptCacheConfig::default())
    }

    pub fn with_config(config: PromptCacheConfig) -> Self {
        let mut cache = Self {
            config,
            cache: HashMap::new(),
            dirty: false,
        };

        // Load existing cache
        let _ = cache.load_cache();

        // Auto cleanup if enabled
        if cache.config.enable_auto_cleanup {
            let _ = cache.cleanup_expired();
        }

        cache
    }

    /// Get cached optimized prompt
    pub fn get(&mut self, prompt_hash: &str) -> Option<&CachedPrompt> {
        if let Some(entry) = self.cache.get_mut(prompt_hash) {
            entry.last_used = Self::current_timestamp();
            entry.usage_count += 1;
            self.dirty = true;
            Some(entry)
        } else {
            None
        }
    }

    /// Store optimized prompt in cache
    pub fn put(&mut self, entry: CachedPrompt) -> Result<(), PromptCacheError> {
        // Check quality threshold
        if let Some(quality) = entry.quality_score {
            if quality < self.config.min_quality_threshold {
                return Ok(()); // Don't cache low-quality entries
            }
        }

        // Check cache size limit
        if self.cache.len() >= self.config.max_cache_size {
            self.evict_oldest()?;
        }

        self.cache.insert(entry.prompt_hash.clone(), entry);
        self.dirty = true;

        Ok(())
    }

    /// Check if prompt is cached
    pub fn contains(&self, prompt_hash: &str) -> bool {
        self.cache.contains_key(prompt_hash)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.cache.len();
        let total_usage = self.cache.values().map(|e| e.usage_count).sum::<u32>();
        let total_tokens_saved = self
            .cache
            .values()
            .filter_map(|e| e.tokens_saved)
            .sum::<u32>();
        let avg_quality = if !self.cache.is_empty() {
            self.cache
                .values()
                .filter_map(|e| e.quality_score)
                .sum::<f64>()
                / self.cache.len() as f64
        } else {
            0.0
        };

        CacheStats {
            total_entries,
            total_usage,
            total_tokens_saved,
            avg_quality,
        }
    }

    /// Clear all cache entries
    pub fn clear(&mut self) -> Result<(), PromptCacheError> {
        self.cache.clear();
        self.dirty = true;
        self.save_cache()
    }

    /// Generate hash for prompt
    pub fn hash_prompt(prompt: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Save cache to disk
    pub fn save_cache(&self) -> Result<(), PromptCacheError> {
        if !self.dirty {
            return Ok(());
        }

        // Ensure cache directory exists
        fs::create_dir_all(&self.config.cache_dir).map_err(|e| PromptCacheError::Io(e))?;

        let cache_path = self.config.cache_dir.join("prompt_cache.json");
        let data = serde_json::to_string_pretty(&self.cache)
            .map_err(|e| PromptCacheError::Serialization(e))?;

        fs::write(cache_path, data).map_err(|e| PromptCacheError::Io(e))?;

        Ok(())
    }

    /// Load cache from disk
    fn load_cache(&mut self) -> Result<(), PromptCacheError> {
        let cache_path = self.config.cache_dir.join("prompt_cache.json");

        if !cache_path.exists() {
            return Ok(());
        }

        let data = fs::read_to_string(cache_path).map_err(|e| PromptCacheError::Io(e))?;

        self.cache = serde_json::from_str(&data).map_err(|e| PromptCacheError::Serialization(e))?;

        Ok(())
    }

    /// Clean up expired cache entries
    fn cleanup_expired(&mut self) -> Result<(), PromptCacheError> {
        let now = Self::current_timestamp();
        let max_age_seconds = self.config.max_age_days * 24 * 60 * 60;

        self.cache
            .retain(|_, entry| now - entry.created_at < max_age_seconds);

        self.dirty = true;
        Ok(())
    }

    /// Evict oldest cache entries when cache is full
    fn evict_oldest(&mut self) -> Result<(), PromptCacheError> {
        if self.cache.is_empty() {
            return Ok(());
        }

        // Find the oldest entry
        let oldest_key = self
            .cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| key.clone())
            .unwrap();

        self.cache.remove(&oldest_key);
        self.dirty = true;

        Ok(())
    }

    /// Get current timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

impl Drop for PromptCache {
    fn drop(&mut self) {
        let _ = self.save_cache();
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_usage: u32,
    pub total_tokens_saved: u32,
    pub avg_quality: f64,
}

/// Prompt cache errors
#[derive(Debug, thiserror::Error)]
pub enum PromptCacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cache full")]
    CacheFull,
}

/// Prompt optimizer that uses caching
pub struct PromptOptimizer {
    cache: PromptCache,
    llm_provider: Box<dyn crate::llm::provider::LLMProvider>,
}

impl PromptOptimizer {
    pub fn new(llm_provider: Box<dyn crate::llm::provider::LLMProvider>) -> Self {
        Self {
            cache: PromptCache::new(),
            llm_provider,
        }
    }

    pub fn with_cache(mut self, cache: PromptCache) -> Self {
        self.cache = cache;
        self
    }

    /// Optimize a prompt using caching
    pub async fn optimize_prompt(
        &mut self,
        original_prompt: &str,
        target_model: &str,
        context: Option<&str>,
    ) -> Result<String, PromptOptimizationError> {
        let prompt_hash = PromptCache::hash_prompt(original_prompt);

        // Check cache first
        if let Some(cached) = self.cache.get(&prompt_hash) {
            return Ok(cached.optimized_prompt.clone());
        }

        // Generate optimized prompt
        let optimized = self
            .generate_optimized_prompt(original_prompt, target_model, context)
            .await?;

        // Calculate tokens saved (rough estimate)
        let original_tokens = Self::estimate_tokens(original_prompt);
        let optimized_tokens = Self::estimate_tokens(&optimized);
        let tokens_saved = original_tokens.saturating_sub(optimized_tokens);

        // Create cache entry
        let entry = CachedPrompt {
            prompt_hash: prompt_hash.clone(),
            original_prompt: original_prompt.to_string(),
            optimized_prompt: optimized.clone(),
            model_used: target_model.to_string(),
            tokens_saved: Some(tokens_saved),
            quality_score: Some(0.8), // Placeholder quality score
            created_at: PromptCache::current_timestamp(),
            last_used: PromptCache::current_timestamp(),
            usage_count: 1,
        };

        // Store in cache
        self.cache.put(entry)?;

        Ok(optimized)
    }

    /// Generate optimized prompt using LLM
    async fn generate_optimized_prompt(
        &self,
        original_prompt: &str,
        target_model: &str,
        context: Option<&str>,
    ) -> Result<String, PromptOptimizationError> {
        let system_prompt = format!(
            "You are an expert prompt engineer. Your task is to optimize prompts for {} \
             to make them more effective, clearer, and more likely to produce high-quality responses. \
             Focus on improving clarity, specificity, structure, and effectiveness while preserving \
             the original intent and requirements.",
            target_model
        );

        let mut user_prompt = format!(
            "Please optimize the following prompt for {}:\n\nORIGINAL PROMPT:\n{}\n\n",
            target_model, original_prompt
        );

        if let Some(ctx) = context {
            user_prompt.push_str(&format!("CONTEXT:\n{}\n\n", ctx));
        }

        user_prompt.push_str(
            "OPTIMIZATION REQUIREMENTS:\n\
             1. Make the prompt clearer and more specific\n\
             2. Improve structure and formatting\n\
             3. Add relevant context or examples if helpful\n\
             4. Ensure the prompt is appropriate for the target model\n\
             5. Maintain the original intent and requirements\n\
             6. Keep the optimized prompt concise but comprehensive\n\n\
             Provide only the optimized prompt without any explanation or additional text.",
        );

        let request = crate::llm::provider::LLMRequest {
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: system_prompt,
                    tool_calls: None,
                    tool_call_id: None,
                },
                Message {
                    role: MessageRole::User,
                    content: user_prompt,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            system_prompt: None,
            tools: None,
            model: target_model.to_string(),
            max_tokens: Some(2000),
            temperature: Some(0.3),
            stream: false,
            tool_choice: None,
            parallel_tool_calls: None,
            reasoning_effort: None,
        };

        let response = self
            .llm_provider
            .generate(request)
            .await
            .map_err(|e| PromptOptimizationError::LLMError(e.to_string()))?;

        Ok(response
            .content
            .unwrap_or_else(|| original_prompt.to_string()))
    }

    /// Estimate token count (rough approximation)
    fn estimate_tokens(text: &str) -> u32 {
        // Rough approximation: 1 token ≈ 4 characters for English text
        (text.len() / 4) as u32
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Clear cache
    pub fn clear_cache(&mut self) -> Result<(), PromptCacheError> {
        self.cache.clear()
    }
}

/// Prompt optimization errors
#[derive(Debug, thiserror::Error)]
pub enum PromptOptimizationError {
    #[error("LLM error: {0}")]
    LLMError(String),

    #[error("Cache error: {0}")]
    CacheError(#[from] PromptCacheError),
}

#[cfg(test)]
mod tests {
    use crate::llm::provider::{
        FinishReason, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, LLMError,
    };
    use super::*;

    #[test]
    fn test_prompt_hash() {
        let prompt = "Test prompt";
        let hash1 = PromptCache::hash_prompt(prompt);
        let hash2 = PromptCache::hash_prompt(prompt);
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_cache_operations() {
        let mut cache = PromptCache::new();

        let entry = CachedPrompt {
            prompt_hash: "test_hash".to_string(),
            original_prompt: "original".to_string(),
            optimized_prompt: "optimized".to_string(),
            model_used: crate::config::constants::models::GEMINI_2_5_FLASH.to_string(),
            tokens_saved: Some(100),
            quality_score: Some(0.9),
            created_at: 1000,
            last_used: 1000,
            usage_count: 0,
        };

        cache.put(entry).unwrap();
        assert!(cache.contains("test_hash"));

        let retrieved = cache.get("test_hash");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().usage_count, 1);
    }

    // Mock provider for testing
    struct MockProvider;

    #[async_trait::async_trait]
    impl LLMProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        async fn generate(
            &self,
            _request: LLMRequest,
        ) -> Result<LLMResponse, LLMError> {
            Ok(LLMResponse {
                content: Some("Optimized prompt".to_string()),
                tool_calls: None,
                usage: None,
                finish_reason: FinishReason::Stop,
            })
        }

        fn supported_models(&self) -> Vec<String> {
            vec!["mock".to_string()]
        }

        fn validate_request(
            &self,
            _request: &LLMRequest,
        ) -> Result<(), LLMError> {
            Ok(())
        }
    }
}
