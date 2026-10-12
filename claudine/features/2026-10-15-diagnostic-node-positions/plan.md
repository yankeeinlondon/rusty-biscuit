---
total_phases: 4
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/positions.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/mapping_orders.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/tests/positions.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.json
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.pretty
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/diagnostic_positions.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/enum_member_invalid/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/missing_required/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_invalid/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/property_union_nested_object/expected.pretty
    - darkmatter/cli/tests/fixtures/schema_validate_baseline/root_union_none_match/expected.pretty
    - darkmatter/lib/tests/position_fixtures/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/diagnostic_positions.rs
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
    - darkmatter/lib/src/markdown/schemas/tests/positions.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/position_parity_tests.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/errors/file-reference-failures.md
    - darkmatter/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/schema.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
---

# Diagnostic Node Positions Implementation Plan

## Phase 1 — Scope, Design, and Feasibility

### Necessary Rules

- **Implementation ownership:** This is a Darkmatter library, CLI, and DMLS-test change. Its planning document belongs to Claudine's rollout feature `2026-10-15-diagnostic-node-positions`. Leave the finalized specification unchanged; record any implementation departure in the implementation log.
- **Source authority:** Prefer `Markdown::loaded_source()` and `extract_frontmatter_block()` over reconstructed text. `Frontmatter::raw_source()` currently rejoins lines with LF, so it cannot supply original CRLF byte offsets. Use that retained YAML only when original loaded text is unavailable; preserve the existing serialized-view behavior for programmatically constructed Markdown. Do not reread a physical file to obtain positions.
- **Snapshot lifetime:** Build one located tree and one line-start index for each validation source snapshot, outside problem construction and union-arm iteration. Reuse them across every problem from that snapshot. An effective value changed by an assignment still refers to the original authored tree. Do not compare replacement scalar values with authored scalar values to decide whether a position exists.
- **Location selection:** Value failures select value starts; unknown keys use the existing `offending_property` to select key starts; missing fields select their parent mapping. Root failures use the located root start. Missing nested paths use the nearest located non-root ancestor value, then the original top-level key, then no position. Do not fall back to the root for a newly supplied, unauthored top-level property.
- **Semantic alignment:** Traverse decoded `JsonPointer::segments()` through the authored node kind. Numeric segments index sequences only; mapping keys remain decoded strings. Never expose a nested position where source projection cannot be matched to the authored semantic structure. Unsupported YAML may disable the entire precise projection; retain the existing top-level scan and semantic validation result in that case. No new alias, merge-key, or YAML presentation support is required.
- **Coordinate contract:** Library/CLI positions are 1-based UTF-8 byte columns. Apply the Markdown delimiter offset once to all location classes; `build_position_map()` continues to address its supplied YAML slice. A block scalar or quoted scalar decoded into an object remains one authored scalar node. DMLS range selection remains unchanged, and parity compares byte offsets after converting its negotiated UTF-8 or UTF-16 positions.
- **Diagnostic stability:** Preserve every field except `line` and `column`, problem count/order, union-arm selection, selected unknown property, validation decisions, JSON null handling, and CLI exit codes. Retain `PositionMap::new()` for source-free validation. A representation change is allowed, with all actual construction/lookup sites migrated together.
- **Robustness matrix applicability:** This change projects locations over existing frontmatter semantics; it does not add or change a format/configuration reader, deserializer, or acceptance policy. The configuration Input Robustness Matrix is therefore not applicable. Test presentation and semantic-result preservation below. If implementation requires changing a semantic loader, stop that expansion and document the need for an author ruling; do not silently broaden this location-only change.
- **Spike scope:** Run the single locator feasibility spike below once. No performance spike or benchmark gate: the specification already supplies the required cost model, one source projection per snapshot. No multi-host measurement, new CI cells, or new test tiers.

### Work and Success

Replace the public top-level-only position lookup with an authored-node position index, connect category-aware lookup to validation, and expose the resulting coordinates through `md schema validate` and composition error blocks. Update the affected documentation and prove parity with DMLS without changing editor ranges or the validation contract.

