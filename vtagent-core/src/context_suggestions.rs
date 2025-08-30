//! Context-Aware Code Suggestions Module
//!
//! This module provides intelligent code suggestions based on:
//! - Code analysis and patterns
//! - Context-aware completion
//! - Best practices and conventions
//! - Performance optimizations
//! - Security recommendations

use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::enhanced_file_ops::EnhancedFileOps;
use crate::tree_sitter::{LanguageSupport, TreeSitterAnalyzer};

/// Code suggestion with context and confidence
#[derive(Debug, Clone)]
pub struct CodeSuggestion {
    pub suggestion_type: SuggestionType,
    pub title: String,
    pub description: String,
    pub code_changes: Vec<CodeChange>,
    pub confidence: f64, // 0.0 to 1.0
    pub impact: SuggestionImpact,
    pub rationale: String,
    pub file_path: PathBuf,
    pub line_range: Option<(usize, usize)>,
}

/// Type of code suggestion
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionType {
    BugFix,
    Performance,
    Security,
    CodeStyle,
    BestPractice,
    Refactoring,
    Completion,
    Documentation,
}

/// Impact level of the suggestion
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionImpact {
    Low,
    Medium,
    High,
    Critical,
}

/// Specific code change in a suggestion
#[derive(Debug, Clone)]
pub struct CodeChange {
    pub change_type: ChangeType,
    pub old_code: Option<String>,
    pub new_code: String,
    pub line_number: usize,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    Replace,
    Insert,
    Delete,
    Comment,
}

/// Context-aware suggestion engine
pub struct ContextSuggestionEngine {
    file_ops: Arc<EnhancedFileOps>,
    tree_sitter: Arc<TreeSitterAnalyzer>,
    suggestion_rules: HashMap<LanguageSupport, Vec<SuggestionRule>>,
    suggestion_cache: HashMap<PathBuf, Vec<CodeSuggestion>>,
}

impl ContextSuggestionEngine {
    /// Create a new context suggestion engine
    pub fn new(file_ops: Arc<EnhancedFileOps>, tree_sitter: Arc<TreeSitterAnalyzer>) -> Self {
        let mut engine = Self {
            file_ops,
            tree_sitter,
            suggestion_rules: HashMap::new(),
            suggestion_cache: HashMap::new(),
        };

        engine.initialize_default_rules();
        engine
    }

