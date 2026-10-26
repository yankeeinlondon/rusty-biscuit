---
$schema: feature-review.yaml
ready: false
findings:
    - title: Markdown line-ending protection misses generated attributes and link/image titles
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-04T02:58:05-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
implemented_by: claude/opus
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-11.md
previous: 2026-10-02-inline-prose/review-10.md
next: 2026-10-02-inline-prose/review-12.md
---

# Inline Prose — Review 11

The feature is **not production ready**. Review 10's three pipe-escaping sites are repaired. A related sweep found that generated class attributes, progress attributes, and link/image titles still mishandle line endings. Tables can split or disappear, headings can end early, and otherwise intact cells can silently change title values.

This review evaluates the current working tree, including the existing implementation changes. Permanent edits are limited to this review and the requested previous-review/spec metadata. Temporary public-API probes were removed. No implementation repair, commit, or lifecycle move was made.

## Previous review follow-through

Review 10 had one unblocked finding and no blocked findings.

| Previous finding/site | Result |
|---|---|
| Raw HTML containing pipes in table cells | Implemented. The retained every-split, ten-wrapper, header/body, either-column matrix passes in both dialects. |
| Generated span classes containing pipes | Implemented. Attribute quoting is escaped and cell pipes use character references; retained regressions pass. |
| Footnote reference identifiers containing pipes | Implemented. References remain paired with their definitions; retained regressions pass. |
| Required sibling sweep | Expanded during repair to blocks in cells/headings, unsupported placeholders, unrepresentable footnote labels, and definition continuation indentation. Those retained tests pass. The line-ending siblings below remain incomplete. |

Reviews 1–9 contain no outstanding blocked code findings. Reviews 2–3 requested human attention to incomplete repair cycles, rather than a new design decision or a human-only test. Nothing was subsequently unblocked that requires a separate implementation here.

## Recurrence

This finding repeats review 10's **“Markdown table escaping misses raw HTML, span classes, and footnote identifiers”**: an authored field bypasses the encoding required by its enclosing Markdown container. It also repeats the line-ending preservation class from review 9's **“HTML block protection mistakes a CRLF split across raw nodes for a blank line”** and review 6's **“Markdown break serialization changes meaning at boundaries and inside raw HTML.”** Review 7's HTML-context finding and reviews 1–5's literal/break serialization findings are related earlier instances of context-dependent output preservation.

The previous fixes should have checked both delimiter characters and line endings in generated class attributes, progress labels and all four progress glyph attributes, link/image titles, destinations, labels, code, raw HTML, footnote identifiers, and unsupported placeholders. Protecting pipes in the first two generated-HTML routes did not protect their line endings; using the text-cell escape for titles also missed lone CR and changed LF into literal `<br>` text. The complete affected list and clean sibling controls are carried below.

The findings of every earlier review were compared. No additional recurrence was reproduced in diagnostic migration, code opacity, wrapper scanning, portable prose destinations, soft-break trimming, paragraph-tag deserialization, or documented background tags. Relevant retained tests remain green; this does not claim that every consumer suite was rerun.

## Unblocked Findings

### High: Markdown line-ending protection misses generated attributes and link/image titles

**Defect class:** a serializer applies text or HTML quoting rules to authored fields without also preserving the enclosing Markdown line and the field's own meaning.

In **renderable**, [the Markdown writer](../../../renderable/src/tree/render/markdown.rs:243) generates progress HTML, and [its span writer](../../../renderable/src/tree/render/markdown.rs:1193) generates class attributes. Their cell helper, [escape_cell_attribute](../../../renderable/src/tree/render/markdown.rs:2470), only protects pipes. HTML attribute quoting does not prevent LF or CR from ending a Markdown table row or heading.

The same package's [link_target](../../../renderable/src/tree/render/markdown.rs:1334) sends link and image titles through the text-cell escape. That escape converts LF/CRLF into `<br>` but leaves lone CR. A title is an attribute value: `<br>` there is literal tooltip text, rather than a displayed break. Heading titles receive no line-ending protection.

