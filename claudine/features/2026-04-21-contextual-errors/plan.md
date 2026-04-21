---
phases: 5
created: 2026-04-21
start_phase: 0
current_phase: 0
---
# Execution Plan — Contextual Errors

Source: [spec.md](./spec.md)

Validated against the current implementation in:

- `claudine/lib/src/error.rs`
- `claudine/lib/src/system_prompt/prepare.rs`
- `claudine/lib/src/composition/error.rs`
- `claudine/lib/src/composition/preflight.rs`
- `claudine/lib/src/composition/prepare.rs`
- `claudine/cli/src/main.rs`
- `claudine/cli/src/commands/compose.rs`
- `claudine/cli/src/commands/sequence.rs`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/output/shell_expansion_error.rs`
- `darkmatter/lib/src/markdown/errors/mod.rs`
- `claudine/cli/tests/wrap_commands.rs`

Two implementation constraints are already visible in the current tree and must
shape the work:

1. Rich Darkmatter rendering will not reach the CLI until typed
   `MarkdownError` values survive both the library layer and the CLI's
   `color-eyre` conversion points.
2. The spec's "walker in `main.rs`" requires undoing several command-local
   `log::error(...)` + `std::process::exit(...)` paths that currently swallow
   rich errors before they reach the process top level.

## Phase Index

| Phase | Outcome | Depends on |
|---|---|---|
| 0 | Scope is locked to real lossy call sites and rendering seams | none |
| 1 | Claudine library errors preserve typed `MarkdownError` values | 0 |
| 2 | CLI has one top-level BlockError renderer and no duplicate shell renderer | 1 |
| 3 | Snapshot and integration coverage proves the three headline failure paths | 1, 2 |
| 4 | Manual verification, deferred-scope notes, and public docs are complete | 1, 2, 3 |

## Phase 0 — Lock The Real Error Seams

1. Confirm every current metadata-destroying conversion that this feature must
   remove:
   `SystemPromptComposition(e.to_string())` in
   `claudine/lib/src/system_prompt/prepare.rs`,
   `PreFlightDiscoveryFailed(e.to_string())` in
   `claudine/lib/src/composition/preflight.rs`,
   `ComposeFailed(other.to_string())` in
   `claudine/lib/src/composition/prepare.rs`,
   and CLI-side `eyre!("{e}")` / `eyre!("...: {e}")` conversions in
   `commands/{compose,sequence,wrap/mod}.rs`.
   Observable result: there is a checked-in implementation list of the exact
   call sites that must stop flattening Darkmatter errors.
2. Confirm every current pre-render suppression seam that conflicts with the
   new top-level walker:
   `run_compose`,
   `run_inline_compose`,
   `run_sequence`,
   `run_provider_wrapper`,
   and the shell-specific helper module
   `claudine/cli/src/output/shell_expansion_error.rs`.
   Observable result: one agreed list exists of the functions that must stop
   doing local rich rendering or sentinel suppression.
3. Choose the test homes before editing code:
   unit rendering tests in a CLI output/helper module,
   integration coverage in `claudine/cli/tests/`,
   and reuse of ANSI/path normalization helpers from
   `claudine/cli/tests/wrap_commands.rs`.
   Observable result: each acceptance criterion has a concrete destination.
4. Decide the typed-error shape for composition failures up front.
   The current code only preserves `ShellExpansionError`; to satisfy the spec's
   transclusion-cycle and preflight criteria, the plan must preserve general
   `MarkdownError` sources for non-shell compose failures as well.
   Observable result: the implementation target is explicit before any enum
   changes begin.

Validation checkpoint:

- `rg -n "to_string\\(\\)|eyre!\\(\" claudine/lib/src/system_prompt claudine/lib/src/composition claudine/cli/src/commands claudine/cli/src/output`
- `cargo test -p claudine-cli --test wrap_commands`

### Phase 0 Completion — Confirmed Call Sites

**Metadata-destroying conversions confirmed (Phase 1 targets):**

