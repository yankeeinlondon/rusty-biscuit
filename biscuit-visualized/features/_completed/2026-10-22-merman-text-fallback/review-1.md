---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): bt turns Mermaid rendering failures into successful source output"
    - "R1-F2 (high): failed preprocessing is mistaken for an ineligible diagram"
    - "R1-F3 (high): text fallback can discard an explicitly supplied title"
    - "R1-F4 (high): line-number gutters make accepted text drawings overflow"
    - "R1-F5 (high): required styling tests depend on ambient terminal capabilities"
human_review: true
human_review_items:
    - "Swimlane joins the eligible families because the pinned renderer reports it as diagrammatic; the spec amendment preserves its metadata rule."
    - "The diagram hook uses three outcomes: finished output, render-tree nodes, and declined."
    - "Darkmatter adds automatic terminal mode while preserving its existing browser image behavior."
    - "Fallback feature names and restrictive combination rules are shared through biscuit-visualized's optional clap support."
    - "Clap supplies empty-name and unknown-name errors without a custom parser."
    - "Automatic mode tries text at full graphics capability when the effective image route cannot show pixels."
    - "The existing Unicode capability selects drawing characters; no new terminal capability is introduced."
    - "Git graph commands accept fallback flags even though their renderer metadata always selects source."
    - "No new benchmark or additional performance measurement is required; deterministic work counters prove bypassed work."
    - "The temporary source-only hook below full graphics was replaced when text rendering landed; nothing remains deferred there."
    - "Mermaid promotion is disabled inside hook-returned fragments to prevent recursive source promotion."
    - "A new biscuit-terminal render-tree topic page supplies the documentation that previously had no dedicated page."
    - "Text rendering requires the terminal feature so fit uses biscuit-terminal's display-cell measurement."
    - "Text layout uses standard widths; a separate CJK width option is omitted because terminal measurement has no matching mode."
    - "Text requests reuse the diagram's existing configurable time budget."
    - "Production requests allow reported overflow and select source; a separately generated real structured report tests the projection guard."
    - "Class and ER results trust the renderer's diagrammatic classification even when their drawing includes a relationship list."
    - "Text retains the existing label-newline rewrite, including the upstream limitation that class labels can show literal br markup."
    - "The hook request carries the Mermaid mode so automatic and image-only requests remain distinguishable."
    - "Image availability uses the same effective route decision as image display, including Zellij without measured cells."
    - "The fixed two-column decoration reserve needs correction for line-number gutters under R1-F4."
    - "Source fallback uses code nodes, accepting their fence styling and trailing-newline normalization; the raw source method remains available."
    - "Explicit titles use a bold paragraph; suppression of that paragraph needs the output-aware correction in R1-F3."
    - "The diagram renderer now stores policy and is constructed with default() instead of as a unit struct."
    - "Mermaid uses a separate small syntax set to avoid measured startup cost; embedded JavaScript, HTML, and YAML get limited highlighting."
    - "The flattened syntax includes missing family headers and rewrites rules that syntect misreads; changes are recorded beside it."
    - "The no-mermaid-rendering feature reuses the existing text mode instead of adding another mode."
    - "A public terminal-options field and page builder carry fallback policy into Darkmatter."
    - "Flags target the nine existing bt commands; no generic mermaid or xy-chart command is added to match mistaken plan names."
    - "Successful bt exits on invalid Mermaid without images are rejected by R1-F1; restore the specified failure behavior."
    - "bt uses the permitted raw code-renderer adapter with a local deprecation allowance and a public component-width calculation."
    - "Previously stale JSON snapshots were re-recorded for the existing insertion-order contract after parsed-value equality checks."
    - "Bar and line charts retain source fallback when their generated source lacks a y-axis range; adding ranges would change image output."
    - "No external report was filed for the upstream XY-chart diagnostic-looking text because external actions were outside authorization."
    - "Invalid-Unicode fallback environment values behave like other unrecognized values and retain the default policy."
    - "An unreachable presentation-node default remains; the image outcome is handled before nodes are requested."
    - "Documentation, license notices, and dependency direction use review and graph inspection rather than new automated tests."
    - "Unrelated existing documentation references were left alone to keep changes surgical."
    - "Unchanged downstream tests were not repeated after documentation-only and internal-test-only changes."
    - "Additional local cross-OS runs were left to CI; cross-OS evidence does not affect this review's readiness decision."
    - "Implementation 1 (R1-F1.2): without images and with text forbidden, bt checks the source with one strict Merman parse (validate: no layout, fonts, or text request); a failure Merman reaches only while drawing SVG (math labels) is still not reported there."
    - "Implementation 1 (R1-F2.11): an undetectable diagram type is a parse failure on the text and hook routes (strictness applies); ZenUML, which Merman detects but cannot draw as text, stays a quiet decline."
    - "Implementation 1 (R1-F2.12): bt's --title is now written into frontmatter as a YAML double-quoted scalar, fixing a pre-existing break on titles such as 'Q: [draft]'; pie/quadrant/timeline/XY title lines are still unescaped."
    - "Implementation 1 (R1-F3): text title dedupe trusts only the drawn title rows Merman emits (sequence first row, XY chart title row); bt also sets the component title (image alt text only, new cache key), and timeline writes only its title line, so its JSON instructions lose the frontmatter block."
    - "Implementation 1 (R1-F4): biscuit-terminal's hook boundary declines a node fragment whose finished rows exceed the fence's available width, rendering ordinary code and dropping the fragment's diagnostics."
    - "Implementation 1 (R1-F5.9): bt's pre-existing NO_COLOR integration tests now also clear FORCE_COLOR and CLICOLOR_FORCE on the child."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-24T15:34:32-07:00"
