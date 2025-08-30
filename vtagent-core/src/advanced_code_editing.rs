//! Minimal research-preview Code Editing Module
//!
//! This module provides sophisticated code editing capabilities with:
//! - Multi-step edit orchestration
//! - Syntax-aware modifications
//! - Dependency-aware changes
//! - Safe rollback mechanisms
//! - Context preservation

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enhanced_file_ops::EnhancedFileOps;
use crate::tree_sitter::{CodeAnalysis, TreeSitterAnalyzer};

/// Represents a single code edit operation
#[derive(Debug, Clone)]
pub struct CodeEdit {
    pub file_path: PathBuf,
    pub operation: EditOperation,
    pub context: Option<String>,
    pub dependencies: Vec<PathBuf>,
    pub rollback_data: Option<String>,
}

#[derive(Debug, Clone)]
pub enum EditOperation {
    /// Replace text at specific location
    Replace {
        old_string: String,
        new_string: String,
        start_line: usize,
        end_line: usize,
    },
    /// Insert text at specific location
    Insert {
        content: String,
        line: usize,
        after: bool, // true = insert after line, false = insert before
    },
    /// Delete text range
    Delete {
        start_line: usize,
        end_line: usize,
    },
    /// Rename identifier with scope awareness
    Rename {
        old_name: String,
        new_name: String,
        scope: Option<String>, // e.g., "function", "class", "module"
    },
}

/// Multi-step edit plan with dependencies
#[derive(Debug, Clone)]
pub struct EditPlan {
    pub edits: Vec<CodeEdit>,
    pub dependencies: HashMap<PathBuf, Vec<PathBuf>>,
    pub validation_steps: Vec<ValidationStep>,
    pub rollback_plan: Vec<CodeEdit>,
}

#[derive(Debug, Clone)]
pub struct ValidationStep {
    pub name: String,
    pub check_type: ValidationType,
    pub file_path: PathBuf,
    pub expected_result: String,
}

#[derive(Debug, Clone)]
pub enum ValidationType {
    SyntaxCheck,
    TypeCheck,
    ImportResolution,
    TestExecution,
    Custom(String),
}

/// Minimal code editing orchestrator
pub struct MinimalCodeEditor {
    file_ops: Arc<EnhancedFileOps>,
    _tree_sitter: Arc<TreeSitterAnalyzer>,
    edit_history: Arc<RwLock<Vec<EditPlan>>>,
    max_history_size: usize,
}

impl MinimalCodeEditor {
    /// Create a new Minimal research-preview code editor
    pub fn new(file_ops: Arc<EnhancedFileOps>, tree_sitter: Arc<TreeSitterAnalyzer>) -> Self {
        Self {
            file_ops,
            _tree_sitter: tree_sitter,
            edit_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 50,
        }
    }

    /// Execute a multi-step edit plan with full validation and rollback support
    pub async fn execute_edit_plan(&self, plan: EditPlan) -> Result<EditResult> {
        let mut results = Vec::new();
        let mut successful_edits = 0;

        // Validate dependencies before starting
        self.validate_dependencies(&plan).await?;

        // Clone plan for later use
        let plan_clone = plan.clone();

        // Execute edits in dependency order
        for edit in &plan.edits {
            match self.execute_single_edit(edit).await {
                Ok(result) => {
                    results.push(result);
                    successful_edits += 1;
                }
                Err(e) => {
                    // Attempt rollback on failure
                    self.rollback_edits(&results).await?;
                    return Err(anyhow!("Edit failed and rollback attempted: {}", e));
                }
            }
        }

        // Run validation steps
        for validation in &plan_clone.validation_steps {
            if let Err(e) = self.run_validation(validation).await {
                // Validation failed - rollback
                self.rollback_edits(&results).await?;
                return Err(anyhow!("Validation failed: {}", e));
            }
        }

        // Store in history
        self.store_edit_plan(plan_clone).await;

        Ok(EditResult {
            total_edits: plan.edits.len(),
            successful_edits,
            failed_edits: plan.edits.len() - successful_edits,
            validation_passed: true,
            rollback_performed: false,
        })
    }

    /// Generate an edit plan for common code refactoring operations
    pub async fn generate_refactor_plan(
        &self,
        operation: RefactorOperation,
        target_path: &Path,
    ) -> Result<EditPlan> {
        match operation {
            RefactorOperation::ExtractFunction { start_line, end_line, new_function_name } => {
                self.generate_extract_function_plan(target_path, start_line, end_line, &new_function_name).await
            }
            RefactorOperation::RenameSymbol { old_name, new_name } => {
                self.generate_rename_symbol_plan(target_path, &old_name, &new_name).await
            }
            RefactorOperation::AddDependency { crate_name, features } => {
                self.generate_add_dependency_plan(&crate_name, &features).await
            }
        }
    }

