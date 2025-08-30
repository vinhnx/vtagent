use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use vtagent::markdown_renderer::MarkdownRenderer;

/// Simulate streaming markdown text character by character
fn simulate_streaming(markdown_text: &str) -> io::Result<()> {
    let mut renderer = MarkdownRenderer::new();

    println!("Simulating streaming markdown response:");
    println!("=====================================");

    // Simulate streaming by processing character by character
    for ch in markdown_text.chars() {
        renderer.render_chunk(&ch.to_string())?;
        // Small delay to simulate real streaming
        thread::sleep(Duration::from_millis(10));
    }

    renderer.flush()?;
    Ok(())
}

fn main() -> io::Result<()> {
    // Example markdown text that might come from a Gemini API response
    let example_response = "# Project Analysis

## Overview
This is a **comprehensive** analysis of your project. The codebase includes several *important* components:

- **src/main.rs**: Main entry point
- **src/lib.rs**: Library code
- **Cargo.toml**: Project configuration

Here's some `inline code` that demonstrates a key concept.

### Recommendations
1. Consider adding more unit tests
2. Review the documentation
3. Optimize performance bottlenecks

> This is a blockquote with some *emphasized* text.";

    simulate_streaming(example_response)
}
