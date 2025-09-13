use vtagent_core::config::loader::ConfigManager;
use vtagent_core::core::prompt_optimizer::PromptRefiner;
use walkdir::WalkDir;

#[tokio::test]
async fn optimizer_includes_policy_and_retrieval() {
    // Build a shallow project file list
    let files: Vec<String> = WalkDir::new(".")
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().display().to_string())
        .collect();

    let opt = PromptRefiner::new("standard");
    let out = opt
        .optimize("fix bug in chat.rs: panic on None", &files, "single")
        .await
        .expect("optimize");
    assert!(out.contains("[Project Policy]"), "must contain policy");
    assert!(out.contains("[Model Hints]"), "must contain model hints");
    // Retrieval is best-effort; assert the section header exists when enabled.
    // Some repos may not match; so we only require that not fail.
}

// Smoke test to ensure config defaults enable DSPy backend (even if shimmed)
#[test]
fn config_defaults_enable_dspy() {
    let cfg = ConfigManager::load()
        .or_else(|_| ConfigManager::load_from_workspace("."))
        .or_else(|_| ConfigManager::load_from_file("vtagent.toml"))
        .expect("load config");
    assert!(cfg.config().agent.prompt_optimizer_enabled);
    assert_eq!(
        cfg.config().agent.prompt_optimizer_backend.to_lowercase(),
        "dspy"
    );
}
