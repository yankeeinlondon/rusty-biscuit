---
spec: /Volumes/coding/wt/rusty-biscuit/feat-symbols/tree-hugger/features/2026-10-08-improved-performance/spec.md
plan: tree-hugger/features/2026-10-08-improved-performance/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
  - tree-hugger
  - tree-hugger-cli
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_1:
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/dead_code.rs
  - tree-hugger/lib/src/rule_registry.rs
  - tree-hugger/lib/src/ignore_directives.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/tests/symbols/report.rs
docs_updated_during_phase_2:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_3: []
docs_updated_during_phase_3:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - tree-hugger/lib/src/analysis/declaration_names.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/analysis/scope/ecma.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/analysis/scope/rust.rs
  - tree-hugger/lib/src/analysis/syntax_markers.rs
  - tree-hugger/lib/src/dead_code.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/doc_comments.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/generics.rs
  - tree-hugger/lib/src/file/extract/header/keyword.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/navigation.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/file/extract/relations.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/extract/scope.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/static_flags/cpp.rs
  - tree-hugger/lib/src/file/extract/static_flags/php.rs
  - tree-hugger/lib/src/file/extract/static_flags/python.rs
  - tree-hugger/lib/src/file/extract/static_flags/rust.rs
  - tree-hugger/lib/src/file/extract/static_flags/scala.rs
  - tree-hugger/lib/src/file/extract/visibility/c.rs
  - tree-hugger/lib/src/file/extract/visibility/cpp.rs
  - tree-hugger/lib/src/file/extract/visibility/csharp.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/file/extract/visibility/java.rs
  - tree-hugger/lib/src/file/extract/visibility/lua.rs
  - tree-hugger/lib/src/file/extract/visibility/rust.rs
  - tree-hugger/lib/src/file/extract/visibility/swift.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/parents.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/src/file/tree_file/relations/lexical.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
docs_updated_during_phase_4:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_5:
  - tree-hugger/cli/src/scan.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/src/file/source_file.rs
docs_updated_during_phase_5:
  - tree-hugger/docs/references.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_6:
  - tree-hugger/lib/src/scanner.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/tests/l1/file_identity.rs
  - tree-hugger/cli/src/changed.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/scan.rs
  - tree-hugger/cli/tests/symbols/filters.rs
  - tree-hugger/cli/tests/symbols/spelling.rs
  - tree-hugger/cli/tests/changed/ownership.rs
docs_updated_during_phase_6:
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/docs/cache-design.md
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
source_files_during_phase_7:
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/tests/l1/symbols_occurrences.rs
docs_updated_during_phase_7:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/tree-hugger/SKILL.md
source_code:
  - tree-hugger/lib/src/cache/stored_evidence.rs
  - tree-hugger/lib/src/cache/symbol_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/tree_file/definitions.rs
  - tree-hugger/lib/src/analysis/mod.rs
  - tree-hugger/lib/src/analysis/occurrences.rs
  - tree-hugger/lib/src/file/extract/declaration.rs
  - tree-hugger/lib/src/file/extract/qualified_name.rs
  - tree-hugger/lib/src/dead_code.rs
  - tree-hugger/lib/src/rule_registry.rs
  - tree-hugger/lib/src/ignore_directives.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/tests/symbols/report.rs
  - tree-hugger/lib/src/analysis/declaration_names.rs
  - tree-hugger/lib/src/analysis/scope/ecma.rs
  - tree-hugger/lib/src/analysis/scope/mod.rs
  - tree-hugger/lib/src/analysis/scope/rust.rs
  - tree-hugger/lib/src/analysis/syntax_markers.rs
  - tree-hugger/lib/src/file/extract/attributes.rs
  - tree-hugger/lib/src/file/extract/classify.rs
  - tree-hugger/lib/src/file/extract/doc_comments.rs
  - tree-hugger/lib/src/file/extract/flags.rs
  - tree-hugger/lib/src/file/extract/generics.rs
  - tree-hugger/lib/src/file/extract/header/keyword.rs
  - tree-hugger/lib/src/file/extract/modifiers.rs
  - tree-hugger/lib/src/file/extract/navigation.rs
  - tree-hugger/lib/src/file/extract/relations.rs
  - tree-hugger/lib/src/file/extract/rust_impl.rs
  - tree-hugger/lib/src/file/extract/scope.rs
  - tree-hugger/lib/src/file/extract/signatures.rs
  - tree-hugger/lib/src/file/extract/static_flags/cpp.rs
  - tree-hugger/lib/src/file/extract/static_flags/php.rs
  - tree-hugger/lib/src/file/extract/static_flags/python.rs
  - tree-hugger/lib/src/file/extract/static_flags/rust.rs
  - tree-hugger/lib/src/file/extract/static_flags/scala.rs
  - tree-hugger/lib/src/file/extract/visibility/c.rs
  - tree-hugger/lib/src/file/extract/visibility/cpp.rs
  - tree-hugger/lib/src/file/extract/visibility/csharp.rs
  - tree-hugger/lib/src/file/extract/visibility/ecma.rs
  - tree-hugger/lib/src/file/extract/visibility/java.rs
  - tree-hugger/lib/src/file/extract/visibility/lua.rs
  - tree-hugger/lib/src/file/extract/visibility/rust.rs
  - tree-hugger/lib/src/file/extract/visibility/swift.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/file/parents.rs
  - tree-hugger/lib/src/file/tree_file/relations.rs
  - tree-hugger/lib/src/file/tree_file/relations/lexical.rs
  - tree-hugger/lib/src/shared/schema_v2/mod.rs
  - tree-hugger/cli/src/scan.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/src/refs.rs
  - tree-hugger/cli/src/stats.rs
  - tree-hugger/cli/src/report.rs
  - tree-hugger/cli/src/prelude.rs
  - tree-hugger/cli/tests/symbols/refs.rs
  - tree-hugger/lib/src/god_files/analysis.rs
  - tree-hugger/lib/src/file/source_file.rs
  - tree-hugger/lib/src/scanner.rs
  - tree-hugger/lib/tests/l1/file_identity.rs
  - tree-hugger/cli/src/changed.rs
  - tree-hugger/cli/tests/symbols/filters.rs
  - tree-hugger/cli/tests/symbols/spelling.rs
  - tree-hugger/cli/tests/changed/ownership.rs
  - tree-hugger/lib/tests/l1/symbols_occurrences.rs
documentation:
  - tree-hugger/features/2026-10-08-improved-performance/plan.md
  - tree-hugger/features/2026-10-08-improved-performance/spec.md
  - tree-hugger/features/2026-10-08-improved-performance/implementation-log.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/cli/README.md
  - .claude/skills/tree-hugger/SKILL.md
  - tree-hugger/docs/references.md
  - tree-hugger/docs/cache-design.md
completed_phase: 7
implemented: true
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

## Phase 2

Byte-identical fixes with evidence in hand. The four Wave 1 groups were done
in sequence by one agent (they share one `target/` directory, so parallel
builds would only queue). The base binary for the output diff protocol is
`target/profiling/hug` at `350cec8a2` (no tree-hugger source change since
`03c5d835c`), saved as `/tmp/th-p2/hug-base` before any edit.

### 2A: cache validation (`stored_evidence.rs`, `symbol_cache.rs`)

- **C1.** `unbound_names_have_a_binding` now tests `checked.contains(text)`
  first and asks a `Covering` cursor over the start-sorted, deduplicated
  `taken` spans. **Departure from the plan's wording:** the plan said
  "merged" spans; merging overlapping spans would accept a reading that
  crosses two spans and is inside neither, which the old
  `taken.iter().any(..)` refused. The cursor instead keeps the furthest end
  among spans starting at or before the reading, which is exactly the old
  predicate (readings arrive with non-decreasing starts).
  `identifier_readings` returns an iterator. The encoded-token pass runs only
  when `memchr::memmem` finds `r#` (Rust) or `\u` (TS/JS); no other language
  can produce an encoded witness, so the skip is exact. Its exact-span test
  is now a `binary_search` on the sorted spans.
- **C2.** `scoped` is keyed by `(owner, target, name_identity::key, kind)`,
  so `made` is two set lookups (`equal` is defined as key equality). The
  per-record `dependencies.contains` / `references.contains` use a `RefList`
  map keyed by `(id, name, qualified_name)` and confirm with full
  `SymbolRef` equality (the source span is not hashable). The per-owner
  scoped list became `OwnScoped`: the first position per target ID, and the
  ascending positions of no-ID occurrences by name key, so
  `summaries_in_first_occurrence_order` uses a map lookup and a
  `partition_point` instead of linear `find`s. The greedy earliest-first
  choice is unchanged.
- **C7.** `StoredEntry::validate` builds one `LineStarts` and passes it to
  `validate_imports` and (inside `StoredEvidence`) `validate_evidence`.
  `import_inventory` and `write_imports` serialize a borrowed
  `StoredImportView` with the stored form's field names and order;
  `StoredImport` is now deserialize-only. Hashing stays on
  `biscuit_hash::blake3_hash`.
- **Tests added:**
  - `cache::stored_evidence::tests::import_inventory_digest_is_pinned`:
    digests of a two-import list (every optional field both set and unset)
    and of the empty list, captured from the **old** code before the change.
  - `cache::stored_evidence::tests::covering_agrees_with_a_scan_of_every_span`:
    the cursor against the old linear predicate for every range over
    overlapping, touching, nested and empty spans.
  - `symbols_cache::encoded_binding_witnesses_keep_unbound_uses_served`
    (lib L1): a TS `\u0078` import, a JS `\u{79}` import and a Rust
    `r#match_` import each witness an unbound `Scoped` use, and the warm
    read is a `Hit` served equal to the miss. Mutation check: changing the
    TS/JS probe to a string that never matches makes this test fail.
    The refusal side was already pinned by
    `encoded_binding_sources_preserve_lost_id_rejection`.
- **Targeted run:** `just test cache_robustness symbols_cache
  stored_evidence cache_upgrade symbols_cache_imports cache_lexical`:
  400 passed.

### 2B: query pipeline and native diagnostics

- **P4.** `capture_definitions` keeps its `local.definition.import`
  captures in match order and returns them in a new `Definitions` struct
  (`symbols`, `import_nodes`, `scopes`) from both `definitions()` and
  `definitions_with_scopes()`; the tuple is gone. `imported_symbols()` now
  collects its own captures and calls `imports_from_nodes`, which holds the
  old body (sort and dedup by `(start_byte, end_byte)` unchanged). The
  imports query is the locals query's text (`resolve_query_text` falls
  through to `resolve_locals_query` for `Imports`), so both runs see the
  same nodes in the same match order, and the stable sort keeps the same
  survivor among equal spans. `run_query` reads `import_nodes`; so do the
  public `referenced_symbols`, `lint_diagnostics`, `try_lint_diagnostics`
  and `try_diagnostics`, which also ran the query twice. `QueryKind::Imports`,
  `RECORD_QUERIES` and the counters are unchanged. The `occurrences.rs`
  test helper keeps calling `imported_symbols()` as an independent oracle.
- **P9.** `ExtractedSymbol::is_exported` is computed at the end of
  `capture_definitions`, on the finished symbol (`is_exported_definition`
  reads only `kind`, `language` and the anchor; nothing mutates a symbol's
  kind after extraction). `exports_from`, `exported_symbols`,
  `local_symbols` and `records_from` read the field.
- **L8 (library).** `RuleRegistry::builtin() -> &'static RuleRegistry`
  (a `LazyLock`); `diagnostic_metadata` uses it. `new()` and
  `get_or_default` are unchanged. The registry has no public mutators, so
  sharing it is safe.
- **L9.** `build_source_context_from_range` reads the line through a lazy
  `OnceLock<Vec<usize>>` line index on `TreeFile` (`line_starts` +
  `nth_line`). **Correction to the plan's wording:** `str::lines()` drops
  a `\r` only from a line that ended in `\n`; an unterminated last line
  keeps its `\r` (`"a\r".lines()` is `["a\r"]`). `nth_line` copies that
  rule exactly rather than always dropping one `\r`. Both
  `named_child(index)` loops (`dead_code_ranges`, `find_dead_code_after`)
  iterate `named_children(&mut cursor)`; tree-sitter's `named_children`
  yields the same nodes as `named_child(0..named_child_count())`.
- **L10.** `apply_ignore_directives` returns early when there are no
  diagnostics or `memmem` finds no `tree-hugger-ignore` in the source.
  `IGNORE_LINE_PREFIX` is now `pub(crate)` and documents that every
  directive starts with it. No work counter is attached to the parse.
- **Tests added:**
  - `file::tree_file::definitions::tests::imports_from_the_definition_run_equal_imported_symbols`:
    every parseable file under `lib/tests/fixtures/` (asserting more than 50
    files and 50 imports), plus inline grouped and repeated captures (Rust
    `use a::{self, *}`, Scala `{_, given}`, a Go alias, Swift selective
    imports, TS default/named/namespace/re-export, Python grouped `from`):
    the imports from the definition run equal `imported_symbols()`.
  - `file::tree_file::tests::nth_line_agrees_with_lines`: LF, CRLF, a bare
    `\r`, a trailing unterminated `\r`, `\r\r\n`, no final newline, the
    empty file, multibyte text, and every index up to past the end.
  - `rule_registry::tests::builtin_is_one_shared_new_registry`: one
    instance; the same rule IDs and `get_or_default` (an alias and an
    unknown rule included) as `new()`.
  - L10 and the `named_children` change are pinned by the existing
    `lint_diagnostics` directive tests (line, file, bare and in-string) and
    the `dead_code` suite.
- **Targeted run:** `just test tree_file lint_diagnostics symbols_query
  symbols_cache symbols_coverage ignore_directives dead_code` plus the new
  tests: 385 passed.

### 2C: occurrences and declaration climbs

- **O5.** `resolve` computes `written_path` once and passes it to
  `resolve_supported` as `path: Option<&str>`; the occurrence's
  `qualified_name` takes the same value.
- **P1(a).**
  - `is_scope_boundary` tests the kind first (`is_scope_boundary_kind`)
    and `parent().is_none()` last.
  - `resolve_declaration_node` asks each step for its parent once (and
    only when the kind is not a boundary), and reuses the name's parent at
    the end. The order of the boundary and declaration tests is unchanged.
  - `Builder::prefix` stops at the root by ID. `qualified_names` finds the
    root with one climb from the first symbol's anchor rather than taking a
    new `root` parameter, so its three callers (one in `definitions.rs`,
    two in `classify.rs`) are untouched.
  - The `fill_metadata` scope reuse stays deferred to Phase 4, as the plan
    says.
- **Tests:** no new test; these are pure refactors of pure functions. They
  are pinned by `just test symbols_occurrences symbols_qualified_names
  symbols_scope symbols_relations symbols_owner_identity` (151 passed) and
  by the output diff protocol (`--json-full`, `refs`) in Wave 2.

