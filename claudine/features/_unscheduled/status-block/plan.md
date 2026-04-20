---
phases: 4
created: 2026-04-19
start_phase: 1
---

# StatusBlock Execution Plan

Scope: implement the `StatusState` aliased deprecation, add `StatusState::default_color()`, build the new `StatusBlock` component in `biscuit-terminal`, migrate the four Claudine call sites to `StatusBlock`, and update the two docs. Out-of-scope per spec: migrating the ~23 `StatusState::Failure` call sites and refactoring `ICON_LOOKUP` onto `default_color()`.

## Dependency Map

```
Phase 1 (StatusState foundation in biscuit-terminal)
    │
    ▼
Phase 2 (StatusBlock component + unit tests in biscuit-terminal)
    │
    ▼
Phase 3 (Claudine call-site migrations, parallelizable across files)
    │
    ▼
Phase 4 (Docs + final verification)
```

Phase 1 is a hard blocker for Phase 2 (`StatusBlock` uses `StatusState::Error` and `default_color()` on day one). Phase 2 is a hard blocker for Phase 3 (Claudine cannot migrate to a type that does not exist). Phase 4's SKILL/README updates should reflect landed code, so it runs last.

## Phase 1 — StatusState foundation (biscuit-terminal)

**Goal:** land the aliased `Error` deprecation and `default_color()` on `StatusState` without changing any existing `Status` render output.

**Files touched:**
- `biscuit-terminal/lib/src/components/status.rs`

**Steps (sequential within phase):**

1. Edit `StatusState` in `biscuit-terminal/lib/src/components/status.rs`:
   - Add new variant `Error` with `#[serde(alias = "Failure")]` so legacy JSON still deserializes.
   - Mark the existing `Failure` variant with `#[deprecated(note = "use StatusState::Error instead")]`.
   - Handle `Error` everywhere `Failure` is currently matched inside `status.rs` itself (`ICON_LOOKUP`, any `match` on `StatusState`, `Default` impl if relevant). `Error` maps to the same icon/color currently used for `Failure` (`Red500`). Silence the `deprecated` lint locally (`#[allow(deprecated)]`) on any internal match arm that still references `Failure` so the crate keeps compiling cleanly.
   - **Do not** touch other call sites referencing `StatusState::Failure` — that is explicitly out of scope per spec.
2. Add the new public method on `StatusState`:
   ```rust
   impl StatusState {
       /// Canonical Tailwind color for this variant.
       pub fn default_color(&self) -> Color { /* … */ }
   }
   ```
   The mapping must match the per-variant colors in the spec table: `Error→Red500`, `Warning→Orange500`, `Info→Blue500`, `Success→Green500`, `NotStarted→Gray500`, `Active→Gray600`, `ToolUse→Purple500`, `Subagent→Violet500`. Deprecated `Failure` returns the same color as `Error` (`Red500`).
3. Add two small unit tests inside `status.rs` (next to the existing tests):
   - `default_color_error_is_red500` — asserts `StatusState::Error.default_color() == Color::Tailwind(Tailwind::Red500)`.
   - `failure_alias_deserializes_as_error` — feeds the JSON string `"\"Failure\""` through `serde_json::from_str::<StatusState>` and asserts it equals `StatusState::Error`.

**Parallelizable:** steps 1 and 2 mutate the same file and the same impl, so they run sequentially. Step 3 is additive and could be written in parallel with 1–2 but lives in the same file, so keep it sequential to avoid merge churn.

**Validation checkpoint (gate to Phase 2):**
- `cargo build -p biscuit-terminal`
- `cargo test -p biscuit-terminal --lib` — all pre-existing tests still pass; the two new tests pass.
- `cargo clippy -p biscuit-terminal --lib -- -D warnings` — no deprecation warnings leak out of the crate (the internal `allow(deprecated)` covers the intentional self-references).

## Phase 2 — StatusBlock component + unit tests (biscuit-terminal)

**Goal:** land `StatusBlock` with its full `Renderable` impl, builder surface, severity-derived defaults, and the 19 unit tests listed in the spec.

