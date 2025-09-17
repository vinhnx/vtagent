//! Speckit tool for spec-driven development
//!
//! This tool integrates Speckit (Spec Kit) for spec-driven development workflows.
//! Speckit enables structured development through specifications that generate
//! code, tests, and documentation automatically.

use super::traits::Tool;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::process::Command;

/// Speckit tool for spec-driven development
#[derive(Clone)]
pub struct SpeckitTool {
    workspace_root: PathBuf,
}

impl SpeckitTool {
    /// Create a new Speckit tool
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Execute Speckit command via uvx
    async fn execute_speckit_command(
        &self,
        speckit_command: &str,
        args: Vec<String>,
    ) -> Result<Value> {
        // Build the full uvx command
        let mut full_args = vec![
            "--from".to_string(),
            "git+https://github.com/github/spec-kit.git".to_string(),
            "specify".to_string(),
        ];

        // Add the Speckit subcommand
        full_args.push(speckit_command.to_string());

        // Add any additional arguments
        full_args.extend(args);

        let full_command = format!("uvx {}", full_args.join(" "));

        // Execute the command
        let work_dir = self.workspace_root.clone();
        let mut cmd = Command::new("uvx");
        cmd.args(&full_args);
        cmd.current_dir(&work_dir);

        let output = cmd
            .output()
            .await
            .with_context(|| format!("Failed to execute Speckit command: {}", full_command))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(json!({
            "success": output.status.success(),
            "exit_code": output.status.code().unwrap_or_default(),
            "stdout": stdout,
            "stderr": stderr,
            "command": full_command,
            "working_directory": work_dir.display().to_string(),
            "speckit_command": speckit_command
        }))
    }

    /// Validate Speckit command arguments
    fn validate_speckit_args(&self, command: &str, args: &[String]) -> Result<()> {
        match command {
            "init" => {
                if args.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Speckit init requires a project name or --here flag"
                    ));
                }
            }
            "check" => {
                // check command doesn't require arguments
            }
            "/specify" => {
                if args.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Speckit /specify requires a specification description"
                    ));
                }
            }
            "/plan" => {
                // plan command can work with current directory context
            }
            "/tasks" => {
                // tasks command can work with current directory context
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown Speckit command: {}", command));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Tool for SpeckitTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        // Validate arguments first
        self.validate_args(&args)?;

        let command = args
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        let speckit_args = args
            .get("args")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        // Execute the Speckit command
        self.execute_speckit_command(command, speckit_args).await
    }

    fn name(&self) -> &'static str {
        "speckit"
    }

    fn description(&self) -> &'static str {
        "Speckit tool for project initialization and system verification. Supports commands: init, check"
    }

    fn validate_args(&self, args: &Value) -> Result<()> {
        if !args.is_object() {
            return Err(anyhow::anyhow!("Arguments must be an object"));
        }

        let obj = args.as_object().unwrap();

        if !obj.contains_key("command") {
            return Err(anyhow::anyhow!("Missing required 'command' parameter"));
        }

        let command = obj
            .get("command")
            .unwrap()
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'command' must be a string"))?;

        // Validate command is supported - only init and check are available
        match command {
            "init" | "check" => {}
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported Speckit command: {}. Available commands: init, check",
                    command
                ));
            }
        }

        Ok(())
    }
}
