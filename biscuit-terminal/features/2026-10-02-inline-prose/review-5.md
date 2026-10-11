---
$schema: feature-review.yaml
ready: false
findings:
    - title: Markdown literal encoding misses email autolinks and block syntax across text boundaries
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-03T22:11:24-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
implemented_by: claude/opus
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-5.md
previous: 2026-10-02-inline-prose/review-4.md
next: 2026-10-02-inline-prose/review-6.md
---

# Inline Prose — Review 5

The feature is **not production ready**. Three findings from review 4 are resolved. The literal Markdown serialization repair fixes the previously reported examples, but still turns other literal values into links, lists, headings, quotes, or code blocks. One high-priority finding below carries the remaining cases together.

This review covers the working tree, including existing uncommitted implementation changes. Only review documents and requested metadata were changed permanently. Temporary public-API probes were removed. Cross-OS receipts and human sign-off are not readiness criteria here.

## Previous review follow-through

Review 4 has four unblocked findings and no blocked findings. There were no blocked findings to become unblocked. Its `human_review: false` remains appropriate: the repairs require no new product decision.

| Review 4 finding | Result |
|---|---|
| Wrapper scanning still interprets quoted and escaped tag text | Addressed. The recursive scanner now skips escaped pairs and recognized declarations as whole units. The retained matrix covers all nine wrapper categories, nested `href`, code-block `lang`, styled attributes, escaped tag text, inline code, and explicit code bodies through both components' trees and HTML. The Markdown link-label scanner also has a quoted-bracket regression. These tests pass. |
| Markdown serialization reinterprets literal text and entity spellings | Addressed for its enumerated punctuation/entity cases, captions, alternatives, titles, destinations, and disclosure contexts. The new independent-reader tests pass. The broader literal-preservation class remains incomplete; see the finding below. |
| Terminal capture tests depend on the shell displaying the full command | Addressed for the feature's prose, container, status, and style helpers. Printed begin/end markers replace echo matching. Eight fast detector regressions pass, and the previously failing inline-code scenarios now execute successfully in tmux and WezTerm. |
| The serialized paragraph tag lacks a retained robustness matrix | Addressed. The checked-in test now starts from production serialization, checks all seven valid tags and the legacy default through rendered HTML, and rejects every required malformed shape. It passes in the full renderable run. |

Earlier repairs for portable relative destinations, splitting links around block code, soft-break whitespace across wrappers, and diagnostic hard-break selection remain present. The focused prose run exercises their public regressions. The code-link template/completion run passes as well.

## Recurrence

The finding below repeats review 4's **“Markdown serialization reinterprets literal text and entity spellings.”** It also belongs to the broader literal-preservation class in reviews 1 and 2's **“Markdown serialization changes literal backslashes and break meaning.”** Review 3 verified the backslash repair; that repair remains clean.

The last sweep should also have checked email autolinks whose first character is not a letter, block markers assembled from adjacent literal nodes, and indentation that CommonMark treats as code. Its retained values cover `<https://x.io>` and whole `1. literal` strings, but omit those shapes. The sweep below repeats all 29 retained text contexts, titles, destinations, disclosure summaries, and code controls, then varies every split position in the block-marker values. `recurrence: true` records the incomplete sweep; it does not require a human approval step.

## Unblocked Findings

### High: Markdown literal encoding misses email autolinks and block syntax across text boundaries

**Defect class:** a serializer encodes literal data without accounting for every syntax opener or for syntax assembled across adjacent text values, so reading its output changes the text or document structure.

