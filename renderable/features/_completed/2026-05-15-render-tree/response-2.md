# Render Tree Spec — Response to Second-Pass Review

**Date:** 2026-05-16
**Reviewed document:** [`feeback-2.md`](./feeback-2.md)
**Revised document:** [`spec.md`](./spec.md)
**Prior round:** [`response.md`](./response.md)

Per-item decisioning for the second review. The second pass targeted API-shape
inconsistencies rather than architecture, so the changes are local but
load-bearing.

Summary: **all 11 findings accepted.** Two were resolved with a design choice
different from the reviewer's suggestion (items 6, 7); the rest were taken as
proposed.

---

## Item 1 — Renderer return types are internally inconsistent

**Verdict:** Agree — a real contradiction.

The spec showed `render_markdown(...) -> Rendered<String>` in one place,
`Result<Rendered<T>, RenderError>` in another, and a component example that
used `.output` directly (which only compiles against the non-`Result` form).

**Changes:** `Result<Rendered<T>, RenderError>` is now the **single canonical
signature**, stated explicitly as authoritative. Every renderer signature and
every example obeys it. Infallible-trait adapter policy is handled under item 2.

---

## Item 2 — The one-line target-trait delegation claim is too optimistic

**Verdict:** Agree — the example did not compile.

`BrowserRenderable::render_html_fragment` is infallible and also requires
`as_any`; `TerminalRenderable` requires `layout` / `layout_mut` / `as_any` and
owns layout state a bare `RenderNode` cannot supply.

**Changes:** "Component integration" rewritten. The "one-line" claim is dropped;
the honest claim is "cheap and uniform, not literally one line." Per-target
reality is spelled out: Markdown is friction-free; `BrowserRenderable` needs a
documented **error policy** (render `Warn`/`Lossy`, fallback fragment on
diagnostics; `Strict` is unreachable through an infallible trait);
`TerminalRenderable` needs layout ownership. A reusable `TreeComponent<T>`
adapter carries the boilerplate — placed in **`biscuit-terminal`**, since it
carries the `TerminalRenderable` impl and the layout type, consistent with the
Terminal renderer's placement.

---

## Item 3 — `PageOptions` / `MarkdownOptions` are the wrong renderer option types

**Verdict:** Agree.

`PageOptions` is page-assembly config; `MarkdownOptions` predates the strictness
model and has no dialect/loss policy.

**Changes:** dedicated `MarkdownRenderOptions` (`dialect`, `strictness`, `style`)
and `BrowserRenderOptions` (`strictness`, `raw_html`, optional `page`)
introduced; `biscuit-terminal` gets `TerminalRenderOptions`. The old types are
embedded only where genuinely needed (`PageOptions` inside `BrowserRenderOptions`
for full-page assembly).

---

## Item 4 — `Document` is introduced, but renderers only accept `RenderNode`

**Verdict:** Agree.

Renderers taking only `&RenderNode` would silently drop frontmatter, the source
registry, and document metadata.

**Changes:** renderers now come in **two layers** — `render_*_node(&RenderNode,
…)` and `render_*_document(&Document, …)`. Document-level functions call the
node-level ones internally but let metadata reach the renderer. Browser's
document-level function returns `HtmlPage` (vs. `BrowserFragment` at node
level), matching the page/fragment distinction.

---

## Item 5 — `SourceSpan` cannot represent synthetic nodes

**Verdict:** Agree — strongly. The earlier shape forced synthetic nodes to
invent a fake `SourceId` and `0..0` range.

**Changes:** `SourceSpan` split into always-present `provenance` and an
**optional** `location: Option<SourceLocation>`, where `SourceLocation` holds
`source` + `bytes`. Synthetic/generated nodes carry provenance with
`location: None` — no fake data. `RenderNode.span` is consequently no longer
`Option` (every node has a provenance); builders set
`SourceSpan { provenance: Synthetic, location: None }`.

---

## Item 6 — `SourceId` is too central to leave open

**Verdict:** Agree the decision must be made now; **registry placement adopted,
representation resolved a step further.**

The reviewer proposed `Document { sources: SourceRegistry, … }`. Adopted. I also
closed the representation question rather than leaving it open: `SourceId` is a
**small interned handle** into that registry — not an `Arc<Path>` or origin enum
embedded per node. Rationale: keeps every node cheap and uniform, and a
serialized `Document` stays self-contained because the registry travels with it.
What remains genuinely open is narrower — the contents of a `SourceDescriptor`
(component-origin naming, path absoluteness) — and is now the open question in
its place.

**Changes:** `Document` gains `sources: SourceRegistry`; `SourceSpan`/
`SourceLocation` carry the handle; "Resolved" records the `SourceId` decision;
serialization fixtures must include the registry.

---

## Item 7 — `NodeAttrs::style: CssStyle` overfits browser semantics

