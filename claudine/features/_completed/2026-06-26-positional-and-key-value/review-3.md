---
ready: false
agent: codex/default
created: 2026-06-25T15:18:20
implemented: true
---

# Review 3 - Positional and Key/Value Lifecycle Actions

## Verdict

Not production ready.

The parser and focused L1 lifecycle tests now cover the implementation gaps found in the earlier reviews, including missing required key/value descriptor parameters, optional descriptor tails, overloaded expression-function signatures, literal-default action values, parse-time unknown-verb checks, and removed short-form rejection. The remaining blocker is documentation correctness: the current lifecycle topic still teaches action shapes that the spec explicitly removed and the parser now rejects.

## Findings

### High - Lifecycle docs still show removed stack-item action shapes

The spec removes the stack-item-level `action: <verb>` plus sibling-keys path and says a stack item's `action:` value must contain either a positional action map or an explicit key/value action object. The implementation enforces that in `parse_lifecycle_stack_item`: any sibling parameter keys next to a scalar `action` are rejected, and direct stack items without an `action` key are rejected as missing `action`.

Current docs still show both removed shapes as accepted:

- `claudine/docs/topics/lifecycle.md:158`, `:217`, `:220`, `:240`, `:283`, `:297`, `:366`, `:380`, `:396`, and `:560` show positional action maps directly as stack items, e.g. `stack: - set_frontmatter: [...]` or `stack: - info: ...`. These should be `- action: { set_frontmatter: [...] }` for a single action, or entries inside an `action:` array.
- `claudine/docs/topics/lifecycle.md:203`, `:228`, `:250`, and `:377` show the removed stack-item-level key/value shorthand, e.g. `- action: shell` followed by sibling `command:`/`no_error:` keys. These should be nested key/value action objects, e.g. `- action: { action: shell, command: "npm run typecheck" }`, or multiline under the stack item's `action:` value.
- `claudine/docs/topics/lifecycle.md:127-138` is especially risky because it is in the canonical "Action Forms" section and demonstrates key/value form with the removed stack-item-level shorthand.

This is user-facing. Authors copying the docs will get `LifecycleStackInvalidShape` instead of a working lifecycle action, even though the same page claims it documents the new two-form grammar. It also violates the migration acceptance criterion that lifecycle docs describe exactly the two supported forms.

Verification level: documentation/regression-sweep gap. No L2/L3 terminal behavior is required for this finding. The relevant implementation behavior is covered at Level 1 by parser tests such as `rejects_stack_item_missing_action_key` and `rejects_unknown_stack_item_key`; the docs need to be brought back into alignment with that behavior.

Fix direction: rewrite every stack example so each stack item has an `action:` key. Use `- action: { verb: value }` for single positional actions, `- action: { action: verb, param: value }` for single key/value actions, and `- action: [ ... ]` only when multiple actions share one `when` gate.

## Verification Notes

- Ran `cargo nextest run --color=never -p claudine -E 'test(/composition::lifecycle::tests::/) + test(/composition::lifecycle_actions::tests::/)'`: 203 passed.
- Strongest relevant verification present:
  - Positional/key-value parsing, literal defaults, typed whole-value arguments, unknown verbs, short-form rejection, control placement/cardinality, key/value missing required parameters, optional tails, overload merging, and variadic key/value rejection: Level 1.
  - Mixed positional/key-value compose execution, typed side-effect write-through, and key/value literal defaults through a real tmux pane: Level 2 via `claudine/cli/tests/level2_lifecycle_action_forms.rs`.
- No Level 3 requirements are implied by this feature; the spec does not require OS keyboard injection or terminal input-encoder behavior.
