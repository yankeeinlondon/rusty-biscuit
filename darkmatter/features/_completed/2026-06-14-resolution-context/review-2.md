---
ready: false
agent: codex
model: ""
---

# Review: Resolution Context & Token Resolution, Iteration 2

## Findings

1. **High: Claudine hook `doc.*` is not resolved from the same object as bare `doc`.**

   The spec defines `doc` as the whole root object and `doc.<path>` as dotted traversal into that object. The hook adapter documents the same invariant: bare `doc` is the whole event object and missing `doc.<path>` must not collapse into another namespace. The implementation does not enforce that shape. `EventMetaExpressionLookup::get("doc.<rest>")` strips the prefix and calls `self.get(rest)` (`claudine/lib/src/dispatch/expression.rs:91-96`), which runs the normal lookup ladder instead of traversing `event_doc_object(...)`.

   That makes `doc.*` capable of resolving values that are absent from bare `doc`. The current test expects `doc.git.branch` to resolve (`claudine/lib/src/dispatch/expression.rs:884-887`), but `event_doc_object` only includes the top-level keys listed in `TOP_LEVEL_KEYS` (`claudine/lib/src/dispatch/expression.rs:243-255`) and does not include `git`. A user can therefore observe inconsistent semantics: `doc.git.branch` resolves while the object returned by `doc` has no `git` field. This violates the feature's `doc` / `doc.<path>` contract and is easy to miss because the existing negative test only checks `doc.missing`.

   **Fix:** build the event `doc` object once and resolve `doc.<path>` by dotted traversal into that object, or expand `event_doc_object` to include the same nested event/environment shape that `doc.<path>` is allowed to address. Add tests that assert `doc.<path>` and traversal through `doc` expose the same tree, including a negative case for a path that normal lookup would otherwise resolve.

## Test Rigor

- Frontmatter read-side functions, including the original `file_exists(possible_spec)` workflow: **Level 1** CLI/in-process coverage is appropriate. The tests exercise the frontmatter pass and optional `file` validation without terminal rendering or keyboard behavior.
- Frontmatter remote URL rejection: **Level 1** coverage now matches the spec. `frontmatter_resolution_context()` omits `remote_fetch` (`darkmatter/lib/src/markdown/compose/types.rs:1241-1246`), both frontmatter interpolation passes use it (`darkmatter/lib/src/markdown/compose/mod.rs:626-631`, `:700-705`), and `frontmatter_value_remote_url_fails_loudly` asserts no fetch occurs plus a warning is emitted (`darkmatter/lib/src/markdown/compose/mod.rs:5815-5857`).
- `$()` token resolution and no-command diagnostics: **Level 1** parser/execution coverage is appropriate. The validity rule is implemented at parse time (`darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:266-307`) with targeted tests for non-ternary and ternary expression-only `$()` values (`:2313-2342`).
- Optional `file` empty-as-absent behavior: **Level 1** schema coverage is appropriate. Optional scalar `file` fields wrap the file fragment in an empty-string arm (`darkmatter/lib/src/markdown/schemas/simplified/convert.rs:199-215`) and required files remain strict (`:694-718`).
- Claudine hook `doc.*`: **Level 1** coverage is present but incomplete/wrong for the object traversal contract. The strongest test verifies selected paths resolve, but it does not verify that `doc.<path>` is a traversal of the bare `doc` object.

## Verification

Focused checks run:

- `cargo test --color=never -p darkmatter --lib remote_transclusion_tests`
- `cargo test --color=never -p darkmatter --lib frontmatter_shell_expansion`
- `cargo test --color=never -p claudine --lib doc_namespace`

All three passed.

## Notes

The previous review's frontmatter remote-read blocker appears fixed in darkmatter proper. The remaining issue is isolated to the Claudine hook adapter's `doc` namespace semantics, but it is part of the spec's in-scope surface list, so I would not mark this production-ready yet.
