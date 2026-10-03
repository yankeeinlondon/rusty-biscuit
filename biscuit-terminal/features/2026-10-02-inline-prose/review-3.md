---
$schema: feature-review.yaml
ready: false
findings:
    - title: Opaque code bodies and quoted attributes are altered by preprocessing
      priority: high
    - title: Explicit code blocks inside links invalidate the entire block output
      priority: high
    - title: Relative link destinations do not meet the specified portable output
      priority: medium
    - title: Soft-break whitespace trimming stops at inline wrapper boundaries
      priority: medium
    - title: New terminal rendering behavior lacks Level 2 verification
      priority: high
    - title: Single-newline callers still lose their line structure
      priority: high
human_review: true
human_review_items:
    - |-
        Review why the last implementation cycle completed only the Markdown backslash finding while six actionable findings remain. Check that the next implementation covers every affected site in the instance tables below before restarting the automatic review loop. No product design decision needs to be reopened.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-03T16:17:14-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: false
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-3.md
previous: 2026-10-02-inline-prose/review-2.md
---

# Inline Prose — Review 3

The feature is **not production ready**. The Markdown backslash fix passes its new independent-reader regressions. Six actionable findings from review 2 remain: code contents are altered, linked block code invalidates output, relative links become machine-specific, soft-break whitespace survives across wrappers, terminal display lacks appropriate verification, and existing diagnostic rows collapse.

This review covers the working tree, including preexisting uncommitted implementation changes. Only review documents and requested metadata were changed permanently. Temporary public-API probes were removed after execution. Cross-OS evidence and human review are external to the readiness decision.

## Previous review follow-through

Review 2 lists seven unblocked findings and no blocked findings. Its human review item concerns the incomplete implementation cycle; it blocks no individual code fix. There were consequently no blocked findings to become unblocked.

| Review 2 finding | Result in this review |
|---|---|
| Markdown serialization changes literal backslashes and break meaning | Addressed. Six public component regressions and 15 shared-renderer regressions pass. Both Markdown dialects are read back with an independent CommonMark parser; coverage includes break modes, literal backslash counts, emphasis delimiters, table cells, inline HTML, labels, titles, and controls. |
| Opaque code bodies and quoted attributes are altered by preprocessing | Still reproducible in both components; full explicit/fenced body and attribute matrix repeated below. |
| Explicit code blocks inside links invalidate the entire block output | Still reproducible standalone and in all seven block containers; alternative wrappers and fenced syntax remain clean. |
| Relative link destinations do not meet the specified portable output | Still reproducible in both components and both link syntaxes; HTTPS controls remain unchanged. |
| Soft-break whitespace trimming stops at inline wrapper boundaries | Still reproducible in every wrapper branch, both directions, both components. |
| New terminal rendering behavior lacks Level 2 verification | No new backtick-span or new-break geometry scenarios found in either terminal suite. |
| Single-newline callers still lose their line structure | All listed diagnostic builder families and CLI sites still use default soft breaks. Public component probes still collapse their copied row shapes. |

The implementation log records completion of the Markdown serialization finding only. The earlier diagnostic escaping fix remains present; this review also reruns its original nested-expression regression. The requested `implemented: true` on review 2 records completion of its implementation cycle, not resolution of every finding.

## Recurrence

Both earlier reviews in this directory were compared. All six findings below recur, so `recurrence: true` stops automatic iteration for a human check of the incomplete cycle. The fixes themselves remain unblocked.

