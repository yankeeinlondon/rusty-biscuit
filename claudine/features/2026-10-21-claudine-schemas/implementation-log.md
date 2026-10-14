---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-21-claudine-schemas/spec.md
plan: claudine/features/2026-10-21-claudine-schemas/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: 
    - darkmatter/lib/src/markdown/schemas/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: 
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
source_files_during_phase_2: 
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
    - claudine/schemas/linking.yaml
    - claudine/schemas/claudine.yaml
    - claudine/schemas/inline-compose.yaml
    - claudine/schemas/partials/lifecycle.yaml
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/fixtures/claudine-schemas/
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/meta_schema_reference_graph.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
docs_updated_during_phase_2: 
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_2: 
    - claudine/schemas/README.md
skills_files_updated_during_phase_2: 
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/claudine/cli-commands.md
human_review: false
message_to_agent: |-
    Phase 3 is done; read "## Phase 3" in this log first. What Phase 4 needs:
    - Launch validation now loads trigger schemas (`claudine/lib/src/composition/schema/mod.rs::load_effective_schema`; embedded bundle in `schema/bundle.rs`). Docs still describe the old authored-only verdict and must change: `claudine/docs/topics/composition.md` launch-validation section (AC11), `frontmatter-properties.md:17` and `execution-flow.md:519` ("invalid optionals are dropped"), plus the R8 drift list from Phase 1. Describe the strict policy: an invalid Claudine control fails the launch; authored application optionals keep drop-and-warn; token ownership and supplied eager files still read the authored `$schema` only.
    - DMLS gap (found, not fixed): completion and hover read only the authored SimplifiedSchema (`effective.simplified`), so a document with no `$schema` gets validation from a trigger payload but no completion or hover from it. `darkmatter/docs/topics/schemas/dmls-schema-support.md` says the effective schema drives completion and hover; when you update that page (AC8), state the actual behavior (validation only for trigger payloads) unless the author rules the gap must be fixed first.
    - Consumers still loading the retiring Darkmatter copies, to migrate before deleting them: `darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs::with_ctx` (schema extension), `dmls/tests/l1/mapping_only_corpus.rs`, `dmls/src/providers/frontmatter/sequence_tests.rs`, `dmls/src/overlay/mod.rs` (~line 1501), `darkmatter/cli/tests/l1/schema_validate.rs::schema_validate_shipped_claudine_schema_rejects_an_empty_lifecycle_item`, and the module docs of `dmls/src/diagnostics/nested_span.rs`. Both inventory guards already read `claudine/schemas/`.
    - Size: the resolved payload is ~2.5 MB. Darkmatter now memoizes embedded payloads, merges, phase schemas and root-union arms, shares context-free validators, and the workspace `profile.test` optimizes jsonschema/regex/serde_json; see the darkmatter skill `schema.md`. A Claudine spawn still costs ~0.5 s more in debug; root-union + `file(match)` documents ~1 s more. Do not add per-call clones/compiles of an effective schema.
    - Diagnostic quality (found, not changed): a misspelled long-form key reports `/start/0/shell: {"cmd":…} is not valid under any of the schemas listed in the 'anyOf' keyword` plus an `is not of type "string"` line; `cmd` appears only in the echoed value. A Darkmatter union-narrowing fix changed 15 content-policy/DMLS diagnostics, so it was reverted; it needs its own change.
source_files_during_phase_3: 
    - Cargo.toml
    - claudine/lib/src/composition/schema/bundle.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/error/render/schema.rs
    - claudine/lib/src/composition/lifecycle/tests/nested_span.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_matrix.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/launch_trigger_schemas.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: 
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/darkmatter/schema.md
---
# Implementation Log for 2026-10-21-claudine-schemas (5 phases)

## Phase 1

Phase 1 is the contract phase: no schema file and no production code changed.
The only source change is two Darkmatter characterization tests (see
"Characterization" below). Everything else here is the frozen evidence the
later phases build on. File references are to the checkout as of 2026-10-13.

### Prerequisite gate

All four prerequisite specs report `implemented: true`.
`2026-10-20-lifecycle-grammar` is still active (in its review cycle), and the
working tree carries its uncommitted review fixes; none of those files were
touched here.

| Prerequisite | Status | Implementation | Focused evidence (all passed) |
| --- | --- | --- | --- |
| built-in `shell` / `expression` | present | `darkmatter/lib/src/markdown/schemas/simplified/types.rs:555,561`; `convert.rs:1534` | `schemas_core_type_values::{shell_accepts_any_non_blank_command_without_parsing_it, shell_scalars_coerce_to_text_like_expression, shell_constraints_and_defaults_mirror_expression}` (5/5) |
| non-null presence in trigger `match:` | **partial — see R1** | `triggers/lint.rs:44-93`, `triggers/matcher.rs`; union rejected at `triggers/grammar.rs:468-473` | `triggers::lint::tests::*`, `matcher::tests::{null_treated_as_absent, gate_required_absent_defeats, presence_follows_the_shared_effective_calculation}`, `discovery::tests::vacuous_trigger_is_hard_error` (17/17) |
| `required` through named references | present | `schemas/resolve.rs` named-type expansion | `schemas_named_type_merge::{required_and_generated_survive_every_reference_spelling, union_arms_hoist_inherited_required, …}` (5/5) |
| `@this` precedence, `name@./file.yaml` | present | `resolve.rs:1055-1110,1359` | `schemas_named_type_merge::a_bare_keyword_is_the_built_in_and_keyword_at_this_is_the_local_type`, `resolve::…::resolves_this_self_import` (5/5) |
| `[]` on a union type → `items.anyOf` | present | — | `schemas_union_postfix` (32/32) |
| `<string>` / `<pattern::RE>` keys | present | `simplified/convert.rs:656-850` | `convert::…::{catch_all_lowers_to_additional_properties, pattern_key_lowers_with_literal_precedence, …}` (7/7) |
| library / trigger envelopes; trigger-as-payload rejected | present | `triggers/envelope.rs:54`; `triggers/assemble.rs:229-330` | `envelope::tests::*`, `assemble::tests::{rejects_payload_referencing_another_trigger, rejects_root_union_payload, …}` (13/13) |
| lifecycle parser | present | `claudine/lib/src/composition/lifecycle/{parse,action_shape,actions,validate,signatures}.rs`; `MAX_CONDITIONAL_DEPTH = 5` at `parse.rs:67` | `tests::grammar::{conditionals_nest_five_levels_deep, a_sixth_level_is_a_typed_error_naming_its_path, …}`, `tests::action_shape_control::*` (13/13) |

### Rulings

