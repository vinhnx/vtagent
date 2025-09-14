use anyhow::{Context, Result};
use console::style;
use vtagent_core::{
    config::{ConfigManager, constants::tools},
    gemini::{Tool, GenerateContentRequest, Content, Part},
    gemini::models::SystemInstruction,
    llm::make_client,
    models::ModelId,
    prompts::read_system_prompt_from_md,
    tools::{ToolRegistry, build_function_declarations},
    types::AgentConfig as CoreAgentConfig,
    utils::summarize_workspace_languages,
};
use vtagent_core::gemini::function_calling::{FunctionCall, FunctionResponse};
use vtagent_core::gemini::models::ToolConfig;
use std::io::{self, Write};
use serde_json::json;
use regex::Regex;

/// Handle the chat command
pub async fn handle_chat_command(config: &CoreAgentConfig, force_multi_agent: bool, skip_confirmations: bool) -> Result<()> {
    eprintln!("[DEBUG] Entering handle_chat_command");
    eprintln!("[DEBUG] Workspace: {:?}", config.workspace);
    eprintln!("[DEBUG] Model: {}", config.model);

    println!("{}", style("Interactive chat mode selected").blue().bold());
    println!("Model: {}", config.model);
    println!("Workspace: {}", config.workspace.display());

    if let Some(summary) = summarize_workspace_languages(&config.workspace) {
        println!("Detected languages: {}", summary);
        eprintln!("[DEBUG] Language detection: {}", summary);
    }
    println!();

    // Create model-agnostic client
    let model_id = config.model.parse::<ModelId>().map_err(|_| {
        anyhow::anyhow!("Invalid model: {}", config.model)
    })?;
    let mut client = make_client(config.api_key.clone(), model_id);

    // Initialize tool registry and function declarations
    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    let function_declarations = build_function_declarations();
    let tools = vec![Tool {
        function_declarations,
    }];

    // Load configuration from vtagent.toml first
    let config_manager = ConfigManager::load_from_workspace(&config.workspace)
        .context("Failed to load configuration")?;
    let vtcode_config = config_manager.config();

    // Multi-agent mode logic
    if force_multi_agent || vtcode_config.multi_agent.enabled {
        println!("{}", style("Multi-agent mode enabled").green().bold());
        // Multi-agent implementation would go here
        println!("Multi-agent functionality not fully implemented in this minimal version");
        return Ok(());
    }

    // Single agent mode - Chat loop implementation
    println!("{}", style("Single agent mode").cyan());
    println!("Type 'exit' to quit, 'help' for commands");

    // Initialize conversation history (no system messages in contents for Gemini)
    let mut conversation_history: Vec<Content> = vec![];

    // Create system instruction
    let system_instruction = SystemInstruction::new(
        &read_system_prompt_from_md()
            .unwrap_or_else(|_| "You are a helpful coding assistant. You can help with programming tasks, code analysis, and file operations.".to_string())
    );

    loop {
        // Get user input
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Handle special commands
        match input {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  exit/quit - Exit the chat");
                println!("  help - Show this help message");
                println!("  Any other text will be sent to the AI assistant");
                continue;
            }
            "" => continue,
            _ => {}
        }

        // Add user message to history
        conversation_history.push(Content::user_text(input));

        // Create request
        let request = GenerateContentRequest {
            contents: conversation_history.clone(),
            tools: Some(tools.clone()),
            tool_config: Some(ToolConfig::auto()),
            system_instruction: Some(system_instruction.clone()),
            generation_config: None,
        };

        // Tool-aware response loop: handle function calls, then re-ask until no calls
        let mut loop_guard = 0;
        let mut any_write_effect = false;
        let mut final_text: Option<String> = None;
        let mut working_history = conversation_history.clone();

        'outer: loop {
            loop_guard += 1;
            if loop_guard > 4 { break 'outer; }

            let req = GenerateContentRequest {
                contents: working_history.clone(),
                tools: Some(tools.clone()),
                tool_config: Some(ToolConfig::auto()),
                system_instruction: Some(system_instruction.clone()),
                generation_config: None,
            };

            let response = match client.generate(&req).await {
                Ok(r) => r,
                Err(e) => { eprintln!("Error: {}", e); break 'outer; }
            };

            // default: capture first text
            final_text = None;
            let mut function_calls: Vec<FunctionCall> = Vec::new();
            if let Some(candidate) = response.candidates.first() {
                for part in &candidate.content.parts {
                    match part {
                        Part::Text { text } => {
                            // hold onto most recent text; we will print after tool loop ends
                            final_text = Some(text.clone());
                        }
                        Part::FunctionCall { function_call } => {
                            function_calls.push(function_call.clone());
                        }
                        _ => {}
                    }
                }
            }

            if function_calls.is_empty() {
                // No tool calls: print final_text if present and finish
                if let Some(text) = final_text.clone() { println!("{}", text); }
                // Commit the assistant text to true history
                if let Some(text) = final_text { conversation_history.push(Content::system_text(text)); }
                break 'outer;
            }

            // Execute each function call, with human-in-the-loop policy prompt in REPL,
            // append function responses, then continue loop
            for call in function_calls {
                let name = call.name.as_str();
                let args = call.args.clone();
                eprintln!("[TOOL] {} {}", name, args);

                // Use the ToolRegistry's execute_tool method which handles policy checking
                // This will properly handle the human-in-the-loop confirmation when policy is Prompt
                match tool_registry.execute_tool(name, args).await {
                    Ok(tool_output) => {
                        // Display tool execution results to user
                        // For streaming mode, output was already displayed during execution
                        let is_streaming = tool_output.get("streaming").and_then(|s| s.as_bool()).unwrap_or(false);
                        let shell_rendered = tool_output.get("shell_rendered").and_then(|s| s.as_bool()).unwrap_or(false);

                        if !is_streaming || !shell_rendered {
                            if let Some(stdout) = tool_output.get("stdout").and_then(|s| s.as_str()) {
                                if !stdout.trim().is_empty() {
                                    println!("{}", stdout);
                                }
                            }
                            if let Some(stderr) = tool_output.get("stderr").and_then(|s| s.as_str()) {
                                if !stderr.trim().is_empty() {
                                    eprintln!("{}", stderr);
                                }
                            }
                        }

                        // For PTY mode, show additional PTY information
                        if tool_output.get("pty_enabled").and_then(|p| p.as_bool()).unwrap_or(false) {
                            if !is_streaming {
                                println!("{}", style("[PTY Session Completed]").blue().bold());
                            } else {
                                println!("{}", style("[PTY Streaming Session Completed]").green().bold());
                            }
                        }
                        if tool_output.get("streaming_enabled").and_then(|s| s.as_bool()).unwrap_or(false) {
                            println!("{}", style("[Streaming Session Completed]").green().bold());
                        }

                        // Heuristic: mark write effects for write/edit/create/delete
                        if matches!(name, "write_file" | "edit_file" | "create_file" | "delete_file") {
                            any_write_effect = true;
                        }
                        let fr = FunctionResponse { name: call.name.clone(), response: tool_output };
                        // Push function response as a new content turn
                        working_history.push(Content::user_parts(vec![Part::FunctionResponse { function_response: fr }]));
                    }
                    Err(e) => {
                        // Check if the error is due to policy denial
                        if e.to_string().contains("execution denied by policy") {
                            println!("{} Tool '{}' was denied by policy. You can change this with :policy commands in chat mode.",
                                style("[DENIED]").yellow().bold(), name);
                        } else {
                            eprintln!("{} Tool '{}' failed: {}", style("[ERROR]").red().bold(), name, e);
                        }
                        let err = json!({ "error": e.to_string() });
                        let fr = FunctionResponse { name: call.name.clone(), response: err };
                        working_history.push(Content::user_parts(vec![Part::FunctionResponse { function_response: fr }]));
                    }
                }
            }
            // loop continues to let the model use the tool results
        }

        // Post-response guard with config-driven behavior
        if let Some(last) = conversation_history.last() {
            if let Some(text) = last.parts.first().and_then(|p| p.as_text()) {
                let claims_write = text.contains("updated the `") || text.contains("I've updated") || text.contains("I have updated");

                // Detect any patch blocks (Codex or fenced unified diffs)
                let detected_patch = detect_any_patch_block(text);

                let cfg_manager = ConfigManager::load_from_workspace(&config.workspace).ok();
                let gating = cfg_manager.as_ref().map(|m| m.config().security.require_write_tool_for_claims).unwrap_or(true);
                let auto_apply = cfg_manager.as_ref().map(|m| m.config().security.auto_apply_detected_patches).unwrap_or(false);

                if gating && claims_write && !any_write_effect {
                    println!("\nProposed changes (not applied): The assistant reported edits but no write tool ran.");
                    if let Some(patch) = detected_patch {
                        if auto_apply {
                            match tool_registry.execute_tool(tools::APPLY_PATCH, json!({"input": patch})).await {
                                Ok(out) => println!("Patch applied: {}", out),
                                Err(e) => eprintln!("Failed to apply patch: {}", e),
                            }
                        } else {
                            println!("Apply detected patch now? [y/N]");
                            io::stdout().flush().ok();
                            let mut ans = String::new();
                            io::stdin().read_line(&mut ans).ok();
                            if matches!(ans.trim().to_lowercase().as_str(), "y" | "yes") {
                                match tool_registry.execute_tool(tools::APPLY_PATCH, json!({"input": patch})).await {
                                    Ok(out) => println!("Patch applied: {}", out),
                                    Err(e) => eprintln!("Failed to apply patch: {}", e),
                                }
                            } else {
                                println!("Skipped applying patch. You can copy the patch into apply_patch later.");
                            }
                        }
                    } else {
                        println!("No patch block detected. Ask the assistant to provide a patch or call edit_file/write_file.");
                    }
                } else if claims_write && !any_write_effect {
                    println!("\nNote: No write tools were executed. The changes shown above are not persisted. Use edit_file/write_file or approve tool policy to apply.");
                }
            }
        }
    }

    Ok(())
}

