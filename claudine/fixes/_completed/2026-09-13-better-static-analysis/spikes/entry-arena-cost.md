---
created: 2026-09-13
kind: spike
area: claudine
packages:
    - dmls
question: Can D7's sequence descent ship with the entry arena's current linear
    pointer scans, or does the arena need a pointer-to-index map first?
related:
    - claudine/fixes/2026-09-13-better-static-analysis/spec.md
---

# Entry-arena cost under sequence descent

## The question

D7 teaches `lower_mapping` (`darkmatter/dmls/src/overlay/frontmatter.rs`) to
descend into `Node::Sequence`. `FmEntry` identity is its `pointer` string, and
`index_of`, `entry_by_pointer`, `entry_by_dotted` and `entry_or_ancestor` all
find an entry by scanning the arena linearly. Descent multiplies the entry
count. Does that turn a per-keystroke path into a problem, and does the change
have to carry an index?

## Method

Nothing in the repository was modified for the measurements. A standalone
harness in the session scratchpad vendors the current `lower`/`lower_mapping`
and the five lookup helpers verbatim, against the same parser DMLS uses
(`rlsp-yaml-parser` 0.11), and adds two descent variants:

- **keys-only** — descend into sequence items, but emit no entry for the item
  itself; only the keys of a mapping item become entries.
- **item-entries** — additionally emit one entry per sequence item, keyed by
  its decimal index.

The distinction is not cosmetic; it changes the arena size by another factor
and changes `entry_or_ancestor`'s answer (see the round-trip section).

Corpus: every `.md` file in the worktree outside `target/`, `node_modules/`
and `.git/` — 4,241 files, of which **2,441 have a parseable frontmatter
mapping**. `prompts/`, `prompts/_implement/`, `prompts/_reviews/` and
`claudine/cli/tests/fixtures/` are included in that sweep and are called out
separately below.

Timing: `--release`, three separate process runs, nine repetitions each, and
each lookup repeated 1,000 times inside a rep. Figures below are the per-run
medians; the **drift bracket across the three runs was ≤ 10% on every line**,
so treat any difference under ~15% as noise, not signal.

## Finding 1 — entry counts (measured)

Arena size across all 2,441 frontmattered documents:

| | p50 | p90 | p99 | max | mean |
| --- | --- | --- | --- | --- | --- |
| today (mapping-only) | 4 | 28 | 69 | 96 | 10.0 |
| descent, keys-only | 4 | 38 | 617 | 1,795 | 32.3 |
| descent, item-entries | 4 | 104 | 945 | 2,321 | 50.9 |

Multiplier, restricted to the 1,071 documents with at least five entries
today (item-entries mode): **p50 1.52×, p90 8.4×, p99 55.7×, max 89.3×**.
Keys-only: p50 1.00×, p90 5.2×, max 69×.

Two populations are hiding in that distribution:

- **Real prompts** behave as the spec describes. All 44 documents under
  `prompts/` total 381 entries today → 527 keys-only → 662 item-entries. The
  largest is `prompts/_implement/implement-plan.md`: **32 → 53 → 70** entries,
  max depth **1 → 5**, 43 frontmatter `{{ … }}` spans. Next are
  `prompts/_reviews/feature-review.md` (22 → 41 → 56) and the shipped fixture
  `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`
  (29 → 44 → 55). Nothing in the authoring corpus exceeds ~70 entries.
- **Research documents** are the worst case and are two orders of magnitude
  larger. `claudine/docs/research/steering/opencode.md`: **26 → 1,795 →
  2,321**. Six more steering/agent-cli research docs land between 1,200 and
  2,000. `fixes/2026-07-22-mega-merge/plan.md` is the pure-scalar-sequence
  shape: 44 → 44 (keys-only leaves it untouched) → **2,267** under
  item-entries. These are indexed and openable like any other document, so
  they are in scope for the request paths even though nobody authors
  expressions in them.

## Finding 2 — where the linear scans are, and what triggers them

Every callsite outside `frontmatter.rs` itself:

| Scan | Callsite | Trigger | Shape |
| --- | --- | --- | --- |
| `entry_or_ancestor` | `diagnostics/frontmatter.rs:565` (`semantic_problem_range`) | per **validation problem**, diagnostics pass | O(problems × depth × n) |
| `entry_by_pointer` | `diagnostics/frontmatter.rs:338` | per authored `$schema` value | O(values × n) |
| full scan | `diagnostics/frontmatter.rs:470` (`nested_property_value_span`) | per **failing** meta-schema value only | O(failures × n) |
| `entry_by_dotted` | `diagnostics/frontmatter.rs:671`, `715`; `providers/dsl.rs:396`, `876`, `900`, `912`; `providers/code_actions.rs:285`; `graph/substrate.rs:383` | per expression root / per style warning | O(roots × n) |
| `index_of` (via `key_path`) | `providers/frontmatter.rs:1092` (hover), `1917` (completion) | **once** per hover / completion | O(n) |
| `entry_at_offset` | `providers/frontmatter.rs:1091`, `1167` | once per hover / definition | O(n) |
| whole-arena iteration | `providers/frontmatter.rs:502`, `902`, `1378`, `1405`; `providers/dsl.rs:934`; `overlay/schema.rs:219`; `graph/substrate.rs:524` | diagnostics / links / folds | O(n) + per-entry work |

