//! VTCode - Research-preview Rust coding agent
//!
//! Thin binary entry point that delegates to modular CLI handlers.

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use colorchoice::ColorChoice as GlobalColorChoice;
use is_terminal::IsTerminal;
use rpassword::read_password;
use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};
use vtcode_core::cli::args::{Cli, Commands};
use vtcode_core::config::api_keys::{ApiKeySources, get_api_key, load_dotenv};
use vtcode_core::config::loader::ConfigManager;
use vtcode_core::config::models::Provider as ModelProvider;
use vtcode_core::config::types::AgentConfig as CoreAgentConfig;
use vtcode_core::ui::theme::{self as ui_theme, DEFAULT_THEME_ID};
use vtcode_core::{initialize_dot_folder, load_user_config, update_theme_preference};

mod agent;
mod cli; // local CLI handlers in src/cli // agent runloops (single-agent only)
mod workspace_trust;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env (non-fatal if missing)
    load_dotenv().ok();

    let args = Cli::parse();
    args.color.write_global();
    if args.no_color {
        GlobalColorChoice::Never.write_global();
    }

    // Resolve workspace (default: current dir, canonicalized when present)
    let workspace_override = args
        .workspace_path
        .clone()
        .or_else(|| args.workspace.clone());

    let workspace = resolve_workspace_path(workspace_override)
        .context("Failed to resolve workspace directory")?;

    if let Some(path) = &args.workspace_path {
        if !workspace.exists() {
            bail!(
                "Workspace path '{}' does not exist. Initialize it first or provide an existing directory.",
                path.display()
            );
        }
    }

    cli::set_workspace_env(&workspace);

    // Load configuration (vtcode.toml or defaults) from resolved workspace
    let config_manager = ConfigManager::load_from_workspace(&workspace).with_context(|| {
        format!(
            "Failed to load vtcode configuration for workspace {}",
            workspace.display()
        )
    })?;
    let cfg = config_manager.config();

    if args.full_auto {
        let automation_cfg = &cfg.automation.full_auto;
        if !automation_cfg.enabled {
            bail!(
                "Full-auto mode is disabled in configuration. Enable it under [automation.full_auto]."
            );
        }

        if automation_cfg.require_profile_ack {
            let profile_path = automation_cfg.profile_path.clone().ok_or_else(|| {
                anyhow!(
                    "Full-auto mode requires 'profile_path' in [automation.full_auto] when require_profile_ack = true."
                )
            })?;
            let resolved_profile = if profile_path.is_absolute() {
                profile_path
            } else {
                workspace.join(profile_path)
            };

            if !resolved_profile.exists() {
                bail!(
                    "Full-auto profile '{}' not found. Create the acknowledgement file before using --full-auto.",
                    resolved_profile.display()
                );
            }
        }
    }

    let skip_confirmations = args.skip_confirmations || args.full_auto;

    // Resolve provider/model/theme with CLI override
    let provider = args
        .provider
        .clone()
        .unwrap_or_else(|| cfg.agent.provider.clone());
    let model = args
        .model
        .clone()
        .unwrap_or_else(|| cfg.agent.default_model.clone());

    initialize_dot_folder().ok();
    let user_theme_pref = load_user_config().ok().and_then(|dot| {
        let trimmed = dot.preferences.theme.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let mut theme_selection = args
        .theme
        .clone()
        .or(user_theme_pref)
        .or_else(|| Some(cfg.agent.theme.clone()))
        .unwrap_or_else(|| DEFAULT_THEME_ID.to_string());

    if let Err(err) = ui_theme::set_active_theme(&theme_selection) {
        if args.theme.is_some() {
            return Err(err.context(format!("Failed to activate theme '{}'", theme_selection)));
        }
        eprintln!(
            "Warning: {}. Falling back to default theme '{}'.",
            err, DEFAULT_THEME_ID
        );
        theme_selection = DEFAULT_THEME_ID.to_string();
        ui_theme::set_active_theme(&theme_selection)
            .with_context(|| format!("Failed to activate theme '{}'", theme_selection))?;
    }

    update_theme_preference(&theme_selection).ok();

    // Resolve API key for chosen provider
    let sources = ApiKeySources::default();
    let api_key = match get_api_key(&provider, &sources) {
        Ok(key) => key,
        Err(error) => match prompt_for_api_key(&provider)? {
            Some(key) => {
                persist_api_key_env(&provider, &key);
                key
            }
            None => {
                return Err(error.context(format!("API key not found for provider '{}'", provider)));
            }
        },
    };

    // Bridge to local CLI modules
    let core_cfg = CoreAgentConfig {
        model: model.clone(),
        api_key,
        provider: provider.clone(),
        workspace: workspace.clone(),
        verbose: args.verbose,
        theme: theme_selection.clone(),
        reasoning_effort: cfg.agent.reasoning_effort,
    };

    match &args.command {
        Some(Commands::ToolPolicy { command }) => {
            vtcode_core::cli::tool_policy_commands::handle_tool_policy_command(command.clone())
                .await?;
        }
        Some(Commands::Models { command }) => {
            vtcode_core::cli::models_commands::handle_models_command(&args, command).await?;
        }
        Some(Commands::Chat) => {
            cli::handle_chat_command(&core_cfg, skip_confirmations, args.full_auto).await?;
        }
        Some(Commands::Ask { prompt }) => {
            cli::handle_ask_single_command(&core_cfg, prompt).await?;
        }
        Some(Commands::ChatVerbose) => {
            // Reuse chat path; verbose behavior is handled in the module if applicable
            cli::handle_chat_command(&core_cfg, skip_confirmations, args.full_auto).await?;
        }
        Some(Commands::Analyze) => {
            cli::handle_analyze_command(&core_cfg).await?;
        }
        Some(Commands::Performance) => {
            cli::handle_performance_command().await?;
        }
        Some(Commands::Trajectory { file, top }) => {
            cli::handle_trajectory_logs_command(&core_cfg, file.clone(), *top).await?;
        }
        Some(Commands::CreateProject { name, features }) => {
            cli::handle_create_project_command(&core_cfg, name, features).await?;
        }
        Some(Commands::CompressContext) => {
            cli::handle_compress_context_command(&core_cfg).await?;
        }
        Some(Commands::Revert { turn, partial }) => {
            cli::handle_revert_command(&core_cfg, *turn, partial.clone()).await?;
        }
        Some(Commands::Snapshots) => {
            cli::handle_snapshots_command(&core_cfg).await?;
        }
        Some(Commands::CleanupSnapshots { max }) => {
            cli::handle_cleanup_snapshots_command(&core_cfg, Some(*max)).await?;
        }
        Some(Commands::Init) => {
            cli::handle_init_command(&workspace, false, false).await?;
        }
        Some(Commands::Config { output, global }) => {
            cli::handle_config_command(output.as_deref(), *global).await?;
        }
        Some(Commands::InitProject {
            name,
            force,
            migrate,
        }) => {
            cli::handle_init_project_command(name.clone(), *force, *migrate).await?;
        }
        Some(Commands::Benchmark) => {
            cli::handle_benchmark_command().await?;
        }
        Some(Commands::Man { command, output }) => {
            cli::handle_man_command(command.clone(), output.clone()).await?;
        }
        _ => {
            // Default to chat
            cli::handle_chat_command(&core_cfg, skip_confirmations, args.full_auto).await?;
        }
    }

    Ok(())
}

fn resolve_workspace_path(workspace_arg: Option<PathBuf>) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to determine current working directory")?;

    let mut resolved = match workspace_arg {
        Some(path) if path.is_absolute() => path,
        Some(path) => cwd.join(path),
        None => cwd,
    };

    if resolved.exists() {
        resolved = resolved.canonicalize().with_context(|| {
            format!(
                "Failed to canonicalize workspace path {}",
                resolved.display()
            )
        })?;
    }

    Ok(resolved)
}

fn prompt_for_api_key(provider: &str) -> Result<Option<String>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    if !stdin.is_terminal() || !stdout.is_terminal() {
        return Ok(None);
    }

    let env_hint = provider
        .parse::<ModelProvider>()
        .ok()
        .map(|p| p.default_api_key_env().to_string());

    println!(
        "No API key found for provider '{}'. Enter it now to continue or press Enter to cancel.",
        provider
    );
    if let Some(env_name) = env_hint {
        println!(
            "Tip: set the {} environment variable or add it to your .env file to skip this prompt.",
            env_name
        );
    }
    print!("API key: ");
    stdout
        .flush()
        .context("Failed to flush stdout before reading API key")?;

    let input = read_password().context("Failed to read API key input")?;
    println!();
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    Ok(Some(trimmed.to_string()))
}

fn persist_api_key_env(provider: &str, key: &str) {
    if let Ok(provider_enum) = provider.parse::<ModelProvider>() {
        // SAFETY: setting environment variables is safe for this single-process CLI runtime.
        unsafe {
            env::set_var(provider_enum.default_api_key_env(), key);
        }
    }
}
