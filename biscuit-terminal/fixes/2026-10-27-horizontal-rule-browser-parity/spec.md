---
kind: fix
created: 2026-10-27
status: draft-spec
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-27
review_iterations: 2
completed: true
clarified: false
implemented: true
implemented_by: "claude/opus"
area: biscuit-terminal
human_review: false
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
---

# HorizontalRule: remove the legacy browser SVG and revive the tests that hid it

`cargo check --workspace --tests --all-features` fails in `biscuit-terminal`:

```text
error[E0433]: cannot find type `HorizontalRule` in this scope
   --> biscuit-terminal/lib/src/components/horizontal_rule/mod.rs:2863:24
error[E0061]: this function takes 7 arguments but 6 arguments were supplied
   --> biscuit-terminal/lib/src/components/horizontal_rule/mod.rs:2865:26
```

The feature-gated module contains thirteen tests that ordinary local and CI
feature selections omit. Enabling `serde` exposes compile errors before those
tests can run. Eight compare duplicate browser SVG implementations, one checks
the terminal fallback, and four check serialization. All three kinds of
coverage need an explicit disposition.

The goal is one browser rendering path and working serialization coverage.
This specification changes browser behavior to match the existing shared
renderer; it does not redesign terminal rendering.

## Source contracts

The relevant APIs and their owners are:

| API | Package and purpose |
| --- | --- |
| [HorizontalRule](../../lib/src/components/horizontal_rule/mod.rs) | `biscuit-terminal`: configurable separator; its tree projection preserves style, alignment, weight, width, color, and outer layout. |
| [BrowserRenderable](../../../renderable/src/browser/mod.rs) | `renderable`: public fragment and page rendering contract. |
| [BrowserTreeComponent](../../lib/src/render_tree/browser_adapter.rs) | `biscuit-terminal`: adapts a tree-producing component to browser output with the existing non-panicking error policy. |
| [ThematicBreakAttrs and HrAlignment](../../../renderable/src/tree/attrs.rs) | `renderable`: typed separator appearance and alignment passed between components and renderers. |
| [horizontal_rule_svg](../../../renderable/src/tree/graphics.rs) | `renderable`: shared SVG geometry, alignment, and width/color validation. |
| [Browser tree renderer](../../../renderable/src/tree/render/browser.rs) | `renderable`: lowers the separator and applies its outer layout. |

Paths below are relative to the `biscuit-terminal` area unless stated otherwise.

## What is wrong

### 1. Nine tests are nested in a feature-gated module

In `lib/src/components/horizontal_rule/mod.rs`, the test module
`#[cfg(feature = "serde")] mod serde_roundtrip` (line ~2797) imports only
`RuleAlignment`, `RuleStyle`, and `RuleWeight`. Nine tests that have nothing to
do with serde sit inside it:

| Test | Purpose |
| --- | --- |
| `parity_default_dashes` | `renderable` SVG == `HorizontalRule::render_browser_svg` |
| `parity_waves_thick_blue` | same |
| `parity_dots_thin_custom_width` | same |
| `parity_line_star` | same |
| `parity_line_circle` | same |
| `parity_inset_line` | same |
| `parity_curtain_rod` | same |
| `parity_with_custom_margins` | same |
| `render_text_tier_matches_render_when_image_unavailable` | the text tier equals `render()` when images are unavailable |

Without `serde` they are compiled out. With `serde` they fail to compile:
`HorizontalRule`, `ScopedLcAll`, and `text_terminal_with_width` are not in
scope.

### 2. The parity calls are one argument behind

On 2026-06-08 (`8c40526eb`), `renderable::tree::graphics::horizontal_rule_svg`
gained an `alignment: Option<HrAlignment>` parameter (third position, seven
in total). The eight `parity_*` tests still pass six arguments. Several also
pass strings where the current function accepts typed `HrKind` and `HrWeight`
values. Adding the missing argument alone would not restore these tests;
removing the duplicate renderer makes these comparisons unnecessary.

### 3. Declared test features omit serialization coverage

