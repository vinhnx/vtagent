//! Command execution tool with multiple modes

use super::traits::{ModeTool, Tool};
use super::types::*;
use crate::config::constants::tools;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
// PtySession import removed as it's not directly used
use rexpect::spawn;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;

/// Command execution tool with multiple modes
#[derive(Clone)]
pub struct CommandTool {
    workspace_root: PathBuf,
}

impl CommandTool {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Execute standard terminal command
    async fn execute_terminal_command(&self, input: &EnhancedTerminalInput) -> Result<Value> {
        if input.command.is_empty() {
            return Err(anyhow!("command array cannot be empty"));
        }

        let program = &input.command[0];
        let cmd_args = &input.command[1..];

        // Build the command
        let mut cmd = tokio::process::Command::new(program);
        cmd.args(cmd_args);

        // Set working directory if provided
        if let Some(ref working_dir) = input.working_dir {
            let work_path = self.workspace_root.join(working_dir);
            cmd.current_dir(work_path);
        } else {
            cmd.current_dir(&self.workspace_root);
        }

        // Set timeout
        let timeout = Duration::from_secs(input.timeout_secs.unwrap_or(30));

        // Execute with timeout
        let output = tokio::time::timeout(timeout, cmd.output())
            .await
            .map_err(|_| anyhow!("Command timed out after {} seconds", timeout.as_secs()))?
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(json!({
            "success": output.status.success(),
            "exit_code": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
            "mode": "terminal",
            "command": input.command.join(" ")
        }))
    }

    /// Execute PTY command (for both pty and streaming modes)
    async fn execute_pty_command(&self, input: &EnhancedTerminalInput) -> Result<Value> {
        if input.command.is_empty() {
            return Err(anyhow!("command array cannot be empty"));
        }

        let full_command = input.command.join(" ");

        // Change to working directory if provided
        let work_dir = if let Some(ref working_dir) = input.working_dir {
            self.workspace_root.join(working_dir)
        } else {
            self.workspace_root.clone()
        };

        // Set timeout
        let timeout_ms = input.timeout_secs.unwrap_or(30) * 1000;

        // Show command execution start for shell-like experience
        if input.mode.as_deref() == Some("streaming") {
            println!("$ {}", full_command);
        }

        // Execute command in PTY
        let mut pty_session = spawn(&full_command, Some(timeout_ms))
            .map_err(|e| anyhow!("Failed to spawn PTY session: {}", e))?;

        // Change directory
        pty_session
            .send_line(&format!("cd {}", work_dir.display()))
            .map_err(|e| anyhow!("Failed to change directory: {}", e))?;

        // For streaming mode, show a progress indicator while waiting
        if input.mode.as_deref() == Some("streaming") {
            println!("Executing command in PTY session...");
        }

        // Wait for command to complete and capture output
        let output = pty_session
            .exp_eof()
            .map_err(|e| anyhow!("PTY session failed: {}", e))?;

        Ok(json!({
            "success": true,
            "exit_code": 0,
            "stdout": output,
            "stderr": "",
            "mode": "pty",
            "pty_enabled": true,
            "streaming": input.mode.as_deref() == Some("streaming"),
            "shell_rendered": true,
            "command": full_command
        }))
    }

    /// Execute streaming command (similar to PTY but with streaming indication)
    async fn execute_streaming_command(&self, input: &EnhancedTerminalInput) -> Result<Value> {
        // Use PTY implementation for streaming as well, since PTY provides pseudo-terminal capabilities
        let mut result = self.execute_pty_command(input).await?;

        // Mark as streaming mode
        if let Some(obj) = result.as_object_mut() {
            obj.insert("mode".to_string(), json!("streaming"));
            obj.insert("streaming_enabled".to_string(), json!(true));
        }

        Ok(result)
    }

    /// Validate command for security
    fn validate_command(&self, command: &[String]) -> Result<()> {
        if command.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        let program = &command[0];

        // Basic security checks
        let dangerous_commands = ["rm", "rmdir", "del", "format", "fdisk"];
        if dangerous_commands.contains(&program.as_str()) {
            return Err(anyhow!("Dangerous command not allowed: {}", program));
        }

        // Check for suspicious patterns
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

        // Validate command for security
        self.validate_command(&input.command)?;

        let mode_clone = input.mode.clone();
        let mode = mode_clone.as_deref().unwrap_or("terminal");
        self.execute_mode(mode, serde_json::to_value(input)?).await
    }

    fn name(&self) -> &'static str {
        tools::RUN_TERMINAL_CMD
    }

    fn description(&self) -> &'static str {
        "Enhanced command execution tool with multiple modes: terminal (default), pty, streaming"
    }

    fn validate_args(&self, args: &Value) -> Result<()> {
        let input: EnhancedTerminalInput = serde_json::from_value(args.clone())?;
        self.validate_command(&input.command)
    }
}

#[async_trait]
impl ModeTool for CommandTool {
    fn supported_modes(&self) -> Vec<&'static str> {
        vec!["terminal", "pty", "streaming"]
    }

    async fn execute_mode(&self, mode: &str, args: Value) -> Result<Value> {
        let input: EnhancedTerminalInput = serde_json::from_value(args)?;

        match mode {
            "terminal" => self.execute_terminal_command(&input).await,
            "pty" => self.execute_pty_command(&input).await,
            "streaming" => self.execute_streaming_command(&input).await,
            _ => Err(anyhow!("Unsupported command execution mode: {}", mode)),
        }
    }
}
