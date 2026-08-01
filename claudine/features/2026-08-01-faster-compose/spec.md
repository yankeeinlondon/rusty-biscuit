---
area: claudine
status: draft
created: 2026-08-01
packages:
    - darkmatter
    - sniff
    - claudine
    - claudine-cli
related:
    - ../../fixes/2026-08-01-propagated-context/spec.md
    - ../2026-07-13-file-resolution/spec.md
review_iterations: 0
---

# Faster compose: reduce the work one composition performs

## Outcome

Composing a Markdown document performs work proportional to what the document
actually contains. A document with no links pays no link-resolution parses; a
document with no shell directives pays no shell error-context construction; a
stage with nothing to do allocates nothing. Repeated inputs — the same schema
file, the same trigger roots, the same canonicalized path, the same host
evidence — are read and computed once per invocation and reused.

The companion propagated-context fix eliminates *redundant context discovery*
across Claudine's startup layers. This feature eliminates *redundant work
inside the composition itself*: the Darkmatter pipeline, the Claudine
prepare/execute flow that drives it, and the Sniff structure probe beneath
both. The two changes are complementary and independently landable, but this
feature assumes the fix's invocation-scoped context owner exists and builds
its API seams to accept that owner's evidence.

None of claudine, darkmatter, or sniff has production users yet. Where a
breaking API change buys a structurally better contract — an explicit compose
session, a demand-driven default context, tier-gated repository detection —
this specification prefers the break now over carrying compatibility weight
indefinitely.

## Motivating evidence

A source audit of the compose path (2026-08-01, single host, rusty-biscuit
monorepo of ~74 workspace members and ~10.5k tracked files) found the
following structural costs. These are code-level counts, not wall-clock
estimates, and they compound with the sequence and fleet workloads Claudine
runs.

Darkmatter pipeline, per compose of one document:

- Link resolution runs ten independent full-document parses (eight MDAST
  parses for the HTML extractors plus two pulldown parses for inline links
  and images), and link normalization at the root repeats all ten — roughly
  twenty full parses and twenty `LineIndex` builds per document even when the
  document contains no links at all.
- `source_context_for_errors` copies the entire body into a fresh `Arc` and
  canonicalizes the source path on every call; the full-source variant also
  reconstructs frontmatter plus body into a new `String` first. These run
  unconditionally at several pipeline boundaries and inside per-directive
  loops, including before the shell stages' empty-directive early-outs.
- The no-op paths of text replacement and interpolation each allocate a full
  body copy that the caller discards; inline cleanup hashes the body twice
  and rebuilds it even when nothing changed.
- Transclusion recomputes per-directive what is loop-invariant per phase:
  the expression resolution context for `when:` guards, `options_hash`
  (which re-canonicalizes external state and re-serializes the entire
  context value map and environment per directive), a `canonicalize` of the
  same child path up to three times, and a double pass over the directive
  slice for file versus code kinds. The shared `PipelineRuntime` sits behind
  one mutex that every parallel child locks twice.
- Code transclusion reads the target file before consulting the compose
  cache, so a cache hit still pays the read.
- The interpolation depth loop re-scans the entire document once per nesting
  level rather than only the rewritten regions.
- `ComposeOptions::new()` and the `EffectiveStateBuilder` fallback trigger
  the full ten-group ambient capture — Git, repository structure, docs scan,
  OS, hardware, and GPU probes — regardless of what the document references.
  Every context construction additionally snapshots the entire process
  environment, and the docs group is captured serially on the coordinating
  thread while the other probes run spawned.

Cross-document (sequence, fleet, and repeated-invocation) costs:

- A `$schema` file is read from disk and parsed per document; only the
  compiled validator is cached, and only after the merged JSON is rebuilt.
  A 500-step sequence pointing at one schema file re-reads and re-parses it
  500 times. Named-type imports have the same shape.