**R1 — Trigger presence is spelled as nested `any:` arms, not a property-level
union.** Darkmatter's match grammar rejects a property-level union inside a
condition (`grammar.rs:468`: "may not be a property-level union; use nested
`any:` instead"), so the spec's literal example
`start: [string(required), number, boolean, object, "unknown[]"]` does not
load. `2026-10-16-core-schema-types` delivered the union for ordinary schema
properties only. Rule 1 forbids adding grammar here, and the grammar already
names the equivalent form, so each identifying property becomes five
single-type required conditions under `any:`:

```yaml
match:
  any:
    - start: string(required)
    - start: number(required)
    - start: boolean(required)
    - start: object(required)
    - start: "unknown[](required)"
    # … the same five for initialize, blocked, success, failure, finalize,
    # loop, prompt, agent, model
```

The inline trigger is `all: [ any: <five prompt arms>, none: <five sequence
arms> ]`. Verified with `target/debug/md schema triggers` on a temp repo:
`start` as string, number, list, mapping, or boolean selects; `start: null`,
`start:`, and `ticket: 1` alone do not; `prompt: x` selects inline;
`prompt: x` + `sequence: 42` is defeated by the `none:` child;
`prompt: x` + `sequence: null` selects inline. This is the spec's selection
semantics with a different spelling; it is a departure from the spec's
example text, recorded here and to be reflected in the docs (Phase 4).

**R2 — `types.<export>: null` defines the null type; it is not an error.** The
plan's library matrix says an explicit-null export definition errors. Since
`2026-10-16-core-schema-types`, a native YAML null at a type position is the
`null` type (accepts null and omission, rejects every non-null value). That
cell's outcome is corrected to the decided semantics: probe `null` accepted,
`42` rejected, and distinct from a missing export (which errors with
"`<name>` not found").

**R3 — Non-string top-level keys in a trigger envelope are silently
accepted.** `triggers/envelope.rs:69` collects unknown keys with
`filter_map(YamlValue::as_str)`, so `42: extra` passes. This is a silent-loss
defect in a reader this feature extends. It is fixed in Phase 2 by the
"Prepare origin support" owner, with its own matrix cell (below). The library
envelope already rejects the same key (`standalone.rs:137-170`, via
`map.len() != 2`), though with an empty key list in the message.

**R4 — Library syntax/duplicate errors surface generically.**
`simplified/standalone.rs:126-129` maps a YAML parse error to `Ok(None)`, so a
duplicate key or trailing garbage in an imported library reports
`AmbiguousReferenced` rather than the YAML detail. The required outcome
("visible error, no fallback") holds. Phase 2 may surface the YAML message on
the import path; it is not required for any acceptance criterion.

**R5 — Scalar coercion in validation.** Darkmatter validation coerces scalars to
text, so `model: 42` passes `model: string`. Every negative probe in the matrix
and the AC5 fixtures uses a non-coercible value (a list or mapping where a
string is declared). AC5's own probes are unaffected: `start: 42` and
`loop: 42` violate list/mapping types.

**R6 — `agent: []` is rejected by the payload.** The reader accepts an empty
agent list (`composition/hints.rs:79-118`), but AC5 requires the payload to
reject it. `agent` is typed `[string, string[](min(1))]` in `claudine.yaml`.
Because winning Claudine declarations fail launch strictly, an empty agent
list becomes a launch failure: an intended, spec-mandated narrowing. `model`
keeps the reader's forms (`[string, string[]]`; an empty list reads as
absent), since the spec names no stricter rule for it.

**R7 — Bare-name arm membership comes from the parser.** `parse_bare_verb`
(`lifecycle/parse.rs:640-690`) accepts any known verb whose arity check
accepts zero arguments. That is `stop`, `skip`, `error`, `retry`, and every
catalog function with a zero-argument overload. From the catalog data, the
candidates are `and`, `or`, `branch_exists_on_remote`, `ipv4`, `ipv6`, and
`remote_vendor`. The Phase 2 parity test derives the set by running the parser
over `all_lifecycle_verbs()` as bare items and compares it with the schema
enum, so the enum is never hand-guessed. `skip` stays in the enum for every
event; "skip only in initialize" is a parser rule.

**R8 — Frontmatter drift found while freezing the inventory.** The code is
taken as correct, so the payload follows the code. The page
(`claudine/docs/topics/frontmatter-properties.md`) is corrected in Phase 4:

- `timeout`, `step_timeout`, `timeout_warn`, `step_timeout_warn`,
  `exit_expressions`, and `guard_settings` are read by the code but missing
  from the page. They are typed in the payload, because the spec lists them.
- `argument-hint` and `max_turns` are missing from the page's linking table.
  The lifecycle table lacks `initialize` and `finalize`.
- The page's sequence-overlay keys (`previous_state`, `next_state`,
  `is_first`, `is_last`, `step`, `total_steps`) are retired
  (`sequence/model.rs:283-290`). The current overlay keys are
  `ROOT_OVERLAY_KEYS` (`sequence/reserved.rs:31`): `state`, `previous`,
  `next`, `outputs`, `sequence_id`.
  **Ruling:** declare these five as `unknown` with descriptions (hover and
  completion, no type constraint). Claudine injects them only into sequence
  steps; a standalone prompt using `state: draft` as application data works
  today, and a typed Claudine declaration would make it a strict launch
  failure, contradicting "public frontmatter stays open".
- `operation` and `yolo` are not read from document frontmatter (flag/env
  only; `operation` is a sequence task option). The spec lists both as run
  controls, so they are declared as `string` and `boolean` (the types the
  retiring Darkmatter copy used), and the docs say they are not document
  controls. `title` has no reader (dry-run reads `name`/`description`); it is
  declared `string` per the spec.
- `hash` can hold Darkmatter's `detailed` object form before closure
  normalizes it, so it is typed `[string, object]`.
- `verbosity` matching is trimmed and case-insensitive, and an invalid value is
  ignored. It is typed `enum(silent, quiet, verbose)`, so a casing variant now
  fails launch. **Flagged** for the Phase 3 owner: if any shipped or test
  prompt uses a non-lowercase spelling, use `string` with a description
  instead and log the choice.
- `guard_settings` numbers accept `0` in code; the retiring copy's `min(1)` is
  dropped (`min(0)` / plain integer).
- Several keys reject explicit null in code (`loop`, the four timeouts,
  top-level `fail_fast`, `prompt`). A schema cannot say "absent ok, null
  rejected" for an optional property under Darkmatter's optional-null
  semantics. The parser stays authoritative there.

**R9 — Additional retirement consumers not in the plan's list.**
`claudine/lib/src/composition/lifecycle/tests/nested_span.rs::single_pass_inventory_matches_the_authored_claudine_schema`
reads both retiring files (`../../darkmatter/docs/schemas`). It moves to
`claudine/schemas/` together with the DMLS inventory test. Basename-only
mentions of `claudine.yaml` in `darkmatter/lib/src/markdown/schemas/mod.rs`,
`triggers/assemble.rs`, and `dmls/tests/l1/schema_roots_parity.rs` are
synthetic temp-dir fixtures, not consumers. The fixture
`darkmatter/tests/fixtures/schema-triggers/schemas/claudine.yaml` is a generic
trigger fixture, not a retiring copy. `shipped_schema_corpus.rs:154-166`
registers exceptions for `claudine/schemas/claudine.yaml` (Placeholder) and
`partials/lifecycle.yaml`; those entries must change when Phase 2 replaces the
files, or that test fails.

**R10 — Historical references for AC14.** These name the retiring paths and are
not spec snapshots:
- `darkmatter/dmls/design/research/r3-yaml-parser-bakeoff.md` (dated design
  research)
- `features/_completed/2026-09-22-consolidated-test-binaries-wave-2/{baseline/test-input-probe-before.json, spikes/s2-scan-output.txt}`
  (captured tool output inside a completed feature directory)
- the implementation logs, plans, and reviews of completed features and fixes

**Ruling (to apply in Phase 4):** a feature/fix directory is a snapshot in its
entirety (its spec, plan, log, baselines, and spikes), so those are exempt. The
DMLS design-research note is a dated research record of the same nature, also
left unchanged. Every live file (source, test, config example, `docs/` page,
README, skill) must stop naming them. Phase 4 re-runs the search and lists the
final exempt set.

### Launch-validation seam (as found)

- **Trigger discovery is never enabled by Claudine.** Every verdict site loads
  through `schema::load_effective_schema`
  (`claudine/lib/src/composition/schema/mod.rs:466`), which builds
  `DarkmatterSchemas::new(ctx).with_caller_input_records(..)` without
  `with_trigger_discovery`/`with_trigger_registry`. Darkmatter compose's own
  stage (`compose/schema_validation.rs:191-245`) discovers triggers only when
  `ComposeOptions::with_trigger_schemas(true)`, which Claudine never sets.
- **Four authored-only gates** (`contains_key("$schema")` early returns):
  `schema/mod.rs:636` (`pre_validate_with_origin`), `schema/mod.rs:896`
  (`drop_invalid_optionals_with_origin`), `prepare.rs:455-475`
  (`resolve_launch_schema`), and `schema/mod.rs:430`
  (`authored_schema_parameters`, token ownership). `post_shell_validate`
  (`schema/mod.rs:231`) inherits the `prepare.rs` gate via
  `prepared.launch_schema`.
- **Drop/retry sites** (all decide by required/eager only; no layer
  provenance): compose-time `translate.rs:76-194` with
  `filter_droppable_invalid_optionals` (`translate.rs:409`); post-shell
  `schema/mod.rs:296-362`; pre-preflight `drop_invalid_optionals_with_origin`
  (`schema/mod.rs:885-1013`).
