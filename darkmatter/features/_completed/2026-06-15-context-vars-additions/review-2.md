---
ready: true
agent: codex
model: ""
implemented: true
---

# Review: Context Variables and Expression Function Additions

## Findings

No blocking findings.

The two issues from `review-1.md` appear addressed:

- `has_skill` / `has_local_skill` now search all four local-scoped roots for recognized agents while preserving the generic local-root fallback for unknown agents. The implementation is in `SkillRoots::local_roots`, and Level 1 tests cover cross-agent local-root lookup plus root ordering and unknown-agent behavior.
- HTTP(S) classification is now case-insensitive via URL parsing in `is_remote_url`, and Level 1 tests cover uppercase and mixed-case schemes for file-only helpers, `join`, one-argument `link`, and two-argument `link`.

## Test Rigor Assessment

This feature is expression evaluation, deterministic Prose-to-terminal-string rendering, filesystem path shaping, environment capture, and local directory discovery. I did not find any user-observable requirement that needs Level 2 real-terminal capture or Level 3 OS keyboard injection. The `terminal(string)` function returns the deterministic string emitted by `Prose`; it does not assert live terminal emulator rendering, keyboard behavior, paste behavior, scrolling, or terminal input encoding.

Requirement-to-level mapping:

- `ctx.agent` / `ctx.model` capture, trimming, defaults, descriptor wiring, and lazy `CtxLookup`: Level 1 is appropriate and present through context capture and lookup tests.
- Pure functions (`is_positive`, `is_negative`, `is_integer`, `without_date`, `ensure_leading`, `ensure_trailing`, `terminal`): Level 1 is appropriate and present through unit tests for success, null behavior, type mismatch, arity, and dispatch.
- Filesystem functions (`is_indexed_file`, `file_index`, index mutation, basename/dir/ext helpers, `join`): Level 1 is appropriate and present through unit tests covering relative paths, missing paths, zero-padded indexes, non-indexed names, separator normalization, and remote URL rejection.
- `link`: Level 1 is appropriate and present for local one-argument links, local/HTTP(S) two-argument links, link-text escaping, destination wrapping, null/type/arity behavior, and uppercase HTTP(S) schemes.
- `has_skill` / `has_local_skill`: Level 1 is appropriate and present with temporary directory roots, injected home directories, recognized/unknown agent behavior, local-vs-user scope, basename rejection, nested-directory exclusion, and missing-root behavior.
- Compose integration: Level 1 is appropriate and present through integration tests for representative interpolation (`basename`, `terminal`, `ctx.agent`) and `when=` conditions (`is_indexed_file`, `has_skill`).
- Descriptor/export catalog parity: Level 1 is appropriate and present through descriptor/signature parity tests and the exported expression catalog regression.

## Verification

- Reviewed the spec, implementation, docs, descriptor catalogs, context capture, expression dispatch, path/link/skill helpers, and regression tests.
- Ran `cargo test -p darkmatter --lib markdown::compose::expression --color=never`: 425 passed.
- Ran `cargo test -p darkmatter --test expression_regression --color=never`: 44 passed.

## Production Readiness

Ready for production.