    /// Generate context-aware suggestions for a file
    pub async fn generate_suggestions(&self, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
        // Check cache first
        if let Some(cached) = self.suggestion_cache.get(file_path) {
            return Ok(cached.clone());
        }

        let mut suggestions = Vec::new();

        // Read and analyze the file
        let (content, _) = self.file_ops.read_file_enhanced(file_path, None).await?;

        // Determine language
        let language = self.tree_sitter.detect_language_from_path(file_path)?;

        // Get applicable rules
        if let Some(rules) = self.suggestion_rules.get(&language) {
            for rule in rules {
                if let Ok(rule_suggestions) = rule.apply(&content, file_path).await {
                    suggestions.extend(rule_suggestions);
                }
            }
        }

        // Sort by confidence and impact
        suggestions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| match (&a.impact, &b.impact) {
                    (SuggestionImpact::Critical, _) => std::cmp::Ordering::Less,
                    (_, SuggestionImpact::Critical) => std::cmp::Ordering::Greater,
                    (SuggestionImpact::High, _) => std::cmp::Ordering::Less,
                    (_, SuggestionImpact::High) => std::cmp::Ordering::Greater,
                    (SuggestionImpact::Medium, _) => std::cmp::Ordering::Less,
                    (_, SuggestionImpact::Medium) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                })
        });

        // Cache the results (would need to change method signature for mutability)
        // self.suggestion_cache.insert(file_path.to_path_buf(), suggestions.clone());

        Ok(suggestions)
    }

    /// Generate suggestions for code completion
    pub async fn generate_completion_suggestions(
        &self,
        file_path: &Path,
        line: usize,
        column: usize,
        prefix: &str,
    ) -> Result<Vec<CodeSuggestion>> {
        let (content, _) = self.file_ops.read_file_enhanced(file_path, None).await?;
        let lines: Vec<&str> = content.lines().collect();

        if line >= lines.len() {
            return Ok(Vec::new());
        }

        let current_line = lines[line];
        let language = self.tree_sitter.detect_language_from_path(file_path)?;

        let mut suggestions = Vec::new();

        match language {
            LanguageSupport::Rust => {
                suggestions.extend(
                    self.generate_rust_completions(current_line, prefix, line, column)
                        .await?,
                );
            }
            LanguageSupport::Python => {
                suggestions.extend(
                    self.generate_python_completions(current_line, prefix, line, column)
                        .await?,
                );
            }
            LanguageSupport::JavaScript | LanguageSupport::TypeScript => {
                suggestions.extend(
                    self.generate_js_completions(current_line, prefix, line, column)
                        .await?,
                );
            }
            _ => {} // Generic completions could be added here
        }

        Ok(suggestions)
    }

    /// Analyze code patterns and suggest improvements
    pub async fn analyze_code_patterns(&self, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
        let (content, _) = self.file_ops.read_file_enhanced(file_path, None).await?;
        let language = self.tree_sitter.detect_language_from_path(file_path)?;

        let mut suggestions = Vec::new();

        match language {
            LanguageSupport::Rust => {
                suggestions.extend(self.analyze_rust_patterns(&content, file_path).await?);
            }
            LanguageSupport::Python => {
                suggestions.extend(self.analyze_python_patterns(&content, file_path).await?);
            }
            _ => {}
        }

        Ok(suggestions)
    }

    /// Clear suggestion cache
    pub fn clear_cache(&mut self) {
        self.suggestion_cache.clear();
    }

    /// Initialize default suggestion rules
    fn initialize_default_rules(&mut self) {
        // Rust rules
        let rust_rules = vec![
            SuggestionRule::new(
                "unused_imports",
                SuggestionType::CodeStyle,
                0.8,
                Box::new(rust_unused_imports_rule),
            ),
            SuggestionRule::new(
                "missing_docs",
                SuggestionType::Documentation,
                0.6,
                Box::new(rust_missing_docs_rule),
            ),
            SuggestionRule::new(
                "unwrap_usage",
                SuggestionType::BestPractice,
                0.7,
                Box::new(rust_unwrap_usage_rule),
            ),
            SuggestionRule::new(
                "large_function",
                SuggestionType::Refactoring,
                0.5,
                Box::new(rust_large_function_rule),
            ),
        ];
        self.suggestion_rules
            .insert(LanguageSupport::Rust, rust_rules);

        // Python rules
        let python_rules = vec![
            SuggestionRule::new(
                "type_hints",
                SuggestionType::BestPractice,
                0.6,
                Box::new(python_type_hints_rule),
            ),
            SuggestionRule::new(
                "docstrings",
                SuggestionType::Documentation,
                0.5,
                Box::new(python_docstrings_rule),
            ),
            SuggestionRule::new(
                "exception_handling",
                SuggestionType::BestPractice,
                0.7,
                Box::new(python_exception_handling_rule),
            ),
        ];
        self.suggestion_rules
            .insert(LanguageSupport::Python, python_rules);

        // JavaScript/TypeScript rules
        let _js_rules = [
            SuggestionRule::new(
                "const_usage",
                SuggestionType::BestPractice,
                0.6,
                Box::new(js_const_usage_rule),
            ),
            SuggestionRule::new(
                "async_await",
                SuggestionType::BestPractice,
                0.8,
                Box::new(js_async_await_rule),
            ),
            SuggestionRule::new(
                "error_handling",
                SuggestionType::BestPractice,
                0.7,
                Box::new(js_error_handling_rule),
            ),
        ];
        self.suggestion_rules
            .insert(LanguageSupport::JavaScript, Vec::new());
        self.suggestion_rules
            .insert(LanguageSupport::TypeScript, Vec::new());
    }

    /// Generate Rust-specific completion suggestions
    async fn generate_rust_completions(
        &self,
        _current_line: &str,
        prefix: &str,
        line: usize,
        _column: usize,
    ) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();

        // Common Rust patterns
        let rust_patterns = [
            (
                "println!",
                "println!(\"{}\", );",
                "Print formatted text to stdout",
            ),
            ("vec!", "vec![", "Create a new vector"),
            ("Some(", "Some(value)", "Wrap value in Option::Some"),
            ("Ok(", "Ok(value)", "Wrap value in Result::Ok"),
            ("Err(", "Err(error)", "Wrap error in Result::Err"),
            (
                "match ",
                "match value {\n    pattern => result,\n    _ => default,\n}",
                "Pattern matching",
            ),
            (
                "if let ",
                "if let Some(value) = option {\n    // use value\n}",
                "Pattern matching with if let",
            ),
            (
                "while let ",
                "while let Some(value) = iterator.next() {\n    // process value\n}",
                "Iterate with pattern matching",
            ),
        ];

        for (trigger, completion, description) in &rust_patterns {
            if trigger.starts_with(prefix) || prefix.is_empty() {
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::Completion,
                    title: format!("Complete: {}", trigger),
                    description: description.to_string(),
                    code_changes: vec![CodeChange {
                        change_type: ChangeType::Insert,
                        old_code: None,
                        new_code: completion.to_string(),
                        line_number: line,
                        context: format!("Completing '{}'", prefix),
                    }],
                    confidence: 0.8,
                    impact: SuggestionImpact::Low,
                    rationale: format!("Common Rust pattern completion for '{}'", trigger),
                    file_path: PathBuf::new(), // Will be set by caller
                    line_range: Some((line, line)),
                });
            }
        }

        Ok(suggestions)
    }

    /// Generate Python-specific completion suggestions
    async fn generate_python_completions(
        &self,
        _current_line: &str,
        prefix: &str,
        line: usize,
        _column: usize,
    ) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();

        let python_patterns = [
            ("def ", "def function_name(parameters):\n    \"\"\"Docstring\"\"\"\n    pass", "Function definition"),
            ("class ", "class ClassName:\n    \"\"\"Docstring\"\"\"\n    \n    def __init__(self):\n        pass", "Class definition"),
            ("if __name__", "if __name__ == \"__main__\":\n    main()", "Main guard"),
            ("with open(", "with open(\"filename.txt\", \"r\") as f:\n    content = f.read()", "File handling"),
            ("try:", "try:\n    # risky code\nexcept Exception as e:\n    print(f\"Error: {e}\")", "Exception handling"),
            ("list(", "list_comprehension = [x for x in iterable if condition]", "List comprehension"),
            ("dict(", "dict_comprehension = {key: value for item in iterable}", "Dictionary comprehension"),
        ];

        for (trigger, completion, description) in &python_patterns {
            if trigger.starts_with(prefix) || prefix.is_empty() {
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::Completion,
                    title: format!("Complete: {}", trigger),
                    description: description.to_string(),
                    code_changes: vec![CodeChange {
                        change_type: ChangeType::Insert,
                        old_code: None,
                        new_code: completion.to_string(),
                        line_number: line,
                        context: format!("Completing '{}'", prefix),
                    }],
                    confidence: 0.8,
                    impact: SuggestionImpact::Low,
                    rationale: format!("Common Python pattern completion for '{}'", trigger),
                    file_path: PathBuf::new(),
                    line_range: Some((line, line)),
                });
            }
        }

        Ok(suggestions)
    }

    /// Generate JavaScript/TypeScript completion suggestions
    async fn generate_js_completions(
        &self,
        _current_line: &str,
        prefix: &str,
        line: usize,
        _column: usize,
    ) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();

        let js_patterns = [
            ("function ", "function functionName(parameters) {\n    // implementation\n}", "Function declaration"),
            ("const ", "const variableName = value;", "Constant declaration"),
            ("let ", "let variableName = value;", "Variable declaration"),
            ("async function", "async function functionName(parameters) {\n    try {\n        // async code\n    } catch (error) {\n        console.error(error);\n    }\n}", "Async function"),
            ("try {", "try {\n    // risky code\n} catch (error) {\n    console.error(error);\n} finally {\n    // cleanup\n}", "Try-catch-finally"),
            ("=> ", "(parameters) => {\n    // implementation\n}", "Arrow function"),
            ("class ", "class ClassName {\n    constructor(parameters) {\n        // initialization\n    }\n    \n    methodName() {\n        // implementation\n    }\n}", "Class definition"),
        ];

        for (trigger, completion, description) in &js_patterns {
            if trigger.starts_with(prefix) || prefix.is_empty() {
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::Completion,
                    title: format!("Complete: {}", trigger),
                    description: description.to_string(),
                    code_changes: vec![CodeChange {
                        change_type: ChangeType::Insert,
                        old_code: None,
                        new_code: completion.to_string(),
                        line_number: line,
                        context: format!("Completing '{}'", prefix),
                    }],
                    confidence: 0.8,
                    impact: SuggestionImpact::Low,
                    rationale: format!("Common JavaScript pattern completion for '{}'", trigger),
                    file_path: PathBuf::new(),
                    line_range: Some((line, line)),
                });
            }
        }

        Ok(suggestions)
    }

    /// Analyze Rust code patterns
    async fn analyze_rust_patterns(
        &self,
        content: &str,
        file_path: &Path,
    ) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Check for common Rust anti-patterns
        for (i, line) in lines.iter().enumerate() {
            // Check for unwrap() usage
            if line.contains(".unwrap()") && !line.contains("#[allow") {
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::BestPractice,
                    title: "Replace unwrap() with proper error handling".to_string(),
                    description: "Using unwrap() can cause panics. Consider using ? operator or proper error handling.".to_string(),
                    code_changes: vec![], // Would need more context to suggest replacement
                    confidence: 0.8,
                    impact: SuggestionImpact::Medium,
                    rationale: "unwrap() can cause unexpected panics in production code".to_string(),
                    file_path: file_path.to_path_buf(),
                    line_range: Some((i, i)),
                });
            }

            // Check for TODO comments
            if line.contains("TODO") || line.contains("FIXME") {
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::BestPractice,
                    title: "Address TODO comment".to_string(),
                    description: "TODO comments indicate incomplete work that should be addressed."
                        .to_string(),
                    code_changes: vec![],
                    confidence: 0.6,
                    impact: SuggestionImpact::Low,
                    rationale: "TODO comments should be resolved or converted to proper issues"
                        .to_string(),
                    file_path: file_path.to_path_buf(),
                    line_range: Some((i, i)),
                });
            }
        }

        Ok(suggestions)
    }

    /// Analyze Python code patterns
    async fn analyze_python_patterns(
        &self,
        content: &str,
        file_path: &Path,
    ) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            // Check for bare except clauses
            if line.trim().starts_with("except:") || line.trim().starts_with("except :") {
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::BestPractice,
                    title: "Avoid bare except clauses".to_string(),
                    description: "Bare except clauses catch all exceptions including system exits. Specify exception types.".to_string(),
                    code_changes: vec![],
                    confidence: 0.9,
                    impact: SuggestionImpact::High,
                    rationale: "Bare except clauses can hide important errors and make debugging difficult".to_string(),
                    file_path: file_path.to_path_buf(),
                    line_range: Some((i, i)),
                });
            }

            // Check for print statements (suggest logging)
            if line.contains("print(") && !line.contains("#") {
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::BestPractice,
                    title: "Consider using logging instead of print".to_string(),
                    description: "For production code, consider using the logging module instead of print statements.".to_string(),
                    code_changes: vec![],
                    confidence: 0.5,
                    impact: SuggestionImpact::Low,
                    rationale: "Logging provides better control over output levels and destinations".to_string(),
                    file_path: file_path.to_path_buf(),
                    line_range: Some((i, i)),
                });
            }
        }

        Ok(suggestions)
    }
}

