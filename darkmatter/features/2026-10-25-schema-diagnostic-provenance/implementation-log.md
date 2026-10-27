---
spec: /Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/darkmatter/features/2026-10-25-schema-diagnostic-provenance/spec.md
plan: darkmatter/features/2026-10-25-schema-diagnostic-provenance/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/schema.md
packages:
    - darkmatter
    - dmls
human_review: false
message_to_agent: |-
    Phase 1 already builds every ConstraintConflict participant with its real
    value in resolve.rs (bounds = surviving strict bound; scheme = inherited
    list snapshotted BEFORE retain; defaults; canonical option sets with
    Options(vec![]) for bare alphanumeric). Phase 2 Wave 2 should not
    re-snapshot; it adds Constraint::to_schema_text, the shared excerpt
    helper, Display/StatusBlock values, and the full test matrix.
    Both failure types box their error (`error: Box<SchemaError>`) to satisfy
    clippy::result_large_err; match with `*failure.error`.
    The crate-private carrier is simplified::DefinitionFailure with
    parse_{schema_declaration,yaml_schema,property_definition}_located.
    parse_schema_shape takes the shape path and attributes to key,
    `$constraints`, or root-arm index; nested recursion (parse_mapping_atom,
    sequence arms, second_suggestion_span, duplicate_overload) still returns
    SchemaError and is attributed to its containing property. Wave 3 threads
    the path into parse_property_definition_at_depth / parse_mapping_atom.
    DefinitionFailure.path carries #[cfg_attr(not(test), expect(dead_code))]:
    remove it once source.rs reads the path (it will then fail to compile).
    Path prefixing for standalone documents uses SchemaSourcePath::join.
---

# Implementation Log for 2026-10-25-schema-diagnostic-provenance (4 phases)

## Phase 1

Wave 1, **Define provenance contracts**, implemented in one pass (no subagents:
the change is one tightly coupled type migration).

### What changed

- `simplified/source.rs`
    - new public `SchemaConstraintParticipant { origin, constraint }`
    - new public `LocatedDefinitionFailure { path, error, authored_span,
      value_span }` with documented path roots (definition / declaration /
      envelope with `types` or `$schema`) and UTF-8 byte coordinates with the
      caller's `source_offset` applied once; `Display`/`Error` delegate to the
      inner `SchemaError`, and `From<LocatedDefinitionFailure> for SchemaError`
      lets callers that do not need locations keep using `?`
    - new `SchemaSourcePath::join` (the path-prefix convention: a failure path
      relative to a definition is prefixed by its envelope path)
    - corrected the stale module header: the strict locator already follows
      literal and folded block scalars (Decision 5)
- `simplified/mod.rs`
    - internal carrier `DefinitionFailure { path, error }` with
      `From<DefinitionFailure> for SchemaError`
    - `parse_schema_declaration_located`, `parse_yaml_schema_located`, and
      `parse_property_definition_located` (crate-private); the source-free
      `parse_schema_declaration`, `parse_yaml_schema`, and
      `parse_property_definition` are now thin wrappers that discard the path,
      so their signatures and errors are unchanged
    - `parse_schema_shape` takes the shape's path and attributes failures to
      the property key, `$constraints`, or root-union arm index that contains
      them; deeper recursion (nested objects, sequence arms, cross-arm
      suggestion/overload checks) is not threaded yet and is attributed to the
      nearest containing value, which Phase 2 Wave 3 refines
    - unit tests for literal keys with dots/brackets/slashes/Unicode, root-union
      arm indices, `$constraints`, structural root failures, the property-call
      root, and `join`; each also asserts the located error's text equals the
      source-free error
- `schemas/mod.rs`, `simplified/mod.rs`: re-export both new public types
- `schemas/errors.rs`: `ConstraintConflict.constraints` is
  `Vec<SchemaConstraintParticipant>`; `Display` and `StatusBlock` read
  `participant.origin` and still render keywords (Phase 2 switches them to the
  shared excerpt); unit test migrated
- `schemas/resolve.rs`: `MergeSite::conflict` takes `Vec<Tagged>` and builds
  participants; every constructor passes the value that took part:
    - bounds: the surviving strict bound of each side, in stream order
    - scheme: the inherited list snapshotted **before** `retain`, then the
      use-site list
    - default: both declared values (merge path and item/container
      reconciliation)
    - options: canonical base set (`Options(vec![])` for bare
      `alphanumeric`, at the existing synthetic origin) and the canonical
      use-site set
  `locate_origin_spans` now walks participants' origins
