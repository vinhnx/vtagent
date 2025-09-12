//! Bash-like tool using direct command execution
//!
//! This tool provides bash-like functionality by directly executing
//! common shell commands without complex abstractions.

use super::traits::Tool;
use crate::bash_runner::BashRunner;
use crate::config::constants::tools;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;

/// Bash-like tool for direct command execution
#[derive(Clone)]
pub struct BashTool {
    runner: BashRunner,
}

impl BashTool {
    /// Create a new bash tool
    pub fn new(workspace_root: PathBuf) -> Self {
        let runner = BashRunner::new(workspace_root);
        Self { runner }
    }

    /// Execute ls command
    async fn execute_ls(&self, args: Value) -> Result<Value> {
        let path = args.get("path").and_then(|v| v.as_str());
        let show_hidden = args.get("show_hidden").and_then(|v| v.as_bool()).unwrap_or(false);

        let result = self.runner.ls(path, show_hidden)?;
        Ok(json!({
            "command": "ls",
            "path": path,
            "show_hidden": show_hidden,
            "output": result
        }))
    }

    /// Execute pwd command
    async fn execute_pwd(&self) -> Result<Value> {
        let result = self.runner.pwd();
        Ok(json!({
            "command": "pwd",
            "output": result
        }))
    }

    /// Execute grep command
    async fn execute_grep(&self, args: Value) -> Result<Value> {
        let pattern = args.get("pattern")
            .and_then(|v| v.as_str())
            .context("pattern is required for grep")?;

        let path = args.get("path").and_then(|v| v.as_str());
        let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);

        let result = self.runner.grep(pattern, path, recursive)?;
        Ok(json!({
            "command": "grep",
            "pattern": pattern,
            "path": path,
            "recursive": recursive,
            "output": result
        }))
    }

    /// Execute find command
    async fn execute_find(&self, args: Value) -> Result<Value> {
        let path = args.get("path").and_then(|v| v.as_str());
        let name_pattern = args.get("name_pattern").and_then(|v| v.as_str());
        let type_filter = args.get("type_filter").and_then(|v| v.as_str());

        let result = self.runner.find(path, name_pattern, type_filter)?;
        Ok(json!({
            "command": "find",
            "path": path,
            "name_pattern": name_pattern,
            "type_filter": type_filter,
            "output": result
        }))
    }

    /// Execute cat command
    async fn execute_cat(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .context("path is required for cat")?;

        let start_line = args.get("start_line").and_then(|v| v.as_u64()).map(|v| v as usize);
        let end_line = args.get("end_line").and_then(|v| v.as_u64()).map(|v| v as usize);

        let result = self.runner.cat(path, start_line, end_line)?;
        Ok(json!({
            "command": "cat",
            "path": path,
            "start_line": start_line,
            "end_line": end_line,
            "output": result
        }))
    }

    /// Execute head command
    async fn execute_head(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .context("path is required for head")?;

        let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let result = self.runner.head(path, lines)?;
        Ok(json!({
            "command": "head",
            "path": path,
            "lines": lines,
            "output": result
        }))
    }

    /// Execute tail command
    async fn execute_tail(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .context("path is required for tail")?;

        let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        let result = self.runner.tail(path, lines)?;
        Ok(json!({
            "command": "tail",
            "path": path,
            "lines": lines,
            "output": result
        }))
    }

    /// Execute mkdir command
    async fn execute_mkdir(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .context("path is required for mkdir")?;

        let parents = args.get("parents").and_then(|v| v.as_bool()).unwrap_or(false);

        self.runner.mkdir(path, parents)?;
        Ok(json!({
            "command": "mkdir",
            "path": path,
            "parents": parents,
            "status": "success"
        }))
    }

    /// Execute rm command
    async fn execute_rm(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .context("path is required for rm")?;

        let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
        let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

        self.runner.rm(path, recursive, force)?;
        Ok(json!({
            "command": "rm",
            "path": path,
            "recursive": recursive,
            "force": force,
            "status": "success"
        }))
    }

    /// Execute cp command
    async fn execute_cp(&self, args: Value) -> Result<Value> {
        let source = args.get("source")
            .and_then(|v| v.as_str())
            .context("source is required for cp")?;

        let dest = args.get("dest")
            .and_then(|v| v.as_str())
            .context("dest is required for cp")?;

        let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);

        self.runner.cp(source, dest, recursive)?;
        Ok(json!({
            "command": "cp",
            "source": source,
            "dest": dest,
            "recursive": recursive,
            "status": "success"
        }))
    }

    /// Execute mv command
    async fn execute_mv(&self, args: Value) -> Result<Value> {
        let source = args.get("source")
            .and_then(|v| v.as_str())
            .context("source is required for mv")?;

        let dest = args.get("dest")
            .and_then(|v| v.as_str())
            .context("dest is required for mv")?;

        self.runner.mv(source, dest)?;
        Ok(json!({
            "command": "mv",
            "source": source,
            "dest": dest,
            "status": "success"
        }))
    }

    /// Execute stat command
    async fn execute_stat(&self, args: Value) -> Result<Value> {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .context("path is required for stat")?;

        let result = self.runner.stat(path)?;
        Ok(json!({
            "command": "stat",
            "path": path,
            "output": result
        }))
    }

    /// Execute arbitrary command
    async fn execute_run(&self, args: Value) -> Result<Value> {
        let command = args.get("command")
            .and_then(|v| v.as_str())
            .context("command is required for run")?;

        let cmd_args = args.get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<&str>>())
            .unwrap_or_default();

        let result = self.runner.run(command, &cmd_args)?;
        Ok(json!({
            "command": "run",
            "executed_command": command,
            "args": cmd_args,
            "output": result
        }))
    }
}

#[async_trait]
impl Tool for BashTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        let command = args.get("bash_command")
            .and_then(|v| v.as_str())
            .unwrap_or("ls");

        match command {
            "ls" => self.execute_ls(args).await,
            "pwd" => self.execute_pwd().await,
            "grep" => self.execute_grep(args).await,
            "find" => self.execute_find(args).await,
            "cat" => self.execute_cat(args).await,
            "head" => self.execute_head(args).await,
            "tail" => self.execute_tail(args).await,
            "mkdir" => self.execute_mkdir(args).await,
            "rm" => self.execute_rm(args).await,
            "cp" => self.execute_cp(args).await,
            "mv" => self.execute_mv(args).await,
            "stat" => self.execute_stat(args).await,
            "run" => self.execute_run(args).await,
            _ => Err(anyhow::anyhow!("Unknown bash command: {}", command)),
        }
    }

    fn name(&self) -> &'static str {
        tools::BASH
    }

    fn description(&self) -> &'static str {
        "Bash-like commands: ls, pwd, grep, find, cat, head, tail, mkdir, rm, cp, mv, stat, run"
    }
}