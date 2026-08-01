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
review_iterations: 1
---

# Faster compose: reduce the work one composition performs

## Outcome

Composing a Markdown document performs work proportional to what the document
actually contains. A document with no links pays a bounded set of shared
reference-parser passes rather than one parse per extractor; a document with
no shell directives pays no shell error-context construction; a stage with
nothing to do avoids body-sized copies and stage-specific setup.
Repeated stable inputs — the same schema file, trigger discovery key,
normalized path request, and host evidence — are read or computed once per
invocation and reused. Long-running invocations revalidate mutable supporting
inputs at the freshness boundaries defined below.

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
  base (which re-canonicalizes external state and re-serializes the entire
  context value map and environment per directive), a `canonicalize` of the
  same child path up to three times, and a double pass over the directive
  slice for file versus code kinds. Parallel children do not compose while
  holding the shared `PipelineRuntime` mutex, but every child serializes its
  runtime snapshot and merge through that one lock.
- Code transclusion reads and hashes the target before the run-local cache
  lookup. Strict persistent-cache validation also rereads operation sources,
  so neither kind of validated hit is content-read-free today.
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
  A 500-step sequence pointing at one unchanged schema file can therefore
  re-read and re-parse it 500 times. Named-type imports are deduplicated only
  within one schema-resolution call and have the same cross-document shape.
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
  arguments during preparation; the broader lifecycle path can perform a
  third near-identical capture for lifecycle context.
- `FORCE_COLOR`/terminal detection is probed four separate times in one
  command path, even where the same output sink and policy are being queried.

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

## Cost and freshness terminology

- A **compose session** is the invocation-scoped Darkmatter authority defined
  in F6. Claudine creates one for a CLI invocation; a standalone Darkmatter
  convenience call may create a private single-use session.
- A **document decision** is one preparation attempt over one immutable set of
  source bytes. A lifecycle-mandated reread — post-`initialize`
  stabilization, JIT step entry, retry, or resume — starts a new decision.
  Those rereads remain observable behavior and are not redundant work.
- A **content read** reads file bytes. A metadata probe (`metadata`, size,
  modification time, or equivalent platform evidence) is counted separately.
  "Zero reads on a cache hit" below means zero target-content reads; strict
  validation may still perform bounded metadata probes. A cold strict lookup
  that must read content to establish identity is validation work, not a
  validated hit; later reuse of that established identity is the hit governed
  by the zero-content-read bound.
- A **stable-input count** applies while the observed metadata and discovery
  roots do not change. An invalidated entry may be reread or rewalked and is
  counted as required work, not a regression.

## Required design

### F1 — One parse serves all reference extraction

Reference extraction must stop re-parsing the document per extractor. Link
resolution and link normalization each build at most one MDAST product and one
pulldown event stream, plus one shared `LineIndex`, and run every classifier
against those products. A future implementation may collapse to one parser
family only if it preserves the current syntax and span behavior. If the body
is byte-identical between the link-resolve and link-normalization stages, the
parse products may be reused across them; reuse is keyed by a content identity
computed once for that body version, not by assuming an intervening stage did
not mutate it.

Bound: composing a document performs a number of full-document parses that is
independent of the number of extractor kinds: at most one parse per required
parser family for a body version and stage, with cross-stage reuse when the
body identity matches. A linkless document therefore performs at most the two
shared parses that prove it has no references, rather than the current ten per
stage.

Changing the signature or ownership model of the internal reference-extractor
APIs is acceptable. The extracted reference records, their spans, and their
provenance must remain identical to current output. Record ordering is also
part of the contract wherever a public reference API exposes it; a shared AST
walk must not silently replace the current extractor-group order with document
order.

### F2 — No-op stages are near-free

- A stage that changes nothing must not allocate a body-sized copy. The
  replacement and interpolation fast paths return a borrowed/unchanged
  signal (for example `Cow` or an explicit no-change variant) instead of
  `to_string()` copies the caller discards. Small rule, warning, and report
  allocations are not prohibited by this bound and are counted separately
  only when evidence shows they matter.
- Inline cleanup must not hash the body twice. It may skip reconstruction only
  when a cheap predicate or existing parse product proves that cleanup and
  optional fixed-width reflow are byte no-ops; otherwise it follows the
  existing cleanup path. The optimization must not guess from a substring and
  bypass normalization that would have changed the document.
- `source_context_for_errors` and `full_source_context_for_errors` become
  lazy: the body `Arc`, the frontmatter reconstruction, and the path
  `canonicalize` are deferred until an error actually needs them, then
  memoized by source/body identity. The shell-expansion and shell-block stages
  must hit their empty-directive early-outs before any error-context
  construction.
