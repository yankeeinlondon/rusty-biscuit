---
$schema: feature-review.yaml
ready: false
findings:
    - title: Neighboring inline-code fences merge and change code contents
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-04T03:23:26-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
implemented_by: claude/opus
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-12.md
previous: 2026-10-02-inline-prose/review-11.md
next: 2026-10-02-inline-prose/review-13.md
---

# Inline Prose — Review 12

The feature is **not production ready**. Review 11's attribute and title line-ending finding is implemented across its affected sites. A further sweep found that the shared Markdown writer changes code contents when two code spans become neighbors after parsing or wrapper flattening. Both public prose components reproduce the defect, and even strict rendering accepts it without a diagnostic.

This review evaluates the current working tree. Permanent edits are limited to this review and the requested review/spec frontmatter. Temporary public-API probes were removed. No implementation repair, commit, or lifecycle move was made.

## Previous review follow-through

Review 11 had one unblocked finding and no blocked findings.

| Previous site | Result |
|---|---|
| Generated span class attributes | Implemented. CR/LF character references preserve the attribute value in cells, ordinary and section headings, and paragraphs. Plain Markdown retains its existing class-removal policy. |
| Progress accessible label and all four glyph attributes | Implemented. Generated attributes retain their values without splitting Markdown rows, headings, or inline HTML across blank lines. |
| Link and image titles | Implemented. Field-specific escaping preserves LF, CR, CRLF, repeated endings, and title values instead of inserting literal `<br>` text. |
| Clean siblings and additional repair | Text, link labels, code, destinations, raw HTML, footnotes, placeholders, and HTML-backed containers retain their policies. The additional image-alternative-text repair preserves line endings inside headings. |
| Retained verification | [The public-result matrix in renderable](../../../renderable/tests/markdown_line_endings_in_fields.rs) exercises all affected fields, ten wrapper routes, both dialects, all three strictness modes, every table position, and heading/paragraph contexts. It checks decoded values and neighboring content, not just table counts. All four tests pass in the full renderable suite. |

Every earlier review was compared. Reviews 1–11 contain no outstanding blocked code finding. Reviews 2–3 requested human attention to incomplete repair cycles, rather than a missing design choice or a human-only test. Nothing was subsequently unblocked that requires a separate repair here.

## Recurrence

The finding repeats the Markdown meaning-preservation class from reviews 1–2, **“Markdown serialization changes literal backslashes and break meaning,”** review 4, **“Markdown serialization reinterprets literal text and entity spellings,”** and review 5, **“Markdown literal encoding misses email autolinks and block syntax across text boundaries.”** Individually safe fragments become unsafe when a serializer joins them. Review 6's break-boundary finding is another instance of this sequence-level problem.

Those repairs swept text and break boundaries but did not sweep neighboring code delimiters. The original safe-code-fence implementation also tested values individually rather than composing two code values. The missing sibling sites are ordinary inline sequences, composition sequences, wrapper bodies and flattened wrappers, link labels, headings, all table positions, list/quote/footnote bodies, and the Markdown-parsed parts of disclosures and columns. Their full affected and clean list is below. The HTML-backed paths repaired in reviews 7–9 remain clean; reviews 10–11's pipe and quoted-field repairs also pass.

The diagnostic migration, opaque-input scanners, portable destinations, soft-break trimming, paragraph-tag reader, terminal-capture helper, and documented background-tag repairs were also compared with the earlier findings. No new defect in those classes was reproduced by this review's checks.

## Unblocked Findings

### High: Neighboring inline-code fences merge and change code contents

**Defect class:** a serializer fences each code value safely in isolation but concatenates neighboring fences into a different Markdown delimiter run, changing literal code contents.

In **renderable**, [the inline-code writer](../../../renderable/src/tree/render/markdown.rs:359) calls the shared fence helper separately for each value. [The inline-piece joiner](../../../renderable/src/tree/render/markdown.rs:570) appends those strings without protecting their shared backtick boundary. [Composition sequences](../../../renderable/src/tree/render/markdown.rs:843) use that same joiner. Selecting a fence longer than each value's backticks does not make two touching fences safe.

