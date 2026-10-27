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
    - darkmatter-cli
human_review: false
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/signature.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
    - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
    - darkmatter/dmls/tests/l1/core_schema_types.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/schema.md
message_to_agent: |-
    Phase 3 (DMLS) is complete; Phase 4 owns the docs, which now lag the code:
    - dmls/docs/diagnostics.md: inline `$schema` and `schema`/`type-definition`
      meta-typed grammar errors underline the token (structural failures keep
      the value; a `schema` failure with no property segment keeps the whole
      value); a definition inside a pure root-union mapping arm is now
      dm.schema.invalid_type_definition at its token (wrong-type arm stays
      invalid_schema_shape); conflict related info reads
      "`max(5)` is declared here" / "`scheme(https)` is the effective
      constraint from this declaration", and bare `alphanumeric` relates its
      definition span; an invalid tagged envelope (extra key) reports
      document_malformed even if a definition inside is also bad.
    - docs/topics/schemas/dmls-schema-support.md: anchored/tagged plain
      continuations complete while typing; the cursor must be inside the
      scalar, at its end, or after trailing spaces/tabs on its last line.
    New public library API to document (definition.md / library surfaces):
    locate_type_definition_cursor_in_scalar and
    locate_schema_declaration_cursor_in_scalar (cursor.rs). Library fix:
    decode_plain_scalar_node returns None at an empty holder's line break.
    All decisions are in the log under Phase 3 -> Design Decisions.
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

## Phase 2

Waves 2–4 plus **Verify input boundaries**. The plain decoder was written
directly; **Retain conflict values** and **Thread failure paths** ran as two
concurrent subagents with disjoint file ownership; Wave 4 projection, the
standalone locator, and the input-boundary matrix were written directly after
both returned.

### What changed

- **Plain decoding** (`simplified/yaml_scalar.rs`): new public
  `decode_plain_scalar_node(source, start, parent_indent)`. It shares
  `skip_node_properties` and a new `decode_plain_content` (the plain branch
  factored out of `decode_scalar_node`), so its map and consumed end are
  identical to the general decoder's for every value it accepts. Re-exported
  from `simplified` and `schemas`.
- **Conflict values** (`simplified/serialize.rs`, `schemas/errors.rs`):
    - `Constraint::to_schema_text(&self, Option<SimplifiedType>) -> String`
      renders one constraint through the existing writer; options render as
      canonical `alphanumeric(allow-dashes,…)` or bare `alphanumeric` (no
      panic path; the `unreachable!` in `write_constraint` is still only
      reachable from `render_constraints`, which filters options first)
    - `Constraint::diagnostic_excerpt(&self) -> String` is the one shared
      excerpt: `to_schema_text(None)`, then `\n`/`\r`/`\t` and `\u{hex}` for
      every other control character and U+2028/U+2029, then shortened to at
      most 160 Unicode scalars including the final `…` (never splitting an
      escape)
    - `ConstraintConflict`'s `Display` and terminal `StatusBlock` show each
      participant's excerpt in a Markdown code span
      (`renderable::markdown::code_span`, so a backtick in a value cannot
      close the span; terminal code spans render literally, so schema text
      cannot become markup)
    - serializer quoting fix: a string default that reads as a number,
      `true`, `false`, or `null` is quoted (`default('3')` vs `default(3)`);
      this also fixed `"3"` reparsing as a number and `"1e999"` failing to
      reparse. Compound (Rust-only) defaults are compact JSON instead of a
      broken quoted string
    - Phase 1's participant construction in `resolve.rs` was correct for
      every case tested; `resolve.rs` was not changed
- **Failure paths** (`simplified/mod.rs`, `grammar.rs`, `signature.rs`):
  `parse_property_definition_at_depth`, `parse_mapping_atom`, and
  `parse_dollar_constraints` take the node's path and return
  `DefinitionFailure`. Attribution: scalar grammar error → the property;
  nested mapping key → `path.property(k)`; property-level union item *i*
  (grammar error, `|` union in a list item, wrong item type) →
  `path.union_arm(i)`; empty union → the sequence itself; second `suggest` →
  the arm holding it (`second_suggestion_span` now returns
  `(arm, span)`); duplicate overload → the later arm (new
  `signature::duplicate_overload_arm`); depth limit → the mapping that
  exceeded it; `$constraints` entry → `$constraints.<key>`. The
  `expect(dead_code)` on `DefinitionFailure.path` was removed (source.rs now
  reads it).
