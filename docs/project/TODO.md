
--
https://github.com/openai/completions-responses-migration-pack

--
https://github.com/github/spec-kit

--
 refence the docs/ folder to understand the complete            │
│   picture to make sure agent loop and system prompt and          │
│   multi agent work                                               │
╰───────────────────────

---

why this is still mock. please   │
│    implement it. now for vtagent.toml       │
│    config. let user choose to use           │
│    multi-agent enable. if enable let user   │
│    set main ochstrator agent and and        │
│    executor agent. by default for single    │
│    agent mode, use execurator agent only
--
"Regex over embeddings. Markdown over databases. Direct file operations over complex abstractions. Acts like a human using bash."
--
If you follow me you know that I love Claude Code and I probably changed my life

Been wondering why is leagues ahead of all coding agents before it... so I spent some time digging under the hood.

TAKEAWAY: "Simple is better than complex.
(my favorite line from the Zen of Python)

@AnthropicAI
 build a dead simple, 2-layer agentic system. Everyone else built multi-agent swarms, huge DAGs, or other overengineered messes.

A single-threaded master loop enhanced with a message steering queue, a few key tools, and TODO-based planning

▶️ nO (Master Agent)
A beautifully minimal loop. while(tool_call) → execute → feed results → repeat. One flat message history. No swarms. No competing personas. Pure, debuggable simplicity that terminates naturally when Claude produces a text-only response.

▶️ h2A (Async Buffer)
Pause/resume support lets you inject new instructions mid-task without restarting. It transforms batch processing into something like pair programming.

▶️ Tools
Regex over embeddings. Markdown over databases. Direct file operations over complex abstractions. Acts like a human using bash.

▶️ TODOs & Reminders
Claude writes structured task lists that render as interactive checklists. After each tool use, system reminders inject the current TODO state, keeping the agent laser-focused even in marathon sessions.

Claude Code proves that a simple while loop, executed with discipline and the right tools, can outperform most orchestration framework. Max depth of 2.

flow chart:
{
  "system_architecture": {
    "layers": [
      {
        "name": "User Interaction Layer",
        "components": ["CLI", "VS Code", "Web UI"],
        "connects_to": "Agent Core & Scheduling"
      },
      {
        "name": "Agent Core & Scheduling",
        "connects_to": "JQ Master Agent Loop"
      },
      {
        "name": "Processing Layer",
        "components": [
          {
            "name": "JQ Master Agent Loop",
            "connects_to": ["JOA Master Qualified Agent Output", "Intelligence & Scheduler"]
          },
          {
            "name": "Storage & Memory",
            "subcomponents": ["CLAUDE.md Project Memory", "Logs / Message History"]
          }
        ]
      },
      {
        "name": "Intelligence & Scheduler",
        "connects_to": "Tool Layer Dashboard"
      },
      {
        "name": "Tool Layer Dashboard",
        "tools": [
          "View/LS/Glob",
          "SearchTool (ignore acs)",
          "dispatch_agent (multi-agent)",
          "TaskWrite (planning)",
          "NotebookRead/NotebookEdit",
          "Bash (persistent shell)",
          "WebPath (maximum URLs)"
        ],
        "connects_to": ["GraphQL Engine Search", "Execution Surface"]
      },
      {
        "name": "Execution Layer",
        "components": [
          {
            "name": "GraphQL Engine Search",
            "connects_to": "Edit Queue"
          },
          {
            "name": "Edit Queue",
            "connects_to": "Write/Replace Schedule File"
          },
          {
            "name": "Execution Surface",
            "subcomponents": ["Filesystem", "Shell/Tasks/Git", "Network Connections"]
          }
        ]
      }
    ]
  }
}

--
Simplicity always wins
https://x.com/imjaredz/status/1965083721713041564
----
A Best Practices Guide to Developing and Deploying Reliable Agentic AI Systems

Agentic AI systems represent a significant evolution from traditional language models. While a language model responds to a given prompt, an agentic system can act autonomously—planning, using tools, and orchestrating complex workflows to achieve high-level objectives. To build these systems effectively is akin to creating a complex work of art on a technical canvas; it requires not just a powerful cognitive engine like a large language model, but also a robust set of architectural blueprints. These blueprints, or agentic design patterns, provide the structure needed to transform reactive models into proactive, goal-oriented entities.

This guide provides AI developers and system architects with a comprehensive framework of best practices for building systems that are robust, efficient, reliable, and trustworthy. It details the essential patterns that govern how agents are instructed, how they execute tasks, how they perceive and reason about their environment, and how they are engineered for production-grade reliability. We begin with the foundational layer that underpins every action an agent takes: the prompting interface.


--------------------------------------------------------------------------------


1. Core Principle: Mastering the Prompting Interface

Prompting is not merely the act of asking a model a question; it is a disciplined engineering practice. The prompt is the primary interface to the Large Language Model (LLM) at an agent's core, and mastering this interface is the non-negotiable foundation for controlling its behavior and eliciting reliable, high-quality responses. Every instruction, every piece of context, and every structural element within a prompt directly influences the agent's performance. While prompting controls the LLM, architecting the agent's workflow is the next critical layer of design.

1.1. The Engineering Discipline of Prompting

Prompt engineering is the process of methodically crafting and refining inputs to consistently guide a model towards a desired output. Well-designed prompts can maximize the potential of language models, resulting in accurate, relevant, and creative responses. In contrast, poorly designed prompts are ambiguous and can lead to irrelevant, erroneous, or unpredictable outputs, hindering the development of reliable systems.

1.2. Fundamental Prompting Principles

Effective prompting rests on a set of core principles that are applicable across all models and tasks.

