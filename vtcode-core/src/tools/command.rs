//! Command execution tool

use super::traits::{ModeTool, Tool};
use super::types::*;
use crate::config::constants::tools;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{process::Command, time::timeout};

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

        // Check if command contains shell metacharacters that require shell interpretation
        let full_command = input.command.join(" ");
        let has_shell_metacharacters = full_command.contains('|')
            || full_command.contains('>')
            || full_command.contains('<')
            || full_command.contains('&')
            || full_command.contains(';')
            || full_command.contains('(')
            || full_command.contains(')')
            || full_command.contains('$')
            || full_command.contains('`')
            || full_command.contains('*')
            || full_command.contains('?')
            || full_command.contains('[')
            || full_command.contains(']')
            || full_command.contains('{')
            || full_command.contains('}');

        let (program, args) = if has_shell_metacharacters {
            // Use shell to interpret metacharacters
            ("sh", vec!["-c".to_string(), full_command])
        } else {
            // Execute directly
            (input.command[0].as_str(), input.command[1..].to_vec())
        };

        let mut cmd = Command::new(program);
        cmd.args(&args);

        let work_dir = if let Some(ref working_dir) = input.working_dir {
            self.workspace_root.join(working_dir)
        } else {
            self.workspace_root.clone()
        };

        cmd.current_dir(work_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let duration = Duration::from_secs(input.timeout_secs.unwrap_or(30));
        let command_str = input.command.join(" ");
        let output = timeout(duration, cmd.output())
            .await
            .with_context(|| {
                format!(
                    "command '{}' timed out after {}s",
                    command_str,
                    duration.as_secs()
                )
            })?
            .with_context(|| format!("failed to run command: {}", command_str))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(json!({
            "success": output.status.success(),
            "exit_code": output.status.code().unwrap_or_default(),
            "stdout": stdout,
            "stderr": stderr,
            "mode": "terminal",
            "pty_enabled": false,
            "command": command_str,
            "used_shell": has_shell_metacharacters
        }))
    }

    fn validate_command(&self, command: &[String]) -> Result<()> {
        if command.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        let program = &command[0];
        let full_command = command.join(" ");

        // If this is a shell command (sh -c), validate the actual command being executed
        if program == "sh" && command.len() >= 3 && command[1] == "-c" {
            let actual_command = &command[2];

            // Check for extremely dangerous patterns even in shell commands
            if actual_command.contains("rm -rf /")
                || actual_command.contains("sudo rm")
                || actual_command.contains("format")
                || actual_command.contains("fdisk")
                || actual_command.contains("mkfs")
            {
                return Err(anyhow!(
                    "Potentially dangerous command pattern detected in shell command"
                ));
            }

            return Ok(());
        }

        // For direct commands, check the program name
        let dangerous_commands = ["rm", "rmdir", "del", "format", "fdisk", "mkfs", "dd"];
        if dangerous_commands.contains(&program.as_str()) {
            return Err(anyhow!("Dangerous command not allowed: {}", program));
        }

        // Check for dangerous patterns in the full command
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
