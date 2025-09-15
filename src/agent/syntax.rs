//! Enhanced syntax highlighting module for VTAgent
//!
//! This module provides advanced syntax highlighting capabilities using syntect,
//! with theme caching, custom theme loading, and performance optimizations.

use anyhow::{Context, Result};
use std::io::BufRead;
use std::path::Path;
use std::sync::Arc;
use once_cell::sync::Lazy;

// Syntax highlighting imports
use syntect::dumps::{dump_to_file, from_dump_file};
use syntect::easy::{HighlightFile, HighlightLines};
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use vtagent_core::config::loader::VTAgentConfig;
use vtagent_core::tools::tree_sitter::analyzer::{LanguageSupport, TreeSitterAnalyzer};

/// Enhanced syntax highlighter with theme caching and performance optimizations
#[derive(Clone)]
pub struct EnhancedSyntaxHighlighter {
    syntax_set: Arc<SyntaxSet>,
    theme: Arc<Theme>,
    theme_name: String,
}

impl EnhancedSyntaxHighlighter {
    /// Create a new enhanced syntax highlighter with caching
    pub fn new(theme_name: Option<&str>) -> Result<Self> {
        let syntax_set = Arc::new(SyntaxSet::load_defaults_newlines());
        let theme_name = theme_name.unwrap_or("base16-ocean.dark");

        // Try to load theme with caching
        let theme = Self::load_theme_cached(theme_name)?;

        Ok(Self {
            syntax_set,
            theme: Arc::new(theme),
            theme_name: theme_name.to_string(),
        })
    }

    /// Load theme with caching for better performance
    fn load_theme_cached(theme_name: &str) -> Result<Theme> {
        let ts = ThemeSet::load_defaults();

        // First try embedded themes
        if let Some(theme) = ts.themes.get(theme_name) {
            return Ok(theme.clone());
        }

        // Try to load from file with caching
        let theme_path = Path::new(theme_name);
        if theme_path.exists() {
            let cache_path = theme_path.with_extension("tmdump");

            if cache_path.exists() {
                // Load from cache
                match from_dump_file(&cache_path) {
                    Ok(theme) => return Ok(theme),
                    Err(_) => {
                        // Cache corrupted, remove it and fall back to loading from file
                        let _ = std::fs::remove_file(&cache_path);
                    }
                }
            }

            // Load from file and cache it
            let theme = ThemeSet::get_theme(theme_path)
                .with_context(|| format!("Failed to load theme from: {}", theme_name))?;

            // Cache the theme for future use
            if let Err(e) = dump_to_file(&theme, &cache_path) {
                eprintln!("Warning: Failed to cache theme: {}", e);
            }

            Ok(theme)
        } else {
            // Fall back to default theme
            ts.themes
                .get("base16-ocean.dark")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Default theme not found"))
        }
    }

    /// Highlight code content with enhanced performance
    pub fn highlight_code(&self, content: &str) -> Result<String> {
        // Handle empty content
        if content.trim().is_empty() {
            return Ok(content.to_string());
        }

        // Detect language from content patterns
        let syntax = self.detect_language(content);

        // Use HighlightLines for string content (more efficient than HighlightFile)
        let mut highlighter = HighlightLines::new(syntax, &self.theme);

        let mut highlighted = String::new();

        // Process line by line for better memory efficiency
        for line in LinesWithEndings::from(content) {
            let regions: Vec<(Style, &str)> = highlighter.highlight_line(line, &self.syntax_set)?;
            let escaped = as_24_bit_terminal_escaped(&regions[..], false);
            highlighted.push_str(&escaped);
        }

        // Ensure proper ANSI reset
        highlighted.push_str("\x1b[0m");

        Ok(highlighted)
    }

    /// Highlight file content using HighlightFile for better performance
    pub fn highlight_file(&self, file_path: &Path) -> Result<String> {
        let mut highlighter = HighlightFile::new(file_path, &self.syntax_set, &self.theme)
            .with_context(|| format!("Failed to create highlighter for file: {:?}", file_path))?;

        let mut highlighted = String::new();
        let mut line = String::new();

        // Read and highlight line by line
        while highlighter.reader.read_line(&mut line)? > 0 {
            let regions: Vec<(Style, &str)> = highlighter
                .highlight_lines
                .highlight_line(&line, &self.syntax_set)?;

            let escaped = as_24_bit_terminal_escaped(&regions[..], true);
            highlighted.push_str(&escaped);
            line.clear();
        }

        // Ensure proper ANSI reset
        highlighted.push_str("\x1b[0m");

        Ok(highlighted)
    }