| File | Line | Current Code | Must Change To |
|------|------|--------------|---------------|
| `claudine/lib/src/system_prompt/prepare.rs` | 37 | `SystemPromptComposition(e.to_string())` | `SystemPromptComposition(e)` via `#[from] MarkdownError` |
| `claudine/lib/src/composition/preflight.rs` | 58 | `PreFlightDiscoveryFailed(e.to_string())` | `PreFlightDiscoveryFailed(e)` via `#[from] MarkdownError` |
| `claudine/lib/src/composition/prepare.rs` | 17 | `ComposeFailed(other.to_string())` | `ComposeFailed(other)` via `#[from] MarkdownError` |

**CLI eyre conversions that must become source-preserving (Phase 2 targets):**

| File | Function | Lines | Pattern |
|------|----------|-------|---------|
| `claudine/cli/src/commands/compose.rs` | `run_compose` | 242-244 | `eyre!("{e}")` after `is_pre_rendered` check |
| `claudine/cli/src/commands/compose.rs` | `run_inline_compose` | 256-258 | Same pattern |
| `claudine/cli/src/commands/sequence.rs` | `run_sequence` | 46-48 | Same pattern |
| `claudine/cli/src/commands/wrap/mod.rs` | `run_provider_wrapper` | 749-751 | Same pattern |

**Pre-render suppression seams that conflict with top-level walker (Phase 2 targets):**

| Function/Module | Issue |
|----------------|-------|
| `run_compose` | Local `log::error()` + `is_pre_rendered()` guard suppresses rich error |
| `run_inline_compose` | Same pattern |
| `run_sequence` | Same pattern |
| `run_provider_wrapper` | Same pattern |
| `claudine/cli/src/output/shell_expansion_error.rs` | Contains `PRE_RENDERED_MARKER` sentinel, `is_pre_rendered()`, and duplicate shell-specific block rendering |

**Test homes confirmed (Phase 3 destinations):**

| Test Type | Location | Coverage |
|-----------|----------|----------|
| Unit rendering tests | `claudine/cli/src/output/` (new module or existing `shell_expansion_error.rs`) | ANSI-stripped snapshots at 80 columns for `SystemPromptComposition`, `ShellExpansionFailed`, transclusion-cycle `MarkdownError` |
| CLI integration | `claudine/cli/tests/wrap_commands.rs` | Shell expansion, system prompt composition, transclusion cycle failure paths |
| ANSI/path helpers | `claudine/cli/tests/wrap_commands.rs::strip_ansi` | Reuse existing normalization |

**Typed-error shape decision (Decision 2 from spec):**

```
ClaudineError::SystemPromptComposition(#[from] MarkdownError)
ClaudineError::CompositionFailed(#[from] MarkdownError)  // new variant (replaces ComposeFailed)
CompositionError::PreFlightDiscoveryFailed(#[from] MarkdownError)
```

All three use `#[from]` to enable `?` propagation. If `MarkdownError` size becomes problematic, switch to `Box<MarkdownError>` with custom `From` impl.

## Phase 1 — Preserve Typed Darkmatter Errors In The Library

1. Refactor `ClaudineError::SystemPromptComposition` in
   `claudine/lib/src/error.rs` from `String` to typed `MarkdownError` via
   `#[from]`, matching the spec's preferred shape.
   Observable result: callers can use `?` and the enum still carries the
   original Darkmatter error as a source.
2. Update `claudine/lib/src/system_prompt/prepare.rs` so
   `compose_prompt_markdown(...)` uses typed propagation instead of
   `e.to_string()`.
   Observable result: system prompt failures keep path, line, hint, and nested
   source information.
3. Refactor composition-layer error storage so non-shell Darkmatter failures
   are no longer flattened.
   This likely means changing `CompositionError::ComposeFailed(String)` and
   `CompositionError::PreFlightDiscoveryFailed(String)` to typed-source variants
   or adding new typed variants with `#[source] MarkdownError`.
   Observable result: shell expansion, transclusion, and other Markdown errors
   all remain discoverable through the error chain.
4. Update `claudine/lib/src/composition/prepare.rs` and
   `claudine/lib/src/composition/preflight.rs` to use the new typed variants
   instead of `.to_string()` wrappers.
   Observable result: direct compose and preflight discovery now preserve
   structured Darkmatter metadata.
