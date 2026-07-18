---
agent: codex/
total_phases: 9
created: 2026-07-17
phase: 1
yolo: true
---

# Better Metrics Execution Plan

This plan implements the durable performance-measurement platform described in
`spec.md`. It deliberately excludes performance optimizations, public compose
metrics API changes, benchmark-target renames, CI infrastructure, dashboards,
and rewrites of historical evidence.

The implementation has two tracks after preflight. The area-level benchmark
platform is the critical path (Phases 2, 4, 5, and 6). The CLI-private `--perf`
correction (Phase 3) can proceed in parallel with Phase 2, but both tracks must
land before the acceptance replay in Phase 8.

## Phase 1 — Freeze Scope, Blast Radius, and Acceptance Evidence

**Depends on:** Nothing.

**Parallelization:** Discovery commands may run concurrently. No source symbol
may be edited until its individual upstream impact result has been reviewed.

### Tasks

- [ ] 1.1 Run `sniff repo packages` from the repository root and record the affected package areas, packages, and expected downstream consumers in a new feature evidence index at `darkmatter/features/2026-07-16-better-metrics/results.md`. Start with `darkmatter` and `darkmatter-cli`; add `biscuit-terminal`, `sniff`, `biscuit-file`, or `biscuit-hash` to the implementation gate only if their source is changed rather than consumed through an existing API.
- [ ] 1.2 Refresh the GitNexus index if possible, then run upstream impact analysis on every symbol proposed for modification. At minimum, analyze `run_compose`, `format_compose_perf_report`, `CliComposePerfReport`, `benchmark_manifest_matches_recorded_identities`, `Harness`, `Harness::interleaved_pair`, and `fixture_text`; record direct callers, affected processes, confidence, and risk. Stop and warn before editing any HIGH or CRITICAL target.
- [ ] 1.3 Inventory the current performance surface and record it in `results.md`: all 15 `[[bench]]` targets from `darkmatter/lib/Cargo.toml`, the seven area `just bench*` recipes, every retained crate-private harness test using `perf_harness`, the 13 committed fixtures, the feature-local generator/manifest/recompute tool, and existing raw run-record conventions.
- [ ] 1.4 Build an acceptance traceability table mapping specification acceptance criteria 1–12 and each verification bullet to an owning phase, artifact, and observable test or run record. Mark the command-level replay of `db7e46792` versus `b425fb466` as the release blocker.
- [ ] 1.5 Record the immutable guardrails in `results.md`: no optimization; no compose/render/hash/validation behavior change; no public `ComposePerfReport`, `ComposePerfMetric`, or `ComposeStage` shape change; no phase-named benchmark rename; no write-mode formatter; no historical result rewrite; and no automatic baseline refresh.

### Validation checkpoint

- [ ] 1.V1 Confirm the recorded scope names every source file expected to change, every direct downstream consumer found by GitNexus, and the exact package-area gates that will be run. Resolve any HIGH/CRITICAL risk or unexplained package before Phase 2 or Phase 3 begins.
- [ ] 1.V2 Confirm the traceability table has an observable proof for all 12 acceptance criteria, including equivalent output, invalid-run retention, same-session baseline rebuilding, three-OS smoke evidence, and the required failing historical replay.

## Phase 2 — Promote the Durable Evidence Home and Define Its Schema

**Depends on:** Phase 1.

**Parallelization:** May run in parallel with Phase 3. Fixture copying, profile
authoring, and README schema documentation can be split once the manifest field
names are frozen.

### Tasks

