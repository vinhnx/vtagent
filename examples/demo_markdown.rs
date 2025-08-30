use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use vtagent::markdown_renderer::MarkdownRenderer;

/// Demo the markdown renderer with sample content
fn demo_markdown_rendering() -> io::Result<()> {
    let mut renderer = MarkdownRenderer::new();

    // Sample markdown content that might come from a Gemini API response
    let sample_content = vec![
        "# Project Analysis\n",
        "\n",
        "## Overview\n",
        "\n",
        "This is a **comprehensive** analysis of your project. The codebase includes several *important* components:\n",
        "\n",
        "- **src/main.rs**: Main entry point\n",
        "- **src/lib.rs**: Library code\n",
        "- **Cargo.toml**: Project configuration\n",
        "\n",
        "Here's some `inline code` that demonstrates a key concept.\n",
        "\n",
        "### Recommendations\n",
        "\n",
        "1. Consider adding more unit tests\n",
        "2. Review the documentation\n",
        "3. Optimize performance bottlenecks\n",
        "\n",
        "> This is a blockquote with some *emphasized* text.\n",
    ];

    println!("Demo: Markdown Streaming Renderer");
    println!("=================================");
    println!();

    // Simulate streaming by processing line by line with delays
    for line in sample_content {
        renderer.render_chunk(line)?;
        io::stdout().flush()?;
        // Small delay to simulate real streaming
        thread::sleep(Duration::from_millis(50));
    }

    // Flush any remaining content
    renderer.flush()?;

    Ok(())
}

fn main() -> io::Result<()> {
    demo_markdown_rendering()
}
