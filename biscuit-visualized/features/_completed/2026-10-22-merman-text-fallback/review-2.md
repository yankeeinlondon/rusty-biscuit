---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (high): R1-F1 sibling — source-only CLI routes still hide the disabled math capability"
    - "R2-F2 (high): page geometry overrides disabled graphics and emits Kitty images"
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-24T17:07:56-07:00"
spec: 2026-10-22-merman-text-fallback/spec.md
implemented: true
description: "A **feature** review of `2026-10-22-merman-text-fallback/spec.md`"
feature: 2026-10-22-merman-text-fallback/review-2.md
previous: 2026-10-22-merman-text-fallback/review-1.md
next: 2026-10-22-merman-text-fallback/review-3.md
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-22-merman-text-fallback/log.md
implemented_by: claude/opus
human_review_items:
    - "Implementation 2 (R2-F1): source-only fallible selection asks Merman's render capability plan (svg_plan missing_capabilities) instead of a $$ scan or image render; it also reports other non-built-in capabilities and costs one extra parse on the Refused branch"
    - "Implementation 2 (R2-F2): piped/redirected page output with geometry (line numbers, width, margin, padding, background) no longer emits images at all, including under TerminalImageMode::Force, because the detected nonterminal route is now kept"
---

# Review 2: Merman text fallback

The feature is **not production ready**. The five previous findings' enumerated reproductions are repaired, but a newly checked sibling of the CLI failure issue and a separate page capability defect remain. Both have repairs within the spec; neither needs an author decision.

Reviewed the specification, previous review, repair `Finding Ledger` in `log.md`, implementation comments, text backend, component selection, generic hook, Darkmatter page integration, CLI adapters, test declarations, and affected documentation. The previous review contains no `Human rulings` section. Its accepted decisions remain accepted; informational items added by the implementer are not author rulings.

## Previous Finding Verification

The instance lists include the repair ledger's added children. Verification closes those enumerated cases; a newly checked sibling is identified separately rather than described as a regression of a passing repair.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: bt turns Mermaid rendering failures into successful source output | R1-F1.1–R1-F1.7: invalid flowchart with default/forbidden text, valid text, image-export failure, fallible backend, infallible component, common handler for nine commands | **verified fixed** | bt now uses `try_present`. Missing closing brackets exit 1 under default, never-text, and no-partial policies; valid flowchart/pie controls succeed. Backend request-count and injected image-export failure tests pass. Inspected all nine command call sites, including the common XY handler. | **R2-F1** is a newly checked capability-failure sibling, not a failure of the original broken-flowchart reproductions. The ledger acknowledges the math residual under R1-F1.2. |
| R1-F2: failed preprocessing is mistaken for an ineligible diagram | R1-F2.1–R1-F2.13: both eligibility readers, hook and strictness consumers, positive/unsupported/parse controls, infallible component, unknown type, title escaping, UTF-8 separator handling, image preparation and readers | **verified fixed** | `source_theme` returns `Result`; fallible eligibility, text rendering, and component selection propagate errors. Public hook tests verify malformed frontmatter and unknown headers under every strictness; valid frontmatter, pie, and ZenUML remain quiet. Full visualized tests include quoted-title round trips, the multibyte separator case, image goldens, and gitGraph preparation. bt scalar/title tests pass. Darkmatter installs this same hook. | None. The infallible convenience lookup still returns `None` on error, but fallible selection no longer consumes it. |
| R1-F3: text fallback can discard an explicitly supplied title | R1-F3.1–R1-F3.6: matching frontmatter title, untitled text, source routes, image-error source, bt title propagation, drawn titles and same-named labels | **verified fixed** | Deduplication follows the emitted presentation. Passing visible-occurrence assertions cover matching/absent frontmatter, sequence/XY titles, same-named labels, policy/width/ineligible source, and image failure. The bt title test covers all nine commands and restrictive source policies for its text-drawn families. | None. Title detection's family cases do not participate in eligibility. |
| R1-F4: line-number gutters make accepted text drawings overflow | R1-F4.1–R1-F4.9: backend fit, numbered/unnumbered top-level and nested output, sufficient-width controls, two-digit state gutter, standalone/bt adapters, generic boundary | **verified fixed** | `render_diagram_fragment` measures finished display-cell widths after code decoration. Passing Darkmatter tests cover the 58-cell chain at 60/62/64 columns, nested lists, and the two-digit state gutter. Generic tests cover silent decline under every strictness, exact fit, nesting, and bypass for already-rendered image output. Standalone uses plain code; bt has no numbered-code option. | None in finished-width checking. **R2-F2** is a separate capability defect before the public page reaches that check. |
| R1-F5: required styling tests depend on ambient terminal capabilities | R1-F5.1–R1-F5.9: note and md page fixtures, bt color/no-color fixtures, syntax fixture, plain assertions, styled control, hook Unicode, force-color leakage in bt tests | **verified fixed** | Component fixtures pin images, color depth, Unicode, and italics; md's fixture sets page color depth. ANSI assertions remain intact. Inspected bt fixtures clearing both force-color variables. Full visualized/terminal suites pass; component and md feature tests also pass under `NO_COLOR=1 TERM=dumb LC_ALL=C`. Hook Unicode no longer depends on locale. | None. The production adapter defect below is independent of the repaired test fixtures. |

