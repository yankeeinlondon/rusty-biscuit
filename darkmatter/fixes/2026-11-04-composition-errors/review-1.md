---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): HTML reference discovery silently drops multiline tags and media targets"
  - "R1-F2 (high): Diagnostic projection discards known source locations"
  - "R1-F3 (high): Unlimited component output still uses a million-column sentinel"
  - "R1-F4 (high): Frontmatter flow-list references do not report their value locations"
  - "R1-F5 (high): The new styled reference report lacks real-terminal verification"
human_review: true
human_review_items:
  - "Retain fractional width values until resolution so flooring can produce a warning."
  - "Retain Darkmatter warning codes in the CLI while the shared library returns neutral warning text."
  - "Retain the explicit plain profile as the switch for plain structural rendering."
  - "Retain the old direct-terminal constructor behavior outside the new output route; repair the new unlimited bridge under R1-F3."
  - "Retain request-captured terminal environment values instead of adding ambient reads to detection."
  - "Retain prose-element browser width constraints; nested code remains subject to its containing list item."
  - "Retain Markdown wrap-point selection that avoids accidentally introducing block syntax."
  - "Retain styled code truncation without ellipses and the Mermaid drawing exception."
  - "Retain authored destinations for verbose nonclickable links and the unified image fallback."
  - "Retain forwarding verbosity to schema-validation and Sniff link rendering and updating affected pipe assertions."
  - "Retain the existing broad legacy-title detector, including its treatment of prose containing an assignment."
  - "Retain biscuit-file's existing pretty TOML layout."
  - "Retain the additive Json5::from_value constructor to avoid serialization followed by reparsing."
  - "Retain failing compose when its reference check cannot run, including previously swallowed out-of-tree and child failures."
  - "Retain passive, lenient reference preparation and its frontmatter interpolation so executable values can wait for composition."
  - "Retain applying document recoveries before passive reference checking, with remote/cache configuration kept afterward."
  - "Retain authored and post-interpolation link-title scans, including warnings inside inactive blocks."
  - "Retain link-title warnings and their denial codes in md render."
  - "Retain clean-only legacy-title migration, preserving unsupported prose and leaving expression-bearing or image-shared titles unchanged."
  - "Retain the existing nonreference warning-row test because the plan incorrectly described it as a report-view assertion."
  - "Retain the shared borderless table and detail-row handling; the early boxed-overhead defect was repaired later."
  - "Retain the report's terminal tree override for bare file URLs and terminal link parameters."
  - "Retain warnings-only report titles, summary arithmetic, stdin labels, canonical relative paths, and destination delimiter encoding as documented."
  - "Retain BOM, CRLF, literal-frontmatter, and synthesized-TOC choices; repair flow-list value positions under R1-F4."
  - "Retain existing fragment-validation behavior while reporting final numbered section slugs separately."
  - "Retain proven source-text locations, but repair discarded known diagnostic source information under R1-F2."
  - "Repair deferred multiline HTML/CSS discovery under R1-F1 and deferred condition-warning location propagation under R1-F2."
  - "Retain the explicit test color profile that removes ambient terminal dependence."
  - "Retain classifying the sandboxing corpus failure as cross-platform rather than marking Windows alone as failing."
  - "Retain the log's reconstruction of changes from shared-worktree commits; no history rewrite is requested by this review."
  - "Retain moving lifecycle stdout actions and command output to stderr for formatted composition, including prose rendering."
  - "Retain source bytes for implicit unknown-width render Markdown; explicit unlimited width still joins paragraphs."
  - "Retain the envelope document without the final newline added by command printing, preserving the previous envelope convention."
  - "Retain data-mode show printing and opening the payload, including failures, and treating opener failure as a failed run."
  - "Retain the existing opener test seam extended with MD_DRY_RUN=fail."
  - "Retain refusing interactive approval and suppressing tracing initialization for data output."
  - "Retain the single global width argument rather than duplicating it at every command leaf."
  - "Retain moving max-width onto render options so both explicit and implicit render accept it."
  - "Retain compose's existing cleanup-frontmatter behavior to honor the spec's explicit unchanged-default-bytes promise."
  - "Retain the final shared selector for schema-about pipe output; the temporary optimistic profile was removed later."
  - "Retain removing width-implies-styling behavior and honoring explicit hyperlink and italic settings."
  - "Retain routing multiple-heading notices through tracing instead of library stderr writes."
  - "Retain render style diagnostics for page/data output, with source Markdown unchanged."
  - "Retain preparing one target eagerly so rendering errors occur before output delivery."
  - "Retain removing duplicate generic lifecycle diagnostics after a reference refusal."
  - "Retain updating Claudine's stale terminal fallback assertion to match the shared rendering change."
  - "Retain silent terminal/plain removal saves and explicit Markdown/HTML removal summaries."
  - "Retain clean's existing diagnostic rows and use run_diagnostics for invocation-level records."
  - "Retain wrapping get/schema-detect warning and failure values to protect user-owned object keys."
  - "Retain simple prose/code projections for newly supported non-document formatted output."
  - "Retain classifying incompatible directory-hash options and invalid highlight arguments as usage errors."
  - "Retain removing unreachable code-block show handling; adding that feature is outside this spec."
  - "Retain schema-validation problems in per-file records without duplicating every problem into run diagnostics."
  - "Retain omitting schema-validation verbose timings from data output because the spec adds compose perf only."
  - "Retain stripping escapes at the CLI boundary for status components that still paint unconditionally."
  - "Retain Unix script-backed edit-success tests; native Windows success coverage remains an informational limitation."
  - "Retain stacking report cells when a borderless row cannot fit instead of hiding its locations."
  - "Retain decimal strings for unsigned integers outside TOML's signed range, preserving exact digits and existing JSON data."
  - "Retain input-read failure class and reference fields in the existing diagnostic shape."
  - "Retain the two new topic pages, README reduction, and corrections to contradictory CLI documentation."
  - "Repair the deferred million-column bridge under R1-F3 rather than accepting its documented limit."
  - "Retain failing on missing configured editors, allowing a valid VISUAL after an invalid EDITOR, and protecting pre-existing files."
  - "Retain returning fold diagnostics through library APIs without printing warnings about generated command reports; authored-document forwarding is deferred."
  - "Retain unbroken reference and location columns with stacking at narrow widths."
  - "Retain schema-about's removal of its minimum width and its local narrow-table fallback."
  - "Retain reference preparation rejecting malformed inline-code expressions when composition rejects the same input."
  - "Retain the existing references_valid field semantics and the separately documented warning-inclusive human summary count."
  - "Retain reporting the unrelated sandboxing corpus failure transparently; the skipped test is not passing evidence."
  - "Retain the bounded interrupt-flag settling wait after signal-ended commands to preserve exit status 130."
  - "Retain recording native Linux library evidence as incomplete after linker memory failures; WSL2 is separate evidence."
  - "Retain recording final L2/browser evidence as macOS-only rather than claiming cross-platform visual proof."
  - "Retain other CLIs adopting the output trait as future work explicitly outside this spec."
  - "Retain the render-style redesign in its already designated follow-up rather than adding that scope here."
  - "Retain leaving pre-existing unrelated documentation links outside this surgical change."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-05T20:01:25-08:00"
