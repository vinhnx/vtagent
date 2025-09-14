use anyhow::{Context, Result};
use console::style;
use std::io::{self, Write};
use vtagent_core::config::loader::{ConfigManager, VTAgentConfig};
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;

// Single-agent engine (Gemini + ToolRegistry)
use vtagent_core::gemini::{Client as GeminiClient, Content, GenerateContentRequest, Part, Tool};
use vtagent_core::gemini::function_calling::{FunctionCall, FunctionResponse};
use vtagent_core::gemini::models::{SystemInstruction, ToolConfig};
use vtagent_core::prompts::read_system_prompt_from_md;
use vtagent_core::tools::{build_function_declarations, ToolRegistry};
use vtagent_core::utils::utils::summarize_workspace_languages;
use vtagent_core::llm::{factory::create_provider_for_model, provider as uni, make_client, AnyClient};
use vtagent_core::models::{ModelId, Provider};

// Multi-agent engine
use vtagent_core::core::agent::integration::MultiAgentSystem;
use vtagent_core::core::agent::multi_agent::AgentType;
use vtagent_core::config::multi_agent::MultiAgentSystemConfig;
use vtagent_core::core::router::{Router, TaskClass};
use vtagent_core::core::trajectory::TrajectoryLogger;

fn read_prompt_refiner_prompt() -> Option<String> {
    std::fs::read_to_string("prompts/prompt_refiner.md").ok()
}

