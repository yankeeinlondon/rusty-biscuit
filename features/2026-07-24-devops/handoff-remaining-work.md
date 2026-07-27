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

## Open, merge in THIS order

1. **#7** `devops-phase-4-orchestration` — Task 4.4 + failure-class summary
2. **#8** `devops-phase-5-scheduled` — Phase 5 scheduled automation
3. **#9** `devops-ci-commit` — CI-aware `just commit` (+ plan/execution doc updates)
4. **#13** `exempt-native-deps` — one-file area model

#7→#8→#9 is a linear stack; each contains the ones below it, so the diffs shrink
as they merge. **#13 must go last**: #7 edits `.github/ci/exemptions.json`, which
#13 deletes. After #9 merges, rebase #13 onto main and drop the exemptions hunk.

**Verified deltas (all measured, all acceptable):** #7/#8/#9 each show exactly
**two** new failures vs baseline — `messenger-desktop / WSL2 ubuntu` and
`rendezvous / native (windows-latest)`. Both are pre-existing failures that
orchestration newly *reveals*: red on `main` continuously since 2026-06-17 and
2026-07-23, failing in `Vampire/setup-wsl@v4` and the Rendezvous suite — steps
this work never touched. #13 was still queuing at handoff; check its delta before
merging.

After merging, rebase each next branch onto the new main and re-verify
(`actionlint -shellcheck=`, `cargo nextest run -p test-toolkit --test
ci_workflow_contracts`, `python3 scripts/ci/test_affected_scope.py`,
`just check-canonical`).

## Then: remaining work

- **Phase 4/5 validation checkpoints** in `plan.md` are still unticked. Dispatch
  `maintenance-audit.yml` once #8 lands — it is new, so it cannot be dispatched
  until it is on `main`.
- **AC30 caveat (bench-nightly).** Budget is 90 min, provisional. Warm runs
  measured 14–18 min; every cold run was truncated by the old 30-min ceiling, so
  the cold duration has **never been observed**. The job now records its own
  duration/runner/toolchain — tighten from the first cold run, or split the 16
  bench targets.
- **`messenger-desktop-tests.yml`** still installs `libdbus-1-dev` inline; after
  #13 it can be declared instead.
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
