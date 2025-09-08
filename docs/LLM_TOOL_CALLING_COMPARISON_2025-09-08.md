# LLM tool/function calling comparison (Sep 8, 2025)

This document provides a provider-agnostic overview of tool/function calling capabilities, error handling, security, rate limits, pricing references, and recent updates for major LLM providers. It also includes best practices for robust agent loops, multi-agent orchestration, async/parallel tool execution, and fallbacks.

Note: Pricing and limits change frequently. Always verify on the linked official pages.

## Sources checked (Sep 8, 2025)

- Anthropic
  - Tool use: https://docs.anthropic.com/en/docs/build-with-claude/tool-use
  - API reference (Messages): https://docs.anthropic.com/en/api/messages
  - Rate limits overview: https://docs.anthropic.com/en/docs/build-with-claude/rate-limits
  - Pricing (incl. tools): https://www.anthropic.com/pricing
- Google Gemini (Direct API)
  - Function calling: https://ai.google.dev/gemini-api/docs/function-calling
  - Pricing: https://ai.google.dev/gemini-api/docs/pricing
  - Rate limits: https://ai.google.dev/gemini-api/docs/rate-limits
- Google Vertex AI Gemini
  - Function calling: https://cloud.google.com/vertex-ai/generative-ai/docs/multimodal/function-calling
- AWS Bedrock
  - Tool use via Converse API: https://docs.aws.amazon.com/bedrock/latest/userguide/tool-use.html
- Aggregators/Hosts for OSS models (Llama, Qwen, Mistral, etc.)
  - Fireworks function calling (OpenAI-compatible): https://docs.fireworks.ai/guides/function-calling
  - Together AI function calling: https://docs.together.ai/docs/function-calling

OpenAI official docs were intermittently blocked via our fetcher; for syntax we referenced the OpenAI Cookbook example:
- OpenAI Cookbook (function calling via Chat Completions): https://cookbook.openai.com/examples/how_to_call_functions_with_chat_models

xAI (Grok) docs were not reachable; treat details as subject to change and verify in their console/docs.

---

## Quick capability matrix (high level)

- Schema for tools/functions
  - OpenAI: JSON Schema via `tools: [{ type: "function", function: { name, description, parameters } }]`; `tool_choice` controls usage.
  - Anthropic: `tools` in Messages API; model emits `tool_use` blocks; client returns `tool_result` blocks. Sequential calls preferred.
  - Gemini API: `tools: { function_declarations: [...] }`; model returns `functionCalls` parts; supports parallel calls in a single turn.
  - Vertex AI Gemini: Similar to Gemini API with GCP auth and quotas.
  - Bedrock: Tool definitions passed to Converse API; model returns a tool request; you post back the result; consistent across supported models.
  - Fireworks/Together (Llama/Qwen/Mistral): OpenAI-compatible `tools` and `tool_calls` formats across many models.

- Built-in/server tools
  - Anthropic: Server tools include Web Search; client-implemented tools include Computer Use and Text Editor.
  - Gemini API: Built-in tools include Google Search grounding, code execution, URL context, and Live tool use.
  - OpenAI: Historically via Assistants (e.g., code interpreter, file search, browsing); verify current availability in OpenAI docs/console.
  - Bedrock: No provider-wide built-ins; depends on underlying model; Converse API normalizes the pattern.
  - Aggregators: Typically do not provide server tools; some may add integrations.

- Parallel tool calls
  - Gemini: Yes (multiple function calls in one turn).
  - OpenAI: Yes (`tool_calls` array in Chat Completions; execute in parallel when independent).
  - Anthropic: Generally one at a time; chain over turns with `tool_use` → `tool_result`.
  - Bedrock: Pattern supports multiple tools across turns; parallelism is client-managed.
  - Aggregators: Depends on model; OpenAI-compatible models usually support multiple calls.

- Streaming
  - All support streaming of text and, in many cases, tool call metadata; details vary (event streams vs SSE vs SDK abstractions).

- Error handling and retries
  - Rate-limit status codes and `Retry-After`/backoff semantics present across providers. Use exponential backoff with jitter.
  - Validate tool arguments against JSON Schema before executing, return structured errors back to the model when appropriate.

