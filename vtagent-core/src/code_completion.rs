use crate::performance_monitor::PERFORMANCE_MONITOR;
use crate::tree_sitter::{CodeAnalysis, TreeSitterAnalyzer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Code completion suggestion with metadata
#[derive(Debug, Clone)]
pub struct CompletionSuggestion {
    /// Completion acceptance rate (target: 70%)
    pub acceptance_rate: f64,
    /// Learning data for improving suggestions
    pub learning_data: CompletionLearningData,
    pub text: String,
    pub kind: CompletionKind,
    pub confidence: f64,
    pub context: CompletionContext,
    pub metadata: HashMap<String, String>,
    pub accepted_count: usize,
    pub rejected_count: usize,
}

/// Type of completion suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompletionKind {
    Function,
    Method,
    Variable,
    Class,
    Struct,
    Enum,
    Trait,
    Module,
    Keyword,
    Snippet,
    Import,
    Type,
}

/// Context information for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionContext {
    pub line: usize,
    pub column: usize,
    pub prefix: String,
    pub language: String,
    pub scope: Vec<String>,
    pub imports: Vec<String>,
    pub recent_symbols: Vec<String>,
}

/// Code completion engine
pub struct CodeCompletionEngine {
    analyzers: HashMap<String, TreeSitterAnalyzer>,
    suggestion_cache: Arc<RwLock<HashMap<String, Vec<CompletionSuggestion>>>>,
    learning_data: Arc<RwLock<CompletionLearningData>>,
    performance_stats: Arc<RwLock<CompletionStats>>,
}

/// Learning data for improving completion accuracy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionLearningData {
    pub user_patterns: HashMap<String, Vec<String>>, // Simplified: just preferred suggestions
    pub language_patterns: HashMap<String, Vec<String>>, // Simplified: just common patterns
    pub acceptance_rates: HashMap<String, f64>,      // Simplified: just acceptance rates
}

impl Default for CompletionLearningData {
    fn default() -> Self {
        Self {
            user_patterns: HashMap::new(),
            language_patterns: HashMap::new(),
            acceptance_rates: HashMap::new(),
        }
    }
}

/// User-specific completion patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPattern {
    pub preferred_suggestions: Vec<String>,
    pub rejected_suggestions: Vec<String>, // Changed from HashSet to Vec for serialization
    pub common_prefixes: HashMap<String, Vec<String>>,
    pub last_updated: String, // Changed from Instant to String for serialization
}

/// Language-specific patterns
#[derive(Debug, Clone)]
pub struct LanguagePattern {
    pub common_completions: HashMap<String, Vec<CompletionSuggestion>>,
    pub context_patterns: Vec<ContextPattern>,
    pub semantic_rules: Vec<SemanticRule>,
}

/// Context pattern for completion
#[derive(Debug, Clone)]
pub struct ContextPattern {
    pub trigger: String,
    pub context_type: String,
    pub suggestions: Vec<String>,
    pub confidence: f64,
}

/// Semantic rule for intelligent completion
#[derive(Debug, Clone)]
pub struct SemanticRule {
    pub pattern: String,
    pub condition: String,
    pub action: String,
    pub priority: i32,
}

/// Acceptance statistics for learning
#[derive(Debug, Clone)]
pub struct AcceptanceStats {
    pub total_suggestions: usize,
    pub accepted_suggestions: usize,
    pub acceptance_rate: f64,
    pub last_updated: Instant,
}

/// Completion performance statistics
#[derive(Debug, Clone, Default)]
pub struct CompletionStats {
    pub total_requests: usize,
    pub cache_hits: usize,
    pub average_response_time: Duration,
    pub language_stats: HashMap<String, LanguageCompletionStats>,
}

/// Language-specific completion stats
#[derive(Debug, Clone, Default)]
pub struct LanguageCompletionStats {
    pub requests: usize,
    pub successful_completions: usize,
    pub average_confidence: f64,
    pub top_suggestion_accuracy: f64,
}

