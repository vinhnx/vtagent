//! Terminal streaming utilities for displaying real-time token streams with animation
//!
//! This module provides utilities for displaying streaming tokens from LLM APIs
//! with optional typing animation effects.

use crate::llm::provider::StreamToken;
use futures::StreamExt;
use tokio::io::{self, AsyncWriteExt};
use tokio::time::{Duration, sleep};
use tokio_stream::Stream;

/// Configuration for terminal streaming
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TerminalStreamingConfig {
    /// Whether streaming is enabled
    pub enabled: bool,
    /// Delay between characters for typing animation (in milliseconds)
    pub typing_delay_ms: u64,
    /// Whether to enable typing animation
    pub enable_animation: bool,
    /// Prefix to display before each response
    pub prefix: String,
}

impl Default for TerminalStreamingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            typing_delay_ms: 50,
            enable_animation: false,
            prefix: "Assistant: ".to_string(),
        }
    }
}

/// Terminal streamer for displaying streaming tokens
pub struct TerminalStreamer {
    stdout: io::Stdout,
    config: TerminalStreamingConfig,
}

impl TerminalStreamer {
    /// Create a new terminal streamer with default configuration
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
            config: TerminalStreamingConfig::default(),
        }
    }

    /// Create a new terminal streamer with custom configuration
    pub fn with_config(config: TerminalStreamingConfig) -> Self {
        Self {
            stdout: io::stdout(),
            config,
        }
    }

    /// Stream response tokens to the terminal
    ///
    /// This method consumes a stream of tokens and displays them in real-time.
    /// If animation is enabled, it will simulate typing by adding delays between characters.
    /// Returns the collected content and finish reason.
    pub async fn stream_response<S>(
        &mut self,
        mut stream: S,
    ) -> Result<(String, Option<String>), Box<dyn std::error::Error>>
    where
        S: Stream<Item = Result<StreamToken, Box<dyn std::error::Error + Send + Sync>>> + Unpin,
    {
        let mut total_tokens = 0;
        let mut collected_content = String::new();
        let mut finish_reason = None;

        while let Some(result) = stream.next().await {
            match result {
                Ok(token) => {
                    if !token.text.is_empty() {
                        collected_content.push_str(&token.text);

                        if self.config.enable_animation {
                            // Simulate typing animation
                            for char in token.text.chars() {
                                print!("{}", char);
                                self.stdout.flush().await?;
                                sleep(Duration::from_millis(self.config.typing_delay_ms)).await;
                            }
                        } else {
                            // Direct output without animation
                            print!("{}", token.text);
                            self.stdout.flush().await?;
                        }
                        total_tokens += 1;
                    }

                    if token.is_final {
                        finish_reason = token.finish_reason.clone();
                        println!(); // New line after completion
                        if let Some(reason) = &token.finish_reason {
                            if reason != "STOP" {
                                println!("Finished: {} (tokens: {})", reason, total_tokens);
                            }
                        }
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("\nError: {}", e);
                    break;
                }
            }
        }

        Ok((collected_content, finish_reason))
    }
}

/// Convenience function to create a basic terminal streamer
pub fn create_terminal_streamer() -> TerminalStreamer {
    TerminalStreamer::new()
}

/// Convenience function to create an animated terminal streamer
pub fn create_animated_streamer(typing_delay_ms: u64) -> TerminalStreamer {
    let config = TerminalStreamingConfig {
        typing_delay_ms,
        enable_animation: true,
        ..Default::default()
    };
    TerminalStreamer::with_config(config)
}

/// Interactive chat loop using streaming
pub async fn run_interactive_streaming<F>(
    create_stream: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(&str) -> Box<dyn Stream<Item = Result<StreamToken, Box<dyn std::error::Error + Send + Sync>>> + Unpin>,
{
    println!("Interactive Streaming Chat");
    println!("==========================");
    println!("Type 'quit' to exit, 'animate' to toggle animation, 'delay <ms>' to set delay");

    let mut streamer = create_terminal_streamer();
    let mut input = String::new();

    loop {
        print!("\nYou: ");
        io::stdout().flush().await?;

        input.clear();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "quit" => break,
            "animate" => {
                streamer.config.enable_animation = !streamer.config.enable_animation;
                println!("Animation: {}", if streamer.config.enable_animation { "ON" } else { "OFF" });
                continue;
            }
            cmd if cmd.starts_with("delay ") => {
                if let Some(delay_str) = cmd.strip_prefix("delay ") {
                    if let Ok(delay) = delay_str.parse::<u64>() {
                        streamer.config.typing_delay_ms = delay;
                        println!("Typing delay set to {}ms", delay);
                    } else {
                        println!("Invalid delay value");
                    }
                }
                continue;
            }
            "" => continue,
            _ => {}
        }

        if !input.is_empty() {
            let stream = create_stream(input);
            streamer.stream_response(stream).await?;
        }
    }

    println!("Goodbye!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::{StreamExt, wrappers::ReceiverStream};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_terminal_streamer_creation() {
        let streamer = create_terminal_streamer();
        assert!(!streamer.config.enable_animation);
        assert_eq!(streamer.config.typing_delay_ms, 50);
    }

    #[tokio::test]
    async fn test_animated_streamer_creation() {
        let streamer = create_animated_streamer(100);
        assert!(streamer.config.enable_animation);
        assert_eq!(streamer.config.typing_delay_ms, 100);
    }

    #[tokio::test]
    async fn test_stream_token_creation() {
        let token = StreamToken {
            text: "Hello".to_string(),
            is_final: false,
            finish_reason: None,
        };
        assert_eq!(token.text, "Hello");
        assert!(!token.is_final);
    }
}