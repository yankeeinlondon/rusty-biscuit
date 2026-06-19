---
ready: false
agent: codex
model: ""
---

# Review: Tree Features

## Findings

### High: Exact-width link and image content is no longer truncated

The spec requires exact width, maximum width, alignment, padding, and truncation
for hyperlink labels and image placeholders. It also preserves the distinction
that `width` establishes an exact field while `max_width` is only a ceiling
([spec.md](spec.md:368)).

Darkmatter attaches every hyperlink/image `width` hint with
`TextOverflow::Preserve`
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:400)).
The terminal fold only truncates content wider than an exact `width` when the
hint says `Truncate`
([render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:886)).
Consequently, a long label under `style.hyperlinks.width: 5` exceeds the
five-column field. The retired implementation truncated long content for an
exact width, so this is a behavior regression rather than an unspecified
overflow choice
([bespoke.rs](../../../darkmatter/lib/src/style/bespoke.rs:431)).

The current Level 1 tests cover short exact-width padding and long
`max_width` truncation, but not long exact-width truncation
([tree_features_characterization.rs](../../../darkmatter/lib/tests/tree_features_characterization.rs:275),
[render.rs](../../../biscuit-terminal/lib/src/render_tree/render.rs:3053)).
The Level 2 suite likewise verifies only short-label padding
([level2_layout.rs](../../../darkmatter/cli/tests/level2_layout.rs:2333)).
Add Level 1 regressions for long link labels and image alt text under exact
width, then add Level 2 real-terminal capture proving the visible field does
not overflow.

### High: Per-image structured CSS is still emitted as literal title text

`parse_image_directive` constructs a plain `ImageRef`, calls `with_title`, and
then reads `ImageRef::style`
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:603)).
`with_title` only stores title text; it does not parse the structured directive
([image_ref.rs](../../../darkmatter/lib/src/render/image_ref.rs:727)).
The resulting snapshot therefore contains the frontmatter red style while
leaking `style='color: blue;'` into the HTML `title`, instead of merging the
per-node blue declaration with per-node precedence
([snapshot](../../../darkmatter/lib/tests/snapshots/tree_features_characterization__char_local_image_inline_css_precedence_browser.snap:5)).

This fails the required structured image directive and validated
`inline_style` behavior. The characterization test's comment still describes
this as pre-cutover output even though the snapshot was reaccepted during the
cutover
([tree_features_characterization.rs](../../../darkmatter/lib/tests/tree_features_characterization.rs:252)).
Parse the complete Markdown image through the existing `ImageRef` parser (as
the production Markdown helper already does), populate typed image/browser
attrs, and assert that raw directive syntax is absent.

The strongest current verification is a Level 1 snapshot that approves the
wrong output. Add a structural Level 1 assertion for the initial tree and a
real-browser computed-style test proving that per-node CSS wins over
frontmatter defaults.

### High: Browser document folds discard root inheritance and construction copies page colors onto components

The spec requires page-level inheriting text appearance on the root, normal
renderer traversal through `InheritedStyle`, no copied page colors on component
nodes, and a completed `Document` that can be passed directly to every
renderer. The implementation does the opposite in two places:

- `component_color` and `component_bg_color` fall back to page colors, and
  `apply_component_color` writes those resolved values onto each component
  ([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:63),
  [build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:229)).
- Both browser document writers skip a `Root` node and render only its children,
  so the root's style is neither emitted nor threaded into descendants
  ([browser.rs](../../../renderable/src/tree/render/browser.rs:150),
  [browser.rs](../../../renderable/src/tree/render/browser.rs:209)).

The generic inheritance contract only carries foreground color and emphasis;
background deliberately does not inherit
([inherit.rs](../../../renderable/src/tree/inherit.rs:5)). However,
`apply_page_colors` claims both root foreground and background inherit
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:152)).
Darkmatter's retained browser page wrapper separately emits page foreground and
background CSS, masking the missing direct-document behavior in its public page
path
([page.rs](../../../darkmatter/lib/src/layout/page.rs:1398)).

Remove page fallback from component policy resolution, make the browser folds
honor root text inheritance, and keep page background on the retained page
frame. Add Level 1 direct-document tests for root foreground inheritance and
absence of copied component colors, plus a real-browser computed-style test.
The existing Level 1 test only checks that root attrs were populated, while the
browser test checks the separate wrapper path
([build_context.rs](../../../darkmatter/lib/src/markdown/render_tree/build_context.rs:779),
[page.rs](../../../darkmatter/lib/src/layout/page.rs:2778)).

## Verification Levels

| Requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Browser alpha lowering | Real-browser computed-style tests | Appropriate |
| Terminal alpha degradation | Level 1 byte/output equivalence, with Level 2 color coverage for related component paths | Appropriate for degradation; no input encoder behavior is involved |
| Exact/max link and image width | Level 1 plus Level 2 for short exact-width padding and max-width cases | Gap: long exact-width overflow is untested and broken |
| List-item placement | Level 1 plus Level 2 real-terminal capture | Appropriate |
| Structured link attributes | Level 1 tree/HTML snapshots and browser-fold parity | Appropriate for deterministic attributes |
| Structured image CSS precedence | Level 1 snapshot only, asserting incorrect output | Gap: needs structural Level 1 and real-browser computed style |
| Root page-style inheritance in direct browser folds | Root-attr Level 1 test and separate page-wrapper browser test | Gap: direct fold behavior is neither tested nor implemented |
| Keyboard, mouse, paste, IME, or hotkey behavior | Not applicable | No Level 3 requirement |

## Verification

- `cargo test -p renderable --color=never`: passed.
- `cargo test -p biscuit-terminal --color=never`: passed.
- `cargo test -p darkmatter --color=never`: passed, including browser and
  `cutover_reference` tests.
- Level 2 terminal suites were inspected but not executed; repository guidance
  requires running them through the package `just test-l2` harness.

The requested `root` skill is not present in the advertised skill catalog or
local skill directory. This review used the required `renderable` skill,
`rust-testing`, and the repository-root instructions supplied for this session.

## Readiness

Not ready for production. The implementation passes the Level 1 and cutover
suites, but three user-facing or architectural requirements remain broken, and
the missing higher-level tests currently allow those regressions to pass.
