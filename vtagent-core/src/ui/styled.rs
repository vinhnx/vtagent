//! Styled text output using anstyle for cross-platform terminal styling
//!
//! This module provides a consistent API for styled terminal output that works
//! across different platforms and respects environment variables like NO_COLOR.

use anstream::println as styled_println;
use anstyle::{AnsiColor, Color, Effects, Style};

/// Style presets for consistent UI theming
pub struct Styles;

impl Styles {
    /// Error message style (red)
    pub fn error() -> Style {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)))
    }

    /// Warning message style (yellow)
    pub fn warning() -> Style {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
    }

    /// Success message style (green)
    pub fn success() -> Style {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)))
    }

    /// Info message style (blue)
    pub fn info() -> Style {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Blue)))
    }

    /// Debug message style (cyan)
    pub fn debug() -> Style {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)))
    }

    /// Bold text style
    pub fn bold() -> Style {
        Style::new().effects(Effects::BOLD)
    }

    /// Bold error style
    pub fn bold_error() -> Style {
        Style::new()
            .fg_color(Some(Color::Ansi(AnsiColor::Red)))
            .effects(Effects::BOLD)
    }

    /// Bold success style
    pub fn bold_success() -> Style {
        Style::new()
            .fg_color(Some(Color::Ansi(AnsiColor::Green)))
            .effects(Effects::BOLD)
    }

    /// Bold warning style
    pub fn bold_warning() -> Style {
        Style::new()
            .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
            .effects(Effects::BOLD)
    }

    /// Header style (bold blue)
    pub fn header() -> Style {
        Style::new()
            .fg_color(Some(Color::Ansi(AnsiColor::Blue)))
            .effects(Effects::BOLD)
    }

    /// Code style (magenta)
    pub fn code() -> Style {
        Style::new().fg_color(Some(Color::Ansi(AnsiColor::Magenta)))
    }
}

/// Print a styled error message
pub fn error(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::error().render(),
        message,
        Styles::error().render_reset()
    );
}

/// Print a styled warning message
pub fn warning(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::warning().render(),
        message,
        Styles::warning().render_reset()
    );
}

/// Print a styled success message
pub fn success(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::success().render(),
        message,
        Styles::success().render_reset()
    );
}

/// Print a styled info message
pub fn info(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::info().render(),
        message,
        Styles::info().render_reset()
    );
}

/// Print a styled debug message
pub fn debug(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::debug().render(),
        message,
        Styles::debug().render_reset()
    );
}

/// Print a bold message
pub fn bold(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::bold().render(),
        message,
        Styles::bold().render_reset()
    );
}

/// Print a message with a custom style
pub fn custom(message: &str, style: Style) {
    styled_println!("{}{}{}", style.render(), message, style.render_reset());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styles() {
        // These should not panic
        error("Test error");
        warning("Test warning");
        success("Test success");
        info("Test info");
        debug("Test debug");
        bold("Test bold");
        custom("Test custom", Styles::header());
    }
}
