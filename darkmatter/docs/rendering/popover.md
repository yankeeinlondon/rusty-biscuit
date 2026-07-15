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
- The light panel uses the shared `--color-bg` / `--color-fg` / `--color-border`
  semantic tokens (with light literal fallbacks); a `@media
  (prefers-color-scheme: dark)` rule switches to a dark palette so the tooltip
  stays legible on an OS-dark page.
- **Viewport safety.** The base rule left-anchors the panel (`left:0`) and lets
  it grow to `max-content`, which alone would overflow when the link sits near
  the **right** edge. Two guards prevent that: a
  `max-width:min(20rem, calc(100vw - 1rem))` cap so the panel is never wider
  than the viewport (long prompts wrap), and a `@supports
  (position-try-fallbacks: flip-inline)` block that re-anchors the panel to the
  link with CSS anchor positioning and flips it to the opposite side when it
  would overflow — keeping the bounding box on-screen at either edge. Engines
  without anchor positioning keep the `max-width`-capped, left-anchored layout.

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

**Chromium is the only automatically-verified engine.** Automated
headless-Chrome tests (`darkmatter/lib/tests/browser_render.rs`) assert against
*computed* styles, live geometry, and browser-dispatched input:

- `browser_prompted_link_popover_reveals_on_focus` — `display:block`, hidden by
  default, revealed on keyboard focus, `href` and `aria-describedby` preserved.
- `browser_popover_stays_within_viewport_{right,left}_edge` — the revealed panel
  stays on-screen when the link is pinned to either viewport edge (validates the
  anchor-positioning flip).
- `browser_popover_long_prompt_wraps_within_viewport` — a long prompt wraps
  inside the `max-width` cap with no horizontal overflow.
- `browser_popover_color_modes_differ` — the panel's applied colors differ
  between an emulated dark and light `prefers-color-scheme`.
- `browser_popover_reduced_motion_suppresses_transition` — the transition
  collapses to `0s` under `prefers-reduced-motion: reduce`.
- `browser_popover_{tab_reaches_anchor,enter_activates_link,pointer_hover_reveals_prompt}`
  — CDP input proves Tab focus reveals the prompt, Enter navigation, and pointer
  hover while Chrome remains headless and isolated from the host OS input state.

**Firefox and WebKit are NOT part of the automated verification set** — the repo
has no automation harness for those engines, and CSS anchor positioning (which
powers the edge-flip) is Chromium-only at the time of writing. On Firefox/WebKit
the panel falls back to the `max-width`-capped, left-anchored layout. Ordinary
inline links stay on-screen and long prompts wrap inside the cap, but a link
positioned very near the **right** viewport edge can have its panel's right edge
overflow. This is an **accepted, documented v1 limitation, not a defect**: a
portable pure-CSS edge-flip is not achievable without CSS anchor positioning or
JavaScript, and JavaScript is out of v1 scope (the popover is CSS-only). A future
vendored or JS-enhanced path could close the gap. The spec's
[Popover Feature](../../features/2026-07-09-features/spec.md) section records the
same supported-engine contract. Treat the manual checklist below as unverified
guidance for those engines, not an automated support claim:

1. Render a doc with a `prompt='…'` structured link to HTML
   (`md compose … | md render --html`, or `Markdown::as_html`).
2. Hover the link — the prompt appears. Tab to the link — the prompt appears
   (`:focus-within`). Click/Enter still navigates. With reduced motion enabled,
   the fade is disabled.
3. Confirm the prompt is not clipped at a viewport edge for a long wrapping
   prompt.
4. Confirm no network request is made — the feature is inline CSS only.