| Recurring class and earlier finding | Sibling sites the earlier cycle should have swept | Current complete sweep |
|---|---|---|
| Opaque preprocessing — same title in reviews 1 and 2 | Explicit code, fenced code, and quoted attributes in both components | Five code bodies across both syntaxes/components, plus fence-looking and ordinary multiline attributes. |
| Block children inside links — same title in reviews 1 and 2 | Link, style, and transparent projection; standalone and every block container | Explicit/fenced syntax across bold, color, underline, link, and clipboard; standalone, seven containers, and inline component controls. |
| Nonportable destinations — same title in reviews 1 and 2 | Both authored link forms in both components; portable and terminal output | HTML and Markdown relative/HTTPS pairs; terminal destination resolution inspected. |
| Wrapper-boundary whitespace — same title in reviews 1 and 2 | Bold, color, underline, link, transparent, and unwrapped text | Both boundary directions through both components for each wrapper, plus whitespace controls. |
| Missing terminal verification — same title in reviews 1 and 2 | Standalone code, enclosing styles/links, fallback, table cells, paragraph/break geometry | All library/CLI terminal modules inspected, with declarations/features/live recipes checked. |
| Collapsed display rows — same title in review 2 | Lifecycle, provider, schema, sequence diagnostics; help, logs, hooks, provider reports; migrated darkmatter controls | All twelve diagnostic shapes exercised through public `StatusBlock`; four CLI source sites checked against copied public-rendering shapes, with paragraph and hard-mode controls. |

These are unresolved previously enumerated classes, rather than newly discovered isolated siblings. The corrected Markdown serializer is not a recurring finding in this review.

## Unblocked Findings

### High: Opaque code bodies and quoted attributes are altered by preprocessing

**Defect class:** preprocessing rewrites opaque regions before the parser protects them, leaking generated markup or internal placeholders into public output.

In **biscuit-terminal**, [fence lifting and inline preprocessing](../../lib/src/components/prose/markdown.rs) still protect only fenced bodies before the inline phases. [The token parser](../../lib/src/components/prose/tokens.rs) treats explicit code bodies as literal only after they have been rewritten. Fence lifting also still scans quoted attribute lines as ordinary input.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `Prose` explicit code | `<code-block>` body containing backtick-delimited `x` | `<pre><code>` contains U+0002, `0`, U+0002 | Original backticks and `x`. |
| `InlineProse` explicit code | Same | `<code>` contains the same placeholder | Original body as inline code. |
| Both explicit-code paths | Body `**x**` | Code contains literal generated `<b>x</b>` | Original `**x**`. |
| Both explicit-code paths | Body `[x](https://e.io)` | Code contains literal generated anchor markup | Original Markdown syntax. |
| Both explicit-code paths | Literal U+0002 | Added backslash | Original character. |
| Both explicit-code paths | Ordinary multiline `x` / `y` | Block retains LF; inline uses space | Same; clean control. |
| Both fenced-code paths | Each of the five bodies above | Literal contents preserved; inline LF normalized | Same; clean siblings. |
| Both quoted-attribute paths | `href="x\n```\ny\n```\nz"` | `y` is replaced by a fence placeholder in the URL | Preserve attribute content. |
| Both quoted-attribute paths | Newline without fence-looking lines | Attribute remains intact before destination resolution | Same; clean opacity control. |

Recognize opaque bodies and quoted attributes before transformation. Retain this complete body/attribute matrix as public output tests, with no placeholder characters in generated content unless they were actually authored.

### High: Explicit code blocks inside links invalidate the entire block output

**Defect class:** one inline wrapper retains a block child instead of splitting around it, producing a tree that output validation rejects.

In **biscuit-terminal**, [the link projection](../../lib/src/components/prose/tokens.rs) still wraps every child in a `Link`. [Styled projection](../../lib/src/components/prose/tree.rs) already splits around block code. The public reproduction remains:

```rust
Prose::new("<a href=\"https://e.io\">a<code-block>x</code-block>b</a>")
    .render_html_fragment().render(); // empty string
```

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Standalone `Prose` link | Explicit code between linked `a` and `b` | Invalid tree; empty HTML | Linked paragraphs around a sibling code block. |
| Markdown link through `bt prose --html` | `[a<code-block>x</code-block>b](https://e.io)` | Validation error | Same valid block structure. |
| `UnorderedList` | Same explicit-anchor `Prose` | Invalid tree | Valid embedded blocks. |
| `OrderedList` | Same | Invalid tree | Valid embedded blocks. |
| `BlockQuote` | Same | Invalid tree | Valid embedded blocks. |
| `TwoColumn` | Same | Invalid tree | Valid embedded blocks. |
| `StatusBlock::body` | Same | Invalid tree | Valid embedded blocks. |
| `Section` | Same | Invalid tree | Valid embedded blocks. |
| `Compose` | Same | Invalid tree | Valid embedded blocks. |
| Bold, color, underline, transparent wrappers | Same explicit code, standalone and each container above | Valid trees; code is a sibling | Same; clean projection branches. |
| Link wrapper, standalone and each container above | Fenced code instead | Valid trees; links resume after code | Same; clean alternate syntax. |
| `InlineProse`, each of five wrapper branches | Explicit and fenced code | Valid phrasing trees | Same; clean component sibling. |

Apply split-and-resume projection to links, preserving destinations on the inline runs. Add explicit code alongside fenced code in the existing wrapper/container matrix.

### Medium: Relative link destinations do not meet the specified portable output

**Defect class:** target-neutral parsing resolves authored relative destinations against the local checkout, making portable output machine-specific.

In **biscuit-terminal**, [the token parser](../../lib/src/components/prose/tokens.rs) still calls [the filesystem-oriented destination resolver](../../lib/src/components/prose/styles.rs) before choosing a rendering target. This fails the first acceptance example.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `InlineProse`, Markdown link | `[the plan](plan.md)` | HTML and Markdown contain the review checkout's absolute `file://` URL | Preserve `plan.md`. |
| `Prose`, Markdown link | Same | Same rewriting | Preserve `plan.md`. |
| `InlineProse`, explicit anchor | `href="plan.md"` | Same rewriting | Preserve `plan.md`. |
| `Prose`, explicit anchor | Same | Same rewriting | Preserve `plan.md`. |
| Both components | HTTPS destination | Unchanged | Same; clean control. |
| Terminal link projection | Relative destination | Parser supplies resolved file URL; terminal consumes it | Retain a usable file link through terminal-specific resolution. |

Preserve authored destinations in the tree and resolve file references when rendering terminal links, using the repository's file-reference API. Update [the current prose documentation](../../docs/components/prose.md), which explicitly describes rewriting on every target. The implementation log's acknowledged departure and an HTTPS substitute in examples do not satisfy the relative-link acceptance criterion.

### Medium: Soft-break whitespace trimming stops at inline wrapper boundaries

**Defect class:** whitespace normalization stops at recursive text buffers, leaving extra whitespace around soft breaks that touch inline wrappers.

In **biscuit-terminal**, [the soft-break parser branch](../../lib/src/components/prose/tokens.rs) still trims only its local text buffer.

Each wrapper was tested in both directions: `a <wrapper> \nb</wrapper>` and `<wrapper>a \n</wrapper> b`, through both public components.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Bold | Both boundary directions | Text contains two spaces | One space, preserving bold extent. |
| Color | Both directions | Two spaces | One space, preserving color extent. |
| Underline | Both directions | Two spaces | One space, preserving underline extent. |
| Link | Both directions | Two spaces | One space, preserving link extent. |
| Transparent clipboard | Both directions | Two spaces | One space. |
| Unwrapped text | `a \t\n\t  b` | One space; existing test passes | Same; clean control. |
| Whitespace away from a break | `a  b` | Two spaces; existing test passes | Same; clean control. |

Browser output retains the extra text spaces even where HTML display collapses them. Terminal soft-break lowering emits a space too, so the tree's extra whitespace is visible there. Normalize neighboring inline nodes across wrapper boundaries while preserving styling and whitespace away from breaks; extend the existing whitespace table with every wrapper branch.

### High: New terminal rendering behavior lacks Level 2 verification

**Defect class:** new terminal display contracts are verified only with generated nodes or bytes, without checking the real terminal's displayed cells.

