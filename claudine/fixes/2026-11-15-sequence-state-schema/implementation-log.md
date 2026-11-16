---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-state-schema/claudine/fixes/2026-11-15-sequence-state-schema/spec.md"
plan: "claudine/fixes/2026-11-15-sequence-state-schema/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - claudine/lib/src/composition/sequence/declarations.rs
    - claudine/lib/src/composition/sequence/declarations/tests.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/reserved.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/sequence.rs
    - claudine/lib/src/composition/sequence/task/group.rs
    - claudine/lib/src/composition/sequence/task/group/branch.rs
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - claudine/lib/generated/shipped.json
    - claudine/schemas/sequence.yaml
    - claudine/schemas/sequence-types.yaml
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_2:
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/tests.rs
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/sequence_sources_cli.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/sequence_loop.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/service.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/sequence/document.rs
    - claudine/lib/src/composition/sequence/document/tests.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/lib/src/composition/sequence/normalize.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
docs_updated_during_phase_2:
    - claudine/docs/topics/frontmatter-properties.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
packages:
    - claudine
    - claudine-cli
decisions_for_review:
    - |-
        Template defaults may not set `$schema`, in addition to `$state` and `template`. The spec lists only `$state`/`template` as declarations a value route may not write. A template default is merged into every item, and on an inline nested sequence item a `$schema` key is that container's check, so a `$schema` default would silently give every container a check. Alternative: allow it and let the per-item placement rules decide (it would then be accepted on inline containers and refused elsewhere). Reversing means removing `CHECK_KEY` from `template_shape` in `sequence/declarations.rs` and its row in `template_defaults_cannot_write_declarations`.
    - |-
        A `$schema`, `$state`, or `template` written on a group object is a typed placement error from the parser (`SequenceDeclarationMisplaced` at `…group.$state`), with a hint pointing at an inline sequence leg. Before, the parser ignored unknown group keys and only the closed group schema reported them. Alternative: leave group objects to the schema alone. Reversing means deleting `reject_group_declarations` and its call in `grammar.rs::group_object_grammar`.
    - |-
        The `$state` violation message keeps its old text verbatim and now always appends `` (`$state` of `<container>` in <file>) ``, even for a root list. Alternative: append provenance only for nested or referenced owners, which keeps root messages byte-identical but makes the suffix's presence vary. Reversing touches the `#[error]` format of `SequenceStateSchemaViolation` and one assertion in `declarations/tests.rs`.
    - |-
        Error typing per value route. A setter or `--set` key naming a declaration reuses `SequenceDeclarationMisplaced` (located at the sequence document, property = the key, hint "a setter supplies values, not declarations"). A runtime `set` gets a new `RuntimeMutationError::DeclarationKey`, and `with:` keeps the existing `SequenceTaskWithReserved` (its message now mentions the declarations). Alternative: one dedicated `SequenceDeclarationWrite { route, key }` variant for all three. Reversing would touch `declarations::reject_declaration_write`, `RuntimeState::set_batch`, and `TaskExecution::evaluate_with`.
    - |-
        The passive `$state` view is just `$schema: <declaration>` in an empty in-memory frontmatter, resolved in the declaring file's context. Plan rule 8 also mentions including "the owner's sibling `types` definitions". Document frontmatter has no `types` key in Darkmatter (named types live in standalone schema documents and are imported with `Name@file`), so nothing was added. If a frontmatter-level `types` is wanted, it would have to be added in `PreparedSequenceSchema::prepare`, and Darkmatter would need to support it first.
    - |-
        A referenced whole document's `template:` keeps the referring document's values as a *lowest* layer, below the document's own properties and the caller's setters. The spec says the referrer's properties never enter the referenced owner's values. Following it strictly breaks every shipped research fleet (`claudine/docs/research/*/_fleet.md`): their shared rosters (`_provider-tasks.yaml`, `_local-runner-tasks.yaml`) fill `prompt: "{{ research_prompt }}"` from the fleet, and until Phase 4 there is no other channel. The spec's intended channel, runtime `set` writes from `initialize` reaching root graph templates, arrives in Phase 4. Alternatives: (a) drop the layer now and break the fleets until they move to `initialize: set` after Phase 4; (b) keep this layer permanently as a documented feature. A document's own value always wins, so a directly invoked roster and a referenced one differ only for keys the roster leaves undefined. Removing the layer later means deleting the marked "Transitional" block in `sequence/source.rs::plan_referenced_sequence` and the test `document::tests::a_referrer_value_is_only_a_lowest_layer_under_a_referenced_documents_own`, after migrating the 22 fleets.
    - |-
        An owner's values are prepared through `prepare_sequence_document_values` (one frontmatter compose) only when its `template:` has at least one string (expression) default *and* the caller supplied a captured compose context. Otherwise the template reads the authored values under the caller's setters, unconverted. This avoids a compose per sequence resolution when no template expression reads the values. Alternative: always prepare when a context is supplied, so `SequenceDocumentScope::values` is always the prepared form, at the cost of one extra frontmatter compose per owner. Reversing touches only the condition in `sequence/document.rs::owner_values`.
    - |-
        The missing-input error's one-line text changed from "…failed for N step(s); …" to "…failed for N step(s) or document(s); …", and its rendered summary now counts each kind ("in 1 step and 2 documents of the sequence"). A document entry is headed `Document <file>`, plus `` (`<container path>`) `` for a referenced owner, and never gets a step number. Alternative: keep the old one-line text and change only the rendered block. Reversing touches the `#[error]` on `SequenceMissingProperties` and two tests in `composition/error/tests.rs`.
    - |-
        In the preflight graph, a root row's task is still located as `tasks[i]`, so an inline container's check and `$state` failures under a root row read `tasks[0].$schema` / `tasks[0].sequence`. The input inventory and the formal normalizer use the authored path `sequence[i]`, which is what a user sees in the file. The graph's `tasks[i]` predates this fix and appears in about 80 test assertions across lib and CLI, so I left it alone. Alternative: rename the graph's root prefix to `sequence[i]` so every location matches the file. Doing so means changing `walk_plan`'s `format!("tasks[{index}]")` and updating those assertions.
    - |-
        A nested reread (`prepare_nested_sequence`) still discovers command sites from the reread rows, read as authored with plain normalization (no template fill, no `$state`), and keeps the frozen plan for execution. Rule 19's "discover refreshed command sites against frozen child nodes" could also mean walking only the frozen children. I tried that first: it silently removes the existing refusal of a command changed on disk (`changed_nested_source_refuses_new_commands_before_start`), which the plan says to retain. Alternative: walk frozen children only and accept that an edited inline child runs its approved, frozen command without refusal. Changing this touches the `refresh` arms in `Loader::load_sequence_with`.
    - |-
        Rows of a root `sequence: child.yaml` resolve their `prompt:`/`task:`/`sequence:` references from the invoking file's directory, because `walk_plan` runs them as the root's rows. The inventory does the same, so the two passes agree. A nested `sequence: child.yaml` resolves from `child.yaml`'s directory. Rule 16 asks to keep the root row shape, so I did not change it. Alternative: resolve root-reference rows from the referenced file, matching nested references. Doing so means changing `walk_plan`'s origin and `inputs.rs::walk_referenced`'s `rows_origin`.
    - |-
        The root reference's document metadata reaches input collection as the inventory's `RootReference` owner (file, property `sequence`, prepared values, missing entry). I added no `PreflightGraph` field for it. Alternative: also record `plan.document_scope` on the graph for a later consumer. That is an additive field in `walk_plan`.
    - |-
        An authored executable reference containing `{{ … }}` (for example `prompt: "{{ plan }}"`) is pending only while its owner is missing an input. Once the inputs are supplied, it is read as the literal file name it always was, and fails as not found, exactly as the graph does. Only `template:` defaults are interpolated. Alternative: interpolate authored executable references in both the inventory and the graph. That would be a new feature beyond this fix.
    - |-
        Discovery judges declaration shape and placement but does not resolve `$state` or an inline `$schema` reference. An unloadable one fails at the graph build that follows, before approval. This avoids preparing every declaration twice. Alternative: prepare them in discovery too, through the shared `NestedSources` cache. Doing so means calling `NestedSources::prepared_schema` in `inputs.rs::walk_executable`.
