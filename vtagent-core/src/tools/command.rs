//! Command execution tool

use super::traits::{ModeTool, Tool};
use super::types::*;
use crate::config::constants::tools;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{path::PathBuf, process::Stdio};
use tokio::process::Command;

/// Command execution tool using standard process handling
#[derive(Clone)]
pub struct CommandTool {
    workspace_root: PathBuf,
}

impl CommandTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    async fn execute_terminal_command(&self, input: &EnhancedTerminalInput) -> Result<Value> {
        if input.command.is_empty() {
            return Err(anyhow!("command array cannot be empty"));
        }

        let mut cmd = Command::new(&input.command[0]);
        if input.command.len() > 1 {
            cmd.args(&input.command[1..]);
        }

        let work_dir = if let Some(ref working_dir) = input.working_dir {
            self.workspace_root.join(working_dir)
        } else {
            self.workspace_root.clone()
        };

        cmd.current_dir(work_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .map_err(|e| anyhow!("failed to run command: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(json!({
            "success": output.status.success(),
            "exit_code": output.status.code().unwrap_or_default(),
            "stdout": stdout,
            "stderr": stderr,
            "mode": "terminal",
            "pty_enabled": false,
            "command": input.command.join(" ")
        }))
    }

    fn validate_command(&self, command: &[String]) -> Result<()> {
        if command.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        let program = &command[0];
        let dangerous_commands = ["rm", "rmdir", "del", "format", "fdisk"];
        if dangerous_commands.contains(&program.as_str()) {
            return Err(anyhow!("Dangerous command not allowed: {}", program));
        }

        let full_command = command.join(" ");
        if full_command.contains("rm -rf /") || full_command.contains("sudo rm") {
            return Err(anyhow!("Potentially dangerous command pattern detected"));
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for CommandTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let input: EnhancedTerminalInput = serde_json::from_value(args)?;
        self.validate_command(&input.command)?;
        self.execute_terminal_command(&input).await
    }

    fn name(&self) -> &'static str {
        tools::RUN_TERMINAL_CMD
    }

    fn description(&self) -> &'static str {
        "Execute terminal commands"
    }

    fn validate_args(&self, args: &Value) -> Result<()> {
        let input: EnhancedTerminalInput = serde_json::from_value(args.clone())?;
        self.validate_command(&input.command)
    }
}

#[async_trait]
impl ModeTool for CommandTool {
    fn supported_modes(&self) -> Vec<&'static str> {
        vec!["terminal"]
    }

    async fn execute_mode(&self, mode: &str, args: Value) -> Result<Value> {
        let input: EnhancedTerminalInput = serde_json::from_value(args)?;
        match mode {
            "terminal" => self.execute_terminal_command(&input).await,
            _ => Err(anyhow!("Unsupported command execution mode: {}", mode)),
        }
    }
}