- **Order** (`cli/src/commands/compose/prep.rs::run_composition_inner`):
  snapshot → supplied eager files (`resolve_supplied_file_inputs`, before
  `initialize`) → pre-validation only when the document authors no
  `initialize` → `prepare_and_run_active_document` (schema stage `Validate` on
  first entry without `initialize`, else `DeferToStabilizedReread`) →
  bootstrap + `initialize` + `staged_boot::reread_and_audit` → 
  `prepare_document(Validate)` → `prepare_*_with_prompt` /
  `prepare_inline` → `resolve_launch_schema` → `translate_schema_failure` →
  `post_shell_validate`.
- **Retained schema:** `PreparedComposition.launch_schema: Option<LaunchSchema>`
  (`composition/types.rs:690,743-755`) → `HarnessPromptState` →
  `CompletionContext.launch_schema` (`completion.rs:150`).
- **Fresh-read boundaries:** retry/resume and proxy target go through
  `harness_orch/prompt.rs::materialize_harness_prompt` → `prepare_document`;
  sequence steps through `wrap/sequence/jit.rs:179-222`; the sequence document
  level through `commands/sequence.rs:417` (`drop_invalid_optionals` only).
  Loop iterations call `prepare_without_schema` (no verdict).
- **Snapshot for matching:** `effective_for(&Markdown)` matches against the
  Markdown's own frontmatter. Inline compose validates a
  `Markdown::with_frontmatter` temp (`prepare.rs:743`) that has no
  `ComposeSource::File`, so `$path` conditions would not match it, and
  discovery needs a document-scoped `FileResolutionContext`
  (`ctx.for_source(resolved_path)`), not the launch-scope one.
- **"Dropped" wording** to revisit in Phase 3: `schema/status_render.rs:38-42`,
  `cli/src/commands/schema_interactive/mod.rs:396-405`,
  `cli/src/commands/schema_interactive/status.rs:34`, and docs
  `composition.md:1042-1044`, `frontmatter-properties.md:17`,
  `execution-flow.md:519`.

### Agreed loader seam (contract for Phases 2–3)

Darkmatter already has an in-memory source for the shipped baseline:
`schemas/embedded.rs`, `resolve::prepare_embedded_schema` (`resolve.rs:890`),
`NamespaceKey::Embedded` / `NamespaceSource::Embedded` (`resolve.rs:1082,1139`),
and `SchemaSourceId::Embedded(String)` (`simplified/source.rs:94-106`). The seam
generalizes that; it adds no new engine or virtual filesystem.

1. **Inputs.** The host passes a bundle: a bundle name plus a slice of
   `(file name, &'static str text)` pairs. Claudine supplies five
   `include_str!` entries; Darkmatter defines no Claudine text.
2. **Source identity.** A trigger source becomes
   `File(PathBuf) | Embedded { bundle, name }` for `LoadedTrigger`,
   `TriggerLayer`, trace entries, and `ShadowedFile`. `SchemaOrigin` carries a
   source id (reusing `SchemaSourceId::{File, Embedded}`) instead of
   `uri: Option<PathBuf>`. An embedded identity is never a fabricated path:
   DMLS keeps linking only `File` origins
   (`dmls/src/diagnostics/frontmatter.rs:843-864`).
3. **Discovery order.** `scan` walks the five roots exactly as today. Then, for
   each bundle file whose **name** is not in the disk `first_by_name` set,
   embedded candidates are appended. A disk file of that name, whether
   matching, non-matching, non-trigger, or malformed, suppresses the embedded
   one. A malformed claimed disk trigger is a `TriggerLoad` error with no
   fallback.
4. **Import anchoring.** Disk triggers resolve imports with today's
   file-reference resolver, anchored at the envelope's directory.
   Embedded triggers resolve `$schema` and every relative import lexically
   inside the bundle (`embedded::resolve_path`), with no disk or roots
   fallback; a missing bundle name is an error. A disk import never resolves
   to a same-basename embedded file. The trigger-as-payload cycle check
   compares embedded names, not canonical paths.
5. **Ownership.** The winning declaration is the existing `origins[key]`
   (`build_origin_map_with_triggers`, `schemas/mod.rs:1682-1716`: document
   wins, otherwise the last declaring trigger, otherwise baseline). A property
   is **Claudine-owned** when its origin is `Trigger` and the trigger source is
   either the embedded Claudine bundle or a disk file whose name is one of the
   bundle's trigger names (`claudine.yaml`, `inline-compose.yaml`). An
   authored redeclaration flips the origin to `Document`/`ReferencedFile`, and
   with it ownership (pinned by
   `authored_redeclaration_takes_over_a_trigger_property_origin`). Claudine
   owns that classification; Darkmatter only reports origins.
6. **Root unions.** An authored root-union `$schema` gets the trigger layers
   merged into every arm, and `origins` is empty (pinned by
   `root_union_document_schema_merges_trigger_per_arm_without_origins`). No
   property is then Claudine-owned, so the existing cleanup policy applies
   unchanged. This is preserved; no per-arm ownership is invented.
7. **Dependencies.** Disk envelopes and payload files stay in
   `dependencies` as today. Embedded sources add none (they cannot change at
   run time), so DMLS invalidation is unaffected.
8. **Launch use.** `load_effective_schema` is the single substitution point:
   it gains trigger discovery with the embedded fallback, using a
   document-scoped context (`file_resolution_context.for_source(resolved_path)`),
   and all four authored-only gates are removed in the same change. Inline
   matching uses a file-backed `Markdown` built from the effective snapshot so
   `$path` keeps working. The strict policy replaces the required/eager-only
   drop decision at all three drop sites with "never drop a Claudine-owned
   property". The `LaunchSchema` retained for completion is that same
   effective schema.

### Characterization (source change in this phase)

Existing tests pin most contracts later phases must keep. A pre-change
baseline run of the Claudine schema, preparation, and completion suites
(`composition::schema::`, `composition::prepare`, `composition::completion`,
`compose_schema_cli`, `sequence_schema`, `completion_compose`,
`completion_inline_compose`, `completion_committed_prompt_schema`,
`inline_completion_lifecycle`, `compose_initialize_staged_boot`,
`single_pass_inventory`) gave **386/386 passed**. Notable owners:
`schema/tests.rs::{invalid_optional_is_dropped_and_retried, drop_invalid_optionals_skips_template_bearing_values, drop_invalid_optionals_keeps_optional_eager_file_failures}`,
`compose_schema_cli::{compose_invalid_optional_setter_is_dropped_and_run_succeeds, compose_invalid_optional_in_file_emits_visible_warning}`,
`completion/tests.rs` (frozen launch schema).

Two contracts had no explicit test, so two were added to
`darkmatter/lib/src/markdown/schemas/mod.rs` (`phase4_trigger_assembly`):

- `authored_redeclaration_takes_over_a_trigger_property_origin`: an authored
  `model: number` replaces the trigger's `model: string(required)`; the origin
  becomes `Document` and `model: 42` validates.
- `root_union_document_schema_merges_trigger_per_arm_without_origins`: with
  a two-arm authored root union, `origins` is empty, the root stays `anyOf`,
  the trigger's `model: string` rejects `model: [1]` in every arm, and a
  conforming document passes.

Both pass.

### Inventories (frozen)

**Hand verbs (parser).** communicate: `say`, `speak`, `effect`, `message`,
`notify`, `stderr`, `stdout`, `info`, `warn`, `success`
(`actions.rs:645`). flow: `stop`, `skip`, `error`, `proxy`, `retry`, `resume`,
`defer` (`signatures.rs:249`; `defer` parses, then fails at run time with
`DeferredExecutionUnsupported`). shell: `shell`. expression: `set`
(mapping only). Planned in schema only: `break`, `prep` (unknown to the
parser). The AC2 test asserts the schema minus the parser equals exactly
`{break, prep}`.

**Generated verbs.** Side effects: `darkmatter::effects::effect_descriptors()`,
13 unique names (`set`, `set_frontmatter`, `merge_frontmatter`,
`delete_frontmatter`, `increment_frontmatter`, `decrement_frontmatter`,
`append_frontmatter`, `prepend_frontmatter`, `ensure_file`, `ensure_dir`,
`append_line`, `append_jsonl`, `http_post`). `set` overlaps the hand-typed
`set` and is listed once, as the hand verb. Expression functions:
`darkmatter::markdown::compose::expression::expression_function_descriptors()`,
113 unique names. The descriptors have no `name` field, so the test splits each
`signature` at `(` and deduplicates overloads. The loop control `max` shares
its name with the `max` function: in the `loop` type, `max` is declared once,
as the iteration control.