- **Projection** (`simplified/source.rs`):
    - `parse_property_definition_with_source` and
      `parse_schema_declaration_with_source` return
      `Result<SourceAware<T>, LocatedDefinitionFailure>`. A semantic failure
      locates the YAML once (strict grammar) and follows the path in lockstep
      with the parsed value (`follow_path`: same kind, same size, same key at
      the same position). `value_span` is the rejected node when its subtree
      verifies against the parsed value; `authored_span` projects a
      `Grammar` span only through a scalar whose decoded text equals the
      rejected string, and only when both the span and its projection are
      nonempty. Offsets are applied once. A projection failure after a
      successful parse is the projection error at `root()` with no ranges.
    - new public `locate_standalone_definition_failure(source)`; it uses
      `standalone::recognize_standalone`, extracted from
      `parse_standalone_schema_document` (same admitted-YAML loader, same
      catalog and tagged-envelope checks, now one function both use)
    - exported from `simplified` and `schemas`
- DMLS `overlay/named_types.rs` still calls the declaration API with
  `.ok()?` and compiles unchanged; Phase 3 consumes the located failure.

### Design Decisions

- **Outer vs definition failure in the standalone locator is decided by the path**
    - **decision:** `locate_standalone_definition_failure` returns a failure
      only when its path contains at least one property segment; a failure at
      the payload root or only at a root-union arm index (payload of the wrong
      type, empty root union, wrong arm type, invalid/remote reference) is a
      declaration-shape failure and returns `None`. Recognition errors
      (invalid YAML, catalog, malformed tagged envelope) return `None` too.
    - **alternatives considered:** list the outer error messages; return every
      parser failure and let DMLS filter.
    - **why:** the spec requires `None` for "outer envelope or
      declaration-shape failures" while definitions inside root-union mapping
      arms must be located; the structural path draws exactly that line
      without matching message text.
    - **downsides:** a root-level shape failure inside a tagged `types`
      mapping (a non-string or malformed pattern key, attributed to the
      mapping itself) also returns `None` and keeps DMLS's outer fallback.
- **The standalone-only cross-definition checks stay outer**
    - **decision:** the tagged envelope's named-type duplicate-overload and
      undeclared-category checks (`check_named_types`) are not reported by the
      locator; they remain `SchemaDocument` errors from the classifier.
    - **alternatives considered:** run them in the locator with
      `duplicate_overload_arm` and synthesize a `Grammar` error.
    - **why:** the spec scopes the locator to the shared semantic parser's
      first failure; synthesizing a new error would change the semantic error
      the classifier reports.
    - **downsides:** those two diagnostics keep the existing whole-definition
      fallback in DMLS.
- **An alias gets a value range but no token**
    - **decision:** when the rejected node is an alias, `value_span` is the
      alias token (where the definition is used) and `authored_span` is
      `None`.
    - **alternatives considered:** project into the anchored definition;
      leave both `None`.
    - **why:** the spec forbids substituting another node's token; the alias
      itself is the trustworthy location of the rejected use, matching
      `verify_located`'s existing rule that an alias agrees with its value.
    - **downsides:** the underline is the alias, not the anchored text.
- **Excerpt escaping** (subagent decision, adopted)
    - **decision:** `\n`, `\r`, `\t`, and `\u{hex}` for other controls and
      U+2028/U+2029; backslashes are not doubled.
    - **alternatives considered:** caret notation; hex bytes; doubling
      backslashes.
    - **why:** familiar, unambiguous for controls, keeps regex-like values
      readable; the excerpt is for reading, not reparsing (typed data holds
      the value).
    - **downsides:** an authored `\n` (two characters) looks like an escaped
      line feed.
