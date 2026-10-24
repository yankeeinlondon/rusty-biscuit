---
area: darkmatter
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-23
review_iterations: 3
completed: true
clarified: false
created: 2026-10-22
owner: Ken Snyder <ken@ken.net>
packages:
    - dmls
related:
    - 2026-10-22-dmls-watched-file-flood
human_review: true
needs_rulings: false
implemented: true
implemented_by: "claude/opus"
---

# DMLS: update the workspace graph incrementally

## Goal and scope

Decide whether updating only the affected parts of the live workspace graph
would materially reduce language-server CPU use or editor delays after
`2026-10-22-dmls-watched-file-flood`. Build an optimization only if the
remaining cost justifies its additional state and invalidation rules.

The first deliverable is a short measurement record and a decision. Phase 1
has run (see the measurement and decision record): the remaining cost is
real, it comes from open-buffer edits rather than watched-file events, and
[Phase 2](#phase-2-cheaper-rebuilds-for-content-edits) specifies the change.
This spec does not authorize a new graph representation.

## Current behavior

In the **dmls** package,
[WorkspaceIndex](../../dmls/src/graph/invalidate.rs) owns the file list,
open-buffer parses, lazily parsed disk targets, and the current immutable
[WorkspaceGraph](../../dmls/src/graph/arena.rs). Its private rebuild method
recreates the wiki candidate universe and the graph's nodes, edges, reverse
lookups, frontmatter-key lookups, and portability collisions.

Every listed Markdown file has a document root in the live graph, but closed
files are parsed only when needed. Referenced Markdown files outside the
list can also be parsed as dependencies. This is different from a graph
containing the parsed contents of every workspace file: references, renames,
and workspace symbols use the persistent index store and, when needed,
separate request graphs. See [workspace indexing](../../docs/lsp/indexing.md)
and [server architecture](../../docs/lsp/architecture.md).

The watched-file fix batches each notification into at most one rebuild and
ignores unlisted creations unless an unresolved reference is waiting for
that path. Two watched-event cases can still require a rebuild:

1. **A listed Markdown file joins or leaves the file list.** Graph document
   IDs are assigned in path order, so insertion or removal can change IDs
   throughout a newly built snapshot. Wiki matches and portability
   collisions can also change for documents whose text did not change.
2. **An unlisted Markdown file is created where a broken reference waits.**
   In dmls, [WorkspaceGraph::awaits](../../dmls/src/graph/arena.rs) checks the
   planned candidate paths recorded for unresolved references. A rebuild
   re-runs normal resolution; creation alone does not guarantee a successful
   match. The target initially remains a file outside the graph, but normal
   target preparation can later parse it as a dependency when needed.

Open-buffer edits and closes, changed lazy parses, listing installation,
wiki-root changes, context invalidation, and fallback re-resolution can also
rebuild. They must be distinguished from watched-file cases when measuring;
the optimization should address the cost actually observed.

> **Review note:** Stable document IDs and copy-on-write edge updates were
> previously presented as required solutions. Neither is an established
> requirement, and stable document IDs are now out of scope (see the ruling
> under Conditional implementation requirements). An unlisted creation can affect reference
> precedence and derived lookups as well as one edge. The design must first
> show which work can safely be reused. The existing snapshot-local ID
> contract and full rebuild remain the default.

## Phase 1: a bounded measurement

Use one host, a release build, and a quick sample of normal editing on this
repository's workspace (approximately 6,000 Markdown files; record the actual
listed and parsed counts). No OS-specific cost has been identified, so this
is not a cross-platform performance campaign. Do not add a larger synthetic
workspace, repeated sessions, or statistical acceptance criteria.

The existing watched-file fix already measured 200 ignored Markdown
creations in a 3,000-document workspace: roughly 8.3–8.9 seconds before the
fix and 52–53 milliseconds afterward, in debug builds. Its structural tests
also prove that unreferenced creations do not rebuild. That answers whether
ordinary build-output floods require incremental graph updates: do not
repeat that spike or use the old pre-fix CPU saturation as evidence of a
remaining problem. These figures do not establish the cost of a live graph
rebuild during editing.

### What to record

