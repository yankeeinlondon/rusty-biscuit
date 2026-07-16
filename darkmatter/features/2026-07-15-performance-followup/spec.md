---
status: draft
reviewed: true
reviewed_by: claude/default
reviewed_on: 2026-07-15
created: 2026-07-15
source_review: ../../reviews/2026-07-12-perf/spec.md
source_assessment: ../../reviews/2026-07-12-perf/review-3.md
source_baseline: ../../reviews/2026-07-12-perf/baseline.md
source_results:
  - ../../reviews/2026-07-12-perf/results.md
  - ../../reviews/2026-07-12-perf/results-2.md
related:
  - ../2026-07-15-reference-graph
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
separate [Opaque Reference Graph](../2026-07-15-reference-graph/spec.md)
specification. This document retains Finding 18 only as audit history and does
not duplicate that feature's implementation.

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

| Finding | Status | Current implementation | Work retained here |
|---:|---|---|---|
| 1 | Partial | Darkmatter explicitly calls `detect_timezone_with_options(false)`, removing its unused NTP request. | Restore bare `sniff::detect_timezone()` to its compatible full-report behavior by delegating to `true`; retain Darkmatter's explicit `false` call. |
| 2 | Partial | OSC 10 text-color results are process-cached. | Add L2 proof that repeated terminal construction emits one OSC 10 request and record repeated-construction latency. |
| 3 | Partial | The compose CLI shares a terminal through a per-invocation `OnceCell`. | Add an end-to-end CLI case exercising verbose, performance, and warning-report branches and prove one detection per invocation. |
| 4 | Partial | TOC line lookup uses an offset table and binary search; compatibility tests cover line/span behavior. | Replace the non-comparable closeout with identical, hashed fixtures, predeclared thresholds, and retained raw samples. |
| 5 | Complete | Schema validation uses the already resolved effective schema rather than resolving and coercing twice. | No implementation work. Protect in the final benchmark and regression gates. |
| 6 | Complete | Coercion participates in validator-cache reuse instead of compiling uncached validators per union arm. | No implementation work. |
| 7 | Partial | Reference-graph reuse and safe preflight target reuse reduce repeated walks. | Design and measure compatible sharing of prepared/interpolated content across validate, preflight, and compose. |
| 8 | Complete | Validator/coercion/namespace caches are reused with the required identity inputs and bounded behavior. | No implementation work. |
| 9 | Complete | The built-in baseline schema uses shared process-cached ownership rather than repeated conversion and deep cloning. | No implementation work. |
| 10 | Complete | `ctx.*` lookup no longer clones the full context-values map for each access. | No implementation work. |
| 11 | Open | The frontmatter interpolation fixpoint still repeatedly extracts references and clones maps/values as keys become eligible. | Parse dependency information once, maintain incremental readiness, and avoid rebuilding seed maps while preserving cycles, shell deferral, best-effort propagation, and key-scoped errors. |
| 12 | Open | Expression functions still receive an owned `Option<ResolutionContext>`, cloning its context for repeated calls. | Add an internal borrowed/shared path while retaining the owned public facade where compatibility requires it. |
| 13 | Open | Text replacement still scales with document length times rule count and builds a character-index vector. | Implement and benchmark a faster exact matcher or record a requirement-matched no-win result; preserve rule order, overlap, Unicode, and replacement semantics. |
| 14 | Partial | Literal conversion now skips its scan when `{{{` is absent. | Reduce repeated Markdown-aware scans and full-body copies when interpolation is present; benchmark nested and no-expression cases separately. |
| 15 | Complete | Parent headings and line offsets are parsed once and queried through memoized/indexed structures. | No implementation work. |
| 16 | Partial | Some graph/preflight data is shared through `Arc`, but visited documents may still be composed again. | Solve the remaining condition-aware prepared-content duplication without reusing bodies whose output depends on parent state or directive position. |
| 17 | Partial | Parallel body-shell execution was correctly rejected because commands must retain source-order side effects. | Replace or avoid the independent 10 ms completion polling loop and prove unchanged timeout, output, and failure semantics. Arbitrary directive parallelism remains prohibited. |
| 18 | Complete / Separated | Graph construction is reused and fragment slug lookup is memoized. | No performance work. Document/options/mode identity and graph opacity belong exclusively to the linked `ReferenceGraph` feature. |
| 19 | Complete | Protected-range parsing is gated behind a plausible delimiter scan. | No implementation work. |
| 20 | Complete | Text events without disclosure directives retain their borrowed/event representation instead of being unconditionally reallocated. | No implementation work. |
| 21 | Partial | The macOS appearance probe is cached and gated away from non-TTY paths. | Verify it together with Findings 2 and 3 in real-terminal L2 coverage; piped CLI timing is insufficient. |
| 22 | Open / Correction | Directory hashing now unconditionally excludes `node_modules`, `target`, and `vendor`, changing aggregate membership. | Restore prior membership. A future exclusion policy requires a separately approved compatibility ruling, migration semantics for persisted hashes, and an end-to-end aggregate/exit-status test. |
| 23 | Partial | Syntect themes are borrowed instead of deep-cloned per code block. | Resolve environment/theme choice once per render snapshot rather than reading it per block; retain dynamic behavior across separate renders. |
| 24 | Complete | Code-block emission writes directly into the output buffer instead of allocating per-token formatted strings. | No implementation work. |
| 25 | Partial | Four placeholder replacements are fused into one scan. | Measure and, when beneficial, combine compatible ordered line-based cleanup passes; preserve exact pass ordering and canonical output. |
| 26 | Complete | Validator cache identity uses the repository's fast hashing path rather than repeated SHA-256 work. | No implementation work. |
| 27 | Complete | Named-type namespace reads/parses are memoized and `@this` avoids rebuilding equivalent data. | No implementation work. |
| 28 | Complete | Example target validation and file work reuse the resolution/cache machinery. | No implementation work. |
| 29 | Complete | Effective schemas use shared `Arc<Value>` ownership; built-in baseline paths avoid deep clones. `results-2.md` contains a same-source A/B comparison. | Preserve the approved public ownership exception and its owned compatibility facade. |
| 30 | Complete | `doc.*` lookup walks effective state by reference and clones only the selected result. | No implementation work. |
| 31 | Complete | Variable interpolation stringifies the first lookup result rather than performing the lookup twice. | No implementation work. |
| 32 | Open | Each shell directive still clones read-only policy rule collections into a snapshot. | Snapshot once per stage or use safe shared read ownership; preserve the policy state seen by the stage and avoid holding locks while executing commands. |
| 33 | Partial | Remote discovery skips the expensive scan when no HTTP marker exists. | Replace per-expression prefix rescans for line numbers with one forward offset table/pass and measure remote-heavy input. |
| 34 | Complete | Cleanup change detection no longer clones both full bodies solely to compare them. | No implementation work. |
| 35 | Partial | The `md delta` full-document clones were removed. | Complete or disposition the seven remaining copy/hash/read sub-items listed below. |