message_to_agent: |-
    Phase 3 landed the library side of input discovery and the owner-scoped graph. Use these:
    - `composition::discover_sequence_inputs(source, InputDiscoveryOptions::new(&resolution, &context).with_caller_layers(&layers).with_invocation(&invocation, &source_context))` returns `SequenceInputInventory`. Each `owners[i].missing` is already a `SequenceMissingSubject::Document` entry, and `missing_error()` is the combined report. `prompts` lists prompt contexts (path, `with:`, owner) for Phase 1c to probe. `pending` lists template references whose owner is missing an input: they resolve after collection, so rerun discovery when an input is supplied (rule 13's loop).
    - Then build the graph with `build_preflight_graph_after_discovery(…, &inventory)`, which reuses discovery's parses.
    - The graph is not wired into the CLI yet. `commands/sequence.rs`/`wrap/sequence/mod.rs` still call `build_preflight_graph_with_invocation`, and Phase 1c still loops twice over root steps. That is Wave 7.
    - `PreflightSequence::inline_schema` is the prepared inline check (`PreparedSequenceSchema::validate`). Rule 17's execution check before `run_sequence` opens its guard is not implemented. Raise `SequenceContainerSchemaViolation` from it. `prepare_nested_sequence` already returns the reread container's current check, for the retry/pass boundary.
    - The `Loader` builds a nested reference's caller layers from `caller_setters` against the referring file's context, so their recorded origins are not the caller's own. Pass the real `CallerInputLayers` when you wire the CLI.
    - Graph root rows are located `tasks[i]`, and the inventory uses `sequence[i]` (see decisions_for_review).
    - Authored `file(eager)` values in a referenced document resolve from the request context, not the document's directory. Setters convert correctly. See the Phase 3 observations in the log.
source_files_during_phase_3:
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/sequence/declarations.rs
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/inputs.rs
    - claudine/lib/src/composition/sequence/preflight/inputs/tests.rs
    - claudine/lib/src/composition/sequence/preflight/tests/scoped_graph.rs
    - claudine/lib/src/work_counters.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
---

# Implementation Log for 2026-11-15-sequence-state-schema (6 phases)

## Phase 1


### Wave 1 — Declaration contracts (rules 1, 5–8, 20)

- New module `lib/src/composition/sequence/declarations.rs` owns placement,
  shape, and preparation of sequence declarations:
  - `root_declarations` / `task_declarations` judge by key *presence*
    (explicit `null` included). Root `$state`/`template` require the root
    `sequence:` to be an inline list; task `$schema`/`$state`/`template`
    require the task's own inline list. Every other placement (prompt, shell,
    side-effect, body-only, `task:` reference, `sequence: file.yaml`, group
    object) is `SequenceDeclarationMisplaced` with a hint naming where the
    declaration belongs. `reject_group_declarations` keeps group objects
    closed to them.
  - Shapes: a schema declaration is an inline mapping (`{}` = open), a
    non-empty reference string, or a non-empty union of mappings/non-empty
    references; `null`, `[]`, `""`, numbers, booleans, and bad arms are
    `SequenceDeclarationShape` (arms located as `$state[1]`). `template` must
    be a mapping (`{}` = no-op); `null` is invalid. Template keys may not be a
    reserved state key, a declaration, or `$schema`
    (`SequenceReservedTemplateKey`), judged before any row is read.
  - `PreparedSequenceSchema` resolves a declaration once through
    `DarkmatterSchemas::new(ctx.for_source(declaring_path)).effective_for`
    (no triggers, no baseline — same as the old per-row path) and judges each
    `StepState::to_value()` passively with `EffectiveSchema::validate`.
    Unparsable/unresolvable declarations are `SequenceDeclarationLoad`
    carrying the boxed `simplified_schema::SchemaError`.
