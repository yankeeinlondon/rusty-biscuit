# Faster-compose implementation plan

Reference: [`spec.md`](spec.md)

## Review conclusion — 2026-09-08

The feature remains worthwhile. The current source review corrects the August
baseline: `ComposeOptions::new()` is already minimal, Claudine already shares
`DocumentEpoch` context, and `compose_preflight()` is a dedicated collector,
not a second full compose. Its existing graph is the integration seam to reuse.
The bare `EffectiveStateBuilder` still captures fully; duplicate Markdown
reads, repeated reference parsing, and the documented Sniff costs remain.

Strict persistent validation reads current content. Zero-read assertions apply
to reuse of validated immutable inputs inside a valid epoch, never to arbitrary
persistent hits. Required validation is included in lookup counters. Session
sharing must preserve launch/source separation, live `current.*`, policy,
mutation boundaries, and bounded memory.

This review changes documentation only. No fresh timing measurements or Rust
test results are claimed. D0 establishes the current implementation baseline;
do not claim already-shipped improvements as gains from this feature.

## Existing dependencies

The [propagated-context fix](../../fixes/_completed/2026-08-01-propagated-context/spec.md)
and [launch-anchor fix](../../fixes/_completed/2026-08-12-ctx-launch-anchor/spec.md)
have landed. Use their `InvocationContext`, `SourceContext`, `DocumentEpoch`,
and scoped work counters. Preserve the current
[finalized-reference contract](../2026-08-26-finalized-references/spec.md),
including caller-file provenance and repository scope catalogs.

D1–D3 and S1 can proceed independently after D0. D4 extends existing demand
capture, D5 consumes the retained invocation evidence, and C1 integrates the
session with the existing collector graph. No temporary ambient-discovery
substitute or competing context owner is needed.

## Completion contract

Implementation is complete only when all of the following are true:

- reference extraction performs at most one MDAST parse and one pulldown
  parse per body version and stage, with cross-stage reuse on identical body
  identity, and reference records, spans, provenance, and ordering are
  unchanged;
- no-op stages perform no body-sized copies; error source contexts are lazy
  and memoized; forced errors render identically to current fixtures;
- per-directive loop-invariant work (expression contexts, base options hash,
  env whitelists, transclusion options) is computed once per phase while
  directive-specific overlays keep distinct cache identities;
- same-epoch validated-input reuse performs zero duplicate target-content reads;
  parallel children fold child-local deltas in prepared order instead of
  serializing whole-runtime clone/merge on one mutex;
- no default or convenience Darkmatter entry point performs the eager
  ten-group host capture; one environment snapshot per session is shared with
  immutable overlays; docs capture is spawned with its peers; regional
  interpolation is implemented only if D0 justifies it, otherwise its measured deferral is recorded;
- a compose session (cheap `Arc`-backed handle, `Send + Sync`, single-flight
  population) carries propagated evidence, the canonicalization memo, the
  layered schema-source cache, and the trigger-discovery cache, with the
  spec's freshness contract enforced at every lifecycle boundary;
- Claudine prepares each file-backed document decision from at most one
  source-byte read, one canonical full compose with compatible collector
  artifact reuse, and one captured runtime base, preserving normalized-command
  approval identity, source provenance, static preflight,
  dry-run, and lifecycle semantics;
- the Sniff structure tier reuses compatible marker evidence or adds one parallel observation,
  syntax-aware pre-filtering where beneficial, tier-gated metadata with explicit
  absence, evidence-fed glob expansion, no unconditional PATH scan, and
  memoized `RepoInfo` ownership lookups, with requested detection output
  unchanged;
- work-count regression tests assert every bound through request/session
  scoped counters (raw versus typed manifest work, content reads versus
  metadata probes) with no process-global counters and no elapsed-time
  assertions; and
- all monorepo consumers of every broken API are migrated, package gates
  pass, and before/after work counts plus diagnostic wall-clock evidence are
  recorded.

## Reviewed source surface

