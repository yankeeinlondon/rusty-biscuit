---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-18-dmls-cache-optimization-follow-ups/spec.md"
plan: "darkmatter/fixes/2026-10-18-dmls-cache-optimization-follow-ups/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/detailed_resolution.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/resolve/searched_tests.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/payload_memo_tests.rs
docs_updated_during_phase_1:
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-file/references/file-references.md
packages:
    - darkmatter
    - biscuit-file
human_review: false
message_to_agent: |-
    Phase 1 landed `ResolvedSchema::searched` -> `ResolvedPayload.searched` ->
    `TriggerRegistry::searched_locations()`. Facts Phase 2 relies on:

    - Entries are absolute and LEXICAL (biscuit-file's new
      `DetailedResolution::probed_paths()`, the same normalization as
      `matched_path()`), never canonicalized. They include every probed
      candidate up to and including the selected file, and a nearer candidate
      that exists as a directory (`NonFile`). Recursive `%` search roots are
      omitted (R1). Existing dependency files in `dependencies` keep their
      canonical spelling, so the same file can appear in both lists with
      different spellings: compare per list, never across them.
    - On macOS a temp dir under `/var` is reached via `/private/var`; fixtures
      that canonicalize their root (as `payload_memo_tests::Fixture` does) get
      matching spellings.
    - R6 needed no cache-carry: `ImportEngine::resolve_namespace` probes
      before it consults `namespace_cache`, so every import site records its
      own candidates (locked by
      `resolve::searched_tests::second_import_of_a_cached_namespace_still_records_its_own_search`).
    - Stale comments left for the Phase 4 sweep: `ResolvedSchema::imports` /
      `examples` docs still say "invalidation wiring is deferred", and the
      `file_payload_memo` doc in `assemble.rs` (~line 328) still claims root and
      selected-folder listings cover every searched directory (Phase 2 should
      replace that claim when it extends `inputs_fingerprint`).
    - `just cross-check` re-splits its args: pass nextest positional filters
      (`just cross-check darkmatter --os windows name_a name_b`), not `-E
      '...(...)'`, which fails with a remote shell syntax error.
---

# Implementation Log for 2026-10-18-dmls-cache-optimization-follow-ups (4 phases)

## Phase 1

### What changed

- **`ResolvedSchema::searched`** (`lib/src/markdown/schemas/resolve.rs`): a new
  `Vec<PathBuf>` beside `imports` / `examples` / `referenced_files`. It holds
  every location probed while selecting a schema file, named-type import, or
  example file, up to and including the selected one. It is sorted (via
  `BTreeSet`), deduplicated, lexical (absent paths have no canonical form),
  and filled after evaluation, so the candidate order is never changed. The
  existing dependency lists are untouched (R2).
- **One recording point per probe kind (R5):**
  `resolve_file_reference_in_context` now calls `resolve_detailed` on the same
  `for_cwd(base_dir)` context, records `probed_paths()`, then projects with
  `into_convenience()`. That is the exact body of `resolve_in_context`, so the
  error and `NoMatch` mapping is identical. `try_bare_name_in_roots` does the
  same per root on `for_trusted_external_cwd(root)`.
- **Threading:** a `searched: &mut BTreeSet<PathBuf>` sink goes through
  `expand_document_imports` / `expand_imports` (the `ImportEngine` gets a
  `searched` field, merged out at the end), `named_type_file_path`,
  `rehome_examples` / `rehome_atom_examples` (now `&mut self`),
  `resolve_document_examples` / `resolve_examples_in_json` /
  `resolve_one_example` / `resolve_example_path`. Delegation hops merge
  through `accumulate_searched` (the `accumulate_referenced_file` analogue);
  both root-union resolvers merge string arms and pass their own sink to
  inline arms.
- **Discarded sinks:** the public `named_type_reference_path` (editor
  go-to-definition) and `expand_named_types` (editor expansion) pass a
  throwaway set, because their signatures return no `ResolvedSchema`.
  Embedded expansion passes a throwaway set too, since it probes no disk.
- **Propagation (Wave 1.2):** `ResolvedPayload.searched` is copied from
  `resolved.searched` in `resolve_file_payload` (empty for embedded payloads),
  and `TriggerRegistry::searched_locations()` sits next to `dependencies()`
  in `discovery.rs`. The memo `Entry` clones the whole `ResolvedPayload`, so
  a hit reports the same set (tested).

### Departures from the plan

- **biscuit-file gained `DetailedResolution::probed_paths()`** (plan listed
  only `darkmatter`). The probed candidate paths are not normalized: they can
  be relative (resolved against `cwd`) or contain `.`/`..` (`./name`,
  `../x`). biscuit-file's lexical normalizer (`normalize_absolute`) was
  crate-private. Writing a third ad-hoc normalizer in darkmatter (there is
  already `effects::fs_write::lexically_clean`) would split the spelling rule.
  Instead `probed_paths()` yields each non-`SearchRoot` candidate through the
  same `normalize_absolute` used for `matched_path()`, so the selected
  candidate's entry equals the matched path exactly. `normalize_absolute`
  became `pub(crate)`. This is additive: no existing behavior changes.
  Recursive `%` search roots are omitted, which implements R1 at the source.
