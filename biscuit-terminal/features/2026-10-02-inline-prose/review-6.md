---
$schema: feature-review.yaml
ready: false
findings:
    - title: Markdown break serialization changes meaning at boundaries and inside raw HTML
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-03T22:41:37-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
implemented_by: claude/opus
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-6.md
previous: 2026-10-02-inline-prose/review-5.md
next: 2026-10-02-inline-prose/review-7.md
---

# Inline Prose — Review 6

The feature is **not production ready**. Review 5's enumerated literal-text defects are repaired. One high-priority finding remains: Markdown serialization does not preserve authored breaks in every context. A trailing hard break can become a visible backslash, and a leading soft break inside a styled MarkdownPlus span can make nested formatting display literally.

This review examines the current working tree, including the uncommitted implementation. Only this review and the requested spec/previous-review metadata are permanent changes. Temporary public-API probes were removed. Cross-OS receipts and human sign-off are not readiness criteria.

## Previous review follow-through

Review 5 contains one unblocked finding, **“Markdown literal encoding misses email autolinks and block syntax across text boundaries,”** and no blocked findings. There were no blocked findings to become unblocked. Reviews 2 and 3 requested human attention to incomplete repair cycles, but explicitly had no blocked code findings; review 5 had already removed that process-only request. No new product decision or human-only test is required here.

| Previous repair | Result in this review |
|---|---|
| Email autolinks whose local part starts with digits or punctuation | Fixed. The retained independent-reader matrix passes in every text context and both Markdown dialects. |
| Block markers assembled across adjacent text children and flattened wrappers | Fixed. The retained matrix covers every split position of the reported markers; additional splits of punctuation, entities, email-looking values, and longer numbered markers also pass. |
| Leading indentation and whitespace removed by a Markdown reader | Fixed for the enumerated paragraph, heading, cell, alternative-text, and container routes. Four spaces, tabs, and mixed indentation retain their literal value. |
| Further literal-text siblings found during implementation | Retained tests for table delimiter rows, trailing whitespace, heading text line feeds, and lone carriage returns pass. |
| Earlier code opacity, wrapper scope, portable destinations, diagnostic line modes, and terminal capture repairs | Relevant public prose regressions pass; the code-link/template/completion tests and representative real-terminal captures pass. |

The new shared line writer correctly accounts for adjacent **literal text**. The finding below concerns **structural break nodes** and the syntax context surrounding them, which that literal matrix does not exercise.

## Recurrence

This finding repeats the output-preservation class of reviews 1 and 2's **“Markdown serialization changes literal backslashes and break meaning.”** Reviews 4 and 5 extended that class to other ways emitted Markdown changes the tree's meaning. The literal-backslash, punctuation, entity, email, split-marker, and indentation repairs remain clean; this is a missed sibling rather than evidence that those repairs failed.

Those repairs should also have swept `SoftBreak` and `HardBreak` at the beginning and end of an inline sequence, after delimiter-edge movement, at a paragraph boundary before another block, inside a MarkdownPlus span, and inside a raw-HTML disclosure summary. The current sweep includes those positions, the shared block/container contexts, and clean link-label/table-cell controls. The heading limitation already recorded in the implementation log is listed explicitly as a related renderer limitation rather than hidden in a passing result.

## Unblocked Findings

### High: Markdown break serialization changes meaning at boundaries and inside raw HTML

**Defect class:** a serializer writes a structural break using a context-independent spelling, so a consumer changes the authored break, exposes syntax as text, or changes the surrounding content's interpretation.