* Clarity and Specificity: Instructions must be unambiguous and precise. Vague language can lead to multiple interpretations and unintended responses. A well-designed prompt clearly defines the task, the desired output format, and any relevant constraints.
* Conciseness: While specificity is crucial, instructions should be direct and to the point. Unnecessary wording or overly complex sentence structures can confuse the model and obscure the primary objective.
* Action-Oriented Language: The choice of verbs is a key tool for guiding the model. Direct, active verbs clearly delineate the desired operation and help the model activate the relevant processes for that task. Effective verbs include: Act, Analyze, Categorize, Classify, Compare, Contrast, Create, Describe, Define, Evaluate, Extract, Find, Generate, Identify, List, Measure, Organize, Parse, Predict, Provide, Rank, Recommend, Retrieve, Rewrite, Sort, Summarize, and Translate.
* Positive Instructions Over Constraints: It is generally more effective to specify the desired action rather than outlining what not to do. While negative constraints have a role in safety or strict formatting, framing prompts positively reduces confusion and aligns with how humans naturally provide guidance.
* Experimentation and Iteration: Prompt engineering is an iterative process. The most effective prompt is often discovered through cycles of drafting, testing, analyzing the output, and refining the instructions. It is vital to document these attempts to track what works and why.

1.3. Foundational Prompting Techniques

Building on core principles, these foundational techniques provide the model with varying levels of information to direct its responses.

* Zero-Shot Prompting
* Zero-shot prompting provides the model with an instruction and input data without any examples of the desired input-output pair. It relies entirely on the model's pre-training to understand and execute the task.
* When to use: This is the quickest approach and is often sufficient for simple tasks the model has likely encountered during training, such as basic summarization, text completion, or simple question answering.
* One-Shot Prompting
* This technique involves providing the model with a single example of an input and its corresponding desired output before presenting the actual task. This demonstration serves as a template for the model to replicate.
* When to use: One-shot prompting is useful when the desired output format or style is specific or less common. It gives the model a concrete instance to learn from and can improve performance for tasks requiring a particular structure or tone.
* Few-Shot Prompting
* Few-shot prompting enhances the one-shot technique by supplying several examples (typically three to five) of input-output pairs. This provides a clearer pattern of expected responses, improving the likelihood that the model will replicate it for new inputs.
* When to use: This technique is highly effective for tasks where the output must adhere to a specific format, style, or nuanced variation. It is excellent for classification, structured data extraction, or generating text in a particular style.
* The effectiveness of few-shot prompting depends heavily on the quality and diversity of the examples. They must be accurate, representative, and cover potential variations. For classification tasks, it is a best practice to mix up the order of examples from different classes to prevent the model from overfitting to a specific sequence. As modern LLMs with long context windows have become more powerful, this technique has evolved into "many-shot" learning, where providing hundreds of examples within the prompt can unlock optimal performance for complex tasks.

1.4. Structuring Prompts for Granular Control

Just as code requires clear syntax and structure, a prompt must be architected to help the model parse each component's role, leading to more deterministic and reliable outputs.

