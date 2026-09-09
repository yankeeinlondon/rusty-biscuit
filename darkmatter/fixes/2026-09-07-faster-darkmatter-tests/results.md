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
| Verified on CI | **Deferred** | The candidate is committed but not pushed. There is no candidate CI source, only one compatible baseline sample per leg. Three consecutive baseline and candidate samples per configured leg are required before budgets and final performance verification can close. Ruled non-blocking for readiness; see [`deferred-performance-measurement.md`](deferred-performance-measurement.md) and [`log.md`](log.md#phase-10--ci-evidence-and-operator-handoff-2026-09-08). |

No Darkmatter production API changed. The implementation changes are confined
to test code, test-only feature instrumentation, fixtures, tier routes, and
audit tooling, so no downstream Claudine API verification is required. The
precise byte-level scope of that claim is reconciled below.

## Candidate identity

[`review-1.md`](review-1.md) raised, as a High finding, that the reviewed tree
was not an isolated candidate: it measured 1,306 modified tracked files, 264
untracked files, 8,067 changed symbols, 119 affected execution flows, and
CRITICAL aggregate risk, spanning production Darkmatter, Sniff, rendering,
schema, and unrelated package code.

That measurement was correct for what it measured, and it did not measure this
fix. The branch `fix/cli-slow-tests` carries **three sibling fixes** — the
Darkmatter, Claudine, and Sniff test-performance fixes — plus their shared
tooling, and the reviewer's `detect_changes(scope=all)` covered the whole
branch and every concurrent agent's uncommitted working tree at once.

### The commit set this fix owns

| Commit | Subject | Files | +/− |
|---|---|---:|---:|
| `8540d36df` | `test(darkmatter-cli): migrate L1 tests to CliProcessFixture with structural guard` | 47 | +6,493 / −1,443 |
| `a1746f217` | `test(darkmatter-dmls): hermetic LSP test env, ChildGuard, ZedDmlsFixture` | 8 | +890 / −242 |
| `a05e3b747` | `refactor(darkmatter): unindent inner test modules and extract image support` | 21 | +6,199 / −5,556 |

Supporting, shared with the sibling fixes and separately attributable:
`d35b4c23b` (promotes `tools/test-audit`) and `b0a5142aa` (its skill/docs
drift). Planning and evidence commits: `3ea71de24`, `6457bd7d9`, `dabbeca02`,
`702cfd833`, `1f49a456e`, `e4a090218`.

The remaining ~45 commits on the branch belong to the Claudine and Sniff
fixes and are not this fix's bytes.

### Change detection on the candidate

Re-running GitNexus scoped to the candidate range rather than the whole branch
(`detect_changes(scope=compare, base_ref=f0aaa4832)` — the commit immediately
preceding this fix's first implementation commit):

| Measure | Whole undifferentiated tree (review) | Candidate range |
|---|---:|---:|
| Changed symbols | 8,067 | 1,444 |
| **Affected execution flows** | **119** | **0** |
| Changed files | 1,306 | 99 |
| Aggregate risk | **CRITICAL** | **low** |

Of the 1,444 changed symbols, **1,396 are in `darkmatter/`** (47 files under
`cli/`, 20 under `lib/`, 6 under `dmls/`). The residual 35 Claudine and 13
Sniff symbols are the sibling agents' concurrent uncommitted work plus
`1e7f2fc30`, which the range unavoidably includes; none of them belong to this
fix.

Zero affected execution flows is the substantive result. It is the direct
evidence for the scope claim above, and it is what the CRITICAL aggregate
obscured.

### Reconciling the "test code only" claim

The claim is accurate in substance but was stated imprecisely. Ten of the 76
candidate files are not under a `tests/` directory:

- `darkmatter/lib/src/layout/page/tests.rs` and the `#[cfg(test)]` modules
  inside `code_block.rs`, `catalog.rs`, `alias.rs`, `code_renderer.rs`, and
  `entrypoints.rs` — test code that happens to live in `src/`. `a05e3b747`
  unindents these inner modules, which is why their diffs are large.
- Within those same files, production items changed **by line-wrapping only**.
  Every non-test hunk inspected is a reflow with no semantic change.
- One substantive test-isolation change lives in `src/`:
  `catalog.rs`'s `capture_shape_matches_projected_type` no longer walks the
  real monorepo to find a Git root; it initializes a disposable repository via
  `gix::init` in a temporary directory. That is a hermeticity fix, not a
  production change.
- `Cargo.lock`, `darkmatter/lib/Cargo.toml`, `darkmatter/dmls/Cargo.toml`
  (test targets and dev-dependencies) and `darkmatter/cli/README.md`
  (documented fixture workflow) round out the set.

So: no production behavior, signature, or API changed. Test code inside `src/`
did.

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

#### Operator ruling — hosted budget evidence is not a readiness blocker

[`review-1.md`](review-1.md) raised the absent candidate CI comparison and
unratified budgets as a High finding. The operator ruled on that finding
directly: *"this bar was set to high and should not be considered a blocker
being production ready."*

The ruling changes the finding's severity, not the evidence. Everything above
stays exactly as measured — no budget is invented, no local timing is promoted
to a target, and no leg is marked green. What changes is the consequence: the
three-sample hosted comparison is **deferred owned work**, not a gate this fix
must clear before merge. The reasoning is that the requirement is structurally
unsatisfiable from the implementation side — a candidate revision cannot be
sampled on hosted Ubuntu, macOS, Windows, and WSL2 runners until it is pushed,
and pushing is the operator's action, so holding readiness on it would make the
fix permanently un-mergeable by its own author.

The deferral, its exact missing artifacts, and the sequence that closes it are
recorded in
[`deferred-performance-measurement.md`](deferred-performance-measurement.md).

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
| Resource cleanup | HTTP workers are shut down and joined. The `dmls` **subprocess** path is proven by `child_guard_reaps_process_during_unwind`: a real spawned probe cannot write its detached-completion marker after unwind. The **in-memory** server threads are proven separately by `server_worker_finishes_before_workspace_release_during_unwind`, which asserts the recorded teardown ordering (worker completion observed, workspace still present at that instant, deletion strictly afterwards) rather than merely the absence of a hang. Post-suite sweeps found no surviving `md`, `dmls`, broker, test, or Chrome-debug process. |

## Coverage changes

The two L1 populations each add **93 asserted identities and remove none**
(84 at the original report, plus 9 from the review-1 isolation-guard widening).
The local/CI slow-population difference remains exactly one existing `slow_`
test. Additions are reported separately:

| Change | Identities | Replacement or dependent proof |
|---|---:|---|
| Pure renderer tests moved from the browser-only boundary into L1 by renaming `browser_*` to `render_browser_*` | 44 | Bodies and exact HTML/CSS assertions are unchanged; the ordinary L1 selector executes all 44. Real headless-browser tests remain browser-routed. |
| `md` fixture contract | 21 | Real child probes cover pinned CWD/home/cache/PATH, hostile values, both command surfaces, Windows plumbing, repository topology, symlink containment, and byte-for-byte shipped-content relocation. A declared behavior input out-ranks both the inherited value and the scrub; every variable the contract pins or scrubs is asserted to be classified in `protected_env.rs`; and the builder rejects a declaration naming a containment variable, the wrong class, or `PATH`. |
| `md` spawn/isolation structural guard | 23 | Negative planted-spawn and post-build escape cases, stale-entry failure, source sanitizer negatives, governed-population checks, and zero generic migration exemptions. The isolation gate covers the launch directory, `PATH`, `env_clear()`, **and** any post-`build()` `.env`/`.env_remove` naming a protected key, with one negative case per namespace — home/config/cache/temp, Git plumbing, rendering, and darkmatter application — plus a case asserting a builder declaration is not an escape. |
| Loopback HTTP ownership/cache contract | 2 | Missing expected requests terminate within the bound; repeated TTL reads return the dependent rendered result with one recorded request. |
| Always-built pixel classifier | 1 | Exact 64×64 magenta and black inputs preserve total, near-target, and non-black count assertions while the L3 consumer remains opt-in. |
| DMLS subprocess unwind/cancellation ownership | 2 | A real spawned probe cannot write its detached-completion marker after unwind or cancellation. |
| DMLS in-memory session cleanup ownership | 1 | `server_worker_finishes_before_workspace_release_during_unwind` drives the shared `common::LspFixture` to a failing assertion inside `catch_unwind` and asserts the recorded teardown sequence is `[WorkerFinished { workspace_present: true }, WorkspaceReleased]`. Non-vacuity was demonstrated by two temporary breaks: reducing the completion bound to zero yields `[WorkerAbandoned]`, and detaching the worker without waiting yields `[]`. |

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
  input; it was compile-checked but not represented as a pass. The
  synchronization change those tests received, and the exact identities left
  unavailable, are recorded in
  [§ Level-3 synchronization and availability](#level-3-synchronization-and-availability).
- Native Windows/WSL2 candidate runtime, three-sample CI performance evidence,
  and `just zed-verify` candidate CI execution remain pending until a candidate
  is pushed.
- The literal `git diff main -- .config/nextest.toml` is not empty because the
  shared branch contains sibling Claudine history. This fix made no working-tree
  change to that file and added no retry, timeout, tier, or override. The
  operator must reconcile branch history before the exact invariant can close.

## Level-3 synchronization and availability

### Why the synchronization changed

Spec §4 requires tests to "poll the final asserted condition" rather than wait a
fixed interval. Both Level-3 files violated that, and the violation was not a
cosmetic one.

`level3_popover.rs` slept 150 ms after every `cliclick` injection and then read
the page once. `level3_image_painting.rs` slept 400 ms after `cat`-ing the
iTerm2 payload and then took one `screencapture`. A fixed sleep of that kind
fails in two opposite directions simultaneously:

- **It fabricates failures.** Quartz event delivery, a Chrome paint, a CDP round
  trip, or a WezTerm image decode do not fit a constant budget on a machine that
  is also compiling. The correct product then loses the race and the test
  reports a regression that does not exist.
- **It hides passes.** A sleep asserts nothing about *when* the value became
  true. A page whose prompt was already visible, or a terminal that already had
  magenta on screen, satisfies a post-sleep read exactly as a page that
  responded to the injected input would. Level 3 exists precisely to prove the
  OS-to-application path carried the event; a sleep cannot distinguish that from
  a state that predates the event.

The replacement is stronger, not merely faster. Each wait now polls **the same
value the test asserts**, so it has no weaker intermediate signal available to
terminate on, and there is no second read that could observe a different frame
than the one that satisfied the wait:

| Site | Polled predicate | Assertion it feeds |
|---|---|---|
| `level3_popover_tab_focuses_anchor_and_reveals_prompt` | `value == "active=true;vis=visible"` | `assert_eq!(.., "active=true;vis=visible")` |
| `level3_popover_enter_activates_link` | `value == "https://example.com/"` | `assert_eq!(.., "https://example.com/")` |
| `level3_popover_pointer_hover_reveals_prompt` | `value == "visible"` | `assert_eq!(.., "visible")` |
| `verify_keyboard_canary` | text `== KEY_CANARY` and event count `> 0` | the identical check on the returned value |
| `verify_pointer_canary` | event count `> 0` | the identical check on the returned value |
| `level3_rich_image_node_paints_distinctive_pixels` | `magenta > 1000 && non_black * 100 >= total` | `assert!(magenta > 1000)`, guarded by the same non-black test |

Two secondary gains follow. On expiry, `wait_for_evaluation` reports the last
observed value and what it was waiting for, which a sleep-then-`assert_eq!`
cannot: by then the interesting value is whatever the single read happened to
catch. And the image test no longer converts a transient `screencapture`
failure into a skip — one `None` used to abandon the pixel claim, where now only
an all-`None` five seconds does.

`resolve_window_process_id` changed shape for the same reason: a 20 × 100 ms
attempt loop became a 2 s deadline polled at 25 ms. Its condition — the headed
Chrome window is resolvable by title — is unchanged, so the only differences are
a faster first success and a bound stated in time rather than in attempts.

### Whether this can mask a regression

Yes, and the bounds were chosen with that in mind rather than around it. A
genuine product regression that pushed anchor focus to 2.9 s, or image paint to
4 s, would pass under the new deadlines where the old 150 ms and 400 ms sleeps
would have failed. The bounds are 3 s (popover) and 5 s (image paint).

That trade is accepted because **neither file asserts a latency contract**. Both
claim only that OS input or protocol bytes reach the application and produce the
observable effect at all; no assertion in either has a timing floor, and no tier
covers interaction or paint latency. A sleep that failed at 151 ms was not
enforcing a budget either — it was enforcing whatever the host happened to be
doing. Asserting latency would require an explicit, measured budget, which is
out of this fix's scope; converting a synchronization timeout into a de facto
one would be exactly the "timing-based exclusion" the spec forbids.

One cost is worth naming: `capture_window_png` raises the WezTerm window and
lets the compositor settle before sampling, so an image-paint poll iteration
costs about 350 ms regardless of the 50 ms sleep, and the 5 s bound therefore
buys roughly a dozen attempts, each re-raising the window. This is an
attended-host tier; the repeated raise is visible to whoever is at the machine.

The full rationale also lives with the code, in the module documentation of
[`darkmatter/lib/tests/level3_popover.rs`](../../lib/tests/level3_popover.rs)
and
[`darkmatter/lib/tests/level3_image_painting.rs`](../../lib/tests/level3_image_painting.rs),
so it survives independently of this report.

### Identities recorded as UNAVAILABLE

These four are **not** passes. They have never been executed on this branch:

| Identity | Claim |
|---|---|
| `darkmatter::level3_popover level3_popover_tab_focuses_anchor_and_reveals_prompt` | OS Tab focuses the prompted link and reveals its prompt |
| `darkmatter::level3_popover level3_popover_enter_activates_link` | OS Enter activates the prompted link's ordinary href |
| `darkmatter::level3_popover level3_popover_pointer_hover_reveals_prompt` | OS pointer hover reveals the prompted link's prompt |
| `darkmatter::level3_image_painting level3_rich_image_node_paints_distinctive_pixels` | The emitted iTerm2 payload is decoded and painted as pixels |

Their prerequisite is an **attended macOS host**: foreground window focus that
may be stolen, OS input injection through `cliclick` with Accessibility trust
granted, headed Chrome, WezTerm, and Screen Recording permission for the parent
terminal. Running them from this session would seize the desktop of whoever is
using the machine, which the repository's Level-3 contract forbids.

The only evidence collected is compile reachability, which is not behavioral
evidence:

```text
cargo nextest list -p darkmatter --features terminal-tests,browser-tests \
  -E 'test(/(^|::)level3_/)'
```

All four identities compile under the gating features and are selected by the
`test-l3` filterset. Nothing was executed.

### How a skip is recorded, and why these greens are not evidence

libtest has no runtime "skipped" outcome. `require_level!` resolves an
unsatisfied gate to `LevelDecision::Skip`, prints `skipping: …` to stderr and
`return`s (`tools/test-toolkit/src/lib.rs`), so the identity is reported
**green**. That is a monorepo-wide property, not a Darkmatter one, and it means
a green Level-3 result is only evidence in combination with the run's stderr.

Three mechanisms keep that green out of this fix's record, and they were
verified rather than assumed:

1. `_tier_filter L1` excludes `test(/(^|::)level3_/)`, so the canonical
   `just test` never selects these four. No Level-3 green appears in the L1
   evidence at all.
2. `just test-l3` refuses outright without a TTY unless `BISCUIT_L3_TAKE_FOCUS=1`
   is set, and prompts for confirmation otherwise (`just/devops.just`). An
   unattended session cannot produce even a skipped green — which is exactly what
   happened here, and is the recorded reason the tier was not run.
3. `BISCUIT_TEST_LEVEL_REQUIRED=3` promotes an unavailable harness from `Skip` to
   `Panic`. It applies to label-only gates as well as backend gates, which
   matters because the three popover tests gate on the string
   `"headed Chrome + cliclick + macOS Accessibility"` and therefore contribute no
   `BISCUIT_TEST_REQUIRED_BACKENDS` evidence of their own.

**One gap in that chain was real and has been closed.** `require_level!`
enforces only the gate at the top of a test.
`level3_rich_image_node_paints_distinctive_pixels` has two further exits decided
mid-body — every capture returned `None`, or the capture came back essentially
black because Screen Recording was never granted — and both were bare `return`s.
Under `BISCUIT_TEST_LEVEL_REQUIRED=3` the enforcement variable did not reach
them, so an operator's authorized Level-3 run on a mis-permissioned host would
have filed a green pass for a pixel claim that was never evaluated. Both exits
now route through `skip_pixel_assertion`, which applies the same
`BISCUIT_TEST_LEVEL_REQUIRED=3` rule the toolkit applies to the gate and panics
with the reason. The behavior is inert unless the variable is set.

Consequently the attended run that closes this deferral must be invoked as:

```sh
cd darkmatter
BISCUIT_L3_TAKE_FOCUS=1 BISCUIT_TEST_LEVEL_REQUIRED=3 just test-l3
```

so that a missing resource fails the tier instead of passing it, and its stderr
must be retained to show that no `skipping` line was emitted for any of the four
identities. Until that evidence exists, these claims stay unavailable.

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
| AC2 — deterministic spawn isolation | **Verified locally** | Fixture/guard tests pass; zero raw `md` sites and zero generic migration exemptions. The isolation gate now also rejects a post-`build()` `.env`/`.env_remove` of any variable the contract pins, classified by `cli/tests/common/protected_env.rs` — the same table the builder validates a declaration against, with a fixture test asserting no pinned or scrubbed variable is left unclassified. Every intentional test-specific claim was migrated to a named builder declaration (`rendering_input`, `application_input`, `plain_terminal`); the allow-list still carries its single `md_process_fixture.rs` entry for parent-side `git()` helpers. Six `zed-dmls` cases use their specifically justified fixture equivalent. |
| AC3 — hostile-input independence | **Verified locally** | Real-child hostile environment probes plus disposable relative-reference/source-context repositories pass. A post-`build()` re-set of the hostile inputs those probes rule out — `HOME`, `GIT_DIR`, `COLUMNS`, `DARKMATTER_*` — is now a structural failure rather than a silent one. |
| AC4 — bounded resource ownership | **Verified locally** | HTTP missing-interaction shutdown, request corpus, protocol response synchronization, and survivor sweeps pass. DMLS cleanup is proven on both session shapes: the subprocess path by `child_guard_reaps_process_during_unwind`, and the in-memory path by `server_worker_finishes_before_workspace_release_during_unwind`. All three in-memory targets (`lsp_session`, `no_side_effects`, `suggest_constraint_phase1`) now share one owned fixture, `dmls/tests/common/mod.rs`, whose teardown is non-panicking during unwind and bounded by `SERVER_EXIT_BOUND` (10 s) on the worker's outcome channel; on expiry the worker is deliberately detached and reported rather than joined, because `JoinHandle` offers no timed join. The fixture borrows its workspace, so dropck — not drop-order convention — is what forbids releasing the workspace while a session is live. |
| AC5 — passive/no-effect and replacement proof | **Verified locally** | Effect/network counters, sentinels, shipped-artifact corpus, real invocation, persisted round trips, and the coverage table above. |
| AC6 — gates and unchanged coverage | **Verified locally; CI pending** | L1/lint/doctest/L2/browser/Windows compile gates pass at the unchanged implementation source state; two redundant loaded closure attempts and their passing isolated diagnostics are disclosed above. L3 is unavailable by policy; CI features and slow selection are unchanged; candidate CI is absent. |
| AC7 — results and comparable evidence | **Deferred (operator-ruled non-blocking)** | This report separates status and all local evidence, but budgets and three matched candidate samples per leg cannot be produced before push. Ruled not a readiness blocker; deferral and closing sequence in [`deferred-performance-measurement.md`](deferred-performance-measurement.md). No limit or retry was weakened. |
| AC8 — docs and downstream impact | **Verified locally** | Darkmatter README/CLI README and the Darkmatter/rust-testing skills describe the new fixture workflow. No production API changed. |

The fix is ready for review and operator-owned CI sampling. It is not ready to
claim final CI performance verification or archival completion.
