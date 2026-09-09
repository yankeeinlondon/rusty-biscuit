---
fix: 2026-09-07-faster-claudine-tests
phase: 2
created: 2026-09-08
revision: 78b44a96651e33b0eb7cb2f82a96255853cc2998 (+ uncommitted working tree)
supersedes-revision: 9fc5151a0df52acb63c5458b1564aa9c01bfe9b1
host: aarch64-apple-darwin (Darwin 27.0.0), 16 cores
toolchain: rustc 1.97.1 (8bab26f4f 2026-07-14)
nextest: cargo-nextest 0.9.136
---

# Test inventory — the eight Claudine-area packages

Phase 2 of [`plan.md`](plan.md), satisfying RB1 and opening AC1. Document-only:
no package source file was changed.

Every number here is mechanical. The runner population comes from sixteen
`cargo nextest list --message-format json` captures kept under
[`enumeration/`](enumeration/); the source population comes from an attribute
scan of the same tree; the cost figures come from Phase 1's verbatim gate logs
under [`baseline/local-gates/`](baseline/local-gates/). Nothing was estimated.

> **Two revisions, both stated.** The identity counts were first taken at
> `9fc5151a0`. Review 1's finding 1 then moved nineteen PTY tests from L2 to
> L1, and this fix's own Phases 4–7 added tests, so the captures were retaken
> from the working tree at `78b44a96651e` — no committed candidate revision
> exists yet, and `enumeration/captures.json` records both that fact and the
> uncommitted delta it includes. The superseded listings are kept verbatim
> under [`enumeration/9fc5151a0/`](enumeration/9fc5151a0/); they are what the
> **Executed** and **Summed cost** columns below were measured against, and
> those two columns are *not* restated here. Re-measurement is deferred with a
> named blocker — see [`results.md`](results.md) § Measurement re-run.

