---
kind: rulings
created: 2026-09-18
plan: ./plan.md
spec: ./spec.md
---

# Dasherized Identifiers — Phase 1 Rulings and Spike Findings

Rulings R-1 through R-10 settle the ambiguities that [plan.md](./plan.md)
Phase 1 lists. Each gives the verdict, the reason, and the phase it unblocks.
The spike findings (S-1 through S-4) follow. All code citations were checked
against the worktree on 2026-09-18, including its uncommitted changes.

## Corrections to the plan's ground truth

- **`FunctionBinding` exists, and so does an alias registry.** The plan says
  neither exists, which is wrong.
  - `FunctionBinding { canonical, aliases, evaluation, handler }` is at
    `lib/src/markdown/compose/expression/functions/mod.rs:73` (introduced in
    `f27934eda`).
  - Every function group registers through `BINDING_GROUPS` (`:178`).
    Dispatch resolves a name as canonical or alias (`:2719`, `:2751`).
  - The absence predicates are registered with aliases:
    `is_null`/`isnull` and `is_empty`/`isempty` (`functions/predicates.rs:7`,
    `:9`).
  - This changes R-1.
- **The `root_identifier` consumer citations are stale.** The diagnostic
  severity sites are `dmls/src/providers/dsl.rs:740` and
  `dmls/src/diagnostics/frontmatter.rs:653-654`, not `:741`/`:655`. The spec's
  `dsl.rs:696` citation is older still.
- **The corpus is larger than the plan counted.**
  - Root `prompts/` holds 72 files recursively: 69 `.md`, 1 `.yaml`, and 2
    other files. The plan's 45 was a top-level entry count.
  - `.claude/commands/` holds 6 `.md` files. `darkmatter/prompts/` and
    `claudine/prompts/` hold 1 `.md` file each.
- **The corpus contains Claudine condition strings.** Lifecycle `stack[].when`
  and loop `while`/`until` values are Darkmatter conditions. They are
  declared in `darkmatter/docs/schemas/claudine-types.yaml` and are present in
  `prompts/_implement/*.md` and `prompts/merge-conflicts.md`. The spec's
  surface list does not name them, but they are executable Darkmatter
  expressions, so the audit covers them (see R-7).

## Rulings

### R-1 — Absence-predicate resolution mechanism *(Phase 5, Phase 6)*

**Verdict: resolve through the existing `FunctionBinding` registry. Add a
typed marker to the binding, and expose one crate-level predicate that
resolves a callee name as canonical or alias.**

- Add a marker to `FunctionBinding`, for example `absence_predicate: bool` or
  a small `FunctionRole` enum. Set it on exactly the `is_null` and `is_empty`
  bindings.
- Add `pub(crate) fn is_absence_predicate(name: &str) -> bool` beside
  `dispatch` in `functions/mod.rs`. It matches `canonical == name ||
  aliases.contains(&name)` over `bindings()`, the same resolution rule that
  dispatch uses.
- The runtime classifier (Phase 5) and the DMLS walk (Phase 6) call it. DMLS
  reaches it through a public wrapper on the `expression` module; the binding
  table stays private.
- Neither walker may spell `"is_null"` or `"is_empty"`.
- Add a registration test asserting that exactly those two canonicals carry
  the marker, and that each alias (`isnull`, `isempty`) resolves to true.

**Why:** the spec's wording ("registered aliases, resolved through the
function catalog (`FunctionBinding`)") describes a mechanism that exists
today. Plan options (a) and (b) assumed it did not. A marker on the binding
honors the spec literally, covers the aliases for free, and adds no registry.
Resolved Decision 16 schedules the rule for retirement; deleting it then
means removing one flag and one function.

### R-2 — The shared classifier *(Phase 5, Phase 6)*

**Verdict: write one new shared classifier. Leave `collect_variable_roots` and
`walk_context_variables` byte-for-byte unchanged. `ctx.*` warnings keep
today's exhaustive behavior.**

- The new walk sits over `Expr`/`SpannedExpr`. It yields each `Variable` root
  in evaluated positions, tagged with a suppression reason:
  - `Fallback` primary: suppressed.
  - `Fallback` RHS: not suppressed.
  - `Ternary` condition: suppressed.
  - A `Ternary` branch reference to a root guarded by the condition:
    suppressed.
  - Other branch references: not suppressed.
  - A direct bare-variable argument to an absence predicate (R-1):
    suppressed.
  - A nested argument such as `is_empty(trim(x))`: not suppressed.
