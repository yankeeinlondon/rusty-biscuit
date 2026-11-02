---
implementation_1: "2026-10-30T15:34:21-07:00"
implementation_2: "2026-10-30T16:32:17-07:00"
---

# Implementation Log: Extract SimplifiedSchema

## Implementation of Review Findings #1

> **started at:** 2026-10-30T15:34:21-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-28-extract-simplified-schema/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- review `implemented` flag confirmed `false` before repairs began
- the author ruled on Human item 1 (permit `globset` in the minimal build); it adds row R1-H1 below and unblocks R1-F2

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-H1 | (high) carry out ruling on item 1: permit `globset` as passive syntax dependency; amend spec R2/AC14, graph checks, feature docs | fixed | spec R2 + AC14 amended (dated, attributed to ruling); `biscuit-file/docs/dependencies.md`, `simplified-schema/docs/topics/features.md` say `globset` is passive; minimal graph = only `globset` of the resolver set; no guard asserted its exclusion |
| R1-F1 | (high) Root-union validation drops raw JSON Schema assertions | fixed | all children R1-F1.1–F1.20 fixed or clean; simplified-schema `just test` 1240 + 1099 passed; darkmatter `just test` 12771 passed |
| R1-F1.1 | Content factory strict: root `required` beside `anyOf` rejects missing `name` | fixed | `root_union_assertions::root_required_beside_any_of_is_enforced_by_the_content_factory_strictly` (red before fix) |
| R1-F1.2 | Content factory soft fast path rejects the same | fixed | `…content_factory_softly` (red before fix) |
| R1-F1.3 | JSON file factory, both modes reject | fixed | `root_union_assertions::files::root_required_beside_any_of_is_enforced_by_the_file_factories` (`try_from_json_schema`, both modes) |
| R1-F1.4 | YAML file factory raw-schema classification, both modes reject | fixed | same test, `try_from_yaml` on `schema.yaml`, both modes |
| R1-F1.5 | Async `resolve`, JSON and YAML loaders, both modes reject | fixed | same test, `resolve` on `.json` and `.yaml`, both modes |
| R1-F1.6 | Public `Prepared` validation, both modes reject | fixed | `…enforced_by_prepared` (`Prepared::new(Arc<Value>)`, both modes) |
| R1-F1.7 | Darkmatter effective-schema normal validation reports missing-required | fixed | darkmatter `raw_root_union_assertions::effective_schema_validation_keeps_root_required_beside_any_of` (red before fix) |
| R1-F1.8 | Darkmatter effective-schema raw validation reports missing-required | fixed | darkmatter `…effective_schema_raw_validation_keeps_root_required_beside_any_of` (`validate_raw`) |
| R1-F1.9 | `anyOf: [false, false]` rejects every value | fixed | `root_union_of_false_arms_rejects_every_value` (4 values, both modes `InvalidData`) |
| R1-F1.10 | `anyOf: [7, {...}]` and `[7]` give typed preparation failure | fixed | `root_union_with_a_non_schema_arm_is_a_preparation_failure` (`[7, {...}]` and `[7]` → `Unprepared(BuildValidator)`) |
| R1-F1.11 | `anyOf: []` gives typed preparation failure | fixed | `empty_root_union_is_a_preparation_failure` → `Unprepared(BuildValidator)` |
| R1-F1.12 | Control: object without `anyOf` keeps `/required` problem | clean | `root_required_without_any_of_reports_the_required_keyword` passes before and after |
| R1-F1.13 | Control: arm-level `required` rejected | clean | `arm_level_required_is_enforced` passes before and after |
| R1-F1.14 | Control: generated SimplifiedSchema root unions keep acceptance/diagnostics | clean | `generated_root_union_keeps_its_arm_diagnostics` + existing union/discriminant/golden/coercion suites pass; generated root has only `$schema`/`anyOf`/`$defs`, so the helper returns the arm unchanged |
| R1-F1.15 | Other `wrap_arm_as_root_schema` callers (Darkmatter rewrite/selection, coercion fallback) audited | fixed | prepared `accepts`/`collect_problems`, `coerce.rs` fallback, darkmatter effective build, phase projection fixed via shared helper; `validator()`, `property_wise`, `collect_applicable_root_schema_fragments`, `schema_declares_property`, `bound_operands`, scope `root_members` clean (each reads root siblings or folds every applicator); residuals split into F1.16–F1.20 |
| R1-F1.16 | `rewrite.rs` eager-file rewrite walks root `properties` beside `anyOf` | fixed | `eager_file_declared_beside_root_any_of_is_normalized` (was `"./a.md"`, expected `"a.md"`) |
| R1-F1.17 | `schema_validation.rs` per-arm caller-input conversion applies root sibling conversions | fixed | `caller_file_resolves_from_its_origin_when_root_properties_convert_beside_any_of` (was `SchemaValidationFailed`) |
| R1-F1.18 | `clean.rs` `schemas_along` sees root-declared properties under a root union | fixed | `clean_quotes_a_number_under_a_string_declared_beside_root_any_of` (red before fix) |
| R1-F1.19 | problem description for a root-declared property under a root union | fixed | `root_property_beside_any_of_keeps_its_description` (was `None`) |
| R1-F1.20 | problem description for a nested node's `properties` beside its own `anyOf` (`push_union_arms`) | fixed | `nested_property_beside_any_of_keeps_its_description` (was `None`) |
| R1-F2 | (high) Minimal dependency graph retains explicitly excluded globset | fixed | carried out per ruling item 1 (R1-H1); children verified |
| R1-F2.1 | Library minimal build graph: `globset` permitted by ruling; `gix`/`walkdir` absent | fixed | `cargo tree -p simplified-schema --no-default-features -e normal --prefix none` → `globset v0.4.18` only; no `gix`/`walkdir` (permitted by the amended R2/AC14) |
| R1-F2.2 | biscuit-file syntax feature boundary consistent with ruling; docs say passive | fixed | `biscuit-file` `file-reference-syntax = ["dep:globset", "url"]` unchanged per ruling; feature docs now call it passive syntax |
| R1-F2.3 | Clean: minimal graph has no `gix`, `walkdir`, `sniff` | clean | minimal graph: no `gix`, `walkdir`, `sniff` |
| R1-F2.4 | Clean: default graph carries resolver deps | clean | default graph holds `gix`, `walkdir`, `globset`, `sniff` |
| R1-F3 | (high) Default dependency graph includes the HTTP effect stack | fixed | children verified; sniff `just test` 3523 passed, `just lint` exit 0 |
| R1-F3.1 | Default schema-only graph has no `reqwest`/`hyper`/`tokio` | fixed | default graph has no `reqwest`/`hyper`/`tokio`/`schematic-schema` (`cargo tree … -i reqwest` → no match) |
| R1-F3.2 | `sniff` repository-only use avoids `schematic-schema`; remote feature still builds | fixed | `schematic-schema` optional, enabled by `remote`; `cargo tree -p sniff -e normal` without it; `--features remote` nextest 198 + 107 passed; `network`-only dead-code warnings gated; worktree-cli declares `remote` |
| R1-F3.3 | Clean: minimal graph has no sniff/reqwest/hyper/tokio | clean | minimal graph: no sniff/reqwest/hyper/tokio |
| R1-F3.4 | Clean: no Darkmatter/DMLS/Claudine/Content Policy runtime/biscuit-terminal in default or minimal | clean | neither graph holds darkmatter, dmls, claudine, content-policy, biscuit-terminal |
| R1-F4 | (high) Shipped model-config documents fail their schema contract | fixed | children verified; claudine `just test` 11807 passed, `just lint` exit 0 |
| R1-F4.1–F4.7 | Seven `model-config/*.md` documents: scalar `notes` wrapped as one-item lists | fixed | 127 scalars wrapped across 10 documents (the review's 7 plus antigravity, kilo, pi); `every_shipped_model_config_document_satisfies_its_sidecar` |
| R1-F4.8 | Three record validators still reject scalar notes (no weakening) | fixed | sidecar unchanged; `a_scalar_notes_is_rejected_in_every_record_type` (one problem at `/<record>/0/notes`) |
| R1-F4.9 | Four `cli_switches` generator tests pass | fixed | the four `cli_switches` tests pass after the data fix and the `@this` gate fix in `schema_compat.rs`; claudine-gen l1 146/146 |
| R1-F4.10 | Array-form control passes | fixed | all ten documents validate; 22-folder sweep 205/205 |
| R1-F5 | (high) Schema factories lose typed grammar errors and document locations | fixed | children verified; simplified-schema, darkmatter (incl. dmls, darkmatter-cli) full `just test` pass |
| R1-F5.1 | Content YAML root `$schema`: typed grammar cause + supplied-text span (LF/CRLF) | fixed | `located_schema_errors::content_yaml_root_payload_failure_is_located_in_the_supplied_text` (LF + CRLF, `Grammar` cause, exact span) |
| R1-F5.2 | YAML file factory root: file identity + typed located cause | fixed | `located_schema_errors::files::yaml_file_factory_locates_root_and_types_failures` (root) |
| R1-F5.3 | Content YAML `types` table: typed cause + nested span | fixed | `content_yaml_types_table_failure_is_located_in_the_supplied_text` |
| R1-F5.4 | YAML file factory `types` table | fixed | `files::yaml_file_factory_locates_root_and_types_failures` (types) |
| R1-F5.5 | Content Markdown: document offset (CRLF) | fixed | `content_markdown_failure_is_located_in_the_whole_document`; `markdown_frontmatter_yaml_failure_points_into_the_whole_document` |
| R1-F5.6 | Markdown file factory: file identity + document span | fixed | `files::markdown_file_factory_reports_the_file_and_document_span` |
| R1-F5.7 | Imported schema reader: typed cause + referenced-file location | fixed | `files::imported_named_type_failure_is_located_in_the_imported_file`; async `resolve` delegates: `files::async_resolve_reports_the_same_located_failure` |
| R1-F5.8 | Clean: direct type-string parser keeps substring-relative span | clean | `direct_definition_parser_keeps_its_type_string_span` (`11..11`); the API accepts a type string, so a substring span is its contract |
| R1-F5.9 | Clean: raw JSON loader keeps line/column | clean | existing `raw_json_schema_keyword_matrix` and `json_schema_files_go_through_the_resolvers_json_loader` keep line/column; pass |

### Progress

- starting the work on 'R1-H1 carry out ruling on item 1' at 15:35:05
        - spec R2 and acceptance criterion 14 amended in place, each marked "amended 2026-10-30 by the author's ruling on review 1, Human item 1"; decision 17 names no excluded crate and needed no change
        - `biscuit-file/docs/dependencies.md` and `simplified-schema/docs/topics/features.md` now state that `globset` is a passive syntax dependency, not a resolver dependency
        - no source or Cargo-graph guard asserted a `globset` exclusion: `simplified-schema/justfile`'s `test-minimal` comment and `biscuit-file/lib/tests/l1/reference_syntax_only.rs` already name only `gix`/`walkdir`
        - `cargo tree -p simplified-schema --no-default-features -e normal --prefix none` → of gix/walkdir/globset/sniff/reqwest/hyper/tokio/darkmatter/biscuit-terminal only `globset v0.4.18` is present
        - departure recorded as author ruling: the minimal build keeps `globset` (passive `match(...)` glob syntax validation); it is not described as satisfying the unamended criterion
- work completed for 'R1-H1 carry out ruling on item 1' at 15:36:15
- R1-F1, R1-F3, R1-F4 dispatched at 15:36:00 to three parallel subagents (disjoint files: simplified-schema/darkmatter validation; sniff Cargo features; claudine research data); R1-F5 is held until R1-F1 lands because both edit `simplified-schema/lib/src`
- R1-F3 subagent reported at 15:49:30
        - defect class: an unrelated unconditional transitive dependency defeats a package's standalone dependency boundary
        - the only path simplified-schema → reqwest/hyper/tokio was sniff → `schematic-schema` → `schematic-runtime`; `schematic-schema` is now optional and enabled by sniff's `remote` feature (`remote = ["network", "dep:schematic-schema"]`); every `schematic_schema` use sits in `sniff/lib/src/remote/`, already `#[cfg(feature = "remote")]`
        - 29 sniff consumers grepped; darkmatter, dmls, and worktree already enable `remote`; worktree-cli uses `sniff::remote` but got the feature only through worktree
        - `sniff/docs/dependencies.md` drift corrected (it called `remote` an alias of `network` and the HTTP stack unconditional); `simplified-schema/docs/dependencies.md` corrected; sniff skill notes HTTP deps stay behind `network`/`remote`
        - evidence: `cargo tree -p simplified-schema -e normal -i reqwest` → no match; sniff `--features remote` nextest 198 + 107 passed; sniff no-feature 75 passed; clippy with and without `remote` clean; `cargo check` sniff-cli, rendezvous-core, tree-hugger-cli, worktree-cli pass
- R1-F1 subagent reported at 15:50:00
        - defect class: projecting a schema to its alternatives changed its contract — sibling assertions beside `anyOf` dropped, non-object arms (`false`, `7`) turned into `{}`, and a malformed `anyOf` (empty, non-schema arm) accepted instead of failing preparation
        - repair at the one shared helper: `wrap_arm_as_root_schema` (`simplified-schema/lib/src/validate.rs`) keeps every root keyword and moves the arm under `allOf` whenever the root carries an assertion beside `anyOf` or the arm is not an object; generated SimplifiedSchema unions (root only `$schema`/`anyOf`/`$defs`) keep identical arms and schema paths
        - `wrapped_root_arms` (`prepared.rs`) returns `None` for `anyOf: []` or a non-object/non-boolean arm, so the whole root compiles and `jsonschema` rejects it as `Unprepared(BuildValidator)`
        - sibling callers audited: `prepared.rs` `accepts`/`collect_problems`, `coerce.rs` pending-value fallback, Darkmatter effective-schema build, phase projection, `validator()`, `rewrite.rs` arm selection, `compose/schema_validation.rs` per-arm caller-input check — all fixed or clean through the helper
        - two residual instances of the same class reported by the subagent (eager-file rewrite walks only the arm; per-arm caller-input conversion uses arm + `$defs` only) — not accepted as "left as they were"; dispatched to a follow-up subagent as R1-F1.16 and R1-F1.17
        - tests: `simplified-schema/lib/tests/l1/root_union_assertions.rs` (12) and `darkmatter/lib/tests/l1/raw_root_union_assertions.rs` (2); with the fix reverted 8/12 and 2/2 fail (the 4 passing are controls)
        - evidence: simplified-schema filtered nextest 217 passed (default+features), 201 passed (`--no-default-features`); darkmatter filtered nextest 346 passed; `just lint` clean in both areas
        - docs: `simplified-schema/docs/topics/language/schema-documents.md` now explains root `anyOf` validation of raw JSON Schema
- R1-F4 subagent reported at 16:05:00
        - defect class: shipped data uses a single value where the schema that reads it requires a list (the sidecar changed `notes: string` → `string[]` without migrating its documents)
        - the review named 7 documents; the sweep found 10 — `antigravity.md`, `kilo.md`, `pi.md` had the same scalar `notes`; 127 scalars wrapped as one-item lists in total (claude 11, codex 11, gemini 11, goose 13, kimi 12, opencode 16, qwen 15, antigravity 6, kilo 19, pi 13); frontmatter parsed before/after shows only the wraps
        - second, hidden defect: once the data validated, the four generator tests failed in `claudine/gen/src/schema_compat.rs` with `SchemaIncompatible { field: "offering_sources", path: "local_runners", found: "runner[]@this" }`; the gate expanded only `./`-imported named types, not types declared in the sidecar's own `types:`; fixed (`sidecar_types`), an undeclared name is `SidecarInvalid`
        - `catalog.json` regenerated (`claudine-gen generate --yes`; JSON diff = exactly the 127 wraps) and its pinned hash in `generated-artifact-baseline.json` updated; the research prompt `_research.md` now says `notes` is a list
        - sibling sweep: every non-underscore document in all 22 research folders with a `_schema.yaml` validated through `load_validated_frontmatter` — 195/195 others passed before, 205/205 after
        - `last_updated` left unchanged (research date; no `content_policy`)
        - tests: `claudine/gen/tests/l1/model_config_research.rs` (all ten documents validate; scalar `notes` rejected in each of the three record types; the gate inlines `@this` record types)
        - evidence: `cargo nextest run -p claudine-gen --test l1` 146/146; claudine shipped corpus 67/67; claudine-cli research sequence 7/7; `claudine-gen check` clean; `just lint` (claudine) exit 0
- starting the work on 'R1-F5' at 15:50:30
        - defect class: a loading adapter erases typed diagnostic provenance, or exposes a local substring offset as the only available location
        - `SchemaError::SchemaDocument` extended with `source: Option<Box<SchemaError>>` (the original typed cause) and `span: Option<Range<usize>>` (bytes in the exact supplied text, LF or CRLF; for Markdown the whole document including fences); `message`/`Display` unchanged, so `md schema` and DMLS output are unchanged
        - shared locating helper (`parse_keyed_declaration`/`locate_keyed_failure` in `simplified/source.rs`) serves root payload, `types` table, Markdown frontmatter, and the existing `locate_standalone_definition_failure`; locating runs only after a failure, nothing is reread
        - siblings fixed: the standalone lint failure keeps its typed cause; frontmatter "not valid YAML" gets a whole-document span; imported named-type files (`load_named_types`) are located in the imported file
        - siblings clean: third-party UTF-8/YAML/JSON wrappers already carry whole-text line/column; embedded-import closures wrap reason strings, not `SchemaError`s
        - departure: the facade's Markdown grammar failure is now `Schema(SchemaDocument { source: Grammar, .. })` instead of a bare `Grammar` (no Darkmatter production code used that path); recorded as a human review item
        - tests: `simplified-schema/lib/tests/l1/located_schema_errors.rs` (10), asserting file path, the `Grammar` cause (`property == "age"`), and `&text[span] == "'number(min('"` for LF and CRLF
        - evidence: simplified-schema whole crate 1239/1239 (`work-counters,terminal`), 1098/1098 (`--no-default-features`), doctests 29 passed; darkmatter filtered 358/358; dmls filtered 525/525; darkmatter-cli `test(schema)` 421/421; `cargo check` darkmatter, darkmatter-cli, dmls, claudine, claudine-cli, claudine-gen, messenger `--all-targets` clean; `just lint` simplified-schema and darkmatter exit 0
        - docs: `simplified-schema/docs/topics/diagnostics.md` (new section; corrected example that expected a bare `Grammar`), `schema-object.md`, `language/schema-documents.md`; skills `simplified-schema/SKILL.md` and `darkmatter/schema.md` (drifted "folded into the message" text corrected)
- work completed for 'R1-F5' at 16:05:10
- R1-F1 follow-up subagent (R1-F1.16–R1-F1.19) reported at 16:05:20
        - R1-F1.16 fixed: `rewrite.rs` `rewrite_root_union` rewrites the root's own `properties` beside `anyOf` before each accepting arm (eager file beside root `anyOf` was not normalized: `"./a.md"` instead of `"a.md"`)
        - R1-F1.17 fixed: `schema_validation.rs` `root_schema_arm_applies` converts against the arm, then the root's sibling keywords through the new shared `simplified_schema::validate::root_union_sibling_schema` (one keyword list, `has_root_union_sibling_assertions`); before, a caller file lost its origin and failed with `SchemaValidationFailed`
        - R1-F1.18 (found by sweep) fixed: `clean.rs` `schemas_along` starts from the expanded arm plus the root node, so `md clean` sees root-declared properties
        - R1-F1.19 (found by sweep) fixed: `validate.rs` problem descriptions for a root-declared property under a root union
        - sweep verdicts recorded in the ledger (R1-F1.15); generated unions unaffected (root has no `properties` or assertions)
        - tests (each red before its fix): `eager_file_declared_beside_root_any_of_is_normalized`, `caller_file_resolves_from_its_origin_when_root_properties_convert_beside_any_of`, `clean_quotes_a_number_under_a_string_declared_beside_root_any_of`, `root_property_beside_any_of_keeps_its_description`
        - evidence: darkmatter filtered 1062/1062; simplified-schema filtered 96/96; `just lint` darkmatter and simplified-schema exit 0
        - docs: `darkmatter/docs/inline/schema-validation.md` paragraph + example; `schema-documents.md` description sentence
- orchestrator follow-ups at 15:58–16:06
        - R1-F1.20: the subagent named a nested (non-root) node with `properties` beside its own `anyOf` losing descriptions in `push_union_arms`; same class, not accepted as out of scope. Reproduced red with `nested_property_beside_any_of_keeps_its_description` (description `None`), fixed by pushing a union node that declares `properties`/`items` beside its `anyOf`; doc comment updated; filtered run 97/97
        - R1-F3 follow-on: sniff `--features network` without `remote` warned that `credentials::provider_token{,_variables}` were dead code (only `remote` uses them); both now `#[cfg(feature = "remote")]`; `cargo check -p sniff` with no features, `network`, `remote` → no warnings
        - R1-F3 follow-on: `worktree-cli` uses `sniff::remote` but relied on `worktree` enabling the feature; its sniff dependency now declares `features = ["remote"]`; `cargo check -p worktree-cli` passes, `Cargo.lock` unchanged
- full gates (`just test`, `just lint` in simplified-schema, sniff, claudine, darkmatter) started at 16:06:40, run serially with no subagent active
- full gates finished at 16:20:00 with no subagent running
        - simplified-schema `just test`: 1240 passed (default + `work-counters,terminal`), 1099 passed (`--no-default-features`); `just lint` exit 0
        - sniff `just test`: 3523 passed, 33 skipped; `just lint` exit 0
        - claudine `just test`: 11807 passed, 15 skipped; `just lint` exit 0
        - darkmatter `just test`: 12771 passed, 10 skipped (darkmatter, darkmatter-cli, dmls); `just lint` exit 0 (incl. zed-dmls wasm32-wasip2)
        - `just check-tier-coverage` simplified-schema, claudine, darkmatter, sniff: 0 stranded
        - `just cross-check simplified-schema --os windows`: 1241/1241 passed (paths and CRLF spans in the new located-error tests)

### Completion Report

- **outcome:** complete
- **elapsed:** 15:34:21 → 16:25:40 (about 51 minutes)
- **parent findings:** 5 total = 5 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed
        - ruling row R1-H1 (author ruling on Human item 1) carried out; it closed R1-F2
- **instance rows:** 47 closed (39 fixed, 8 clean), none open; sweeps added R1-F1.16–R1-F1.20, three extra model-config documents, and the claudine-gen `@this` gate defect
- **verified repairs**
        - R1-F1: raw root assertions beside `anyOf` now apply to every arm in every consumer (library factories, `Prepared`, `resolve`, Darkmatter effective/raw validation, eager rewrite, caller conversion, `md clean`, problem descriptions); malformed `anyOf` is a typed preparation failure; `false` arms reject
        - R1-F2: the minimal graph keeps only `globset` (passive glob syntax), per the amended spec
        - R1-F3: the default graph no longer holds the HTTP stack; sniff's `schematic-schema` sits behind `remote`
        - R1-F4: all ten model-config documents satisfy their sidecar; the generator gate expands sidecar-local record types; the four generator tests pass
        - R1-F5: schema-document failures keep their typed cause, file identity, and a byte span into the exact supplied text (LF/CRLF, whole Markdown document, imported files)
- **open findings:** none
- **performance:** no measurement required by these findings; `deferred_perf_measurement` not set
- **changed files**
        - R1-H1/R1-F2: `darkmatter/features/2026-10-28-extract-simplified-schema/spec.md`, `biscuit-file/docs/dependencies.md`, `simplified-schema/docs/topics/features.md`
        - R1-F1: `simplified-schema/lib/src/{validate.rs,prepared.rs}`, `darkmatter/lib/src/markdown/schemas/{rewrite.rs,clean.rs}`, `darkmatter/lib/src/markdown/compose/schema_validation.rs`, `simplified-schema/lib/tests/l1/{root_union_assertions.rs,main.rs}`, `darkmatter/lib/tests/l1/{raw_root_union_assertions.rs,main.rs}`, `simplified-schema/docs/topics/language/schema-documents.md`, `darkmatter/docs/inline/schema-validation.md`
        - R1-F3: `sniff/lib/Cargo.toml`, `sniff/lib/src/credentials.rs`, `worktree/cli/Cargo.toml`, `sniff/docs/dependencies.md`, `simplified-schema/docs/dependencies.md`, `.claude/skills/sniff/SKILL.md`
        - R1-F4: `claudine/docs/research/model-config/{antigravity,claude,codex,gemini,goose,kilo,kimi,opencode,pi,qwen,_research}.md`, `claudine/gen/src/schema_compat.rs`, `claudine/docs/providers/catalog.json`, `claudine/gen/tests/fixtures/generated-artifact-baseline.json`, `claudine/gen/tests/l1/{model_config_research.rs,main.rs}`, `.claude/skills/claudine/research-contracts.md`
        - R1-F5: `simplified-schema/lib/src/{errors.rs,resolve.rs,simplified/source.rs,simplified/standalone.rs,simplified/mod.rs,schema/layer.rs}`, `darkmatter/dmls/src/overlay/mod.rs`, `darkmatter/lib/src/markdown/schemas/triggers/assemble.rs`, `darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs`, darkmatter tests `schemas_alphanumeric_check_cache.rs`, `schemas_named_import_errors.rs`, `schemas_named_type_memo.rs`, `simplified-schema/lib/tests/l1/{located_schema_errors.rs,schema_object.rs,main.rs}`, `simplified-schema/docs/topics/{diagnostics.md,schema-object.md}`, `.claude/skills/simplified-schema/SKILL.md`, `.claude/skills/darkmatter/schema.md`
- **decisions for the author** (also in the review's `human_review_items`): root assertions merged into each arm under `allOf` only when needed; malformed `anyOf` via `Unprepared(BuildValidator)`; caller conversion runs arm then root siblings; `SchemaDocument` extended rather than a new variant; facade Markdown grammar failure now wraps `Grammar`; `span` falls back to the whole value and successful layers keep no source maps; `schematic-schema` gated on `remote`; ten documents repaired plus the gate fix

## Implementation of Review Findings #2

> **started at:** 2026-10-30T16:32:17-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-28-extract-simplified-schema/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- review `implemented` flag confirmed `false` before repairs began
- the review carries one parent finding (R2-F1, high) with 19 child rows; it reports `recurrence: true` (sibling of R1-F1); no human-in-the-loop items and no blockers

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | (high) Root-union projection loses evaluation results from other accepting arms | fixed | all children fixed or clean; repair at shared `wrap_arm_as_root_schema`; simplified-schema `just test` 1252 + 1110 passed; darkmatter `just test` 12776 passed; claudine-gen L1 146 passed; both lints exit 0 |
| R2-F1.1 | Library content factory strict: `anyOf` name/age + `unevaluatedProperties: false`, `{name, age}` accepted | fixed | `simplified-schema` L1 `root_union_assertions::fields_of_two_accepting_arms_pass_unevaluated_properties_in_the_content_factory_strictly` (red before fix; also asserts complete validator accepts) |
| R2-F1.2 | Library content factory soft: same, accepted and borrowed | fixed | `simplified-schema` L1 `root_union_assertions::…_content_factory_softly` (accepted, input borrowed; red before fix) |
| R2-F1.3 | Public `Prepared` strict/soft: same, both accept | fixed | `simplified-schema` L1 `root_union_assertions::…_in_prepared` (strict and soft; red before fix) |
| R2-F1.4 | JSON file factory strict/soft: both accept | fixed | `simplified-schema` L1 `root_union_assertions::files::fields_of_two_accepting_arms_pass_unevaluated_properties_in_the_file_factories` (`try_from_json_schema`, both modes) |
| R2-F1.5 | YAML file factory strict/soft: both accept | fixed | same test, `try_from_yaml` on `schema.yaml`, both modes |
| R2-F1.6 | Async `resolve`, JSON and YAML, strict/soft: all accept | fixed | same test, async `resolve` on `.json` and `.yaml`, both modes |
| R2-F1.7 | Darkmatter effective-schema normal validation: valid | fixed | darkmatter L1 `raw_root_union_assertions::effective_schema_validation_accepts_fields_of_two_accepting_arms_beside_unevaluated_properties` (also asserts `extra` still rejected; red before fix) |
| R2-F1.8 | Darkmatter effective-schema raw validation: valid | fixed | darkmatter L1 `raw_root_union_assertions::effective_schema_raw_validation_accepts_fields_of_two_accepting_arms_beside_unevaluated_properties` (red before fix) |
| R2-F1.9 | Darkmatter document validation (Markdown referencing schema): valid frontmatter | fixed | darkmatter L1 `raw_root_union_assertions::document_with_fields_of_two_accepting_arms_beside_unevaluated_properties_is_valid` (red before fix) |
| R2-F1.10 | Darkmatter eager-file normalization / accepting-arm selection: `doc: ./a.md` normalizes to `a.md` | fixed | darkmatter L1 `raw_root_union_assertions::eager_file_beside_a_union_restricting_unevaluated_properties_is_normalized` (`./a.md` → `a.md`; red before fix) |
| R2-F1.11 | Darkmatter caller-input composition / arm applicability: composes, keeps caller file origin | fixed | darkmatter L1 `raw_root_union_assertions::caller_file_beside_a_union_restricting_unevaluated_properties_composes_from_its_origin` (caller origin retained; red before fix) |
| R2-F1.12 | Library strict/soft, `contains` alternatives + `unevaluatedItems: false`, `["Ada",36]` accepted | fixed | `simplified-schema` L1 `root_union_assertions::items_of_two_accepting_arms_pass_unevaluated_items` (strict and soft) |
| R2-F1.13 | Library strict, `unevaluatedProperties: {type: boolean}`: accepted | fixed | `simplified-schema` L1 `root_union_assertions::schema_valued_unevaluated_properties_sees_fields_of_two_accepting_arms` (plus `flag: true` accepted, `flag: "yes"` rejected) |
| R2-F1.14 | Controls: single alternative (object and array) accepted | clean | `simplified-schema` L1 `root_union_assertions::a_single_accepting_arm_passes_unevaluated_restrictions` (object and array controls accept, before and after) |
| R2-F1.15 | Control: `{name, extra}` under false restriction rejected | clean | `simplified-schema` L1 `root_union_assertions::a_field_no_accepting_arm_evaluates_is_still_rejected` (strict/soft reject, problems name `extra`) |
| R2-F1.16 | Controls: nested property union and root `allOf` holder accept | clean | `simplified-schema` L1 `root_union_assertions::a_union_with_unevaluated_properties_under_a_property_or_root_all_of_accepts_both_fields`; nested/`allOf` holders compile the complete `anyOf`, and 2020-12 `unevaluated*` there sees only its own subschema |
| R2-F1.17 | Controls: generated SimplifiedSchema root unions (discriminant, conversion, arm diagnostics) unchanged | clean | generated-union branch of the wrapper is unchanged byte for byte; `generated_root_union_keeps_its_arm_diagnostics` and coerce/union/prepared filters (105) pass; full library suites pass |
| R2-F1.18 | Coercion commit gate: complete-schema acceptance precedes arm fallback; fallback uses corrected wrapper | fixed | complete-schema acceptance: soft F1.2/F1.3 cases borrow the input; pending fallback: unit test `coerce::tests::root_union_arm_fallback_sees_every_accepting_arm_beside_a_pending_key` (red before fix) calls `accepts` with the corrected wrapper |
| R2-F1.19 | Cleanup and problem-description projections: no independent evaluation-loss defect | clean | `resolve_problem_description` and the description/cleanup walks read the unmodified root `anyOf[arm_index]`, not the wrapper, and decide no validity; existing description tests in `root_union_assertions` pass |

### Progress

- starting the work on 'R2-F1: Root-union projection loses evaluation results from other accepting arms' at 16:36:10
        - defect class: splitting a root union into independently compiled alternatives drops evaluation context (evaluated properties/items) that a sibling assertion of the complete root reads across all successful alternatives, so the per-alternative verdict disagrees with the complete schema
        - shared construction point `simplified_schema::validate::wrap_arm_as_root_schema`; consumers swept: `prepared::wrapped_root_arms` → `CompiledValidators::{accepts, collect_problems}`, `coerce::accepts` pending/template fallback, darkmatter `schemas/rewrite.rs` `root_arm` (eager-file arm selection), darkmatter `compose/schema_validation.rs` `root_schema_arm_applies` (caller-input), plus `root_union_sibling_schema` conversion users
        - delegated to one foreground subagent (serial; single finding)
        - design: for a raw root that declares `unevaluatedProperties` or `unevaluatedItems` (new `ROOT_UNION_EVALUATION_READERS`), each wrapped arm keeps the original `anyOf` with a `true` alternative appended, and still holds the selected arm under `allOf`; the retained union never fails, but it supplies every accepting alternative's evaluated fields/items, so "some arm accepts" ⇔ "complete root accepts"
                - alternatives rejected: retaining `anyOf` without `true` (adds a keyless root `anyOf` error to each arm report, breaking `only_set_aside_problems` and Darkmatter's pending-key arm check); judging validity with the complete root and using arms only for reporting (more sites, Darkmatter's rewrite/caller-input checks build their own validators)
                - roots without those keywords (all R1-F1 cases) and generated SimplifiedSchema unions get the identical wrapper as before
        - all consumers go through the one helper: `prepared::wrapped_root_arms` → `CompiledValidators::{accepts, collect_problems}`, the `coerce::accepts` fallback, darkmatter `schemas/rewrite.rs` `root_arm`, darkmatter `compose/schema_validation.rs` `root_schema_arm_applies`; no other callers found
        - added 11 library L1 tests, 1 `validate` unit test, 1 `coerce` unit test, 5 darkmatter L1 tests; 7 library L1, the coerce unit test, and 5 darkmatter tests were red before the fix
        - discovery: conversion through `root_union_sibling_schema` with `unevaluatedProperties: false` among siblings still commits via the property-wise commit (proven by R2-F1.11); left unchanged
        - discovery (existing behavior, unchanged by this repair): with any assertion beside a raw root union and pending keys, the conversion core's whole-schema check fails on the pending value before `coerce::accepts` runs, so R2-F1.18's pending case is a unit test on `accepts`
        - discovery: `just test-minimal <filter>` ignores its filter and runs the full minimal suite
        - the subagent ran three diagnostic `--no-fail-fast` runs on narrow filters with the fix disabled to prove red-before; final verification did not use it
        - docs updated: `simplified-schema/docs/topics/language/schema-documents.md` (raw-root `unevaluated*` paragraph and example), `darkmatter/docs/inline/schema-validation.md`, `.claude/skills/darkmatter/schema.md`, and the `///` docs of `wrap_arm_as_root_schema` and `CompiledValidators::RootUnion`
- work completed for 'R2-F1: Root-union projection loses evaluation results from other accepting arms' at 16:49:00
- orchestrator reconciliation at 16:52:00
        - inspected the `wrap_arm_as_root_schema` diff and checked every ledger row's evidence against the listed tests (`cargo nextest list` selects the 11 library and 5 darkmatter tests)
        - 2020-12 sibling check: `unevaluated*` inside a root `allOf` member or a `$ref` target sees only its own subschema's evaluations, so the root-level keywords are the only readers of the root `anyOf`'s results
        - cross-OS check skipped: the repair is pure in-memory JSON Schema construction with no path, process, or OS-specific code; the new file-factory tests use temp dirs in the same way as the existing R1-F1 file tests

### Completion Report

- **outcome:** complete; elapsed about 20 minutes (16:32:17 → 16:52)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - child rows: 19 = fixed 14 (F1.1–F1.13, F1.18) + clean 5 (F1.14–F1.17, F1.19)
- **open findings:** none
- **changed files (this iteration)**
        - `simplified-schema/lib/src/validate.rs`: `ROOT_UNION_EVALUATION_READERS`, `wrap_arm_as_root_schema` and its docs, unit test (R2-F1)
        - `simplified-schema/lib/src/prepared.rs`: `CompiledValidators::RootUnion` doc (R2-F1 docs drift)
        - `simplified-schema/lib/src/coerce.rs`: unit test (R2-F1.18)
        - `simplified-schema/lib/tests/l1/root_union_assertions.rs`: 11 tests (R2-F1.1–F1.6, F1.12–F1.16)
        - `darkmatter/lib/tests/l1/raw_root_union_assertions.rs`: 5 tests (R2-F1.7–F1.11)
        - `simplified-schema/docs/topics/language/schema-documents.md`, `darkmatter/docs/inline/schema-validation.md`, `.claude/skills/darkmatter/schema.md`: repaired behavior described
- **final verification**
        - `just test` in `simplified-schema/`: 1252 passed (`work-counters,terminal`) and 1110 passed (no default features) (R2-F1 library rows)
        - `just test` in `darkmatter/`: 12776 passed, 10 skipped (R2-F1.7–F1.11 and host regressions)
        - `cargo nextest run -p claudine-gen --test l1`: 146 passed (consumer regression)
        - `just lint` in `simplified-schema/` and `darkmatter/`: exit 0
        - `just check-tier-coverage simplified-schema` and `darkmatter`: 0 stranded
