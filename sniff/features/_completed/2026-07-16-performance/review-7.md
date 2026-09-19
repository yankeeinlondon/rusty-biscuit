---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T07:59:03-07:00
spec: 2026-07-16-performance/spec.md
implemented: false
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-7.md
---

# Review 7

## Findings

### High: native Linux and Windows Level-1 execution and matched work-count artifacts are still absent

The completion boundary requires native tests to pass on macOS, Linux, and Windows and the scheduled
matrix to emit comparable work-count artifacts ([spec.md:396](spec.md#L396)). The retained cycle-6
record still says that Linux and Windows were not executed for the final implementation, that the
changed Unix descendant sampling has run only on macOS, and that the Windows Job Object path has only
been cross-compiled
([deferred-perf-tests.md:167](deferred-perf-tests.md#L167),
[deferred-perf-tests.md:189](deferred-perf-tests.md#L189)). The repository defines the right native
test and work-count matrices
([test.yml:56](../../../.github/workflows/test.yml#L56),
[sniff-performance.yml:118](../../../.github/workflows/sniff-performance.yml#L118)), but no retained
run for exact SHA `c32f78e43139868cf5831905e891c388d5fa3e74` is attached or publicly discoverable. A workflow
definition is future coverage, not execution evidence.

This review passed both canonical Level-1 components natively on macOS: 1,670/1,670 library tests
and 781/781 CLI tests. That does not exercise Linux `/proc` process discovery, Windows Job Objects and
registry probes, or native path/case behavior. Retain green `just test` and `just lint` runs for this
exact immutable implementation on all three OSes, plus the three per-OS `work_counts` artifacts.
Cross-compilation cannot close this Level-1 verification gap.

### Medium: the claimed-unreachable Unix process escape is reachable through installer call sites

The bounded runner now accurately documents that Unix containment is best-effort: a descendant can
fork, call `setsid()`, and be reparented entirely between the 250 ms samples
([process.rs:16](../../lib/src/process.rs#L16), [process.rs:29](../../lib/src/process.rs#L29)). The
implementation and architecture then justify the residual by saying no caller in the crate can reach
it ([process.rs:540](../../lib/src/process.rs#L540)). That is not an enforceable property of the
actual callers. The same runner executes package managers such as Brew, npm, pip, Cargo, and Go, plus
downloaded remote shell installers
([command.rs:265](../../lib/src/programs/install/command.rs#L265),
[command.rs:277](../../lib/src/programs/install/command.rs#L277),
[execute.rs:137](../../lib/src/programs/install/execute.rs#L137)). Those programs and scripts may run
third-party lifecycle/build code, so Sniff cannot assert that none forks and detaches during the
sampling gap. In that case Sniff can report an installation timeout while the escaped process keeps
modifying the host.

The new successful-parent-exit Level-1 regression is useful, but its fixture deliberately remains a
descendant for three sample intervals before exiting; it proves the sampled case, not the documented
residual ([process.rs:1012](../../lib/src/process.rs#L1012)). Either provide containment appropriate
for untrusted installer subprocess trees, keep installer execution outside the stronger supervision
claim and expose its best-effort timeout semantics to callers, or constrain installer methods so the
unreachable assertion is actually true. Add a Unix Level-1 fixture that detaches wholly between
samples and verifies the selected contract. Level 2 and Level 3 are not applicable to subprocess
lifecycle behavior.

## Verification Levels

| User-observable requirement | Strongest present verification | Review result |
|---|---|---|
| Bare aggregate JSON schema, JSON-only stdout, exit behavior, scope buckets, context, one Git discovery/status/ref walk, and no unrendered docs/formatting work | Level 1 spawned-CLI, snapshot, unit, and work-counter tests on macOS | Appropriate tier and green. Review 6's aggregate and instrumentation findings are closed. |
| Structure/focused request semantics, inventory truncation, bounded history/containment, remote snapshot reuse, sequential WAN fallback, default NTP policy, service batching, manifest reuse, and ownership | Level 1 unit/integration/work-counter tests on macOS | Appropriate tier and green locally; native Linux/Windows execution is missing. |
| Subprocess deadlines, concurrent pipe draining, direct-child reaping, timeout cleanup, and sampled `setsid()` descendants after successful parent exit | Level 1 process fixtures on macOS | Appropriate tier for the tested cases; the between-samples installer escape above is unverified and explicitly not contained. |
| macOS/Linux/Windows path, case, process, registry/Job Object, and work-count behavior | Native Level 1 on macOS; Windows GNU cross-compilation recorded by cycle 6 | Insufficient. Native Linux/Windows Level-1 runs and a matched three-OS artifact set are absent. |
| CLI glyphs, widths, SGR styling, and scrolling | No changed terminal-presentation requirement; existing L2 status-cell coverage predates these non-rendering changes | No new Level-2 requirement. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
sniff repo packages

just test
  initial cold build was stopped at the non-interactive limit
  warm rerun completed sniff-lib: 1,670 passed, 17 skipped, 1 leak-check retry

just _test sniff-cli
  sniff-cli: 781 passed, 3 skipped

just lint
  passed

just build
  sniff library build passed; the cold combined CLI build was stopped at the limit

cargo build -p sniff-cli --color=never
  passed

git diff --check
  passed before review-file/frontmatter edits
```

Review 6's unrequested aggregate walk, disabled-instrumentation clock reads, and sampled detached
descendant after successful parent exit are implemented and green on macOS. The missing native
platform evidence and the installer-reachable Unix containment gap keep the feature from meeting its
production completion boundary.

Production readiness: **not ready**.
