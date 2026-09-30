---
spec: /Volumes/coding/wt/rusty-biscuit/fix-hash-writer/darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/spec.md
plan: darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/plan.md
implemented_by: claude/opus
started_phase: 1
packages:
    - darkmatter
    - darkmatter-cli
    - claudine
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/cli/tests/l1/hash_kind_save_diff.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/cli/tests/l1/hash_kind_save_diff.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3:
    - darkmatter/docs/cli/hash.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/frontmatter.md
source_code:
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/cli/tests/l1/hash_kind_save_diff.rs
documentation:
    - darkmatter/docs/cli/hash.md
    - claudine/docs/topics/composition.md
    - .claude/skills/darkmatter/frontmatter.md
completed_phase: 3
implemented: true
---

# Implementation Log for 2026-09-28-hash-writer-byte-fidelity (3 phases)

## Phase 1

Started and finished 2026-09-29 on macOS. Phase 1 is tests only; no writer
behavior changed.

### Spike: lone CR and BOM

A scratch `#[test]` (deleted after the run) showed:

- Case 4's lone-CR input extracts (`yaml_span` 4..69), passes
  `validate_block_mapping`, parses with `serde_yaml_ng`, and yields correct
  `TextNode` ranges from `parse_text_frontmatter`. **The lone-CR path is
  viable**, so no ruling is needed.
- `yaml.lines()` treats CR-only YAML as a single line. The flow-root
  pre-check still catches a flow root that comes first. It misses one that
  follows a comment line (`# c\r{a: 1}\r`). That gap already existed, is
  outside this fix, and case 4 does not depend on it.
- A BOM before existing frontmatter extracts correctly, and case 5's expected
  output re-parses. Empty frontmatter (`---\n---\n`) gives an empty
  `yaml_span`. A body with no terminator extracts normally.

### Red-test gating decision

Phase 1's red tests would make `just test` fail, but the phase is only done
when `just test` passes. To satisfy both, I followed the existing repo
convention (the schema-plus scaffolding in
`lib/src/markdown/schemas/simplified/grammar.rs`): each red test carries
`#[ignore = "red until phase 2 of 2026-09-28-hash-writer-byte-fidelity: <slice>"]`.
Phase 2 removes each gate when it lands that slice. New tests that already
pass stay ungated as regression guards.

### Tests added (`darkmatter/lib/src/markdown/hash/write.rs`, `mod tests`, L1)

Helpers:

- `assert_fidelity(input, output, bumped)` (R8). Re-parses both documents with
  `parse_text_frontmatter` and requires the records to be equal once `hash`
  (and `last_updated` when bumped) are removed. It also asserts that `hash` is
  present, that `last_updated == TODAY` when bumped, that body bytes are
  identical (`body_span`, or the whole text minus a BOM when there is no
  block), and that a leading BOM stays first.
- `assert_refused(input, decision)`. Expects `Err(FrontmatterTextEdit)`,
  asserts the caller's `String` equals a clone taken before the call, and
  returns the reason.
- `save_canonical` / `saved_canonical` / `assert_refused_canonical` use the
  canonical `aaaa000000000000-bbbb000000000000` simple hash, whose bytes do
  not change. `saved_body_kind` uses a `Body`-kind hash that serializes to
  three lines, which exercises R2.

