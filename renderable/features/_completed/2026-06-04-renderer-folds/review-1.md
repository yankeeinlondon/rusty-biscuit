---
ready: false
agent: codex
model: ""
---

# Review: Renderer Folds

## Findings

### High: `Fixed` and capped `Auto` boxes are aligned inside themselves, not within the parent content area

The spec requires every sub-available painted box to be placed within `available - margin` for all three width modes. The implementation only uses the parent area for `FitContent`; `Fixed` and `Auto` use `content_width + painted_padding` as their alignment basis (`biscuit-terminal/lib/src/render_tree/render.rs:355-364`). Consequently, an 80-column parent with `Fixed(20)` and centered content produces a 9-cell lead, as the new test explicitly expects, instead of centering the 20-column box in the 80-column parent (`render.rs:3413-3434`). `Auto` has the same defect whenever `max_width` makes it sub-available.

Compute the painted box width once, derive slack from the full area between margins for every width mode, and add tests for left/center/right across `Auto + max_width`, `Fixed`, and `FitContent`.

Verification level: L1 currently encodes the wrong behavior. No L2 capture verifies placement in a real terminal.

### High: terminal sizing does not preserve content-box width or percentage-padding semantics

The outer fold resolves horizontal padding against the parent width (`render.rs:296-300`), but the nested styled render resolves it again against the narrowed `content_width` (`render.rs:203-206`, `render.rs:237-245`). Percentage padding therefore changes basis. For example, 10% horizontal padding in an 80-column parent is budgeted as 8 cells per side, then repainted as a percentage of the narrowed box instead of the parent.

Border overhead is also subtracted after `Fixed(n)` has been selected (`render.rs:195-201`, `render.rs:306-310`). A `Fixed(20)` node with left and right borders renders only 18 content columns, although the spec defines `used` as the content width and requires border and padding to be added around it. Finally, padding is skipped entirely when the node has no non-empty `Style` because `render_styled` returns before padding is applied (`render.rs:190-193`). Transparent CSS padding must still reserve cells.

Resolve margin, padding, border, and content width once from the parent basis, pass the resolved padding into the paint step, and apply padding even when `Style` is empty. Add a hand-computed matrix covering percentage padding, borders with `Fixed`/`FitContent`, no-style padding, narrow clamps, and `max_width`.

Verification level: partial L1 only; the current tests cover background-painted `ch` padding and unbordered widths, not these combinations. No L2 coverage exists.

### High: browser lowering omits the required `box-sizing:content-box`

`layout_to_css` emits padding and width declarations (`renderable/src/tree/render/browser.rs:2508-2529`), and border declarations are assembled separately, but `node_attributes` only concatenates those strings (`browser.rs:2255-2273`). Nothing emits `box-sizing:content-box` when width, padding, or border is non-default. A global `* { box-sizing:border-box }` reset therefore changes the renderable width contract, directly failing acceptance criterion 5 and the hostile-stylesheet test requirement.

Add `box-sizing:content-box` during final per-node declaration assembly whenever non-default width, padding, or border is present, plus an HTML fixture containing a hostile reset and assertions on the generated inline style.

Verification level: no test currently covers this requirement. The new browser tests only assert padding and width tokens (`browser.rs:3952-3984`).

### High: the terminal user-observable requirements have no Level 2 verification

Acceptance criterion 1 explicitly requires a real-terminal check for painted padding. Criteria 2-4 also assert terminal-observable cell widths, alignment, border adjacency, glyph placement, and background painting, for which Level 2 is the appropriate verification tier. The L2 suite invokes foreground, background, emphasis, and legacy border helpers only (`biscuit-terminal/cli/tests/level2_render_tree_style.rs:589-634`); it has no padding, width-mode, alignment, or no-implicit-gap case. This change also removes the previous real-terminal fill-band checks without replacing them with padding-based equivalents.

Add L2 fixtures in at least one styling-capable terminal for padding/background SGR coverage and pane-text captures for `Auto`/`Fixed`/`FitContent`, center/right placement, and border adjacency. A tmux text capture is sufficient for geometry and glyph adjacency; WezTerm or Kitty is needed for painted background cells.

Strongest verification by requirement:

| Requirement | Strongest present | Required | Result |
|---|---|---|---|
| Terminal painted padding and transparent margin | L1 | L2 | Gap |
| Terminal width modes, caps, and box-order clamp | L1 | L2 | Gap |
| Terminal center/right alignment | L1 | L2 | Gap |
| Terminal border adjacency and explicit padding gap | L1 | L2 | Gap |
| Browser padding/width lowering | L1 | L1 | Partial; `box-sizing` missing |
| Browser border matrix | L1 | L1 | Partial matrix coverage |
| Borrowed-accessor performance invariant | L1 perf gate | L1 perf gate | Pass |
| Darkmatter unchanged | Source inspection | Source inspection | Pass |
| Skill and documentation updates | Source inspection | Source inspection | Pass, but currently overstates completeness |

### Medium: `BlockQuote` overwrites caller-supplied left padding

The migration unconditionally replaces `self.layout.padding.left` with `1ch` (`biscuit-terminal/lib/src/components/block_quote.rs:653-655`). A caller that deliberately configured a larger, percentage-based, or target-specific left padding silently loses that setting in both terminal and browser output. Preserve explicit caller padding and introduce the compatibility gap only for the component's default layout, or define and test an additive policy.

## Verification

- `cargo test --color=never -p renderable tree::render::browser`: 108 passed.
- `cargo test --color=never -p biscuit-terminal render_tree`: 193 library render-tree tests passed; focused parity subsets also passed.
- `cargo test --color=never -p biscuit-terminal --test perf_gate`: 2 passed, including zero renderable-owned hint round-trips.
- No L2 run can exercise the new behavior because the suite contains no corresponding cases.

## Recommendation

Do not ship this feature yet. Correct the terminal content-box calculation and alignment basis, add browser `box-sizing:content-box`, preserve custom `BlockQuote` padding, and add the missing L2 terminal coverage before marking the renderer folds production-ready.