5. Audit any remaining library-level Markdown conversion sites introduced by
   this feature's paths and convert them to source-preserving propagation.
   Observable result: no targeted path still collapses `MarkdownError` to plain
   text before reaching the CLI.
6. Measure enum size after the type changes.
   If `ClaudineError` becomes unreasonably large, switch the variant to
   `Box<MarkdownError>` and add the equivalent `From` impl the spec allows.
   Observable result: the chosen storage shape is justified by an explicit size
   check rather than guesswork.

Parallelizable work:

- Steps 1.1 and 1.3 can proceed in parallel once the final typed-error shape is
  chosen in Phase 0.
- Step 1.6 can run as soon as the first enum refactor compiles.

Validation checkpoint:

- `cargo check -p claudine`
- `cargo test -p claudine system_prompt`
- `cargo test -p claudine preflight`
- `rg -n "SystemPromptComposition\\(|ComposeFailed\\(|PreFlightDiscoveryFailed\\(" claudine/lib/src`

## Phase 2 — Centralize CLI Rendering Around `BlockError`

1. Introduce one CLI helper responsible for rich Darkmatter rendering from a
   `color-eyre` report by walking the cause chain, calling
   `darkmatter::markdown::errors::as_block_error(...)` on each cause, and
   choosing the deepest typed match.
   Observable result: there is a single reusable function that answers
   "can this report be rendered as a Darkmatter block error?".
2. Wire that helper into `claudine/cli/src/main.rs` so process-top-level error
   handling does exactly one of two things:
   render the deepest `BlockError` and suppress default eyre output,
   or fall back to the existing generic error rendering when no typed match is
   present.
   Observable result: `main.rs` becomes the only rich-error decision point.
3. Refactor command entrypoints to stop pre-rendering or double-logging errors:
   `run_compose`,
   `run_inline_compose`,
   `run_sequence`,
   and `run_provider_wrapper` should return failures upward instead of checking
   `is_pre_rendered(...)` and logging locally.
   Observable result: block-style rendering is no longer bypassed by local
   command handlers.
4. Replace CLI-side stringifying conversions on relevant Darkmatter paths with
   source-preserving conversions.
   In practice this means removing uses of
   `pretty_or_report`,
   `pretty_markdown_report`,
   and `eyre!("{e}")` on compose/preflight/system-prompt paths in favor of
   returning the typed error or wrapping it with source-preserving context.
   Observable result: the top-level walker can still see the original typed
   errors even when CLI context strings are added.
5. Delete the sentinel-based shell renderer path by removing:
   `PRE_RENDERED_MARKER`,
   `is_pre_rendered(...)`,
   and the duplicate block-building logic in
   `claudine/cli/src/output/shell_expansion_error.rs`.
   If a helper module remains, it should only host the generic report walker or
   small shared rendering glue, not shell-specific report construction.
   Observable result: Darkmatter's own `BlockError` impls become the sole rich
   renderer for shell expansion and other Markdown errors.
6. Keep agent-process exit-code rendering separate from BlockError handling.
   `AgentErrorReport` should still render provider-native failures after a
   nonzero child exit, but it must not interfere with the new rich-rendered
   composition/system-prompt/preflight failures.
   Observable result: child CLI failures still render as before, while
   Darkmatter-driven failures flow through the new top-level walker.

Parallelizable work:

- Steps 2.1 and 2.4 can move in parallel once the Phase 1 typed-error variants
  compile.
- Step 2.6 can be validated independently after the top-level renderer is in
  place.

Validation checkpoint:

- `cargo check -p claudine-cli`
- `rg -n "PRE_RENDERED_MARKER|is_pre_rendered|pretty_markdown_report|pretty_or_report" claudine/cli/src`
- `cargo test -p claudine-cli`

## Phase 3 — Prove The Three Headline Failure Paths

