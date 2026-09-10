---
fix: 2026-09-07-faster-claudine-tests
review: 2026-09-07-faster-claudine-tests/review-1.md
reviews:
    - 2026-09-07-faster-claudine-tests/review-1.md
    - 2026-09-07-faster-claudine-tests/review-3.md
updated: 2026-09-09T15:44:00-07:00
log: claudine/fixes/2026-09-07-faster-claudine-tests/log.md
created: 2026-09-09T13:24:00-07:00
---

# Deferred performance measurement

Two performance measurements could not be taken during review cycle 1. Both are
recorded here in full rather than absorbed into a summary, because each has a
concrete, verified blocker and a concrete condition that closes it.

Neither deferral is a CPU-load artifact. Both are structural.

## 1. Cross-platform CI budgets

- **Maps back to:** review-1 § Findings → **3. High: the required CI performance
  contract has no candidate evidence or ratified budgets**, and Closure Criterion
  4. Review file:
  [`review-1.md`](review-1.md).
- **Specification requirement:** RB5 — "Retain baseline and three consecutive
  candidate CI runs for each configured package/environment leg, including
  Claudine CLI's native Linux/macOS/Windows and WSL2 legs" — and the acceptance
  criterion "the ratified performance budgets have compatible CI evidence".

### What this cycle did close

The review names four sub-tasks in Closure Criterion 4. Exactly one of them is
code rather than evidence, and it shipped:

**The JUnit-to-family aggregation exists.** `test-audit attribute aggregate` in
`tools/test-audit/src/attribute/aggregate.ts` walks the stored
`<run-id>/<env>/<tier>/<package>.xml` staging trees and resolves every
`<testcase>` through the reconciler's existing `familiesMatching`, emitting the
`perLegFamilySummed` shape `deriveBudgets` already consumes. It is a join, not a
second classifier.

Against the one real stored run, `baseline/34173378609/`, the join is total —
0 identities unclaimed, 0 double-claimed, on all four legs — and reproduces
`results.md` § CI baseline to 0.1 s:

| Leg | Cell | Tests | Summed |
|---|---|---:|---:|
| `ubuntu-latest` | `L1/claudine-cli` | 2466 | 324.66 s |
| `macos-latest` | `L1/claudine-cli` | 2466 | 753.49 s |
| `windows-latest` | `L1/claudine-cli` | 2105 | 355.79 s |
| `wsl2-ubuntu` | `L1/claudine-cli` | 2466 | 692.16 s |

The observable effect on the budget gate is that its refusal **narrowed rather
than disappeared**:

```text
before:  [missing-leg]        ubuntu-latest: declared but carries no measurements
after:   [insufficient-runs]  ubuntu-latest: 1 green run(s); 3 consecutive are required
```

That is the correct end state at one stored run. `runsPerLeg` was not raised and
`perLegFamilySummed` was not hand-filled in
`attribution/budgets-pending.json`; the recorded refusal reproduces
byte-identical.

### Why the rest is deferred

Every remaining sub-task requires actions this session structurally cannot take:

1. **Integrate current `main`.** The branch conflicts in thirteen files — eight
   under `claudine/cli/tests/`, `claudine/justfile`'s `test-real` recipe,
   `lib/src/diagnostics/registry.rs`, `dispatch-inventory.json`, a skills
   `timeline.md`, and one completed spec. Resolving a thirteen-file merge is an
   operator decision, and the handoff is already written in
   [`candidate/README.md`](candidate/README.md).
2. **Commit.** `CLAUDE.md` requires every commit to be OpenPGP-signed. This
   session is non-interactive; a signed commit would block on the passphrase via
   `gpg-agent`/`pinentry` rather than fail, and bypassing with `--no-gpg-sign` is
   explicitly forbidden.
3. **Push.** `origin/fix/cli-slow-tests` no longer exists — GitHub auto-deleted
   it when PR #69 merged — so the push has to recreate the remote branch, and the
   pull request body is staged at [`candidate/pr-body.md`](candidate/pr-body.md).
4. **Collect the runs.** Three consecutive green GitHub Actions runs per leg
   across `ubuntu-latest`, `macos-latest`, `windows-latest` and `wsl2-ubuntu`.
   Current state: **baseline 1 of 3**, **candidate 0 of 3**. The two further
   baseline samples need `gh run rerun 34173378609` at the same source state
   (`444213eb5`); `main` has since moved to `6504747e2`, whose run
   `34232285291` measures a different tree and is therefore not a substitute.
5. **Ratify budgets.** Downstream of 4 by construction. `deriveBudgets` refuses
   non-CI provenance and refuses fewer than three consecutive green runs per leg,
   both by design and both covered by named tests, so no number can be produced
   in the interim without disabling a gate — which the specification forbids.

