# Phase 1 — Baseline Orientation and Contracts: Findings

Validation checkpoint artifact for Phase 1 of the Lifecycle Formalization
plan. Captures the touched-module list, API surface inventory, and the
contract decisions later phases must honor.

## Touched Module List (planned, by phase)

The implementation will modify these modules. Phase 1 itself only
records the contracts; no code is changed.

| Phase | Module                                                        | Reason                                                                       |
|-------|---------------------------------------------------------------|------------------------------------------------------------------------------|
| 2     | `claudine/lib/src/composition/lifecycle.rs`                   | Add `Initialize`/`Finalize`/`Loop` signals, `LifecycleConfig::get` arms,    |
|       |                                                               | `LifecycleNotification::info`/`warn`/`stack` fields, short/long-form        |
|       |                                                               | action parsing, cardinality + "Where valid" validation.                      |
| 2     | `claudine/lib/src/composition/types.rs`                       | New typed action model (`LifecycleStackItem`, `LifecycleActionRef`,          |
|       |                                                               | control/communication/shell/side-effect/expression-function actions).        |
| 2     | `claudine/lib/src/composition/error.rs`                       | New `CompositionError` variants + `BlockError` renderings for parse-time     |
|       |                                                               | action errors, `err` misuse, `stdout` rejection.                             |
| 2     | `claudine/lib/src/composition/loop_config.rs`                 | Extend `KNOWN_LOOP_KEYS` and parsing to accept the lifecycle-concern keys    |
|       |                                                               | alongside iteration controls; thread lifecycle concerns into `LoopConfig`.   |
| 3     | `claudine/lib/src/composition/lifecycle.rs`                   | `err`/`timing`/`current` execution context construction, `err` static scan, |
|       |                                                               | extended leak + undefined-variable scans over the new event surfaces.        |
| 3     | `claudine/lib/src/composition/preflight.rs`                   | Collect shell commands from every reachable lifecycle stack.                  |
| 4     | `claudine/lib/src/composition/lifecycle.rs` (new sub-module)  | Stack execution engine; ordered top-level-then-stack walking, `when:`        |
|       |                                                               | evaluation, action dispatch, error propagation by event.                     |
| 5     | `claudine/cli/src/commands/wrap/composition/mod.rs`           | Insert `initialize` slot; route `Skip`/`Proxy`/`Error`/`Retry`/`Resume`/     |
|       |                                                               | `Requeue`; preserve `LifecycleRunGuard` safety net.                          |
| 6     | `claudine/lib/src/composition/loop_engine.rs`                 | Re-enter at `start`, fire `finalize` per iteration, move condition check to  |
|       |                                                               | post-`finalize` gate; concerns-before-condition-before-mutation ordering.    |
| 7     | `claudine/docs/topics/composition.md`                         | Document the new lifecycle model and loop gate semantics.                    |
| 7     | `.opencode/skill/claudine/SKILL.md` (+ architecture.md)       | Update event inventory, communication channels, and loop gate notes.         |

