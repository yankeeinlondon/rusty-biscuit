---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-08-improved-performance/spec.md
plan: tree-hugger/features/2026-10-08-improved-performance/plan.md
implemented_by: claude/opus
started_phase: 1
packages: []
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
skills_files_updated_during_phase_1: []
---

# Implementation Log for 2026-10-08-improved-performance (7 phases)

## Phase 1

Baseline and no-build spikes. No code changed.

### Setup

- **Revision:** `03c5d835c` (branch `feat/symbols`). `target/profiling/hug`
  was already fresh for this tree (`cargo build --profile profiling -p
  tree-hugger-cli` finished in 0.93 s with nothing to rebuild).
- **Host:** macOS dev host, 16 cores (12 performance + 4 efficiency).
  The 1-minute load average stayed between **6.4 and 14.5** for every run
  below, so it was under the core count (rule 2) throughout. That is much
  quieter than the spec's 40–56, and it is the main reason the absolute
  times here are lower than the spec's.
- **Cache isolation:** one scratch `TMPDIR` per experiment under
  `/tmp/th-p1/` (`mono`, `p99`, `o4c`, `tsc`, `dm/cold`, `dm/warm`).
  "Cold" means an empty `TMPDIR`.
- **Timing:** single runs use `/usr/bin/time -l` through a small helper
  (`/tmp/th-p1/bin/m.sh`). Repeated runs use `hyperfine -N`.
- All commands run from the repository root
  (`/Volumes/coding/wt/rusty-biscuit/feat-symbols`) unless stated.

### Results table

`H` = `target/profiling/hug`. Threads are rayon's default (16) unless stated.
Every row is on revision `03c5d835c`, arm `base`, dated 2026-10-08.

| Finding | Command | Threads | Cache | Load (1m) | Wall | User | Counts / notes |
|---|---|---|---|---|---|---|---|
| baseline | `H symbols --count` (monorepo) | 16 | cold | 11.2 | 6.93 s | 38.10 s | 149,370 records; 4,311 files; 259 MB RSS; 5.5× CPU/wall |
| baseline | `H symbols --count` | 16 | warm | 14.5 | 0.50 s / 0.52 s | 1.75 s | 4,311 hits, 0 parsed; 150 MB RSS |
| baseline / L1 | `H symbols --plain` | 16 | warm | 14.4 | 2.41 s / 2.49 s | 3.50–3.60 s | 18.1 MB stdout; ~1.9 s over `--count` |
| baseline / C5 | `H --json symbols`, first after `--count` cache | 16 | warm (narrow entries) | 14.4 | 9.42 s | 44.10 s | 257 MB stdout; re-parses (wider than cold `--count`) |
| baseline | `H --json symbols`, second | 16 | warm | 13.3 | 1.70 s | 2.32 s | |
| baseline / C4 | cache directory after `--count` + `--json` | – | – | – | – | – | 758 MB |
| C1 | `H --json-full symbols --no-cache schematic/definitions/src/openai/types.rs` (901 KB) | 16 | none | 14.5 | 0.81 s ×2 | 0.73 s | 24.6 MB stdout |
| C1 | same, first run without `--no-cache` | 16 | miss | 13.7 | 0.84 s | 0.75 s | |
| C1 | same, warm | 16 | hit | 13.7 | 1.25 s / 1.27 s | 1.18 s | **warm is 54% slower than `--no-cache`** |
| C1 | `H symbols --count` on that file over the wide entry | 16 | hit | 13.7 | 1.21 s / 1.25 s | 1.15 s | 7,732 records |
| C1 | `H --json-full symbols --no-cache biscuit-terminal/lib/src/components/filesystem/mod.rs` (298 KB) | 16 | none | 13.7 | 1.29 s ×2 | 1.21 s | 9.5 MB stdout |
| C1 | same, first run without `--no-cache` | 16 | miss | 13.7 | 1.29 s | 1.21 s | |
| C1 | same, warm | 16 | hit | 14.5 | 0.39 s / 0.38 s | 0.32 s | the cache helps here |
| C1 | `H symbols --count` on that file over the wide entry | 16 | hit | 14.5 | 0.36 s / 0.35 s | 0.30 s | 427 records |
| L3 | `H symbols --count tree-hugger/lib/src/lib.rs`, from repo root | 16 | hit | 13.5 | 70.6 ± 4.2 ms | 18.3 ms | sys 51.9 ms; 16 records; hyperfine `-w 2 -r 15` |
| L3 | `H --version` | – | – | 13.5 | 6.1 ± 0.4 ms | 3.0 ms | floor |
| L3 | `H symbols --count lib.rs` (a copy), from `/tmp/th-p1/outside` | 16 | hit | ~13 | 7.5 ± 0.3 ms | 3.9 ms | **9.4× faster than in-repo** |
| O4 | `H --json-full symbols --no-cache scripts/ci/test_affected_scope.py` (377 KB, 8,507 lines) | 16 | none | 10.2 | 0.76 s | 0.59 s | see O4 below; 373 MB RSS |
| O4 | same, miss then warm (fresh `TMPDIR`) | 16 | miss / hit | 10.0 | 0.99 s / 0.75 s | 0.75 / 0.56 s | 585–589 MB RSS; one 223 MB cache entry |
| O4 | `H --json-full symbols --no-cache big_class.py` (generated, 29 KB) | 16 | none | 10.2 | 0.14 s | 0.12 s | 47.9 MB stdout |
| L5 | `H lint --plain 'src/**'`, 2,000 generated `.ts` files, full `PATH` (52 entries) | 16 | warm | ~8 | 146.1 ± 1.8 ms | 113 ms | oxlint absent; 0 diagnostics |
| L5 | same, `PATH=/usr/bin:/bin` | 16 | warm | ~8 | 82.6 ± 2.5 ms | 94 ms | |
| L5 | `H symbols --count 'src/**'` on the same corpus | 16 | warm | ~8 | 67.4 ± 6.2 ms | 79 ms | no adapter |
| L6 | `script -q /dev/null H symbols tree-hugger/lib/src/lib.rs` (pseudo-TTY) | 16 | hit | ~10 | **2.117 ± 0.011 s** | 22 ms | two unanswered OSC queries |
| L6 | `script -q /dev/null H symbols --count …` (pseudo-TTY) | 16 | hit | ~10 | 68.7 ± 2.0 ms | 18 ms | |
| L6 | `H symbols --plain …` piped | 16 | hit | ~10 | 68.3 ± 4.8 ms | 18 ms | |
| L6 | `H symbols --count …` piped | 16 | hit | ~10 | 66.0 ± 2.3 ms | 17 ms | |
| L2 | `darkmatter/`: `H symbols --count` | 1 / 4 / 8 / 16 | cold | 6.4 → 8.3 | 7.446 / 2.767 / 1.777 / 1.230 s | 7.17 / 7.33 / 7.51 / 8.05 s | 744 files, 25,622 records; speed-up 1 / 2.69 / 4.19 / **6.05×** |
| L2 | `darkmatter/`: `H symbols --count` | 1 / 4 / 8 / 16 | warm | 8.3 → 10.2 | 337 / 146 / 113 / 94 ms | 0.27 / 0.29 / 0.32 / 0.36 s | 744 hits; speed-up 1 / 2.32 / 2.98 / 3.57× |

