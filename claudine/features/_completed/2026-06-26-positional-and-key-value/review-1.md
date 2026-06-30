---
ready: false
agent: codex/default
created: 2026-06-25T12:32:08
implemented: true
---

# Review 1 - Positional and Key/Value Lifecycle Actions

## Verdict

Not production ready.

The implementation covers the main positional/key-value grammar and has a strong L1 parser suite plus L2 tmux compose coverage for mixed-form execution. However, two parse-time contract gaps remain in the descriptor-driven action paths. Both are Level 1 requirements: they are deterministic parser/shape rules and should fail before execution.

## Findings

### High - key/value side-effect and expression-function actions do not enforce required parameters

Key/value action parsing consumes whichever named parameters are present, rejects unknown extras, and then builds the action without checking that required descriptor parameters were supplied. For expression functions, `build_action_from_params` walks `signature.params` and pushes only keys found in `params_map`, then returns an `ExpressionFunctionAction` with a shortened `args` vector (`claudine/lib/src/composition/lifecycle.rs:2315`). Side effects have the same issue (`claudine/lib/src/composition/lifecycle.rs:2346`).

That means malformed actions like these parse successfully and fail later during execution:

```yaml
- action: { action: contains, haystack: "{{ haystack }}" }
- action: { action: set_frontmatter, file: "state.md" }
```

The spec requires key/value form to match concrete descriptor parameter names unambiguously and requires useful parse-time diagnostics for action shape errors. This implementation lets invalid user-authored lifecycle stacks reach runtime, which loses the frontmatter-excerpt/highlight path and weakens pre-flight validation.

Verification level: L1 parser gap. Existing L1 tests cover the happy path for `{ action: contains, haystack, needle }` and positional wrong arity, but I found no L1 tests asserting missing required key/value params are rejected for either expression functions or side effects.

Fix direction: after collecting args from a descriptor signature, enforce `required_count()` through `max_count()` and report a typed `CompositionError` listing missing required parameter names. Add L1 tests for missing `needle` on `contains` and missing `prop`/`value` on `set_frontmatter`.

### High - expression-function signature parsing mishandles optional and overloaded catalog signatures

The new `parse_signature` helper only recognizes optional params with a trailing `?` (`claudine/lib/src/composition/lifecycle_actions.rs:381`). Darkmatter's expression-function catalog uses bracket notation for optional defaults, for example `number(x, [default])` and `round(x, [default])` (`darkmatter/lib/src/markdown/compose/expression/catalog.rs:536`). Those are parsed as two required parameters, including the literal parameter name `[default]`.

Overloaded expression functions are also merged by replacing the param list with the longest signature while leaving `optional_tail` at its previous value (`claudine/lib/src/composition/lifecycle_actions.rs:600`). For overloads such as `frontmatter(file)` / `frontmatter(file, prop)`, `link(file)` / `link(target, desc)`, and `validate_schema(file)` / `validate_schema(file, obj)`, the one-argument positional form is therefore treated as wrong arity even though the catalog and runtime support it.

This violates the acceptance criterion that all action families remain reachable and that positional arrays are zipped against the canonical signature, accepting optional-tail arities where supported.

Verification level: L1 parser gap. Existing L1 expression-function tests cover `length`, `contains`, `and`, and `or`, but not optional/default or overloaded expression-function signatures.

Fix direction: make the signature helper understand Darkmatter's existing optional syntax (`[default]`) and merge expression-function overloads the same way side-effect overloads are merged. Add L1 tests for `number: "{{ value }}"`, `frontmatter: "state.md"`, and `link: "state.md"` parsing with one argument.

## Verification Notes

- Ran `cargo nextest run --color=never -p claudine -E 'test(/composition::lifecycle::tests::/)'`: 176 passed.
- Strongest relevant verification present:
  - Grammar parsing, literal defaults, short-form rejection, known-verb checks: Level 1.
  - Mixed positional/key-value compose execution, typed write-through, key/value literal default in a real tmux pane: Level 2 via `claudine/cli/tests/level2_lifecycle_action_forms.rs`.
- No Level 3 requirements are implied by this feature; there is no OS keyboard or terminal input-encoder behavior in the spec.