- **Number-looking string defaults are quoted by `admit_number_spelling`** (subagent decision, adopted)
    - **decision:** quote any string the number admitter does not reject as
      `NotANumber`, plus `true`/`false`/`null`.
    - **alternatives considered:** the lexer's narrower number test.
    - **why:** one authority decides what reads as a number; strings such as
      `1e999` were previously unreparseable.
    - **downsides:** canonical text changes for those strings (`.5` →
      `'.5'`); meaning is unchanged.
- **`duplicate_overload` kept beside `duplicate_overload_arm`** (subagent decision, adopted)
    - **decision:** the classifier's `check_named_types` keeps calling
      `duplicate_overload`, now a thin wrapper.
    - **alternatives considered:** one function with an index everywhere.
    - **why:** the parser needs the arm; the classifier needs only the
      message.
    - **downsides:** two entry points for one check.
- **Bare `alphanumeric` reached through an alias names the alias** (observed, unchanged)
    - **decision:** keep Phase 1's synthetic origin, which names the
      declaration the use site referenced (for `far.slug` ← `near.slug_mid`,
      the `slug_mid` declaration).
    - **alternatives considered:** carry the far declaration's provenance
      through alias resolution.
    - **why:** the spec says to keep existing origin selection; that origin is
      a real declaration of the type the use site named.
    - **downsides:** DMLS's related location points at the alias rather than
      the far declaration that wrote bare `alphanumeric`.

### Requirement-to-test mapping (Phase 2)

| Requirement | Test |
| ----------- | ---- |
| plain helper: anchors/tags either order, agrees with general decoder | `yaml_scalar::tests::plain_node_skips_same_line_anchors_and_tags_in_either_order` |
| plain continuation, LF/CRLF, projected continuation | `yaml_scalar::tests::plain_node_follows_continuation_lines_with_lf_and_crlf` |
| blank folding, comment termination, dedent | `yaml_scalar::tests::plain_node_folds_blank_lines_and_stops_at_comments_and_dedent` |
| sequence indentation (dash column) | `yaml_scalar::tests::plain_node_in_a_sequence_uses_the_dash_column` |
| rejects quoted/block/collections/alias/comment/empty/later-line properties | `yaml_scalar::tests::plain_node_rejects_every_other_presentation` |
| all four conflict kinds via two imports: values, origins, occurrences, spans, order, Display | `schemas_named_type_merge::{bound_conflicts_through_two_imports_carry_each_surviving_bound, scheme_conflicts_through_two_imports_carry_the_effective_inherited_list, default_conflicts_through_two_imports_keep_scalar_identity, option_conflicts_through_two_imports_carry_both_canonical_sets}` |
| stricter bound / equal bound keeps first origin | `schemas_named_type_merge::a_stricter_bound_replaces_the_looser_one_and_an_equal_bound_keeps_the_first_origin` |
| item/container default reconciliation | `schemas_named_type_merge::item_and_container_defaults_reconcile_with_their_values` |
| single-constraint text, options, scalar identity, compound JSON | `serialize::tests::{to_schema_text_writes_one_constraint_canonically, default_strings_that_look_like_other_scalars_are_quoted_and_round_trip}` |
| excerpt escaping and 160-scalar limit | `serialize::tests::{diagnostic_excerpt_escapes_controls_and_line_separators, diagnostic_excerpt_keeps_at_most_160_scalars_and_never_splits_an_escape}` |
| Display and StatusBlock share the excerpt; markup literal; full value kept | `errors::tests::conflict_excerpts_*` (5 tests) |
| failure paths: nested literal keys, union members, mapping arms, second suggestion, duplicate overload, root-union arms, depth limit, `$constraints` | `simplified::tests::definition_failure_*`, `property_definition_failure_starts_at_the_definition`, `signature::tests::duplicate_overload_arm_names_the_later_duplicate` |
| token projection through plain, quoted (escaped), continued (LF/CRLF), block scalars with offset | `schemas_source_projection::a_located_property_failure_projects_its_token_through_each_scalar_presentation` |
| declaration API: literal keys, same-prefix second member, mapping arm, root-union arm | `schemas_source_projection::a_located_declaration_failure_follows_literal_keys_and_sequence_members` |
| cross-arm second suggestion (flow and block) | `schemas_source_projection::a_second_suggestion_across_valid_arms_is_located_at_the_second_candidate` |
| structural failures: value range, no token | `schemas_source_projection::structural_failures_have_a_value_range_but_no_token` |
| unsupported sibling, alias, semantic/source mismatch keep the error | `schemas_source_projection::an_unprojectable_failure_keeps_its_semantic_error` |
| projection-only failure wrapped at root | `schemas_source_projection::a_projection_only_failure_is_wrapped_at_the_root_without_ranges` |
| non-ASCII + CRLF + offset | `schemas_source_projection::a_located_failure_ranges_cover_non_ascii_and_crlf_documents` |
| inline frontmatter declaration in document bytes (LF/CRLF) | `schemas_source_projection::an_inline_frontmatter_declaration_failure_is_located_in_document_bytes` |
| standalone: both envelopes, plain/quoted/continued/block, nested literal key, root-union arm; classifier keeps its `SchemaDocument` error | `schemas_source_projection::standalone_definition_failures_are_located_in_both_envelopes` |
| standalone failure returned even when unlocatable | `schemas_source_projection::a_standalone_failure_is_returned_even_when_it_cannot_be_located` |

