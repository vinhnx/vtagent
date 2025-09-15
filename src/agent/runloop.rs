use anyhow::{Context, Result};
use std::io::{self, Write};
use vtagent_core::config::loader::{ConfigManager, VTAgentConfig};
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::utils::ansi::{AnsiRenderer, MessageStyle};

// Single-agent engine with Decision Ledger support
use vtagent_core::gemini::function_calling::{FunctionCall, FunctionResponse};
use vtagent_core::gemini::models::{SystemInstruction, ToolConfig};
use vtagent_core::gemini::{Client as GeminiClient, Content, GenerateContentRequest, Part, Tool};
use vtagent_core::llm::{factory::create_provider_for_model, provider as uni};
use vtagent_core::models::{ModelId, Provider};
use vtagent_core::prompts::read_system_prompt_from_md;
use vtagent_core::tools::{ToolRegistry, build_function_declarations};
use vtagent_core::utils::utils::summarize_workspace_languages;

use vtagent_core::core::decision_tracker::{Action as DTAction, DecisionOutcome, DecisionTracker};
use vtagent_core::core::router::{Router, TaskClass};
use vtagent_core::core::trajectory::TrajectoryLogger;

// Import syntax highlighting module
use crate::agent::syntax;

fn read_prompt_refiner_prompt() -> Option<String> {
    std::fs::read_to_string("prompts/prompt_refiner.md").ok()
}