### 2D: CLI lint and output correctness (`cli/src/main.rs`)

- **L8 (CLI).** The four `RuleRegistry::new()` sites (`summarize_file`,
  `add_external_lint_diagnostics`, `render_summary`, `rule_list_items`) use
  `RuleRegistry::builtin()`.
- **Second policy pass removed.** First proved idempotent by
  `tests::lint_policy_applied_twice_equals_once` (bin unit test, written and
  passing **before** the pass was removed): every built-in rule, the
  `unreachable-code` alias, an unknown native rule and an unknown
  `oxlint/…` rule, each with no metadata, native metadata and external-tool
  metadata, under `--deny`/`--warn`/`--allow` shapes (none, `all`, every
  category, every rule ID, and mixed pairs), × `--strict` × 
  `--experimental-semantics`; more than 10,000 policy results compared.
  `render_summary` now renders `summary.lint` as summarized (the JSON
  output already used exactly that list).
- **L11 (correctness half).** `imports`/`lint` text and JSON go through one
  `BufWriter<StdoutLock>`; `print_file_header`, `render_summary`,
  `render_diagnostics_filtered`, `render_unified_diagnostic`,
  `render_source_context` and `render_imports` take `&mut dyn Write` and
  return `io::Result`. The writer is flushed (and dropped) before the
  `--verbose` stats line, so stdout/stderr order is unchanged; adapter
  warnings still go to stderr before any report bytes.
  `declarations::write_status` is now `pub(crate)` and decides the exit:
  a closed pipe gives 0, any other write failure prints
  `error: cannot write output: …` and gives 1, otherwise lint errors or
  failed files give 1 as before. Note the consequence, inherited from the
  declaration commands: a lint run with error findings whose reader closes
  early now exits 0 (it used to panic with 101).
- **Tests added:** `report::imports_and_lint_end_quietly_on_a_closed_pipe`
  (CLI L1): `imports` (default, `--plain`, `--json`) and `lint` (default,
  `--plain --deny all`, `--json`) into a pipe whose reader closed before
  `hug` started: no panic, no write-error message, exit 0. The
  `write_status` mapping itself is already pinned by
  `declarations::tests` (broken pipe, "disk full", success).
- **Reproduction:** with the reader gone before the first write,
  `hug-base imports 'tree-hugger/lib/src/**'` and `hug-base --json lint …`
  exit 101 with a `panicked at …/stdio.rs` message; the new binary exits 0
  with nothing on stderr.

### Wave 2: docs, verification and timing

#### Docs

- `docs/symbol-reports.md` ("Failed files and exit status"): `imports` and
  `lint` now treat stdout like the declaration commands: a closed pipe
  exits `0`, another write failure prints `error: cannot write output` and
  exits `1`, with an example.
- `cli/README.md` (Lint Command): `lint`'s exit status and the quiet
  closed-pipe exit for `lint` and `imports`.
- `.claude/skills/tree-hugger/SKILL.md` (Rule Registry): one line on
  `RuleRegistry::builtin()`, and one on the CLI applying lint policy once.
- `docs/cache-design.md`: unchanged. It has no notes on the validation
  algorithms C1/C2/C7 replaced, and its robustness tables still hold.

#### Gates

- `just test` (tree-hugger area), first run: **2 timeouts**,
  `cache_robustness::every_kind_evidence_identities_name_retained_declarations_in_every_language`
  and `…retained_geometry_written_paths_and_candidate_order_are_checked_in_every_language`
  (30 s nextest limit), at 1710/2034 when fail-fast stopped the run. That
  run overlapped the output diff protocol on a host at load ~31. Alone,
  the two took 5.8 s and 6.6 s. **Second run, nothing else running: 2034
  passed, 0 failed** (163 slow), exit 0.
- `just lint`, first run: one clippy `type_complexity` error on `RefList`'s
  map type. Fixed with a `RefIdentity` type alias. Second run: exit 0.
  `just test stored_evidence symbols_cache` re-run after the alias: 133
  passed.
- Scope calculator (run from the repository root after all edits):

  ```sh
  git status --porcelain --untracked-files=all | awk '{print $NF}' \
    | xargs python3 scripts/ci/affected_scope.py
  ```

  `"packages": ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`,
  `"change_class": "package"`, so both gates were required and were run.
  `test-toolkit` was not edited. The calculator includes it as a package
  that reads these files; its gates were not run here (they belong to its
  own area, and CI will run them).

#### Output diff protocol

Base: `hug-base` (see the top of this phase). New: `target/profiling/hug`
after every Phase 2 code edit. Both ran from the repository root
(`/Volumes/coding/wt/rusty-biscuit/feat-symbols`), one fixed scratch
`TMPDIR` per arm, `NO_COLOR=1`. Scripts: `/tmp/th-p2/bin/protocol.sh`
and `/tmp/th-p2/bin/protocol2.sh` (scratch, not in the repo).

- **Corpora:** `tree-hugger/**`; the 16 files of
  `lib/tests/fixtures/symbols/multi/**` (passed with `--file`); the two p99
  files.
- **Commands:** `--json-full symbols`, `--json symbols`,
  `symbols --plain`, `classes --plain`, `imports`, `--json lint`, and `refs`
  (Rust `resolve_declaration_node` over `tree-hugger/`).
- **States:** cold (`--no-cache`), warm twice, then once more after a wide
  `--json-full symbols` run.
- **Result: 336 output/stderr/exit-code files compared; 331 identical, and
  all 5 differences are benign:**
  - `th.lint.cold.out`: `adapter_metadata[].elapsed_time_ms` (wall-clock
    time) differs. With that field masked, the 100 MB output is
    byte-identical.
  - `fix.lint.{cold,warm1,warm2,wide}.err`: `lint` takes no `--file`, so
    both binaries printed a usage error that names the binary (`hug-base`
    vs `hug`). This row tested nothing, so it was replaced (next point).
- **Supplementary rows** (`protocol2.sh`, run from inside the fixture
  directory because `lint` and `refs` take no `--file` and ordinary scans
  skip `fixtures/`; the base binary copied to `/tmp/th-p2/base/hug` so
  usage text matches): `--json lint`, `lint --plain`, `imports --plain`,
  `refs build_widget rust.rs`, `refs buildPanel typescript.ts`,
  `refs --json Panel typescript.ts`, `refs --plain --callers main
  nested/go.go`, in all four states. **84 files compared, 84 identical,**
  every exit code 0.
- **`hug imports 'tree-hugger/**' | head -1`** (under bash, reading
  `PIPESTATUS`): base exits **101** with `panicked at …/stdio.rs`; new
  exits **0** with an empty stderr.

#### Timing sample

Same host, both arms back to back (so they share the load), revision
`350cec8a2` + Phase 2's working tree, profiling build, rayon default (16
threads), 2026-10-08 20:20. The 1-minute load average was 17–21 during
these rows. A burst outside this session had pushed it to ~140 a few
minutes earlier. The Phase 1 rows were taken at 6–14, so compare
base with new here, not with Phase 1.

| Command | Cache | Base wall / user | New wall / user | Note |
|---|---|---|---|---|
| monorepo `symbols --count` | cold | 6.95 s / 38.26 s | **6.19 s / 33.27 s** | −13% CPU (P4, P9, P1(a)); 4,311 misses both |
| monorepo `symbols --count` | warm ×2 | 0.50 / 0.49 s | 0.64 / 0.55 s | noise at this load; 4,311 hits both |
| monorepo `symbols --plain` | warm | 2.32 s | 2.37 s | L1 is Phase 5 |
| monorepo `--json symbols`, first after `--count` | partial | 8.55 s / 43.81 s | **7.72 s / 39.50 s** | C5 is Phase 7 |
| monorepo `--json symbols`, second | warm | 1.04 s | 1.06 s | |
| `openai/types.rs --json-full` | `--no-cache` | 0.81 s | 0.74 s | |
| same | miss | 0.83 s | 0.75 s | |
| same | **warm hit** | **1.25 / 1.28 s** | **0.16 / 0.17 s** | **C1/C2: 7.8× faster, now well below `--no-cache`** |
| `openai/types.rs symbols --count` over the wide entry | hit | 1.26 s | **0.12 s** | 10.5× |
| `filesystem/mod.rs --json-full` | `--no-cache` | 1.27 s | 1.13 s | |
| same | warm hit | 0.36 / 0.38 s | **0.11 / 0.10 s** | 3.4× |
| `filesystem/mod.rs symbols --count` over the wide entry | hit | 0.35 s | 0.09 s | |
| `symbols --count tree-hugger/lib/src/lib.rs` (hyperfine `-w 2 -r 15`) | hit | 67.6 ± 3.5 ms | 69.2 ± 3.6 ms | unchanged; L3 is Phase 6 |

`--verbose` showed `1 hits, 0 partial hits, 0 misses; parsed 0 files` on
every warm p99 row in both arms, so the speed-up is validation, not a
re-parse.

**Checkpoint 2:** met. Output is byte-identical (only the adapter's
wall-clock field differs), the full test and lint gates are green, a warm
wide-entry hit on `openai/types.rs` is faster than `--no-cache`, the
cold-scan change is logged, and `imports | head` exits 0. Stop for review
before pushing.

#### Requirement-to-test mapping

| Change | Test(s) |
|---|---|
| C1 cursor | `stored_evidence::tests::covering_agrees_with_a_scan_of_every_span` (new); `cache_robustness`, `symbols_cache` (existing) |
| C1 encoded-token guard | `symbols_cache::encoded_binding_witnesses_keep_unbound_uses_served` (new, mutation-checked); `encoded_binding_sources_preserve_lost_id_rejection` (existing) |
| C1 iterator readings | `stored_evidence::tests::a_dollar_run_is_also_read_as_its_pieces` (adapted) |
| C2 | `cache_robustness::summary_and_evidence_list_edits_are_recomputed_in_every_resolving_language`, `reference_evidence_is_served_only_when_consistent`, `symbols_cache` (existing, every refusal still produced) |
| C7 digest | `stored_evidence::tests::import_inventory_digest_is_pinned` (new, digests from the old code) |
| C7 single `LineStarts` | `stored_evidence::tests::points_are_one_based_lines_and_byte_columns`, `symbols_cache_imports` (existing) |
| P4 | `definitions::tests::imports_from_the_definition_run_equal_imported_symbols` (new); output diff protocol |
| P9 | `symbols_query`, `symbols_coverage` (existing); output diff protocol (`is_exported` in every `--json` record) |
| L8 registry | `rule_registry::tests::builtin_is_one_shared_new_registry` (new) |
| L8 single policy pass | `tests::lint_policy_applied_twice_equals_once` (new, bin); output diff protocol (`lint` text) |
| L9 line index | `tree_file::tests::nth_line_agrees_with_lines` (new) |
| L9 `named_children` | `dead_code`, `lint_diagnostics` (existing) |
| L10 | `lint_diagnostics` directive tests (existing: line, file, bare, in-string) |
| O5, P1(a) | `symbols_occurrences`, `symbols_qualified_names`, `symbols_scope`, `symbols_relations`, `symbols_owner_identity` (existing); output diff protocol |
| L11 | `report::imports_and_lint_end_quietly_on_a_closed_pipe` (new, CLI); `declarations::tests` for `write_status` (existing) |

All new tests are compiled by declared targets: the lib unit tests and the
`hug` bin unit tests (`just test` runs both), `lib/tests/l1/symbols_cache.rs`
(the `l1` binary), and `cli/tests/symbols/report.rs` (the `symbols`
binary). None carries a tier marker, so all run in L1. The fixture walk
joins `tests/fixtures` onto `CARGO_MANIFEST_DIR`.

#### OS considerations

Nothing here is OS-specific: no path or `#[cfg]` code changed. The
closed-pipe test reuses the existing `hug_into_closed_pipe` helper, which
already runs on every OS in CI. Line indexing follows `str::lines()`, so
CRLF sources behave as before (pinned by `nth_line_agrees_with_lines`). No
cross-OS rig run was needed.

## Phase 3

One spike session: every arm behind a `HUG_SPIKE` switch in one build, then
the measurements, then a rulings summary for the author. **No repository
code changed.** The spike code never touched this working tree.

### Setup

- **Spike tree.** A detached worktree at `/tmp/th-p3/spike`: `HEAD`
  (`350cec8a2`) plus Phase 2's working tree, applied as a patch, plus the
  arms below. It has its own target directory (`/tmp/th-p3/target-spike`),
  so the main `target/` was not evicted. The spike is saved as
  `/tmp/th-p3/spike.patch` and the worktrees are removed (Checkpoint 3).
  Nothing was committed and no branch was created.
- **Binaries.** All are `profiling` builds of the same Phase 2 code.
  - `hug-spike`: the spike build (one build; a first attempt stopped on
    two compile errors, fixed in place).
  - `target/profiling/hug`: Phase 2's binary, the "no spike code" control.
  - `hug-b12`: fat LTO + `codegen-units = 1` + mimalloc (4 m 38 s).
  - `hug-b2`: mimalloc only, built to isolate B2 (see B1 + B2); 23 s,
    because the compiler cache already held its dependencies.
- **Host.** macOS, 16 cores (12P + 4E). Every timing row records the
  1-minute load average. Most rows ran at 7–14. Outside activity pushed some
  rows to 15–19, and those are marked; noisy rows were repeated interleaved.
- **Timing.** `hyperfine -N` through `/tmp/th-p3/bin/t.sh` (`-w 1 -r 3`
  unless stated). Profiles use `/usr/bin/sample` at 1 ms, through
  `/tmp/th-p3/bin/prof.sh`. They are parsed by `/tmp/th-p3/bin/callers.py`,
  which reports inclusive samples per caller and the topmost
  parent-navigation frames (`ts_node_parent`, `ts_node_child_with_descendant`,
  `ts_node_*_sibling`, the spike memo) beneath each. All runs are from the
  repository root unless stated, dated 2026-10-08, revision `350cec8a2` +
  Phase 2's working tree.

### The arms

| Switch | What it does | Output |
|---|---|---|
| `p1b` | A per-thread memo behind `spike::parent(node)`. A miss descends from the root once (mirroring `ts_node_parent`) and records the parent of every node on the path. Cleared before every parse and keyed by tree. **Departure from the plan:** every tree-sitter `.parent()` in the library (201 call sites in 37 files) goes through it, not only the seven named callers, so the arm measures (b)'s ceiling. | identical |
| `p1b-check` | Adds `assert_eq!(memo, node.parent())` on every lookup. A runtime assert, because `debug_assert` is compiled out of a `profiling` build. | — |
| `p2-prefilter` | Skips `rust_pattern_non_bindings` unless the file has a glob `use` (line scan of `use` statements), a `const` generic, or a bare pattern name equal to a `const`/`static`/type/import capture or `None`/`Self`. Counts skipped and run walks. | identical |
| `p3-share` | Stashes `impl_self_types`' `Lexical::build` result (it reads only the entries' node IDs and kinds) and hands it to `link_relations` when the entries' `(name, declaration, anchor, kind)` key matches. Counts builds, reuses and key mismatches. | identical |
| `p5-gate` | No `records_from` metadata for `Local` records. | knowingly different |
| `p11-nocount` | `count_shared_context_steps` returns at once. | identical |
| `window` | `in_file_order` as a sliding reorder window (W = 4 × threads, scoped worker threads, delivery on the calling thread). The spike version does not handle a panicking head job, which Phase 5 must. | identical |
| `flags-on-miss` | A cache-enabled miss analyzes the request plus Flags and stores that coverage. | identical |

