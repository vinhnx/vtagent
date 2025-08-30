use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

/// A simple markdown renderer for streaming output
struct MarkdownRenderer {
    buffer: String,
}

impl MarkdownRenderer {
    fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Process a chunk of text and render it with basic markdown formatting
    fn render_chunk(&mut self, chunk: &str) -> io::Result<()> {
        self.buffer.push_str(chunk);

        // Process complete lines
        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].to_string();
            self.buffer.drain(..=newline_pos);
            self.render_line(&line)?;
        }

        // Render any remaining partial line
        if !self.buffer.is_empty() {
            print!("{}", self.buffer);
            io::stdout().flush()?;
        }

        Ok(())
    }

    /// Render a complete line with basic markdown formatting
    fn render_line(&self, line: &str) -> io::Result<()> {
        // Handle headers
        if line.starts_with("# ") {
            println!("\n\x1b[1m{}\x1b[0m", &line[2..]); // Bold
            return Ok(());
        } else if line.starts_with("## ") {
            println!("\n\x1b[1m{}\x1b[0m", &line[3..]); // Bold
            return Ok(());
        }

        // Handle lists
        if line.starts_with("- ") || line.starts_with("* ") {
            println!("  \x1b[36m•\x1b[0m {}", &line[2..]); // Cyan bullet
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

    /// Flush any remaining buffered content
    fn flush(&mut self) -> io::Result<()> {
        if !self.buffer.is_empty() {
            println!("{}", self.buffer);
            self.buffer.clear();
            io::stdout().flush()?;
        }
        Ok(())
    }
}

/// Simple CLI with markdown streaming
#[derive(Parser, Debug)]
#[command(
    name = "markdown-streamer",
    version,
    about = "Demonstrates markdown streaming text rendering"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Stream markdown text with formatting
    Stream { text: Vec<String> },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Stream { text } => {
            let mut renderer = MarkdownRenderer::new();

            let full_text = text.join(" ");

            println!("Streaming markdown text:");
            println!("======================");

            // Simulate streaming by processing character by character
            for ch in full_text.chars() {
                renderer.render_chunk(&ch.to_string())?;
                // Small delay to simulate real streaming
                thread::sleep(Duration::from_millis(10));
            }

            renderer.flush()?;
        }
    }

    Ok(())
}
