---
last_updated: 2026-07-13
blast_radius:
  - darkmatter/lib/src/render/link.rs
  - renderable/src/tree/render/browser.rs
  - renderable/src/browser/feature.rs
---

# Prompted Links (Popover)

A **prompted link** is an ordinary navigable link that also carries a short
prompt — a hint the reader can reveal without leaving the page. Darkmatter
lowers the prompt to browser markup as a CSS-only *popover* that appears on
hover or keyboard focus and degrades to a plain link when the enhancement is
unavailable.

Authors attach a prompt with the structured-link `prompt=` metadata (see the
[`Link` struct](../structs/Link.md)):

```md
[Docs](https://example.com prompt='Open the documentation site')
```

## Design (v1)

Prompted links use **native HTML/CSS behavior with no custom JavaScript**. The
CSS is injected once per page through the `PageFeature::Popover` feature (the
render-tree browser writer requests it whenever a link carries a lowered
`data-prompt`; `DefaultFeatureResolver` supplies the shared stylesheet). Markup,
accessibility attributes, and unique-ID allocation are the renderer's
responsibility — the resolver only owns the CSS.

The chosen approach is the spec's **progressive-enhancement** design: preserve
the anchor and its real `href`, wrap the anchor and prompt in a stable
container, expose the prompt through `:hover` and `:focus-within`, and retain
`interestfor` plus `popover="hint"` as an enhancement where the browser
supports the interest/popover invokers.

## Emitted markup contract

A prompted link lowers to the following structure (the internal `data-prompt`
transport attribute is consumed and never re-emitted; every other existing link
attribute and the real `href`/`title` are preserved):

```html
<span class="dm-popover-wrapper">
  <a href="…" {existing attrs} interestfor="ID" aria-describedby="ID">Docs</a>
  <span id="ID" class="dm-popover-prompt" popover="hint" role="note">Open the documentation site</span>
</span>
```

- **`href` preserved** — the anchor is the primary control and always
  navigates on click/Enter.
- **Escaped prompt** — the prompt content is HTML-escaped, so hostile
  characters cannot break out of the markup.
- **Association** — `aria-describedby` is the always-on accessible association;
  `interestfor` is the progressive-enhancement invoker where supported. Both
  name the same document-unique `id`.

### Keyboard and progressive fallback

- The wrapper reveals the prompt on `:hover` **and** `:focus-within`, so tabbing
  to the anchor shows the prompt with no JavaScript.
- The prompt sets an explicit `display:block` so a popover-supporting UA's
  `[popover]{display:none}` rule cannot defeat the CSS-only `:hover`/
  `:focus-within` fallback (author rules beat the UA sheet). Visibility is
  governed by the injected stylesheet everywhere.
- A `@media (prefers-reduced-motion: reduce)` rule disables the fade transition.
- Colors use the shared `--color-bg` / `--color-fg` / `--color-border` semantic
  tokens with dark-mode-safe literal fallbacks; a fixed `max-width` plus
  `left:0; right:auto` keeps the panel inside the viewport for inline links.

### Unique IDs

The render is given a document-scoped deterministic ID allocator shared by the
fragment and streaming writers. The base is a readable slug derived from the
link target; the first occurrence uses it verbatim and later occurrences append
`-N`. Because both writers walk links in document order, they derive identical
id sequences — so **multiple identical prompted links never generate duplicate
IDs**, and the two browser paths stay byte-identical.

## Standalone helper

`Link::to_html_with_popover()` (and its `to_browser_with_popover()` alias)
produces the same canonical wrapper/anchor/prompt structure for a caller
rendering a single `Link` outside the document pipeline. It returns
`Option<String>` (`None` when the link carries no prompt) and deliberately
mirrors the production render-tree markup so the two never diverge. The shared
Popover CSS is still supplied separately by the feature resolver.

## Target support

Feature assets are a **browser-only** concern. Prompted links receive no feature
assets on any other target:

| Target       | Behavior                                                            |
|--------------|--------------------------------------------------------------------|
| Browser      | Full wrapper/anchor/prompt markup + injected Popover CSS (once).   |
| Terminal     | Ordinary link (OSC 8); no prompt markup, no CSS.                   |
| Markdown     | Ordinary Markdown link; the prompt rides link metadata only.      |
| MarkdownPlus | Same as Markdown — features never alter Markdown-family output.   |

A page with no prompted link gets no Popover CSS at all.

## Cross-browser verification

Chromium behavior is verified by an automated headless-Chrome test
(`darkmatter/lib/tests/browser_render.rs`) that asserts against *computed*
styles and live focus (the prompt's `display` is `block`, it is hidden by
default and becomes visible on keyboard focus, and the anchor keeps its `href`
and `aria-describedby` association). The CSS-only fallback is engine-portable;
Firefox and WebKit are verified with the manual checklist below (no automation
harness for those engines exists in the repo):

1. Render a doc with a `prompt='…'` structured link to HTML
   (`md compose … | md render --html`, or `Markdown::as_html`).
2. Hover the link — the prompt appears. Tab to the link — the prompt appears
   (`:focus-within`). Click/Enter still navigates. With reduced motion enabled,
   the fade is disabled.
3. Confirm the prompt is not clipped at a viewport edge for a long wrapping
   prompt.
4. Confirm no network request is made — the feature is inline CSS only.
