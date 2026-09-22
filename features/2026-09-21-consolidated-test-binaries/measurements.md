---
kind: measurements
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
harness: spikes/s3-measure.sh
---

# Pilot measurements — `claudine-cli`

## Protocol (R6)

The spec §8 matched conditions, fixed here so the Phase 3 after series is
the same recipe. `spikes/s3-measure.sh` applies them. Its
`<series>-conditions.txt` output records the realized values per series.

| Condition | Value |
|---|---|
| Toolchain | `rust-toolchain.toml` pin: rustc 1.98.1 |
| Host triple | `aarch64-apple-darwin` (Apple M4 Max, 16 cores, 128 GiB) |
| Cargo profile | `test` (dev, unoptimized + debuginfo) |
| Feature set | `terminal-tests`. It builds 136 of the 139 targets (all but the three `real-tests` targets) and keeps the bundled DuckDB build (`daemon-tests`) out, so C++ compile time does not mask link time. The same set is used in both series. |
| Linker | platform default (Apple `ld` via `cc`); no `.cargo/config.toml` linker override exists |
| Worker count | Cargo default (`hw.ncpu` = 16) |
| Compiler wrapper | `RUSTC_WRAPPER` unset (kache off) |
| Clean build | `cargo test --no-run --locked -p claudine-cli --features terminal-tests` into a **fresh** `mktemp` target dir per series |
| Peak compiler memory | `/usr/bin/time -l` max RSS of the largest waited-for descendant |
| Target count / size | `compiler-artifact` messages of kind `test` for `claudine-cli`; executable byte sum |
| Edit loop | `touch` the file that defines `handle_rejects_present_non_absolute_agent_cwd` (located by content, so it follows the move), then `cargo nextest run -p claudine-cli --features terminal-tests -E "$(just _tier_filter L1 claudine-cli)" handle_rejects_present_non_absolute_agent_cwd`. The command is identical before and after, because the positional filter matches the test name either way. One warm-up, then five timed trials. Report the median and the slowest. |
| Host/load | `uptime` load averages plus `sniff cpu --json` / `sniff memory --json` at the start and end of each series |
| Guardrail | split into stable subject groups only if the median edit latency rises by **both** more than 50% **and** more than 5 s, or the consolidated target cannot compile reliably or exceeds runner memory |

**Alternation.** Spec §8 asks for five *alternating* before/after edit trials.
Alternation needs both trees at once, so the Phase 3 after series must run
from the migrated tree while a `git worktree` of the pre-migration revision
(recorded below) runs the before trials between them, both warmed first.
The Phase 1 before series below is the protocol dry-run and the reference
point. It is not a substitute for the alternating trials.

**Harness gap for alternation (must be closed in Phase 3).** As written,
`s3-measure.sh` runs its trials in one block after its own clean build.
Interleaving before/after trials needs an edit-only entry point that reuses
an existing, already-warm target dir and runs one trial per call. It should
not do a clean build per trial. Adding that mode is allowed only if the
measured `touch` + `cargo nextest run …` command stays identical. Record the
addition as a protocol amendment here.

## Before series (Phase 1 dry-run, unmigrated tree)

Source: `spikes/s3/before-phase1-*`. Write-up: `spikes/s3-measurement.md`.

| Field | Value |
|---|---|
| Revision | `c0f911f5a` (`claudine/cli` clean) |
| Toolchain / nextest | rustc 1.98.1 / cargo-nextest 0.9.136 |
| Host | `aarch64-apple-darwin`, Apple M4 Max, 16 cores, 128 GiB |
| Features / profile / jobs | `terminal-tests` / test (dev) / 16 |
| Wrapper | `RUSTC_WRAPPER` unset |
| Load (1-min avg) at start / end | 10.8 / 57.1 (**not idle**: shared host) |

| Measure | Before |
|---|---:|
| Clean test build, wall | 280.1 s |
| Clean build, peak RSS | 3.63 GB |
| Produced test targets | 136 |
| Test executable bytes | 2.78 GB (mean 20.4 MB) |
| Edit-to-one-test, warm-up | 7.7 s |
| Edit-to-one-test, trials | 15.0, 14.2, 10.2, 9.1, 9.2 s |
| **Edit-to-one-test, median / slowest** | **10.2 s / 15.0 s** |
| Edit loop, peak RSS | 1.25 GB |

The guardrail at this median: split only if the after median exceeds
**15.3 s** (more than 50% **and** more than 5 s above 10.2 s). Because the host
was loaded, the Phase 3 decision must use the alternating, load-matched pair,
not this row alone.