### Correctness of the byte-identical arms

- **`p1b-check`:** no mismatch over:
  - the monorepo `--json symbols --no-cache` (20,425,057 checked lookups);
  - `--json-full` over `tree-hugger/**` (3,857,028) and `darkmatter/**`
    (16,355,288);
  - `--json lint` over both;
  - monorepo `refs`;
  - the three large files.

  Over the monorepo `--json` run, 92% of lookups were memo hits and 0.76 M
  were root descents.
- **Output equivalence** (`/tmp/th-p3/bin/equiv.sh`, `NO_COLOR=1`, every
  output hashed). Seven commands, each with `--no-cache`:
  - monorepo `--json symbols` and `symbols --plain`;
  - `--json-full symbols` over `tree-hugger/**`, `darkmatter/**`, and the two
    p99 files;
  - monorepo `refs resolve_declaration_node`;
  - `--json lint 'tree-hugger/**'`, with `elapsed_time_ms` masked.

  Eight configurations: Phase 2's binary, the spike with no switch, `p1b`,
  `p2-prefilter`, `p3-share`, `window`, `p11-nocount`, and all five
  together. **Each command gave exactly one hash across all eight.**

### Results table

| Finding | Command | Arm | Threads | Cache | Load | Wall | User | Counts / notes |
|---|---|---|---|---|---|---|---|---|
| P1 | `darkmatter/` `symbols --count` | none / `p1b` | 16 | `--no-cache` | 7.6 / 10.3 | 1.668 / **1.141 s** | 9.22 / 6.76 s | −32% wall |
| P1 | `tree-hugger/` `symbols --count` | none / `p1b` | 16 | `--no-cache` | 10.0 / 9.4 | 0.650 / **0.518 s** | 2.80 / 2.19 s | −20% |
| P1 | monorepo `symbols --count` | none / `p1b` | 16 | `--no-cache` | 9.4 / 8.9 | 5.892 / **4.244 s** | 33.45 / 24.94 s | −28% wall, −25% CPU (r = 2) |
| P1 / O2 | `--json-full symbols schematic/definitions/src/openai/mod.rs` (`/usr/bin/time`, one run each) | none / `p1b` | 16 | `--no-cache` | 9–17 | **535.75 / 570.15 s** | – / 553.00 s | the pathological file; one busy thread |
| P2 | monorepo `symbols --count --language rust` | none / `p2-prefilter` | 16 | `--no-cache` | 9.3 / 9.0 | 5.571 / 5.666 s | 32.48 / 31.86 s | skips 1,004 of 3,660 walks (27%); no wall gain |
| P3 | same | none / `p3-share` | 16 | `--no-cache` | 9.3 / 9.1 | 5.571 / **5.001 s** | 32.48 / 28.89 s | `Lexical::build` 5,713 → 4,113 (1,600 reused, 0 key mismatches); −10% wall, −11% CPU |
| P2 + P3 | same | `p2-prefilter,p3-share` | 16 | `--no-cache` | 13.2 | 4.915 s | 28.66 s | |
| P1–P3 | same | `p1b` / `p1b,p2-prefilter,p3-share` | 16 | `--no-cache` | 11.7 / 12.9 | 4.144 / **3.697 s** | 25.12 / 22.55 s | −34% against none |
| P5 | monorepo `symbols --count` | none / `p5-gate`, round 1 | 16 | `--no-cache` | 16.8 / 16.1 | 6.805 / 6.367 s | 38.09 / 35.33 s | loaded host; interleaved |
| P5 | same | `p1b` / `p1b,p5-gate`, round 1 | 16 | `--no-cache` | 11.6 / 13.4 | 4.683 / 4.453 s | 27.80 / 26.18 s | −6% CPU |
| P5 | same | none / `p5-gate`, round 2 | 16 | `--no-cache` | 14.4 / 14.1 | 6.555 / 6.418 s | 36.85 / 35.64 s | −3% CPU |
| P5 | same | `p1b` / `p1b,p5-gate`, round 2 | 16 | `--no-cache` | 16.8 / 16.8 | 4.909 / 4.680 s | 28.96 / 27.08 s | −6% CPU |
| L2 / P11 | `darkmatter/` `symbols --count` | none / `window` / `p11-nocount` / both | 1 | `--no-cache` | 6.9–10.0 | 8.258 / 8.208 / 8.301 / 8.249 s | 8.03–8.09 s | |
| L2 / P11 | same | same | 4 | `--no-cache` | 6.2–8.0 | 3.414 / **2.771** / 3.363 / 2.754 s | 8.28–8.34 s | |
| L2 / P11 | same | same | 8 | `--no-cache` | 8.0–8.6 | 2.137 / **1.698** / 2.159 / 1.682 s | 8.43–8.46 s | |
| L2 / P11 | same | same | 16 | `--no-cache` | 8.2–8.3 | 1.577 / **1.245** / 1.589 / 1.260 s | 9.14–9.17 s | speed-up 5.2× → 6.6× |
| L2 | `darkmatter/` `symbols --count` (`-w 2 -r 5`) | none / `window` / `window,p11-nocount` | 1 | warm | 8.1–8.2 | 0.529 / 0.509 / 0.514 s | 0.39–0.40 s | |
| L2 | same | same | 4 | warm | 8.3 | 0.273 / 0.229 / 0.224 s | 0.43 s | |
| L2 | same | same | 8 | warm | 8.1–8.3 | 0.214 / 0.176 / 0.170 s | 0.45–0.46 s | |
| L2 | same | same | 16 | warm | 8.1 | 0.192 / **0.159** / 0.157 s | 0.51–0.53 s | −17% |
| L2 | monorepo `symbols --count` | none / `window` | 16 | `--no-cache` | 9.1 / 8.2 | 5.922 / **4.579 s** | 33.47 / 33.56 s | −23% wall, same CPU |
| all | same | `p1b,p3-share,window` | 16 | `--no-cache` | 13.0 | **2.951 s** | 22.36 s | **−50% wall, −33% CPU** |
| all | same | `p1b,p2-prefilter,p3-share,window` | 16 | `--no-cache` | 17.3 | 2.919 s | 21.89 s | |
| C3 | `symbols --count` over `tree-hugger/**`, `darkmatter/**` and the two p99 files (1,160 files) (`-w 2 -r 10`) | Phase 2 binary | 16 | warm, narrow entries | 16.5 / 15.2 | 0.268 / 0.269 s | 0.66 s | 1,160 hits; cache 314 MB |
| C3 | same | same | 16 | warm, entries widened by `--json-full` | 16.1 / 14.2 | **0.557 / 0.565 s** | 2.18 s | 1,160 hits; cache 934 MB; **2.1× wall, 3.3× CPU** |
| C3 | `symbols --count openai/types.rs` | same | 16 | warm narrow / wide | 13.7 | 0.100 / 0.129 s | 0.04 / 0.07 s | |
| C5 | monorepo `symbols --count`, empty cache each run | none / `flags-on-miss`, round 1 | 16 | cold miss | 13.4 / 15.3 | 6.137 / **6.930 s** | 33.92 / 37.77 s | identical output; **+13% wall, +11% CPU** |
| C5 | same, round 2 | same | 16 | cold miss | 13.9 / 16.8 | 6.129 / 6.960 s | 33.83 / 37.77 s | |
| C5 | monorepo `--json symbols` right after a `symbols --count` | none / `flags-on-miss` | 16 | listing's entries | 18.7 / 12.9 | 7.811 / **1.076 s** | 40.17 / 2.29 s | 4,311 partial hits (re-parse) → 4,311 hits, 0 parsed; cache 759 MB in both |
| B1 + B2 | monorepo `symbols --count` | profiling / LTO + mimalloc, two rounds | 16 | `--no-cache` | 9.7–13.1 | 6.190, 5.884 / 5.892, 5.878 s | 34.52, 33.17 / 33.14, 33.39 s | no clear change |
| B1 + B2 | monorepo `symbols --count` (`-w 2 -r 10`) | same | 16 | warm | 10.2–14.5 | 0.528, 0.503 / 0.497, 0.481 s | 1.74, 1.73 / 1.54, 1.55 s | −4–6% wall, −11% CPU |
| B1 + B2 | monorepo `symbols --plain` (`-r 5`) | same | 16 | warm | 10.0–14.9 | 2.351, 2.335 / **1.965, 1.984 s** | 3.45, 3.43 / 2.94, 2.94 s | **−15%** |
| B1 vs B2 | monorepo `symbols --plain`, fastest run | profiling / mimalloc only / LTO + mimalloc | 16 | warm | 12–18 (noisy) | 2.620, 2.582 / 2.239, 2.156 / 2.469, 2.141 s | 3.94–4.40 / 3.32–3.66 / 3.67–4.80 s | the allocator gives the gain; LTO adds none measurable |

### P1: parent lookups

- The memo is exact (above) and cuts cold CPU by 25% over the monorepo,
  27% on `darkmatter/` and 22% on `tree-hugger/`, measured on the
  `p1b` arm. That is close to the spec's 25–35% estimate.
- **Re-sample (O-findings profile, below):** with `p1b`, parent navigation
  under `run_query` falls from 76–80% to 60–74% on Rust and TS. Inclusive
  `run_query` samples fall 53% (Rust `--json-full`), 35% (Rust `refs`), 24%
  (TS `--json-full`), 32% (TS `refs`) and 29–30% (Python).
- `fill_metadata`'s per-candidate scope still runs once per candidate (it
  goes through the memo too). It remains for Phase 4, as planned.
- **Decisive for Phase 4's (b) vs (c) choice: a pathological file.**
  `schematic/definitions/src/openai/mod.rs` (401 KB, generated; one
  function builds `Vec::from([...])` of 265 large `Endpoint` literals full of
  `vec![...]` token trees) takes **535.75 s** for `--json-full symbols
  --no-cache`. Every other file of the 12 largest takes 0.56–1.41 s. It is
  also why a monorepo `refs --no-cache` takes 7–8 minutes on one busy
  thread. Sampled:
  - **Without `p1b`:** the busy thread is 100% parent navigation inside
    `Regions::around` (99.8%), all in `ts_node_child_iterator_next` and
    `ts_node_child_with_descendant`.
  - **With `p1b`:** **570.15 s** (553.00 s user), no better. The memo
    still pays one root descent per node it has not seen, and in this tree
    each descent is the whole cost. Only the split shifts, to
    `Regions::around` 40% and `scope_around` 20% of the busy samples.

  A lazy memo (P1(b)) cannot fix this; a full parent index from one
  `TreeCursor` pass (P1(c)), which is linear in nodes, can. O2's stack sweep
  for `Regions::around`/`scope_around` addresses the same climbs.
  **Recommendation for Phase 4: (c), the full index.** Make this file the
  P1/O2 acceptance check: time it before and after, and add a work-count
  assertion rather than a timing assertion.

### P2: the Rust pattern walk

- 3,660 of the monorepo's 4,113 Rust files run the walk. The prefilter skips
  1,004 of them, **a 27% skip rate**, far below the plan's ~80% bar.
- No wall gain and −2% CPU, because the files that are skipped are the
  cheap ones. In the Rust `--json-full` profile,
  `rust_pattern_non_bindings` is 2.8% of samples (2.7% with `p1b`).
- **Phase 4: take P2(a), sharing the walk between definitions and bind,
  not the prefilter.**

### P3: one `Lexical` build per Rust file

- `Lexical::build` runs drop from 5,713 to 4,113: one per Rust file. The
  1,600 files with impl members each reused the first build, with 0 key
  mismatches. The plan expected the count "to halve on Rust"; it falls by
  28% because only 1,600 of 4,113 files had a second build.
- −10% wall and −11% CPU on cold Rust. With `p1b` and P2, −34% over none.
- The spike shared only the `Lexical::build` result (keyed by node IDs);
  `Declarations::new` still runs twice. Phase 4's P3 should carry the
  decisions as planned.

### P5: the gain bound

- The metadata block for `Local` records costs **3–7% of cold CPU** (−6%
  with `p1b` in both rounds, −3% and −7% without). The spec estimated
  10–15%. This supports the spec's recommendation to keep scope-independent
  metadata.

### O-findings: references profile

Workloads:
- 100 copies of `tree-hugger/lib/src/file/tree_file.rs` (Rust);
- 300 copies of `tools/test-audit/tests/aggregate.test.ts` (the largest TS
  file in the repository, 30 KB, nested `describe`/`it`; no larger
  TS/TSX file exists);
- 4 copies of `scripts/ci/test_affected_scope.py` (Python).

Each ran `--json-full symbols --no-cache` and `refs <decl>`, with and
without `p1b`. Profiles are in `/tmp/th-p3/prof/`.

| Workload | `run_query` samples, off → `p1b` | Navigation share of `run_query` | `OccurrenceSyntax::resolve` | O1: `is_ecma_export_name` | O2: `Regions::around` | O2: `scope_around` | `Walk::run` |
|---|---|---|---|---|---|---|---|
| Rust `--json-full` | 17,797 → 8,323 | 76% → 60% | 9,214 → 4,187 (87% → 83% navigation) | – | 2,255 → 1,416 (99%) | 1,796 → 891 (99%) | 610 → 336 (1% / 0%) |
| Rust `refs` | 13,723 → 8,922 | 77% → 70% | 8,569 → 6,325 | – | 2,036 → 2,118 | 1,492 → 1,141 | 486 → 497 |
| TS `--json-full` | 30,181 → 22,876 | 78% → 68% | 21,069 → 15,667 | **6,113 → 5,945 (96–98%)** | 2,115 → 2,583 | 1,716 → 1,747 | 6,067 → 3,011 (81% → 46%) |
| TS `refs` | 26,641 → 18,122 | 80% → 74% | 22,409 → 15,781 | 3,911 → 3,203 (95–97%) | 2,373 → 2,219 | 2,020 → 1,799 | 5,736 → 3,590 (81% → 68%) |
| Python `--json-full` | 1,463 → 1,046 | 48% → 36% | 1,080 → 823 | – | 152 → 173 | 22 → 16 | 30 → 32 |
| Python `refs` | 1,253 → 872 | 52% → 44% | 1,056 → 831 | – | 147 → 146 | 19 → 15 | 28 → 26 |

