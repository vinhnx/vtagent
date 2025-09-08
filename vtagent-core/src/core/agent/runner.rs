//! Agent runner for executing individual agent instances

use crate::core::agent::multi_agent::*;
use crate::gemini::{Content, FunctionResponse, GenerateContentRequest, Part, Tool, ToolConfig};
use crate::llm::{AnyClient, make_client};
use crate::config::models::ModelId;
use crate::tools::{ToolRegistry, build_function_declarations};
use anyhow::{Result, anyhow};
use console::style;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

/// Individual agent runner for executing specialized agent tasks
pub struct AgentRunner {
    /// Agent type and configuration
    agent_type: AgentType,
    /// LLM client for this agent
    client: AnyClient,
    /// Tool registry with restricted access
    tool_registry: Arc<ToolRegistry>,
    /// System prompt content
    system_prompt: String,
    /// Session information
    session_id: String,
    /// Workspace path
    workspace: PathBuf,
}

impl AgentRunner {
    /// Create a new agent runner
    pub fn new(
        agent_type: AgentType,
        model: String,
        api_key: String,
        workspace: PathBuf,
        session_id: String,
    ) -> Result<Self> {
        let model_id = model
            .parse::<ModelId>()
            .map_err(|_| anyhow::anyhow!("Invalid model: {}", model))?;
        let client = make_client(api_key, model_id);
        let tool_registry = Arc::new(ToolRegistry::new(workspace.clone()));

        // Load system prompt for this agent type
        let system_prompt = Self::load_system_prompt(agent_type, &workspace)?;

        Ok(Self {
            agent_type,
            client,
            tool_registry,
            system_prompt,
            session_id,
            workspace,
        })
    }

    /// Load system prompt for the agent type
    fn load_system_prompt(agent_type: AgentType, workspace: &PathBuf) -> Result<String> {
        let prompt_path = workspace.join(agent_type.system_prompt_path());

        if prompt_path.exists() {
            std::fs::read_to_string(&prompt_path).map_err(|e| {
                anyhow!(
                    "Failed to read system prompt from {}: {}",
                    prompt_path.display(),
                    e
                )
            })
        } else {
            // Fall back to default prompts
            Ok(match agent_type {
                AgentType::Explorer => "You are an Explorer Agent, a specialized investigative agent designed to understand, verify, and report on system states and behaviors. You operate as a read-only agent with deep exploratory capabilities.".to_string(),
                AgentType::Coder => "You are a Coder Agent, a state-of-the-art AI software engineer with extraordinary expertise spanning the entire technology landscape. You operate as a write-capable implementation specialist.".to_string(),
                AgentType::Orchestrator => "You are a Lead Architect Agent. You solve terminal-based tasks by strategically delegating work to specialised subagents while maintaining a comprehensive understanding of the system.".to_string(),
                AgentType::Single => "You are a helpful coding assistant.".to_string(),
            })
        }
    }

