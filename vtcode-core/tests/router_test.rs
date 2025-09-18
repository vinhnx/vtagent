use vtcode_core::config::loader::VTCodeConfig;
use vtcode_core::config::types::AgentConfig as CoreAgentConfig;
use vtcode_core::core::router::{Router, TaskClass};

fn core_cfg(model: &str) -> CoreAgentConfig {
    CoreAgentConfig {
        model: model.to_string(),
        api_key: "test".to_string(),
        workspace: std::env::current_dir().unwrap(),
        verbose: false,
        theme: vtcode_core::ui::theme::DEFAULT_THEME_ID.to_string(),
    }
}

#[test]
fn classify_simple_and_codegen() {
    assert_eq!(Router::classify_heuristic("list files"), TaskClass::Simple);
    assert_eq!(
        Router::classify_heuristic(
            "```
fn main() {}
```"
        ),
        TaskClass::CodegenHeavy
    );
}

#[test]
fn route_uses_model_mapping() {
    let mut cfg = VTCodeConfig::default();
    cfg.router.enabled = true;
    cfg.router.models.standard = "gemini-2.5-flash-lite-preview-06-17".to_string();
    cfg.router.models.codegen_heavy = "gemini-2.5-pro".to_string();

    let core = core_cfg("gemini-2.5-flash-lite-preview-06-17");
    let r1 = Router::route(&cfg, &core, "summarize this text");
    assert_eq!(r1.selected_model, "gemini-2.5-flash-lite-preview-06-17");

    let r2 = Router::route(&cfg, &core, "Provide a patch:\n```diff\n- a\n+ b\n```\n");
    assert_eq!(r2.selected_model, "gemini-2.5-pro");
}
