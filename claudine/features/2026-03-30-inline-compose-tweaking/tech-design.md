# Inline Compose Tweaking Tech Design

This document turns the inline-compose tweaking spec into an implementation-ready design for Claudine's current composition and wrapper stack.

Primary inputs:

- `claudine/features/2026-03-30-inline-compose-tweaking/spec.md`
- `claudine/docs/topics/composition.md`
- `claudine/cli/src/commands/compose.rs`
- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/lib/src/composition/prepare.rs`
- `claudine/cli/src/output.rs`
- `claudine/cli/tests/wrap_commands.rs`
- local provider research in `claudine/docs/research/non-interactive-sessions/`

## Summary

The spec bundles together two different issues:

1. the canonical `claudine inline-compose <file>` flow
2. the mistaken provider-first invocation `claudine <provider> inline-compose <file>`

The first path is already much closer to the desired architecture than the spec suggests. `compose` and `inline-compose` already share `execute_composition_request(...)`, both route through the wrapper-grade streaming pipeline, and canonical `inline-compose` already prints an `Agent Prompt:` block, emits the provider session ID, renders assistant Markdown through Darkmatter, and performs inline closure.

The confusing behavior in the spec's example comes from the second path. `claudine claude inline-compose @file.md` does not invoke the composition executor at all. It enters the plain Claude wrapper, forwards `inline-compose` and the file path as passthrough provider arguments, and the provider reasonably treats that as a normal prompt/request. That is why the agent answers with "What specifically do you need me to do with inline-compose?"

The design therefore focuses on four concrete changes:

1. hard-reject misplaced provider-first composition invocations with actionable suggestions
2. add a deprecated `compose-inline` top-level shim that fails with a corrective message
3. tighten prompt presentation so the displayed prompt matches the actual first-attempt prompt
4. define a bounded `--perf` design for composition commands, since the spec promises it but no implementation exists yet

## Findings From Spec Review

### 1. Canonical `inline-compose` already uses the shared composition executor

Current code path:

1. `claudine/cli/src/commands/compose.rs`
2. `claudine::composition::prepare_inline(...)`
3. `claudine/cli/src/commands/wrap/composition.rs::execute_composition_request(...)`

This is the same high-level executor used by `claudine compose`.

### 2. Canonical `inline-compose` already shows the prompt block

`execute_composition_request(...)` calls:

- `crate::output::log_wrapper_header(...)`
- `crate::output::log_compose_prompt(&request.prepared.prompt, ...)`

That behavior exists today for the top-level command. The gap is that there is no regression test that locks it in.

### 3. The example failure is reproduced by the wrong entrypoint, not by `inline-compose`

Current provider wrapper behavior:

1. `claudine claude inline-compose @file.md`
2. `inline-compose` is parsed as passthrough provider input
3. `has_prompt_source(...)` sees a positional arg and flips the wrapper into non-interactive mode
4. the wrapper forwards those args to the provider instead of invoking composition

This is the real source of the "begs a LOT of questions" output in the spec.

### 4. The spec's `compose-inline` and `--perf` requirements are not implemented

Current state:

- `compose-inline` is an unknown clap subcommand
- `--perf` does not exist on `compose`, `inline-compose`, or the shared request type

### 5. Prompt display can still drift from the prompt actually sent to the provider

Even in the canonical path, the displayed prompt is currently `request.prepared.prompt`, but the actual launch prompt may differ before execution:

- MCP tag cleanup rewrites `effective_prompt`
- future prompt normalization may further rewrite the first-attempt prompt
- harness retry overlays can rewrite later attempts

For the initial attempt, the prompt banner should reflect the post-normalization prompt that is actually passed to `profile.apply_prompt_body(...)`.

## Goals

1. Preserve the existing shared executor for `compose` and `inline-compose`.
2. Make the user-facing CLI surface unambiguous.
3. Fail early when users accidentally invoke composition through a provider wrapper.
4. Add a dyslexia-safe `compose-inline` compatibility shim that teaches the right command.
5. Make prompt presentation match the actual first-attempt prompt.
6. Add regression coverage for the prompt banner, session banner, and incorrect invocation paths.
7. Define a bounded `--perf` rollout for composition commands.

## Non-Goals

1. Rewriting the composition executor from scratch.
2. Auto-translating provider-first composition invocations into top-level composition commands.
3. Redesigning harness retry semantics.
4. Changing how inline closure rewrites frontmatter.
5. Building a repo-wide performance framework for every Claudine command.

## Design

### 1. Command Surface Hardening

### 1.1 Add a hidden deprecated `compose-inline` top-level subcommand

Current behavior is a generic clap "unknown subcommand" error. The spec wants something better: a specific, immediate correction.

Recommended parser change in `claudine/cli/src/args.rs`:

```rust
#[command(name = "compose-inline", hide = true)]
ComposeInlineDeprecated(commands::compose::DeprecatedComposeInlineArgs),
```

Recommended handler in `claudine/cli/src/commands/compose.rs`:

- accept trailing args so clap does not reject old invocations
- always return an error
- suggest `claudine inline-compose ...`

Suggested behavior:

```text
compose-inline has been retired; use `claudine inline-compose <file>` instead
```

If a file argument is present, the error can include a concrete migration example:

```text
compose-inline has been retired; use `claudine inline-compose <same-file>` instead
```

### 1.2 Reject misplaced composition invocations inside provider wrappers

Add a new guard in `claudine/cli/src/commands/wrap/mod.rs` before prompt detection and before any provider mutation:

```rust
reject_misplaced_composition_invocation(provider, &child_args)?;
```

Recommended detection heuristic:

1. first passthrough token is one of:
   - `compose`
   - `inline-compose`
   - `compose-inline`
2. second passthrough token looks like a Markdown file reference:
   - starts with `@`
   - ends with `.md` or `.markdown`
   - or resolves to an existing markdown file

This heuristic avoids blocking legitimate prompts like:

```text
claudine claude compose the release notes
```

while still catching the clearly mistaken forms:

```text
claudine claude inline-compose @foo.md
claudine codex compose ./prompt.md
claudine goose compose-inline docs/spec.md
```

Suggested error shape:

```text
`inline-compose` is a top-level Claudine command, not a Claude passthrough argument.
Use `claudine inline-compose --claude @claudine/docs/research/permissions/claude.md` instead.
```

Equivalent suggestions should be generated for `compose` and `compose-inline`.

### 1.3 Do not auto-rewrite the command

The wrapper should reject, not silently transform, the invocation.

Reasons:

1. provider wrappers intentionally treat passthrough args as opaque
2. automatic rewrite would make wrapper semantics surprising
3. false positives would be harder to reason about than explicit failure
4. the corrective command is straightforward and easy to teach

### 2. Prompt Presentation Alignment

### 2.1 Define the displayed prompt as the materialized first-attempt prompt

The prompt banner should show the prompt that the first provider attempt will actually receive after Claudine-owned preprocessing, not the earlier prepared prompt snapshot.

For v1, "materialized first-attempt prompt" means:

1. Darkmatter composition already applied
2. inline guardrails already appended
3. MCP `#tag` cleanup already applied
4. no handler retry overlays yet
5. no provider-native argv transformation shown

