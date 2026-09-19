---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T05:17:07-07:00
spec: 2026-07-16-performance/spec.md
implemented: true
implemented_by: claude/default
log: sniff/features/2026-07-16-performance/log.md
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-6.md
---

# Review 6

## Findings

### High: native Linux and Windows Level-1 execution remains absent

The completion boundary still requires the cross-platform tests to pass on macOS, Linux, and
Windows and the scheduled matrix to emit comparable work-count artifacts
([spec.md:396](spec.md#L396)). The feature's deferred record still states that no native Linux or
Windows Level-1 run and no matched three-OS work-count artifact set exist for the reviewed
implementation ([deferred-perf-tests.md:129](deferred-perf-tests.md#L129)). The two implementation
commits after review 5 do not add retained execution evidence.

This review passed the canonical Level-1 suites on macOS and cross-compiled all sniff library and
CLI targets for Windows GNU. The Linux cross-check stopped in `aws-lc-sys` before sniff code because
the host has no `x86_64-linux-gnu-gcc`. Neither cross-compilation nor a workflow definition executes
Windows Job Objects, Linux `/proc` process discovery, native path/case behavior, or the filesystem
work-count fixtures. GitHub run history was also unavailable because this non-interactive host has
no GitHub CLI credentials.

Retain green native `just test` and `just lint` runs for the exact final implementation on macOS,
Linux, and Windows, together with all three work-count artifacts under that same identifier. This
remains a Level-1 verification gap; Level 2 or Level 3 would not substitute for native host
behavior.

### High: bare aggregate JSON performs unrequested Markdown and formatting observation

The new dedicated aggregate boundary starts from `FilesystemRequest::new()` and disables only file
inventory ([aggregate_view.rs:125](../../lib/src/filesystem/repo/aggregate_view.rs#L125)). That
constructor enables both formatting and docs by default
([request.rs:858](../../lib/src/request.rs#L858)). Docs are therefore treated as a repository-wide
walk consumer ([mod.rs:84](../../lib/src/filesystem/mod.rs#L84)), and the shared walk also collects
and inspects manifests merely because docs are enabled
([mod.rs:278](../../lib/src/filesystem/mod.rs#L278)). Bare `sniff repo --json` renders Git-derived
`documentation_changes`, but it does not render the filesystem Markdown inventory or formatting
result.

The command's own counters demonstrate the scale on this worktree: `sniff --base sniff/lib --perf
repo --json` started one shared walk, visited 10,925 entries, and parsed 6,972 Markdown documents.
That is work outside the aggregate output contract, contrary to the feature goal that observation
run no more often than output requires ([spec.md:57](spec.md#L57)) and that focused requests remain
shallow ([spec.md:58](spec.md#L58)). On a documentation-heavy monorepo this can dominate the command
the feature is specifically optimizing.

Add `.without_docs().without_formatting()` to the aggregate request and a Level-1 command-boundary
work-count fixture containing Markdown and `.editorconfig` files. Assert zero
`filesystem.docs.documents_parsed` and that those unrendered projections remain absent. The current
aggregate accounting test asserts Git discovery/status/ref bounds but contains no Markdown and does
not assert either request flag ([commands/mod.rs:2230](../../cli/src/commands/mod.rs#L2230)), so it
cannot catch this regression.

### High: Unix descendant termination loses escaped children after direct-parent exit

The shared subprocess boundary is documented as owning process-tree termination and reaping
([sniff-library-architecture.md:345](../../docs/sniff-library-architecture.md#L345)). The review-5
fix snapshots descendants from the direct child's PID immediately before signaling them, then kills
the original process group ([process.rs:202](../../lib/src/process.rs#L202)). This closes the tested
timeout case only while the direct parent remains alive.

When a child spawns a descendant that calls `setsid()` and then exits successfully, the descendant
can be reparented before the post-`try_wait` cleanup at
[process.rs:526](../../lib/src/process.rs#L526). At that point its `parent()` no longer traces to the
stored child PID, and it is outside the original process group, so neither half of `terminate()`
addresses it. The same snapshot-then-signal design also leaves a fork/`setsid()` race between the
snapshot and group kill. A quiet helper may therefore continue changing external state after sniff
returns even though the API and architecture claim tree termination.

The new regression keeps its direct parent asleep until the timeout
([process.rs:685](../../lib/src/process.rs#L685)), and the older successful-parent-exit test covers
only a descendant that remains in the inherited group
([process.rs:787](../../lib/src/process.rs#L787)). Add a portable Unix Level-1 fixture that reports a
detached PID, lets the direct parent exit successfully, and proves the reported PID is gone after
the helper returns. Either retain descendant identity while the parent is live and close the race,
or explicitly narrow the contract and prevent commands capable of daemonizing from using the
stronger guarantee.

### Medium: aggregate timing instrumentation reads clocks when collection is inactive

The aggregate boundary unconditionally calls `Instant::now()` twice before invoking detection
([aggregate_view.rs:125](../../lib/src/filesystem/repo/aggregate_view.rs#L125)). The performance
instrumentation contract requires checking collection before setting up instrumentation and using
the gated stage timer so ordinary calls do not read a clock. This is small compared with the
unrequested document walk, but it is directly on the new public aggregate path and weakens the
feature's zero-overhead-when-disabled accounting design.

Use `StageTimer::start` (or an equivalent collection guard) for both stages and add the aggregate
entry point to the existing opt-in instrumentation test.

## Verification Levels

| Requirement | Strongest present verification | Review result |
|---|---|---|
| Bare aggregate schema, JSON-only stdout, one Git discovery/status/ref walk, branch/worktree/history reuse | Level 1 unit, spawned-CLI, snapshot, and work-counter tests on macOS | Appropriate tier and green, but the work-count fixture does not verify that unrendered docs/formatting are skipped. |
| Subprocess deadlines, large concurrent pipe drains, direct-child reaping, and timeout cleanup of a pre-existing detached descendant | Level 1 process fixtures on macOS | Appropriate tier, but successful-parent-exit containment and the snapshot/fork race are unverified and incomplete. |
| macOS/Linux/Windows output, path, process, registry/Job Object, case, and work-count behavior | Native Level 1 on macOS; Windows GNU cross-compilation only | Insufficient: native Linux/Windows Level-1 runs and retained three-OS work-count artifacts are absent. |
| CLI text/plain presentation, terminal glyphs, widths, SGR styling, and scrolling | Existing Level 1 output parity; no changed presentation requirement | Level 2 is not required for this feature. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
sniff repo packages

just test
  sniff-lib: 1667 passed, 16 skipped
  the combined recipe was interrupted after the library suite to honor the non-interactive
  command limit; the CLI tier was then run directly with the canonical Level-1 filter

cargo nextest run -p sniff-cli -E '<canonical Level-1 filter>'
  sniff-cli: 780 passed, 3 skipped (3 slow, 1 flaky)

cargo nextest run -p sniff --features remote \
  -E 'test(/a_quiet_session_detached_descendant_is_terminated/) | \
      test(/a_session_detached_descendant_cannot_block_pipe_cleanup/)'
  2 passed

just lint
just build

sniff --base sniff/lib --perf repo --json
  filesystem.walk.walks_started: 1
  filesystem.walk.entries_visited: 10925
  filesystem.docs.documents_parsed: 6972

cargo check -p sniff -p sniff-cli --all-targets --features sniff/remote \
  --target x86_64-pc-windows-gnu
  passed with target-gated warnings

cargo check -p sniff -p sniff-cli --all-targets --features sniff/remote \
  --target x86_64-unknown-linux-gnu
  failed before sniff code: x86_64-linux-gnu-gcc is not installed for aws-lc-sys
```

GitNexus was 16 commits stale. Its required refresh exceeded the non-interactive command limit and
was aborted, so stale graph results were not used as review evidence. Direct source, current tests,
CLI counters, and cross-target compilation were used instead.

Review 5's public `GitInfo`/`GitMetadataRequest` compatibility finding is closed by the dedicated
aggregate boundary, and its deterministic timeout fixture now passes on macOS. The remaining native
platform evidence gap, unrequested aggregate filesystem work, and Unix descendant-lifecycle hole
keep the feature from meeting its completion boundary.

Production readiness: **not ready**.