In **renderable**, [the shared Markdown writer](../../../renderable/src/tree/render/markdown.rs#L374) emits a newline for every soft break and backslash-newline for every hard break outside a table. Those spellings are valid only in particular Markdown positions. [The inline sequence writer `join_pieces`](../../../renderable/src/tree/render/markdown.rs#L545), which assembles child output, does not adjust them at paragraph edges. [The span writer `render_span`](../../../renderable/src/tree/render/markdown.rs#L971), which preserves color and underline in MarkdownPlus, and [the disclosure writer `render_disclosure`](../../../renderable/src/tree/render/markdown.rs#L402), which emits a raw HTML summary, also reuse the spellings in contexts with different parsing rules.

These defects are reachable through **biscuit-terminal**'s public [InlineProse](../../lib/src/components/prose/inline_prose.rs) and [Prose](../../lib/src/components/prose/prose.rs), which parse authored rich text and render it across targets:

```rust
let value = InlineProse::new("a\n").with_line_breaks(LineBreaks::Hard);
value.render_html_fragment().render(); // "a<br>"
value.render_markdown();               // "a" + backslash + newline
// An independent CommonMark reader displays "a" + a literal backslash,
// with no hard break.

let value = Prose::new("<b>a\n</b>").with_line_breaks(LineBreaks::Hard);
// HTML: <p><strong>a<br></strong></p>
// Markdown read back: <p><strong>a</strong>\</p>

let value = Prose::new("<red>\n**a**</red>");
// HTML contains a red span with a leading space and <strong>a</strong>.
// MarkdownPlus: <span style="color: rgb(128, 0, 0)"> followed by
// a newline, then **a**</span>.
// A CommonMark reader treats this as a raw HTML block and leaves **a**
// literal instead of producing bold text. InlineProse has the same defect.
```

A following paragraph does not rescue the trailing hard break: a public tree with `Paragraph(Text("a"), HardBreak)` followed by `Paragraph(Text("next"))` serializes to `a` + backslash + three newlines + `next`; the reader retains the backslash instead of the authored break. Adding a newline at the end of the output is therefore insufficient.

Soft breaks at sequence edges also lose their visible space. `InlineProse::new("\na")` produces HTML ` a`, but its Markdown reads as `a`. A soft break before bold text inside a color, underline, or classed span can activate CommonMark's raw HTML block rule. A hard break in a MarkdownPlus disclosure summary becomes a literal backslash plus HTML whitespace, because Markdown escapes are not interpreted inside that raw HTML block.

The probe copied the retained [literal-text fixture builders](../../../renderable/tests/markdown_literal_text.rs), changed the value to a neutral span containing structural break nodes, and rendered through `render_markdown_node`. Both dialects were read with `pulldown-cmark`; `<br>` was interpreted as a hard break and the documented table soft-break-to-space rule was honored. Additional public component probes compared emitted HTML, Markdown reader events, and independently generated HTML. The component wrapper sweep used bold, italic, strikethrough, color, underline, dim, mark, clipboard, and explicit links.

The complete break shapes were: a break between `a` and `b`; before `a`; after `a`; a break alone; two trailing breaks; two hard breaks between words; an explicit backslash-newline; and a leading soft break before nested bold text. Both soft and hard modes were exercised on the public components. Direct trees additionally exercised two consecutive soft-break nodes, which the prose grammar normally collapses or splits into paragraphs. End-of-paragraph cases were repeated before a following paragraph. Ordinary text, literal backslashes, and code remained controls.

In the table, **edge** means leading/trailing/only-break values, **interior** means breaks between words, and **raw HTML opener** means a leading soft break before nested formatting. The expected result is the break's visible meaning and the requested structure, allowing the existing deliberate movement of emphasis-edge whitespace/breaks outside delimiters and table soft breaks becoming spaces.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `InlineProse`, both dialects | All component shapes, with and without the nine wrapper categories | Trailing hard breaks become backslashes; leading/trailing soft breaks disappear; interior breaks pass | Preserve hard breaks and soft-break spaces. |
| `Prose`, both dialects | Breaks inside each wrapper; bare input controls | Breaks retained inside tags reach the defective writer. Bare leading/trailing blank lines are intentionally removed and are clean controls | Preserve every break still present in the public tree; keep intentional block trimming. |
| Paragraph; paragraph after text | Edge and interior | Trailing hard break becomes backslash; edge soft breaks disappear; interior controls pass | Preserve authored content and paragraph shape. |
| Root phrasing; sequence container; sequence after text | Edge and interior | Same edge failures as paragraph; interior controls pass | Preserve the sequence's visible break meaning. |
| After a soft break; after a hard break; break following a newline in a text value | Edge and interior | Last hard break becomes backslash; edge/consecutive soft breaks can disappear or end the paragraph; interior controls pass | Preserve only the authored breaks and blocks. |
| Paragraph before a following paragraph | Trailing hard/soft, leading/interior controls | Extra block-separator newlines do not preserve the trailing hard break; soft edge is lost | Preserve the first paragraph's authored ending. |
| Block quote | Edge and interior | Trailing hard break becomes backslash; edge soft breaks disappear. `prefix_lines` also drops the final newline | Preserve the requested quote and its content. |
| Unordered list paragraph; ordered item holding bare phrasing | Edge and interior | Same edge failures; interior breaks pass | Preserve the requested item and breaks. |
| Footnote body | Edge and interior | Trailing hard break becomes backslash; a leading soft break can move content outside the definition | Keep the body and breaks inside the definition. |
| Bold, italic, strike at sequence edges | Edge and interior | Delimiter-edge movement leaves a trailing hard break at paragraph end; it becomes a backslash. Edge soft breaks disappear | Preserve break meaning after the deliberate delimiter-edge movement. |
| Bold with ordinary text on both sides | Same shapes | Single edge breaks now have following content and pass; two adjacent soft breaks create a paragraph boundary in direct-tree probes | Preserve the authored inline sequence. |
| Mark and dim extended wrappers | Same shapes | Output puts trailing hard breaks outside delimiters and they become backslashes; leading soft breaks disappear | Preserve breaks; extension delimiter semantics remain the extension reader's responsibility. |
| Color/underline span, plain Markdown | Edge and interior | Flattening exposes the same paragraph-edge failures | Preserve break meaning after style degradation. |
| Color/underline/classed span, MarkdownPlus | Edge and interior; raw HTML opener | Hard breaks and most single soft breaks remain valid because a closing tag follows. A leading soft break can open a raw HTML block and leave nested bold syntax literal | Preserve formatting and soft-break meaning without activating a raw HTML block. |
| Styled span after ordinary text | Same shapes | Plain Markdown still loses trailing breaks; single breaks in MarkdownPlus pass. Two direct-tree soft breaks can create a raw HTML block | Preserve content in both dialects. |
| Transparent span; unknown extended wrapper | Edge and interior | Same failures as ordinary paragraph text | Flattening must preserve break meaning. |
| Link label | Edge and interior | Single leading/trailing hard and soft breaks pass because the closing bracket follows. Two consecutive direct-tree soft breaks invalidate the link spelling | Keep the requested link and its label. |
| Table cell; bold inside a cell | Every structural shape | Clean: hard breaks are `<br>`; soft breaks are spaces, including encoded edge spaces | Retain these clean row-safe paths. |
| Disclosure body, both dialects | Edge and interior | Paragraph-edge failures; interior controls pass | Preserve the body before the closing disclosure syntax. |
| Disclosure summary, plain Markdown | Leading/trailing/interior hard and soft | Single interior breaks read as breaks; trailing or leading soft breaks introduce blank lines; a trailing hard break before the separator loses its meaning | Preserve the summary's authored content under its existing output format. |
| Disclosure summary, MarkdownPlus | Leading/trailing/interior hard and soft | Hard breaks emit literal backslash-newline inside raw HTML, even between words; soft newlines act as HTML whitespace | Emit an HTML break for a hard break in raw HTML; retain soft whitespace. |
| Heading text containing structural break nodes | Every structural shape | All break positions terminate the ATX heading or lose the break; hard breaks can expose a backslash | Related pre-existing renderer limitation already recorded in the log; handle through explicit degradation rather than silently claiming faithful output. Prose does not author heading nodes. |
| Image alternatives, captions, titles, destinations; inline and fenced code | Retained literal-data and code matrices | Clean repaired controls; these string fields/code values do not contain structural break children | Keep existing normalization/trim/code policies; do not treat code contents as breaks. |
| Raw `Html` nodes | Inspection control | Verbatim by contract | Outside the structural-break encoding repair. |

The prose containers delegate to this shared writer; they do not need local escaping workarounds. The paragraph, quote, list, table, and sequence reproductions exercise their shared output shapes. The helper `renderable::markdown::escape_text` encodes literal text rather than structural break nodes and passes the extended literal-boundary matrix.

Repair break spelling with awareness of the surrounding syntax and paragraph position. Keep the specified backslash-newline form where it actually encodes a hard break, and handle positions with no faithful spelling explicitly. Raw HTML requires HTML break handling; a span opener followed immediately by a newline must not accidentally disable parsing of its Markdown children. Record any necessary boundary fallback departure in the implementation log and current docs, while leaving the historical spec intact. Do not erase an authored break or add visible filler to make a byte-level assertion pass.

Retain a public result matrix for every row above. Read Markdown back independently, assert visible text, breaks, and structure, and normalize deliberate `<br>`/table-space lowering. Include component inputs, both dialects, all wrapper categories, paragraph endings before another block, and the raw HTML summary route. Ordinary interior-break tests cannot catch these failures. Direct-tree-only consecutive soft breaks and the known heading limitation should be tested or explicitly reported through the renderer's degradation policy, rather than confused with prose grammar behavior.

## Blocked Findings

None. The existing contract requires Markdown output to preserve parsed meaning. No human-only activity blocks the shared renderer repair.

## Input Robustness Matrix

The only new load-bearing serialized field is JSON `browser.block_element`. The retained [compatibility matrix in renderable's browser tests](../../../renderable/src/tree/render/browser.rs#L4880) starts from production serialization, renders accepted values through the public HTML result, and rejects malformed shapes. It passed in the 609-test run.

| Shape | JSON `browser.block_element`: defined outcome and observed result |
|---|---|
| Positive control | All seven supported lowercase tags render the named element with the fixture's `data-role="note"`; pass. |
| Absent | Legacy paragraph renders `<p>`; pass. |
| Explicit null | Reject; pass. |
| Wrong whole-field type | Number rejected; pass. |
| Wrong type, one element | Mixed array rejected because this is a scalar field; pass. |
| Wrong type, every element | All-invalid array rejected; pass. |
| Empty | Array, object, and string each rejected; pass. |
| Duplicate key | Duplicate field rejected; pass. |
| Trailing content | Valid JSON followed by garbage rejected; pass. |
| Invalid spelling | Unknown and uppercase tag names rejected; pass. |

`InlineProse` adds no deserialization format. The status line-break setting is skipped during serde input. Markup strings are grammar input, not a new file/configuration format. No additional new load-bearing configuration field was found.

## Verification

| Check | Result |
|---|---|
| Renderable `just test` | 609 passed, including the repaired literal matrix, delimiter edges, browser/streaming behavior, validation, and JSON compatibility. |
| Biscuit-terminal `just test prose_` | 223 passed. |
| Darkmatter `just test code_link` | 16 passed, including templates, public rendering, runtime binding, and DMLS completion. |
| Biscuit-terminal `just test-l2 inline_code` | Four scenarios executed and passed in tmux/WezTerm. Two Kitty scenarios returned through availability gates; no Kitty evidence credited. The library portion selected no tests; the CLI portion ran the six named scenarios. |
| Root `just check-tier-coverage biscuit-terminal` and `renderable` | Zero stranded tests in either area. |
| Renderable and biscuit-terminal `just lint` | Both passed. |
| Extended copied literal fixture matrix | Passed across the 29 retained text contexts and both dialects, including every split position of the additional single-line values. |
| Temporary public component and structural-tree probes | Reproduced the finding across the instance table. Diagnostic sweep tests printed all mismatches instead of asserting success, so their process exit is **not** passing behavior evidence. Removed after execution. |
| Biscuit-terminal full `just test` | Stopped at the previously recorded `layout_matrix::warning_layout_matrix_snapshots_disabled_pending_table_width_contract`: 2,210 passed, one failed, 57 skipped; 1,324 tests not run after cancellation. |

The full-area failure checks for a missing table-width specification and is unrelated to this feature's changed behavior. It is not a new finding or evidence that the newly passing prose tests failed. Full downstream suites, auxiliary target compilation, browser-tier execution, and cross-OS runs were not repeated in this review. Two attempted filtered broad reruns failed at command/recipe argument parsing and produced no test evidence; counts above exclude them. One temporary example initially needed the independent reader's optional HTML feature enabled; its successful run used that feature explicitly.

The prose test modules are compiled by the declared consolidated Level 1 target. The terminal scenarios are compiled by the CLI's declared `level2` target, require `terminal-tests`, have a live tier recipe, and have that feature in CI metadata. The harness recipe used detached/background resources; no focus activation was requested. No permanent test was added or renamed by this review.

## Requirement-to-verification mapping

Criterion numbers below refer to the spec's **Acceptance Criteria**; each row states the behavior so the numbers are optional navigation aids.

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, wrapper-free inline HTML, empty content, relative links (1, 2, 7, 11, 25) | Level 1 public tree/HTML/Markdown tests; representative Level 2 link/break capture | Retained cases pass; edge break serialization has the finding above. |
| Paragraph boundaries, soft/hard modes, whitespace, escaping, normalized line endings (3–7, 26–27) | Level 1 grammar and independent-reader matrices; Level 2 prose/container geometry | Interior cases pass; edge and raw HTML contexts are incomplete. |
| Paragraph tags, streaming parity, legacy JSON, invalid placements (8, 31) | Level 1 browser/streaming/validation and JSON matrix | Pass; in-process checks are appropriate for emitted structure and serialization. |
| Fenced blocks, scope across paragraphs, opaque code spans, inline fence degradation (9, 14, 16, 18–19, 27–28, 30) | Level 1 public trees/read-back; Level 2 container capture | Retained cases pass. Shared hard-break output needs the context correction above. |
| Layout exactly once, container content types, valid embedded trees (10, 12, 31) | Level 1 component/CLI tests and existing Level 2 geometry suites | Appropriate verification levels present; focused regressions pass. No new browser-tier execution claimed. |
| Migrated diagnostic rows and literal code values (13, 17, 20) | Level 1 diagnostic/snapshot regressions; existing Level 2 status capture | Earlier repairs remain present; downstream diagnostic suites were not rerun here. |
| Dim code, enclosing color restoration, hyperlink extent, unstyled fallback (15, 21, 29) | Level 2 tmux/WezTerm rendered-cell/style/link tests; Level 1 capability tests | Current representative real-terminal runs pass. |
| Safe code-link labels, matching resolution, templates, catalog and completion (22–23) | Level 1 composition/public-result/binding/completion tests | 16 focused tests pass. Installed personal prompt copies remain the spec's separate manual maintenance item. |
| Affected-area tests/lint and auxiliary compilation (24) | Current scoped tests/lint plus broader implementation-log evidence | The full biscuit-terminal run encounters the unrelated existing guard failure; no complete fresh downstream/auxiliary run claimed. |

No keyboard, mouse, paste, or input-encoder behavior is added, so Level 3 is not required. No additional terminal verification-level mismatch was found. No performance or ergonomics change beyond the shared serialization repair is required for readiness.