| Requirement | Test | State now |
| --- | --- | --- |
| Case 1 (LF) | `textual_save_case1_empty_date_gets_one_space` | red: `last_updated:2026-09-28`, does not parse |
| Case 1 (CRLF) | `textual_save_case1_empty_date_gets_one_space_crlf` | red: same |
| Case 2 | `textual_save_case2_empty_date_keeps_comment_spacing` | red: value parses as `"2026-09-28# todo"` |
| Case 3 | `textual_save_case3_lf_date_line_in_crlf_file_keeps_lf` | red: `\n\r\n` |
| Case 4 | `textual_save_case4_lone_cr_terminators_survive` | red: hash line LF, date line CRLF |
| Case 5 | `textual_save_case5_bom_stays_before_new_block` | red: BOM lands in the body |
| Case 6 | `textual_save_case6_refuses_anchored_date` | red: writes an orphaned `*lu` |
| Alias (source elsewhere), tag, bare anchor | `textual_save_refuses_aliased_tagged_and_bare_anchored_dates` | red: date replaces the property |
| No bump skips the property check (spec rule) | `textual_save_without_bump_ignores_date_node_properties` | green |
| Mixed file, inserted date (LF-in-CRLF and CRLF-in-LF), empty frontmatter uses the opening delimiter's terminator (R3) | `textual_save_inserted_date_inherits_preceding_terminator_in_mixed_file` | red: global newline |
| Mixed file, hash-only, multi-line node with the same, longer, and shorter line counts (R2) | `textual_save_hash_only_multi_line_node_keeps_each_line_terminator` | red: global newline |
| Lone-CR hash-only replacement (single and multi-line) | `textual_save_lone_cr_hash_only_replacement` | red: LF written |
| BOM before existing frontmatter | `textual_save_keeps_bom_before_existing_frontmatter` | green |
| Body with no final terminator, with and without frontmatter (new block uses LF) | `textual_save_body_without_final_terminator_stays_unterminated` | green |
| New block uses the body's first terminator (lone CR; LF-then-CRLF) | `textual_save_new_block_uses_first_body_terminator` | red: global newline |
| R7: hash replacement orphans an alias | `textual_save_refuses_hash_replacement_that_orphans_an_alias` | red: invalid output returned |
| Robustness matrix (25 rows, control first) | `textual_save_last_updated_robustness_matrix` | red (see below) |

The robustness matrix is built from one control fixture (`CONTROL`), and each
row makes exactly one `replacen` edit. A guard asserts that every non-control
edit actually changed the fixture. The date column covers: control, absent,
empty, empty with comment, empty with a single-space comment, `~`, `null`
with extra leading whitespace, `""`, `''`, block collection, flow sequence,
flow mapping, anchor, alias (`*r`, source `reviewed: &r`), tag, bare anchor,
duplicate key, and invalid YAML. The hash column covers: absent, empty, block
collection, flow collection, alias, duplicate key, and an edit that orphans
an alias. Every written row also runs `assert_fidelity`.

I instrumented a temporary run with `catch_unwind` per row (reverted
afterward). Exactly these rows fail today: `empty`, `empty with comment`,
`empty with single-space comment`, `flow sequence`, `flow mapping`, `anchor`,
`alias`, `tag`, `bare anchor`, and `hash edit orphans an alias`. All other
rows, including the control, pass.

"Edit produces invalid YAML" in the date column has no reachable input once
the refusals are in place. The validation row is exercised through the hash
column (R7), as the plan's matrix allows.

### Test added (`darkmatter/cli/tests/l1/hash_kind_save_diff.rs`, L1)

- `test_hash_save_refuses_anchored_last_updated_without_writing` launches
  through `CliProcessFixture`. It uses a stale stored hash (which forces a
  date bump) with `last_updated: &lu 2026-01-01` and `reviewed: *lu`, and
  asserts a non-zero exit, stderr containing `last_updated`, and byte-identical
  file contents. It is red today ("Unexpected success"; the CLI writes the
  broken file). I confirmed that writer refusal reasons reach stderr, using
  the existing flow-root refusal.

### Tier and placement

The library tests are in `lib/src/markdown/hash/write.rs` `mod tests`. The CLI
test is in the already-declared `cli/tests/l1/hash_kind_save_diff.rs`. No
test-path segment carries a tier marker, so all of them are L1. No features
are needed.

### Gates

- `cargo nextest run -p darkmatter --lib --run-ignored all -E 'test(textual_save)'`:
  25 run, 11 pass, 14 fail (exactly the gated tests, each for the expected
  reason).
- `cargo nextest run -p darkmatter-cli --run-ignored all -E ...`: the new CLI
  test fails with "Unexpected success", and the flow-mapping sibling passes.
