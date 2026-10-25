---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - |-
        Review the current macOS and Linux diagram galleries beside the Zed screenshots. Approve their appearance or identify the diagram types and visual changes wanted. Include the repaired sequence labels and the preserved author-colored venn label. This is the existing appearance review, not a new approval request.
    - |-
        Resolve the four styling choices already presented: passing palette colors through renderer configuration, static fills for diagram types without rotating fills, replacing the entire compiled theme when an author supplies a theme color, and matching sequence participants by name. Retain all four, identify individual choices to change, or request another implementation of the original styling rules. The readability defect is repaired independently of these choices.
    - |-
        Review the repair's preservation of explicit author text colors on every fill. The venn fixture's author-selected dark gray label remains at 1.29:1 contrast on Dark; library-selected labels are corrected. Retain author text colors as shipped, or request recoloring where they fail on library-selected fills. This carries the previous review's decision forward without treating it as approved.
    - |-
        Review the PNG compression tradeoff: zlib level 1 with the Up filter preserves pixels and meets the render-time limit, with files about twice as large in the measured timeline example. Retain this choice, or request a separately evaluated compression approach that preserves the speed limit. No new encoder is required by this review.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-23T17:05:39-07:00"
spec: 2026-10-21-merman/spec.md
implemented: false
description: "A **feature** review of `2026-10-21-merman/spec.md`"
feature: 2026-10-21-merman/review-3.md
previous: 2026-10-21-merman/review-2.md
---

# Merman implementation review, iteration 3

The feature is **production ready under this review's readiness rules**. All four numbered findings from the previous review are verified fixed, including their required sibling instances. There are no new findings. Appearance approval and the previously disclosed styling choices remain for human review; neither is recorded here as approved.

The author's separate Darkmatter key-order repair remains assigned to `2026-10-23-frontmatter-key-order`. This review does not reopen that accepted assignment, claim its completion, or claim that every dependent package's full suite was rerun.

## Previous Finding Verification

