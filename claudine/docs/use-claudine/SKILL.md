---
name: use-claudine
description: A comprehensive guide to using the Claudine CLI for agentic processes. It includes full CLI API surface, details around the composition pipeline (including interpolation, transclusion, and more), as well as how to leverage lifecycle events, define and benefit from `SimplifiedSchema` schemas, how to install Claudine, how to install shell completions, and how to use the DMLS language server with Claudine.
---
# Claudine

Claudine is a powerful CLI that wraps many of the Agentic CLI providers (claude, codex, gemini, antigravity, opencode, pi, etc.) so that it:

- **Consistent CLI switches:** make the CLI interface consistent across all of the underlying agentic CLI's
- **Shared Resources:** ensure your agent skills, slash commands, and subagent definitions are available across all agentic CLI's
- **Shared Hooks:** provide a event hook `system that can be used across all of the underlying agentic CLI's
- **Composable Prompts:**
- **Schema Support:**
- **Looping:**
- **Sequences:**

## CLI API

::file ^claudine/docs/cli/index.md

## Shared Resources

TODO

## Shared Hooks


## Composable Prompts

A _prompt_ file in Claudine is a **composable** document that can dynamically respond to the environment it is in:

- [**Interpolation**]() allows a prompt author to replace handlebar templates:
    - this can be a straight Frontmatter interpolation like `{{ foo }}` where the contents of the Frontmatter property `foo` replace the template tag
    - but Claudine also allows a robust
