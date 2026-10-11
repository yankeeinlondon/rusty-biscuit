---
total_phases: 3
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/resolve.rs
docs_updated_during_phase_2:
  - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_code:
  - darkmatter/lib/src/markdown/schemas/resolve.rs
documentation:
  - darkmatter/docs/topics/schemas/definition.md
completed_phase: 3
implemented: true
packages:
  - darkmatter
---

# Shared Named-Type Tables Implementation Plan

## Phase 1: Establish Scope and Evidence

### Necessary Rules

Implement `2026-10-12-shared-type-tables` as a private Darkmatter ownership
refactor. Replace repeated whole-table copies with immutable
`Arc<IndexMap<String, PropertyDef>>` ownership, prove allocation sharing through
the actual resolver, and document the per-expansion lifetime. Claudine motivates
the work but requires no implementation change.

Successful completion means the first file load, cache hits, and `@this` share
the correct allocation; independently expanded use sites retain identical
behavior; the existing Darkmatter L1 suite and lint pass without changing
expectations; and comments and current documentation describe the ownership
accurately.

- **No unresolved design rulings.** The specification settles ownership,
  lifetime, coverage, and exclusions. Preserve its decisions without extending
  cache scope or combining this work with presence or local-name changes.
- **Preserve resolution behavior.** Keep `IndexMap` ordering, public APIs,
  serialized output, errors and error text, dependency edges, parsed schemas,
  origin information, referenced files, examples, and advisories unchanged.
  In particular, preserve today's stripping of imported top-level `required`
  and `generated` constraints.
- **Preserve engine boundaries.** Sharing lasts within each existing import
  expansion. Root-union arms and separately expanded files retain their
  current engine boundaries. No global cache, locks, interior mutability,
  `Arc::make_mut`, or shared expanded definitions.
- **No spike required.** The specification explicitly chooses allocation
  identity as the performance regression check. A timing sample or benchmark
  would not answer an unresolved implementation question. Schedule neither a
  performance spike nor a later performance re-check.
- **Input robustness matrix: not applicable.** This work changes ownership of
  already parsed tables, not file-format parsing, classification, configuration
  loading, or deserialization. Leave `load_named_types` and the standalone
  classifier's accepted inputs and rejection behavior unchanged. If a parser
  change appears necessary, stop that expansion of scope and record it for the
  author rather than silently adding it to this plan.
- **Portable, bounded verification.** Tests are hermetic L1 and must work on
  macOS, Linux, native Windows, and WSL2. Use existing nextest-backed recipes
  and CI coverage; add no CI cells or terminal/browser tests. Do not run
  `cargo fmt`, commit, or move the feature into `_completed`.

### Wave 1: Scope and Baseline

This wave runs sequentially before implementation. All paths below are relative
to the repository root.

- [x] **Map ownership**
  - Inspect `darkmatter/lib/src/markdown/schemas/resolve.rs`: `Namespace`,
    `CachedNamespace`, `expand_document_imports`, `resolve_namespace`,
    `load_named_types`, and the selected-definition clone in import expansion.
  - Identify the three table-copy sites: the `this` branch, the cache-hit
    branch, and first-load cache insertion. Confirm the root properties copy
    is a separate, intentional one-time copy after the no-import early return.
  - Record the invariants to review after editing: dependency insertion before
    cache lookup, canonical cache keys, imported-file base directories,
    request-scoped resolution context, schema roots, cycle stack, and depth cap.

- [x] **Map coverage**
  - Inventory tests in the resolver's existing private test modules, including
    `schema_plus_phase1` and `multiple_imports_from_same_file_resolve_and_dedup`,
    plus standalone-envelope and shipped-schema corpus coverage.
  - Map existing tests to cross-file reuse, root `@this`, imported-file `@this`,
    both mapping envelopes, use-site independence, ordering, dependency
    deduplication, cycles, depth limits, and rejected named-import targets.
  - List only missing acceptance coverage for Phase 2. Existing single-import
    tests do not alone prove two use sites remain independent.
  - Follow the existing hermetic temporary-file setup and test-toolkit
    conventions; construct request context from fixture inputs and avoid
    process-wide environment changes or new dependencies.

- [x] **Capture baseline**
  - Run `just test` from `darkmatter/` before source changes and save its result
    in the implementation log. Record pre-existing failures separately.
  - Record the existing resolver/corpus test identities so final validation
    can show they passed without expectation edits.
  - Checkpoint: all implementation sites and coverage gaps are identified;
    no unresolved ruling or exploratory spike blocks Phase 2.

## Phase 2: Share Tables and Prove Independence

### Wave 2: Implementation and Documentation

Depends on Wave 1. The two tasks may run concurrently with separate owners:
the resolver owner edits `resolve.rs`, while the documentation owner edits only
`definition.md`. Tests in Wave 3 remain with the resolver owner because they
share the implementation file.