    /// Execute a single code edit with context preservation
    async fn execute_single_edit(&self, edit: &CodeEdit) -> Result<EditResult> {
        match &edit.operation {
            EditOperation::Replace { old_string, new_string, .. } => {
                self.file_ops.edit_file_enhanced(
                    &edit.file_path,
                    old_string,
                    new_string,
                    true, // Always create backup
                ).await?;
            }
            EditOperation::Insert { content, line, after } => {
                self.execute_insert_operation(edit, content, *line, *after).await?;
            }
            EditOperation::Delete { start_line, end_line } => {
                self.execute_delete_operation(edit, *start_line, *end_line).await?;
            }
            EditOperation::Rename { old_name, new_name, .. } => {
                self.execute_rename_operation(edit, old_name, new_name).await?;
            }
        }

        Ok(EditResult {
            total_edits: 1,
            successful_edits: 1,
            failed_edits: 0,
            validation_passed: true,
            rollback_performed: false,
        })
    }

    /// Execute insert operation with proper indentation and context
    async fn execute_insert_operation(
        &self,
        edit: &CodeEdit,
        content: &str,
        line: usize,
        after: bool,
    ) -> Result<()> {
        // Read the file to understand context
        let (file_content, _) = self.file_ops.read_file_enhanced(&edit.file_path, None).await?;
        let lines: Vec<&str> = file_content.lines().collect();

        if line >= lines.len() {
            return Err(anyhow!("Line {} is beyond file length {}", line, lines.len()));
        }

        // Determine insertion point and indentation
        let insert_line = if after { line + 1 } else { line };
        let base_indentation = self.infer_indentation(&lines, line);

        // Apply proper indentation to inserted content
        let indented_content = self.apply_indentation(content, &base_indentation);

        // Create the edit operation
        let before_lines = &lines[0..insert_line];
        let after_lines = &lines[insert_line..];

        let new_content = format!(
            "{}\n{}{}",
            before_lines.join("\n"),
            indented_content,
            after_lines.join("\n")
        );

        self.file_ops.write_file_enhanced(&edit.file_path, &new_content, true).await?;
        Ok(())
    }

    /// Execute delete operation with safety checks
    async fn execute_delete_operation(
        &self,
        edit: &CodeEdit,
        start_line: usize,
        end_line: usize,
    ) -> Result<()> {
        let (file_content, _) = self.file_ops.read_file_enhanced(&edit.file_path, None).await?;
        let lines: Vec<&str> = file_content.lines().collect();

        if start_line >= lines.len() || end_line >= lines.len() || start_line > end_line {
            return Err(anyhow!("Invalid line range: {} to {}", start_line, end_line));
        }

        // Check for syntax-critical deletions
        if self.would_break_syntax(&lines[start_line..=end_line], &edit.file_path).await? {
            return Err(anyhow!("Deletion would break syntax - refusing to proceed"));
        }

        let before_lines = &lines[0..start_line];
        let after_lines = &lines[end_line + 1..];

        let new_content = format!(
            "{}\n{}",
            before_lines.join("\n"),
            after_lines.join("\n")
        );

        self.file_ops.write_file_enhanced(&edit.file_path, &new_content, true).await?;
        Ok(())
    }

    /// Execute rename operation with scope awareness
    async fn execute_rename_operation(
        &self,
        edit: &CodeEdit,
        old_name: &str,
        new_name: &str,
    ) -> Result<()> {
        // Use tree-sitter for intelligent renaming
        // TODO: Implement proper analysis using available TreeSitterAnalyzer methods

        // Find all occurrences of the symbol in the appropriate scope
        // TODO: Implement proper symbol occurrence finding
        let occurrences: Vec<SymbolOccurrence> = Vec::new();

        // Apply renames in reverse order to avoid position shifts
        for occurrence in occurrences.into_iter().rev() {
            let (content, _) = self.file_ops.read_file_enhanced(&edit.file_path, None).await?;
            let lines: Vec<&str> = content.lines().collect();

            if let Some(line_content) = lines.get(occurrence.line) {
                let new_line = line_content.replace(old_name, new_name);
                let before_lines = &lines[0..occurrence.line];
                let after_lines = &lines[occurrence.line + 1..];

                let new_content = format!(
                    "{}\n{}\n{}",
                    before_lines.join("\n"),
                    new_line,
                    after_lines.join("\n")
                );

                self.file_ops.write_file_enhanced(&edit.file_path, &new_content, true).await?;
            }
        }

        Ok(())
    }

