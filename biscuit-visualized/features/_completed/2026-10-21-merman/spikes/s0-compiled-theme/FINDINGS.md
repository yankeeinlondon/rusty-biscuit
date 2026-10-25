# Spike S0: Merman compiled themes

Spike for `2026-10-21-merman` (design note "Two ways to build the styling layer", Q3). Run 2026-10-21. Throwaway code; not a workspace member.

## Question

Can our theming (D6, D7) be expressed with Merman's alpha compiled-theme API (`ThemeDesignV1`/`ThemeRecipeV1` → `DiagramThemeCompiler` → `RenderRequest::with_theme`)? Specifically:

1. background, foreground, node, edge, and label colors for a flowchart;
2. rotating accents on un-styled nodes, or which part must be our own post-processing;
3. `SvgPipeline::resvg_safe()` plus resvg 0.45;
4. sequence and gitGraph;
5. gitGraph layout data through `layout_json`.

## Setup

- Merman: git dependency `https://github.com/Latias94/merman.git`, `rev = 482fcde7faffb05a466f198085d615e604075d97` (`main`, 2026-10-20, crate 0.9.1). The git fetch worked, so no path fallback was needed.
- Features: `default-features = false`, `["svg", "diagram-flowchart", "diagram-sequence", "diagram-git-graph"]` (no ELK, no Cytoscape, no `png`).
- Rasterizer: our own `resvg = "0.45"` (0.45.1), system fonts, as in `biscuit-visualized/src/src/raster/png.rs`.
- Palette: a hand-typed Tokyo-Night-like palette: bg `#1a1b26`, fg `#c0caf5`, ANSI 0–15; accents are ANSI 1–6.
- Run: `CARGO_TARGET_DIR=<outside repo> cargo run` (writes `out/`); `cargo run -- --probe` runs one-facet isolation probes.
- The design is built as `serde_json` and deserialized into `ThemeDesignV1`, which is the wire shape a palette type would emit. `out/theme-design.json` is the recommended design; `out/theme-design-gitgraph.json` is the gitGraph variant.

## 1. Base colors (flowchart): works

`canvas.base`, `node` (fill, stroke paint, stroke width), `node-label`, `edge`, `edge-label`, `edge-label-background`, `cluster`, `cluster-label`, `text`, and `title` all apply, in both `neo` (default) and `classic`. Tokens (`{"ref":"fg"}`) work. Compile takes 1–12 ms.

