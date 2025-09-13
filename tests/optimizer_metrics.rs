use vtagent_core::core::prompt_optimizer::PromptRefiner;
use walkdir::WalkDir;

fn score_output(s: &str) -> usize {
    let mut score = 0;
    for tag in [
        "[Task]",
        "[User Prompt]",
        "[Project Policy]",
        "[Model Hints]",
        "[Deliverables]",
        "[Plan]",
    ] {
        if s.contains(tag) {
            score += 1;
        }
    }
    score
}

#[tokio::test]
async fn optimizer_improves_structure_score() {
    let files: Vec<String> = WalkDir::new(".")
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().display().to_string())
        .collect();

    let raw = "fix bug in chat.rs";
    let base_score = score_output(raw);
    let out = PromptRefiner::new("standard")
        .optimize(raw, &files, "single")
        .await
        .expect("optimize");
    let opt_score = score_output(&out);
    assert!(
        opt_score > base_score,
        "optimized structure score should be higher"
    );
}
