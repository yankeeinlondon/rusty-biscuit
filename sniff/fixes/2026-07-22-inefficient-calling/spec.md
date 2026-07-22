---
date: 2026-07-22
agent: "${env.AGENT}"
---

## Problem Statement

`sniff` re-runs its expensive, **process-stable** discovery — OS, hardware
(CPU/memory/storage), and GPU — on every detection call with no process-lifetime
memoization. Each call re-spawns the same subprocess probes (`system_profiler`,
`sysctl`, `sw_vers`, Metal enumeration) and re-walks the same repo structure, even
though none of that data can change while a single process is alive.

This is normally invisible because most callers detect once per process. It
becomes a measurable defect when an upstream caller legitimately invokes
detection more than once per process: every redundant call re-pays the full
subprocess cost, and under parallel load those subprocess spawns contend and
produce **intermittent timeouts** — i.e. flakiness, not just slowness.

### The exposing signal

A claudine unit test surfaced this as a flake:

```
FLAKY 2/4 [ 23.089s] claudine composition::lifecycle::executor::tests::runtime_set::set_refuses_every_reserved_root_key
```

- **Test:** `set_refuses_every_reserved_root_key` at
  [`claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs:191`](../../../claudine/lib/src/composition/lifecycle/executor/tests/runtime_set.rs)
- It loops over the five reserved root keys and runs one lifecycle event per key.
  The reserved-key check itself is immediate and deterministic; the flakiness is
  purely the *cost* of the per-event machinery crossing a timeout threshold, not
  the assertion logic.

## Root Cause

Two layers compound. The **proximate** cause is upstream (claudine invokes the
full capture repeatedly); the **sniff-level** cause — the subject of this fix —
is that sniff makes each of those invocations expensive instead of cheap.

### Proximate (upstream, tracked separately — see Non-Goals)

Claudine's lifecycle executor evaluates each side-effect argument through
`eval_expr` (`claudine/lib/src/composition/lifecycle/executor.rs:784`), which
builds a resolution context via `resolution_context()` (`...executor.rs:727`) →
`document_expression_resolution_context`
(`claudine/lib/src/composition/mod.rs:172`). When no prepared context snapshot
is supplied, that path calls darkmatter's **full**
`ComposeContext::capture()` (`darkmatter/.../context/runtime.rs`) — *all* context
groups — once per argument. It does **not** reuse the demand-driven capture the
same call already builds for the expression state. For the exposing test that is
~2 full captures per event × 5 events = ~10 full captures.

### Sniff-level (this fix)

Each full capture (`darkmatter/.../context/capture/snapshot.rs`,
`ContextCapture::new`) invokes these `sniff` entry points:

| sniff entry point | Stability during a process | Cost class |
|---|---|---|
| `sniff::os::detect_os_with_request` (`os/mod.rs`) | **stable** | subprocess (`sw_vers`/`uname`) |
| `sniff::hardware::detect_hardware_summary` (`hardware/mod.rs`) | **stable** | subprocess + sysctl; skill notes ~1.5s macOS |
| `sniff::hardware::detect_gpus` (`hardware/mod.rs`) | **stable** | Metal / `system_profiler` |
| `sniff::filesystem::repo::detect_repo_structure` | stable (read-mostly) | filesystem walk |
| `sniff::filesystem::git::GitRepo::discover` + `file_changes()` | identity stable; **file changes mutable** | gix walk |

None of the stable rows is memoized across calls. A `grep` for process-lifetime
cache primitives (`OnceLock`/`OnceCell`/`Lazy`/`static … Cache`) confirms **no**
such caching exists under `sniff/lib/src/hardware/` or the OS identity paths of
`sniff/lib/src/os/` (only WAN IP carries a TTL cache, and the program
`ExecutableIndex` is `Arc`-shared within one detection plan). So every repeated
capture re-runs the subprocess probes in full.

### Why this reads as flaky, not just slow

Instrumented on this checkout, each full capture took **~480–517 ms** and the
exposing test ran **~5.8–8.5 s** (nextest flagged it `SLOW > 5s`). Sibling tests
that perform a single event settle at ~1.7 s. The ~480 ms per capture is
dominated by subprocess spawns and a repo filesystem walk whose wall time varies
with host load; under parallel `nextest` / CI contention those spawns balloon
past the flake threshold on some runs and not others — producing the observed
intermittent (2/4) failures rather than a consistent slowdown.

## What This Investigation Established vs. Open Questions

Established in this limited window (measured, not inferred):

