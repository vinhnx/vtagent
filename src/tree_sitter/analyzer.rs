//! Core tree-sitter analyzer for code parsing and analysis

use crate::tree_sitter::analysis::CodeAnalysis;
use crate::tree_sitter::languages::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use tree_sitter_swift; // TODO: Enable when Swift support is resolved
use std::path::Path;
use tree_sitter::{Language, Parser, Tree};

/// Tree-sitter analysis error
#[derive(Debug, thiserror::Error)]
pub enum TreeSitterError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("File read error: {0}")]
    FileReadError(String),

    #[error("Language detection failed: {0}")]
    LanguageDetectionError(String),

    #[error("Query execution error: {0}")]
    QueryError(String),
}

/// Language support enumeration
#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum LanguageSupport {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    // Swift, // TODO: Enable Swift support once tree-sitter-swift API is resolved
}

/// Syntax tree representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxTree {
    pub root: SyntaxNode,
    pub source_code: String,
    pub language: LanguageSupport,
    pub diagnostics: Vec<Diagnostic>,
}

/// Syntax node in the tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxNode {
    pub kind: String,
    pub start_position: Position,
    pub end_position: Position,
    pub text: String,
    pub children: Vec<SyntaxNode>,
    pub named_children: HashMap<String, Vec<SyntaxNode>>,
}

/// Position in source code
#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct Position {
    pub row: usize,
    pub column: usize,
    pub byte_offset: usize,
}

/// Diagnostic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub position: Position,
    pub node_kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}

/// Main tree-sitter analyzer
pub struct TreeSitterAnalyzer {
    parsers: HashMap<LanguageSupport, Parser>,
    supported_languages: Vec<LanguageSupport>,
    current_file: String,
}

impl TreeSitterAnalyzer {
    /// Create a new tree-sitter analyzer
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();

        // Initialize parsers for all supported languages
        let languages = vec![
            LanguageSupport::Rust,
            LanguageSupport::Python,
            LanguageSupport::JavaScript,
            LanguageSupport::TypeScript,
            LanguageSupport::Go,
            LanguageSupport::Java,
        ];

        for language in &languages {
            let mut parser = Parser::new();
            let ts_language = get_language(language.clone())?;
            parser.set_language(ts_language)?;
            parsers.insert(language.clone(), parser);
        }

