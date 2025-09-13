# Anthropic Tooling Alignment Refactor

This refactor aligns VTAgent’s tools, prompts, and policies with Anthropic’s guidance for building ergonomic, reliable agent tools.

## What Changed

- Parameter schemas clarified; unambiguous names and examples embedded in error messages and specs.
- Token efficiency: response_format added (default `concise`), plus pagination for file discovery; search caps produce guidance.
- Helpful errors: actionable messages for missing/invalid args with concrete examples.
- Scoped permissions: workspace policy supports `constraints` (allowed modes, caps, defaults) enforced at runtime by `ToolRegistry`.
- Prompt updates: system/tool usage guidance steers agents to plan, paginate, and prefer concise outputs.

## Files Updated

- `vtagent-core/src/tools/types.rs`: new fields `page`, `per_page`, `response_format`.
- `vtagent-core/src/tools/file_ops.rs`: pagination, concise formatting, helpful errors.
- `vtagent-core/src/tools/search.rs`: `response_format`, concise transform, helpful errors, tests.
- `vtagent-core/src/tools/registry.rs`: function specs updated; policy constraints applied before execution.
- `vtagent-core/src/tool_policy.rs`: new `ToolConstraints` and `constraints` map with getters.
- `vtagent-core/src/prompts/templates.rs` and `prompts/system.md`: guidance for planning, pagination, concise output, and policy caps.
- `.vtagent/tool-policy.json`: added scoped `constraints` examples.
- `docs/tools/TOOL_SPECS.md`: consolidated tool specs and examples.

## Testing Strategy

- Unit tests: `transform_matches_to_concise` in search module validates concise response conversion.
- Integration tests (suggested):
  - `list_files` pagination and response_format behavior on a temp workspace.
  - Policy constraint enforcement via `ToolRegistry` with workspace `.vtagent/tool-policy.json`.
- E2E suggestions: Script to simulate real flows (search → read → edit → write) and assert guidance on truncation.

## Backward Compatibility

- Existing tool names and core behaviors preserved.
- New parameters are optional with safe defaults.
- Policy `constraints` are optional; absence preserves prior behavior.

## Rationale

These changes aim to improve reliability, reduce hallucination risk, and optimize token usage while keeping the surface area stable and focused on tools only. The refactor avoids new dependencies and large architectural changes.