- `formal.rs`: `FormalKeys::schema` → `state_schema`, read from `$state`
  through `root_declarations`; `formal_keys` now takes the declaring path.
  `normalize_formal_plan` prepares `$state` before shorthand/template/rows,
  so an empty list still fails on a malformed or unloadable `$state`. The old
  per-row `validate_state_schema` (a synthetic document per row) is gone.
- `grammar.rs`: `validate_task_grammar` calls `task_declarations` first;
  `validate_sequence_root` calls `root_declarations`; `group_object_grammar`
  calls `reject_group_declarations`.
- `reserved.rs`: `SEQUENCE_DECLARATION_KEYS = ["$state", "template"]`, now in
  `is_reserved_state_key`; `$schema`/`$state`/`template` added to
  `TASK_OPTION_KEYS` and the sequence executable's allowed options; one
  `declaration_written_by`/`is_declaration_write` predicate (root of a dotted
  or indexed path) for every value route. `GENERATED_STATE_KEYS` doc now says
  `$state`.
- Value routes: `RuntimeState::{new_sequence, from_sequence_snapshot}` set a
  private sequence flag; `set_batch` refuses a declaration write only on such
  a cell (`RuntimeMutationError::DeclarationKey`). `new`/`Default`/
  `from_snapshot` are unchanged for ordinary document runs. The CLI root
  (`wrap/sequence/mod.rs`), the container fallback cells
  (`task/sequence.rs`), and group branches (`task/group.rs`,
  `task/group/branch.rs`) use the sequence constructors. `with:` evaluation
  refuses dotted declaration paths too. `claudine sequence` refuses a setter
  or `--set` key that writes a declaration (`reject_declaration_write`) before
  anything reads it.
- Errors (`composition/error/mod.rs`): new `SequenceDeclarationMisplaced`,
  `SequenceDeclarationShape`, `SequenceDeclarationLoad`,
  `SequenceContainerSchemaViolation` (defined now, raised from Phase 4's
  inline check). `SequenceStateSchemaViolation` gains `source_path` and
  `container`; its message keeps the old clause verbatim and appends
  `` (`$state` of `<container>` in <file>) ``. All render through the generic
  Prose catch-all, as `TaskLoopMisplaced` does.