Current source anchors (reviewed 2026-09-08; run impact analysis before
editing symbols during implementation):

Darkmatter (`darkmatter/lib/src/markdown/`):

- `compose/link_resolve.rs` and `compose/link_normalization.rs` — ten
  independent extractor parses each via `reference/html.rs`
  (`collect_from_html_nodes` → `output::parse_mdast`, eight extractors) and
  `reference/local.rs` (`extract_inline_refs`, links + images);
- `mod.rs` — `source_context_for_errors` (body `Arc` +
  `canonicalize` per call) and `full_source_context_for_errors`
  (reconstruct + `Arc` + `canonicalize`), called at pipeline boundaries and
  inside per-directive loops, including before the shell stages'
  empty-directive early-outs;
- `compose/inline/replacement.rs`, `compose/interpolation/rewrite.rs`,
  `compose/pipeline/phases.rs` (cleanup), `compose/inline/normalize.rs` —
  no-op body copies and double hashing;
- `compose/transclusion/engine.rs` — per-directive `options_hash` /
  `classify_options` / `graph_context_fingerprint`, per-directive
  `expression_resolution_context`, double directive-slice pass, pre-cache
  `std::fs::read`, and the `PipelineRuntime` mutex funnel;
- `compose/context/options.rs` (`ComposeOptions::new` →
  `ComposeContext::capture_minimal`, already implemented), `compose/context/effective_state.rs`
  (fallback capture), `compose/context/runtime.rs` (ambient capture may resnapshot
  environment; invocation-backed construction reuses evidence), `compose/context/capture/snapshot.rs` (docs group captured
  serially);
- `schemas/resolve.rs` (`load_schema_from_path_in_context`,
  `load_named_types` — per-document reads/parses) and
  `schemas/triggers/discovery.rs` (per-document root walks and file reads).

Claudine (`claudine/cli/src/commands/`):

- `compose/prep.rs` — shared epoch context already exists; repeated
  terminal/`FORCE_COLOR` queries remain;
- `claudine/lib/src/composition/preflight.rs` — extracts approval candidates
  but discards `ComposePreflightReport::preflight_graph`;
- `claudine/lib/src/composition/resolve.rs` — Markdown resolution/reload
  still loads both original text and the path-backed Markdown separately;
- `wrap/composition/pipeline.rs` — lifecycle reuses and extends the prepared
  snapshot; preserve this and the separate live `current.*` capture.

Sniff (`sniff/lib/src/filesystem/repo/`):

- `nested.rs` (`walk_for_nested_markers` — serial `WalkBuilder`),
  `glob.rs` (`walk_manifest_dirs` — per-expansion fallback walks),
  `cargo.rs`/`npm.rs` (typed parse before workspace-table check),
  `detection.rs` (`create_package_with_request` unconditional
  name/version resolution; `ExecutableIndex::build_path_only` in all
  tiers), `ownership.rs` + `types.rs` (`package_for_dir`/`area_for_dir`
  per-lookup index rebuild with per-package `canonicalize`),
  `area.rs` (`detect_area` → full `detect_repo`).

Downstream consumers to migrate when APIs break: Darkmatter CLI (`md`),
DMLS, Claudine (lib, cli, contract), Reaper, and research tooling — confirm
the concrete list with GitNexus before each breaking phase.

## Locked design decisions

### 1. The compose session is the Darkmatter request authority

One session type in Darkmatter owns retained evidence and the pure
invocation-scoped caches (canonicalization memo, schema-source cache,
trigger-discovery cache, environment snapshot). It is a cheap cloneable
`Arc`-backed handle, `Send + Sync`, with single-flight population.
`ComposeOptions` construction becomes session-explicit; convenience entry
points create a private single-use session. Claudine creates exactly one per
CLI invocation, fed by its existing invocation owner. Per-source views use
Darkmatter/biscuit-file value types, never a dependency back to Claudine.
Decision/mutation epochs, policy partitioning, memory budgets, and publication
validation follow F6; pure caches do not own shared mutable execution state.