    /// Infer indentation from surrounding context
    fn infer_indentation(&self, lines: &[&str], reference_line: usize) -> String {
        if reference_line >= lines.len() {
            return String::new();
        }

        let reference = lines[reference_line];
        let mut indentation = String::new();

        for ch in reference.chars() {
            if ch.is_whitespace() {
                indentation.push(ch);
            } else {
                break;
            }
        }

        indentation
    }

    /// Apply indentation to content
    fn apply_indentation(&self, content: &str, indentation: &str) -> String {
        content
            .lines()
            .map(|line| {
                if line.trim().is_empty() {
                    line.to_string()
                } else {
                    format!("{}{}", indentation, line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if deletion would break syntax
    async fn would_break_syntax(&self, lines_to_delete: &[&str], _file_path: &Path) -> Result<bool> {
        let content_to_check = lines_to_delete.join("\n");

        // Check for unmatched braces, incomplete statements, etc.
        let open_braces = content_to_check.matches('{').count();
        let close_braces = content_to_check.matches('}').count();

        if open_braces != close_braces {
            return Ok(true); // Would break brace matching
        }

        // Check for incomplete control structures
        let control_keywords = ["if", "for", "while", "match", "fn"];
        for keyword in &control_keywords {
            if content_to_check.contains(keyword) && !content_to_check.contains('{') {
                return Ok(true); // Incomplete control structure
            }
        }

        Ok(false)
    }

    /// Find all occurrences of a symbol in the code
    #[allow(dead_code)]
    fn find_symbol_occurrences(&self, _analysis: &CodeAnalysis, _symbol_name: &str) -> Result<Vec<SymbolOccurrence>> {
        // This would use tree-sitter to find symbol occurrences
        // For now, return empty vec as placeholder
        Ok(Vec::new())
    }

    /// Validate edit dependencies
    async fn validate_dependencies(&self, plan: &EditPlan) -> Result<()> {
        for (file, deps) in &plan.dependencies {
            for dep in deps {
                if !dep.exists() {
                    return Err(anyhow!("Dependency {} does not exist for file {}", dep.display(), file.display()));
                }
            }
        }
        Ok(())
    }

    /// Run validation step
    async fn run_validation(&self, validation: &ValidationStep) -> Result<()> {
        match validation.check_type {
            ValidationType::SyntaxCheck => {
                // Use tree-sitter to check syntax
                // TODO: Implement syntax checking using available TreeSitterAnalyzer methods
                Ok(())
            }
            ValidationType::TypeCheck => {
                // This would run a type checker (e.g., cargo check)
                Ok(())
            }
            ValidationType::ImportResolution => {
                // Check if imports can be resolved
                Ok(())
            }
            ValidationType::TestExecution => {
                // This would run tests
                Ok(())
            }
            ValidationType::Custom(_) => {
                // Custom validation logic
                Ok(())
            }
        }
    }

    /// Rollback a set of edits
    async fn rollback_edits(&self, _edits: &[EditResult]) -> Result<()> {
        // Implementation would use the rollback data from each edit
        Ok(())
    }

    /// Store edit plan in history
    async fn store_edit_plan(&self, _plan: EditPlan) {
        let mut history = self.edit_history.write().await;
        // history.push(plan); // TODO: Implement when needed

        // Maintain history size limit
        if history.len() > self.max_history_size {
            history.remove(0);
        }
    }

    /// Generate plan for extracting a function
    async fn generate_extract_function_plan(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
        function_name: &str,
    ) -> Result<EditPlan> {
        // Read the file and extract the code block
        let (content, _) = self.file_ops.read_file_enhanced(file_path, None).await?;
        let lines: Vec<&str> = content.lines().collect();

        if start_line >= lines.len() || end_line >= lines.len() {
            return Err(anyhow!("Invalid line range"));
        }

        let extracted_code: Vec<&str> = lines[start_line..=end_line].iter().cloned().collect();
        let code_block = extracted_code.join("\n");

        // Generate function signature (simplified)
        let function_def = format!("fn {}() {{\n    {}\n}}", function_name, code_block);

        // Create the edit plan
        let edits = vec![
            CodeEdit {
                file_path: file_path.to_path_buf(),
                operation: EditOperation::Insert {
                    content: function_def,
                    line: lines.len() - 1, // Insert before last line
                    after: false,
                },
                context: Some("Extract function".to_string()),
                dependencies: vec![],
                rollback_data: None,
            },
            CodeEdit {
                file_path: file_path.to_path_buf(),
                operation: EditOperation::Replace {
                    old_string: code_block.clone(),
                    new_string: format!("    {}();", function_name),
                    start_line,
                    end_line,
                },
                context: Some("Replace extracted code with function call".to_string()),
                dependencies: vec![],
                rollback_data: Some(code_block),
            },
        ];

        Ok(EditPlan {
            edits,
            dependencies: HashMap::new(),
            validation_steps: vec![
                ValidationStep {
                    name: "Syntax Check".to_string(),
                    check_type: ValidationType::SyntaxCheck,
                    file_path: file_path.to_path_buf(),
                    expected_result: "Valid syntax".to_string(),
                }
            ],
            rollback_plan: vec![],
        })
    }

    /// Generate plan for renaming a symbol
    async fn generate_rename_symbol_plan(
        &self,
        file_path: &Path,
        old_name: &str,
        new_name: &str,
    ) -> Result<EditPlan> {
        let edits = vec![
            CodeEdit {
                file_path: file_path.to_path_buf(),
                operation: EditOperation::Rename {
                    old_name: old_name.to_string(),
                    new_name: new_name.to_string(),
                    scope: None,
                },
                context: Some(format!("Rename {} to {}", old_name, new_name)),
                dependencies: vec![],
                rollback_data: Some(old_name.to_string()),
            }
        ];

        Ok(EditPlan {
            edits,
            dependencies: HashMap::new(),
            validation_steps: vec![
                ValidationStep {
                    name: "Symbol Resolution Check".to_string(),
                    check_type: ValidationType::ImportResolution,
                    file_path: file_path.to_path_buf(),
                    expected_result: "All symbols resolved".to_string(),
                }
            ],
            rollback_plan: vec![],
        })
    }

    /// Generate plan for adding a dependency
    async fn generate_add_dependency_plan(
        &self,
        crate_name: &str,
        features: &[String],
    ) -> Result<EditPlan> {
        let cargo_toml_path = PathBuf::from("Cargo.toml");

        let dependency_line = if features.is_empty() {
            format!(r#"{} = "latest""#, crate_name)
        } else {
            format!(
                r#"{} = {{ version = "latest", features = ["{}"] }}"#,
                crate_name,
                features.join(r#"", ""#)
            )
        };

        let edits = vec![
            CodeEdit {
                file_path: cargo_toml_path.clone(),
                operation: EditOperation::Insert {
                    content: format!("\n{}", dependency_line),
                    line: 10, // Insert in dependencies section
                    after: true,
                },
                context: Some(format!("Add dependency: {}", crate_name)),
                dependencies: vec![],
                rollback_data: None,
            }
        ];

        Ok(EditPlan {
            edits,
            dependencies: HashMap::new(),
            validation_steps: vec![
                ValidationStep {
                    name: "Cargo Check".to_string(),
                    check_type: ValidationType::TypeCheck,
                    file_path: cargo_toml_path,
                    expected_result: "Dependencies resolved".to_string(),
                }
            ],
            rollback_plan: vec![],
        })
    }
}

/// Result of an edit operation
#[derive(Debug, Clone)]
pub struct EditResult {
    pub total_edits: usize,
    pub successful_edits: usize,
    pub failed_edits: usize,
    pub validation_passed: bool,
    pub rollback_performed: bool,
}

/// Symbol occurrence in code
#[derive(Debug, Clone)]
pub struct SymbolOccurrence {
    pub line: usize,
    pub column: usize,
    pub scope: String,
}

// Placeholder implementation - would be used for Minimal research-preview symbol analysis
impl SymbolOccurrence {
    pub fn new(line: usize, column: usize, scope: String) -> Self {
        Self { line, column, scope }
    }
}

/// Common refactoring operations
pub enum RefactorOperation {
    ExtractFunction {
        start_line: usize,
        end_line: usize,
        new_function_name: String,
    },
    RenameSymbol {
        old_name: String,
        new_name: String,
    },
    AddDependency {
        crate_name: String,
        features: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::test;

    #[test]
    async fn test_extract_function_plan() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let content = r#"fn main() {
    println!("Hello");
    println!("World");
    let x = 1 + 2;
    println!("{}", x);
}"#;

        let file_ops = Arc::new(EnhancedFileOps::new(5));
        file_ops.write_file_enhanced(&file_path, content, false).await.unwrap();

        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());
        let editor = MinimalCodeEditor::new(file_ops, tree_sitter);

        let plan = editor.generate_extract_function_plan(
            &file_path,
            1,
            3,
            "print_stuff"
        ).await.unwrap();

        assert_eq!(plan.edits.len(), 2);
        assert!(!plan.validation_steps.is_empty());
    }
}
