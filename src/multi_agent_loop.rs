//! Multi-agent conversation loop with orchestrator-driven execution

use vtagent_core::agent::{MultiAgentConfig, OrchestratorAgent, get_multi_agent_function_declarations, MultiAgentTools, execute_multi_agent_tool, ExecutionMode, VerificationStrategy, DelegationStrategy, ContextStoreConfig};
use vtagent_core::config::{ConfigManager, VTAgentConfig, MultiAgentSystemConfig, MultiAgentDefaults, ContextStoreDefaults};
use vtagent_core::llm::{make_client};
use vtagent_core::types::{AgentConfig as CoreAgentConfig, *};
use vtagent_core::gemini::{GenerateContentRequest, Content, Part, FunctionResponse, Tool, ToolConfig, FunctionDeclaration};
use anyhow::{Result, anyhow};
use console::style;
use serde_json::json;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Convert MultiAgentSystemConfig to MultiAgentConfig
fn convert_multi_agent_config(system_config: &MultiAgentSystemConfig) -> MultiAgentConfig {
    let execution_mode = match system_config.execution_mode.as_str() {
        "single" => ExecutionMode::Single,
        "multi" => ExecutionMode::Multi,
        "auto" => ExecutionMode::Auto,
        _ => ExecutionMode::Auto, // Default fallback
    };

    MultiAgentConfig {
        enable_multi_agent: MultiAgentDefaults::ENABLE_MULTI_AGENT,
        execution_mode,
        orchestrator_model: system_config.orchestrator_model.clone(),
        subagent_model: system_config.subagent_model.clone(),
        max_concurrent_subagents: system_config.max_concurrent_subagents,
        context_store_enabled: system_config.context_store_enabled,
        enable_task_management: MultiAgentDefaults::ENABLE_TASK_MANAGEMENT,
        enable_context_sharing: MultiAgentDefaults::ENABLE_CONTEXT_SHARING,
        enable_performance_monitoring: MultiAgentDefaults::ENABLE_PERFORMANCE_MONITORING,
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
    println!("{}", style("   Orchestrator will coordinate specialized agents").dim());

    // Create orchestrator agent
    let session_id = format!("session_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    let orchestrator_client = make_client(config.api_key.clone(), vtcode_config.multi_agent.orchestrator_model.clone());

    let multi_agent_config = convert_multi_agent_config(&vtcode_config.multi_agent);
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
            println!("{}", style("Maximum conversation turns reached. Session ending for safety.").red().bold());
            break;
        }

        if session_start.elapsed() >= max_session_duration {
            println!("{}", style("Maximum session duration reached. Session ending for safety.").red().bold());
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

        // Create system instruction for orchestrator
        let system_instruction = Content::system_text(format!(
            "{}\n\n## Current Session\nSession ID: {}\nUser Request: {}",
            orchestrator_prompt,
            session_id,
            input
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
                serde_json::from_value(response_json)?;            if let Some(candidate) = response.candidates.first() {
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

                            println!("{} Orchestrator calling: {} {}",
                                    style("[TOOL]").magenta().bold(),
                                    tool_name,
                                    args);

                            // Execute multi-agent tool
                            let tool_result = execute_multi_agent_tool(tool_name, args, &multi_agent_tools).await
                                .unwrap_or_else(|e| json!({
                                    "ok": false,
                                    "error": e.to_string()
                                }));

                            conversation.push(Content::user_parts(vec![
                                Part::FunctionResponse {
                                    function_response: FunctionResponse {
                                        name: tool_name.clone(),
                                        response: tool_result,
                                    },
                                },
                            ]));
                        }
                        Part::FunctionResponse { .. } => {
                            // Should not happen in orchestrator response
                        }
                    }
                }

                if had_tool_call {
                    // Continue orchestrator loop for tool responses
                    continue 'orchestrator_loop;
                } else if had_text_response {
                    // Orchestrator provided final response, end turn
                    break 'orchestrator_loop;
                } else {
                    // No content, something went wrong
                    println!("{}", style("Orchestrator provided no response").yellow());
                    break 'orchestrator_loop;
                }
            } else {
                println!("{}", style("No response from orchestrator").yellow());
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
