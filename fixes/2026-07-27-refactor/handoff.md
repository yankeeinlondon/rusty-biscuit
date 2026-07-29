---
title: Implementation handoff — cross-platform CI refactor
status: implemented, unverified on a real runner
created: 2026-07-28
implements: fixes/2026-07-27-refactor/plan.md
---

# Handoff

Everything below is implemented and green against local gates. **Nothing has run
on a GitHub runner.** The devops handoff records five CI-only bugs that passed
every local check, so treat the branch PR run as the real gate.

## Merge ordering — read this first

The pieces are individually safe and collectively dangerous in the wrong order.

1. **Do not merge `soft_os` removal without the baseline + `ci-verdict`.**
   `continue-on-error` did not make a leg non-blocking, it removed the leg from
   the run's verdict. Deleting it makes ~14 red Windows areas merge-blocking
   immediately. `.github/ci/ci-baseline.toml` plus the `ci-verdict` job are what
   absorb that. They are in the same change set — keep them together.
2. **Branch protection must move to `ci-verdict` in the same step.** Expected-red
   producer jobs must stay visibly red but must *not* be required checks, or
   their failure bypasses the verdict. Not yet done — deliberately.
3. **The baseline is entirely unverified.** All 31 entries were migrated with
   `reason: "migrated from baseline-failures.txt, unverified"`. Three are known
   stale and will fire `baseline-now-passing` on the first run: `biscuit-speaks`
   lint, `biscuit-terminal` lint, `darkmatter/ubuntu/3-4`. That is the retirement
   path working, not a bug — remove them.
4. **Two new blocking classes have zero baseline entries**: `check` cells, and
   L2/browser cells that go MISSING when L1 fails upstream. Expect first-run
   noise there.

Judge the first run by **diffing the failure set against `main`**, never by "is
it green" — the devops handoff's ONE RULE.

## What is deliberately switched off

| Thing | State | Unblocks when |
|---|---|---|
| `wsl2-ubuntu` | built, in no area's `environments` | someone opts an area in; `_wsl-ci.yml` is `workflow_dispatch`-able for validation first |
| 6 promotable areas | recipes written, still `ci: false` | the baseline is validated on a real run |
| Windows L2 | owned policy gap, expires 2027-01-31 | plan §2.3 spike (`wezterm-mux-server`) |
| branch protection | untouched | step 2 above |

## Corrections to the plan

- **§3.5's reference case is wrong.** `homelab` is cited as the worked example of
  a legitimate capability-based exclusion. It is `ci: true` — it gates on
  build/lint/L1 with 248 tests, and only its `test-real` tier touches hardware,
  which the matrix never runs. Its exclusion is *tier*-level inside a CI-enabled
  area. No `ci: false` record has that shape, so measured against the plan's own
  standard, none of the 10 was a legitimate permanent exclusion.
- **§4.1's per-cause counts do not reconcile.** 8 + 14 + 2 = 24 forces cause 3 to
  be 1 test, not the stated 2. The breakdown also omits 2 `completion_round_trip`
  failures, and fixing the library required pre-emptively touching two
  *non-failing* files — reasoning from the 24-item list alone would have traded
  24 red for a different set of red.
- **§1.2's constraint names the wrong axis.** "Never reduce tests within an area"
  is enforced against platform only. See below.

## The feature-flag hole — the biggest thing the plan missed

Cargo features reduce tests within an area invisibly, and `--expected-manifest`
**cannot** detect it in principle: the manifest is generated from the same build
configuration it audits, so both sides shrink together and the cell reads a clean
PASS.

Measured, repo-wide:

| area | as-invoked | `--all-features` | hidden | gating feature |
|---|---|---|---|---|
| schematic | 302 | 535 | **+233** | `openapi` |
| playa | 50 | 188 | **+138** | `audio-ducking`, `sound-effects`, … |
| biscuit-terminal | 2867 | — | **+137** | `image` |
| messenger | 398 | 451 | +53 | 7 provider transports |
| biscuit-speaks | 357 | 380 | +23 | `playa` |
| biscuit-hash | 26 | 46 | +20 | `blake3`, `argon2id` |
| biscuit-file | 727 | 747 | +20 | `fetch` |
| biscuit-icon, biscuit-tui, sniff, homelab | | | +7 total | various |

**Fixed** (feature is what in-repo consumers actually build, and pulls no
network/device/GUI/paid API): schematic, biscuit-terminal, biscuit-speaks,
biscuit-hash, biscuit-icon, sniff — **≈419 tests now run that never ran before**.

**Left alone, needing a judgement call not a recipe edit:** `messenger` (live
provider transports), `playa` (real audio devices, ~30 MB assets), `biscuit-file`
(`fetch` = reqwest; note `darkmatter` enables it, so those 20 tests cover a
shipped configuration), `biscuit-tui` (`renderables` deliberately off, no in-repo
consumer enables it), `homelab/unfolded-integration-helper` (`mdns` multicast).

### Recommended follow-up

A root `just check-feature-coverage`: per area, `cargo nextest list` twice — once
with the area's real spec, once with `--all-features` — and fail when the latter
is a strict superset, printing the excess identities and the features gating
them. `--all-features` is the right *reference for detection* even where it is
the wrong thing *to run*.

