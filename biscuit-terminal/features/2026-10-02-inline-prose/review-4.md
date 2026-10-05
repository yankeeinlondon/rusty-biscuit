---
$schema: feature-review.yaml
ready: false
findings:
    - title: Wrapper scanning still interprets quoted and escaped tag text
      priority: high
    - title: Markdown serialization reinterprets literal text and entity spellings
      priority: high
    - title: Terminal capture tests depend on the shell displaying the full command
      priority: high
    - title: The serialized paragraph tag lacks a retained robustness matrix
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-03T20:51:02-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: true
implemented_by: claude/opus
log: biscuit-terminal/features/2026-10-02-inline-prose/log.md
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-4.md
previous: 2026-10-02-inline-prose/review-3.md
next: 2026-10-02-inline-prose/review-5.md
---

# Inline Prose — Review 4

The feature is **not production ready**. The implementation addresses all six findings from review 3 at their previously reported reproduction sites. Further sweeps found an uncorrected wrapper scanner, literal Markdown output that changes meaning, and terminal tests that fail when the shell shortens its command echo. The serialized paragraph-tag field also needs the complete regression matrix retained in the repository.

This review examines the current working tree, including the author's uncommitted changes. Only this review and the requested review/spec metadata are changed permanently. Cross-OS evidence and human sign-off are excluded from the readiness decision.

## Previous review follow-through

Review 3 had six unblocked findings and no blocked findings. Its human-review item concerned incomplete implementation of earlier reviews; it did not block a code repair. There were no blocked findings to become unblocked, and that process concern does not require another human decision now that all six repairs were attempted.

| Review 3 finding | Result |
|---|---|
| Opaque code bodies and quoted attributes are altered by preprocessing | The explicit/fenced body matrix, placeholder controls, multiline attributes, and attributes containing Markdown now pass. However, a later wrapper scanner still reads tag-shaped text inside attributes; finding below. |
| Explicit code blocks inside links invalidate the entire block output | Addressed for the previous cases. Links, styles, transparent wrappers, nested wrappers, and paragraphs share block splitting. The public standalone/container tests and the shipped CLI regression pass. |
| Relative link destinations do not meet the specified portable output | Addressed. Both components and link syntaxes preserve authored HTML/Markdown/tree destinations; terminal resolution uses the file-reference API. The destination tests pass. |
| Soft-break whitespace trimming stops at inline wrapper boundaries | Addressed for the full previous wrapper table, both directions and both components. Shared Markdown delimiter fixes also pass their independent-reader tests. Literal punctuation remains unsafe in the same writer; finding below. |
| New terminal rendering behavior lacks Level 2 verification | Appropriate scenarios now exist for standalone code, enclosing color, hyperlink extent, unstyled fallback, tables, containers, and break geometry. WezTerm prose/container checks, tmux table checks, and both backends' status checks pass. Two tmux command-driven scenarios fail in the capture helper; finding below. |
| Single-newline callers still lose their line structure | Addressed at the enumerated lifecycle, provider, schema, sequence, help, detail-report, hook-cell, and error-report sites. The migrated builders choose hard breaks or explicit composition spacing. The focused claudine run includes passing public diagnostic row-boundary tests. |

## Recurrence

All three earlier reviews were compared. Recurrence records an incomplete class sweep; it does not make any repair dependent on a human decision.

| Current class | Earlier finding | Siblings the earlier repair should have swept |
|---|---|---|
| Wrapper scanning reads opaque attribute/escaped text as structure | Reviews 1, 2, and 3: “Opaque code bodies and quoted attributes are altered by preprocessing” | In addition to lifting, Markdown conversion, and paragraph splitting, the recursive wrapper-body scanner in `tokens.rs`. It is another reader of the same opaque input. The complete affected wrapper categories and clean scanners are listed below. |
| Markdown writing changes literal values by exposing syntax | Reviews 1 and 2: “Markdown serialization changes literal backslashes and break meaning”; review 3 verified that repair | The same writer's ordinary punctuation, entity text, image alternatives, captions, titles, and destinations, alongside the backslash and delimiter-edge cases. The repaired backslash cases remain clean; the broader literal-preservation class does not. |

The terminal echo dependency is a new test defect in the added coverage. The incomplete JSON regression table was described in earlier reviews but was not an earlier titled finding.

## Unblocked Findings

### High: Wrapper scanning still interprets quoted and escaped tag text

