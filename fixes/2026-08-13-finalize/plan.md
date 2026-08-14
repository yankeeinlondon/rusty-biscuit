---
total_phases: 6
created: 2026-08-13
phase: 6
agent: codex/default
yolo: "true"
source_files_during_phase_1:
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
docs_updated_during_phase_1:
  - fixes/2026-08-13-finalize/plan.md
  - fixes/2026-08-13-finalize/spec.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - Cargo.lock
  - claudine/cli/Cargo.toml
  - claudine/cli/tests/ctx_launch_anchor_baseline.rs
  - claudine/cli/tests/shipped_prompt_contract.rs
  - claudine/lib/Cargo.toml
  - claudine/lib/src/invocation_context/tests.rs
  - claudine/lib/src/system_prompt/context.rs
  - claudine/lib/src/system_prompt/prepare.rs
  - claudine/lib/src/system_prompt/prepare/tests.rs
  - claudine/lib/src/system_prompt/types.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/strings.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
  - darkmatter/lib/src/markdown/compose/tests/literal_interpolation.rs
  - darkmatter/lib/src/markdown/compose/tests/mod.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
  - darkmatter/lib/src/markdown/compose/tests/shell.rs
  - darkmatter/lib/tests/compose_phase6.rs
  - darkmatter/lib/tests/predict_conflicts.rs
docs_updated_during_phase_2:
  - claudine/docs/dependencies.md
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/schemas/expression-functions.yaml
  - darkmatter/docs/topics/darkmatter-expressions.md
  - fixes/2026-08-13-finalize/plan.md
  - prompts/code-comment-quality.md
  - prompts/context.md
  - prompts/faster-builds-and-tests.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_3:
  - .github/workflows/_package-ci.yml
  - claudine/cli/Cargo.toml
  - claudine/cli/examples/ctx_launch_anchor_provider_fixture.rs
  - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
  - claudine/cli/tests/ctx_launch_anchor_baseline.rs
  - claudine/justfile
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs_updated_during_phase_3:
  - .github/ci/README.md
  - fixes/2026-08-13-finalize/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/perf/mod.rs
  - claudine/cli/src/perf/report.rs
  - claudine/cli/src/perf/tests/perf_tree.rs
  - claudine/cli/src/perf/tests/report.rs
  - claudine/cli/src/perf/tree.rs
  - claudine/cli/tests/wrap_perf.rs
  - claudine/lib/src/invocation_context.rs
  - claudine/lib/src/invocation_context/tests.rs
  - claudine/lib/src/system_prompt/prepare.rs
  - claudine/lib/src/system_prompt/prepare/tests.rs
docs_updated_during_phase_4:
  - claudine/README.md
  - claudine/docs/topics/performance-testing.md
  - fixes/2026-08-13-finalize/plan.md
docs_created_during_phase_4:
  - fixes/2026-08-13-finalize/latency.md
skills_files_updated_during_phase_4: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - fixes/2026-08-13-finalize/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
  - .github/workflows/_package-ci.yml
  - Cargo.lock
  - claudine/cli/Cargo.toml
  - claudine/cli/examples/ctx_launch_anchor_provider_fixture.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
  - claudine/cli/src/perf/mod.rs
  - claudine/cli/src/perf/report.rs
  - claudine/cli/src/perf/tests/perf_tree.rs
  - claudine/cli/src/perf/tests/report.rs
  - claudine/cli/src/perf/tree.rs
  - claudine/cli/tests/ctx_launch_anchor_baseline.rs
  - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
  - claudine/cli/tests/shipped_prompt_contract.rs
  - claudine/cli/tests/wrap_perf.rs
  - claudine/justfile
  - claudine/lib/Cargo.toml
  - claudine/lib/src/invocation_context.rs
  - claudine/lib/src/invocation_context/tests.rs
  - claudine/lib/src/system_prompt/context.rs
  - claudine/lib/src/system_prompt/prepare.rs
  - claudine/lib/src/system_prompt/prepare/tests.rs
  - claudine/lib/src/system_prompt/types.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/strings.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
  - darkmatter/lib/src/markdown/compose/tests/literal_interpolation.rs
  - darkmatter/lib/src/markdown/compose/tests/mod.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
  - darkmatter/lib/src/markdown/compose/tests/shell.rs
  - darkmatter/lib/tests/compose_phase6.rs
  - darkmatter/lib/tests/predict_conflicts.rs
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
documentation:
  - .github/ci/README.md
  - claudine/README.md
  - claudine/docs/dependencies.md
  - claudine/docs/topics/performance-testing.md
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/schemas/expression-functions.yaml
  - darkmatter/docs/topics/darkmatter-expressions.md
  - fixes/2026-08-13-finalize/latency.md
  - fixes/2026-08-13-finalize/plan.md
  - fixes/2026-08-13-finalize/spec.md
  - prompts/code-comment-quality.md
  - prompts/context.md
  - prompts/faster-builds-and-tests.md
