---
fix: 2026-09-07-faster-darkmatter-tests
status: implementation-complete-ci-pending
created: 2026-09-08
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - zed-dmls-cli
---

# Faster Darkmatter tests — results

## Completion status

| Claim | Status | Evidence |
|---|---|---|
| Implementation | **Complete** | Deterministic `md` and `zed-dmls` launches are fixture-owned; passive paths have counter/sentinel proofs; HTTP, child, and protocol fixtures own bounded cleanup; no generic spawn exemption remains. See [`log.md`](log.md) Phases 4–8. |
| Verified locally | **Complete for available resources** | Canonical L1, lint, doctest, L2, browser, Windows cross-compile, Zed check, reconciler, and leak-sweep evidence are recorded below and in [`log.md`](log.md). L3 is intentionally unavailable in an unattended session because it requires foreground OS input. |
| Verified on CI | **Pending** | The implementation has not been committed or pushed. There is no candidate CI source, only one compatible baseline sample per leg. Three consecutive baseline and candidate samples per configured leg are required before budgets and final performance verification can close. See [`log.md`](log.md#phase-10--ci-evidence-and-operator-handoff-2026-09-08). |

No Darkmatter production API changed. The implementation changes are confined
to test code, test-only feature instrumentation, fixtures, tier routes, and
audit tooling, so no downstream Claudine API verification is required.

## Performance evidence

### Budgets

No budget is ratified. This is a required pending result, not an inferred
target: `test-audit attribute budgets` rejects every leg with
`insufficient-runs` because only one of three required consecutive green
baseline samples exists for Ubuntu, macOS, Windows, and WSL2. The fixed future
rule is `ceil(worst observed family summed duration × 1.25)` independently for
each environment and family. Local timings are never accepted as budget input.
The complete observed one-sample family table and refusal output are in
[`inventory.md` § Budgets](inventory.md#budgets).

Consequently there is no honest budget pass or miss to report yet. The owned
operator sequence is in [`log.md` § PR and CI handoff](log.md#pr-and-ci-handoff):
obtain two additional compatible baseline samples, push one stable candidate,
collect three consecutive green candidate runs per configured leg, reconcile
each staging tree, and compare matched identities against the then-ratified
ceilings without changing them.

### Local cohorts: build/setup, runner, and summed cost

Five alternating warm samples per revision were accepted for each L1 cohort.
All runs had stable within-revision identities and zero failures, timeouts,
leaks, or retries. Build/setup is separate from the two test-cost columns.

| Cohort | Revision | Identities | Build/setup | Runner min / median / max | Summed min / median / max | Skips |
|---|---|---:|---:|---:|---:|---:|
| local default | baseline | 7,661 | 1.4–1.6 s | 43.17 / 56.54 / 58.32 s | 684.79 / 899.03 / 925.70 s | 50 |
| local default | candidate | 7,745 | 1.5–1.6 s | 50.02 / 62.36 / 63.70 s | 787.43 / 988.08 / 1,010.29 s | 6 |
| CI-selected slow population | baseline | 7,662 | 1.4–1.5 s | 51.60 / 57.28 / 60.57 s | 817.09 / 909.24 / 959.65 s | 49 |
| CI-selected slow population | candidate | 7,746 | 1.5–1.6 s | 60.87 / 62.50 / 75.52 s | 963.46 / 990.97 / 1,195.39 s | 5 |

The aggregate candidate medians are 9–10% higher, but both deltas fall inside
the host's measured drift brackets and are not established. The changed
`darkmatter-cli` integration cohort fell from 194.03 s to 100.81 s median
summed duration (−48.0%), and its loopback HTTP cohort from 25.27 s to 3.13 s
(−87.6%). These remain local attribution, not targets or CI claims. Full
per-identity data is in [`measurement/report.md`](measurement/report.md).

### CI legs

The compatible baseline is run `34008778001`, one green sample for each leg.
There is no candidate row to compare because no candidate revision was pushed.

| Leg | Baseline L1 runner / summed / tests | Candidate | Additional configured tier evidence |
|---|---:|---|---|
| Ubuntu | 167.4 / 668.2 s / 6,332 | pending | CLI L2 7.1 s / 69; DMLS L2 1.7 s / 3; browser 40.6 s / 86 |
| macOS | 175.2 / 699.5 s / 6,332 | pending | CLI L2 14.0 s / 69; DMLS L2 4.1 s / 3 |
| Windows | 329.0 / 1,315.4 s / 6,338 | pending | none configured |
| WSL2 Ubuntu | 295.4 / 1,181.6 s / 6,338 | pending | none configured |

Matched-identity comparison, malformed/missing report gating, and per-family
budget comparison remain pending with the candidate artifacts. Cross-platform
counts will be compared within each environment; added, removed, and gated
identities will not be netted or compared across platforms.

### Eliminated-work evidence

Timing is not used as the proof of removed work. The complete mapping is in
[`measurement/work-evidence.md`](measurement/work-evidence.md):

| Work | Observable proof |
|---|---|
| Host/repository discovery | Hostile CWD, home/config/cache, Git plumbing, PATH, rendering, and application inputs do not reach the real `md` child; the structural guard reports zero raw `md` spawn bypasses. |
| Composition/effects on passive paths | Effect-engine build counts remain unchanged while schema and DMLS tests still assert diagnostics, completion, hover, definitions, references, and malformed/valid representation behavior. |
| Network on passive or denied paths | Network counters remain unchanged; denied loopback cases record zero requests, while allowed/cache cases assert exact paths, Host headers, freshness, refresh, fallback, and one-request repeated TTL reads. |
| Resource cleanup | HTTP workers are shut down and joined; DMLS children/server threads are reaped on normal, unwind, and cancellation paths; post-suite sweeps found no surviving `md`, `dmls`, broker, test, or Chrome-debug process. |

## Coverage changes

The two L1 populations each add **84 asserted identities and remove none**.
The local/CI slow-population difference remains exactly one existing `slow_`
test. Additions are reported separately:

| Change | Identities | Replacement or dependent proof |
|---|---:|---|
| Pure renderer tests moved from the browser-only boundary into L1 by renaming `browser_*` to `render_browser_*` | 44 | Bodies and exact HTML/CSS assertions are unchanged; the ordinary L1 selector executes all 44. Real headless-browser tests remain browser-routed. |
| `md` fixture contract | 17 | Real child probes cover pinned CWD/home/cache/PATH, hostile values, both command surfaces, Windows plumbing, repository topology, symlink containment, and byte-for-byte shipped-content relocation. |
| `md` spawn/isolation structural guard | 18 | Negative planted-spawn and post-build escape cases, stale-entry failure, source sanitizer negatives, governed-population checks, and zero generic migration exemptions. |
| Loopback HTTP ownership/cache contract | 2 | Missing expected requests terminate within the bound; repeated TTL reads return the dependent rendered result with one recorded request. |
| Always-built pixel classifier | 1 | Exact 64×64 magenta and black inputs preserve total, near-target, and non-black count assertions while the L3 consumer remains opt-in. |
| DMLS unwind/cancellation ownership | 2 | A real spawned probe cannot write its detached-completion marker after unwind or cancellation. |

The browser cohort changes from 86 to 43: 44 in-process renderer tests moved to
L1, and the real Mermaid sanitizer gained an honest browser route. Nothing was
moved to a higher tier, ignored, or stripped of assertions. The six original
`zed-dmls` CLI cases retain their exact command arguments plus exit, output,
staging-directory, link, and log assertions through the fixture-equivalent
contract. Persisted frontmatter and hash tests retain repeated
read/write/read and idempotence assertions. Passive shipped-schema corpus and
normal shipped-artifact invocation coverage remain in place; no parser,
schema, template, prompt, configuration, or persistence format changed.

## Failures, skips, and unavailable evidence

- The accepted local samples contain zero failures, timeouts, leaks, or
  retries. Local default skips six identities: four opt-in performance
  harnesses, one host-login-shell `ll` smoke test, and one locally excluded
  `slow_` cleanup test. The CI-selected local cohort skips five because it
  includes the slow test.
- One unsuitable high-load attempt is retained under
  [`measurement/rejected-high-load/`](measurement/rejected-high-load/).
- One loaded HTTP warm-up exposed an accepted-socket `WouldBlock` race. Its
  exact failing input and run are retained under
  [`measurement/rejected-http-regression/`](measurement/rejected-http-regression/);
  the corrected fixture then passed twelve loaded executions with zero retry.
- Two redundant Phase 11 closure attempts ran while other agents repeatedly
  rebuilt the shared Cargo target. The first timed out
  `markdown::compose::frontmatter_interpolation::tests::seed_state_tests::env_resolves`
  after 2,177 passes; the second timed out
  `markdown::compose::type_tests::test_compose_context_capture` after 5,837
  passes. The exact identities then passed without a retry policy or timeout
  change in 6.65 and 6.87 seconds respectively. These loaded attempts are
  rejected evidence; Phase 9's six green full L1 runs and Phase 10's green
  consolidated gate remain applicable because Phase 11 changed documentation
  only.
- L3 runtime is pending because it requires explicit foreground focus and OS
  input; it was compile-checked but not represented as a pass.
- Native Windows/WSL2 candidate runtime, three-sample CI performance evidence,
  and `just zed-verify` candidate CI execution remain pending until a candidate
  is pushed.
- The literal `git diff main -- .config/nextest.toml` is not empty because the
  shared branch contains sibling Claudine history. This fix made no working-tree
  change to that file and added no retry, timeout, tier, or override. The
  operator must reconcile branch history before the exact invariant can close.

## Residual findings and owners

No generic fixture migration is deferred. The two code/document findings are
owned by Ken Snyder in
[`darkmatter/fixes/_unscheduled/test-suite-residuals/spec.md`](../_unscheduled/test-suite-residuals/spec.md):
the existing preflight-proptest override review and three stale `#[ignore]`
comments. Promotion of the now-stabilized fixture core is separately owned in
[`promote-cli-process-fixture/spec.md`](../../features/_unscheduled/promote-cli-process-fixture/spec.md).

Hosted evidence is pending rather than deferred; its owner, exact missing
artifacts, and non-interactive handoff are recorded in
[`log.md` § PR and CI handoff](log.md#pr-and-ci-handoff).

## Acceptance review

| Criterion | Status | Evidence |
|---|---|---|
| AC1 — complete inventory/routes | **Verified locally** | Fresh 11-selection reconciliation assigns 7,892 feature-unioned identities exactly once across 65 families; higher tiers, Zed, benches/fuzz, ignored tests, and the VS Code documented absence have routes/dispositions in [`inventory.md`](inventory.md). |
| AC2 — deterministic spawn isolation | **Verified locally** | Fixture/guard tests pass; zero raw `md` sites and zero generic migration exemptions. Six `zed-dmls` cases use their specifically justified fixture equivalent. |
| AC3 — hostile-input independence | **Verified locally** | Real-child hostile environment probes plus disposable relative-reference/source-context repositories pass. |
| AC4 — bounded resource ownership | **Verified locally** | HTTP missing-interaction shutdown, request corpus, DMLS unwind/cancellation, protocol response synchronization, and survivor sweeps pass. |
| AC5 — passive/no-effect and replacement proof | **Verified locally** | Effect/network counters, sentinels, shipped-artifact corpus, real invocation, persisted round trips, and the coverage table above. |
| AC6 — gates and unchanged coverage | **Verified locally; CI pending** | L1/lint/doctest/L2/browser/Windows compile gates pass at the unchanged implementation source state; two redundant loaded closure attempts and their passing isolated diagnostics are disclosed above. L3 is unavailable by policy; CI features and slow selection are unchanged; candidate CI is absent. |
| AC7 — results and comparable evidence | **Pending CI** | This report separates status and all local evidence, but budgets and three matched candidate samples per leg cannot be produced before push. No limit or retry was weakened. |
| AC8 — docs and downstream impact | **Verified locally** | Darkmatter README/CLI README and the Darkmatter/rust-testing skills describe the new fixture workflow. No production API changed. |

The fix is ready for review and operator-owned CI sampling. It is not ready to
claim final CI performance verification or archival completion.