- Security & privacy
  - Auth via API keys (Gemini direct/Vertex AI use Google Cloud auth; Bedrock via AWS auth).
  - Sandboxing: Anthropic provides hosted code execution; otherwise you must sandbox tools (containers, timeouts, allowlists).
  - Data handling and compliance: Check provider-specific data retention and enterprise controls.

- Pricing & limits
  - Anthropic: Per MTok input/output; extra charges for Web Search and Code Execution (see pricing page).
  - Gemini: Per 1M tokens with model-specific rates; Batch mode at 50% price; grounding surcharge after free tier.
  - OpenAI: Refer to OpenAI pricing page in console/docs; varies by model and feature.
  - Bedrock: Pricing per underlying model/provider terms.
  - Aggregators: Per provider/model; check model card.

---

## Provider specifics (concise)

### OpenAI (Chat Completions / Responses / Assistants)

- Tool syntax (Chat Completions):
  - `tools: [{ type: "function", function: { name, description, parameters (JSON Schema) } }]`
  - `tool_choice`: `"auto" | "none" | { type: "function", function: { name } }`
  - Model output includes `tool_calls[]` with `{ id, type: "function", function: { name, arguments (JSON string) } }`
- Notes
  - Execute tools client-side; append tool results as messages for the next turn.
  - Streaming provides incremental deltas and tool call events.
  - Rate limits: 429 with `Retry-After`; consider token RPM/TPM budgeting.
  - Pricing/limits: Check OpenAI pricing and rate-limits pages for current values.

References: OpenAI Cookbook example (above). Official API docs recommended via platform portal.

### Anthropic (Claude Messages API)

- Tool syntax
  - Provide `tools` in request; model emits `tool_use` content blocks.
  - Client executes tool and returns `tool_result` with matching id.
  - Supports client tools (custom/Anthropic-defined like Computer Use/Text Editor) and server tools (Web Search).
- Behavior
  - Multi-step sequences across turns; best used sequentially (one tool at a time), based on docs guidance.
- Limits & pricing
  - Service tiers: priority/standard/batch; batch saves ~50%.
  - Web Search priced per 1K searches; Code Execution priced per container-hour after daily free allowance; plus token costs.

Refs: Tool use guide, Messages API reference, rate limits, pricing pages (links above).

### Google Gemini API

- Tool syntax
  - `tools: { function_declarations: [{ name, description, parameters (JSON Schema) }, ...] }`
  - Model may return multiple `functionCalls` in a single turn; parallel execution is supported client-side.
- Built-in tools
  - Google Search grounding, code execution, URL context; Live API supports tool use in interactive sessions.
- Pricing & limits
  - Pricing per 1M tokens with separate input/output rates; batch mode is ~50% of interactive.
  - Grounding has free daily quota, then per-request fee.
  - Rate limits vary by project tier (RPM, TPM, RPD), with upgrade paths based on spend.

Refs: Function calling, pricing, and rate-limits pages (links above).

### Google Vertex AI (Gemini on GCP)

- Similar function-calling semantics, with GCP auth/quotas and enterprise controls.
- Use for production workloads needing GCP integration, governance, and billing consolidation.

Ref: Vertex AI function calling docs (link above).

### AWS Bedrock (Converse API)

- Tool use pattern
  - Define tools for the conversation; model returns tool request; you call the tool and post results back via Converse.
  - Works across multiple foundation models; Converse normalizes the interface.
- Good fit for AWS-native deployments and multi-model orchestration under one API.

Ref: Bedrock tool use docs (link above).

### Aggregators/Hosts for OSS models (Llama, Qwen, Mistral)

- Fireworks, Together, and others provide OpenAI-compatible `tools`/`tool_calls` across many models (e.g., Llama 3.x/4 variants, Qwen 2.5, Mistral).
- Check each model’s card for `supportsTools`/capabilities and context length.

Refs: Fireworks and Together docs (links above).

### xAI (Grok)

- Public documentation access may vary; verify current tool/function calling and limits in xAI’s official docs/console.

---

## Agent loop patterns and safeguards

- Contract
  - Input: message history, tool catalog (JSON Schema), policy context (limits, safety), optional memory/state.
  - Output: either final text/structured result or tool invocation(s) with validated args.
  - Error modes: invalid args, tool failure, timeouts, rate limits, hallucinated tools.