The **biscuit-terminal-cli** [prose tests](../../cli/tests/level2/level2_prose_styling.rs), [table tests](../../cli/tests/level2/prose_cells.rs), and [container code tests](../../cli/tests/level2/level2_container_fenced_code.rs) still use explicit styling or block-code forms. Source inspection of every library and CLI Level 2 module found no backtick-span scenario. Neither the library nor CLI Level 2 suite adds backtick-span scenarios for this feature.

| Requirement / site | Shape inspected | Strongest relevant verification | Expected verification |
|---|---|---|---|
| Standalone inline code | Backtick span is dim and delimiters disappear | Level 1 tree and byte assertions | Level 2 displayed text and style capture. |
| Code in colored prose | Parent color resumes after code | Level 1 byte assertions | Level 2 style capture covering code and following text. |
| Code inside a hyperlink | Style and hyperlink extent survive | Level 1 byte assertions | Level 2 cell/style/link capture. |
| Unstyled fallback | Safe delimiters remain | Level 1 configured-capability output | Level 2 visible fallback text. |
| Table inline code and normalized fence | Code survives borders, width, and wrapping | Level 1 cell/tree output | Level 2 cell capture, including no style bleed. |
| Paragraphs and soft/hard breaks | New block/reflow geometry | Level 1 grammar assertions | Level 2 representative geometry capture. |
| Existing explicit tags and ordinary links | Existing real-terminal inputs | Level 2 tests exist | Appropriate level; clean siblings. |
| Existing explicit block-code styling | Existing colored block inputs | Level 2 tests exist | Appropriate level; does not prove inline spans. |
| Browser tags/layout, wrapper-free fragments, tree validity | Public structural output | Level 1 | Appropriate for serialized output contracts. |
| Code-span parsing, Markdown fences/breaks, link helper/catalog | Public parser/serializer/API results | Level 1 | Appropriate level; output defects above remain. |
| Keyboard input | No new keyboard behavior | No feature-specific Level 3 | No Level 3 required. |

Add focused cases to the existing terminal capture suites. Keep windows from gaining focus. The targets and module declarations are live; `terminal-tests` is enabled by the Level 2 recipe and CI metadata. The final tier-coverage check found no stranded tests. This finding concerns missing coverage, not missing cross-OS evidence or a human visual sign-off.

### High: Single-newline callers still lose their line structure

**Defect class:** callers that use single newlines as display separators still pass their content through default soft-break prose, joining previously separate display rows.

In **claudine**, the [lifecycle diagnostics](../../../claudine/lib/src/composition/error/render/lifecycle.rs), [provider diagnostics](../../../claudine/lib/src/composition/error/render/provider.rs), [schema diagnostics](../../../claudine/lib/src/composition/error/render/schema.rs), and [sequence diagnostics](../../../claudine/lib/src/composition/error/render/sequence_loop.rs) build line-oriented lists but supply ordinary strings to `StatusBlock::body`. That conversion uses default `Prose`. Pre-rendered lists passed back through this body parser have the same problem.

The public component probe copied each diagnostic builder's list shape, supplied two entries, and rendered through `StatusBlock::body`. Every affected row below joins its header/items in one paragraph without `<br>`. For example, `"Tried:\n  1. one.md\n  2. two.md"` becomes `Tried: 1. one.md 2. two.md`. These probes exercise the production component conversion; they do not constitute end-to-end tests of all error variants.

