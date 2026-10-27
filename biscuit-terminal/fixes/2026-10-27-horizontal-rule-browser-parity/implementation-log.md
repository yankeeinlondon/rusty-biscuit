---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/biscuit-terminal/fixes/2026-10-27-horizontal-rule-browser-parity/spec.md"
plan: "biscuit-terminal/fixes/2026-10-27-horizontal-rule-browser-parity/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
  - biscuit-terminal
source_files_during_phase_1:
  - biscuit-terminal/lib/src/components/horizontal_rule/browser.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/color.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/snapshots/
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
message_to_agent: |-
  Phase 1 already completed two Phase 2 tasks, "Migrate surviving browser
  tests" and "Regenerate snapshots", because deleting render_browser_svg broke
  their call sites. Both are checked off in plan.md. Phase 2 only needs "Add
  fragment contract tests". 22 browser snapshots changed, not 23:
  browser_custom_attributes, a Centered rule, renders identically on the new
  path. `just test` still uses `--features image` only, so the four serde tests
  run only with `--features image,serde` until Phase 3 changes the recipe.
decisions_for_review:
  - |-
    The surviving browser tests were migrated, and their snapshots regenerated,
    in Phase 1, not Phase 2 as the plan scheduled. Deleting the old browser SVG
    method (required by Phase 1) left ten test call sites with nothing to call,
    and Phase 1's exit check requires the tests to compile and pass. Each test
    was migrated exactly as Phase 2's "Migrate surviving browser tests" task
    describes: it calls the public fragment, the 50% test selects Centered, the
    60ch test selects Left and asserts width="60ch", and the percent-margin test
    asserts `margin-top:2%` on the outer wrapper. The alternative was a
    temporary test-only shim that kept the old method alive until Phase 2. That
    would have kept the deleted code reachable and needed a second edit pass.
    If a reviewer prefers the original phase split, nothing needs to be redone;
    only the bookkeeping changes (those two Phase 2 tasks are already checked
    off).
  - |-
    The plan expected 23 browser snapshots to change, but 22 did. The
    `browser_custom_attributes` snapshot (a Centered 75% rule) renders
    identically on the old and new paths, because the old path always centered.
    Its only stale part was the `source:` header line, which insta does not
    rewrite when the content matches, so the file is untouched. No action is
    needed unless a reviewer wants that header refreshed.
---

# Implementation Log for 2026-10-27-horizontal-rule-browser-parity (5 phases)

## Phase 1

### Changes

- **`horizontal_rule/color.rs` (new):** `parse_basic_color`, `parse_hex_color`, and `nearest_basic_color` were moved here verbatim (docs included) from `browser.rs` and remain `pub(super)`.
- **`horizontal_rule/browser.rs`:** this file now holds only `impl BrowserRenderable for HorizontalRule`. `render_html_fragment` delegates unconditionally to `BrowserTreeComponent::new(self.clone()).render_html_fragment()`, and `as_any` returns `self`. `render_browser_svg` and the `MarginToCss` trait and impl are deleted.
- **`horizontal_rule/mod.rs`:**
  - rewrote the module doc to list `color` and `browser`
  - added `mod color;`, and the import now reads `use self::color::{…}`
  - corrected the tree-projection doc: browser output is the `ThematicBreak` rendering (inline SVG under rich graphics, `<hr>` with `data-hr-*` under `GraphicsMode::Off`)
  - replaced the unknown-color warning with the exact text from plan rule 5, and its test now asserts `"Browser rendering validates color values independently"`
- **Test module:**
  - deleted the eight `parity_*` tests, their comment block, and the three `test_margin_*` `to_css_value` tests
  - moved `render_text_tier_matches_render_when_image_unavailable` into the ungated `mod tests` with `#[serial_test::serial(locale_env)]`
  - `serde_roundtrip` now holds exactly the four serialization tests
  - the ten former `render_browser_svg()` call sites now use `render_html_fragment().render()`, adjusted as Phase 2 describes (see decisions_for_review)
- **Snapshots:** 22 `browser_*` snapshots were regenerated, and no `terminal_*` snapshot changed. Diffs were inspected: Full-alignment rules now emit `margin: 0 0 0 0` instead of `0 auto 0 auto`, and `browser_custom_attributes_2` (Right, 50ch) emits `margin: 0 0 0 auto` while keeping `width="50ch"`. The only other change is the stale `source:` header (`horizontal_rule.rs` → `horizontal_rule/mod.rs`).

### Requirement → test mapping

- Legacy path gone and the crate still compiles: `cargo check -p biscuit-terminal --tests --all-features` builds clean (previously 28 errors), `cargo check -p biscuit-terminal --no-default-features` builds clean, and `rg "render_browser_svg|MarginToCss" biscuit-terminal/lib/src` finds nothing.
- Delegated fragment output: `test_render_to_browser*` (7 tests), `test_render_to_browser_percent_margin`, `test_snapshot_render_to_browser_all_styles`, and `test_snapshot_render_with_custom_attributes` all assert on the public `render_html_fragment().render()`. The dedicated routing and contract tests are Phase 2 work.
- Warning text: `test_apply_terminal_color_unknown_name_warns_with_asymmetry_note`.
- Relocated test is ungated: `render_text_tier_matches_render_when_image_unavailable` appears in the plain `just test` (`--features image`) listing and passes.
- Serde tests compile and pass: `cargo nextest run -p biscuit-terminal --features image,serde -E 'test(/horizontal_rule/)'` passes 139/139 after the snapshots were accepted. `just test` does not select them yet; Phase 3 adds `serde` to the recipe.
- Terminal behavior unchanged: every terminal test and `terminal_*` snapshot passes unmodified.

### Gates

- `just test` (biscuit-terminal): 3919 run, 3919 passed, 55 skipped.
- `just lint` (biscuit-terminal): passed.
- Downstream:
  - claudine calls only terminal `render`, which is unchanged
  - darkmatter imports only the unchanged style enums
  - `cargo nextest run -p darkmatter -E 'test(/horizontal_rule/) | test(/hr_/)'`: 108/108 passed
- Affected scope (`git status … | xargs python3 scripts/ci/affected_scope.py`): `packages` = `[biscuit-terminal, test-toolkit, tree-hugger-cli]`, `change_class` = `package`, so the gates ran.
- Skill check: `rg -i "horizontalrule|darkmatter-hr|--hr"` over both biscuit-terminal skill copies finds only a terminal-image-tier note in `image-rendering.md`, which is still accurate. No skill edit was made.
- Cross-OS: no OS-specific code was touched (pure rendering and test changes), so no cross-check ran.