packages:
  - claudine
  - claudine-cli
  - darkmatter
---

# Execution Plan: Finalize `fix/ctx-launch-anchor`

Reference: [`spec.md`](spec.md)

## Goal

Return the launch-anchor branch to a trustworthy merge verdict by correcting
the branch-owned P1-P4 failures, proving the fixes on their affected operating
systems, and dispositioning the independently verified main-drift cells without
allowing a cell-wide baseline to hide a new test identity.

## Completion contract

The work is complete when the raw/effective context distinction remains intact,
the shipped prompt fixture and hash pin match reviewed source, interpolated
values survive CommonMark as the intended literal text, projected Windows paths
never expose a verbatim prefix, native provider fixtures transport multiline
arguments and stdin correctly, the real-composition tests fit the CI budget for
an explained reason, and the final CI identity diff contains no regression
relative to run `31651014023`.

## Governing decisions and constraints

The plan proceeds with the specification's recommended decisions, but Phase 1
must ratify them before their dependent changes begin:

| Decision | Planned choice | Enforcement point |
|---|---|---|
| Body interpolation semantics | Option A: literal text by default with an explicit `raw_markdown(value)` escape hatch | Phase 1 corpus audit and decision record |
| Main-drift disposition | Option A: short-lived cell baselines, expiring `2026-09-30` | Phase 5, only after P1-P4 are green |

Do not change `ComposeContext::get()`, recapture launch evidence to apply target
identity, globally rewrite backslashes, raise the global nextest timeout, add a
second production Claudine binary, or absorb the P5-P7 product fixes listed as
out of scope in the specification. Tests must not mutate process-global
`AGENT`/`MODEL`, compile fixtures at test runtime, or focus terminal/browser
windows.

## Dependency and parallelization map

```text
Phase 1: decisions + F1/F2
    ├── Phase 2: F3/F4 product behavior ─┐
    ├── Phase 3: F5/F6 Windows tests ────┼── native-Windows checkpoint
    └── Phase 4: F7 latency ─────────────┘   + two Ubuntu proofs
                                              ↓
Phase 5: conditional F8 gate policy
                                              ↓
Phase 6: full acceptance and identity diff
```

Phases 2, 3, and 4 are parallelizable after Phase 1. Within Phase 2, the
Darkmatter interpolation track and Claudine path-projection track are also
parallelizable. Phase 5 is blocked until all branch-owned P1-P4 identities are
green; Phase 6 waits for both the Windows and latency checkpoints.

## Phase 1 — Ratify contracts and clear cross-platform blockers

**Objective:** settle both open questions and remove the deterministic P1/P2
failures without altering launch capture semantics.

### Tasks

- [x] Reconfirm the branch-owned failure ledger against branch run
  `31651014023` and main run `31588186544`: P1-P4 remain implementation scope,
  while every P5-P7 identity remains a policy or handoff item only.

- [x] **[Parallelizable]** Audit shipped prompts and representative downstream
  documents for body expressions that intentionally inject Markdown structure;
  record every required `raw_markdown(...)` migration and distinguish those
  sites from ordinary scalar/path interpolation.

- [x] Ratify Open Question 1 in [`spec.md`](spec.md). Select Option A unless the
  corpus audit demonstrates a prohibitive compatibility surface; if it does,
  stop F3, document the complete path grammar and tests required by Option B,
  and obtain a revised contract before implementation.

- [x] Ratify Open Question 2 in [`spec.md`](spec.md), including the planned
  `2026-09-30` expiration and the rule that a cell without identity-level diff
  evidence must follow Option B instead of being baselined.

- [x] Correct
  `stabilized_reread_extends_one_launch_epoch_without_reanchoring_identity` in
  `claudine/cli/src/commands/wrap/harness_orch/prompt.rs`: read `agent` and
  `model` from `ComposeContext::as_object()` on both preparations, retain raw
  `.get()` assertions for `area` and extended `os`, and keep the expected work
  counts at one construction, one extension, and zero ambient fallbacks.

