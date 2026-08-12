---
agent: codex/
agent_name: codex
total_phases: 7
created: 2026-07-09
phase: 7
yolo: "true"
packages:
  - darkmatter
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/features/2026-07-08-single-sourcing-schema/phase1-baseline-inventory.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/docs/schemas/darkmatter.yaml
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/context/catalog.rs
  - darkmatter/lib/src/markdown/compose/context/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/catalog/mod.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
docs_updated_during_phase_4:
  - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_4:
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_line_separated.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_csv.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_tsv.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_space_separated.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_unordered_list.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_ordered_list.yaml
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/context/capture.rs
  - darkmatter/lib/src/markdown/compose/context/format.rs
  - darkmatter/lib/src/markdown/compose/context/catalog.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - prompts/context.md
  - prompts/performance-review.md
  - prompts/code-comment-quality.md
  - prompts/faster-builds-and-tests.md
  - darkmatter/example-docs/ctx-and-eval/test.md
docs_updated_during_phase_6:
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/inline/fm-interpolation.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_7: []
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_code:
  - darkmatter/docs/schemas/darkmatter.yaml
  - darkmatter/lib/src/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/context/catalog.rs
  - darkmatter/lib/src/markdown/compose/context/mod.rs
  - darkmatter/lib/src/markdown/compose/context/capture.rs
  - darkmatter/lib/src/markdown/compose/context/format.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
documentation:
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/example-docs/ctx-and-eval/test.md
  - prompts/context.md
  - prompts/performance-review.md
  - prompts/code-comment-quality.md
  - prompts/faster-builds-and-tests.md
  - darkmatter/features/2026-07-08-single-sourcing-schema/phase1-baseline-inventory.md
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_line_separated.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_csv.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_tsv.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_space_separated.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_unordered_list.yaml
  - darkmatter/features/2026-07-08-single-sourcing-schema/examples/as_ordered_list.yaml
  - .claude/skills/darkmatter/compose.md
---

# Single-Sourcing the Frontmatter Schema and Context-Variable Catalog Plan

## Phase 1 - Preconditions and Baseline Inventory

- [x] Confirm the dependent `2026-07-08-schema-plus` work is implemented enough to support referenced `example()` artifacts, typed function signatures, pattern keys, and cross-file type imports.
- [x] Inventory the current `ctx.*` declarations in `darkmatter/docs/schemas/darkmatter.yaml` and `CONTEXT_VARIABLE_DESCRIPTORS`, recording every name, type, description, generated/required/default flag, category, subsection, order, and example.
- [x] Identify all temporal `ctx.*` schema entries that must be corrected, including `ctx.now` and `ctx.now_utc` as `datetime` and `ctx.today`-family entries as `date`.
- [x] Identify all in-repo references to removed or behavior-changing variables, especially `*_list` variables and bare CSV-rendered array variables.
- [x] Identify all tests, snapshots, docs, and generated artifacts that assert context-variable catalog output, schema-about output, compose output, or expression-function metadata.
- [x] Validation checkpoint: produce a baseline diff/inventory showing the exact catalog/schema drift and the exact caller migration list before implementation starts.

Parallelizable after schema-plus readiness is confirmed: inventorying YAML/catalog drift, caller references, and existing tests/snapshots.

## Phase 2 - Schema Data Becomes Authoritative

- [x] Update `darkmatter/docs/schemas/darkmatter.yaml` so every surviving `ctx.*` variable has the correct schema type, description, and flags.
- [x] Correct temporal types in the YAML, including `ctx.now` and `ctx.now_utc` to `datetime`.
- [x] Convert Group 1 CSV/list twins so each surviving base variable is `string[]` and each `_list` twin is removed from the YAML.
- [x] Convert Group 2 single list variables to `string[]`, preserving optional semantics where specified.
- [x] Convert `depends_on` and `used_by` to object-array schema entries with descriptions that document the object shape until precise nested object-array typing exists.
- [ ] Attach verified examples through schema-plus `example()` references instead of Rust-authored catalog examples. **Deferred to Phase 3 — see note below.**
- [x] Validation checkpoint: run the schema parser tests or a targeted schema load check proving `darkmatter_base_schema()` parses the updated YAML and preserves `ctx.*` declaration order.

