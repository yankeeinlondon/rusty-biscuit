---
title: Preflight — Requirements
status: draft
last_updated: 2026-04-11
---

# Preflight — Requirements

This document captures the requirements for Claudine's pre-flight shell-command
discovery, validation, and user-facing messaging as communicated in-session. It
is a requirements document, not a design — implementation shape is intentionally
left open.

## Goals

Before Claudine launches any provider agent for a `compose`, `inline-compose`,
or `sequence` run, the operator must have already seen and approved every shell
command Claudine may execute on their behalf. The operator should be able to
review all approvals once, up front, and then walk away while the work runs.

## Functional requirements

### FR-1 — Discovery must never execute shell

Pre-flight discovery **must not** run any shell command. It is a pure inspection
pass. The operator's first warning that a command is about to run must come
*before* the agent and the runtime compose pass that would execute it, never
after.

Running shell commands "ahead of time" — i.e. at pre-flight time, in a different
order than their owning step would have run them, or with different filesystem
state — is explicitly unacceptable.

### FR-2 — Use Darkmatter's shell-discovery library function

Claudine must not re-implement shell-command discovery. Darkmatter already
exposes a library function that walks a document and returns every shell
command it would execute, without executing any of them. Claudine's pre-flight
must consume that function directly.

The user has stated that frontmatter shell commands and body `::shell`
directives should not require different discovery paths — the Darkmatter
library function handles both.

> **Open question (OQ-1)** — exact surface area. The current
> `darkmatter::markdown::compose::shell_expansion::discovery::collect_shell_commands`
> walks body `::shell` directives and Darkmatter's own frontmatter shell
> expressions. It does **not** know about Claudine's harness `pre_checks` /
> `post_checks` / handler shell commands, which are declared in a Claudine-
> specific frontmatter schema. Before implementation begins, we need to
> confirm whether:
>
> 1. Claudine's harness shell declarations should migrate to Darkmatter's
>    shell-expression syntax so the single library function covers everything, or
> 2. Claudine continues parsing its own harness shell model, and the
>    shell-disabled compose pass (the same approach `collect_shell_commands`
>    uses internally for Phase 2) is used to get interpolated frontmatter
>    without executing any shell.
>
> Either resolution is compatible with FR-1. Pick one before writing code.

### FR-3 — `compose` / `inline-compose` pre-flight

For `claudine compose <file>` and `claudine inline-compose <file>`, pre-flight
must discover and approve every shell command the run would execute — in the
body, in the frontmatter, and in any harness hooks declared by the document —
before the provider agent is launched.

### FR-4 — `sequence` pre-flight covers every step

For `claudine sequence <file>`, pre-flight must run for **every step** in the
sequence, not just once for the document. Each step has its own overlay state
(`state`, `previous_state`, `next_state`, `step`, `total_steps`) which can
cause `::shell` directives and harness commands to expand to different literal
commands per step. Every step's expansions must be discovered and approved
before the first step executes.

### FR-5 — All sequence approvals happen before the first agent launches

The sequence-level pre-flight completes for **all** steps before step 1 begins
executing. The operator is prompted (if prompting is needed at all) exactly
once, at the top of the run, for every distinct command the entire sequence
might invoke. After that, no approval prompt may fire mid-sequence under normal
operation.

### FR-6 — Shared approval cache across a single run

A single Claudine invocation (one `compose`, one `inline-compose`, or one
`sequence`) shares a single approval cache. "Allow once" decisions made at any
point in pre-flight — template or harness, first step or last step — must
prevent re-prompting for the same normalized command later in the same run.

### FR-7 — TTY-gated interactive approval (already implemented)

The interactive approval handler is installed whenever `stdin` and `stderr` are
both TTYs, independent of whether the spawned agent runs interactively. Pre-
flight runs before any agent is launched, so there is no TTY contention. Non-
TTY contexts (CI, piped input) get no handler and unapproved commands hard-fail.

## UX requirements

### UX-1 — Emit a single `Status::from_prose(...).state(StatusState::Info)` message

When pre-flight completes successfully, Claudine emits exactly one status line
to **stderr** using `biscuit_terminal::components::status::Status::from_prose(...)`
in the `StatusState::Info` state. The message's purpose is to signal to the
operator: "I have finished reviewing every shell command this run might
execute; no more approval prompts should fire."

### UX-2 — Message wording is tailored per command

The message body differs by surface:

- **`compose`**: indicates pre-flight completed for *this composition*.
- **`inline-compose`**: indicates pre-flight completed for *this inline
  composition*.
- **`sequence`**: indicates pre-flight completed *for all steps in the
  sequence*. The wording must explicitly convey that every step has already
  been checked — not just the current one.

Exact prose is left to implementation.

### UX-3 — Message is a completion indicator, not a progress bar

Only one message fires per run. It is emitted after pre-flight is done and
before step execution begins. Per-step "starting <name>" status lines
continue to emit from the sequence orchestrator as they do today — those are
execution progress, not pre-flight output.

### UX-4 — Silent and quiet modes suppress the message

When `--silent` is set, the pre-flight status message is not emitted. When
`--quiet` is set, it is also suppressed (quiet suppresses informational status
lines while keeping error output).

## Non-goals

- **Executing shell commands early.** Not acceptable. See FR-1.
- **Reordering per-step execution.** Steps still run in declared order with
  their own compose pass at their own time. Only the *discovery* pass moves
  up front.
- **Replacing per-step harness runtime validation.** The harness loop still
  runs pre-check / post-check shell commands at runtime as it does today.
  Pre-flight is a *validation* pass, not a *substitution* for runtime checks.
- **Bypassing denied commands.** If any shell command is blacklisted or the
  operator denies it at the prompt, pre-flight fails fast and no agent is
  launched.

## Open questions

- **OQ-1** — see FR-2. Whether Claudine harness shell commands should be
  unified with Darkmatter's shell-expression model, or whether we keep them
  separate and call the shell-disabled compose pass to parse them safely.
- **OQ-2** — Should the pre-flight status message include counts (e.g. "N
  commands approved across M steps") or stay purely qualitative? Counts are
  informative but have to be plumbed from two discovery surfaces; qualitative
  wording avoids the plumbing.
- **OQ-3** — For `sequence` runs where an operator's workspace has stale
  whitelist state, what is the expected behavior on re-runs: silently reuse
  the cached approvals from the `.darkmatter-shell-whitelist`, or re-prompt?
  (Current darkmatter behavior is to reuse — confirm this is still desired.)

## Tests to preserve

These existing tests codify the per-step-preflight guarantee and must continue
to pass through any refactor:

- `claudine/cli/tests/sequence_cli.rs::sequence_preflight_applies_whitelist_on_every_step`
- `claudine/lib/src/composition/preflight.rs::warm_cache_prevents_second_handler_invocation`
- `claudine/lib/src/composition/preflight.rs::shared_cache_across_distinct_options_prevents_reprompt`
- `claudine/lib/src/composition/preflight.rs::shared_cache_covers_harness_command_path`
- `claudine/lib/src/composition/preflight.rs::shared_cache_spans_template_and_harness_sources`

Any new behavior (up-front sequence discovery, single status line) should add
its own tests alongside these.
