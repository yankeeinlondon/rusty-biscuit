---
kind: plan
total_phases: 4
created: 2026-10-27
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
---

# Shell-site placeholder syntax reuse

## Summary and Definition of Done

Prepare one retained syntax collection per `SequencePreflight::prepare_site` invocation, then use it for every static placeholder analysis. Darkmatter owns scanning, parsing, grammar, projection, and root analysis; Claudine retains approval-scope and target-identity policy. Preserve existing text APIs, stored template representations, evaluation operations, approval membership, and error precedence.

Completion means:

- Each placeholder occurrence is parsed exactly once for static analysis, including malformed occurrences, repeated text, failed template grammar, late roots, and presence lookup. Spanned syntax survives for diagnostics; conversion to `Expr` happens at most once per successful occurrence.
- The existing counted sequence fixture records exactly `S × H × 6` expression parses, with unchanged graph contents. At 100 steps and ten holes this is 6,000 instead of 10,000.
- Existing and extended L1 tests preserve typed errors and their locations, template timing/types/roots, exact approval versus template approval, fresh evaluation, and ordinary CLI wiring.
- Both area's `just test` and `just lint` pass with counter tests actually enabled. Only the existing A6 allocation scenario and `A6_shell_preparation` timing case are sampled, on the current host.
- Current documentation, API comments, skill guidance, and an implementation log describe the delivered boundary and measured results. No commit, lifecycle move, or formatting command is part of execution.

All paths below are repository-relative. Waves have globally unique numbers. Tasks within a wave may use concurrent non-interactive agents; later waves require the preceding wave's checkpoint. Each task owns its listed files, and agents must not edit another task's files. The orchestrator executes every phase through completion without a human gate.

## Phase 1 — Establish the syntax boundary

### Necessary Rules

1. **Optimize only sequence shell-site static analysis.** Migrate `prepare_site`, its `refuse_unapprovable_reads` and `mark_late_holes` helpers, and its calls to presence lookup. Its task and collected stack callers are in `claudine/lib/src/composition/sequence/preflight/mod.rs`; container lifecycle callers also reach it from `preflight/containers.rs`. Keep prompt/container discovery paths that bypass this method unchanged. Evidence: those callers and the specification's preparation boundary. The container callers that already use `prepare_site` receive the optimization automatically.

