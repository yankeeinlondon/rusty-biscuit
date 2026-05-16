# The Fold — Challenges and Options

**Date:** 2026-05-16
**Status:** Analysis / design discussion
**Subject:** `darkmatter::markdown::render_tree::fold_markdown_to_document`

## What "the fold" is

The **fold** is the events → tree pass in darkmatter. It takes a Markdown
string, runs `pulldown-cmark` 0.13, and folds the resulting event stream into a
canonical `renderable::tree::Document`:

```rust
pub fn fold_markdown_to_document(
    source: SourceDescriptor,
    input: &str,
) -> (Document, Vec<Diagnostic>);
```

Mechanically it is a **stack-based pass** over
`Parser::new_ext(input, options).into_offset_iter()`:

- a stack of `Frame`s holds in-progress containers, seeded with a synthetic
  `Root` frame;
- `Event::Start(tag)` pushes a `Frame`, recording the start byte offset;
- `Event::End` pops the frame, computes its `SourceSpan`, and appends the
  finished node to the parent;
- leaf events (`Text`, `Code`, breaks, `Rule`, raw HTML, footnote references,
  task markers) append leaves directly;
- the fold is **total** — every event becomes a node or raises a `Diagnostic`;
  nothing is silently dropped.

It is the part of the render-tree architecture that proves the `NodeKind`
vocabulary is sufficient against *real* parser output, and it is the eventual
replacement spine for darkmatter's `as_html` / `for_terminal` pipelines. It is
currently **experimental and internal** — no public path routes through it.

The fold works and is well tested for the common CommonMark + GFM subset. The
challenges below are the things standing between "works for the Milestone 1
subset" and "can carry darkmatter's public rendering."

---

## Challenge 1 — Custom inline styles and HR attributes cannot reach the fold

**The problem.** darkmatter has two bespoke event-stream processors:
`InlineStyleProcessor` (recognizes `==mark==` and dim inline styles) and
`RuleProcessor` (horizontal rules carrying attribute blocks). The legacy
`as_html` path consumes them as `RuleProcessor::new(InlineStyleProcessor::new(parser))`.

The fold **cannot** use them:

- Both are iterator adapters bounded `where I: Iterator<Item = Event<'a>>` and
  yield a custom `InlineEvent`. They cannot consume an `OffsetIter` (whose item
  is `(Event, Range<usize>)`).
- Worse, they *structurally destroy* byte offsets: `InlineStyleProcessor` splits
  one `Event::Text` into several synthetic `Text` events built from owned
  strings; `RuleProcessor` buffers a whole paragraph and replaces it.

So `==mark==`, dim, and HR-with-attributes folding is **deferred** — the fold
runs on plain `into_offset_iter()` and never sees those constructs. This is the
single largest gap between the fold and feature parity with `as_html`.

### Option 1A — Teach the processors to thread `Range`

Change `InlineStyleProcessor` / `RuleProcessor` to be generic over
`Iterator<Item = (Event, Range)>` and preserve/synthesize ranges through their
transforms.

- **Pros:** the fold gets the custom constructs *with* real spans; one event
  pipeline serves both the legacy renderers and the fold; no parallel parsing.
- **Cons:** invasive change to battle-tested code that the legacy path depends
  on; "synthesize a range for a split text segment" has no single correct
  answer (sub-ranges of the original? the whole original repeated?); risk of
  regressing `as_html` / `for_terminal`.

### Option 1B — Re-implement the custom syntax inside the fold

Drop the processors for the fold's purposes; recognize `==mark==`, dim, and HR
attributes directly while folding (e.g. post-process `Text` nodes, or a small
fold-local scanner) so byte offsets are preserved natively.

- **Pros:** the fold owns its own pipeline end to end; offsets stay exact; no
  change to legacy code.
- **Cons:** **duplicates** the custom-syntax grammar — two implementations to
  keep in sync, exactly the divergence-bug class the render tree exists to kill;
  the custom inline grammar is non-trivial (flanking rules).

### Option 1C — Two-pass correlation

Keep the offset-preserving fold as the structural pass; run a second pass
through the processor stream purely to locate `mark`/`dim`/HR regions, then
correlate the two by position.

- **Pros:** no change to legacy code; structural spans stay exact.
- **Cons:** the processor stream has *no offsets*, so correlation is heuristic
  and fragile; likely more code and more risk than the rest of the fold
  combined; was already assessed as the worst option during implementation.