- [ ] 2.1 Create the area-owned `darkmatter/benchmarks/` layout with `README.md`, `generate.sh`, `manifest.yaml`, `fixtures/`, `profiles/`, `baselines/`, `run-command.ts`, `recompute.ts`, and `raw/`. Preserve the feature-local `2026-07-15-performance-followup/benchmarks/` directory as read-only historical evidence.
- [ ] 2.2 Copy the existing generator, recomputation tool, and all 13 fixture bytes into the area home exactly once. Record source and destination byte counts, Darkmatter frontmatter/body hashes, combined `md hash` identities, and `biscuit-hash` xxHash64 values; fail promotion if any source/destination identity differs.
- [ ] 2.3 Extend `manifest.yaml` into the single versioned authority for generators, fixtures, execution profiles, workloads, and the benchmark catalog. Define stable IDs and explicit references rather than inferring relationships from filenames or bench code.
- [ ] 2.4 Define every execution-profile field required by AD-1: runner, boundary, argv, logical working-directory fixture/anchor reference, stdin/TTY and output mode, environment allowlist and overrides, context policy, trigger-root policy, cache state, network/shell/prompt policy, expected exit status, and output-identity policy.
- [ ] 2.5 Add committed JSON profiles for `micro-minimal`, `micro-cli-frozen`, `command-cli-default`, and `command-cli-stateful`. Give `micro-minimal` empty context and `ComposeOptions::new()` semantics; give `micro-cli-frozen` the Darkmatter baseline schema, a committed trigger registry, and a frozen representative context; keep both command profiles on ordinary live demand-driven capture, with `command-cli-stateful` adding public `--state` and `--set` inputs.
- [ ] 2.6 Add a deterministic workspace-root fixture used by both command binaries, including any trigger schemas, transclusion children, state/set inputs, and cache seed required by the profiles. Keep checkout-absolute paths out of committed identity.
- [ ] 2.7 Define workload entries that bind exactly one fixture/root, one execution profile, and one boundary (`component`, `library-operation`, or `command`). Require authoritative workloads to declare relative and absolute budgets, maximum dispersion and bracket drift, warm-up/sample minima, confidence rule, effects policy, expected output identity, owner, and authority status.
- [ ] 2.8 Register `compose_trivial × command-cli-default` as the initial authoritative command workload with 5% and 1 ms budgets, and register corresponding `micro-minimal` and `micro-cli-frozen` diagnostic workloads without presenting them as command gates.
- [ ] 2.9 Record each profile file's exact byte count and `biscuit-hash` xxHash64 over committed bytes. Do not reuse `compose_cache_fingerprint`, `ReferenceGraphOptionsIdentity`, or any other production implementation identity.
- [ ] 2.10 Document the area-level schema, boundary selection rule, output-equivalence rule, effects-deny defaults, immutable fixture policy, dated run-record shape, invalid/blocked/pass/fail states, raw-vector retention, and explicit baseline-refresh policy in `darkmatter/benchmarks/README.md`.

### Validation checkpoint

- [ ] 2.V1 Run the promoted generator twice in clean temporary directories and prove byte-for-byte deterministic output with LF line endings on every fixture.
- [ ] 2.V2 Verify every profile and workload is reachable from the manifest, every logical path resolves inside the declared fixture root, duplicate IDs are absent, and command profiles deny network, shell execution, and prompts by default.
- [ ] 2.V3 Verify the historical feature-local manifest, fixtures, raw observations, summaries, and results have no content changes; only later cross-links may point to the new area home.

## Phase 3 — Correct the CLI-Private `--perf` Structural Tree

**Depends on:** Phase 1.

**Parallelization:** May run in parallel with Phase 2. It must merge before
Phase 8. This phase must not change the public flat compose-performance types.

### Tasks

- [ ] 3.1 Re-run exact-symbol GitNexus impact analysis immediately before editing `darkmatter/cli/src/commands/compose.rs`, and record the result. Preserve `ComposePerfReport`, `ComposePerfMetric`, and `ComposeStage` source compatibility and keep Claudine outside the edit scope.
- [ ] 3.2 Replace the inclusive CLI-private `build options` duration with `prepare options` self time accumulated from non-overlapping pre-validation and post-validation spans. Ensure `validate references` is measured exactly once and excluded from `prepare options`.
- [ ] 3.3 Measure the full `run_compose` envelope with non-overlapping structural children in this order: `resolve input`, `load input`, `capture context`, `prepare options`, `validate references`, `compose pipeline`, and `emit output and diagnostics`. Compute `unattributed` as the envelope duration saturating minus the structural-child sum.
- [ ] 3.4 Keep context-group timings beneath `capture context` and public compose-stage metrics beneath `compose pipeline` as explicitly labeled breakdown nodes. Exclude all breakdown values from structural reconciliation because they may overlap, execute concurrently, or include merged child work.
- [ ] 3.5 Replace the hand-built `TwoColumn`/`BlockQuote` perf presentation with a CLI-private projection to `biscuit_terminal::MetricsTree`, then render it through `TerminalRenderable` using the per-invocation detected `Terminal`. Preserve stderr ordering after normal output and diagnostics and allow the component's normal Unicode/color degradation.
- [ ] 3.6 Update behavior-adjacent comments and tests so none describe the removed inclusive `build options` meaning. Do not alter unrelated formatting or compose behavior.
- [ ] 3.7 Add focused unit tests for structural reconciliation, `prepare options` exclusion, zero/unattributed cases, breakdown labeling, call counts, and rendering with and without a compose-stage report. Add an integration assertion that `--perf` leaves stdout content identity unchanged and emits the structural report only to stderr.