### 2. Parse products are keyed by body-version identity

Reference extraction builds at most one MDAST product and one pulldown event
stream plus one shared `LineIndex` per body version and stage, and runs
classifiers over those products. Cross-stage reuse requires a content
identity computed for that body version — never an assumption that an
intervening stage did not mutate. Extractor-group output ordering is part of
the preserved contract.

### 3. Error source contexts are lazy and memoized

`source_context_for_errors`/`full_source_context_for_errors` become deferred
constructions memoized by source/body identity. Construction happens at
first error use; repeated errors for the same identity share one context.
Shell stages early-out on empty directives before any context work.

### 4. Cache identity is base-plus-overlay and complete

The phase-wide base hash (context, environment, external state) is computed
once per transclusion phase; each directive contributes only its overlay
hash (`set=`, replacements, kind-specific options). Every cache key includes
all inputs that affect the cached result; sharing may broaden only with
demonstrated semantic irrelevance of omitted inputs.

### 5. Transclusion children return deltas

Children receive immutable shared handles and return a child-local delta
folded in prepared-item order. Narrow locks remain only where a genuinely
shared facility (single-flight cache, shell allow-once state, remote
runtime) requires them. Lock-free structures are not required. Source buffers/hashes are reused within valid epochs. Strict persistent
validation keeps its content reads and dependency/TTL checks; counters cover
lookup through result delivery, including validation and artifact reads.

### 6. Capture is demand-driven by default, explicit when full

Preserve the minimal constructor and document-driven upgrade. Require an
explicit context or requirements for the bare `EffectiveStateBuilder` before
it materializes a fixed `ctx` map; never silently omit requested groups.
Share the invocation environment, retain per-epoch date/time and live
`current.*`, and spawn docs capture with the expensive peers. Dirty-region
interpolation is conditional on D0 evidence, with conservative full-scan
fallback whenever scanner context cannot be proven local.

### 7. Claudine reuses its existing preflight graph

Retain `ComposePreflightReport` through authorization and thread compatible
`PreflightGraphNode` evidence into canonical composition. Preserve
condition-blind collection, normalized command identity, original provenance,
sequence static snapshots, and the pre-execution validation gate. Reuse
resolution/source products only while source, context, overlays, and policy
remain compatible; body mutation requires current replacement spans. One
content read constructs original text and fallibly parsed Markdown/YAML.
Keep the existing shared epoch context and per-sink terminal policy.

### 8. Sniff structure work is tier-shaped with explicit absence

The structure tier runs one marker-only parallel observation (same walker
machinery as the shared system view) only when compatible evidence is absent,
feeding both nested-marker candidates and manifest-directory evidence.
Syntax-aware pre-filters are used only when beneficial and conservative (no
false negatives for accepted syntax; inconclusive falls back to the current
parser; malformed-manifest errors preserved). Declined name/version/
executable metadata is represented as absent via a lightweight topology
result or optional fields — never fabricated. `RepoInfo` lookups reuse an
index tied to immutable topology or
explicitly invalidated mutation; public mutable `root`/`packages` prohibit
a naive permanent `OnceLock`. Define clone/deserialization behavior.
`detect_area` runs the structure tier.

### 9. Counters are scoped, split, and shared correctly

Darkmatter counters live on the session/run cache; Sniff's counters gain the
raw-read / syntax-validation / typed-parse split and report through a
request-scoped collector from parallel workers. No process-global mutable
counters. Stable-input counts and invalidation work are reported separately.

## Implementation order

```text
baseline and counter seams (D0)
    -> Darkmatter no-op stages and hoisting (D1)
    -> shared reference parse (D2)
    -> transclusion runtime and cache purity (D3)
    -> demand-driven capture defaults (D4, preserve landed capture seams)
    -> compose session and cross-document caches (D5, use landed invocation owner)
    -> Claudine graph reuse / one-read / shared epoch (C1, after D5)
    -> documentation and final gates (D6)

Sniff structure tier (S1) proceeds in parallel from D0.
```

