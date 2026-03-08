---
name: acp
description: Provides expertise on Agent Client Protocol (ACP). Describes the protocol, information on how to work with specific Agentic software, and libraries available to both the Rust and Typescript programming languages to act as a client in ACP.
prompt: |-
    Your task is to follow these steps exactly:

    1. ensure that this repo's `.claude/skills/acp` directory exists
    2. copy this document to `.claude/skills/acp/SKILL.md` (overwriting any prior version which may have been there)
    3. in the "copied" version you should remove the `prompt` frontmatter property
    4. now copy all of the Markdown document in @claudine/docs/acp to the .claude/skills/acp directory
    5. communicate to the user that the local repo-based "acp" skill has been created
    6. Now copy all files in .claude/skills/acp to the user scoped directory ~/.claude/skills/acp
    7. Now communicate that the user scoped "acp" skill has been created as well
last_updated: 2026-02-21
---

# Agent Client Protocol (ACP) Skill

## Overview

ACP is a bidirectional JSON-RPC protocol for connecting:

- a **Client** (usually an editor, IDE, or terminal UI), and
- an **Agent** (a coding assistant process that plans, edits, runs tools, and reports progress).

Conceptually:

- **LSP** standardized editor ↔ language-tooling interactions.
- **ACP** standardizes client ↔ coding-agent interactions.

It is transport-agnostic, but today the primary transport is newline-delimited JSON over stdio.

- for much more details on the ACP protocol read [What is ACP](./what-is-acp.md)


## Software Support for ACP

ACP has broad support across editors and agentic software platforms. While originally developed by the folks at [Zed](https://zed.dev/) and [Jetbrains](https://www.jetbrains.com/) it now the most common way for editors or other software to _interact_ with Agentic software.

For greater details read [Who Supports ACP](./who-supports-acp.md):
    - provides details on editors, agents, official SDK's, and community libraries

## ACP Libraries

For a deep dive into the most commonly used libraries to programmatically create an ACP client in Rust or Typescript follow the links below:

- [ACP Rust Libraries](./rust-crates.md)
- [ACP Typescript Libraries](./typescript-libraries.md)

## JSON-RPC

ACP it based on the JSON-RPC protocol which is a stateless, lightweight remote procedure call (RPC) protocol that uses JSON ([RFC 4627](https://www.ietf.org/rfc/rfc4627.txt)) as its data format.

- For more details on the JSON-RPC specification read: [JSON-RPC Details](./json-rpc.md)


## Agentic Code Examples

The following documents will provide examples of how a Rust program can work with specific Agentic software vendors using ACP:

1. [Kimi Code ACP](./kimi-code-cli.md)
2. [Gemini ACP](./gemini-cli.md)
3. [OpenCode ACP](./opencode.md)
4. [Qwen ACP](./quen-cli.md)
5. [Codex ACP](./codex-cli.md)

