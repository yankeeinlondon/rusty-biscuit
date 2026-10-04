---
$schema: feature-review.yaml
ready: false
findings:
    - title: HTML-backed Markdown output does not lower its children for the enclosing context
      priority: high
    - title: Prose documentation recommends unsupported basic background tags
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-03T23:22:00-07:00
spec: 2026-10-02-inline-prose/spec.md
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
implemented: true
implemented_by: claude/opus
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-7.md
previous: 2026-10-02-inline-prose/review-6.md
next: 2026-10-02-inline-prose/review-8.md
---

# Inline Prose — Review 7

The feature is **not production ready**. Review 6's enumerated break defects are repaired. The broader context-preservation sweep found one high-priority omission: some HTML-backed Markdown paths still serialize child nodes as Markdown where the consumer expects HTML or plain text. Code containing HTML or an entity can therefore lose its literal meaning. A separate documentation finding concerns unsupported background-tag examples.

This review covers the current working tree, including the uncommitted implementation. Permanent changes are limited to this review and the requested review/spec metadata. No implementation changes, commits, or lifecycle moves were made.

## Previous review follow-through

Review 6 had one unblocked finding, **“Markdown break serialization changes meaning at boundaries and inside raw HTML,”** and no blocked findings. There was nothing to unblock before the latest implementation. Earlier process-only human-review requests do not require reaffirmation.

| Review 6 repair | Current result |
|---|---|
| Leading, trailing, solitary, and consecutive structural breaks | Retained independent-reader matrix passes for both dialects. |
| Breaks moved outside emphasis delimiters | Passes for bold, italic, strike, mark, and dim, including neighboring text. |
| Paragraph endings before another paragraph | Passes; trailing hard breaks use `<br>` rather than a visible backslash. |
| Quotes, lists, footnotes, root/sequence containers, links, and table cells | Reported structural-break shapes pass. |
| MarkdownPlus span opener followed by a soft break | Passes without turning nested bold syntax into a raw HTML block. |
| Raw HTML summary hard/soft breaks | Passes for text-only shapes; formatted children are explicitly skipped by the retained matrix and remain defective below. |
| Heading breaks and direct-child breaks discovered during repair | Covered by the new matrix; pass. |
| Earlier literal-text, opacity, destination, code-link, and terminal repairs | Retained renderable/prose suites, code-link tests, and representative real-terminal captures pass. |

The expanded temporary break probe copied the retained structural-break fixture builders and substituted `# x`, `---`, `===`, `1. x`, four-space indentation, `[x](u)`, `&copy;`, a backslash, `<x>`, `**x**`, and trailing spaces for ordinary text. It checked before/between/after-break positions across every fixture context and both dialects. No additional mismatch was printed. Its diagnostic test's successful exit is not itself proof of behavior; the retained assertion-based matrices are the passing evidence.

## Recurrence

The high finding repeats the context-dependent output-preservation class in review 6's **“Markdown break serialization changes meaning at boundaries and inside raw HTML”** and reviews 1–2's **“Markdown serialization changes literal backslashes and break meaning.”** Reviews 4–5 also found cases where serialized output reinterpreted authored content. Their specific repaired instances still pass.

The raw HTML repair should have swept all phrasing children of a disclosure summary, the HTML column bodies, and the progress route that derives its HTML label from child output. Changing only text and break spelling leaves code, links, emphasis, and other structural children using the wrong language. The latest implementation log acknowledges the summary omission but defers it as a separate class. At the serializer boundary it is the same decision: choose an encoding that the enclosing consumer actually reads. No additional product decision is needed to keep code literal or to match the existing browser's plain progress-label policy.

The background-tag documentation finding was not a finding in an earlier review. The implementation log already notes the bad `<bg-blue>` example, but a log entry does not correct the current documentation.

## Unblocked Findings

### High: HTML-backed Markdown output does not lower its children for the enclosing context

**Defect class:** a serializer embeds child output in a context that consumes a different language, so formatting becomes visible syntax, literal code becomes HTML, or a plain label contains generated markup.

In **renderable**, [the Markdown writer](../../../renderable/src/tree/render/markdown.rs) now selects raw HTML text encoding for disclosure summaries, but the `InlineCode`, `Link`, `Image`, and delimiter-wrapper routes still produce Markdown. The column writer also puts Markdown inside an opening `<div>` raw HTML block. The progress writer renders structural children before extracting its label, whereas [the browser writer](../../../renderable/src/tree/render/browser.rs) extracts their plain text.

