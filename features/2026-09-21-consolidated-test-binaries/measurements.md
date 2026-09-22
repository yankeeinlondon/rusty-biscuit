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

## After series (Phase 3)

_Not yet run._

## Seam decision (Phase 3 Wave 4)

_Not yet decided._
