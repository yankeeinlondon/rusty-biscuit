---
agent: claude
model: claude-opus-4-7
ready: false
---

# Review: Style Primitive Spec (Prose cross-target focus)

## Summary

The spec is directionally sound. Re-homing scattered appearance (`BlockQuote`
colors, `Section` SGR, `Table` tints, `PageBackground`) onto one declarative
`Style` primitive — declared by components, applied by the tree renderers — is
the correct sibling move to `Layout`, and reusing `TargetValue<T>` and
`renderable::color` rather than inventing parallel infrastructure is right.

But this review was asked specifically about **rendering `Prose` cross-target**,
and on that axis the spec is not yet ready to become a plan. `Prose` is the
single inline-styling component in the workspace, it is *terminal-only today*
(it lives in `biscuit-terminal` and resolves its three grammars straight to
ANSI), and the spec itself names the `Style`↔`Prose` interaction as "the single
biggest unresolved interaction" — then leaves it **(OPEN)** in D6 and as Open
Question #1. Every other open decision (slots, inheritance, module home,
Markdown emission, the `Emphasis` field) depends on how that question resolves.
The draft cannot be planned until OQ#1 is closed.

The findings below are ordered by how much they block the Prose cross-target
story.

## Findings

### High — D6/OQ#1: the spec must commit to `Style` as Prose's substrate

D6 offers two options and picks neither:

> *(a)* `Style` covers block-level appearance only and `Prose` keeps inline; or
> *(b)* `Style` becomes the substrate `Prose` lowers onto.

These are not co-equal. Option (a) leaves `Prose` permanently terminal-only:
its grammars produce ANSI, never tree nodes, so the Browser and Markdown tree
renderers can never see Prose-authored emphasis or color. That directly
contradicts the `renderable` premise (one owned tree → three targets) and
recreates, for inline text, exactly the "appearance has no home in the tree
pipeline" problem this spec exists to kill. It is also the same failure mode as
`BlockQuote::render_tree()` dropping its own appearance (lessons-learned #6).

Option (b) is the only one that makes Prose cross-target: Prose's grammar
becomes a **parser that emits a subtree of inline `RenderNode`s** (`Span` /
`Emphasis` / `Strong` / `InlineCode` / link nodes) carrying `Style`, and the
three tree renderers fold it. Prose stops hand-writing ANSI; the terminal
renderer lowers `Style` to SGR in the fold (D7), the browser renderer lowers to
CSS, the Markdown renderer re-emits `**`/`_`/`~~`. This is also consistent with
how the spec already (correctly) keeps `CodeRenderer` separate: a grammar/parse
capability is distinct from the declarative `Style` it produces.

**The spec should adopt (b) explicitly**, state "make `Prose` render to all
three targets" as a goal, and treat Prose's de-ANSI-fication as a first-class
deliverable — see the migration and test findings below. If the package area
genuinely wants (a), that is a decision to *write down* with its consequence
("Prose is terminal-only by design") spelled out, not an open bullet.

### High — D7/D9: Markdown emission is wrong for inline emphasis

D7 copies Spec A's Markdown rule verbatim: "emits **no** style into the
Markdown body, by design, no diagnostic." That is correct for `Layout` — a
margin has no Markdown form. It is **wrong for the `emphasis` field of `Style`
on inline nodes.** Bold, italic, and strikethrough are *Markdown-native*:
`**bold**`, `_italic_`, `~~strike~~` (GFM). If a `Prose("**bold**")` lowers to
an inline node with `Style { emphasis.bold }` and the Markdown renderer drops
it, the round-trip `**bold**` → tree → Markdown loses the bold — a real
fidelity regression, not a clean by-design drop.

`Style` therefore needs a **split emission contract on the Markdown target**,
not Spec A's blanket "drop everything":

| `Style` facet | Markdown | MarkdownPlus | Rationale |
|---|---|---|---|
| `emphasis.bold` / `.italic` / `.strikethrough` | emit `**`/`_`/`~~` | same | Markdown-native, must round-trip |
| `emphasis.underline` | drop (or `<u>`?) | `<u>` | no CommonMark form |
| `color` / `background` / `border` / `fill` | drop, no diagnostic | maybe `<span style>` | not representable in plain Markdown |

This also collides with the existing `Emphasis` / `Strong` **node kinds** (D6
lists them as inline kinds). The spec proposes an `Emphasis` *struct* (D4) while
an `Emphasis` *node kind* already exists — same name, two meanings, guaranteed
confusion. The spec must (1) rename one of them, and (2) decide the
representational question: is "bold" a node *kind* (`Strong`, semantic) or a
`Style.emphasis` *attribute* (presentational)? Prose's `**bold**` is
presentational; darkmatter's parsed Markdown `**` may be semantic. Pick the
model and say how Prose lowers — this is load-bearing for the whole Markdown
round-trip.

### High — D9: Prose is absent from the migration list

D9 enumerates the components to migrate — `BlockQuote`, `Section`, `Table`,
`Progress` — and **omits `Prose` entirely**, despite Prose being the component
the spec's hardest open question is *about*. If Prose stays a bespoke
`TerminalRenderable` that hand-writes ANSI, the tree renderer never sees its
inline styles and the `Styling`-facet drift the spec promises to burn down
simply reappears as inline drift.

D9 needs an explicit Prose item: Prose's three grammars (atomic tokens, block
tags, Markdown subset) become a parser producing inline `Style`-bearing
`RenderNode`s; the bespoke ANSI path is retired (or kept as a thin
`TreeRenderable` shim during migration, per the Spec A non-big-bang pattern).
Note the placement question: Prose lives in `biscuit-terminal` but the tree and
`Style` live in `renderable` — the spec should say whether the Prose *parser*
moves to `renderable` or stays in `biscuit-terminal` emitting
`renderable::tree` nodes.

