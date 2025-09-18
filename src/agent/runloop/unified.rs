use anyhow::{Context, Result, anyhow};
use std::io;
use std::path::Path;

use vtcode_core::config::constants::{defaults, tools};
use vtcode_core::config::loader::VTCodeConfig;
use vtcode_core::config::types::AgentConfig as CoreAgentConfig;
use vtcode_core::core::decision_tracker::{Action as DTAction, DecisionOutcome, DecisionTracker};
use vtcode_core::core::router::{Router, TaskClass};
use vtcode_core::llm::{factory::create_provider_for_model, provider as uni};
use vtcode_core::tools::registry::{ToolErrorType, ToolExecutionError};
use vtcode_core::tools::{ToolRegistry, build_function_declarations};
use vtcode_core::ui::{Spinner, theme};
use vtcode_core::utils::ansi::{AnsiRenderer, MessageStyle};
use vtcode_core::utils::dot_config::update_theme_preference;

use super::context::{
    apply_aggressive_trim_unified, enforce_unified_context_window, load_context_trim_config,
    prune_unified_tool_responses,
};
use super::git::confirm_changes_with_git_diff;
use super::is_context_overflow_error;
use super::prompt::refine_user_prompt_if_enabled;
use super::slash_commands::{SlashCommandOutcome, handle_slash_command};
use super::telemetry::build_trajectory_logger;
use super::text_tools::detect_textual_tool_call;
use super::tool_output::render_tool_output;
use super::ui::render_session_banner;
use super::welcome::prepare_session_bootstrap;

fn persist_theme_preference(renderer: &mut AnsiRenderer, theme_id: &str) -> Result<()> {
    if let Err(err) = update_theme_preference(theme_id) {
        renderer.line(
            MessageStyle::Error,
            &format!("Failed to persist theme preference: {}", err),
        )?;
    }
    Ok(())
}

fn ensure_turn_bottom_gap(renderer: &mut AnsiRenderer, applied: &mut bool) -> Result<()> {
    if !*applied {
        renderer.line(MessageStyle::Output, "")?;
        *applied = true;
    }
    Ok(())
}

