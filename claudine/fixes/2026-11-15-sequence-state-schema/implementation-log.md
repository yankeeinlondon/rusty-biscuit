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
message_to_agent: |-
    Phase 1 landed the declaration contract in `claudine/lib/src/composition/sequence/declarations.rs`: `root_declarations`/`task_declarations` (placement + shape, by key presence), `reject_group_declarations`, `reject_declaration_write`, and `PreparedSequenceSchema` (prepare once per owner, then `validate_states(plan, container)` or a passive `validate(value)`). Use these rather than re-reading keys. `formal_keys(root, path)` now takes the declaring path. `FormalKeys` has `template` and `state_schema`.
    Already done ahead of Phase 2: `resolve.rs::FORMAL_SEQUENCE_KEYS` is `["$state", "template"]`, so direct YAML keeps its root `$schema` for composition. Still yours: remove the YAML-only gate in `without_formal_sequence_keys`/`is_direct_formal_document`, protect nested declarations, and route Markdown inline lists through `normalize_formal_plan`.
    `validate_task_grammar` already accepts `$schema`/`$state`/`template` beside an inline `sequence:` list (they are task options, never parent state), but nothing applies a nested `$state`/`template` yet, and the inline `$schema` check is not run. `SequenceContainerSchemaViolation` exists for Phase 4.
    `RuntimeState::new_sequence`/`from_sequence_snapshot` carry a private sequence flag. Any new sequence-owned runtime cell must use them, or declaration writes will be accepted there.
    `just test` under heavy host load (other sessions) shows about 44 schema-heavy tests at 5–10 s. Every one passed at 5 s or less when re-measured alone, so re-measure before treating them as defects.
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
