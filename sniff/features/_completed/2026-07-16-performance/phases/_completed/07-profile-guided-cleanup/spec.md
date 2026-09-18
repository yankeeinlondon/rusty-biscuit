---
sub-spec: true
depends-on:
  - ../05-git-observation/spec.md
  - ../06-remote-network-and-subprocess/spec.md
  - ../../../spec.md
phase: 7
status: complete
date: 2026-07-17
---

# Phase 7 — Profile-gated remaining hot loops

Implement umbrella requirements **R13.3–R13.4** and **R14**, both of which are written as
*conditionals*: R13.3 applies "when profiling confirms the current linear scans remain visible",
R13.4 "must be justified by measurement", and R14 opens with "After R2–R13, **profile before
implementing**". This phase's deliverable is therefore the measurement and the keep/defer record —
not a list of edits.

The plan's own framing governs: this phase "is optional per candidate and must not create
speculative abstractions", and its validation checkpoint requires "a repeatable material improvement
with identical results **before retaining** a micro-optimization."

**Outcome: every candidate is deferred, with evidence. No production code changed in this phase.**

That is a result, not an absence of one. The structural phases (2–6) removed the duplicated work;
what remains in the profile is dominated by single-visit I/O against distinct paths, which no
candidate on the R14 list addresses. Implementing them anyway would have added abstractions and
contract risk to buy back a few tenths of a percent — the exact outcome R14 was written to prevent.

## Materiality threshold

Fixed **before** editing, per the plan.

A candidate is **material**, and therefore eligible for implementation, only if it clears **both**:

1. **Work**: it removes ≥ 5% of a dominant counter in a representative acceptance case, *or*
2. **Time**: it accounts for ≥ 5% of sampled on-CPU time in a representative case,

**and** the improvement is repeatable with byte-identical results. Below 5% nothing on this host is
distinguishable from drift: Phase 3 recorded **+330%** on a Criterion case whose counters were
byte-identical, purely from host load, and this phase's own host sat at load 14→47 on 16 cores.

The 5% figure is chosen so that a candidate must be worth more than the measurement noise floor of
the only hardware available to judge it. A candidate below the threshold is not "not worth doing" in
principle — it is **not measurable as an improvement here**, so retaining it would mean keeping a
change whose benefit cannot be demonstrated, which is what the checkpoint forbids.

## Method

Two representative workloads were re-profiled after Phases 2–6, chosen because the R13/R14
candidates are split across them — the R14 filesystem items live in repo detection, while R13.3,
R13.4, L9, and L14 live in the programs path, which repo detection never touches.

| Workload | Entry point | Fixture |
|---|---|---|
| Repo detection (full) | `detect_repo` | bench `huge_monorepo` (375 packages) |
| Programs fan-out | `ProgramsInfo::detect` | live host `PATH` |

Three instruments, in descending order of trust:

1. **Work counters** — `cargo run -p sniff --release --features network --example work_counts`.
   Exact and load-independent. Primary evidence, per the umbrella spec's verification strategy.
2. **Probe attribution** — a temporary `#[track_caller]` shim on `probe_exists`/`probe_is_dir`
   recording `(call site → probed path → count)`, which answers the question the aggregate counter
   cannot: *how much of this work is duplicate?* Reverted after measurement; it is a measurement
   instrument, not a feature.
3. **Sampled profile** — macOS `sample`, 12s at 1 ms. **Used only for composition** (does a symbol
   appear at all?), never for absolute attribution: the sampler cost ~7× (58 iterations under
   `sample` vs 396 unsampled over the same 25s window), and the host was loaded. A symbol absent from
   a profile this coarse is decisively cold; a symbol present is not thereby material.

### Cold-cache profiling

Not run, deliberately. macOS `purge` requires `sudo`, which this session cannot use, and no
cold-cache proxy on APFS survives having just written the fixture.

This does not weaken the conclusion — it strengthens it. The distinct-vs-redundant probe ratio below
is a property of the *call pattern*, not of the cache, and a cold cache makes each distinct
syscall **more** expensive relative to userspace work. Every R14 candidate is a userspace
micro-optimization. Cold caches therefore shift the balance further toward "defer", never toward
"implement". Warm measurement is the *conservative* case for these candidates, so warm-only evidence
is sufficient to defer them.

## Post-structural profile — repo detection

Counters for `repo_full_huge_375_packages` (375 packages), post-Phase-6:

| Counter | Value |
|---|---:|
| `filesystem.io.metadata_probes` | 13,275 |
| `filesystem.walk.entries_visited` | 4,685 |
| `filesystem.io.read_dirs` | 701 |
| `filesystem.io.file_opens` | 708 |
| `filesystem.repo.manifest_parses` | 454 |
| `filesystem.repo.package_enrichments` | 300 |
| `filesystem.io.canonicalizations` | 600 |