/// Suggestion rule with application logic
pub struct SuggestionRule {
    pub name: String,
    pub suggestion_type: SuggestionType,
    pub confidence: f64,
    pub apply_fn: Box<dyn Fn(&str, &Path) -> Result<Vec<CodeSuggestion>> + Send + Sync>,
}

impl SuggestionRule {
    pub fn new<F>(name: &str, suggestion_type: SuggestionType, confidence: f64, apply_fn: F) -> Self
    where
        F: Fn(&str, &Path) -> Result<Vec<CodeSuggestion>> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            suggestion_type,
            confidence,
            apply_fn: Box::new(apply_fn),
        }
    }

    pub async fn apply(&self, content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
        (self.apply_fn)(content, file_path)
    }
}

// Rule implementations for different languages

fn rust_unused_imports_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();

    // Simple heuristic: look for use statements that might be unused
    let use_regex = Regex::new(r"use\s+([^;]+);").unwrap();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("use ") {
            if let Some(captures) = use_regex.captures(line) {
                if let Some(import_path) = captures.get(1) {
                    let import = import_path.as_str();

                    // Check if this import is used elsewhere in the file
                    let mut used = false;
                    for other_line in &lines {
                        if other_line.contains(import) && other_line != line {
                            used = true;
                            break;
                        }
                    }

                    if !used {
                        suggestions.push(CodeSuggestion {
                            suggestion_type: SuggestionType::CodeStyle,
                            title: format!("Remove unused import: {}", import),
                            description: "This import appears to be unused and can be safely removed.".to_string(),
                            code_changes: vec![CodeChange {
                                change_type: ChangeType::Delete,
                                old_code: Some(line.to_string()),
                                new_code: String::new(),
                                line_number: i,
                                context: "Unused import".to_string(),
                            }],
                            confidence: 0.7,
                            impact: SuggestionImpact::Low,
                            rationale: "Removing unused imports improves code clarity and reduces compilation time".to_string(),
                            file_path: file_path.to_path_buf(),
                            line_range: Some((i, i)),
                        });
                    }
                }
            }
        }
    }

    Ok(suggestions)
}