async fn refine_user_prompt_if_enabled(
    raw: &str,
    cfg: &CoreAgentConfig,
    vt_cfg: Option<&VTAgentConfig>,
) -> String {
    if std::env::var("VTAGENT_PROMPT_REFINER_STUB").is_ok() {
        return format!("[REFINED] {}", raw);
    }
    let Some(vtc) = vt_cfg else { return raw.to_string() };
    if !vtc.agent.refine_prompts_enabled { return raw.to_string(); }

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
            Provider::OpenAI => vtagent_core::config::constants::models::openai::GPT_5_MINI.to_string(),
            _ => cfg.model.clone(),
        }
    };

    let Ok(mut refiner) = create_provider_for_model(&refiner_model, cfg.api_key.clone()) else {
        return raw.to_string();
    };

    let system = read_prompt_refiner_prompt().unwrap_or_else(|| "You are a prompt refiner. Return only the improved prompt.".to_string());
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

    match refiner.generate(req).await.and_then(|r| Ok(r.content.unwrap_or_default())) {
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
    println!("{}", style("Interactive chat (tools)").blue().bold());
    println!("Model: {}", config.model);
    println!("Workspace: {}", config.workspace.display());
    if let Some(summary) = summarize_workspace_languages(&config.workspace) {
        println!("Detected languages: {}", summary);
    }
    println!();

    let mut client = GeminiClient::new(config.api_key.clone(), config.model.clone());
    let traj = ConfigManager::load_from_workspace(&config.workspace)
        .ok()
        .map(|m| m.config().telemetry.trajectory_enabled)
        .map(|enabled| if enabled { TrajectoryLogger::new(&config.workspace) } else { TrajectoryLogger::disabled() })
        .unwrap_or_else(|| TrajectoryLogger::new(&config.workspace));

    let mut tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    let function_declarations = build_function_declarations();
    let tools = vec![Tool { function_declarations }];

    let mut conversation_history: Vec<Content> = vec![];
    let system_instruction = SystemInstruction::new(
        &read_system_prompt_from_md()
            .unwrap_or_else(|_| "You are a helpful coding assistant for a Rust workspace.".to_string()),
    );

    println!("{}", style("Type 'exit' to quit, 'help' for commands").dim());
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "" => continue,
            "exit" | "quit" => { println!("Goodbye!"); break; }
            "help" => { println!("Commands: exit, help"); continue; }
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
            match decision.class { TaskClass::Simple=>"simple", TaskClass::Standard=>"standard", TaskClass::Complex=>"complex", TaskClass::CodegenHeavy=>"codegen_heavy", TaskClass::RetrievalHeavy=>"retrieval_heavy" },
            decision.use_multi_agent,
            &input.chars().take(120).collect::<String>(),
        );

        // Auto handoff to multi-agent for this turn if advised
        if decision.use_multi_agent {
            if let Some(vt) = vt_cfg {
                let (orc, exec) = if vt.multi_agent.use_single_model {
                    let single = if vt.multi_agent.executor_model.is_empty() { config.model.clone() } else { vt.multi_agent.executor_model.clone() };
                    (single.clone(), single)
                } else {
                    (
                        if vt.multi_agent.orchestrator_model.is_empty() { config.model.clone() } else { vt.multi_agent.orchestrator_model.clone() },
                        if vt.multi_agent.executor_model.is_empty() { config.model.clone() } else { vt.multi_agent.executor_model.clone() },
                    )
                };
                let sys_cfg = MultiAgentSystemConfig {
                    enabled: true,
                    execution_mode: vtagent_core::config::multi_agent::ExecutionMode::Multi,
                    use_single_model: vt.multi_agent.use_single_model,
                    orchestrator_model: orc,
                    executor_model: exec,
                    max_concurrent_subagents: vt.multi_agent.max_concurrent_subagents,
                    context_sharing_enabled: vt.multi_agent.context_sharing_enabled,
                    task_timeout_seconds: vt.multi_agent.task_timeout_seconds,
                };
                let mut system = MultiAgentSystem::new(
                    sys_cfg,
                    config.api_key.clone(),
                    config.workspace.clone(),
                    Some(vt.agent.reasoning_effort.clone()),
                    config.verbose,
                ).await?;
                let refined = refine_user_prompt_if_enabled(input, config, Some(vt)).await;
                let required = classify_agent_type(&refined);
                match system.execute_task_optimized("Routed Task".to_string(), refined, required).await {
                    Ok(result) => { print_compact_summary(&result.final_summary); }
                    Err(e) => eprintln!("{} {}", style("[ERROR]").red().bold(), e),
                }
                system.shutdown().await.ok();
                continue; // skip single-agent flow for this turn
            }
        }

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
        let mut final_text: Option<String> = None;

        'outer: loop {
            loop_guard += 1;
            if loop_guard >= max_tool_loops { break 'outer; }

            // Apply budgets for Gemini-native path as generation_config
            let (gen_cfg, _parallel_any) = if let Some(vt) = vt_cfg {
                // Decide class to pick budget
                let decision = Router::route_async(vt, config, &config.api_key, input).await;
                let key = match decision.class { TaskClass::Simple=>"simple", TaskClass::Standard=>"standard", TaskClass::Complex=>"complex", TaskClass::CodegenHeavy=>"codegen_heavy", TaskClass::RetrievalHeavy=>"retrieval_heavy" };
                if let Some(b) = vt.router.budgets.get(key) {
                    let mut cfg = serde_json::json!({});
                    if let Some(mt) = b.max_tokens { cfg["maxOutputTokens"] = serde_json::json!(mt as u32); }
                    (Some(cfg), b.max_parallel_tools.unwrap_or(0) > 1)
                } else { (None, false) }
            } else { (None, false) };

            let req = GenerateContentRequest {
                contents: working_history.clone(),
                tools: Some(tools.clone()),
                tool_config: Some(ToolConfig::auto()),
                system_instruction: Some(system_instruction.clone()),
                generation_config: gen_cfg,
            };

            let response = client.generate(&req).await?;
            final_text = None;
            let mut function_calls: Vec<FunctionCall> = Vec::new();
            if let Some(candidate) = response.candidates.first() {
                for part in &candidate.content.parts {
                    match part {
                        Part::Text { text } => final_text = Some(text.clone()),
                        Part::FunctionCall { function_call } => function_calls.push(function_call.clone()),
                        _ => {}
                    }
                }
            }

            if function_calls.is_empty() {
                if let Some(text) = final_text.clone() { println!("{}", text); }
                if let Some(text) = final_text { working_history.push(Content::system_text(text)); }
                break 'outer;
            }

            for call in function_calls {
                let name = call.name.as_str();
                let args = call.args.clone();
                eprintln!("[TOOL] {} {}", name, args);
                match tool_registry.execute_tool(name, args).await {
                    Ok(tool_output) => {
                        render_tool_output(&tool_output);
                        if matches!(name, "write_file" | "edit_file" | "create_file" | "delete_file") {
                            any_write_effect = true;
                        }
                        let fr = FunctionResponse { name: call.name.clone(), response: tool_output };
                        working_history.push(Content::user_parts(vec![Part::FunctionResponse { function_response: fr }]));
                    }
                    Err(e) => {
                        let err = serde_json::json!({ "error": e.to_string() });
                        let fr = FunctionResponse { name: call.name.clone(), response: err };
                        working_history.push(Content::user_parts(vec![Part::FunctionResponse { function_response: fr }]));
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
                let claims_write = text.contains("I've updated") || text.contains("I have updated") || text.contains("updated the `");
                if claims_write && !any_write_effect {
                    println!("\nNote: The assistant mentioned edits but no write tool ran.");
                }
            }
        }
    }

    Ok(())
}