**Files touched (new + modified):**
- `biscuit-terminal/lib/src/components/status_block.rs` *(new)*
- `biscuit-terminal/lib/src/components/mod.rs`
- `biscuit-terminal/lib/src/prelude.rs`

**Steps:**

1. Create `biscuit-terminal/lib/src/components/status_block.rs` with the struct exactly as specified:
   ```rust
   #[derive(Debug, Clone)]
   pub struct StatusBlock {
       severity: StatusState,
       header: Option<String>,
       body: Option<RenderableContent>,
       hint: Option<String>,
       border_color: Option<Color>,
       border: String,
       layout: Layout,
   }
   ```
   Use the default values from the spec table:
   - `border = "▌ ".to_string()`
   - `layout.left_margin = Margin::Chars(0)`
   - `layout.right_margin = Margin::Chars(5)`
   - `layout.word_wrap = WordWrap::WrapProse(Some(8), None)`
2. Implement the constructor and builder methods:
   - `pub fn new(severity: StatusState) -> Self` — severity is required; everything else defaults per the table above.
   - `header(impl Into<String>)`, `body(impl Into<RenderableContent>)`, `hint(impl Into<String>)`, `border_color(Color)`, `border(impl Into<String>)` — each returns `Self` by value.
   - A private helper `fn resolved_border_color(&self) -> Color` returning `self.border_color.unwrap_or_else(|| self.severity.default_color())`.
3. Implement `Renderable`:
   - `fn is_block_level(&self) -> bool { true }`
   - `fn layout(&self) -> &Layout` / `fn layout_mut(&mut self) -> &mut Layout` so the trait-default builders (`left_margin`, `right_margin`, `word_wrap`, …) from spec section "Builder Methods" apply automatically.
   - `fn render(&self, term: &Terminal) -> String` following the pseudocode in the spec: emit the `Status` header (when present), the `BlockQuote(body)` (when present) wired with `resolved_border_color()` and `self.border`, and the `Prose(hint)` (when present). Apply this instance's `layout` fields to the `BlockQuote`'s `layout_mut()` before rendering. Join the non-empty parts with `"\n"` and **do not** append a trailing newline — callers own surrounding whitespace.
   - Provide `render_optimistic` either by deferring to `render` or by mirroring the render path exactly so the `render_optimistic_matches_render` test passes.
4. Register the module in `biscuit-terminal/lib/src/components/mod.rs` (`pub mod status_block;` plus `pub use status_block::StatusBlock;`).
5. Export `StatusBlock` from `biscuit-terminal/lib/src/prelude.rs` alongside the other component exports.
6. Add the 19 unit tests from the spec's test table into a `#[cfg(test)] mod tests` block in `status_block.rs`, following the style in `biscuit-terminal/lib/src/components/block_quote.rs`:
   - `body_only`, `with_header`, `with_hint`, `all_parts`
   - `error_severity_uses_red500`, `warning_severity_colors`, `info_severity_colors`
   - `default_color_matches_status_icon` (cover at least `Error`, `Warning`, `Info`, `Success`, `Active`, `ToolUse`, `Subagent`)
   - `custom_border_color_overrides_severity`, `custom_border_glyph`
   - `body_from_plain_string`, `body_from_prose`, `body_from_compose`
   - `margins_respected`, `render_optimistic_matches_render`, `is_block_level`
   - `clone_preserves_all_fields`, `debug_output`, `empty_body_no_block_quote`
   Where a test inspects rendered text, assert on stable substrings (ANSI sequences, border glyph, content) rather than full strings to avoid brittleness — follow the existing pattern in `block_quote.rs`.

**Parallelizable within phase:** steps 1–3 (module scaffolding) can be authored alongside step 6 (tests) as TDD iterations. Steps 4 and 5 are single-line edits that happen once the module compiles. All work is confined to `biscuit-terminal`, so fan-out across files isn't meaningful.