fn rust_missing_docs_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("pub fn ") || line.trim().starts_with("pub struct ") {
            // Check if there's a doc comment above
            let mut has_docs = false;
            if i > 0 {
                let prev_line = lines[i - 1].trim();
                if prev_line.starts_with("///") || prev_line.starts_with("//!") {
                    has_docs = true;
                }
            }

            if !has_docs {
                let item_type = if line.contains("fn ") {
                    "function"
                } else {
                    "struct"
                };
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::Documentation,
                    title: format!("Add documentation for public {}", item_type),
                    description: format!(
                        "Public {} should be documented with /// comments.",
                        item_type
                    ),
                    code_changes: vec![CodeChange {
                        change_type: ChangeType::Insert,
                        old_code: None,
                        new_code: format!("/// TODO: Add documentation\n{}", line),
                        line_number: i,
                        context: format!("Missing docs for public {}", item_type),
                    }],
                    confidence: 0.6,
                    impact: SuggestionImpact::Low,
                    rationale: "Public APIs should be documented for better developer experience"
                        .to_string(),
                    file_path: file_path.to_path_buf(),
                    line_range: Some((i, i)),
                });
            }
        }
    }

    Ok(suggestions)
}

fn rust_unwrap_usage_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains(".unwrap()") && !line.contains("#[allow") {
            suggestions.push(CodeSuggestion {
                suggestion_type: SuggestionType::BestPractice,
                title: "Replace unwrap() with proper error handling".to_string(),
                description:
                    "Using unwrap() can cause panics. Consider using ? operator or match statement."
                        .to_string(),
                code_changes: vec![], // Would need AST analysis for proper replacement
                confidence: 0.8,
                impact: SuggestionImpact::Medium,
                rationale: "unwrap() can cause unexpected panics in production code".to_string(),
                file_path: file_path.to_path_buf(),
                line_range: Some((i, i)),
            });
        }
    }

    Ok(suggestions)
}

