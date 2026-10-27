---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-pipeline-finalization/darkmatter/fixes/2026-10-25-host-function-catalog-leak/spec.md"
plan: "darkmatter/fixes/2026-10-25-host-function-catalog-leak/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
human_review: false
message_to_agent: |-
    Phase 1 changed no source. Read the log's "Consumer inventory" before editing: there are 12
    `Described` implementations (8 in expression/semantics.rs), not 5. Claudine reads the static
    `.example` field directly in cli/src/commands/context/{expressions,effects,tests}.rs and its
    format.rs helpers take `Option<&Example>`; Phase 1 decided they take `Option<Example<'_>>` via
    the trait method. `aliases` is not leaked today (it comes from the static binding table and is
    `&[]` for host functions); Decision 1 still makes it owned. Test code that copies descriptors:
    lib/tests/l1/lifecycle_schema_generation.rs (`*descriptors...find`, `.copied()` at 201/219).
    Pre-refactor output baselines are in this fix directory's `baselines/` (md schema about,
    claudine context --expressions/--side-effects, both generator examples, and DMLS function
    hover/completion); diff post-refactor output against them, and Phase 3 should add exact-text
    tests for the two unpinned outputs (md schema about Expression Functions section, DMLS
    function hover/completion). The facts-matrix gaps are listed under "Facts-artifact matrix audit".
---

# Implementation Log for 2026-10-25-host-function-catalog-leak (4 phases)

## Phase 1

Phase 1 is the plan's Wave 1 (contract inventory). No source file changed. The
inventory below is the work list for Phase 2 (ownership migration) and Phase 3
(cache replacement and tests). Pre-refactor output baselines were captured
under `baselines/` in this fix directory.

### Consumer inventory (Map consumers)

**Shared framework** — `darkmatter/lib/src/catalog/mod.rs`

- `Example { invocation, result: &'static str, verification }` and
  `ExampleVerification::DisplayOnly(&'static str)`; both `Copy`.
- `Described::{key, description, category} -> &'static str`,
  `example() -> Option<&Example>`.
- Generic consumers: `describe`, `suggest`, `describe_for_error` (reads
  `example.invocation/result`), plus the module's own test descriptor
  (`TestDescriptor`, line ~176) and the `.example()` reads at lines 128 and 434.
- `suggest_strings` doc comment claims it exists because `Described::key` is
  `&'static str`; it must be reworded (plain strings vs descriptor records).

