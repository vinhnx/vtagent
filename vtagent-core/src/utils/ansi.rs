use anstream::{AutoStream, ColorChoice};
use anstyle::{AnsiColor, Color, Reset, Style};
use anstyle_query::{clicolor, clicolor_force, no_color, term_supports_color};
use anyhow::Result;
use std::io::{self, Write};

/// Styles available for rendering messages
#[derive(Clone, Copy)]
pub enum MessageStyle {
    Info,
    Error,
    Output,
}

impl MessageStyle {
    fn style(self) -> Style {
        match self {
            Self::Info => Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Blue)))
                .bold(),
            Self::Error => Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Red)))
                .bold(),
            Self::Output => Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))),
        }
    }
}

/// Renderer with deferred output buffering
pub struct AnsiRenderer {
    writer: AutoStream<io::Stdout>,
    buffer: String,
    color: bool,
}

impl AnsiRenderer {
    /// Create a new renderer for stdout
    pub fn stdout() -> Self {
        let color =
            clicolor_force() || (!no_color() && clicolor().unwrap_or_else(term_supports_color));
        let choice = if color {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };
        Self {
            writer: AutoStream::new(std::io::stdout(), choice),
            buffer: String::new(),
            color,
        }
    }

    /// Push text into the buffer
    pub fn push(&mut self, text: &str) {
        self.buffer.push_str(text);
    }

    /// Flush the buffer with the given style
    pub fn flush(&mut self, style: MessageStyle) -> Result<()> {
        let style = style.style();
        if self.color {
            writeln!(self.writer, "{style}{}{Reset}", self.buffer)?;
        } else {
            writeln!(self.writer, "{}", self.buffer)?;
        }
        self.writer.flush()?;
        self.buffer.clear();
        Ok(())
    }

    /// Convenience for writing a single line
    pub fn line(&mut self, style: MessageStyle, text: &str) -> Result<()> {
        self.buffer.clear();
        self.push(text);
        self.flush(style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styles_construct() {
        let info = MessageStyle::Info.style();
        assert_eq!(info, MessageStyle::Info.style());
    }

    #[test]
    fn test_renderer_buffer() {
        let mut r = AnsiRenderer::stdout();
        r.push("hello");
        assert_eq!(r.buffer, "hello");
    }
}