This is reachable through **biscuit-terminal**'s public [TwoColumn](../../lib/src/components/two_column.rs), which accepts block [Prose](../../lib/src/components/prose/prose.rs):

```rust
let columns = TwoColumn::new(Prose::new("`<em>x</em>`"), Prose::new("right"));
let markdown = columns.render_markdown_plus();
// The first column contains `<div ...>` followed by `<em>x</em>`
// surrounded by backticks, rather than an escaped <code> element.
```

The emitted document begins with `<div>`, so an independent CommonMark reader passes the entire single-block column container through as raw HTML. Backticks do not protect `<em>x</em>` there: the browser sees an emphasis element and displays backticks. The same code in a disclosure summary has the same failure. An inline-code value `&copy;` is emitted inside backticks without HTML escaping and becomes a copyright character rather than the literal entity spelling. This violates the feature's code-opacity contract, beyond simply losing a style.

The public `RenderNode` reproduction is:

```rust
let node = RenderNode::disclosure(
    vec![RenderNode::inline_code("<em>x</em>")],
    vec![RenderNode::paragraph(vec![RenderNode::text("body")])],
    None,
);
// MarkdownPlus: <details><summary>`<em>x</em>`</summary> ...
// Browser: <details><summary><code>&lt;em&gt;x&lt;/em&gt;</code></summary> ...
```

The sweep copied the retained structural-break fixture builders, changed one child per case, rendered through public `render_markdown_node`, and compared the output with public `render_browser_node` plus independent CommonMark events. It covered every phrasing node category: text, strong, emphasis, delete, span, extended wrappers, inline code, link, image, footnote reference, soft break, and hard break. Code controls included HTML-looking content, an entity spelling, Markdown punctuation/link syntax, and embedded backticks. Color, underline, neutral spans, and unknown extensions tested recursive nesting. The column case was additionally reproduced through the shipped `TwoColumn` component.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| MarkdownPlus disclosure summary | Strong, emphasis, strike, link, image, footnote; code containing HTML, entities, Markdown, and backticks | Markdown syntax remains literal in the raw HTML summary. Code HTML/entity content is interpreted as HTML | Lower phrasing children to HTML; code content must be escaped and literal. |
| Summary color/underline spans, neutral spans, unknown extensions | Code nested in each wrapper | Nesting does not protect code; HTML and entities remain active | Apply the same child policy recursively. |
| Summary mark/dim extensions | One formatted text child | Extension delimiters are literal inside raw HTML | Preserve through an HTML equivalent where supported, or report explicit degradation; do not silently expose generated syntax. |
| Summary literal text and structural breaks | Literal Markdown/HTML/entity text, soft and hard break controls | Clean: text is HTML-escaped; soft break is space, hard break is `<br>` | Retain these repaired paths. |
| MarkdownPlus columns, including public `TwoColumn(Prose, Prose)` | Same phrasing/category/code matrix in a one-paragraph column | Whole container is raw HTML; Markdown formatting is literal and code HTML/entities become active | Preserve accepted block content and opaque code within the HTML container. |
| Columns with text and break controls | Literal syntax, solitary soft/hard breaks | Literal Markdown escapes appear as backslashes; HTML-looking text can become a tag. Hard `<br>` control works; a soft edge reference retains its space | Encode literal text for HTML when it is in HTML; retain working breaks. |
| MarkdownPlus progress paragraph with structured children | Same matrix followed by ` 60%` | Visible and accessible labels contain `**a**`, backticks, link/image/footnote syntax, or serialized span markup; browser labels use plain child text | Extract plain text before label generation, matching the browser. |
| Progress ordinary text and break controls | Literal markup-looking text, soft/hard break plus percentage | Clean: literal text survives and breaks become spaces under the existing label policy | Preserve the plain-label contract. |
| Ordinary paragraph; MarkdownPlus styled inline span; disclosure body after its blank-line separator | Same phrasing/category/code matrix | Standard formatting, links, and code retain their meaning; these are Markdown-parsed contexts | Keep their Markdown lowering. Mark/dim remain extension syntax, not standard CommonMark features. |
| Table caption, link/image string fields | Literal HTML control; retained literal-data matrices | Clean encoding controls; not structural-child writers | Keep existing string-field policies. |
| Plain Markdown counterparts | Retained structural/literal matrices and inspected shared branches | Use Markdown/extension syntax without adding the HTML container; not the failing raw HTML routes | Keep the plain dialect's documented degradation policies. |
| Raw `Html` node | Source inspection | Verbatim by contract | Outside this repair: do not reinterpret deliberately raw HTML as code. |