- Runtime (Phase 5) adds short-circuit reachability on top of this. DMLS
  (Phase 6) uses the structural result alone (Resolved Decision 6).
- `collect_variable_roots` (`subtree.rs:536`) keeps its subtree-strict
  semantics. That is a public `SubtreeStrictness::Strict` contract, and
  changing it would alter Claudine's explicit subtree choices.
- `walk_context_variables` / `collect_context_warnings`
  (`interpolation/evaluator.rs:396`) stay as they are. So
  `{{ ctx.typo || 'default' }}` still emits its `ctx.*` warning. Changing
  that is unrequested scope, as the plan recommends.

### R-3 — Fatality scope: `interpolate_text` call sites *(Phase 4)*

**Verdict: the spec excludes subtree; every other full-document call site
becomes strict.**

Call sites, verified by searching for `interpolate_text(` and
`interpolate_value(`. `interpolate_value` delegates mixed text to
`interpolate_text` at `rewrite.rs:298` and `:315`.

| Site | Verdict |
| --- | --- |
| Body stage (`inline/interpolation.rs:59`, via `interpolate_text_located`) | **Strict** (the spec names it) |
| Frontmatter mixed text (`frontmatter_interpolation.rs:246`, via `interpolate_value`) | **Strict** (the spec names it) |
| Frontmatter shell expansion (`frontmatter_shell_expansion.rs:1546`, `:1727`, `:1785`) | **Strict**. All three already pass `true`; the typed policy only makes that explicit |
| Directive targets (`directive_targets.rs:371`, via `interpolate_value`) | **Strict**. Only whole-value spans reach it, and those are already fatal, so this is a no-op in practice; pin it anyway |
| Preflight best-effort frontmatter (`interpolate_frontmatter_best_effort`, `frontmatter_interpolation.rs:431`) | **Unchanged (Lenient)**. It is approval discovery, not output. It deliberately tolerates per-key failures, and the real run surfaces them strictly |
| Subtree (`subtree.rs:497`, via `interpolate_value`) | **Unchanged**. `SubtreeStrictness` still governs it (the spec excludes it) |

**Why:** each included site evaluates author-written expressions during
full-document composition. Excluding one would recreate the silent
passthrough this feature closes. The S-3 findings below record what turns
red.

### R-4 — Shape of the failure-policy parameter *(Phase 4)*

**Verdict: replace `fail_fast: bool` on `interpolate_text` /
`interpolate_text_located` with a typed two-variant `ExpressionFailurePolicy
{ Lenient, Strict }`.**

- Every caller states its policy explicitly. Subtree maps
  `SubtreeStrictness::Lenient` to `Lenient`. Document stages pass `Strict`.
- `ComposeOptions::with_fail_fast` stays public and keeps governing the
  non-expression stages (TOC linking, non-structural transclusion). It stops
  reaching the expression helper.
- The helper is `pub(crate)`, so no public signature changes, which satisfies
  the spec's Compatibility section.

### R-5 — Accumulator ownership and threading *(Phase 5)*

**Verdict: fold per unit. Candidates travel on `ComposeReport` and are merged
at the existing `ComposeReport::merge` points. No lock is added to
`ComposeOptions`.** Spike S-2 settles this; the rejected option's failure
modes are recorded there.

- Body-stage evaluation cannot write to the report directly.
  - `Evaluator` holds `&L` (`interpolation/evaluator.rs:211-214`), and every
    `EvaluationLookup` method takes `&self`.
  - So candidates are staged per stage: in the interpolation result, or in a
    stage-local, uncontended cell such as the `CurrentScope.memo` precedent
    (`context/current.rs:355`).
  - `run_stage` (`inline/interpolation.rs:24-71`) then pushes them onto
    `&mut ComposeReport`, exactly as it already does for `warnings`.
- `FrontmatterShellExpansionReport` (`frontmatter_shell_expansion.rs:414`)
  needs a candidates field beside its `warnings`. Ternary evaluation there is
  sequential; only the prepared shell commands run under rayon (`:1393`
  versus `:1413`).
- Reconciliation into `dm.expression.unknown_identifier` warnings happens
  after the merge, where the effective schema is in scope (R-9).

