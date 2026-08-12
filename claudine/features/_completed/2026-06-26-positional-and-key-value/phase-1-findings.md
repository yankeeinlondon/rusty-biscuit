---
phase: 1
plan: claudine/features/2026-06-26-positional-and-key-value/plan.md
date: 2026-06-26
---

# Phase 1 Findings — Positional and Key/Value Lifecycle Actions

This document captures the baseline orientation and scope inventory for the
positional-and-key-value lifecycle action grammar. No code changes are made in
Phase 1.

## Workspace Members Touched

From `cargo metadata --no-deps --format-version 1`, the workspace members in the
`claudine` package area are:

- `claudine` — library crate (`claudine/lib/Cargo.toml`)
- `claudine-cli` — CLI crate (`claudine/cli/Cargo.toml`)
- `claudine-contract` — contract crate (`claudine/contract/Cargo.toml`)

The feature implementation lives almost entirely in `claudine` (lib).
`claudine-cli` consumes the new grammar through existing composition commands,
and `claudine-contract` is outside the lifecycle parsing surface.

## In-Scope Code Paths

### `claudine/lib/src/composition/lifecycle.rs`

| Symbol / Branch | Line Range | Notes |
|-----------------|------------|-------|
| `parse_lifecycle_stack_item` | 1424–1608 | Entry point for every stack item; contains the sibling-keys path and the `action:` value match arms. |
| Sibling-key collection | 1469–1502 | Collects everything except `when`/`action`/`no_error` into `sibling_params`; these apply only when `action` is a scalar string. This path is **removed** in Phase 5. |
| Scalar-string `action:` branch | 1504–1512 | Dispatches to `parse_scalar_action`; handles short form (`verb(args)`) and bare-verb-with-siblings. Kept as the bare-verb zero-arg path in Phase 5. |
| Array string-element branch | 1526–1528 | Dispatches each string element to `parse_short_form_action`. Becomes the bare-verb zero-arg path in Phase 5, rejecting strings containing `(`. |
| Single-object `action:` rejection | 1548–1559 | Currently rejects `action: { ... }`; Phase 4 reworks this to accept positional `{ verb: value }`. |
| `parse_scalar_action` | 1613–1658 | Branches on `(` to short form vs. bare-verb long form. Short-form branch removed in Phase 5. |
| `parse_long_form_action_object` | 1664–1726 | Parses `{ action: verb, ... }` key/value objects. Parameter values currently route through `value_to_expr`; Phase 5 switches to `action_value_to_expr`. |
| `parse_short_form_action` | 1729–1828 | Full `verb(args)` parser. Deleted in Phase 5. |
| `is_single_text_arg_verb` | 1839–1842 | Multi-arg branch removed in Phase 5; single-text classification becomes irrelevant. |
| `unwrap_wrapping_quotes` | 1850–1862 | Used only by short-form literal parsing. Deleted in Phase 5. |
| `value_to_expr` | 1875–1900 | Legacy expression-fallback converter. Replaced by `action_value_to_expr` for lifecycle parameters in Phase 3/5. |
| `parse_action_arg` | 1911–1946 | Parses one short-form argument as a Darkmatter expression. Deleted in Phase 5. |
| `has_unquoted_whitespace` | 1950–1969 | Used only by `parse_action_arg`. Deleted in Phase 5. |
| `build_action` | 1974–2031 | Dispatches parsed verb + positional args to action builders. |
| `build_action_from_params` | 2033–2136 | Dispatches long-form named parameters to action builders. |
| `parse_lifecycle_control_short` | 2138–2179 | Short-form control-verb dispatcher (`stop`, `skip`, `error`, `proxy`, `retry`, `resume`, `defer`). |
| `parse_lifecycle_control_long` | 2182–2244 | Key/value control-verb dispatcher. |

### `claudine/lib/src/composition/lifecycle_actions.rs`

| Symbol | Line Range | Notes |
|--------|------------|-------|
| `LifecycleControlAction` | 104–158 | Control verbs and their runtime shapes. |
| `CommunicationChannel` | 242–298 | Communication verbs (`say`, `speak`, `effect`, `message`, `notify`, `stderr`, `info`, `warn`, `success`, `stdout`). |
| `side_effect_signature` | 353–370 | Hand-maintained positional parameter table for Darkmatter side-effect verbs. Phase 2 replaces or derives this from `EFFECT_DESCRIPTORS`. |
| `is_known_side_effect` | 380–382 | Predicate used by the executor to route side-effect verbs. |

### `claudine/lib/src/composition/error.rs`

| Symbol | Line Range | Notes |
|--------|------------|-------|
| `LifecycleStackInvalidShape` | 325–342 |
| `LifecycleActionInvalidShortForm` | 344–363 | Reused by the new short-form-removed did-you-mean error in Phase 5, or replaced by a new variant. |
| `LifecycleActionInvalidLongForm` | 365–380 |
| `LifecycleActionPlacement` | 382–400 |
| `LifecycleMultipleLifecycleActions` | 402–417 |
| `LifecycleActionOrder` | 419–433 |
| `LifecycleInvalidArgs` | 435–454 | Wrong-arity positional errors will reuse this shape. |
| `LifecycleErrNotAvailable` | 456+ | Existing err-placement scan. |

