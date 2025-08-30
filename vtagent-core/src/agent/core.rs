//! Core agent implementation and orchestration

use crate::conversation_summarizer::ConversationSummarizer;
use crate::decision_tracker::DecisionTracker;
use crate::error_recovery::{ErrorRecoveryManager, ErrorType};
use crate::llm::{make_client, AnyClient};
use crate::tools::{build_function_declarations, ToolRegistry};
use crate::tree_sitter::{CodeAnalysis, TreeSitterAnalyzer};
use crate::types::*;
use crate::agent::compaction::CompactionEngine;
use anyhow::{anyhow, Result};
use console::style;
use std::sync::Arc;

/// Main agent orchestrator
pub struct Agent {
    config: AgentConfig,
    client: AnyClient,
    tool_registry: Arc<ToolRegistry>,
    decision_tracker: DecisionTracker,
    error_recovery: ErrorRecoveryManager,
    summarizer: ConversationSummarizer,
    tree_sitter_analyzer: TreeSitterAnalyzer,
    compaction_engine: Arc<CompactionEngine>,
    session_info: SessionInfo,
    start_time: std::time::Instant,
}

impl Agent {
    /// Create a new agent instance
    pub fn new(config: AgentConfig) -> Result<Self> {
        let client = make_client(config.api_key.clone(), config.model.clone());
        let tool_registry = Arc::new(ToolRegistry::new(config.workspace.clone()));
        let decision_tracker = DecisionTracker::new();
        let error_recovery = ErrorRecoveryManager::new();
        let summarizer = ConversationSummarizer::new();
        let tree_sitter_analyzer = TreeSitterAnalyzer::new()
            .map_err(|e| {
                eprintln!("Warning: Failed to initialize tree-sitter analyzer: {}", e);
                e
            })
            .unwrap_or_else(|_| {
                // Create a fallback analyzer that gracefully handles errors
                TreeSitterAnalyzer::new().unwrap_or_else(|_| {
                    panic!("Critical: Could not initialize tree-sitter analyzer")
                })
            });

        let session_id = format!(
            "session_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        let session_info = SessionInfo {
            session_id,
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            total_turns: 0,
            total_decisions: 0,
            error_count: 0,
        };

        Ok(Self {
            config,
            client,
            tool_registry,
            decision_tracker,
            error_recovery,
            summarizer,
            tree_sitter_analyzer,
            compaction_engine: Arc::new(CompactionEngine::new()),
            session_info,
            start_time: std::time::Instant::now(),
        })
    }

    /// Initialize the agent with system setup
    pub async fn initialize(&mut self) -> Result<()> {
        // Initialize available tools in decision tracker
        let tool_names = build_function_declarations()
            .iter()
            .map(|fd| fd.name.clone())
            .collect::<Vec<_>>();
        self.decision_tracker.update_available_tools(tool_names);

        // Update session info
        self.session_info.start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if self.config.verbose {
            println!("{} {}", style("[INIT]").cyan().bold(), "Agent initialized");
            println!("  {} Model: {}", style("").dim(), self.config.model);
            println!(
                "  {} Workspace: {}",
                style("").dim(),
                self.config.workspace.display()
            );
            println!(
                "  {} Tools loaded: {}",
                style("").dim(),
                build_function_declarations().len()
            );
            println!(
                "  {} Session ID: {}",
                style("🆔").dim(),
                self.session_info.session_id
            );
            println!();
        }

        Ok(())
    }

    /// Get the agent's current configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Get session information
    pub fn session_info(&self) -> &SessionInfo {
        &self.session_info
    }

    /// Get performance metrics
    pub fn performance_metrics(&self) -> PerformanceMetrics {
        let duration = self.start_time.elapsed();

        PerformanceMetrics {
            session_duration_seconds: duration.as_secs(),
            total_api_calls: self.session_info.total_turns,
            total_tokens_used: None, // Would need to track from API responses
            average_response_time_ms: if self.session_info.total_turns > 0 {
                duration.as_millis() as f64 / self.session_info.total_turns as f64
            } else {
                0.0
            },
            tool_execution_count: self.session_info.total_decisions,
            error_count: self.session_info.error_count,
            recovery_success_rate: self.calculate_recovery_rate(),
        }
    }

    /// Get decision tracker reference
    pub fn decision_tracker(&self) -> &DecisionTracker {
        &self.decision_tracker
    }

    /// Get mutable decision tracker reference
    pub fn decision_tracker_mut(&mut self) -> &mut DecisionTracker {
        &mut self.decision_tracker
    }

    /// Get error recovery manager reference
    pub fn error_recovery(&self) -> &ErrorRecoveryManager {
        &self.error_recovery
    }

    /// Get mutable error recovery manager reference
    pub fn error_recovery_mut(&mut self) -> &mut ErrorRecoveryManager {
        &mut self.error_recovery
    }

