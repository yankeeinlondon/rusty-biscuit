---
prompt: |-
    [Merman](https://github.com/Latias94/merman) is a popular open-source library that renders MermaidJS visualizations as raster images using Rust. This library is now used by the [Zed](https://zed.dev) editor (although they have added their own styles to add a bit of polish on top).

    Your task is to do deep into the **Merman** library. Your research can be conducted online but you can also refer to local clone of the merman repo (/Users/ken/coding/forks/merman) for evaluation as well.

    Your research should be able to answer (at a minimum):

    - what is Merman's release history? What is the latest version? What is the release cadence?
    - what philosophy and testing does Merman use toward testing their compatibility with the MermaidJS project?
    - what version of MermaidJS does Merman report to have compatibility with (if any)?
    - are there any chart types (including beta charts) that are not supported?
    - how does Merman compare with [mermaid-rs-renderer]()?
    - choose 4-5 chart types and give examples of how you would render them in Merman
    - how do you configure base styles and/or themes?
        - is the Zed editor's CSS styling publically available? Is it offered under a certain license?
        - what other abstractions does Zed put on top of Merman to do it's rendering?
        - is the tabbed layout which can toggle between the configuration and the rendered illustration part of Merman or Zed's extension?
    - does Merman support the inclusion of popular icon packs?
    - what performance metrics are there?
        - comparisons with MermaidJS?
        - comparisons with `mermaid-rs-renderer`?
    - what output formats does mermaid-rs-renderer render to? PNG? JPG? SVG?
hash: 16c83e6dcd58fe48-1f742d39dbaf797a
last_updated: 2026-10-20
---

# Merman: Research Notes

Research date: 2026-10-20. Sources: the local clone (`/Users/ken/coding/forks/merman`, at `482fcde7f`, just past `v0.9.1`), the GitHub and crates.io APIs, and the Zed repository. Claims not backed by one of those are flagged.

## Summary

- Merman is a **headless Rust port of Mermaid** (parser, layout, SVG renderer). It aims at *Mermaid-structural* SVG, not a restyled look.
- **SVG is the primary output.** PNG, JPEG, PDF, and Unicode/ASCII text are derived targets behind Cargo features. PNG/JPEG/PDF start from *validated SVG*, not a browser screenshot, so "raster images" is accurate only for the opt-in export path.
- It is dual-licensed MIT OR Apache-2.0, and independent of the Mermaid project (no affiliation).
- Compared with `mermaid-rs-renderer` (mmdr), Merman covers more diagram families (37 vs 23) and more output formats, and aims for Mermaid parity. mmdr is a separate, faster-on-tiny-inputs design that accepts divergence.
- Zed does use Merman (merged 2026-05-27). The polish layer lives in Zed's GPL-3.0-or-later `mermaid_render` crate, not in Merman.

## Release history and cadence

Facts: first commit 2026-01-11; first tag `v0.1.0` 2026-02-23; crates.io shows **25 versions** and a latest of **0.9.1 (2026-10-18)**; about 151k total downloads, 6,189 commits on the checked-out history.

| Date               | Release                                  | Note                                                                  |
| ------------------ | ---------------------------------------- | --------------------------------------------------------------------- |
| 2026-02-23         | 0.1.0                                    | First crates.io publish                                               |
| 2026-02-26 → 03-13 | 0.2.0, 0.3.0, 0.4.0                      | Rapid early iteration (one minor every few days)                      |
| 2026-05-19 / 05-28 | 0.5.0 / 0.6.0                            | After a two-month gap; 0.6 adds the raster-safe SVG pipeline Zed uses |
| 2026-06-02/03      | 0.4.1, 0.5.1, 0.6.1, 0.6.2, 0.4.2, 0.5.2 | Backports to older minors                                             |
| 2026-06-05 → 06-09 | 0.7.0-alpha.1/.2, 0.7.0                  |                                                                       |
| 2026-06-10 → 09-30 | 0.8.0-alpha.1 … alpha.7                  | Four-month alpha train                                                |
| 2026-10-06         | 0.8.0                                    |                                                                       |
| 2026-10-12         | 0.9.0                                    |                                                                       |
| 2026-10-18         | 0.9.1                                    | Fixes on the 0.9.0 line                                               |

Cadence: a minor roughly every 1–6 weeks, with patch releases clustered after each. The project is young (under nine months) and pre-1.0, so expect API churn: `0.8 → 0.9` has an upgrade guide (`docs/release/V080_TO_V090_UPGRADE_GUIDE.md`), and `-e` CLI aliases were removed in 0.9.0. Several sibling packages (`tree-sitter-mermaid`, Flutter) are versioned independently. The README requires Rust 1.95+.

## Mermaid.js compatibility: philosophy and testing

**Reported target.** Published `0.9.0`/`0.9.1` pin **Mermaid 12.1.0**. The development branch (unreleased) targets **Mermaid 12.2.0** at commit `1fb78698…`, pinned in `tools/upstreams/REPOS.lock.json`.

**Philosophy** (from `docs/workstreams/PARITY_BOUNDARY.md`):

- Optimize for semantic compatibility (same graph, labels, links, security decisions), stable visual similarity, and a deterministic, browser-free implementation.
- Chromium pixel parity is a *diagnostic*, not the product boundary. Sub-pixel browser text metrics are not chased with hand-written special cases.
- Mermaid semantics and layout rules *must* be derived in code. "Fixture-answer literals" (per-fixture or per-icon constants added to turn a diff green) are explicitly rejected.
- Residual drift (text measurement, `foreignObject` HTML labels, RoughJS hand-drawn paths) is documented per family rather than hidden.

**Testing.**

- Upstream Mermaid is run to generate reference SVGs; a per-family `compare-*-svgs` xtask command compares Merman's SVG DOM against them. Modes are `structure`, `parity`, `parity-root` (also checks root `viewBox`/`max-width`), and `strict` XML.
- All 37 primary families have fixture corpora ("N" normalized, "N+D" plus deferred cases) and a registered executable compare fact. Each family has `*_MINIMUM.md` and `*_UPSTREAM_TEST_COVERAGE.md` docs mapping upstream tests to local ones.
- The 12.2.0 runtime was bound in all 37 manifests, with three-mode DOM and strict validation passing on 2026-10-13 (per `docs/alignment/STATUS.md`, which warns a strict result belongs to the exact revision that ran it).
- CI also runs fuzzing (`fuzz.yml`), a `mermaid-admission.yml` workflow, and security audits.

Caveat: the dashboard's own wording is that admission "does not certify that every gate passes after the latest uncommitted changes" and that pixel-identical Chromium output is not guaranteed.

## Supported and unsupported diagram types

`docs/alignment/STATUS.md` lists **37 families in the primary SVG matrix**, all with semantic, layout, and SVG coverage:

`agentflow`, `usecase`, `er`, `flowchart`, `state`, `class`, `sequence`, `info`, `pie`, `sankey`, `packet`, `timeline`, `journey`, `kanban`, `gitgraph`, `gantt`, `c4`, `block`, `radar`, `requirement`, `mindmap`, `architecture`, `quadrantchart`, `treemap`, `xychart`, `treeView`, `ishikawa`, `eventmodeling`, `error`, `venn`, `swimlane`, `railroad` (+ `railroadEbnf`, `railroadAbnf`, `railroadPeg`), `wardley`, `cynefin`.

- Beta charts are included: `treeView-beta`, `ishikawa-beta`, `xychart-beta`, `treemap`, `radar`, `packet`, `kanban`, `venn`, `wardley`, `cynefin` and others.
- **ZenUML** is not in the primary matrix. It has a "separate external comparison lane": a full grammar, typed layout, and headless SVG are implemented, but fidelity is checked against the ZenUML plugin run in an opaque browser realm.
- I found **no Mermaid family that Merman wholly lacks**. Gaps are in fidelity, not coverage: browser text metrics (e.g. `treeView`), RoughJS vs Rust `roughr` path geometry for hand-drawn looks, and `foreignObject` HTML labels on consumers that cannot display them.
- **ASCII/Unicode output is much narrower** (`docs/rendering/ASCII_SUPPORT_MATRIX.md`): flowcharts are only "Partial" (icons, images, callbacks, links, some shapes unsupported); mindmap and treeView render as outlines.
- Diagram support is per-Cargo-feature: a known family compiled out returns `UnsupportedDiagram`.

## Merman vs mermaid-rs-renderer (mmdr)

|                | Merman                                                                                                     | mmdr (`1jehuang/mermaid-rs-renderer`)                                         |
| -------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Latest release | 0.9.1 (2026-10-18)                                                                                         | 0.3.1 (2026-07-06); first 2026-01-23, 7 crates.io versions                    |
| License        | MIT OR Apache-2.0 (ELK part EPL-2.0, math fonts may be OFL-1.1)                                            | MIT                                                                           |
| Families       | 37 (+ZenUML lane)                                                                                          | 23                                                                            |
| Goal           | Mermaid DOM/semantic parity, pinned to upstream                                                            | Fast native renderer; README says output may not match mermaid-cli everywhere |
| Layout         | Ports of Dagre, Cytoscape/fCoSE, ELK (compiled in by default)                                              | Own layout engine                                                             |
| Outputs        | SVG, PNG, JPEG, PDF, Unicode/ASCII, layout JSON                                                            | SVG, PNG; plus a `scene` drawing-command API                                  |
| Extras         | LSP, linting, tree-sitter grammar, Markdown/Rustdoc tooling, WASM/Node/FFI/Python/Flutter bindings, themes | CLI and library; `--fastText` calibrated widths                               |
| Zed            | Current backend                                                                                            | Zed's earlier backend (`mermaid-rs`, replaced by PR #57644)                   |

Mind the naming: the crates.io crate is `mermaid-rs-renderer`; Zed's *previous* backend was named `mermaid-rs`, which the PR text and a Zed discussion conflate with it. I did not confirm they are the same project.

**Output formats of mmdr.** SVG and PNG only (`-e svg|png`). PNG is produced by SVG → `usvg`/`resvg` behind a default-on `png` feature. The README mentions no JPG or PDF. Flags include `-i`, `-o`, `-c config.json` (themeVariables, spacing), `--width/--height`, `--nodeSpacing`, `--rankSpacing`, `--preferredAspectRatio`, `--fastText`. Taken from the project's README; I did not build it.

## Examples (Rust API and CLI)

Pattern: `Renderer::new().render(RenderRequest::<target>(source, OperationControl::new(), <Request>))`, then match the `RenderOutput` variant. The SVG form below is taken from the README and from `crates/merman/examples/theme_preset.rs`. I did not compile the PNG/ASCII variants, so verify field names against docs.rs.

```rust
use merman::{OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};

fn svg(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let out = Renderer::new().render(RenderRequest::svg(
        source, OperationControl::new(), SvgRequest::default()))?;
    let RenderOutput::Svg(Some(svg)) = out else { return Err("no diagram".into()) };
    Ok(svg.svg().to_owned())
}
```

Five sources, each a valid Mermaid diagram passed to the function above (or `merman-cli render`):

```text
flowchart LR
    A[Source] --> B{Parse?}
    B -- yes --> C[Layout]
    B -- no --> D[Error]

sequenceDiagram
    participant U as User
    participant R as Renderer
    U->>R: render(source)
    R-->>U: svg

pie title Build time
    "Parse" : 10
    "Layout" : 55
    "Paint" : 35

gantt
    title Release
    dateFormat YYYY-MM-DD
    section Build
    Implement :a1, 2026-10-01, 7d
    Test      :after a1, 4d

erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE_ITEM : contains
```

CLI forms (from the CLI README): `merman-cli render d.mmd --output d.svg`, `--format png|jpg|pdf|unicode`, `--raster-fit-width 1600`, `--theme-preset <name>`, `--background white`, `--font-file`/`--font-dir`, `--icon-pack @iconify-json/logos`. For PNG in code, enable the `png` feature and use `RenderRequest::png(source, control, PngRequest { svg, options })` (`options` is `merman_export::RasterOptions`; `RenderOutput` has a matching variant and the result exposes `bytes()`). For a Zed-like pipeline set `request.pipeline = Some(SvgPipeline::resvg_safe())` so the SVG has no `foreignObject` HTML labels.

## Styles and themes

Three layers, in precedence order of the project docs (`MERMAID_THEME_STYLE_PRECEDENCE.md`, `custom-diagram-themes.md`):

1. **Mermaid configuration**: `themeVariables`, `theme:` (`default`, `base`, `dark`, `forest`, `neutral`, `neo`, `redux` variants), frontmatter, and `%%{init}%%` directives, plus a `site_config` / `MermaidConfig` given to the renderer. Zed uses this for base colors.
2. **Compiled themes (new, unreleased on `main`)**: `DiagramThemeCompiler::compile_preset(ThemePreset::…)` then `RenderRequest::with_theme(theme)`. Presets include Brutalist, Spotless, Cyberpunk, OneDark, editor-dark. Themes are versioned JSON (`ThemeDesignV1`, `ThemeRecipeV1`) usable from Rust, Web, Node, and the CLI (`--theme-preset`, `--theme-file`, `--theme-design-file`). The authoring format is marked **alpha**. This replaces the removed "presentation profile" API.
3. **Raw CSS**: bounded CSS admission is capability-gated; fonts are referenced by name only. No fonts are bundled, and font bytes in theme recipes are rejected.

The repo also ships a `design-merman-theme` agent skill and a Playground **Theme** panel (presets, import/export, share links).

### Zed

- **CSS public and licensed?** Yes, public, but it lives *in Zed*, in `crates/mermaid_render` (`postprocess/inject_css.rs`, `accent_colors/`). That crate declares `license = "GPL-3.0-or-later"` with a `LICENSE-GPL` file. Zed's colors come from the user's theme, so there is no standalone stylesheet. Reusing it in a non-GPL project is not permitted without a separate grant; Merman itself is MIT/Apache and has no such constraint.
- **Abstractions Zed adds** (from the crate docs): a `MermaidTheme` struct (gpui `Hsla` colors, dark flag, font) converted into a `merman::MermaidConfig`; rendering with `layout-cytoscape` + `svg` features and the raster-safe pipeline; then a `quick-xml` event-iterator post-process in passes (`accent_colors` assigning `zed-accent-N` classes from `player_colors`, `inject_css`, `element_fixup`, `strip_foreignobject`), rasterized by `usvg`/`resvg`. Failure falls back to showing source rather than panicking. Merman 0.6 absorbed Zed's generic `foreignObject`/CSS cleanup; Zed keeps theme CSS, accents, and the `data-merman-foreignobject="fallback"` overlay handling.
- **Tabbed source/preview toggle:** *Not part of Merman* (a headless library with no UI). In Zed I found the toggle in PR #56430 (merged 2026-05-13), which adds Mermaid to the **Agent Panel**, rendering the diagram by default with a switch back to code. The Markdown preview (Mermaid support since Zed 0.225.0, 2026-02) renders the diagram, falling back to the source block on failure; I found no evidence of a toggle there. A tab/toggle UI belongs to the host application. Merman's Playground has its own panes, which is separate.

## Icon packs

Yes, Iconify-format packs. The library builds an immutable registry from IconifyJSON bytes (`svg` feature; transactional, validated, no partial publish). The CLI adds sources: `--icon-pack @iconify-json/logos` (local `node_modules`), `--icon-pack-source logos#icons.json`, and `file://` under the `icons` feature; HTTP(S) needs `network-icons` plus `--allow-network`. Limits: at most 16 packs, 16 MiB per pack, 32 MiB total, and 32,768 direct-icon entries per registry. A calibration run (`ICON_REGISTRY_CALIBRATION_2026-08-04.md`) was done against complete `logos`, `material-symbols`, `mdi`, and `simple-icons` collections (29,739 icons). Mermaid's `architecture` (`service db(logos:aws)`) and mindmap `::icon()` use them. No packs are bundled. ASCII output does not render icons.

## Performance

Merman's performance docs (`docs/performance/`, ~150 dated reports) are internal regression tracking. Treat all numbers as maker-reported.

- **vs mmdr** (2026-07-28, Apple M4 Pro, Criterion, warm in-process, same-input fixtures): across 30 byte-identical fixtures, Merman faster in 18 and slower in 12; **median Merman/mmdr ratio 0.664×**, geometric mean 0.297×. Flowchart dominates: `flowchart_medium` 3.52 ms vs 101.33 ms; `flowchart_ports_heavy` 1.11 ms vs 1.24 s. mmdr leads on tiny inputs: `class_tiny` 52 µs vs 18 µs, `architecture_medium` 40 µs vs 18 µs, `mindmap_medium` 165 µs vs 74 µs. The report itself warns the aggregate "is not a quality-adjusted score" and that the engines differ in layout and output richness. An earlier Windows run (2026-05-10, i7-11700) was close to parity on flowchart (8.34 ms vs 13.58 ms) with mmdr ahead on architecture and mindmap.
- **vs Mermaid.js**: the harness measures repeated warm `mermaid.render()` in one Puppeteer/Chromium process (p50/p95/p99), and separately a browser-WASM lane; but I found **no published Merman-vs-Mermaid.js ratio** (the May table has `-` there). Do not cite one. mmdr's own README claims roughly 100–1400× over mermaid-cli (cold CLI, Linux, mermaid-cli 11.4.2), shrinking to ~48× at 200 nodes; that mostly measures avoiding browser startup and is unverified.
- Cold-start CLI, font measurement, and theme overhead have their own reports (e.g. `CLI_STARTUP_INVESTIGATION_2026-10-13.md`).

## Fit for biscuit-visualized

- Merman gives terminal-adjacent SVG→PNG without Chromium, with broad family coverage and a stable Rust API; the cost is dependency weight (default features compile ELK, EPL-2.0 notices apply) and pre-1.0 churn. Disable ELK with `default-features = false, features = ["all-diagrams", "complete-svg"]`.
- Use `SvgPipeline::resvg_safe()` or the PNG target to avoid `foreignObject` labels when rasterizing with resvg.
- Pin the exact version; theming is the least stable area (alpha on `main`, not in 0.9.1).
- Following Zed's pattern (a thin theme-to-`MermaidConfig` layer plus post-processing) is sound. Copying Zed's code would pull in GPL-3.0-or-later obligations.

## Gaps in this research

- Not compiled or run: PNG/ASCII example code, mmdr, or the benchmarks.
- No independently verified Merman-vs-Mermaid.js numbers.
- Zed's Markdown preview behavior on source/preview toggling was inferred from search results, not from Zed's source.

## Sources

- Local clone: `README.md`, `CHANGELOG.md`, `docs/alignment/STATUS.md`, `docs/workstreams/PARITY_BOUNDARY.md`, `docs/FEATURES.md`, `crates/merman-cli/README.md`, `docs/rendering/custom-diagram-themes.md`, `docs/performance/*`
- [Merman](https://github.com/Latias94/merman) · [crates.io/merman](https://crates.io/crates/merman)
- [mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer)
- [Zed PR #57644](https://github.com/zed-industries/zed/pull/57644) · [Zed PR #56430](https://github.com/zed-industries/zed/pull/56430) · [zed `mermaid_render`](https://github.com/zed-industries/zed/tree/main/crates/mermaid_render)