spec: 2026-10-22-merman-text-fallback/spec.md
next: 2026-10-22-merman-text-fallback/review-2.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-22-merman-text-fallback/log.md
description: "A **feature** review of `2026-10-22-merman-text-fallback/spec.md`"
feature: 2026-10-22-merman-text-fallback/review-1.md
---

# Review 1: Merman text fallback

The feature is **not production ready**. Five findings remain. All have a repair consistent with the spec; none needs an author decision before repair. This is the first review, so there are no previous findings to carry or close.

The review covered the spec, plan, implementation log, text backend, component selection, generic diagram hook, Darkmatter integration and highlighting, CLI wiring, test declarations, and changed documentation. Two independent agents read the implementation decisions and the spec in parallel before the comparison below.

## Findings

### R1-F1 (high): bt turns Mermaid rendering failures into successful source output

**Authority — spec:** “The `bt` CLI currently reports rendering failures and exits unsuccessfully. Preserve that behavior and its JSON/image metadata contract.” The selection table explicitly applies to the standalone component's **infallible** rendering; it does not authorize changing the CLI's error contract.

**Defect class:** a fallible caller loses the reason for source fallback by consuming an infallible presentation result.

In biscuit-terminal-cli, [display_mermaid](../../../biscuit-terminal/cli/src/commands/mermaid.rs:59) skips the fallible operation when pixels are unavailable. It synthesizes `NoImageSupport`, and [handle_mermaid_error](../../../biscuit-terminal/cli/src/commands/mermaid.rs:161) prints the component's infallible presentation and returns success. The component correctly hides a text-rendering error for its own infallible contract, but the CLI consequently cannot report that error.

Reproduced with the built CLI, using the existing two-node flowchart control and one missing closing bracket:

```sh
target/debug/bt --plain flowchart 'A --> B'
target/debug/bt --plain flowchart 'A[unclosed --> B'
target/debug/bt --plain flowchart --feature no-mermaid-fallbacks 'A[unclosed --> B'
```

