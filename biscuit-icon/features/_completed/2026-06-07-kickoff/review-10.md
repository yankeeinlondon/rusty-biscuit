---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 10 closes the iteration-9 concurrency, Level 2 image gating, and
canonical Level 1 filtering findings. The feature is not ready for production
because upgrading the previous cache schema silently discards non-zero view-box
origins from already cached icons.

## Findings

### 1. High: The cache migration destroys non-zero view-box origins

The v0 schema stores the complete view box as text. During migration, the code
drops that column and only afterward adds `left` and `top` with zero defaults.
An existing row such as `view_box = "10 20 32 32"` therefore becomes
`left = 0, top = 0, width = 32, height = 32`. Subsequent SVG assembly uses the
wrong `viewBox`, which can shift or clip the cached artwork.

The migration test does not detect this because its sole fixture uses a
zero-origin view box and asserts zero after migration.

Evidence:

- `lib/src/cache/store.rs:89-100` drops `view_box` before adding zero-filled
  origin columns.
- `lib/src/cache/store.rs:157-169` reconstructs every cached body from those
  migrated `left` and `top` values.
- `lib/src/style.rs:112-144` uses the reconstructed origin for SVG assembly and
  transforms.
- `lib/src/cache/store.rs:367-417` covers only `view_box = "0 0 24 24"`.
- The specification requires storing the complete view box and states that
  cached entries persist until explicitly cleared.

Parse and copy the first two `view_box` components into `left` and `top` before
removing the legacy column, preferably in one transaction. Add a migration test
using a non-zero and/or negative origin and assert the final `Icon::svg()` output,
not only the database row.

### 2. Low: Cache timestamps do not use the specified RFC 3339 format

Both icon and set writes use SQLite `datetime('now')`, which produces a value
such as `2026-06-08 12:34:56`. The schema contract specifies RFC 3339, which
requires the date/time separator and timezone designator. This is not currently
user-visible because timestamps are neither exposed nor used for expiry, but the
persisted format does not match the documented contract.

Evidence:

- `lib/src/cache/store.rs:182-184` writes icon timestamps with
  `datetime('now')`.
- `lib/src/cache/store.rs:226-227` does the same for set metadata.
- `features/2026-06-07-kickoff/spec.md:237` specifies RFC 3339.

Write an RFC 3339 UTC value, for example with SQLite's `strftime` and a `Z`
suffix, and add a focused format assertion.

## Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Domain enums, SVG assembly, styling, current-schema cache, and HTTP contracts | Level 1 | Passed |
| Previous-schema cache compatibility | Level 1 | Gap: migration test misses non-zero-origin data loss |
| Default command, direct lookup, cache clear, completions, online merge, prefix filtering, bounded concurrency | Level 1 CLI subprocess | Passed |
| Unicode glyph rendering | Level 2 tmux | Passed |
| Nerd Font glyph rendering | Level 2 tmux | Passed |
| Text fallback and multi-row listing | Level 2 tmux | Passed |
| Styled CLI errors | Level 2 tmux | Passed |
| Image-protocol fallback | Level 2 WezTerm with screenshot comparison | Passed |
| OS keyboard/mouse behavior | Not applicable | No Level 3 requirement |

## Commands Run

- `just test`: library 93/93 passed; CLI 23/23 Level 1 tests passed; Level 2
  tests were correctly excluded.
- `just test-l2`: all six Level 2 tests passed, including the image pixel
  assertion in WezTerm.
- `just lint`: passed for both crates.
- `git diff --check`: passed.

The required `biscuit-icon` skill is absent from the authoritative local skill
catalog and configured skill roots. Package-specific guidance was therefore
derived from the specification, implementation, prior reviews, and repository
instructions. The `rust-testing` skill was used for the verification-level audit.
