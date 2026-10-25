# Spike S1: comparison of today's renderer and Merman

Spike for `2026-10-21-merman`, Phase 1 (R-S1–R-S7, R-S9; D1, D2, D12, D26, D29). Run on 2026-10-21. This is throwaway code and is not a workspace member. It records facts for Ken's D1 decision and makes no recommendation.

- **Gallery:** `gallery/index.html` (macOS, dark), `gallery/index-light.html` (macOS, light), and `gallery/index-linux.html` (Linux, dark).
- **Raw data:** `gallery/scorecard.json`.

## Setup

| Item | Value |
|---|---|
| Current column | `biscuit-visualized` by path (feature `image`), through `mermaid::MermaidDiagram::render` (format SVG). This is mermaid-rs-renderer 0.3.1 plus our merge repair, tag spacing, SVG fixes, and themes (D26). |
| Merman | git `rev = 482fcde7faffb05a466f198085d615e604075d97` (same as S0), `default-features = false`, `["svg", "all-diagrams", "layout-cytoscape"]`, `SvgPipeline::resvg_safe()`. ELK is off except in one build-weight variant. |
| Theme pairing | Dark: `MermaidTheme::Dark` against Merman site config `{"theme": "dark"}`. Light: `MermaidTheme::Default` against `{"theme": "default"}`. These are Merman's out-of-the-box Mermaid themes, with no compiled theme or palette work. |
| Rasterizer | The same code for both columns: a copy of `biscuit-visualized`'s resvg 0.45.1 path, with system fonts and 2× the natural width capped at 1000 px. One deliberate addition, the rasterizer-side fallback planned as D24: after loading system fonts, `sans-serif`, `serif`, and `monospace` each point at the first installed family from a fixed candidate list. On macOS that resolves to Arial, Times New Roman, and Courier New (fontdb's own defaults, so no change there). On Linux it resolves to DejaVu Sans, DejaVu Serif, and DejaVu Sans Mono. Without it, fontdb's fixed defaults are missing on the Linux host and resvg silently dropped every Merman label. `Cargo.lock` was resolved fresh, with resvg/usvg 0.45.1, tiny-skia 0.11.4, fontdb 0.23.0, rustybuzz 0.20.1, and mermaid-rs-renderer 0.3.1, all identical to the workspace lock. |
| macOS host | macOS 27.2, Apple M4 Max, 16 threads |
| Linux host | `$BUILD_LINUX`: Debian 13, AMD EPYC 9175F, 16 threads. Its only fonts are DejaVu. |
| References | Merman fixtures carry stored Mermaid.js **12.2.0** SVGs. Our test diagrams and the gitGraph cases were rendered with the local `mmdc`, which is Mermaid **11.12.0**, default theme. |

The build output never entered the repository (`CARGO_TARGET_DIR` was in the session scratchpad). The PNGs, SVGs, and `results-*.json` also live there. Only the gallery embeds the PNGs.

## Corpus (`corpus/`, 2 MB, licenses in `corpus/LICENSES/`, no fonts)

- **43 Merman fixtures** (`scripts/build_corpus.py`, hand-picked): 1–3 per shared family (23 families) and 1 per Merman-only family (14). All but one have a Mermaid.js reference; the ZenUML fixture has none.
- **4 gitGraph cases** (`corpus/cases/`) covering R-S7: a labeled merge, two labeled merges with tags, adjacent colliding tags, and two tags on one commit.
- **56 unique Mermaid sources from the repository's Rust tests** (`scripts/extract_test_diagrams.py`, 21 files). They break down as gitGraph 37, flowchart 10, sequence 2, xychart 2, and one each of ER, pie, quadrant, state, and timeline.
    - Sources assembled with `format!` and interpolated values cannot be extracted statically. They are left out, which leaves AC-S2 short by that amount.
    - `manifest.json` records every place each source occurs.
- **"Shared" vs "Merman-only"** follows the research doc's list of headers that mermaid-rs-renderer detects. Render results confirmed the list: every shared family rendered in both renderers.

## Headline results