- `resolve.rs::FORMAL_SEQUENCE_KEYS` is now `["$state", "template"]` so a
  directly invoked YAML document keeps its root `$schema` for composition
  (rule 1's "keep root `$schema` in document composition"); the YAML-only
  gate and nested protection remain Phase 2's (rule 3).

### Wave 1 — Shipped contracts (rule 22)

- `schemas/sequence.yaml`: root `$state: schema`, retained `template: object`
  with an inline-list description.
- `schemas/sequence-types.yaml`: recursive `task` gains `$schema: schema`,
  `$state: schema`, `template: object`, each described as an inline-sequence
  declaration beside `sequence:`. No defaults; group stays closed.

### Wave 2 — Contract checkpoint

- `just shipped-schemas` (repo root) regenerated only
  `claudine/lib/generated/shipped.json` (3 lines changed); no other
  generated file moved.
- Extended `lib/tests/l1/sequence_schema.rs` with
  `sequence_declarations_are_typed_alike_on_disk_and_embedded`: a valid
  document using root `$schema`/`$state`/`template` plus a nested container's
  `$schema`/`$state` (as a relative reference)/`template`, a root
  `$state: 42`, and a nested `template: 3`. It is judged through the disk
  bundle and the embedded bundle, and the verdicts must be equal. That covers
  disk/embedded parity for the new declarations. `claudine_schema_parity.rs`
  derives events/actions from the registry and has no declaration surface,
  so it needed no change. Freshness is the existing
  `shipped_artifacts::shipped_artifact_is_fresh`, which passes.

### Requirement → test mapping (Phase 1)

All of these are L1 tests at F1/F2 (in-process public resolver, plus one CLI
subprocess for setter wiring). The boundary is the sequence resolver
(`resolve_sequence_plan`) over direct YAML, direct Markdown, and referenced
documents.

- Placement of root `$state`/`template` beside a non-list source, with
  `null` still counting as present, in YAML and Markdown:
  `declarations::tests::root_declarations_beside_a_non_list_source_are_misplaced`
- Root `$schema` is not a state declaration:
  `…::root_schema_is_not_a_state_declaration`
- Absent / `{}` / `null` / `[]` / `""` / number / boolean / bad union arm /
  bad template shapes, on an empty list:
  `…::declaration_shapes_are_judged_by_presence_on_an_empty_list`
- Unloadable or unparsable `$state` on an empty list, with a downcastable
  typed cause: `…::an_unloadable_state_schema_fails_an_empty_list`
- Template defaults cannot write declarations or reserved state:
  `…::template_defaults_cannot_write_declarations`
- Inline containers (and group legs) own their declarations as options, not
  parent state: `…::inline_containers_accept_their_own_declarations`
- Task placement errors for prompt, shell, side-effect, body-only,
  reference, and group, plus a group object:
  `…::task_declarations_beside_a_non_list_are_misplaced`
- Task shape errors located at the full container path:
  `…::task_declaration_shapes_are_located_at_the_task`
- `$state` violation identity and owner provenance, including a referenced
  owner: `…::state_violations_name_the_step_and_the_declaring_owner`
- Relative `$state` reference resolved in the declaring file's directory:
  `…::a_state_reference_resolves_in_the_declaring_documents_context`
- One write predicate (dotted and indexed paths):
  `…::declaration_writes_are_detected_by_their_root`; step state cannot
  author a declaration: `…::step_state_cannot_author_a_declaration_key`
- Runtime `set` refusal only on sequence cells, branch included:
  `runtime_state::tests::only_a_sequence_cell_refuses_declaration_writes`
- Setter / `--set` wiring through the real CLI (setter before and after the
  file), with no step composed:
  `cli … sequence_sources_cli::a_setter_cannot_write_a_sequence_declaration`
- Renamed row-schema tests now use `$state`:
  `sequence_sources_cli::{a_referenced_formal_documents_state_declaration_validates_the_step_state,
  a_directly_invoked_documents_state_declaration_validates_the_step_state,
  a_formal_document_normalizes_identically_through_both_entry_paths}`
  and `sequence::tests::formal_documents::*`. `external_template_non_object_fails`
  now expects `SequenceDeclarationShape`.
- Shipped meta-types plus disk/embedded parity:
  `sequence_schema::sequence_declarations_are_typed_alike_on_disk_and_embedded`

Not yet covered (later phases own the behavior): Markdown `$state`/`template`
normalization, `$state` on nested inline lists, and the inline `$schema`
check at task admission. Phase 1 only validates the syntax and placement of
those declarations.

### Gates

- `just lint` (claudine/): passes. The first run failed the transport guard
  `no_unallowlisted_typed_error_collapses`, because `PreparedSequenceSchema::prepare`
  stringified a frontmatter insert error. It now keeps the typed
  `SequenceExternalLoad { source: SequenceLoadCause::Frontmatter }`.
- `just test` (claudine/): 12,187 tests passed, exit 0; 44 recorded at or
  above 5 s, all under 10 s. The host was under heavy unrelated load (load
  average 130–173 from other sessions' Blender and rustc jobs; no subagents
  of mine were running), so the 10 s allowance applied. I re-measured all 44
  with `-j 1` at load about 20–85: 41 were at or under 5 s. The other three were
  re-measured individually at load about 20: `deep_container_deferral…` 3.61 s
  (8.58 s in the -j1 batch), `shipped_artifacts::the_checked_in_artifact_round_trips…`
  2.67 s (5.36 s), `claudine_schemas_matrix::yaml_trigger_combinator::control`
  2.58 s (5.01 s). The new `sequence_declarations_are_typed_alike_on_disk_and_embedded`
  took 5.92 s in the package run and 3.71 s alone. All are recorded as
  load-sensitive, not defects.
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents: only in-area crates (`claudine-contract`, `claudine-cli`)
  depend on `claudine`, and `just test` covers both.
- Cross-OS: no cross-check was run. There is no new path handling:
  diagnostics use `to_portable_string`, and test path checks use
  component-wise `Path::ends_with`. CI's nightly legs cover Windows and WSL2.

### Notes for later phases

- `FORMAL_SEQUENCE_KEYS` moved to `["$state", "template"]` in this phase,
  because otherwise the key split from rule 1 would be inconsistent. The
  YAML-only gate in `without_formal_sequence_keys`, the nested protection,
  and Markdown routing are still rule 3 / rule 15 work.
- `PreparedSequenceSchema` is prepared once per `normalize_formal_plan` call
  (not per row). There is no invocation-level cache across owner rereads yet;
  that belongs with `NestedSources` (rule 8, Phase 3). `dependencies()` is
  exposed for that cache.
- `SequenceContainerSchemaViolation` is defined but not raised yet (rule 17,
  Phase 4). `PreparedSequenceSchema::validate` is the passive judge to use
  for it.

## Phase 2

### Wave 3 — Composition views (rules 3, 4, 10)

- `resolve.rs`: `without_formal_sequence_keys` no longer checks the file
  format. When the root `sequence:` is an authored list, the composition view
  drops root `$state`/`template` and `$schema`/`$state`/`template` from every
  inline container task beneath it (nested `sequence:` lists and inline
  `group:` legs). Only that task structure is walked, so a plain value holding
  a `template` key is left alone, and so is a document with no list.
  `composition_view` (crate-internal) returns a `Cow` and borrows when nothing
  needs dropping, so ordinary documents are not cloned. The root `$schema`
  stays. Mapping orders travel with the rebuilt frontmatter. Removing keys
  moves no list index, and an object's recorded order skips the keys it no
  longer holds.
- New `reload_authored_source` (declarations kept). `reload_composition_source`
  now means "authored reload, then the composition view". Phase 3's nested
  refresh should use the authored one.
- The shared preparation service applies the view at its entry points
  (`prepare_document_selecting`, `prepare_bootstrap`), so a library caller
  gets the same rule as the sequence CLI.
- New `sequence/document.rs`:
  - `SequenceDocumentScope` (declaring path, property, `Arc` authored root,
    `Arc` owner values).
  - `SequenceDocumentPreparation`.
  - `prepare_sequence_document_values(source, PrepareOptions, CompositionMode)`.
    It is the canonical option assembly narrowed to the frontmatter surface,
    with frontmatter `$( … )` expansion *disabled* (passive, not refused) and
    the body never read. `prepare.rs::prepare_values_on` composes, and the
    schema layer judges the result through a new
    `schema::PrepareMode::Values(mode)`. That reuses the existing
    categorization, drop-and-retry, and post-shell verdict unchanged.
    `defer_schema_verdict` withholds the verdict, as bootstrap does.
    `translate.rs` now reads the phase through `PrepareMode::is_inline`.

### Wave 3 — Input subjects (rule 14)

- `SequenceMissingPropertiesStep` was replaced by
  `SequenceMissingPropertiesEntry { subject: SequenceMissingSubject, … }`, where
  the subject is `Step { number, name }` or `Document { property }`. Both are
  exported from `composition`. The renderer keeps the old step headings and
  adds `Document <file>`, or `Document <file> (`<container path>`)` for a
  referenced owner. The opening line counts steps and documents separately
  ("in 1 step and 2 documents"). The `Display` text now reads
  "… for N step(s) or document(s); …". Phase 1c builds `Step` subjects, and
  its interactive status heading handles both subjects. Collection order is
  unchanged.
- Docs: `claudine/docs/topics/frontmatter-properties.md` named the old type
  and was updated.

### Wave 4 — Owner normalization (rules 9, 10, 15, 16)

- `SequenceSourceOptions` fields are now private, with builders:
  `new` (no caller input), `with_shell_runner`, `with_compose_context`, and
  `with_caller_layers`. All struct-literal callers (CLI `sequence.rs`, lib and
  CLI tests) use the builders.
- `formal::is_direct_formal_document` is gone. Every authored root list, YAML
  or Markdown, goes through `normalize_formal_plan`. Expression and shell
  sources keep the data pipeline. `normalize_formal_plan` now takes a
  `FormalOwner { keys, declaring_path, list_property, values }` and calls
  `normalize_plan_at`. Template expressions are anchored at the *declaring*
  file (rule 9). Before, they were anchored at the invoking file, which was
  wrong for a referenced document in another folder. `invocation_path` still
  seeds `sequence_id`.
- Owner values (`document::owner_values`): when the template has a string
  default and the options carry a captured `ComposeContext`, the owner is
  prepared with `prepare_sequence_document_values` (verdict withheld). Without
  a context, the values are the authored root under the caller's setters, by
  top-level key. The normalizer never captures a CWD or environment.
- A referenced whole document (`source.rs::plan_referenced_sequence`) is its
  own owner. Its template reads its own properties under the caller's
  setters, never the referrer's frontmatter. Its prepared source is built
  lazily (`whole_document_source`): Markdown is reread, data formats become
  body-less frontmatter. `::map`/`::name`/`::template` operators on data
  selections still read the referrer's frontmatter, as before. The preflight
  `Loader` passes its `caller_setters` as caller layers, so a nested
  referenced document fills its template the same way.
- `SequencePlan::document_scope` is set for a root inline list (the invoked
  file) and for a whole referenced document (the target file). It is `None`
  for selections and dynamic sources. `normalize.rs` initializes it to
  `None`.
- Transitional referrer layer: a whole referenced document's template still
  reads the referrer's values, but only as the *lowest* layer, below the
  document's own values and the setters. Without it, all 22 shipped research
  fleets break (`research_sequence_artifacts` failed 4 tests): their shared
  rosters read `research_prompt` from the fleet, and the spec's intended
  channel (`initialize` `set` writes reaching root templates) is Phase 4's.
  See `decisions_for_review`.
- `cli/tests/l1/composition_seams.rs`: the R1 guard needed an allowlist entry
  for the new `compose_with` site `composition::prepare::prepare_values_on`.
  It is a stage of the sanctioned owner (frontmatter surface, shells disabled,
  no prompt), like `read_selection_hints`/`compose_bootstrap`.
- Not in this phase (per plan): the CLI root does not pass a captured context
  or caller layers to the resolver yet (Wave 7), and `Loader::load_sequence`
  inline children do not use the formal pipeline yet (Wave 6).

### Requirement → test mapping (Phase 2)

All L1, in-process at the library's public entry points (F1/F2). The
boundary is document loading and preparation (filesystem fixtures in temp
dirs, the captured `ComposeContext`, the real schema layer).
`lib/src/composition/sequence/document/tests.rs`:

- Markdown/YAML projection parity, root `$schema` kept, `$state`/`template`
  dropped at the root, in nested containers, and in a group leg's container,
  a plain value's `template` key kept, and root/nested key order kept:
  `both_formats_compose_without_their_declarations_and_keep_the_document_schema`
- An ordinary document's `template` property, a reference-only sequence, and
  an undeclared list are unchanged: `a_document_without_list_declarations_is_unchanged`
- Authored and composition rereads (fresh text, nested declarations kept vs.
  dropped): `the_two_rereads_differ_only_in_the_declarations`
- A `file(eager)` setter supplied from another directory is converted before
  `plan: "{{ dirname(spec) + '/plan.md' }}"` reads it, no frontmatter shell
  runs, and the body is not read (both formats):
  `document_values_are_converted_before_a_derived_property_reads_them`
- A missing document input on `sequence: []` is the typed `MissingProperties`
  report; a withheld verdict returns values:
  `a_missing_document_input_is_typed_unless_the_verdict_is_withheld`
- Markdown and YAML lists are filled and typed alike, with one template value
  that reads an item-only field and one that reads a setter-supplied document
  input; Markdown `$state` failure names the step; `document_scope` set:
  `markdown_and_yaml_lists_are_filled_and_typed_alike`
- Referenced whole YAML/Markdown/JSON/JSON5 documents are their own owners:
  own value over referrer, setter over own value, `file_exists` resolved
  beside the declaring file, `document_scope` is the target:
  `a_referenced_whole_document_fills_its_template_from_its_own_values`
- The transitional referrer layer is lowest:
  `a_referrer_value_is_only_a_lowest_layer_under_a_referenced_documents_own`
- Passive `$state`: a declared default is never written into the step:
  `state_validation_never_writes_into_the_step`
- Offset and offset+operator selections ignore the selected file's
  declarations and carry no scope:
  `a_selection_ignores_the_selected_documents_declarations`
- Document missing-input subject rendering (root without a container, a
  referenced owner with its container path, no step 0, mixed counts):
  `composition::error::tests::sequence_missing_properties_heads_a_document_entry_by_its_file_and_container`;
  existing step-rendering tests were adapted to the new type.

The Input Robustness Matrix was not re-walked here. This phase adds no new
field validator (declaration shapes are Phase 1's validators), and the
whole-document loaders are unchanged. The JSON5 syntax row (comments,
trailing commas) is exercised by the referenced-document test above.

### Gates (Phase 2)

- `just lint` (claudine/): passes.
- `just test` (claudine/): 12,198 passed, 0 failed, exit 0. The first run
  failed only the R1 `compose_with` guard, fixed as above. 84 tests were
  recorded at or above 5 s; the slowest was 8.07 s, so none reached 10 s.
  The host load average was 145–210 on 16 cores from other sessions. None of
  the slow tests is new or touches Phase 2 code: they are the schema-matrix,
  shipped-corpus, container-schema-recursion, and preflight shell-template
  tests that drive `DarkmatterSchemas` or preflight directly, and
  `shipped.json` did not change. Re-measurement could not clear them on this
  host. A serial `-j 1` rerun at load 145–200 came out slower (one test hit
  the nextest timeout). Alone, at load about 120–166:
  `group_group_restricts_cancelled` 12.5 s, then 6.6 s;
  `deep_container_deferral_…` 15.1 s, then 8.8 s; `json_trigger_combinator::duplicate`
  12.4 s; `frozen_cli_review` 6.9 s; `shared_payload_properties_have_identical_types` 8.7 s.
  Phase 1 measured the same tests at 2.6–3.6 s alone on a quieter host. I
  tried to time them on build-linux with `just cross-check … -E '<filter>'`,
  but the remote recipe re-splits a filterset argument (bash syntax error at
  `(`), so no idle-host measurement exists. **These remain uncleared
  load-sensitive timings, not cleared by the 5 s re-measurement rule.**
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents: only in-area crates (`claudine-contract`, `claudine-cli`) depend
  on `claudine`, and `just test` covers both.
- Cross-OS: no cross-check suite run. The new path handling is limited to
  reading files the resolver already resolved. Test path checks compare
  trailing components, so the platform's path form (verbatim prefix, short
  names) cannot fail them. CI's nightly covers Windows and WSL2.

## Phase 3

Phase 2's changes were still uncommitted when this phase started, so the
working tree holds Phases 2 and 3 together. This phase's own files are listed
in `source_files_during_phase_3`.

### Wave 5 — Input inventory (rules 12, 13 library side)

- New `lib/src/composition/sequence/preflight/inputs.rs`:
  `discover_sequence_inputs(source, InputDiscoveryOptions)` returns a
  `SequenceInputInventory`:
  - `owners` (`InputOwner`): the root, the whole document a root
    `sequence:` references (`RootReference`), and every whole document a
    nested `sequence:` task references (`Nested`), whether it is reached
    inline, from a group leg (inline, group file, or catalog entry), or from a
    `kind: task` file. Owners are listed in first-encounter order, including
    owners with no steps. Each owner carries its file, the referencing file
    and container property (`sequence[2].group.tasks[0].sequence`), its
    prepared values (verdict withheld), and its missing inputs as a
    `SequenceMissingSubject::Document` entry.
  - `prompts`: each distinct prompt reference (path plus `with:`), with its
    nearest owner.
  - `inline_containers`: check-only inline containers, kept apart from owners.
    Their `$schema` and `$state` never become inputs.
  - `pending`: a reference that reads a `{{ … }}` (an authored one, or one a
    `template:` default fills) while its owner is missing an input. It is not
    resolved, and is reported only through that owner's missing entry.
  - `missing_entries()` / `missing_error()` give the combined typed
    `SequenceMissingProperties`.
- Owner preparation is Phase 2's `prepare_sequence_document_values`: verdict
  first, then, only on `MissingProperties`, once more with the verdict
  withheld to get the values. Any other preparation error (an invalid
  supplied value, an unloadable schema) fails discovery. Caller layers are
  applied by `CallerInputLayers::apply_to` and never mutated.
- The walk is a `Loader` (its `resolve_reference`, `load_data_document`,
  `load_prompt`, `enter`/`leave`, cycle check, and catalog selection).
  `InputDiscoveryOptions::with_invocation` gives it per-repository source
  contexts, as the graph has. The inventory keeps the walk's `NestedSources`,
  and `build_preflight_graph_after_discovery` hands them to the graph walk, so
  a referenced file is parsed once for both passes.
- Every authored item of an owner list is judged by `validate_task_grammar`,
  which also judges every inline list nested in it. Malformed declarations
  therefore fail behind a false `when:` too.
- A `template:` executable default is merged in before the item's executable
  is read: literal values directly, and expressions filled with
  `formal::fill_template_default`, a new one-key form of `apply_template`.
- A root reference's rows are resolved from the invoking file's directory,
  matching `walk_plan`, which runs them as root rows. A nested whole
  document's rows are resolved from that document's own directory.

### Wave 6 — Scoped graph (rules 8, 15, 17, 19)

- `Loader::load_sequence` (inline) now goes through
  `formal::normalize_formal_plan_prepared` with the task's own
  `template`/`$state` (`declarations::inline_container_declarations`, a new
  shape-only form of `task_declarations` for a task known to hold a list). The
  values are the walk's current `step_frontmatter`, that is, the enclosing
  document's values. A parent's template and `$state` stop at the container
  row. The parent's `$state` still judges that row's own state.
- `normalize_formal_plan` now locates a nested `$state` at its owner's path
  (`sequence[1].$state`); a root `$state` keeps the property `$state`.
- A nested whole-document reference passes the walk's captured
  `ComposeContext`, so its template reads its prepared values (for example, a
  caller's `file(eager)` setter converted by the document's `$schema`).
- `PreflightSequence::inline_schema: Option<PreparedSequenceSchema>` is
  prepared beside the container (`tasks[0].$schema`). It is used nowhere else:
  not in `owner_schema` typing, provider prediction, inventory, or parent
  state. `PreflightSequence::document_scope()` exposes the plan's document
  scope (the referenced file for `sequence: child.yaml`).
- Declarations are cached in `NestedSources::prepared_schema`, keyed by
  (file, property). An entry is reused while the declaration value and the
  bytes of every schema file it read (`EffectiveSchema::dependencies`) are
  unchanged. New counter `Work::SequenceSchemaPrepare`, counted inside
  `PreparedSequenceSchema::prepare`.
- Rule 19: `prepare_nested_sequence` calls `load_sequence_with(…, refresh =
  true)`. It rejudges the reread container's declarations (shape and
  placement), prepares its current check, and rereads its events. It
  discovers command sites from the reread rows as authored, with plain
  `normalize_plan_at` (no template fill, no `$state`), so the existing refusal
  of unapproved commands still holds
  (`changed_nested_source_refuses_new_commands_before_start` passes). The
  frozen plan, ids, template-filled state, steps, and loop controls are kept.
  A reread whole document's `formal_keys` and root grammar are judged the same
  way. My first attempt walked the frozen children instead, which silently
  dropped that refusal. The existing test caught it.