**Defect class:** a recursive scope scanner treats tag-shaped characters inside quoted attributes or escaped literal text as actual opening/closing tags, corrupting content and wrapper extent.

In **biscuit-terminal**, [the wrapper-body scanner `scan_inner`](../../lib/src/components/prose/tokens.rs#L222) finds suffixes resembling tags without tracking attribute quotes or escapes. The new quote-aware lifting and declaration scanners do not protect this later scan. This violates the shared grammar's attribute-opacity and escape contracts.

The probe copied the anchor shape from [the opacity fixture](../../lib/tests/l1/prose_opacity.rs), inserted a tag spelling into its destination, and called both public components. For example:

```rust
Prose::new(r#"<b>before <a href="https://e.io/</b>">label</a> after</b> tail"#)
    .render_html_fragment().render()
```

Observed:

```html
<p><strong>before &lt;a href="https://e.io/</strong>"&gt;label&lt;/a&gt; after&lt;/b&gt; tail</p>
```

Expected: an intact link whose destination is `https://e.io/</b>`, with `before`, `label`, and `after` bold and `tail` outside bold. Substituting `<b>` in the attribute instead consumes the real closing tag as an extra nesting level and styles `tail`. Escaped `\</b>` in ordinary content prematurely closes the wrapper too.

Every row below was exercised through **both `Prose` and `InlineProse`**. Quoted attributes were tested on nested anchors (`href`), code-block declarations (`lang`), and styled declarations; each used closing-tag text, opening-tag text, and a backslash before closing-tag text. Ordinary escaped opening/closing text was also tested.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Semantic bold `<b>` | The attribute and escaped-text matrix above | Closers truncate the wrapper; openers swallow its real closer and include `tail` | Attribute characters remain literal; escapes do not change scope. |
| Semantic italic `<i>` | Same substitutions using `i` | Same corruption | Same contract. |
| Semantic strike `<~>` | Same substitutions using `~` | Same corruption | Same contract. |
| Foreground `<red>` | Same substitutions using `red` | Same corruption | Same contract. |
| Background `<bg-navy>` | Same substitutions using `bg-navy` | Same corruption | Same contract. |
| Dim `<dim>` | Same substitutions using `dim` | Same corruption | Same contract. |
| Underline `<u>` | Same substitutions using `u` | Same corruption | Same contract. |
| Transparent `<clipboard>` | Same substitutions using `clipboard` | Attribute breaks or real closing tag becomes visible text | Keep content and remove only the actual transparent wrapper. |
| Link `<a href="outer">` | Code-block/styled declaration attributes and escaped `a` tags | Link ends early or extends through `tail` | Preserve the authored link extent. |
| All nine wrapper categories | Matching tag spellings inside an inline code span or explicit code body | Literal code remains intact; wrapper resumes correctly | Clean opaque-code controls. |
| Standalone quoted declaration, without enclosing wrapper | Previously reported fence-looking, multiline, and Markdown attribute fixtures | Value retained; opacity tests pass | Clean declaration/lifting controls. |
| Stage-zero lifting, paragraph splitter, link/bold/italic preprocessing, token declaration scanner | Same attribute grammar, plus the checked-in opacity matrix | These paths now protect the declaration; the failure occurs when `scan_inner` scans its enclosing body | Keep these paths clean and bring recursive scope scanning under the same rule. |

All styled aliases and named colors dispatch to the same styled branch and scanner; links and transparent wrappers also call it. Explicit code bodies use lifted content and were clean controls. Fix the shared scanner rather than individual tag names. Retain the full wrapper/attribute/escaped-text matrix as public HTML/tree assertions that check the exact destination, content, and wrapper extent.

### High: Markdown serialization reinterprets literal text and entity spellings

**Defect class:** the Markdown writer emits literal data in syntax-bearing positions without sufficient escaping, so a reader turns text into formatting, links, HTML, or decoded entities.

In **renderable**, [the shared text writer](../../../renderable/src/tree/render/markdown.rs#L985) escapes literal backslashes but deliberately leaves other Markdown punctuation untouched. **Biscuit-terminal** removes authored prose escapes before producing `Text`, so the writer must encode the resulting literal value. Both public components produce HTML containing literal `**literal**` for source `\**literal\**`, but their Markdown is `**literal**`: an independent CommonMark reader reports bold text and loses the literal asterisks. The same error occurs when using `Prose::escape_text` to insert a literal link example.

The sweep reused the prose escaping fixture shape, changing only its literal value. Each value was rendered through **both components, both Markdown dialects**, and read back with `pulldown-cmark`. The values were escaped bold, escaped underscore emphasis, escaped `[literal](https://x.io)`, escaped `<em>literal</em>`, and literal `&copy;`; backslashes and inline/block code were controls. Public tree builders exercised the additional contexts/fields below.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Ordinary text, paragraphs/root, heading, quote, list body | All five literal values | Formatting/link/tag syntax becomes active; `&copy;` becomes `©` | Retain the literal visible value and only the tree's intended structure. |
| Bold, italic, strike wrappers | Same values inside the wrapper | Edge `_` is protected, but `**literal**` gains inner emphasis; link/tag/entity values still change meaning | Preserve literal punctuation and the original outer formatting only. |
| Color/underline spans, transparent wrapper, unknown extended wrapper | Same values | Plain Markdown changes all five; MarkdownPlus HTML spans protect tag/entity text but still activate Markdown emphasis/links | Preserve every literal value in both dialects. |
| Link label | Same values inside a public link | Literal link example creates nested link syntax; punctuation/entities change meaning | Preserve the original link and its literal label. |
| Table cell | Same five values, both dialects, independent GFM read-back | Cell structure survives, but its text acquires formatting/links/HTML or decodes to `©` | Preserve literal cell content as well as pipe safety. |
| Disclosure body and plain-Markdown summary | Same values | Syntax remains active; body values change meaning | Preserve literal text. |
| MarkdownPlus disclosure summary | Same values | HTML/entity values are escaped; summary is emitted in a raw HTML block, unlike its Markdown-parsed body | Clean HTML/entity encoding control; do not apply body escaping blindly to raw HTML. |
| [Table caption](../../../renderable/src/tree/render/markdown.rs#L270) | Five raw values via `set_table_title` | Bold/italic/link examples become markup; angle brackets and `&copy;` are correctly escaped | Preserve all literal caption values. |
| [Image alternative text](../../../renderable/src/tree/render/markdown.rs#L318) | Same raw values | Reader treats formatting/tags/link-looking content as syntax; literal delimiters are lost and entities decode | Preserve the alternative text literally. |
| [Link/image title](../../../renderable/src/tree/render/markdown.rs#L1053) | Same raw values | Markdown punctuation remains literal in the title, but `&copy;` becomes `©` | Preserve literal title, including entity spellings. |
| [Link/image destination](../../../renderable/src/tree/render/markdown.rs#L1580) | URL suffix containing each raw value | Punctuation controls preserve destination, but `https://x.io/&copy;` becomes `https://x.io/©` | Keep the exact authored destination. |
| Inline code and fenced block code | Same five raw values | Literal contents preserved | Clean controls; never apply prose-text escapes inside code. |
| Literal backslash before breaks/wrapper edges | Earlier regression matrix | Existing tests pass | Clean repaired siblings. |

Footnote bodies use the same block/text writer; sequence containers use the same inline join. Image titles/destinations share `link_target` with links. These shared routes need the same correction rather than consumer-specific escaping. Raw `Html` nodes are intentionally verbatim and are outside the literal-`Text` contract.

Repair the shared writer's context-appropriate encoding and test public read-back results across this complete matrix. The comment that escaping punctuation would change many documents is not a correctness exception: formatting is represented by structural nodes, and literal values must remain literal. Review changed snapshots for preserved visible content, not merely byte differences. This finding extends the earlier serializer repair; it does not invalidate its passing backslash controls.

### High: Terminal capture tests depend on the shell displaying the full command

**Defect class:** a test's output-region detector depends on a full command echo that shell line editing may shorten, causing false failures even when the real terminal displays correct output.

In **biscuit-terminal-cli**, [the new `rows_after_echo` helper](../../cli/tests/common/mod.rs#L133), used to identify the rows produced by a command, requires the entire sent command to appear in the captured pane. It handles wrapping, but a long command in the actual tmux shell was horizontally shortened with a leading `<`. The missing prefix cannot be reconstructed by joining rows. [The polling caller](../../cli/tests/common/mod.rs#L165) then waits 20 seconds and returns an empty row list.

The real failure frame contained:

```text
<g/wt/rusty-biscuit/fix-path-spelling/target/debug/bt' prose --force-color 'See `md hash` here'
See md hash here
bash-3.2$
```

The test reported zero output rows. The container test likewise captured `│ see md hash here` and then reported zero rows. These failures precede the style assertions and are unrelated to color or dim support.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Prose styling/geometry scenario, tmux | First inline-code command through the shipped binary's absolute path | Fails after 20 seconds; output is visibly present | Identify the one output row independently of shell echo presentation. |
| Container inline-code/break scenario, tmux | First quote command with the same path | Fails after 20 seconds; quote output is visibly present | Identify the one quoted row. |
| Same prose/container scenarios, WezTerm | Same scenario functions and fixtures | Both pass | Clean backend controls; passing one shell does not make the detector portable. |
| `display_bytes_rows`, status scenario | Short `cat` command for hard and soft output | Passes in tmux and WezTerm | Clean shorter-command control; it still shares the same fragile detector. |
| `display_bytes_rows`, prose/container hard-mode cases | The same short-command route | WezTerm scenarios pass; tmux scenarios stop before reaching these cases | Preserve this route when replacing the common detector. |
| Table inline-code/fence/wrap scenario, tmux | Existing border-based cell detector | Passes, including exact dim runs and code wrapping | Clean independent detector. |
| Kitty prose/container scenarios | Backend availability gate | Initial run reports pass, but Kitty is unavailable and the gate returns early | No execution evidence claimed for Kitty. |

These are every caller family of the new helpers: prose styling/break geometry, container geometry, and status byte display. The table suite uses its existing capture/cell helpers instead. Replace echo matching with a stable output boundary, such as explicit unique begin/end markers emitted by the executed shell command, and verify the detector against shortened/wrapped echoes and output resembling a prompt. Do not fix this by increasing the pane width, shortening this checkout's path, or adding retries: those hide the same dependency in archived or differently located runs. Add a fast regression for the detector as well as rerunning the real-terminal scenarios.

This is an unreliable test in the newly added feature coverage, not a request for additional cross-OS receipts. It is high severity under the review's terminal-verification requirements.

### Medium: The serialized paragraph tag lacks a retained robustness matrix

**Defect class:** regression coverage for a load-bearing serialized field retains only selected invalid shapes, leaving the complete public-input contract dependent on temporary review probes.

In **renderable**, [the compatibility test for `browser.block_element`](../../../renderable/src/tree/render/browser.rs#L4885), which selects the paragraph's HTML element, retains absent, explicit valid, unknown enum, and numeric cases. It omits explicit null, array element shapes, empty shapes, duplicate keys, and trailing content. The complete public probe below is clean, but the review instructions require one checked-in test walking the whole format matrix, including a positive control and rendered public results.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Legacy paragraph JSON | Field absent | Renders `<p data-role="note">x</p>`; retained test | Defined legacy default, distinct from invalid values. |
| Explicit paragraph JSON | `"div"` | Renders `<div data-role="note">x</div>`; retained test | Positive load-bearing control. |
| Same field | Unknown enum and number | Rejects; retained test | Reject. |
| Same field | Null, mixed/all-invalid arrays, empty array/object/string, duplicate key, trailing content | Each rejects in temporary public probe; no retained full matrix | Retain every rejection in one public-result regression. |
| Browser fragment/streaming paths and other node placements | Every valid paragraph tag, invalid attribute placements | Existing dedicated tests present | Clean projection/validation siblings. |

Only JSON serialization is introduced for this field; there is no new manifest/configuration reader. `href` and `lang` above are markup-string grammar, rather than a new file format. Add the matrix to the existing compatibility test using one serialized fixture and one field edit per row.

## Blocked Findings

None. All findings can be repaired under the existing contract; no design choice or human-only verification blocks them.

## Input Robustness Matrix

The public probe copied the production-serialized paragraph from the compatibility test, changed only `browser.block_element`, deserialized public `RenderNode`, and rendered accepted values with the browser renderer.

| Shape | JSON `browser.block_element` outcome | Checked-in matrix coverage |
|---|---|---|
| Positive control: `"div"` | `<div data-role="note">x</div>` | Yes. |
| Absent | `<p data-role="note">x</p>` | Yes; defined default. |
| Explicit null | Rejected | Missing. |
| Wrong whole-field type: `123` | Rejected | Yes. |
| Wrong one element: `["div",123]` | Rejected; scalar enum field | Missing. |
| Wrong every element: `[123]` | Rejected | Missing. |
| Empty: `[]`, `{}`, `""` | Each rejected | Missing. |
| Duplicate key | Rejected as duplicate `block_element` | Missing. |
| Valid JSON plus garbage | Rejected as trailing content | Missing. |

The loader does not conflate invalid values with absence. The finding concerns retained verification, not a demonstrated permissive deserializer.

## Verification

| Command/check | Result |
|---|---|
| Biscuit-terminal: `just test prose_` | 215 passed, including opacity, destinations, grammar, container, serializer, and CLI regressions. |
| Renderable: `just test delimiter_edge_tests::` | 13 passed. |
| Renderable: `just test paragraph_without_block_element_field_keeps_p` | One passed. |
| Claudine: `just test each_` | 89 passed, including the affected diagnostic row tests and search/candidate output tests. |
| Biscuit-terminal: `just test-l2 inline_code` | Container tmux test fails in echo detection; fail-fast leaves four scenarios unrun. The preceding Kitty case exits through its unavailable-backend gate. |
| Biscuit-terminal: `just test-l2 level2_prose_inline_code_and_breaks_in_tmux` | Fails independently in the same echo detector. |
| Biscuit-terminal: `just test-l2 level2_prose_inline_code_and_breaks_in_wezterm` | One passed with real cell/style/link and geometry capture. |
| Biscuit-terminal: `just test-l2 level2_container_inline_code_and_breaks_in_wezterm` | One passed. |
| Biscuit-terminal: `just test-l2 level2_prose_cells_in_tmux` | One passed, including the new inline-code/fence/wrap matrix. |
| Biscuit-terminal: `just test-l2 level2_status_hard_line_breaks` | Two passed, tmux and WezTerm. |
| Biscuit-terminal: `just test-l2 level2_container_inline_code_and_breaks_in_kitty --success-output immediate` | Confirmed `skipping: requires kitty`; no real Kitty evidence credited. |
| Root: `just check-tier-coverage biscuit-terminal` | Passed; zero stranded tests. |
| Temporary public-API example | Reproduced the wrapper, literal serialization, and JSON matrix results above; removed after use. |

The new L1 files are declared in the library's consolidated `l1` target. All four Level 2 modules are declared in the CLI's `level2` target, whose `terminal-tests` feature has a live recipe and is enabled by CI metadata. The new tests are selected by the proper tier. No permanent tests were added or renamed by this review. Level 2 execution used the repository recipe's background/detached harness paths without requesting focus.

Full affected-area tests, lint, examples/benchmarks, and cross-OS runs were not repeated. The implementation log records baseline failures outside these focused checks; this review does not count an unverified baseline claim as a new feature defect. The claudine build emitted a large unwind-section linker warning but completed. Narrow passing tests do not override the reproduced defects.

## Requirement-to-verification mapping

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Portable authored relative links and the acceptance example | Level 1 destination matrix; Level 2 terminal destination/extent capture | Passing controls. |
| Paragraph splitting, soft/hard breaks, normalized line endings, wrapper scope | Level 1 grammar/container matrices; Level 2 prose/container/status geometry | Ordinary grammar passes; quoted/escaped wrapper scope fails at Level 1. Terminal helper is unreliable on tmux. |
| Block tags, valid trees, layout, wrapper-free inline fragments, streaming parity | Level 1 public tree/HTML and serializer tests | Existing checks pass; complete malformed JSON matrix needs retention. Serialized-output contracts are appropriately tested in process. |
| Literal code, safe fences, empty/space values, Markdown break spellings | Level 1 public output and independent-reader tests | Existing code/backslash controls pass. Literal text serialization still changes meaning. |
| Dim inline code, restored enclosing color, hyperlink extent, unstyled fallback | Level 2 WezTerm cell/style/link capture | Appropriate level exists and passes; tmux counterpart has the capture defect. |
| Table inline code, fence normalization, wrapping and style containment | Level 2 tmux cell capture | Passes at the appropriate level. |
| Existing callers retain diagnostic/display rows | Level 1 actual diagnostic builder output assertions; Level 2 representative status/container rows | Previous collapse cases are repaired; selected row tests pass. |
| `code_link`, expression catalog/completion, migrated templates | Existing Level 1 public expression/template/catalog tests inspected | Appropriate level; these tests were not rerun here. |
| Keyboard input | No added keyboard behavior | No Level 3 requirement. |

No additional optimization or ergonomic change is required for readiness beyond the shared fixes above.