2. **Introduce `PreparedInterpolation` in Darkmatter's expression preparation module.** Implement it in `darkmatter/lib/src/markdown/compose/expression/prepared.rs` and re-export it and `PreparedPlaceholder` from `expression/mod.rs`. Use `PreparedInterpolation::new(text: &str) -> Self`, `text() -> &str`, `placeholders() -> &[PreparedPlaceholder]`, and `view(range: Range<usize>) -> PreparedInterpolationView<'_>`; also re-export the view type. The collection owns its source and scanner metadata. Each occurrence stores whole-placeholder range, trimmed expression text, expression offset, and `Result<PreparedExpression, PreparationError>`. Accessors expose these read-only. Add `PreparedPlaceholder::expr() -> Option<&Expr>` backed by `std::sync::OnceLock<Expr>` and `SpannedExpr::erase`; failures return `None`. Do not key syntax by expression text. Evidence: `PreparedExpression` already carries text, offset and spanned syntax; `parse` itself erases `parse_spanned`, and the spec requires occurrence identity and preserved failures.

3. **Share the existing scanner/parser implementation.** Extract a private occurrence-scanning helper from `prepared.rs::scan_leaf`. It uses `ExpressionFinder::scan_plain` and `parse_in_mode(..., ParseMode::Interpolation)`, attempts each occurrence once, and retains failures in source order. `scan_leaf` consumes this machinery with first-error short circuit so `prepare_value` keeps its all-or-error contract. The new collection consumes every occurrence. Preserve literal/token metadata for grammar rejection. Keep `Parser`'s existing `ExpressionParse` instrumentation; do not introduce an uncounted parser. Evidence: `scan_leaf`, `parse_in_mode`, `PreparationError`, and `expression/parser.rs`.

4. **Views carry coordinates, never new syntax.** A full view covers `0..raw.len()`; an inner command view covers its trimmed byte range in raw source. View iteration selects contained occurrences and translates whole-hole spans and expression offsets by subtracting the range start; literal/token rejection likewise considers only metadata contained in the view. Raw diagnostic coordinates remain available from the parent. Store no schema, guards, late classification, values, or approval decisions in these types. Use byte ranges, including for Unicode whitespace and preceding multibyte text. Views accept character-boundary ranges within the source; invalid caller ranges are programmer errors, asserted rather than turned into authored-input errors. Evidence: `PreparedExpression::leaf_span`, `Placeholder::span`, and the two coordinate systems in `prepare_site`.

5. **Extract `set` structure without parsing a template.** Add `authored_shell_command<'a>(value: &'a str, key: &str, ctx: &SourceContext) -> Option<AuthoredShellCommand<'a>>` in `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`, exported through `compose/mod.rs`. Give the result `command: &'a str`, `span: Range<usize>` into the original value, and `conditional: bool`. Factor the existing `authored_shell_template` extraction into it, using the same `find_unquoted_closing_paren` and `split_top_level_ternary`. Have the old API delegate and retain its existing return type and parsing behavior. Do not replace extraction with `parse_frontmatter_shell_value_spanned`: that API is explicitly lenient and builds a broader shell description. Do not add suffix validation here: existing `authored_shell_template` extracts the inner command without validating trailing suffixes; later resolution owns that validation. Conditional bodies still produce `TemplateGrammarError::Conditional`. Evidence: both extraction APIs in this file and `prepare_site`'s `SetValue` branch.

6. **Use explicit parsed counterparts and shared bodies.** Export `parse_authored_prepared(view: PreparedInterpolationView<'_>) -> Result<CommandTemplate, TemplateGrammarError>`, `project_hole_expr(expr: &Expr, scope: &TypeScope<'_>) -> HoleType`, and `CommandTemplate::project_types_prepared(&mut self, view: PreparedInterpolationView<'_>, scope: &TypeScope<'_>)` from the template surface. Add `DeferredShellRoots::late_roots_of_expr(&self, expr: &Expr, derived: &HashSet<String>) -> Option<Vec<String>>` and `lifecycle::shell_template::placeholder_guard_expr(expr: &Expr, scope: &TypeScope<'_>) -> Option<GuardOrigin>`. Existing text functions delegate to the same analysis bodies after their existing parsing step. Match template holes to retained occurrences by whole-placeholder span, never text. Failed expressions keep `HoleType::fallback(TypeProvenance::NotBareProperty)`. Evidence: `grammar.rs::Parser::take_hole`, `typing.rs::project_hole`, `template/mod.rs::project_types`, `candidate.rs::late_roots_of`, and `shell_template.rs::placeholder_guard`.

7. **Preserve grammar and root semantics exactly.** Grammar still rejects interpolation literals/tokens, program-word holes, unsupported operators, redirections, substitutions, and quoting failures. Keep authored text and template shape unchanged. Move the existing `read_roots`/filter/`dedup` body into `late_roots_of_expr`; retain its traversal order and adjacent-duplicate policy rather than replacing it with a set or global deduplication. Evidence: `grammar.rs::parse_authored` and `candidate.rs::late_roots_of`.

8. **Reuse diagnostic validation and its peculiar ordering.** Add `expression::validate_interpolation(prepared: &PreparedInterpolation, view: &BindingView) -> Result<Vec<ValidationDiagnostic>, PreparationError>` using the same relocation logic as `validate_prepared`. Any retained failure returns the first `PreparationError` and no diagnostics. Add Claudine `approval_diagnostics_prepared(prepared: &PreparedInterpolation, scope: LifecycleScope)` with the existing result type; the existing text API constructs the collection and delegates. Keep all normal validation diagnostics first, then the `LATE_NAMESPACES` append pass. Preserve that append pass's current local read spans and error `span: None`; correcting those is outside this optimization. Evidence: `claudine/lib/src/composition/lifecycle/bindings.rs::approval_diagnostics` and `unavailable`, plus Darkmatter `validate_prepared`, `validate_expression`, and `relocate`.

9. **Preserve target policy and winning errors.** Add `shape::first_target_identity_root_prepared(&PreparedInterpolation) -> Option<String>`; keep its text entry point for existing tests by delegating. Iterate successful retained expressions in occurrence order and call `static_access_reads`, retaining `target_identity_path` unchanged. Malformed expressions suppress the entire approval-diagnostics result but never suppress valid target reads. Thus malformed plus `err` proceeds to resolution/grammar handling; malformed plus `ctx.agent` fails with `SequenceShellTargetIdentity`. A malformed `set` wrapper fails extraction before collection. After refusal checks, a required template's grammar error precedes resolution; an optional invalid template proceeds to exact-byte resolution; missing values are checked after resolution with the existing guard exception. Evidence: `prepare_site`, `refuse_unapprovable_reads`, and `shape.rs::first_target_identity_root`.

10. **Keep raw and authored consumers distinct.** Approval diagnostics, target identity, and failed-grammar fallback holes consume the raw view, matching today's calls. Grammar, template typing, and successful-template timing consume the authored view. Preserve the `raw.contains("{{")` route selection, including literal interpolation syntax. Do not infer placeholder presence from collection length and silently change the route. Presence lookup for `resolved.missing` uses the first raw occurrence with the same trimmed expression text; identical text has identical syntax, while template analyses still use occurrence spans. This lookup must never call a text parser. Evidence: `prepare_site`'s fast path, fallback `scan_plain(raw)`, and `resolve_approval_time`'s text-based missing-value deduplication.

11. **Evaluation stays independent.** Leave `resolve_approval_time`, `compose_approval_value`, `record_resolved_bytes`, frontmatter late-root scans, removed-read checks, runtime fills, and presence evaluation unchanged. Keep the collection on the preparation stack and drop it on every return; do not add it to `PreparedShellSite`, `CommandTemplate`, `Placeholder`, `ShellCandidate`, or approval records. Evidence: `resolve_approval_time` deliberately composes both the raw command and individual approval-time holes, and the spec excludes those evaluations from reuse.

12. **No spike or broad benchmark campaign.** The specification's five-pass trace and 6.4% estimate already justify implementation. Run the existing quick A6 samples once after correctness passes; preserve their workloads and Criterion's existing ten samples, 300 ms warmup, and two-second measurement. Report regressions as measured results without inventing a speedup threshold or broadening the workload. Evidence: acceptance item 5 and `claudine/lib/benches/pipeline_work.rs::configured`.

13. **No configuration matrix is required.** This changes retained syntax for an interpolation text grammar, not a file-format/configuration loader, schema field, or deserializer. The task's Input Robustness Matrix explicitly excludes text grammars. Cover scanner, syntax failure, and coordinate behavior with the tests below instead.

14. **Keep changes portable and scoped.** Use ordinary Rust data structures, temporary fixtures, the existing test toolkit, and the native argv-recorder fixture; add no OS branch or dependency. Do not change CI scope or run real providers, terminal prompts, L2/L3 windows, `cargo fmt`, commits, or `just complete`. Area `just test` already enables `claudine/work-counters` and `darkmatter/work-counters`, as shown in both justfiles. `just/devops.just::_lint` runs Clippy and does not require formatting.

### Wave 1

- [ ] **Characterize precedence**
  - Own `claudine/lib/src/composition/sequence/preflight/tests.rs`. Extend `graph_for`/`err_for` cases before any Claudine consumer migration. Use public graph construction, asserting concrete `CompositionError` variants, typed source chains, expression text, binding spans, and rendered messages.
  - Pin malformed expression alone, malformed beside `err`, malformed beside target identity, and each valid read both before and after the malformed occurrence. Pin first-task program-hole exact eligibility and later-task grammar refusal, plus malformed `set` extraction precedence.
  - Add padded/Unicode `set` and accepted suffix fixtures; assert diagnostic offsets against raw wrapper bytes and template spans against extracted command bytes. Preserve the existing late-namespace span behavior described in Rule 8.
  - Validate these tests against the unchanged preparation implementation using the area's narrowed `just test` filter. This supplies executable compatibility expectations before migration.

- [ ] **Retain occurrence syntax**
  - Own Darkmatter `expression/prepared.rs` and `expression/mod.rs`. Implement Rules 2–4 and `validate_interpolation` from Rule 8. Reuse the validator's diagnostic mapping rather than duplicating traversal or relocation.
  - Add unit coverage alongside preparation code for mixed valid/invalid occurrences, first-error compatibility of `prepare_value`, repeated expression text at distinct spans, cached erasure, no placeholders, triple braces, literal tokens, and Unicode offsets.
  - Under `work-counters`, assert exactly one parser invocation per scanned occurrence, including failed parses. Repeated view access, validation, and erased-AST lookup add zero parses. Construction and validation invoke no file function or lazy provider.

Checkpoint: characterization tests pass on the old Claudine route, and Darkmatter's collection retains both failures and successful syntax with exact scanner coordinates. These two tasks touch disjoint files and can run concurrently.

## Phase 2 — Connect static consumers

### Wave 2

- [ ] **Add parsed consumers**
  - Own Darkmatter `template/{mod,grammar,typing,candidate}.rs`, `lifecycle/shell_template.rs`, `frontmatter_shell_expansion.rs`, and `compose/mod.rs`.
  - Implement Rules 5–7. Factor grammar into a shared scanner-span-driven body; the prepared path obtains initial hole types from retained syntax. Keep the existing text grammar path's parse behavior stable for callers outside sequence preparation.
  - Make projection, late roots, and presence guards share their AST-based analysis bodies with text wrappers. Project both valid and failed occurrences without parsing again. Derive occurrence identity from template spans, including quoted/mixed arguments.
  - Extend `template/tests.rs` and inline shell-template/extraction tests with text/prepared result parity for grammar errors, initial fallback typing, schema validators, refinements, root order, literal syntax, and wrapper extraction ranges.

- [ ] **Adapt host policy**
  - Own Claudine `composition/lifecycle/bindings.rs` and `sequence/preflight/shape.rs`.
  - Add the prepared approval and target helpers specified in Rules 8–9. Keep text wrappers usable by existing callers and tests; migrate no unrelated call sites.
  - Add direct parity assertions for dead-branch unavailable reads/unknown functions, malformed suppression, valid target reads beside malformed occurrences, dotted/static-bracket equivalence, numeric/computed indexes, and unrelated `ctx`/`env` properties.
  - Prerequisite: Wave 1's collection and validation API. This task can run concurrently with parsed template consumers because it only depends on the expression surface.

Checkpoint: targeted nextest tests through area `just test` filters pass for both implementations; every added helper has fixed signatures and passive behavior. Existing public text API tests remain unchanged and passing.

## Phase 3 — Integrate and prove preparation behavior

### Wave 3

- [ ] **Wire site preparation**
  - Own `claudine/lib/src/composition/sequence/preflight/mod.rs`. Require all Wave 2 APIs. Extract the authored command first, create one raw collection, form its authored view, and build the template with `parse_authored_prepared`; retain the existing early plain-command path and extraction errors.
  - Pass the collection into refusal checks. Replace `project_types`, `mark_late_holes`, deferred guard lookup, failed-template fallback scanning/parsing, and missing guard lookup with retained-syntax consumers according to Rule 10.
  - Change `mark_late_holes` to accept the authored view; correlate every hole by span. Keep site schema/guard construction and all mode-selection/error branches in their existing order. Keep fallback valid late reads visible beside malformed expressions.
  - Review touched `///`, `//!`, and inline comments for drift. Preserve templates and approval records without syntax ownership changes.
  - Run Phase 1 characterization and existing sequence preflight tests. Confirm by source search that the preparation path no longer invokes text parsing helpers for any static consumer.

### Wave 4

- [ ] **Prove work counts**
  - Own `claudine/lib/src/composition/sequence/preflight/tests/scaling.rs`. Strengthen `shell_preflight_parses_the_same_expressions_per_step_at_10_and_100_steps` to require exactly `steps × 10 × 6`, retaining `assert_ten_hole_graph`. Add a small distinct-placeholder fixture with a different H to prove the formula is occurrence-based.
  - Add a private test helper in the preflight module's tests that constructs the collection and executes the same static-analysis helpers, excluding evaluation. Assert H parses for repeated text, deferred/guarded holes, failed grammar containing a valid late read, and a mixed malformed command; assert zero for plain/literal-only scanner cases. Do not assert the six-parse formula for malformed or guarded full preflight.
  - Use snapshot deltas from existing process-wide work counters, not replacement counters. Nextest process isolation prevents unrelated test increments.

- [ ] **Preserve observable behavior**
  - Own `claudine/lib/src/composition/sequence/preflight/tests.rs`, `claudine/lib/src/composition/lifecycle/executor/tests/shell_templates.rs`, and `claudine/cli/tests/l1/sequence_shell_templates.rs`. Wave 3 must be complete; this task runs alongside the work-count task in separate files.
  - Retain and extend existing cases `the_first_task_is_resolved_and_a_later_task_is_a_template`, `forbidden_roots_stay_refused_even_beside_a_late_root`, `a_value_derived_from_a_late_root_is_late_not_missing`, `stack_set_shell_values_are_collected_and_approved`, target-identity matrices, and guard/presence matrices. Assert `HoleTiming`, roots, validators, and refinements through returned graph templates.
  - Add uncovered cases for empty string/zero/false presence, unavailable reads and unknown functions inside dead branches, setup/teardown/on_error collection, and padded/suffixed whole-value `set` coordinates. Reuse existing fixtures and avoid duplicating cases already establishing the contract.
  - Add a hermetic fixture-file test using `frontmatter('fixture.md', 'status')`: prepare the command `tool {{ frontmatter('fixture.md', 'status') }}` once through sequence preflight, then fill its retained template twice with the existing expression evaluator and document-relative resolution context. Change the temporary file's `status` from `draft` to `ready` between fills and assert the changed argument. The real file function and context setup are demonstrated in Darkmatter `functions/fs_tests.rs::frontmatter_reads_whole_map_and_single_prop`; `functions/fs.rs::load_markdown` reads the file on each evaluation. Never reuse evaluated values in this test's setup.
  - Retain executor tests `a_shape_approved_shell_fills_from_the_state_current_when_it_runs` and `an_exact_approval_runs_the_stamped_bytes_and_never_rereads_the_value` as the separate resolved-byte/fresh-template proof; these already assert mutation after approval and need no duplicated test.
  - Keep the normal `Sequence`/`CliProcessFixture` invocation in `a_later_task_is_approved_only_by_its_shape` as the representative CLI acceptance proof, with its existing exact argv assertions. It already supplies an isolated home, policy, executable recorder, and fake provider without an approval prompt.

Checkpoint: exact parse counts pass with the feature enabled, all characterization assertions still pass, and returned graph/argv assertions prove analysis was not skipped. The new file-evaluation test demonstrates syntax retention cannot freeze values.

## Phase 4 — Document and validate the result

### Wave 5

- [ ] **Document syntax lifetime**
  - Own `claudine/docs/topics/pre-flight-checks.md`, `darkmatter/docs/inline/preflight-checks.md`, `darkmatter/docs/topics/expression-engine.md`, `.claude/skills/darkmatter/expressions.md`, and `.claude/skills/claudine/SKILL.md`.
  - Describe one sequence shell-site's passive syntax collection, occurrence identity, wrapper coordinate mapping, and drop-at-return lifetime; distinguish static analysis from evaluation and approval-mode behavior. Include a compact example with repeated `{{ name }}` occurrences and a small Mermaid flow from raw source through shared analyses to existing resolution.
  - Document the parsed helper surface and continued text entry points. State that malformed syntax is retained without early reporting. Do not link current documentation to this feature. Existing READMEs are overviews and need no change because public behavior and installation are unchanged; dependencies remain unchanged.

- [ ] **Audit static reuse**
  - Review the integrated source against every Necessary Rule. Search `prepare_site`, refusal checks, `mark_late_holes`, and their new helpers for `parse`, `prepare_value`, `project_hole`, `late_roots_of`, and `placeholder_guard`; classify each remaining call as collection construction or excluded evaluation and remove static text reparses.
  - Verify the collection never enters stored templates, candidates, approval records, preflight fields, or global caches. Review documentation/comments on every behavior-touched symbol, and check additive API exports compile for existing consumers through the two area's suites.
  - This read-only audit can run concurrently with documentation. Correct defects in the owning source files after collecting the findings; rerun affected targeted tests before the final gates.

### Wave 6

- [ ] **Run final gates**
  - Sequentially run `just test` and `just lint` in `darkmatter/`, then `just test` and `just lint` in `claudine/`. Use the existing recipes without `cargo test`, `cargo fmt`, workspace-wide gates, or extra terminal tiers.
  - Confirm nextest output includes the exact-count test and the new static occurrence tests. Both justfiles already enable `work-counters`; a filtered-out counter test is an incomplete validation result.
  - Fix failures caused by this implementation and rerun the affected gate. Record unrelated environmental failures with their exact command and evidence in the implementation log; do not mark the corresponding gate passed.

- [ ] **Record A6 evidence**
  - Run after final gates, with no simultaneous tests/builds or benchmark agents. Use the current host only and these two commands from the repository root:
    - `cargo bench --color=never -p claudine --features work-counters --bench pipeline_work_alloc -- A6`
    - `cargo bench --color=never -p claudine --bench pipeline_work -- A6_shell_preparation`
  - The allocation target's existing A6 selector emits both 10-step and 100-step rows; retain both and compare the 100-step row with the specification. The Criterion invocation has no `work-counters` feature, matching its documented measurement setup. Keep existing allocator/build settings and record environment overrides that affect them.
  - Create `claudine/features/2026-10-27-shell-site-placeholder-parse-once/implementation.md` recording revision, uncommitted patch status, host/OS, build profile/features, commands, test results, parse counts, allocation count/bytes/peak, and Criterion interval/median. Place the original 10,000 parses, 26,709,585 bytes, and 8.2117 ms beside the new 100-step observations. Separate count reduction from actual allocation/timing changes; account for retained spanned syntax and erased-tree allocation costs without claiming the estimated 6.4% as achieved.
  - Record implementation departures and remaining validation failures explicitly. End at implementation complete and ready for review, keeping this feature active. No human decision interrupts any preceding task.

Checkpoint: documentation and skill guidance match the code; both full area gates pass; one-host A6 observations and all compatibility evidence are recorded. The implementation is ready for the normal review cycle.
