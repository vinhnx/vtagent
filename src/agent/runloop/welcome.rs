use std::fs;
use std::path::Path;

use vtagent_core::config::core::AgentOnboardingConfig;
use vtagent_core::config::loader::VTAgentConfig;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::ui::styled::Styles;
use vtagent_core::utils::utils::{
    ProjectOverview, build_project_overview, summarize_workspace_languages,
};

#[derive(Default, Clone)]
pub(crate) struct SessionBootstrap {
    pub welcome_text: Option<String>,
    pub placeholder: Option<String>,
    pub prompt_addendum: Option<String>,
    pub language_summary: Option<String>,
}

pub(crate) fn prepare_session_bootstrap(
    runtime_cfg: &CoreAgentConfig,
    vt_cfg: Option<&VTAgentConfig>,
) -> SessionBootstrap {
    let onboarding_cfg = vt_cfg
        .map(|cfg| cfg.agent.onboarding.clone())
        .unwrap_or_default();

    let project_overview = build_project_overview(&runtime_cfg.workspace);
    let language_summary = summarize_workspace_languages(&runtime_cfg.workspace);
    let guideline_highlights = if onboarding_cfg.include_guideline_highlights {
        extract_guideline_highlights(
            &runtime_cfg.workspace,
            onboarding_cfg.guideline_highlight_limit,
        )
    } else {
        None
    };

    let welcome_text = if onboarding_cfg.enabled {
        Some(render_welcome_text(
            &onboarding_cfg,
            project_overview.as_ref(),
            language_summary.as_deref(),
            guideline_highlights.as_deref(),
        ))
    } else {
        None
    };

    let prompt_addendum = if onboarding_cfg.enabled {
        build_prompt_addendum(
            &onboarding_cfg,
            project_overview.as_ref(),
            language_summary.as_deref(),
            guideline_highlights.as_deref(),
        )
    } else {
        None
    };

    let placeholder = {
        let trimmed = onboarding_cfg.chat_placeholder.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };

    SessionBootstrap {
        welcome_text,
        placeholder,
        prompt_addendum,
        language_summary,
    }
}

fn render_welcome_text(
    onboarding_cfg: &AgentOnboardingConfig,
    overview: Option<&ProjectOverview>,
    language_summary: Option<&str>,
    guideline_highlights: Option<&[String]>,
) -> String {
    let mut lines = Vec::new();
    let intro = onboarding_cfg.intro_text.trim();
    if !intro.is_empty() {
        lines.push(intro.to_string());
    }

    if onboarding_cfg.include_project_overview {
        if let Some(project) = overview {
            let summary = project.short_for_display();
            if let Some(first_line) = summary.lines().next() {
                push_section_header(&mut lines, "Project context summary:");
                lines.push(format!("  - {}", first_line.trim()));
            }
        }
    }

    if onboarding_cfg.include_language_summary {
        if let Some(summary) = language_summary {
            push_section_header(&mut lines, "Detected stack:");
            lines.push(format!("  - {}", summary));
        }
    }

    if onboarding_cfg.include_guideline_highlights {
        if let Some(highlights) = guideline_highlights {
            if !highlights.is_empty() {
                push_section_header(&mut lines, "Key guidelines:");
                for item in highlights.iter().take(2) {
                    lines.push(format!("  - {}", item));
                }
            }
        }
    }

    lines.join("\n")
}

fn push_section_header(lines: &mut Vec<String>, header: &str) {
    if !lines.is_empty() && !lines.last().map(|line| line.is_empty()).unwrap_or(false) {
        lines.push(String::new());
    }
    let style = Styles::header();
    let styled = format!("{}{}{}", style.render(), header, style.render_reset());
    lines.push(styled);
}

fn extract_guideline_highlights(workspace: &Path, limit: usize) -> Option<Vec<String>> {
    if limit == 0 {
        return None;
    }
    let path = workspace.join("AGENTS.md");
    let content = fs::read_to_string(path).ok()?;
    let mut highlights = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- ") {
            let highlight = trimmed.trim_start_matches("- ").trim();
            if !highlight.is_empty() {
                highlights.push(highlight.to_string());
            }
        }
        if highlights.len() >= limit {
            break;
        }
    }
    if highlights.is_empty() {
        None
    } else {
        Some(highlights)
    }
}

fn build_prompt_addendum(
    onboarding_cfg: &AgentOnboardingConfig,
    overview: Option<&ProjectOverview>,
    language_summary: Option<&str>,
    guideline_highlights: Option<&[String]>,
) -> Option<String> {
    let mut lines = Vec::new();
    lines.push("## SESSION CONTEXT".to_string());

    if onboarding_cfg.include_project_overview {
        if let Some(project) = overview {
            lines.push("### Project Overview".to_string());
            let block = project.as_prompt_block();
            let trimmed = block.trim();
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        }
    }

    if onboarding_cfg.include_language_summary {
        if let Some(summary) = language_summary {
            lines.push("### Detected Languages".to_string());
            lines.push(format!("- {}", summary));
        }
    }

    if onboarding_cfg.include_guideline_highlights {
        if let Some(highlights) = guideline_highlights {
            if !highlights.is_empty() {
                lines.push("### Key Guidelines".to_string());
                for item in highlights.iter().take(2) {
                    lines.push(format!("- {}", item));
                }
            }
        }
    }

    let content = lines.join("\n");
    if content.trim() == "## SESSION CONTEXT" {
        None
    } else {
        Some(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_prepare_session_bootstrap_builds_sections() {
        let tmp = tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\ndescription = \"Demo project\"\n",
        )
        .unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(
            tmp.path().join("AGENTS.md"),
            "- Follow workspace guidelines\n- Prefer 4-space indentation\n- Run cargo fmt before commits\n",
        )
        .unwrap();
        fs::write(tmp.path().join("README.md"), "Demo workspace\n").unwrap();

        let mut vt_cfg = VTAgentConfig::default();
        vt_cfg.agent.onboarding.include_language_summary = false;
        vt_cfg.agent.onboarding.guideline_highlight_limit = 2;
        vt_cfg.agent.onboarding.usage_tips = vec!["Tip one".into()];
        vt_cfg.agent.onboarding.recommended_actions = vec!["Do something".into()];
        vt_cfg.agent.onboarding.chat_placeholder = "Type your plan".into();

        let runtime_cfg = CoreAgentConfig {
            model: vtagent_core::config::constants::models::google::GEMINI_2_5_FLASH_LITE
                .to_string(),
            api_key: "test".to_string(),
            workspace: tmp.path().to_path_buf(),
            verbose: false,
            theme: vtagent_core::ui::theme::DEFAULT_THEME_ID.to_string(),
        };

        let bootstrap = prepare_session_bootstrap(&runtime_cfg, Some(&vt_cfg));

        let welcome = bootstrap.welcome_text.expect("welcome text");
        assert!(welcome.contains("Tip one"));
        assert!(welcome.contains("Follow workspace guidelines"));

        let prompt = bootstrap.prompt_addendum.expect("prompt addendum");
        assert!(prompt.contains("## SESSION CONTEXT"));
        assert!(prompt.contains("Suggested Next Actions"));

        assert_eq!(bootstrap.placeholder.as_deref(), Some("Type your plan"));
    }
}
