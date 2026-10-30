---
kind: log
implementation_1: "2026-10-27T18:49:52-07:00"
---

## Implementation of Review Findings #1

> **started at:** 2026-10-27T18:49:52-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-27-shell-site-placeholder-parse-once/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag is `false` before repairs begin

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1    | Parse-count cases never exercise late or guarded placeholders | fixed | all required children fixed or clean; `scaling_tests::counted::static_analysis_parses_each_occurrence_once` asserts parses, per-occurrence late roots/proof, and hole timings |
| R1-F1.1  | Repeated `a` three times: three parses | clean | 3 parses; each `a` early, unproven; timings approval ×3 |
| R1-F1.2  | Deferred and guarded holes: real late root + presence condition; assert both states and count | fixed | `echo {{ seq.outputs[0] }} {{ seq.outputs[1] }} {{ a }}` under guard `seq.outputs[0]`: 3 parses, timings guarded:seq / deferred:seq / approval; unguarded control row: deferred:seq ×2 / approval |
| R1-F1.3  | Failed grammar beside a real late read; fallback analysis reports its root | fixed | `echo {{ seq.outputs[0] }} > out` under the guard: 1 parse, grammar fails (`timings` None), fallback reports root `seq`, proven |
| R1-F1.4  | Malformed `a +` beside valid `b`: two parses, one retained failure | clean | 2 parses; malformed `a +` retained as an approval-time hole; no late roots |
| R1-F1.5  | Real late read beside malformed `b +`; malformed neighbor does not hide the root | fixed | `echo {{ seq.outputs[0] }} {{ b + }}`: 2 parses, root `seq` in occurrences and timing deferred:seq |
| R1-F1.6  | Plain command: zero parses | clean | 0 parses; no occurrences; template with no holes |
| R1-F1.7  | Triple-brace literal: zero parses | clean | 0 parses; no occurrences; grammar rejects |
| R1-F1.8  | Data literal token: zero parses | clean | 0 parses; no occurrences; grammar rejects |
| R1-F1.9  | Full-preflight count controls (600 / 6,000 / 72) | clean | unchanged; `just test scaling_tests::` 14 passed |
| R1-F1.10 | Existing condition tests (missing `msg`, late `seq.outputs[3]`) | clean | unchanged; `just test composition::sequence::preflight::tests::` 230 passed |
| R1-F2    | Preserve schema validators and refinements in sequence-preflight tests | fixed | three hermetic tests in `preflight/tests.rs::schema_typed_holes`; mutation check (drop guard / drop schema in `prepare_site`) failed them, then restored |
| R1-F2.1  | Primary shell site with `number(min(1))` + `enum(release, debug)` under `mode == 'release'`: graph keeps validator and refinement (accept/reject each) | fixed | `a_later_task_keeps_its_schema_validator_and_guard_refinement`: count 2 accepted, 0 and "2" `DeclaredType`; mode release accepted, debug `GuardRefinement`; control `without_a_guard_the_declared_enum_accepts_every_member` accepts debug, rejects nightly |
| R1-F2.2  | Collected setup/teardown/`on_error` commands share the projection; no task condition | clean | invariant: stack sites (`shape::collect_stack_shell_sites`, scope `guard: None`, mod.rs ~1783) and container events (`containers.rs:23`, `guard: None`) all call `prepare_site`, whose `owner_schema`/`site_types`/`project_types_prepared` (mod.rs ~1900–1911) is the one projection |
| R1-F2.3  | Whole-value `set` command shares the projection with translated spans | clean | `ShellSiteKind::SetValue` uses `prepared.view(span)` into the same projection; `a_container_set_command_keeps_the_schema_validator` (count 0 rejected, no refinement) |
| R1-F2.4  | Darkmatter standalone text-versus-prepared projection parity | clean | Darkmatter `projected_types_match_with_schema_validators_and_refinements` unchanged |