### Input robustness matrix

Default outcome (per the plan): an invalid recognized schema is an error from
the public classifier; the source-aware route adds the best trustworthy
location; nothing executes or publishes. Outer loader errors keep their
existing classification; the standalone locator returns `None` for them.

- **Loader (admitted YAML, shared by classifier and locator), one branch:**
  control (valid tagged and pure documents → `None`, classifier `Ok`),
  invalid YAML, duplicate key, second YAML document, trailing invalid content
  → classifier `Err`, locator `None`
  (`standalone_documents_without_a_definition_failure_return_none`,
  `outer_envelope_and_declaration_shape_failures_return_none`). YAML null
  syntax (`nothing:`) is the null type → valid. JSON-shaped YAML (flow
  `[...]`) uses the same loader and is covered by flow cases above.
- **Envelope recognition:** missing/other `kind` with `types` → unrecognized;
  extra pure-root keys → unrecognized; catalog, `types: 7`, `types:` (null),
  missing `types`, extra tagged keys, a **non-string** extra key (`7: x`,
  proves the `filter_map(as_str)` + length check still rejects) → outer
  error; `types: {}` → explicit empty schema, valid.
- **Pure `$schema` payload:** `7`, null, `[]`, `[7]`, remote reference arm →
  outer error / `None`; whole-file reference and mapping arms → valid.
- **Property-definition validator (one shared validator; every `types` entry
  uses it):** control `[string, number]` valid; null → null type (data, not
  absent); `{}` → valid empty object; wrong whole type (`7`, `true`) → error at
  the property with its value range; empty `[]` → structural error; one wrong
  element `[string, 7]` → arm 1; every element wrong `[7, true]` → arm 0
  (first, never filtered); nested sequence `[[string]]` → arm 0
  (`standalone_definition_field_shapes_are_located_through_one_validator`,
  `structural_failures_have_a_value_range_but_no_token`). Absent: a missing
  map entry declares no property; no API takes an absent value.
- **`categories`:** unchanged validator; its existing matrix
  (`schema_grammar_descriptors::a_declared_categories_list_limits_category_names`)
  covers absent (unrestricted), `[]` (none allowed), null, wrong member type,
  duplicate, blank name.
- **Smell grep** over touched loading/dispatch code (`standalone.rs`,
  `source.rs` new code, `simplified/mod.rs`): no `#[serde(default)]`, no
  `unwrap_or_default()`; `filter_map(YamlValue::as_str)` only builds the
  unsupported-key message and is backed by the length check (tested above);
  `.ok()` occurs only where declining is the contract (`recognize_standalone`
  outer errors → `None`; an unlocatable source → no ranges, error kept), and
  in `constraints_usize`, where an out-of-range integer still becomes the
  `$constraints.<key>` error through `ok_or_else`.