- [x] Prove the F1 test is ambient-independent without `set_var`/`remove_var`:
  run it once with host `AGENT`/`MODEL` absent and once with unrelated values,
  and verify both runs expose `codex`/`gpt-test` only through the effective
  context view.

- [x] Diff `prompts/_implement/implement-plan.md` against
  `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`;
  mirror the merged `phase:` fallback into the fixture and verify the remaining
  `iteration`-to-`phase` delta is confined to the intentionally omitted
  `success.stack` block.

- [x] Run `md hash prompts/_implement/implement-plan.md`, then execute the
  documented update-mode nextest command to refresh
  `claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json`.

- [x] Re-run `shipped_prompt_route_drift` without
  `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES`; require both the byte-drift test and
  `fixture_preserves_the_shipped_schema_and_loop_semantics` to pass, since the
  mutating update run is not verification.

### Validation checkpoint

- [x] Run the targeted P1 unit test and the normal, non-update
  `shipped_prompt_route_drift` integration test through nextest on macOS; do not
  begin dependent product work with either deterministic blocker still red.

### Phase 1 decision record

- Runs `31651014023` and `31588186544` confirm the existing failure ledger:
  P1-P4 remain branch implementation scope. P5-P7 reproduce on main or are
  already baseline-covered, so they remain Phase 5 policy or main-side handoff
  items only.
- Body interpolation uses literal-text semantics by default, with an explicit
  `raw_markdown(value)` escape hatch. The shipped-prompt audit found three body
  sites that intentionally generate Markdown and must migrate in Phase 2:
  `prompts/faster-builds-and-tests.md:17`,
  `prompts/code-comment-quality.md:18`, and `prompts/context.md:34`. Each calls
  `as_unordered_list`; representative Claudine research fleet documents use
  only ordinary scalar/path body interpolation. Frontmatter render helpers and
  fenced documentation examples are outside the body-prose contract.
- Main-drift cells may receive short-lived baselines expiring `2026-09-30` only
  when an exact identity-level diff exists. A cell without that evidence must
  follow Option B and receive its main-side fix before this branch proceeds.

## Phase 2 — Implement literal interpolation and safe path projection

**Objective:** fix the two Windows product defects while preserving directive,
code, cache-key, and authored-path contracts on all platforms.

### Track A — Darkmatter body interpolation (F3)

- [x] **[Parallelizable]** Use repository impact analysis to inventory every
  caller of Darkmatter body interpolation and its cleanup pass, with explicit
  attention to Claudine, transclusion, preflight, shell directives, inline code,
  opted-in fenced code, and the shipped prompt corpus.

- [x] Add failing OS-independent regression tests before implementation. Cover
  drive paths, UNC paths, hidden-directory segments, and every ASCII
  punctuation character CommonMark may consume after a Windows separator; make
  assertions against parsed text/events rather than serialized source spelling.

- [x] Extend interpolation scanning/rewrite metadata so each replacement knows
  whether it occupies prose, inline code, opted-in fenced/indented code, or a
  Darkmatter directive. Keep frontmatter on its existing typed/raw path and do
  not turn code or command bytes into escaped prose.

- [x] Implement literal-by-default body interpolation at the syntax-aware
  boundary: prose and inline-code values must parse back to their exact scalar
  text, while opted-in code and directive arguments preserve raw bytes. Avoid a
  global `\\` rewrite and preserve source ranges and existing typed errors.

- [x] Add and catalog the explicit `raw_markdown(value)` escape hatch, including
  expression documentation and tests that prove intentional emphasis, links,
  and other authored Markdown remain possible without making ordinary data
  syntax-active.

- [x] Lock preflight/execution parity with tests showing `::shell` sees the same
  raw command bytes during collection and execution, including a Windows-shaped
  argument; add separate cases for prose, inline code, opted-in fences,
  frontmatter, directives, and transcluded children.

- [x] Add a passive shipped-prompt corpus test that detects accidental output
  drift and migrate only the intentional-Markdown sites found in Phase 1 to
  `raw_markdown(...)`.

- [x] Add a Claudine end-to-end composition regression through the normal
  invocation path that interpolates a Windows-shaped `ctx.repo_root` and proves
  the provider receives the exact parsed path text.

- [x] Update `darkmatter/docs/inline/interpolation.md`, the expression-function
  catalog/reference, and `.claude/skills/darkmatter/compose.md` so the public
  literal/raw semantics, code-region rules, and implementation description
  agree with the code. Remove or correct any drifted source-first/single-pass
  claims encountered in those edited sections.