### R-6 — `fatality_characterization.rs` disposition *(Phase 4)*

**Verdict: update in place. It stays the living drift guard and becomes the
authoritative statement of the post-change contract.**

- The matrix currently calls `interpolate_text` with `fail_fast` directly.
  After R-4 its axis becomes `ExpressionFailurePolicy`:
  - `Strict` cells: every failure is fatal.
  - `Lenient` cells: keep today's lenient expectations.
- Also pin that the document body entry uses `Strict`, so a caller regressing
  to `Lenient` is caught, not only the helper.
- Update the module's `//!` policy prose in the same change.

### R-7 — Corpus audit walk parameters *(Phase 1)*

**Verdict: walk the four directories recursively, covering `.md` and `.yaml`
files and every subtree, `_`-prefixed ones included.**

- **`.md` files:**
  - Body `{{ … }}` through `ExpressionFinder::new(...).scan()`, which skips
    literals and fenced or indented code.
  - `when=` options through `scan_darkmatter_directives`.
  - Every frontmatter string value, recursively, through
    `ExpressionFinder::scan_plain`.
  - Claudine condition keys (`when`, `while`, `until`) as condition
    expressions.
  - `$()` ternary conditions, plus value branches, through
    `parse_frontmatter_shell_value_spanned`.
- **`.yaml` files:** string values are scanned the same way as frontmatter.
- **Excluded by construction:** `{{{ … }}}`, fenced examples, and GitHub
  Actions `${{ … }}`.

### R-8 — Quick-fix structured-data carrier *(Phase 6)*

**Verdict: carry the replacement in `Diagnostic.data`. Fall back to
recomputing from `diag.range` plus a re-parse when `data` is absent. Never
parse the message.**

- DMLS uses `lsp-types = 0.97` / `lsp-server = 0.8` (`dmls/Cargo.toml:21-22`).
  `Diagnostic.data` is unused today, and `code_actions.rs:47-87` dispatches
  on `diag.code` alone.
- `dmls/design/markdown-lsp.md:96` already expects `data` to round-trip into
  code actions.
- DMLS never checks the client's `publishDiagnostics.dataSupport`. Not every
  client echoes `data`, and nobody has verified whether Zed does. That makes
  the range-plus-reparse fallback required, not optional.
- Payload shape: a small typed struct serialized with `serde_json`, for
  example `{ root, replacement }`. Keep one type shared between the diagnostic
  producer and the code-action consumer.

### R-9 — Runtime known-root coverage for caller layers *(Phase 5)*

**Verdict: the body lookup chain does not answer the known-root question
today. Phase 5 must extend the lookup contract. The effective schema is
supplied at reconciliation, not through the lookup.**

- `is_known_variable_root` has exactly three implementations:
  - The trait default, `true` (`expression/mod.rs:291`).
  - `LayeredLookup` (`subtree.rs:268`).
  - A forwarding impl on `DeferrableLookup` (`inline/interpolation.rs:173`).
- `ResolvingLookup` and `EffectiveState` keep the default. So the body chain
  answers "known" for every root today.
- `EffectiveState.data` (`context/effective_state.rs:146`, built at
  `pipeline/mod.rs:396-417`) already contains:
  - `--set` overrides.
  - Caller file parameters.
  - `external_state`.
  - A transcluded child's inherited parent state, which arrives as its
    `external_state` (`transclusion/engine.rs:1438`, `:1457`) minus
    `prologue`, `epilogue`, and `ctx` (`:1627-1638`).
- Phase 5 therefore overrides `is_known_variable_root` on `EffectiveState` in
  the same shape as `LayeredLookup`: data keys plus reserved roots.
  `ResolvingLookup` forwards to it. That covers the caller layers.
  - Unverified: whether `caller_input_records` (`options.rs:134`) ever holds
    a root that never reaches `data`. Phase 5 must pin that with a test.
- The effective schema (document `$schema` plus `baseline_schema`,
  `trigger_schemas`, and extensions) exists only as a pipeline-local
  `PreparedSchemas` (`pipeline/mod.rs:184`, `schema_validation.rs:71-177`).
  - The lookup never holds it.
  - Reconciliation runs in the pipeline, where `prepared_schemas` is in
    scope. It drops any candidate whose root the effective schema declares.
  - This follows the spec's rule not to reconstruct schema state inside the
    evaluator.