- **Remaining rebuild cost.** Time the actual dmls live-index rebuild,
  including the wiki universe and final snapshot construction. Keep parsing,
  discovery, store revalidation, and separate request graphs out of this
  duration, and identify them separately if they explain a perceived delay.
  Record listed files, parsed documents, nodes, and edges so the result is
  interpretable. Do not parse every listed file to set up the measurement.
- **Remaining rebuild frequency.** During the same short editing sample,
  count actual rebuilds by cause and record the sample duration. Count a
  batch's final rebuild once; a batch may have multiple contributing causes.
  The generation counter measures snapshot publications, not incoming
  notifications or time spent rebuilding. Temporary cause counters should
  not become a permanent telemetry subsystem.
- **Where a slow rebuild spends time.** Only if the sample shows a remaining
  problem, use a quick profile or temporary stage timings to distinguish
  wiki-universe construction, node/edge assembly, and final lookup creation.
  Consider a cheaper full rebuild before introducing incremental state.

In dmls, the current [benchmark tests](../../dmls/tests/l1/bench_index.rs)
and [benchmark implementation](../../dmls/src/bench.rs) measure index-store
cold fill and warm request behavior. They do **not** currently report live
rebuild assembly and finalization separately. An old stage-timing comment
in the graph assembly code is not evidence that this instrumentation exists.
Use a small temporary measurement seam in the live rebuild path if needed;
changing the public benchmark format is outside this phase.

Record the host/OS, revision and local changes relevant to the measurement,
release command or temporary seam, sample duration, counts, and observed
costs here. A future reader must be able to distinguish measured facts from
estimates. Do not log document text or heading contents.

### Decision rule

Retain the draft's approximate starting bounds: under **1% of one CPU
core** spent rebuilding during the sample, and under **50 ms per rebuild**
on this repository. These are decision aids, not timing assertions in CI
or a guarantee that every editor request completes in 50 ms.

CPU share is total CPU time spent rebuilding divided by elapsed sample time;
for example, 0.3 CPU-seconds in 60 seconds is 0.5% of one core. Multiplying
wall-clock rebuild duration by rebuild frequency estimates occupied time,
not CPU consumption. If only wall-clock data is available, label that
estimate and do not claim it proves the CPU bound.

- If both bounds are met and no remaining editor delay is attributable to
  rebuilding, retain full rebuilds and record why incremental state is not
  justified. Leave lifecycle closure to the author.
- If a cheaper full rebuild is likely to address the measured cost, specify
  that smaller change and its invalidation rules first.
- Otherwise, specify incremental work for the observed dominant cause.
  Prefer the unlisted-creation case only when it materially contributes;
  being smaller does not make it useful if it rarely occurs.
- If the short sample is inconclusive, retain full rebuilds: the outcome is
  the same as meeting both bounds. Record in the measurement record which
  figure was inconclusive and why. Do not expand the performance study or
  change the thresholds to force an answer.

### Measurement and decision record

Measured 2026-10-23. Everything below is measured unless marked as an
estimate.

**Setup.** Apple M4 Max (16 cores, 128 GiB), macOS 27.2, revision
`b49658f73`, `cargo build --release -p dmls`. The only local change to
dmls was a temporary seam, since reverted: `WorkspaceIndex::rebuild` took
`#[track_caller]` to attribute each rebuild to its calling method, deferred
callers inside a batch were attributed to the batch's one rebuild, and each
rebuild, assembly stage, and finalize stage printed its wall time and its
thread CPU time (`CLOCK_THREAD_CPUTIME_ID`) to stderr. A Python LSP client
drove the binary over stdio on this repository and logged no document text.
The client advertised dynamic watched-file registration. After startup
settled, it ran a scripted editing sample:

- it opened three documents and typed about 250 characters across them, at
  six characters per second, with pauses;
- it sent a hover and a completion request, and saved twice with matching
  watched-file change events;
- a listed Markdown file joined the list and later left it;
- 50 unlisted, unreferenced creations under `target/` arrived in one
  notification;
- it closed one buffer.

Server CPU was the process's `ps` CPU time across the sample.

**Counts.** 5,901 listed Markdown files; 1–4 parsed documents (1–3 open,
0 dependencies); 5,935–6,203 nodes; 30–426 edges. Startup published 2
snapshots, which are excluded below.