## Unblocked Findings

### R2-F1 (high): R1-F1 sibling — source-only CLI routes still hide the disabled math capability

**Authority — spec, Dependency and compatibility boundaries:** “The `bt` CLI currently reports rendering failures and exits unsuccessfully. Preserve that behavior and its JSON/image metadata contract.” This finding concerns source-only outcomes where no successful text drawing replaces the image; it does not require rejecting a successful partial text drawing.

**Defect class:** parse success is used as proof that fallible source selection preserves rendering failures, although a known required rendering capability is disabled.

In biscuit-visualized, [`validate`](../../src/src/mermaid/render.rs:965) checks parsing only. [`try_present`](../../src/src/components/mermaid.rs:886), used by bt, considers that sufficient for `Unrequested` source output and performs no compatibility check for `Refused` source output. Both branches lose the disabled math capability. The repair log explicitly records this residual under R1-F1.2.

The existing image regression fixture is a realistic formula label:

```text
flowchart LR
    A["$$x^2$$"] --> B
```

The public artifact image API returns a `RenderFailed` error saying the render session lacks the `math` capability required by `flowchart-v2`. Public `validate()` returns `Ok(())`. With `BISCUIT_GRAPHICS=none`, these shipped CLI commands all exit **0**, print source, and emit no error:

```sh
bt --plain flowchart --feature no-mermaid-fallbacks 'A["$$x^2$$"] --> B'
bt --plain flowchart --feature no-mermaid-partial-fallbacks 'A["$$x^2$$"] --> B'
bt --plain flowchart --width 8 'A["$$x^2$$"] --> B'
MERMAID_FALLBACK=code-block bt --plain flowchart 'A["$$x^2$$"] --> B'
```

Controls removed `MERMAID_FALLBACK` unless testing the variable. Replacing the formula input with a missing closing bracket now correctly exits 1 under each policy, confirming the original repair.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Public artifact image render | Existing formula-label fixture | Missing math capability error | Same; clean reference for the preserved failure |
| R2-F1.2 | Public parse validator | Same fixture | `Ok(())` | Clean under its parse-only contract; insufficient as a complete compatibility check |
| R2-F1.3 | Fallible component's unrequested source branch | Formula with `Never` and `NoPartial` | `Ok(Source)` | Preserve the known disabled-capability failure on source-only CLI selection |
| R2-F1.4 | CLI policy consumers | Either restrictive feature, or environment setting | Exit 0, source, no error | Nonzero exit with capability error |
| R2-F1.5 | Fallible refusal branch and CLI width consumer | Same fixture, width 8 | Exit 0, source, no error | Same capability error on this source-only branch |
| R2-F1.6 | Successful text route | Formula with text allowed, width 80 | `Ok(Text)`, literal formula and partial note | Clean under metadata-driven partial-text selection; no additional rejection requested |
| R2-F1.7 | Original parse-error controls | Broken node label under default/never/no-partial policies | Exit 1 with parse error | Same; clean |
| R2-F1.8 | Valid controls and shared command consumers | Ordinary flowchart/pie; all nine command call sites inspected | Valid text/source succeeds; commands share the handler | Same; clean; repair shared selection rather than nine builders |
| R2-F1.9 | Infallible component and Markdown hook | Source/decline decisions inspected | Infallible failures become source; policy and width refusal decline | Clean under their separate contracts; preserve those behaviors |

