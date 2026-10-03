# Spike: Frontmatter Reader and Byte-Exact Edits

Date: 2026-09-28. Run on macOS. Throwaway code in this directory:
`src/lib.rs` (strict reader, date editor, property insertion), `src/bin/corpus.rs`
(repo-wide parity run), `src/bin/matrix.rs` (fixture matrix). Raw output:
`results/corpus.txt`, `results/matrix.txt`.

## Question

Is "one strict frontmatter reader plus span-targeted edits through Biscuit
File's YAML helpers" sound on the repository's real Markdown, and how does it
relate to Darkmatter's more forgiving parser and its `last_updated` writer?

## 1. Corpus Parity with Darkmatter's Parser

5,113 `.md` files; 3,084 begin with `---`.

| Outcome | Files |
| --- | --- |
| Both parse, identical records | 3,029 |
| Both reject (19 duplicate `packages:` keys, 5 YAML errors) | 24 |
| Both see no frontmatter | 3 |
| Spike parses, Darkmatter fails | 0 |
| Darkmatter parses only through a fallback | 28 |

- A first run showed 42 record mismatches, all a clip-chomped block scalar
  (`description: >`) as the last key. Parsing the YAML without its final line
  terminator, as Darkmatter does, removes all 42. The real reader must do the
  same.
- **18 files need Darkmatter's tab normalization.** All are research documents
  with `last_updated` (for example `.claude/skills/lsp/*`, `playa`/`sniff`
  audio-programming, `biscuit-visualized` charting). Eight of them also carry
  a tab-indented `update_policy: - Duration(6mo)`.
- **10 files need `{{ }}` expression protection.** All are in `prompts/` and
  carry no date or policy key. None needed `$(...)` protection.

## 2. Fixture Matrix (64 fixtures)

- 49 edits passed the byte-diff, re-parse, and record checks.
- 12 were refused as intended.
- 1 produced a broken file: an anchor on the target
  (`last_updated: &lu 2026-09-28` with `reviewed: *lu`). The located span
  includes `&lu`, so replacing it breaks the alias. Darkmatter's writer has
  the same defect.
- All 12 date-only edit paths change only the date bytes under LF, CRLF, lone
  CR, and a UTF-8 BOM. CRLF needs no offset translation, because `locate`
  runs on the raw slice.
- How YAML 1.1-looking values arrive: `yes`, `on`, `y` stay strings; `True`
  becomes `true`; `0o7` becomes `7`; `010` stays the string `"010"`; dates stay
  strings; `.inf`/`.nan` become `null` (so they read as a missing baseline).

## 3. Parity with Darkmatter's `last_updated` Writer

25 outputs byte-identical, 9 different. The differences are Darkmatter
defects, not spike choices, except the last:

- `last_updated:` with no value becomes `last_updated:2026-09-28`, which does
  not parse.
- `last_updated:   # todo` becomes the value `"2026-09-28# todo"`.
- An LF line in a file containing any CRLF gains an extra `\n\r\n`.
- A lone CR becomes CRLF.
- With a BOM and no frontmatter, the new block is written before the BOM.
- Mixed line endings differ by design: Darkmatter uses one global newline
  style; the spike uses the ending of the line preceding the edit.

## 4. Biscuit File Helper Behavior

CRLF, trailing comments, BOM, multibyte text, and `apply_edit_set` all behaved
correctly. Three defects:

- **Zero-indent lists get the wrong parent.** `content_policy:\n- X` is located
  at root path `[Index(0)]` instead of under the key, because `parent_path`
  only accepts a strictly less-indented parent. Five of the six
  `content_policy: Duration(...)` notes use this style, as do 199 documents
  across the corpus. Renewing `last_updated` is unaffected; editing a date
  inside a rule in that style is refused safely.
- Multi-line scalars return a first-line fragment rather than `None`, contrary
  to the module docs.
- The returned span includes anchors and tags, and can be an alias.

## Recommendations

- **Reader contract:** strict parsing, with a rejection message that names the
  Darkmatter fallback that would have applied (tab indentation, or an unquoted
  template expression). Pair it with a one-time tab cleanup of the 18 research
  documents. Replicating the fallbacks would copy about 250 lines of
  Darkmatter code that would drift, and renewal still could not edit those
  documents byte-exactly. Alternative: accept tab normalization for evaluation
  only.
- **Add to renewal's refuse list:** unterminated blocks (including a `...`
  closing line) rather than creating a second block; `----` near-miss fences;
  anchors, aliases, or tags on the edited value; flow-mapping items
  (`- {rule: …}`); rule dates in zero-indent lists until Biscuit File is fixed
  (with a message that does not say "flow"); a located span that does not
  decode to the current value.
- **Safety net:** re-read after every edit and require the record to be
  unchanged apart from the target; otherwise write nothing.
- **Biscuit File:** fix zero-indent parenting (prerequisite only for editing
  inline rule dates in zero-indent lists); correct the multi-line behavior or
  docs; expose the anchor/tag case.
- **Spec corrections:**
  - `content_policy: [ValidFor(3mo, @last_updated)]` is invalid YAML in both
    parsers (`@` is reserved once the comma splits the item), so the
    comma-trap diagnostic is never reached for `@` references. The quoted form
    works and renews.
  - "Matches Darkmatter's writer" must exclude the defects in section 3.
  - A further 17 documents carry `update_policy:` with `Duration(6mo)`
    (`.claude/skills`, `claudine/docs`, `sniff/docs`); the migration count of
    six omits them.
