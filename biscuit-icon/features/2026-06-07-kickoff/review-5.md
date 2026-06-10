---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 5 closes the catalog deduplication, transform-branch coverage, and
license-field persistence findings from iteration 4. The feature is not ready
for production: image rendering still has no effective Level 2 verification,
non-zero Iconify origins are discarded during final SVG assembly, existing
cache databases are not migrated to the new schema, and the `image` feature
still does not provide the specified dependency boundary.

## Findings

### 1. High: The image Level 2 test reports success without exercising a terminal image

The test detects image support in the nextest process rather than inside the
Kitty or WezTerm pane where `icon` runs. When the runner itself does not
advertise an image protocol, the test returns normally and nextest records a
pass. This happened in validation: the focused test printed `skipping: terminal
does not advertise image support` and finished `ok` even with
`BISCUIT_TEST_LEVEL=2`.

If that early return is removed, the remaining assertion is also not valid for
this harness. `KittyHarness::capture()` and `WezTermHarness::capture()` call the
terminal's `get-text` API, which returns the rendered cell grid after graphics
protocol commands have been consumed. Searching that text capture for the
original Kitty/iTerm graphics escape bytes cannot prove that an image was
painted. The WezTerm harness already exposes `capture_window_png()` specifically
for pixel-based Level 2 image assertions.

Evidence:

- `cli/tests/level2_terminal.rs:156-161` checks `Terminal::new()` in the test runner and silently returns.
- `cli/tests/level2_terminal.rs:179-187` and `:212-223` search text captures for graphics protocol bytes.
- `biscuit-test-harness/src/wezterm.rs:428-478` documents and implements the image-specific pixel capture.
- `biscuit-test-harness/src/wezterm.rs:630-644` and `src/kitty.rs:342-360` capture terminal text, not the original output byte stream.

Run capability detection inside the spawned pane, render a sharp visual witness,
and assert its pixels through `capture_window_png()` or an equivalent Kitty
screen capture. A missing image-capable backend may skip through
`require_level!`, but once a required backend is available the test must not
convert a missing capability or capture into a pass.

### 2. High: Non-zero Iconify view-box origins are still discarded when rendering

The client and cache now preserve `left` and `top`, but `Style::assemble` always
emits a zero-origin view box. An icon fetched with `left = 10`, `top = 20`,
`width = 32`, and `height = 32` is rendered as `viewBox="0 0 32 32"` instead of
`viewBox="10 20 32 32"`. The flip, rotation, and transparent bounding-box
calculations also assume a zero origin. Such icons can remain shifted or clipped
in browser, Markdown, and terminal-image output.

Evidence:

- `lib/src/body.rs:30-39` stores and exposes the complete view box.
- `lib/src/style.rs:76-95` computes transforms from width and height only.
- `lib/src/style.rs:120-134` emits the bounding rect and SVG view box from zero.
- `lib/src/iconify/client.rs:233-252` tests only the fetched `IconBody`; no test assembles that body into final SVG.

Use `body.left` and `body.top` throughout assembly and transform math. Add an L1
fetch-to-cache-to-`Icon::svg()` test for a non-zero-origin fixture, including at
least the untransformed output and one transform.

### 3. High: Existing cache databases are not migrated to the new columns

`open_at` uses `CREATE TABLE IF NOT EXISTS`, which does not add `left`, `top`,
`license_title`, or `license_url` to a database created by the previous schema.
All subsequent icon reads and set reads address those columns unconditionally.
A reproduced old-schema cache fails a direct lookup with `no such column: left`.
Because the cache has no expiry, preserving existing databases is part of the
user-facing persistence contract.

Evidence:

- `lib/src/cache/store.rs:43-74` creates tables but performs no schema-version migration.
- `lib/src/cache/store.rs:100-115` unconditionally selects `left` and `top`.
- `lib/src/cache/store.rs:187-230` unconditionally selects the new license columns.
- All cache tests create fresh databases; none opens a previous schema fixture.

Add a `PRAGMA user_version` migration path (or equivalent transactional column
inspection), preserve existing rows with zero origins/null license details, and
test upgrading the exact previous schema before exercising reads and writes.

### 4. High: The `image` feature still does not gate the `resvg` dependency

The payload and runtime image attempt are now feature-gated, but the feature is
empty and `biscuit-terminal` still pulls `resvg` unconditionally. Consequently a
default `biscuit-icon` build compiles and links the rasterization stack despite
the spec saying the default-off feature pulls in `resvg` and keeps it out of the
default path. `cargo tree -p biscuit-icon --no-default-features -i resvg@0.45.1`
still reaches `resvg` through `biscuit-terminal`.

