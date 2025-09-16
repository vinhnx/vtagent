use anyhow::{Context, Result};
use std::io::{self, Write};
use vtagent_core::config::constants::context as context_defaults;
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

fn read_prompt_refiner_prompt() -> Option<String> {
    std::fs::read_to_string("prompts/prompt_refiner.md").ok()
}

fn render_tool_output(val: &serde_json::Value) {
    let mut renderer = AnsiRenderer::stdout();
    if let Some(stdout) = val.get("stdout").and_then(|v| v.as_str())
        && !stdout.trim().is_empty()
    {
        let _ = renderer.line(MessageStyle::Info, "[stdout]");
        let _ = renderer.line(MessageStyle::Output, stdout);
    }
    if let Some(stderr) = val.get("stderr").and_then(|v| v.as_str())
        && !stderr.trim().is_empty()
    {
        let _ = renderer.line(MessageStyle::Error, "[stderr]");
        let _ = renderer.line(MessageStyle::Error, stderr);
    }
}

#[derive(Clone, Copy)]
struct ContextTrimConfig {
    max_tokens: usize,
    trim_to_percent: u8,
    preserve_recent_turns: usize,
}

impl ContextTrimConfig {
    fn target_tokens(&self) -> usize {
        let percent = (self.trim_to_percent as u128).clamp(
            context_defaults::MIN_TRIM_RATIO_PERCENT as u128,
            context_defaults::MAX_TRIM_RATIO_PERCENT as u128,
        );
        ((self.max_tokens as u128) * percent / 100) as usize
    }
}

#[derive(Default)]
struct ContextTrimOutcome {
    removed_messages: usize,
}

impl ContextTrimOutcome {
    fn is_trimmed(&self) -> bool {
        self.removed_messages > 0
    }
}

fn prune_gemini_tool_responses(history: &mut Vec<Content>, preserve_recent_turns: usize) -> usize {
    if history.is_empty() {
        return 0;
    }

    let keep_from = history.len().saturating_sub(preserve_recent_turns);
    if keep_from == 0 {
        return 0;
    }

    let mut removed = 0usize;
    let mut index = 0usize;
    history.retain(|message| {
        let contains_tool_response = message
            .parts
            .iter()
            .any(|part| matches!(part, Part::FunctionResponse { .. }));
        let keep = index >= keep_from || !contains_tool_response;
        if !keep {
            removed += 1;
        }
        index += 1;
        keep
    });
    removed
}

fn prune_unified_tool_responses(
    history: &mut Vec<uni::Message>,
    preserve_recent_turns: usize,
) -> usize {
    if history.is_empty() {
        return 0;
    }

    let keep_from = history.len().saturating_sub(preserve_recent_turns);
    if keep_from == 0 {
        return 0;
    }

    let mut removed = 0usize;
    let mut index = 0usize;
    history.retain(|message| {
        let contains_tool_payload = message.is_tool_response() || message.has_tool_calls();
        let keep = index >= keep_from || !contains_tool_payload;
        if !keep {
            removed += 1;
        }
        index += 1;
        keep
    });
    removed
}

fn apply_aggressive_trim_gemini(history: &mut Vec<Content>, config: ContextTrimConfig) -> usize {
    if history.is_empty() {
        return 0;
    }

    let keep_turns = config
        .preserve_recent_turns
        .min(context_defaults::AGGRESSIVE_PRESERVE_RECENT_TURNS)
        .max(context_defaults::MIN_PRESERVE_RECENT_TURNS)
        .min(history.len());

    let remove = history.len().saturating_sub(keep_turns);
    if remove == 0 {
        return 0;
    }

    history.drain(0..remove);
    remove
}

fn apply_aggressive_trim_unified(
    history: &mut Vec<uni::Message>,
    config: ContextTrimConfig,
) -> usize {
    if history.is_empty() {
        return 0;
    }

    let keep_turns = config
        .preserve_recent_turns
        .min(context_defaults::AGGRESSIVE_PRESERVE_RECENT_TURNS)
        .max(context_defaults::MIN_PRESERVE_RECENT_TURNS)
        .min(history.len());

    let remove = history.len().saturating_sub(keep_turns);
    if remove == 0 {
        return 0;
    }

    history.drain(0..remove);
    remove
}

