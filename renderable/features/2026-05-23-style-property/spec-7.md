---
status: draft
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 7-of-7
depends-on: spec.md, spec-2.md..spec-6.md
---

# `style:` Frontmatter — Sub-Spec #7: Bespoke Knobs

## Problem

After sub-specs #1–#6 ship, the schema is fully wired except for the
*bespoke* keys — those that don't follow the `CommonStyle` shape and
each represent a distinct mini-feature. The remaining unwired keys are:

- `style.page.stylesheet` — external CSS pointer (HTML target only).
- `style.page.meta` — opaque map of `<meta>` tags / metadata.
- `style.page.code.theme` — code-block theme override at the page level.
- `style.hyperlinks.*` — hyperlink-specific styling (currently
  no rendering path honors it).
- `style.hyperlinks.local-style` — overrides applied only to local
  (file://) hyperlinks.
- `style.images.local-style` — overrides applied only to local image
  references.

Each of these is independent enough that we may break this sub-spec
further into per-knob mini-features if scope grows. The default plan
treats them as one phase.

## Goals

- Wire each of the bespoke keys to its respective rendering path.
- Each wiring suppresses its `KnownButInactive { sub_spec: 7 }`
  warnings.
- After this sub-spec, `KnownButInactive` should be empty for every
  valid frontmatter key in the schema.

## Non-Goals

- No new schema fields beyond what already exists in the v1 schema.
- No new color or layout primitives.

## Dependencies

- Sub-specs #1–#6 merged.

## Bespoke Knob Details

### `style.page.stylesheet`

**Purpose:** Reference an external CSS file (or HTTP URL) to apply on the
browser/HTML target.

**Decisions:**

1. **Local vs. remote.** Support both local paths (relative to the
   document) and HTTP(S) URLs.
2. **Embedding strategy.** For local files, either:
   - Inline the CSS content into the generated HTML.
   - Emit a `<link>` tag referencing the resolved file path.
   Recommended: inline (self-contained output). The exception is when
   `--externalize-css` (new CLI flag) is set, then emit `<link>`.
3. **Validation.** Do we validate the CSS is well-formed? Probably no —
   pass-through.
4. **Terminal target.** Ignored (warn at `tracing::info!` level).

### `style.page.meta`

**Purpose:** Emit `<meta>` tags in the HTML head.

**Decisions:**

1. **Shape.** `Option<serde_json::Value>` in v1. Refine to a typed
   `PageMeta { description: Option<String>, keywords: Vec<String>, ... }`?
   Or accept any string → string map and emit `<meta name="k" content="v">`?
   Recommended: typed map for the common keys (description, keywords,
   author, viewport), passthrough for others.
2. **Terminal target.** Ignored.

### `style.page.code.theme`

**Purpose:** Override the default code-block theme for the entire page.

**Decisions:**

1. **Precedence.** `style.page.code.theme: dracula` overrides `--code-theme`
   from the CLI? Probably no — CLI wins (matches the precedence rule
   from sub-spec #2). Frontmatter is the document-level default; CLI is
   the invocation-level override.
2. **Theme name validation.** Reject unknown theme names at parse time
   (the schema already accepts any `String`; validation here is at apply
   time).
3. **Interaction with sub-spec #5's `style.code-blocks.color`.** The
   color overrides the panel, the theme controls per-token highlights.
   Both coexist.

### `style.hyperlinks.*`

**Purpose:** Apply `CommonStyle` (color, bg-color, alignment, etc.) to
hyperlinks.

**Decisions:**

1. **New `PageComponent::Hyperlinks` variant?** Add one. Apply the
   common-mutation wiring from sub-specs #3 and #5 to it.
2. **Terminal rendering.** Hyperlinks today render with OSC8 hyperlink
   sequences (`\x1b]8;;url\x1b\\text\x1b]8;;\x1b\\`) plus default
   coloring. Add SGR color application via the bg-color/color settings.
3. **HTML rendering.** Emit `<a style="color: ...">` on the HTML target.

### `style.hyperlinks.local-style`

**Purpose:** Override `CommonStyle` for *local* (`file://` or relative)
hyperlinks only.

**Decisions:**

1. **What counts as "local"?** Relative paths and `file://` URLs.
   `http(s)://` to `localhost` is debatable — recommend treating as
   remote.
2. **Apply precedence.** `local-style` wins over `hyperlinks.*` for
   matching links.

### `style.images.local-style`

**Purpose:** Same as hyperlinks.local-style but for image references.

**Decisions:** Mirror the hyperlinks rule.

## Public API (Sketch)

```rust
// darkmatter::layout::DarkmatterPage — extended

impl DarkmatterPage {
    pub fn with_stylesheet(self, path_or_url: impl Into<String>) -> Self;
    pub fn with_page_meta(self, meta: PageMeta) -> Self;
    pub fn with_page_code_theme(self, theme: impl Into<String>) -> Self;
}

pub enum PageComponent {
    // ... previous variants ...
    Hyperlinks,
}

// darkmatter::style — extended

pub fn apply_bespoke_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    cli_overrides: &CliLayoutOverrides,
) -> Result<(DarkmatterPage, Vec<StyleWarning>), StyleApplyError>;
```

## Tests

1. **External stylesheet (local file)** — generated HTML contains the
   stylesheet's CSS inlined in `<style>`.
2. **External stylesheet (HTTP URL)** — generated HTML contains
   `<link rel="stylesheet" href="...">`.
3. **Page meta** — `style.page.meta.description: "..."` →
   `<meta name="description" content="...">` in HTML head.
4. **Page code theme override** — `style.page.code.theme: dracula` →
   code blocks rendered with dracula theme; `--code-theme nord`
   overrides back to nord.
5. **Hyperlink color** — `style.hyperlinks.color: red-500` → all links
   render with red SGR on terminal and `color:rgb(239,68,68)` on HTML.
6. **Local hyperlink override** — `style.hyperlinks.color: red-500` +
   `style.hyperlinks.local-style.color: blue-500` → external links red,
   local links blue.
7. **Local image override** — same for images.

## Acceptance Criteria

- Every bespoke knob honors its documented behavior on the appropriate
  target(s).
- `PageComponent::Hyperlinks` exists and is honored by sub-spec #5 color
  paths.
- All previous sub-spec tests pass.
- `KnownButInactive` is empty for every key in the v1 schema after this
  sub-spec lands.

## Risks

- **Scope creep.** Each bespoke knob is a mini-feature; the whole sub-spec
  could grow large. Be ready to split into 7a / 7b / 7c if needed.
- **Stylesheet security.** Allowing arbitrary HTTP URLs to be fetched at
  render time is a security/privacy footgun. Recommended: support local
  paths in v1, defer HTTP URLs to a sub-spec #7-followup with explicit
  fetching policy (cache, timeout, etc.).
- **Page meta taxonomy.** `<meta>` tag types are open-ended (Open Graph,
  Twitter cards, etc.). Decide which we support natively vs. via
  passthrough.

## Open Questions

1. Should HTTP stylesheet URLs ship in v1, or defer behind a follow-up
   spec with a security policy?
2. `PageMeta` typed shape vs. open map.
3. `style.page.code.theme` vs. CLI `--code-theme` precedence — confirm
   CLI wins.
4. Local-vs-remote definition for hyperlinks/images.

## Out-of-Spec

This is the final sub-spec for the `style:` frontmatter contract. After
it lands, the entire `docs/rendering/style.md` schema is operational. A
follow-up "polish & docs" pass can update the documentation, add a
comprehensive end-to-end fixture, and produce a migration guide.
