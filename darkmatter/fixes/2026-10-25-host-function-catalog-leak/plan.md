---
area: darkmatter
fix: 2026-10-25-host-function-catalog-leak
total_phases: 4
created: 2026-10-26
phase: 1
agent: "codex/gpt-6.1-sol"
yolo: true
human_review: false
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
---

# Host function catalog ownership plan

## Phase 1 — Establish contracts and verification

Replace the expression catalog's leaked storage with one owned representation,
migrate the shared metadata API and every repository consumer, and retain only
the latest host function result per discovery path. Preserve expression behavior,
metadata ordering, generated text, diagnostics, and facts-artifact bytes.

Successful completion means old host catalog owners become unreachable after
replacement and release of caller-held values, while deliberately held older
sets continue to work. Failed construction releases partial allocations. Static
catalog APIs retain permanent borrowed slices and lifecycle string views.
Darkmatter and Claudine pass their scoped L1 and lint checks, both generator
examples compile, and generated regions remain byte-identical. No dependencies,
unsafe lifetime extensions, artifact-format changes, or terminal/browser sessions
are needed.

### Decisions

1. **One owned model**
   - **Decision:** Use `Box<str>`, boxed slices, and owned `FunctionType` values
     throughout `FunctionCatalog`, categories, parameters, descriptors, and
     examples, including aliases and display-only reasons. Projection helpers
     borrow inputs only while constructing independent owned results.
   - **Alternatives considered:** A separate host-only model; `Arc<str>` fields;
     leaked storage with a compatibility API.
   - **Why:** The specification settles this design; the same loader serves
     embedded, mutation, and host catalogs, and host catalogs already share their
     complete owner through `Arc<CatalogData>`.
   - **Downsides:** Public field types change and repeated text can have separate
     allocations. Consumers must borrow explicitly or deliberately clone.

2. **Borrowed metadata views**
   - **Decision:** Give `Example` and `ExampleVerification` a lifetime parameter;
     keep them `Copy` borrowed views. Change `Described` string methods to
     `&str` and `example()` to `Option<Example<'_>>`. Remove the descriptor's
     duplicate `example` field and derive its view from the first inline call
     example, retaining the existing verification mapping.
   - **Alternatives considered:** Duplicate owned examples; `Cow<'static, str>`;
     static example strings retained through leaks.
   - **Why:** The specification requires complete ownership, including the less
     obvious shared example fields. Views avoid self-references and duplicate
     storage while allowing static tables to store `Example<'static>`.
   - **Downsides:** Every implementation and direct field consumer must migrate,
     including the implementations in expression semantics, beyond the spec's
     approximate implementation count.

3. **Static owner lifetimes**
   - **Decision:** Keep built-in and mutation catalogs in their existing
     `LazyLock`s. Borrow permanent descriptor and lifecycle string views from
     those concrete static owners. `descriptors_for(&self, ...)` and
     `HostFunctionCatalog::function_names()` instead borrow from `self`.
   - **Alternatives considered:** Shorten all public static APIs; preserve the
     trait's permanent-borrow promise using leaks.
   - **Why:** Ownership and permanent borrowing are compatible when the owner is
     static; the specification explicitly preserves these built-in contracts.
   - **Downsides:** Generic `Described` callers cannot assume permanent borrows.

4. **Latest revision cache**
   - **Decision:** Key the host function map by the existing discovery `PathBuf`.
     Each entry stores the catalog digest, complete `ArtifactKey`, and result.
     Compare and construct under the existing mutex; reuse unchanged successes
     and errors and replace changed results, including success-to-error.
     Use the existing `biscuit_hash::blake3_hash` call without a new hash helper.
   - **Alternatives considered:** Revision history; weak-only entries; global
     eviction; removal based on absence from one discovery; mtime identity.
   - **Why:** This is the settled specification policy. Document-specific roots
     make absence-based eviction unsound, and byte identity detects artifact-only
     changes. Preserve path identity, ordering, and shadowing without normalization.
   - **Downsides:** One result remains per distinct path ever encountered.
     Caller-held old sets retain their owners intentionally. Construction stays
     serialized; two files are not an atomic snapshot. “Latest” means the last
     revision processed under the lock.