| | Current | Merman |
|---|---|---|
| Shared families rendered (23) | 23, with gitGraph partial at 36/37 valid inputs | 23 |
| Merman-only families (14) | 0. It errors on `info` and `eventmodeling`, and draws a wrong flowchart for the other 12 (mermaid-rs-renderer's flowchart fallback) | 14 |
| Test inputs Mermaid.js rejects (9) | 6 rendered anyway, 3 rejected | 9 rejected, the same set as Mermaid.js |
| `foreignObject` in output | none | none |

- **The one current failure.** The input is `biscuit-visualized/src/src/tests/gitgraph_tests.rs:249`, a merge of a branch with no commit of its own. Our repair rejects it on purpose: `docs/mermaid-gitgraph.md` says "as mermaid.js refuses". Mermaid.js 11.12.0 and Merman both render it, so that claim may not hold for current Mermaid.
- **Test inputs Mermaid.js rejects.** All of them come from our own tests: `direction LR` inside gitGraph, branch names with spaces, a merge from an empty `main`, and the deliberately malformed `A to B` and `--&gt;` flowcharts. Today's renderer accepts most of them.

## Per-family observations

The gallery's "Observed" column has a note for every family. The patterns:

- **Fidelity.** Merman matches the Mermaid.js reference in layout, shapes, and syntax coverage in nearly every family.
    - Today's renderer misreads or simplifies many inputs:
        - journey: actors and title are misparsed.
        - radar: one axis is drawn and node syntax prints as raw text.
        - packet: the whole diagram collapses into one strip.
        - block: blocks are dropped.
        - treemap: siblings are nested inside each other.
        - sankey: nodes land in the wrong columns.
        - kanban and mindmap: metadata prints as raw text.
        - sequence: participant types print as text.
        - gantt: compact mode is ignored and the axis is misread.
        - timeline: all events go on one row.
        - gitGraph: the bottom-to-top orientation is ignored.
    - The exceptions are flowchart `outgoing_links_2b`, where current is closer to the reference, and class, where current's first fixture is clean.
- **Merman label clipping.** It comes from measurement, not from the SVG. It affects class, state, ER, mindmap, kanban, requirement, block, swimlane, and usecase: 218 label renders on macOS and 268 on Linux. Each label sits in a nested `<svg style="overflow:hidden">` sized by Merman's default measurer, which is font-free, while the drawn faces are wider; Chrome clips the same SVGs. Measuring with the real fonts removes it (218 → 2, 268 → 2; see "Merman (real fonts)").
- **Missing class and ER dividers (Merman's "neo" look, dark theme).** Merman does emit the dividers (`<g class="divider"><path …>`). Its stylesheet then sets `[data-look="neo"].node path{stroke:url(#…-gradient)}`, a gradient with `gradientUnits="objectBoundingBox"`. A perfectly horizontal divider has a zero-height bounding box, and per the SVG specification an object-bounding-box paint server does not paint such an element, so the line disappears. The same gradient makes node outlines fade from light on the left to dark on the right. The survey found 36 such lines, all in class and ER and only in the dark theme (Mermaid's default theme uses no gradient). Fix (b) restores all of them. Our compiled theme sets node stroke paint inline (S0), which would also avoid this, but the guard tests should cover it.
- **Serif text in Merman.** Railroad (all four), treeView, and cynefin request `"courier"`, which failed fontdb's case-sensitive match and fell back to serif. With the real-fonts chain it resolves to Courier on macOS and DejaVu Sans Mono on Linux. Only wardley sets no font; it gets the sans-serif answer.
- **Merman's out-of-the-box dark theme has contrast problems.** This is Mermaid's own `dark` theme on a transparent background:
    - The sequence `box` is painted light grey, which hides its messages.
    - Pie and radar use near-black and maroon fills with grey labels.
    - Packet bit numbers, treeView names, and gantt labels beside done tasks are dark on dark.
    - ZenUML ignores the dark theme.

    These are the cases the D6 styling layer and D18 contrast pass target. Today's dark theme has its own unreadable cases: timeline cards, quadrant axis labels, and C4 edge labels.
- **gitGraph (R-S7, AC-S5).**
    - **Labeled merges.** Merman draws `merge feat id: "M" tag: "v1"` with both parents, the tag, and the merge commit shape, without our repair. Its layout data agrees: `M ← A, B`; `MH ← m1, h1`; `MF ← MH, f2`; `Release 1.0 ← ONE, B`.
    - **Tag collisions.** Adjacent tags (`main`, `origin/main`, `release/2026-10` on consecutive commits) overlap in Merman exactly as they do in the Mermaid.js reference, and today's tag spacing separates them. Merman therefore matches Mermaid.js here, but it does **not** make our tag-spacing correction unnecessary if non-overlap is still required.
    - **Stacked tags.** Two tags on one commit stack without overlap in all three renderers.
- **Pie.** Merman matches the reference's slice order, colors, start angle, and outer stroke. Current swaps two slice colors and starts at 3 o'clock. Both have dark-theme contrast issues, described in the gallery.

## Merman (real fonts)

The gallery's fourth column renders Merman with text measured in the real fonts the rasterizer draws with. An earlier version of this column patched label viewports to `overflow:visible`; Ken rejected that as a hack that hides the measurement mismatch, and it is gone.

**How measurement is wired.** Merman measures through our `TextMeasurer` (`src/measure.rs`), installed through Merman's public API: `TextMeasurementProfile::new` → `TextMeasurementPolicy::uniform` → `SvgEnvironment::with_text_measurement_policy` on the `merman::svg` facade (some trait hook methods are `doc(hidden)`). It shapes each label with usvg 0.45 on the rasterizer's own font database. Merman's wrapping helper is not exported and the trait's default ignores `max_width`, so the spike wraps with a plain greedy word wrap.

**One font-resolution chain** (`FontChain::resolve` in `src/fonts.rs`) serves both measuring and drawing, so the measured face is the drawn face by construction:

1. `MERMAID_FONT`, if set and installed (matched ignoring case). Unset or empty: skipped silently. Set but not installed: one warning naming it, then continue.
2. The label's font stack (Mermaid's default is `"trebuchet ms", verdana, arial, sans-serif`), matched ignoring case as CSS does.
3. The OS's own generic family via fontique (fontconfig on Linux, CoreText on macOS, DirectWrite on Windows).
4. An installed face whose OS/2 table classifies it as sans-serif (not reached on either host).