The reproduction copies the retained two-row, two-column fixture in [the pipe regressions](../../../renderable/tests/markdown_table_cell_pipes.rs:134), changing one target cell to:

```rust
RenderNode::span(vec!["a\nb".into()], vec![RenderNode::text("t")])
```

The public `render_markdown_node` API with MarkdownPlus and `Strict` succeeds without a diagnostic and emits:

```markdown
| <span class="a
b">t</span> | H2 |
| --- | --- |
| y | z |
```

An independent GFM reader no longer finds the intended table. The same value in a heading emits `## <span class="a` on its first line; the intended text `t` leaves the heading. A progress paragraph with label `a\nb 60%` reproduces the cell failure through its generated `aria-label`, even though its visible label has been made single-line.

A separate title reproduction uses `RenderNode::link("https://x.test", Some("a\nb".into()), vec![RenderNode::text("t")])`. The cell remains structurally intact, but the independent reader returns title **`a<br>b`**, with no diagnostic under `Strict`. Images behave identically. Changing the title to `a\rb` breaks the table instead. Counting cells alone therefore cannot verify this contract.

The primary field sweep exercised **2,856 cases**: 17 writer fields/routes, a positive control plus LF, lone CR, CRLF, and repeated forms of each, both dialects, all three strictness modes, and all four header/body/column positions. It recorded **468 table-structure failures**; this is a lower bound because that count excludes retained-table value changes. Every positive control retained the table. A **2,520-case** extension checked class attributes and both title fields through ten wrappers in cells, ordinary headings, and section headings. A **490-case** context probe checked progress fields, ordinary paragraphs, disclosure summaries, and columns. A **112-case** title probe read the independent reader's actual title values.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Generated span class attribute | LF, CR, CRLF, repeated endings; table header/body, either column; ten wrappers | MarkdownPlus accepts all strictness modes but can destroy or split the table. Plain Markdown's existing class-removal policy keeps it intact. | Encode attribute characters for a single Markdown row; retain plain-Markdown degradation. |
| Same class attribute in ordinary/section headings | Same shapes and wrappers | MarkdownPlus heading ends at the first line ending; intended text leaves the heading. | Preserve the complete heading and attribute meaning. |
| Same class attribute in an ordinary paragraph | Blank LF/CR/CRLF lines | Generated tag becomes literal text across separate paragraphs. | Keep the generated inline element intact; encode blank lines inside the attribute. |
| Progress accessible label (`aria-label`) | Label text `a<ending>b 60%`; all six endings, table positions | MarkdownPlus succeeds under `Strict`, `Warn`, and `Lossy`, but literal endings in the attribute break rows. Plain Markdown's fallback preserves row structure. | Keep label semantics and the cell's intended position. |
| Progress `fill_char`, `empty_char`, `left_bracket`, `right_bracket` | Each field independently set to LF or CR | All four generated `data-*` attributes break MarkdownPlus rows in all positions and strictness modes. | Encode the character in the attribute or explicitly reject it under a documented contract; never silently break the row. |
| Progress label and all four glyph fields carried into an ordinary/section heading by an inline extension | Same line-ending shapes | `Warn` reports block flattening but still emits a literal ending that splits the heading. | The documented one-line degradation must actually stay on the heading's line. |
| Generated progress HTML in ordinary paragraphs | Blank endings in accessible label | A blank attribute line interrupts inline HTML parsing. | Keep generated attribute syntax intact. |
| Link title | LF/CRLF and repeated forms; both dialects, all table positions | LF becomes literal `<br>` in the parsed title. Lone/repeated CR can break the row. `Strict` reports no loss. | Preserve the title value with field-specific encoding, or explicitly report an unavoidable loss. |
| Image title | Same matrix | Same value changes and CR failures as link titles. | Same title policy as links. |
| Both title fields in ordinary/section headings, through ten wrappers | All six ending shapes, both dialects | Literal endings terminate the heading and expose the remaining link/image syntax as paragraph content. | Keep title and link/image inside the intended heading. |
| Ordinary text and link label | Same six endings, all table positions/dialects/modes | Table structure remains intact. | Retain their text/break policy. |
| Inline code | Same matrix | Row structure remains intact; code-span helper normalizes line endings to spaces. | Retain specified code normalization. |
| Link/image destinations; image alternative text | Same matrix | Row structure remains intact under their existing destination/text policy. | Retain it; do not apply title or attribute encodings indiscriminately. |
| Raw HTML payload | Quoted attribute containing each ending, all table positions/modes | `Strict` rejects; `Warn`/`Lossy` protect the row under the existing raw-content policy. | Retain explicit treatment of unrepresentable raw bytes. |
| Footnote reference identifier | All six endings, table positions | Strict rejection or documented label degradation; no row corruption. | Retain matching reference/definition spelling and loss reporting. |
| Unsupported placeholder label | All six endings, table positions | Strict rejection, safe Warn comment, or Lossy omission; no row corruption. | Retain comment-specific protection. |
| Generated attributes inside MarkdownPlus disclosure summaries and columns | Class/label blank-line shapes and progress glyph endings | HTML-block protection keeps the container open and the following paragraph outside it. | Keep this clean path; its multiline HTML-block policy differs from a cell or heading. |
| Generated CSS and numeric progress attributes | Writer routes inspected | Typed CSS/numeric generation supplies no authored CR/LF. | No speculative escaping or new validation needed. |
| Code language/metadata and other block content in cells/headings | Writer inspected; retained block matrix rerun | One-line degradation drops code info strings and reports block loss. | Retain this policy while repairing generated attributes that survive degradation. |

