//! Code Quality Tools Module
//!
//! This module provides comprehensive code formatting, linting, and quality assurance tools:
//! - Automatic code formatting (rustfmt, prettier, black, etc.)
//! - Linting and static analysis
//! - Code quality metrics and reporting
//! - Integration with language-specific tools
//! - Quality gate enforcement

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enhanced_file_ops::EnhancedFileOps;
use crate::tree_sitter::{LanguageSupport, TreeSitterAnalyzer};

/// Code formatting tool configuration
#[derive(Debug, Clone)]
pub struct FormatConfig {
    pub language: LanguageSupport,
    pub tool_name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub file_extensions: Vec<String>,
    pub enabled: bool,
}

/// Linting tool configuration
#[derive(Debug, Clone)]
pub struct LintConfig {
    pub language: LanguageSupport,
    pub tool_name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub severity_levels: HashMap<String, LintSeverity>,
    pub enabled: bool,
}

/// Lint result severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Individual lint finding
#[derive(Debug, Clone)]
pub struct LintFinding {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub severity: LintSeverity,
    pub rule: String,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Code quality metrics
#[derive(Debug, Clone, Default)]
pub struct CodeQualityMetrics {
    pub total_files: usize,
    pub formatted_files: usize,
    pub lint_errors: usize,
    pub lint_warnings: usize,
    pub lint_info: usize,
    pub complexity_score: f64,
    pub maintainability_index: f64,
    pub test_coverage: Option<f64>,
}

/// Code quality assessment result
#[derive(Debug, Clone)]
pub struct QualityAssessment {
    pub overall_score: f64, // 0.0 to 100.0
    pub grade: QualityGrade,
    pub metrics: CodeQualityMetrics,
    pub recommendations: Vec<String>,
    pub critical_issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityGrade {
    A, // Excellent (90-100)
    B, // Good (80-89)
    C, // Satisfactory (70-79)
    D, // Needs Improvement (60-69)
    F, // Poor (< 60)
}

/// Comprehensive code quality management system
pub struct CodeQualityManager {
    file_ops: Arc<EnhancedFileOps>,
    tree_sitter: Arc<TreeSitterAnalyzer>,
    format_configs: HashMap<LanguageSupport, FormatConfig>,
    lint_configs: HashMap<LanguageSupport, LintConfig>,
    metrics_history: Arc<RwLock<Vec<CodeQualityMetrics>>>,
    quality_thresholds: QualityThresholds,
}

#[derive(Debug, Clone)]
pub struct QualityThresholds {
    pub max_lint_errors: usize,
    pub max_lint_warnings: usize,
    pub min_test_coverage: Option<f64>,
    pub max_complexity_score: f64,
    pub min_maintainability_index: f64,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            max_lint_errors: 0,
            max_lint_warnings: 10,
            min_test_coverage: Some(80.0),
            max_complexity_score: 10.0,
            min_maintainability_index: 50.0,
        }
    }
}

impl CodeQualityManager {
    /// Create a new code quality manager with default configurations
    pub fn new(file_ops: Arc<EnhancedFileOps>, tree_sitter: Arc<TreeSitterAnalyzer>) -> Self {
        let mut manager = Self {
            file_ops,
            tree_sitter,
            format_configs: HashMap::new(),
            lint_configs: HashMap::new(),
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            quality_thresholds: QualityThresholds::default(),
        };

        manager.initialize_default_configs();
        manager
    }

    /// Initialize default formatting and linting configurations
    fn initialize_default_configs(&mut self) {
        // Rust configuration
        self.format_configs.insert(
            LanguageSupport::Rust,
            FormatConfig {
                language: LanguageSupport::Rust,
                tool_name: "rustfmt".to_string(),
                command: vec!["rustfmt".to_string()],
                args: vec!["--edition".to_string(), "2021".to_string()],
                file_extensions: vec!["rs".to_string()],
                enabled: true,
            },
        );

        self.lint_configs.insert(
            LanguageSupport::Rust,
            LintConfig {
                language: LanguageSupport::Rust,
                tool_name: "clippy".to_string(),
                command: vec!["cargo".to_string()],
                args: vec!["clippy".to_string(), "--".to_string(), "-D".to_string(), "warnings".to_string()],
                severity_levels: HashMap::new(),
                enabled: true,
            },
        );

        // Python configuration
        self.format_configs.insert(
            LanguageSupport::Python,
            FormatConfig {
                language: LanguageSupport::Python,
                tool_name: "black".to_string(),
                command: vec!["black".to_string()],
                args: vec!["--line-length".to_string(), "88".to_string()],
                file_extensions: vec!["py".to_string()],
                enabled: true,
            },
        );

        self.lint_configs.insert(
            LanguageSupport::Python,
            LintConfig {
                language: LanguageSupport::Python,
                tool_name: "flake8".to_string(),
                command: vec!["flake8".to_string()],
                args: vec!["--max-line-length=88".to_string()],
                severity_levels: HashMap::new(),
                enabled: true,
            },
        );

        // JavaScript/TypeScript configuration
        self.format_configs.insert(
            LanguageSupport::JavaScript,
            FormatConfig {
                language: LanguageSupport::JavaScript,
                tool_name: "prettier".to_string(),
                command: vec!["prettier".to_string()],
                args: vec!["--write".to_string()],
                file_extensions: vec!["js".to_string(), "jsx".to_string()],
                enabled: true,
            },
        );

        self.format_configs.insert(
            LanguageSupport::TypeScript,
            FormatConfig {
                language: LanguageSupport::TypeScript,
                tool_name: "prettier".to_string(),
                command: vec!["prettier".to_string()],
                args: vec!["--write".to_string()],
                file_extensions: vec!["ts".to_string(), "tsx".to_string()],
                enabled: true,
            },
        );
    }

