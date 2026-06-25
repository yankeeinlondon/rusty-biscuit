---
agent: "open_code/zai-coding-plan/glm-5.2"
phases: 8
created: 2026-06-26
start_phase: 1
yolo: "true"
packages:
  - claudine
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - claudine/features/2026-06-26-positional-and-key-value/phase-1-findings.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/composition/lifecycle_actions.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/lifecycle.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/composition/lifecycle.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/lifecycle_actions.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/lifecycle_actions.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/lifecycle_executor.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/tests/level2_lifecycle_control.rs
  - claudine/cli/tests/level2_lifecycle_dispatch.rs
  - claudine/cli/tests/level2_lifecycle_loop.rs
  - claudine/cli/tests/level2_wrap_ctrl_c_loop_wedge_tmux.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - claudine
source_files_during_phase_6:
  - claudine/cli/tests/level2_lifecycle_action_forms.rs
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - claudine
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - claudine/docs/topics/lifecycle.md
  - claudine/docs/topics/composition.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .opencode/skill/claudine/SKILL.md
  - .opencode/skill/claudine/timeline.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/timeline.md
packages_during_phase_7:
  - claudine
source_files_during_phase_8: []
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
packages_during_phase_8:
  - claudine
source_code:
  - claudine/lib/src/composition/lifecycle.rs
  - claudine/lib/src/composition/lifecycle_actions.rs
  - claudine/lib/src/composition/lifecycle_executor.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/lib/src/composition/loop_config.rs
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/tests/level2_lifecycle_action_forms.rs
  - claudine/cli/tests/level2_lifecycle_control.rs
  - claudine/cli/tests/level2_lifecycle_dispatch.rs
  - claudine/cli/tests/level2_lifecycle_loop.rs
  - claudine/cli/tests/level2_wrap_ctrl_c_loop_wedge_tmux.rs
documentation:
  - claudine/features/2026-06-26-positional-and-key-value/phase-1-findings.md
  - claudine/docs/topics/lifecycle.md
  - claudine/docs/topics/composition.md
---

# Lifecycle Action Forms: Positional and Key/Value Execution Plan

Success means Claudine parses and dispatches lifecycle stack actions in exactly
two forms — **positional** (single-key `{verb: value}`) and **key/value**
(`{action: verb, ...}`) — applies one universal evaluation rule (literal text
with `{{ … }}` interpolation; only `when`/`until`/`while` are expressions),
rejects the removed `verb(args)` short form with a typed did-you-mean rewrite,
validates verbs against the union of the Darkmatter and Claudine catalogs at
parse time, and proves the new grammar through L1 parser tests plus an L2
end-to-end `compose` run. Docs and skills describe exactly two forms; no
in-tree example references `verb(args)`.

The motivating shape (`success: [{ success: "…", effect: "…" }]`) parses and
runs.

## Phase 1 - Baseline Orientation and Scope Inventory

- [x] Run `cargo metadata --no-deps --format-version 1` and record the claudine
      workspace member names that this feature touches (`claudine`, `claudine-cli`).
- [x] Read every short-form code path in `claudine/lib/src/composition/lifecycle.rs`
      and record line ranges: `parse_scalar_action`, `parse_short_form_action`,
      `split_action_args`/`parse_action_arg`/`has_unquoted_whitespace`,
      `value_to_expr`, `is_single_text_arg_verb`, `unwrap_wrapping_quotes`, the
      scalar-string and string-element branches of the stack `action:` match,
      and the sibling-keys path inside `parse_stack_item`.
- [x] Confirm the **loop surface is out of scope**: `loop_config.rs` uses
      `split_action_args` for `increment`/`decrement`/`set`/`append`/`prepend`/
      `merge` under the loop `action:` value — a different surface from the
      lifecycle stack `action:`. Record that `split_action_args` MUST stay even
      after short-form removal from lifecycle stacks.
- [x] Inventory Darkmatter descriptor catalogs: `EXPRESSION_FUNCTION_DESCRIPTORS`
      (`darkmatter/lib/src/markdown/compose/expression/catalog.rs`) and
      `EFFECT_DESCRIPTORS` (`darkmatter/lib/src/effects/catalog.rs`), noting the
      canonical signature strings (`"set_frontmatter(file, prop, value)"`) that
      decision #6 says to parse via a small helper rather than ad-hoc splits.
