---
$schema: feature-review.yaml
ready: false
findings:
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
    - title: Single-newline callers still lose their line structure
      priority: high
human_review: true
human_review_items:
    - |-
        Six defect classes from review 1 remain reproducible after its implementation cycle. Before another automatic cycle, review why only the diagnostic escaping finding was completed and confirm that the next implementation addresses every instance table below. No new product design decision is needed; the remaining changes follow the existing specification.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-03T15:31:15-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-2.md
previous: 2026-10-02-inline-prose/review-1.md
next: 2026-10-02-inline-prose/review-3.md
---

# Inline Prose — Review 2

The feature is **not production ready**. The diagnostic escaping fix works at the reviewed sites, but six findings from review 1 remain reproducible. The migration sweep also found callers whose lists and display lines collapse under the new default newline behavior.

This review covers the current working tree, including implementation changes already present when the review began. Only review documents and the requested lifecycle metadata were changed permanently. Temporary public-API examples were removed. Cross-OS evidence is left to CI and does not affect this readiness decision.

## Previous review follow-through

Review 1 has a single `Findings` section, rather than separate unblocked and blocked sections. Its metadata declares no blocked findings and no required human review. All seven findings were therefore actionable; there were no blocked findings to become unblocked.

| Review 1 finding | Current result |
|---|---|
| Diagnostic call sites still escape literal code-span contents | Addressed at all 23 enumerated formatter sites. Lifecycle, schema, provider, and selection formatters now use safe code-span construction. The original nested-expression regression and the lifecycle/selection literal-value tests pass. |
| Markdown serialization changes literal backslashes and break meaning | Still reproducible; finding below. |
| Opaque code bodies and quoted attributes are altered by preprocessing | Still reproducible; finding below. |
| Explicit code blocks inside links invalidate the entire block output | Still reproducible in standalone output and all seven block containers. |
| Relative link destinations do not meet the specified portable output | Still reproducible in both components and both authored link forms. |
| Soft-break whitespace trimming stops at inline wrapper boundaries | Still reproducible in all five wrapper branches. |
| New terminal rendering behavior lacks Level 2 verification | No new tests for the missing scenarios were found. |

The escaping fix also covers **claudine-gen**'s unrecognized-answer formatter, **darkmatter-cli**'s trigger-grammar formatter, and **biscuit-terminal**'s colorless list reparse. The new shared `escape_text_outside_code_spans` helper preserves code in whole messages. These are useful extensions of the corrected class. Source inspection confirms the original 23 replacements; executed tests establish the scopes listed under Verification, rather than every downstream diagnostic.

The requested `implemented: true` on review 1 records that its implementation cycle finished; it does **not** mean every finding was resolved.

## Recurrence

Every earlier review file in this directory was examined: review 1 is the only predecessor. The first six findings below repeat its classes and titles exactly.

| Earlier finding in review 1 | Sites its implementation should have covered | Sweep performed again |
|---|---|---|
| Markdown serialization changes literal backslashes and break meaning | Shared text/break lowering, both components, both break modes and Markdown dialects, inline wrappers, tables | Public component output plus direct render-tree output; table and wrapper controls included. |
| Opaque code bodies and quoted attributes are altered by preprocessing | Explicit and fenced code in both components; quoted attributes before fence lifting | Same body matrix across both syntaxes and components; fence-looking attribute lines included. |
| Explicit code blocks inside links invalidate the entire block output | Link projection alongside style/transparent projection; every block container | Both code syntaxes, five wrapper branches, both components, and all seven containers. |
| Relative link destinations do not meet the specified portable output | Markdown links and explicit anchors in both components, portable and terminal targets | Both authored link forms through public HTML and Markdown output; HTTPS control and terminal resolution inspected. |
| Soft-break whitespace trimming stops at inline wrapper boundaries | Bold, color, underline, link, transparent wrappers, and ordinary text | Both boundary directions in both components for every wrapper branch; existing unwrapped controls run. |
| New terminal rendering behavior lacks Level 2 verification | Standalone code, enclosing color/link, fallback, cells, paragraph/break geometry | Both library and CLI terminal suites, target declarations, features, and recipes inspected. |

