//! Loading spinner utilities for terminal UI

use console::style;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Show a loading spinner with the given message
pub fn show_loading_spinner(message: &str) {
    println!("{}", style(format!("⏳ {}", message)).cyan());
}

/// Start a loading spinner in a background thread
pub fn start_loading_spinner(
    is_loading: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
) -> std::thread::JoinHandle<()> {
    thread::spawn(move || {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

        while is_loading.load(Ordering::Relaxed) {
            for &spinner in &spinner_chars {
                if !is_loading.load(Ordering::Relaxed) {
                    break;
                }

                let current_status = {
                    let status_guard = status.lock().unwrap();
                    status_guard.clone()
                };

                print!("\r{} {}", style(spinner).cyan(), style(&current_status).dim());
                std::io::Write::flush(&mut std::io::stdout()).ok();

                thread::sleep(Duration::from_millis(100));
            }
        }

        // Clear the spinner line
        print!("\r{}{}", " ".repeat(60), "\r");
        std::io::Write::flush(&mut std::io::stdout()).ok();
    })
}