For raw HTML, do not replace arbitrary bytes using the policy for generated HTML. For generated quoted attributes, character references can protect line endings without introducing new markup. Titles need their own encoding; inserting `<br>` changes the value. Apply protection to cells and headings, and prevent blank lines inside generated inline attributes from breaking ordinary paragraphs. If a faithful representation is unavailable, use the established Strict/Warn/Lossy policy.

Retain a public-result matrix derived from the existing fixture, with positive controls and one field change per case. Assert table structure, neighboring sentinels, target content, parsed title values, and heading membership. Cover all affected fields and wrappers above together. Update the writer's comments and [renderable's rendering documentation](../../../renderable/docs/tree-rendering.md) where the field policies change.

These are shared-renderer defects in code changed during this feature, rather than additional prose grammar requirements. The latest class/progress pipe repair exposed the missed line-ending siblings. Level 1 independent parsing is the appropriate verification level; no keyboard injection or new design decision is required.

## Blocked Findings

None.

## Input Robustness Matrix

No new format/configuration reader was added by the latest repair. The feature's introduced load-bearing serialized field remains JSON `browser.block_element`. Its retained test derives cases from a production-serialized paragraph and asserts browser output or public deserialization rejection. It passes in the full renderable suite.

| Shape | JSON `browser.block_element`: expected and observed |
|---|---|
| Positive control | All seven lowercase tags render their element and retain other attributes. |
| Absent | Legacy paragraph renders `<p>`. |
| Explicit null | Rejected. |
| Wrong whole-field type | Number rejected. |
| Wrong type, one element | Mixed array rejected for this scalar field. |
| Wrong type, every element | All-invalid array rejected. |
| Empty | Empty string, array, and object rejected. |
| Duplicate key | Rejected. |
| Trailing or invalid content | Rejected. |
| Invalid spelling | Unknown and uppercase names rejected. |

## Verification