**No local run substitutes.** The fix's own Phase 3 recorded why in numbers:
`.config/nextest.toml`'s CI profile binds `claudine-cli`'s L1 suite to
`max-threads = 1`, so on CI the summed duration is the leg's floor, while
locally the same suite reports 289.12 s summed behind 18.12 s elapsed. Local
numbers are simultaneously inflated by contention and deflated by parallelism,
and neither correction is arithmetic.

### Closes when

The operator merges `main`, commits with signing available, pushes, opens the
PR, and collects the runs with the recipe in
[`candidate/README.md`](candidate/README.md). Then
`test-audit attribute aggregate` produces `perLegFamilySummed` from those trees,
`budgets-pending.json` reaches `runsPerLeg` 3, and `deriveBudgets` emits the
table that lands beside the baseline in
[`inventory.md` § Budgets](inventory.md#budgets). Tracked as residual 7 in
[`../_unscheduled/test-suite-residuals/spec.md`](../_unscheduled/test-suite-residuals/spec.md).

## 2. Local re-measurement of the moved cohorts

- **Maps back to:** review-1 Closure Criterion 1, whose final clause is
  "regenerate inventory and measurement evidence". Review file:
  [`review-1.md`](review-1.md).
- **Specification requirement:** RB5 — "Collect five alternating warm local runs
  per revision for changed cohorts and the relevant full L1 suites."

### What this cycle did close

The **inventory** half was regenerated in full. The `9fc5151a0` listings were
preserved under `enumeration/9fc5151a0/` and new listings captured from the
working tree at `78b44a96651e`, with `capture --note` recording why no committed
revision was used. `inventory.md` states both revisions' numbers side by side:
7,400 → 7,417 runner identities, 163 → 165 build targets, 7,455 → 7,472 source
attributes, exclusions steady at 55, and `terminal-tests` dropping from
241 identities across 34 binaries to 222 across 30 — the direct, visible
consequence of moving the four `expectrl` binaries onto the bare L1 route.
`inventory-reconciler.ts` exits 0 against the regenerated captures.

### Why the timing half is deferred

`attribute` rejects a red run by construction — a run with failures measures
nothing — and this branch is red from **18 failures that predate this cycle**.
The list was re-derived by running the gates, not assumed:

| Count | Identity |
|---:|---|
| 5 | `claudine-cli::bin/claudine …loop_control::target_launch::tests::*` |
| 1 | `claudine-cli::propagated_context_fixtures::isolated_fixture_can_opt_in_to_provider_memory_discovery` (fails in isolation) |
| 1 | `claudine-cli::spawn_inventory::production_spawn_inventory_is_complete_and_governed` — line-number drift in production sources from commit `f0aaa4832` |
| 1 | `claudine-cli::wrap_sigint::compose_sigint_during_prep_exits_130_with_notice` — times out at 30 s in isolation, so a genuine failure rather than a load artifact |
| 10 | `claudine-gen::drift` / `claudine-gen::generate_ux` — the known archived-baseline break |

None is in a file this cycle touched: the only edits to shared test code
(`claudine/cli/tests/common/mod.rs`, `common/pty.rs`) are comment-only, verified
by reading the diff, and `just test-cli` reports 2523 run / 2515 passed both
before and after every unit in the cycle.

Running the spec's five-alternating-warm-run protocol against that suite would
produce exactly the evidence the gate is built to reject. Rather than fabricate
or scale a number, `inventory.md` keeps `Executed` and `Summed cost` at their
`9fc5151a0` values, labelled with that revision, and states plainly that for the
six families whose membership moved the recorded cost is now a **lower bound,
not a measurement**.

### Closes when

The 18 pre-existing failures are repaired — most likely as part of the `main`
integration, since at least the `spawn_inventory` drift is a direct consequence
of commit `f0aaa4832` and looks trivially repairable — and the five-run protocol
is then run against a green suite at the merged revision. Tracked as residual 8
in [`../_unscheduled/test-suite-residuals/spec.md`](../_unscheduled/test-suite-residuals/spec.md).

## Review-3 update (2026-09-09, iteration 3)

Review 3 restates both deferrals as findings — § Findings **2** (CI runs and
budgets) and the measurement half of Closure Criterion **1**. Neither status
changed, but the *reason* for one of them did, and a third deferral is added.

### 1. Cross-platform CI budgets — unchanged status, hardened gate

Still **baseline 1 of 3, candidate 0 of 3, no budgets**. Every blocker listed
above (merge `main`, sign a commit, recreate the deleted remote branch, collect
the runs) remains an operator action a non-interactive session cannot take.
This maps to review 3 finding 2 and Closure Criterion 5.

What review-3 iteration 3 *did* close is Closure Criterion **4**, the provenance
half of that finding. The aggregator can no longer be satisfied by arbitrary
staging directories:

- staging now writes a `provenance.json` per tree carrying `GITHUB_SHA`,
  `GITHUB_RUN_ID`, `GITHUB_RUN_NUMBER`, `GITHUB_RUN_ATTEMPT`, `GITHUB_REF`,
  `GITHUB_EVENT_NAME` and `GITHUB_WORKFLOW`;
- `aggregate()` **derives** the provenance kind from that stamp instead of
  trusting a caller-supplied `--provenance` flag, which is gone;
- a sample set spanning two source revisions, or carrying non-consecutive run
  numbers, now raises `mixed-source-revisions` / `non-consecutive-runs`;
- manifest cells must match one-to-one — `duplicate-manifest-cell` and
  `unexpected-manifest-cell` invalidate the evidence set instead of being
  silently ignored.

**A consequence worth stating plainly:** the one stored baseline run,
`baseline/34173378609/`, was staged before provenance existed and therefore
now reads as `local` rather than `ci`. It no longer counts toward `runsPerLeg`.
That does not change the gate's verdict — three consecutive runs were already
missing — but it does mean the baseline must be **re-collected**, not merely
topped up by two. `candidate/README.md`'s recipe should be read with that in
mind.

### 2. Local re-measurement — the blocker moved

The blocker recorded above was "18 pre-existing failures; `attribute` rejects a
red run by design". **Fifteen of the eighteen were repaired in this cycle** (six
by the launch-identity scrub, ten by regenerating the generator-owned sources
that a formatting sweep had rewritten, one by refreshing the `spawn_inventory`
line pins; `wrap_sigint` does not reproduce in isolation over five runs). The
canonical L1 suite went from **20 failed + 1 timed out** to **4 failed**.

