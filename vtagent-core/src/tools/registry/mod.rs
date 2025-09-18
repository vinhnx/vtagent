//! Tool registry and function declarations

mod astgrep;
mod builtins;
mod cache;
mod declarations;
mod error;
mod executors;
mod legacy;
mod policy;
mod pty;
mod registration;
mod utils;

pub use declarations::{build_function_declarations, build_function_declarations_for_level};
pub use error::{ToolErrorType, ToolExecutionError, classify_error};
pub use registration::{ToolExecutorFn, ToolHandler, ToolRegistration};

use builtins::register_builtin_tools;
use utils::normalize_tool_output;

use crate::config::PtyConfig;
use crate::tool_policy::ToolPolicyManager;
use crate::tools::ast_grep::AstGrepEngine;
use crate::tools::grep_search::GrepSearchManager;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use super::bash_tool::BashTool;
use super::command::CommandTool;
use super::file_ops::FileOpsTool;
use super::search::SearchTool;
use super::simple_search::SimpleSearchTool;
use super::srgn::SrgnTool;

#[cfg(test)]
use super::traits::Tool;
#[cfg(test)]
use crate::config::constants::tools;
#[cfg(test)]
use crate::config::types::CapabilityLevel;

#[derive(Clone)]
pub struct ToolRegistry {
    workspace_root: PathBuf,
    search_tool: SearchTool,
    simple_search_tool: SimpleSearchTool,
    bash_tool: BashTool,
    file_ops_tool: FileOpsTool,
    command_tool: CommandTool,
    grep_search: Arc<GrepSearchManager>,
    ast_grep_engine: Option<Arc<AstGrepEngine>>,
    tool_policy: Option<ToolPolicyManager>,
    pty_config: PtyConfig,
    active_pty_sessions: Arc<AtomicUsize>,
    srgn_tool: SrgnTool,
    tool_registrations: Vec<ToolRegistration>,
    tool_lookup: HashMap<&'static str, usize>,
    preapproved_tools: HashSet<String>,
}

