---
spec: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/sniff/fixes/2026-11-20-commit-link-walks/spec.md"
plan: "sniff/fixes/2026-11-20-commit-link-walks/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
message_to_agent: |-
  Phase 1 captured the baseline only; no source was edited. Raw samples live in
  target/commit-link-walks-baseline/ (ignored). Baseline at HEAD fce8e78bb: text
  `repo git-status --perf` median 3537 ms, `--json --perf` median 147 ms; both
  report git.commit_visits 84, ref_walks 1, repository_discoveries 1, and no
  filesystem.git.commit_links stage. The remote-history union is 16,682 commits
  over 25 non-symbolic origin tips (origin/HEAD is symbolic and excluded); all 10
  displayed commits are absent from every remote; no commit-graph exists.
  For Phase 4: HEAD will have moved past fce8e78bb as phases are committed, so the
  ten displayed targets will differ. Before re-sampling, diff
  `git for-each-ref refs/remotes --format='%(refname) %(objectname)'` against
  target/commit-link-walks-baseline/remote-refs.txt (md5
  ab99d39475562ab63a939bef11d32d4c), recount the union with git rev-list --count,
  and confirm the new ten targets are still absent remotely. Then the
  linked-versus-JSON visit delta should equal that union.
---

# Implementation Log for 2026-11-20-commit-link-walks (4 phases)

## Phase 1

### Capture baseline (complete)

No source files were edited in this phase. Everything below was measured before any Phase 2 change.

**Build.** `cargo build --release --locked -p sniff-detection-cli --bin sniff` finished as already fresh (0.87 s) against the current source. Binary: `/home/build/wt/rusty-biscuit/fix-biscuit-terminal/target/release/sniff`, invoked with `--base /home/build/wt/rusty-biscuit/fix-biscuit-terminal`.

**Repository state at sampling.**

- Source HEAD: `fce8e78bba99d2c874b39eae068f09cc753f1016` on `fix/biscuit-terminal`. This differs from the `fb35e1684` noted at planning time: the branch has gained commits since then, but the workload shape below is unchanged.
- Remote refs: 26 under `refs/remotes/origin/`, including the symbolic `origin/HEAD -> refs/remotes/origin/main` (`36097348949d…`), which `remote_branch_tips` excludes. The full name/ID list is saved as `target/commit-link-walks-baseline/remote-refs.txt` (md5 `ab99d39475562ab63a939bef11d32d4c`). Phase 4 should diff against this file to prove the tips are still pinned.
- Distinct reachable commits over the 25 non-symbolic remote tips: **16,682** (`git rev-list --count <tips>`). HEAD alone reaches 16,683.
- Ten displayed targets (`git log -10 HEAD`): **10 of 10 are absent** from every remote branch (`git branch -r --contains` returned 0 for each).
- Commit graph: **absent**. `/home/build/coding/rusty-biscuit/.git/objects/info/` (the common dir) holds no `commit-graph` file or `commit-graphs/` chain.

**Samples.** I ran one warm-up per mode, then three interleaved runs each of `repo git-status --perf` and `repo git-status --json --perf`. Wall time is measured externally (`date +%s%N` around the process). Load average was 0.82 before and 1.22 after, on 16 cores. Raw stdout, stderr and wall times are in `target/commit-link-walks-baseline/`, which is ignored by `.gitignore` `**/target/*`.

| Run | text `--perf` wall | JSON `--json --perf` wall |
| --- | --- | --- |
| warm-up | 3524 ms | 150 ms |
| 1 | 3494 ms | 146 ms |
| 2 | 3537 ms | 150 ms |
| 3 | 3539 ms | 147 ms |
| **median** | **3537 ms** | **147 ms** |

- Reported in-process `Total` (text): 136.7 / 131.8 / 137.4 ms. JSON `total_duration_ms`: 136.3 / 140.7 / 137.7 ms. Text and JSON report the same detection cost, so the ~3.4 s text excess is entirely post-snapshot renderer work.
- Reported `git.commit_visits`: **84** in every text run, every JSON stdout `performance` object, and every JSON stderr diagnostic. The other counters are identical in both modes: `ref_walks` 1, `repository_discoveries` 1, `repository_opens` 1, `status_walks` 2.
- Linking stage: **absent** in both modes. The stages are `detect.filesystem`, `detect.total`, `filesystem.docs`, `filesystem.git`, `filesystem.inventory`, `filesystem.repo`, and no `filesystem.git.commit_links` appears. As the plan expects, the baseline counters exclude the renderer's `commit_links_at` lookups: they run after `CliPerf::build_report`, so the ~3.4 s of independent ancestry walks is invisible to the counters.
- The acceptance target for Phase 4 is terminal status within 0.5 s of focused JSON. The baseline gap is about 3.39 s (3537 − 147 ms).

### Gates

The phase changed only the plan and this log. I ran this from the repository root after saving all edits:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

It reported `packages: []` and `change_class: documentation`, so this phase skipped `just test` and `just lint` in `sniff/`. Phase 1 added no tests, so there is no requirement-to-test mapping for it. Phase 2 owns the first behavior tests.

### Cross-OS rigs

No local test rigs are configured. `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL` and `BUILD_MACOS` are all unset, so the baseline is Linux-only, as the plan specifies.
