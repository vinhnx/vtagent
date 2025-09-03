Refactor and extract the core logic from #file:main.rs, #file:main_tui.rs, and #file:tea_chat.rs into small, modular pieces of work. The main core has become too large. Run jscpd to detect duplicated code and fix any identified duplications. Ensure the refactored modules are well-organized, maintainable, and follow best practices for code separation.

---
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

---

<https://cognition.ai/blog/dont-build-multi-agents>

---

research claude code and apply
https://claudelog.com/

---

codex
https://github.com/openai/codex/tree/main/codex-rs

---

https://github.com/replit/ruspty

---

https://devin.ai/agents101#introduction


----

https://gerred.github.io/building-an-agentic-system/index.html

---

cfonts "VT Code"


 ██╗   ██╗ ████████╗  █████╗   ██████╗  ███████╗ ███╗   ██╗ ████████╗
 ██║   ██║ ╚══██╔══╝ ██╔══██╗ ██╔════╝  ██╔════╝ ████╗  ██║ ╚══██╔══╝
 ██║   ██║    ██║    ███████║ ██║  ███╗ █████╗   ██╔██╗ ██║    ██║
 ╚██╗ ██╔╝    ██║    ██╔══██║ ██║   ██║ ██╔══╝   ██║╚██╗██║    ██║
  ╚████╔╝     ██║    ██║  ██║ ╚██████╔╝ ███████╗ ██║ ╚████║    ██║
   ╚═══╝      ╚═╝    ╚═╝  ╚═╝  ╚═════╝  ╚══════╝ ╚═╝  ╚═══╝    ╚═╝


revamp welcome message

---

https://raw.githubusercontent.com/google-gemini/gemini-cli/main/docs/tools/file-system.md
https://raw.githubusercontent.com/google-gemini/gemini-cli/main/docs/tools/file-system.md


---

Failed to initialize indexer: error returned from database: (code: 1) unrecognized token: "#"
Note: Indexing will be disabled for this session

---

streaming

---

markdown render

---

event handling and turn based management for agent loop

---

https://github.com/rust-cli/roff-rs

---

https://github.com/rust-cli/env_logger

---

https://github.com/rust-cli/confy
---
https://github.com/zhiburt/tabled

---

https://github.com/zhiburt/ansi-str

https://github.com/zhiburt/ansi-str/tree/master/examples

----

https://github.com/Danau5tin/multi-agent-coding-system

----

https://github.com/Danau5tin/multi-agent-coding-system/blob/main/PROJECT_STRUCTURE.md

---

long term plan: https://agentclientprotocol.com/overview/introduction for IDE integration


----

implement edit_tools if not exist in config, add to config with prompt.

check if old_string and new_string are the same, if so, skip edit

--

extract vtagent-core/src/prompts/system.rs

to .md for human readable. let the vtagent read from .md system prompt. then we can edit the .md directly

---

https://ast-grep.github.io/guide/introduction.html

---

enhance

"write_file" => self.write_file(args).await,
"edit_file" => self.edit_file(args).await,
use these for reference
> https://github.com/openai/codex/blob/main/codex-rs/core/src/tool_apply_patch.rs
> https://github.com/openai/codex/blob/main/codex-rs/apply-patch/apply_patch_tool_instructions.md

--

>https://github.com/openai/codex/blob/main/codex-rs/core/src/prompt_for_compact_command.md
--

use ast-grep for code searches and refactors has turned it into an unstoppable coding monster

https://ast-grep.github.io/llms-full.txt

https://ast-grep.github.io/llms.txt

--

https://github.com/coderabbitai/ast-grep-essentials

--

https://ast-grep.github.io/

--

https://ast-grep.github.io/catalog

--

claude code tools: bash, file search, file listing, file read and write, web fetch and search, TODOs, subagent

--

Key combos for common tasks:

1. `Shift+tab` to auto-accept edits
2. `#` to create a memory
3. `!` to enter bash mode
4. `@` to add a file/folder to context
5. `Esc` to cancel
6. `Double-esc` to jump back in history, --resume to resume
7. `ctrl+r` for verbose output
8. `/vibe`

---
https://davidlattimore.github.io/posts/2025/09/02/rustforge-wild-performance-tricks.html

--