- Schema-trigger discovery re-walks its roots and re-reads every trigger
  file per document; the registry is reused only across the two passes of a
  single compose.

Claudine drive path, per document:

- The prepare flow composes the document twice end to end: once as a
  discovery pass for shell-approval collection (with deferred schema
  verdicts) and once as the canonical staged pass.
- Source resolution reads the document file twice — once for original text
  and once again through the Markdown constructor — and the reload path used
  by sequence JIT steps and post-initialize stabilization repeats the same
  double read.
- The runtime context for the same document is captured twice with identical
  arguments during preparation, and a third near-identical capture runs for
  the lifecycle context.
- `FORCE_COLOR`/terminal detection is probed four separate times in one
  command path.

Sniff structure probe (`RepoRequest::structure()`), per call:

- The nested-workspace-marker walk enumerates every non-ignored file in the
  repository serially, while the sibling shared-view walk is already
  parallel.
- Effectively every manifest in the tree is fully TOML/JSON-parsed: nested
  candidate detectors parse before checking for the workspace table, and
  per-seed name/version resolution parses member and root manifests
  unconditionally in every tier.
- Workspaces that declare membership globs trigger additional serial
  whole-tree walks (twice per Cargo workspace: members and exclude).
- A PATH-wide executable scan runs on every successful detection in every
  tier.
- `RepoInfo::package_for_dir`/`area_for_dir` rebuild the package-ownership
  index — including one `canonicalize` syscall per package — on every
  lookup, though internal `_with_index` variants already exist.
- `detect_area` runs full repository detection (inventory, dependencies,
  test runners) to produce a single package-area string that the structure
  tier already supplies.

## Relationship to the propagated-context fix

The fix owns: invocation-scoped context capture, per-invocation repository
topology memoization, `FileResolutionContext` propagation into every
Claudine-created `ComposeOptions`, harness eligibility separation, and
performance attribution for the startup stages. Its acceptance criteria are
prerequisites here, not duplicated obligations.

This feature owns: the cost of the composition work itself once context is
propagated, the Darkmatter session seam that carries both the propagated
evidence and the invocation-scoped caches, the consolidation of Claudine's
duplicate compose/read/capture passes, and the cost of a single Sniff
structure probe.

Where the fix says "probe at most once," this feature says "make the probe
and everything after it cheap."

## Required design

### F1 — One parse serves all reference extraction

Reference extraction must stop re-parsing the document per extractor. Link
resolution and link normalization each build one shared parse product (MDAST
and/or pulldown event stream plus one `LineIndex`) and run every extractor
against it. If the body is unchanged between the link-resolve and
link-normalization stages, the parse product may be reused across them;
staleness is decided by the existing content-hash discipline, not by trust.

Bound: composing a document performs a number of full-document parses that is
independent of the number of extractor kinds. A linkless document performs no
extraction-driven parse beyond the shared one that discovers it has no
references.

Changing the signature or ownership model of the internal reference-extractor
APIs is acceptable. The extracted reference records, their spans, and their
provenance must remain byte-identical to current output.

### F2 — No-op stages are near-free

- A stage that changes nothing must not allocate a body copy. The
  replacement and interpolation fast paths return a borrowed/unchanged
  signal (for example `Cow` or an explicit no-change variant) instead of
  `to_string()` copies the caller discards.
- Inline cleanup must not hash the body twice and rebuild it when no cleanup
  target exists.
- `source_context_for_errors` and `full_source_context_for_errors` become
  lazy: the body `Arc`, the frontmatter reconstruction, and the path
  `canonicalize` are deferred until an error actually needs them, or the
  context is built once per compose and shared. The shell-expansion and
  shell-block stages must hit their empty-directive early-outs before any
  error-context construction.
- Error rendering quality must not regress: when an error does occur, the
  rendered block carries the same source excerpt, path, and span data it
  carries today.

### F3 — Loop-invariant work is hoisted; canonicalization is deduplicated