The remaining work was not a missed isolated sibling: these six classes were already enumerated and remain unresolved. `recurrence: true` calls for a human check of the incomplete cycle before further automatic iteration. The code fixes themselves are unblocked.

## Unblocked Findings

### High: Markdown serialization changes literal backslashes and break meaning

**Defect class:** literal text is serialized without Markdown escaping, so adjacent break or wrapper syntax consumes backslashes or changes formatting.

In **renderable**, [the text renderer](../../../renderable/src/tree/render/markdown.rs) still returns `value.to_string()` outside inline HTML. Changing the hard-break marker alone cannot preserve a literal backslash next to it. This affects both **biscuit-terminal** components.

For example, `Prose::new("a\\\\\nb")` produces HTML `<p>a\ b</p>` but Markdown containing one backslash immediately before a newline. A Markdown reader consumes that backslash as a hard-break marker. In hard mode the two output backslashes instead encode a literal backslash followed by a soft break.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `Prose`, soft mode | Two source backslashes then LF | One output backslash then LF | Preserve literal backslash and soft break independently. |
| `InlineProse`, soft mode | Same | Same defect | Same preservation. |
| Both components, hard mode | Same | Two output backslashes then LF | Literal backslash plus hard break. |
| Both components, either mode | Three source backslashes then LF | Two output backslashes then LF | Literal backslash plus explicit hard break. |
| `Prose`, either mode | Two source backslashes then blank line | One output backslash before paragraph separator | Preserve literal backslash. |
| Direct tree, Markdown and MarkdownPlus | `Text("a\\")`, soft/hard break, `Text("b")` | Same one/two-backslash collisions | Preserve text and break semantics. |
| Direct strong wrapper, both dialects | `Strong[Text("a\\")]` before either break | `**a\**` precedes the break; the backslash escapes a closing emphasis delimiter | Literal backslash inside valid emphasis. |
| Direct table cell, both dialects | Plain text ending in backslash before soft/hard break | Space / `<br>` avoids the newline collision | Same; clean for the break collision. |
| Strong wrapper in a table cell, both dialects | Same wrapped text | `**a\**` still corrupts closing emphasis | Escape text independently of wrapper delimiters. |
| Both components | Ordinary soft break, explicit hard break, trailing backslash | Correct output for these controls | Same. |
| Browser output | Escaped-backslash cases | Correct literal text and break structure | Same; clean target. |

Fix text serialization at the shared renderer boundary. Test adjacent text nodes and wrappers as well as direct text/break pairs, and parse the resulting Markdown with an independent Markdown parser to assert visible text and break types. Existing tests checking only HTML miss this defect.

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

The **biscuit-terminal-cli** [prose tests](../../cli/tests/level2/level2_prose_styling.rs), [table tests](../../cli/tests/level2/prose_cells.rs), and [container code tests](../../cli/tests/level2/level2_container_fenced_code.rs) still use explicit styling or block-code forms. Neither the library nor CLI Level 2 suite adds backtick-span scenarios for this feature.

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
| Claudine CLI [help title](../../../claudine/cli/src/commands/help.rs) | Source inspection: title LF description | Default `Prose` retains a soft break | Preserve title/description rows. |
| Claudine CLI [error detail report](../../../claudine/cli/src/commands/logs/errors.rs) | Source inspection: `lines.join("\n")` | Default soft-break parse joins detail fields | Preserve each detail row. |
| Claudine CLI [hook action cell](../../../claudine/cli/src/commands/hooks/list.rs) | Source inspection: two actions joined by LF, pre-rendered using `Prose` | Soft-break output joins actions before table conversion | Preserve action rows, using inline content in hard mode. |
| Claudine CLI [provider error report](../../../claudine/cli/src/output/error_report.rs) | Source inspection: summary/footer LF, leading LF detail/hint, pre-rendered `Compose` body | Default prose joins lines and drops leading separator rows | Preserve the existing display structure. |
| Darkmatter shell-expansion diagnostics and YAML excerpts | Source inspection of the spec's named migration clusters | Explicit hard-break mode is present | Clean migrated siblings. |

