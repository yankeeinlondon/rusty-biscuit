---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-06-dotted-paths/spec.md
plan: darkmatter/features/2026-11-06-dotted-paths/plan.md
implemented_by: claude/opus
started_phase: "1"
decisions_for_review:
    - |-
        Compose setters keep reaching transcluded child documents, and each
        document reads a setter through its own schema. The plan (rule 16) chose
        this; the spec does not cover the case. `md compose p.md v=7` already
        changes a child's `v` today, because options flow down to children, so
        keeping it preserves current behavior. One new consequence: a nested
        setter that meets a scalar parent or an out-of-range index in a child
        fails the whole run and names that child.
        Alternatives: apply setters to the root document only, which makes
        children predictable but silently changes what `v=7` does today; or skip
        a child where the path cannot apply, which never fails on a child but
        hides a setter that did nothing.
        To choose another option, change the setter plan's application step in
        the pipeline and preflight callers, plus the child-document tests. The
        path parser, value parser, and text editor would not change.
    - |-
        The spec's phrase "recursive merge behavior of those older options" is
        wrong about `--set`, and later phases follow the code. `--set` replaces a
        top-level key whole and only `--state` deep-merges, as
        `compose/util.rs` shows. The plan called for this correction. Neither
        option changes, and the spec stays as written because a spec is a
        snapshot; current docs will describe the real behavior.
        The alternative was to make `--set` merge recursively so the spec's
        wording became true, but that changes the behavior of an existing option
        and the spec puts both options out of scope.
        If the reviewer wants `--set` to merge, the change is confined to the
        `--set` block of `prepare_frontmatter_for_compose` and its tests, and the
        documented precedence examples would need rewriting.
    - |-
        Under a declared type of `unknown`, a command-line value is guessed the
        way an undeclared one is, so `x=5` is the number 5. The plan says any
        declared shape reads the text through the conversion engine, which would
        keep "5" as a string, because `unknown` accepts anything unchanged.
        Following the plan literally would change what `x=5` does today for every
        key typed `unknown`, including keys in open objects, while the type says
        nothing about which reading is meant.
        Alternative: follow the plan, so a declared key is always text first; the
        author then writes numbers in a container or `--set` to get a number.
        To choose it, delete `declares_nothing` in `setter_value.rs` and its test.
    - |-
        A YAML flow value with a tag, anchor, or alias is read as plain text, not
        as an error. The plan says the flow reader rejects tags and block
        documents without saying what rejecting means, and does not mention
        anchors or aliases. Text fits the spec's "syntax rejection can fall back
        to text", and nothing in the value runs. A declared object type then
        fails schema validation with the text in view.
        Alternatives: fail with a setter error, which is clearer for someone who
        really meant a tag but breaks a plain string that merely looks like one;
        or allow anchors and aliases, which have no use within one argument.
        To change it, adjust `is_one_plain_flow_mapping` in `setter_value.rs`,
        plus its tests.
    - |-
        A bare number that cannot be held exactly, such as `9007199254740993`,
        becomes text when no type is declared. The plan says this "stays text as
        today", but reading today's `parse_shorthand_value` shows it stays a JSON
        number and only non-finite values become text. I took the plan's outcome,
        since the spec forbids an invalid number becoming a rounded value. The
        change is visible only to code that checks the value's type; it prints
        the same.
        Alternative: keep today's number, which keeps exact digits in the value
        but lets a number outside the numeric model into the pipeline.
        To change it, adjust `guess_scalar` in `setter_value.rs` and the
        `a_bare_inadmissible_number_stays_text` test.
    - |-
        A JSON5-quoted value is not converted when it is bound: `"5"` under
        `number` stays the string "5" at that step. The plan names binding for
        text and containers only. Schema validation then treats it exactly as it
        treats an authored `count: "5"`, so compose still gets 5.
        Alternative: bind quoted values too, so `md set` would write 5 rather than
        "5", overriding quotes the author typed on purpose.
        To change it, route the `Quoted` arm of `SetterValue::bind` through
        `bound_or_original`.
    - |-
        A value such as `{foo}` now reads as the object `{foo: null}`, where today
        it is the string "{foo}". This follows from the spec's YAML fallback, which
        requires `{ a:1, max-width: 2 }` to read `a:1` as a key with no value; the
        two cannot be told apart. `{{ template }}` values stay text, because their
        key would be a mapping, which no JSON object can hold.
        Alternative: refuse keys with no value, which keeps `{foo}` as text but
        contradicts the spec's example.
        To change it, add that check to `json_from_flow_yaml` and update the
        `{ a:1, max-width: 2 }` test.
    - |-
        Additions the plan did not list. `PathEditError::RemovalConflict` reports
        duplicate or ancestor removals, since none of the four planned variants
        describes them. `json5_is_well_formed` in biscuit-file tells a grammar error
        from an integer beyond `u64`: `Json5::from_str` fails on both, and treating
        that as syntax let `{a:99999999999999999999}` reach YAML as a key named
        `a:9999...`. `set_at` and `RemovalPlan` take the frontmatter `IndexMap` only,
        while `resolve` also takes a `Value`, because every planned caller edits a
        frontmatter map. `Resolved` uses the existing `PathStep` as its typed step
        rather than a parallel type.
        Alternatives would be a separate conflict error type owned by `md rm`, or
        value-rooted edits; either is a small change to `property_path.rs`.
    - |-
        I added `edited_document_text` beside the planned `edit_frontmatter_paths`.
        It holds the fallback in the library, so `md set` and `md rm` call one
        function and Phase 4's `edited_text` becomes a thin wrapper. The plan named
        only the editor and described the fallback as a rule. The fallback also
        differs from today's: it puts the serialized YAML between the source's own
        fences and keeps the body's bytes. Today's `Markdown::as_string` drops the
        body's final line break and turns CRLF into LF, which breaks the spec's rule
        that the body stays unchanged.
        Alternative: leave the fallback in the command and keep `as_string`, which
        is less code here but keeps that body damage.
        To change it, edit `serialized_into_source` in `hash/path_edit.rs`.
    - |-
        The editor changes a flow collection when the path names it whole, and
        treats an empty `{}` as a parent it can add keys to. The plan lists flow
        collections among the forms to refuse. Refusing only walks into a non-empty
        one keeps comments in more documents, and matters in practice: `md rm` leaves
        `{}` behind, and the next `md set` under it would otherwise re-serialize the
        whole frontmatter and lose every comment.
        Alternative: refuse every flow collection, which is a simpler rule but costs
        those comments.
        To change it, remove the `empty_mapping` branch in `walk`.
    - |-
        Only a parse failure that says `unknown anchor` counts as an orphaned alias,
        and has no fallback. Any other parse failure of the edited text is an
        unsupported form, which falls back. The plan does not say how to classify a
        result that no longer parses. The parser's wording is the only signal, so an
        upgrade that rewords it would turn this refusal into a fallback; the values
        would still be right, with the alias written out, and
        `removing_an_anchor_an_alias_needs_is_refused_with_no_fallback` would fail.
        Alternative: treat every parse failure as a refusal, which needs no wording
        check but blocks a save the fallback could do.
        To change it, edit `reload_values`.
    - |-
        Any difference outside the named paths is refused with no fallback, even if
        the document has no alias. This is what the plan says, but it means a mistake
        in the editor's own positioning would show as an error message about aliases,
        and the user could not save until the source changed. Nothing is written in
        that case. Testing found one such mistake, a new key cutting into a `|+`
        scalar, which is fixed.
        Alternative: refuse only when the source has an anchor, and otherwise fall
        back, which lets such a save succeed but no longer proves the editor correct
        by failing loudly.
        To change it, add that check in `verify_edited`.
    - |-
        Three comment choices the plan leaves open. A comment on the line above a
        removed node stays, because it is outside the node. A comment inside a
        replaced container goes with the container, except on its key's line, which
        stays. When a value grows to several lines, a comment after it moves to its
        first line, because a block scalar's last line would read it as content.
        Alternative: remove a comment directly above a removed node, on the guess that
        it describes that node; this tidies the common case but sometimes deletes a
        comment about something else.
        To change it, adjust `remove_node` and `in_place`.
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-file/lib/src/json5/mod.rs
    - darkmatter/cli/tests/l1/output_commands.rs
    - biscuit-file/lib/src/json5/scan.rs
    - biscuit-file/lib/src/lib.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/numeric.rs
    - simplified-schema/lib/src/numeric/ingress.rs
    - simplified-schema/lib/src/property_path.rs
    - simplified-schema/lib/src/setter_value.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/property_paths.rs
    - simplified-schema/lib/tests/l1/setter_values.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
    - biscuit-file
    - darkmatter
    - darkmatter-cli
    - simplified-schema
