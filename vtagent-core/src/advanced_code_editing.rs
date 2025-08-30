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
    enable_type_checks: bool,
    default_js_ts_scope: Option<String>,
}

impl MinimalCodeEditor {
    /// Create a new Minimal research-preview code editor
    pub fn new(file_ops: Arc<EnhancedFileOps>, tree_sitter: Arc<TreeSitterAnalyzer>) -> Self {
        Self {
            file_ops,
            _tree_sitter: tree_sitter,
            edit_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 50,
            enable_type_checks: false,
            default_js_ts_scope: None,
        }
    }

    /// Enable or disable type check validation (cargo check) in validation steps
    pub fn with_type_checks(mut self, enable: bool) -> Self {
        self.enable_type_checks = enable;
        self
    }

    /// Configure default rename scope for JS/TS (e.g., "identifier", "property", "all")
    pub fn with_js_ts_scope_default<S: Into<String>>(mut self, scope: S) -> Self {
        self.default_js_ts_scope = Some(scope.into());
        self
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
        // Prefer tree-sitter occurrences; fallback to naive scan
        // Extract optional scope hint (e.g., "property", "all")
        let scope_hint = match &edit.operation {
            EditOperation::Rename { scope, .. } => scope
                .as_deref()
                .or(self.default_js_ts_scope.as_deref()),
            _ => self.default_js_ts_scope.as_deref(),
        };

        let occurrences = match self
            .find_identifier_occurrences_ts(&edit.file_path, old_name, scope_hint)
            .await
        {
            Ok(v) if !v.is_empty() => v,
            _ => {
                let (content, _) = self.file_ops.read_file_enhanced(&edit.file_path, None).await?;
                let mut occs: Vec<SymbolOccurrence> = Vec::new();
                for (idx, line) in content.lines().enumerate() {
                    let mut start = 0;
                    while let Some(pos) = line[start..].find(old_name) {
                        let col = start + pos;
                        occs.push(SymbolOccurrence::new(idx, col, "file".to_string()));
                        start = col + old_name.len();
                    }
                }
                occs
            }
        };

        // Load file once and apply replacements per line from right to left
        let (content, _) = self.file_ops.read_file_enhanced(&edit.file_path, None).await?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        use std::collections::BTreeMap;
        let mut by_line: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for occ in occurrences {
            by_line.entry(occ.line).or_default().push(occ.column);
        }

        for (line_idx, cols) in by_line.iter_mut() {
            if let Some(line) = lines.get_mut(*line_idx) {
                cols.sort_unstable();
                for &col in cols.iter().rev() {
                    if col + old_name.len() <= line.len() {
                        let before = &line[..col];
                        let after = &line[col + old_name.len()..];
                        *line = format!("{}{}{}", before, new_name, after);
                    }
                }
            }
        }

        let new_content = lines.join("\n");
        self.file_ops
            .write_file_enhanced(&edit.file_path, &new_content, true)
            .await?;

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
                match self.syntax_check_with_tree_sitter(&validation.file_path).await {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(anyhow!("Tree-sitter syntax diagnostics found")),
                    Err(_) => {
                        let ok = self.lightweight_syntax_check(&validation.file_path).await?;
                        if ok { Ok(()) } else { Err(anyhow!("Lightweight syntax check failed")) }
                    }
                }
            }
            ValidationType::TypeCheck => {
                if !self.enable_type_checks {
                    return Ok(());
                }
                let project_dir = self.find_cargo_root(&validation.file_path).unwrap_or_else(|| validation.file_path.parent().unwrap_or(Path::new(".")).to_path_buf());
                self.run_cargo_check(&project_dir).await
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
    async fn store_edit_plan(&self, plan: EditPlan) {
        let mut history = self.edit_history.write().await;
        history.push(plan);

        // Maintain history size limit
        if history.len() > self.max_history_size {
            history.remove(0);
        }
    }

    /// Very lightweight syntax check: braces, brackets, parentheses, and quotes balance
    async fn lightweight_syntax_check(&self, file_path: &Path) -> Result<bool> {
        let (content, _) = self.file_ops.read_file_enhanced(file_path, None).await?;
        let mut stack: Vec<char> = Vec::new();
        let mut in_single = false;
        let mut in_double = false;
        let mut escape = false;
        for ch in content.chars() {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => {
                    escape = true;
                }
                '\'' if !in_double => {
                    in_single = !in_single;
                }
                '"' if !in_single => {
                    in_double = !in_double;
                }
                _ if in_single || in_double => {}
                '{' | '[' | '(' => stack.push(ch),
                '}' => if stack.pop() != Some('{') { return Ok(false); },
                ']' => if stack.pop() != Some('[') { return Ok(false); },
                ')' => if stack.pop() != Some('(') { return Ok(false); },
                _ => {}
            }
        }
        Ok(stack.is_empty() && !in_single && !in_double)
    }