**Correction (Phase 3): the Phase 1 series above ran with kache on.** The
dev host's `~/.cargo/config.toml` sets `rustc-wrapper = "kache"`, and its
`PATH` carries kache `cc`/`gcc`/`clang` shims. `s3-measure.sh` only
`unset RUSTC_WRAPPER`, which does not override a config value, so
`rustc_wrapper=unset` in its conditions file was not true. kache also turns
off incremental compilation, which is why the Phase 1 edit loop took about
10 s. That series is kept as the record of the dry-run, not as a before
number. The Phase 3 pair below supersedes it.

## Protocol amendments (Phase 3)

1. **kache really off.** `s3-measure.sh` exports an explicitly empty
   `RUSTC_WRAPPER=""`, which overrides the config, and removes
   `…/kache/shims` from `PATH`. The conditions file records
   `rustc_wrapper=empty (disabled)` and `cc=/usr/bin/cc`. Found when a first
   Phase 3 attempt showed a 33.8 s "clean" after build following a 114 s before
   build: the second build restored the first one's dependencies. That attempt
   is discarded.
2. **Alternation.** `S3_MODE=build` does the clean build and target
   accounting into a named fresh `S3_TARGET_DIR`. `S3_MODE=edit
   S3_TRIAL=<n|warmup>` runs one edit trial against it. The measured command
   (`touch <file>` then `cargo nextest run --locked -p claudine-cli --features
   terminal-tests --target-dir <dir> -E <L1 filter> --no-tests=fail
   handle_rejects_present_non_absolute_agent_cwd`) is byte-identical in every
   mode and in both trees. The before tree is a `git worktree` of `9621882ae`
   (`ctb-before-measure`). The after tree is the migrated working tree on top
   of the same revision. Order: before clean build, after clean build, before
   warm-up, after warm-up, then trials 1–5 alternating before/after.

## After series (Phase 3, alternating pair)

Source: `pilot/measure/{before,after}-p3-*`. 2026-09-22 04:51–04:56 UTC.

| Field | Value |
|---|---|
| Before revision | `9621882ae` (`claudine/cli` clean) |
| After revision | `9621882ae` + the Phase 3 working tree (uncommitted) |
| Toolchain / nextest | rustc 1.98.1 / cargo-nextest 0.9.136 |
| Host | `aarch64-apple-darwin`, Apple M4 Max, 16 cores, 128 GiB |
| Features / profile / jobs | `terminal-tests` / test (dev) / 16 |
| Wrapper | `RUSTC_WRAPPER=""` (kache off), `cc` = `/usr/bin/cc` |
| Load (1-min avg), before build start / after build start / end | 6.1 / 48.2 / 13.8. The after build started as the before build finished, and other sessions shared the host, so the clean-build comparison leans against the after side if anything. The edit trials alternate, so they are load-matched. |

| Measure | Before | After | Change |
|---|---:|---:|---:|
| Clean test build, wall | 125.4 s | 99.9 s | −20% |
| Clean build, peak RSS | 3.74 GB | 3.76 GB | +0.3% |
| Produced test targets (`terminal-tests`) | 136 | 3 (`l1`, `level2`, `level3`) | −133 |
| Test executable bytes | 2.80 GB | 0.23 GB | −92% |
| Target dir bytes after the clean build | 12.69 GB | 7.41 GB | −42% |
| Edit-to-one-test, warm-up | 3.92 s | 2.76 s | |
| Edit-to-one-test, trials 1–5 | 2.42, 2.45, 2.41, 2.38, 2.41 s | 2.71, 2.77, 2.71, 2.72, 2.69 s | |
| **Edit-to-one-test, median / slowest** | **2.41 s / 2.45 s** | **2.71 s / 2.77 s** | **+0.30 s (+12%)** |
| Edit loop, peak RSS | 1.31 GB | 1.78 GB | +36% |

The edit loop now recompiles the whole 102-module `l1` crate incrementally
instead of one 4-test crate. That costs 0.3 s and about 0.47 GB of compiler
memory. The clean build is faster because 133 fewer executables are linked.

## Seam decision (Phase 3 Wave 4)

**Keep one binary per execution contract** (the spec §8 default). Against the
guardrail:

- Median edit-to-one-test latency rose by 0.30 s (+12%). The split threshold
  is **both** more than 50% **and** more than 5 s. Neither part is met.
- The consolidated targets compiled on every attempt: the clean build, the
  five macOS feature-set captures (twice), the Linux captures for all five
  feature sets, and the `x86_64-pc-windows-gnu` cross-compile. Peak compiler
  memory is unchanged (3.74 → 3.76 GB), about 54% of the tightest hosted
  runner, `macos-latest` at 7 GB (the `os` skill's `ci-runners.md`). The
  consolidated crates add no memory peak over what the per-file build
  already reached. The edit loop's 1.78 GB is a developer-host figure.

No subject-group split is needed. Waves 1–3 do not re-run.

## Rollout observations (Phases 4–6)

_Not yet recorded._