message_to_agent: |-
    Phase 3 added `darkmatter::markdown::hash::{PathEdit, PathEditRefusal,
    edit_frontmatter_paths, edited_document_text}`. For Phase 4, `edited_text` should
    call `edited_document_text(&md, &edits)` on the Markdown whose map already holds
    the pure edit: it runs the editor, applies the fallback, and verifies both. Build
    `PathEdit::Set(path.resolve_map(md.frontmatter().as_map()).steps().to_vec())` after
    `set_at`; build removals with `plan.targets().map(PathEdit::Remove)` from the plan
    made on the unedited map, in any order. A refusal means write nothing and report
    a run failure (exit 1). Calling with no edits while the map differs from the source
    is itself a refusal, so always pass the real edits. The editor does not reflow, so
    keep `at_width` after it. Four existing tests ran past 5 s under the full suite but
    take 2 to 3 s alone; they are listed in the Phase 3 log.
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/hash/path_edit.rs
    - darkmatter/lib/src/markdown/hash/mod.rs
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/lib/tests/l1/frontmatter_path_edit.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/frontmatter.md
---

# Implementation Log for 2026-11-06-dotted-paths (10 phases)

## Phase 1

Phase 1 changes no code. It records the contract the later phases implement and
establishes the test baseline they are measured against.

