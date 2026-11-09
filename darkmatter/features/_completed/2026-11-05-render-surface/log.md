---
kind: log
spec: 2026-11-05-render-surface/spec.md
implementation_1: "2026-11-07T18:22:24-08:00"
---

# Render surface review-repair log

## Implementation of Review Findings #1

> **started at:** 2026-11-07T18:22:24-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-render-surface/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- the review lists no `human_in_the_loop` items and no blocked findings; recurrence is `false`

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Early failures and selective denial discard style diagnostics | fixed | all 15 rows tests in `cli/tests/l1/render_style_carried_warnings.rs` pass; children below |
| R1-F1.1 | `--style missing.yaml`: keep document + trigger warnings with input-read failure | fixed | `a_missing_style_file_keeps_the_document_and_trigger_warnings` pass |
| R1-F1.2 | `--style bad.yaml` (malformed): same preservation, authored positions unchanged | fixed | `a_malformed_style_file_keeps_the_document_and_trigger_warnings` pass (asserts positions 3:3, 5:5) |
| R1-F1.3 | `--style theme@offset.yaml` (nonmapping offset): same preservation | fixed | `an_offset_that_is_not_a_mapping_keeps_the_document_and_trigger_warnings` pass |
| R1-F1.4 | Remote YAML / unsupported `.css` reference: same preservation; no network read | fixed | `a_remote_style_reference_keeps_the_document_and_trigger_warnings` and `an_unsupported_style_extension_keeps_the_document_and_trigger_warnings` pass; no `--allow host`, so no read |
| R1-F1.5 | `style..x=1` setter-path parse failure: same preservation | fixed | `a_setter_that_is_not_a_property_path_keeps_the_document_and_trigger_warnings` pass |
| R1-F1.6 | Earlier valid file's warning kept when a later file fails | fixed | `a_later_failing_file_keeps_an_earlier_files_warning` pass |
| R1-F1.7 | `--deny dm.style.unknown_key`: keep deprecation + trigger warnings and blocked record | fixed | `denying_the_unknown_key_code_keeps_the_other_warnings` pass |
| R1-F1.8 | `--deny dm.style.deprecated`: keep unknown-key + trigger warnings and blocked record | fixed | `denying_the_deprecation_code_keeps_the_other_warnings` pass |
| R1-F1.9 | `style.margin=abc` clean sibling: type error plus warnings | clean | `a_setter_type_error_keeps_the_document_and_trigger_warnings` pass (warnings now precede the error in `diagnostics`) |
| R1-F1.10 | `style={margin:2,margin:3}` clean sibling: invalid-setter plus warnings | clean | `a_duplicate_key_setter_value_keeps_the_document_and_trigger_warnings` pass |
| R1-F1.11 | `--deny style` clean sibling: both warnings appear as errors | clean | `denying_style_makes_both_document_warnings_errors` pass |
| R1-F1.12 | Successful preparation clean sibling: authored + trigger warnings appear | clean | `a_successful_render_reports_the_document_and_trigger_warnings` pass (no override and `style.margin=2`) |
| R1-F1.13 | Formatted output consumer: applicable warnings on stderr, no page on failure | fixed | `a_formatted_failure_prints_the_warnings_and_no_page` pass (plain output; missing file and `--deny dm.style.unknown_key`) |
| R1-F2 | Whole-style setter type failures use the wrong diagnostic family | fixed | `dm.schema.type_mismatch` from the shared `ValidationProblemCode::TypeMismatch` code; children below |
| R1-F2.1 | `style={ margin: ` → `dm.schema.type_mismatch` naming `style`, exit 1 | fixed | `style_layer_matrix::root_setter_malformed_object_fails`; CLI `render_style_acceptance::a_malformed_whole_style_setter_is_a_schema_type_mismatch` pass |
| R1-F2.2 | `style=4` → schema type mismatch | fixed | `root_setter_number_fails`, `style_layers::admission::a_cli_root_must_be_a_mapping` pass |
| R1-F2.3 | `style=null` → schema type mismatch | fixed | `root_setter_null_fails` pass; null members stay absence (`root_setter_null_member_is_absent`) |
| R1-F2.4 | `style=[]` → schema type mismatch | fixed | `root_setter_empty_array_fails`, `root_setter_array_fails` pass |
| R1-F2.5 | No setter clean sibling → success | clean | `root_no_layer_is_the_default_style` pass |
| R1-F2.6 | `style={}` clean sibling → no-op success | clean | `root_inline_empty_over_a_document_is_a_no_op` pass |
| R1-F2.7 | `style.page=4` clean sibling → `dm.schema.type_mismatch` | clean | `container_setter_page_number_fails` pass |
| R1-F2.8 | `style.margin=abc` clean sibling → `dm.schema.type_mismatch` | clean | `container_setter_scalar_field_fails` pass |
| R1-F2.9 | `style.table.border={ left:` clean sibling → `dm.schema.constraint` | clean | `container_setter_union_malformed_object_fails` → `dm.schema.constraint` pass |
| R1-F2.10 | File root scalar/array clean sibling → keeps `dm.style.invalid_root` | clean | `root_file_array_fails`, `root_file_string_fails`, `render_style_files` invalid-root tests pass |
| R1-F2.11 | Docs and tests pinning invalid-root for the setter case updated | fixed | `docs/cli/data-contracts.md` and `docs/cli/render.md` updated; `docs_style_examples` 14/14 pass |
| R1-F3 | Superseded duplicate-key setters still fail parsing | fixed | replacement settled before value parsing in `assemble_setters`; children below |
| R1-F3.1 | Whole-style duplicate key then replacement → success | fixed | `superseded_root_setter_duplicate_keys_are_not_read` pass |
| R1-F3.2 | Union-object duplicate then scalar border → success | fixed | `superseded_union_setter_duplicate_keys_are_not_read` pass |
| R1-F3.3 | Open metadata duplicate then replacement → success | fixed | `superseded_open_object_setter_duplicate_keys_are_not_read` pass |
| R1-F3.4 | Cleanup object duplicate then replacement → success, inert | fixed | `superseded_cleanup_setter_duplicate_keys_are_not_read` pass |
| R1-F3.5 | `style.margin=abc style.margin=2` clean sibling → success | clean | `superseded_scalar_setter_is_not_converted` pass |
| R1-F3.6 | Malformed text then replacement clean sibling → success | clean | `superseded_malformed_root_setter_is_not_read` pass |
| R1-F3.7 | Surviving duplicate object clean sibling → rejection | clean | `inline_container_setter_duplicate_keys_fail`, `surviving_setter_duplicate_keys_still_fail` pass |
| R1-F3.8 | Separate files clean sibling → earlier invalid file still rejected | clean | `style_layers::every_file_is_checked_even_when_a_later_one_replaces_it` pass |
| R1-F3.9 | One CLI wiring test for the replacement | fixed | `render_style_acceptance::a_superseded_duplicate_key_style_setter_is_not_read` pass (exit 0) |
| R1-F4 | Typo hints ignore the document's effective style schema | fixed | suggestions drawn from the effective schema (`AdmittedKeys`, `declared_keys`); children below |
| R1-F4.1 | Custom schema `accent`: `style.acent=blue` suggests `style.accent` | fixed | `style_layers::suggestions::a_custom_schema_setter_typo_suggests_the_custom_key`; CLI `a_custom_style_schema_setter_typo_suggests_the_custom_key` pass |
| R1-F4.2 | Custom schema: `style.magin=4` never suggests rejected built-in `margin` | fixed | `a_custom_schema_setter_typo_never_suggests_a_rejected_built_in_key` pass |
| R1-F4.3 | No document schema: built-in suggestions unchanged | clean | `a_baseline_setter_typo_suggests_the_built_in_sibling`, `a_schemaless_setter_typo_suggests_the_descriptor_sibling` pass |
| R1-F4.4 | File/document rename-only hints unchanged | clean | `file_and_document_rename_suggestions_are_unchanged_under_the_baseline` pass |
| R1-F1.14 | Sibling: a document `$schema` that cannot be built discarded trigger and document warnings when CLI styles were given | fixed | `a_document_schema_that_fails_keeps_the_document_and_trigger_warnings` pass |
| R1-F1.15 | Sibling: the source-output CLI-only retry dropped the first pass's document warnings on failure | fixed | defensive repair; no reachable reproduction (a CLI-layer problem already fails the first pass), so no test |
| R1-F2.12 | Sibling: with no effective schema, `style=4` used the same branch | fixed | `root_setter_number_without_a_schema_fails` pass; nested `style.page=4` with no schema stays `dm.style.invalid_value` (`container_setter_page_number_without_a_schema_is_invalid`); decision recorded in review `human_review_items` |
| R1-F2.13 | Siblings: `style.markdown=4`, open `style.page.meta=4` | clean | `container_setter_markdown_number_fails`, `container_setter_open_object_number_fails` pass (`dm.schema.type_mismatch`) |
| R1-F3.10 | Sibling: custom-schema object field with superseded duplicate key | fixed | `superseded_custom_field_setter_duplicate_keys_are_not_read` pass |
| R1-F4.5 | Sibling: rename-only hint named a key a custom schema rejects (file, document, setter) | fixed | `a_file_rename_suggestion_never_names_a_key_the_custom_schema_rejects` pass |
| R1-F4.6 | Sibling: typo under a nested custom object | fixed | `a_nested_custom_object_setter_typo_suggests_its_sibling` pass |
| R1-F4.7 | Sibling: pattern-opened `style` admits the key, so no error | clean | `a_pattern_opened_style_admits_a_typo_without_a_suggestion_error` pass |
| R1-F4.8 | CLI wiring for effective-schema suggestions | fixed | `render_style_acceptance::a_custom_style_schema_setter_typo_suggests_the_custom_key` pass |