Audit totals are 17 complete findings, 13 partial findings, and 5 open or
correction findings. Finding 18 is counted as complete for its performance
portion and separated for its correctness portion.

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

- a source-path test proving Darkmatter selects `false`;
- Sniff tests proving the bare API selects `true` and the configurable API
  respects both values;
- no live network dependency in ordinary Darkmatter compose tests;
- Sniff and Darkmatter L1/lint gates.

### 2. Build Requirement-Matched Terminal Evidence

Findings 2, 3, and 21 share one verification gap. Add a checked-in L2 helper
that runs under a supported real PTY and can observe OSC requests without
depending on a user's shell theme.

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
fixtures or a checked-in deterministic generator plus a manifest containing:

- generator version and command;
- exact byte size and structural counts for every fixture;
- Darkmatter frontmatter/body hash identities for Markdown fixtures;
- an xxHash whole-file identity through `biscuit-hash` where byte identity is
  required;
- commands, release profile, host facts, TTY mode, warm-up, sample count, and
  raw result locations;
- predeclared improvement and no-regression thresholds.

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
[Opaque Reference Graph](../2026-07-15-reference-graph/spec.md) feature.

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
- **F13:** benchmark an exact multi-pattern matcher against the current ordered
  rules. Reject any design that changes first-rule precedence, overlap,
  cascading behavior, Unicode indices, or empty-pattern handling.
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
4. Route `::toc-linking` target reads through the run cache so one target is
   not read independently by graph discovery and composition.