| Step | macOS | Linux |
|---|---|---|
| 1. `MERMAID_FONT` | unset: skipped; `helvetica` → Helvetica | unset: skipped; `DejaVu Serif` → applied |
| 2. Stack | trebuchet ms → Trebuchet MS, verdana → Verdana, arial → Arial, courier → Courier, Helvetica, Open Sans | none installed |
| 3. fontique generic | sans-serif → Helvetica, serif → Times, monospace → Courier | DejaVu Sans / DejaVu Serif / DejaVu Sans Mono; courier → DejaVu Sans Mono |
| 4. OS/2 class | not reached | not reached |

fontique's faces map to usvg's fontdb by file path plus face index; every one did on both hosts. Glyphs missing from a face fall back through usvg's `select_fallback` in both measuring and drawing, so the two still agree. fontique adds 11 crates (5.3 s uncached release build on its own), scans fonts once per process (about 200 ms on macOS, 10 ms on Linux), and passes a `cargo check --target x86_64-pc-windows-gnu`.

**Corrections kept.** The gradient-stroke fix (`fix_merman_svg`, a genuine SVG-spec issue) and the sans-serif answer as the default for text with no font set. No overflow patch.

**Results (both themes, valid inputs; default measurement → real fonts).**

| Family | Clipped, macOS | Clipped, Linux | Past its shape, Linux | Outside canvas, Linux |
|---|---|---|---|---|
| class | 60 → 0 | 62 → 0 | 0 | 4 → 0 |
| state | 10 → 0 | 18 → 0 | 12 → 0 | 0 |
| er | 36 → 0 | 58 → 0 | 8 → 0 | 2 → 0 |
| mindmap | 28 → 0 | 30 → 0 | 2 → 0 | 0 |
| kanban | 40 → 0 | 54 → 0 | 0 | 0 |
| block | 8 → 0 | 6 → 0 | 0 | 0 |
| requirement | 16 → 0 | 18 → 0 | 0 | 2 → 0 |
| usecase | 10 → 0 | 12 → 0 | 2 → 0 | 0 |
| swimlane | 10 → 2 | 10 → 2 | 4 → 2 | 0 |
| gitGraph | 0 | 0 | 28 → 2 | 0 |
| flowchart, sequence, gantt | 0 | 0 | 16 → 0 | 0 |
| radar | 0 | 0 | 6 → 6 | 2 → 2 |
| **All** | **218 → 2** | **268 → 2** | **78 → 10** (macOS 32 → 8) | **16 → 2** (macOS 6 → 2) |