### Progress

- starting the work on 'R1-F1: Early failures and selective denial discard style diagnostics' at 18:22:58
        - defect class: a failure early in style preparation, or a policy stop on one warning, returned before the warnings already found for the other inputs were published, so the final failure reported only its own record
        - sibling sites swept: schema selection (`select_document_schema`, which dropped trigger findings when the document's schema failed to assemble), every early return in `load_style_file`/`classify_style_argument` (missing file, malformed data, offset not a mapping, remote reference, unsupported extension), `setter_layer`, the source-output retry in `prepare_render_style` (its CLI-only pass dropped the first pass's document warnings on failure), and `Render::stop_if_denied`; the link-title policy stop was checked and already carries its earlier warnings (`BlockedDiagnostics`)
            - new sibling R1-F1.14: with a CLI style layer, a document `$schema` that cannot be assembled discarded the trigger findings and the document's style warnings
            - new sibling R1-F1.15: the source-output retry's failure discarded the document warnings of the first pass (fixed defensively; no reachable reproduction found, since a CLI-layer problem already fails the first pass)
        - changes
            - `render_style.rs`: `read_cli_style_inputs` returns `ReadStyleInputs { inputs, failure }`, keeping the trigger findings and every layer read before the first failure; `prepare_cli_style_inputs` is now a thin wrapper with its old contract; `prepare_read_render_style` merges the document layer and the layers that were read even when reading failed and returns `RenderStyleFailure { error, warnings }`; the retry branch prepends the first pass's document warnings to a CLI-only failure
            - `commands/schema/validate.rs`: `select_document_schema` returns the trigger findings beside the schema result; the now-unused `SelectedDocumentSchema` was removed and `effective_document_schema` returns the schema alone (`args/path_completion.rs` updated)
            - `commands/render.rs`: `prepare_style` reports the carried warnings (sink, or stderr for a formatted output, with the existing source-output suppression of document warnings) before returning a failure; on a selective `--deny <code>` stop, the unblocked style warnings are reported and the blocked ones stay the failure's error records
            - `docs/cli/render.md`: documented that a rejected style still reports the earlier warnings, with an example
        - tests: `cli/tests/l1/render_style_carried_warnings.rs` (15 tests, one per row R1-F1.1..R1-F1.14; R1-F1.4 split into remote and `.css`), declared in `tests/l1/main.rs`
        - commands: `just test render_style_carried_warnings` 15/15 passed; `just test render_style` 105/105; `just test render` 1104/1104; `just test style` 818/818; `just test invalid_trigger` 85/85; `just test path_completion` 87/87; `just test schema_validate` 198/198; `just test docs_style` 14/14; `just test output_commands` 175/175; `just test layout_style` 11/11; `just test trigger` 626/626; `just test policy` 265/265; `just test deny` 35/35; `just lint` clean
