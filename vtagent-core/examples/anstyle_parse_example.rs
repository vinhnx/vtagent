//! Example demonstrating anstyle-parse usage
//!
//! This example shows how to parse ANSI escape sequences using anstyle-parse.

use anstyle_parse::{Parser, Params, Perform, Utf8Parser};

// A simple struct to collect parsed escape sequences
#[derive(Debug, Default)]
struct EscapeCollector {
    sequences: Vec<String>,
}

impl Perform for EscapeCollector {
    fn print(&mut self, character: char) {
        // Handle printable characters
        print!("{}", character);
    }

    fn execute(&mut self, byte: u8) {
        // Handle control characters
        if byte == b'\n' {
            println!();
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: u8) {
        // Handle CSI (Control Sequence Introducer) sequences
        let sequence = format!("CSI params={:?}, intermediates={:?}, ignore={}, action={}", 
            params.iter().collect::<Vec<_>>(), intermediates, ignore, action as char);
        self.sequences.push(sequence);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        // Handle OSC (Operating System Command) sequences
        let sequence = format!("OSC params={:?}, bell_terminated={}", params, bell_terminated);
        self.sequences.push(sequence);
    }
    
    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        // Handle ESC sequences
        let sequence = format!("ESC intermediates={:?}, ignore={}, byte={}", 
            intermediates, ignore, byte as char);
        self.sequences.push(sequence);
    }
}

fn main() {
    println!("=== anstyle-parse Example ===");
    
    // Sample text with ANSI escape sequences
    let sample_text = "\x1b[31mRed Text\x1b[0m Normal Text\x1b[1;32mBold Green Text\x1b[0m";
    
    println!("Parsing: {}", sample_text);
    println!("Output:");
    
    // Create a parser using the Utf8Parser
    let mut parser = Parser::<Utf8Parser>::new();
    let mut collector = EscapeCollector::default();
    
    // Parse the sample text
    for byte in sample_text.bytes() {
        parser.advance(&mut collector, byte);
    }
    
    println!("\n\nParsed escape sequences:");
    for sequence in collector.sequences {
        println!("  {}", sequence);
    }
}