async fn run_prompt_only_loop(config: &CoreAgentConfig) -> Result<()> {
    println!("{}", style("Interactive chat (prompt-only)").blue().bold());
    println!("Model: {}", config.model);
    println!("Workspace: {}", config.workspace.display());
    println!("{}", style("Type 'exit' to quit").dim());

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
        if input.is_empty() { continue; }
        if matches!(input, "exit" | "quit") { println!("Goodbye!"); break; }

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

        let resp = client.generate(input).await?;
        println!("{}", resp.content);
    }
    Ok(())
}

pub async fn run_multi_agent_loop(config: &CoreAgentConfig, vt_cfg: &VTAgentConfig) -> Result<()> {
    println!("{}", style("Multi-agent mode enabled").green().bold());

    let (orchestrator_model, executor_model) = if vt_cfg.multi_agent.use_single_model {
        let single = if vt_cfg.multi_agent.executor_model.is_empty() { config.model.clone() } else { vt_cfg.multi_agent.executor_model.clone() };
        (single.clone(), single)
    } else {
        (
            if vt_cfg.multi_agent.orchestrator_model.is_empty() { config.model.clone() } else { vt_cfg.multi_agent.orchestrator_model.clone() },
            if vt_cfg.multi_agent.executor_model.is_empty() { config.model.clone() } else { vt_cfg.multi_agent.executor_model.clone() },
        )
    };

    let sys_cfg = MultiAgentSystemConfig {
        enabled: true,
        execution_mode: vtagent_core::config::multi_agent::ExecutionMode::Multi,
        use_single_model: vt_cfg.multi_agent.use_single_model,
        orchestrator_model,
        executor_model,
        max_concurrent_subagents: vt_cfg.multi_agent.max_concurrent_subagents,
        context_sharing_enabled: vt_cfg.multi_agent.context_sharing_enabled,
        task_timeout_seconds: vt_cfg.multi_agent.task_timeout_seconds,
    };

    let mut system = MultiAgentSystem::new(
        sys_cfg,
        config.api_key.clone(),
        config.workspace.clone(),
        Some(vt_cfg.agent.reasoning_effort.clone()),
        config.verbose,
    )
    .await
    .context("Failed to initialize multi-agent system")?;

    println!("{}", style("Type 'exit' to quit, 'help' for commands").dim());
    println!("{}", style("Use ':agent coder' or ':agent explorer' to force role.").dim());

    let mut forced_agent: Option<AgentType> = None;
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if let Some(rest) = input.strip_prefix(":agent ") {
            forced_agent = match rest.trim().to_lowercase().as_str() {
                "coder" => { println!("Forcing agent: Coder"); Some(AgentType::Coder) }
                "explorer" => { println!("Forcing agent: Explorer"); Some(AgentType::Explorer) }
                _ => forced_agent,
            };
            continue;
        }

        match input {
            "" => continue,
            "exit" | "quit" => { println!("Goodbye!"); break; }
            "help" => { println!("Multi-agent: executes tasks via orchestrator"); continue; }
            _ => {}
        }

        // Optional prompt refinement primarily for GPT‑5
        let refined = refine_user_prompt_if_enabled(input, config, Some(vt_cfg)).await;
        let required = forced_agent.unwrap_or_else(|| classify_agent_type(&refined));
        match system
            .execute_task_optimized("User Task".to_string(), refined, required)
            .await
        {
            Ok(result) => {
                print_compact_summary(&result.final_summary);
                println!("{}", style("[SUCCESS] Task completed").green().bold());
            }
            Err(e) => eprintln!("{} {}", style("[ERROR]").red().bold(), e),
        }
    }

    system.shutdown().await.ok();
    Ok(())
}

fn classify_agent_type(input: &str) -> AgentType {
    let text = input.to_lowercase();
    let coder_keywords = [
        "build", "implement", "code", "refactor", "add", "fix", "write", "create file",
        "edit file", "delete file", "apply patch", "compile", "run tests", "tests",
        "benchmark", "optimize", "patch", "apply_patch", "rename", "move file",
    ];
    if coder_keywords.iter().any(|k| text.contains(k)) {
        AgentType::Coder
    } else {
        let explorer_keywords = [
            "explain", "list", "where is", "where's", "show", "read", "search", "find",
            "why", "how does", "how do", "analyze", "explore", "describe", "what is",
            "help", "overview", "document", "docs", "spec", "path", "structure",
            "which file", "grep", "tree-sitter", "context", "summary",
        ];
        if explorer_keywords.iter().any(|k| text.contains(k)) || text.split_whitespace().count() < 3 {
            AgentType::Explorer
        } else {
            AgentType::Coder
        }
    }
}

