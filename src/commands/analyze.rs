//! Analyze command implementation - workspace analysis

use crate::tools::ToolRegistry;
use crate::tree_sitter::{CodeAnalyzer, TreeSitterAnalyzer};
use crate::types::{AgentConfig, AnalysisDepth, OutputFormat};
use anyhow::Result;
use console::style;
use serde_json::json;

/// Handle the analyze command - comprehensive workspace analysis
pub async fn handle_analyze_command(
    config: AgentConfig,
    depth: String,
    format: String,
) -> Result<()> {
    println!("{}", style("🔍 Analyzing workspace...").cyan().bold());

    let depth = match depth.to_lowercase().as_str() {
        "basic" => AnalysisDepth::Basic,
        "standard" => AnalysisDepth::Standard,
        "deep" => AnalysisDepth::Deep,
        _ => {
            println!("{}", style("⚠️  Invalid depth. Using 'standard'.").yellow());
            AnalysisDepth::Standard
        }
    };

    let _output_format = match format.to_lowercase().as_str() {
        "text" => OutputFormat::Text,
        "json" => OutputFormat::Json,
        "html" => OutputFormat::Html,
        _ => {
            println!("{}", style("⚠️  Invalid format. Using 'text'.").yellow());
            OutputFormat::Text
        }
    };

    let mut registry = ToolRegistry::new(config.workspace.clone());

    // Step 1: Get high-level directory structure
    println!("{}", style("1. Getting workspace structure...").dim());
    let root_files = registry
        .execute("list_files", json!({"path": ".", "max_items": 50}))
        .await;

    match root_files {
        Ok(result) => {
            println!("{}", style("✓ Root directory structure obtained").green());
            if let Some(files_array) = result.get("files") {
                println!(
                    "   Found {} files/directories in root",
                    files_array.as_array().unwrap_or(&vec![]).len()
                );
            }
        }
        Err(e) => println!("{} {}", style("✗ Failed to list root directory:").red(), e),
    }

    // Step 2: Look for important project files
    println!("{}", style("2. Identifying project type...").dim());
    let important_files = vec![
        "README.md",
        "Cargo.toml",
        "package.json",
        "go.mod",
        "requirements.txt",
        "Makefile",
    ];

    for file in important_files {
        let check_file = registry
            .execute("list_files", json!({"path": ".", "include_hidden": false}))
            .await;
        if let Ok(result) = check_file {
            if let Some(files) = result.get("files") {
                if let Some(files_array) = files.as_array() {
                    for file_obj in files_array {
                        if let Some(path) = file_obj.get("path") {
                            if path.as_str().unwrap_or("") == file {
                                println!("   {} Detected: {}", style("✓").green(), file);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 3: Read key configuration files
    println!("{}", style("3. Reading project configuration...").dim());
    let config_files = vec!["README.md", "Cargo.toml", "package.json"];

    for config_file in config_files {
        let read_result = registry
            .execute("read_file", json!({"path": config_file, "max_bytes": 2000}))
            .await;
        match read_result {
            Ok(result) => {
                println!(
                    "   {} Read {} ({} bytes)",
                    style("✓").green(),
                    config_file,
                    result
                        .get("metadata")
                        .and_then(|m| m.get("size"))
                        .unwrap_or(&serde_json::json!(null))
                );
            }
            Err(_) => {} // File doesn't exist, that's ok
        }
    }

    // Step 4: Analyze source code structure
    println!("{}", style("4. Analyzing source code structure...").dim());

    // Check for common source directories
    let src_dirs = vec!["src", "lib", "pkg", "internal", "cmd"];
    for dir in src_dirs {
        let check_dir = registry
            .execute("list_files", json!({"path": ".", "include_hidden": false}))
            .await;
        if let Ok(result) = check_dir {
            if let Some(files) = result.get("files") {
                if let Some(files_array) = files.as_array() {
                    for file_obj in files_array {
                        if let Some(path) = file_obj.get("path") {
                            if path.as_str().unwrap_or("") == dir {
                                println!(
                                    "   {} Found source directory: {}",
                                    style("✓").green(),
                                    dir
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 6: Advanced code analysis with tree-sitter (for deep analysis)
    if matches!(depth, AnalysisDepth::Deep) {
        println!(
            "{}",
            style("6. Advanced code analysis with tree-sitter...").yellow()
        );
        match perform_tree_sitter_analysis(&config).await {
            Ok(_) => println!("   {} Tree-sitter analysis complete", style("✓").green()),
            Err(e) => println!("   {} Tree-sitter analysis failed: {}", style("✗").red(), e),
        }
    }

    println!(
        "{}",
        style("✅ Workspace analysis complete!").green().bold()
    );
    println!(
        "{}",
        style("💡 You can now ask me specific questions about the codebase.").dim()
    );

    if matches!(depth, AnalysisDepth::Deep) {
        println!(
            "{}",
            style("🔍 Advanced analysis available with tree-sitter integration.").dim()
        );
    }

    Ok(())
}

/// Perform advanced code analysis using tree-sitter
async fn perform_tree_sitter_analysis(config: &AgentConfig) -> Result<()> {
    use crate::tree_sitter::analyzer::LanguageSupport;

    let mut analyzer = TreeSitterAnalyzer::new()?;
    let code_analyzer = CodeAnalyzer::new(&LanguageSupport::Rust); // Default to Rust

    // Find code files to analyze
    let mut registry = ToolRegistry::new(config.workspace.clone());
    let list_result = registry
        .execute("list_files", json!({"path": ".", "recursive": true}))
        .await?;

    if let Some(files) = list_result.get("files") {
        if let Some(files_array) = files.as_array() {
            let mut analyzed_files = 0;
            let mut total_lines = 0;
            let mut total_functions = 0;

            for file_obj in files_array {
                if let Some(path) = file_obj.get("path").and_then(|p| p.as_str()) {
                    if path.ends_with(".rs") {
                        // Analyze Rust files
                        match analyzer.parse_file(std::path::Path::new(path)) {
                            Ok(syntax_tree) => {
                                let analysis = code_analyzer.analyze(&syntax_tree, path);
                                analyzed_files += 1;
                                total_lines += analysis.metrics.lines_of_code;
                                total_functions += analysis.metrics.functions_count;

                                if config.verbose {
                                    println!(
                                        "     {} Analyzed {}: {} lines, {} functions",
                                        style("📄").dim(),
                                        path,
                                        analysis.metrics.lines_of_code,
                                        analysis.metrics.functions_count
                                    );

                                    if !analysis.issues.is_empty() {
                                        println!(
                                            "       {} {} issues found",
                                            style("⚠️").yellow(),
                                            analysis.issues.len()
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                if config.verbose {
                                    println!(
                                        "     {} Failed to analyze {}: {}",
                                        style("❌").red(),
                                        path,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }

            if analyzed_files > 0 {
                println!(
                    "     {} Analyzed {} files: {} total lines, {} functions",
                    style("📊").cyan(),
                    analyzed_files,
                    total_lines,
                    total_functions
                );

                // Calculate quality metrics for the project
                let avg_lines_per_function = if total_functions > 0 {
                    total_lines as f64 / total_functions as f64
                } else {
                    0.0
                };

                println!(
                    "     {} Average lines per function: {:.1}",
                    style("📏").dim(),
                    avg_lines_per_function
                );

                if avg_lines_per_function > 50.0 {
                    println!(
                        "       {} Consider breaking down large functions",
                        style("💡").yellow()
                    );
                }
            }
        }
    }

    Ok(())
}
