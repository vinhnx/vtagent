use vtagent_core::config::loader::VTAgentConfig;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::core::router::{Router, TaskClass};

fn core_cfg(model: &str) -> CoreAgentConfig {
    CoreAgentConfig {
        model: model.to_string(),
        api_key: "test".to_string(),
        workspace: std::env::current_dir().unwrap(),
        verbose: false,
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
    let mut cfg = VTAgentConfig::default();
    cfg.router.enabled = true;
    cfg.router.models.standard = "gemini-2.5-flash-lite".to_string();
    cfg.router.models.codegen_heavy = "gemini-2.5-pro".to_string();

    let core = core_cfg("gemini-2.5-flash-lite");
    let r1 = Router::route(&cfg, &core, "summarize this text");
    assert_eq!(r1.selected_model, "gemini-2.5-flash-lite");

    let r2 = Router::route(&cfg, &core, "Provide a patch:\n```diff\n- a\n+ b\n```\n");
    assert_eq!(r2.selected_model, "gemini-2.5-pro");
}