**Baseline, 99.3 s sample.**

| Cause | Rebuilds | CPU median | CPU total |
| --- | --- | --- | --- |
| open-buffer edit (`set_document`) | 311 | 43.0 ms | 13.04 s |
| context relink (`relink`) | 6 | 49.5 ms | 0.27 s |
| lazy target parse (`ensure_parsed`) | 2 | 41.1 ms | 0.08 s |
| listed join (batched `disk_changed`) | 1 | 43.6 ms | 0.04 s |
| listed leave (batched `disk_deleted`) | 1 | 44.3 ms | 0.04 s |
| buffer close (`close_buffer`) | 1 | 31.5 ms | 0.03 s |
| **all** | **322** | **42 ms** | **13.51 s** |

Rebuilding used 13.5 CPU-seconds in 99.3 s, **13.6% of one core**. The bound
is 1%. Rebuilding was 65% of the server's 20.6 CPU-seconds. Median wall time
per rebuild was 52 ms, just over the 50 ms bound. Wall-time outliers (up to
545 ms) had normal CPU (≈45 ms), so the rebuild thread was waiting, not
working. The 50 build-output creations caused no rebuild.

**Where one rebuild spends its time** (a second 97.5 s baseline run, CPU and
wall medians per rebuild):

| Stage | Median |
| --- | --- |
| wiki universe construction | 13.9 ms CPU |
| per-document canonical wiki path, recomputed during assembly | 10.9 ms |
| the rest of the per-document assembly loop | 6.1 ms |
| reference resolution | 1.5 ms |
| portability collisions | 9.4 ms |
| frontmatter-key lookup | 0.8 ms |
| reverse lookups | < 0.1 ms |

About 34 of the 42 ms depend only on the set of document paths and the wiki
roots, which an edit to an open buffer never changes. The cost scales with
the 5,901 listed files, almost all of them unparsed, not with the edited
text.

**Prototype of Phase 2, step 1, 96.8 s sample.** A temporary prototype,
since reverted, cached the step 1 state. Median rebuild CPU fell from
41.7 ms to **12.35 ms** (p90 14.7 ms), and median wall time to 14.3 ms.
Rebuild CPU totaled 4.23 s, **4.4% of one core**. Server CPU fell from
20.7 s to 12.6 s. Comparing the cached universe's inputs on each rebuild
cost 0.3 ms. The remaining time is per-document bookkeeping for unparsed
documents (7.6 ms), reference resolution (1.7 ms), key and portability
grouping (1.7 ms), and reverse lookups. With the prototype applied to
`b49658f73` and the timing seam removed, `cargo nextest run -p dmls` passed
1,269 tests (6 skipped). The prototype did not yet hold dependencies in the
cached universe; the sample had none.

**Decision.** Both bounds are missed. The dominant cause is open-buffer
edits: 97% of rebuilds and 97% of rebuild CPU. Membership changes
contributed two rebuilds and awaited creations none, so neither is
optimized. Under the decision rule, a cheaper full rebuild comes first. Step
1 of Phase 2 is that change, and its prototype meets the 50 ms bound. It
still misses the 1% bound, so step 2 reuses the remaining path-derived
document state on content-only rebuilds. Step 2 is the incremental work for
the dominant cause, and it keeps full assembly of every parsed document's
nodes and edges.

## Phase 2: cheaper rebuilds for content edits

Phase 2 addresses the measured cause: one full rebuild per open-buffer edit,
dominated by path-derived work over every listed document. Full rebuilds
remain the only way a snapshot is published, at most one per notification.
Document IDs stay dense and path-ordered, and a content edit cannot change
them, because the set of document paths is unchanged. Every other trigger,
including listing changes, wiki-root changes, relinks, and awaited
creations, keeps today's behavior apart from step 1's caching.

### Step 1: compute path-derived wiki state once per document set

- **Cache the live index's wiki universe.** `WorkspaceIndex` keeps the last
  `Arc<WikiUniverse>` with the inputs it was built from: the sorted candidate
  paths (listed files plus non-dependency parsed documents), the dependency
  paths, and the wiki roots. A rebuild whose inputs compare equal reuses the
  cached universe. Comparing inputs costs 0.3 ms on this repository and
  cannot miss an invalidation. Per-method dirty flags are rejected because a
  missed flag would silently serve stale wiki answers.