### Baseline: comparison to the spec

The relative findings all reproduce on a quieter host. The absolute numbers
are 2–4× lower than the spec's, because the spec's host was at load 40–56.

- **C1 is confirmed:** on `openai/types.rs` a warm hit (1.25 s) is still
  slower than `--no-cache` (0.81 s), and a narrow `--count` over the wide
  entry costs the same 1.2 s.
- **C5 is confirmed:** the first `--json` after a `--count` cache costs
  9.4 s and 44 s of CPU (more than the 6.9 s cold `--count`), against 1.7 s
  for the second run.
- **L1:** warm `--plain` costs ~1.9 s more than warm `--count` (spec: ~4 s
  under load).
- **L3 is confirmed:** in-repo single-file is 70.6 ms, against 7.5 ms from
  outside the repo and 6.1 ms for `--version`. Most of the gap is system
  time (52 ms), which fits a tree walk.
- **C4:** the cache was 758 MB after `--count` + `--json` (spec: 786 MB).

**Note for later phases:** a positional absolute path to a file outside the
current directory fails with `Error: NoSourceFiles { path: "<cwd>" }`;
positional filters resolve against the current directory. The "outside the
repo" L3 case therefore uses a copy of `lib.rs` in the scratch directory.
That matters for L3's fast path in Phase 6: it must keep this "outside the
cwd" behavior unless the author rules otherwise (Open Question on L3).

### O4: candidate sizing

Counted with `/tmp/th-p1/bin/o4.py` (run with `python3 -I`), which reads
`indexes[].occurrences[].candidates` from the `--json-full` output.

| Input | Source size | Occurrences | With candidates | Σ `candidates.len()` | Max per occurrence | `self` occurrences | Σ candidates on `self` | `--json-full` bytes | Candidate JSON bytes |
|---|---|---|---|---|---|---|---|---|---|
| generated `class Big` with 300 methods using `self` (3 uses each + `__init__`) | 28,938 B | 2,102 | 1,501 | 361,501 | 301 | 901 | 271,201 | 47,928,516 | ~45,239,498 (94%) |
| `scripts/ci/test_affected_scope.py` | 377,290 B | 9,813 | 6,210 | 1,172,770 | 507 | 2,054 | 1,041,378 (89%) | 221,854,736 | ~210,721,861 (95%) |

- Every `self` use carries one candidate per method in the file's
  `self` parameters (301 in the generated class, up to 507 in the real file).
  `self` alone produces 89% of all candidates in the real file.
- The output is **588×** the source for the real file and **1,656×** for
  the generated one; ~95% of it is candidate lists.
- The persistent cache stores the same data: the real file's single cache
  entry is **223 MB** (591× its source), and peak RSS is ~590 MB.