fn rust_large_function_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut function_start = None;
    let mut brace_count = 0;

    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("fn ") {
            function_start = Some(i);
            brace_count = 0;
        }

        if let Some(start) = function_start {
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;

            // If we hit the end of the function
            if brace_count == 0 && start != i {
                let function_length = i - start;
                if function_length > 50 {
                    // Arbitrary threshold
                    suggestions.push(CodeSuggestion {
                        suggestion_type: SuggestionType::Refactoring,
                        title: "Consider breaking down large function".to_string(),
                        description: format!("This function spans {} lines and might benefit from being split into smaller functions.", function_length),
                        code_changes: vec![], // Would need more sophisticated analysis
                        confidence: 0.5,
                        impact: SuggestionImpact::Medium,
                        rationale: "Large functions are harder to understand and maintain".to_string(),
                        file_path: file_path.to_path_buf(),
                        line_range: Some((start, i)),
                    });
                }
                function_start = None;
            }
        }
    }

    Ok(suggestions)
}

fn python_type_hints_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("def ") {
            // Check if function has type hints
            if !line.contains(" -> ") && line.matches(':').count() < 2 {
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::BestPractice,
                    title: "Add type hints to function".to_string(),
                    description:
                        "Consider adding type hints to function parameters and return type."
                            .to_string(),
                    code_changes: vec![], // Would need more sophisticated analysis
                    confidence: 0.6,
                    impact: SuggestionImpact::Low,
                    rationale:
                        "Type hints improve code readability and catch type-related errors early"
                            .to_string(),
                    file_path: file_path.to_path_buf(),
                    line_range: Some((i, i)),
                });
            }
        }
    }

    Ok(suggestions)
}