- starting the work on 'R1-F1 parse-count late/guarded cases' at 18:51:00
        - defect class: a test counts work without proving that its input reaches the analysis state it claims to cover
        - discovered: production builds the guarded scope with the private `TaskGuard::parse` + `site_types` (mod.rs), which the child test module can call directly, so the helper reuses them instead of a new `TypeScope::unguarded` site (the type-scope ownership guard needs no new entry)
        - discovered: `seq.outputs[0]` as the `when:` condition (truthiness, the shape of the existing `a_guarded_late_value_is_left_to_its_command_not_the_step_barrier` test) proves `seq.outputs[0]` present and does not prove `seq.outputs[1]`
        - discovered: `echo {{ a + }} {{ b }}` and `echo {{ seq.outputs[0] }} {{ b + }}` pass template grammar (the malformed hole is retained, timing `ApprovalTime`); only the redirection, triple-brace, and data-token rows fail grammar
        - changed `claudine/lib/src/composition/sequence/preflight/tests/scaling.rs`: `static_parses(raw, when, derived)` parses the guard and builds the scope before the counter snapshot and returns `StaticAnalysis { parses, occurrences: (text, late roots, proven), timings: Option<labels> }`; deferred holes the guard proves present are retimed `Guarded`, as in `prepare_site`; `timings` is `None` when `parse_authored_prepared` fails
        - changed the table: R1-F1.2 split into "deferred and guarded holes" (`echo {{ seq.outputs[0] }} {{ seq.outputs[1] }} {{ a }}` under `seq.outputs[0]`, 3 parses, timings guarded:seq / deferred:seq / approval) plus an unguarded control (deferred:seq ×2, approval); R1-F1.3 `echo {{ seq.outputs[0] }} > out` under the guard (1 parse, grammar fails, fallback root `seq`, proven); R1-F1.5 `echo {{ seq.outputs[0] }} {{ b + }}` (2 parses, root `seq` in occurrences and timing deferred:seq); clean rows keep their counts and now also assert no late roots/no proof
        - no production change; R1-F1.9 and R1-F1.10 untouched (clean)
        - verified: `just test static_analysis_parses_each_occurrence_once` → 1 passed
        - verified: `just test scaling_tests::` → 14 passed (incl. `scaling_tests::counted::static_analysis_parses_each_occurrence_once`)
        - verified: `just test composition::sequence::preflight::tests::` → 230 passed
        - verified: `just lint` → exit 0; `just lint` does not enable `work-counters`, so also ran `cargo clippy -p claudine --lib --tests --features work-counters --keep-going -- -D warnings`: lib-test target (scaling.rs) clean; two pre-existing `clippy::result_large_err` errors in the untouched `claudine/lib/tests/l1/invalid_trigger_findings.rs` (lines 238, 256), visible only with `work-counters`