- work completed for 'R1-F1: Early failures and selective denial discard style diagnostics' at 18:33:53
- starting the work on 'R1-F2: Whole-style setter type failures use the wrong diagnostic family' at 18:33:53
        - defect class: a setter bound at the whole-style container path took a style-specific type-error branch (`dm.style.invalid_root`) instead of the shared schema type-mismatch code every other setter reports
        - sibling sites swept: the whole-style branch of `assemble_setters` (the only setter site reporting `STYLE_INVALID_ROOT_CODE`); nested container setters `style.page=4`, `style.markdown=4`, the open object `style.page.meta=4`, `style.table=null`, and the union `style.table.border={ left:` (all already reported through schema admission); the file-root adapter in `prepare_layer` and the CLI's `KEY@FILE` offset failure in `render_style.rs::located_failure` (kept `dm.style.invalid_root`, R1-F2.10); the document root (`dm.style.invalid_value`, not a setter, unchanged)
            - new sibling R1-F2.12: with no effective schema, `style=4` follows the same branch and is now `dm.schema.type_mismatch` too; a nested container setter with no schema (`style.page=4`) stays `dm.style.invalid_value`, as every descriptor-read setter value does, and is pinned as such
        - changes
            - `lib/src/style/layer.rs`: the whole-style setter branch reports `ValidationProblemCode::TypeMismatch.diagnostic_code()` with the unchanged `style` message and setter origin; `STYLE_INVALID_ROOT_CODE`'s doc now names style files only
            - `docs/cli/data-contracts.md`: moved `style='{ margin: '` (and `style=4`, `null`, `[]`) from the `dm.style.invalid_root` row to the `dm.schema.type_mismatch` row; invalid-root now covers style files and offsets only
            - `docs/cli/render.md`: added the non-mapping `style=` row to the "When a style is rejected" table
        - tests
            - `lib/tests/l1/style_layer_matrix.rs`: root setter cells now expect type mismatch (`root_setter_array_fails`, `root_setter_null_fails`) and new cells `root_setter_empty_array_fails`, `root_setter_number_fails`, `root_setter_malformed_object_fails`, `root_setter_number_without_a_schema_fails`, `root_setter_null_member_is_absent`, plus a container-setter block (`container_setter_page_number_fails`, `container_setter_markdown_number_fails`, `container_setter_open_object_number_fails`, `container_setter_table_null_is_absent`, `container_setter_union_malformed_object_fails`, `container_setter_scalar_field_fails`, `container_setter_page_number_without_a_schema_is_invalid`); a schema-less `Absent` cell kind was added
            - `lib/tests/l1/style_layers.rs`: `a_cli_root_must_be_a_mapping` keeps invalid-root for files and asserts type mismatch, the `style` message, and the setter origin for `style=5`
            - `cli/tests/l1/render_style_acceptance.rs`: `a_malformed_whole_style_setter_is_a_schema_type_mismatch` (JSON output, exit 1, one `dm.schema.type_mismatch` naming `style` and the setter, `source` null)
        - commands: `just test style_layer` 152/152 passed; `just test render_style` 106/106; `just test invalid_root` 1/1; `just test docs_style_examples` 14/14; `just test render_style_acceptance` 6/6; `just test style` 831/831; `just lint` clean (after renaming the matrix's schema-less cell kind to satisfy `clippy::enum_variant_names`)
- work completed for 'R1-F2: Whole-style setter type failures use the wrong diagnostic family' at 18:44:53
- starting the work on 'R1-F3: Superseded duplicate-key setters still fail parsing' at 18:44:53
        - defect class: replacement of a repeated exact setter path was settled after the fallible per-assignment value parse, so an assignment the contract discards could still reject the command
        - sibling sites swept: every raw setter family in `assemble_setters` (whole root, container `style.markdown`, union `style.table.border`, open object `style.page.meta`, custom schema fields, scalars); the other per-assignment checks before replacement (the `style` prefix and the open-object-ancestor refusal) depend only on the exact path, so a superseded and a surviving assignment fail them alike and they are unchanged; the CLI (`render_style.rs`, `commands/render.rs`) builds raw setters only and never parses a value itself; the shared `compose::setters::SetterPlan::new` already settles overlaps before `SetterValue::parse`, and the style plan now mirrors it; equivalent-spelling conflicts are found after assembly by `normalize::canonicalize_aliases`, unaffected; style files stay separate checked inputs (R1-F3.8)
            - new sibling R1-F3.10: a custom schema field (`style.extra`, `object`) with a superseded duplicate key; R1-F3.11: a surviving duplicate-key assignment after a valid earlier one still fails
        - changes
            - `lib/src/style/layer.rs`: `assemble_setters` first decodes and checks each path and keeps the last assignment per exact path (at its first position), then parses only the survivors; the docblock states that a superseded value is never read
            - `docs/cli/render.md`, `docs/topics/layouts-and-style.md`: repeating a path never reads the replaced value; only the surviving value must be valid (with an example checked by `docs_style_examples`)
        - tests
            - `lib/tests/l1/style_layer_matrix.rs`: `superseded_root_setter_duplicate_keys_are_not_read` (R1-F3.1), `superseded_union_setter_duplicate_keys_are_not_read` (R1-F3.2), `superseded_open_object_setter_duplicate_keys_are_not_read` (R1-F3.3), `superseded_cleanup_setter_duplicate_keys_are_not_read` (R1-F3.4), `superseded_scalar_setter_is_not_converted` (R1-F3.5), `superseded_malformed_root_setter_is_not_read` (R1-F3.6), `superseded_custom_field_setter_duplicate_keys_are_not_read` (R1-F3.10), `surviving_setter_duplicate_keys_still_fail` (R1-F3.11); R1-F3.7 stays `inline_container_setter_duplicate_keys_fail`, R1-F3.8 stays `style_layers::every_file_is_checked_even_when_a_later_one_replaces_it`
            - the five duplicate-key cells failed with the old parse-first order restored temporarily and pass with the fix; R1-F3.5/F3.6 pass both ways, as clean siblings
            - `cli/tests/l1/render_style_acceptance.rs`: `a_superseded_duplicate_key_style_setter_is_not_read` (R1-F3.9: JSON output, exit 0, no diagnostics, outcome success)
        - commands: `just test superseded_` 7/7 passed; `just test a_superseded_duplicate_key_style_setter_is_not_read` 1/1; `just test style_layer` 160/160; `just test render_style` 107/107; `just test setter` 233/233; `just test render_style_acceptance` 7/7; `just test docs_style_examples` 14/14; `just test style` 840/840; `just lint` clean
- work completed for 'R1-F3: Superseded duplicate-key setters still fail parsing' at 18:53:15
- starting the work on 'R1-F4: Typo hints ignore the document's effective style schema' at 18:53:15
        - defect class: typo suggestions were drawn from a different key universe (the built-in style descriptor) than the effective schema that decides admission, so a hint could name a rejected key or miss an accepted one
        - siblings swept: every style-key suggestion site in the render style path (`suggest`, `nearest`, `did you mean`, `levenshtein`, `distance` under `lib/src/style` and `cli/src/render_style.rs`); `report_unknown` is the only producer of user-facing hints during preparation; `walker.rs` hints feed only the no-schema path, where the descriptor is the right universe
            - setter nearest-key hint (root and nested custom objects): defective, fixed
            - setter/file/document rename-only hint (`left-margin` → `margin-left`): defective under a custom schema (named a key that schema rejects), fixed by keeping the rename mapping and dropping it when the schema does not admit its target; unchanged under the baseline
            - pattern-opened style (`"<string>": unknown`): clean, a typo is admitted, no unknown-key error
            - `cli/src/render_style.rs`: clean, produces no suggestions of its own
        - changes:
            - `lib/src/markdown/schemas/completion.rs`: new public `declared_keys(effective, frontmatter, path)`, the keys a schema declares under a path, read through the same `SchemaView` the shell completion uses
            - `lib/src/style/descriptor.rs`: the bounded distance choice extracted into `nearest_sibling(segment, candidates)`; `nearest_key` (descriptor siblings) calls it, behavior unchanged
            - `lib/src/style/layer.rs`: `AdmittedKeys` built from the layer's effective schema; `report_unknown` filters the rename hint through `declared_shape` and takes the setter nearest key from `declared_keys` of the parent; descriptor siblings only when there is no schema
            - docs: `docs/cli/data-contracts.md` (`dm.schema.unknown_key` row), `docs/cli/render.md` (nearest valid key), `docs/topics/layouts-and-style.md` (suggestions come from the judging schema, with the `accent` example)
        - tests (`lib/tests/l1/style_layers.rs`, `mod suggestions`):
            - R1-F4.1 `a_custom_schema_setter_typo_suggests_the_custom_key`
            - R1-F4.2 `a_custom_schema_setter_typo_never_suggests_a_rejected_built_in_key` (also the rename hint `style.left-margin`)
            - R1-F4.3 `a_baseline_setter_typo_suggests_the_built_in_sibling`, `a_schemaless_setter_typo_suggests_the_descriptor_sibling`
            - R1-F4.4 `file_and_document_rename_suggestions_are_unchanged_under_the_baseline`
            - siblings: `a_nested_custom_object_setter_typo_suggests_its_sibling`, `a_pattern_opened_style_admits_a_typo_without_a_suggestion_error`, `a_file_rename_suggestion_never_names_a_key_the_custom_schema_rejects`
            - CLI wiring (`cli/tests/l1/render_style_acceptance.rs`): `a_custom_style_schema_setter_typo_suggests_the_custom_key` (JSON output, exit 1, message names `style.accent`; `style.magin=4` names nothing)
            - with the schema keys disabled temporarily, the four custom-schema tests failed and the baseline/no-schema ones passed; all pass with the fix
        - commands: `just test style_layers::suggestions::` 8/8; `just test a_custom_style_schema_setter_typo_suggests_the_custom_key` 1/1; `just test style_layer` 168/168; `just test render_style` 108/108; `just test suggest` 197/197; `just test setter` 239/239; `just test docs_style_examples` 14/14; `just test completion::` 141/141; `just test nearest_key` 1/1; `just lint` clean
- work completed for 'R1-F4: Typo hints ignore the document's effective style schema' at 19:02:55
- starting final reconciliation and phase gates (`just test`, `just lint`, `just check-tier-coverage darkmatter`) at 19:02:55
- phase gates completed at 19:06:43
        - `just test` (darkmatter): 15243 run, 15243 passed, 10 skipped, exit 0 (131 s)
        - `just lint` (darkmatter): clean, including the zed-dmls wasm32-wasip2 check
        - `just check-tier-coverage darkmatter`: zero stranded tests
        - every new test is L1 in a declared binary (`cli/tests/l1/main.rs` declares `render_style_carried_warnings`; the other tests extend existing files)
        - `just test-l2` was not rerun: no repair touched real-terminal rendering, completion scripts, or the no-input terminal path; the formatted-output stderr consumer is covered by an L1 spawned-binary test
        - no cross-OS check was run: the repairs change diagnostic carrying, code selection, setter ordering, and suggestion candidates, none of which involve paths, processes, or OS-specific code; CI covers the other OSes

### Completion Report

- **outcome:** complete
- **elapsed:** 18:22:24 → 19:06:43 (about 1 h 15 min)
- **parent findings:** total 4 = fixed 4 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **instance rows:** 41 review/required rows plus 9 sibling rows found by the sweeps; 32 fixed (including the 4 parents), 18 clean, 0 open
- **verified repairs**
        - R1-F1: style preparation now carries trigger findings, the document's style warnings, and earlier file layers' warnings into every early failure (file opening, data loader, offset, classification, setter path, document schema assembly), and a selective `--deny` keeps the unblocked warnings; formatted output prints them on stderr without a page
        - R1-F2: a non-mapping whole-style setter reports the shared `dm.schema.type_mismatch` naming `style`; file and offset roots keep `dm.style.invalid_root`; docs updated
        - R1-F3: the last assignment per exact setter path is chosen before any value is parsed, so a superseded malformed or duplicate-key value no longer fails the command; the surviving assignment is still fully checked
        - R1-F4: setter typo suggestions and rename hints come from the effective schema when one exists, falling back to the descriptor only without a schema
- **open findings:** none
- **author decisions recorded** in the review's `human_review_items`: R1-F2.12 (schemaless root code), R1-F1 record order and first-failure stop, R1-F4.5 rename-hint filtering
- **changed files**
        - R1-F1: `darkmatter/cli/src/render_style.rs`, `darkmatter/cli/src/commands/render.rs`, `darkmatter/cli/src/commands/schema/validate.rs`, `darkmatter/cli/src/args/path_completion.rs`, `darkmatter/cli/tests/l1/render_style_carried_warnings.rs` (new), `darkmatter/cli/tests/l1/main.rs`, `darkmatter/docs/cli/render.md`
        - R1-F2: `darkmatter/lib/src/style/layer.rs`, `darkmatter/lib/tests/l1/style_layer_matrix.rs`, `darkmatter/lib/tests/l1/style_layers.rs`, `darkmatter/cli/tests/l1/render_style_acceptance.rs`, `darkmatter/docs/cli/data-contracts.md`, `darkmatter/docs/cli/render.md`
        - R1-F3: `darkmatter/lib/src/style/layer.rs`, `darkmatter/lib/tests/l1/style_layer_matrix.rs`, `darkmatter/cli/tests/l1/render_style_acceptance.rs`, `darkmatter/docs/cli/render.md`, `darkmatter/docs/topics/layouts-and-style.md`
        - R1-F4: `darkmatter/lib/src/markdown/schemas/completion.rs`, `darkmatter/lib/src/style/descriptor.rs`, `darkmatter/lib/src/style/layer.rs`, `darkmatter/lib/tests/l1/style_layers.rs`, `darkmatter/cli/tests/l1/render_style_acceptance.rs`, `darkmatter/docs/cli/data-contracts.md`, `darkmatter/docs/cli/render.md`, `darkmatter/docs/topics/layouts-and-style.md`
- **final verification:** `just test` 15243/15243 passed (all findings); `just lint` clean; `just check-tier-coverage darkmatter` zero stranded; focused filters per finding are recorded in each finding's sub-bullets above
