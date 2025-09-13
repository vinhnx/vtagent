//! Color utilities for the VT Agent
//!
//! This module provides color manipulation capabilities using the colored crate,
//! which offers a simpler and more robust API for terminal color styling.

use colored::*;

/// Apply red color to text
pub fn red(text: &str) -> ColoredString {
    text.red()
}

/// Apply green color to text
pub fn green(text: &str) -> ColoredString {
    text.green()
}

/// Apply blue color to text
pub fn blue(text: &str) -> ColoredString {
    text.blue()
}

/// Apply yellow color to text
pub fn yellow(text: &str) -> ColoredString {
    text.yellow()
}

/// Apply purple color to text
pub fn purple(text: &str) -> ColoredString {
    text.purple()
}

/// Apply cyan color to text
pub fn cyan(text: &str) -> ColoredString {
    text.cyan()
}

/// Apply white color to text
pub fn white(text: &str) -> ColoredString {
    text.white()
}

/// Apply black color to text
pub fn black(text: &str) -> ColoredString {
    text.black()
}

/// Apply bold styling to text
pub fn bold(text: &str) -> ColoredString {
    text.bold()
}

/// Apply italic styling to text
pub fn italic(text: &str) -> ColoredString {
    text.italic()
}

/// Apply underline styling to text
pub fn underline(text: &str) -> ColoredString {
    text.underline()
}

/// Apply dimmed styling to text
pub fn dimmed(text: &str) -> ColoredString {
    text.dimmed()
}

/// Apply blinking styling to text
pub fn blink(text: &str) -> ColoredString {
    text.blink()
}

/// Apply reversed styling to text
pub fn reversed(text: &str) -> ColoredString {
    text.reversed()
}

/// Apply strikethrough styling to text
pub fn strikethrough(text: &str) -> ColoredString {
    text.strikethrough()
}

/// Apply custom RGB color to text
pub fn rgb(text: &str, r: u8, g: u8, b: u8) -> ColoredString {
    text.truecolor(r, g, b)
}

/// Combine multiple color and style operations
pub fn custom_style(text: &str, styles: &[&str]) -> ColoredString {
    let mut colored_text = ColoredString::from(text);
    
    for style in styles {
        colored_text = match *style {
            "red" => colored_text.red(),
            "green" => colored_text.green(),
            "blue" => colored_text.blue(),
            "yellow" => colored_text.yellow(),
            "purple" => colored_text.purple(),
            "cyan" => colored_text.cyan(),
            "white" => colored_text.white(),
            "black" => colored_text.black(),
            "bold" => colored_text.bold(),
            "italic" => colored_text.italic(),
            "underline" => colored_text.underline(),
            "dimmed" => colored_text.dimmed(),
            "blink" => colored_text.blink(),
            "reversed" => colored_text.reversed(),
            "strikethrough" => colored_text.strikethrough(),
            _ => colored_text, // Ignore unknown styles
        };
    }
    
    colored_text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_colors() {
        let red_text = red("Hello");
        assert!(red_text.to_string().contains("Hello"));
        
        let green_text = green("World");
        assert!(green_text.to_string().contains("World"));
    }

    #[test]
    fn test_styles() {
        let bold_text = bold("Bold");
        assert!(bold_text.to_string().contains("Bold"));
        
        let italic_text = italic("Italic");
        assert!(italic_text.to_string().contains("Italic"));
    }

    #[test]
    fn test_rgb() {
        let rgb_text = rgb("RGB Color", 255, 128, 64);
        assert!(rgb_text.to_string().contains("RGB Color"));
    }

    #[test]
    fn test_custom_style() {
        let styled_text = custom_style("Styled", &["red", "bold"]);
        assert!(styled_text.to_string().contains("Styled"));
    }
}