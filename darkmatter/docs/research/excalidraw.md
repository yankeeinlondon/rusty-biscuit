---
prompt: |-
    ## Context

    The [Excalidraw](https://github.com/excalidraw/excalidraw) project is a popular way for people to illustrate drawings and we would like to be able to render these diagrams in **Darkmatter**. We would need to be able to:

    - render a file reference to a `*.excalidraw` file _or_ reference the JSON inside of a codeblock
    - in both scenarios we will need to render to the following targets:
        - **terminal** (e.g., convert to raster image and render in terminals which support Kitty)
        - **markdown**
            - generate a sidecar image file and have the Markdown reference the diagram as an image
            - we will likely also provide the option to embed as 
        - **html** (e.g., convert Darkmatter document to HTML, CSS, JS)

    ## Task

    Your job is not to implement this in Darkmatter but instead to provide the foundational research on Excalidraw which we
    will need to implement this. Your research should be able to answer the following questions:

    - What is the overall architecture of Excalidraw?
        - what does the render process look like?
        - what programming languages are involved?
    - Are there any Rust crates which would help in the implementation of this?
        - If there are, what features do they expose and what does each feature bring to the crate?
    - Can the rendering of a document be done in pure Rust or do we need to rely on Excalidraw technology stack to be on the
      host computer so that we can leverage at "compose" time?
    - What "gotchas" do developers report encountering when working with similar tasks? How can the pitfalls be avoided?

    The research content should be written to the body of this document as idiomatic Markdown (Commonmark + GFM). If you want
    to provide a visualization then using Mermaid diagrams is the preferred way to do that.
last_updated: 2026-05-11
---

## Excalidraw Architecture Overview

Excalidraw is an open-source virtual whiteboard for sketchy hand-drawn diagrams. The codebase is overwhelmingly **TypeScript (~94%)** with SCSS for styling and a small amount of MDX/JS/HTML, distributed as a React component library targeting modern browsers ([excalidraw/excalidraw](https://github.com/excalidraw/excalidraw)).

### Render backend

Excalidraw renders to **HTML `<canvas>`** at runtime using a dual-canvas architecture:

- A **static canvas** holds the committed scene (shapes, text, images).
- An **interactive canvas** overlays selections, handles, and editing UI.

The static canvas draws shape primitives through **[Rough.js](https://roughjs.com/)** — a JavaScript library by Preet Shihn that produces the hand-drawn "sketchy" look by jittering paths and applying hachure fills. Rough.js itself is rendering-backend-agnostic and can target Canvas2D or emit SVG path data; Excalidraw uses both: Canvas2D for the editor, SVG path output for `exportToSvg`.

### Monorepo layout

The current `packages/` directory (master branch) ships these workspace packages:

| Package                             | Purpose                                                                                                                                                                                                                         |
|-------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `@excalidraw/excalidraw`            | The React component you embed. Exports `<Excalidraw />` plus the public APIs (`exportToCanvas`, `exportToSvg`, `exportToBlob`, `exportToClipboard`, `serializeAsJSON`, `restoreElements`, `convertToExcalidrawElements`, etc.). |
| `@excalidraw/element`               | Element types, geometry, bindings, hit-testing, transforms, snapping.                                                                                                                                                           |
| `@excalidraw/math`                  | Pure 2D math primitives (points, vectors, segments, arcs, intersections).                                                                                                                                                       |
| `@excalidraw/common`                | Constants, color palettes, shared utilities, the schema version constant.                                                                                                                                                       |
| `@excalidraw/utils`                 | Standalone export helpers (`exportToBlob`, `exportToSvg`, `exportToCanvas`, `loadFromBlob`, `loadLibraryFromBlob`, `serializeAsJSON`, `restore`).                                                                               |
| `@excalidraw/fractional-indexing`   | Excalidraw's fractional z-index/order indexing (for stable ordering across collaborative edits).                                                                                                                                |
| `@excalidraw/mermaid-to-excalidraw` | Converts Mermaid source into Excalidraw element skeletons. Lives in its own repo ([excalidraw/mermaid-to-excalidraw](https://github.com/excalidraw/mermaid-to-excalidraw)) and depends on Mermaid (JS).                         |

> Note: `@excalidraw/mermaid-to-excalidraw` is no longer in the main repo's `packages/` directory — it's a separate repository published to npm.

### Render pipeline

```mermaid
flowchart LR
    A[.excalidraw JSON] --> B[restore<br/>normalize + migrate]
    B --> C[ExcalidrawElement[]<br/>+ appState + files]
    C --> D[renderStaticScene]
    D --> E[RoughGenerator<br/>seed-driven]
    E --> F{Output target}
    F -->|editor / exportToCanvas| G[HTML Canvas 2D]
    F -->|exportToSvg| H[SVG path 'd' strings]
    G --> I[PNG via canvas.toBlob<br/>exportToBlob]
    H --> J[SVG document]
    I -.optional.-> K[Embed scene<br/>tEXt chunk]
    J -.optional.-> L[Embed scene<br/>SVG metadata]
```

The critical step is `RoughGenerator` consuming each element's stored `seed` so the sketchy stroke is **identical on every render** (see [Determinism and `seed`](#common-gotchas) below).

### Official export utilities

From `@excalidraw/utils` / `@excalidraw/excalidraw` ([Export Utilities docs](https://docs.excalidraw.com/docs/@excalidraw/excalidraw/api/utils/export)):

- `exportToCanvas({ elements, appState, files, getDimensions?, exportPadding?, maxWidthOrHeight? })` — returns an `HTMLCanvasElement`.
- `exportToBlob({ ...same, mimeType?, quality? })` — wraps `exportToCanvas` and returns a `Blob` (PNG by default).
- `exportToSvg({ elements, appState, files, exportPadding?, renderEmbeddables?, exportingFrame? })` — returns an `SVGSVGElement`.
- `exportToClipboard({ ...same, type: "png" | "svg" | "json" })` — copies to system clipboard.
- `loadFromBlob(blob, localAppState, localElements, fileHandle?)` — reverse direction: extracts an embedded scene from a `.excalidraw`, `.excalidraw.png`, or `.excalidraw.svg`.

Export options live partly in `appState` and partly as direct function arguments:

| Option                | Where        | Default     | Meaning                                                      |
|-----------------------|--------------|-------------|--------------------------------------------------------------|
| `exportBackground`    | `appState`   | `true`      | Paint `viewBackgroundColor` behind elements.                 |
| `viewBackgroundColor` | `appState`   | `"#ffffff"` | Background color (also used as SVG `<rect>` fill).           |
| `exportScale`         | `appState`   | `1`         | DPR multiplier — canvas dims become `w * exportScale`.       |
| `exportWithDarkMode`  | `appState`   | `false`     | Applies the dark-theme color filter.                         |
| `exportEmbedScene`    | `appState`   | `false`     | Embed the JSON scene inside PNG/SVG metadata for round-trip. |
| `exportPadding`       | function arg | `10` px     | Padding around the bounding box.                             |
| `maxWidthOrHeight`    | function arg | unbounded   | Hard cap on either dimension (overrides `exportScale`).      |

## The `.excalidraw` File Format

### Top-level JSON shape

```json
{
  "type": "excalidraw",
  "version": 2,
  "source": "https://excalidraw.com",
  "elements": [ /* ExcalidrawElement[] */ ],
  "appState": {
    "viewBackgroundColor": "#ffffff",
    "gridSize": null
  },
  "files": {
    "<fileId>": {
      "mimeType": "image/png",
      "id": "<fileId>",
      "dataURL": "data:image/png;base64,iVBORw0KGgo...",
      "created": 1714000000000,
      "lastRetrieved": 1714000000000
    }
  }
}
```

- `type` — always `"excalidraw"` for documents, `"excalidrawlib"` for libraries, `"excalidraw/clipboard"` when copied to clipboard.
- `version` — schema version (`2` at time of writing). Bumps trigger migration in `restore.ts` ([Scene Serialization](https://deepwiki.com/excalidraw/excalidraw/6.2-json-serialization)).
- `source` — origin URL; used only for diagnostics.
- `elements` — array of `ExcalidrawElement` discriminated by `type`.
- `appState` — UI/editor state. Most fields are NOT persisted to disk (`server: false`); the relevant exported keys are `viewBackgroundColor`, `gridSize`, and the `export*` settings above.
- `files` — `BinaryFiles` map keyed by `fileId`, containing base64 data URLs referenced by `image` elements.

### Element types and base fields

All elements extend `_ExcalidrawElementBase` ([packages/element/src/types.ts](https://github.com/excalidraw/excalidraw/blob/master/packages/element/src/types.ts)):

```ts
type _ExcalidrawElementBase = {
  id: string;
  type: ElementType;
  x: number;
  y: number;
  width: number;
  height: number;
  angle: number;           // radians
  strokeColor: string;
  backgroundColor: string;
  fillStyle: "solid" | "hachure" | "cross-hatch" | "zigzag" | "zigzag-line" | "dots" | "dashed";
  strokeWidth: number;     // 1 | 2 | 4 typical
  strokeStyle: "solid" | "dashed" | "dotted";
  roughness: 0 | 1 | 2;    // 0 = "architect", 1 = default, 2 = "cartoonist"
  roundness: { type: number; value?: number } | null;
  opacity: number;         // 0..100
  seed: number;            // PRNG seed for Rough.js
  version: number;         // monotonically incremented on edit
  versionNonce: number;    // random nonce for collab tie-break
  index: string | null;    // fractional z-index
  isDeleted: boolean;
  groupIds: string[];
  frameId: string | null;
  boundElements: { id: string; type: "arrow" | "text" }[] | null;
  updated: number;         // ms timestamp
  link: string | null;
  locked: boolean;
};
```

Element-type-specific fields:

| Type                                | Extra fields                                                                                                                                                                                                                                                                                |
|-------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `rectangle` / `ellipse` / `diamond` | none beyond base                                                                                                                                                                                                                                                                            |
| `arrow` / `line`                    | `points: [x, y][]` (local, relative to `x,y`); `startBinding`/`endBinding: { elementId, focus, gap } \| FixedPointBinding`; `startArrowhead`/`endArrowhead: "arrow" \| "dot" \| "triangle" \| "bar" \| null`; `elbowed: boolean` for elbow arrows                                                |
| `freedraw`                          | `points: [x, y][]`; `pressures: number[]`; `simulatePressure: boolean`; `lastCommittedPoint`                                                                                                                                                                                                |
| `text`                              | `text: string`; `fontSize: number`; `fontFamily: 5 \| 6 \| 7 \| 8` (Excalifont/Nunito/Lilita One/Comic Shanns); `textAlign: "left" \| "center" \| "right"`; `verticalAlign: "top" \| "middle"`; `containerId: string \| null`; `originalText: string`; `autoResize: boolean`; `lineHeight: number` |
| `image`                             | `fileId: string`; `status: "pending" \| "saved" \| "error"`; `scale: [number, number]` (e.g. `[-1, 1]` for horizontal flip); `crop: { x; y; width; height; naturalWidth; naturalHeight } \| null`                                                                                              |
| `frame` / `magicframe`              | `name: string \| null`; children are addressed via `frameId` on other elements                                                                                                                                                                                                               |
| `embeddable`                        | renders an iframe-embed of a URL (`link`)                                                                                                                                                                                                                                                   |
| `iframe`                            | optional `customData.generationData` (used by Magicframe / AI features)                                                                                                                                                                                                                     |

### Arrow / linear binding internals

Arrows attach to shapes through a bidirectional reference ([Arrow Bindings](https://deepwiki.com/ahmadawais/excalidraw-cli/4.4-arrow-bindings)):

```jsonc
{
  "type": "arrow",
  "startBinding": { "elementId": "abc", "focus": -0.023, "gap": 15.68 },
  "endBinding":   { "elementId": "def", "focus":  0.0,   "gap":  5.00 }
}
// And on each bound shape:
{ "id": "abc", "boundElements": [{ "id": "<arrow-id>", "type": "arrow" }] }
```

- `focus` — normalized position along the shape edge (`-1..1`, `0` = center).
- `gap` — distance in pixels from the shape outline.

PR [\#9670](https://github.com/excalidraw/excalidraw/pull/9670) introduced an alternative `FixedPointBinding` form `{ elementId, mode: "inside" | "orbit", fixedPoint: [nx, ny] }` for newer elbow/non-elbow snapping, with a migration path in `data/restore.ts`.

### Embedded images and the `files` map

`image` elements carry only a `fileId`. The actual bytes live in the document's `files` map as a base64 data URL with `mimeType`, `id`, `dataURL`, `created`, and `lastRetrieved` timestamps. If a renderer encounters an `image` element whose `fileId` is missing from `files`, the image is displayed as a broken-image placeholder.

### Sidecar formats: `.excalidraw.png` and `.excalidraw.svg`

Excalidraw can embed the **entire scene JSON** inside an exported PNG or SVG so the rendered image is round-trippable through the editor ([Scene Embedding](https://deepwiki.com/zsviczian/excalidraw/7.3-scene-data-embedding-and-serialization)):

- **PNG**: scene is `serializeAsJSON()` → encoded + compressed → injected as a PNG **`tEXt` chunk** with keyword `application/vnd.excalidraw+json`, inserted immediately before the `IEND` chunk. Excalidraw is migrating toward **`iTXt`** chunks for proper UTF-8 support ([\#9269](https://github.com/excalidraw/excalidraw/issues/9269)), while continuing to read `tEXt` for backward compatibility.
- **SVG**: scene is appended as an SVG `<metadata>` element containing the same encoded/compressed payload.
- **Import** (`loadFromBlob`): scans for the metadata chunk, decompresses, runs `restore()` to rebuild elements, fix bindings, and validate. If no embedded payload is found it throws `ImageSceneDataError` with code `IMAGE_NOT_CONTAINS_SCENE_DATA`.

Relevant source: `packages/excalidraw/data/image.ts` (`encodePngMetadata`), `packages/excalidraw/data/blob.ts`, `packages/utils/src/export.ts`.

## Rust Crate Landscape

### No first-class Excalidraw crate exists

A `crates.io` search surfaces **no general-purpose `excalidraw` parser/renderer crate**. The only Rust crate with the name in its identifier is `mdbook-excalidraw` (an mdBook preprocessor; not a renderer). For Darkmatter this means we either roll our own `serde` model or shell out to a JS tool.

### `roughr` — Rust port of Rough.js

The most important upstream library is **[`roughr`](https://crates.io/crates/roughr)** v0.12.0 (MIT), part of the [`orhanbalci/rough-rs`](https://github.com/orhanbalci/rough-rs) workspace. Verified directly from the repository's `Cargo.toml` files (May 2026):

- `roughr` core has **NO `[features]` section** — it is a pure primitive generator with no optional features. Dependencies are unconditional: `points_on_curve`, `svg_path_ops`, `euclid 0.22`, `rand 0.8`, `num-traits 0.2`, `derive_builder 0.12`, `svgtypes 0.11`, `palette 0.7`.
- Backends are **separate sibling crates**, not feature flags on the core. Pick the one(s) you need:

| Crate                | Version | Pulls in             | Output                                                                    |
|----------------------|---------|----------------------|---------------------------------------------------------------------------|
| `roughr`             | 0.12    | (core, no rendering) | Path primitives only                                                      |
| `rough_piet`         | 0.13    | `piet 0.8`           | Anything Piet targets (Cairo / Direct2D / CoreGraphics via `piet-common`) |
| `rough_tiny_skia`    | 0.12    | `tiny-skia 0.11`     | Raster `Pixmap` → PNG                                                     |
| `rough_plotters_svg` | —       | `plotters-svg`       | SVG document                                                              |
| `rough_iced`         | —       | `iced`               | Iced UI widgets                                                           |
| `rough_vello`        | —       | `vello`              | GPU (wgpu) rendering                                                      |

`roughr` exposes fill styles `Hachure`, `Zigzag`, `Cross-Hatch`, `Dots`, `Dashed`, `Zigzag-Line` — a direct match for Excalidraw's `fillStyle` enum. The `OptionsBuilder` builder takes `stroke`, `fill`, `fill_style`, `fill_weight`, `hachure_angle`, `hachure_gap`, `roughness`, `bowing`, `stroke_width`, `seed`, `disable_multi_stroke`, etc. — note `seed` is exposed, so a Darkmatter renderer can pass each Excalidraw element's `seed` directly to keep doodles stable.

> Caveat: `roughr` uses `rand 0.8` and the Rough.js algorithm depends on the host PRNG sequence. **Identical `seed` values between Rough.js (JS) and `roughr` (Rust) will NOT produce identical pixels** because the underlying PRNGs differ. Visual style will match; exact stroke jitter will not.

### General 2D rendering options for Rust

| Crate       | Type         | Notes                                                                                                                                      |
|-------------|--------------|--------------------------------------------------------------------------------------------------------------------------------------------|
| `tiny-skia` | CPU raster   | Pure-Rust Skia subset (~14 KLOC, +200 KiB binary). No GPU, no text. Backend for `resvg`. ([repo](https://github.com/linebender/tiny-skia)) |
| `resvg`     | SVG → raster | SVG renderer atop `usvg` + `tiny-skia`. ~1600 regression tests, fully reproducible across platforms.                                       |
| `usvg`      | SVG parser   | Simplified SVG AST. Use stand-alone with your own renderer.                                                                                |
| `skia-safe` | CPU/GPU      | Bindings to full Skia. Big build, fast, complete (text, PDF).                                                                              |
| `raqote`    | CPU raster   | Pure Rust, smaller than tiny-skia but slower and less actively maintained.                                                                 |
| `femtovg`   | GPU vector   | OpenGL ES; canvas-style API.                                                                                                               |
| `vello`     | GPU vector   | Linebender's compute-shader 2D engine; cutting edge but `0.x`.                                                                             |
| `piet`      | Abstraction  | 2D-API trait layer (`rough_piet` builds on this).                                                                                          |
| `cairo-rs`  | C binding    | Mature, large system dep.                                                                                                                  |

### Text rendering crates

| Crate         | Role                                                                        |
|---------------|-----------------------------------------------------------------------------|
| `fontdb`      | Font discovery / loading. Used by `usvg-text-layout`.                       |
| `cosmic-text` | Modern shaping + line-break + bidi. Best fit for arbitrary Unicode + emoji. |
| `swash`       | High-quality shaping + rasterization (Pop!_OS).                             |
| `rusttype`    | Older, glyph rasterization only. Largely superseded.                        |
| `ab_glyph`    | Lightweight glyph outlines. Used by `glyph_brush`.                          |

### Standard pure-Rust SVG → PNG pipeline

```text
SVG bytes
   │
   ▼ usvg::Tree::from_data(&data, &opts)
usvg::Tree
   │
   ▼ resvg::render(&tree, transform, &mut pixmap.as_mut())
tiny_skia::Pixmap
   │
   ▼ pixmap.encode_png()
PNG bytes
```

This is the canonical path used by `svg-to-png-cli` and many static-site tools. It is **deterministic across OS/arch** and requires no system libraries ([resvg](https://github.com/linebender/resvg)).

## Pure-Rust Feasibility vs Needing the JS Stack

A pure-Rust Excalidraw renderer is **feasible for the common case** but has real gaps. Recommended Darkmatter architecture: pure-Rust as the default; optional Node/headless fallback for parity-critical cases.

### What pure Rust can handle well

- Rectangle, ellipse, diamond, line, freedraw — straight-line `roughr` mapping.
- Arrows with simple endpoints — endpoints, arrowheads, dashed strokes.
- Solid and hachure fills — supported by `roughr`.
- `roughness 0/1/2`, `strokeStyle`, `strokeWidth`, `opacity`, `angle` — direct mapping.
- Frame backgrounds and group bounding boxes — pure geometry.
- Image elements — decode the base64 `dataURL` from `files` and blit through `image` crate + `tiny-skia` / `resvg`.
- SVG output — generate `<path d="…">` strings directly from `roughr` primitives; this is the closest-to-Excalidraw output path.
- PNG output — `resvg`-render the generated SVG, or rasterize `roughr` paths via `rough_tiny_skia`.

### What is hard or requires the JS stack

| Gap                                                   | Recommendation                                                                                                                                                                           |
|-------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Exact stroke-for-stroke parity with Excalidraw editor | Not possible — Rough.js JS PRNG vs Rust `rand`. Aim for stylistic parity, not pixel parity.                                                                                              |
| Hand-drawn font (Excalifont / Virgil)                 | Bundle `Excalifont-Regular.woff2` (OFL-1.1) via `fontdb` registration. Without it, text falls back to a generic font and the diagram looks wrong.                                        |
| Text layout for mixed scripts, bidi, emoji            | Use `cosmic-text` rather than `ab_glyph`. Excalidraw's `lineHeight` and `textAlign` semantics must be reproduced manually.                                                               |
| Mermaid-in-Excalidraw                                 | `@excalidraw/mermaid-to-excalidraw` invokes Mermaid (JS) — no Rust port exists. Either pre-process via Node, or accept that Mermaid blocks become a placeholder.                         |
| LaTeX/math (community plugins)                        | Out of scope for upstream Excalidraw; ignore unless Darkmatter targets the Obsidian plugin's superset.                                                                                   |
| `embeddable` / `iframe` elements                      | Render as a styled rectangle with the URL as caption. Real iframes require a browser.                                                                                                    |
| Complex elbow-arrow routing                           | Implementable but non-trivial; see `packages/element/src/binding.ts` and `calculateFixedPointForNonElbowArrowBinding`.                                                                   |
| Scene-embedded PNG/SVG round-trip                     | PNG `tEXt`/`iTXt` chunk insertion can be done with the `png` crate. The encoding (compress + base64 + UTF-8) must mirror Excalidraw's exactly — see `packages/excalidraw/data/image.ts`. |

### Headless / JS fallback options

Three community approaches when fidelity matters more than self-contained binaries:

1. **[`Timmmm/excalidraw_export`](https://github.com/Timmmm/excalidraw_export)** — Node CLI that exports `.excalidraw` → SVG/PDF with `--embed-fonts`. Most reliable for SVG.
2. **[`@tommywalkie/excalidraw-cli`](https://github.com/tommywalkie/excalidraw-cli)** — Uses `node-canvas` + a Rough.js reimplementation. Doesn't share Excalidraw's renderer; output drifts from the official look.
3. **[`excalidraw-brute-export-cli`](https://github.com/realazthat/excalidraw-brute-export-cli)** — Playwright + Firefox. Runs Excalidraw's actual `exportToSvg` / `exportToCanvas` in a real browser → identical output, heavyweight dep.
4. **DIY Puppeteer + `renderStaticScene`** — Inject `@excalidraw/excalidraw` into a headless page, call `exportToCanvas`, screenshot.

For Darkmatter, a useful split is: **pure-Rust path for terminal preview and HTML inline rendering** (good-enough fidelity, fast, no Node); **opt-in Node fallback** behind a feature for users who want pixel-exact output for publishing.

## Common Gotchas

### Fonts: Virgil → Excalifont, and broken SVG text

Excalidraw originally shipped **Virgil** ([excalidraw/virgil](https://github.com/excalidraw/virgil), OFL-1.1). In 2024 it was replaced by **Excalifont** ([plus.excalidraw.com/excalifont](https://plus.excalidraw.com/excalifont)) — also OFL-1.1, designed for better legibility while preserving the hand-drawn look. Older `.excalidraw` files still reference Virgil (`fontFamily: 1`); newer ones reference Excalifont. Cascadia (monospace) and Helvetica/Assistant (normal) are the other built-in families.

The well-known broken-text bug: Excalidraw's `exportToSvg` produces `<text>` nodes referencing the fonts via `@font-face` with **URLs pointing back to `excalidraw.com`**. The fonts are only loaded inside a browser, and even then only when the SVG is loaded as a document — not when displayed in an `<img>` tag — and never by Inkscape, macOS Finder previews, GitHub-rendered markdown, or any other non-browser renderer ([\#1972](https://github.com/excalidraw/excalidraw/issues/1972), [\#2192](https://github.com/excalidraw/excalidraw/issues/2192), [\#2263](https://github.com/excalidraw/excalidraw/issues/2263), [excalidraw-vscode #28](https://github.com/excalidraw/excalidraw-vscode/issues/28)).

Mitigations Darkmatter should implement:

- **Bundle Excalifont + Virgil + Cascadia** and inject them as `data:font/woff2;base64,…` `@font-face` rules into emitted SVG (what `excalidraw_export --embed-fonts` does).
- **Optional text-to-path conversion** for maximum portability — see [\#1972](https://github.com/excalidraw/excalidraw/issues/1972). Increases file size but is the only fool-proof option for renderers that ignore SVG fonts.
- For raster output, register the bundled fonts in `fontdb` before invoking `resvg`/`cosmic-text`.

### Determinism and the `seed` field

Rough.js itself has **no built-in seeding**; Excalidraw works around this by storing a `seed` integer on every element and replacing `Math.random` with a seeded PRNG for the duration of each Rough.js call ([\#70](https://github.com/excalidraw/excalidraw/issues/70)). Re-rendering the same scene must honor the same `seed` per element, or the doodle visibly changes — which becomes painful when caching or diffing rendered images.

Subtle caveat ([\#211](https://github.com/excalidraw/excalidraw/issues/211)): when an element has a fill, Rough.js consumes a **variable number of PRNG calls** while drawing the fill, so the stroke's effective seed shifts and resizing the same element changes the stroke. The workaround is to draw fill and stroke separately. Implementors writing a pure-Rust renderer should be aware that pixel-stable resizing requires the same separated approach.

### Arrow binding math is non-trivial

`startBinding.focus`/`gap` plus the new `FixedPointBinding` mean an arrow's actual endpoint is not its serialized `points[0]`/`points[N-1]` — it is computed dynamically from the bound shape's current geometry. Renderers that just draw `points` verbatim will draw arrows that either float in space or fail to touch their targets after edits. The geometry to study lives in `packages/element/src/binding.ts` (`updateBoundElements`, `calculateFixedPointForNonElbowArrowBinding`, `bindBindingElement`).

### Image elements depend on the `files` map

Image elements reference `fileId` but the bytes live in `files`. If `files` is missing or the entry is absent, the editor shows a broken-image placeholder. When importing third-party `.excalidraw` files, **always validate** that every `image` element's `fileId` resolves.

### Schema versions

The top-level `version` field is bumped on schema changes; `data/restore.ts` runs migrations on load (legacy bindings, font-family enum renames, removal of fields, etc.). A naive `serde` model will deserialize an old document but may miss migrations — Darkmatter should either implement the migrations it cares about or accept "best effort" rendering for old files.

### `exportScale`, padding, dark mode

- `exportScale` multiplies output dimensions; `exportPadding` adds whitespace around the bounding box.
- `exportWithDarkMode: true` applies the dark-theme color filter — this is NOT a re-coloring of elements, it's a global filter inversion. Reproducing it pixel-exactly requires implementing Excalidraw's `THEME_FILTER` CSS in your renderer.
- Frames clip their children when exported with `exportingFrame`.

### `roughness` modes

- `roughness: 0` — "architect" mode. Near-clean strokes, minimal jitter.
- `roughness: 1` — default sketchy look.
- `roughness: 2` — "cartoonist". Heavy jitter.

`roughr` accepts the same `roughness` parameter on `OptionsBuilder`.

### Z-order, groups, frames during export

- Element order in the `elements` array defines z-order (later = on top), with `index` (fractional indexing) used during collaborative editing to keep ordering stable.
- `groupIds` is purely organizational at render time — groups don't change drawing order.
- `frame` elements clip child elements (those with `frameId` matching the frame's `id`) when exporting that frame specifically.

### PNG metadata stripping breaks round-tripping

Any image optimizer (`oxipng -strip all`, `pngquant`, ImageOptim) that removes ancillary chunks **destroys the embedded scene** and the file becomes a plain image, not re-importable. There is also an active regression ([\#9540](https://github.com/excalidraw/excalidraw/issues/9540)) where even unstripped PNGs sometimes fail to re-open with the embedded scene. The `tEXt` → `iTXt` migration ([\#9269](https://github.com/excalidraw/excalidraw/issues/9269)) is also in flight — readers should accept both keywords (`excalidraw` and `application/vnd.excalidraw+json`) and both chunk types.

### Self-hosted asset paths

`exportToSvg()` historically did not respect `window.EXCALIDRAW_ASSET_PATH` ([\#5063](https://github.com/excalidraw/excalidraw/issues/5063), [\#7543](https://github.com/excalidraw/excalidraw/issues/7543)), so SVGs exported from a self-hosted Excalidraw still reference `excalidraw.com` fonts — another reason to either embed fonts or convert to paths in Darkmatter's pipeline.

## References

- [Excalidraw repository](https://github.com/excalidraw/excalidraw)
- [Excalidraw JSON schema docs](https://docs.excalidraw.com/docs/codebase/json-schema)
- [Export utilities API](https://docs.excalidraw.com/docs/@excalidraw/excalidraw/api/utils/export)
- [Scene serialization / embedding (DeepWiki)](https://deepwiki.com/excalidraw/excalidraw/6.2-json-serialization)
- [Element types source](https://github.com/excalidraw/excalidraw/blob/master/packages/element/src/types.ts)
- [Rough.js](https://roughjs.com/) / [rough-stuff/rough](https://github.com/rough-stuff/rough)
- [orhanbalci/rough-rs (roughr, rough_tiny_skia, rough_piet, rough_plotters_svg, rough_iced, rough_vello)](https://github.com/orhanbalci/rough-rs)
- [`roughr` on crates.io](https://crates.io/crates/roughr) / [docs.rs](https://docs.rs/roughr/latest/roughr/)
- [linebender/resvg](https://github.com/linebender/resvg) / [linebender/tiny-skia](https://github.com/linebender/tiny-skia)
- [Excalifont](https://plus.excalidraw.com/excalifont) / [Virgil (archived)](https://github.com/excalidraw/virgil)
- [Timmmm/excalidraw_export](https://github.com/Timmmm/excalidraw_export)
- [excalidraw-brute-export-cli](https://github.com/realazthat/excalidraw-brute-export-cli)
- [@tommywalkie/excalidraw-cli](https://github.com/tommywalkie/excalidraw-cli)
- Issues: [\#70 seed](https://github.com/excalidraw/excalidraw/issues/70), [\#211 stable strokes](https://github.com/excalidraw/excalidraw/issues/211), [\#1972 outline text](https://github.com/excalidraw/excalidraw/issues/1972), [\#2192 embed fonts](https://github.com/excalidraw/excalidraw/issues/2192), [\#9269 iTXt migration](https://github.com/excalidraw/excalidraw/issues/9269), [\#9540 embed-scene regression](https://github.com/excalidraw/excalidraw/issues/9540), [PR #9670 binding modes](https://github.com/excalidraw/excalidraw/pull/9670)
