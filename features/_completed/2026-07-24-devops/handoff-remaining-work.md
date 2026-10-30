# DevOps CI/CD — handoff (rewritten 2026-07-27)

Source of truth: `features/2026-07-24-devops/{spec.md, plan.md, execution.md,
ci-failure-inventory.md}`. Read `ci-failure-inventory.md` first — it explains why
CI looks catastrophic and isn't.

## THE ONE RULE THAT MATTERS

**A full-scope CI run cannot be green, and could not before this work.** 31 jobs
were already failing on `main` before any of it. Judge a branch by **diffing its
failure set against the baseline**, never by counting reds:

```bash
BASE=/tmp/baseline.txt        # regenerate from any recent main run if lost
gh pr checks <pr> --json name,state \
  | jq -r '.[]|select(.state=="FAILURE")|.name' \
  | sed 's/^canary \/ //' | sort -u > /tmp/f<pr>.txt
comm -13 "$BASE" /tmp/f<pr>.txt        # ONLY the new failures
```

For each new failure, open the job's step list
(`gh api repos/yankeeinlondon/rusty-biscuit/actions/jobs/<id>`). If the steps the
branch changed passed and a product/test step failed, the branch is sound.
Confirm with `gh run list --workflow=<wf> --branch main`.

Fixing the 31 baseline failures is an **explicit spec non-goal** — they belong to
their owning areas. `ci-failure-inventory.md` breaks all 34 down to ~14 root
causes (mostly one-line lint fixes; `biscuit-terminal`'s three dead-code items
alone account for six red jobs).

## Merged to `main`

| PR | What |
|---|---|
| #6 | Canary set trimmed to green areas (dropped `darkmatter`) |
| #5 | One native-library installer (`just _ensure-native-libs`), run before every build |
| #10 | Removed three shadow workspaces (`schematic`/`unchained-ai`/`tree-hugger`) |
| #11 | homelab justfile no longer needs `sniff` to load + `check-canonical` guard |
| #7 | Task 4.4 orchestration + failure-class summary |
| #8 | Phase 5 scheduled automation |
| #9 | CI-aware `just commit` (+ plan/execution doc updates) |
| #13 | One-file area model (`areas.json` absorbs `exemptions.json`) |
| #14 | `maintenance-audit` broken pipe (found by dispatching it) |
| #15 | `maintenance-audit` exit 101 without cargo-nextest |

Nothing from this feature is open. #7/#8/#9/#13 each measured exactly **two** new
failures vs the then-current baseline — `messenger-desktop / WSL2 ubuntu` and
`rendezvous / native (windows-latest)` — both pre-existing failures that
orchestration only *renames* into the `ci` umbrella (red on `main` continuously
since 2026-06-17 and 2026-07-23, in steps this work never touched). Both are now
recorded in `baseline-failures.txt`, so a current branch should show **zero**.

## Then: remaining work

- **Phase 4/5 validation checkpoints** are ticked; evidence is in `execution.md`
  under "Validation checkpoints 4 and 5 closed".
- **AC30 is resolved.** The first cold `bench-nightly` run took **34.7 min against
  the 90-min budget** — and 34.7 min is exactly why every earlier cold run died
  under the old 30-min ceiling. Warm runs are 14–18 min. There is ~2.6x headroom;
  the budget can be tightened without splitting the 16 bench targets.
- **`messenger-desktop-tests.yml`** still installs `libdbus-1-dev` inline even
  though its area record now declares it, because that workflow does not call
  `just _ensure-native-libs`. Switching it over is a small, safe follow-up.
- **Coverage runs L2 tests.** `cargo llvm-cov --workspace` shells out to `cargo
  test --tests --workspace`, bypassing the nextest filterset that excludes
  `level2_`, so real-terminal tests run headless there. Belongs with #16.
- **Windows work** — Ken has a Windows environment now and will do it in one go.
  Ctrl+C tests were re-tiered to ordinary `#[cfg(windows)]` L1 tests
  (`wrap_ctrl_c_windows.rs`, `sequence_ctrl_c_windows.rs`); they have **never
  run**, because `claudine / test` is staged behind `claudine / lint (ubuntu)`,
  which fails on one clippy hint in `messenger`.

## Hard-won gotchas — do not relearn these

- **CI-only bugs are the norm.** Five so far passed every local check: composite
  `if:`+local-`uses:` load error; `kache-action@v1` rejects `win32-x64`;
  `${VAR:-{}}` → jq `{}}`; a red canary blocking all fan-out; a Windows path
  separator in an error message. **Verify every workflow change on a branch PR run.**
- **`just --dry-run` does NOT evaluate top-level backticks.** It cannot reproduce
  a `VAR := \`cmd\`` load-time failure. Use a real recipe run.
- **A real `just <recipe>` run evaluates ALL top-level backticks**, even for
  unrelated recipes. `check-canonical` now rejects non-portable ones.
- **Local `actionlint` hangs on `ci.yml`** via its shellcheck integration. Use
  `actionlint -shellcheck=`; run shellcheck on `run:` blocks separately.
- **Scheduled workflows get no PR run**, and a *new* one cannot even be
  dispatched until it is on `main`. Extract their `run:` blocks and execute them
  locally with `GITHUB_STEP_SUMMARY` pointed at a temp file — that caught three
  silent defects in `maintenance-audit`/`bench-nightly`.
- **…but a local dry-run is necessary, not sufficient.** Two more defects
  survived it and needed a live dispatch (#14, #15). Also run the block with the
  tool under test removed from `PATH`: the runner has only what the job installs,
  and `cargo nextest --version` exits 101 there while passing on any dev host.
- **A zero-delta vs baseline does not mean a changed code path works.** #13
  deleted `exemptions.json` but left `$exemptions_json` referenced in
  `_ensure-native-libs`, silently installing nothing on Linux. The only job that
  exercises that path was already red for another reason, so the breakage was
  invisible to a failure-set diff. Two red jobs can hide each other — exercise a
  changed path directly.
- **`ci.yml` sets `cancel-in-progress: true` on `ci-${{ github.ref }}`**, so
  back-to-back merges cancel `main`'s full-scope run. Do not expect a completed
  `main` run to regenerate the baseline from during a merge sequence; prefer
  adding known entries over regenerating (a surplus entry is harmless to
  `comm -13`, a missing one is a false alarm).
- **Contract tests substring-match `areas.json` formatting.** Reformatting the
  file (e.g. `json.dumps`) breaks them.
- **Matrix job outputs are last-writer-wins**, so a reusable workflow's legs
  cannot report a failure class back to the caller. Hence the per-area
  `classify` job writing to the shared run summary.
- **A red canary blocks the entire fan-out.** Canary areas must be otherwise green.
- Repo rules: never `cargo fmt`; **no AI attribution in commits or PR bodies**;
  commit only when asked; regenerate a skill's `hash:` with `md hash <file>`
  after editing it.

## Local gates

```bash
actionlint -shellcheck=
cargo nextest run -p test-toolkit --test ci_workflow_contracts
python3 scripts/ci/test_affected_scope.py && rm -rf scripts/ci/__pycache__
just check-canonical
```
