Your job is to act as an orchestrator to update Darkmatter CLI command docs.

Create one subagent per CLI command and run them in parallel:

- `read` (default behavior)
- `clean`
- `compose`
- `delta`
- `get`
- `hash`
- `toc`

Each subagent is responsible for exactly one file under `@darkmatter/docs/cli`:

- `read.md`
- `clean.md`
- `compose.md`
- `delta.md`
- `get.md`
- `hash.md`
- `toc.md`

## Subagent Instructions

Each subagent must:

1. Inspect the command definition in `@darkmatter/cli/src/args.rs`.
2. Inspect runtime behavior in `@darkmatter/cli/src/commands.rs` and `@darkmatter/cli/src/output.rs` where relevant.
3. Inspect command-specific tests in `@darkmatter/cli/tests/cli.rs`.
4. Compare with the current command doc in `@darkmatter/docs/cli/<command>.md`.
5. Update the doc so it reflects actual behavior today.

Authoritative source order:

1. source code
2. tests
3. existing docs / README text

If docs conflict with code/tests, docs must be corrected to match code/tests.

## Required Document Structure

Every command doc must use these H2 sections:

- `## Overview`
- `## Reporting`
- `## Lessons Learned`
- `## Issues`

Within `## Reporting`, include command-relevant H3 subsections as needed, typically:

- `### Usage`
- `### Arguments`
- `### Options`
- `### Output Behavior`

Add additional H3 sections only when useful (for example: `Verbose Mode`, `JSON Output`, `Flag Precedence`, `Validation and Errors`, `Save Mode Constraints`, etc.).

## Content Standards

- Prefer concise functional descriptions over marketing language.
- Include at least one realistic CLI example block per command.
- Document defaults, aliases, and edge cases that are visible in behavior.
- Explicitly call out root-level flags that affect command behavior (for example `-v` or global render flags), when applicable.
- If a command has mode-dependent behavior, describe each mode clearly.
- Keep statements specific and verifiable from code/tests.

## Scope Rules

- Do not redesign command behavior.
- Do not add speculative features.
- Do not reference internal component names unless they directly impact user-visible behavior.

## Completion

As orchestrator, wait for all subagents to finish, then report completion to the user.
