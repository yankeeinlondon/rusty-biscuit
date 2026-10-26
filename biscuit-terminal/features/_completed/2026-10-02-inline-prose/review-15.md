---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Markdown documentation still promises unrestricted reader fidelity
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-10T17:15:31-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: false
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-15.md
previous: 2026-10-02-inline-prose/review-14.md
---

# Inline Prose — Review 15

The feature is **production ready within the specification's stated scope**. The required repair from review 14 preserves code-shaped and comment-shaped link destinations in both prose components. No high, medium, or low finding remains in this review. One documentation observation remains; it does not require more Markdown escaping machinery.

This review covers the existing working tree, including the uncommitted repair. Permanent changes made by this review are this document and the requested review/spec frontmatter. No implementation, test, snapshot, installed prompt, or lifecycle directory was changed. The biscuit-terminal, rust-testing, and biscuit-test-harness skills guided the review.

## Previous review follow-through

Review 14 had one unblocked finding, **“Link destinations change across prose parsing and expression composition,”** and no blocked findings. Its implementation log reports no deferred or disputed findings. The repair addressed the portions authorized by the specification's October 4 scope decisions; it did not repair every instance originally listed by review 14.

| Portion of the previous finding | Assessment |
|---|---|
| Code spans, completed comments, and explicit code bodies become placeholders inside Markdown destinations | Fixed. The whole destination is protected before its contents can be interpreted as code, comments, or tags. Both components retain the destination, label, and trailing text on every target. |
| Paragraph splitting interprets tags inside a destination | Fixed through the same protected destination. Explicit code bodies in the retained destination matrix no longer create a block or alter surrounding scope. |
| Code and comments in link labels | Preserved. A code label remains an inline-code node; a label comment remains invisible. |
| Explicit anchors created with the public attribute-quoting helper | Clean control across all thirteen destination payloads and both components. |
| Nested parentheses, angle-bracket delimiters, double quotes, and escaped entity spellings in prose destinations | Not repaired. The log records comparison with the pre-feature base showing these four behaviors already existed. The specification explicitly authorizes their repair here only when they are regressions. They are not re-raised. |
| Darkmatter expression destination formatting, cleanup reserialization, and pull-request/CI-job helpers | Not repaired. The current specification explicitly lists this preexisting escaping behavior as out of scope, including the formatting shared by `link()` and `code_link()`. It is not re-raised under a stronger independent-reader expectation. |

I compared the finding classes and blocked status in all fourteen earlier reviews. None contains a blocked code finding that became actionable before the last repair. Reviews 2 and 3 requested attention to incomplete repair cycles, rather than a missing product decision or a human-only verification task. Those requests do not block this implementation. No disputed finding is re-raised unchanged.

## Destination repair sweep

The bounded class checked here is **inline syntax being interpreted inside a literal link destination**. The retained public-output matrix covers thirteen payloads through Markdown links and explicit anchors, in both `Prose` and `InlineProse`. It checks tree destinations, decoded HTML attributes, Markdown read by an independent parser, terminal OSC 8 targets, label text, trailing text, and absence of generated placeholders. Separate controls cover label syntax, escaped closing parentheses, paragraph boundaries, incomplete markup, and unused destination placeholders.

| Site in the affected path | Shape checked | Observed and required result |
|---|---|---|
| Initial code-span scan | Single/double backtick runs, padded runs, unmatched backtick | Matched and unmatched destination text stays in the destination; code in the label still becomes inline code. |
| Initial comment scan | Six completed comment forms/pairs and an unclosed comment | Destination text stays literal; label comments disappear; an unclosed comment outside a link stays visible. |
| Explicit code-body lifting and block splitting | `<code-block>y</code-block>` in a destination | Kept in the destination without introducing a code block. |
| Label/destination boundary scan | Escaped `)`, blank-line labels, escaped opening bracket, `](…)` without an open label | Existing destination closing and component-specific paragraph behavior preserved. |
| Link conversion and attribute reading | The same protected destination values | Destination resolves into the link attribute without a parser placeholder. |
| Later bold/italic phases | Ordinary destinations; destination payloads with label code and styled labels | Protected destinations survive; labels retain their formatting. |
| Explicit anchor reader | Same thirteen payloads via `Prose::quoted_attr` | Original destinations retained independently of Markdown link recognition. |
| Tree, browser, Markdown, and terminal projections | Every retained matrix row in both components | Exact destination values and unchanged label/tail through public results. |
| Shipped `bt prose --html` | Seven destination payloads with plain, code, and bold labels | All 21 cases retain decoded destinations and visible text; no generated sentinel characters. |

The shipped-command check used Python's HTML parser to decode attributes and extract text independently of the Rust test helper. No new parser or projection defect was reproduced for a realistic caller input. The log's unusual unknown-tag/closing-bracket ambiguity does not justify a new protection layer in this cycle.

## Unblocked Findings

None.

## Blocked Findings

None.

## Observations

### Markdown documentation still promises unrestricted reader fidelity