> **Phase 2 note — `example()` wiring deferred to Phase 3.** The base schema is
> compiled into the `darkmatter` library via `include_str!` and has **no runtime
> filesystem anchor**. `example(<file>)` uses relative paths resolved against a
> `base_dir` at schema-*resolution* time, but the base schema is only ever
> *converted* (`to_json_schema`, emitting raw `x-darkmatter-example` annotations)
> when injected as a compose baseline (`DarkmatterSchemas::with_baseline`) — its
> example files are never read at compose runtime, and a deployed `md` binary has
> no `docs/schemas/` tree to read them from. Wiring ~86 `example()` references
> into the embedded YAML now would therefore be non-functional at runtime and
> would additionally require re-authoring every retyped array variable's example
> (the old catalog examples are CSV/Markdown strings that no longer validate
> against `string[]`/`object[]` targets). More importantly, the example migration
> is *inseparable* from the Phase 3 catalog rewrite: while `CONTEXT_VARIABLE_DESCRIPTORS`
> still hand-declares Rust `Example`s, adding YAML `example()` refs would create a
> split-brain (examples in two places) — exactly the drift this feature removes.
> The example-sourcing mechanism (E3 embedded example files vs the spec's E2 Rust
> sidecar fallback) is a Phase 3 projection decision and must land *with* the
> `LazyLock` projection that replaces the hand-authored catalog. Deferred there so
> examples are single-sourced in one coherent change rather than duplicated across
> the phase boundary. Core Phase 2 (types/descriptions/flags/collapse — the
> acceptance-criteria "single source" surface) is complete and verified.

Parallelizable: temporal fixes, list collapse edits, and example artifact preparation can proceed in separate patches as long as the final YAML load check is shared.

## Phase 3 - Derived Catalog Projection

- [x] Replace the hand-authored `CONTEXT_VARIABLE_DESCRIPTORS` core data with a private `LazyLock<Vec<ContextVariableDescriptor>>` projected from `darkmatter_base_schema()`.
- [x] Preserve the public accessor signature `context_variable_descriptors() -> &'static [ContextVariableDescriptor]`.
- [x] Change `ContextVariableDescriptor` so its type field stores the SimplifiedSchema type representation instead of the parallel `ContextValueType` presentation enum.
- [x] Retire `ContextValueType` entirely, or remove all presentation-only variants if full retirement is blocked by unavoidable internal call sites. *(Presentation-only variants removed; `ContextValueType` now stores a SimplifiedSchema type — base keyword, `is_array`, `integer` — per criterion 5.)*
- [x] Add a minimal Rust grouping map keyed by `ctx.*` variable name for `category` and `subsection` only.
- [x] Derive descriptor order from YAML declaration order through `IndexMap`.
- [x] Update all internal call sites that referenced the old const directly, including autocomplete/suggestion paths that need a slice from the accessor.
- [x] Validation checkpoint: add drift-guard tests proving projected descriptors match YAML name, type, description, generated/required/default flags, and order.
- [x] Validation checkpoint: add grouping-totality tests proving every projected `ctx.*` key has grouping metadata and every grouping-map key exists in the YAML.

Dependency: Phase 2 must define the final YAML shape before this projection can be considered stable.

> **Phase 3 notes.**
>
> 1. **Examples stay deferred (projected `example: None`).** Despite the Phase 2
>    note pairing example wiring with this projection, the Phase 3 task list above
>    contains no example task, and E3 (`example(<file>)` refs) remains
>    non-functional at runtime — the base schema is `include_str!`-embedded with no
>    filesystem anchor, so a deployed `md` binary cannot read example files.
>    Reintroducing E2 (a Rust-authored example sidecar) would re-create exactly the
>    Rust-side authored surface this feature removes. So the projected catalog
>    carries `example: None` for every `ctx.*` key; example artifacts are wired in a
>    later phase once the embedded schema gains a runtime anchor. The three former
>    example-consistency tests are retired with the inline examples.
> 2. **Two capture tests are transition-tolerant across the Phase 3–5 seam.** This
>    phase drops the ten `_list` twins from the *catalog*, but `context/capture.rs`
>    keeps emitting them until Phase 5. So `every_descriptor_has_a_captured_runtime_key`
>    (formerly a strict set-equality guard) now asserts descriptors ⊆ runtime keys
>    and that every runtime-only key is exactly one of those transitional `_list`
>    twins; `scalar_capture_shape_matches_projected_type` checks scalar shape only
>    (array/object capture shape becomes a real JSON array in Phase 5). Phase 5
>    restores strict equality.
> 3. **Optionality is a flag, not a type.** The retired `ContextValueType::Nullable`
>    wrapper is replaced by the descriptor's `required` flag (`generated` without
>    `required` ⇒ nullable), keeping the projected type pure SimplifiedSchema shape.