Both invalid commands exit **0**, print a Mermaid source block, and emit no error. The valid control exits 0 and prints a text drawing. The log explicitly records the changed exit behavior; its claim that preservation requires another text request is unnecessary if the first request's error is retained.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | CLI no-image display and fallback handler | Broken flowchart, default fallback policy | Exit 0, source, no error | Nonzero exit with rendering error |
| R1-F1.2 | CLI policy-forbidden source route | Same broken flowchart, `no-mermaid-fallbacks` | Exit 0, source, no error | Preserve CLI failure behavior |
| R1-F1.3 | CLI successful text route | Unedited `A --> B` control | Exit 0, text | Same; clean |
| R1-F1.4 | CLI image-export error handler | Existing injected `DisplayError("encoder refused")` test | Returns error | Same; clean |
| R1-F1.5 | biscuit-visualized fallible text backend | Same broken flowchart | Returns parse error after one request | Same; clean |
| R1-F1.6 | biscuit-visualized standalone infallible presentation | Same broken flowchart | Source | Same under its explicit contract; clean |

**Sibling sweep:** every Mermaid CLI route reaches this same display function: flowchart, state diagram, ERD, pie chart, quadrant, timeline, bar chart, line chart, and git graph. Bar and line charts share the XY builder. None has an independent downstream decision restoring the lost error. Their common handler is the repair site; do not add nine validation implementations. The existing image-error test verifies the handler's error arm but cannot exercise the bypass introduced before that arm.

**Recommended repair:** retain a fallible presentation path for the CLI, with ordinary ineligibility and width refusal distinguished from actual failures. The standalone component can continue converting those failures to source. Preserve the one-text-request limit and font/artifact bypass; do not retry image rendering just to discover an error. Add L1 CLI coverage for invalid input without images, including policy-forbidden text, and update the comments/docs that currently describe successful fallback for every no-image failure.

### R1-F2 (high): failed preprocessing is mistaken for an ineligible diagram

**Authority — spec, automatic-mode decision:** “A real failure (a parse error, a Merman error, a deadline or limit, or an image failure after the image route was chosen) is a hook error and follows that policy.” The generic declined outcome is reserved for normal ineligibility, insufficient width, and policy refusal.

**Defect class:** error-erasing eligibility detection turns a known preprocessing failure into a normal negative classification, bypassing downstream strictness.

In biscuit-visualized, [source_theme](../../src/src/mermaid/render.rs:1174) catches preprocessing errors and returns no family. Both [render_text_with](../../src/src/mermaid/render.rs:975) and [text_nodes](../../src/src/components/mermaid.rs:864) read that missing family as ordinary ineligibility. The latter returns before a fallible render, so [TerminalDiagramRenderer](../../src/src/components/render_hook.rs:93) returns `Declined` and the generic renderer has no error to handle. The old image path eventually called Merman and surfaced the failure; the new text path exits early.

Public-API reproduction: render a `mermaid` code node through biscuit-terminal's `render_terminal_node`, with the real biscuit-visualized hook, `Automatic`, `Vector`, and explicit image support disabled. Change the valid flowchart control by adding malformed Mermaid frontmatter:

```text
---
title: [broken
---
flowchart LR
 A --> B
```

`Strict` returns success, `Warn` returns no diagnostic, and both show source. For comparison, `flowchart LR\n A[unclosed --> B` correctly errors under `Strict` and produces one diagnostic under `Warn`.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Source preprocessing/classification | Valid flowchart plus malformed frontmatter | Failure becomes missing family | Preserve the preprocessing error for fallible selection |
| R1-F2.2 | Fallible artifact text selection | Same malformed frontmatter through public `render_text` | `Ok(Source(Ineligible))` | Rendering error |
| R1-F2.3 | Component text-node selection and hook outcome | Same malformed frontmatter | `None`, then `Declined` | Hook error |
| R1-F2.4 | Generic renderer, `Strict` | Same malformed frontmatter through the real hook | Successful source output | Rejected render |
| R1-F2.5 | Generic renderer, `Warn` | Same input | Successful source, zero diagnostics | Source with one failure diagnostic |
| R1-F2.6 | Generic renderer, permissive | Same input | Successful source, zero diagnostics | Same visible output; clean, but upstream classification still needs repair |
| R1-F2.7 | Generic renderer, all three strictness settings | Unedited flowchart control | Successful text, no diagnostics | Same; clean |
| R1-F2.8 | Generic renderer, all three strictness settings | Valid pie source | Ordinary declined source, no diagnostics | Same; clean |
| R1-F2.9 | Generic renderer, all three strictness settings | Eligible header with broken node label | Strict error; warn diagnostic; silent permissive source | Same; clean |
| R1-F2.10 | Standalone infallible presentation | Malformed source | Source by its explicit failure contract | Same; clean |