- The canvas paint reaches the PNG (corner pixel `#1a1b26`, opaque; the baseline is transparent).
- Arrowheads follow the edge color in the output. A `marker` style is unsupported (`no-supported-route` / `unsupported-paint`), and we do not need one.
- Author styles win: `classDef` and `style` nodes keep their own fill and stroke (only the theme's stroke width reaches them). This matches R-T3.

## 2. Rotating accents: fill yes, stroke no

| Mechanism | Result |
|---|---|
| `palettes.node` (ordinal palette) | works: fill rotates per node |
| `rules[]` with `ordinal` + `fill` | works |
| `rules[]` with `ordinal` + `stroke` | unsupported (`unsupported-paint`) |

The Zed-style rotating accent border stays ours. The spike's `accent_strokes()` rewrites the inline `fill:<tint_i> !important;stroke:<muted> !important` to `stroke:<accent_i>` (`out/flowchart-themed-accent-postprocess.png`). It keys on colors we chose, not on Merman class names.

A `<style>` block cannot do this, because Merman writes theme paint as inline `!important`, which a stylesheet cannot override. Our post-processing must edit attributes. This also limits D6's "scoped CSS injection": it can add styling but cannot override theme paint.

Precedence surprises, each confirmed with a probe:

1. A static `styles.node.fill` beats `palettes.node`, although the guide says palettes apply last. Leave the node base fill unset.
2. An unsupported rule erases a supported one. Unsupported ordinal stroke rules removed the static node stroke from every node (`out/flowchart-ordinal-stroke-rules.png`). Unsupported gitGraph ordinal fill rules sent commits back to Mermaid's default colors (`out/gitgraph-ordinal-rules.png`).
3. `design.mermaid.variables` overrides the ordinal palette (every flowchart node took `primaryColor`) and turned the sequence diagram from `Verified` to `Residual` (`out/flowchart-themed-vars.png`). Keep them out of the flowchart design.

Circles report `unsupported-paint` for the palette and node rule but render painted. Revisit this in the styling phase.

## 3. Raster-safe pipeline and resvg 0.45: works

All 14 renders have no `foreignObject` and exactly one `<style>`, and they rasterize with resvg 0.45.1 without error (240–520 ms each in a debug build, mostly font loading). Merman's own resvg/usvg 0.47 comes in only with its `png` feature; with that off, the tree holds one usvg (0.45.1).

The SVG requests `"Recursive Variable",arial,sans-serif`, so it falls back to Arial here. This is the font-measurement risk the spec already lists.

## 4. Sequence and gitGraph

**Sequence: works fully.** `actor`, `actor-label`, `lifeline`, `message`, `message-label`, `note`, `note-label`, `activation`, `loop`, `loop-label`, and `text` all apply, with status `Verified` and no diagnostics. An ordinal palette on `actor` is unsupported, so per-actor accents would be post-processing.

**gitGraph: works with one design per family.**

- `palettes.node` drives the branch colors (lanes, commit dots, branch labels). Ordinal rules are unsupported (surprise 2).
- Palettes cannot be family-scoped, and the flowchart tints look muddy as branch colors (`out/gitgraph-themed.png`). Compile a separate design per family (1–12 ms each) with full-strength accents.
- Commit and tag labels were not reached by the compiled styles tried. `design.mermaid.variables` (`commitLabelColor`, `tagLabel*`, `git0..7`, `gitBranchLabel0..7`) fixes them here without side effects (`out/gitgraph-themed-family.png`, the best result).
- Stroke width on `edge` or `node` gives `unsupported-geometry`; omit widths.
- The family id is `gitGraph`; `git-graph` returns `unknown-family`.

## 5. gitGraph layout data

`RenderRequest::layout_json(src, control, SvgRequest)` returns `RenderOutput::LayoutJson(Some(SvgLayoutOutput))`, whose `.layout()` is a `ManagedSemanticJson` with `meta`, `semantic`, and `layout`.

`layout.GitGraphDiagram` holds:

- `bounds`, `direction`, `diagram_padding`, `max_pos`, and flags;
- `branches[] {name, index, pos, bbox_width, bbox_height}`;
- `commits[] {id, message, seq, commit_type, custom_type, custom_id, tags[], parents[], branch, pos, pos_with_offset, x, y}`;
- `arrows[] {from, to, class_index, d}`.

Parents, positions, and lanes are present (`out/gitgraph-layout.json`). Tag bounds are not (tags are names only, so they must come from the SVG). There is no `commit_step`, but spacing shows in `pos` (50 px here). The typed structs in `merman-render/src/model.rs` are not re-exported by the facade, so we would deserialize into our own structs.

## Styling that remains ours

| Need | Why | How |
|---|---|---|
| Accent stroke per un-styled flowchart node | ordinal stroke unsupported | inline-style rewrite keyed on the palette fill |
| Sequence actor accents | `actor` ordinal palette unsupported | same approach |
| Class and mindmap accents | not tested; Merman's source lists mindmap `node` ordinal palette as supported | verify in the styling phase |
| Contrast-aware label over tints | one `node-label` color | pick readable tints, or rewrite per node |
| Scoped CSS | cannot override inline `!important` | use only for things the theme does not set |
| gitGraph commit and tag label colors | compiled styles did not reach them | Mermaid variables in the gitGraph design (no CSS) |

## API stability

- Everything is versioned (`schema_version: 1`, `*V1` types, `deny_unknown_fields`, `#[non_exhaustive]`, `claim_revision`), and the docs call it "experimental" and "alpha, revision-scoped".
- The facade `merman::diagram_theme` re-exports everything needed, so there is no direct dependency on `merman-render` or `merman-theme-contract`.
- Behavior departs from the docs (precedence surprises above). Golden snapshots plus assertions on `theme_evidence().status()` and `theme_diagnostics()` (`code()`) are needed to catch upgrade drift.
- `describe_theme_support` for `png` always returns `unverified` (`output-qualification-not-owned-by-renderer`), so raster safety must be proved by our own tests.
- There is heavy recent churn in the theme area on `main`. Pin an exact rev.

## Compile time and dependencies (16-core Apple Silicon)

- Uncached debug: 23 s. Uncached release: 43 s. Cached debug (kache): 19 s.
- About 211 entries in `cargo tree` (normal + build); 182 packages in `Cargo.lock`; 145 unique under `merman`.
- Merman crates compiled: `merman`, `merman-core`, `merman-render`, `merman-theme-contract`, `merman-unicode-linebreak`, `dugong`, `dugong-graphlib`, `roughr-merman`.
- Release binary: 18 MB unstripped.
- Duplicate versions: base64, bitflags, kurbo, miniz_oxide, and phf (build).

## Recommendation

**Partially viable.** Use compiled themes as the main mechanism and add a small post-process that edits inline styles for accent strokes; do not fall back to `themeVariables` plus CSS.

- Emit one design per family.
- Emit only facets that `describe_theme_support` reports as `conditional` or `supported`.
- Leave the node base fill unset.
- Golden tests should assert the evidence status and the absence of `unsupported-*` diagnostics per family.

The main risk is an alpha API on unreleased `main`: pin the rev and plan a migration when a release ships this API.