- Loop structure
  1) Plan: ask model to propose tool steps with rationale (hidden) and minimal args.
  2) Act: execute tool(s) with strict validation and allowlist; sanitize inputs.
  3) Observe: append results; include concise, structured summaries to reduce tokens.
  4) Stop when success criteria met or max turns/time/token budget reached.
- Safeguards
  - Max turns and token budget per task; kill switches.
  - JSON Schema validation; coerce types; reject on mismatch and ask model to correct.
  - Timeouts, retries with backoff; circuit breakers for flaky tools.
  - Clear tool allowlist and scoping; redact secrets; sandbox code; network egress policy.

## Async, parallel, and fallback

- Async
  - Use concurrent execution for independent tool calls; aggregate results and provide a single `tool_result` turn.
  - For providers that emit multiple calls per turn (OpenAI/Gemini), execute in parallel; for sequential-only providers (Anthropic guidance), split across turns.
- Parallel
  - Batch requests cautiously to respect TPM/RPM/RPD; implement per-provider rate-limiters.
- Fallbacks
  - Model-level: try primary model; on catastrophic errors/timeouts, fallback to a cheaper/robust model.
  - Provider-level: use an adapter layer to switch providers without changing business logic.
  - Logic-level: if function calling fails repeatedly, degrade to summarization or user handoff.

## Multi-agent orchestration

- Patterns
  - Coordinator-worker (router delegates to specialists), debate/consensus, reviewer/executor, sandboxed tool runner.
- Frameworks
  - LangChain Agents/Tools, AutoGen (conversational agents), or light custom orchestration.
- Best practices
  - Keep messages concise; share only necessary context.
  - Add role prompts and guardrails per agent; bound turn counts.
  - Emit trace spans and metrics per agent/tool for observability.

## Vendor-neutral adapter design (sketch)

- Concepts
  - Tool spec (JSON Schema) is the lingua franca. Normalize to an internal schema and map to provider shapes.
  - Message abstraction: role/content with provider-specific tool/result parts.
  - Adapter per provider handling: request building, streaming, tool-call extraction, error mapping, and retries.
- Minimal interface
  - createCompletion(request): stream/text
  - extractToolCalls(response): ToolCall[]
  - sendToolResults(calls, results): response
  - errors: Unified error types (RateLimited, InvalidRequest, ServerOverloaded, TransientNetwork)

## Testing & validation

- Unit-test schema validation and argument coercion.
- Golden tests per provider adapter for request/response transforms.
- Chaos tests for rate limits and transient faults.
- Record/replay against sandboxes where available.

## Practical tips

- Use JSON Schema `enum`, `oneOf`, and string formats to constrain arguments.
- Favor structured outputs for final answers when supported.
- Cache immutable tool results; summarize verbose results before feeding back.
- Log tool inputs/outputs with PII redaction; attach correlation IDs.

## What changed recently (high level)

- Gemini: Expanded 2.5 models, Live API tool use, grounding quotas/pricing, batch mode discounts.
- Anthropic: Server tools (web search) pricing published; Messages API tool use patterns clarified; batch tier emphasized.
- Bedrock: Converse API standardizes multi-model tool use.
- OSS hosts: Broad OpenAI-compatible tool calling across Llama/Qwen/Mistral.
- OpenAI: Function calling remains centered on JSON Schema; check platform docs for current Responses/Assistants features, models, and built-in tools.

---

## Appendix: request/response shapes (abbreviated)

- OpenAI-like (Chat Completions)
  - Request: `{ model, messages, tools: [{ type: "function", function: { name, description, parameters } }], tool_choice }`
  - Response: `{ choices: [{ message: { role: "assistant", content, tool_calls: [{ id, type: "function", function: { name, arguments } }] }, finish_reason }] }`
- Anthropic (Messages)
  - Request includes `tools`; response includes `content` with `type: "tool_use"` blocks.
  - Send `tool_result` with matching id in next turn.
- Gemini API
  - Request contains `tools.function_declarations`; response includes `functionCalls` parts; send `functionResponses` back, then final.
- Bedrock Converse
  - Request with tool definitions; response indicates tool to call; send results back; continue.

Always refer to official docs for full fields/options.

---

Prepared for: vinhnx / vtagent
Last updated: 2025-09-08
