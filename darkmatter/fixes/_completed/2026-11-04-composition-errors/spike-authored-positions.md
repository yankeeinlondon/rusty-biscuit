# Spike: authored positions for reference diagnostics

Date: 2026-11-05. A time-boxed spike for `2026-11-04-composition-errors`.

The question: can the spec promise that every broken-reference diagnostic
carries an exact authored-file `line`/`column`, a `section` slug, and a
transclusion trail of `(document, directive line)` steps? The spike ran the
worktree's `md` (`cargo build -p darkmatter-cli`) and a throwaway probe crate
against the `darkmatter` library. Neither the library nor the CLI was
changed. Fixtures and the probe are in the session scratchpad
(`…/scratchpad/spike-positions/{fx,probe}`).

## Verdict

**Yes, for every reference that starts in authored text, at low cost.** The
map needed already exists and holds up through every stage that runs during
reference preparation, but it is thrown away before references are
extracted. Today every reported position is in *prepared-body* space, which
is wrong whenever the document has frontmatter or any stage rewrote the
body.

Two cases cannot have an exact position of their own, and the spec needs a
rule for each:

1. A reference whose first byte was **inserted** by an expression, a shell
   command, or a `replace:` value. It has no authored bytes. Point at the
   authored construct that inserted it instead.
2. A document with **no position map**: one built in memory, or one whose
   frontmatter block failed to extract. Report no position rather than a
   prepared-space number.

The section slug and the trail can be exact in every case where the location
can.

## Fixtures

`root.md` has 6 frontmatter lines (`---`, `title`, `multi` (three lines),
`draft: false`, `replace: {ZZ: …}`, `---`). Its body contains:

- a heading with an interpolation: `# Intro {{ title }}`
- `{{ multi }}`, which expands to 3 lines
- `ZZ then [missing two]…`, where a replacement on the same line lengthens it
- a 4-line `::block when="draft"` region that is removed
- `Café — [missing five]…`, to test a multibyte column
- `::file ./child.md`
- a broken `#no-such-heading` fragment

`child.md` has 4 frontmatter lines, a heading, a broken file link, a broken
cross-file fragment, a broken local fragment, and
`Data: {{ inserted }}`, where `inserted` is a Markdown link.
`root-crlf.md` is `root.md` with CRLF line endings. `twice.md` transcludes
`mid.md` twice, and `mid.md` transcludes `child.md`.

## Findings table

**Reported** is what `md validate refs --fragments --format json` emits
today: `line` only, with no column, span, or trail. LF and CRLF give
identical output. The **prepared col** column comes from `ReferenceOrigin.span`
via the probe; no output exposes it. **True** is the authored file position.
Columns are 1-based characters, measured to the reference's first character
(the `[` of a Markdown link), which is how the spec's `types.md:14:83`
example counts.

| # | Issue | Doc | Reported line | Prepared col (byte / char) | True line:col | Section (prepared slug) | Error |
|---|---|---|---|---|---|---|---|
| 1 | `./missing-one.md` | root | 3 | 5 / 5 | **9:5** | `intro-root` | −6 (frontmatter) |
| 2 | `./missing-two.md` | root | 11 | 32 / 32 | **15:9** | `second` | −6 fm, +2 interpolation; col +23 (replace) |
| 3 | `./missing-three.md` | root | 14 | 13 / 13 | **22:13** | `second` | −6 fm, +2 interp, −4 block |
| 4 | `./missing-five.md` | root | 16 | 11 / 8 | **24:8** | `second` | as #3; byte col ≠ char col |
| 5 | `#no-such-heading` | root | 20 | 1 / 1 | **28:1** | `second` | as #3 |
| 6 | `./root.md#nope` | child | 3 | 9 / 9 | **7:9** | `child-heading` | −4 (child frontmatter) |
| 7 | `./missing-four.md` | child | 3 | 43 / 43 | **7:43** | `child-heading` | −4 |
| 8 | `./missing-ins.md` (warning, inserted) | child | 5 | 7 / 7 | none of its own; inserted by `{{ inserted }}` at **9:7** | `child-heading` | −4; no authored bytes |
| 9 | `#absent` | child | 7 | 1 / 1 | **11:1** | `child-heading` | −4 |
| T | trail of #6–#9 | root→child | `directive_line=18` | — | **`root.md:26`** | (`Second`) | −6 fm, +2, −4 |