### `claudine/lib/src/composition/frontmatter_excerpt.rs`

| Symbol | Line Range | Notes |
|--------|------------|-------|
| `FrontmatterExcerpt` | 26–82 | Captures frontmatter block + highlight line; renders TTY appendix and strips escapes at `ColorDepth::None`. |
| `capture_frontmatter_block` | 96–114 | |
| `locate_property_line` | 130–169 | Dotted property resolution for highlighting. |

## Out-of-Scope Loop Surface

`claudine/lib/src/composition/loop_config.rs` uses `split_action_args` at line
391 for the loop `action:` DSL (`increment`, `decrement`, `set`, `append`,
`prepend`, `merge`). This is a separate surface from the lifecycle stack
`action:` value. `split_action_args` **must stay** even after short-form removal
from lifecycle stacks.

The loop DSL grammar is unchanged by this feature.

## Darkmatter Descriptor Catalogs

### Side-Effect Catalog (`darkmatter/lib/src/effects/catalog.rs`)

`EFFECT_DESCRIPTORS` defines the following canonical signatures:

- `set_frontmatter(file, prop, value)`
- `merge_frontmatter(file, obj)`
- `delete_frontmatter(file, prop)`
- `increment_frontmatter(file, prop)`
- `decrement_frontmatter(file, prop)`
- `append_frontmatter(file, prop, value)`
- `prepend_frontmatter(file, prop, value)`
- `ensure_file(file)`
- `ensure_file(file, content)`
- `ensure_dir(dir)`
- `append_line(file, text)`
- `append_jsonl(file, obj)`
- `http_post(url, body)`

Decision #6 requires a small signature parser that turns these canonical
strings into typed shapes (verb name, ordered positional names, optional-tail
flags, variadic flag) rather than ad-hoc splits.

### Expression-Function Catalog (`darkmatter/lib/src/markdown/compose/expression/catalog.rs`)

`EXPRESSION_FUNCTION_DESCRIPTORS` defines canonical signatures such as:

- Variadic: `and(...)`, `or(...)`
- Concrete multi-arg: `contains(haystack, needle)`, `starts_with(x, find)`,
  `set_frontmatter(file, prop, value)` (read-side mirror), `validate_schema(file, obj)`,
  `validate_schema(file)`, `join(left, right)`, `number(x, [default])`,
  `round(x, [default])`, `link(file)`, `link(target, desc)`, `date(iso, fmt)`
- Single-arg: `length(x)`, `is_string(x)`, `file_exists(file)`, etc.

The `is_known_lifecycle_verb` predicate in Phase 2 must union these verbs with
communication channels, `shell`, lifecycle-control verbs, and side-effect verbs.

## `value_to_expr` Baseline

Current behavior in `lifecycle.rs:1875-1900`:

- `String` values are parsed with Darkmatter's `parse()`.
  - If parsing succeeds, the resulting `Expr` is stored (e.g. `"ctx.repo"` →
    `Expr::Context`, `"3"` → `Expr::NumberLiteral(3.0)`).
  - If parsing fails, the value is stored as `Expr::StringLiteral(s)`.
- `Number` values become `Expr::NumberLiteral(f64)`.
- `Bool` values become `Expr::BoolLiteral`.
- `Null`, `Array`, and `Object` values are rejected.

This is the expression-fallback behavior. Phase 3 intentionally changes it to
literal-default: a string becomes `Expr::StringLiteral` unless its trimmed
content is exactly one `{{ ... }}` span, in which case it resolves to the
expression's typed value via Darkmatter whole-value expansion. Numeric and
boolean YAML scalars map to typed literals. Direct object/array values are
rejected (must come through whole-value interpolation).

## Existing L1 Tests Asserting Short-Form Acceptance

The `lifecycle.rs` `mod tests` block (starts at line 3430) contains 141 tests.
The following tests currently assert that `verb(args)` short form parses and
will need to flip to asserting the short-form-removed error in Phase 5:

