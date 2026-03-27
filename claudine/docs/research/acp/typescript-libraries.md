---
prompt: |-
    The ACP protocol sits on top of JSON-RPC and provides an open-source standard (spearheaded by the Zed and JetBrains teams) that does for AI coding agents what the Language Server Protocol (LSP) did for language tooling.

    Your task is to do research into who which Typescript libraries support the use of the ACP protocol. For each library found:

    - Name of the library
    - URL (the primary URL for the software)
    - How well does this library cover the uses cases typically associated with the ACP protocol?
    - Which libraries are most commonly compared to this library? How do they compare?

    After detailing all the libraries which cater to the ACP protocol, discuss how you might approach a bespoke/custom build instead of using one of these packages.

    - list out when you recommend using one of the crates found
    - list out when you recommend building a bespoke solution for ACP

    ## Frontmatter:

    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)

    ## Research

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
update_policy:
    - Duration(3 mo)
---

## TypeScript ACP library landscape (as of 2026-02-21)

The official ACP docs currently list one first-party TypeScript SDK and one community React library. In practice, the TypeScript ecosystem also includes adapter and bridge libraries that make ACP usable in AI SDK and editor workflows.

### 1. `@agentclientprotocol/sdk` (official ACP TypeScript SDK)

Name: `@agentclientprotocol/sdk`  
Maintainer: Agent Client Protocol project (Zed + JetBrains-led open-source effort)  
URL: https://agentclientprotocol.com/libraries/typescript  

Coverage of ACP use cases: High. This is the core library for implementing either side of ACP in TypeScript (`AgentSideConnection` and `ClientSideConnection`) and is the right default when you need protocol-level control, transport handling, and full customization.

Most common comparisons and how they compare:
- Compared with `use-acp`: `use-acp` is UI-focused (React + WebSocket) while this SDK is protocol/foundation-focused.
- Compared with `@mcpc-tech/acp-ai-provider`: the provider is easier for AI SDK app integration, while this SDK is better for direct ACP protocol implementations.
- Compared with Python/Rust/Kotlin ACP SDKs: feature intent is similar; pick TypeScript when your runtime is Node/browser and your toolchain is TS-first.

### 2. `use-acp` (community React hooks library)

Name: `use-acp`  
Maintainer: marimo-team  
URL: https://github.com/marimo-team/use-acp  

Coverage of ACP use cases: Medium. Strong for frontend ACP client workflows over WebSockets (connection state, notifications timeline, permission flow handling). It is not intended to be a full ACP server/agent implementation toolkit.

Most common comparisons and how they compare:
- Compared with `@agentclientprotocol/sdk`: faster for React UI integration; less flexible for low-level protocol work.
- Compared with custom React + raw WebSocket implementations: `use-acp` reduces boilerplate and provides ACP-aware primitives out of the box.

### 3. `@mcpc-tech/acp-ai-provider` / `@mcpc/acp-ai-provider` (AI SDK bridge)

Name: `@mcpc-tech/acp-ai-provider` (npm) / `@mcpc/acp-ai-provider` (JSR)  
Maintainer: mcpc-tech  
URL: https://ai-sdk.dev/providers/community-providers/acp  

Coverage of ACP use cases: Medium to high for AI SDK applications. It bridges ACP agents into Vercel AI SDK `LanguageModel` flows, including tool plumbing and process lifecycle. It is excellent when you want ACP agents behind AI SDK abstractions, but it is not a general-purpose ACP protocol SDK.

Most common comparisons and how they compare:
- Compared with `@agentclientprotocol/sdk`: quicker for AI SDK-based products; less control over ACP protocol internals.
- Compared with provider-native AI SDK integrations: ACP bridge gives agent portability across Claude/Codex/Gemini-style ACP agents, often with more setup complexity.

### 4. `@zed-industries/claude-agent-acp` (ACP adapter package)

Name: `@zed-industries/claude-agent-acp`  
Maintainer: Zed Industries  
URL: https://github.com/zed-industries/claude-code-acp  

Coverage of ACP use cases: High for “run Claude Code as an ACP agent.” It supports practical IDE agent workflows (tool calls, permissions, slash commands, terminals, MCP server integration), but it is adapter-specific rather than a general ACP toolkit.

Most common comparisons and how they compare:
- Compared with `@zed-industries/codex-agent-acp`: similar adapter role, different underlying model stack and auth/runtime behavior.
- Compared with building your own ACP adapter on `@agentclientprotocol/sdk`: this is faster to adopt; custom adapter gives deeper control over UX/policy/telemetry.

### 5. `@zed-industries/codex-agent-acp` (ACP adapter package)

Name: `@zed-industries/codex-agent-acp`  
Maintainer: Zed Industries  
URL: https://github.com/zed-industries/codex-acp  

Coverage of ACP use cases: High for “run Codex as an ACP agent.” It covers common agent UX features (tool calls, permissions, slash commands, MCP server support) for ACP clients, but like Claude’s adapter, it is not a general SDK.

Most common comparisons and how they compare:
- Compared with `@zed-industries/claude-agent-acp`: same adapter shape; choose based on preferred model provider, auth model, and operational constraints.
- Compared with implementing your own Codex ACP wrapper via `@agentclientprotocol/sdk`: this adapter wins on speed; bespoke wins on custom behavior and governance.

## Practical recommendation: library vs bespoke ACP build

Use one of the libraries above when:
- You need ACP integration quickly with minimal protocol engineering.
- You are building a React ACP client (`use-acp`) or AI SDK app (`@mcpc-tech/acp-ai-provider`).
- You want a ready-to-run agent adapter for Claude or Codex in ACP clients.
- You can accept the package’s abstractions and release cadence.

Build a bespoke ACP solution when:
- You need strict control over transport, permissions, security boundaries, audit logging, and policy enforcement.
- You need custom protocol extensions, domain-specific slash commands, or novel tool orchestration behavior.
- You have to support multi-tenant/runtime-specific constraints not well served by existing adapters.
- You need deterministic behavior across editor clients and want to own compatibility and conformance testing end-to-end.

## Suggested bespoke architecture (TypeScript)

Start from `@agentclientprotocol/sdk`, then layer:
- A transport/runtime boundary (`stdio` for local agent process, WebSocket for remote/browser paths).
- A capability gate for filesystem/terminal/tool permissions with explicit policy checks.
- A typed session state machine for init/session setup/prompt-turn lifecycles.
- Observability hooks (structured logs + traces) around tool calls, terminal events, and permission decisions.
- A conformance suite that replays ACP interaction traces across supported clients.

## Notes on scope

“ACP” is overloaded in the ecosystem. This document is specifically about the Agent Client Protocol at https://agentclientprotocol.com (not Agentic Commerce Protocol packages like `acp-handler`).
