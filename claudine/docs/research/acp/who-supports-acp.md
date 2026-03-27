---
prompt: |-
    The ACP protocol sits on top of JSON-RPC and provides an open-source standard (spearheaded by the Zed and JetBrains teams) that does for AI coding agents what the Language Server Protocol (LSP) did for language tooling.

    Your task is to do research into who supports ACP. For every library/software which is identified as supporting ACP detail out the following:

    - Name of the software / library (if library also indicate package manager and programming language)
    - URL (the primary URL for the software)
    - What kind of tool is this? Agent? Editor? etc.
    - What is the tool used for? What use cases does it solve for for this tool?

    ## Frontmatter:

    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)

    ## Research

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
update_policy:
    - Duration(3 mo)
---

# Who Supports ACP (Agent Client Protocol)?

The [Agent Client Protocol](https://agentclientprotocol.com/) is an open standard spearheaded by [Zed](https://zed.dev) and [JetBrains](https://www.jetbrains.com) that standardizes communication between code editors and AI coding agents over JSON-RPC. The [specification and SDKs](https://github.com/agentclientprotocol/agent-client-protocol) are open source.

## Editors / Clients

These tools act as ACP **clients** — they host and communicate with ACP-compatible agents.

| Name | URL | Type | Use Cases |
|------|-----|------|-----------|
| **Zed** | [zed.dev](https://zed.dev/docs/ai/external-agents) | Code Editor | Native ACP support for external agents. Real-time editing, syntax highlighting, agent-following. The originators of ACP. |
| **JetBrains IDEs** | [jetbrains.com](https://www.jetbrains.com/help/ai-assistant/acp.html) | Code Editor Suite | ACP support in IntelliJ IDEA, PyCharm, WebStorm, and all JetBrains IDEs (v2025.3+). ACP Agent Registry for one-click agent installation. |
| **Neovim** | [neovim.io](https://neovim.io) | Code Editor | ACP via plugins: [CodeCompanion.nvim](https://codecompanion.olimorris.dev/), [avante.nvim](https://github.com/yetone/avante.nvim), [agentic.nvim](https://github.com/schalkt/agentic.nvim). Brings AI agents into Vim-based workflows. |
| **Emacs** | [github.com/xenodium/agent-shell](https://github.com/xenodmin/agent-shell) | Code Editor | ACP integration via agent-shell.el plugin. Native Emacs shell for interacting with LLM agents. |
| **Visual Studio Code** | [github.com/formulahendry/vscode-acp](https://github.com/formulahendry/vscode-acp) | Code Editor | ACP extension for VS Code. Brings ACP agent ecosystem into the VS Code editor. |
| **Obsidian** | [obsidian.md](https://obsidian.md) | Note-Taking / Knowledge Management | ACP plugin adds side-panel agent chat. AI-assisted writing, content generation, and research workflows. |
| **marimo** | [marimo.io](https://marimo.io) | Python Notebook | Built-in ACP support in interactive Python notebook environment. AI-assisted data analysis and notebook authoring. |
| **AionUi** | [github.com/iOfficeAI/AionUi](https://github.com/iOfficeAI/AionUi) | Desktop Application | Free, local, open-source GUI app with ACP support. General-purpose agent interface. |
| **ACP UI** | [github.com/formulahendry/acp-ui](https://github.com/formulahendry/acp-ui) | Web Application | General-purpose ACP web interface for interacting with agents. |
| **Agent Studio** | [github.com/sxhxliang/agent-studio](https://github.com/sxhxliang/agent-studio) | Desktop Application | Dedicated agent development environment. |
| **Agmente** | [agmente.halliharp.com](https://agmente.halliharp.com) | Mobile App (iOS) | Native iOS ACP client for interacting with agents on mobile. |
| **aizen** | [aizen.win](https://aizen.win) | Web Application | Browser-based ACP client. |
| **Chrome ACP** | [github.com/Areo-Joe/chrome-acp](https://github.com/Areo-Joe/chrome-acp) | Browser Extension | Chrome browser integration for ACP agents. |
| **DeepChat** | [github.com/ThinkInAIXYZ/deepchat](https://github.com/ThinkInAIXYZ/deepchat) | Desktop Application | Chat interface for interacting with ACP agents. |
| **DuckDB ACP** | [github.com/sidequery/duckdb-acp](https://github.com/sidequery/duckdb-acp) | Database Extension | DuckDB extension enabling database access via ACP. |
| **Mitto** | [github.com/inercia/mitto](https://github.com/inercia/mitto) | Desktop Application | Data workflow tool with ACP integration. |
| **Sidequery** | [sidequery.dev](https://sidequery.dev) | Data Tool | Browser-based data development environment. ACP support coming soon. |
| **Tidewave** | [tidewave.ai](https://tidewave.ai) | Application | ACP-compatible development platform. |
| **Toad** | [batrachian.ai](https://www.batrachian.ai) | Application | Agent interface with ACP support. |

## Agents

These tools implement the ACP **agent** (server) side — they are AI-powered coding agents that can be used inside any ACP-compatible editor.

| Name | URL | Type | Use Cases |
|------|-----|------|-----------|
| **Claude Code** | [anthropic.com](https://docs.anthropic.com/en/docs/claude-code) | AI Coding Agent | Anthropic's Claude-powered coding agent. Code generation, refactoring, debugging, and autonomous development tasks. Works via Zed's SDK adapter. |
| **Gemini CLI** | [github.com/google-gemini/gemini-cli](https://github.com/google-gemini/gemini-cli) | AI Coding Agent | Google's reference ACP implementation. Deep codebase understanding, multimodal capabilities, code generation. |
| **GitHub Copilot** | [github.com/features/copilot](https://github.com/features/copilot) | AI Pair Programmer | GitHub's AI pair programmer. Code completion, suggestion, and generation. Currently in ACP public preview. |
| **Goose** | [block.github.io/goose](https://block.github.io/goose/docs/guides/acp-clients) | AI Coding Agent | Block's open-source agent with native ACP implementation. Autonomous coding, debugging, and development workflows. |
| **Codex CLI** | [openai.com](https://developers.openai.com/codex/cli) | AI Coding Agent | OpenAI's code generation agent. Streaming terminal output, code generation, and refactoring. Works via Zed's adapter. |
| **Augment Code (Auggie)** | [augmentcode.com](https://docs.augmentcode.com/cli/acp) | AI Coding Agent | Full-featured coding assistant optimized for large-scale refactors. Deep context engine for codebase understanding. |
| **Kiro CLI** | [kiro.dev](https://kiro.dev/docs/cli/acp/) | AI Coding Agent | Amazon/AWS coding agent. Runs `kiro-cli acp` for ACP mode. Supports slash commands, MCP tools, and session management. |
| **OpenCode** | [github.com/sst/opencode](https://github.com/sst/opencode) | AI Coding Agent | Community-driven, fully open-source coding agent. General-purpose coding assistance. |
| **Mistral Vibe** | [github.com/mistralai/mistral-vibe](https://github.com/mistralai/mistral-vibe) | AI Coding Agent | Mistral's lightweight, fast terminal-native coding agent. Built on Mistral's models. |
| **Qwen Code** | [github.com/QwenLM/qwen-code](https://github.com/QwenLM/qwen-code) | AI Coding Agent | Alibaba's open-source coding agent optimized for Qwen3-Coder. Strong multilingual support. |
| **Docker cagent** | [github.com/docker/cagent](https://github.com/docker/cagent) | Multi-Agent Runtime | Docker's open-source multi-agent runtime. Orchestrates AI agents using YAML configuration. Containerized agent execution. |
| **Cline** | [cline.bot](https://cline.bot/) | AI Coding Agent | Open-source AI coding agent for autonomous code analysis and modification. |
| **OpenHands** | [openhands.dev](https://docs.openhands.dev/openhands/usage/run-openhands/acp) | AI Development Platform | Community-driven AI development platform. Autonomous coding and development workflows. |
| **Blackbox AI** | [blackbox.ai](https://docs.blackbox.ai/features/blackbox-cli/introduction) | AI Coding Agent | Advanced CLI for AI-powered code generation and assistance. |
| **Factory Droid** | [factory.ai](https://factory.ai/) | AI Coding Agent | Specialized agent for automated code generation workflows. Available in JetBrains ACP Registry. |
| **JetBrains Junie** | [jetbrains.com/junie](https://www.jetbrains.com/junie/) | AI Coding Agent | JetBrains' own AI agent. Coming soon with native ACP support. |
| **Kimi CLI** | [github.com/MoonshotAI/kimi-cli](https://github.com/MoonshotAI/kimi-cli) | AI Coding Agent | Moonshot AI's CLI coding agent. |
| **fast-agent** | [fast-agent.ai](https://fast-agent.ai/acp) | Agent Framework | Create and interact with sophisticated agents and workflows. Framework for building custom ACP agents. |
| **Stakpak** | [github.com/stakpak/agent](https://github.com/stakpak/agent) | DevOps Agent | Open-source DevOps agent for infrastructure automation and deployment. |
| **AutoDev** | [github.com/phodal/auto-dev](https://github.com/phodal/auto-dev) | AI Coding Agent | Automated development agent for code generation and analysis. |
| **AgentPool** | [github.com/phil65/agentpool](https://phil65.github.io/agentpool/advanced/acp-integration/) | Agent Orchestration | Unified orchestration hub for managing heterogeneous AI agents. |
| **Code Assistant** | [github.com/stippi/code-assistant](https://github.com/stippi/code-assistant) | AI Coding Agent | AI assistant built in Rust for autonomous code analysis and modification. |
| **fount** | [github.com/steve02081504/fount](https://github.com/steve02081504/fount) | AI Agent | Multi-purpose AI agent with ACP support. |
| **Minion Code** | [github.com/femto/minion-code](https://github.com/femto/minion-code) | AI Coding Agent | Minion-based coding assistant. |
| **Pi** | [github.com/svkozak/pi-acp](https://github.com/svkozak/pi-acp) | AI Coding Agent | ACP adapter for running the Pi coding agent inside ACP-compatible editors. |
| **Qoder CLI** | [qoder.com](https://docs.qoder.com/cli/acp) | AI Coding Agent | Full-featured CLI coding agent with ACP support. |
| **VT Code** | [github.com/vinhnx/vtcode](https://github.com/vinhnx/vtcode) | AI Coding Agent | Open-source coding agent with semantic code intelligence. |

## Official SDKs

These are the official ACP SDK packages maintained by the [agentclientprotocol](https://github.com/agentclientprotocol) organization.

| Name | Language | Package Manager | URL | Description |
|------|----------|-----------------|-----|-------------|
| **@agentclientprotocol/sdk** | TypeScript | npm | [npm](https://www.npmjs.com/package/@agentclientprotocol/sdk) | Official TypeScript SDK for building ACP agents and clients. |
| **agent-client-protocol** | Rust | crates.io | [crates.io](https://crates.io/crates/agent-client-protocol) | Official Rust SDK for building ACP agents and clients. |
| **agent-client-protocol** | Python | pip / uv | [github](https://github.com/agentclientprotocol/python-sdk) | Official Python SDK with async base classes, stdio JSON-RPC plumbing, and lifecycle helpers. |
| **acp-kotlin** | Kotlin | Maven/Gradle | [github](https://github.com/agentclientprotocol/agent-client-protocol/tree/main/sdks/kotlin) | Official Kotlin SDK. Supports JVM; other targets in progress. |

## Community Libraries

Community-maintained ACP implementations across additional languages.

| Name | Language | URL | Description |
|------|----------|-----|-------------|
| **acp.el** | Emacs Lisp | [github.com/xenodmin/acp.el](https://github.com/xenodmin/acp.el) | ACP implementation in Emacs Lisp. |
| **acp-go-sdk** | Go | [github.com/coder/acp-go-sdk](https://github.com/coder/acp-go-sdk) | Go SDK for ACP, maintained by Coder. |
| **acp_dart** | Dart | [github.com/SkrOYC/acp-dart](https://github.com/SkrOYC/acp-dart) | Dart implementation of ACP. |
| **use-acp** | React (JS) | [github.com/marimo-team/use-acp](https://github.com/marimo-team/use-acp) | React hooks for ACP integration, by the marimo team. |
| **swift-acp** | Swift | [github.com/wiedymi/swift-acp](https://github.com/wiedymi/swift-acp) | Swift ACP implementation. |
| **swift-sdk** | Swift | [github.com/aptove/swift-sdk](https://github.com/aptove/swift-sdk) | Alternative Swift ACP SDK. |
| **acp-swift-sdk** | Swift | [github.com/rebornix/acp-swift-sdk](https://github.com/rebornix/acp-swift-sdk) | Third Swift ACP implementation. |
| **@zed-industries/agent-client-protocol** | TypeScript | npm | [npm](https://www.npmjs.com/package/@zed-industries/agent-client-protocol) | Zed's original TypeScript ACP adapter package. Used by agents like Claude Code and Codex CLI. |