- **O1, export climb:** `is_ecma_export_name` is 11–17% of TS analysis and
  95–98% navigation, and `p1b` does not reduce it. It needs O1's
  nesting-aware index.
- **O1, excluded-context climb:** it is inlined into
  `OccurrenceSyntax::occurrence` (the `ancestors(..).any(is_excluded_context)`
  test), so the profile does not attribute it separately. Its cost is
  inside `OccurrenceSyntax::resolve`'s navigation, which stays at 79–87%
  with `p1b`. To separate it, Phase 4 can mark that closure
  `#[inline(never)]` in a scratch build.
- **O2:** `Regions::around` and `scope_around` stay 93–100% navigation with
  `p1b`. A memoized climb is still one lookup per level, and the
  pathological file above is all `Regions::around`.
- **O3:** `Walk::run` is 13–16% of TS analysis, 81% of it navigation.
  `p1b` halves its navigation, so O3's cursor-based walk is still worth
  doing.
- **P8:** after `p1b`, sibling navigation at the top of the stack is 105
  samples of 8,323 on Rust (1.3%) and negligible on TS and Python. P8's
  condition, that the `prev_sibling` share is still visible, is **not met**.
- **P12/O7:** no hashing or `rust_impl_type` frames show at a visible share
  in these profiles, so these conditional items are not supported.
- `work_counters()`: the spike changes no counter, so `reference_captures`,
  `occurrence_resolutions` and `definition_captures` stay at 1.

### C3: narrow hits over wide entries

After C1/C2, a narrow warm `--count` over entries widened by `--json-full`
costs **2.1× the wall time and 3.3× the CPU** of the same `--count` over
narrow entries (1,160 files: 0.557 s against 0.268 s). On the single
largest file the gap is small (0.129 s against 0.100 s), because C1/C2
removed the quadratic part. The remaining cost is decoding and validating
evidence the narrow query discards. The spec still recommends keeping full
validation; the gap is now for the author to weigh (rulings below).

### C5: Flags on every cache-enabled miss

- **Cost:** a cold monorepo `symbols --count` with cache writes is **+13%
  wall and +11% CPU** (6.13 → 6.95 s; 33.9 → 37.8 s user, two rounds), with
  identical output. The cache size after the listing is unchanged (759 MB),
  because the flag fields already serialize.
- **Benefit:** the `--json symbols` that follows drops from **7.81 s to
  1.08 s**. That is 4,311 hits with nothing parsed, against 4,311 partial
  hits that re-parse every file.

### C4: decode bench

A `cache_decode` criterion group in the spike's `lib/benches/tree_file.rs`
reads real entries through a hidden `cache::spike_decode_stored_entry` hook
(exactly `serde_json::from_slice::<StoredEntry>`). Command:
`TH_C4_SAMPLE=/tmp/th-p3/c4/sample cargo bench -p tree-hugger --bench
tree_file -- cache_decode`.

- **Entries:** p50, p90, p99 and max of the monorepo's `--json` cache
  (4,311 entries, 786.7 MB), plus p90 and max of a `--json-full` cache
  (346 entries).
- **The stripped form** is made by `/tmp/th-p3/c4/bin/strip.py`. It drops
  the per-record `schema_version`, `language`, `identity.stable_key`,
  `source.file_path` and `provenance`, keeping provenance once per entry,
  and turns every span into `[start_byte, end_byte, line, col, line, col]`.
- **Size:** the stripped monorepo cache is **500.0 MB against 786.7 MB
  (0.636×)**. Per entry the ratio is 0.61–0.88.
- The stripped form cannot be decoded into the typed records without a
  mirror DTO, so its typed decode is estimated. The estimate scales the
  typed full decode by the measured untyped ratio (`serde_json::Value`,
  which allocates like a typed decode); the pure-parse ratio (`IgnoredAny`)
  is the cross-check. Reconstruction is timed directly: re-adding the file
  path and formatting the stable key per record.

| Entry | Records | Typed decode, current | `Value` stripped / full | `IgnoredAny` stripped / full | Estimated typed stripped | Reconstruction |
|---|---|---|---|---|---|---|
| mono p50 | 6 | 92.6 µs | 0.78 | 0.69 | ~72 µs | 1.0 µs |
| mono p90 | 248 | 582.8 µs | 0.61 | 0.64 | ~354 µs | 64.7 µs |
| mono p99 | 736 | 2.391 ms | 0.55 | 0.65 | ~1.33 ms | 216.5 µs |
| mono max (`openai/types.rs`) | 7,732 | 35.80 ms | 0.65 | 0.67 | ~23.4 ms | 1.26 ms |
| wide p90 | 288 | 2.067 ms | 0.58 | 0.66 | ~1.21 ms | 44.4 µs |
| wide max (`openai/types.rs`) | 7,732 | 39.77 ms | 0.70 | 0.71 | ~27.9 ms | 1.26 ms |

**Decode-time ratio:** with reconstruction included, about **1.3–1.5×
faster decode** (p99: 2.39 ms → ~1.55 ms), plus 36% less disk and write
volume. That is below the spec's 2–4×, which counted on lazy facet decode
(C4(c)) as well; that was not prototyped. The bench ran while the
equivalence checks used 4 of 16 cores. The ratios are within-run
comparisons on the same entries, so they are not affected.

### L2 and P11

- The window recovers **17–23% of wall time** at 4–16 threads, cold and
  warm, with unchanged CPU. That is the barrier idle the spec predicted.
  Monorepo cold is 5.92 → 4.58 s.
- `p11-nocount` changes nothing at any thread count. The counters are not a
  contention cost.
- **Phase 5: do L2. Drop P11** (keep the atomics).
- Even with the window, 16 threads reach 6.6× the 1-thread time on
  `darkmatter/` (8.21 / 1.245 s). The 4 efficiency cores account for part
  of that. In the monorepo, the pathological file above dominates any
  workload that includes it.

### B1 and B2

- The combined build changes cold `--count` by 0–5% (not clearly above
  noise) and warm `--count` by −4–6% wall and −11% CPU. Warm `--plain` is
  **−15%** (2.34 → 1.97 s).
- That gain was clearly above noise, so the two were isolated with a
  mimalloc-only build. Mimalloc alone gives the same `--plain` gain (fastest
  runs 2.58–2.62 s → 2.16–2.24 s); LTO adds nothing measurable on top.
  These rows ran at load 12–18, so they compare fastest runs.
- The LTO build takes about the same time on this host (4 m 38 s), but
  `codegen-units = 1` will lengthen incremental release builds.

### Rulings summary (for the author)

Each Open Question with this phase's numbers and the spec's
recommendation. **The author's rulings are not recorded yet:** this is a
non-interactive session, so the questions are raised in the spec's
`human_review_items`. Phase 7 implements only what the author approves.

| Question | Numbers | Spec's recommendation |
|---|---|---|
| **C5** — which capabilities a miss computes | Flags on every cache-enabled miss: +13% wall / +11% CPU on a cold listing; the next `--json` 7.81 → 1.08 s; no size change | Flags only, for cache-enabled misses; `--no-cache` stays selective |
| **C3 / C6** — may narrow requests skip evidence validation | After C1/C2, a narrow hit over wide entries is 2.1× wall and 3.3× CPU of a narrow-only hit (0.557 vs 0.268 s, 1,160 files); single large file 0.129 vs 0.100 s | Keep full validation; no checksum substitute |
| **C4 / P15** — private storage form | Compact private JSON: 0.636× bytes, ~1.3–1.5× faster decode including reconstruction; `declaration_text` is 4% of cache size (spec) | Compact private JSON with exact public reconstruction; P15 as a private range only if approved |
| **P5** — scope-dependent metadata | Gain bound 3–7% of cold CPU | Keep scope-independent metadata |
| **O4** — uncertain candidates | Phase 1: `self` gives 89% of candidates; `--json-full` is 588× the source; one cache entry 223 MB | Preserve public lists; share lookup and storage internally |
| **L5** — batched linter metadata | oxlint not installed: 32 µs per file of discovery, serial; spawn counts not measurable here | One metadata record per actual batch |
| **L3** — literal positional paths | Phase 1: 70.6 ms in-repo vs 7.5 ms outside | Conservative fast path; keep positional semantics |
| **L6** — terminal discovery | Pseudo-TTY worst case 2.1 s (unanswered OSC queries); the real WezTerm sample is still the author's | Deterministic unstyled constructor only |
| **C9** — obsolete cache entries | Monorepo `--count` + `--json` cache 759 MB; a `--json-full` cache for 1,160 files is 934 MB | Explicit `hug cache clear` |
| **B1 / B2** — release profile, allocator | LTO adds nothing measurable; mimalloc gives −15% on warm `--plain` and −11% CPU on warm `--count`, nothing cold | Defer until the algorithm fixes land |

**Plan decisions this phase settles** (not Open Questions; the plan made
them conditional on these numbers):
- **P1:** (c), the full index, not (b). See the pathological file.
- **P2:** (a), sharing the walk, not the prefilter (27% skip rate).
- **P3:** do it.
- **P8:** not done (the sibling share is 1.3%).
- **P12, O7:** not supported by the profile.
- **L2:** do it in Phase 5.
- **P11:** drop.

### Checkpoint 3

- Every spike row is logged above.
- The rulings are **not** recorded: they await the author (`human_review`
  on the spec).
- No CI run happened.
- The spike worktrees are removed; the spike survives only as
  `/tmp/th-p3/spike.patch` (plus `/tmp/th-p3/b12.patch`), for reference.

### Gates

- Scope calculator, run from the repository root after every Phase 3 edit:

  ```sh
  git status --porcelain --untracked-files=all | awk '{print $NF}' \
    | xargs python3 scripts/ci/affected_scope.py
  ```

  `"packages": ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`,
  `"change_class": "package"` (exit 0). Phase 3 itself changed only this
  feature's three Markdown files. The packages come from Phase 2's source
  changes, which are still uncommitted in this working tree, so the gates
  were required and were run. The tree also holds edits outside this
  feature (`CLAUDE.md`, `.claude/skills/os/SKILL.md`,
  `.claude/skills/rust-devops/ci-cd.md`, `docs/topics/ci-cd.md`,
  `features/2026-10-08-windows-to-nightly/`), which this phase did not touch.
- `just test` (in `tree-hugger/`, nothing else running): **2034 passed, 0
  failed** (204 slow), exit 0, 263 s.
- `just lint`: exit 0, no warnings.
- `test-toolkit`'s gates were not run here; this phase changed nothing it
  reads.

### Requirement-to-test mapping

Phase 3 changes no behavior in the repository. Every arm lived in the
removed spike worktree, so no test was added, changed or removed. The
spike's own correctness evidence was the `p1b-check` runtime assert and
the eight-configuration output equivalence above. A grep of `tree-hugger/`
for `spike::`, `HUG_SPIKE`, `SpikeParent` and `.sparent()` finds nothing.

### OS considerations

Nothing to cross-check. No code changed, and the measurements are one-host
samples by design (spec: "Spikes are quick samples on one host"). B2's
supported-target build evidence (Windows, WSL2) is needed only if the
author adopts mimalloc in Phase 7.

### Scratch artifacts

These are kept under `/tmp/th-p3/` and are not in the repository:
- `spike.patch`, `b12.patch`, `hug-spike`;
- `bin/` (the `equiv.sh`, `t.sh`, `prof.sh` and `callers.py` helpers);
- `prof/` (the `sample` profiles);
- `c4/sample/` with `c4/bin/strip.py`;
- `timing-*.txt`, `equiv-*.txt` and `c4-bench.txt`.

The scratch caches and the three target directories were deleted.

## Phase 4

Navigation and Rust sharing. Revision `853688d67` (branch `feat/symbols`),
macOS, 16 cores. The base binary for the output diff protocol is
`/tmp/th-p4/hug-base`, a `profiling` build of `853688d67` made before any
edit.

### P1: the parent index

- **What:** `lib/src/file/parents.rs`. One `TreeCursor` pass records every
  node's parent by node id (a `HashMap` with a multiplicative id hasher),
  as Phase 3 recommended: (c), not the lazy memo (b).
- **Departure from the spec ("threaded into the readers"):** the index is
  installed on the calling thread for the duration of one closure
  (`with_parent_index(&Tree, f)`, entered by `run_query`), and read through
  `parents::parent(node)` / `node.indexed_parent()`. A lookup on another
  tree, on another thread or outside the closure falls back to
  `Node::parent()`. Reason: the readers are the per-language extraction
  helpers, 201 call sites in 37 files; threading a parameter through all of
  them would touch nearly every signature for no difference in what is
  computed. The index still never outlives its tree: the closure borrows
  the `Tree`, and the index is uninstalled on exit, panics included.
- **The library's first `unsafe`:** one `Node::from_raw` in
  `parents::parent`, turning a recorded parent back into a `Node`. The
  safety argument is at the call: the index is only installed while
  `with_parent_index` borrows its tree, and a node is answered from it only
  when its tree pointer equals the index's.
- **Callers:** every tree-sitter `.parent()` in `lib/src` (outside tests)
  now goes through the index, as the spike did, plus `scope::ancestors`.
  The `Path::parent` calls (`cache/mod.rs`, `naming.rs`, `flags.rs`) are
  untouched.
- **`fill_metadata` scope reuse:** the winner's flags read `symbol.scope`
  (computed by `materialize` with the same declaration, name and kind); each
  later candidate computes its own scope once.
- **Pin:** `file::parents::tests::indexed_parents_match_node_parents_over_the_symbols_fixtures`
  asserts `parent(n) == n.parent()` for every node of every
  `tests/fixtures/symbols/**` file (46 files, all parsed) plus three broken
  sources; it asserts the corpus held anonymous, aliased, missing and error
  nodes. Three more unit tests cover the fallback for another tree, nested
  scopes, and uninstalling on panic.
- **Pathological file** (`schematic/definitions/src/openai/mod.rs`,
  `--json-full symbols --no-cache`, one run, load ~20): **1.04 s** with the
  index, against 535.75 s in Phase 3.

### O2: scope and region climbs

- `ScopeTree::scope_around`, `Regions::around` and `ScopeTree::with_rules`
  (parents, boundaries, the depth tie-break) climb through
  `scope::ancestors`, which now reads the P1 index: each step is one map
  lookup instead of a root descent.
- **Departure from the plan:** `with_rules`' parents and depth are not a
  span-sorted stack sweep. Byte-span containment cannot prove ancestry for
  zero-width (`MISSING`) or equal-range nodes, the same concern the spec
  raises for O1; an indexed ancestor climb is exact by construction and
  `O(depth)` per scope node.

### O1: excluded-context and export-name lookups

- `Walk::run` keeps a frame per node on its cursor path and records two
  node-id sets in the same pass:
  - `in_excluded_context`: nodes with an excluded-context ancestor
    (`is_excluded_context`, per language);
  - `export_names`: TS/JS tokens `is_ecma_export_name` accepts, decided from
    the nearest `export_statement` on the path (and whether it has a
    `source`) and the node's parent.