**Sibling sweep:** both eligibility readers are listed above, followed through the hook's decline and each generic strictness decision. Darkmatter installs this same hook rather than a separate eligibility implementation; its page path inherits the classification error. Normal unsupported families must remain quiet declines. The image route is not affected by this early return because it reaches the fallible image renderer.

**Recommended repair:** keep the convenience eligibility lookup if useful, but give fallible text/hook selection access to preprocessing errors rather than converting them to absence. Test the malformed-frontmatter case through the public generic rendering result under all three strictness settings. No additional text render request or general configuration-validation framework is needed.

### R1-F3 (high): text fallback can discard an explicitly supplied title

**Authority — spec, Text presentation:** “Preserve an explicitly supplied component title once, including when source is selected. Do not invent a second title when the diagram already supplies the same title.”

**Defect class:** deduplication tests the original source rather than the presentation actually emitted, suppressing the only visible copy of a value.

In biscuit-visualized, [titled](../../src/src/components/mermaid.rs:899) omits the title paragraph whenever the source contains the same title. Merman's flowchart text drawing does not contain that frontmatter title, and successful text presentation does not include the source. The resulting output contains no title.

Reproduced through the public component with an explicitly configured no-image terminal:

```rust
let source = "---\ntitle: Release\n---\nflowchart LR\n A --> B";
let diagram = biscuit_visualized::components::MermaidDiagram::new(source)
    .with_title("Release");
```

Rendering this component produces **zero** occurrences of `Release`. Removing only the frontmatter produces one. Forcing source with `TextFallback::Never` also produces one, so the defect is specific to deduplicating successful text against source that will not be displayed.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Component text presentation and shared title suppression | Explicit title equals source frontmatter title | No visible title | One visible title |
| R1-F3.2 | Component text presentation | Same explicit title, no source title | One visible title paragraph | Same; clean |
| R1-F3.3 | Component source presentation | Explicit title equals source frontmatter title | Title appears once in source | Same; clean |
| R1-F3.4 | Component source presentation | Explicit title, no source title | Title paragraph appears once | Same; clean |

**Sibling sweep:** `present_with` applies the same `titled` helper to successful text, ineligible/policy/width source, and image-error source. Source nodes retain the original title, so source deduplication is justified; text nodes do not inherit that guarantee. The Markdown hook constructs no explicit component title, so it has no second title-suppression implementation to repair. Existing title tests cover untitled text and titled source, leaving this combination untested.

**Recommended repair:** make deduplication depend on the selected output. Keep source deduplication, but preserve the explicit title when a text drawing does not itself display it. Add the matching-title text control beside the existing title tests, checking one visible occurrence rather than only node kinds. Correct the helper's comment with its behavior.

### R1-F4 (high): line-number gutters make accepted text drawings overflow

**Authority — spec, Text presentation:** “Bound output by the columns available at the diagram's actual nested position, less the `txt` code block's own padding and borders”; an unfitting drawing must show source “rather than clipping, wrapping finished diagram rows.”

**Defect class:** fit is decided before downstream decoration adds a variable-width gutter, using a fixed allowance that understates actual output width.

