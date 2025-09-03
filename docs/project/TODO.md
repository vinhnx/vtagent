Refactor and extract the core logic from #file:main.rs, #file:main_tui.rs, and #file:tea_chat.rs into small, modular pieces of work. The main core has become too large. Run jscpd to detect duplicated code and fix any identified duplications. Ensure the refactored modules are well-organized, maintainable, and follow best practices for code separation.

--
<https://github.com/laude-institute/terminal-bench?tab=readme-ov-file#submit-to-our-leaderboard>

run some lt-bench agent benchmark to test agent capability. then update the the report in readme. checking for existing benchs

--

<https://app.primeintellect.ai/dashboard/environments>



--

-   [ ] Update documentation and README.md to reflect all recent changes, including new features, configuration options, and usage instructions.
-   [ ] Add a comprehensive usage guide to the README.md, covering setup, available commands, configuration via AGENTS.md, and example workflows.
-   [ ] Ensure all documented commands and options match the current implementation.
-   [ ] Review and update any outdated instructions or references in both documentation and README.md.



-

implement prompt caching to save token cost with context engineering. use mcp for agent provider agnostic (gemini, anthropic, openai)
prompt caching guide and apply to our system

--

<https://cognition.ai/blog/dont-build-multi-agents>

--

research claude code and apply
https://claudelog.com/

--

codex
https://github.com/openai/codex/tree/main/codex-rs

--

https://github.com/replit/ruspty

--

https://devin.ai/agents101#introduction


---

https://gerred.github.io/building-an-agentic-system/index.html

--

cfonts "vtagent"


 ██╗   ██╗ ████████╗  █████╗   ██████╗  ███████╗ ███╗   ██╗ ████████╗
 ██║   ██║ ╚══██╔══╝ ██╔══██╗ ██╔════╝  ██╔════╝ ████╗  ██║ ╚══██╔══╝
 ██║   ██║    ██║    ███████║ ██║  ███╗ █████╗   ██╔██╗ ██║    ██║
 ╚██╗ ██╔╝    ██║    ██╔══██║ ██║   ██║ ██╔══╝   ██║╚██╗██║    ██║
  ╚████╔╝     ██║    ██║  ██║ ╚██████╔╝ ███████╗ ██║ ╚████║    ██║
   ╚═══╝      ╚═╝    ╚═╝  ╚═╝  ╚═════╝  ╚══════╝ ╚═╝  ╚═══╝    ╚═╝


revamp welcome message

--

https://raw.githubusercontent.com/google-gemini/gemini-cli/main/docs/tools/file-system.md
https://raw.githubusercontent.com/google-gemini/gemini-cli/main/docs/tools/file-system.md


--

Failed to initialize indexer: error returned from database: (code: 1) unrecognized token: "#"
Note: Indexing will be disabled for this session

--

streaming

--

markdown render

--

event handling and turn based management for agent loop

--



https://github.com/rust-cli/roff-rs

--

https://github.com/rust-cli/env_logger

--

https://github.com/rust-cli/confy
--
https://github.com/zhiburt/tabled

--

https://github.com/zhiburt/ansi-str

https://github.com/zhiburt/ansi-str/tree/master/examples

---

https://github.com/Danau5tin/multi-agent-coding-system

---

https://github.com/Danau5tin/multi-agent-coding-system/blob/main/PROJECT_STRUCTURE.md


--

port https://github.com/rust-cli/rexpect

--

remove run_terminal_cmd and use run_pty_cmd tool 