In **renderable**, [the literal encoder `escape_markdown_text`](../../../renderable/src/tree/render/markdown.rs#L1490), which protects text values, escapes `<` only before letters, `/`, `!`, `?`, or an unknown next character. CommonMark email autolinks can start with digits and other punctuation too. Literal `<3@example.com>` therefore becomes an email link and loses its visible angle brackets. In **biscuit-terminal**, both public components produce escaped literal HTML for this input but unsafe Markdown:

```rust
Prose::new("<3@example.com>").render_html_fragment().render()
// <p>&lt;3@example.com&gt;</p>
Prose::new("<3@example.com>").render_markdown()
// <3@example.com> — an independent reader creates an email link
```

In **renderable**, [the inline sequence writer `join_pieces`](../../../renderable/src/tree/render/markdown.rs#L530), which joins rendered children, decides whether a literal starts a line from the output seen so far. It does not protect a marker completed by a later text node, or a marker after indentation emitted by an earlier node. These are reachable through authored prose, without constructing an unusual tree:

```rust
Prose::new("1<clipboard>. literal</clipboard>").render_markdown()
// 1. literal — becomes a numbered list instead of paragraph text
```

`12<clipboard>) literal</clipboard>` likewise becomes a list. Direct public trees with `Text(" ")` followed by `Text("# literal")`, `Text("> literal")`, or `Text("- literal")` can become headings, quotes, or lists. The same decisions affect transparent spans and unknown extended wrappers because the writer flattens their children.

Finally, [the block-start protector `block_start_escape`](../../../renderable/src/tree/render/markdown.rs#L1598) returns no protection for indentation deeper than three spaces and does not recognize a leading tab. Both prose components serialize `"    literal"` and `"\tliteral"` as indented code, although their trees and HTML hold ordinary paragraph/inline text. After a hard break, that indentation can instead disappear. This contradicts the promised preservation of ordinary spaces and tabs. GFM cells also strip unencoded leading cell whitespace; their row-safe encoder is another affected literal-data position.

The public probes copied [the retained literal-text fixture builders](../../../renderable/tests/markdown_literal_text.rs) and changed only the value, or split that value into adjacent text children of a neutral span. Both dialects were read back with `pulldown-cmark`, with table, strike, and footnote support enabled. Both prose components were exercised separately for the email, numbered-marker, indentation, and clean underscore/heading controls.

The shape matrix was:

- Email-looking text: `<3@example.com>`, `<+foo@example.com>`, and local parts starting with `3`, `.`, `!`, `#`, `$`, `%`, `+`, `-`, `/`, `?`, `^`, `{`, `|`, and `}`. Digit, `.`, `#`, `$`, `%`, `+`, `-`, `^`, `{`, `|`, and `}` starts are unsafe in ordinary text; existing escapes protect `!`, `/`, `?`, and the underscore control. A pipe inside a GFM cell is already escaped and was a clean cell control.
- Block-marker values: `# literal`, `## literal`, `> literal`, `- literal`, `+ literal`, `1. literal`, `12) literal`, `---`, and `===`, with zero through three leading spaces, split at every character boundary. Whole-value spellings are controls. Split numbering fails without indentation; split indentation additionally exposes heading, quote, bullet, and thematic-break syntax. `===` has existing escapes and does not create an unintended block in the tested cases.
- Indentation: four leading spaces and one leading tab, at paragraph start, after soft/hard breaks, within a multiline value, and in the other fixture contexts. Code and intentionally trimmed caption values are separate controls.
- Every original punctuation/entity value, link/image title and destination, disclosure summary, inline/block code, and backslash/break control was repeated.

In the instance table, **email** means the first shape group, **split markers** the second, and **indentation** the third. “Clean” means the reader retained the relevant literal value/structure; a context that does not store child nodes cannot have a split-field reproduction. The expected result in every affected row is literal content with only the structure authored in the tree.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `Prose` and `InlineProse`, both Markdown dialects | Email, numbering across clipboard wrapper, indentation | Email links, numbered lists, or indented code replace literal text | Match the literal content and paragraph/inline shape shown by HTML/tree. |
| Paragraph and root text | All three groups | Email links; split numbering/indented markers activate blocks; four spaces/tab activate code | Literal text; no added block or link. |
| Text after ordinary `a `; sequence after `a ` | Email and split markers | Email fails; unindented block-marker controls remain inline | Preserve email-looking text too. |
| After soft break; after hard break; multiline text value | All three groups | Email fails; split markers interrupt the paragraph; indentation is lost or changes block shape | Keep the break and literal text. |
| Heading text | Email, split markers, indentation | Email fails; unindented numbering is clean; leading indentation is stripped by the reader | Preserve the heading's literal value. |
| Block quote | All three groups | Email fails; split markers create nested blocks; indentation can become code | Preserve only the requested quote. |
| Unordered list paragraph; ordered item holding bare text | All three groups | Email fails; split markers create nested lists/blocks; indentation can become code | Preserve only the requested list/item. |
| Bold, italic, strike | All three groups | Email creates inner links. Unindented split numbering stays text. Leading whitespace moves outside delimiters under the existing wrapper-edge policy | Preserve literal email text; retain the existing deliberate edge-whitespace policy. |
| Color/underline span, plain Markdown | All three groups | Email fails; flattened split markers activate blocks; indentation can become code | Literal text after style degradation. |
| Color/underline span, MarkdownPlus | All three groups | Email and split-block controls are clean inside the HTML span; indentation text remains inside that span | Clean encoding control. |
| Styled span after `a ` | Email, split markers | Plain Markdown email fails; MarkdownPlus email is escaped; marker controls stay inline | Literal value in both dialects. |
| Transparent span; unknown extended wrapper | All three groups | Same failures as ordinary paragraph text | Flattening must preserve literal content. |
| Link label | Email, split markers, indentation | Email becomes a link event inside the requested link; block-marker and leading-indentation controls stay literal inside the label | One requested link with literal label. |
| Table cell; bold inside a cell | All three groups | Email fails except pipe-protected control; split block markers stay inline; leading cell indentation is stripped | Preserve literal cell content as well as row structure. |
| Footnote body | All three groups | Email fails; split markers create blocks inside the definition; indentation can become code | Preserve the original footnote body. |
| Sequence at line start | All three groups | Same failures as paragraph/root text | Literal sequence with no added blocks. |
| Disclosure body | All three groups | Email fails in both dialects; split markers introduce blocks (plain Markdown follows paragraph-interruption rules); indentation can become code | Preserve body text and requested disclosure structure. |
| Plain-Markdown disclosure summary | Email and indentation; original values | Email becomes a link; original punctuation/entity controls pass | Literal summary text. |
| MarkdownPlus disclosure summary | Same values | Raw HTML text is correctly HTML-escaped; no email link created | Clean raw-HTML encoding control. |
| Table caption | Email, indentation, original values | Email is HTML-escaped; caption trimming follows the documented existing policy | Clean; retain intentional caption trimming. |
| Image alternative; multiline alternative; image in table cell | Email, indentation, original values | Email becomes inner link syntax; multiline indentation is stripped; first-line ordinary indentation is retained | Literal alternative text, without added links or lost multiline whitespace. |
| Link/image titles and destinations, including table-cell routes | Email and original values | Email-shaped values remain literal; original entity/punctuation tests pass | Clean. Destination tabs/newlines use the existing space-normalization policy, excluded from the text defect. |
| Inline code and fenced block code | All values | Literal data remains code; email-looking contents do not become links | Clean; do not add text escaping inside code. |
| Literal backslashes and delimiter/break edges | Retained regression matrix | Pass | Clean repaired controls. |

The newly exposed **renderable** [`markdown::escape_text`](../../../renderable/src/markdown.rs#L56), intended for callers writing Markdown manually, delegates to the same literal encoder and therefore needs the email/indentation correction too. This shared route was inspected; it is not a separate independently implemented serializer. Raw `Html` nodes intentionally remain verbatim and are outside the literal-text contract. Other packages' standalone serializers are outside this feature's implementation review; this finding enumerates the affected shared renderer and its component entry points.

Repair the shared encoding decisions, rather than adding escapes in individual prose consumers. Preserve awareness of a logical line across adjacent literal children and transparent wrappers, cover the full email-autolink grammar, and encode indentation in positions where a reader would remove it or read code. Retain the full value/context/split-boundary matrix through public rendered results and an independent reader. A regression that only compares emitted escape bytes will miss the structural changes above. Review the helper's new literal-preservation documentation in the same repair.

## Blocked Findings

None. The existing contract requires literal text to remain literal; no new design decision or human-only activity blocks this repair.

## Input Robustness Matrix

The only newly load-bearing serialized field is JSON `browser.block_element`. Markup strings are grammar input, not a new configuration/file format. The compatibility test in **renderable** [the browser renderer](../../../renderable/src/tree/render/browser.rs#L4895) now retains the complete matrix:

| Shape | Public result | Retained coverage |
|---|---|---|
| Positive controls: all seven valid tag names | Render the named paragraph element with `data-role="note"` | Present and passing. |
| Absent | Render legacy `<p data-role="note">x</p>` | Present and passing; defined default. |
| Explicit null | Reject | Present and passing. |
| Wrong whole-field type: number | Reject | Present and passing. |
| Mixed element types: `["div",123]` | Reject; field is scalar | Present and passing. |
| Every element invalid: `[123]` | Reject | Present and passing. |
| Empty array, object, or string | Reject each | Present and passing. |
| Duplicate key | Reject duplicate `block_element` | Present and passing. |
| Valid JSON plus garbage | Reject trailing content | Present and passing. |
| Unknown or uppercase enum spelling | Reject | Present and passing. |

`Status::line_breaks` is skipped during serde input; `InlineProse` and the changed table content types do not introduce deserialization. No second newly load-bearing configuration field was found.

## Verification

| Command/check | Result |
|---|---|
| Renderable: `just test` | 603 passed, including literal text, delimiter edges, browser/streaming tags, validation, and JSON compatibility. |
| Biscuit-terminal: `just test prose_` | 222 passed, including grammar, opacity, destinations, containers, Markdown read-back, and shipped CLI cases. |
| Biscuit-terminal: `just test output_markers` | Eight passed. |
| Darkmatter: `just test code_link` | 16 passed, including template rendering, code labels, binding behavior, and DMLS completion. |
| Biscuit-terminal: `just test-l2 inline_code` | Four scenarios executed and passed in tmux/WezTerm. Two Kitty scenarios returned through availability gates; no Kitty execution evidence credited. |
| Biscuit-terminal: `just test-l2 level2_status_hard_line_breaks` | Two passed, tmux and WezTerm. |
| Biscuit-terminal: `just test-l2 level2_prose_cells_in_tmux` | One passed. |
| Root: `just check-tier-coverage biscuit-terminal` | Zero stranded tests. |
| Renderable and biscuit-terminal: `just lint` | Both passed on implementation source after removal of temporary probes. |
| Temporary public component example and copied independent-reader matrices | Reproduced the finding, including every retained context and every character split in the block-marker matrix. Removed after execution. |

The temporary matrices intentionally printed all mismatches to finish the sweep rather than stopping at the first failing row. Their logs are not retained regression tests. A plain-disclosure assertion also failed on the new email value, independently confirming that route. An initial target-name filter selected zero tests; it was corrected to `--test` selection before collecting probe results. Counts above exclude that empty run.

The new prose and marker test modules are declared in the consolidated Level 1 targets. The terminal modules are declared in the CLI's `level2` target, enabled through `terminal-tests`, selected by a live recipe, and covered by CI feature metadata. These checks used the repository's detached/background harness recipe; no focus activation was requested. No permanent test was added or renamed by this review.

Full downstream area tests, examples/benchmarks, browser-tier execution, and cross-OS runs were not repeated. The implementation log's remaining baseline failures are not independently attributed here. Narrow passing checks do not override the reproduced literal-output defect.

## Requirement-to-verification mapping

The numbers below refer to the numbered **Acceptance Criteria** in the specification; the behavior is stated here so the numbers are not needed to understand the assessment.

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, wrapper-free inline HTML, relative link acceptance example, empty content (1, 2, 7, 11, 25) | Level 1 public tree/HTML/Markdown tests; Level 2 representative link/break capture | Passing retained cases; literal output has the finding above. |
| Paragraph splitting, soft/hard modes, whitespace, normalized line endings and escapes (3–7, 26–27) | Level 1 grammar/read-back matrices; Level 2 prose/container/status geometry | Passing grammar cases; emitted Markdown can still reinterpret literal indentation or split markers. |
| Paragraph tags, streaming parity, legacy serialization, invalid placements (8, 31) | Level 1 public HTML/streaming/validation and complete JSON matrix | Passing; in-process checks are appropriate for serialization and emitted element structure. |
| Fenced block shape, styles across paragraphs, literal code, inline fence degradation (9, 14, 16, 18–19, 27–28, 30) | Level 1 public tree/independent-reader tests; Level 2 table/container capture | Passing retained cases. |
| Layout exactly once, container content types and valid embedded trees (10, 12, 31) | Level 1 layout/container and CLI tests; existing Level 2 geometry/style suites | Appropriate test levels present; focused component tests pass. No separate browser-tier execution claimed here. |
| Migrated diagnostic row structure and literal code values (13, 17, 20) | Level 1 actual diagnostic/snapshot tests in affected packages; representative Level 2 status capture | Retained prior repairs inspected; current focused prose/status checks pass. Downstream diagnostic suites were not rerun in this iteration. |
| Dim code, enclosing color restoration, hyperlink extent, unstyled fallback (15, 21, 29) | Level 2 tmux/WezTerm cell/style/link capture, with Level 1 capability tests | Appropriate level executes and passes. |
| Safe code-link labels, matching resolution, four migrated templates and completion (22–23) | Level 1 composition, public render results, binding tests, and DMLS completion | 16 focused tests pass. Installed personal prompt copies remain the spec's separate manual maintenance item, not a code finding. |
| Affected-area checks and compiled auxiliary targets (24) | Scoped nextest and lint runs; implementation log records broader earlier checks | This review does not claim a complete rerun of every affected package/auxiliary target. |

No keyboard, mouse, paste, or input-encoder behavior is added, so Level 3 is not required. No new terminal verification-level mismatch was found. No additional performance or ergonomics change is required for readiness beyond correcting the shared literal serializer.