### High — D6/OQ#3: "non-inherited, uniform with Layout" breaks Prose nesting

D6 leans toward a non-inherited `Style` "for v1 (uniform with `Layout`)." That
is defensible for *block* Style but is **wrong, or at least insufficient, for
inline Style**, and the spec does not distinguish the two cases.

Prose grammars nest: `<bold><blue>text</blue></bold>`. Terminal SGR is
inherently *stateful/accumulative* — the inner `blue` run is also bold. If
inline `Style` is non-inherited, then either:

- the renderers must **accumulate `Style` down the inline ancestor chain**
  (inline Style *does* cascade, unlike block Layout), or
- Prose must **flatten** the nesting at lowering time so every inline leaf node
  carries the fully-resolved cumulative `Style`.

Both are viable; doing neither produces wrong output. The spec's one-line
"non-inherited for v1" glosses over this. Recommendation: state that **inline
`Style` accumulates along the inline ancestor chain** (matching SGR and CSS
`color`/`font-weight` inheritance) while **block `Style` does not** — and if
that asymmetry is unacceptable, require Prose to flatten. Either way it must be
written down before planning.

### Medium — D2/D4: `Emphasis` is missing Prose's vocabulary

D4's `Emphasis` struct has `bold, italic, dim, underline, strikethrough`. The
`Prose` component (per `biscuit-terminal/docs/components/prose.md`) also
supports **`blink`**. Either `blink` is added to `Emphasis` (with documented
terminal-only degradation — there is no live browser or Markdown analogue,
`text-decoration: blink` is dead in CSS), or `blink` is explicitly dropped from
Prose. The spec must reconcile the two vocabularies; a silent mismatch will
surface as a migration gap.

Relatedly, the per-target lowering of each emphasis flag needs an explicit
matrix (see the High Markdown finding) — `underline` and `strikethrough` lower
cleanly to terminal SGR and browser `text-decoration` but diverge on Markdown,
and `dim` lowers to `opacity` on browser but has no Markdown form.

### Medium — D2/D7: Prose's color tags must be proven to map onto `Color`