**Payload (`claudine.yaml`), all optional:**

| Property | Type | Reader |
| --- | --- | --- |
| `prompt` | `string` | `prepare.rs:485-500` |
| `agent` | `[string, string[](min(1))]` (R6) | `hints.rs:79-118` |
| `model` | `[string, string[]]` | `hints.rs:125-157` |
| `interactive` | `boolean` | `hints.rs:159-169` |
| `$schema` | `[object, string, object[]]`; exact authored-form union verified in Phase 2 | Darkmatter |
| `last_updated` | `string` | `completion.rs:164` |
| `hash` | `[string, object]` (R8) | `closure.rs:220-226` |
| `fail_fast` | `boolean` | `sequence/mod.rs:122` |
| `sequence` | `[unknown[], string]` | `sequence/grammar.rs:89-139` |
| `loop` | `loop@./lifecycle.yaml` | `looping/config.rs:67-123` |
| `argv` | `string[]` | `ownership.rs:43` |
| `title`, `description` | `string` | `dry_run.rs:91` (description only) |
| `initialize` | `initialize-stack@./lifecycle.yaml` | `lifecycle/parse.rs:98-147` |
| `start`, `blocked`, `success`, `failure`, `finalize` | `stack@./lifecycle.yaml` | `lifecycle/parse.rs:94-117` |
| `timeout`, `step_timeout`, `timeout_warn`, `step_timeout_warn` | `string(not-empty)` (numbers are rejected by the reader) | `harness/parse/mod.rs:52-115` |
| `exit_expressions` | `[exit-expression[], exit-expression-layer]`; entry `{pattern, patterns, kind: enum(literal, regex), ignore_case, scope}` closed; layer `{mode: enum(merge, override), rules}` | `runaway/config.rs:84,190,364-376` |
| `guard_settings` | `{repetition: {enabled, max_repeats, max_cycle_length}, volume: {enabled, max_lines, max_bytes}}`, closed, integers ≥ 0 | `runaway/config.rs:391-446` |
| `operation` | `string` | not a document control (R8) |
| `yolo` | `boolean` | not a document control (R8) |
| `verbosity` | `enum(silent, quiet, verbose)` (R8 flag) | `render/prompt/frontmatter.rs:14` |
| `mode` | `enum(append, replace)` | `system_prompt/prepare.rs:67` |
| `state`, `previous`, `next`, `outputs`, `sequence_id` | `unknown`, described (R8) | `sequence/reserved.rs:31` |
| linking keys | imported from `linking.yaml` | below |

**Linking (`linking.yaml`).** The linking reader keeps frontmatter as untyped
YAML and stringifies every value (`linking/compatibility/properties.rs:145-186`),
falling back to line-wise strings when YAML fails to parse
(`frontmatter_io.rs:66-85`). Types follow what providers write: `name: string`;
`allowed-tools: [string, string[]]`; `tools: [string, string[], object]`;
`skills: [string, string[]]`; `license: string`;
`compatibility: [string, object]`; `metadata: "{ <string>: unknown }"` (open);
`user-invocable: boolean`; `disable-model-invocation: boolean`;
`argument-hint: [string, string[]]` (`[msg]` parses as a list);
`max_turns: number(integer)`.

**Loop.** `while` / `until: expression`; `action` / `actions`:
`[string, loop-action, (string | loop-action)[]]`, where `loop-action` is
`{op: enum(increment, decrement, set, append, prepend, merge)(required),
prop: string(not-empty; required), value: unknown}`, closed;
`max: number(integer; min(1))` (a string is rejected); `fail_fast: boolean`;
`on_rate_limit: enum(pause, abort, continue)`; `gate: stack`; plus the loose
verb keys of all four categories and `no_error` (`LOOP_ITERATION_KEYS`,
`lifecycle/parse.rs:71-79`). The XOR rules (`while`/`until`,
`action`/`actions`, gate vs loose verbs, no root `when`) stay parser-only.

**Inline payload (`inline-compose.yaml`).** inline-compose reads only `prompt`,
`sequence` (the mismatch check), the closure-owned `hash` and `last_updated`,
and `$schema`. The payload declares `prompt`, `hash`, and `last_updated`, typed
identically to `claudine.yaml`. `sequence` appears only in the `none:`
condition. A Phase 2 test asserts every key declared in both payloads has an
identical lowered type.

### Input robustness matrix (finalized)

Both syntaxes (block YAML, and JSON syntax inside a `.yaml` file) were
confirmed accepted by the reader, and both reject duplicates and trailing
content. Observed outcomes match the plan's tables except where noted.

Library envelope (via an import):

| Shape | `kind` | `types` | `types.<export>` |
| --- | --- | --- | --- |
| Control | matches plan | matches | matches |
| Absent | matches | matches (message lists no keys; acceptable) | matches |
| Null | matches | matches | **R2:** null type; accepts `null`, rejects `42` |
| Wrong type, whole | matches | matches | matches |
| Wrong type, one / every element | matches | matches | matches |
| Empty | matches | matches | matches |
| Duplicate | matches (R4 generic message) | matches (R4) | matches (R4) |
| Trailing | matches | matches | matches |

Trigger envelope: every cell matches the plan, plus one added cell:

| Extra shape | Field | Required outcome | Today |
| --- | --- | --- | --- |
| non-string top-level key (`42: extra`) | envelope | Error, "unknown key" | **accepted (R3); fixed in Phase 2** |

**Nested columns** added for the fields that changed code traverses. Each
walks the same eight shapes in the Phase 2 matrix test, through the public
validation result, with one probe pair per column:

| Column | Positive probe | Violating probe |
| --- | --- | --- |
| each `action.yaml` export (`communicate`, `flow`, `shell`, `expression`, `bundle`, `initialize-bundle`) | `start: [{info: hi}]` | `start: [{inf0: hi}]` |
| each `lifecycle.yaml` export (`stack`, `initialize-stack`, `loop`) | depth-3 valid stack | `initialize: [{shell: x}]` |
| `linking.yaml` exports | `allowed-tools: [Read]` | `user-invocable: [1]` |
| trigger condition operand (`start` arm) | `start: hi` selects | `start: null` does not select |
| trigger combinator (`none:` in inline) | `prompt: x` selects | `prompt: x` + `sequence: 42` does not |
| payload property definition (`timeout`) | `timeout: "30s"` | `timeout: [1]` |

The import cells use the shipped bundle with one source edit per cell, run for
disk and embedded origins. Syntax-invalid embedded cells use test-provided
text through the same seam, never a second production bundle.

### AC ledger

| AC | Evidence (planned) | Owner / phase | Target |
| --- | --- | --- | --- |
| 1 | five files load through `md schema validate` and the loader; `partials/` gone | Phase 2 Wave 3 | `darkmatter-cli` L1 + `claudine` lib L1 |
| 2 | hand-verb parity (planned-only = `{break, prep}`); generated keys = both catalogs | Phase 2 Wave 2 | `claudine/lib/tests/l1` |
| 3 | three typed levels for `stack`, `initialize-stack`, `loop`; loose below | Phase 2 Wave 3 | `claudine/lib/tests/l1` |
| 4 | bundle key set = union + `when`/`no_error`; short-form equality; closure; no catch-all | Phase 2 Wave 2 | `claudine/lib/tests/l1` |
| 5 | fixture suite via `md schema validate` and `with_trigger_discovery` → `effective_for` → `validate` | Phase 2 Wave 3 | `claudine/lib/tests/fixtures` + l1; `claudine/cli` l1 for `md` |
| 6 | `include_str!` byte identity; kinds; all imports resolve with no disk bundle, CWD outside the checkout | Phase 3 Wave 5 | `claudine/lib/tests/l1` |
| 7 | DMLS inventory test moved; trigger picked up under a fixture Claudine area | Phase 3 Wave 6 | `darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs`, dmls l1 |
| 8 | docs pages and `schemas/README.md` | Phase 4 Wave 7 | docs |
| 9 | single measurement logged | Phase 2 Wave 4 | log only |
| 10 | CLI L1 through `compose` / `inline-compose` with fake providers | Phase 3 Wave 6 | `claudine/cli/tests/l1` (`compose_schema_cli`, `sequence_schema`, completion) |
| 11 | `composition.md` launch-validation section | Phase 4 | docs |
| 12 | `just test` / `just lint` in claudine and darkmatter | Phase 5 | — |
| 13 | location / shadowing / symlink / embedded-vs-disk fixtures | Phase 3 Wave 5 | `darkmatter/lib` unit + `claudine/lib/tests/l1` |
| 14 | old files deleted; search clean except R10 exemptions; `mapping_only_corpus` | Phase 4 | `darkmatter/dmls/tests/l1` |

