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
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
human_review: false
source_files_during_phase_2:
    - darkmatter/lib/src/catalog/mod.rs
    - darkmatter/lib/src/effects/catalog.rs
    - darkmatter/lib/src/effects/types.rs
    - darkmatter/lib/src/lifecycle/schema_projection.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/semantics.rs
    - darkmatter/lib/tests/l1/function_catalog_parity.rs
    - darkmatter/lib/tests/l1/function_dispatch.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/cli/src/commands/schema/about.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/cli/src/commands/context/effects.rs
    - claudine/cli/src/commands/context/expressions.rs
    - claudine/cli/src/commands/context/format.rs
    - claudine/cli/src/commands/context/mod.rs
    - claudine/cli/src/commands/context/tests.rs
    - claudine/cli/tests/l1/context_command.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
docs_updated_during_phase_2:
    - darkmatter/lib/src/catalog/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/expressions.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/cli/src/commands/schema/about.rs
    - darkmatter/cli/tests/l1/schema_about.rs
    - darkmatter/dmls/src/overlay/expressions.rs
docs_updated_during_phase_3:
    - darkmatter/docs/topics/host-functions.md
    - darkmatter/lib/src/catalog/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/library-surfaces.md
source_files_during_phase_4:
    - claudine/lib/src/composition/lifecycle/prewarm/fakes.rs
    - claudine/lib/src/composition/lifecycle/prewarm/worker/tests.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
    - darkmatter/lib/src/catalog/mod.rs
    - darkmatter/lib/src/effects/catalog.rs
    - darkmatter/lib/src/effects/types.rs
    - darkmatter/lib/src/lifecycle/schema_projection.rs
    - darkmatter/lib/src/lifecycle/signatures.rs
    - darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/semantics.rs
    - darkmatter/lib/tests/l1/function_catalog_parity.rs
    - darkmatter/lib/tests/l1/function_dispatch.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/cli/src/commands/schema/about.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/cli/src/commands/context/effects.rs
    - claudine/cli/src/commands/context/expressions.rs
    - claudine/cli/src/commands/context/format.rs
    - claudine/cli/src/commands/context/mod.rs
    - claudine/cli/src/commands/context/tests.rs
    - claudine/cli/tests/l1/context_command.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/catalog/host_function_cache_tests.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/cli/tests/l1/schema_about.rs
    - claudine/lib/src/composition/lifecycle/prewarm/fakes.rs
    - claudine/lib/src/composition/lifecycle/prewarm/worker/tests.rs
documentation:
    - darkmatter/lib/src/catalog/README.md
    - darkmatter/docs/topics/host-functions.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/library-surfaces.md
completed_phase: 4
implemented: true
pipeline_refreshed: []
decisions_for_author: []
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

## Phase 2

Phase 2 is the plan's Wave 2 (Own catalog data). The expression-function
catalog now owns every value it projects, and every repository consumer
compiles against borrowed views. The host-function cache (Wave 3) is unchanged.

### Changes

- **Shared framework** (`lib/src/catalog/mod.rs`): `Example<'a>` and
  `ExampleVerification<'a>` are `Copy` borrowed views (`DisplayOnly(&'a str)`).
  `Described::{key, description, category}` return `&str` borrowed from
  `&self`; `example()` returns `Option<Example<'_>>`. Trait, module, and
  `suggest_strings` comments rewritten (the latter no longer cites
  `&'static str`).
- **Expression catalog** (`expression/catalog/mod.rs`): `CatalogCategory`,
  `FunctionParameterDescriptor`, `FunctionExample`, and
  `ExpressionFunctionDescriptor` hold `Box<str>`, `Box<[_]>`, and an owned
  `FunctionType`; they lose `Copy` and keep `Clone`. `aliases` is
  `Box<[Box<str>]>`. `FunctionCatalog::{categories, descriptors}` are boxed
  slices. The duplicate `example` field is removed; `Described::example`
  derives the view from the first `FunctionExample::Call`, with the same
  display-only mapping. `leak()` and every `Box::leak` are gone from
  `load_catalog`, `project_examples` (now takes `&FunctionType`), and
  `file_example`, so a later projection error frees earlier overloads' data.
  `descriptors_for(&self)` borrows from `self`; `expression_function_descriptors()`
  and `function_descriptors()` keep `&'static` by borrowing the `LazyLock` owner.
  Loader doc comment corrected.