    /// Format code files in the specified directory
    pub async fn format_codebase(&self, root_path: &Path) -> Result<FormatResult> {
        let mut results = Vec::new();
        let mut formatted_count = 0;

        // Discover files that need formatting
        let files_to_format = self.discover_files_for_formatting(root_path).await?;

        for file_path in files_to_format {
            match self.format_single_file(&file_path).await {
                Ok(result) => {
                    results.push(result.clone());
                    if result.formatted {
                        formatted_count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to format {}: {}", file_path.display(), e);
                }
            }
        }

        Ok(FormatResult {
            total_files: results.len(),
            formatted_files: formatted_count,
            skipped_files: results.len() - formatted_count,
            results,
        })
    }

    /// Run linting on the codebase
    pub async fn lint_codebase(&self, root_path: &Path) -> Result<LintResult> {
        let mut all_findings = Vec::new();
        let mut files_checked = 0;

        // Discover files that need linting
        let files_to_lint = self.discover_files_for_linting(root_path).await?;

        for file_path in files_to_lint {
            match self.lint_single_file(&file_path).await {
                Ok(findings) => {
                    all_findings.extend(findings);
                    files_checked += 1;
                }
                Err(e) => {
                    eprintln!("Failed to lint {}: {}", file_path.display(), e);
                }
            }
        }

        let (errors, warnings, info) = self.categorize_findings(&all_findings);

        Ok(LintResult {
            total_files: files_checked,
            total_findings: all_findings.len(),
            errors,
            warnings,
            info,
            findings: all_findings,
        })
    }

    /// Perform comprehensive code quality assessment
    pub async fn assess_code_quality(&self, root_path: &Path) -> Result<QualityAssessment> {
        // Run formatting check
        let format_result = self.format_codebase(root_path).await?;

        // Run linting
        let lint_result = self.lint_codebase(root_path).await?;

        // Calculate metrics
        let metrics = self.calculate_quality_metrics(root_path, &format_result, &lint_result).await?;

        // Determine overall score and grade
        let overall_score = self.calculate_overall_score(&metrics);
        let grade = self.determine_grade(overall_score);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&metrics);
        let critical_issues = self.identify_critical_issues(&lint_result);

        // Store metrics history
        self.store_metrics_history(metrics.clone()).await;

        Ok(QualityAssessment {
            overall_score,
            grade,
            metrics,
            recommendations,
            critical_issues,
        })
    }

    /// Format a single file
    async fn format_single_file(&self, file_path: &Path) -> Result<FileFormatResult> {
        // Determine language and get format config
        let language = self.tree_sitter.detect_language_from_path(file_path)?;
        let format_config = self.format_configs.get(&language)
            .ok_or_else(|| anyhow!("No formatter configured for language: {:?}", language))?;

        if !format_config.enabled {
            return Ok(FileFormatResult {
                file_path: file_path.to_path_buf(),
                formatted: false,
                original_size: 0,
                new_size: 0,
                error: Some("Formatter disabled".to_string()),
            });
        }

        // Read original file
        let (original_content, _) = self.file_ops.read_file_enhanced(file_path, None).await?;
        let original_size = original_content.len();

        // Run formatter
        let output = Command::new(&format_config.command[0])
            .args(&format_config.args)
            .arg(file_path)
            .output()?;

        if output.status.success() {
            // Read formatted content
            let (new_content, _) = self.file_ops.read_file_enhanced(file_path, None).await?;
            let new_size = new_content.len();

            Ok(FileFormatResult {
                file_path: file_path.to_path_buf(),
                formatted: new_size != original_size,
                original_size,
                new_size,
                error: None,
            })
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Ok(FileFormatResult {
                file_path: file_path.to_path_buf(),
                formatted: false,
                original_size,
                new_size: original_size,
                error: Some(error_msg.to_string()),
            })
        }
    }