- `parses_short_form_say_action` (line 5018) — `say('hello world')`
- `single_text_arg_is_taken_literally` (line 5036) — `say(ctx.repo)`
- `parses_when_condition_with_stack` (line 5054) — `say('using claude')`
- `parses_multiple_actions_per_stack_item` (line 5071) — `say('first')`, `info('second')`
- `parses_retry_with_count_in_blocked` (line 5119) — `retry(3)`
- `parses_proxy_with_file_arg_in_initialize` (line 5133) — `proxy('@fallback.md')`
- `parses_side_effect_short_form` (line 5189) — `ensure_file('@out/log.md')`
- `flow_control_is_universal_across_events` (line 5217) — `proxy('@other.md')`, `resume('...')`, `defer('5m')`, `retry(2)`
- `accepts_recovery_actions_in_finalize` (line 5242) — `retry(1)`, `resume('...')`, `defer('5m')`, `proxy('@other.md')`
- `accepts_lifecycle_action_as_last` (line 5293) — `say('one')`
- `accepts_unquoted_multi_word_literal_in_single_text_short_form` (line 5311) — `say(using codex)`, `warn(phase 6, too big)`, `error('invalid phase: 6')`, `effect(crowd-applause)`
- `parses_stdout_short_form_action` (line 5440) — `stdout('hello')`
- `err_in_start_stack_when_clause_is_rejected` (line 5601) — `say('has error')`
- `err_member_access_in_single_text_arg_is_literal` (line 5623) — `say(err.msg)`
- `err_in_single_text_arg_is_literal_across_no_error_events` (line 5638) — `say(err)`
- `err_in_blocked_failure_finalize_is_allowed` (line 5664) — `say(err.msg)`
- `doc_err_escape_hatch_is_allowed_everywhere` (line 5681) — `say(doc.err)`
- `err_in_control_reason_single_text_arg_is_literal` (line 5708) — `error(err.msg)`
- `err_in_shell_command_single_text_arg_is_literal` (line 5722) — `shell(err.msg)`
- `err_interpolation_span_in_stack_message_rejected_in_no_error_event` (line 5754) — `message(❌️ {{err.msg}})`
- `stack_string_literal_with_interpolation_span_is_leak` (line 5814) — `say('leaked {{ broken( }}')`
- `stack_undefined_variable_in_when_clause_is_rejected` (line 5907) — `say('hi')`
- `stack_err_global_is_not_undefined_in_failure` (line 5930) — `say(err.msg)`
- `stack_timing_and_current_globals_are_not_undefined` (line 5947) — `say(timing.document_ms)`, `say(current.ctx.agent)`
- `stack_bare_token_in_action_arg_is_literal_not_undefined_variable` (line 5964) — `say(missing_var)`
- `collect_lifecycle_shell_commands_extracts_literal_commands` (line 6032) — `say('not a shell command')`
- `no_error_flag_is_accepted_on_every_action_category` (line 6097) — `say('hi')`, `length('hello')`
- `no_error_on_scalar_form_threads_to_every_category` (line 6125) — `say('hi')`
- `no_error_defaults_to_false` (line 6143) — `say('hi')`

Tests that use bare-verb zero-arg form (`stop`, `skip`) do **not** need to flip;
they remain valid positional zero-arg actions.

Tests that assert short-form *errors* will also change shape:

- `rejects_retry_with_too_many_args` (line 5352) — `retry(3, 4)` becomes a
  short-form-removed error rather than a wrong-arity error.
- `rejects_proxy_missing_target` (line 5366) — bare `proxy` becomes a positional
  wrong-arity error rather than a long-form missing-parameter error.
- `rejects_missing_closing_paren` (line 5338) — `say('hi'` becomes a
  short-form-removed error.
- `rejects_unknown_stack_item_key` (line 5396) — `action: "stop", bogus: true`
  becomes the ambiguous multi-key-no-`action:` error.

## Migration Target List

### `claudine/docs/topics/lifecycle.md`

`rg` reports 38 short-form hits (lines 23, 43, 86, 87, 101, 102, 103, 111, 112,
117, 119, 124, 125, 152, 153, 154, 155, 156, 185, 188, 221, 240, 251, 265,
321, 322, 334, 335, 348, 364, 380, 381, 398, 404, 408, 422, 528). Phase 7 must
rewrite every `verb(args)` example into positional or key/value form and update
the Action Forms / Flow Control sections.

### `claudine/docs/topics/composition.md`

One hit on line 105: the `shell(...)` parenthetical in the deferred-lifecycle-keys
paragraph. Phase 7 updates it to the long-form `command:` shape or removes the
short-form parenthetical.

### Skill Docs

Both `.opencode/skill/claudine/` and `.claude/skills/claudine/` must be updated:

- `SKILL.md` (line ~118) — contains `message(❌️ {{err.msg}})` and `shell(...)`
  references in the late-binding paragraph.
- `cli-reference.md` (lines ~594–599) — contains `message()`, `info()`,
  `warn()`, `error()` table rows.
- `timeline.md` — add a `2026-06-26 — positional-and-key-value` entry.

### Prompts

`claudine/prompts/` contains only `new-provider.md` and no short-form examples.
The spec-referenced `claudine/prompts/review-feature.md` does **not** exist in
tree; Phase 7 must not create it as a side effect. No prompt migration is
required.

## Summary

Phase 1 confirms the scope: implement positional and key/value grammar in
`claudine/lib/src/composition/lifecycle.rs` and `lifecycle_actions.rs`, keep the
loop surface untouched, derive verb validation from the Darkmatter catalogs, and
migrate documentation in Phase 7. The test inventory above is the checklist for
flipping short-form acceptance to short-form rejection in Phase 5.