Columns and structured progress children expose older renderer limitations, rather than regressions introduced entirely by this feature. They are included because the requested sibling sweep reaches them; the column case directly affects the newly projected Prose code nodes in a specified container. Existing comments claim single-block columns are unaffected, which the one-paragraph public reproduction disproves. Review the associated comments and docs during repair.

Use a shared, context-appropriate lowering path for HTML-backed content, with correct code/text escaping, and extract plain text for progress labels. Reuse existing browser semantics where practical; avoid reparsing generated Markdown or adding per-component escaping workarounds. Keep plain Markdown and genuinely Markdown-parsed inline HTML spans on their current routes.

Retain independent-reader/public-result tests for the entire table, including nested code, both dialects, plain-label accessible text, and the public `TwoColumn` route. Remove the formatted-summary skip in [the new break matrix](../../../renderable/tests/markdown_structural_breaks.rs). The existing columns tests assert generated Markdown bytes inside HTML; that assertion cannot establish what a reader renders. Level 1 independent parsing and HTML structure checks are appropriate for this defect; it does not need keyboard injection or human approval.

### Medium: Prose documentation recommends unsupported basic background tags

**Defect class:** reference examples describe a grammar input as supported even though the public parser preserves it as literal text.

The **biscuit-terminal** [prose topic page](../../docs/components/prose.md) recommends `<bg-blue>`. The [styling skill](../../../.claude/skills/biscuit-terminal/styling.md) recommends both `<bg-red>` and `<bg-blue>`. The shared [tag resolver](../../lib/src/components/prose/tokens.rs) accepts background web colors, Tailwind names, and RGB tags; it does not accept the basic foreground color names with `bg-` prepended.

The public-API probe copied each documented tag into `<tag>x</tag>` and rendered both Prose components to HTML. It also swept all eight basic colors and their eight bright variants with the same background prefix. Every basic/bright background tag remained escaped literal text. The code is the authority here: correct the documentation rather than silently expanding the grammar.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Prose topic page, “Supported Tags” background example | `<bg-blue>x</bg-blue>`, both components | Literal escaped tag text | Recommend an accepted background spelling and state the accepted categories. |
| Styling skill, background examples | `<bg-red>`, `<bg-blue>`, both components | Both remain literal | Replace both examples with supported names or RGB forms. |
| Shared resolver sibling inputs | All eight basic and eight bright names prefixed with `bg-` | All remain literal, consistently | Documentation must not imply every foreground spelling accepts the prefix. |
| Topic/skill controls | `<bg-coral>`, `<bg-red-800>`, `<bg-rgb 255,128,0>` | Actual background CSS, both components | Retain these valid examples. |
| Topic foreground/RGB controls | `<red>`, `<blue>`, `<rgb #ff0000>` | Actual foreground CSS, both components | Retain their separate grammar. |
| CLI skill's background examples | `bg-rgb`, `bg-coral`, `bg-red-800` | Clean supported spellings | No correction needed. |
| README and remaining area topic pages | Search for background-tag examples | No additional instances | No sibling left to repair there. |

Update the topic page and styling skill together. A small public-result check for the replacement examples is sufficient; adding an accepted basic-background syntax is not required by this finding.

## Blocked Findings

None. Existing code-opacity and browser-rendering contracts determine the expected behavior. No human-only activity or new design decision blocks these repairs.

## Input Robustness Matrix