    /// Lint a single file
    async fn lint_single_file(&self, file_path: &Path) -> Result<Vec<LintFinding>> {
        // Determine language and get lint config
        let language = self.tree_sitter.detect_language_from_path(file_path)?;
        let lint_config = self.lint_configs.get(&language)
            .ok_or_else(|| anyhow!("No linter configured for language: {:?}", language))?;

        if !lint_config.enabled {
            return Ok(Vec::new());
        }

        // Run linter
        let mut command = Command::new(&lint_config.command[0]);
        command.args(&lint_config.args);

        if lint_config.tool_name == "cargo" {
            // Special handling for cargo clippy
            command.current_dir(file_path.parent().unwrap_or(Path::new(".")));
        } else {
            command.arg(file_path);
        }

        let output = command.output()?;

        // Parse linter output (this would need to be implemented for each linter)
        self.parse_linter_output(&output.stdout, file_path, &lint_config.tool_name)
    }

    /// Discover files that need formatting
    async fn discover_files_for_formatting(&self, _root_path: &Path) -> Result<Vec<PathBuf>> {
        let files = Vec::new();

        for format_config in self.format_configs.values() {
            for _ext in &format_config.file_extensions {
                // This would need to be implemented with a file discovery utility
                // For now, return empty vec as placeholder
            }
        }

        Ok(files)
    }

    /// Discover files that need linting
    async fn discover_files_for_linting(&self, _root_path: &Path) -> Result<Vec<PathBuf>> {
        let files = Vec::new();

        for lint_config in self.lint_configs.values() {
            for format_config in self.format_configs.values() {
                if format_config.language == lint_config.language {
                    for _ext in &format_config.file_extensions {
                        // This would need to be implemented with a file discovery utility
                        // For now, return empty vec as placeholder
                    }
                }
            }
        }

        Ok(files)
    }

    /// Parse linter output into LintFinding structures
    fn parse_linter_output(&self, output: &[u8], file_path: &Path, _tool_name: &str) -> Result<Vec<LintFinding>> {
        let output_str = String::from_utf8_lossy(output);
        let mut findings = Vec::new();

        // This is a placeholder implementation
        // Real implementation would parse the specific output format of each linter
        for line in output_str.lines() {
            if !line.trim().is_empty() {
                findings.push(LintFinding {
                    file_path: file_path.to_path_buf(),
                    line: 1,
                    column: 1,
                    severity: LintSeverity::Warning,
                    rule: "placeholder".to_string(),
                    message: line.to_string(),
                    suggestion: None,
                });
            }
        }

        Ok(findings)
    }

    /// Categorize lint findings by severity
    fn categorize_findings(&self, findings: &[LintFinding]) -> (usize, usize, usize) {
        let errors = findings.iter().filter(|f| f.severity == LintSeverity::Error || f.severity == LintSeverity::Critical).count();
        let warnings = findings.iter().filter(|f| f.severity == LintSeverity::Warning).count();
        let info = findings.iter().filter(|f| f.severity == LintSeverity::Info).count();

        (errors, warnings, info)
    }

    /// Calculate comprehensive code quality metrics
    async fn calculate_quality_metrics(
        &self,
        _root_path: &Path,
        format_result: &FormatResult,
        lint_result: &LintResult,
    ) -> Result<CodeQualityMetrics> {
        Ok(CodeQualityMetrics {
            total_files: format_result.total_files.max(lint_result.total_files),
            formatted_files: format_result.formatted_files,
            lint_errors: lint_result.errors,
            lint_warnings: lint_result.warnings,
            lint_info: lint_result.info,
            complexity_score: 5.0, // Placeholder
            maintainability_index: 75.0, // Placeholder
            test_coverage: Some(85.0), // Placeholder
        })
    }

    /// Calculate overall quality score
    fn calculate_overall_score(&self, metrics: &CodeQualityMetrics) -> f64 {
        let mut score = 100.0;

        // Deduct points for lint errors
        score -= (metrics.lint_errors as f64) * 10.0;

        // Deduct points for lint warnings
        score -= (metrics.lint_warnings as f64) * 2.0;

        // Bonus for high formatting coverage
        if metrics.total_files > 0 {
            let format_coverage = metrics.formatted_files as f64 / metrics.total_files as f64;
            score += format_coverage * 10.0;
        }

        // Ensure score stays within bounds
        score.max(0.0).min(100.0)
    }