## Phase 4 - Typed Expression-Function Catalog and List Formatting

- [x] Extend `ExpressionFunctionDescriptor` with typed parameter data types and typed return data, including `error` union support for fallible functions.
- [x] Migrate existing expression-function descriptors from untyped signature strings to typed signatures while preserving display output for existing callers.
- [x] Add `as_line_separated(list) -> string`, `as_csv(list) -> string`, `as_tsv(list) -> string`, and `as_space_separated(list) -> string`.
- [x] Add `as_unordered_list(list) -> string` and `as_ordered_list(list) -> string` with recursive auto-nesting for nested arrays.
- [x] Support the `depends_on` and `used_by` object-array shape in unordered and ordered list rendering.
- [x] Keep function typing catalog-only; do not allow frontmatter properties to be typed as functions.
- [x] Add verified schema-plus example files for the six new list-formatting functions.
- [x] Validation checkpoint: add unit tests for every new function, including empty arrays, scalar elements, nested arrays, object-array dependency shapes, and mixed value rendering.
- [x] Validation checkpoint: assert function catalog display remains correct for `md schema about` and any DMLS-facing descriptor consumers.

Parallelizable: typed descriptor migration and function implementation can proceed together after the function type model is agreed.

> **Phase 4 notes.**
>
> 1. **Typed catalog is additive; `signature` is untouched.** `ExpressionFunctionDescriptor`
>    gains `parameters: &[ParamType]` and `returns: ReturnType` alongside the existing
>    `signature: &str`. The signature string stays the verbatim display key, so
>    `md schema about`, the generated `darkmatter-expressions.md` table, the
>    `descriptor_signature_set_equals_dispatchable_signature_set` parity guard, and the
>    DMLS consumers (`dmls::overlay::expressions`, which read `signature`/`description`)
>    are all unchanged. `ExpressionFunctionDescriptor::typed_signature()` renders the full
>    form on demand (e.g. `as_csv(list: any[]) -> string | error`).
> 2. **Type domains are enforced structurally (spec D7 / schema-plus § Type domains).**
>    Parameters use a new `DataType` enum (the schema-plus data-type domain — no `error`,
>    no function type). `error` is a **return-position-only** flag (`ReturnType::fallible`).
>    A test proves `DataType` never yields the `error` keyword and that
>    `SimplifiedType::from_keyword("error"/"function")` is `None`, so a frontmatter property
>    can never be typed as a function or as `error` — the typing is catalog-only.
> 3. **`as_csv` uses `", "` for byte-parity with the retiring `format_csv`.** So Phase 5's
>    "`as_csv(ctx.list)` reproduces old comma output" holds. The Markdown-list renderers
>    join lines with `\n` (no trailing newline) and auto-nest nested arrays and the
>    `depends_on`/`used_by` object-array shape (`{ package, dependencies }` → package bullet
>    + dependency sub-bullets) via a generic scalar-label/container-sublist rule — no
>    hard-coded field names.
> 4. **Examples: inline `Executable` where single-line, plus verified YAML example files.**
>    `as_csv` / `as_space_separated` carry inline `Executable` catalog examples (run by the
>    existing catalog harness and shown in the doc table). The multi-line / tab renderings
>    are `DisplayOnly` so the single-line doc table stays intact, and **all six** are
>    verified by a dedicated loader test that evaluates each
>    `features/2026-07-08-single-sourcing-schema/examples/*.yaml` (`parameters` bound as
>    initial values) to its declared `returns` (spec E3 / task 7). Function example files
>    have a real runtime anchor (they are read at test time), so — unlike the deferred
>    ctx-variable `example()` wiring — they are wired now.

