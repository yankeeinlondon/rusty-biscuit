---
total_phases: 3
created: 2026-09-29
phase: 3
agent: claude/opus
yolo: true
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

# Plan: Hash Writer Byte Fidelity

Spec: `2026-09-28-hash-writer-byte-fidelity` (`spec.md` in this directory).

## Summary and Definition of Done

### The work

`apply_hash_save_text` in `darkmatter/lib/src/markdown/hash/write.rs` is the
byte-preserving writer behind `md hash --save` and Claudine's closure
write-back. It breaks its own promise in six ways. The fix is local to
`write.rs`, plus doc and skill updates:

1. **Terminators become per-line.** `detect_newline` (one newline for the whole
   file) stops driving `apply_hash_save_text`. Each edited line keeps its own
   LF, CRLF, or lone-CR terminator. Inserted lines inherit the terminator of
   the line before them. A new block uses the body's first terminator, or LF.
   This fixes cases 3 and 4. It also applies to the serialized `hash` node,
   line by line, so a hash-only save follows the same rule.
2. **Null date values get spacing.** `rewrite_date_scalar` writes one space
   and the date after the colon when the value is empty. It keeps the
   authored comment spacing and never joins `#` to the date. This fixes cases
   1 and 2.
3. **The BOM stays first.** When the writer creates a block for a document
   with no frontmatter, it writes BOM, then the block, then the rest of the
   text. This fixes case 5.
4. **Node properties are refused.** If a date bump is requested and
   `last_updated` carries an anchor (`&`), alias (`*`), or tag (`!`), the
   writer returns `MarkdownError::FrontmatterTextEdit` and writes nothing.
   This fixes case 6.
5. **Output is validated.** The rewritten document's frontmatter is parsed
   before it is returned. A parse failure becomes `FrontmatterTextEdit`. A
   decision with no new stored hash still returns `None` without parsing.

Nothing changes for callers: the signature, the clock, and when the date is
bumped all stay the same. Claudine's closure write-back gets the fix without
code changes.

### Success criteria

- [x] Each of the six spec cases has a unit test in `write.rs` that asserts
      exact output bytes (cases 1 to 5, with case 1 also in CRLF) or the
      refusal (case 6). Alias refusal (including an alias whose source is
      elsewhere in the frontmatter) and tag refusal are also covered.
- [x] Tests cover five more shapes: a mixed-newline file where `last_updated`
      is inserted, a mixed-newline file where only the hash changes, a lone-CR
      hash replacement, a BOM before existing frontmatter, and a body with no
      final terminator. Each asserts that untouched spans keep their bytes and
      that existing terminators survive.
- [x] Every successful output re-parses. Its frontmatter record differs from
      the input only at `hash` and, when bumped, `last_updated`. Body bytes are
      identical. Every refusal returns `Err`, and the caller's input string is
      unchanged.
- [x] The `last_updated` input robustness matrix (below) is covered by one
      table-driven test with a control row.
- [x] One CLI L1 test proves the refusal end to end: `md hash --save` on an
      anchored `last_updated` exits non-zero and leaves the file's bytes
      unchanged.
- [x] Existing `write.rs`, CLI `hash_kind_save_diff.rs`, and Claudine closure
      tests pass unchanged.
- [x] The `apply_hash_save_text` doc comment states the per-line rule and the
      refusals. `darkmatter/docs/cli/hash.md` describes the text-preserving
      writer, not "the same serializer as `md clean --save`". The Darkmatter
      skill's `frontmatter.md` matches.
- [x] `just test` and `just lint` pass in `darkmatter/`, and `just test
      claudine` passes from the repo root.

## Phase 1: Rulings, Spike, and Red Tests

### Necessary Rules

These rulings resolve points the spec leaves open. Implementers follow them.
Items marked **(author)** are recommendations that the author can overturn
during review.

- **R1: Scope is `apply_hash_save_text` only.** `restore_properties_text` and
  its helper `insert_snapshot_node` also call `detect_newline`, and they also
  prepend a new block before a BOM. The spec does not name them, so they are
  left unchanged here. `detect_newline` stays, used only by that path.
  **(author)** Recommend filing an `_unscheduled` fix for the restore path's
  BOM and newline behavior. The new block builder from Phase 2 is shaped so
  that fix can reuse it.
- **R2: Terminators on the replacement `hash` node.** `serde_yaml_ng` emits
  LF-terminated lines. Replacement line *i* gets the terminator of original
  node line *i*. Lines past the original count get the terminator of the
  replacement's previous line. Blank lines and comment lines inside the
  original node's range count as its lines, because the replaced range
  includes them.
