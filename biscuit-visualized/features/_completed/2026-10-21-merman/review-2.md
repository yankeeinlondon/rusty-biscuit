---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (high): Library-selected text becomes unreadable on author-selected fills"
    - "R2-F2 (high): Graph PNG cache ignores the fonts used to draw text"
    - "R2-F3 (high): Recorded sequence and timeline renders exceed the warm limit"
    - "R2-F4 (medium): Level-1 rendering tests exceed the five-second budget"
human_review: true
human_review_items:
    - |-
        Review the current macOS and Linux diagram galleries beside the Zed screenshots and approve their appearance or name the diagram types and visual changes you want. This remains the visual review requested in the first review; snapshot generation does not constitute approval.
    - |-
        Record the still-pending styling choice from the first review: retain palette colors passed through renderer configuration, static fills for families without rotating fills, whole-theme replacement after an author color override, and sequence participant matching by name; or identify which approaches must change. These are the same four choices already presented, not new requests. Retaining them still requires repairing the unreadable library-selected text reported here.
    - |-
        R2-F1 repair decision: text colors the author writes (CSS `color:`, C4 `$fontColor`/`$textColor`, text/label/title/font theme keys) are never rewritten, even on a library fill; as a result the unedited venn "AB" label (author `color:#333`) is now kept at 1.29:1 on Dark. Confirm, or restore the old "or the fill is a palette color" clause (which would recolor C4's author red/blue text).
    - |-
        R2-F3 repair decision: PNG encoding switched to zlib level 1 with the `Up` filter to meet the 50 ms warm rule (timeline 84 → 41 ms on build-linux); pixels are unchanged but default PNGs are about 2× larger (timeline 142 → 282 KB). Confirm, or ask for a custom parallel deflate encoder to keep sizes.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-23T15:05:51-07:00"
spec: 2026-10-21-merman/spec.md
implemented: true
description: "A **feature** review of `2026-10-21-merman/spec.md`"
feature: 2026-10-21-merman/review-2.md
previous: 2026-10-21-merman/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-merman/log.md
implemented_by: claude/opus
next: 2026-10-21-merman/review-3.md
---

# Merman implementation review, iteration 2

The feature is **not production ready**. Four findings remain: unreadable library-selected labels, stale graph PNGs after a font change, recorded performance limit overruns, and rendering tests exceeding the five-second test budget. All four can be repaired against the current specification without waiting for appearance approval. No source repairs are included in this review.

The specification, plan, both implementation logs, previous review, renderer, font/cache paths, styling passes, geometry reader, benchmark loader, and relevant test targets were inspected. The previous review deliberately performed no code review. This is the first implementation assessment, rather than verification of a completed code repair.

## Previous Finding Verification

`review-1.md` has `findings: []`. Its repair's `### Finding Ledger` in `log.md` explicitly states that no source changes were made. Thus there are **zero previous numbered findings to mark verified fixed**. The following table also accounts for every decision row in that ledger; the `R1-H*` labels identify ledger rows, not previously issued defect IDs. Neither completion metadata nor pending decisions demonstrate a repair.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-H1: Gallery appearance approval | Current macOS and Linux galleries, including later readability changes | still blocked | No `## Human rulings` or recorded approval in the previous review; `human_resolved: false` | Existing appearance review; human review alone does not block `ready` |
| R1-H2: Four styling departures | Site configuration colors; static fills; whole-theme response to an author color; participant name matching | still blocked | `design.rs`, `render.rs`, and `accents.rs` still implement the disclosed choices; no author ruling recorded | Record the existing choice; this review does not invent a ruling or treat the implementer's disclosure as approval |
| R1-H3: Sequence-box contrast | Dark, full-ANSI, and background-only sequence goldens; library-selected text on `box LightGrey` | unresolved | Public SVG render and independent text/background oracle reproduce 1.073:1; test allow-list still exempts all three cases | R2-F1 carries the unchanged instance and newly found author-theme siblings; no partial repair occurred |
| R1-H4: Sequence/timeline warm limit | All benchmark families on the Linux reference host, particularly sequence and timeline | unresolved | Direct inspection of `bench/baseline.json`: 56.381 ms and 84.306 ms; benchmark comparison still uses 50 ms | R2-F3; no accepted exception or repaired measurement exists in the supplied record |
| R1-H5: Dependent-suite and terminal verification | All packages named by the dependent-suite acceptance criterion; previously failing worktree terminal case | unresolved | Repair ledger says no code/tests changed. Implementation log still records the separate Darkmatter key-order repair and unresolved Kitty result | Existing separate repair and test gates remain; no claim of cross-package success is made here. The accepted assignment to `2026-10-23-frontmatter-key-order` is retained, not reopened or duplicated |
| R1-H6: Graph font cache identity | Expression and DOT input, PNG and SVG output, downstream components | unresolved | Public API in separate processes: Inter then Arial gives PNG cache hits; a fresh Arial cache gives different PNG bytes | R2-F2; SVG controls are unchanged, narrowing the repair to font-dependent output |

None of the three defects below was repaired between reviews. The new family instances of unreadable text are newly identified siblings, not regressions caused by the empty repair cycle.

## Unblocked Findings

### R2-F1 (high): Library-selected text becomes unreadable on author-selected fills

**Authority: spec.** The styling requirement says “Text color is chosen for contrast against the fill behind it, and every text element meets 4.5:1,” and the golden-text acceptance criterion explicitly includes derived palettes. The existing sequence fixture and ordinary author configuration both violate that contract. This finding concerns text selected by the library/renderer, rather than deliberately poor text colors supplied by an author.

**Defect class:** readability correction is skipped, or cannot determine the painted background, when author styling changes the surface behind library-selected text.

In biscuit-visualized, [`MermaidDiagram::render_svg`](../../src/src/mermaid/render.rs:673), which produces SVG before either export or PNG rasterization, calls `readable_text` only when `prepared.design` exists. An author color removes that design, thereby removing the readability pass. For example, on a dark diagram:

```mermaid
%%{init: {'themeVariables': {'primaryColor': '#ffffff'}}}%%
flowchart LR
  A[Start] --> B[Finish]
```

The author selects only the white node fill. The resulting `Start` and `Finish` labels remain `#cccccc`, giving **1.606:1** on white. Unedited controls are readable. The same happens with sequence participants and class labels. Independently, the existing sequence fixture's `box LightGrey Alice and Bob` leaves message and section text at **1.073:1** on `#d3d3d3`, even when a compiled design exists: [`readable_text`](../../src/src/mermaid/contrast.rs:52) does not resolve those stylesheet-painted regions/text. The golden oracle currently allow-lists that failure.

**Sweep and reproduction.** Every family fixture in `src/src/tests/fixtures/merman/families/` was rendered through public `MermaidDiagram::render` as a dark SVG, first unchanged, then changing only `themeVariables.primaryColor` to white. The directive was inserted **after** any YAML frontmatter, preserving its existing configuration. Each output was passed to the existing independent oracle, which resolves text through usvg and samples the actual text-free raster background. Temporary diagnostic tests were removed afterward. These observations are not passing readability assertions.

The table lists all 33 sibling families, including clean controls. “Control failures” includes existing explicitly author-colored C4 and venn text; those intentional author choices are excluded from this finding. An edited failure count includes those same failures; the added failures are the repair inventory. A clean row means this exact reproduction stayed readable, not that agreement between two rendering paths proved correctness.

| Child ID | Site (family fixture) | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | `agentflow` | Unedited / white `primaryColor` | 0 control / 2 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.2 | `architecture` | Unedited / white `primaryColor` | 0 control / 0 edited failures; clean | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.3 | `block` | Unedited / white `primaryColor` | 0 control / 5 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.4 | `c4` | Unedited / white `primaryColor` | 5 control / 14 edited failing texts; minimum 1.012:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.5 | `class` | Unedited / white `primaryColor` | 0 control / 15 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.6 | `cynefin` | Unedited / white `primaryColor` | 0 control / 7 edited failing texts; minimum 1.282:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.7 | `er` | Unedited / white `primaryColor` | 0 control / 26 edited failing texts; minimum 1.287:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.8 | `eventmodeling` | Unedited / white `primaryColor` | 0 control / 12 edited failing texts; minimum 1.063:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.9 | `flowchart` | Unedited / white `primaryColor` | 0 control / 6 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.10 | `gantt` | Unedited / white `primaryColor` | 0 control / 16 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.11 | `gitgraph` | Unedited / white `primaryColor` | 0 control / 3 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.12 | `info` | Unedited / white `primaryColor` | 0 control / 0 edited failures; clean | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.13 | `ishikawa` | Unedited / white `primaryColor` | 0 control / 5 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.14 | `journey` | Unedited / white `primaryColor` | 0 control / 4 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.15 | `kanban` | Unedited / white `primaryColor` | 0 control / 5 edited failing texts; minimum 4.362:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.16 | `mindmap` | Unedited / white `primaryColor` | 0 control / 1 edited failing texts; minimum 2.910:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.17 | `packet` | Unedited / white `primaryColor` | 0 control / 31 edited failing texts; minimum 1.662:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.18 | `pie` | Unedited / white `primaryColor` | 0 control / 2 edited failing texts; minimum 1.109:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.19 | `quadrantchart` | Unedited / white `primaryColor` | 0 control / 10 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.20 | `radar` | Unedited / white `primaryColor` | 0 control / 0 edited failures; clean | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.21 | `railroad` | Unedited / white `primaryColor` | 0 control / 6 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.22 | `requirement` | Unedited / white `primaryColor` | 0 control / 9 edited failing texts; minimum 1.162:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.23 | `sankey` | Unedited / white `primaryColor` | 0 control / 0 edited failures; clean | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.24 | `sequence` | Unedited / white `primaryColor` | 13 control / 23 edited failing texts; minimum 1.073:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.25 | `state` | Unedited / white `primaryColor` | 0 control / 9 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.26 | `swimlane` | Unedited / white `primaryColor` | 0 control / 6 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.27 | `timeline` | Unedited / white `primaryColor` | 0 control / 0 edited failures; clean | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.28 | `treeView` | Unedited / white `primaryColor` | 0 control / 11 edited failing texts; minimum 1.662:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.29 | `treemap` | Unedited / white `primaryColor` | 0 control / 0 edited failures; clean | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.30 | `usecase` | Unedited / white `primaryColor` | 0 control / 9 edited failing texts; minimum 1.606:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.31 | `venn` | Unedited / white `primaryColor` | 1 control / 3 edited failing texts; minimum 1.051:1 | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.32 | `wardley` | Unedited / white `primaryColor` | 0 control / 0 edited failures; clean | Library-selected text ≥4.5:1; preserve explicit author text choices |
| R2-F1.33 | `xychart` | Unedited / white `primaryColor` | 0 control / 0 edited failures; clean | Library-selected text ≥4.5:1; preserve explicit author text choices |

The decisions consuming those colors were also checked:

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.34 | Sequence golden, Dark | Existing `box LightGrey`, no added directive | 13 texts below the floor; 1.073:1; allowed by test | Correct library-selected text on the light box |
| R2-F1.35 | Sequence golden, terminal ANSI palette | Same existing box | Oracle allow-list requires the failure still to exist; original golden contrast test passes with exemption | Correct text; remove the exemption |
| R2-F1.36 | Sequence golden, background-only terminal palette | Same existing box | Same active exemption | Correct text; remove the exemption |
| R2-F1.37 | Sequence golden, Default/Forest/Neutral | Same existing box | Clean under the corpus oracle, with no matching sequence exemptions | Stay readable |
| R2-F1.38 | SVG export, PNG raster input, HTML static output | Shared `render_svg` result | Code invariant: all use the same pre-output correction branch; rasterization and HTML font/overflow edits do not repair text paint | Repair before the paths diverge |
| R2-F1.39 | Contrast-test classification | Library-selected text versus explicit author text | Sequence library text is exempted alongside intentional author colors | Keep only explicit author text exemptions; assert this demonstrated defect fails until repaired |

**Smallest repair.** Preserve the author's fills and continue correcting library-selected text when the compiled theme is disabled. Add focused handling for the existing sequence-box region. A general CSS engine or a new protection/escaping layer would be disproportionate; this finding does not request one. Use representative public renders plus the existing oracle to guard the shared decisions, and keep the family sweep's clean cases clean. Retaining the previously proposed whole-theme response to author colors does not require disabling contrast correction.

### R2-F2 (high): Graph PNG cache ignores the fonts used to draw text

**Authority: defect.** An ordinary caller changing `MERMAID_FONT` from Inter to Arial gets a previously cached graph drawn in the old font. The shared rasterizer now uses the font-resolution chain, but biscuit-visualized's [`GraphDiagram::cache_key`](../../src/src/graph/render.rs:427), which selects a cached graph artifact, includes no identity for that chain. The graph's requested CSS font string is insufficient: the environment override changes the resolved font without changing that string.

**Defect class:** raster output depends on resolved fonts that are absent from its cache identity.

**Reproduction.** In separate processes sharing a fresh `BISCUIT_VISUALIZED_CACHE_DIR`, render `GraphDiagram::from_expression("Start -> Finish")` and `GraphDiagram::from_dot("digraph { Start -> Finish; }")` with default requests. First set `MERMAID_FONT=Inter`, then `MERMAID_FONT=Arial`. Repeat Arial with a fresh cache. Both shared-cache Arial PNG requests report `cache_hit: true`; the fresh Arial PNGs differ byte-for-byte from the returned Inter PNGs. Source, request, and graph theme remain identical. SVG controls are byte-identical between the font settings, as expected: their stored text has not been rasterized.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F2.1 | Expression → PNG | Inter → Arial, same cache | Same key `8196462308943357392`, hit; fresh Arial bytes differ | A font change misses and returns the Arial render |
| R2-F2.2 | DOT → PNG | Same font change | Same key `11803635337487597980`, hit; fresh Arial bytes differ | Same font-sensitive invalidation |
| R2-F2.3 | Expression → SVG | Same font change | Clean: stored SVG bytes unchanged | Font-independent SVG may reuse its key |
| R2-F2.4 | DOT → SVG | Same font change | Clean: stored SVG bytes unchanged | Same |
| R2-F2.5 | Mermaid → PNG/SVG | Resolved-font identity | Clean implementation: `cache_key_for` includes `chain.cache_identity`; existing one-input-at-a-time font-key tests exercise it | Retain font-sensitive identity |
| R2-F2.6 | GraphExpression terminal component and graph render-tree hook | Same graph source/request | Both delegate to `GraphDiagram::render`; the cache hit precedes rasterization, so consumers cannot repair it | Shared graph cache fix reaches both consumers |
| R2-F2.7 | Standalone SVG rasterization API | Same SVG, different resolved font | Clean contract: no artifact cache lookup; each call draws with the current process chain | No new cache layer needed |

**Smallest repair.** Include the font chain identity in graph PNG keys, using the existing font identity implementation. This naturally prevents reuse of old font-independent PNG keys. Keep SVG caching unchanged where the output bytes do not depend on resolved fonts. Cover both graph source constructors and prove the new font's pixels are returned; merely checking a key string misses the original harm.

### R2-F3 (high): Recorded sequence and timeline renders exceed the warm limit

**Authority: spec.** The speed rule says “no diagram family exceeds a **50 ms median** for a warm full render: measure + layout + SVG + post-process + raster.” Its reference-host decision applies that limit on the Linux build host. The saved release benchmark records **56.381 ms** for sequence and **84.306 ms** for timeline there. Neither the previous review nor the repair log contains an accepted exception.

**Defect class:** two measured render paths fail the absolute per-family performance requirement even though their values are stored as the regression baseline.

This is an audit of the supplied reference-host measurements, **not a newly measured Linux run**. The primary data are [`bench/baseline.json`](../../bench/baseline.json), independently inspected rather than inferred from the completion summary. [`report`](../../bench/src/main.rs:318), which decides whether a benchmark run fails, correctly enforces the absolute limit separately from comparison with that baseline. A baseline entry is therefore not an exemption. The repair cycle made no performance changes or replacement measurements.

**Bounded sweep.** All 33 baseline families were compared with both absolute limits using their recorded warm and cold medians. Every cold median is below 500 ms. The two failing warm families and all clean siblings are listed below. Each row is the fixture-backed full-render path measured by the shipped benchmark; the same check is applied to every row.

| Child ID | Site / family | Shape tested | Observed result (warm / cold median, ms) | Expected result |
| --- | --- | --- | --- | --- |
| R2-F3.1 | `agentflow` | Recorded uncached full render | 6.192 / 15.546; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.2 | `architecture` | Recorded uncached full render | 19.756 / 30.841; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.3 | `block` | Recorded uncached full render | 11.998 / 22.818; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.4 | `c4` | Recorded uncached full render | 39.888 / 52.161; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.5 | `class` | Recorded uncached full render | 22.601 / 35.038; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.6 | `cynefin` | Recorded uncached full render | 25.515 / 39.250; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.7 | `er` | Recorded uncached full render | 29.677 / 41.843; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.8 | `eventmodeling` | Recorded uncached full render | 45.301 / 59.216; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.9 | `flowchart` | Recorded uncached full render | 27.252 / 41.678; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.10 | `gantt` | Recorded uncached full render | 13.328 / 23.578; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.11 | `gitgraph` | Recorded uncached full render | 13.369 / 23.275; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.12 | `info` | Recorded uncached full render | 2.444 / 11.613; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.13 | `ishikawa` | Recorded uncached full render | 19.068 / 30.318; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.14 | `journey` | Recorded uncached full render | 25.375 / 39.705; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.15 | `kanban` | Recorded uncached full render | 42.078 / 54.186; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.16 | `mindmap` | Recorded uncached full render | 38.503 / 53.306; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.17 | `packet` | Recorded uncached full render | 19.607 / 31.932; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.18 | `pie` | Recorded uncached full render | 18.860 / 30.379; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.19 | `quadrantchart` | Recorded uncached full render | 14.551 / 25.100; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.20 | `radar` | Recorded uncached full render | 27.051 / 39.555; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.21 | `railroad` | Recorded uncached full render | 14.008 / 24.892; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.22 | `requirement` | Recorded uncached full render | 13.511 / 23.795; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.23 | `sankey` | Recorded uncached full render | 20.807 / 33.109; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.24 | `sequence` | Recorded uncached full render | 56.381 / 73.124; warm fails | Warm ≤50 ms; cold <500 ms |
| R2-F3.25 | `state` | Recorded uncached full render | 26.657 / 38.681; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.26 | `swimlane` | Recorded uncached full render | 28.770 / 43.657; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.27 | `timeline` | Recorded uncached full render | 84.306 / 98.032; warm fails | Warm ≤50 ms; cold <500 ms |
| R2-F3.28 | `treeView` | Recorded uncached full render | 8.444 / 18.014; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.29 | `treemap` | Recorded uncached full render | 25.792 / 37.214; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.30 | `usecase` | Recorded uncached full render | 22.926 / 33.955; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.31 | `venn` | Recorded uncached full render | 18.855 / 31.310; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.32 | `wardley` | Recorded uncached full render | 28.425 / 43.362; clean | Warm ≤50 ms; cold <500 ms |
| R2-F3.33 | `xychart` | Recorded uncached full render | 14.700 / 26.643; clean | Warm ≤50 ms; cold <500 ms |

**Smallest repair.** Profile and reduce work in the two failing reference-host paths, then rerun those families in release with cache bypass. Keep the benchmark's existing absolute checks. If the author instead chooses explicit exceptions, record that changed contract and its scope; no such choice is inferred here. A new general performance framework or CI benchmark cell is unnecessary.

### R2-F4 (medium): Level-1 rendering tests exceed the five-second budget

**Authority: defect.** The review's test contract explicitly counts an L1 test over five seconds as failing. The restored package suite, run with **four test threads**, still produces the over-budget cases inventoried below. These are ordinary committed rendering tests with their normal fixtures, not the temporary diagnostic probes. The earlier default-concurrency run also reported 144 slow cases before its interruption.

**Defect class:** render-verification tests perform enough font initialization, repeated rendering, or corpus work in one test process to exceed the required per-test budget under suite load.

The family smoke helpers in [`mermaid_family_tests.rs`](../../src/src/tests/mermaid_family_tests.rs:35) render all four themes per test through the production entry point, which initializes the installed-font chain. Other direct public-render tests do the same initialization; the tag-size tests compare geometry and drawing in each process. The existing golden and layout tests demonstrate an alternative using injected bundled faces where installed-font behavior is not being tested. The sequence contrast test already uses injected faces, so its timing needs examination of the corpus/oracle workload rather than a blanket font-only fix.

The timings below are observations on this host under suite load, not a claim that every test exceeds five seconds in isolation or on every machine. That distinction makes this **medium**, but does not satisfy the explicit suite budget. No parser mismatch or deadline assertion failure is inferred from a slow line.

**Smallest repair.** Keep targeted coverage of real host-font resolution, but use the existing injected-face path for deterministic artifact checks that do not need installed fonts. Profile the remaining corpus cases and split independently meaningful matrix cases where necessary, retaining a guard that checks the full case list. Do not hide them with a `slow_` marker, skip, retry, or longer nextest timeout. The required outcome is that these L1 cases stay within five seconds in the package run without dropping their assertions.

**Sweep.** Every completed test in the final restored-source L1 run was checked by the same elapsed-time rule. The child rows below enumerate every case over the threshold; the following module table includes clean siblings and the count checked, so repair verification can retain that coverage. Temporary probe timings are excluded.

| Child ID | Site (package and test path) | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F4.1 | `biscuit-visualized components::git_graph::tests::every_emitted_graph_parses` | Committed L1 fixture, four-thread suite | 7.035 s | ≤5 s, with assertions retained |
| R2-F4.2 | `biscuit-visualized components::git_graph::tests::measured_plans_place_merges_and_tags_without_overlap` | Committed L1 fixture, four-thread suite | 8.612 s | ≤5 s, with assertions retained |
| R2-F4.3 | `biscuit-visualized components::git_graph::tests::measured_sizes_trim_to_the_width_cap` | Committed L1 fixture, four-thread suite | 6.845 s | ≤5 s, with assertions retained |
| R2-F4.4 | `biscuit-visualized components::git_graph::tests::render_without_notes_leaves_the_notes_to_the_caller` | Committed L1 fixture, four-thread suite | 6.160 s | ≤5 s, with assertions retained |
| R2-F4.5 | `biscuit-visualized components::mermaid::tests::a_render_over_its_time_budget_shows_the_code_block_with_a_warning` | Committed L1 fixture, four-thread suite | 5.660 s | ≤5 s, with assertions retained |
| R2-F4.6 | `biscuit-visualized components::mermaid::tests::diagram_try_render_populates_width_cells` | Committed L1 fixture, four-thread suite | 6.433 s | ≤5 s, with assertions retained |
| R2-F4.7 | `biscuit-visualized components::mermaid::tests::scaled_columns_come_from_the_measured_svg_width` | Committed L1 fixture, four-thread suite | 8.655 s | ≤5 s, with assertions retained |
| R2-F4.8 | `biscuit-visualized tests::contrast_corpus_tests::sequence_text_is_readable` | Committed L1 fixture, four-thread suite | 6.175 s | ≤5 s, with assertions retained |
| R2-F4.9 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_inherit_is_measured_as_drawn` | Committed L1 fixture, four-thread suite | 5.095 s | ≤5 s, with assertions retained |
| R2-F4.10 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_not_a_size_is_measured_as_drawn` | Committed L1 fixture, four-thread suite | 6.042 s | ≤5 s, with assertions retained |
| R2-F4.11 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_relative_to_an_overridden_base_is_measured_as_drawn` | Committed L1 fixture, four-thread suite | 18.423 s | ≤5 s, with assertions retained |
| R2-F4.12 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_unitless_string_is_measured_as_drawn` | Committed L1 fixture, four-thread suite | 18.074 s | ≤5 s, with assertions retained |
| R2-F4.13 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_unset_is_measured_as_drawn` | Committed L1 fixture, four-thread suite | 17.955 s | ≤5 s, with assertions retained |
| R2-F4.14 | `biscuit-visualized tests::gitgraph_tests::tag_font_size_zero_is_measured_as_drawn` | Committed L1 fixture, four-thread suite | 16.236 s | ≤5 s, with assertions retained |
| R2-F4.15 | `biscuit-visualized tests::gitgraph_tests::the_spaced_layout_is_the_measured_one` | Committed L1 fixture, four-thread suite | 9.394 s | ≤5 s, with assertions retained |
| R2-F4.16 | `biscuit-visualized tests::html_static_tests::default_target_svg_keeps_original_fonts_and_clipping` | Committed L1 fixture, four-thread suite | 5.244 s | ≤5 s, with assertions retained |
| R2-F4.17 | `biscuit-visualized tests::html_static_tests::html_static_target_svg_leads_label_fonts_with_the_measured_face` | Committed L1 fixture, four-thread suite | 5.725 s | ≤5 s, with assertions retained |
| R2-F4.18 | `biscuit-visualized tests::html_static_tests::png_render_rasterizes_the_raster_safe_svg_under_the_html_static_target` | Committed L1 fixture, four-thread suite | 6.020 s | ≤5 s, with assertions retained |
| R2-F4.19 | `biscuit-visualized tests::measure_tests::measured_faces_are_drawn_faces_for_the_wrap_corpus_on_host_fonts` | Committed L1 fixture, four-thread suite | 6.404 s | ≤5 s, with assertions retained |
| R2-F4.20 | `biscuit-visualized tests::measure_tests::oracle_widths_match_usvg_for_the_mixed_script_corpus` | Committed L1 fixture, four-thread suite | 7.071 s | ≤5 s, with assertions retained |
| R2-F4.21 | `biscuit-visualized tests::mermaid_cache_key_tests::the_key_follows_the_rewritten_source` | Committed L1 fixture, four-thread suite | 5.124 s | ≤5 s, with assertions retained |
| R2-F4.22 | `biscuit-visualized tests::mermaid_cache_key_tests::the_key_is_checked_before_any_merman_svg_call` | Committed L1 fixture, four-thread suite | 5.423 s | ≤5 s, with assertions retained |
| R2-F4.23 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_agentflow::png` | Committed L1 fixture, four-thread suite | 5.135 s | ≤5 s, with assertions retained |
| R2-F4.24 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_agentflow::svg` | Committed L1 fixture, four-thread suite | 5.120 s | ≤5 s, with assertions retained |
| R2-F4.25 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_c4::png` | Committed L1 fixture, four-thread suite | 5.898 s | ≤5 s, with assertions retained |
| R2-F4.26 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_c4::svg` | Committed L1 fixture, four-thread suite | 5.028 s | ≤5 s, with assertions retained |
| R2-F4.27 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_class::png` | Committed L1 fixture, four-thread suite | 6.453 s | ≤5 s, with assertions retained |
| R2-F4.28 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_class::svg` | Committed L1 fixture, four-thread suite | 5.877 s | ≤5 s, with assertions retained |
| R2-F4.29 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_cynefin::png` | Committed L1 fixture, four-thread suite | 6.275 s | ≤5 s, with assertions retained |
| R2-F4.30 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_cynefin::svg` | Committed L1 fixture, four-thread suite | 6.002 s | ≤5 s, with assertions retained |
| R2-F4.31 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_er::png` | Committed L1 fixture, four-thread suite | 7.909 s | ≤5 s, with assertions retained |
| R2-F4.32 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_er::svg` | Committed L1 fixture, four-thread suite | 6.575 s | ≤5 s, with assertions retained |
| R2-F4.33 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_event_modeling::png` | Committed L1 fixture, four-thread suite | 6.665 s | ≤5 s, with assertions retained |
| R2-F4.34 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_event_modeling::svg` | Committed L1 fixture, four-thread suite | 6.289 s | ≤5 s, with assertions retained |
| R2-F4.35 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_flowchart::png` | Committed L1 fixture, four-thread suite | 6.580 s | ≤5 s, with assertions retained |
| R2-F4.36 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_flowchart::svg` | Committed L1 fixture, four-thread suite | 6.313 s | ≤5 s, with assertions retained |
| R2-F4.37 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_gantt::png` | Committed L1 fixture, four-thread suite | 7.662 s | ≤5 s, with assertions retained |
| R2-F4.38 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_gantt::svg` | Committed L1 fixture, four-thread suite | 7.072 s | ≤5 s, with assertions retained |
| R2-F4.39 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_gitgraph::png` | Committed L1 fixture, four-thread suite | 7.808 s | ≤5 s, with assertions retained |
| R2-F4.40 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_gitgraph::svg` | Committed L1 fixture, four-thread suite | 7.074 s | ≤5 s, with assertions retained |
| R2-F4.41 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_info::png` | Committed L1 fixture, four-thread suite | 6.180 s | ≤5 s, with assertions retained |
| R2-F4.42 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_info::svg` | Committed L1 fixture, four-thread suite | 6.090 s | ≤5 s, with assertions retained |
| R2-F4.43 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_ishikawa::png` | Committed L1 fixture, four-thread suite | 5.952 s | ≤5 s, with assertions retained |
| R2-F4.44 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_ishikawa::svg` | Committed L1 fixture, four-thread suite | 5.683 s | ≤5 s, with assertions retained |
| R2-F4.45 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_journey::png` | Committed L1 fixture, four-thread suite | 6.150 s | ≤5 s, with assertions retained |
| R2-F4.46 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_journey::svg` | Committed L1 fixture, four-thread suite | 5.910 s | ≤5 s, with assertions retained |
| R2-F4.47 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_kanban::png` | Committed L1 fixture, four-thread suite | 7.749 s | ≤5 s, with assertions retained |
| R2-F4.48 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_kanban::svg` | Committed L1 fixture, four-thread suite | 6.459 s | ≤5 s, with assertions retained |
| R2-F4.49 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_mindmap::png` | Committed L1 fixture, four-thread suite | 7.546 s | ≤5 s, with assertions retained |
| R2-F4.50 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_mindmap::svg` | Committed L1 fixture, four-thread suite | 6.335 s | ≤5 s, with assertions retained |
| R2-F4.51 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_quadrant_chart::png` | Committed L1 fixture, four-thread suite | 5.280 s | ≤5 s, with assertions retained |
| R2-F4.52 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_railroad::png` | Committed L1 fixture, four-thread suite | 5.276 s | ≤5 s, with assertions retained |
| R2-F4.53 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_requirement::png` | Committed L1 fixture, four-thread suite | 5.209 s | ≤5 s, with assertions retained |
| R2-F4.54 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_sequence::png` | Committed L1 fixture, four-thread suite | 5.463 s | ≤5 s, with assertions retained |
| R2-F4.55 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_state::png` | Committed L1 fixture, four-thread suite | 5.774 s | ≤5 s, with assertions retained |
| R2-F4.56 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_swimlane::png` | Committed L1 fixture, four-thread suite | 5.564 s | ≤5 s, with assertions retained |
| R2-F4.57 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_timeline::png` | Committed L1 fixture, four-thread suite | 6.525 s | ≤5 s, with assertions retained |
| R2-F4.58 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_timeline::svg` | Committed L1 fixture, four-thread suite | 6.215 s | ≤5 s, with assertions retained |
| R2-F4.59 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_tree_view::png` | Committed L1 fixture, four-thread suite | 5.491 s | ≤5 s, with assertions retained |
| R2-F4.60 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_tree_view::svg` | Committed L1 fixture, four-thread suite | 5.503 s | ≤5 s, with assertions retained |
| R2-F4.61 | `biscuit-visualized tests::mermaid_family_tests::smoke_renders_usecase::png` | Committed L1 fixture, four-thread suite | 5.092 s | ≤5 s, with assertions retained |
| R2-F4.62 | `biscuit-visualized tests::mermaid_tests::an_svg_render_over_its_time_budget_fails_with_a_warning` | Committed L1 fixture, four-thread suite | 5.111 s | ≤5 s, with assertions retained |
| R2-F4.63 | `biscuit-visualized tests::mermaid_tests::backgrounds_paint_the_theme_a_solid_color_or_nothing` | Committed L1 fixture, four-thread suite | 5.078 s | ≤5 s, with assertions retained |
| R2-F4.64 | `biscuit-visualized tests::mermaid_tests::mermaid_render_separates_backgrounds_and_geometry` | Committed L1 fixture, four-thread suite | 5.239 s | ≤5 s, with assertions retained |
| R2-F4.65 | `biscuit-visualized tests::mermaid_tests::mermaid_transparent_background_changes_svg_output` | Committed L1 fixture, four-thread suite | 5.019 s | ≤5 s, with assertions retained |

Clean sibling groups from the same full run (the over-budget cases above are excluded from these counts):

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F4.66 | `biscuit-visualized components::display::tests` | Remaining 6 committed L1 cases | All ≤5 s; max 0.029 s | Keep coverage and budget |
| R2-F4.67 | `biscuit-visualized components::git_graph::tests` | Remaining 63 committed L1 cases | All ≤5 s; max 4.804 s | Keep coverage and budget |
| R2-F4.68 | `biscuit-visualized components::graph_expression::tests` | Remaining 8 committed L1 cases | All ≤5 s; max 0.035 s | Keep coverage and budget |
| R2-F4.69 | `biscuit-visualized components::mermaid::tests` | Remaining 21 committed L1 cases | All ≤5 s; max 4.693 s | Keep coverage and budget |
| R2-F4.70 | `biscuit-visualized mermaid::render::tests` | Remaining 3 committed L1 cases | All ≤5 s; max 0.023 s | Keep coverage and budget |
| R2-F4.71 | `biscuit-visualized tests` | Remaining 11 committed L1 cases | All ≤5 s; max 1.129 s | Keep coverage and budget |
| R2-F4.72 | `biscuit-visualized tests::artifact_tests` | Remaining 5 committed L1 cases | All ≤5 s; max 0.033 s | Keep coverage and budget |
| R2-F4.73 | `biscuit-visualized tests::bundled_font_tests` | Remaining 2 committed L1 cases | All ≤5 s; max 0.023 s | Keep coverage and budget |
| R2-F4.74 | `biscuit-visualized tests::cache_identity_tests` | Remaining 9 committed L1 cases | All ≤5 s; max 0.037 s | Keep coverage and budget |
| R2-F4.75 | `biscuit-visualized tests::cache_tests` | Remaining 14 committed L1 cases | All ≤5 s; max 0.029 s | Keep coverage and budget |
| R2-F4.76 | `biscuit-visualized tests::contrast_corpus_tests` | Remaining 37 committed L1 cases | All ≤5 s; max 4.997 s | Keep coverage and budget |
| R2-F4.77 | `biscuit-visualized tests::corpus_measure_tests` | Remaining 2 committed L1 cases | All ≤5 s; max 0.023 s | Keep coverage and budget |
| R2-F4.78 | `biscuit-visualized tests::corpus_measure_tests::agentflow` | Remaining 2 committed L1 cases | All ≤5 s; max 0.123 s | Keep coverage and budget |
| R2-F4.79 | `biscuit-visualized tests::corpus_measure_tests::architecture` | Remaining 2 committed L1 cases | All ≤5 s; max 0.110 s | Keep coverage and budget |
| R2-F4.80 | `biscuit-visualized tests::corpus_measure_tests::block` | Remaining 2 committed L1 cases | All ≤5 s; max 0.139 s | Keep coverage and budget |
| R2-F4.81 | `biscuit-visualized tests::corpus_measure_tests::c4` | Remaining 2 committed L1 cases | All ≤5 s; max 1.184 s | Keep coverage and budget |
| R2-F4.82 | `biscuit-visualized tests::corpus_measure_tests::class` | Remaining 2 committed L1 cases | All ≤5 s; max 0.550 s | Keep coverage and budget |
| R2-F4.83 | `biscuit-visualized tests::corpus_measure_tests::cynefin` | Remaining 2 committed L1 cases | All ≤5 s; max 0.109 s | Keep coverage and budget |
| R2-F4.84 | `biscuit-visualized tests::corpus_measure_tests::er` | Remaining 2 committed L1 cases | All ≤5 s; max 0.792 s | Keep coverage and budget |
| R2-F4.85 | `biscuit-visualized tests::corpus_measure_tests::event_modeling` | Remaining 2 committed L1 cases | All ≤5 s; max 0.474 s | Keep coverage and budget |
| R2-F4.86 | `biscuit-visualized tests::corpus_measure_tests::flowchart` | Remaining 2 committed L1 cases | All ≤5 s; max 0.373 s | Keep coverage and budget |
| R2-F4.87 | `biscuit-visualized tests::corpus_measure_tests::gantt` | Remaining 2 committed L1 cases | All ≤5 s; max 0.483 s | Keep coverage and budget |
| R2-F4.88 | `biscuit-visualized tests::corpus_measure_tests::gitgraph` | Remaining 2 committed L1 cases | All ≤5 s; max 0.264 s | Keep coverage and budget |
| R2-F4.89 | `biscuit-visualized tests::corpus_measure_tests::info` | Remaining 2 committed L1 cases | All ≤5 s; max 0.058 s | Keep coverage and budget |
| R2-F4.90 | `biscuit-visualized tests::corpus_measure_tests::ishikawa` | Remaining 2 committed L1 cases | All ≤5 s; max 0.433 s | Keep coverage and budget |
| R2-F4.91 | `biscuit-visualized tests::corpus_measure_tests::journey` | Remaining 2 committed L1 cases | All ≤5 s; max 0.214 s | Keep coverage and budget |
| R2-F4.92 | `biscuit-visualized tests::corpus_measure_tests::kanban` | Remaining 2 committed L1 cases | All ≤5 s; max 1.302 s | Keep coverage and budget |
| R2-F4.93 | `biscuit-visualized tests::corpus_measure_tests::mindmap` | Remaining 2 committed L1 cases | All ≤5 s; max 0.945 s | Keep coverage and budget |
| R2-F4.94 | `biscuit-visualized tests::corpus_measure_tests::packet` | Remaining 2 committed L1 cases | All ≤5 s; max 0.370 s | Keep coverage and budget |
| R2-F4.95 | `biscuit-visualized tests::corpus_measure_tests::pie` | Remaining 2 committed L1 cases | All ≤5 s; max 0.128 s | Keep coverage and budget |
| R2-F4.96 | `biscuit-visualized tests::corpus_measure_tests::quadrant_chart` | Remaining 2 committed L1 cases | All ≤5 s; max 0.187 s | Keep coverage and budget |
| R2-F4.97 | `biscuit-visualized tests::corpus_measure_tests::radar` | Remaining 2 committed L1 cases | All ≤5 s; max 0.127 s | Keep coverage and budget |
| R2-F4.98 | `biscuit-visualized tests::corpus_measure_tests::railroad` | Remaining 2 committed L1 cases | All ≤5 s; max 0.188 s | Keep coverage and budget |
| R2-F4.99 | `biscuit-visualized tests::corpus_measure_tests::requirement` | Remaining 2 committed L1 cases | All ≤5 s; max 0.248 s | Keep coverage and budget |
| R2-F4.100 | `biscuit-visualized tests::corpus_measure_tests::sankey` | Remaining 2 committed L1 cases | All ≤5 s; max 0.102 s | Keep coverage and budget |
| R2-F4.101 | `biscuit-visualized tests::corpus_measure_tests::sequence` | Remaining 2 committed L1 cases | All ≤5 s; max 0.904 s | Keep coverage and budget |
| R2-F4.102 | `biscuit-visualized tests::corpus_measure_tests::state` | Remaining 2 committed L1 cases | All ≤5 s; max 0.425 s | Keep coverage and budget |
| R2-F4.103 | `biscuit-visualized tests::corpus_measure_tests::swimlane` | Remaining 2 committed L1 cases | All ≤5 s; max 0.314 s | Keep coverage and budget |
| R2-F4.104 | `biscuit-visualized tests::corpus_measure_tests::timeline` | Remaining 2 committed L1 cases | All ≤5 s; max 0.827 s | Keep coverage and budget |
| R2-F4.105 | `biscuit-visualized tests::corpus_measure_tests::tree_view` | Remaining 2 committed L1 cases | All ≤5 s; max 0.152 s | Keep coverage and budget |
| R2-F4.106 | `biscuit-visualized tests::corpus_measure_tests::treemap` | Remaining 2 committed L1 cases | All ≤5 s; max 0.246 s | Keep coverage and budget |
| R2-F4.107 | `biscuit-visualized tests::corpus_measure_tests::usecase` | Remaining 2 committed L1 cases | All ≤5 s; max 0.328 s | Keep coverage and budget |
| R2-F4.108 | `biscuit-visualized tests::corpus_measure_tests::venn` | Remaining 2 committed L1 cases | All ≤5 s; max 0.107 s | Keep coverage and budget |
| R2-F4.109 | `biscuit-visualized tests::corpus_measure_tests::wardley` | Remaining 2 committed L1 cases | All ≤5 s; max 0.129 s | Keep coverage and budget |
| R2-F4.110 | `biscuit-visualized tests::corpus_measure_tests::xychart` | Remaining 2 committed L1 cases | All ≤5 s; max 0.080 s | Keep coverage and budget |
| R2-F4.111 | `biscuit-visualized tests::font_chain_tests` | Remaining 31 committed L1 cases | All ≤5 s; max 0.593 s | Keep coverage and budget |
| R2-F4.112 | `biscuit-visualized tests::gitgraph_layout_tests` | Remaining 76 committed L1 cases | All ≤5 s; max 0.153 s | Keep coverage and budget |
| R2-F4.113 | `biscuit-visualized tests::gitgraph_tests` | Remaining 20 committed L1 cases | All ≤5 s; max 4.904 s | Keep coverage and budget |
| R2-F4.114 | `biscuit-visualized tests::golden_tests` | Remaining 198 committed L1 cases | All ≤5 s; max 2.549 s | Keep coverage and budget |
| R2-F4.115 | `biscuit-visualized tests::graph_tests` | Remaining 28 committed L1 cases | All ≤5 s; max 0.899 s | Keep coverage and budget |
| R2-F4.116 | `biscuit-visualized tests::html_static_tests` | Remaining 5 committed L1 cases | All ≤5 s; max 0.584 s | Keep coverage and budget |
| R2-F4.117 | `biscuit-visualized tests::measure_tests` | Remaining 8 committed L1 cases | All ≤5 s; max 4.958 s | Keep coverage and budget |
| R2-F4.118 | `biscuit-visualized tests::mermaid_cache_key_tests` | Remaining 8 committed L1 cases | All ≤5 s; max 4.828 s | Keep coverage and budget |
| R2-F4.119 | `biscuit-visualized tests::mermaid_family_tests` | Remaining 28 committed L1 cases | All ≤5 s; max 4.997 s | Keep coverage and budget |
| R2-F4.120 | `biscuit-visualized tests::mermaid_tests` | Remaining 34 committed L1 cases | All ≤5 s; max 4.885 s | Keep coverage and budget |
| R2-F4.121 | `biscuit-visualized tests::newline_tests` | Remaining 14 committed L1 cases | All ≤5 s; max 0.035 s | Keep coverage and budget |
| R2-F4.122 | `biscuit-visualized tests::newline_tests::rendered` | Remaining 6 committed L1 cases | All ≤5 s; max 4.220 s | Keep coverage and budget |
| R2-F4.123 | `biscuit-visualized tests::palette_tests` | Remaining 14 committed L1 cases | All ≤5 s; max 0.039 s | Keep coverage and budget |
| R2-F4.124 | `biscuit-visualized tests::postprocess_tests` | Remaining 7 committed L1 cases | All ≤5 s; max 0.219 s | Keep coverage and budget |
| R2-F4.125 | `biscuit-visualized tests::raster_tests` | Remaining 5 committed L1 cases | All ≤5 s; max 0.664 s | Keep coverage and budget |
| R2-F4.126 | `biscuit-visualized tests::raster_tests::requests` | Remaining 3 committed L1 cases | All ≤5 s; max 0.900 s | Keep coverage and budget |
| R2-F4.127 | `biscuit-visualized tests::theme_tests` | Remaining 41 committed L1 cases | All ≤5 s; max 4.039 s | Keep coverage and budget |
| R2-F4.128 | `biscuit-visualized::l1 display_size` | Remaining 10 committed L1 cases | All ≤5 s; max 3.824 s | Keep coverage and budget |
| R2-F4.129 | `biscuit-visualized::l1 graph_expression_parity` | Remaining 13 committed L1 cases | All ≤5 s; max 0.021 s | Keep coverage and budget |
| R2-F4.130 | `biscuit-visualized::l1 graph_expression_parity::parity_helpers::tests` | Remaining 24 committed L1 cases | All ≤5 s; max 0.028 s | Keep coverage and budget |
| R2-F4.131 | `biscuit-visualized::l1 layout_matrix` | Remaining 6 committed L1 cases | All ≤5 s; max 4.347 s | Keep coverage and budget |
| R2-F4.132 | `biscuit-visualized::l1 mermaid_parity` | Remaining 13 committed L1 cases | All ≤5 s; max 3.992 s | Keep coverage and budget |
| R2-F4.133 | `biscuit-visualized::l1 mermaid_parity::parity_helpers::tests` | Remaining 24 committed L1 cases | All ≤5 s; max 0.023 s | Keep coverage and budget |
| R2-F4.134 | `biscuit-visualized::l1 render_tree_diagram_hook` | Remaining 5 committed L1 cases | All ≤5 s; max 4.197 s | Keep coverage and budget |

## Blocked Findings

None. The existing gallery review and styling choices remain human review items, not newly invented defects. The current contrast and speed requirements give the repair cycle a concrete target. The previous review's proposed deferrals were options awaiting a decision, not accepted exclusions.

## Recurrence

`review-1.md` disclosed the sequence-box readability problem, the graph font-cache omission, and the two performance overruns in its author-decision entries and “Deferred work” table. It issued no numbered findings, so this review assigns `R2-F1`–`R2-F3` rather than inventing earlier finding IDs.

- **R2-F1:** the sequence-box instance is unchanged. The author-color failures in the family table are newly found siblings of the same unreadable-library-text class. The earlier pass explicitly did not sweep the code; the empty repair ledger did not claim to fix them. A complete repair must inspect both the no-design branch and stylesheet-painted sequence regions, then verify all listed families and output paths.
- **R2-F2:** unchanged instance, now reproduced and narrowed to graph PNGs. Both expression and DOT constructors feed the defective key; both must be covered.
- **R2-F3:** unchanged over-limit measurements, not a slowdown introduced by the repair cycle. All baseline families have now been checked against the same rule.

`recurrence: true` preserves those previously disclosed classes in the repair inventory. It does not imply an attempted code repair regressed. R2-F4 is newly verified test-speed debt; the earlier review did not issue a finding for that class. No finding requests a stronger guarantee than the current spec or a response to adversarial input.

## Verification levels and input robustness

| User-facing requirement | Strongest relevant coverage inspected | Assessment |
| --- | --- | --- |
| Family support, valid SVG/PNG, no HTML labels in raster input | L1 family smoke tests and pinned-font SVG/PNG goldens | Appropriate for generated artifacts |
| Font selection, matching measurement/drawing, fallback glyphs, newlines | L1 injected-font and usvg measurement-oracle tests; wrapping corpus | Appropriate for the library contract |
| Tag topology, tag bounds, collision spacing, author spacing precedence | L1 public geometry/render tests, drawn-tag comparisons, consumer graph tests | Appropriate for geometry; no keyboard interaction is added |
| Palette/style precedence and readable text | L1 output-paint assertions plus independent raster contrast oracle | Appropriate level, but the exemptions and omitted author-color cases leave R2-F1 |
| Font-dependent cache invalidation | L1 font-key tests for Mermaid; public separate-process graph reproduction | Graph coverage misses R2-F2 |
| Timeout fallback and warnings | L1 component/render and layout-deadline tests | Appropriate; no terminal input encoder behavior is promised |
| OSC 4 parsing, partial/silent replies, caching, query gates | L1 fake-TTY tests; Windows console-query test (placed in the L2 target) checks forwarded queries/replies through ConPTY with manufactured answers | These prove parser/gate and console transport behavior, not an emulator’s actual color answers. The spec explicitly asks for fake-TTY/parser checks here; cross-OS proof is left to CI |
| Diagram display and browser embedding | Consumer worktree Kitty L2 tests and Darkmatter browser-tier static-SVG/sanitizer tests | Correct resource tiers exist; this review does not claim the previously failing Kitty case passed |
| Warm/cold speed | Local release benchmark on the designated host | Correct measurement boundary; supplied result fails R2-F3 |
| Gallery appearance | Human comparison | Still pending; does not independently set `ready: false` |

The formal file/configuration matrix applies to the benchmark baseline loader. Text grammars, SVG writers, renderer fields, and terminal reply bytes are not configuration files. The gitGraph layout JSON is an internal Merman-produced document, not user configuration; its strict reader nevertheless has malformed-shape tests.

For the baseline, the default is a located error before rendering/comparison or writing the baseline. `reference_host` must be a nonempty string; objects and positive finite timing numbers are required at their respective paths. The exception is an explicit empty `families: {}`, which is valid and produces “no baseline” for missing families rather than a passing comparison. The shared positive-duration validator handles all four timing fields and rejects zero/negative values; percentile order is checked separately. The strict JSON visitor rejects duplicate keys at every depth, and the loader calls the deserializer's end check to reject trailing content. Existing loader/wiring tests cover a control, missing/null/wrong-type durations, empty array/object values, invalid numbers, duplicate keys, trailing text, absent families, and unknown keys/families. The standalone benchmark tests were deliberately placed outside CI by the specification; no new CI cell is requested.

The requested smell search found no `#[serde(default)]` in the baseline or layout readers. The layout reader's parent projection filters references only after validating them. Terminal parser `.ok()` values deliberately mean an unknown reply, as required. Font metadata parsing deliberately excludes unusable faces. No additional realistic configuration defect was established.

## Commands and limits of this review

- **Restored-source L1:** `just test --test-threads 4` in `biscuit-visualized/` completed in 295.893 s: **933 passed, 3 skipped, 65 over five seconds**. Assertions pass; the test-speed contract does not (R2-F4). The slowest case took 18.423 s. The original default-concurrency attempt stopped after one test process received SIGKILL; an intermediate run was invalidated by rebuilding temporary diagnostic binaries while it was still spawning tests. The final run above used restored sources and completed without those interruptions.
- **Lint:** `just lint` passed on restored sources. A temporary diagnostic's clippy warning was removed with the probe and is not a production finding.
- **Tier selection:** `just check-tier-coverage biscuit-visualized` passed with zero stranded tests after removing the temporary example. The library test modules are declared, the terminal integration binary is declared, and CI enables all features.
- **Public-result probes:** expression/DOT × SVG/PNG × font-change cache controls; simplified author-color controls in four core families; the existing sequence fixture; all 33 family fixtures with and without the single author-color edit. The 33 diagnostic cases completed and reported their oracle observations. No diagnostic code remains in the repository.
- **No fresh reference-host benchmark or full dependent-suite rerun:** R2-F3 uses the saved reference-host measurements; the accepted separate Darkmatter key-order work and prior Kitty failure are not falsely marked repaired. Cross-OS receipts are not a readiness finding.

Temporary probe code and diagnostic tests were removed; the production source diff is empty. No formatter, commit, GUI focus change, or upstream publication was performed. The report, previous-review link, and specification review counter are the deliverables. The specification remains incomplete while findings are open.
