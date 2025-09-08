//! Multi-agent conversation loop with orchestrator-driven execution

use anyhow::{Result, anyhow};
use console::style;
use serde_json::json;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use vtagent_core::agent::{
    ContextStoreConfig, DelegationStrategy, ExecutionMode, MultiAgentConfig, MultiAgentTools,
    OrchestratorAgent, VerificationStrategy, execute_multi_agent_tool,
    get_multi_agent_function_declarations,
};
use vtagent_core::config::{
    ConfigManager, ContextStoreDefaults, MultiAgentDefaults, MultiAgentSystemConfig, VTAgentConfig,
};
use vtagent_core::gemini::{
    Content, FunctionDeclaration, FunctionResponse, GenerateContentRequest, Part, Tool, ToolConfig,
};
use vtagent_core::llm::make_client;
use vtagent_core::models::{ModelId, Provider};
use vtagent_core::types::{AgentConfig as CoreAgentConfig, *};

/// Convert MultiAgentSystemConfig to MultiAgentConfig
fn convert_multi_agent_config(system_config: &MultiAgentSystemConfig) -> MultiAgentConfig {
    let execution_mode = match system_config.execution_mode.as_str() {
        "single" => ExecutionMode::Single,
        "multi" => ExecutionMode::Multi,
        "auto" => ExecutionMode::Auto,
        _ => ExecutionMode::Auto, // Default fallback
    };

    // Parse provider from string
    let provider = system_config
        .provider
        .parse::<Provider>()
        .unwrap_or(Provider::Gemini); // Default to Gemini if parsing fails

    // Use provider-specific models if not explicitly configured
    let orchestrator_model = if system_config.orchestrator_model.is_empty() {
        ModelId::default_orchestrator_for_provider(provider.clone())
            .as_str()
            .to_string()
    } else {
        system_config.orchestrator_model.clone()
    };

    let subagent_model = if system_config.subagent_model.is_empty() {
        ModelId::default_subagent_for_provider(provider.clone())
            .as_str()
            .to_string()
    } else {
        system_config.subagent_model.clone()
    };

    MultiAgentConfig {
        enable_multi_agent: MultiAgentDefaults::ENABLE_MULTI_AGENT,
        execution_mode,
        provider,
        orchestrator_model,
        subagent_model,
        max_concurrent_subagents: system_config.max_concurrent_subagents,
        context_store_enabled: system_config.context_store_enabled,
        enable_task_management: MultiAgentDefaults::ENABLE_TASK_MANAGEMENT,
        enable_context_sharing: MultiAgentDefaults::ENABLE_CONTEXT_SHARING,
        enable_performance_monitoring: MultiAgentDefaults::ENABLE_PERFORMANCE_MONITORING,
        debug_mode: system_config.debug_mode,
        task_timeout: MultiAgentDefaults::task_timeout(),
        context_window_size: MultiAgentDefaults::CONTEXT_WINDOW_SIZE,
        max_context_items: MultiAgentDefaults::MAX_CONTEXT_ITEMS,
        verification_strategy: VerificationStrategy::Always,
        delegation_strategy: DelegationStrategy::Adaptive,
        context_store: ContextStoreConfig {
            max_contexts: ContextStoreDefaults::MAX_CONTEXTS,
            auto_cleanup_days: ContextStoreDefaults::AUTO_CLEANUP_DAYS,
            enable_persistence: ContextStoreDefaults::ENABLE_PERSISTENCE,
            compression_enabled: ContextStoreDefaults::COMPRESSION_ENABLED,
            storage_dir: ContextStoreDefaults::STORAGE_DIR.to_string(),
        },
    }
}