fn python_docstrings_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("def ") || line.trim().starts_with("class ") {
            // Check if next few lines contain docstring
            let mut has_docstring = false;
            for j in (i + 1)..std::cmp::min(i + 5, lines.len()) {
                let next_line = lines[j].trim();
                if next_line.starts_with("\"\"\"") || next_line.starts_with("'''") {
                    has_docstring = true;
                    break;
                } else if !next_line.is_empty() && !next_line.starts_with("#") {
                    // Non-empty, non-comment line before docstring
                    break;
                }
            }

            if !has_docstring {
                let item_type = if line.contains("def ") {
                    "function"
                } else {
                    "class"
                };
                suggestions.push(CodeSuggestion {
                    suggestion_type: SuggestionType::Documentation,
                    title: format!("Add docstring to {}", item_type),
                    description: format!(
                        "{} should have a docstring describing its purpose and parameters.",
                        item_type
                    ),
                    code_changes: vec![], // Would need more sophisticated insertion
                    confidence: 0.5,
                    impact: SuggestionImpact::Low,
                    rationale: "Docstrings improve code documentation and developer experience"
                        .to_string(),
                    file_path: file_path.to_path_buf(),
                    line_range: Some((i, i)),
                });
            }
        }
    }

    Ok(suggestions)
}

