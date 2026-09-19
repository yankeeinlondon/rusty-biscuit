---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T22:20:52-07:00
spec: 2026-07-16-performance/spec.md
log: sniff/features/2026-07-16-performance/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-5.md
---

# Review 5

## Findings

### High: native Linux and Windows Level-1 execution remains absent

The completion boundary still requires the cross-platform tests to pass on macOS, Linux, and
Windows and the scheduled matrix to emit comparable work-count artifacts
([spec.md:396](spec.md#L396)). The feature's own current deferred record explicitly says that no
native Linux or Windows Level-1 suite and no three-OS work-count set were retained for the review-4
implementation ([deferred-perf-tests.md:92](deferred-perf-tests.md#L92),
[deferred-perf-tests.md:114](deferred-perf-tests.md#L114)). The two commits after that record change
aggregate Git observation and the service benchmark, but add no execution artifact.

This review passed the canonical Level-1 suite on macOS and cross-compiled `sniff-cli` for Windows
GNU. Cross-compilation does not exercise Windows path, process, Job Object, registry, or filesystem
behavior, and the exact `sniff` library all-target cross-check with `bench-internals` did not finish
within this non-interactive session's 60-second command limit. Retain a green native run for this
exact implementation on all three OSes, together with the three work-count artifacts. This is a
Level-1 verification gap and therefore remains high severity; Level 2 or Level 3 would not substitute
for native host behavior.

### High: a quiet session-detached Unix descendant survives timeout cleanup

The subprocess module promises process-tree termination and reaping so a child cannot outlive the
bounded operation ([process.rs:1](../../lib/src/process.rs#L1)). On Unix it can only signal the
original process group ([process.rs:201](../../lib/src/process.rs#L201)). After the direct child
exits or times out, the helper cancels its pipe-reader threads and returns even when a descendant has
escaped that group with `setsid()` ([process.rs:482](../../lib/src/process.rs#L482)). Closing the read
ends bounds the helper, but it does not terminate an escaped descendant that remains quiet or ignores
the resulting broken pipe.

The new Level-1 regression does not verify process-tree cleanup. Its detached fixture continuously
writes to both inherited pipes ([process.rs:619](../../lib/src/process.rs#L619)), so it exits when the
reader ends are dropped, and the assertion checks only that the direct child is no longer waitable
([process.rs:727](../../lib/src/process.rs#L727)). A detached descendant that simply sleeps retains
the pipes, survives the original group kill, and is reparented; the current `waitpid(-1)` assertion
cannot observe it. This matters now that the universal runner also supervises installers and remote
refresh: an escaped helper can continue changing external state after Sniff reports a timeout.

Add a portable Unix Level-1 fixture that reports the detached PID over a separate channel, remains
quiet after `setsid()`, and asserts that PID no longer exists after return. Either implement actual
descendant termination or narrow the documented contract and explicitly constrain which subprocesses
may daemonize. The current implementation satisfies the deadline/pipe-deadlock portion of R12 but not
its claimed process-tree cleanup boundary.

### High: aggregate reuse introduces an unapproved public Rust result contract

The specification preserves existing Git result shapes and limits accepted source-breaking additions
to inventory completeness and `GitRequest`'s listed fine-grained metadata controls
([spec.md:296](spec.md#L296), [spec.md:306](spec.md#L306)). The review-4 fix instead adds a public,
Serde-skipped `aggregate` field to the public `GitInfo` result and publicly re-exports
`GitAggregateEvidence` ([types.rs:1171](../../lib/src/filesystem/git/types.rs#L1171),
[mod.rs:26](../../lib/src/filesystem/git/mod.rs#L26)). Every downstream `GitInfo` struct literal must
now add that field even though it is CLI-private evidence. `#[doc(hidden)]` and `#[serde(skip)]`
preserve generated docs and JSON, but they do not preserve Rust source compatibility.

The request surface also exposes `GitMetadataRequest::aggregate`, which was not one of R9's named
controls and serializes when enabled ([request.rs:325](../../lib/src/request.rs#L325)). Its public
semantics are internally inconsistent: `GitMetadataRequest::all()` is documented as every metadata
observation but sets `aggregate: false` ([request.rs:356](../../lib/src/request.rs#L356)), and the
focused-control round-trip test never covers the new flag
([request.rs:982](../../lib/src/request.rs#L982)).

Keep CLI-only aggregate evidence behind a dedicated library projection/opaque return boundary, or
amend the specification through a reviewed compatibility decision and fully document and test the
new public request/result contract. The aggregate command's one-discovery/one-status-walk behavior
is now correct, but achieving it by silently expanding a preserved public result is not production
ready.

## Verification Levels

| Requirement | Strongest present verification | Review result |
|---|---|---|
| Bare aggregate schema, stdout/stderr split, one discovery/status/ref walk, and complete `--perf` accounting | Level 1 unit, spawned-CLI, snapshot, and work-counter tests on macOS | Appropriate tier and green; review-4's two aggregate findings are closed. |
| Service batching and 500/2,000-service synthetic benchmark fixture | Level 1 parser/chunk/counter tests plus feature-enabled benchmark compilation on macOS | Appropriate tier and green locally. |
| Subprocess deadlines, large pipes, and direct-child reaping | Level 1 process fixtures on macOS | Appropriate tier, but the detached test proves bounded return rather than termination of a quiet escaped descendant. |
| macOS/Linux/Windows output, path, process, and case behavior | Level 1 macOS execution; Windows GNU cross-compilation only | Insufficient: native Linux/Windows Level-1 runs and retained work-count artifacts are absent. |
| Terminal glyphs, widths, SGR styling, and scrolling | No changed presentation requirement | Level 2 is not required for this feature. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
sniff repo packages --json
just test
  sniff-lib: 1665 passed, 14 skipped
  sniff-cli: 779 passed, 3 skipped
cargo nextest run -p sniff --features remote \
  -E 'test(/a_session_detached_descendant_cannot_block_pipe_cleanup/)'
  1 passed
cargo nextest run -p sniff --features remote,bench-internals \
  -E 'test(/large_service_workloads_preserve_cardinality_and_chunk_bounds/)'
  1 passed
just lint
just build
cargo check -p sniff --benches --features remote,bench-internals
  passed
cargo check -p sniff-cli --all-targets --target x86_64-pc-windows-gnu
  passed with three target-gated warnings
cargo check -p sniff --all-targets --features remote,bench-internals \
  --target x86_64-pc-windows-gnu
  stopped at the 60-second non-interactive limit while checking sniff; no source error observed,
  but this is not recorded as a pass
```

The GitNexus index was 13 commits stale. Its refresh also exceeded the 60-second non-interactive
limit and was stopped, so stale graph results were not used as review evidence. Direct source,
commit, test, and work-counter inspection was used instead.

The review-4 aggregate accounting/reuse and synthetic service-benchmark findings are implemented and
green on macOS. The unresolved native-platform evidence, escaped-descendant lifecycle, and public
compatibility expansion keep the feature from meeting its completion boundary.

Production readiness: **not ready**.
