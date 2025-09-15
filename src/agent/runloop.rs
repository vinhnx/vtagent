use anstyle::{Reset, Style as AnsiStyle};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use vtagent_core::config::loader::{ConfigManager, VTAgentConfig};
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::utils::ansi::{AnsiRenderer, MessageStyle};

// Single-agent engine with Decision Ledger support
use vtagent_core::gemini::function_calling::{FunctionCall, FunctionResponse};
use vtagent_core::gemini::models::{SystemInstruction, ToolConfig};
use vtagent_core::gemini::{Client as GeminiClient, Content, GenerateContentRequest, Part, Tool};
use vtagent_core::llm::{
    AnyClient, factory::create_provider_for_model, make_client, provider as uni,
};
use vtagent_core::models::{ModelId, Provider};
use vtagent_core::prompts::read_system_prompt_from_md;
use vtagent_core::tools::{ToolRegistry, build_function_declarations};
use vtagent_core::utils::utils::summarize_workspace_languages;

use vtagent_core::core::decision_tracker::{Action as DTAction, DecisionOutcome, DecisionTracker};
use vtagent_core::core::router::{Router, TaskClass};
use vtagent_core::core::trajectory::TrajectoryLogger;

fn read_prompt_refiner_prompt() -> Option<String> {
    std::fs::read_to_string("prompts/prompt_refiner.md").ok()
}

fn render_tool_output(val: &serde_json::Value) {
    let mut renderer = AnsiRenderer::stdout();
    let command = val.get("command").and_then(|v| v.as_str()).unwrap_or("");
    if let Some(stdout) = val.get("stdout").and_then(|v| v.as_str()) {
        if !stdout.trim().is_empty() {
            let _ = renderer.line(MessageStyle::Info, "[stdout]");
            if command.starts_with("ls") {
                let _ = render_ls(stdout, &mut renderer);
            } else if command.starts_with("git") {
                let _ = render_git(stdout, &mut renderer);
            } else {
                let _ = renderer.line(MessageStyle::Output, stdout);
            }
        }
    }
    if let Some(stderr) = val.get("stderr").and_then(|v| v.as_str()) {
        if !stderr.trim().is_empty() {
            let _ = renderer.line(MessageStyle::Error, "[stderr]");
            let _ = renderer.line(MessageStyle::Error, stderr);
        }
    }
}

fn parse_ls_colors() -> HashMap<String, AnsiStyle> {
    let mut map = HashMap::new();
    if let Ok(colors) = std::env::var("LS_COLORS") {
        for entry in colors.split(':') {
            if let Some((key, val)) = entry.split_once('=') {
                if let Some(style) = anstyle_ls::parse(val) {
                    map.insert(key.to_string(), style);
                }
            }
        }
    }
    map
}

fn render_ls(stdout: &str, renderer: &mut AnsiRenderer) -> Result<()> {
    let styles = parse_ls_colors();
    for line in stdout.lines() {
        let mut buf = String::new();
        for token in line.split_whitespace() {
            let style = fs::symlink_metadata(token).ok().and_then(|md| {
                if md.file_type().is_dir() {
                    styles.get("di").copied()
                } else if md.file_type().is_symlink() {
                    styles.get("ln").copied()
                } else if md.permissions().mode() & 0o111 != 0 {
                    styles.get("ex").copied()
                } else {
                    styles.get("fi").copied()
                }
            });
            if let Some(st) = style {
                buf.push_str(&format!("{st}{token}{Reset} "));
            } else {
                buf.push_str(token);
                buf.push(' ');
            }
        }
        renderer.raw_line(buf.trim_end())?;
    }
    Ok(())
}