### Contract recorded from the plan

`plan.md` is the authority for the detail of each rule; this digest is for
orientation.

| Rule | Contract | Phase |
|---|---|---|
| 1 | `PropertyPath` in `simplified-schema`: `\.` and `\\` are the two escapes; the whole argument `''` is one empty segment; other empty segments are an error; printing round-trips | 2 |
| 2 | `PropertyPath::resolve` walks a value; a canonical decimal is an index only on an array; overflow is out of range; a `null` or scalar cannot be traversed; conversion to `PathStep` and `JsonPointer` needs the walk | 2 |
| 3 | Pure `set_at` and `RemovalPlan`/`remove_at` with `PathEditError`; mapping parents are created; a digit under a missing parent is an error; emptied parents stay | 2 |
| 4 | A path syntax failure is a usage error, exit 2; an edit failure or rejected value is a run failure, exit 1 | 4 |
| 5 | `SetterValue::parse`: JSON5, then YAML flow for a value starting with `{`, then text | 2 |
| 6 | Parsed containers are admitted at the token level through `json5_scan` and `numeric::admit_json5_text_numbers`; a bad container fails and never becomes text | 2 |
| 7 | `SetterValue::bind` converts through the declared `Shape`; with no declaration it keeps today's JSON5 guess | 2 |
| 8 | `hash/path_edit.rs` edits nested nodes and refuses forms it cannot edit safely | 3 |
| 9 | Every edit is reparsed and compared with the expected values before any write; `WouldChangeUnnamed` has no fallback; `UnsupportedForm` falls back to verified serialization | 3, 4 |
| 10 | `md get` stays schema-free and flat; paths are parsed before the document loads | 4 |
| 11 | `md set` blocks on a problem at or under the changed path, or on any new problem; `$schema` only checks its reference | 4 |
| 12 | `md rm` plans against the original; conflicts and missing targets cancel the command; array removals use original indexes | 4 |
| 13 | `compose::setters::classify_token` is the one setter/path classifier, shared by execution and completion | 5 |
| 14 | An ordered `SetterPlan` replaces the setter map; overlaps resolve by specificity, before any document is read | 5 |
| 15 | `apply_setter_plan` runs per document right after schemas are prepared; an empty plan costs nothing; no retry | 5 |
| 16 | Setters keep reaching transcluded children, bound per document (see below) | 5 |
| 17 | Provenance and caller origins cover only a setter's own path | 5 |
| 18 | `MarkdownError::Setter` is reported as `dm.setter.invalid` with `source: null` | 5 |
| 19 | One neutral `path_candidates` function serves completion; no baseline union over a valid effective schema | 6 |
| 20 | The CLI adapter alone reads process arguments; every failure means fewer candidates and no output | 7 |
| 21 | Custom `EnvCompleter`s only where the built-in shell scripts fail | 7 |
| 22 | Tests are tiered and hermetic; an absent shell is recorded as unproven, never as passing | 8 |

### Planning decision: setters and transcluded children (rule 16)

Setters keep reaching transcluded children, and each document binds a setter
through its own schema. The spec does not cover this case. A nested setter that
hits a scalar parent or an out-of-range index in a child fails the run and names
that child. This is listed in `decisions_for_review`.

### Correction to the spec

The spec's "recursive merge behavior of those older options" describes `--state`
only. In `prepare_frontmatter_for_compose` (`darkmatter/lib/src/markdown/compose/util.rs`),
`--state` goes through `deep_merge` (line 177), while `--set` overwrites each
top-level key with `fm.insert` (line 191). I confirmed this by reading the code
on 2026-11-06 and did not rerun the commands; the plan records the runtime
observation. Neither option changes. The spec is left as written.

### Baseline results (macOS, 2026-11-06)

`just test` in each area, before any code change:

| Area | Result | Exit |
|---|---|---|
| `biscuit-file` | 1187 passed, 0 skipped; then `test-minimal`: 6 passed, 67 filtered out by that recipe | 0 |
| `simplified-schema` | 1401 passed, 0 skipped; then a second run: 1213 passed, 0 skipped | 0 |
| `darkmatter` | 13979 passed, 10 skipped | 0 |

The 10 darkmatter skips are excluded by the recipe's selection, in both runs. I did
not enumerate them.

### Gates

The scope check does not allow skipping the gates. Its output for the phase's plan
and log edits was `packages: [darkmatter, darkmatter-cli]`, `change_class: package`:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

`just lint` and `just test` in `darkmatter/` were therefore run after the plan and
log edits. Lint exited 0. Tests exited 0 with 13979 passed and 10 skipped.

### Load-sensitive test

`darkmatter-cli::l1 compose_shell_template::every_suggested_rule_approves_the_command`
passed in all three runs but went over the 5 s L1 budget in one of them:

| Run | Duration |
|---|---|
| first full suite | 4.341 s |
| second full suite, host load about 37 | 5.522 s |
| alone, host load about 32 | 2.819 s |

No code changed in this phase, so this phase did not cause it. Other work on the
host, including Blender, accounted for part of the load. I left the test as it is.

### Test design and other checks

- **Requirement-to-test mapping:** none. This phase changes no behavior and adds no
  tests; later phases carry the mapping.
- **Input Robustness Matrix:** not applicable to this phase, since it adds no reader.
- **Cross-OS check:** not run; a Markdown-only change carries no OS risk.
- **Spec:** unchanged. This phase names no spec frontmatter to set.
- **Skill:** the `darkmatter` skill is unchanged; nothing it records has changed.

## Phase 2

Added the path type and its pure edits (rules 1 to 3) and the shared value parser
(rules 5 to 7). No command uses them yet, so `md` behaves as it did.

### What landed

| Item | Where |
|---|---|
| `PropertyPath`, `Resolved`, `ResolveStop`, `set_at`, `remove_at`, `RemovalPlan`, `PathEditError` | `simplified-schema/lib/src/property_path.rs` |
| `SetterValue`, `SetterValueError` | `simplified-schema/lib/src/setter_value.rs` |
| `numeric::admit_json5_text_numbers` | `simplified-schema/lib/src/numeric/ingress.rs` |
| `json5_scan`, `Json5Scan`, `json5_is_well_formed` | `biscuit-file/lib/src/json5/scan.rs` |

### Defects the tests found in my first version

- **Oversized integer bypassed admission.** `json_five` accepts the grammar of
  `{a:99999999999999999999}` but cannot build a value, so `Json5::from_str` fails.
  Treating that as a syntax error sent it to YAML, which read a key named
  `a:99999999999999999999`. `json5_is_well_formed` now separates the two cases.
- **`removal_order` was not a total order** (found on review, before running). Rust's
  sort may panic on an inconsistent comparator. It now orders every pair, and
  `removal_order_is_consistent_across_unrelated_targets` covers mixed targets.

### Behavior worth knowing

- In YAML flow, `{ a: NaN, max-width: 1 }` holds the string "NaN", because YAML spells
  it `.nan`. `{ a: 1e400, max-width: 1 }` holds the string "1e400", which is how
  `load_admitted_yaml` already reads frontmatter. Neither is rounded, and a declared
  `number` still rejects both. The spec's failure rule is therefore enforced for
  YAML's own non-finite spellings, not for these strings.
- `{ name:, b: 1 }` (no space after the colon) is a YAML syntax error in this parser,
  so it stays text. `{ name: , b: 1 }` and `{ b: 1, name: }` are null.

### Requirement-to-test mapping

All tests are L1, in process through each crate's public API (fidelity F1). No file,
terminal, or CLI boundary is crossed yet; the command-level wiring and shipped-artifact
tests belong to Phases 4, 5, and 8.