    /// Execute a task with this agent
    pub async fn execute_task(
        &mut self,
        task: &Task,
        contexts: &[ContextItem],
    ) -> Result<TaskResults> {
        println!(
            "{} Executing {} task: {}",
            style("[AGENT]").magenta().bold(),
            self.agent_type,
            task.title
        );

        // Prepare conversation with task context
        let mut conversation = Vec::new();

        // Add system instruction
        let system_content = self.build_system_instruction(task, contexts)?;
        let system_instruction = Content::system_text(system_content);

        // Add the task as user message
        conversation.push(Content::user_text(&task.description));

        // Get filtered tools for this agent type
        let tools = self.get_filtered_tools();

        let mut created_contexts = Vec::new();
        let modified_files = Vec::new();
        let executed_commands = Vec::new();
        let mut warnings = Vec::new();
        let mut has_completed = false;

        // Agent execution loop (max 10 turns to prevent infinite loops)
        for turn in 0..10 {
            if has_completed {
                break;
            }

            let request = GenerateContentRequest {
                contents: conversation.clone(),
                tools: Some(tools.clone()),
                tool_config: Some(ToolConfig::auto()),
                system_instruction: Some(system_instruction.clone()),
                generation_config: None,
            };

            let response = self.client.generate(&request).await.map_err(|e| {
                anyhow!(
                    "Agent {} execution failed at turn {}: {}",
                    self.agent_type,
                    turn,
                    e
                )
            })?;

            if let Some(candidate) = response.candidates.first() {
                let mut had_tool_call = false;

                for part in &candidate.content.parts {
                    match part {
                        Part::Text { text } => {
                            if !text.trim().is_empty() {
                                println!(
                                    "{} [{}]: {}",
                                    style("[RESPONSE]").cyan().bold(),
                                    self.agent_type,
                                    text.trim()
                                );
                                conversation.push(Content {
                                    role: "model".to_string(),
                                    parts: vec![Part::Text { text: text.clone() }],
                                });
                            }
                        }
                        Part::FunctionCall { function_call } => {
                            had_tool_call = true;
                            let tool_name = &function_call.name;
                            let args = function_call.args.clone();

                            println!(
                                "{} [{}] Calling tool: {} {}",
                                style("[TOOL]").yellow().bold(),
                                self.agent_type,
                                tool_name,
                                args
                            );

                            // Check if tool is allowed for this agent
                            if !self.is_tool_allowed(tool_name) {
                                let denied = json!({
                                    "ok": false,
                                    "error": "access_denied",
                                    "message": format!("Agent {} is not allowed to use tool {}", self.agent_type, tool_name)
                                });
                                conversation.push(Content::user_parts(vec![
                                    Part::FunctionResponse {
                                        function_response: FunctionResponse {
                                            name: tool_name.clone(),
                                            response: denied,
                                        },
                                    },
                                ]));
                                continue;
                            }

                            // Handle special agent-specific tools
                            let tool_result = match tool_name.as_str() {
                                "report" => {
                                    // Agent is completing the task
                                    has_completed = true;
                                    self.handle_report_tool(&args, &mut created_contexts)
                                        .await?
                                }
                                _ => {
                                    // Try to execute the tool through the tool registry
                                    match self.execute_tool(&tool_name, &args).await {
                                        Ok(result) => result,
                                        Err(e) => {
                                            // Log the error and return a proper error response
                                            eprintln!(
                                                "Error executing tool '{}': {}",
                                                tool_name, e
                                            );
                                            json!({
                                                "error": format!("Failed to execute tool '{}': {}", tool_name, e),
                                                "tool": tool_name
                                            })
                                        }
                                    }
                                }
                            };

                            conversation.push(Content::user_parts(vec![Part::FunctionResponse {
                                function_response: FunctionResponse {
                                    name: tool_name.clone(),
                                    response: tool_result,
                                },
                            }]));
                        }
                        Part::FunctionResponse { .. } => {
                            // Should not happen in agent response
                            warnings
                                .push("Unexpected function response in agent output".to_string());
                        }
                    }
                }

                if !had_tool_call && !has_completed {
                    // Agent provided only text response without completing - this indicates completion
                    has_completed = true;
                }
            } else {
                return Err(anyhow!(
                    "No response candidate from agent {}",
                    self.agent_type
                ));
            }
        }

        let summary = if has_completed {
            format!(
                "{} task '{}' completed successfully",
                self.agent_type, task.title
            )
        } else {
            format!(
                "{} task '{}' reached turn limit",
                self.agent_type, task.title
            )
        };

        Ok(TaskResults {
            created_contexts,
            modified_files,
            executed_commands,
            summary,
            warnings,
        })
    }

