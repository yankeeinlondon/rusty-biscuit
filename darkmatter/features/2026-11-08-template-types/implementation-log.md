---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-08-template-types/spec.md"
plan: "darkmatter/features/2026-11-08-template-types/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
decisions_for_review:
  - |-
    The rename guard in the plan (Wave 2) uses the regex character class `[ -_]`, which is a
    range from space to underscore and so matches many unrelated characters (for example
    `value(slot` and `to_value(SlotDiagnosticData`). The guard therefore can never print
    nothing. I ran the intended guard with the class written as `[-_ ]` instead.
    Even corrected, "value position" still matches comments and test names in an unrelated
    sense: YAML key/value positions (biscuit-file), CLI option-value positions (biscuit-tui),
    editor cursor positions (darkmatter DMLS `Position::Value`, `autocomplete.md`), expression
    parser arrows, tree-hugger, and recorded agent transcripts (claudine/agent-output,
    claudine/lib/tests/fixtures/providers). The spec bans "value position" only in the slot
    sense, so I left those alone rather than edit packages outside this feature. The
    slot-sense guard (`value[-_ ]slot|value-position[-_ ]slot|whole[-_ ]type[-_ ]slot|
    ValueSlotKind|DynamicValue`, excluding the two transcript folders) prints nothing.
    Alternative: rename every "value position" in the repo so the plan's literal command is
    silent; that touches biscuit-file, biscuit-tui, tree-hugger and recorded fixtures for no
    behavioral gain. Redo cost if the reviewer prefers that: a mechanical sweep of ~25 lines.
  - |-
    Plan rule R2 says Claudine's "value slot" becomes "setter value" at every listed site. Several
    of those sites are not about a `name=value` setter at all but about the value that follows a
    CLI option (`--asp <file>`, `--debug <level>`, `-c <value>`, clap's value-bearing options).
    Calling those "setter value" would be wrong, so I used "option value" there (and in renamed
    test functions such as `wrapper_asp_flag_option_value_emits_no_candidates`) and "setter
    value" only where the text is about `name=value`. Redo cost if the reviewer wants R2's single
    term: a text substitution in about ten comment lines and three test names.
  - |-
    Tuple-element check placement. The plan (R11) said to move the tuple element checks that read an
    element's type into `TupleType::check_element` and call it at parse for concrete atoms. In the code no
    such element check exists: every type-reading check in `parse_tuple` is about the spread, which refuses a
    template type anyway. I added `TupleType::check_element` (no description on an element, plus the union
    invariants) for template-type resolution to call after substitution, and did NOT call it at parse,
    because running the union invariants (notably the `non-null` rule) on authored tuple elements would have
    changed what static schemas accept. Alternative: call it at parse too, accepting that change. Redo cost:
    one call in `grammar.rs::parse_tuple` plus tests for any newly rejected static schema.
  - |-
    Extra refused position: `narrows(...)` targets. The plan lists tuple spread, function parameter, function
    return, and `property(...)` as refused grammar positions. A `function(...)`'s `narrows(name: T; ...)`
    target is also inside a function signature, so I refused a template type there too, with its own name
    in the message ("a `function(...)` `narrows(...)` target"), and made every refused position sticky for
    anything nested inside it (for example a tuple inside a function return). Alternative: allow it; nothing
    could resolve it, since signatures never resolve slots. Redo cost: delete one enum variant and its use.
  - |-
    Root `$schema` refusal is narrow. A root `$schema` string (or root-union string arm) is refused only when
    it is one leading `{{ … }}` slot followed by nothing or by a type suffix (`(`, `[`, `|`, `->`). A file
    reference that merely starts with slot text, such as `{{ dir }}/schema.yaml`, keeps its existing
    classification, because Darkmatter interpolates frontmatter before validation and such a path is
    meaningful. Alternative: refuse any root string starting with `{{`. Redo cost: one condition in
    `simplified/mod.rs::refuse_root_slot` and its test.
  - |-
    Grouped unions accept a template type. `({{ x }} | string)[]` (an array whose item union has a template
    arm) is accepted, as is `({{ x }})[]`. The spec's table names "a property-level union arm"; I read a
    grouped union written as a property's type as the same position (it is the property's own type), and
    substitution must flatten a returned union into the item union. Alternative: refuse inside groups.
    Redo cost: set a refused `SlotPosition` in `grammar.rs::parse_group` and adjust two tests.
  - |-
    `TypeDefinition::is_required()` (the public schema view) now answers `false` for a pending template
    type, and a new `TypeDefinition::requiredness()` returns `Requiredness::Pending`. The spec says a
    `(required)` suffix alone cannot establish presence before resolution, and R26 moves callers to
    `requiredness()`. Alternative: leave `is_required` reading the suffix and document it as resolved-only.
    Redo cost: one line in `schema/view.rs` and one assertion.
  - |-
    "Quote the slot" proof and extra site. R7 said to hint when the decode error's line, or the line before,
    holds an unquoted slot. That rule alone blamed a slot for an unrelated error on the next line, so the
    hint is given only when quoting that value makes the decode error disappear or move later
    (`slots::decode_error_slot`). A schema file's most common mistake (`status: {{ x }}`) also never reached
    the mapping-key error the plan named: the envelope's JSON conversion fails first, so the hint is wired
    there too (`envelope.rs`). The message names the real key (`quote the slot: write \`status: "{{ … }}"\``).
    Darkmatter frontmatter's fallback parser already rescues the common unquoted forms as strings, so the
    frontmatter hint fires only when decoding still fails. Alternative: the plan's literal line rule.
    Redo cost: drop the proof call in two places and the envelope wiring.
  - |-
    Late dependencies of a template type go into the result's `imports` (and `searched`), not
    `referenced_files` as the plan's R9 wrote. The field docs reserve `referenced_files` for the schema
    files a `$schema` names directly, while `imports` is the list of `Name@file` import edges, which is what
    a returned definition's import is. Both are cache dependency edges in Darkmatter, so nothing else
    changes. Example files a returned definition names reach the result's `examples` through the normal
    lowering pass. Redo cost if the reviewer wants R9 literally: one line per call site (four) plus
    `schema/layer.rs`.
  - |-
    Each template-type occurrence is expanded on its own (a synthetic namespace holding only the returned
    definition, keyed on the slot's defining file) and then spliced into its position, rather than
    re-expanding the whole enclosing property. A property can hold slots from different files (a union whose
    arm came from an imported named type), so one namespace per property would have used the wrong import
    base. Splicing mirrors `expand_union_arms` (hoisted markers, flattening) and the union invariants are
    re-checked afterwards. Trade-off: a type that a returned definition imports AND the schema imports
    statically is selected for `$defs` storage by each expansion separately, so its JSON shape (inline vs
    `$ref`) can differ from the all-static schema even though validation is identical. Redo cost: a
    whole-property re-expansion would need per-arm namespaces in the import engine.
  - |-
    Lowering is checked inside substitution. The plan's matrix names conflicting defaults as the suffix
    conflict; a review test found that a suffix constraint the returned type cannot take
    (`"{{ t }}(pattern('^a'))"` returning `boolean`) otherwise "resolved" into a schema that only failed
    later at export, and `SimplifiedSchema::resolve_slots` keeps a lowering failure on the layer rather than
    reporting it. Now the bare expansion must lower (else `NotATypeDefinition`) and the suffixed one too
    (else `SuffixConflict`), skipped only while a template constraint in the suffix is still unfilled.
    Alternative: let lowering report it as an ordinary `Convert` error. Redo cost: delete two calls in
    `slots/template_type.rs`.
  - |-
    Examples a returned definition names are gated. A slot written in a file gets them rehomed by its
    expansion under the gate. A slot with no defining file (an inline document `$schema`) has no file
    namespace, so its returned definition's examples are pre-resolved against the document directory under
    the gate before expansion; `example(this)` there fails, as it does for any inline schema. Pattern keys
    (`<pattern::…>`) in a returned inline object count as introduced patterns too. Neither detail was in
    the plan. Redo cost: small, in `resolve.rs::rehome_returned_examples` and `template_type.rs::def_patterns`.
  - |-
    Read admission is strict and deduplicated. Each (slot, path, kind) is asked at most once per resolution
    (the bare and the suffixed expansion would otherwise ask twice); `observe` is not deduplicated, so a host
    may see a read reported twice and must treat observations as a set. A reference whose candidates
    cannot be planned fails as `SchemaError::Unresolved` without probing, and every location the selection
    actually probed is re-admitted after the fact, so an unplanned probe can never feed a result. Also
    added: a public `slots::RefuseTemplateReads` (refuse every read) for hosts and doc examples, which the
    plan's R3 name list did not include. Redo cost: remove the `ADMITTED` set in `slots/reads.rs`, or the
    helper type and its three uses.
  - |-
    A wrong JSON shape (null, mapping, number, boolean) is `NotATypeDefinition` wrapping a
    `SchemaError::Grammar` whose message is "expected a type definition (a string or an array), found
    <kind>", because R4 gave `NotATypeDefinition` an inner `SchemaError` rather than a `WrongShape` kind.
    Alternative: use `SlotResultKind::WrongShape` for these four. Redo cost: one branch in
    `definition_from_value` and its tests.
source_files_during_phase_1:
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/recursive.rs
- simplified-schema/lib/src/schema/view.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/src/simplified/grammar/slot_tests.rs
- simplified-schema/lib/src/simplified/lint.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/narrowing.rs
- simplified-schema/lib/src/simplified/serialize.rs
- simplified-schema/lib/src/simplified/source.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/src/slots/substitute.rs
- simplified-schema/lib/tests/fixtures/dynamic_slots/template-constraints.yaml
- simplified-schema/lib/tests/l1/declaration_grammar.rs
- simplified-schema/lib/tests/l1/dynamic_slots.rs
- simplified-schema/lib/tests/l1/schema_object.rs
- darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
- darkmatter/lib/tests/l1/dynamic_schema_slots.rs
- darkmatter/cli/tests/l1/hash_kind_save_diff.rs
- claudine/cli/src/argv/partition.rs
- claudine/cli/src/argv/partition/tests.rs
- claudine/cli/src/completion/engine/mod.rs
- claudine/cli/src/completion/engine/tests.rs
- claudine/cli/src/completion/schema_completion/candidates.rs
- claudine/cli/src/completion/schema_completion/tests.rs
- claudine/cli/src/completion/setter_value.rs
- claudine/cli/tests/common/owned_value_options.rs
- claudine/cli/tests/l1/completion_cli.rs
- claudine/cli/tests/l1/completion_ownership.rs
- claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap
docs_updated_during_phase_1:
- simplified-schema/docs/topics/language/slots.md
- simplified-schema/lib/tests/fixtures/dynamic_slots/README.md
- darkmatter/docs/topics/schemas/schema-slots.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
- claudine/docs/topics/completions/shell-completions.md
- claudine/docs/topics/composition.md
- claudine/docs/topics/execution-flow.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/schema.md
packages:
- simplified-schema
- darkmatter
- darkmatter-cli
- darkmatter-lsp
- claudine
- claudine-cli
- claudine-gen
message_to_agent: |-
  Phase 3 (library resolution engine) is done; Phase 4 replaces Darkmatter's stub read gate.
  - `SlotHost::template_reads() -> Rc<dyn TemplateReads>` is required. `DarkmatterSlotHost::template_reads`
    (darkmatter/lib/src/markdown/schemas/slots/host.rs) currently returns `UngrantedTemplateReads`, which refuses
    every read with `SLOT_APPROVAL_REQUIRED_CODE` and observes nothing; R18 replaces it. Import-free returned
    definitions already resolve end to end in Darkmatter today.
  - `TemplateReads::admit(slot, SchemaRead { path, kind: Content|Probe })` is asked once per (slot, path, kind)
    per resolution, before anything is probed or read; `observe(slot, SchemaReadOutcome { path, kind, bytes, exists })`
    is called after each probe/read, including failures, and can repeat for the same read (bare + suffixed
    expansion): treat observations as a set. Probe paths are the lexical candidate paths (tempdirs show `/var/…`
    vs `/private/var/…` spellings).
  - A refusal surfaces as `SlotResolutionError::Refused(vec![(slot, SlotAdmissionError)])` with your code kept
    (sticky: it outranks whatever the expansion made of it). `SchemaError::SlotReadRefused` only appears inside
    the library; `resolve_with`'s `Attempt::Refused` maps it.
  - Late imports appear in `ResolvedSchema.imports`/`searched` (not `referenced_files`) and in
    `SchemaProvenance` for `SimplifiedSchema::resolve_slots`. Replays run under `buffers::with_reads_retained`.
  - New `SlotResultKind`s are produced now (`NotATypeDefinition`, `NestedSlot`, `ReachableSlot` with `related`
    naming the slot's file and span, `LocalReference`, `SuffixConflict`, `InvalidExpansion`, `PatternRejected`
    for returned patterns), each with `property: Some(path)`. Darkmatter's `slot_result_code` still maps them
    through its fallback; Phase 4 should give them codes.
  - The darkmatter docs (`schema-slots.md`, `authoring-schemas.md`) now say Darkmatter refuses a returned
    definition's reads with approval_required (marked planned); update them when R18 lands.
source_files_during_phase_2:
- claudine/gen/src/schema_compat.rs
- claudine/lib/src/composition/schema/classify.rs
- darkmatter/cli/src/commands/schema/assignment.rs
- darkmatter/dmls/src/overlay/named_types.rs
- darkmatter/dmls/src/overlay/suggestions.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/lib/src/lifecycle/schema_projection.rs
- darkmatter/lib/src/markdown/compose/context/catalog.rs
- darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/data/contract.rs
- darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/frontmatter.rs
- darkmatter/lib/src/markdown/schemas/completion.rs
- darkmatter/lib/src/markdown/schemas/detect.rs
- darkmatter/lib/src/markdown/schemas/triggers/finding.rs
- darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
- simplified-schema/lib/src/completion.rs
- simplified-schema/lib/src/envelope.rs
- simplified-schema/lib/src/errors.rs
- simplified-schema/lib/src/file_match.rs
- simplified-schema/lib/src/format.rs
- simplified-schema/lib/src/frontmatter_shape.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/phase.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/graph.rs
- simplified-schema/lib/src/resolve/recursive.rs
- simplified-schema/lib/src/schema/mod.rs
- simplified-schema/lib/src/schema/view.rs
- simplified-schema/lib/src/simplified/convert.rs
- simplified-schema/lib/src/simplified/definitions.rs
- simplified-schema/lib/src/simplified/grammar.rs
- simplified-schema/lib/src/simplified/lint.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/narrowing.rs
- simplified-schema/lib/src/simplified/query.rs
- simplified-schema/lib/src/simplified/serialize.rs
- simplified-schema/lib/src/simplified/source.rs
- simplified-schema/lib/src/simplified/standalone.rs
- simplified-schema/lib/src/simplified/subtype.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/src/slots/substitute.rs
- simplified-schema/lib/tests/l1/dynamic_slots.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/src/slots/unquoted.rs
- simplified-schema/lib/tests/fixtures/dynamic_slots/template-types/properties.yaml
- simplified-schema/lib/tests/fixtures/dynamic_slots/template-types/status-type.yaml
- simplified-schema/lib/tests/l1/quote_the_slot.rs
- simplified-schema/lib/tests/l1/template_types_passive.rs
docs_updated_during_phase_2:
- simplified-schema/docs/topics/language/slots.md
- simplified-schema/lib/tests/fixtures/dynamic_slots/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/schema.md
- .claude/skills/simplified-schema/SKILL.md
source_files_during_phase_3:
- simplified-schema/lib/src/slots.rs
- simplified-schema/lib/src/slots/host.rs
- simplified-schema/lib/src/slots/pending.rs
- simplified-schema/lib/src/slots/substitute.rs
- simplified-schema/lib/src/slots/reads.rs
- simplified-schema/lib/src/slots/template_type.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/select.rs
- simplified-schema/lib/src/schema/mod.rs
- simplified-schema/lib/src/schema/layer.rs
- simplified-schema/lib/src/simplified/definitions.rs
- simplified-schema/lib/src/simplified/query.rs
- simplified-schema/lib/tests/l1/main.rs
- simplified-schema/lib/tests/l1/dynamic_slots.rs
- simplified-schema/lib/tests/l1/template_types_passive.rs
- simplified-schema/lib/tests/l1/template_type_results.rs
- simplified-schema/lib/tests/l1/template_type_imports.rs
- simplified-schema/lib/tests/l1/template_type_refusals.rs
- darkmatter/lib/src/markdown/schemas/slots/host.rs
docs_updated_during_phase_3:
- simplified-schema/docs/topics/language/slots.md
- darkmatter/docs/topics/schemas/schema-slots.md
- darkmatter/docs/topics/schemas/authoring-schemas.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/simplified-schema/SKILL.md
- .claude/skills/darkmatter/schema.md
---

# Implementation Log for 2026-11-08-template-types (8 phases)

## Phase 1

### Wave 1 — Rename

- Symbols (R1): `ValueSlotKind` → `TemplateConstraintKind`, `Constraint::DynamicValue` →
  `Constraint::TemplateConstraint` in every Rust file (14 files, all in simplified-schema;
  no downstream crate named either symbol).
- Prose in simplified-schema source (`grammar.rs` module doc and three item docs, `types.rs`,
  `slots.rs`, `slots/substitute.rs`, `schema/view.rs`): "value position"/"value slot" in the
  slot sense → "template constraint"/"constraint argument".
- Fixture `lib/tests/fixtures/dynamic_slots/value-positions.yaml` → `template-constraints.yaml`
  (`git mv`); references in `tests/l1/dynamic_slots.rs`, `tests/l1/schema_object.rs`
  (`include_str!` literals stay inside each `#[test]`), and the fixtures `README.md`.
- Test names: `slot_tests.rs` (3), `dynamic_slots.rs` (`every_template_constraint_takes_its_shape`,
  module `template_constraints`, `a_document_template_constraint_slot_is_spanned_…`).
- Docs: `simplified-schema/docs/topics/language/slots.md` intro now names both kinds;
  "Whole Types (Planned)" → "Template Types (Planned)" (still planned, anchor
  `#template-types-planned`); darkmatter `schema-slots.md` and `authoring-schemas.md` use the
  new terms and the new anchor. Skills: `.claude/skills/simplified-schema/SKILL.md`,
  `.claude/skills/darkmatter/schema.md`.
- Darkmatter test wording: `triggers/grammar.rs` test-case label, `dynamic_schema_slots.rs`
  doc comment, `hash_kind_save_diff.rs` comment (`{date}` is a "value placeholder").
- Claudine (R2): see `decisions_for_review` — "setter value" for `name=value`, "option value"
  for clap option values. Three test functions in `completion_cli.rs` and one in
  `engine/tests.rs` renamed.
- No file listed by the plan under `dmls-schema-support.md`, `dmls/docs/autocomplete.md`,
  `.claude/skills/darkmatter/dmls.md`, `recursive_schema_editor.rs`, `schema_union_matrix.rs`,
  or `dynamic_slots_fixtures` held a slot-sense hit ("whole type" there means a type
  displayed whole, or a wrong whole value), so they are unchanged.
- Note: while this phase ran, the author committed the working tree, including this phase's
  rename, as `2c69d8424` ("feat(simplified-schema): add template types and rename
  constraints"); the plan/log/spec landed in `f0fd83b76`. The working tree after that held only
  the plan's checkboxes, so `source_files_during_phase_1` lists the files this phase edited
  even though they are already committed.

### Wave 2 — Rename guard

Guard command (character class corrected, transcripts excluded):

```sh
git grep -n -i -E "value[-_ ]slot|value-position[-_ ]slot|whole[-_ ]type[-_ ]slot|ValueSlotKind|DynamicValue" \
  -- ':!**/_completed/**' ':!darkmatter/features/2026-11-08-template-types/**' \
  ':!claudine/agent-output/**' ':!claudine/lib/tests/fixtures/providers/**'
```

Result: no output (exit 1). Every remaining "value position" hit is the unrelated sense listed in
`decisions_for_review`.

Requirement-to-test mapping: the phase changes no behavior (two symbol renames, a fixture file
name, test names, and prose). Proof is the unchanged existing suites: the renamed fixture is
read by `dynamic_slots::every_template_constraint_takes_its_shape`,
`schema_object::…::every_shipped_template_fixture_is_admitted_without_lowering`, and
`export_keeps_every_slot_as_authored_and_round_trips`; the renamed constraint is exercised by
`slot_tests::template_constraints_*`, `declaration_grammar` and `dynamic_slots::template_constraints::*`.
No new test was needed (no behavior to pin).

Gates:

- simplified-schema `just test`: 1454 passed, 0 skipped; `just test-minimal`: 1454 passed;
  `just lint`: clean.
- darkmatter `just test`: 16248 passed, 10 skipped. Twelve existing tests ran just over 5 s
  (compose_shell_template, schema_validate_baseline, standalone_baseline, claudine_editor_parity,
  …); none is touched by this rename and the host was shared at the time, so they are recorded
  as load-sensitive rather than rerun. `just lint`: clean (including the zed-dmls wasm check).
- claudine `just test`: the first run stopped on `claudine-cli::l1 wrap_basics::help_lists_wrapper_subcommands`.
  This failure was already there and has nothing to do with the rename: commit `d41d24a40` ("correct grouped help command
  registry", 2026-11-08) intentionally added the `kilo`, `pi`, and `antigravity` rows and removed
  the stale `budget` row from grouped help, but did not update the insta snapshot. I accepted
  the snapshot to match that intended output (`claudine/cli/tests/l1/snapshots/l1__wrap_basics__help_lists_wrapper_subcommands.snap`;
  insta also corrected its `source:` header to the consolidated `tests/l1/` path). Rerun:
  11845 passed, 15 skipped. `just lint`: clean.
- Downstream packages: the two renamed symbols are used only inside simplified-schema; its
  dependents darkmatter (lib, cli, dmls) and claudine compiled and passed their full suites
  above, so no further downstream run is needed.
- Cross-OS: no path, process, or OS-specific code changed (renames and prose only), so no
  `just cross-check` was run.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
  python3 scripts/ci/affected_scope.py`): `packages: [claudine, claudine-cli, darkmatter,
  darkmatter-cli]`, `change_class: package`. The scope is not empty, so the gates were required, and all of them were run (see above).
  (The working tree at that point held only the snapshot and the plan/log edits, because the
  rest of this phase was already committed in `2c69d8424`.)

Checkpoint 1: guard silent (slot sense), all three areas green, behavior unchanged.

## Phase 2

### Wave 3 — The atom and the grammar

- `TypeExpr::Slot(Slot)` (types.rs) is the pending type, with `PropertyAtom::is_pending_type`,
  `PropertyDef::has_pending_type`, `Requiredness { Required, Optional, Pending }` and
  `PropertyDef::requiredness()` (re-exported at the crate root). `PropertyDef::check_union_invariants` and
  `TupleType::check_element` exist for substitution (see `decisions_for_review` on why `check_element` is not
  called at parse).
- The union checks that were copied three times (string definition, inline-object member, YAML list union) are
  one function, `grammar::union_invariant_problem` → `UnionInvariant`; each caller keeps its own span mapping,
  so every existing error message and span is unchanged (proved by the unchanged suite).
- Errors (R4/R3): `SlotRelated`; `SchemaError::SlotResult { slot, kind, property, related }`;
  `SlotResultKind::{NotATypeDefinition(Arc<SchemaError>), NestedSlot, ReachableSlot, LocalReference,
  SuffixConflict, InvalidExpansion}` (manual `PartialEq`); `SchemaError::SlotReadRefused { slot, refusal }`;
  status-block rendering for both.
- Grammar (R5/R6): `parse_template_type` reads the slot with the existing `read_slot` and the suffix with
  `parse_reference_suffix`, now shared with `parse_named_reference`. `SlotPosition` + `refusal()` builds the
  one message shape; positions are sticky inside a refused construct. Refused in the grammar: tuple spread,
  function parameter, return, `narrows` target, `property(...)`, `{{` after `@`, slot joined to a type name or
  `@`, inline-object key. Refused in the YAML-shape layer (`simplified/mod.rs`): root string, root-union string
  arm, property key, pattern key.
- Slot collection (R25): `collect_slots`/`definition_slots`/`reachable_slots`, origin stamping, `def_refs`
  and `omit_properties` handle the atom; `Substitution::atom` passes it through (Phase 3 replaces that arm).

### Waves 4–5 — Readers (R26/R27 audit)

Adding the variant broke 28 exhaustive matches in simplified-schema, 20 in darkmatter (lib, cli, dmls), and 3
in claudine (lib, gen). Each got an explicit arm:

- (A) template-aware: `query.rs` (`suggestions_for_def` → `pending: true`, no items; `flattened_alternatives`
  exposes the atom), `subtype.rs` (identical slot only), `narrowing.rs` (`families` → all,
  `container_admission` → Provisional), `completion.rs` (`pending`), `lint.rs`, `definitions.rs`, `phase.rs`
  (a pending atom is left as authored), `file_match.rs`, `frontmatter_shape.rs`, `format.rs` (forms judged
  after resolution), `serialize.rs` (`{{ expression }}` + suffix), `source.rs` (spans projected, including a
  `|` union in one scalar and tuple elements, which were not projected before), `schema/view.rs`
  (`TypeDefinition::requiredness()`), DMLS hover ("type pending") and suggestions, Claudine `classify.rs`
  and `gen/schema_compat.rs` (unsupported shape naming the template type).
- (B) expression typing → untyped: `call_check.rs`, `call_binding/translate.rs` (`Shape::Any`),
  `scope/iterable.rs`, `data/contract.rs`, `context/catalog.rs`; `lifecycle/schema_projection.rs` reports
  an unsupported parameter type.
- (C) lowering: `convert.rs`'s per-atom guard now includes the type slot and the type match has a defensive
  `UnresolvedSlots` arm; `resolve.rs`/`graph.rs`/`recursive.rs` pass it through.
- Wildcards: the audit script (every `match` on `.ty` with a `_ =>` arm, every `matches!`/`if let`/`let …
  else` on `TypeExpr`) listed 85 sites in simplified-schema and about 50 in darkmatter/claudine. All but
  three answer "not this specific type", which is correct for a pending type. The three that would have
  claimed a fact were changed: `format.rs::alternative_form` (was OtherForm → Admitted),
  `narrowing.rs::container_admission` (was NotContainer → Provisional), `query.rs::suggestions_for_def`
  (was None → pending). `signature.rs` canonicalizes binding contracts only and lowers nothing.
- "Quote the slot" (R7, subagent): `slots/unquoted.rs` (`unquoted_slot_hint`, `decode_error_slot`,
  `quote_the_slot_message`), wired in `simplified/source.rs` (mapping-key error, source-aware paths),
  `simplified/standalone.rs` (decode failure), `envelope.rs` (JSON-conversion failure, the path a schema
  file's unquoted `status: {{ x }}` actually takes), and darkmatter `markdown/frontmatter.rs` (final decode
  failure only). Docs: new subsection in `simplified-schema/docs/topics/language/slots.md`.

### Requirement → test mapping

| Requirement | Tests |
| --- | --- |
| Allowed positions, both YAML forms, suffixes, `requiredness` | `template_types_passive::{a_property_type_parses_*, a_nested_object_property_*, a_named_type_body_*, a_union_arm_*, a_grouped_union_*, a_tuple_element_*, every_fixture_position_*, every_suffix_*, requiredness_is_pending_*}` |
| Quoted brace text stays literal | `quoted_brace_text_in_an_enum_member_or_default_is_no_slot` |
| Passive query/export evaluate nothing; export round-trips | `query_property_and_named_type_answer_from_the_authored_template` (also `TypeDefinition::{is_required, requiredness}`), `export_writes_each_template_type_*`, `serializing_a_slot_atom_*` |
| Lowering/validate/layer return `UnresolvedSlots` (one per entry point) | `to_json_schema_on_the_object_is_unresolved`, `validate_strict_*`, `validate_soft_*`, `layer_under_*`, `layer_over_*`, `lowering_a_parsed_root_*`, `lowering_with_definitions_*`, `resolve_yaml_schema_*`, `resolve_schema_from_json_*` |
| Reader answers | `suggestions_for_a_template_type_are_pending_and_empty`, `completion_offers_nothing_*`, `a_template_type_is_a_subtype_only_of_the_identical_slot`, `static_union_arms_still_prove_*`, `narrowing_and_container_admission_*`, `flattened_alternatives_expose_the_slot_atom` |
| Refused positions (one test each) | `refused_at_the_root`, `refused_in_a_root_union_arm`, `refused_as_a_yaml_property_key`, `refused_as_an_inline_object_property_key`, `refused_in_a_pattern_key`, `refused_in_an_import_path`, `refused_after_a_type_name`, `refused_before_a_type_name`, `refused_before_an_import_suffix`, `refused_in_a_tuple_spread`, `refused_as_a_function_parameter`, `refused_as_a_function_return`, `refused_as_a_narrows_target`, `refused_in_a_property_declaration` |
| Spans: plain, quoted, block, frontmatter, CRLF | `a_plain_scalar_*`, `quoted_template_types_project_*`, `block_scalar_*`, `a_yaml_list_arm_projects`, `a_union_in_one_scalar_and_tuple_elements_project`, `markdown_frontmatter_*`, `crlf_line_endings_*`, `a_factory_built_fixture_*`, `a_fixture_read_from_its_file_*`, `a_refused_template_type_is_located_*` |
| Quote the slot (schema file + frontmatter, positive and negative) | `quote_the_slot::*` (14 tests), darkmatter `frontmatter::tests::{an_unquoted_schema_slot_that_breaks_yaml_reports_quote_the_slot, an_unquoted_schema_slot_and_interpolation_still_parse, an_unrelated_frontmatter_error_keeps_its_message}` |

Fixtures: `simplified-schema/lib/tests/fixtures/dynamic_slots/template-types/{properties,status-type}.yaml`.
Defect found by the tests and fixed: an import-path refusal span ended at `{{x` (the file-reference scanner
stops at `}`); it now ends where the slot ends.

### Gates

- Scope calculator: `packages: [claudine, claudine-gen, darkmatter, darkmatter-cli, darkmatter-lsp,
  simplified-schema, test-toolkit]`, `change_class: package`; gates required and run.
- simplified-schema: `just test` 1521 passed; `just test-minimal` 1521 passed; `just lint` clean.
- darkmatter: `just test` 16251 passed, 10 skipped. Eight existing tests ran just over 5 s
  (`compose_remote_caching`, `compose_report_diagnostics`, `compose_shell_template`, `frontmatter_key_order`,
  `render_basic`, `render_dispatch`, `claudine_editor_parity`, `content_policy_baseline`); none is touched by
  this phase, so they are recorded as load-sensitive. `just lint` clean (including the zed-dmls wasm check).
- claudine (dependent): `just test` 11845 passed, 15 skipped; `just lint` clean.
- Other dependents of simplified-schema/darkmatter (messenger, research, sniff-cli, …) match no `TypeExpr` or
  changed error field; the workspace `cargo check` of the touched packages is clean.
- Cross-OS: no path, process, or OS-specific code changed (grammar, AST, readers, and text scanning), so no
  `just cross-check` was run.

## Phase 3

### Wave 6 — Boundary and plumbing

- `slots/reads.rs` (new): `TemplateReads { admit, observe }`, `SchemaRead`, `SchemaReadKind::{Content, Probe}`,
  `SchemaReadOutcome`, `RefuseTemplateReads`, the scope `with_template_reads(reads, f) -> (T, first refusal)`,
  the crate-private `expanding(slot, f)` scope and `ReadGate` (`admit` records the first refusal and is asked
  once per (slot, path, kind) per resolution; `read` admits, reads through `buffers::read`, and observes success
  or failure). `SlotHost::template_reads` is required; the in-repo hosts were updated (`StubHost`,
  `PanickingHost`, the `schema/mod.rs` doc example, `slots.md`, `DarkmatterSlotHost`, which refuses every read
  with `SLOT_APPROVAL_REQUIRED_CODE` until Phase 4).
- `PendingSchema::resolve` and `SimplifiedSchema::resolve_slots` install the host's reads for each attempt; a
  recorded refusal becomes `Attempt::Refused` → `SlotResolutionError::Refused`, whatever the replay returned.
  Both replays run under `buffers::with_reads_retained` (R17).
- `apply_active`/`apply_active_to_arm` take an `ExpansionSite` and return `LateDependencies { imports, searched }`;
  the four `resolve.rs` sites and `schema/layer.rs::lower_expanded` (a fifth site the plan did not list) merge
  them.
- Import engine: `ImportEngine.template_gate` is captured from the `expanding` scope only. Guarded paths:
  `gated_schema_file_path` (probes admitted through the new `select::select_schema_file_checked` before any is
  made, then every probe observed), `resolve_namespace` (Content admitted and the bytes read before the engine's
  namespace cache or the process memo is consulted; `load_named_types` was split so the memo parses the gated
  bytes), and `rehome_atom_examples` (now takes `this_file: Option<&Path>` and the gate).

### Wave 7 — Substitution

- `slots/template_type.rs` (new): `substitute_types` walks the root and every stored body (keys via the new
  `DefinitionTable::entries_mut`), depth first, and replaces each `TypeExpr::Slot` with a value:
  `definition_from_value` (string/array only; parser errors kept in `NotATypeDefinition`; `NestedSlot`,
  `LocalReference`), then `resolve::expand_template_definition` twice (synthetic namespace of the slot's
  defining file holding only the returned definition; bare, then with the written suffix). The bare expansion
  is checked for reachable slots (`ReachableSlot` + related file/span), introduced patterns (constraints and
  `<pattern::…>` keys, in the definition and every stored definition it reaches) and that it lowers; the
  suffixed one maps merge/lowering failures to `SuffixConflict`/`InvalidExpansion`. Single positions are
  replaced; union arms (property-level and grouped) are spliced as `expand_union_arms` does, with hoisted
  markers; union invariants and `TupleType::check_element` are re-checked where a slot was replaced. Stored
  definitions published by the expansions are absorbed into the site's table. Then the existing constraint
  substitution runs.
- A refusal recorded during an expansion outranks its result (`refusal_first`).
- Suffix messages never show the synthetic name (`returned_type_wording`).

### Wave 8 — Resolution tests

Three files were written in parallel by subagents (each verified alone); one failing test they reported
(`suffix_constraint_over_a_type_without_it_is_rejected`) exposed a real defect, fixed by checking lowering
inside substitution (see `decisions_for_review`). The `test-minimal` run then showed the file-reading modules
needed `#![cfg(feature = "file")]` and the `opened` assertions `work-counters`; both gated like their siblings.

### Requirement → test mapping

| Requirement | Tests |
| --- | --- |
| Equivalence with the static schema (JSON + strict validation of a valid and an invalid document) | every positive test in `template_type_results.rs` (`enum_value_equals_static_enum` is the control) and `template_type_imports.rs` |
| R8/R28 result shapes (null, mapping, number, boolean, `[]`, `""`, `["string",123]`, `[123]`, `"null"`, `[null]`, string-encoded list) | `non_string_non_array_values_are_not_type_definitions`, `empty_array_keeps_the_ordinary_union_error`, `empty_string_keeps_the_ordinary_definition_error`, `array_with_a_non_string_element_is_rejected_not_filtered`, `null_string_is_the_null_type`, `array_holding_json_null_is_the_null_type_list_form`, `string_encoded_list_is_a_union` |
| R11 union flattening, array suffix, presence | `array_result_flattens_into_a_yaml_list_union`, `array_result_flattens_into_a_pipe_union`, `array_suffix_applies_to_the_whole_returned_union`, `required_suffix_over_null_admitting_types_stays_optional`, `required_suffix_over_string_is_required`, `nullable_items_do_not_make_the_array_nullable` |
| Suffix merge and conflicts | `conflicting_default_is_a_suffix_conflict`, `non_null_over_a_null_admitting_result_is_an_invalid_expansion`, `suffix_constraint_over_a_type_without_it_is_rejected`, `array_and_constraint_suffix_merge_like_a_named_type`, `template_constraint_in_the_suffix_still_resolves`, `description_suffix_survives_expansion` |
| Positions (nested object, named type body, tuple element, grouped union) and property path in errors | `inline_object_property_position`, `nested_mapping_property_position`, `nested_mapping_reports_the_property_path_for_a_wrong_shape`, `named_type_body_position`, `tuple_element_position`, `grouped_union_with_array_suffix_position` |
| `SlotKey` identity (evaluated once, substituted at both sites) | `named_type_template_used_twice_is_evaluated_once` |
| R10/R13 defining-file base, data file never the base | `inline_document_slot_imports_from_the_resolution_directory`, `referenced_schema_file_slot_imports_from_its_own_directory`, `slot_inside_an_imported_named_type_imports_from_that_files_directory`, `a_value_read_from_a_data_file_elsewhere_resolves_against_the_defining_file`, `local_references_inside_an_imported_file_resolve_in_that_file` |
| `LocalReference`, cycles distinct from refusals, recursive imports | `returned_local_references_are_rejected_as_local_reference`, `an_alias_cycle_in_returned_imports_is_the_static_cycle_error_not_a_refusal`, `a_self_recursive_imported_type_resolves_through_defs` |
| R17 read once; dependencies on success and failure | `a_shared_import_of_two_template_types_is_opened_once` (`work-counters`), `imports_and_searched_record_what_the_returned_definition_read`, `a_missing_returned_import_is_still_observed_as_probed` |
| R15/R16 gate before probe/open/memo, sticky typed refusal, static import does not authorize, gated examples | `a_refused_probe_stops_before_any_read_and_keeps_its_code`, `a_refusal_wins_even_when_a_static_property_imports_the_same_file`, `an_example_reached_by_a_returned_import_is_gated`, `refusing_only_the_example_refuses_the_resolution`, unit `resolve::named_type_file_memo::a_refused_template_type_import_never_reaches_the_memo` |
| R12 nested / reachable slots, literal brace text | `a_returned_definition_holding_an_active_slot_is_a_nested_slot`, `quoted_brace_text_in_a_returned_enum_member_or_default_stays_literal`, `a_returned_import_of_a_slot_bearing_type_is_a_reachable_slot`, `a_slot_reached_transitively_names_the_file_that_holds_it`, `an_unused_slot_elsewhere_in_an_imported_file_does_not_disqualify_it`, `a_statically_authored_import_of_a_slot_bearing_type_resolves_as_a_template` |
| R12 introduced patterns | `a_returned_pattern_at_the_byte_cap_is_accepted_and_one_past_it_refused`, `a_returned_pattern_with_lookaround_is_refused`, `a_returned_pattern_with_a_backreference_is_refused`, `a_returned_pattern_beyond_the_regex_size_limit_is_refused`, `a_lookaround_pattern_reached_through_a_returned_import_is_refused`, `a_lookaround_pattern_key_in_a_returned_inline_object_is_refused`, `an_authored_lookaround_pattern_resolves_as_before`, `a_lookaround_pattern_in_the_use_site_suffix_is_exempt` |

Fidelity: F1/L1, in-process at the crate's public API (`resolve_yaml_schema` → `PendingSchema::resolve`,
`SimplifiedSchema::resolve_slots`); the boundary crossed is the host trait, played by table-driven test hosts.
Darkmatter's end-to-end path (real host, grants, CLI) is Phase 4/5's. The Input Robustness Matrix applies to the
returned value only (`definition_from_value` is its one validator); the written suffix and the data path each
have a control and an invalid shape; no loader changed.

Code-smell check (Checkpoint 3): `rg -n "serde\(default\)|filter_map\(.*as_str|unwrap_or_default\(\)|\.ok\(\)"` over
`template_type.rs`, `reads.rs`, `select.rs`: `template_type.rs` formats an optional " through `file`" into a
message (not load-bearing); `select.rs`'s `first_planned_candidate` `.ok()?` is pre-existing; a third hit, the
new `planned_probes` swallowing a planning error, was load-bearing and was fixed (a planning failure is now
`SchemaError::Unresolved` with nothing probed).

### Gates

- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages: [claudine, claudine-gen, darkmatter, darkmatter-cli,
  darkmatter-lsp, simplified-schema, test-toolkit]`, `change_class: package`; gates required and run.
  (The working tree still holds Phase 2's uncommitted changes too, so the list covers both phases.)
- simplified-schema: `just test` 1789 passed (default + `work-counters,terminal`) and 1548 passed
  (`test-minimal`, `--no-default-features`); `just lint` clean.
- darkmatter: `just test` 16251 passed, 10 skipped; `just lint` clean (including the zed-dmls wasm check).
- Dependents: claudine, claudine-cli, claudine-gen scoped to `test(/slot|template|schema_completion/)`: 357
  passed (no crate outside darkmatter uses `SlotHost`, `PendingSchema`, or the changed crate-private
  functions; claudine reaches template types only through Darkmatter composition). `cargo check --tests` of
  messenger and the claudine crates is clean.
- Cross-OS: `just cross-check simplified-schema --os windows` could not run: SSH to `build-win-native` failed
  with "Host key verification failed" before any test ran (a host-key trust problem on this Mac, not a test
  failure; accepting an unknown host key is outside this session's authority). No Windows failure was
  observed, so `os_failing` is not set; the nightly Windows leg covers it. Path-sensitive points to watch
  there: `select::planned_probes` (relative candidates joined to the base or root) and the exact-path gate
  assertions in `tests/l1/template_type_imports.rs` (they normalize with `canonicalize_simplified`).
- No pre-existing failures; no load-sensitive reruns were needed.