- The claudine→darkmatter→sniff call chain and the per-capture cost (~480–517 ms)
  were measured with `Instant` instrumentation at the claudine layer.
  `execute_event` is ~1.1–1.5 s; `build_state` is only ~60 ms; the cost is
  almost entirely the full `ComposeContext::capture()` invoked by
  `resolution_context()`.
- The set of `sniff` entry points darkmatter's full capture invokes is confirmed
  from source (`snapshot.rs`).
- The absence of process-lifetime caching for OS/hardware/GPU in `sniff/lib/src`
  is confirmed by search.
- OS, hardware, and GPU are provably process-stable, so caching them for a
  process lifetime is semantics-preserving.

Open for implementation (not yet measured):

- **Within-sniff attribution of the ~480 ms.** We measured the cost at the
  claudine/darkmatter boundary; we did *not* profile inside sniff to split the
  ~480 ms across OS / hardware / GPU / repo-structure / file-changes. The skill's
  "~1.5s macOS" hardware figure is for the full hardware summary in isolation and
  exceeds the whole-capture budget measured here, so either the in-capture
  hardware probe is cheaper than the standalone figure or parallelism
  (`std::thread::scope`) is hiding it. Precise attribution should use sniff's own
  stage decomposition (`--perf`) — which itself depends on the in-flight
  `corrected-perf-flag` fix landing first.
- **Repo-structure cache invalidation policy.** Package layout is read-mostly
  but *can* change if a process writes files; the right staleness boundary
  (process-lifetime vs. explicit invalidation) is a design decision for R2.
- **Whether to cache at the Tier-3 function boundary, the request/plan boundary,
  or both.** See R4.

## Goals

1. Make repeated detection of process-stable facts (OS, hardware, GPU) **O(1)
   after the first call** within a single process, transparently, with no change
   to detected values.
2. Make sniff robust to upstream over-calling: even if a caller invokes
   detection many times per process (as claudine/darkmatter do today), sniff
   stays fast. This is defense-in-depth that benefits every caller in the
   monorepo, not just the one that exposed it.
3. Preserve the existing `force_refresh` precedent (network WAN IP) so callers
   that genuinely need fresh data can bypass the cache.

## Non-Goals (Out of Scope)

- **The upstream claudine/darkmatter dedup is a separate, complementary fix.**
  Claudine's `resolution_context()` should reuse the demand-driven capture it
  already builds instead of re-running the full capture per `eval_expr`. That
  removes the *redundancy*; this sniff fix removes the *per-call cost*. Both are
  worth doing — the claudine fix because redundant work is wasteful regardless of
  sniff speed, and this fix because it makes sniff correct-to-call repeatedly.
  The claudine fix is tracked separately (not in this spec).
- **Changing detection accuracy or the detected value set.** Cached results must
  be byte-identical to fresh results for stable fields.
- **Caching mutable state without an invalidation story.** Git file-changes
   (`file_changes()` / git status) are mutable and are explicitly out of scope
   for unconditional process-lifetime caching (see R2).
- **Cross-process / on-disk caching.** This is an in-process memoization only;
   no cache files, no shared memory, no persistence.

## Requirements

### R1 — Memoize process-stable discovery

`sniff` MUST memoize, for the lifetime of the process, the results of discovery
whose value cannot change while the process runs:

- OS identity: `os::detect_os_type()`, `os::detect_os_with_request()` (and
  `os::detect_os()`).
- Hardware summary: `hardware::detect_hardware_summary()` (CPU, memory, storage).
- GPU: `hardware::detect_gpus()`.

A second call within the same process, with an equivalent request, MUST return
the cached result without re-running subprocess probes or re-walking hardware.

Semantics are unchanged: the cached value is exactly what a fresh call would
have returned at first-call time. For OS/hardware/GPU that is the process's
ground truth for its entire run.

### R2 — Mutable discovery is excluded or explicitly invalidatable

Git working-tree state (`GitRepo::file_changes()`, git status) is **mutable**
and MUST NOT be unconditionally cached for the process lifetime. Either:

- leave it uncached (simplest, preserves current behavior), or
- cache it behind an explicit invalidation/refresh API consistent with the
  network WAN-IP `force_refresh` precedent.

Repo *identity* (`GitRepo::discover` → repo root) and repo *structure*
(`detect_repo_structure`, package layout) are read-mostly and are acceptable
candidates for process-lifetime caching keyed by base dir / repo root, but the
implementation MUST document the staleness assumption (a process that mutates
package layout on disk accepts a stale structure view). The default choice —
cache identity, leave `file_changes` uncached — is acceptable.

### R3 — Cache keying and correctness