### Requirement → test mapping (Phase 3)

All tests are L1, in-process, at the library's public entry points
(`discover_sequence_inputs`, `build_preflight_graph*`,
`prepare_nested_sequence`), with filesystem fixtures in temp dirs (F2). The
boundary is document loading and preparation through the real schema layer.

`preflight/inputs/tests.rs`:
- Owner file/property metadata for root, nested, group-leg, and task-file
  owners, plus an empty owner, in YAML and Markdown roots; no inline/`$state`
  inputs; the combined report's `Document` subjects; prompt ownership:
  `every_owning_document_is_inventoried_with_its_file_and_container`
- Independent requirements: `a_supplied_input_clears_only_the_owner_that_declares_it`,
  `supplying_every_owners_inputs_clears_the_report`
- The spec's `file(eager)` reproduction: pending template and authored
  references while `spec` is missing, a literal reference still resolved, and
  caller-relative conversion once `spec` is supplied:
  `a_reference_reading_a_missing_input_is_pending_until_the_input_is_supplied`
- Root reference (JSON5 whole document, with comments and a trailing comma) is
  an owner; an offset selection is not:
  `a_root_reference_is_an_owner_and_a_selection_is_not`
- Malformed declaration behind `when: "false"`, an invalid supplied value
  (`SchemaValidation`, not missing), and a cycle:
  `malformed_declarations_and_invalid_values_fail_discovery`