Other observations:

- `md compose root.md --output report-json`: all six reference diagnostics
  have `"source": null, "span": null`. Compose does not check fragments, so
  #5, #6, and #9 are absent there.
- `twice.md`: the same `mid.md` and `child.md` issues are reported once per
  insertion path, so each issue has exactly one trail. Every node shows
  `directive_line=3` for `mid.md`'s `::file`, but the authored line is 6.
- The same root cause already affects a non-reference diagnostic.
  `::file ./child.md when="nosuch_fn(1)"` at authored line 26 reports
  `Failed to evaluate condition … at line 18` with `source: null`.
- Proof that the map works end to end: an interpolation error placed after
  the replacement and the removed block (`err.md`, `err-crlf.md`) reports
  `source: {line: 24, column: 8}`. That is exact, with a character column
  after `Café — `, and it is the same for LF and CRLF. That path goes
  through `BodyOrigin` and `AuthoredSpan::locate`.

## Root causes

1. **References are extracted from the prepared body.**
   `build_node` calls `prepare_content` (`lib/src/markdown/reference/graph.rs:300`),
   then `extract_all_references(&prepared_content, …)` (`graph.rs:326`).
   `prepare_content` (`graph.rs:942–977`) runs `compose_with_options` with
   only TextReplacement, PageBlocks, Interpolation, and ShellExpansion, and
   returns `result.content()`. That is the body without frontmatter, after
   rewriting. Every `ReferenceOrigin.line`/`span` (`types.rs:242–251`) is
   therefore body-relative and post-rewrite. The JSON emits it unchanged
   (`validate.rs:298`), and so does the CLI's "inside inserted text"
   message (`cli/src/commands/compose.rs:1037`).
2. **The map back is computed and then discarded.** The pipeline captures
   `BodyProvenance` (`compose/pipeline/mod.rs:230`), and every inline-pre
   stage advances it: `replacement.rs:68`, `page_blocks.rs:78`,
   `interpolation.rs:148–149`, `shell_expansion.rs:188`. At the end, only
   `body.data` and `body.scopes`/`authored` are copied into the report
   (`pipeline/mod.rs:323–325`). `body.origin` is dropped, so
   `prepare_content` has nothing to project with.
3. **Transclusion and TOC directive lines are prepared-space too.**
   `ReferenceInsertion.directive_line` is `directive.line` from
   `parse_directives_in(&prepared_content, …)` (`graph.rs:385–470`).
4. **Some spans are in other coordinate spaces.**
   - CSS `@import`/`@font-face` records use offsets into the style block's
     `css_content`, not into the body (`reference/css.rs:46–57`).
   - Prologue and epilogue records use the sentinels `line: 0` and
     `line: usize::MAX` with `span: 0..0` (`graph.rs:696`, `graph.rs:781`).
5. **Headings carry no offsets out of the graph.** `build_heading_index`
   (`graph.rs:232–245`) has each TOC node's `source_span` and `line_range`,
   both in prepared space, but keeps only `(line, title, level)` plus the
   slugs. `PreparedHeadingSnapshot` (`reference/snapshot.rs:32`) stores
   `slugs` only.

## How reliable the map is

`BodyOrigin` (`compose/body_origin.rs`) maps runs of the current body to
**byte offsets in the loaded file, frontmatter included**. Once projected,
`AuthoredSpan::locate` (`markdown/types.rs:87`) gives a 1-based line and a
character column. CRLF is handled: `capture` skips each `\r` before a `\n`
(`body_origin.rs:115–140`), and `locate` counts `\n` and the characters
since the last one.

**When it is dropped.** "Advisory" means the map is discarded when it stops
describing the text. That happens in three places:

1. `capture` returns `None`: the document was not parsed from text
   (`loaded_source()` is `None`, so a `Markdown::new`/in-memory document), or
   `extract_frontmatter_block` failed (`body_origin.rs:109–114`).