Read [review 2](review-2.md), [review 1](review-1.md), both implementation logs, and the repair's complete `### Finding Ledger` before assessing repairs. The required instance ranges below include **all 213 child rows**, including those previously classified clean. Verification uses the implementation and current test results rather than ledger completion labels.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: Library-selected text becomes unreadable on author-selected fills | R2-F1.1–R2-F1.33: every family, unedited and white `primaryColor`; R2-F1.34–R2-F1.37: sequence fixed and derived palettes; R2-F1.38: SVG, PNG, HTML static; R2-F1.39: author-text classification | verified fixed | All 33 white-fill tests and all corpus contrast tests passed in the current package run. The SVG correction now runs with or without a compiled theme; stylesheet-painted regions are resolved through usvg. The three sequence exemptions are removed. Code inspection confirms correction precedes target selection and rasterization. Author color classification and its focused tests preserve explicit author text choices. | None of the original defect remains. The author-colored venn label is a disclosed styling choice, not an unrepaired library-selected label. |
| R2-F2: Graph PNG cache ignores the fonts used to draw text | R2-F2.1–R2-F2.7: expression/DOT × PNG/SVG, Mermaid keys, terminal consumers, standalone rasterization | verified fixed | Expression and DOT regression tests passed: a different injected face misses the shared cache, changes pixels, and equals a fresh render with that face. SVG reuse controls passed. PNG keys include the resolved chain for the actual graph label stack, and that same chain reaches rasterization. Mermaid retains its font identity; standalone rasterization has no artifact cache. The graph component delegates to the repaired graph renderer; the render-tree diagram hook renders Mermaid, already keyed. | None. The previous review's description of a graph render-tree hook was imprecise; no separate graph cache consumer was missed. |
| R2-F3: Recorded sequence and timeline renders exceed the warm limit | R2-F3.1–R2-F3.33: all saved reference-host family measurements and their limit decisions | verified fixed | Independently checked every entry of the replacement `build-linux` baseline: 33/33 satisfy warm ≤50 ms and cold <500 ms. Sequence is 33.815 ms warm; timeline is 40.756 ms warm and 57.527 ms cold. Inspected retained mappings, in-place alpha conversion, encoder settings, and cache invalidation. The benchmark still checks absolute limits separately from baseline regression. | None against the supplied measurement evidence. This review did not make a fresh Linux performance measurement. |
| R2-F4: Level-1 rendering tests exceed the five-second budget | R2-F4.1–R2-F4.134: each named slow test and all remaining sibling modules/binaries in the ledger | verified fixed | Current `just test`: 976 passed, 3 excluded, 20.837 s test execution; slowest executed test 2.650 s, zero over 5 s. Every listed module is compiled and selected. Smoke tests use bundled faces while a public host-font render remains; measurement tests retain host-font coverage. The macOS setup changes the CoreText bundle directory without dropping assertions or moving tests to another tier. Tier audit: zero stranded tests. | None. This establishes the local repair, not a promise about every future runner load. |
| R1-H1: Gallery appearance approval | Current macOS and Linux themed galleries | still blocked | No recorded human appearance approval in the supplied reviews. The galleries and snapshot tests cannot substitute for it. | Existing external human review; this was a decision row, not a numbered defect. |
| R1-H2: Four styling departures | Configuration colors, static fills, whole-theme replacement, participant name matching | still blocked | The disclosed implementations remain in the design, preparation, and accent paths; no human ruling approves or rejects them. | Existing external styling review; no new decision requested. |
| R1-H3: Sequence-box contrast | Dark, full-ANSI, and background-only sequence boxes | verified fixed | Covered by R2-F1.34–R2-F1.39 above; current oracle passes without the former sequence exemptions. | None. |
| R1-H4: Sequence/timeline warm limit | Both failing families and the complete baseline | verified fixed | Covered by the 33-entry baseline audit for R2-F3. | None against the recorded reference-host evidence. |
| R1-H5: Dependent-suite and terminal verification | Separate Darkmatter key-order repair and previously reported worktree Kitty case | still blocked | The original implementation log records the author's explicit assignment of the key-order repair to `2026-10-23-frontmatter-key-order`; it also identifies the Kitty case as pre-existing. The later repair log reports narrowed diagram-dependent runs, which are not full dependent-suite proof. | The separately assigned repair and its full Darkmatter gate remain external. No new Merman defect is established by those old verification records, and no claim of all-package or Kitty success is made. |
| R1-H6: Graph font cache identity | Both constructors, both formats, downstream use | verified fixed | Covered by every R2-F2 child above. | None. |

The `R1-H*` names are the earlier repair ledger's decision-row labels, not omitted numbered findings. Human approval alone and work explicitly assigned by the author to another repair do not become new findings in this review.

## Unblocked Findings

None.

## Blocked Findings

None. Pending human decisions are listed in frontmatter and do not block `ready` under the requested rules.

## Recurrence

No finding is carried or newly issued, so `recurrence: false`. The repeated classes reported in review 2 are repaired; none was observed to regress. There is no partial repair or newly discovered sibling to distinguish from those fixes.

## Verification by requirement and level