fn render_tool_output(val: &serde_json::Value) {
    if let Some(stdout) = val.get("stdout").and_then(|v| v.as_str()) {
        if !stdout.trim().is_empty() {
            println!("{}\n{}", style("[stdout]").blue().bold(), stdout);
        }
    }
    if let Some(stderr) = val.get("stderr").and_then(|v| v.as_str()) {
        if !stderr.trim().is_empty() {
            eprintln!("{}\n{}", style("[stderr]").yellow().bold(), stderr);
        }
    }
    // Render PTY/session info if present
    if val.get("pty_enabled").and_then(|p| p.as_bool()).unwrap_or(false) {
        println!("{}", style("[PTY session output rendered]").dim());
    }
}

fn print_compact_summary(text: &str) {
    use console::style;
    const MAX_CHARS: usize = 1200;
    const HEAD_CHARS: usize = 800;
    const TAIL_CHARS: usize = 200;
    let clean = text.trim();
    if clean.chars().count() <= MAX_CHARS {
        println!("{}", clean);
        return;
    }
    let mut head = String::new();
    for (i, ch) in clean.chars().enumerate() {
        if i >= HEAD_CHARS { break; }
        head.push(ch);
    }
    let total = clean.chars().count();
    let tail_start = total.saturating_sub(TAIL_CHARS);
    let tail: String = clean.chars().skip(tail_start).collect();
    println!("{}\n…\n{}", head, tail);
    println!("{} truncated summary ({} chars).", style("[NOTE]").dim(), total);
}

