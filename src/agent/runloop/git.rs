use anyhow::{Context, Result};
use std::io::{self, Write};

fn is_git_repo() -> bool {
    std::process::Command::new("git")
        .args(&["rev-parse", "--git-dir"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(crate) async fn confirm_changes_with_git_diff(
    modified_files: &[String],
    skip_confirmations: bool,
) -> Result<bool> {
    if skip_confirmations {
        return Ok(true);
    }

    if !is_git_repo() {
        println!("Not in a git repository; skipping diff confirmation.");
        return Ok(true);
    }

    for file in modified_files {
        let output = std::process::Command::new("git")
            .args(&["diff", file])
            .output()
            .with_context(|| format!("Failed to run git diff for {}", file))?;

        let diff = String::from_utf8_lossy(&output.stdout);
        if !diff.is_empty() {
            println!("Changes to {}:\n{}", file, diff);
            print!("Apply these changes? (y/n): ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                std::process::Command::new("git")
                    .args(&["checkout", "--", file])
                    .status()
                    .with_context(|| format!("Failed to revert {}", file))?;
                return Ok(false);
            }
        }
    }
    Ok(true)
}
