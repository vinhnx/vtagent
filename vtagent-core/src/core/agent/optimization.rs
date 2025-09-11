//! Performance optimization for multi-agent systems
//!
//! This module provides performance monitoring, optimization strategies, and
//! resource management for the multi-agent architecture.

use crate::core::agent::multi_agent::AgentType;
use crate::config::models::ModelId;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// Performance metrics for agents and tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average task completion time by agent type
    pub avg_completion_time: HashMap<AgentType, Duration>,
    /// Success rate by agent type
    pub success_rate: HashMap<AgentType, f64>,
    /// Throughput (tasks per minute) by agent type
    pub throughput: HashMap<AgentType, f64>,
    /// Model performance statistics
    pub model_performance: HashMap<String, ModelPerformance>,
    /// Resource utilization metrics
    pub resource_utilization: ResourceMetrics,
    /// Error rate statistics
    pub error_rates: HashMap<AgentType, f64>,
    /// Queue statistics
    pub queue_stats: QueueStatistics,
}

/// Performance metrics for specific models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    /// Model identifier
    pub model_id: String,
    /// Average response time
    pub avg_response_time: Duration,
    /// Success rate
    pub success_rate: f64,
    /// Token usage statistics
    pub token_stats: TokenStatistics,
    /// Cost metrics
    pub cost_metrics: CostMetrics,
    /// Quality scores
    pub quality_scores: QualityMetrics,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStatistics {
    /// Average input tokens
    pub avg_input_tokens: f64,
    /// Average output tokens
    pub avg_output_tokens: f64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Peak token usage
    pub peak_tokens: u64,
}

/// Cost tracking metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostMetrics {
    /// Total estimated cost
    pub total_cost: f64,
    /// Cost per task
    pub cost_per_task: f64,
    /// Cost per token
    pub cost_per_token: f64,
    /// Daily cost
    pub daily_cost: f64,
}

/// Quality assessment metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Average confidence score
    pub avg_confidence: f64,
    /// Average completeness score
    pub avg_completeness: f64,
    /// Revision rate
    pub revision_rate: f64,
    /// User satisfaction score
    pub satisfaction_score: f64,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// Memory usage statistics
    pub memory_usage: MemoryUsage,
    /// CPU utilization
    pub cpu_utilization: f64,
    /// Network I/O statistics
    pub network_io: NetworkStats,
    /// Concurrent agent count
    pub concurrent_agents: usize,
}

/// Memory usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Current memory usage in MB
    pub current_mb: f64,
    /// Peak memory usage in MB
    pub peak_mb: f64,
    /// Average memory usage in MB
    pub avg_mb: f64,
    /// Memory efficiency score
    pub efficiency_score: f64,
}

/// Network I/O statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Average latency
    pub avg_latency: Duration,
    /// Peak latency
    pub peak_latency: Duration,
}

/// Queue performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatistics {
    /// Average queue length
    pub avg_queue_length: f64,
    /// Peak queue length
    pub peak_queue_length: usize,
    /// Average wait time
    pub avg_wait_time: Duration,
    /// Queue processing rate
    pub processing_rate: f64,
}

/// Performance optimization strategies
#[derive(Debug, Clone)]
pub struct OptimizationStrategy {
    /// Strategy identifier
    pub id: String,
    /// Strategy description
    pub description: String,
    /// Strategy type
    pub strategy_type: StrategyType,
    /// Expected improvement
    pub expected_improvement: f64,
    /// Implementation complexity
    pub complexity: OptimizationComplexity,
}

/// Types of optimization strategies
#[derive(Debug, Clone)]
pub enum StrategyType {
    /// Load balancing optimization
    LoadBalancing,
    /// Model selection optimization
    ModelSelection,
    /// Caching optimization
    Caching,
    /// Parallel processing optimization
    Parallelization,
    /// Resource allocation optimization
    ResourceAllocation,
    /// Queue management optimization
    QueueManagement,
}

/// Optimization complexity levels
#[derive(Debug, Clone)]
pub enum OptimizationComplexity {
    Low,
    Medium,
    High,
    Critical,
}

/// Performance monitor for multi-agent systems
pub struct PerformanceMonitor {
    /// Performance metrics storage
    metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Task execution history
    task_history: Arc<RwLock<VecDeque<TaskExecutionRecord>>>,
    /// Model performance history
    model_history: Arc<RwLock<VecDeque<ModelExecutionRecord>>>,
    /// Configuration
    config: PerformanceConfig,
    /// Optimization strategies
    strategies: Vec<OptimizationStrategy>,
    /// Start time for uptime tracking
    start_time: Instant,
}