- No frontmatter shell, prompt shell, or `initialize` action runs:
  `discovery_runs_nothing`
- Shared parse with the graph: `counted::the_graph_walk_reuses_the_parses_discovery_made`

`preflight/tests/scoped_graph.rs`:
- Own template, enclosing values, root template stopping at the container:
  `each_inline_list_is_filled_by_its_own_template_from_the_enclosing_values`
- Immediate-row ownership; parent-row validation at the container boundary;
  file and full container path for inline, group-leg, and task-file owners:
  `each_state_contract_judges_only_its_immediate_rows`
- Inline check prepared and located; not a document scope; a referenced
  document has none; unloadable check is `SequenceDeclarationLoad`:
  `an_inline_check_is_prepared_beside_its_container_and_not_a_document_schema`
- Frozen rows, ids, and template state across a reread that changes the
  template, `$state`, and check; a reread malformed `$state` is a shape error:
  `a_reread_judges_declarations_but_keeps_the_frozen_rows`
- Nested whole document filled from its own prepared values:
  `a_referenced_document_fills_its_rows_from_its_own_prepared_values`
- Preparation reuse, with no schema resolved per row (12 rows → 2
  preparations; 3 unchanged rereads → 0):
  `counted::declarations_are_prepared_once_per_owner_revision`