impl ToolRegistry {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self::new_with_config(workspace_root, PtyConfig::default())
    }

    pub fn new_with_config(workspace_root: PathBuf, pty_config: PtyConfig) -> Self {
        let grep_search = Arc::new(GrepSearchManager::new(workspace_root.clone()));

        let search_tool = SearchTool::new(workspace_root.clone(), grep_search.clone());
        let simple_search_tool = SimpleSearchTool::new(workspace_root.clone());
        let bash_tool = BashTool::new(workspace_root.clone());
        let file_ops_tool = FileOpsTool::new(workspace_root.clone(), grep_search.clone());
        let command_tool = CommandTool::new(workspace_root.clone());
        let srgn_tool = SrgnTool::new(workspace_root.clone());

        let ast_grep_engine = match AstGrepEngine::new() {
            Ok(engine) => Some(Arc::new(engine)),
            Err(err) => {
                eprintln!("Warning: Failed to initialize AST-grep engine: {}", err);
                None
            }
        };

        let policy_manager = match ToolPolicyManager::new_with_workspace(&workspace_root) {
            Ok(manager) => Some(manager),
            Err(err) => {
                eprintln!("Warning: Failed to initialize tool policy manager: {}", err);
                None
            }
        };

        let mut registry = Self {
            workspace_root,
            search_tool,
            simple_search_tool,
            bash_tool,
            file_ops_tool,
            command_tool,
            grep_search,
            ast_grep_engine,
            tool_policy: policy_manager,
            pty_config,
            active_pty_sessions: Arc::new(AtomicUsize::new(0)),
            srgn_tool,
            tool_registrations: Vec::new(),
            tool_lookup: HashMap::new(),
            preapproved_tools: HashSet::new(),
        };

        register_builtin_tools(&mut registry);
        registry
    }

    pub fn register_tool(&mut self, registration: ToolRegistration) -> Result<()> {
        if self.tool_lookup.contains_key(registration.name()) {
            return Err(anyhow!(format!(
                "Tool '{}' is already registered",
                registration.name()
            )));
        }

        let index = self.tool_registrations.len();
        self.tool_lookup.insert(registration.name(), index);
        self.tool_registrations.push(registration);
        Ok(())
    }

    pub fn available_tools(&self) -> Vec<String> {
        self.tool_registrations
            .iter()
            .map(|registration| registration.name().to_string())
            .collect()
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tool_lookup.contains_key(name)
    }

    pub fn with_ast_grep(mut self, engine: Arc<AstGrepEngine>) -> Self {
        self.ast_grep_engine = Some(engine);
        self
    }

    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }

    pub async fn initialize_async(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn execute_tool(&mut self, name: &str, args: Value) -> Result<Value> {
        let skip_policy_prompt = self.preapproved_tools.remove(name);

        if !skip_policy_prompt {
            if let Ok(policy_manager) = self.policy_manager_mut() {
                if !policy_manager.should_execute_tool(name)? {
                    let error = ToolExecutionError::new(
                        name.to_string(),
                        ToolErrorType::PolicyViolation,
                        format!("Tool '{}' execution denied by policy", name),
                    );
                    return Ok(error.to_json_value());
                }
            }
        }

        let args = match self.apply_policy_constraints(name, args) {
            Ok(args) => args,
            Err(err) => {
                let error = ToolExecutionError::with_original_error(
                    name.to_string(),
                    ToolErrorType::InvalidParameters,
                    "Failed to apply policy constraints".to_string(),
                    err.to_string(),
                );
                return Ok(error.to_json_value());
            }
        };

        let registration = match self
            .tool_lookup
            .get(name)
            .and_then(|index| self.tool_registrations.get(*index))
        {
            Some(registration) => registration,
            None => {
                let error = ToolExecutionError::new(
                    name.to_string(),
                    ToolErrorType::ToolNotFound,
                    format!("Unknown tool: {}", name),
                );
                return Ok(error.to_json_value());
            }
        };

        let uses_pty = registration.uses_pty();
        if uses_pty {
            if let Err(err) = self.start_pty_session() {
                let error = ToolExecutionError::with_original_error(
                    name.to_string(),
                    ToolErrorType::ExecutionError,
                    "Failed to start PTY session".to_string(),
                    err.to_string(),
                );
                return Ok(error.to_json_value());
            }
        }

        let handler = registration.handler();
        let result = match handler {
            ToolHandler::RegistryFn(executor) => executor(self, args).await,
            ToolHandler::TraitObject(tool) => tool.execute(args).await,
        };

        if uses_pty {
            self.end_pty_session();
        }

        match result {
            Ok(value) => Ok(normalize_tool_output(value)),
            Err(err) => {
                let error_type = classify_error(&err);
                let error = ToolExecutionError::with_original_error(
                    name.to_string(),
                    error_type,
                    format!("Tool execution failed: {}", err),
                    err.to_string(),
                );
                Ok(error.to_json_value())
            }
        }
    }
}

impl ToolRegistry {
    /// Prompt for permission before starting long-running tool executions to avoid spinner conflicts
    pub fn preflight_tool_permission(&mut self, name: &str) -> Result<bool> {
        if let Ok(policy_manager) = self.policy_manager_mut() {
            let allowed = policy_manager.should_execute_tool(name)?;
            if allowed {
                self.preapproved_tools.insert(name.to_string());
            }
            return Ok(allowed);
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use tempfile::TempDir;

    const CUSTOM_TOOL_NAME: &str = "custom_test_tool";

    struct CustomEchoTool;

    #[async_trait]
    impl Tool for CustomEchoTool {
        async fn execute(&self, args: Value) -> Result<Value> {
            Ok(json!({
                "success": true,
                "args": args,
            }))
        }

        fn name(&self) -> &'static str {
            CUSTOM_TOOL_NAME
        }

        fn description(&self) -> &'static str {
            "Custom echo tool for testing"
        }
    }

    #[tokio::test]
    async fn registers_builtin_tools() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let registry = ToolRegistry::new(temp_dir.path().to_path_buf());
        let available = registry.available_tools();

        assert!(available.contains(&tools::READ_FILE.to_string()));
        assert!(available.contains(&tools::RUN_TERMINAL_CMD.to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn allows_registering_custom_tools() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        registry.register_tool(ToolRegistration::from_tool_instance(
            CUSTOM_TOOL_NAME,
            CapabilityLevel::CodeSearch,
            CustomEchoTool,
        ))?;

        registry.sync_policy_available_tools();

        registry.allow_all_tools().ok();

        let available = registry.available_tools();
        assert!(available.contains(&CUSTOM_TOOL_NAME.to_string()));

        let response = registry
            .execute_tool(CUSTOM_TOOL_NAME, json!({"input": "value"}))
            .await?;
        assert!(response["success"].as_bool().unwrap_or(false));
        Ok(())
    }
}