- `Resolver::occurrence` reads the sets instead of climbing. Both sets key
  on node identity, so byte-equal siblings and zero-width nodes never share
  an answer.
- `is_ecma_export_name` (still used by lint's reference reader) and the walk
  share one decision, `names_module_export(node, parent, statement,
  from_source)`.

### O3: no parent or sibling calls in the walk

- TS/JS: `extra_scope` and `dynamic` no longer ask every named node for its
  parent (`is_with_body`). A `with_statement` reports its body
  (`LanguageRules::extra_child_scope`, `ecma::with_body`), and `dynamic`
  returns `Dynamic::Scope(body)` for the statement. Both consumers (the
  occurrence walk and `Lexical::build`) put scopes and dynamics into a
  `ScopeTree`, which sorts scopes and only sets flags, so visiting the
  statement instead of its body changes nothing.
- Swift: `swift_element_labels` reads siblings from one child list instead
  of `next_sibling`/`next_named_sibling` per child.
- The smaller per-node callers (Java, C, Python, ECMA) keep their parent
  calls; those are now index lookups.
- `just test symbols_occurrences symbols_scope symbols_relations
  symbols_owner_identity symbols_qualified_names occurrences scope`: 179
  passed.

### P3: one relation walk per Rust file

- `Lexical::build` (and `place`, `redirect_descendants`) now take entries
  under any lifetime: the walk reads only their nodes, kinds and the
  source, so its result lives as long as the tree. No `transmute` (the
  spike's approach) is needed.
- `relations::LexicalWalk` holds the built `Lexical` (behind an `Rc`), the
  walk's `Collected` lists and a key: each entry's `(name, declaration,
  anchor)` node ids and kind. `impl_self_types` builds it, uses it, and
  returns it; it travels as `Definitions::rust_walk` through `run_query`
  into `TreeFile::records_from` and `link_relations`, whose
  `Declarations::walk` uses it when the key equals its own entries' key
  and builds its own otherwise.
- **Not shared: the self-type decisions.** `link_relations` still resolves
  each impl's self type with its own `Declarations`, because it needs the
  same lookup site to resolve the impl's trait, and its entries carry the
  records' qualified names, which the key does not compare. Sharing the
  walk removes the duplicate `Lexical::build`, which is the cost Phase 3
  measured (5,713 → 4,113 builds, −10% wall on cold Rust).
- **P14 (the `link_relations` part):** `written_base_lists` reads each
  wanted record's written bases first. Outside Rust and Swift, a file with
  none returns before building `Declarations`. `written_bases` resolves
  the precomputed list, so `declared_bases` still runs once per record.
- **Work counts change on purpose:** a reused walk adds no
  `shared_context_steps` for the second build, and a skipped linker adds
  none for `Declarations::new` (`entries.len()`). No existing expectation
  in `symbols_shared_context` or `symbols_relation_work` changed: they
  assert linear bounds, which still hold.
- Tests (`file::tree_file::tests`):
  - `rust_relation_linker_reuses_the_definition_walk`: a Rust file with
    inherent, trait and nested-module impls gives the same records (JSON)
    with and without the shared walk, takes fewer shared-context steps with
    it, links `new`→`S` and `u`→`U` and `S implements T`; a file without
    impl members carries no walk.
  - `a_file_without_written_bases_builds_no_relation_index`: Go, Python and
    TS files without bases take one relation link and zero shared-context
    steps in `records_from`; a Python class with a base still resolves it.

### P2: the Rust pattern walk, shared (P2(a))

- `rust_pattern_non_bindings` returns its placement as
  `occurrences::RustPatterns` (the `Walk`, the `ScopeTree` with the sites
  placed, and `PlacedSites`). `capture_definitions` keeps it only on the
  bind path (`definitions_with_scopes`) as `Definitions::rust_patterns`,
  and `run_query` hands it to `OccurrenceSyntax::new`.
- `place_sites` now always binds with the definition pass's targets
  (`Unreported`) and returns `PlacedSites { reads, targeted }`, the binding
  indexes whose target it asked for and their name spans. `bind_all` sets
  each one's target to its record (`ScopeTree::set_target`). No placement
  decision reads a target other than `Import`/`Unknown`
  (`bare_pattern_meaning`), and those are never retargeted, so the
  finished tree equals the one the bind pass built before.
- The prefilter (P2(b)) was not built: Phase 3 measured a 27% skip rate and
  no wall gain.
- Test: `analysis::occurrences::tests::shared_pattern_placement_gives_the_walked_evidence`
  compares, as JSON, the evidence of the shared placement with a second
  walk over two Rust sources covering constants, unit structs, imports,
  globs, `const` generics, `None`, nested and closure patterns; it requires
  a resolved record target, so a missed retarget fails it.

### Small cleanups (P6, P7, P10, P14 remainder, O6, O8)

Done:
- **P6:** `MetadataSelection` keeps the kinds asked for (`selected`) beside
  the widened set. `records_from` fills header texts, keyword, generics,
  attributes, modifiers, type-alias data, flags and metrics only where
  `selection.selects(kind)`; `fill_metadata` and the field/variant
  completion still read `wants` (access, `static`, doc, type metadata).
  Between `records_from` and `retain(query.selects)`, `run_query` reads
  only identities, kinds, IDs, spans and relations of unselected records,
  so nothing they lose was read.
  - One consequence on malformed input only: a widened member with more
    than `u16::MAX` generic parameters no longer fails the query
    (`TooManyGenericParameters`), since its generics are no longer read.
  - Test: `file::tree_file::tests::widened_members_complete_their_owners_without_record_facets`
    (Rust struct/enum, Java class/enum): the type records of a narrow query
    equal those of a query over every kind (Flags, Metrics, Containment),
    and the widened members carry no header texts, attributes or metrics.
- **P7:** `SymbolRecord::with_stable_key(info, key)` is `From<SymbolInfo>`
  with the final key; `records_from` spells the file path once
  (`stable_key_path`) and builds one key and one ID per record
  (`stable_key_with_path`). `provenance.extractor_version` is formatted
  once per process. Public field types are unchanged; the provenance
  strings are still cloned per record.
  - Tests: `shared::schema_v2::tests::stable_key_with_path_is_stable_symbol_key`
    (backslashes, edge spaces, non-ASCII, empty; the converted record's key,
    ID and extractor version) and
    `file::tree_file::tests::record_keys_and_ids_are_the_stable_symbol_keys`
    (repeated Rust locals, C++ overloads: every key equals
    `stable_symbol_key` with the counted ordinal, every ID its hash). The
    literal ID pin `symbol_ids_survive_schema_and_revision_bumps` still
    passes.
- **P10:** capture groups are keyed by the name's span (its text is read
  once, when a group is created), and each capture index's role (import,
  scope, definition kind, `.context`) is decided once per run instead of
  by string compares per capture.
- **P14:** `align_field_metadata` and `reconcile_variants` pair the k-th
  listed entry of a name with the k-th member through a name → queue map
  (`unpaired_by_name`) instead of a linear `find`; `binding_name` borrows
  an ASCII Rust name without `r#`; `docs_pass` takes the `DocsFacet`
  instead of cloning it. The `link_relations` early return is under P3.
- **O8:** `ScopeTree::visible` finds the latest ordered binding in one pass
  (ties go to the last, as `max_by_key` did); `import_at` reads a
  name-start map built once in `OccurrenceSyntax::new`.

Not done, with reason:
- **O6** (`is_call_target`, `qualified_parent`): the spec rates it low and
  "mostly subsumed by P1/O2"; with the index each `parent()` is one map
  read.
- **P14:** the double parameter parse (`signature` and `symbol_metrics`) and
  the cyclomatic arm siblings: neither showed in a profile, and the
  second runs under Metrics only.
- **O8:** owned `String` keys in `ScopeTree::table`, SipHash sets, kind-id
  compares, `definition_ranges` built twice, merging
  `wrapped_member_callees` into the walk, and the lint-path clones: none
  showed in the Phase 3 profiles, and each touches shared types for a gain
  below noise.
- **P8, P12, O7:** conditional; Phase 3's profiles did not meet their
  conditions (see the plan).

Targeted run after these: `just test symbols_occurrences symbols_relations
symbols_owner_identity symbols_shared_context symbols_relation_work
symbols_query symbols_cache symbols_flags symbols_metrics
symbols_field_lists symbols_scope symbols_docs tree_file occurrences scope
schema_v2`: **576 passed**.

### Output diff protocol

Base: `/tmp/th-p4/hug-base` (`853688d67`, built before any edit). New:
the `profiling` build of this phase's final code (`/tmp/th-p4/hug-new`).
Both ran from the repository root, one fixed scratch `TMPDIR` per arm,
`NO_COLOR=1`. Scripts (scratch, not in the repository):
`/tmp/th-p4/bin/protocol.sh` (Phase 2's, re-pointed),
`protocol2.sh` and `protocol3.sh`.

- **Main rows** (`tree-hugger/**`, the 16 `fixtures/symbols/multi/**` files
  by `--file`, `openai/types.rs`, `filesystem/mod.rs`; `--json-full
  symbols`, `--json symbols`, `symbols --plain`, `classes --plain`,
  `imports`, `--json lint`, `refs`; cold, warm ×2, after a wide run):
  **336 files, 332 identical.** The 4 differences are
  `fix.lint.{cold,warm1,warm2,wide}.err`, the usage error `lint --file`
  prints, which names the binary (`hug-base` against `hug-new`); Phase 2
  logged the same four.
- **Fixture rows** (`protocol2.sh`, from the fixture directory: `--json
  lint`, `lint --plain`, `imports --plain`, three `refs` and `refs --json`,
  `refs --plain --callers`, in all four states): **84 of 84 identical**,
  every exit code 0.
- **Wider rows** (`protocol3.sh`, cold, added because the parent index
  touches every language): monorepo `--json symbols` (258 MB) and
  `symbols --plain`; `--json-full symbols` over `darkmatter/**` (700 MB),
  `tools/**` (TS, 30 MB) and `scripts/**` (Python, 421 MB); `--json lint
  'tools/**'`. **18 of 18 identical** (outputs, stderr, exit codes).
- **The pathological file** (`openai/mod.rs`, `--json-full symbols
  --no-cache`, 90 MB): identical to the base output, both after P1 alone
  and with the whole phase. Base took **714.7 s** wall (669.6 s user, load
  ~20); new takes ~1 s.

### Timing sample

`hyperfine -N` (`-w 1 -r 3` unless stated), base and new back to back,
profiling builds, 16 threads, 2026-10-08 22:50–23:01, from the repository
root. An outside burst pushed the load to ~130 during the middle rows; base
and new of one row share the same load, and the CPU (user) time is the
steadier figure. Script: `/tmp/th-p4/bin/timing.sh`.

| Command | Cache | Load | Base wall / user | New wall / user | Change |
|---|---|---|---|---|---|
| monorepo `symbols --count` (`-r 5`) | `--no-cache` | 8.6 | 5.994 s / 33.43 s | **3.097 s / 18.54 s** | −48% wall, −45% CPU |
| monorepo `symbols --count` (`-r 5`) | warm | ~28 | 0.505 s / 1.73 s | 0.543 s / 1.73 s | unchanged (cache hits) |
| `darkmatter/**` `symbols --count` | `--no-cache` | 128 | 3.120 s / 14.10 s | 1.833 s / 7.99 s | −43% CPU |
| monorepo `symbols --count --language rust` | `--no-cache` | 130 | 10.929 s / 50.97 s | 6.671 s / 30.60 s | −40% CPU |
| `--json-full symbols tree-hugger/**` (Rust) | `--no-cache` | 130 | 4.495 s / 14.14 s | **1.405 s / 4.79 s** | −66% CPU |
| `--json-full symbols tools/**` (TS) | `--no-cache` | 115 | 887 ms / 2.52 s | 381 ms / 1.16 s | −54% CPU |
| `--json lint tree-hugger/**` | `--no-cache` | 107 | 1.312 s / 5.16 s | 848 ms / 3.36 s | −35% CPU |
| monorepo `types --json` | `--no-cache` | 92 | 10.644 s / 49.59 s | 3.947 s / 22.45 s | −55% CPU (P6 included) |
| monorepo `refs resolve_declaration_node` (one run each) | `--no-cache` | 53–57 | **538.88 s** / 628.94 s | **6.77 s** / 36.65 s | 80× wall; output identical |
| `openai/mod.rs --json-full symbols` (one run each) | `--no-cache` | ~20 | 714.67 s / 669.62 s | ~1 s | output identical |

Against Phase 2's cold monorepo `symbols --count` (6.19 s / 33.27 s at load
17–21) and Phase 3's best combined spike (`p1b,p3-share,window`: 2.951 s /
22.36 s, which includes Phase 5's window): this phase reaches 3.10 s /
18.54 s without the window.

### Acceptance check for the pathological file (P1/O2)

Phase 3 asked for a work-count assertion, not a timing one.
`file::parents::tests::a_complete_analysis_of_a_wide_generated_file_never_descends_from_the_root`
generates the shape of `openai/mod.rs` (one function building
`Vec::from([...])` of 120 struct literals full of `vec![...]` token trees
and closures), runs `SymbolQuery::complete_index()`, and asserts, through a
test-only per-thread lookup counter in `parents::parent`, that more than
10,000 parent lookups were answered by the index and **none** fell back to
`Node::parent()`. Removing the install from `run_query` sends every lookup
to the fallback and fails it.

### Gates

- `just test` (tree-hugger area, after every code edit but the last):
  **2045 passed, 0 failed** (78 slow), 206 s. `just lint` then failed on one
  clippy `too_many_arguments` (`OccurrenceSyntax::new` with the pattern
  placement as an 8th argument); fixed by moving it to
  `OccurrenceSyntax::with_rust_patterns`, not by an `allow`. Then `just
  lint`: clean; the occurrence/cache/relation/`tree_file`/`refs` subset:
  459 passed; and `just test` again: **2045 passed, 0 failed** (82 slow),
  189 s.
- The output diff protocol and timing ran on the binary built before the
  last two edits (the test-only counter, compiled out of non-test builds,
  and the builder refactor, which moves one argument).
- Scope calculator, run from the repository root after every Phase 4 edit:

  ```sh
  git status --porcelain --untracked-files=all | awk '{print $NF}' \
    | xargs python3 scripts/ci/affected_scope.py
  ```

  `"packages": ["tree-hugger", "test-toolkit"]`, `"change_class":
  "package"`, so both gates were required and were run. `test-toolkit` was
  not edited; the calculator lists it as a reader of these files. Its
  gates were not run here (its own area; CI runs them).
- No pre-existing or skipped failures.

### Requirement-to-test mapping