5. **Private ownership probes**
   - **Decision:** Add the smallest library-internal `cfg(test)` access needed to
     downgrade `CatalogData` and inspect the production cache's entry for a
     specific path. If the function-local static must become module-private to
     support this, retain its singleton and locking behavior. Never expose a
     production inspection API or clear the shared cache in tests.
   - **Alternatives considered:** A test-only replacement cache; public owner
     accessors; process-memory assertions.
   - **Why:** Acceptance requires the real lookup path. A weak owner probe plus
     owned-field inspection proves reclamation without allocator-dependent noise.
   - **Downsides:** Small internal test seams are needed; a dead weak pointer
     alone cannot prove every nested allocation was owned.

6. **Preserve input outcomes**
   - **Decision:** Keep catalog and artifact parsing, field validation, and
     diagnostics unchanged. Invalid catalogs return the existing typed error;
     missing, unreadable, stale, or invalid facts normally return a successful
     set with incomplete functions. Reuse the existing public-API artifact
     matrix and add only uncovered cases required by changed code.
   - **Alternatives considered:** Turn untrusted facts into catalog failures;
     rebuild every format test around spawned CLIs.
   - **Why:** The spec forbids semantic and format changes. The current
     `host_functions.rs` already has a facts-artifact robustness matrix, and
     `analysis_only` intentionally preserves incomplete analysis.
   - **Downsides:** Assertions must distinguish rejected catalogs from successful
     incomplete sets; the blanket “invalid input errors” rule has this explicit
     existing exception.

7. **Deterministic evidence**
   - **Decision:** Run no spike. Use L1 owner/revision tests, compile-time lifetime
     checks, output parity, and a scoped ownership audit. Keep the excluded
     context-catalog leak and test-only static fixtures unchanged.
   - **Alternatives considered:** RSS benchmarks; allocation benchmarks;
     cross-host performance sampling; a repository-wide leak cleanup.
   - **Why:** The spec explicitly states that deterministic ownership evidence
     answers the retention question and no performance spike is required.
   - **Downsides:** This proves ownership and cache bounds rather than an RSS
     reduction; allocator behavior is outside the acceptance claim.

8. **Facts from `2026-10-25-fix-now`** (recorded after that fix was implemented)
   - **Decision:** `host_function_set`, `ArtifactKey`, and the function cache are
     unchanged by that fix, so Decision 4 stands. For Decision 5, the same file
     now has `catalog::discovery_probe` (`cfg(any(test, feature =
     "work-counters"))`, thread-local): a read log, a build counter, and injected
     read errors for lifecycle discovery. Extend it, or follow its pattern, for
     any host-function observation rather than adding a second seam. The
     lifecycle profile cache is a separate `profile_cache()`; do not merge the two.
   - **Alternatives considered:** A separate test-only seam for host functions;
     merging both caches into one map.
   - **Why:** One observation mechanism per discovery file keeps test seams
     consistent; the two caches have different failure policies (see the spec's
     "Relationship to the lifecycle profile cache").
   - **Downsides:** `discovery_probe` currently instruments only lifecycle reads;
     the artifact read in `host_function_set` still goes through `ArtifactKey::read`
     and would need routing through the probe to be observed.

### Execution rules

Waves have globally unique numbers and run in order. Tasks within a wave may
run concurrently only where explicitly stated; a later wave starts after its
predecessor's checkpoint. Parallel implementation agents must receive the
non-interactive session constraint and bounded file ownership. These are future
implementation assignments, not a requirement to spawn agents during planning.
Stay in this worktree; do not commit, run `cargo fmt`, or move the fix into
`_completed`.

### Wave 1 — Contract inventory

- [x] **Map consumers**
  - Trace descriptor field reads, `Copy` assumptions, `Described` implementations,
    stored `Example` types, and direct `.example` reads across Darkmatter,
    DMLS, generators, tests, and Claudine. Use targeted text search; account for
    all actual implementations rather than relying on the spec's count.
  - Record affected files and existing test names in the implementation log.
    Identify each static API and each owner-borrowed API before editing.