- [x] **Share namespaces**
  - In `darkmatter/lib/src/markdown/schemas/resolve.rs`, use
    `std::sync::Arc` and change both private named-table fields to
    `Arc<IndexMap<String, PropertyDef>>`.
  - Wrap the table returned by `load_named_types` once on first load. Move one
    pointer into the returned namespace and clone the pointer for the cache so
    insertion and the first return share the same allocation.
  - Make the cache-hit and `this` branches clone only the `Arc`; use explicit
    `Arc::clone` where it clarifies ownership.
  - Keep the root properties copy in `expand_document_imports`, then wrap that
    copy in an `Arc`. Preserve the no-import early return before allocation.
  - Retain the selected `PropertyDef` clone before recursive expansion and
    use-site modifiers. Do not alter loader signatures, classification,
    expansion order, dependency recording, or engine lifetime.
  - Update affected field/module comments to describe immutable shared tables
    and their lifetime. Correct comments that describe named types as only
    `$schema` entries to include `kind: schema` / `types`; preserve literal
    error text even where it contains the older wording.
  - Checkpoint: the diff contains exactly the intended ownership changes and
    relevant comment corrections; no public API or dependency changes.

- [x] **Document sharing**
  - Update `darkmatter/docs/topics/schemas/definition.md` in the named-import
    discussion with a short implementation note: unexpanded tables are shared
    within an expansion, while each use receives its own expanded definition.
  - Correct the nearby `$schema`-only description of named tables to name both
    already supported mapping envelopes. Align the note with the later
    standalone-envelope explanation.
  - Explain that sharing does not persist across calls or guarantee a fixed
    end-to-end speedup. Keep the note free of feature/fix references.
  - Checkpoint: the documentation describes the exact implemented behavior
    and adds no planned behavior. No README or dependency-document update is
    needed because public behavior and crate dependencies are unchanged.

### Wave 3: Allocation and Behavior Tests

Depends on the resolver changes in Wave 2. These tasks run sequentially under
one resolver owner to avoid concurrent edits to `resolve.rs`.

- [x] **Prove allocation identity**
  - Add focused private tests beside the resolver implementation that call
    `ImportEngine::resolve_namespace` on hermetic temporary schema files.
  - Compare the first returned file namespace with its cache entry using
    `Arc::ptr_eq`. Repeat resolution for imports selecting two distinct names
    from that file and compare their tables with the original allocation.
    Do not limit assertions to two cache hits or to reference counts.
  - Resolve `this` twice from an inline-root namespace and compare both tables
    with that root's original table. Repeat from an actually loaded file
    namespace and compare with its original loaded table.
  - Assert the loaded file's table is distinct from the root table. Exercise
    the actual resolution method rather than merely cloning fixture pointers.
  - Use existing canonical-path helpers and fixture context for cache access;
    introduce no public inspection API, counters, or observer callbacks.

- [x] **Prove independent imports**
  - Add or reuse small public-resolution regression tests importing one type
    twice, applying a constraint or description only to one use, and asserting
    the expected JSON Schema for both properties.
  - Cover cross-file reuse, inline-root `@this`, and `@this` authored inside an
    imported file. For the last case, give root and imported namespaces
    distinguishable definitions so the output proves which table was selected.
  - Exercise both pure `$schema` and `kind: schema` / `types` named-type
    envelopes using compact parameterized fixtures where practical. Record
    which existing tests satisfy cases instead of duplicating adequate tests.
  - Retain existing dependency assertions, ordering assertions, error tests,
    and corpus expectations. Add assertions only for uncovered acceptance
    claims; do not rewrite expected results to accommodate the refactor.
  - Checkpoint: allocation tests prove first-load and subsequent sharing;
    output assertions prove use-site independence and correct namespace choice
    for both envelopes.

## Phase 3: Validate and Prepare Review

### Wave 4: Validation and Handoff

Depends on all Phase 2 tasks. Run validation sequentially to keep failures
attributable and avoid unnecessary competing builds.

- [x] **Run acceptance checks**
  - Run `just test` from `darkmatter/`; require the new private tests and the
    existing resolver, cycle, envelope, and shipped-schema corpus tests to pass.
    Keep all pre-existing expectations unchanged.
  - Run `just lint` from `darkmatter/` and resolve regressions within scope.
    Do not run `cargo fmt` as a lint workaround.
  - Record commands and outcomes in the implementation log, distinguishing
    local macOS evidence from evidence produced by existing CI environments.
    Preserve existing cross-platform coverage without adding special runs or
    claiming an OS result that was not obtained.

- [x] **Audit final diff**
  - Verify all three full-table copy sites now clone shared pointers, first
    load creates exactly one table allocation, and root setup retains exactly
    its intentional properties copy.
  - Confirm the selected definition still gets cloned before use-site changes;
    no shared table is mutated and no expanded result is shared.
  - Check every Phase 1 resolution invariant, unchanged no-import behavior,
    unchanged serialization ordering, and unchanged public resolver results.
    Use existing tests as evidence and inspect the diff for any altered path
    that those tests do not directly cover.
  - Review behavior-adjacent comments and the documentation note together.
    Confirm no parser, public API, dependency, CI, or unrelated changes entered
    the patch.

- [x] **Prepare review handoff**
  - Record acceptance criteria 1–6 with the test names or diff evidence that
    satisfies each, along with any baseline failures or remaining blockers.
  - Record any departure from the specification in the implementation log;
    leave the specification unchanged.
  - Mark completed plan tasks and report **implementation complete, ready for
    review** only when all checkpoints pass. Leave lifecycle closure and any
    commit to the author.