D1–D3 are internal to Darkmatter and independently mergeable. Public signature changes in any phase require a caller sweep and migration;
internal-only intent is not proof that a phase is nonbreaking.

## Phase D0 — Baselines and counter seams

### Production changes

- [ ] Add the request/session-scoped Darkmatter work counters (full-document
  parses per parser family, context captures, environment snapshots, schema
  content reads/parses, trigger-root walks/file reads, canonicalization
  hits/misses, target-content reads versus metadata probes, body-sized
  copies, preflight walks, artifact reads, validation reads, and eviction
  misses) on the run cache so later phases can assert against them. Reuse
  existing Claudine epoch counters; do not add competing capture counters.
- [ ] Split/extend Sniff's manifest counter into raw content reads, syntax
  validations, and typed parses; route parallel-worker counts into a
  request-scoped collector.
- [ ] No behavior changes in this phase beyond counter plumbing.

### Baseline capture

- [ ] Record current counter values for the representative corpus: linkless,
  link-heavy, HTML-reference, transclusion-heavy, schema-bearing documents,
  a multi-step sequence sharing one schema, and the structure-tier large
  fixture.
- [ ] Run the ignored wall-clock diagnostics (root launch, isolated launch,
  sequence, transclusion fanout) and retain results as supporting evidence.
  Use fake providers and fixture-owned HOME/CWD/config with rendezvous disabled;
  intentional root-launch diagnostics must be labeled separately from hermetic
  L1. Record commit, request shape, platform, cache state, and corpus size.
- [ ] Measure interpolation scanner work after simpler optimizations; gate
  regional scanning on material remaining cost and record deferral if absent.
- [ ] Refresh GitNexus impact analysis for `ComposeOptions` construction,
  `run_compose_pipeline`, the reference extractors, `ManifestStore`,
  `RepoInfo` lookups, and record the concrete downstream consumer list for
  each planned break.

### Checkpoint D0

Counters observable in tests; all baselines recorded; zero user-visible
change (`just test`/`just lint` in `darkmatter` and `sniff`).

## Phase D1 — No-op stages and loop-invariant hoisting (F2, F3)

### Production changes

- [ ] Replacement and interpolation fast paths return borrowed/no-change
  signals; callers skip assignment on no-change.
- [ ] Inline cleanup: single hash; skip reconstruction only under a
  byte-no-op-proving predicate; otherwise current path.
- [ ] Lazy, identity-memoized `source_context_for_errors` /
  `full_source_context_for_errors`; shell stages early-out before context
  work.
- [ ] Hoist the link-normalization env-path whitelist to once per stage.
- [ ] Transclusion prepare walks the directive slice once, partitioned by
  kind.
- [ ] Introduce the canonicalization memo on the run cache (session
  migration happens in D5): keyed by resolution-base identity plus input,
  no case-folding or symlink merging, failed entries not retained across
  mutation-capable stages; invalidate successful entries too on target
  replacement or symlink retargeting.

### L1 tests

- [ ] Plain-document compose: zero body-sized copies, zero error-context
  constructions, zero shell-stage work (counter assertions).
- [ ] Cleanup no-op path only for byte-identical fixtures; corpus output
  equivalence.
- [ ] Forced failures in shell, transclusion, link, and schema stages render
  blocks identical to current fixtures; repeated errors reuse one context.
- [ ] Canonicalization memo: repeated requests hit; relative spellings in
  two directories do not alias; failure-then-create revalidates.

### Checkpoint D1

`just test`/`just lint` in `darkmatter`; corpus byte-equivalence suite
green; no public API change yet.

## Phase D2 — Shared reference parse (F1)

### Production changes

- [ ] Build one MDAST product, one pulldown event stream, and one shared
  `LineIndex` per body version/stage; convert the eight HTML extractors and
  two inline extractors into classifiers over those products.
- [ ] Key cross-stage reuse by content identity and parser options; attach
  source-specific provenance per consumer; link-normalization reuses
  link-resolve products when the body is byte-identical.
