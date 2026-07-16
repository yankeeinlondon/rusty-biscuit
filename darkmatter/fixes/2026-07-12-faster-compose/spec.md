---
clarified: claude/opus-4.8
status: draft
---

# Faster Compose — Demand-Driven Context Capture

`ComposeOptions::new()` eagerly runs a full `ComposeContext::capture()` before
the document being composed is known. That capture requests every runtime
context group—repository and git state, file changes, language and document
scans, OS information, hardware, and GPU detection—even when the document never
references `ctx.*`.

Demand-driven capture already exists through
`ComposeContext::capture_for_content` and `capture_for_document`. The Darkmatter
CLI already uses `capture_for_document` and passes the result through
`ComposeOptions::new_with_context`; the unresolved default-path problem is
primarily the public library constructor and consumers that still call it.

This fix will make the default library compose path avoid unrequested context
groups without weakening `ctx.*` correctness. Process-level host caching and
filesystem scan bounds are related opportunities, but they are independent
workstreams and cannot be counted as part of the core fix unless separately
measured and completed.

> **Pre-review correction (2026-07-15):** The original draft treated historical
> CI timeouts as current evidence, described the no-`ctx.*` fallback as zero-I/O,
> assumed the whole Hardware group was process-stable, and implied that the CLI
> still used the eager default. Those statements are no longer accurate. This
> revision records the design and measurement decisions that must be resolved
> before implementation.

## Motivation and Current Evidence

The issue originally surfaced when several compose-heavy tests exceeded
nextest's termination ceiling on a large Linux CI working tree. Tests that
constructed `ComposeOptions::new()` repeatedly were especially sensitive, and
some were changed to use an explicitly cheap context as a local workaround.

Those observations remain useful historical evidence, but they are not a
current baseline. The complete Darkmatter Level 1 and Level 2 suites pass on the
post-performance-review branch, and the performance review removed the network
NTP request from the always-on DateTime group. Before this fix is implemented,
the affected tests and production paths must be remeasured on the current branch
to establish whether the remaining eager capture cost is still material and
which context groups dominate it.

The expected production benefit is still plausible: library callers that use
`ComposeOptions::new()` currently pay for full capture at construction time,
regardless of document content. Plausibility is not completion evidence; the
saved baseline defined below is required before code changes begin.

## Current Architecture

- `ComposeOptions` stores a concrete `ComposeContext`.
  `ComposeOptions::new()` calls `ComposeContext::capture()`, while
  `new_with_context` and `with_context` accept an explicitly prepared context.
- `ComposeContext::capture()` captures every `ContextGroup` relative to the
  process working directory at construction time.
- `capture_for_content` scans one string for `ctx.*` references;
  `capture_for_document` scans serialized frontmatter plus the Markdown body.
  Both always include DateTime.
- The no-reference path avoids network, git, repository, document-tree, and
  hardware probes. It still performs local clock work, a cheap OS timezone read,
  and environment capture, so it is not literally zero-I/O.
- The Darkmatter CLI already captures demand-driven context from the loaded root
  document and shares it between validation and compose.
- The compose pipeline consumes `options.context()` and child transclusion
  pipelines inherit that context. It does not currently discover or add context
  groups required only by a nested local or remote document.
- `ContextGroup::Hardware` contains both stable values (`cpu_arch`, core count,
  total memory) and dynamic values (`memory_used`, `memory_avail`). The complete
  group cannot be cached indefinitely without changing observable behavior.

## Scope and Delivery Contract

This specification contains three independently measured workstreams:

1. Demand-driven behavior for the default library compose path—the core fix.
2. Caching of context facts proven safe to snapshot for the process lifetime.
3. Bounds for expensive repository and filesystem scans.

Each workstream and each separately identified sub-item must have its own
baseline, implementation status, and checkpoint. A safe subset does not complete
the rest of its workstream. Complexity, blast radius, regression risk, or an
assertion that a path is uncommon can determine sequencing but cannot close an
item.

An item can close without implementation only when a requirement-matched
benchmark shows no repeatable benefit outside harness noise, a proposed approach
is explicitly rejected for violating a correctness invariant, or the repository
owner approves moving it to a linked active or unscheduled spec. An approved
deferral is a scope decision, not delivered work.

