//! Core traits for the composable tool system

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;

/// Core trait for all agent tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Execute the tool with given arguments
    async fn execute(&self, args: Value) -> Result<Value>;
    
    /// Get the tool's name
    fn name(&self) -> &'static str;
    
    /// Get the tool's description
    fn description(&self) -> &'static str;
    
    /// Validate arguments before execution
    fn validate_args(&self, args: &Value) -> Result<()> {
        // Default implementation - tools can override for specific validation
        Ok(())
    }
}

/// Trait for tools that operate on files
#[async_trait]
pub trait FileTool: Tool {
    /// Get the workspace root
    fn workspace_root(&self) -> &PathBuf;
    
    /// Check if a path should be excluded
    async fn should_exclude(&self, path: &std::path::Path) -> bool;
}

/// Trait for tools that support multiple execution modes
#[async_trait]
pub trait ModeTool: Tool {
    /// Get supported modes
    fn supported_modes(&self) -> Vec<&'static str>;
    
    /// Execute with specific mode
    async fn execute_mode(&self, mode: &str, args: Value) -> Result<Value>;
}

/// Trait for caching tool results
#[async_trait]
pub trait CacheableTool: Tool {
    /// Generate cache key for given arguments
    fn cache_key(&self, args: &Value) -> String;
    
    /// Check if result should be cached
    fn should_cache(&self, args: &Value) -> bool {
        true // Default: cache everything
    }
    
    /// Get cache TTL in seconds
    fn cache_ttl(&self) -> u64 {
        300 // Default: 5 minutes
    }
}

/// Main tool executor that coordinates all tools
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool by name
    async fn execute_tool(&self, name: &str, args: Value) -> Result<Value>;
    
    /// List available tools
    fn available_tools(&self) -> Vec<String>;
    
    /// Check if a tool exists
    fn has_tool(&self, name: &str) -> bool;
}
