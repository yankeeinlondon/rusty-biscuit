---
agent: claude/
phases: 5
created: 2026-06-24
start_phase: 1
yolo: true
packages:
  - biscuit-tui
source_files_during_phase_1:
  - biscuit-tui/lib/src/core/split_pane.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/prelude.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .opencode/skill/biscuit-tui/SKILL.md
  - .claude/skills/biscuit-tui/SKILL.md
source_files_during_phase_2:
  - biscuit-tui/lib/src/core/split_pane.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_one/tests.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .opencode/skill/biscuit-tui/SKILL.md
  - .claude/skills/biscuit-tui/SKILL.md
source_files_during_phase_4:
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/tests/public_api_names.rs
docs_updated_during_phase_4:
  - biscuit-tui/docs/components/index.md
  - biscuit-tui/lib/CHANGELOG.md
  - biscuit-tui/README.md
  - biscuit-tui/lib/README.md
docs_created_during_phase_4:
  - biscuit-tui/docs/components/split_pane.md
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - biscuit-tui/cli/tests/keyboard_protocol.rs
  - biscuit-tui/cli/tests/completions_shell.rs
  - biscuit-tui/cli/tests/real_terminal_render.rs
  - biscuit-tui/cli/tests/choose_cli.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
  - biscuit-tui/lib/src/core/split_pane.rs
  - biscuit-tui/lib/src/core/mod.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_one/tests.rs
  - biscuit-tui/lib/src/lib.rs
  - biscuit-tui/lib/tests/public_api_names.rs
  - biscuit-tui/cli/tests/keyboard_protocol.rs
  - biscuit-tui/cli/tests/completions_shell.rs
  - biscuit-tui/cli/tests/real_terminal_render.rs
  - biscuit-tui/cli/tests/choose_cli.rs
documentation:
  - biscuit-tui/docs/components/index.md
  - biscuit-tui/lib/CHANGELOG.md
  - biscuit-tui/README.md
  - biscuit-tui/lib/README.md
  - biscuit-tui/docs/components/split_pane.md
---

# SplitPane — Execution Plan

Derived from [`spec.md`](./spec.md) (status: *ready for planning and
implementation*; all open questions D1–D9 **RESOLVED**).

## Goal & Success Criteria

Ship a geometry-only `SplitPane` layout primitive in `biscuit-tui/lib`, plus the
spec's explicitly-scoped `ChooseOne` companion accessors that make the
master/detail pattern (§6.4) usable. Done means:

- `biscuit_tui::core::{SplitPane, SplitDirection, SplitRatio}` exist, are
  re-exported through the prelude, and produce two child `Rect`s that satisfy
  the §7.1 acceptance invariant for every area/config.
- `ChooseOneState::active_option()` / `active_value()` (+ optional
  `active_description`) exist with the §6.4 disabled-passthrough semantics.
- All new L1 unit tests pass under `just test`; `just lint` is clean.
- Public docs, CHANGELOG, READMEs, and the public-API name guard are updated.
- No render wrapper, no CLI command, no `cargo fmt` write-mode (per spec
  non-goals and repo formatting authority).

## Architectural Anchors (verified against current code)

- **Module home** (D7): new file `lib/src/core/split_pane.rs`, declared
  `pub mod split_pane;` in `lib/src/core/mod.rs` alongside the other 9 core
  modules; mirrors `FrameChrome` (`core/frame.rs`).
- **Export sites** to touch: `core/mod.rs` (line ~31 re-export block),
  `prelude.rs` (the `crate::core::{…}` block), `lib.rs` (module-doc surface
  list), `lib/tests/public_api_names.rs`.
- **`ResolvedAxis` stays crate-private** (D3/D7); `SplitDirection::resolve` is a
  private fn.