- [x] Inventory every existing `CompositionError` lifecycle variant
      (`error.rs:325-454`) and the `FrontmatterExcerpt` renderer
      (`frontmatter_excerpt.rs`, TTY vs `ColorDepth::None` paths) so the new
      variants reuse the same excerpt/highlight pipeline.
- [x] Build the migration target list with `rg`: every `verb(args)` short-form
      example in `claudine/docs/topics/lifecycle.md` (~38 hits), the
      `composition.md` `shell(...)` mention, the claudine skill docs
      (`.opencode/skill/claudine/` + `.claude/skills/claudine/` — `SKILL.md`,
      `cli-reference.md`, `timeline.md`), and confirm `claudine/prompts/`
      contains no short-form today (note: `prompts/review-feature.md` referenced
      by the spec does **not** exist in-tree — record this so the migration
      task does not block on a missing file).
- [x] Capture the current `value_to_expr` behavior in a written baseline
      (strings that parse as Darkmatter expressions become expressions; others
      become `StringLiteral`) so Phase 3 can prove the literal-default change
      is intentional and behavior-changing.
- [x] Record the existing L1 parser tests in `lifecycle.rs` `mod tests`
      (~141 `#[test]`s) that assert short-form acceptance — these flip to
      asserting rejection in Phase 5 and need an explicit list.
- [x] Validation checkpoint: write `phase-1-findings.md` containing the
      in-scope file/line list, the out-of-scope loop surface note, the
      migration target list, the missing-`review-feature.md` note, and the
      `value_to_expr` baseline. No code change in this phase.

Parallelizable after this phase: Phase 2 (catalog helper) and Phase 3
(literal-default converter) can proceed in parallel — they touch disjoint
helpers and only Phase 4 depends on both.

## Phase 2 - Catalog Signature Helper and Known-Verb Validator

