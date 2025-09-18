//! Markdown rendering utilities for terminal output

use termimad::*;

/// Render markdown text to terminal with advanced formatting using termimad
pub fn render_markdown(text: &str) {
    let skin = MadSkin::default();
    skin.print_text(text);
}