fn is_context_overflow_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("context length")
        || lower.contains("context window")
        || lower.contains("maximum context")
        || lower.contains("model is overloaded")
        || lower.contains("reduce the amount")
        || lower.contains("token limit")
        || lower.contains("503")
}

fn load_context_trim_config(vt_cfg: Option<&VTAgentConfig>) -> ContextTrimConfig {
    let context_cfg = vt_cfg.map(|cfg| &cfg.context);
    let max_tokens = std::env::var("VTAGENT_CONTEXT_TOKEN_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .or_else(|| {
            context_cfg
                .map(|cfg| cfg.max_context_tokens)
                .filter(|value| *value > 0)
        })
        .unwrap_or(context_defaults::DEFAULT_MAX_TOKENS);

    let trim_to_percent = context_cfg
        .map(|cfg| cfg.trim_to_percent)
        .unwrap_or(context_defaults::DEFAULT_TRIM_TO_PERCENT)
        .clamp(
            context_defaults::MIN_TRIM_RATIO_PERCENT,
            context_defaults::MAX_TRIM_RATIO_PERCENT,
        );

    let preserve_recent_turns = context_cfg
        .map(|cfg| cfg.preserve_recent_turns)
        .unwrap_or(context_defaults::DEFAULT_PRESERVE_RECENT_TURNS)
        .max(context_defaults::MIN_PRESERVE_RECENT_TURNS);

    ContextTrimConfig {
        max_tokens,
        trim_to_percent,
        preserve_recent_turns,
    }
}

fn enforce_gemini_context_window(
    history: &mut Vec<Content>,
    config: ContextTrimConfig,
) -> ContextTrimOutcome {
    if history.is_empty() {
        return ContextTrimOutcome::default();
    }

    let tokens_per_message: Vec<usize> = history
        .iter()
        .map(approximate_gemini_message_tokens)
        .collect();
    let mut total_tokens: usize = tokens_per_message.iter().sum();

    if total_tokens <= config.max_tokens {
        return ContextTrimOutcome::default();
    }

    let target_tokens = config.target_tokens();
    let mut remove_count = 0usize;
    let mut preserve_boundary = history.len().saturating_sub(config.preserve_recent_turns);
    if preserve_boundary > history.len().saturating_sub(1) {
        preserve_boundary = history.len().saturating_sub(1);
    }

    while remove_count < preserve_boundary && total_tokens > config.max_tokens {
        total_tokens = total_tokens.saturating_sub(tokens_per_message[remove_count]);
        remove_count += 1;
        if total_tokens <= target_tokens {
            break;
        }
    }

    while remove_count < history.len().saturating_sub(1) && total_tokens > config.max_tokens {
        total_tokens = total_tokens.saturating_sub(tokens_per_message[remove_count]);
        remove_count += 1;
    }

    if remove_count == 0 {
        return ContextTrimOutcome::default();
    }

    history.drain(0..remove_count);
    ContextTrimOutcome {
        removed_messages: remove_count,
    }
}

fn enforce_unified_context_window(
    history: &mut Vec<uni::Message>,
    config: ContextTrimConfig,
) -> ContextTrimOutcome {
    if history.is_empty() {
        return ContextTrimOutcome::default();
    }

    let tokens_per_message: Vec<usize> = history
        .iter()
        .map(approximate_unified_message_tokens)
        .collect();
    let mut total_tokens: usize = tokens_per_message.iter().sum();

    if total_tokens <= config.max_tokens {
        return ContextTrimOutcome::default();
    }

    let target_tokens = config.target_tokens();
    let mut remove_count = 0usize;
    let mut preserve_boundary = history.len().saturating_sub(config.preserve_recent_turns);
    if preserve_boundary > history.len().saturating_sub(1) {
        preserve_boundary = history.len().saturating_sub(1);
    }

    while remove_count < preserve_boundary && total_tokens > config.max_tokens {
        total_tokens = total_tokens.saturating_sub(tokens_per_message[remove_count]);
        remove_count += 1;
        if total_tokens <= target_tokens {
            break;
        }
    }

    while remove_count < history.len().saturating_sub(1) && total_tokens > config.max_tokens {
        total_tokens = total_tokens.saturating_sub(tokens_per_message[remove_count]);
        remove_count += 1;
    }

    if remove_count == 0 {
        return ContextTrimOutcome::default();
    }

    history.drain(0..remove_count);
    ContextTrimOutcome {
        removed_messages: remove_count,
    }
}

