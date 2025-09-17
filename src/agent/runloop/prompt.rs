use vtagent_core::config::loader::VTAgentConfig;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::llm::{factory::create_provider_for_model, provider as uni};
use vtagent_core::models::{ModelId, Provider};

fn read_prompt_refiner_prompt() -> Option<String> {
    std::fs::read_to_string("prompts/prompt_refiner.md").ok()
}

pub(crate) async fn refine_user_prompt_if_enabled(
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

    let model_provider = cfg
        .model
        .parse::<ModelId>()
        .ok()
        .map(|model| model.provider())
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
        .map(|response| response.content.unwrap_or_default())
    {
        Ok(text) if !text.trim().is_empty() => text,
        _ => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prompt_refinement_applies_to_gemini_when_flag_disabled() {
        unsafe {
            std::env::set_var("VTAGENT_PROMPT_REFINER_STUB", "1");
        }

        let cfg = CoreAgentConfig {
            model: vtagent_core::config::constants::models::google::GEMINI_2_5_FLASH_LITE
                .to_string(),
            api_key: "test".to_string(),
            workspace: std::env::current_dir().unwrap(),
            verbose: false,
        };

        let mut vt = VTAgentConfig::default();
        vt.agent.refine_prompts_enabled = true;

        let raw = "make me a list of files";
        let out = refine_user_prompt_if_enabled(raw, &cfg, Some(&vt)).await;

        assert!(out.starts_with("[REFINED] "));

        unsafe {
            std::env::remove_var("VTAGENT_PROMPT_REFINER_STUB");
        }
    }
}