**Recommended repair:** keep the public validator's parse-only meaning, and preserve the known disabled math capability on fallible source-only selection, including both unrequested and refused outcomes. Prefer the parsed representation or an existing capability check. Do not recreate every possible SVG failure, render an image for validation, load fonts, add a second text request, or introduce a scanner over generated drawings. One existing formula fixture, both source branches, and an ordinary valid-source control are sufficient. Qualify touched documentation's general math/source statements for the successful text route rather than making those statements impose another eligibility restriction.

### R2-F2 (high): page geometry overrides disabled graphics and emits Kitty images

**Authority — defect:** Darkmatter's capability adapter replaces an explicitly disabled graphics route with an optimistic Kitty route when a non-TTY render has a width. Rendering a Mermaid page with line numbers or a width limit while `BISCUIT_GRAPHICS=none` therefore emits image protocol bytes instead of text. The spec also requires using “existing effective capabilities and route policy, including caller overrides” before image work.

**Defect class:** layout configuration replaces the capability snapshot, granting image support that the effective route denied.

In Darkmatter, [`terminal_options_from_terminal_options`](../../../darkmatter/lib/src/markdown/render_tree/entrypoints.rs:701) detects a terminal, then substitutes `Terminal::new_optimistic` when `max_width` is set and stdout is not a TTY. That constructor hard-codes Kitty support. The page's [`has_frame_geometry`](../../../darkmatter/lib/src/layout/page.rs:1367) selects this path for margins, padding, background, maximum width, and line numbering. Direct Markdown terminal rendering and page rendering share the adapter. The repair log noticed the line-number case during R1-F4 and deferred it as needing a choice; preserving the effective graphics route is an ordinary repair within the spec.

A temporary public-API example ran with `BISCUIT_GRAPHICS=none CFProcessPath=/usr/bin/true` and non-TTY stdout. It rendered the same two-node flowchart through `DarkmatterPage::render`, changing one setting per case. The default page produced text without Kitty output. Each geometry condition instead produced Kitty image sequences. Direct `Markdown::as_terminal` repeated the defect with `TerminalOptions.max_width = Some(60)`. No terminal window opened.

The essential reproduction is:

```rust
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::{Markdown, output::MermaidMode};

let md: Markdown = "```mermaid\nflowchart LR\n Alpha --> Beta\n```\n".into();
let page = DarkmatterPage::new(&Terminal::new_optimistic(80))
    .with_mermaid_mode(MermaidMode::Automatic)
    .with_line_numbers(true);
let output = page.render(&md).unwrap();
// With BISCUIT_GRAPHICS=none and non-TTY stdout:
assert!(!output.contains("\x1b_G")); // Fails.
```

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Default page route | Automatic, no geometry | Text; no Kitty | Same; clean |
| R2-F2.2 | Page line-number geometry | `with_line_numbers(true)` | Kitty image; no drawing | Text if it fits after numbering, otherwise source; no image |
| R2-F2.3 | Page width geometry | `with_max_width(60)` | Kitty image | Text within 60 columns, otherwise source |
| R2-F2.4 | Page margin geometry | `with_margin(1)` | Kitty image | Text/source within content box |
| R2-F2.5 | Page padding geometry | `with_padding(1)` | Kitty image | Text/source within content box |
| R2-F2.6 | Page background geometry | `PageBackground::Subtle` | Kitty image | Background decoration with text/source; no image |
| R2-F2.7 | Direct terminal adapter | Automatic, width absent versus `Some(60)` | Absent: text; present: Kitty | Both honor the no-pixels route |
| R2-F2.8 | Source-only mode | Numbered page with `MermaidMode::Text` | No Kitty | Same; clean: code mode bypasses promotion |
| R2-F2.9 | Generic hook and numbered renderer | Explicit no-image capability, automatic Rich/Vector; real Darkmatter code renderer with numbering | Real-hook and numbered-width tests pass | Same; clean when given the correct capability snapshot |
| R2-F2.10 | Explicit image prohibition | Adapter's `TerminalImageMode::Never` mapping | Maps to `GraphicsMode::Off`, suppressing promotion | Clean by dispatch invariant; preserve this independent prohibition |

**Recommended repair:** when selecting optimistic color/hyperlink capabilities for deliberate page geometry, preserve the detected effective graphics route and transport restrictions. Width, numbering, and background must not grant image support. Keep the existing deliberate color/hyperlink behavior; this does not require reopening that design choice. Extend the public page test with a numbered or width-constrained no-graphics case, and cover the direct adapter with one width control. The five geometry conditions share one adapter, so a new large matrix or real-terminal suite is unnecessary.