pub async fn run_single_agent_loop(config: &CoreAgentConfig) -> Result<()> {
    // Detect provider from model; fall back to prompt-only for non-Gemini
    let provider = config
        .model
        .parse::<ModelId>()
        .ok()
        .map(|m| m.provider())
        .unwrap_or(Provider::Gemini);

    // Load vtagent.toml for tool loop knob and reasoning effort (non-fatal on failure)
    let cfg_manager = ConfigManager::load_from_workspace(&config.workspace).ok();
    let vt_cfg = cfg_manager.as_ref().map(|m| m.config());

    if provider != Provider::Gemini {
        return run_single_agent_loop_unified(config, provider, vt_cfg).await;
    }
    let mut renderer = AnsiRenderer::stdout();
    renderer.line(MessageStyle::Info, "Interactive chat (tools)")?;
    renderer.line(MessageStyle::Output, &format!("Model: {}", config.model))?;
    renderer.line(
        MessageStyle::Output,
        &format!("Workspace: {}", config.workspace.display()),
    )?;
    if let Some(summary) = summarize_workspace_languages(&config.workspace) {
        renderer.line(
            MessageStyle::Output,
            &format!("Detected languages: {}", summary),
        )?;
    }
    renderer.line(MessageStyle::Output, "")?;

    let mut client = GeminiClient::new(config.api_key.clone(), config.model.clone());
    let traj = ConfigManager::load_from_workspace(&config.workspace)
        .ok()
        .map(|m| m.config().telemetry.trajectory_enabled)
        .map(|enabled| {
            if enabled {
                TrajectoryLogger::new(&config.workspace)
            } else {
                TrajectoryLogger::disabled()
            }
        })
        .unwrap_or_else(|| TrajectoryLogger::new(&config.workspace));
    let ledger = DecisionTracker::new();

    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    let function_declarations = build_function_declarations();
    let tools = vec![Tool {
        function_declarations,
    }];
    let _available_tool_names: Vec<String> = tools
        .iter()
        .flat_map(|t| t.function_declarations.iter().map(|d| d.name.clone()))
        .collect();

    let mut conversation_history: Vec<Content> = vec!();
    let _base_system_prompt = read_system_prompt_from_md()
        .unwrap_or_else(|_| "You are a helpful coding assistant for a Rust workspace.".to_string());

    let traj = ConfigManager::load_from_workspace(&config.workspace)
        .ok()
        .map(|m| m.config().telemetry.trajectory_enabled)
        .map(|enabled| {
            if enabled {
                TrajectoryLogger::new(&config.workspace)
            } else {
                TrajectoryLogger::disabled()
            }
        })
        .unwrap_or_else(|| TrajectoryLogger::new(&config.workspace));
    let mut ledger = DecisionTracker::new();

    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    let function_declarations = build_function_declarations();
    let tools = vec![Tool {
        function_declarations,
    }];
    let _available_tool_names: Vec<String> = tools
        .iter()
        .flat_map(|t| t.function_declarations.iter().map(|d| d.name.clone()))
        .collect();

    let mut conversation_history: Vec<Content> = vec![];

    renderer.line(
        MessageStyle::Info,
        "Type 'exit' to quit, 'help' for commands",
    )?;
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "" => continue,
            "exit" | "quit" => {
                renderer.line(MessageStyle::Info, "Goodbye!")?;
                return Ok(());
            }
            "help" => {
                renderer.line(MessageStyle::Info, "Commands: exit, help")?;
                continue;
            }
            _ => {}
        }

        conversation_history.push(Content::user_text(input));

        // Router decision (LLM if configured)
        let decision = if let Some(vt) = vt_cfg {
            Router::route_async(vt, config, &config.api_key, input).await
        } else {
            Router::route(&VTAgentConfig::default(), config, input)
        };

        // Simplified logging with error handling
        traj.log_route(
            conversation_history.len(),
            &decision.selected_model,
            &decision.class.to_string(),
            &input.chars().take(100).collect::<String>(),
        );

        // Ensure Gemini client uses the routed model for this turn
        if decision.selected_model != config.model {
            client = GeminiClient::new(config.api_key.clone(), decision.selected_model.clone());
        }

        let mut working_history = conversation_history.clone();
        // Configurable inner tool-call loop cap: env > vtagent.toml > default(6)
        let max_tool_loops = std::env::var("VTAGENT_MAX_TOOL_LOOPS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .or_else(|| vt_cfg.map(|c| c.tools.max_tool_loops).filter(|&n| n > 0))
            .unwrap_or(6);
        let mut loop_guard = 0;
        let mut any_write_effect = false;
        let mut _final_text: Option<String> = None;

        'outer: loop {
            loop_guard += 1;
            if loop_guard >= max_tool_loops {
                break 'outer;
            }

            // Apply budgets for Gemini-native path as generation_config
            let (gen_cfg, _parallel_any) = if let Some(vt) = vt_cfg {
                // Decide class to pick budget
                let decision = Router::route_async(vt, config, &config.api_key, input).await;
                let key = match decision.class {
                    TaskClass::Simple => "simple",
                    TaskClass::Standard => "standard",
                    TaskClass::Complex => "complex",
                    TaskClass::CodegenHeavy => "codegen_heavy",
                    TaskClass::RetrievalHeavy => "retrieval_heavy",
                };
                if let Some(b) = vt.router.budgets.get(key) {
                    let mut cfg = serde_json::json!({});
                    if let Some(mt) = b.max_tokens {
                        cfg["maxOutputTokens"] = serde_json::json!(mt as u32);
                    }
                    (Some(cfg), b.max_parallel_tools.unwrap_or(0) > 1)
                } else {
                    (None, false)
                }
            } else {
                (None, false)
            };

            // Update decision ledger for this turn
            ledger.start_turn(working_history.len(), Some(input.to_string()));
            ledger.update_available_tools(_available_tool_names.clone());

            // Compose refreshed system instruction including ledger
            let (lg_enabled, lg_max, lg_include) = vt_cfg
                .map(|c| {
                    (
                        c.context.ledger.enabled,
                        c.context.ledger.max_entries,
                        c.context.ledger.include_in_prompt,
                    )
                })
                .unwrap_or((true, 12, true));

            // Update ledger context first
            let last_content = working_history.last().and_then(|m| {
                m.parts.first().and_then(|p| match p {
                    Part::Text { text } => Some(text.clone()),
                    _ => None,
                })
            });
            ledger.start_turn(working_history.len(), last_content);
            let tool_names: Vec<String> = tools
                .iter()
                .flat_map(|t| t.function_declarations.iter().map(|fd| fd.name.clone()))
                .collect();
            ledger.update_available_tools(tool_names);

            let base_system_prompt = read_system_prompt_from_md().unwrap_or_else(|_| {
                "You are a helpful coding assistant for a Rust workspace.".to_string()
            });
            let system_prompt = if lg_enabled && lg_include {
                format!(
                    "{}\n\n[Decision Ledger]\n{}",
                    base_system_prompt,
                    ledger.render_ledger_brief(lg_max)
                )
            } else {
                base_system_prompt
            };
            let sys_inst = SystemInstruction::new(&system_prompt);

            let req = GenerateContentRequest {
                contents: working_history.clone(),
                tools: Some(tools.clone()),
                tool_config: Some(ToolConfig::auto()),
                system_instruction: Some(sys_inst),
                generation_config: gen_cfg,
            };

            let response = match client.generate(&req).await {
                Ok(r) => r,
                Err(e) => {
                    renderer.line(MessageStyle::Error, &format!("Provider error: {e}"))?;
                    break 'outer;
                }
            };
            _final_text = None;
            let mut function_calls: Vec<FunctionCall> = Vec::new();
            if let Some(candidate) = response.candidates.first() {
                for part in &candidate.content.parts {
                    match part {
                        Part::Text { text } => _final_text = Some(text.clone()),
                        Part::FunctionCall { function_call } => {
                            function_calls.push(function_call.clone())
                        }
                        _ => {}
                    }
                }
            }

            if function_calls.is_empty() {
                if let Some(text) = _final_text.clone() {
                    renderer.line(MessageStyle::Response, &text)?;
                }
                if let Some(text) = _final_text {
                    working_history.push(Content::system_text(text));
                }
                break 'outer;
            }

            for call in function_calls {
                let name = call.name.as_str();
                let args = call.args.clone();
                renderer.line(MessageStyle::Info, &format!("[TOOL] {} {}", name, args))?;
                let decision_id = ledger.record_decision(
                    format!("Execute tool '{}' to progress task", name),
                    DTAction::ToolCall {
                        name: name.to_string(),
                        args: args.clone(),
                        expected_outcome: "Use tool output to decide next step".to_string(),
                    },
                    None,
                );

                // Use the ToolRegistry's execute_tool method which handles policy checking
                // This will properly handle the human-in-the-loop confirmation when policy is Prompt
                match tool_registry.execute_tool(name, args).await {
                    Ok(tool_output) => {
                        // Display tool execution results to user
                        // For streaming mode, output was already displayed during execution
                        render_tool_output(&tool_output);
                        if matches!(
                            name,
                            "write_file" | "edit_file" | "create_file" | "delete_file"
                        ) {
                            any_write_effect = true;
                        }
                        let fr = FunctionResponse {
                            name: call.name.clone(),
                            response: tool_output,
                        };
                        working_history.push(Content::user_parts(vec![Part::FunctionResponse {
                            function_response: fr,
                        }]));
                        ledger.record_outcome(
                            &decision_id,
                            DecisionOutcome::Success {
                                result: "tool_ok".to_string(),
                                metrics: Default::default(),
                            },
                        );
                    }
                    Err(e) => {
                        renderer.line(MessageStyle::Error, &format!("Tool error: {e}"))?;
                        let err = serde_json::json!({ "error": e.to_string() });
                        let fr = FunctionResponse {
                            name: call.name.clone(),
                            response: err,
                        };
                        working_history.push(Content::user_parts(vec![Part::FunctionResponse {
                            function_response: fr,
                        }]));
                        ledger.record_outcome(
                            &decision_id,
                            DecisionOutcome::Failure {
                                error: e.to_string(),
                                recovery_attempts: 0,
                                context_preserved: true,
                            },
                        );
                    }
                }
            }
        }

        if let Some(last_system) = working_history.last() {
            if let Some(text) = last_system.parts.first().and_then(|p| p.as_text()) {
                conversation_history.push(Content::system_text(text.to_string()));
            }
        }

        if let Some(last) = conversation_history.last() {
            if let Some(text) = last.parts.first().and_then(|p| p.as_text()) {
                let claims_write = text.contains("I've updated")
                    || text.contains("I have updated")
                    || text.contains("updated the `");
                if claims_write && !any_write_effect {
                    renderer.line(MessageStyle::Output, "")?;
                    renderer.line(
                        MessageStyle::Info,
                        "Note: The assistant mentioned edits but no write tool ran.",
                    )?;
                }
            }
        }
    }
}