impl CodeCompletionEngine {
    pub fn new() -> Self {
        let mut analyzers = HashMap::new();

        // Initialize tree-sitter analyzers for supported languages
        analyzers.insert("rust".to_string(), TreeSitterAnalyzer::new().unwrap());
        analyzers.insert("python".to_string(), TreeSitterAnalyzer::new().unwrap());
        analyzers.insert("javascript".to_string(), TreeSitterAnalyzer::new().unwrap());
        analyzers.insert("typescript".to_string(), TreeSitterAnalyzer::new().unwrap());

        Self {
            analyzers,
            suggestion_cache: Arc::new(RwLock::new(HashMap::new())),
            learning_data: Arc::new(RwLock::new(CompletionLearningData::default())),
            performance_stats: Arc::new(RwLock::new(CompletionStats::default())),
        }
    }

    /// Generate completion suggestions for given context
    pub async fn generate_completions(
        &self,
        code: &str,
        context: CompletionContext,
        max_suggestions: usize,
    ) -> Result<Vec<CompletionSuggestion>, CompletionError> {
        let start_time = Instant::now();
        PERFORMANCE_MONITOR
            .start_operation(format!("completion_{}", context.language))
            .await;

        let cache_key = format!(
            "{}:{}:{}:{}",
            context.language, context.line, context.column, context.prefix
        );

        // Check cache first
        if let Some(cached) = self.check_cache(&cache_key).await {
            PERFORMANCE_MONITOR
                .end_operation(format!("completion_{}", context.language), true)
                .await;
            return Ok(cached.into_iter().take(max_suggestions).collect());
        }

        // Generate new suggestions
        let suggestions = self
            .generate_fresh_completions(code, &context, max_suggestions)
            .await?;

        // Cache the results
        self.update_cache(cache_key, suggestions.clone()).await;

        // Update performance stats
        let duration = start_time.elapsed();
        self.update_performance_stats(&context.language, duration, !suggestions.is_empty())
            .await;

        PERFORMANCE_MONITOR
            .end_operation(format!("completion_{}", context.language), true)
            .await;

        Ok(suggestions)
    }

    /// Check cache for existing suggestions
    async fn check_cache(&self, cache_key: &str) -> Option<Vec<CompletionSuggestion>> {
        let cache = self.suggestion_cache.read().await;
        cache.get(cache_key).cloned()
    }

    /// Update cache with new suggestions
    async fn update_cache(&self, cache_key: String, suggestions: Vec<CompletionSuggestion>) {
        let mut cache = self.suggestion_cache.write().await;
        cache.insert(cache_key, suggestions);

        // Keep cache size manageable
        if cache.len() > 1000 {
            // Remove oldest entries (simple FIFO eviction)
            let keys_to_remove: Vec<String> = cache.keys().take(100).cloned().collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }
    }

    /// Generate fresh completion suggestions
    async fn generate_fresh_completions(
        &self,
        code: &str,
        context: &CompletionContext,
        max_suggestions: usize,
    ) -> Result<Vec<CompletionSuggestion>, CompletionError> {
        let mut suggestions = Vec::new();

        // Analyze code structure using tree-sitter
        // Create a new analyzer for each call to avoid borrowing issues
        if let Ok(mut analyzer) = TreeSitterAnalyzer::new() {
            // Create a temporary file path for analysis
            let temp_path = std::path::Path::new("temp");
            if let Ok(analysis) = analyzer.analyze_file_with_tree_sitter(temp_path, code) {
                // Generate semantic completions
                let semantic_suggestions = self
                    .generate_semantic_completions(&analysis, context)
                    .await?;
                suggestions.extend(semantic_suggestions);

                // Generate context-aware completions
                let context_suggestions = self
                    .generate_context_completions(&analysis, context)
                    .await?;
                suggestions.extend(context_suggestions);
            }
        }

        // Generate pattern-based completions
        let pattern_suggestions = self.generate_pattern_completions(context).await?;
        suggestions.extend(pattern_suggestions);

        // Generate keyword completions
        let keyword_suggestions =
            self.generate_keyword_completions(&context.language, &context.prefix)?;
        suggestions.extend(keyword_suggestions);

        // Rank and filter suggestions
        let ranked_suggestions = self.rank_suggestions(suggestions, context).await?;
        let filtered_suggestions = self
            .filter_suggestions(ranked_suggestions, max_suggestions)
            .await?;

        Ok(filtered_suggestions)
    }

