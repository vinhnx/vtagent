use std::fs;

#[test]
fn test_system_prompt_documentation_exists() {
    let system_md = fs::read_to_string("prompts/system.md").expect("system.md should exist");

    // Verify key sections exist
    assert!(system_md.contains("## Main System Prompt"));;
    assert!(system_md.contains("## Configuration Integration"));
    assert!(system_md.contains("AVAILABLE TOOLS"));
    assert!(system_md.contains("AST-Grep Power Tools"));
}

#[test]
fn test_codex_alignment_analysis_exists() {
    let analysis = fs::read_to_string("prompts/codex_alignment_analysis.md")
        .expect("codex_alignment_analysis.md should exist");

    // Verify analysis sections
    assert!(analysis.contains("## Alignment Assessment"));
    assert!(analysis.contains("### Strong Alignments"));
    assert!(analysis.contains("### VTAgent Innovations Beyond Codex"));
    assert!(analysis.contains("## Recommended Enhancements"));
}

#[test]
fn test_system_prompt_content_accuracy() {
    let system_md = fs::read_to_string("prompts/system.md").expect("system.md should exist");

    // Verify extracted prompt contains key VTAgent features
    assert!(system_md.contains("ast_grep_search"));
    assert!(system_md.contains("batch_file_operations"));
    assert!(system_md.contains("run_pty_cmd"));
    assert!(system_md.contains("vtagent.toml"));
    assert!(system_md.contains("CODE QUALITY & MAINTAINABILITY PRINCIPLES"));
}