Evidence:

- `lib/Cargo.toml:10-12` declares `image = []`.
- `biscuit-terminal/lib/Cargo.toml:48` declares `resvg` unconditionally.
- `spec.md:105-109` defines `image` as the feature that pulls in `resvg`.
- `docs/dependencies.md:10` documents the current transitive dependency, which conflicts with the specification.

Make the terminal rasterization dependencies optional and forward the feature
from `biscuit-icon`, or revise the specification before release. Runtime gating
alone does not satisfy the dependency-cost contract.

### 5. Medium: Online CLI behavior is untested and the L1 suite calls the live API

The iteration-4 online merge was implemented directly around
`IconifyClient::new()`, so the command cannot be pointed at wiremock. No CLI test
verifies that an offline hit is merged with additional online hits, that online
results honor `--from`, or that fetched search results enter the cache. Existing
assert-cmd tests such as the offline `apple` and `sets ic` cases now contact the
public Iconify service during `just test`, contrary to the specified no-live-API
test strategy. This makes the suite network-dependent and leaves the new branch
without deterministic coverage.

Evidence:

- `cli/src/commands.rs:60-88` always constructs the public client after rendering offline hits.
- `cli/src/commands.rs:100-147` does the same for set listings.
- `cli/tests/cli.rs:39-96` exercises these paths without a mock endpoint.
- `lib/src/iconify/client.rs:7` and `:85-96` expose custom bases only through explicit client construction.

Inject the client/base URL at the command boundary, add wiremock CLI tests for
offline-plus-online merge, prefix filtering, caching, and offline fallback, and
ensure ordinary L1 tests cannot reach the public network. Configure a finite HTTP
timeout so offline operation cannot wait indefinitely on a stalled connection.

### 6. Low: The README still describes the pre-iteration-5 search behavior

The CLI now attempts online search even when offline matches exist, but the README
says the online catalog is reached only when there are no offline matches. This is
behavioral documentation drift introduced by the implementation change.

Evidence:

- `cli/src/commands.rs:60-62` always attempts online search.
- `README.md:205` states that online search occurs only when offline matches are absent.

Update the README alongside the finalized online/offline policy.

## Verification Matrix

| User-facing requirement | Strongest verification | Assessment |
|---|---:|---|
| Embedded enum/string lookup | Level 1 | Appropriate |
| Local SVG styling and escaping | Level 1 | Appropriate for zero-origin icons; non-zero origins are broken |
| Non-zero view-box preservation | Level 1 partial | Fetch/cache fields are tested, but final SVG output is wrong |
| Cache-first network lookup | Level 1 wiremock | Appropriate for fresh-schema databases |
| Persistent cache compatibility | None | Missing migration coverage; previous databases fail |
| Online icon catalog merge | None deterministic | Live API is contacted; required merge/filter/cache behavior is unverified |
| Online set listing and attribution | Level 1 client wiremock | Parse/persistence units exist; CLI path still uses the live API |
| Browser inline SVG | Level 1 | Appropriate level; inherits non-zero-origin rendering bug |
| Markdown inline SVG / strict rejection | Level 1 | Appropriate |
| Terminal Unicode/Nerd Font/text via CLI | Level 2 | Appropriate |
| Terminal ladder through shared render tree | Level 1 | Appropriate for glyph/text selection |
| Image-protocol fallback | Level 2 attempted | **Level mismatch: test exits as a false pass or asserts on non-observable protocol bytes** |
| Visually rendered icon/name output | Level 2 | Appropriate for glyph/text tiers |
| `--from` filtering and CSV completion | Level 1 | Appropriate for offline/direct cases; online filtering is unverified |
| Dynamic built-in/cache completions | Level 1 | Appropriate |
| Styled CLI errors | Level 2 | Appropriate |
| OS keyboard behavior | Not applicable | No Level 3 requirement |

## Validation

- `just test`: passed 86 library tests and 17 CLI tests.
- `just lint`: passed for both packages.
- Image-feature nextest run: passed 104 tests, but the image test's pass was the silent early-return path.
- `just test-l2`: reported 6 passes and 12 skips; the image test completed in 0.17 seconds without rendering.
- Focused image test with `--nocapture`: printed `skipping: terminal does not advertise image support` and reported success.
- `cargo tree -p biscuit-icon --no-default-features -i resvg@0.45.1`: confirmed `resvg` remains in the default dependency graph.
- Previous-schema database probe: direct cached lookup failed with `no such column: left`.