**No production path runs a linear scan inside a per-entry loop.** The
entry-iterating passes use `key_path_at(index)`, which walks the `parent`
chain (O(depth)), not `key_path(entry)`, which goes through `index_of`
(O(n)). The only `key_path(entry)`-per-entry shape in the tree is a unit test.
That is the single most load-bearing fact in this spike, and it is a property
of the current code that a careless D7 edit could lose.

Scheduling context, from `diagnostics/scheduler.rs`: dispatch is currently
**synchronous and the configured debounce is not yet applied** — "the
scheduler publishes immediately after each re-index rather than running a
debounce timer". The diagnostics pass therefore does run per `didChange`, on
the loop thread. Everything in the table above is a per-keystroke budget, not
a per-save one.

## Finding 3 — measured cost

`prompts/_implement/implement-plan.md`, the heaviest real prompt (n 32 → 53 →
70). Per-run medians, three runs:

| Operation | today (n=32) | keys-only (n=53) | item-entries (n=70) |
| --- | --- | --- | --- |
| `entry_by_pointer`, miss | 9.8 ns | 18–20 ns | 22–23 ns |
| `entry_or_ancestor`, deep miss | 161–165 ns | 299–311 ns | 293–301 ns |
| `entry_at_offset` (hover) | 14 ns | 24–26 ns | 31–40 ns |
| `key_path_at` over the whole arena | 0.7 µs | 1.1–1.2 µs | 2.1–2.3 µs |
| `key_path` over the whole arena (**O(n²)**, not used in production) | 1.0 µs | 1.8 µs | 3.6 µs |

`claudine/docs/research/steering/opencode.md`, the worst document in the
repository (n 26 → 1,795 → 2,321):

| Operation | today (n=26) | keys-only (n=1,795) | item-entries (n=2,321) |
| --- | --- | --- | --- |
| `entry_by_pointer`, miss | 7.6 ns | 1.16–1.34 µs | 1.53–1.78 µs |
| `entry_or_ancestor`, deep miss | 207–216 ns | 3.12–3.14 µs | 4.11–4.16 µs |
| `entry_at_offset` (hover) | 12 ns | 1.98–2.00 µs | 2.55–2.58 µs |
| `key_path_at` over the whole arena | 0.54 µs | 23.2–23.8 µs | 34.2–35.0 µs |
| `key_path` over the whole arena (**O(n²)**) | 0.83 µs | 715–751 µs | **1.49–1.52 ms** |

Read that last row as the warning it is: the quadratic shape is the only
thing anywhere near a millisecond, and it is the one shape production does not
currently use.

Every lookup that production does use stays in the **microsecond** range even
at 2,321 entries. A hover doing one `entry_at_offset` plus one `key_path`
costs ~4 µs on the worst document in the repo, up from ~25 ns. A diagnostics
pass would need on the order of **250 `entry_or_ancestor` calls on that same
worst-case document** to spend a single millisecond in ancestor lookup.

## Finding 4 — the cost that actually moves (measured count, extrapolated time)

The multiplier that matters is not in the pointer scans. `expression_values`
(`providers/frontmatter.rs:896`) and the document-links pass (`:1378`) call
`def_at_path_ctx` **once per scalar entry**, and for a nested entry that goes
through `nested_shape_for_completion`, which does `root.clone()` plus one
shape clone **per ancestor level**.

Schema-shape clones per diagnostics pass, counted over the whole corpus
(measured counts, one clone per ancestor level per scalar entry):

| | p50 | p90 | p99 | max |
| --- | --- | --- | --- | --- |
| today | 0 | 1 | 39 | 63 |
| after descent (item-entries) | 0 | 83 | 1,583 | **3,683** |

`prompts/_implement/implement-plan.md` goes from 16 clones per pass to 93 —
today's frontmatter is almost entirely depth 0/1, and descent is what creates
depth 2–5 entries in bulk.

The per-clone cost is **not measured** — it needs a built `dmls`/`darkmatter`,
which did not fit the budget. `SchemaShape` is a property map of the whole
darkmatter base schema plus overlays, with `String` keys and owned
`PropertyDef` values, so a clone is plausibly 1–5 µs. Extrapolating: the worst
document moves from ~0.1 ms to **4–18 ms per diagnostics pass**, on the loop
thread, undebounced. That is the number worth chasing, and it is an order of
magnitude above everything in Finding 3.

## Pointer round-trip for index segments (measured)

Verified directly on a probe document containing `tags: [a, b]`, a
lifecycle-shaped `initialize.stack` sequence of mappings, a key containing a
literal `/`, and a mapping key spelled `"0"`:

- `split_pointer` → `join_pointer` round-trips **every** probe pointer
  exactly, including `/odd~1key/0/nested`. Index segments are decimal digits,
  so `encode_pointer_segment` is the identity on them and the split/join pair
  is an exact inverse on already-escaped text. There is no silent mangling.