    /// Generate semantic completions based on code analysis
    async fn generate_semantic_completions(
        &self,
        analysis: &CodeAnalysis,
        context: &CompletionContext,
    ) -> Result<Vec<CompletionSuggestion>, CompletionError> {
        let mut suggestions = Vec::new();

        // Extract symbols from current scope
        for symbol in &analysis.symbols {
            if symbol.name.starts_with(&context.prefix) && symbol.name != context.prefix {
                let kind = self.map_symbol_kind(&format!("{:?}", symbol.kind));
                let confidence = self.calculate_semantic_confidence(symbol, context);

                suggestions.push(CompletionSuggestion {
                    acceptance_rate: 0.0,
                    learning_data: CompletionLearningData {
                        user_patterns: HashMap::new(),
                        language_patterns: HashMap::new(),
                        acceptance_rates: HashMap::new(),
                    },
                    text: symbol.name.clone(),
                    kind,
                    confidence,
                    context: context.clone(),
                    metadata: HashMap::from([
                        (
                            "scope".to_string(),
                            symbol.scope.clone().unwrap_or_default(),
                        ),
                        ("source".to_string(), "semantic".to_string()),
                    ]),
                    accepted_count: 0,
                    rejected_count: 0,
                });
            }
        }

        // Add method completions for current object/class
        if let Some(current_object) = self.infer_current_object(analysis, context) {
            let methods = self
                .get_object_methods(&current_object, &context.language)
                .await?;
            for method in methods {
                if method.starts_with(&context.prefix) {
                    suggestions.push(CompletionSuggestion {
                        acceptance_rate: 0.0,
                        learning_data: CompletionLearningData {
                            user_patterns: HashMap::new(),
                            language_patterns: HashMap::new(),
                            acceptance_rates: HashMap::new(),
                        },
                        text: method,
                        kind: CompletionKind::Method,
                        confidence: 0.85,
                        context: context.clone(),
                        metadata: HashMap::from([
                            ("object".to_string(), current_object.clone()),
                            ("source".to_string(), "method".to_string()),
                        ]),
                        accepted_count: 0,
                        rejected_count: 0,
                    });
                }
            }
        }

        Ok(suggestions)
    }

    /// Generate context-aware completions
    async fn generate_context_completions(
        &self,
        _analysis: &CodeAnalysis,
        context: &CompletionContext,
    ) -> Result<Vec<CompletionSuggestion>, CompletionError> {
        let mut suggestions = Vec::new();

        // Check learning data for context patterns (simplified)
        let learning_data = self.learning_data.read().await;

        if let Some(patterns) = learning_data.language_patterns.get(&context.language) {
            for pattern in patterns {
                if context.prefix.contains(pattern) {
                    suggestions.push(CompletionSuggestion {
                        acceptance_rate: 0.0,
                        learning_data: CompletionLearningData {
                            user_patterns: HashMap::new(),
                            language_patterns: HashMap::new(),
                            acceptance_rates: HashMap::new(),
                        },
                        text: pattern.clone(),
                        kind: CompletionKind::Snippet,
                        confidence: 0.7,
                        context: context.clone(),
                        metadata: HashMap::from([
                            ("pattern".to_string(), pattern.clone()),
                            ("source".to_string(), "context".to_string()),
                        ]),
                        accepted_count: 0,
                        rejected_count: 0,
                    });
                }
            }
        }

        Ok(suggestions)
    }

    /// Generate pattern-based completions
    async fn generate_pattern_completions(
        &self,
        context: &CompletionContext,
    ) -> Result<Vec<CompletionSuggestion>, CompletionError> {
        let mut suggestions = Vec::new();

        let learning_data = self.learning_data.read().await;

        if let Some(user_patterns) = learning_data.user_patterns.get("default") {
            for suggestion in user_patterns {
                if suggestion.starts_with(&context.prefix) {
                    suggestions.push(CompletionSuggestion {
                        acceptance_rate: 0.0,
                        learning_data: CompletionLearningData {
                            user_patterns: HashMap::new(),
                            language_patterns: HashMap::new(),
                            acceptance_rates: HashMap::new(),
                        },
                        text: suggestion.clone(),
                        kind: CompletionKind::Snippet,
                        confidence: 0.7,
                        context: context.clone(),
                        metadata: HashMap::from([("source".to_string(), "pattern".to_string())]),
                        accepted_count: 0,
                        rejected_count: 0,
                    });
                }
            }
        }

        Ok(suggestions)
    }

