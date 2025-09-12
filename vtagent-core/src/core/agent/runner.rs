//! Agent runner for executing individual agent instances

use crate::config::models::ModelId;
use crate::core::agent::multi_agent::*;
use crate::gemini::{Content, GenerateContentRequest, Part, Tool, ToolConfig};
use crate::llm::{AnyClient, create_provider_with_config, make_client};
use crate::tools::{ToolRegistry, build_function_declarations};
use anyhow::{Result, anyhow};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

/// Wrapper for LMStudio provider to implement LLMClient trait
struct LMStudioClientWrapper {
    provider: Box<dyn crate::llm::provider::LLMProvider>,
    model: String,
}

#[async_trait::async_trait]
impl crate::llm::client::LLMClient for LMStudioClientWrapper {
    async fn generate(
        &mut self,
        prompt: &str,
    ) -> Result<crate::llm::types::LLMResponse, crate::llm::provider::LLMError> {
        // Parse the prompt as a GenerateContentRequest if it's a serialized request
        let request: crate::gemini::GenerateContentRequest = match serde_json::from_str(prompt) {
            Ok(req) => req,
            Err(_) => {
                // If parsing fails, treat it as a simple text prompt
                crate::gemini::GenerateContentRequest {
                    contents: vec![crate::gemini::Content::user_text(prompt.to_string())],
                    tools: None,
                    tool_config: None,
                    system_instruction: None,
                    generation_config: None,
                }
            }
        };

        // Convert Gemini format to LLM provider format
        let messages: Vec<crate::llm::provider::Message> = request
            .contents
            .into_iter()
            .map(|content| {
                let role = match content.role.as_str() {
                    crate::config::constants::message_roles::USER => crate::llm::provider::MessageRole::User,
                    "model" => crate::llm::provider::MessageRole::Assistant,
                    crate::config::constants::message_roles::SYSTEM => crate::llm::provider::MessageRole::System,
                    _ => crate::llm::provider::MessageRole::User,
                };

                // Extract text content from parts
                let content_text = content
                    .parts
                    .into_iter()
                    .filter_map(|part| match part {
                        crate::gemini::Part::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                crate::llm::provider::Message {
                    role,
                    content: content_text,
                    tool_calls: None,
                    tool_call_id: None,
                }
            })
            .collect();

        // Create LLM request
        let llm_request = crate::llm::provider::LLMRequest {
            messages,
            system_prompt: request.system_instruction.as_ref().map(|si| {
                si.parts
                    .iter()
                    .filter_map(|part| match part {
                        crate::gemini::Part::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
            tools: request.tools.as_ref().map(|gemini_tools| {
                gemini_tools
                    .iter()
                    .flat_map(|tool| &tool.function_declarations)
                    .map(|decl| crate::llm::provider::ToolDefinition {
                        name: decl.name.clone(),
                        description: decl.description.clone(),
                        parameters: decl.parameters.clone(),
                    })
                    .collect::<Vec<_>>()
            }),
            model: self.model.clone(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: false,
        };

        // Get response from provider and convert it to the right type
        let provider_response = self.provider.generate(llm_request).await?;

        Ok(crate::llm::types::LLMResponse {
            content: provider_response.content.unwrap_or_default(),
            model: self.model.clone(),
            usage: provider_response.usage.map(|u| crate::llm::types::Usage {
                prompt_tokens: u.prompt_tokens as usize,
                completion_tokens: u.completion_tokens as usize,
                total_tokens: u.total_tokens as usize,
            }),
        })
    }

    fn backend_kind(&self) -> crate::llm::types::BackendKind {
        crate::llm::types::BackendKind::OpenAI
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

/// Individual agent runner for executing specialized agent tasks
pub struct AgentRunner {
    /// Agent type and configuration
    agent_type: AgentType,
    /// LLM client for this agent
    client: AnyClient,
    /// Tool registry with restricted access
    tool_registry: ToolRegistry,
    /// System prompt content
    system_prompt: String,
    /// Session information
    session_id: String,
    /// Workspace path
    workspace: PathBuf,
}

impl AgentRunner {
    /// Create informative progress message based on operation type
    fn create_progress_message(&self, operation: &str, details: Option<&str>) -> String {
        match operation {
            "thinking" => "Analyzing request and planning approach...".to_string(),
            "processing" => format!("Processing turn with {} model", self.client.model_id()),
            "tool_call" => {
                if let Some(tool) = details {
                    format!("Executing {} tool for task completion", tool)
                } else {
                    "Executing tool to gather information".to_string()
                }
            },
            "file_read" => {
                if let Some(file) = details {
                    format!("Reading {} to understand structure", file)
                } else {
                    "Reading file to analyze content".to_string()
                }
            },
            "file_write" => {
                if let Some(file) = details {
                    format!("Writing changes to {}", file)
                } else {
                    "Writing file with requested changes".to_string()
                }
            },
            "search" => {
                if let Some(pattern) = details {
                    format!("Searching codebase for '{}'", pattern)
                } else {
                    "Searching codebase for relevant information".to_string()
                }
            },
            "terminal" => {
                if let Some(cmd) = details {
                    format!("Running terminal command: {}", cmd.split(' ').next().unwrap_or(cmd))
                } else {
                    "Executing terminal command".to_string()
                }
            },
            "completed" => "Task completed successfully!".to_string(),
            "error" => {
                if let Some(err) = details {
                    format!("Error encountered: {}", err)
                } else {
                    "An error occurred during execution".to_string()
                }
            },
            _ => format!("{}...", operation)
        }
    }

    /// Create a new agent runner
    pub fn new(
        agent_type: AgentType,
        model: ModelId,
        api_key: String,
        workspace: PathBuf,
        session_id: String,
    ) -> Result<Self> {
        // Create client based on model - if it's an LMStudio model, create the provider directly
        let client: AnyClient =
            if model.as_str().contains("lmstudio") || model.as_str().contains("qwen") {
                // For LMStudio models, we create the provider directly
                let provider = create_provider_with_config(
                    "lmstudio",
                    Some(api_key.clone()),
                    Some("http://localhost:1234/v1".to_string()),
                    Some(model.as_str().to_string()),
                )
                .map_err(|e| anyhow::anyhow!("Failed to create LMStudio provider: {}", e))?;
                // Wrap the provider in a client that implements the LLMClient trait
                Box::new(LMStudioClientWrapper {
                    provider,
                    model: model.as_str().to_string(),
                })
            } else {
                // For other models, use the standard approach
                make_client(api_key, model)
            };

        // Create system prompt based on agent type
        let system_prompt = match agent_type {
            AgentType::Coder => include_str!("../../../../prompts/coder_system.md").to_string(),
            AgentType::Explorer => {
                include_str!("../../../../prompts/explorer_system.md").to_string()
            }
            AgentType::Orchestrator => {
                include_str!("../../../../prompts/orchestrator_system.md").to_string()
            }
            AgentType::Single => include_str!("../../../../prompts/system.md").to_string(),
        };

        Ok(Self {
            agent_type,
            client,
            tool_registry: ToolRegistry::new(workspace.clone()),
            system_prompt,
            session_id,
            workspace,
        })
    }

    /// Execute a task with this agent
    pub async fn execute_task(
        &mut self,
        task: &Task,
        contexts: &[ContextItem],
    ) -> Result<TaskResults> {
        // Create a progress bar for agent execution
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {prefix:.bold.dim} {msg}")
                .unwrap(),
        );
        pb.set_prefix(format!("[{}]", self.agent_type));
        pb.set_message(self.create_progress_message("thinking", None));
        pb.enable_steady_tick(Duration::from_millis(100));

        println!(
            "{} Executing {} task: {}",
            style("[AGENT]").blue().bold().on_black(),
            self.agent_type,
            task.title
        );

        // Prepare conversation with task context
        let mut conversation = Vec::new();

        // Add system instruction as the first message
        let system_content = self.build_system_instruction(task, contexts)?;
        conversation.push(Content::user_text(system_content));

        // Add task description
        conversation.push(Content::user_text(format!(
            "Task: {}\nDescription: {}",
            task.title, task.description
        )));

        // Add context items if any
        if !contexts.is_empty() {
            let context_content: Vec<String> = contexts
                .iter()
                .map(|ctx| format!("Context [{}]: {}", ctx.id, ctx.content))
                .collect();
            conversation.push(Content::user_text(format!(
                "Relevant Context:\n{}",
                context_content.join("\n")
            )));
        }

        // Build available tools for this agent
        let tools = self.build_agent_tools()?;

        // Track execution results
        let created_contexts = Vec::new();
        let mut modified_files = Vec::new();
        let mut executed_commands = Vec::new();
        let mut warnings = Vec::new();
        let mut has_completed = false;

        // Agent execution loop (max 10 turns to prevent infinite loops)
        for turn in 0..10 {
            if has_completed {
                break;
            }

            pb.set_message(format!(
                "{} {} is processing turn {}...",
                self.agent_type,
                style("(PROC)").yellow().bold(),
                turn + 1
            ));

            let request = GenerateContentRequest {
                contents: conversation.clone(),
                tools: Some(tools.clone()),
                tool_config: Some(ToolConfig::auto()),
                system_instruction: None,
                generation_config: None,
            };

            let response = self
                .client
                .generate(&serde_json::to_string(&request)?)
                .await
                .map_err(|e| {
                    pb.finish_with_message(format!("{} Failed", style("(ERROR)").red().bold().on_black()));
                    anyhow!(
                        "Agent {} execution failed at turn {}: {}",
                        self.agent_type,
                        turn,
                        e
                    )
                })?;

            // Update progress for successful response
            pb.set_message(format!(
                "{} {} received response, processing...",
                self.agent_type,
                style("(RECV)").green().bold()
            ));

            // Use response content directly
            if !response.content.is_empty() {
                // Try to parse the response as JSON to check for tool calls
                let mut had_tool_call = false;

                // Try to parse as a tool call response
                if let Ok(tool_call_response) = serde_json::from_str::<Value>(&response.content) {
                    // Check for standard tool_calls format
                    if let Some(tool_calls) = tool_call_response
                        .get("tool_calls")
                        .and_then(|tc| tc.as_array())
                    {
                        had_tool_call = true;

                        // Process each tool call
                        for tool_call in tool_calls {
                            if let Some(function) = tool_call.get("function") {
                                if let (Some(name), Some(arguments)) = (
                                    function.get("name").and_then(|n| n.as_str()),
                                    function.get("arguments"),
                                ) {
                                    println!(
                                        "{} [{}] Calling tool: {}",
                                        style("[TOOL_CALL]").blue().bold(),
                                        self.agent_type,
                                        name
                                    );

                                    // Execute the tool
                                    match self.execute_tool(name, &arguments.clone()).await {
                                        Ok(result) => {
                                            pb.set_message(format!(
                                        "{} {} tool executed successfully",
                                        style("(OK)").green(),
                                        name
                                    ));

                                            // Add tool result to conversation
                                            let tool_result = serde_json::to_string(&result)?;
                                            conversation.push(Content {
                                                role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                                parts: vec![Part::Text {
                                                    text: format!(
                                                        "Tool {} result: {}",
                                                        name, tool_result
                                                    ),
                                                }],
                                            });

                                            // Track what the agent did
                                            executed_commands.push(name.to_string());

                                            // Special handling for certain tools
                                            if name == "write_file" {
                                                if let Some(filepath) =
                                                    arguments.get("path").and_then(|p| p.as_str())
                                                {
                                                    modified_files.push(filepath.to_string());
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            pb.set_message(format!(
                                                "{} {} tool failed: {}",
                                                style("(ERR)").red(),
                                                name,
                                                e
                                            ));
                                            warnings.push(format!("Tool {} failed: {}", name, e));
                                            conversation.push(Content {
                                                role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                                parts: vec![Part::Text {
                                                    text: format!("Tool {} failed: {}", name, e),
                                                }],
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Check for Gemini functionCall format
                    else if let Some(function_call) = tool_call_response.get("functionCall") {
                        had_tool_call = true;

                        if let (Some(name), Some(args)) = (
                            function_call.get("name").and_then(|n| n.as_str()),
                            function_call.get("args"),
                        ) {
                            println!(
                                "{} [{}] Calling tool: {}",
                                style("[TOOL_CALL]").blue().bold(),
                                self.agent_type,
                                name
                            );

                            // Execute the tool
                            match self.execute_tool(name, args).await {
                                Ok(result) => {
                                    pb.set_message(format!(
                                "{} {} tool executed successfully",
                                style("(OK)").green(),
                                name
                            ));

                                    // Add tool result to conversation
                                    let tool_result = serde_json::to_string(&result)?;
                                    conversation.push(Content {
                                        role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                        parts: vec![Part::Text {
                                            text: format!(
                                                "Tool {} result: {}",
                                                name, tool_result
                                            ),
                                        }],
                                    });

                                    // Track what the agent did
                                    executed_commands.push(name.to_string());

                                    // Special handling for certain tools
                                    if name == "write_file" {
                                        if let Some(filepath) =
                                            args.get("path").and_then(|p| p.as_str())
                                        {
                                            modified_files.push(filepath.to_string());
                                        }
                                    }
                                }
                                Err(e) => {
                                    pb.set_message(format!(
                                        "{} {} tool failed: {}",
                                        style("(ERR)").red().bold(),
                                        name,
                                        e
                                    ));
                                    warnings.push(format!("Tool {} failed: {}", name, e));
                                    conversation.push(Content {
                                        role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                        parts: vec![Part::Text {
                                            text: format!("Tool {} failed: {}", name, e),
                                        }],
                                    });
                                }
                            }
                        }
                    }
                    // Check for tool_code format (what agents are actually producing)
                    else if let Some(tool_code) = tool_call_response
                        .get("tool_code")
                        .and_then(|tc| tc.as_str())
                    {
                        had_tool_call = true;

                        println!(
                            "{} [{}] Executing tool code: {}",
                            style("[TOOL_EXEC]").cyan().bold().on_black(),
                            self.agent_type,
                            tool_code
                        );

                        // Try to parse the tool_code as a function call
                        // This is a simplified parser for the format: function_name(args)
                        if let Some((func_name, args_str)) = parse_tool_code(tool_code) {
                            println!(
                                "{} [{}] Parsed tool: {} with args: {}",
                                style("[TOOL_PARSE]").yellow().bold().on_black(),
                                self.agent_type,
                                func_name,
                                args_str
                            );

                            // Parse arguments as JSON
                            match serde_json::from_str::<Value>(&args_str) {
                                Ok(arguments) => {
                                    // Execute the tool
                                    match self.execute_tool(&func_name, &arguments).await {
                                        Ok(result) => {
                                            pb.set_message(format!(
                                        "{} {} tool executed successfully",
                                        style("(OK)").green(),
                                        func_name
                                    ));

                                            // Add tool result to conversation
                                            let tool_result = serde_json::to_string(&result)?;
                                            conversation.push(Content {
                                                role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                                parts: vec![Part::Text {
                                                    text: format!(
                                                        "Tool {} result: {}",
                                                        func_name, tool_result
                                                    ),
                                                }],
                                            });

                                            // Track what the agent did
                                            executed_commands.push(func_name.to_string());

                                            // Special handling for certain tools
                                            if func_name == "write_file" {
                                                if let Some(filepath) =
                                                    arguments.get("path").and_then(|p| p.as_str())
                                                {
                                                    modified_files.push(filepath.to_string());
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            pb.set_message(format!(
                                                "{} {} tool failed: {}",
                                                style("(ERROR)").red().bold(),
                                                func_name,
                                                e
                                            ));
                                            warnings
                                                .push(format!("Tool {} failed: {}", func_name, e));
                                            conversation.push(Content {
                                                role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                                parts: vec![Part::Text {
                                                    text: format!(
                                                        "Tool {} failed: {}",
                                                        func_name, e
                                                    ),
                                                }],
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "Failed to parse tool arguments '{}': {}",
                                        args_str, e
                                    );
                                    warnings.push(error_msg.clone());
                                    conversation.push(Content {
                                        role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                        parts: vec![Part::Text { text: error_msg }],
                                    });
                                }
                            }
                        } else {
                            let error_msg = format!("Failed to parse tool code: {}", tool_code);
                            warnings.push(error_msg.clone());
                            conversation.push(Content {
                                role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                parts: vec![Part::Text { text: error_msg }],
                            });
                        }
                    }
                    // Check for tool_name format (alternative format)
                    else if let Some(tool_name) = tool_call_response
                        .get("tool_name")
                        .and_then(|tn| tn.as_str())
                    {
                        had_tool_call = true;

                        println!(
                            "{} [{}] Calling tool: {}",
                            style("[TOOL_CALL]").blue().bold().on_black(),
                            self.agent_type,
                            tool_name
                        );

                        if let Some(parameters) = tool_call_response.get("parameters") {
                            // Execute the tool
                            match self.execute_tool(tool_name, parameters).await {
                                Ok(result) => {
                                    pb.set_message(format!(
                                        "{} {} tool executed successfully",
                                        style("(SUCCESS)").green().bold(),
                                        tool_name
                                    ));

                                    // Add tool result to conversation
                                    let tool_result = serde_json::to_string(&result)?;
                                    conversation.push(Content {
                                        role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                        parts: vec![Part::Text {
                                            text: format!(
                                                "Tool {} result: {}",
                                                tool_name, tool_result
                                            ),
                                        }],
                                    });

                                    // Track what the agent did
                                    executed_commands.push(tool_name.to_string());

                                    // Special handling for certain tools
                                    if tool_name == "write_file" {
                                        if let Some(filepath) =
                                            parameters.get("path").and_then(|p| p.as_str())
                                        {
                                            modified_files.push(filepath.to_string());
                                        }
                                    }
                                }
                                Err(e) => {
                                    pb.set_message(format!(
                                        "{} {} tool failed: {}",
                                        style("(ERROR)").red().bold(),
                                        tool_name,
                                        e
                                    ));
                                    warnings.push(format!("Tool {} failed: {}", tool_name, e));
                                    conversation.push(Content {
                                        role: "user".to_string(), // Gemini API only accepts "user" and "model"
                                        parts: vec![Part::Text {
                                            text: format!("Tool {} failed: {}", tool_name, e),
                                        }],
                                    });
                                }
                            }
                        }
                    } else {
                        // Regular content response
                        println!(
                            "{} [{}]: {}",
                            style("[RESPONSE]").cyan().bold(),
                            self.agent_type,
                            response.content.trim()
                        );
                        conversation.push(Content {
                            role: "model".to_string(),
                            parts: vec![Part::Text {
                                text: response.content.clone(),
                            }],
                        });
                    }
                } else {
                    // Regular text response
                    println!(
                        "{} [{}]: {}",
                        style("[RESPONSE]").cyan().bold(),
                        self.agent_type,
                        response.content.trim()
                    );
                    conversation.push(Content {
                        role: "model".to_string(),
                        parts: vec![Part::Text {
                            text: response.content.clone(),
                        }],
                    });
                }

                // Check for task completion indicators in the response
                if !has_completed {
                    let response_lower = response.content.to_lowercase();

                    // More comprehensive completion detection
                    let completion_indicators = [
                        "task completed", "task done", "finished", "complete", "summary",
                        "i have successfully", "i've completed", "i have finished",
                        "task accomplished", "mission accomplished", "objective achieved",
                        "work is done", "all done", "completed successfully",
                        "task execution complete", "operation finished"
                    ];

                    // Check if any completion indicator is present
                    let is_completed = completion_indicators
                        .iter()
                        .any(|&indicator| response_lower.contains(indicator));

                    // Also check for explicit completion statements
                    let has_explicit_completion = response_lower.contains("the task is complete") ||
                                                 response_lower.contains("task has been completed") ||
                                                 response_lower.contains("i am done") ||
                                                 response_lower.contains("that's all") ||
                                                 response_lower.contains("no more actions needed");

                    if is_completed || has_explicit_completion {
                        has_completed = true;
                        pb.set_message(format!(
                            "{} {} completed task successfully",
                            self.agent_type,
                            style("(SUCCESS)").green().bold()
                        ));
                    }
                }

                // Improved loop termination logic
                // Continue if: we had tool calls, task is not completed, and we haven't exceeded max turns
                let should_continue = had_tool_call || (!has_completed && turn < 9);

                if !should_continue {
                    if has_completed {
                        pb.set_message(format!(
                            "{} {} finished - task completed",
                            self.agent_type,
                            style("(SUCCESS)").green().bold()
                        ));
                    } else if turn >= 9 {
                        pb.set_message(format!(
                            "{} {} finished - maximum turns reached",
                            self.agent_type,
                            style("(TIME)").yellow().bold()
                        ));
                    } else {
                        pb.set_message(format!(
                            "{} {} finished - no more actions needed",
                            self.agent_type,
                            style("(FINISH)").blue().bold()
                        ));
                    }
                    break;
                }
            } else {
                // Empty response - check if we should continue or if task is actually complete
                if has_completed {
                    pb.set_message(format!(
                        "{} {} finished - task was completed earlier",
                        self.agent_type,
                        style("(SUCCESS)").green().bold()
                    ));
                    break;
                } else if turn >= 9 {
                    pb.set_message(format!(
                        "{} {} finished - maximum turns reached with empty response",
                        self.agent_type,
                        style("(TIME)").yellow().bold()
                    ));
                    break;
                } else {
                    // Empty response but task not complete - this might indicate an issue
                    pb.set_message(format!(
                        "{} {} received empty response, continuing...",
                        self.agent_type,
                        style("(EMPTY)").yellow()
                    ));
                    // Don't break here, let the loop continue to give the agent another chance
                }
            }
        }

        // Finish the progress bar
        pb.finish_with_message("Done");

        // Generate meaningful summary based on agent actions
        let summary = self.generate_task_summary(&modified_files, &executed_commands, &warnings, &conversation);

        // Return task results
        Ok(TaskResults {
            created_contexts,
            modified_files,
            executed_commands,
            summary,
            warnings,
        })
    }

    /// Build system instruction for agent based on task and contexts
    fn build_system_instruction(&self, task: &Task, contexts: &[ContextItem]) -> Result<String> {
        let mut instruction = self.system_prompt.clone();

        // Add task-specific information
        instruction.push_str(&format!("\n\nTask: {}\n{}", task.title, task.description));

        // Add context information if any
        if !contexts.is_empty() {
            instruction.push_str("\n\nRelevant Context:");
            for ctx in contexts {
                instruction.push_str(&format!("\n[{}] {}", ctx.id, ctx.content));
            }
        }

        Ok(instruction)
    }

    /// Build available tools for this agent type
    fn build_agent_tools(&self) -> Result<Vec<Tool>> {
        // Build function declarations based on available tools
        let declarations = build_function_declarations();

        // Filter tools based on agent type and permissions
        let allowed_tools: Vec<Tool> = declarations
            .into_iter()
            .filter(|decl| self.is_tool_allowed(&decl.name))
            .map(|decl| Tool {
                function_declarations: vec![decl],
            })
            .collect();

        Ok(allowed_tools)
    }

    /// Check if a tool is allowed for this agent type
    fn is_tool_allowed(&self, tool_name: &str) -> bool {
        // First check agent type restrictions
        let agent_type_allowed = match self.agent_type {
            AgentType::Coder => {
                // Coder agents can use file operations and command execution
                matches!(
                    tool_name,
                    "read_file" | "write_file" | "list_files" | "run_terminal_cmd"
                )
            }
            AgentType::Explorer => {
                // Explorer agents can use search and file listing
                matches!(tool_name, "rp_search" | "list_files")
            }
            AgentType::Orchestrator => {
                // Orchestrator can coordinate but not directly manipulate files
                matches!(tool_name, "rp_search" | "list_files")
            }
            AgentType::Single => {
                // Single agents have limited tool access
                matches!(tool_name, "rp_search" | "list_files")
            }
        };

        // If agent type doesn't allow it, deny access
        if !agent_type_allowed {
            return false;
        }

        // Check tool policy (allow if Allow or Prompt, deny if Deny)
        let policy = self.tool_registry.policy_manager().get_policy(tool_name);
        match policy {
            crate::tool_policy::ToolPolicy::Allow | crate::tool_policy::ToolPolicy::Prompt => true,
            crate::tool_policy::ToolPolicy::Deny => false,
        }
    }

    /// Execute a tool by name with given arguments
    async fn execute_tool(&self, tool_name: &str, args: &Value) -> Result<Value> {
        // Clone the tool registry for this execution
        let mut registry = self.tool_registry.clone();

        // Initialize async components
        registry.initialize_async().await?;

        // Try to execute the tool
        match registry.execute_tool(tool_name, args.clone()).await {
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

    /// Generate a meaningful summary of the task execution
    fn generate_task_summary(
        &self,
        modified_files: &[String],
        executed_commands: &[String],
        warnings: &[String],
        conversation: &[Content],
    ) -> String {
        let mut summary = vec![];

        // Add task title and agent type
        summary.push(format!("Task: {}", conversation.get(0).and_then(|c| c.parts.get(0)).and_then(|p| p.as_text()).unwrap_or(&"".to_string())));
        summary.push(format!("Agent Type: {:?}", self.agent_type));

        // Add executed commands
        if !executed_commands.is_empty() {
            summary.push("Executed Commands:".to_string());
            for command in executed_commands {
                summary.push(format!(" - {}", command));
            }
        }

        // Add modified files
        if !modified_files.is_empty() {
            summary.push("Modified Files:".to_string());
            for file in modified_files {
                summary.push(format!(" - {}", file));
            }
        }

        // Add warnings if any
        if !warnings.is_empty() {
            summary.push("Warnings:".to_string());
            for warning in warnings {
                summary.push(format!(" - {}", warning));
            }
        }

        // Add final status
        let final_status = if conversation.last().map_or(false, |c| c.role == "model" && c.parts.iter().any(|p| p.as_text().map_or(false, |t| t.contains("completed") || t.contains("done") || t.contains("finished")))) {
            "Task completed successfully".to_string()
        } else {
            "Task did not complete as expected".to_string()
        };
        summary.push(final_status);

        // Join all parts with new lines
        summary.join("\n")
    }
}

/// Parse tool code in the format: function_name(arg1=value1, arg2=value2)
fn parse_tool_code(tool_code: &str) -> Option<(String, String)> {
    // Remove any markdown code blocks
    let code = tool_code.trim();
    let code = if code.starts_with("```") && code.ends_with("```") {
        code.trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        code
    };

    // Try to match function call pattern: name(args)
    if let Some(open_paren) = code.find('(') {
        if let Some(close_paren) = code.rfind(')') {
            let func_name = code[..open_paren].trim().to_string();
            let args_str = &code[open_paren + 1..close_paren];

            // Convert Python-style arguments to JSON
            let json_args = convert_python_args_to_json(args_str)?;
            return Some((func_name, json_args));
        }
    }

    None
}

/// Convert Python-style function arguments to JSON
fn convert_python_args_to_json(args_str: &str) -> Option<String> {
    if args_str.trim().is_empty() {
        return Some("{}".to_string());
    }

    let mut json_parts = Vec::new();

    for arg in args_str.split(',').map(|s| s.trim()) {
        if arg.is_empty() {
            continue;
        }

        // Handle key=value format
        if let Some(eq_pos) = arg.find('=') {
            let key = arg[..eq_pos].trim().trim_matches('"').trim_matches('\'');
            let value = arg[eq_pos + 1..].trim();

            // Convert value to JSON format
            let json_value = if value.starts_with('"') && value.ends_with('"') {
                value.to_string()
            } else if value.starts_with('\'') && value.ends_with('\'') {
                format!("\"{}\"", value.trim_matches('\''))
            } else if value == "True" || value == "true" {
                "true".to_string()
            } else if value == "False" || value == "false" {
                "false".to_string()
            } else if value == "None" || value == "null" {
                "null".to_string()
            } else if let Ok(num) = value.parse::<f64>() {
                num.to_string()
            } else {
                // Assume it's a string that needs quotes
                format!("\"{}\"", value)
            };

            json_parts.push(format!("\"{}\": {}", key, json_value));
        } else {
            // Handle positional arguments (not supported well, but try)
            return None;
        }
    }

    Some(format!("{{{}}}", json_parts.join(", ")))
}
