# Phase 1 Test Matrix

Phase 1 establishes executable contracts before production changes. Future
contracts are `#[ignore]` so the normal suite remains green; an explicit
ignored-only run proves that they compile and currently fail at the missing
semantic behavior. Test assertions use public parser, validator, resolver,
filesystem round-trip, CLI, and LSP session surfaces rather than private
implementation details.

## Acceptance criteria

| AC | Observable requirement | Concrete coverage |
|---:|---|---|
| 1 | `type-definition` is canonical and accepts one `PropertyDef` | `darkmatter/lib/tests/meta_schema_phase1.rs::type_definition_keyword_round_trips_and_reaches_the_catalog` and `type_definition_validation_matches_the_property_definition_matrix`; later canonical proptest cases in `schemas_grammar_proptest.rs` |
| 2 | Scalar, mapping, and union parity, including malformed values | `type_definition_validation_matches_the_property_definition_matrix` includes plain/single/double-quoted definitions, native mappings, non-empty mixed unions, native boolean/number/null, missing value, empty union, invalid arm, and bad constraint |
| 3 | `schema` accepts inline shapes, local references, and root unions without I/O | `schema_validation_is_syntax_only_and_rejects_remote_references` uses nonexistent path-qualified and bare local references plus inline and union values through `DarkmatterSchemas::validate` |
| 4 | Invalid scalars, remotes, empty unions, and bad arms are rejected | Negative matrix in `schema_validation_is_syntax_only_and_rejects_remote_references`; resolver acceptance remains asserted by `shipped_base_schema_retypes_schema_and_preserves_resolution_acceptance` |
| 5 | Both lower to carrier domains and grammar-backed keywords without mutating validation-only input | `shipped_base_schema_retypes_schema_and_preserves_resolution_acceptance` recursively checks the compiled custom keyword; Phase 4 adds exact fragments in `schemas_compile_validation.rs` and compose write-back coverage in `darkmatter/cli/tests/schema_compose.rs` |
| 6 | Semantic arrays are distinct from union-valued items | `semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips` covers `type-definition[]`, `schema[]`, flat arrays, nested union items, validation, and two write/read/write/read filesystem cycles |
| 7 | Semantic-only/source-aware parser parity and precise authored spans | `existing_source_projection_covers_union_quote_crlf_and_utf8_variants`; Phase 2 adds semantic-product parity and structural sidecar assertions in `schemas_source_projection.rs`, with generators in `schemas_grammar_proptest.rs` |
| 8 | Base `$schema` becomes `schema`; valid old forms remain; DMLS hover changes | `shipped_base_schema_retypes_schema_and_preserves_resolution_acceptance` uses inline, referenced simplified, root-union, and referenced raw JSON forms; `darkmatter/dmls/tests/lsp_session.rs::meta_schema_phase1_schema_hover_uses_nominal_type` covers the real LSP path |
| 9 | DMLS completion, diagnostics, and standalone activation use parser state | Phase 6 session cases in `darkmatter/dmls/tests/lsp_session.rs`, backed by unit state transitions in `darkmatter/dmls/src/schema_overlay.rs`; corpus fixtures use all shipped `docs/schemas/*.yaml` artifacts |
| 10 | DMLS is side-effect-free and retains last-good standalone state | Phase 6 L2 sessions in `lsp_session.rs` update a valid standalone schema to malformed content, assert dependent hover/completion state is retained, and use nonexistent references to prove no resolution I/O |
| 11 | One nesting boundary produces structured errors | `native_mapping_depth_uses_the_shared_structured_limit` tests `MAX_INLINE_OBJECT_DEPTH` and one beyond; Phase 2 proptests generate bounded string-form and native mappings |
| 12 | Existing behavior is byte-identical except the intentional type/failure-stage changes | `phase1-baseline*.txt`, the shipped-corpus classifier, the source projection regression, resolver acceptance assertions, and Phase 8 baseline replays; import-name regression cases are added to `schemas_grammar_proptest.rs` |
| 13 | Scoped L1/L2 suites and lints pass portably | Area `just test`, `just test-l2`, and `just lint`; test paths use `tempfile`, `Path`, `FileReference`, and file URLs rather than macOS-specific separators or locations |

## Shipped-artifact and end-to-end coverage

`shipped_schema_corpus_is_passively_classified_without_resolution` is the
passive corpus test. It reads every shipped YAML artifact in `docs/schemas` and
asserts the known classification outcome, including the intentionally rejected
schema claim. The base-schema contract loads the real shipped baseline through
`with_darkmatter_baseline_json_schema`, then validates a Markdown document by
the ordinary public invocation path. The DMLS hover contract initializes a
real server session, opens a document, and requests hover through LSP.

## Phase 1 expected-failure evidence

The ignored-only library run executes six contracts. All six fail at intended
missing behavior: unknown `type-definition`, unknown `schema`, absent native
mapping depth enforcement, or absent `x-darkmatter-schema` compilation. The
ignored DMLS session fails because the observable hover is still
`Type: **any**`. The two non-ignored Phase 1 regressions pass.

## Validation results

- The full L1 population passed across `darkmatter`, `darkmatter-cli`, and
  `dmls` using deterministic `just test --partition hash:N/8` shards. The
  shards were necessary because a monolithic run exceeded the non-interactive
  60-second command ceiling; before interruption it had completed 2,589 tests
  with no assertion failures.
- `BISCUIT_L2_THREADS=2 just test-l2` passed all 19 Darkmatter library L2
  tests. The CLI tier has two pre-existing, scope-unrelated failures:
  `level2_code_block_clears_inherited_dim_before_theme_colors` and
  `level2_code_block_inverts_to_light_in_dark_terminal`. Both failed again in
  an isolated serial rerun on their color-luminance assertions. No Phase 1
  source or test participates in those flows.
- `just lint` passed for all three affected packages.