async fn run_single_agent_loop_unified(
    config: &CoreAgentConfig,
    _provider: Provider,
    _vt_cfg: Option<&VTAgentConfig>,
) -> Result<()> {
    let mut renderer = AnsiRenderer::stdout();
    renderer.line(MessageStyle::Info, "Interactive chat (tools)")?;
    renderer.line(MessageStyle::Output, &format!("Model: {}", config.model))?;
    renderer.line(
        MessageStyle::Output,
        &format!("Workspace: {}", config.workspace.display()),
    )?;
    if let Some(summary) = summarize_workspace_languages(&config.workspace) {
        renderer.line(
            MessageStyle::Output,
            &format!("Detected languages: {}", summary),
        )?;
    }
    renderer.line(MessageStyle::Output, "")?;

    renderer.line(
        MessageStyle::Info,
        "Type 'exit' to quit, 'help' for commands",
    )?;
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "" => continue,
            "exit" | "quit" => {
                renderer.line(MessageStyle::Info, "Goodbye!")?;
                return Ok(());
            }
            "help" => {
                renderer.line(MessageStyle::Info, "Commands: exit, help")?;
                continue;
            }
            _ => {}
        }

        // For now, just acknowledge the input
        renderer.line(MessageStyle::Output, &format!("You said: {}", input))?;
    }
}

