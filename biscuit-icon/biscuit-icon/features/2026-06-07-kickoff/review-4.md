---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 4 fixes the Iconify collections response shape and CSV completion
behavior, and the current L1, lint, and L2 recipes pass. The feature is not ready
for production because image rendering still lacks valid Level 2 verification
and the default-off `image` feature contract is not implemented.

## Findings

### 1. High: The image Level 2 test can pass without rendering an image

The WezTerm branch treats any increase in occupied pane rows as proof of image
rendering. Running `icon icons apple` necessarily adds rows for the echoed
command, prompt, and printed identifier, so `observed_delta > 0` can succeed when
the CLI emitted only text. The iteration-4 L2 run passed on the available WezTerm
backend, but this assertion does not establish that WezTerm displayed graphics.

Evidence:

- `cli/tests/level2_terminal.rs:215-235` accepts any pane-row increase.
- `cli/tests/level2_terminal.rs:219-226` runs a command that itself produces
  visible text.
- The CLI path under test uses `TerminalRenderable`, while no L2 test exercises
  image rendering through `Icon::render_tree()`.

Use a sentinel-based geometry assertion that isolates rows occupied by the image,
or query terminal image/cell metadata where supported. Add a separate L2 test
that folds `Icon::render_tree()` inside the real terminal. Until both paths have
effective L2 coverage, the image requirement has a test-level mismatch.

### 2. High: The default-off `image` feature does not gate tree-path image rendering

The `image` feature is empty and the render-tree payload always includes SVG.
The shared terminal renderer rasterizes that SVG whenever image support is
advertised, even when `biscuit-icon` was compiled with default features. In
addition, `biscuit-terminal` pulls `resvg` unconditionally, so the feature does
not provide the dependency-cost boundary specified by the design.

Evidence:

- `lib/Cargo.toml:10-12` declares `image = []`.
- `lib/src/render.rs:33-39` always serializes `self.svg()`.
- `biscuit-terminal/lib/src/render_tree/render.rs:1501-1505` unconditionally
  attempts SVG image rendering.
- `biscuit-terminal/lib/Cargo.toml:44-48` makes `resvg` unconditional.
- `spec.md:105-109` and `spec.md:267-273` require image rendering and `resvg`
  only when the feature is enabled.

Gate the SVG payload and rasterization capability behind the feature, and make
the relevant terminal image dependencies optional, or revise the specification
before release.

### 3. Medium: Offline set listing is not deduplicated by prefix

`offline_sets` documents prefix deduplication but uses `BTreeSet<SetInfo>`.
`SetInfo` ordering includes title and license, so a built-in placeholder and a
cached metadata row with the same prefix are distinct and both reach offline
output. The current test uses a cached prefix that does not overlap a built-in
prefix and cannot detect this.

Evidence:

- `lib/src/catalog.rs:33-62` inserts full `SetInfo` values into a `BTreeSet`.
- `lib/tests/catalog.rs:35-48` caches only the non-overlapping `lucide` prefix.

Use a map keyed by prefix, with cached metadata replacing built-in placeholders,
and test an overlapping prefix such as `ic`.

### 4. Medium: Non-zero Iconify view boxes are discarded

The client models only width and height, while the cache omits the specified
`view_box` column. Any Iconify entry with non-zero `left` or `top` coordinates is
therefore reconstructed as `0 0 width height`, which can shift or clip artwork.

Evidence:

- `lib/src/iconify/client.rs:35-51` does not deserialize origin coordinates.
- `lib/src/iconify/client.rs:120-123` constructs `IconBody` from dimensions only.
- `lib/src/cache/store.rs:47-55` stores no view box.
- `spec.md:229-239` requires persisted view-box data.

Represent and persist the complete view box, add a matching schema migration,
and test a non-zero-origin API fixture through fetch, cache, and SVG assembly.

### 5. Medium: Set attribution metadata is still partially discarded

The client now correctly parses license title, SPDX identifier, and URL, but the
CLI reduces that object to SPDX alone before caching. This does not preserve the
attribution metadata required by the design when SPDX is absent or when title
and URL carry necessary attribution.

Evidence:

- `lib/src/iconify/client.rs:53-62` models all three fields.
- `cli/src/commands.rs:109-116` persists only `license.spdx`.

Persist a structured representation or dedicated columns for the complete
license metadata and add a round-trip test.

### 6. Low: Several styling branches remain untested

SVG assembly has direct tests for horizontal flip and 90-degree rotation only.
Vertical/both flips and 180/270-degree rotations have distinct transform math,
and Markdown has no test for the specified text fallback when raw HTML is
disallowed.

Evidence:

- `lib/src/style.rs:189-209` covers only `Rotate::R90` and
  `Flip::Horizontal`.
- `lib/src/render.rs:70-79` covers only MarkdownPlus inline SVG.

Add focused L1 assertions for every transform variant and a CommonMark/raw-HTML
denied fallback test.

## Verification Matrix

| User-facing requirement | Strongest verification | Assessment |
|---|---:|---|
| Embedded enum/string lookup | Level 1 | Appropriate |
| Local SVG styling and escaping | Level 1 | Appropriate level; transform branches remain uncovered |
| Cache-first network lookup | Level 1 | Appropriate |
| Online set listing response | Level 1 wiremock | Appropriate; persisted attribution is incomplete |
| Browser inline SVG | Level 1 | Appropriate |
| Markdown inline SVG | Level 1 | Appropriate; non-HTML fallback is untested |
| Terminal Unicode/Nerd Font/text via CLI | Level 2 | Appropriate |
| Terminal ladder through shared render tree | Level 1 | Glyph/text covered; image requires Level 2 |
| Image-protocol fallback | Level 2 attempted | **Level mismatch: WezTerm assertion permits text-only false positives** |
| Visually rendered icon/name output | Level 2 | Appropriate for non-image tiers |
| `--from` filtering and CSV completion | Level 1 | Appropriate |
| Offline built-in/cache icon listings | Level 1 | Appropriate |
| Offline set listings | Level 1 | Broken same-prefix deduplication is untested |
| Styled CLI errors | Level 2 | Appropriate |
| OS keyboard behavior | Not applicable | No Level 3 requirement |

## Validation

- `just test`: passed 79 library tests and 17 CLI tests.
- `just lint`: passed for both packages.
- `just test-l2`: reported 6 passes and 12 skips. The image test passed on the
  available WezTerm path, but its assertion is not sufficient evidence of image
  rendering.