- `entry_or_ancestor`'s segment-popping walk behaves correctly on index
  segments: `/initialize/stack/1/missing` falls back to `/initialize/stack/1`
  (the item mapping) under item-entries, and to `/initialize/stack` under
  keys-only. Both are correct answers for their mode; neither is silent.
- **No pointer collision is introduced.** A YAML node is either a mapping or a
  sequence, never both, so a mapping key spelled `0` and sequence item `0` can
  never share a parent pointer. The probe's `"0":` key confirms this.

Two things do change loudly, which is the desired failure mode:

1. `test_entry_or_ancestor_falls_back_for_array_index`
   (`overlay/frontmatter.rs:565`) asserts that `/tags/1` falls back to `/tags`
   with `kind == Sequence`. Under **item-entries** descent `/tags/1` becomes an
   exact hit with `kind == Scalar`, and that test fails. Under **keys-only** it
   still passes. That test is the de-facto switch between the two descent
   designs and should be rewritten deliberately, not adjusted to whatever the
   implementation happens to produce.
2. `ValidationProblem.path` is already an RFC 6901 pointer with decimal array
   indices (`/tags/2`, documented in
   `darkmatter/lib/src/markdown/schemas/mod.rs:1166`). Under item-entries
   descent, schema problems inside arrays stop ranging the whole sequence and
   start ranging the failing item. That is a user-visible improvement and a
   behavior change; it needs its own acceptance criterion.

One residual risk the round-trip check does *not* cover: `entry_by_dotted`
matches the **dotted** spelling by exact string, and the harness produced
`initialize.stack[0].when`. Any darkmatter-side producer of dotted paths that
spells an array step differently (`initialize.stack.0.when`) will miss and the
caller will `continue` — a silently dropped diagnostic. `style_diagnostics`
(`diagnostics/frontmatter.rs:715`) is the live instance of that pattern.
Fixing this is not optional work but it is a spelling decision, not a
structural one.

## Verdict

**Sequence descent can ship with the current linear scans. It does not need a
pointer-to-index map, and adding one would be premature optimization.**

- The arena stays small where it matters: every real authoring document ends
  under ~70 entries, where a full scan is ~23 ns.
- Even the worst document in the repository (2,321 entries) keeps every
  production lookup in the microseconds — `entry_or_ancestor` at 4.2 µs,
  hover at 2.6 µs. A hash index would save ~1.5 µs per lookup at a cost of
  one owned `String` key per entry, on a call volume of single-digit lookups
  per request. It buys nothing measurable.
- There is no quadratic path in production today. `key_path_at` (O(depth)), not
  `key_path` (O(n)), is what the per-entry loops use.

Three conditions attach to that verdict:

1. **Choose keys-only vs item-entries deliberately, and state it in the
   spec.** Item-entries doubles the arena again on scalar-heavy documents
   (`fixes/2026-07-22-mega-merge/plan.md`: 44 → 44 → 2,267) and changes
   `entry_or_ancestor`'s answer for `/tags/1`. Keys-only is sufficient for
   D7's stated purpose — reaching expressions authored inside `- item` lines —
   and is the cheaper, less disruptive half. Item-entries is what you want if
   schema problems should range the failing array item rather than the whole
   sequence. Both are defensible; drifting into one by accident is not.
2. **The index that is actually needed is a schema-shape memo, not a pointer
   map.** Memoize `nested_shape_for_completion` per ancestor path for the
   duration of one pass — a `HashMap<Vec<String>, SchemaShape>` (or a
   `Vec<(Vec<String>, SchemaShape)>`, given the tiny key count) built inside
   `expression_values` and the links pass and thrown away after. Maintenance
   cost is one hash per scalar entry and a handful of retained shapes; it
   turns 3,683 worst-case clones into at most one per distinct ancestor path,
   which on real documents is fewer than ten. This is cheap enough to land in
   the same change and is the only performance work D7 warrants.
3. **Guard the quadratic shape.** Nothing may call `key_path(entry)` or
   `entry_by_pointer`/`entry_by_dotted` inside a loop over `entries()`. A
   comment on `index_of` saying so — "O(n); use `key_path_at` inside a loop" —
   costs nothing and is the difference between a 34 µs pass and a 1.5 ms one.

## What this spike did not settle

- **The per-clone cost of `SchemaShape`** is unmeasured; the 4–18 ms
  worst-case figure is extrapolated from a counted 3,683 clones and an assumed
  1–5 µs each. Building `dmls` to measure it did not fit the budget. If the
  memo in condition 2 lands, the number stops mattering; if it does not, that
  measurement should be taken before D7 merges.
- **How many validation problems a real invalid lifecycle document produces**
  was not measured, so the `entry_or_ancestor` call count per pass is an
  assumption. At 4.2 µs a call it would take ~250 problems on the repo's worst
  document to reach 1 ms, which makes it hard to worry about, but it is an
  argument rather than a measurement.
- **Timings are macOS-only**, single host, `--release`, and no cross-OS
  comparison was attempted. The conclusions rest on orders of magnitude, not
  on the specific nanosecond figures, so this is not a material gap.