- DMLS `diagnostics/named_types.rs` (related information) and
  `overlay/mod.rs` (watched files) read `participant.origin`; wording unchanged
- tests: `recursive_schema_types.rs`, `schemas_core_type_grammar.rs`,
  `schemas_named_type_merge.rs`, `schemas_union_postfix.rs` migrated to
  `.origin`; added value assertions for the multi-hop bound (`Min(10.0)` /
  `Max(5.0)`), scheme (immediate and post-narrowing: inherited participant is
  `scheme(https)` at origin `link`), and bare-options (`Options([])`, no span,
  vs `Options([AllowDashes])`)

### Call-site inventory and migration ownership

| Surface | Sites | Owner |
| ------- | ----- | ----- |
| `ConstraintConflict` constructors | `resolve.rs` `MergeSite::conflict` (bounds, scheme, default, options, item/container default); `resolve/recursive.rs` reuses `merge_constraints` | migrated in Phase 1; Phase 2 Wave 2 adds the full test matrix |
| `ConstraintConflict` renderers | `errors.rs` `Display`, `StatusBlock` | Phase 1 (field access); Phase 2 Wave 2 (values + excerpt) |
| `ConstraintConflict` matches | `resolve.rs:861`, `triggers/finding.rs:204` (site only), `shipped_schema_corpus.rs:473` (`..`) | unaffected |
| DMLS conflict consumers | `diagnostics/named_types.rs` related info; `overlay/mod.rs` watched files | Phase 1 (field access); Phase 3 Wave 5 (values, effective-scheme wording, bare-options span) |
| `parse_property_definition_with_source` | `source.rs` tests; lib tests `schemas_source_projection.rs`, `schema_yaml_spellings.rs`, `schemas_core_type_grammar.rs` | Phase 2 Wave 4 |
| `parse_schema_declaration_with_source` | DMLS `overlay/named_types.rs:72`; `source.rs` tests; lib tests above | Phase 2 Wave 4 (signature), Phase 3 (DMLS consumer) |
| standalone guessing | DMLS `diagnostics/frontmatter.rs` `grammar_token_span`, `collect_string_scalars`, `standalone_type_definition_diagnostic` | Phase 3 Wave 5 |
| meta-type validators | `grammar.rs:1816-1819`, DMLS `diagnostics/frontmatter.rs:469-471` (source-free) | unchanged signatures; Phase 3 adapts DMLS consumers |
| Claudine | no direct `ConstraintConflict` or source-aware parser use (searched) | test-only verification |

### Design Decisions

- **Participants carry real values now, not placeholders**
    - **decision:** construct each participant with the value that took part
      (including the pre-`retain` scheme snapshot) in Phase 1, because every
      constructor already holds the `Tagged` pair.
    - **alternatives considered:** fill `constraint` with the authored origin
      value and fix in Phase 2; keep origins and add values in Phase 2.
    - **why:** the plan forbids placeholder values; the values are in hand, so
      a mechanical-but-correct migration costs nothing extra.
    - **downsides:** Phase 2 Wave 2's resolver work is reduced to tests,
      serializer, and rendering; it must not re-snapshot schemes.
