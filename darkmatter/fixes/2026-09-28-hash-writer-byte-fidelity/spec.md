---
area: darkmatter
status: draft-spec
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-29
review_iterations: 0
created: 2026-09-28
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
related:
    - 2026-09-28-content-policy
human_review: false
message_to_agent: |-
    Phase 1 is complete. All red tests are in place and gated with
    `#[ignore = "red until phase 2 of 2026-09-28-hash-writer-byte-fidelity: <slice>"]`,
    following the schema-plus convention in `grammar.rs`. As each Phase 2 task
    lands, delete the `#[ignore]` on the tests for its slice (the reason names
    the slice: empty date value, per-line terminators, BOM stays first,
    node-property refusal, output validation, robustness matrix). Also remove
    the one in `darkmatter/cli/tests/l1/hash_kind_save_diff.rs`
    (`test_hash_save_refuses_anchored_last_updated_without_writing`). Update the
    comment above `CANONICAL_HASH` in `write.rs` `mod tests` once no gated test
    remains. List them with:
    `cargo nextest run -p darkmatter --lib --run-ignored only -E 'test(textual_save)'`.

    Messages the tests pin: node-property refusals must contain `last_updated`
    and the word `anchor`, `alias`, or `tag` (a bare `&a` counts as anchor).
    The CLI test asserts stderr contains `last_updated`; writer reasons do reach
    stderr through the `MarkdownError` block.

    Matrix rows that are red today, and only these: the three empty-value rows,
    the two single-line flow-collection rows (R6), the four node-property rows,
    and `hash edit orphans an alias` (R7, caught by output validation). Every
    other row already passes, so a Phase 2 change that turns one of them red is
    a regression.

    Spike result: lone-CR frontmatter parses end to end (extraction,
    `validate_block_mapping`, `serde_yaml_ng`, node location). No workaround is
    needed for case 4.

    One unrelated L1 failure predates this fix on this branch:
    `markdown::schemas::file_match::tests::conversion_emits_every_root_union_glob`.
    It is not caused by this work.
---

# Hash Writer Byte Fidelity

## Problem

Darkmatter's byte-preserving frontmatter writer, `apply_hash_save_text` and
its `rewrite_date_scalar` helper in
[`darkmatter/lib/src/markdown/hash/write.rs`](../../lib/src/markdown/hash/write.rs),
promises to change only the managed `hash` node and the `last_updated` scalar
and to keep "the document newline style and all other frontmatter and body
bytes". Two callers rely on that promise:

- `md hash` ([`darkmatter/cli/src/commands/hash.rs`](../../cli/src/commands/hash.rs))
- Claudine's closure write-back
  ([`claudine/lib/src/composition/closure.rs`](../../../claudine/lib/src/composition/closure.rs))

Darkmatter's effect auto-rehash does **not** use this writer. It calls the
map-based `Markdown::apply_hash_save`, which re-serializes the whole
document, so it is out of scope here.

