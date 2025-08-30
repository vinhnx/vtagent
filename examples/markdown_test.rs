use std::io;
use vtagent::markdown_renderer::MarkdownRenderer;

fn main() -> io::Result<()> {
    let mut renderer = MarkdownRenderer::new();

    // Test various markdown elements
    let markdown_text = "# Hello World\n\nThis is a **bold** statement and this is *italic* text.\n\nHere's some `inline code`.\n\n## Features\n\n- Feature 1\n- Feature 2\n- Feature 3\n\n";

    println!("Rendering markdown:");
    println!("==================");

    // Render character by character to simulate streaming
    for ch in markdown_text.chars() {
        renderer.render_chunk(&ch.to_string())?;
    }

    renderer.flush()?;

    Ok(())
}