- **Located-failure carrier is crate-private and attributes to the nearest
  containing value until threaded**
    - **decision:** `DefinitionFailure` lives in `simplified/mod.rs`; located
      entry points exist for declaration, YAML schema, and property definition;
      top-level shapes attribute to key / `$constraints` / root-arm paths;
      deeper recursion is attributed to its containing property. The `path`
      field uses `#[cfg_attr(not(test), expect(dead_code, …))]`, which stops
      compiling once Phase 2 reads it, so the attribute removes itself.
    - **alternatives considered:** thread the full recursion now (Phase 2
      Wave 3's ownership); define the type without any producer (dead code).
    - **why:** keeps Phase 1 a contract phase while making the carrier real and
      tested; "nearest containing value" is the spec's own rule for failures
      without a deeper attribution, so no recorded path is wrong.
    - **downsides:** nested-object, sequence-arm, and cross-arm paths are still
      coarse until Wave 3.
- **`LocatedDefinitionFailure` converts into `SchemaError`**
    - **decision:** implement `From<LocatedDefinitionFailure> for SchemaError`
      plus `Display`/`Error` delegating to `error`.
    - **alternatives considered:** require every caller to write `.error`.
    - **why:** the spec notes callers that do not need locations "access the
      inner error"; `?` conversion keeps that migration mechanical.
    - **downsides:** a caller can silently drop locations with `?`; DMLS
      consumers must deliberately keep the struct.
- **`error` is boxed in both failure types**
    - **decision:** `LocatedDefinitionFailure.error` and
      `DefinitionFailure.error` are `Box<SchemaError>` (the spec's sketch shows
      a bare `SchemaError`).
    - **alternatives considered:** `#[allow(clippy::result_large_err)]` on
      every source-aware function; returning `Box<LocatedDefinitionFailure>`.
    - **why:** `just lint` rejected `Result<_, DefinitionFailure>` (144 bytes)
      under `result_large_err`; the library's established answer is to box the
      large field (`compose/frontmatter_interpolation.rs`,
      `shell_expansion/types.rs`). The spec's return type
      `Result<SourceAware<T>, LocatedDefinitionFailure>` is kept exactly.
    - **downsides:** callers write `*failure.error` / `&*failure.error` to
      match the variant.
- **Path-prefix convention is `SchemaSourcePath::join`**
    - **decision:** paths are relative to the parse call's root; a caller
      nesting that value prefixes with `envelope.join(&path)`.
    - **alternatives considered:** pass a prefix into every parse call.
    - **why:** the source-free parsers stay prefix-free, and the standalone
      helper (`types` / `$schema`) prefixes once.
    - **downsides:** one more public method on `SchemaSourcePath`.

### Verification

- `cargo check -p darkmatter -p dmls -p darkmatter-cli --tests`: clean, no
  warnings
- focused: `cargo nextest run -p darkmatter -p dmls -E 'test(definition_failure)
  | test(failure_paths_join) | test(property_definition_failure) |
  test(conflict) | test(schemes_intersect) | test(bound) | test(options) |
  test(default) | test(watch) | test(related)'` → 594 passed
- `just lint` (darkmatter): first run failed on `clippy::result_large_err`
  for `DefinitionFailure` (144 bytes); fixed by boxing `error` (see decision
  above); rerun passes, including the zed-dmls wasm check
- `just test` (darkmatter): **12615 passed**, 10 skipped, 0 failed (148 s)
- downstream Claudine: `cargo check -p claudine -p claudine-cli -p
  claudine-gen --tests` clean; `cargo nextest run -p claudine -p claudine-cli
  -E 'test(schema) | test(conflict) | binary(l1) &
  test(/schema|catalog|recursion/)'` → **1363 passed**. Claudine has no
  direct use of `ConstraintConflict` fields or the source-aware parsers, so
  the full suite was not run
- cross-OS check skipped: the phase touches no paths, processes, or
  OS-specific code (type and error-shape changes only)
- the scope-calculator gate skip does not apply: the phase changed compiled
  source in `darkmatter` and `dmls`

### Requirement-to-test mapping (Phase 1)

| Requirement | Test |
| ----------- | ---- |
| participant carries surviving bound values, inherited first | `schemas_named_type_merge::a_multi_hop_bound_conflict_names_both_origins_with_the_defining_span` |
| scheme participant is the pre-intersection effective list (immediate and post-narrowing) | `schemas_named_type_merge::schemes_intersect_and_an_empty_intersection_is_a_conflict` |
| bare `alphanumeric` participates as `Options([])` at the synthetic origin with no span | `schemas_core_type_grammar::a_conflict_against_a_base_type_without_options_names_its_declaration` |
| conflict rendering still works with participants | `errors::tests::constraint_conflict_lists_every_origin_in_merge_order` |
| located parse returns the source-free error unchanged plus a structural path (literal keys, root arms, `$constraints`, structural roots, property root) | `simplified::tests::definition_failure_*`, `property_definition_failure_starts_at_the_definition` |
| envelope-prefix convention | `simplified::tests::failure_paths_join_under_an_envelope_prefix` |
| DMLS related info and watched files still read every origin | existing DMLS L1 watched-file and named-type diagnostic sessions (in `just test`) |

Phase 1 status: **implementation complete, ready for review**.