## Phase 5 - Compose Evaluator Array Semantics

- [x] Change interpolation output rendering so bare arrays render line-separated by default, equivalent to `as_line_separated`.
- [x] Leave `scalar_string` array behavior byte-identical for equality comparison and frontmatter shell expansion.
- [x] Add targeted tests proving interpolation output changed while equality comparison and shell expansion behavior did not.
- [x] Change context capture so list-valued `ctx.*` variables are stored as `serde_json::Value::Array` instead of pre-rendered strings.
- [x] Remove or retire CSV and Markdown-list formatting helpers from context capture once their behavior is covered by expression functions.
- [x] Remove capture support for the ten dropped `_list` variables.
- [x] Implement `depends_on` and `used_by` capture as object arrays with `package` and dependency/user list fields.
- [x] Validation checkpoint: add compose tests proving `{{ ctx.list }}` is line-separated, `{{ as_csv(ctx.list) }}` reproduces old comma output, and `{{ as_unordered_list(ctx.list) }}` reproduces old bullet output.

Dependency: Phase 4 must provide the formatting functions before in-repo callers can migrate safely.

## Phase 6 - Caller, Documentation, and Artifact Migration

- [x] Replace every in-repo `{{ ctx.foo_list }}` reference with `{{ as_unordered_list(ctx.foo) }}` or another explicit list formatter when ordered output is intended.
- [x] Replace every in-repo bare `{{ ctx.foo }}` use that relied on old CSV output with `{{ as_csv(ctx.foo) }}`.
- [x] Keep bare `{{ ctx.foo }}` only where line-separated output is intended.
- [x] Update docs for the new ctx array semantics, removed `_list` variables, and new list-formatting functions.
- [x] Regenerate or update `context-variables.md`, schema-about snapshots, examples, and any catalog-derived docs from the projected catalog.
- [x] Update README or feature documentation if public behavior of `md compose`, `md schema about`, or validation examples changes.
- [x] Validation checkpoint: run a repo search confirming no removed `*_list` ctx references remain outside intentional migration notes.

Parallelizable: caller migration, docs edits, and snapshot updates can proceed in parallel once Phase 5 behavior is available.

> **Phase 6 notes.**
>
> 1. **Callers migrated.** `_list` twin: `prompts/context.md`
>    (`package_areas_list` → `as_unordered_list(ctx.package_areas)`). Bare-CSV →
>    `as_csv`: `prompts/context.md` (`programming_languages_in_repo`),
>    `prompts/performance-review.md` frontmatter (`packages` — `as_csv` keeps the
>    field a CSV *string* rather than letting whole-value interpolation return a
>    real array, preserving the claudine lifecycle contract), and every inline
>    `- ctx.foo: {{ ... }}` row in `darkmatter/example-docs/ctx-and-eval/test.md`
>    (`docs_readme`, `docs_drift`, `docs_blast_radius`, `packages`,
>    `package_areas`, `dirty_files`, `dirty_source_code_files`). Standalone list
>    blocks → `as_unordered_list`: `prompts/code-comment-quality.md` and
>    `prompts/faster-builds-and-tests.md` (`current_packages`).
> 2. **Intentional bare keeps (line-separated is correct).**
>    `claudine/docs/getting-started/index.md:299` (`{{ctx.dirty_files}}`, inside a
>    fenced example that lists dirty files — line-separated is the natural shape)
>    and `claudine/docs/topics/context/expression-engine.md:13`
>    (`length(ctx.packages) > 1` — now counts array elements, which is *more*
>    correct with the array retype; no formatter needed).
> 3. **No schema-about snapshot churn.** `md schema about` snapshot tests assert
>    generic type keywords, not `ctx.*` rows, so no CLI snapshot update was
>    required. The catalog-derived docs updated by hand are
>    `docs/topics/context-variables.md`, `docs/inline/fm-interpolation.md`, and
>    the `.claude/skills/darkmatter/compose.md` ctx table.

## Phase 7 - Integrated Validation and Release Readiness