### Track B — Claudine projected paths (F4)

- [x] **[Parallelizable]** Promote `dunce` from a `claudine` dev-dependency to a
  production dependency in `claudine/lib/Cargo.toml`; update
  `claudine/docs/dependencies.md` to describe its projected-path role rather
  than the current test-only role.

- [x] Replace `system_prompt/context.rs::canonical_or_self` with one projected
  path helper that uses `dunce::canonicalize` for existing paths and
  `dunce::simplified` for the authored fallback. Apply it consistently to CWD,
  repository, package-area, package, and selection comparisons.

- [x] Keep `invocation_context::canonical_key` unchanged and add a regression
  that demonstrates cache equality still uses canonical keys while authored
  and prepared projections use simplified paths.

- [x] Apply the same canonicalize-then-simplify algorithm to path expectations
  in `claudine/cli/tests/ctx_launch_anchor_baseline.rs`; update behavior comments
  that currently imply direct `std::fs::canonicalize` is safe for projected
  values.

- [x] Add native-Windows drive and UNC tests asserting that `LaunchContext`,
  prepared `ctx.*`, `SystemPromptContext`, and `PreparedSystemPrompt::source`
  never begin with `\\?\`, while canonical cache-key comparisons still match.

### Validation checkpoint

- [x] Run `cd darkmatter && just test && just lint` for the OS-independent
  interpolation matrix and `cd claudine && just test && just lint` for path
  projection. Require no Unix prompt output or directive-command drift before
  the native-Windows batch in Phase 3.

## Phase 3 — Replace Windows stubs and normalize the JIT expectation

**Objective:** make the new Windows tests exercise product behavior through a
native executable fixture, then expose and resolve any remaining expectation
failures in one Windows CI round trip.

### Tasks

- [x] Add `exit /b 0` to the Windows `printf.cmd` fixture emitted by
  `stage_printf`, and add a focused assertion that expected stdout is returned
  with a zero status.

- [x] Add the non-production Cargo example
  `claudine/cli/examples/ctx_launch_anchor_provider_fixture.rs` with explicit
  Codex, Goose, and counting-Codex modes. It must capture stdin, preserve
  multiline argv boundaries, emit the generated Goose response, and update the
  loop counter without invoking a shell trampoline.

- [x] Add a Claudine just prerequisite that builds the example before L1 tests,
  add `claudine-provider-fixture` to `claudine-cli`'s `runner-tools`, and teach
  `_package-ci.yml` to build and export its deterministic path before native
  L1 execution. Update the closed vocabulary in `.github/ci/README.md`; do not
  add another `[[bin]]` to the shipped `claudine-cli` package or build the
  fixture inside a test process.

- [x] Replace `stage_windows_provider`'s PowerShell/`.cmd` trampolines by
  copying the prebuilt fixture under `codex.exe` and `goose.exe`; pass fixture
  mode through an environment variable or fixture-owned argument that never
  collides with provider argv.

- [x] Add a direct native-Windows fixture contract test proving that quotes,
  embedded newlines, separate argv elements, and stdin arrive byte-for-byte,
  and that the helper works from a nextest-built artifact with Cargo/rustc
  unavailable at runtime.

- [x] Update
  `template_preflight_combines_launch_facts_with_the_selected_target` to build
  the expected command from executable plus argument vector using
  `darkmatter::markdown::compose::shell_expansion::policy::normalize_command`;
  do not normalize the approval set again or change product quoting.

### Validation checkpoint

- [ ] Run one native `windows-latest` L1 batch for `darkmatter`, `claudine`, and
  `claudine-cli`. Require the five P3a regressions, the projected-path tests,
  all five P3c-affected `ctx_launch_anchor_baseline` tests, and the P3d JIT test
  to pass; inspect logs for any prepared value or source beginning with
  `\\?\`.

### Phase 3 validation record

- macOS targeted and package-area gates pass: the native fixture example build,
  all six applicable `ctx_launch_anchor_baseline` cases, the JIT preflight
  regression, `cd claudine && just test`, and `cd claudine && just lint`.
- CI configuration coverage passes: all 58 `affected_scope` tests and all 124
  `test-toolkit` L1 tests, including the new runner-tool workflow contract;
  `cd tools && just lint` also passes.
- `cd claudine && just check-windows` passes for the complete
  `claudine`/`claudine-cli --tests` MinGW graph, and the provider example passes
  a separate `x86_64-pc-windows-gnu` compile.
- Native execution remains pending because this worktree is running on macOS
  without a Windows runtime, and the required changes are uncommitted by
  instruction so they cannot be dispatched to `windows-latest`. The checkpoint
  stays open rather than treating cross-compilation as runtime evidence.

## Phase 4 — Measure and bound real-composition latency

**Objective:** explain the Ubuntu timeouts with per-stage evidence, then remove
the measured cost or restructure test process boundaries without weakening
their output contracts.

### Tasks

- [x] **[Parallelizable with Phases 2 and 3]** Record a reproducible timing
  baseline for each invocation in
  `shipped_implement_prompt_runs_real_router_target`,
  `compose_perf_stdout_matches_non_perf`, and
  `inline_compose_perf_stdout_matches_non_perf`, covering both a repository
  launch and a non-repository temporary `HOME`; store the measurements and
  runner characteristics in `fixes/2026-08-13-finalize/latency.md`.

- [x] Extend the existing `--perf`/`InvocationWorkSnapshot` instrumentation,
  rather than adding ad hoc user output, to time invocation capture,
  repository observation, topology initialization, launch-context capture,
  system-prompt preparation, composition preparation, and provider handoff.

- [x] Assert the per-process work contract in focused tests: one launch
  observation, one launch-context construction for one preparation epoch, no
  duplicate topology probe for one repository, and no topology probe for a
  confirmed non-repository launch. Treat separate CLI processes as separate
  invocation epochs.

- [x] If measurement finds duplicate or super-linear discovery inside one
  process, remove that production cost and add a counter/timing regression that
  fails when the extra observation returns.

- [x] If each process is bounded and only the pairwise tests exceed 90 seconds
  because they launch perf and non-perf processes serially, split each pair
  into per-mode tests that compare against the same fixed stdout fixture. Keep
  parity transitive through that shared fixture; do not merely drop the
  equality assertion.

- [x] Treat a test-specific timeout increase as a final exception only after
  `latency.md` proves one irreducible invocation exceeds 90 seconds on a
  standard two-core runner; do not change the global nextest profile.

### Phase 4 decision record

- Instrumented work counts show bounded per-process discovery. The repository
  launch plus external source performs two distinct Git observations and one
  topology probe; a confirmed non-repository launch performs one observation
  and no topology probe. No production discovery change is warranted.
- The two serial-process parity tests were split by mode. Every mode asserts the
  same fixed stdout fixture, so parity remains transitive without one test
  spending two process budgets.
- No test-specific or global timeout was increased.
- Local macOS validation passed the complete `claudine` package-area `just test`
  gate and `just lint`. The required consecutive `ubuntu-latest` evidence
  remains pending because these uncommitted changes cannot be dispatched to CI
  without the prohibited commit-and-push step.

### Validation checkpoint

- [ ] Run the router-target and perf/non-perf output contracts on
  `ubuntu-latest` twice in consecutive CI runs. Record stage timings, work
  counts, and the selected production-fix or test-restructuring rationale for
  both runs.

## Phase 5 — Disposition verified main-drift cells

**Objective:** apply the ratified gate policy only after branch-owned failures
are green, with identity-level evidence preventing a broad cell baseline from
masking new regressions.

### Entry gate

- [ ] Confirm P1-P3 are green on their macOS and Windows blocking cells and the
  P4 family has passed its first Ubuntu proof. If any branch-owned identity is
  still red, do not edit `.github/ci/ci-baseline.toml`.

### Tasks

- [ ] Diff exact failed test identities between source run `31588186544`
  (including its `wsl2-ubuntu` leg) and branch run `31651014023`; associate each
  proposed cell with the P5/P6 failure family documented in `problems.md` and
  reject any cell whose identity evidence is incomplete.

- [ ] Under ratified Option A, add non-duplicate baseline entries only for:
  `claudine/wsl2-ubuntu/L1`, `sniff/wsl2-ubuntu/L1`,
  `messenger/wsl2-ubuntu/L1`, `sniff/{macos-latest,ubuntu-latest,windows-latest}/L1`,
  `sniff/ubuntu-latest/lint`, `sniff-cli/ubuntu-latest/lint`,
  `dmls/ubuntu-latest/L2`, `rendezvous-daemon/windows-latest/L1`, and
  `unchained-ai/windows-latest/L1`.

- [ ] Give every new entry `owner = "@yankeeinlondon"`,
  `source_run = "31588186544"`, `expiry = "2026-09-30"`, and a reason naming
  the exact observed failure family. Preserve existing entries and do not
  broaden a package/environment/tier key beyond the approved list.

- [ ] Verify `problems.md` retains a concrete main-side handoff for every
  accepted cell, including the cheap mechanical fixes, without implementing
  those unrelated fixes on this branch.

- [ ] Run the full verdict and inspect every baseline finding. Require no
  `baseline-no-result`, `baseline-now-passing`, expired, missing, or cancelled
  entry; a newly passing cell must have its entry removed rather than accepted.

- [ ] Run `just ci-diff` for the branch and review the identity-level output,
  not only the green verdict. If a new failing identity appears inside an
  accepted cell, remove that baseline entry or resolve the regression before
  proceeding.

### Option B path

- [ ] If Open Question 2 is ratified as Option B, leave the baseline unchanged,
  land or wait for the main-side fixes, rebase the branch, and repeat the entry
  gate and identity diff before Phase 6.

## Phase 6 — Final cross-platform acceptance

**Objective:** run the complete package and CI matrix, audit documentation and
scope, and produce the evidence needed for merge.

### Tasks

- [x] Run `cd darkmatter && just test && just lint`; require the
  OS-independent punctuation/context matrix, passive shipped corpus, and
  intentional-Markdown escape-hatch tests to pass.

- [x] Run `cd claudine && just test && just lint`; require F1, the normal F2
  drift test, F4 path projections, the fixture contract, F6 command
  normalization, and the F7 work-count regressions to pass.

- [x] Re-run `md hash prompts/_implement/implement-plan.md` and the normal
  non-update `shipped_prompt_route_drift` test as a final mutation guard.

- [ ] Run native `windows-latest` L1 cells for `darkmatter`, `claudine`, and
  `claudine-cli`; require every P1-P3 identity from `problems.md` to be green
  and inspect output for Unix drift, command-byte drift, or `\\?\` leakage.

- [ ] Complete the second consecutive `ubuntu-latest` `claudine-cli` L1 proof
  for the F7 router/perf family and attach the final per-stage timing/work-count
  comparison.

- [ ] Run the canonical Claudine and Darkmatter package-area Level-2 suites
  through `just test-l2`, without focusing a host terminal window, and resolve
  or disposition every failure before accepting the parent fix.

- [ ] Run full CI/`ci-verdict`, followed by `just ci-diff`; require no
  unapproved blocking cell, no new red identity relative to run `31651014023`,
  and no baseline that is passing, expired, missing, or unsupported by source
  evidence.

- [x] Audit the final diff for scope: no P5-P7 implementation fixes, no global
  timeout increase, no process-global environment mutation, no runtime fixture
  compilation, no production helper binary, and no focused terminal/browser
  window.

- [x] Review all edited `///`, `//!`, and inline comments plus public
  interpolation/path documentation. Correct drift in the touched behavior
  sections and confirm the Claudine and Darkmatter skill snapshots describe the
  shipped contracts.

