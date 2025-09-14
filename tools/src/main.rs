//! A tool to help migrate from console::style to anstyle in VTAgent
//!
//! USAGE:
//!   cargo run -- <file>
//!
//! DEPENDENCIES:
//!   regex = "1.0"

use regex::Regex;
use std::env;
use std::fs;
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <file>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];

    // Read the file
    let mut file = fs::File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Store original for comparison
    let original_contents = contents.clone();

    // Update imports
    contents = update_imports(contents);

    // Update style calls
    contents = update_style_calls(contents);

    // Write the updated file if changes were made
    if contents != original_contents {
        let mut file = fs::File::create(file_path)?;
        file.write_all(contents.as_bytes())?;
        println!("Updated {}", file_path);
    } else {
        println!("No changes needed for {}", file_path);
    }

    Ok(())
}

fn update_imports(contents: String) -> String {
    let import_re = Regex::new(r#"use console::style;"#).unwrap();
    import_re.replace(&contents, "use vtagent_core::ui::styled::*;").to_string()
}

fn update_style_calls(contents: String) -> String {
    let mut updated_contents = contents;

    // Define transformations for style calls
    let transformations = vec![
        // Error styling - red bold
        (r#"style\(([^)]+)\)\.red\(\)\.bold\(\)"#, "Styles::bold_error().render(), $1, Styles::bold_error().render_reset()"),

        // Warning styling - yellow bold
        (r#"style\(([^)]+)\)\.yellow\(\)\.bold\(\)"#, "Styles::bold_warning().render(), $1, Styles::bold_warning().render_reset()"),

        // Success styling - green bold
        (r#"style\(([^)]+)\)\.green\(\)\.bold\(\)"#, "Styles::bold_success().render(), $1, Styles::bold_success().render_reset()"),

        // Info styling - blue bold
        (r#"style\(([^)]+)\)\.blue\(\)\.bold\(\)"#, "Styles::header().render(), $1, Styles::header().render_reset()"),

        // Debug styling - cyan
        (r#"style\(([^)]+)\)\.cyan\(\)"#, "Styles::debug().render(), $1, Styles::debug().render_reset()"),

        // Code styling - magenta
        (r#"style\(([^)]+)\)\.magenta\(\)"#, "Styles::code().render(), $1, Styles::code().render_reset()"),

        // Simple colors
        (r#"style\(([^)]+)\)\.red\(\)"#, "Styles::error().render(), $1, Styles::error().render_reset()"),
        (r#"style\(([^)]+)\)\.green\(\)"#, "Styles::success().render(), $1, Styles::success().render_reset()"),
        (r#"style\(([^)]+)\)\.blue\(\)"#, "Styles::info().render(), $1, Styles::info().render_reset()"),
        (r#"style\(([^)]+)\)\.yellow\(\)"#, "Styles::warning().render(), $1, Styles::warning().render_reset()"),

        // Bold styling
        (r#"style\(([^)]+)\)\.bold\(\)"#, "Styles::bold().render(), $1, Styles::bold().render_reset()"),

        // Dim styling
        (r#"style\(([^)]+)\)\.dim\(\)"#, "Styles::debug().render(), $1, Styles::debug().render_reset()"),
    ];

    // Apply transformations
    for (pattern, replacement) in transformations {
        let re = Regex::new(pattern).unwrap();
        updated_contents = re.replace_all(&updated_contents, replacement).to_string();
    }

    // Update println and eprintln calls that contain style
    let print_re = Regex::new(r#"println!\("([^"]*)", (.*)\);"#).unwrap();
    updated_contents = print_re.replace_all(&updated_contents, r#"println!("{}{}{}", $2);"#).to_string();

    let eprint_re = Regex::new(r#"eprintln!\("([^"]*)", (.*)\);"#).unwrap();
    updated_contents = eprint_re.replace_all(&updated_contents, r#"eprintln!("{}{}{}", $2);"#).to_string();

    updated_contents
}