- The per-directive `when:` expression resolution context, `options_hash`
  (including `classify_options` and the context/environment fingerprint),
  and the transclusion options clone are computed once per transclusion
  phase, not once per directive.
- The link-normalization env-path whitelist (env-var read plus
  `canonicalize`) is resolved once per stage, not once per record.
- One request-scoped canonicalization memo serves the whole pipeline: a
  path canonicalized anywhere in a compose (resolver, cache keying, error
  contexts, normalization) is canonicalized once. The memo lives on the run
  cache, not in a process-global.
- The transclusion prepare step walks the directive slice once, partitioning
  by kind, instead of filtering the full slice per kind.

### F4 — Transclusion runtime contention and cache-hit purity

- The shared `PipelineRuntime` must not serialize parallel children on one
  coarse mutex for the clone-in/merge-out pair. Narrow the critical
  sections, shard the state, or switch to lock-free accumulation — the
  observable merge semantics (deterministic ordering of merged child state)
  must be preserved.
- A compose-cache hit for a code or file transclusion must not read the
  target from disk. All reads move inside the cached computation.

### F5 — Runtime context capture is demand-driven everywhere and captured once

- The implicit full ten-group capture is removed from the public surface:
  `ComposeOptions::new()` no longer performs `ComposeContext::capture()`.
  Callers either supply a context, get demand-driven capture at first use,
  or explicitly request a full host capture. This is a deliberate behavioral
  break for any caller that silently depended on eager whole-host capture;
  the `EffectiveStateBuilder` fallback follows the same rule.
- One process-environment snapshot is taken per compose session and shared
  by every context construction in that session (composition contexts,
  file-resolution contexts, expression evaluation), consistent with the
  fix's immutable launch capture.
- The docs group capture is spawned like the other expensive groups instead
  of running serially on the coordinating thread.
- The interpolation depth loop re-scans only regions produced by the
  previous pass, not the entire document per nesting level. Nesting
  semantics and the existing depth limit are unchanged.

### F6 — A compose session owns cross-document reuse

Introduce a request-scoped compose session in Darkmatter — the single object
Claudine's invocation owner hands its evidence to, and the home for every
invocation-scoped cache. It carries at minimum:

- the propagated `FileResolutionContext` and repository evidence (the fix's
  outputs);
- the shared environment snapshot (F5);
- the canonicalization memo (F3);
- a schema-source cache: raw bytes and parsed/converted schema per resolved
  schema path, including named-type imports, so a sequence whose documents
  share a schema parses it once;
- a trigger-discovery cache keyed by the discovered root set, so trigger
  roots are walked and trigger files read once per invocation;
- the existing remote runtime, run-local compose cache, and persistent
  store handles, which already have the right scope.

Freshness contract: session caches are invocation-scoped, never
process-global, and revalidate by cheap metadata (size + mtime) or content
hash before reuse within long-running invocations. The lifecycle rules that
force document rereads at retry/resume/JIT boundaries are unchanged —
documents are not cached by the session; their *supporting inputs* are.

`ComposeOptions` construction becomes explicit about its session: a
convenience path may create a private single-use session, but Claudine's
canonical paths create one session per invocation and thread it through
compose, inline-compose, sequence steps, system-prompt and appendix
composition, and harness materialization. Renaming or restructuring
`ComposeOptions` construction to make the session explicit is an acceptable
break.

### F7 — Claudine composes and reads each document once per decision

- The prepare flow must stop running two full compose pipelines per
  document. The shell-approval discovery pass and the canonical staged pass
  are consolidated: either one pipeline run produces both the approval
  inventory and the canonical output, or the discovery pass is reduced to
  the minimal stages required to enumerate shell directives byte-for-byte.
  The security contract is untouched: every shell command is still approved
  against its exact bytes before any execution, sequences still snapshot
  dynamic sources once during static preflight, and `--dry-run` semantics
  are unchanged.