## Workstream 1 — Demand-Driven Default Compose

### Required outcome

- Constructing default compose options must not trigger git, repository,
  document-tree, language, OS, hardware, or GPU detection.
- Before a default compose evaluates `ctx.*`, it must have every context group
  required by the logical compose operation.
- `new_with_context` and `with_context` remain explicit-context paths. A supplied
  context must not be silently replaced or augmented unless a separately named
  API makes that behavior explicit.
- Demand-driven state must resolve per compose invocation. Reusing or cloning
  default options across different documents must not make the first document's
  captured groups sticky.
- Validation, preflight, and terminal compose passes that belong to one logical
  operation must observe one consistent context snapshot.
- Recursive local and remote documents must not resolve a `ctx.*` value as empty
  merely because the root document did not reference its group.

The implementation may use an internal context policy or deferred state, but the
public behavior decisions below must be approved first. The implementation must
not expose a run-local cache or mutable capture primitive merely to avoid an
internal refactor.

### Blocking compatibility decisions

Review cannot approve this workstream until the following are explicit:

1. **Pre-compose accessor:** Define what `ComposeOptions::context()` returns when
   called after `ComposeOptions::new()` but before a document is composed.
2. **Capture instant:** Decide whether DateTime and environment values represent
   options construction, the first compose entry, or another documented point.
3. **Directory anchor:** Decide whether deferred capture is rooted at the working
   directory observed during options construction, the compose source, or the
   working directory at compose time. A later ambient `current_dir` change must
   not silently change resolution.
4. **Options reuse:** Define behavior when one default options value or its clones
   are used for multiple documents with different context requirements.
5. **Nested documents:** Decide whether child documents extend one root-anchored
   snapshot or receive document-relative context. The decision must preserve or
   explicitly revise existing transclusion semantics.
6. **Generated references:** Define whether context references introduced by
   replacement, interpolation, shell output, or external state are supported by
   pre-scan capture. If supported, the design must discover them without an
   unbounded recapture loop.
7. **Compatibility ruling:** Changing when and how `new()` captures context is a
   public semantic change even if Rust signatures remain source-compatible. The
   approved exception and migration guidance must be recorded before landing it.

### Impact boundary

A 2026-07-15 GitNexus analysis classified changing `ComposeOptions::new()` as
**CRITICAL** impact: 12 direct dependents, 98 affected symbols within three
levels, four execution-flow families, and 13 modules including Claudine. Rerun
impact analysis immediately before editing because the index and callers may
change while this spec is unscheduled. Review direct callers before selecting an
API shape, and run `detect_changes` before commit.

## Workstream 2 — Process-Snapshot Host Facts

This workstream is optional until its own baseline demonstrates a repeatable
benefit after Workstream 1. It must not cache a whole `ComposeContext`, because
that would also snapshot time, environment, directory-sensitive state, and
mutable repository information.

Candidate facts must be classified individually:

- CPU architecture, logical core count, total memory, and OS identity are likely
  safe process snapshots, subject to verification of their `sniff` inputs.
- `memory_used` and `memory_avail` are dynamic and must remain live unless an
  explicit behavior change approves snapshot semantics.
- Package-manager detection can change during a long-running process.
- GPU inventory can change through hot-plugging or driver/device changes.

If caching remains worthwhile, cache only the proven-stable raw facts or define a
bounded refresh policy. Record how cache hits affect capture diagnostics and
timings. The cache must be thread-safe, bounded by its finite fact set, and
independent of the current directory and environment.

## Workstream 3 — Repository and Filesystem Scan Bounds

This workstream is also optional until independently measured. Excluding
directories can change public `ctx.*` values, including document inventories,
language detection, package metadata, and repository summaries; it is not an
automatically transparent optimization.

Before implementation:

- Identify which `sniff` walker owns each expensive scan and whether it already
  applies ignore rules.
- Define the exact exclusion set. Do not infer that every underscore-prefixed or
  vendored directory is semantically irrelevant.
- Record the before/after effect on every affected `ctx.*` property.
- Obtain an explicit compatibility ruling for any membership change.
- Prefer a shared, configurable `sniff` policy over Darkmatter-specific duplicate
  walking logic when the semantics belong to host/repository discovery.
