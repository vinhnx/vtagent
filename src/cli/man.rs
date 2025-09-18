use anyhow::{Context, Result};
use vtcode_core::cli::ManPageGenerator;

pub async fn handle_man_command(
    command: Option<String>,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let content = match command.as_deref() {
        Some(cmd) => ManPageGenerator::generate_command_man_page(cmd)
            .with_context(|| format!("Failed to generate man page for {}", cmd))?,
        None => ManPageGenerator::generate_main_man_page()
            .context("Failed to generate main man page")?,
    };

    if let Some(path) = output {
        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write man page to {}", path.display()))?;
        println!("Wrote man page to {}", path.display());
    } else {
        println!("{}", content);
    }

    Ok(())
}
