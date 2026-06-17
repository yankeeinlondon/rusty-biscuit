---
status: ready for planning and implementation
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 7-of-7
depends-on: spec-1.md (sub-spec #1), spec-2.md (sub-spec #2), spec-3.md (sub-spec #3), spec-4.md (sub-spec #4), spec-5.md (sub-spec #5), spec-6.md (sub-spec #6)
reviewed: true
---

# `style:` Frontmatter - Sub-Spec #7: Bespoke Knobs

## Problem

After sub-specs #1-#6 ship, the schema is fully wired except for the bespoke
keys: fields that do not simply lower a `CommonStyle` bucket onto the existing
page/component layout and color maps. The remaining unwired keys are:

- `style.page.stylesheet` - external CSS pointer for HTML output.
- `style.page.meta` - page-level HTML metadata.
- `style.page.code.theme` - page-level code-block theme default.
- `style.hyperlinks.{width,max-width,alignment}` - link text layout/style.
- `style.hyperlinks.local-style.*` - overrides for local hyperlinks.
- `style.images.local-style.*` - overrides for local image references.

`style.hyperlinks.{color,bg-color}` are parsed as sub-spec #5 color keys, but
they intentionally remain visually inactive until this sub-spec because there
was no hyperlink render path for the color maps to target.

## Goals

- Wire every remaining v1 schema key to a rendering path or to a documented
  validation error.
- Keep CLI precedence from sub-spec #2: invocation-level CLI flags win over
  document frontmatter.
- Keep render-time I/O explicit and bounded. Local stylesheets may be read from
  disk; remote stylesheets must not be fetched by the renderer.
- Apply hyperlink and image `local-style` only to local references.
- Suppress `KnownButInactive { sub_spec: 7 }` warnings for every key wired in
  this sub-spec.
- After this sub-spec, no valid v1 schema key should emit
  `KnownButInactive`.

## Non-Goals

- No new v1 schema fields.
- No new CLI flags.
- No network fetching.
- No new color, length, or layout primitives.
- No Markdown output styling for links/images beyond the existing lossless
  metadata behavior. Portable Markdown remains structurally equivalent.
- No graph-wide propagation of style from parent documents into child
  documents.

## Dependencies

- Sub-spec #1 (schema/parser and structured warnings).
- Sub-spec #2 (page application, CLI precedence, strict style, active wiring).
- Sub-spec #3 (component layout wiring).
- Sub-spec #4 (list split and list layout wiring).
- Sub-spec #5 (color/bg-color storage and lowering).
- Sub-spec #6 (HR migration and `PageComponent::Hr`).

## Design Decisions

1. **Advance the active wiring phase to `7` only after all keys here are
   wired.** Do not mutate `SchemaLeaf::sub_spec`; that field remains roadmap
   metadata.
2. **Use existing render types where possible.** `Link` and `ImageRef` already
   carry typed `CssStyle` and render inline HTML `style` attributes. Extend
   those existing paths rather than adding a new global selector-only system.
3. **CSS length support is target-specific.** `CommonStyle.width` and
   `max-width` values on hyperlinks and image `local-style` may be
   `Length::Css` for HTML because inline CSS can represent it. Terminal
   lowering rejects `Length::Css` with a `StyleApplyError` if that value would
   affect terminal layout. This is intentionally narrower than page/component
   fill, where CSS lengths are invalid for all targets today.
4. **Width and max-width conflict for terminal link/image layout.** If both are
   present in the same `hyperlinks`, `hyperlinks.local-style`, or
   `images.local-style` bucket, return `StyleApplyError` before rendering,
   matching the single-fill-slot precedent from sub-specs #3, #4, and #6. HTML
   may represent both, but accepting the same document for one target and
   rejecting it for another would make `md` behavior hard to predict. Use one
   of the two fields in v1.
5. **Remote stylesheet support means link emission, not fetching.** HTTP(S)
   values for `style.page.stylesheet` are accepted and emitted as
   `<link rel="stylesheet" href="...">` in HTML output. The renderer never
   downloads remote CSS. Local paths are read from disk and inlined into a
   `<style data-darkmatter-source="...">` block so the default artifact remains
   self-contained without adding an `--externalize-css` flag.
6. **`style.page.meta` remains an open object at the schema boundary.** The
   parser already stores `serde_json::Value`; do not replace it with a closed
   `PageMeta` struct in this phase. At apply time, accept only an object and
   convert supported values into typed `MetaTag` entries.
7. **`style.page.code.theme` parses to `ThemePair` at apply time.** Unknown
   theme names return `StyleApplyError::InvalidCodeTheme` before rendering.
   CLI `--code-theme` wins over frontmatter, because CLI flags are
   invocation-level overrides and sub-spec #2 already establishes that rule.
8. **Local reference detection uses the existing link classification rule.**
   A local hyperlink is any non-HTTP(S) `Link` (`LinkType::File`), including
   relative paths, absolute paths, anchors, and `file://` URLs. `http://` and
   `https://` URLs, including localhost, are remote. A local image is any
   primary `src`/`srcset` candidate that is not HTTP(S). Data URLs are remote
   for local-style purposes because they are self-contained resources, not file
   references.
9. **Local-style merges over bucket style field-by-field.**
   For a matching local reference, `local-style` overrides only fields it sets.
   Unset fields fall back to the outer `style.hyperlinks.*` or
   `style.images.*` values.
10. **Hyperlink color is activated in this phase.**
    Sub-spec #5 creates the common color storage, but hyperlinks were excluded
    from the component list because links are inline content rather than page
    components. This sub-spec wires `style.hyperlinks.color` and
    `style.hyperlinks.bg-color` through the same `StyleColor` lowering helpers
    used by sub-spec #5.

## Bespoke Knob Details

### `style.page.stylesheet`

**Purpose:** Add page-level CSS to HTML output.

Behavior:

- Local relative paths resolve relative to the source Markdown document's
  directory. If the caller has no source path, relative paths resolve against
  the current working directory. Absolute paths are accepted.
- Local files are read once during HTML artifact construction and inlined into
  the page head.
- HTTP(S) values are emitted as `<link rel="stylesheet" href="...">`.
- `file://` URLs are rejected in v1. They are ambiguous across platforms and
  should be written as normal local paths.
- Terminal output ignores this field after parsing and emits no warning beyond
  normal tracing at `debug` level.

Errors:

- Missing local file: `StyleApplyError::StylesheetNotFound { path }`.
- Unreadable local file: `StyleApplyError::StylesheetRead { path, source }`.
- Empty stylesheet value: `StyleApplyError::EmptyStylesheet`.
- `file://` stylesheet value: `StyleApplyError::UnsupportedStylesheetScheme`.

Reader note: the original draft proposed `--externalize-css`. This reviewed
spec removes that flag because the v1 contract already has enough surface area
and no existing CLI precedence rule for stylesheet externalization.

### `style.page.meta`

**Purpose:** Emit page-level HTML `<meta>` tags.

Accepted shape:

```yaml
style:
    page:
        meta:
            description: "Short summary"
            author: "Ken"
            keywords: ["rust", "markdown"]
            viewport: "width=device-width, initial-scale=1"
            "og:title": "Open Graph title"
            "twitter:card": "summary"
            charset: "utf-8"
```

Rules:

- `meta` must be an object. Non-object values return
  `StyleApplyError::InvalidMetaShape`.
- String, number, and boolean values lower to `content`.
- Array values are accepted only for `keywords` and are joined with `, ` after
  each element is converted to a string.
- `charset` lowers to `<meta charset="...">`.
- Keys beginning with `og:` lower to `<meta property="og:..." content="...">`.
- Every other key lowers to `<meta name="..." content="...">`.
- Duplicate keys are allowed only if the input format preserves them; JSON/YAML
  map parsing usually means the last value wins before this code sees it.
- HTML escaping must be performed by the HTML tag renderer, not by string
  concatenation in the style applicator.

### `style.page.code.theme`

**Purpose:** Override the page-level default code-block theme.

Behavior:

- Parse the string with `ThemePair::try_from`, using the same accepted names as
  `--code-theme` and `--list-themes`.
- Apply the value before rendering terminal and HTML output.
- Preserve the existing code-block contrast model: `ThemePair` resolves against
  the inverted color mode for code blocks.
- CLI `--code-theme` wins over `style.page.code.theme`.

Errors:

- Unknown theme: `StyleApplyError::InvalidCodeTheme { value }`.

### `style.hyperlinks.*`

**Purpose:** Apply style to inline hyperlinks.

Behavior:

- `color` and `bg-color` lower to terminal SGR around the link display text and
  to inline CSS on the HTML `<a>` element.
- Terminal OSC 8 sequences must wrap the already-styled display text, and SGR
  reset must close before the OSC 8 end sequence so color does not leak.
- `width`, `max-width`, and `alignment` affect terminal fallback/display text
  before OSC 8 wrapping. For HTML they lower to inline CSS declarations on
  `<a>`.
- Existing per-link inline CSS from `Link::with_style` wins over global
  frontmatter style for the same CSS property. Frontmatter fills missing
  declarations.
- Portable Markdown output keeps the existing metadata behavior. When inline
  HTML mode is active, the styled HTML anchor is emitted as it is today.

### `style.hyperlinks.local-style`

**Purpose:** Override hyperlink style for local references only.

Rules:

- Local hyperlinks are `LinkType::File`: relative paths, absolute paths,
  anchors, and `file://` URLs.
- HTTP(S), including localhost, are remote.
- `local-style` merges over `style.hyperlinks.*` field-by-field.
- The same terminal and HTML lowering rules as `style.hyperlinks.*` apply.

### `style.images.local-style`

**Purpose:** Override image style for local image references only.

Rules:

- A local image has a primary `src` or `srcset` candidate that is not HTTP(S)
  and is not a `data:` URL.
- For HTML, local style merges into the image's inline `style` attribute through
  `ImageRef::with_style`.
- For terminal fallback/OSC 8 output, `color` and `bg-color` style the alt text
  only. `width`, `max-width`, and `alignment` apply to the rendered fallback
  text, not to raster decoding or terminal image protocols.
- Existing per-image inline CSS wins over global frontmatter style for the same
  CSS property.

## Public API

```rust
// darkmatter::layout::DarkmatterPage - extended

impl DarkmatterPage {
    pub fn with_stylesheet(self, stylesheet: PageStylesheet) -> Self;
    pub fn with_page_meta(self, meta: PageMeta) -> Self;
    pub fn with_page_code_theme(self, theme: ThemePair) -> Self;
    pub fn with_hyperlink_style(self, style: CommonStyle) -> Self;
    pub fn with_local_hyperlink_style(self, style: CommonStyle) -> Self;
    pub fn with_local_image_style(self, style: CommonStyle) -> Self;
}

pub enum PageStylesheet {
    Inline { source: PathBuf, css: String },
    Remote { href: String },
}

pub struct PageMeta {
    pub tags: Vec<MetaTag>,
}

pub enum MetaTag {
    Charset(String),
    Name { name: String, content: String },
    Property { property: String, content: String },
}

// darkmatter::style - extended

pub fn apply_bespoke_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    overrides: BespokeStyleOverrides,
    source_path: Option<&Path>,
) -> Result<DarkmatterPage, StyleApplyError>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BespokeStyleOverrides {
    pub code_theme: bool,
}
```

The CLI render pipeline becomes:

```text
DarkmatterPage::new(...)
  -> apply_cli_layout_flags(...)
  -> apply_page_style(...)
  -> apply_component_style(...)
  -> apply_list_style(...)
  -> apply_color_style(...)
  -> apply_hr_style(...)
  -> apply_bespoke_style(...)
  -> render / render_to_browser
```

`BespokeStyleOverrides::code_theme` is set when `--code-theme` was supplied.
There are no stylesheet/meta CLI overrides in v1.

## Implementation Notes

- Add `style.hyperlinks.{color,bg-color}` to the final no-inactive guarantee
  even though their descriptor rows are sub-spec #5. Their visual application
  lands here because links are inline render objects, not `PageComponent`
  values.
- Reuse sub-spec #5 helpers for `StyleColor` to terminal SGR and CSS.
- Add a small helper that converts `CommonStyle` into a CSS declaration overlay:
  `width`, `max-width`, `text-align`, `color`, and `background-color`.
- When merging inline CSS, parse into `CssStyle`, preserve existing properties,
  and add only properties not already present.
- Keep diagnostics and errors in canonical kebab-case:
  `style.hyperlinks.local-style.max-width`, not
  `style.hyperlinks.local_style.max_width`.
- Update `darkmatter/docs/rendering/style.md` after implementation so #7 is
  marked live and the stylesheet/meta/code-theme behaviors are documented.

## Tests

1. **Local stylesheet inline** - `style.page.stylesheet: ./style.css` renders
   HTML with the file contents in a `<style>` tag and does not emit
   `KnownButInactive`.
2. **Remote stylesheet link** - `style.page.stylesheet:
   https://example.com/site.css` renders a `<link rel="stylesheet">` and does
   not attempt a network request.
3. **Missing stylesheet error** - missing local path returns
   `StyleApplyError::StylesheetNotFound`.
4. **Page meta** - description, keywords array, `og:title`, and `charset`
   produce the expected escaped meta tags.
5. **Invalid meta shape** - `style.page.meta: "description"` returns
   `StyleApplyError::InvalidMetaShape`.
6. **Code theme frontmatter** - `style.page.code.theme: dracula` changes
   terminal and HTML code block rendering to the Dracula `ThemePair`.
7. **CLI code theme override** - `--code-theme nord` wins over
   `style.page.code.theme: dracula`.
8. **Invalid code theme** - unknown theme name returns
   `StyleApplyError::InvalidCodeTheme`.
9. **Hyperlink color terminal** - `style.hyperlinks.color: red-500` wraps link
   display text in foreground SGR while preserving OSC 8 boundaries.
10. **Hyperlink color HTML** - the same setting renders an `<a>` with a CSS
    `color` declaration unless the link already has that property inline.
11. **Local hyperlink override** - global red plus local blue makes remote
    links red and local/anchor/file links blue.
12. **Local image override HTML** - local image refs receive merged inline CSS;
    HTTP and data images do not.
13. **Local image override terminal** - local image fallback alt text receives
    color/background styling; remote image fallback remains unchanged.
14. **Width/max-width conflict** - setting both under `hyperlinks`,
    `hyperlinks.local-style`, or `images.local-style` returns the documented
    `StyleApplyError`.
15. **Active wiring warnings** - after this sub-spec, every valid v1 style key,
    including hyperlink colors and local-style leaves, emits zero
    `KnownButInactive` warnings.

## Acceptance Criteria

- Every v1 style schema key is either visually honored or rejected with a
  documented `StyleApplyError`.
- `ACTIVE_STYLE_WIRING_SUB_SPEC` is `7`.
- `KnownButInactive` is empty for every valid key in the v1 schema.
- HTML output supports page stylesheets, page meta tags, page code theme,
  styled links, and local image style overrides.
- Terminal output supports page code theme, styled hyperlinks, and local
  hyperlink/image fallback styling without leaking SGR.
- All previous sub-spec tests still pass.

## Risks

- **Inline styling can bypass central CSS.** This is acceptable for v1 because
  `Link` and `ImageRef` already expose typed inline CSS and many references are
  produced outside a single page selector context.
- **Remote stylesheet privacy.** The renderer does not fetch remote CSS, but
  opening the generated HTML can. This is user-authored behavior and should be
  documented near `style.page.stylesheet`.
- **Terminal link styling and OSC 8 ordering.** Tests must preserve ANSI bytes;
  ANSI-stripping helpers cannot verify this behavior.
- **Meta tag scope.** The open object model is intentionally permissive. If
  callers need richer Open Graph/Twitter validation later, that should be a
  separate schema version.

## Open Questions

None. The reviewed design above makes the remaining v1 decisions.

## Out-of-Spec

This is the final sub-spec for the `style:` frontmatter v1 contract. After it
lands, a follow-up polish pass should update user-facing docs, add an
end-to-end fixture that exercises the whole style block, and produce a short
migration note for deprecated snake-case aliases and top-level HR style.