| Requirement | Tests |
|---|---|
| Rule 1: escapes, left-to-right pairs, literal backslash | `property_paths::dots_separate_and_the_two_escapes_decode`, `escape_pairs_match_left_to_right`, `a_backslash_before_anything_else_is_itself` |
| Rule 1: empty key, `a.`, `.a`, `a..b` | `the_whole_empty_argument_is_one_empty_key`, `an_empty_segment_in_a_longer_path_is_a_syntax_error`, `an_escaped_dot_does_not_empty_a_segment` |
| Rule 1: Unicode, whitespace, `$schema` | `unicode_and_whitespace_are_kept_unnormalized`, `a_special_first_character_is_an_ordinary_key` |
| Rule 1: round trip, canonical printing, decoded identity | `round_trip_holds_for_the_hard_spellings`, `parse_of_print_is_the_segments` (proptest), `the_argument_is_kept_as_written_and_printing_is_canonical`, `equality_and_hashing_compare_decoded_segments`, `from_segments_refuses_what_no_path_can_spell` |
| Rule 2: index versus key, `items.01`, 40-digit index | `a_number_is_an_index_on_an_array_and_a_key_on_a_mapping`, `only_a_canonical_decimal_is_an_index`, `a_huge_index_is_out_of_range_rather_than_wrapping` |
| Rule 2: scalar and null traversal, stop point, conversions | `a_present_null_or_scalar_cannot_be_traversed_but_can_be_found`, `a_walk_reports_how_far_it_got`, `the_pointer_comes_from_the_typed_walk_and_escapes_its_keys`, `a_value_root_and_a_map_root_walk_alike` |
| Rule 3: parents, `NumberUnderMissingParent`, no partial write | `set_creates_missing_mapping_parents`, `a_number_under_a_missing_parent_is_refused_even_at_the_end`, `a_refusal_deep_in_an_existing_parent_creates_nothing`, `a_digit_key_on_an_existing_mapping_is_a_key`, `a_parent_that_cannot_hold_the_key_names_its_type` |
| Rule 3: replacing, key order, arrays never append | `naming_a_null_or_scalar_replaces_it`, `replacing_keeps_the_key_position_and_new_keys_append`, `set_edits_one_existing_array_element_and_never_appends`, `a_literal_dotted_key_differs_from_nested_segments`, `the_empty_key_is_a_top_level_key` |
| Rule 3: removal, emptied parents, original indexes, conflicts | `removing_the_last_child_leaves_the_parent`, `surviving_mapping_keys_keep_their_order`, `array_removals_use_original_indexes_in_either_order`, `a_removal_inside_an_element_survives_removing_an_earlier_element`, `removal_order_is_consistent_across_unrelated_targets`, `a_target_that_does_not_exist_cancels_the_whole_removal`, `a_duplicate_or_an_ancestor_with_its_descendant_is_a_conflict`, `siblings_sharing_a_key_prefix_do_not_conflict`, `a_plan_exposes_typed_targets_in_argument_order` |
| Rule 5: forms, YAML fallback, `{ a:1, max-width: 2 }`, tags, block documents, neither | `setter_values::json5_and_yaml_flow_both_read_as_objects`, `json5_goes_first_so_a_mixed_value_reads_a_colon_key_through_yaml`, `a_quoted_string_decodes_once`, `null_is_null_and_its_lookalikes_are_not`, `a_scalar_keeps_its_spelling_until_bound`, `empty_input_is_the_empty_string`, `text_that_parses_as_neither_is_text`, `a_template_is_never_read_as_a_mapping`, `a_tagged_yaml_value_is_text_and_runs_nothing`, `an_exclamation_mark_inside_a_scalar_is_no_tag`, `an_anchor_or_alias_is_text`, `a_block_or_second_document_is_text`, `what_yaml_flow_syntax_allows_is_accepted`, `both_loaders_preserve_mapping_order` |
| Rule 6: duplicates, non-finite, inexact, underflow in both loaders | `a_duplicate_key_fails_in_both_loaders`, `an_inadmissible_number_in_a_container_fails_in_both_loaders`, `a_shadowed_number_is_still_judged`, `spellings_yaml_reads_as_strings_are_never_rounded`, `a_bare_inadmissible_number_stays_text`, `json5_number_admission_reports_where`, `json5_number_admission_accepts_every_admissible_spelling`; token walk in `biscuit_file json5::scan::tests` (11 tests) |
| Rule 7: `string`, `string \| number`, `numberlike`, `boolish`, failure keeps raw, no declaration | `text_goes_through_the_declared_type`, `a_value_the_type_rejects_stays_as_written_for_validation_to_report`, `with_no_declaration_text_is_guessed_as_today`, `a_type_declaring_no_structure_guesses_too`, `a_quoted_string_is_not_converted_by_binding`, `null_binds_to_null_whatever_the_type`, `a_number_in_a_container_has_already_lost_its_spelling` |

### Input Robustness Matrix

Default outcome: an inadmissible parsed container is a typed `SetterValueError`.
Exceptions, each tested: rejected syntax is text; empty input is `""`; `null` is kept
for the schema to judge; scalar spelling is kept until bound; a value that fails binding
is returned unconverted for validation to report.