So the *stated* reason no longer holds. Two different reasons do:

1. **The protocol needs two comparable revisions.** `spec.md` § 5 asks for
   "five alternating warm local runs **per revision**". Alternating between a
   baseline and a candidate revision requires a committed candidate, and this
   session cannot commit — `CLAUDE.md` mandates OpenPGP signing and a signed
   commit blocks on `pinentry` in a non-interactive session.
2. **The tree is not measurable.** Four identities are red because another
   session is concurrently editing the repository's shipped `prompts/` corpus
   (last write 15:08, mid-cycle). `attribute` rejects a red run, and a tree a
   third party is writing during the run is not a revision anything can be
   attributed to.

`inventory.md`'s **Executed** and **Summed cost** columns therefore stay at
their `9fc5151a0` values and stay labelled as such. The identity counts *were*
refreshed — `test-audit capture` re-ran all 16 selections at `fe83e7481` and
`inventory-reconciler.ts` exits 0 over 7,420 runner identities — so the
enumeration half is current even though the timing half is not.

### 3. New: Level-2 GUI backends could not be verified

- **Maps back to:** review-3 § Findings **3** and Closure Criterion 3, whose
  final clause is "Do not treat skipped GUI backends as passing evidence".
- **What shipped:** tmux and Terminal.app now build the same two-stage
  login-plus-rc-suppressed shell invocation that WezTerm and Kitty use, from one
  shared implementation (`biscuit-test-harness/src/lib.rs` `login_shell_argv` /
  `login_shell_command_line`). The tmux half is proven end-to-end by
  `biscuit-test-harness/tests/level2_tmux_shell_startup.rs` against a temporary
  `HOME` whose interactive rc plants a sentinel, and it is non-vacuous —
  reverting the change makes the sentinel appear in the pane.
- **What could not be verified, and why:** Terminal.app's change is covered only
  by a composition unit test on the escaped `do script` payload, because
  spawning it opens a **visible window** on this host and
  [L3/L2 tests must never steal focus](../../../.claude/skills/rust-testing/SKILL.md).
  WezTerm and Kitty report `available() == false` unless the suite is launched
  from inside them, and cold-starting either also opens a GUI window. Their code
  path is unchanged apart from delegating to the shared helper.
- **A second, smaller gap:** `claudine-gen`'s L2 leg canonically runs the serial
  broker path; both L2 runs in this cycle used parallel self-spawn mode
  (`BISCUIT_L2_THREADS`) precisely so the broker would not open an Apple
  Terminal window. That half therefore deviated from the canonical recipe.
- **Closes when:** an operator runs `claudine/just test-l2` and
  `biscuit-test-harness/just test-l2` on a host where a foreground window is
  acceptable — or CI does, which is the same push this document's § 1 is
  waiting on. Tracked as residual 9 in
  [`../_unscheduled/test-suite-residuals/spec.md`](../_unscheduled/test-suite-residuals/spec.md).
