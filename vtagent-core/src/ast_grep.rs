//! AST-grep integration for VTAgent
//!
//! This module provides integration with the ast-grep CLI tool for
//! syntax-aware code search, transformation, linting, and refactoring.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use tokio;

/// AST-grep engine for syntax-aware code operations
pub struct AstGrepEngine {
    /// Path to the ast-grep executable
    sgrep_path: String,
}

impl AstGrepEngine {
    /// Create a new AST-grep engine
    pub fn new() -> Result<Self> {
        // Try to find ast-grep in PATH
        let sgrep_path = if cfg!(target_os = "windows") {
            "ast-grep.exe"
        } else {
            "ast-grep"
        };
        
        // Verify ast-grep is available
        let output = std::process::Command::new(sgrep_path)
            .arg("--version")
            .output()
            .context("Failed to execute ast-grep")?;
            
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "ast-grep not found or not working properly"
            ));
        }
        
        Ok(Self {
            sgrep_path: sgrep_path.to_string(),
        })
    }
    
    /// Search code using AST-grep patterns
    pub async fn search(
        &self,
        pattern: &str,
        path: &str,
        language: Option<&str>,
    ) -> Result<Value> {
        let sgrep_path = self.sgrep_path.clone();
        let pattern = pattern.to_string();
        let path = path.to_string();
        let language = language.map(|s| s.to_string());
        
        let handle = tokio::task::spawn_blocking(move || {
            let mut cmd = std::process::Command::new(&sgrep_path);
            cmd.arg("run")
                .arg("--pattern")
                .arg(&pattern)
                .arg("--json")
                .arg(&path);
                
            if let Some(lang) = language {
                cmd.arg("--lang").arg(&lang);
            }
            
            cmd.output()
        });
        
        let output = handle
            .await
            .context("Failed to spawn ast-grep search task")?
            .context("Failed to execute ast-grep search")?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "ast-grep search failed: {}",
                stderr
            ));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Value = serde_json::from_str(&stdout)
            .context("Failed to parse ast-grep search results")?;
            
        Ok(json!({ "success": true, "matches": results }))
    }
    
    /// Transform code using AST-grep patterns
    pub async fn transform(
        &self,
        pattern: &str,
        replacement: &str,
        path: &str,
        language: Option<&str>,
        preview_only: bool,
    ) -> Result<Value> {
        let sgrep_path = self.sgrep_path.clone();
        let pattern = pattern.to_string();
        let replacement = replacement.to_string();
        let path = path.to_string();
        let language = language.map(|s| s.to_string());
        
        let handle = tokio::task::spawn_blocking(move || {
            let mut cmd = std::process::Command::new(&sgrep_path);
            cmd.arg("run")
                .arg("--pattern")
                .arg(&pattern)
                .arg("--rewrite")
                .arg(&replacement)
                .arg("--json")
                .arg(&path);
                
            if let Some(lang) = language {
                cmd.arg("--lang").arg(&lang);
            }
            
            if preview_only {
                // For preview, we'll get the matches but not apply changes
                cmd.arg("--interactive").arg("never");
            }
            
            cmd.output()
        });
        
        let output = handle
            .await
            .context("Failed to spawn ast-grep transform task")?
            .context("Failed to execute ast-grep transform")?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "ast-grep transform failed: {}",
                stderr
            ));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Value = serde_json::from_str(&stdout)
            .context("Failed to parse ast-grep transform results")?;
            
        Ok(json!({ "success": true, "changes": results }))
    }
    
    /// Lint code using AST-grep rules
    pub async fn lint(
        &self,
        path: &str,
        language: Option<&str>,
    ) -> Result<Value> {
        let sgrep_path = self.sgrep_path.clone();
        let path = path.to_string();
        let language = language.map(|s| s.to_string());
        
        let handle = tokio::task::spawn_blocking(move || {
            let mut cmd = std::process::Command::new(&sgrep_path);
            cmd.arg("run")
                .arg("--pattern")
                .arg("TODO($$)")
                .arg("--json")
                .arg(&path);
                
            if let Some(lang) = language {
                cmd.arg("--lang").arg(&lang);
            }
            
            cmd.output()
        });
        
        let output = handle
            .await
            .context("Failed to spawn ast-grep lint task")?
            .context("Failed to execute ast-grep lint")?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "ast-grep lint failed: {}",
                stderr
            ));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Value = serde_json::from_str(&stdout)
            .context("Failed to parse ast-grep lint results")?;
            
        Ok(json!({ "success": true, "issues": results }))
    }
    
    /// Get refactoring suggestions using AST-grep
    pub async fn refactor(
        &self,
        path: &str,
        language: Option<&str>,
        refactor_type: &str,
    ) -> Result<Value> {
        // Different refactoring suggestions based on type
        let (pattern, replacement) = match refactor_type {
            "extract_function" => (
                "function $func($$) { $$ }",
                "// TODO: Extract function $func to separate module\nfunction $func($$) { $$ }"
            ),
            "remove_console_logs" => (
                "console.log($$)",
                ""
            ),
            "simplify_conditions" => (
                "if ($cond) { true } else { false }",
                "$cond"
            ),
            _ => (
                "$$",
                "$$"
            )
        };
        
        let sgrep_path = self.sgrep_path.clone();
        let path = path.to_string();
        let language = language.map(|s| s.to_string());
        let pattern = pattern.to_string();
        let replacement = replacement.to_string();
        
        let handle = tokio::task::spawn_blocking(move || {
            let mut cmd = std::process::Command::new(&sgrep_path);
            cmd.arg("run")
                .arg("--pattern")
                .arg(&pattern)
                .arg("--rewrite")
                .arg(&replacement)
                .arg("--json")
                .arg(&path);
                
            if let Some(lang) = language {
                cmd.arg("--lang").arg(&lang);
            }
            
            cmd.output()
        });
        
        let output = handle
            .await
            .context("Failed to spawn ast-grep refactor task")?
            .context("Failed to execute ast-grep refactor")?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "ast-grep refactor failed: {}",
                stderr
            ));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Value = serde_json::from_str(&stdout)
            .context("Failed to parse ast-grep refactor results")?;
            
        Ok(json!({ "success": true, "suggestions": results }))
    }
}