- Third-party `EvaluationLookup` implementations keep the default `true`, so
  they are never warned (Resolved Decision 10).

### R-10 — Severity raise blast radius *(Phase 6)*

**Verdict: the raise is safe. One unit test and one doc row change. No
snapshot, protocol test, or editor configuration filters on this severity.**

The raise touches `dsl.rs:740` and `diagnostics/frontmatter.rs:653-654`.
Items that must change:

1. The unit test
   `unknown_expression_root_is_informational_frontmatter_diagnostic`
   (`dmls/src/diagnostics/frontmatter.rs:1188-1198`) asserts `INFORMATION`.
   Rename it and flip the assertion.
2. The `## Severity` section of `dmls/docs/diagnostics.md` (`:150-164`, table
   row near `:143`) needs a **Warning** entry.

Items that stay green:

- `frontmatter.rs:1213`, `:1256`, and `:1572` filter by code, not severity.
- `dmls/tests/lsp_session.rs:1902` asserts absence.
- There are no `.snap` files for this code. The Zed extension lives at
  `dmls/zed-dmls`, not the `darkmatter/zed-dmls` path the plan cites, and has
  no severity configuration.

## Spike Findings

### S-1 — Corpus collision spike

The spike is the permanent test
`darkmatter/lib/tests/dasherized_identifier_corpus.rs`, run against the
unmodified lexer.

- **Real breaking-form usages:** exactly one.
  - `prompts/_reviews/review-spec-inline.md`: `::block when="depends-on"`.
  - It does **not** rely on subtraction. With the current `md`,
    `when="depends-on"` fails with `Subtraction requires numeric operands`
    (exit 1) whenever its enclosing block is reached. The enclosing block
    checks `frontmatter(spec, "depends-on")`, so the author evidently meant
    that key.
  - The pre-approved rewrite (`x-1` → `x - 1`) would keep the prompt broken.
  - The sibling `when="parent"` / `when="peers"` blocks share the same
    wrong-document mistake; they render silently empty.
  - **Resolution:** all three now read `when='frontmatter(spec, "…")'`.
    Verified with `md compose`: the `depends-on` block renders when the spec
    declares the key, and the others stay hidden.
- **Unparseable shipped expressions, which gate B found:** three prompts.
  Each already fails today, independent of this feature.
  - `prompts/_docs.md`: `::block when=!ctx.is_monorepo"` is missing its
    opening quote (parse error, exit 1). The quote is now added.
  - `prompts/documentation.md`: `when="file_exists({{doc.doc}})"` and
    `when="file_empty({{doc.doc}})"`. A `when=` is not interpolated before it
    is parsed, so both fail to parse (exit 1). Both now read `(doc.doc)`.
    - **Residual, not fixed:** `file_empty` is not a catalog function. That
      block still fails at evaluation (`Unknown function: file_empty`)
      whenever it is reached. Only the author knows the intent.
  - `prompts/_add/add-expressions.md` (untracked, in-flight work): the
    frontmatter `description` contains `` `{{ … }}` ``. Today it warns and
    exits 0; under R2 it becomes fatal. Now `{{{ … }}}`, which renders the
    intended `{{ … }}` text. Verified: frontmatter-only compose is
    warning-free.
- **Not counted, correctly:** every other dash-bearing `{{ … }}` in the corpus
  sits inside a string literal (`'/review-plan-' + iteration`,
  `'-' + (name || …)`). There are no GitHub Actions `${{ }}` spans, and no
  `$()` ternaries (the one `$(…)` is a plain shell command).
- **Exit:** zero real collisions remain. Phase 2 is unblocked.

### S-2 — Accumulator threading spike

The finding comes from reading the merge points; nothing was prototyped as
code.

- **Where results are merged:**
  - Transclusion runs its rayon step over prepared items at
    `pipeline/phases.rs:334-335`. It folds each result single-threaded with
    `report.merge(resolved.report)` at `:418`.
  - Each child owns a fresh `ComposeReport`: `engine.rs:950` for local
    children, `:1093-1102` for remote ones.
  - A cached child is replayed with `report.merge(cached.report.clone())` at
    `engine.rs:1486`.
  - Nested `as_markdown()` children fold in `NestedCompose::finish` at
    `nested.rs:164`.
  - The plan's `engine.rs:~605` citation is the prepare step, not a merge.
