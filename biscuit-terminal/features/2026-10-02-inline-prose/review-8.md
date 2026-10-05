---
$schema: feature-review.yaml
ready: false
findings:
    - title: HTML block protection rewrites opaque raw HTML
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-03T23:56:22-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
implemented_by: claude/opus
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-8.md
previous: 2026-10-02-inline-prose/review-7.md
next: 2026-10-02-inline-prose/review-9.md
---

# Inline Prose — Review 8

The feature is **not production ready**. Both findings from review 7 have been implemented for their enumerated inputs. The repair introduced one remaining defect in the shared renderer: protecting an HTML container from Markdown blank-line splitting also rewrites deliberately raw HTML inside it. This can make valid JavaScript invalid and change HTML attributes.

This review covers the current working tree, including its existing uncommitted changes. Only this review and the requested previous-review/spec metadata were changed permanently. Temporary probes were removed; no implementation changes, commits, or lifecycle moves were made.

## Previous review follow-through

Review 7 had two unblocked findings and no blocked findings. Nothing needed to become unblocked before the latest implementation. All earlier reviews also declare no blocked findings. The process-only human-review requests in reviews 2–3 do not impose a new product decision here.

| Review 7 finding | Result |
|---|---|
| HTML-backed Markdown output does not lower its children for the enclosing context | Implemented for the complete reported phrasing matrix: summaries and columns use browser HTML; progress labels use plain child text. The independent-reader matrix passes, including recursively wrapped code and the formerly skipped formatted-summary breaks. The public `TwoColumn(Prose, Prose)` regressions pass. The new postprocessing of raw HTML has the finding below. |
| Prose documentation recommends unsupported basic background tags | Implemented. The topic page, styling skill, CLI README, and related examples distinguish accepted background categories. Public-result tests check documented examples in both components. The repair correctly notes that `bg-black` and `bg-white` are accepted Tailwind spellings, correcting the previous review's overly broad eight-color claim. |

## Recurrence

This finding repeats the context-dependent encoding class from review 7's **“HTML-backed Markdown output does not lower its children for the enclosing context.”** That repair swept structural phrasing children but should also have exercised raw `Html` descendants through both new HTML-lowering routes. Its direct raw-HTML control did not cover the subsequent whole-string rewrite. The implementation explicitly promises that raw HTML remains verbatim, yet applies character-reference encoding after raw and generated HTML have been concatenated.

The same broader opacity problem appeared in reviews 1–3's **“Opaque code bodies and quoted attributes are altered by preprocessing”** and review 4's **“Wrapper scanning still interprets quoted and escaped tag text.”** Those Prose parser instances remain repaired; this is a different shared-renderer site. Reviews 1–2 and 4–6's literal/break encoding findings likewise concerned encoding chosen without all of its surrounding context. Their retained regression matrices pass. No earlier diagnostic migration, destination, terminal-capture, or serialized-field finding is newly reproducible in the checks run here.

The complete affected route list is disclosure-summary HTML lowering and columns HTML lowering, including either column and recursively nested descendants. The progress plain-text route and direct MarkdownPlus raw-HTML writer do not perform this rewrite. Both affected routes and the clean siblings were swept below.

## Unblocked Findings

### High: HTML block protection rewrites opaque raw HTML

**Defect class:** a serializer applies text character-reference encoding to an already serialized HTML fragment, including regions whose consumers do not decode those references or use whitespace as syntax.