Successful completion means the nested `sya` fixture reports line 5, column 9 through the library, CLI JSON, human validation output, and compose error output, while retaining pointer `/start/0/then/1` and `offending_property: sya`. Supported keys, values, mappings, and sequence items agree with DMLS at the same original byte offset. Assignments, decoded scalars, unsupported YAML, CRLF, and source-free validation follow the specified fallbacks. Darkmatter's L1 and lint gates pass, and any consumers affected by the public map migration compile.

### Wave 1 — Design and One-Time Spike

Tasks in this wave are sequential: finish the contract inventory before the spike. This wave establishes the boundary that subsequent parallel work will share.

- [x] **Contract Inventory**
  - Inspect `darkmatter/lib/src/markdown/schemas/validate.rs`, `schemas/mod.rs::positions_for`, and `schemas/simplified/source.rs`. Inventory every repository `PositionMap` construction, insertion, lookup, and iteration with a Rust-scoped search, including tests and DMLS's empty-map calls.
  - Trace `positions_for()` through validation/coercion and composition; confirm the original loaded snapshot survives assignments and both composition validation passes. Inspect `Markdown::loaded_source()` and the byte-preserving frontmatter extractor before selecting the source.
  - Record a minimal representation that holds distinct key/value/root starts, legacy top-level fallback positions, and the shared line index. Prefer wrapping the existing located tree over introducing a second parser or new dependencies. Specify empty construction and how one source-relative line offset reaches every position.
  - Identify actual public-map consumers separately from locator consumers: `markdown/hash/write.rs` and `markdown/compose/frontmatter_interpolation.rs` share the locator and must retain their behavior.
  - Review the relevant Darkmatter/schema/testing skills and repository Test Toolkit. Before downstream package discovery, use the sniff skill and Sniff; use GitNexus only when its caller graph resolves a remaining question faster than source reads.

- [x] **Locator Spike**
  - **Prerequisite:** Contract Inventory. Run a small, local probe using the existing locator on compact mappings in sequences, flow collections, escaped/empty/numeric keys, block scalars, leading comments, LF/CRLF, and valid unsupported YAML. Inspect key/value/root spans and confirm which token marks a mapping or scalar start.
  - Probe the boundary between structural projection and actual authored semantics, especially unsupported/ambiguous forms and decoded scalar descendants. Determine a conservative way to reject unsafe nested projection without evaluating expressions, resolving files, or parsing YAML for every diagnostic.
  - Record expected offsets and the chosen source/index representation. Keep useful cases as eventual L1 fixtures; do not add a permanent benchmark or repeat this spike later.
  - If the required supported presentations cannot be projected with the existing locator, record the smallest contract-preserving adjustment and any specification departure. Unsupported presentation coverage must remain best effort rather than becoming a new parser project.

**Checkpoint:** The source snapshot, map API, structural matching boundary, fallback order, and fixture coordinates are concrete enough for implementation. No unresolved decision changes validation semantics or DMLS range policy.

## Phase 2 — Authored Position Index and Diagnostic Selection

### Wave 2 — Source Infrastructure

- [x] **Position Index**
  - **Prerequisite:** Wave 1. Implement the chosen `PositionMap` representation and update `build_position_map()` to invoke `locate_frontmatter_value()` once, retain decoded keys and distinct key/value spans, and build one byte-based line-start index. Retain the existing top-level scan as fallback when precise projection is unavailable.
  - Walk pointer segments by source node kind; distinguish root from an empty mapping key, and mappings from sequence indices. Stop descending at scalar nodes and select their start for decoded-content descendants. Exclude a block-list dash from the item value/mapping start.
  - Update `positions_for()` to project the original loaded YAML bytes and offset every key, value, root, and fallback location exactly once. Preserve raw-YAML and serialized-view fallbacks, including empty source-free maps.
  - Migrate actual map construction/lookup/iteration sites and their docs in the same task so the repository compiles between waves. Do not change shared locator behavior unless the Phase 1 finding requires a minimal adjustment.
  - Add focused tests of source-relative versus document-relative coordinates, key/value distinction, root location, and empty-map construction. Prove original CRLF offsets using byte slices rather than line-only assertions.

**Checkpoint:** All migrated map callers compile. Supported projection retains original byte spans, and unsupported projection yields the legacy fallback without introducing a validation error.

### Wave 3 — Semantic Integration

