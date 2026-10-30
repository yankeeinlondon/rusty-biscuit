---
area: biscuit-terminal
status: draft-spec
created: 2026-10-15
depends-on:
    - 2026-10-15-upgraded-rendering
packages:
    - biscuit-visualized
    - biscuit-terminal
    - biscuit-terminal-cli
implemented: false
reviewed: false
---

# Compare Merman and mermaid-rs-renderer

## Problem and Intended Outcome

We need evidence to decide whether Merman should replace `mermaid-rs-renderer`
as our Mermaid backend. Newer diagram syntax and mindmap fidelity are motivating
concerns, but a replacement must be evaluated across all diagram families and
against the performance and integration behavior we already rely on.

Build a reproducible visual gallery and performance report comparing Merman with
our existing renderer. Then stop at a human decision checkpoint. Implementing
this spec does not authorize a production migration.

## Prerequisite and Baseline

Implement this work after `2026-10-15-upgraded-rendering` has completed its
implementation and validation. That work upgrades shared SVG dependencies and
evaluates unreleased changes to the current Mermaid renderer. Do not benchmark
against a baseline that is still changing underneath this comparison.

The primary incumbent is the exact renderer version or revision selected by that
work, running through the resulting `biscuit-visualized` integration. Record
released 0.3.1 as a separate historical baseline if the selected incumbent is an
unreleased revision. Label every column with its actual version and revision;
never substitute historical 0.3.1 results for the production baseline.

Select and pin a Merman release or full commit at implementation time. Record its
Rust requirement, feature selection, layout engines, licenses and notices, and
configuration. Do not assume Zed's pinned version or examples on Merman's moving
main branch match the version chosen for this experiment.

## Scope and Ownership

- Put the comparison harness, corpus, and artifact generation in the
  `biscuit-visualized` area, which owns diagram rendering.
- Put the report describing the results in the area's `docs/` tree and link to
  it from the terminal area's Mermaid research document. Current documentation
  must describe the experiment directly without linking to this feature spec.
- Exercise `biscuit-terminal` adapters for representative sizing, display, and
  fallback checks. Keep parsing and layout out of the terminal package.
- Keep experimental Merman dependencies isolated from ordinary production
  builds, using a dedicated harness/tool manifest where practical. Avoid adding
  a public backend-selection API solely for the experiment.
- Use Merman directly and its available raster-safe pipeline. Do not copy or
  link Zed's GPL integration wrapper. Build any host theme mapping independently.

Do not change the production backend, graphics protocols, public diagram APIs,
or defaults. A production migration requires a subsequent authorized change.

## Complete Diagram Corpus

Inventory the union of diagram families supported by either selected renderer.
Every family gets a gallery entry and report row, including families unsupported
by the other renderer. Record exact syntax spellings and layout variants rather
than treating a diagram-family support claim as full syntax compatibility.

The initial checklist includes all 23 families advertised by
`mermaid-rs-renderer`: flowchart/graph, sequence, class, state, ER, pie, XY,
quadrant, Sankey, Gantt, timeline, journey, Kanban, C4, block, architecture,
requirement, mindmap, gitGraph, ZenUML, packet, radar, and treemap. Add any
additional families found in the chosen Merman version. For C4 and other families
with multiple supported forms, inventory and exercise the supported variants.

For each family, include:

1. A minimal valid diagram that establishes basic support.
2. A representative diagram with realistic labels and branching or data.
3. A targeted challenge case, such as dense connections, deep nesting, long
   labels, multiple series, or unusual supported shapes.

Reuse suitable upstream fixtures with provenance and required attribution rather
than inventing redundant examples. Keep a frozen local corpus so upstream edits
cannot change the experiment. Record any family-specific exception to these
three cases with a reason in the manifest; unsupported input is a result, not a
reason to omit the family.

Include these specific regression surfaces:

- Mindmap radial/default, tidy-tree, and lr-tree modes where available; deep
  branches, root shapes, long labels, and multiline labels. Include the upstream
  issue #7572 reproduction and record actual versus literal newline handling.
- The user's Research mindmap if its exact source becomes available. Do not
  reconstruct it from the screenshot and present it as the original input.
- Labeled gitGraph merges and overlapping tags covered by our existing repairs.
- Flowchart subgraphs, direction overrides, and edge labels.
- Radar data preservation and labels, state fork/join, and requirement text.
- Unicode labels and explicit source styling/configuration on a representative
  subset, to expose font or configuration differences.

The source must be identical between candidates. If a renderer needs an input
translation, display that as a separately named adapted case with the original,
translation, and rationale; preserve the unmodified-input result.

## Visual Comparison

Generate a self-contained local HTML gallery containing side-by-side SVG and PNG
results, source text, configuration, versions, timing summaries, and errors. It
must remain usable without network access and provide navigation by diagram
family, fixture, theme, and result status.

Run the corpus in light and dark themes with a documented mapping of equivalent
semantic settings: fonts, font size, backgrounds, text, borders, and branch
colors. Show transparency behavior on a representative subset. Preserve explicit
source styling and record configuration precedence differences.

For the main comparison, use the same font files, available fonts, rasterizer
version, target PNG width, and background policy for both candidates. Keep SVG
viewBox and natural dimensions available for inspection. The browser display
must preserve aspect ratio rather than stretch unequal outputs into matching
boxes. Optional native/default-theme views must be clearly separated from the
comparison using matched settings.