Run it on the nightly/`coverage.yml` schedule, not per-PR: it doubles compile
cost, and `--all-features` does not currently build for `biscuit-terminal`.

Store exemptions as a `feature_gap` block per area in `areas.json` with the same
`owner`/`reason`/`expiry` shape as `policy_gaps`, validated by
`affected_scope.py`. That turns "these 53 messenger tests never run" into an
owned, expiring record instead of an invisible property of a recipe string.

## A PR can schedule no run at all — now guarded

`pull_request` workflows execute against the merge commit `refs/pull/N/merge`.
GitHub cannot create that ref while a PR conflicts with its base, so it creates
**no workflow run at all — silently**. No check suite, no annotation, nothing in
the Actions tab. The PR renders exactly like one whose CI has not started, and
`ci-verdict` never reports, so nothing blocks and nothing explains why.

This bit PR #19: a conflicted `baseline-failures.txt` suppressed the entire
matrix, undetected until mergeability was inspected by hand. It is the same
defect class as `soft_os` — a leg that does not run and produces no signal.

`.github/workflows/pr-health.yml` closes it. The guard cannot live in a
`pull_request` workflow, since that is precisely what does not fire; it uses
`pull_request_target` (runs against the base branch, needs no merge commit) plus
`push` as a backstop for a conflict introduced by a later push. It never checks
out pull-request code, which a contract test enforces along with the trigger
choice and the requirement that a conflict *fails* rather than warns.

Two implementation notes worth keeping:

- `mergeable` is computed asynchronously and reads null until GitHub's
  background job finishes, so the check polls. A persistent null warns rather
  than fails — gating merges on a background job would trade one silent failure
  for another.
- `gh api` switches to POST as soon as a `-f` flag is present. `gh api
  repos/OWNER/REPO/pulls -f state=open` therefore calls *create-a-pull-request*,
  403s, and — if stderr is discarded — puts its JSON error body on stdout where
  an unvalidated capture treats it as a PR number. Put the query in the URL, and
  validate what you capture.

## Pre-existing breakage found, not fixed

1. **`scripts/drift.rs` does not compile** — 6 errors, `fallback_render` was
   removed from `TerminalRenderable`. `cargo nextest run --manifest-path
   scripts/Cargo.toml` was already red before this work. Sidestepped by putting
   `repo-deps`/`drift` behind a `local-tools` feature; `ci-rollup` builds with
   `--no-default-features` in ~8s instead of 40s+.
2. **`biscuit-terminal --all-features` does not compile** — 28 errors in the
   `#[cfg(feature = "serde")] mod serde_roundtrip` block of
   `components/horizontal_rule/mod.rs`; calls pass 6 args to a 7-arg
   `horizontal_rule_svg`. Dead since the signature changed; nothing ever built it.
3. **`biscuit-terminal` `horizontal_rule_parity`** — 2 tests fail once
   `--features image` is enabled. Verified dormant: the file is byte-identical to
   `main` and `main`'s recipe never passed the feature. Left red.
4. **`messenger just lint`** — pre-existing `clippy::unneeded-wildcard-pattern`
   at `messenger/lib/src/prepared.rs:117`.
5. **`tree-hugger-cli::level2_god_files_pretty_report_in_tmux`** — red on macOS
   independently of any change here.

## Smaller follow-ups

- **Wire `backend-executions.jsonl` into `ci-rollup`.** §1.1's execution proof
  exists but the rollup does not consume it. Without it, a policy gap that
  genuinely *closes* cannot be detected — `expiry` is the only forcing function.
  A `require_level!` early return renders as a JUnit pass, so passing counts
  cannot distinguish "ran" from "skipped cleanly".
- **`test_packages` in `areas.json`.** The per-area package spec is currently
  hand-copied into workflow YAML. Validate it in `affected_scope.py` instead —
  the same second-copy drift `_tier_filter` was introduced to eliminate.
- **Specialized workflows emit no JUnit.** `rendezvous-tests`, `playa-windows`,
  `messenger-desktop-tests`, `biscuit-tui-windows-captured-stdout` contribute
  only a job-level pass/fail and are reported `baseline-out-of-scope`. §3.5 wants
  them in the verdict.
- **`build`/`lint`/`check` still run at default features** in the six areas whose
  *test* recipes were widened — clippy still never sees that code.
- **`features/2026-07-27-local-runners/spec.md`** reasons at length about
  `soft_os` as a rejected alternative. That argument is now moot; it needs its
  owner, not a find-and-replace.
- **Recorded vs measured test counts drift** in all 6 promotable areas (e.g.
  `biscuit-browser-harness` recorded 13, has 6; `messenger` recorded 564,
  contract-discovers 502). Reconcile before flipping promotion.

## Local gates — all green

```
actionlint -shellcheck=                                        exit 0
cargo nextest run -p test-toolkit --test ci_workflow_contracts  38/38
python3 scripts/ci/test_affected_scope.py                       60/60
cargo nextest run --manifest-path scripts/Cargo.toml
  --no-default-features                                        108/108
cargo nextest run -p test-toolkit                              104/104
just check-canonical                                       27/27 areas
just check-test-interrupts                                 34/34 areas
```

Every new guard was proven non-vacuous by neutering it, confirming red, and
restoring — 40+ mutations across the work.