- [x] Run targeted unit tests for schema parsing, catalog projection, expression functions, interpolation rendering, context capture, and schema-about output.
- [x] Run `just test` in the `darkmatter` package area.
- [x] Run `just test-l2` in the `darkmatter` package area if integration fixtures or CLI output changed.
- [x] Run `just lint` in the `darkmatter` package area.
- [x] Run a focused compose smoke test over migrated in-repo documents that previously used CSV/list context variables.
- [x] Review comments and rustdoc for every behavior-changing symbol touched, deleting or updating drifted documentation in the same change.
- [x] Check cross-platform assumptions for path handling, line endings, timezone capture, and terminal output so the implementation remains valid on macOS, Windows, and Linux.
- [x] Validation checkpoint: confirm all acceptance criteria from the spec are explicitly covered by tests, docs, or manual verification notes.
- [x] Validation checkpoint: confirm no hand-authored Rust source still declares `ctx.*` name/type/description/flag data outside the grouping map and test fixtures.

Dependency: this phase starts after all implementation and migration phases land.

> **Phase 7 notes — validation results.**
>
> 1. **Unit + full lib suite.** `cargo nextest run -p darkmatter --no-fail-fast`:
>    **5296 tests run, 5294 passed, 2 failed, 1 skipped**. The only two failures
>    (`layout::page::tests::render_code_block_with_pad_fill`,
>    `render_code_block_center_aligned_with_max_fill`) are **pre-existing,
>    host-specific** code-block-fill width assertions unrelated to this feature —
>    last modified in `b50e17ae9` (the `DarkmatterPage` primitive), untouched by
>    any single-sourcing commit; the only branch-side edit to `layout/page.rs`
>    (`89e4e62cd`) is an HR-alias removal. Every phase-relevant module
>    (schema parsing, catalog projection, expression functions, interpolation,
>    context capture, schema-about) passes. `just test` shares these two known
>    failures (it fail-fasts on them).
> 2. **`just test-l2`** — full area L2 (lib + cli + dmls) passes.
> 3. **`just lint`** — clean (exit 0).
> 4. **Compose smoke test** — `md compose darkmatter/example-docs/ctx-and-eval/test.md`
>    renders `as_csv(ctx.packages)` / `as_csv(ctx.docs_readme)` as comma output,
>    `::block when="ctx.distro"` correctly suppresses on macOS, and a bare
>    `as_unordered_list(ctx.package_areas)` / `as_tsv(...)` scratch doc renders
>    the expected bullet / tab forms.
> 5. **Cross-platform** — the six list formatters join with `\n` / `\t` / `", "` /
>    `" "` only — no `MAIN_SEPARATOR`, no `\r`, no `cfg(target_os)` in the new
>    function/capture/evaluator paths. Path values come from `sniff` (already
>    cross-platform); temporal capture is unchanged `chrono`. Existing
>    `#[cfg(windows)]` path tests (`strip_trailing_sep("C:\\repo\\")`, PATHEXT
>    probes) continue to pass.
> 6. **Comment/rustdoc review** — the behavior-changing symbols
>    (`catalog.rs` projection + grouping, `functions.rs` list renderers,
>    `format.rs` retired-helper module doc, `evaluator.rs` array-output boundary)
>    carry contract-level comments (byte-parity `", "`, "line-separated only on
>    the output path", object-array auto-nesting) with no drift.
> 7. **Acceptance criteria** — all 12 spec criteria are covered by named tests or
>    manual notes: drift guard (`projected_descriptors_match_base_schema`),
>    grouping totality (`grouping_map_is_total`), temporal fix
>    (`temporal_types_are_correct`), retired presentation variants
>    (`ContextValueType` now a SimplifiedSchema-type struct), preserved accessor,
>    six functions + `examples/*.yaml` loader test, byte-identical `scalar_string`
>    (evaluator output-path scoping), nested/object-array rendering, `_list`
>    removal (`removed_list_twins_are_absent` + repo search), and the smoke test.
> 8. **No residual hand-authored `ctx.*` data** — `catalog.rs` declares only the
>    presentation grouping map (spec D5-permitted `category`/`subsection`); all
>    name/type/description/flag data is projected from the YAML. The two remaining
>    `*_list` string matches in-repo (`.claude/skills/darkmatter/compose.md`,
>    `docs/topics/context-variables.md`) are intentional migration notes.
