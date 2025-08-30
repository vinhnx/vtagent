use std::io::{self, Write};

/// A simple markdown renderer for streaming output
pub struct MarkdownRenderer {
    in_code_block: bool,
    code_language: Option<String>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            in_code_block: false,
            code_language: None,
        }
    }

    /// Process a chunk of text and render it with basic markdown formatting
    pub fn render_chunk(&mut self, chunk: &str) -> io::Result<()> {
        // Process the chunk line by line for proper markdown rendering
        // This handles streaming responses correctly
        let lines: Vec<&str> = chunk.split('\n').collect();

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                println!(); // Add newline between lines
            }

            if !line.is_empty() {
                // Use render_line for proper markdown processing
                if let Err(_) = self.render_line(line) {
                    // Fallback to plain text if rendering fails
                    print!("{}", line);
                }
            }
        }

        io::stdout().flush()?;
        Ok(())
    }

    /// Render a complete line with basic markdown formatting
    pub fn render_line(&mut self, line: &str) -> io::Result<()> {
        // Handle code block start/end
        if line.starts_with("```") {
            if self.in_code_block {
                // End of code block
                self.in_code_block = false;
                self.code_language = None;
                println!("\x1b[0m"); // Reset styling
                return Ok(());
            } else {
                // Start of code block
                self.in_code_block = true;
                let language = line.strip_prefix("```").unwrap_or("").trim();
                self.code_language = if language.is_empty() {
                    None
                } else {
                    Some(language.to_string())
                };
                println!("\x1b[47m\x1b[30m"); // White background, black text for code blocks
                if let Some(lang) = &self.code_language {
                    println!("{}:", lang);
                }
                return Ok(());
            }
        }

        // Handle headers
        if let Some(stripped) = line.strip_prefix("# ") {
            println!("\n\x1b[1m{}\x1b[0m", stripped); // Bold
            return Ok(());
        } else if let Some(stripped) = line.strip_prefix("## ") {
            println!("\n\x1b[1m{}\x1b[0m", stripped); // Bold
            return Ok(());
        }

        // Handle lists
        if line.starts_with("- ") || line.starts_with("* ") {
            println!("  \x1b[36m•\x1b[0m {}", &line[2..]); // Cyan bullet
            return Ok(());
        }

        // If we're in a code block, render as code
        if self.in_code_block {
            println!("{}", line);
            return Ok(());
        }

        // Handle bold text (**text**)
        let processed_line = self.process_bold_text(line);

        // Handle italic text (*text* or _text_)
        let processed_line = self.process_italic_text(&processed_line);

        // Handle inline code (`code`)
        let processed_line = self.process_inline_code(&processed_line);

        println!("{}", processed_line);
        Ok(())
    }

    /// Process bold text (**text**)
    fn process_bold_text(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();
        let mut in_bold = false;

        while let Some(ch) = chars.next() {
            if ch == '*' && chars.peek() == Some(&'*') {
                // Found **
                chars.next(); // consume second *
                if in_bold {
                    result.push_str("\x1b[0m"); // Reset styling
                    in_bold = false;
                } else {
                    result.push_str("\x1b[1m"); // Start bold
                    in_bold = true;
                }
            } else {
                result.push(ch);
            }
        }

        // Reset styling at the end if needed
        if in_bold {
            result.push_str("\x1b[0m");
        }

        result
    }

    /// Process italic text (*text* or _text_)
    fn process_italic_text(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();
        let mut in_italic = false;
        let mut delimiter = '\0';

        while let Some(ch) = chars.next() {
            if (ch == '*' || ch == '_') && !in_italic {
                // Start italic
                delimiter = ch;
                result.push_str("\x1b[3m"); // Start italic
                in_italic = true;
            } else if ch == delimiter && in_italic {
                // End italic
                result.push_str("\x1b[0m"); // Reset styling
                in_italic = false;
            } else {
                result.push(ch);
            }
        }

        // Reset styling at the end if needed
        if in_italic {
            result.push_str("\x1b[0m");
        }

        result
    }

    /// Process inline code (`code`)
    fn process_inline_code(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();
        let mut in_code = false;

        while let Some(ch) = chars.next() {
            if ch == '`' {
                if in_code {
                    result.push_str("\x1b[0m"); // Reset styling
                    in_code = false;
                } else {
                    result.push_str("\x1b[47m\x1b[30m"); // White background, black text
                    in_code = true;
                }
            } else {
                result.push(ch);
            }
        }

        // Reset styling at the end if needed
        if in_code {
            result.push_str("\x1b[0m");
        }

        result
    }


}