- Source resolution and reload read the document file once, constructing
  original text and parsed Markdown from the same bytes. The
  retry/resume/JIT freshness boundaries keep their reread — but one reread,
  not two.
- The duplicate runtime-context captures in preparation collapse to one
  capture shared by shell-preflight options, the prepared context, and the
  lifecycle context, with overrides applied to the shared capture. (The
  topology cost of these captures is bounded by the fix; this feature
  removes the duplicate capture work itself.)
- Terminal/`FORCE_COLOR` detection is probed once and carried on the
  preparation context.

### F8 — The structure probe costs what structure costs

Sniff's structure tier is the probe the fix memoizes; this feature makes the
single probe cheap. All changes preserve detection results byte-for-byte
unless a tier explicitly declines the data.

- The nested-marker walk uses the same parallel walker as the shared system
  view.
- Nested candidate detectors pre-filter on raw manifest text (for example a
  `[workspace]` / `"workspaces"` substring check through the existing
  manifest store) before committing to a full TOML/JSON parse, so leaf
  manifests are rejected without structured parsing. Detectors whose
  pre-filter matches parse exactly as today.
- Per-seed package name and version resolution is gated by the request
  tier. The structure tier resolves names only if its consumers need them
  and versions only on request; Claudine's launch/topology consumers
  declare what they actually read. If nothing needs versions, the root
  manifest is not re-parsed for workspace version inheritance.
- The structure tier gets a marker-collecting shared walk: one walk
  produces both nested-marker candidates and manifest-directory evidence so
  membership-glob expansion consumes evidence instead of launching its own
  whole-tree walks.
- The PATH executable scan runs only for requests that consume executable
  provenance, not unconditionally in every tier.
- `RepoInfo` path lookups (`package_for_dir`, `area_for_dir`,
  `package_area_label_for_dir`) reuse a lazily built, memoized ownership
  index on the `RepoInfo` instead of rebuilding it — with per-package
  canonicalize syscalls — per lookup. Exposing the `_with_index` variants
  or memoizing internally are both acceptable; changing the lookup method
  signatures is an acceptable break.
- `detect_area` and any other single-answer helpers run the structure tier,
  not full detection.

### F9 — Regressions are guarded by work counts

Extend the existing seams rather than inventing new ones:

- Sniff's performance counters already count walks, manifest parses, and
  probes; tests assert the new bounds (one parallel marker walk, parse
  count proportional to workspace-relevant manifests, zero glob walks when
  evidence exists, zero PATH scans in structure tier).
- Darkmatter gains request-scoped counters (on the session or run cache,
  never process-global statics) for full-document parses, context
  captures, environment snapshots, schema file reads, trigger-root walks,
  and canonicalize calls. Deterministic tests assert bounded counts for
  representative documents: linkless, link-heavy, transclusion-heavy,
  schema-bearing, and a multi-step sequence sharing one schema.
- Claudine asserts one compose pipeline execution and one file read per
  document per decision through the same kind of scoped seam.
- Wall-clock benchmarks (the existing ignored diagnostics plus a
  transclusion-fanout case) record before/after evidence on one host but
  are not CI gates.

## Scope

### In scope

- Darkmatter compose pipeline work reduction (F1–F5)
- The Darkmatter compose session and its invocation-scoped caches (F6)
- Claudine prepare/execute consolidation to one compose, one read, one
  capture per document decision (F7)
- Sniff structure-tier cost reduction and lookup memoization (F8)
- Work-count instrumentation and diagnostic benchmarks (F9)
- Breaking API changes in all three package areas where they serve the
  above, including their downstream call-site updates within this monorepo
- Documentation and skill updates for changed contracts

### Out of scope

- Context propagation and topology memoization (owned by the
  propagated-context fix)
- Changing composition semantics: stage order, expression language,
  schema validation results, trigger matching, transclusion output,
  lifecycle behavior, or rendered errors' content
