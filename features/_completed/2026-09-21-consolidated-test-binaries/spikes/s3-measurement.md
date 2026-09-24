---
kind: spike-record
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
status: complete
harness: spikes/s3-measure.sh
artifacts: spikes/s3/
---

# S3 — Measurement harness dry-run (current `claudine-cli`)

## What was run

```sh
features/2026-09-21-consolidated-test-binaries/spikes/s3-measure.sh \
  before-phase1 "$PWD/features/2026-09-21-consolidated-test-binaries/spikes/s3" 5
```

This ran from the repository root at revision `c0f911f5a`, with `claudine/cli`
clean. The script performs the R6 protocol end to end, in this order:

1. conditions record
2. Sniff/`uptime` snapshot
3. clean `cargo test --no-run --locked -p claudine-cli --features
   terminal-tests` into a fresh `mktemp` target directory under
   `/usr/bin/time -l`
4. target count and executable bytes from Cargo's JSON artifacts
5. one warm-up plus five timed edit-to-one-test trials (`touch` +
   `cargo nextest run … -E "<L1 filter>"
   handle_rejects_present_non_absolute_agent_cwd --no-tests=fail`)
6. closing snapshot

Raw outputs are in `spikes/s3/before-phase1-*`. The scratch target
directory (10.4 GB) was deleted after recording.

## Validation of the harness

- **Clean build:** it measures what it claims. The target dir was fresh
  (`mktemp`), `RUSTC_WRAPPER` was unset, and Cargo compiled the full
  dependency graph (280 s wall).
- **Target count** is 136, which equals the 139 declared targets minus the
  three `real-tests` targets excluded by the `terminal-tests` feature set. It
  also matches the spec's independent observation of 136 executables.
- **Executable bytes** are 2.78 GB (2,776,791,528 bytes, mean 20.4 MB). The
  spec observed 2.67 GB with a 19 MB mean in a long-lived worktree, which is
  consistent.
- **Edit loop:** `--no-tests=fail` proves the positional filter selected the
  test every trial. The edit file was located by content
  (`claudine/cli/tests/agent_cwd.rs`), so the Phase 3 after series touches
  `tests/l1/agent_cwd.rs` with **no change to the command**.
- **Peak RSS** comes from `/usr/bin/time -l`: 3.63 GB for the clean build,
  1.25 GB for each edit trial. That is the largest single descendant (a
  `rustc` or linker process), not the sum.

## Numbers (become the pilot's recorded before series)

| Measure | Value |
|---|---:|
| Clean test build, wall | 280.1 s |
| Clean build, peak RSS (largest process) | 3.63 GB |
| Produced test targets | 136 |
| Test executable bytes on disk | 2.78 GB |
| Fresh target dir total | 10.4 GB |
| Edit warm-up | 7.7 s |
| Edit trials 1–5 | 15.0, 14.2, 10.2, 9.1, 9.2 s |
| **Edit median / slowest** | **10.2 s / 15.0 s** |
| Edit peak RSS | 1.25 GB |

## Protocol deviation to carry forward

The host was **not idle**. Load averages were 10.8 at the start (1-minute) and
57.1 at the end, from other sessions on this shared machine. The trial spread
(9.1–15.0 s, while the warm-up was 7.7 s) shows it. Consequences:

- The after series must be compared with a before series taken **under
  matched load**. Phase 3 must re-run this script as the before side in a
  `git worktree` of the pre-migration revision, alternating with the after
  runs per spec §8. It should not reuse these numbers as the comparison
  baseline unless the host load during the after series is comparable.
- The guardrail arithmetic for reference: at a 10.2 s before median, the
  split condition (rise of more than 50% **and** more than 5 s) triggers only
  if the after median exceeds **15.3 s**.

## Command list for the after series (byte-for-byte the same)

```sh
# pre-migration worktree (before) and migrated tree (after), alternating:
features/2026-09-21-consolidated-test-binaries/spikes/s3-measure.sh before-phase3 <out> 5   # in the before worktree
features/2026-09-21-consolidated-test-binaries/spikes/s3-measure.sh after-phase3  <out> 5   # in the migrated tree
```

The script must not be edited between the two. Any needed change is a
protocol deviation, recorded in `measurements.md`.