In the **renderable** package, [the tree-rendering documentation](../../../renderable/docs/tree-rendering.md:253) still says that “whatever characters a value holds, a CommonMark/GFM reader sees exactly those characters.” [The renderable skill's Markdown topic](../../../.claude/skills/renderable/markdown.md:103) repeats that a text value reads back as exactly its characters. The specification's October 4 decision and Definition of Done call for narrowing these promises. Describe the escaping rules and their supported examples, and state that arbitrary content is not guaranteed to round-trip through every reader. This is a documentation follow-up, recorded as a non-blocking observation under this review's explicit rule for overpromising pages; it is not authority for further serializer repairs.

## Recurrence

No finding is raised, so `recurrence` is `false`. The previously recurring opacity defect is verified as repaired within its current authorized boundary. Earlier arbitrary Markdown-fidelity cases are not promoted back into requirements.

## Input Robustness Matrix

The latest repair changes a prose text grammar, not a configuration reader, so the configuration matrix does not apply to its destinations. The original feature does add the serialized JSON field `browser.block_element`. Its retained matrix, [in renderable's browser renderer tests](../../../renderable/src/tree/render/browser.rs:4952), starts from a production-serialized paragraph and checks public HTML for accepted shapes. It passed in the fresh renderable suite.

| Shape | JSON `browser.block_element`: expected and observed |
|---|---|
| Positive control | All seven lowercase tags render the matching element and retain the fixture's other attribute. |
| Absent | Legacy paragraph renders `<p>`. |
| Explicit null | Rejected. |
| Wrong type, whole field | Number rejected. |
| Wrong type, one element | Mixed array rejected; the field is scalar. |
| Wrong type, every element | All-invalid array rejected. |
| Empty | Empty string, array, and object rejected. |
| Duplicate key | Rejected. |
| Trailing or invalid content | Trailing garbage rejected. |
| Invalid spelling | Unknown and uppercase tag names rejected. |

## Requirement-to-verification mapping

Criterion numbers below identify entries in the specification's Acceptance Criteria list; each row also describes the behavior being checked.

| User-facing requirement | Appropriate verification present | Assessment |
|---|---|---|
| Inline/block shape, empty inputs, public exports, block tags, and browser streaming parity (1, 8–9, 11, 25, 31) | Level 1 public tree/HTML, export, validation, and serialized-fixture tests | Appropriate. Fresh prose and renderable runs pass. |
| Paragraphs, soft/hard breaks, whitespace, line endings, and opaque code (2–7, 18, 26–28) | Level 1 grammar tests plus Level 2 captured terminal rows | Appropriate. Fresh prose tests and tmux break/paragraph scenario pass. |
| Layout once, wrapping, and container content shapes (10, 12, 27–28, 31) | Level 1 container/layout assertions; retained Level 2 container/layout captures | Coverage present. Fresh prose container assertions pass; the complete container terminal suite was not rerun. |
| Code styling, style restoration, hyperlink extent, and unstyled fallback (1, 14–16, 21, 29) | Level 1 capability-specific assertions plus Level 2 captured styles and links | Appropriate. The fresh tmux scenario asserts actual displayed dim/color/link extents and fallback delimiters. |
| Code contents, safe fences, hard-break Markdown, and table behavior (14–19, 21, 25, 28, 30) | Level 1 direct-node and independent-reader tests, plus public downstream consumer matrices | Appropriate. Fresh renderable, prose, and darkmatter runs pass. |
| Diagnostic, newline, and template migrations (13, 17, 20, 22) | Retained Level 1 output snapshots/regressions and template tests | Coverage present. Fresh template tests pass; the complete downstream diagnostic suites were not rerun. |
| `code_link()` arguments, labels, catalog, CLI listing, and completion (22–23) | Level 1 expression/template tests, catalog parity, claudine listing test, and DMLS completion test | Appropriate. Fresh darkmatter/DMLS tests pass; claudine's declared listing test was inspected but not rerun. |
| Affected-area tests/lint and examples/benchmarks (24) | Prior implementation records, supplemented by scoped fresh runs below | Latest source repair is in biscuit-terminal; its fresh lint checks all targets. This review does not claim a new all-consumer gate run. |
| Literal code/comment destinations on every target (32) | Level 1 both-component public-output matrix and shipped CLI check | Appropriate for destination values. All 35 retained tests and 21 independent CLI cases pass. Real terminal style/link rendering is separately checked at Level 2. |
| Keyboard, mouse, paste, or input encoding | No input interaction introduced | No Level 3 requirement. |

The new destination module is declared by `lib/tests/l1/main.rs`, compiles in the consolidated `l1` target, has no excluding tier markers or feature gate, and runs through `just test`. Its assertions inspect public output rather than merely accepting a parser result. No test in the fresh Level 1 runs exceeded five seconds. The 6.7-second real-terminal scenario is Level 2.

## Verification

All commands ran in this worktree using absolute package working directories.

| Fresh check | Result |
|---|---|
| biscuit-terminal `just test prose_destination_syntax` | 35 passed; 3,624 excluded by filter/tier. |
| biscuit-terminal `just test prose` | 490 passed; 3,169 excluded by filter/tier. Includes the 35 destination tests, not an additional independent count. |
| renderable `just test` | 684 passed; zero skipped. |
| darkmatter `just test code_link serialized_code_neighbors` | 52 passed; 9,070 excluded by filter/tier. Includes template behavior, DMLS completion, and serialized-neighbor consumers. |
| biscuit-terminal `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_prose_inline_code_and_breaks_in_tmux` | One CLI scenario passed; backend proof: `run=1`, `skip=0`, `panic=0`. Zero library tests matched. Harnesses ran in the background without requesting focus. |
| biscuit-terminal `just lint` | Library and CLI passed, including all-target compilation. |
| Root `just check-tier-coverage biscuit-terminal` | Zero stranded tests. |
| Fresh shipped `bt prose --html` | Original comment reproduction and 21 independently parsed HTML cases passed. |

Full downstream suites, all browser/terminal scenarios, and cross-OS execution were not repeated. Cross-OS evidence and external human sign-off do not determine this review's readiness verdict. No separate performance or ergonomic defect warrants another repair cycle.
