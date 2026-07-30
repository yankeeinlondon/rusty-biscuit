---
title: Live CI measurement — reference run 30323254931
status: measured
created: 2026-07-27
measures:
  - run 30323254931 (fix/linux-darkmatter, pull_request, completed failure)
purpose: |
  plan.md tags every count with run 30323254931 and instructs the implementer to
  re-measure before acting. This is that re-measurement, taken 2026-07-27.
---

# Live CI measurement

Reference run: [30323254931](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/30323254931)
— `fix/linux-darkmatter`, `pull_request`, concluded **failure**.

Job outcomes: **79 success, 28 failure, 58 skipped, 1 cancelled**.

Later `main` runs (30330445785, 30315635772) concluded *success*, but they are
narrow-scope runs — not evidence that the failures below are resolved. This is
the plan's success criterion 7 working as designed, and it is why a run's
top-line conclusion is not a usable signal.

## Confirms the plan

| Plan claim | Measured | Verdict |
|---|---|---|
| 14 red Windows areas | 14 distinct areas fail `test (windows-latest)` | ✅ |
| Lint blocks 5 areas, not the baseline's 7 | 4 failed + 1 cancelled = 5 | ✅ |
| `baseline-failures.txt` is stale in both directions | `biscuit-speaks` / `biscuit-terminal` lint both pass now | ✅ |
| `sniff` fails on macOS | `sniff / test (macos-latest)` fails, as do its ubuntu and windows legs | ✅ |
| `claudine` was *cancelled*, not failed | `claudine / lint (ubuntu)` = cancelled | ✅ |

### Windows L1 failures (14 areas)

`biscuit-file`, `biscuit-icon`, `biscuit-speaks`, `biscuit-terminal`,
`biscuit-tui`, `playa` (canary), `darkmatter` (all 4 shards), `model-citizen`,
`queue`, `renderable`, `schematic`, `sniff`, `unchained-ai`.

### Linux L1 failures

`darkmatter` shards 1/4, 2/4, 4/4 (3/4 passes — a sharded area is not uniformly
red, which the current display cannot express), and `sniff`.

### Lint failures blocking their areas' L1 legs (5)

`homelab`, `research`, `tree-hugger`, `worktree` (failed) + `claudine`
(cancelled).

## New findings the plan does not record

### 1. Skipped matrix legs render with an un-interpolated job name

Five jobs appear literally as:

```
claudine / test (${{ matrix.os }}${{ (((matrix.shard != '1/1') && format(' {0}', matrix.shard)) || '') }})
```

for `claudine`, `homelab`, `research`, `tree-hugger`, and `worktree` — exactly
the five areas whose lint failed or was cancelled. When `needs: lint` skips a
matrix job, GitHub never evaluates the matrix context, so the name expression is
reported raw and **the whole matrix collapses into one skipped job**. The
individual OS/shard legs have no rows at all.

Two consequences, both already anticipated by the plan and now evidenced:

- **§0.3** — a lint failure does not merely block the L1 legs, it erases them
  from the job list. There is no artifact, no name, and nothing to key policy to.
- **§1.3** — "do not key policy to mutable GitHub display names" is not a
  stylistic preference. For a skipped area, the display name is not merely
  unstable, it is *unresolvable*. Any baseline keyed to display names silently
  fails to match these rows. This is the direct argument for the
  `{area, environment, tier, shard}` identity and for `NOT SCHEDULED` being a
  distinct cell state from `MISSING`.

### 2. Specialized workflows are red and outside the verdict

| Job | Conclusion |
|---|---|
| `messenger-desktop / messenger / desktop / WSL2 ubuntu` | failure |
| `rendezvous / rendezvous / native (windows-latest)` | failure |
| `Coverage for affected packages` | failure |

None are in `areas.json`, so none feed the area rollup. §3.5 requires them to
emit the same result schema and feed the same verdict.

### 3. WSL2 already exists in CI, and fails during provisioning

`messenger-desktop-tests.yml:64` has a `wsl-tests` job. It fails **before any
Rust work**, inside `Vampire/setup-wsl`:

```
E: Sub-process /usr/bin/dpkg returned an error code (1)
##[error]The process 'C:\Windows\system32\wsl.exe' failed with exit code 100
```

i.e. `update: true` plus `additional-packages` broke during apt. The job also
confirms every defect §2.1/§2.2/§3.3 describes:

- `wsl-version: 1` is pinned (plan: remove it; the action already defaults to 2)
- rustup is installed *inside the guest* and the repo is compiled there
  (plan §2.2: replaced by `nextest archive`)
- it builds over the default `GITHUB_WORKSPACE`, i.e. `/mnt/c`
  (plan §2.1: clone into the guest filesystem instead)
- it uses `cargo test`/`cargo check` directly, not the canonical `just` recipes,
  so it is outside the "same canonical recipe" objective entirely

This is a rewrite target, not a repair target.

## Caveats

- Counts are per **job**, not per **test**. Per-test counts are exactly what the
  Phase 0.2 rollup exists to produce, and cannot be derived until 0.1 lands —
  the uploaded JUnit currently holds only the last package of each multi-package
  area.
- `baseline-failures.txt` entries migrated into the new baseline must be marked
  unverified. This measurement is a single run on a single branch; it establishes
  the shape of the backlog, not a per-test ground truth.