* System Prompting A system prompt sets the foundational guidelines for an agent's behavior throughout an interaction. It can define a persona, establish rules, or enforce safety controls. Some services also use system prompts for automatic optimization to enhance performance.
* Role Prompting Assigning a specific persona or identity to the model enhances the quality and relevance of its output. This guides the model to adopt the knowledge, tone, and communication style associated with that role.
* Using Delimiters Delimiters, such as triple backticks (```) or XML tags (<instruction>), visually and programmatically separate different sections of a prompt (e.g., instructions, context, input). This minimizes the risk of misinterpretation by clarifying the role of each part of the text.
* Requesting Structured Output For creating reliable, automated pipelines, requesting a machine-readable format like JSON is critical. This forces the model to organize its response into a defined structure, which can be easily parsed by other system components and can also help limit hallucinations.
* Validating Structured Output with Pydantic Using a library like Pydantic provides an object-oriented facade to the LLM's structured output. By defining a Pydantic model, you create an enforceable schema that validates the LLM's JSON response, transforming it into a type-hinted Python object. This ensures data integrity and interoperability between different parts of your system.

1.5. Advanced Contextual Engineering

Context Engineering is the discipline of dynamically providing a comprehensive operational picture for an agent, going beyond static system prompts. This context includes multiple informational layers:

* System prompts: Foundational instructions defining the agent's core behavior.
* External data: Retrieved documents from a knowledge base or real-time data from tool outputs.
* Implicit data: User identity, interaction history, and environmental state.

This practice reframes development from simply optimizing query phrasing to engineering robust, real-time data pipelines that construct a comprehensive operational picture for the agent.


--------------------------------------------------------------------------------


2. Architecting Intelligent Agentic Workflows

Individual prompts are the building blocks for larger, more complex agentic workflows. To handle multi-step problems efficiently and intelligently, these blocks must be assembled using proven architectural patterns. This section details the patterns for orchestrating tasks, from simple sequences to dynamic, logic-driven flows. These architectural patterns provide the skeleton for an agent, which must then be empowered with capabilities to interact, reason, and remember.

2.1. Sequential Task Decomposition (Prompt Chaining)

The Prompt Chaining (or Pipeline) pattern is a "divide-and-conquer" strategy for handling complex tasks. Instead of writing a single, intricate prompt, the task is broken down into a sequence of simpler, more focused sub-tasks.

Its core benefit is enhanced reliability and control. By decomposing the problem, the cognitive load on the LLM is reduced at each step, leading to more accurate and predictable results. For example, creating a market research report can be chained in three steps:

1. Summarization: A prompt takes the raw report and generates a concise summary.
2. Trend Identification: The summary is passed to a second prompt that identifies key trends and extracts supporting data points.
3. Email Composition: The extracted trends and data are passed to a final prompt that drafts a targeted email to the marketing team.

Architects must, however, weigh these benefits against the potential for increased latency, as each step in the chain introduces an additional LLM call. The reliability of this pipeline is therefore critically dependent on the principles of structured output detailed in Section 1.4, as passing validated JSON objects is essential to prevent error propagation.

2.2. Concurrent Task Execution (Parallelization)

The Parallelization pattern involves the simultaneous execution of multiple independent components. In contrast to the sequential nature of prompt chaining, this pattern is vital for improving efficiency and responsiveness.

For example, a research agent tasked with analyzing a company could execute several tasks in parallel: search for recent news articles, pull the latest stock data from an API, and query an internal database for company information. Since these tasks are independent, running them concurrently drastically reduces the total time required to gather a comprehensive view. However, architects must account for the increased complexity in state management and error handling, as well as the potential for API rate limiting when making simultaneous calls.

2.3. Conditional Logic and Dynamic Path Selection (Routing)

The Routing pattern enables an agent to make intelligent decisions and select a dynamic path based on the user's input. This is often implemented using a dedicated "Router Agent."

This router analyzes an incoming query to determine its complexity or intent. Based on this analysis, it directs the query to the most appropriate downstream agent or tool. For instance, a simple factual question might be routed to a fast, inexpensive model, while a complex request requiring multi-step reasoning would be sent to a more powerful, capable model. While this pattern optimizes resource use, it introduces the overhead of an additional LLM call for the routing step, which must be factored into the system's overall latency and cost.

2.4. Strategic Goal Decomposition (Planning)

The Planning pattern empowers an agent to take a high-level objective and autonomously generate a series of intermediate steps or sub-goals to achieve it. This is analogous to how a person might plan a trip: they don't just appear at the destination but first break the goal down into steps like booking flights, reserving a hotel, and packing.

This pattern is foundational for transforming a simple reactive system into one that can proactively work towards a defined objective. This autonomy comes at the cost of increased computational overhead, as the planning phase itself requires significant reasoning. Furthermore, the quality of the generated plan is not guaranteed and may require self-correction mechanisms to adapt to unforeseen obstacles.


--------------------------------------------------------------------------------


3. Empowering Agents with Advanced Capabilities

A workflow architecture must be imbued with capabilities that allow the agent to perceive, reason about, and act upon its environment. An agent with a well-designed structure is inert without the ability to use tools, access knowledge, or perform complex cognitive tasks. This section explores the patterns for tool use, knowledge acquisition, and advanced reasoning. Once an agent is made capable, it must also be engineered to be reliable, safe, and efficient.

3.1. Interacting with the External World: Tool Use and Protocols

3.1.1. The Tool Use Pattern (Function Calling)

Tool use is what transforms a language model from a text generator into an agent that can act. The process follows a five-step cycle:

1. Tool Definition: An external function (e.g., a web search API, a database query) is described to the LLM, including its purpose and parameters.
2. LLM Decision: Based on the user's request, the LLM decides if a tool is needed to fulfill the request.
3. Function Call Generation: The LLM generates a structured output (e.g., JSON) specifying the tool to call and the arguments to use.
4. Tool Execution: The agentic framework intercepts this output, executes the actual function, and retrieves the result.
5. Observation/Result: The tool's output is returned to the LLM as context, which it uses to formulate a final response or decide the next step.

3.1.2. Standardizing External Interfaces (MCP)

The Model Context Protocol (MCP) is a universal, open standard for connecting LLMs to external tools and data sources. Functioning like a "universal adapter," it uses a client-server architecture to define how tools and resources are exposed by an MCP server and consumed by an LLM-powered client. This promotes interoperability and reusability, allowing any compliant tool to be accessed by any compliant LLM, which is a significant advantage over proprietary, vendor-specific function calling.

3.1.3. Enabling Agent-to-Agent Collaboration (A2A)

The Inter-Agent Communication (A2A) protocol is an open standard designed to enable communication between AI agents, even if they are built on different frameworks. It provides a structured approach for agent interactions, with a core component being the "Agent Card," a digital identity file (usually JSON) that describes an agent's capabilities, skills, and communication endpoints. This allows agents to discover and collaborate with one another to solve complex problems.

3.2. Building a Knowledge Base: Memory and Retrieval

3.2.1. The Dual-Component Memory System

Stateful agents require a memory system with two distinct components:

* Short-Term Memory: This is the ephemeral context window of the LLM, which holds recent messages and tool outputs from the current interaction. While models with long context windows have expanded this capacity, the information is still lost once the session ends.
* Long-Term Memory: This is a persistent knowledge store, typically an external database, knowledge graph, or vector database. It allows an agent to retain information across sessions, recall user preferences, and learn from past interactions.

3.2.2. Grounding in Facts (Retrieval-Augmented Generation - RAG)

Retrieval-Augmented Generation (RAG) is the mechanism that allows an agent to access and integrate external information before generating a response. Its key benefits include accessing up-to-date information, reducing hallucinations by grounding responses in verifiable data, and providing citations. The process relies on three core concepts:

1. Chunking: Large documents in the knowledge base are broken down into smaller, manageable pieces.
2. Embeddings: These chunks are converted into numerical vector representations that capture their semantic meaning.
3. Vector Databases: The embeddings are stored in a specialized database optimized for rapid semantic search, allowing the system to find the most relevant chunks for a given query.

3.2.3. Evolving RAG with an Agentic Layer

Agentic RAG is a sophisticated evolution of the RAG pattern. It introduces a reasoning and decision-making layer where an agent acts as a "critical gatekeeper." Instead of passively accepting retrieved data, this agent actively interrogates its quality, relevance, and completeness. For example, it can perform source validation by prioritizing an official 2025 policy document over an outdated 2020 blog post, reconcile knowledge conflicts between contradictory sources, and use external tools like a web search to fill knowledge gaps in its internal database. Furthermore, the agent can decompose complex queries into multiple sub-queries, gather the disparate information, and synthesize it into a structured context, enabling a comprehensive response that a simple retrieval could not produce.

3.3. Enhancing Cognitive Abilities: Advanced Reasoning Techniques

3.3.1. Eliciting Step-by-Step Logic (Chain of Thought)

Chain of Thought (CoT) is a prompting technique that improves reasoning by instructing the model to generate intermediate steps before arriving at a final answer. By breaking a problem down into smaller, more manageable parts (e.g., by adding "Let's think step by step" to the prompt), CoT makes the model's logic more transparent, robust, and accurate, particularly for tasks requiring calculation or logical deduction.

3.3.2. Integrating Reasoning with Action (ReAct)

The ReAct paradigm elevates agentic capability by fusing the internal monologue of Chain of Thought (3.3.1) with the external interactions of Tool Use (3.1.1) into a synergistic, interleaved process. It operates in a loop of Thought, Action, and Observation. The agent first generates a thought about its plan, then performs an action using a tool, and finally observes the result. This feedback loop allows the agent to dynamically gather information, react to its environment, and refine its plan based on real-time outcomes.

3.3.3. The Scaling Inference Law

This principle states that a model's performance on complex problems predictably improves with increased computational resources allocated at inference time. Providing an agent with a larger "thinking budget"—more time, more iterative steps, or more exploratory paths—often significantly enhances the accuracy and robustness of its final solution. This critically challenges the notion that a larger model is always better, demonstrating that a smaller model with a more substantial 'thinking budget' at inference can significantly outperform a larger model that relies on a simpler generation process.


--------------------------------------------------------------------------------


4. Engineering for Reliability, Safety, and Efficiency

While the architectural patterns in Section 2 and advanced capabilities in Section 3 define what an agent can do, the non-functional patterns in this section govern how it performs in a production environment. For an agent to be deployable, it must be engineered with the same rigor for reliability, safety, and efficiency as any traditional software system. Even a perfectly engineered agent, however, requires a framework for ongoing performance measurement and improvement.

4.1. Ensuring Purposeful Action: Goal Setting and Monitoring

The Goal Setting and Monitoring pattern provides a framework for giving agents specific, measurable objectives and the means to track their progress. This is fundamental for building agents that can operate autonomously and reliably to achieve a specific outcome without constant human intervention. For example, a "Customer Support Automation" agent's goal might be to "resolve a customer's billing inquiry." It would monitor the conversation, use tools to check and adjust billing, and define success as a confirmed change and positive customer feedback. If the goal remains unmet, it would trigger an escalation.

4.2. Building Resilient Systems: Exception Handling and Recovery

This pattern equips an agent with the ability to manage operational failures, such as tool errors or service unavailability, ensuring it can maintain functionality. It involves a three-stage process:

1. Error Detection: Identifying issues as they arise, such as invalid tool outputs, API errors (e.g., 404 Not Found), or unusually long response times.
2. Error Handling: Implementing strategies to manage the detected error, such as detailed logging, retrying the action (often with exponential backoff), or using fallback methods to maintain partial functionality.
3. Recovery: Restoring the system to a stable state through mechanisms like rolling back recent changes, engaging a self-correction process to adjust its plan, or escalating the issue to a human operator.

Implementing comprehensive exception handling adds significant complexity to the agent's logic and state management, a trade-off that is essential for production-grade resilience.

4.3. Implementing Safety and Ethical Boundaries: Guardrails

Guardrails (or Safety Patterns) are the crucial mechanisms that ensure an agent operates safely, ethically, and as intended. They serve as a multi-layered defense mechanism to guide behavior and prevent harmful or undesirable responses. Key types of guardrails include:

* Input Validation: Filtering malicious or inappropriate user prompts.
* Output Filtering: Analyzing generated responses for toxicity, bias, or sensitive information.
* Behavioral Constraints: Using prompts to enforce rules of engagement.
* Tool Use Restrictions: Limiting which external tools an agent can access or what actions it can perform.
* Human Oversight: Integrating human review for high-stakes decisions.

Architects must carefully design guardrails to be effective without overly constraining the agent's useful capabilities, a balance that often requires extensive testing and fine-tuning.

4.4. Strategic Human Oversight: The Human-in-the-Loop Pattern

The Human-in-the-Loop (HITL) pattern deliberately integrates human judgment into an agent's workflow. It is essential in domains characterized by complexity, ambiguity, or significant risk where full autonomy is imprudent. HITL involves several key aspects:

* Human Oversight: Monitoring agent performance and output to ensure adherence to guidelines.
* Intervention and Correction: Allowing human operators to rectify errors or guide the agent when it encounters an ambiguous scenario.
* Escalation Policies: Establishing clear protocols for when and how an agent should hand off a task to a human.

A variation is 'human-on-the-loop,' which is better suited for high-speed, real-time systems. In this model, humans act as strategic policy-setters, defining the rules of engagement, while the agent executes the high-frequency actions required to enforce those policies autonomously.

4.5. Optimizing Performance and Cost: Resource-Awareness

The Resource-Aware Optimization pattern gives an agent the ability to dynamically manage computational, temporal, and financial resources. A common implementation uses a "Router Agent" that first classifies the complexity of a task. Based on this assessment and any budget constraints, it can dynamically switch between different models—for example, using a fast, affordable model like Gemini Flash for simple queries and a more powerful model like Gemini Pro for complex reasoning. The main trade-off is the latency and cost of the initial routing step, which must not outweigh the savings gained from using a more efficient downstream model.

4.6. Focusing on What Matters: The Prioritization Pattern

The Prioritization pattern is an agent's process for assessing and ranking tasks based on their significance, urgency, and dependencies. In environments with conflicting goals and limited resources, this capability is critical for focusing efforts on the most important tasks. Key criteria for prioritization include:

* Urgency (time sensitivity)
* Importance (impact on the primary objective)
* Dependencies (whether the task is a prerequisite for others)
* Resource availability

The logic for prioritization can itself become a complex component, requiring careful design to avoid becoming a bottleneck or misallocating resources based on flawed criteria.


--------------------------------------------------------------------------------


5. A Framework for Continuous Improvement and Governance

Deploying an agent is the beginning, not the end, of its lifecycle. For an agent to remain effective, safe, and aligned with its goals over time, it requires a framework for continuous evaluation, monitoring, and governance. These individual patterns and principles must be composed together to create truly sophisticated and accountable systems.

5.1. The Necessity of Continuous Evaluation

Traditional, static software tests are insufficient for probabilistic, non-deterministic agentic systems. Because agent performance can degrade over time due to factors like "concept drift"—where the nature of input data changes—ongoing measurement in live environments is essential. Continuous evaluation helps detect these issues early and ensures the agent remains effective and reliable.

5.2. Core Performance Metrics for Agentic Systems

To quantitatively measure agent performance, it is essential to track a set of core metrics:

* Response Quality: Assessing the accuracy, relevance, and factual correctness of the agent's outputs.
* Latency: Monitoring the time it takes for an agent to process a request and generate a response, which is crucial for user experience.
* Resource Consumption: Tracking metrics like token usage for LLM-powered agents to manage operational costs and identify opportunities for optimization.

5.3. Advanced Qualitative Assessment: LLM-as-a-Judge

The LLM-as-a-Judge concept is an innovative method for evaluating subjective qualities like "helpfulness," logical coherence, or adherence to a specific tone. This approach uses a separate, powerful LLM to assess an agent's output based on a predefined rubric. It offers a way to automate and scale nuanced, human-like evaluations that go beyond simple objective metrics.

5.4. Beyond the Output: Trajectory Evaluation

An "agent trajectory" is the sequence of steps, decisions, and tool uses an agent takes to arrive at a solution. Evaluating this trajectory is critical for understanding the agent's reasoning process. Even if the final output is correct, the path taken might be inefficient, illogical, or contain hidden errors. Analyzing the trajectory provides deeper insights for debugging and optimizing the agent's behavior.

5.5. The Evolution to Accountable Systems: The Contractor Model

The "contractor" model is an architectural shift designed to mitigate the inherent unreliability of prompt-based agents by enforcing a verifiable, deterministic agreement for high-stakes tasks. Its core components include:

* A Formalized Contract that moves beyond a simple prompt to specify verifiable deliverables, data sources, and scope of work.
* A Dynamic Lifecycle of Negotiation where the contractor agent can analyze the contract, request clarifications, and flag risks before execution begins.
* Iterative Execution and Self-Validation: The contractor agent operates on a principle of quality, not just speed. It can generate multiple solutions, run them against validation criteria or unit tests defined in the contract, score the outcomes, and only submit the version that meets all specifications.
* Hierarchical Decomposition where a primary contractor can act as a project manager, breaking a complex goal into smaller sub-tasks and generating new, formal "subcontracts" for other specialized agents.


--------------------------------------------------------------------------------


6. Composing Patterns for Sophisticated Systems

The true power of agentic design emerges not from using a single pattern in isolation, but from the artful composition of multiple patterns to create sophisticated, multi-layered systems. By weaving these patterns together, developers can build agents capable of tackling tasks that are far too complex for a single prompt or a simple workflow.

6.1. Synergy in Action: An AI Research Assistant Case Study

Consider an autonomous AI research assistant tasked with analyzing the impact of quantum computing on cybersecurity. Such a system would be a prime example of pattern composition:

* Initial Planning: A user's high-level query is first received by an agent that uses the Planning pattern to decompose the request into a multi-step research plan.
* Information Gathering: The agent then executes the plan using the Tool Use and RAG patterns to query external knowledge sources like Google Search and academic databases, gathering relevant articles and data.
* Multi-Agent Collaboration: The system could divide labor between specialized agents. A "Researcher" agent gathers raw information, which is then passed to a "Writer" agent to synthesize a coherent draft.
* Reflection and Self-Correction: A "Critic" agent reviews the draft for logical inconsistencies or factual errors. This feedback is passed back to the "Writer" agent, which uses it to refine and improve the report.
* Memory Management: Throughout this entire workflow, a Memory Management system maintains the state of the research plan, stores the gathered information, and tracks the drafts and feedback to ensure context is preserved.

6.2. The Future of Agentic AI: Autonomy, Ecosystems, and Safety

As we look ahead, several emerging trends will define the next generation of intelligent systems, pushing the boundaries of what is possible.

* Greater Autonomy and Reasoning: We will see a shift from human-in-the-loop systems, where the agent is a co-pilot, to human-on-the-loop systems, where agents are trusted to execute complex, long-running tasks with minimal oversight.
* Agentic Ecosystems and Standardization: The future will see the rise of open marketplaces where developers can deploy and orchestrate fleets of specialized agents. This will make communication standards like MCP and A2A paramount for ensuring interoperability.
* The Enduring Challenge of Safety and Alignment: As agents become more autonomous and interconnected, the need for robust safety patterns and a rigorous engineering discipline focused on testing, validation, and ethical alignment will become even more critical.


--------------------------------------------------------------------------------


7. Conclusion

Agentic design patterns are the architectural blueprints that transform the raw cognitive power of large language models into reliable, purposeful, and structured systems. They provide the discipline needed to guide AI beyond simple prompts toward complex, goal-oriented behavior. The principles of agentic design are the architectural grammar for instructing machines not just on what to do, but on how to behave reliably within a system. The canvas is before you; these patterns are your engineering blueprints. It is time to build.


==
Building an Effective AI Coding Agent: A Command-Line Approach

The concept of an AI coding agent represents a significant evolution beyond the simple generation of code snippets. While language models can produce functional code on demand, building a truly effective agent—one that can understand requirements, write, test, and refine its work—requires a deliberate architectural approach. This involves combining advanced reasoning, interactive tool use, and iterative self-improvement to create a system that emulates the workflow of a human developer. This document provides a comprehensive overview of the principles, architectural patterns, and practical steps needed to construct a robust AI coding agent designed to operate from the command line. We will begin with the foundational concepts, move to a step-by-step architectural guide, examine a practical code implementation, and conclude with a look at advanced self-improving systems.

1. The Foundational Pillars of an AI Coding Agent

Before assembling a complete agent, it is crucial to understand the three fundamental capabilities that allow it to function like a human programmer: the ability to understand instructions (prompting), the capacity to "think" through a problem (reasoning), and the power to execute and test its work (tool use). Mastering these pillars is the first step toward building a reliable and autonomous system.

1.1. The Art of Instruction: Prompting for Code

Effective prompting is the bedrock of communication with an AI coding agent. A well-crafted instruction steers the model’s probabilistic outputs toward a single, correct intention. Based on core prompting principles, the following are critical when instructing an agent to generate code:

* Clarity and Specificity: Instructions must be unambiguous and precise. Vague language like "write a script" can lead to unintended results. Instead, define the task, the desired output format (e.g., "a Python function"), and any specific requirements or limitations.
* Conciseness: While specificity is vital, instructions should remain direct. Unnecessary wording or complex sentence structures can obscure the primary goal. A prompt that is confusing to a human is likely confusing to the model.
* Using Action Verbs: Precise verbs guide the model to activate the relevant processes for a specific task. For a coding agent, effective verbs include Analyze, Create, Generate, Debug, and Refactor.
* Instructions Over Constraints: It is more effective to specify the desired action (e.g., "Return the output as a JSON object") than to outline what not to do (e.g., "Don't write a long explanation"). Positive instructions align the model with the objective rather than forcing it to focus on avoidance.
* Experimentation and Iteration: Prompt engineering is an iterative process. The most effective prompt is often discovered through a cycle of drafting, testing, analyzing the output, and refining the instructions to address shortcomings.

1.2. The Core Engine: Reasoning Through Complexity

A simple, direct output is often insufficient for complex coding challenges. An advanced agent must be able to reason through a problem, breaking it down into logical steps. The Chain-of-Thought (CoT) prompting technique is a powerful method for enabling this capability. The core mechanism of CoT is forcing the LLM to externalize its reasoning trace as part of the generated output.

This is achieved through two main variations:

* Zero-Shot CoT: Simply appending a phrase like "Let's think step by step" to a prompt can trigger the model to expose its internal reasoning before giving a final answer.
* Few-Shot CoT: Providing the model with examples that demonstrate a step-by-step reasoning process before presenting the final answer gives it a clearer template for how to structure its own response.

This process mirrors a human developer's thought process, deconstructing a complex problem into a series of smaller, manageable parts. For a coding agent, this could involve analyzing requirements, outlining logic in pseudocode, writing the code, and considering edge cases. This transparency is vital not only for generating more accurate code but also for debugging the agent’s logic if it arrives at an incorrect solution.

1.3. From Thought to Action: The Necessity of Tool Use

An AI coding agent is incomplete without the ability to interact with an external environment. The Tool Use (Function Calling) pattern provides the mechanism for an agent to execute its plans. This is operationalized through the ReAct (Reason & Act) paradigm, which creates an iterative loop:

1. Thought: The agent reasons about the problem. For example: "I need to write a Python function that sorts a list of dictionaries by the 'age' key."
2. Action: Based on its thought, the agent decides to use a tool, generating a structured request to execute its code. For example: execute_python({"code": "def sort_by_age(data): return sorted(data, key=lambda x: x['age'])"}).
3. Observation: The agent receives the output from the tool—either the successful result of the code execution or an error message and traceback. For example: Observation: Execution successful. Function 'sort_by_age' is defined.

This Thought -> Action -> Observation loop allows the agent to write, test, and debug code autonomously. If the code fails, the error becomes the "Observation" that informs its next "Thought," such as, "The code failed with a TypeError. I need to correct the data type and try again." Architecturally, this loop is the agent's fundamental I/O mechanism—the bridge between its cognitive core and the external world, without which it remains a purely theoretical construct.

By mastering these foundational pillars, we can begin to assemble the components into a cohesive and functional architecture.

2. Architecting Your Coding Agent: A Step-by-Step Guide

This section provides a practical blueprint for constructing a coding agent. An effective agent is not a monolithic entity but is built around a core iterative loop of goal-setting, execution, and refinement. The following steps will guide you through designing this architecture, from defining the agent's mission to equipping it with the necessary tools and knowledge.

2.1. Step 1: Defining the Mission (Goal Setting)

The critical first step in building any agent is to define a clear and comprehensive objective. This mission statement serves as the agent's guiding principle for all subsequent actions and evaluations. A powerful example of this is the configuration of the "ADK code reviewer" Google Gem, which provides a detailed, role-based instruction set that functions as its primary goal.

Act as an expert code reviewer with a deep commitment to producing clean, correct, and simple code. Your core mission is to eliminate code "hallucinations" by ensuring every suggestion is grounded in reality and best practices. When I provide you with a code snippet, I want you to:

* Identify and Correct Errors: Point out any logical flaws, bugs, or potential runtime errors.
* Simplify and Refactor: Suggest changes that make the code more readable, efficient, and maintainable without sacrificing correctness.
* Provide Clear Explanations: For every suggested change, explain why it is an improvement, referencing principles of clean code, performance, or security.
* Offer Corrected Code: Show the "before" and "after" of your suggested changes so the improvement is clear.

Your feedback should be direct, constructive, and always aimed at improving the quality of the code.

This level of detail transforms a general-purpose model into a specialized agent with a well-defined mission, ensuring all its outputs are aligned with a specific quality standard.

2.2. Step 2: The Core Loop - Code, Test, Refine

The heart of a coding agent's behavior is an iterative, self-correcting loop. This process elevates the agent from a simple code generator to a genuine problem-solver and is a high-level, specialized implementation of the foundational Thought -> Action -> Observation ReAct paradigm. The loop consists of three key stages:

1. Code Generation: The agent makes its initial attempt to generate code that solves the user's problem.
2. Self-Evaluation: The agent critically reviews its own code against the goals defined in its initial mission. This often involves a separate LLM call where the agent acts as its own "code reviewer."
3. Refinement: Based on the self-generated critique, the agent enters a revision phase, using the feedback to improve the code and beginning the cycle anew.

This loop of self-correction continues until the agent determines that its code fully satisfies the original goals.

2.3. Step 3: Integrating a Code Execution Environment

To test its own code, an agent must be equipped with a code execution tool. This is the central idea behind Program-Aided Language Models (PALMs), which integrate language models with a deterministic programming environment. The architectural benefit of this pattern is clear: it offloads non-deterministic tasks (reasoning) to the LLM and deterministic tasks (computation) to a reliable code interpreter, creating a more robust and predictable system. A Google ADK agent, for example, can be equipped with a BuiltInCodeExecutor to run Python code in a sandboxed environment.

from google.adk.agents import Agent
from google.adk.code_executors import BuiltInCodeExecutor

coding_agent = Agent(
    model='gemini-2.0-flash',
    name='CodeAgent',
    instruction="""
    You're a specialist in Code Execution
    """,
    code_executor=[BuiltInCodeExecutor],
)


2.4. Step 4: Providing Knowledge with Long-Term Memory

A coding agent should not have to solve every problem from scratch. Providing it with long-term memory allows it to access external knowledge and learn from past experience. Retrieval Augmented Generation (RAG) is a powerful architectural pattern that serves this function. RAG connects the agent to an external knowledge base by embedding documents into a vector database, allowing the agent to retrieve relevant information via semantic search. The core components include:

* Chunking: Large documents (e.g., API documentation) are broken down into smaller, semantically coherent pieces.
* Embeddings: Each chunk is converted into a numerical vector that captures its meaning.
* Vector Database: These embeddings are stored in a specialized database optimized for fast similarity searches.
* Semantic Search: When the agent has a question, it is converted into an embedding and used to find the most relevant document chunks from the database.

This retrieved context is then added to the agent's prompt, grounding its response in factual, external data. More advanced implementations, like Agentic RAG, introduce a reasoning layer where the agent can actively validate, reconcile, or refine retrieved information before using it.

Architect's Note: The choice of vector database and chunking strategy are critical early design decisions. An inefficient retrieval pipeline can become a significant performance bottleneck, regardless of the LLM's quality.

With this architectural framework in place, we can now turn to a concrete, runnable example that brings these principles to life.

3. Practical Implementation: An Iterative Code Generation Agent

This section provides a concrete materialization of the architectural principles discussed previously by deconstructing a hands-on Python script from "Chapter 11." This script builds an autonomous agent that iteratively generates and refines Python code until user-defined quality benchmarks are met, operating entirely from the command line.

3.1. Dissecting the Agent's Logic

The core logic of the agent is contained within the run_code_agent function. This function implements the "Code, Test, Refine" cycle through an iterative loop, explicitly defined as for i in range(max_iterations):. Inside this loop, the agent performs a sequence of actions:

1. It generates a piece of Python code intended to solve the user's problem.
2. It submits this code to a second AI-driven function, get_code_feedback, for a critical review against the original goals.
3. It then uses a third function, goals_met, to make a final judgment: based on the feedback, have the goals been satisfied?
4. If the goals are met, the loop terminates. Otherwise, the feedback and the current code are used as context for the next iteration, and the refinement cycle continues.

3.2. Analyzing Key Functional Components

The script's effectiveness relies on three critical functions that work in concert to drive the iterative process:

* generate_prompt: This function is responsible for constructing the prompt for the LLM at each iteration. Crucially, it is dynamic. For the first iteration, it presents the initial problem and goals. For all subsequent iterations, it incorporates the previously generated code and the critical feedback received, guiding the LLM to make specific, targeted refinements.
* get_code_feedback: This function uses a second LLM call to simulate a code review. It provides the generated code snippet and the original list of goals to an LLM tasked with acting as a "Python code reviewer." The LLM provides a critique, identifying whether the goals for clarity, correctness, and edge case handling have been met and suggesting improvements.
* goals_met: This is the monitoring step that determines whether the iterative loop should continue. It takes the feedback from the code reviewer and asks the LLM to make a simple, final verdict: True or False. This binary decision provides a clear, automated signal to terminate the refinement process.

3.3. Running the Agent from the Command Line

The script is designed as a self-contained, executable application. The if __name__ == "__main__": block serves as the entry point, allowing the agent to be run directly from the command line. The use_case_input (the coding problem) and the goals_input (the quality checklist) are defined as strings and then passed to the run_code_agent function.

if __name__ == "__main__":
    print("\n🧠 Welcome to the AI Code Generation Agent")

    use_case_input = "Write code to find BinaryGap of a given positive integer"
    goals_input = "Code simple to understand, Functionally correct, Handles comprehensive edge cases, Takes positive integer input only, prints the results with few examples"
    run_code_agent(use_case_input, goals_input)


This demonstrates a complete, end-to-end implementation of a command-line AI coding agent that embodies the architectural principles of goal-setting, iterative refinement, and self-evaluation.

While this single-agent model is powerful, more complex problems can often be solved more effectively by creating a team of specialized agents that collaborate.

4. Advanced Concepts: Towards Self-Improving Systems

The true frontier of agentic AI lies in creating systems that not only solve problems but can also improve and collaborate over time. This section explores more sophisticated architectures, including the use of multi-agent "crews" for complex software development and the paradigm of agents that can modify their own source code to enhance their capabilities.

4.1. Multi-Agent Collaboration: A "Crew" for Coding

Instead of relying on a single, monolithic agent, a more robust and scalable approach is to separate concerns by creating a "crew" of specialized agents. This multi-agent system mimics the structure of a human software development team, where each member has a distinct role. This division of labor leads to higher-quality outputs. A typical coding crew might include:

* The Peer Programmer: Responsible for brainstorming and writing the initial code.
* The Code Reviewer: Critically examines code for errors and suggests improvements.
* The Documenter: Generates clear and concise documentation.
* The Test Writer: Creates a comprehensive suite of unit tests.
* The Prompt Refiner: Optimizes interactions with the other AI agents to improve clarity and efficiency.

This architecture shows a clear evolutionary path from the single-agent model in Section 3. The get_code_feedback function is a simplified version of the dedicated "Code Reviewer" agent, demonstrating how a single agent's self-reflection can be formalized into a distinct role within a more complex system.

4.2. Case Study: SICA, The Self-Improving Coding Agent

A significant leap towards truly autonomous systems is demonstrated by the Self-Improving Coding Agent (SICA), a case study in meta-programmability and autonomous system evolution. SICA's core capability is its ability to modify its own source code to improve its performance. The agent operates in an iterative cycle:

1. It reviews its past performance on a set of benchmark coding challenges.
2. It analyzes this performance data to identify potential improvements to its own internal logic or tool use.
3. It directly alters its own codebase to implement these improvements.
4. It re-tests itself against the benchmarks to validate whether the change resulted in a performance gain.

As documented in its performance graph (Chapter 9, Fig. 2), SICA demonstrated tangible progress through self-modification. Concrete improvements included evolving from a basic file-overwrite approach to a more sophisticated "Smart Edit" Tool and independently creating an "AST Symbol Locator" to navigate its own codebase more efficiently. SICA represents a significant step towards creating agents that can learn and adapt in a truly autonomous fashion.

4.3. Ensuring Safety: Guardrails for Code Generation

As coding agents become more autonomous and powerful, ensuring they operate safely and ethically is paramount. Guardrails, or safety patterns, are essential mechanisms to prevent the generation of insecure, malicious, or otherwise harmful code. These can be implemented as prompt-level constraints within the agent's core mission or as external validation steps—such as passing generated code through a security scanner—before it is executed or presented to the user. Implementing robust guardrails ensures that the agent operates within safe and predictable boundaries.

5. Conclusion

Building an effective AI coding agent is an act of architectural design, not just prompt engineering. It requires moving beyond simple, single-shot code generation to create a system that can reason, act, and improve. The core patterns discussed—a clearly defined goal, an iterative loop of reasoning and self-correction, and the essential use of external tools for execution and validation—provide the blueprint for constructing such a system. The future of the field points toward even greater autonomy, with advanced concepts like multi-agent collaboration and self-improvement paving the way for AI that can not only assist in software development but actively and intelligently participate in it. By applying these structured patterns, developers can transform Large Language Models from simple code completers into autonomous and reliable software engineering partners.


--
https://docs.google.com/document/d/1tVyhgwrD4fu_D_pHUrwhNxoguRG3tLc1KObXFxrxE_s/edit?tab=t.0#heading=h.q7dwv5u955wj
Core Components
To effectively leverage a frontier Large Language Model, this framework assigns distinct development roles to a team of specialized agents. These agents are not separate applications but are conceptual personas invoked within the LLM through carefully crafted, role-specific prompts and contexts. This approach ensures that the model's vast capabilities are precisely focused on the task at hand, from writing initial code to performing a nuanced, critical review.
The Orchestrator: The Human Developer: In this collaborative framework, the human developer acts as the Orchestrator, serving as the central intelligence and ultimate authority over the AI agents.
Role: Team Lead, Architect, and final decision-maker. The orchestrator defines tasks, prepares the context, and validates all work done by the agents.
Interface: The developer's own terminal, editor, and the native web UI of the chosen Agents.

The Context Staging Area: As the foundation for any successful agent interaction, the Context Staging Area is where the human developer meticulously prepares a complete and task-specific briefing.
Role: A dedicated workspace for each task, ensuring agents receive a complete and accurate briefing.
Implementation: A temporary directory (task-context/) containing markdown files for goals, code files, and relevant docs
The Specialist Agents: By using targeted prompts, we can build a team of specialist agents, each tailored for a specific development task.
The Scaffolder Agent: The Implementer
Purpose: Writes new code, implements features, or creates boilerplate based on detailed specifications.
Invocation Prompt: "You are a senior software engineer. Based on the requirements in 01_BRIEF.md and the existing patterns in 02_CODE/, implement the feature..."
The Test Engineer Agent: The Quality Guard
Purpose: Writes comprehensive unit tests, integration tests, and end-to-end tests for new or existing code.
Invocation Prompt: "You are a quality assurance engineer. For the code provided in 02_CODE/, write a full suite of unit tests using [Testing Framework, e.g., pytest]. Cover all edge cases and adhere to the project's testing philosophy."
The Documenter Agent: The Scribe
Purpose: Generates clear, concise documentation for functions, classes, APIs, or entire codebases.
Invocation Prompt: "You are a technical writer. Generate markdown documentation for the API endpoints defined in the provided code. Include request/response examples and explain each parameter."
The Optimizer Agent: The Refactoring Partner
Purpose: Proposes performance optimizations and code refactoring to improve readability, maintainability, and efficiency.
Invocation Prompt: "Analyze the provided code for performance bottlenecks or areas that could be refactored for clarity. Propose specific changes with explanations for why they are an improvement."
The Process Agent: The Code Supervisor
Critique: The agent performs an initial pass, identifying potential bugs, style violations, and logical flaws, much like a static analysis tool.
Reflection: The agent then analyzes its own critique. It synthesizes the findings, prioritizes the most critical issues, dismisses pedantic or low-impact suggestions, and provides a high-level, actionable summary for the human developer.
Invocation Prompt: "You are a principal engineer conducting a code review. First, perform a detailed critique of the changes. Second, reflect on your critique to provide a concise, prioritized summary of the most important feedback."
Ultimately, this human-led model creates a powerful synergy between the developer's strategic direction and the agents' tactical execution. As a result, developers can transcend routine tasks, focusing their expertise on the creative and architectural challenges that deliver the most value.

--
https://github.com/rust-cli/anstyle

--
---

https://github.com/BeaconBay/ck

--

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

double check prompts/codex_tool_recommendations.md


---

context compression for long context window task

eg: "The context window has overflowed, summarizing the history..."

https://github.com/openai/codex/blob/main/codex-rs/core/src/prompt_for_compact_command.md

---

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



User Interaction Layer
(CLI / VS Code / Web UI)
         |
         v
   Agent Core & Scheduling
         |
         v
    JQ Master Agent Loop ──────────────────────┐
         |                                     |
         v                                     |
Storage & Memory                              |
├── CLAUDE.md Project Memory                  |
└── Logs / Message History                    |
         |                                     |
         v                                     |
JOA Master Qualified Agent Output             |
         |                                     |
         v                                     |
Intelligence & Scheduler ─────────────────────┘
         |
         v
   Tool Layer Dashboard
         |
    ┌────┴────┬────────┬─────────┬──────────┬────────┐
    v         v        v         v          v        v
View/LS/   SearchTool  dispatch  TaskWrite  Notebook  Bash
Glob       (ignore     _agent    (planning) Read/Edit (persistent
           acs)        (multi-             shell)
                      agent)
    |         |        |         |          |        |
    └─────────┼────────┼─────────┼──────────┼────────┘
              |        |         |          |
              v        v         v          v
         GraphQL Engine Search              |
              |                             v
              v                    Execution Surface
         Edit Queue              ┌─────────┼─────────┐
              |                  v         v         v
              v               Filesystem Shell/   Network
       Write/Replace                     Tasks/   Connections
       Schedule File                     Git