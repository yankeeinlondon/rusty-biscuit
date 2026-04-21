# How Claudine Can Benefit from Darkmatter's Rich Error Metadata

## Current State

Darkmatter now has 16 error enums with 100+ variants, all implementing the BlockError trait from biscuit-terminal -- each carrying structured metadata (line numbers, paths, origin context, caret columns, stderr output, transclusion chains) and rendering as a StatusBlock with title + body + hint.
Claudine has three rendering pathways for Darkmatter errors, and they vary wildly in quality:

- Shell expansion in compose
- System prompt composition
- Terminal rendering fallback

## Key Opportunities

1. Use BlockError directly instead of re-rendering

    Claudine's `claudine/cli/src/output/shell_expansion_error.rs` (598 lines; located in the CLI, not the library) builds its own StatusBlock from ShellExpansionError fields. Darkmatter's ShellExpansionError already implements BlockError with title + body + hint rendering. Note that `CompositionError::ShellExpansionFailed` already carries `Box<ShellExpansionError>` (typed, not stringified) -- the remaining work is deleting the duplicate CLI-side renderer at `claudine/cli/src/output/shell_expansion_error.rs` and letting the walker call `.report_block_error(&terminal)` directly. This eliminates the duplicate rendering logic and automatically picks up any future Darkmatter error improvements.

    Additionally, this file hosts two pieces of machinery that the redesign removes: `pretty_markdown_report` (around line 25) and the `PRE_RENDERED_MARKER` sentinel (around line 75). See Decision 3 below.

2. Fix the system prompt lossy conversion

    `ClaudineError::SystemPromptComposition(String)` at `claudine/lib/src/system_prompt/prepare.rs:37` calls `.to_string()` on `MarkdownError`, destroying all structured metadata (line numbers, paths, shell command context, hints). Changing this to carry the typed `MarkdownError` would preserve:
    - Source file and line number for the failing directive
    - The exact shell command that was denied/failed/timed out
    - The contextual hint (e.g., "Add the command to your whitelist at ~/.config/md/shell-whitelist")
    - Transclusion ancestry chains for cycle detection errors

    This is the single highest-impact fix -- system prompt composition failures are user-facing and currently show as flat text.

3. Leverage the as_block_error registry

    Darkmatter's `as_block_error()` at `darkmatter/lib/src/markdown/errors/mod.rs:54` performs runtime downcasting across all 16 concrete error types against `dyn BlockError`. Claudine's CLI already uses color-eyre and walks cause chains. Adding a pass that calls `as_block_error` on each cause in the chain would give Claudine access to Darkmatter's full rendering for any error variant -- not just shell expansion.

4. Fix the silent harness gaps _(OUT OF SCOPE for this feature -- deferred to a follow-up spec; see Decision 1)_

    Two harness sites discard Darkmatter error detail:
    - `claudine/lib/src/harness/parse.rs:964` -- `tokenize()` errors are caught with `|_e|` (error value completely discarded)
    - `claudine/lib/src/harness/audit.rs:80` -- `parse_directives()` errors become `.to_string()`

    Both could carry the original error and render it via BlockError. These sites are library-internal and not user-facing today, so they are deferred.

5. Add a From<MarkdownError> for ClaudineError impl

    There's no blanket conversion -- every call site manually converts MarkdownError, leading to inconsistent handling. A single #[from] impl that preserves the error (not its string) would enforce consistent preservation of metadata across all entry points.

6. Pre-flight discovery could preserve structure

    `claudine/lib/src/composition/preflight.rs:57` flattens `MarkdownError` to string via `PreFlightDiscoveryFailed(e.to_string())`. If discovery hits a transclusion cycle or invalid reference, the user sees flat text instead of the chain of files and line numbers that Darkmatter already computed. This falls out naturally once opportunity #5 adds `From<MarkdownError> for ClaudineError`.

## Summary

The biggest win is adopting Darkmatter's BlockError rendering directly rather than re-implementing or stringifying. The as_block_error registry + cause-chain walking would give Claudine rich error output for free across all 16 error types. The system prompt pipeline is the most impactful single fix since it's the most common user-facing failure path and currently the most lossy.

## Design Decisions

### Decision 1 -- Scope: Unified redesign

This feature ships as **one coherent change** covering opportunities #1, #2, #3, #5, and #6 together. The scope is:

- **#5** -- Add `From<MarkdownError> for ClaudineError` so every call site can use `?` instead of manual `.map_err(|e| ... e.to_string())`.
- **#2** -- Refactor `ClaudineError::SystemPromptComposition(String)` to carry the typed `MarkdownError`.
- **#1** -- Retire/delete the CLI's duplicate renderer at `claudine/cli/src/output/shell_expansion_error.rs`. `ShellExpansionError` already implements `BlockError` and can be rendered directly by the walker.
- **#3** -- Add a cause-chain walker in `claudine/cli/src/main.rs` that calls `as_block_error` on each cause and renders the deepest typed match.
- **#6** -- Preflight at `claudine/lib/src/composition/preflight.rs:57` preserves `MarkdownError` structure (naturally falls out of #5).