2. Alignment stops at the first byte where the body and the loaded text
   differ other than CRLF. Bytes after that point have no origin. The probe
   (`probe/src/bin/capture.rs`, which emulates `capture`) found full
   alignment for LF, CRLF, BOM with and without frontmatter, empty
   frontmatter with CRLF, no frontmatter with CRLF, blank lines after
   frontmatter, trailing whitespace on the fences, trailing newlines, and
   tabs. No real-world case of truncation was found.
3. `advance` filters the map on `describes(before)` (`body_origin.rs:801`).
   This drops it only if some stage rewrote the body without reporting its
   edits. The same edits drive `DataRanges`, which is **not** advisory:
   `ensure_describes` makes the next `advance` fail
   (`body_origin.rs:767`). So an untracked rewrite during inline-pre fails
   preparation outright. It cannot leave a stale `BodyOrigin` behind a valid
   run. In practice, for a file-sourced document the map is always present
   at the end of `prepare_content`.

**What does not project:**

- **Inserted text** (`EditOrigin::Data` or a `replace:` value) has no
  segment. `project` returns `None` (test `inserted_text_has_no_origin`).
  The authored construct that inserted it lies exactly in the gap between
  the neighboring segments' origins. A small helper can return that range.
- **A whole span that straddles an edit** does not project (test
  `a_span_straddling_an_edit_does_not_project`), for example
  `[{{ title }} docs](./x.md)`. Only the reference's **first character**
  needs projecting, and `[` is authored. `site_scope::authored_location`
  (`site_scope.rs:187–199`) already uses this fallback chain: whole span,
  then opener, then file only.
- **`::map`/`::filter` bodies** are never expanded by `prepare_content`
  (`only(&enabled)` excludes CollectionBlocks). References inside them are
  read from authored template text. Their positions project, but they are
  per template, not per item. This was already true; it is not caused by
  the position change.

**Cost.** The map is already built on every compose. Carrying it out is one
field on `ComposeReport` (beside `body_data`, `context/report.rs:100`).
Projection is a binary search per record, plus one `locate` per record over
the loaded text. A line-start index makes that O(log n). The cost is
negligible next to preparation itself.

**Headings and sections.** The TOC that `build_heading_index` builds over the
prepared content has byte `source_span`s in the same prepared space as
reference spans. The innermost heading for a reference can therefore be
chosen by prepared byte offset, with no projection. The slug that matters is
the **prepared** one: `# Intro {{ title }}` is anchored as `intro-root` in
the composed output, and its authored text would slug to `intro-title`, an
anchor that exists nowhere. The spec's "innermost heading above the problem
in the authored file" should say *as the document's anchors name it*. A
heading inside a removed `::block` correctly does not count. One risk, not
verified: `build_heading_index` re-parses the prepared body with
`Markdown::new`, so a body that begins with `---` could be misread as
frontmatter and shift every span.

**Trail.** Directive openers (`::`) are never written by a stage
(`body_origin.rs` `scoped_site` comment). Projecting `directive.span.start`
therefore always succeeds when the map exists. Each issue is already
emitted once per insertion path (the `twice.md` result), so a trail can be
accumulated during the graph walk without deduplication problems.

## Options

| Option | Pros | Cons |
|---|---|---|
| **A. Carry `BodyOrigin` out of `prepare_content` and project each record's first character** (recommended) | Exact for all authored references, CRLF, and multibyte text. Reuses `BodyOrigin` and `AuthoredSpan`, which already back expression diagnostics. Small change. | Needs explicit rules for inserted references and for a missing map. Changes the meaning of `md validate refs` JSON `line` (baseline fixtures). |
| B. Add `frontmatter_line_count()` to the prepared line | Trivial. | Still wrong after any interpolation, block, or replacement (#2–#5 above). No column. Rejected. |
| C. Extract references from the authored body, then apply preparation separately | Positions exact by construction. | Loses everything preparation exists for: `when=`, interpolated targets, and removed blocks. Rejected. |
| D. Re-locate each reference by searching the authored text for its target string | No pipeline change. | Ambiguous when a target appears twice, wrong for interpolated targets, and fragile. Rejected. |

## Recommendation: what the spec should promise

Replace "every broken-reference diagnostic carries an exact authored
`line`/`column`" with this rule:

1. **Exact.** The first character of the reference is projected through the
   document's body map to the loaded file. `line` and `column` are then the
   1-based authored line and character column (CRLF-neutral). This covers
   every reference whose first character the author wrote.
2. **Inserted.** If the first character was written by an expression, a
   shell command, or a `replace:` value, `line`/`column` locate the authored
   construct that wrote it (`{{ … }}`, `$( … )`, or the replaced key), and
   the diagnostic says so (for example `source.inserted: true`; the
   existing "inside inserted text" wording). It is never reported as if the
   author typed the link there.
3. **Unlocated.** If the document has no body map (built in memory, or its
   frontmatter failed to extract), `line` and `column` are `null` and
   `source` still names the file. A prepared-space number is never emitted
   as an authored one. For a file-sourced document this should be
   unreachable; a `debug_assert` guards that.
4. **Frontmatter `prologue`/`epilogue`.** The location is the authored
   position of the key's value in the frontmatter, not a body position.
5. **Section.** The slug is that of the innermost heading at or before the
   reference in the prepared body, as the document's anchors name it (after
   interpolation, with the same duplicate numbering). Omitted above the
   first heading. Section is available in cases 1–3.