In biscuit-visualized, [text_nodes](../../src/src/components/mermaid.rs:878) always reserves two columns. Darkmatter's [render_terminal_code_block](../../../darkmatter/lib/src/markdown/output/code_block.rs:76) instead adds the number of line-number digits plus three columns when line numbering is enabled. The generic [TerminalRenderContext](../../../biscuit-terminal/lib/src/render_tree/options.rs:148) carries this setting, but its diagram request does not carry the resulting content budget. Darkmatter's existing page/CLI line-number option therefore reaches the generated `txt` node after fit has already been accepted.

Public-API reproduction: install Darkmatter's real code renderer and the real diagram hook on `TerminalRenderOptions`, set `Automatic`, `Vector`, explicit TrueColor/no-image capabilities, width 60, and `context.line_numbers = true`. Render the existing chain fixture:

```text
flowchart LR
 A[Alpha] --> B[Beta] --> C[Gamma] --> D[Delta] --> E[Epsilon]
```

The compact drawing is 58 cells. The numbered output row is **62** cells in a **60**-column area. Rendering it in a list item in a 62-column terminal produces a **64**-cell row. A canonical state drawing demonstrates that this affects both layout families and longer line-number gutters.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F4.1 | Backend display-cell fit | Compact chain at 58 content columns | 58-cell drawing | Same; clean before decoration |
| R1-F4.2 | Hook plus Darkmatter code renderer, no line numbers | Chain in 60 columns | 60-cell rendered row | Same; clean |
| R1-F4.3 | Hook plus Darkmatter code renderer, one-digit gutter | Same chain in 60 columns, line numbers on | 62-cell row accepted | Source, because finished drawing cannot fit |
| R1-F4.4 | Same numbered renderer with sufficient width | Same chain in 62 and 64 columns | Rows fit their respective limits | Same; clean |
| R1-F4.5 | Nested-list width decision, no line numbers | Chain in list item on 62-column terminal | 62-cell outer row | Same; clean |
| R1-F4.6 | Nested-list width decision, line numbers on | Same list fixture | 64-cell outer row | Source inside the item |
| R1-F4.7 | Canonical layout plus two-digit gutter | Existing Idle/Running/Finished state fixture in 14 columns | Text accepted; widest output row 17 cells | Source |
| R1-F4.8 | Plain standalone component and bt's current code adapter | Same chain, no numbered gutter | Existing width tests and explicit unnumbered render fit | Same; clean under these callers' current configuration |

**Sibling sweep:** the backend correctly measures drawing cells, and ordinary unnumbered component and CLI rendering remain within their tested budgets. The loss occurs where the generic hook's accepted nodes acquire Darkmatter's numbering decoration. Both top-level and nested dispatch, automatic compact layout, and canonical-only layout were exercised. The ordinary source renderer's existing overflow behavior is explicitly allowed and is not part of this finding.

**Recommended repair:** account for actual installed-renderer decoration before accepting the drawing. A small alternative is to verify the rendered drawing fragment's width at the hook boundary and decline to ordinary source if it exceeds the available columns; that does not require another Merman request. Do not clip or wrap the diagram and do not add a large family-specific sizing matrix. Add L1 tests for a numbered top-level and nested drawing, including a multi-digit gutter.

### R1-F5 (high): required styling tests depend on ambient terminal capabilities

**Authority — spec, Acceptance and verification:** “Keep these tests at L1 with explicit terminal capabilities.” The exact note styling and highlighted terminal source are required by the acceptance criteria for presentation and `no-mermaid-rendering`.

**Defect class:** tests assert forced ANSI output while leaving the effective rendering capability dependent on the host.

