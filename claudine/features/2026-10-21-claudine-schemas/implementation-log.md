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
human_review: false
message_to_agent: |-
    Phase 1 (contracts) is done; read "## Phase 1" in this log before authoring any YAML. Key rulings you must apply:
    - R1: Darkmatter's trigger `match:` grammar rejects property-level unions. Write each identifying property as five single-type `(required)` conditions under `any:` (string, number, boolean, object, "unknown[]"); the inline trigger is `all: [any: <prompt arms>, none: <sequence arms>]`. Verified with `md schema triggers`.
    - R3: fix `darkmatter/lib/src/markdown/schemas/triggers/envelope.rs:69` (non-string top-level keys silently accepted) in the "Prepare origin support" task, with a matrix cell.
    - R5: Darkmatter validation coerces scalars to strings (`model: 42` passes `string`); use list/mapping values as negative probes.
    - R6: type `agent` as `[string, string[](min(1))]` so `agent: []` is rejected (AC5).
    - R7: derive the bare-verb enum from the parser (zero-arity verbs include `and`, `or`, `branch_exists_on_remote`, `ipv4`, `ipv6`, `remote_vendor` besides stop/skip/error/retry); test it against `all_lifecycle_verbs()`.
    - R8: payload follows the code, not the drifted frontmatter-properties page; sequence overlay keys (`state`, `previous`, `next`, `outputs`, `sequence_id`) are declared `unknown` with descriptions; check whether any prompt uses non-lowercase `verbosity` before typing it as an enum.
    - R9: replacing `claudine/schemas/claudine.yaml` and deleting `partials/lifecycle.yaml` requires updating the exception entries in `darkmatter/lib/tests/l1/shipped_schema_corpus.rs:154-166`, and `claudine/lib/src/composition/lifecycle/tests/nested_span.rs::single_pass_inventory_matches_the_authored_claudine_schema` is an extra consumer of the retiring Darkmatter copies.
    - The loader seam builds on Darkmatter's existing embedded baseline source (`schemas/embedded.rs`, `SchemaSourceId::Embedded`); see "Agreed loader seam".
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