`lib/Cargo.toml`'s `[package.metadata.ci.tests]` sets `features = ["image",
"terminal-tests", "browser-tests"]`, and every `just` recipe passes
`--features image`. The `justfile` has carried this since 2026-07-28:

> NOT `--all-features`: the `serde` feature's test module does not compile
> (its `renderable::tree::graphics::horizontal_rule_svg` calls are one
> argument behind that function's signature) …

The four serialization tests (`test_rule_style_serde_roundtrip`,
`test_rule_style_serialized_as_kebab_case`,
`test_rule_alignment_serde_roundtrip`, `test_rule_weight_serde_roundtrip`)
are therefore absent from the declared CI feature selection as well. The same comment block also says `image` gates
the "horizontal-rule parity suites", which is wrong: `serde` gates them.

### 4. A superseded browser path was never removed

When `renderable` gained the typed render tree, the rule's browser SVG moved
into `renderable::tree::graphics::horizontal_rule_svg`, and existing
`biscuit-terminal` components already demonstrate browser rendering through
`BrowserTreeComponent::new(self.clone())`, as `Prose` does. `HorizontalRule` was left with its original implementation:
`render_browser_svg` in `horizontal_rule/browser.rs`, which
`impl BrowserRenderable for HorizontalRule` emits as a raw-HTML island. The
`parity_*` tests were the stopgap that kept the two copies equal, and once they
stopped compiling the copies drifted. Since `8c40526eb`:

| | render tree (`renderable`) | `render_browser_svg` (legacy) |
| --- | --- | --- |
| Horizontal margins | follow alignment: `Left` → `0 auto`, `Right` → `auto 0`, `Full` → `0 0`, `Center` → `auto auto` | always `auto auto` (centered) |
| `Full` width | always `100%` (documented `HrAlignment::Full` cross-target contract) | the authored `width` |
| `width` / `color` | checked by `is_safe_css_dimension` / `is_safe_css_color`; unsafe values fall back to `100%` / `currentColor` | interpolated into the attribute and inline `style` unchecked |
| Outer `Layout` | placed by the tree renderer, as on Terminal | top/bottom margins only, via `MarginToCss` |

So the same rule renders differently depending on how it reaches the browser:

```rust
let rule = HorizontalRule::new()
    .alignment(RuleAlignment::Left)
    .width("50%");
// render tree:        style="… margin: 0 auto 0 0; …"   (left-anchored)
// BrowserRenderable:  style="… margin: 0 auto 0 auto; …" (centered — wrong)
```

and `BrowserRenderable` writes a `width` or `color` such as
`50%" onload="…` into the HTML unchanged.

## Fix

Only the render-tree path may exist. Remove the legacy one; do not repair it.

1. **Delete the legacy browser path.** Remove `render_browser_svg` and the
   `MarginToCss` trait (it has no other caller) from
   `horizontal_rule/browser.rs`, along with their tests: the eight `parity_*`
   tests and the `to_css_value` helper tests. Migrate every ungated test
   calling `render_browser_svg()` to the public fragment output where it
   covers observable behavior (style geometry, weight, color variables,
   margins). Drop only checks whose sole purpose was comparing the two
   copies. Preserve terminal assertions in mixed terminal/browser tests.
   Review the existing browser snapshots, regenerate only those changed by
   the intended browser behavior, and remove snapshots only when their
   owning tests are removed. Existing width assertions using the default
   `Full` alignment must either expect `100%` or explicitly select a
   non-`Full` alignment when testing authored width.

2. **Render `BrowserRenderable` from the tree.** `impl BrowserRenderable for
   HorizontalRule` returns `BrowserTreeComponent::new(self.clone())
   .render_html_fragment()`, as `Prose` does for its layout case. Its doc
   comment says the output is the render tree's `ThematicBreak` rendering.
   The adapter uses default browser options: `GraphicsMode::Rich`, which
   emits SVG. It has no graphics-mode setter. Callers needing plain `<hr>`
   with `data-hr-*` attributes use the lower-level tree renderer with
   `GraphicsMode::Off`; no new component option is introduced. Preserve
   `as_any()` returning the original `HorizontalRule`, and keep existing
   page rendering and CSS-variable substitution APIs working through the
   delegated fragment. This fix does not change the shared renderer or its
   validation rules.

3. **Keep the terminal color helpers.** `parse_basic_color`,
   `parse_hex_color`, and `nearest_basic_color` live in `browser.rs` but serve
   the terminal renderer (`mod.rs` ~722). Move the helpers to `color.rs`
   and keep the browser impl in `browser.rs`; update imports and the module
   doc at the top of `mod.rs`, which lists `MarginToCss`. This keeps terminal color
   parsing independent of browser rendering without relocating the impl
   into the already large parent module. Preserve helper behavior and tests.
   Correct the tree projection's stale comment that browser output is
   always `<hr>`. Review the terminal unknown-color warning and its test:
   it must no longer promise that arbitrary raw color strings reach the
   browser. State that browser output validates colors independently;
   valid browser-only colors such as `tomato` still work. Terminal drawing
   and color selection remain unchanged.

4. **Ungate the remaining tests and test the browser contract.** Move
   `render_text_tier_matches_render_when_image_unavailable` out of
   `serde_roundtrip` into the ungated test module. Add
   `#[serial_test::serial(locale_env)]` while retaining its existing locale
   guard, matching nearby tests that change the same process-wide variables.
   Leave only the four serialization tests under `cfg(feature = "serde")`.
   Add ordinary L1 tests on `render_html_fragment().render()` (the HTML
   string, rather than comparing fragment objects):

    - compare against the adapter's rendered HTML for a rule setting all
      fields, including nondefault outer layout. This is a routing check;
      the independent assertions below establish behavior.
    - check `Left`, `Right`, and `Centered` at `width("50%")` retain that
      width and have the expected automatic margin on the opposite side or
      both sides. Check `Full` uses `100%` and zero horizontal margins even
      when given `50%`.
    - check outer top/bottom and side margins, width, maximum width, and
      alignment through the existing tree layout contract; distinguish the
      outer wrapper from the SVG's intrinsic alignment. Include a
      browser-specific margin differing from the terminal value and check
      it is resolved for the browser and applied once. A default layout
      must add no outer wrapper.
    - test unsafe width and color independently using a non-`Full`
      alignment, so forced full width cannot hide a validation failure.
      Include a quote/attribute injection value and a semicolon/CSS
      declaration injection value. Assert the input is absent, no injected
      attribute or declaration exists, and the respective fallback is
      `100%` or `currentColor`. Retain positive coverage for safe authored
      width and color so rejecting every value cannot pass.
    - preserve coverage of all seven rule styles and three weights, and
      verify composing the public fragment into an HTML page still emits
      the rule. Reuse existing tests where they already prove this.

   These tests need no running browser or terminal window. Do not give
   string-only L1 tests a `browser_` prefix: that prefix selects the
   headless-browser tier in this repository. Tests invoking the actual
   browser harness retain their tier and run headlessly without focus.

