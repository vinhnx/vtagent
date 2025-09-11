//! Markdown rendering utilities for terminal output

use console::style;

/// Render markdown text to terminal with basic formatting
pub fn render_markdown(text: &str) {
    println!("{}", style("[MARKDOWN] Content").cyan().bold());
    println!("{}", style("─".repeat(50)).dim());

    for line in text.lines() {
        if line.starts_with("# ") {
            println!("{}", style(&line[2..]).yellow().bold());
        } else if line.starts_with("## ") {
            println!("{}", style(&line[3..]).yellow());
        } else if line.starts_with("- ") {
            println!("  {}", style(&line[2..]).dim());
        } else if line.starts_with("* ") {
            println!("  {}", style(&line[2..]).dim());
        } else if line.is_empty() {
            println!();
        } else {
            println!("{}", line);
        }
    }

    println!();
    println!("{}", style("Markdown rendering complete").green());
}