New tests register in the existing consolidated binaries (`claudine` lib
`tests/l1/main.rs`, `claudine-cli` `tests/l1/main.rs`, Darkmatter lib/CLI/DMLS
`tests/l1`). No new binary, tier, or CI cell is needed.

### Phase 1 verification

- `cargo nextest run -p darkmatter --lib -E 'test(/phase4_trigger_assembly::(authored_redeclaration|root_union_document|origins_attribute)/)'`: 3/3 passed.
- Claudine schema/preparation/completion baseline (filter above): 386/386 passed.
- `just test` in `claudine/`: 8885 passed, 9 skipped, 0 failed.
- `just lint` in `claudine/`: clean.
- `cargo clippy -p darkmatter --all-targets -- -D warnings`: clean (the only
  touched crate outside Claudine).
- OS: the two new tests use temp dirs and plain file writes, with no path
  comparison, symlinks, or process spawns. macOS evidence only; Linux,
  Windows, and WSL2 are left to the normal CI schedule, with no identified
  OS risk.

## Phase 2

Phase 2 authored the five-file disk bundle, proved it against the parser and
Darkmatter's catalogs, added Darkmatter's generic embedded trigger bundle
seam, and took the one AC9 measurement. Claudine's launch verdict does not
load the bundle yet; that is Phase 3. Work was split into two parallel owners
(the Darkmatter resolver seam; the YAML bundle and its tests) with disjoint
files, then integrated and verified here.

### What landed

- **Bundle** (`claudine/schemas/`): `action.yaml`, `lifecycle.yaml`, and
  `linking.yaml` (`kind: schema`); `claudine.yaml` (replaced) and
  `inline-compose.yaml` (`kind: trigger-schema`); `README.md` with the file
  classification. `partials/lifecycle.yaml` and `partials/` are deleted;
  `review.yaml` is untouched. A repository-wide search for `partials/lifecycle`
  finds no live reference. The unrelated `claudine/docs/schemas/partials/lifecycle.yaml`
  exception in the shipped corpus is a different file.
- **Category types** are closed objects of their verbs with
  `min-keys: 1; max-keys: 1`, so a single-verb item matches its category and a
  multi-verb mapping must match the short-form bundle. Bundles have no
  `min-keys`, so `start: {}` (an empty stack to the parser) stays valid.
  The generated verbs are listed as `unknown` literal keys: 12 side effects (set
  dedupes to the hand verb) and 113 expression functions. The bare-verb enum
  is `stop, skip, error, retry, and, or, branch_exists_on_remote, ipv4, ipv6,
  remote_vendor`, derived by the parity test from the parser (R7 confirmed).
  Two helper exports: `interpolation` (a whole-value `{{ … }}` string) and
  `count` (`[number(integer; min(0)), interpolation]`).
- **Lifecycle**: `stack`/`item-N`/`conditional-N` for N = 1..3, the
  `initialize-*` family with no `shell` arm or bundle key, `loose-body`
  (`[unknown[], "{ <string>: unknown }"]`) inside level-3 conditionals, and
  `loop` (controls, `gate: stack`, loose short-form verb keys, `no_error`, no
  `when`; `max` declared once as the loop control).
- **Trigger selection** follows R1: 50 single-type `(required)` arms under
  `any:` in `claudine.yaml`, and `all: [any: <prompt arms>, none: <sequence
  arms>]` in `inline-compose.yaml`.
- **Darkmatter seam (generic, no Claudine text)**:
  - The API: `EmbeddedSchemaBundle`, `DarkmatterSchemas::with_embedded_trigger_bundle`,
    `triggers::scan_with_embedded`, and `TriggerRegistry::append_embedded_bundle`.
    `TriggerSource::{File, Embedded { bundle, name }}` is the identity on
    loaded triggers and traces. `SchemaOrigin { kind, source: Option<SchemaSourceId> }`
    replaces `uri`, read through `file()` (real files only) and
    `trigger_source()`.
  - Errors: `SchemaError::EmbeddedTriggerLoad` and `EmbeddedBundle`.
  - DMLS still links only `ReferencedFile` origins. `md schema triggers` lists
    an embedded trigger as `embedded:{bundle}/{name}`.
  - Bundle files load in file-name byte order after every disk trigger, and
    are skipped when any root listed a same-name file. That skipping is not
    recorded in `ShadowedFile`: recording it would mean reading files left
    unread on purpose.
- **R3 fixed** (`envelope.rs`): a non-string top-level key is an unknown-key
  error that shows the key's YAML text.
- **R4 done**: a syntax-invalid imported library now reports `not valid YAML:
  …`, on disk and embedded, instead of `AmbiguousReferenced`.
  `schemas_constraints_block_matrix` was updated to recognize syntax errors by
  the new wording.
- **Empty match condition** (new finding, fixed): `{}` inside a combinator
  parsed as `all([])`, which is always true, so `none: [{}]` made a trigger
  unmatchable and `all: [{}, …]` hid a dropped condition. `grammar.rs::parse_match_expr`
  now rejects an empty condition mapping at every level. The top-level
  `match: {}` previously failed as vacuous and now fails with this message
  (`schema_roots::path_field_input_matrix` fragment updated). Documented in
  `definition.md`.

### Departures and rulings in this phase

- **D1: payload types that differ from the Phase 1 table.**
  - `last_updated: date` replaces `string`. A trigger declaration replaces the
    baseline's declaration of the same name, so `string` would have weakened
    the baseline's `date`.
  - `hash` mirrors the baseline's longhand union, which already covers the
    detailed object form.
  - `$schema` is declared as `schema`, matching the baseline. Darkmatter
    strips `$schema` before validating (`frontmatter_as_json`), so this only
    affects hover and completion.
- **D2: the R8 `verbosity` flag is resolved to `string(suggest(silent, quiet,
  verbose))`.** The reader is trimmed and case-insensitive, and its unit tests
  use `Quiet` and `SILENT`; an enum would turn those into launch failures.
- **D3: `proxy`/`prep` targets are `string`, not `file`.** `file` rejects
  `{{ ctx.next }}`; checked with `md`. Retry/resume counts use `count`, because
  the docs show `max_attempts: "{{ retries }}"`.
- **D4: the schema is stricter than the parser in these places.**
  - A single long-form verb with a sibling `no_error`.
  - A single-verb long form as a root, `then`/`else`, or loop dictionary. The
    spec decides this.
  - One-argument verbs written as one-element lists (`say: [hi]`).
  - Non-integer or negative counts.
  - `agent: []` (R6).

  Each becomes a launch failure once Phase 3 applies the strict policy. A
  `md schema validate` sweep of 66 real prompts (claudine docs, prompts, and
  test fixtures; the repository root, `darkmatter/prompts`, and
  `.claude/commands` through `SCHEMAS_DIR`) hit none of them. The only payload
  rejection was `prompts/fix.md`, which uses the removed `action:` grammar the
  parser also refuses.
- **D5: the schema is looser than the parser in these places, and the parser
  stays authoritative.**
  - Everything below level 3.
  - `say: null`/`proxy: null`, because optional properties accept null.
  - `when` on a root dictionary; a `when` item with only `no_error`; `- {}`
    and empty bodies.
  - `set` with a sibling `no_error`.
  - Ordering, cardinality, and `say` beside `speak`.
  - `skip` outside `initialize`; `gate` beside loose keys.
  - Unknown effect names and generated-verb values.
  - Numeric timeouts, because scalars are coerced (R5).