> **Local numbers are attribution only.** They set no target. Ratified budgets
> land in [Budgets](#budgets), beside the baseline table, once Phase 1's CI
> window closes; Phase 3 fixed the procedure and recorded the refusal there.
> Where the cost *goes* is [`attribution.md`](attribution.md).

## How to reproduce the reconciliation

```bash
npx tsx claudine/fixes/2026-09-07-faster-claudine-tests/inventory-reconciler.ts \
  claudine/fixes/2026-09-07-faster-claudine-tests/enumeration \
  --families  claudine/fixes/2026-09-07-faster-claudine-tests/families.json \
  --inventory claudine/fixes/2026-09-07-faster-claudine-tests/inventory.md \
  --repo-root .
```

The gate fails on an identity assigned to zero families, an identity assigned
to more than one, a family that matches nothing, an inventory row whose count
disagrees with the captures, a row with no disposition, a source test the
runner never lists that no exclusion explains, and a stale exclusion. It exits
1 on any of those and 2 on a usage error. Its own suite is
[`inventory-reconciler.test.ts`](inventory-reconciler.test.ts) — 69 tests,
`npx tsx --test inventory-reconciler.test.ts`.

## Population

| Package | Runner identities | Source attributes | Δ (platform-gated) | Runner at `9fc5151a0` |
|---|---:|---:|---:|---:|
| `claudine` | 4,151 | 4,169 | 18 | 4,149 |
| `claudine-catalog-types` | 21 | 21 | 0 | 21 |
| `claudine-cli` | 2,760 | 2,775 | 15 | 2,746 |
| `claudine-contract` | 52 | 52 | 0 | 52 |
| `claudine-gen` | 158 | 158 | 0 | 158 |
| `rendezvous-client` | 21 | 24 | 3 | 21 |
| `rendezvous-core` | 82 | 88 | 6 | 82 |
| `rendezvous-daemon` | 172 | 185 | 13 | 171 |
| **total** | **7,417** | **7,472** | **55** | **7,400** |

7,417 identities across 165 build targets, under the union of every feature
selection CI or a canonical recipe uses. The 55-identity delta is entirely
`cfg(windows)` / `cfg(target_os = "linux")` code that this macOS host cannot
enumerate; each one is listed in
[cfg/feature exclusions](#cfgfeature-exclusions) with its gate and its actual
execution route. The exclusion list is unchanged from `9fc5151a0`: the 17 new
identities are all host-enumerable, so the platform-gated delta stayed at 55.

**What moved between the two revisions.** 7,400 → 7,417 runner identities,
163 → 165 build targets, 7,455 → 7,472 source attributes, all in three places:

| Source of the change | Identities | Build targets |
|---|---:|---:|
| `claudine-cli::contamination_probes` — AC3's isolation probes (Phase 4) | +8 | +1 |
| `claudine-cli::compose_frontmatter_model` — commit `f0aaa4832` | +5 | +1 |
| net change inside existing binaries: `cli_process_fixture` 14→22, `spawn_site_guard` 15→19, `claudine` lib 4,040→4,042, `pairing_and_sync` 3→4, `error_guards` 18→8, `context_command` 27→26 | +4 | 0 |

The nineteen PTY tests review 1 reclassified moved *tier*, not *count*: they
were already in the union through `cli-terminal`. What changed is the route
that reaches them — `cli-bare` now lists 4 more binaries and 19 more
identities, and `terminal-tests` adds correspondingly fewer.

**The plan's ≈7,300 estimate was low and its per-package split was stale**
(it said `lib` 4086 / `cli` 2728 / `rendezvous` 296 / `gen` 159 / `contract` 52
/ `catalog-types` 21). Measured: 4,151 / 2,760 / 275 / 158 / 52 / 21. The
grounding fact is superseded by the captures.

### Enumeration substrate

Every capture records the command, revision, working-tree dirty list,
provenance note, toolchain, nextest version, and platform in
[`enumeration/captures.json`](enumeration/captures.json); each run's stderr
sits beside its JSON as `<label>.err`. The reconciler rejects a JSON file in
that directory that the manifest does not declare, and a declared capture whose
file is missing, so the substrate cannot silently drift.

The sixteen superseded listings for `9fc5151a0` are preserved unmodified in
[`enumeration/9fc5151a0/`](enumeration/9fc5151a0/), manifest included, so the
numbers this document previously recorded remain reproducible from the evidence
they were derived from. The reconciler reads only the top-level listings; a
revision-named subdirectory is inert to it.

| Capture | Package | Features | Suites | Identities | Identities at `9fc5151a0` |
|---|---|---|---:|---:|---:|
| `catalog-types-bare` | `claudine-catalog-types` | — | 1 | 21 | 21 |
| `lib-bare` | `claudine` | — | 14 | 4,151 | 4,149 |
| `contract-bare` | `claudine-contract` | — | 1 | 48 | 48 |
| `contract-real` | `claudine-contract` | `real-tests` | 2 | 52 | 52 |
| `cli-bare` | `claudine-cli` | — | 95 | 2,532 | 2,499 |
| `cli-daemon` | `claudine-cli` | `daemon-tests` | 95 | 2,537 | 2,504 |
| `cli-terminal` | `claudine-cli` | `terminal-tests` | 125 | 2,754 | 2,740 |
| `cli-real` | `claudine-cli` | `real-tests` | 96 | 2,533 | 2,500 |
| `cli-ci` | `claudine-cli` | `daemon-tests,terminal-tests` | 125 | 2,759 | 2,745 |
| `cli-all` | `claudine-cli` | all three | 126 | 2,760 | 2,746 |
| `gen-bare` | `claudine-gen` | — | 11 | 155 | 155 |
| `gen-terminal` | `claudine-gen` | `terminal-tests` | 12 | 158 | 158 |
| `rz-core-bare` | `rendezvous-core` | — | 1 | 82 | 82 |
| `rz-core-testsupport` | `rendezvous-core` | `test-support` | 1 | 82 | 82 |
| `rz-daemon-bare` | `rendezvous-daemon` | — | 5 | 172 | 171 |
| `rz-client-bare` | `rendezvous-client` | — | 4 | 21 | 21 |

Feature deltas worth naming:

- `daemon-tests` adds **5** identities to `claudine-cli` and pulls bundled
  DuckDB into the build graph. Five tests is the entire visible return on the
  heaviest optional dependency in the area.
- `terminal-tests` adds **222** identities to `claudine-cli` (30 L2/L3 binaries)
  and **3** to `claudine-gen`. It was 241 across 34 binaries at `9fc5151a0`;
  the four `expectrl` binaries review 1 reclassified dropped their
  `required-features = ["terminal-tests"]`, so their 19 identities now list on
  the bare L1 route instead.
- `real-tests` adds **1** identity to `claudine-cli` and **4** to
  `claudine-contract`.
- `rendezvous-core`'s `test-support` adds **0** identities. It exists for
  *other* crates' fixtures (`rendezvous-daemon` and `rendezvous-client`
  dev-depend on it with the feature on), which is correct but means the flag
  changes nothing about `rendezvous-core`'s own suite.

### Build targets that list no test

Seven targets are compiled and produce no identity:

| Target | Why |
|---|---|
| `claudine-cli::wrap_ctrl_c_windows` | whole file is `cfg(windows)`; one identity on `windows-latest` |
| `claudine-cli::sequence_ctrl_c_windows` | same |
| `claudine-cli::level3_windows_sequence_ctrl_c` | same |
| `claudine-cli::level3_linux_sequence_ctrl_c` | whole file is `cfg(target_os = "linux")`; one identity on `ubuntu-latest`/WSL2 |
| `claudine-gen::bin/claudine-gen` | binary target, no `#[test]` — normal |
| `rendezvous-daemon::bin/rendezvous-daemon` | binary target, no `#[test]` — normal |
| `rendezvous-client::bin/rendezvous-test-client` | binary target, no `#[test]` — normal |

The four platform-gated ones cost compile time on every host and prove nothing
off their platform. That is the intended trade (`just check-windows` is what
keeps them from rotting), and it is recorded rather than treated as an
omission.

## Baseline

### CI — one run of three (2026-09-08)

The predecessor merged to `main` as `444213eb5` (PR #69). The first post-merge
`ci` run, `34173378609`, is green on all four legs; its artifacts are under
[`baseline/34173378609/`](baseline/34173378609/) and the gate's own table is
reproduced here. **Two more consecutive green `main` runs are pending** — see
[`baseline/README.md`](baseline/README.md) for why later `main` pushes are a
different source state and what an operator has to do about it.

| Environment | Build/setup | Runner elapsed | Summed duration | Tests | Failures | Skips |
|---|---:|---:|---:|---:|---:|---:|
| `ubuntu-latest` | 695.1 s | 324.9 s | 324.7 s | 2466 | 0 | 0 |
| `macos-latest` | 832.3 s | 753.7 s | 753.5 s | 2466 | 0 | 0 |
| `windows-latest` | 1012.9 s | 356.1 s | 355.8 s | 2105 | 0 | 0 |
| `wsl2-ubuntu` | 81.3 s | 692.7 s | 692.2 s | 2466 | 0 | 0 |

`claudine-cli`'s CI profile runs at `max-threads = 1`, which is why runner
elapsed and summed duration agree to within a second on every leg. The build
column is large because each leg compiles from a cold runner cache; the WSL2 leg
is the exception because its `nextest archive` was built on the host job.
`windows-latest` runs 361 fewer identities: 372 Unix-only tests (every
`#![cfg(unix)]` binary) against 11 Windows-only ones; the three Unix legs carry
identical identity sets. The PR's own `pull_request` run on the identical tree
(`34159725015`) is stored beside it as a supplementary sample: same identities
on every leg, matched-summed ratio 0.93 / 1.05 / 0.86 / 1.00 against the `main`
run — the run-to-run noise a three-run baseline exists to bracket.

### Local — attribution only

The table below is the local attribution Phase 1 recorded, kept in the three
separate columns the plan requires. It is evidence about *this host*, not a
target.

| Gate | Build/setup | Runner elapsed | Summed test duration | Identities |
|---|---:|---:|---:|---:|
| `just test` (L1, five claudine crates) | not separated by the recipe | 36.0 s | 561.0 s | 6,861 run / 11 skipped |
| `just test-cli` (L1) | not separated by the recipe | 18.1 s | 289.1 s | 2,489 run / 10 skipped |
| `just test-l2` | not separated by the recipe | 66.8 s + 2.7 s | 497.4 s | 240 |
| `just test-rendezvous` | not separated by the recipe | 13.9 s total (0.06 + 9.2 + 0.75 s per crate) | 138.5 s | 272 run / 2 skipped |
| `just doctest` | 0.73 s merged compilation | 0.74 s | — | 32 |

**The recipes do not separate build/setup from runner elapsed.** `_test_local_all`
reports only nextest's own summary line, so the build half is invisible in the
local logs. `junit-metrics.ts` derives all three columns from the CI staging
manifest (`duration_s` − `<testsuites time>`), which is why the CI tranche is
the one that can answer the three-column question. Recorded here as a
limitation of the local evidence, not a gap in the plan.

**How the two caps change what these columns mean.** The CI profile runs
`claudine-cli`'s L1 suite at `max-threads = 1` and `claudine`'s at 4, so on CI
the *summed* column is the leg's floor rather than something parallelism hides.
Phase 3's [attribution](attribution.md#the-concurrency-fact-that-decides-which-column-matters)
works through the consequences.

### Budgets

**Not ratified — the input does not exist yet, and the refusal is mechanical.**

Phase 3 ran and answered three of the spec's four draft decisions with
measurements; see [`attribution.md`](attribution.md). The fourth — *what
per-family timing budgets are justified on the existing CI runners* — cannot be
answered from the table above, because a budget derived from a local run is
exactly what the plan's checkpoint forbids, and Phase 1's CI tranche is still
[blocked](log.md#blocked-the-ci-half) on the predecessor's merge.

Rather than publish a provisional number, the refusal is code:
[`attribution.ts`](attribution.ts)'s `deriveBudgets` emits **no** budget and a
named violation for non-CI provenance, for a leg with fewer than three
consecutive green runs, for a declared leg carrying no measurements, and for a
family with fewer samples than runs. `attribution/budgets-pending.json` declares
the four legs at zero runs, so anyone can reproduce the refusal today:

```bash
npx tsx attribution.ts baseline/local-gates/just-test-cli.log \
  --families families.json --budgets attribution/budgets-pending.json
# 4 violation(s): [missing-leg] ubuntu-latest … macos-latest … windows-latest …
#                 wsl2-ubuntu: declared but carries no measurements
# GATE EXIT=1
```

The ratification procedure is fixed **now**, before the numbers exist, so no
budget can be fitted to whatever the candidate run produces
([full statement](attribution.md#the-ratification-procedure-fixed-now)):
per leg and family, `observedMax` is the largest summed duration across that
leg's three consecutive green baseline runs, and `budget = observedMax × 1.25`
rounded up to 0.01 s. Legs are never merged; a miss is reported with its cause
and never closed by adjusting the budget after the fact (AC7).

When Phase 9 fills `perLegFamilySummed` from the stored JUnit artifacts, the
resulting table lands **here**, beside the baseline above, so budget review and
evidence review remain one act (spec RB5).

**Phase 9 status (2026-09-08): still refused, for a smaller reason.** The
provenance is now real — `budgets-pending.json` declares one collected `main`
run per leg (`34173378609`) — but one is not three, and nothing yet joins a
JUnit identity to a family (`attribution.ts` reads nextest logs, not the CI
XML). Both remain open; neither is closed by a local number.

**2026-09-09: the join now exists; the numbers still do not.** The missing half
of the procedure is built —
`test-audit attribute aggregate` (`tools/test-audit/src/attribute/aggregate.ts`,
reachable through this directory's wrapper) walks the stored
`<run-id>/<env>/<tier>/<package>.xml` staging trees and emits
`perLegFamilySummed` in exactly the shape `deriveBudgets` reads, resolving every
`<testcase>` through the reconciler's own `familiesMatching` rather than a
second classifier:

```bash
npx tsx attribution.ts aggregate baseline/34173378609 --out /tmp/agg.json
# GATE EXIT=0 — 18 families on each Unix leg, 15 on windows-latest,
#               summed duration identical to the § CI baseline table
npx tsx attribution.ts budgets --runs /tmp/agg.json
# 4 violation(s):
#   [insufficient-runs] ubuntu-latest: 1 green run(s); 3 consecutive are required
#   … macos-latest … windows-latest … wsl2-ubuntu
# GATE EXIT=1
```

So the refusal moved from `missing-leg` ("nothing joins XML to families") to
`insufficient-runs` ("one run is not three"). That is the whole change: the
*procedure* is complete and reproducible, and the *numbers* are still refused
for want of two more consecutive green `main` runs per leg. No table lands here
until they exist. `attribution/budgets-pending.json` is deliberately left at
`runsPerLeg` 1 with an empty `perLegFamilySummed` — the aggregator's output is
what will replace it, and raising either by hand would be fabricating evidence.

## Family index

Fifty families cover all 7,417 identities exactly once (7,400 at `9fc5151a0`).
The reconciler proves it: `assigned == universe.identities.size`, no identity
matched twice, no family matched nothing. Counts in this table are checked
against the captures by the gate, so a stale number here fails the run.

The **Identities** column tracks the current captures and is the one the gate
checks, so it carries no annotation. Six families moved between the two
revisions:

| Family | At `9fc5151a0` | Now | What moved |
|---|---:|---:|---|
| `lib-unit` | 3,833 | 3,835 | two tests added in the library |
| `cli-l1-fixture` | 282 | 287 | gained `compose_frontmatter_model` |
| `cli-l1-fixture-selftest` | 14 | 30 | gained `contamination_probes` (8) and 8 builder tests |
| `cli-l1-source-scan-guards` | 79 | 73 | `error_guards` 18→8 consolidated, `spawn_site_guard` 15→19 |
| `cli-l1-raw-context-resources` | 88 | 87 | one `context_command` test removed |
| `rz-daemon-integration` | 18 | 19 | one `pairing_and_sync` test added |

**Executed** and **Summed cost** were measured at `9fc5151a0` and are left at
those values throughout this document: the five-run protocol that would restate
them cannot run against this branch's pre-existing red suite, and inventing a
number is worse than a dated one. For those six families the cost column is now
a lower bound rather than a measurement. See [`results.md`](results.md)
§ Measurement re-run for the named blocker.

| Family | Package | Identities | Executed | Summed cost | Disposition |
|---|---|---:|---:|---:|---|
| `catalog-types-unit` | `claudine-catalog-types` | 21 | 21 | 0.29 s | satisfactory |
| `lib-unit` | `claudine` | 3835 | 3833 | 196.58 s | satisfactory |
| `lib-unit-task-shell` | `claudine` | 106 | 106 | 58.14 s | remediation in this fix |
| `lib-unit-atomic-config` | `claudine` | 5 | 5 | 1.55 s | satisfactory |
| `lib-unit-model-catalog` | `claudine` | 71 | 71 | 2.66 s | remediation in this fix |
| `lib-unit-clock-streams` | `claudine` | 24 | 24 | 0.52 s | remediation in this fix |
| `lib-override-seeded-loop` | `claudine` | 1 | 1 | 0.03 s | remediation in this fix |
| `lib-corpus-replay` | `claudine` | 82 | 82 | 3.63 s | satisfactory |
| `lib-structural-guards` | `claudine` | 18 | 18 | 0.42 s | satisfactory |
| `lib-agent-errors-fleet` | `claudine` | 1 | 1 | 0.04 s | satisfactory |
| `lib-override-agent-errors-remediation` | `claudine` | 1 | 1 | 0.13 s | remediation in this fix |
| `lib-model-catalog-integration` | `claudine` | 2 | 2 | 0.03 s | satisfactory |
| `lib-tts-contract` | `claudine` | 5 | 5 | 0.07 s | satisfactory |
| `contract-unit` | `claudine-contract` | 48 | 48 | 0.78 s | satisfactory |
| `contract-real-provider` | `claudine-contract` | 4 | 0 | not run | follow-up: real-tier evidence is pending, see AC6 |
| `cli-unit` | `claudine-cli` | 1178 | 1173 | 34.08 s | satisfactory |
| `cli-unit-child-exec` | `claudine-cli` | 199 | 199 | 7.59 s | satisfactory |
| `cli-unit-completion-engine` | `claudine-cli` | 322 | 322 | 8.01 s | satisfactory |
| `cli-l1-fixture` | `claudine-cli` | 287 | 282 | 75.09 s | satisfactory |
| `cli-l1-fixture-selftest` | `claudine-cli` | 30 | 14 | 2.44 s | remediation in this fix |
| `cli-l1-completion-ui` | `claudine-cli` | 79 | 79 | 3.16 s | satisfactory |
| `cli-l1-source-scan-guards` | `claudine-cli` | 73 | 79 | 39.53 s | remediation in this fix |
| `cli-l1-shipped-corpus` | `claudine-cli` | 6 | 6 | 0.57 s | satisfactory |
| `cli-l1-md-fixture` | `claudine-cli` | 1 | 1 | 0.30 s | satisfactory |
| `cli-l1-perf-budget` | `claudine-cli` | 16 | 12 | 2.34 s | remediation in this fix |
| `cli-l1-raw-compose` | `claudine-cli` | 69 | 69 | 14.86 s | remediation in this fix |
| `cli-l1-raw-sequence-loop` | `claudine-cli` | 154 | 154 | 34.76 s | remediation in this fix |
| `cli-l1-raw-context-resources` | `claudine-cli` | 87 | 83 | 29.07 s | remediation in this fix |
| `cli-l1-pty` | `claudine-cli` | 11 | 11 | 19.19 s | remediation in this fix |
| `cli-l1-pty-interactive` | `claudine-cli` | 19 | 19 | 54.61 s | remediation in this fix: review-1 finding 1 reclassified it L2 -> L1 |
| `cli-l1-live-child` | `claudine-cli` | 1 | 0 | not run | remediation in this fix |
| `cli-l2-lifecycle` | `claudine-cli` | 114 | 114 | 200.35 s | remediation in this fix |
| `cli-l2-render-capture` | `claudine-cli` | 74 | 74 | 188.12 s | remediation in this fix |
| `cli-l2-autocomplete` | `claudine-cli` | 27 | 27 | 44.68 s | satisfactory |
| `cli-l2-ctrl-c` | `claudine-cli` | 3 | 3 | 6.90 s | satisfactory |
| `cli-l3-keyboard` | `claudine-cli` | 4 | 0 | not run | follow-up: L3 needs an operator-attended host, see AC6 |
| `cli-real-provider` | `claudine-cli` | 1 | 0 | not run | follow-up: real-tier evidence is pending, see AC6 |
| `cli-override-loop-rate-limit` | `claudine-cli` | 1 | 1 | 4.04 s | satisfactory |
| `cli-override-empty-state-routing` | `claudine-cli` | 1 | 1 | 0.04 s | remediation in this fix |
| `cli-override-context-width` | `claudine-cli` | 1 | 1 | 5.21 s | remediation in this fix |
| `cli-override-perf-parity` | `claudine-cli` | 2 | 2 | 0.82 s | remediation in this fix |
| `gen-unit` | `claudine-gen` | 93 | 93 | 1.24 s | satisfactory |
| `gen-artifact-pipeline` | `claudine-gen` | 52 | 51 | 5.42 s | remediation in this fix |
| `gen-generate-ux` | `claudine-gen` | 10 | 10 | 8.34 s | satisfactory |
| `gen-l2-report` | `claudine-gen` | 3 | 3 | 2.75 s | satisfactory |
| `rz-core-unit` | `rendezvous-core` | 82 | 82 | 0.91 s | satisfactory |
| `rz-daemon-unit` | `rendezvous-daemon` | 153 | 153 | 107.75 s | remediation in this fix |
| `rz-daemon-integration` | `rendezvous-daemon` | 19 | 16 | 25.46 s | remediation in this fix |
| `rz-client-unit` | `rendezvous-client` | 14 | 14 | 0.18 s | satisfactory |
| `rz-client-integration` | `rendezvous-client` | 7 | 7 | 4.21 s | satisfactory |

*Executed* is what Phase 1's gate logs actually ran on this host; a shortfall
against *Identities* is either a `#[ignore]`, a feature the local recipe does
not enable, or a tier the local gates do not run. Every shortfall is explained
in the family's row below. **No family was dispositioned by timing threshold.**

## Families

Fields per row: purpose; assertion quality (does it distinguish a plausible
failure?); shared helpers; required effects; CWD/home/cache/environment/tool
dependencies; timing floor; runner overrides in force; resource ownership;
tier, features and platforms; canonical recipe; observed cost with provenance;
disposition.

Where a family's rows share every field, they share a row here and the members
are enumerated. Where a member differs in setup or in what it proves, it has
its own family — that is what the six `*-override-*`, `lib-unit-task-shell`,
`lib-unit-atomic-config`, `lib-unit-model-catalog`, `lib-unit-clock-streams`
and `cli-unit-child-exec` carve-outs are.

### `claudine-catalog-types`

#### `catalog-types-unit` — 21 identities

- **Members** — the whole `claudine-catalog-types` lib suite. Wire-shape and
  serde round-trip tests for the catalog value types.
- **Purpose** — prove the catalog's serialized forms match the shipped schema
  shape.
- **Assertion quality** — good: each asserts an exact serialized form, so a
  field rename or a changed default is distinguishable. No tautologies found.
- **Shared helpers** — none.
- **Effects / dependencies** — none. Pure in-process, no CWD, home, cache,
  environment or tool dependency.
- **Timing** — summed 0.29 s, max 0.02 s. No floor.
- **Overrides** — none.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test` (parent area).
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

### `claudine` (library)

#### `lib-unit` — 3,835 identities

- **Members** — the `claudine` lib suite minus the four carve-outs and the
  override target below. By top-level module:
  `composition` 1,171 (after the 106-test `sequence::task` carve-out),
  `stream` 810, `dispatch` 209, `config` 199, `render` 162, `linking` 183,
  `protect` 127, `harness` 112, `permissions` 98, `diagnostics` 84,
  `messaging` 81, `signals` 79, `mcp` 76, `runaway` 74, `provider` 72,
  `hook_adapters` 68, `system_prompt` 58, `events` 41, `actions` 34,
  `reporting` 34, `invocation_context` 28, `error` 16, `opencode_config` 9,
  `child_environment` 8, `path_semantics` 3.
  Within `composition`: `lifecycle` 390, `sequence` 151, `looping` 139,
  `error` 103, `prepare` 75, `schema` 75, `select` 42, `coordinator` 38,
  `preflight` 29, `closure` 27, `frontmatter_excerpt` 23, `resolve` 21,
  `mismatch` 19, `runtime_state` 13, `file_detail` 8, `agent_message` 5,
  `guardrails` 5, `interpolation_conformance` 4, `hints` 3.
- **Purpose** — in-process unit coverage of the library's parsing, rendering,
  dispatch, configuration and provider-protocol logic.
- **Assertion quality** — sampled, not exhaustively reviewed at this
  granularity; the sample found no tautology. The two structural guards
  (`test_placement`, `boundary_lint`) already enforce placement and layering
  for this population, so quality review is targeted rather than blanket.
  Phase 6 owns any repair found during attribution.
- **Shared helpers** — `tempfile`, `serial_test`, `rstest`, `proptest`,
  `tracing-test`, `test-toolkit`, `fs4` (dev-dependencies). 46 `#[serial]`
  sites across 16 modules; `serial_test` is a no-op across nextest processes,
  so those coordinate only within one binary.
- **Effects / dependencies** — mostly none. A minority build a temporary
  directory through `tempfile`. No test in this family requires a repository,
  a home directory, a network endpoint or an external tool.
- **Timing** — summed 196.58 s across 3,833 identities (51 ms mean); slowest
  is `composition::sequence::preflight::tests::shell::bracket_target_identity_is_rejected_on_every_graph_shell_surface`
  at 6.35 s. No intentional floor.
- **Overrides** — the CI profile binds this family to the `claudine-l1`
  test-group (`max-threads = 4`). No per-test override survives here after the
  carve-outs.
- **Resource ownership** — none beyond per-test temporary directories.
- **Route** — L1, no features, all platforms; `just test` / `just test-library`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory. The 6.35 s outlier is Phase 3 attribution
  input, not a Phase 2 finding.

#### `lib-unit-task-shell` — 106 identities

- **Members** — `composition::sequence::task::*` (`tests.rs`, `shell.rs`,
  `shell/tests.rs`).
- **Purpose** — prove the sequence task shell runner captures output, honors
  timeouts, and reaps backgrounded descendants.
- **Assertion quality** — strong. `the_system_shell_kills_a_backgrounded_descendant_holding_stdout`
  distinguishes exactly the defect a naive `wait()` produces.
- **Shared helpers** — none shared with `cli/tests`; the family builds its own
  shell commands.
- **Effects / dependencies** — spawns **real system shells** (`sh`/`cmd`) and
  real descendants; needs a working shell on `PATH`. Ten sleep sites live here
  (8 in `tests.rs`, 2 in `shell.rs`).
- **Timing** — summed 58.14 s across 106 identities (**548 ms mean — 11× the
  `lib-unit` mean**), max 2.06 s. The sleeps are part of the floor.
- **Overrides** — none; the family fits the default ceiling.
- **Resource ownership** — owns child processes and process groups; on Unix it
  reaches descendants with `kill(-pgid, …)`.
- **Route** — L1, no features. Six identities exist as `cfg(unix)`/`cfg(windows)`
  pairs, so one arm of each is platform-excluded (see the exclusion table).
  `just test` / `just test-library`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — **remediation in this fix** (Phase 7). The sleeps are
  readiness waits, not timeout contracts, in at least the reaping cases; each
  needs bounded observation of the final required condition with a deadline.

#### `lib-unit-atomic-config` — 5 identities

- **Members** — `config::atomic::*`.
- **Purpose** — prove `atomic_write`'s last-rename-wins contract and its
  Windows transient-error retry policy.
- **Assertion quality** — strong; `concurrent_writers_produce_intact_payload`
  distinguishes a torn write from a serialized one.
- **Shared helpers** — `tempfile`, `fs4`.
- **Effects / dependencies** — real filesystem writes in a temporary directory;
  one `sleep` in the retry path.
- **Timing** — summed 1.55 s, max 1.20 s (`concurrent_writers_produce_intact_payload`).
- **Overrides** — none.
- **Resource ownership** — temporary files, removed on drop.
- **Route** — L1, no features. Four of the nine source-side tests here are
  `cfg(windows)` and run only on `windows-latest`. `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory. The 1.20 s is a genuine concurrency proof.

#### `lib-unit-model-catalog` — 71 identities

- **Members** — `model_catalog::*`.
- **Purpose** — prove catalog refresh, provider-source merging and staleness
  handling.
- **Assertion quality** — good. `refresh_blocking_does_not_panic` is the weak
  one: a panic-absence assertion distinguishes very little.
- **Shared helpers** — `serial_test` (4 sites in `service/tests.rs`).
- **Effects / dependencies** — two `sleep` sites (`service/tests.rs`,
  `provider_sources.rs`); temporary catalog files.
- **Timing** — summed 2.66 s, max 1.28 s (`service::tests::refresh_blocking_does_not_panic`).
- **Overrides** — none.
- **Resource ownership** — a process-global catalog cache, which is why the
  `#[serial]` sites exist; that coordination is intra-binary only.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — **remediation in this fix** (Phase 6/7): replace the two
  readiness sleeps with bounded observation, and replace
  `refresh_blocking_does_not_panic` with an assertion on the refreshed
  catalog's content.

#### `lib-unit-clock-streams` — 24 identities

- **Members** — `render::assistant_stream::*` (16), `render::thinking_stream::*` (8).
- **Purpose** — prove the streaming renderers flush on idle and reset their
  growth clocks correctly.
- **Assertion quality** — good; each asserts the rendered frame, not merely
  that a flush happened.
- **Shared helpers** — none.
- **Effects / dependencies** — five `sleep` sites drive the idle clock.
- **Timing** — summed 0.52 s, max 0.06 s. Cheap despite the sleeps.
- **Overrides** — none.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — **remediation in this fix** (Phase 7). Cost is not the
  problem; the sleeps are the timeout contract in some cases and a readiness
  wait in others, and the inventory cannot tell which without the audit
  Phase 7 owns. Carved out so that audit has an addressable row.

#### `lib-override-seeded-loop` — 1 identity

- **Member** — `composition::looping::engine::tests::seed_state::seeded_loop_repro_runs_to_completion_with_live_derived_variable`.
- **Purpose** — replay a seeded loop against live derived state.
- **Assertion quality** — good; it is a named regression repro.
- **Shared helpers** — none.
- **Effects / dependencies** — resolves live derived state.
- **Timing** — 0.03 s observed on this host.
- **Overrides** — **two dead ones.** Both profiles carry
  `test(=composition::loop_engine::tests::seeded_loop_repro_runs_to_completion_with_live_derived_variable)`.
  The module was renamed `loop_engine` → `looping::engine` (and a `seed_state`
  module inserted); `test(=…)` is exact-match and nextest does not warn on a
  filter that selects nothing, so **neither override has bound to anything
  since the rename**. See [Runner overrides](#runner-override-census).
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — **remediation in this fix** (Phase 6). The test runs in
  0.03 s here, so the override is not protecting anything measurable; both
  entries are removal candidates under AC5, not repair candidates.

#### `lib-corpus-replay` — 82 identities

- **Members** — `protocol_fixture_replay` (9), `semantic_fidelity` (35),
  `typed_stream_protocols` (23), `kimi_wire` (6), `opencode_stderr_lifecycle`
  (6), `deprecated_compatibility` (3).
- **Purpose** — replay recorded provider wire fixtures through the production
  stream parsers and assert the normalized lifecycle events.
- **Assertion quality** — strong, and this is the house shape the spec names:
  one binary loading a corpus once, extended per regression rather than
  rescanned per case.
- **Shared helpers** — `lib/tests/fixtures/`.
- **Effects / dependencies** — reads the committed fixture corpus from the
  checkout; no network, no child process, no home directory.
- **Timing** — summed 3.63 s across 82 identities, max 0.21 s.
- **Overrides** — none.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

#### `lib-structural-guards` — 18 identities

- **Members** — `boundary_lint` (7), `canonical_dispatch` (7),
  `diagnostic_detail_conformance` (4).
- **Purpose** — fail the build when a layering boundary, a dispatch route or a
  diagnostic detail drifts from its declared contract.
- **Assertion quality** — strong; each names the offending symbol.
- **Shared helpers** — their own source scanners.
- **Effects / dependencies** — read `lib/src` from the checkout.
- **Timing** — summed 0.42 s, max 0.10 s. Cheap because they scan a single
  crate's sources, in contrast with the CLI-side scan family below.
- **Overrides** — none.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

#### `lib-agent-errors-fleet` — 1 identity

- **Member** — `agent_errors_fleet::fleet_lifecycle_config_parses`.
- **Purpose** — prove the shipped agent-errors fleet document parses as a
  lifecycle config.
- **Assertion quality** — adequate; it asserts a parse, which is what the
  shipped artifact must satisfy.
- **Shared helpers** — none.
- **Effects / dependencies** — reads a shipped prompt document.
- **Timing** — 0.04 s.
- **Overrides** — none.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

#### `lib-override-agent-errors-remediation` — 1 identity

- **Member** — `agent_errors_fleet::exhausted_remediation_fails_finalize_and_preserves_findings`.
- **Purpose** — prove an exhausted remediation loop still fails `finalize` and
  preserves its findings.
- **Assertion quality** — strong; it asserts the preserved findings, not just
  the exit.
- **Shared helpers** — none.
- **Effects / dependencies** — several complete lifecycle passes.
- **Timing** — **0.13 s on this host.** The default-profile override that
  covers it cites "~21s alone on an 8-core dev machine and 83s under full-suite
  contention" (`.config/nextest.toml`, CI profile note). That was measured
  before the predecessor's fixture work; the current local number is three
  orders of magnitude smaller.
- **Overrides** — `[[profile.default.overrides]] test(=exhausted_remediation_fails_finalize_and_preserves_findings)`
  → `slow-timeout = { period = "30s", terminate-after = 3 }`. Default profile
  only; under `NEXTEST_PROFILE=ci` the global 30 s × 3 already applies.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — **remediation in this fix** (Phase 6). The override's own
  justification no longer matches the measurement; AC5 requires it to be
  re-justified against a current contract or removed. Do not remove it on this
  host's number alone — CI evidence decides.

#### `lib-model-catalog-integration` — 2 identities

- **Members** — `model_catalog_integration` (2).
- **Purpose** — prove configuration overrides propagate into catalog validation.
- **Assertion quality** — good.
- **Shared helpers** — none.
- **Effects / dependencies** — temporary config files.
- **Timing** — summed 0.03 s.
- **Overrides** — none.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

#### `lib-tts-contract` — 5 identities

- **Members** — `tts_phase1_contract` (3), `tts_phase5_contract` (2).
- **Purpose** — prove the shipped claudine artifacts enable detached native
  audio and that the TTS contract surface is stable.
- **Assertion quality** — good; asserts the shipped artifact's content.
- **Shared helpers** — none.
- **Effects / dependencies** — reads shipped artifacts. **No audio device is
  touched and no terminal or window gains focus.**
- **Timing** — summed 0.07 s.
- **Overrides** — none.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

### `claudine-contract`

#### `contract-unit` — 48 identities

- **Members** — the `claudine-contract` lib suite.
- **Purpose** — prove the `InferenceAdapter` implementation's argv assembly,
  stream draining, schema validation and error mapping.
- **Assertion quality** — good; `runner_drains_stderr_concurrently_without_deadlock`
  distinguishes the deadlock it names.
- **Shared helpers** — `tokio` multi-thread runtime, `tracing-subscriber`,
  `tempfile`.
- **Effects / dependencies** — a temporary session working directory; no
  provider binary and no network.
- **Timing** — summed 0.78 s, max 0.06 s.
- **Overrides** — none.
- **Resource ownership** — per-test temporary directories.
- **Route** — L1, no features, all platforms; `just test` / `just test-contract`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

#### `contract-real-provider` — 4 identities

- **Members** — `real_provider` (4), gated `required-features = ["real-tests"]`.
- **Purpose** — drive an installed, authenticated provider end to end.
- **Assertion quality** — unreviewed against a live run; the tier has no
  evidence in this fix yet.
- **Shared helpers** — none.
- **Effects / dependencies** — `CLAUDINE_CONTRACT_REAL=1`, a provider binary on
  `PATH`, provider credentials, network. Each case skips cleanly when its
  provider or model is unavailable.
- **Timing** — unknown; not run.
- **Overrides** — none.
- **Resource ownership** — a real provider session.
- **Route** — real tier, `real-tests` feature; `just test-real`. **That recipe
  shells out to `cargo test`, not nextest** — the only recipe in the area that
  does, so this family produces no JUnit report and no nextest identity list.
- **Cost provenance** — none; not executed by any Phase 1 gate.
- **Disposition** — **follow-up**: real-tier evidence is pending (AC6 records
  unavailable runtime evidence as pending, not passing). The `cargo test`
  route is a separate finding, resolved in Phase 6.

### `claudine-cli` — in-crate unit tests

#### `cli-unit` — 1,178 identities

- **Members** — the `claudine-cli` bin suite minus `commands::wrap::exec` and
  `completion`. By module: `commands` 988 (`wrap` 729 after the `exec`
  carve-out, `config_tui` 55, `compose` 53, `dashboard` 28,
  `schema_interactive` 26, `context_render` 22, `context` 13, `handle` 10,
  `init` 9, `exec_prep` 8, `providers` 7, `hooks` 5, `sequence` 5, `logs` 4,
  `errors` 4, `init_wizard` 3, `signals` 3, `help` 2, `actions` 1, `skills` 1),
  `argv` 73, `output` 58, `perf` 40, `telemetry` 10, `log` 7, `cli_utils` 1,
  `provider_values` 1. Within `commands::wrap`: `harness_orch` 165,
  `profile` 122, `composition` 112, `live_semantic_sink` 92, `flags` 27,
  `launch_plan` 26, `env` 23, `sequence` 20, `repo_home` 19, `system_prompt` 18,
  `runaway_guard` 17, `wrapper_stages` 17, `selection_ui` 15, `stream_io` 10,
  `session_report` 8, `policy` 7, `section` 7, `tests` 6, `inline` 5,
  `prompt_source` 5, `wrapper_mcp` 5, `resume` 2, `overlay` 1.
- **Purpose** — in-process unit coverage of argv normalization, wrap
  orchestration, provider profiles, rendering and telemetry.
- **Assertion quality** — sampled; no tautology found in the sample.
- **Shared helpers** — `serial_test` (56 sites across 17 modules), `tempfile`.
- **Effects / dependencies** — mostly none; a minority build temporary
  workspaces. No child `claudine` process.
- **Timing** — summed 34.08 s across 1,173 executed identities (29 ms mean),
  max 1.04 s.
- **Overrides** — CI binds this family to `claudine-cli-ci-l1`
  (`max-threads = 1`). No per-test override.
- **Resource ownership** — per-test temporary directories.
- **Route** — L1. **Five identities are `daemon-tests`-gated** (1,178 listed vs
  1,173 executed under the bare local recipe); those five are what the entire
  bundled-DuckDB dependency buys. All platforms; `just test` / `just test-cli`,
  and `just test-daemon` for the five.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

#### `cli-unit-child-exec` — 199 identities

- **Members** — `commands::wrap::exec::*` (`spawn`, `termination`, `watchdog`,
  `timeouts`, `stream_capture`).
- **Purpose** — prove child spawning, capture, timeout and termination,
  including the Windows job-object path.
- **Assertion quality** — strong; `run_child_capture_wall_clock_timeout_reaps_child`
  distinguishes a leaked child from a reaped one.
- **Shared helpers** — none; the family builds its own child commands.
- **Effects / dependencies** — spawns **real child processes**; reads and
  writes their pipes; on Windows uses job objects and console control events.
- **Timing** — summed 7.59 s across 199 identities, max 1.22 s.
- **Overrides** — CI `claudine-cli-ci-l1` group only.
- **Resource ownership** — owns child processes; nextest's leak policy is the
  proof that they are reaped.
- **Route** — L1, no features. Five identities are `cfg(windows)` and run only
  on `windows-latest`; `just check-windows` is the only local gate that
  compiles them.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory. Carved out of `cli-unit` because it owns
  processes and `cli-unit` does not — a different cleanup contract, not a
  different subject.

#### `cli-unit-completion-engine` — 322 identities

- **Members** — `completion::*` (`engine` 53, `setter_value` 37,
  `composition` 36, `schema_completion` 33, `frontmatter` 29,
  `operation_file` 26, `root_menu` 25, `bootstrap` 22, `walker` 15,
  `scopes` 14, `autocomplete_ui` 13, `fuzzy` 12, `default_glob` 5, `tests` 2).
- **Purpose** — prove completion candidate generation, ranking, scoping and
  the autocomplete UI's hyperlink encoding.
- **Assertion quality** — good; candidate-set assertions are exact.
- **Shared helpers** — temporary workspaces built in-module.
- **Effects / dependencies** — walks a temporary directory tree; no child
  process.
- **Timing** — summed 8.01 s across 322 identities, max 0.25 s.
- **Overrides** — CI `claudine-cli-ci-l1` group only.
- **Resource ownership** — temporary directories.
- **Route** — L1, no features. Three identities are `cfg(windows)`; one
  (`repository_suggestions_do_not_follow_directory_symlinks`) is a
  unix/windows pair.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory. Carved out of `cli-unit` because it is the
  single largest filesystem-walking cohort in the binary and Phase 6's
  discovery-cost audit needs it addressable.

### `claudine-cli` — L1 integration binaries

The predecessor's migration split this population in two, and the split is the
axis this fix works along. Thirty-two binaries build their child through
`CliProcessFixture`; thirty-four still spawn `std::process::Command` and set
their own environment; those thirty-four are exactly the
`SPAWN_ALLOWLIST` population, and they are grouped below into the three
batches Phase 5 migrates.

**Authoritative spawn census, re-read at this revision** (from the guard's own
artifact, `enumeration/recipes/spawn-site-burn-down.jsonl`):

| Reason | Files | Sites |
|---|---:|---:|
| `outside this fix's scope` | 34 | 170 |
| `needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither` | 2 | 2 |
| **total allow-listed** | **36** | **172** |
| scanned sites (all governed files) | — | 172 |
| governed files (spawn gate) | 89 | — |
| governed files (isolation gate) | 37 | — |
| isolation escapes | 0 | 0 |

**This supersedes the plan's grounding fact of 170 sites / 168 out-of-scope /
83 governed / 32 isolation-governed.** The guard says 172 / 170 / 89 / 37. The
plan's per-file census table is accurate except for
`level1_structured_error_message.rs`, which holds **1** site, not 2; its stated
totals were the predecessor's historical numbers, and its own table summed to
173 rather than the 170 it claimed. Phase 5 must re-read the artifact, not the
table.

#### `cli-l1-fixture` — 287 identities

- **Members** (33 binaries) — `agent_cwd` 4, `argv_normalization` 14,
  `characterization_error_routes` 8, `command_routing` 8,
  `compose_caller_file_provenance` 15, `compose_frontmatter_model` 5,
  `compose_header_first` 5,
  `contextual_errors` 4, `ctx_launch_anchor` 8, `detached_audio` 1,
  `handle_repo_config` 2, `hooks_cli` 8, `inline_compose_sequence_mismatch` 17,
  `mcp_cli` 17, `prompt_reporting` 12, `propagated_context_fixtures` 5,
  `sequence_schema` 6, `wrap_antigravity_exit_signal` 1, `wrap_basics` 24,
  `wrap_compose_agent` 13, `wrap_compose_exec` 8, `wrap_compose_preflight` 7,
  `wrap_compose_validation` 18, `wrap_direct_argv` 2,
  `wrap_incomplete_subagents` 6, `wrap_inline_compose` 17,
  `wrap_inline_compose_interactive` 2, `wrap_opencode` 15,
  `wrap_opencode_models` 7, `wrap_provider_flags` 5, `wrap_structured_stream` 12,
  `wrap_watchdog_startup_stall` 5, `wrap_watchdog_timeout` 6.
- **Purpose** — drive the real `claudine` binary end to end for wrap,
  compose, MCP, hooks and provider-routing behavior.
- **Assertion quality** — good; these assert stdout/stderr content and exit
  codes, and the migration added the isolation the assertions depend on.
- **Shared helpers** — `common/mod.rs` (`CliProcessFixture`,
  `ClaudineCommandBuilder`, `scrub_inherited_environment`, the containment
  guard), `common/wrap.rs`, `common/incomplete_subagents.rs`.
- **Effects / dependencies** — spawns the `claudine` binary with a **cleared**
  environment, a pinned CWD outside the checkout, pinned home variables and a
  composed `PATH`; some build a real git repository as the assertion's subject.
  No network, no provider binary, no user configuration.
- **Timing** — summed 75.09 s across 282 identities (266 ms mean), max 4.33 s
  (`wrap_watchdog_startup_stall::watchdog_startup_bytes_move_the_deadline`,
  which is a deliberate deadline contract).
- **Overrides** — CI `claudine-cli-ci-l1` group. One member,
  `command_routing::agents_and_commands_route_to_empty_state_messages`, carries
  a per-test override and is a family of its own below.
- **Resource ownership** — one child process per case, reaped by `assert_cmd`;
  one temporary fixture root per case, outside the checkout even after
  canonicalization.
- **Route** — L1, no features. Seven identities in `wrap_compose_agent` are
  macro-generated and `cfg(unix)`. All platforms otherwise;
  `just test` / `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory. The spec is explicit that already-migrated
  tests get evaluation and regression verification, not a second rewrite.
  Two Windows-only unused-import warnings in `wrap_basics.rs` and
  `compose_caller_file_provenance.rs` (Phase 1 finding 1) are Phase 5's to
  clear alongside the change that motivates them.

#### `cli-l1-fixture-selftest` — 30 identities

- **Members** — `cli_process_fixture` (22), `contamination_probes` (8).
- **Purpose** — prove the fixture builder itself: the environment scrub, the
  named escapes (`fake_only_path`, `host_path`, `ambient_context`,
  `inherit_no_env`), and the checkout-containment precondition.
  `cli_process_fixture` proves the mechanism against a recording stub;
  `contamination_probes` (AC3) proves the consequence, by exporting each
  scrubbed family in the parent and asserting an ordinary `claudine compose`
  run still produces the same observable result.
- **Assertion quality** — strong, and load-bearing: this is the only place the
  builder's contract is asserted rather than assumed.
  `ambient_context_escape_pins_the_cwd_to_a_test_built_repository` distinguishes
  a fixture that leaked the developer's CWD.
- **Shared helpers** — `common/mod.rs`, `common/wrap.rs`.
- **Effects / dependencies** — spawns `claudine` and a recording stub.
- **Timing** — summed 2.44 s, max 0.26 s, measured at `9fc5151a0` over the
  then-14 identities.
- **Overrides** — CI `claudine-cli-ci-l1` group.
- **Resource ownership** — child processes and temporary fixture roots.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — **remediation in this fix** (Phase 4), **done**. The
  builder gained its `std::process::Command` surface and this family gained the
  drift tests that prove the two surfaces produce the same effective
  environment — `both_command_surfaces_hand_the_child_the_same_environment`,
  `both_command_surfaces_clear_the_environment_the_same_way`, and the two that
  assert a policy identically *wrong* on both would still fail
  (`..._disable_rendezvous_reporting_over_an_enabled_parent`,
  `..._keep_audio_out_of_the_developers_machine`) — plus the raw surface's
  escape and containment arms. The Windows arm compiles via `cfg!(windows)`.

#### `cli-l1-completion-ui` — 79 identities

- **Members** — `completion_cli` 14, `completion_compose` 32,
  `completion_inline_compose` 8, `completion_sequence` 8, `completion_setter` 17.
- **Purpose** — prove shell-completion candidate output through the real
  binary's `complete` route.
- **Assertion quality** — good; exact candidate lists.
- **Shared helpers** — `common/mod.rs`, `common/completion.rs`.
- **Effects / dependencies** — spawns `claudine` with a seeded temporary
  workspace and a fake home.
- **Timing** — summed 3.16 s across 79 identities (40 ms mean), max 0.10 s.
  The cheapest process-spawning family in the area.
- **Overrides** — CI `claudine-cli-ci-l1` group.
- **Resource ownership** — child processes, temporary workspaces.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

#### `cli-l1-source-scan-guards` — 73 identities

- **Members** — `composition_seams` 19, `spawn_site_guard` 19,
  `dispatch_inventory` 12, `test_placement` 9, `error_guards` 8,
  `spawn_inventory` 2, `run_harness_loop_call_sites` 2,
  `diagnostic_discovery` 2. `error_guards` was 18 at `9fc5151a0`: Phase 5's
  consolidation replaced twelve per-guard cases that each re-parsed the corpus
  in its own process with two that parse it once
  (`production_sources_pass_every_scan_backed_guard` plus a named-failure arm).
  `spawn_site_guard` gained four detector tests when review 1's finding 1 made
  it classify by resource instead of filename.
- **Purpose** — structural guards over `lib/src`, `cli/src` and `cli/tests`:
  typed errors must not collapse to prose, dispatch must stay inventoried,
  spawns must go through the fixture, tests must sit in the declared place.
- **Assertion quality** — strong and specific; each names the offending file
  and site. `spawn_site_guard` additionally carries negative detector tests and
  stale-exemption failure arms.
- **Shared helpers** — `common/source_scan.rs`, `error_guards/source_scan.rs`
  (a 50 KB `syn`-based scanner), `error_guards/*.toml` allow-lists.
- **Effects / dependencies** — read the checkout's own sources. `error_guards`
  parses them with `syn` rather than grepping, which is why it costs what it
  costs; it spawns no subprocess.
- **Timing** (measured at `9fc5151a0`, before the consolidation below) —
  **summed 39.53 s across the then-79 identities — 500 ms mean, ten times the
  `lib-unit` mean.** `error_guards` alone was 34.01 s over 18 cases, max 3.21 s
  (`every_code_a_diagnostic_claims_is_a_registered_code`, one of the twelve
  cases Phase 5 folded into a single corpus parse). The post-consolidation cost
  is not restated here — see [`results.md`](results.md) § Measurement re-run.
  `spawn_inventory` is 1.91 s over 2. Nextest gives each case its own process,
  so a process-local cache shares nothing: **the corpus is re-scanned once per
  case**, and the 18 `error_guards` cases each pay a full parse.
- **Overrides** — CI `claudine-cli-ci-l1` group.
- **Resource ownership** — none.
- **Route** — L1, no features, all platforms; `just test-cli`, and
  `error_guards` separately via `just lint-transport`.
- **Cost provenance** — `baseline/local-gates/just-test.log` and
  `just-test-cli.log` (34.01 s and 30.07 s for `error_guards` respectively —
  the spread is scheduler contention, not a different workload).
- **Disposition** — **remediation in this fix** (Phase 6). This is the clearest
  repeated-work candidate in the area and the draft decision the spec asks
  about ("which source scans can share work without losing independent failure
  detail"). The house shape — one passive corpus binary that scans once and is
  *extended* per regression — applies directly, but consolidation must keep
  each guard's independent diagnostic attribution and selective execution;
  Phase 3 measures before Phase 6 decides.

#### `cli-l1-shipped-corpus` — 6 identities

- **Members** — `shipped_prompt_contract` 3, `shipped_prompt_route_drift` 3.
- **Purpose** — prove the shipped prompt corpus parses, routes as declared, and
  has not drifted from its committed routes.
- **Assertion quality** — good; this is the passive-corpus shape the spec
  endorses, and `shipped_prompt_contract` retains a real-binary case.
- **Shared helpers** — `common/mod.rs`, `write_executable`.
- **Effects / dependencies** — reads the shipped prompt corpus from the
  checkout; `shipped_prompt_contract` spawns `claudine`.
- **Timing** — summed 0.57 s, max 0.42 s.
- **Overrides** — CI `claudine-cli-ci-l1` group.
- **Resource ownership** — child processes for the contract half.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — satisfactory. Its sibling `shipped_prompts` sits in
  `cli-l1-raw-context-resources` because it still spawns raw; the two rejoin
  after Phase 5C.

#### `cli-l1-md-fixture` — 1 identity

- **Member** — `inline_compose_hash::inline_compose_writes_hash_that_passes_md_diff`.
- **Purpose** — prove the Darkmatter content hash claudine writes round-trips
  through `md diff`.
- **Assertion quality** — strong; it is the only test that proves the
  cross-tool hash contract.
- **Shared helpers** — `common/mod.rs`.
- **Effects / dependencies** — **requires the `md` binary**, built by the area's
  `_ensure-md` pre-build into `../target/debug/md` or found on `PATH`. On the
  toolchain-free WSL2 archive leg `_ensure-md` warns and this one test fails.
- **Timing** — 0.30 s.
- **Overrides** — CI `claudine-cli-ci-l1` group.
- **Resource ownership** — a `claudine` child and an `md` child.
- **Route** — L1, no features, all platforms except the WSL2 prebuilt-archive
  leg; `just test-cli` (via `_ensure-md`).
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — satisfactory. The WSL2 failure is a known, documented
  limitation of the archive route, recorded in the `_ensure-md` recipe.

#### `cli-l1-perf-budget` — 16 identities, 12 executed

- **Members** — `wrap_perf` 11 (minus the 2 override targets below → 9),
  `sequence_perf` 3, `system_prompt_perf_bench` 4.
- **Purpose** — assert wall-clock and structural budgets on the perf-report
  rendering paths.
- **Assertion quality** — mixed. The parity assertions
  (`…_perf_stdout_matches_non_perf`) are strong. The four
  `system_prompt_perf_bench` cases are `#[ignore]`d harnesses — they are
  measurement tools, not tests, and they assert nothing in an ordinary run.
- **Shared helpers** — `common/mod.rs`; `system_prompt_perf_bench` uses a raw
  `std::process::Command::new("git")` for its parent-side setup, which is why
  the isolation gate deliberately does not govern it.
- **Effects / dependencies** — spawns `claudine`; `system_prompt_perf_bench`
  additionally shells out to `git`.
- **Timing** — summed 2.34 s across 12 executed identities, max 0.33 s.
- **Overrides** — `compose_perf_stdout_matches_non_perf` and
  `inline_compose_perf_stdout_matches_non_perf` carry CI-profile overrides and
  are a separate family below.
- **Resource ownership** — child processes.
- **Route** — L1, no features, all platforms. **Four identities are
  `#[ignore]`d** and run only under `cargo nextest run --ignored`, which no
  canonical recipe invokes.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — **remediation in this fix** (Phase 6). The four ignored
  benchmark harnesses are unreachable coverage that reads as coverage; they
  belong either in `lib/benches` beside the criterion suite or behind an
  explicit recipe. Recorded here, decided in Phase 6.

#### `cli-l1-raw-compose` — 69 identities (Phase 5A)

- **Members** — `compose_schema_cli` 35, `composition_outputs` 14,
  `compose_removed_validation_keys` 8, `compose_interactive_timeout_cli` 5,
  `compose_cli` 4, `inline_compose_cli` 2, `compose_system_prompt_lifetime` 1.
  Allow-listed spawn sites: 20 + 2 + 1 + 5 + 4 + 2 + 1 = **35**.
- **Purpose** — prove compose and inline-compose behavior through the real
  binary: schema handling, removed-key diagnostics, output accumulation,
  system-prompt lifetime.
- **Assertion quality** — good in subject, **fragile in setup**. Two members
  fail today under an inherited terminal width: with `COLUMNS=44`,
  `compose_schema_cli::inline_compose_wrong_type_prompt_takes_precedence_over_schema_scrub`
  and
  `composition_outputs::a_loop_accumulates_outputs_and_retains_mutations_across_iterations`
  redden. That is the predecessor's recorded residual and AC3's named case.
- **Shared helpers** — `common/mod.rs`, `common/completion.rs` — but **not**
  the builder: each site constructs `std::process::Command` and sets its own
  environment.
- **Effects / dependencies** — spawns `claudine` with **inherited** environment
  except for what each site sets by hand; therefore inherits `COLUMNS`,
  `FORCE_COLOR`, `CLAUDINE_*`, Git plumbing and the developer's home.
- **Timing** — summed 14.86 s across 69 identities (215 ms mean), max 0.92 s.
- **Overrides** — CI `claudine-cli-ci-l1` group.
- **Resource ownership** — child processes; fixture roots not guaranteed
  outside the checkout.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — **remediation in this fix** (Phase 5A). Migration to
  `CliProcessFixture::command()` closes the two inherited-width failures and
  deletes seven `SPAWN_ALLOWLIST` entries.

#### `cli-l1-raw-sequence-loop` — 154 identities (Phase 5B)

- **Members** — `sequence_cli` 30 (minus nothing), `sequence_errors_cli` 29,
  `sequence_sources_cli` 22, `sequence_groups` 20, `loop_cli` 18 (minus the
  override target → 17), `wrap_sequence_composition` 16, `sequence_jit` 13,
  `sequence_magic_reference` 4, `sequence_prompt_property` 3.
  Allow-listed spawn sites: 20 + 1 + 1 + 4 + 18 + 9 + 1 + 4 + 3 = **61**.
- **Purpose** — prove sequence and loop execution through the real binary:
  step ordering, per-step state, group contention, JIT expansion, magic
  references, error routes.
- **Assertion quality** — good; `sequence_injects_per_step_state_into_prompt`
  and the group-contention cases distinguish real ordering defects.
- **Shared helpers** — `common/mod.rs`, `strip_ansi`.
- **Effects / dependencies** — spawns `claudine` raw. **`sequence_cli.rs` alone
  carries roughly twenty `.env("PATH", augmented_path(…))` pairs**, each an
  ad-hoc reimplementation of one of the builder's named escapes.
- **Timing** — summed 34.76 s across 154 identities (226 ms mean), max 2.34 s
  (`compose_loop_step_timeout_surfaces_as_iteration_failure`, a deadline
  contract).
- **Overrides** — CI `claudine-cli-ci-l1` group;
  `loop_cli::compose_loop_rate_limit_pause_waits_then_continues` is a separate
  family below.
- **Resource ownership** — child processes; some sequences spawn grandchildren.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — **remediation in this fix** (Phase 5B). Each raw
  `PATH` pair must be replaced by the named escape that expresses its actual
  need (`host_path()` where a real tool is the subject, `fake_only_path()`
  where absence is the assertion, the default otherwise) — not carried across.

#### `cli-l1-raw-context-resources` — 87 identities, 83 executed (Phase 5C)

- **Members** — `context_command` 26 (minus the override target → 25),
  `skills_integration` 22, `completion_contract` 10, `completion_perf` 9,
  `errors_command` 5, `effective_diagnostic_render` 5,
  `completion_resolution_round_trip` 2, `handle_deadline` 2, `shipped_prompts` 2,
  `compose_ttff_perf` 1, `handle_blocking_output` 1,
  `level1_structured_error_message` 1, `protect_cli` 1,
  `provider_error_finalize` 1.
  Allow-listed spawn sites: 24 + 22 + 2 + 2 + 5 + 3 + 1 + 2 + 1 + 1 + 1 + 1 +
  1 + 1 = **66**.
- **Purpose** — prove context rendering, skill/resource linking, the errors and
  protect commands, structured error messages, and the handle route's deadline
  and blocking-output behavior.
- **Assertion quality** — good; `context_no_markdown_parsing_artifacts` and the
  skills-linking cases distinguish real rendering and linking defects.
- **Shared helpers** — `common/mod.rs`, `common/completion.rs`,
  `common/wrap.rs`; `context_command`, `errors_command` and
  `diagnostic_discovery` declare **no** `mod common` at all and build
  everything themselves.
- **Effects / dependencies** — spawns `claudine` raw with an inherited
  environment; `skills_integration` links real resource trees;
  `handle_deadline` and `compose_ttff_perf` hold a **live `std::process::Child`**
  and cannot use `assert_cmd`.
- **Timing** — summed 29.07 s across 83 executed identities (350 ms mean), max
  2.84 s. `context_command` alone is 25.08 s over 27 cases — the most expensive
  L1 integration binary in the area after `error_guards`.
- **Overrides** — CI `claudine-cli-ci-l1` group;
  `context_command::context_reports_preserve_all_columns_at_minimum_supported_width`
  is a separate family below.
- **Resource ownership** — child processes, two of them live and polled.
- **Route** — L1, no features, all platforms. **Five identities are
  `#[ignore]`d** (`completion_perf` ×4, `compose_ttff_perf` ×1) and run in no
  canonical recipe.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — **remediation in this fix** (Phase 5C). `handle_deadline`
  and `compose_ttff_perf` route through Phase 4's raw-command path rather than
  `assert_cmd`; the rest migrate to the fixture.

#### `cli-l1-pty` — 11 identities

- **Members** — `sequence_overlay_pty` 7,
  `level1_compose_autocomplete_failure_pty` 2,
  `level1_inline_compose_mismatch_pty` 2.
- **Purpose** — prove interactive prompts and step overlays over a real PTY at
  L1, without a terminal multiplexer.
- **Assertion quality** — good; each asserts the rendered PTY frame.
- **Shared helpers** — `common/pty.rs` (`expectrl::session::OsSession`),
  `common/mod.rs`.
- **Effects / dependencies** — allocates a PTY (`/dev/ptmx`); spawns `claudine`
  raw. **Nine `sleep(20 ms)` readiness sites** — five in `common/pty.rs`, two in
  each `level1_*_pty` binary. Seven `#[serial]` sites in
  `sequence_overlay_pty`.
- **Timing** — **summed 19.19 s across 11 identities — 1.74 s mean, the most
  expensive L1 family per test in the area.** Max 3.19 s.
- **Overrides** — CI `claudine-cli-ci-l1` group.
- **Resource ownership** — a PTY and a child process per case.
- **Route** — L1 (not L2 — no multiplexer, no `require_level!` gate),
  `#[cfg(unix)]` on `mod pty;`, so the family does not exist on Windows.
  `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — **remediation in this fix** (Phases 5D and 7).
  `common/pty.rs`'s session construction routes through Phase 4's raw-command
  path so the three binaries inherit the environment policy; the nine readiness
  sleeps become bounded observation with a deadline.

#### `cli-l1-pty-interactive` — 19 identities

Recorded here as `cli-l2-pty` until review-1 finding 1. The tier claim was
wrong: none of these binaries creates a terminal-emulator session, so the
"PTY plus a terminal session" resource below was never owned and the L2 route
was not earned. They are Level 1 and now carry `level1_` names.

- **Members** — `level1_schema_prompt_pty` 11,
  `level1_provided_partial_file_pty` 4, `level1_dry_run_pty` 2,
  `level1_pty_wrapper_summary` 2.
- **Purpose** — prove interactive prompt collection over a PTY, including the
  dry-run approval prompt's parity with normal mode, and — for
  `level1_pty_wrapper_summary` — that the wrapper's pre-delegation summary
  reaches an interactive terminal, as text, before the wrapped child's output.
- **Assertion quality** — strong; parity assertions distinguish a divergence
  between the two prompt paths. The claims are textual: `expectrl` matches
  substrings in a byte stream, so no assertion in this family sees glyph width,
  SGR styling, or layout. `level1_pty_wrapper_summary` was named and described
  as proving the badge row was "visible as rendered terminal UI"; that claim is
  withdrawn (review-1 finding 2), and no L2 capture replaces it — see the row's
  disposition.
- **Shared helpers** — `common/mod.rs` (`CliProcessFixture`), `common/pty.rs`,
  `common/wrap.rs`.
- **Effects / dependencies** — a PTY (`/dev/ptmx`) and a `claudine` child built
  by the fixture builder. No terminal emulator, no multiplexer.
- **Timing** — summed 54.61 s across 19 identities (2.87 s mean), max 4.71 s
  under the L2 route's parallel self-spawn mode. On the L1 route after the
  migration the same 19 finish in 4.72 s wall (`just test-cli`, 2026-09-09).
- **Overrides** — none. The `package(claudine-cli) & test(/level2_/)` blanket no
  longer selects them; they inherit the default 5 s × 6 slow-timeout and the CI
  `claudine-cli-ci-l1` group like every other CLI L1 identity.
- **Resource ownership** — a PTY and a child process per test.
- **Route** — L1, unix, no feature gate; `just test` / `just test-cli`. Was L2
  behind `terminal-tests` and `just test-l2`.
- **Cost provenance** — `baseline/local-gates/just-test-l2.log` for the summed
  figure above (taken while the family was routed L2).
- **Disposition** — **remediation in this fix** (review-1 finding 1: reclassified L2 -> L1). `common/pty.rs`'s
  sleeps are counted against `cli-l1-pty`, which owns that helper's row.
  Review-1 closure criterion 2 (wrapper-summary rendering claim) is closed by
  narrowing, not by new coverage. Every `level2_*` and `level3_*` binary in
  `claudine/cli/tests`, `claudine/lib/tests` and `claudine/gen/tests` was read:
  none asserts the header row that `output::log_wrapper_header` emits. The two
  that come closest still do not — `level2_perf_capture` runs
  `compose --goose --perf --dry-run --yolo` under `FORCE_COLOR=1`, so the row
  is printed, but its own comment records that the headline scrolls out of the
  viewport and every assertion is on the perf tree; `level2_stalled_generation_capture`
  is the only L2 test on the bare wrap path (`claudine opencode '<prompt>'`)
  and asserts only the `Agent Error` block. `level2_dry_run_metadata_capture`
  asserts a red `YOLO` cell in the `--dry-run` *metadata table*, a different
  surface, and both `level2_wrap_ctrl_c_*` binaries run `compose --opencode`
  under `NO_COLOR=1` for a shell sentinel. The badge constants in
  `claudine/lib/src/badges.rs` are `Prose` output, and that renderer's SGR
  emission is proven in a real emulator by
  `biscuit-terminal-cli::level2_prose_styling`
  (`level2_prose_rich_styling_emits_sgr_in_wezterm` / `…_in_kitty` decode bold,
  italic and fg/bg RGB from the terminal's own capture) — component-level
  evidence for the primitive, not for this row's composition, spacing, glyph
  width, or truncation at pane width. A dedicated L2 capture of the header was
  not added: the review permits narrowing, and ~2 s of L2 cost for an unproven
  regression risk is against Rule 2 in a test-performance fix. Recorded as a
  gap, not as coverage.

#### `cli-l1-live-child` — 1 identity, 0 executed

- **Members** — `wrap_sigint` 1 (`slow_compose_sigint_during_prep_exits_130_with_notice`),
  plus `wrap_ctrl_c_windows` and `sequence_ctrl_c_windows`, which list **no**
  identity on this host because both files are `cfg(windows)` in their
  entirety.
- **Purpose** — prove signal delivery to a wrapped child: SIGINT during prep
  exits 130 with a notice on Unix; `GenerateConsoleCtrlEvent` terminates the
  wrapper subtree on Windows.
- **Assertion quality** — strong in subject. The Windows pair additionally
  carries an environment hazard the predecessor recorded but did not fix: an
  inherited `CLAUDINE_TIMEOUT` can produce a **silent false pass**.
- **Shared helpers** — `common/mod.rs`, `common/wrap.rs`.
- **Effects / dependencies** — hold a live `std::process::Child`, need
  `CREATE_NEW_PROCESS_GROUP` on Windows, poll `try_wait`, and sleep (1 site in
  `wrap_sigint`, 3 in each Windows file, including a 60 s and a 3 s wait).
  `assert_cmd::Command` has neither `spawn` nor a way back to the inner
  `std::process::Command`, which is the whole `NEEDS_LIVE_CHILD` exemption.
- **Timing** — not observed. Nothing in this family ran in any Phase 1 gate.
- **Overrides** — none.
- **Resource ownership** — a live child and, on Windows, a process group.
- **Route** — **this is a reachability finding.**
  `slow_compose_sigint_during_prep_exits_130_with_notice` is named `slow_*`, so
  the L1 tier filter excludes it, and it is included only when
  `BISCUIT_L1_INCLUDE_SLOW=1`. That flag comes from
  `[package.metadata.ci.tests] l1-include-slow`, which **only darkmatter's four
  packages declare** — `claudine-cli` does not. It is the only `slow_` test in
  the eight packages, and it therefore **runs in no canonical recipe and no CI
  leg**. The Windows pair runs only on `windows-latest`.
- **Cost provenance** — none.
- **Disposition** — **remediation in this fix** (Phases 5D and 6). Phase 5D
  routes all three through the raw-command path; Phase 6 must decide whether
  `slow_compose_sigint_…` earns `l1-include-slow = true` on `claudine-cli` or a
  rename out of the `slow_` tier — migrating a test that never runs would
  otherwise be work with no gate behind it.

### `claudine-cli` — L2, L3 and real tiers

#### `cli-l2-lifecycle` — 114 identities

- **Members** — `level2_lifecycle_control` 96, `level2_lifecycle_dispatch` 11,
  `level2_lifecycle_loop` 4, `level2_lifecycle_action_forms` 3.
- **Purpose** — drive the full lifecycle (initialize → dispatch → loop →
  finalize) through a real claudine run against a fake provider inside tmux,
  and assert the emitted event order and rendered frames.
- **Assertion quality** — strong; these are the only tests that see the
  lifecycle as a user does.
- **Shared helpers** — `common/wrap.rs`, `common/mod.rs`,
  `biscuit-test-harness` (`SharedHarness`, tmux backend).
- **Effects / dependencies** — a tmux session per test (self-spawn mode), a
  login shell, a real claudine run, a fake provider stub on `PATH`. 96
  `#[serial]` sites and 23 sleep sites in `level2_lifecycle_control` alone.
- **Timing** — **summed 200.35 s across 114 identities — 1.76 s mean**, max
  6.82 s. The single most expensive family in the area.
- **Overrides** — `[[profile.default.overrides]] package(claudine-cli) & test(/level2_/)`
  → 30 s × 3 = 90 s ceiling, because the binaries own an internal ~40 s
  wait-for-marker deadline that the default 30 s ceiling sits below. CI adds
  `test(/level2_/) retries = 0`.
- **Resource ownership** — one tmux session and one claudine process tree per
  test, Drop-cleaned.
- **Route** — L2, `terminal-tests` feature, unix + windows where tmux exists;
  `just test-l2`. Skips cleanly when the backend is absent.
- **Cost provenance** — `baseline/local-gates/just-test-l2.log`.
- **Disposition** — **remediation in this fix** (Phase 7). The 23 sleep sites
  are readiness waits inside a family that already owns a deadline; each needs
  bounded observation of the final required condition. The `#[serial]`
  annotations are a no-op across nextest processes and every test self-isolates,
  so they are removal candidates — the resource is per-test, not shared.

#### `cli-l2-render-capture` — 74 identities

- **Members** — `level2_typed_error_render_capture` 12,
  `level2_sequence_task_stream_capture` 10, `level2_context_capture` 19,
  `level2_file_resolution_capture` 7, `level2_dry_run_metadata_capture` 6,
  `level2_prompt_reporting_capture` 4, `level2_invalid_file_reference_capture` 4,
  `level2_perf_capture` 2, `level2_inline_compose_mismatch_capture` 2,
  `level2_removed_validation_key_capture` 2,
  `level2_dry_run_approval_capture` 1, `level2_incomplete_subagents_capture` 1,
  `level2_interrupt_feedback_capture` 1, `level2_malformed_frontmatter_capture` 1,
  `level2_schema_parse_capture` 1, `level2_stalled_generation_capture` 1.
- **Purpose** — capture a real terminal frame (tmux or WezTerm) and assert the
  rendered styling, wrapping, hyperlinks and diagnostic highlighting.
- **Assertion quality** — strong; these are the only assertions that see real
  SGR and OSC 8 output.
- **Shared helpers** — `common/mod.rs`, `common/wrap.rs`, `biscuit-test-harness`.
- **Effects / dependencies** — a tmux or WezTerm session per test; several need
  a specific terminal width; `level2_prompt_reporting_capture` and
  `level2_dry_run_metadata_capture` need WezTerm specifically for OSC 8.
- **Timing** — **summed 188.12 s across 74 identities — 2.54 s mean**, max
  15.33 s (`level2_sequence_task_stream_capture::level2_prompt_idle_flush_keeps_the_task_bar_in_tmux`,
  which waits on an idle-flush clock).
- **Overrides** — the `package(claudine-cli) & test(/level2_/)` blanket; CI
  `retries = 0`.
- **Resource ownership** — one terminal session per test, Drop-cleaned.
- **Route** — L2, `terminal-tests`; `just test-l2`.
- **Cost provenance** — `baseline/local-gates/just-test-l2.log`.
- **Disposition** — **remediation in this fix** (Phase 7). Twenty sleep sites
  across the family; the 15.33 s idle-flush case is the clearest bounded-
  observation candidate in L2.

#### `cli-l2-autocomplete` — 27 identities

- **Members** — `level2_auto_complete_operation_file` 19,
  `level2_auto_complete_chooser` 6, `level2_explicit_operation_file_miss` 2.
- **Purpose** — prove the interactive chooser's layout and selection behavior
  in a real terminal at several heights.
- **Assertion quality** — strong; asserts the captured frame including the
  detail pane's placement.
- **Shared helpers** — `common/mod.rs`, `biscuit-test-harness`.
- **Effects / dependencies** — tmux session per test; keyboard input is
  injected through the harness, not the OS.
- **Timing** — summed 44.68 s across 27 identities (1.65 s mean), max 3.59 s.
- **Overrides** — the L2 blanket; CI `retries = 0`.
- **Resource ownership** — one tmux session per test.
- **Route** — L2, `terminal-tests`; `just test-l2`.
- **Cost provenance** — `baseline/local-gates/just-test-l2.log`.
- **Disposition** — satisfactory.

#### `cli-l2-ctrl-c` — 3 identities

- **Members** — `level2_wrap_ctrl_c_tmux` 2,
  `level2_wrap_ctrl_c_loop_wedge_tmux` 1.
- **Purpose** — prove Ctrl+C delivered through a multiplexer terminates the
  wrapped child, and that a double Ctrl+C force-exits a loop wedged between
  iterations.
- **Assertion quality** — strong; the wedge case is a named regression.
- **Shared helpers** — `common/wrap.rs`, `biscuit-test-harness`.
- **Effects / dependencies** — tmux session, real signal delivery, a claudine
  process tree.
- **Timing** — summed 6.90 s across 3 identities, max 2.73 s.
- **Overrides** — the L2 blanket; CI `retries = 0`.
- **Resource ownership** — a tmux session and a process tree per test.
- **Route** — L2, `terminal-tests`; `just test-l2`.
- **Cost provenance** — `baseline/local-gates/just-test-l2.log`.
- **Disposition** — satisfactory.

#### `cli-l3-keyboard` — 4 identities, 0 executed

- **Members** — `level3_wrap_ctrl_c` 2, `level3_auto_complete_chooser` 1,
  `level3_sequence_ctrl_c` 1, plus `level3_linux_sequence_ctrl_c` and
  `level3_windows_sequence_ctrl_c`, which list no identity on this host.
- **Purpose** — prove genuine OS keystroke delivery (cliclick on macOS,
  xdotool on Linux/X11, PowerShell SendKeys on Windows) into a real WezTerm
  window.
- **Assertion quality** — unreviewed against a live run.
- **Shared helpers** — `common/wrap.rs`, `write_executable`,
  `biscuit-test-harness` (WezTerm backend + per-OS injector).
- **Effects / dependencies** — **a real WezTerm window that takes focus**, an
  OS keystroke injector, and an attended host. `RUN_LEVEL3=1` plus a
  `require_level!(Level::L3, …)` gate; each skips cleanly when its backend is
  absent.
- **Timing** — not observed.
- **Overrides** — none.
- **Resource ownership** — a WezTerm window and the host's input focus.
- **Route** — L3, `terminal-tests`; `just test-l3`, `-j 1`.
- **Cost provenance** — none; L3 was not run in this phase.
- **Disposition** — **follow-up**: L3 needs an operator-attended host and L3
  must never steal focus from an unattended session. Recorded as pending, not
  as passing (AC6). Phase 7's "no terminal or browser gains focus" check
  applies to the tiers this fix actually touches; L3's focus is its contract.

#### `cli-real-provider` — 1 identity, 0 executed

- **Member** — `real_opencode_yolo_subagent` 1.
- **Purpose** — regression-test the OpenCode YOLO subagent's
  external-directory behavior against the real provider.
- **Assertion quality** — unreviewed against a live run.
- **Shared helpers** — `common/mod.rs`, `init_git_repo`.
- **Effects / dependencies** — `CLAUDINE_CONTRACT_REAL=1`, the `opencode`
  binary on `PATH`, credentials, network.
- **Timing** — not observed.
- **Overrides** — none.
- **Resource ownership** — a real provider session and a git repository.
- **Route** — real tier, `real-tests`; `just test-real`, which runs it through
  `cargo test`, not nextest.
- **Cost provenance** — none.
- **Disposition** — **follow-up**: real-tier evidence is pending (AC6).

### `claudine-cli` — runner-override targets

Each of these is a family of its own because AC5 requires a per-override
justification, and a justification needs a row to sit in.

#### `cli-override-loop-rate-limit` — 1 identity

- **Member** — `loop_cli::compose_loop_rate_limit_pause_waits_then_continues`.
- **Purpose** — prove the rate-limit pause policy waits and then continues.
- **Assertion quality** — strong; the wait *is* the subject.
- **Shared helpers** — `common/mod.rs`; spawns raw (allow-listed file).
- **Effects / dependencies** — spawns `claudine`; waits on a real clock.
- **Timing** — 4.04 s observed. **This is a genuine timeout contract**: the
  test asserts a real pause, so the sleep is the assertion, not a readiness
  wait.
- **Overrides** — both profiles: `slow-timeout = { period = "30s", terminate-after = 3 }`.
- **Resource ownership** — a child process.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — satisfactory, and its override is **justified**: the floor
  expresses a real rate-limit pause contract, and 4.04 s of deliberate waiting
  against a 30 s slow mark is a correct ratio.

#### `cli-override-empty-state-routing` — 1 identity

- **Member** — `command_routing::agents_and_commands_route_to_empty_state_messages`.
- **Purpose** — prove the agents and commands routes emit empty-state messages.
- **Assertion quality** — good.
- **Shared helpers** — `common/mod.rs`; **fixture-migrated**.
- **Effects / dependencies** — shells out to the CLI several times.
- **Timing** — **0.04 s observed.** The override's justification says it "is
  below the default timeout isolated, but can exceed 30 s under full-suite
  contention" — three orders of magnitude above the measurement.
- **Overrides** — both profiles: 30 s × 3.
- **Resource ownership** — child processes.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — **remediation in this fix** (Phase 6). The stated cost
  predates the fixture migration. AC5 requires re-justification against a
  current contract or removal with the cost it hid; CI evidence decides, not
  this host.

#### `cli-override-context-width` — 1 identity

- **Member** — `context_command::context_reports_preserve_all_columns_at_minimum_supported_width`.
- **Purpose** — prove every context report column survives at the minimum
  supported terminal width.
- **Assertion quality** — strong; it is the width-regression guard.
- **Shared helpers** — none; `context_command` declares no `mod common` and
  spawns raw.
- **Effects / dependencies** — spawns `claudine` repeatedly with an inherited
  environment.
- **Timing** — **5.21 s observed — the most expensive single L1 identity in
  the area.** It is 21 % of `context_command`'s 25.08 s.
- **Overrides** — default profile only: 30 s × 3. Named explicitly by the spec
  as the known example.
- **Resource ownership** — child processes.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — **remediation in this fix** (Phases 5C and 6). The spec's
  instruction is explicit: this override is removed **together with the cost it
  was hiding**, not on its own. Phase 5C migrates the binary; Phase 6 removes
  the entry once the cost is addressed.

#### `cli-override-perf-parity` — 2 identities

- **Members** — `wrap_perf::compose_perf_stdout_matches_non_perf`,
  `wrap_perf::inline_compose_perf_stdout_matches_non_perf`.
- **Purpose** — prove `--perf` does not change stdout.
- **Assertion quality** — strong; a byte-for-byte parity assertion.
- **Shared helpers** — `common/mod.rs`; fixture-migrated.
- **Effects / dependencies** — two `claudine` runs per case, compared.
- **Timing** — summed 0.82 s, max 0.45 s.
- **Overrides** — CI profile only: 30 s × 3 each.
- **Resource ownership** — child processes.
- **Route** — L1, no features, all platforms; `just test-cli`.
- **Cost provenance** — `baseline/local-gates/just-test-cli.log`.
- **Disposition** — **remediation in this fix** (Phase 6). Two complete runs is
  the comparison, so the cost is intrinsic — but 0.45 s against a 30 s slow
  mark is not a floor that needs expressing. Re-justify against CI numbers or
  remove.

### `claudine-gen`

#### `gen-unit` — 93 identities

- **Members** — the `claudine-gen` lib suite.
- **Purpose** — prove catalog generation, schema compatibility, signal
  detection compilation and report rendering.
- **Assertion quality** — good.
- **Shared helpers** — `tempfile`, `strum` variant reflection.
- **Effects / dependencies** — temporary directories.
- **Timing** — summed 1.24 s across 93 identities, max 0.04 s.
- **Overrides** — none.
- **Resource ownership** — temporary directories.
- **Route** — L1; `terminal-tests` adds nothing to this suite. All platforms;
  `just test` / `just test-gen`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory.

#### `gen-artifact-pipeline` — 52 identities, 51 executed

- **Members** — `pipeline` 18, `agent_errors_check` 10, `signals_validation` 9,
  `drift` 6, `vocabulary` 4, `registry_coverage` 3, `fixtures_provenance` 1,
  `signals_sidecar_mirror` 1.
- **Purpose** — prove the committed catalog matches what regeneration produces,
  that the signal corpus validates, and that generated tables do not drift from
  their sources.
- **Assertion quality** — strong; `committed_catalog_matches_regenerated_inputs`
  is the drift gate.
- **Shared helpers** — `gen/tests/fixtures/`.
- **Effects / dependencies** — read the committed catalog and corpus; write
  regenerated output to temporary directories.
- **Timing** — summed 5.42 s across 51 executed identities, max 0.59 s.
- **Overrides** — none.
- **Resource ownership** — temporary directories.
- **Route** — L1, no features, all platforms; `just test-gen`, which also runs
  `just signals-check` (`cargo run -p claudine-cli -- signals check`).
  **`signals_validation::real_corpus_builds_deterministically` is named
  `real_*`**, so the L1 tier filter excludes it — and this is a reachability
  finding, below.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — **remediation in this fix** (Phase 6). The `real_*` name
  costs the family a test in every environment.

#### `gen-generate-ux` — 10 identities

- **Members** — `generate_ux` 10.
- **Purpose** — prove the generator's interactive UX: drift reporting,
  `--yes` behavior, reconvergence after a write.
- **Assertion quality** — strong; `yes_writes_all_drift_and_check_reconverges`
  asserts both the write and the subsequent clean check.
- **Shared helpers** — none.
- **Effects / dependencies** — copies the catalog into a temporary tree and
  regenerates it there; **never writes the real checkout**.
- **Timing** — summed 8.34 s across 10 identities (834 ms mean), max 1.47 s.
  The most expensive `claudine-gen` family.
- **Overrides** — none.
- **Resource ownership** — temporary directories.
- **Route** — L1, no features, all platforms; `just test-gen`.
- **Cost provenance** — `baseline/local-gates/just-test.log`.
- **Disposition** — satisfactory. The cost is full regeneration, which is the
  assertion.

#### `gen-l2-report` — 3 identities

- **Members** — `level2_report_terminal` 3, gated
  `required-features = ["terminal-tests"]`.
- **Purpose** — prove the generator's report renders correctly in a real
  terminal, including wrapping and placeholder retention.
- **Assertion quality** — strong; captured-frame assertions.
- **Shared helpers** — `biscuit-test-harness`, `serial_test`.
- **Effects / dependencies** — a tmux session per test; `#[serial]`-chained, so
  the plain serial path rather than self-spawn.
- **Timing** — summed 2.75 s, max 1.55 s.
- **Overrides** — none. Notably the `package(claudine-cli) & test(/level2_/)`
  blanket does **not** cover this package, so these three run under the default
  5 s × 6 ceiling.
- **Resource ownership** — one tmux session per test.
- **Route** — L2, `terminal-tests`; `just test-l2` (second invocation).
- **Cost provenance** — `baseline/local-gates/just-test-l2.log`.
- **Disposition** — satisfactory.

### Rendezvous

The parent area's `just test` covers only the five claudine crates. These three
crates are reached solely by `just test-rendezvous` from the parent area, or
`just test` inside `claudine/rendezvous`. Costs below come from a Phase 2 run
of `just test-rendezvous`, logged at
[`enumeration/recipes/just-test-rendezvous.log`](enumeration/recipes/just-test-rendezvous.log).

#### `rz-core-unit` — 82 identities

- **Members** — the `rendezvous-core` lib suite.
- **Purpose** — prove endpoint derivation, envelope encoding, and identity
  handling.
- **Assertion quality** — strong, and deliberately independent:
  `cfg(unix)` endpoint tests assert against `libc`'s effective UID rather than
  against `sniff`, so the code cannot agree with itself.
- **Shared helpers** — `tempfile`, `libc` (unix dev-dependency).
- **Effects / dependencies** — temporary runtime directories.
- **Timing** — summed 0.91 s across 82 identities, max 0.02 s.
- **Overrides** — none.
- **Resource ownership** — temporary directories.
- **Route** — L1, no features (the `test-support` feature adds no identity to
  this crate). Six identities are `cfg(windows)`. `just test-rendezvous`.
- **Cost provenance** — `enumeration/recipes/just-test-rendezvous.log`.
- **Disposition** — satisfactory.

#### `rz-daemon-unit` — 153 identities

- **Members** — the `rendezvous-daemon` lib suite (`service`, `session_log`,
  `private_dir`, `local_transport`, `server`).
- **Purpose** — prove session registration under concurrency, session-log
  replay and rehydration, private-directory ownership, and local transport
  binding.
- **Assertion quality** — strong;
  `service::tests::session_register::concurrent_end_and_start_new_session_no_lost_update`
  distinguishes exactly the lost-update it names.
- **Shared helpers** — `tokio` (79 `#[tokio::test]` sites across the three
  rendezvous crates), `tempfile`.
- **Effects / dependencies** — real redb and DuckDB storage in temporary data
  roots; real local endpoints (Unix domain sockets, Windows named pipes).
- **Timing** — **summed 107.75 s across 153 identities — 704 ms mean, the
  highest of any unit-test family in the eight packages**, max 6.80 s. The
  crate's `just test` elapsed is 9.2 s, so the cost is hidden by parallelism
  rather than absent.
- **Overrides** — none. No `[package.metadata.ci.tests]`, so CI runs it in the
  defaulted L1 tier with no features.
- **Resource ownership** — data directories, embedded databases, and bound
  endpoints, all per-test.
- **Route** — L1, no features. Thirteen identities are `cfg(windows)`.
  `just test-rendezvous`.
- **Cost provenance** — `enumeration/recipes/just-test-rendezvous.log`.
- **Disposition** — **remediation in this fix** (Phases 3, 6 and 7). This is
  the largest non-spawn cost the plan's attribution question ("which non-spawn
  families account for the remaining execution cost") has surfaced so far, and
  it sits in a crate the parent area's ordinary `just test` never runs. Phase 3
  attributes it; Phase 7 owns the isolated-endpoint and cleanup requirements
  the spec names for daemon/session/IPC tests.

#### `rz-daemon-integration` — 19 identities, 16 executed

- **Members** — `phase6_integration` 12, `pairing_and_sync` 4,
  `peer_discovery` 3.
- **Purpose** — prove multi-node pairing, mesh convergence, and peer discovery
  across real daemons.
- **Assertion quality** — strong; `repos_register_converges_across_mesh`
  asserts convergence, not merely that a message was sent.
- **Shared helpers** — `rendezvous-core` with `test-support` (the endpoint
  constructor for daemon-spawning fixtures).
- **Effects / dependencies** — spawns real daemons, binds real endpoints, and
  for the two mDNS cases uses **real multicast on the host network**.
- **Timing** — summed 25.46 s across 16 executed identities (1.59 s mean), max
  2.31 s.
- **Overrides** — none.
- **Resource ownership** — daemon processes, data roots and bound endpoints.
- **Route** — L1 for 16 identities. **Two are named `real_*`**
  (`peer_discovery::real_two_daemons_discover_each_other_via_mdns`,
  `peer_discovery::real_mdns_discovered_peer_cannot_sync_before_approval`), so
  the L1 filter excludes them — and the rendezvous justfile defines **no**
  `test-real` recipe, while the crate declares no `[package.metadata.ci.tests]`
  and therefore defaults to the L1 tier in CI. Both run **nowhere**.
- **Cost provenance** — `enumeration/recipes/just-test-rendezvous.log`.
- **Disposition** — **remediation in this fix** (Phase 6/7). Two unreachable
  mDNS tests plus the isolated-endpoint and cleanup obligations the spec places
  on this cohort.

#### `rz-client-unit` — 14 identities

- **Members** — the `rendezvous-client` lib suite (`connector`).
- **Purpose** — prove connection classification, retry budgets and deadline
  behavior.
- **Assertion quality** — strong; `every_classified_error_names_the_endpoint`
  and `a_zero_budget_still_tries_once_and_never_sleeps` distinguish real
  defects rather than restating the signature.
- **Shared helpers** — `tokio`.
- **Effects / dependencies** — none beyond a refused local connection.
- **Timing** — summed 0.18 s across 14 identities, max 0.01 s.
- **Overrides** — none.
- **Resource ownership** — none.
- **Route** — L1, no features. Three identities are `cfg(windows)` real-pipe
  tests. `just test-rendezvous`.
- **Cost provenance** — `enumeration/recipes/just-test-rendezvous.log`.
- **Disposition** — satisfactory.

#### `rz-client-integration` — 7 identities

- **Members** — `local_round_trip` 5, `session_log_round_trip` 2.
- **Purpose** — prove ping/status round-trip over a real local endpoint, that
  dropping the handle removes the socket file, that two clients are served
  concurrently, and that appends persist to redb and eventually to DuckDB.
- **Assertion quality** — strong, and `append_persists_to_redb_and_eventually_to_duckdb`
  is a genuine repeated write/read round trip through persistence.
- **Shared helpers** — `rendezvous-core` with `test-support`.
- **Effects / dependencies** — spawns a daemon, binds a real endpoint, writes
  real databases.
- **Timing** — summed 4.21 s across 7 identities (601 ms mean), max 0.73 s.
- **Overrides** — none.
- **Resource ownership** — a daemon process, an endpoint and a data root per
  test.
- **Route** — L1, no features, all platforms; `just test-rendezvous`.
- **Cost provenance** — `enumeration/recipes/just-test-rendezvous.log`.
- **Disposition** — satisfactory.

## cfg/feature exclusions

Fifty-five source-defined tests that the runner never lists on this host,
listed **separately from executed tests** as RB1 requires, each with its gate
and its actual execution route. The reconciler derives this set mechanically
(source attribute scan minus runner universe, per package and leaf name) and
fails on an undeclared entry or a stale one, so the table cannot rot in either
direction.

Every one is a platform gate. **No test in the eight packages is excluded by a
feature it never gets.**

| Package | Test | Gate | Executes on |
|---|---|---|---|
| `claudine` | `unix_absolute_sensitive_paths_are_detected` | `cfg(target_os = "linux")` | ubuntu-latest and wsl2-ubuntu legs |
| `claudine` | `a_successful_command_reaps_its_backgrounded_descendant` | `cfg(unix) / cfg(windows) pair` | both arms exist; the unix arm runs here, the windows twin on windows-latest |
| `claudine` | `the_system_shell_captures_a_pipeline` | `cfg(unix) / cfg(windows) pair` | both arms exist; the unix arm runs here, the windows twin on windows-latest |
| `claudine` | `the_system_shell_kills_a_backgrounded_descendant_holding_stdout` | `cfg(unix) / cfg(windows) pair` | both arms exist; the unix arm runs here, the windows twin on windows-latest |
| `claudine` | `the_system_shell_times_out_a_nested_tree` | `cfg(unix) / cfg(windows) pair` | both arms exist; the unix arm runs here, the windows twin on windows-latest |
| `claudine` | `declined_device_namespace_falls_back_to_plain_text` | `cfg(windows)` | windows-latest leg |
| `claudine` | `display_label_declined_portable_spelling_keeps_native_text` | `cfg(windows)` | windows-latest leg |
| `claudine` | `handle_command_quotes_native_windows_executable_path` | `cfg(windows)` | windows-latest leg |
| `claudine` | `non_transient_windows_persist_error_is_not_retried` | `cfg(windows)` | windows-latest leg |
| `claudine` | `system_shell_runner_preserves_nested_quotes` | `cfg(windows)` | windows-latest leg |
| `claudine` | `the_system_shell_preserves_nested_quotes` | `cfg(windows)` | windows-latest leg |
| `claudine` | `transient_windows_persist_errors_are_narrowly_classified` | `cfg(windows)` | windows-latest leg |
| `claudine` | `transient_windows_persist_retries_are_bounded` | `cfg(windows)` | windows-latest leg |
| `claudine` | `transient_windows_retries_reuse_the_written_temp_file` | `cfg(windows)` | windows-latest leg |
| `claudine` | `windows_drive_and_unc_targets_use_valid_file_uris` | `cfg(windows)` | windows-latest leg |
| `claudine` | `windows_resource_link_creates_file_symlink` | `cfg(windows)` | windows-latest leg |
| `claudine` | `windows_resource_link_reports_already_linked_file` | `cfg(windows)` | windows-latest leg |
| `claudine` | `windows_resource_link_skips_real_file_collision` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `job_close_terminates_a_descendant_when_the_wait_scope_ends` | `cfg(all(test, windows))` | windows-latest leg |
| `claudine-cli` | `repeated_wait_scopes_do_not_grow_the_process_handle_count` | `cfg(all(test, windows))` | windows-latest leg |
| `claudine-cli` | `level3_linux_sequence_ctrl_c_fans_out_to_parallel_children` | `cfg(target_os = "linux") on the test module` | ubuntu-latest and wsl2-ubuntu `just test-l3` |
| `claudine-cli` | `perf_enter_compose_partial_meets_target` | `cfg(unix) / cfg(not(unix)) pair` | both arms exist; the unix arm runs here, the non-unix twin on windows-latest |
| `claudine-cli` | `repository_suggestions_do_not_follow_directory_symlinks` | `cfg(unix) / cfg(windows) pair` | both arms exist; the unix arm runs here, the windows twin on windows-latest |
| `claudine-cli` | `child_env_invariant_accepts_windows_key_spelling` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `ctrl_c_terminates_wrapped_child_on_windows` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `file_url_uses_windows_drive_uri_shape` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `non_unix_wait_loop_returns_on_child_exit` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `portable_text_declines_unc_namespace_rewriting` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `windows_completion_termination_uses_job_object_path` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `windows_drive_hyperlink_encodes_reserved_characters` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `windows_unc_hyperlink_uses_authority_and_encodes_path` | `cfg(windows)` | windows-latest leg |
| `claudine-cli` | `level3_windows_sequence_ctrl_c_fans_out_to_parallel_children` | `cfg(windows) on the test module` | windows-latest `just test-l3` |
| `claudine-cli` | `sequence_ctrl_c_fans_out_to_parallel_children_on_windows` | `cfg(windows) on the test module` | windows-latest `just test-cli` |
| `rendezvous-client` | `a_real_busy_pipe_is_connected_once_an_instance_frees_up` | `cfg(windows)` | windows-latest leg |
| `rendezvous-client` | `a_real_pipe_that_stays_busy_gives_up_at_the_deadline` | `cfg(windows)` | windows-latest leg |
| `rendezvous-client` | `a_real_saturated_pipe_reports_the_code_the_retry_loop_recognizes` | `cfg(windows)` | windows-latest leg |
| `rendezvous-core` | `windows_accepts_pipes_and_rejects_sockets` | `cfg(windows)` | windows-latest leg |
| `rendezvous-core` | `windows_default_is_a_sid_qualified_pipe` | `cfg(windows)` | windows-latest leg |
| `rendezvous-core` | `windows_derivation_refuses_a_unix_identity` | `cfg(windows)` | windows-latest leg |
| `rendezvous-core` | `windows_override_rejects_a_filesystem_path` | `cfg(windows)` | windows-latest leg |
| `rendezvous-core` | `windows_override_rejects_malformed_pipe_names` | `cfg(windows)` | windows-latest leg |
| `rendezvous-core` | `windows_override_wins_and_stays_a_pipe_name` | `cfg(windows)` | windows-latest leg |
| `rendezvous-daemon` | `a_data_root_owned_by_another_account_is_rejected` | `cfg(windows)` | windows-latest leg |
| `rendezvous-daemon` | `a_directory_owned_by_another_account_is_rejected` | `cfg(windows)` | windows-latest leg |
| `rendezvous-daemon` | `created_directories_are_owned_by_this_account` | `cfg(windows)` | windows-latest leg |
| `rendezvous-daemon` | `the_current_user_descriptor_names_this_account_and_nobody_else` | `cfg(windows)` | windows-latest leg |
| `rendezvous-daemon` | `the_data_root_is_created_owned_by_this_account` | `cfg(windows)` | windows-latest leg |
| `rendezvous-daemon` | `a_remote_form_client_is_refused_while_the_local_one_is_served` | `cfg(windows) on the parent module declaration` | windows-latest leg |
| `rendezvous-daemon` | `a_second_daemon_cannot_take_a_running_daemon_s_name` | `cfg(windows) on the parent module declaration` | windows-latest leg |
| `rendezvous-daemon` | `concurrent_clients_are_each_given_an_instance` | `cfg(windows) on the parent module declaration` | windows-latest leg |
| `rendezvous-daemon` | `dropping_the_handle_releases_the_name` | `cfg(windows) on the parent module declaration` | windows-latest leg |
| `rendezvous-daemon` | `shutdown_releases_the_name` | `cfg(windows) on the parent module declaration` | windows-latest leg |
| `rendezvous-daemon` | `the_acceptor_stays_available_across_connections` | `cfg(windows) on the parent module declaration` | windows-latest leg |
| `rendezvous-daemon` | `the_handle_reports_the_name_it_bound` | `cfg(windows) on the parent module declaration` | windows-latest leg |
| `rendezvous-daemon` | `the_pipe_dacl_names_this_user_and_nobody_else` | `cfg(windows) on the parent module declaration` | windows-latest leg |

**Windows-only coverage is 47 of these 55 identities, and none of it has ever
been *run* anywhere this fix can see.** `just check-windows` (mingw
`x86_64-pc-windows-gnu`, exit 0 in Phase 1) proves they compile; the
`windows-latest` leg is the only authority on whether they pass, and that leg
is pending along with the rest of the CI tranche.

## Doctests, benches, fuzz

### Doctests — 32 identities

`just doctest`, 0.74 s, 0.73 s merged compilation.

| Package | Doctests | Passed | Ignored | Compile-fail |
|---|---:|---:|---:|---:|
| `claudine-catalog-types` | 0 | 0 | 0 | 0 |
| `claudine` | 30 | 20 | 7 | 3 |
| `claudine-contract` | 2 | 2 | 0 | 0 |
| `claudine-cli` | — | — | — | — |
| `claudine-gen` | 0 | 0 | 0 | 0 |

- **Route** — `just doctest` per package; `claudine-cli` is skipped by the
  recipe with the message *"skipping doctests for claudine-cli: no lib target"*,
  which is correct for a binary-only crate.
- **The seven ignored ones** are `no_run`/`ignore` examples in
  `composition::frontmatter_excerpt`, `linking::symlink`, `messaging::send`,
  `permissions::engine`, `render::prompt::formatting`, and
  `render::prompt::truncation` ×2. They are compiled but not executed, so they
  prove syntax and signatures only.
- **The three compile-fail ones** (`composition::coordinator::invocation`
  `InvocationInputs` and `RunLedger`, `composition::coordinator::handoff`
  `ProxyHandoff`) are genuine negative tests: they prove an API misuse does not
  compile. They cost 0.38 s of the 0.74 s total.
- **Disposition** — satisfactory. Two zero-doctest packages
  (`claudine-catalog-types`, `claudine-gen`) are a coverage observation, not a
  performance one, and are out of this fix's scope.

### Benches — 5 entry points, 1 real

`claudine/lib/benches/` holds five files. **Four of them are zero bytes.**

| Entry point | Size | Harness | Benchmarks |
|---|---:|---|---:|
| `runtime_hot_paths.rs` | 6.7 KB | criterion (`harness = false`, declared `[[bench]]`) | 3 groups: `bench_protect_service`, `bench_stream_parser`, `bench_runtime_config_loading` |
| `claude_parse.rs` | 0 B | libtest (auto-discovered) | 0 |
| `opencode_parse.rs` | 0 B | libtest (auto-discovered) | 0 |
| `pre_flight_checks.rs` | 0 B | libtest (auto-discovered) | 0 |
| `prompt_preparation.rs` | 0 B | libtest (auto-discovered) | 0 |

All five are tracked in git (added by `0110a7a76 feat(claudine): complete
2026-05-15-schemas feature`). Only `runtime_hot_paths` is declared as a
`[[bench]]` target; the other four are auto-discovered by cargo, compile as
empty bench targets with the default libtest harness, and contribute nothing.

**This contradicts the plan's grounding fact**, which lists five bench entry
points as if all five existed. One does.

- **Route** — `just bench` → `cargo bench -p claudine`.
- **Disposition** — **remediation in this fix** is *not* claimed here; four
  empty files are a documentation and hygiene finding, not a test-performance
  one, and deleting them is a behavior-free change that belongs with whoever
  owns the benchmark surface. Recorded for Phase 10's residual-findings list.

### Fuzz — none

The area has no `fuzz/` directory and no `cargo-fuzz` targets. `just fuzz`
prints *"fuzz: not applicable for claudine (planned Phase 5)"* — a forward
reference to a plan that is not this fix's. Recorded as a finding, per the
plan's instruction, rather than silently omitted.

## Shared fixture machinery

First-class rows: these are not tests, but every cost and every contamination
risk in the CLI families passes through them.

| Component | Size | Used by | What it owns |
|---|---:|---|---|
| `cli/tests/common/mod.rs` | 783 lines | 84 of 87 L1 binaries, all L2/L3 | `CliProcessFixture`, `ClaudineCommandBuilder`, `scrub_inherited_environment`, `restore_windows_console_variables`, `path_value`, `augmented_path`, the named escapes, and the checkout-containment guard |
| `cli/tests/common/wrap.rs` | 201 lines | 18 binaries | wrap-specific helpers, provider stubs |
| `cli/tests/common/pty.rs` | 288 lines | 5 binaries | `expectrl::session::OsSession` construction, `#[cfg(unix)]` on `mod pty;` |
| `cli/tests/common/completion.rs` | 147 lines | 9 binaries | seeded completion workspaces |
| `cli/tests/common/source_scan.rs` | 144 lines | `error_guards`, and available to the other guards | shared source-walking primitives |
| `cli/tests/common/incomplete_subagents.rs` | 139 lines | 2 binaries | incomplete-subagent fixtures |
| `cli/tests/error_guards/source_scan.rs` | 50 KB | `error_guards` | the `syn`-based diagnostic-contract scanner |
| `cli/tests/error_guards/*.toml` | 21 KB | `error_guards` | the transport and boxed-diagnostic allow-lists |
| `claudine/justfile` `_ensure-md` | — | `cli-l1-md-fixture` | builds darkmatter's `md` into the workspace target dir before `just test` / `just test-cli` |

**`common/mod.rs`'s environment policy is applied inline in
`ClaudineCommandBuilder::build`** — `env_clear` + `restore_windows_console_variables`,
then `scrub_inherited_environment`, then eleven `.env`/`.env_remove` calls and
`path_value()`. It is written against `assert_cmd::Command` receivers and is
not extractable as-is; that is precisely what Phase 4 changes. The ordering is
a contract: the scrub runs *before* the defaults, so
`CLAUDINE_RENDEZVOUS_REPORT` survives its own namespace sweep, and a per-key
`.env` after `build()` still wins.

- **Disposition** — `common/mod.rs`: **remediation in this fix** (Phase 4).
  `common/pty.rs`: **remediation in this fix** (Phases 5D and 7). The rest:
  satisfactory.

## Runner override census

Twenty-four override blocks exist in `.config/nextest.toml` — **10 in the
default profile and 14 in the CI profile**, of which **17 are per-test
`slow-timeout` entries** (9 default + 8 CI), 1 is a package-wide L2 blanket, 3
are `test-group` bindings, 2 are `retries = 0` re-assertions, and 1 is a
`threads-required` pin.

**The plan's grounding fact said 18 per-test overrides (9 + 9). The file has
17.** It also listed `every_catalog_variable_survives_ambient_options` as
Claudine-scoped; that test lives in
`darkmatter/lib/tests/ambient_ctx_capture.rs` and is out of this fix's scope.

`git diff main -- .config/nextest.toml` is **empty** at this revision. No
override, retry, tier change or disabled assertion has been introduced (AC5,
AC7).

> **Phase 6 resolution (2026-09-08).** Every verdict below was acted on. Eight
> blocks were removed and none added; the diff against `main` is now 16
> insertions / 41 deletions, and **every inserted line is a comment**. See
> [§ Phase 6 disposition](#phase-6-disposition) after the tables.

### Claudine-scoped entries

| # | Profile | Filter | Effect | Binds? | Verdict |
|---|---|---|---|---|---|
| 1 | default + ci | `test(=compose_loop_rate_limit_pause_waits_then_continues)` | 30 s × 3 | yes → `claudine-cli::loop_cli` | **justified** — the floor expresses a real rate-limit pause contract; 4.04 s observed against a 30 s slow mark |
| 2 | default + ci | `test(=agents_and_commands_route_to_empty_state_messages)` | 30 s × 3 | yes → `claudine-cli::command_routing` | **remove with the cost it hides** — 0.04 s observed; the stated "can exceed 30 s under contention" predates the fixture migration. Confirm against CI before removing |
| 3 | default + ci | `test(=composition::loop_engine::tests::seeded_loop_repro_runs_to_completion_with_live_derived_variable)` | 30 s × 3 | **NO — dead filter** | **remove.** The module was renamed `composition::loop_engine` → `composition::looping::engine` (with a `seed_state` module inserted). `test(=…)` is exact-match and nextest does not warn on a filter that selects nothing, so neither entry has bound to anything since the rename. The test runs in 0.03 s under the ordinary ceiling |
| 4 | default | `test(=exhausted_remediation_fails_finalize_and_preserves_findings)` | 30 s × 3 | yes → `claudine::agent_errors_fleet` | **re-justify or remove** — 0.13 s observed against a documented "~21 s alone, 83 s under contention". CI decides |
| 5 | default | `test(=context_reports_preserve_all_columns_at_minimum_supported_width)` | 30 s × 3 | yes → `claudine-cli::context_command` | **remove together with the cost it hides** (spec names this one) — 5.21 s observed, the most expensive single L1 identity in the area |
| 6 | ci | `test(=compose_perf_stdout_matches_non_perf)` | 30 s × 3 | yes → `claudine-cli::wrap_perf` | **re-justify or remove** — 0.37 s observed |
| 7 | ci | `test(=inline_compose_perf_stdout_matches_non_perf)` | 30 s × 3 | yes → `claudine-cli::wrap_perf` | **re-justify or remove** — 0.45 s observed |
| 8 | default | `package(claudine-cli) & test(/level2_/)` | 30 s × 3 | yes → 241 identities | **justified** — the L2 binaries own an internal ~40 s wait-for-marker deadline, which the default 30 s termination ceiling sits *below*; without the raise a healthy contended run is killed before its own deadline can settle |
| 9 | ci | `test(/level2_/)` | `retries = 0` | yes | **justified** — it re-asserts the CI profile's own `retries = 0` for a tier where a transient PTY failure must stay visible. Redundant, not wrong |
| 10 | ci | `package(claudine) & !level2_ & !level3_ & !browser_ & !real_ & !slow_` | test-group `claudine-l1` (max-threads 4) | yes → 4,149 identities | **justified** — the lib's L1 tests perform repeated repository discovery; CI bounds their concurrency on contended runners while local runs use the normal scheduler |
| 11 | ci | `package(claudine-cli) & !level2_ & …` | test-group `claudine-cli-ci-l1` (max-threads 1) | yes → ~2,500 identities | **justified today, and the single largest CI cost lever in the area.** One thread means the CI runner's summed duration *is* its elapsed time (Phase 1's real report: 170.5 s elapsed vs 170.4 s summed). Phase 3 must attribute how much of that serialization the fixture migration makes unnecessary — but do **not** relax it in this fix without CI evidence |

### Entries in this file that are not Claudine's

Recorded so a future reader does not chase them, and so AC5's "in the scoped
packages" boundary is explicit.

| Profile | Filter | Owner |
|---|---|---|
| default + ci | `test(=every_catalog_variable_survives_ambient_options)` | `darkmatter` (`lib/tests/ambient_ctx_capture.rs`) |
| default + ci | `test(=markdown::compose::preflight::acceptance_tests::execution_subset_of_approval_across_randomized_conditions)` | `darkmatter` |
| default | `test(=markdown::compose::tests::rendering::slow_compose_cleanup_preserves_quoted_marker_looking_indented_code)` | `darkmatter` |
| default + ci | `test(=level2_render_tree_style_in_wezterm)` | `biscuit-terminal` |
| ci | `test(=test_detect_completes_in_reasonable_time)` | `sniff` |
| ci | `platform = cfg(windows)`, `package(sniff) + package(sniff-cli)` | `sniff` |
| ci | `test(/browser_/)` `retries = 0` | vacuous for claudine — the area has no `browser_` tier (`just test-browser` prints *"not applicable"*) |

### Phase 6 disposition

Measured on 2026-09-08 on a 16-core Mac, after Phase 5's burn-down and Phase
6's context and fixture work. Every number is a full-suite `just test` /
`just test-cli` per-test duration, not an isolated run.

| # | Filter | Then | Now | Action |
|---|---|---:|---:|---|
| 1 | `compose_loop_rate_limit_pause_waits_then_continues` | 4.04 s | 4.335 s | **kept, both profiles.** The elapsed time *is* the contract; it sits just under the default 5 s slow mark |
| 2 | `agents_and_commands_route_to_empty_state_messages` | 0.04 s | 0.05 s | **removed, both profiles.** The comment claimed "can exceed 30 s under contention"; it predates the fixture migration |
| 3 | `composition::loop_engine::…seeded_loop_repro…` | dead | dead | **removed, both profiles.** The module is `composition::looping::engine`; the real test runs in 0.023 s |
| 4 | `exhausted_remediation_fails_finalize_and_preserves_findings` | 0.13 s | 0.059 s | **removed.** The comment documented ~21 s alone / 83 s contended |
| 5 | `context_reports_preserve_all_columns_at_minimum_supported_width` | 5.21 s | **0.897 s** | **removed with the cost it hid** — the entry the spec names. The cost was 12 `TempDir` + `git init` fixtures inside one width sweep |
| 6 | `compose_perf_stdout_matches_non_perf` | 0.37 s | 0.476 s | **removed** |
| 7 | `inline_compose_perf_stdout_matches_non_perf` | 0.45 s | 0.530 s | **removed** |
| 8 | `package(claudine-cli) & test(/level2_/)` | — | — | **kept.** The L2 binaries own an internal ~40 s deadline that the 30 s default kill sits below |
| 9 | `test(/level2_/)` `retries = 0` | — | — | **kept** |
| 10, 11 | the two `test-group` bindings | — | — | **kept.** `claudine-cli-ci-l1` at `max-threads = 1` remains the largest CI cost lever in the area, and relaxing it still needs CI evidence this fix does not have |

**Finding: every per-test `slow-timeout` override in the `ci` profile was a
no-op.** All eight set `{ period = "30s", terminate-after = 3 }`, which is that
profile's own default — they changed nothing, in either direction, for as long
as they existed. The five Claudine-scoped ones are gone (entries 1's CI copy is
the exception, kept deliberately so the contract survives a change to that
default, with the redundancy stated at the entry); the three belonging to
darkmatter and biscuit-terminal are out of AC5's scope and are named above. The
profile's own comment now records the rule, so a ninth is not added by
inheritance.

The **default** profile is where a per-test entry still means something:
`{ period = "5s", terminate-after = 6 }` marks slow at 5 s and kills at 30 s, so
an override raises both. That is why entries 2–5 were live rather than inert.

## Recipe and package-metadata reconciliation

### `test-real` bypasses nextest

`claudine/justfile`'s `test-real` runs

```bash
CLAUDINE_CONTRACT_REAL=1 cargo test -p claudine-contract --features real-tests --test real_provider
CLAUDINE_CONTRACT_REAL=1 cargo test -p claudine-cli --features real-tests --test real_opencode_yolo_subagent
```

It is the only recipe in the area that uses `cargo test` rather than nextest.
Consequences: no JUnit report, no `slow-timeout`/`leak-timeout` policy, no
test-group scheduling, and the five identities it covers appear in no CI
artifact. The recipe's own comment explains the aggregation logic (one
unavailable provider must not mask the other's result), which `cargo test`
happens to make easy — but nextest's `--no-fail-fast` plus two invocations
would do the same. **Phase 6 owns bringing it onto nextest or recording why it
cannot.**

> **Resolved in Phase 6.** `test-real` now calls `just _test_real` twice, once
> per package, so it stages a JUnit report and obeys the slow/leak policy like
> every other tier. The aggregation reason survives unchanged: two invocations,
> `set -uo pipefail`, exit code aggregated. Verified — nextest selects the four
> `claudine-contract::real_provider` identities and the one
> `claudine-cli::real_opencode_yolo_subagent` identity. The four contract
> identities fail on the 2026-09-08 host with `Unauthorized`, and fail
> identically under the retired `cargo test` form, so that is a pre-existing
> host condition rather than a regression from the change.

### Rendezvous defines 8 of the 12 canonical recipes

`claudine/rendezvous/justfile` has `build`, `test`, `test-l2` (a stub that
prints *"not applicable"*), `lint`, `lint-fix`, `check`, `install`,
`poc-demo`. Missing: `sanity`, `test-l3`, `test-browser`, `test-real`,
`doctest`, `bench`, `coverage`, `fuzz`, `all`.

The absent `test-real` is not cosmetic: `rendezvous-daemon::peer_discovery`
holds two `real_*` identities, and with no `test-real` recipe **there is no way
to run them through a canonical route at all**.

> **Resolved in Phase 6.** All nine missing recipes were added, four of them as
> reasoned "not applicable" stubs (`test-l3`, `test-browser`, `bench`, `fuzz`)
> matching the area convention. `test-real` is the one with teeth: it runs the
> two mDNS identities, and they pass —
> `2 tests run: 2 passed, 169 skipped` — where before they compiled on every
> run and executed on none. `just check-tier-coverage` covers the stub recipes,
> and the two stubbed tiers select nothing in this area.

### Five crates declare no `[package.metadata.ci.tests]`

`claudine-contract`, `claudine-catalog-types`, `rendezvous-core`,
`rendezvous-daemon`, `rendezvous-client`. `scripts/ci/affected_scope.py`
defaults a package with no block to `gates = true`, `tiers = ["L1"]`,
`features = []`, `l1-include-slow = false`. So:

- `claudine-contract`'s four `real_provider` identities are gated behind the
  `real-tests` feature that CI never enables — correct, but undeclared rather
  than chosen.
- `rendezvous-daemon`'s two `real_*` mDNS identities are excluded by the L1
  filter and reachable by no recipe.
- `claudine` (the lib) declares only `[package.metadata.ci.native]`, so it too
  defaults to L1 with no features. That is correct for it and needs no change.

**Phase 6 owns declaring these or recording the deliberate default.**

> **Resolved in Phase 6.** All five carry a `[package.metadata.ci.tests]` block
> stating `tiers = ["L1"]`, `features = []`, `local-features = []` — the same
> route they were defaulted to — with a comment saying *why* that is right for
> each: the contract crate's `real_provider` target is deliberately outside the
> CI compile graph; `rendezvous-core`'s `test-support` feature widens a surface
> no shipped binary reaches; `rendezvous-daemon`'s two `real_*` identities are
> excluded by name and route through the new area `test-real`;
> `rendezvous-client`'s integration targets spawn an in-process daemon rather
> than an external resource. `scripts/ci/affected_scope.py --all` validates and
> accepts all five, and still rejects an unknown field (neuter transcript in
> `log.md`).

## Reachability findings

Four identities exist, compile, and run in **no canonical recipe and no CI
leg**. None was found by looking at timing; all four came out of reconciling
the runner listing against the tier filter.

| Identity | Why it is unreachable |
|---|---|
| `claudine-cli::wrap_sigint::slow_compose_sigint_during_prep_exits_130_with_notice` | named `slow_*`, so `_tier_filter L1` excludes it. Inclusion needs `BISCUIT_L1_INCLUDE_SLOW=1`, which comes from `[package.metadata.ci.tests] l1-include-slow`; only darkmatter's four packages set it. This is the **only** `slow_` test in the eight packages |
| `claudine-gen::signals_validation::real_corpus_builds_deterministically` | named `real_*`, so excluded from L1. `just test-real` covers only `claudine-contract` and `claudine-cli`, and `claudine-gen`'s metadata declares `tiers = ["L1", "L2"]` |
| `rendezvous-daemon::peer_discovery::real_two_daemons_discover_each_other_via_mdns` | named `real_*`; the rendezvous justfile has no `test-real` recipe and the crate defaults to the L1 tier in CI |
| `rendezvous-daemon::peer_discovery::real_mdns_discovered_peer_cannot_sync_before_approval` | same |

Nine further identities are `#[ignore]`d performance harnesses
(`completion_perf` ×4, `system_prompt_perf_bench` ×4, `compose_ttff_perf` ×1).
They are reachable only by `--ignored`, which no canonical recipe passes. They
assert budgets, so they read as coverage while proving nothing in an ordinary
run.

Dispositions: the four unreachable identities and the nine ignored harnesses
are **remediation in this fix** (Phase 6), recorded in the families that own
them. This fix does not change their tier markers unilaterally — AC5 and AC7
forbid a tier change as a substitute for a fix, and the correct resolution for
each is a declaration, a rename, or a recipe, decided with the numbers Phase 3
produces.

> **Phase 6 resolution.** All four unreachable identities now run. Two needed a
> **recipe** and two needed a **rename**; neither is a tier change used to hide
> a cost, because in every case the measured cost is under 0.2 s and the change
> *adds* the identity to a gate rather than removing it.
>
> | Identity | Cost | Resolution |
> |---|---:|---|
> | `wrap_sigint::slow_compose_sigint_during_prep_exits_130_with_notice` | 0.183 s | renamed `compose_sigint_during_prep_exits_130_with_notice`. The "slow" was the *fake provider's* 10 s sleep, which is the floor the test's own 4 s latency assertion measures against — interrupting it is the point |
> | `claudine-gen::signals_validation::real_corpus_builds_deterministically` | 0.092 s | renamed `shipped_corpus_builds_deterministically`. The "real" meant the shipped corpus as opposed to this file's tampered fixtures — an ordinary L1 subject reading files in the checkout |
> | `rendezvous-daemon::peer_discovery::real_two_daemons_discover_each_other_via_mdns` | 0.204 s | kept `real_*` — it binds a real multicast responder — and given the route it never had: `just test-real` inside `claudine/rendezvous` |
> | `rendezvous-daemon::peer_discovery::real_mdns_discovered_peer_cannot_sync_before_approval` | 0.204 s | same |
>
> The **nine `#[ignore]`d performance harnesses stay ignored**, as a recorded
> decision rather than an omission. Four (`system_prompt_perf_bench`) are
> explicitly diagnostic: they print step-by-step measurements and assert
> nothing, so they are a tool, not coverage. The other five
> (`completion_perf` ×4, `compose_ttff_perf` ×1) assert wall-clock budgets —
> p95 ≤ 100 ms and a time-to-first-frame ceiling — which are exactly the
> host-load-sensitive gates the spec's own measurement rules say must not be
> compared across unmatched conditions. Promoting them into the always-on L1
> gate would trade a silent non-assertion for a flaky one, and would need the
> `slow-timeout` override AC7 forbids. Their invocation is documented in each
> file's module doc; the route is `--ignored`, and that is the answer, not an
> oversight. Owner: this row.

## Sleep sites

Phase 7's audit of every `thread::sleep` / `tokio::time::sleep` in the eight
packages' **test** paths, plus the production sleeps the plan named. Three
dispositions: *observed* (replaced by bounded observation of the final required
condition), *cadence* (a poll interval inside a loop that already has a
deadline — this is what RB4 asks for, not what it forbids), and *contract* (the
elapsed time is itself what the test asserts).

Every budget below is derived under the implemented
[startup-stall fix](../_completed/2026-08-31-silent-success-and-startup-stall/spec.md)'s
spawn-fallback silence clock, not the pre-fix first-event grace.

### L1 — `claudine/cli/tests` (17 sites in 7 files)

The plan's grounding fact said 13 across 5 files; it omitted the two
`level1_*_pty.rs` binaries, which carry two apiece.

| Site | Sites | Disposition | Budget / cadence and why |
|---|---:|---|---|
| `common/pty.rs` `read_for` / `wait_for_marker` / `wait_for_raw_mode` / `wait_for_raw_mode_reentry` | 5 | cadence | 20 ms between `try_read` attempts, each inside a caller-supplied deadline (10 s for a marker, 5 s for a re-entry). 20 ms is below one frame of PTY output latency, so the observation granularity never dominates what is being waited for. |
| `level1_compose_autocomplete_failure_pty.rs` | 2 | cadence | 20 ms, same shape, own deadline loop. |
| `level1_inline_compose_mismatch_pty.rs` | 2 | cadence | 20 ms, same shape. |
| `wrap_sigint.rs` | 1 | cadence | 20 ms polling for the fake provider's readiness marker, with a deadline. The file's own comment records why a fixed sleep was replaced here: it was flaky under full-suite contention. |
| `completion_perf.rs` | 1 | cadence | 5 ms between `try_read` attempts while waiting for the chooser marker. Tighter than the PTY harness because the value being measured *is* a latency. |
| `wrap_ctrl_c_windows.rs` | 3 | cadence ×2, fixture ×1 | 50 ms polling for the wrapped grandchild's readiness marker (30 s deadline), 100 ms polling `try_wait` for termination (15 s deadline). The third is the fixture provider's own 60 s idle loop, not a test wait. |
| `sequence_ctrl_c_windows.rs` | 3 | cadence ×2, contract ×1 | 200 ms polling for every task's heartbeat (60 s deadline) and for the orchestrator's exit (60 s deadline). The contract sleep is 3 s between two heartbeat samples: the tick loop runs at ~1 Hz, so three ticks is the margin that makes "the counter did not move" mean "the tree stopped" rather than "we looked too soon". Windows-only; compile-verified by `just check-windows`, runtime behavior is `windows-latest`'s. |

**The finding that mattered was not in this table.** `sequence_overlay_pty`'s
seven tests each cost ~2.4 s, and 2.05 s of that was the *child* waiting out a
timeout the harness could have ended: `biscuit_terminal::Terminal` construction
writes OSC 11 and OSC 10 colour queries to `/dev/tty`, and a bare expectrl PTY
answered neither, so the child paid `DEFAULT_TIMEOUT` (1 s) twice before its
first byte. Measured on the PTY master:

```text
PROBE spawn      136 ms
PROBE-READ t=0.11s  "\e]11;?\a"     <- background query
PROBE-READ t=1.13s  "\e]10;?\a"     <- 1.02 s later: OSC 11 timed out
PROBE-READ t=2.18s  "…Sequence: 2 step…"  <- 1.05 s later: OSC 10 timed out
```

`common/pty.rs` now answers both on observation, which is the same contract it
already honoured for the DSR cursor probe and which `biscuit-terminal`'s own
`ProbeAnswer` documents ("answering on observation rather than after a fixed
delay is what makes a probe test independent of process-start latency"). The
replies are biscuit-terminal's own manufactured pair. Matching is against the
newly-read chunk rather than the cumulative transcript, so a reply is written
exactly once: a second one would arrive after the prompt entered raw mode,
where its leading `ESC` reads as a cancel keystroke.

`sequence_overlay_pty`: **17.88 s summed / 3.03 s elapsed → 4.66 s / 1.11 s.**

### Library — `claudine/lib/src`

| Site | Sites | Disposition | Budget / cadence and why |
|---|---:|---|---|
| `composition/sequence/task/tests.rs` reap waits | 5 | observed | Was `sleep(1600 ms)` then "the marker did not appear". Now the descendant publishes its pid under its own `/bin/sh -c` (POSIX `$$` inside `( … )` is the *parent* shell's pid, so a subshell would prove nothing) and the test polls `kill(pid, 0)` until it is gone, 10 s deadline, 5 ms cadence. This turns a negative inferred from a wait into a positive observation, and it lands as soon as the kill lands: **1.6 s → 0.04–0.05 s each.** |
| `composition/sequence/task/tests.rs` interrupt readiness | 1 | observed | Was `sleep(300 ms)` before setting the interrupt flag. The tree now `touch`es a marker as its first command and the setter thread waits for it (30 s deadline, 5 ms cadence), then the test asserts the watcher was released by the marker and not by its own timeout — so an interrupt that fired before anything ran fails instead of passing vacuously. **0.33 s → 0.06 s.** |
| `composition/sequence/task/tests.rs` Windows reap twin | 1 | contract | `cmd` has no portable way for a backgrounded process to publish its own pid. Retained at 1600 ms: the fixture writes at 1 s, so 600 ms is the margin a loaded runner has to reach that write. |
| `composition/sequence/task/tests.rs` `FakeTaskShell` per-command delay | 1 | contract | The delay *is* the fixture: it is how a parallel-group test inverts completion order. Where a test needs the *stages* ordered rather than merely the durations, `CommandGate` already supplies the guarantee and `gate_released()` fails a gate that timed out instead of ordering anything. |
| `composition/sequence/task/shell.rs` `POLL_INTERVAL` | 2 | cadence | Production. The runner's own poll of the child's status against the caller's deadline. |
| `render/assistant_stream.rs`, `render/thinking_stream.rs` idle flush | 5 | contract | 15–20 ms against a 5 ms threshold, i.e. a 3–4× margin, with a paired `flush_idle(60 s)` negative control in every case so the threshold is proved honoured rather than merely exceeded. `flush_idle` reads a real `Instant`; making these zero-cost needs an injected clock, which is a production change this fix puts out of scope. ~95 ms total. |
| `composition/looping/engine.rs` `interruptible_sleep` | 1 | cadence | Production, and already the shape RB4 asks for: it slices the pause into `PAUSE_POLL_INTERVAL` steps and returns early on interrupt. |
| `config/atomic.rs` persist backoff | 1 | cadence | Production, Windows-only, and the model case for the whole audit: the sleep is injected as `S: FnMut(Duration)`, so the retry tests pass a fake and no test sleeps at all. |
| `model_catalog/provider_sources.rs` `wait_for_user_interrupt` | 1 | cadence | Production. 50 ms, with the resulting worst-case interrupt-to-return latency stated at the call site. |
| `model_catalog/service/tests.rs` contention window | 1 | contract, with a caveat | 50 ms after the first fetcher has parked on `release.notified()`, to give the second caller room to reach the in-flight `OnceCell`. Without it the test still passes but degrades silently to the sequential path, so the sleep is what makes it exercise contention. Replacing it with observation needs a seam that counts callers *entering* `refresh_provider`, which is a production change; recorded here rather than made. |

### Retained sleeps outside the plan's list

`wrap_watchdog_startup_stall`, `wrap_watchdog_timeout`, `loop_cli`'s rate-limit
pause, `sequence_schema` and `wrap_opencode` express their budgets in *shell
fixtures* (`sleep 30`, `/bin/sleep 1` loops) rather than in Rust. Every one is a
timeout contract: the fixture outlives the deadline under test so that the
deadline firing is the only way the test can pass. Phase 6 already measured the
loop-pause family at 4.335 s and recorded that it is a real rate-limit wait that
should not move.

## Reconciler output

Run from the working tree at `78b44a96651e`, `2026-09-09`:

```text
| Package | Runner identities | Source attributes |
|---|---:|---:|
| `claudine` | 4151 | 4169 |
| `claudine-catalog-types` | 21 | 21 |
| `claudine-cli` | 2760 | 2775 |
| `claudine-contract` | 52 | 52 |
| `claudine-gen` | 158 | 158 |
| `rendezvous-client` | 21 | 24 |
| `rendezvous-core` | 82 | 88 |
| `rendezvous-daemon` | 172 | 185 |
| **total** | **7417** | **7472** |

cfg/feature exclusions (source-defined, runner never lists): 55
source diagnostics (142): tree-sitter-rust 0.24 grammar gaps (`&raw`, `raw` as
  an identifier, `unsafe extern`); each is a local ERROR node that loses no test
build targets listing no test: claudine-cli::level3_linux_sequence_ctrl_c,
  claudine-cli::level3_windows_sequence_ctrl_c,
  claudine-cli::sequence_ctrl_c_windows, claudine-cli::wrap_ctrl_c_windows,
  claudine-gen::bin/claudine-gen, rendezvous-daemon::bin/rendezvous-daemon,
  rendezvous-client::bin/rendezvous-test-client
GATE EXIT=0
```

Zero identities unassigned, zero double-assigned, zero stale families, zero
inventory drift, zero undeclared or stale exclusions.

The run at `9fc5151a0` that this replaces reported the same shape over
7,400 / 7,455 identities, 163 build targets, and the same 55 exclusions and
7 empty targets. Its inputs are preserved under
[`enumeration/9fc5151a0/`](enumeration/9fc5151a0/).

## Validation checkpoint 2

| Requirement | Status |
|---|---|
| The reconciler exits 0 | **done** — output above, reproducible with the command at the top |
| Inventory covers all eight packages | **done** — 7,417 identities, 165 build targets (7,400 / 163 at `9fc5151a0`) |
| …all tiers | **done** — L1, L2, L3 and real each have families; L3 and real are named *pending* rather than assumed |
| …doctests | **done** — 32, with the ignored and compile-fail splits |
| …benches | **done** — and four of the five entry points turned out to be empty files |
| …excluded/ignored tests | **done** — 55 platform exclusions listed separately, 9 `#[ignore]`d harnesses named, 4 unreachable identities named |
| …shared fixtures | **done** — 9 components, with `common/mod.rs`'s ordering contract recorded |
| Every row has a disposition | **done** — checked mechanically; the gate rejects an empty or unrecognized one |
| The override census is complete | **done** — 24 blocks, 11 Claudine-scoped with a verdict each, 7 attributed to other packages |
| No row dispositioned by timing threshold | **done** — every disposition names a contract, a reachability defect, or a pending tier |