### Verification

- focused while implementing: `cargo nextest run -p darkmatter --lib -E
  'test(/yaml_scalar::tests/)'` → 28 passed; `cargo nextest run -p darkmatter
  -E 'binary(l1) & test(/schemas_source_projection::/)'` → 24 passed (two
  first-run failures were my fixture's invalid quoted sibling, fixed in the
  fixture); subagent scoped runs: conflict/serializer filters 683 passed,
  grammar/path filters 669 passed, darkmatter-cli conflict tests 10 passed,
  dmls conflict/named-type tests 24 passed
- `cargo check -p darkmatter -p dmls -p darkmatter-cli --tests`: clean
- `just lint` (darkmatter): passed, including the zed-dmls wasm check
- `just test` (darkmatter): **12655 passed**, 10 skipped, 0 failed (88 s)
- downstream Claudine (well over 200 tests, so targeted): `cargo check -p
  claudine -p claudine-cli --tests` clean; `cargo nextest run -p claudine -p
  claudine-cli -E 'test(schema) | test(conflict) | test(catalog) |
  test(preflight) | test(trigger) | test(completion) | test(default)'` →
  **2608 passed**, 0 failed
- gate skip did not apply: the phase changed compiled source in
  `darkmatter` (and a `darkmatter-cli` test), so both gates ran
- cross-OS check skipped: pure in-memory string/byte parsing with no paths,
  processes, or OS-specific code; CRLF and non-ASCII byte ranges are covered
  by the tests above
- `darkmatter/cli/tests/l1/schema_validate.rs` was edited by the conflict
  subagent: its multi-hop assertion expected the old keyword-only text and now
  expects `` `min(10)` in `` / `` `max(5)` in ``
- left for Phase 4 (its ownership): `darkmatter/docs/topics/schemas/definition.md`
  still describes conflict participants as origins only and the source-aware
  APIs' old error type

Phase 2 status: **implementation complete, ready for review**.

## Phase 3

Wave 5, both tasks (**Publish typed diagnostics** and **Complete anchored
continuations**), implemented directly in one pass without subagents: the two
tasks share the DMLS test file `schema_yaml_spellings.rs` and the completion
task needed a small library addition first.

### What changed

- **Library cursor over a decoded scalar** (`simplified/cursor.rs`,
  `yaml_scalar.rs`, both export modules): new public
  `locate_type_definition_cursor_in_scalar(source, &DecodedScalar, cursor)` and
  `locate_schema_declaration_cursor_in_scalar`. They read the cursor through
  the map `decode_plain_scalar_node` returned (crate-private
  `DecodedScalar::prefix_at`), so a folded continuation is read as YAML folds
  it and an anchor/tag never enters the token. The existing raw-text
  `scan_scalar`/`whole_arm` now delegate to the same `scan_decoded` /
  `whole_decoded`, so there is still one cursor scanner.
- **Library fix** (`yaml_scalar.rs`): `decode_plain_scalar_node` called at an
  empty holder's line break decoded the *following* lines as the holder's
  scalar (`entry:\n  - x\n  ma` decoded `" - x ma"`), contradicting its
  documented "empty value → `None`". A value whose start is a line break,
  space, or tab is now `None`; regression rows added to
  `plain_node_rejects_every_other_presentation`.
- **DMLS completion** (`providers/frontmatter.rs`): `continued_scalar_holder`
  now returns the holder line end and the holder's collection column (key
  column, or the dash column of a `- ` item) and has no prefix blacklist; the
  new `continued_scalar_cursor` decodes from the holder's value start with
  `decode_plain_scalar_node`, requires the cursor to be reached, and carries
  the decoded scalar in a new `SemanticValueCursor` (replacing the
  `(usize, Vec<MetaSchemaKind>)` tuple) into the in-scalar cursor locators.
