# Render Tree Spec — Response to Adversarial Review

**Date:** 2026-05-16
**Reviewed document:** [`feedback.md`](./feedback.md)
**Revised document:** [`spec.md`](./spec.md)

This records the decisioning for each feedback item: whether it was accepted,
accepted-with-a-different-solution, or declined, and what changed in the spec.

Summary: **all 15 findings accepted.** Three were resolved with a design
different from the reviewer's suggestion (items 4, 8, 9); the rest were taken
largely as proposed. The "Quick Improvement Pass" suggestions were folded into
the same changes.

---

## Item 1 — Sequencing undercuts the validation loop

**Verdict:** Agree.

The two-unit plan (`renderable` walkers first, `darkmatter` fold later) would
harden the public tree API before any real parser event had exercised it. The
fold is the part that proves the vocabulary is sufficient; deferring it defers
the actual risk.

The crate split (`renderable` owns the tree, `darkmatter` owns the fold) is
about *where code lives*, not *when it ships* — those were wrongly conflated.

**Changes:** "Scope & sequencing" rewritten around **vertical-slice
milestones**. Milestone 1 now includes a minimal fold, the Markdown renderer,
and `text → tree → Markdown` golden fixtures — an end-to-end slice through real
parser events. Milestones explicitly cross crate boundaries.

---

## Item 2 — Called a render tree, but modeled as a Markdown AST

**Verdict:** Agree, with a scoped resolution.

The node model lacked any carrier for styling/identity, the promised
`==mark==`/dim nodes, and HR attributes. Two corrections were needed: enrich the
model *and* be honest about what the tree is for.

- **Enriched:** added `NodeAttrs` (`id`, `classes`, `style`, `data`) to the
  envelope **now, not deferred**; added a `Span` variant as the styling carrier;
  `ThematicBreak` attributes ride in `attrs`.
- **Scoped honestly:** the tree is a *document-structural IR*, not a universal
  component IR. Encoding full terminal layout/capability into nodes was
  rejected — that is renderer *context* (see item 11), not node data. Components
  whose intent exceeds document structure keep direct per-target impls.

**Changes:** new "What the tree is — and is not" section; `NodeAttrs` added to
the envelope; `Span` variant added; rationale updated.

---

## Item 3 — Crate-boundary story for terminal walking is incomplete

**Verdict:** Agree — this was a genuine architectural hole.

A meaningful Terminal renderer must name `Terminal`, `Layout`, color-depth,
OSC8, and image-protocol types — all in `biscuit-terminal`, which `renderable`
cannot depend on. The earlier "generic Terminal walker in `renderable`" hid a
cycle.

**Resolution:** the Terminal renderer **moves to `biscuit-terminal`**, which
already depends on `renderable` and can therefore consume `RenderNode` freely.
`renderable` keeps the Markdown and Browser renderers (both emit types it
already owns).

**Changes:** Summary, Architecture diagram, and Crate-boundaries table updated
to a three-crate split; the rejected option is stated explicitly.

---

## Item 4 — `Visitor` default-method design risks silently losing semantics

**Verdict:** Agree on the problem; **different solution.**

Default "recurse into children" is unsafe for rendering (drops `Link` URLs,
vanishes `Image`, erases `Heading` depth). The reviewer proposed a `RenderLoss`
warning mechanism plus an exhaustiveness test.

**More elegant solution:** drop the default-method `Visitor` for render
renderers entirely and use an **exhaustive `match` on `NodeKind`**. The Rust
compiler then *is* the exhaustiveness gate — adding a variant fails compilation
in every renderer until it makes a deliberate decision. This needs no separate
warning system or hand-maintained test. A default-recursing traversal helper
may still exist, but only for *transform* passes, where recurse-by-default is
correct.

**Changes:** "Target renderers" section specifies exhaustive `match` for render
renderers and confines default-recursion to transform traversal.

---

## Item 5 — `position` is not as free or complete as claimed

**Verdict:** Agree.

A bare `Option<Position>` byte range cannot answer "which source?",
"synthetic?", or "pre- or post-compose?" — and container-node ranges are not
trivially derived from start/end events.