- [x] **Problem Selection**
  - **Prerequisite:** Wave 2. Replace `build_problem()`'s top-level-key lookup with category-aware selection using existing typed error identity, `instance_path`, and `offending_property`. Do not parse messages or alter pointers to obtain a location.
  - Implement exact-node selection and the specified ancestor/top-level/no-position fallback chain. Cover missing root fields, nested missing fields, replaced authored paths, new nested paths, and new top-level paths.
  - Keep projection outside per-problem and union-arm loops. Verify reuse structurally, or with existing test instrumentation if available; do not introduce production counters solely for testing.
  - Assert public `ValidationReport` results for type, constraint, file-reference, unknown-key, missing-field, and union failures. Compare position-enabled and source-free reports after removing only coordinates to prove all other diagnostic fields, problem order, and validity remain equal.
  - Include an error containing several unknown keys and confirm lookup follows the validator's existing selected property without splitting or reordering that problem. Review and correct the changed symbols' top-level-only comments.

**Checkpoint:** Library validation produces the acceptance fixture's exact coordinates and preserves diagnostic identity and outcomes. This stable report contract unlocks independent CLI and DMLS work.

## Phase 3 — Presentation, Parity, and Regression Coverage

### Wave 4 — Parallel Consumer Tasks

These tasks may run concurrently in separate CLI/error-rendering and DMLS files after Wave 3. Share the same complete Markdown fixture bytes and expected offsets; neither task may change the position-index contract independently.

- [x] **CLI Presentation**
  - Update `darkmatter/cli/src/commands/schema/validate.rs::format_location()` to show line and column when both are present; retain sensible output for a missing column or completely missing location. Correct the obsolete column-1 comment and the wording that labels document-absolute lines as frontmatter-relative.
  - Inspect `darkmatter/lib/src/markdown/errors/blocks.rs::schema_validation_block`: it already renders `line:column`. Preserve that behavior and verify excerpts/carets, where present, address the offending original node correctly. Make only changes justified by a failing nested-coordinate test; render through existing biscuit-terminal components.
  - Extend `darkmatter/cli/tests/l1/schema_validate.rs` and suitable compose L1 tests using Darkmatter's `CliProcessFixture`. Supply a fixture schema for `start`/`then`/`say`; do not depend on future lifecycle grammar.
  - Assert line 5, column 9 in human output, JSON fields, and `md compose`'s validation error. Include flow nodes whose errors differ only by column, JSON location nulls, and existing success/failure exit codes.

- [x] **DMLS Parity**
  - Extend tests around `darkmatter/dmls/src/diagnostics/frontmatter.rs` using its existing `DocumentContext`, overlay, and `SourceMap` fixtures. Keep production DMLS range rules unchanged.
  - Validate the same complete Markdown bytes through the library and CLI evidence, then compare byte offsets with DMLS diagnostic starts for values, unknown keys, missing parent mappings, and root failures on supported nodes.
  - Exercise both supported position encodings, `PositionEncoding::Utf8` and `Utf16`, with ASCII and a same-line non-ASCII prefix containing a supplementary-plane character. Convert using the actual source line/SourceMap; do not compare raw CLI and LSP columns.
  - Assert existing DMLS range ends and ranged text as well as starts, so parity cannot pass by narrowing or otherwise changing editor ranges.

**Checkpoint:** Human validation, JSON, compose, and DMLS agree on supported source starts; DMLS still highlights its original ranges.

### Wave 5 — Parallel Coverage and Documentation

Both tasks depend on Wave 4. They may run concurrently with distinct ownership: library regression tests versus README/topic/skill documentation. Coordinate any shared fixture changes before editing.

- [x] **Regression Coverage**
  - Extend public validation/composition tests with the cases in the table below. Use data-driven fixtures where appropriate and assert public diagnostics, not merely locator return values.
  - Check block-scalar and quoted JSON/YAML-content coercion cases report the authored scalar start. Confirm valid unsupported YAML retains semantic validity and top-level fallback reporting when an independent schema violation exists.
  - Retain existing hash-writer and frontmatter-interpolation regression coverage; add a focused regression only if their shared locator behavior had to change. Include the existing shipped-schema corpus checks to detect accidental presentation regressions.
  - Keep all added tests hermetic L1, portable across macOS, Linux, native Windows, and WSL2, with no terminal/browser windows or focus changes. Avoid compile-time host paths and ambient CWD/home/provider dependencies.