- **Companion accessors** live in `impl<V: Clone + PartialEq> ChooseOneState<V>`
  in `lib/src/components/choose_one.rs` (next to `hover()` @188–196,
  `options()` @183–186, `selected_value()` @210–215). `ChoiceOption<V>` is
  **not modified** (D-companion #2).
- **Master/detail Prose bridge is a docs note only** (companion #3). `ansi-to-tui`
  and `biscuit-terminal` are **not** added as `biscuit-tui` deps. The lib has no
  `[dev-dependencies]` section today and this plan adds none — the §6.4 example
  ships as documented illustration, not a compiled example. (Flag below.)

## Decision Flags / Assumptions (surface before coding — Rule 1)

1. **No compiled master/detail example.** The Prose→ansi-to-tui→Paragraph
   bridge is documented in `split_pane.md` as illustrative code, matching the
   spec's "example/dev-only, not library deps" stance. If a *runnable, compiled*
   example is wanted, it requires a `[dev-dependencies]` section
   (`ansi-to-tui`, `biscuit-terminal`) — out of scope here; raise if desired.
2. **`split()` defends public fields.** Because `SplitPane`'s enum fields are
   `pub` (spec §4.3 keeps them public for pattern-matching), `split()` itself
   re-normalizes the ratio (clamp `Percent` to `1..=99`, `*Fixed` to `>=1`)
   before building constraints, so a raw `SplitRatio::Percent(0)` struct literal
   cannot bypass the invariant (spec §4.2/§4.3 explicitly require this).
3. **Spare-cell post-adjust may be required.** ratatui's solver rounding for
   `Percentage(50)/Percentage(50)` is not assumed; Task 1.4 verifies the 9⇒5/4
   behavior empirically and adds a post-adjust step only if the solver gives the
   spare to the second pane (spec §4.3 implementation note).

---

## Phase 1 — Geometry Core: types, normalization, and `split()`

Goal: a compiling, exported `SplitPane` core. Pure rectangle math, no widgets.

- [x] **1.1** Create `lib/src/core/split_pane.rs` with the module doc (`//!`),
      and define `SplitDirection` (pub enum: `Auto` [#default], `Horizontal`,
      `Vertical`) with derives `Debug, Clone, Copy, PartialEq, Eq, Hash, Default`
      and the rustdoc from spec §4.1. Use `## Examples`/`## Notes` H2 convention,
      no H1.
- [x] **1.2** Add the crate-private `enum ResolvedAxis { Horizontal, Vertical }`
      and the private `impl SplitDirection { fn resolve(self, area: Rect) ->
      ResolvedAxis }` per spec §4.1/D9: explicit axes pass through; `Auto` maps
      via raw-cell `area.width >= area.height` ⇒ `Horizontal`, else `Vertical`
      (the `>=` tie-break sends square ⇒ `Horizontal`).
- [x] **1.3** Define `SplitRatio` (pub enum: `Percent(u8)`, `FirstFixed(u16)`,
      `SecondFixed(u16)`) with derives `Debug, Clone, Copy, PartialEq, Eq, Hash`,
      a `Default` impl returning `Self::percent(50)`, and the normalizing
      constructors `percent(p) -> clamp(1,99)`, `first_fixed(n) -> max(1)`,
      `second_fixed(n) -> max(1)` (spec §4.2, D5).
- [x] **1.4** Define `SplitPane` (pub struct: `direction: SplitDirection`,
      `ratio: SplitRatio`, `gap: u16`, all fields `pub`) with derives
      `Debug, Clone, Copy, PartialEq, Eq, Hash, Default`. Add builders
      `new()`, `with_direction()`, `with_ratio()` (re-normalizes its arg),
      `with_gap()`.
- [x] **1.5** Implement `SplitPane::split(&self, area: Rect) -> (Rect, Rect)`:
      resolve direction → `ResolvedAxis` → ratatui `Direction`; build
      `[Constraint]` from a **re-normalized** ratio
      (`Percent(p)`⇒`[Percentage(p), Percentage(100-p)]`;
      `FirstFixed(n)`⇒`[Length(n), Min(1)]`; `SecondFixed(n)`⇒`[Min(1),
      Length(n)]`); apply `gap` via `Layout::spacing(gap)` (ratatui 0.30); return
      `(first, second)` with `first` = left/top.
- [x] **1.6** Empirically confirm the spare-cell rule (9 cells @ 50/50 ⇒ first 5,
      second 4). If ratatui allocates the spare to the second pane, add a
      post-adjust that shifts one cell to the first pane along the split axis
      (spec §4.3 note). Confirm the cross axis passes through to both panes at
      `area`'s full extent.
- [x] **1.7** Handle degenerate cases inside `split()` so it never panics/
      overflows (spec §5.2): zero-sized `area` ⇒ both zero; `*Fixed` length ≥
      available ⇒ fixed clamped to available, flex collapses to zero; `gap ≥`
      split-axis length ⇒ clamp gap to that length, both panes collapse to zero.
- [x] **1.8** Wire exports: in `core/mod.rs` add `pub mod split_pane;` and
      `pub use split_pane::{SplitPane, SplitDirection, SplitRatio};`; in
      `prelude.rs` add the same three to the `crate::core::{…}` block. Keep
      `ResolvedAxis` unexported.

**Checkpoint 1:** `cargo check -p biscuit-tui` compiles; the three types resolve
from both `biscuit_tui::core` and `biscuit_tui::prelude`. (Verified fully in
Phase 5; a local `cargo check -p biscuit-tui` here is the fast gate.)

---

## Phase 2 — Geometry Unit Tests (L1, no terminal)

Goal: lock every behavioral contract from spec §7.2 + the §7.1 invariant.
Depends on Phase 1.

- [x] **2.1** Add an inline `#[cfg(test)] mod tests` in `split_pane.rs` (small
      file ⇒ inline module per repo convention) with a helper that asserts the
      §7.1 acceptance invariant for a `(SplitPane, Rect)` pair: both rects within
      `area`; non-overlapping modulo `gap`; split-axis `first.len + gap +
      second.len == area.len` where the area allows; cross-axis full extent on
      both; no panic/overflow.
- [x] **2.2** Direction & ratio tests: 50/50 halves an even area; odd length
      gives the **first** pane the spare (9⇒5/4); explicit `Horizontal` vs
      `Vertical` split the expected axis.
- [x] **2.3** `Auto` resolution tests: wide area ⇒ `Horizontal`; tall ⇒
      `Vertical`; **square ⇒ `Horizontal`** (the `>=` tie-break, named test per
      D9).
- [x] **2.4** Fixed-ratio tests: `FirstFixed`/`SecondFixed` honor the fixed pane
      exactly and flex the other to `Min(1)` where area allows; `*Fixed(0)`
      clamped to `>=1` on construction; `with_ratio`/`split` re-normalize a raw
      `SplitRatio::*Fixed(0)` / `Percent(0)` literal.
- [x] **2.5** Degenerate named tests: the single zero-pane exception (`*Fixed`
      length ≥ available ⇒ flex collapses, no overflow/panic); `gap ≥` split-axis
      length clamps and collapses both panes (named); `0×0` and `1×N` areas never
      overflow/panic and stay within `area`.
- [x] **2.6** Gap tests: `gap` reduces total before division and lands between
      panes; with a `*Fixed` ratio the fixed pane keeps exact `n` and the
      **flexible** pane absorbs the gap; spare-cell survives a gap (odd `gap` +
      odd remaining axis under `Percent(50)` ⇒ spare to **first** pane).
- [x] **2.7** `Percent` boundary clamping at `0`/`100` into `1..=99`.
- [x] **2.8** Invariant sweep: run the 2.1 helper across a representative spread
      of areas (incl. tiny, wide, tall, square, odd) × configs (each direction ×
      each ratio variant × gap ∈ {0,1,large}).

**Checkpoint 2:** `just test` passes all new geometry tests (run in Phase 5;
locally `cargo nextest run -p biscuit-tui split_pane` for the fast gate).

---

## Phase 3 — `ChooseOne` Companion Accessors (parallelizable with Phases 1–2)

Goal: the spec-scoped active-item accessors enabling master/detail. **Independent
of SplitPane geometry** (different file: `components/choose_one.rs`) — may run
concurrently with Phases 1–2; only the docs/CHANGELOG in Phase 4 join the two.

- [x] **3.1** In `impl<V: Clone + PartialEq> ChooseOneState<V>`
      (`components/choose_one.rs`), add `active_option(&self) ->
      Option<&ChoiceOption<V>>` keyed off `hover()`:
      `self.hover().and_then(|i| self.options().get(i))`. Semantics: returns the
      option at the highlight **as-is**, including a `disabled` one; `None` when
      no active row (empty list). Rustdoc must state the disabled-passthrough +
      `None`-means-no-active-row contract (spec companion #1).
- [x] **3.2** Add `active_value(&self) -> Option<&V>` =
      `self.active_option().map(|o| &o.value)`.
- [x] **3.3** (Optional, spec companion #2) Add convenience
      `active_description<'a>(&self, map: &'a HashMap<String, String>) ->
      Option<&'a str>` performing the `active_option().id → map` lookup —
      documented as sugar over `active_option()`, **not** a new data model;
      `ChoiceOption` stays unchanged.
- [x] **3.4** Tests in `components/choose_one/tests.rs` (match existing
      `fixture_input()` / `press()` style): empty options ⇒ `None`; initial
      active row returns the first option/value; active row moves with
      `Down`/`Up` navigation; **disabled-option contract** — when the highlight
      rests on a disabled option, `active_option()`/`active_value()` still return
      it (use existing navigation, invent no new focus rules).

**Checkpoint 3:** `cargo nextest run -p biscuit-tui choose_one` passes the new
accessor tests; existing ChooseOne tests stay green.

---

## Phase 4 — Public-API Guard, Docs, CHANGELOG & READMEs

Goal: every live public surface that advertises core exports reflects SplitPane
+ the companion accessors (spec §4.4, §7.3). Depends on Phases 1 & 3 (final
type/accessor names).

- [x] **4.1** Extend `lib/tests/public_api_names.rs`: import `SplitPane,
      SplitDirection, SplitRatio` from `biscuit_tui`, and in
      `canonical_public_names_compile()` add binding-style assertions matching
      the file's pattern (e.g. `let _sp = SplitPane::new();`, `let _dir =
      SplitDirection::Auto;`, `let _ratio = SplitRatio::percent(50);`).
- [x] **4.2** Update `lib.rs` module-doc core surface list if it stays explicit
      (it lists representative core primitives) — add SplitPane to the `core`
      bullet examples for accuracy.
- [x] **4.3** Create `docs/components/split_pane.md` following the
      `frame_chrome.md` heading shape (Description / Parameters & Defaults
      [SplitPane, SplitDirection, SplitRatio, gap] / Usage Examples [geometry-only
      §6.1, fixed sidebar §6.2, nesting §6.3, master/detail §6.4] / a "Not a CLI
      command" note). Include the **Prose ⇄ ratatui bridge docs note** (companion
      #3): `TerminalRenderable`s yield an ANSI `String` (no `render_in_width`);
      bridge via `ansi-to-tui` into a `Paragraph`; these deps are example/dev-only,
      not library deps.
- [x] **4.4** Link `split_pane.md` from `docs/components/index.md` under
      **Container Components** (next to FrameChrome / Input Table) so it's
      discoverable as a layout/container primitive.
- [x] **4.5** Update `lib/CHANGELOG.md` `## [Unreleased] → ### Added`: a bullet
      for `core::split_pane::{SplitPane, SplitDirection, SplitRatio}` (geometry-
      only 2-pane layout primitive) and a bullet for the new `ChooseOneState`
      active-item accessors. Match the existing rustdoc-style bullet wrapping.
- [x] **4.6** Update both READMEs' core-primitive lists (`biscuit-tui/README.md`
      Core Primitives section; `biscuit-tui/lib/README.md` Core Primitives
      section) with a `SplitPane` entry describing the geometry-only 2-pane
      split. Note SplitPane is a *container/layout* primitive, not a 7th input
      component (keep the "six input components" count intact).

**Checkpoint 4:** `cargo nextest run -p biscuit-tui --test public_api_names`
passes; docs cross-links resolve.

---

## Phase 5 — Validation & Verification

Goal: prove the goal/success criteria with the repo's canonical commands.
Depends on all prior phases.

- [x] **5.1** From `biscuit-tui/`, run `just test` (L1 lib + CLI). All new
      geometry and accessor tests pass; nothing regresses.
- [x] **5.2** Run `just lint` (clippy on lib + CLI) — required because this
      changes public docs/exports. Resolve warnings without running
      `cargo fmt` write-mode (match surrounding style by hand; repo formatting
      authority is `main`).
- [x] **5.3** Confirm acceptance invariant coverage: the §7.1 invariant sweep
      (Task 2.8) and all named degenerate tests (§5.2) are present and green.
- [x] **5.4** Confirm scope discipline: no render wrapper shipped, no
      `question split-pane` CLI command added, `ChoiceOption` unmodified, no new
      library dependencies, `ResolvedAxis` still crate-private.
- [x] **5.5** Final review pass: rustdoc on new public items follows repo
      convention (no H1 in `///`, `## H2` sections, summary→Examples→…); inline
      comments carry contract/why info only (no HOW-narration, no format-string
      narration).

**Definition of Done:** Checkpoints 1–5 satisfied; `just test` and `just lint`
clean from `biscuit-tui/`; spec §7.1 invariant and all §7.2 enumerated cases
covered by passing L1 tests.

---

## Dependency & Parallelism Summary

```
Phase 1 (geometry core) ─┬─► Phase 2 (geometry tests) ─┐
                         │                              ├─► Phase 5 (validate)
Phase 3 (ChooseOne) ─────┴──────────────────────────────┤
                                                         │
Phase 4 (docs/guard) ◄── needs Phase 1 + Phase 3 ───────┘
```

- **Parallelizable now:** Phase 3 is independent of Phases 1–2 (disjoint files)
  — assign to a separate worker immediately.
- **Within Phase 1:** Tasks 1.1–1.4 (type defs) can be drafted together; 1.5–1.7
  depend on them; 1.8 (exports) last.
- **Within Phase 2:** all test tasks are mutually independent once 2.1's helper
  exists.
- **Serial gates:** Phase 4 waits on final names from 1 & 3; Phase 5 waits on all.
