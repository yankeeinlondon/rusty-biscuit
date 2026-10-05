---
$schema: feature-review.yaml
ready: false
findings:
    - title: Generated code-span separators become visible in repository consumers
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-04T03:55:39-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
implemented_by: claude/opus
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-13.md
previous: 2026-10-02-inline-prose/review-12.md
next: 2026-10-02-inline-prose/review-14.md
---

# Inline Prose — Review 13

The feature is **not production ready**. Review 12's neighboring-code repair preserves code values for a standard Markdown reader. Its new invisible HTML separator, however, becomes visible text in the repository's terminal and browser consumers. Strict terminal rendering rejects the serialized content altogether.

This review evaluates the working tree. Permanent changes are limited to this review and the requested metadata. Temporary public-API probes were removed; no implementation repair, formatter, commit, or lifecycle move was made.

## Previous review follow-through

Review 12 had one unblocked finding and no blocked findings.

| Previous requirement | Result |
|---|---|
| Protect neighboring code fences, including boundaries exposed by empty nodes, flattening, and style degradation | Implemented in renderable's shared sequence writer. Both public prose components now emit `` `a`<!-- -->`b` `` for the original reproduction. |
| Sweep paragraphs, compositions, roots, headings, all table positions, wrapper/link bodies, quotes, lists, footnotes, disclosures, and columns | Implemented. The retained independent-reader matrix checks these contexts, eleven wrapper routes, thirteen boundaries, six value pairs, both dialects, and all strictness modes. It passes. |
| Preserve clean separator, empty-value, HTML-backed and block-degradation controls | Passing retained controls. The additional empty-code whitespace repair is exercised by the matrix. |
| Retain public prose regressions and document the sequence policy | Implemented. [The declared prose regressions](../../lib/tests/l1/prose_markdown_code_neighbors.rs) and [the shared-writer matrix](../../../renderable/tests/markdown_adjacent_code_spans.rs) pass. The documentation describes the inserted comment. |
| Preserve meaning through repository consumers | Incomplete. Tests read Markdown with pulldown-cmark but discard its HTML events instead of rendering those events through the shipped consumers. The finding below covers this missing integration. |

I compared the findings and blocked sections of reviews 1–12. No blocked code finding was subsequently unblocked. Reviews 2–3 requested human attention to repair completeness, not an unresolved design choice or a human-only activity. Those requests do not block this repair.

## Recurrence

This repeats review 12's **“Neighboring inline-code fences merge and change code contents”** at the broader class level: serialized inline content must preserve its visible meaning across consumers. Reviews 1–2's **“Markdown serialization changes literal backslashes and break meaning”** and review 4's **“Markdown serialization reinterprets literal text and entity spellings”** are earlier instances of that class.

Review 12's repair swept the writer's sequence sites but stopped at an independent Markdown reader. It should also have swept darkmatter's event-to-tree fold and terminal/browser entry points, the shared raw-HTML rendering policies, and both prose readers of their own emitted subset. The complete affected and clean consumer list is below. The original touching-fence defect is repaired; this finding concerns the added representation's integration.

## Unblocked Findings

### High: Generated code-span separators become visible in repository consumers

**Defect class:** a serializer adds ostensibly invisible structural markup without ensuring that the supported consumers interpret it structurally, so serialization introduces visible text or a render failure.

In the **renderable** package, [the code-sequence writer](../../../renderable/src/tree/render/markdown.rs:1496) inserts `<!-- -->` between touching code fences. In **darkmatter**, [the Markdown event fold](../../../darkmatter/lib/src/markdown/render_tree/fold.rs:211) converts that comment into an ordinary raw-HTML node. In **biscuit-terminal**, [the terminal raw-HTML renderer](../../lib/src/render_tree/render.rs:2145) prints it under Warn and rejects it under Strict. In **renderable**, the default browser policy escapes it into visible text. Both **biscuit-terminal** prose parsers also retain the comment as literal text.