Modules that are explicitly **not** in scope for this feature (per the
spec's "pre-flight" definition):

- `claudine/lib/src/harness/validate/mod.rs` and the harness DSL — the
  legacy `pre_checks`/`post_checks`/`handler` DSL is being retired by a
  companion feature (`2026-06-21-remove-validations`). This feature must
  not change its behavior; future phases only consume its
  `collect_auditable_commands` shell-audit output the same way the
  composition pipeline already does.

## Darkmatter Expression Surface (Phase 2 + 3 will reuse)

Import path: `darkmatter::markdown::compose::expression::*`.

- **AST**: `Expr` (in `ast.rs`) — `Variable`, `StringLiteral`,
  `NumberLiteral`, `BoolLiteral`, `UnaryNot`, `UnaryMinus`, `Paren`,
  `Binary { op, left, right }`, `Index { base, index }`,
  `MemberAccess { base, name }`, `Fallback { primary, fallback }`,
  `Ternary { condition, then_branch, else_branch }`,
  `Comparison { left, op, right }`, `FunctionCall { name, args }`.
  Recursive walkers (already in `lifecycle.rs` and `loop_config.rs`)
  pattern-match on these variants.
- **Parsers** (in `parser.rs`):
  - `parse(input) -> Result<Expr, ParseError>` — interpolation mode
    (`||` = fallback).
  - `parse_condition(input) -> Result<Expr, ParseError>` — condition
    mode (`||` = logical OR); used by `loop_config` for `while`/`until`
    and required for `when:` clauses.
- **Span finder**: `ExpressionFinder::find_all_plain(input) ->
  Vec<ExpressionLocation>` — finds `{{ ... }}` spans in a plain string
  with no markdown code-region exclusion. Already used by the leak and
  undefined-variable guards in `lifecycle.rs`.
- **Evaluator**: `evaluate<L: EvaluationLookup>(expr, lookup) ->
  Result<Value, String>` plus `is_truthy(value) -> bool`. Phase 4 will
  use these against a `LifecycleLookup` that resolves `err`/`timing`/
  `current` plus the existing `ctx`/`env`/`doc` roots.
- **Truthiness**: `null`, `false`, `0`, `0.0`, `""`, `[]`, `{}` are
  falsy; everything else is truthy. `when:` clauses inherit this.
- **Comparison operators**: `==`, `!=`, `>`, `>=`, `<`, `<=` (via
  `ComparisonOp`).

### Short-form Action Grammar Decision

The short form `verb(args)` is **not** parsed by Darkmatter today. The
existing `loop_config::split_action_args` already implements the
balanced-delimiter, quote-aware splitter this feature needs. Phase 2
will reuse that approach:

1. Split on the first `(`, verify the matching `)` is the final char.
2. Split the args with the existing balanced/quote-aware splitter.
3. Parse each arg with `darkmatter::markdown::compose::expression::parse`
   (the spec mandates expression semantics — `say(ctx.repo)` evaluates
   `ctx.repo`, `say('hello')` is a string literal, `retry(3)` is the
   integer 3, etc.).
4. Reject a single arg that trims to multiple bare words — that is the
   "unquoted multi-word literal" the spec calls out as a parse-time
   error. Detection: an arg that fails to parse AND has whitespace
   outside any quote/paren region.

## Darkmatter Side-Effect Surface (Phase 4 will invoke)

Import path: `darkmatter::effects::*` (re-exported from
`darkmatter::lib::src::effects`).

- **Engine**: `EffectEngine::builder().mutation_root(path).
  allowed_hosts(hosts).auto_rehash(bool).build()` — construction is
  cheap; the engine is `Clone + Debug`.
- **Catalog** (for documentation only): `effect_descriptors() ->
  &'static [EffectDescriptor]`. Already rendered by `claudine context
  --side-effects`; the catalog is the authoritative capability surface.
- **Typed verbs** (no string dispatcher — Claudine must build a small
  name → typed-call adapter):

  | Verb                                                            | Returns                          |
  |-----------------------------------------------------------------|----------------------------------|
  | `set_frontmatter(file, prop, value)`                            | prior value (or `null`)          |
  | `merge_frontmatter(file, obj)`                                  | merged object                    |
  | `delete_frontmatter(file, prop)`                                | removed value (or `null`)        |
  | `increment_frontmatter(file, prop)`                             | new number                       |
  | `decrement_frontmatter(file, prop)`                             | new number                       |
  | `append_frontmatter(file, prop, value)`                         | new array                        |
  | `prepend_frontmatter(file, prop, value)`                        | new array                        |
  | `ensure_file(file)` / `ensure_file_with_content(file, content)` | absolute path                    |
  | `ensure_dir(dir)`                                               | absolute path                    |
  | `append_line(file, text)`                                       | absolute path                    |
  | `append_jsonl(file, obj)`                                       | absolute path                    |
  | `http_post(url, body)`                                          | `{ status, body }`               |

- **Error type**: `EffectError` (`Io`, `PropertyType`, `HostNotAllowed`,
  `InvalidUrl`, `Network`, `Markdown`). Phase 4 will wrap these into
  `CompositionError::LifecycleActionFailed` (variant to be added in
  Phase 2) for uniform CLI rendering.

## Shell-Audit Input Surfaces (Phase 3 will extend)

`claudine/lib/src/composition/preflight.rs::resolve_shell_approvals`
collects commands from exactly two source kinds today:

1. **Template `::shell` directives** — via
   `markdown.compose_preflight(opts)` → `preflight.entries` (each entry
   carries `normalized`, `source_file`, and `origin.line_number()`).
   Condition-blind: every command under any document state is gathered.
2. **Harness plan** — via `collect_auditable_commands(plan, None)`.
   Sources covered (see `claudine/lib/src/harness/audit.rs`):
   - harness `pre_checks` and `post_checks` (`ShellCommand` rules)
   - declarative `deviate` handlers
   - programmatic `handle` command (when present)
   - any `::shell` directives in the harness source page

**New surface (Phase 3):** every reachable lifecycle stack shell
command. Each `action: shell` in a `stack:` (and any shell command
reachable through `loop.stack` and the new events' stacks) must be
collected with its source provenance (`source_path`, property path like
`start.stack[2].action.command`, and the source line if known) and
fed into `resolve_shell_approvals` alongside the existing sources.
Denials produce a `blocked` outcome exactly like harness audit denials
do today — never a provider invocation.

## Spec "Pre-flight" Definition — Honor Bound

The spec is explicit: "pre-flight checks" in this feature means **only**
`$schema` validation and the lifecycle-stack shell audit. The legacy
harness `pre_checks`/`post_checks`/handler DSL is **not** part of
pre-flight for this feature — it is being retired by the companion
`2026-06-21-remove-validations` feature. Phases 3, 4, and 5 must not
retire the harness DSL ahead of that companion feature; they only
consume its shell-audit output through the existing API.

## Backward-Compatibility Contract

Phase 7 ("Backward Compatibility, Documentation, and UX Polish") must
keep every assertion in
[`baseline.md`](baseline.md) passing for top-level-only prompts:

- All four existing `LifecycleSignal` variants keep their
  `property_name()` and `status_state()`.
- `LifecycleNotification` keeps its six existing fields with the same
  audio-phase ordering (`say+effect` → effect first; `say_first+effect`
  → speech first).
- `say` / `say_first` remain mutually exclusive.
- `LifecycleRunGuard` keeps its Drop safety net and transition rules.
- Lifecycle chatter continues to stay off stdout; stderr, messenger,
  TTS, sound effects, and desktop notifications remain the only
  emission channels.

## Phase-1 Verification

- No code changed in Phase 1 — `cargo check` was not run because no
  source was touched.
- `baseline.md` records the existing test coverage that pins the
  baseline; existing tests already exercise every behavior the later
  phases must preserve.
- No functional changes to validate; Phase 1 is orientation and
  contract capture.