fn python_exception_handling_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("except:") || line.trim().starts_with("except :") {
            suggestions.push(CodeSuggestion {
                suggestion_type: SuggestionType::BestPractice,
                title: "Avoid bare except clauses".to_string(),
                description: "Bare except clauses catch all exceptions. Specify the exception types you want to catch.".to_string(),
                code_changes: vec![], // Would need more context for proper replacement
                confidence: 0.9,
                impact: SuggestionImpact::High,
                rationale: "Bare except clauses can hide important errors and make debugging difficult".to_string(),
                file_path: file_path.to_path_buf(),
                line_range: Some((i, i)),
            });
        }
    }

    Ok(suggestions)
}

fn js_const_usage_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains("var ") && !line.contains("for(var ") {
            suggestions.push(CodeSuggestion {
                suggestion_type: SuggestionType::BestPractice,
                title: "Use const or let instead of var".to_string(),
                description: "Prefer const for variables that don't change, let for those that do."
                    .to_string(),
                code_changes: vec![], // Would need more sophisticated replacement
                confidence: 0.7,
                impact: SuggestionImpact::Low,
                rationale: "Modern JavaScript best practices prefer const/let over var".to_string(),
                file_path: file_path.to_path_buf(),
                line_range: Some((i, i)),
            });
        }
    }

    Ok(suggestions)
}

fn js_async_await_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains(".then(") || line.contains(".catch(") {
            suggestions.push(CodeSuggestion {
                suggestion_type: SuggestionType::BestPractice,
                title: "Consider using async/await instead of promises".to_string(),
                description: "async/await syntax is generally more readable than promise chains."
                    .to_string(),
                code_changes: vec![], // Would need AST analysis for proper conversion
                confidence: 0.6,
                impact: SuggestionImpact::Medium,
                rationale: "async/await improves code readability and error handling".to_string(),
                file_path: file_path.to_path_buf(),
                line_range: Some((i, i)),
            });
        }
    }

    Ok(suggestions)
}

fn js_error_handling_rule(content: &str, file_path: &Path) -> Result<Vec<CodeSuggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.contains("console.error(") {
            suggestions.push(CodeSuggestion {
                suggestion_type: SuggestionType::BestPractice,
                title: "Consider proper error handling instead of console.error".to_string(),
                description: "For production code, consider throwing errors or using a proper logging framework.".to_string(),
                code_changes: vec![], // Would need more context
                confidence: 0.5,
                impact: SuggestionImpact::Low,
                rationale: "Proper error handling improves application reliability".to_string(),
                file_path: file_path.to_path_buf(),
                line_range: Some((i, i)),
            });
        }
    }

    Ok(suggestions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::test;

    #[test]
    async fn test_suggestion_engine_creation() {
        let _temp_dir = TempDir::new().unwrap();
        let file_ops = Arc::new(EnhancedFileOps::new(5));
        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());

        let engine = ContextSuggestionEngine::new(file_ops, tree_sitter);

        // Test that default rules are loaded
        assert!(engine.suggestion_rules.contains_key(&LanguageSupport::Rust));
        assert!(engine
            .suggestion_rules
            .contains_key(&LanguageSupport::Python));
    }

    #[test]
    async fn test_rust_completion_suggestions() {
        let _temp_dir = TempDir::new().unwrap();
        let file_ops = Arc::new(EnhancedFileOps::new(5));
        let tree_sitter = Arc::new(TreeSitterAnalyzer::new().unwrap());

        let engine = ContextSuggestionEngine::new(file_ops, tree_sitter);

        let suggestions = engine
            .generate_rust_completions("print", "print", 1, 5)
            .await
            .unwrap();

        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.title.contains("println!")));
    }

    #[test]
    async fn test_unused_imports_rule() {
        let content = r#"use std::collections::HashMap;
use std::fs;

fn main() {
    println!("Hello");
}"#;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let suggestions = rust_unused_imports_rule(content, &file_path).unwrap();

        // Should suggest removing unused HashMap import
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.title.contains("HashMap")));
    }
}
