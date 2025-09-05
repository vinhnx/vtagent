//! Performance optimization and caching systems for the coding agent
//!
//! This module implements Research-preview performance features including:
//! - Intelligent caching with LRU eviction
//! - Parallel processing for large codebases
//! - Memory-efficient data structures
//! - Response time optimization

use crate::tree_sitter::CodeAnalysis;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Intelligent caching system with LRU eviction
pub struct IntelligentCache<T> {
    cache: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    access_order: Arc<RwLock<VecDeque<String>>>,
    max_size: usize,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    timestamp: Instant,
    access_count: usize,
    size_estimate: usize,
}

impl<T> IntelligentCache<T> {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            ttl,
        }
    }

    pub async fn get(&self, key: &str) -> Option<T>
    where
        T: Clone,
    {
        let mut cache = self.cache.write().await;
        let mut access_order = self.access_order.write().await;

        if let Some(entry) = cache.get_mut(key) {
            // Check if entry has expired
            if entry.timestamp.elapsed() > self.ttl {
                cache.remove(key);
                access_order.retain(|k| k != key);
                return None;
            }

            // Update access statistics
            entry.access_count += 1;

            // Move to front of access order
            access_order.retain(|k| k != key);
            access_order.push_front(key.to_string());

            Some(entry.data.clone())
        } else {
            None
        }
    }

    pub async fn put(&self, key: String, value: T, size_estimate: usize) {
        let mut cache = self.cache.write().await;
        let mut access_order = self.access_order.write().await;

        // Remove existing entry if present
        if cache.contains_key(&key) {
            cache.remove(&key);
            access_order.retain(|k| k != &key);
        }

        // Evict entries if cache is full
        while cache.len() >= self.max_size {
            if let Some(evict_key) = access_order.pop_back() {
                cache.remove(&evict_key);
            }
        }

        // Add new entry
        let entry = CacheEntry {
            data: value,
            timestamp: Instant::now(),
            access_count: 1,
            size_estimate,
        };

        cache.insert(key.clone(), entry);
        access_order.push_front(key);
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        let mut access_order = self.access_order.write().await;
        cache.clear();
        access_order.clear();
    }

    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let _access_order = self.access_order.read().await;

        let total_entries = cache.len();
        let total_accesses: usize = cache.values().map(|e| e.access_count).sum();
        let total_size: usize = cache.values().map(|e| e.size_estimate).sum();
        let avg_access_count = if total_entries > 0 {
            total_accesses as f64 / total_entries as f64
        } else {
            0.0
        };

        CacheStats {
            total_entries,
            total_accesses,
            total_size_bytes: total_size,
            avg_access_count,
            hit_rate: 0.0, // Would need to track hits/misses separately
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_accesses: usize,
    pub total_size_bytes: usize,
    pub avg_access_count: f64,
    pub hit_rate: f64,
}

/// Parallel processing engine for large codebases
pub struct ParallelProcessor {
    max_concurrent_tasks: usize,
}

impl ParallelProcessor {
    pub fn new(max_concurrent_tasks: usize) -> Self {
        Self {
            max_concurrent_tasks,
        }
    }

    /// Process multiple files in parallel
    pub async fn process_files<F, Fut, T>(
        &self,
        files: Vec<PathBuf>,
        processor: F,
    ) -> Result<Vec<T>>
    where
        F: Fn(PathBuf) -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        use futures::stream::{self, StreamExt};

        let results: Vec<Result<T>> = stream::iter(files)
            .map(|file| {
                let processor = processor.clone();
                async move { processor(file).await }
            })
            .buffer_unordered(self.max_concurrent_tasks)
            .collect()
            .await;

        // Collect successful results, propagating first error
        let mut successful_results = Vec::new();
        for result in results {
            successful_results.push(result?);
        }

        Ok(successful_results)
    }

    /// Process files with priority-based scheduling
    pub async fn process_with_priority<F, Fut, T, P>(
        &self,
        files_with_priority: Vec<(PathBuf, P)>,
        processor: F,
    ) -> Result<Vec<T>>
    where
        F: Fn(PathBuf) -> Fut + Send + Sync + Clone,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send,
        P: Ord + Send,
    {
        // Sort by priority (highest first)
        let mut sorted_files: Vec<_> = files_with_priority;
        sorted_files.sort_by(|a, b| b.1.cmp(&a.1));

        let files: Vec<PathBuf> = sorted_files.into_iter().map(|(file, _)| file).collect();

        self.process_files(files, processor).await
    }
}

