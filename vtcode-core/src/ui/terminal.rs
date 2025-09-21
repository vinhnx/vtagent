//! Terminal utilities and helpers

use is_terminal::IsTerminal;
use std::io::Write;
use terminal_size::{terminal_size, Width, Height};

/// Get the terminal width, fallback to 80 if unable to determine
pub fn get_terminal_width() -> usize {
    if let Some((Width(w), _)) = terminal_size() {
        w as usize
    } else {
        80
    }
}

/// Get the terminal height, fallback to 24 if unable to determine
pub fn get_terminal_height() -> usize {
    if let Some((_, Height(h))) = terminal_size() {
        h as usize
    } else {
        24
    }
}

/// Get both terminal width and height as a tuple
pub fn get_terminal_size() -> (usize, usize) {
    if let Some((Width(w), Height(h))) = terminal_size() {
        (w as usize, h as usize)
    } else {
        (80, 24)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_terminal_width() {
        let width = get_terminal_width();
        assert!(width > 0, "Terminal width should be greater than 0");
        assert!(width >= 40, "Terminal width should be at least 40 (minimum)");
    }

    #[test]
    fn test_get_terminal_height() {
        let height = get_terminal_height();
        assert!(height > 0, "Terminal height should be greater than 0");
        assert!(height >= 10, "Terminal height should be at least 10 (minimum)");
    }

    #[test]
    fn test_get_terminal_size() {
        let (width, height) = get_terminal_size();
        assert!(width > 0, "Terminal width should be greater than 0");
        assert!(height > 0, "Terminal height should be greater than 0");
        assert!(width >= 40, "Terminal width should be at least 40");
        assert!(height >= 10, "Terminal height should be at least 10");
    }
}

/// Flush stdout to ensure output is displayed immediately
pub fn flush_stdout() {
    std::io::stdout().flush().ok();
}

/// Read a line from stdin with proper error handling
pub fn read_line() -> std::io::Result<String> {
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer)?;
    Ok(buffer.trim().to_string())
}

/// Check if output is being piped (not a terminal)
pub fn is_piped_output() -> bool {
    !std::io::stdout().is_terminal()
}

/// Check if input is being piped (not a terminal)
pub fn is_piped_input() -> bool {
    !std::io::stdin().is_terminal()
}
