use super::theme;
use anstream::println as styled_println;
use anstyle::{Effects, Reset, Style};

/// Style presets for consistent UI theming
pub struct Styles;

impl Styles {
    /// Error message style (red)
    pub fn error() -> Style {
        theme::active_styles().error
    }

    /// Warning message style (yellow)
    pub fn warning() -> Style {
        theme::active_styles().secondary
    }

    /// Success message style (green)
    pub fn success() -> Style {
        theme::active_styles().primary
    }

    /// Info message style (blue)
    pub fn info() -> Style {
        theme::active_styles().output
    }

    /// Debug message style (cyan)
    pub fn debug() -> Style {
        theme::active_styles().response
    }

    /// Bold text style
    pub fn bold() -> Style {
        Style::new().effects(Effects::BOLD)
    }

    /// Bold error style
    pub fn bold_error() -> Style {
        theme::active_styles().error.bold()
    }

    /// Bold success style
    pub fn bold_success() -> Style {
        theme::active_styles().primary.bold()
    }

    /// Bold warning style
    pub fn bold_warning() -> Style {
        theme::active_styles().secondary.bold()
    }

    /// Header style matching the welcome accent
    pub fn header() -> Style {
        theme::welcome_header_style()
    }

    /// Code style (magenta)
    pub fn code() -> Style {
        theme::active_styles().secondary
    }

    /// Render style to ANSI string
    pub fn render(style: &Style) -> String {
        format!("{}", style)
    }

    /// Render reset ANSI string
    pub fn render_reset() -> String {
        format!("{}", Reset)
    }
}

/// Print a styled error message
pub fn error(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::render(&Styles::error()),
        message,
        Styles::render_reset()
    );
}

/// Print a styled warning message
pub fn warning(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::render(&Styles::warning()),
        message,
        Styles::render_reset()
    );
}

/// Print a styled success message
pub fn success(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::render(&Styles::success()),
        message,
        Styles::render_reset()
    );
}

/// Print a styled info message
pub fn info(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::render(&Styles::info()),
        message,
        Styles::render_reset()
    );
}

/// Print a styled debug message
pub fn debug(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::render(&Styles::debug()),
        message,
        Styles::render_reset()
    );
}

/// Print a styled bold message
pub fn bold(message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::render(&Styles::bold()),
        message,
        Styles::render_reset()
    );
}

/// Print a styled message with custom style
pub fn styled(style: &Style, message: &str) {
    styled_println!(
        "{}{}{}",
        Styles::render(style),
        message,
        Styles::render_reset()
    );
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
        styled(&Styles::header(), "Test custom");
    }
}
