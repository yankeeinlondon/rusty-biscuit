---
ready: false
agent: codex
model: ""
---

# Review: Resolution Context & Token Resolution, Iteration 3

## Findings

1. **High: Claudine hook `doc.*` still does not traverse the same object returned by bare `doc`.**

   The spec makes `doc` a reserved namespace with a strict object invariant: bare `doc` is the whole root object, and `doc.<path>` is dotted traversal into that object. A missing `doc.<path>` must not fall through to any other namespace. The hook adapter still violates that contract.

   In `EventMetaExpressionLookup::get`, bare `doc` returns `event_doc_object(self.meta)`, but `doc.<rest>` strips the prefix and recursively calls `self.get(rest)` instead of traversing `event_doc_object(...)` (`claudine/lib/src/dispatch/expression.rs:91-95`). That means `doc.*` can resolve values that are not present in the object returned by `doc`. The current test locks in the mismatch by expecting `doc.git.branch` to resolve (`claudine/lib/src/dispatch/expression.rs:884-887`), while `event_doc_object` only includes the 14 top-level event keys listed in `TOP_LEVEL_KEYS` and does not include `git`, `os`, `hardware`, or `project` (`claudine/lib/src/dispatch/expression.rs:243-259`).

   This is user-observable in hook `when=` expressions and templates: `doc.git.branch` can be truthy even though `doc` has no `git` child. It also contradicts the comment immediately above the implementation that says missing `doc.<path>` never collapses into another namespace. Similar leakage exists for `doc.env.PATH`, because the recursive `self.get(rest)` enters the normal `env.*` branch even though `env` is not part of bare `doc`.

   **Fix:** make `doc.<path>` traverse the same value returned by `event_doc_object(self.meta)`, or expand `event_doc_object` to contain the complete event/environment tree that `doc.<path>` is allowed to expose. Add tests that assert `doc.<path>` and manual traversal through the bare `doc` object agree, including a path that normal lookup would otherwise resolve (`git.branch` or `env.PATH`).

## Test Rigor

- Frontmatter read-side functions and the motivating optional `spec: file` ternary: **Level 1** CLI/in-process coverage is appropriate. There is no terminal encoder/renderer behavior involved.
- `$()` token resolution, no-command diagnostics, PATH/property precedence, and branch approval discovery: **Level 1** parser/execution coverage is appropriate. The implementation has focused unit and integration tests for these paths.
- Remote URL behavior split between body-side remote reads and local-only frontmatter surfaces: **Level 1** compose tests are appropriate. The tests verify the fetch count and diagnostics at the real compose call sites.
- Optional `file` empty-as-absent behavior: **Level 1** schema tests are appropriate.
- Claudine loop/hook read-side base-dir behavior: **Level 1** is appropriate. Existing tests prove base-dir resolution for loop conditions and hook conditions.
- Claudine hook `doc.*`: **Level 1** coverage is present but asserts the wrong contract. The strongest tests verify selected `doc.*` paths resolve, but they do not verify that those paths are traversals of bare `doc`.

No Level 2 or Level 3 tests are required for this feature because the user-observable requirements are expression evaluation, file/URL resolution, schema validation, and command approval classification, not real-terminal rendering or OS keyboard input.

## Verification

Focused checks run:

- `cargo test --color=never -p claudine --lib doc_namespace`
- `cargo test --color=never -p claudine --lib read_side_function_resolves_against_base_dir`
- `cargo test --color=never -p darkmatter --lib remote_transclusion_tests`
- `cargo test --color=never -p darkmatter --lib frontmatter_shell_expansion`

All four passed.

## Notes

The darkmatter-side blockers from the earlier reviews appear resolved: frontmatter now uses a local-only resolution context, body interpolation keeps the remote runtime, and `$()` ternary conditions/branches share the local-only frontmatter context. The remaining blocker is in the Claudine hook adapter, which is still part of the spec's in-scope evaluation surfaces.