- Error rendering quality must not regress: when an error does occur, the
  rendered block carries the same source excerpt, path, and span data it
  carries today. Lazy construction is an internal ownership change, not a
  lossy error representation; repeated errors for the same source/body version
  reuse the same constructed source context.

### F3 — Loop-invariant work is hoisted; canonicalization is deduplicated

- The per-directive `when:` expression resolution context, the base options
  hash (including `classify_options` and the context/environment fingerprint),
  and the transclusion options clone are computed once per transclusion phase,
  not once per directive. Directive-specific overlays still receive their own
  overlay hash and are combined with the phase-wide base; the complete cache
  identity must not collapse two distinct `set=` or replacement options.
- The link-normalization env-path whitelist (env-var read plus
  `canonicalize`) is resolved once per stage, not once per record.
- One session-owned canonicalization memo serves the whole pipeline. Within a
  document-decision epoch, each distinct canonicalization input used by the
  resolver, cache keying, error contexts, or normalization is canonicalized
  once. A relative input is keyed with its resolution-base identity (or is
  made absolute first), so identical spelling in two directories cannot
  alias. The memo does not case-fold, stringify, or merge symlink aliases that
  the platform APIs distinguish. Failed canonicalizations are not retained
  across a stage that may create the target, and JIT/retry/resume freshness
  boundaries advance the epoch or revalidate the entry. The memo lives on the
  session/run cache, never in a process-global.
- The transclusion prepare step walks the directive slice once, partitioning
  by kind, instead of filtering the full slice per kind.

### F4 — Transclusion runtime contention and cache-hit purity

- The shared `PipelineRuntime` must not funnel every parallel child through a
  whole-runtime clone/merge mutex. Prefer immutable shared handles plus a
  child-local delta returned with each result and folded in prepared-item
  order; narrower locks are acceptable where a genuinely shared facility
  (single-flight cache, shell allow-once state, or remote runtime) requires
  them. Lock-free accumulation is not a requirement. Existing cycle,
  dependency, cache-stat, and shell semantics must be preserved.
- A validated run-local or persistent cache hit for a code or file
  transclusion performs zero target-content reads. Cache lookup and cheap
  freshness metadata checks happen before reading bytes; a miss or invalidated
  entry reads and hashes content inside the cached computation/revalidation
  path. `Strict` must not silently become `Optimistic` to satisfy the counter.

### F5 — Runtime context capture is demand-driven everywhere and captured once

- The implicit full ten-group capture is removed from the public surface:
  `ComposeOptions::new()` no longer performs `ComposeContext::capture()`.
  Callers either supply a context, get demand-driven capture at first use,
  or explicitly request a full host capture. This is a deliberate behavioral
  break for any caller that silently depended on eager whole-host capture;
  the `EffectiveStateBuilder` fallback follows the same rule. Explicit APIs
  whose name requests a full capture remain valid; the prohibition is on
  default and convenience construction doing it implicitly.
- One process-environment snapshot is taken per compose session and shared
  by every context construction in that session (composition contexts,
  file-resolution contexts, expression evaluation), consistent with the
  fix's immutable launch capture. Provider/model and document-specific values
  are explicit immutable overlays on that base snapshot; callers do not mutate
  a shared environment map in place.
- The docs group capture is spawned like the other expensive groups instead
  of running serially on the coordinating thread.
- The interpolation depth loop tracks dirty regions produced by the previous
  pass and re-scans only regions whose scanner context is locally stable.
  Dirty windows include enough boundary context to detect delimiters formed
  across a replacement edge. A replacement that can change Markdown block/code
  classification, or any case the regional scanner cannot prove local, falls
  back to a full-document scan. Nesting semantics, warnings, code-block
  exclusion, literal conversion, and the existing depth limit are unchanged.

### F6 — A compose session owns cross-document reuse

Introduce a request-scoped compose session in Darkmatter — the single object
Claudine's invocation owner hands its evidence to, and the home for every
invocation-scoped cache. It carries at minimum:

- the propagated `FileResolutionContext` and repository evidence (the fix's
  outputs);
- the shared environment snapshot (F5);
- the canonicalization memo (F3);
- a layered schema-source cache: raw bytes and passive syntax trees keyed by
  canonical source identity plus freshness evidence; resolved/converted
  schemas additionally key every semantic resolution input (ordered schema
  roots, file-resolution context/fallback identity, meta-schema controls, and
  imported-file content identities). Named-type imports use the same source
  cache, so a sequence whose documents share an unchanged schema parses it
  once without reusing a conversion under the wrong resolution context;
