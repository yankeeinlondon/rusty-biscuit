---
$schema: feature-review.yaml
ready: false
findings:
    - title: Diagnostic call sites still escape literal code-span contents
      priority: high
    - title: Markdown serialization changes literal backslashes and break meaning
      priority: high
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
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-03T14:01:52-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-1.md
next: 2026-10-02-inline-prose/review-2.md
---

# Inline Prose — Review 1

The feature is **not production ready**. The component split, paragraph tags, structural container embedding, and `code_link()` are implemented, but the sweeps below found incorrect output and incomplete migration. No finding requires a new human design decision; the specification already states the relevant contracts.

This review covers the current working tree, including its preexisting uncommitted implementation changes. No implementation code was changed. Temporary public-API probes were removed after use. This is the first implementation review for this feature; findings in its implementation log are not earlier review iterations.

## Findings

### High: Diagnostic call sites still escape literal code-span contents

**Defect class:** callers apply prose escaping to values that the new code-span grammar treats literally, so diagnostics show added backslashes in identifiers, paths, and suggested expressions.

In the **claudine** package, [the lifecycle diagnostic formatter](../../../claudine/lib/src/composition/error/render/lifecycle.rs), [schema diagnostic formatter](../../../claudine/lib/src/composition/error/render/schema.rs), [selection diagnostic formatter](../../../claudine/lib/src/composition/error/render/selection.rs), and [provider diagnostic formatter](../../../claudine/lib/src/composition/error/render/provider.rs) retain this pattern. The shared [escape_prose_path helper](../../../claudine/lib/src/composition/error/render/mod.rs) now delegates to `Prose::escape_text`, so searching for that method alone misses most siblings. This violates the literal-content and call-site migration requirements.

The existing `nested_span_error_renders_property_literal_rewrite_and_escape_hint` test was run through the claudine area's `just test` recipe and **failed**. Its real fixture's diagnostic shows `\{\{ctx.area}}` instead of `{{ctx.area}}`. The same escaping remains on both the nested expression and the enclosing literal.

The sibling sweep copied each formatter's actual string into a temporary probe, supplied `_a_[x]` through the site's escaping operation, and rendered it through biscuit-terminal's public `Prose` API. All 23 formatters below emitted code containing `\_a\_\[x\]`; each should show `_a_[x]` without added backslashes. Named context fields in the copied templates were also filled for the probe; the finding concerns the explicitly escaped arguments listed here.

| Site in claudine's diagnostic renderers | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `lifecycle.rs:86` — nested expression, enclosing literal, illustrative braces | Escaped code-label value | Added backslashes; real fixture also fails | Literal values |
| `lifecycle.rs:122` — undefined variable | Escaped `_a_[x]` | Added backslashes | Literal value |
| `lifecycle.rs:150` — evaluation property | Same | Added backslashes | Literal value |
| `lifecycle.rs:183` — removed validation key | Same | Added backslashes | Literal value |
| `lifecycle.rs:236` — short-form action | Same | Added backslashes | Literal value |
| `lifecycle.rs:403` — set destination key | Same | Added backslashes | Literal value |
| `lifecycle.rs:432` — whole-mapping interpolation | Same | Added backslashes | Literal value |
| `lifecycle.rs:457` — dynamic mapping key | Same | Added backslashes | Literal value |
| `lifecycle.rs:480` — proxy target | Same | Added backslashes | Literal value |
| `lifecycle.rs:552` — handoff target | Same | Added backslashes | Literal value |
| `lifecycle.rs:585` — each chain entry | Same | Added backslashes | Literal value |
| `lifecycle.rs:588` — repeated target | Same | Added backslashes | Literal value |
| `lifecycle.rs:685` — removed action and replacement expression | Same, both arguments | Added backslashes | Literal values |
| `lifecycle.rs:826` — unresolved reference | Same | Added backslashes | Literal value |
| `selection.rs:33` — no-match query | Same | Added backslashes | Literal value |
| `selection.rs:43` — over-limit query | Same | Added backslashes | Literal value |
| `selection.rs:67` — canceled query | Same | Added backslashes | Literal value |
| `schema.rs:45` — property name | Same | Added backslashes | Literal value |
| `provider.rs:120` — magic reference payload | Same | Added backslashes | Literal value |
| `provider.rs:130` — search-root path | Same | Added backslashes | Literal value |
| `provider.rs:158` — reference and launch directory | Same, both arguments | Added backslashes | Literal values |
| `provider.rs:183` — candidate path | Same | Added backslashes | Literal value |
| `provider.rs:204` — suggestion path | Same | Added backslashes | Literal value |
| `lifecycle.rs:51,154,188` and `sequence_loop.rs:122,129,140` — text outside code | Escaped value outside backticks | Escapes resolve normally | Same; clean controls |
| biscuit-terminal's literal code-span API | Unescaped `_a_[x]` inside backticks | Literal value | Same; clean control |
| darkmatter's `code_link()` | Brackets and backslashes in labels | Literal labels in both parsers; integration tests passed | Same; clean sibling |