- [x] Add a signature-parser helper that turns a canonical descriptor signature
      string (`"set_frontmatter(file, prop, value)"`, `"ensure_file(file, content?)"`,
      `"and(...)"`) into a typed shape: verb name, ordered positional parameter
      names, optional-tail flags, and a variadic flag. Place it next to
      `side_effect_signature` in `lifecycle_actions.rs` (or in Darkmatter next
      to the catalogs if the helper is reusable across crates — prefer the
      existing public catalog APIs over duplicating string lists per
      decision #6).
- [x] Replace the hand-maintained `side_effect_signature` table with a
      derivation from `EFFECT_DESCRIPTORS` where possible; keep the existing
      `&'static [&'static str]` return shape so call sites compile, OR migrate
      call sites to the new typed shape (pick one approach and apply it
      consistently).
- [x] Add a `is_known_lifecycle_verb(verb)` predicate that unions:
      `CommunicationChannel::from_verb`, the literal `"shell"`, the
      `parse_lifecycle_control_long`/`parse_lifecycle_control_short` verb set
      (`stop`/`skip`/`error`/`proxy`/`retry`/`resume`/`defer`), the side-effect
      catalog verbs, and the expression-function catalog verbs. This is the
      parse-time validator required by decision #6 and the disambiguation
      table.
- [x] Add typed `CompositionError` variants for the new failure modes the
      disambiguation table requires: unknown-verb single-key positional,
      ambiguous multi-key-no-`action:` object, object-data-through-interpolation
      (positional map value), object-data-through-interpolation (key/value map
      parameter), wrong-arity positional zip, and short-form-removed
      did-you-mean. Reuse the existing `source_path`/`property` shape and wire
      each through `FrontmatterExcerpt` so TTY output highlights the offending
      line and `ColorDepth::None` stays escape-free.
- [x] Add a `rewrite_to_positional(verb, raw)` helper that produces the
      did-you-mean message body — e.g. `success("x")` → `success: "x"`,
      `set_frontmatter('a','b','c')` → `set_frontmatter: ["a","b","c"]` — used
      by the short-form-removed error variant.
- [x] Validation checkpoint: L1 unit tests for the signature parser (positional,
      optional-tail, variadic, malformed), the known-verb predicate (one
      positive and one negative per family), and each new error variant's
      rendering in both TTY and `ColorDepth::None`.

Parallelizable with: Phase 3 (disjoint helpers).

## Phase 3 - Literal-Default Value Converter

- [x] Introduce `action_value_to_expr(&serde_json::Value) -> Result<Expr, ...>`
      (successor to `value_to_expr` for action parameters) implementing the
      single evaluation rule: a string is stored as `Expr::StringLiteral`
      **unless** its trimmed content is exactly one `{{ expr }}` span, in which
      case it resolves to the expression's typed value via the Darkmatter
      whole-value expansion path.
- [x] Map YAML numeric and boolean scalars to `Expr::NumberLiteral` /
      `Expr::BoolLiteral` so `retry: 3` and `set_frontmatter: ["s.md", "ready",
      "{{ true }}"]` carry typed values through to side-effects.
- [x] Reject direct YAML `Object` and `Array`-as-data values (object-valued
      side-effect args must come through a whole-value interpolation span like
      `"{{ payload }}"`) with the object-data-through-interpolation error from
      Phase 2. Keep the **positional array-of-arguments** case dispatching in
      Phase 4 — this converter only sees one argument at a time.
- [x] Preserve the `when`/`until`/`while` exception: those keys still route
      through `parse_condition` (boolean-expression parse), never through
      `action_value_to_expr`. Add an inline note at the call site so a future
      reader does not "uniformize" them.
- [x] Validate whole-value typed resolution against the Darkmatter contract:
      `"{{ true }}"` → bool, `"{{ 3 }}"` → number, `"{{ payload }}"` → whatever
      typed value `payload` holds (object/array passthrough for
      `merge_frontmatter`/`append_jsonl`).
- [x] Validation checkpoint: L1 unit tests for `action_value_to_expr` covering
      plain literal, multi-span interpolation (stays literal with embedded
      spans), whole-value typed bool/number/null, whole-value object passthrough,
      YAML scalar typing, and the object-data rejection for both map and
      array-as-data inputs.

Parallelizable with: Phase 2 (disjoint helpers).

## Phase 4 - Positional Parser

- [x] Add the positional branch to the stack `action:`-array element match in
      `parse_stack_item`: when an element is an `Object` with exactly one key
      and that key is a known verb (and not `action`), classify the value and
      dispatch through a new `parse_positional_action` helper. Keep the existing
      key/value (`action:`-keyed) branch selectable alongside it.
- [x] Add the same positional handling to the single-object `action:` value
      path (currently rejected with "object form is not supported" at
      `lifecycle.rs:1551`) so `action: { success: "…" }` parses.
- [x] Implement value classification per the spec: scalar (string/number/bool)
      → 1 argument; array → N arguments zipped against the verb's signature
      via `side_effect_signature` (Phase 2 shape) or the control/communication
      arity; null or empty array → 0 arguments; bare verb-name string element
      → 0 arguments.
- [x] Implement array→positional zip with optional-tail acceptance: any arity
      from required-minimum through full signature length is valid
      (`ensure_file: ["a.md"]` and `ensure_file: ["a.md", "content"]` both
      parse); a wrong-arity array yields the typed wrong-arity error naming
      the expected count and the verb's parameter names.
- [x] Route control verbs through their existing scalar/null single-optional-arg
      shape (`error: "reason"`, `retry: 3`, `proxy: "x.md"`) and accept all
      three zero-arg spellings (`- stop:`, `- stop: []`, `- stop`) as
      equivalent. A zero-arg spelling on a verb that requires an argument
      (`- proxy`) is the typed wrong-arity error.
- [x] Route expression-function verbs positional-first: variadic descriptors
      (`and`/`or`) accept positional arrays of any length; descriptors with
      concrete named parameters additionally accept key/value form (handled in
      Phase 5).
- [x] Reject a positional value that is a direct YAML map with the
      object-data-through-interpolation error (positional variant).
- [x] Validation checkpoint: L1 parser tests for positional parsing — scalar,
      array, null, empty-array, and bare-verb-name value forms across
      communication (`message`/`effect`/`stderr`), shell, a 3-arg side-effect
      (`set_frontmatter`), a 2-arg side-effect (`append_line`), an
      optional-tail side-effect (`ensure_file`), and the control verbs
      (`stop`/`error`/`retry`/`proxy`); plus wrong-arity (`set_frontmatter:
      ["a"]`, `message: ["a","b"]`) and bare-`proxy` arity errors; plus typed
      args (`{{ true }}` writes bool, `"3"` stays string, `{{ payload }}`
      passes an object to `merge_frontmatter`/`append_jsonl`).

Dependency note: depends on Phase 2 (signature helper + known-verb predicate
+ new error variants) and Phase 3 (literal-default converter). The old
short-form branches still parse during this phase so the test suite stays
green between commits.

## Phase 5 - Short-Form Removal, Key/Value Migration, and Disambiguation

- [x] Replace the scalar-string branch of `parse_scalar_action`
      (`lifecycle.rs:1613`) so a scalar string `action:` value accepts ONLY the
      bare verb-name zero-arg form (no parens). A string containing `(` is the
      short-form-removed error with the Phase 2 did-you-mean rewrite.
- [x] Remove the sibling-keys path (decision #3): `action: <verb>` plus
      sibling parameter keys is no longer accepted — key/value must use the
      explicit `{ action: verb, ... }` **object** form. A multi-key stack item
      without an `action:` key produces the typed ambiguous error with a
      did-you-mean pointing at both the key/value and single-key positional
      rewrites.
- [x] Replace the string-element branch in the `action:` array
      (`lifecycle.rs:1527`) with the bare-verb-name zero-arg form; a string
      element containing `(` is the short-form-removed error (never a zero-arg
      action).
- [x] Migrate `parse_long_form_action_object` to route every parameter value
      through `action_value_to_expr` (Phase 3) instead of the legacy
      expression-fallback `value_to_expr`. Key/value string parameters are now
      literal by default (`message: "ctx.area"` sends the literal string).
- [x] Delete `parse_short_form_action`, `parse_action_arg`,
      `has_unquoted_whitespace`, and the multi-arg branch of
      `is_single_text_arg_verb`/`unwrap_wrapping_quotes` now that no lifecycle
      stack path reaches them. Keep `split_action_args` in `loop_config.rs`
      because the loop `action:` surface (Phase 1 confirmation) still uses it.
      Audit with `rg` that no other call site remains.
- [x] Wire the full disambiguation table from the spec: array element bare
      known-verb string → positional zero-arg; array element string with `(` →
      short-form-removed; object with `action:` key → key/value; single-key
      object with known verb → positional; single-key object with unknown verb
      → unknown-verb error; multi-key object without `action:` → ambiguous
      error; positional value map → object-data error; key/value parameter map
      → object-data error.
- [x] Add parse-time known-verb validation (decision #6): a positional or
      key/value verb that is not in the `is_known_lifecycle_verb` union fails
      immediately with did-you-mean suggestions where the editor-distance
      helper can offer one (`sucess` → `success`).
- [x] Update existing L1 tests in `lifecycle.rs::tests` that previously
      asserted short-form acceptance (`parses_short_form_say_action`,
      `single_text_arg_is_taken_literally`, the `say('hello world')` /
      `say('using codex')` / `say(ctx.repo)` fixtures, etc. — list from
      Phase 1) to assert the short-form-removed error and its did-you-mean
      rewrite instead.
- [x] Validation checkpoint: L1 parser tests prove — (a) short-form rejection
      for `success("x")`, `shell(git push)`, `set_frontmatter('a','b','c')`
      with the correct positional rewrite in the message and the excerpt
      highlighting the offending line in TTY; (b) bare `stop` accepted,
      bare `proxy` rejected as wrong-arity (bare-string is gated on arity, not
      on the short-form path); (c) key/value literal default —
      `{ action: message, message: "ctx.area" }` sends the literal string,
      `{ action: message, message: "{{ ctx.area }}" }` resolves the context
      value; (d) the full disambiguation table from the spec; (e) predicate
      exception — `when`/`until`/`while` still evaluate as boolean expressions
      while a positional value bare token stays literal; (f) known-verb
      validation for typoed positional and key/value verbs; (g) expression-
      function actions — positional `length: "{{ items }}"`, key/value
      `{ action: contains, haystack: "{{ haystack }}", needle: "needle" }`,
      and variadic `and`/`or` reject key/value form with a positional-only
      diagnostic.

Dependency note: depends on Phase 4 (positional branch must exist before the
short-form branches are deleted, so the parser is never left without a
positional path).

## Phase 6 - L2 Integration and Regression Sweep

- [x] Add an L2 `compose` fixture matching the spec's acceptance block: a
      `success` stack with `when:` gate, a positional multi-arg
      `set_frontmatter`, a positional communication action, and a key/value
      `shell` action — verify event-time interpolation (`{{iteration}}`,
      `{{ctx.area}}`, `{{err.msg}}`) resolves and the side-effect writes the
      expected frontmatter. (`level2_action_forms_mixed_success_stack_writes_frontmatter`
      in `cli/tests/level2_lifecycle_action_forms.rs`; uses event-time
      `{{ doc.phase }}`/`{{ doc.title }}` for deterministic interpolation —
      `success` has no `err`, and `iteration`/`ctx.area` are non-deterministic
      in a bare tempdir; `--yolo` auto-approves the key/value shell action.)
- [x] Add an L2 fixture proving typed argument write-through:
      `set_frontmatter: ["s.md", "ready", "{{ true }}"]` writes boolean
      `true` to frontmatter; `merge_frontmatter: ["s.md", "{{ payload }}"]`
      merges the object stored in `payload`.
      (`level2_action_forms_typed_argument_write_through`; `payload` referenced
      via the whole-value `{{ doc.payload }}` span.)
- [x] Add an L2 fixture proving the literal-default breaking change is
      observable end-to-end: a key/value `{ action: message, message:
      "ctx.area" }` sends the literal string `ctx.area`, and the `{{ ctx.area }}`
      equivalent sends the context value. (`level2_action_forms_keyvalue_literal_default`;
      uses the `stderr` channel with `doc.title` — `stderr` writes plain prose
      to the pane deterministically where `message` routes through statusful
      logging suppressed in the piped, non-verbose test environment; the
      literal-default rule is channel-agnostic.)
- [x] Regression sweep: `rg -n '\b(say|message|success|effect|shell|set_frontmatter|append_line|stderr|warn|info|error|stop|skip|retry|proxy|resume|defer|requeue|notify)\('`
      across `claudine/docs/`, `claudine/prompts/`, `.opencode/skill/claudine/`,
      `.claude/skills/claudine/`, and the lifecycle test fixtures. **Code
      surface is clean:** the only YAML short-form hit in the lifecycle test
      fixtures is `action: "increment(phase)"` in `level2_lifecycle_loop.rs`,
      which is the **loop `loop.action:` surface** (`loop_config.rs`, out of
      scope per Phase 1); the lifecycle *stacks* use the new positional/key
      value forms. `claudine/prompts/` has no short-form. **Deferred to
      Phase 7:** `claudine/docs/topics/lifecycle.md` (24 `verb(args)` action
      examples) and the claudine skill docs still carry short-form examples —
      these are the explicit doc/skill-migration deliverables of Phase 7 (per
      this phase's own parallelization note), not a Phase 6 regression.
- [x] Regression sweep: `rg -n 'parse_short_form_action|parse_action_arg|has_unquoted_whitespace'`
      returns no lifecycle-stack call sites — only spec/plan/findings prose and
      the historical late-binding plan reference them; all three symbols were
      deleted in Phase 5 (no `loop_config.rs` retention was needed for these —
      only `split_action_args` was kept for the loop surface).
- [x] Regression sweep: confirm cardinality, ordering, and per-event placement
      checks (`is_valid_for`) still fire identically for positional and
      key/value control actions; add an L1 test if coverage is missing. (Added
      `control_checks_fire_identically_for_key_value_form` in
      `lib/src/composition/lifecycle.rs::tests` — the positional-form tests
      already pin placement/cardinality/ordering; the new test pins the same
      diagnostics for the `{action: verb}` key/value form.)
- [x] Validation checkpoint: `just test` and `just test-l2` pass in the
      `claudine` package area; the L2 fixtures above run green; the two `rg`
      sweeps return only the documented exceptions. (`just test`: 1687 cli +
      lib + contract pass; `just test-l2`: 96/96 pass; `just lint`: clean.
      **Regression found and fixed by the L2 sweep:** the pre-existing
      `level2_lifecycle_control::level2_lifecycle_proxy_target_harness_plan_failure_routes_blocked_finalize_with_err`
      failed because `materialize_harness_prompt` (`harness_orch/prompt.rs`)
      composed with bare `ComposeOptions::new()` — it did **not** defer the
      seven `LIFECYCLE_EVENT_KEYS`, so on a proxy/retry re-materialization the
      target's lifecycle `{{ err.* }}` spans resolved at materialize-time
      (before `err` exists) and baked to empty, while the prep-time seed
      (`prepare.rs`) correctly deferred them. Fixed by mirroring the prep-time
      `.with_exclude_keys(LIFECYCLE_EVENT_KEYS)` on the Passthrough and Compose
      branches; re-exported `LIFECYCLE_EVENT_KEYS` from `composition::mod`.
      Validated against the full L2 lifecycle suite — retries, proxies, and
      downgrades — plus the 720 L1 composition tests.)

Parallelizable: the L2 fixtures can be authored alongside Phase 7's doc
rewrites once Phase 5 is stable, but they MUST land before Phase 8.

## Phase 7 - Documentation, Skill, and Prompt Migration

- [x] Rewrite the **Action Forms** section of `claudine/docs/topics/lifecycle.md`
      to describe exactly two forms (positional and key/value) and the single
      evaluation rule (literal text with `{{ … }}`; only `when`/`until`/`while`
      are expressions). Delete every short-form example in that file (~38 hits
      from the Phase 1 inventory) and replace each with its positional or
      key/value equivalent.
- [x] Update the **Flow Control Actions** table and surrounding prose in
      `lifecycle.md` so the verb signatures read as positional examples
      (`error: "reason"`, `proxy: "@other.md"`, `retry: 3`) rather than
      `verb(args)`.
- [x] Add a lifecycle topic subsection explaining object-valued side-effect
      args: place the object in frontmatter/context and pass it via a
      whole-value `{{ … }}` span. Explicitly state that direct nested YAML
      object literals are NOT accepted in action parameters.
- [x] Add a migration callout in `lifecycle.md` covering both breaking-change
      families: (a) `verb(args)` short form is rejected with a did-you-mean
      rewrite, and (b) key/value string parameters are now literal by default —
      `target: next_prompt` means the literal string; use `target: "{{ next_prompt }}"`
      for expression evaluation.
- [x] Update the single `shell(...)` reference in
      `claudine/docs/topics/composition.md` (line ~105) to the long-form
      `command:` shape (or remove the short-form parenthetical).
- [x] Update the claudine skill docs in BOTH `.opencode/skill/claudine/` and
      `.claude/skills/claudine/`: `SKILL.md` (the late-binding paragraph that
      quotes `message(❌️ {{err.msg}})` and `shell(...)`), `cli-reference.md`
      (the `message()` table row), and `timeline.md` (add a
      `2026-06-26 — positional-and-key-value` entry documenting the two-form
      grammar, the single evaluation rule, the short-form removal, and the
      key/value literal-default breaking change).
      (`.opencode/skill/claudine` is a symlink to `.claude/skills/claudine`, so
      one edit covers both. SKILL.md late-binding paragraph migrated and a new
      "Lifecycle action grammar (two forms)" bullet added; the `stdout(...)`
      reference updated. timeline.md gained the 2026-06-26 entry. **cli-reference.md
      left unchanged:** its `message()`/`info()`/`warn()`/`error()` table rows are
      `log.rs` Rust output functions, not lifecycle `verb(args)` short forms — the
      Phase 1 inventory over-flagged them. Skill `hash:` frontmatter re-stamped
      with `md hash --save`.)
- [x] Port any in-tree prompt file that uses short form to positional form.
      Phase 1 inventory recorded that `claudine/prompts/review-feature.md`
      (named in the spec) does **not** exist in-tree; if a sweep finds no other
      short-form prompt, record that in `phase-1-findings.md` and skip — do
      NOT create the missing file as a side effect of this feature.
      (Sweep confirms `claudine/prompts/` holds only `new-provider.md` with no
      short form; already recorded in `phase-1-findings.md` ("No prompt migration
      is required"). Missing file not created.)
- [x] Validation checkpoint: `rg` over docs/skills/prompts returns no
      short-form examples outside the deliberate "this was removed" callout;
      every code fence in the rewritten Action Forms section parses against
      the new grammar (spot-check with `claudine compose --dry-run` against a
      scratch fixture if cheap to do).
      (`rg` over docs/skills/prompts returns only the deliberate removal callouts
      — lifecycle.md migration subsection and the timeline.md 2026-06-26 entry's
      `success("x") → success: "x"` rewrites; the dated `late-binding` /
      `literal-short-form-args` changelog entries retain their historical
      short-form prose by design, a changelog preserving what was true at each
      date. Rewritten code fences match the exact grammar exercised by the
      passing L2 fixtures in `level2_lifecycle_action_forms.rs` — positional
      `- {verb: value}` lists, `set_frontmatter: [...]`, single positional maps,
      and key/value `action: shell` + `command:`. `just lint` clean; `just test`
      1687/1687 pass.)

Parallelizable: doc rewriting can start once Phase 5 behavior is stable, but
final wording MUST be reconciled with the actual diagnostics after Phase 5
lands.

## Phase 8 - Final Verification and Release Readiness

- [x] Run `just test` in the `claudine` package area and address failures
      without broad formatting churn (no `cargo fmt` write-mode — match
      surrounding style by hand per repo convention). (1687/1687 pass.)
- [x] Run `just test-l2` in the `claudine` package area. (96/96 pass.)
- [x] Run `just lint` in the `claudine` package area and address warnings
      relevant to the changed code. (clean across lib/contract/cli.)
- [x] Run `cargo fmt --check` (read-only) for diagnosis only; do NOT
      reformulate if it complains about pre-existing style. (Repo-wide local
      rustfmt drift vs `main` — diffs span files this feature never touched
      (`interrupt.rs`, `loop_run.rs`, `loop_engine.rs`); left untouched per
      the formatting-authority convention.)
- [x] Manually invoke `claudine compose --dry-run` against (a) the spec's
      motivating `success` stack and (b) a legacy top-level-only lifecycle
      prompt — confirm both parse, the first dispatches the positional and
      key/value actions, and the second is unchanged. (Both exit 0; the
      motivating stack parses positional `success`/`effect`, positional
      `set_frontmatter: [...]`, positional `stderr`, and key/value
      `action: shell` + `command:`; the legacy top-level `failure` block
      resolves unchanged with its `{{err.msg}}` span correctly deferred.)
- [x] Confirm short-form rejection renders correctly in both TTY (excerpt
      highlights the offending line) and non-TTY (`ColorDepth::None`, escape-
      free) by piping a short-form fixture through compose with and without
      color. (`success("review passed")` in an `action:` array → typed
      `short-form action removed` error with did-you-mean rewrite
      `success: "review passed"`; `NO_COLOR` output is escape-free (0 ESC
      bytes); `FORCE_COLOR=1` renders the line-numbered YAML excerpt
      highlighting the offending line 6.)
- [x] Confirm lifecycle chatter stays on stderr (no lifecycle side effect
      writes to stdout) by piping the L2 fixture's compose output. (Dry-run
      stdout carries only the composed prompt body; the resolved-frontmatter
      table, document info, and deferred-event notice all route to stderr;
      runtime side-effect channel routing is already pinned green by the
      Phase 6 L2 fixtures.)
- [x] Review `git diff` for accidental unrelated refactors, formatting-only
      churn, or comment drift (per the repo's comment-quality rules: behavior-
      changing edits MUST include a pass over their `///`/`//!` and inline
      `//` comments; fix or delete drifted ones in the same change). (Phase 8
      introduced no source churn — only `plan.md` checkbox edits and out-of-tree
      scratch fixtures. The feature's source diffs were reviewed within their
      implementing phases; `rg` confirms zero stale references to the removed
      `parse_short_form_action`/`parse_action_arg`/`has_unquoted_whitespace`
      symbols in any source comment or code path; no formatting-only churn was
      added — the only `cargo fmt --check` diffs are pre-existing repo-wide
      rustfmt drift vs `main`.)
- [x] Validation checkpoint: every acceptance criterion in the spec is mapped
      to a passing L1/L2/manual check or explicitly documented as blocked with
      a concrete reason. The motivating shape from the spec's introduction
      parses and runs. (1687 L1 + 96 L2 green; lint clean; the motivating
      `success` stack parses and dry-runs cleanly; short-form removal renders
      its did-you-mean rewrite in both TTY and escape-free non-TTY; key/value
      literal-default and typed-argument write-through proven by the Phase 6 L2
      fixtures. No blocked criteria.)
