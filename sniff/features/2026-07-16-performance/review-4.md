---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T16:57:48-07:00
spec: 2026-07-16-performance/spec.md
log: sniff/features/2026-07-16-performance/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-16-performance/spec.md`"
feature: 2026-07-16-performance/review-4.md
---

# Review 4

## Findings

### High: bare aggregate execution still rediscovers and independently queries the repository

R2 requires repository discovery to be shared across aggregate-local facts and explicitly removes
the aggregate path's independent identity, branch, and worktree queries
([spec.md:128](spec.md#L128), [spec.md:130](spec.md#L130)). The command first runs the filesystem
detection plan, then calls `observe_repo_aggregate` afterward
([commands/mod.rs:1671](../../cli/src/commands/mod.rs#L1671)). That second call performs another
`GitRepo::discover`, then independently resolves identity, branches, worktrees, and current-worktree
state ([aggregate_view.rs:131](../../lib/src/filesystem/repo/aggregate_view.rs#L131),
[aggregate_view.rs:159](../../lib/src/filesystem/repo/aggregate_view.rs#L159)). Detection is even
configured to omit branches and worktrees because the later observation supplies them separately
([commands/mod.rs:2132](../../cli/src/commands/mod.rs#L2132)). The result is two repository
discoveries for one bare `repo --json` command and the exact post-detection queries R2.6 says to
remove.

The Level-1 work-count tests do not enforce the command-wide contract. They say the two arms are
summed, but assert one discovery in the detection arm and another discovery in the observation arm
([aggregate_view.rs:512](../../lib/src/filesystem/repo/aggregate_view.rs#L512),
[aggregate_view.rs:616](../../lib/src/filesystem/repo/aggregate_view.rs#L616),
[aggregate_view.rs:642](../../lib/src/filesystem/repo/aggregate_view.rs#L642)). Their premise that
filesystem stage-thread counters are unavailable is stale: the planner now propagates collectors,
and `git_stage_counters_survive_the_scoped_thread` explicitly proves it
([filesystem/mod.rs:826](../../lib/src/filesystem/mod.rs#L826)). Produce `RepoAggregate` during the
original library detection using its existing `GitRepo`/ref snapshot, or extend `GitInfo` with the
needed projection shapes. Add one Level-1 command-path counter test that runs detection and aggregate
assembly together and asserts one discovery total, one status walk total, and no independent branch
or worktree observation.

### High: `repo --json --perf` omits most aggregate work from its report

R1 makes reliable work accounting acceptance evidence, but `detect_with_plan` snapshots its
collector before returning ([lib.rs:368](../../lib/src/lib.rs#L368)). The CLI then performs aggregate
observation and JSON assembly after that snapshot and emits the stale `result.performance`
([commands/mod.rs:1677](../../cli/src/commands/mod.rs#L1677)). `CliPerf::emit` prefers that supplied
snapshot over its own end-to-end elapsed report ([perf.rs:56](../../cli/src/perf.rs#L56)), so neither
the later elapsed time nor its counters reach the user.

On the current workspace binary, the command reported `Total: 305.95 ms`, one repository discovery,
and one status walk while the same invocation took 0.95 seconds wall-clock. Source inspection shows
the omitted second discovery and branch/worktree/history work described above. This makes the CLI's
performance output materially misleading and prevents the real aggregate path from serving as R1/R2
regression evidence. Keep the collector alive through aggregate observation and projection, or move
that observation inside the library request before its snapshot. Add a Level-1 spawned-CLI test that
asserts the emitted report covers the complete command and includes the command-wide counter bounds.

### High: Unix process-group cleanup does not guarantee a bounded process tree

R12 requires a child to time out without pipe deadlock. The new Unix implementation creates a
process group and signals that group, then unconditionally joins the pipe-reader threads
([process.rs:77](../../lib/src/process.rs#L77), [process.rs:351](../../lib/src/process.rs#L351)). A
grandchild can call `setsid` or move to another process group while retaining inherited stdout or
stderr. It then survives the group signal, `read_to_end` never receives EOF, and the unbounded joins
can still hold detection past its advertised deadline. A process group is not a process-tree
containment primitive.

The Level-1 regression only spawns a sleeping descendant that remains in the original group
([process.rs:424](../../lib/src/process.rs#L424), [process.rs:491](../../lib/src/process.rs#L491)), so
it cannot exercise this escape. Add a Unix fixture whose descendant starts a new session while
retaining both pipes and enforce a bounded cleanup design for that case. Windows Job Objects provide
the stronger containment model, but native Windows execution evidence is still absent as noted
below. Levels 2 and 3 are not applicable to subprocess supervision.

### Medium: the documented universal subprocess boundary still has production bypasses

The process module, architecture guide, and Sniff skill now state that every child goes through
`run_with_timeout` ([process.rs:1](../../lib/src/process.rs#L1),
[sniff-library-architecture.md:345](../../docs/sniff-library-architecture.md#L345)). Production code
still invokes `Command::output` directly for installation and the uv bootstrap
([execute.rs:100](../../lib/src/programs/install/execute.rs#L100),
[execute.rs:153](../../lib/src/programs/install/execute.rs#L153),
[execute.rs:319](../../lib/src/programs/install/execute.rs#L319)), and explicit remote refresh uses an
unbounded `git fetch` subprocess ([remote_refresh.rs:635](../../lib/src/filesystem/git/remote_refresh.rs#L635)).
The installation bypass is also a functional API defect: `InstallOptions::timeout_secs` is public and
documented, but none of the execution paths reads it
([options.rs:11](../../lib/src/programs/install/options.rs#L11)).

Either narrow the universal documentation to the detector probes actually covered by R12 or route
these paths through a builder-capable bounded runner that preserves cwd/environment configuration and
uses the requested installation timeout. The latter is preferable because it makes the existing
public timeout contract real. Add Level-1 short-timeout fixtures for ordinary, versioned, uv, and
refresh execution paths; no terminal-emulator or keyboard tier is involved.

### High: native Linux and Windows Level-1 completion remains unverified

The acceptance boundary requires cross-platform tests to pass on macOS, Linux, and Windows
([spec.md:396](spec.md#L396)). The canonical macOS suite is now green and the Windows GNU target
compiles, but the feature's own deferred record says no native Linux or Windows Level-1 binaries were
run and that the workflow definition is not execution evidence
([deferred-perf-tests.md:56](deferred-perf-tests.md#L56)). That is an honest record, but it leaves the
acceptance criterion open. Retain a green `sniff-cross-platform` run for this exact implementation and
the three per-OS work-count artifacts before production readiness. This is Level 1 on each OS; Levels
2 and 3 do not substitute for native host/path/process behavior.

### Medium: the required synthetic service benchmark is still absent

The specification requires a large synthetic service-listing Criterion workload
([spec.md:362](spec.md#L362)). The benchmark catalog explicitly says it is not implemented
([README.md:193](../../lib/benches/README.md#L193)), and the deferred record leaves it open
([deferred-perf-tests.md:73](deferred-perf-tests.md#L73)). Existing Level-1 chunking and timeout tests
are valuable structural evidence, but they do not instantiate the required hundreds/thousands-service
workload. A benchmark-only or feature-gated fixture seam can expose private parser/runner inputs to
Criterion without adding to the default production API. Add that workload and map its spawn/chunk
bounds, or narrow the benchmark requirement through a reviewed specification change.

## Verification Levels

| User-observable requirement | Strongest present verification | Review result |
|---|---|---|
| Bare aggregate JSON schema, stdout/stderr split, scope buckets, context, and version | Level 1 process/snapshot/unit tests | Appropriate tier and green on macOS, but the command-wide discovery/work bounds are not tested and are violated. |
| `--perf` work counts and elapsed report | Level 1 library counter tests plus manual CLI observation | Appropriate tier, but the emitted report snapshots before aggregate work and is incomplete. |
| Structure/focused request semantics, inventory truncation, Git bounds, remote reuse, NTP policy, manifest reuse, and path ownership | Level 1 unit/integration/work-count tests | Appropriate tier; focused macOS checks are green. |
| Subprocess deadlines and descendant cleanup | Level 1 direct-child and same-process-group descendant tests | Appropriate tier, but the Unix detached-descendant case and production bypass paths are unverified. |
| macOS/Linux/Windows output, path, process, and case behavior | Level 1 macOS execution plus Windows GNU cross-target compile | Insufficient: compile coverage is not native Level-1 execution on Linux/Windows. |
| Terminal glyphs, widths, SGR styling, and scrolling | No changed presentation requirement in this feature | Level 2 is not required; the aggregate work is JSON/host observation. |
| Keyboard, modifier, hotkey, paste, IME, and mouse behavior | No requirement | Level 3 is not applicable. |

## Checks Run

```text
bf reference @sniff/features/2026-07-16-performance/spec.md
bf reference @sniff/features/2026-07-16-performance/review-4.md
sniff repo packages --json
just test
  sniff-lib: 1657 passed, 9 skipped
  sniff-cli: 777 passed, 3 skipped
  two library tests triggered leaked-handle retries and passed on retry
just lint
just build
cargo check -p sniff --benches --features remote
cargo check -p sniff --all-targets --features remote --target x86_64-pc-windows-gnu
  passed with four target-gated test warnings
cargo run -q -p sniff-cli --bin sniff -- --base sniff/lib --perf repo --json
/usr/bin/time -p target/debug/sniff --base sniff/lib --perf repo --json
  emitted Total: 305.95 ms; end-to-end wall time: 0.95 s
```

The review-3 fixes for aggregate-builder purity, linked-worktree status reuse, deterministic
snapshots, non-Cargo manifest failure caching, component-aware area matching, and most benchmark
families are present and their focused tests pass. The remaining aggregate observation boundary is
still outside both reuse and accounting, and the subprocess/cross-platform gaps keep the feature from
meeting its completion boundary.

Production readiness: **not ready**.