| User-facing requirement | Strongest relevant verification inspected | Assessment |
| --- | --- | --- |
| Render supported families to SVG/PNG under the four fixed themes | Level 1: family registry guard, smoke renders, SVG snapshots, decoded PNG comparisons | Appropriate for an artifact-producing library. Current suite passes. |
| Readable library-selected labels, including terminal-derived palettes and author-selected fills | Level 1: independent text-free raster contrast oracle over every stored SVG, plus the 33-family white-fill sweep | Current suite passes; explicit author text choices remain intentionally preserved. |
| Match text measurement to drawing, choose fonts and fallback consistently | Level 1: injected-font resolution tests, mixed-script and wrapping oracle, installed-font tests | Current suite passes. The repair retains installed-font verification. |
| Invalidate cached output for source, palette, font, target, or PNG settings changes | Level 1: key tests plus cached-versus-fresh pixel comparisons | Current suite passes; graph and Mermaid use the chain they key. |
| Space gitGraph tags while retaining topology and author spacing | Level 1: layout, geometry, spacing, malformed-layout and rendered-tag tests | Current package suite passes. Inspected the measured spacing and geometry consumers; no new defect found. |
| Preserve HTML static labels and sanitize shadows while rejecting external references | Level 1: HTML-static target tests in this package; inspected Darkmatter sanitizer and declared static-SVG integration tests. Consumer browser tests also exist. | Target-specific font/overflow edits stay out of PNG. This review reran this package's tests, not Darkmatter's full suite or browser tier. |
| Bound renderer work and retain source fallback on failure | Level 1: low-budget component and geometry tests, warning capture, invalid/unsupported-family cases | Current suite passes; both Merman request types receive operation deadlines. |
| Parse terminal ANSI replies, cache the query, and suppress disallowed queries | Level 1: inspected parser cases and manufactured PTY startup-exchange tests in biscuit-terminal | These test the explicitly specified parsing/query contract. They do not claim real-emulator reply behavior or keyboard encoding. No keyboard interaction is introduced by this feature. |
| Meet warm/cold speed limits and flag regressions | Release benchmark evidence from the designated Linux host; independently audited baseline and enforcement code | Every recorded family satisfies the absolute limits. The local benchmark test run is a wiring/loader check, not a reference-host timing run. |
| Approve diagram appearance beside Zed | Human visual review | Still pending, as required by the specification; external to code readiness. |

No modifier, hotkey, paste, IME, or mouse behavior is introduced here. Artifact colors and layout can be checked in-process; terminal display remains a consumer boundary. No missing Level-3 keyboard test is inferred from merely displaying a diagram. The earlier pre-existing Kitty verification case is not newly attributed to this renderer.

## Input Robustness Matrix

The Mermaid text grammar and generated SVG are outside this matrix's stated scope. The file-format reader introduced here is the benchmark JSON baseline; Merman's internal layout JSON is also read and validated.

**Default outcome:** invalid load-bearing shapes fail with a field path or document error before a baseline comparison or geometry result is produced. Inspected both strict loaders, including duplicate-key detection and end-of-document checks. Neither silently filters malformed collection elements. The lone parent-edge `filter_map` follows validation that every named parent exists.

| Shared decision | Valid control and exceptions | Invalid-shape coverage inspected |
| --- | --- | --- |
| Baseline numeric statistics | Positive finite median/p95; p95 must be at least median | Absent, null, string, empty array/object, zero/negative, invalid numeric literals; shared validator serves warm/cold statistics. |
| Baseline object and family readers | Valid saved baseline; `{}` families explicitly means no baseline per family, which the comparison reports separately | Required-field checks, wrong object shapes, unknown keys/families; strict document loader rejects duplicate keys at multiple depths and trailing garbage. CLI wiring tests include successful comparison and invalid-baseline rejection without overwriting the file. |
| Layout identities, coordinates, collections, direction, bounds | Frozen real Merman document; empty parents means a root and empty tags means no tags | Located missing/null/wrong-type cases, invalid elements, identity/reference checks, malformed and duplicate-key documents, trailing data; spacing and geometry consume validated values. Existing author spacing is deliberately honored, including a value Merman ignores. |

No additional configuration-shape finding was established. The checks above do not impose a new external file-format contract on Mermaid source text or SVG output.

## Commands and evidence limits

- `just test` in `biscuit-visualized`: 976 passed, 3 excluded by the recipe's filter; no executed test exceeded five seconds.
- `just lint` in `biscuit-visualized`: passed with all features and all targets.
- `just check-tier-coverage biscuit-visualized`: zero stranded tests.
- `cargo nextest run --manifest-path biscuit-visualized/bench/Cargo.toml`: 46 passed, zero skipped; slowest wiring case 1.689 s. These tests exercise baseline validation and CLI enforcement, without rewriting the saved baseline.
- Audited all 33 entries of `bench/baseline.json` against the absolute limits; reviewed the benchmark's uncached warm/cold timing boundaries and enforcement.

No production source was repaired, no snapshots were regenerated, and no commit was created by this review. Cross-OS proof and human appearance approval are not inferred from these local results. The prior review's `implemented` and `implemented_by` values are preserved.