- **Other `Described` impls**: `EffectDescriptor`, `ContextVariableDescriptor`,
  and the eight `expression/semantics.rs` descriptors store
  `Option<Example<'static>>` and return it by value.
- **Library consumers**: `effects/types.rs` (borrow of the static
  `descriptors`, `&*descriptor.name` map keys), `lifecycle/signatures.rs`
  (`&'static` views borrowed from the static descriptors), `lifecycle/schema_projection.rs`
  (`&descriptor.function_type`), `context/capture/capabilities.rs`,
  `expression/functions/mod.rs`, `expression/host/mod.rs`
  (`function_names(&self) -> Vec<&str>`; `load` doc no longer says descriptors
  live for the process).
- **CLI / DMLS / Claudine** (compile adaptations only, no output change):
  `cli/src/commands/schema/about.rs`, `dmls/src/overlay/expressions.rs`;
  Claudine `cli/src/commands/context/{expressions,effects,format,mod}.rs` call
  `example()` and its helpers take `Option<Example<'_>>` (Phase 1 Decision 2).
- **Tests migrated** for borrowed fields and no `Copy`: `catalog/tests.rs`,
  `effects/types.rs`, `lifecycle/signatures.rs` and `functions/mod.rs` unit
  tests; lib L1 `lifecycle_schema_generation.rs` (`.cloned()`, `name.into()`),
  `predict_conflicts.rs` (`example()`), `function_catalog_parity.rs`,
  `function_dispatch.rs`; DMLS `overlay/expressions.rs`, `providers/dsl.rs`,
  `tests/l1/lsp_session.rs`; Claudine `cli/src/commands/context/tests.rs`,
  `cli/tests/l1/context_command.rs`, `lib/tests/l1/claudine_schemas.rs`.
- **Docs**: `lib/src/catalog/README.md` (borrowed `example()` view, `&str`
  trait returns, `Option<Example<'static>>` for static tables). Skill:
  `.claude/skills/darkmatter/expressions.md` ownership note.

Ownership audit of the changed loader/projection paths: no `leak`,
`Box::leak`, or `'static` storage remains in `expression/catalog/mod.rs`,
`expression/host/`, or `effects/types.rs`. Out of scope and unchanged per the
spec: `context/catalog.rs::leak` (context catalog) and the test-only leaked
fixture in `lifecycle/signatures.rs`.

### Requirement-to-test mapping

| Behavior | Test |
| -------- | ---- |
| Loaded catalog owns its data independent of the source text and path | new `catalog::tests::a_loaded_catalog_outlives_the_text_it_was_loaded_from` (drops the `String` source and path, then reads names, types, examples, reason, categories, lookups) |
| `Described::example` derives from the first inline call; display-only reason carries; file-only and no-example functions have no view | new `catalog::tests::the_described_example_is_a_view_of_the_first_inline_call`; existing `examples_keep_their_authored_text_and_files_are_embedded`, `predict_conflicts.rs::shipped_catalog_describes_predict_conflicts_exactly` |
| Static APIs keep `&'static` | compile-time: `Overload::{parameters, names, display}`, `all_lifecycle_verbs`, `expression_function_signature` and `effect_registration` still return `&'static` from static owners (Phase 3 adds explicit assignment checks) |
| Preserved output | baselines diffed byte-for-byte: both generator examples, `md schema about` (plain and verbose), `claudine context --expressions/--side-effects --plain` (stdout) — all identical; existing exact-text pins (`narrative_doc_function_table_matches_catalog`, `the_generated_regions_are_byte_identical_to_a_regeneration`, `a3_completions_are_unchanged`) pass |

Fidelity: F1 unit tests at the library's public loader result (L1); the
CLI-boundary parity is shown by the baseline diffs and the existing CLI L1
tests. No input format, parser, or validator changed in this phase, so the
input robustness matrix is not re-walked here (it belongs to Phase 3's cache
and facts work).