- [x] **Pin preserved output**
  - After consumer inventory, identify existing catalog metadata, example,
    plain `md schema about`, DMLS completion/hover/call-diagnostic, and Claudine
    registration/artifact expectations. Preserve their current expected values.
  - Locate `function_catalog_parity.rs`, `host_functions.rs`,
    `lifecycle_schema_generation.rs`, and
    `dmls/tests/l1/host_function_diagnostics.rs`; identify missing coverage for
    Phase 3. Capture generated output before the refactor if no committed
    expectation already covers it.
  - Inspect the existing public facts matrix against the input coverage below;
    record existing coverage and exact gaps rather than duplicate it.

**Checkpoint:** Every affected ownership contract and verification route is
identified; no unresolved design choices or spikes remain.

## Phase 2 — Migrate owned descriptors and consumers

### Wave 2 — Ownership migration

- [ ] **Own catalog data**
  - Prerequisite: Wave 1. One task owns the shared framework and core catalog
    migration because their lifetime changes are coupled.
  - Convert `catalog/mod.rs`, expression catalog types, `load_catalog`,
    `project_examples`, and `file_example` to the decided model. Remove every
    expression-catalog leak and its helper, including allocations made before a
    later projection error. Keep owned names, examples, aliases, reasons, and
    compiled types independent of loader input.
  - Update all `Described` implementations and static example declarations in
    effects, context, expression semantics, roots where applicable, and fixtures.
    Change direct expression example-field reads to the trait method.
  - Adapt static catalog accessors, effects types, lifecycle signatures and schema
    projection, host registration, and artifact reconstruction. Clone an owned
    `FunctionType` where registration needs an independent value.
  - Correct the loader, host loader, shared trait, and `suggest_strings` lifetime
    comments in the same change. Audit comments on every changed symbol.

**Checkpoint:** The library compiles with owned descriptors and borrowed views.
Static accessors still type-check as permanent slices, and no production leak
primitive remains in the changed loader/projection paths.

## Phase 3 — Replace revisions and prove behavior

### Wave 3 — Independent implementation lanes

The following four tasks are independent after Wave 2. Run up to three worker
agents at once and queue the fourth when the coordinator occupies the remaining
session slot. Keep cache/host
module tests in the first lane, CLI/DMLS/Claudine files in the second, library
integration tests/examples in the third, and docs/skills in the fourth. Coordinate
any newly discovered core API correction through the Wave 2 owner before merging
dependent edits; do not have two lanes edit the core framework simultaneously.

- [ ] **Bound revision retention**
  - Replace the revision-tuple map in `schemas/catalog.rs::host_function_set`
    with one stored entry per path. Preserve `ArtifactKey::{Absent, Unreadable,
    Text}`, including unreadable reasons and exact text comparison.
  - Put lookup, revision comparison, construction, and replacement under the
    existing mutex. Never serve an old success for a new failing revision or
    retain old artifact text/error results after replacement.
  - Add private owner/cache probes and L1 module tests exercising the real
    lookup. Use unique temporary paths, inspect only the tested path's entry,
    and never assert total global cache size or clear it.
  - Test catalog-only edits, artifact-only edits, successful-to-failing catalog
    replacement, unchanged success reuse, unchanged error reuse, and error-to-
    success recovery. Assert stored digest/artifact/result and exactly one
    entry for that path after each transition. Where no owner was constructed,
    assert revision/result rather than attempting to create a weak probe.
    Use private identity or construction-count observation to distinguish an
    unchanged cached error from reconstructing an equal error.
  - Cover facts transitions valid → absent → invalid JSON → stale catalog hash
    → non-UTF-8 bytes → valid, asserting current classifications, diagnostics,
    and recovery. Use a portable byte fixture rather than permissions to make
    the artifact unreadable. Check owner reclamation for catalog-only,
    artifact-only, and failed-catalog replacements, including inline examples
    with display-only reasons.
  - Hold an old set and scope guard during replacement: prove its old
    signatures, registrations, and call analysis remain available, and the new
    set exposes the replacement. Drop every old clone, set, catalog, and guard,
    then require the old weak pointer to fail to upgrade. Prove two distinct
    paths retain independent results and preserve discovery ordering/shadowing.
    Check each set under its intended thread-local scope precedence so a newer
    guard cannot accidentally substitute its registrations for the old set.
  - Correct `host_function_catalogs`' process-lifetime comment and cache comments.