| Site | Shape tested / inspected | Observed result | Expected result |
|---|---|---|---|
| Lifecycle invalid-property expected fields | Header plus two field rows | Header and fields joined | Separate rows. |
| Lifecycle invalid file-reference candidate plan | Header plus two numbered paths | Numbered paths joined | Ordered paths on separate rows. |
| Provider empty-prompt overrides | Header plus two override keys | Keys joined | Separate rows. |
| Provider magic-search roots | Two rendered list rows after message | Root rows joined on body reparse | Separate rows. |
| Provider candidate paths | Provenance labels plus two paths | Paths joined after `Tried:` | Separate rows. |
| Provider suggestions | Two paths after `Did you mean:` | Suggestions joined | Separate rows. |
| Schema validation problems | Header plus two problems | Problems joined | Separate rows. |
| Schema missing typed properties | Two property/type rows | Properties joined | Separate rows. |
| Schema pointer-path fallback | Two pointer rows | Pointers joined | Separate rows. |
| Sequence description | Description after step heading | Description joins heading | Preserve description row. |
| Sequence missing typed properties | Two property rows after step heading | Properties join heading | Separate rows within each step. |
| Sequence pointer-path fallback | Two pointer rows after step heading | Pointers join heading | Separate rows within each step. |
| Selection diagnostics and dispatcher failure suffix | No list, or blank-line paragraph separators | No affected list separator found | Clean sibling builders. |
| Public paragraph control | `First\n\nSecond` | Separate paragraphs retained | Same; clean control. |
| Claudine CLI [help title](../../../claudine/cli/src/commands/help.rs) | Source inspection plus copied title LF description | `Claudine Description` on one terminal row | Preserve title/description rows. |
| Claudine CLI [error detail report](../../../claudine/cli/src/commands/logs/errors.rs) | Source inspection plus two copied detail rows | `field: one field: two` on one terminal row | Preserve each detail row. |
| Claudine CLI [hook action cell](../../../claudine/cli/src/commands/hooks/list.rs) | Source inspection plus two copied action rows | `action one action two` before table conversion | Preserve action rows, using inline content in hard mode. |
| Claudine CLI [provider error report](../../../claudine/cli/src/output/error_report.rs) | Source inspection plus summary/footer LF and leading-LF composition probes | `summary footer` and `summarydetail` | Preserve the existing display structure. |
| Darkmatter shell-expansion diagnostics and YAML excerpts | Source inspection of the spec's named migration clusters | Explicit hard-break mode is present | Clean migrated siblings. |

Use `Prose` in hard-break mode for existing line-oriented block strings and `InlineProse` in hard mode for inline cells; use explicit composition spacing where a leading LF previously added a row. This follows the existing migration requirement and does not require redesigning pre-rendered container callers. Add output assertions for row boundaries and item counts, rather than substring-only assertions that pass after every item collapses onto one line.

This class was first reported in review 2 and remains reproducible. The implementation log notes the pre-rendered-list collapse without recording a completed fix. The probes copy the builders’ line-oriented shapes and exercise the public component conversion; they do not prove every error variant end to end.

## Blocked Findings

None. All six fixes follow the existing specification. The human review item concerns the implementation cycle, not a missing design decision or permission to fix code.

## Input Robustness Matrix

The added serialized field is **renderable**’s JSON `browser.block_element`, which chooses a paragraph’s HTML tag. A temporary public-API probe copied the production-serialized paragraph fixture from the [legacy compatibility test](../../../renderable/src/tree/render/browser.rs), changed only this field, deserialized public `RenderNode`, and rendered accepted inputs through the public browser renderer. There is no new manifest, lockfile, or configuration reader. Adding `code_link` does not change the expression catalog reader.

| Shape | JSON `browser.block_element` public result |
|---|---|
| Unedited positive control, `"div"` | `<div data-role="note">x</div>` |
| Absent | `<p data-role="note">x</p>`; absence is the defined legacy default. |
| Explicit null | Rejected. |
| Wrong whole-field type, `123` | Rejected. |
| Wrong one element, `["div",123]` | Rejected; this field is a scalar enum. |
| Wrong every element, `[123]` | Rejected. |
| Empty array / object / string | Each rejected. |
| Duplicate key | Rejected as duplicate `block_element`. |
| Valid document plus garbage | Rejected as trailing content. |

The checked-in compatibility test passes and covers absent, explicit valid, unknown enum, and wrong scalar values. The remaining malformed shapes were checked by the review probe rather than retained as a full regression matrix. No permissive parsing defect was found. Preserve the complete table in one public-output regression when extending this field.

## Verification