The reproduction changes no fixture values: it uses the public prose input from review 12 and the retained regression test:

```rust
let markdown = Prose::new("`a`<clipboard>`b`</clipboard>").render_markdown();
// markdown is "`a`<!-- -->`b`"
let terminal = Markdown::new(&markdown).as_terminal(TerminalOptions::default())?;
let html = Markdown::new(&markdown).as_html(HtmlOptions::default())?;
```

Here `Prose` is biscuit-terminal's block component, and `Markdown` is darkmatter's document renderer. Terminal output contains the visible string `<!-- -->` between `a` and `b`; darkmatter's default HTML body is:

```html
<p><code>a</code>&lt;!-- --&gt;<code>b</code></p>
```

The browser therefore displays `a<!-- -->b`, rather than `ab`. Reparsing the same emitted Markdown with either public prose component produces the same visible comment in terminal output and the same escaped comment in HTML. Direct rendering of the original prose input correctly shows only the two code values. The shipped `bt prose --md` command emits the comment-bearing Markdown above.

I copied the retained adjacency matrix's fixture builders into a temporary declared darkmatter L1 module. The sweep serialized and folded **27,264 cases**: sixteen contexts × eleven wrapper routes × thirteen boundary shapes × six value pairs × two dialects, excluding invalid nested links. Each serialized result was rendered through the public terminal-document API. **11,196 cases contain a generated comment; every one prints it under Warn, rejects under Strict, and drops it under Lossy.** This checks the actual public result, rather than assuming that an ignored parser event is invisible to the renderer.

Every context has 1,704 cases. The numbers below count cases with visible generated separators; each also rejects in strict terminal rendering. Controls without a generated separator are included in the same sweep.

| Writer site → darkmatter fold → terminal renderer | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Ordinary paragraph | Full boundary/value/wrapper matrix, both dialects | 748 visible-separator cases | Preserve code values and visible text without adding a comment. |
| Composition root with no separator | Same matrix | 748 cases | Preserve its no-added-text contract. |
| Ordinary root, separate children | Same matrix | 680 cases inside wrappers that group code; independently separated blocks are clean | Preserve block separation and grouped content. |
| Root with grouped inline children | Same matrix | 748 cases | Preserve grouped content. |
| Ordinary heading | Same matrix | 748 cases | Preserve heading content. |
| Section heading | Same matrix | 748 cases | Preserve heading content. |
| Table header, first column | Same matrix in the two-row/two-column sentinel fixture | 748 cases | Preserve cell content and row structure. |
| Table header, second column | Same matrix | 748 cases | Same contract. |
| Table body, first column | Same matrix | 748 cases | Same contract. |
| Table body, second column | Same matrix | 748 cases | Same contract. |
| Quote paragraph | Same matrix | 748 cases | Preserve quoted content. |
| List-item paragraph | Same matrix | 748 cases | Preserve item content. |
| Footnote-definition paragraph | Same matrix with paired reference | 748 cases | Preserve definition content. |
| Disclosure summary | Same matrix | 396 cases in plain Markdown; MarkdownPlus HTML-backed summary emits no separator | Preserve summary text; retain the clean HTML-backed spelling. |
| Disclosure body | Same matrix | 748 cases across both dialects | Preserve body content. |
| Columns | Same matrix | 396 cases in plain Markdown; MarkdownPlus HTML-backed columns emit no separator | Preserve column content; retain the clean HTML-backed spelling. |

The following table completes the consumer and boundary sweep. These are sibling decisions about the same generated input, rather than additional findings.

