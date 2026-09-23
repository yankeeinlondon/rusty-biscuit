# Phase 1 baseline

Recorded before any feature production/test edit, per the plan's Phase 1 first
task. This is the reference every later checkpoint compares against.

## Starting state

| Item | Value |
|---|---|
| Commit | `6cdb8bf56321c3747d5ea16a1241e47c2bff7fce` |
| Branch | `proxy-with` |
| Worktree | `/Users/ken/.claudine/worktrees/rusty-biscuit/proxy-with` |
| Host | macOS (Darwin 25.5.0) |
| Date | 2026-07-16 |

### Pre-existing worktree changes at plan time

Documentation only — no production or test source was modified, so the baseline
is attributable to the recorded commit without needing a disposable worktree:

- `M claudine/features/2026-07-13-proxy-with/plan.md`
- `M claudine/features/2026-07-13-proxy-with/spec.md`
- `?? claudine/features/2026-07-13-proxy-with/review-1.md`

### Authorization

The spec is `status: draft`, and the plan gates Phase 1 repository changes on
owner authorization. The task prompt commissioning this phase is that explicit
authorization to work against the reviewed draft.

## Result: `just test` (L1), clean HEAD

| Package | Result |
|---|---|
| `claudine-catalog-types` | 21 passed, 0 skipped |
| `claudine` (lib) | 3423 passed (73 slow, 2 flaky), 7 skipped |
| `claudine-contract` | 47 passed, 5 skipped |
| `claudine-cli` | **1962 passed, 0 failed** (115 slow, 13 flaky), 157 skipped |
| `claudine-gen` | 110 passed, **1 failed**, 4 skipped (41 not run — fail-fast) |

### Pre-existing failure 1 — `claudine-gen::drift` (attributable to HEAD)

`committed_generated_artifacts_match_phase_1_byte_baseline` fails on a clean
tree at the recorded commit. An archived review moved its fixture; the failure
predates this feature and is **not** a Phase 1 regression. It fail-fasts, hiding
41 later `claudine-gen` tests.

**Do not treat this as waived** — it is recorded, not fixed. Later checkpoints
compare against this exact failure; a *different* `claudine-gen` failure is a
regression.

### Pre-existing failure 2 — `inline_compose_hash` (environment, now resolved)

`inline_compose_writes_hash_that_passes_md_diff` initially failed with
`md binary not found at target/debug/md`. It is a host tooling gap, not a code
defect: the test shells out to Darkmatter's `md` CLI, which the `claudine`
`just test` recipe does not build. Resolved for this baseline by running
`cargo build -p darkmatter-cli --bin md`. **A fresh clone or a cleaned `target/`
will reproduce it** — build `md` first.

### Flakiness

13 flaky in `claudine-cli`, 2 in `claudine` (all passed on retry). Consistent
with the recorded guidance that L1 timings drift under concurrent load. Do not
compare counts across runs without a drift bracket.

## Result: `just lint`, clean HEAD

Green.

## Result: `just test-l2`

Not run at the recorded commit. The Phase 1 L2 addition
(`level2_lifecycle_initialize_proxy_to_looping_target_matches_direct_run`) ships
`#[ignore]`d precisely so it cannot move the L2 baseline, and no other Phase 1
change touches an L2 surface. Checkpoint 1 requires only `just test` + `just lint`.
Phase 5 is the first checkpoint that gates on `just test-l2`; establish the L2
baseline there.

## How the guards were verified to bite

A drift guard that cannot fail is not a guard. Both Phase 1 guards were checked
against their own failure modes rather than only against a passing tree:

- `check_baseline` is unit-tested for a new site, a stale entry, and a changed
  call count (`composition_seams.rs::scanner_tests`).
- The resolver fix's regression test was run against the **unfixed** production
  code and observed to fail with `got Continue { next_attempt: 1 }` — i.e. the
  nonexistent target was adopted as the active document — then to pass with the
  fix. That is the defect the plan describes, reproduced and closed.

## Expected, sanctioned drift introduced by Phase 1

`claudine/docs/providers/dispatch-inventory.json` regenerated: 1356 → 1357
sites. The new L1 regression test names `Provider::Goose` to build its fixture,
and the inventory's exemption rule covers `/tests.rs` but not in-crate `tests/`
**directories**, so a sibling test module counts as a site. This is a
pre-existing property of that guard, not a new dispatch site in production code;
regeneration is its documented flow.