Invisible gradient lines go 36 → 0 on both hosts.

**What remains.**

- Swimlane "Needs code change": Merman's layout wraps it to two lines, but the resvg-safe output draws one line in the two-line box, so it is cut whatever font measures it (a Merman bug).
- Radar legend and one gitGraph tag: most likely detector false positives (the label is compared against an unrelated shape behind it).
- Usecase: outlined stereotype text and a dark note; styling, not measurement.

**Cross-check.** Merman's own native measurement (`native-fonts` feature, `NativeFontOptions` + `with_native_font_context`, given the chain's resolved font files) gives identical defect counts on both hosts; 112 of 188 SVGs differ in small layout details. Explicit files were used instead of raising `NativeFontLimits.max_system_cmap_bytes`, so the face set is exact.

## Defect survey

`src/survey.rs` measures defects from usvg's parse with the rasterizer's fonts. Per family it records:

- labels clipped by an ancestor viewport;
- zero-area lines painted by an objectBoundingBox gradient;
- text extending more than 1 px past its backing shape;
- text outside the canvas;
- which face each label's requested font resolved to.

The gallery's "Defect survey" section and `scorecard.json` (`defect_survey`, `merman_cli_survey`, `case_insensitive_fonts`) hold the per-family tables for both hosts.

**Merman's own export.** merman-cli at the same rev reproduces the defects with Merman's default measurement: 208 clipped labels and 36 invisible lines, in class, state, ER, kanban, block, requirement, mindmap, swimlane and usecase. With `--font-measurement native` and explicit font files, clipped labels drop to 4 and text past its box from 34 to 8. Native measurement with system font discovery fails on macOS because of a font-size limit.

**Browser check.** The same resvg-safe SVGs clip identically in headless Chrome ("Animal": 51.3 px of text in a 44.8 px box, with Chrome using real Trebuchet MS), and the dark-theme dividers are missing there too. Both defects are in Merman's resvg-safe output, not in resvg, and are candidates for an upstream report.

**Why Merman's playground looks clean.** It mounts the SVG inline using the `parity` pipeline (HTML labels in `foreignObject`, so no fixed-width clip box) with browser text measurement and Mermaid's default light theme (`playground/src/lib/workspace-snapshot.ts`).

**Case-insensitive font matching.** fontdb matches family names case-sensitively, so Mermaid's lowercase `"trebuchet ms"` and `"courier"` never match by default. Matching alone does not fix clipping (218 → 214 on macOS with Merman's default measurement), because Merman's default measurer models no particular font. The real-fonts chain above matches ignoring case and measures with the matched face, which does fix it.

**Mermaid's default font.** Mermaid.js (11.12.2) and Merman both default `fontFamily` to the CSS stack `"trebuchet ms", verdana, arial, sans-serif`. Trebuchet MS is only the first preference, not a requirement.

### Upstream issue draft (not filed)

