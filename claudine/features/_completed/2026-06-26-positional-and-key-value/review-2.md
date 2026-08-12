---
ready: false
agent: codex/default
created: 2026-06-25T12:55:20
implemented: true
---

# Review 2 - Positional and Key/Value Lifecycle Actions

## Verdict

Not production ready.

The review-1 parser findings appear to be fixed: key/value descriptor actions now reject missing required parameters at parse time, and expression-function signatures now handle bracket-optional parameters plus overloads. Focused L1 lifecycle tests pass. The remaining blocker is the migration/documentation contract: several current docs still describe removed `shell(...)` short form as an accepted lifecycle shell-action surface.

## Findings

### High - Current docs still advertise removed `shell(...)` lifecycle short form

The spec requires the migration to rewrite lifecycle and composition docs to the two-form grammar and states that docs/examples should no longer reference `verb(args)` short form as an accepted action shape. The implementation rejects `verb(args)` correctly, but these current docs still say lifecycle shell pre-flight scans `shell(...)` short-form actions:

- `claudine/docs/topics/frontmatter-properties.md:30` calls `shell(...)` commands the lifecycle early-binding exception and pairs them with key/value `command:`.
- `claudine/docs/topics/pre-flight-checks.md:11`, `:52`, and `:161` say lifecycle shell actions include `shell(...)` short-form and long-form `shell:` items.
- `.claude/skills/claudine/timeline.md:12` is historical context, but it is still part of the authoritative skill docs and says ``shell(...)` is the early-binding exception` without a superseding note on that line. The immediately newer timeline entry is correct, so this is a smaller risk than the topic docs, but it still shows up in the required skill-doc sweep.

This is user-facing because authors following the pre-flight/frontmatter references will write a lifecycle shell action that now fails with `LifecycleShortFormRemoved`. It also leaves the docs out of sync with the production behavior and the spec's migration acceptance criteria.

Verification level: documentation/regression-sweep gap. No L2/L3 behavior is required for this finding; the relevant check is a source/doc sweep plus L1 short-form rejection, and the L1 rejection coverage is present.

Fix direction: update those references to describe positional `shell: "..."` and key/value `{ action: shell, command: "..." }` only. In historical timeline entries, either rewrite the stale sentence to mention the grammar was superseded or add an explicit superseded note so the current skill docs do not teach the removed form.

## Verification Notes

- Ran `cargo nextest run --color=never -p claudine -E 'test(/composition::lifecycle::tests::/) + test(/composition::lifecycle_actions::tests::/)'`: 203 passed.
- Strongest relevant verification present:
  - Positional/key-value parsing, missing required descriptor params, optional/overloaded signatures, literal defaults, short-form rejection, known-verb checks: Level 1.
  - Mixed positional/key-value compose execution, typed side-effect write-through, and key/value literal default through a real tmux pane: Level 2 via `claudine/cli/tests/level2_lifecycle_action_forms.rs`.
- No Level 3 requirements are implied by this feature; the spec does not require OS keyboard or terminal input-encoder behavior.