Sampled composition (26,110 samples; leaf-attributed, profiler-inflated — read as shape only):

| Bucket | Samples | Share |
|---|---:|---:|
| `open`/`stat`/`getdirentries`/`read` and friends | ~18,600 | ~71% |
| Thread park/wait (`__semwait_signal`, `__ulock_wait`) | ~3,771 | ~14% |
| `std::path` component compare/iterate | ~754 | ~2.9% |
| malloc/free | ~600 | ~2.3% |

The residual cost is **syscalls, not CPU**. The single largest userspace bucket is path-component
comparison at ~2.9%, and it is spread across the `starts_with` prefix logic that ownership and
membership correctness actually depend on.

### The decisive measurement: probes are not duplicate work

`metadata_probes` at 13,275 against 4,685 walked entries reads like the dominant reuse defect in the
codebase, and it was this phase's leading hypothesis. Attributing every probe to its call site and
probed path **disconfirms it**:

```
TOTAL probes=13275  distinct=12675  redundant=600   (4.5% redundant)
```

| Call site | Total | Distinct | Redundant | Worst path |
|---|---:|---:|---:|---:|
| `test_runner_usage.rs:290` (`locate_config`) | 3,100 | 2,702 | 398 | 200 |
| `detection.rs:1097` | 600 | 600 | 0 | 1 |
| `test_runner_usage.rs:884` | 500 | 500 | 0 | 1 |
| `npm.rs:157` | 404 | 202 | 202 | 2 |
| every other site | 300 | 300 | 0 | 1 |

**95.5% of probes are first-and-only visits to distinct paths.** They are the marker-presence checks
that *are* the detection contract ("does `<pkg>/vitest.config.ts` exist?"), asked once each. There is
no observe-once win left here: the tree is already observed once, and these are questions about paths
the walk legitimately did not answer.

The 600 redundant probes are ~4.5% of probes and ≈ 0.9 ms of a ~63 ms detection — **below the
threshold on both axes**. Their composition is recorded for the record:

- **398** at `locate_config`, of which the worst single path repeats **200×**: the root-scoped
  nextest config (`<repo_root>/.config/nextest.toml`), re-probed once per member. This is the
  residue of **Phase 4's open R5.6 item** ("root-scoped test-runner configuration at most once per
  detection"), which is an R5 obligation owned by Phase 4 — **not an R14 candidate**, and out of
  scope here. Recorded so Phase 4's remaining work has a measured size: ~200 probes, ~0.3 ms.
- **202** at `npm.rs:157`, each path probed exactly twice.

## Post-structural profile — programs fan-out

`ProgramsInfo::detect`, 147,777 samples:

| Bucket | Samples | Share |
|---|---:|---:|
| `swtch_pri` + `__psynch_cvwait` (Rayon park/spin) | ~142,080 | ~96% |
| `stat` | 3,680 | ~2.5% |
| `__open_nocancel` | 907 | ~0.6% |
| `Program::from_binary_name` (L9) | **0** | **absent** |
| `method_available` (L9) | **0** | **absent** |
| `local_bin` / `LocalBinIndex` (L14) | **0** | **absent** |

R13.3 and R13.4 are conditional on their targets being "visible" / "justified by measurement". They
are not visible at 1 ms sampling. **The conditions in the requirements themselves are not met**, so
deferring is compliance with R13, not a departure from it.

## Keep/defer decision — every R13.3/R13.4/R14 candidate

Required by the plan: "for every deferred candidate, record evidence that its cost is negligible or
its contract risk exceeds measured benefit."

