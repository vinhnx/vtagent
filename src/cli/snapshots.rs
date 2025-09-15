use anyhow::Result;
use console::style;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::core::agent::snapshots::{SnapshotConfig, SnapshotManager};

pub async fn handle_snapshots_command(config: &CoreAgentConfig) -> Result<()> {
    println!("{}", style("Available Snapshots").blue().bold());
    let snap_dir = config.workspace.join("snapshots");
    let manager = SnapshotManager::new(SnapshotConfig {
        directory: snap_dir,
        ..Default::default()
    });
    let snaps = manager.list_snapshots().await?;
    if snaps.is_empty() {
        println!("(none)");
    } else {
        for s in snaps {
            println!(
                "- turn {}  size={}B  created={}  file={}",
                s.turn_number, s.size_bytes, s.created_at, s.filename
            );
        }
    }
    Ok(())
}

pub async fn handle_cleanup_snapshots_command(
    config: &CoreAgentConfig,
    max: Option<usize>,
) -> Result<()> {
    println!("{}", style("Cleanup Snapshots").blue().bold());
    let snap_dir = config.workspace.join("snapshots");
    let mut cfg = SnapshotConfig {
        directory: snap_dir,
        ..Default::default()
    };
    if let Some(m) = max {
        cfg.max_snapshots = m;
        println!("Keeping maximum {} snapshots...", m);
    }
    let manager = SnapshotManager::new(cfg);
    manager.cleanup_old_snapshots().await?;
    println!("Cleanup complete.");
    Ok(())
}