**Recommendation:** 1A is the principled fix and the only one that avoids a
second grammar implementation. Scope it as its own feature with `as_html`
parity fixtures as the regression gate. 1B is the pragmatic fallback if 1A's
range-synthesis proves intractable.

---

## Challenge 2 — Frontmatter is never folded

**The problem.** `DocumentMetadata.frontmatter` is always `None`. darkmatter
strips frontmatter *before* the parser sees the content, and the fold's options
do not enable metadata blocks. The `Document` has a typed frontmatter slot that
the fold never fills.

### Option 2A — Accept frontmatter as a parameter

`fold_markdown_to_document` takes an `Option<Frontmatter>` (or the already-parsed
metadata) from darkmatter's existing extraction step and stores it on the
`Document`.

- **Pros:** trivial; reuses darkmatter's proven frontmatter extraction; keeps
  the fold's single responsibility (events → tree).
- **Cons:** the caller must remember to extract and pass it; the fold's output
  is only complete if the caller cooperates.

### Option 2B — Fold extracts frontmatter itself

The fold detects and strips the leading frontmatter block before parsing, or
enables `ENABLE_YAML_STYLE_METADATA_BLOCKS` / pluses-delimited metadata and
folds `MetadataBlock` events into `DocumentMetadata`.

- **Pros:** one call yields a fully-populated `Document`; no caller discipline
  needed.
- **Cons:** duplicates extraction logic darkmatter already has; the
  `pulldown-cmark` metadata extension and darkmatter's own frontmatter rules may
  not agree on edge cases (formats, delimiters).

### Option 2C — Leave it `None`, document it as out of scope

Treat frontmatter as a document-assembly concern handled above the fold; the
fold only ever produces body structure.

- **Pros:** zero work; clean separation.
- **Cons:** `Document` advertises a frontmatter slot it never fills — a
  latent "why is this always `None`" trap; transforms that need metadata
  (interpolation, TOC) have no home for it.

**Recommendation:** 2A — the fold consumes darkmatter's existing extraction
result. It keeps one frontmatter implementation and makes the `Document`
genuinely complete.

---

## Challenge 3 — Parser-option divergence from the legacy render path

**The problem.** The fold parses with
`ENABLE_TABLES | ENABLE_STRIKETHROUGH | ENABLE_TASKLISTS | ENABLE_FOOTNOTES |
ENABLE_SUPERSCRIPT | ENABLE_SUBSCRIPT`. darkmatter's legacy `markdown_parse_options()`
enables only `ENABLE_TABLES | ENABLE_STRIKETHROUGH`. For the *same input* the
fold and the legacy renderers see **different event streams** — the fold
recognizes task lists, footnotes, and super/subscript that `as_html` /
`for_terminal` treat as plain text. When the fold is meant to replace those
renderers, this is a behavior-change hazard hiding inside an option flag.

### Option 3A — Make the fold match the legacy option set exactly

Fold with `ENABLE_TABLES | ENABLE_STRIKETHROUGH` only.

- **Pros:** byte-for-byte the same event stream; migration changes the renderer,
  not the parse; trivially defensible parity.
- **Cons:** throws away footnotes / task lists / super-subscript — real
  features the tree already models well; the tree becomes *less* capable than
  it is today for no benefit beyond conservatism.

### Option 3B — Single shared options constant

Promote one `parse_options()` constant used by *both* the legacy path and the
fold; widening it is then a deliberate, reviewed, one-line change that moves
both pipelines together.

- **Pros:** the two pipelines can never silently diverge; widening coverage is
  explicit and shared; honest single source of truth.
- **Cons:** widening the constant changes legacy `as_html` / `for_terminal`
  output *now* (e.g. task lists start rendering) — that is a behavior change to
  the shipping path and needs its own review and fixtures.

### Option 3C — Keep divergence, gate it with parity fixtures

Accept that the fold is intentionally richer; make the parity suite assert,
per construct, whether the difference is "fold is more capable" (accepted) or
"semantic mismatch" (blocker), so the divergence is visible and classified.

- **Pros:** keeps the tree's richer capability; the divergence is measured, not
  hidden; matches the existing Phase 11 parity philosophy.
- **Cons:** divergence still exists at cutover — each extra option is a
  migration decision; relies on fixture discipline to stay honest.

**Recommendation:** 3B as the structural fix (one shared constant), executed
with 3C's discipline (parity fixtures classify every construct the wider option
set newly recognizes). Decide option-by-option whether the legacy path adopts
each extension at migration time.

---

## Challenge 4 — Container span fidelity