    /// Build system instruction with task context and provided contexts
    fn build_system_instruction(&self, task: &Task, contexts: &[ContextItem]) -> Result<String> {
        let mut instruction = self.system_prompt.clone();

        // Add task-specific context
        instruction.push_str("\n\n## Current Task\n");
        instruction.push_str(&format!("**Title**: {}\n", task.title));
        instruction.push_str(&format!("**Description**: {}\n", task.description));

        // Add provided contexts
        if !contexts.is_empty() {
            instruction.push_str("\n## Provided Contexts\n");
            for context in contexts {
                instruction.push_str(&format!("### {}\n", context.id));
                instruction.push_str(&context.content);
                instruction.push_str("\n\n");
            }
        }

        // Add context bootstrap files
        if !task.context_bootstrap.is_empty() {
            instruction.push_str("\n## Context Bootstrap\n");
            for bootstrap in &task.context_bootstrap {
                instruction.push_str(&format!("**{}**: {}\n", bootstrap.path, bootstrap.reason));

                // Try to read the file content
                let path = self.workspace.join(&bootstrap.path);
                if path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        instruction.push_str(&format!("```\n{}\n```\n\n", content));
                    }
                }
            }
        }

        Ok(instruction)
    }

    /// Get tools filtered for this agent type
    fn get_filtered_tools(&self) -> Vec<Tool> {
        let all_tools = build_function_declarations();
        let allowed_tools = self.agent_type.allowed_tools();
        let restricted_tools = self.agent_type.restricted_tools();

        // Add agent-specific tools
        let mut filtered_tools = Vec::new();

        // Add the report tool for all agents
        filtered_tools.push(Tool {
            function_declarations: vec![crate::gemini::FunctionDeclaration {
                name: "report".to_string(),
                description: "Submit final report with contexts and completion status".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "contexts": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": {"type": "string", "description": "Context ID (snake_case)"},
                                    "content": {"type": "string", "description": "Context content"}
                                },
                                "required": ["id", "content"]
                            },
                            "description": "List of contexts to report"
                        },
                        "comments": {
                            "type": "string",
                            "description": "Comments about task completion"
                        }
                    },
                    "required": ["contexts", "comments"]
                }),
            }]
        });

        // Filter regular tools
        for decl in all_tools {
            let name = &decl.name;
            // Check if tool is explicitly allowed
            let is_allowed = allowed_tools.contains(&"*") || allowed_tools.contains(&name.as_str());

            // Check if tool is explicitly restricted
            let is_restricted = restricted_tools.contains(&name.as_str());

            if is_allowed && !is_restricted {
                filtered_tools.push(Tool {
                    function_declarations: vec![decl],
                });
            }
        }
        filtered_tools
    }

    /// Check if a tool is allowed for this agent type
    fn is_tool_allowed(&self, tool_name: &str) -> bool {
        let allowed_tools = self.agent_type.allowed_tools();
        let restricted_tools = self.agent_type.restricted_tools();

        let is_allowed = allowed_tools.contains(&"*") || allowed_tools.contains(&tool_name);
        let is_restricted = restricted_tools.contains(&tool_name);

        is_allowed && !is_restricted
    }

    /// Handle the report tool call
    async fn handle_report_tool(
        &self,
        args: &Value,
        created_contexts: &mut Vec<String>,
    ) -> Result<Value> {
        if let Some(contexts) = args.get("contexts").and_then(|c| c.as_array()) {
            for context in contexts {
                if let (Some(id), Some(content)) = (
                    context.get("id").and_then(|i| i.as_str()),
                    context.get("content").and_then(|c| c.as_str()),
                ) {
                    created_contexts.push(id.to_string());

                    // Create a proper context item (for future context store integration)
                    let _context_item = ContextItem {
                        id: id.to_string(),
                        content: content.to_string(),
                        created_by: self.agent_type,
                        session_id: self.session_id.clone(),
                        created_at: std::time::SystemTime::now(),
                        tags: Vec::new(),
                        context_type: match self.agent_type {
                            AgentType::Explorer => ContextType::Analysis,
                            AgentType::Coder => ContextType::Implementation,
                            _ => ContextType::General,
                        },
                        related_files: Vec::new(),
                    };

                    println!(
                        "{} [{}] Created context: {} - {}",
                        style("[CONTEXT]").green().bold(),
                        self.agent_type,
                        id,
                        content.chars().take(100).collect::<String>()
                    );
                }
            }
        }

        let comments = args
            .get("comments")
            .and_then(|c| c.as_str())
            .unwrap_or("Task completed");

        println!(
            "{} [{}] Task completed: {}",
            style("[REPORT]").green().bold(),
            self.agent_type,
            comments
        );

        Ok(json!({
            "ok": true,
            "message": "Report submitted successfully",
        }))
    }

    /// Execute a tool by name with given arguments
    async fn execute_tool(&self, tool_name: &str, args: &Value) -> Result<Value> {
        use crate::tools::registry::ToolRegistry;

        // Create a tool registry and try to execute the tool
        let mut registry = ToolRegistry::new(self.workspace.clone());

        // Register default tools
        registry.register_default_tools()?;

        // Try to execute the tool
        match registry.execute_tool(tool_name, args).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // If the tool doesn't exist in the registry, return an error
                Err(anyhow!(
                    "Tool '{}' not found or failed to execute: {}",
                    tool_name,
                    e
                ))
            }
        }
    }
}
