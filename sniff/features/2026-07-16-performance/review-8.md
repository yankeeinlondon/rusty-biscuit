---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T08:27:46-07:00
spec: 2026-07-16-performance/spec.md
log: sniff/features/2026-07-16-performance/log.md
implemented: true
implemented_by: claude/default
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-8.md
---

# Review 8

## Findings

### High: native Linux and Windows Level-1 execution and matched work-count artifacts remain absent

The completion boundary still requires tests to pass on macOS, Linux, and Windows and the scheduled
matrix to emit comparable per-OS work-count artifacts ([spec.md:396](spec.md#L396)). HEAD is now
`98fa4d992adf13a85c31995b3feb3d270a9a26d1`, but no remote branch contains that commit. Therefore
the repository's native three-OS test matrix and work-count matrix cannot have run for this exact
implementation. The cycle-7 deferral explicitly records that the Unix escape fixture has run only on
macOS and must run on Linux, where descendant discovery uses `/proc`
([deferred-perf-tests.md:236](deferred-perf-tests.md#L236)).

This review passed 1,671/1,671 library tests and 781/781 CLI tests natively on macOS. The repository
defines the correct macOS/Linux/Windows jobs
([test.yml:56](../../../.github/workflows/test.yml#L56),
[sniff-performance.yml:118](../../../.github/workflows/sniff-performance.yml#L118)), but workflow
definitions and Windows cross-compilation are not native execution evidence. Publish one immutable
implementation identifier, retain green native test/lint runs on all three OSes, and retain the three
work-count tables under that same identifier.

### High: the installer timeout disclosure breaks the public Rust contract but is discarded by the main callers

Cycle 7 adds the required public field `InstallCapturedResult::timed_out`
([options.rs:93](../../lib/src/programs/install/options.rs#L93)). That is a source break for every
downstream struct literal, while the specification approves source breaks only for inventory
completeness and `GitRequest` and requires separate review for any additional contract change
([spec.md:306](spec.md#L306)). No migration or separately approved contract is recorded.

The new field also does not reach the primary caller-facing surfaces. `execute_install` and
`execute_versioned_install` discard it and return the same generic `PackageManagerFailed` error used
for an ordinary non-zero exit ([execute.rs:47](../../lib/src/programs/install/execute.rs#L47)). The
interactive interview likewise drops `timed_out`, renders the ordinary failure body, and returns the
same `Failed` outcome ([interview.rs:236](../../lib/src/programs/install/interview.rs#L236)). Thus a
CLI user is still not told that a detached installer descendant may be modifying the host after the
timeout. The low-level captured API documents the hazard, but the public field imposes compatibility
cost without delivering the warning through the flows that consume it.

Choose and review one public timeout contract, then propagate it end to end: a distinct error/outcome
for the legacy APIs, a semantic interview event/outcome, and an explicit terminal warning before any
retry. Add Level-1 injected-runner and interview/PTY tests. The existing timeout helper does not even
assert that `result.timed_out` is true ([execute.rs:457](../../lib/src/programs/install/execute.rs#L457)).
Level 2 and Level 3 are not required unless a separate requirement is added for real-terminal styling
or OS input.

### Medium: the between-samples Unix regression can report a green test without testing the residual

The new Level-1 fixture is the correct tier and did execute its core assertion on this macOS review
run. However, when timing crosses a sample boundary under load, the test prints that the residual
assertion was skipped and returns successfully ([process.rs:1009](../../lib/src/process.rs#L1009)).
Nextest records that path as `PASS`, not as a skip or inconclusive result. A loaded Linux CI runner
can therefore provide the appearance of native coverage without verifying the contract the test was
added to pin.

Make the sampler observable/injectable in tests so the fork can be synchronized deterministically
between two samples. If that is not practical, fail after bounded retries rather than converting a
no-verdict run into success. Keep the RAII cleanup guard so every attempt reaps the manufactured
escapee.

### Medium: completion records still contradict the implemented and reviewed state

The cycle-7 implementation log stops immediately after “starting the work” and has no actions,
verification, deferral, or completion record ([log.md:12](log.md#L12)), even though cycle-7 source
landed in HEAD. Phase 8 also still calls the synthetic large-service Criterion workload deferred
([phases/_completed/08-cross-platform-validation/spec.md:288](phases/_completed/08-cross-platform-validation/spec.md#L288)),
while the maintained deferral record says review cycle 4 resolved it with 500- and 2,000-service
workloads ([deferred-perf-tests.md:73](deferred-perf-tests.md#L73)). These contradictions make the
performance evidence difficult to audit and violate the specification's documentation-maintenance
expectation.

Complete the cycle-7 log with the exact `98fa4d992` result and reconcile Phase 8 with the resolved
service benchmark and the still-open native-platform evidence. Historical measurements can remain,
but current-state claims must not conflict with the source or the deferral registry.

## Verification Levels

| User-observable requirement | Strongest present verification | Review result |
|---|---|---|
| Aggregate JSON schema/stdout/exit behavior; structure/focused semantics; inventory bounds; Git, remote, WAN, NTP, service, manifest, and ownership work bounds | Level 1 unit, integration, spawned-CLI, snapshot, and work-counter tests on macOS | Appropriate tier and green locally. Native Linux/Windows execution remains missing. |
| Installation timeout and the warning that a Unix descendant may survive | Level 1 process fixture and internal installer timeout tests on macOS | Gap: the timeout flag is not asserted and the legacy/interview/CLI callers discard it, so no user-facing Level-1 verification exists. |
| Between-samples `setsid()` escape | Level 1 Unix subprocess fixture on macOS | Appropriate tier, and the assertion ran locally; the test can silently return a green no-verdict under load. Native Linux execution is absent. |
| Windows Job Object, registry, native path/case behavior, and Linux `/proc` descendant discovery | Native Level 1 on macOS only; Windows GNU cross-compilation is recorded | Insufficient. Native Linux and Windows L1 plus matched work-count artifacts are required. |
| CLI glyphs, widths, SGR styling, and scrolling | No changed terminal-presentation requirement | No new Level-2 requirement. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
sniff repo packages --json
  succeeded

just test / just _test sniff-cli
  sniff-lib: 1,671 passed, 19 skipped
  sniff-cli: 781 passed, 3 skipped

cargo nextest run -p sniff --features remote
  focused between-samples escape test passed and did not take its no-verdict path

just lint
  passed

just build
  passed

git branch -r --contains 98fa4d992adf13a85c31995b3feb3d270a9a26d1
  no remote branch contains the reviewed commit

git diff --check
  passed before review-file/frontmatter edits
```

The structural performance implementation has strong macOS Level-1 evidence, and cycle 7 now
documents the Unix containment residual honestly. The missing native platform evidence, incomplete
timeout propagation, unreviewed public source break, and non-deterministic residual test keep the
feature from meeting its production completion boundary.

Production readiness: **not ready**.