- a trigger-discovery cache keyed by the boundary, ordered nearest-first root
  vector, file-resolution context identity, and freshness evidence. Root order
  is semantic because it controls shadowing. Stable trigger roots are walked
  and stable trigger files read once per invocation; changed directory/file
  metadata invalidates only the affected registry entry;
- the existing remote runtime, run-local compose cache, and persistent
  store handles, which already have the right scope.

Freshness contract: session caches are invocation-scoped, never
process-global. Stable file entries are revalidated with the strongest cheap
metadata token already accepted by the corresponding cache contract; changed
or inconclusive metadata falls back to a content read/hash. Trigger discovery
revalidates the membership evidence for every visited directory, not only the
root, so nested additions, removals, and shadowing changes cause a rescan. If
the platform cannot establish that evidence cheaply, the decision boundary
falls back to a walk. The lifecycle rules that force document rereads at
post-`initialize`, retry, resume, and JIT boundaries are unchanged: documents
are not cached by the session, and supporting-input reuse never allows a
stale schema or trigger registry to cross a required freshness boundary.

`ComposeOptions` construction becomes explicit about its session: a
convenience path may create a private single-use session, but Claudine's
canonical paths create one session per invocation and thread it through
compose, inline-compose, sequence steps, system-prompt and appendix
composition, and harness materialization. Renaming or restructuring
`ComposeOptions` construction to make the session explicit is an acceptable
break.

The session is a cheap cloneable handle (typically `Arc`-backed), is `Send +
Sync`, and provides single-flight population for caches reachable from parallel
transclusion or sequence work. Concurrent requests for the same cold entry
perform the underlying read/parse once; counters are shared by the session but
never by unrelated tests or invocations.

### F7 — Claudine composes and reads each document once per decision

- The prepare flow must stop running two full compose pipelines per
  document decision. Darkmatter exposes a reusable prepared plan (or an
  equivalent staged artifact) that contains the exact shell-approval
  inventory and all work that can safely precede approval. Canonical
  execution consumes that same plan after approval instead of recomposing
  the source. A smaller discovery scanner is acceptable only if it neither
  executes shell nor causes the canonical path to repeat the full pipeline.
  The security contract is untouched: every shell command is still approved
  against its exact bytes before any execution, sequences still snapshot
  dynamic sources once during static preflight, and `--dry-run` semantics are
  unchanged.
- Source resolution and reload each read the document file once, constructing
  original text and parsed Markdown (including YAML-origin documents) from
  the same byte buffer. Post-`initialize`, retry, resume, and JIT boundaries
  begin new document decisions and keep their required reread — but one
  content read within each decision, not two.
- The duplicate runtime-context captures in one decision collapse to one
  captured base shared by shell-preflight options, the prepared context, and
  lifecycle evaluation. Provider/model, step, and lifecycle values are
  immutable overlays; they are not reasons to repeat host discovery or the
  environment snapshot. A new decision may intentionally refresh the
  document-derived overlay without recapturing invocation-stable evidence.
- Terminal/`FORCE_COLOR` capability is probed once per output sink and policy
  and carried on the preparation context. Stdout and stderr remain distinct
  when their terminal capabilities differ.

### F8 — The structure probe costs what structure costs

Sniff's structure tier is the probe the fix memoizes; this feature makes the
single probe cheap. All changes preserve detection results byte-for-byte
for fields requested by the caller. A tier may explicitly decline a field,
but absence must be represented in the API rather than fabricated as an empty
string or directory-derived value.

- One marker-only parallel observation, built on the same walker machinery as
  the shared system view, collects nested-marker candidates and manifest
  directories. Membership-glob expansion consumes that evidence instead of
  launching its own whole-tree walks. This observation does not pay for
  repository inventory, file classification, dependency detection, or typed
  manifest parsing.
- Nested candidate detectors pre-filter on raw manifest text (for example a
  `[workspace]` / `"workspaces"` substring check through the existing
  manifest store) before committing to typed TOML/JSON deserialization. The
  pre-filter is conservative: it has no false negatives for syntax accepted
  today. A negative candidate still receives whatever syntax validation is
  necessary to preserve current malformed-manifest errors, but does not build
  the typed value used by workspace detection. An inconclusive pre-filter
  falls back to the current parser.
