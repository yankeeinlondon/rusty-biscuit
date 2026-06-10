---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 3 closes the direct `--from`, set-completion, and SQL `NULL` issues
from the previous review. The feature is still not ready for production: the
real Iconify collections response cannot be decoded, the canonical render tree
still omits two terminal tiers, and the required image Level 2 test fails.

## Findings

### 1. High: Production Iconify collection responses cannot be decoded

`CollectionMeta::license` is modeled as `Option<String>`, and its test fixture
also supplies a string. The real `/collections` API returns an `IconifyInfo`
license object containing fields such as `title`, `spdx`, and `url`. Serde will
therefore reject normal production responses with “invalid type: map, expected
a string.” This breaks online `icon sets`, prevents set metadata from being
cached, and still does not preserve the attribution shape required by the spec.

Evidence:

- `lib/src/iconify/client.rs:53-59` declares `license: Option<String>`.
- `lib/src/iconify/client.rs:228-242` uses a non-representative string fixture.
- Iconify's `/collections` contract documents `license` as an object:
  <https://iconify.design/docs/api/collections.html>.

Model the upstream license object, choose an explicit persisted representation
(at minimum SPDX plus title/URL where present), and test with a fixture matching
the documented API response.

### 2. High: The shared render tree still does not implement the terminal ladder

The new `"icon"` extension payload contains only the Unicode glyph or identifier.
It has no Nerd Font glyph, SVG/image data, feature state, or terminal capability
context. Consequently, terminal folding through `render_tree()` can never
perform the specified Nerd Font → Unicode → image → text ladder. The implementation
also teaches the generic `biscuit-terminal` renderer a package-specific token,
coupling a shared adapter to `biscuit-icon`.

Evidence:

- `lib/src/render.rs:18-27` serializes only Unicode/text into the payload.
- `biscuit-terminal/lib/src/render_tree/render.rs:886-891` hard-codes `"icon"`
  and returns that payload directly.
- `lib/src/render.rs:84-115` tests only Unicode and identifier projections.
- `lib/src/icon.rs:209-257` retains the complete ladder in the separate
  `TerminalRenderable` path.

Represent the icon through target-neutral IR that the terminal adapter already
understands, or add a generic extension-rendering mechanism. Add tree-path
tests for Nerd Font selection and image-capable rendering, not only the
separate trait implementation.

### 3. High: Image fallback has no passing Level 2 verification

`just test-l2` fails on the available WezTerm backend. The test requires image
protocol bytes in `CapturedFrame.raw`, but WezTerm may consume those bytes and
expose only occupied pane rows. The existing `biscuit-terminal` image suite
handles this documented capture behavior by accepting either protocol bytes or
verified pane geometry. This test has no such fallback, retries four times, and
fails every time.

The test also unconditionally asserts that Kitty or WezTerm is available,
contrary to the repository's skip-clean L2 contract. Required-host enforcement
belongs to `BISCUIT_TEST_LEVEL_REQUIRED=2`, not an unconditional assertion in
one test.

Evidence:

- `cli/tests/level2_terminal.rs:147-154` hard-fails when neither backend is
  available.
- `cli/tests/level2_terminal.rs:198-214` accepts only captured protocol bytes.
- The review run failed four attempts with visible icon rows but no retained
  protocol escape sequence.

Use `require_level!` per backend and the established byte-or-pane-geometry
assertion strategy. Until that passes, the image behavior has no effective
Level 2 coverage and is a required level mismatch.

### 4. Medium: `--from` completion does not handle its CSV value shape

The set completer compares the entire current argument with each prefix. After
the first value, a token such as `mdi,i` cannot match or complete `ic`; candidates
also do not preserve the already entered `mdi,` prefix. The new integration test
only exercises the positional `sets` filter, not either `--from` argument.

Evidence:

- `cli/src/args.rs:93-125` treats the current value as one substring.
- `cli/tests/cli.rs:145-183` covers `icon sets ic` only.

Split at the final comma, complete only the active segment, and reconstruct the
CSV candidate. Test both explicit and default-command `--from` completion.

## Verification Matrix

| User-facing requirement | Strongest verification | Assessment |
|---|---:|---|
| Embedded enum/string lookup | Level 1 | Appropriate |
| Local SVG styling and escaping | Level 1 | Appropriate |
| Cache-first network lookup | Level 1 | Appropriate |
| Online set listing/metadata | Level 1 mock | Broken contract fixture; production response fails |
| Browser inline SVG | Level 1 | Appropriate |
| Markdown inline SVG | Level 1 | Appropriate |
| Terminal ladder via `TerminalRenderable` | Level 2 | Unicode, Nerd Font, and text pass |
| Terminal ladder through shared render tree | Level 1 | Incomplete: no Nerd Font or image tier |
| Image-protocol fallback | Level 2 attempted | **Level mismatch: required test fails** |
| Visually rendered icon/name output | Level 2 | Appropriate for non-image tiers |
| `--from` filtering | Level 1 | Appropriate |
| Offline built-in/cache listings | Level 1 | Appropriate |
| Built-in/cached icon completions | Level 1 | Appropriate |
| Set-name completions | Level 1 | Positional works; CSV `--from` remains broken |
| Styled CLI errors | Level 2 | Appropriate |
| OS keyboard behavior | Not applicable | No Level 3 requirement |

## Validation

- `just test`: passed 77 library tests and 15 CLI tests.
- `just lint`: passed for both packages.
- Image-feature no-run builds passed; the CLI test target emitted one unused
  `WezTermHarness` import warning.
- `just test-l2`: failed in
  `level2_image_protocol_fallback_renders_graphics` after four attempts; five
  remaining L2 tests were canceled.