### Design Decisions

1. **Consumer compile adaptations land in Phase 2**
   - **decision:** Make the CLI, DMLS, and Claudine consumers (and their tests)
     compile in this phase with mechanical borrow changes only; Phase 3's
     "Adapt external consumers" lane keeps the output-pinning tests.
   - **alternatives considered:** leave the external crates broken until
     Phase 3; feature-gate the change.
   - **why:** the phase gate is the darkmatter area's full `just test`, which
     builds the CLI and DMLS; a broken Claudine would also fail its dependents.
     The changes are type-level only, and every output was diffed equal.
   - **downsides:** Phase 3's consumer lane has less to do; it must still add the
     exact-text pins listed in Phase 1.
2. **`function_type` is an owned `FunctionType`, not `Box<FunctionType>`**
   - **decision:** Store `function_type: FunctionType` in the descriptor and
     clone it out of the parsed `TypeExpr::Function` box.
   - **alternatives considered:** `Box<FunctionType>` to keep the descriptor
     small.
   - **why:** plan Decision 1 says "owned `FunctionType` values"; consumers
     borrow `&descriptor.function_type` or `.clone()` it exactly as before.
   - **downsides:** a larger descriptor value; irrelevant at catalog sizes.
3. **Aliases are copied per overload**
   - **decision:** Build each function's `Box<[Box<str>]>` aliases once and
     clone it into each overload's descriptor.
   - **alternatives considered:** share through `Arc<[..]>`.
   - **why:** spec's settled downside ("repeated names … may be allocated
     separately"); aliases are empty for every host function.
   - **downsides:** duplicate small allocations for overloaded built-ins.

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, claudine-cli, darkmatter, darkmatter-cli, dmls, test-toolkit]`,
  `change_class: package`, so the gates ran.
- Targeted first: `cargo nextest run -p darkmatter --lib -E 'test(/expression::catalog::/) | …'`
  → 388 passed.
- `just lint` (darkmatter): passed (run again after the test migrations).
- `just test` (darkmatter, includes CLI and DMLS): 12595 passed, 10 skipped,
  0 failed (107 slow under load), 243 s. The first run failed to compile two
  feature-gated L1 files (`function_catalog_parity.rs`, `function_dispatch.rs`,
  behind `effects-instrumentation`/`test-fixtures`); fixed, then green.
- Dependents (scoped, serially): `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/commands::context::/) | test(/context_command/) | test(/shipped_lifecycle_artifacts/)'`
  → 44 passed; `cargo nextest run -p claudine -E 'test(/claudine_schemas/) | test(/host_catalog/)'`
  → 845 passed. `just lint` (claudine): passed (pre-existing linker note only).
  `test-toolkit` references none of the changed symbols; not run.
- Cross-OS check: not run. The change is type/ownership only, with no paths,
  processes, or `cfg` code.

## Phase 3

Phase 3 is the plan's Wave 3: four lanes. The coordinator wrote the cache lane
(it touches the core discovery code); three subagents ran the consumer-pin,
catalog-contract, and documentation lanes in parallel with disjoint file
ownership.

### Bound revision retention (coordinator)

- `lib/src/markdown/schemas/catalog.rs`: `host_function_set` now keeps one
  `HostFunctionEntry { catalog_hash, artifact: ArtifactKey, result }` per
  discovery `PathBuf` in `host_function_cache()` (the same singleton
  `LazyLock<Mutex<..>>` pattern as `profile_cache()`, moved out of the
  function so the probe can inspect it). Lookup, comparison (catalog BLAKE3
  digest and the complete `ArtifactKey`: absent / unreadable with reason /
  exact text), build, and replacement all run under the one lock. An
  unchanged revision returns its cached result, error included; any change
  inserts the new result over the old entry, so the old set (or error, and
  the old artifact text) is dropped from the cache. The old
  `(PathBuf, String, ArtifactKey)` tuple key and its comment are gone.
- `host_function_catalogs`' doc comment no longer says a loaded catalog lives
  for the rest of the process; it states the per-path latest-revision rule
  and that held sets survive replacement.