    /// Detect language from content patterns (enhanced version with tree-sitter)
    fn detect_language<'a>(&'a self, content: &str) -> &'a syntect::parsing::SyntaxReference {
        // First try tree-sitter for more accurate detection
        if let Some(ts_language) = self.detect_language_with_tree_sitter(content) {
            match ts_language {
                LanguageSupport::Rust => {
                    return self.syntax_set.find_syntax_by_extension("rs").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
                }
                LanguageSupport::Python => {
                    return self.syntax_set.find_syntax_by_extension("py").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
                }
                LanguageSupport::JavaScript => {
                    return self.syntax_set.find_syntax_by_extension("js").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
                }
                LanguageSupport::TypeScript => {
                    return self.syntax_set.find_syntax_by_extension("ts").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
                }
                LanguageSupport::Go => {
                    return self.syntax_set.find_syntax_by_extension("go").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
                }
                LanguageSupport::Java => {
                    return self.syntax_set.find_syntax_by_extension("java").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
                }
                LanguageSupport::Swift => {
                    return self.syntax_set.find_syntax_by_extension("swift").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
                }
            }
        }

        // Fall back to pattern-based detection
        self.detect_language_with_patterns(content)
    }

    /// Use tree-sitter for accurate language detection
    fn detect_language_with_tree_sitter(&self, content: &str) -> Option<LanguageSupport> {
        // Try parsing with different tree-sitter parsers
        let languages = [
            LanguageSupport::Rust,
            LanguageSupport::Python,
            LanguageSupport::JavaScript,
            LanguageSupport::TypeScript,
            LanguageSupport::Go,
            LanguageSupport::Java,
            LanguageSupport::Swift,
        ];

        if let Ok(mut analyzer) = TreeSitterAnalyzer::new() {
            for language in &languages {
                if let Ok(_) = analyzer.parse(content, *language) {
                    // If parsing succeeds with minimal errors, this is likely the correct language
                    return Some(*language);
                }
            }
        }

        None
    }

    /// Detect language using pattern matching (fallback method)
    fn detect_language_with_patterns<'a>(&'a self, content: &str) -> &'a syntect::parsing::SyntaxReference {
        // More sophisticated language detection
        let content_lower = content.to_lowercase();

        // Check for shebang lines first
        if content.starts_with("#!") {
            if content.contains("python") {
                return self.syntax_set.find_syntax_by_extension("py").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
            } else if content.contains("bash") || content.contains("sh") {
                return self.syntax_set.find_syntax_by_extension("sh").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
            } else if content.contains("ruby") {
                return self.syntax_set.find_syntax_by_extension("rb").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
            } else if content.contains("node") {
                return self.syntax_set.find_syntax_by_extension("js").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
            }
        }

        // Check for common language patterns
        if content.contains("fn ") && content.contains("let ") && content.contains("use ") {
            self.syntax_set.find_syntax_by_extension("rs").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if (content.contains("def ") || content.contains("import ")) && (content.contains("class ") || content.contains(":")) {
            self.syntax_set.find_syntax_by_extension("py").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("function") || content.contains("const ") || content.contains("let ") || content.contains("=>") {
            self.syntax_set.find_syntax_by_extension("js").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("package ") || content.contains("func ") || content.contains("import ") {
            self.syntax_set.find_syntax_by_extension("go").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("public class") || content.contains("import java") {
            self.syntax_set.find_syntax_by_extension("java").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("#include") || content.contains("int main") {
            self.syntax_set.find_syntax_by_extension("c").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("#include") && content.contains("using namespace") {
            self.syntax_set.find_syntax_by_extension("cpp").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("<?php") {
            self.syntax_set.find_syntax_by_extension("php").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("<!DOCTYPE html") || content.contains("<html") {
            self.syntax_set.find_syntax_by_extension("html").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("SELECT ") || content.contains("FROM ") || content.contains("WHERE ") {
            self.syntax_set.find_syntax_by_extension("sql").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else if content.contains("interface ") || content.contains("class ") || content.contains("namespace ") {
            self.syntax_set.find_syntax_by_extension("cs").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        } else {
            // Try to detect by file extension patterns in content
            if content_lower.contains(".rs\"") || content_lower.contains(".rs ") {
                self.syntax_set.find_syntax_by_extension("rs").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
            } else if content_lower.contains(".py\"") || content_lower.contains(".py ") {
                self.syntax_set.find_syntax_by_extension("py").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
            } else if content_lower.contains(".js\"") || content_lower.contains(".js ") {
                self.syntax_set.find_syntax_by_extension("js").unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
            } else {
                // Default to plain text if no language detected
                self.syntax_set.find_syntax_plain_text()
            }
        }
    }

    /// Get performance statistics
    pub fn get_stats(&self) -> SyntaxHighlightStats {
        SyntaxHighlightStats {
            theme_name: self.theme_name.clone(),
            syntax_count: self.syntax_set.syntaxes().len(),
        }
    }
}

/// Statistics for syntax highlighting performance
#[derive(Debug, Clone)]
pub struct SyntaxHighlightStats {
    pub theme_name: String,
    pub syntax_count: usize,
}

/// Global syntax highlighter instance with lazy initialization
static SYNTAX_HIGHLIGHTER: Lazy<Result<EnhancedSyntaxHighlighter>> = Lazy::new(|| {
    EnhancedSyntaxHighlighter::new(None)
});

/// Create or update syntax highlighter with configuration
pub fn create_syntax_highlighter_with_config(config: Option<&VTAgentConfig>) -> Result<EnhancedSyntaxHighlighter> {
    if let Some(vt_config) = config {
        if vt_config.syntax_highlighting.enabled {
            EnhancedSyntaxHighlighter::new(Some(&vt_config.syntax_highlighting.theme))
        } else {
            // Return a disabled highlighter that just returns plain text
            Err(anyhow::anyhow!("Syntax highlighting disabled"))
        }
    } else {
        // Fallback to default
        EnhancedSyntaxHighlighter::new(None)
    }
}

/// Detect language from content and return syntax-highlighted version
pub fn syntax_highlight_code(content: &str) -> Result<String> {
    match &*SYNTAX_HIGHLIGHTER {
        Ok(highlighter) => highlighter.highlight_code(content),
        Err(e) => {
            eprintln!("Warning: Syntax highlighting failed to initialize: {}", e);
            // Fallback to plain text
            Ok(content.to_string())
        }
    }
}

/// Enhanced version that uses configuration
pub fn syntax_highlight_code_with_config(content: &str, config: Option<&VTAgentConfig>) -> Result<String> {
    if let Some(vt_config) = config {
        if !vt_config.syntax_highlighting.enabled {
            return Ok(content.to_string());
        }

        // Check file size limit
        if content.len() > (vt_config.syntax_highlighting.max_file_size_mb * 1024 * 1024) {
            return Ok(content.to_string());
        }
    }

    // Try to use configured highlighter, fallback to global
    match create_syntax_highlighter_with_config(config) {
        Ok(highlighter) => highlighter.highlight_code(content),
        Err(_) => syntax_highlight_code(content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlight_rust_code() {
        let rust_code = r#"
fn main() {
    println!("Hello, world!");
    let x = 42;
}
"#;

        let result = syntax_highlight_code(rust_code);
        assert!(result.is_ok());
        let highlighted = result.unwrap();
        // Should contain ANSI escape codes for highlighting
        assert!(highlighted.contains("\x1b["));
        // Should end with reset code
        assert!(highlighted.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_syntax_highlight_python_code() {
        let python_code = r#"
def hello():
    print("Hello, world!")
    x = 42
    return x
"#;

        let result = syntax_highlight_code(python_code);
        assert!(result.is_ok());
        let highlighted = result.unwrap();
        assert!(highlighted.contains("\x1b["));
        assert!(highlighted.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_syntax_highlight_empty_content() {
        let result = syntax_highlight_code("");
        assert!(result.is_ok());
        let highlighted = result.unwrap();
        // Empty content should remain empty (no ANSI codes added)
        assert_eq!(highlighted, "");
    }

    #[test]
    fn test_enhanced_syntax_highlighter_creation() {
        let highlighter = EnhancedSyntaxHighlighter::new(None);
        assert!(highlighter.is_ok());
    }

    #[test]
    fn test_enhanced_syntax_highlighter_with_theme() {
        let highlighter = EnhancedSyntaxHighlighter::new(Some("base16-ocean.dark"));
        assert!(highlighter.is_ok());
    }
}