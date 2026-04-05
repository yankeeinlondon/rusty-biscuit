---
name: acp
description: Detailed information on the Agent Client Protocol (ACP), libraries to use in Rust and Typescript, background details on the underlying JSON-RPC standard. Also includes detailed strategies for interacting with claude code, codex, kimi-code, opencode, gemini-cli, and other Agentic CLI providers.
argument-hint: "[--deploy]"

prompt: |-
    The first thing we must do is refresh the metadata for this document.

    - set the `last_updated` property to today's date in the format YYYY-MM-DD
    - make sure that the `md` (darkmatter CLI) is in the host's executable path
        - if it is NOT in the path then just exit this prompt and tell the user that they must install the darkmatter CLI before running this prompt
    - check if the current value for the `hash` frontmatter of this document is equal to `md hash @claudine/docs/acp/SKILL.md --body`
        - tell the user that "the SKILL.md has not changed since the last time"


    If `$ARGUMENTS` contains `--deploy`:

    1. Create the directory `~/.claude/skills/acp/` if it does not exist
    2. Create the directory `@.claude/skills/acp/` in this repo if it does not exist
    3. Copy all `.md` files from the directory containing this SKILL.md (`@claudine/docs/acp/`) to `~/.claude/skills/acp/`, preserving filenames
    4. Copy all `.md` files from the directory containing this SKILL.md (`@claudine/docs/acp/`) to `@.claude/skills/acp/`, preserving filenames
    5. Remove the `prompt` property from ~/.claude/skills/acp/SKILL.md
    6. Remove the `prompt` property from @.claude/skills/acp/SKILL.md
    7. Inform the user that the 'acp' skill has been deployed to both the user scope _and_ the repo scope

    If `$ARGUMENTS` does not contain `--deploy`:

    8. Create the directory `@.claude/skills/acp/` in this repo if it does not exist
    9. Copy all `.md` files from the directory containing this SKILL.md (`@claudine/docs/acp/`) to `@.claude/skills/acp/`, preserving filenames
    10. Remove the `prompt` property from @.claude/skills/acp/SKILL.md
    11. Inform the user that the 'acp' skill has been deployed to both the repo scope (but not the User scope)

last_updated: 2026-02-24
---


## What is ACP?

The protocol name is **ACP (Agent Client Protocol)**.

ACP is a bidirectional JSON-RPC protocol for connecting:

- a **Client** (usually an editor, IDE, or terminal UI), and
- an **Agent** (a coding assistant process that plans, edits, runs tools, and reports progress).

Conceptually:

- **LSP** standardized editor ↔ language-tooling interactions.
- **ACP** standardizes client ↔ coding-agent interactions.

It is transport-agnostic, but today the primary transport is newline-delimited JSON over stdio.

For more details, review the [What is ACP?](./what-is-acp.md) document.

## JSON-RPC semantics

ACP uses JSON-RPC 2.0 semantics:

- **Requests**: include `id`; must receive either `result` or `error`.
- **Notifications**: omit `id`; no response is expected.
- **Errors**: standard JSON-RPC error object (`code`, `message`, optional `data`).

For more details, review the [JSON-RPC](./json-rpc.md) document.

## Software Libraries for creating an ACP Client

- [Typescript Libraries](./typescript-libraries.md)
- [Rust Libraries](./rust-crates.md)


## Support for ACP

ACP is a relatively new standard but it has a lot of support already across both editors (clients) and Agentic providers (services).

- Read the document [Who Supports ACP](./who-supports-acp.md) for a detailed view of major providers who already support ACP


### Agentic Use Cases

Using Rust as a programming language, the following documents provide details on how to interact with various Agentic CLI's