| Shape, inside a container (both loaders) | Outcome | Test |
|---|---|---|
| control: valid text-to-number conversion | converts | `control_a_conversion_inside_a_container_converts` |
| absent field | stays absent | `an_absent_field_stays_absent` |
| explicit null, including YAML `k: ` | null, not absent | `an_explicit_null_field_stays_null_and_is_not_absent` |
| wrong type, whole field | kept for validation | `a_wrong_typed_field_keeps_the_whole_value_for_validation` |
| wrong type, one element | kept, not filtered | `one_invalid_element_is_kept_not_filtered` |
| wrong type, every element | kept, not emptied | `every_invalid_element_is_kept_not_emptied` |
| empty `""`, `[]`, `{}` | empty, not absent | `an_empty_field_is_empty_not_absent` |
| duplicate key | error | `a_duplicate_key_fails_in_both_loaders` |
| trailing or invalid content | text | `a_block_or_second_document_is_text`, `text_that_parses_as_neither_is_text` |
| format syntax: JSON5 comments and trailing commas; YAML comment, `k: `, second document | as the format defines | `json5_comments_and_trailing_commas_are_accepted`, `what_yaml_flow_syntax_allows_is_accepted`, `a_block_or_second_document_is_text` |

Code-smell grep over `setter_value.rs`, `property_path.rs`, and `json5/scan.rs`: no
`unwrap_or_default`, no `filter_map`, and three `.ok()`, all intentional:

- `setter_value.rs`, `serde_json::to_value` of an already admitted YAML number; a
  failure leaves the value as text.
- `property_path.rs`, `segment.parse().ok()`; an index too large for `usize` is out of
  range by contract.
- `json5/scan.rs`, the second parse of a key with an escape; on failure the key compares
  by its written text.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping the gates. It reported
`packages: [biscuit-file, simplified-schema, test-toolkit, darkmatter, darkmatter-cli]`,
`change_class: package`.

| Area | `just test` | `just lint` |
|---|---|---|
| `simplified-schema` | exit 0: 1479 passed, 0 skipped; second run 1291 passed, 0 skipped | exit 0 |
| `biscuit-file` | exit 0: 1198 passed, 0 skipped; `test-minimal` 6 passed, 67 filtered out by that recipe | exit 0 |
| `darkmatter` | exit 0: 13979 passed, 10 skipped by the recipe's selection, as at baseline | exit 0 |

The `simplified-schema` and `biscuit-file` gates ran before the one-line test fix below,
which is in neither package. The `darkmatter` gates ran after it, on the final source.

The 8 new doctests passed through `cargo test --doc`, filtered to the new symbols,
because `just test` does not run doctests.

### Dependent packages

This phase only adds symbols, so no existing test in a dependent exercises the change.
That made a compile check the right evidence:
`cargo check --all-targets` exited 0 with no warnings for `claudine`, `claudine-cli`,
`claudine-gen`, `messenger`, `messenger-cli`, `biscuit-file-cli`, `biscuit-terminal`,
`content-policy`, `model-citizen`, `playa-cli`, `research`, `schematic-define`,
`schematic-gen`, `sniff`, `sniff-detection-cli`, `tree-hugger`, `tree-hugger-cli`, and
`worktree`. Their test suites were not run.

`test-toolkit`, which the scope check selected, had 1 failure:
`archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path`. It was not
caused by this phase. It flagged an `env!("CARGO_MANIFEST_DIR")` at
`darkmatter/cli/tests/l1/output_commands.rs:687`, a file this branch had not touched.
It reproduced when run alone. I replaced it with `biscuit_test_harness::manifest_dir!()`.
After the fix, 479 passed and 2 skipped, and darkmatter's
`every_leaf_has_a_schema_file_and_a_conformance_test` passes.

### Load-sensitive tests

Five existing tests passed but went over the 5 s L1 budget in the final darkmatter run,
with host load around 15 to 25. None of them calls code this phase added. Each is well
inside the budget alone, so I left them as they are.

| Test | Full suite | Alone |
|---|---|---|
| `compose_remote_caching::test_compose_denied_host_never_reads_a_seeded_cache_entry` | 5.902 s | 2.369 s |
| `compose_report_diagnostics::other_families_report_their_positions_and_no_reference_details` | 5.955 s | 2.301 s |
| `compose_shell_template::every_suggested_rule_approves_the_command` | 6.486 s | 2.824 s |
| `schema_triggers::dialect_family_has_identical_compose_validate_and_trace_activation` | 5.108 s | 2.057 s |
| `claudine_editor_parity::the_shared_corpus_valid_generated_verb_fixtures_raise_no_schema_diagnostic` | 5.158 s | 2.114 s |

### Cross-OS check