- Per-seed package name and version resolution is gated by the request
  tier. The structure tier resolves names only if its consumers need them
  and versions only on request; Claudine's launch/topology consumers
  declare what they actually read. If nothing needs versions, the root
  manifest is not re-parsed for workspace version inheritance. Because
  `Package.name` is currently required, this change must either introduce a
  lightweight topology result or make declined metadata explicitly optional;
  it must not populate a required field with a placeholder. Claudine requests
  package names where its context contract exposes them, but does not request
  versions or executable provenance unless a consumer reads them.
- The PATH executable scan runs only for requests that consume executable
  provenance, not unconditionally in every tier.
- `RepoInfo` path lookups (`package_for_dir`, `area_for_dir`,
  `package_area_label_for_dir`) reuse a lazily built, memoized ownership
  index on the `RepoInfo` instead of rebuilding it — with per-package
  canonicalize syscalls — per lookup. Exposing the `_with_index` variants
  or memoizing internally are both acceptable; changing the lookup method
  signatures is an acceptable break. The lookup path itself may still need
  one platform-aware canonicalization per call; eliminating the repeated
  per-package index construction is the required bound.
- `detect_area` and any other single-answer helpers run the structure tier,
  not full detection.

### F9 — Regressions are guarded by work counts

Extend the existing seams rather than inventing new ones:

- Sniff's performance counters already count walks, manifest parses, and
  probes, but the current manifest counter also covers raw-text acquisition.
  Split or extend it so raw content reads, syntax validations, and typed
  manifest parses are independently observable. Tests assert the new bounds
  (one parallel marker walk, typed parse count proportional to
  workspace-relevant manifests, zero glob walks when evidence exists, zero
  PATH scans when executable provenance was not requested). Parallel workers
  report into a request-scoped collector rather than global mutable counters.
- Darkmatter gains request-scoped counters (on the session or run cache,
  never process-global statics) for full-document parses, context
  captures, environment snapshots, schema content reads/parses,
  trigger-root walks/file reads, canonicalization cache hits/misses,
  target-content reads versus metadata probes, parse-product builds, and
  body-sized copies. Deterministic tests assert bounded counts for
  representative documents: linkless, link-heavy, transclusion-heavy,
  schema-bearing, and a multi-step sequence sharing one schema. Stable-input
  counts and invalidation work are reported separately.
- Claudine asserts one compose pipeline execution and at most one source-byte
  read per file-backed document decision through the same kind of scoped seam.
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

- Land in independently verifiable increments with the relevant counters and
  tests. The numbered sections are requirements, not necessarily independent
  commits: F6 is the ordering pivot, F1–F5 may land before it, and F7 consumes
  its session and prepared-plan seams.
- Session and caches are `Send + Sync` wherever parallel transclusion or
  sequence work can share them; no new global mutable state.
- Preserve typed error provenance and spans through every consolidation;
  lazy error contexts must produce diagnostics indistinguishable from
  today's.
- Byte-for-byte output equivalence for composed documents across F1–F5.
  Sniff preserves every requested field and detection decision across F8;
  fields a request explicitly declines are represented as absent.
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
- Every cache key includes all inputs that affect the cached result. An
  optimization may broaden sharing only after demonstrating that omitted
  inputs are semantically irrelevant.

## Test requirements

### Darkmatter — L1

- Parse-count tests: linkless, link-heavy, and HTML-reference documents
  compose with at most one MDAST parse and one pulldown parse per body
  version/stage; extractor count does not affect the bound, and output plus
  record ordering match the current extractor results.
- No-op stage tests: a plain document's compose performs zero
  replacement/interpolation body-sized copies, zero error-context
  constructions, and zero shell-stage work (observed through the session
  counters). Cleanup takes its no-op path only for fixtures whose current
  output is byte-identical.
- Lazy error-context tests: forced failures in shell, transclusion, link,
  and schema stages render blocks identical to current fixtures.
- Transclusion: validated cache-hit composes perform zero target-content
  reads while permitted metadata probes remain visible; strict validation is
  unchanged. Parallel fanout with many small children completes with merge
  ordering identical to today; phase-wide hash/context work is counted once,
  while directive-specific option overlays retain distinct identities.
- Session caching: a multi-document run sharing one `$schema` and one
  trigger root performs one schema content read/parse and one trigger walk
  while inputs stay stable. Modifying a schema, adding/removing a trigger, or
  changing root order between documents invalidates the affected entry and
  produces current output.
- Capture: `ComposeOptions` without an explicit context performs no eager
  ten-group capture; a document referencing only `ctx.datetime` performs no
  host probes; environment is snapshotted once per session. An explicitly
  named full-capture API still captures every group.