> **1. Default text measurement assumes fonts the host may not have (resvg-safe labels clip)**
>
> merman 0.9.1 @ `482fcde7faffb05a466f198085d615e604075d97`. `merman-cli render fixtures/class/upstream_examples_class_basic_class_inheritance_001.mmd --format png -c dark.json` (dark.json is `{"theme":"dark"}`); same for `fixtures/er/upstream_examples_er_basic_er_schema_001.mmd`.
>
> - **Expected:** full labels. **Actual:** "Anima", "ıame", "rderDate"; 208 of 1,416 label renders clipped across the fixtures. Chrome clips the same resvg-safe SVG ("Animal": 51.3 px of text in a 44.8 px box).
> - **Cause:** the resvg-safe pipeline sizes each label's `<svg style="overflow:hidden">` from the built-in deterministic measurer, which models no particular font. Any real face drawing Mermaid's default stack is wider.
> - **Evidence:** measuring with the faces actually drawn (`--font-measurement native` with `--font-file`, or a host `TextMeasurer`) cuts clipping to 2 per host.
> - **Suggestions:** document that raster output needs real-font measurement; make native measurement the default for PNG export; or let the label viewport not clip. Also, native measurement with system font discovery fails on macOS ("system-cmap-bytes exceeded").
>
> **2. Wrapped labels lose their line breaks in resvg-safe output**
>
> Swimlane `upstream_docs_basic`, edge label "Needs code change": the layout box is sized for two lines (120×48), but the fallback text is one line, so it is cut even with real-font measurement.
>
> **3. Neo-look dark theme draws no class or ER dividers**
>
> `[data-look="neo"].node path{stroke:url(#<id>-gradient)}` uses an `objectBoundingBox` gradient. On a zero-height divider it paints nothing per the SVG spec, in resvg and in Chrome alike (36 lines across the fixtures, dark theme only). Suggest the solid node stroke, or `userSpaceOnUse`, for divider paths.

## Timing (R-S5, D12)

- **Method.** Warm, in-process, release build, dark theme, 25 iterations per input, rasterization excluded. Each family figure is the median of per-input medians in ms.
- **Cache I/O in current's figures.** Current's time includes `MermaidDiagram::render`'s cache-miss file work: the existence check, `create_dir_all`, the write, and the spike's read-back. Measured on its own, that work is about 0.24 ms on macOS, so on macOS most of current's figure is file I/O.
- **Columns.** "Net" is Merman divided by (current − cache I/O). The Linux columns are medians of per-input medians as well.

| Family | Current macOS | of which I/O | Merman macOS | Ratio | Ratio net of I/O | Current Linux | Merman Linux |
|---|---:|---:|---:|---:|---:|---:|---:|
| flowchart | 0.32 | 0.24 | 1.16 | 3.6× | 14× | 0.13 | 1.64 |
| sequence | 0.55 | 0.24 | 1.02 | 1.9× | 3.3× | 0.37 | 1.37 |
| class | 0.35 | 0.23 | 3.63 | 10× | 29× | 0.13 | 4.20 |
| state | 1.11 | 0.25 | 2.68 | 2.4× | 3.1× | 1.28 | 3.46 |
| er | 1.58 | 0.27 | 3.52 | 2.2× | 2.7× | 1.84 | 3.95 |
| gitgraph | 0.30 | 0.23 | 0.49 | 1.7× | 6.9× | 0.07 | 0.62 |
| pie | 0.30 | 0.25 | 0.32 | 1.1× | 6.5× | 0.07 | 0.43 |
| xychart | 0.29 | 0.23 | 0.42 | 1.5× | 6.9× | 0.05 | 0.52 |
| quadrantchart | 0.28 | 0.23 | 0.28 | 1.0× | 6.5× | 0.06 | 0.30 |
| gantt | 0.45 | 0.27 | 0.73 | 1.6× | 4.0× | 0.17 | 0.88 |
| timeline | 0.34 | 0.23 | 0.87 | 2.6× | 7.9× | 0.09 | 0.99 |
| journey | 0.39 | 0.28 | 1.32 | 3.4× | 12× | 0.11 | 1.56 |
| mindmap | 0.46 | 0.28 | 5.78 | 13× | 32× | 0.17 | 5.57 |
| kanban | 0.42 | 0.27 | 5.04 | 12× | 35× | 0.15 | 5.07 |
| sankey | 0.33 | 0.23 | 0.43 | 1.3× | 4.3× | 0.08 | 0.56 |
| c4 | 0.41 | 0.29 | 1.07 | 2.6× | 9.4× | 0.13 | 1.25 |
| block | 0.32 | 0.27 | 1.31 | 4.1× | 25× | 0.06 | 1.64 |
| architecture | 0.37 | 0.28 | 0.75 | 2.0× | 8.9× | 0.10 | 0.93 |
| requirement | 0.35 | 0.27 | 1.91 | 5.4× | 22× | 0.08 | 2.25 |
| packet | 2.72 | 0.32 | 0.40 | 0.15× | 0.2× | 2.73 | 0.35 |
| radar | 0.33 | 0.27 | 0.30 | 0.9× | 5.1× | 0.06 | 0.26 |
| treemap | 0.42 | 0.30 | 0.68 | 1.6× | 5.9× | 0.11 | 0.73 |
| zenuml | 0.42 | 0.28 | 0.28 | 0.7× | 2.0× | 0.17 | 0.25 |