- **Option (b), per-unit fold on `ComposeReport`: chosen.** No cross-document
  lock is taken, so Performance invariant 3 holds.
- **Option (a), `Arc<Mutex<…>>` on `ComposeOptions`: rejected.**
  - `ComposeOptions` is `Clone` and is cloned for every child
    (`engine.rs:1083`, `:1453`, `nested.rs:132`, `:193`). A shared
    accumulator would therefore have to be one `Arc<Mutex>` contended by
    every rayon worker, which breaks invariant 3.
  - Concrete failure mode: run-local compose caching is single-flight
    (`cache/runtime.rs:167`). A child transcluded twice runs its compose
    closure once. With a shared lock the second parent records no
    candidates, so its warning disappears. With option (b) the candidates
    ride inside `cached.report` and are replayed. A shared set would also
    lose the per-source attribution that Requirement 5's identity key needs.

### S-3 — Fatality blast-radius spike

The probe made both lenient branches in `interpolate_text_located`
(`rewrite.rs`) unconditionally fatal, which includes the subtree's lenient
path. It then ran the darkmatter and Claudine suites. The probe was reverted
by restoring the file from a byte copy.

**Totals:**

| Suite | Probe run | Baseline (probe reverted) | Probe-induced |
| --- | --- | --- | --- |
| `darkmatter` `just test` | 8127 run, 26 failed | 8127 run, 0 failed, 29 timed out* | **26** |
| `claudine` `just test` | 7036 run, 4 failed | 7036 run, 4 failed (the same 4) | **0** |

\* The 29 timeouts are the known HTTP-client cluster
(`remote_fetch::persistent_cache_tests`, `tests::provider_network`) under a
load average of 17–37. All 55 tests in those two modules pass when re-run at
`--test-threads 2`.

**Claudine's four failures predate this session.** They are
`shipped_implement_plan_prepares_with_unset_optional_commit_message`,
`shipped_implement_plan_preserves_supplied_commit_message_in_preflight_command`,
`shipped_implement_plan_launches_without_a_sibling_spec`, and
`shipped_implement_prompts_have_not_drifted_from_their_fixture`. All four
come from the uncommitted edit to `prompts/_implement/implement-plan.md`
(`LifecycleObjectDataThroughInterpolationPositional` on `initialize.set`, plus
the route-drift hash pin). This phase did not touch that file. Making every
expression failure fatal breaks **no** Claudine test.

**Catalog of the 26 darkmatter failures, sized for Phase 4:**

- **Tests that deliberately assert lenient document behavior (22).** These
  must be inverted to assert the fatal, typed, located error:
  - `interpolation::fatality_characterization`, 4 tests:
    `fatality_matrix_is_locked`, and `body_{arg_type,arity,parse_failure}_is_warning_in_lenient_mode`.
    These are the R-6 in-place rewrite.
  - `frontmatter_interpolation::…::mixed_malformed_interpolation_stays_warning_without_fail_fast`
    (1).
  - `compose::tests::rendering`, 2 tests:
    `test_interpolation_parse_error_preserves_original` and
    `test_interpolation_bare_pipe_produces_parse_error`.
  - `compose::tests::provider_network::generic_expression_failures_still_warn_on_the_body_surface`
    (1).
  - `tests/expression_regression.rs`, 11 tests: the
    `regression_arithmetic_type_mismatch_*_non_fail_fast` family,
    `regression_division_by_zero_default_non_fail_fast`,
    `regression_division_by_zero_error_in_interpolation`, and
    `regression_remainder_by_zero_default_non_fail_fast`.
  - `tests/context_functions.rs::recent_commits_rejects_invalid_counts_in_the_body`
    (1). It asserts "a type error only warns in the lenient body".
  - `tests/nested_composition.rs`, 2 tests:
    `empty_and_non_string_arguments` (`as_markdown(missing)` with a null
    argument) and `nested_diagnostics_keep_their_provenance` (an inner
    `length(1, 2)` arity failure).
- **Fixtures that use a broken expression as scaffolding (3).** The failing
  expression is incidental; Phase 4 swaps the fixture, not the assertion:
  - `tests/nested_composition.rs::self_composition_reaches_the_depth_limit`.
    It relies on `as_markdown(again)` with a null `again` staying lenient to
    reach the depth limit.
  - `cli/tests/compose_terminal_detection.rs`, 2 tests
    (`compose_redirected_does_not_spawn_appearance_defaults` and
    `compose_verbose_perf_performs_single_terminal_detection`). Their
    `DOC_WITH_WARNING` uses `{{ 1 + }}` only to force the warnings footer.
    They need a warning source that survives R2, such as a `ctx.*` typo
    warning.