        Ok(Self {
            parsers,
            supported_languages: languages,
            current_file: String::new(),
        })
    }

    /// Get supported languages
    pub fn supported_languages(&self) -> &[LanguageSupport] {
        &self.supported_languages
    }

    /// Detect language from file extension
    pub fn detect_language_from_path<P: AsRef<Path>>(&self, path: P) -> Result<LanguageSupport> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| {
                TreeSitterError::LanguageDetectionError("No file extension found".to_string())
            })?;

        match extension {
            "rs" => Ok(LanguageSupport::Rust),
            "py" => Ok(LanguageSupport::Python),
            "js" => Ok(LanguageSupport::JavaScript),
            "ts" => Ok(LanguageSupport::TypeScript),
            "tsx" => Ok(LanguageSupport::TypeScript),
            "jsx" => Ok(LanguageSupport::JavaScript),
            "go" => Ok(LanguageSupport::Go),
            "java" => Ok(LanguageSupport::Java),
            // "swift" => Ok(LanguageSupport::Swift), // TODO: Enable when API is resolved
            _ => Err(TreeSitterError::UnsupportedLanguage(extension.to_string()).into()),
        }
    }

    /// Parse source code into a syntax tree
    pub fn parse(&mut self, source_code: &str, language: LanguageSupport) -> Result<Tree> {
        let parser = self
            .parsers
            .get_mut(&language)
            .ok_or_else(|| TreeSitterError::UnsupportedLanguage(format!("{:?}", language)))?;

        let tree = parser.parse(source_code, None).ok_or_else(|| {
            TreeSitterError::ParseError("Failed to parse source code".to_string())
        })?;

        Ok(tree)
    }

    /// Extract symbols from a syntax tree
    pub fn extract_symbols(
        &self,
        _syntax_tree: &Tree,
        _source_code: &str,
        _language: LanguageSupport,
    ) -> Result<Vec<SymbolInfo>> {
        // For now, return empty vector - full implementation needs more work
        Ok(Vec::new())
    }

    /// Extract dependencies from a syntax tree
    pub fn extract_dependencies(
        &self,
        _syntax_tree: &Tree,
        _language: LanguageSupport,
    ) -> Result<Vec<crate::tree_sitter::analysis::DependencyInfo>> {
        // For now, return empty vector - full implementation needs more work
        Ok(Vec::new())
    }

    /// Calculate code metrics from a syntax tree
    pub fn calculate_metrics(
        &self,
        _syntax_tree: &Tree,
        source_code: &str,
    ) -> Result<crate::tree_sitter::analysis::CodeMetrics> {
        // For now, return basic metrics - full implementation needs more work
        Ok(crate::tree_sitter::analysis::CodeMetrics {
            lines_of_code: source_code.lines().count(),
            lines_of_comments: source_code
                .lines()
                .filter(|l| l.trim().starts_with("//") || l.trim().starts_with("/*"))
                .count(),
            blank_lines: source_code.lines().filter(|l| l.trim().is_empty()).count(),
            functions_count: 0,
            classes_count: 0,
            variables_count: 0,
            imports_count: 0,
            comment_ratio: source_code
                .lines()
                .filter(|l| l.trim().starts_with("//") || l.trim().starts_with("/*"))
                .count() as f64
                / source_code.lines().count() as f64,
        })
    }

    /// Parse file into a syntax tree
    pub fn parse_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<SyntaxTree> {
        let file_path = file_path.as_ref();
        let language = self.detect_language_from_path(file_path)?;

        let source_code = std::fs::read_to_string(file_path)
            .map_err(|e| TreeSitterError::FileReadError(e.to_string()))?;

        let tree = self.parse(&source_code, language.clone())?;

        // Convert tree-sitter tree to our SyntaxTree representation
        let root = self.convert_tree_to_syntax_node(tree.root_node(), &source_code);
        let diagnostics = self.collect_diagnostics(&tree, &source_code);

        Ok(SyntaxTree {
            root,
            source_code,
            language,
            diagnostics,
        })
    }

    /// Convert tree-sitter node to our SyntaxNode
    pub fn convert_tree_to_syntax_node(
        &self,
        node: tree_sitter::Node,
        source_code: &str,
    ) -> SyntaxNode {
        let start = node.start_position();
        let end = node.end_position();

        SyntaxNode {
            kind: node.kind().to_string(),
            start_position: Position {
                row: start.row,
                column: start.column,
                byte_offset: node.start_byte(),
            },
            end_position: Position {
                row: end.row,
                column: end.column,
                byte_offset: node.end_byte(),
            },
            text: source_code[node.start_byte()..node.end_byte()].to_string(),
            children: node
                .children(&mut node.walk())
                .map(|child| self.convert_tree_to_syntax_node(child, source_code))
                .collect(),
            named_children: self.collect_named_children(node, source_code),
        }
    }

    /// Collect named children for easier access
    fn collect_named_children(
        &self,
        node: tree_sitter::Node,
        source_code: &str,
    ) -> HashMap<String, Vec<SyntaxNode>> {
        let mut named_children = HashMap::new();

        for child in node.named_children(&mut node.walk()) {
            let kind = child.kind().to_string();
            let syntax_node = self.convert_tree_to_syntax_node(child, source_code);

            named_children
                .entry(kind)
                .or_insert_with(Vec::new)
                .push(syntax_node);
        }

        named_children
    }

    /// Collect diagnostics from the parsed tree
    pub fn collect_diagnostics(&self, tree: &Tree, _source_code: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Basic diagnostics collection - can be extended with more sophisticated analysis
        if tree.root_node().has_error() {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Error,
                message: "Syntax error detected in code".to_string(),
                position: Position {
                    row: 0,
                    column: 0,
                    byte_offset: 0,
                },
                node_kind: "root".to_string(),
            });
        }

        diagnostics
    }

    /// Get parser statistics
    pub fn get_parser_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert(
            "supported_languages".to_string(),
            self.supported_languages.len(),
        );
        stats
    }
}

/// Helper function to get tree-sitter language
fn get_language(language: LanguageSupport) -> Result<Language> {
    match language {
        LanguageSupport::Rust => Ok(tree_sitter_rust::language()),
        LanguageSupport::Python => Ok(tree_sitter_python::language()),
        LanguageSupport::JavaScript => Ok(tree_sitter_javascript::language()),
        LanguageSupport::TypeScript => Ok(tree_sitter_typescript::language_tsx()),
        LanguageSupport::Go => Ok(tree_sitter_go::language()),
        LanguageSupport::Java => Ok(tree_sitter_java::language()),
        // LanguageSupport::Swift => Ok(tree_sitter_swift::language()), // TODO: Enable when API is resolved
    }
}