The reproduction uses the code-span and wrapper shape from [biscuit-terminal's prose grammar tests](../../lib/tests/l1/prose_grammar.rs), changing only the wrapper between code values:

```rust
Prose::new("`a`<clipboard>`b`</clipboard>").render_markdown()
InlineProse::new("`a`<clipboard>`b`</clipboard>").render_markdown()
```

Both return the following in both dialects:

```markdown
`a``b`
```

An independent CommonMark/GFM reader returns one code value **```` a``b ````**, instead of the intended neighboring values `a` and `b` with visible text `ab`. Browser output is correct: `<code>a</code><code>b</code>` (inside `<p>` for block prose). The shipped `bt prose --md` command reproduces the same incorrect Markdown.

Two other public inputs are affected:

- `` `a`<b></b>`b` ``: removing an empty wrapper exposes the same touching fences.
- `` `a`<code-block>b</code-block> ``: `InlineProse` correctly normalizes the explicit block to inline code, then loses its contents during Markdown serialization. `Prose` correctly keeps the code block separate.

Dim styling that degrades to its children also exposes the boundary. Different fence lengths do not solve it: values containing one and two backticks produce a combined five-backtick run and are read back incorrectly. Edge spaces and all-space values are also altered. Rendering a directly constructed paragraph containing `InlineCode("a")` and `InlineCode("b")` succeeds with **zero diagnostics under Strict, Warn, and Lossy**, in both dialects.

The sweep copied the two-row, two-column fixture from the retained table tests and checked **17,600 public-render cases**: 16 contexts, ten enclosing wrapper routes, both dialects, five value pairs, and eleven boundary shapes under Warn. **7,788 cases changed the combined code value.** The remaining cases are meaningful controls. The value pairs cover ordinary text, unequal backtick runs, spaces at both edges, all-space code, and empty code. Boundaries cover direct neighbors, neutral wrapping, empty strong/text/code/extension nodes, a visible space, a soft break, and a separate strong/link/class wrapper around the second code value. Markdown contexts were read with pulldown-cmark; actual HTML code elements were inspected for the HTML-backed controls.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `Prose` paragraph and `InlineProse` fragment | Transparent wrapper, empty wrapper, or dim wrapper between two code values; both dialects | Touching fences insert backticks into code. HTML retains the intended values. | Preserve the code values and visible text across targets. |
| `InlineProse` explicit-code-block normalization | Inline span followed immediately by `<code-block>` | Two correct tree values serialize as one incorrect span. | Preserve both normalized code values. |
| Ordinary shared paragraph and neutral inline span | Five value pairs and eleven boundaries | Nonempty neighboring values collide; empty-value and visible-separator controls pass. | Empty nodes must not undermine delimiter safety. |
| Root with composition/no-separator policy | Same matrix | Same collision through `render_sequence`. | Preserve the composition's no-added-whitespace contract. |
| Ordinary document root with separate code children | Same matrix | Clean when each child remains its own block. A wrapper grouping both codes creates the affected inline sequence inside that block. | Retain block separation and fix grouped inline content. |
| Ordinary heading and section heading | Same matrix | Heading stays intact, but its code contents change. | Preserve both heading membership and code values. |
| Table header, first and second columns | Same fixture and matrix, each position separately | Table structure survives, but target code contents change. | Preserve cell content as well as row structure. |
| Table body, first and second columns | Same fixture and matrix, each position separately | Same content corruption. Neighboring sentinel cells stay intact. | Same cell-content contract. |
| Strong, emphasis, and delete wrapper bodies | Same matrix inside each wrapper | Outer styling does not prevent an internal code-fence collision. | Preserve wrapper structure and its literal code values. |
| Link-label body | Same matrix inside one link | Link remains, but its code label changes. | Preserve the destination and intended label. |
| Neutral span and unknown extension bodies | Same matrix | Flattening leaves colliding fences. | Treat transparent nodes as part of the enclosing sequence. |
| Classed span body | Same matrix | Plain Markdown degradation and MarkdownPlus's Markdown-parsed span body both corrupt internal code values. | Protect code boundaries inside either representation. |
| Mark and dim extension bodies | Same matrix | Their outer delimiters do not protect neighboring code values inside. | Preserve internal code values independently of extension spelling. |
| Quote and list-item paragraphs | Same matrix | Markers/indentation survive; inline code changes. | Preserve the content inside each block. |
| Footnote-definition paragraph | Same matrix with a matching reference | Definition remains paired, but its code changes. | Retain both pairing and definition content. |
| Disclosure summary, plain Markdown | Same matrix | Shared inline writing corrupts the summary's code. | Preserve code inside the disclosure syntax. |
| Disclosure body, both dialects | Same matrix | Markdown-parsed body has the same collision. | Preserve body code. |
| Columns, plain Markdown | Same matrix | Sequential paragraph fallback has the same collision. | Preserve code in each column's content. |
| Disclosure summary and columns, MarkdownPlus | Same matrix | Clean: HTML lowering emits separate `<code>` elements with the exact values. | Retain this HTML-context policy. |
| Separate strong/link wrapper around the second code value | Same five pairs in all contexts/routes | Clean: actual intervening markup separates the fences. | Retain distinct styles and link extents during repair. |
| Class wrapper around the second value | Same pairs/routes | Clean in MarkdownPlus, where actual HTML separates the fences; plain Markdown removes the class and exposes the collision. | Make degradation feed the same safe sequence handling. |
| Empty code, visible space, or soft-break boundary | Same pairs/routes | Clean controls. Empty code contributes no fence; visible separators separate nonempty fences. | Do not add a visible separator where the input has none. |
| Block code degraded inside a cell or heading | Inline code before and after an extension containing a code block, both dialects | Clean: the existing `<br>` separators preserve all three code values. | Retain documented block degradation. |
| Shared safe-fence helper and `code_link()` | Existing isolated-value, literal-backslash, backtick, edge-space, empty-label and migrated-template tests | Clean. Complete links supply their own syntax boundaries. | Keep the helper's value normalization and destination policy. |

Repair the shared sequence handling, including boundaries exposed by flattening and degradation. Do not insert visible spaces or blindly merge nodes with different styles or links. A representation that preserves code contents and distinct styling must remain valid in both dialects. The single-value fence helper can remain shared; it needs sequence-aware use.

Retain a public-result regression matrix derived from the existing fixtures. Assert decoded code values, complete visible text, wrappers/link extent, table sentinels, and heading membership. Include adjacent values with equal and unequal fence lengths, edge/all-space values, empty nodes, transparent/style degradation, and the clean HTML/block/separator controls above. Check the strictness outcomes too: no mode should silently change literal code. Update the relevant writer comments and [renderable's Markdown documentation](../../../renderable/docs/tree-rendering.md) alongside any changed serialization policy.

This violates the feature's opaque-code and Markdown meaning-preservation contracts. Level 1 independent parsing is the correct verification level; no new design decision or keyboard test is needed.

## Blocked Findings

None.

## Input Robustness Matrix

The latest repair adds no file/configuration reader. The feature's introduced load-bearing serialized field remains JSON `browser.block_element`. [Its retained production-derived fixture matrix](../../../renderable/src/tree/render/browser.rs:4936) passes in the full renderable suite.

| Shape | JSON `browser.block_element`: expected and observed |
|---|---|
| Positive control | All seven lowercase tags render their element and retain the other attributes. |
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
| Renderable `just test` | 668 passed, zero skipped; includes all earlier literal/break/HTML/pipe matrices and the latest attribute/title matrix. |
| Renderable `just lint` | Passed. No formatter was run. |
| Biscuit-terminal `just test prose` | 447 passed; other tests excluded by the requested filter/tier. |
| Biscuit-terminal `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 prose` | CLI tier: 29 successful exits. Actual tmux, WezTerm, and Apple Terminal scenarios ran; unavailable Kitty scenarios exited through their gates and are not credited as evidence. Required-backend proof records two actual tmux executions. Library tier selected zero tests. |
| Biscuit-terminal `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 inline_code` | Six successful CLI-tier exits: four actual tmux/WezTerm executions and two unavailable-Kitty gate exits. Required-backend proof records two actual tmux executions. Library tier selected zero tests. |
| Darkmatter `just test code_link` | 16 passed, including literal values, argument/null/error behavior, and the four migrated templates through public render results. |
| Claudine `just test context_expressions_lists_code_link` | One passed through the shipped CLI. |
| Darkmatter `just test function_completion_offers_code_link` | One DMLS completion test passed. |
| Root `just check-tier-coverage biscuit-terminal` and `just check-tier-coverage renderable` | Both passed; zero stranded tests. |
| Temporary public API and independent-reader probes | Reproduced the full class sweep, both public prose entry points, strictness acceptance, and clean block-code controls; removed after use. |
| Shipped `bt prose --md` reproduction | Emits the corrupt neighboring-fence spelling shown above. |

The added retained L1 files are selected by declared targets; renderable uses automatic integration-test discovery. Biscuit-terminal's consolidated L1 and Level 2 roots declare their prose/container modules, and its `terminal-tests` feature is enabled in CI metadata and by the live tier recipe. No permanent test was added or renamed by this review. Real-terminal runs used the repository's background/detached recipe; no foreground spawn or focus request was introduced.

Full consumer suites, example/benchmark compilation, and cross-OS checks were not repeated in this review. The implementation log records broader verification and unrelated baseline failures; those claims are not substituted for this review's focused results. Cross-OS evidence and human sign-off are excluded from readiness as requested.

## Requirement-to-verification mapping

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline and block shape; empty inputs; neutral single-node projection; paragraph tags; wrapper-free fragments; fragment/streaming parity (criteria 1, 8–9, 11, 25, 31) | Level 1 public tree/HTML/serialization matrices | Passing; appropriate level for these structural contracts. |
| Paragraph boundaries, soft/hard breaks, whitespace, escaped backslashes, CRLF/CR, style scope and code opacity (criteria 2–7, 18, 26–28) | Level 1 grammar/opacity/container matrices and Level 2 prose/container row captures | Passing controls; Markdown code adjacency fails separately as reported above. |
| Layout exactly once, wrapping, code blocks as siblings, container type/shape contracts (criteria 10, 12, 27–28, 31) | Level 1 container/layout assertions; retained Level 2 prose wrapping and container/table captures | Appropriate coverage exists; selected real-terminal scenarios pass. |
| Inline-code appearance, enclosing-style restoration, hyperlink destination/extent, unstyled fallback (criteria 1, 14–16, 21, 29) | Level 1 capability/tree matrices plus Level 2 tmux/WezTerm cell-style and link captures | Appropriate terminal level exists; selected scenarios pass. |
| Literal code, safe Markdown fences, hard-break spelling, table preservation (criteria 14–19, 21, 25, 28, 30) | Level 1 public rendering and independent CommonMark/GFM readers | Individual values and earlier boundary classes pass. Neighboring code fences fail; finding above. |
| Migrated callers retain diagnostic/display lines and literal interpolated values (criteria 13, 17, 20) | Retained Level 1 diagnostic/snapshot regressions; Level 2 representative container geometry | Appropriate coverage inspected; all consumer suites were not rerun. |
| `code_link()`, argument behavior, templates, expression listing and completion (criteria 22–23) | Level 1 public expression/template results, CLI process, and LSP session | Selected tests pass. |
| Affected-area tests/lint and example/benchmark compilation (criterion 24) | Implementation-log results plus this review's selected nextest/lint checks | Scope of fresh verification is stated above; the reproduced defect is sufficient to block readiness. |
| Keyboard/input encoding | No keyboard behavior added | No Level 3 requirement. |

No separate optimization or ergonomic change is necessary for readiness beyond correcting shared code-boundary serialization.