### Validation checkpoint

- [ ] 3.V1 Run the targeted `darkmatter-cli` Nextest selection for compose command tests and a release `md compose --perf` smoke over `compose_trivial`; assert structural children never exceed the command envelope and `unattributed` is non-negative.
- [ ] 3.V2 Hash stdout from otherwise identical `md compose` runs with and without `--perf` using Darkmatter's Markdown-aware hasher and require equal frontmatter/body identities and equal exit status.
- [ ] 3.V3 Confirm the diff contains no changes to `darkmatter/lib/src/markdown/compose/perf.rs` public type definitions and no Claudine source changes.

## Phase 4 — Implement Shared Profile Construction and Manifest Enforcement

**Depends on:** Phase 2.

**Parallelization:** The Rust profile-support module and manifest-guard test can
be implemented in parallel after the serialized schema is frozen. Bench-target
wiring follows both.

### Tasks

- [ ] 4.1 Add a non-production Rust support module under `darkmatter/benchmarks/support/` that deserializes the area manifest/profile schema and owns construction of `micro-minimal` and `micro-cli-frozen`. Include it from Criterion targets and the manifest guard without widening production visibility or adding a public library API.
- [ ] 4.2 Resolve committed logical file references through `biscuit_file::FileReference` relative to the declared benchmark root. Keep absolute checkout paths as per-run facts and never serialize them into profile or workload identity.
- [ ] 4.3 Construct the frozen CLI-shaped micro options from the committed Darkmatter baseline schema, trigger registry, context value, source path, cache policy, and effects policy in one authority. Remove equivalent ad hoc near-default option construction from authoritative bench cases while retaining explicitly diagnostic `micro-minimal` cases.
- [ ] 4.4 Repoint `darkmatter/lib/src/perf_harness.rs::fixture_text` and every authoritative Criterion fixture loader to the area manifest/workload authority. Retain ignored, environment-gated crate-private harnesses and their raw sample-vector format.
- [ ] 4.5 Expand `darkmatter/lib/tests/benchmark_fixtures.rs` into the area-manifest guard. Validate fixture and profile identities, generator version, stable ordering, unique IDs, all references, boundary/profile compatibility, required budgets/confidence/output/effects fields, explicit side-effect exceptions, and catalog completeness.
- [ ] 4.6 Make the guard reject an authoritative benchmark or command case that lacks a workload/profile declaration, a profile file without recorded identity, a missing fixture/root, an undeclared cache/context policy, or a command profile that enables network, shell, or prompts without a workload-level exception.
- [ ] 4.7 Add deterministic tests proving frozen micro-profile serialization and identity are identical across clones and a fresh subprocess. Separately prove command-profile recipe bytes are stable while live resolved context group names and redacted shapes are captured only as run facts.
- [ ] 4.8 Update the 15 Criterion targets only as required to select declared workload IDs and the shared micro constructor. Do not rename phase-named targets or rewrite their historical command references.

### Validation checkpoint

- [ ] 4.V1 Run the manifest guard in normal mode and add negative fixtures/tests for changed fixture bytes, changed profile bytes, duplicate IDs, missing references, incomplete authoritative workload declarations, and undeclared effects.
- [ ] 4.V2 Run the relevant Criterion targets in compile/smoke mode and the ignored harness selection with a temporary `DM_PERF_RAW_DIR`; prove both resolve the same workload/profile contract and emit retained observation vectors.
- [ ] 4.V3 Confirm no production item visibility was widened and the only production-library edit, if any, is additive measurement/profile plumbing permitted by invariant 1.

## Phase 5 — Build the Portable Runner, Statistics, Fitness, and Run Records

**Depends on:** Phase 4.

**Parallelization:** Pure statistics/fitness tests, host-fingerprint capture,
and portable process orchestration may be implemented concurrently, then joined
behind one run-record writer.

### Tasks

