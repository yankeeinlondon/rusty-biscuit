# Stage 3 — Structural Projection Completion

## Context

Stage 2 implemented `TreeRenderable`, `MarkdownRenderable`, and
`BrowserRenderable` for every one of the twelve `biscuit-terminal`
components (`BlockQuote`, `Compose`, `FileSystem`, `OrderedList`, `Progress`,
`Section`, `StatusBlock`, `Table`, `TextBlock`, `Todo`, `TwoColumn`,
`UnorderedList`). The Markdown / MarkdownPlus and Browser targets all flow
through the canonical tree across all twelve. The Terminal target flows
through the tree for ten components; **`FileSystem` still defaults to its
bespoke terminal renderer**, and the spec for `FileSystem` explicitly
sanctioned that as a staged migration. One additional limitation was
deferred across all containers: nested non-`Prose` components flatten to
ANSI-stripped text in their projected tree because they do not override
`TerminalRenderable::render_tree_node`.

The Stage 2 baseline — as it exists on disk today — is mixed:

| Component | `render_tree_node` override | `render_bespoke` public hook | Terminal `render` path |
|-----------|----------------------------|------------------------------|------------------------|
| `BlockQuote` | **missing** | none (gated internally on `has_default_border`) | tree (default border) / bespoke (custom border) |
| `Compose` | present | none (retired) | tree |
| `FileSystem` | **missing** | none | **bespoke** |
| `OrderedList` | present | `pub fn render_bespoke` | tree |
| `Progress` | present | `pub fn render_bespoke` | tree |
| `Section` | present | `pub fn render_bespoke` (+ `_optimistic`) | tree |
| `StatusBlock` | **missing** | `pub fn render_bespoke` | tree (default border) / bespoke (custom border) |
| `Table` | present | `pub fn render_bespoke` | tree (default) / bespoke (`prefer_cursor_alignment` + TTY) |
| `TextBlock` | present | `pub fn render_bespoke` (+ `_optimistic`) | tree |
| `Todo` | present | `pub fn render_bespoke` | tree |
| `TwoColumn` | present | `pub fn render_bespoke` (+ `_optimistic`) | tree (default) / bespoke (image overlay) |
| `UnorderedList` | present | `pub fn render_bespoke` | tree |

Stage 3 is the **structural-projection completion pass**. It closes the
nested-component projection gap, decides the `FileSystem` terminal path,
audits the now-redundant compatibility surfaces, and publishes the
migration recipe as a checklist so future contributors do not have to
re-derive it from twelve worked examples.

## Authoritative references

- [tree-rendering.md](../../docs/tree-rendering.md) — render-tree contract
- [layout-and-style.md](../../docs/layout-and-style.md) — `Layout` / `Style`
  contract
- [lessons-learned.md](./lessons-learned.md) — Stage 1 + Stage 2 ledger,
  particularly the **"Nested non-`Prose` block components flatten to text"**
  and **"Is the recipe ready for extraction?"** sections
- [approved-render-tree-functionality.md](./approved-render-tree-functionality.md)
  — the eleven `RT-*` additions consumed by Stage 2

## Core problem to solve

When a container holds a `RenderableTerminalContent::Component(c)`, projection
flows through `RenderableTerminalContent::to_tree_nodes` → `Component(c)` arm
→ `c.render_tree_node()`. That trait method defaults to `None`, so under
`RenderStrictness::Warn` the projector falls back to:

1. Rendering `c` to a terminal string via the bespoke path
2. Stripping ANSI
3. Wrapping the result in a `RenderNode::text` node

Observable effect today:

```text
TwoColumn::new(BlockQuote::new("quoted inside", None), "right")
                         │
                         ▼
NodeKind::BlockQuote {                       ← columns carrier
  children: [
    Paragraph {                              ← should be BlockQuote
      Text("│ quoted inside")                ← prefix baked into text
    },
    Paragraph { Text("right") }
  ]
}
```

`Prose` escapes the trap because the `Component` arm has a dedicated downcast
to `prose.to_render_nodes()`. Stage 3 generalizes that escape hatch so every
IR-aware component projects structurally.

## Work items

### S3-1: Add `render_tree_node` overrides where missing; decide `FileSystem`'s terminal path

**Decision: option (b) — `TerminalRenderable::render_tree_node` is overridden
per-component, not via a centralized downcast chain in
`project_renderable_content`.**

Per the baseline table in the Context section, three components are missing
the override on disk: `BlockQuote`, `FileSystem`, `StatusBlock`. `FileSystem`
also still uses its bespoke renderer for the terminal `render` path, which
is an additional dimension that Stage 3 must decide explicitly.

**Why (b) over a centralized downcast chain:**

