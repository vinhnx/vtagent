//! Tree-sitter integration demonstration
//!
//! This example demonstrates how to use the tree-sitter integration
//! for advanced code analysis and understanding.

use std::path::Path;
use vtagent::tree_sitter::{CodeAnalyzer, LanguageSupport, TreeSitterAnalyzer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌲 Tree-sitter Integration Demo");
    println!("================================\n");

    // Initialize the analyzer
    let mut analyzer = TreeSitterAnalyzer::new()?;
    let code_analyzer = CodeAnalyzer::new(&LanguageSupport::Rust);

    println!("✅ Tree-sitter analyzer initialized");
    println!(
        "   Supported languages: {:?}",
        analyzer.supported_languages()
    );

    // Example Rust code to analyze
    let example_code = r#"
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Person {
    pub name: String,
    pub age: u32,
    pub email: Option<String>,
}

impl Person {
    pub fn new(name: &str, age: u32) -> Self {
        Self {
            name: name.to_string(),
            age,
            email: None,
        }
    }

    pub fn greet(&self) -> String {
        format!("Hello, my name is {} and I am {} years old.",
                self.name, self.age)
    }

    pub fn set_email(&mut self, email: String) {
        self.email = Some(email);
    }
}

fn main() {
    let mut person = Person::new("Alice", 30);
    println!("{}", person.greet());

    person.set_email("alice@example.com".to_string());

    let mut people = HashMap::new();
    people.insert("alice", person);

    for (name, person) in &people {
        println!("Person {}: {}", name, person.greet());
    }
}
"#;

    println!("\n📄 Analyzing example Rust code...");
    println!("==================================");

    // Parse the code
    let tree = analyzer.parse(example_code, LanguageSupport::Rust)?;
    println!("✅ Code parsed successfully");

    // Create a syntax tree
    let syntax_tree = vtagent::tree_sitter::analyzer::SyntaxTree {
        root: analyzer.convert_tree_to_syntax_node(tree.root_node(), example_code),
        source_code: example_code.to_string(),
        language: LanguageSupport::Rust,
        diagnostics: analyzer.collect_diagnostics(&tree, example_code),
    };

    // Perform comprehensive analysis
    let analysis = code_analyzer.analyze(&syntax_tree, "example.rs");

    println!("\n📊 Code Analysis Results");
    println!("========================");
    println!("Language: {}", analysis.language);
    println!("Lines of code: {}", analysis.metrics.lines_of_code);
    println!("Functions: {}", analysis.metrics.functions_count);
    println!("Classes/Structs: {}", analysis.metrics.classes_count);
    println!(
        "Comment ratio: {:.1}%",
        analysis.metrics.comment_ratio * 100.0
    );

    println!("\n🏗️  Code Structure");
    println!("=================");
    println!("Modules: {:?}", analysis.structure.modules);
    println!("Functions: {:?}", analysis.structure.functions);
    println!("Classes: {:?}", analysis.structure.classes);

    println!("\n🔍 Extracted Symbols");
    println!("===================");
    for symbol in &analysis.symbols {
        println!(
            "• {} ({:?}) at line {}",
            symbol.name,
            symbol.kind,
            symbol.position.row + 1
        );
        if let Some(sig) = &symbol.signature {
            println!("  Signature: {}", sig);
        }
    }

    println!("\n⚠️  Analysis Issues");
    println!("=================");
    if analysis.issues.is_empty() {
        println!("No issues found! ✅");
    } else {
        for issue in &analysis.issues {
            println!(
                "• {}: {} (line {})",
                match issue.level {
                    vtagent::tree_sitter::analysis::IssueLevel::Error => "❌ ERROR",
                    vtagent::tree_sitter::analysis::IssueLevel::Warning => "⚠️  WARNING",
                    vtagent::tree_sitter::analysis::IssueLevel::Info => "ℹ️  INFO",
                },
                issue.message,
                issue.position.row + 1
            );
        }
    }

    println!("\n📈 Complexity Metrics");
    println!("=====================");
    println!(
        "Cyclomatic complexity: {}",
        analysis.complexity.cyclomatic_complexity
    );
    println!(
        "Cognitive complexity: {}",
        analysis.complexity.cognitive_complexity
    );
    println!(
        "Average function length: {:.1} lines",
        analysis.complexity.function_length_average
    );
    println!(
        "Average parameters: {:.1}",
        analysis.complexity.parameters_average
    );

    // Demonstrate navigation capabilities
    println!("\n🧭 Code Navigation Demo");
    println!("=======================");

    let mut navigator = vtagent::tree_sitter::navigation::CodeNavigator::new();
    navigator.build_index(&analysis.symbols);

    // Find a specific function
    if let Some(location) = navigator.goto_definition("greet") {
        println!(
            "Found 'greet' function at line {}",
            location.target.get_position().row + 1
        );
    }

    // Get symbols in scope
    let global_symbols = navigator.get_symbols_in_scope(None);
    println!("Global symbols found: {}", global_symbols.len());

    println!("\n🎯 Tree-sitter Integration Complete!");
    println!("====================================");
    println!("The tree-sitter integration provides:");
    println!("• Advanced syntax-aware code parsing");
    println!("• Comprehensive code analysis and metrics");
    println!("• Symbol extraction and navigation");
    println!("• Complexity analysis");
    println!("• Multi-language support (Rust, Python, JavaScript, etc.)");
    println!("\nThis enhances vtagent's ability to understand and work with code!");
    println!("🚀 Ready for SWE-bench level performance optimization!");

    Ok(())
}