/// Run the multi-agent conversation loop
pub async fn run_multi_agent_conversation(
    config: &CoreAgentConfig,
    vtcode_config: &VTAgentConfig,
) -> Result<()> {
    println!("{}", style("Multi-Agent System Active").cyan().bold());
    println!(
        "{}",
        style("   Orchestrator will coordinate specialized agents").dim()
    );

    // Create orchestrator agent
    let session_id = format!(
        "session_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    let orchestrator_model_id = vtcode_config
        .multi_agent
        .orchestrator_model
        .parse::<ModelId>()
        .map_err(|_| {
            anyhow!(
                "Invalid orchestrator model: {}",
                vtcode_config.multi_agent.orchestrator_model
            )
        })?;
    let orchestrator_client = make_client(config.api_key.clone(), orchestrator_model_id);

    let multi_agent_config = convert_multi_agent_config(&vtcode_config.multi_agent);
    let debug_mode = multi_agent_config.debug_mode;

    // Enable debug logging if debug mode is enabled
    if debug_mode {
        println!(
            "{}",
            style("[DEBUG] Multi-agent debug mode enabled").cyan().dim()
        );
        println!(
            "{}",
            style(format!("[DEBUG] Session ID: {}", session_id))
                .cyan()
                .dim()
        );
        println!(
            "{}",
            style(format!("[DEBUG] Provider: {}", multi_agent_config.provider))
                .cyan()
                .dim()
        );
        println!(
            "{}",
            style(format!(
                "[DEBUG] Orchestrator model: {}",
                multi_agent_config.orchestrator_model
            ))
            .cyan()
            .dim()
        );
        println!(
            "{}",
            style(format!(
                "[DEBUG] Subagent model: {}",
                multi_agent_config.subagent_model
            ))
            .cyan()
            .dim()
        );
        println!(
            "{}",
            style(format!(
                "[DEBUG] Max concurrent subagents: {}",
                multi_agent_config.max_concurrent_subagents
            ))
            .cyan()
            .dim()
        );
    }
    let orchestrator = Arc::new(Mutex::new(OrchestratorAgent::new(
        multi_agent_config,
        orchestrator_client,
        session_id.clone(),
        config.api_key.clone(),
        config.workspace.clone(),
    )));

    // Create multi-agent tools
    let multi_agent_tools = MultiAgentTools::new(orchestrator.clone());

    // Get orchestrator system prompt
    let orchestrator_prompt = load_orchestrator_system_prompt(&config.workspace)?;

    // Conversation history for orchestrator
    let mut conversation: Vec<Content> = vec![];

    // Safety limits
    let mut total_turns = 0;
    let max_conversation_turns = vtcode_config.agent.max_conversation_turns;
    let session_start = std::time::Instant::now();
    let max_session_duration = vtcode_config.session_duration();

    // Get multi-agent tools for orchestrator
    let tools = get_multi_agent_function_declarations();
    let function_declarations: Vec<FunctionDeclaration> = tools
        .into_iter()
        .filter_map(|tool| serde_json::from_value(tool).ok())
        .collect();
    let tool_declarations = vec![Tool {
        function_declarations,
    }];

    loop {
        // Safety checks
        total_turns += 1;
        if total_turns >= max_conversation_turns {
            println!(
                "{}",
                style("Maximum conversation turns reached. Session ending for safety.")
                    .red()
                    .bold()
            );
            break;
        }

        if session_start.elapsed() >= max_session_duration {
            println!(
                "{}",
                style("Maximum session duration reached. Session ending for safety.")
                    .red()
                    .bold()
            );
            break;
        }

        // Get user input
        print!("{} ", style("You:").green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            println!("{}", style("Goodbye!").yellow());
            break;
        }

        // Add user message to conversation
        conversation.push(Content::user_text(input));

        if debug_mode {
            println!(
                "{}",
                style(format!("[DEBUG] User input: '{}'", input))
                    .cyan()
                    .dim()
            );
            println!(
                "{}",
                style(format!(
                    "[DEBUG] Conversation length: {} messages",
                    conversation.len()
                ))
                .cyan()
                .dim()
            );
        }

        // Create system instruction for orchestrator
        let system_instruction = Content::system_text(format!(
            "{}\n\n## Current Session\nSession ID: {}\nUser Request: {}",
            orchestrator_prompt, session_id, input
        ));

        // Orchestrator execution loop
        let mut orchestrator_turns = 0;
        let max_orchestrator_turns = 20; // Prevent infinite orchestrator loops

        'orchestrator_loop: loop {
            orchestrator_turns += 1;
            if orchestrator_turns > max_orchestrator_turns {
                println!("{}", style("Orchestrator turn limit reached").yellow());
                break 'orchestrator_loop;
            }

            let request = GenerateContentRequest {
                contents: conversation.clone(),
                tools: Some(tool_declarations.clone()),
                tool_config: Some(ToolConfig::auto()),
                system_instruction: Some(system_instruction.clone()),
                generation_config: None,
            };

            println!("{} ", style("Orchestrator:").blue().bold());

            let response_json = {
                let mut orchestrator_guard = orchestrator.lock().unwrap();
                orchestrator_guard.execute_orchestrator(&request).await
            }
            .map_err(|e| anyhow!("Orchestrator execution failed: {}", e))?;

            // Parse response back to GenerateContentResponse
            let response: vtagent_core::gemini::GenerateContentResponse =
                serde_json::from_value(response_json)?;

            // Debug logging
            if debug_mode {
                println!(
                    "{}",
                    style(format!(
                        "[DEBUG] Orchestrator response candidates: {}",
                        response.candidates.len()
                    ))
                    .cyan()
                    .dim()
                );
                if let Some(candidate) = response.candidates.first() {
                    if let Some(finish_reason) = &candidate.finish_reason {
                        println!(
                            "{}",
                            style(format!("[DEBUG] Finish reason: {}", finish_reason))
                                .cyan()
                                .dim()
                        );
                    }
                }
            }

            if let Some(candidate) = response.candidates.first() {
                // Check for malformed function call error
                if let Some(finish_reason) = &candidate.finish_reason {
                    if finish_reason == "MALFORMED_FUNCTION_CALL" {
                        println!("{}", style("(malformed function call in orchestrator - retrying with simpler approach)").dim().yellow());

                        // Add a recovery message to help the orchestrator
                        conversation.push(Content::user_text(
                            "Your previous function call was malformed. Please try again with a simpler approach, ensuring proper JSON format for function arguments."
                        ));

                        if debug_mode {
                            println!(
                                "{}",
                                style("[DEBUG] Added recovery message for malformed function call")
                                    .cyan()
                                    .dim()
                            );
                        }

                        continue 'orchestrator_loop;
                    }
                }

                let mut had_tool_call = false;
                let mut had_text_response = false;

                for part in &candidate.content.parts {
                    match part {
                        Part::Text { text } => {
                            if !text.trim().is_empty() {
                                println!("{}", text);
                                had_text_response = true;
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
                                "{} Orchestrator calling: {} {}",
                                style("[TOOL]").magenta().bold(),
                                tool_name,
                                args
                            );

                            if debug_mode {
                                println!(
                                    "{}",
                                    style(format!(
                                        "[DEBUG] Executing tool '{}' with args: {}",
                                        tool_name, args
                                    ))
                                    .cyan()
                                    .dim()
                                );
                            }

                            // Execute multi-agent tool
                            let tool_result =
                                execute_multi_agent_tool(tool_name, args, &multi_agent_tools)
                                    .await
                                    .unwrap_or_else(|e| {
                                        if debug_mode {
                                            println!(
                                                "{}",
                                                style(format!(
                                                    "[DEBUG] Tool execution error: {}",
                                                    e
                                                ))
                                                .red()
                                                .dim()
                                            );
                                        }
                                        json!({
                                            "ok": false,
                                            "error": e.to_string()
                                        })
                                    });

                            if debug_mode {
                                println!(
                                    "{}",
                                    style(format!(
                                        "[DEBUG] Tool result: {}",
                                        serde_json::to_string_pretty(&tool_result)
                                            .unwrap_or_else(|_| "Failed to serialize".to_string())
                                    ))
                                    .cyan()
                                    .dim()
                                );
                            }

                            conversation.push(Content::user_parts(vec![Part::FunctionResponse {
                                function_response: FunctionResponse {
                                    name: tool_name.clone(),
                                    response: tool_result,
                                },
                            }]));
                        }
                        Part::FunctionResponse { .. } => {
                            // Should not happen in orchestrator response
                        }
                    }
                }

                if had_tool_call {
                    // Continue orchestrator loop for tool responses
                    if debug_mode {
                        println!(
                            "{}",
                            style("[DEBUG] Tool call detected, continuing orchestrator loop")
                                .cyan()
                                .dim()
                        );
                    }
                    continue 'orchestrator_loop;
                } else if had_text_response {
                    // Orchestrator provided final response, end turn
                    if debug_mode {
                        println!(
                            "{}",
                            style("[DEBUG] Text response received, ending orchestrator turn")
                                .cyan()
                                .dim()
                        );
                    }
                    break 'orchestrator_loop;
                } else {
                    // No content, something went wrong
                    println!("{}", style("Orchestrator provided no response").yellow());
                    if debug_mode {
                        println!(
                            "{}",
                            style("[DEBUG] No content in orchestrator response")
                                .red()
                                .dim()
                        );
                    }
                    break 'orchestrator_loop;
                }
            } else {
                println!("{}", style("No response from orchestrator").yellow());
                if debug_mode {
                    println!(
                        "{}",
                        style("[DEBUG] No candidates in orchestrator response")
                            .red()
                            .dim()
                    );
                }
                break 'orchestrator_loop;
            }
        }

        println!(); // Add spacing between turns
    }

    Ok(())
}

/// Load orchestrator system prompt
fn load_orchestrator_system_prompt(workspace: &PathBuf) -> Result<String> {
    let prompt_path = workspace.join("prompts/orchestrator_system.md");

    if prompt_path.exists() {
        std::fs::read_to_string(&prompt_path)
            .map_err(|e| anyhow!("Failed to read orchestrator prompt: {}", e))
    } else {
        Ok("You are a Lead Architect Agent. You solve terminal-based tasks by strategically delegating work to specialised subagents while maintaining a comprehensive understanding of the system.

Your role is to:
• Build and maintain a clear mental map of the environment relevant to solving the task
• Make architectural decisions about information flow and context distribution
• Coordinate high-level, general-purpose subagents through strategic task delegation
• Shape what information subagents include in their returned reports through well-crafted task descriptions
• Leverage accumulated context to guide increasingly sophisticated actions
• Ensure task completion through verification
• Maintain time-conscious orchestration by providing precise, tightly-scoped tasks with complete context

All terminal operations and file manipulations flow through your subagents - you orchestrate while they execute.".to_string())
    }
}