- **A subtree regression pin (1).**
  `subtree::tests::dm2_lenient_tolerates_malformed_span_in_mixed_string`
  failed because the probe also flipped the helper for subtree. That proves
  `compose_subtree(..., Lenient)` routes through the same helper. Under R-4 it
  must stay green unchanged. This is the spec's "lenient subtree is
  byte-for-byte unchanged" regression.
- **Genuine latent breakage in shipped artifacts:** none in the test suites.
  - The corpus's only latent R2 failure was the
    `prompts/_add/add-expressions.md` description, now fixed (see S-1).
  - The spec-named `{{absolute-filepath}}` and `{{review-file}}` no longer
    appear in any executable fixture.
  - `{{review-file}}` survives only as a hand-built error value in two
    Claudine unit tests (`claudine/lib/src/composition/error/tests.rs:52`,
    `claudine/cli/src/output/error_walker/tests.rs:159`). Neither composes,
    so neither turns red. It also survives as narrative in
    `claudine/docs/topics/composition.md:430` and
    `.claude/skills/claudine/composition.md:545`, which go stale after R1
    (Phase 7).

**Policy question for Phase 4 (not blocking): nested `as_markdown()`.** The
inner document of `as_markdown()` is itself a full-document composition, so
under R-3 it is **Strict**. `nested_diagnostics_keep_their_provenance` must
then assert a fatal inner error with provenance instead of a lenient warning.

**Does any fixture need the R1 lexer fix first?** No. None of the 26 failures
involves a dash-joined identifier, so Phase 4 can proceed independently of
Phase 2.

**Hazard found while probing, for Phase 4.** A `{{{ … }}}` literal in a
frontmatter value becomes the text `{{ … }}` after frontmatter
interpolation. If the body then interpolates that value (`{{ description }}`),
`interpolate_text`'s rescan pass finds the generated `{{ … }}` and tries to
parse it. Today that is a warning. Under a Strict body policy it would be
fatal, even though the author wrote a literal.
- Reproduction: frontmatter `description: "callable from {{{ … }}}"`, body
  `{{ description }}`.
- Phase 4 must decide whether rescan-pass (`depth > 0`) failures are strict.
  Those failures already carry no span (`rewrite.rs:201`), which is a hint
  that generated text differs from authored text.

### S-4 — DMLS cursor-scan spike

- **Chosen approach for Phase 2, Task 2.3:** add one pure public helper beside
  `read_variable` in `lexer.rs`, for example `identifier_prefix_start(prefix:
  &str) -> usize`, sharing its dash rule. `completion_partial`
  (`overlay/expressions.rs:249`) and `value_completion_partial` (`:619`) both
  call it. These are the only two scans in `dmls/src` with this character
  class.
- **Why not `lex_spanned`:**
  - It fails the whole input on the first lexer error, and completion input
    is usually incomplete.
  - Under the new rule, `spec-` lexes as `Variable(spec)`, `Minus`, which
    would drop the `spec-*` filter mid-typing.
  - It would also force a `ParseMode` choice onto a cursor scan.
- **How the helper decides:**
  - Scan backward over `[alnum _ . -]` to a candidate start, then walk
    forward applying the lexer rule.
  - An inner `-` joins only if the character before it continues an
    identifier and the character after it does too:
    - `foo--bar` gives the partial `bar`.
    - `a -b` and `a- b` do not join.
  - The leftmost segment must start with a letter or `_`, so `4-2` gives the
    partial `2`.
  - `.` joins only before a letter or `_`, as in `lexer.rs:889-892`.
  - A **trailing** dash at the cursor joins for completion filtering only,
    and only after an identifier character. `spec-` gives the partial
    `spec-`; `spec--` and `a -` do not join.
- **Hover and go-to-definition** already work from parser AST spans:
  - Hover: `dsl.rs:261` → `hover_markdown`, and
    `diagnostics/frontmatter.rs` `hover_markdown_condition`.
  - Definition: `dsl.rs:394-398`.
  - All of these follow the lexer change automatically. Phase 2 still needs
    cursor-on-dash tests for each.