- Process-global caches keyed across invocations, and caching fully
  composed documents across invocations
- Remote-fetch policy, cache-root policy, or consent behavior
- Rewriting the reference/AST layer beyond what shared-parse reuse requires
- Sniff detectors for build systems not present in the walk today
- Async/streaming compose or replacing rayon

## Implementation constraints

- Land in independently verifiable increments; each F-section must be
  mergeable alone with its counters and tests. F6 is the ordering pivot:
  F1–F5 may land before it, but F7 consumes it.
- Session and caches are `Send`-compatible where the parallel transclusion
  phase touches them; no new global mutable state.
- Preserve typed error provenance and spans through every consolidation;
  lazy error contexts must produce diagnostics indistinguishable from
  today's.
- Byte-for-byte output equivalence for composed documents across F1–F5 and
  F8, verified against the existing corpus tests before/after.
- The fix's no-late-ambient-reads rule applies to all new code: the session
  is fed, it does not discover. Compatibility constructors that still
  discover must be clearly separated from the canonical session path.
- Keep `biscuit-file` free of Sniff dependencies; the session lives in
  Darkmatter, not `biscuit-file`.
- Any behavior-adjacent comment ("composed twice", "captures on
  construction", "rebuilds the index per lookup") is corrected in the same
  change that alters the behavior.
- Preserve macOS, Linux, and Windows behavior; canonicalization memos and
  ownership indexes must respect platform path identity rules (drive
  letters, UNC, case behavior) exactly as the underlying APIs do today.

## Test requirements

### Darkmatter — L1

- Parse-count tests: linkless, link-heavy, and HTML-reference documents
  compose with the bounded number of full parses; output equivalence against
  the current extractor results.
- No-op stage tests: a plain document's compose performs zero
  replacement/interpolation body allocations, zero error-context
  constructions, and zero shell-stage work (observed through the session
  counters).
- Lazy error-context tests: forced failures in shell, transclusion, link,
  and schema stages render blocks identical to current fixtures.
- Transclusion: cache-hit composes perform zero target-file reads; parallel
  fanout with many small children completes with merge ordering identical
  to today; per-directive hash/context work is counted once per phase.
- Session caching: a multi-document run sharing one `$schema` and one
  trigger root performs one schema read/parse and one trigger walk;
  modifying the schema file between documents (mtime/hash change) triggers
  revalidation.
- Capture: `ComposeOptions` without an explicit context performs no eager
  ten-group capture; a document referencing only `ctx.datetime` performs no
  host probes; environment is snapshotted once per session.

### Claudine — L1

- One-compose-per-decision: preparing and executing a document with shell
  directives runs one full pipeline, produces the identical approval
  inventory (byte-for-byte command text), and executes approved commands
  exactly as today, including the sequence static-preflight snapshot
  semantics and `--dry-run` behavior.
- One-read: source resolution and each JIT/retry/resume reread perform one
  file read per boundary, with content freshness behavior preserved.
- One-capture: shell-preflight options, prepared context, and lifecycle
  context observe the same captured runtime values.
- Real-CLI regression: the composition, sequence, lifecycle, and system
  prompt integration suites pass unchanged; previously slow tests do not
  regress under concurrent nextest load.

### Sniff — L1

- Counter-bound tests for the structure tier on the existing large fixture:
  one marker walk (parallel), manifest parses bounded by
  workspace-relevant manifests rather than total manifests, zero
  membership-glob walks when walk evidence exists, zero PATH scans, and
  name/version parses only when requested.
- Result equivalence: structure detection output on the fixture corpus is
  identical before and after, for every tier.
- `RepoInfo` lookup memoization: repeated `package_for_dir` calls build the
  ownership index once; canonicalize counts are flat across repeated
  lookups.
- `detect_area` produces its current answers through the structure tier.

### Cross-cutting

- Windows/macOS path semantics covered for the canonicalization memo and
  ownership index (drive/UNC fixtures where symlinks are unavailable).
- No test asserts elapsed time; ignored diagnostic benchmarks record
  cold/warm wall-clock for the root-launch, sequence, and
  transclusion-fanout scenarios using the fix's before/after methodology.

## Verification scope

Before implementation:

1. capture baseline work counts (Sniff counters plus temporary tracing) and
   wall-clock diagnostics for: root-launch compose, isolated compose, a
   representative sequence, and a transclusion-heavy document;
2. run GitNexus impact analysis on `ComposeOptions` construction,
   `run_compose_pipeline`, the reference extractors, `ManifestStore`,
   `RepoInfo` lookup methods, and every non-Claudine consumer of the
   affected public APIs (Reaper, Darkmatter CLI, DMLS, research tooling)
   before breaking them;
3. confirm the propagated-context fix's owner/evidence types, and agree the
   session handoff shape with that implementation if it is in flight.

After implementation:

1. `just test` and `just lint` in `darkmatter`, `sniff`, and `claudine`
   package areas; affected `biscuit-file` tests if its contracts moved;
2. `just _test claudine-cli --no-fail-fast` from the repository root;
3. Darkmatter corpus/output-equivalence suites and `just test-l2` only where
   terminal rendering is part of an asserted contract;
4. re-run the baseline scenarios and record before/after work counts and
   wall-clock; work counts are the regression gate, wall-clock is evidence.

## Documentation maintenance

- Darkmatter skill and `compose.md`: the session as the request authority,
  demand-driven-by-default capture, and the cache/freshness contract.
- Sniff skill: tier semantics (what structure does and does not resolve),
  the marker-collecting walk, and lookup memoization.
- Claudine skill/architecture and composition docs: one-compose-per-decision
  preparation and the session handoff from the invocation owner.
- Per-area `docs/dependencies.md` and READMEs if crate boundaries or public
  surfaces change.
- Delete or correct every comment describing the removed double passes,
  eager captures, and per-lookup index rebuilds.

## Acceptance criteria

- [ ] Reference extraction performs a bounded, extractor-count-independent
      number of full-document parses per compose, with byte-identical
      reference output.
- [ ] No-op stages perform no body allocations or error-context
      construction; forced errors render identically to current fixtures.
- [ ] Per-directive/per-record loop-invariant work (expression contexts,
      options hashing, env whitelists, canonicalization) is computed once
      per phase, with a request-scoped canonicalization memo in place.
- [ ] Transclusion cache hits read no target files; parallel children are
      not serialized on a coarse runtime lock; merge semantics unchanged.
- [ ] No public Darkmatter entry point performs an eager ten-group host
      capture; capture is demand-driven with one environment snapshot per
      session; docs capture is spawned with its peers.
- [ ] A Darkmatter compose session carries propagated evidence and
      invocation-scoped schema, trigger, and canonicalization caches; a
      sequence sharing one schema parses it once with metadata
      revalidation.
- [ ] Claudine prepares each document with one compose pipeline run, one
      file read per freshness boundary, and one runtime capture, while
      preserving byte-for-byte shell approval, static-preflight, dry-run,
      and lifecycle semantics.
- [ ] The Sniff structure tier uses a parallel marker walk, pre-filtered
      manifest parsing, tier-gated name/version resolution, evidence-fed
      glob expansion, and no unconditional PATH scan, with detection output
      unchanged; `RepoInfo` lookups memoize their ownership index.
- [ ] Work-count regression tests cover Darkmatter, Claudine, and Sniff
      bounds without process-global counters or elapsed-time assertions.
- [ ] Downstream consumers of every broken API are migrated in the same
      change set; `just test`/`just lint` pass in all touched package
      areas and the no-fail-fast `claudine-cli` gate passes.
- [ ] Before/after work counts and diagnostic wall-clock results are
      recorded, demonstrating material reduction on the baseline scenarios.