- Schema-file freshness: `counted::a_changed_schema_file_invalidates_the_prepared_check`

Input Robustness Matrix: this phase adds no field validator. Declaration
shapes are Phase 1's validators, which the inventory calls, and one control
re-proves the wiring (`$state: 42` behind a guard). The loaders are the
existing ones, reused through the `Loader`. The JSON5-specific syntax row is
covered by the root-reference test.

### Observations for later phases

- Authored (not supplied) `file(eager)` values in a referenced document
  resolve from the request's file context, not the document's directory. A
  probe showed `spec: work/spec.md` in `nested/child.yaml` failing to resolve
  from the root. Setter values convert correctly. This is how the preparation
  layer behaves today; I did not change it.
- The `Loader` builds caller layers for nested references from its
  `caller_setters` against the referring file's context, so their recorded
  origins are not the caller's own. Wave 7 should pass the invocation's real
  `CallerInputLayers` through.

### Gates (Phase 3)

- Scope calculator (`git status … | xargs python3 scripts/ci/affected_scope.py`):
  `packages: [claudine, claudine-cli, test-toolkit, darkmatter]`,
  `change_class: package`, so the gates ran. The working tree still holds
  Phase 2's uncommitted files too. Only `claudine` (lib) source changed in this
  phase.
- `just lint` (claudine/): passes. The first run caught a `type_complexity`
  (now `DeclarationKey`) and a needless lifetime in a test helper.