- **R6 needed no cache-carry code.** `ImportEngine::resolve_namespace`
  resolves the reference (and so probes) *before* it consults
  `namespace_cache`, and a cached namespace's nested imports and examples are
  re-expanded at every use site. Every site therefore records its own probes.
  The `searched` field doc on `ImportEngine` states this invariant, and
  `second_import_of_a_cached_namespace_still_records_its_own_search` locks it.

### Requirement → test mapping

Boundary: library resolver → `ResolvedSchema` (public field) →
`TriggerRegistry::searched_locations()` (public). Fidelity F2 (real temp
filesystem and real `FileReference` resolution), tier L1. No CLI or terminal
boundary is crossed in this phase.

| Requirement | Test |
| --- | --- |
| Concrete example records absent `schemas/shared/types.yaml` and selected `shared/types.yaml`; dependency lists unchanged | `resolve::searched_tests::implicit_import_records_its_absent_nearer_candidate_and_the_selection` |
| Explicit `./` / `../` records one location (no fallback) | `…::explicit_relative_references_record_only_their_one_location`; biscuit-file `probed_paths_lists_every_candidate_of_a_no_match` |
| Bare name records each root tried, stops at selection | `…::bare_name_records_each_schema_root_tried_through_the_selection` |
| Delegation hop merge | `…::whole_file_delegation_merges_each_hop` |
| Root-union arm merge | `…::root_union_arms_merge_their_searches` |
| Example file nearer candidate | `…::example_file_records_its_absent_nearer_candidate` |
| Transitive import + rehomed example search from their own folder | `…::transitive_import_and_its_rehomed_example_are_recorded` |
| Namespace-cache hit still records (R6) | `…::second_import_of_a_cached_namespace_still_records_its_own_search` |
| Nearer candidate that is a directory is recorded (seed for the Phase 2 swap) | `…::nearer_candidate_that_is_a_directory_is_recorded_before_the_fallback` |
| Inline mapping imports | `…::inline_mapping_records_the_searches_of_its_imports` |
| Failed resolution still errors (R4) | `…::failed_resolution_still_errors_and_returns_no_partial_record` |
| Registry exposes searched apart from dependencies; memo hit returns the same set | `triggers::payload_memo_tests::a_registry_reports_searched_locations_apart_from_its_dependencies` |
| biscuit-file lexical normalization (`../` collapsed in an absent folder), selection is last | `biscuit-file l1 detailed_resolution::probed_paths_lists_missing_candidates_through_the_match_lexically` |
| Recursive search roots omitted (R1) | `biscuit-file l1 detailed_resolution::probed_paths_omits_recursive_search_roots` |

Targeted runs: `cargo nextest run -p darkmatter --lib -E
'test(payload_memo_tests) | test(searched_tests)'` → 30 passed;
`cargo nextest run -p biscuit-file -E 'test(probed_paths) | binary(l1) &
test(detailed_)'` → 17 passed. No existing `dependencies` assertion in
`payload_memo_tests.rs` was changed.

Input Robustness Matrix: not applicable (no file-format or configuration
reader added).

Docs: biscuit-file's `docs/topics/file-references.md` and the `biscuit-file`
skill reference list `probed_paths()`. Darkmatter's `docs/lsp/indexing.md`
and the darkmatter skill are scheduled for Phase 4 and are not touched here.
Phase 1 changes no observable darkmatter behavior: nothing reads `searched`
yet.

### Gates

- `just lint` (darkmatter): pass. `just test` (darkmatter): 10545 passed, 0
  failed, 10 skipped (pre-existing skips), first run, no reruns.
- `just test` (biscuit-file): 1147 passed plus the minimal-feature run; `just
  lint` (biscuit-file): pass.
- `just check-tier-coverage darkmatter` / `biscuit-file`: 0 stranded.
- Dependents: no package outside `darkmatter/` names `ResolvedSchema`,
  `searched_locations`, or `probed_paths`, and no struct literal of
  `ResolvedSchema` / `ResolvedPayload` exists outside their own files, so no
  dependent test exercises a changed symbol. `cargo check -p claudine --tests`
  (largest consumer) compiles clean. The biscuit-file change is purely
  additive.
- CI scope (`affected_scope.py` over the working tree): `packages:
  [biscuit-file, darkmatter, test-toolkit]`, `change_class: package`, so the
  gates were required and were run.
- Cross-OS (path normalization risk): `just cross-check darkmatter --os
  windows searched_tests payload_memo_tests` → 30 passed;
  `just cross-check biscuit-file --os windows probed_paths detailed_` → pass.
  The first attempt with `-E '...(...)'` failed before running anything
  (just re-splits args); positional filters work.
- No load-sensitive or pre-existing failures observed.

Phase 1 status: implementation complete, ready for review.