This keeps the banner semantically correct without dumping native CLI implementation details.

### 2.2 Add an explicit prompt display value in the executor

Recommended change in `claudine/cli/src/commands/wrap/composition.rs`:

- stop calling `log_compose_prompt(&request.prepared.prompt, ...)`
- instead call it with a new `display_prompt` derived from the final initial prompt string

Recommended flow:

1. prepare `effective_prompt` from `request.prepared.prompt`
2. apply MCP tag cleanup and any other pre-launch prompt rewrites
3. assign `display_prompt = effective_prompt.clone()`
4. log `display_prompt`
5. launch using `display_prompt`

This fixes the existing drift where the UI can show prompt text containing unresolved `#mcp` tags even though the provider receives a cleaned prompt.

### 2.3 Harness path should keep a single initial prompt banner

For harness-enabled runs:

1. print the prompt banner once for the initial attempt
2. do not reprint the full prompt after every retry
3. surface retry changes through validation and handler reporting instead

This keeps output readable and matches current composition UX.

### 3. Shared Execution Invariants

The spec is right to insist that `compose` and `inline-compose` share one execution model. The code already mostly does that, so this design treats shared execution as an invariant to protect rather than a new architecture to invent.

The following behaviors must remain shared:

1. provider selection via the same precedence chain
2. wrapper-grade env setup and sanitization
3. structured stream handling where the provider supports it
4. session ID reporting from `LiveStreamSink`
5. Darkmatter rendering of assistant Markdown in non-interactive mode
6. harness detection from effective composed frontmatter
7. unified summary footer

Recommended documentation note for `claudine/docs/topics/composition.md`:

- explicitly state that provider-first forms like `claudine claude inline-compose ...` are invalid and will be rejected

### 4. `--perf` Design

The spec says both commands should offer `--perf`, but the codebase has no existing composition perf flag or shared perf collector. This design therefore scopes `--perf` to a composition-local first version.

### 4.1 CLI shape

Add to both `ComposeArgs` and `InlineComposeArgs`:

```rust
#[arg(long)]
pub perf: bool,
```

Thread this through `CompositionExecutionRequest`.

### 4.2 Perf data model

Add a lightweight CLI-side trace struct in `claudine/cli/src/commands/wrap/composition.rs`:

```rust
struct CompositionPerfTrace {
    resolve_ms: u64,
    prepare_ms: u64,
    provider_select_ms: u64,
    env_setup_ms: u64,
    mcp_ms: u64,
    harness_parse_ms: Option<u64>,
    pre_checks_ms: Option<u64>,
    provider_exec_ms: u64,
    inline_closure_ms: Option<u64>,
    total_ms: u64,
}
```