| Check | Result |
|---|---|
| Biscuit-terminal: `just test prose_markdown_escaping::` | Six passed. Both components and both Markdown dialects are parsed back independently. |
| Renderable: `just test literal_backslash_tests::` | 15 passed, including wrapper edge delimiters and table/title contexts. |
| Biscuit-terminal: `just test prose_grammar::` | 40 passed. These tests omit the failing wrapper-boundary and explicit-code preprocessing cases. |
| Biscuit-terminal: `just test prose_containers::` | Ten passed. These tests omit explicit block code inside a link. |
| Renderable: `just test paragraph_without_block_element_field_keeps_p` | One passed. |
| Temporary public-API example | Reproduced the opacity, wrapper/container validity, relative-link, whitespace, diagnostic-row, and JSON tables above. |
| Shipped `bt prose '[a<code-block>x</code-block>b](https://e.io)' --html` | Printed a render-tree validation error; process status zero. |
| Claudine: `just test nested_span_error_renders_property_literal_rewrite_and_escape_hint` | One passed; the original diagnostic escaping regression remains fixed. |
| Root: `just check-tier-coverage biscuit-terminal` | Passed; zero stranded tests. |

The new `prose_markdown_escaping` module is declared in the library’s consolidated `l1` target, has no tier marker, and ran through the L1 recipe. The existing grammar/container modules also ran. The Level 2 modules are declared in the terminal targets, require `terminal-tests`, and have live recipes and CI feature declarations. Their absence of feature-specific scenarios is a coverage defect, not a declaration defect.

No implementation tests were added or renamed. The temporary example was removed. The terminal suites were inspected rather than executed because they omit the new scenarios. Full affected-area suites, lint, browser automation, and cross-OS execution were not repeated; narrow passing tests do not establish production readiness. The claudine test build emitted a linker warning about the large unwind section but completed successfully.

## Requirement-to-verification mapping

Criterion numbers below refer to the numbered Acceptance Criteria in the specification; their meaning is included so the numbers need not be looked up to understand the result.

| Requirements | Verification present | Assessment |
|---|---|---|
| 1: portable relative-link example | Level 1 public HTML/Markdown reproduction | Fails; relative destinations become absolute file URLs. Terminal appearance also belongs to Level 2. |
| 2–7, 13, 26: soft/hard breaks, paragraphs, line-oriented caller migration, line endings | Level 1 grammar and independent-reader tests; caller probes | Core grammar controls pass. Caller migration fails. New terminal geometry lacks Level 2 scenarios. |
| 8–12, 25, 27–28, 31: paragraph tags, valid block shapes, layout, empty content, wrapper-free fragments, containers, fences | Level 1 public tree/HTML/Markdown tests and browser serializer tests | Controls pass; explicit code under links and preprocessing remain defective. New terminal geometry and table-code appearance lack Level 2 scenarios. |
| 14, 16–18, 20: inline-code projection, literal contents, removal of the link rewrite, escaping and spaces | Level 1 grammar/output tests and existing diagnostic regression | Ordinary code-span controls pass; explicit code bodies still lose opacity. |
| 15, 21, 29: terminal inline-code styling, fallback, style restoration and hyperlink extent | Level 1 configured-terminal bytes and trees | Level 2 cell/style/link captures are missing; high finding above. |
| 19, 30: safe Markdown fences and hard-break output in both dialects | Level 1 direct-renderer and public component tests | Appropriate verification level; corrected backslash regressions pass. |
| 22–23: `code_link`, migrated templates, catalog and completion | Level 1 tests present in darkmatter and claudine; runtime/catalog/template sources inspected | Appropriate level for expression and serialized-output contracts. Not rerun in this review. |
| 24: affected-area tests, lint, examples and benchmarks | Focused L1 runs in this review; earlier full-area runs recorded in the implementation log | This review does not claim a fresh full-area validation. |
| Keyboard input | No new keyboard contract | No Level 3 requirement. |

The scope excludes changing how terminal inline code itself wraps; this review requests Level 2 coverage of its appearance within existing table and container geometry, not a new wrapping design.
