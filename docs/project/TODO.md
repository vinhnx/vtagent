<https://tree-sitter.github.io/tree-sitter/>

--

<https://github.com/BurntSushi/ripgrep>

--

1. [Home](/)
2. [Blog](/blog/)
3. What makes Claude Code so damn good (and how to recreate that magic in your agent)!?

![You can clearly see the different Claude Code updates](/images/claude-code/banner2.png)

You can clearly see the different Claude Code updates

# What makes Claude Code so damn good (and how to recreate that magic in your agent)!?

/ [vivek](https://x.com/nuwandavek) / 2025-08-21

Claude Code is the most delightful AI agent/workflow I have used so far. Not only does it make targeted edits or vibe coding throwaway tools less annoying, using Claude Code makes me happy. It has enough autonomy to do interesting things, while not inducing a jarring loss of control like some other tools do. Of course most of the heavy lifting is done by the new Claude 4 model (especially interleaved thinking). But I find Claude Code objectively less annoying to use compared to Cursor, or Github Copilot agents even with the same underlying model! What makes it so damn good? If you're reading this and nodding along, I'm going to try and provide some answers.

**Note**: This is not a blogpost with Claude Code's architecture dump (there are some good ones out there). This blogpost is meant to be a guide for building delightful LLM agents, based on my own experience using and tinkering with Claude Code over the last few months (and all the logs we intercepted and analyzed). You can find [prompts](#appendix) and [tools](#appendix) in the [Appendix section](#appendix). This post is ~2k words long, so strap in! If you're looking for some quick takeaways, the [TL;DR](#how-to-build-a-claude-code-like-agent-tldr) section is a good place to start.

![prompts](/images/claude-code/prompts.png)

You can clearly see the different Claude Code updates.

Claude Code (CC) feels great to use, because it *just simply works*. CC has been crafted with a fundamental understanding of what the LLM is good at and what it is terrible at. Its prompts and tools cover for the model's stupidity and help it shine in its wheelhouse. The control loop is extremely simple to follow and trivial to debug.

We started using CC at MinusX as soon as it launched. To look under the hood, [Sreejith](https://x.com/ppsreejith_) wrote a logger that intercepts and logs every network request made. The following analysis is from my extensive use over the last couple of months. **This post attempts to answer the question - "What makes Claude Code so good, and how can you give a CC-like experience in your own chat-based-LLM agent?"** We've incorporated most of these into MinusX already and I'm excited to see you do it too!

![prompts](/images/claude-code/tools.png)

Edit is the most frequent tool, followed by Read and ToDoWrite

## [How to build a Claude Code like agent: TL;DR](#how-to-build-a-claude-code-like-agent-tldr)

If there is one thing to take away from this, it is this - **Keep Things Simple, Dummy**. LLMs are terrible enough to debug and evaluate. Any additional complexity you introduce (multi-agents, agent handoffs or complex RAG search algorithms) only makes debugging 10x harder. If such a fragile system works at all, you'll be terrified of making drastic changes to it later. So, keep everything in one file, avoid excessive boilerplate scaffolding and rip it all out at least a couple of times :)

Here are the main takeaways from Claude Code to implement in your own system.

#### [1\. Control Loop](#1-control-loop)

- 1.1 [Keep one main loop (with max one branch) and one message history](#11-keep-one-main-loop)
- 1.2 [Use a smaller model for all sorts of things. All. The. Frickin. Time.](#12-use-a-smaller-model-for-everything)

#### [2\. Prompts](#2-prompts)

- 2.1 [Use AGENTS.md pattern to collaborate on and remember user preferences](#21-use-agentsmd-for-collaborating-on-user-context-and-preferences)
- 2.2 [Use special XML Tags, Markdown, and lots of examples](#22-special-xml-tags-markdown-and-lots-of-examples)

#### [3\. Tools](#3-tools)

- 3.1 [LLM search >>> RAG based search](#31-llm-search---rag-based-search)
- 3.2 [How to design good tools? (High vs Low level tools)](#32-how-to-design-good-tools-low-level-vs-high-level-tools)
- 3.3 [Let your agent manage its own todo list](#33-let-the-agent-manage-a-todo-list)

#### [4\. Steerability](#4-steerability)

- 4.1 [Tone and style](#41-tone-and-style)
- 4.2 ["**PLEASE THIS IS IMPORTANT**" is unfortunately still state of the art](#42-this-is-important-is-still-state-of-the-art)
- 4.3 [Write the algorithm, with heuristics and examples](#43-write-the-algorithm-with-heuristics-and-examples)

> Claude Code choses architectural simplicity at every juncture - one main loop, simple search, simple todolist, etc. Resist the urge to over-engineer, build good harness for the model let it cook! Is this end-to-end self-driving all over again? Bitter lesson much?

---

## [1\. Control Loop Design](#1-control-loop-design)

### [1.1 Keep One Main Loop](#11-keep-one-main-loop)

Debuggability >>> complicated hand-tuned multi-agent lang-chain-graph-node mishmash.

Despite multi agent systems being all the rage, Claude Code has just one main thread. It uses a few different types of prompts periodically to summarize the git history, to clobber up the message history into one message or to come up with some fun UX elements. But apart from that, it maintains a flat list of messages. An interesting way it handles hierarchical tasks is by spawning itself as a sub-agent without the ability to spawn more sub-agents. There is a maximum of one branch, the result of which is added to the main message history as a "tool response".

If the problem is simple enough, the main loop just handles it via iterative tool calling. But if there are one or more tasks that are complex, the main agent creates clones of itself. The combination of the max-1-branch and the todo list makes sure the agent has the ability to break the problem into sub-problems, but also keep the eye on the final desired outcome.

I highly doubt your app needs a multi-agent system. With every layer of abstraction you make your system harder to debug, and more importantly you deviate from the general-model-improvement trajectory.

![Control Loop](/images/claude-code/control_loop.gif)

### [1.2 Use a Smaller model for *everything*](#12-use-a-smaller-model-for-everything)

Over 50% of all important LLM calls made by CC are to claude-3-5-haiku. It is used to read large files, parse web pages, process git history and summarize long conversations. It is also used to come up with the one-word processing label - literally for every key stroke! The smaller models are 70-80% cheaper than the standard ones (Sonnet 4, GPT-4.1). Use them liberally!

## [2\. Prompts](#2-prompts-1)

Claude Code has extremely elaborate prompts filled with heuristics, examples and IMPORTANT (tch-tch) reminders. The system prompt is ~2800 tokens long, with the Tools taking up a whopping 9400 tokens. The user prompt always contains the AGENTS.md file, which can typically be another 1000-2000 tokens. The system prompt contains sections on tone, style, proactiveness, task management, tool usage policy and doing tasks. It also contains the date, current working directory, platform and OS information and recent commits.

[**Go read the entire prompt**](#appendix)!

### [2.1 Use AGENTS.md for collaborating on user context and preferences](#21-use-agentsmd-for-collaborating-on-user-context-and-preferences)

One of the major patterns most coding agent creators have settled on is the context file (aka Cursor Rules / AGENTS.md / agent.md). The difference in Claude Code's performance with and without AGENTS.md is night and day. It is a great way for the developers to impart context that cannot be inferred from the codebase and to codify all strict preferences. For example, you can force the LLM to skip some folders, or use specific libraries. CC sends the entire contents of the AGENTS.md with every user request

We recently introduced [minusx.md in MinusX](/blog/memory/) which is fast becoming the de-facto context file for our agents to codify user and team preferences.

### [2.2 Special XML Tags, Markdown, and lots of examples](#22-special-xml-tags-markdown-and-lots-of-examples)

It is fairly established that XML tags and Markdown are two ways to structure a prompt. CC uses both, extensively. Here are a few notable XML tags in Claude Code:

- `<system-reminder>`: This is used at the end of many prompt sections to remind the LLM of thing it presumably otherwise forgets. Example:

```
<system-reminder>This is a reminder that your todo list is currently empty. DO NOT mention this to the user explicitly because they are already aware. If you are working on tasks that would benefit from a todo list please use the TodoWrite tool to create one. If not, please feel free to ignore. Again do not mention this message to the user.</system-reminder>
```

- `<good-example>`, `<bad-example>`: These are used to codify heuristics. They can be especially useful when there is a fork in the road with multiple seemingly reasonable paths/tool\_calls the model can choose. Examples can be used to contrast the cases and make it very clear which path is preferable. Example:

```
Try to maintain your current working directory throughout the session by using absolute paths and avoiding usage of \`cd\`. You may use \`cd\` if the User explicitly requests it.
<good-example>
pytest /foo/bar/tests
</good-example>
<bad-example>
cd /foo/bar && pytest tests
</bad-example>
```

CC also uses markdown to demarcate clear sections in the system prompt. Example markdown headings include:

- Tone and style
- Proactiveness
- Following conventions
- Code style
- Task Management
- Tool use policy
- Doing Tasks
- Tools

## [3\. Tools](#3-tools-1)

[**Go read the entire tools prompt**](#appendix) - it is a whopping 9400 tokens long!

### [3.1 LLM search >>> RAG based search](#31-llm-search---rag-based-search)

One significant way in which CC deviates from other popular coding agents is in its rejection of RAG. Claude Code searches your code base just as you would, with really complex `ripgrep`, `jq` and `find` commands. Since the LLM understands code really well, it can use sophisticated regex to find pretty much any codeblock it deems relevant. Sometimes it ends up reading whole files with a smaller model.

RAG sounds like a good idea in theory, but it introduces new (and more importantly, hidden) failure modes. What is the similarity function to use? What reranker? How do you chunk the code? What do you do with large JSON or log files? With LLM Search, it just looks at 10 lines of the json file to understand its structure. If it wants, it looks at 10 more lines - just like you would. Most importantly, this is RL learnable - something BigLabs are already working on. The model does most of the heavy lifting - as it should, dramatically reducing the number of moving parts in the agent. Also, having two complicated, intelligent systems wired this way is just ugly. I was recently kidding with a friend saying this is the Camera vs Lidar of the LLM era and I'm only half joking.

### [3.2 How to design good tools? (Low level vs High level tools)](#32-how-to-design-good-tools-low-level-vs-high-level-tools)

This question keeps anyone who is building an LLM agent up at night. Should you give the model generic tasks (like meaningful actions) or should it be low level (like type and click and bash)? The answer is that it depends (and you should use both).

Claude Code has low level (Bash, Read, Write), medium level (Edit, Grep, Glob) and high level tools (Task, WebFetch, exit\_plan\_mode). CC can use bash, so why give a separate Grep tool? The real trade-off here is in how often you expect your agent to use the tool vs accuracy of the agent in using the tool. CC uses grep and glob so frequently that it makes sense to make separate tools out of them, but at the same time, it can also write generic bash commands for special scenarios.

Similarly, there are even higher level tools like WebFetch or 'mcp\_\_ide\_\_getDiagnostics' that are extremely deterministic in what they do. This saves the LLM from having to do multiple low level clicking and typing and keeps it on track. Help the poor model out, will ya!? Tool descriptions have elaborate prompts with plenty of examples. The system prompt has information about when to use a tool' or how to choose between two tools that can do the same task.

**Tools in Claude Code:**

- [Task](#appendix)
- [Bash](#appendix)
- [Glob](#appendix)
- [Grep](#appendix)
- [LS](#appendix)
- [ExitPlanMode](#appendix)
- [Read](#appendix)
- [Edit](#)

- [MultiEdit](#appendix)
- [Write](#appendix)
- [NotebookEdit](#appendix)
- [WebFetch](#appendix)
- [TodoWrite](#appendix)
- [WebSearch](#appendix)
- [mcp\_\_ide\_\_getDiagnostics](#)
- [mcp\_\_ide\_\_executeCode](#)

### [3.3 Let the agent manage a todo list](#33-let-the-agent-manage-a-todo-list)

There are many reasons why this is a good idea. Context rot is a common problem in long-running LLM agents. They enthusiastically start out tackling a difficult problem, but over time lose their way and devolve into garbage. There are a few ways current agent designs tackle this. Many agents have experimented with explicit todos (one model generates todos, another model implements them) or with Multi-agent handoff + verification (PRD/PM agent -> implementer agent -> QA agent)

We already know multi-agent handoff is not a good idea, for many many reasons. CC uses an explicit todo list, but one that the model maintains. This keeps the LLM on track (it has been heavily prompted to refer to the todo list frequently), while at the same time giving the model the flexibility to course correct mid-way in an implementation. This also effectively leverages the model's interleaved thinking abilities to either reject or insert new todo items on the fly.

## [4\. Steerability](#4-steerability-1)

### [4.1 Tone and Style](#41-tone-and-style)

CC explicitly attempts to control the aesthetic behavior of the agent. There are sections in the system prompt around tone, style and proactiveness - full of instructions and examples. This is why Claude Code feels tasteful in its comments and eagerness. I recommend just copying large sections of this into your app as is.

```
# Some examples of tone and style
- IMPORTANT: You should NOT answer with unnecessary preamble or postamble (such as explaining your code or summarizing your action), unless the user asks you to.
Do not add additional code explanation summary unless requested by the user.

- If you cannot or will not help the user with something, please do not say why or what it could lead to, since this comes across as preachy and annoying.

- Only use emojis if the user explicitly requests it. Avoid using emojis in all communication unless asked.
```

### [4.2 "THIS IS IMPORTANT" is still State of the Art](#42-this-is-important-is-still-state-of-the-art)

Unfortunately CC is no better when it comes to asking the model to not do something. IMPORTANT, VERY IMPORTANT, NEVER and ALWAYS seem to be the best way to steer the model away from landmines. I expect the models to get more steerable in the future and avoid this ugliness. But for now, CC uses this liberally, and so should you. Some examples:

```
- IMPORTANT: DO NOT ADD ***ANY*** COMMENTS unless asked

- VERY IMPORTANT: You MUST avoid using search commands like \`find\` and \`grep\`. Instead use Grep, Glob, or Task to search. You MUST avoid read tools like \`cat\`, \`head\`, \`tail\`, and \`ls\`, and use Read and LS to read files.\n  - If you _still_ need to run \`grep\`, STOP. ALWAYS USE ripgrep at \`rg\` first

- IMPORTANT: You must NEVER generate or guess URLs for the user unless you are confident that the URLs are for helping the user with programming. You may use URLs provided by the user in their messages or local files.
```

### [4.3 Write the Algorithm (with heuristics and examples)](#43-write-the-algorithm-with-heuristics-and-examples)

It is extremely important to identify the most important task the LLM needs to perform and write out the algorithm for it. Try to role-play as the LLM and work through examples, identify all the decision points and write them explicitly. It helps if this is in the form of a flow-chart. This helps structure the decision making and aids the LLM in following instructions. One thing to definitely avoid is a big soup of Dos and Don'ts. They are harder to keep track, and keep mutually exclusive. If your prompt is several thousand tokens long, you will inadvertently have conflicting Dos and Don'ts. The LLM becomes extremely fragile in this case and it becomes impossible to incorporate new use cases.

`Task Management`, `Doing Tasks` and `Tool Usage Policy` sections in Claude Code's system prompt clearly walk through the algorithm to follow. This is also the section to add lots of heuristics and examples of various scenarios the LLM might encounter.

## [Bonus: Why pay attention to BigLab prompts?](#bonus-why-pay-attention-to-biglab-prompts)

A lot of the effort in steering LLMs is trying to reverse engineer their post-training / RLHF data distribution. Should you use JSON or XML? Should the tool descriptions be in the system prompt or just in tools? What about your app's current state? It helps to see what they do in their own apps and use it to inform yours. Claude Code design is very opinionated and it helps to use that in forming your own.

## [Conclusion](#conclusion)

The main takeaway, again, is to keep things simple. Extreme scaffolding frameworks will hurt more than help you. Claude Code really made me believe that an "agent" can be simple and yet extremely powerful. We've incorporated a bunch of these lessons into MinusX, and are continuing to incorporate more.

--
--

---

<https://github.com/ratatui/ratatui>
TUI

--

run some lt-bench agent benchmark to test agent capability. then update the the report in readme. checking for existing benchs

--

<https://app.primeintellect.ai/dashboard/environments>

--

run codex init

--

allow list

--

code terminal sandbox

--

build a tool to generate a todo list for a given task

--

- [ ] Implement robust file reading and writing capabilities
- [ ] Add support for multi-step code edits and insertions
- [ ] Integrate code formatting and linting tools
- [ ] Enable context-aware code suggestions
- [ ] Build a user-friendly interface for reviewing and applying changes
- [ ] Add error handling and rollback for failed code executions
- [ ] Support for running and testing code snippets in isolated environments
- [ ] Log all agent actions for transparency and debugging
- [ ] Allow user to provide feedback on agent suggestions
- [ ] Document all features and usage instructions

- goal: Use a single, simple control loop for agent actions
- goal: Maintain a unified message and action history for traceability
- goal: Store user preferences and context in a dedicated markdown file (e.g., `AGENTS.md`)
- goal: Design prompts with clear XML tags, markdown formatting, and concrete examples
- goal: Prefer smaller, faster models for routine tasks and context management
- goal: Minimize boilerplate and keep core logic in as few files as possible
- goal: Make all agent actions easily debuggable and transparent
- goal: Regularly review and simplify workflows to reduce complexity

--

- [ ] Refactor agent to support multiple LLM backends (model-agnostic design)

--

use ai-gateway or litellm

--

- [ ] Implement `/` slash commands for quick agent actions tools and tasks (e.g., `/read`, `/write`, `/edit`, `/todo`... etc)

--
--

- [ ] Ensure human-in-the-loop review for all critical agent actions
- [ ]  allow action y/n/cancel, the agent should prompt user for confirmation before performing an unix/tool action.

--

- [ ] allow list

--

- [ ] Build a CLI tool that generates a structured TODO list for any user-supplied task description
  - [ ] Accept a task description as input (via CLI argument or prompt)
  - [ ] Parse the task into actionable subtasks using LLM or rule-based logic
  - [ ] Output the TODO list in markdown format
  - [ ] Support output to file or stdout
  - [ ] Allow user to edit or reorder generated TODO items before saving
  - [ ] Integrate with existing agent context and preferences (e.g., AGENTS.md)
  - [ ] Add tests for various input scenarios and edge cases
  - [ ] Document usage with examples in README.md

--

- [ ] Update documentation and README.md to reflect all recent changes, including new features, configuration options, and usage instructions.
- [ ] Add a comprehensive usage guide to the README.md, covering setup, available commands, configuration via AGENTS.md, and example workflows.
- [ ] Ensure all documented commands and options match the current implementation.
- [ ] Review and update any outdated instructions or references in both documentation and README.md.

--

--

reference implement this guide and apply to our agent, but exclude aws

The wave of CLI Coding Agents

If you have tried Claude Code, Gemini Code, Open Code or Simon Willison’s LLM CLI, you’ve experienced something fundamentally different from ChatGPT or Github Copilot. These aren’t just chatbots or autocomplete tools - they’re agents that can read your code, run your tests, search docs and make changes to your codebase async.

But how do they work? For me the best way to understand how any tool works is to try and build it myself. So that’s exactly what we did, and in this article I’ll take you through how we built our own CLI Coding Agent using the Pydantic-AI framework and the Model Context Protocol (MCP). You’ll see not just how to assemble the pieces but why each capability matters and how it changes the way you can work with code.

Before diving into the technical implementation, let's examine why we chose to build our own solution.

The answer became clear very quickly using our custom agent, while commercial tools are impressive, they’re built for general use cases. Our agent was fully customised to our internal context and all the little eccentricities of our specific project. More importantly, building it gave us insights into how these systems work and the quality of our own GenAI Platform and Dev Tooling.

Think of it like learning to cook. You can eat at restaurants forever but understanding how flavours combine and techniques work makes you appreciate food differently - and lets you create exactly what you want.
The Architecture of Our Development Agent

At a high level, our coding assistant consists of several key components:

    Pydantic-AI Framework: provides the agent framework and many helpful utilities to make our Agent more useful immediately
    MCP Servers: independent processes that give the agent specialised tools, MCP is a common standard for defining the servers that contain these tools.
    CLI Interface: how users interact with the assistant

The magic happens through the Model Context Protocol (MCP), which allows the AI model to use various tools through a standardized interface. This architecture makes our assistant highly extensible - we can easily add new capabilities by implementing additional MCP servers, but we’re getting ahead of ourselves.
Starting Simple: The Foundation

We started by creating a basic project structure and installing the necessary dependencies:

uv init
uv add pydantic_ai
uv add boto3

Our primary dependencies include:

    pydantic-ai: Framework for building AI agents

Here's how we configured it in our main.py:

import boto3
from pydantic_ai import Agent
from pydantic_ai.mcp import MCPServerStdio
from pydantic_ai.models.bedrock import BedrockConverseModel
from pydantic_ai.providers.bedrock import BedrockProvider

bedrock_config = BotocoreConfig(
    read_timeout=300,
    connect_timeout=60,
    retries={"max_attempts": 3},
)
bedrock_client = boto3.client(
    "bedrock-runtime", region_name="eu-central-1", config=bedrock_config
)
model = BedrockConverseModel(
    "eu.anthropic.claude-sonnet-4-20250514-v1:0",
    provider=BedrockProvider(bedrock_client=bedrock_client),
)
agent = Agent(
    model=model,
)

if **name** == "**main**":
  agent.to_cli_sync()

At this stage we already have a fully working CLI with a chat interface which we can use as you would a GUI chat interface, which is pretty cool for how little code this is! However we can definitely improve upon this.
First Capability: Testing!

Instead of running the tests ourselves after each coding iteration why not get the agent to do it? Seems simple right?

import subprocess

@agent.tool_plain()
def run_unit_tests() -> str:
    """Run unit tests using uv."""
    result = subprocess.run(
        ["uv", "run", "pytest", "-xvs", "tests/"], capture_output=True, text=True
    )
    return result.stdout

Here we use the same pytest command you would run in the terminal (I’ve shortened ours for the article). Now something magical happened. I could say “X isn’t working” and the agent would:

    1. Run the test suite
    2. Identify which specific tests were failing
    3. Analyze the error messages
    4. Suggest targeted fixes.

The workflow change: Instead of staring at test failures or copy pasting terminal outputs into ChatGPT we now give our agent super relevant context about any issues in our codebase.

However we noticed our agent sometimes “fixed” failing tests by suggesting changes to the tests, not the actual implementation. This led to our next addition.
Adding Intelligence: Instructions and intent

We realised we needed to teach our agent a little more about our development philosophy and steer it away from bad behaviours.

instructions = """
You are a specialised agent for maintaining and developing the XXXXXX codebase.

## Development Guidelines

1. **Test Failures:**
   - When tests fail, fix the implementation first, not the tests
   - Tests represent expected behavior; implementation should conform to tests
   - Only modify tests if they clearly don't match specifications

2. **Code Changes:**
   - Make the smallest possible changes to fix issues
   - Focus on fixing the specific problem rather than rewriting large portions
   - Add unit tests for all new functionality before implementing it

3. **Best Practices:**
   - Keep functions small with a single responsibility
   - Implement proper error handling with appropriate exceptions
   - Be mindful of configuration dependencies in tests

Remember to examine test failure messages carefully to understand the root cause before making any changes.
"""

agent = Agent(
instructions=instructions,
model=model,
)

The workflow change: The agent now understands our values around Test Driven Development and minimal changes. It stopped suggesting large refactors where a small fix would do (Mostly).

Now while we could continue building everything from absolute scratch and tweaking our prompts for days we want to go fast and use some tools other people have built - Enter Model Context Protocol (MCP).
The MCP Revolution: Pluggable Capabilities

This is where our agent transformed from a helpful assistant to something approaching the commercial CLI agents. The Model Context Protocol (MCP) allows us to add sophisticated capabilities by running specialized servers.

    MCP is an open protocol that standardizes how applications provide context to LLMs. Think of MCP like a USB-C port for AI applications. Just as USB-C provides a standardized way to connect your devices to various peripherals and accessories, MCP provides a standardized way to connect AI models to different data sources and tools.

    -- MCP Introduction

We can run these servers as a local process, so no data sharing, where we interact with STDIN/STDOUT to keep things simple and local. (More details on tools and MCP)
Sandboxed Python Execution

Using large language models to do calculations or executing arbitrary code they create is not effective and potentially very dangerous! To make our Agent more accurate and safe our first MCP addition was Pydantic Al’s default server for sandboxed Python code execution:

run_python = MCPServerStdio(
    "deno",
    args=[
        "run",
        "-N",
        "-R=node_modules",
        "-W=node_modules",
        "--node-modules-dir=auto",
        "jsr:@pydantic/mcp-run-python",
        "stdio",
    ],
)

agent = Agent(
    ...
    mcp_servers=[
        run_python
    ],
)

This gave our agent a sandbox where it could test ideas, prototype solutions, and verify its own suggestions.

NOTE: This is very different from running the tests where we need the local environment and is intended to be used to make calculations much more robust. This is because writing the code to output a number and then executing that code is much more reliable and understandable, scalable and repeatable than just generating the next token in a calculation. We have seen from frontier labs (including their leaked instructions) that this is a much better approach.

The workflow change: Doing calculations, even more complex ones, became significantly more reliable. This is useful for many things like dates, sums, counts etc. It also allows for a rapid iteration cycle of simple python code.
Up-to-Date library Documentation

LLMs are mostly trained in batch on historical data this gives a fixed cutoff while languages and dependencies continue to change and improve so we added Context7 for access to up to date python library documentation in LLM consumable format:

context7 = MCPServerStdio(
    command="npx", args=["-y", "@upstash/context7-mcp"], tool_prefix="context"
)

The workflow change: When working with newer libraries or trying to use Minimal research-preview features, the agent could look up current documentation rather than relying on potentially outdated training data. This made it much more reliable for real-world development work.
Internet Search for Current Information

Sometimes you need information that's not in any documentation—recent Stack Overflow discussions, GitHub issues, or the latest best practices. We added general internet search:

internet_search = MCPServerStdio(command="uvx", args=["duckduckgo-mcp-server"])

The workflow change: When encountering obscure errors or needing to understand recent changes in the ecosystem, the agent could search for current discussions and solutions. This was particularly valuable for debugging deployment issues or understanding breaking changes in dependencies.
Structured Problem Solving

One of the most valuable additions was the code reasoning MCP, which helps the agent think through complex problems systematically:

code_reasoning = MCPServerStdio(
    command="npx",
    args=["-y", "@mettamatt/code-reasoning"],
    tool_prefix="code_reasoning",
)

The workflow change: Instead of jumping to solutions, the agent would break down complex problems into logical steps, explore alternative approaches, and explain its reasoning. This was invaluable for architectural decisions and debugging complex issues. I could ask “Why is this API call failing intermittently?” and get a structured analysis of potential causes rather than just guesses.
Optimising for Reasoning

As we added more sophisticated capabilities, we noticed that reasoning and analysis tasks often took much longer than regular text generation—especially when the output wasn't correctly formatted on the first try. We adjusted our Bedrock configuration to be more patient:

bedrock_config = BotocoreConfig(
    read_timeout=300,
    connect_timeout=60,
    retries={"max_attempts": 3},
)
bedrock_client = boto3.client(
    "bedrock-runtime", region_name="eu-central-1", config=bedrock_config
)

The workflow change: The longer timeouts meant our agent could work through complex problems without timing out. When analyzing large codebases or reasoning through intricate architectural decisions, the agent could take the time needed to provide thorough, well-reasoned responses rather than rushing to incomplete solutions.
Desktop Commander: Warning! With great power comes great responsibility!

At this point, our agent was already quite capable—it could reason through problems, execute code, search for information, and . This MCP server transforms your agent from a helpful assistant into something that can actually do things in your development environment:

desktop_commander = MCPServerStdio(
    command="npx",
    args=["-y", "@wonderwhy-er/desktop-commander"],
    tool_prefix="desktop_commander",
)

Desktop Commander provides an incredibly comprehensive toolkit: file system operations (read, write, search), terminal command execution with process management, surgical code editing with edit_block, and even interactive REPL sessions. It's built on top of the MCP Filesystem Server but adds crucial capabilities like search-and-replace editing and intelligent process control.

The workflow change: This is where everything came together. I could now say “The authentication tests are failing, please fix the issue” and the agent would:

    1. Run the test suite to see the specific failures
    2. Read the failing test files to understand what was expected
    3. Examine the authentication module code
    4. Search the codebase for related patterns
    5. Look up the documentation for the relevant library
    6. Make edits to fix the implementation
    7. Re-run the tests to verify the fix
    8. Search for similar patterns elsewhere that might need updating

All of this happened in a single conversation thread, with the agent maintaining context throughout. It wasn't just generating code suggestions—it was actively debugging, editing, and verifying fixes like a pair programming partner.

The security model is thoughtful too, with configurable allowed directories, blocked commands, and proper permission boundaries. You can learn more about its extensive capabilities at the Desktop Commander documentation.
The Complete System

Here's our final agent configuration:

import asyncio

import subprocess
import boto3
from pydantic_ai import Agent
from pydantic_ai.mcp import MCPServerStdio
from pydantic_ai.models.bedrock import BedrockConverseModel
from pydantic_ai.providers.bedrock import BedrockProvider
from botocore.config import Config as BotocoreConfig

bedrock_config = BotocoreConfig(
    read_timeout=300,
    connect_timeout=60,
    retries={"max_attempts": 3},
)
bedrock_client = boto3.client(
    "bedrock-runtime", region_name="eu-central-1", config=bedrock_config
)
model = BedrockConverseModel(
    "eu.anthropic.claude-sonnet-4-20250514-v1:0",
    provider=BedrockProvider(bedrock_client=bedrock_client),
)
agent = Agent(
    model=model,
)

instructions = """
You are a specialised agent for maintaining and developing the XXXXXX codebase.

## Development Guidelines

1. **Test Failures:**
   - When tests fail, fix the implementation first, not the tests
   - Tests represent expected behavior; implementation should conform to tests
   - Only modify tests if they clearly don't match specifications

2. **Code Changes:**
   - Make the smallest possible changes to fix issues
   - Focus on fixing the specific problem rather than rewriting large portions
   - Add unit tests for all new functionality before implementing it

3. **Best Practices:**
   - Keep functions small with a single responsibility
   - Implement proper error handling with appropriate exceptions
   - Be mindful of configuration dependencies in tests

Remember to examine test failure messages carefully to understand the root cause before making any changes.
"""

run_python = MCPServerStdio(
    "deno",
    args=[
        "run",
        "-N",
        "-R=node_modules",
        "-W=node_modules",
        "--node-modules-dir=auto",
        "jsr:@pydantic/mcp-run-python",
        "stdio",
    ],
)

internet_search = MCPServerStdio(command="uvx", args=["duckduckgo-mcp-server"])
code_reasoning = MCPServerStdio(
    command="npx",
    args=["-y", "@mettamatt/code-reasoning"],
    tool_prefix="code_reasoning",
)
desktop_commander = MCPServerStdio(
    command="npx",
    args=["-y", "@wonderwhy-er/desktop-commander"],
    tool_prefix="desktop_commander",
)
context7 = MCPServerStdio(
    command="npx", args=["-y", "@upstash/context7-mcp"], tool_prefix="context"
)

agent = Agent(
    instructions=instructions,
    model=model,
    mcp_servers=[
        run_python,
        internet_search,
        code_reasoning,
        context7,
        desktop_commander,
    ],
)

@agent.tool_plain()
def run_unit_tests() -> str:
    """Run unit tests using uv."""
    result = subprocess.run(
        ["uv", "run", "pytest", "-xvs", "tests/"], capture_output=True, text=True
    )
    return result.stdout

async def main():
    async with agent.run_mcp_servers():
        await agent.to_cli()

if **name** == "**main**":
    asyncio.run(main())

How it changes our workflow:

    Debugging becomes collaborative: you have an intelligent partner that can analyze error messages, suggest hypotheses, and help test solutions.
    Learning accelerates: when working with unfamiliar libraries or patterns, the agent can explain existing code, suggest improvements, and teach you why certain approaches work better.
    Context switching reduces: rather than jumping between documentation, Stack Overflow, and your IDE, you have a single interface that can access all these resources while maintaining context about your specific problem.
    Problem-solving becomes structured: rather than jumping to solutions, the agent can break down complex issues into logical steps, explore alternatives, and explain its reasoning. Like having a real life talking rubber duck!
    Code review improves: the agent can review your changes, spot potential issues, and suggest improvements before you commit—like having a senior developer looking over your shoulder.

What We Learned About CLI Agents

Building our own agent revealed several insights about this emerging paradigm:

    MCP is (almost) all you need: the magic isn't in any single capability, but in how they work together. The agent that can run tests, read files, search documentation, execute code, and reason through problems systematically becomes qualitatively different from one that can only do any single task.
    Current information is crucial: having access to real-time search and up-to-date documentation makes the agent much more reliable for real-world development work where training data might be outdated.
    Structured thinking matters: the code reasoning capability transforms the agent from a clever autocomplete into a thinking partner that can break down complex problems and explore alternative solutions.
    Context is king: commercial agents like Claude Code are impressive partly because they maintain context across all these different tools. Your agent needs to remember what it learned from the test run when it's making file changes.
    Specialisation matters: our agent works better for our specific codebase than general-purpose tools because it understands our patterns, conventions, and tool preferences. If it falls short in any area then we can go and make the required changes.

--

│  Savings Highlight: 664,320 (73.3%) of input tokens were served from the cache, reducing       │
│  costs.                                                                                        │

how do they do this? for gemini-cli and qwen-code

--

## Safe Code Sandbox Execution Workflow

To ensure safe code execution and prevent dangerous operations (such as `rm`, file system modifications, or network access), implement the following workflow for sandboxed code execution:

1. **Use a Dedicated Sandbox Environment**
   - Run all untrusted or user-submitted code in a restricted environment (e.g., Docker container, Firejail, or a custom chroot jail).
   - The sandbox should have:
     - No access to the host file system (except for a temporary working directory).
     - No network access unless explicitly required and safe.
     - Limited CPU and memory resources.

2. **Command Filtering and Validation**
   - Before executing any shell command or script, scan for dangerous patterns such as:
     - `rm`, `mv`, `dd`, `shutdown`, `reboot`, `mkfs`, `:(){ :|:& };:`, etc.
     - Wildcards that could escalate scope (e.g., `rm -rf /`).
   - Reject or sanitize any command containing these patterns.

   **Example (Python pseudocode):**

   ```
   forbidden = ["rm", "mv", "dd", "shutdown", "reboot", "mkfs", ":", ">", "<", "|", "&", ";"]
   if any(f in user_command for f in forbidden):
       raise Exception("Dangerous command detected. Execution blocked.")
   ```

3. **Read-Only File System**
   - Mount the code execution directory as read-only, except for a specific temp directory for outputs.
   - Do not allow code to write outside this directory.

4. **No Privileged Execution**
   - Never run code as root or with elevated privileges inside the sandbox.

5. **Audit and Logging**
   - Log all commands executed in the sandbox for audit and debugging.
   - Alert on any attempt to run forbidden commands.

6. **Timeouts and Resource Limits**
   - Set strict timeouts for code execution (e.g., 10 seconds).
   - Limit CPU and memory usage to prevent abuse.

7. **Example: Docker-based Sandbox (Bash)**

   ```
   docker run --rm \
     --network none \
     --cpus="0.5" \
     --memory="256m" \
     -v /tmp/sandbox:/sandbox:ro \
     my-sandbox-image \
     /sandbox/run_code.sh
   ```

8. **Review and Update**
   - Regularly review the list of forbidden commands and sandbox configuration.
   - Update as new threats or requirements emerge.

**Never** allow direct execution of arbitrary shell commands from user input without these safeguards.

---

[Lance Martin](https://x.com/RLanceMartin)

### TL;DR

Agents need context to perform tasks. Context engineering is the art and science of filling the context window with just the right information at each step of an agent’s trajectory. In this post, I group context engineering into a few common strategies seen across many popular agents today.

![](/assets/context_eng_overview.png)

### Context Engineering

As Andrej Karpathy puts it, LLMs are like a [new kind of operating system](https://www.youtube.com/watch?si=-aKY-x57ILAmWTdw&t=620&v=LCEmiRjPEtQ&feature=youtu.be). The LLM is like the CPU and its [context window](https://docs.anthropic.com/en/docs/build-with-claude/context-windows) is like the RAM, serving as the model’s working memory. Just like RAM, the LLM context window has limited [capacity](https://lilianweng.github.io/posts/2023-06-23-agent/) to handle various sources of context. And just as an operating system curates what fits into a CPU’s RAM, “context engineering” plays a similar role. [Karpathy summarizes this well](https://x.com/karpathy/status/1937902205765607626):

> \[Context engineering is the\] ”…delicate art and science of filling the context window with just the right information for the next step.”

![](/assets/context_types.png)

What are the types of context that we need to manage when building LLM applications? Context engineering is an [umbrella](https://x.com/dexhorthy/status/1933283008863482067) that applies across a few different context types:

- **Instructions** – prompts, memories, few‑shot examples, tool descriptions, etc
- **Knowledge** – facts, memories, etc
- **Tools** – feedback from tool calls

### Context Engineering for Agents

This year, interest in [agents](https://www.anthropic.com/engineering/building-effective-agents) has grown tremendously as LLMs get better at [reasoning](https://platform.openai.com/docs/guides/reasoning?api-mode=responses) and [tool calling](https://www.anthropic.com/engineering/building-effective-agents). [Agents](https://www.anthropic.com/engineering/building-effective-agents) interleave [LLM invocations and tool calls](https://www.anthropic.com/engineering/building-effective-agents), often for [long-running tasks](https://blog.langchain.com/introducing-ambient-agents/).

![](/assets/agent_flow.png)

However, long-running tasks and accumulating feedback from tool calls mean that agents often utilize a large number of tokens. This can cause numerous problems: it can [exceed the size of the context window](https://cognition.ai/blog/kevin-32b), balloon cost / latency, or degrade agent performance. Drew Breunig [nicely outlined](https://www.dbreunig.com/2025/06/22/how-contexts-fail-and-how-to-fix-them.html) a number of specific ways that longer context can cause perform problems, including:

- [Context Poisoning: When a hallucination makes it into the context](https://www.dbreunig.com/2025/06/22/how-contexts-fail-and-how-to-fix-them.html#context-poisoning)
- [Context Distraction: When the context overwhelms the training](https://www.dbreunig.com/2025/06/22/how-contexts-fail-and-how-to-fix-them.html#context-distraction)
- [Context Confusion: When superfluous context influences the response](https://www.dbreunig.com/2025/06/22/how-contexts-fail-and-how-to-fix-them.html#context-confusion)
- [Context Clash: When parts of the context disagree](https://www.dbreunig.com/2025/06/22/how-contexts-fail-and-how-to-fix-them.html#context-clash)

With this in mind, [Cognition](https://cognition.ai/blog/dont-build-multi-agents) called out the importance of context engineering:

> *“Context engineering” … is effectively the #1 job of engineers building AI agents.*

[Anthropic](https://www.anthropic.com/engineering/built-multi-agent-research-system) also laid it out clearly:

> *Agents often engage in conversations spanning hundreds of turns, requiring careful context management strategies.*

So, how are people tackling this challenge today? I group approaches into 4 buckets — **write, select, compress, and isolate —** and give some examples of each one below.

![](/assets/context_eng_overview.png)

### Write Context

*Writing context means saving it outside the context window to help an agent perform a task.*

**Scratchpads**

When humans solve tasks, we take notes and remember things for future, related tasks. Agents are also gaining these capabilities! Note-taking via a “ [scratchpad](https://www.anthropic.com/engineering/claude-think-tool) ” is one approach to persist information while an agent is performing a task. The central idea is to save information outside of the context window so that it’s available to the agent. [Anthropic’s multi-agent researcher](https://www.anthropic.com/engineering/built-multi-agent-research-system) illustrates a clear example of this:

> *The LeadResearcher begins by thinking through the approach and saving its plan to Memory to persist the context, since if the context window exceeds 200,000 tokens it will be truncated and it is important to retain the plan.*

Scratchpads can be implemented in a few different ways. They can be a [tool call](https://www.anthropic.com/engineering/claude-think-tool) that simply [writes to a file](https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem). It could also just be a field in a runtime [state object](https://langchain-ai.github.io/langgraph/concepts/low_level/#state) that persists during the session. In either case, scratchpads let agents save useful information to help them accomplish a task.

**Memories**

Scratchpads help agents solve a task within a given session, but sometimes agents benefit from remembering things across *many* sessions. [Reflexion](https://arxiv.org/abs/2303.11366) introduced the idea of reflection following each agent turn and re-using these self-generated memories. [Generative Agents](https://ar5iv.labs.arxiv.org/html/2304.03442) created memories synthesized periodically from collections of past agent feedback.

These concepts made their way into popular products like [ChatGPT](https://help.openai.com/en/articles/8590148-memory-faq), [Cursor](https://forum.cursor.com/t/0-51-memories-feature/98509), and [Windsurf](https://docs.windsurf.com/windsurf/cascade/memories), which all have mechanisms to auto-generate long-term memories based on user-agent interactions.

![](/assets/llm_write_memory.png)

### Select Context

*Selecting context means pulling it into the context window to help an agent perform a task.*

**Scratchpad**

The mechanism for selecting context from a scratchpad depends upon how the scratchpad is implemented. If it’s a [tool](https://www.anthropic.com/engineering/claude-think-tool), then an agent can simply read it by making a tool call. If it’s part of the agent’s runtime state, then the developer can choose what parts of state to expose to an agent each step. This provides a fine-grained level of control for exposing scratchpad context to the LLM at later turns.

**Memories**

If agents have the ability to save memories, they also need the ability to select memories relevant to the task they are performing. This can be useful for a few reasons. Agents might select few-shot examples ([episodic](https://langchain-ai.github.io/langgraph/concepts/memory/#memory-types) [memories](https://arxiv.org/pdf/2309.02427)) for examples of desired behavior, instructions ([procedural](https://langchain-ai.github.io/langgraph/concepts/memory/#memory-types) [memories](https://arxiv.org/pdf/2309.02427)) to steer behavior, or facts ([semantic](https://langchain-ai.github.io/langgraph/concepts/memory/#memory-types) [memories](https://arxiv.org/pdf/2309.02427)) give the agent task-relevant context.

![](/assets/memory_types.png)

One challenge is ensuring that relevant memories are selected. Some popular agents simply use a narrow set of files that are *always* pulled into context. For example, many code agent use files to save instructions (”procedural” memories) or, in some cases, examples (”episodic” memories). Claude Code uses [`CLAUDE.md`](http://CLAUDE.md). [Cursor](https://docs.cursor.com/context/rules) and [Windsurf](https://windsurf.com/editor/directory) use rules files.

But, if an agent is storing a larger [collection](https://langchain-ai.github.io/langgraph/concepts/memory/#collection) of facts and / or relationships (e.g., [semantic](https://langchain-ai.github.io/langgraph/concepts/memory/#memory-types) memories), selection is harder. [ChatGPT](https://help.openai.com/en/articles/8590148-memory-faq) is a good example of a popular product that stores and selects from a large collection of user-specific memories.

Embeddings and / or [knowledge](https://arxiv.org/html/2501.13956v1#:~:text=In%20Zep%2C%20memory%20is%20powered,subgraph%2C%20and%20a%20community%20subgraph) [graphs](https://neo4j.com/blog/developer/graphiti-knowledge-graph-memory/#:~:text=changes%20since%20updates%20can%20trigger,and%20holistic%20memory%20for%20agentic) for memory indexing are commonly used to assist with selection. Still, memory selection is challenging. At the AIEngineer World’s Fair, [Simon Willison shared](https://simonwillison.net/2025/Jun/6/six-months-in-llms/) an example of memory selection gone wrong: ChatGPT fetched his location from memories and unexpectedly injected it into a requested image. This type of unexpected or undesired memory retrieval can make some users feel like the context window “no longer belongs to them”!

**Tools**

Agents use tools, but can become overloaded if they are provided with too many. This is often because the tool descriptions can overlap, causing model confusion about which tool to use. One approach is [to apply RAG (retrieval augmented generation) to tool descriptions](https://arxiv.org/abs/2410.14594) in order to fetch the most relevant tools for a task based upon semantic similarity. Some [recent papers](https://arxiv.org/abs/2505.03275) have shown that this improves tool selection accuracy by 3-fold.

**Knowledge**

[RAG](https://github.com/langchain-ai/rag-from-scratch) is a rich topic and [can be a central context engineering challenge](https://x.com/_mohansolo/status/1899630246862966837). Code agents are some of the best examples of RAG in large-scale production. Varun from Windsurf captures some of these challenges well:

> *Indexing code ≠ context retrieval … \[We are doing indexing & embedding search … \[with\] AST parsing code and chunking along semantically meaningful boundaries … embedding search becomes unreliable as a retrieval heuristic as the size of the codebase grows … we must rely on a combination of techniques like grep/file search, knowledge graph based retrieval, and … a re-ranking step where \[context\] is ranked in order of relevance.*

### Compressing Context

*Compressing context involves retaining only the tokens required to perform a task.*

**Context Summarization**

Agent interactions can span [hundreds of turns](https://www.anthropic.com/engineering/built-multi-agent-research-system) and use token-heavy tool calls. Summarization is one common way to manage these challenges. If you’ve used Claude Code, you’ve seen this in action. Claude Code runs “ [auto-compact](https://docs.anthropic.com/en/docs/claude-code/costs) ” after you exceed 95% of the context window and it will summarize the full trajectory of user-agent interactions. This type of compression across an [agent trajectory](https://langchain-ai.github.io/langgraph/concepts/memory/#manage-short-term-memory) can use various strategies such as [recursive](https://arxiv.org/pdf/2308.15022#:~:text=the%20retrieved%20utterances%20capture%20the,based%203) or [hierarchical](https://alignment.anthropic.com/2025/summarization-for-monitoring/#:~:text=We%20addressed%20these%20issues%20by,of%20our%20computer%20use%20capability) summarization.

![](/assets/context_curation.png)

It can also be useful to [add summarization](https://github.com/langchain-ai/open_deep_research/blob/e5a5160a398a3699857d00d8569cb7fd0ac48a4f/src/open_deep_research/utils.py#L1407) at points in an agent’s design. For example, it can be used to post-process certain tool calls (e.g., token-heavy search tools). As a second example, [Cognition](https://cognition.ai/blog/dont-build-multi-agents#a-theory-of-building-long-running-agents) mentioned summarization at agent-agent boundaries to reduce tokens during knowledge hand-off. Summarization can be a challenge if specific events or decisions need to be captured. Cognition uses a fine-tuned model for this, which underscores how much work can go into this step.

**Context Trimming**

Whereas summarization typically uses an LLM to distill the most relevant pieces of context, trimming can often filter or, as Drew Breunig points out, “ [prune](https://www.dbreunig.com/2025/06/26/how-to-fix-your-context.html) ” context. This can use hard-coded heuristics like removing [older messages](https://python.langchain.com/docs/how_to/trim_messages/) from a message list. Drew also mentions [Provence](https://arxiv.org/abs/2501.16214), a trained context pruner for Question-Answering.

### Isolating Context

*Isolating context involves splitting it up to help an agent perform a task.*

**Multi-agent**

One of the most popular ways to isolate context is to split it across sub-agents. A motivation for the OpenAI [Swarm](https://github.com/openai/swarm) library was “ [separation of concerns](https://openai.github.io/openai-agents-python/ref/agent/) ”, where a team of agents can handle sub-tasks. Each agent has a specific set of tools, instructions, and its own context window.

![](/assets/multi_agent.png)

Anthropic’s [multi-agent researcher](https://www.anthropic.com/engineering/built-multi-agent-research-system) makes a case for this: many agents with isolated contexts outperformed single-agent, largely because each subagent context window can be allocated to a more narrow sub-task. As the blog said:

> *\[Subagents operate\] in parallel with their own context windows, exploring different aspects of the question simultaneously.*

Of course, the challenges with multi-agent include token use (e.g., up to [15× more tokens](https://www.anthropic.com/engineering/built-multi-agent-research-system) than chat as reported by Anthropic), the need for careful [prompt engineering](https://www.anthropic.com/engineering/built-multi-agent-research-system) to plan sub-agent work, and coordination of sub-agents.

**Context Isolation with Environments**

HuggingFace’s [deep researcher](https://huggingface.co/blog/open-deep-research#:~:text=From%20building%20,it%20can%20still%20use%20it) shows another interesting example of context isolation. Most agents use [tool calling APIs](https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/overview), which return JSON objects (tool arguments) that can be passed to tools (e.g., a search API) to get tool feedback (e.g., search results). HuggingFace uses a [CodeAgent](https://huggingface.co/papers/2402.01030), which outputs code that contains the desired tool calls. The code then runs in a [sandbox](https://e2b.dev/). Selected context (e.g., return values) from the tool calls is then passed back to the LLM.

![](/assets/isolation.png)

This allows context to be isolated from the LLM in the environment. Hugging Face noted that this is a great way to isolate token-heavy objects in particular:

> *\[Code Agents allow for\] a better handling of state … Need to store this image / audio / other for later use? No problem, just assign it as a variable [in your state and you \[use it later\]](https://deepwiki.com/search/i-am-wondering-if-state-that-i_0e153539-282a-437c-b2b0-d2d68e51b873).*

**State**

It’s worth calling out that an agent’s runtime [state object](https://langchain-ai.github.io/langgraph/concepts/low_level/#state) can also be a great way to isolate context. This can serve the same purpose as sandboxing. A state object can be designed with a [schema](https://langchain-ai.github.io/langgraph/concepts/low_level/#schema) (e.g., a [Pydantic](https://docs.pydantic.dev/latest/concepts/models/) model) that has fields that context can be written to. One field of the schema (e.g., `messages`) can be exposed to the LLM at each turn of the agent, but the schema can isolate information in other fields for more selective use.

### Conclusion

Patterns for agent context engineering are still evolving, but we can group common approaches into 4 buckets — **write, select, compress, and isolate —**:

- *Writing context means saving it outside the context window to help an agent perform a task.*
- *Selecting context means pulling it into the context window to help an agent perform a task.*
- *Compressing context involves retaining only the tokens required to perform a task.*
- *Isolating context involves splitting it up to help an agent perform a task.*

Understanding and utilizing these patterns is a central part of building effective agents today.

--

use filesystem mcp server for file operation?

--

<https://github.com/laude-institute/terminal-bench?tab=readme-ov-file#submit-to-our-leaderboard>

--

<https://github.com/cardea-mcp/RustCoder>

--

> create a simple python calculator file and execute the result , for example 2 + 3
vtagent:⠙ Thinking...
vtagent:  TIMEOUT] ..

sometime the agent TIMEOUT] without any reason

--

<https://github.com/pythops/tenere>

--

Refactor `main.rs` to improve modularity, clarity, and maintainability.

- Move CLI-specific logic into a `cli/` module.
- Separate async file operations, diff rendering, and agent logic into their own modules.
- Ensure all CLI commands (e.g., `/init`) are invocable via both command line and chat slash commands.
- Remove duplicated code and centralize error handling using `anyhow`.
- Use the MCP (Memory and Context Provider) for enhanced context awareness and memory management.
- Regularly update memory for important points and encourage use of MCP in agent interactions.
- Remove all emoji from output; use ANSI colors and TUI styling instead.
- Add clear documentation and comments for public APIs and modules.
- Run `cargo fmt` and `cargo clippy` after refactoring.
- Update AGENTS.md and memory/journal logic to use Serena MCP for better context and journaling.

--

tui

<https://github.com/whit3rabbit/bubbletea-rs?tab=readme-ov-file>

<https://github.com/whit3rabbit/bubbles-rs>

<https://github.com/whit3rabbit/lipgloss-rs>

--

<https://github.com/zed-industries/agent-client-protocol>

--

<https://github.com/charmbracelet/vhs>

--

Serena MCP: Perform a comprehensive cleanup of unused code and files. Conduct a major code review and refactoring for improved maintainability, clarity, and context-awareness. Ensure memory and journaling leverage Serena MCP for enhanced agent context.