- **Hold dependencies in the cached universe.** The live index appends each
  dependency path to its universe as an unmatchable slot. Today assembly
  pushes those slots itself (`Arc::make_mut`), which would copy a shared,
  cached universe on every rebuild that has a dependency. Wiki matching is
  unchanged because an unmatchable slot never matches.
- **Store each slot's portability key.** `WikiUniverse` computes a slot's
  portability key once, when the slot is added. `portability_collisions`
  groups graph documents by their slots' keys and still considers only the
  graph's non-dependency documents, so a request graph over a larger
  universe reports the same collisions as today.
- **Take canonical paths from the universe.** Assembly takes a
  non-dependency document's canonical path and workspace ID from its
  universe slot instead of recomputing them. A universe slot is built with
  the same wiki roots as the assembly, so the values are identical. A
  dependency's record keeps its real canonical path, computed as today.
  A document missing from the supplied universe is computed and pushed as
  today. Request graphs keep that path.

### Step 2: reuse the document table on content-only rebuilds

When the graph's sorted document paths, dependency set, listing state, and
universe (the same `Arc`) all equal the previous build's, the new snapshot
shares the previous snapshot's path-derived document table instead of
rebuilding it. That table holds:

- each document's path, canonical path, workspace ID, dependency flag, and
  wiki slot;
- the path-to-document map and the slot-to-document map;
- the frontmatter-key lookup and the portability collisions.

The table moves behind an `Arc` so that sharing it costs a reference-count
increment. Per-snapshot document state (content hash and parsed flag) is
stored apart from the table, and readers keep using the same accessors.
When any input differs, the table is rebuilt from scratch.

To make document roots independent of parsed content, roots are numbered
first: document `d`'s root is `NodeId(d)`, and content nodes follow in
document order. Node IDs stay dense and snapshot-local, and per-document
node order is unchanged, so readers that filter nodes by document see the
same sequence. This is a numbering change inside one snapshot, not the
stable-identity representation the ruling below excludes. Heading facts are
kept only for parsed documents, and an unparsed document reads as having
none.

Parsed documents' nodes and edges are still assembled and resolved in full
on every rebuild. No edge, wiki resolution, or awaited path is carried from
one snapshot to the next.

### Acceptance

- **Equivalence.** For content edits, buffer opens and closes, listed joins
  and leaves, dependency transitions, and wiki-root changes, the snapshot
  produced through the caches equals a fresh full build over the same
  inputs. Compare paths, spans, resolutions, wiki outcomes, portability
  collisions, awaited paths, and provider answers, as Verification and
  delivery requires.
- **Work counts.** Deterministic tests count operations, not elapsed time:
  - a content-only edit computes no `WikiDoc::for_path` and no portability
    key for an unchanged document, and builds no new document table;
  - a listed join or leave, a wiki-root change, and a dependency transition
    each rebuild the universe and the table exactly once.
- **Re-measurement.** Re-run the Phase 1 sample (same host class, release
  build, same scripted session) and add the results to the measurement
  record. This is a record, not a gate: the bounds remain decision aids.
  Phase 2 ends with step 2 whatever the figure. If editing still exceeds
  the CPU bound, any further reduction, such as coalescing keystrokes or
  changing the representation, is new work with its own spec.

## Conditional implementation requirements

The next phase must name the measured cause it addresses, what is reused,
what invalidates that reuse, and when a full rebuild is still required.
A narrow optimization may retain full rebuilds for listing, configuration,
context, or wiki-root changes. It need not solve every trigger at once.

**Ruling (2026-10-23): membership changes keep the full rebuild.** A listed
Markdown file joining or leaving the file list rebuilds the whole graph, and
snapshots keep their dense, path-ordered document IDs. Stable document
identities, whether stored per document with explicit lookup maps or
remapped to dense IDs at publication, are out of scope for this spec. The
conditional phase may optimize another measured cause or make full-build work
cheaper, but it must not change the ID representation. If Phase 1 shows
membership changes are the remaining bottleneck, record that finding in the
measurement record; a representation change then needs its own spec, which
must define document and node lifetime rules, deterministic output ordering,
and wiki-slot mapping.