In **renderable**, [the Markdown writer's `lower_to_html`](../../../renderable/src/tree/render/markdown.rs) obtains browser HTML and then passes the entire string to `keep_html_block_open`. That helper replaces every carriage return with `&#13;` and a newline before a blank or whitespace-only line with `&#10;`. This is appropriate for escaped text inside generated `<code>` elements. It is incorrect for arbitrary raw HTML preserved by [the browser lowering helper](../../../renderable/src/tree/render/browser.rs).

The public reproduction uses the same disclosure fixture as the retained HTML-context tests, replacing its one child with raw HTML:

```rust
let node = RenderNode::disclosure(
    vec![RenderNode::html(
        "<script>const a = 1;\n\nconsole.log(a);</script>",
        false,
    )],
    vec![RenderNode::paragraph(vec![RenderNode::text("body")])],
    None,
);
```

With `render_markdown_node` and the MarkdownPlus dialect, the summary contains:

```html
<script>const a = 1;&#10;
console.log(a);</script>
```

HTML does not decode character references inside script data. An independent HTML reader extracts `const a = 1;&#10;` followed by the remaining script; Node's JavaScript compiler rejects it with `SyntaxError: Unexpected token '&'`. The original script is valid. A lone carriage return produces the same failure with `&#13;`.

Whitespace also separates unquoted attributes. Raw `<span data-a=x\rdata-b=y>` becomes `<span data-a=x&#13;data-b=y>`. An independent HTML reader now sees only `data-a`, with value `x\rdata-b=y`; the separate `data-b` attribute is lost. With a blank line instead, `data-b` survives but `data-a` gains a newline in its value.

The temporary public-API sweep tested 17 input shapes in 13 placements, for 221 cases. It reported 120 raw-source changes across eight affected placements. That count includes transformations that preserve ordinary HTML text semantics; it is **not** a claim that all 120 produce a visible defect. Script contents and attribute parsing were independently checked with Python's HTML reader, and the extracted script was compiled without execution. The probe's assertion failed as expected.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| MarkdownPlus disclosure summary | Script with blank line, whitespace-only line, or lone CR; style with blank line | References inserted into script/style source; extracted script fails compilation | Preserve opaque source or explicitly report an unsupported representation; never silently change its language. |
| Summary inside strong, classed span, link, or unknown extension | Same 17 raw-child shapes | Same rewrite after recursive lowering | Apply the same raw-content policy recursively. |
| MarkdownPlus columns: left, right, and direct block HTML child | Same 17 raw-child shapes | Same rewrite in all three placements | Preserve raw payloads independently of column position and block/inline node shape. |
| Both affected routes, raw unquoted attributes | Blank-line separator and lone-CR separator | Blank line changes the first attribute's value; CR merges two attributes into one | Preserve attribute boundaries and values. |
| Both affected routes, other opaque HTML regions | Blank lines in `xmp`, `iframe`, `noembed`, `noframes`, and comments | Raw source gains literal character-reference spellings | Do not apply ordinary text encoding inside opaque regions. Comment contents are a source-preservation control rather than a visible-text defect. |
| Both affected routes, ordinary raw HTML controls | Blank lines in ordinary span text, quoted attribute, `pre`, and `textarea` | Source changes; these contexts decode references, so the newline representation itself is semantically appropriate | Keep the distinction between reference-decoding contexts and opaque/syntactic contexts. The documented verbatim guarantee still needs to be reconciled. |
| Both affected routes, script controls | Single LF without a blank line; no line ending | Unchanged | Retain these clean cases. |
| Direct raw node, ordinary paragraph, disclosure body, classed inline span, link label | Each of the 17 shapes | Raw source remains byte-identical | Retain the separate direct raw-HTML policy. These source checks do not claim that preexisting arbitrary raw HTML renders identically in every Markdown placement. |
| MarkdownPlus progress | Shared plain-text extraction inspected; retained structured-child matrix rerun | Raw nodes contribute no label text, matching the browser; no whole-HTML rewrite | Retain the browser's plain-label policy. |
| Generated text, inline code, fenced code, formatting, links, images, and breaks | Retained HTML-context and structural-break matrices, both dialects | Pass, including code-block blank lines | Preserve Review 7's repaired behavior. |
| Plain Markdown summary/columns/progress; ordinary raw-HTML branch | Shared branches inspected and retained suites run | Do not call `keep_html_block_open`; plain Markdown retains its documented raw-HTML degradation | Do not introduce the rewrite into these siblings. |

This is a regression in code added during the latest repair, rather than a new Prose grammar requirement. The shared renderer is within this feature's changed code, and [its current documentation](../../../renderable/docs/tree-rendering.md) explicitly says raw `Html` stays verbatim. It therefore matters to readiness even though authored Prose does not itself produce script nodes.

Preserve the distinction between escaped/generated text and deliberately raw HTML before protecting blank lines. Do not repair this by substituting a different character reference throughout the final string. Where a raw payload cannot be embedded faithfully in the selected Markdown context, use the renderer's explicit unsupported/lossy handling rather than silently corrupting it. Keep the generated-code blank-line repair. Update the helper comments and documentation so their raw-content guarantee matches the implementation.

Retain a public-result regression matrix for all table rows, with independent HTML attribute/script-content checks. The current test reader treats references uniformly and does not model script/style parsing states, so extending only that simplified decoder would repeat the mistake. Level 1 parsing and script compilation are appropriate for this encoding contract; keyboard injection and human approval are unnecessary.

## Blocked Findings

None. The existing raw-HTML and explicit-degradation contracts determine the repair goal.

## Input Robustness Matrix

The only new load-bearing configuration field identified remains JSON `browser.block_element`. The production-serialized fixture matrix ran in the full renderable suite and passed.

| Shape | JSON `browser.block_element`: expected and observed |
|---|---|
| Positive control | All seven lowercase tag variants retain their element and the fixture's other attributes. |
| Absent | Legacy paragraph uses `<p>`. |
| Explicit null | Rejected. |
| Wrong whole-field type | Number rejected. |
| Wrong type, one element | Mixed array rejected for the scalar field. |
| Wrong type, every element | All-invalid array rejected. |
| Empty | Empty string, array, and object rejected. |
| Duplicate key | Rejected. |
| Trailing or invalid content | Rejected. |
| Invalid spelling | Unknown and uppercase tags rejected. |

No additional file/configuration reader was introduced by the latest repair. Markup and raw HTML are rendering inputs, not additional configuration formats.

## Verification

| Check | Result |
|---|---|
| Renderable `just test` | 618 passed, including HTML-context, literal-text, structural-break, streaming, validation, and serialized-field matrices. |
| Biscuit-terminal `just test prose_` | 228 passed, including documented-background examples and prior opacity/break/destination cases. |
| Biscuit-terminal `just test two_column_markdown_html` | Three public-container regressions passed. |
| Darkmatter `just test code_link` | 16 passed, including templates and DMLS completion. |
| Biscuit-terminal `just test-l2 inline_code` | Four scenarios executed in tmux/WezTerm and passed. Two Kitty tests returned through availability gates; their successful exits are not Kitty rendering evidence. |
| Root `just check-tier-coverage biscuit-terminal` and `renderable` | Zero stranded tests. |
| Renderable and biscuit-terminal `just lint` | Passed. |
| Temporary raw-HTML sweep | Failed as expected: 120 source changes across 221 public-rendering cases. Removed afterward. Independent script compilation and attribute parsing confirm the semantic defects described above. |

An initial temporary-probe filter selected zero tests; it was corrected to the actual test name before collecting evidence. No zero-test run is credited. Permanent new tests from the latest implementation are declared in biscuit-terminal's consolidated Level 1 target; renderable's integration files compile automatically. No permanent test was added or renamed during this review.

The terminal recipe owned pane/window setup and teardown; no foreground activation was requested. No full downstream suite, browser tier, auxiliary-target compilation sweep, or cross-OS run was repeated. Previously recorded unrelated failing guards and downstream tests were not used as new findings or readiness gates.

## Requirement-to-verification mapping

Numbers below refer to the spec's Acceptance Criteria; each row names the requirement in words.

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, wrapper-free output, emptiness, portable links (1, 2, 7, 11, 25) | Level 1 public tree/HTML/Markdown tests; representative Level 2 links/breaks | Retained checks pass. |
| Paragraphs, newline modes, whitespace, escapes, normalized line endings (3–7, 26–27) | Level 1 independent-reader/component matrices; Level 2 displayed row geometry | Pass in the checked cases. |
| Paragraph tags, streaming parity, serialization, validation (8, 31) | Level 1 emitted structure and production-fixture robustness matrix | Appropriate level; pass. |
| Fences, style scope, code opacity, inline fence degradation (9, 14, 16, 18–19, 27–28, 30) | Level 1 trees/read-back and Level 2 container capture | Typed Prose/code checks pass; the ancillary raw-HTML regression is listed above. |
| Layout exactly once and container projections (10, 12, 31) | Level 1 component/CLI tests; existing Level 2 geometry coverage | Latest public TwoColumn regressions pass. |
| Migrated diagnostic rows and literal code values (13, 17, 20) | Retained Level 1 snapshots/diagnostic tests; Level 2 status capture | Prior repairs remain present; full consumer suites were not repeated. |
| Dim code, restored enclosing styles, hyperlink extent, unstyled fallback (15, 21, 29) | Level 2 rendered cells/style/link capture; Level 1 capabilities | Current tmux/WezTerm representative scenarios pass. |
| Safe code-link labels, destination parity, templates, catalog/completion (22–23) | Level 1 public composition/rendering and completion tests | Current targeted suite passes. |
| Tests, lint, changed examples/benchmarks, consumer compilation (24) | Current scoped tests/lint and earlier implementation records | This review adds no compilation evidence for unrerun auxiliary targets or downstream suites. |

No feature requirement involves keyboard or mouse input, so Level 3 is not required. No missing verification-level finding was identified in the retained feature tests. Cross-OS evidence and human sign-off are external to this readiness assessment.