- [ ] **Adapt external consumers**
  - Update Darkmatter CLI descriptor consumers, DMLS completion/hover, and
    actual Claudine metadata consumers for borrowed fields and removed `Copy`
    behavior. Claudine currently registers no host expression functions:
    preserve its host-catalog absence guard, expression rendering, lifecycle
    registrations, and lifecycle-artifact freshness tests; do not add a host
    expression feature to satisfy the spec's historical consumer description.
    Read each area's skill before editing there. Keep user output unchanged.
  - Extend existing entry-point tests only where needed to pin deterministic
    plain `md schema about` output and DMLS completion, hover, and call
    diagnostics. Reuse existing process fixtures; no editor UI is needed.
  - Preserve Claudine registration and facts-artifact expectations without
    regenerating different artifact bytes or accepting new snapshots.

- [ ] **Pin catalog contracts**
  - Add compile checks assigning `expression_function_descriptors()`,
    `function_descriptors()`, and `effect_function_descriptors()` to their
    existing `&'static` slice types, plus lifecycle verb and parameter names to
    `&'static str`. Migrate test fixtures that previously copied descriptors.
  - Test borrowed example selection and verification, including no inline
    example, first of multiple inline calls, error outcome, display-only reason,
    and static views; compare metadata values rather than Rust object bytes.
  - Load from temporary source/path strings, drop those inputs, and assert the
    retained catalog's metadata and registrations through public behavior.
  - Exercise partial construction with an earlier valid inline example and a
    later non-embedded file example; assert the existing typed host load error.
    Keep nested-allocation proof as the scoped ownership audit, not an RSS test.
  - Adapt and compile `expression_doc_generator` and
    `lifecycle_schema_generator`; retain metadata ordering and generated text.
    Complete the bounded input coverage described below using existing tests.

- [ ] **Document ownership contracts**
  - Update `docs/topics/host-functions.md` with owner lifetimes, per-path latest
    revision retention, deliberate survival of held old sets, and error recovery.
    Give compact examples; distinguish incomplete facts from catalog failures.
    Include a Mermaid ownership/replacement flow for developers new to the area.
  - Update `lib/src/catalog/README.md` for borrowed examples and trait methods;
    update `.claude/skills/darkmatter/expressions.md` and
    `library-surfaces.md` for ownership, static views, and cache retention.
  - Check `docs/topics/darkmatter-expressions.md` for affected API claims while
    preserving the generated table. Update the area README only if it contains
    an affected claim; keep it an overview. Docs must describe behavior directly
    and must not link to or name a dated fix. No dependency-doc change is needed
    because this refactor adds or removes no crates.

### Input coverage boundary

This is an ownership/cache refactor of existing format readers, not a new
configuration format. Preserve their validators. Default invalid-format behavior
is an existing located error, with no handler execution and no output artifact
publication. For catalogs, use `HostFunctionCatalog::load` and the production
discovery result to assert the path and offending entry. Do not invent finer
source coordinates than the existing error contract provides.

The explicit exception is facts input: whole-document problems make all host
functions incomplete, and invalid function entries make the affected functions
incomplete, with the existing artifact path/field reason. Missing function entries
are incomplete; empty collections and explicit nullable fields retain their
defined meanings. These cases remain successful `HostFunctions` results, never
silently empty or substituted with a previous successful revision.

Use the real project catalog/facts fixtures under
`lib/tests/fixtures/host_functions/`, keeping a positive unedited control and
one mutation per case. Reuse
`the_facts_artifact_reader_walks_the_input_robustness_matrix` in
`lib/tests/l1/host_functions.rs`: inventory load-bearing fields against the
actual `required`, string, array/object, numeric, and entry validators in
`host/artifact.rs`. For each distinct validator cover absent, null, wrong whole
type, empty, and, for collections, one/all wrong elements once; each additional
field needs a valid control and one invalid wiring case. Include the top-level
`catalog_hash`, `format_version`, and `functions`, function-entry fields, and
nested fact/recovery/exclusion/reference fields. Retain legal explicit nulls
such as absent recovery rules as data, and explicit empty arrays as empty rules.
Explicit null for `conversion_recovery` and `malformed_reference` means no rule;
omitting these fields is an error. Fill actual gaps only; do not multiply
shared-validator cases per field.

