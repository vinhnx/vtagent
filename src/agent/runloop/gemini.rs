use anyhow::Result;
use std::io::{self, Write};

use vtagent_core::config::loader::VTAgentConfig;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::core::decision_tracker::{Action as DTAction, DecisionOutcome, DecisionTracker};
use vtagent_core::core::router::{Router, TaskClass};
use vtagent_core::gemini::function_calling::{FunctionCall, FunctionResponse};
use vtagent_core::gemini::models::{SystemInstruction, ToolConfig};
use vtagent_core::gemini::{Client as GeminiClient, Content, GenerateContentRequest, Part, Tool};
use vtagent_core::tools::{ToolRegistry, build_function_declarations};
use vtagent_core::utils::ansi::{AnsiRenderer, MessageStyle};
use vtagent_core::utils::utils::summarize_workspace_languages;

use super::context::{
    apply_aggressive_trim_gemini, enforce_gemini_context_window, load_context_trim_config,
    prune_gemini_tool_responses,
};
use super::git::confirm_changes_with_git_diff;
use super::is_context_overflow_error;
use super::telemetry::build_trajectory_logger;
use super::text_tools::detect_textual_tool_call;
use super::tool_output::render_tool_output;

pub(crate) async fn run_single_agent_loop_gemini(
    config: &CoreAgentConfig,
    vt_cfg: Option<&VTAgentConfig>,
    skip_confirmations: bool,
) -> Result<()> {
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
    let traj = build_trajectory_logger(&config.workspace, vt_cfg);
    let mut ledger = DecisionTracker::new();

    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    let function_declarations = build_function_declarations();
    let tools = vec![Tool {
        function_declarations,
    }];
    let available_tool_names: Vec<String> = tools
        .iter()
        .flat_map(|tool| {
            tool.function_declarations
                .iter()
                .map(|declaration| declaration.name.clone())
        })
        .collect();

    let mut conversation_history: Vec<Content> = vec![];
    let _base_system_prompt = read_system_prompt();

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

        if decision.selected_model != config.model {
            client = GeminiClient::new(config.api_key.clone(), decision.selected_model.clone());
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

            let mut final_text: Option<String> = None;

            let _ = enforce_gemini_context_window(&mut working_history, trim_config);

            let (gen_cfg, _parallel_any) = if let Some(vt) = vt_cfg {
                let decision = Router::route_async(vt, config, &config.api_key, input).await;
                let key = match decision.class {
                    TaskClass::Simple => "simple",
                    TaskClass::Standard => "standard",
                    TaskClass::Complex => "complex",
                    TaskClass::CodegenHeavy => "codegen_heavy",
                    TaskClass::RetrievalHeavy => "retrieval_heavy",
                };
                if let Some(budget) = vt.router.budgets.get(key) {
                    let mut cfg = serde_json::json!({});
                    if let Some(max_tokens) = budget.max_tokens {
                        cfg["maxOutputTokens"] = serde_json::json!(max_tokens as u32);
                    }
                    (Some(cfg), budget.max_parallel_tools.unwrap_or(0) > 1)
                } else {
                    (None, false)
                }
            } else {
                (None, false)
            };

            ledger.start_turn(working_history.len(), Some(input.to_string()));
            ledger.update_available_tools(available_tool_names.clone());

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
                working_history.last().and_then(|message| {
                    message.parts.first().and_then(|part| match part {
                        Part::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                }),
            );
            let tool_names: Vec<String> = tools
                .iter()
                .flat_map(|tool| {
                    tool.function_declarations
                        .iter()
                        .map(|declaration| declaration.name.clone())
                })
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
                    Ok(result) => {
                        working_history = attempt_history.clone();
                        break result;
                    }
                    Err(error) => {
                        if is_context_overflow_error(&error.to_string())
                            && retry_attempts <= vtagent_core::config::constants::context::CONTEXT_ERROR_RETRY_LIMIT
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
                                        vtagent_core::config::constants::context::CONTEXT_ERROR_RETRY_LIMIT,
                                    ),
                                )?;
                                conversation_history.clone_from(&attempt_history);
                                continue;
                            }
                        }
                        renderer.line(MessageStyle::Error, &format!("Provider error: {error}"))?;
                        break 'outer;
                    }
                }
            };

            let mut aggregated_text: Vec<String> = Vec::new();
            let mut function_calls: Vec<FunctionCall> = Vec::new();
            let mut response_content: Option<Content> = None;
            if let Some(candidate) = response.candidates.first() {
                response_content = Some(candidate.content.clone());
                for part in &candidate.content.parts {
                    match part {
                        Part::Text { text } => aggregated_text.push(text.clone()),
                        Part::FunctionCall { function_call } => {
                            function_calls.push(function_call.clone())
                        }
                        _ => {}
                    }
                }
            }

            if !aggregated_text.is_empty() {
                final_text = Some(aggregated_text.join("\n"));
            }

            if function_calls.is_empty() {
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
                        let fc = FunctionCall {
                            name,
                            args,
                            id: None,
                        };
                        if let Some(ref mut content) = response_content {
                            content.parts = vec![Part::FunctionCall {
                                function_call: fc.clone(),
                            }];
                        } else {
                            response_content = Some(Content {
                                role: "model".to_string(),
                                parts: vec![Part::FunctionCall {
                                    function_call: fc.clone(),
                                }],
                            });
                        }
                        function_calls.push(fc);
                        final_text = None;
                    }
                }
            }

            if let Some(content) = response_content {
                working_history.push(content);
            }

            if function_calls.is_empty() {
                if let Some(text) = final_text.clone() {
                    if !text.trim().is_empty() {
                        renderer.line(MessageStyle::Response, &text)?;
                    }
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
                            && confirm_changes_with_git_diff(&modified_files, skip_confirmations)
                                .await?
                        {
                            renderer.line(MessageStyle::Info, "Changes applied successfully.")?;
                        } else if !modified_files.is_empty() {
                            renderer.line(MessageStyle::Info, "Changes discarded.")?;
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
                    Err(error) => {
                        renderer.line(MessageStyle::Error, &format!("Tool error: {error}"))?;
                        let err = serde_json::json!({ "error": error.to_string() });
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
                                error: error.to_string(),
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
            if let Some(text) = last.parts.first().and_then(|part| part.as_text()) {
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
