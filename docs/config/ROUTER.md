# Router Configuration

The Router enables dynamic model and engine selection based on task complexity, aligning with agentic best practices (routing, resource awareness, and prioritization).

- Config: `vtcode.toml` → `[router]` section.
- No hardcoded model IDs: values should match `vtcode-core/src/config/constants.rs` and the authoritative `docs/models.json`.

## Keys

- `enabled`: Turn routing on/off (default: true).
- `heuristic_classification`: Use lightweight heuristics to classify prompts without an extra LLM call (default: true).
- `llm_router_model`: Optional model to perform an LLM-based routing step (currently unused when `heuristic_classification = true`).

### `[router.models]`
Map complexity → model. Recommended defaults:

- `simple` → `gemini-2.5-flash-lite`
- `standard` → `gemini-2.5-flash`
- `complex` → `gemini-2.5-pro`
- `codegen_heavy` → fast model unless accuracy demands `pro`
- `retrieval_heavy` → fast model

### `[router.budgets]` (optional)
Per‑class resource hints used by future tuning (max tokens, parallel tool limits, latency targets).

## Behavior

- Heuristics detect code fences/patches, retrieval keywords, prompt length, and complex phrasing to pick a class.
- Multi‑agent routing is deprecated; the threshold is ignored. Routing still selects the model per turn for the single‑agent loop.
- Single‑agent tool chat uses the selected model per turn. If the selected model changes provider, the client is recreated.

## Tests

See `vtcode-core/tests/router_test.rs` for classification and mapping tests. Run with `cargo nextest run`.