**Validation checkpoint (gate to Phase 3):**
- `cargo build -p biscuit-terminal`
- `cargo test -p biscuit-terminal --lib status_block` — all 19 new tests pass.
- `cargo test -p biscuit-terminal --lib` — pre-existing `Status` / `BlockQuote` tests still pass.
- `cargo clippy -p biscuit-terminal --lib -- -D warnings`
- Quick manual ergonomics smoke: `StatusBlock::new(StatusState::Error).header("…").body("…").hint("…").render(&Terminal::default())` compiles and renders without panics in a scratch binary or `cargo test` scratch assertion.

## Phase 3 — Claudine call-site migrations

**Goal:** replace the manual `Status` + `BlockQuote` + `Prose` wiring at four call sites with `StatusBlock`. Preserve visible colors and margins exactly — none of these migrations are intended to change rendered output.

**Files touched:**
- `claudine/cli/src/commands/wrap/live_semantic_sink.rs`
- `claudine/cli/src/output/shell_expansion_error.rs`
- `claudine/cli/src/output/error_report.rs`

**Parallelization:** the three files are independent. They can be migrated in parallel by three sub-agents or three commits on the same branch. Merge order doesn't matter because they share no symbols.

**Step 3.1 — `live_semantic_sink::render_error_block`**
- Replace the manual `BlockQuote::new(...)` wiring with:
  ```rust
  let block = StatusBlock::new(StatusState::Error)
      .body(Prose::new(body))
      .border_color(border_color)
      .left_margin(Margin::Chars(0))
      .right_margin(Margin::Chars(0));
  ```
- Keep the `emit_section_line` loop over `rendered.lines()` intact.
- `.left_margin(0)` is now redundant but kept for traceability. `.right_margin(0)` is an intentional override of the new `Margin::Chars(5)` default and must stay.

**Step 3.2 — `live_semantic_sink::render_warning_header_and_body`**
- Replace the manual header-then-body wiring with a single `StatusBlock`:
  ```rust
  let block = StatusBlock::new(StatusState::Warning)
      .header(header_prose)
      .body(Prose::new(body_prose.to_string()))
      .border_color(Color::Tailwind(Tailwind::Orange700))
      .border("┃ ")
      .left_margin(Margin::Chars(0))
      .right_margin(Margin::Chars(0));
  ```
- Preserve the explicit `Orange700` border override so this migration is a pure refactor. The harmonized `Orange500` default is **not** adopted in this feature.
- Preserve the explicit `"┃ "` border glyph override.
- `render_file_tool_error` needs **no direct change** — it is a thin wrapper around `render_warning_header_and_body` per spec §3.5.

**Step 3.3 — `shell_expansion_error` (`ShellExpansionReport`)**
- Refactor `ShellExpansionReport`: replace the struct with header/body/hint fields by a `build_error_block(source_path, error) -> StatusBlock` function as shown in the spec.
- `render_with_terminal` becomes:
  ```rust
  fn render_with_terminal(source_path: &Path, error: &ShellExpansionError, term: &Terminal) {
      let report = build_error_block(source_path, error);
      log::message("");
      log::message(&report.render(term));
      log::message("");
  }
  ```
- The existing `build_header_prose`, `build_body_content` (or equivalent), and `build_hint` helpers remain — they just feed into the new `StatusBlock` assembly.
- Remove the now-unused `ShellExpansionReport` struct and its `build_header` / `build_body` wrappers if they have no remaining callers. Keep any helpers that are still referenced elsewhere.
- Rely on the blanket `impl<T: Renderable + 'static> From<T> for RenderableContent` — don't wrap body content in an explicit `RenderableContent::from(...)`.

**Step 3.4 — `error_report::AgentErrorReport`**
- Build the composed body exactly as today (`Compose::default()` + title, optional body list, optional hint, optional suggestions), then wrap it in `StatusBlock`:
  ```rust
  let block = StatusBlock::new(StatusState::Error)
      .body(compose)
      .border_color(border_color)
      .left_margin(Margin::Chars(2))
      .right_margin(Margin::Chars(2));
  log::message("");
  log::message(&block.render(term));
  log::message("");
  ```