Not run. This phase adds string and in-memory JSON code, with no file paths, processes,
or OS-specific crates. A `\` in a path is property-key syntax here, not a separator.
Other operating systems are unexecuted and run under scheduled CI.

### Skills and spec

No skill or `docs/` page changed in this phase, since no command behaves differently
yet; Phase 9 owns both. This phase names no spec frontmatter to set.

## Phase 3

Added the nested text editor (rules 8 and 9). No command calls it yet, so `md` behaves
as it did; Phase 4 connects it.

### What landed

| Item | Where |
|---|---|
| `PathEdit`, `PathEditRefusal`, `edit_frontmatter_paths`, `edited_document_text` | `darkmatter/lib/src/markdown/hash/path_edit.rs` |
| Twelve helpers made `pub(super)`, with no behavior change | `darkmatter/lib/src/markdown/hash/write.rs` |

The editor reads node positions from `locate_frontmatter_value`, one top-level property
at a time. After each edit it reloads the text through `Markdown::try_from_content`, the
loader that produced `expected`, so lenient forms such as a bare `{{ name }}` compare
correctly. Whole-value comparison is the safety net: a wrong position becomes a refusal,
never a bad write.

### Defects the tests found in my first version

- **Fallback damaged the body.** `Markdown::as_string` dropped the final line break and
  normalized CRLF. The fallback now keeps the source's fences and body bytes.
- **A comment became content.** Growing `name: Alice # c` to a block scalar put `# c` on
  the scalar's last line. It now goes on the first line.
- **A new key cut into a `|+` scalar.** The located span stops before the blank lines that
  scalar owns. The append point now moves past them.
- **Unreachable branch.** Removing the inner item of `- - x` could never run, because the
  locator cannot read that form. I deleted it and pinned the refusal.

### Behavior worth knowing

- An indented `  ---` inside a block scalar closes the frontmatter for every reader,
  because the fence scan compares trimmed lines. This is existing behavior.
- A flow bracket closing at column 0 and a compact `- - x` are unsupported forms, so they
  fall back and lose comments.

### Requirement-to-test mapping

All tests are L1 in `darkmatter/lib/tests/l1/frontmatter_path_edit.rs` (50 tests,
`frontmatter_path_edit::`), in process through the crate's public functions, with
`expected` made by the real `set_at` and `RemovalPlan` (fidelity F1). The boundary is the
library's text-to-text contract. No file or CLI boundary is crossed yet, so persistence,
exit status, and the no-write-on-refusal behavior belong to Phase 4's CLI tests.

| Requirement | Tests |
|---|---|
| Comments inside a shared parent; sibling scalar spelling | `a_nested_edit_keeps_comments_and_spelling_in_the_shared_parent`, `a_replaced_scalar_keeps_the_comment_after_it`, control `control_an_unedited_document_is_returned_unchanged` |
| Appending to a block mapping at its indent; created parents | `a_new_key_appends_at_its_parents_indent`, `missing_parents_are_created_inside_the_nearest_block_mapping`, `a_new_top_level_key_goes_at_the_end_of_the_frontmatter`, `an_empty_block_or_absent_frontmatter_gains_the_key`, `an_empty_flow_mapping_takes_a_new_key_as_a_block` |
| Replacing in a block sequence | `an_item_in_a_block_sequence_is_replaced_in_place`, `a_key_inside_a_sequence_item_is_replaced_and_appended_at_the_items_indent`, `a_whole_item_can_become_a_mapping_or_a_scalar`, `an_item_with_no_value_or_on_its_own_line_can_be_replaced`, `a_sequence_nested_in_a_sequence_is_edited_on_its_own_lines_only` |
| Replacing across shapes and spellings | `a_scalar_becomes_a_container_and_keeps_its_comment`, `a_container_becomes_a_scalar_and_keeps_the_key_lines_comment`, `a_replaced_container_keeps_its_childrens_column`, `an_empty_value_is_filled_in_after_its_colon`, `multi_line_scalars_are_replaced_whole`, `a_multi_line_value_is_written_inside_its_parent`, `a_value_that_would_read_as_another_type_stays_a_string`, `a_comment_stays_on_the_first_line_of_a_value_that_grows`, `a_flow_collection_named_whole_is_replaced_in_place`, `replacing_an_alias_itself_changes_only_that_value` |
| Removal, including the last child and original indexes | `removing_a_node_takes_its_own_lines_and_leaves_its_neighbors`, `removing_the_last_child_leaves_an_empty_parent`, `removing_the_only_top_level_key_leaves_an_empty_block`, `removals_use_original_indexes_in_either_order`, `removing_a_key_that_shares_a_line_with_a_dash_keeps_the_dash`, `an_item_whose_dash_is_on_its_own_line_is_removed_with_it` |
| LF and CRLF (LF throughout) | `a_crlf_document_keeps_its_line_endings`, `a_crlf_document_with_no_frontmatter_gets_a_crlf_block`, `the_fallback_keeps_the_fences_and_body_bytes_of_a_crlf_document`, `a_lone_cr_document_is_edited_exactly_or_refused` |
| Flow collection falling back | `walking_into_a_flow_collection_is_refused_and_falls_back` |
| Anchor whose edit changes an alias: refused, no fallback | `an_edit_an_alias_reads_is_refused_with_no_fallback`, `removing_an_anchor_an_alias_needs_is_refused_with_no_fallback`, `an_anchor_redeclared_earlier_cannot_silently_redirect_an_alias`, `an_expected_value_the_text_does_not_hold_is_refused`; negative control `an_anchored_node_with_no_alias_reading_the_change_can_be_edited` |
| Other refused forms | `an_indented_root_is_refused_and_serialization_writes_it`, `walking_through_an_alias_or_tag_is_refused_and_serialization_keeps_the_values`, `frontmatter_only_the_lenient_loader_reads_falls_back` |
| Source forms that could mislead the locator | `a_bare_template_beside_the_edit_is_kept`, `keys_that_need_quoting_are_found_and_written_as_keys`, `unusual_key_spellings_are_matched_by_their_decoded_text`, `a_merge_key_is_an_ordinary_key_beside_the_edit`, `text_that_only_looks_like_structure_is_not_mistaken_for_it`, `a_scalar_keeping_its_trailing_blank_lines_is_not_cut_short` |
| Read/write/read round trip | `only_the_named_node_changes_across_a_read_write_read_round_trip`, `a_document_built_in_memory_is_serialized` |