5. **Run the `serde` tests.** Add `serde` to the library's L1 features in
   `[package.metadata.ci.tests]` (`features` and `local-features`) and to the
   `justfile` recipes that pass `--features image`. Remove the "NOT
   `--all-features`" paragraph, and correct the `image` comment so it no
   longer claims to gate the parity suites. `cargo check -p biscuit-terminal
   --tests --all-features` must succeed. Keep `serde` opt-in for production;
   this changes test feature selection only. Update the adjacent Cargo
   policy comment to say L1 includes `image` and `serde`. Preserve the
   existing `terminal-tests` and `browser-tests` feature gates and declared
   tiers; no CI job, environment, or duplicate suite is added.

## Behavior change to note in docs

`HorizontalRule` rendered through `BrowserRenderable` now looks exactly as it
does on the render-tree path. In particular `HorizontalRule::new()` defaults
to `RuleAlignment::Full`, which on the tree has no side margins and always
spans `100%`, so a `Full` rule given a custom `width` fills its available
outer box in the browser. An explicit outer layout width or maximum width can still constrain
that box. Update `biscuit-terminal/docs/components/horizontal-rule.md` and the
`biscuit-terminal` skill's components reference wherever they describe
the old browser output. Preserve documentation for the SVG CSS variables:
those remain supported by the shared renderer. Correct examples that expect
custom width with default `Full` alignment by selecting `Centered`, `Left`,
or `Right`. Explain the separate roles of rule alignment and outer layout,
and document invalid width/color fallback rather than promising raw color
pass-through. Where touched, replace stale `Renderable` trait names with
`TerminalRenderable` and remove the claim that terminal images rasterize the
browser SVG: the terminal image path builds its own SVG.

**Reader's note:** consolidating rendering intentionally changes existing
browser markup and layout, including full-width behavior and placing outer
margins on a wrapper. Keeping legacy markup would preserve the duplicate
implementation and its unsafe interpolation. Consumers should use the public
rendering API and the documented CSS variables rather than depend on the
old wrapper shape. No public builder or trait method is removed; the deleted
SVG method and margin trait are crate-private.

## Out of scope

The terminal renderer honors an authored `width` on a `Full` rule (flush left,
not stretched), while the browser contract forces `100%`. That is a
cross-target inconsistency in `renderable`'s `Full` contract, not part of this
fix; terminal drawing is unchanged. Only the warning wording described above
may change.

## Verification

During implementation, run from the package area:

- `just test`: ordinary L1 coverage for the library and CLI, now including
  the four serialization tests and relocated terminal test. Inspect the test
  listing/results to confirm all five are selected, rather than inferring
  coverage from a green exit alone.
- `cargo check -p biscuit-terminal --tests --all-features`: verify the
  previously broken feature combination compiles, including higher-tier
  test code without launching it.
- `cargo check -p biscuit-terminal --no-default-features`: preserve the
  no-feature library build; enabling serialization for tests must not make
  it mandatory for users.
- `just lint` and `just doctest`: check source and the updated public examples.

Reuse existing L1 terminal and page-composition tests. No new higher-tier
suite or performance spike is needed for removing a duplicate SVG generator.
If existing harness tests are changed, run their owning recipe
(`just test-browser` or `just test-l2`); keep browsers headless and terminal
windows from gaining focus. Keep validation scoped to this package area;
the workspace-wide command above describes the original discovery, not a
required verification run.

## Open Questions

None. The shared renderer's existing behavior decides the browser contract;
the known terminal/browser difference for `Full` remains explicitly outside
this change.

## Acceptance

- `render_browser_svg` and `MarginToCss` no longer exist; the only browser
  rendering of a horizontal rule is `renderable`'s tree renderer.
- `cargo check -p biscuit-terminal --tests --all-features` compiles.
- `just test` in `biscuit-terminal` runs the serde tests and the relocated
  test, and all pass.
- All four alignments follow the existing shared browser contract, and
  browser-specific outer layout is applied exactly once.
- Unsafe `width`/`color` values fall back safely through the public fragment;
  safe values, CSS variables, all styles/weights, and page composition retain
  coverage.
- The no-feature library build remains supported; serialization stays opt-in
  outside the local/CI test selection.
- Browser snapshots, component docs, relevant source comments, and skill
  guidance agree with the delegated browser path. Terminal rendering tests
  continue to pass without changing their expected output.