- Preserve the 2-char left/right margins — both are intentional overrides of the new defaults.

**Step 3.5 — imports and cleanup**
- Add `use biscuit_terminal::StatusBlock;` (or the prelude equivalent) at each migrated file.
- Remove now-unused imports (`BlockQuote`, possibly `Status` from site 3.2 if no longer needed locally, etc.).
- Keep `Margin`, `Tailwind`, `Color`, `Prose` imports where still used.

**Validation checkpoint (gate to Phase 4):**
- `cargo build -p claudine-cli` (or `cargo build -p claudine` — use whichever name matches the CLI crate; confirm via `cargo metadata`).
- `cargo clippy -p claudine-cli -- -D warnings` — no unused imports, no deprecation warnings leaking out.
- `cargo test -p claudine-cli` — all existing tests pass; any existing snapshot/golden tests that touch these render paths either still match or are updated intentionally (with rationale in the commit message).
- Manual smoke (authoritative — spec is explicit that output must be visually equivalent):
  - Trigger a shell expansion error in a `compose` / `inline-compose` dry run and visually confirm the output still renders a red-bordered block with header, body, and hint.
  - Trigger an `AgentErrorReport` path (e.g. a forced provider error in a wrapper run) and confirm the 2-char margins, red border, title, body list, hint, and suggestions all render as before.
  - Trigger a `render_error_block` and `render_warning_header_and_body` path from the live sink (simulate an API error and a file tool warning) and confirm colors and glyphs match pre-migration output.

## Phase 4 — Documentation and final verification

**Goal:** update the two docs in the spec checklist and run the repo-wide validation sweep.

**Files touched:**
- `.claude/skills/biscuit-terminal/SKILL.md`
- `biscuit-terminal/README.md`

**Steps:**

1. Update `.claude/skills/biscuit-terminal/SKILL.md`:
   - Add `StatusBlock` to the components catalog alongside `Status`, `BlockQuote`, `Prose`, `Compose`.
   - Mention the aliased-deprecation story for `StatusState::Error` / `StatusState::Failure` and the new `StatusState::default_color()` helper.
   - Keep the entry compact (<200 lines budget still applies to the SKILL entry point). Link to `biscuit-terminal/README.md` for deeper usage examples.
2. Update `biscuit-terminal/README.md`:
   - Add a `StatusBlock` section showing the canonical builder usage (`header` + `body` + `hint`), the severity-derived color table from the spec, and the two override knobs (`border_color`, `border`).
   - Document that `StatusState::Failure` is deprecated in favor of `StatusState::Error` and that persisted JSON still deserializes via `#[serde(alias = "Failure")]`.
   - Note that `StatusBlock` defaults (`left_margin=0`, `right_margin=5`, `word_wrap=WrapProse(Some(8), None)`) are chosen to align the `▌` border with a preceding `Status` icon header.
3. **Do not** add entries to `docs/dependencies.md` (no new crates) or to other READMEs (no other public behavior changed).

**Final validation checkpoint (phase-exit):**
- Re-run the Phase 2 and Phase 3 command sets (`cargo build`, `cargo test`, `cargo clippy`) for both `biscuit-terminal` and the Claudine CLI crate.
- `just test` at the `biscuit-terminal` area and at the `claudine` area (if each has a justfile — per the repo conventions, confirm before running).
- Sanity-skim the rendered SKILL/README by viewing them in a terminal renderer (`bf` or equivalent) to catch obvious markdown breakage.
- Tick every box in the spec's Implementation checklist (lines 361–372) against the actual diff before declaring the feature done. Flag any unchecked item as a blocker for merge.

## Out-of-Scope Reminders

- Migrating the ~23 `StatusState::Failure` references across the wider workspace and deleting the `Failure` variant — follow-up PR.
- Refactoring `ICON_LOOKUP` in `Status` to consume `default_color()` — follow-up PR.
- Any color rebalancing of `render_warning_header_and_body` (it keeps `Orange700`, not the new `Orange500` default).
- Any change to wrapper-run behavior, dispatch, or streaming — this feature is purely presentational.