Use `Prose` in hard-break mode for existing line-oriented block strings and `InlineProse` in hard mode for inline cells; use explicit composition spacing where a leading LF previously added a row. This follows the existing migration requirement and does not require redesigning pre-rendered container callers. Add output assertions for row boundaries and item counts, rather than substring-only assertions that pass after every item collapses onto one line.

This is a newly reported migration class. Review 1 discussed escaping in these diagnostic families but did not list their lost row structure as a finding. The implementation log itself notes the pre-rendered-list collapse, without recording a completed fix.

## Blocked Findings

None. All seven fixes can proceed using the existing specification. The human review item concerns recurrence and completeness of the implementation cycle, rather than a missing design decision.

## Input Robustness Matrix

The feature introduces one load-bearing serialized render-tree field: **renderable**'s JSON `browser.block_element`. The probe used the production-serialized paragraph fixture from [the legacy compatibility test](../../../renderable/src/tree/render/browser.rs), edited only this field, deserialized through public `RenderNode`, and rendered successful controls through the public browser renderer.

| Shape | JSON `browser.block_element` result |
|---|---|
| Unedited positive control: `"div"` | `<div data-role="note">x</div>` |
| Absent | Accepted with default paragraph element; existing executed prose tag tests and compatibility-test source establish `<p>`. Absence is explicitly defined as the legacy default. |
| Explicit null | Rejected. |
| Wrong whole-field type: `123` | Rejected. |
| Wrong one element: `["div",123]` | Rejected; the field is a scalar enum. |
| Wrong every element: `[123]` | Rejected. |
| Empty array / object / string | All rejected. |
| Duplicate key | Rejected as duplicate `block_element`. |
| Valid document followed by garbage | Rejected as trailing content. |

No permissive parsing defect was found. The checked-in compatibility test still retains only part of this matrix; consolidate the full table into one regression test when extending this field. No manifest, lockfile, or configuration-file reader was added by the prose grammar or link helper. The expression catalog's existing file-reader behavior was not changed by adding the function entry.

## Verification

| Check | Result |
|---|---|
| Biscuit-terminal area: `just test prose_grammar::` | 40 passed; 3,489 filtered out. |
| Claudine area: `just test nested_span_error_renders_property_literal_rewrite_and_escape_hint` | One passed; the review 1 regression is corrected. |
| Claudine area: `just test show_code_span_values_literally` | Two passed: lifecycle and selection diagnostic tables. |
| Claudine area: `just test candidate_no_match_shows_reference_paths_and_suggestions_literally` | One passed; candidate paths and suggestions remain literal. |
| Public-API temporary example | Reproduced break serialization, opacity, relative links, wrapper whitespace, diagnostic list collapse, and wrapper/container validity with the controls listed above. |
| Shipped `bt prose --html` with a Markdown link containing explicit code | Emitted `[render-tree error: render tree failed validation with 1 error(s)]`; process exit status was zero. |
| JSON field matrix in temporary example | Valid control rendered; absent accepted; all malformed shapes rejected. |
| Repository root: `just check-tier-coverage biscuit-terminal` | Final run passed: zero stranded tests. Initial run failed to list tests while the temporary example did not compile; removed that probe before the successful retry. |

The probe first failed to compile because it referred to an incorrect browser-renderer function name; that was corrected before any reported output was collected. The example was deleted after execution. No implementation tests were added or renamed.

The grammar and container test modules are declared in the consolidated L1 binary. The relevant Level 2 modules are declared in the feature-gated terminal binaries, and their recipes are live. The Level 2 suite was inspected rather than executed because it has none of the new scenarios required by the coverage finding. Full package suites, lint, browser automation, and cross-OS execution were not repeated in this review. Passing narrow tests do not establish production readiness.