- [ ] Preserve extractor-group output ordering explicitly.

### L1 tests

- [ ] Parse-count bounds: at most one parse per parser family per body
  version/stage across linkless, link-heavy, and HTML-reference documents;
  extractor count does not affect the bound.
- [ ] Reference records, spans, provenance, and ordering identical to
  current output over the full reference corpus.
- [ ] Mutated-body-between-stages fixture forces a re-parse (identity
  mismatch); identical bodies at different paths retain correct provenance.

### Checkpoint D2

`just test`/`just lint` in `darkmatter`; internal extractor API changes
only; `md` CLI behavior unchanged.

## Phase D3 — Transclusion runtime and cache purity (F4)

### Production changes

- [ ] Phase-wide base options hash plus per-directive overlay hashes;
  memoize `classify_options`/fingerprints per phase.
- [ ] Hoist the `when:` expression resolution context and the transclusion
  options clone per phase.
- [ ] Replace whole-runtime clone/merge locking with immutable shared
  handles plus child-local deltas folded in prepared order; keep narrow
  locks only for genuinely shared facilities.
- [ ] Move target-content reads and hashing inside the cached
  computation/revalidation path. Same-epoch immutable input reuse avoids
  duplicate reads; strict persistent validation still hashes current bytes.
  Separate validation reads, artifact reads, and avoided computation.

### L1 tests

- [ ] Same-epoch input reuse: zero duplicate target reads. Strict persistent
  lookups count all validation reads and detect same-size/restored-mtime edits
  to sources and dependencies; TTL and remote freshness behavior stay intact.
- [ ] Distinct `set=`/replacement overlays produce distinct cache
  identities; identical directives share the phase base.
- [ ] Parallel fanout determinism: merge ordering identical to today;
  cycle, dependency, cache-stat, and shell semantics preserved.

### Checkpoint D3

`just test`/`just lint` in `darkmatter`; transclusion-fanout diagnostic
rerun and recorded.

## Phase D4 — Demand-driven capture defaults (F5) — breaking

### Production changes

- [ ] Preserve already-minimal `ComposeOptions::new()`; require explicit
  context or requirements for bare `EffectiveStateBuilder`, migrating callers
  without silently blanking groups; retain explicit full capture.
- [ ] One environment snapshot per session/run shared by all context
  construction; immutable overlays for provider/model and document values.
- [ ] Spawn the docs group with its peers in the capture scope.
- [ ] If justified by D0, dirty-region interpolation with boundary-context
  windows and conservative full-scan fallback; otherwise record deferral.
- [ ] Migrate every monorepo caller of the changed constructors (Claudine
  `overlay.rs` harness materialization is expected to be covered by the
  fix; verify and close any remainder).

### L1 tests

- [ ] No default/convenience entry performs a ten-group capture; a
  `ctx.datetime`-only document performs zero host probes; named
  full-capture still captures every group.
- [ ] One environment snapshot per session across multiple context
  constructions.
- [ ] If implemented, regional interpolation: cross-boundary delimiter formation and
  fence/classification changes match full-scan output via the fallback.

### Checkpoint D4

`just test`/`just lint` in `darkmatter` and every migrated consumer's
package area; GitNexus confirms no un-migrated caller.

## Phase D5 — Compose session and cross-document caches (F6) — breaking

### Production changes