| Change | Tests |
|---|---|
| P1 index equals `Node::parent()` | `file::parents::tests::indexed_parents_match_node_parents_over_the_symbols_fixtures` (46 fixtures + 3 broken sources; anonymous, aliased, missing, error nodes), `indexed_parents_are_node_parents_inside_and_outside_the_scope` (new) |
| P1 scope rules (fallback, nesting, panic) | `another_trees_nodes_fall_back_and_the_scope_restores_on_exit`, `a_panic_inside_the_scope_uninstalls_the_index` (new) |
| P1/O2 acceptance (no root descents) | `a_complete_analysis_of_a_wide_generated_file_never_descends_from_the_root` (new); pathological file byte-identical |
| P1 `fill_metadata` scope reuse | `symbols_scope`, `symbols_query`, `symbols_field_lists` (existing); protocol |
| O1 walk-recorded sets | `symbols_occurrences` (existing, incl. TS re-exports and excluded contexts); protocol (`refs`, `--json-full`, TS `lint`) |
| O3 `with` body, Swift labels | `symbols_occurrences`, `symbols_scope` (existing); protocol |
| P3 shared walk, P14 early return | `file::tree_file::tests::rust_relation_linker_reuses_the_definition_walk`, `a_file_without_written_bases_builds_no_relation_index` (new); `symbols_relations`, `symbols_relation_work`, `symbols_shared_context` (existing) |
| P2(a) shared placement | `analysis::occurrences::tests::shared_pattern_placement_gives_the_walked_evidence` (new); `PATTERN_IDENTITY_MATRIX`, `PATTERN_RECORD_MATRIX` (existing, now on the shared path) |
| P6 record facets for selected kinds | `file::tree_file::tests::widened_members_complete_their_owners_without_record_facets` (new); `symbols_field_lists` (existing) |
| P7 key and ID once | `shared::schema_v2::tests::stable_key_with_path_is_stable_symbol_key`, `file::tree_file::tests::record_keys_and_ids_are_the_stable_symbol_keys` (new); `symbol_ids_survive_schema_and_revision_bumps` (existing literal pin) |
| P10, P14 rest, O8 | existing suites (`symbols_field_lists`, `symbols_query`, `symbols_docs`, `symbols_occurrences`, `symbols_scope`); protocol |

Every new test is an L1 unit test in the library (`file/parents.rs`,
`file/tree_file.rs`, `analysis/occurrences.rs`, `shared/schema_v2/mod.rs`),
compiled by the lib target and selected by `just test` (no tier marker in
any path segment). The fixture test reads
`tests/fixtures/symbols` through `CARGO_MANIFEST_DIR` joined with a
literal. The Input Robustness Matrix does not apply: no parser or
configuration reader changed.

### OS considerations

No cross-check was run. Nothing here reads paths, the environment or the
terminal: the index keys on tree-sitter node addresses and tree pointers
inside one process, and the thread-local install is plain `std`. The P7
key path keeps `stable_symbol_key`'s exact `\` → `/` spelling (pinned with
a Windows-style path in `stable_key_with_path_is_stable_symbol_key`). CI
runs the pull request on Linux and macOS.

### Checkpoint 4

- PR 2's changes are in the working tree (uncommitted, per instructions):
  byte-identical output diff, green `just test` and `just lint`.
- Cold monorepo `symbols --count` is logged: **3.097 s / 18.54 s CPU**,
  against 5.994 s / 33.43 s for the base binary at the same load.
- Stop for review.

## Phase 5

Output pipeline and command-level work. Base revision `853688d67` plus
Phase 4's uncommitted tree; macOS, 16 cores. The base binary for the output
diff protocol is `/tmp/th-p5/hug-base`, a `profiling` build of that tree
made before any Phase 5 edit.

### L2: the sliding window (`cli/src/scan.rs`)

- `in_file_order` now runs `rayon::current_num_threads()` scoped worker
  threads (not the rayon pool, so the delivering thread never shares the
  worker its head result needs). Workers claim indices from an atomic
  counter and send `(index, result)`; delivery takes the head from a
  `BTreeMap` as it arrives.