    /// Get conversation summarizer reference
    pub fn summarizer(&self) -> &ConversationSummarizer {
        &self.summarizer
    }

    /// Get tool registry reference
    pub fn tool_registry(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.tool_registry)
    }

    /// Get mutable tool registry reference
    pub fn tool_registry_mut(&mut self) -> &mut ToolRegistry {
        Arc::get_mut(&mut self.tool_registry)
            .expect("ToolRegistry should not have other references")
    }

    /// Get model-agnostic client reference
    pub fn llm(&self) -> &AnyClient { &self.client }

    /// Get tree-sitter analyzer reference
    pub fn tree_sitter_analyzer(&self) -> &TreeSitterAnalyzer {
        &self.tree_sitter_analyzer
    }

    /// Get mutable tree-sitter analyzer reference
    pub fn tree_sitter_analyzer_mut(&mut self) -> &mut TreeSitterAnalyzer {
        &mut self.tree_sitter_analyzer
    }

    /// Get compaction engine reference
    pub fn compaction_engine(&self) -> Arc<CompactionEngine> {
        Arc::clone(&self.compaction_engine)
    }

    /// Make intelligent compaction decision using context analysis
    pub async fn make_intelligent_compaction_decision(&self) -> Result<crate::agent::intelligence::CompactionDecision> {
        // Minimal implementation - return a simple decision since the compaction engine is minimal
        Ok(crate::agent::intelligence::CompactionDecision {
            should_compact: false,
            strategy: crate::agent::intelligence::CompactionStrategy::Conservative,
            reasoning: "Minimal implementation - no compaction needed".to_string(),
            estimated_benefit: 0,
        })
    }

    /// Check if compaction is needed
    pub async fn should_compact(&self) -> Result<bool> {
        Ok(false) // Minimal implementation
    }

    /// Perform intelligent message compaction
    pub async fn compact_messages(&self) -> Result<crate::agent::compaction::CompactionResult> {
        // Minimal implementation
        Ok(crate::agent::compaction::CompactionResult {
            messages_processed: 0,
            messages_compacted: 0,
            original_size: 0,
            compacted_size: 0,
            compression_ratio: 1.0,
            processing_time_ms: 0,
        })
    }

    /// Perform context compaction
    pub async fn compact_context(&self, _context_key: &str, _context_data: &mut std::collections::HashMap<String, serde_json::Value>) -> Result<crate::agent::compaction::CompactionResult> {
        // Minimal implementation
        Ok(crate::agent::compaction::CompactionResult {
            messages_processed: 0,
            messages_compacted: 0,
            original_size: 0,
            compacted_size: 0,
            compression_ratio: 1.0,
            processing_time_ms: 0,
        })
    }

    /// Get compaction statistics
    pub async fn get_compaction_stats(&self) -> Result<crate::agent::compaction::CompactionStatistics> {
        // Minimal implementation
        Ok(crate::agent::compaction::CompactionStatistics {
            total_messages: 0,
            messages_by_priority: std::collections::HashMap::new(),
            total_memory_usage: 0,
            average_message_size: 0,
            last_compaction_timestamp: 0,
            compaction_frequency: 0.0,
        })
    }

    /// Analyze a file using tree-sitter
    pub fn analyze_file_with_tree_sitter(
        &mut self,
        file_path: &std::path::Path,
        source_code: &str,
    ) -> Result<CodeAnalysis> {
        // Detect language from file extension
        let language = self
            .tree_sitter_analyzer
            .detect_language_from_path(file_path)
            .map_err(|e| {
                anyhow!(
                    "Failed to detect language for {}: {}",
                    file_path.display(),
                    e
                )
            })?;

        // Parse the file
        let syntax_tree = self
            .tree_sitter_analyzer
            .parse(source_code, language.clone())?;

        // Extract symbols
        let symbols = self
            .tree_sitter_analyzer
            .extract_symbols(&syntax_tree, source_code, language.clone())
            .unwrap_or_default();

        // Extract dependencies
        let dependencies = self
            .tree_sitter_analyzer
            .extract_dependencies(&syntax_tree, language.clone())
            .unwrap_or_default();

        // Calculate metrics
        let metrics = self
            .tree_sitter_analyzer
            .calculate_metrics(&syntax_tree, source_code)
            .unwrap_or_default();

        Ok(CodeAnalysis {
            file_path: file_path.to_string_lossy().to_string(),
            language,
            symbols,
            dependencies,
            metrics,
            issues: Vec::new(),
            complexity: crate::tree_sitter::analysis::ComplexityMetrics::default(),
            structure: crate::tree_sitter::analysis::CodeStructure::default(),
        })
    }

    /// Update session statistics
    pub fn update_session_stats(&mut self, turns: usize, decisions: usize, errors: usize) {
        self.session_info.total_turns = turns;
        self.session_info.total_decisions = decisions;
        self.session_info.error_count = errors;
    }

    /// Check if context compression is needed
    pub fn should_compress_context(&self, context_size: usize) -> bool {
        self.error_recovery.should_compress_context(context_size)
    }

    /// Generate context preservation plan
    pub fn generate_context_plan(
        &self,
        context_size: usize,
    ) -> crate::error_recovery::ContextPreservationPlan {
        self.error_recovery
            .generate_context_preservation_plan(context_size, self.session_info.error_count)
    }

    /// Check for error patterns
    pub fn detect_error_pattern(&self, error_type: &ErrorType, time_window_seconds: u64) -> bool {
        self.error_recovery
            .detect_error_pattern(error_type, time_window_seconds)
    }

    /// Calculate recovery success rate
    fn calculate_recovery_rate(&self) -> f64 {
        let stats = self.error_recovery.get_error_statistics();
        if stats.total_errors > 0 {
            stats.resolved_errors as f64 / stats.total_errors as f64
        } else {
            1.0 // Perfect rate if no errors
        }
    }

    /// Show transparency report
    pub fn show_transparency_report(&self, detailed: bool) {
        let report = self.decision_tracker.generate_transparency_report();
        let error_stats = self.error_recovery.get_error_statistics();

        if detailed && self.config.verbose {
            println!(
                "{} {}",
                style("[TRANSPARENCY]").magenta().bold(),
                "Session Transparency Summary:"
            );
            println!(
                "  {} total decisions made",
                style(report.total_decisions).cyan()
            );
            println!(
                "  {} successful ({}% success rate)",
                style(report.successful_decisions).green(),
                if report.total_decisions > 0 {
                    (report.successful_decisions * 100) / report.total_decisions
                } else {
                    0
                }
            );
            println!(
                "  {} failed decisions",
                style(report.failed_decisions).red()
            );
            println!("  {} tool calls executed", style(report.tool_calls).blue());
            println!(
                "  Session duration: {} seconds",
                style(report.session_duration).yellow()
            );
            if let Some(avg_confidence) = report.avg_confidence {
                println!(
                    "  {:.1}% average decision confidence",
                    avg_confidence * 100.0
                );
            }

            // Error recovery statistics
            println!(
                "\n{} {}",
                style("[ERROR RECOVERY]").red().bold(),
                "Error Statistics:"
            );
            println!(
                "  {} total errors occurred",
                style(error_stats.total_errors).red()
            );
            println!(
                "  {} errors resolved ({}% recovery rate)",
                style(error_stats.resolved_errors).green(),
                if error_stats.total_errors > 0 {
                    (error_stats.resolved_errors * 100) / error_stats.total_errors
                } else {
                    0
                }
            );
            println!(
                "  {:.1} average recovery attempts per error",
                style(error_stats.avg_recovery_attempts).yellow()
            );

            // Conversation summarization statistics
            let summaries = self.summarizer.get_summaries();
            if !summaries.is_empty() {
                println!(
                    "\n{} {}",
                    style("[CONVERSATION SUMMARY]").green().bold(),
                    "Statistics:"
                );
                println!("  {} summaries generated", style(summaries.len()).cyan());
                if let Some(latest) = self.summarizer.get_latest_summary() {
                    println!(
                        "  {} Latest summary: {} turns, {:.1}% compression",
                        style("📝").dim(),
                        latest.total_turns,
                        latest.compression_ratio * 100.0
                    );
                }
            }
        } else {
            // Brief summary for non-verbose mode
            println!("{}", style(format!("  ↳ Session complete: {} decisions, {} successful ({}% success rate), {} errors",
                         report.total_decisions, report.successful_decisions,
                         if report.total_decisions > 0 { (report.successful_decisions * 100) / report.total_decisions } else { 0 },
                         error_stats.total_errors)).dim());
        }
    }

    /// Shutdown the agent and cleanup resources
    pub async fn shutdown(&mut self) -> Result<()> {
        // Show final transparency report
        self.show_transparency_report(true);

        if self.config.verbose {
            println!(
                "{} {}",
                style("[SHUTDOWN]").cyan().bold(),
                "Agent shutdown complete"
            );
        }

        Ok(())
    }
}

/// Builder pattern for creating agents with custom configuration
pub struct AgentBuilder {
    config: AgentConfig,
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                model: "gemini-2.5-flash-lite".to_string(),
                api_key: String::new(),
                workspace: std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from(".")),
                verbose: false,
            },
        }
    }

    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.config.api_key = api_key.into();
        self
    }

    pub fn with_workspace<P: Into<std::path::PathBuf>>(mut self, workspace: P) -> Self {
        self.config.workspace = workspace.into();
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.config.verbose = verbose;
        self
    }

    pub fn build(self) -> Result<Agent> {
        Agent::new(self.config)
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}
