---
phases: 6
created: 2026-04-29
start_phase: 1
packages:
  - claudine
  - claudine-cli
  - darkmatter
source_files_during_phase_1:
  - claudine/lib/src/events/event_meta.rs
  - claudine/lib/src/dispatch/template.rs
  - claudine/lib/src/dispatch/matcher.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/dispatch/template.rs
  - claudine/lib/src/dispatch/runner.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/actions/hook_action.rs
  - claudine/lib/src/dispatch/runner.rs
  - claudine/lib/src/dispatch/loader.rs
  - claudine/lib/src/events/config.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/dispatch/matcher.rs
  - claudine/lib/src/dispatch/loader.rs
  - claudine/lib/src/dispatch/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/lib/src/harness/validate.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - claudine/cli/README.md
  - claudine/docs/topics/validations-and-handlers.md
  - claudine/.claude/skills/claudine/SKILL.md
  - claudine/features/2026-04-29-leverage-dm-parser/plan.md
docs_updated_during_phase_6:
  - claudine/cli/README.md
  - claudine/docs/topics/validations-and-handlers.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - claudine/.claude/skills/claudine/SKILL.md
---

# Leverage Darkmatter Parser in Claudine - Execution Plan

Source document:

- `claudine/features/2026-04-29-leverage-dm-parser/spec.md`

Goal: replace Claudine's ad-hoc event template and conditional logic with Darkmatter's exposed expression parser where it gives immediate DRY and user-facing value, while leaving lower-value reporting/linking expression filters as explicitly deferred follow-up work.

## Phase Index

| Phase | Outcome | Depends on |
| --- | --- | --- |
| 1 | Shared `EventMeta` expression bridge proves Darkmatter can resolve Claudine event data | none |
| 2 | Dispatch templates use Darkmatter interpolation expressions with legacy compatibility | 1 |
| 3 | Hook actions support optional `when` conditions evaluated before execution | 1, 2 |
| 4 | Event binding matchers support expression mode with regex fallback | 1, 3 |
| 5 | Harness validation messages use the shared expression renderer | 1, 2 |
| 6 | Docs, skill notes, and explicit deferral of reporting/linking expression filters are complete | 2-5 |

## Phase 1: Shared EventMeta Expression Bridge

Outcome: Claudine has one tested adapter from `EventMeta` to Darkmatter expression evaluation, usable by templates, hook `when`, matchers, and later harness/reporting work.

Files:

- `claudine/lib/src/events/event_meta.rs`
- `claudine/lib/src/dispatch/template.rs`
- `claudine/lib/src/dispatch/matcher.rs`

Steps:

- [ ] Add a small internal expression helper, preferably in a new module such as `claudine/lib/src/dispatch/expression.rs` if reuse from dispatch and harness is cleaner than embedding it in `template.rs`.
- [ ] Define an `EventMetaExpressionLookup<'a>` wrapper around `&'a EventMeta` that implements `darkmatter::markdown::compose::expression::EvaluationLookup`.
- [ ] Resolve top-level event fields exactly as current template keys do: `provider`, `event`, `timestamp`, `session_id`, `cwd`, `tool_name`, `tool_input`, `tool_response`, `error`, `prompt`, `agent_type`, `notification_type`, `notification_message`.
- [ ] Resolve existing grouped template paths against `EventMeta.env`: `os.*`, `hardware.*`, `git.*`, and `project.*`.
- [ ] Resolve `extra.*` against `EventMeta.extra`, preserving JSON scalar/object/array values for Darkmatter functions such as `length(...)`.
- [ ] Let Darkmatter keep ownership of `env.*` and `ctx.*` semantics. Do not duplicate environment-variable fallback logic in Claudine.
- [ ] Add tests that evaluate variables, booleans, comparisons, fallbacks, ternaries, helper functions, nested `tool_input` paths, and missing keys through the shared bridge.

Parallelizable:

- Field mapping tests can be written in parallel with the lookup implementation once the wrapper type name and module location are fixed.
- `extra.*` and nested JSON path handling can be implemented independently from the typed `EventMeta.env` mappings.

Validation checkpoint:

- `cargo test -p claudine dispatch::expression`
- `cargo test -p claudine events::event_meta`

## Phase 2: Dispatch Template Interpolation

Outcome: `interpolate()` keeps existing simple templates working and gains Darkmatter expression support for fallbacks, comparisons, ternaries, and functions.

Files:

- `claudine/lib/src/dispatch/template.rs`
- `claudine/lib/src/dispatch/runner.rs`

Steps:

- [ ] Replace `HANDLEBARS_RE` replacement with Darkmatter's expression finder/parser path in interpolation parse mode, using the shared `EventMetaExpressionLookup`.
- [ ] Preserve `rewrite_legacy_single_brace_placeholders()` for one compatibility cycle so `{provider}` still rewrites to `{{provider}}` with the existing warning.
- [ ] Remove or stop using `resolve_expression()`, `resolve_env_expression()`, and `parse_default_literal()` once Darkmatter handles `env.NAME || "fallback"`.
- [ ] Convert evaluated JSON values to template strings with Darkmatter's scalar conversion behavior; confirm booleans render as `true`/`false` and missing optional fields render as empty strings only where the previous public contract required that.
- [ ] Preserve malformed or unknown interpolation tokens unchanged, matching current `interpolate()` behavior, unless Darkmatter already provides a richer non-panicking error path that can do the same.
- [ ] Add regression tests for existing templates: `{{provider}}`, `{{git.branch}}`, `{{hardware.cores}}`, `{{env.CLAUDINE_TEST_VAR | "fallback"}}` if legacy pipe syntax is intentionally retained, and single-brace rewrite.
- [ ] Add new tests for `{{git.is_dirty ? "dirty" : "clean"}}`, `{{hardware.cores > 8 ? "fast" : "slow"}}`, `{{env.CI || "local"}}`, and `{{length(git.branch) > 30 ? "long-branch" : git.branch}}`.
- [ ] Run at least one dispatch runner test that exercises `Speak`, `Bash.params`, `ReportHandler.template`, and `Message.message` interpolation through the public action paths.

Parallelizable:

- Template unit tests and runner-level action tests can be written in parallel after the expression bridge compiles.
- Cleanup of dead helper functions can happen after the first passing interpolation tests.

Validation checkpoint:

- `cargo test -p claudine dispatch::template`
- `cargo test -p claudine dispatch::runner`

## Phase 3: Hook Action `when`

Outcome: every hook action can opt into a Darkmatter boolean condition without changing action ordering, blocking semantics, or existing configs.

Files:

- `claudine/lib/src/actions/hook_action.rs`
- `claudine/lib/src/dispatch/runner.rs`
- `claudine/lib/src/dispatch/loader.rs`
- `claudine/lib/src/events/config.rs`

Steps:

- [ ] Extend each `HookAction` variant with `#[serde(default, skip_serializing_if = "Option::is_none")] when: Option<String>`. Because the enum uses `deny_unknown_fields`, this is the schema-enabling change.
- [ ] Add a `HookAction::when(&self) -> Option<&str>` accessor so runner code can check conditions without repeating a match over every variant.
- [ ] Add tests proving old JSON action configs still deserialize and new action configs with `when` round-trip for every variant.
- [ ] Before executing each action in `execute_actions`, evaluate `when` against the current `EventMeta` serialized to `serde_json::Value` using `darkmatter::markdown::compose::conditions::evaluate_condition_against`.
- [ ] Treat a false condition as a skipped action and emit a debug trace with `action_index`, `action_kind`, and the condition string.
- [ ] Treat an invalid condition as a non-fatal skipped action with a warning, unless product requirements prefer hard failure. Record this behavior in tests and docs.
- [ ] Ensure skipped `Call` actions cannot produce or replace a blocking `HookResponse`.
- [ ] Add runner tests for true condition executes, false condition skips, invalid condition warns/skips, `env.*` fallback works, and `ctx.*` lazy fields do not require precomputed event metadata.

Parallelizable:

- Serde tests for `HookAction` can be completed before runner behavior is wired.
- Runner skip behavior can be tested with existing fake `DispatchConfig` actions while docs are updated later.

Validation checkpoint:

- `cargo test -p claudine actions::hook_action`
- `cargo test -p claudine dispatch::runner`
- Manual config smoke test with one `speak` action using `when: "tool_name == 'Bash'"` and one using `when: "tool_name == 'Read'"`.

## Phase 4: Event Binding Matcher Expressions

Outcome: event bindings can filter across multiple `EventMeta` fields with Darkmatter conditions while current regex matchers continue to work.

Files:

- `claudine/lib/src/dispatch/matcher.rs`
- `claudine/lib/src/dispatch/loader.rs`
- `claudine/lib/src/dispatch/mod.rs`

Steps:

- [ ] Replace the runtime matcher representation from `Option<Regex>` to an internal enum, for example `RuntimeMatcher::Regex(Regex)` and `RuntimeMatcher::Expression(String)` or a parsed expression form if Darkmatter exposes a stable AST type.
- [ ] At config load time, attempt to parse the matcher string as a Darkmatter condition first. If parsing succeeds, store expression mode. If parsing fails, compile the existing regex mode.
- [ ] Preserve existing invalid-regex behavior for strings that are neither valid conditions nor valid regexes: warn and skip the binding.
- [ ] Update `matches_with_regex` naming and call sites to a general `matches(...)` API while keeping old regex-only helper behavior in tests if it is still useful.
- [ ] For expression matchers, evaluate against the full `EventMeta` JSON value and return false on invalid evaluation with a warning.
- [ ] Preserve regex matching fields exactly: tool events match `tool_name`, notification events match `notification_type`, and other events with a regex matcher return true.
- [ ] Add tests for `tool_name == 'Bash' && git.branch == 'main'`, `provider == 'claude' && !git.is_dirty`, regex fallback with `Bash|Edit`, invalid matcher skip, missing field false, and non-tool regex true.
- [ ] Add a loader integration test proving configured `matcher` strings compile to the expected runtime matcher mode.

Parallelizable:

- Matcher enum/load-time compilation can be implemented independently from the dispatch call-site rename.
- Regex compatibility tests can be written while expression evaluation tests wait on the shared bridge.

Validation checkpoint:

- `cargo test -p claudine dispatch::matcher`
- `cargo test -p claudine dispatch::loader`
- `cargo test -p claudine dispatch::`

## Phase 5: Harness Validation Message Templates

Outcome: harness validation messages use the same expression renderer as dispatch templates, with the current simple `{{key}}` behavior preserved.

Files:

- `claudine/lib/src/harness/validate.rs`

Steps:

- [ ] Introduce a small validation-message lookup wrapper over the `HashMap<&str, String>` produced by `build_vars`.
- [ ] Replace literal `String::replace` in `render_template()` with Darkmatter interpolation parsing and evaluation.
- [ ] Preserve unknown or malformed tokens unchanged so validation output remains best-effort and cannot mask the underlying validation result.
- [ ] Add tests for existing replacements, missing keys, fallback expressions, ternary expressions, and helper functions.
- [ ] Decide whether validation templates should receive only the existing `build_vars` map or a richer context including frontmatter/effective state. Keep v1 to the existing map unless a caller already passes richer data at this seam.
- [ ] Confirm pre-check and post-check status rendering still uses `StatusState` for pass/fail and does not embed extra glyphs in message text.

Parallelizable:

- Existing replacement regression tests can be written before the renderer changes.
- The richer-context decision can be documented while the narrow map-backed implementation is underway.

Validation checkpoint:

- `cargo test -p claudine harness::validate`

## Phase 6: Docs, Skill Notes, and Deferred Enhancements

Outcome: public docs explain the new expression syntax, and lower-priority reporting/linking expression ideas are captured as future work instead of being half-implemented.

Files:

- `claudine/cli/README.md`
- `claudine/docs/topics/validations-and-handlers.md`
- `claudine/.claude/skills/claudine/SKILL.md`
- `claudine/features/2026-04-29-leverage-dm-parser/plan.md`

Steps:

- [ ] Update hook/action documentation with `when` examples for `speak`, `notify`/message, `bash`, and `call`.
- [ ] Update template documentation to show Darkmatter-supported expressions: fallback with `||`, ternary, comparisons, and `length(...)`.
- [ ] Update matcher documentation to distinguish expression matchers from regex fallback and state the backward-compatibility rule.
- [ ] Update validation-message documentation with the supported template expression subset.
- [ ] Update the local Claudine skill with the new dispatch template, hook `when`, and matcher behavior.
- [ ] Record reporting query filters as deferred: needs a separate design because in-memory filtering can conflict with current SQL aggregation efficiency and CLI UX.
- [ ] Record resource linking filters as deferred: needs a separate design because current prefix/suffix/negation syntax is simple and shared by skills, commands, and agents.
- [ ] Run formatting and focused test suites, then run the full Claudine package tests if focused suites pass.

Parallelizable:

- README/docs updates can proceed in parallel with skill updates once behavior is settled.
- Deferred reporting/linking notes can be written independently of the final test sweep.

Validation checkpoint:

- `cargo fmt --all --check`
- `cargo test -p claudine dispatch::template`
- `cargo test -p claudine dispatch::matcher`
- `cargo test -p claudine dispatch::runner`
- `cargo test -p claudine harness::validate`
- `cargo test -p claudine`
- Optional CLI smoke tests:
  - `claudine hooks --variables`
  - `claudine actions`
  - `claudine handle before_tool --provider claude < fixture-before-tool.json`