## Blocked Findings

None. Both repairs have a recommendation within the existing decisions. No public format change, new dependency, scope transfer, or human-only evidence is needed.

## Recurrence

R2-F1 repeats the failure-preservation class of **review-1.md, R1-F1**, as a sibling the earlier sweep missed. The original broken-flowchart cases remain fixed. That sweep should also have exercised the existing math-label failure fixture through source-only CLI selection. The new parse-only validator and separate width-refusal branch both need the bounded check listed here. This does not authorize an unbounded promise to predict every image-stage failure without rendering.

R2-F2 is a separate production capability-selection defect, noted during repair but not addressed. It is not a recurrence of R1-F4's downstream gutter arithmetic or R1-F5's ambient test fixtures.

## Verification and acceptance coverage

The spec explicitly assigns rendering, styling, and integration checks to **Level 1 with explicit capabilities**, adds no keyboard interaction, and says real windows and focus changes are unnecessary. Level 2 and Level 3 are not missing requirements here.

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Diagrammatic family/result eligibility, unsupported source, metadata snapshot | L1 metadata snapshot, real structured-result guard, component/hook tests | Covered; previous overflow-policy decision retained |
| Code-node languages, exact note, emphasis and suppression | L1 node assertions, plain snapshot, explicit styled terminal | Covered; styling fixture repaired |
| Policy, environment and flag precedence | L1 injected environment, CLI parser/process and public page tests | Covered on tested routes; geometry bypass under R2-F2 |
| Feature repetition, commas, casing, errors, completions | L1 parser and CLI tests for bt/md | Covered |
| Automatic Rich/Vector/Off, decline, strictness, unchanged Code/Image | L1 generic and real-hook tests, malformed-frontmatter and unknown-header controls | Hook covered; page adapter fails R2-F2 |
| md automatic selection and no-rendering across terminal/HTML/Markdown/JSON | L1 page/CLI tests including terminal highlighting | Covered at default layout; page geometry separately reported |
| Auto/compact/canonical layout, cell fit, nesting, numbered gutters | L1 backend reports, real installed-code-renderer and generic boundary tests | Covered; gutter repair verified |
| Unicode/ASCII, spaces, multiline labels, controls, failures | L1 exact snapshots, width assertions, injected limit/deadline cases | Covered |
| Explicit title once on text/source/image failure and bt commands | L1 visible-occurrence and nine-command tests | Covered |
| No image/font/artifact work on text, one request, image metadata | L1 counters, request checks, existing image regression tests | Backend/component covered; wrong page route under R2-F2 performs image work |
| CLI failure behavior | L1 invalid-source/export-error tests plus public formula and CLI probes | Original parse cases repaired; R2-F1 remains |
| Terminal/HTML syntax and dependency direction | L1 syntax and bt adapter tests; unchanged manifest direction inspected | Covered; no Darkmatter/syntect library dependency introduced |
| Mode-match audit and documentation | Earlier workspace check recorded; current mode mappings and repair docs inspected | No missing match found; qualify math explanation with R2-F1 |

Fresh local verification through repository recipes:

- biscuit-visualized `just test`: **1,147 passed, 4 skipped**.
- biscuit-terminal `just test`: **3,919 passed, 55 skipped**, including bt.
- renderable `just test`: **684 passed**.
- Darkmatter `just test mermaid`: **104 passed**, with 10,620 tests excluded by the focused filter. The full unrelated suite was not repeated.
- biscuit-terminal `just test render_tree_diagram_hook`: **20 passed**, including the finished-width repair.
- Under `NO_COLOR=1 TERM=dumb LC_ALL=C`, visualized `just test component_fallback_tests`: **31 passed**; Darkmatter `just test mermaid_features`: **9 passed**. A temporary review example initially interrupted the latter's build with an incorrect color constructor; it was corrected, removed, and the recipe rerun successfully. This was a probe error, not a product failure.
- `just check-tier-coverage` for all four areas: **zero stranded tests**. Checked consolidated module declarations and enabled terminal/image features.
- Public API and shipped CLI probes reproduced both findings. The temporary example was removed. This review changes no production code or permanent tests.

The input robustness matrix does not apply: this feature handles Mermaid text grammar and rendering, and the repair propagates errors from existing Mermaid preprocessing rather than adding a configuration/file-format loader. Cross-OS execution evidence remains CI's responsibility and does not affect readiness. No feature directory was moved, and no commit was made.