- OS/hardware/GPU need **no key**: they are process-global.
- Any path-keyed cache (repo identity/structure) MUST key on the canonical path
  supplied by the caller, and the cache hit MUST return a result equivalent to a
  fresh call for that same path.
- The cache MUST be coherent: concurrent first-callers for the same key compute
  the result once (or at worst a bounded number of times), not once per racing
  thread.

### R4 — Thread-safety and placement

The cache MUST be `Sync` (sniff already runs domains in `std::thread::scope`
workers and is called from Rayon parallel paths). Preferred primitives are
`std::sync::OnceLock` (process-global, keyless fields) or a small
`Mutex`/`DashMap` for keyed entries — matching the existing style of the WAN-IP
TTL cache and the `Arc`-shared `ExecutableIndex`.

Implementation MAY memoize at the Tier-3 function boundary
(`detect_hardware_summary`, `detect_gpus`, `detect_os_*`) or at the request/plan
boundary (`DetectionPlan`) — whichever is smaller and lower-risk — but the
memoization MUST be transparent to all three API tiers (convenience, plan-based,
module-level) so a caller cannot bypass it by choosing a different tier.

### R5 — Opt-out escape hatch

Following the WAN-IP `force_refresh` precedent, provide a documented way to
bypass the cache for callers that need a fresh read (tests, diagnostic CLIs).
Default behavior is cached (safe, since the data is process-stable); the escape
hatch is explicit and opt-in only.

## Affected Code

| File / area | Change |
|---|---|
| `sniff/lib/src/hardware/mod.rs` (+ `cpu.rs`/`gpu.rs`/`memory.rs` as needed) | R1: memoize `detect_hardware_summary()` and `detect_gpus()` process-globally. |
| `sniff/lib/src/os/mod.rs` | R1: memoize `detect_os_type()` / `detect_os_with_request()` / `detect_os()` process-globally. |
| `sniff/lib/src/filesystem/git/` and `filesystem/repo/` | R2: cache repo identity (keyed by path); decide and document `file_changes` / `detect_repo_structure` staleness policy. |
| New `sniff/lib/src/cache.rs` (or extend an existing module) | R3/R4: shared memoization primitives + keying, if a dedicated module is cleaner than per-function `OnceLock`s. |
| `sniff/lib/README.md`, sniff skill | Document the process-lifetime cache, what is/isn't cached, and the R5 bypass. |

No public API signature needs to change for the default (cached) path; R5's
bypass is additive.

## Testing

### Library (`sniff/lib`)

- **Stable fields are memoized.** Call `detect_hardware_summary()` twice in one
  process; assert the second returns in microseconds and the subprocess probe
  runs exactly once (count spawns, or assert via a timing/`--perf`-style guard).
  Same for `detect_gpus()` and `detect_os_with_request()`.
- **Cache is correct.** For stable fields, `cached == fresh-equivalent` (within
  a process, a second uncached-style call returns the same values). A test that
  forces a bypass (R5) returns the same values as the cached path for stable
  fields.
- **Mutable fields stay fresh.** A `file_changes()` call after a simulated
  working-tree mutation reflects the change (i.e. it is not poisoned by a stale
  process cache). This guards R2.
- **Concurrency.** N threads racing the first call to a memoized function
  compute the underlying probe a bounded number of times (ideally once) and all
  observe the same value (R3/R4).
- **All three API tiers hit the cache.** The same stable value is returned and
  the probe fires once whether the caller used Tier-1 `detect()`, a
  `DetectionPlan`, or the Tier-3 function directly (R4).

### Integration signal (claudine)

The exposing test is the cross-package regression signal. After this fix (and
independently of the claudine-side dedup), running it should drop from ~5.8–8.5 s
to well under the nextest `SLOW` threshold, and the intermittent FLAKY failures
should disappear. (With the complementary claudine dedup, the per-event cost
collapses further still.)

```
just test set_refuses_every_reserved_root_key      # from claudine/ area
```

## Success Criteria

1. A second in-process call to `detect_hardware_summary` / `detect_gpus` /
   `detect_os_*` is O(1) and runs no subprocess probes; the first call's result
   is reused.
2. Detected values for OS/hardware/GPU are unchanged (cache is transparent).
3. Git working-tree state is never served stale (R2 honored; `file_changes`
   remains fresh).
4. The claudine exposing test is no longer `SLOW`/flaky, and its wall time drops
   substantially even before the claudine-side dedup lands.
5. A documented opt-out (R5) exists for callers needing a fresh read, and
   `sniff`'s three API tiers all benefit from the cache without per-tier wiring.
