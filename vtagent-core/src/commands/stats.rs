//! Stats command implementation - show session statistics and performance metrics

use crate::config::types::{AgentConfig, OutputFormat, PerformanceMetrics};
use crate::tools::build_function_declarations;
use anyhow::Result;
use console::style;

/// Handle the stats command - display session statistics and performance metrics
pub async fn handle_stats_command(
    config: AgentConfig,
    detailed: bool,
    format: String,
) -> Result<()> {
    let output_format = match format.to_lowercase().as_str() {
        "text" => OutputFormat::Text,
        "json" => OutputFormat::Json,
        "html" => OutputFormat::Html,
        _ => OutputFormat::Text,
    };

    println!("{}", style("Session Statistics").cyan().bold());

    // Mock performance metrics (in a real implementation, these would come from the agent)
    let metrics = PerformanceMetrics {
        session_duration_seconds: 0, // Would be tracked
        total_api_calls: 0,
        total_tokens_used: None,
        average_response_time_ms: 0.0,
        tool_execution_count: 0,
        error_count: 0,
        recovery_success_rate: 0.0,
    };

    match output_format {
        OutputFormat::Text => display_text_stats(&config, &metrics, detailed),
        OutputFormat::Json => display_json_stats(&config, &metrics),
        OutputFormat::Html => display_html_stats(&config, &metrics),
    }

    Ok(())
}

fn display_text_stats(config: &AgentConfig, metrics: &PerformanceMetrics, detailed: bool) {
    println!("{} Configuration:", style("⚙️").dim());
    println!("  Model: {}", style(&config.model).cyan());
    println!("  Workspace: {}", style(config.workspace.display()).cyan());
    println!(
        "  Verbose Mode: {}",
        if config.verbose {
            "Enabled"
        } else {
            "Disabled"
        }
    );

    println!("\n{} Tool Information:", style("").dim());
    let tool_count = build_function_declarations().len();
    println!("  Available Tools: {}", style(tool_count).cyan());

    if detailed {
        println!("  Tools:");
        for tool in build_function_declarations() {
            println!("    • {}", style(&tool.name).yellow());
        }
    }

    println!("\n{} Performance Metrics:", style("📈").dim());
    println!(
        "  Session Duration: {} seconds",
        style(metrics.session_duration_seconds).cyan()
    );
    println!("  API Calls: {}", style(metrics.total_api_calls).cyan());
    println!(
        "  Tool Executions: {}",
        style(metrics.tool_execution_count).cyan()
    );
    println!("  Errors: {}", style(metrics.error_count).red());
    println!(
        "  Recovery Rate: {:.1}%",
        style(metrics.recovery_success_rate * 100.0).green()
    );

    if let Some(tokens) = metrics.total_tokens_used {
        println!("  Total Tokens: {}", style(tokens).cyan());
    }

    println!(
        "  Avg Response Time: {:.0}ms",
        style(metrics.average_response_time_ms).cyan()
    );

    if detailed {
        println!("\n{} System Information:", style("💻").dim());
        println!(
            "  Rust Version: {}",
            style(env!("CARGO_PKG_RUST_VERSION")).cyan()
        );
        println!(
            "  vtagent Version: {}",
            style(env!("CARGO_PKG_VERSION")).cyan()
        );
        println!(
            "  Build Profile: {}",
            if cfg!(debug_assertions) {
                "Debug"
            } else {
                "Release"
            }
        );
    }
}

fn display_json_stats(config: &AgentConfig, metrics: &PerformanceMetrics) {
    let stats = serde_json::json!({
        "configuration": {
            "model": config.model,
            "workspace": config.workspace,
            "verbose": config.verbose
        },
        "tools": {
            "count": build_function_declarations().len(),
            "available": build_function_declarations().iter().map(|t| &t.name).collect::<Vec<_>>()
        },
        "performance": {
            "session_duration_seconds": metrics.session_duration_seconds,
            "total_api_calls": metrics.total_api_calls,
            "total_tokens_used": metrics.total_tokens_used,
            "average_response_time_ms": metrics.average_response_time_ms,
            "tool_execution_count": metrics.tool_execution_count,
            "error_count": metrics.error_count,
            "recovery_success_rate": metrics.recovery_success_rate
        },
        "system": {
            "rust_version": env!("CARGO_PKG_RUST_VERSION"),
            "vtagent_version": env!("CARGO_PKG_VERSION"),
            "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" }
        }
    });

    println!("{}", serde_json::to_string_pretty(&stats).unwrap());
}

fn display_html_stats(config: &AgentConfig, metrics: &PerformanceMetrics) {
    println!("<!DOCTYPE html>");
    println!("<html><head><title>vtagent Statistics</title></head><body>");
    println!("<h1>vtagent Session Statistics</h1>");

    println!("<h2>Configuration</h2>");
    println!("<ul>");
    println!("<li><strong>Model:</strong> {}</li>", config.model);
    println!(
        "<li><strong>Workspace:</strong> {}</li>",
        config.workspace.display()
    );
    println!(
        "<li><strong>Verbose Mode:</strong> {}</li>",
        if config.verbose {
            "Enabled"
        } else {
            "Disabled"
        }
    );
    println!("</ul>");

    println!("<h2>Tool Information</h2>");
    println!(
        "<p><strong>Available Tools:</strong> {}</p>",
        build_function_declarations().len()
    );
    println!("<ul>");
    for tool in build_function_declarations() {
        println!("<li>{}</li>", tool.name);
    }
    println!("</ul>");

    println!("<h2>Performance Metrics</h2>");
    println!("<ul>");
    println!(
        "<li><strong>Session Duration:</strong> {} seconds</li>",
        metrics.session_duration_seconds
    );
    println!(
        "<li><strong>API Calls:</strong> {}</li>",
        metrics.total_api_calls
    );
    println!(
        "<li><strong>Tool Executions:</strong> {}</li>",
        metrics.tool_execution_count
    );
    println!("<li><strong>Errors:</strong> {}</li>", metrics.error_count);
    println!(
        "<li><strong>Recovery Rate:</strong> {:.1}%</li>",
        metrics.recovery_success_rate * 100.0
    );
    if let Some(tokens) = metrics.total_tokens_used {
        println!("<li><strong>Total Tokens:</strong> {}</li>", tokens);
    }
    println!(
        "<li><strong>Avg Response Time:</strong> {:.0}ms</li>",
        metrics.average_response_time_ms
    );
    println!("</ul>");

    println!("</body></html>");
}