1. Add unit-level ANSI-stripped rendering tests for the generic BlockError
   report path with width fixed at 80 columns.
   Cover at minimum:
   `ClaudineError::SystemPromptComposition`,
   a shell-expansion composition failure,
   and a transclusion-cycle `MarkdownError`.
   Observable result: the CLI's rich-rendering boundary is stable under
   snapshots or exact string assertions without terminal-width drift.
2. Add CLI integration coverage for shell expansion failure in
   `claudine compose`.
   Build a temp Markdown fixture with a denied `::shell` directive and assert
   stderr contains the command name, line number, and whitelist/approval hint.
   Observable result: the shell path proves the duplicate shell renderer is no
   longer needed.
3. Add CLI integration coverage for system prompt composition failure through an
   actual wrapper command that exercises
   `claudine/lib/src/system_prompt/prepare.rs`.
   Use a broken system prompt file and assert stderr contains the prompt source
   path, line number, and Darkmatter hint text.
   Observable result: the highest-impact user-facing path from the spec is
   covered end-to-end.
4. Add CLI integration coverage for a transclusion cycle on a compose/preflight
   path and assert stderr shows the file chain with line numbers.
   Observable result: non-shell Markdown errors now survive all the way to
   final rendering.
5. Keep test helpers deterministic by stripping ANSI and redacting temp-home or
   temp-path fragments where necessary, following the existing integration test
   conventions in `claudine/cli/tests/wrap_commands.rs`.
   Observable result: the new tests are stable in CI and on developer machines.

Parallelizable work:

- Step 3.1 can start as soon as the generic rendering helper from Phase 2
  exists.
- Steps 3.2, 3.3, and 3.4 are independent once the CLI error path is stable.

Validation checkpoint:

- `cargo test -p claudine-cli --test wrap_commands`
- `cargo test -p claudine-cli`
- `cargo test -p claudine`

## Phase 4 — Manual Verification, Deferred Scope, And Public Notes

1. Run the three manual acceptance scenarios from the spec against real CLI
   commands:
   `claudine compose` with a denied shell command,
   a wrapper command with a broken system prompt file,
   and a transclusion cycle fixture.
   Observable result: each scenario prints rich errors with file paths, line
   numbers, and hints instead of flat text.
2. Record the deferred harness gap explicitly in the feature closure:
   `claudine/lib/src/harness/parse.rs:964` and
   `claudine/lib/src/harness/audit.rs:80` remain out of scope for this feature.
   Observable result: the merge artifact makes it clear those lossy internal
   sites were intentionally left for follow-up.
3. Add a public-facing compatibility note for the `ClaudineError` enum change.
   There is no obvious `claudine` changelog file today, so update the nearest
   crate-facing documentation surface, likely `claudine/lib/README.md`, and
   note the typed `SystemPromptComposition` payload change there.
   Observable result: the breaking public enum change is documented in a stable
   repo location before merge.
4. Do a final grep-based cleanup to confirm the old shell-specific machinery is
   gone and that targeted Markdown paths no longer use stringifying wrappers.
   Observable result: the codebase reflects the new architecture rather than a
   hybrid of old and new flows.

Validation checkpoint:

- `cargo test -p claudine-cli && cargo test -p claudine`
- `cargo clippy -p claudine-cli -p claudine`
- `rg -n "PRE_RENDERED_MARKER|pretty_markdown_report|SystemPromptComposition\\(String\\)|to_string\\(\\)" claudine`

## Completion Criteria

The implementation is ready to close when all of the following are true:

1. `SystemPromptComposition` carries typed `MarkdownError` data and callers use
   typed propagation.
2. Composition and preflight paths preserve non-shell `MarkdownError` sources
   strongly enough for a top-level cause-chain walker to discover them.
3. `main.rs` is the single place that decides whether to render a Darkmatter
   `BlockError` or fall back to generic eyre output.
4. `claudine/cli/src/output/shell_expansion_error.rs` no longer contains custom
   shell-only block rendering or sentinel-based suppression logic.
5. Unit and integration tests cover shell expansion, system prompt
   composition, and transclusion-cycle failures end-to-end.
6. Manual runs show contextual paths, line numbers, and hints, and the two
   harness lossiness sites are explicitly documented as deferred.