### Input Robustness Matrix

Not applicable: this phase adds a writer, not a reader of a file format. It does reread its
own output, through the existing document loader. Code-smell grep over `path_edit.rs`: no
`filter_map` or `#[serde(default)]`. Two intentional exceptions:

- `split_whitespace().next().unwrap_or_default()` in `keeps_trailing_lines`: empty text has no
  block header.
- `item_dash(...).ok()` in `first_child_column`: with no dash, the replacement uses the
  default indent, and verification still judges the result.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping the gates. It reported
`packages: [biscuit-file, darkmatter, darkmatter-cli, simplified-schema, test-toolkit]`,
`change_class: package`. The uncommitted tree still holds Phase 2's files, which is why
`biscuit-file`, `simplified-schema`, and `test-toolkit` appear; this phase changed none of
them, so their gates were not rerun.

| Gate | Result |
|---|---|
| `just lint` in `darkmatter/` | exit 0 |
| `just test` in `darkmatter/` | exit 0: 14029 passed, 10 skipped by the recipe's selection; 13979 at baseline plus these 50 |
| `just check-tier-coverage darkmatter` (from the root) | exit 0, 0 stranded |

`just lint` and `just test` ran before the plan, log, and skill text were finished, and no
source file changed after them.

### Dependent packages

This phase only adds symbols and narrows no visibility, so a compile check was the right
evidence: `cargo check --all-targets` exited 0 with no warnings for `biscuit-icon-cli`,
`biscuit-speaks-cli`, `biscuit-terminal-cli`, `claudine`, `claudine-cli`, `claudine-gen`,
`messenger`, `messenger-cli`, `playa-cli`, `research`, and `sniff-detection-cli`. Their test
suites were not run. `darkmatter-cli` and `darkmatter-lsp` ran in full under `just test`.

### Load-sensitive tests

Four existing tests passed but went over the 5 s L1 budget in the full run. None of them
calls the new code, and I left them as they are.

| Test | Full suite | Alone |
|---|---|---|
| `compose_report_diagnostics::other_families_report_their_positions_and_no_reference_details` | 5.035 s | 2.291 s |
| `compose_shell_template::every_suggested_rule_approves_the_command` | 7.607 s | 2.830 s |
| `claudine_editor_parity::the_shared_corpus_valid_depth_fixtures_raise_no_schema_diagnostic` | 5.456 s | 2.381 s |
| `content_policy_baseline::every_shared_declaration_publishes_the_baseline_verdict_at_its_anchor` | 5.331 s | 2.161 s |

### Cross-OS check

Not run. The code is string editing with no file paths, processes, or OS-specific crates, and
CRLF and lone-CR text are covered by tests on every OS. Other operating systems are unexecuted
and run under scheduled CI.

### Skills, docs, and spec

Updated `.claude/skills/darkmatter/frontmatter.md`, because it told agents not to add a YAML
writer and now has to name this one. No `docs/` page changed, since no command behaves
differently yet; Phase 9 owns those. This phase names no spec frontmatter to set.