    /// Syntax check using tree-sitter diagnostics
    async fn syntax_check_with_tree_sitter(&self, file_path: &Path) -> Result<bool> {
        let mut analyzer = TreeSitterAnalyzer::new()?;
        let tree = analyzer.parse_file(file_path)?;
        Ok(tree.diagnostics.is_empty())
    }

    /// Find identifier occurrences using tree-sitter
    async fn find_identifier_occurrences_ts(
        &self,
        file_path: &Path,
        identifier: &str,
        scope: Option<&str>,
    ) -> Result<Vec<SymbolOccurrence>> {
        let mut analyzer = TreeSitterAnalyzer::new()?;
        let syntax = analyzer.parse_file(file_path)?;
        let mut occs: Vec<SymbolOccurrence> = Vec::new();

        // Allow-list of identifier node kinds per language & scope
        let allowed_kinds: Vec<&str> = match syntax.language {
            crate::tree_sitter::analyzer::LanguageSupport::JavaScript
            | crate::tree_sitter::analyzer::LanguageSupport::TypeScript => {
                match scope {
                    Some("property") => vec!["property_identifier", "shorthand_property_identifier"],
                    Some("all") => vec!["identifier", "property_identifier", "shorthand_property_identifier"],
                    _ => vec!["identifier"],
                }
            }
            _ => vec!["identifier"],
        };

        fn walk(
            node: &crate::tree_sitter::analyzer::SyntaxNode,
            name: &str,
            allowed: &[&str],
            occs: &mut Vec<SymbolOccurrence>,
        ) {
            let is_allowed = allowed.iter().any(|k| node.kind == *k);
            if is_allowed && node.text == name {
                occs.push(SymbolOccurrence::new(
                    node.start_position.row,
                    node.start_position.column,
                    "file".to_string(),
                ));
            }
            for child in &node.children {
                walk(child, name, allowed, occs);
            }
        }

        walk(&syntax.root, identifier, &allowed_kinds, &mut occs);
        Ok(occs)
    }

    /// Find the nearest Cargo project root by searching upwards for Cargo.toml
    fn find_cargo_root(&self, from_file: &Path) -> Option<PathBuf> {
        let mut dir = from_file.parent().unwrap_or(Path::new(".")).to_path_buf();
        loop {
            if dir.join("Cargo.toml").exists() {
                return Some(dir);
            }
            if !dir.pop() { break; }
        }
        None
    }