- [ ] 5.1 Implement `darkmatter/benchmarks/run-command.ts` as a non-interactive macOS/Linux/Windows runner that loads one declared command workload, materializes its workspace fixture into one temporary root, and launches child processes with explicit argv arrays. Invoke `hyperfine` with `--shell=none`; do not use shell pipelines, redirection, `/dev/null`, or platform-specific null-device assumptions.
- [ ] 5.2 Resolve profile file and working-directory anchor references outside timed samples through the existing `bf reference` command (which delegates to `FileReference`), derive a directory from the declared anchor where needed, check that results remain inside the declared root, and record resolved absolute paths only in the run record.
- [ ] 5.3 Enforce each profile's exact working directory, stdin/TTY mode, output mode, environment allowlist/overrides (`NO_COLOR`, locale, and timezone included), context policy, trigger root, cache state, expected exit status, and network/shell/prompt denial before timing begins.
- [ ] 5.4 Run each implementation once outside timed samples, capture exit status and output, and require equivalence before invoking `hyperfine`. Hash Markdown with Darkmatter frontmatter/body hashing and all other output with `biscuit-hash`; record both hashes and abort as a correctness failure on mismatch.
- [ ] 5.5 Extend `recompute.ts` with one reusable pure evaluator for raw Criterion/harness/hyperfine vectors: per-observation normalization, mean, median, standard deviation/dispersion, min/max, bootstrap 95% intervals, candidate bracket drift, fitness state, and final budget/confidence verdict. Make the runner and offline recomputation use the same evaluator.
- [ ] 5.6 Implement portable same-session fitness checks based on warm-up stabilization, maximum sample dispersion, pre/post calibration, and admissible drift. Accept injected probe results so fit, unfit, excessive-drift, failed-calibration, and unsupported-supplementary-signal cases are deterministic tests.
- [ ] 5.7 Capture the static host fingerprint through `sniff os --json` and `sniff hardware --json`: OS, architecture, CPU identity, logical core count, and memory. Record portable supplementary signals only if `sniff` exposes them; do not add hand-rolled load-average, active-core, or thermal probes and do not fail because those optional signals are absent.
- [ ] 5.8 Write the common dated run record under `raw/<checkpoint>/<run-id>/`, including manifest/workload/profile identities, commits, commands, host/toolchain facts, redacted environment/context shape, cache/TTY state, warm-up and sample counts, thresholds declared before capture, raw observations, output identities, calibration/dispersion/drift values, and one of `invalid`, `blocked`, `pass`, or `fail`.
- [ ] 5.9 Add a run-record validator that rejects missing raw observations, missing fitness verdicts, undeclared thresholds, unresolved identities, or pass/fail verdicts on invalid data. Retain invalid observations and diagnostics, but make them inadmissible as performance evidence.

### Validation checkpoint

- [ ] 5.V1 Run deterministic unit tests over synthetic observation vectors for fit, unfit, excessive dispersion, excessive drift, overlapping confidence intervals, unsupported optional signals, and malformed run records.
- [ ] 5.V2 Execute a candidate-only local smoke twice with the same command recipe and prove profile identity is unchanged, live context facts are separately recorded, temporary-root discovery is independent of the source worktree, and raw statistics recompute byte-for-byte from the saved vectors.
- [ ] 5.V3 Force an output mismatch and a noisy/failed calibration in controlled tests; require the first to abort before timing and the second to retain an `invalid` run without a pass/fail verdict.

## Phase 6 — Add Same-Session Baselines and Hard-Gate Verdicts

**Depends on:** Phase 5.

**Parallelization:** Baseline descriptor validation and synthetic verdict tests
can proceed in parallel. Build orchestration depends on both.

### Tasks

- [ ] 6.1 Define committed baseline descriptors under `darkmatter/benchmarks/baselines/` that name the known-good Git revision, workload ID, profile identity/revision, advisory historical observations, and owner-approved refresh rationale. Treat committed absolute medians as trend data only.
- [ ] 6.2 Extend the runner to materialize the declared known-good revision and current candidate in isolated checkout and `CARGO_TARGET_DIR` locations, build release binaries non-interactively, and record commits, candidate dirty-diff xxHash identity, Cargo lockfile identities, toolchain, build commands/logs, and binary hashes.
- [ ] 6.3 Treat a baseline checkout/build failure as `blocked`. Never advance or rewrite the baseline automatically, and never convert a blocked comparison into permission to use a historical absolute median as a hard gate.
- [ ] 6.4 Run one same-session bracket in the order candidate A, baseline, candidate B against the same absolute binaries, temporary workspace root, working directory, environment, cache state, and output contract. Preserve all three raw observation arms.
- [ ] 6.5 Compute `B` as the baseline median, `C` as the lower candidate median, and measured drift as the absolute difference between candidate medians. Fail only when `C - B` exceeds `max(relative_budget × B, absolute_budget, measured_drift)`, the baseline bootstrap interval overlaps neither candidate interval, and outputs/exits are equivalent.
- [ ] 6.6 Align retained component comparisons with the common record/verdict contract: prefer `Harness::interleaved_pair` sample-by-sample; where interleaving is impossible, require the same candidate/baseline/candidate bracket. Never use a component verdict to pass a command claim.
- [ ] 6.7 Implement an explicit owner-driven baseline refresh operation that requires a passing same-session comparison, equivalent output, a recorded reason, and updated profile/workload identities. Refuse refresh when any prerequisite is absent.
- [ ] 6.8 Add deterministic verdict tests for below-budget movement, relative-only and absolute-only movement, a regression masked by measured drift, overlapping confidence intervals, output mismatch, invalid fitness, baseline build failure, and the known +3.3 ms / approximately +31% command regression shape.