**Verdict:** Agree on the problem; **different solution than the suggested
`InlineSemanticStyle` enum.**

Carrying `CssStyle` in the core tree forces every non-browser target to
downsample CSS and biases the model toward the browser.

The reviewer offered two paths (a small semantic-style enum, or keep `CssStyle`
with a documented downsampling contract). I took a third: **drop the typed
`style` field entirely.** `NodeAttrs` already has `classes`; a separate
`InlineSemanticStyle` enum would create *two* styling channels (typed enum +
string classes) for one concern. Instead, styling intent in the core is
**semantic, carried solely by `classes`** — a documented vocabulary (`mark`,
`dim`, `sup`, `sub`, …) each renderer interprets its own way; browsers also map
unknown classes to CSS classes, terminals ignore them. Raw CSS, when a
browser-specific component truly needs it, lives in namespaced `attrs.data`
(`renderable.target.browser.style`).

**Changes:** `style` field removed from `NodeAttrs`; rationale added; the `Span`
doc comment and `ThematicBreak` rationale updated to reference `classes` /
`attrs.data` only.

---

## Item 8 — `data: BTreeMap<String, String>` is a weak carrier

**Verdict:** Agree.

Stringly-typed extension data does not round-trip structured metadata (link
`target`/`rel`, HR width/color) without ad-hoc parsing in every renderer.

**Changes:** `data` is now `BTreeMap<String, serde_json::Value>` with
**namespaced keys** (`darkmatter.hr.width`, `darkmatter.link.target`,
`renderable.target.browser.style`). It stays the escape hatch for
*not-yet-promoted* fields; the spec states that a load-bearing construct gets
**promoted to a typed field** in a follow-up rather than living in `data`
forever. (Known link metadata was not promoted *now* — that would re-inflate the
`Link` variant the first review deliberately kept lean; promotion is gated on
parity testing.)

---

## Item 9 — Suspicious parser-inventory mappings

**Verdict:** Agree.

**Changes:** new "Inventory notes" subsection:

- **`TableHead`** has no `NodeKind`; it folds to an ordinary `TableRow` placed
  first under `Table`. Renderers and the validator treat `children[0]` as the
  header row — a documented positional convention (MDAST-aligned). The table row
  was split so `TableHead` has its own line.
- **`TaskListMarker`** with no enclosing `ListItem` is malformed: the fold drops
  it **with a diagnostic**, never silently.
- **`MetadataBlock`** is stored verbatim into `DocumentMetadata.frontmatter`
  with its detected format; the fold does not parse YAML/TOML/JSON.
- **Math / definition lists**, if `Unsupported`, must have fixtures asserting a
  `Strict`-mode diagnostic.

---

## Item 10 — Validation needs a mode and should return `Result`

**Verdict:** Agree.

**Changes:** validation findings carry a `Severity` (`Error` / `Warning`);
`validate(node, ValidationMode)` takes a `Full` / `FailFast` mode; `ensure_valid`
returns `Result<(), ValidationError>`. **Renderer policy is now explicit:** each
`render_*` calls `ensure_valid` internally and a structural `Error` fails
rendering *regardless of strictness*; `Warning`s follow the strictness model.

---

## Item 11 — Serialization fixtures need diagnostics + source registry

**Verdict:** Agree.

**Changes:** the serialization fixture requirement now covers the **whole public
JSON surface** — every `NodeKind`; `SourceSpan` across all four `Provenance`
variants plus `SourceLocation`; `NodeAttrs`; `Document` including
`SourceRegistry` and `DocumentMetadata`; `Diagnostic`; and `Unsupported` nodes.
The Serialization section names the same surface so the two agree.

---

## Quick Improvement Pass

All ten items map onto the changes above:

1. Consistent `Result<Rendered<T>, RenderError>` — item 1.
2. Adapter policy for infallible target traits — item 2 (`TreeComponent<T>`).
3. `MarkdownRenderOptions` / `BrowserRenderOptions` — item 3.
4. Document-level render functions — item 4.
5. `SourceSpan` redesign — item 5.
6. `Document` owns `SourceRegistry` — item 6.
7. `CssStyle` clarified — item 7 (removed from core, not "documented
   downsampling").
8. Namespaced/structured `data` — item 8.
9. Inventory tightened — item 9.
10. Validation severities + renderer policy — item 10.

## Net effect

This round changed no architecture — it closed API-shape gaps the first
revision left ambiguous. The spec now states one canonical renderer signature,
an honest (not overstated) component-integration story, a `SourceSpan` that does
not lie about synthetic nodes, and a fully specified serialization surface.
Implementation scope is unchanged; the milestones still hold.

Two decisions went a step beyond the suggestions: `SourceId` representation was
*resolved* (interned handle) rather than left open, and `NodeAttrs` styling was
*simplified* to a single semantic `classes` channel rather than gaining a second
typed-enum channel.