    /// Generate keyword completions for the language
    fn generate_keyword_completions(
        &self,
        language: &str,
        prefix: &str,
    ) -> Result<Vec<CompletionSuggestion>, CompletionError> {
        let keywords = self.get_language_keywords(language);
        let mut suggestions = Vec::new();

        for keyword in keywords {
            if keyword.starts_with(prefix) && keyword != prefix {
                suggestions.push(CompletionSuggestion {
                    acceptance_rate: 0.0,
                    learning_data: CompletionLearningData {
                        user_patterns: HashMap::new(),
                        language_patterns: HashMap::new(),
                        acceptance_rates: HashMap::new(),
                    },
                    text: keyword.to_string(),
                    kind: CompletionKind::Keyword,
                    confidence: 0.9,
                    context: CompletionContext {
                        line: 0,
                        column: 0,
                        prefix: prefix.to_string(),
                        language: language.to_string(),
                        scope: vec![],
                        imports: vec![],
                        recent_symbols: vec![],
                    },
                    metadata: HashMap::from([("source".to_string(), "keyword".to_string())]),
                    accepted_count: 0,
                    rejected_count: 0,
                });
            }
        }

        Ok(suggestions)
    }

    /// Rank suggestions by relevance and confidence
    async fn rank_suggestions(
        &self,
        suggestions: Vec<CompletionSuggestion>,
        context: &CompletionContext,
    ) -> Result<Vec<CompletionSuggestion>, CompletionError> {
        let _learning_data = self.learning_data.read().await;

        let mut ranked = suggestions;
        ranked.sort_by(|a, b| {
            // Calculate composite score
            let a_score = self.calculate_suggestion_score(a, context, &_learning_data);
            let b_score = self.calculate_suggestion_score(b, context, &_learning_data);

            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(ranked)
    }

    /// Calculate score for a suggestion
    fn calculate_suggestion_score(
        &self,
        suggestion: &CompletionSuggestion,
        context: &CompletionContext,
        _learning_data: &CompletionLearningData,
    ) -> f64 {
        let mut score = suggestion.confidence;

        // Boost score based on acceptance history
        if suggestion.accepted_count > 0 {
            let acceptance_rate = suggestion.accepted_count as f64
                / (suggestion.accepted_count + suggestion.rejected_count) as f64;
            score += acceptance_rate * 0.2;
        }

        // Boost based on recent usage
        if context.recent_symbols.contains(&suggestion.text) {
            score += 0.1;
        }

        // Boost based on scope relevance
        if context.scope.contains(&suggestion.text) {
            score += 0.15;
        }

        score.min(1.0)
    }

    /// Filter and limit suggestions
    async fn filter_suggestions(
        &self,
        suggestions: Vec<CompletionSuggestion>,
        max_suggestions: usize,
    ) -> Result<Vec<CompletionSuggestion>, CompletionError> {
        // Simplified filtering - just limit the number of suggestions
        let filtered: Vec<_> = suggestions.into_iter().take(max_suggestions).collect();

        Ok(filtered)
    }

    /// Record suggestion acceptance for learning
    pub async fn record_acceptance(&self, suggestion: &CompletionSuggestion, accepted: bool) {
        let mut learning_data = self.learning_data.write().await;

        let key = format!("{}:{}", format!("{:?}", suggestion.kind), suggestion.text);

        // Update acceptance rate (simplified)
        let current_rate = learning_data
            .acceptance_rates
            .get(&key)
            .copied()
            .unwrap_or(0.0);
        let new_rate = if accepted {
            (current_rate + 1.0) / 2.0 // Simple moving average
        } else {
            current_rate * 0.9 // Decay for rejections
        };
        learning_data.acceptance_rates.insert(key, new_rate);

        // Update user patterns (simplified)
        if let Some(user_patterns) = learning_data.user_patterns.get_mut("default") {
            if accepted && !user_patterns.contains(&suggestion.text) {
                user_patterns.push(suggestion.text.clone());
                // Keep only recent preferences
                if user_patterns.len() > 100 {
                    user_patterns.remove(0);
                }
            }
        }
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> CompletionStats {
        self.performance_stats.read().await.clone()
    }

    /// Update performance statistics
    async fn update_performance_stats(&self, language: &str, duration: Duration, success: bool) {
        let mut stats = self.performance_stats.write().await;

        stats.total_requests += 1;

        // Update average response time
        let total_duration =
            stats.average_response_time * (stats.total_requests - 1) as u32 + duration;
        stats.average_response_time = total_duration / stats.total_requests as u32;

        // Update language-specific stats
        let lang_stats = stats
            .language_stats
            .entry(language.to_string())
            .or_default();
        lang_stats.requests += 1;
        if success {
            lang_stats.successful_completions += 1;
        }

        // Update acceptance rates from learning data
        let learning_data = self.learning_data.read().await;
        if let Some(acceptance_rate) = learning_data.acceptance_rates.values().next() {
            lang_stats.top_suggestion_accuracy = *acceptance_rate;
        }
    }

    // Helper methods
    fn map_symbol_kind(&self, symbol_kind: &str) -> CompletionKind {
        match symbol_kind {
            "function" => CompletionKind::Function,
            "method" => CompletionKind::Method,
            "variable" => CompletionKind::Variable,
            "class" => CompletionKind::Class,
            "struct" => CompletionKind::Struct,
            "enum" => CompletionKind::Enum,
            "trait" => CompletionKind::Trait,
            "module" => CompletionKind::Module,
            "type" => CompletionKind::Type,
            _ => CompletionKind::Variable,
        }
    }

    fn calculate_semantic_confidence(
        &self,
        symbol: &crate::tree_sitter::SymbolInfo,
        context: &CompletionContext,
    ) -> f64 {
        let mut confidence: f64 = 0.8;

        // Boost confidence for symbols in current scope
        if let Some(scope) = &symbol.scope {
            if context.scope.contains(scope) {
                confidence += 0.1;
            }
        }

        // Boost confidence for recently used symbols
        if context.recent_symbols.contains(&symbol.name) {
            confidence += 0.05;
        }

        confidence.min(1.0)
    }

    fn infer_current_object(
        &self,
        _analysis: &CodeAnalysis,
        _context: &CompletionContext,
    ) -> Option<String> {
        // For now, return None as we need to implement proper line access
        // This would require storing the source code lines in the CodeAnalysis struct
        None
    }

    async fn get_object_methods(
        &self,
        _object: &str,
        language: &str,
    ) -> Result<Vec<String>, CompletionError> {
        // This would integrate with language-specific analyzers
        // For now, return common methods
        match language {
            "rust" => Ok(vec![
                "clone()".to_string(),
                "as_ref()".to_string(),
                "unwrap()".to_string(),
                "expect()".to_string(),
                "len()".to_string(),
                "is_empty()".to_string(),
            ]),
            "python" => Ok(vec![
                "append()".to_string(),
                "extend()".to_string(),
                "insert()".to_string(),
                "remove()".to_string(),
                "pop()".to_string(),
                "len()".to_string(),
            ]),
            _ => Ok(vec![]),
        }
    }

    fn get_language_keywords(&self, language: &str) -> Vec<&'static str> {
        match language {
            "rust" => vec![
                "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait", "mod",
                "use", "pub", "crate", "super", "self", "Self", "async", "await", "move", "if",
                "else", "match", "loop", "while", "for", "in", "break", "continue", "return", "as",
                "dyn", "where", "unsafe",
            ],
            "python" => vec![
                "def", "class", "if", "elif", "else", "for", "while", "try", "except", "finally",
                "with", "as", "import", "from", "return", "yield", "lambda", "and", "or", "not",
                "in", "is", "None", "True", "False", "self", "super",
            ],
            "javascript" | "typescript" => vec![
                "function",
                "const",
                "let",
                "var",
                "if",
                "else",
                "for",
                "while",
                "try",
                "catch",
                "finally",
                "return",
                "async",
                "await",
                "class",
                "extends",
                "implements",
                "interface",
                "enum",
                "import",
                "export",
                "from",
                "as",
                "this",
                "super",
                "new",
                "typeof",
                "instanceof",
            ],
            _ => vec![],
        }
    }
}

/// Completion engine error
#[derive(Debug, thiserror::Error)]
pub enum CompletionError {
    #[error("Tree-sitter analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Invalid language: {0}")]
    InvalidLanguage(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Global completion engine instance
lazy_static::lazy_static! {
    pub static ref COMPLETION_ENGINE: CodeCompletionEngine = CodeCompletionEngine::new();
}