Identify failures by stage: parse/configuration, layout/SVG, rasterization, or
terminal display. Missing output, unsupported syntax, timeouts, and crashes must
appear as explicit results. Preserve diagnostics without dropping failed rows.

Record per-fixture findings for semantic correctness, clipping, overlaps,
connectivity, label readability, and layout quality. Use focused geometry or
semantic assertions where available; visual inspection remains necessary.
If official Mermaid output is needed to resolve a disputed result, pin its
version and record it as a reference, without making a browser runtime part of
either native candidate's measured rendering path.

## Performance Measurements

Measure both candidates on the same host, build profile, inputs, fonts, and
output settings. Use release builds with matching optimization settings and
record the effective settings. Build time and dependency compilation are outside
render latency and may be reported separately.

Report these operations separately:

| Measurement | Boundary |
| ----------- | -------- |
| Cold SVG | Fresh worker process, first render; includes process startup, initialization, and font setup |
| Warm SVG | Repeated source-to-final-SVG calls in one initialized worker, including required SVG cleanup |
| Warm SVG-to-PNG | Rasterization of the prepared SVG with the common rasterizer |
| Warm source-to-PNG | Full rendering and rasterization, including required corrections and cleanup |

Do not label source-to-SVG as parser-only or layout-only timing. Internal stage
timings may be added only when both candidates expose comparable boundaries.
Explain any measurement that includes startup or excludes initialization.

Bypass our artifact cache for primary render measurements and verify that actual
rendering occurs. A separate cache-hit measurement may characterize host behavior
but must not be used to compare renderer speed. Record any unavoidable internal
renderer caching and whether repeated calls reparse and recompute layout.

Use at least 10 fresh-process samples for cold measurements. For warm results,
perform at least 5 unmeasured warmups and 30 measured iterations per successful
fixture/theme/candidate combination. Report sample count, median, p95, minimum,
and maximum. Interleave or alternate candidates to reduce order bias. Retain raw
samples and use identical, explicitly recorded time and resource limits.

Report speed ratios alongside absolute times and scope aggregates by family and
fixture size. Do not claim overall speed superiority from a handful of tiny
diagrams. A warmup failure or resource-limit breach is a failure row, not an
omitted sample. Run untrusted or pathological fixtures in workers that the harness
can terminate without hanging the report generation.

Record OS, architecture, CPU, memory, Rust/toolchain versions, lockfiles, renderer
revisions, common rasterizer, font identities, sample policy, and configuration
in a machine-readable run manifest. Use `sniff` for host discovery and
`biscuit-hash` if content fingerprints are needed. Timing uses a monotonic clock.
Peak memory and output size are useful secondary measurements if collected
consistently; mark unavailable metrics explicitly.

## Reproducibility and Validation

- One documented command generates the gallery, machine-readable results, raw
  timings, run manifest, and a readable report under an explicit output directory.
  A small smoke mode is available before the full experiment.
- Validate the manifest against the inventory: no missing family, fixture,
  candidate, or requested theme. Failed and unsupported cells count as recorded
  outcomes, never as successful coverage.
- Check that successful SVGs are parseable, PNG dimensions and alpha behavior
  meet the request, gallery references resolve locally, and report aggregates
  agree with raw data. Add focused tests for failure accounting and aggregation.
- Run relevant package checks through existing `just` and nextest workflows.
  Do not add the full comparison or performance run to routine CI. Freeze only
  fixtures needed by meaningful regression checks; generated reports are local
  artifacts unless deliberately selected for publication.
- Keep harness code portable across macOS, Linux, native Windows, and WSL2.
  A primary performance report may come from one named host; it must not imply
  cross-platform performance evidence. Identify additional platform validation
  needed before a migration decision can be considered fully supported.
- Representative terminal checks must preserve fallback and image dimensions.
  Terminal/browser windows must never gain focus during automated tests; gallery
  generation must not automatically open a browser.

## Human Decision Checkpoint

End implementation with a reviewable report containing:

1. A complete support matrix, visual gallery location, and reproduction command.
2. Improvements and regressions by diagram family, with examples and severity.
3. Absolute latency distributions and ratios, with measurement limitations.
4. Integration costs: configuration, themes, font behavior, raster-safe cleanup,
   dependency footprint, Rust requirements, notices, platform evidence, cache
   identity changes, and existing corrections that a migration must revisit.
5. A recommendation to migrate, retain the incumbent, or investigate named gaps,
   clearly separated from the observed results.

Do not collapse visual fidelity and performance into a single score. No automatic
speed or support threshold authorizes migration. The author makes the tradeoff
after reviewing the evidence. Record the decision and any conditions in the
implementation log; if no decision has arrived, report that the comparison is
ready for that checkpoint and leave production behavior unchanged.

## Acceptance Criteria

- The prerequisite is complete and the exact resulting baseline is recorded.
- All families in the selected renderers' support union are represented, with
  requested fixtures/themes and explicit failures or unsupported outcomes.
- The offline gallery supports visual inspection of both SVG and PNG output.
- The performance report follows the declared sampling and cache policy and
  retains raw evidence sufficient to reproduce its summaries.
- Mindmap differences and existing local corrections receive explicit review.
- A report and recommendation are ready for the human migration decision.
- Production dependencies, renderer selection, and public APIs are unchanged by
  the experiment; implementation completion does not constitute migration.

Keep implementation evidence and departures from this draft in an adjacent
`implementation-log.md`. The author closes the review cycle and moves the
feature into `_completed` when appropriate.
