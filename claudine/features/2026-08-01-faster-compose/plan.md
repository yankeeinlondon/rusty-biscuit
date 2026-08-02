# Faster-compose implementation plan

Reference: [`spec.md`](spec.md)

## Review conclusion

The architect's round-1 revisions are accepted in full. Source inspection
supports each tightened claim:

- parallel transclusion children compose outside the shared `PipelineRuntime`
  mutex but serialize their runtime snapshot and merge through it, so the
  correct target is the clone/merge funnel, not "children compose under a
  lock";
- code transclusion reads and hashes target bytes before the run-local cache
  lookup, and strict persistent-cache validation rereads operation sources, so
  the zero-read bound had to be stated as zero *content* reads on a
  *validated* hit with metadata probes still permitted;
- the reference extractors are two parser families (MDAST and pulldown), so
  the F1 bound is "at most one parse per required parser family per body
  version/stage," not "one parse total";
- `Package.name` is a required field in Sniff's package model, so tier-gated
  name/version resolution needs an explicit lightweight-topology or
  optional-field API decision rather than placeholder values; and
- the added cost/freshness terminology (document decision, content read
  versus metadata probe, stable-input count) is what makes the counter bounds
  testable without ambiguity.

One formatting correction was applied during this review (stray continuation
indentation in the F6 freshness-contract paragraph). No architect content was
rejected.

Symbols and paths in this plan were checked against the audit of 2026-08-01
on commit `0f6ab97` plus the architect's working-tree revision. Locate
implementation sites by symbol when lines move.

## Dependency on the propagated-context fix

The fix's plan (`../../fixes/2026-08-01-propagated-context/plan.md`) lands
the invocation owner, the seeded Sniff observation seam, and Darkmatter's
evidence-aware demand capture. This feature builds on those seams:

- Phases D1–D3 here (Darkmatter internals) and Phase S1 (Sniff structure
  tier) are independent of the fix and may proceed in parallel with it.
- Phase D4 (capture defaults) extends the fix's evidence-aware capture and
  should follow the fix's Phase 3.
- Phase D5 (compose session) consumes the fix's invocation owner as its
  evidence source and must follow the fix's Phase 2.
- Phase C1 (Claudine one-pass) consumes the session and must follow D5.

If the fix's implementation is deferred, D5 may substitute a
session-constructed evidence capture behind the same seam, but the
propagation boundary defined by the fix remains authoritative — this feature
must not introduce its own ambient discovery to compensate.

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
- a validated transclusion cache hit performs zero target-content reads;
  parallel children fold child-local deltas in prepared order instead of
  serializing whole-runtime clone/merge on one mutex;
- no default or convenience Darkmatter entry point performs the eager
  ten-group host capture; one environment snapshot per session is shared with
  immutable overlays; docs capture is spawned with its peers; interpolation
  re-scans only dirty regions with a conservative full-scan fallback;
- a compose session (cheap `Arc`-backed handle, `Send + Sync`, single-flight
  population) carries propagated evidence, the canonicalization memo, the
  layered schema-source cache, and the trigger-discovery cache, with the
  spec's freshness contract enforced at every lifecycle boundary;
- Claudine prepares each file-backed document decision from at most one
  source-byte read, one compose plan/pipeline execution, and one captured
  runtime base, with byte-for-byte shell approval, static preflight,
  dry-run, and lifecycle semantics preserved;
- the Sniff structure tier uses one parallel marker-only observation,
  conservative raw-text pre-filtering, tier-gated metadata with explicit
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

## Verified impact surface

Confirmed hot sites by area (symbols verified against source):

Darkmatter (`darkmatter/lib/src/markdown/`):

- `compose/link_resolve.rs` and `compose/link_normalization.rs` — ten
  independent extractor parses each via `reference/html.rs`
  (`collect_from_html_nodes` → `output::parse_mdast`, eight extractors) and
  `reference/local.rs` (`extract_inline_refs`, links + images);
- `markdown/mod.rs` — `source_context_for_errors` (body `Arc` +
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
  `ComposeContext::capture`), `compose/context/effective_state.rs`
  (fallback capture), `compose/context/runtime.rs` (`std::env::vars()` per
  construction), `compose/context/capture/snapshot.rs` (docs group captured
  serially);
- `schemas/resolve.rs` (`load_schema_from_path_in_context`,
  `load_named_types` — per-document reads/parses) and
  `schemas/triggers/discovery.rs` (per-document root walks and file reads).

Claudine (`claudine/cli/src/commands/`):

- `compose/prep.rs` — discovery compose pass (`resolve_shell_approvals`
  with deferred schema verdicts) plus canonical `prepare_staged` pass;
  duplicate `capture_for_document` calls; four `FORCE_COLOR` probes;
