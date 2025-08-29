use console::{style, Emoji};
use std::io::{self, Write};

/// A simple markdown renderer that converts markdown to styled terminal output
pub struct MarkdownRenderer {
    buffer: String,
    in_code_block: bool,
    code_block_language: String,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            in_code_block: false,
            code_block_language: String::new(),
        }
    }

    /// Process a chunk of markdown text and render it to the terminal
    pub fn render_chunk(&mut self, chunk: &str) -> io::Result<()> {
        self.buffer.push_str(chunk);
        
        // Process complete lines when we have them
        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].to_string();
            self.buffer.drain(..=newline_pos);
            
            self.render_line(&line)?;
        }
        
        // If we have remaining text and it's not empty, render it as a partial line
        if !self.buffer.is_empty() {
            self.render_partial_line(&self.buffer)?;
        }
        
        io::stdout().flush()?;
        Ok(())
    }

    /// Render a complete line of markdown
    fn render_line(&mut self, line: &str) -> io::Result<()> {
        // Handle code block boundaries
        if line.trim_start().starts_with("```") {
            if self.in_code_block {
                // End code block
                self.in_code_block = false;
                self.code_block_language.clear();
                println!("{}", style("```").dim());
            } else {
                // Start code block
                self.in_code_block = true;
                self.code_block_language = line.trim_start()[3..].to_string();
                println!("{}", style(line).dim());
            }
            return Ok(());
        }
        
        // If we're in a code block, render as-is
        if self.in_code_block {
            println!("{}", line);
            return Ok(());
        }
        
        // Handle headers
        if line.starts_with("# ") {
            println!("\n{}", style(&line[2..]).bold().underlined());
            return Ok(());
        } else if line.starts_with("## ") {
            println!("\n{}", style(&line[3..]).bold());
            return Ok(());
        } else if line.starts_with("### ") {
            println!("\n{}", style(&line[4..]).cyan());
            return Ok(());
        }
        
        // Handle lists
        if line.starts_with("- ") || line.starts_with("* ") {
            println!("  • {}", &line[2..]);
            return Ok(());
        }
        
        // Handle bold text (**text**)
        let line = self.process_bold_text(line);
        
        // Handle italic text (*text* or _text_)
        let line = self.process_italic_text(line);
        
        // Handle inline code (`code`)
        let line = self.process_inline_code(line);
        
        // Print the processed line
        println!("{}", line);
        Ok(())
    }

    /// Render partial line (for streaming)
    fn render_partial_line(&mut self, text: &str) -> io::Result<()> {
        // For partial lines, we simply print them without special formatting
        // to avoid formatting incomplete markdown elements
        print!("{}", text);
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
                    result.push_str(&style("").reset().to_string());
                    in_bold = false;
                } else {
                    result.push_str(&style("").bold().to_string());
                    in_bold = true;
                }
            } else {
                result.push(ch);
            }
        }
        
        // Reset formatting at the end
        if in_bold {
            result.push_str(&style("").reset().to_string());
        }
        
        result
    }

    /// Process italic text (*text* or _text_)
    fn process_italic_text(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();
        let mut in_italic = false;
        let mut delimiter = ' ';
        
        while let Some(ch) = chars.next() {
            if (ch == '*' || ch == '_') && !in_italic {
                // Start italic
                delimiter = ch;
                result.push_str(&style("").italic().to_string());
                in_italic = true;
            } else if ch == delimiter && in_italic {
                // End italic
                result.push_str(&style("").reset().to_string());
                in_italic = false;
            } else {
                result.push(ch);
            }
        }
        
        // Reset formatting at the end
        if in_italic {
            result.push_str(&style("").reset().to_string());
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
                    result.push_str(&style("").reset().to_string());
                    in_code = false;
                } else {
                    result.push_str(&style("").on_black().white().to_string());
                    in_code = true;
                }
            } else {
                result.push(ch);
            }
        }
        
        // Reset formatting at the end
        if in_code {
            result.push_str(&style("").reset().to_string());
        }
        
        result
    }

    /// Flush any remaining buffered content
    pub fn flush(&mut self) -> io::Result<()> {
        if !self.buffer.is_empty() {
            self.render_line(&self.buffer)?;
            self.buffer.clear();
        }
        Ok(())
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}