**`Described` implementations (12, not the spec's 5)**

| File | Type(s) | Example storage |
| ---- | ------- | --------------- |
| `lib/src/catalog/mod.rs` (tests) | `TestDescriptor` | `Option<Example>` field, `const UPPER_EXAMPLE` |
| `lib/src/effects/catalog.rs` | `EffectDescriptor` | `pub example: Option<Example>`; 14 static `Example { .. DisplayOnly(..) }` literals in `EFFECT_DESCRIPTORS` |
| `lib/src/markdown/compose/context/catalog.rs` | `ContextVariableDescriptor` | `pub example: Option<Example>` |
| `lib/src/markdown/compose/expression/catalog/mod.rs` | `ExpressionFunctionDescriptor` | `pub example: Option<Example>` (to be removed; derive from first `FunctionExample::Call`) |
| `lib/src/markdown/compose/expression/semantics.rs` | `OperatorDescriptor`, `TruthinessDescriptor`, `ModeDescriptor`, `NullPropagationDescriptor`, `UnaryOperatorDescriptor`, `ComparisonOperatorDescriptor`, `ArithmeticOperatorDescriptor`, `VariableAccessDescriptor` | `pub example: Option<Example>` each; `.example()` reads in its tests (lines ~877–1031) |

Static tables become `Option<Example<'static>>`; their `example()` returns the
copied view.

**Direct `.example` field reads (must switch to the trait method or a copied view)**

- `claudine/cli/src/commands/context/expressions.rs` lines 102–296: eight
  `format_example(desc.example.as_ref(), term)` over the semantics descriptors.
- `claudine/cli/src/commands/context/effects.rs` lines 83, 115 and
  `claudine/cli/src/commands/context/tests.rs` line 476:
  `format_effect_example(effect.example.as_ref(), ..)`.
- `claudine/cli/src/commands/context/format.rs` lines 92 and 151: helper
  signatures take `Option<&darkmatter::catalog::Example>`.
- No reader of `ExpressionFunctionDescriptor::example` exists outside
  `catalog/mod.rs` (`effects/catalog.rs:377` and `catalog/mod.rs:434` use the
  trait method).

**Expression catalog** — `lib/src/markdown/compose/expression/catalog/mod.rs`

- Leaks: `leak()` (categories, names, signature, description, returns,
  parameter names and type text, inline call/result/display-only text,
  file-example path/invocation/returns/description), `Box::leak` of the
  category slice, each `FunctionType`, each parameter slice, each examples
  slice, and the descriptor slice. `project_examples` takes
  `&'static FunctionType`; `file_example` leaks on success only, but earlier
  overloads' leaks survive a later `file_example` error (acceptance 4).
- `aliases: &'static [&'static str]` is **not** leaked today: it comes from
  `functions::registered_aliases`, the static binding table (`&[]` for every
  host function). Plan Decision 1 still makes it owned; noted so Phase 2 does
  not hunt for a leak there.
- `descriptors_for(&self, ..) -> &'static [..]`, `FunctionCatalog.categories`
  and `.descriptors` public `&'static` fields.
- Static APIs (keep `&'static`): `expression_function_catalog()`,
  `expression_function_descriptors()`, `function_descriptors(name)`,
  `effect_function_catalog()` / `effect_function_descriptors()`
  (`lib/src/effects/types.rs`), lifecycle `VerbSignature::{parameters, names,
  display, verb}` and `all_lifecycle_verbs()` (`lib/src/lifecycle/signatures.rs`,
  `from_catalog(verb, &'static [..])`).
- Owner-borrowed APIs (become `&self`): `FunctionCatalog::descriptors_for`,
  `HostFunctionCatalog::function_names` (`host/mod.rs:151`, `Vec<&'static str>`),
  `HostFunctionCatalog::registration_builder` (clones `function_type`; line 170).
  `HostFunctions::function_names` (line 373) already returns `Vec<&str>`.
- `generate_expression_function_table` reads `descriptor.examples/category/signature/description`.

**Other library consumers**

- `lib/src/effects/types.rs`: `catalog.descriptors` iteration, `descriptor.function_type.clone()`,
  `HashMap<&'static str, ..>` keyed on `descriptor.name` inside a `LazyLock` (static owner: fine);
  tests read `.signature/.category/.fallible/.parameters`.
- `lib/src/lifecycle/signatures.rs`: `descriptor: &'static ExpressionFunctionDescriptor`;
  test fixture leaks (out of scope per spec) and `VerbSignature::from_catalog("joined", catalog.descriptors)`.
- `lib/src/lifecycle/schema_projection.rs`: `LifecycleSchemaProjection { expression_functions:
  &'a [..], side_effects: &'a [..] }`, `descriptor.function_type` collected as `&FunctionType`
  (line 404; becomes `&descriptor.function_type`), `parameter.name.to_string()`.
- `lib/src/markdown/compose/context/capture/capabilities.rs:129`,
  `lib/src/markdown/compose/expression/mod.rs:784` (`suggest(expression_function_descriptors(), ..)`),
  `lib/src/markdown/compose/expression/functions/mod.rs:266–279` (registry build; clones
  `function_type`) and its tests (2004–2015).
- `lib/src/lib.rs` crate docs name the accessors (no lifetime claim).

**CLI** — `darkmatter/cli/src/commands/schema/about.rs`

- `expression_function_signatures_markdown()` (line ~496): `catalog.categories`,
  `d.category == category.name`, `escape_markdown(category.name)`,
  `descriptor.description` in `format!`. Unit tests at lines 849 and 870
  (`expression_function_signatures_render_typed_list_formatters`,
  `..._follow_category_list_and_alphabetical_order`) and line 876 (`.categories`).

**DMLS** — `darkmatter/dmls/src/overlay/expressions.rs`

- `function_descriptors()`, `function_overloads()`, `function_descriptor()`
  return `&'static` (static owner: keep); `function_description() -> Option<&'static str>`
  (keep via static owner); `format_function_block`, `format_function_overloads_block`,
  completion at line ~865 (`descriptor.aliases.iter().any(|alias| alias.starts_with(..))`,
  `descriptor.signature.to_string()`).
- Tests: `test_function_descriptor_returns_typed_signature` (`Some(length.description)`),
  `an_alias_resolves_and_completes_as_its_canonical` (`aliases.contains(&"isstring")`),
  `overloads_hover_together`, `test_function_descriptor_fallible_signature_has_error_suffix`,
  `shipped_catalog_descriptors_all_reach_completion` (`documentation.as_deref() == Some(descriptor.description)`).
  These comparisons will need `&*`/`as_ref()` after `Box<str>`.
- `dmls/src/providers/dsl.rs:297` is the only production caller of
  `host_function_catalogs`.

**Claudine**

- `claudine/cli/src/commands/context/mod.rs:71`: groups
  `Vec<(&'static str, Vec<&'static ExpressionFunctionDescriptor>)>` from
  `expression_function_catalog().categories/.descriptors` (static owner: keep).
- `claudine/cli/src/commands/context/format.rs:108` `format_function_examples`
  matches `FunctionExample::{Call, File}` fields (bindings become `&Box<str>`; `format!` unaffected).
- Tests reading fields: `claudine/cli/tests/l1/context_command.rs` 471–608
  (`f.signature`, `function.returns == "recent-commits"`, `.description.contains`),
  `claudine/lib/tests/l1/claudine_schemas.rs:380` (`name_of(descriptor.signature)`).
- Claudine registers no host expression functions:
  `claudine/lib/tests/l1/host_catalog.rs::claudine_ships_no_host_function_catalog_and_no_facts_artifact`
  is the absence guard. Lifecycle artifacts: `claudine/cli/tests/l1/shipped_prompts.rs::shipped_lifecycle_artifacts_use_mapping_only_set`
  and `claudine/lib/examples/lifecycle_artifact.rs`.
- Completed-spec spikes under `claudine/features/_completed/*/spike*` read
  descriptor fields but are not workspace members' targets; leave untouched.

**Darkmatter tests that copy or read descriptors (Copy / field-type assumptions)**

- `lib/tests/l1/lifecycle_schema_generation.rs`: `with_function_named` derefs a
  descriptor (`*descriptors.iter().find(..)`) and assigns `renamed.name = name`;
  lines 201 and 219 use `.copied()` on descriptor iterators. Migrate to `.cloned()`
  and `name.into()`.
- `lib/tests/l1/function_catalog_parity.rs` (130–202: `descriptor.aliases`, `.signature`),
  `lib/tests/l1/predict_conflicts.rs` (181–198), `lib/tests/l1/recent_commits_shared_type.rs`
  (67–69: `descriptor.function_type.returns`, `.description.contains`),
  `lib/tests/l1/function_dispatch.rs`, `dmls/tests/l1/lsp_session.rs` (2734–3138:
  `json!(descriptor.description)`, `json!(descriptor.signature)` — `Box<str>` serializes as a string,
  so these keep compiling), `lib/src/markdown/compose/expression/catalog/tests.rs`.

**Examples** — `darkmatter/lib/examples/expression_doc_generator.rs` and
`lifecycle_schema_generator.rs` call the generators only; neither reads
descriptor fields directly. Both compiled and ran pre-refactor.

### Preserved-output inventory (Pin preserved output)

| Output | Existing pin | Status |
| ------ | ------------ | ------ |
| Expression function table in `docs/topics/darkmatter-expressions.md` | `catalog/tests.rs::narrative_doc_function_table_matches_catalog` (exact text), `the_generated_table_shows_inline_display_only_and_file_examples` | covered |
| Lifecycle generated regions | `lib/tests/l1/lifecycle_schema_generation.rs::the_generated_regions_are_byte_identical_to_a_regeneration`, `projecting_twice_renders_identical_regions` | covered |
| DMLS lifecycle-verb completions | `dmls/tests/l1/frontmatter_response_contracts.rs::a3_completions_are_unchanged` (golden `a3_completions.json`) | covered |
| Host call diagnostics (DMLS) | `dmls/tests/l1/host_function_diagnostics.rs` (6 tests: complete, missing artifact, stale version, missing entry, failed catalog load, watched-change recovery) | covered |
| Claudine expression report | `claudine/cli/tests/l1/context_command.rs::context_expressions_function_order_snapshot`, `context_expressions_includes_every_function`; `claudine/cli/src/commands/context/tests.rs::function_examples_render_both_forms_one_per_line` | covered (order + examples) |
| Claudine facts/lifecycle artifacts | `host_catalog.rs::claudine_ships_no_host_function_catalog_and_no_facts_artifact`, `shipped_prompts.rs::shipped_lifecycle_artifacts_use_mapping_only_set` | covered |
| Darkmatter facts artifact bytes | `lib/tests/l1/host_functions.rs::the_committed_facts_artifact_is_fresh_and_deterministic` | covered |
| `md schema about` Expression Functions section (plain) | `cli/tests/l1/schema_about.rs::schema_about_verbose_prints_context_and_expression_sections` checks needles only | **gap: no exact-text pin** |
| DMLS function hover block / function completion items | `overlay/expressions.rs` unit tests compare against the descriptor (tautological after refactor) and ordering | **gap: no exact-text pin** |

Captured pre-refactor baselines (all byte-deterministic across two runs where checked):

- `baselines/md-schema-about-verbose.txt` and `baselines/md-schema-about.txt`
  (`NO_COLOR=1 COLUMNS=200 md [--verbose] schema about`, run from `/tmp`)
- `baselines/claudine-context-expressions.txt`, `baselines/claudine-context-side-effects.txt`
  (`NO_COLOR=1 COLUMNS=200 claudine context --expressions|--side-effects --plain`)
- `baselines/expression_doc_generator.stdout`, `baselines/lifecycle_schema_generator.stdout`
  (`cargo run --quiet --color=never -p darkmatter --example …`, default stdout mode)
- `baselines/dmls-function-hover-completion.txt`: every function's
  `format_function_overloads_block`, every `completion_candidates("", &[])`
  item (label, insert text, kind, detail, documentation), and three
  `unknown_function_message` outputs. Produced by a temporary unit test
  appended to `dmls/src/overlay/expressions.rs` and removed immediately after
  (the file is byte-identical to `HEAD`).

Phase 3 should turn the two gaps into exact-text tests (pin the
`md schema about` Expression Functions section and a DMLS function
hover/completion sample) and Phase 4 diffs the baselines against post-refactor
output.

### Facts-artifact matrix audit

`lib/tests/l1/host_functions.rs::the_facts_artifact_reader_walks_the_input_robustness_matrix`
(50 cells, control row present) against the validators in `host/artifact.rs`:

- Document shapes (strict loader `parse_strict`): malformed JSON, duplicate
  top-level key, duplicate nested key, trailing content, non-object root — covered.
  `artifact.rs` unit test `the_strict_parser_rejects_duplicate_keys_at_any_depth_and_trailing_content` also covers it.
  Non-UTF-8 bytes are decided by `ArtifactKey::from_read` in `schemas/catalog.rs`
  (not by the reader); no test today → Phase 3 cache tests.
- `required` / `required_in`: absent and null covered (top, entry, `facts[0].kind/type`).
- `nullable` (`conversion_recovery`, `malformed_reference`): absent → error and
  wrong type covered; explicit null → no rule covered by the control (fixture
  has both null). **Gap:** no valid non-null control for either object, and
  none of their nested fields (`subject`, `fallback`, `parameter`, `position`,
  nested unknown field, position mismatch) has a cell.
- `array`: wrong whole type covered via `facts`; empty `facts`/`source_exclusions`
  covered. **Gap (wiring):** `aliases` wrong type and `aliases[i]` wrong element;
  `source_exclusions` wrong whole type.
- `object`: entry wrong type/null, `facts[i]` wrong, `source_exclusions[i]` null covered.
- `string`: covered via `catalog_hash`, `lazy`. **Gap (wiring):**
  `source_exclusions[i].parameter`, `source_exclusions[i].kinds` (absent,
  wrong type, empty → error "empty kinds", unknown kind value).
- `position`: mismatch covered. **Gap:** `facts[0].position` wrong type.
- `reject_unknown`: top and function level covered. **Gap (wiring):** fact,
  exclusion, recovery, and reference objects.
- Count rule "at most one fact": **gap**.

YAML catalog loader (`load_catalog` via `parse_standalone_schema_document`):
`catalog/tests.rs` covers no categories, unlisted category, duplicate overloads,
missing/null/wrong kind, null entry, non-function entry, ordering, aliases,
examples/file embedding. `host_functions.rs::registration_rejects_collisions_parity_gaps_and_invalid_facts_before_evaluation`
and `a_function_catalog_in_a_schema_root_is_never_a_document_schema` cover the
host path. Phase 3 adds only the partial-construction case (valid inline
example then a non-embedded file example through `HostFunctionCatalog::load`).

Smell grep over the changed loading paths (`catalog/mod.rs`, `host/artifact.rs`,
`schemas/catalog.rs` host section): `raw_type_entries` uses
`.ok()`-style early returns and `filter_map(.. as_str())`, and `unwrap_or_default()`
on `raw_entries.get(..)`. These are intentional: they only recover authored
display text and fall back to the JSON spelling; the authoritative parse is
`parse_standalone_schema_document`. `atom.description.clone().unwrap_or_default()`
makes an absent description empty — existing behavior, preserved.
`file_example` maps an absent `returns` to `""` — existing behavior, preserved.

### Cache and probe inventory

- `lib/src/markdown/schemas/catalog.rs::host_function_set` (line ~591):
  function-local `static CACHE: LazyLock<Mutex<HashMap<(PathBuf, String, ArtifactKey), Result<HostFunctions, HostFunctionCatalogError>>>>`;
  lookup, build, insert under one lock; never evicts. Artifact read by
  `ArtifactKey::read` (`std::fs::read`, not routed through `discovery_probe`).
- `host_function_catalogs` doc comment (line ~520) states "a loaded catalog
  lives for the rest of the process" — to correct in Phase 3.
- `discovery_probe` (`cfg(any(test, feature = "work-counters"))`, thread-local):
  `read`, `note_build`, `fail_reads_of`, `builds`, `take_reads`,
  `cached_outcome(path)` for `profile_cache()`. Phase 3 extends it (per plan
  Decision 8) with a host-function cache observation and routes the artifact
  read through `discovery_probe::read`.
- Existing related tests: `schemas/catalog.rs::lifecycle_profiles_follow_each_root_file_s_current_content`,
  `host_functions.rs::a_function_catalog_in_a_schema_root_is_never_a_document_schema`,
  `dmls/tests/l1/host_function_diagnostics.rs::a_running_editor_recovers_and_withdraws_host_facts_after_watched_changes`.

### Design Decisions

1. **Baselines as fix-directory files, tests in Phase 3**
   - **decision:** Capture pre-refactor output as files under this fix's
     `baselines/` directory rather than add golden tests in Phase 1.
   - **alternatives considered:** add golden fixtures and tests now; rely only
     on existing tests.
   - **why:** the plan assigns new tests to Phase 3 and Phase 1 is the
     inventory wave; the existing tests leave two outputs unpinned, and a
     golden written after the refactor could not prove parity.
   - **downsides:** the files are not read by CI; Phase 3/4 must diff against
     them explicitly.
2. **Claudine example helpers take a view by value**
   - **decision:** In Phase 2, change `format_example` and
     `format_effect_example` to take `Option<Example<'_>>` and call them with
     `desc.example()` (the trait method), replacing `desc.example.as_ref()`.
   - **alternatives considered:** keep `Option<&Example<'_>>` and pass
     `desc.example.as_ref()` to the static field.
   - **why:** matches the new `Described::example() -> Option<Example<'_>>`
     contract and Decision 2's "direct field consumers switch to the trait method".
   - **downsides:** Claudine imports `Described` in those modules.

### Results

- Source files changed: none (temporary DMLS capture test added and removed).
- Builds run: `cargo build -p darkmatter-cli -p claudine-cli`, both generator
  examples via `cargo run --example`, and the temporary DMLS capture test via
  `cargo nextest run -p dmls --lib -E 'test(tmp_capture_function_baseline)'` (1 passed).

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: ["darkmatter"]`, `change_class: package` (selection reason: "changed test input
  read by package(s) darkmatter"; a darkmatter test reads the fix directory), so the gates ran.
- `just lint` (darkmatter): passed.
- `just test` (darkmatter, includes DMLS): 12593 passed, 10 skipped, 0 failed (1 slow), 100.8 s.
- No dependent-package tests: no source changed, so no downstream symbol changed.
- Cross-OS check: not run; Phase 1 changed no code.
