---
ready: false
agent: codex
model: ""
---

# Review: Compose Pipeline v2 — Iteration 2

## Verdict

Not ready for production.

The prior review's concrete implementation bugs around cache-key operators, remote URL child collection, and pre-approved membership checks have been addressed and are covered by focused L1 tests. One production-blocking gap remains at the normal CLI/library approval path: pre-flight is still not the approval lifecycle used by `md compose` unless the caller has already supplied a pre-approved set.

## Findings

### High: normal `md compose` still uses per-directive approval instead of the designed up-front pre-flight approval

The design requires a single condition-blind pre-flight stage before any shell command executes:

- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:107`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:110`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:117`

The implementation only runs the new up-front membership validation when `ComposeOptions::pre_approved_commands` is already `Some`:

- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:216`
- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs:221`

But the normal CLI compose path does not compute a pre-flight report, approve it, and pass it back as `with_pre_approved_commands(...)`. It installs `CliShellApprovalHandler` and calls `md.compose_with(options)`:

- `darkmatter/cli/src/commands/compose.rs:326`
- `darkmatter/cli/src/commands/compose.rs:365`

That falls back to `prepare_directive()`'s legacy in-stage approval flow:

- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:165`
- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:232`

So an interactive `md compose` run can still approve/execute an earlier frontmatter `$(...)` command before discovering a later body, shell-block, or transcluded command that needs approval. It also still prompts per reached directive rather than presenting the condition-blind batch, so commands hidden behind currently false page blocks or transclusion `when=` conditions are not approved up front through this user-facing path.

Verification level: this is L1. Add an in-process CLI/library test with a side-effecting frontmatter command and a later unapproved command, using an approval handler rather than `pre_approved_commands`, and assert the side effect does not happen before the later command is rejected or denied. Add a CLI test for the positive path that `md compose` computes the condition-blind approval set once and executes using a pre-approved membership set with no in-stage prompts. No L2/L3 testing is required because this is compose orchestration, not terminal rendering or keyboard behavior.

Suggested fix: make the user-facing compose path run `compose_preflight()` before `compose_with()`, validate blacklist/whitelist policy against the collected approval set, batch prompt for the remainder, then call `compose_with(options.with_pre_approved_commands(...))`. Keep `prepare_directive()`'s `pre_approved_commands` fast path as the execution membership gate. If direct library `compose_with()` is intentionally not responsible for prompting, document that contract explicitly and make the CLI the reference implementation of the full pre-flight approval lifecycle.

### Medium: pre-flight graph metadata is collected but not reused by transclusion

The design says the pre-flight collection walk should cache graph metadata and reuse it to seed graph composition, avoiding a second directive parse/target-resolution pass:

- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:121`
- `darkmatter/features/2026-06-16-compose-pipe-2/tech-design.md:123`

Iteration 2 adds `ComposePreflightReport::preflight_graph`, but the doc comment explicitly frames it as something "that lets a future integration skip re-discovering the graph":

- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:102`
- `darkmatter/lib/src/markdown/compose/preflight/mod.rs:106`

There are no non-test consumers of `preflight_graph` outside `compose_preflight()`, and the transclusion engine still resolves directives from scratch:

- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:462`
- `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:492`

This does not break the core approval/execution safety invariant by itself, but it leaves one explicit design goal incomplete and preserves the collector/executor drift risk the redesign was meant to reduce.

Verification level: L1 is sufficient. Once graph reuse is wired, add a test that proves a preflight-collected local or remote child is consumed by the transclusion stage without repeating target discovery. If reuse is intentionally deferred, update the design/plan to mark graph reuse as a later milestone instead of part of this production-ready slice.

## Test Coverage Assessment

- L1 present: condition-blind local and remote collection, false page-block/transclusion collection, dynamic command-shape rejection, `execution_set ⊆ approval_set` examples including randomized conditions, cache default behavior, `--no-cache`, volatile warnings, and cache-key separation for chain operators/redirections.
- L1 gap: the normal CLI approval lifecycle is not tested as a single up-front pre-flight approval pass.
- L1 gap: graph metadata reuse is not tested because it is not implemented.

No Level 2 or Level 3 tests are required for the findings above. The reviewed requirements are compose semantics and approval orchestration, not terminal emulator rendering or OS keyboard input.

## Verification Run

I ran:

```text
cargo test --color=never -p darkmatter markdown::compose::preflight --lib
cargo test --color=never -p darkmatter markdown::compose::shell_expansion::integration_tests --lib
```

Both passed.