- **D6: spike fixture corrections.** The fixtures now live in
  `claudine/lib/tests/fixtures/claudine-schemas/`, 32 files.
  - In `root-dictionary`, `effect: success` became `crowd-applause`; in
    `loop-gate`, `tick` became `click`. Both original names fail the parser's
    effect catalog.
  - In `long-forms`, `retry` moved last, as a directive must be, and a `shell`
    long form was added.
  - `wrong-verb.md` is now `misspelled-verb-in-then.md`.
  - The malformed-sequence probe uses a mapping, because `sequence: 42`
    coerces (R5).
- **D7: JSON-syntax bundles load only on one line.** A pretty-printed JSON
  document inside a `.yaml` file fails with "could not project
  SimplifiedSchema expression spans through YAML source". This is the strict
  span locator's existing multi-line flow-collection boundary, so the JSON
  rows of both matrices use single-line JSON. This is a bounded finding, not
  changed here.

### Requirement-to-test map

The test files are `claudine/lib/tests/l1/claudine_schemas.rs` (CS),
`claudine/lib/tests/l1/claudine_schemas_matrix.rs` (CM),
`darkmatter/cli/tests/l1/claudine_schema_bundle.rs` (DC), and
`darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs` (EB). All are
registered in the existing consolidated `l1` binaries.

| Requirement | Tests |
| --- | --- |
| AC1: five files load; `partials/` gone | CS `every_file_loads_and_every_export_resolves`; DC `claudine_bundle_loads_and_both_triggers_are_discovered`; `shipped_schema_corpus` (new `Class::Trigger` entries for both triggers; four files added to `CORE_TYPE_USERS`) |
| AC2: hand verbs = parser + exactly `{break, prep}`; generated keys = both catalogs; bare-verb enum | CS `hand_verbs_equal_the_parser_inventory_plus_the_planned_verbs`, `generated_verb_keys_equal_the_darkmatter_catalogs`, `bare_verb_enum_equals_what_the_parser_accepts_as_a_bare_item` |
| AC3: three typed levels, loose below | CS `event_stacks_are_typed_three_levels_deep_and_loose_below`, `initialize_stacks_are_typed_three_levels_deep_without_shell`, `loop_gate_is_typed_three_levels_deep_and_loose_below` |
| AC4: bundle keys and values; closure; no catch-all; required through references | CS `bundle_keys_are_the_union_of_their_categories_plus_when_and_no_error`, `bundle_values_are_the_category_short_forms`, `action_objects_are_closed_and_data_maps_stay_open`, `required_long_form_fields_survive_every_reference` |
| AC5: every table row, through `with_trigger_discovery` → `effective_for` → `validate`, with selection asserted separately and instance path plus key asserted; parser-only rows also run through `parse_lifecycle_config` | CS `ac5_depths_and_stack_shapes`, `ac5_shell_placement`, `ac5_malformed_actions`, `ac5_trigger_selection`, `ac5_groups_name_every_fixture_once` (guard), `linked_prompt_declares_every_linking_key_and_keeps_the_root_open` |
| AC5 through `md schema validate` | DC `lifecycle_defects_are_reported_at_their_event_and_key`, `malformed_identifying_values_select_the_trigger_and_unrelated_data_does_not`, `parser_only_defects_pass_schema_validation` |
| Shared payload types equal; linking reader forms accepted | CS `shared_payload_properties_have_identical_types`, `linking_types_accept_every_reader_form` |
| Disk-origin input matrix: 18 columns × 8 shapes plus a control, block YAML and one-line JSON | CM `matrix_tests!` (36 tests), `matrix_tests_cover_the_table` (guard), `json_syntax_bundle_is_the_shipped_bundle`, `a_non_string_top_level_trigger_key_is_an_error` (R3 disk cell) |
| Embedded seam: import anchoring, suppression, errors, origins, dependencies, trace | EB: 15 behavior tests (see below) |
| Embedded-origin trigger envelope matrix, both syntaxes, plus the R3 cell | EB `the_trigger_envelope_matrix_holds_for_block_yaml`, `the_trigger_envelope_matrix_holds_for_json_syntax` (29 cells each) |
| R3 unit | `triggers::envelope::tests::rejects_non_string_top_level_keys_by_their_yaml_text` |
| Empty match condition | `triggers::grammar::tests::rejects_an_empty_condition_mapping_at_every_level`; CM `*_trigger_combinator` Absent cell |
| Bundle name and path rules | `embedded::tests::{bundle_paths_resolve_inside_the_bundle_only, bundle_checks_reject_unusable_names, bundle_debug_names_files_but_not_their_text}` |

The EB behavior tests:

- `an_embedded_trigger_applies_with_its_imports_resolved_inside_the_bundle`
- `registering_the_bundle_before_discovery_gives_the_same_registry`
- `a_string_payload_names_a_bundle_document`
- `a_payload_naming_a_trigger_is_rejected_inside_the_bundle`
- `a_same_name_matching_disk_trigger_replaces_the_embedded_one`
- `a_same_name_disk_trigger_that_does_not_match_still_suppresses_the_embedded_one`
- `a_same_name_disk_file_that_is_not_a_trigger_suppresses_the_embedded_one`
- `a_same_name_malformed_disk_trigger_is_an_error_with_no_embedded_fallback`
- `a_disk_trigger_never_imports_from_the_bundle`
- `embedded_imports_that_miss_or_leave_the_bundle_are_errors`
- `a_syntax_invalid_embedded_library_reports_its_yaml_error`
- `an_unusable_bundle_shape_is_rejected_when_registered`
- `origins_name_the_embedded_or_disk_trigger_that_declared_each_property`
- `an_embedded_trigger_adds_no_dependencies`
- `the_trigger_trace_lists_an_embedded_trigger_distinctly`

### Input matrix outcomes

- **Embedded origin** (EB, synthetic text): every cell matches the plan's
  trigger table in both syntaxes. Empty `match` (`{}` or `[]`) is an error;
  `$schema: {}` applies with no constraints; `""`/`[]` payloads are errors;
  the non-string key is an error.
- **Disk origin** (CM, shipped bundle): every cell is asserted. Where a cell
  departs from the plan's generic outcome, the reason is one of these:
  - **R2:** a null export defines the `null` type and rejects both probes. For
    `initialize-stack: null`, only the violating `initialize` probe uses it,
    so the control verdicts hold.
  - **Empty export map:** `{}` defines an empty closed object, so the probes
    pass through the other union arms.
  - **Payload property:** removing a property removes only its constraint.
  - **`kind`:** a wrong or missing `kind` leaves the file unclassified, so it
    contributes nothing.
  - **Combinator:** the Absent cell is now `Error`, after the empty-condition
    fix.

### AC9 measurement (single sample)

- Harness: a scratch crate outside the repository,
  `/tmp/claudine-schema-measure`, path-depending on `darkmatter` and
  `biscuit-file`, built `--release`. It parses `claudine.yaml`, takes its
  `$schema` payload, calls `darkmatter::markdown::schemas::resolve::resolve_schema(&payload,
  schemas_dir, &FileResolutionContext::new(schemas_dir))` once, and
  serializes `json_schema`. The spike's `size` mode was not used, because it
  builds synthetic event properties.
- Command: `target/release/claudine-schema-measure $PWD/claudine/schemas`.
- Host: Apple M4 Max, macOS 27.2.
- Revision: HEAD `816cdff3a` plus this phase's working tree (bundle files as
  above; `claudine.yaml` sha1 `55e7c1cc`).
- Scope: the full `claudine.yaml` payload with every action, lifecycle, and
  linking import resolved, plus serialization of the resolved JSON Schema.
  Validation and trigger discovery are not included.
- Result: **resolved_bytes=2,497,899; resolve_ms=48.7**.
- Finding (bounded): the resolved size is about 1.8× the spike's 1.4 MB
  catch-all figure, because of the closed literal keys repeated per typed
  level. Resolution is faster than the spike's 82 ms. The decided shape is
  kept. No timing test or benchmark was added.

### Phase 2 verification