5. Reuse one document hash computation across `md hash --diff` and `--save`,
   including explanation output, without changing stored hash semantics.
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
2. The existing Finding 29 `Arc<Value>` exception remains the only public Rust
   API shape change in this follow-up. The opaque graph feature owns its own
   separately approved compatibility ruling.
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
| F17/F32 shell | Cross-platform process/policy tests, timeout and cleanup tests, L1/L2 where a real process is required |
| F22 directory hash | Library collector tests and end-to-end CLI aggregate/exit-status test |
| F23/F25 render/cleanup | Snapshot/golden output, L2 terminal frames where applicable, code-heavy render and cleanup benchmarks |
| F33/F35 residuals | Focused behavior tests and one target/control benchmark per sub-item |
| Feature closeout | `just test`, `just test-l2`, and `just lint` in every touched area; root recipes for cross-package changes; `cargo check --workspace`; `cargo fmt --check`; `git diff --check` |

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
- Record one disposition and evidence location for every open sub-item.
- Document the restored Sniff and directory-hash compatibility behavior.
- Update public rustdoc and README material only where behavior or supported
  construction changes.

## Architecture Decisions

### Architecture Decision A — Feature-local evidence with focused runners

Create `results.md` beside this specification as the disposition and evidence
index. Store the fixture manifest and either committed fixtures or the pinned
deterministic generator in a sibling `benchmarks/` directory. The manifest is
the single authority for fixture identity across all checkpoints.

One manifest does not imply one universal runner. Use the existing Criterion
recipes for library microbenchmarks, a release CLI runner for command-level
measurements, and the checked-in PTY/L2 helper for interactive terminal
measurements. Each runner records its commands, raw samples, and environment in
the feature-local evidence index and consumes the shared manifest wherever it
uses file fixtures. Do not force CLI or PTY evidence through `just bench`, which
is a Criterion runner.

The 2026-07-12 review remains historical evidence. Add only dated
correction/supersession notices and cross-links there; do not rewrite its body,
checkboxes, or original measurements.

### Architecture Decision B — Shared classification, purpose-specific identities

Define one crate-private, exhaustive `ComposeOptions` field-classification
authority in the `ComposeOptions` owning module. It destructures
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
  run-local reuse. A key that depends on pointer/instance identity must not read
  or write a persistent cache entry;
- when equivalence cannot be established, reject reuse rather than guessing.

Canonical value encoding uses field names, type boundaries, sorted unordered
collections, and a versioned domain marker. It must not use `Debug` output. The
implementation replaces or delegates the existing
`cache::hashing::options_hash`; it does not add a parallel third options
fingerprint. Selecting this shared authority requires the linked opaque-graph
specification and implementation to use the same field-classification contract
in the coordinated change.

## Acceptance Criteria

This feature is complete when:

1. Findings 1–4's compatibility/evidence gaps are closed.
2. Findings 7, 11–14, 16, 17, 23, 25, 32, 33, and every remaining Finding 35
   sub-item has an implementation or an allowed evidence-backed disposition.
3. Finding 22's unapproved membership change is reverted, unless the owner
   separately approves and documents a new compatibility exception.
4. The opaque graph feature owns all remaining Finding 18 correctness work,
   with no duplicated or conflicting implementation here.
5. Reproducible same-byte benchmark artifacts meet their predeclared
   thresholds and retain raw samples.
6. Behavioral, L1, L2, lint, workspace, formatting-check, and whitespace gates
   pass, with Linux and Windows evidence recorded.
7. The audit table and original review documentation reflect the final honest
   disposition of every finding.
8. Architecture Decisions A and B are implemented: evidence remains
   feature-local behind one fixture manifest and focused runners, while graph
   provenance and compose caching derive purpose-specific identities from one
   exhaustive `ComposeOptions` field classification.