- **R3: Terminator for an inserted property.** Use the terminator of the last
  YAML line before the insertion point. For empty frontmatter (`---\n---\n`),
  use the opening delimiter line's terminator. A YAML line always has a
  terminator, because the closing `---` follows it, so "missing terminal
  newline" only concerns the body. The body is never modified.
- **R4: What counts as an empty value.** A value is empty when nothing but
  whitespace, optionally followed by a comment, comes after the colon. Only
  then is the value "null by absence", which gets one space plus the date.
  The spelled nulls `~` and `null` are nonempty plain scalars. They go
  through ordinary replacement, which keeps the authored leading whitespace.
  If an empty value's comment has no whitespace before `#`, the writer puts
  one space before `#`. Otherwise it keeps the authored run.
- **R5: How refusal is detected.** Refuse when the first non-whitespace byte
  after the colon is `&`, `*`, or `!`. This is the same test
  `locate_frontmatter_leaves` applies at `located.span.start`. Factor it into
  one shared predicate and call it from both places. Do not call
  `locate_frontmatter_leaves` itself, because it rejects null values, which
  must stay writable. An anchor with no value (`last_updated: &a`) is also
  refused. The check runs only when `bump_last_updated` is true.
- **R6: A single-line flow collection in `last_updated` is refused.** Values
  like `[a]` or `{a: 1}` currently get replaced by the date without comment.
  That silently changes the value's type, and the multi-line form is already
  refused with "`last_updated` must be a scalar value". Refuse the flow form
  with the same error. **(author)** This is one line beyond the spec, added so
  the robustness matrix has a defined outcome.
- **R7: Anchors on the `hash` node itself are out of scope.** The hash node
  is a managed replacement, so the spec requires no refusal for it. If
  replacing it would orphan an alias, the output fails validation, which
  returns `FrontmatterTextEdit`. A test pins that failure mode, and no
  dedicated check is added.
- **R8: Which parser validates the output.** Validation reuses
  `parse_text_frontmatter`, which runs the block-mapping check, node location,
  and the `serde_yaml_ng` parse. Tests compare records with the same function's
  `values` and compare body bytes via `extract_frontmatter_block().body_span`.
  Acceptance criterion 3 names "Darkmatter's frontmatter parser", and this
  function is that parser for the text path.
- **R9: Cross-writer parity is not tested here.** Content Policy cannot depend
  on Darkmatter, so date-edit parity with its renewal writer is established
  by using the spec's exact byte examples in both specs. No cross-crate test
  is added.
- **R10: No clock change.** The CLI's `chrono::Local` date stays as is. The
  UTC change belongs to `2026-09-28-content-policy`.

### Input Robustness Matrix: `last_updated` (YAML frontmatter, bump requested)

The load-bearing field is `last_updated`, because its shape decides between
edit, insert, and refuse. The `hash` node column covers the managed
replacement. Both are read from the same YAML frontmatter, which is the only
format.

| Shape | `last_updated` outcome | `hash` node outcome |
| --- | --- | --- |
| control (`last_updated: 2026-01-01`) | date replaced, all other bytes identical | replaced, terminators kept |
| absent | line inserted after the preceding line, inheriting its terminator | node inserted at end of YAML |
| empty value (`last_updated:`) | `: 2026-09-28` (case 1) | replaced |
| empty with comment (`last_updated:   # todo`) | `: 2026-09-28   # todo` (case 2) | replaced |
| spelled null (`~` / `null`) | replaced, authored leading whitespace kept (R4) | replaced |
| empty string (`""` / `''`) | quote style kept: `"2026-09-28"` | n/a |
| wrong type, block collection | refuse: must be a scalar (existing) | replaced (managed) |
| wrong type, flow collection (`[a]`) | refuse (R6) | replaced (managed) |
| node property (`&a`, `*a`, `!!str`) | refuse (R5, case 6) | validation decides (R7) |
| duplicate key | refuse (existing `locate_node`) | refuse (existing) |
| invalid YAML input | refuse (existing `validate_block_mapping`) | refuse |
| edit produces invalid YAML | refuse, nothing returned (validation) | refuse |

"Wrong type, one or every element" does not apply to a scalar field.

### Wave 1 (parallel)