| Check | Result |
|---|---|
| Renderable `just test`, before temporary probes | 664 passed. Includes previous repairs, pipe/block/footnote matrices, raw/generated boundaries, paragraph tags, streaming parity, and literal/break serialization. |
| Biscuit-terminal `just test prose_` | 228 passed. |
| Darkmatter `just test code_link` | 16 passed, including template rendering and DMLS completion. |
| Root tier coverage for renderable and biscuit-terminal | Zero stranded tests. |
| Renderable and biscuit-terminal `just lint`, after probe removal | Passed. No formatting edits were made. |
| Temporary field/wrapper/context/title probes | Reproduced the finding through the public API and independent GFM parsing; detailed scope above. |
| Biscuit-terminal `just test-l2 inline_code` | tmux container scenario passed. WezTerm container scenario failed because Bash emitted `child setpgid: Operation not permitted` between capture markers; the rendered list row was correct. Fail-fast left three scenarios unrun. Kitty availability exit supplies no Kitty evidence. |
| Biscuit-terminal `just test-l2 prose_inline_code` | tmux prose scenario passed. WezTerm prose scenario failed on the same Bash diagnostic adding an output row. Kitty availability exit supplies no Kitty evidence. |

The WezTerm host-shell failure also appeared in review 10. It is disclosed as incomplete current terminal evidence, rather than attributed to the prose renderer or hidden by a passing retry. Representative tmux captures and retained Level 2 assertions remain available. Recipes owned terminal setup and cleanup; no foreground activation or OS keyboard injection was requested.

Initial temporary-probe runs had a root-recipe argument error, a relative-path error, and probe-only constructor/control-fixture errors. Those supplied no implementation evidence; the final matrices above ran after correction. No permanent test was added or renamed. Renderable discovers its integration targets automatically; biscuit-terminal's relevant L1/L2 modules are declared in consolidated targets, and L2 has a live feature-enabled recipe.

Full downstream suites, browser-resource tests, auxiliary-target compilation, and cross-OS runs were not repeated. Earlier unrelated failures are not newly attributed to this feature. Cross-OS proof and human sign-off are external to this readiness decision.

## Requirement-to-verification mapping

Numbers identify the spec's Acceptance Criteria; descriptions state the behavior.

| Requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, wrapper-free output, links, empty input (1–2, 7, 11, 25) | Level 1 public tree/HTML/Markdown; representative Level 2 display | Targeted checks pass. |
| Paragraphs, break modes, whitespace, line endings, style scope (3–7, 26–27) | Level 1 grammar/read-back matrices; retained Level 2 geometry | Prose grammar checks pass; shared field-serialization defect is above. |
| Paragraph tags, validation, JSON compatibility, streaming parity (8, 31) | Level 1 public-result fixture matrix | Appropriate level; pass. |
| Opaque code, safe fences, fenced-to-inline normalization (9, 14, 16, 18–19, 27–28, 30) | Level 1 projection/read-back; Level 2 container capture | Code checks pass; ancillary attribute/title routes remain defective. |
| Layout exactly once and structural embedding (10, 12, 31) | Level 1 container/CLI checks; representative Level 2 geometry | Relevant targeted checks pass. |
| Migrated line structure, diagnostics, literal code values (13, 17, 20) | Retained Level 1 regressions/snapshots; Level 2 container checks | Relevant coverage remains present; complete consumer suites not rerun. |
| Dim code, style restoration, hyperlink extent, unstyled fallback (15, 21, 29) | Level 1 capability tests; Level 2 cell/style/link capture | Current tmux scenarios pass; WezTerm evidence is limited by the disclosed host-shell failure. |
| `code_link` labels/destinations, templates, catalog/completion (22–23) | Level 1 public composition/rendering and completion | Targeted suite passes. |
| Affected tests/lint and auxiliary/consumer compilation (24) | Current scoped checks plus earlier implementation records | No new claim for suites/targets not rerun. |

No requirement handles keyboard or mouse input, so Level 3 is unnecessary. No additional requirement-to-test-level mismatch was identified.