**Changes:** `position` replaced by a `SourceSpan { source, bytes, provenance }`
type with a `Provenance` enum (`Parsed` / `Synthetic` / `Generated` /
`Transcluded`). Container-span computation is named as a defined fold
responsibility with documented rules. The model carries source identity from
the start rather than retrofitting it later.

---

## Item 6 — "Unhandled events become no-op" is too permissive

**Verdict:** Agree.

Silent no-op drop means the new canonical path could be *less* correct than the
legacy event path while still passing smoke tests — the opposite of the
feature's purpose.

**Changes:** the fold is now "total **and loud**." Every event resolves to a
defined disposition; `Noise` (silent drop) is reserved for events *proven*
structural. Anything without a faithful node becomes a `NodeKind::Unsupported`
node plus a diagnostic. The fold returns `(Document, Vec<Diagnostic>)`.

---

## Item 7 — Node vocabulary missing Markdown constructs

**Verdict:** Agree.

`Break` conflating soft/hard was an outright bug; footnotes, HTML block-vs-
inline, task-list markers, and the custom inline styles had no representation.

**Changes:**

- `Break` split into `SoftBreak` / `HardBreak`.
- Added `FootnoteReference`, `FootnoteDefinition`.
- `Html` gained a `block: bool` field.
- `Span` covers `==mark==`, dim, superscript, subscript.
- `Unsupported` variant added for the genuinely unrepresentable.
- New "Parser event inventory" section with a disposition table — explicitly
  marked as requiring verification against the pinned `pulldown-cmark` 0.13
  event set, and made a required Milestone 1 deliverable.

---

## Item 8 — MDAST serialization asserted but not specified

**Verdict:** Agree; **claim withdrawn rather than chased.**

An envelope struct with `attrs`/`span` and a tagged `kind` does not produce
MDAST-shaped JSON, and `Table.align`/`NodeAttrs`/`SourceSpan` have no MDAST
equivalent. Rather than contort the type to chase wire-compatibility, the
honest choice is to stop claiming it.

**Changes:** new "Serialization" section. `RenderNode` serializes to **its own
documented format**; the vocabulary is "MDAST-*inspired*," not MDAST. An
MDAST-JSON adapter for `as_ast` is a separate, fixture-tested concern if
external consumers need it. Per-kind serialization fixtures are a required
test. (Earlier draft's `items`/`rows` were also renamed to `children` for
uniformity — a side benefit.)

---

## Item 9 — Component integration blurs opt-in target traits

**Verdict:** Agree; resolution close to the reviewer's `TreeRenderable`
suggestion.

Overloading `AstRenderable` left it ambiguous whether implementing it conferred
all targets. It also could not satisfy the existing target-trait contracts
(`BrowserRenderable` returns `BrowserFragment<Ready>`, etc.).

**Resolution:** introduce `TreeRenderable { fn render_tree(&self) -> RenderNode
}`, which **replaces** the placeholder `AstRenderable`. **No blanket impls** —
renderers are free functions; a component opts into a target by writing a
one-line trait impl delegating to its own tree. "For free" is now honest (the
*work* is one line) and opt-in is real (the impl is still explicit).

**Changes:** "Component integration" rewritten around `TreeRenderable` +
delegation; `AstRenderable` marked removed/replaced; the `render_ast` open
question resolved.

---

## Item 10 — `Markdown` vs `MarkdownPlus` needs a loss policy

**Verdict:** Agree.

"Plain Markdown emits no HTML" silently implied data loss for documents that
contain raw HTML or non-Markdown nodes.

**Changes:** a `Strict` / `Warn` / `Lossy` strictness model (shared with item
14) governs what happens when plain Markdown meets an `Html` node or a
non-representable `Span`: `Strict` errors, `Warn` degrades with a diagnostic,
`Lossy` degrades only for documented cases — never a silent default drop. The
round-trip goal is stated as **semantic** stability, not byte stability.

---

## Item 11 — Target walkers need option and context types

**Verdict:** Agree.

A visitor trait without the rendering state is only abstractly specified.