- Add macOS, Linux, and Windows fixtures for separators, hidden directories,
  symlinks/junctions, and repositories containing authored Markdown inside a
  candidate excluded directory.

Broader `sniff` performance work that does not contribute to a measured
Darkmatter path belongs in its own package-area spec.

## Baseline and Benchmark Contract

The baseline phase must complete before implementation and save a named
Criterion baseline from the exact benchmark source that will measure the final
change. It must include:

- `ComposeOptions::new()` construction, measured separately from compose.
- Default compose with no `ctx.*` reference.
- Frontmatter-only and body-only references.
- One case for each context group, with expensive groups separable in results.
- A large `ctx.*`-free document to measure scan overhead.
- Repeated compose and multi-document options-reuse cases.
- A root document with no reference whose nested local child requires one group.
- An explicit `new_with_context` control that must remain behaviorally unchanged.

Committed fixtures must be deterministic and record byte sizes plus Darkmatter
frontmatter/body hashes. Host-sensitive end-to-end measurements must record the
OS, hardware, repository size/state, TTY mode, warm-up, sample count, and
dispersion; results from different hosts are not point-comparable pass/fail
evidence.

The baseline report must declare numeric target and regression thresholds before
implementation. Targeted cases must improve repeatably beyond harness noise, and
unaffected controls must show no statistically credible regression. Historical
CI timeouts and measurements from the earlier performance review cannot replace
this same-source baseline.

## Correctness and Validation

### Mechanism tests

- Instrument capture selection so tests assert which context groups and `sniff`
  probes ran. Do not infer zero-I/O from an empty timing vector.
- Prove a `ctx.*`-free default compose performs no network, git, repository,
  document-tree, language, OS, hardware, or GPU probe.
- Prove each known `ctx.*` key selects its required group from frontmatter and
  body content without selecting unrelated groups.
- Preserve diagnostics for partial capture failures and define diagnostics on a
  cache hit if Workstream 2 lands.

### Semantic regression tests

- Explicit contexts supplied through `new_with_context` and `with_context` remain
  authoritative.
- Options reuse across documents does not leak captured groups or values.
- Validation, preflight, compose, and recursive transclusion observe the approved
  snapshot and directory-anchor semantics.
- Local and remote child documents that uniquely reference Repo, OS, Hardware,
  Documents, Languages, FileChanges, GPU, Agent, and DateTime resolve correctly.
- Delayed composition and process-directory changes exercise the approved capture
  instant and base-directory behavior.
- Values that are intentionally dynamic remain live across captures.

### Performance and package gates

- Rerun the historically slow preflight acceptance tests on the current branch
  and record their distributions rather than only whether they stay below a
  timeout ceiling.
- Run `just test`, `just test-l2`, and `just lint` for Darkmatter.
- Because the constructor reaches Claudine flows, run the corresponding Claudine
  package tests and checks selected by impact analysis.
- Run build, test, and lint recipes for every package area selected by impact
  analysis. If Workstream 2 or 3 changes `sniff`, include its package-area
  gates; do not replace this scoped matrix with a workspace-wide Cargo check.
- Establish macOS execution plus Linux and Windows CI compile/test evidence. A
  macOS-only pass cannot close the cross-platform contract.

## Relationship to the Performance Review

The 2026-07-12 performance review removed Darkmatter's live NTP request and
strengthened benchmark reproducibility, compatibility, and finding-disposition
rules. Its statement that demand-driven context capture is a good existing
pattern applies to explicit content/document capture paths, including the CLI;
it does not mean `ComposeOptions::new()` is already demand-driven.

This fix must use the performance review's fixture-identity and completion rules.
It must not count a host cache, scan exclusion, or test timeout improvement as a
substitute for proving the default library constructor and compose behavior.

## Out of Scope

- Reopening unrelated performance-review findings.
- Caching repository, file-change, document, language, DateTime, environment, or
  other mutable context as process-global state.
- Changing `ctx.*` output membership through scan exclusions without an explicit
  compatibility ruling.
- General `sniff` optimization not supported by a measured Darkmatter path.
- Removing the existing test-side `capture_for_content` workaround merely for
  cleanup; it can be reconsidered after the default path is proven cheap.
