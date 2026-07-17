---
status: draft
reviewed: true
reviewed_by: claude/default
reviewed_on: 2026-07-15
review_iterations: 5
created: 2026-07-15
source_review: ../../reviews/2026-07-12-perf/spec.md
source_assessment: ../../reviews/2026-07-12-perf/review-3.md
source_baseline: ../../reviews/2026-07-12-perf/baseline.md
source_results:
  - ../../reviews/2026-07-12-perf/results.md
  - ../../reviews/2026-07-12-perf/results-2.md
related:
  - ../_completed/2026-07-15-reference-graph
recovery_branch: rescue/review3-terminated-agent
audit_commit: 51c1f16e10ffe825b56987573ba4eabc659c768e
---

# Performance Follow-up

## Status

Draft. This specification records the unfinished work found by auditing all 35
findings in the 2026-07-12 performance review against the current branch. It is
the owner-approved scope move required by that review's delivery contract; the
moved work is not thereby considered complete.

The opaque `ReferenceGraph` correctness work is intentionally owned by the
separate [Opaque Reference Graph](../_completed/2026-07-15-reference-graph/spec.md)
specification (archived to `_completed/` after this feature's Phase 11). This
document retains Finding 18 only as audit history and does not duplicate that
feature's implementation.

## Summary

The original performance work produced substantial, credible gains:

- ordinary compose no longer performs an NTP request;
- TOC line lookup is no longer quadratic;
- duplicate schema resolution, validation, compilation, and ownership costs
  were substantially reduced;
- reference-graph construction is reused by graph validation;
- common render-path parsing and allocation costs were removed;
- Finding 29 has a strong same-source Criterion comparison supporting its
  approved `Arc<Value>` compatibility exception.

It did not satisfy its full delivery contract. Several optimizations are only
partially implemented, some were not attempted, the terminal claims lack the
required real-terminal evidence, the command/TOC measurements did not use
identical hashed fixtures, and Finding 22 introduced an unapproved behavior
change. This feature closes those gaps without reopening completed work.

## Audit Method

The audit used the implementation at
`51c1f16e10ffe825b56987573ba4eabc659c768e`, the reindexed GitNexus graph, the
original specification and plan, all three review/result documents, and the
current Darkmatter, Sniff, Biscuit Terminal, and CLI source paths.

Status means:

- **Complete** — the requested mechanism and its relevant behavioral coverage
  are present. A feature-wide reproducibility gate may still apply.
- **Partial** — a safe subset landed, but at least one requested sub-item or
  requirement-matched verification remains open.
- **Open** — the requested optimization is absent, or a landed change must be
  corrected before the finding can close.
- **Separated** — implementation belongs to another linked active feature.

The terminated agent's work on `rescue/review3-terminated-agent` is not counted
as current implementation. Individual patches, tests, and measurements may be
recovered only after ordinary review against this specification and rerunning
the applicable gates.

## Finding-by-Finding Audit

The **Status** and **Work retained here** columns record the audit as of
`51c1f16e10ffe825b56987573ba4eabc659c768e` and are the input that scoped this
feature. **Final** records what actually landed, added at the Phase-11 closeout
(2026-07-16); it is the honest disposition and it supersedes Status wherever the
two disagree. Every Final entry's evidence — measurement, thresholds, tests,
and cross-platform classification — is in
[`results.md`](./results.md).

| Finding | Status | Current implementation | Work retained here | Final (2026-07-16) |
|---:|---|---|---|---|
| 1 | Partial | Darkmatter explicitly calls `detect_timezone_with_options(false)`, removing its unused NTP request. | Restore bare `sniff::detect_timezone()` to its compatible full-report behavior by delegating to `true`; retain Darkmatter's explicit `false` call. | **Corrected.** Bare API delegates to `true` again; Darkmatter keeps `false` via a local seam. |
| 2 | Partial | OSC 10 text-color results are process-cached. | Add L2 proof that repeated terminal construction emits one OSC 10 request and record repeated-construction latency. | **Verified (L2)** on macOS (real WezTerm, **gated**) **and real Linux** (real kitty under a Linux kernel in Docker, **retained manual run**). 3 constructions → exactly 1 OSC 10; macOS median 0.970 ms. Oracle is theme-independent: the pane's foreground is pinned and demanded back verbatim. Work-2.3's *"record repeated-construction latency with warm-up, sample count, dispersion"* is satisfied by the **run record** ([`run-20260716T065617`](./benchmarks/raw/f2f3f21-terminal-evidence/run-20260716T065617/summary.md): warm-up 3, 50 samples, median 0.970 ms, stddev 0.022 ms) — the run record, not a test assertion, is the measurement of record. ⚠ Two earlier-revision corrections: this row once claimed real-Linux L2 on the strength of a **manufactured-PTY (L1)** run (corrected 2026-07-16, review-2); and `terminal_repeated_construction_latency`'s wall-clock threshold — which review-3 found flaky under area-test load — was replaced (review-3) by a **structural invariant**: exactly one OSC 10 across 53 constructions, counted on the PTY master's bytes and cross-checked against the crate-private `osc_query_attempt` tracing event. No new public API. |
| 3 | Partial | The compose CLI shares a terminal through a per-invocation `OnceCell`. | Add an end-to-end CLI case exercising verbose, performance, and warning-report branches and prove one detection per invocation. | **Verified.** Detections *counted* = 1 across verbose + perf + warning branches. |
| 4 | Partial | TOC line lookup uses an offset table and binary search; compatibility tests cover line/span behavior. | Replace the non-comparable closeout with identical, hashed fixtures, predeclared thresholds, and retained raw samples. | **Closed.** Reconstructed on identical hashed bytes: `toc_large` 488 → 23 ms. |
| 5 | Complete | Schema validation uses the already resolved effective schema rather than resolving and coercing twice. | No implementation work. Protect in the final benchmark and regression gates. | **Unchanged**; held by the final gates. |
| 6 | Complete | Coercion participates in validator-cache reuse instead of compiling uncached validators per union arm. | No implementation work. | **Unchanged.** |
| 7 | Partial | Reference-graph reuse and safe preflight target reuse reduce repeated walks. | Design and measure compatible sharing of prepared/interpolated content across validate, preflight, and compose. | **No safe broadening remained** — existing reuse already at the spec's level 3 (full semantic identity, single-flight). Nothing added, so nothing to remove. |
| 8 | Complete | Validator/coercion/namespace caches are reused with the required identity inputs and bounded behavior. | No implementation work. | **Unchanged.** |
| 9 | Complete | The built-in baseline schema uses shared process-cached ownership rather than repeated conversion and deep cloning. | No implementation work. | **Unchanged.** |
| 10 | Complete | `ctx.*` lookup no longer clones the full context-values map for each access. | No implementation work. | **Unchanged.** |
| 11 | Open | The frontmatter interpolation fixpoint still repeatedly extracts references and clones maps/values as keys become eligible. | Parse dependency information once, maintain incremental readiness, and avoid rebuilding seed maps while preserving cycles, shell deferral, best-effort propagation, and key-scoped errors. | **Implemented**, byte-identical. `O(sweeps×keys)` re-parse + `O(keys²)` cloning → `O(keys+edges)`. Below the whole-pipeline measurement floor; retained as a structural work-reduction. |
| 12 | Open | Expression functions still receive an owned `Option<ResolutionContext>`, cloning its context for repeated calls. | Add an internal borrowed/shared path while retaining the owned public facade where compatibility requires it. | **Implemented**, byte-identical. Defaulted `resolution_context_ref` borrow path; owned public method unchanged. Strictly-fewer-clones, not separately measurable. |
| 13 | Open | Text replacement still scales with document length times rule count and builds a character-index vector. | Implement and benchmark a faster exact matcher or record a requirement-matched no-win result; preserve descending key-byte-length/ascending lexical precedence, left-to-right non-recursive matching, Unicode boundaries, empty-key handling, and scalar coercion. | **Implemented — measured win ≈27×** (2.371 ms → 0.087 ms). Aho–Corasick `LeftmostLongest`; every precedence rule preserved. |
| 14 | Partial | Literal conversion now skips its scan when `{{{` is absent. | Reduce repeated Markdown-aware scans and full-body copies when interpolation is present; benchmark nested and no-expression cases separately. | **Implemented — measured win ≈104× on skipped work** (240.1 µs → 2.3 µs) for `{{`-free bodies. Nested/rescan path untouched. |
| 15 | Complete | Parent headings and line offsets are parsed once and queried through memoized/indexed structures. | No implementation work. | **Unchanged.** |
| 16 | Partial | Some graph/preflight data is shared through `Arc`, but visited documents may still be composed again. | Solve the remaining condition-aware prepared-content duplication without reusing bodies whose output depends on parent state or directive position. | **No safe broadening remained** (see Finding 7). Parent state / directive position / conditions already yield distinct keys. |
| 17 | Partial | Parallel body-shell execution was correctly rejected because commands must retain source-order side effects. | Replace or avoid the independent 10 ms completion polling loop and prove unchanged timeout, output, and failure semantics. Arbitrary directive parallelism remains prohibited. | **Implemented.** Both loops → one blocking `wait_with_timeout`. ✅ **Linux + Windows behavioral runs DONE** — 14/14 on real Windows 11 ARM64, 26/26 on a real Linux kernel. |
| 18 | Complete / Separated | Graph construction is reused and fragment slug lookup is memoized. | No performance work. Document/options/mode identity and graph opacity belong exclusively to the linked `ReferenceGraph` feature. | **Separated — nothing landed here.** Only the shared field classification is *consumed*, from that feature's commit `a8e5e98d9`. |
| 19 | Complete | Protected-range parsing is gated behind a plausible delimiter scan. | No implementation work. | **Unchanged.** |
| 20 | Complete | Text events without disclosure directives retain their borrowed/event representation instead of being unconditionally reallocated. | No implementation work. | **Unchanged.** |
| 21 | Partial | The macOS appearance probe is cached and gated away from non-TTY paths. | Verify it together with Findings 2 and 3 in real-terminal L2 coverage; piped CLI timing is insufficient. | **Verified.** PATH-shim sentinel proves no `defaults` fork on redirected output. |
| 22 | Open / Correction | Directory hashing now unconditionally excludes `node_modules`, `target`, and `vendor`, changing aggregate membership. | Restore prior membership. A future exclusion policy requires a separately approved compatibility ruling, migration semantics for persisted hashes, and an end-to-end aggregate/exit-status test. | **Reverted.** Only dot-prefixed dirs pruned; no migration needed (never released). ✅ **Linux + Windows behavioral runs DONE** — 15/15 CLI + lib unit on both; Windows agrees on membership. |
| 23 | Partial | Syntect themes are borrowed instead of deep-cloned per code block. | Resolve environment/theme choice once per render snapshot rather than reading it per block; retain dynamic behavior across separate renders. | **Implemented — contract met, no measurable win** (≈0.1 % net vs a control that moved the same). Retained as plan-mandated, byte-identical; honest claim recorded. |
| 24 | Complete | Code-block emission writes directly into the output buffer instead of allocating per-token formatted strings. | No implementation work. | **Unchanged.** |
| 25 | Partial | Four placeholder replacements are fused into one scan. | Measure and, when beneficial, combine compatible ordered line-based cleanup passes; preserve exact pass ordering and canonical output. | **No-win — profiled, not implemented.** Ceiling <7 % of cleanup ≈0.5 % of compose, below σ; exact equivalence unavailable; `cleanup_content_internal` impact HIGH. No code written or retained. |
| 26 | Complete | Validator cache identity uses the repository's fast hashing path rather than repeated SHA-256 work. | No implementation work. | **Unchanged.** |
| 27 | Complete | Named-type namespace reads/parses are memoized and `@this` avoids rebuilding equivalent data. | No implementation work. | **Unchanged.** |
| 28 | Complete | Example target validation and file work reuse the resolution/cache machinery. | No implementation work. | **Unchanged.** |
| 29 | Complete | Effective schemas use shared `Arc<Value>` ownership; built-in baseline paths avoid deep clones. `results-2.md` contains a same-source A/B comparison. | Preserve the approved public ownership exception and its owned compatibility facade. | **Preserved.** Ownership exception + owned facade intact; not reopened. |
| 30 | Complete | `doc.*` lookup walks effective state by reference and clones only the selected result. | No implementation work. | **Unchanged.** |
| 31 | Complete | Variable interpolation stringifies the first lookup result rather than performing the lookup twice. | No implementation work. | **Unchanged.** |
| 32 | Open | Each shell directive still clones read-only policy rule collections into a snapshot. | Snapshot once per stage or use safe shared read ownership; preserve the policy state seen by the stage and avoid holding locks while executing commands. | **Implemented** via the Required-Work-6 *"share immutable collections"* option: the rule sets are `Arc`-shared with copy-on-write writes, and `prepare_directive` takes its own view per directive (three refcount bumps, no rule copy). **No behavior change, no owner decision outstanding.** ⚠ An earlier revision hoisted the snapshot to the 3 stage orchestrators, which changed prompt frequency; that was **reverted at review-1** — a mid-stage persist again suppresses a later same-stage prompt (`persistence_mid_stage_is_policy_input_for_the_same_stage` pins 1 approval). Error-path reservation cleanup is verified by a **deterministic** oracle (review-3): `pending_allow_once_for_test()` asserts the runtime's reservation set directly, replacing the wall-clock inference. |
| 33 | Partial | Remote discovery skips the expensive scan when no HTTP marker exists. | Replace per-expression prefix rescans for line numbers with one forward offset table/pass and measure remote-heavy input. | **Implemented — measured win −82.5 %** (2.394 ms → 419.95 µs) vs a ≥30 % floor; ≈−78 % after discounting build drift. Guard retained. |
| 34 | Complete | Cleanup change detection no longer clones both full bodies solely to compare them. | No implementation work. | **Unchanged.** |
| 35 | Partial | The `md delta` full-document clones were removed. | Complete or disposition the seven remaining copy/hash/read sub-items listed below. | **All 7 dispositioned:** 35.1/35.2/35.4/35.6/35.7 implemented (35.2 **−98.6 %**, 35.6 **−90.9 %**); **35.3 and 35.5's shared-seam step both no-win → reverted.** 35.5 keeps its S0→S1 win (**≈ −16.7 % CLI**) but its S2 cross-call seam (measured −37.6 %) was **removed at review-4**: the `#[doc(hidden)]` `internal` module behind the non-default `internal-hash-orchestration` feature was still additive public API any downstream crate could enable, so it breached compatibility invariant 2. No owner exception was sought or granted; `internal.rs`, the feature, and `plan_hash_save_explained` are deleted and `md hash --diff`/`--save` are back on the public two-call path (double-compute reinstated). A public `compare + explain` pairing API remains a future proposal needing an explicit owner compatibility decision. ⚠ Earlier magnitude figures (`−98.8 % / −91.1 % / −18.0 % / −37.6 %`) are historical: 35.2/35.6 were recomputed from retained raw vectors (review-2), and the 35.5 CLI figures measured now-reverted states. |

Audit totals are 17 complete findings, 13 partial findings, and 5 open or
correction findings. Finding 18 is counted as complete for its performance
portion and separated for its correctness portion.

### Final totals (2026-07-16 closeout; reconciled 2026-07-17 at review-3)

> **This feature is NOT fully closed.** Every *finding* below has a disposition
> backed by a retained artifact or a named test, but the feature-level
> **integrated compose-regression threshold is unresolved** — neither pass nor
> fail. Acceptance criteria 5 and 6 are therefore not met, and the table below
> must not be read as a closure. See *Feature-level open item* at the end of this
> section.

Of the 18 findings carrying retained work (1–4, 7, 11–14, 16, 17, 21–23, 25, 32,
33, and 35's seven sub-items):

- **11 implemented with a measured or structural win** — 1, 11, 12, 13, 14, 17,
  23, 32, 33, and 35.1/35.2/35.4/35.5/35.6/35.7.
- **5 closed as verification/evidence** — 2, 3, 4, 21, and Finding 18's
  separation boundary.
- **3 closed as an evidence-backed no-win or no-op**, per the standing contract
  — 25 (profiled, not implemented), 35.3 (implemented, measured, **reverted**),
  and 7/16 (existing reuse already safe; no speculative code added).
- **1 reverted as a forbidden behavior change** — 22.
- **Cross-platform evidence** — Finding 2's Linux L2 gap is **closed**, but *not*
  for the reason this bullet previously gave. It claimed the gap was closed
  because "its PTY tests pass under a real Linux kernel"; that is the exact
  fallacy `results.md` documents at length — those PTY tests **manufacture their
  own OSC reply bytes**, so they are Level 1 regardless of which kernel runs them.
  The gap is closed by a **real kitty emulator** under a real Linux kernel
  parsing and answering the query, with the library reporting the pinned
  foreground back and `osc10_actual_queries=1`
  ([run record](./benchmarks/raw/f2f3f21-terminal-evidence/run-20260716T180700-linux-kitty-l2/summary.md)).
  Windows **compilation** including
  target-gated test code is evidenced for all four packages. Windows
  **behavioral** runs for 17's wait primitive and 22's directory CLI case are
  **closed as of 2026-07-17**: 30/30 pass on real Windows 11 (10.0.26200.8737,
  ARM64) via a Parallels VM driven by `prlctl exec`
  ([run record](./benchmarks/raw/f-cumulative-closeout/run-20260717T020000-windows/windows-behavioral-run.txt)).
  The earlier claim here that "no Windows host is reachable" was **wrong** — two
  Windows 11 VMs were present on the implementation host throughout. See
  *Cross-platform evidence* in [`results.md`](./results.md).
- **0 behavior changes awaiting owner acceptance.** ⚠ An earlier revision of this
  bullet read *"1 deliberate behavior change awaiting owner acceptance — 32's
  mid-stage prompt frequency"*. That is **no longer true and is corrected here**:
  the prompt-frequency change was **reverted at review-1**, and the live code
  asserts the original frequency
  (`persistence_mid_stage_is_policy_input_for_the_same_stage` → 1 approval;
  `exact_persist_mid_stage_suppresses_only_the_same_command` → 2, the
  over-authorization control). Finding 32's clone removal was kept by sharing the
  rule sets behind `Arc` with copy-on-write writes, so **no compatibility ruling
  is owed**. See *Finding 32* in [`results.md`](./results.md).

**Feature-level open item — the integrated compose-regression threshold.** The
`f-cumulative-closeout` gate (no case may regress `audit → head` by >5 % outside
dispersion) recorded a +11–34 % compose regression that bisected to the linked
reference-graph feature's two commits, **not** to this follow-up's own diff
(which is flat or improving on every case). A remediation — `92a3d502e`,
baseline-schema canonical-JSON caching — has since been measured and **robustly
removes ~25 percentage points** of it (compose cases +12.9…+26.6 % before →
−15.6…+0.76 % after, controls flat). But the **threshold verdict is NOT
ESTABLISHED — neither pass nor fail**: both retained runs' point estimates fall
under 5 %, while the *same* audit-commit binary measured 11.049 ms and 11.957 ms
~1 minute apart — **+8.2 % drift on identical code, larger than the 5 % gate
being adjudicated**. The blocker is **a quiet host, not an owner ruling**; no
ruling can make a load-110 host resolve a 5 % effect. Predeclared admissibility
criteria for the required re-run are recorded in
[`results.md`](./results.md) → *Reference-graph setup remediation*. An owner
decision is required **only if** a clean re-run lands >5 %. The execution
checklist and all future host-availability attempts, admissibility decisions,
and results for this deferred measurement belong in
[`performance-compliance.md`](./performance-compliance.md).

## Required Work

### 1. Restore Finding 1's Compatibility Boundary

Darkmatter needs local timezone information and must continue to call:

```rust
detect_timezone_with_options(false)
```

The zero-argument Sniff convenience API historically requests the full report,
including NTP status. Restore its delegation to
`detect_timezone_with_options(true)` and align its rustdoc and tests. This is a
Sniff compatibility correction, not a rollback of Darkmatter's speedup.

Acceptance requires:

- a Darkmatter-local injectable wrapper or equivalent decision seam proving its
  production path selects `false`, not a source-text assertion;
- Sniff-internal tests proving the bare API selects `true` and the configurable
  API respects both values without making a live NTP request; a dependency's
  `cfg(test)` instrumentation is not assumed to be visible to Darkmatter tests;
- no live network dependency in ordinary Darkmatter compose tests;
- Sniff and Darkmatter L1/lint gates.

### 2. Build Requirement-Matched Terminal Evidence

Findings 2, 3, and 21 share one verification gap. Extend the checked-in Biscuit
Terminal `discovery_probe` + test PTY path so it runs under a supported real PTY
and can observe OSC requests without depending on a user's shell theme. Do not
add a second generic PTY abstraction.

It must verify:

1. two or more `Terminal` constructions in one process emit one OSC 10 query;
2. the cached response is reused, not merely equal by coincidence;
3. repeated construction latency is recorded with warm-up, sample count, and
   dispersion;
4. a single `md compose` invocation that renders verbose, performance, and
   warning output performs one terminal detection;
5. macOS appearance discovery does not spawn for fully redirected output;
6. Unix-only PTY code is target-gated so Windows continues to compile.

Interactive measurements and piped command measurements must be reported
separately. No Level 3 input-protocol test is required.

### 3. Replace the Command/TOC Closeout Harness

The current `baseline.md` and `results.md` demonstrate direction but are not a
release gate because the before/after fixture bytes differ. Add either committed
fixtures or a checked-in deterministic generator plus an immutable fixture
manifest containing:

- generator version and command;
- exact byte size and structural counts for every fixture;
- Darkmatter frontmatter/body hash identities for Markdown fixtures;
- an xxHash whole-file identity through `biscuit-hash` where byte identity is
  required;

For each measurement, add a dated run record containing:

- baseline/candidate commits, commands, release profile, host facts, environment,
  and TTY mode;
- warm-up, sample count, statistic, dispersion, raw result locations, and
  predeclared improvement/no-regression thresholds.

The manifest is the authority for immutable fixture identity. Run records are
the authority for measurement context and link back to the manifest entries
they consume. A checkpoint-specific fixture must be registered and hashed
before that checkpoint captures its baseline.

For the historical closeout, build the before and after binaries from the
pinned commits, then run both against the same immutable fixture directory. The
"before" binary is the pre-optimization baseline `83aaecc8f` (the commit
`baseline.md` was captured from); the "after" binary is this feature's audit
commit `51c1f16e10ffe825b56987573ba4eabc659c768e`. `baseline.md` recorded fixture
sizes but **not** the fixture bytes or their hashes, and told re-runs to use
"any deterministic generator of the same sizes" — that missing byte identity is
the reproducibility hole this work item closes, so the manifest above must be
reconstructed (committed fixtures or a pinned generator) rather than trusted
from the prior capture. At minimum cover `md --help`, render, hash, trivial
compose, schema/transclusion compose, the three TOC size tiers, and the
code-heavy render cases. Do not use measurements from different hosts as a
pass/fail comparison.

Those pins reconstruct the accumulated 2026-07-12 result only. They are not the
baseline/candidate pair for changes implemented by this follow-up. Each new
optimization checkpoint must compare its immediate pre-change implementation
(or a saved same-source Criterion baseline) with its candidate on identical
input and harness bytes. Closeout also runs the complete manifest against the
final feature head so the cumulative result includes every follow-up change.

### 4. Finish Cross-Pass Compose Reuse (Findings 7 and 16)

The remaining duplication is not safely solved by copying preflight's composed
child body into the main pass: conditions, parent state, directive position,
and lifecycle decisions may change the result. The implementation must first
define a cache key or reusable intermediate whose identity contains every
semantic input.

Do not design this identity as a greenfield key. The current transclusion path
already combines `cache::hashing::options_hash(options)` with source, effective
state, context, and directive-overlay identities, and the result can drive both
run-local single-flight reuse and persistent cache reads/writes. Audit that
existing key before changing reuse boundaries. Its selected-field `Debug`-based
encoding is not the exhaustive canonical authority required by the linked
[Opaque Reference Graph](../_completed/2026-07-15-reference-graph/spec.md) feature.

Both consumers must derive from the shared field-classification authority in
[Architecture Decision B](#architecture-decision-b--shared-classification-purpose-specific-identities).
They must not share one undifferentiated fingerprint: graph provenance is a
conservative in-process compatibility comparison, while a compose cache key is
a purpose-specific output identity that may persist across processes.

Preferred design order:

1. share parsed source and reference metadata;
2. share context-independent prepared representations;
3. share fully rendered content only if a complete semantic identity can be
   demonstrated;
4. otherwise retain recomposition and record a same-fixture no-win result for
   narrower candidates.

The cache must be run-local or bounded, preserve condition-aware behavior, and
must not retain unrelated contexts, graphs, callbacks, or runtimes. Because the
transclusion phase composes children concurrently, any shared prepared-content
cache introduced here must be concurrency-safe (or partitioned per compose run);
a data race or a lock held across child composition is a correctness and
liveness regression, not just a performance one.

### 5. Reduce Frontmatter and Expression Rework (Findings 11–14)

Treat these as separate checkpoints even if they share fixtures:

- **F11:** extract each templated key's dependencies once, maintain unresolved
  dependency counts/reverse edges, and enqueue newly eligible keys. Avoid
  rebuilding the full seed state for each successful key where mutation can be
  made incremental.
- **F12:** allow evaluators and expression functions to borrow or cheaply share
  `ResolutionContext`. Preserve public owned-return APIs unless an explicit
  compatibility exception is approved.
- **F13:** benchmark an exact multi-pattern matcher against the current
  canonical rule order: descending key byte length, then ascending lexical
  order. Reject any design that changes left-to-right non-overlapping matching,
  the rule chosen at a shared start position, the fact that replacement output
  is not rescanned, UTF-8 character-boundary behavior, empty-key omission, or
  scalar-value coercion.
- **F14:** combine compatible discovery/emission work and construct output once
  per interpolation depth where practical. Nested interpolation still requires
  semantic fixpoint behavior; it does not authorize rescanning unrelated
  protected ranges.

Fixtures must include wide dependency graphs, deep dependency chains, cycles,
shell-pending keys, best-effort errors, many replacement rules, Unicode, code
fences, literal escapes, multiline indentation, and nested interpolation.

### 6. Remove Shell Polling and Policy Clones (Findings 17 and 32)

Body shell commands remain sequential. Optimize their wait mechanism without
changing source-order execution, timeout boundaries, captured output, process
cleanup, or error selection. Prefer blocking wait primitives or event-driven
notification available on all supported operating systems; any platform split
must be target-gated and tested.

For policy state, take one immutable stage snapshot or share immutable
collections. Do not hold a policy mutex across command execution. Tests must
show that all directives in a stage see the intended stable policy and that a
subsequent stage can observe an allowed policy update.

### 7. Finish Render and Cleanup Sub-items (Findings 23 and 25)

Resolve code theme and relevant environment inputs once at the start of a
render, then pass the snapshot to every code block. Separate render invocations
must still observe environment changes allowed by the existing contract.

For cleanup, first profile individual passes on representative documents.
Combine line passes only when their ordering and boundary behavior can be made
exactly equivalent. A no-win disposition is acceptable when the same-fixture
benchmark shows that fusion falls within noise or increases allocation/code
complexity without a repeatable end-to-end gain.

### 8. Restore Directory-Hash Semantics (Finding 22)

Remove the unconditional `node_modules`, `target`, and `vendor` exclusions so
the aggregate includes the same Markdown membership as before the performance
change. Add an end-to-end CLI test that freezes the aggregate, diagnostics, and
exit status for a tree containing those directory names.

The revert itself needs no hash-migration step: the exclusion change was never
released and there are no external consumers, so any aggregate computed under it
is a private working-tree artifact, not stored state to migrate. The migration
requirement applies only to a *future* opt-in ignore policy that changes
membership again. Such a policy may be proposed separately; changing the default
again requires owner approval and must explain how any then-stored aggregate
hashes migrate.

### 9. Complete Remote Discovery (Finding 33)

Retain the cheap no-HTTP guard. For documents that do contain remote
expressions, compute line positions in a single forward pass or through a
shared offset table rather than rescanning from byte zero for every expression.
Verify byte offsets at LF, CRLF, Unicode, start/end-of-file, and multiple
expressions on one line.

### 10. Complete Finding 35's Residual Items

The following remain independently open:

1. Compute `effective_state_hash` once per transclusion phase, not once per
   `::file` directive.
2. Build heading line offsets once and emit releveling spans/output without
   copying the whole child once per heading.
3. Store fetched response bodies as `Arc<str>` internally while preserving the
   current owned public facade where required.
4. Route `::toc-linking` target reads through one compose-run-owned source cache
   so one target is not read independently by graph discovery and composition.
   Separate graph and transclusion caches of the same type do not satisfy this
   item; preserve authoritative-read and invalidation behavior.
5. Within each mutually exclusive `md hash --diff` or `md hash --save`
   invocation, compute each unique `(kind, effective hash options)` artifact at
   most once and reuse it across that mode's comparison/planning and explanation
   output. `--save` may legitimately need separate stored-policy comparison and
   selected-policy baseline artifacts; do not collapse those distinct semantics.
   **Disposition (review-4): only partially achievable within invariant 2.** The
   intra-call reuse inside `explain_hash_diff` and `plan_hash_save` shipped and
   stands. The *cross-call* reuse between the CLI's comparison and explanation
   could reach `darkmatter-cli` only as new public API, so it was reverted; the
   CLI computes the artifact twice again. Full satisfaction requires a public
   pairing API and therefore an explicit owner compatibility decision, deferred.
   See the Finding 35 audit row.
6. Make `normalize_body_rhythm` avoid allocating an ANSI-stripped string for
   every output-line check.
7. Borrow link/image URL and title data through policy application, including
   the empty-policy fast path, while retaining owned public output nodes.

Each item needs its own behavioral tests and measurement disposition. Combining
them under one aggregate benchmark cannot conceal a no-win or regression in an
individual path.

## Compatibility and Correctness Invariants

1. Compose Markdown, validation results, rendered output, graph/CLI JSON,
   diagnostics, and exit status remain byte-for-byte and error-for-error
   compatible.
2. This follow-up introduces no new public Rust API shape change. It preserves
   the previously approved Finding 29 `Arc<Value>` exception and its owned
   compatibility facade; the opaque graph feature owns its own separately
   approved compatibility ruling.
3. The bare Sniff API's full NTP-reporting behavior is restored; Darkmatter's
   explicit local-only call remains.
4. Directory-hash membership returns to its pre-Finding-22 behavior.
5. Body shell directives execute in source order and retain observable side
   effects and failure ordering.
6. Cache identity includes every semantic input. Caches are bounded or
   run-local and safe under concurrent library use.
7. Internal borrowing and sharing must not weaken owned public facades unless
   a new compatibility exception is explicitly approved.
8. Implementations compile and behave on macOS, Linux, and Windows.

## Benchmark and Evidence Contract

Every optimization checkpoint must declare before measurement:

- the target operation and unaffected control groups;
- fixture identity and size;
- build profile, commands, environment, host, and TTY mode;
- warm-up, sample count, statistic, and dispersion;
- the minimum repeatable win and maximum permitted control regression.

Baseline and candidate must use identical source, fixture, and harness bytes
except for the code change under test. Retain raw samples. A local
microbenchmark establishes mechanism; an end-to-end command establishes user
impact. Findings with no repeatable improvement outside noise close through a
recorded no-win disposition and removal of unnecessary code.

When a required performance capture is deferred because the host cannot meet
its predeclared admissibility conditions, track the outstanding tasks and all
subsequent attempts or results in
[`performance-compliance.md`](./performance-compliance.md). The review that
identified the gap remains the historical assessment; later implementation
reviews should carry only the non-performance findings still requiring review.

For Markdown content identities, use Darkmatter's Markdown-aware hashing. Use
`biscuit-hash` xxHash for non-Markdown content or exact whole-file fixture
identity. Do not introduce ad hoc hashing implementations.

## Verification Matrix

| Work | Required verification |
|---|---|
| F1 Sniff correction | Sniff L1/lint; Darkmatter context tests and L1/lint; no-network compose proof |
| F2/F3/F21 terminal | Biscuit Terminal L1/L2/lint; Darkmatter CLI L1/L2/lint; OSC request count and interactive latency artifact |
| F4 closeout | TOC unit/property coverage; identical-fixture micro and CLI results; threshold report |
| F7/F16 cross-pass reuse | Reference, preflight, transclusion, condition, lifecycle, and cache-identity suites; compose benchmark |
| F11–F14 interpolation/replacement | Focused units plus compose integration and scale benchmarks |
| F17/F32 shell | Cross-platform L1 process/policy tests, timeout/stream-saturation/cleanup tests; L2 only if a real terminal or PTY is required |
| F22 directory hash | Library collector tests and end-to-end CLI aggregate/exit-status test |
| F23/F25 render/cleanup | Snapshot/golden output, headless Browser tests for F23, L2 terminal frames only where applicable, code-heavy render and cleanup benchmarks |
| F33/F35 residuals | Focused behavior tests and one target/control benchmark per sub-item |
| Feature closeout | `just build`/`just test`/`just lint` in every affected area selected by impact analysis; `just test-l2` only in areas with F2/F3/F21 PTY coverage; Darkmatter `just test-browser` for F23; exact root selectors where supported for cross-package changes; `cargo fmt --check`; `git diff --check` |

No write-mode formatter is authorized. Linux and Windows evidence must be
recorded before completion; macOS-only success is insufficient for the stated
cross-platform contract.

The cross-platform gate is targeted, not blanket. Findings whose code path is
genuinely OS-divergent **require** a real non-macOS behavioral run, not merely a
successful cross-compile: F17's shell wait primitive (blocking-wait vs.
event-driven notification differs by OS), the F2/F3/F21 PTY/L2 terminal helper
(Unix-only PTY, target-gated on Windows), and F22's directory traversal and path
handling. For findings that are OS-identical by construction (pure allocation,
scanning, caching, and hashing changes with no `cfg`-gated or filesystem-shape
branch), state that identity in the disposition and treat Windows compile
evidence plus the macOS behavioral run and the repository's ordinary Linux CI
as sufficient. Make that classification from the implementation actually
changed, not from the finding number: F12 can reach filesystem-backed expression
functions, for example, so Findings 5–14 are not categorically OS-identical.
This keeps the gate honest without demanding a per-finding Windows behavioral
run for code that cannot vary across platforms.

## Documentation Deliverables

- Add a dated correction/supersession notice to the old plan/results, linking to
  this feature's audit and final dispositions. Do not rewrite their original
  body or checkboxes: they remain the historical `codex/default` record. This
  feature's own dispositions, measurements, and manifests live in the
  feature-local evidence home defined by Architecture Decision A.
- Link the original review to this active follow-up and to the opaque graph
  feature.
- Record one disposition and evidence location for every retained partial,
  open, or correction item from the audit table, including evidence-only gaps.
- Document the restored Sniff and directory-hash compatibility behavior.
- Update public rustdoc and README material only where behavior or supported
  construction changes.

## Architecture Decisions

### Architecture Decision A — Feature-local evidence with focused runners

Create `results.md` beside this specification as the disposition and evidence
index. Store the immutable fixture manifest and either committed fixtures or the
pinned deterministic generator in a sibling `benchmarks/` directory. The
manifest is the single authority for fixture identity across all checkpoints;
dated run records under `benchmarks/raw/<checkpoint>/<run-id>/` own commands,
environment, host, samples, dispersion, thresholds, and raw-result locations.

One manifest does not imply one universal runner. Use the existing Criterion
recipes for library microbenchmarks, a release CLI runner for command-level
measurements, and the existing Biscuit Terminal probe/PTY path for interactive
terminal measurements. Each runner writes a dated run record linked from the
feature-local evidence index and consumes the shared manifest wherever it uses
file fixtures. Do not force CLI or PTY evidence through `just bench`, which is a
Criterion runner.

The 2026-07-12 review remains historical evidence. Add only dated
correction/supersession notices and cross-links there; do not rewrite its body,
checkboxes, or original measurements.

### Architecture Decision B — Shared classification, purpose-specific identities

Define one crate-private, exhaustive `ComposeOptions` field-classification
authority in the `ComposeOptions` owning module. The linked Opaque Reference
Graph feature owns the single prerequisite landing commit; this feature consumes
it and must not create a competing inventory. The authority destructures
`ComposeOptions` without `..` and requires every field to be classified when a
field is added. Both graph provenance and compose caching derive their own
identity products from that classification; neither maintains an independent
field inventory.

The derived products retain distinct contracts:

- `ReferenceGraphOptionsIdentity` is conservative and fail-closed. It may use
  weak/minimal instance handles for stateful callbacks or runtimes and may
  include fields irrelevant to rendered output.
- the compose-cache value fingerprint includes only canonical value semantics
  relevant to the cached artifact, combined with the existing source, effective
  state, context, directive-overlay, and pass-scope dimensions;
- process-local identity required by a stateful field participates only in
  run-local reuse. The run-local key must distinguish independently constructed
  stateful instances while remaining stable across clones of the same shared
  instance. Process-local identity bytes never enter a persistent key, and a
  key that requires them must not read or write a persistent cache entry;
- when equivalence cannot be established, reject reuse rather than guessing.

Canonical value encoding is typed and length-delimited, distinguishes `None`
from empty values, preserves ordered vectors such as `magic_paths` and
`env_path_whitelist`, sorts only genuinely unordered collections, and uses a
versioned domain marker. It must not use `Debug` output. A changed encoding uses
a new cache-key domain so legacy persistent entries cannot be read under the new
semantics. The implementation replaces or delegates the existing
`cache::hashing::options_hash`; it does not add a parallel third options
fingerprint. Selecting this shared authority requires the linked opaque-graph
specification and implementation to use the same field-classification contract
in the coordinated change.

## Acceptance Criteria

This feature is complete when:

1. Findings 1–4 and 21's compatibility/evidence gaps are closed.
2. Findings 7, 11–14, 16, 17, 23, 25, 32, 33, and every remaining Finding 35
   sub-item has an implementation or an allowed evidence-backed disposition.
3. Finding 22's unapproved membership change is reverted, unless the owner
   separately approves and documents a new compatibility exception.
4. The opaque graph feature owns all remaining Finding 18 correctness work,
   with no duplicated or conflicting implementation here.
5. Reproducible same-byte benchmark artifacts meet their predeclared
   thresholds and retain raw samples.
6. Behavioral, L1, requirement-matched L2, lint, workspace, formatting-check,
   and whitespace gates pass, with Linux and Windows evidence recorded.
7. The audit table and original review documentation reflect the final honest
   disposition of every finding.
8. Architecture Decisions A and B are implemented: evidence remains
   feature-local behind one fixture manifest and focused runners, while graph
   provenance and compose caching derive purpose-specific identities from one
   exhaustive `ComposeOptions` field classification.