The only new load-bearing serialized field remains JSON `browser.block_element`. The production-serialization fixture matrix in [renderable's browser tests](../../../renderable/src/tree/render/browser.rs) passed in the full renderable run.

| Shape | JSON `browser.block_element`: expected and observed outcome |
|---|---|
| Positive control | All seven supported lowercase tags render their element and `data-role="note"`; pass. |
| Absent | Legacy paragraph renders `<p>`; pass. |
| Explicit null | Reject; pass. |
| Wrong whole-field type | Number rejected; pass. |
| Wrong type, one element | Mixed array rejected for the scalar enum; pass. |
| Wrong type, every element | All-invalid array rejected; pass. |
| Empty | Array, object, and string rejected; pass. |
| Duplicate key | Reject; pass. |
| Trailing or invalid content | Valid JSON plus garbage rejected; pass. |
| Invalid spelling | Unknown and uppercase names rejected; pass. |

InlineProse adds no file/configuration reader. Markup strings are grammar input rather than a new serialized configuration format. No additional load-bearing format field was identified.

## Verification

| Check | Result |
|---|---|
| Renderable `just test` | 614 passed, including retained literal encoding, structural breaks, browser/streaming behavior, validation, and JSON compatibility. |
| Biscuit-terminal `just test prose_` | 225 passed; includes the new component break matrix and earlier opacity/destination regressions. |
| Darkmatter `just test code_link` | 16 passed, including templates, public rendering, catalog binding, and DMLS completion. |
| Biscuit-terminal `just test-l2 inline_code` | Four scenarios executed in tmux/WezTerm and passed. Two Kitty scenarios returned through availability gates; no Kitty rendering evidence credited. |
| Root `just check-tier-coverage biscuit-terminal` and `renderable` | Zero stranded tests. |
| Renderable and biscuit-terminal `just lint` | Passed. |
| Temporary expanded break probe | No mismatch printed across the copied fixture contexts, two modes, and both dialects. Diagnostic output, not an assertion-based passing suite. |
| Temporary HTML-child and color probes | Reproduced both findings through public APIs. Diagnostic successful exits do not mean the defects passed. Removed after use. |

No full downstream suite, full biscuit-terminal suite, browser tier, auxiliary compilation sweep, or cross-OS run was repeated here. The previously recorded missing-table-spec guard failure is unrelated to this feature and is not a new finding. No cross-OS evidence or human sign-off was used as a readiness gate.

The new prose test module is declared by the consolidated Level 1 target. Renderable's new integration file is automatically compiled and selected by L1. The terminal tests are in the declared CLI Level 2 target, require `terminal-tests`, have a live recipe, and are enabled in CI metadata. The tier recipe owned resource setup and teardown; no foreground activation was requested. No permanent test was added or renamed during this review.

## Requirement-to-verification mapping

Criterion numbers refer to the spec's Acceptance Criteria; each row describes the requirement independently.

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, wrapper-free HTML, emptiness, relative links (1, 2, 7, 11, 25) | Level 1 public tree/HTML/Markdown tests; representative Level 2 link/break capture | Retained cases pass. |
| Paragraphs, newline modes, whitespace, escapes, normalized line endings (3–7, 26–27) | Level 1 independent-reader/component matrices; existing Level 2 geometry tests | Review 6's enumerated breaks now pass. |
| Paragraph tags, streaming parity, serialization compatibility, validation (8, 31) | Level 1 browser/streaming/validation and robustness matrix | Pass; this level is appropriate for emitted structure. |
| Fences, wrapper scope, opaque code, inline fence degradation (9, 14, 16, 18–19, 27–28, 30) | Level 1 public trees/read-back; Level 2 container capture | Core cases pass; HTML-backed Markdown code opacity has the high finding. |
| Layout exactly once and container type/projection contracts (10, 12, 31) | Level 1 component/CLI tests; existing Level 2 geometry tests | Core shape checks pass; TwoColumn's MarkdownPlus child output remains defective. |
| Migrated diagnostic rows and literal code values (13, 17, 20) | Level 1 diagnostic/snapshot tests; existing Level 2 status capture | Earlier repairs remain present; full downstream diagnostic suites were not rerun here. |
| Dim code, enclosing appearance restoration, hyperlink extent, unstyled fallback (15, 21, 29) | Level 2 rendered-cell/style/link capture; Level 1 capability checks | Current tmux/WezTerm representative scenarios pass. |
| Safe code-link labels, resolution parity, templates, catalog and completion (22–23) | Level 1 composition/public-result/binding/completion tests | 16 focused tests pass. Installed personal prompt copies remain separate manual maintenance. |
| Affected-area tests/lint and auxiliary targets (24) | Current scoped tests/lint plus implementation-log evidence | No fresh complete downstream/auxiliary proof claimed. |
| Current grammar documentation and skills | Public API probe of examples | Unsupported background examples have the medium finding. |

No keyboard, mouse, paste, or input-encoder behavior is added, so Level 3 is unnecessary. No additional terminal verification-level mismatch was found. No separate performance or ergonomics change is required for readiness.
