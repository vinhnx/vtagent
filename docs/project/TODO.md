implement openrouter provider

---

implement lmstudio provider

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

---

long term plan: https://agentclientprotocol.com/overview/introduction for IDE integration


---


https://github.com/openai/codex/blob/main/codex-rs/core/src/prompt_for_compact_command.md

--

https://github.com/whit3rabbit/bubbletea-rs/tree/main/examples/fullscreen

--

https://github.com/whit3rabbit/bubbletea-rs/tree/main/examples/altscreen-toggle
--

https://github.com/tbillington/kondo

--

---

implement and update case-insensitive search for file and content

---

https://ast-grep.github.io/llms-full.txt

---

study prompt

https://github.com/openai/codex/blob/main/codex-rs/core/prompt.md

--

extract the system prompt in vtagent-core/src/prompts/system.rs to put in md vtagent/prompts/