pub(crate) async fn run_single_agent_loop_unified(
    config: &CoreAgentConfig,
    vt_cfg: Option<&VTCodeConfig>,
    skip_confirmations: bool,
    full_auto: bool,
) -> Result<()> {
    let session_bootstrap = prepare_session_bootstrap(config, vt_cfg);
    let mut renderer = AnsiRenderer::stdout();
    render_session_banner(&mut renderer, config, &session_bootstrap)?;

    if let Some(text) = session_bootstrap.welcome_text.as_ref() {
        renderer.line(MessageStyle::Response, text)?;
        renderer.line(MessageStyle::Output, "")?;
    }

    let placeholder_hint = session_bootstrap.placeholder.clone();
    let mut placeholder_shown = false;

    let provider_client = create_provider_for_model(&config.model, config.api_key.clone())
        .context("Failed to initialize provider client")?;

    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    if let Some(cfg) = vt_cfg {
        if let Err(err) = tool_registry.apply_config_policies(&cfg.tools) {
            eprintln!(
                "Warning: Failed to apply tool policies from config: {}",
                err
            );
        }
    }

    if full_auto {
        let automation_cfg = vt_cfg
            .map(|cfg| cfg.automation.full_auto.clone())
            .ok_or_else(|| anyhow!("Full-auto configuration unavailable"))?;

        tool_registry.enable_full_auto_mode(&automation_cfg.allowed_tools);
        let allowlist = tool_registry
            .current_full_auto_allowlist()
            .unwrap_or_default();
        if allowlist.is_empty() {
            renderer.line(
                MessageStyle::Info,
                "Full-auto mode enabled with no tool permissions; tool calls will be skipped.",
            )?;
        } else {
            renderer.line(
                MessageStyle::Info,
                &format!(
                    "Full-auto mode enabled. Permitted tools: {}",
                    allowlist.join(", ")
                ),
            )?;
        }
    }
    let declarations = build_function_declarations();
    let tools: Vec<uni::ToolDefinition> = declarations
        .into_iter()
        .map(|decl| uni::ToolDefinition::function(decl.name, decl.description, decl.parameters))
        .collect();

    let trim_config = load_context_trim_config(vt_cfg);
    let mut conversation_history: Vec<uni::Message> = vec![];
    let mut ledger = DecisionTracker::new();
    let traj = build_trajectory_logger(&config.workspace, vt_cfg);
    let base_system_prompt = read_system_prompt(
        &config.workspace,
        session_bootstrap.prompt_addendum.as_deref(),
    );

    renderer.line(
        MessageStyle::Info,
        "Type 'exit' to quit, 'help' for commands",
    )?;
    renderer.line(
        MessageStyle::Info,
        "Slash commands: /help, /list-themes, /theme <id>, /command <program>",
    )?;
    loop {
        if !placeholder_shown {
            if let Some(ref hint) = placeholder_hint {
                renderer.line(MessageStyle::Info, &format!("Suggested input: {}", hint))?;
            }
            placeholder_shown = true;
        }
        let styles = theme::active_styles();
        renderer.inline_with_style(styles.primary, "❯ ")?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

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

        if let Some(command_input) = input.strip_prefix('/') {
            match handle_slash_command(command_input, &mut renderer)? {
                SlashCommandOutcome::Handled => continue,
                SlashCommandOutcome::ThemeChanged(theme_id) => {
                    persist_theme_preference(&mut renderer, &theme_id)?;
                    continue;
                }
                SlashCommandOutcome::ExecuteTool { name, args } => {
                    match tool_registry.preflight_tool_permission(&name) {
                        Ok(true) => {
                            let tool_spinner = Spinner::new(&format!("Running tool: {}", name));
                            match tool_registry.execute_tool(&name, args.clone()).await {
                                Ok(tool_output) => {
                                    tool_spinner.finish_and_clear();
                                    traj.log_tool_call(
                                        conversation_history.len(),
                                        &name,
                                        &args,
                                        true,
                                    );
                                    render_tool_output(&tool_output);
                                }
                                Err(err) => {
                                    tool_spinner.finish_and_clear();
                                    traj.log_tool_call(
                                        conversation_history.len(),
                                        &name,
                                        &args,
                                        false,
                                    );
                                    renderer.line(
                                        MessageStyle::Error,
                                        &format!("Tool '{}' failed: {}", name, err),
                                    )?;
                                }
                            }
                        }
                        Ok(false) => {
                            let denial = ToolExecutionError::new(
                                name.clone(),
                                ToolErrorType::PolicyViolation,
                                format!("Tool '{}' execution denied by policy", name),
                            )
                            .to_json_value();
                            traj.log_tool_call(conversation_history.len(), &name, &args, false);
                            render_tool_output(&denial);
                        }
                        Err(err) => {
                            traj.log_tool_call(conversation_history.len(), &name, &args, false);
                            renderer.line(
                                MessageStyle::Error,
                                &format!("Failed to evaluate policy for tool '{}': {}", name, err),
                            )?;
                        }
                    }
                    continue;
                }
                SlashCommandOutcome::Exit => {
                    renderer.line(MessageStyle::Info, "Goodbye!")?;
                    break;
                }
            }
        }

        let refined_user = refine_user_prompt_if_enabled(input, config, vt_cfg).await;
        conversation_history.push(uni::Message::user(refined_user));
        let _pruned_tools = prune_unified_tool_responses(
            &mut conversation_history,
            trim_config.preserve_recent_turns,
        );
        // Removed: Tool response pruning message
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

        let mut working_history = conversation_history.clone();
        let max_tool_loops = vt_cfg
            .map(|cfg| cfg.tools.max_tool_loops)
            .filter(|&value| value > 0)
            .unwrap_or(defaults::DEFAULT_MAX_TOOL_LOOPS);

        let mut loop_guard = 0usize;
        let mut any_write_effect = false;
        let mut last_tool_stdout: Option<String> = None;
        let mut bottom_gap_applied = false;

        'outer: loop {
            if loop_guard == 0 {
                renderer.line(MessageStyle::Output, "")?;
            }
            loop_guard += 1;
            if loop_guard >= max_tool_loops {
                if !bottom_gap_applied {
                    renderer.line(MessageStyle::Output, "")?;
                }
                let notice = format!(
                    "I reached the configured tool-call limit of {} for this turn and paused further tool execution. Increase `tools.max_tool_loops` in vtcode.toml if you need more, then ask me to continue.",
                    max_tool_loops
                );
                renderer.line(MessageStyle::Response, &notice)?;
                ensure_turn_bottom_gap(&mut renderer, &mut bottom_gap_applied)?;
                working_history.push(uni::Message::assistant(notice));
                break 'outer;
            }

            let _ = enforce_unified_context_window(&mut working_history, trim_config);

            let decision = if let Some(cfg) = vt_cfg.filter(|cfg| cfg.router.enabled) {
                Router::route_async(cfg, config, &config.api_key, input).await
            } else {
                Router::route(&VTCodeConfig::default(), config, input)
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
            let (max_tokens_opt, parallel_cfg_opt) = if let Some(vt) = vt_cfg {
                let key = match decision.class {
                    TaskClass::Simple => "simple",
                    TaskClass::Standard => "standard",
                    TaskClass::Complex => "complex",
                    TaskClass::CodegenHeavy => "codegen_heavy",
                    TaskClass::RetrievalHeavy => "retrieval_heavy",
                };
                let budget = vt.router.budgets.get(key);
                let max_tokens = budget.and_then(|b| b.max_tokens).map(|value| value as u32);
                let parallel = budget.and_then(|b| b.max_parallel_tools).map(|value| {
                    vtcode_core::llm::provider::ParallelToolConfig {
                        disable_parallel_tool_use: value <= 1,
                        max_parallel_tools: Some(value),
                        encourage_parallel: value > 1,
                    }
                });
                (max_tokens, parallel)
            } else {
                (None, None)
            };

            let (lg_enabled, lg_max, lg_include) = vt_cfg
                .map(|cfg| {
                    (
                        cfg.context.ledger.enabled,
                        cfg.context.ledger.max_entries,
                        cfg.context.ledger.include_in_prompt,
                    )
                })
                .unwrap_or((true, 12, true));

            ledger.start_turn(
                working_history.len(),
                working_history
                    .last()
                    .map(|message| message.content.clone()),
            );
            let tool_names: Vec<String> = tools
                .iter()
                .map(|tool| tool.function.name.clone())
                .collect();
            ledger.update_available_tools(tool_names);

            let system_prompt = if lg_enabled && lg_include {
                format!(
                    "{}\n\n[Decision Ledger]\n{}",
                    base_system_prompt,
                    ledger.render_ledger_brief(lg_max)
                )
            } else {
                base_system_prompt.clone()
            };

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
                    reasoning_effort: vt_cfg.map(|cfg| cfg.agent.reasoning_effort.clone()),
                };

                let spinner = Spinner::new("Thinking");
                match provider_client.generate(request).await {
                    Ok(result) => {
                        spinner.finish_and_clear();
                        working_history = attempt_history.clone();
                        break result;
                    }
                    Err(error) => {
                        let error_text = error.to_string();
                        if is_context_overflow_error(&error_text)
                            && retry_attempts <= vtcode_core::config::constants::context::CONTEXT_ERROR_RETRY_LIMIT
                        {
                            let removed_tool_messages = prune_unified_tool_responses(
                                &mut attempt_history,
                                trim_config.preserve_recent_turns,
                            );
                            let removed_turns =
                                apply_aggressive_trim_unified(&mut attempt_history, trim_config);
                            let total_removed = removed_tool_messages + removed_turns;
                            if total_removed > 0 {
                                spinner.finish_and_clear();
                                renderer.line(MessageStyle::Info, "↻ Adjusting context")?;
                                renderer.line(
                                    MessageStyle::Info,
                                    &format!(
                                        "Context overflow detected; removed {} older messages (retry {}/{}).",
                                        total_removed,
                                        retry_attempts,
                                        vtcode_core::config::constants::context::CONTEXT_ERROR_RETRY_LIMIT,
                                    ),
                                )?;
                                conversation_history.clone_from(&attempt_history);
                                continue;
                            }
                        }
                        spinner.finish_and_clear();

                        let has_tool = working_history
                            .iter()
                            .any(|msg| msg.role == uni::MessageRole::Tool);

                        if has_tool {
                            eprintln!("Provider error (suppressed): {error_text}");
                            let reply = derive_recent_tool_output(&working_history)
                                .unwrap_or_else(|| "Command completed successfully.".to_string());
                            renderer.line(MessageStyle::Response, &reply)?;
                            ensure_turn_bottom_gap(&mut renderer, &mut bottom_gap_applied)?;
                            working_history.push(uni::Message::assistant(reply));
                            let _ = last_tool_stdout.take();
                            break 'outer;
                        } else {
                            renderer.line(
                                MessageStyle::Error,
                                &format!("Provider error: {error_text}"),
                            )?;
                        }

                        continue 'outer;
                    }
                }
            };

            let mut final_text = response.content.clone();
            let mut tool_calls = response.tool_calls.clone().unwrap_or_default();
            let mut interpreted_textual_call = false;

            if tool_calls.is_empty()
                && let Some(text) = final_text.clone()
                && let Some((name, args)) = detect_textual_tool_call(&text)
            {
                let args_display =
                    serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string());
                renderer.line(
                    MessageStyle::Info,
                    &format!(
                        "Interpreting textual tool request as {} {}",
                        &name, &args_display
                    ),
                )?;
                let call_id = format!("call_textual_{}", working_history.len());
                tool_calls.push(uni::ToolCall::function(
                    call_id.clone(),
                    name.clone(),
                    args_display.clone(),
                ));
                interpreted_textual_call = true;
                final_text = None;
            }

            if tool_calls.is_empty()
                && let Some(text) = final_text.clone()
            {
                working_history.push(uni::Message::assistant(text));
            } else {
                let assistant_text = if interpreted_textual_call {
                    String::new()
                } else {
                    final_text.clone().unwrap_or_default()
                };
                working_history.push(uni::Message::assistant_with_tools(
                    assistant_text,
                    tool_calls.clone(),
                ));
                for call in &tool_calls {
                    let name = call.function.name.as_str();
                    let args_val = call
                        .parsed_arguments()
                        .unwrap_or_else(|_| serde_json::json!({}));
                    let args_display =
                        serde_json::to_string(&args_val).unwrap_or_else(|_| "{}".to_string());
                    renderer.line(
                        MessageStyle::Tool,
                        &format!("[TOOL] {} {}", name, args_display),
                    )?;
                    let dec_id = ledger.record_decision(
                        format!("Execute tool '{}' to progress task", name),
                        DTAction::ToolCall {
                            name: name.to_string(),
                            args: args_val.clone(),
                            expected_outcome: "Use tool output to decide next step".to_string(),
                        },
                        None,
                    );

                    match tool_registry.preflight_tool_permission(name) {
                        Ok(true) => {
                            let tool_spinner = Spinner::new(&format!("Running tool: {}", name));
                            match tool_registry.execute_tool(name, args_val.clone()).await {
                                Ok(tool_output) => {
                                    tool_spinner.finish_and_clear();
                                    traj.log_tool_call(
                                        working_history.len(),
                                        name,
                                        &args_val,
                                        true,
                                    );
                                    render_tool_output(&tool_output);
                                    last_tool_stdout = tool_output
                                        .get("stdout")
                                        .and_then(|value| value.as_str())
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty());
                                    let modified_files: Vec<String> = if let Some(files) =
                                        tool_output
                                            .get("modified_files")
                                            .and_then(|value| value.as_array())
                                    {
                                        files
                                            .iter()
                                            .filter_map(|file| {
                                                file.as_str().map(|value| value.to_string())
                                            })
                                            .collect()
                                    } else {
                                        vec![]
                                    };

                                    if matches!(
                                        name,
                                        "write_file"
                                            | "edit_file"
                                            | "create_file"
                                            | "delete_file"
                                            | "srgn"
                                    ) {
                                        any_write_effect = true;
                                    }

                                    if !modified_files.is_empty()
                                        && confirm_changes_with_git_diff(
                                            &modified_files,
                                            skip_confirmations,
                                        )
                                        .await?
                                    {
                                        renderer.line(
                                            MessageStyle::Info,
                                            "Changes applied successfully.",
                                        )?;
                                    } else if !modified_files.is_empty() {
                                        renderer.line(MessageStyle::Info, "Changes discarded.")?;
                                    }

                                    let content = serde_json::to_string(&tool_output)
                                        .unwrap_or("{}".to_string());
                                    working_history.push(uni::Message::tool_response(
                                        call.id.clone(),
                                        content,
                                    ));
                                    ledger.record_outcome(
                                        &dec_id,
                                        DecisionOutcome::Success {
                                            result: "tool_ok".to_string(),
                                            metrics: Default::default(),
                                        },
                                    );

                                    if should_short_circuit_shell(input, name, &args_val) {
                                        let reply = last_tool_stdout.clone().unwrap_or_else(|| {
                                            "Command completed successfully.".to_string()
                                        });
                                        renderer.line(MessageStyle::Response, &reply)?;
                                        ensure_turn_bottom_gap(
                                            &mut renderer,
                                            &mut bottom_gap_applied,
                                        )?;
                                        working_history.push(uni::Message::assistant(reply));
                                        let _ = last_tool_stdout.take();
                                        break 'outer;
                                    }
                                }
                                Err(error) => {
                                    tool_spinner.finish_and_clear();
                                    renderer.line(
                                        MessageStyle::Tool,
                                        &format!("Tool {} failed.", name),
                                    )?;
                                    traj.log_tool_call(
                                        working_history.len(),
                                        name,
                                        &args_val,
                                        false,
                                    );
                                    renderer.line(
                                        MessageStyle::Error,
                                        &format!("Tool error: {error}"),
                                    )?;
                                    let err = serde_json::json!({ "error": error.to_string() });
                                    let content = err.to_string();
                                    working_history.push(uni::Message::tool_response(
                                        call.id.clone(),
                                        content,
                                    ));
                                    let _ = last_tool_stdout.take();
                                    ledger.record_outcome(
                                        &dec_id,
                                        DecisionOutcome::Failure {
                                            error: error.to_string(),
                                            recovery_attempts: 0,
                                            context_preserved: true,
                                        },
                                    );
                                }
                            }
                        }
                        Ok(false) => {
                            let denial = ToolExecutionError::new(
                                name.to_string(),
                                ToolErrorType::PolicyViolation,
                                format!("Tool '{}' execution denied by policy", name),
                            )
                            .to_json_value();
                            traj.log_tool_call(working_history.len(), name, &args_val, false);
                            render_tool_output(&denial);
                            let content =
                                serde_json::to_string(&denial).unwrap_or("{}".to_string());
                            working_history
                                .push(uni::Message::tool_response(call.id.clone(), content));
                            ledger.record_outcome(
                                &dec_id,
                                DecisionOutcome::Failure {
                                    error: format!("Tool '{}' execution denied by policy", name),
                                    recovery_attempts: 0,
                                    context_preserved: true,
                                },
                            );
                            continue;
                        }
                        Err(err) => {
                            traj.log_tool_call(working_history.len(), name, &args_val, false);
                            renderer.line(
                                MessageStyle::Error,
                                &format!("Failed to evaluate policy for tool '{}': {}", name, err),
                            )?;
                            let err_json = serde_json::json!({
                                "error": format!(
                                    "Policy evaluation error for '{}' : {}",
                                    name, err
                                )
                            });
                            working_history.push(uni::Message::tool_response(
                                call.id.clone(),
                                err_json.to_string(),
                            ));
                            let _ = last_tool_stdout.take();
                            ledger.record_outcome(
                                &dec_id,
                                DecisionOutcome::Failure {
                                    error: format!(
                                        "Failed to evaluate policy for tool '{}': {}",
                                        name, err
                                    ),
                                    recovery_attempts: 0,
                                    context_preserved: true,
                                },
                            );
                            continue;
                        }
                    }
                }
                continue;
            }

            if let Some(mut text) = final_text.clone() {
                let do_review = vt_cfg
                    .map(|cfg| cfg.agent.enable_self_review)
                    .unwrap_or(false);
                let review_passes = vt_cfg
                    .map(|cfg| cfg.agent.max_review_passes)
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
                            reasoning_effort: vt_cfg.map(|cfg| cfg.agent.reasoning_effort.clone()),
                        };
                        let rr = provider_client.generate(review_req).await.ok();
                        if let Some(r) = rr.and_then(|result| result.content)
                            && !r.trim().is_empty()
                        {
                            text = r;
                        }
                    }
                }
                let trimmed = text.trim();
                let suppress_response = trimmed.is_empty()
                    || last_tool_stdout
                        .as_ref()
                        .map(|stdout| stdout == trimmed)
                        .unwrap_or(false);

                if !suppress_response {
                    renderer.line(MessageStyle::Response, &text)?;
                    ensure_turn_bottom_gap(&mut renderer, &mut bottom_gap_applied)?;
                }
                working_history.push(uni::Message::assistant(text));
                let _ = last_tool_stdout.take();
            } else {
                ensure_turn_bottom_gap(&mut renderer, &mut bottom_gap_applied)?;
            }
            break 'outer;
        }

        conversation_history = working_history;

        let _pruned_after_turn = prune_unified_tool_responses(
            &mut conversation_history,
            trim_config.preserve_recent_turns,
        );
        // Removed: Tool response pruning message after completion
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

        if let Some(last) = conversation_history.last()
            && last.role == uni::MessageRole::Assistant
        {
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

    Ok(())
}

