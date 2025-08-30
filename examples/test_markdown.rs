use std::io;
use vtagent::markdown_renderer::MarkdownRenderer;

fn main() -> io::Result<()> {
    let mut renderer = MarkdownRenderer::new();

    // Test various markdown elements
    let test_lines = vec![
        "# Hello World\n",
        "This is a **bold** statement.\n",
        "This is *italic* text.\n",
        "Here's some `inline code`.\n",
        "## Features\n",
        "- Feature 1\n",
        "- Feature 2\n",
        "- Feature 3\n",
    ];

    println!("Testing markdown renderer:");
    println!("========================");

    for line in test_lines {
        renderer.render_chunk(line)?;
    }

    renderer.flush()?;

    Ok(())
}