**The problem.** A container node's `SourceLocation` is
`Event::Start.range.start .. Event::End.range.end`. This is well defined and
fine for diagnostics, but it has not been validated as **byte-faithful** for
consumers that want to *rewrite* source from spans (formatters, minimal-diff
edits, transform passes). Edge cases: trailing blank lines in loose lists,
nested-container boundaries, a paragraph's range versus its enclosing list
item's range.

### Option 4A — Keep Start..End, document it as the contract

Declare "container span = the tag's `Start..End` byte range" as the spec and
require consumers to live with it.

- **Pros:** zero work; already implemented and tested; predictable.
- **Cons:** if it is subtly wrong for some container, every consumer inherits
  the bug; "we never checked" is not a contract.

### Option 4B — Property-test spans against the source

Add fixtures / property tests asserting `input[node.span.bytes]` re-parses to an
equivalent subtree for every container kind, across loose/tight lists, nested
quotes, and blank-line cases. Fix the fold where it fails.

- **Pros:** turns the span contract from assumed to proven; catches the subtle
  cases before a transform feature depends on them.
- **Cons:** real effort; some cases (a paragraph inside a loose list item) may
  have no "obviously correct" range and force a documented judgment call.

### Option 4C — Defer precise spans until a consumer needs them

Mark container spans "best-effort, diagnostics-grade only" and revisit when the
first source-rewriting transform is built.

- **Pros:** honest about current guarantees; no speculative work.
- **Cons:** the `compose/` re-homing (a stated roadmap item) *is* that consumer
  — deferring just moves the work; spans baked into serialized documents are
  hard to tighten later without a format note.

**Recommendation:** 4B, scoped to land alongside the first transform feature
that consumes spans (likely the `compose/` re-homing). Until then, 4C's honest
labelling ("diagnostics-grade") is the correct interim posture.

---

## Challenge 5 — Owned-tree memory cost

**The problem.** The fold builds a fully **owned** tree: every `RenderNode` is
heap-allocated and every string is `CowStr::into_string()` — copied out of
`pulldown-cmark`'s events, which are themselves mostly *borrowed* slices of the
input. The whole document is resident at once. The legacy streaming serializers
never materialize the document. For large documents, transcluded content, and
generated subtrees the cost is real, and the spec called it out as a risk.

### Option 5A — Accept it; keep benchmarks honest

Treat the owned tree as the deliberate cost of "parse once, walk per target",
and rely on the Phase 11 stress benchmarks to keep the number visible.

- **Pros:** the owned tree is what makes the model simple (no lifetimes,
  serializable, transformable); benchmarks already exist; most documents are
  small.
- **Cons:** does nothing for genuinely large inputs; "we benchmarked it" is not
  "we bounded it."

### Option 5B — Borrow with a lifetime (`CowStr` / `&str` in nodes)

Make `RenderNode` carry `CowStr<'a>` so unmodified text stays borrowed from the
input.

