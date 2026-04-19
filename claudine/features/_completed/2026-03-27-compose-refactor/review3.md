# Compose Refactor Review 3

`just test` in `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine` passed while reviewing this change. The issues below are implementation-fidelity gaps against the spec and tech design, not failing-test regressions.

## Findings

### 1. Repo-scoped favorite provider is never consulted during composition selection

- File: `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:944`
- Severity: High

`execute_composition_request()` is supposed to honor `settings.linking.preference[0]` from repo config before falling back to interactive selection, but `load_config_favorite()` calls `load_config(None, None)`. Passing `None` for `repo_root` means `<repo>/.claudine/config.json` is ignored entirely, so composition only sees the user-level favorite.

That diverges from the tech design and breaks a real path: in a repo that sets a composition-specific preferred provider, `claudine compose` / `claudine inline-compose` will still fall through to interactive selection or a non-TTY error unless the same preference also exists in the user config.

Recommended fix:

- Thread the repo root into `load_config_favorite()`.
- Add an integration test that creates a repo-local `.claudine/config.json`, installs multiple providers, runs `claudine compose` without explicit flags, and asserts the repo favorite wins.

### 2. Built-in inline closure failures bypass the harness handler system

- Files:
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:1779`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:534`
- Severity: High

The spec and tech design explicitly say the built-in inline closure validations should be normalized into the same stage model as harness validations. That did not land.

Today, if inline output is empty, frontmatter-only, or unchanged, both execution paths call `extract_replacement_body()` / `apply_inline_closure()` directly and immediately return exit code `1`. Those failures never become harness post-check failures, so `retry`, `resume`, or `redirect` handlers cannot recover from them.

That is the most important missing behavior in the closure path, because these are exactly the safety checks the design called out as built-in closure validations.

Recommended fix:

- Represent empty/unchanged replacement-body failures as harness post-check failures.
- Run them through the same `resolve_handler()` flow as user-authored post-checks.
- Add an integration test where the first inline attempt returns the unchanged body and a handler retries successfully.

### 3. Composition metadata for future resume/reporting is still incomplete

- Files:
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:822`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:695`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs:881`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs:2353`
  - `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/stream/reporting.rs:20`
- Severity: Medium

The refactor was supposed to preserve enough metadata from composition runs to support future recent-session / resume UX. That is only partially implemented.

Structured live events do include `composition_file_ref`, `composition_mode`, and `composition_source_path` via `with_context_extra()`, but the synthetic summary event written at session end drops that context because `summary_to_event_meta()` only serializes the stream summary itself. The problem is worse for non-structured composition paths: `run_legacy_inline()` and the legacy direct path return raw process results without writing any composition-specific synthetic event at all.

That means composition runs are not logged consistently across providers, and the end-of-session record is missing the file-level context the spec called out as future-facing resume metadata.

Recommended fix:

- Extend `summary_to_event_meta()` to accept extra context and merge the composition fields into the synthetic summary event.
- Emit an equivalent synthetic session-end event for non-structured composition runs too.
- Add a logging test that asserts the session-end event carries composition file/mode/source metadata.

## Coverage Gaps

The integration coverage is strong around interactive launch, MCP, inline rewrite, and no cross-provider retry, but there are still notable holes:

- No integration test proves repo-scoped config favorites participate in composition selection.
- No integration test covers the spec’s “ambiguous `agent` hint with no TTY returns an error” path.
- No integration test proves effective composed frontmatter, not raw frontmatter, activates harness behavior in the CLI path.
- No integration test covers built-in inline closure validation recovery because that recovery path is not implemented yet.

## Ergonomics / Performance

- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/compose.rs` still duplicates the provider-override flag set across `ComposeArgs` and `InlineComposeArgs`. The tech design’s proposed shared `ProviderOverrideArgs` would reduce maintenance drift.
- `/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/types.rs` captures `original_frontmatter_hash` and `managed_fields`, but the current closure path only hardcodes `last_updated`. Either wire those fields into tamper/managed-field handling or drop them until they are needed.
- The composition executor still carries a fair amount of duplicated non-harness wrapper launch/reporting logic. Extracting a shared execution-result helper would make future wrapper and composition changes less drift-prone.
