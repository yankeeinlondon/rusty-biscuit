---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

The implementation compiles and its current Level 1 suite passes, but it is not
ready for production. Core Phase 3 behavior is incomplete, the terminal rendering
contract has no Level 2 verification, and several library contracts differ from
the design.

## Findings

### 1. High: The primary CLI listing flows are incomplete

`icons` only performs a direct network lookup when the filter contains `:`; all
other requests search cached icon names only. It never includes the built-in
domain catalog, never searches the online Iconify catalog, and returns an error
for an empty isolated cache. The parsed `--from` value is explicitly ignored.
`sets` always calls the network and therefore cannot provide the specified
built-in-plus-cache offline listing.

Evidence:

- `cli/src/commands.rs:24-43` ignores `_from` and lists only cache rows.
- `cli/src/commands.rs:46-57` always calls `fetch_collections()`.
- `lib/src/cache/store.rs:35-48` does not create the specified `sets` table.

Implement a unified catalog query over embedded IDs and cached IDs, honor
`--from`, persist set metadata, and define the online catalog search used to
extend offline results.

### 2. High: Terminal-visible behavior has no Level 2 verification

The CLI promises visually rendered icons, glyph selection, image-protocol
fallback, styled errors, widths, and names. All current renderer checks are
Level 1 string assertions. The package `justfile` declares Level 2 testing "not
applicable", so no real terminal verifies glyph width, SGR styling, image
rendering, or the final listing layout.

Evidence:

- `lib/src/render.rs:76-103` contains only in-process assertions.
- `cli/tests/cli.rs:4-36` tests help, cache clearing, and completion script output.
- `justfile:41-45` disables both terminal tiers.

Add Level 2 capture tests through `biscuit-test-harness` for Unicode and Nerd
Font output, text fallback, image-capable fallback, listing alignment, and styled
errors. Level 3 is not required because the specification has no OS keyboard
event requirement.

### 3. High: Dynamic completions omit all built-in names

The completer queries only SQLite. On a new installation it returns no icon
candidates, contrary to the requirement that completions always know built-in
set and icon names. The generated shell-script test does not invoke or inspect
the dynamic candidate path.

Evidence:

- `cli/src/args.rs:56-66` only calls `IconCache::search_names`.
- `cli/tests/cli.rs:28-36` checks only that an AOT script contains `icon`.

Merge `domain::all_iconify_ids()` and cached IDs, deduplicate them, and add an
isolated-home completion integration test for both built-in and cached values.

### 4. High: The rendering implementation does not honor the specified feature boundary

The design requires image rendering behind a default-off `image` feature.
`biscuit-icon` declares no features and always attempts `TerminalImage` whenever
the terminal reports image support. This changes default behavior and dependency
cost. In addition, the canonical `TreeRenderable` projection contains only an
HTML node; terminal degradation is a separate inherent method rather than the
specified shared rendering integration.

Evidence:

- `lib/Cargo.toml:10-23` has no `image` feature or optional `resvg`.
- `lib/src/render.rs:17-21` projects only raw HTML.
- `lib/src/render.rs:36-67` implements an unconditional runtime image path.

Either implement the design as written or update the design before claiming
conformance. In either case, add Markdown projection tests and verify terminal
projection through the normal shared adapter, not only direct method calls.

### 5. High: SVG style values are inserted without escaping or validation

`color`, `width`, and `height` are public string inputs interpolated directly
into XML attributes. Quotes or markup can produce malformed SVG or inject
attributes/elements. Invalid `flip` and `rotate` values are silently ignored.
Also, rotating a non-square icon around its center without swapping or adjusting
the viewport can clip 90/270-degree output.

Evidence:

- `lib/src/style.rs:47-73` directly interpolates all style strings.
- `lib/src/style.rs:30-40` silently ignores unsupported transform values.

Use typed flip/rotation enums, XML-safe attribute serialization, and geometry
that preserves non-square icons. Add malformed-input and non-square rotation
tests.

### 6. Medium: Text fallback loses the icon identifier

The design requires the icon identifier as the final terminal fallback, but
every glyph-less icon renders the same `[icon]` text. `Icon` does not retain an
identifier, so the required fallback cannot be produced.

Evidence:

- `lib/src/icon.rs:21-26` stores no identifier.
- `lib/src/render.rs:55` emits a constant placeholder.

Store the domain/network identifier in `Icon` and render it in the fallback.

### 7. Medium: CLI diagnostics and verbosity do not match the contract

The CLI writes raw escape sequences directly, contrary to the required
`Prose`-styled error boundary. It has no `--debug` flag, and `--verbose` controls
raw tracing rather than styled user output. Error chains are not deliberately
deduplicated.

Evidence:

- `cli/src/main.rs:28-53` maps `--verbose` to tracing.
- `cli/src/main.rs:34` emits raw ANSI escape codes.
- `cli/src/args.rs:7-17` defines no `--debug`.

Use `Prose`/`TerminalRenderable` for diagnostics and separate user verbosity
from tracing configuration as specified.

### 8. Medium: Identifier parsing accepts malformed IDs

The documentation says exactly one colon is allowed, but `split_once` accepts
inputs such as `mdi:home:extra`. Empty/invalid upstream names are otherwise
passed into a manually formatted URL.

Evidence:

- `lib/src/iconify/client.rs:20-24`
- `lib/src/iconify/client.rs:70-89`

Reject additional separators, validate Iconify prefix/name syntax, and construct
requests with `reqwest` query APIs.

### 9. Low: Synchronous SQLite work runs directly in async lookup paths

`Icon::iconify_with` performs cache reads and writes synchronously on the async
runtime thread. This can stall a current-thread runtime or a busy worker when
the database is locked or storage is slow.

Evidence:

- `lib/src/icon.rs:58-69`

Use `spawn_blocking`, an async SQLite layer, or clearly expose cache operations
as a caller-managed blocking boundary.

## Verification Matrix

| User-facing requirement | Strongest verification | Assessment |
|---|---:|---|
| Embedded enum/string lookup | Level 1 | Appropriate, though constructor coverage is sampled rather than exhaustive |
| Local SVG styling | Level 1 | Appropriate level, but important edge cases are missing |
| Cache-first network lookup | Level 1 | Appropriate |
| Browser inline SVG | Level 1 | Appropriate |
| Markdown inline SVG | None | Gap |
| Terminal glyph/image/text ladder | Level 1 | **Level mismatch: requires Level 2** |
| Visually rendered `icons` output | None | **Level mismatch: requires Level 2** |
| `--from` filtering | None | Functional and test gap |
| Offline built-in/cache listings | None | Functional and test gap |
| Built-in plus cached dynamic completions | None | Functional and test gap |
| Styled CLI errors | None | **Level mismatch: requires Level 2** |
| OS keyboard behavior | Not applicable | No Level 3 requirement in this feature |

## Validation

`cargo test -p biscuit-icon -p biscuit-icon-cli --color=never` passed: 58
library unit/integration tests and 3 CLI tests. No Level 2 or Level 3 tests exist
for this package area.
