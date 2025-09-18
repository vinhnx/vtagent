use anyhow::Result;
use console::style;
use std::fs;
use vtcode_core::config::types::AgentConfig as CoreAgentConfig;

pub async fn handle_revert_command(
    config: &CoreAgentConfig,
    turn: usize,
    partial: Option<String>,
) -> Result<()> {
    println!("{}", style("Revert Agent State").blue().bold());
    let file = config
        .workspace
        .join("snapshots")
        .join(format!("turn_{}.json", turn));
    if !file.exists() {
        println!("Snapshot not found: {}", file.display());
        return Ok(());
    }
    let data = fs::read_to_string(&file)?;
    println!(
        "Found snapshot file: {} ({} bytes)",
        file.display(),
        data.len()
    );
    println!("Note: full state revert requires a running Agent; printing metadata only.");
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&data)
        && let Some(meta) = val.get("metadata")
    {
        println!("metadata: {}", meta);
    }
    if let Some(p) = partial {
        println!("Requested partial revert: {} (not applied)", p);
    }
    Ok(())
}