- `claudine/lib/src/composition/resolve.rs` —
  `resolve_composition_source_in_context` double read
  (`fs::read_to_string` + `Markdown::try_from`), `reload_composition_source`
  repeating the pattern;
- `wrap/composition/pipeline.rs` — third near-identical lifecycle capture.

Sniff (`sniff/lib/src/filesystem/repo/`):

- `nested.rs` (`walk_for_nested_markers` — serial `WalkBuilder`),
  `glob.rs` (`walk_manifest_dirs` — per-pattern serial walks),
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

One session type in Darkmatter owns propagated evidence and every
invocation-scoped cache (canonicalization memo, schema-source cache,
trigger-discovery cache, environment snapshot). It is a cheap cloneable
`Arc`-backed handle, `Send + Sync`, with single-flight population.
`ComposeOptions` construction becomes session-explicit; convenience entry
points create a private single-use session. Claudine creates exactly one per
CLI invocation, fed by the fix's invocation owner.

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
runtime) requires them. Lock-free structures are not required. Content reads
move inside the cached computation/revalidation path; `Strict` validation
semantics are unchanged.

### 6. Capture is demand-driven by default, explicit when full

`ComposeOptions::new()` and the `EffectiveStateBuilder` fallback stop
performing the eager ten-group capture. Explicitly named full-capture APIs
remain. One process-environment snapshot per session; provider/model,
step, and document values are immutable overlays. The docs group spawns with
the other expensive groups. The interpolation loop tracks dirty regions with
boundary context and falls back to a full scan whenever regional stability
cannot be proven.

### 7. Claudine consumes one prepared plan per document decision

Darkmatter exposes a reusable prepared plan carrying the exact
shell-approval inventory and all pre-approval work; canonical execution
consumes the same plan after approval. One content read constructs both
original text and parsed Markdown. One captured runtime base serves
shell-preflight options, prepared context, and lifecycle evaluation.
Terminal capability is snapshotted per output sink and policy.

### 8. Sniff structure work is tier-shaped with explicit absence

The structure tier runs one marker-only parallel observation (same walker
machinery as the shared system view) feeding both nested-marker candidates
and manifest-directory evidence. Raw-text pre-filters are conservative (no
false negatives for accepted syntax; inconclusive falls back to the current
parser; malformed-manifest errors preserved). Declined name/version/
executable metadata is represented as absent via a lightweight topology
result or optional fields — never fabricated. `RepoInfo` lookups memoize
one ownership index per instance. `detect_area` runs the structure tier.

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
    -> demand-driven capture defaults (D4, after fix phase 3)
    -> compose session and cross-document caches (D5, after fix phase 2)
    -> Claudine one-pass / one-read / one-capture (C1, after D5)
    -> documentation and final gates (D6)