fn read_system_prompt(workspace: &Path, session_addendum: Option<&str>) -> String {
    let mut prompt = vtcode_core::prompts::read_system_prompt_from_md()
        .unwrap_or_else(|_| "You are a helpful coding assistant for a Rust workspace.".to_string());

    if let Some(overview) = vtcode_core::utils::utils::build_project_overview(workspace) {
        prompt.push_str("\n\n## PROJECT OVERVIEW\n");
        prompt.push_str(&overview.as_prompt_block());
    }

    if let Some(guidelines) = vtcode_core::prompts::system::read_agent_guidelines(workspace) {
        prompt.push_str("\n\n## AGENTS.MD GUIDELINES\n");
        prompt.push_str(&guidelines);
    }

    if let Some(addendum) = session_addendum {
        let trimmed = addendum.trim();
        if !trimmed.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(trimmed);
        }
    }

    prompt
}

fn should_short_circuit_shell(input: &str, tool_name: &str, args: &serde_json::Value) -> bool {
    if tool_name != tools::RUN_TERMINAL_CMD && tool_name != tools::BASH {
        return false;
    }

    let command = args
        .get("command")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            let mut tokens = Vec::new();
            for item in items {
                if let Some(text) = item.as_str() {
                    tokens.push(text.trim_matches(|c| c == '\"' || c == '\'').to_string());
                } else {
                    return None;
                }
            }
            Some(tokens)
        });

    let Some(command_tokens) = command else {
        return false;
    };

    if command_tokens.is_empty() {
        return false;
    }

    // Don't short-circuit for commands that contain shell metacharacters
    // as these need more sophisticated reasoning
    let full_command = command_tokens.join(" ");
    if full_command.contains('|')
        || full_command.contains('>')
        || full_command.contains('<')
        || full_command.contains('&')
        || full_command.contains(';')
    {
        return false;
    }

    let user_tokens: Vec<String> = input
        .split_whitespace()
        .map(|part| part.trim_matches(|c| c == '\"' || c == '\'').to_string())
        .collect();

    if user_tokens.is_empty() {
        return false;
    }

    if user_tokens.len() != command_tokens.len() {
        return false;
    }

    user_tokens
        .iter()
        .zip(command_tokens.iter())
        .all(|(user, cmd)| user == cmd)
}