- **DMLS standalone diagnostics** (`diagnostics/frontmatter.rs`):
  `standalone_type_definition_diagnostic` calls
  `locate_standalone_definition_failure`; range order authored token → value →
  deepest frontmatter entry along the path's leading property keys
  (`enclosing_entry_span`, whole segments, never split) → whole document.
  `grammar_token_span`, `collect_string_scalars`, and the separate serde parse
  are deleted.
- **DMLS inline `$schema` route**: `schema_prepare_diagnostic_with_origin`'s
  `Grammar`/`Convert` branch uses `inline_definition_span`, which takes the
  located failure from `AuthoredDeclaration::frontmatter_failure` (new; shares
  `parse_frontmatter_declaration` with `AuthoredDeclaration::frontmatter`, so
  the `.ok()?` at `overlay/named_types.rs:72` is now a deliberate "no
  declaration" while the failure is exposed) when its message equals the
  prepare error; otherwise the existing property lookup. Referenced-file
  failures never take this path.
- **DMLS meta-type consumers**: `semantic_authored_diagnostics` and
  `semantic_problem_range` locate through `locate_meta_type_failure` (the
  source-aware property/declaration parser on the value re-parsed in place).
  Acceptance is still decided by the source-free parsers, so union acceptance
  is unchanged. `smallest_invalid_definition_span`,
  `collect_definition_candidates`, and `nested_property_value_span` (reparse /
  property-label guessing) are deleted.
- **DMLS conflict related information** (`diagnostics/named_types.rs`):
  `` `<excerpt>` is declared here `` using `Constraint::diagnostic_excerpt()`;
  every `Scheme` participant says `` `<excerpt>` is the effective constraint
  from this declaration ``. `origin_location` became `participant_location`,
  which falls back to the origin's `Definition` span for an empty `Options`
  participant without a constraint span. Watched files
  (`overlay/mod.rs::error_dependencies`) already walked participants through
  `ReferencedSchema`/`Aggregate`; unchanged, now covered by a session test.

### Design Decisions

- **Read the continuation through the decoded map with a library helper**
    - **decision:** add `locate_*_cursor_in_scalar` taking the
      `DecodedScalar` from `decode_plain_scalar_node`, instead of passing raw
      text from the value start.
    - **alternatives considered:** pass the raw slice after the node
      properties to the existing locator (anchor excluded, but no folding);
      keep the raw slice from the anchor.
    - **why:** the plan says to carry the decoded map into cursor and
      replacement projection; it also makes a quoted argument folded across
      lines read as YAML decodes it. Sharing `scan_decoded` keeps one scanner.
    - **downsides:** two more public functions in the schema surface.
- **A cursor after trailing blanks on the scalar's last line is accepted**
    - **decision:** the cursor must be inside the decoded scalar or at its
      end, or separated from the end only by spaces/tabs on the same line
      (a fresh position there).
    - **alternatives considered:** the spec's literal "within the consumed
      scalar or immediately after its final byte".
    - **why:** YAML trims trailing blanks from a plain scalar, so the literal
      rule would stop completion after `max(2); ` on a continuation line,
      which worked before this change (loaded and incomplete alike). Blanks
      cannot hide a comment, sibling, or later line, which is what the rule
      guards against.
    - **downsides:** one documented widening of the spec's boundary.
- **Empty holder values never decode later lines** (library fix)
    - **decision:** `decode_plain_scalar_node` returns `None` when the value
      start is a line break or blank.
    - **alternatives considered:** reject in DMLS before calling.
    - **why:** the function's contract already promised it; fixing it at the
      authority keeps DMLS free of a second rule.
    - **downsides:** none observed; the general decoder is unchanged.
- **An invalid tagged envelope reports the envelope, not a definition inside it**
    - **decision:** `meta_schema_standalone_block_plain_flow_indicator_retains_last_good`
      now expects `dm.schema.document_malformed` (naming the unsupported
      `description` key) instead of `dm.schema.invalid_type_definition`.
    - **alternatives considered:** keep DMLS's own serde parse to dig inside
      invalid envelopes; make the library locator tolerate extra keys.
    - **why:** the spec requires the locator to return `None` for outer
      envelope failures and DMLS to keep its outer route; the old result came
      from the separate serde parse the spec deletes. The new diagnostic is
      the classifier's own error, matching `md schema validate`.
    - **downsides:** a file with both an envelope error and a bad definition
      shows the envelope error first.
- **A definition inside a root-union mapping arm is an inner-definition error**
    - **decision:** `$schema: [ {count: number(bogus(1))} ]`-style arm
      definitions now publish `dm.schema.invalid_type_definition` at the
      token (previously `dm.schema.invalid_schema_shape` on the arm); an arm of
      the wrong type is still `invalid_schema_shape` on the arm.
    - **alternatives considered:** keep the shape code for anything under an
      arm.
    - **why:** the spec says definitions inside pure root-union mapping arms
      are covered by the locator and carry the indexed path; the code follows
      the failure kind, as for a tagged `types` entry.
    - **downsides:** clients that matched the shape code for those errors see
      the definition code.
- **Meta-typed values keep source-free acceptance; source-aware only locates**
    - **decision:** decide acceptance with the source-free parsers, then call
      the source-aware parser for the chosen failure and use it only if its
      message equals the reported error. A `schema` failure with no property
      segment keeps the whole value.
    - **alternatives considered:** accept/reject with the source-aware parser
      directly.
    - **why:** a projection-only failure after a successful semantic parse
      would otherwise reject a valid union arm (weakening acceptance is
      forbidden); the no-property rule mirrors the standalone locator's
      outer/inner line and keeps shape diagnostics on their value
      (`a_diagnostic_range_covers_a_collection_holding_a_content_colon`).
    - **downsides:** a failing value is parsed twice (failure path only).
- **Grammar errors in meta-typed values underline the token**
    - **decision:** updated three existing expectations from the whole
      value to the offending token (`nope`, `bogus`):
      `meta_schema_diagnostics_replace_generic_keyword_failures`,
      `meta_schema_phase7_inline_hover_completion_and_diagnostics`,
      `a_malformed_arm_after_the_first_block_sequence_item_is_ranged_exactly`.
      Structural failures (`42`, `[]`) keep their value range.
    - **alternatives considered:** keep the smallest-invalid-value guess for
      meta-typed values.
    - **why:** the plan selects authored token, then value, then fallback for
      these consumers; the guess was the reparse pattern this feature removes.
    - **downsides:** a visibly narrower squiggle than before for those cases.
- **Unmatched inline errors keep the property-label lookup**
    - **decision:** when the located failure's message differs from the
      prepare error (e.g. a `Convert` error the parser never reports),
      `inline_definition_span` keeps the existing `$schema.<property>` lookup.
    - **alternatives considered:** fall straight to the `$schema` value.
    - **why:** surgical; those errors have no structural path to use.
    - **downsides:** that legacy lookup remains for conversion errors only.

### Requirement-to-test mapping (Phase 3)

| Requirement | Test |
| ----------- | ---- |
| decoded-map cursor excludes anchors/tags; LF/CRLF; blank fold | `cursor::tests::a_decoded_continuation_reads_the_cursor_without_its_anchor_or_tag` |
| cursor accepted at end / after trailing blanks; rejected in anchor, sibling, comment | `cursor::tests::a_decoded_scalar_accepts_only_cursors_it_authored_or_trailing_blanks` |
| declaration arm read through the decoded map | `cursor::tests::a_decoded_declaration_arm_is_one_token_without_its_anchor` |
| empty holder never decodes later lines (fix) | `yaml_scalar::tests::plain_node_rejects_every_other_presentation` (new rows) |
| anchored/tagged/blank-fold/sequence-item continuation: loaded vs incomplete identical labels and edit, `&e`/`!!str` preserved, LF/CRLF, UTF-8/UTF-16 | `schema_yaml_spellings::an_anchored_or_tagged_continuation_completes_like_the_loaded_document` |
| comment line, trailing comment, single/double quoted, block, alias, flow, dedented sibling, sequence sibling offer nothing | `schema_yaml_spellings::a_line_the_plain_scalar_does_not_reach_offers_none_of_its_completions` |
| completion is passive (no `$(...)` run, no import read) | `schema_yaml_spellings::continuation_completion_runs_and_loads_nothing` |
| existing continuation/indentless/non-BMP completion | existing `completion_on_continuation_text_and_indentless_items_replaces_the_authored_token`, `completion_after_non_bmp_text_on_a_continuation_uses_negotiated_columns` |
| related info shows values (`min(10)`/`max(5)` declared here) across files | `named_type_references::a_multi_hop_contradiction_names_the_reference_and_both_declarations` (extended) |
| options values in related info | `core_schema_types::an_options_conflict_relates_the_definition_and_the_use_site` (extended) |
| bare `alphanumeric` relates its definition span | `core_schema_types::a_bare_alphanumeric_conflict_participant_relates_its_definition` |
| effective scheme wording after narrowing | `core_schema_types::a_narrowed_scheme_conflict_relates_the_effective_constraint` |
| wrapped conflict watches every participant file | `named_type_references::a_wrapped_conflict_watches_every_participant_file` |
| standalone token failure via locator (both envelopes, continuation, non-BMP, LF/CRLF, UTF-8/16); unprojectable fallback | existing `schema_yaml_spellings` malformed-definition tests (all pass on the new route) |
| root-union arm definition → token; wrong-type arm stays shape | `schema_yaml_spellings::a_definition_inside_a_root_union_arm_is_ranged_at_its_token` |
| inline `$schema` nested mapping (literal `a.b` key), flow and block sequence member; LF/CRLF, non-BMP prefix, UTF-8/16 | `schema_yaml_spellings::an_inline_schema_definition_error_is_ranged_at_its_token` |
| referenced-file failure stays on the local reference | `schema_yaml_spellings::a_referenced_definition_error_keeps_the_local_reference_range` |
| meta-type token ranges; union acceptance and shape ranges unchanged | `diagnostics::frontmatter::tests::{meta_schema_diagnostics_replace_generic_keyword_failures, a_diagnostic_range_covers_a_collection_holding_a_content_colon}`, `lsp_session::{meta_schema_phase7_inline_hover_completion_and_diagnostics, a_malformed_arm_after_the_first_block_sequence_item_is_ranged_exactly}`, existing union tests |

Input robustness: this phase changes no loader or field validator (it
consumes Phase 2's locator and parsers); the smell grep over the touched DMLS
code finds only deliberate `.ok()` uses (YAML that does not parse on its own
→ no location; `AuthoredDeclaration::frontmatter` → no declaration while
`frontmatter_failure` exposes the error).

### Verification

- focused while implementing: library cursor/scalar tests 61 passed; DMLS
  completion filter 103 passed; DMLS diagnostic filters 172 and 73 passed;
  full `cargo nextest run -p dmls` found four expectation changes recorded
  above, then passed
- `just lint` (darkmatter): passed, including the zed-dmls wasm check
- `just test` (darkmatter): **12667 passed**, 10 skipped, 0 failed (142 s);
  new tests each run under 0.25 s
- downstream Claudine (well over 200 tests, targeted): `cargo check -p
  claudine -p claudine-cli --tests` clean; `cargo nextest run -p claudine -p
  claudine-cli -E 'test(schema) | test(completion) | test(cursor) |
  test(catalog)'` → **2088 passed**, 0 failed. Claudine uses none of the new
  or changed symbols
- gate skip did not apply: compiled source changed in `darkmatter` and `dmls`
- cross-OS check skipped: in-memory string/byte parsing with no paths,
  processes, or OS-specific code; CRLF and UTF-16 positions are covered
- left for Phase 4 (its ownership): `dmls/docs/diagnostics.md` (token ranges
  for inline and meta-typed values, root-union arm code, related-information
  wording) and `docs/topics/schemas/dmls-schema-support.md` (anchored/tagged
  continuation completion and the trailing-blank rule) still describe the old
  behavior

Phase 3 status: **implementation complete, ready for review**.
