# CLI Atheist — Decision Log

## ADR-1: `CliStyleClaims` lives on `darkmatter::style`

**Status:** Ratified in Phase 4.

**Decision:** `CliStyleClaims` lives on `darkmatter::style` because that is where the `apply_*_style` functions already live. It is a neutral data model expressed in library/layout types (`PageComponent`, `Layout`, `PaintColor`, `PageBackground`, `Alignment`, `Length`, `ThemePair`) — no clap, no CLI-only wrapper types. The CLI builder (`darkmatter/cli/src/style_claims.rs`) is the only code that knows about `Cli`, `CliFill`, and clap aliases.

**Consequences:**
- Precedence and override logic is defined once in the library and reused by the CLI.
- The library can be unit-tested in isolation without pulling in clap or the CLI arg surface.
- Future frontmatter/CLI consumers can build `CliStyleClaims` directly from other input sources.

## ADR-7: Shared Level 2 harness lives in `tests/common/level2.rs`

**Status:** Ratified in Phase 7.

**Decision:** The shared Level 2 WezTerm harness, just-built `md` shim,
skip/enforce policy, sentinel polling, and fixture-running helpers live in
`darkmatter/cli/tests/common/level2.rs`. Each top-level `level2_*` test file
imports that harness through `mod common;` and keeps only concern-specific
assertions.

**Consequences:**
- No duplicated WezTerm setup across the split Level 2 layout suites.
- Skip/enforce behavior stays consistent for all layout-related real-terminal tests.
- Tests are less likely to accidentally exercise a host-installed `md` instead of the just-built binary.
- `tests/common` is more substantial, so helpers must stay clearly namespaced under `common::level2`.

## Phase 8: Deferred bool parser cleanup

`parse_bool_str` / `parse_bool_env` duplication remains intentionally
out-of-scope for CLI Atheist. Their eventual move to `biscuit-terminal::env`
is a low-priority cleanup and not required for this structural split.