fn approximate_gemini_message_tokens(message: &Content) -> usize {
    let mut total_chars = message.role.len();
    for part in &message.parts {
        match part {
            Part::Text { text } => {
                total_chars += text.len();
            }
            Part::FunctionCall { function_call } => {
                total_chars += function_call.name.len();
                total_chars += serde_json::to_string(&function_call.args)
                    .map(|value| value.len())
                    .unwrap_or_default();
            }
            Part::FunctionResponse { function_response } => {
                total_chars += function_response.name.len();
                total_chars += serde_json::to_string(&function_response.response)
                    .map(|value| value.len())
                    .unwrap_or_default();
            }
        }
    }

    total_chars.div_ceil(context_defaults::CHAR_PER_TOKEN_APPROX)
}

fn approximate_unified_message_tokens(message: &uni::Message) -> usize {
    let mut total_chars = message.content.len();
    total_chars += message.role.as_generic_str().len();

    if let Some(tool_calls) = &message.tool_calls {
        for call in tool_calls {
            total_chars += call.id.len();
            total_chars += call.call_type.len();
            total_chars += call.function.name.len();
            total_chars += call.function.arguments.len();
        }
    }

    if let Some(tool_call_id) = &message.tool_call_id {
        total_chars += tool_call_id.len();
    }

    total_chars.div_ceil(context_defaults::CHAR_PER_TOKEN_APPROX)
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
    let trim_config = load_context_trim_config(vt_cfg);
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
        let pruned_tools = prune_gemini_tool_responses(
            &mut conversation_history,
            trim_config.preserve_recent_turns,
        );
        if pruned_tools > 0 {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Dropped {} earlier tool responses to conserve context.",
                    pruned_tools
                ),
            )?;
        }
        let trim_result = enforce_gemini_context_window(&mut conversation_history, trim_config);
        if trim_result.is_trimmed() {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Trimmed {} earlier messages to respect the context window (~{} tokens).",
                    trim_result.removed_messages, trim_config.max_tokens,
                ),
            )?;
        }

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

            let _ = enforce_gemini_context_window(&mut working_history, trim_config);

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

            let mut attempt_history = working_history.clone();
            let mut retry_attempts = 0usize;
            let response = loop {
                retry_attempts += 1;
                let _ = enforce_gemini_context_window(&mut attempt_history, trim_config);

                let req = GenerateContentRequest {
                    contents: attempt_history.clone(),
                    tools: Some(tools.clone()),
                    tool_config: Some(ToolConfig::auto()),
                    system_instruction: Some(sys_inst.clone()),
                    generation_config: gen_cfg.clone(),
                };

                match client.generate(&req).await {
                    Ok(r) => {
                        working_history = attempt_history.clone();
                        break r;
                    }
                    Err(e) => {
                        if is_context_overflow_error(&e.to_string())
                            && retry_attempts <= context_defaults::CONTEXT_ERROR_RETRY_LIMIT
                        {
                            let removed_tool_messages = prune_gemini_tool_responses(
                                &mut attempt_history,
                                trim_config.preserve_recent_turns,
                            );
                            let removed_turns =
                                apply_aggressive_trim_gemini(&mut attempt_history, trim_config);
                            let total_removed = removed_tool_messages + removed_turns;
                            if total_removed > 0 {
                                renderer.line(
                                    MessageStyle::Info,
                                    &format!(
                                        "Context overflow detected; removed {} older messages (retry {}/{}).",
                                        total_removed,
                                        retry_attempts,
                                        context_defaults::CONTEXT_ERROR_RETRY_LIMIT,
                                    ),
                                )?;
                                conversation_history.clone_from(&attempt_history);
                                continue;
                            }
                        }
                        renderer.line(MessageStyle::Error, &format!("Provider error: {e}"))?;
                        break 'outer;
                    }
                }
            };

            _final_text = None;
            let mut function_calls: Vec<FunctionCall> = Vec::new();
            let mut response_content: Option<Content> = None;
            if let Some(candidate) = response.candidates.first() {
                response_content = Some(candidate.content.clone());
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

            if let Some(content) = response_content {
                working_history.push(content);
            }

            if function_calls.is_empty() {
                if let Some(text) = _final_text.clone() {
                    renderer.line(MessageStyle::Response, &text)?;
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

        conversation_history = working_history;

        let pruned_after_turn = prune_gemini_tool_responses(
            &mut conversation_history,
            trim_config.preserve_recent_turns,
        );
        if pruned_after_turn > 0 {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Dropped {} older tool responses after completion to conserve context.",
                    pruned_after_turn
                ),
            )?;
        }

        let post_trim = enforce_gemini_context_window(&mut conversation_history, trim_config);
        if post_trim.is_trimmed() {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Trimmed {} earlier messages to respect the context window (~{} tokens).",
                    post_trim.removed_messages, trim_config.max_tokens,
                ),
            )?;
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

    let trim_config = load_context_trim_config(vt_cfg);

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
        let pruned_tools = prune_unified_tool_responses(
            &mut conversation_history,
            trim_config.preserve_recent_turns,
        );
        if pruned_tools > 0 {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Dropped {} earlier tool responses to conserve context.",
                    pruned_tools
                ),
            )?;
        }
        let trim_result = enforce_unified_context_window(&mut conversation_history, trim_config);
        if trim_result.is_trimmed() {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Trimmed {} earlier messages to respect the context window (~{} tokens).",
                    trim_result.removed_messages, trim_config.max_tokens,
                ),
            )?;
        }

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

            let _ = enforce_unified_context_window(&mut working_history, trim_config);

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

            let mut attempt_history = working_history.clone();
            let mut retry_attempts = 0usize;
            let response = loop {
                retry_attempts += 1;
                let _ = enforce_unified_context_window(&mut attempt_history, trim_config);

                let request = uni::LLMRequest {
                    messages: attempt_history.clone(),
                    system_prompt: Some(system_prompt.clone()),
                    tools: Some(tools.clone()),
                    model: active_model.clone(),
                    max_tokens: max_tokens_opt.or(Some(2000)),
                    temperature: Some(0.7),
                    stream: false,
                    tool_choice: Some(uni::ToolChoice::auto()),
                    parallel_tool_calls: None,
                    parallel_tool_config: parallel_cfg_opt.clone(),
                    reasoning_effort: vt_cfg.map(|c| c.agent.reasoning_effort.clone()),
                };

                match provider_client.generate(request).await {
                    Ok(r) => {
                        working_history = attempt_history.clone();
                        break r;
                    }
                    Err(e) => {
                        let error_text = e.to_string();
                        if is_context_overflow_error(&error_text)
                            && retry_attempts <= context_defaults::CONTEXT_ERROR_RETRY_LIMIT
                        {
                            let removed_tool_messages = prune_unified_tool_responses(
                                &mut attempt_history,
                                trim_config.preserve_recent_turns,
                            );
                            let removed_turns =
                                apply_aggressive_trim_unified(&mut attempt_history, trim_config);
                            let total_removed = removed_tool_messages + removed_turns;
                            if total_removed > 0 {
                                renderer.line(
                                    MessageStyle::Info,
                                    &format!(
                                        "Context overflow detected; removed {} older messages (retry {}/{}).",
                                        total_removed,
                                        retry_attempts,
                                        context_defaults::CONTEXT_ERROR_RETRY_LIMIT,
                                    ),
                                )?;
                                conversation_history.clone_from(&attempt_history);
                                continue;
                            }
                        }
                        renderer.line(
                            MessageStyle::Error,
                            &format!("Provider error: {error_text}"),
                        )?;
                        continue 'outer;
                    }
                }
            };

            _final_text = response.content.clone();

            // Extract tool calls if any
            if let Some(tool_calls) = response.tool_calls.clone() {
                if tool_calls.is_empty() {
                    if let Some(text) = _final_text.clone() {
                        working_history.push(uni::Message::assistant(text));
                    }
                } else {
                    if let Some(text) = _final_text.clone() {
                        working_history
                            .push(uni::Message::assistant_with_tools(text, tool_calls.clone()));
                    }
                    // Execute each function call
                    for call in &tool_calls {
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
            } else if let Some(text) = _final_text.clone() {
                working_history.push(uni::Message::assistant(text));
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
                renderer.line(MessageStyle::Response, &text)?;
                working_history.push(uni::Message::assistant(text));
            }
            break 'outer;
        }

        // Commit turn transcript (including tool traffic) back to conversation history
        conversation_history = working_history;

        let pruned_after_turn = prune_unified_tool_responses(
            &mut conversation_history,
            trim_config.preserve_recent_turns,
        );
        if pruned_after_turn > 0 {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Dropped {} older tool responses after completion to conserve context.",
                    pruned_after_turn
                ),
            )?;
        }

        let post_trim = enforce_unified_context_window(&mut conversation_history, trim_config);
        if post_trim.is_trimmed() {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Trimmed {} earlier messages to respect the context window (~{} tokens).",
                    post_trim.removed_messages, trim_config.max_tokens,
                ),
            )?;
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

    #[test]
    fn test_enforce_gemini_context_window_trims_excess_tokens() {
        let mut history: Vec<Content> = (0..16)
            .map(|i| Content::user_text(format!("message {}", i)))
            .collect();
        let original_len = history.len();
        let config = ContextTrimConfig {
            max_tokens: 24,
            trim_to_percent: 75,
            preserve_recent_turns: 4,
        };

        let outcome = enforce_gemini_context_window(&mut history, config);

        assert!(outcome.is_trimmed());
        assert_eq!(original_len - history.len(), outcome.removed_messages);

        let remaining_tokens: usize = history.iter().map(approximate_gemini_message_tokens).sum();
        assert!(remaining_tokens <= config.max_tokens);

        let last_text = history
            .last()
            .and_then(|msg| msg.parts.first().and_then(|p| p.as_text()))
            .unwrap_or_default();
        assert_eq!(last_text, "message 15");
    }

    #[test]
    fn test_enforce_unified_context_window_trims_and_preserves_latest() {
        let mut history: Vec<uni::Message> = (0..12)
            .map(|i| uni::Message::assistant(format!("assistant step {}", i)))
            .collect();
        let original_len = history.len();
        let config = ContextTrimConfig {
            max_tokens: 18,
            trim_to_percent: 70,
            preserve_recent_turns: 3,
        };

        let outcome = enforce_unified_context_window(&mut history, config);

        assert!(outcome.is_trimmed());
        assert_eq!(original_len - history.len(), outcome.removed_messages);

        let remaining_tokens: usize = history.iter().map(approximate_unified_message_tokens).sum();
        assert!(remaining_tokens <= config.max_tokens);

        let last_content = history
            .last()
            .map(|msg| msg.content.clone())
            .unwrap_or_default();
        assert!(last_content.contains("assistant step 11"));
    }

    #[test]
    fn test_prune_gemini_tool_responses_removes_older_entries() {
        let mut history = vec![
            Content::user_text("keep0"),
            Content::user_parts(vec![Part::FunctionResponse {
                function_response: FunctionResponse {
                    name: "tool_a".to_string(),
                    response: serde_json::json!({"output": "value"}),
                },
            }]),
            Content {
                role: "model".to_string(),
                parts: vec![Part::Text {
                    text: "assistant0".to_string(),
                }],
            },
            Content::user_text("keep1"),
            Content::user_parts(vec![Part::FunctionResponse {
                function_response: FunctionResponse {
                    name: "tool_b".to_string(),
                    response: serde_json::json!({"output": "new"}),
                },
            }]),
            Content {
                role: "model".to_string(),
                parts: vec![Part::Text {
                    text: "assistant1".to_string(),
                }],
            },
        ];

        let removed = prune_gemini_tool_responses(&mut history, 4);

        assert_eq!(removed, 1);
        assert_eq!(history.len(), 5);
        assert!(history.iter().any(|msg| {
            msg.parts
                .iter()
                .any(|part| matches!(part, Part::FunctionResponse { .. }))
        }));
        assert_eq!(
            history
                .last()
                .and_then(|msg| msg.parts.first())
                .and_then(|part| part.as_text()),
            Some("assistant1")
        );
    }

    #[test]
    fn test_prune_unified_tool_responses_respects_recent_history() {
        let mut history: Vec<uni::Message> = vec![
            uni::Message::user("keep".to_string()),
            uni::Message::tool_response("call_1".to_string(), "{\"result\":1}".to_string()),
            uni::Message::assistant("assistant0".to_string()),
            uni::Message::user("keep2".to_string()),
            {
                let mut msg = uni::Message::assistant("assistant_with_tool".to_string());
                msg.tool_calls = Some(vec![uni::ToolCall::function(
                    "call_2".to_string(),
                    "tool_b".to_string(),
                    "{}".to_string(),
                )]);
                msg
            },
            uni::Message::tool_response("call_2".to_string(), "{\"result\":2}".to_string()),
        ];

        let removed = prune_unified_tool_responses(&mut history, 4);

        assert_eq!(removed, 1);
        assert!(history.len() >= 4);
        assert_eq!(history.first().unwrap().content, "keep".to_string());
        assert!(history.iter().any(|msg| msg.is_tool_response()));
    }

    #[test]
    fn test_apply_aggressive_trim_gemini_limits_history() {
        let mut history: Vec<Content> = (0..14)
            .map(|i| Content::user_text(format!("message {i}")))
            .collect();
        let config = ContextTrimConfig {
            max_tokens: 120,
            trim_to_percent: 75,
            preserve_recent_turns: 12,
        };

        let removed = apply_aggressive_trim_gemini(&mut history, config);

        let expected_len = context_defaults::AGGRESSIVE_PRESERVE_RECENT_TURNS;
        assert_eq!(removed, 14 - expected_len);
        assert_eq!(history.len(), expected_len);
        let expected_first = format!(
            "message {}",
            14 - context_defaults::AGGRESSIVE_PRESERVE_RECENT_TURNS
        );
        assert_eq!(
            history
                .first()
                .and_then(|msg| msg.parts.first())
                .and_then(|part| part.as_text()),
            Some(expected_first.as_str())
        );
    }

    #[test]
    fn test_apply_aggressive_trim_unified_limits_history() {
        let mut history: Vec<uni::Message> = (0..15)
            .map(|i| uni::Message::assistant(format!("assistant step {i}")))
            .collect();
        let config = ContextTrimConfig {
            max_tokens: 140,
            trim_to_percent: 80,
            preserve_recent_turns: 10,
        };

        let removed = apply_aggressive_trim_unified(&mut history, config);

        let expected_len = context_defaults::AGGRESSIVE_PRESERVE_RECENT_TURNS;
        assert_eq!(removed, 15 - expected_len);
        assert_eq!(history.len(), expected_len);
        let expected_first = format!(
            "assistant step {}",
            15 - context_defaults::AGGRESSIVE_PRESERVE_RECENT_TURNS
        );
        assert!(
            history
                .first()
                .map(|msg| msg.content.clone())
                .unwrap_or_default()
                .contains(&expected_first)
        );
    }
}