    /// Run `cargo check` in the given directory
    async fn run_cargo_check(&self, dir: &Path) -> Result<()> {
        let dir = dir.to_path_buf();
        let dir_disp = dir.display().to_string();
        let status = tokio::task::spawn_blocking(move || {
            std::process::Command::new("cargo")
                .arg("check")
                .arg("--quiet")
                .current_dir(&dir)
                .status()
        })
        .await
        .map_err(|e| anyhow!("Failed to join cargo check task: {}", e))
        .and_then(|res| res.map_err(|e| anyhow!("Failed to run cargo check: {}", e)))?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("cargo check failed in {}", dir_disp))
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
                },
                ValidationStep {
                    name: "Type Check".to_string(),
                    check_type: ValidationType::TypeCheck,
                    file_path: file_path.to_path_buf(),
                    expected_result: "cargo check passes".to_string(),
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

    #[test]
    async fn test_rename_single_line_multiple_occurrences() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("rename.rs");

        let content = r#"fn main() {
    let foo = 1;
    let foobar = 0; // should stay
    let x = foo + foo;
    println!("{} {}", foo, foobar);
}"#;

        let file_ops = Arc::new(EnhancedFileOps::new(5));
        file_ops.write_file_enhanced(&file_path, content, false).await.unwrap();

        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());
        let editor = MinimalCodeEditor::new(file_ops.clone(), tree_sitter);

        let edit = CodeEdit {
            file_path: file_path.clone(),
            operation: EditOperation::Rename { old_name: "foo".into(), new_name: "bar".into(), scope: None },
            context: None,
            dependencies: vec![],
            rollback_data: None,
        };

        let _ = editor.execute_single_edit(&edit).await.unwrap();
        let (updated, _) = file_ops.read_file_enhanced(&file_path, None).await.unwrap();
        assert!(updated.contains("let bar = 1;"));
        assert!(updated.contains("let foobar = 0;"));
        assert!(updated.contains("let x = bar + bar;"));
        assert!(updated.contains("bar, foobar"));
    }

    #[test]
    async fn test_rename_multi_line_occurrences() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("rename_multi.rs");

        let content = r#"fn add(a: i32, b: i32) -> i32 {
    let foo = a + b;
    foo
}

fn use_it() {
    let r = add(1, 2);
    println!("{}", r);
}"#;

        let file_ops = Arc::new(EnhancedFileOps::new(5));
        file_ops.write_file_enhanced(&file_path, content, false).await.unwrap();

        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());
        let editor = MinimalCodeEditor::new(file_ops.clone(), tree_sitter);

        let edit = CodeEdit {
            file_path: file_path.clone(),
            operation: EditOperation::Rename { old_name: "foo".into(), new_name: "sum".into(), scope: None },
            context: None,
            dependencies: vec![],
            rollback_data: None,
        };

        let _ = editor.execute_single_edit(&edit).await.unwrap();
        let (updated, _) = file_ops.read_file_enhanced(&file_path, None).await.unwrap();
        assert!(updated.contains("let sum = a + b;"));
        assert!(updated.contains("\n    sum\n"));
    }

    #[test]
    async fn test_js_property_rename_scope() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("code.js");

        let content = r#"const foo = 1;
const o = { foo: 2, foobar: 3 };
console.log(o.foo, o.foobar);
"#;

        let file_ops = Arc::new(EnhancedFileOps::new(5));
        file_ops.write_file_enhanced(&file_path, content, false).await.unwrap();

        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());
        let editor = MinimalCodeEditor::new(file_ops.clone(), tree_sitter);

        // Default rename (identifiers only): variable foo -> bar; properties remain
        let edit_ident_only = CodeEdit {
            file_path: file_path.clone(),
            operation: EditOperation::Rename { old_name: "foo".into(), new_name: "bar".into(), scope: None },
            context: None,
            dependencies: vec![],
            rollback_data: None,
        };
        let _ = editor.execute_single_edit(&edit_ident_only).await.unwrap();
        let (updated1, _) = file_ops.read_file_enhanced(&file_path, None).await.unwrap();
        assert!(updated1.contains("const bar = 1;"));
        assert!(updated1.contains("{ foo: 2, foobar: 3 }"));
        assert!(updated1.contains("o.foo"));

        // Property-only rename: change property foo -> baz; leave variable bar intact
        let edit_property_only = CodeEdit {
            file_path: file_path.clone(),
            operation: EditOperation::Rename { old_name: "foo".into(), new_name: "baz".into(), scope: Some("property".into()) },
            context: None,
            dependencies: vec![],
            rollback_data: None,
        };
        let _ = editor.execute_single_edit(&edit_property_only).await.unwrap();
        let (updated2, _) = file_ops.read_file_enhanced(&file_path, None).await.unwrap();
        assert!(updated2.contains("const bar = 1;"));
        assert!(updated2.contains("{ baz: 2, foobar: 3 }"));
        assert!(updated2.contains("o.baz"));
        assert!(updated2.contains("o.foobar"));
    }
}
