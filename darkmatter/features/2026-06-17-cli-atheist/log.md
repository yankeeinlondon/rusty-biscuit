# CLI Atheist — Decision Log

## ADR-1: `CliStyleClaims` lives on `darkmatter::style`

**Status:** Ratified in Phase 4.

**Decision:** `CliStyleClaims` lives on `darkmatter::style` because that is where the `apply_*_style` functions already live. It is a neutral data model expressed in library/layout types (`PageComponent`, `Layout`, `PaintColor`, `PageBackground`, `Alignment`, `Length`, `ThemePair`) — no clap, no CLI-only wrapper types. The CLI builder (`darkmatter/cli/src/style_claims.rs`) is the only code that knows about `Cli`, `CliFill`, and clap aliases.

**Consequences:**
- Precedence and override logic is defined once in the library and reused by the CLI.
- The library can be unit-tested in isolation without pulling in clap or the CLI arg surface.
- Future frontmatter/CLI consumers can build `CliStyleClaims` directly from other input sources.