- **Pros:** eliminates the bulk of the string copying for parsed documents.
- **Cons:** a lifetime parameter on the *entire* public tree API — infects
  `TreeRenderable`, every renderer, every component; breaks `serde` owned
  round-trips; synthetic/component/transcluded nodes have no input to borrow
  from. This was explicitly rejected in the spec ("owned strings… no lifetime
  parameter") and reversing it is a deep API change.

### Option 5C — Intern strings / share via `Arc<str>`

Keep the tree owned but deduplicate repeated strings (class names, repeated
component subtrees) through an interner or `Arc<str>`.

- **Pros:** cuts memory for the repetitive cases (component subtrees, classes)
  without a lifetime; stays serializable.
- **Cons:** adds an interner to thread through the fold; helps repetition, not
  a single large unique document; `Arc` clones add atomic traffic.

**Recommendation:** 5A for now — it is the deliberate, already-benchmarked
design. Document expected-size boundaries next to the benchmarks. Revisit 5C
only if a concrete workload (large transcluded corpora, many repeated subtrees)
shows up in the numbers. 5B is effectively off the table given the spec's
owned-tree decision.

---

## Challenge 6 — Lossy GFM constructs lose parser data

**The problem.** Some `pulldown-cmark` constructs carry data with **no
`NodeKind` field to hold it**. The clearest case: `Tag::BlockQuote(Option<BlockQuoteKind>)`
— a GFM alert (Note / Tip / Important / Warning / Caution). The fold folds it to
a plain `BlockQuote` and raises a `Diagnostic::lossy` ("GFM alert kind … dropped").
The alert kind is genuinely lost. The same shape will recur for any future
parser data the tree has no slot for.

### Option 6A — Carry the lost data in namespaced `attrs.data`

Store the alert kind under e.g. `attrs.data["darkmatter.alert.kind"]` rather
than dropping it.

- **Pros:** no data loss; no `NodeKind` change; `attrs.data` exists precisely as
  this escape hatch; renderers can opt in (a browser renderer could emit an
  alert `<div class="alert-note">`).
- **Cons:** stringly-ish / namespaced-key data instead of a typed field;
  every consumer must know the key; if alerts turn out load-bearing it should
  really be promoted to a typed field later anyway.

### Option 6B — Promote alerts to a first-class `NodeKind`

Add an `Alert { kind, children }` variant (or a field on `BlockQuote`).

- **Pros:** fully typed; exhaustive `match` forces every renderer to handle it;
  no lost data.
- **Cons:** grows the core `NodeKind` enum for one GFM-specific construct;
  every renderer must now render it; the spec deliberately kept `NodeKind`
  lean — each addition is a real cost across three renderers.

### Option 6C — Keep dropping it, keep the diagnostic

Accept the loss; the `Diagnostic::lossy` already makes it loud.

- **Pros:** zero work; the loss is never silent.
- **Cons:** a `Strict` render of any document with a GFM alert fails; alerts are
  a common, visible feature — quietly degrading them to plain quotes is a real
  fidelity gap versus `as_html`.

**Recommendation:** 6A now (no loss, no enum churn), with a documented rule —
if parity testing shows alerts are load-bearing for darkmatter's corpus,
promote to 6B in a follow-up. This mirrors the spec's stated `attrs.data` →
typed-field promotion policy.

---

## Challenge 7 — Inventory, fold, and `pulldown-cmark` version coupling

**The problem.** Three things must agree: the `inventory` module's documented
dispositions, the `disposition_for_*` functions, and the fold's own `match`
arms. The inventory has compile-time exhaustive-match guards (a `pulldown-cmark`
enum change breaks the build), which is good — but the **dispositions** and the
fold arms are *manual mappings*. A `pulldown-cmark` 0.13 → 0.14 bump, or a new
variant, forces edits in all three places, and the inventory's disposition can
silently disagree with what the fold actually does.

### Option 7A — Make the fold consume the inventory dispositions directly

Drive the fold's unsupported/lossy classification from
`inventory::disposition_for_*` rather than re-deciding in the fold's `match`.

- **Pros:** one source of truth for "what disposition does this event get";
  inventory and fold cannot drift; the compile-time guard then protects both.
- **Cons:** the fold still needs per-`Node` construction logic the inventory
  cannot express; only the *classification* unifies, not the construction —
  partial dedup.

### Option 7B — A test asserting fold behavior matches the inventory

Add a test that, for every event variant, checks the fold's actual output
disposition equals the inventory's documented disposition.

- **Pros:** catches drift without restructuring the fold; cheap; keeps the
  inventory honest as living documentation.
- **Cons:** another artifact to maintain; proves consistency, does not *enforce*
  a single source of truth.

### Option 7C — Pin and gate the `pulldown-cmark` version

Treat the `pulldown-cmark` minor version as a deliberate dependency: pin it
exactly, and make a bump an explicit task with an inventory re-verification
checklist.

- **Pros:** no surprise event-shape changes; the existing compile-time guard
  plus a checklist is a solid process.
- **Cons:** process, not mechanism — relies on discipline; delays picking up
  upstream parser fixes.

**Recommendation:** 7A + 7B together — fold classification driven by the
inventory (mechanism) *and* a behavior-equivalence test (safety net). 7C is the
sensible default dependency posture regardless.

---

## Challenge 8 — The fold only ever produces one `Parsed` source

**The problem.** The tree model has a rich provenance system: `SourceRegistry`
with multiple `SourceId`s, and `Provenance::{Parsed, Synthetic, Generated,
Transcluded}`. The fold uses **none of it** — `single_source_registry` registers
exactly one source and every node is `Provenance::Parsed` from it (the `Root` is
`Synthetic` only because the builder makes it so). Transclusion (folding another
document in as `Transcluded`), component-subtree splicing, and transform-
generated nodes (`Generated`) are modeled but not produced. The machinery exists
ahead of any code that exercises it.

### Option 8A — Leave it; the machinery is intentionally ahead of need

The provenance model is reserved for the deferred `compose/` re-homing and
component splicing; the single-source fold is correct for what it does today.

- **Pros:** no speculative work; the model is ready when those features land;
  YAGNI is satisfied because the *features* are planned, just deferred.
- **Cons:** untested code paths (`Transcluded`, `Generated`, multi-source
  registries) — "modeled but never produced" tends to be subtly wrong when
  first exercised; serialized-format coverage for those variants is thin.

### Option 8B — Exercise the model with a minimal transclusion fold now

Add a small multi-source capability — e.g. fold a document that `@includes`
another, registering a second source and emitting `Transcluded` nodes — purely
to prove the provenance machinery end to end.

- **Pros:** converts "modeled" into "proven"; surfaces registry/serialization
  bugs early; gives the `compose/` re-homing a working foundation.
- **Cons:** real scope; risks pulling transclusion design forward before it is
  properly specified; the spec explicitly defers the `compose/` pipeline.

### Option 8C — Add provenance fixtures without a real producer

Keep the single-source fold, but add serialization/round-trip fixtures that
hand-construct `Transcluded` / `Generated` / multi-source `Document`s so the
*format* is covered even though the fold does not yet emit them.

- **Pros:** cheap; closes the serialization-coverage gap; no premature
  transclusion design.
- **Cons:** does not exercise a real producer path; fixtures can drift from
  whatever the eventual transclusion fold actually emits.

**Recommendation:** 8A + 8C — keep the single-source fold (it is correct), but
add 8C's hand-constructed fixtures so the provenance variants and multi-source
registries are at least serialization-tested. Defer 8B to the `compose/`
re-homing feature, which is its natural home.

---

## Smaller known issues

These are real but lower-stakes; each likely needs only one fix, not an options
analysis.

- **`build_container`'s unreachable arm fabricates a node silently.** `Root`,
  `HtmlBlock`, `TableHead`, and `Unsupported` are handled in `Fold::end`; the
  arm for them in `build_container` returns `RenderNode::unsupported("internal:
  unhandled")` with **no diagnostic**. If a future refactor ever reaches it, the
  failure is invisible. Prefer `unreachable!()` with an explanatory message — a
  loud panic on an internal-invariant violation is better than a fabricated
  node. (It is genuinely unreachable today; this is about future-proofing.)
- **`Unsupported` is currently unreachable from real Markdown.** Math and
  definition-list options are disabled, so no Markdown input folds to an
  `Unsupported` node. The fold's totality is correct and worth keeping, but the
  `Unsupported` path is exercised only by direct unit tests, never by a fixture.
  If the option set widens (Challenge 3), this changes — keep an eye on it.
- **Table header is a positional convention.** `TableHead` folds into
  `Table.children[0]` as an ordinary `TableRow`; "row 0 is the header" is
  enforced by documentation, not types. A table with no header row, or a
  malformed one, can be misread by a consumer that forgets the convention. A
  typed marker (a `header: bool` on `TableRow`, or a distinct first field on
  `Table`) would remove the footgun at the cost of a `NodeKind` change.
- **`code_text` only concatenates direct `Text` children.** Code-block bodies
  arrive as `Event::Text` lines, so this is correct today — but it silently
  ignores any non-`Text` child. If `pulldown-cmark` ever emits something else
  inside a code block, the content is dropped without a diagnostic.

## Summary of recommendations

| Challenge | Recommended direction |
|-----------|------------------------|
| 1. Custom styles / HR attrs | 1A — thread `Range` through the processors (own feature, parity-gated); 1B as fallback |
| 2. Frontmatter | 2A — fold consumes darkmatter's existing extraction result |
| 3. Option divergence | 3B — one shared `parse_options()` constant, 3C parity discipline |
| 4. Container spans | 4B — property-test spans, scoped with the `compose/` re-homing; 4C labelling until then |
| 5. Memory cost | 5A — accept + benchmark; 5C interning only if a workload demands it |
| 6. Lossy GFM constructs | 6A — namespaced `attrs.data`, promote to typed (6B) if load-bearing |
| 7. Inventory / version drift | 7A + 7B — inventory-driven classification plus an equivalence test |
| 8. Single-source fold | 8A + 8C — keep it, add provenance serialization fixtures; defer 8B to `compose/` |

The throughline: the fold is correct and well tested for what it does today;
the open work is **coverage breadth** (Challenges 1–3, 6), **guarantee depth**
(Challenges 4, 7), and **proving the reserved machinery** (Challenge 8). None of
it blocks the current experimental status — all of it is on the path to letting
the fold carry darkmatter's public rendering.