// Unified single-agent tool-calling loop for OpenAI / Anthropic providers
async fn run_single_agent_loop_unified(
    config: &CoreAgentConfig,
    provider: Provider,
    vt_cfg: Option<&VTAgentConfig>,
) -> Result<()> {
    println!("{}", style("Interactive chat (tools)").blue().bold());
    println!("Model: {}", config.model);
    println!("Workspace: {}", config.workspace.display());
    if let Some(summary) = summarize_workspace_languages(&config.workspace) {
        println!("Detected languages: {}", summary);
    }
    println!("");

    // Create provider client from model + api key
    let mut provider_client = create_provider_for_model(&config.model, config.api_key.clone())
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

    // System prompt (used via system_prompt field in LLMRequest)
    let system_prompt = read_system_prompt_from_md()
        .unwrap_or_else(|_| "You are a helpful coding assistant for a Rust workspace.".to_string());

    let traj = ConfigManager::load_from_workspace(&config.workspace)
        .ok()
        .map(|m| m.config().telemetry.trajectory_enabled)
        .map(|enabled| if enabled { TrajectoryLogger::new(&config.workspace) } else { TrajectoryLogger::disabled() })
        .unwrap_or_else(|| TrajectoryLogger::new(&config.workspace));
    println!("{}", style("Type 'exit' to quit, 'help' for commands").dim());
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "" => continue,
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("Commands: exit, help");
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
        let mut final_text: Option<String> = None;

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
                match decision.class { TaskClass::Simple=>"simple", TaskClass::Standard=>"standard", TaskClass::Complex=>"complex", TaskClass::CodegenHeavy=>"codegen_heavy", TaskClass::RetrievalHeavy=>"retrieval_heavy" },
                decision.use_multi_agent,
                &input.chars().take(120).collect::<String>(),
            );

            // Auto handoff to multi-agent for this turn if advised
            if decision.use_multi_agent {
                if let Some(vt) = vt_cfg {
                    let (orc, exec) = if vt.multi_agent.use_single_model {
                        let single = if vt.multi_agent.executor_model.is_empty() { config.model.clone() } else { vt.multi_agent.executor_model.clone() };
                        (single.clone(), single)
                    } else {
                        (
                            if vt.multi_agent.orchestrator_model.is_empty() { config.model.clone() } else { vt.multi_agent.orchestrator_model.clone() },
                            if vt.multi_agent.executor_model.is_empty() { config.model.clone() } else { vt.multi_agent.executor_model.clone() },
                        )
                    };
                    let sys_cfg = MultiAgentSystemConfig {
                        enabled: true,
                        execution_mode: vtagent_core::config::multi_agent::ExecutionMode::Multi,
                        use_single_model: vt.multi_agent.use_single_model,
                        orchestrator_model: orc,
                        executor_model: exec,
                        max_concurrent_subagents: vt.multi_agent.max_concurrent_subagents,
                        context_sharing_enabled: vt.multi_agent.context_sharing_enabled,
                        task_timeout_seconds: vt.multi_agent.task_timeout_seconds,
                    };
                    let mut system = MultiAgentSystem::new(
                        sys_cfg,
                        config.api_key.clone(),
                        config.workspace.clone(),
                        Some(vt.agent.reasoning_effort.clone()),
                        config.verbose,
                    ).await?;
                    let refined = refine_user_prompt_if_enabled(input, config, Some(vt)).await;
                    let required = classify_agent_type(&refined);
                    match system.execute_task_optimized("Routed Task".to_string(), refined, required).await {
                        Ok(result) => { print_compact_summary(&result.final_summary); }
                        Err(e) => eprintln!("{} {}", style("[ERROR]").red().bold(), e),
                    }
                    system.shutdown().await.ok();
                    break 'outer; // skip single-agent inner loop this turn
                }
            }

            let active_model = decision.selected_model;
            // Apply budgets if present
            let (max_tokens_opt, parallel_cfg_opt) = if let Some(vt) = vt_cfg {
                use vtagent_core::config::router::ResourceBudget;
                let key = match decision.class { TaskClass::Simple=>"simple", TaskClass::Standard=>"standard", TaskClass::Complex=>"complex", TaskClass::CodegenHeavy=>"codegen_heavy", TaskClass::RetrievalHeavy=>"retrieval_heavy" };
                let budget = vt.router.budgets.get(key);
                let max_tokens = budget.and_then(|b| b.max_tokens).map(|n| n as u32);
                let parallel = budget
                    .and_then(|b| b.max_parallel_tools)
                    .map(|n| vtagent_core::llm::provider::ParallelToolConfig {
                        disable_parallel_tool_use: n <= 1,
                        max_parallel_tools: Some(n),
                        encourage_parallel: n > 1,
                    });
                (max_tokens, parallel)
            } else { (None, None) };
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

            let response = provider_client
                .generate(request)
                .await
                .context("Provider request failed")?;

            final_text = response.content.clone();

            // Extract tool calls if any
            if let Some(tool_calls) = response.tool_calls.as_ref() {
                if tool_calls.is_empty() {
                    // No tools requested; finish
                } else {
                    // Execute each function call
                    for call in tool_calls {
                        let name = call.function.name.as_str();
                        let args_val = call.parsed_arguments().unwrap_or_else(|_| serde_json::json!({}));
                        match tool_registry.execute_tool(name, args_val.clone()).await {
                            Ok(tool_output) => {
                                traj.log_tool_call(working_history.len(), name, &args_val, true);
                                render_tool_output(&tool_output);
                                if matches!(name, "write_file" | "edit_file" | "create_file" | "delete_file") {
                                    any_write_effect = true;
                                }
                                // Send tool response back (OpenAI expects tool role with tool_call_id)
                                let content = serde_json::to_string(&tool_output).unwrap_or("{}".to_string());
                                working_history.push(uni::Message::tool_response(call.id.clone(), content));
                            }
                            Err(e) => {
                                traj.log_tool_call(working_history.len(), name, &args_val, false);
                                let err = serde_json::json!({ "error": e.to_string() });
                                let content = err.to_string();
                                working_history.push(uni::Message::tool_response(call.id.clone(), content));
                            }
                        }
                    }
                    // Continue inner loop to let model use tool results
                    continue;
                }
            }

            // If we reach here, no (more) tool calls
            if let Some(mut text) = final_text.clone() {
                // Optional self-review/refinement pass based on config
                let do_review = vt_cfg.map(|c| c.agent.enable_self_review).unwrap_or(false);
                let review_passes = vt_cfg.map(|c| c.agent.max_review_passes).unwrap_or(1).max(1);
                if do_review {
                    let review_system = "You are the agent's critical code reviewer. Improve clarity, correctness, and add missing test or validation guidance. Return only the improved final answer (no meta commentary).".to_string();
                    for _ in 0..review_passes {
                        let review_req = uni::LLMRequest {
                            messages: vec![
                                uni::Message::user(format!(
                                    "Please review and refine the following response. Return only the improved response.\n\n{}",
                                    text
                                )),
                            ],
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
                            if !r.trim().is_empty() { text = r; }
                        }
                    }
                }
                println!("{}", text);
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
                    println!("\nNote: The assistant mentioned edits but no write tool ran.");
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
        std::env::set_var("VTAGENT_PROMPT_REFINER_STUB", "1");

        // Minimal CoreAgentConfig
        let cfg = CoreAgentConfig {
            model: vtagent_core::config::constants::models::google::GEMINI_2_5_FLASH_LITE.to_string(),
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
        std::env::remove_var("VTAGENT_PROMPT_REFINER_STUB");
    }
}