| Consumer or boundary | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Darkmatter public terminal entry point | Original clipboard, empty-bold, and dim inputs serialized by Prose | All three show the comment; default capability fallback also shows code fences | Show the intended code text and capability-appropriate styling only. |
| Darkmatter public HTML entry point | Same three inputs | All three escape the comment into visible text | Preserve visible text `ab`. |
| Shared browser renderer, Allow | Generated comment, all three strictness modes | Clean: real comment between two `<code>` elements; zero diagnostics | Retain invisible structure. |
| Shared browser renderer, Escape (default) | Same input, all modes | Visible escaped comment; Strict and Warn report a diagnostic, Lossy does not | Generated neutral structure must not require allowing arbitrary raw HTML. |
| Shared browser renderer, Reject | Same input, all modes | Strict rejects; Warn/Lossy show escaped comment | Neutral generated structure must not prevent safe rendering. |
| Shared terminal renderer | Every comment-bearing matrix output, all modes | Warn prints comment; Strict rejects; Lossy drops it | Preserve intended output without requiring callers to opt into losing content. |
| `Prose` reads its emitted Markdown | Original three inputs | Comment is literal in terminal and escaped in HTML | Handle the writer's generated boundary without adding visible content. |
| `InlineProse` reads its emitted Markdown | Same three inputs, plus normalized explicit code block | Same defect in all four inputs | Same contract. |
| Independent CommonMark/GFM reader | Retained full writer matrix | Clean: two original code values; comment is an HTML event | Keep the repaired code-fence safety. |
| Markdown reserialization after darkmatter folding | Generated-comment fixture | Clean spelling remains `` `a`<!-- -->`b` ``; rerendering still meets the affected consumer paths | Preserve code values and make final-target rendering faithful. |
| Visible space or soft break between codes | Full sweep | No generated comment | Retain existing spacing. |
| Actual strong/link syntax around second value | Full sweep | No generated comment at that boundary | Retain wrapper and link extent. |
| Empty first or second code value | Full sweep | No generated comment for that pair | Empty code adds no visible content. |
| Direct, flattened, empty-node, class/style-degradation boundaries | Full sweep | Every case that inserts a comment exposes the consumer defect, regardless of fence length or code spaces | A boundary repair must work after every degradation route. |
| MarkdownPlus HTML-backed summary/columns | Full sweep and retained independent-reader matrix | No generated code separator; existing HTML policies apply independently | Preserve this clean code-adjacency control. |
| Isolated safe-fence helper, complete `code_link()` link, `<br>`-separated cell block code | Retained controls inspected; shared-writer suite passed | No touching-code separator required | Retain single-value and separate-block behavior. |

Repair the generated-boundary integration across the listed consumers, while keeping the independent-reader matrix green. An HTML comment has no visible content; model that fact before raw-HTML escaping/rejection or choose another representation that all supported paths can consume. Do not enable unrestricted raw HTML just to hide this separator, and do not globally remove comment-shaped text: code values, literal escaped text, and quoted attributes must remain opaque. Preserve distinct code values, styles, links, table membership, and strictness behavior.

Retain tests that take the actual writer output through darkmatter's public terminal and HTML methods and through both prose components. Extend the existing fixture matrix rather than adding one isolated example. Check default browser policy as well as Allow/Reject, and all terminal strictness modes. Include comments as literal code/text controls so a repair cannot pass by deleting user content. Review the affected reader/renderer comments and current documentation in the same change.

This is a Level 1 integration defect: the wrong string and escaped HTML are already produced before any terminal emulator handles them. No keyboard behavior or human design decision is involved.

## Blocked Findings

None.

## Input Robustness Matrix

The latest repair introduces no file/configuration reader. The feature's load-bearing serialized field remains JSON `browser.block_element`. Its production-serialized fixture matrix passed in the full renderable suite.

| Shape | JSON `browser.block_element`: expected and observed |
|---|---|
| Positive control | All seven lowercase tags render correctly and retain other attributes. |
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