- `cd claudine && just test`: 8943 passed, 9 skipped, 0 failed.
- `cd claudine && just lint`: exit 0. The only warning is the macOS linker's
  `__eh_frame section too large` note, a toolchain note on the large
  `claudine` binary.
- `cd darkmatter && just test` (lib, CLI, DMLS): 9978 passed, 10 skipped, 0
  failed. The first run failed only `schema_roots::path_field_input_matrix`,
  whose fragment changed with the empty-condition fix; it was updated and the
  rerun was green.
- `cd darkmatter && just lint`: clean.
- Slowest new L1 tests: about 1.7 s serially and 2.3 s at `-j4` on this Mac,
  within the 5 s budget but worth watching on 2-core runners.
- OS: embedded names are joined lexically with `/` and never canonicalized.
  The tests use temp-dir fixtures and no `#[cfg]` gating, and introduce no
  path comparisons. Evidence is from macOS only. Linux, Windows, and WSL2 are
  left to the normal CI schedule; no OS-specific risk was identified.
- No `cargo fmt`, commit, or CI change.

## Phase 3

Phase 3 embedded the bundle in Claudine and made the launch verdict load
trigger schemas. Wave 5 (the loader, ordering, and verdict integration) was
done by the integrator; Wave 6 and the Wave 5 fixture suites were written by
three parallel test owners (Claudine lib, Claudine CLI, DMLS) on disjoint
files, then integrated and verified here.

### What landed

- **Embedded bundle** (`claudine/lib/src/composition/schema/bundle.rs`):
  `CLAUDINE_SCHEMA_FILES` is `include_str!` of the five files,
  `claudine_schema_bundle()` the `EmbeddedSchemaBundle` named `claudine`, and
  `is_claudine_owned(effective, property)` the ownership rule from the Phase 1
  seam: origin kind `Trigger`, and the source is the embedded `claudine` bundle's
  `claudine.yaml`/`inline-compose.yaml` or a disk file of one of those names.
- **One effective-schema path** (`schema/mod.rs::load_effective_schema`): it
  discovers from the document's context
  (`file_resolution_context.for_source(resolved_path)`), registers the bundle,
  then validates under the launch context (`with_file_resolution_context`), so
  file values resolve exactly as before. Triggers match the snapshot being
  judged (`matching_markdown`: the source's own `$schema` and file, carrying the
  judged values): raw frontmatter plus overrides for pre-validation and the drop
  pass, the composed `effective_frontmatter` for the retained launch schema.
- **Gates removed**: `pre_validate_with_origin`, `drop_invalid_optionals_with_origin`,
  and `prepare.rs::resolve_launch_schema` no longer return early without
  `$schema`. `post_shell_validate` no longer skips a schema with no
  SimplifiedSchema; for an authored raw JSON Schema it judges only
  trigger-declared properties (Darkmatter's compose stage already applied the
  raw schema).
- **Kept authored-only** (`load_authored_schema`): `authored_schema_parameters`
  (argument-token ownership) and `schema::supplied` (caller eager files). These
  are about the author's own parameters, not the launch verdict. Including
  triggers would have turned `model=x` argument tokens into setters, changing
  valid launches.
- **Strict policy**: `classify.rs::categorize_problems` treats a Claudine-owned
  problem as hard at every phase, and `drop_invalid_optionals_with_origin` never
  drops one. Undeclared-in-shape properties were already conservatively hard, so
  this mostly makes the rule explicit. An authored redeclaration flips ownership
  and restores the authored cleanup.
- **Retired spellings** (`prefer_removed_grammar`): a schema verdict that fails
  on a document whose lifecycle uses removed grammar reports the parser's
  `RemovedGrammar` (with its replacement) instead. Without this, the 21
  `compose_removed_grammar` CLI tests regressed to a bare schema mismatch.
- **Completion rendering** (`error/render/schema.rs`): the completion block now
  lists, under **Problems:**, every problem on a property absent from the
  authored status table. For a document with no `$schema` it says "its schema"
  and drops the "raw JSON Schema" placeholder, which previously named neither
  the property nor the pointer (found by the CLI test owner). With an authored
  `$schema` the existing wording and table are unchanged. The hint now covers
  invalid present values too.
- **Comments**: module docs of `schema/mod.rs`, `resolve_launch_schema`, and
  `emit_dropped_optional_warnings` updated for the trigger layers and the
  strict policy. No terminal component changed; the new list uses `Prose`.

### Performance (bounded finding, fixed within scope)

Wiring the bundle in first made CLI L1 tests 10–20× slower (for example
`handoff_owners::a_chain_past_the_hop_limit…` 0.7 s → 13.4 s,
`sequence_prompt_property::…` 0.3 s → 3.8 s), because the resolved payload is
~2.5 MB and every verdict cloned, hashed, or recompiled it. A baseline was taken
by temporarily routing `load_effective_schema` to the authored-only loader.
Fixes, all generic Darkmatter changes with unchanged verdicts:

- Embedded trigger loads are memoized per process
  (`discovery.rs::load_embedded_trigger`), keyed by the identity of every text
  in the bundle (a payload reads the libraries too; the first key, trigger text
  only, was stale for an edited library, as the lib test owner noticed). Only
  successes are kept.
- `ResolvedPayload.json_schema` and `TriggerLayer.json_schema` are
  `Arc<Value>`. A lone trigger layer is shared, and merges are memoized
  (`mod.rs::merged_trigger_schema`, Arc identity plus authored-schema equality).
- `ValidatorCache` compiles a schema that names no `darkmatter-file*` format
  and no `x-darkmatter-match` once for all contexts (the structural slot)
  (`validate.rs::SchemaDigest`). The test
  `cache_keeps_contexts_sharing_a_key_without_recompiling` now uses a
  file-format schema, since its subject is file-judging schemas.
- `validate_for_phase` reuses a memoized phase schema (`phase_json_schema`) and
  the shared validator cache, instead of an uncached compile per call. When
  projection and `make_passive` change nothing, it shares the effective schema
  itself.
- A root union compiles only its arms (`CompiledValidators::RootUnion`; the
  union root validator was never used to judge), via memoized wrapped arms.
  `EffectiveSchema::validator()` now returns `Option` (no callers outside a
  spike).
- `coerce_property_union` skips validating arms when no arm changes the value.
  Before, it serialized each 100 KB+ arm on every validate.
- Workspace `Cargo.toml`: `profile.test` builds `jsonschema`, `referencing`,
  `regex-automata`, `regex-syntax`, `fancy-regex`, `ahash`, and `serde_json`
  at `opt-level = 3`, following the existing tree-sitter precedent. Measured
  on the root-union CLI tests: 4.25 s → 2.7 s.

The overhead left on this Mac (debug) is about 0.5 s per `claudine` spawn for a
document with Claudine controls (once-per-process payload resolution ~180 ms,
plus one or two compiles), and about 1 s for root-union `file(match)`
documents, whose arms are file-judging and context-keyed. Under full parallel
local load, four root-union/caller-file tests were flagged SLOW (>5 s) in one
run and none in the next; serially they take 1.7–2.7 s. The real lever is the
payload's size: Darkmatter inlines every named type, so `stack` appears about
seven times. Emitting shared types once is an engine change outside this
feature; it is recorded here, as AC9's bounded-finding rule asks, rather than
changing the decided shape.

### Departures and findings

- **D8: a pre-existing assertion was updated.**
  `prepare/tests.rs::inline_composition_uses_effective_frontmatter` asserted
  `launch_schema.is_none()` for a document with no `$schema`. That is now
  intentionally `Some` (the Claudine payload), so the assertion checks that
  instead. No other existing assertion was edited. The completion-block test
  `inline_completion_lifecycle::a_missing_completion_property…` passes
  unchanged, because the authored-schema wording was kept.
- **D9: non-Claudine disk triggers.** The launch verdict now loads every
  matching trigger, as the spec requires. A property owned by a non-Claudine
  trigger and absent from the authored shape is classified hard (the existing
  conservative rule for properties the shape does not declare), not dropped.
  The spec only defines the Claudine/authored split.
