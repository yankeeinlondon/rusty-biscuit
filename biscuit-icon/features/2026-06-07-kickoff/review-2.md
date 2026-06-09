---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 2 resolves most findings from the first review, and the L1 suite, lint,
image-feature compilation, and dedicated L2 recipe all pass. The feature is
still not ready for production because the canonical render tree does not
provide the specified terminal degradation and the image fallback has no
effective Level 2 verification.

## Findings

### 1. High: The canonical render tree emits raw SVG to terminal targets

`Icon::render_tree()` contains only a raw HTML node. Folding that tree through
the shared terminal adapter therefore prints the SVG markup instead of selecting
the Nerd Font, Unicode, image, or identifier fallback. The new test explicitly
asserts this incorrect behavior, while the working degradation ladder remains a
separate `TerminalRenderable` implementation.

Evidence:

- `lib/src/render.rs:12-16` projects only `RenderNode::html`.
- `lib/src/render.rs:71-87` expects `<svg` from `render_terminal_node`.
- `lib/src/icon.rs:209-228` contains the actual terminal degradation separately.

This violates the central rendering decision that `Icon` composes through the
shared multi-target tree. Represent a target-appropriate fallback in the tree,
or extend the shared IR/adapter so terminal folding delegates to the icon's
terminal representation. Replace the current test with parity assertions for
glyph and identifier fallback.

### 2. High: Image-protocol rendering is still unverified at Level 2

The image test can pass without rendering an image: it accepts the text
identifier as a successful result. It is also Kitty-only, and `require_level!`
returns early when Kitty is unavailable. In this review's `just test-l2` run,
the image test completed in 27 ms while no Kitty shared pane was spawned,
indicating that the body was skipped even though nextest reported a pass.

Evidence:

- `cli/tests/level2_terminal.rs:145-150` gates solely on Kitty availability.
- `cli/tests/level2_terminal.rs:164-170` accepts text fallback.
- `justfile:48-49` enables `image`, but does not require an image-capable backend.

An image-enabled L2 test must hard-fail when its selected backend is required and
must assert actual graphics protocol output, not the fallback. Cover an available
image-capable backend such as WezTerm as well as Kitty, or expose a harness
capability selector.

### 3. Medium: `--from` is ignored for direct `prefix:name` lookups

The direct-lookup branch runs before `--from` is parsed. Consequently,
`icon icons ic:baseline-apple --from mdi` succeeds and prints the `ic` icon,
although the requested set filter excludes it.

Evidence:

- `cli/src/commands.rs:33-38` returns before constructing `allowed`.
- `cli/src/commands.rs:40-46` applies the filter only to substring listings.

Parse and validate `--from` before the direct branch, then reject or return no
results when the identifier's prefix is not allowed. Add tests for both explicit
and default-command forms.

### 4. Medium: Dynamic completions do not offer icon-set names

The specification requires completions to always know set names as well as
built-in and cached icon names. The only custom completer returns
`prefix:name` identifiers, and neither the `sets` filter nor `--from` has a set
completer. The integration test verifies built-in and cached icons only.

Evidence:

- `cli/src/args.rs:39-49` attaches no completer for set filters or `--from`.
- `cli/src/args.rs:70-88` builds only icon identifier candidates.
- `cli/tests/cli.rs:75-117` does not exercise set-name completion.

Add a set completer backed by built-in prefixes plus cached `sets`/icon prefixes,
attach it to `sets` and `--from`, and test it through `CompleteEnv`.

### 5. Medium: Set attribution metadata is discarded and cache failures are hidden

The cache schema includes license metadata and the non-goals require preserving
attribution metadata, but the collections response parses only the set name.
The CLI stores every online set with `license: None` and discards any cache-write
error. `put_set` also serializes `None` as an empty string, which reads back as
`Some("")`.

Evidence:

- `lib/src/iconify/client.rs:53-57` omits collection license metadata.
- `cli/src/commands.rs:104-110` writes `license: None` and ignores the result.
- `lib/src/cache/store.rs:153-159` converts `None` to an empty string.

Parse and persist Iconify's license/attribution fields, preserve SQL `NULL`, and
surface or trace cache-write failures according to the CLI error policy.

## Verification Matrix

| User-facing requirement | Strongest verification | Assessment |
|---|---:|---|
| Embedded enum/string lookup | Level 1 | Appropriate |
| Local SVG styling and escaping | Level 1 | Appropriate |
| Cache-first network lookup | Level 1 | Appropriate |
| Browser inline SVG | Level 1 | Appropriate |
| Markdown inline SVG | Level 1 | Appropriate |
| Terminal glyph and text ladder via `TerminalRenderable` | Level 2 | Appropriate for the separate trait path |
| Terminal ladder through the shared render tree | Level 1 | Functional gap: test verifies raw SVG |
| Image-protocol fallback | None effective | **Level mismatch: requires Level 2** |
| Visually rendered icon/name output | Level 2 | Appropriate for Unicode, Nerd Font, and text fallback |
| `--from` filtering | Level 1 | Direct-identifier case missing and broken |
| Offline built-in/cache listings | Level 1 | Appropriate |
| Built-in plus cached icon completions | Level 1 | Appropriate |
| Icon-set completions | None | Functional and test gap |
| Styled CLI errors | Level 2 | Appropriate |
| OS keyboard behavior | Not applicable | No Level 3 requirement |

## Validation

- `just test`: passed 75 library tests and 12 CLI tests.
- `just lint`: passed for both packages.
- `cargo test -p biscuit-icon --features image --no-run --color=never`: passed.
- `cargo test -p biscuit-icon-cli --features image --no-run --color=never`: passed.
- `just test-l2`: reported 6 passes, but the image test skip-cleaned because
  Kitty was unavailable; the remaining five terminal checks executed.
