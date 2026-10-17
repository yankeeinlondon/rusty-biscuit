# Browser Render Testing

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Use the Browser tier (`browser_*` tests) when a crate emits HTML/CSS and you must
prove the browser *applies* it. Tests drive a real headless Chrome/Chromium over
CDP through `biscuit-browser-harness` (`ChromeHarness`, the `BrowserHarness`
trait, `find_chrome`, `wrap_fragment`, `require_browser`). The worked example is
`darkmatter/lib/tests/browser/browser_render.rs`. Run the tier with
`just test-browser` (`-j 1`, one Chrome at a time); gating and
`BISCUIT_BROWSER_REQUIRED=1` are in [tiers-and-gating.md](tiers-and-gating.md).

## Headless and focus-isolated

A `browser_*` test never creates or activates a visible window, requests focus,
moves the pointer, or injects OS input: no headed flags, `osascript`, `cliclick`,
`xdotool`, `SendInput`, or similar. It must be safe to run while someone uses the
machine. Drive keyboard, pointer, media queries, and viewport through CDP (the
harness wraps key/mouse dispatch and media emulation) and assert on the live DOM,
computed styles, accessibility state, or geometry.

Use Level 3 only when the requirement is the OS-to-application input path itself.
Such a test lives outside Browser-tier binaries, is opt-in via `RUN_LEVEL3=1`, and
is never added just to strengthen a DOM test that CDP already covers.

## Assertion rules

- **Assert what the browser computed**, via `computed_style(selector, property)`,
  not a substring of the emitted HTML: reading back the resolved value proves the
  CSS is valid and applied.
- **No pixel assertions** in the tier; screenshots vary by font rendering and OS.
  Reserve them for opt-in baselines pinned to one OS or container.
- **Wrap fragments** with `wrap_fragment(fragment, body_background)` before
  rendering: a fragment has no page background, so contrast and layout are
  meaningless without one.
- **Skip cleanly** when `ChromeHarness::available()` is false; `CHROME=/path`
  overrides discovery.

```rust
let mut h = ChromeHarness::new();
h.spawn().await?;
h.render_html(&wrap_fragment("<div class='x'>hi</div>", "#fff")).await?;
assert_eq!(h.computed_style(".x", "background-color").await?, "rgb(17, 27, 39)");
```

## Visual checks

For a one-off "render it and let me look", use a screenshot-to-file `examples/`
binary such as `darkmatter/lib/examples/html_to_png.rs` (`cargo run -p <crate>
--example html_to_png -- input.md out.png`) rather than a browser MCP server; it
adds no persistent tool surface or accessibility-tree dumps. Use an interactive
browser tool only for multi-step navigation of a live app.

## Gotchas

- **Every CDP call hangs** if the `Browser::launch` handler future is not polled;
  `ChromeHarness` spawns the pump. Do the same if you drive `chromiumoxide`
  directly.
- **Prefer a temp `file://` URL over `data:`**; navigation waits may never resolve
  for `data:` URLs.
- **`#[non_exhaustive]` option structs** in the crate under test (for example
  `HtmlOptions`) must be built from `Default::default()` and mutated.