- **Admission is bounded by flushed output, not by deliveries.** A file is
  started only while `index < flushed + W` (`W = window_size() = 4 ×
  threads`, today's chunk size). `deliver` now receives a `Flush` and is
  asked to flush every `W` deliveries, before waiting for a head result
  that has not arrived, and after the last file; the window advances only
  past a delivery that flushed. So a failed write (seen at a flush) leaves
  at most one window started, the bound the existing closed-pipe tests
  assert for chunks (`CHUNK = 8` at two threads).
  - Found while testing: with admission tied to deliveries, `--plain types`
    into a closed pipe parsed 9 files (bound 8), because `ListingStream`
    held a one-file unit until the next file arrived, so the first flush
    had nothing to write. **Fix:** a one-file unit is complete on arrival
    and is now written at `push` (`ListingStream::units_are_files`). The
    byte order is unchanged (protocol below).
- Cancellation: `Break` stops the window and wakes waiting workers; the
  receiver is dropped before the scope joins, so no analysis outlives the
  call. A panicking job is caught and re-raised when its file is next to
  be delivered (earlier files are still delivered), never left as a gap.
- `flush_at_chunk_end`/`chunk_size` are gone; `JsonReport::flush(Flush)`
  replaces `flush_at_chunk_end`.
- **P11 dropped**, as Phase 3 decided; the atomic counters stay.

### L1: rendering on the workers (`cli/src/render.rs`, `declarations.rs`)

- `FileOutcome<P>` is generic, and each writer `map`s the prepared records
  into its per-file form on the worker. `write_listing`, when the stream's
  units are files (`--sort file` without groups, or `--group-by-file`
  alone) and the layout is the terminal or Prose one, renders the file's
  unit into a `Vec<u8>` through a fresh layout (`Renderer::package_listing`,
  now `pub(crate)`); `FileListing::rendered` carries the bytes and the
  stream writes them as they are. `Renderer` is `Sync` (it compiles).
  Width and capabilities come from the `Renderer` built once on the main
  thread; rendering reads them only.
- **Not on workers:** Markdown (its layout carries blank-line state from
  one block to the next), and the cross-file units (`--group-by-module`,
  flat `--sort name|kind`, `--prelude`), `classes`, `--tree` and tables,
  which keep the delivery path.
- `Renderer::term_at` no longer clones the `Terminal` per line: when
  wrapping, `Renderer::new` lays out one terminal per indent `0..=16`
  (`indented`), with a clone only beyond that.
- **Not done:** the biscuit-terminal literal-row fast path (a separate
  item in the plan). Worker rendering already took warm `--plain` from
  2.35 s to 0.72–0.77 s (timing below), and the fast path would add a new
  biscuit-terminal API for the remaining worker-side cost.

### L4: two-phase `hug refs` (`cli/src/refs.rs`, `scan.rs`)

- Phase 1 reads every file once (`Analyzer::read`) and analyzes its
  declarations over every kind and scope (`SymbolQuery::new()`); a file
  whose records match keeps its `SourceFile` snapshot (`Phased::Read`).
  Ambiguity (several matches) and the empty report are decided from phase
  1, before anything is written. Phase 2 analyzes the one declaring file
  with Occurrences from that snapshot (`Analyzer::analyze_source`), and
  the reported record is phase 2's record with the same ID. No text
  prefilter.
- **A scan of exactly one file** asks for Occurrences in phase 1 and keeps
  the result (`Phased::Analyzed`), with no phase 2. Without this, the
  common `hug refs name file.rs` would parse its file twice on a cold run;
  with it, every existing `refs` test (72, many pinning exact `--verbose`
  totals) passes unchanged.
- **`--verbose` totals change for multi-file scans, on purpose:** each
  phase's lookup is counted. Cold over three files with one declaring:
  `0 hits, 1 partial hits, 3 misses; parsed 4 files`; warm: `4 hits`.
  Documented in `docs/references.md`.
- **Departure (observable only on failure):** a non-declaring file whose
  *occurrence* analysis would fail while its declarations succeed is no
  longer reported as a file error, since its occurrences are no longer
  analyzed. A failure in phase 2 itself is reported as a file error with
  an empty report.

### L7, L14, L15

- **L7:** `listing_order` memoizes `find_package_root` by start directory
  for the run (a local `HashMap`), so it is the same function on the same
  inputs, asked once per directory.
- **L14, JSON:** `--json`/`--json-full` entries and `--jsonl` lines are
  projected (`trim_record`, `relativize_index`), ordered and serialized on
  the worker (`report::serialized`); `JsonReport::entry` appends the bytes.
  A serialization error becomes the same write error as before.
- **L14, stats:** `Stats` counts by `SymbolKindV2`, `VisibilityLevel` and
  `ProgrammingLanguage` (`Counts<K>`, first-seen order) and stringifies
  only on output, into `BTreeMap`s keyed by the serialized spelling, so key
  order and labels are unchanged. (The old shared label map could collide
  on `Unknown`, kind and visibility; both label it `unknown`, so per-map
  labels are identical.)
- **L14, not done:** the run-scoped `is_test_file` manifest memo and the
  precomputed root identity for `portable_relative`. Both now run on the
  workers. Measured: replaying `is_test_file`'s ancestor `Cargo.toml`
  probes for all 4,040 Rust files with records, twice each, costs 48 ms of
  serial Python stat calls (27,674 stats), about 2–3 ms of wall time over
  16 workers against a 491–641 ms warm `--json`. The memo would need a
  new run-scoped option on the public `SymbolQuery` (which derives `Eq`)
  for under 1%.
- **L15:** the scan takes reused prelude analyses out of the reuse map
  (`Mutex<HashMap>`, `remove`) instead of deep-cloning them; each scanned
  file is listed once, and the prelude filter is built before the scan.
  The prelude file is read once: its export lines are checked against the
  text the analysis read (`Analyzer::read` + `analyze_source`). **Not
  done:** resolving prelude targets in parallel (few targets, and the serial
  order is what keeps the reuse map deterministic).

### L11: streaming `imports` and `lint` (`cli/src/main.rs`)

- Text mode renders each file's summary in the delivery closure into the
  buffered stdout and flushes as the window asks; a write error breaks the
  scan (`hug imports | head -1` now stops within one window). `--json`
  still collects one envelope, serialized by reference (`JsonOutput<'a>`),
  not from clones.
- Lint's clean-file suppression is decided up front from `files.len() > 1`
  (the spec's rule). **Departure:** with two or more selected files of
  which only one is linted (the others failed), the clean one is now hidden
  and `No lint diagnostics in 1 files.` is printed; before, it was shown
  with `(no diagnostics)`. `N` still counts the linted files. Documented in
  `docs/symbol-reports.md` and the CLI README.
- An unavailable adapter (`oxlint`) is warned once per tool, on stderr,
  right before the summary of the first file it affected, after flushing
  the earlier files' output. Before, every warning came after the scan and
  before the whole report. Exit codes are unchanged (error count is summed
  as files arrive).
- Stderr error lines now interleave with stdout in file order, as for the
  declaration commands.

### L12: `god-files` (`lib/src/god_files/analysis.rs`, `file/source_file.rs`)

- Discovery rejects a file smaller than 400 bytes by metadata before
  reading it (a file has at most one physical line per byte).
- Analysis reads each candidate once: the physical-line count and the
  parse come from the same bytes (`SourceFile::from_read`, a crate-private
  constructor split out of `read_with_language`; non-UTF-8 bytes fall back
  to physical-line banding as before). The parsed source is borrowed, not
  copied.
- The three queries are not merged (the spec's "first remove duplicate
  reads"). Timing: monorepo `god-files --plain` 2.620 s → 2.619 s, no
  change: parsing dominates. Output identical, text and `--json`.

### Output diff protocol

Base `/tmp/th-p5/hug-base`, new `/tmp/th-p5/hug-new` (all Phase 5 code but
the last clippy borrow fix in `god_files/analysis.rs`, which removes three
`&`s). Both from the repository root, one fixed scratch `TMPDIR` per arm.
Scripts (scratch): `/tmp/th-p5/bin/protocol.sh`, `protocol2.sh`.

- **Main rows** (`tree-hugger/**`, the 16 `fixtures/symbols/multi/**` files
  by `--file`, `openai/types.rs`, `filesystem/mod.rs`; cold, warm ×2, after
  a wide run). Phase 4's commands plus: `--plain imports`, `--plain lint`,
  `--jsonl`, `--count`, `--stats --json`, `--plain --stats`, `--format
  prose`, `--format markdown`, `--group-by-file`, `--group-by-module`,
  `--sort name`, `--format table`, `--tree`, `--json functions`, `--format
  tsv`, a forced-color `COLUMNS=100` listing (`CLICOLOR_FORCE=1`, so the
  wrapped `term_at` path runs; 15,967 lines with escapes over
  `tree-hugger/**`), and multi-file `refs` over `tree-hugger/**`: one match
  (`resolve_declaration_node`), ambiguous (`new`, 59 matches, exit 1), and
  none. **1,164 files, 1,148 identical.** The 16 differences are the usage
  error `lint`/`refs --file` prints, which names the binary (`hug-base`
  against `hug-new`), the same kind Phase 4 logged.
- **Fixture rows** (`protocol2.sh`, from the fixture directory, so `refs`
  and `lint` scan all 16 files): multi-file `refs` (JSON, text,
  `--callees`, `--callers`, no match, `new`), `--plain lint`, `--json
  lint`, `--plain imports`, `--plain lint --warn all`; cold and warm ×2.
  **90 of 90 identical**, every exit code 0.
- `god-files --plain` and `--json god-files` over the monorepo: identical.

### Timing sample

`hyperfine -N` (`-w 1 -r 5` unless stated), profiling builds, 16 threads,
2026-10-08 23:33–23:50, from the repository root, warm cache shared by both
binaries unless stated. Load 7–11 for every row (one burst to 35 at the
very end, after the last row).

| Command | Cache | Base | New | Change |
|---|---|---|---|---|
| monorepo `symbols --plain` | warm | 2.352 s / 3.74 s user | **0.774 s** / 4.16 s | −67% wall (L1) |
| monorepo `--json symbols` | warm | 1.140 s / 2.62 s | **0.491 s** / 2.57 s | −57% (L14 on workers, L2) |
| monorepo `symbols --count` | warm | 0.574 s / 2.03 s | 0.420 s / 2.03 s | −27%, same CPU (L2) |
| monorepo `refs resolve_declaration_node` (`-r 2`) | `--no-cache` | 4.883 s / 27.18 s | **2.380 s** / 18.55 s | −51% (L4) |
| same (`-r 3`, a cache per binary) | warm | 1.799 s / 6.54 s | **0.366 s** / 1.71 s | −80%; cache 2.6 GB → 760 MB |
| `--plain imports \| head -1` (shell) | warm | 568 ms | **138 ms** | L11 |
| same | `--no-cache` | 3.076 s | **0.169 s** | stops within one window |
| monorepo `god-files --plain` (`-r 3`) | — | 2.620 s | 2.619 s | none (L12) |

An earlier sample at load 7–9 gave warm `--plain` 2.354 → 0.723 s and warm
`--json` 1.061 → 0.641 s.

### Gates

- Targeted while iterating: `just test golden report export outline filters
  grouping order classes terminal_scope wiring` (600 passed), `just test
  refs` (72 → 75 passed with the new tests), `just test prelude` (17), the
  whole `tree-hugger-cli` package (868 passed before the new L11 tests),
  `god_files` lib tests (54 → 56).
- `just lint` first failed on three clippy `needless_borrow`s in
  `god_files/analysis.rs` (the source became a `&str`); fixed by dropping
  the `&`. Then clean (exit 0).
- `just test` (tree-hugger area, after the last code edit): **2059 passed,
  0 failed** (20 slow), 122 s.
- `just test-l2` (L1 changes rendering): **7 passed** (tmux and WezTerm
  run; the two kitty cases gate themselves off on this host and pass in
  12 ms). No window gained focus (the harness's background spawning).
- `just check-tier-coverage tree-hugger`: listed, 0 stranded.
- No pre-existing or skipped failures.

### Requirement-to-test mapping

| Change | Tests |
|---|---|
| L2 order with any worker count, flush at least once per window and at the end | `scan::tests::every_file_is_delivered_once_in_file_order_with_any_worker_count` (1, 2, 8 workers, uneven durations) |
| L2 empty input | `scan::tests::an_empty_input_delivers_nothing` |
| L2 slow head, one worker included | `scan::tests::a_slow_head_holds_the_window` (no file past one window starts while file 0 runs; 1, 2, 4 workers) |
| L2 failed write | `scan::tests::a_failed_write_stops_within_one_window` (≤ W started, no work after return) |
| L2 panicking head / later job | `scan::tests::a_panicking_job_is_propagated_in_file_order` |
| L2 closed pipe, every streamed layout | existing `symbols/report.rs` bounds (`a_failing_stdout_stops_every_streamed_layout_within_two_chunks`, `an_uneven_first_package_*`, `a_json_report_without_entries_*`, `a_reader_closing_early_*`), unchanged; the first one caught the held-unit bug |
| L1 worker rendering byte-identical | goldens (`golden`, `export_*`, `grouping_*`, `outline`, `terminal_scope`), `level2_declarations` in tmux/WezTerm; protocol rows `plain`, `prose`, `byfile`, `styled` (`COLUMNS=100`, forced color) |
| L1 `term_at` cache | protocol `styled` rows; `level2_declarations` (wrapped output) |
| L4 evidence only in the declaring file, same report as a one-file scan, cold/warm/`--no-cache` totals | `refs::a_multi_file_scan_analyzes_evidence_only_in_the_declaring_file` (new; fails on the old code: every entry gained occurrences) |
| L4 ambiguous and missing decided without an evidence phase | `refs::ambiguous_and_missing_selectors_run_no_evidence_phase` (new) |
| L4 failed file beside the declaring file | `refs::a_failed_file_beside_the_declaring_file_keeps_the_report` (new) |
| L4 one-file scans unchanged, locals, IDs, cache states | the 72 existing `refs` tests, unchanged |
| L7 package-root memo | `grouping_*`, `report::an_uneven_first_package_*` (deepest-package boundaries); protocol `bymodule`/`byname` |
| L14 JSON on workers | `report`, `golden`, `export_plumbing`; `report::failures_are_reported_in_file_order_with_any_thread_count`; protocol `json`/`jsonfull`/`jsonl` |
| L14 enum-keyed stats | `stats::tests::counts_are_keyed_and_ordered_by_their_serialized_spelling` (new: key spelling and order, `C#` before `C++`, labels); existing stats tests; protocol `stats`/`statsplain` |
| L15 consumed reuse, one prelude read | `symbols/prelude.rs` (reuse counts and parse counts pinned) |
| L11 streaming stop | `report::imports_and_lint_text_stop_at_a_failing_stdout` (new; fails on the old code: all 96 files parsed) |
| L11 suppression from the selected count, failed file, single clean file, all clean | `report::lint_text_hides_clean_files_by_the_selected_file_count` (new) |
| L11 adapter-warning placement | `report::an_unavailable_adapter_is_warned_once_before_its_first_file` (new; stdout and stderr in one pipe, empty `PATH`) |
| L11 closed pipe and other write failures | existing `report::imports_and_lint_end_quietly_on_a_closed_pipe`, `export_write_errors_preserve_broken_pipe_and_other_failure_statuses` |
| L12 line counting (CRLF, final line) | `god_files::analysis::tests::physical_line_counter_counts_correctly` (bytes, CRLF cases added) |
| L12 size pre-reject boundary | `candidate_discovery_size_screen_keeps_the_line_threshold` (new: 400 newlines kept, 399 dropped, CRLF) |
| L12 one read, non-UTF-8 fallback | `analysis_bands_a_non_utf8_candidate_by_its_physical_lines` (new); `level2_god_files`; protocol god-files rows |

Every new test is L1: the `scan`, `stats` and `god_files` tests are unit
tests in their modules; the CLI tests are in `cli/tests/symbols/{refs,
report}.rs`, compiled by the `symbols` test target. No path segment starts
with a tier marker. The Input Robustness Matrix does not apply: no parser,
manifest or configuration reader changed.

### OS considerations

- The window is plain `std` (`thread::scope`, `mpsc`, `Mutex`/`Condvar`,
  `catch_unwind`); nothing is OS-specific.
- Riskiest on native Windows: the adapter-warning test (one pipe for
  stdout and stderr, an empty `PATH`) and the closed-pipe bounds. A
  filtered `just cross-check tree-hugger-cli --os windows` of the ten new
  window, `refs` and `lint`/`imports` tests: **10 passed** on
  `build-win-native`.
- L12's size screen reads `fs::metadata().len()`, and its tests write
  CRLF and non-UTF-8 bytes explicitly.
- Linux and macOS run on the pull request.

### Scope calculator

Run from the repository root after every Phase 5 edit (plan, log and spec
frontmatter included):

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

`"packages": ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`,
`"change_class": "package"`, so the gates were required and were run.
`test-toolkit` was not edited; the calculator lists it as a reader of these
files (as in Phase 4). Its gates were not run here (its own area; CI runs
them).

### Checkpoint 5

- PR 3's changes are in the working tree (uncommitted, per instructions):
  byte-identical output (protocol), green `just test`, `just test-l2` and
  `just lint`, and this log entry.
- Stop for review.

## Phase 6

Native path identity (C8, L13, L3). Base revision `610ce33a7` (Phase 5
committed). The `os` skill's `windows-paths.md` was loaded first; nothing in
this phase changes how an identity is spelled, only how often it is
computed and which directories the walk opens.

### C8: one `realpath` per scanned file (`lib/src/file/source_file.rs`)

- `SourceFile::from_read` now resolves the identity and hands it to a new
  crate-private `SourceFile::from_identity(path, supplied, language,
  source)`, which holds the rest of the old body (language detection,
  hash). Errors still name `supplied`.
- New public `SourceFile::read_identified(identity, language)`: reads the
  file and keeps `identity` verbatim as `path()`.
- **Departure from the plan:** the plan asked for a *crate-private*
  constructor, but the scanner's main consumer, `Analyzer::read`, lives in
  the `tree-hugger-cli` crate. So `read_identified` is public, with the
  contract in its docs (only for paths that are already their own
  `file_identity`; anything else becomes the file's path, key and IDs).
  `from_identity` stays crate-private and is what god-files uses for its
  candidates (already identities). Public `read`/`read_with_language`
  still canonicalize. No schema or record change.
- A `debug_assert` that the path equals its identity was considered and
  dropped: it does I/O, and CLI unit tests (`export_write_tests`,
  `declarations::grouping_tests`) analyze raw temp-dir spellings (macOS
  `/var/…`), which `Analyzer` now keeps verbatim. Production callers are
  all identities: scanner output (`declarations`, `lint`/`imports`,
  `refs`) and `prelude.rs` (which calls `file_identity` first).

### L13: identity-only `--changed` membership (`cli/src/changed.rs`, `main.rs`)

`ChangedFiles::contains(file)` (which ran `file_identity` on every scanned
file, serially) is replaced by `contains_identity(identity)`, a plain set
lookup; its only caller was the scanner intersection in `main.rs`, so the
arbitrary-spelling form was removed rather than kept unused. Ownership
checks, nested-checkout rejection and conflict errors are untouched.

### L3: literal positional paths (`lib/src/scanner.rs`)

Rather than re-implementing the walker's rules for one path (the spec warns
`matched_path_or_any_parents` is not a substitute), the fast path **keeps
the `ignore` walker and prunes it**:

- `literal_targets(&glob_inputs)` returns the root-relative paths when
  *every* walked pattern is literal and anchored: contains `/`; no
  `* ? [ ] { } \ :`; not starting with `!`, `#` or `/`; no leading or
  trailing whitespace; no empty, `.` or `..` component. Otherwise (and for
  no patterns) `None`. These rules come from `ignore` 0.4.25's
  `GitignoreBuilder::add_line` (comments, negation, `/` anchoring,
  dir-only trailing slash, whitespace trimming, `**/` prefix for slashless
  patterns, `backslash_escape(true)`).
- With targets, `WalkBuilder::filter_entry` keeps an entry only when its
  root-relative path is a component-wise prefix of a target. `filter_entry`
  runs after `ignore`'s own checks, and walk roots are never filtered, so
  every entry the walk reaches is still decided by the same overrides,
  ignore stack, hidden and symlink rules; pruning siblings never changes a
  kept directory's ignore stack. A literal pattern's whitelist can only
  match its own path, so the full walk could only have selected targets.
- **One behavior difference (error path only):** an unreadable directory
  elsewhere in the package is never opened, so it no longer fails the
  scan (the full walk returns its `walkdir` error). Selected files,
  records and exit codes for readable trees are unchanged. Recorded in
  `docs/symbol-reports.md`.
- Bare names (`lib.rs`), globs and any mix with a glob still walk the
  whole tree, as before.

### Output diff protocol

Base `/tmp/th-p6/base/hug` (Phase 5's `hug-new`, the committed code but
the clippy-only borrow fix), new `/tmp/th-p6/new/hug` (profiling build of
this phase). Both binaries are named `hug` so usage errors compare equal.
Scripts (scratch): `/tmp/th-p6/bin/protocol6.sh`, `protocol2.sh`.

- **Main rows**, from the repository root, cold (`--no-cache`), warm ×2,
  after a wide run; `--json-full symbols`, `--json symbols`, `symbols
  --plain`, `classes --plain`, `imports`, `--json lint`, `symbols --count`,
  `--json functions`; corpora: `tree-hugger/**` (glob, full walk), the 16
  `multi/**` fixtures by `--file`, `openai/types.rs` and
  `filesystem/mod.rs` (literal, fast path), three literals together, a
  literal plus a glob, a literal fixture (`lib/tests/fixtures/sample.rs`), a
  literal under ignored `target/`, a missing literal, a bare name
  (`scanner.rs`), `--changed` and `--changed <literal>` (the working tree
  had this phase's edits), `refs` (JSON and text) on `openai/types.rs`,
  and `god-files --plain` / `--json god-files`. **1,206 files, 1,205
  identical.** The one difference is `elapsed_time_ms` (1 vs 0) in the
  wide-state `--json lint` over `tree-hugger/**`, timing metadata the
  protocol excludes. The ignored and missing literals exit 1
  (`NoSourceFiles`) in both arms; the fixture `lint`/`refs` rows are the
  known `--file` usage error (exit 2) in both.
- **Fixture rows** (`protocol2.sh`, from the `multi/` directory, cold and
  warm ×2): `refs` one-file, JSON, multi-file, `--callees`, ambiguous
  `new`, `--plain lint`, `--json lint`, `--plain imports`, `./rust.rs`
  (a `.` component, so the walk; nothing selected in both). **81 of 81
  identical.**

### Timing sample

`hyperfine -N`, profiling builds, 16 threads, warm shared cache, from the
repository root, 2026-10-09 00:08–00:09, load 9–13.

| Command | Base | New | Change |
|---|---|---|---|
| `symbols --count tree-hugger/lib/src/lib.rs` (`-w 2 -r 15`) | 66.9 ± 4.8 ms (sys 49.1 ms) | **9.2 ± 0.6 ms** (sys 4.4 ms) | 7.2× (L3, C8); Phase 1: 70.6 ms in-repo, 7.5 ms outside |
| `symbols --count --changed` (`-w 1 -r 8`) | 252.1 ± 3.8 ms | **204.9 ± 5.3 ms** | 1.23× (L13, C8) |
| `symbols --count --changed tree-hugger/lib/src/scanner.rs` | 144.4 ± 7.7 ms | **82.2 ± 1.4 ms** | 1.76× (L3 + L13) |

Phase 1's 70–103 ms single-file in-repo latency is now within 2 ms of the
outside-the-repository figure.

### Gates

- Targeted while iterating: `just test filters spelling changed
  cache_lookup export_write grouping file_identity god_files cli::` (192
  passed), then the plan's `just test filters spelling changed cache_lookup
  file_identity god_files` (161 passed).
- Negative control: with `literal_targets` forced to `None`,
  `filters::a_literal_positional_path_does_not_open_unrelated_directories`
  fails (the walk hits the locked directory) and the equivalence test still
  passes, so the narrowness test detects the fast path; restored after.
- `just lint` first failed on one clippy `cloned_ref_to_slice_refs` in the
  new `file_identity.rs` test (`[identity.clone()]`); fixed with
  `std::slice::from_ref`. Then clean (exit 0).
- `just test` (after the last code edit): **2063 passed, 0 failed** (16
  slow), 104 s.
- `just check-tier-coverage tree-hugger`: listed, 0 stranded.
- `just test-l2` not run: no rendering changed (the plan's Phase 6 verify
  list does not include it).
- No pre-existing or skipped failures.

### Requirement-to-test mapping

| Change | Tests |
|---|---|
| L3 selects exactly what the walker selects: anchored literal, `.gitignore`d file re-included, ignored parent, nested `.ignore` file and directory, fixture, hidden directory, several patterns, one missing among several, no match (`NoSourceFiles`, exit 1), non-source file, directory named like a source file, case mismatch, `--exclude-files` literal and glob, file symlink and symlinked directory (Unix) | `filters::literal_positional_paths_select_what_the_walker_selects` (new; each literal compared byte-for-byte, exit code and stderr, with the same pattern bracketed, plus the expected file list) |
| L3 bare name still matches at any depth; literal plus glob selects the union | same test (`lib.rs` ≡ `**/lib.rs`; `src/lib.rs other/*.rs`) |
| L3 opens only the directories on the way | `filters::a_literal_positional_path_does_not_open_unrelated_directories` (new, Unix; fails with the fast path disabled) |
| L3 + C8 identity: literal and walked selections share one cache entry and report | `spelling::literal_and_glob_selections_share_one_identity` (new; macOS `/var` alias root) |
| L3 fixture re-inclusion from the repository root | existing `cli.rs` tests naming `tree-hugger/lib/tests/fixtures/sample.*` positionally (now on the fast path) |
| C8 `read_identified` = `read` for scanner output (path, language, hash, cache key, records, IDs) under the alias and canonical root, literal and glob; forced language; missing file and unsupported language errors name the path; a non-identity spelling is kept verbatim while `read` resolves it | `file_identity::scanner_output_reads_identified_as_read_does` (new, lib L1) |
| C8 every spelling still one identity through the CLI | existing `spelling::*`, `prelude::prelude_reuse_holds_for_every_spelling_of_an_explicit_file`, lib `file_identity::*` |
| C8 god-files candidates | existing `god_files` lib tests (56), `file_identity::god_files_paths_are_relative_to_any_spelling_of_the_root`; protocol god-files rows |
| L13 membership by identity, ownership unchanged | existing `changed` suite (`selection`, `paths`, `ownership`, `cache`, `empty`, …; every scratch root is the macOS `/var` alias), `changed::tests::an_empty_changed_set_contains_nothing`; protocol `--changed` rows |

Every new test is L1: the CLI tests are in `cli/tests/symbols/{filters,
spelling}.rs` (the `symbols` target), the lib test in
`lib/tests/l1/file_identity.rs` (the `l1` target). No path segment starts
with a tier marker. The Input Robustness Matrix does not apply: no parser,
manifest or configuration reader changed (the positional pattern grammar is
`ignore`'s, unchanged; `literal_targets` only decides when pruning is
safe, and its equivalence is what the new tests compare).

### OS considerations

- `literal_targets` rejects `\` and `:`, so a native Windows separator or a
  drive-qualified pattern always takes the full walk. Targets are built
  from `/`-split components and compared component-wise
  (`Path::starts_with`), so the walker's native `\` entries match.
- Comparison is case-sensitive, as `ignore`'s overrides are
  (`case_insensitive(false)`), so `SRC/lib.rs` selects nothing on macOS and
  Windows in both paths (tested).
- `read_identified` keeps the scanner's spelling, which is
  `canonicalize_simplified` output (no `\\?\`, long names), exactly what
  the second `file_identity` call returned before.
- Rigs, `filters spelling changed cache_lookup` on `tree-hugger-cli` and
  `file_identity god_files` on `tree-hugger`:
  - Linux (`just cross-check tree-hugger-cli --os linux …`, `build-linux`):
    **67 passed**.
  - Native Windows (`./scripts/cross-check.sh … --os windows`,
    `build-win-native`): the first attempt never ran (the host's own
    `git fetch` from `github.com:22` timed out); the second ran 66 CLI tests,
    **64 passed, 2 failed** (below); after the fix, **66 passed**, with PASS
    lines for `filters::literal_positional_paths_select_what_the_walker_selects`
    and `spelling::literal_and_glob_selections_share_one_identity`. The lib
    run: **60 passed**, including
    `file_identity::scanner_output_reads_identified_as_read_does`.
  - WSL2 (`build-win`): **67 passed**.
  - The two Unix-only tests (`symlink`, `chmod 000`) are `cfg(unix)` /
    `cfg!(unix)`; the Windows leg runs the rest of each case table.

### Pre-existing Windows failures fixed (`cli/tests/changed/ownership.rs`)

Two `--changed` ownership tests from `cd0173e4b` (on this branch, not yet on
`main`, so never run by Windows CI) failed on native Windows before reaching
any Phase 6 code:

- `external_directory_alias_cannot_return_changed_identity_to_active_checkout`
  failed its setup assertion `file_identity(alias.rs) ==
  file_identity(clean.rs)` (left `…\alias.rs`), before `hug` ran.
- `nested_candidate_alias_cannot_return_changed_identity_to_outer_checkout`
  failed in `changed::resolve` → `checkout_path` (unchanged) with
  `os error 123` on `alias.rs`.

Cause: symlink targets spelled with `/` (`"bridge/back.rs"`,
`"../clean.rs"`, `"inner/subdir"`) do not resolve on Windows, the `os`
skill's `windows-paths.md` item 10 trap. Fix (test-only): those targets are
joined name by name (`Path::new("bridge").join("back.rs")`), identical bytes
on Unix. The other `inner/../clean.rs` spellings in the file already passed
on Windows and were left alone. Not a Phase 6 behavior change; fixed here
because this phase's pull request is the `ci:all-os` one.

### Docs and skill

- `docs/symbol-reports.md`: how a positional file filter is matched
  (relative `.gitignore`-style pattern, re-includes ignored/fixture files
  but not files under an ignored directory, never a symlink, bare names at
  any depth), the literal-path fast path and its one error-path
  difference.
- `docs/cache-design.md#file-identity`: scanner output is read with
  `SourceFile::read_identified`, and `--changed` membership is by identity.
- `.claude/skills/tree-hugger/SKILL.md`: the scanner identity and
  literal-path bullet, `read_identified` in the API block,
  `contains_identity` in the `--changed` paragraph.
- No README or dependency change (no crate added; `ignore` was already a
  dependency).

### Scope calculator

Run from the repository root after every Phase 6 edit (plan, log and spec
frontmatter included):

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

`"packages": ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`,
`"change_class": "package"`, so the gates were required and were run.
`test-toolkit` was not edited; the calculator lists it as a reader of these
files (as in Phases 4 and 5). Its gates were not run here (its own area; CI
runs them).

### Checkpoint 6

- PR 4's changes are in the working tree (uncommitted, per instructions):
  byte-identical output (protocol), green `just test` and `just lint`, the
  targeted suites green on every OS rig, and this log entry. The
  `ci:all-os` push is the commit step's.
- Stop for review.

## Phase 7

Approved policy changes. Base revision `610ce33a7` plus Phase 6's
uncommitted working tree.

### Rulings check: none recorded

Phase 7 implements only the Open Questions the author approved. Before any
edit, the spec's frontmatter, its Open Questions section, this log, the
plan and the feature directory's git history were searched for a recorded
ruling. **None exists:** `human_review` is still `true` with the questions
unanswered, and Phase 6's `message_to_agent` says to stop on every
ruling-gated item if none are recorded. So:

| Item | Gate | Outcome |
|---|---|---|
| **O4, free part** | none ("free part always") | **Built** (below) |
| O4, contract part | ruling | Deferred: no ruling |
| C5 (Flags on cache-enabled misses) | ruling | Deferred: no ruling |
| C4 / P15 (compact private cache form) | ruling | Deferred: no ruling |
| P5 (scope-dependent metadata) | ruling | Deferred: no ruling |
| L5 (batched oxlint metadata) | ruling | Deferred: no ruling; oxlint is also not installed on the dev host |
| L6 (unstyled terminal constructor) | ruling | Deferred: no ruling |
| C9 (`hug cache clear`) | ruling | Deferred: no ruling |
| B1 / B2 (build profile, mimalloc) | ruling | Deferred: no ruling (the spec recommends deferring anyway) |
| C3 (narrow validation) | only if the author overrules "keep full validation" | Not built: not overruled |

Each deferred item becomes a small follow-up once the author answers the
matching `human_review_items` entry on the spec.

### O4 free part: candidates sorted once (`lib/src/analysis/occurrences.rs`)

Before: every `NameOnly` resolution in a language without binding rules
cloned the whole same-named bucket (`by_name.get(..).cloned()`), and every
call of `name_only` stable-sorted its list by `(name_span.start, id)`, then
removed adjacent duplicate IDs (first wins). In a Python class with M
methods, each of the M `self` uses re-sorted the M-long bucket.

Now:
- `Resolver::new` stable-sorts each `by_name` bucket once with
  `Resolver::candidate_key` (the same key).
- `records_named` filters a sorted bucket. A stable sort followed by a filter
  gives the same order as a filter followed by the sort, so every filtered
  caller (member calls, `item_records`, Rust paths) is already in candidate
  order.
- The rule-less path passes the bucket to `name_only` as a slice: no clone.
- `name_only(&[usize])` no longer sorts. It keeps the first of adjacent
  equal IDs while building the `SymbolRef`s, as `Vec::dedup_by` did. The
  dedup deliberately stays per call: running it once in `new` and filtering
  afterwards could drop the kept copy and change which duplicate survives.
- The one input not drawn from a bucket, `Lookup::Found` with several
  bindings (in lookup order), sorts its own list before `uncertain`.
- `uncertain` takes `&[usize]`; the eight callers pass references.

No public change: the order (`docs/references.md`: "in declaration order")
and the dedup rule are the same, and no counter changes.

**Negative controls.** I removed each sort in turn (the bucket sort and the
`Lookup::Found` sort), and neither the new test nor
`occurrences_and_coverage_are_in_canonical_order` failed. I then added a
temporary `eprintln!` probe that reported any bucket or binding list arriving
out of candidate order. I ran it over every `symbols/**` fixture,
`tree-hugger/lib/src/**`, `scripts/**` and `tools/**`, plus constructed
JS/TS/Python/C++/Go/Rust cases (redeclared `var`s, hoisted functions,
overloads, decorators, out-of-line C++ members, Go methods before their
type). It reported **none**: records come in declaration order, and scope
buckets in placement order, which follows source order. Both sorts are kept
as defensive guarantees, because the old code sorted unconditionally and a
sort over an already-sorted list is one linear pass. The probe was removed
(the file was restored from a copy taken before the probe and checked with
`grep -c O4PROBE` = 0).

### Output diff protocol

Base `/tmp/th-p7/base/hug` (Phase 6's profiling build), new
`/tmp/th-p7/new/hug` (profiling build of this phase, after the last code
edit). Scripts (scratch): `/tmp/th-p7/bin/protocol7.sh` (Phase 6's rows plus
a `py` corpus, `scripts/**/*.py`, with `refs --json main` and
`refs --plain run`) and `/tmp/th-p7/bin/protocol2.sh` (Phase 6's fixture
rows plus `--json-full symbols nested/python.py` and
`refs --json grow nested/python.py`).

- **Main rows:** **1,326 files, 1,325 identical.** The one difference is
  `elapsed_time_ms` (0 vs 1) in the warm `--json lint` over
  `tree-hugger/**`, timing metadata the protocol excludes, as in Phase 6.
  The Python corpus's cold `--json-full symbols` alone is 388 MB with 33,168
  non-empty candidate lists (including `scripts/ci/test_affected_scope.py`,
  O4's 222 MB file), byte-identical in all four states. Exit codes match
  (the ambiguous `refs main` and the missing/ignored literals exit 1; the
  fixture `lint`/`refs` rows are the known `--file` usage error, exit 2).
- **Fixture rows:** **99 of 99 identical.**

### Timing sample

`hyperfine -N -w 1 -r 10`, profiling builds, `refs --json --no-cache
seed_build_inputs scripts/ci/test_affected_scope.py` (resolves every
occurrence of the 377 KB file), 2026-10-09 00:41, **load 120** (not a
quiet host; this is a sample, not a scaling spike):

| Base | New | Change |
|---|---|---|
| 325.7 ± 4.2 ms (user 267.5) | 322.7 ± 4.3 ms (user 267.2) | 1.01 ± 0.02×: no measurable change |

That matches the spec's "low–medium" expectation for the free part. Most of
the remaining cost is building each occurrence's public `SymbolRef` list,
which is the contract part (deferred).

### Gates

- Targeted: `just test name_only_candidates_are_in_declaration_order_and_distinct
  occurrences_and_coverage_are_in_canonical_order`: 2 passed.
- `just lint`: exit 0, no warnings.
- `just test` (after the last code edit): **2064 passed, 0 failed** (39
  slow), 163 s. Some tests were slow because the protocol was running on the
  same host.
- `just test-l2` not run: no terminal or rendering change.
- No skipped or pre-existing failures.

### Requirement-to-test mapping

| Change | Tests |
|---|---|
| O4: candidates still in name-position-then-ID order and distinct, for every language fixture | `symbols_occurrences::name_only_candidates_are_in_declaration_order_and_distinct` (new; compares each candidate's record `name_span.start`, then ID, through `reference_evidence` and `symbol_index_v2`) |
| O4: rule-less bucket path (no clone, no per-call sort): every `self` in a Python class lists each method's `self` in declaration order | same test, Python case |
| O4: filtered bucket path (`records_named`): `x.m()` lists both in-file `m` methods in declaration order | same test, Rust case |
| O4: unsorted input path (`Lookup::Found`, several bindings): a body `var` redeclaring a parameter lists the parameter, then the `var` | same test, JS case |
| O4: distinct candidates (first-wins dedup) | existing `occurrences_and_coverage_are_in_canonical_order`; the new test's strict `<` |
| O4: cache reader still accepts the order it writes | existing `symbols_cache` (`CANDIDATE_ORDER` cells), part of `just test` |
| O4: byte-identical public output | protocol (both scripts, Python corpus included) |

The new test sits in `lib/tests/l1/symbols_occurrences.rs`, already
declared by the `l1` target; no path segment starts with a tier marker. The
Input Robustness Matrix does not apply: no parser, manifest or configuration
reader changed.

### OS considerations

Nothing OS-specific changed: an in-memory sort and a slice borrow in the
resolver, with no paths, I/O or `cfg`. No rig cross-check was run; CI covers
the other OSes.

### Docs and skill

- `.claude/skills/tree-hugger/SKILL.md`: the cache-validation bullet now
  names `Resolver::candidate_key` as the source of candidate order (sorted
  once per bucket in `Resolver::new`; `name_only` keeps the first of
  adjacent equal IDs).
- `docs/references.md` unchanged: it states the contract ("in declaration
  order"), which still holds.
- No README or dependency change.

### Scope calculator

Run from the repository root after every Phase 7 edit (plan, log and spec
frontmatter included):

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

`"packages": ["tree-hugger", "tree-hugger-cli", "test-toolkit"]`,
`"change_class": "package"` (exit 0), so the gates were required and were
run. The working tree still holds Phase 6's uncommitted changes as well.
`test-toolkit` was not edited; the calculator lists it as a reader of these
files, as in Phases 4–6, and its gates were not run here.

### Checkpoint 7

- The O4 free part is in the working tree (uncommitted, per instructions):
  byte-identical output (protocol), green `just test` and `just lint`, and
  this log entry.
- Every other Phase 7 item is explicitly deferred above, because no author
  ruling is recorded. The spec's `human_review_items` carry the questions,
  with a lead item that explains the deferral.
- The plan is marked `implemented: true` and `completed_phase: 7`. The
  feature directory stays active for author review.
- Scratch: `/tmp/th-p7/` (`base/hug`, `new/hug`, `bin/protocol7.sh`,
  `bin/protocol2.sh`, `protocol2/`, `protocol7.log`, `scope.json`; the 388 MB
  protocol outputs and scratch caches were deleted).
