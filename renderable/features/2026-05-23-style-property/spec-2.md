---
status: draft
date: 2026-05-23
owner: ken
parent: docs/rendering/style.md
sub-spec: 2-of-7
depends-on: spec.md (sub-spec #1)
---

# `style:` Frontmatter — Sub-Spec #2: Page-Level Wiring

## Problem

Sub-spec #1 ships a parser that produces a `StyleFrontmatter` AST. The
`md` CLI doesn't yet *use* it — every parsed key still emits a
`KnownButInactive` warning. This sub-spec wires the **page-level** subset of
the AST (`style.page.*`) into the `DarkmatterPage` builders that
`darkmatter/cli/src/output.rs:render_terminal_output` and `html_artifact`
already use, so that `md style-prop.md` finally honors page margins, padding,
max-width, alignment, and page-background settings.

This is the first sub-spec that produces a **user-visible behavior change**
for the originally reported bug, partially (page-level only; component-level
keys land in #3 and #4).

## Goals

- Read `style.page.*` from the parsed `StyleFrontmatter` and apply each
  setting onto a `DarkmatterPage` before rendering.
- Define a clear precedence rule between **frontmatter style** and
  **CLI flags** (`--margin`, `--padding`, etc.). Recommended default: CLI
  flags override frontmatter (explicit beats implicit).
- Introduce `md --strict-style` CLI flag that promotes schema-validation
  warnings (`UnknownKey`, `Deprecated`) to errors. `KnownButInactive` does
  not fail strict mode.
- Emit `tracing` events for parsed-but-unwired keys so `RUST_LOG=darkmatter=info`
  surfaces what was found but not yet applied.
- Suppress `KnownButInactive { sub_spec: 2 }` warnings for the keys this
  sub-spec wires — they have moved from "inactive" to "active" and shouldn't
  generate informational noise.

## Non-Goals

- No component-level wiring (table/images/block-quote/lists — #3, #4).
- No color/bg-color application (#5).
- No HR migration (#6).
- No new bespoke knobs (#7).
- No changes to the schema or parser produced by sub-spec #1.

## Dependencies

- **Sub-spec #1** must be merged. This sub-spec reads its `StyleFrontmatter`
  type and `from_frontmatter` / `into_strict` functions.

## Decisions to Settle (Brainstorm Inputs)

1. **CLI flag vs. frontmatter precedence.** Recommended: CLI > frontmatter
   for every overlap. Justification: explicit invocation beats document
   metadata; matches the existing `apply_cli_layout_flags` precedence model.
   Alternative: frontmatter > CLI (document is authoritative). Pick one;
   document in `darkmatter/docs/rendering/style.md`.
2. **`--strict-style` semantics.** Confirmed: applies to schema warnings
   only (`UnknownKey`, `Deprecated`). Does NOT fail on
   `KnownButInactive` — that's enforced by `into_strict`'s existing
   behavior. The flag is a library passthrough.
3. **Warning suppression strategy.** Where does the
   "this sub-spec wires the key" knowledge live? Options:
   - Track wiring status in code (a `Wired` enum returned from the
     applicator) and filter warnings *after* wiring runs.
   - Edit the `SCHEMA` descriptor to bump `sub_spec` to `1` (meaning "wired
     in v1") as each phase ships. Simpler but mutates the descriptor.
   - Both. Recommended: just bump the descriptor — single source of truth.
4. **Length unit semantics on the `DarkmatterPage` boundary.**
   - `renderable::layout::Length::Ch(u32)` → `DarkmatterPage::with_margin_left(u16)`. Needs a saturating cast.
   - `Length::Percent(f32)` → `DarkmatterPage` does NOT accept percent for
     margins. Either: (a) reject `style.page.left-margin: 50%` with a clear
     error, (b) resolve against the terminal width at apply time, (c)
     extend `DarkmatterPage` to accept percent. Recommended: (b) for v1
     (resolve in the applicator).
5. **`page.background` enum mapping.** `darkmatter::layout::PageBackground` has
   `Transparent | Subtle | Pronounced`. `DarkmatterPage::with_page_background`
   already accepts this. Trivial wiring.
6. **What about `page.alignment`?** `DarkmatterPage` doesn't currently have a
   page-level alignment knob — `use_alignment_for_all` applies to *every*
   component. Decide: does `style.page.alignment: center` mean "center every
   component"? Probably yes. Document.

## Public API (Sketch)

```rust
// darkmatter::style — new

/// Apply parsed page-level style onto a DarkmatterPage builder.
///
/// CLI flags override frontmatter for any overlapping field. Returns the
/// page with `style.page.*` settings applied, plus any `KnownButInactive`
/// warnings re-classified now that v1 wiring is live.
pub fn apply_page_style(
    page: DarkmatterPage,
    style: &StyleFrontmatter,
    cli_overrides: &CliLayoutOverrides,  // existing flag bundle
) -> Result<(DarkmatterPage, Vec<StyleWarning>), StyleApplyError>;
```

`darkmatter/cli/src/output.rs:render_terminal_output` and `html_artifact`
both call this helper before invoking `page.render(md)`.

## CLI Changes

`darkmatter/cli/src/args.rs` adds:

```rust
/// Promote schema-validation warnings (unknown / deprecated keys) to errors.
#[clap(long)]
pub strict_style: bool,
```

`darkmatter/cli/src/output.rs` reads the parsed frontmatter via
`darkmatter::style::from_frontmatter`, applies `into_strict` when
`--strict-style` is set, then calls `apply_page_style`.

## Tests

1. **Page margin from frontmatter** — `style.page.left-margin: 2ch` →
   rendered output starts every non-empty content line with two columns of
   leading whitespace. Snapshot or invariant test.
2. **CLI flag overrides frontmatter** — both `--margin-left 4` and
   `style.page.left-margin: 2ch` → CLI wins.
3. **Percent margin resolves against terminal width** —
   `style.page.left-margin: 10%` at 80-col terminal → 8 columns left margin.
4. **`--strict-style` fails on unknown key** — exit non-zero with the
   `StyleWarning` details on stderr.
5. **`--strict-style` succeeds on schema-clean document** — only
   `KnownButInactive` warnings → exit 0.
6. **Integration with `style-prop.md`** — extend the existing
   `style_frontmatter.rs` integration test (or add a new one alongside) that
   asserts the rendered page output has the expected margins/padding/
   max-width without the integration test asserting on terminal-specific
   ANSI bytes.

## Acceptance Criteria

- `md darkmatter/example-docs/rendering/style-prop.md` produces output where
  the page-level margins (2ch left, 4ch right, 1 row top, 0 rows bottom)
  are visibly applied. Render-snapshot test confirms.
- Existing `cargo nextest run -p darkmatter` passes.
- No regression on `apply_cli_layout_flags` users (existing CLI flag
  behavior unchanged).
- Documentation in `darkmatter/docs/rendering/style.md` updated to reflect
  page-level support is live.
- `KnownButInactive { sub_spec: 2 }` warnings no longer fire for keys
  wired here.

## Risks

- **Precedence regression.** If we get the CLI-vs-frontmatter rule wrong,
  existing CLI users could see surprising changes when a doc happens to
  include `style:` block. Mitigation: end-to-end CLI test asserting CLI
  flags still win unchanged.
- **`Length::Percent` resolution timing.** Resolving at apply time requires
  the terminal width to be known. `DarkmatterPage::new(&term)` already
  captures it, so this is fine — but document the contract.
- **Pre-existing double-alias gap** from sub-spec #1's final review (the
  `block_quote.max_width` → `UnknownKey` case) only matters here if a user
  writes the snake-case container form. Fix in the walker as part of this
  sub-spec or roll into a separate `sub-spec #1.5`.

## Open Questions

1. CLI flag precedence direction — needs final user call.
2. Should `style.page.alignment` apply to every component, or be a
   container-level alignment of the whole rendered page (currently no such
   knob exists)? Lean toward "applies to every component" because that's
   what `use_alignment_for_all` does.
3. Where does the warning suppression for now-wired keys live — descriptor
   edit, or runtime filter? Recommended: edit descriptor sub_spec to 1
   (meaning "wired in this binary now") when sub-spec lands.

## Out-of-Spec

After sub-spec #2 + #3 + #4 land, the user's original bug is fully fixed.
After #5 lands, color/bg-color works. After #6, HR migration is complete.
After #7, the last bespoke knobs work.
