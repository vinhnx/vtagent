reference this as roadmap for future improvements

https://x.com/iannuttall/status/1964976282237649041
k Claude Code could do to win back people switching to Codex CLI:

- open source Claude Code
- reduce sycophancy/make it less verbose (or add option for that)
- more transparency about how/why the model degrades
- fix tui flashing bug! PLEASE
- improve model hallucinations like GPT-5 has
- better thinking for removing files/lines of code to prevent accidental deletions
- less boilerplate or pseudo implementations (break it into working chunks if needed)
- ability to change/remove/reduce the system reminder prompts
- file based session auto-compact with much more detail on the conversation for future reference

what would you want to see improve to make CC work better for you?
--
GPT 5 only
If you want better results with GPT-5, use AI to rewrite your human-written prompt before providing it to GPT-5. Provide your prompt-writing agent with a link to https://cookbook.openai.com/examples/gpt-5/gpt-5_prompting_guide for even better results.

--

review and reduce number of tools in vtagent-core/src/tools.rs

--

double check prompts/codex_tool_recommendations.md

--

double check prompts/system_codex_enhanced.md

--

implement openrouter provider

---

implement lmstudio provider

--

context compression for long context window task

The context window has overflowed, summarizing the history...

--

https://github.com/openai/codex/blob/main/codex-rs/core/src/prompt_for_compact_command.md

---

implement ollama provider

--
<https://github.com/laude-institute/terminal-bench?tab=readme-ov-file#submit-to-our-leaderboard>

run some lt-bench agent benchmark to test agent capability. then update the the report in readme. checking for existing benchs

---

<https://app.primeintellect.ai/dashboard/environments>



---

-   [ ] Update documentation and README.md to reflect all recent changes, including new features, configuration options, and usage instructions.
-   [ ] Add a comprehensive usage guide to the README.md, covering setup, available commands, configuration via AGENTS.md, and example workflows.
-   [ ] Ensure all documented commands and options match the current implementation.
-   [ ] Review and update any outdated instructions or references in both documentation and README.md.

-

implement prompt caching to save token cost with context engineering. use mcp for agent provider agnostic (gemini, anthropic, openai)
prompt caching guide and apply to our system

--

streaming

---
Ensure that the event handling system in the agent loop properly captures, processes, and responds to all relevant events (such as user inputs, system triggers, or external signals) without conflicts or delays, while implementing a robust turn-based management structure that enforces sequential execution of agent actions, maintains state consistency across turns, handles interruptions gracefully, and includes error recovery mechanisms for seamless operation in multi-agent or interactive environments.
--


research claude code and apply
https://claudelog.com/

--

markdown render

---

long term plan: https://agentclientprotocol.com/overview/introduction for IDE integration

---

implement and update case-insensitive search tools for file and content

---

study prompt

IMPORTANT: apply to vtagent https://github.com/openai/codex/blob/main/codex-rs/core/prompt.md

---

https://ai.google.dev/gemma/docs/embeddinggemma/inference-embeddinggemma-with-sentence-transformers


--

implement dot folder config/cache like in user home
/Users/vinh.nguyenxuan/.claude/projects

check existing config and cache and move there

---

also implement model switch config on tui and cli at vtagent-core/src/models.rs

--

https://github.com/orhun/git-cliff

---
