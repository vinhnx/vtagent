# DSPy-Style Prompt Optimizer

This project integrates a DSPy-style prompt optimization flow for transforming vague user prompts into structured, context-grounded instructions that fit the VTAgent codebase.

- Design mirrors DSPy (Signature → Program → Compile → Infer)
- No model IDs hardcoded; read from config/constants and `vtagent.toml`
- Default path is deterministic and offline; optional DSRs hooks are feature-gated

## Usage

- The optimizer is used automatically when `agent.prompt_optimizer_enabled = true` in `vtagent.toml`.
- Adjust aggressiveness via `agent.prompt_optimizer_level = "light" | "standard" | "aggressive"`.

## DSRs (dspy-rs) Integration

The core implements feature hooks for a future/optional full DSPy pipeline using the DSRs `dspy-rs` crate.

1) The optimizer backend is controlled by `agent.prompt_optimizer_backend`:
   - `dspy` (default): use DSPy backend if available, else fall back to heuristic.
   - `heuristic`: force deterministic compiler.

2) To wire the real DSRs engine, add the dependency in the workspace:

```toml
# Top-level Cargo.toml
[workspace.dependencies]
dspy-rs = { git = "https://github.com/krypticmouse/DSRs", package = "dspy-rs" }
```

3) Build with `--features vtagent-core/dspy` (or add `dspy` to default features in `vtagent-core`).

Notes:
- We intentionally avoid adding a hard dependency here to preserve offline builds and keep the default path deterministic. When the feature is enabled, the optimizer exposes hook points to compile and run a program with dspy-rs teleprompters.
 - When `dspy` feature is active, the optimizer's `compile()` will run a real DSPy program (e.g., Bootstrap/Teleprompter) and use its compiled prompts for inference.

## Retrieval Augmentation (RAG-lite)

Set in `vtagent.toml` under `[agent]`:
- `optimizer_retrieval_enabled = true`
- `optimizer_retrieval_max_bytes = 12288`

The optimizer ranks candidate files by filename match and appends small snippets under `[Retrieved Context]` to ground the model in project code and docs.

## Output Structure

The optimized prompt includes clearly separated sections:
- `[Task]`: concise restatement of the intent (fix/refactor/add/other)
- `[User Prompt]`: original user text
- `[Suspected Scope]`: likely files inferred from the prompt and file list
- `[Model Hints]`: provider/model hints from configuration
- `[Project Policy]`: key rules pulled from `.ruler/AGENTS.md` and related docs
- `[Plan]` and `[Checklist]` (for `standard`/`aggressive` levels)
- `[Deliverables]`: patch instructions and expectations

## Conventions Enforced

- No hardcoded constants; use `vtagent-core/src/config/constants.rs`
- All documentation belongs in `./docs/`
- Use `apply_patch` for changes; minimal, focused diffs
- Prefer `cargo check`, `cargo clippy`, `cargo fmt` in validation