// Detect either Codex patch blocks or fenced unified diff blocks and return a
// Codex-compatible patch string if possible.
fn detect_any_patch_block(text: &str) -> Option<String> {
    if let Some(p) = detect_codex_patch(text) { return Some(p); }
    if let Some(p) = detect_fenced_unified_diffs_as_codex(text) { return Some(p); }
    None
}

fn detect_codex_patch(text: &str) -> Option<String> {
    if let Some(start) = text.find("*** Begin Patch") {
        if let Some(end) = text.find("*** End Patch") {
            let end_ix = end + "*** End Patch".len();
            return Some(text[start..end_ix].to_string());
        }
    }
    None
}

fn detect_fenced_unified_diffs_as_codex(text: &str) -> Option<String> {
    let re = Regex::new(r"```(diff|patch)\n(?s)(.*?)\n```\n?").ok()?;
    let mut files: Vec<DiffFile> = Vec::new();
    for caps in re.captures_iter(text) {
        let body = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        if let Some(parsed) = parse_unified_diff(body) {
            files.extend(parsed);
        }
    }
    if files.is_empty() { return None; }
    let mut out = String::from("*** Begin Patch\n");
    for f in files {
        match f.kind.as_str() {
            "add" => {
                out.push_str(&format!("*** Add File: {}\n", f.path));
                for line in f.add_content { out.push('+'); out.push_str(&line); out.push('\n'); }
            }
            "delete" => {
                out.push_str(&format!("*** Delete File: {}\n", f.path));
            }
            _ => {
                out.push_str(&format!("*** Update File: {}\n", f.path));
                for line in f.hunks { out.push_str(&line); out.push('\n'); }
                out.push_str("*** End of File\n");
            }
        }
    }
    out.push_str("*** End Patch");
    Some(out)
}