| Check | Fresh result |
|---|---|
| Renderable `just test` | 677 passed, zero skipped; includes all retained literal, break, HTML, pipe, quoted-field, and adjacency matrices. |
| Renderable `just lint` | Passed. |
| Biscuit-terminal `just test prose` | 451 passed; 3,153 excluded by scope/tier. Includes the four new public prose adjacency regressions. |
| Biscuit-terminal `just lint` | Passed for library and CLI. |
| Root `just check-tier-coverage biscuit-terminal` and `just check-tier-coverage renderable` | Passed; zero stranded tests. |
| Temporary declared darkmatter public-API probes | Completed the 27,264-case consumer sweep, public terminal/HTML reproductions, both prose reparsing paths, browser policy/strictness sweep, and Markdown reserialization control. Removed after use. |
| Shipped `bt prose --md` | Reproduces the generated-comment spelling. |
| Real-terminal verification | `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 prose`: 29 successful CLI-tier exits, zero library-tier selections. Actual tmux, WezTerm, and Apple Terminal scenarios ran; unavailable Kitty gate exits are not credited as rendering evidence. Required-backend proof records two actual tmux executions. The separately isolated `just test-l2 inline_code` run also passed: six successful exits, four actual tmux/WezTerm executions and two unavailable-Kitty gate exits; its proof records two actual tmux executions. |

The temporary probes used a declared L1 module and nextest. No permanent test was added or renamed. Existing biscuit-terminal consolidated roots declare the new adjacency file and the relevant Level 2 modules; their terminal feature has live recipes and CI metadata. Real-terminal recipes used background/detached harness spawning, without a focus request. No formatter was run.

Two initial Level 2 invocations overlapped. One failed to capture a WezTerm cell row; another captured a shell `setpgid` error in tmux. Those runs are not evidence of an implementation regression, and their shared backend-proof totals are not credited. The isolated result is reported separately above.

Full consumer suites, examples/benchmarks, and cross-OS runs were not repeated. The implementation log records broader checks and baseline failures; those are not substituted for fresh results here. Cross-OS evidence and human sign-off do not determine readiness.

## Requirement-to-verification mapping

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, empty inputs, neutral single-node projection, paragraph tags, wrapper-free fragments and streaming parity (criteria 1, 8–9, 11, 25, 31) | Level 1 public tree/HTML/serialized-fixture tests | Appropriate level; retained tests pass. |
| Paragraph boundaries, soft/hard breaks, whitespace, escaped backslashes, line-ending normalization, scope and opacity (criteria 2–7, 18, 26–28) | Level 1 grammar/opacity/container matrices; retained Level 2 row captures | Appropriate levels exist; fresh terminal-run limits are stated above. |
| Layout once, wrapping and container contracts (criteria 10, 12, 27–28, 31) | Level 1 layout/container assertions and Level 2 prose/container captures | Coverage exists; no new mismatch in required verification level identified. |
| Inline-code appearance, enclosing-style restoration, links and unstyled fallback (criteria 1, 14–16, 21, 29) | Level 1 capability matrices and retained Level 2 cell/style/link captures | Direct rendering has appropriate coverage. Serialized consumer rendering fails before terminal display. |
| Literal code, safe fences, hard-break spelling and table preservation (criteria 14–19, 21, 25, 28, 30) | Level 1 independent-reader matrices plus this review's public consumer sweep | Original adjacency defect repaired; generated separators violate visible meaning in repository consumers. Finding above. |
| Diagnostic/display migrations and interpolated literal values (criteria 13, 17, 20) | Retained Level 1 snapshots/regressions and representative Level 2 geometry | Inspected; full consumer suites not rerun. |
| `code_link()`, arguments, templates, expression listing and completion (criteria 22–23) | Retained Level 1 expression/template/CLI/LSP tests | Appropriate level; not rerun in this iteration. |
| Affected-area tests/lint and example/benchmark compilation (criterion 24) | Implementation log plus focused fresh checks above | Fresh scope is explicit; no claim of full affected-area verification. |
| Keyboard/input encoding | No keyboard behavior added | No Level 3 requirement. |

No separate performance or ergonomic finding was reproduced. Messenger's independently implemented Discord/Slack code-fence adjacency issue, recorded by the previous repair log, predates this generated-comment integration and remains outside this feature's implementation scope; it is not used to block readiness here.
