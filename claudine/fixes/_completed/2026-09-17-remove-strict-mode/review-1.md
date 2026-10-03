---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-17-remove-strict-mode/spec.md`
fix: 2026-09-17-remove-strict-mode/review-1.md
spec: 2026-09-17-remove-strict-mode/spec.md
created: 2026-10-01T18:24:37-07:00
reviewed_by: claude/opus
implemented: true
next: 2026-09-17-remove-strict-mode/review-2.md
implemented_by: claude/opus
log: claudine/fixes/2026-09-17-remove-strict-mode/log.md
ready: false
human_review: false
recurrence: false
findings:
    - "High: a triple-brace escape in a lifecycle value is evaluated instead of rendered as literal text"
    - "Medium: the lifecycle topic doc teaches `{{{ … }}}` as the way to inject a value, contradicting its own escape rule"
    - "Low: a sequence item's bare `doc` and its dotted paths can disagree about nested values"
    - "Low: the task-value wrapper has no test proving its typed Darkmatter cause survives"
---

# Review 1: Remove Strict Mode and Centralize Expression Binding

## Summary

This fix replaces Darkmatter's "strict mode" with one rule: a bare name in an
expression is a document property, and an absent property evaluates to `null`.
A new binding model lets a host (Claudine) declare globals such as `err`,
`timing`, and `group`, and mark each one available or unavailable per
lifecycle event. Darkmatter enforces those declarations at preparation and at
runtime, so Claudine no longer walks expression trees itself.

Almost all of that is implemented well and verified at the right level. The fix
is **not production ready** because of one in-scope contract violation:

> In a Claudine lifecycle value (an `initialize`/`start`/… action, a `set:`
> value, a lifecycle message), the triple-brace escape `{{{ title }}}` is
> **evaluated** and prints the title, instead of printing the literal text
> `{{ title }}`.

The specification's Language Contract, acceptance criterion 8, and acceptance
criterion 17 require lifecycle text to follow ordinary Darkmatter escape rules,
where all three escape forms (`{{{ x }}}`, `\{{ x }}`, `\{\{ x }}`) produce
literal `{{ x }}`. The two backslash forms already do; the triple-brace form
does not. Every test passes because the tests that "prove" lifecycle escape
parity feed lifecycle JSON straight into the executor, skipping the document
pipeline step where the escape is lost.

### What was verified, and how

| Check | Method | Result |
| --- | --- | --- |
| Darkmatter L1 (`just test` in `darkmatter/`) | ran it | 8,774 passed, 12 skipped |
| Claudine L1 (`just test` in `claudine/`) | ran it | 8,068 passed, 9 skipped |
| `just lint` in both areas | ran it | exit 0 |
| Absent property is `null`: ternary takes the falsy branch, mixed text renders empty, `\|\|` picks its default, `is_null` is true, a `set:` copies `null` | real `claudine compose` and `md compose` in a scratch repository, hermetic environment (temporary `HOME`, `PLAYA_DRY_RUN=1`, private audio spool) | correct |
| Bare `repo` no longer falls back to `ctx.repo` (spec R3) | same | renders empty, with the new advisory "is an undeclared document property (unknown type; `null` unless supplied at runtime)" |
| Bare `err` in `initialize`, even in a branch that never runs, fails at preparation before any side effect; `doc.err` reads the document | same | correct |
| An unknown function in a `set:` batch halts the whole batch with no partial write | same | correct |
| Removed symbols (`SubtreeStrictness`, `.strict()`, `with_strictness`, `validate_strict_roots`, `is_known_variable_root`, `first_undefined_stack_variable`, `LifecycleUndefinedVariable`, `LayeredLookup`, `LATE_BINDING_ROOTS`, `reject_surviving_spans`) | repository-wide search, excluding specs and history | gone (acceptance criteria 1 and 5) |
| Strict-mode or "add a fallback to make it legal" guidance in current docs, skills, and DMLS messages | repository-wide search | none (the remaining "strict mode" hits are schema, hash, and file-link strictness, which are different features) |
| `prompts/` audit for leftover `\|\| false` legality guards (spec R8) | search plus the implementation log's audit table | the router guards are gone; the remaining `\|\| ''` uses in `pr.md`, `_pr/dirty.md`, `_pr/push.md` are real defaults (they pass `''` rather than `null` into a string-typed target) |
| Prompts that silently relied on the removed bare-name `ctx` fallback | searched every bare context-variable name in prompts, docs, and skills | every remaining use is declared in its document's frontmatter, is a hook-event template (a different template system), or is a literal escape example |
| New test files compiled by a declared target and selected by a running tier | checked `tests/l1/main.rs` declarations (`autotests = false` packages) and name markers | all declared, all L1 |

### Strengths worth keeping

- [`BindingView`, `EvaluationSession`, `RuntimeBinding`](../../../darkmatter/lib/src/markdown/compose/expression/binding.rs)
  (package `darkmatter`) form a clean seam. An unavailable global is an
  explicit entry, never an omission, and association rejects an omitted,
  contradicting, duplicate, or reserved-name registration before anything is
  evaluated.
- [`lifecycle/bindings.rs`](../../lib/src/composition/lifecycle/bindings.rs)
  (package `claudine`) is the one place the lifecycle availability matrix lives.
  Its tests walk every global against every one of the eleven scopes, through
  both Darkmatter's passive validation (in a branch that never runs) and the
  runtime session, against a document whose properties share the globals'
  names, so any fall-through would show.
- Every `EvaluationLookup` wrapper forwards `resolve`, so no wrapper turns an
  unavailable-global error back into a silent `null`. The `lookup_parity` test
  keeps that inventory current.
- The reported-case regression
  (`claudine/cli/tests/l1/shipped_prompts.rs::shipped_implement_router_reads_absent_optional_inputs_unguarded`)
  reads the real shipped router with `include_str!`, runs hermetically, and
  asserts the routing error, the rendered lines, no provider launch, and no
  audio.

### Test level

Every requirement in this fix is about expression semantics, error routing,
and editor diagnostics. None concerns key presses or terminal rendering, so
Level 1 tests (in-process, plus spawning the real `claudine` binary) are the
appropriate level. No finding below is about the wrong test level.

The Input Robustness Matrix does not apply: no file-format or configuration
reader changed.

## Findings

### 1. High: a triple-brace escape in a lifecycle value is evaluated instead of rendered as literal text

**Defect class:** the escape forms the spec calls equivalent do not agree on
the lifecycle surfaces once text passes through the real document pipeline.

**What the spec requires.** The Language Contract section "Successful literal
output retains ordinary Darkmatter semantics" says lifecycle frontmatter uses
ordinary escape behavior: triple braces, `\{{ x }}`, and `\{\{ x }}` all
produce the literal text `{{ x }}`. Acceptance criteria 8 and 17 repeat it.
The Claudine Level 1 verification asks for "the same fixtures for all three
literal escape forms … in Darkmatter and lifecycle consumers to prove parity".

**What happens.** In a scratch repository with `title: probe`, `claudine compose
probe.md -y --claude`, with an `initialize` stack that stops on an authored
`error` so no provider starts:

| Site | Authored text | Observed | Expected |
| --- | --- | --- | --- |
| lifecycle `stderr` action, mixed text | `A=[{{{ title }}}]` | `A=[probe]` | `A=[{{ title }}]` |
| lifecycle `stderr` action, whole value | `{{{ title }}}` | `probe` | `{{ title }}` |
| lifecycle `set:` value, then read back | `stored: "{{{ title }}}"` | `{{ stored }}` gives `probe` | `{{ title }}` |
| lifecycle `stderr`, single backslash | `\{{ title }}` | `{{ title }}` | same (clean) |
| lifecycle `stderr`, double backslash | `\{\{ title }}` | `{{ title }}` | same (clean) |
| ordinary key holding a literal, read in lifecycle | `note: "{{{ title }}}"`, then `{{ note }}` | `{{ title }}` | same (clean) |
| `md compose` body | `{{{ title }}}` | `{{ title }}` | same (clean) |

Top-level lifecycle fields such as `start: { message: … }` use the same path
(below), so they behave like the first three rows.

**Why.** Claudine asks Darkmatter's compose to leave the seven lifecycle keys
alone so it can evaluate them at event time
([`prepare.rs:319`](../../lib/src/composition/prepare.rs),
`with_exclude_keys(LIFECYCLE_EVENT_KEYS…)`). Darkmatter's first frontmatter
pass skips excluded keys when decoding literal tokens, but **not** when
converting literals.
[`frontmatter_interpolation.rs:814-833`](../../../darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs)
(package `darkmatter`) runs `convert_authored_literals` over the excluded
lifecycle keys too, which rewrites `{{{ title }}}` to `{{ title }}`, and then
on purpose does not mark the result as data (`if !excluded {
converted.extend(scanned); }`; the comment says "Excluded (event-time) keys
keep their long-standing conversion"). Claudine then parses the lifecycle
configuration from that rewritten text, so the executor sees an ordinary
`{{ title }}` and evaluates it.

The behavior predates this fix (it is present before commit `44c8f090b`, and
in older form before `9f5655a08`). It is still in scope: the specification
made ordinary escape semantics for lifecycle text a requirement of this fix,
and the implementation log cites acceptance criteria 8 and 17 as met.

**Why the tests did not catch it.** The tests cited as evidence,
`interpolation_conformance.rs::every_engine_agrees_on_missing_properties_and_escapes`
and `executor/tests/binding_contract.rs::event_time_values_follow_the_shared_missing_property_table`,
hand lifecycle JSON straight to the executor (`run(json!(…))`). That skips the
compose pass where the escape is lost, so they pass. The shared table
(`missing_property_cases`) also has rows only for the triple-brace form, not
for `\{{ x }}` or `\{\{ x }}`, so it does not meet the "same fixtures for all
three forms" verification item either. Darkmatter's own
`absent_property_contract::every_escape_form_is_inert` covers all three forms,
but only for an ordinary document body.

**Blast radius of the fix.** No shipped prompt under `prompts/` uses
`{{{ … }}}` inside a lifecycle key, so changing the behavior breaks no prompt
in the repository. The docs that teach the old behavior are Finding 2.

**Required change.**

1. Stop converting `{{{ … }}}` under excluded keys in Darkmatter's first pass,
   or convert it to a stored literal (data) that the event-time evaluation
   delivers verbatim. Either way, the lifecycle executor must receive text
   whose escape still means "literal". Pick whichever keeps Darkmatter's
   "excluded keys keep their raw text for the caller" contract (the decode
   loop's comment at lines 795-797) true for both loops.
2. Add a Claudine test that goes through the **real compose path**, not
   `run(json!(…))`. A CLI L1 test with `CliProcessFixture` like the probe
   above is the simplest: one document with all three escape forms in a
   mixed-text message, a whole-value message, a `set:` value, and a top-level
   `start.message`, stopping on an authored `error`. Assert each prints
   `{{ title }}`.
3. Add the `\{{ … }}` and `\{\{ … }}` rows to `missing_property_cases` so the
   shared table covers all three forms.
4. Add a Darkmatter unit case in `exclude_keys_tests`
   (`frontmatter_interpolation.rs`) proving an excluded key keeps
   `{{{ … }}}` as authored.

### 2. Medium: the lifecycle topic doc teaches `{{{ … }}}` as the way to inject a value, contradicting its own escape rule

**Defect class:** documentation that describes two incompatible meanings for
the same syntax (spec R9: "one vocabulary"; acceptance criterion 12).

[`claudine/docs/topics/flow-control/lifecycle.md`](../../docs/topics/flow-control/lifecycle.md)
says at line 110 that `{{{ … }}}` "is an escape that renders the literal text
`{{ … }}` without evaluating it" (the spec's rule). Other passages in the same
page describe it as the *evaluated* form:

| Line | Text | State |
| --- | --- | --- |
| 78 | "The variables a lifecycle `{{{ … }}}` span can read …" | treats it as evaluated |
| 194 | "Every value in a lifecycle action is literal text. Use `{{{ … }}}` to inject a variable or expression." | treats it as the inject form; this is the sentence Phase 7 meant to correct |
| 260 | "pass it through a whole-value `{{{ … }}}` span" (object arguments) | treats it as evaluated |
| 411 | "a `{{{ … }}}` interpolation that raised" | treats it as evaluated |
| 885 | "checks every `{{{ … }}}` span in communication and action strings" | treats it as evaluated |
| 110, 256, 819, 849, 881 | `{{{ … }}}` as a literal escape | consistent with the spec |

Today's binary matches the wrong passages (Finding 1), which is likely why
they survived. Once Finding 1 is fixed, rewrite lines 78, 194, 260, 411, and
885 to use `{{ … }}` for evaluation and keep `{{{ … }}}` only as the literal
escape. Sweep `.claude/skills/claudine/` for the same wording in the same
change.

### 3. Low: a sequence item's bare `doc` and its dotted paths can disagree about nested values

**Defect class:** a layered document lookup that merges layers differently for
the whole `doc` object than for a dotted path. Status: **plausible**, found by
reading the code, not reproduced through the CLI.

[`SourceExpressionLookup`](../../lib/src/composition/sequence/expr.rs)
(package `claudine`) evaluates sequence source and template expressions, with
a sequence item's fields layered over the invoking document's frontmatter.
For a dotted path, `document()` tries the item, and on a miss *at any depth*
falls back to the frontmatter. For bare `doc`, the new `document_object()`
replaces whole top-level keys. With an item `config: {a: 1}` over frontmatter
`config: {b: 2}`:

- `config.b` and `doc.config.b` read `2` (the item misses `b`, so the lookup
  falls through to the frontmatter);
- `doc['config']['b']` reads `null` (the merged object has the item's
  `config`, which has no `b`).

The spec's equivalence (`plan.path` ≡ `doc.plan.path`, `items[0]` ≡
`doc.items[0]`) expects these to agree. The per-path fall-through existed
before this fix; `document_object()` is new in it. The sibling
`LoopExpressionLookup` (`looping/expression.rs`) is clean: its `doc` reads only
frontmatter and its ambient loop names are not nested.

**Suggested change:** make `document()` stop at the first layer that has the
*top-level* key (matching `document_object()`), and add a unit test with the
example above.

### 4. Low: the task-value wrapper has no test proving its typed Darkmatter cause survives

**Defect class:** acceptance criterion 16 and spec R6 ("retain the original
typed Darkmatter error through the Claudine library boundary") applied to each
of the four `SubtreeCompose` consumers.

| Consumer | Wrapper | Typed source kept | Test that downcasts it |
| --- | --- | --- | --- |
| lifecycle event time | `LifecycleEvaluationError`, `LifecycleProxyWithEvaluationFailed` | yes | `binding_contract::the_typed_darkmatter_cause_survives_every_library_wrapper` |
| lifecycle shell approval | `LifecycleShellResolution` | yes | `prepare/tests.rs` (around line 1232) |
| sequence shell approval | `SequenceShellLateBinding`, `SequenceShellResolution` | yes | `sequence/task/tests.rs` (around line 5412) |
| sequence task values | `SequenceTaskValueResolution` | yes (`source: Box<MarkdownError>`) | **none** |

The code is correct. Add one test that triggers a task-value failure (for
example `params: { x: "{{ no_such_fn() }}" }`) and downcasts the source to
the Darkmatter error, so a later refactor cannot flatten it unnoticed.

## Observations not caused by this fix

These do not affect readiness.

- [`prompts/_reviews/observability.md`](../../../prompts/_reviews/observability.md)
  reads `{{ctx.review_relative}}` where it means the frontmatter key
  `review_relative`, and `{{error.msg}}` where it means `err.msg`. Before this
  fix the bare `error` would have stopped the run; now both render empty,
  which is the new contract working as designed. Fix the prompt.
- The open follow-ups already listed in the spec's `message_to_agent` stand:
  `prompts/_agent-skills.md` calls functions that do not exist, and the loop
  action renderer does not honor `{{{ … }}}`. The spec's Non-Goals keep loop
  action handling out of scope.