- `just test` (darkmatter, `--no-fail-fast`): 8654 run, 8653 pass, 27
  skipped, 1 failed. The failure,
  `markdown::schemas::file_match::tests::conversion_emits_every_root_union_glob`,
  is **pre-existing and unrelated**. It is a pure schema-conversion
  assertion, and this phase's library diff adds only `#[cfg(test)]` code to
  `write.rs`. I established this by inspection; I did not re-run it at a
  clean HEAD.
- `just lint` (darkmatter): exit 0, no warnings.

### OS considerations

The new tests are pure in-memory string transforms plus one CLI temp-file
round trip that uses the existing fixture. There is no platform-specific path
or process behavior, so I did not run a cross-OS pass in Phase 1. CI and
Phase 2 will cover this.

### Skill

No Darkmatter skill change was needed for Phase 1. Phase 3 owns the
`frontmatter.md` update.

## Phase 2

Started and finished 2026-09-29 on macOS. All changes are in
`darkmatter/lib/src/markdown/hash/write.rs`, plus removing one `#[ignore]`
line from `darkmatter/cli/tests/l1/hash_kind_save_diff.rs`.

### What changed

- **Terminator helpers.** `line_terminators(text)` lists every line's
  terminator (`"\r\n"`, `"\n"`, `"\r"`, or `""`), built on `line_spans`.
  `apply_terminators(serialized_lf, terminators)` implements R2: line *i*
  takes `terminators[i]`, and a line past the end (or whose original had no
  terminator) repeats the previous line's. `preceding_terminator(document,
  insert_at)` implements R3 from the bytes just before the insertion point.
  For empty frontmatter, those bytes are the opening delimiter's terminator.
- **Per-line `hash` node.** `serialize_entry` and `serialize_existing_entry`
  now return LF output and take no newline. An existing node is re-terminated
  with its own `line_terminators`, and an inserted node with
  `&[preceding_terminator(..)]`, so one function covers both R2 and R3.
- **`rewrite_date_scalar`.** It takes the node's own terminator from
  `line_spans` instead of stripping a global newline. An empty value (R4)
  becomes `key: {today}`, followed by the authored whitespace and comment
  when there is one (a single space is supplied if that run is empty). A
  value that starts with `[` or `{` is refused as "must be a scalar value"
  (R6). A nonempty value keeps the authored leading whitespace, quote style,
  and comment, as before.
- **Node-property refusal (R5).** The shared predicate is
  `leading_node_property(bytes) -> Option<&'static str>`, returning
  `"anchor"`, `"alias"`, or `"tag"`. It returns the kind, not a `bool`, so
  the refusal can name the property. **This departs from the plan's name**
  (`starts_with_node_property`). `locate_frontmatter_leaves` calls it with
  `.is_some()`, which is behavior-neutral, and all of its tests pass.
  `rewrite_date_scalar` calls it before anything is emitted and returns
  "`last_updated` uses a YAML {kind}; replacing it would change other
  values". The check runs only when the date is bumped, because
  `rewrite_date_scalar` is reached only on that path.
- **New block.** `new_frontmatter_block(document, yaml)` strips a leading
  BOM and emits BOM, block, rest. `new_block_terminator(document)` returns
  the first line's terminator, or LF. Both are private and shaped for the R1
  restore follow-up.
- **Output validation.** `validated(document)` runs `parse_text_frontmatter`
  and maps any failure to "rewritten frontmatter did not parse: ...". Both
  the new-block branch and the edit branch return through it. The
  `new_stored == None` early return still comes first and parses nothing.
- **Docs.** I rewrote the `apply_hash_save_text` `///` block: the per-line
  rule, the inserted-property rule, the BOM and new-block rule, the empty
  date, post-edit parsing, and an extended `## Errors`. The module `//!` doc
  ("preserve every byte outside the managed hash and `last_updated` nodes")
  and the `Markdown::apply_hash_save` doc are still accurate, and I found no
  drift. `detect_newline` now has one caller, `insert_snapshot_node` (the
  restore path, R1).
- **Gates removed.** I deleted all 14 library `#[ignore = "red until phase
  2 ..."]` attributes, the CLI one, and the test-module comment that
  described the gating.

### Tests added in Phase 2 (L1, `write.rs` `mod tests`)

- `textual_save_repeated_save_is_byte_stable`: a read/write/read round trip.
  It saves each spec-case input (cases 1, 2, 3, 4, and 5, plus the mixed
  multi-line `hash` node) twice and asserts the second output is
  byte-identical to the first and passes `assert_fidelity`.
- A new robustness matrix row, `empty with trailing whitespace`
  (`last_updated:   `), which becomes `last_updated: 2026-09-28`. The matrix
  now has 26 rows.

### Requirement-to-test mapping (all green now)

| Requirement | Test |
| --- | --- |
| Case 1 (LF, CRLF) | `textual_save_case1_empty_date_gets_one_space`, `..._crlf` |
| Case 2 | `textual_save_case2_empty_date_keeps_comment_spacing` |
| Case 3 | `textual_save_case3_lf_date_line_in_crlf_file_keeps_lf` |
| Case 4 | `textual_save_case4_lone_cr_terminators_survive` |
| Case 5 | `textual_save_case5_bom_stays_before_new_block` |
| Case 6, alias, tag, bare anchor | `textual_save_case6_refuses_anchored_date`, `textual_save_refuses_aliased_tagged_and_bare_anchored_dates` |
| Case 6 end to end (CLI exits non-zero, file bytes unchanged) | `hash_kind_save_diff::test_hash_save_refuses_anchored_last_updated_without_writing` |
| R2 | `textual_save_hash_only_multi_line_node_keeps_each_line_terminator`, `textual_save_lone_cr_hash_only_replacement` |
| R3 | `textual_save_inserted_date_inherits_preceding_terminator_in_mixed_file` |
| New-block terminator, no final terminator | `textual_save_new_block_uses_first_body_terminator`, `textual_save_body_without_final_terminator_stays_unterminated` |
| R6, R7, and the whole `last_updated` / `hash` matrix | `textual_save_last_updated_robustness_matrix` |
| Output validation (R7) | `textual_save_refuses_hash_replacement_that_orphans_an_alias` |
| Round trip | `textual_save_repeated_save_is_byte_stable` |
| `locate_frontmatter_leaves` refactor is neutral | `node_properties_and_unmodeled_shapes_fail_closed` and siblings |

### Input-robustness smell check

In the `write.rs` date path, there is no `#[serde(default)]`,
`unwrap_or_default()`, or `.ok()` on `last_updated`. The one
`unwrap_or_default()` in `serialize_existing_entry` reads serde's own output,
not user input.

### Gates

- `cargo nextest run -p darkmatter --lib -E 'test(textual_save)'`: 26 of 26
  pass.
- `cargo nextest run -p darkmatter-cli -E 'binary(l1) & test(/hash_save/)'`:
  10 of 10 pass, including the un-gated refusal test.
- `just test --no-fail-fast` (darkmatter): 8669 run, 8668 pass, 12 skipped,
  1 failed. Skipped fell from 27 to 12, exactly the 15 un-gated tests. The
  failure is the same pre-existing, unrelated
  `markdown::schemas::file_match::tests::conversion_emits_every_root_union_glob`
  that Phase 1 recorded. It is an in-memory schema-conversion assertion
  (`left: Null`, `right: ["*.md"]` at `file_match.rs:329`), and no file it
  exercises was touched.
- `just lint` (darkmatter): exit 0, no warnings. I ran it after the tests,
  not at the same time.
- `grep -n detect_newline write.rs` shows only the definition and the
  restore caller.

### OS considerations

Every change is a pure in-memory byte and string transform. Terminators are
handled explicitly as bytes (`\r\n`, `\n`, `\r`) and never through
platform line APIs, and nothing touches paths or processes. The CLI test
reuses the existing `CliProcessFixture` temp-file pattern. I judged the OS
risk too low to justify `just cross-check`; CI's Linux and macOS legs cover
it.

### Skill

No Darkmatter skill change was needed in Phase 2. Phase 3 owns the
`frontmatter.md` update.

## Phase 3

Started and finished 2026-09-29 on macOS. Documentation, skill drift, and
downstream verification only; no source file changed.

### What changed

- **`darkmatter/docs/cli/hash.md`.** I replaced the `--save` paragraph that
  said the CLI "persists the canonical frontmatter (via the same serializer as
  `md clean --save`)". It now describes the in-place writer: only the hash
  property and a bumped `last_updated` change, and every other byte is kept,
  including a BOM and each line's own terminator. It lists the per-line
  terminator rules and gives a before/after example for an empty
  `last_updated:` with a comment. A refusal list covers anchor, alias, or tag,
  and a block or flow collection, with exit `1` and the file left unchanged.
  It also covers the post-edit parse failure. The page does not mention this
  fix.
- **`.claude/skills/darkmatter/frontmatter.md`.** I added a note that
  `restore_properties_text` still uses one newline for the whole file and
  prepends a new block before a BOM (R1). I also added a "Text-Preserving Hash
  Save" section with the per-line rules, the empty-date rule, the refusals, the
  shared `leading_node_property` predicate, and output validation.
- **`claudine/docs/topics/composition.md`. This goes beyond the plan's file
  list.** Claudine's closure maps every writer error to
  `CompositionError::InlineHashMalformed` before it writes (`closure.rs:181`).
  So an anchored, aliased, tagged, or collection `last_updated` now fails the
  closure without writing. The topic page's `hash` property section did not
  say so, and the repo's drift rule treats omitted behavior as a defect. I
  added one bullet, "Unwritable `last_updated`". No Claudine code changed.

### Downstream verification

- **Claudine fixtures.** `grep -rnE "last_updated:[[:space:]]*[&*!\[{]"
  claudine` finds no fixture or test with a node property or collection on
  `last_updated`. The only code hit is `closure/tests.rs:237`, where `{quote}`
  is a `format!` placeholder for a quote character. No Claudine test depends
  on the old anchored behavior.
- **`just test claudine`** (repo root): 8316 run, 8315 pass, 11 skipped, 1
  failed. The failure is
  `claudine-cli::l1 compose_schema_cli::compose_enforces_each_root_union_arm_match_before_provider_launch`
  ("/spec: no existing file matched reference `features/x/spec.md`"). That
  is schema root-union file matching, the same area as the known Darkmatter
  `file_match` failure. **It already failed before this fix:** I ran the same
  test in a throwaway detached worktree at `dadebc029`, the commit before
  Phase 1, and it failed the same way. I then removed that worktree. All
  Claudine closure tests pass.
- **Darkmatter CLI.** The plan's filter `binary(hash_kind_save_diff) |
  binary(hash)` matches nothing now, because the CLI tests are consolidated
  into the `l1` binary. I used `cargo nextest run -p darkmatter-cli -E
  'binary(l1) & test(/hash/)'`: 49 of 49 pass. The library's
  `test(/hash::write/)` also passes, 50 of 50.

### Gates

- `just test --no-fail-fast` (darkmatter): 8670 run, 8669 pass, 12 skipped,
  1 failed. The failure is the same
  `markdown::schemas::file_match::tests::conversion_emits_every_root_union_glob`
  that Phases 1 and 2 recorded, and it is unrelated to this fix.
- `just lint` (darkmatter): exit 0. I ran it after the tests, not at the same
  time.
- `git diff --stat` for Phase 3 shows `darkmatter/docs/cli/hash.md`,
  `.claude/skills/darkmatter/frontmatter.md`,
  `claudine/docs/topics/composition.md`, and this plan and log. Only the
  Claudine doc is outside the plan's list, as explained above. I did not run
  `cargo fmt` or commit.

### OS considerations

Phase 3 changed only documentation, so it carries no OS risk. I did not run
`just cross-check`.

### Status

Implementation complete, ready for review. Moving the fix to `_completed` is
the author's step.