fn render_git(stdout: &str, renderer: &mut AnsiRenderer) -> Result<()> {
    let add_color = std::env::var("GIT_ADD_COLOR").unwrap_or_else(|_| "green".to_string());
    let mod_color = std::env::var("GIT_MODIFY_COLOR").unwrap_or_else(|_| "yellow".to_string());
    let del_color = std::env::var("GIT_DELETE_COLOR").unwrap_or_else(|_| "red".to_string());
    let add_style = anstyle_git::parse(&add_color).unwrap_or_default();
    let mod_style = anstyle_git::parse(&mod_color).unwrap_or_default();
    let del_style = anstyle_git::parse(&del_color).unwrap_or_default();
    for line in stdout.lines() {
        let style = if line.starts_with('A') || line.starts_with("??") {
            add_style
        } else if line.starts_with('M') || line.starts_with(" M") {
            mod_style
        } else if line.starts_with('D') || line.starts_with(" D") {
            del_style
        } else {
            AnsiStyle::new()
        };
        renderer.line_with_style(style, line)?;
    }
    Ok(())
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
        .and_then(|r| Ok(r.content.unwrap_or_default()))
    {
        Ok(text) if !text.trim().is_empty() => text,
        _ => raw.to_string(),
    }
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
    let mut ledger = DecisionTracker::new();

    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    let function_declarations = build_function_declarations();
    let tools = vec![Tool {
        function_declarations,
    }];
    let available_tool_names: Vec<String> = tools
        .iter()
        .flat_map(|t| t.function_declarations.iter().map(|d| d.name.clone()))
        .collect();

    let mut conversation_history: Vec<Content> = vec![];
    let _base_system_prompt = read_system_prompt_from_md()
        .unwrap_or_else(|_| "You are a helpful coding assistant for a Rust workspace.".to_string());

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
                break;
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
        traj.log_route(
            conversation_history.len(),
            &decision.selected_model,
            match decision.class {
                TaskClass::Simple => "simple",
                TaskClass::Standard => "standard",
                TaskClass::Complex => "complex",
                TaskClass::CodegenHeavy => "codegen_heavy",
                TaskClass::RetrievalHeavy => "retrieval_heavy",
            },
            &input.chars().take(120).collect::<String>(),
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
            ledger.update_available_tools(available_tool_names.clone());

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
                    renderer.line(MessageStyle::Output, &text)?;
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
                match tool_registry.execute_tool(name, args).await {
                    Ok(tool_output) => {
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

    Ok(())
}

async fn run_prompt_only_loop(config: &CoreAgentConfig) -> Result<()> {
    let mut renderer = AnsiRenderer::stdout();
    renderer.line(MessageStyle::Info, "Interactive chat (prompt-only)")?;
    renderer.line(MessageStyle::Output, &format!("Model: {}", config.model))?;
    renderer.line(
        MessageStyle::Output,
        &format!("Workspace: {}", config.workspace.display()),
    )?;
    renderer.line(MessageStyle::Info, "Type 'exit' to quit")?;

    // Load VT config for optional router
    let cfg_manager = ConfigManager::load_from_workspace(&config.workspace).ok();
    let vt_cfg = cfg_manager.as_ref().map(|m| m.config());

    let model = config
        .model
        .parse::<ModelId>()
        .map_err(|_| anyhow::anyhow!("Invalid model: {}", config.model))?;
    let mut client: AnyClient = make_client(config.api_key.clone(), model);

    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if matches!(input, "exit" | "quit") {
            renderer.line(MessageStyle::Info, "Goodbye!")?;
            break;
        }

        // Router-based model override per turn
        let selected_model = vt_cfg
            .filter(|c| c.router.enabled)
            .map(|c| Router::route(c, config, input).selected_model)
            .unwrap_or_else(|| config.model.clone());
        // If model differs significantly (provider change), recreate client
        if selected_model != client.model_id() {
            if let Ok(new_model) = selected_model.parse::<ModelId>() {
                client = make_client(config.api_key.clone(), new_model);
            }
        }

        let resp = match client.generate(input).await {
            Ok(r) => r,
            Err(e) => {
                renderer.line(MessageStyle::Error, &format!("Provider error: {e}"))?;
                continue;
            }
        };
        renderer.line(MessageStyle::Output, &resp.content)?;
    }
    Ok(())
}

// Unified single-agent tool-calling loop for OpenAI / Anthropic providers
async fn run_single_agent_loop_unified(
    config: &CoreAgentConfig,
    _provider: Provider,
    vt_cfg: Option<&VTAgentConfig>,
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

    // Create provider client from model + api key
    let provider_client = create_provider_for_model(&config.model, config.api_key.clone())
        .context("Failed to initialize provider client")?;

    // Tool registry + tools as provider-agnostic definitions
    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    // Map Gemini declarations -> universal ToolDefinition
    let declarations = build_function_declarations();
    let tools: Vec<uni::ToolDefinition> = declarations
        .into_iter()
        .map(|decl| uni::ToolDefinition::function(decl.name, decl.description, decl.parameters))
        .collect();

    // Conversation history (provider-agnostic)
    let mut conversation_history: Vec<uni::Message> = vec![];
    let mut ledger = DecisionTracker::new();

    // System prompt (used via system_prompt field in LLMRequest)
    let _system_prompt = read_system_prompt_from_md()
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
                break;
            }
            "help" => {
                renderer.line(MessageStyle::Info, "Commands: exit, help")?;
                continue;
            }
            _ => {}
        }

        // Optional prompt refinement for provider-native path
        let refined_user = refine_user_prompt_if_enabled(input, config, vt_cfg).await;
        conversation_history.push(uni::Message::user(refined_user));

        // Working copy for inner tool loop
        let mut working_history = conversation_history.clone();
        let max_tool_loops = std::env::var("VTAGENT_MAX_TOOL_LOOPS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .or_else(|| vt_cfg.map(|c| c.tools.max_tool_loops).filter(|&n| n > 0))
            .unwrap_or(6);

        let mut loop_guard = 0usize;
        let mut any_write_effect = false;
        let mut _final_text: Option<String> = None;

        'outer: loop {
            loop_guard += 1;
            if loop_guard >= max_tool_loops {
                break 'outer;
            }

            // Build LLMRequest with router-selected model & budgets
            let decision = if let Some(c) = vt_cfg.filter(|c| c.router.enabled) {
                Router::route_async(c, config, &config.api_key, input).await
            } else {
                Router::route(&VTAgentConfig::default(), config, input)
            };
            traj.log_route(
                working_history.len(),
                &decision.selected_model,
                match decision.class {
                    TaskClass::Simple => "simple",
                    TaskClass::Standard => "standard",
                    TaskClass::Complex => "complex",
                    TaskClass::CodegenHeavy => "codegen_heavy",
                    TaskClass::RetrievalHeavy => "retrieval_heavy",
                },
                &input.chars().take(120).collect::<String>(),
            );

            let active_model = decision.selected_model;
            // Apply budgets if present
            let (max_tokens_opt, parallel_cfg_opt) = if let Some(vt) = vt_cfg {
                let key = match decision.class {
                    TaskClass::Simple => "simple",
                    TaskClass::Standard => "standard",
                    TaskClass::Complex => "complex",
                    TaskClass::CodegenHeavy => "codegen_heavy",
                    TaskClass::RetrievalHeavy => "retrieval_heavy",
                };
                let budget = vt.router.budgets.get(key);
                let max_tokens = budget.and_then(|b| b.max_tokens).map(|n| n as u32);
                let parallel = budget.and_then(|b| b.max_parallel_tools).map(|n| {
                    vtagent_core::llm::provider::ParallelToolConfig {
                        disable_parallel_tool_use: n <= 1,
                        max_parallel_tools: Some(n),
                        encourage_parallel: n > 1,
                    }
                });
                (max_tokens, parallel)
            } else {
                (None, None)
            };
            // Inject Decision Ledger for unified providers as well
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
            ledger.start_turn(
                working_history.len(),
                working_history.last().map(|m| m.content.clone()),
            );
            let tool_names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
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

            // Update ledger context and build system prompt with ledger
            ledger.start_turn(
                working_history.len(),
                working_history.last().map(|m| m.content.clone()),
            );
            let tool_names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
            ledger.update_available_tools(tool_names);

            let request = uni::LLMRequest {
                messages: working_history.clone(),
                system_prompt: Some(system_prompt.clone()),
                tools: Some(tools.clone()),
                model: active_model,
                max_tokens: max_tokens_opt.or(Some(2000)),
                temperature: Some(0.7),
                stream: false,
                tool_choice: Some(uni::ToolChoice::auto()),
                parallel_tool_calls: None,
                parallel_tool_config: parallel_cfg_opt,
                reasoning_effort: vt_cfg.map(|c| c.agent.reasoning_effort.clone()),
            };

            let response = match provider_client.generate(request).await {
                Ok(r) => r,
                Err(e) => {
                    renderer.line(MessageStyle::Error, &format!("Provider error: {e}"))?;
                    break 'outer;
                }
            };

            _final_text = response.content.clone();

            // Extract tool calls if any
            if let Some(tool_calls) = response.tool_calls.as_ref() {
                if tool_calls.is_empty() {
                    // No tools requested; finish
                } else {
                    // Execute each function call
                    for call in tool_calls {
                        let name = call.function.name.as_str();
                        let args_val = call
                            .parsed_arguments()
                            .unwrap_or_else(|_| serde_json::json!({}));
                        let dec_id = ledger.record_decision(
                            format!("Execute tool '{}' to progress task", name),
                            DTAction::ToolCall {
                                name: name.to_string(),
                                args: args_val.clone(),
                                expected_outcome: "Use tool output to decide next step".to_string(),
                            },
                            None,
                        );
                        match tool_registry.execute_tool(name, args_val.clone()).await {
                            Ok(tool_output) => {
                                traj.log_tool_call(working_history.len(), name, &args_val, true);
                                render_tool_output(&tool_output);
                                if matches!(
                                    name,
                                    "write_file" | "edit_file" | "create_file" | "delete_file"
                                ) {
                                    any_write_effect = true;
                                }
                                // Send tool response back (OpenAI expects tool role with tool_call_id)
                                let content =
                                    serde_json::to_string(&tool_output).unwrap_or("{}".to_string());
                                working_history
                                    .push(uni::Message::tool_response(call.id.clone(), content));
                                ledger.record_outcome(
                                    &dec_id,
                                    DecisionOutcome::Success {
                                        result: "tool_ok".to_string(),
                                        metrics: Default::default(),
                                    },
                                );
                            }
                            Err(e) => {
                                traj.log_tool_call(working_history.len(), name, &args_val, false);
                                renderer.line(MessageStyle::Error, &format!("Tool error: {e}"))?;
                                let err = serde_json::json!({ "error": e.to_string() });
                                let content = err.to_string();
                                working_history
                                    .push(uni::Message::tool_response(call.id.clone(), content));
                                ledger.record_outcome(
                                    &dec_id,
                                    DecisionOutcome::Failure {
                                        error: e.to_string(),
                                        recovery_attempts: 0,
                                        context_preserved: true,
                                    },
                                );
                            }
                        }
                    }
                    // Continue inner loop to let model use tool results
                    continue;
                }
            }

            // If we reach here, no (more) tool calls
            if let Some(mut text) = _final_text.clone() {
                // Optional self-review/refinement pass based on config
                let do_review = vt_cfg.map(|c| c.agent.enable_self_review).unwrap_or(false);
                let review_passes = vt_cfg
                    .map(|c| c.agent.max_review_passes)
                    .unwrap_or(1)
                    .max(1);
                if do_review {
                    let review_system = "You are the agent's critical code reviewer. Improve clarity, correctness, and add missing test or validation guidance. Return only the improved final answer (no meta commentary).".to_string();
                    for _ in 0..review_passes {
                        let review_req = uni::LLMRequest {
                            messages: vec![uni::Message::user(format!(
                                "Please review and refine the following response. Return only the improved response.\n\n{}",
                                text
                            ))],
                            system_prompt: Some(review_system.clone()),
                            tools: None,
                            model: config.model.clone(),
                            max_tokens: Some(2000),
                            temperature: Some(0.5),
                            stream: false,
                            tool_choice: Some(uni::ToolChoice::none()),
                            parallel_tool_calls: None,
                            parallel_tool_config: None,
                            reasoning_effort: vt_cfg.map(|c| c.agent.reasoning_effort.clone()),
                        };
                        let rr = provider_client.generate(review_req).await.ok();
                        if let Some(r) = rr.and_then(|r| r.content) {
                            if !r.trim().is_empty() {
                                text = r;
                            }
                        }
                    }
                }
                renderer.line(MessageStyle::Output, &text)?;
                working_history.push(uni::Message::assistant(text));
            }
            break 'outer;
        }

        // Commit assistant response to true conversation history
        if let Some(last) = working_history.last() {
            if last.role == uni::MessageRole::Assistant {
                conversation_history.push(last.clone());
            }
        }

        // Post-response guard re: claimed writes without tools
        if let Some(last) = conversation_history.last() {
            if last.role == uni::MessageRole::Assistant {
                let text = &last.content;
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prompt_refinement_applies_to_gemini_when_flag_disabled() {
        // Arrange: enable stub to avoid network
        unsafe {
            std::env::set_var("VTAGENT_PROMPT_REFINER_STUB", "1");
        }

        // Minimal CoreAgentConfig
        let cfg = CoreAgentConfig {
            model: vtagent_core::config::constants::models::google::GEMINI_2_5_FLASH_LITE
                .to_string(),
            api_key: "test".to_string(),
            workspace: std::env::current_dir().unwrap(),
            verbose: false,
        };

        // VT config with refinement enabled and not restricted to OpenAI
        let mut vt = VTAgentConfig::default();
        vt.agent.refine_prompts_enabled = true;
        vt.agent.refine_prompts_only_for_openai = false;

        // Act
        let raw = "make me a list of files";
        let out = refine_user_prompt_if_enabled(raw, &cfg, Some(&vt)).await;

        // Assert
        assert!(out.starts_with("[REFINED] "));

        // Cleanup
        unsafe {
            std::env::remove_var("VTAGENT_PROMPT_REFINER_STUB");
        }
    }
}