/// Memory-efficient code analysis storage
pub struct MemoryEfficientStorage {
    analyses: Arc<RwLock<HashMap<String, CompressedAnalysis>>>,
    max_memory_mb: usize,
    current_memory_usage: Arc<RwLock<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompressedAnalysis {
    symbols: Vec<u8>,      // Compressed symbol data
    dependencies: Vec<u8>, // Compressed dependency data
    metrics: Vec<u8>,      // Compressed metrics data
    original_size: usize,
    compressed_size: usize,
}

impl MemoryEfficientStorage {
    pub fn new(max_memory_mb: usize) -> Self {
        Self {
            analyses: Arc::new(RwLock::new(HashMap::new())),
            max_memory_mb: max_memory_mb * 1024 * 1024, // Convert to bytes
            current_memory_usage: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn store_analysis(&self, file_path: &Path, analysis: CodeAnalysis) -> Result<()> {
        let compressed = self.compress_analysis(analysis).await?;
        let key = file_path.to_string_lossy().to_string();

        let mut analyses = self.analyses.write().await;
        let mut memory_usage = self.current_memory_usage.write().await;

        // Remove old entry if exists
        if let Some(old_entry) = analyses.remove(&key) {
            *memory_usage = memory_usage.saturating_sub(old_entry.compressed_size);
        }

        // Evict entries if needed
        while *memory_usage + compressed.compressed_size > self.max_memory_mb {
            if let Some((_, entry)) = analyses.iter().next() {
                let entry_size = entry.compressed_size;
                analyses.retain(|_, e| e.compressed_size != entry_size);
                *memory_usage = memory_usage.saturating_sub(entry_size);
                break;
            }
        }

        analyses.insert(key, compressed.clone());
        *memory_usage += compressed.compressed_size;

        Ok(())
    }

    pub async fn get_analysis(&self, file_path: &Path) -> Result<Option<CodeAnalysis>> {
        let analyses = self.analyses.read().await;
        let key = file_path.to_string_lossy().to_string();

        if let Some(compressed) = analyses.get(&key) {
            let analysis = self.decompress_analysis(compressed).await?;
            Ok(Some(analysis))
        } else {
            Ok(None)
        }
    }

    async fn compress_analysis(&self, analysis: CodeAnalysis) -> Result<CompressedAnalysis> {
        use flate2::{Compression, write::GzEncoder};
        use std::io::Write;

        let serialize_and_compress = |data: &serde_json::Value| -> Result<Vec<u8>> {
            let json = serde_json::to_vec(data)?;
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&json)?;
            Ok(encoder.finish()?)
        };

        let symbols = serialize_and_compress(&serde_json::to_value(&analysis.symbols)?)?;
        let dependencies = serialize_and_compress(&serde_json::to_value(&analysis.dependencies)?)?;
        let metrics = serialize_and_compress(&serde_json::to_value(&analysis.metrics)?)?;

        let original_size = analysis.symbols.len()
            + analysis.dependencies.len()
            + std::mem::size_of_val(&analysis.metrics);
        let compressed_size = symbols.len() + dependencies.len() + metrics.len();

        Ok(CompressedAnalysis {
            symbols,
            dependencies,
            metrics,
            original_size,
            compressed_size,
        })
    }

    async fn decompress_analysis(&self, compressed: &CompressedAnalysis) -> Result<CodeAnalysis> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let decompress_and_deserialize = |data: &[u8]| -> Result<serde_json::Value> {
            let mut decoder = GzDecoder::new(data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            Ok(serde_json::from_slice(&decompressed)?)
        };

        let symbols: Vec<crate::tree_sitter::languages::SymbolInfo> =
            decompress_and_deserialize(&compressed.symbols)?
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();

        let _dependencies: Vec<String> = decompress_and_deserialize(&compressed.dependencies)?
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let metrics: crate::tree_sitter::CodeMetrics =
            decompress_and_deserialize(&compressed.metrics)?
                .as_object()
                .and_then(|obj| serde_json::from_value(serde_json::Value::Object(obj.clone())).ok())
                .unwrap_or_default();

        Ok(CodeAnalysis {
            file_path: String::new(), // Would need to be stored separately
            language: crate::tree_sitter::analyzer::LanguageSupport::Rust, // Default to Rust
            symbols,
            dependencies: Vec::new(), // Would need proper deserialization
            metrics,
            issues: Vec::new(),
            complexity: Default::default(),
            structure: Default::default(),
        })
    }
}

/// Response time optimizer
pub struct ResponseOptimizer {
    response_times: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    optimization_strategies: Arc<RwLock<HashMap<String, OptimizationStrategy>>>,
}

#[derive(Debug, Clone)]
pub enum OptimizationStrategy {
    CacheFrequentlyAccessed,
    PrecomputeResults,
    ParallelProcessing,
    ReducePayloadSize,
    StreamResponse,
}

impl ResponseOptimizer {
    pub fn new() -> Self {
        Self {
            response_times: Arc::new(RwLock::new(HashMap::new())),
            optimization_strategies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record response time for a specific operation
    pub async fn record_response_time(&self, operation: &str, duration: Duration) {
        let mut response_times = self.response_times.write().await;

        let times = response_times
            .entry(operation.to_string())
            .or_insert_with(Vec::new);
        times.push(duration);

        // Keep only last 100 measurements
        if times.len() > 100 {
            times.remove(0);
        }

        // Analyze and update optimization strategies
        self.analyze_and_optimize(operation, times).await;
    }

    /// Get optimized response strategy for an operation
    pub async fn get_optimization_strategy(&self, operation: &str) -> OptimizationStrategy {
        let strategies = self.optimization_strategies.read().await;

        strategies
            .get(operation)
            .cloned()
            .unwrap_or(OptimizationStrategy::CacheFrequentlyAccessed)
    }

    async fn analyze_and_optimize(&self, operation: &str, times: &[Duration]) {
        if times.len() < 10 {
            return; // Need more data
        }

        let avg_time: Duration = times.iter().sum::<Duration>() / times.len() as u32;
        let recent_avg: Duration = times.iter().rev().take(5).sum::<Duration>() / 5;

        let mut strategies = self.optimization_strategies.write().await;

        let strategy = if avg_time > Duration::from_millis(1000) {
            // Slow operation - use parallel processing
            OptimizationStrategy::ParallelProcessing
        } else if recent_avg > avg_time * 2 {
            // Performance degrading - precompute results
            OptimizationStrategy::PrecomputeResults
        } else if times.len() > 50 {
            // Frequently called - cache results
            OptimizationStrategy::CacheFrequentlyAccessed
        } else {
            // Default strategy
            OptimizationStrategy::CacheFrequentlyAccessed
        };

        strategies.insert(operation.to_string(), strategy);
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> HashMap<String, PerformanceStats> {
        let response_times = self.response_times.read().await;
        let mut stats = HashMap::new();

        for (operation, times) in response_times.iter() {
            if times.is_empty() {
                continue;
            }

            let avg_time = times.iter().sum::<Duration>() / times.len() as u32;
            let min_time = times.iter().min().unwrap();
            let max_time = times.iter().max().unwrap();

            stats.insert(
                operation.clone(),
                PerformanceStats {
                    operation: operation.clone(),
                    avg_response_time: avg_time,
                    min_response_time: *min_time,
                    max_response_time: *max_time,
                    total_calls: times.len(),
                    p95_response_time: self.calculate_percentile(times, 95),
                },
            );
        }

        stats
    }

    fn calculate_percentile(&self, times: &[Duration], percentile: u8) -> Duration {
        if times.is_empty() {
            return Duration::from_millis(0);
        }

        let mut sorted_times = times.to_vec();
        sorted_times.sort();

        let index = (percentile as f64 / 100.0 * (sorted_times.len() - 1) as f64) as usize;
        sorted_times[index]
    }
}

/// Performance statistics for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub operation: String,
    pub avg_response_time: Duration,
    pub min_response_time: Duration,
    pub max_response_time: Duration,
    pub total_calls: usize,
    pub p95_response_time: Duration,
}

/// Performance monitoring system
pub struct PerformanceMonitor {
    start_time: Instant,
    operation_counts: Arc<RwLock<HashMap<String, usize>>>,
    error_counts: Arc<RwLock<HashMap<String, usize>>>,
    memory_usage: Arc<RwLock<Vec<(Instant, usize)>>>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            operation_counts: Arc::new(RwLock::new(HashMap::new())),
            error_counts: Arc::new(RwLock::new(HashMap::new())),
            memory_usage: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record operation execution
    pub async fn record_operation(&self, operation: &str) {
        let mut counts = self.operation_counts.write().await;
        *counts.entry(operation.to_string()).or_insert(0) += 1;
    }

    /// Record error occurrence
    pub async fn record_error(&self, operation: &str) {
        let mut counts = self.error_counts.write().await;
        *counts.entry(operation.to_string()).or_insert(0) += 1;
    }

    /// Record memory usage
    pub async fn record_memory_usage(&self, usage_bytes: usize) {
        let mut memory_usage = self.memory_usage.write().await;
        memory_usage.push((Instant::now(), usage_bytes));

        // Keep only last 1000 measurements
        if memory_usage.len() > 1000 {
            memory_usage.remove(0);
        }
    }

    /// Generate comprehensive performance report
    pub async fn generate_report(&self) -> PerformanceReport {
        let operation_counts = self.operation_counts.read().await;
        let error_counts = self.error_counts.read().await;
        let memory_usage = self.memory_usage.read().await;

        let total_operations: usize = operation_counts.values().sum();
        let total_errors: usize = error_counts.values().sum();

        let avg_memory_usage = if !memory_usage.is_empty() {
            memory_usage.iter().map(|(_, usage)| usage).sum::<usize>() / memory_usage.len()
        } else {
            0
        };

        let uptime = self.start_time.elapsed();

        PerformanceReport {
            uptime,
            total_operations,
            total_errors,
            error_rate: if total_operations > 0 {
                total_errors as f64 / total_operations as f64
            } else {
                0.0
            },
            avg_memory_usage,
            operations_per_second: total_operations as f64 / uptime.as_secs_f64(),
            operation_breakdown: operation_counts.clone(),
            error_breakdown: error_counts.clone(),
        }
    }
}

/// Comprehensive performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub uptime: Duration,
    pub total_operations: usize,
    pub total_errors: usize,
    pub error_rate: f64,
    pub avg_memory_usage: usize,
    pub operations_per_second: f64,
    pub operation_breakdown: HashMap<String, usize>,
    pub error_breakdown: HashMap<String, usize>,
}
