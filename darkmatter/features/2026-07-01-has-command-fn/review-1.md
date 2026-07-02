---
ready: false
agent: codex/default
created: 2026-07-01T15:26:40
implemented: true
---

# Review 1 — `has_command(cmd)`

## Findings

### High — The feature is not implemented

The spec requires a new filesystem expression function `has_command(cmd)` with `hascommand` as an alias, registered in `FS_FUNCTIONS`, and implemented by delegating to `which::which` without executing the target command. The current implementation has no `has_command_fn` handler and no `FS_FUNCTIONS` registration next to `file_exists`; the filesystem function table still goes directly from `file_exists` to `is_indexed_file` in `darkmatter/lib/src/markdown/compose/expression/functions.rs`.

Impact: every user-facing requirement in the spec is currently unavailable. Expressions such as `has_command("ls")` and `hascommand("cmd")` cannot dispatch, so present-binary, bogus-binary, null/non-string, absolute executable, non-executable, directory, tilde, relative-path, and alias behavior are all missing.

Required fix: add `has_command_fn` modeled on `file_exists_fn`, keep `require_args_expr("has_command", args, 1)`, return `false` for `null`, non-string, empty string, unfound, and non-executable inputs, and register `FsFunction { canonical: "has_command", aliases: &["hascommand"], signatures: &["has_command(cmd)"], handler: has_command_fn }`.

Verification level: none present. This feature is not terminal-rendering or keyboard-input behavior, so Level 1 unit/integration tests are the appropriate minimum. Those tests are absent because the implementation is absent.

### High — Descriptor catalog and generated documentation are missing

The spec requires `has_command(cmd)` in `EXPRESSION_FUNCTION_DESCRIPTORS` under the Filesystem category and regeneration of the generated function table in `darkmatter/docs/topics/darkmatter-expressions.md`. The catalog currently lists `absolute`, `relative`, `file_exists`, then `frontmatter`; the generated table mirrors that and has no `has_command(cmd)` row.

Impact: even after adding the handler, descriptor/dispatch parity tests would fail until the catalog row is added. The public expression reference would also omit the function.

Required fix: add a non-host-dependent descriptor for `has_command(cmd)`, then regenerate the expression docs table with the existing recipe. Keep any example display-only or empty to avoid host-dependent documentation tests.

Verification level: none present. The existing Level 1 catalog parity and generated-doc tests are the appropriate verification surface once the descriptor exists.

### High — Required behavior tests are absent

The Definition of Done requires tests for found and missing commands, null and non-string inputs, empty string, absolute executable and non-existent paths, Unix non-executable paths, directories, tilde and relative-path gaps, and both canonical and alias dispatch. None of those tests exist under the expression function tests or CLI tests.

Impact: the highest verification level for all user-observable behavior is currently none, which is below the required Level 1 minimum. Cross-platform behavior is especially unverified: Windows `PATHEXT` and Unix executable-bit semantics are supposed to be delegated to `which`, but there is no test guard or cfg split proving the expected contract without making CI host-dependent.

Required fix: add focused Level 1 tests beside the existing `file_exists_fn` tests. Prefer temp fixtures and `which::which`-guarded assertions over unconditional assumptions about specific binaries. Add dispatch tests for both `has_command` and `hascommand`.

Verification level: none present. Level 1 is sufficient for this feature because it is an expression evaluation/API behavior, not terminal rendering or OS keyboard input.

### Medium — Filesystem helper prose does not describe the new function or its deliberate gaps

The spec requires prose in `darkmatter/docs/topics/darkmatter-expressions.md` describing that `has_command(cmd)` is a PATH/executable probe that never executes and needs no whitelisting, with explicit notes for no tilde expansion, no relative-path resolution, Windows `PATHEXT`, Unix executable bits, symlinks, directories, and no remote URL argument. The `#### Filesystem Helpers` table has no row for `has_command`.

Impact: users would not know the important boundary conditions even after the function is added. The tilde and relative-path behavior are easy to misread as bugs unless documented.

Required fix: add a concise table row and prose near `file_exists(path)` that captures the execution-safety and path-semantics contract.

Verification level: none present. Documentation table/prose coverage is Level 1 via the existing generated-doc checks plus ordinary review.

## Open Questions

None. The spec is concrete enough to implement directly.

## Production Readiness

Not ready for production. The requested feature is specified, but the implementation, dispatch surface, descriptor/docs, and required Level 1 tests are all missing from the current tree.