Two newly added tests fail in this noninteractive session. In biscuit-visualized, [no_images](../../src/src/tests/component_fallback_tests.rs:68) pins image support but leaves color and italic support detected. The note test's plain snapshot passes, then its bold assertion fails at line 229 because the output has no ANSI sequences. In darkmatter-cli, [page_for](../../../darkmatter/cli/tests/l1/mermaid_features.rs:51) constructs an optimistic terminal, but a default-layout Darkmatter page deliberately uses ambient capability selection unless color depth is explicitly set on the page. Its no-rendering test fails at line 177 because it receives a plain fenced block instead of highlighted output.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F5.1 | biscuit-visualized note style fixture | Ordinary noninteractive `just test` | Required note test fails; bold/dim/italic assertions receive plain output | Explicit style-capable fixture; passing style assertions |
| R1-F5.2 | Darkmatter CLI terminal-page fixture | Ordinary noninteractive `just test` | Required no-rendering highlighting test fails; receives plain Mermaid fence | Pin page color depth; passing highlighting assertion |
| R1-F5.3 | bt source/text highlighting fixtures | Explicit TrueColor/no-image terminal | Both tests pass | Same; clean |
| R1-F5.4 | bt no-color fixture | Explicit `ColorDepth::None` | Plain source/text fences pass | Same; clean |
| R1-F5.5 | Darkmatter Mermaid syntax terminal fixture | Explicit `TerminalOptions` TrueColor and fixed width | Highlighting tests pass | Same; clean |
| R1-F5.6 | Plain note snapshot and generic hook assertions | ANSI stripped or no ANSI assertion | Pass | Same; clean; these do not prove styling |
| R1-F5.7 | Component note with explicitly style-capable terminal | Optimistic terminal with image route replaced by no-image support | Bold, dim, and italic sequences all present | Same; clean control confirming the fixture defect |

**Sibling sweep:** checked the new note-style test, both CLI feature modules, bt's explicit color/no-color adapter tests, and Darkmatter's dedicated Mermaid syntax tests. The two failing fixtures above are the sites that rely on ambient capability while promising styled output. The failures are deterministic capability mismatches, not evidence that the drawing or note text itself is wrong.

**Recommended repair:** explicitly set the component fixture's color/italic capabilities, and explicitly set the Darkmatter page's color depth through its existing builder. Preserve separate plain-output coverage. Do not weaken the ANSI assertions or rely on CI exporting a particular terminal variable. Re-run the affected package recipes without forced host color settings.

## Verification and acceptance coverage

The spec explicitly assigns this feature's output, span styling, and integration checks to **Level 1 with explicit terminal capabilities**, and says real windows and focus changes are unnecessary. No keyboard-input behavior is added. Level 2 or Level 3 evidence is therefore not required by this spec. Temporary examples used to reproduce public-API failures were removed after the review; production code and tests were not changed.

| Acceptance requirement | Strongest relevant verification present | Review result |
| --- | --- | --- |
| Flowchart text; mindmap/pie source | L1 component selection with no-image capability | Covered |
| Structured result of a diagrammatic family selects source | L1 real Merman structured result through the fit judge, plus component overflow selection | Purpose preserved by the documented overflow-policy choice |
| Runtime metadata and seven-family eligibility snapshot | L1 pinned table and public eligibility tests | Covered; no family allowlist in the selection path |
| Code-node languages, exact note, emphasis, note suppression | L1 node/plain assertions and ANSI assertions | Style proof fails without pinned capabilities: R1-F5 |
| Per-setting environment precedence | L1 injected environment and CLI tests | Covered |
| No partial text across eligible families | L1 seven-family fixture loop and request counter | Covered |
| Feature parsing, errors, repetition, casing, completions | L1 parser and CLI tests for both programs | Covered |
| Automatic graphics levels, decline, strictness, old modes | L1 real hook through generic renderer | Ordinary paths covered; preprocessing failure gap: R1-F2 |
| md selects automatic mode | L1 terminal-page wiring test | Covered |
| No-rendering feature across terminal, HTML, Markdown, JSON | L1 page and process tests | HTML/source targets covered; terminal highlight assertion fails: R1-F5 |
| Auto compact layout and canonical-only refusal | L1 request/report assertions | Covered before final code decoration; R1-F4 |
| Mermaid grammar, token colors, flattened form, MIT notices | L1 terminal/HTML token assertions; file/docs review | Covered |
| No image work, unchanged image metadata, bounded requests | L1 artifact/font/request counters and image regression tests | Covered; CLI failure compatibility separately fails: R1-F1 |
| Charsets, Unicode labels, spaces, multiline labels, nested width, controls, failure fallback | L1 exact text and focused assertions | Covered for existing cases; numbered final width and explicit-title combination missing: R1-F4/R1-F3 |
| Workspace mode-match audit | Implementer recorded workspace all-target check; audited changed mode mappings | No missing match found; full workspace check not repeated during review |
| Documentation behavior, examples, no new spec references | Review of updated topic pages and implementation record | Broad topics present; repair behavior explanations alongside R1-F1/R1-F3/R1-F4 |
| bt highlighting and dependency direction | L1 explicit TrueColor source/text adapter tests; manifest dependency inspection | Covered |

