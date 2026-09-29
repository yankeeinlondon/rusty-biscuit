---
area: darkmatter
status: draft-spec
created: 2026-09-28
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
related:
    - 2026-09-28-content-policy
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
writer with a span-targeted editor on 64 fixtures (its `findings.md`, section
3, and `results/matrix.txt`). The writer breaks the promise in five cases, and
a sixth, an anchored target, it cannot edit safely at all:

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
- The located node text includes anchors and tags, and nothing checks for
  them (case 6).

## Expected Behavior

Every case changes only the date bytes (or inserts only the new line or
block) and leaves all other bytes, including line terminators, unchanged.
Each input below is a spike fixture; `2026-09-28` is the stamped date, and the
`hash` value is already current, so only `last_updated` changes.

1. **Empty value.** Insert one space and the date after the colon.

   ```text
   in:  "---\nhash: abc-def\nlast_updated:\ntitle: x\n---\n"
   out: "---\nhash: abc-def\nlast_updated: 2026-09-28\ntitle: x\n---\n"
   ```

   The CRLF form behaves the same:
   `"---\r\nhash: abc-def\r\nlast_updated:\r\n---\r\n"` becomes
   `"---\r\nhash: abc-def\r\nlast_updated: 2026-09-28\r\n---\r\n"`.

2. **Empty value with a comment.** Put the date after one space, keep the
   comment, and keep at least one space before `#`.

   ```text
   in:  "---\nhash: abc-def\nlast_updated:   # todo\n---\n"
   out: "---\nhash: abc-def\nlast_updated: 2026-09-28   # todo\n---\n"
   ```

3. **LF line in a CRLF file.** No extra line terminator is written. The exact
   output depends on the [mixed line-ending decision](#decision-mixed-line-endings).

   ```text
   in:           "---\r\nhash: abc-def\r\nlast_updated: 2026-01-01\n---\r\nBody\r\n"
   out per-line: "---\r\nhash: abc-def\r\nlast_updated: 2026-09-28\n---\r\nBody\r\n"
   out global:   "---\r\nhash: abc-def\r\nlast_updated: 2026-09-28\r\n---\r\nBody\r\n"
   ```

4. **Lone CR.** Every CR terminator survives, on the date line and on the
   rewritten `hash` line.

   ```text
   in:  "---\rhash: abc-def\rlast_updated: 2026-01-01\r---\rBody\r"
   out: "---\rhash: abc-def\rlast_updated: 2026-09-28\r---\rBody\r"
   ```

5. **BOM without frontmatter.** The BOM stays the first bytes of the file,
   and the new block follows it. With the decision to bump `last_updated` (as
   Claudine's write-back forces):

   ```text
   in:  "\u{feff}# Title\n\nBody\n"
   out: "\u{feff}---\nhash: abc-def\nlast_updated: 2026-09-28\n---\n# Title\n\nBody\n"
   ```

6. **Anchored target.** The writer refuses with
   `MarkdownError::FrontmatterTextEdit` and writes nothing. Keeping the anchor
   would silently change every alias of it too, so refusing is the only choice
   that keeps the other values unchanged.

   ```text
   in:  "---\nhash: abc-def\nlast_updated: &lu 2026-01-01\nreviewed: *lu\n---\n"
   out: error, file unchanged
   ```

A `last_updated` value written with an alias (`*d`) or a tag (`!!str …`) is
already replaced correctly and is not part of this fix.

## Decision: Mixed Line Endings

The owner of this fix decides how the writer picks a line terminator when a
file mixes them. This affects case 3, the terminator of any inserted line, and
the re-serialized `hash` node.

| Option | Rule | Consequence |
| --- | --- | --- |
| Global (current) | One newline for the file: CRLF if any CRLF occurs, else LF (extended to recognize a CR-only file) | Rewritten lines are normalized toward the dominant style; a mixed file changes bytes outside the edited value |
| Per-line (the spike's approach) | An edited line keeps its own terminator; an inserted line takes the terminator of the line before it | Only the edited value changes; a mixed file stays exactly as mixed as it was |

The per-line rule is the one that keeps the writer's own promise ("all other
bytes are retained"). Whichever is chosen, the tests for case 3 pin it.

## Relationship to Content Policy

Content Policy's renewal writer (`2026-09-28-content-policy`) edits the same
`last_updated` property with span-targeted byte edits and cannot call this
writer, because the content-policy library must never depend on Darkmatter.
It is specified not to reproduce any of the six defects above. After this fix,
**both writers produce identical bytes for the same `last_updated` edit**, so a
document renewed by `policy renew` and one stamped by `md hash` or Claudine's
write-back differ only in the date they wrote. If this fix chooses the global
line-ending rule, Content Policy's insertions follow the same rule.

## Acceptance Criteria

1. Each of the six cases in [Expected Behavior](#expected-behavior) has a unit
   test in `write.rs` that asserts the exact output bytes (or, for case 6, the
   error and an unchanged input), using the inputs shown.
2. Case 1 also has a CRLF test, and case 3 has a test for an inserted
   `last_updated` line in a mixed file, both matching the chosen line-ending
   rule.
3. Every output in criteria 1 and 2 re-parses with Darkmatter's frontmatter
   parser to the input's record with only `last_updated` changed.
4. The existing `write.rs` tests, including the CRLF and quote-style tests,
   still pass unchanged.
5. The doc comment on `apply_hash_save_text` states the line-ending rule that
   was chosen and the anchored-target refusal.
