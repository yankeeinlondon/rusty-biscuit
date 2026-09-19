# Run record — F2/F3/F21 terminal evidence (Phase 3)

Requirement-matched terminal evidence for Findings 2, 3, and 21. Interactive
(PTY) and piped (redirected CLI) measurements are recorded as **separate cases**
per the evidence contract.

## Commits & scope

- Feature head at measurement: `b425fb466d060a0b7f176be4d5bfb52bd577f957`.
- These are **evidence-only** measurements of already-landed behavior (Findings
  2/3/21 were implemented before this follow-up). There is no baseline/candidate
  code pair; the pass/fail gate is the behavioral proof (one OSC 10 request; one
  terminal detection; no `defaults` fork), not a speed delta. The latency
  numbers document the post-cache cost and bound any future regression.

## Host & environment

- Host: macOS 26.5.2 (build 25F84), Darwin 25.5.0, arm64 (Apple Silicon).
- Binary: `md 0.1.0`, `cargo build --release -p darkmatter-cli` (release profile).
- Probe: `cargo build -p biscuit-terminal --example discovery_probe` (dev profile;
  the PTY probe is a construction-cost micro-measurement, not a release gate).
- Environment: `NO_COLOR=1` for the piped hyperfine case; `DARK_MODE=1`,
  `PROBE_TERM_PROGRAM=WezTerm` for the PTY probe. No CI env vars set.

## Case 1 — Interactive (PTY) repeated Terminal-construction latency

- **TTY mode:** real pseudoterminal (`expectrl`), `is_tty() == true`.
- **Harness:** `biscuit-terminal` `level2_terminal_repeated_construction_latency`
  drives `discovery_probe PROBE=terminal_latency`. The probe answers the first
  OSC 10 / OSC 11 request, warms up to absorb the one-time cold round-trip, then
  times cached repeated constructions.
- **Warm-up:** 3 constructions (untimed). **Samples:** 50 timed constructions.
- **Statistic / dispersion:** median + stddev over 50 samples.
- **Raw samples:** `interactive-pty-latency.txt` (`terminal_latency_raw_ns`).
- **Result:** median **0.970 ms**, mean 0.975 ms, min 0.953 ms, max 1.070 ms,
  stddev 0.022 ms.
- **Interpretation:** repeated construction costs ~1 ms and re-pays **no** tty
  round-trip. A dropped `TEXT_COLOR_CACHE` would re-query `/dev/tty` on every
  construction (two 100 ms timeouts absent a reply, or a full round-trip with
  one), i.e. two orders of magnitude slower — the evidence that the cache holds.

## Case 2 — Piped (redirected CLI) compose invocation latency

- **TTY mode:** fully redirected (hyperfine attaches stdout/stderr to
  `/dev/null`), `is_tty() == false`.
- **Command:** `md compose <benchmarks/fixtures/compose_trivial.md> -vv --perf`
  (verbose summary + `-vv` perf metrics; the `--perf` footer also renders).
- **Runner:** `hyperfine --warmup 5 --runs 40 --shell=none`, `NO_COLOR=1`.
- **Statistic / dispersion:** mean ± stddev over 40 runs.
- **Raw samples:** `piped-compose-vv-perf.json` (hyperfine per-run times).
- **Result:** mean **13.7 ms** ± 0.5 ms (min 13.1 ms, max 15.1 ms).
- **Detection count:** the L1 test
  `darkmatter-cli compose_verbose_perf_performs_single_terminal_detection`
  proves this invocation performs exactly **one** terminal detection across the
  verbose + perf + warning branches (counted via the
  `biscuit_terminal::terminal` "Terminal detected" span, not inferred from equal
  output).

## Thresholds (declared before capture)

- **Behavioral gates (hard):** exactly one OSC 10 request across ≥2 PTY
  constructions; every construction reports the same cached response; exactly one
  terminal detection per `md compose -vv --perf`; zero `defaults` forks on
  redirected macOS output.
- **Latency sanity (loose):** PTY repeated-construction median < 50 ms (catches a
  cache regression that would re-pay the tty round-trip); piped compose within
  normal command-startup noise. Both met with wide margin.

## Cross-platform

- The PTY OSC path is **Unix-only** and target-gated; Windows compiles the
  level2 binary and records a clean skip. Linux provides the required real
  non-macOS L2 run (deferred to the Phase 11 cross-platform closeout on this
  macOS-only host).
- The macOS `defaults` no-fork guard is macOS-specific; the L1 test is
  `#[cfg(target_os = "macos")]` with a clean skip stub elsewhere.