For the strict JSON loader, reuse duplicate-key and trailing/invalid-content
cases, including nested duplicates, and verify outcomes via `analysis_only`.
For YAML catalogs, preserve the existing standalone-schema loader's document
shape coverage and catalog envelope/categories/types controls; test ownership
through `HostFunctionCatalog::load`. Do not add a matrix for the function type
text grammar or for generated output fields. Embedded file-form examples use
the existing example validator: retain its public catalog-loading control and
one invalid example wiring case rather than duplicate its schema matrix.

Search changed loading paths for `#[serde(default)]`, `Option<T>` conflation,
`filter_map` with `as_str`, `unwrap_or_default`, and `.ok()` on load-bearing
parses. Classify existing occurrences against current validator coverage; do not
change intentional behavior or unrelated parsers. If a finding identifies a
matrix gap, inspect the whole affected validator row and loader column before
fixing it. Existing CLI/DMLS wiring controls plus one or two representative
invalid cases suffice; the field matrix stays in-process.

**Checkpoint:** All six acceptance groups from the spec have implementation or
test coverage assigned. New L1 cases each take at most five seconds, use no
shared-cache reset, and require no terminal or browser focus.

## Phase 4 — Integrate and verify scoped completion

### Wave 4 — Combined validation

- [ ] **Verify scoped checks**
  - Prerequisite: all Wave 3 lanes integrated. Run `just test` and `just lint`
    in `darkmatter/` and in `claudine/`. Use the recipes' nextest execution;
    Darkmatter includes DMLS. Run any newly added targeted tests first while
    iterating, then the combined scoped suites. Do not add workspace-wide gates.
  - From the repository root, explicitly compile both examples:
    `cargo check --color=never -p darkmatter --example expression_doc_generator --example lifecycle_schema_generator`.
  - Run the generators in their default stdout mode with
    `cargo run --quiet --color=never -p darkmatter --example expression_doc_generator`
    and the corresponding `lifecycle_schema_generator` command. Compare their
    generated regions directly with the committed regions, respecting marker
    framing. Use existing generated-artifact tests, including
    `the_generated_regions_are_byte_identical_to_a_regeneration`; add a direct
    expression-table comparison if existing checks do not cover exact text.
    Do not accept a changed baseline or use write mode to hide drift.
  - Confirm facts-artifact deterministic output and existing Claudine artifact
    tests still pass. Inspect new test durations and split overlong cases at
    independent transition boundaries while preserving coverage.
  - Keep code and tests portable across macOS, Linux, native Windows, and WSL2:
    no permission-dependent unreadability, path normalization, or platform-only
    assertion. Load the `os` skill before any path-comparison/platform-specific
    edit or environment-specific failure investigation. Report the actual host
    evidence without claiming checks on environments not executed.
  - Use the monorepo test toolkit and existing hermetic process fixtures for
    new L1 checks. Repository reads use `include_str!` or literal runtime-root
    joins recognized by test-input discovery; new tests must run from nextest
    archives without compile-time producer paths.

### Wave 5 — Closure audit

- [ ] **Audit final ownership**
  - Prerequisite: Wave 4 passes. Trace every dynamically projected category,
    descriptor, alias, parameter, compiled type, example text, and reason to its
    owner. Check success and partial-failure paths for leaks, global retention,
    unsafe lifetime extension, self-reference, and accidental borrowed input.
  - Verify the cache retains only current revision/result data per path and no
    previous success on failure; inspect guard/set ownership in reclamation
    tests. Limit leak searches to the changed loading paths and preserve the
    spec's excluded static/test sites.
  - Review final API docs, skill guidance, and generated-output comparisons.
    Fix regressions within this scope and rerun the checks they affect.
  - Record changed files, settled decisions, commands/results, generated parity,
    and any spec departure in the implementation log. Check off completed tasks;
    leave the specification snapshot and lifecycle location intact. Terminal
    state: implementation complete, ready for review, with no pending rulings.

**Checkpoint:** Scoped tests/lints and explicit example compilation pass;
deterministic tests prove reclamation, reuse, replacement, recovery, and held-value
survival; generated text and artifact formats remain unchanged; ownership and
documentation audits are complete.