Prose supports foreground/background color via named ANSI colors, bright
variants, Tailwind colors (`gray-800`), web colors (`coral`), and `<rgb #hex>`.
D2 re-homes `color`/`background` as `Option<TargetValue<Color>>` and D3 says the
terminal renderer degrades via `renderable::color`. Good — but Prose currently
resolves these through `biscuit-terminal`'s *own* color system. The spec should
state as a concrete migration item that **every Prose color tag maps onto a
`renderable::color::Color` variant** (it should — `BasicColor`/`Tailwind`/`Web`/
`Rgb` all exist) and that Prose's bespoke color resolution is retired in favor
of the shared degradation path. Otherwise we keep two color systems and the
"author once, degrade automatically" contract does not actually hold for Prose.

### Medium — D5: the slot system does not model inline styling

The D5 candidates (slot map, per-slot namespaces, typed per-component structs)
are all **block-component-shaped** — table rows/cells, section heading/body.
Inline `Prose` runs have no "slots"; they have a *tree of nested styled spans*.
The spec should explicitly state that inline `Style` is a flat `Style` per
inline node (the nesting *is* the structure, resolved per the inheritance
finding above) and that the slot system applies only to block components. As
written, D5 reads as if every styled thing needs a slot, which does not fit
Prose.

### Low — OQ#8: confirm the Prose ↔ CodeRenderer ↔ Style seam

OQ#8 already notes a code block's *frame* is `Style` while its *content*
highlighting is `CodeRenderer`. Add the parallel Prose statement: inline
`InlineCode` runs carry a `Style` for their *frame* (e.g. a subtle background)
but their content is not syntax-highlighted — Prose's `InlineCode` is plain
styled text, not a `CodeRenderer` seam. Worth one sentence so the boundary is
unambiguous.

### Low — Success criteria and Required tests omit Prose

Given that Prose is the spec's headline open question, neither the Success
Criteria nor the Required-tests list mentions it. Add acceptance criteria:

- Prose's three grammars lower to inline `Style`-bearing `RenderNode`s.
- `**bold**` / `_italic_` / `~~strike~~` round-trip Prose → tree → **Markdown**
  byte-faithfully.
- A `<rgb>` / Tailwind color in Prose degrades correctly at each `ColorDepth`.
- Nested `<bold><blue>…` produces correct cumulative SGR on terminal and
  correct nested CSS on browser.
- Prose color on the Markdown target drops without corrupting the body.

## Suggested Spec Edits Before Planning

1. **Close OQ#1 in D6**: commit to `Style` as the substrate `Prose` lowers
   onto; add "make `Prose` cross-target" as an explicit Goal.
2. Replace D7's blanket "Markdown emits no style" with a per-facet emission
   matrix: emphasis (`bold`/`italic`/`strikethrough`) is Markdown-native and
   must round-trip; color/background/border/fill drop silently.
3. Rename the D4 `Emphasis` struct (or the `Emphasis` node kind) and state
   whether bold is a node *kind* or a `Style` *attribute*, and how Prose lowers.
4. Add a Prose item to D9: grammar → inline `Style`-bearing nodes; retire the
   bespoke ANSI path; decide whether the Prose parser moves to `renderable`.
5. In D6, distinguish inline vs block inheritance explicitly — inline `Style`
   accumulates along the inline chain (or Prose flattens); block does not.
6. Reconcile `Emphasis` with Prose's full vocabulary (`blink`) and give the
   per-target lowering matrix for `underline`/`strikethrough`/`dim`/`blink`.
7. State that Prose color tags map onto `renderable::color::Color` and Prose's
   bespoke color resolution is retired.
8. Clarify in D5 that slots are block-only; inline `Style` is flat-per-node.
9. Add Prose-specific Success Criteria and Required tests.

## Verdict

**Not ready for planning.** The block-appearance half of the spec
(`BlockQuote` / `Section` / `Table` / `Progress` / `PageBackground` re-homing)
is in good shape and could almost stand on its own. But the question this
review was asked — *can `Style` carry `Prose` cross-target?* — is the spec's
own stated "biggest unresolved interaction," and it is still open. Until D6/OQ#1
is decided in favor of `Style`-as-substrate, the Markdown emission contract is
split correctly, and Prose appears in the migration list and test plan, a plan
built on this draft would either ship a terminal-only Prose or rediscover all of
this mid-implementation. Close OQ#1 first; the rest follows from it.
