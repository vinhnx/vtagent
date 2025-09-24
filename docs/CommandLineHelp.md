# Command-Line Help for `vtcode`

This document contains the help content for the `vtcode` command-line program.

**Command Overview:**

* [`vtcode`↴](#vtcode)
* [`vtcode chat`↴](#vtcode-chat)
* [`vtcode ask`↴](#vtcode-ask)
* [`vtcode chat-verbose`↴](#vtcode-chat-verbose)
* [`vtcode analyze`↴](#vtcode-analyze)
* [`vtcode performance`↴](#vtcode-performance)
* [`vtcode trajectory`↴](#vtcode-trajectory)
* [`vtcode benchmark`↴](#vtcode-benchmark)
* [`vtcode create-project`↴](#vtcode-create-project)
* [`vtcode compress-context`↴](#vtcode-compress-context)
* [`vtcode revert`↴](#vtcode-revert)
* [`vtcode snapshots`↴](#vtcode-snapshots)
* [`vtcode cleanup-snapshots`↴](#vtcode-cleanup-snapshots)
* [`vtcode init`↴](#vtcode-init)
* [`vtcode init-project`↴](#vtcode-init-project)
* [`vtcode config`↴](#vtcode-config)
* [`vtcode tool-policy`↴](#vtcode-tool-policy)
* [`vtcode tool-policy status`↴](#vtcode-tool-policy-status)
* [`vtcode tool-policy allow`↴](#vtcode-tool-policy-allow)
* [`vtcode tool-policy deny`↴](#vtcode-tool-policy-deny)
* [`vtcode tool-policy prompt`↴](#vtcode-tool-policy-prompt)
* [`vtcode tool-policy allow-all`↴](#vtcode-tool-policy-allow-all)
* [`vtcode tool-policy deny-all`↴](#vtcode-tool-policy-deny-all)
* [`vtcode tool-policy reset-all`↴](#vtcode-tool-policy-reset-all)
* [`vtcode models`↴](#vtcode-models)
* [`vtcode models list`↴](#vtcode-models-list)
* [`vtcode models set-provider`↴](#vtcode-models-set-provider)
* [`vtcode models set-model`↴](#vtcode-models-set-model)
* [`vtcode models config`↴](#vtcode-models-config)
* [`vtcode models test`↴](#vtcode-models-test)
* [`vtcode models compare`↴](#vtcode-models-compare)
* [`vtcode models info`↴](#vtcode-models-info)
* [`vtcode security`↴](#vtcode-security)
* [`vtcode tree-sitter`↴](#vtcode-tree-sitter)
* [`vtcode man`↴](#vtcode-man)

## `vtcode`

Advanced coding agent with Decision Ledger

Features:
• Single-agent architecture with Decision Ledger for reliable task execution
• Tree-sitter powered code analysis (Rust, Python, JavaScript, TypeScript, Go, Java)
• Multi-provider LLM support (Gemini, OpenAI, Anthropic, DeepSeek)
• Real-time performance monitoring and benchmarking
• Enhanced security with tool policies and sandboxing
• Research-preview context management and conversation compression

Quick Start:
  export GEMINI_API_KEY="your_key"
  vtcode chat

**Usage:** `vtcode [OPTIONS] [WORKSPACE] [COMMAND]`

###### **Subcommands:**

* `chat` — **Interactive AI coding assistant** with advanced capabilities
* `ask` — **Single prompt mode** - prints model reply without tools
* `chat-verbose` — **Verbose interactive chat** with enhanced transparency
* `analyze` — **Analyze workspace** with tree-sitter integration
* `performance` — **Display performance metrics** and system status\n\n**Shows:**\n• Token usage and API costs\n• Response times and latency\n• Tool execution statistics\n• Memory usage patterns\n\n**Usage:** vtcode performance
* `trajectory` — Pretty-print trajectory logs and show basic analytics
* `benchmark` — **Benchmark against SWE-bench evaluation framework**
* `create-project` — **Create complete Rust project with advanced features**
* `compress-context` — **Compress conversation context** for long-running sessions
* `revert` — **Revert agent to a previous snapshot
* `snapshots` — **List all available snapshots**
* `cleanup-snapshots` — **Clean up old snapshots**
* `init` — **Initialize project** with enhanced dot-folder structure
* `init-project` — **Initialize project with dot-folder structure** - sets up ~/.vtcode/projects/<project-name> structure
* `config` — **Generate configuration file - creates a vtcode.toml configuration file
* `tool-policy` — **Manage tool execution policies** - control which tools the agent can use
* `models` — **Manage models and providers** - configure and switch between LLM providers\n\n**Features:**\n• Support for latest models (DeepSeek, etc.)\n• Provider configuration and testing\n• Model performance comparison\n• API key management\n\n**Examples:**\n  vtcode models list\n  vtcode models set-provider deepseek\n  vtcode models set-model deepseek-reasoner
* `security` — **Security and safety management**\n\n**Features:**\n• Security scanning and vulnerability detection\n• Audit logging and monitoring\n• Access control management\n• Privacy protection settings\n\n**Usage:** vtcode security
* `tree-sitter` — **Tree-sitter code analysis tools**\n\n**Features:**\n• AST-based code parsing\n• Symbol extraction and navigation\n• Code complexity analysis\n• Multi-language refactoring\n\n**Usage:** vtcode tree-sitter
* `man` — **Generate or display man pages** for VTCode commands\n\n**Features:**\n• Generate Unix man pages for all commands\n• Display detailed command documentation\n• Save man pages to files\n• Comprehensive help for all VTCode features\n\n**Examples:**\n  vtcode man\n  vtcode man chat\n  vtcode man chat --output chat.1

###### **Arguments:**

* `<WORKSPACE>` — Optional positional path to run vtcode against a different workspace

###### **Options:**

* `--color <WHEN>` — Controls when to use color

  Default value: `auto`

  Possible values: `auto`, `always`, `never`

* `--model <MODEL>` — LLM Model ID with latest model support

   Available providers & models: • gemini-2.5-flash-preview-05-20 - Latest fast Gemini model (default) • gemini-2.5-flash - Fast, cost-effective • gemini-2.5-pro - Latest, most capable • gpt-5 - OpenAI's latest • claude-sonnet-4-20250514 - Anthropic's latest • qwen/qwen3-4b-2507 - Qwen3 local model • deepseek-reasoner - DeepSeek reasoning model • x-ai/grok-code-fast-1 - OpenRouter Grok fast coding model • qwen/qwen3-coder - OpenRouter Qwen3 Coder optimized for IDE usage • grok-2-latest - xAI Grok flagship model
* `--provider <PROVIDER>` — **LLM Provider** with expanded support

   Available providers: • gemini - Google Gemini (default) • openai - OpenAI GPT models • anthropic - Anthropic Claude models • deepseek - DeepSeek models • openrouter - OpenRouter marketplace models • xai - xAI Grok models

   Example: --provider deepseek
* `--api-key-env <API_KEY_ENV>` — **API key environment variable**\n\n**Auto-detects based on provider:**\n• Gemini: `GEMINI_API_KEY`\n• OpenAI: `OPENAI_API_KEY`\n• Anthropic: `ANTHROPIC_API_KEY`\n• DeepSeek: `DEEPSEEK_API_KEY`\n• OpenRouter: `OPENROUTER_API_KEY`\n• xAI: `XAI_API_KEY`\n\n**Override:** --api-key-env CUSTOM_KEY

  Default value: `GEMINI_API_KEY`
* `--workspace <PATH>` — **Workspace root directory for file operations**

   Security: All file operations restricted to this path Default: Current directory
* `--enable-tree-sitter` — **Enable tree-sitter code analysis**

   Features: • AST-based code parsing • Symbol extraction and navigation • Intelligent refactoring suggestions • Multi-language support (Rust, Python, JS, TS, Go, Java)
* `--performance-monitoring` — **Enable performance monitoring**

   Tracks: • Token usage and API costs • Response times and latency • Tool execution metrics • Memory usage patterns
* `--research-preview` — **Enable research-preview features**

   Includes: • Advanced context compression • Conversation summarization • Enhanced error recovery • Decision transparency tracking
* `--security-level <SECURITY_LEVEL>` — **Security level** for tool execution

   Options: • strict - Maximum security, prompt for all tools • moderate - Balance security and usability • permissive - Minimal restrictions (not recommended)

  Default value: `moderate`
* `--show-file-diffs` — **Show diffs for file changes in chat interface**

   Features: • Real-time diff rendering • Syntax highlighting • Line-by-line changes • Before/after comparison
* `--max-concurrent-ops <MAX_CONCURRENT_OPS>` — **Maximum concurrent async operations**

   Default: 5 Higher values: Better performance but more resource usage

  Default value: `5`
* `--api-rate-limit <API_RATE_LIMIT>` — **Maximum API requests per minute**

   Default: 30 Purpose: Prevents rate limiting

  Default value: `30`
* `--max-tool-calls <MAX_TOOL_CALLS>` — **Maximum tool calls per session**

   Default: 10 Purpose: Prevents runaway execution

  Default value: `10`
* `--debug` — **Enable debug output for troubleshooting**

   Shows: • Tool call details • API request/response • Internal agent state • Performance metrics
* `--verbose` — **Enable verbose logging**

   Includes: • Detailed operation logs • Context management info • Agent coordination details
* `--config <CONFIG>` — **Configuration file path**

   Supported formats: TOML Default locations: ./vtcode.toml, ~/.vtcode/vtcode.toml
* `--log-level <LOG_LEVEL>` — Log level (error, warn, info, debug, trace)

   Default: info

  Default value: `info`
* `--no-color` — Disable color output

   Useful for: Log files, CI/CD pipelines
* `--theme <THEME>` — Select UI theme for ANSI styling (e.g., ciapre-dark, ciapre-blue)
* `--skip-confirmations` — **Skip safety confirmations**

   Warning: Reduces security, use with caution
* `--full-auto` — **Enable full-auto mode (no interaction)**

   Runs the agent without pausing for approvals. Requires enabling in configuration.



## `vtcode chat`

**Interactive AI coding assistant** with advanced capabilities

Features: • Real-time code generation and editing • Tree-sitter powered analysis • Research-preview context management

Usage: vtcode chat

**Usage:** `vtcode chat`



## `vtcode ask`

**Single prompt mode** - prints model reply without tools

Perfect for: • Quick questions • Code explanations • Simple queries

Example: vtcode ask "Explain Rust ownership"

**Usage:** `vtcode ask <PROMPT>`

###### **Arguments:**

* `<PROMPT>`



## `vtcode chat-verbose`

**Verbose interactive chat** with enhanced transparency

Shows: • Tool execution details • API request/response • Performance metrics

Usage: vtcode chat-verbose

**Usage:** `vtcode chat-verbose`



## `vtcode analyze`

**Analyze workspace** with tree-sitter integration

Provides: • Project structure analysis • Language detection • Code complexity metrics • Dependency insights • Symbol extraction

Usage: vtcode analyze

**Usage:** `vtcode analyze`



## `vtcode performance`

**Display performance metrics** and system status\n\n**Shows:**\n• Token usage and API costs\n• Response times and latency\n• Tool execution statistics\n• Memory usage patterns\n\n**Usage:** vtcode performance

**Usage:** `vtcode performance`



## `vtcode trajectory`

Pretty-print trajectory logs and show basic analytics

Sources: • logs/trajectory.jsonl (default) Options: • --file to specify an alternate path • --top to limit report rows (default: 10)

Shows: • Class distribution with percentages • Model usage statistics • Tool success rates with status indicators • Time range of logged activity

**Usage:** `vtcode trajectory [OPTIONS]`

###### **Options:**

* `--file <FILE>` — Optional path to trajectory JSONL file
* `--top <TOP>` — Number of top entries to show for each section

  Default value: `10`



## `vtcode benchmark`

**Benchmark against SWE-bench evaluation framework**

Features: • Automated performance testing • Comparative analysis across models • Benchmark scoring and metrics • Optimization insights

Usage: vtcode benchmark

**Usage:** `vtcode benchmark`



## `vtcode create-project`

**Create complete Rust project with advanced features**

Features: • Web frameworks (Axum, Rocket, Warp) • Database integration • Authentication systems • Testing setup • Tree-sitter integration

Example: vtcode create-project myapp web,auth,db

**Usage:** `vtcode create-project <NAME> [FEATURES]...`

###### **Arguments:**

* `<NAME>`
* `<FEATURES>`



## `vtcode compress-context`

**Compress conversation context** for long-running sessions

Benefits: • Reduced token usage • Faster responses • Memory optimization • Context preservation

Usage: vtcode compress-context

**Usage:** `vtcode compress-context`



## `vtcode revert`

**Revert agent to a previous snapshot

Features: • Revert to any previous turn • Partial reverts (memory, context, full) • Safe rollback with validation

Examples: vtcode revert --turn 5 vtcode revert --turn 3 --partial memory

**Usage:** `vtcode revert [OPTIONS] --turn <TURN>`

###### **Options:**

* `-t`, `--turn <TURN>` — Turn number to revert to

   Required: Yes Example: 5
* `-p`, `--partial <PARTIAL>` — Scope of revert operation

   Options: memory, context, full Default: full Examples: --partial memory (revert conversation only) --partial context (revert decisions/errors only)



## `vtcode snapshots`

**List all available snapshots**

Shows: • Snapshot ID and turn number • Creation timestamp • Description • File size and compression status

Usage: vtcode snapshots

**Usage:** `vtcode snapshots`



## `vtcode cleanup-snapshots`

**Clean up old snapshots**

Features: • Remove snapshots beyond limit • Configurable retention policy • Safe deletion with confirmation

Examples: vtcode cleanup-snapshots vtcode cleanup-snapshots --max 20

**Usage:** `vtcode cleanup-snapshots [OPTIONS]`

###### **Options:**

* `-m`, `--max <MAX>` — Maximum number of snapshots to keep

   Default: 50 Example: --max 20

  Default value: `50`



## `vtcode init`

**Initialize project** with enhanced dot-folder structure

Features: • Creates project directory structure • Sets up config, cache, embeddings directories • Creates .project metadata file • Tree-sitter parser setup

Usage: vtcode init

**Usage:** `vtcode init`



## `vtcode init-project`

**Initialize project with dot-folder structure** - sets up ~/.vtcode/projects/<project-name> structure

Features: • Creates project directory structure in ~/.vtcode/projects/ • Sets up config, cache, embeddings, and retrieval directories • Creates .project metadata file • Migrates existing config/cache files with user confirmation

Examples: vtcode init-project vtcode init-project --name my-project vtcode init-project --force

**Usage:** `vtcode init-project [OPTIONS]`

###### **Options:**

* `--name <NAME>` — Project name - defaults to current directory name
* `--force` — Force initialization - overwrite existing project structure
* `--migrate` — Migrate existing files - move existing config/cache files to new structure



## `vtcode config`

**Generate configuration file - creates a vtcode.toml configuration file

Features: • Generate default configuration • Support for global (home directory) and local configuration • TOML format with comprehensive settings • Tree-sitter and performance monitoring settings

Examples: vtcode config vtcode config --output ./custom-config.toml vtcode config --global

**Usage:** `vtcode config [OPTIONS]`

###### **Options:**

* `--output <OUTPUT>` — Output file path - where to save the configuration file
* `--global` — Create in user home directory - creates ~/.vtcode/vtcode.toml



## `vtcode tool-policy`

**Manage tool execution policies** - control which tools the agent can use

Features: • Granular tool permissions • Security level presets • Audit logging • Safe tool execution

Examples: vtcode tool-policy status vtcode tool-policy allow file-write vtcode tool-policy deny shell-exec

**Usage:** `vtcode tool-policy <COMMAND>`

###### **Subcommands:**

* `status` — Show current tool policy status
* `allow` — Allow a specific tool
* `deny` — Deny a specific tool
* `prompt` — Set a tool to prompt for confirmation
* `allow-all` — Allow all tools
* `deny-all` — Deny all tools
* `reset-all` — Reset all tools to prompt



## `vtcode tool-policy status`

Show current tool policy status

**Usage:** `vtcode tool-policy status`



## `vtcode tool-policy allow`

Allow a specific tool

**Usage:** `vtcode tool-policy allow <TOOL>`

###### **Arguments:**

* `<TOOL>` — Tool name to allow



## `vtcode tool-policy deny`

Deny a specific tool

**Usage:** `vtcode tool-policy deny <TOOL>`

###### **Arguments:**

* `<TOOL>` — Tool name to deny



## `vtcode tool-policy prompt`

Set a tool to prompt for confirmation

**Usage:** `vtcode tool-policy prompt <TOOL>`

###### **Arguments:**

* `<TOOL>` — Tool name to set to prompt



## `vtcode tool-policy allow-all`

Allow all tools

**Usage:** `vtcode tool-policy allow-all`



## `vtcode tool-policy deny-all`

Deny all tools

**Usage:** `vtcode tool-policy deny-all`



## `vtcode tool-policy reset-all`

Reset all tools to prompt

**Usage:** `vtcode tool-policy reset-all`



## `vtcode models`

**Manage models and providers** - configure and switch between LLM providers\n\n**Features:**\n• Support for latest models (DeepSeek, etc.)\n• Provider configuration and testing\n• Model performance comparison\n• API key management\n\n**Examples:**\n  vtcode models list\n  vtcode models set-provider deepseek\n  vtcode models set-model deepseek-reasoner

**Usage:** `vtcode models <COMMAND>`

###### **Subcommands:**

* `list` — List all providers and models with status indicators
* `set-provider` — Set default provider (gemini, openai, anthropic, deepseek)
* `set-model` — Set default model (e.g., deepseek-reasoner, gpt-5, claude-sonnet-4-20250514)
* `config` — Configure provider settings (API keys, base URLs, models)
* `test` — Test provider connectivity and validate configuration
* `compare` — Compare model performance across providers (coming soon)
* `info` — Show detailed model information and specifications



## `vtcode models list`

List all providers and models with status indicators

**Usage:** `vtcode models list`



## `vtcode models set-provider`

Set default provider (gemini, openai, anthropic, deepseek)

**Usage:** `vtcode models set-provider <PROVIDER>`

###### **Arguments:**

* `<PROVIDER>` — Provider name to set as default



## `vtcode models set-model`

Set default model (e.g., deepseek-reasoner, gpt-5, claude-sonnet-4-20250514)

**Usage:** `vtcode models set-model <MODEL>`

###### **Arguments:**

* `<MODEL>` — Model name to set as default



## `vtcode models config`

Configure provider settings (API keys, base URLs, models)

**Usage:** `vtcode models config [OPTIONS] <PROVIDER>`

###### **Arguments:**

* `<PROVIDER>` — Provider name to configure

###### **Options:**

* `--api-key <API_KEY>` — API key for the provider
* `--base-url <BASE_URL>` — Base URL for local providers
* `--model <MODEL>` — Default model for this provider



## `vtcode models test`

Test provider connectivity and validate configuration

**Usage:** `vtcode models test <PROVIDER>`

###### **Arguments:**

* `<PROVIDER>` — Provider name to test



## `vtcode models compare`

Compare model performance across providers (coming soon)

**Usage:** `vtcode models compare`



## `vtcode models info`

Show detailed model information and specifications

**Usage:** `vtcode models info <MODEL>`

###### **Arguments:**

* `<MODEL>` — Model name to get information about



## `vtcode security`

**Security and safety management**\n\n**Features:**\n• Security scanning and vulnerability detection\n• Audit logging and monitoring\n• Access control management\n• Privacy protection settings\n\n**Usage:** vtcode security

**Usage:** `vtcode security`



## `vtcode tree-sitter`

**Tree-sitter code analysis tools**\n\n**Features:**\n• AST-based code parsing\n• Symbol extraction and navigation\n• Code complexity analysis\n• Multi-language refactoring\n\n**Usage:** vtcode tree-sitter

**Usage:** `vtcode tree-sitter`



## `vtcode man`

**Generate or display man pages** for VTCode commands\n\n**Features:**\n• Generate Unix man pages for all commands\n• Display detailed command documentation\n• Save man pages to files\n• Comprehensive help for all VTCode features\n\n**Examples:**\n  vtcode man\n  vtcode man chat\n  vtcode man chat --output chat.1

**Usage:** `vtcode man [OPTIONS] [COMMAND]`

###### **Arguments:**

* `<COMMAND>` — **Command name** to generate man page for (optional)\n\n**Available commands:**\n• chat, ask, analyze, performance, benchmark\n• create-project, init, man\n\n**If not specified, shows main VTCode man page**

###### **Options:**

* `-o`, `--output <OUTPUT>` — **Output file path** to save man page\n\n**Format:** Standard Unix man page format (.1, .8, etc.)\n**Default:** Display to stdout



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