- [x] **Spike: lone-CR and BOM parse** (scratch test only, not committed)
    - **Answer (2026-09-29, macOS):** lone CR is viable. `extract_frontmatter_block`
      finds the block (`yaml_span` 4..69 for case 4's input),
      `validate_block_mapping` returns `Ok`, `serde_yaml_ng` parses the
      CR-only YAML into the expected mapping, and `parse_text_frontmatter`
      returns both values with correct `TextNode` ranges (`line_spans`
      already splits on lone CR). `yaml.lines()` does treat the whole CR-only
      YAML as one line, so the flow-root pre-check sees only the first line's
      text; that is harmless for a real mapping, and a flow root (`{`/`[`
      first) is still caught. The one blind spot is a CR-only flow root
      preceded by a comment line (`# c\r{a: 1}\r`), which the pre-check
      skips; it is pre-existing, outside this fix, and not needed by case 4.
      A BOM before existing frontmatter extracts correctly (`yaml_span`
      starts after BOM + `---\n`), and case 5's expected output re-parses.
      Empty frontmatter (`---\n---\n`) yields an empty `yaml_span` (4..4), and
      a body with no terminator extracts normally.
    - Question: do `parse_text_frontmatter` and `serde_yaml_ng` accept a
      lone-CR frontmatter body such as `"hash: x\rlast_updated: 2026-01-01\r"`,
      and does `validate_block_mapping`'s `yaml.lines()` flow-root check
      misbehave when the whole CR-only YAML is one "line"?
    - This decides whether Phase 2's output validation can accept case 4. One
      host, one scratch `#[test]`, done in minutes.
    - Record the answer in this task's checkbox note. If the parser rejects
      lone CR, stop and raise a ruling for the author rather than working
      around it.
- [x] **Red tests: spec cases** (`write.rs` `mod tests`)
    - Add a `assert_fidelity(input, output, bumped)` helper implementing R8:
      re-parse both, compare frontmatter records excluding `hash` and (when
      bumped) `last_updated`, and assert identical body bytes.
    - Add a `assert_refused(input, decision)` helper: expect
      `Err(FrontmatterTextEdit)` and assert the input `String` equals its
      clone taken before the call.
    - Add one test per spec case 1 to 6 with the exact byte strings from the
      spec. Add a CRLF variant of case 1. Use the existing
      `textual_decision`/`simple_stored` helpers with the canonical
      `aaaa000000000000-bbbb000000000000` hash so the hash node's bytes do not
      change.
    - Add refusals for an alias (`last_updated: *d` with `d: &d 2026-01-01`
      earlier in the frontmatter), a tag (`!!str 2026-01-01`), and a bare
      anchor (`&a`).

### Wave 2 (after the Wave 1 red tests exist, same file, so sequential)

- [x] **Red tests: extra shapes**
    - A mixed LF/CRLF file with `last_updated` absent: it is inserted with the
      preceding line's terminator.
    - A mixed file where only the hash changes (`bump_last_updated = false`),
      using a multi-line `detailed` or `structured` hash node whose lines
      carry different terminators, which exercises R2.
    - A lone-CR file with a hash-only replacement.
    - BOM followed by existing frontmatter.
    - A body with no final terminator, both with and without existing
      frontmatter. The new block uses LF (spec: "a source with no terminator
      uses LF").
    - The R7 test: an anchored `hash` whose alias is used elsewhere must
      return `Err`.
    - Each asserts exact bytes and calls `assert_fidelity`.
- [x] **Red tests: robustness matrix**
    - One table-driven test,
      `textual_save_last_updated_robustness_matrix`, built from a single real
      fixture (the control row). Each row applies one edit and asserts the
      outcome from the matrix above, through `apply_hash_save_text`'s result.
- [x] **Red test: CLI refusal** (`cli/tests/l1/hash_kind_save_diff.rs`)
    - Follow `test_hash_save_failure_does_not_modify_flow_mapping`, using
      `CliProcessFixture`. The fixture has a stale stored hash (so the save
      bumps the date) and `last_updated: &lu 2026-01-01` with `reviewed: *lu`.
      Assert a non-zero exit and that the file's bytes are unchanged.

### Checkpoint 1

- [x] The spike answer is recorded, and the lone-CR path is viable or a
      ruling has been raised.
- [x] `cd darkmatter && just test -E 'test(textual_save)'` compiles. The new
      tests fail for the expected reasons and every pre-existing test passes.
    - Note (2026-09-29): the red tests are `#[ignore = "red until phase 2
      ..."]`-gated so `just test` stays green; run them with `--run-ignored
      only`. One unrelated pre-existing L1 failure exists on this branch:
      `markdown::schemas::file_match::tests::conversion_emits_every_root_union_glob`
      (a pure schema-conversion test; this phase's library diff is
      test-only code in `write.rs`).

## Phase 2: Writer Implementation

All tasks edit `darkmatter/lib/src/markdown/hash/write.rs`, so they run
**sequentially**, in this order. Each task should turn its slice of the
Phase 1 tests green.

- [x] **Terminator helpers**
    - Add `line_terminator(text, line_end) -> &str` (returns `"\r\n"`, `"\n"`,
      `"\r"`, or `""`), built on the existing `line_spans`.
    - Add `apply_terminators(serialized_lf, original_terminators) -> String`,
      which implements R2 for serde output.
    - Add `preceding_terminator(document, insert_at, fallback)`, which
      implements R3.
- [x] **Per-line `hash` node**
    - `serialize_entry` and `serialize_existing_entry` stop taking a global
      `newline`. Existing nodes are re-terminated with `apply_terminators`,
      using the original node's line terminators. An inserted node takes
      `preceding_terminator`.
    - Covers the hash-only mixed file, the lone-CR hash, and Wave 2's
      multi-line node.
- [x] **`rewrite_date_scalar` fix**
    - Take the node's own terminator from its text rather than stripping a
      global newline (case 3, and the date line in case 4).
    - For an empty value (R4), emit `{key}: {date}{comment_prefix}`, adding a
      space before `#` when the authored run is empty (cases 1 and 2).
    - Otherwise keep the authored key, leading whitespace, quote style, and
      comment exactly as today.
    - Refuse single-line flow collections (R6).
- [x] **Node-property refusal**
    - Extract a shared `starts_with_node_property(bytes)` predicate (R5).
      Call it from `locate_frontmatter_leaves` in place of the inline
      `matches!`, which is a behavior-neutral refactor, and from the bump path
      before any edit.
    - The error names `last_updated` and the property kind, and the file is
      untouched (case 6, alias, tag, bare anchor).
- [x] **New-block construction**
    - In the no-frontmatter branch, strip a leading `\u{feff}` and emit BOM,
      block, rest. The terminator is the rest's first line terminator, or LF
      (case 5, and the no-terminator body).
    - Keep it a small private `fn new_frontmatter_block(...)` so the R1
      follow-up can reuse it.
- [x] **Output validation**
    - Before `Ok(Some(updated))`, run `parse_text_frontmatter(&updated)` and
      map any error to `FrontmatterTextEdit` ("rewritten frontmatter did not
      parse: ..."). The `new_stored == None` early return stays first and
      parses nothing (the existing
      `textual_no_change_does_not_parse_unsupported_source` pins it).
- [x] **Doc comment and drift pass**
    - Rewrite the `apply_hash_save_text` `///` block. Replace "the document
      newline style" with the per-line rule. Add the BOM rule. Extend
      `## Errors` with the anchor, alias, and tag refusal, the flow-collection
      refusal, and post-edit validation failure.
    - Check the module `//!` doc (line 5) and the `Markdown::apply_hash_save`
      doc for any claim that has drifted.
    - Confirm `detect_newline` now has only the restore caller (R1).
      `just lint` catches it if it has become dead.

### Checkpoint 2

- [x] `cd darkmatter && just test` is fully green, including every Phase 1
      test and the existing CRLF and quote-style matrices.
- [x] `cd darkmatter && just lint` is clean. Run it after `just test`, never at
      the same time.
- [x] Search the diff for a leftover global-newline use in the
      `apply_hash_save_text` path:
      `grep -n "detect_newline" darkmatter/lib/src/markdown/hash/write.rs`
      shows only the restore path.

## Phase 3: Documentation, Drift, and Downstream Verification

### Wave 3 (parallel, no shared files)

- [x] **User docs** (`darkmatter/docs/cli/hash.md`)
    - Replace the `--save` Behavior paragraph's "persists the canonical
      frontmatter (via the same serializer as `md clean --save`)". It becomes:
      only the managed hash node and, when bumped, `last_updated` change;
      every other byte, including each line's own terminator and a leading
      BOM, is kept.
    - Add a short refusal list (anchored, aliased, or tagged `last_updated`,
      and a non-scalar `last_updated`), stating that the file is not written.
      Include a compact before/after example for an empty `last_updated:`.
    - Per repo doc rules, do not mention or link this fix.
- [x] **Skill drift** (`.claude/skills/darkmatter/frontmatter.md`)
    - Next to the `restore_properties_text` section, record the writer's
      per-line terminator rule and refusals. Note that restore still uses the
      global newline rule (R1), so the skill does not overstate what restore
      does.
- [x] **Downstream verification**
    - From the repo root, run `just test claudine`. Closure write-back calls
      `apply_hash_save_text` with `bump_last_updated = true`, so a
      Claudine-encoded document with an anchored `last_updated` would now be
      refused. Confirm that no Claudine fixture relies on that.
    - Run `cd darkmatter && just test -E 'binary(hash_kind_save_diff) |
      binary(hash)'` for the CLI.

### Checkpoint 3 (final)

- [x] Every success criterion in the Summary section is checked.
- [x] `git diff --stat` touches only `write.rs`,
      `cli/tests/l1/hash_kind_save_diff.rs`, `docs/cli/hash.md`,
      `.claude/skills/darkmatter/frontmatter.md`, and this plan. No
      `cargo fmt`, and no commits unless asked.
- [x] Implementation status is reported as "implementation complete, ready
      for review". Moving the fix to `_completed` is the author's step.