- [ ] Record the final CI run IDs, targeted test results, two Ubuntu latency
  proofs, baseline decision, and identity-diff outcome in the PR handoff so the
  merge gate can be independently audited.

### Phase 6 local acceptance record

- The pre-CI local acceptance record covered Level 1 and lint only. No Level-2
  suite ran before CI run `31753281913`; the affected-package Level-2 task above
  is now part of acceptance under the triage fix's ratified Option A.

- `cd darkmatter && just test` passed the `darkmatter`, `darkmatter-cli`, and
  `dmls` L1 gates; `just lint` passed all three package lint gates.
- `cd claudine && just test` passed the `claudine-catalog-types`, `claudine`,
  `claudine-contract`, `claudine-cli`, and `claudine-gen` L1 gates; `just lint`
  passed all five package lint gates and the diagnostic contract guard.
- `md hash prompts/_implement/implement-plan.md` returned
  `62d70fb16a02592c-652e691e678f8b5a`; the normal non-update
  `shipped_prompt_route_drift` run passed all three identities.
- Native CI remains intentionally pending. `HEAD` is
  `565db2360189534be3c142952e4033ec75d2b1de`, while the Phase 1-4 changes are
  uncommitted and the task forbids staging, committing, or pushing them. A CI
  run dispatched now would test older source. In addition, `just ci-diff`
  cannot access GitHub because this non-interactive session has no `GH_TOKEN`.