- **F1: diagnostic wording.** A misspelled long-form key reports
  `/start/0/shell: {"cmd":"git status"} is not valid under any of the schemas
  listed in the 'anyOf' keyword` plus a `… is not of type "string"` line, so
  `cmd` appears only in the echoed value. A Darkmatter change was tried: for an
  object value, report the one object-shaped arm's own errors, which gives
  "Additional properties are not allowed ('cmd' was unexpected)". It changed 15
  established content-policy and DMLS diagnostics, so it was reverted; it
  belongs in its own change. AC10's "naming the key" holds only through the
  echoed value.
- **F2: DMLS completion and hover do not use trigger payloads.** They read
  `effective.simplified` (the authored schema only), so with no `$schema`, DMLS
  validates from the Claudine trigger but offers no completion or hover from it.
  This predates the feature and applies to any trigger. AC7 (validation) holds;
  the docs claim must be corrected in Phase 4, or the gap fixed under a ruling.
- **F3: parser-first failures.** A malformed lifecycle action (for example
  `say: {txet: x}` in `start`) fails in the lifecycle parser, before
  `initialize` runs, because the bootstrap parses every stack. The CLI test of
  "initialize before the verdict" therefore uses a schema-only defect
  (`agent: []`). Likewise `model: {x: 1}` with `initialize` present fails at
  the hint reader ("frontmatter `model` must be a string or array of strings")
  before the verdict. Both block the launch, as before.
- **F4: index staged by a tool.** Mid-phase, the whole working tree appeared in
  the index; nothing in this phase staged it. It was restored with `git reset`,
  which leaves the working tree unchanged.

### Requirement-to-test map

The files are `claudine/lib/tests/l1/claudine_schemas_embedded.rs` (CE),
`claudine/cli/tests/l1/launch_trigger_schemas.rs` (LT),
`darkmatter/dmls/tests/l1/claudine_schema_discovery.rs` (DD), and
`darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs` (EB). All are
registered in their package's existing `l1` binary. LT is `#[cfg(unix)]`, like
`compose_removed_grammar`.

| Requirement | Tests |
| --- | --- |
| AC6: byte identity, kinds, every export and both payloads resolve with no disk schemas, CWD unrelated, no temp writes, no dependencies | CE `embedded_files_are_the_repository_files_byte_for_byte`, `embedded_{action,lifecycle,linking}_exports_resolve_without_disk_schemas` (42 exports, valid and invalid probe each), `embedded_payloads_apply_without_disk_schemas` |
| Embedded-origin input matrix, both syntaxes, plus R3 | CE `embedded_{yaml,json}_<column>` (36), guard `embedded_matrix_tests_cover_the_table`, `embedded_non_string_top_level_trigger_key_is_an_error`; the disk runner in `claudine_schemas_matrix.rs` is now parameterized by `Origin` |
| AC13 (a)–(k) | CE `nearest_claudine_trigger_shadows_farther_ones` (all five roots), `symlinked_trigger_file_in_a_root_is_ignored` (unix), `disk_bundle_resolves_its_relative_imports_from_disk`, `schema_reference_to_a_trigger_envelope_fails`, `same_name_disk_trigger_wins_over_the_embedded_copy`, `embedded_trigger_applies_with_no_disk_bundle`, `authored_schema_override_applies_last_and_takes_ownership`, `malformed_disk_trigger_fails_without_embedded_fallback`, `disk_import_never_falls_back_to_an_embedded_file`, `non_matching_disk_trigger_still_shadows_the_embedded_one`, `disk_claudine_and_embedded_inline_compose_apply_together`, `disk_and_embedded_bundles_own_the_same_properties` |
| Strict policy at the library boundary; retired grammar | CE `invalid_claudine_controls_fail_the_launch_and_are_not_dropped`, `authored_optional_is_dropped_while_a_claudine_control_fails`, `retired_spelling_reports_removed_grammar` |
| AC10 passive CLI/editor parity on equal snapshots | CE `passive_parity_<fixture>` (32), guard `passive_parity_tests_cover_every_ac5_fixture` |
| AC10 an invalid control blocks the launch with no provider, both commands | LT `misspelled_long_form_key`, `unknown_say_parameter`, `empty_agent_list`, `numeric_loop` (× compose/inline_compose), guard `every_table_row_generates_its_tests` |
| AC10 authored required with the payload also applied | LT `compose_reports_an_omitted_authored_property_and_still_applies_the_payload`, `inline_compose_defers_a_required_non_eager_property`, `inline_compose_deferral_does_not_suspend_the_payload`, `inline_compose_rejects_a_missing_required_eager_property` |
| AC10 valid launches unchanged | LT `compose_launches_a_valid_document_with_its_application_data`, `inline_compose_launches_a_valid_document_and_keeps_its_application_data`; the existing suites pass with no assertion edits except D8 |
| AC10 controls are not scrubbed; application cleanup kept | LT `compose_does_not_scrub_a_malformed_optional_control`, `inline_compose_does_not_scrub_a_malformed_optional_control`, `an_invalid_authored_optional_application_value_is_still_dropped` |
| AC10 initialize before the verdict; no `start` or provider | LT `compose_runs_initialize_before_the_verdict_and_never_start`, `inline_compose_runs_initialize_before_the_verdict_and_never_start` |
| AC10 fresh-read boundaries | LT `a_proxy_target_with_an_invalid_control_fails_before_its_provider`, `a_sequence_step_document_with_an_invalid_control_fails_before_its_provider`, `a_retried_attempt_rereads_the_document_through_the_verdict` |
| AC10 frozen completion schema; next run reloads | LT `inline_completion_rejects_an_invalid_control_the_agent_wrote`, `an_agent_written_disk_schema_takes_effect_on_the_next_run`, `inline_completion_lists_a_payload_problem_beside_the_authored_table` |
| AC10 pending and literal values | LT `a_templated_control_is_judged_after_composition`, `a_templated_model_and_a_lifecycle_span_launch` |
| AC7 inventory guards moved to `claudine/schemas/` | `dmls::diagnostics::nested_span::…::lifecycle_inventory_matches_the_authored_claudine_schema` (events, structural keys, loop keys, communication verbs); `claudine::composition::lifecycle::tests::nested_span::single_pass_inventory_matches_the_authored_claudine_schema` |
| AC7 DMLS picks up the trigger under a fixture Claudine area, disk only | DD `a_misspelled_long_form_key_is_reported_under_start_without_a_schema_pointer`, `lifecycle_stacks_and_linking_keys_are_typed_by_the_discovered_trigger`, `the_effective_schema_under_the_claudine_area_is_the_disk_bundle`, `dmls_has_no_claudine_dependency` |
| Darkmatter memo seams | EB `every_registration_of_a_bundle_shares_its_resolved_payload`, `a_merge_is_shared_only_by_equal_authored_schemas`, `a_failing_bundle_fails_on_every_registration`, `an_edited_library_under_the_same_trigger_text_is_resolved_again` (fails with the first, text-only memo key); `validate::tests::a_file_free_schema_shares_one_validator_across_contexts` |

No disk/embedded cell outcome differs. Only the error identity does: an
embedded failure is `SchemaError::EmbeddedTriggerLoad { bundle: "claudine", … }`,
and for the action columns' duplicate and trailing cells it names
`action.yaml`, the first file in byte order. Not covered: DMLS completion and
hover from a trigger payload (F2).

### Phase 3 verification

- `cd claudine && just test`: 9062 passed, 9 skipped, 0 failed (87 s; no SLOW
  test in the final run).
- `cd claudine && just lint`: exit 0. The only warning is the known macOS
  linker `__eh_frame` note.
- `cd darkmatter && just test` (lib, CLI, DMLS): 9986 passed, 10 skipped, 0
  failed. The DMLS discovery test that was SLOW under load was split in two
  (now 0.5 s and 1.1 s alone).
- `cd darkmatter && just lint`: exit 0.
- OS: no new path comparison or `cfg` code in production. Embedded names are
  lexical, and the validator-sharing rule is content-only. The CLI file is
  `#[cfg(unix)]` (shell stubs), like its neighbors; the symlink case is
  `#[cfg(unix)]`. Evidence is from macOS only; Linux, Windows, and WSL2 are
  left to the normal CI schedule, with no identified OS-specific risk. The
  `profile.test` change applies to every OS.
- No `cargo fmt`, commit, or CI change.