struct DiffFile {
    path: String,
    kind: String, // "add" | "delete" | "update"
    hunks: Vec<String>,
    add_content: Vec<String>,
}

// Minimal unified diff parser: extracts per-file adds/deletes/updates
fn parse_unified_diff(input: &str) -> Option<Vec<DiffFile>> {
    let mut lines = input.lines().peekable();
    let mut files: Vec<DiffFile> = Vec::new();
    let mut cur_path: Option<String> = None;
    let mut cur_hunks: Vec<String> = Vec::new();
    let mut cur_kind: String = "update".to_string();
    let mut add_content: Vec<String> = Vec::new();
    let mut old_is_devnull = false;
    let mut new_is_devnull = false;

    while let Some(line) = lines.next() {
        if line.starts_with("diff --git ") { continue; }
        if line.starts_with("index ") { continue; }
        if line.starts_with("--- ") {
            let oldp = line[4..].trim();
            old_is_devnull = oldp == "/dev/null";
            if let Some(next) = lines.peek() {
                if next.starts_with("+++ ") {
                    let newp_raw = (&next[4..]).trim();
                    new_is_devnull = newp_raw == "/dev/null";
                    let clean = newp_raw.trim_start_matches("a/").trim_start_matches("b/");
                    cur_path = Some(clean.to_string());
                    let _ = lines.next(); // consume +++ line
                    cur_kind = if old_is_devnull && !new_is_devnull { "add".into() }
                               else if !old_is_devnull && new_is_devnull { "delete".into() }
                               else { "update".into() };
                    continue;
                }
            }
        }
        if line.starts_with("@@") {
            if cur_path.is_none() { continue; }
            cur_hunks.push(line.to_string());
            while let Some(nl) = lines.peek() {
                let s = *nl;
                if s.starts_with("@@") || s.starts_with("diff --git ") || s.starts_with("--- ") { break; }
                if s.starts_with(' ') || s.starts_with('+') || s.starts_with('-') {
                    if cur_kind == "add" && s.starts_with('+') { add_content.push(s[1..].to_string()); }
                    cur_hunks.push(s.to_string());
                }
                lines.next();
            }
            continue;
        }
        if (line.starts_with("diff --git ") || line.starts_with("--- ")) && cur_path.is_some() && (!cur_hunks.is_empty() || cur_kind != "update") {
            files.push(DiffFile { path: cur_path.take().unwrap(), kind: cur_kind.clone(), hunks: std::mem::take(&mut cur_hunks), add_content: std::mem::take(&mut add_content) });
        }
    }
    if let Some(p) = cur_path.take() {
        if !cur_hunks.is_empty() || cur_kind != "update" {
            files.push(DiffFile { path: p, kind: cur_kind, hunks: cur_hunks, add_content });
        }
    }
    if files.is_empty() { None } else { Some(files) }
}