### Validation checkpoint

- [ ] 6.V1 Run a same-commit self-comparison through the full release-build bracket and require a valid non-failing verdict with equivalent output and retained A/B/A vectors.
- [ ] 6.V2 Feed a synthetic, tightly distributed +3.3 ms command regression through the 5%/1 ms workload and require a hard failure; then increase bracket drift beyond the delta and require an invalid/inadmissible result rather than a false pass or fail.
- [ ] 6.V3 Attempt a baseline refresh without a reason, without equivalent output, and without a passing comparison; require all three attempts to fail without changing committed baseline metadata.

## Phase 7 — Catalog Existing Measurements and Publish the Operating Contract

**Depends on:** Phase 4.

**Parallelization:** May run in parallel with Phases 5 and 6. Final wording and
commands wait for the runner interface to stabilize.

### Tasks

- [ ] 7.1 Catalog all 15 Criterion targets and every retained crate-private harness in `manifest.yaml`, recording owner, boundary, workload IDs, entry point, and one status from `authoritative`, `diagnostic`, `redundant`, or `historical`.
- [ ] 7.2 Add a completeness test that compares the catalog with registered `[[bench]]` targets and known retained harnesses, failing on uncataloged additions or stale catalog entries. Keep `phase6_*`, `phase8_*`, `phase9_*`, and `phase10_*` names unchanged.
- [ ] 7.3 Document concrete commands for manifest validation, diagnostic Criterion runs, command hard gates, offline recomputation, invalid-run inspection, and explicit baseline refresh. Preserve the rule that CLI and PTY evidence do not run through `just bench`, and verify all seven existing `just bench*` recipes remain functional.
- [ ] 7.4 Add cross-links from the 2026-07-15 performance-followup README/results to the area platform without rewriting old paths, commands, thresholds, observations, or verdicts.
- [ ] 7.5 Update `darkmatter/docs/dependencies.md` only if dependency declarations change, and update the `darkmatter` skill with the durable benchmark home, workload/profile boundary rules, and validation commands because this feature changes the area workflow.
- [ ] 7.6 Maintain `darkmatter/features/2026-07-16-better-metrics/results.md` as the evidence index for platform tests, smoke records, replay results, deviations, and final acceptance-criteria dispositions.

### Validation checkpoint

- [ ] 7.V1 Require catalog completeness over all 15 registered Criterion targets and retained in-crate harnesses, with exactly one owner/boundary/status classification per entry.
- [ ] 7.V2 Follow the README from a clean checkout to validate the manifest, run one diagnostic micro workload, run one command self-comparison, and recompute the saved record without undocumented setup.
- [ ] 7.V3 Review the historical feature diff and confirm it contains cross-links only; no historical evidence content or interpretation has changed.

## Phase 8 — Replay the Escaped Regression and Prove Cross-Platform Operation

**Depends on:** Phases 3, 6, and 7.

**Parallelization:** macOS, Linux, and Windows smoke captures may run in
parallel on separate hosts/VMs. The historical hard-gate replay must use one
same-host, same-session bracket and is not parallelized across hosts.

### Tasks