Fresh local runs:

- `biscuit-visualized: just test --no-fail-fast`: **1,127 passed, 1 failed, 4 skipped**. The failed note-styling test is R1-F5. The ordinary first run also failed there; a focused rerun reproduced it.
- `biscuit-terminal: just test`: **3,908 passed, 55 skipped**, including its library and CLI.
- `renderable: just test`: **684 passed**.
- `darkmatter: just test`: **9,046 passed, 1 failed, 10 skipped** before fail-fast left 1,661 tests unrun. The failed terminal highlighting test is R1-F5. This review does not claim the unrun remainder passed.
- `just check-tier-coverage` for all four areas: **zero stranded tests**. biscuit-visualized was rechecked after removing temporary review examples. New tests are declared by the consolidated test roots, and the relevant features are enabled by their package recipes/CI metadata.
- Fresh `cargo tree` inspection found no Darkmatter or syntect dependency under biscuit-visualized or biscuit-terminal's library.

The input robustness matrix does not apply: the added input is Mermaid text grammar and rendering policy, not a new file-format/configuration loader. Existing Mermaid preprocessing errors were still traced through the changed selection readers, producing R1-F2. The new policy has no deserializer; the `.ok()` on `MERMAID_FALLBACK` preserves the specified default for any value other than case-insensitive `code-block`.

## Implementation decisions (informational)

These choices were collected independently from the log's `decisions_for_review` list and implementation headings and checked against the spec, including its bounded-autonomy authorization. They are for the author's later inspection and possible reversal **after** repair; no response is needed to continue the cycle. Choices with defects have a concrete recommended repair above, rather than a decision reserved for the author.