The ruling weighed three options. Stable per-document identities would avoid
renumbering on insertion but need deletion and reuse rules and proof that a
stale handle cannot name a replacement document, and node and edge work may
still dominate. Remapping stable identities to dense IDs at publication keeps
reader APIs but still touches every published reference, so it may keep most
of the rebuild's cost. Keeping full rebuilds preserves current storage,
ordering, and invalidation rules, at the cost that membership changes remain
proportional to workspace size.

Any implementation must preserve these contracts:

- **Immutable reader snapshots.** Readers retaining an old snapshot see its
  original nodes, edges, and lookups throughout an update. Publish a complete
  replacement atomically, with a new generation for changed graph state.
  In dmls, [NodeId](../../dmls/src/graph/node.rs) is meaningful only inside
  its owning snapshot; internal reuse must not imply cross-snapshot validity.
  Preserve at most one publication per watched notification and the existing
  no-rebuild behavior for irrelevant creations and unchanged content.
- **One resolution authority.** Reuse dmls's captured document contexts and
  existing reference resolver, which delegates file references to
  biscuit-file. Do not replace resolution with a direct path join or a claim
  that an event proves existence. Preserve candidate precedence, typed
  failures, and the distinction between a missing path and a probe error.
  Reuse cached path facts only while their wiki roots and contexts match.
- **Complete derived state.** Keep forward edges, reverse lookups, awaited
  candidate paths, heading and key lookups, wiki outcomes, and portability
  collisions consistent. Adding a wiki candidate can make a previously
  unique match ambiguous; removing one can make it unique. Reverse edges
  alone cannot identify every affected reference. An awaited-path membership
  set alone likewise does not identify which source references to re-resolve.
- **Current document ownership.** Preserve open-buffer precedence, lazy
  parsing and stale-parse handling, dependency exclusions from wiki
  candidates, and listing completeness used to suppress premature
  diagnostics. A cache of the wiki universe must account for non-dependency
  open documents outside the list as well as listed files.
- **Unchanged provider answers.** Preserve navigation, diagnostics and their
  source ranges, dependent-diagnostic refresh, completion, references,
  rename edits, and workspace symbols. Keep the persistent facts format and
  request-graph behavior unchanged unless the conditional design explicitly
  justifies an additional change. Do not introduce execution, remote fetching,
  or filesystem mutation into editor analysis.

### Verification and delivery

Compare the optimized graph with a fresh full rebuild over the same listed
paths, parses, open/dependency state, wiki roots, contexts, and filesystem
observations. Compare observable paths, source spans, resolutions, and
provider results rather than raw numeric IDs. The full builder remains the
reference implementation and fallback for unsupported update kinds.

In dmls, [wiki universe equivalence](../../dmls/tests/l1/wiki_universe_equivalence.rs)
and the [index-store oracle](../../dmls/tests/l1/index_store_oracle.rs) provide
patterns to reuse. A small request graph must use the same full wiki
candidate universe as the reference graph; comparing two equally incomplete
universes would miss ambiguity defects.

For the chosen update kind, cover its affected behavior: listed joins and
removals, awaited creations, wiki ambiguity changes, and open/dependency
transitions as applicable. Include an old-snapshot reader and a notification
with multiple changes. Keep unsupported cases on the full-rebuild path and
prove they still reach it. Add server-level coverage through the normal
notification/request path for behavior the optimization changes; direct
builder comparisons alone do not verify batching or diagnostic publication.

Use deterministic correctness and work-count assertions, not elapsed-time
limits in CI. Run the darkmatter area's `just test` and `just lint` for an
implementation; terminal or browser tests are unnecessary for graph-only
changes. Preserve portability on macOS, Linux, native Windows, and WSL2;
this correctness requirement does not expand the one-host performance sample.

If code changes land, update the relevant indexing and architecture topic
pages and skill guidance alongside them. Record departures from this design
in the implementation log. The author handles lifecycle completion after
review; an agent leaves the work ready for review.

## Open Questions

None.