- Regional interpolation: replacements that create delimiters across a dirty
  window boundary or alter Markdown fence/code classification match the
  current full-scan output and exercise the conservative fallback.
- Canonicalization: repeated requests in one decision hit the memo, while a
  failed lookup followed by a mutation-capable stage and JIT/retry/resume
  boundaries revalidate rather than retaining a stale result.

### Claudine — L1

- One-compose-per-decision: preparing and executing a document with shell
  directives runs one full pipeline, produces the identical approval
  inventory (byte-for-byte command text), and executes approved commands
  exactly as today, including the sequence static-preflight snapshot
  semantics and `--dry-run` behavior.
- One-read: source resolution and each JIT/retry/resume reread perform one
  file read per document decision, including Markdown and YAML sources, with
  post-`initialize` and all other freshness behavior preserved.
- One-capture: shell-preflight options, prepared context, and lifecycle
  context observe the same captured runtime values.
- Terminal capability: repeated queries for one sink/policy reuse its
  snapshot, while redirected stdout and terminal stderr can retain different
  answers.
- Real-CLI regression: the composition, sequence, lifecycle, and system
  prompt integration suites pass unchanged; previously slow tests do not
  regress under concurrent nextest load.

### Sniff — L1

- Counter-bound tests for the structure tier on the existing large fixture:
  one marker-only walk (parallel); raw reads, syntax validations, and typed
  parses counted separately; typed parses bounded by workspace-relevant
  manifests rather than total manifests; zero membership-glob walks when
  walk evidence exists; zero PATH scans when executable provenance is not
  requested; and name/version parses only when requested.
- Pre-filter correctness: valid workspace descriptors cannot be rejected by
  the raw filter, inconclusive text falls back to typed parsing, and malformed
  leaf/workspace manifests produce the same errors as today.
- Result equivalence: detection decisions and all requested fields on the
  fixture corpus are identical before and after for every tier; declined
  name/version/provenance fields are explicitly absent rather than fabricated.
- `RepoInfo` lookup memoization: repeated `package_for_dir` calls build the
  ownership index and canonicalize its packages once. Each later query incurs
  at most one query-path canonicalization unless that path is separately
  memoized; it never repeats per-package canonicalization.
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
      number of full-document parses — at most one per required parser family
      for a body version/stage — with byte-identical, identically ordered
      reference output.
- [ ] No-op stages perform no body-sized copies or error-context
      construction; forced errors render identically to current fixtures.
- [ ] Per-directive/per-record loop-invariant work (expression contexts, base
      options hashes, env whitelists, canonicalization) is computed once per
      phase; directive overlays remain distinct, and the session-scoped
      canonicalization memo respects mutation and decision epochs.
- [ ] Validated transclusion cache hits perform no target-content reads;
      metadata probes and strict validation remain observable, parallel
      children are not serialized on a coarse runtime lock, and merge
      semantics are unchanged.
- [ ] No default or convenience Darkmatter entry point performs an eager
      ten-group host capture; explicitly named full capture remains available,
      demand-driven capture uses one environment snapshot per session, and
      docs capture is spawned with its peers.
- [ ] A Darkmatter compose session carries propagated evidence and
      invocation-scoped schema, trigger, and canonicalization caches; a
      sequence sharing stable schema and trigger inputs parses/walks each once,
      while metadata, directory membership, resolution-context, and root-order
      changes invalidate the affected entries.
- [ ] Claudine prepares each file-backed document decision from at most one
      source-byte read and one reusable compose plan/pipeline execution, using
      one captured runtime base and per-sink terminal-capability snapshots,
      while preserving byte-for-byte shell approval, static-preflight,
      dry-run, and lifecycle semantics.
- [ ] The Sniff structure tier uses a parallel marker walk, pre-filtered
      manifest parsing without suppressing malformed-input errors, tier-gated
      name/version resolution with explicit absence, evidence-fed glob
      expansion, and no unconditional PATH scan. Requested detection output is
      unchanged, and `RepoInfo` lookups memoize their ownership index without
      repeating per-package canonicalization.
- [ ] Work-count regression tests cover Darkmatter, Claudine, and Sniff
      bounds — including raw versus typed manifest work and content reads
      versus metadata probes — without process-global counters or elapsed-time
      assertions.
- [ ] Downstream consumers of every broken API are migrated in the same
      change set; `just test`/`just lint` pass in all touched package
      areas and the no-fail-fast `claudine-cli` gate passes.
- [ ] Before/after work counts and diagnostic wall-clock results are
      recorded, demonstrating material reduction on the baseline scenarios.
