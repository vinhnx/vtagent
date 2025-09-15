use serde_json::json;
use std::fs;
use tempfile::tempdir;
use vtagent_core::ToolRegistry;
use vtagent_core::config::constants::tools;

#[tokio::test]
async fn list_files_pagination_and_default_response_format() {
    let dir = tempdir().unwrap();
    let ws = dir.path().to_path_buf();

    // Create workspace files
    fs::create_dir_all(ws.join("src")).unwrap();
    fs::write(ws.join("src/a.rs"), "fn a() {}\n").unwrap();
    fs::write(ws.join("src/b.rs"), "fn b() {}\n").unwrap();

    // Workspace policy with constraints
    let vtagent_dir = ws.join(".vtagent");
    fs::create_dir_all(&vtagent_dir).unwrap();
    fs::write(
        vtagent_dir.join("tool-policy.json"),
        json!({
            "version": 1,
            "available_tools": [tools::LIST_FILES],
            "policies": { tools::LIST_FILES: "allow" },
            "constraints": { tools::LIST_FILES: { "max_items_per_call": 10, "default_response_format": "concise" } }
        }).to_string(),
    ).unwrap();

    let mut registry = ToolRegistry::new(ws.clone());
    let out = registry
        .execute_tool(
            tools::LIST_FILES,
            json!({
                "path": "src",
                "page": 1,
                "per_page": 1
            }),
        )
        .await
        .unwrap();

    assert_eq!(out["response_format"], "concise");
    assert_eq!(out["page"], 1);
    assert_eq!(out["per_page"], 1);
}

#[tokio::test]
#[ignore]
async fn grep_search_default_concise_and_cap() {
    // Skip if ripgrep is not available
    if std::process::Command::new("rg")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping grep_search_default_concise_and_cap: ripgrep not installed");
        return;
    }
    let dir = tempdir().unwrap();
    let ws = dir.path().to_path_buf();
    fs::write(ws.join("file.txt"), "TODO: one\nTODO: two\n").unwrap();

    // Minimal policy that allows grep and caps results
    let vtagent_dir = ws.join(".vtagent");
    fs::create_dir_all(&vtagent_dir).unwrap();
    fs::write(
        vtagent_dir.join("tool-policy.json"),
        json!({
            "version": 1,
            "available_tools": [tools::GREP_SEARCH],
            "policies": { tools::GREP_SEARCH: "allow" },
            "constraints": { tools::GREP_SEARCH: { "max_results_per_call": 1, "default_response_format": "concise" } }
        }).to_string(),
    ).unwrap();

    let mut registry = ToolRegistry::new(ws.clone());
    let out = registry
        .execute_tool(
            tools::GREP_SEARCH,
            json!({
                "pattern": "TODO",
                "path": ".",
                "max_results": 1000
            }),
        )
        .await
        .unwrap();

    // Defaulted to concise
    assert_eq!(out["response_format"], "concise");
    // Cap applied and guidance may be present
    assert!(out["matches"].as_array().unwrap().len() <= 1);
}