spec: 2026-11-04-composition-errors/spec.md
implemented: false
description: "A **fix** review of `2026-11-04-composition-errors/spec.md`"
fix: 2026-11-04-composition-errors/review-1.md
---
# Composition errors review — iteration 1

The fix is **not production ready**. Five findings remain. All have a recommended repair within the existing specification; none waits for an author decision. There is no earlier implementation review to reconcile.

The shared output design, command migration, reference report, and most authored-position handling are implemented. The remaining problems concern omitted references, discarded locations, an unbounded-width fallback, frontmatter list locations, and verification of the new report in an actual terminal.

## Findings

### R1-F1 (high): HTML reference discovery silently drops multiline tags and media targets

**Authority: defect.** In Darkmatter, [the HTML collector](/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/lib/src/markdown/reference/html.rs:91) splits an HTML node into lines before classifying tags. A plausible authored image, `<img` followed on the next line by ` src="./missing.png">`, therefore disappears from validation: `md validate refs doc.md --output json` exits 0 with zero references. A multiline `<style>` block likewise loses missing stylesheet and font URLs. Additionally, [the graph's reference collector](/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/lib/src/markdown/reference/graph.rs:1084) never calls its existing video, audio, source, or iframe extractors, so even their single-line targets are omitted.

**Class:** reference discovery drops records before validation can classify their targets.

I swept every HTML classifier and both CSS consumers through the shipped CLI. Controls used one tag with one missing local destination; the multiline variant changed only the opening-tag whitespace, or put the CSS body on separate lines. No remote checks were enabled. Each row is part of this finding's repair inventory.

| Instance | Site / shape | Observed | Expected |
|---|---|---|---|
| R1-F1.1 | HTML anchor: one line / attribute on next line | One missing-target issue / zero records | One missing-target issue in both |
| R1-F1.2 | HTML image: same shapes | One missing-target issue / zero records | One missing-target issue in both |
| R1-F1.3 | Script `src`: same shapes | One missing-target issue / zero records | One missing-target issue in both |
| R1-F1.4 | Stylesheet `<link>`: same shapes | One missing-target issue / zero records | One missing-target issue in both |
| R1-F1.5 | CSS `@import`: inline / multiline `<style>` | Missing-target issue / only the inline-style record | Preserve the import and its issue |
| R1-F1.6 | CSS `@font-face` URL: same shapes | Missing-target issue / only the inline-style record | Preserve the font URL and its issue |
| R1-F1.7 | Video `src`: one line / multiline | Zero records in both | Discover and validate the target |
| R1-F1.8 | Audio `src`: same shapes | Zero records in both | Discover and validate the target |
| R1-F1.9 | Source `src`: same shapes | Zero records in both | Discover and validate the target |
| R1-F1.10 | Iframe `src`: same shapes | Zero records in both | Discover and validate the target |
| R1-F1.11 | Meta tag: one line / multiline | One inline metadata record / zero records | Preserve the metadata record; no missing-file error is required for an inline metadata value |
| R1-F1.12 | Markdown link and image controls | Missing targets remain discoverable | Clean; these use separate Markdown extraction |
| R1-F1.13 | Validation decision consuming the record set | Multiline image and CSS documents report valid and exit 0 | Missing targets make the report invalid and exit 1 |
| R1-F1.14 | Compose reference check and graph consumers | Code inspection: both consume this same incomplete graph | They must receive the recovered records; do not fix only report rendering |

Minimal reproductions:

```markdown
<img
 src="./missing.png">
```

```html
<style>
@import "./missing.css";
@font-face { src: url(./missing.woff2); }
</style>
```

Run `md validate refs doc.md --output json`. The inline CSS control reports two missing targets; the multiline version reports none. A one-line `<video src="./missing.mp4"></video>` also reports none.

**Recommended repair:** preserve complete tags and raw-text style bodies while finding their individual source spans, then wire the four existing media extractors into reference collection. Keep this focused on discovery; no new rendering or escaping layer is needed. Add public-API tests for the instance rows, with one CLI case proving the invalid result and status. The log's deliberate CSS deferral does not remove this realistic defect from the review.

### R1-F2 (high): Diagnostic projection discards known source locations

**Authority: spec.** “Other problem families” requires data outputs to “carry locations wherever their producers know them.” The data contract further requires filling `source` “falling back to the composed document when it knows only that.” Darkmatter's [diagnostic projection](/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/lib/src/markdown/compose/report_diagnostics.rs:58) instead turns ordinary warnings and multiple typed errors with file/context information into `source: null`. This prevents an editor or script from opening the document without parsing an error message.

**Class:** converting a typed warning or error to the public diagnostic loses available provenance.

For example, compose this document with `--output json`:

```markdown
---
$schema:
  count: definitely_unknown_type
---
body
```

The failure message names the file, but `dm.schema.prepare` has `source: null`. With `::block when="current.nosuch"`, the condition error has a source context but its diagnostic also has `source: null`; with frontmatter present its message even names a prepared-body line. A frontmatter command `x: "$(echo hello)"` that requires approval returns `dm.shell.failure` with null source despite retaining its context and frontmatter origin. This probe refused approval and ran no command.

The sweep covers the projection branches and their source-bearing callers, including clean branches. All CLI probes used temporary documents and parsed the public JSON result.

| Instance | Site / input | Observed | Expected |
|---|---|---|---|
| R1-F2.1 | Schema preparation / unknown named type | File appears in message; null source | File-only source at minimum |
| R1-F2.2 | Schema validation / wrong scalar type | Authored 2:8 retained | Clean; preserve it |
| R1-F2.3 | Schema problems without a line | Code uses `problem.line.map(...)`, discarding the enclosing known path | Retain the file even without coordinates |
| R1-F2.4 | Page-block condition / parse and evaluation failures | Both null source; message uses prepared line | Preserve file; project authored coordinates where proven |
| R1-F2.5 | Transclusion condition / evaluation failure | Authored 4:1 retained | Clean; use this existing provenance pattern |
| R1-F2.6 | Transclusion condition / parse failure | Null source although condition context and authored line are available | Retain known file and authored position |
| R1-F2.7 | Transclusion directive / omitted target | Null source; message knows file-relative line 4 | Retain context; publish only proven authored coordinates |
| R1-F2.8 | Transclusion cycle / child includes itself | Null source; cycle entries name the child | Preserve the responsible child source rather than substituting the root |
| R1-F2.9 | Shell expansion / frontmatter approval refusal | Null source despite `ctx` and `frontmatter.x` origin | Retain file and known frontmatter location |
| R1-F2.10 | Other shell-expansion variants / shared projection | Code invariant: all non-preflight variants are unlocated even though their variants carry context | Preserve each variant's available context; do not change execution policy |
| R1-F2.11 | Shell-block errors / source-file-bearing variants | `MarkdownError::ShellBlock` always projects null | Retain known file; avoid treating prepared lines as authored |
| R1-F2.12 | Ordinary warnings / undeclared body and frontmatter properties | Message names the file/line; source is null | Preserve known file/line in the record |
| R1-F2.13 | Condition context warning / `ctx.toady` after multiline interpolation | Null source; producer attaches a prepared directive line | Carry the authored directive location before projection |
| R1-F2.14 | Warnings already carrying a diagnostic / legacy link title | Authored 1:1 retained | Clean; retain the shared record |
| R1-F2.15 | Body and frontmatter interpolation errors / division by zero | Authored 1:1 and 2:5 retained | Clean |
| R1-F2.16 | Collection failure / undeclared iterable | Authored source and column retained | Clean |
| R1-F2.17 | Reference issue / missing Markdown target | Authored source, span, and reference details retained | Clean |
| R1-F2.18 | Reference-error projection | Directive parse branch preserves file-only context; file-resolution branch returns null | Preserve file-only information when carried or retain it at the failing call site |
| R1-F2.19 | Frontmatter parsing / malformed YAML | Located parse error retains source | Clean; retain file-only fallback when no parser position exists |
| R1-F2.20 | Interpolation/collection source references without a proven span | `located` rejects `SourceRef::OnDisk` along with genuinely unlocated sources | Retain the known file; leave line/column null |
| R1-F2.21 | Parameter-binding / caller-classification and generic fallback branches | Projection unconditionally returns null | Preserve source if the typed error/caller has it; null remains valid for an actually source-free foreign error |
| R1-F2.22 | Lifecycle error text with no typed source / `error: boom` | Null source | Clean for a source-free error value; a typed lifecycle call finding already carries its own record |
| R1-F2.23 | Blocking-policy and lifecycle delivery paths | Code inspection: both reuse `warning_diagnostic` / `error_diagnostics` | Preserve the same corrected source records through every exit, including earlier warnings |

**Recommended repair:** preserve typed source information centrally, use file-only records when a range cannot be proven, and carry an authored condition-warning location from the producer. Do not guess line 1 or extract paths from message text. Review the source-related comments while repairing: the module's current “no authored position means no source” description is weaker than the spec's file-only fallback. Update the existing family test, which currently asserts null for several source-bearing failures, and test the shared projection in-process; CLI cases should prove delivery rather than multiply every variant into subprocess tests.

### R1-F3 (high): Unlimited component output still uses a million-column sentinel

**Authority: spec.** “Width is an enum” explicitly says width is “never a large number,” and “Under `Unlimited` ... nothing wraps **and nothing fills**.” Biscuit Terminal's new [Terminal::from_output_options](/Volumes/coding/wt/rusty-biscuit/feat-better-research/biscuit-terminal/lib/src/output/profile.rs:72), which adapts command options for existing components, substitutes `1 << 20` for both `Source` and `Unlimited`. A public component can therefore emit megabytes for a tiny result. The log acknowledges this departure; documenting it does not satisfy the decided contract.

**Class:** an unbounded-width state is converted to a numeric fill width at an output adapter.

I compiled a small standalone probe against the current built libraries, without changing repository source:

```rust
use biscuit_terminal::{
    components::{horizontal_rule::HorizontalRule, renderable::TerminalRenderable},
    terminal::Terminal,
};
use renderable::{OutputOptions, TerminalProfile, Width};

fn main() {
for width in [Width::Columns(40), Width::Source, Width::Unlimited] {
    let options = OutputOptions::new(width, TerminalProfile::plain());
    let terminal = Terminal::from_output_options(&options);
    let text = HorizontalRule::new().render(&terminal);
    println!("{width:?}: {} chars, {} bytes", text.chars().count(), text.len());
}
}
```

| Instance | Site / shape | Observed | Expected |
|---|---|---|---|
| R1-F3.1 | Bridge + horizontal rule / `Columns(40)` | 40 characters, 120 bytes | Bounded; control establishes fill behavior |
| R1-F3.2 | Same / `Source` | 1,048,576 characters, 3,145,728 bytes | Natural output, with no manufactured million-column width |
| R1-F3.3 | Same / `Unlimited` | 1,048,576 characters, 3,145,728 bytes | Natural output; plain rule should be `---` |
| R1-F3.4 | Generic terminal fold / rule, table, code corpus | Existing explicit-width fold handles the enum; corpus artifacts stay bounded | Clean; preserve this route |
| R1-F3.5 | Darkmatter page / plain corpus at 40, 80, unlimited | Artifacts stay bounded; existing page/plain tests pass | Clean for the documented corpus |
| R1-F3.6 | CLI bridge callers: toc, delta, graph | CLI controls at 40/source/unlimited remain small on the same document | Clean for tested inputs; adapter still discards the enum |
| R1-F3.7 | CLI bridge callers: code-block, schema-about, schema-validate | Tested plain controls remain bounded, including natural schema-about tables | Clean for tested inputs; styled code-block also uses this adapter |
| R1-F3.8 | CLI bridge callers: clean saved-delta and schema-triggers | Code inspection: both use the same adapter and therefore receive the sentinel | Keep enum semantics through these overrides too; test their successful rendering paths |
| R1-F3.9 | Markdown/browser folds | Their options retain `Width` rather than this adapter | Clean by the typed option contract |

**Recommended repair:** retain the width state through the component bridge and make filling components choose natural output under an unbounded width. Reuse the enum-aware tree fold for overrides where that already expresses the component, rather than introducing a second arbitrary numeric limit. Add one public bridge regression and control cases for filling policies. This is larger than deleting the constant, but replacing it with another finite width leaves the same defect.

### R1-F4 (high): Frontmatter flow-list references do not report their value locations

**Authority: spec.** The authored-position acceptance says a `prologue:` / `epilogue:` reference “points at its frontmatter value,” and every authored reference gets its “exact authored line and character column.” Darkmatter's [frontmatter location projection](/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/lib/src/markdown/reference/graph.rs:383) intentionally falls back to the key for a one-line flow list, and loses the position altogether for a multiline flow list. This also removes the parent line from transclusion trails.

**Class:** a container location is substituted for an individual frontmatter reference's authored value location.

Reproduce with `md validate refs doc.md --output json`:

```yaml
---
prologue: [./a.md, ./b.md]
---
# Top
```

Both missing-file issues report 2:1, pointing at `prologue`, instead of 2:12 and 2:20. Breaking the flow list over lines makes both positions null despite the values being authored on lines 3 and 4. `epilogue` behaves identically. This is ordinary supported YAML authored by a person, not a synthetic malformed payload.

| Instance | Site / shape | Observed | Expected |
|---|---|---|---|
| R1-F4.1 | Prologue scalar | 2:11 | Clean |
| R1-F4.2 | Epilogue scalar | 2:11 | Clean |
| R1-F4.3 | Prologue block sequence | 3:5 and 4:5 | Clean |
| R1-F4.4 | Epilogue block sequence | 3:5 and 4:5 | Clean |
| R1-F4.5 | Prologue one-line flow sequence | Both 2:1 | 2:12 and 2:20 |
| R1-F4.6 | Epilogue one-line flow sequence | Both 2:1 | 2:12 and 2:20 |
| R1-F4.7 | Prologue multiline flow sequence | Null positions | 3:3 and 4:3 |
| R1-F4.8 | Epilogue multiline flow sequence | Null positions | 3:3 and 4:3 |
| R1-F4.9 | Prologue child traversal / multiline flow sequence of existing children with broken links | Child positions correct; both parent trail lines null | Parent trail lines 3 and 4 |
| R1-F4.10 | Epilogue child traversal / same shape | Same missing parent lines | Parent trail lines 3 and 4 |
| R1-F4.11 | Report and diagnostic consumers | Both read the incorrect shared authored location | Correct the shared location; preserve it in data and location links |

**Recommended repair:** locate each scalar in flow sequences as a value, using the existing YAML source-position machinery or a focused extension for these sequence leaves. Preserve character columns and CRLF behavior. Replace the test named `a_flow_list_value_falls_back_to_its_key`, which currently pins the spec violation, with exact-value tests for both properties and a child-trail case. No new schema validation policy or alternate syntax is needed.

### R1-F5 (high): The new styled reference report lacks real-terminal verification

**Authority: spec and the review's Test Rigor requirement.** The report requires colored reason rows, dimmed locations, aligned columns, and readable narrow-width output. The review instructions require Level 2 evidence for actual terminal rendering. The new report's strongest dedicated checks are Level 1 tests with spoofed profiles and the `script` pseudo-terminal artifact; neither is an actual emulator or multiplexer capture. Existing Level 2 tests exercise other error blocks, not this grouped reference report.

**Class:** tests verify generated terminal bytes without crossing the terminal-rendering boundary required by the visual contract.

| Instance | Site / requirement | Present evidence | Required evidence |
|---|---|---|---|
| R1-F5.1 | Grouped report reason colors and dimmed location | Level 1 report tests; escaped pseudo-terminal artifact | Level 2 capture through a real terminal backend |
| R1-F5.2 | Aligned report columns and narrow-width stacked fallback | Level 1 rendered-string assertions | Level 2 capture retaining every reference/location at a representative narrow width |
| R1-F5.3 | Existing schema/expression/file-reference error blocks | Level 2 tests in `level2_errors` | Clean for those older blocks; they do not cover the new grouped report |
| R1-F5.4 | Plain escape suppression, JSON streams, exit codes | Level 1 byte/process assertions | Clean; an emulator adds no evidence to these byte contracts |
| R1-F5.5 | Keyboard behavior | No keyboard interaction added by this fix | No Level 3 test required |

**Recommended repair:** add a focused Level 2 test for the existing root/child report fixture using the repository terminal harness, asserting the displayed groups, references, locations, semantic styling, and one narrow-width layout. Register it in the declared Level 2 target and run `just test-l2` with the relevant filter. Use a background pane/window through the harness; never activate a terminal or inject host keyboard events. There is no need for a terminal matrix for every output format.

## Verification and scope

I reviewed the specification, its two design spikes, the plan/implementation records, relevant production paths, output schemas and conformance inventory, report/provenance tests, and the recorded formatted-output artifacts. The two requested decision-inventory agents read the log and spec in parallel before the implementation comparison.

Local verification in this review used the package's nextest recipes:

| Check | Result | Level / purpose |
|---|---|---|
| `just test reference_report::` | 44 passed | Level 1: report construction, locations, data, CLI wiring |
| `just test output_` | 449 passed | Level 1: output selection, envelopes, schemas, streams, width, compatibility |
| `just test compose_report_diagnostics::` | 10 passed | Level 1: diagnostic delivery; several expectations need correction under R1-F2 |
| `just test reference_provenance::` | 38 passed | Level 1: authored positions and trails; flow-list expectation needs correction under R1-F4 |
| `just check-tier-coverage darkmatter` | Passed; zero stranded tests | Test-target and tier registration |
| Shipped CLI probes | Reproduced R1-F1, R1-F2, R1-F4 | Level 1: real command invocation, temporary files, public JSON |
| Standalone public-library probe | Reproduced R1-F3 | Level 1: public component bridge and renderer |

The tests inspected are declared through the library/CLI `tests/l1/main.rs` modules and their explicit Cargo targets. The CLI test-fixtures feature is included in package test metadata. No permanent test was added or renamed by this review.

Requirements concerning output parsing, legacy-flag rejection, schema conformance, exit codes, stderr isolation, source-coordinate arithmetic, Markdown reflow, and JSON5 link metadata have appropriate Level 1 tests. Plain byte equivalence and no-escape assertions are also Level 1 contracts. Existing terminal and headless browser suites provide evidence for shared page/component rendering, but do not substitute for the dedicated new-report Level 2 coverage in R1-F5. The implementation log records their macOS runs; this review does not claim to have rerun those visual suites or obtained new cross-OS evidence.

The input-robustness configuration matrix does not apply to these changed text-grammar, source-projection, output-serialization, and rendering paths. The JSON5 link-title parser is part of Markdown text grammar. Output-schema documents are contracts consumed by an existing loader, not a newly changed configuration-loading policy. No finding is manufactured from permissive parsing outside the specification.

The implementation reports a skipped existing shipped-schema corpus test because a concurrently authored research schema uses planned dynamic enum syntax. That skip is an evidence limitation, not a passing result. Native Linux library evidence was also incomplete after linker memory failures. Neither cross-OS evidence nor the unrelated in-progress document is used as a readiness finding here.

## Implementation decisions (informational)

The author may inspect or reverse these recommendations **after** the repair cycle. No answer is required before repair. The numbered list follows all 68 entries of `implementation-log.md`'s `decisions_for_review` in order, followed by additional decisions and deferrals recorded elsewhere in the log. Multi-part entries retain their related choices together. Items that cannot be retained as implemented name the ordinary finding that repairs them.

The spec explicitly allows decided names to change when shipped documentation records them, and delegates new command data shapes to implementation. Its unchanged-default-bytes promise supports retaining compose's pre-existing cleanup-frontmatter behavior rather than introducing an unrelated default-output change. The new bare-value wrappers, TOML large-integer strings, opener behavior, and editor failure behavior all have a recommended safe choice; none meets the stopping test when a recommendation is available within the agreed contract.

1. Retain fractional width values until resolution so flooring can produce a warning.
2. Retain Darkmatter warning codes in the CLI while the shared library returns neutral warning text.
3. Retain the explicit plain profile as the switch for plain structural rendering.
4. Retain the old direct-terminal constructor behavior outside the new output route; repair the new unlimited bridge under R1-F3.
5. Retain request-captured terminal environment values instead of adding ambient reads to detection.
6. Retain prose-element browser width constraints; nested code remains subject to its containing list item.
7. Retain Markdown wrap-point selection that avoids accidentally introducing block syntax.
8. Retain styled code truncation without ellipses and the Mermaid drawing exception.
9. Retain authored destinations for verbose nonclickable links and the unified image fallback.
10. Retain forwarding verbosity to schema-validation and Sniff link rendering and updating affected pipe assertions.
11. Retain the existing broad legacy-title detector, including its treatment of prose containing an assignment.
12. Retain biscuit-file's existing pretty TOML layout.
13. Retain the additive Json5::from_value constructor to avoid serialization followed by reparsing.
14. Retain failing compose when its reference check cannot run, including previously swallowed out-of-tree and child failures.
15. Retain passive, lenient reference preparation and its frontmatter interpolation so executable values can wait for composition.
16. Retain applying document recoveries before passive reference checking, with remote/cache configuration kept afterward.
17. Retain authored and post-interpolation link-title scans, including warnings inside inactive blocks.
18. Retain link-title warnings and their denial codes in md render.
19. Retain clean-only legacy-title migration, preserving unsupported prose and leaving expression-bearing or image-shared titles unchanged.
20. Retain the existing nonreference warning-row test because the plan incorrectly described it as a report-view assertion.
21. Retain the shared borderless table and detail-row handling; the early boxed-overhead defect was repaired later.
22. Retain the report's terminal tree override for bare file URLs and terminal link parameters.
23. Retain warnings-only report titles, summary arithmetic, stdin labels, canonical relative paths, and destination delimiter encoding as documented.
24. Retain BOM, CRLF, literal-frontmatter, and synthesized-TOC choices; repair flow-list value positions under R1-F4.
25. Retain existing fragment-validation behavior while reporting final numbered section slugs separately.
26. Retain proven source-text locations, but repair discarded known diagnostic source information under R1-F2.
27. Repair deferred multiline HTML/CSS discovery under R1-F1 and deferred condition-warning location propagation under R1-F2.
28. Retain the explicit test color profile that removes ambient terminal dependence.
29. Retain classifying the sandboxing corpus failure as cross-platform rather than marking Windows alone as failing.
30. Retain the log's reconstruction of changes from shared-worktree commits; no history rewrite is requested by this review.
31. Retain moving lifecycle stdout actions and command output to stderr for formatted composition, including prose rendering.
32. Retain source bytes for implicit unknown-width render Markdown; explicit unlimited width still joins paragraphs.
33. Retain the envelope document without the final newline added by command printing, preserving the previous envelope convention.
34. Retain data-mode show printing and opening the payload, including failures, and treating opener failure as a failed run.
35. Retain the existing opener test seam extended with MD_DRY_RUN=fail.
36. Retain refusing interactive approval and suppressing tracing initialization for data output.
37. Retain the single global width argument rather than duplicating it at every command leaf.
38. Retain moving max-width onto render options so both explicit and implicit render accept it.
39. Retain compose's existing cleanup-frontmatter behavior to honor the spec's explicit unchanged-default-bytes promise.
40. Retain the final shared selector for schema-about pipe output; the temporary optimistic profile was removed later.
41. Retain removing width-implies-styling behavior and honoring explicit hyperlink and italic settings.
42. Retain routing multiple-heading notices through tracing instead of library stderr writes.
43. Retain render style diagnostics for page/data output, with source Markdown unchanged.
44. Retain preparing one target eagerly so rendering errors occur before output delivery.
45. Retain removing duplicate generic lifecycle diagnostics after a reference refusal.
46. Retain updating Claudine's stale terminal fallback assertion to match the shared rendering change.
47. Retain silent terminal/plain removal saves and explicit Markdown/HTML removal summaries.
48. Retain clean's existing diagnostic rows and use run_diagnostics for invocation-level records.
49. Retain wrapping get/schema-detect warning and failure values to protect user-owned object keys.
50. Retain simple prose/code projections for newly supported non-document formatted output.
51. Retain classifying incompatible directory-hash options and invalid highlight arguments as usage errors.
52. Retain removing unreachable code-block show handling; adding that feature is outside this spec.
53. Retain schema-validation problems in per-file records without duplicating every problem into run diagnostics.
54. Retain omitting schema-validation verbose timings from data output because the spec adds compose perf only.
55. Retain stripping escapes at the CLI boundary for status components that still paint unconditionally.
56. Retain Unix script-backed edit-success tests; native Windows success coverage remains an informational limitation.
57. Retain stacking report cells when a borderless row cannot fit instead of hiding its locations.
58. Retain decimal strings for unsigned integers outside TOML's signed range, preserving exact digits and existing JSON data.
59. Retain input-read failure class and reference fields in the existing diagnostic shape.
60. Retain the two new topic pages, README reduction, and corrections to contradictory CLI documentation.
61. Repair the deferred million-column bridge under R1-F3 rather than accepting its documented limit.
62. Retain failing on missing configured editors, allowing a valid VISUAL after an invalid EDITOR, and protecting pre-existing files.
63. Retain returning fold diagnostics through library APIs without printing warnings about generated command reports; authored-document forwarding is deferred.
64. Retain unbroken reference and location columns with stacking at narrow widths.
65. Retain schema-about's removal of its minimum width and its local narrow-table fallback.
66. Retain reference preparation rejecting malformed inline-code expressions when composition rejects the same input.
67. Retain the existing references_valid field semantics and the separately documented warning-inclusive human summary count.
68. Retain reporting the unrelated sandboxing corpus failure transparently; the skipped test is not passing evidence.
69. Retain the bounded interrupt-flag settling wait after signal-ended commands to preserve exit status 130.
70. Retain recording native Linux library evidence as incomplete after linker memory failures; WSL2 is separate evidence.
71. Retain recording final L2/browser evidence as macOS-only rather than claiming cross-platform visual proof.
72. Retain other CLIs adopting the output trait as future work explicitly outside this spec.
73. Retain the render-style redesign in its already designated follow-up rather than adding that scope here.
74. Retain leaving pre-existing unrelated documentation links outside this surgical change.