- `just test` (claudine/): 12,213 passed, 0 failed, exit 0, at load 100–157 on
  16 cores from other sessions. 73 tests were recorded at or above 5 s, and
  the slowest took 8.65 s, so none reached 10 s. I re-measured all 73 with
  `-j 2` as load fell from 127 to 24: 72 finished at or under 5 s. The other,
  `claudine_schemas_embedded::disk_bundle_resolves_its_relative_imports_from_disk`,
  took 5.40 s there and 2.56 s alone at load 22. All are cleared. The one new
  test in the list, `inputs::tests::supplying_every_owners_inputs_clears_the_report`,
  took 5.92 s in the package run and passed under 5 s in the re-measure. I
  had already split it from a 7.4 s combined test, which took 1.8 s alone.
- `just check-tier-coverage claudine`: 0 stranded.
- Dependents: only in-area crates (`claudine-contract`, `claudine-cli`) depend
  on `claudine`, and `just test` covers both.
- Cross-OS: no cross-check was run. The new code only adds path keys (it
  compares canonicalized paths the walk already produced) and reads schema
  files by the paths Darkmatter reports. Test path checks compare trailing
  components. CI's nightly runs cover Windows and WSL2.
- Docs: no `docs/` page changed in this phase. The plan gives the topic pages
  for nested declarations and input collection to Phase 6, and the
  user-visible collection behavior is not wired into the CLI until Phase 4.