- **Against D12's starting definition of "clearly worse"** (median more than 3× slower, or over 100 ms):
    - No Merman render exceeded 7 ms. The slowest was the ER fixture at 6.1 ms on macOS, and all Merman-only families stayed under 4 ms.
    - By ratio, Merman is more than 3× slower than current's net render time in most families. This is because mermaid-rs-renderer often finishes in under 0.2 ms.
    - Including cache I/O, which a real cache-miss render also pays, Merman's ratio exceeds 3× in flowchart, class, journey, mindmap, kanban, block, and requirement on macOS.
    - Ken may want to revisit the threshold (D12 allows this).
- **Rasterization** costs more than either render. Each PNG is rasterized once, so these are single-shot medians and vary between runs. On macOS: 9.4 ms for Merman's PNGs and 6.9 ms for current's, against 6.1 and 4.8 ms in an earlier run with the same output. On Linux: 5.3 and 3.6 ms.
- **Real-font measurement cost** (macOS, median per family, ms; context and measurer setup outside the timings):

  | Family | Default | Real fonts, cold | Real fonts, warm | Merman native |
  |---|---:|---:|---:|---:|
  | flowchart | 1.05 | 1.19 | — | — |
  | class | 3.39 | 23.2 | 3.45 | 5.44 |
  | kanban | 4.33 | 105 | 4.38 | 9.54 |
  | c4 | 0.93 | 64 | — | 5.49 |
  | sequence | 0.97 | 27 | — | 3.08 |

  - Our measurer is expensive cold because it parses a one-text SVG per label; with shaping results cached it returns to roughly default cost. A long-running process with a shared measurer pays almost nothing; a one-shot CLI render pays the cold cost unless the measurer shapes directly.
  - Merman's native measurement is about 1.5–2× default, plus 3–5 ms to load its font context. Linux costs are similar.
- **Flowchart outlier.** Current's flowchart p95 of 55.9 ms on macOS is a single input's outlier and is likely file-system jitter, since its median is 0.32 ms.

## Build weight (R-S6, D29; information only)

Uncached release builds of the spike binary on the macOS host, one renderer compiled in, each in a fresh target dir with no `kache`.

| Variant | Build | Binary (stripped) | Packages |
|---|---:|---:|---:|
| current only (biscuit-visualized + mmdr) | 18 s | 8.1 MB (6.7 MB) | 94 |
| Merman only | 53 s | 27.3 MB (22.6 MB) | 174 |
| Merman + ELK (measured once, D15) | 59 s | 29.7 MB (24.6 MB) | 176 |
| both | 90 s | 31.8 MB (26.3 MB) | 193 |

The "both" build ran alongside other local work, so its time is the least reliable figure.

## Cross-OS PNG difference (R-S9, AC-S8 partial)

