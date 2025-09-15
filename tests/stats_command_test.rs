use anyhow::Result;
use tempfile::TempDir;
use tokio::time::{Duration, sleep};
use vtagent_core::{
    Agent, config::constants::models::google::GEMINI_2_5_FLASH_LITE, config::types::AgentConfig,
    handle_stats_command,
};

#[tokio::test]
async fn test_handle_stats_command_returns_agent_metrics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = AgentConfig {
        model: GEMINI_2_5_FLASH_LITE.to_string(),
        api_key: "test_key".to_string(),
        workspace: temp_dir.path().to_path_buf(),
        verbose: false,
    };
    let mut agent = Agent::new(config)?;
    agent.update_session_stats(5, 3, 1);
    sleep(Duration::from_millis(10)).await;
    let metrics = handle_stats_command(&agent, false, "json".to_string()).await?;
    assert_eq!(metrics.total_api_calls, 5);
    assert_eq!(metrics.tool_execution_count, 3);
    assert_eq!(metrics.error_count, 1);
    assert!(metrics.session_duration_seconds > 0);
    Ok(())
}