| Item | Choice or omitted work | Review judgment and reason |
| --- | --- | --- |
| 1 | Include swimlane and amend the eligibility example/snapshot | Accept. Delivered metadata supplies the extra family; bounded autonomy preserves the rule against a handwritten family list. |
| 2 | Three-way generic hook outcome | Accept. Required behaviors are represented directly; the exact enum shape was discretionary. |
| 3 | Darkmatter automatic mode, with existing browser image mapping | Accept. Preserves the terminal/image distinction and browser behavior. |
| 4 | Shared clap fallback names/fold; variants in md's existing enum | Accept. Keeps the restrictive policy consistent without a new dependency. |
| 5 | Standard clap validation | Accept. Existing parsing tests cover invalid and empty names. |
| 6 | Text at full graphics when the effective route has no pixels | Accept. Graphics level is a ceiling, not proof of image support. |
| 7 | Existing Unicode flag chooses connectors | Accept. Keeps authored Unicode and introduces no extra probe. |
| 8 | Fallback flags have no visible effect on git graph | Accept. Its structured-report classification always selects source. |
| 9 | No additional benchmark/measurement | Accept. The spec explicitly permits deterministic request and work counters instead. |
| 10 | Temporary decline below full graphics before text landed | Historical only. Final text integration replaced it; no remaining deferral. |
| 11 | Disable promotion inside hook fragments | Accept. Prevents source code nodes from recursively reentering the hook. |
| 12 | New terminal render-tree topic page | Accept. Fills the requested documentation role without expanding README reference material. |
| 13 | Gate text rendering on `terminal`; retain metadata lookup under `image` | Accept. Uses the required shared display-width measurement without broadening image-only dependencies. |
| 14 | Omit separate CJK width option | Accept. The terminal measurer has no corresponding mode, so exposing it would create inconsistent fit decisions. |
| 15 | Reuse existing time-budget builder | Accept. This is the deadline the spec asks text requests to honor. |
| 16 | Allow overflow in production; test real structured report separately | Accept. Preserves source selection and tests the projection guard without adding fallback-unavailable errors to normal overflow. |
| 17 | Trust class/ER diagrammatic classification for boxes plus relations | Accept. Overriding it by scanning output would contradict the metadata rule. Partial coverage is disclosed. |
| 18 | Keep label-newline normalization despite literal br text in class output | Accept as an upstream partial-output limitation. The spec requires shared normalization, not complete class-label fidelity. |
| 19 | Add Mermaid mode to the generic request | Accept. Lets image-only requests retain their meaning. |
| 20 | Add effective image-route query | Accept. Uses the display route's actual decision, including the Zellij cell-size case. |
| 21 | Fixed two-column text decoration reserve | Correct under R1-F4. Insufficient for the already-supported line-number option; no human choice is needed. |
| 22 | Code-node source fallback changes fence styling/newline normalization | Accept. Required by code-node ownership; original source and raw fallback method remain available. |
| 23 | Bold explicit-title paragraph and source-text deduplication | Presentation choice is reasonable; correct deduplication under R1-F3 so the title survives successful text. |
| 24 | Policy-bearing hook struct with `default()` construction | Accept. Straightforward storage for the required policy. |
| 25 | Independent small Mermaid syntax set | Accept. Avoids measured startup rebuilding; full embedded-language highlighting is not required. |
| 26 | Add other-family headers and rewrite incompatible flattened rules | Accept. Necessary for required header/init/node highlighting; provenance and edits are recorded beside the grammar. |
| 27 | Reuse text mode for no-rendering feature | Accept. Produces code in all required targets and suppresses Mermaid.js. |
| 28 | Public options field and page builder for fallback policy | Accept. Gives callers the required settings using existing options patterns. |
| 29 | Use nine actual bt commands; replace nonexistent plan commands in tests | Accept. Adding new commands merely to match mistaken plan names would expand scope. |
| 30 | bt returns success after no-image parse failure | Reject the behavior under R1-F1. The spec supplies the failure default and a bounded repair; no ruling is needed. |
| 31 | Use permitted deprecated raw renderer, expose component text width, omit unrequested language header | Accept. Existing adapter allowance is explicit; no replacement public constructor is required for this work. |
| 32 | Re-record insertion-order JSON snapshots/expected files | Accept the existing ordering contract. The log records parsed JSON equality, and these changes do not establish a new format decision. |
| 33 | Keep bar/line charts without generated y-axis ranges | Accept. Source fallback remains valid; adding a range would alter image output the spec protects. |
| 34 | Do not file an upstream XY-chart text report | Accept. External posting was not authorized. The limitation is documented; this does not defer a required local feature. |
| 35 | Invalid-Unicode environment value becomes unset/default | Accept. Every unrecognized value already resolves to that same default. |
| 36 | Keep unreachable `nodes().unwrap_or_default()` fallback | Accept. The image arm returns first, and only that outcome has no nodes. |
| 37 | Review docs/license/dependency direction rather than add tests | Accept. The acceptance criteria allow review/graph inspection; no new test framework is warranted. |
| 38 | Leave unrelated existing documentation references untouched | Accept. They do not change this feature's behavior or establish new requirements. |
| 39 | Do not rerun unchanged downstream tests after docs/internal-test changes | Accept. Prior dependency checks are recorded; those final changes do not alter shipped dependent code. |
| 40 | Leave additional cross-OS runs to CI | Informational only. Cross-OS execution evidence is explicitly outside this review's readiness gate. |

No item meets the threshold for stopping for a human: acceptable choices have a recommendation within the spec, and rejected choices have an unblocked repair within its existing decisions. No feature lifecycle directory was moved.