- **Method.** `scripts/cross_os_diff.py` compared every macOS/Linux PNG pair with ImageMagick `compare`. The metrics are AE share (fuzz 0) and normalized RMSE, for same-size pairs only.
- **Current: 194 pairs, 163 differ in pixel size.** mermaid-rs-renderer measures text with the host's fonts, so layout moves between OSes. Of the same-size pairs, the median is 0.3% of pixels differing, with a maximum of 2.6%.
- **Merman: 188 pairs, 2 differ in pixel size.** Layout uses Merman's own text metrics. The rest differ only in glyph rasterization: a median of 0.5% of pixels and a maximum of 3.2%, with a median RMSE of 0.051. An earlier Linux pass drew no Merman text because of the missing font mapping. Its cross-OS numbers were invalid and were replaced.
- **Focus diagram** (`upstream_cypress_sequencediagram_spec_example_001`):
    - Merman dark: 0.27% of pixels differ, RMSE 0.037.
    - Merman light: 0.73% of pixels differ, RMSE 0.066.
    - Current changes size: 1000×1270 on macOS against 1000×1366 on Linux.
- **Windows** was not run. `$BUILD_WIN` is declared but was out of scope for this pass, so AC-S8's Windows leg is still open.

## Not covered by this spike

- **R-S8 / AC-S6.** `GitGraphGeometry` from Merman's layout data and the `worktree` tests were not done. The layout data used above gives commits, parents, and tags, but not tag bounds.
- **AC-S4.** Ken's decision.

## Points that bear on the spec's decisions

- **D5 / R-C4.**
    - The merge repair looks unnecessary with Merman, which keeps both parents.
    - Tag spacing is not: Merman reproduces Mermaid.js's overlapping adjacent tags.
    - Our repair's "empty branch" error rejects an input that Mermaid.js 11.12.0 and Merman accept.
- **D13.**
    - With real fonts, layout follows the fonts installed on the host (not the OS). Between the two hosts the SVG width changes by a median 0.5% (90th percentile 5.9%, largest 18.7%), and 122 of 188 PNGs change size; same-size pairs differ in a median 0.3% of pixels. Today's renderer varies about as much. This is expected, not a defect.
    - So SVG goldens need a pinned font set (`MERMAID_FONT` plus a fixed font directory) or per-host baselines. Merman's default measurement alone is host-independent, but it clips.
    - The figures below were measured with Merman's default measurement:
    - Merman SVG geometry is essentially OS-independent, which suits exact SVG goldens.
    - PNG goldens need a glyph-level tolerance. About 3.2% of pixels at most differ by AE in this corpus, and an RMSE-based tolerance is the gentler metric.
- **D14 / D24.**
    - Cutover needs real-font measurement (our `TextMeasurer`, or Merman's native fonts), the shared font-resolution chain used by both measurer and rasterizer, the gradient-stroke fix, and a sans-serif default for unset fonts.
    - Wrapping should come from Merman (export its wrapping helper, or use native measurement), not the spike's greedy wrap.
    - Without the chain's generic-family step, Linux hosts without Arial lose all Merman text.
- **Darkmatter HTML (D14).** The playground shows the `parity` pipeline (HTML labels) draws cleanly in a browser. Darkmatter's HTML output could use it instead of resvg-safe, but only if Darkmatter's SVG sanitizer admits `foreignObject` HTML, which it strips today. Open question for the spec.
- **D6 / D17 / D18.** Mermaid's stock dark theme on a transparent background has several unreadable cases. The planned styling and contrast passes are where those get fixed.
- **D12.** Merman's absolute times are a few milliseconds. The ratio rule flags most families because the baseline is sub-millisecond.
- **Test corpus hygiene (D22).** Nine of our gitGraph and flowchart test inputs are not valid Mermaid.js, and Merman rejects them. `biscuit-visualized`'s own gitGraph tests would need new inputs under Merman. Those inputs come from the merge-repair tests, not from the `GitGraph` component or `worktree`'s generator.

## Reproduce

- **Full pipeline:** `scripts/run.sh <scratch-dir> [linux-results.json]` runs the corpus, references, renders, and galleries.
- **Build weight:** `scripts/build_weight.sh <scratch-dir>`.
- **Linux run:** ship the spike, `biscuit-visualized/src`, and `biscuit-hash/lib` to the host, then run the binary there with `--os linux`. Fetch `out/` and rewrite the PNG paths in its `results-linux.json`.