Sniff structure tier (S1) proceeds in parallel from D0.
```

D1–D3 are internal to Darkmatter and independently mergeable. D4 is the
first breaking phase; D5 the second; C1 and S1 carry the remaining breaks.

## Phase D0 — Baselines and counter seams

### Production changes

- [ ] Add the request/session-scoped Darkmatter work counters (full-document
  parses per parser family, context captures, environment snapshots, schema
  content reads/parses, trigger-root walks/file reads, canonicalization
  hits/misses, target-content reads versus metadata probes, body-sized
  copies) on the run cache so later phases can assert against them.
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
  mutation-capable stages.

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
- [ ] Key cross-stage reuse by content identity; link-normalization reuses
  link-resolve products when the body is byte-identical.
- [ ] Preserve extractor-group output ordering explicitly.

### L1 tests

- [ ] Parse-count bounds: at most one parse per parser family per body
  version/stage across linkless, link-heavy, and HTML-reference documents;
  extractor count does not affect the bound.
- [ ] Reference records, spans, provenance, and ordering identical to
  current output over the full reference corpus.
- [ ] Mutated-body-between-stages fixture forces a re-parse (identity
  mismatch).

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
  computation/revalidation path; validated hits perform metadata probes
  only.

### L1 tests

- [ ] Validated run-local and persistent hits: zero target-content reads,
  metadata probes visible, `Strict` semantics unchanged (cold strict
  identity establishment still reads).
- [ ] Distinct `set=`/replacement overlays produce distinct cache
  identities; identical directives share the phase base.
- [ ] Parallel fanout determinism: merge ordering identical to today;
  cycle, dependency, cache-stat, and shell semantics preserved.

### Checkpoint D3

`just test`/`just lint` in `darkmatter`; transclusion-fanout diagnostic
rerun and recorded.

## Phase D4 — Demand-driven capture defaults (F5) — breaking

### Production changes

- [ ] Remove the eager ten-group capture from `ComposeOptions::new()` and
  the `EffectiveStateBuilder` fallback; add/retain explicitly named
  full-capture construction.
- [ ] One environment snapshot per session/run shared by all context
  construction; immutable overlays for provider/model and document values.
- [ ] Spawn the docs group with its peers in the capture scope.
- [ ] Dirty-region interpolation with boundary-context windows and
  conservative full-scan fallback.
- [ ] Migrate every monorepo caller of the changed constructors (Claudine
  `overlay.rs` harness materialization is expected to be covered by the
  fix; verify and close any remainder).

### L1 tests

- [ ] No default/convenience entry performs a ten-group capture; a
  `ctx.datetime`-only document performs zero host probes; named
  full-capture still captures every group.
- [ ] One environment snapshot per session across multiple context
  constructions.
- [ ] Regional interpolation: cross-boundary delimiter formation and
  fence/classification changes match full-scan output via the fallback.

### Checkpoint D4

`just test`/`just lint` in `darkmatter` and every migrated consumer's
package area; GitNexus confirms no un-migrated caller.

## Phase D5 — Compose session and cross-document caches (F6) — breaking

### Production changes

- [ ] Introduce the session type (`Arc`-backed, `Send + Sync`,
  single-flight) carrying propagated evidence, environment snapshot,
  canonicalization memo (moved from D1's run scope), layered schema-source
  cache, trigger-discovery cache, and existing remote/cache handles.
- [ ] Schema-source cache: bytes and passive trees keyed by canonical
  source identity plus freshness evidence; resolved/converted schemas
  additionally keyed by ordered roots, file-resolution context/fallback
  identity, meta-schema controls, and imported-content identities;
  named-type imports share the source cache.
- [ ] Trigger cache keyed by boundary, ordered nearest-first root vector,
  file-resolution context identity, and freshness evidence; per-directory
  membership revalidation with walk fallback.
- [ ] Make `ComposeOptions` construction session-explicit; convenience
  paths create a private session; migrate Darkmatter CLI, DMLS, and
  monorepo consumers.
- [ ] Thread Claudine's one-session-per-invocation through compose,
  inline-compose, sequence steps, system-prompt/appendix composition, and
  harness materialization (consuming the fix's invocation owner).

### L1 tests

- [ ] Multi-document run sharing one `$schema` and one trigger root: one
  schema content read/parse and one trigger walk while stable; schema
  modification, trigger add/remove, and root-order change each invalidate
  only the affected entry and produce current output.
- [ ] Resolution-context change forces separate resolved-schema entries
  (no wrong-context reuse).
- [ ] Concurrent cold-entry population is single-flight.
- [ ] JIT/retry/resume boundaries: documents reread; supporting-input reuse
  never crosses a required freshness boundary stale.

### Checkpoint D5

`just test`/`just lint` in `darkmatter` and consumers; sequence-sharing
diagnostic recorded.

## Phase C1 — Claudine one-pass, one-read, one-capture (F7)

### Production changes

- [ ] Add the Darkmatter prepared-plan seam; compose once per document
  decision, extract the exact shell-approval inventory from the plan, and
  execute the same plan post-approval. Preserve byte-for-byte approval,
  sequence static-preflight snapshots, and `--dry-run` semantics.
- [ ] Single content read per resolution/reload constructing original text
  and parsed Markdown (including YAML-origin documents) from one buffer.
- [ ] One captured runtime base per decision shared by shell-preflight
  options, prepared context, and lifecycle evaluation, with immutable
  overlays.
- [ ] Per-sink terminal/`FORCE_COLOR` snapshot carried on the preparation
  context.

### L1 tests

- [ ] One pipeline execution and at most one source-byte read per
  file-backed decision (scoped seam assertions); identical approval
  inventory bytes; approval-then-execute equivalence for shell documents.
- [ ] Post-`initialize`/retry/resume/JIT boundaries each begin a new
  decision with exactly one reread.
- [ ] Shared capture observed by all three consumers; redirected stdout
  versus terminal stderr retain distinct capability answers.
- [ ] Real-CLI regression: composition, sequence, lifecycle, and system
  prompt suites unchanged; no timing regressions under concurrent nextest.

### Checkpoint C1

`just test`/`just lint` in `claudine`; `just _test claudine-cli
--no-fail-fast` from the repository root.

## Phase S1 — Sniff structure tier (F8) — parallel track

### Production changes

- [ ] Marker-only parallel observation on the shared walker machinery
  producing nested-marker candidates and manifest-directory evidence;
  membership-glob expansion consumes it; no inventory/classification/typed
  parsing paid by this observation.
- [ ] Conservative raw-text pre-filters in nested candidate detectors;
  inconclusive text falls back to the current parser; malformed-manifest
  errors preserved.
- [ ] Tier-gate name/version resolution with the explicit-absence API
  decision (lightweight topology result or optional fields); Claudine
  declares its actual reads.
- [ ] Gate `ExecutableIndex::build_path_only` behind requests consuming
  executable provenance.
- [ ] Memoize the ownership index on `RepoInfo`; per-package
  canonicalization happens once per instance.
- [ ] Route `detect_area` (and other single-answer helpers) through the
  structure tier.

### L1 tests

- [ ] Counter bounds on the large fixture: one parallel marker-only walk;
  raw/syntax/typed counts split; typed parses bounded by
  workspace-relevant manifests; zero glob walks with evidence; zero PATH
  scans without provenance requests; name/version parses only on request.
- [ ] Pre-filter correctness: valid workspace descriptors never rejected;
  inconclusive falls back; malformed manifests error as today.
- [ ] Detection decisions and all requested fields identical across tiers
  on the fixture corpus; declined fields explicitly absent.
- [ ] Repeated `package_for_dir`: one index build, flat per-package
  canonicalization, at most one query-path canonicalization per call.
- [ ] `detect_area` current answers via the structure tier.

### Checkpoint S1

`just test`/`just lint` in `sniff`; work-count example
(`work_counts.rs`) rerun and recorded.

## Phase D6 — Documentation and final verification

### Documentation/comment pass

- [ ] `.claude/skills/darkmatter/compose.md` and the darkmatter skill
  overview: session authority, demand-driven default capture, cache and
  freshness contract, prepared-plan seam.
- [ ] `.claude/skills/sniff/`: tier semantics, marker-only observation,
  counter split, lookup memoization, explicit absence.
- [ ] `.claude/skills/claudine/composition.md` and architecture docs:
  one-compose-per-decision and session handoff.
- [ ] Package READMEs and `docs/dependencies.md` where surfaces or crate
  boundaries changed.
- [ ] Delete or correct every comment describing the removed double passes,
  eager captures, pre-cache reads, and per-lookup index rebuilds.

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
- [ ] GitNexus `detect-changes` sweep; expand verification if reach exceeds
  `sniff`, `darkmatter`, and the Claudine area.

## Acceptance-to-phase map

| Spec area | Phase | Primary proof |
|---|---:|---|
| Extractor-independent parse bound, ordering preserved | D2 | parse counters + reference-corpus equivalence |
| No-op stages free; lazy memoized error contexts | D1 | plain-document counters + forced-error fixtures |
| Loop-invariant hoisting; overlay-distinct cache identity | D1, D3 | phase-base/overlay tests + memo tests |
| Zero-content-read validated hits; delta-based merge | D3 | read/probe counters + fanout determinism |
| Demand-driven default capture; one env snapshot; dirty-region interpolation | D4 | capture counters + fallback fixtures |
| Session with schema/trigger/canonicalization caches and freshness | D5 | stable-input counts + invalidation tests |
| One read/plan/capture per document decision | C1 | scoped seams + approval-bytes equivalence |
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
| Stale memo entry after a stage creates the target | Failed lookups not retained across mutation-capable stages; epoch advance at freshness boundaries |
| Base/overlay hash collapses distinct directive options | Complete-key constraint; distinct-overlay identity tests |
| Delta-based merge changes deterministic ordering | Fold in prepared-item order; fanout determinism tests |
| Removing eager capture breaks a hidden dependent | GitNexus caller sweep per breaking phase; named full-capture API retained; consumers migrated in the same change set |
| Session cache crosses a lifecycle freshness boundary | Documents never session-cached; boundary tests for schema/trigger reuse; per-directory trigger revalidation |
| One-pass prepare weakens shell approval | Approval inventory extracted from the same plan, byte-compared to the current pass; security suites unchanged |
| Prepared plan drifts from executed content | Plan is the executed artifact; no recompose between approval and execution within a decision |
| Manifest pre-filter rejects valid syntax | Conservative no-false-negative rule; inconclusive falls back; workspace-descriptor corpus test |
| Tier-gated metadata fabricates values | Explicit-absence API decision (lightweight result or optional fields) made before implementation |
| Parallel marker walk changes ignore semantics | Same walker configuration as the shared view; result equivalence on fixtures |
| Feature absorbs fix scope (or vice versa) | Fix owns probe count/propagation; feature owns probe and pipeline cost; boundary stated in both specs |
| Counters make tests flaky under parallelism | Session/request-scoped counters only; single-flight assertions use controlled concurrency |