- `discovery_probe` (Decision 8: extend the existing probe, no second seam)
  gains a `host_function_builds()` counter (separate from the lifecycle
  `builds()` so existing lifecycle assertions are unaffected),
  `cached_host_functions(path)` (catalog hash, `CachedArtifact`, and the
  cached function names or error — a summary that holds nothing alive), and
  `host_function_entries_under(dir)`. The artifact read is **not** routed
  through `discovery_probe::read`: no test needs to observe or inject it, and
  the non-UTF-8 case is a real byte fixture.
- `lib/src/markdown/compose/expression/host/mod.rs`: `cfg(test)`
  `HostFunctionCatalog::owner_probe()` → `CatalogOwnerProbe` (a `Weak` to
  the private `CatalogData`, `is_alive()`); crate-private, test-only.

New L1 module `lib/src/markdown/schemas/catalog/host_function_cache_tests.rs`
(6 tests, each ~0.06 s; every test uses its own canonical temp roots, goes
through the production `host_function_catalogs`, and counts only entries
under its own temp dir — no global size assertion, no cache reset):

| Acceptance | Test |
| ---------- | ---- |
| 1 catalog-only change reclaims; inline + display-only examples owned; unchanged reuse | `a_catalog_only_change_replaces_the_entry_and_frees_the_old_catalog` |
| 1 artifact-only change reclaims | `an_artifact_only_change_replaces_the_entry_and_frees_the_old_catalog` |
| 1 failing-catalog replacement reclaims; 2 success→error, unchanged cached error (build counter), error→success; 4 partial construction (inline example then file-form example) via discovery | `a_failing_catalog_replaces_a_loaded_one_and_a_valid_one_recovers` |
| 2 facts valid → absent → invalid JSON → stale hash → non-UTF-8 → valid, classification, cached artifact state, one entry | `facts_transitions_classify_each_artifact_state_and_recover` |
| 3 held set + guard keep old registrations and call analysis; scope precedence of a newer guard; reclaimed after both drop | `a_held_set_and_guard_survive_replacement_with_their_own_analysis` |
| 3 two paths independent; discovery order; shadowed file never loaded | `distinct_paths_keep_independent_entries_in_discovery_order` |

Regression proof (by inspection; the old code was not rebuilt while the other lanes compiled): against the previous tuple-keyed cache, each test fails —
a second revision adds a second entry for the path (`entries() == 1`
fails) and the old set stays in the cache (`!owner.is_alive()` fails).
Fidelity: F1 in-process at the crate's discovery API, real files in temp
dirs; L1.

### Adapt external consumers (subagent)

Exact-text pins, expected text copied from the pre-refactor `baselines/`
(so a pass proves parity); no expectation was edited to match output:

- `darkmatter-cli` unit (`commands::schema::about::tests`):
  `expression_function_signatures_pin_category_heading_order` (all 22
  category headings in order), `expression_function_signatures_pin_math_excerpt_exactly`
  (`**Math**` heading + full `abs`/`max`/`min`/`round` lines),
  `expression_function_signatures_pin_overloaded_link_lines_exactly`. An
  excerpt, not a whole-catalog golden, so adding a built-in does not churn it.
- `darkmatter-cli` L1 `schema_about::schema_about_verbose_prints_context_and_expression_sections`
  extended: the spawned `md --verbose schema about` (`NO_COLOR=1`) contains
  the exact seven wrapped lines of the two `link` overloads.
- DMLS (`overlay::expressions::tests`): `overloaded_function_hover_block_is_pinned_exactly`
  (`link`), `single_signature_function_hover_block_is_pinned_exactly`
  (`round`), `function_completion_items_are_pinned_exactly` (label, insert
  text, kind, detail, documentation for three items). Host call diagnostics
  remain pinned by `dmls/tests/l1/host_function_diagnostics.rs` (passing).
- Claudine needed no edit; its registration/context/lifecycle-artifact tests
  pass unchanged (no new host expression feature, per the plan).

### Pin catalog contracts (subagent)