The frontmatter-reader spike of `2026-09-28-content-policy` compared this
writer with a span-targeted editor on 64 fixtures (see its
[findings](../../../content-policy/features/2026-09-28-content-policy/spikes/frontmatter-reader/findings.md#3-parity-with-darkmatters-last_updated-writer)
and [fixture matrix](../../../content-policy/features/2026-09-28-content-policy/spikes/frontmatter-reader/results/matrix.txt)).
The writer breaks the promise in five cases, and a sixth, an anchored target,
it cannot edit safely at all:

| # | Input shape | What the writer produces |
| --- | --- | --- |
| 1 | `last_updated:` with no value | `last_updated:2026-09-28`, no space after the colon; the frontmatter no longer parses |
| 2 | `last_updated:   # todo` (no value, trailing comment) | `last_updated:   2026-09-28# todo`, which parses as the string `"2026-09-28# todo"` |
| 3 | An LF-terminated `last_updated` line in a file containing any CRLF | The line ends `\n\r\n`: an extra blank line inside the frontmatter |
| 4 | A file whose lines end with a lone CR | The `last_updated` line ends CRLF, and the rewritten `hash` line ends LF |
| 5 | A UTF-8 BOM and no frontmatter | The new block is written **before** the BOM, so the BOM lands in the body |
| 6 | An anchor on the target, `last_updated: &lu 2026-01-01`, aliased elsewhere | The replaced span includes `&lu`, so the anchor is deleted and `*lu` no longer resolves: the file does not parse |

### Root causes

- `rewrite_date_scalar` writes `{key}:{leading}{value}{comment}` and reuses the
  authored whitespace after the colon. With no value, `leading` is empty (case
  1) or is the whole run before `#` (case 2), so nothing separates the colon
  from the value, or the value from the comment.
- `detect_newline` picks one newline for the whole file: CRLF if any CRLF
  occurs, else LF. `rewrite_date_scalar` strips that newline from the node's
  line and appends it again. On an LF line in a CRLF file the strip fails and
  the append adds `\r\n` after the kept `\n` (case 3). A lone CR is never
  detected, so LF and CRLF appear where CR was (case 4). The re-serialized
  `hash` node uses the same global newline.
- The no-frontmatter branch prepends the block to the document text, BOM
  included (case 5).
- The located node text includes anchors, aliases, and tags, and nothing
  checks for them (case 6 and related node-property shapes).

## Expected Behavior

For an existing date, change only its scalar spelling and the minimum spacing
needed to keep valid YAML; preserve its line terminator. An inserted property
or frontmatter block necessarily adds bytes. The examples below show only the
date edit or block insertion: the hash decision uses an already canonical
simple hash, so the managed hash node has the same bytes after serialization.
`2026-09-28` is the caller-supplied date. The date writer does not choose a
clock or change when a save bumps the date.

1. **Empty value.** Insert one space and the date after the colon.

   ```text
   in:  "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated:\ntitle: x\n---\n"
   out: "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28\ntitle: x\n---\n"
   ```

   The CRLF form behaves the same:
   `"---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated:\r\n---\r\n"` becomes
   `"---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated: 2026-09-28\r\n---\r\n"`.

2. **Empty value with a comment.** Put the date after one space, keep the
   comment, and keep at least one space before `#`.

   ```text
   in:  "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated:   # todo\n---\n"
   out: "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28   # todo\n---\n"
   ```

3. **LF line in a CRLF file.** No extra line terminator is written. The LF
   survives under the [per-line decision](#decision-preserve-line-endings-per-line).

   ```text
   in:  "---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated: 2026-01-01\n---\r\nBody\r\n"
   out: "---\r\nhash: aaaa000000000000-bbbb000000000000\r\nlast_updated: 2026-09-28\n---\r\nBody\r\n"
   ```

4. **Lone CR.** Every CR terminator survives, on the date line and on the
   rewritten `hash` line.

   ```text
   in:  "---\rhash: aaaa000000000000-bbbb000000000000\rlast_updated: 2026-01-01\r---\rBody\r"
   out: "---\rhash: aaaa000000000000-bbbb000000000000\rlast_updated: 2026-09-28\r---\rBody\r"
   ```

5. **BOM without frontmatter.** The BOM stays the first bytes of the file,
   and the new block follows it. With the decision to bump `last_updated` (as
   Claudine's write-back forces):

   ```text
   in:  "\u{feff}# Title\n\nBody\n"
   out: "\u{feff}---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: 2026-09-28\n---\n# Title\n\nBody\n"
   ```

6. **Anchored target.** The writer refuses with
   `MarkdownError::FrontmatterTextEdit` and writes nothing. Keeping the anchor
   would silently change every alias of it too, so refusing is the only choice
   that keeps the other values unchanged.

   ```text
   in:  "---\nhash: aaaa000000000000-bbbb000000000000\nlast_updated: &lu 2026-01-01\nreviewed: *lu\n---\n"
   out: error, file unchanged
   ```

The same refusal applies to an alias (`*d`) or tag (`!!str 2026-01-01`) on
`last_updated`. Darkmatter's existing leaf locator treats all three as node
properties that cannot be safely edited in place. A tag may carry type
meaning, and replacing an alias can detach it from its source even when the
resulting YAML parses. Neither case is an ordinary quoted or plain scalar.

## Decision: Preserve Line Endings Per Line

Keep each existing edited line's own LF, CRLF, or lone-CR terminator. For an
inserted line, use the terminator of the preceding frontmatter line; for a
new block, use the body's first terminator, or LF if the body has none. This
also governs the serialized `hash` node: preserve the original terminator of
each line that still exists in the replacement, and use the preceding line's
terminator for any additional lines. A missing terminal newline stays missing
where the output can remain valid YAML. The rule applies equally when only
the hash changes and `last_updated` is not bumped.

| Option | Rule | Consequence |
| --- | --- | --- |
| Global (current) | One newline for the file: CRLF if any CRLF occurs, else LF (extended to recognize a CR-only file) | A mixed file changes existing line terminators, and the current implementation can append a second terminator |
| Per-line (chosen) | An edited line keeps its terminator; an inserted line inherits a nearby terminator | Existing line endings survive; a mixed file stays mixed |

The per-line rule follows the writer's byte-preservation contract and the
Content Policy renewal editor's established behavior. The hash node is a
managed replacement, so its value may change, but replacing its scalar must
not needlessly change its original line terminator. This decision selects
the output in case 3.

## Relationship to Content Policy

Content Policy's renewal writer (`2026-09-28-content-policy`) edits the same
`last_updated` property with span-targeted byte edits and cannot call this
writer, because the content-policy library must never depend on Darkmatter.
It is specified not to reproduce any of the six defects above. For the same
input, date, and insertion location, the **date edit** should use the same
bytes in both writers. Whole documents need not match: `md hash` and
Claudine's write-back also update the managed hash, while Content Policy
renewal can edit other policy values. The separate Content Policy feature
also changes the clocks at all three date-stamping call sites to UTC; this
writer accepts a date from its caller and does not implement that clock change.

## Editing and Refusal Rules

- Keep the existing authored key, quote style, whitespace before a nonempty
  value, trailing comment, and line terminator. For a null value, insert a
  single space after the colon, then the date. If there is a comment, keep
  its authored spacing after the inserted date; do not join `#` to the date.
- Refuse an anchored, tagged, or aliased `last_updated` value when a date bump
  is requested. Report `MarkdownError::FrontmatterTextEdit`; leave the input
  untouched. An absent or null date remains writable. A save that does not
  bump the date need not inspect its node properties.
- Reuse Darkmatter's frontmatter extraction and node-location rules. Preserve
  a leading UTF-8 BOM when creating a block. A source with no terminator uses
  LF for its new block.
- Validate the resulting frontmatter before returning a rewritten document.
  If parsing fails, return `FrontmatterTextEdit` so callers never persist a
  partially edited result. A `SaveDecision` with no new stored hash still
  returns `None` without parsing or modifying its input.

## Acceptance Criteria

1. Each of the six cases in [Expected Behavior](#expected-behavior) has a unit
   test in `write.rs` that asserts exact output bytes or the refusal. Case 1
   also covers CRLF. Add the alias and tag refusal cases, including an alias
   whose source is elsewhere in the frontmatter.
2. Cover a mixed file where `last_updated` is inserted, a mixed file where
   only the hash changes, a lone-CR hash replacement, a BOM before existing
   frontmatter, and a body without a line terminator. Assert that untouched
   source spans retain their bytes and existing line terminators survive.
3. Re-parse each successful output with Darkmatter's frontmatter parser. Its
   record differs from the input only at the managed hash and, when bumped,
   `last_updated`; all other properties and body bytes stay the same. For a
   new block, those are the only new properties. Assert that every refusal
   returns an error and leaves caller-owned input unchanged.
4. The existing `write.rs` tests, including CRLF and quote-style coverage,
   still pass. Update the `apply_hash_save_text` doc comment to state the
   per-line rule and the refusal of anchored, tagged, or aliased dates.
5. Correct the current [`md hash` guide](../../docs/cli/hash.md) statement that
   `md hash --save` uses the whole-frontmatter serializer; document its text-preserving
   behavior and these refusal cases in the current user-facing docs.
