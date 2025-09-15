//! User interface utilities and shared UI components
//!
//! This module contains shared UI functionality including loading indicators,
//! markdown rendering, and terminal utilities.

pub mod diff_renderer;
pub mod markdown;
pub mod spinner;
pub mod styled;
pub mod terminal;
pub mod user_confirmation;

// Conditional modules for additional UI features
// Optional module for ANSI style parsing (feature gate not currently used)
pub mod anstyle_parse_utils;

pub use markdown::*;
pub use spinner::*;
pub use styled::*;
pub use terminal::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown() {
        let markdown_text = r#"
# Welcome to VTAgent

This is a **bold** statement and this is *italic*.

## Features

- Advanced code analysis
- Multi-language support
- Real-time collaboration
"#;

        // This should not panic
        render_markdown(markdown_text);
    }
}