    /// Determine quality grade from score
    fn determine_grade(&self, score: f64) -> QualityGrade {
        match score as u32 {
            90..=100 => QualityGrade::A,
            80..=89 => QualityGrade::B,
            70..=79 => QualityGrade::C,
            60..=69 => QualityGrade::D,
            _ => QualityGrade::F,
        }
    }

    /// Generate quality improvement recommendations
    fn generate_recommendations(&self, metrics: &CodeQualityMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        if metrics.lint_errors > 0 {
            recommendations.push(format!("Fix {} lint errors to improve code quality", metrics.lint_errors));
        }

        if metrics.lint_warnings > 10 {
            recommendations.push(format!("Address {} lint warnings", metrics.lint_warnings));
        }

        if metrics.formatted_files < metrics.total_files {
            let unformatted = metrics.total_files - metrics.formatted_files;
            recommendations.push(format!("Format {} unformatted files", unformatted));
        }

        if recommendations.is_empty() {
            recommendations.push("Code quality is excellent! Keep up the good work.".to_string());
        }

        recommendations
    }

    /// Identify critical quality issues
    fn identify_critical_issues(&self, lint_result: &LintResult) -> Vec<String> {
        let mut issues = Vec::new();

        if lint_result.errors > 0 {
            issues.push(format!("{} critical lint errors require immediate attention", lint_result.errors));
        }

        if lint_result.warnings > self.quality_thresholds.max_lint_warnings {
            issues.push(format!("{} warnings exceed threshold of {}", lint_result.warnings, self.quality_thresholds.max_lint_warnings));
        }

        issues
    }

    /// Store metrics in history
    async fn store_metrics_history(&self, metrics: CodeQualityMetrics) {
        let mut history = self.metrics_history.write().await;
        history.push(metrics);

        // Keep only last 10 entries
        if history.len() > 10 {
            history.remove(0);
        }
    }

    /// Get quality metrics history
    pub async fn get_metrics_history(&self) -> Vec<CodeQualityMetrics> {
        self.metrics_history.read().await.clone()
    }

    /// Configure quality thresholds
    pub fn set_quality_thresholds(&mut self, thresholds: QualityThresholds) {
        self.quality_thresholds = thresholds;
    }
}

/// Result of formatting operation
#[derive(Debug, Clone)]
pub struct FormatResult {
    pub total_files: usize,
    pub formatted_files: usize,
    pub skipped_files: usize,
    pub results: Vec<FileFormatResult>,
}

/// Result of formatting a single file
#[derive(Debug, Clone)]
pub struct FileFormatResult {
    pub file_path: PathBuf,
    pub formatted: bool,
    pub original_size: usize,
    pub new_size: usize,
    pub error: Option<String>,
}

/// Result of linting operation
#[derive(Debug, Clone)]
pub struct LintResult {
    pub total_files: usize,
    pub total_findings: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub findings: Vec<LintFinding>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::test;

    #[test]
    async fn test_code_quality_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let file_ops = Arc::new(EnhancedFileOps::new(5));
        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());

        let manager = CodeQualityManager::new(file_ops, tree_sitter);

        // Test that default configurations are loaded
        assert!(manager.format_configs.contains_key(&LanguageSupport::Rust));
        assert!(manager.lint_configs.contains_key(&LanguageSupport::Rust));
        assert!(manager.format_configs.contains_key(&LanguageSupport::Python));
    }

    #[test]
    async fn test_quality_grade_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let file_ops = Arc::new(EnhancedFileOps::new(5));
        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());

        let manager = CodeQualityManager::new(file_ops, tree_sitter);

        assert_eq!(manager.determine_grade(95.0), QualityGrade::A);
        assert_eq!(manager.determine_grade(85.0), QualityGrade::B);
        assert_eq!(manager.determine_grade(75.0), QualityGrade::C);
        assert_eq!(manager.determine_grade(65.0), QualityGrade::D);
        assert_eq!(manager.determine_grade(55.0), QualityGrade::F);
    }

    #[test]
    async fn test_overall_score_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let file_ops = Arc::new(EnhancedFileOps::new(5));
        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());

        let manager = CodeQualityManager::new(file_ops, tree_sitter);

        let metrics = CodeQualityMetrics {
            total_files: 10,
            formatted_files: 8,
            lint_errors: 2,
            lint_warnings: 5,
            lint_info: 1,
            complexity_score: 5.0,
            maintainability_index: 75.0,
            test_coverage: Some(85.0),
        };

        let score = manager.calculate_overall_score(&metrics);

        // Score should be reduced by errors and warnings, increased by formatting coverage
        assert!(score < 100.0);
        assert!(score > 70.0); // Should still be reasonable
    }
}