This should be measured with `std::time::Instant` around already-existing stages rather than by introducing a new tracing subsystem.

### 4.3 Output contract

When `--perf` is set and `--silent` is not set:

1. normal composition output still appears
2. an additional perf footer is emitted on stderr after the summary footer

Suggested normal-mode footer:

```text
Perf: resolve 3ms · prepare 12ms · mcp 4ms · pre-checks 8ms · provider 1180ms · closure 6ms · total 1218ms
```

Suggested `--quiet` footer:

```text
Perf: total 1218ms
```

For inline runs without harness:

- omit harness-specific fields

For direct runs:

- omit `inline_closure_ms`

### 4.4 Scope limit

This v1 perf design only covers top-level `compose` and `inline-compose`.

It does not attempt to retrofit a universal `--perf` story across plain provider wrappers until the rest of the CLI has a broader performance reporting design.

## Implementation Plan

### Phase 1: Fix the confusing invocation surface

Files:

- `claudine/cli/src/args.rs`
- `claudine/cli/src/main.rs`
- `claudine/cli/src/commands/compose.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

Tasks:

1. add hidden `compose-inline` deprecated subcommand
2. add handler that fails with corrective guidance
3. add `reject_misplaced_composition_invocation(...)`
4. call it from provider wrappers before prompt detection

### Phase 2: Align prompt display with launch prompt

Files:

- `claudine/cli/src/commands/wrap/composition.rs`
- possibly `claudine/lib/src/composition/types.rs` if a dedicated display field is preferred

Tasks:

1. compute `display_prompt` after MCP cleanup
2. print `display_prompt` instead of `request.prepared.prompt`
3. keep single initial prompt banner for harness runs

### Phase 3: Add regression coverage

Files:

- `claudine/cli/tests/wrap_commands.rs`

Tests to add:

1. `inline_compose_shows_prompt_banner`
2. `compose_shows_prompt_banner`
3. `inline_compose_prompt_banner_uses_cleaned_mcp_prompt`
4. `provider_wrapper_rejects_misplaced_inline_compose`
5. `provider_wrapper_rejects_misplaced_compose`
6. `compose_inline_deprecated_command_suggests_inline_compose`

### Phase 4: Add `--perf`

Files:

- `claudine/cli/src/commands/compose.rs`
- `claudine/lib/src/composition/types.rs`
- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/tests/wrap_commands.rs`

Tasks:

1. add CLI flag and request plumbing
2. add per-stage timers
3. render perf footer
4. test quiet and normal-mode output

## Testing Strategy

### Unit / focused tests

1. wrapper misuse detection heuristics
2. deprecated `compose-inline` handler message
3. perf formatter output

### Integration tests

Use the existing fake executable pattern in `claudine/cli/tests/wrap_commands.rs` to assert:

1. canonical `inline-compose` stderr contains `Agent Prompt:`
2. canonical `inline-compose` stderr contains the source prompt text
3. canonical `inline-compose` stderr contains the provider session banner
4. `claudine claude inline-compose @file.md` exits early with a corrective message
5. `claudine compose-inline file.md` exits early with a corrective message

### Manual verification

Recommended smoke checks after implementation:

1. `claudine inline-compose --claude @some.md`
2. `claudine compose --codex @some.md`
3. `claudine claude inline-compose @some.md`
4. `claudine compose-inline @some.md`
5. `claudine inline-compose --claude --mcp @some.md`
6. `claudine inline-compose --claude --perf @some.md`

## Risks

### 1. False positives in wrapper misuse detection

A user could genuinely want to prompt the provider with text beginning with `inline-compose`.

Mitigation:

- only reject when the second token looks like a markdown file reference

### 2. Prompt banner drift can recur as new prompt transforms are added

Mitigation:

- define a single "displayed launch prompt" variable in the executor
- test the MCP-cleanup case explicitly

### 3. `--perf` could become a dumping ground for unrelated metrics

Mitigation:

- keep v1 to stage timings only
- do not expand to repo-wide wrapper metrics in this feature

## Open Questions

### 1. Should `compose-inline` eventually become a silent alias?

Recommendation: no. The spec explicitly asks for an error with guidance, and that is safer for users who are trying to learn the canonical command names.

### 2. Should wrapper misuse rejection also catch `claudine claude compose @file.md --claude`-style hybrids?

Recommendation: yes, if the first passthrough token is a composition keyword and the next token is file-like. The provider name is already known from the wrapper subcommand, so the correction remains unambiguous.

### 3. Should `--perf` become available on plain provider wrappers too?

Not in this feature. There is not enough existing perf structure in the wrapper-only path to specify that cleanly here.

## Recommended Outcome

Implement Phases 1 through 3 as the main feature. They directly solve the confusion described in the spec and lock in the shared composition behavior Claudine already has.

Treat Phase 4 (`--perf`) as part of the same feature only if the team wants a bounded composition-local implementation now. Otherwise it should be called out as a follow-up, because the spec is clear about the need but the current codebase has no existing perf contract to extend.
