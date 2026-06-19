---
title: biscuit-icon — Icons and Cache Reporting
status: ready for planning and implementation
reviewed: true
last_updated: 2026-06-09
---

# biscuit-icon — Icons and Cache Reporting Spec

The `icon` CLI defined in `biscuit-icon` has some pretty big gaps in:

- reporting on icons
- reporting on cache

The `icon sets` functionality is fine where it is.

## `show` (default command)

`icon <filter>` is shorthand for `icon show <filter>`. There is no separate `icons` / `list` command at the top level for Iconify lookups — that functionality is folded into `show`. The spec previously called the top-level form `icon icons <filter>`; the awkward expression is gone, but the shorthand `icon <filter>` still works.

`icon show <id...>` accepts one or more icon ids. The four ways the user may want to see an icon:

1. an image of the icon rendered to the terminal (if the terminal doesn't support Kitty graphics we fall back to SVG text)
2. the SVG markup for the icon (plain text)
3. the SVG markup rendered as a code-block, leveraging Darkmatter's highlighter
4. the CSS `url(...)` value for this icon (plain text)

### Argument validation

`icon show` always requires at least one token. With zero arguments, the command must fail with a clear error explaining that at least one icon id or filter is required, and suggest a concrete invocation (e.g., `icon show mdi:home` or `icon show <filter>`).

Every token passed to `icon show` must be at least three characters long. The Iconify catalog contains 200,000+ icons, so filters shorter than three characters match too many icons to be useful and produce unwieldy output. When a token is shorter than three characters, the command must fail with a clear error explaining the minimum length, naming the offending token, and suggesting a longer query or an exact id.

Both errors follow the project's error-UX standard (see Design Decision 7): a plain-language description of what went wrong, a hint about how to fix it, and concrete examples.

### Show flags

- `--svg` — emit the SVG plain text for the icon
- `--code-block` — emit the SVG as a highlighted code block (Darkmatter)
- `--css` — emit the CSS `url(...)` value (see [CSS formatting](#3-css-url-formatting) design decision)
- `--meta` — force tabular output and add Categories, Tags, Author, and License columns (see `--meta` section below)
- `--list` — skip the picker; always list one match per line (`<render>  <id>`)
- `--pick` — force the picker (error in non-TTY)
- `--nerd` — if the icon has a Nerd Font glyph, render that instead of the SVG
- default — render the SVG via the `TerminalImage` ladder, falling back to SVG plain text when the terminal does not support image protocols

> **Formatting Exclusivity:** The format-selection flags `--svg`, `--code-block`, and `--css` are mutually exclusive. Providing more than one of these flags must result in a CLI error and a non-zero exit status (1).

### Multiple icons

When two or more ids are passed and all resolve, the output is a table: first column is the fully-qualified id (`{set}:{name}`), second column is whatever is being displayed (the rendered icon, or SVG text with `--svg`, etc.). With a single id the output is a single render — no table — unless `--meta` is supplied.

If any of the passed IDs fail to resolve, the command must immediately fail, print an error to stderr (accompanied by suggestions where appropriate), and exit with status 1.

## `--meta`

Forces tabular output regardless of arg count. Always shows the columns: `Set`, `Icon`, `Categories`, `Tags`, plus `Author` and `License` when those fields are populated for the matched icon. The display column takes whatever the other flags specify (default ladder, `--svg`, `--code-block`, or `--css`).

## Inexact Matches

`show` accepts three argument shapes:

- `<id>` containing `:` that exactly matches a known icon → render that single icon (ladder or fallback).
- `<id>` containing `:` that does NOT match → error "icon does not exist", plus a suggestion list of icons whose name (the substring after the `:`) matches the filter as a substring.
- `<filter>` without `:`:
    - TTY + ≥2 matches (offline ∪ online) → launch the `choose_many` picker from `biscuit-tui` to allow selecting one or more icons.
        - Picking exactly one icon renders that single icon.
        - Picking two or more icons renders them as a table (identical to multiple ID behavior).
        - Picking zero is a no-op (exit 0).
    - TTY + 1 match → auto-render that single icon.
    - non-TTY (pipe / CI) → list every match to stdout, one per line (`<render>  <id>`).
    - 0 matches → error "no icons match".

`--list` forces the list path (skip the picker) even on a TTY. `--pick` forces the picker and errors in non-TTY. Esc / Ctrl-C inside the picker is a user-initiated abort (exit 130); the user picking zero items is a no-op (exit 0).

## Cache

The `cache` subcommand gets a real `action` enum. The previous bare `icon cache clear` (no subgroup) is **removed** — `icon cache clear` is now parsed as `cache { action: Clear }`.

- `icon cache list` — table of cached icons. Columns: `Set`, `Icon`, `Display`, `Categories`, `Tags`.
- `icon cache clear [filter]` — drop cached icons. With no filter, clear every row in the `icons` and `sets` tables (the cache file itself is preserved). With a filter, clear any icon whose id contains the filter as a substring.

The clap shape becomes `Cache { action: CacheAction::List | CacheAction::Clear { filter: Option<String> } }`.

## Domain

`icon domain` is **curated-enum-only**. It enumerates the 16 hand-curated domain enums in `biscuit-icon/lib/src/domain/` (`Os`, `Emoji`, `File`, `Brand`, `…` — see the `domain-icons.md` skill file for the full list).

- `icon domain` (no args) — table of the 16 enum names. Columns: `Domain Set`, `Variant Count`.
- `icon domain <enum>` — list variants of that enum. Default columns: `Variant`, `Icon`. Each row's `Icon` cell renders the variant through the same ladder used by `icon show`: Nerd Font glyph → Unicode glyph → inline image (Kitty/iTerm/Sixel) → SVG code block (Darkmatter-highlighted) → plain SVG text. The Iconify id is intentionally hidden by default (it is only useful in edge cases); pass `--verbose` / `-v` to add an `Iconify ID` column. The `Icon` cell honors the same `--svg` / `--code-block` / `--css` overrides as `icon show`, applied per-cell.
- `icon domain <enum>:<variant>` — render that single curated icon. **Infallible** (compile-time enum). The default behavior runs the same ladder used by `icon show`: inline image when the terminal supports an image protocol, otherwise an SVG code block (Darkmatter), otherwise plain SVG text. The single-icon form accepts the same `--svg` / `--code-block` / `--css` overrides as `icon show`; these replace the ladder output for that one icon. The Iconify id is **never** emitted by this invocation; if a user needs the id, they look it up in the table form with `--verbose`.

### Domain flags

The `icon domain` subcommand accepts the same flag set as `icon show` (via the shared `ShowFlags` clap struct) so that single-icon rendering is consistent across subcommands:

- `--svg` — emit raw SVG text (single-icon form) or per-cell raw SVG (table form).
- `--code-block` — emit Darkmatter-highlighted SVG code block.
- `--css` — emit CSS `url('data:image/svg+xml,…')` (per Design Decision 3 percent-encoding).
- `--verbose` / `-v` — add the `Iconify ID` column to the table form. No effect on the single-icon form (the id is hidden there by design).
- `--from <csv>` — same meaning as on `icon show`: limit substring search and online lookups to the listed set prefixes.
- `--meta` / `--list` / `--pick` / `--nerd` — accepted for symmetry with `icon show` but no-op on the curated `icon domain` path (the curated enum path doesn't fetch online, doesn't run a picker, and Nerd Font rendering is the same as `icon show`'s default). The flags are accepted silently so a user can copy a `icon show` invocation and switch to `icon domain` without dropping flags.
- `--svg`, `--code-block`, and `--css` are mutually exclusive; the same error UX as `icon show` applies (Design Decision 7).
- `icon domain` is **offline-first by construction**: it does NOT do Iconify lookups, does NOT hit the network, and does NOT use the SQLite cache. All curated variant data — SVG bodies, glyph codepoints, and Iconify-id metadata — is compiled into the binary (via `LazyLock` or an equivalent compile-time mechanism), so the ladder is guaranteed to have a body to render for every variant; missing data is not a possible failure mode. The standard render size for the inline-image step of the ladder is **32×32 pixels**; all curated SVG bodies are sized to look consistent at that dimension, and the terminal-image ladder and the icon library handle rasterization and any size adjustment transparently.

The earlier `icon domain sport:baseball` example in the spec is not a valid invocation — `sport:baseball` is a valid Iconify id, but `sport` is a curated enum, not an Iconify prefix. The spec replaces that example with `icon domain emoji:happy`.

The `choose_many` picker rules from `show` do NOT apply to `domain`. `icon domain foo` is a simple "list enums matching 'foo'" with no network: 0 matches → error, ≥1 match → list. No interactive picker (the curated set is small and finite; offline-first beats picker UX here).

## Library Invariants

The CLI is a thin layer over the `biscuit-icon` library. Every CLI command in this spec depends on the following library-level guarantee; if the library does not honor it, the CLI is not sound.

### Offline Resize

Once an `Icon` exists in memory, resizing it to any dimension is a purely local operation:

- `Icon::width(w).height(h).svg()` re-assembles the `<svg>` locally in `Style::assemble` (`biscuit-icon/lib/src/style.rs`). The body's intrinsic `width`/`height`/`left`/`top` (`IconBody` geometry) flow into the SVG `viewBox`; the user-supplied `width`/`height` flow into the outer attributes. No I/O, no `reqwest` invocation, no `IconifyClient` call.
- The terminal image ladder rasterizes through `biscuit-terminal`'s `TerminalImage` (resvg-backed), which is local.
- The browser/markdown render path emits the assembled SVG verbatim. Local.
- The only network call in the crate is `Icon::iconify` (and its async variants), which lives on the **lookup** path. Once an `Icon` is constructed, no method on it triggers a network call.

This is what makes the ladder safe to invoke on every CLI render, what makes the `icon domain` 32×32 standard (Design Decision 8 point 3) work without re-fetching, and what makes `icon show --css` and `--svg` work at any user-requested size.

For the related guarantee that all curated variant data is compiled into the binary (so the ladder always has a body to render), see Design Decision 8 point 3.

### Test obligation

There must be at least one L1 library test that exercises this guarantee. The test takes a curated `DomainIcon`, calls `.width("64").height("64")`, and asserts:

- The resulting SVG contains `width="64" height="64"` with the original `viewBox` preserved.
- No `reqwest::Client` is constructed and no `IconifyClient::fetch_body` is called in the process.

The same test must pass for a cached Iconify icon: the icon is first fetched via `Icon::iconify_with` against a `wiremock` server (cache-miss path), then re-constructed from the cache to confirm the second call uses the cached body and the resize remains a local operation. The existing `iconify_with_non_zero_origin_persists_through_cache` test (`biscuit-icon/lib/src/icon.rs:382`) covers the viewBox-preservation half; the offline-resize half is new.

## Design & Implementation Decisions

To address several gaps identified in the original specification relative to Iconify's API behavior and the local SQLite cache, the following design decisions are mandated:

### 1. The Metadata Schema Gap for Categories and Tags
**The Gap:** Individual icon-level categories (e.g., `"Audio & Video"`) are defined as a set-wide dictionary mapping categories to lists of icon names, and are only returned by the massive whole-set JSON (e.g., `mdi.json`). The lightweight on-demand endpoint (`prefix.json?icons=name`) does not consistently return individual categories/tags for single icons, and the current `icons` table schema in SQLite does not store them.

- **Design Decision:** Fallback / Set-Level Metadata Mapping
  - `Set`: Displays the human-readable collection name (e.g., `Material Design Icons`) fetched from the `sets` table by joining on prefix. Falls back to the prefix if missing from cache.
  - `Icon`: Displays the local icon name.
  - `Categories`: Displays `N/A` for dynamically-fetched icons (since individual categories aren't fetched on-demand), or matches the Domain Enum Set name for compiled-in curated icons (e.g., `Set: Os`, `Categories: Os`).
  - `Tags`: Displays the set-level tags (e.g., `General, Web, Utility`) fetched from set metadata.
- **Justification:** This respects **Simplicity First** and **performance**. Fetching complete set-level JSONs for single icons on a cache-miss is extremely slow and bandwidth-heavy, while this solution keeps single fetches fast and leverages existing cached set metadata.

### 2. SQLite Database Migration (v2 → v3)
To store set-level metadata (`author_name`, `author_url`, `tags`, `category`) required for the `--meta` tabular view offline, we will run a transactional SQLite schema migration from `v2` to `v3` inside the `IconCache` opening phase:
- **`sets` Table Alteration:**
  - `ALTER TABLE sets ADD COLUMN author_name TEXT;`
  - `ALTER TABLE sets ADD COLUMN author_url TEXT;`
  - `ALTER TABLE sets ADD COLUMN tags TEXT;` (stored as comma-separated values)
  - `ALTER TABLE sets ADD COLUMN category TEXT;`
- This ensures full offline support for metadata columns under `--meta` once the set is listed/cached.

### 3. CSS `url(...)` Formatting
- **Design Decision:** The `--css` flag output must be formatted as an inlined, percent-encoded SVG data URI wrapped in a CSS `url()` function using single quotes:
  `url('data:image/svg+xml,[percent-encoded-svg-markup]')`
- To ensure cross-browser compatibility, characters like `#` (critical for colors), `<`, `>`, `"`, and `'` must be percent-encoded:
  - `#` $\rightarrow$ `%23`
  - `<` $\rightarrow$ `%3C`
  - `>` $\rightarrow$ `%3E`
  - `"` $\rightarrow$ `%22`
  - `'` $\rightarrow$ `%27`

### 4. `Display` Column in `icon cache list`
- **Design Decision:** To prevent redundancy, the `Display` column is only displayed if the terminal is capable of visual icon rendering:
  - Nerd Fonts is enabled (`--nerd` / `ICON_NERD_FONT=1`), or
  - The icon defines a curated Unicode representation (like Emojis), or
  - The `image` feature is enabled AND the terminal supports an image-rendering protocol (Kitty, iTerm, Sixel).
  - If none of these conditions are met, the column is omitted (as falling back to a text identifier would duplicate the `Icon` name column).

### 5. Clear Filter Behavior
- **Design Decision:**
  - `icon cache clear` (no filter) deletes all rows from both `icons` and `sets` tables.
  - `icon cache clear <filter>` deletes rows from `icons` where `prefix || ':' || name` contains the filter (case-insensitive substring match). The `sets` table is unaffected by filtered clears to preserve general metadata.

### 6. Suggestion Search Space
- **Design Decision:** Suggestions for failed `:` lookups (e.g. `icon show uil:bad-name`) are searched from the unified offline catalog (compiled-in domain icons + cached SQLite icons) using case-insensitive substring matches, keeping the error path extremely fast and offline-only.

### 7. Error UX Standard

- **Design Decision:** Every CLI error reported by `icon` must follow a single error-UX standard:
  - State what went wrong in plain language (no stack traces, no jargon).
  - Name the offending input where applicable (e.g., the specific token that was too short, the specific flag combination that was rejected).
  - Provide a concrete next step whenever one is possible: an example invocation, a corrected input, or a list of suggestions. Validation errors (missing args, length violations, mutually exclusive flags) must always include at least one example of a correct invocation.
  - Write to stderr and exit with a non-zero status.
  - This standard applies to `icon show`, `icon domain`, `icon cache`, and any future subcommand.

### 8. Domain Rendering and Iconify ID Hiding

- **Design Decision:** `icon domain` treats the Iconify id as an implementation detail, not user-facing metadata. Three consequences:
  1. The table column that was previously named `Glyph` is renamed to `Icon`, and its cell **renders** the variant through the same ladder used by `icon show` (Nerd Font glyph → Unicode glyph → inline image → SVG code block → plain SVG text). "Glyph" implied a single character; "Icon" accurately conveys "the icon, however it can be shown in this terminal". The ladder must be honored even in a table cell — a code block is the correct text fallback, not an empty cell and not a raw id.
  2. The Iconify id is **never** part of the default output of `icon domain <enum>` or `icon domain <enum>:<variant>`. It is hidden because (a) the curated enum path is the user's authoritative identifier — `os:finder` is the spec, the Iconify id `hugeicons:apple-finder` is just one of several possible backing icons — and (b) the typical reader of `icon domain` output wants to *see* the icons, not their upstream pointers. Power users who need the id (e.g., to file an issue against the curated mapping) opt in with `--verbose` / `-v`, which adds an `Iconify ID` column to the table form. The id is not exposed on the single-icon form even with `--verbose`; users get it from the table.
  3. **Offline-first by construction:** all curated variant data — SVG bodies, glyph codepoints, and Iconify-id metadata — is compiled into the binary via `LazyLock` or an equivalent compile-time mechanism. The ladder is guaranteed to have a body to render for every variant; there is no "what if the body is missing?" failure mode. The standard render size for the inline-image step is **32×32 pixels**; all curated SVG bodies are sized to look consistent at that dimension, and the terminal-image ladder and the icon library handle the actual rasterization and any size adjustment transparently.

## Acceptance Criteria

Each row is a testable behavior.

- `icon show mdi:home` (TTY) — renders the home icon via the ladder or fallback.
- `icon show mdi:home mdi:user` — table with two rows.
- `icon show mdi:home --meta` — 1-row table with `Set`, `Icon`, `Categories`, `Tags`, `Author`, `License`.
- `icon show home` (TTY, ambiguous, ≥2 matches) — picker launches.
- `icon show home` (pipe / non-TTY) — lists every match, one per line.
- `icon show home --list` (TTY) — lists every match, one per line.
- `icon show homex` (no match) — error, non-zero exit.
- `icon show uil:bad-name` (no match) — error plus a suggestion list.
- `icon show mdi:home --svg` — plain SVG text, no ladder.
- `icon show mdi:home --css` — CSS `url(...)` value.
- `icon show mdi:home --code-block` — Darkmatter-highlighted code block.
- `icon domain` (no args) — table of the 16 enum names with columns `Domain Set` and `Variant Count`.
- `icon domain emoji` — table of `Emoji` variants with columns `Variant` and `Icon`; the `Icon` cell renders each variant through the ladder (glyph → image → SVG code block → SVG text).
- `icon domain emoji --verbose` — same as above, plus an `Iconify ID` column.
- `icon domain emoji:happy` — renders the happy emoji through the ladder (infallible, compile-time); never prints the Iconify id.
- `icon domain sport:baseball` — errors with "not a curated enum"; intentional, by design.
- `icon cache list` — table of cached icons.
- `icon cache clear` — clears every cached row in `icons` and `sets`.
- `icon cache clear home` — clears cached icons whose id contains "home" in `icons` (leaves `sets` intact).
- The old `icon cache clear` (no subgroup) is REMOVED — clap should error on that invocation.
- `icon show` (no arguments) — clear error explaining that at least one icon id or filter is required, with an example invocation; exit ≠ 0.
- `icon show ab` (token shorter than 3 characters) — clear error explaining the 3-character minimum, naming the offending token, and suggesting a longer query or exact id; exit ≠ 0.
- `icon show mdi:home` (3+ char token, normal case) — behaves per the existing rules (renders, picks, or lists).
- `icon mdi:home --svg` (top-level default command with show flag) — clap accepts the flag; output is raw SVG, same as `icon show mdi:home --svg`. The format flags (`--svg`, `--code-block`, `--css`, `--meta`, `--list`, `--pick`) and `--from` all work at the top level via the shared `ShowFlags` clap struct, not just under the `show` subcommand.
- `icon domain emoji:happy --svg` (domain subcommand with show flag) — clap accepts the flag; output is raw SVG. The same `--svg` / `--code-block` / `--css` / `--verbose` / `--from` flags work on `icon domain` as on `icon show`.
- `icon show mdi:home --code-block` — output is the Darkmatter-highlighted code block with no debug preamble on stderr (the previous implementation leaked a `CODE_RENDERER width=…` line to stderr).

## Out of Scope

- Shell completions — unchanged.
- `icon sets` — unchanged.
- `icon domain` doing network / Iconify lookups — out of scope by design (D3).
- Future drill-down `icon cache show <icon>` — reserved, not built.
- Any column not listed in the `--meta` section above (D5) — not added in this spec.

## Backwards Compatibility

- `icon cache clear` (no subgroup) is removed. Users on the old form get a clap error.
- `icon <filter>` still works, but its output shape changes: previously it always listed every match; now it renders / picks / lists depending on TTY and arg count per the `show` rules. Scripts that piped `icon <filter>` will keep working (non-TTY → list); scripts that asserted on a picker prompt on a TTY will not.