fn render_tool_output(val: &serde_json::Value) {
    let mut renderer = AnsiRenderer::stdout();
    if let Some(stdout) = val.get("stdout").and_then(|v| v.as_str())
        && !stdout.trim().is_empty()
    {
        let _ = renderer.line(MessageStyle::Info, "[stdout]");
        // Try to syntax highlight the output
        match syntax::syntax_highlight_code(stdout) {
            Ok(highlighted) => {
                // Print highlighted output directly since it already contains ANSI codes
                println!("{}", highlighted);
            }
            Err(_) => {
                // Fallback to regular output if highlighting fails
                let _ = renderer.line(MessageStyle::Output, stdout);
            }
        }
    }
    if let Some(stderr) = val.get("stderr").and_then(|v| v.as_str())
        && !stderr.trim().is_empty()
    {
        let _ = renderer.line(MessageStyle::Error, "[stderr]");
        let _ = renderer.line(MessageStyle::Error, stderr);
    }
}

async fn refine_user_prompt_if_enabled(
    raw: &str,
    cfg: &CoreAgentConfig,
    vt_cfg: Option<&VTAgentConfig>,
) -> String {
    if std::env::var("VTAGENT_PROMPT_REFINER_STUB").is_ok() {
        return format!("[REFINED] {}", raw);
    }
    let Some(vtc) = vt_cfg else {
        return raw.to_string();
    };
    if !vtc.agent.refine_prompts_enabled {
        return raw.to_string();
    }

    // Provider-aware defaults for refiner model selection
    let model_provider = cfg
        .model
        .parse::<ModelId>()
        .ok()
        .map(|m| m.provider())
        .unwrap_or(Provider::Gemini);

    let refiner_model = if !vtc.agent.refine_prompts_model.is_empty() {
        vtc.agent.refine_prompts_model.clone()
    } else {
        match model_provider {
            Provider::OpenAI => {
                vtagent_core::config::constants::models::openai::GPT_5_MINI.to_string()
            }
            _ => cfg.model.clone(),
        }
    };

    let Ok(refiner) = create_provider_for_model(&refiner_model, cfg.api_key.clone()) else {
        return raw.to_string();
    };

    let system = read_prompt_refiner_prompt().unwrap_or_else(|| {
        "You are a prompt refiner. Return only the improved prompt.".to_string()
    });
    let req = uni::LLMRequest {
        messages: vec![uni::Message::user(raw.to_string())],
        system_prompt: Some(system),
        tools: None,
        model: refiner_model,
        max_tokens: Some(800),
        temperature: Some(0.3),
        stream: false,
        tool_choice: Some(uni::ToolChoice::none()),
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: Some(vtc.agent.reasoning_effort.clone()),
    };

    match refiner
        .generate(req)
        .await
        .map(|r| r.content.unwrap_or_default())
    {
        Ok(text) if !text.trim().is_empty() => text,
        _ => raw.to_string(),
    }
}