impl std::fmt::Display for LanguageSupport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let language_name = match self {
            LanguageSupport::Rust => "Rust",
            LanguageSupport::Python => "Python",
            LanguageSupport::JavaScript => "JavaScript",
            LanguageSupport::TypeScript => "TypeScript",
            LanguageSupport::Go => "Go",
            LanguageSupport::Java => "Java",
            // LanguageSupport::Swift => "Swift", // TODO: Enable when API is resolved
        };
        write!(f, "{}", language_name)
    }
}

impl TreeSitterAnalyzer {
    pub fn detect_language_from_content(&self, content: &str) -> Option<LanguageSupport> {
        // Simple heuristic-based language detection
        if content.contains("fn ") && content.contains("{") && content.contains("}") {
            Some(LanguageSupport::Rust)
        } else if content.contains("def ") && content.contains(":") && !content.contains("{") {
            Some(LanguageSupport::Python)
        } else if content.contains("function") && content.contains("{") && content.contains("}") {
            Some(LanguageSupport::JavaScript)
        } else {
            None
        }
    }

    pub fn analyze_file_with_tree_sitter(
        &mut self,
        file_path: &std::path::Path,
        source_code: &str,
    ) -> Result<CodeAnalysis> {
        let language = self
            .detect_language_from_path(file_path)
            .unwrap_or_else(|_| {
                self.detect_language_from_content(source_code)
                    .unwrap_or(LanguageSupport::Rust)
            });

        self.current_file = file_path.to_string_lossy().to_string();

        let _tree = self.parse(source_code, language.clone())?;

        Ok(CodeAnalysis {
            file_path: self.current_file.clone(),
            language: language,
            symbols: vec![],      // Placeholder - would extract actual symbols
            dependencies: vec![], // Placeholder - would extract actual dependencies
            metrics: Default::default(),
            issues: vec![],
            complexity: Default::default(),
            structure: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_test_analyzer() -> TreeSitterAnalyzer {
        let mut analyzer = TreeSitterAnalyzer::new().expect("Failed to create analyzer");
        analyzer
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = create_test_analyzer();
        assert!(analyzer
            .supported_languages
            .contains(&LanguageSupport::Rust));
        assert!(analyzer
            .supported_languages
            .contains(&LanguageSupport::Python));
    }

    #[test]
    fn test_language_detection_from_path() {
        let analyzer = create_test_analyzer();

        // Test basic file extensions
        match analyzer.detect_language_from_path(Path::new("main.rs")) {
            Ok(lang) => assert_eq!(lang, LanguageSupport::Rust),
            Err(e) => panic!("Expected Rust language, got error: {}", e),
        }

        match analyzer.detect_language_from_path(Path::new("script.py")) {
            Ok(lang) => assert_eq!(lang, LanguageSupport::Python),
            Err(e) => panic!("Expected Python language, got error: {}", e),
        }

        // Test unknown extension should return error
        assert!(analyzer
            .detect_language_from_path(Path::new("file.unknown"))
            .is_err());
    }

    #[test]
    fn test_language_detection_from_content() {
        let analyzer = create_test_analyzer();

        // Test Rust content
        let rust_code = r#"fn main() { println!("Hello, world!"); let x = 42; }"#;
        assert_eq!(
            analyzer.detect_language_from_content(rust_code),
            Some(LanguageSupport::Rust)
        );

        // Test Python content
        let python_code = r#"def main(): print("Hello, world!"); x = 42"#;
        assert_eq!(
            analyzer.detect_language_from_content(python_code),
            Some(LanguageSupport::Python)
        );

        // Test unknown content
        let unknown_code = "This is not code just plain text.";
        assert_eq!(analyzer.detect_language_from_content(unknown_code), None);
    }

    #[test]
    fn test_parse_rust_code() {
        let mut analyzer = create_test_analyzer();

        let rust_code = r#"fn main() { println!("Hello, world!"); let x = 42; }"#;

        let result = analyzer.parse(rust_code, LanguageSupport::Rust);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert!(!tree.root_node().has_error());
    }
}