/// Configuration for performance monitoring
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Maximum history size
    pub max_history_size: usize,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Enable detailed tracking
    pub enable_detailed_tracking: bool,
    /// Performance alert thresholds
    pub alert_thresholds: AlertThresholds,
}

/// Alert threshold configuration
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// Maximum acceptable response time
    pub max_response_time: Duration,
    /// Minimum acceptable success rate
    pub min_success_rate: f64,
    /// Maximum acceptable error rate
    pub max_error_rate: f64,
    /// Maximum memory usage threshold
    pub max_memory_mb: f64,
}

/// Task execution record for performance tracking
#[derive(Debug, Clone)]
pub struct TaskExecutionRecord {
    /// Task identifier
    pub task_id: String,
    /// Agent type that executed the task
    pub agent_type: AgentType,
    /// Execution start time
    pub start_time: Instant,
    /// Execution duration
    pub duration: Duration,
    /// Success status
    pub success: bool,
    /// Model used
    pub model: String,
    /// Input token count
    pub input_tokens: u32,
    /// Output token count
    pub output_tokens: u32,
    /// Quality score
    pub quality_score: f64,
}

/// Model execution record for performance tracking
#[derive(Debug, Clone)]
pub struct ModelExecutionRecord {
    /// Model identifier
    pub model_id: String,
    /// Execution timestamp
    pub timestamp: SystemTime,
    /// Response time
    pub response_time: Duration,
    /// Success status
    pub success: bool,
    /// Token usage
    pub tokens_used: u32,
    /// Quality metrics
    pub quality: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_history_size: 10000,
            collection_interval: Duration::from_secs(60),
            enable_detailed_tracking: true,
            alert_thresholds: AlertThresholds {
                max_response_time: Duration::from_secs(30),
                min_success_rate: 0.95,
                max_error_rate: 0.05,
                max_memory_mb: 1000.0,
            },
        }
    }
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(config: PerformanceConfig) -> Self {
        let metrics = Arc::new(RwLock::new(PerformanceMetrics {
            avg_completion_time: HashMap::new(),
            success_rate: HashMap::new(),
            throughput: HashMap::new(),
            model_performance: HashMap::new(),
            resource_utilization: ResourceMetrics {
                memory_usage: MemoryUsage {
                    current_mb: 0.0,
                    peak_mb: 0.0,
                    avg_mb: 0.0,
                    efficiency_score: 1.0,
                },
                cpu_utilization: 0.0,
                network_io: NetworkStats {
                    bytes_sent: 0,
                    bytes_received: 0,
                    avg_latency: Duration::from_millis(0),
                    peak_latency: Duration::from_millis(0),
                },
                concurrent_agents: 0,
            },
            error_rates: HashMap::new(),
            queue_stats: QueueStatistics {
                avg_queue_length: 0.0,
                peak_queue_length: 0,
                avg_wait_time: Duration::from_millis(0),
                processing_rate: 0.0,
            },
        }));

        let strategies = vec![
            OptimizationStrategy {
                id: "smart_model_selection".to_string(),
                description: "Automatically select the best model based on task complexity"
                    .to_string(),
                strategy_type: StrategyType::ModelSelection,
                expected_improvement: 0.25,
                complexity: OptimizationComplexity::Medium,
            },
            OptimizationStrategy {
                id: "adaptive_load_balancing".to_string(),
                description: "Dynamically balance load across available agents".to_string(),
                strategy_type: StrategyType::LoadBalancing,
                expected_improvement: 0.30,
                complexity: OptimizationComplexity::High,
            },
            OptimizationStrategy {
                id: "intelligent_caching".to_string(),
                description: "Cache frequently used results and context".to_string(),
                strategy_type: StrategyType::Caching,
                expected_improvement: 0.40,
                complexity: OptimizationComplexity::Medium,
            },
            OptimizationStrategy {
                id: "parallel_task_execution".to_string(),
                description: "Execute independent tasks in parallel".to_string(),
                strategy_type: StrategyType::Parallelization,
                expected_improvement: 0.50,
                complexity: OptimizationComplexity::High,
            },
        ];

        Self {
            metrics,
            task_history: Arc::new(RwLock::new(VecDeque::new())),
            model_history: Arc::new(RwLock::new(VecDeque::new())),
            config,
            strategies,
            start_time: Instant::now(),
        }
    }

    /// Record task execution metrics
    pub async fn record_task_execution(
        &self,
        task_id: String,
        agent_type: AgentType,
        start_time: Instant,
        duration: Duration,
        success: bool,
        model: String,
        input_tokens: u32,
        output_tokens: u32,
        quality_score: f64,
    ) -> Result<()> {
        let record = TaskExecutionRecord {
            task_id,
            agent_type,
            start_time,
            duration,
            success,
            model: model.clone(),
            input_tokens,
            output_tokens,
            quality_score,
        };

        // Add to history
        let mut history = self.task_history.write().await;
        history.push_back(record);

        // Limit history size
        while history.len() > self.config.max_history_size {
            history.pop_front();
        }
        drop(history);

        // Update metrics
        self.update_performance_metrics(agent_type, duration, success, &model, quality_score)
            .await;

        Ok(())
    }

    /// Update performance metrics based on execution record
    async fn update_performance_metrics(
        &self,
        agent_type: AgentType,
        duration: Duration,
        success: bool,
        model: &str,
        quality_score: f64,
    ) {
        let mut metrics = self.metrics.write().await;

        // Update completion time
        let avg_time = metrics
            .avg_completion_time
            .entry(agent_type)
            .or_insert(Duration::from_secs(0));
        *avg_time = Duration::from_millis(
            (avg_time.as_millis() as f64 * 0.9 + duration.as_millis() as f64 * 0.1) as u64,
        );

        // Update success rate
        let success_rate = metrics.success_rate.entry(agent_type).or_insert(1.0);
        *success_rate = *success_rate * 0.9 + (if success { 1.0 } else { 0.0 }) * 0.1;

        // Update model performance
        let model_perf = metrics
            .model_performance
            .entry(model.to_string())
            .or_insert(ModelPerformance {
                model_id: model.to_string(),
                avg_response_time: Duration::from_secs(0),
                success_rate: 1.0,
                token_stats: TokenStatistics {
                    avg_input_tokens: 0.0,
                    avg_output_tokens: 0.0,
                    total_tokens: 0,
                    peak_tokens: 0,
                },
                cost_metrics: CostMetrics {
                    total_cost: 0.0,
                    cost_per_task: 0.0,
                    cost_per_token: 0.0001,
                    daily_cost: 0.0,
                },
                quality_scores: QualityMetrics {
                    avg_confidence: 0.0,
                    avg_completeness: 0.0,
                    revision_rate: 0.0,
                    satisfaction_score: 0.0,
                },
            });

        model_perf.avg_response_time = Duration::from_millis(
            (model_perf.avg_response_time.as_millis() as f64 * 0.9
                + duration.as_millis() as f64 * 0.1) as u64,
        );
        model_perf.success_rate =
            model_perf.success_rate * 0.9 + (if success { 1.0 } else { 0.0 }) * 0.1;
        model_perf.quality_scores.avg_confidence =
            model_perf.quality_scores.avg_confidence * 0.9 + quality_score * 0.1;
    }

    /// Get current performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Get performance recommendations
    pub async fn get_recommendations(&self) -> Vec<OptimizationStrategy> {
        let metrics = self.metrics.read().await;
        let mut recommendations = vec![];

        // Analyze performance and suggest optimizations
        for (agent_type, success_rate) in &metrics.success_rate {
            if *success_rate < 0.9 {
                recommendations.push(OptimizationStrategy {
                    id: format!("improve_{:?}_reliability", agent_type),
                    description: format!(
                        "Improve reliability for {:?} agents (current: {:.2}%)",
                        agent_type,
                        success_rate * 100.0
                    ),
                    strategy_type: StrategyType::ModelSelection,
                    expected_improvement: 0.1,
                    complexity: OptimizationComplexity::Medium,
                });
            }
        }

        for (agent_type, completion_time) in &metrics.avg_completion_time {
            if completion_time > &Duration::from_secs(30) {
                recommendations.push(OptimizationStrategy {
                    id: format!("optimize_{:?}_speed", agent_type),
                    description: format!(
                        "Optimize speed for {:?} agents (current: {:?})",
                        agent_type, completion_time
                    ),
                    strategy_type: StrategyType::Parallelization,
                    expected_improvement: 0.3,
                    complexity: OptimizationComplexity::High,
                });
            }
        }

        recommendations
    }

    /// Analyze model performance and suggest optimal models
    pub async fn suggest_optimal_models(&self, _agent_type: AgentType) -> Vec<ModelId> {
        let metrics = self.metrics.read().await;
        let mut model_scores: Vec<(String, f64)> = vec![];

        for (model_id, perf) in &metrics.model_performance {
            let score = perf.success_rate * 0.4
                + (1.0 / perf.avg_response_time.as_secs_f64().max(0.1)) * 0.3
                + perf.quality_scores.avg_confidence * 0.3;
            model_scores.push((model_id.clone(), score));
        }

        // Sort by score descending
        model_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Convert to ModelId, taking top 3
        model_scores
            .into_iter()
            .take(3)
            .filter_map(|(model_id, _)| model_id.parse::<ModelId>().ok())
            .collect()
    }

    /// Get system uptime
    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Generate performance report
    pub async fn generate_report(&self) -> PerformanceReport {
        let metrics = self.metrics.read().await;
        let recommendations = self.get_recommendations().await;

        PerformanceReport {
            timestamp: SystemTime::now(),
            uptime: self.get_uptime(),
            metrics: metrics.clone(),
            recommendations,
            summary: self.generate_summary(&metrics).await,
        }
    }

    /// Generate performance summary
    async fn generate_summary(&self, metrics: &PerformanceMetrics) -> PerformanceSummary {
        let history = self.task_history.read().await;
        let total_tasks = history.len();
        let successful_tasks = history.iter().filter(|r| r.success).count();

        PerformanceSummary {
            total_tasks,
            successful_tasks,
            overall_success_rate: if total_tasks > 0 {
                successful_tasks as f64 / total_tasks as f64
            } else {
                0.0
            },
            avg_response_time: if total_tasks > 0 {
                Duration::from_millis(
                    history.iter().map(|r| r.duration.as_millis()).sum::<u128>() as u64
                        / total_tasks as u64,
                )
            } else {
                Duration::from_secs(0)
            },
            peak_performance_agent: self.find_best_performing_agent(metrics),
            bottleneck_analysis: self.analyze_bottlenecks(metrics).await,
        }
    }

    /// Find the best performing agent type
    fn find_best_performing_agent(&self, metrics: &PerformanceMetrics) -> Option<AgentType> {
        let mut best_agent = None;
        let mut best_score = 0.0;

        for (agent_type, success_rate) in &metrics.success_rate {
            if let Some(completion_time) = metrics.avg_completion_time.get(agent_type) {
                let score =
                    *success_rate * 0.7 + (1.0 / completion_time.as_secs_f64().max(0.1)) * 0.3;
                if score > best_score {
                    best_score = score;
                    best_agent = Some(*agent_type);
                }
            }
        }

        best_agent
    }

    /// Analyze system bottlenecks
    async fn analyze_bottlenecks(&self, _metrics: &PerformanceMetrics) -> Vec<String> {
        let mut bottlenecks = vec![];

        // Analyze queue length
        let history = self.task_history.read().await;
        if history.len() > 100 {
            bottlenecks.push("High task queue length detected".to_string());
        }

        // Analyze response times
        if let Some(latest) = history.back() {
            if latest.duration > Duration::from_secs(20) {
                bottlenecks.push("High response times detected".to_string());
            }
        }

        bottlenecks
    }
}

/// Performance report structure
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub timestamp: SystemTime,
    pub uptime: Duration,
    pub metrics: PerformanceMetrics,
    pub recommendations: Vec<OptimizationStrategy>,
    pub summary: PerformanceSummary,
}

/// Performance summary
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub overall_success_rate: f64,
    pub avg_response_time: Duration,
    pub peak_performance_agent: Option<AgentType>,
    pub bottleneck_analysis: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new(PerformanceConfig::default());

        monitor
            .record_task_execution(
                "test_task".to_string(),
                AgentType::Coder,
                Instant::now(),
                Duration::from_secs(5),
                true,
                crate::config::constants::models::GEMINI_2_5_FLASH.to_string(),
                100,
                200,
                0.9,
            )
            .await
            .unwrap();

        let metrics = monitor.get_metrics().await;
        assert!(metrics.success_rate.contains_key(&AgentType::Coder));
    }

    #[test]
    fn test_optimization_strategies() {
        let monitor = PerformanceMonitor::new(PerformanceConfig::default());
        assert!(!monitor.strategies.is_empty());
        assert!(
            monitor
                .strategies
                .iter()
                .any(|s| matches!(s.strategy_type, StrategyType::ModelSelection))
        );
    }
}
