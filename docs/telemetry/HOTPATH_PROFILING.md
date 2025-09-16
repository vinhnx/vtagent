# Hotpath Profiling Integration

VTAgent includes first-class support for the [`hotpath`](https://crates.io/crates/hotpath) profiler to
identify the most time-consuming operations across the agent runtime. The integration is fully
feature-gated so profiling code introduces zero overhead unless explicitly enabled.

## Enabling the Profiler

1. Build or run VTAgent with the profiling feature enabled:

    ```bash
    cargo run --features hotpath
    ```

    The optional allocation-tracking modes from Hotpath are also available. For example, to
    capture peak allocation sizes:

    ```bash
    cargo run --features "hotpath,hotpath-alloc-bytes-max"
    ```

2. Configure runtime behaviour through `vtagent.toml`:

    ```toml
    [telemetry.hotpath]
    enabled = false         # set to true to enable profiling by default
    percentiles = [95, 99]  # percentiles reported in the summary output
    report_format = "table" # "table", "json", or "json_pretty"
    ```

3. Override configuration dynamically with environment variables:

    | Variable | Description |
    | --- | --- |
    | `VTAGENT_HOTPATH_ENABLED` | Accepts `true/false/1/0` to toggle profiling at runtime. |
    | `VTAGENT_HOTPATH_PERCENTILES` | Comma-separated list of percentiles (e.g. `90,99`). |
    | `VTAGENT_HOTPATH_FORMAT` | Output format (`table`, `json`, or `json-pretty`). |

   Environment overrides take precedence over the TOML configuration, allowing ad-hoc profiling
   sessions without editing configuration files.

## Instrumented Scopes

The following execution paths are wrapped with Hotpath scopes to provide detailed timing insights:

- `telemetry.hotpath.main_startup` – overall CLI startup and shutdown lifecycle.
- `telemetry.hotpath.agent_loop` – Gemini-based interactive agent loop.
- `telemetry.hotpath.unified_agent_loop` – OpenAI/Anthropic tool loop path.
- `telemetry.hotpath.prompt_refinement` – prompt refinement pipeline.
- `telemetry.hotpath.gemini_generate` – Gemini content generation API calls.
- `telemetry.hotpath.unified_generate` – provider-neutral LLM calls (OpenAI/Anthropic).
- `telemetry.hotpath.single_prompt_generate` – `vtagent ask` single-shot queries.
- `telemetry.hotpath.gemini_context_trim` / `telemetry.hotpath.unified_context_trim` – context trimming heuristics.
- `telemetry.hotpath.gemini_tool_prune` / `telemetry.hotpath.unified_tool_prune` – removal of stale tool traces.
- `telemetry.hotpath.tool_execution` – full tool execution lifecycle in the registry.
- `telemetry.hotpath.tool_policy_constraints` – tool policy enforcement and argument capping.
- `telemetry.hotpath.tool_output_normalization` – normalisation of tool responses for CLI display.

## Viewing Reports

Hotpath prints a summary report automatically when VTAgent exits. The default table view includes
call counts, average latency, percentile breakdowns, and total time share for each instrumented
scope. Switch to JSON output via either the TOML configuration or
`VTAGENT_HOTPATH_FORMAT=json_pretty` for easier automated analysis.

## Safety Notes

- Profiling is disabled unless the `hotpath` Cargo feature is enabled.
- All instrumentation respects runtime toggles, allowing lightweight binaries in production
  while enabling deep diagnostics locally.
- Allocation-tracking features require running Tokio in `current_thread` mode; VTAgent
  automatically adopts this when the relevant feature flags are enabled.