| # | Candidate (source) | Decision | Evidence |
|---|---|---|---|
| 1 | ASCII case-sensitive fast paths for filename/extension classification | **Defer** | Already ASCII: `classify.rs:570` uses `to_ascii_lowercase`, not `to_lowercase`. Residue is one small allocation per extension classification (3,375/detect) ≈ 0.1–0.2 ms of ~63 ms (**~0.3%**). The named optimization is substantially already done. |
| 2 | Bounded prefix reads for framework hints | **Defer** | `filesystem.io.bytes_read` is **50,548 bytes** for the whole 375-package detection — the entire read volume is ~50 KB. A bounded-prefix scheme cannot remove a material share of 50 KB, and it trades exactness of framework detection for it. **Contract risk exceeds measured benefit.** |
| 3 | Reuse walker metadata for Markdown modification time | **Defer** | `docs.rs:1023-1026`: 182 probes/detect = **1.4% of probes**, ≈ 0.3 ms. Below threshold. Would couple docs projection to walker-entry lifetime for that. |
| 4 | Static regex initialization (L6) | **Defer — premise partly stale** | `standard.rs:608` is now `#[cfg(test)]`-only; the review's "~50×/run during version stamping" **no longer describes production**, which leaves `version` as `None` to honor the no-subprocess boundary. `schema.rs:356`/`:374` each parse the output of a `--version` **subprocess**: a `Regex::new` (~10–50 µs) sits behind a process spawn (~10 ms+), i.e. **≲0.5%** of the operation it belongs to. `:374`'s pattern is dynamic (`info.version_regex`), so hoisting it needs a keyed cache — an abstraction with no measured benefit. The only regex in the sampled profile is `ignore`'s own gitignore parsing, already `OnceLock`-cached upstream. |
| 5 | Allocation-free / set-backed path-list merges (L12) | **Defer — target gone** | `merge_path_lists` no longer exists; Phase 4 removed it with `merge_packages`/`dedupe_packages`. `docs.rs:1047 assign_packages` survives but is **absent from the profile**. |
| 6 | Static program-name maps (L9) | **Defer — condition unmet** | R13.3 is explicitly gated on the scans remaining "visible"; they are absent from the profile. `from_binary_name`'s only non-test caller is `Deserialize` (`inventory.rs:129`) — once per deserialized value, not a detection loop. |
| 7 | Rate-limit matching without whole-body lowercasing (L7) | **Defer** | `body.to_lowercase()` runs **only on a 403 error path**, once, behind a network round trip (~50–500 ms). Immaterial by construction. |
| 8 | Interface-cache clone reduction (L8) | **Defer** | One clone per `detect`, absent from the profile. The review itself only claims "clone cost *may* exceed savings" — speculative, and unmeasurable at this size. |
| 9 | Small cache/index improvements in test-runner resolution (L14) | **Defer — condition unmet** | R13.4 requires parallel resolution be "justified by measurement". `local_bin`/`LocalBinIndex` are **absent** from the programs profile, which is 96% Rayon parking. Adding parallelism to a workload already dominated by park/spin would add contention, not throughput. |

## Non-regression: parallelism and storage concurrency

The plan's final Phase 7 task is a prohibition rather than a change: "Do not add unbounded CPU-scaled
parallelism; keep storage concurrency bounded and internally configurable for benchmarks." Verified
as satisfied on HEAD, and nothing in this phase alters it:

- **No unbounded CPU-scaled parallelism exists or was added.** `num_cpus`, `available_parallelism`,
  `ThreadPoolBuilder`, and `build_global` appear **nowhere** in `sniff/lib/src`. Parallel sites use
  Rayon's default pool or `ignore`'s walker.
- **Storage detection is serial.** `hardware/storage.rs` contains no `par_iter`/`spawn`; there is no
  storage concurrency to bound.
- **Remote fetch parallelism is already bounded**, and documented as such (`remote_refresh.rs:458`).

The programs profile's 96% park/spin is worth recording as a **caution for a future phase, not a
finding for this one**: it means `ProgramsInfo::detect`'s fan-out is already past the point where
more threads help. It is not an R13/R14 candidate and was not measured for wall-clock effect (this
host cannot), so it is noted rather than acted on.

## Acceptance

Phase 7 changes no production code, so its acceptance is that the tree is provably unchanged and
still green:

| Check | Result |
|---|---|
| `just test` | 1603/1604 — the sole failure is the pre-existing `detect_area_errors_when_not_in_repo` timeout, a known baseline since Phase 4, verified on clean HEAD and untouched by this phase |
| `just lint` | clean, zero warnings |
| `just build` | clean |
| `just doctest` | clean |
| Criterion groups | **None run.** The checkpoint scopes them to "only the affected Criterion groups"; no candidate was implemented, so no group is affected. Running them would produce exactly the untrustworthy timings Phases 3, 5, and 6 already deferred to Phase 8. |
| `git diff` for production code | empty for this phase — the measurement shim was reverted, and the three temporary examples deleted |

## Not done / handed onward

- **Cold-cache profiles** — blocked on `sudo`; argued above to be unnecessary for a defer decision,
  since cold caches only strengthen it. If Phase 8 gains an idle host with `sudo`, the cheap
  confirmation is that the ~71% syscall share rises.
- **Root-scoped test-runner config probed 200× per detection** — Phase 4's open R5.6 item, now with a
  measured size (~200 probes, ~0.3 ms). Belongs to Phase 4's `ManifestStore` work, not R14.
- **375-package fixture only.** The 100/500/2,000-package sweep remains Phase 4's open item. The
  probe-redundancy ratio (4.5%) is a per-package call-pattern property and does not change with
  package count; the *absolute* probe count scales linearly, so a larger fixture would not promote
  any deferred candidate above the threshold.