**Changes:** "Target renderers" now specifies each renderer as a function
taking a target-specific options/context type — reusing existing `MarkdownOptions`
and `PageOptions`, and a new `TerminalRenderContext` (width, color depth,
hyperlink mode, image protocols, layout, theme) in `biscuit-terminal`.

---

## Item 12 — Tree transforms motivate the design but aren't designed enough

**Verdict:** Agree in part — accept the *constraint*, decline the *scope*.

Designing the full `compose/` transform pipeline now would over-build ahead of
a feature the spec explicitly defers. But the reviewer's underlying point
holds: the *node model* must not preclude transforms.

**Resolution:** the node model **reserves the hooks** transforms need —
`SourceSpan` provenance (transclusion/generated origin), `NodeAttrs` (`id` for
TOC, `data` for link resolution), and a document-level `DocumentMetadata`
separate from `Root` children (frontmatter/interpolation). The transform
*pipeline* (pass ordering, mutation API, diagnostics) stays a deferred feature,
now stated as such with the reserved hooks named.

**Changes:** `Document { metadata, root }` wrapper added; non-goals and
"deferred" list name the reserved hooks explicitly.

---

## Item 13 — Structural validity deferred, but invalid trees are easy to make

**Verdict:** Agree.

Once components splice subtrees into parsed documents, invalid trees are a live
hazard, not a hypothetical.

**Changes:** new "Validation & builders" section. `validate(&RenderNode) ->
ValidationReport` is **part of the spine** (Milestone 1), not "later if needed."
Builder constructors (`RenderNode::root`, `::paragraph`, `::heading`, …) make
valid construction the easy path. The type-level inline/block split remains a
non-goal — `validate` covers structural correctness instead.

---

## Item 14 — Error handling missing from the public shape

**Verdict:** Agree.

Infallible `String`-returning renderers force silent degradation or panics;
real rendering is fallible (missing highlight languages, unsupported nodes,
invalid spans).

**Changes:** renderers return `Result<Rendered<T>, RenderError>`, where
`Rendered<T>` carries `output` plus `Vec<Diagnostic>`. Combined with the item-10
strictness model: `Strict` turns diagnostics into `Err`; `Warn`/`Lossy` return
`Ok` with diagnostics attached. Loss and unsupported features are always
visible.

---

## Item 15 — Memory risk is broader than "documents are small"

**Verdict:** Agree.

Component subtrees, transcluded documents, generated output, and owned-string
copies all amplify the owned-tree cost.

**Changes:** the Risks section now frames memory beyond document size, and
"Testing & parity gates" requires benchmarks over large/pathological fixtures
(large code blocks and tables, deep nesting, many links/images, transcluded and
repeated content).

---

## Quick Improvement Pass

All seven suggestions were absorbed into the changes above:

- **Vertical-slice first milestone** — adopted (item 1).
- **Split the concepts / `TreeRenderable`** — adopted (item 9). Renderers are
  named functions rather than the suggested `*TreeRenderer` structs; the
  free-function form composes more directly with the one-line delegation
  pattern.
- **Annotation & provenance model sooner** — adopted (`NodeAttrs` +
  `SourceSpan`, items 2 and 5).
- **Inventory every parser event** — adopted as a required, verification-gated
  section (items 6, 7).
- **Loss/strictness policies per target** — adopted (`Strict`/`Warn`/`Lossy`,
  items 10, 14).
- **Rework visitor defaults** — adopted via exhaustive `match` rather than a
  warning mechanism (item 4).
- **Terminal rendering at the right layer** — adopted; Terminal renderer moved
  to `biscuit-terminal` (item 3).
- **Validation and builder APIs** — adopted (item 13).
- **Specify serde with fixtures / or rename the claim** — took the rename:
  "MDAST-inspired," own format, fixtures required (item 8).
- **Performance and parity gates** — adopted (items 14, 15; "Testing & parity
  gates").

## Net effect on scope

The revision makes the spec **more honest, not larger in implementation
scope**. Milestone 1 is still a thin slice; the added rigor is in *decisions*
(strictness, provenance, crate boundaries, exhaustiveness) rather than
speculative code. The transform pipeline, darkmatter renderer migration, and
MDAST adapter remain explicitly deferred — the node model just no longer
*precludes* them.