fn derive_recent_tool_output(history: &[uni::Message]) -> Option<String> {
    let message = history
        .iter()
        .rev()
        .find(|msg| msg.role == uni::MessageRole::Tool)?;

    let value = serde_json::from_str::<serde_json::Value>(&message.content).ok()?;

    let mut output_parts = Vec::new();

    // Add stdout if present
    if let Some(stdout) = value
        .get("stdout")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        output_parts.push(format!("Output:\n{}", stdout));
    }

    // Add stderr if present
    if let Some(stderr) = value
        .get("stderr")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        output_parts.push(format!("Errors:\n{}", stderr));
    }

    // Add exit code if non-zero
    if let Some(exit_code) = value.get("exit_code").and_then(|v| v.as_i64()) {
        if exit_code != 0 {
            output_parts.push(format!("Exit code: {}", exit_code));
        }
    }

    // Add command info if it was a piped command
    if let Some(used_shell) = value.get("used_shell").and_then(|v| v.as_bool()) {
        if used_shell {
            if let Some(command) = value.get("command").and_then(|v| v.as_str()) {
                output_parts.push(format!("Command executed: {}", command));
            }
        }
    }

    if output_parts.is_empty() {
        if let Some(result) = value
            .get("result")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return Some(result);
        }
        return Some("Command completed successfully.".to_string());
    }

    Some(output_parts.join("\n\n"))
}