- [x] **Behavior Documentation**
  - Update `darkmatter/docs/topics/schemas/definition.md`, `darkmatter/README.md`, and `.claude/skills/darkmatter/schema.md` with a compact nested example showing exact key/value coordinates, 1-based UTF-8 byte columns, and complete-document line numbering.
  - Explain original authored locations after assignments, ancestor and top-level fallbacks, unlocated generated properties, scalar-content boundaries, unsupported YAML, and serialized-view locations for constructed documents.
  - Review docs/comments on all behavior-changing symbols and remove or correct top-level-only descriptions. Keep topic docs self-contained and free of feature/spec references. No dependency documentation change is needed unless the implementation actually changes dependencies.

| Regression case | Observable assertion |
| --- | --- |
| Nested `sya` acceptance fixture | Pointer `/start/0/then/1`, selected key `sya`, line 5, column 9 |
| Wrong list item and second mapping missing a field | Value/mapping token start, excluding the dash |
| Nested unknown key versus wrong value | Distinct key/value token coordinates |
| Flow mapping and sequence | Correct same-line byte columns |
| Root missing field and source-free validation | Root mapping start when located; no coordinates for an empty map |
| Existing assignment, new nested path, new top-level path | Original node, nearest non-root ancestor, no position, respectively |
| Literal/folded scalar and decoded JSON/YAML descendant | Authored scalar start; no invented internal span |
| Quoted/escaped keys, `"0"`, `"a/b"`, `"a~b"`, and `""` | Decoded key lookup; correct pointer escaping; mapping/sequence distinction |
| LF/CRLF, leading comments and blank lines | Original-byte offsets and exactly one delimiter-line offset |
| Valid unsupported YAML | Validation remains passive and successful when schema-valid; independent failures keep legacy fallback |
| Union arms and multi-key unknown errors | Original count/order, arm selection, selected property, and all non-position fields |
| Shared locator consumers | Existing hash-writing and interpolation behavior remains intact |

**Checkpoint:** Every acceptance criterion has named, executable L1 coverage, and affected user-facing docs describe the implemented behavior.

## Phase 4 — Verification and Review Handoff

### Wave 6 — Final Gates

This wave is sequential. Run tests and lint separately, as required by Darkmatter's testing guidance; do not run their builds concurrently.

- [ ] **Package Verification**
  - **Prerequisite:** Wave 5. Run `just test` in `darkmatter/`; its current recipe includes the library, CLI, DMLS, and Zed CLI. Then run `just lint` there. Use nextest through the canonical recipes; never run consolidated suites with `cargo test` or run `cargo fmt` without explicit authorization.
  - The current lint recipe also runs `check-zed`, which requires a provisioned `wasm32-wasip2` target. Check that prerequisite before the final gate, report an environment blocker accurately if absent, and do not install toolchain components implicitly or claim the gate passed.
  - If the public map changed, compile the actual affected downstream consumers identified in Phase 1 using package-scoped checks. Do not substitute a workspace-wide build. Fix regressions and rerun the checks affected by the fix.
  - Record command results and the environment that produced them. Tests must remain portable on all four supported environments; do not claim unrun OS results. Consult the `os` skill before any platform-specific code change or claim that an OS cannot be tested.

- [ ] **Review Handoff**
  - **Prerequisite:** Package Verification. Check the final diff against all seven specification acceptance criteria, the regression table, snapshot reuse, passivity, and the unchanged diagnostic/exit-code contract. Confirm no new CI cells or tier requirements were introduced.
  - Record completed tasks, evidence, environmental limitations, and any specification departures in the implementation log. Mark implementation complete and ready for review only when required implementation checks pass; otherwise identify the remaining blocker explicitly.
  - Leave this feature in its active directory for the author's review cycle. Do not move it to `_completed`, run `just complete`, commit, or modify the finalized specification as part of implementation.

**Checkpoint:** Implementation is complete, documented, validated within the actual blast radius, and ready for review with explicit evidence and any remaining environment limitations.