**Out of scope** (deferred to a follow-up spec):

- **#4** -- harness sites at `claudine/lib/src/harness/parse.rs:964` and `claudine/lib/src/harness/audit.rs:80`. These are library-internal and not user-facing today.

### Decision 2 -- API shape: `#[from] MarkdownError` direct variant

`ClaudineError::SystemPromptComposition` carries the typed error directly via `thiserror`'s `#[from]`:

```rust
#[error("system prompt composition failed")]
SystemPromptComposition(#[from] MarkdownError),
```

- Enables `?` at every call site; no manual conversion needed.
- Preserves pattern-matching on `MarkdownError`'s variants.
- Rendering is free because `MarkdownError` already implements `BlockError`.
- **Escape hatch:** if `std::mem::size_of::<MarkdownError>` is measured to be problematic for `ClaudineError`'s overall size (e.g., bloating hot paths), this spec authorises a one-line upgrade to `Box<MarkdownError>` with a custom `From` impl.
- **Not chosen:** `Box<dyn BlockError + Send + Sync + 'static>` (loses pattern-matching for abstract flexibility Claudine doesn't need); keeping `String` with `#[source]` (`Display` impl would still lie to non-walker consumers).

### Decision 3 -- `PRE_RENDERED_MARKER` replaced wholesale

The existing `PRE_RENDERED_MARKER` sentinel pattern in `claudine/cli/src/output/shell_expansion_error.rs` is **deleted**. The new cause-chain walker in `claudine/cli/src/main.rs` becomes the single source of rich rendering:

- Walker iterates the `color-eyre` cause chain from outermost to innermost.
- For each cause, calls `darkmatter::markdown::errors::as_block_error(cause)`.
- Renders the deepest typed `BlockError` match via `report_block_error_optimistic` (or equivalent).
- Falls back to `color-eyre`'s default rendering if no cause implements `BlockError`.
- **Never renders twice**: if the walker renders, it must suppress the default `color-eyre` report for that error.

### Decision 4 -- Testing strategy: snapshots + integration tests

Two complementary test layers:

1. **Unit-level ANSI-stripped snapshot tests** at the conversion boundary, following the pattern at `darkmatter/lib/src/markdown/errors/mod.rs:131`. Fix terminal width to 80 columns. Cover at minimum:
   - `ClaudineError::SystemPromptComposition` rendered via `BlockError`.
   - `CompositionError::ShellExpansionFailed` rendered via `BlockError` (confirms the CLI's duplicate renderer can be deleted without regressing output).
   - A transclusion-cycle `MarkdownError` rendered via `BlockError`.

2. **CLI integration tests via `assert_cmd`** for the three headline failure paths, asserting stderr contains:
   - Shell expansion failure in `claudine compose` -- command name, line number, whitelist hint.
   - System prompt composition failure -- source file path, line number, hint.
   - Transclusion cycle in preflight -- chain of files with line numbers.

## Acceptance Criteria

The feature is "done" when all of the following hold:

1. `ClaudineError::SystemPromptComposition` carries `MarkdownError` (via `#[from]`), not `String`.
2. `From<MarkdownError> for ClaudineError` exists and is used via `?` at every call site that previously did `.to_string()` conversion (notably `claudine/lib/src/system_prompt/prepare.rs:37` and `claudine/lib/src/composition/preflight.rs:57`).
3. The duplicate CLI renderer at `claudine/cli/src/output/shell_expansion_error.rs` is deleted (or reduced to a thin wrapper that adds no rendering logic). `ShellExpansionError`'s `BlockError` impl is the sole renderer.
4. A cause-chain walker in `claudine/cli/src/main.rs` calls `as_block_error` on each cause and renders the deepest typed match. Untyped errors fall back to `color-eyre`'s default rendering.
5. `PRE_RENDERED_MARKER` is removed from the codebase.
6. Unit snapshot tests cover the three variants listed in Decision 4(1). CLI integration tests cover the three failure paths listed in Decision 4(2). All tests pass.
7. Manual verification: running `claudine compose` against fixture markdown with (a) a denied shell command, (b) a bad system prompt directive, and (c) a transclusion cycle produces rich rendered errors with line numbers, paths, and hints -- not flat text.
8. Harness sites at `claudine/lib/src/harness/parse.rs:964` and `claudine/lib/src/harness/audit.rs:80` are explicitly documented as out of scope / deferred.

## Backwards Compatibility

`ClaudineError` is a public enum in `claudine/lib`. Changing `SystemPromptComposition(String)` to `SystemPromptComposition(MarkdownError)` is a **breaking change** for any external crate that pattern-matches on this variant or inspects its string payload. In practice, Claudine's lib is consumed primarily by Claudine's CLI, so the blast radius is expected to be internal-only; this assumption should be confirmed before merge. Document the change in the crate's release notes.
