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
- claudine-cli
message_to_agent: |-
  Phase 1 (rename) is done. Things worth knowing for Phase 2:
  - The constraint variant is now `Constraint::TemplateConstraint { kind: TemplateConstraintKind, target, slot }`;
    write the new names only.
  - The plan's rename-guard regex uses `[ -_]`, a character range that matches unrelated text. Use
    `git grep -n -i -E "value[-_ ]slot|value-position[-_ ]slot|whole[-_ ]type[-_ ]slot|ValueSlotKind|DynamicValue" -- ':!**/_completed/**' ':!darkmatter/features/2026-11-08-template-types/**' ':!claudine/agent-output/**' ':!claudine/lib/tests/fixtures/providers/**'`
    (and `git grep`, not `rg`, from the repo root: `rg` over the worktree took more than 2 minutes).
    Remaining "value position" hits are an unrelated sense (YAML/cursor/CLI option positions) and are deliberately kept.
  - The planned section in simplified-schema/docs/topics/language/slots.md is now "Template Types (Planned)"
    (anchor `#template-types-planned`), linked from darkmatter schema-slots.md and authoring-schemas.md; Phase 8 removes the marker.
  - The author committed the working tree while Phase 1 ran (`2c69d8424`), so expect commits to land mid-phase; check
    `git log` before you assume an edit was lost.
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