6. **Trail.** Each step's directive line is the authored line of the
   `::file`/`::toc-linking` opener. Openers are always authored, so the
   trail is exact whenever the parent has a map, and otherwise follows
   rule 3.

There is no "line only" tier. The map either gives both line and column or
gives neither reliably.

## Implementation tasks

| # | Task | Where | Size |
|---|---|---|---|
| 1 | Keep `body.origin` in `ComposeReport` (a `pub(crate)` field beside `body_data`) and return it from `prepare_content` | `compose/pipeline/mod.rs:323`, `compose/context/report.rs:100`, `reference/graph.rs:942–977` | S |
| 2 | Add `BodyOrigin::producer_of(offset) -> Option<Range<usize>>`, the authored gap that wrote an unmapped offset, with unit tests for interpolation, shell, and replacement | `compose/body_origin.rs` | S |
| 3 | Add an authored location to `ReferenceOrigin` (for example `authored: AuthoredLocation { line, column, inserted }` or `Unlocated`) as a **new** field. Keep `line`/`span` in prepared space, because `file_tree/model.rs:322` matches `directive_line == origin.line` in that space. Fill it in `build_node` for Markdown, HTML, transclusion, TOC-linking, and file-links records | `reference/types.rs:242`, `reference/graph.rs:286–520` | M |
| 4 | Rebase CSS record spans onto the body (add the style block's span start) | `reference/css.rs:46–57`, `graph.rs:882–900` | S |
| 5 | Locate prologue/epilogue values in the frontmatter (the existing `locate_frontmatter_value` machinery) instead of sentinels | `graph.rs:690–830` | S–M |
| 6 | Sections: keep each heading's prepared byte start in `build_heading_index` and `PreparedHeadingSnapshot`, choose the section by byte offset, and check the `Markdown::new(prepared)` frontmatter misread | `graph.rs:232–259`, `reference/snapshot.rs:32` | M |
| 7 | Trail: give `ReferenceInsertion` the authored directive line (project `directive.span.start`), and accumulate `(document, line)` steps on each `ReferenceIssue` during the validation walk | `graph.rs:463–480`, `reference/validate.rs` | M |
| 8 | Consumers: `validate refs` JSON and text, the compose diagnostic `source`, and the compose "inside inserted text" line switch to the authored location. Update the `2026-06-17-cli-atheist` baseline JSON and `docs/cli/report-json.md` | `validate.rs:298`, `cli/src/commands/compose.rs:1037`, report sink | S–M |
| 9 | L1 tests built from this spike's fixtures: frontmatter offset, same-line replacement, a 3-line interpolation, a removed block, a multibyte column, LF/CRLF/BOM parity, a data-inserted link, a link whose text contains an interpolation, a nested trail, and the same child transcluded twice | `lib/tests/l1/` | M |
| 10 | (Adjacent, same root cause) Use the map for the transclusion-stage `when=` error line, which today reports the prepared line | transclusion stage | S, optional |

DMLS is unaffected: it extracts from the raw body and shifts by the
frontmatter base itself (`dmls/src/graph/substrate.rs:225–280`).