- [ ] 8.1 Produce real command-runner and fitness smoke records on macOS, Linux, and Windows using the same committed workload/profile identities. Verify platform-specific process launching, temporary paths, argv handling, environment setup, `hyperfine --shell=none`, raw record shape, and `sniff` fingerprint capture.
- [ ] 8.2 On a host that passes the fitness gate, use the authoritative runner to build and compare `db7e46792` (pre-opacity) against `b425fb466` (post-opacity) for `compose_trivial × command-cli-default` in one candidate A → baseline → candidate B session.
- [ ] 8.3 Require equal exit status and Darkmatter output identity for all replay arms, retain raw vectors and bootstrap intervals, and apply the predeclared 5%/1 ms plus measured-drift rule. The authoritative command verdict must be `fail`; if it is not, stop closure and revise the platform design rather than tuning the budget after capture.
- [ ] 8.4 Reproduce the corresponding `micro-minimal` component comparison with byte-identical current bench/support code staged into both isolated revisions when necessary. Record any non-production overlay hashes and require it to show the much smaller delta that the original component threshold allowed.
- [ ] 8.5 Explain the two simultaneous results in `results.md`: the component measurement is valid for mechanism attribution, the command measurement is valid for the user-visible `md compose` claim, and only the latter may gate that claim.
- [ ] 8.6 Confirm `--perf` structural rows reconcile on the replayed command while context and compose-stage details remain labeled breakdowns and are not double-counted.

### Validation checkpoint

- [ ] 8.V1 Confirm all three OS smoke records are valid, contain raw observations and host fingerprints, and can be recomputed with the committed tool.
- [ ] 8.V2 Confirm the command replay is a valid hard failure with equivalent output, non-overlapping confidence intervals, and a delta larger than the relative budget, absolute budget, and measured drift.
- [ ] 8.V3 Confirm the micro result and command result cite different declared boundaries and that no component-only evidence is presented as passing or failing the CLI claim.
- [ ] 8.V4 Inspect the implementation diff and evidence records for optimization attempts; acceptance requires none.

## Phase 9 — Run Scoped Gates and Close the Feature

**Depends on:** Phase 8.

**Parallelization:** Documentation review and scoped build/test/lint gates may
run concurrently once the final changed-file set is stable. GitNexus change
detection follows all edits.

### Tasks

- [ ] 9.1 Re-run `sniff repo packages` and GitNexus upstream impact analysis over the final changed symbols, then record the actual affected packages, package areas, downstream consumers, and execution flows. Reconcile any difference from Phase 1 before choosing final gates.
- [ ] 9.2 Run `just build`, `just test`, and `just lint` from the Darkmatter package area for `darkmatter` and `darkmatter-cli`; run the corresponding area gates for `biscuit-terminal`, `sniff`, `biscuit-file`, or `biscuit-hash` only if their source changed. Use Nextest through the `just` recipes and do not run unscoped workspace gates.
- [ ] 9.3 Run the manifest/catalog/run-record validators, the command self-comparison, and the historical replay recomputation as explicit feature gates in addition to ordinary unit/integration tests.
- [ ] 9.4 Run `git diff --check` and read-only formatting diagnostics if needed. Do not run `cargo fmt` or `rustfmt` in write mode.
- [ ] 9.5 Run GitNexus `detect_changes({ scope: "compare", base_ref: "main" })` and record the affected symbols and flows. Require the result to show no compose/render/hash/validation behavior change beyond additive measurement support and the approved CLI-private timing-envelope correction.
- [ ] 9.6 Audit every touched `///`, `//!`, and inline comment for behavioral drift, remove stale descriptions of feature-local authority or inclusive `build options`, and avoid narration that merely restates implementation.
- [ ] 9.7 Mark every acceptance criterion in the Phase 1 traceability table pass/fail with a direct test or run-record link. Closure requires criteria 1–12 to pass, including the intentionally failing regression gate and the no-optimization invariant.
- [ ] 9.8 Update the specification/result status and move the feature directory to `_completed` only after all evidence and gates pass. Do not create a Git commit unless separately requested.

### Validation checkpoint

- [ ] 9.V1 Confirm all scoped build/test/lint gates, `git diff --check`, manifest/catalog/run-record validation, and GitNexus change detection pass with recorded output.
- [ ] 9.V2 Confirm the final diff contains the area platform, profile/workload enforcement, portable runner, baseline/fitness logic, CLI-private `MetricsTree` correction, catalog/docs, and evidence records—and contains no optimization, public compose metrics migration, benchmark rename, historical rewrite, or write-mode formatter churn.
- [ ] 9.V3 Confirm `results.md` links valid macOS/Linux/Windows smoke records and the authoritative historical replay whose command gate fails for the intended regression while the diagnostic micro result remains boundary-limited.