- A centralized `if let Some(c) = any.downcast_ref::<BlockQuote>() { … } else
  if … else if …` chain is an O(n) registration site that every new IR-aware
  component must visit. That is exactly the failure mode that bit
  `TwoColumn` in Stage 2.
- The nine components that already have the override use a single private
  projection helper (`to_render_node()` or equivalent) shared with
  `TreeRenderable::render_tree`, locked in by a
  `tree_renderable_and_compat_hook_share_one_projection`-style test.
  Adding the override to the three missing components is a four-line
  mechanical change each.
- Ownership stays with the component, not with a remote helper.

#### S3-1a — Add missing `render_tree_node` overrides

For each of `BlockQuote`, `StatusBlock`, and `FileSystem`, add:

```rust
impl TerminalRenderable for ComponentName {
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(<Self as TreeRenderable>::render_tree(self))
    }
    // ... existing methods
}
```

Add a `tree_renderable_and_compat_hook_share_one_projection`-style test for
each, mirroring what the other nine components carry, so the shared-output
invariant is pinned.

#### S3-1b — Audit the other nine for no-op verification

Verify (don't churn) that `Compose`, `OrderedList`, `Progress`, `Section`,
`Table`, `TextBlock`, `Todo`, `TwoColumn`, and `UnorderedList`:

1. Override `render_tree_node` and delegate to their shared projection
   helper.
2. Carry a parity test that asserts the override and `TreeRenderable::
   render_tree` produce identical output.

Result is a short audit report appended to `lessons-learned.md`. No code
change unless an audit fails — at which point that component joins S3-1a.

#### S3-1c — Decide `FileSystem`'s terminal `render` path

`FileSystem::render` currently calls the bespoke renderer. The
`FileSystem-spec.md` migration sequence sanctioned this until parity tests
for the tree path land. Stage 3 must pick one of three explicit outcomes
and record it in `lessons-learned.md`:

- **(i) Flip to tree.** Replace `FileSystem::render` with a `render_via_tree`
  that calls `<Self as TreeRenderable>::render_tree` and lowers via
  `render_terminal_node`. Add a parity test file
  (`filesystem_parity.rs`) that compares bespoke vs tree across the
  documented variants in [`FileSystem-spec.md`](./components/FileSystem-spec.md)
  (connector geometry, gitignore styling, error/permission states, depth
  limits, highlight precedence, metric annotations, dotfile italic, symlink
  styling). Required if tree-path output is byte-equivalent or acceptably
  divergent across those variants.
- **(ii) Stay bespoke; document permanently.** Keep `FileSystem::render`
  on the bespoke path and document it alongside the other four sanctioned
  escape-hatch components. Required if connector geometry, OSC8 hyperlinks,
  or other features cannot be expressed through the tree even with the
  approved `ListMarkerPolicy::TreeConnectors` and other Stage 1 additions.
- **(iii) Stay bespoke pending Stage 4.** Keep `FileSystem::render` on the
  bespoke path and explicitly defer the flip to a future stage. If chosen,
  the parity-test run that motivated the deferral must name the specific
  missing renderer capability (e.g. "OSC8 hyperlink lowering on `Link`
  nodes," "per-cell color band painting") as the Stage 4 acceptance
  criterion in `lessons-learned.md`. "Missing capability, will revisit"
  is not a sufficient record.

The decision is owned by the implementer based on actual parity-test
results against the `FileSystem-spec.md` variant matrix. The acceptance
criteria below treat all three outcomes as acceptable provided the choice
and (for outcome iii) the missing capability are recorded.

### S3-2: Tighten the deferred parity tests

Once S3-1 lands, every container's nested-component test moves from "text
survives" to "structural kind survives."

**Primary target:** `biscuit-terminal/lib/tests/two_column_parity.rs::
render_via_tree_preserves_nested_block_content`. Replace the relaxed
"text appears somewhere in the left subtree" assertion with:

```rust
let renderable::tree::NodeKind::BlockQuote { children } = &left[0].kind else {
    panic!("expected structural BlockQuote child, got {:?}", left[0].kind);
};
```

**Companion tests to add or tighten:**

- `section_parity.rs` — Section-in-Section, BlockQuote-in-Section,
  Table-in-Section.
- `block_quote.rs` (or a new `block_quote_parity.rs`) — Section-in-BlockQuote,
  List-in-BlockQuote.
- `compose.rs` — expand the nested-component coverage per the fixture
  table below.
- `ordered_list_parity.rs` / `unordered_list_parity.rs` — nested block
  components inside list items.

**Compose nested-component fixture table.** Not every component is a
semantically valid `Compose` part. The expansion covers the components
that *are* valid, with a named minimal fixture and the expected
`NodeKind` per fixture. Components excluded as `Compose` parts are
covered separately via block-container tests.

| Component | Valid as `Compose` part? | Minimal fixture | Expected `NodeKind` (top-level child of the Compose root) |
|-----------|--------------------------|-----------------|----------------------------------------------------------|
| `BlockQuote` | Yes | `BlockQuote::new("quoted", None::<&str>)` | `NodeKind::BlockQuote` |
| `Section` | Yes | `Section::new("Title").push("body")` | `NodeKind::Section { depth: 2, .. }` |
| `Table` | Yes | `Table::new(["A","B"], vec![vec!["x","y"]])` | `NodeKind::Table` |
| `OrderedList` | Yes | `OrderedList::new(vec!["a","b"])` | `NodeKind::List { ordered: true, .. }` |
| `UnorderedList` | Yes | `UnorderedList::new(vec!["a","b"])` | `NodeKind::List { ordered: false, .. }` |
| `Progress` | Yes | `Progress::new(0.5, Some("p".into()))` | `NodeKind::Paragraph` (carries `ProgressHints`) |
| `StatusBlock` | Yes | `StatusBlock::new(Severity::Warning).body(vec![Prose::new("msg")])` | `NodeKind::Root` (composite child layout) |
| `TextBlock` | Yes | `TextBlock::new("text")` | `NodeKind::Paragraph` |
| `Todo` | Yes | `Todo::new("task")` | `NodeKind::List { ordered: false, .. }` |
| `TwoColumn` | Yes | `TwoColumn::new("L", "R")` | `NodeKind::BlockQuote` (columns carrier) |
| `Compose` | Yes (nested) | `Compose::new().push("a").push("b")` | `NodeKind::Root` with `SequenceJoin::None` |
| `FileSystem` | **No** | n/a — requires built `TreeNode` state plus async-style enumeration; not idiomatic as a `Compose` part. Covered via direct projection tests in S3-1c. | n/a |

Each fixture row in the Compose expansion test asserts:

1. The corresponding child node exists in the Compose root's children at
   the expected index.
2. The child's `NodeKind` matches the expected discriminant (use
   `matches!`; do not over-pin field contents the renderer may legitimately
   shape).

Remove every `TODO(stage-3)` comment as its corresponding test tightens.

### S3-3: Strengthen the `to_tree_nodes` fallback policy

After S3-1, the `RenderStrictness::Warn` rendered-then-stripped fallback in
`RenderableTerminalContent::to_tree_nodes` becomes a footgun: any future
component that *forgets* to override `render_tree_node` silently renders to
plain text instead of producing a structural projection.

**Decision: loud-warn, once per concrete type.** Emit `tracing::warn!` from
the fallback path so a missing override is observable in logs and CI test
output without breaking production callers. Matches the Progress /
StatusBlock fallback policy already in place across the codebase.

#### S3-3a — Add a stable type-name helper to `TerminalRenderable`

The current fallback at
`biscuit-terminal/lib/src/render_tree/projection.rs:326-335` builds a label
from `format!("{:?}", component)` and takes the first whitespace-delimited
token. That is not a stable Rust type name and can be unhelpful or
misleading for custom `Debug` implementations.

Add a default-implemented method on `TerminalRenderable`:

```rust
/// Returns the concrete implementer's Rust type name.
///
/// ## Notes
///
/// Used by the render-tree projector to produce stable diagnostic
/// labels when a component falls through to the rendered-then-stripped
/// fallback path. Components rarely need to override this.
fn type_name(&self) -> &'static str {
    std::any::type_name::<Self>()
}
```

Update `RenderableTerminalContent::to_tree_nodes` to use
`component.type_name()` in both the diagnostic message it returns and the
`tracing::warn!` it emits. Delete the `Debug`-based label heuristic.

#### S3-3b — Warn-once policy

Each concrete type emits the fallback warning **at most once per process**.
Implement via a `static` `OnceLock<Mutex<HashSet<&'static str>>>` (or
equivalent) keyed by `type_name()`. The first encounter logs at
`tracing::warn!`; subsequent encounters of the same type emit at
`tracing::debug!` (so the signal is still recoverable for anyone running
with `RUST_LOG=debug`).

#### S3-3c — Negative-path test

Add a single test that:

1. Constructs a deliberately un-overridden `TerminalRenderable`
   implementation in test code.
2. Projects it through `RenderableTerminalContent::to_tree_nodes`.
3. Asserts the resulting node carries the type-name label as documented.
4. Uses `tracing-subscriber` (or `tracing-test`) to capture the warning
   and assert exactly one `warn!` is emitted for the first call and a
   `debug!` for the second.

Explicitly **not** doing:

- **Strict / hard error.** Would break any third-party component that has
  not yet migrated.
- **`#[deprecated]` annotation on `render_tree_node`.** The method itself
  is not deprecated; only the *fallback* is undesirable.

### S3-4: Per-component inventory of `render_bespoke()` retention

Per the baseline table in the Context section, **not every component has a
public `render_bespoke` hook**. Some (Compose) have already retired theirs;
BlockQuote never had one (its bespoke is gated internally on
`has_default_border`); FileSystem never exposed one. Stage 3 treats
`render_bespoke` as an inventory exercise, not a uniform removal step.

**Per-component action table.** The "Action" column is the work to perform
during S3-4. The "Why retain (if applicable)" column documents the
terminal-only knob whose behavior the bespoke path backs.

| Component | Current state | Action | Why retain (if applicable) |
|-----------|---------------|--------|----------------------------|
| `BlockQuote` | No public `render_bespoke`; internal bespoke gated on `has_default_border` | Keep current shape; no action | `with_border(arbitrary)` escape hatch is internal — no test-visible hook needed |
| `Compose` | No `render_bespoke` (already retired) | No action; already done | — |
| `FileSystem` | No `render_bespoke`; `render` still bespoke (see S3-1c) | Decision deferred to S3-1c outcome | If S3-1c picks (i) flip: add `render_bespoke` for parity then retire after Stage 3. If (ii) or (iii) stay bespoke: leave as-is. |
| `OrderedList` | `pub fn render_bespoke` | **Retire** | No escape hatch |
| `Progress` | `pub fn render_bespoke` | **Retire** | No escape hatch |
| `Section` | `pub fn render_bespoke` + `_optimistic` | **Retire** | No escape hatch |
| `StatusBlock` | `pub fn render_bespoke`, used by gated `render` | **Keep `#[doc(hidden)] pub`** | `border(arbitrary)` escape hatch; `render()` calls into it on the non-default path |
| `Table` | `pub fn render_bespoke` | **Keep `#[doc(hidden)] pub`** | `prefer_cursor_alignment` + TTY routes here today; `render()` calls into it on the gated path |
| `TextBlock` | `pub fn render_bespoke` + `_optimistic` | **Retire** | No escape hatch |
| `Todo` | `pub fn render_bespoke` | **Retire** | No escape hatch |
| `TwoColumn` | `pub fn render_bespoke` + `_optimistic` | **Keep `#[doc(hidden)] pub`** | Image-overlay path; `render()` falls back here on `NodeKind::Unsupported` |
| `UnorderedList` | `pub fn render_bespoke` | **Retire** | No escape hatch |

#### S3-4a — Retire `render_bespoke` from six components

For `OrderedList`, `Progress`, `Section`, `TextBlock`, `Todo`,
`UnorderedList`:

- Delete `render_bespoke()` (and any `_optimistic` variant) and the
  supporting bespoke implementation function(s) the public hook backs.
  Note that the *trait method* `TerminalRenderable::render` and its
  helper `render_via_tree` stay; only the public `render_bespoke` hook
  goes away.
- Delete the component's `*_parity.rs` file or collapse it to
  tree-path-only structural checks (kind shape, attrs, hint payloads,
  no bespoke comparator).
- Update `lessons-learned.md` with a one-line note that the
  bespoke-vs-tree parity scaffold has been removed for the component.

#### S3-4b — Keep `#[doc(hidden)] pub fn render_bespoke` for three escape-hatch components

For `StatusBlock`, `Table`, `TwoColumn`:

- **Keep** `#[doc(hidden)] pub fn render_bespoke` as it is today. *Do
  not demote to `pub(crate)`.*
- The reason: each component's `*_parity.rs` test file lives under
  `biscuit-terminal/lib/tests/`, which is compiled as an integration
  test crate external to `biscuit_terminal`. Integration tests cannot
  reach `pub(crate)` items. Demotion would break exactly the parity
  tests these escape hatches need.
- Add a top-of-file comment to each retained `*_parity.rs` file naming
  the specific escape-hatch knob it covers (`border(arbitrary)`,
  `prefer_cursor_alignment`, image-overlay), so future readers know
  why bespoke parity is still meaningful.

#### S3-4c — `BlockQuote` and `Compose` need no action

`BlockQuote` gates its bespoke path internally on `has_default_border` and
never exposed a `render_bespoke` hook. Add a one-line comment near
`has_default_border` (or inside `block_quote.rs`'s top-of-file docs)
explaining the contract so the next reader does not "discover" a
missing-export bug.

`Compose` already retired its bespoke path. No action.

#### S3-4d — `FileSystem` action is tied to S3-1c

`FileSystem`'s bespoke status follows S3-1c:

- If S3-1c chooses (i) flip-to-tree: add `render_bespoke` during the
  parity sweep, then retire it together with the six S3-4a components
  once parity bakes in.
- If S3-1c chooses (ii) or (iii) stay-bespoke: no `render_bespoke`
  needed; the bespoke implementation backs `FileSystem::render`
  directly, and parity testing is whatever S3-1c specifies.

The S3-4d outcome must be recorded in `lessons-learned.md` next to the
S3-1c decision so the two stay linked.

### S3-5: Publish the IR migration checklist

Lessons-learned has identified a six-point recipe that every Stage 2 flip
followed without modification:

1. **Single private projection helper** shared by
   `TreeRenderable::render_tree` and `TerminalRenderable::render_tree_node`
2. **`render_via_tree`** with `tracing::error!` + empty / bespoke fallback
3. **`#[doc(hidden)] pub` (escape hatch) or retired `render_bespoke()`**
   depending on whether integration parity tests still need to compare a
   sanctioned terminal-only path (post-S3-4)
4. **Direct `BrowserRenderable` / `MarkdownRenderable` impls** that delegate
   to the canonical tree
5. **`bt X --md / --md-plus / --html / --example`** with `conflicts_with_all`
   for mutual exclusion
6. **Dedicated `*_parity.rs`** file (where the component has a kept bespoke
   surface) using the gated `render_bespoke()`

**Deliverable:** `renderable/docs/migrate-component-to-ir.md` documenting:

- **Flip-from-bespoke variant** — the Stage 2 pattern for legacy components
  with an existing terminal-only implementation.
- **Born-on-the-tree variant** — for components that start life as a tree
  projection with no bespoke implementation; skip points 3 and partially 2
  (no fallback needed when there is no bespoke path to fall back to).
- **Escape-hatch knobs** — when `render_bespoke()` is justified, when a
  `NodeKind::Unsupported` short-circuit is the right pattern, and how
  `RenderStrictness::{Lossy, Warn, Strict}` interact with infallible-trait
  fallbacks.
- **Cross-cutting helpers** — `project_renderable_content(content,
  ProjectionMode)` with `InlineOnly` and `Structural { terminal_hint }`
  variants, and the CLI helper home at
  `biscuit-terminal/cli/src/commands/shared.rs`.
- **Error-fallback policy** for infallible trait methods. Every Stage 2
  flip converged on the same shape, and the checklist must codify it so
  future migrations do not need to re-derive it:
  - **Terminal** (`TerminalRenderable::render` / `render_optimistic`) —
    log via `tracing::error!(component = "...", error = %error)` and
    return either `String::new()` or, where the component carries a
    documented terminal-only escape hatch, the bespoke fallback path.
    Never emit `[render-tree error: ...]` as in-band text.
  - **Markdown / MarkdownPlus** (`MarkdownRenderable::render_markdown` /
    `render_markdown_plus`) — log via `tracing::error!(component, dialect,
    error)` and return `String::new()`. Do **not** use
    `unwrap_or_default()`; the silent swallow is the anti-pattern.
  - **Browser** (`BrowserRenderable::render_html_fragment`) — log via
    `tracing::error!(component, error)` and return an empty fragment
    (`BrowserFragment::new().finalize()`). Do **not** return a fragment
    containing `[render-tree error: ...]` text.
- **Documentation-update obligations.** Every component migration must
  also update:
  - `renderable/docs/components.md` — the per-component row (Browser,
    Markdown, Tree columns, IR State, bt CLI).
  - The per-component doc under `biscuit-terminal/docs/components/<name>.md`
    (or create one if missing).
  - CLI help text and any `--example` output if `--md`, `--md-plus`,
    `--html`, or `--example` behavior changes.
  - The relevant skill under `.claude/skills/biscuit-terminal/` if the
    public component contract changes.

Reference this doc from `renderable/README.md` and from the renderable skill
under `.claude/skills/renderable/`.

**Note on Stage 3 scope.** S3-5 codifies the error-fallback and
documentation policies as forward-looking conventions for future
migrations. Stage 3 does **not** require auditing every Stage 2 component
for compliance with this policy — that cleanup is tracked separately in
the review-1 addendum's "Good After Stage 3" section.

### S3-6: Layout-matrix harness simplification

The `bespoke` → `via_render` rename landed in the Stage 2 closeout, but the
side-by-side snapshot layout is now redundant — both halves always render
through the tree path because every component is flipped. The matrix still
has value if it covers two *different* entry points.

**Decision:** keep the dual-column layout but rename for accuracy:

- Left column: **`via_render`** — `component.render(&term)` (the
  `TerminalRenderable::render` path that real consumers call).
- Right column: **`via_tree_direct`** — render the
  `TreeRenderable::render_tree(component)` node through
  `render_terminal_node` directly, bypassing `TerminalRenderable`.

The matrix then catches any drift between the two-trait entry points,
preserving its role as a regression net for the
`tree_renderable_and_compat_hook_share_one_projection` invariant.

**Escape-hatch policy.** Not every `component.render(&term)` call is
expected to equal `via_tree_direct`. The five sanctioned terminal-only
escape hatches deliberately route through bespoke code that the tree
cannot represent:

- `BlockQuote::with_border(arbitrary)` (custom prefix)
- `StatusBlock::border(arbitrary)` (custom prefix)
- `Table::prefer_cursor_alignment` (TTY only)
- `TwoColumn` image overlay (`NodeKind::Unsupported` short-circuit)
- `FileSystem::render` (only if S3-1c chooses outcome (ii) or (iii))

The matrix policy is:

1. **Default matrix cases must match.** Every default-configuration row
   in the layout matrix must show `via_render == via_tree_direct`. Any
   drift here is a real regression and must be fixed at the component
   level, not in the harness.
2. **Escape-hatch cases are excluded from `via_tree_direct` parity.**
   Cases that exercise an escape-hatch knob are either:
   - Omitted from the matrix entirely (preferred — keep the matrix
     focused on the default path), or
   - Included in a separate "terminal-only behavior" suite that does
     not run the `via_tree_direct` column.
3. **Any expected drift from sanctioned escape hatches is recorded
   separately** from ordinary render-tree regressions, with a comment
   naming the sanctioning spec section.

**Required default-case coverage.** The matrix must include a
default-configuration row for every tree-backed component whose
`TerminalRenderable::render` is expected to match `via_tree_direct`. Per
review-1's gap analysis, this minimum set is:

- `BlockQuote` (default `│ ` border only — `with_border(arbitrary)` excluded)
- `Compose`
- `OrderedList`
- `Progress`
- `Section`
- `StatusBlock` (default `┃ ` border only — `border(arbitrary)` excluded)
- `Table` (default path only — `prefer_cursor_alignment` excluded)
- `TextBlock`
- `Todo`
- `TwoColumn` (non-image / default path only — image overlay excluded)
- `UnorderedList`
- `FileSystem` **only if S3-1c chooses outcome (i)** (flip to tree)

Review 1 specifically flagged `OrderedList`, `TextBlock`, and `Todo` as
currently absent from the matrix; S3-6 must close those gaps.

**Tasks:**

- Rename the harness fields and snapshot column headers in
  `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs`.
- Add `ComponentCase` entries for every component in the required-coverage
  list above whose row is currently absent. Generate snapshots with
  `INSTA_UPDATE=always`.
- Audit current matrix cases for any escape-hatch usage; move or
  exclude them per the policy above.
- Regenerate snapshots only if any drift surfaces on the default-case
  rows (it should not — the parity tests would have caught it).
- Update the `KNOWN_DRIFT` ledger comment in `render_comparison.rs` to
  describe the new contract and reference the escape-hatch policy.

### S3-7: Workspace-wide cleanup of pre-existing failures and warnings

Items that Stage 2 deliberately left untouched per Rule 3 (Surgical Changes).
Stage 3 closes them out:

- **`test_prose_empty_errors_to_stderr`** in
  `biscuit-terminal/cli/tests/integration_test.rs` — currently fails because
  clap now rejects empty positional args at parse time before the runtime
  "No content provided" error path fires. Fix is a one-line test update:
  assert the clap error message instead of the runtime message, or remove
  the test if the runtime guard is genuinely unreachable.
- **Three clippy warnings:**
  - `biscuit-terminal/lib/src/components/prose/mod.rs:409` —
    `needless_borrows_for_generic_args`.
  - `biscuit-terminal/lib/src/components/prose/mod.rs:421` —
    `needless_borrows_for_generic_args`.
  - `biscuit-terminal/lib/src/discovery/detection/color.rs:255` —
    `clone_on_copy`.

After fixes, `cargo clippy --all-targets -- -D warnings` must be clean
across `renderable`, `biscuit-terminal`, and `biscuit-terminal-cli`.

#### S3-7a — `NO_COLOR` verification for tree-rendered CLI commands

Review 1 raised the concern (across `BlockQuote`, `OrderedList`, and
`Progress`) that commands which moved to the tree path may not honor
`NO_COLOR`. The fix, if needed, is **cross-cutting at the terminal
detection / rendering layer** — not per-component patches.

**Tasks:**

1. **Verify.** Add one CLI integration test against a tree-rendered
   command (e.g. `NO_COLOR=1 bt quote "text"` or
   `NO_COLOR=1 bt progress 50 --fill-color green`) asserting the output
   contains zero `\x1b[` bytes.
2. **If the test passes:** Record the verified behavior in
   `lessons-learned.md` and add **no** component-level patches. The
   review-1 concern is resolved.
3. **If the test fails:** Fix at the shared layer. Two acceptable
   approaches:
   - **(a)** Teach `Terminal::new()` /
     `detect_terminal_honoring_force_color()` to downgrade
     `ColorDepth::None` when the `NO_COLOR` environment variable is
     set (and unset / empty `FORCE_COLOR` is not overriding it).
   - **(b)** Teach the tree renderer's `apply_style` (or the shared
     `TerminalRenderOptions`) to suppress SGR emission when the
     terminal's color depth is `None`.
   - Option (a) is preferred because it benefits every consumer of
     `Terminal`, not just the tree renderer.

Explicitly **out of scope:** per-component `NO_COLOR` patches or
post-render ANSI-stripping in individual `bt` subcommands. Per the
review-1 addendum, those would collapse into the shared fix.

## Suggested phasing

| Phase | Items | Risk | Why phased this way |
|-------|-------|------|---------------------|
| **3a** | S3-1, S3-2 | Medium | The structural pivot; lands the user-visible improvement and tightens the parity tests in one go. |
| **3b** | S3-3 | Low | Tightens the safety net once S3-1 has soaked and no overrides are missing. |
| **3c** | S3-4, S3-6 | Low | Cleanup that benefits from S3-3 already gating new misses. |
| **3d** | S3-5 | None | Pure documentation; lands once 3a–3c stabilize so the recipe matches reality. |
| **3e** | S3-7 | Trivial → Low | Pre-existing failures and warnings are trivial; the S3-7a `NO_COLOR` verification may surface a real shared-layer fix that warrants its own review. |

Phases can be implemented sequentially by an orchestrator + subagent pair
following the Stage 2 pattern: implement → review → fix → next.

## Acceptance criteria

- [ ] `BlockQuote`, `StatusBlock`, and `FileSystem` carry
  `TerminalRenderable::render_tree_node` overrides that delegate to their
  `TreeRenderable::render_tree`, with matching parity tests (S3-1a).
- [ ] The audit of the other nine components is recorded in
  `lessons-learned.md` (S3-1b).
- [ ] The `FileSystem` terminal-`render` decision is recorded in
  `lessons-learned.md` as one of: (i) flip-to-tree with parity tests,
  (ii) permanent bespoke, or (iii) deferred to Stage 4 with a named
  follow-on criterion (S3-1c).
- [ ] Nested `BlockQuote`, `Section`, `Table`, etc. inside any container
  project to a structural `RenderNode` of the matching `NodeKind` — verified
  by tightened parity tests across all containers (S3-2).
- [ ] The Compose nested-component test exercises the fixture table in S3-2
  for every component marked "Valid as `Compose` part? Yes." Components
  excluded from that table are covered via direct projection tests.
- [ ] No `TODO(stage-3)` markers remain in `biscuit-terminal/lib/tests/`.
- [ ] `TerminalRenderable::type_name` exists as a default method returning
  `std::any::type_name::<Self>()` (S3-3a).
- [ ] `RenderableTerminalContent::to_tree_nodes` uses `component.type_name()`
  in both the diagnostic and the `tracing::warn!` it emits; the `Debug`-
  based label heuristic is gone (S3-3a).
- [ ] The fallback warning fires at most once per concrete type per process
  at `warn!` level; subsequent encounters log at `debug!` (S3-3b).
- [ ] A negative-path test pins the warn-once-then-debug behavior with a
  captured-logs assertion (S3-3c).
- [ ] `render_bespoke()` (and any `_optimistic` variant) is removed from
  `OrderedList`, `Progress`, `Section`, `TextBlock`, `Todo`, and
  `UnorderedList`; their `*_parity.rs` files are deleted or collapsed to
  tree-path-only checks (S3-4a).
- [ ] `render_bespoke()` is **kept** as `#[doc(hidden)] pub fn` on
  `StatusBlock`, `Table`, and `TwoColumn`; each retained `*_parity.rs` has
  a top-of-file comment naming the escape-hatch knob it covers (S3-4b).
- [ ] `BlockQuote` carries a one-line comment near `has_default_border`
  documenting why no public `render_bespoke` exists (S3-4c).
- [ ] `FileSystem` action records the S3-1c outcome and any matching
  `render_bespoke` work alongside it in `lessons-learned.md` (S3-4d).
- [ ] `renderable/docs/migrate-component-to-ir.md` exists, covers both
  flip-from-bespoke and born-on-the-tree variants, codifies the
  error-fallback policy (terminal / Markdown / Browser), lists the
  documentation-update obligations, and is referenced from
  `renderable/README.md` and the renderable skill (S3-5).
- [ ] The layout-matrix harness compares `via_render` against
  `via_tree_direct`; default-case rows exist for every component in the
  required-coverage list (S3-6); escape-hatch cases are either excluded
  from the matrix or routed to a separate "terminal-only behavior" suite.
- [ ] `OrderedList`, `TextBlock`, and `Todo` have default-case rows in the
  layout matrix where they were previously absent (S3-6).
- [ ] `cargo test -p renderable -p biscuit-terminal -p biscuit-terminal-cli`
  is fully green; `test_prose_empty_errors_to_stderr` is fixed or retired
  (S3-7).
- [ ] `cargo clippy --all-targets -- -D warnings` is clean across all three
  packages (S3-7).
- [ ] A `NO_COLOR=1` CLI integration test against at least one tree-rendered
  command (`bt quote` or `bt progress`) asserts zero `\x1b[` bytes; if the
  test failed at first run, the shared `Terminal` / detection-layer fix is
  recorded in `lessons-learned.md` and no per-component patches were added
  (S3-7a).
- [ ] `lessons-learned.md` carries a closing "Stage 3 complete" section that
  cross-references the migration checklist as the canonical onward-path
  document.

## Out of scope

Stage 3 does **not**:

- Introduce new `NodeKind` variants — the existing vocabulary is sufficient.
- Change the public Browser / Markdown / Terminal trait surfaces beyond
  the single new default method `TerminalRenderable::type_name` (S3-3a).
- Migrate any new component (the twelve are the universe; born-on-the-tree
  components are a future concern, covered only by the checklist).
- Fold `BlockQuote::with_border(arbitrary)`, `StatusBlock::border(arbitrary)`,
  `Table::prefer_cursor_alignment`, or `TwoColumn` image-overlay into the
  tree — these remain sanctioned terminal-only escape hatches.
- **Commit** to flipping `FileSystem::render` to the tree path. S3-1c is a
  *decision*, not a foregone conclusion; outcome (ii) "permanent bespoke"
  and (iii) "defer to Stage 4" are explicitly acceptable.
- Touch the `darkmatter` rendering pipeline or any other consumer of
  `renderable` outside the twelve `biscuit-terminal` components.

## Risks and mitigations

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| S3-1a override on `BlockQuote` / `StatusBlock` reveals projection drift from existing terminal output | Medium | Add the parity test first; any failure is a legitimate Stage 2 gap and gets fixed in the same change before claiming S3-1a done. |
| S3-1c decision (FileSystem flip vs stay) is reversed late, after S3-4d work has already been done | Low | S3-1c is the gate for S3-4d; do not begin S3-4d work for `FileSystem` until S3-1c is recorded in `lessons-learned.md`. |
| S3-3 warn-once policy hides a real regression after first encounter | Low | Subsequent encounters still emit at `debug!`; CI runs with `RUST_LOG=debug` for the renderable test suite, so the signal is recoverable. |
| S3-4a removal of `render_bespoke` breaks a downstream consumer reaching through `#[doc(hidden)]` | Low | Workspace-grep external consumers in the monorepo before deletion; deprecate-then-remove if any are found. Even hidden symbols should not vanish under unaudited downstream code. |
| S3-4b retained `render_bespoke` hooks drift from the real escape-hatch behavior over time | Low | Each retained `*_parity.rs` has a top-of-file comment naming the knob it covers; future changes that touch the knob must verify the parity test still asserts the right contract. |
| S3-6 reveals real drift between `via_render` and `via_tree_direct` on default cases | Low | The Stage 2 parity tests would have caught this; treat any default-case drift as a regression and fix at the component level, not in the harness. |
| S3-6 escape-hatch case is accidentally left in the default matrix and `via_tree_direct` parity fails | Medium | The migration checklist (S3-5) documents the policy; the matrix audit step in S3-6 explicitly checks for escape-hatch usage in each row. |
| S3-7a `NO_COLOR` verification fails and the shared-layer fix touches `Terminal` construction more broadly than expected | Medium | Scope the fix to `detect_terminal_honoring_force_color` and document the new precedence (`NO_COLOR` set → `ColorDepth::None`, unless `FORCE_COLOR` overrides) in the same change. Add the precedence rule to `lessons-learned.md` so future contributors do not re-litigate it. |

## Success metric

Stage 3 succeeds when a new contributor can pick up a thirteenth component
and migrate it to the IR using `migrate-component-to-ir.md` as the only
reference, without consulting `lessons-learned.md` or any of the twelve
worked examples.
