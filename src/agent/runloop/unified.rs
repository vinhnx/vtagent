use anyhow::{Context, Result};
use std::io::{self, Write};

use vtagent_core::config::loader::VTAgentConfig;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::core::decision_tracker::{Action as DTAction, DecisionOutcome, DecisionTracker};
use vtagent_core::core::router::{Router, TaskClass};
use vtagent_core::llm::{factory::create_provider_for_model, provider as uni};
use vtagent_core::tools::{ToolRegistry, build_function_declarations};
use vtagent_core::utils::ansi::{AnsiRenderer, MessageStyle};
use vtagent_core::utils::utils::summarize_workspace_languages;

use super::context::{
    apply_aggressive_trim_unified, enforce_unified_context_window, load_context_trim_config,
    prune_unified_tool_responses,
};
use super::git::confirm_changes_with_git_diff;
use super::is_context_overflow_error;
use super::prompt::refine_user_prompt_if_enabled;
use super::telemetry::build_trajectory_logger;
use super::text_tools::detect_textual_tool_call;
use super::tool_output::render_tool_output;

pub(crate) async fn run_single_agent_loop_unified(
    config: &CoreAgentConfig,
    vt_cfg: Option<&VTAgentConfig>,
    skip_confirmations: bool,
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

    let provider_client = create_provider_for_model(&config.model, config.api_key.clone())
        .context("Failed to initialize provider client")?;

    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    let declarations = build_function_declarations();
    let tools: Vec<uni::ToolDefinition> = declarations
        .into_iter()
        .map(|decl| uni::ToolDefinition::function(decl.name, decl.description, decl.parameters))
        .collect();

    let trim_config = load_context_trim_config(vt_cfg);
    let mut conversation_history: Vec<uni::Message> = vec![];
    let mut ledger = DecisionTracker::new();
    let traj = build_trajectory_logger(&config.workspace, vt_cfg);

    let _system_prompt = read_system_prompt();

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

        let mut working_history = conversation_history.clone();
        let max_tool_loops = std::env::var("VTAGENT_MAX_TOOL_LOOPS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|&value| value > 0)
            .or_else(|| {
                vt_cfg
                    .map(|cfg| cfg.tools.max_tool_loops)
                    .filter(|&value| value > 0)
            })
            .unwrap_or(6);

        let mut loop_guard = 0usize;
        let mut any_write_effect = false;

        'outer: loop {
            loop_guard += 1;
            if loop_guard >= max_tool_loops {
                break 'outer;
            }

            let _ = enforce_unified_context_window(&mut working_history, trim_config);

            let decision = if let Some(cfg) = vt_cfg.filter(|cfg| cfg.router.enabled) {
                Router::route_async(cfg, config, &config.api_key, input).await
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
                    vtagent_core::llm::provider::ParallelToolConfig {
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

            let base_system_prompt = read_system_prompt();
            let system_prompt = if lg_enabled && lg_include {
                format!(
                    "{}\n\n[Decision Ledger]\n{}",
                    base_system_prompt,
                    ledger.render_ledger_brief(lg_max)
                )
            } else {
                base_system_prompt
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

                match provider_client.generate(request).await {
                    Ok(result) => {
                        working_history = attempt_history.clone();
                        break result;
                    }
                    Err(error) => {
                        let error_text = error.to_string();
                        if is_context_overflow_error(&error_text)
                            && retry_attempts <= vtagent_core::config::constants::context::CONTEXT_ERROR_RETRY_LIMIT
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
                                        vtagent_core::config::constants::context::CONTEXT_ERROR_RETRY_LIMIT,
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

            let mut final_text = response.content.clone();
            let mut tool_calls = response.tool_calls.clone().unwrap_or_default();
            let mut interpreted_textual_call = false;

            if tool_calls.is_empty() {
                if let Some(text) = final_text.clone() {
                    if let Some((name, args)) = detect_textual_tool_call(&text) {
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
                }
            }

            if tool_calls.is_empty() {
                if let Some(text) = final_text.clone() {
                    working_history.push(uni::Message::assistant(text));
                }
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
                            let modified_files: Vec<String> = if let Some(files) = tool_output
                                .get("modified_files")
                                .and_then(|value| value.as_array())
                            {
                                files
                                    .iter()
                                    .filter_map(|file| file.as_str().map(|value| value.to_string()))
                                    .collect()
                            } else {
                                vec![]
                            };

                            if matches!(
                                name,
                                "write_file" | "edit_file" | "create_file" | "delete_file" | "srgn"
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
                                renderer
                                    .line(MessageStyle::Info, "Changes applied successfully.")?;
                            } else if !modified_files.is_empty() {
                                renderer.line(MessageStyle::Info, "Changes discarded.")?;
                            }

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
                        Err(error) => {
                            traj.log_tool_call(working_history.len(), name, &args_val, false);
                            renderer.line(MessageStyle::Error, &format!("Tool error: {error}"))?;
                            let err = serde_json::json!({ "error": error.to_string() });
                            let content = err.to_string();
                            working_history
                                .push(uni::Message::tool_response(call.id.clone(), content));
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
                        if let Some(r) = rr.and_then(|result| result.content) {
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

fn read_system_prompt() -> String {
    vtagent_core::prompts::read_system_prompt_from_md()
        .unwrap_or_else(|_| "You are a helpful coding assistant for a Rust workspace.".to_string())
}