Remove escaping specifically from closed code-span contents. Preserve escaping for ordinary prose and attribute values. Use the shared safe code-fence helper when a dynamic label can contain backticks. Add diagnostic assertions for underscore, bracket, brace, and backslash values; correcting snapshots alone would conceal the defect.

### High: Markdown serialization changes literal backslashes and break meaning

**Defect class:** the Markdown renderer concatenates unescaped literal text with break markers, changing the visible backslash count or turning a soft break into a hard break.

In **renderable**, [the Markdown text and break renderer](../../../renderable/src/tree/render/markdown.rs) emits `Text` verbatim outside HTML. Both biscuit-terminal components use this path. The implementation log already identifies the soft-break case, but it remains unresolved. The escaped-backslash test in [biscuit-terminal's public grammar tests](../../lib/tests/l1/prose_grammar.rs) checks HTML only.

For an unambiguous reproduction, these are **Rust string literals**:

```rust
let soft = Prose::new("a\\\\\nb");
assert_eq!(soft.render_html_fragment().render(), "<p>a\\ b</p>");
// Actual Markdown: one literal backslash immediately followed by a newline.
// A Markdown reader treats that as a hard break and consumes the backslash.
```

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `Prose`, soft mode | Two source backslashes before one newline | Markdown has one backslash plus newline | Escaped literal backslash plus soft break |
| `InlineProse`, soft mode | Same | Same defect | Same preservation |
| Both components, hard mode | Same source, hard mode | Two output backslashes plus newline; reparses as a literal backslash and soft break | Literal backslash followed by a hard break |
| Both components, either mode | Three source backslashes before newline | Same two-backslash output; hard break is lost | Literal backslash plus hard break |
| `Prose`, either mode | Escaped backslash before a paragraph boundary | One output backslash before the blank line | Preserve the backslash without creating a break marker |
| Direct `RenderNode` Markdown and MarkdownPlus | `Text("a\\")` followed by soft/hard break | Same incorrect concatenation | Preserve text and break independently |
| Table-cell Markdown and MarkdownPlus | Literal backslash followed by soft/hard break | Space / `<br>`; no backslash-newline collision | Same; clean controls for this defect |
| Both components | Ordinary soft break; explicit hard break without a literal backslash; trailing backslash at end of input | Correct break or trailing literal | Same; clean controls |
| Terminal and HTML | The escaped-backslash cases above | Correct literal and break structure | Same; clean targets |

Fix escaping at the shared renderer boundary so adjacent `Text` nodes and nested inline wrappers cannot bypass it. Verify the rendered Markdown by parsing it and asserting visible text and break structure in both dialects, including table controls. Comparing output strings alone is insufficient here.

### High: Opaque code bodies and quoted attributes are altered by preprocessing

**Defect class:** preprocessing transforms regions that should be copied literally, allowing generated markup or internal placeholder characters to escape into public output.

In **biscuit-terminal**, [the preprocessing pipeline](../../lib/src/components/prose/markdown.rs) lifts fenced blocks but does not lift explicit `<code-block>` bodies before inline processing. [The token parser](../../lib/src/components/prose/tokens.rs) later treats the transformed body as literal code. The paragraph splitter recognizes explicit code as opaque, but that protection does not cover the earlier inline transformation. Fence lifting also scans lines inside quoted attributes without respecting their boundaries.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `Prose` explicit `<code-block>` | Body containing backtick-delimited `x` | `<pre><code>` contains U+0002, index `0`, U+0002 | Original backticks and `x` |
| `InlineProse` explicit `<code-block>` | Same body | `<code>` contains the same internal placeholder | Original body as one inline-code value |
| Both explicit-code paths | Body `**x**` | Literal generated `<b>x</b>` | Literal `**x**` |
| Both explicit-code paths | Body `[x](https://e.io)` | Literal generated `<a href="https://e.io">x</a>` | Original Markdown syntax |
| Both explicit-code paths | Literal U+0002 characters | Extra escape backslashes in body | Original characters |
| Both explicit-code paths | Ordinary multiline `x` / `y` | Block retains newline; inline becomes `x y` | Same; clean control |
| Both fenced-code paths | Same backticks, emphasis, and link syntax inside a triple-backtick fence | Original contents preserved | Same; clean siblings |
| Both quoted-attribute paths | `<a href="x\n```\ny\n```\nz">t</a>` | Destination contains a lifted-fence placeholder; body `y` disappears | Preserve attribute content; never treat it as a fence |
| Quoted attribute without fence-looking lines | Newlines within quoted `href` | No paragraph split | Same; clean control covered by existing L1 test |

Recognize every opaque region before transformation, with scope-aware scanning. Add public render-result tests for explicit and fenced code using the same body matrix, and quoted attributes containing fence-looking lines. Updating the comments alone would contradict the specified opacity contract.

### High: Explicit code blocks inside links invalidate the entire block output

**Defect class:** one inline wrapper retains a block child instead of splitting around it, creating a tree that the shared renderers reject.

In **biscuit-terminal**, [the link arm of the token parser](../../lib/src/components/prose/tokens.rs) wraps all parsed children in `Link`; [styled wrappers](../../lib/src/components/prose/tree.rs) already split around `Code`. Reproduction:

```rust
Prose::new("<a href=\"https://e.io\">a<code-block>x</code-block>b</a>")
    .render_html_fragment().render(); // actual: ""
```

Validation reports `block-level Code node inside phrasing-only Link container`. A single invalid child causes the standalone HTML output to disappear.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Standalone `Prose` explicit link | Explicit code block between linked `a` and `b` | Invalid tree; empty HTML | Linked paragraphs around a sibling code block |
| Markdown-generated link through `bt prose --html` | `[a<code-block>x</code-block>b](https://e.io)` | CLI emits a render-tree validation error instead of content | Valid linked paragraphs around a sibling code block |
| `UnorderedList` | Same `Prose` component | Same validation error | Valid embedded blocks |
| `OrderedList` | Same | Same validation error | Valid embedded blocks |
| `BlockQuote` | Same | Same validation error | Valid embedded blocks |
| `TwoColumn` | Same | Same validation error | Valid embedded blocks |
| `StatusBlock::body` | Same | Same validation error | Valid embedded blocks |
| `Section` | Same | Same validation error | Valid embedded blocks |
| `Compose` | Same | Same validation error | Valid embedded blocks |
| `Prose` bold, color, underline, transparent clipboard wrappers | Same explicit block | Valid sibling blocks | Same; clean wrapper branches |
| Each of the seven block containers above | Red wrapper instead of link | Valid tree | Same; clean container controls |
| `Prose` explicit link surrounding a fenced block | Triple-backtick form | Valid linked paragraphs and sibling code | Same; clean alternate syntax |
| `InlineProse` explicit link | Explicit or fenced code | Valid link containing inline code | Same; clean component sibling |

Apply the existing split-and-resume behavior to links as well as styles, preserving each link destination on the inline runs. Add the explicit-code case to the shared wrapper and container tests.

### Medium: Relative link destinations do not meet the specified portable output

**Defect class:** target-neutral parsing resolves relative destinations against the local checkout, making browser and Markdown output machine-specific.

In **biscuit-terminal**, [the shared link parser](../../lib/src/components/prose/tokens.rs) calls [resolve_href](../../lib/src/components/prose/styles.rs) before choosing a target. The exact first acceptance example outputs a `file:///Volumes/coding/wt/rusty-biscuit/fix-magic-globs/plan.md` destination in both Markdown and HTML, instead of `plan.md`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `InlineProse` Markdown link | `[the plan](plan.md)` | Absolute checkout URL in Markdown and HTML | Preserve `plan.md` |
| `Prose` Markdown link | Same relative destination | Same rewriting | Preserve relative destination |
| Both components' explicit `<a>` form | `href="plan.md"` | Same rewriting | Preserve relative destination on portable targets |
| Both components | `https://e.io` destination | Unchanged | Same; clean control |
| Terminal projection | Relative destination | Resolved file link | Keep a usable terminal file link |

This behavior predates the feature and is acknowledged as an unresolved departure in the implementation log. It still fails the feature's explicit acceptance criterion: replacing the test input with an HTTPS URL does not prove that example. Preserve authored destinations in the tree and perform terminal-specific file resolution when rendering terminal links. Use the repository's file-reference resolver for that resolution.

### Medium: Soft-break whitespace trimming stops at inline wrapper boundaries

**Defect class:** whitespace normalization operates on each recursive parser buffer independently, leaving extra spaces when a soft break touches a style or link boundary.

In **biscuit-terminal**, [the soft-break branch](../../lib/src/components/prose/tokens.rs) trims only its local `text` buffer. The specification requires spaces and tabs immediately around a prose soft break to be discarded.

| Site | Shape tested | Observed result in both components | Expected result |
|---|---|---|---|
| Bold wrapper | `a <b> \nb</b>` and `<b>a \n</b> b` | `a  b` | `a b` |
| Color wrapper | Same shapes with `<red>` | `a  b` | `a b` |
| Underline wrapper | Same shapes with `<u>` | `a  b` | `a b` |
| Link wrapper | Same shapes with `<a href="https://e.io">` | `a  b` | `a b` |
| Transparent wrapper | Same shapes with `<clipboard>` | `a  b` | `a b` |
| Unwrapped text | `a \t\n\t  b` | `a b` | Same; clean control |
| Text away from a break | `a  b` | Two spaces preserved | Same; clean control |

The values above were checked through terminal rendering with escapes stripped; the browser fragments also retain the extra text spaces. Normalize whitespace across neighboring inline nodes while keeping their styles and preserving whitespace away from breaks. Add wrapper-boundary cases to the existing public whitespace table.

### High: New terminal rendering behavior lacks Level 2 verification

**Defect class:** terminal-visible contracts are checked only against generated trees or renderer bytes, leaving the actual terminal display path unverified.

The **biscuit-terminal-cli** [real-terminal prose tests](../../cli/tests/level2/level2_prose_styling.rs) exercise explicit tags and block code. [The table tests](../../cli/tests/level2/prose_cells.rs) exercise explicit dim tags and ordinary links. Neither supplies a backtick code span through the new grammar. An assertion that a fenced block is dim cannot prove that inline code is dim, restores its enclosing style, or preserves its hyperlink.

| Site / user-facing requirement | Shape inspected | Strongest relevant verification present | Expected verification |
|---|---|---|---|
| Standalone inline code | Backtick span rendered dim, without delimiters | L1 grammar/output assertions | L2 pane capture of the parsed span and its style |
| Inline code inside a colored sentence | Parent color resumes after code | L1 exact renderer bytes | L2 capture checks code and text after it |
| Inline code inside a link | Code appearance and surrounding hyperlink survive | L1 exact renderer bytes | L2 capture checks both displayed code styling and link extent |
| Unstyled fallback | Safe backtick delimiters remain | L1 capability-configured rendering | L2 visible text under unstyled output |
| Table-cell inline code / normalized fence | Inline code survives the cell's width and border handling | L1 grammar/table tests | L2 cell capture with no style bleed |
| Paragraph boundaries and single-newline reflow | New block/soft-break geometry | L1 grammar tests; existing L2 wrapping tests use other inputs | L2 capture of representative new paragraph and break inputs |
| Existing explicit bold/color/dim tags and ordinary links | Existing table payloads | L2 capture tests exist | Correct level; clean sibling coverage |
| Existing fenced block style restoration | Explicit `<code-block>` in colored text | L2 capture tests exist | Correct level; clean sibling coverage |
| Browser tags, tree validity, Markdown fencing, expression catalog | Structural output and public API contracts | L1 tests | Appropriate for these contracts |
| Keyboard behavior | No requirement added | No new L3 test | L3 is unnecessary for this feature |

Add focused tests to the existing real-terminal suite using its pane-capture and style-state helpers. Keep terminal windows from gaining focus. This is missing test coverage, not a demand for cross-OS evidence or a human visual sign-off. Existing tier recipes are live, and `just check-tier-coverage biscuit-terminal` reported no stranded tests.

## Input robustness matrix

The newly added load-bearing serialized field is renderable's JSON paragraph `browser.block_element`. A temporary matrix probe copied the production-serialized paragraph fixture used by [the browser compatibility test](../../../renderable/src/tree/render/browser.rs), changed one field per row, deserialized through public `RenderNode`, and inspected public HTML output on successful reads.

| Shape | `browser.block_element` — JSON result |
|---|---|
| Unedited positive control: `"div"` | `<div data-role="note">x</div>` |
| Absent | `<p data-role="note">x</p>`; the format explicitly defines this legacy default |
| Explicit null | Rejected |
| Wrong whole-field type: `123` | Rejected |
| Wrong element in mixed array: `["div",123]` | Rejected; field is a scalar enum |
| Every element wrong: `[123]` | Rejected |
| Empty array / object / string | Each rejected |
| Duplicate key | Rejected as duplicate `block_element` |
| Valid JSON followed by garbage | Rejected as trailing characters |

No permissive parse defect was found in this field. Existing checked-in tests cover the legacy default, valid tags, unknown tag, numeric wrong type, fragment/streaming agreement, and invalid node placement. They do not yet retain the entire matrix above in one test; consolidate these cases when extending serialization coverage. The prose grammar and expression function do not introduce a manifest, lockfile, or configuration-file reader.

## Verification and scope

| Command | Result |
|---|---|
| biscuit-terminal area: `just test prose_grammar::` | 38 passed |
| biscuit-terminal area: `just test` | 3,469 passed; 57 excluded by tier selection |
| renderable area: `just test` | 564 passed |
| darkmatter area: `just test code_link` | 16 passed; unrelated tests filtered out |
| claudine area: `just test nested_span_error_renders_property_literal_rewrite_and_escape_hint` | One failed; failure reproduced above |
| biscuit-terminal area: `just lint` | Passed for library and CLI |
| Repository root: `just check-tier-coverage biscuit-terminal` | No stranded tests |
| Temporary example using public prose, render-tree validation, Markdown, browser, and terminal APIs | Reproduced each output defect and its clean controls |
| `cargo run -p biscuit-terminal-cli -- prose --html` with a Markdown link containing explicit code | Shipped CLI reproduced the block-in-link validation error |

The grammar, container, and code-link integration modules are declared by their consolidated L1 test binaries. The terminal test targets are declared behind `terminal-tests`, their recipes enable that feature, and the feature is listed in package CI metadata. No implementation tests were added or renamed during this review.

The Level 2 suite was inspected, not executed: it contains no tests of the new backtick-span scenarios needed for the finding. Browser automation and full downstream package suites were not run. Passing results above establish only the stated scopes; they do not establish all acceptance criteria. Cross-OS proof is left to CI and does not affect this review's readiness decision.

The implementation log also records two user-home prompt copies that still need their backtick-wrapped `link()` calls changed to `code_link()`. Those copies are outside the repository and were not modified in this review; the four repository template copies passed their integration test.