- Time is not the problem at this size (0.76 s `--no-cache`, 0.14 s for the
  generated file). The cost is output size, cache size and memory, and it is
  quadratic in methods per class. This strongly supports doing O4's contract
  part, which is the Phase 3 ruling ("How should uncertain reference
  candidates be represented?").

### L5: oxlint cost

`oxlint` is **not installed** on this host (`which oxlint` finds nothing).
Under rule 3, this records the per-file discovery cost instead of spawn
counts. Installing oxlint is the author's call.

- With oxlint absent, each JS/TS file still builds an `OxlintAdapter`, does
  one `node_modules/.bin/oxlint` stat and one stat per `PATH` entry
  (`which_oxlint`), and pushes one `AdapterMetadata`. All of it runs inside
  the serial delivery closure.
- **Measured:** over 2,000 generated `.ts` files, warm, the full 52-entry
  `PATH` costs 146.1 ms and a 2-entry `PATH` costs 82.6 ms. That is
  **~63 ms per 2,000 files (~32 µs per file)**, all serial, which scales with
  the length of `PATH`. Warm `symbols --count` on the same corpus is 67 ms,
  so discovery is about half of warm lint wall time on a TS-only tree.
- The plan asked for a `sample` profile. At 146 ms per run, `sample`'s
  1 ms interval gives too few samples to attribute. The same-binary
  `PATH` A/B isolates the cost more directly, so it replaces the profile.
- **Not measured, because oxlint is absent:** the two spawns per file
  (`oxlint --version` plus the run) that the spec estimates at 20–100 s per
  1,000 files. Phase 3's `oxlint-batch` arm needs oxlint installed to
  measure them.

### L6: TTY sample

- The terminal in use is **WezTerm** (`TERM_PROGRAM=WezTerm`), not iTerm2,
  so the iTerm2 `defaults read` spawns do not apply on this host.
- This session has no TTY (`tty` prints "not a tty"), so the real-terminal
  measurement the plan asks for **could not be taken by the agent**. It is
  deferred to the author: in WezTerm, warm cache, run
  `hyperfine -N -w 2 -r 15 'hug symbols tree-hugger/lib/src/lib.rs' 'hug symbols --count tree-hugger/lib/src/lib.rs'`.
- **Pseudo-TTY (timeout behavior only):** under `script -q /dev/null`,
  styled `hug symbols <file>` takes **2.117 s** against 68.7 ms for
  `--count`. The captured output begins with `ESC]11;?BEL ESC]10;?BEL`: the
  two OSC color queries each wait out their full ~1 s timeout because the
  pty never answers. That is the worst case for a terminal that does not
  answer OSC 10/11, as the spec describes. A real WezTerm answers, so
  the real figure should be much lower. That is what the deferred sample
  would show.
- **Piped (not a TTY):** `--plain` is 68.3 ms against 66.0 ms for
  `--count`, so detection plus rendering for one file costs ~2 ms when stdout
  is not a terminal. That matches the spec: L6's cost is TTY-only.

### L2: symptom check

- **Load condition met:** the 1-minute load average was 6.4 at the start of
  the cold sweep and 10.2 at the end of the warm sweep, under 16 cores.
- **Cold `darkmatter/`:** 7.446 → 1.230 s from 1 to 16 threads, a **6.05×**
  speed-up, under the plan's 8× bar. Scaling is already poor at 4 threads
  (2.69×, against an ideal near 4×). User CPU grows only 12% (7.17 → 8.05 s),
  so the threads are waiting, not doing extra work. That fits a per-chunk
  barrier (L2) better than counter contention (P11), which would show up
  as extra CPU. Separating the two is Phase 3's job (`window` and
  `p11-nocount` arms).
- **Warm:** 3.57× at 16 threads. Warm total time is ~0.1–0.3 s, so start-up
  and file listing dominate it.
- **Monorepo cold** at 16 threads used 38.1 s of CPU in 6.93 s of wall time
  (5.5×).
- Allowing for the 4 efficiency cores, the best realistic speed-up is below
  16×, but 6.05× is still well below it.

**Verdict: the L2 symptom reproduces without heavy contention.** L2 and P11
stay in scope for Phase 3's spike arms and Phase 5.

### Gate decision

Phase 1 changed only the feature directory's Markdown files (this log, the
plan's checkboxes and frontmatter, and the spec's frontmatter). The scope
calculator's result is recorded below.

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `"packages": []`, `"change_class": "documentation"` (exit 0). The
`just test` and `just lint` gates were therefore **skipped**, as the phase
prompt allows. No test was added or changed, because Phase 1 changes no
behavior; its deliverable is the measurements above.

### Deferred or not done

- **L6 real-terminal sample:** needs the author's interactive WezTerm (see
  L6 above).
- **L5 spawn counts:** need oxlint installed (plan rule 3; the author's
  call).
- **Plan frontmatter `packages`:** left as the plan-wide list
  (`tree-hugger`, `tree-hugger-cli`) rather than overwritten with Phase 1's
  empty list, so the plan keeps its scope. This log's `packages` is `[]`.
- Scratch artifacts (fixtures, JSON outputs, the helper scripts) are under
  `/tmp/th-p1/` and are not part of the repository.