- work completed for 'R1-F1 parse-count late/guarded cases' at 18:57:25
- starting the work on 'R1-F2 schema validators and refinements through sequence preflight' at 18:57:25
        - defect class: helper-level type parity leaves the host's schema-and-condition wiring without a result-level regression test
        - discovered: `fill_template` with a closure evaluator distinguishes the two restrictions by `RejectReason`: a value the schema refuses is `DeclaredType`, a value the declaration accepts but the `when:` refinement refuses is `GuardRefinement`
        - discovered (R1-F2.2, clean): every collected site reaches the one body `prepare_site` (mod.rs:1830); task `setup:`/`teardown:` stacks (and their `on_error` companions, from `shape::collect_stack_shell_sites`, mod.rs:1786) are prepared under a scope with `guard: None` (mod.rs:1783), container events (`containers.rs:23`) likewise; the schema is chosen and projected inside that body (`owner_schema` mod.rs:1900, `site_types` mod.rs:1904, `project_types_prepared` mod.rs:1911), so stacks get the validator and never the task condition
        - discovered (R1-F2.3, clean): a whole-value `set` enters the same body as `ShellSiteKind::SetValue` (mod.rs:1857, from `collect_lifecycle_shell` and `containers.rs:46`); its extracted inner span becomes `authored_view = prepared.view(span)` (mod.rs:1891), which feeds the same projection; existing coordinate tests unchanged
        - R1-F2.4 (clean): Darkmatter `projected_types_match_with_schema_validators_and_refinements` left as is
        - changed `claudine/lib/src/composition/sequence/preflight/tests.rs`: new `schema_typed_holes` module; fixture `seq.md` with `$schema` `count: number(min(1))`, `mode: enum(release, debug)`, `count: 2`, `mode: release`, a root `finalize` `set: { stamp: "$(stamp {{ count }})" }`, and `tool {{ count }} {{ mode }}` as task 2 (after `echo first`, so it is a template candidate) with or without `when: "mode == 'release'"`; templates are taken from `graph.template_candidates` by authored command and filled via `fill_template`
        - tests: `a_later_task_keeps_its_schema_validator_and_guard_refinement` (2/release accepted; 0 and "2" rejected `DeclaredType` for `count`; debug rejected `GuardRefinement` for `mode`), `without_a_guard_the_declared_enum_accepts_every_member` (control: debug accepted, nightly rejected `DeclaredType`), `a_container_set_command_keeps_the_schema_validator` (container `set` command: 2 accepted, 0 rejected; no refinement)
        - no production change; no new `TypeScope` construction (no allowlist entry)
        - mutation check (restored from a backup copy, `git diff` on mod.rs empty afterwards): `site_types(&declared, None)` fails the guarded test only; `DeclaredTypes::for_schema(None)` fails all three
        - verified: `just test schema_typed_holes::` → 3 passed
        - verified: `just test composition::sequence::preflight::tests::` → 233 passed
        - verified: `just test type_scope_ownership_guard` → 8 passed
        - verified: `just lint` → exit 0
- work completed for 'R1-F2 schema validators and refinements through sequence preflight' at 19:01:55

### Completion Report

- **outcome:** complete; elapsed about 19 minutes (18:49:52 → 19:08:30)
- **parent findings:** total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - child rows: 4 fixed (R1-F1.2, R1-F1.3, R1-F1.5, R1-F2.1) and 10 clean (R1-F1.1, .4, .6–.10, R1-F2.2–.4); R1-F1.2 also gained an unguarded control row in the test table
- **verified repairs**
        - R1-F1: `static_parses` now builds the guarded scope with production's `TaskGuard::parse` + `site_types` before counting and returns the analysis outcome; the late rows read the declared late root `seq`, and the guarded row is proven by `when: seq.outputs[0]`
        - R1-F2: `preflight/tests.rs::schema_typed_holes` fills the returned graph's templates to show that the `number(min(1))` validator and the `mode == 'release'` refinement survive `prepare_site`, with an unguarded control and a container `set` site; a temporary mutation of `prepare_site` (guard dropped / schema dropped) failed the new tests and was reverted
- **open findings:** none
- **changed files**
        - `claudine/lib/src/composition/sequence/preflight/tests/scaling.rs` (R1-F1)
        - `claudine/lib/src/composition/sequence/preflight/tests.rs` (R1-F2)
        - no production code, docs, or Darkmatter files changed in this cycle
- **final verification** (macOS, after all changes, no subagent running)
        - `claudine/`: `just test` → 11,792 passed, 15 skipped, exit 0 (R1-F1, R1-F2)
        - `claudine/`: `just lint` → exit 0
        - root: `just check-tier-coverage claudine` → 0 stranded
        - `darkmatter/` gates not rerun: no Darkmatter file changed in this cycle
        - no cross-OS check: test-only change with no path or OS-specific code
- **unrelated, noted:** `cargo clippy --features work-counters` reports two `clippy::result_large_err` errors in `claudine/lib/tests/l1/invalid_trigger_findings.rs` (lines 238, 256), from commit `3b3a7f9ef`, which predates this feature; `just lint` does not enable that feature and passes