- `lifecycle::actions::signatures::tests::static_catalog_accessors_lend_permanent_borrows`
  (new; the module is declared from `actions.rs`): binds the three static
  accessors to `&'static` slice function types, `all_lifecycle_verbs()` to
  `Vec<&'static str>`, verb/display/names/parameter names to `&'static str`,
  `parameters()` to `&'static [FunctionParameterDescriptor]`, and a static
  descriptor's `example()` to `Option<Example<'static>>`, then checks values.
- `expression::catalog::tests::the_described_example_is_a_view_of_the_first_inline_call`
  (extended): `=> error` first example; static effect (`DisplayOnly`) and
  operator (`Executable`) views. `a_loaded_catalog_outlives_the_text_it_was_loaded_from`
  (extended): the `example()` view after the source and path drop.
- `host_functions::a_host_catalog_outlives_the_strings_it_was_loaded_from`
  (new L1): load from `String`s, drop both, then names, path, version,
  `analysis_only` registrations/facts, and a byte-identical regenerated facts artifact.
- `host_functions::a_host_catalog_failing_after_a_valid_inline_example_is_a_typed_load_error`
  (new L1, acceptance 4): exact `HostCatalogError` message
  ``project_partial.yaml: `types.project_second`: example file `./function-examples/as_tsv.yaml` is not embedded``.
- `host_functions::the_facts_artifact_reader_walks_the_input_robustness_matrix`
  (extended, 50 → 71 cells plus 7 `conversion_recovery` cells over an inline
  `project_round(x: number, fallback?: number) -> number` catalog whose base
  artifact comes from the public builder): fills every Phase 1 gap —
  `facts[0].position` wrong type, unknown field in a fact, two facts;
  `aliases` valid control, wrong whole type, wrong element;
  `source_exclusions` wrong whole type; exclusion `parameter` wrong type and
  unregistered; `kinds` absent / wrong type / empty / unknown kind; unknown
  field in an exclusion; `malformed_reference` valid control, `parameter`
  wrong type / unregistered, `position` absent / wrong type / mismatch,
  unknown field; `conversion_recovery` valid control, explicit null (no
  rule), `subject` absent / wrong type / unregistered, `fallback` null,
  unknown field. Shared validators get one wiring case per extra field, not
  a matrix per field. No validator defect found.
- Generators: `cargo check -p darkmatter --example expression_doc_generator --example lifecycle_schema_generator`
  passes; both run with exit 0 and their stdout is byte-identical to
  `baselines/*.stdout`.

Smell grep (input-matrix rule): the only changed loading path this phase is
`host_function_set`; it adds no `serde(default)`, `Option` conflation,
`filter_map(.. as_str())`, `unwrap_or_default()`, or `.ok()` on a
load-bearing parse. The Phase 1 classification of the existing occurrences
stands.

### Document ownership contracts (subagent)

- `darkmatter/docs/topics/host-functions.md`: new sections "How long a
  loaded catalog lives" (shared owner, freed with the last catalog clone,
  set, or guard; only installed sets and the built-in catalog are
  process-lifetime) and "Discovery keeps the latest revision of each
  catalog" (entry table, Mermaid lookup/replace flow, unchanged reuse incl.
  errors, replacement, held sets survive, no stale success on failure,
  recovery, facts problems vs catalog failures table, one lock and what
  "latest" means, per-path bound, the lifecycle profile cache's different rules).
- `darkmatter/lib/src/catalog/README.md`: intro no longer says every catalog
  is static; borrowed `Example<'a>` views; owned expression descriptors;
  `suggest_strings` entry.
- Skills: `.claude/skills/darkmatter/expressions.md` (host catalog lifetime
  and per-path cache; lifecycle cache differs) and `library-surfaces.md`
  (owned descriptors, `&'static` views, borrowed `Described`, cache rule).
- `docs/topics/darkmatter-expressions.md` and `darkmatter/README.md` carry no
  affected claim; unchanged. No dependency-doc change.

### Design Decisions

1. **Per-path cache uses its own counter in `discovery_probe`**
   - **decision:** Add `host_function_builds()` beside the lifecycle
     `builds()` counter, plus `cached_host_functions(path)` and
     `host_function_entries_under(dir)` summaries, in the existing probe.
   - **alternatives considered:** reuse `note_build`/`builds()` for both
     caches; a separate test-only seam for host functions.
   - **why:** plan Decision 8 asks for one observation mechanism per file; a
     shared counter would conflate two caches with different failure rules.
   - **downsides:** the probe module grows three host-specific items.
2. **The facts artifact read is not routed through `discovery_probe::read`**
   - **decision:** Keep `ArtifactKey::read` on `std::fs::read`.
   - **alternatives considered:** route it through the probe (Decision 8's note).
   - **why:** no acceptance case needs to observe or inject that read; the
     non-UTF-8 "unreadable" case is a portable byte fixture, and routing would
     add the facts path to the lifecycle tests' read logs.
   - **downsides:** read-failure injection for the facts artifact (other than
     non-UTF-8) is not available to tests.
3. **The probe returns summaries, not cached sets**
   - **decision:** `cached_host_functions` returns the hash, artifact state,
     and function names or error — never a `HostFunctions` clone.
   - **alternatives considered:** return the cached `Result<HostFunctions, _>`.
   - **why:** a returned clone would hold the owner alive and confuse the
     weak-reference reclamation assertions.
   - **downsides:** identity of a reused cached value is shown by the build
     counter rather than pointer equality.
4. **Exact-text pins are excerpts**
   - **decision:** Pin a representative excerpt of `md schema about`'s
     Expression Functions section, the category order, and selected DMLS
     hover/completion items, not the whole built-in catalog.
   - **alternatives considered:** a full golden of the section.
   - **why:** a full golden churns on every new built-in function; the full
     generated table is already pinned exactly by `narrative_doc_function_table_matches_catalog`.
   - **downsides:** a regression confined to an unpinned function's line in
     these two renderers would not be caught by the new pins.

### Requirement-to-test summary

| Spec acceptance | Tests |
| --------------- | ----- |
| 1 Reclaim replaced owners | `host_function_cache_tests::{a_catalog_only_change…, an_artifact_only_change…, a_failing_catalog_replaces…}` |
| 2 Replace results, incl. errors; facts transitions | `a_failing_catalog_replaces_a_loaded_one_and_a_valid_one_recovers`, `facts_transitions_classify_each_artifact_state_and_recover` |
| 3 Held values survive; two paths | `a_held_set_and_guard_survive_replacement_with_their_own_analysis`, `distinct_paths_keep_independent_entries_in_discovery_order` |
| 4 Failed construction owns everything | `host_functions::a_host_catalog_failing_after_a_valid_inline_example_is_a_typed_load_error`, the failing-catalog cache test; loader audit in Phase 2 (repeated in Phase 4) |
| 5 Built-in contracts and output | `static_catalog_accessors_lend_permanent_borrows`, extended example tests, `a_host_catalog_outlives_the_strings…`, CLI/DMLS exact pins, generator diffs, existing generated-region tests |
| 6 Scoped checks | gates below |

### Gates

- `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, claudine-cli, darkmatter, darkmatter-cli, dmls, test-toolkit]`,
  `change_class: package`, so the gates ran.
- `just lint` (darkmatter): exit 0. (One mid-phase `type_complexity` hit in
  `host_functions.rs` was fixed by its lane with a `RecoveryCell` alias.)
- `just test` (darkmatter, incl. CLI and DMLS): 12610 passed, 10 skipped,
  0 failed (3 slow), 103 s, run once after all lanes merged with no subagent running.
- Dependents (scoped, serially): `cargo nextest run -p claudine -E 'test(/host_catalog|claudine_schemas/)'`
  → 845 passed; `cargo nextest run -p claudine-cli --features test-fixtures -E 'test(/commands::context::/) | test(/context_command/) | test(/shipped_lifecycle_artifacts/)'`
  → 44 passed. `test-toolkit` uses none of the changed symbols; not run.
- New tests each run in under 0.2 s; none needs a terminal or browser.
- Cross-OS check: not run. The production change is an in-memory map keyed
  by the discovery path as before (no new path normalization or
  comparison); the new tests build paths from a `canonicalize_simplified`
  temp dir and use a byte fixture rather than permissions. CI's Linux/macOS
  legs and the nightly Windows/WSL2 legs cover it.

## Phase 4

Phase 4 is the plan's Wave 4 (combined validation) and Wave 5 (closure
audit). The coordinator did all of it; no subagent ran.

### Combined validation (Wave 4)

- `just test` (darkmatter, incl. CLI and DMLS): 12610 passed, 10 skipped,
  0 failed, 0 slow, 97 s.
- `just lint` (darkmatter, incl. the zed-dmls wasm check): exit 0.
- `just lint` (claudine): exit 0.
- `just test` (claudine), first run: stopped at one failure,
  `composition::lifecycle::prewarm::worker::tests::a_successful_or_no_op_preparation_counts_as_done`
  (`left: []`, `right: ["done"]`). It passed 5 of 5 runs alone, but the cause
  is a real race in the test, not just load: `FakePreparer::wait_for_calls`
  returns once `prepare` is *invoked*, while `completed` is recorded only when
  the returned future runs to the end, and the test then drops the worker,
  which cancels an unfinished preparation. `failures_warn_once_without_the_phrase_and_later_candidates_still_run`
  had the same shape (`wait_for_calls(3)`, drop, assert `completed()` of
  three). Fixed in the test fake: `FakePreparer::wait_for_completed(count)`
  (a second `watch` counter bumped after `completed` is pushed), used by those
  two tests. The other `wait_for_calls` sites either use the blocking fake or
  rely on serial execution (a second call means the first completed), so they
  are unchanged. Unrelated to the host-function catalog; fixed because it
  blocked the required Claudine gate and would flake in CI.
  `cargo nextest run -p claudine -E 'test(/prewarm/)'` → 57 passed.
- `just test` (claudine), rerun: 11654 passed, 10 skipped, 0 failed (3 slow,
  all pre-existing `claudine-cli` L1 process tests unrelated to this fix:
  `research_sequence_artifacts::shipped_research_fleet_runs_row_freshness_and_success_through_normal_cli`,
  `sequence_task_providers::empty_hint_nested_sequence_list_fails_its_body_step_before_any_task_launches`,
  `wrap_compose_validation::predicates_operands_and_proxy_with_values_are_rejected_before_launch`).
  Claudine's `just test` includes the context-command, `claudine_schemas`, and
  shipped lifecycle-artifact tests, so the Claudine artifact checks pass.
- `cargo check --color=never -p darkmatter --example expression_doc_generator --example lifecycle_schema_generator`:
  passes.
- `cargo run --quiet --color=never -p darkmatter --example expression_doc_generator`
  and `... --example lifecycle_schema_generator`: both exit 0, and `cmp`
  against `baselines/expression_doc_generator.stdout` and
  `baselines/lifecycle_schema_generator.stdout` reports byte-identical. No
  write mode used, no baseline changed.
- Generated-region and facts tests in the darkmatter run:
  `lifecycle_schema_generation::the_generated_regions_are_byte_identical_to_a_regeneration`
  (0.08 s), `expression::catalog::tests::narrative_doc_function_table_matches_catalog`
  (exact expression-table text, 0.05 s), `host_functions::the_committed_facts_artifact_is_fresh_and_deterministic`
  (0.06 s) all pass, so no extra direct table comparison was needed.
- New-test durations (from this run): the six `host_function_cache_tests`
  0.056–0.063 s; `host_functions` additions 0.026–0.093 s; signature, CLI,
  and DMLS pins 0.04–0.17 s; the spawned `schema_about_verbose_prints_context_and_expression_sections`
  0.46 s. None is near the 5 s budget, so nothing was split.
- Portability: nothing this phase added touches paths or platform behavior.
  Evidence is from macOS only. Linux, Windows, and WSL2 were not run here; CI
  and the nightly cover them.

### Closure audit (Wave 5)

- Leak search over every `.rs` file in `26d72d2a3` (Phases 2–3):
  `Box::leak`/`.leak()`/`mem::forget`/`unsafe`/`transmute`/`String::leak`.
  Remaining hits are only the sites the spec excludes:
  `markdown/compose/context/catalog.rs::leak` (the bounded one-time projection
  behind a `LazyLock`), the test fixture in `lifecycle/signatures.rs` tests,
  and the test-only leaks in `claudine/lib/tests/l1/claudine_schemas.rs`.
  `predict_conflicts.rs`'s hits are the word "unsafe" in a git fixture. The
  expression catalog has no `leak` helper left.
- Ownership: `ExpressionFunctionDescriptor` and `FunctionParameterDescriptor`
  own every field (`Box<str>`, `Box<[..]>`); `HostFunctionCatalog` is
  `Arc<CatalogData>` with an owned `PathBuf`, `String` version, and
  `FunctionCatalog`; `&'static` views come only from the static
  `expression_function_catalog()` accessors. Categories, aliases,
  parameters, compiled types, examples, and reasons all trace to that one
  owner.
- Cache (`schemas/catalog.rs::host_function_set`): one `HostFunctionEntry`
  per path. Lookup, build, and replacement run under one lock; an unchanged
  revision (catalog BLAKE3 hash and full `ArtifactKey`) returns the cached
  result, error included; any change overwrites the entry, so neither an
  earlier success nor the old artifact text survives a failure. `result` is
  built before `artifact` moves into the entry, so the compiler proves the
  set does not borrow the artifact text (no self-reference). The reclamation
  tests check `Weak` owners through `HostFunctionCatalog::owner_probe` and
  hold the set and guard explicitly.
- Docs and skills: `docs/topics/host-functions.md`, `lib/src/catalog/README.md`,
  and the two darkmatter skill pages match the code. None names this fix or a
  date, and no stale "lives for the rest of the process" claim remains in the
  changed loading paths.
- Departures from the spec: none beyond those already recorded in Phases 2–3.

### Design Decisions

1. **Fix the Claudine prewarm test race in the fake, not by rerunning**
   - **decision:** Add `FakePreparer::wait_for_completed` and use it in the
     two tests that assert `completed()` after dropping the worker.
   - **alternatives considered:** record the failure as load-sensitive and
     move on; make the recording fake complete synchronously inside `prepare`.
   - **why:** the failure is a real ordering race (completion is recorded
     after the call is counted), so it would recur under CI load. Completing
     inside `prepare` would break the fake's documented contract that an
     unpolled, dropped call is never completed.
   - **downsides:** a test-only change in Claudine, outside this fix's
     catalog scope; the fake grows one more counter.

### Requirement-to-test mapping (final)

The mapping is unchanged from Phase 3's "Requirement-to-test summary".
Phase 4 adds no catalog behavior. Acceptance 6 (scoped checks) is the gate
list above.

### Gates

- Scope check, run after all edits from the repository root:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, test-toolkit, darkmatter]`, `change_class: package`,
  so the gates ran (listed above). `test-toolkit` was not run: the only
  Phase 4 source change is Claudine's crate-private `FakePreparer` and its
  tests, which nothing outside `claudine` can name.
- Dependents: Claudine (`claudine`, `claudine-cli`, `claudine-gen`) ran in
  full via its `just test`. The only Phase 4 source change is in Claudine's
  own tests, so no further dependents apply.
- Cross-OS check: not run. Phase 4 changed only a test fake and two tests,
  with no path or platform code.

## Pipeline Refresh

- `2026-10-25-schema-diagnostic-provenance` (this spec's `next_spec`, plan
  not started): checked and unchanged. It concerns schema diagnostic
  provenance in `simplified/*`, `schemas/mod.rs`, and DMLS named-type
  overlays. It names no host-function catalog, descriptor, or cache symbol
  this fix changed, and none of its files overlap this fix's files.
- `2026-10-25-pipeline-work-reduction` (lists this fix in its chain):
  checked and unchanged. Its process-wide cache rule ("keep the latest
  revision per path … never use `Box::leak` on a path that loads data at
  runtime") is exactly what this fix implemented, and its decided items
  (C*, D*, P1, X*) do not include the host-function cache.