- [ ] Introduce the session type (`Arc`-backed, `Send + Sync`,
  single-flight) carrying retained evidence and per-document resolution views,
  environment snapshot,
  canonicalization memo (moved from D1's run scope), layered schema-source
  cache, trigger-discovery cache, and policy-compatible remote/store handles.
  Keep cycle stacks, effects, and reports decision-local.
- [ ] Bound cache memory and parse-product lifetime, define eviction, and
  make single-flight failure/cancellation/panic release waiters. Detect
  recursive same-key dependencies before waiting; allow retry after failure.
- [ ] Partition semantic results by source/caller provenance, ordered roots,
  fallback context, enabled operations, dry-run and remote/effect policy.
  Reuse passive parsing independently from effectful execution.
- [ ] Schema-source cache: bytes and passive trees keyed by canonical
  source identity plus freshness evidence; resolved/converted schemas
  additionally keyed by ordered roots, file-resolution context/fallback
  identity, meta-schema controls, and imported-content identities;
  named-type imports share the source cache.
- [ ] Trigger cache keyed by boundary, ordered nearest-first root vector,
  file-resolution context identity, and freshness evidence; per-directory
  membership revalidation including missing roots and ignore/config inputs.
  Without trustworthy change evidence, reread/hash files and rewalk membership
  at decision/mutation boundaries; keep parsed products keyed by content.
  Validate evidence at publication to avoid publishing a raced snapshot.
- [ ] Make `ComposeOptions` construction session-explicit; convenience
  paths create a private session; migrate Darkmatter CLI, DMLS, and
  monorepo consumers.
- [ ] Thread Claudine's one-session-per-invocation through compose,
  inline-compose, sequence steps, system-prompt/appendix composition, and
  harness materialization (consuming the fix's invocation owner).

### L1 tests

- [ ] Multi-document run sharing one `$schema` and one trigger root: one
  parse per resident content identity and one walk per valid membership epoch,
  with required validation reads separately counted; schema
  modification, trigger add/remove, and root-order change each invalidate
  only the affected entry and produce current output.
- [ ] Resolution-context change forces separate resolved-schema entries
  (no wrong-context reuse).
- [ ] Concurrent cold-entry population is single-flight; cancellation,
  failure, recursive cycles, and retry release waiters. Cache eviction bounds
  retained bytes and preserves output.
- [ ] Cross-repository source views share a session without changing launch
  `ctx.*`, live `current.*`, finalized sigils, caller provenance, or denials.
- [ ] Same-size/restored-mtime edits, missing-root creation, nested trigger
  shadowing, and symlink retargeting invalidate correctly.
- [ ] JIT/retry/resume boundaries: documents reread; supporting-input reuse
  never crosses a required freshness boundary stale.

### Checkpoint D5

`just test`/`just lint` in `darkmatter` and consumers; sequence-sharing
diagnostic recorded.

## Phase C1 — Preflight graph reuse and single-buffer loading (F7)

### Production changes

- [ ] Retain the existing collector report through Claudine authorization and
  hand compatible graph evidence into `ComposeOptions`; extend immutable
  source/parse retention only where D0 proves repeated work. Preserve the
  canonical validation gate and reparse spans after body mutation.
- [ ] Revalidate affected artifacts after changed source, overlays, interactive
  input, `initialize`, or proxy handoff. Unknown/unapproved commands must fail
  or obtain approval before execution. No effects execute to build a plan.
- [ ] Load Markdown/YAML source bytes once per decision; construct original
  text and the parsed document fallibly from that buffer, preserving source
  path, format errors, BOM/frontmatter behavior, and biscuit-file conversion.
- [ ] Preserve `DocumentEpoch` snapshot reuse and demand extension across
  preflight/body/lifecycle; retain live `current.*` separately.
- [ ] Share terminal/`FORCE_COLOR` capability by sink and policy using captured
  environment; stdout and stderr remain independent.

### L1 tests

- [ ] One canonical full compose, bounded collector/resolution passes, and one
  source read per file-backed decision. Compare command identity/order and
  provenance with the current collector; exercise Markdown and YAML errors.
- [ ] Dead branches, nested/lifecycle commands, dynamic command-shape errors,
  dry-run, denied commands, and post-initialize changes preserve approval and
  effect ordering. A changed body never reuses obsolete replacement spans.
- [ ] JIT/retry/resume/proxy and interactive-input changes begin or invalidate
  the appropriate decision; no fresh source or epoch value is skipped.
- [ ] Existing epoch-context and composition/lifecycle/system-prompt suites
  remain green under nextest concurrency, using hermetic CLI fixtures.

### Checkpoint C1

`just test`/`just lint` in `claudine`; `just _test claudine-cli
--no-fail-fast` from the repository root.

## Phase S1 — Sniff structure tier (F8) — parallel track

### Production changes

- [ ] Marker-only parallel observation on the shared walker machinery
  producing nested-marker candidates and manifest-directory evidence;
  reuse compatible existing `RepoEvidence` without any extra walk;
  membership-glob expansion consumes it; no inventory/classification/typed
  parsing paid by this observation.
- [ ] Use conservative syntax-aware pre-filters in nested candidate detectors
  when measured beneficial;
  inconclusive text falls back to the current parser; malformed-manifest
  errors preserved, including escaped JSON and quoted/dotted TOML keys.
  Keep the current parser if extra syntax validation erases the benefit.
- [ ] Tier-gate name/version resolution with the explicit-absence API
  decision (lightweight topology result or optional fields); Claudine
  declares its actual reads.
- [ ] Gate `ExecutableIndex::build_path_only` behind requests consuming
  executable provenance.
- [ ] Memoize the ownership index on `RepoInfo`; per-package
  canonicalization happens once per immutable topology view. Encapsulate
  mutation/invalidation or use a caller-owned index; define clone/serde behavior.
- [ ] Route `detect_area` (and other single-answer helpers) through the
  structure tier.

### L1 tests

- [ ] Counter bounds on the large fixture: at most one added parallel marker
  walk (zero with compatible evidence);
  raw/syntax/typed counts split; typed parses bounded by
  detection and requested metadata needs when pre-filtering is beneficial;
  zero glob walks with evidence; zero PATH
  scans without provenance requests; name/version parses only on request.
- [ ] Pre-filter correctness: valid workspace descriptors never rejected;
  inconclusive falls back; malformed manifests error as today.
- [ ] Detection decisions and all requested fields identical across tiers
  on the fixture corpus; declined fields explicitly absent.
- [ ] Repeated `package_for_dir`: one index build, flat per-package
  canonicalization, at most one query-path canonicalization per call.
- [ ] `detect_area` current answers via the structure tier.
- [ ] Literal members without manifests, dialect/exclusion/ignore semantics,
  incomplete evidence fallback, nested roots, and deterministic order match.
- [ ] Index lookup after supported mutation, clone, and deserialization is
  current; no stale cache behind public mutable fields.

### Checkpoint S1

`just test`/`just lint` in `sniff`; work-count example
(`work_counts.rs`) rerun and recorded.

## Phase D6 — Documentation and final verification

### Documentation/comment pass

- [ ] `.claude/skills/darkmatter/compose.md` and the darkmatter skill
  overview: session authority, demand-driven default capture, cache and
  freshness contract, existing preflight-artifact seam.
- [ ] `.claude/skills/sniff/`: tier semantics, marker-only observation,
  counter split, lookup memoization, explicit absence.
- [ ] `.claude/skills/claudine/composition.md` and architecture docs:
  preflight-artifact reuse and session handoff.
- [ ] Package READMEs and `docs/dependencies.md` where surfaces or crate
  boundaries changed.
- [ ] Delete or correct every comment describing the removed double passes,
  eager captures, pre-cache reads, and per-lookup index rebuilds; retain comments
  documenting required strict validation reads and live lifecycle capture.

### Package gates

Run in dependency order:

```sh
cd sniff
just test
just lint

cd ../darkmatter
just test
just lint

cd ../claudine
just test
just lint

cd ..
just _test claudine-cli --no-fail-fast
```

Use `just test-l2`/`just test-browser` only where a real terminal or
headless browser is part of an asserted contract. No test may focus a
terminal or browser window.

### Final acceptance audit

- [ ] Map every spec acceptance criterion to a named test, counter
  assertion, diagnostic result, or documentation change.
- [ ] Rerun the D0 baselines; record before/after work counts (regression
  gate) and wall-clock (evidence).
- [ ] `git diff --check`; inspect for unrelated changes and stale comments.
- [ ] GitNexus `detect_changes()` sweep; expand verification if reach exceeds
  `sniff`, `darkmatter`, and the Claudine area.

## Acceptance-to-phase map

| Spec area | Phase | Primary proof |
|---|---:|---|
| Extractor-independent parse bound, ordering preserved | D2 | parse counters + reference-corpus equivalence |
| No-op stages free; lazy memoized error contexts | D1 | plain-document counters + forced-error fixtures |
| Loop-invariant hoisting; overlay-distinct cache identity | D1, D3 | phase-base/overlay tests + memo tests |
| Same-epoch input reuse; strict validation retained; delta merge | D3 | read/probe counters + fanout determinism |
| Demand-driven defaults; shared env; measured interpolation decision | D4 | capture counters + fallback fixtures |
| Session with schema/trigger/canonicalization caches and freshness | D5 | stable-input counts + invalidation tests |
| Collector reuse, one read, preserved epoch context | C1 | scoped seams + approval identity/provenance equivalence |
| Structure tier cost and explicit absence | S1 | split counters + corpus equivalence |
| Scoped work-count regression coverage | D0 + all | counter assertions, no globals, no elapsed time |
| Consumer migration and gates | D4, D5, C1, S1, D6 | package gates + no-fail-fast CLI run |
| Before/after evidence | D0, D6 | recorded counter and diagnostic deltas |

## Principal risks and controls

| Risk | Control |
|---|---|
| Shared AST walk changes reference record ordering | Ordering is an explicit contract; corpus equivalence includes order |
| Cleanup no-op predicate skips a real normalization | Predicate must prove byte-identity; corpus before/after byte comparison |
| Canonicalization memo aliases relative spellings or symlinks | Key by resolution-base identity; no case-folding/merging; alias fixtures |
| Stale memo entry after a stage creates the target | Invalidate successful and failed entries at mutation/freshness barriers |
| Base/overlay hash collapses distinct directive options | Complete-key constraint; distinct-overlay identity tests |
| Delta-based merge changes deterministic ordering | Fold in prepared-item order; fanout determinism tests |
| Removing eager capture breaks a hidden dependent | GitNexus caller sweep per breaking phase; named full-capture API retained; consumers migrated in the same change set |
| Session cache crosses a lifecycle freshness boundary | Documents never session-cached; boundary tests for schema/trigger reuse; per-directory trigger revalidation |
| Collector reuse weakens shell approval | Existing condition-blind collector and normalized identity retained; security suites unchanged |
| Collector evidence drifts from executed content | Reuse compatible collector evidence; retain validation gates; refresh spans and audit changed inputs |
| Manifest pre-filter rejects valid syntax | Conservative no-false-negative rule; inconclusive falls back; workspace-descriptor corpus test |
| Tier-gated metadata fabricates values | Explicit-absence API decision (lightweight result or optional fields) made before implementation |
| Parallel marker walk changes ignore semantics | Same walker configuration as the shared view; result equivalence on fixtures |
| Feature absorbs fix scope (or vice versa) | Fix owns probe count/propagation; feature owns probe and pipeline cost; boundary stated in both specs |
| Counters make tests flaky under parallelism | Session/request-scoped counters only; single-flight assertions use controlled concurrency |

### Added acceptance mapping

| Contract | Phase | Proof |
|---|---|---|
| Already-shipped work excluded from gains | D0, D6 | Current-source baseline and measured deltas |
| Strict freshness includes same-metadata edits | D3, D5 | Full-lookup read counters and mutation fixtures |
| Launch/source/live context and policy remain distinct | D5, C1 | Cross-repository and policy-denial fixtures |
| Bounded memory; cancellation/cycle-safe single-flight | D5 | Eviction and controlled-concurrency tests |
| Existing collector reused without weakening validation | C1 | Dead-branch, dynamic-shape, lifecycle and mutation tests |
| Ownership index remains current | S1 | Mutation/clone/deserialization tests |
| Regional interpolation earns its complexity | D0, D4, D6 | Implementation evidence or explicit measured deferral |
