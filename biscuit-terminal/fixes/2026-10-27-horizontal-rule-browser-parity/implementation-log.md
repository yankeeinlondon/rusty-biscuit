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
source_files_during_phase_2:
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-terminal/lib/Cargo.toml
  - biscuit-terminal/justfile
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
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

## Phase 2

Phase 1 had already completed "Migrate surviving browser tests" and "Regenerate snapshots" (see `decisions_for_review`), so this phase added only the fragment contract tests. Production code is unchanged, and no snapshot changed in this phase.

### Changes

- **`horizontal_rule/mod.rs`, ungated `mod tests`:** added a `fragment_html` helper and eight tests. Each one asserts on the public `render_html_fragment().render()`, and no name starts with `browser_`, so all eight stay in L1.

### Requirement → test mapping

- The rule routes through the tree adapter: `test_fragment_routes_through_tree_adapter`. A rule that sets every field plus a nondefault outer margin renders byte-identical HTML through `render_html_fragment()` and through `BrowserTreeComponent::new(..)`.
- `Left`/`Right`/`Centered` keep the authored width and get the matching auto margin: `test_fragment_alignment_controls_svg_margins`.
- `Full` forces `100%` with zero margins, and `width="50%"` is absent: `test_fragment_full_alignment_forces_full_width`.
- The outer layout wraps the rule exactly once: `test_fragment_outer_layout_wraps_rule`.
  - (a) A per-target left margin resolves to the Browser value `5ch` exactly once, and the Terminal value `2ch` never appears. A vertical `ch` becomes `2lh`, and the SVG sits inside the wrapper.
  - (b) `width:50%`, `max-width:40ch`, and centering auto margins all land on a single wrapper.
- A default layout adds no wrapper: `test_fragment_default_layout_adds_no_wrapper`.
- Unsafe widths fall back to `100%`: `test_fragment_unsafe_width_falls_back_to_full`. It uses `Left` alignment so that `Full` cannot mask the fallback. The quote and `onload` payload and the semicolon payload are both absent from the output.
- Unsafe colors fall back to `currentColor`: `test_fragment_unsafe_color_falls_back_to_current_color`. It covers the quote and `onmouseover` payload and the `;background:url` payload.
- The positive control is `test_fragment_safe_authored_values_pass_through`. `tomato` and `#336699` pass through with `width="50%"`, so a validator that rejected every value would fail this test.
- The 7 styles × 3 weights stay covered by the snapshot sweep that Phase 1 migrated, and the page composition by `lib/tests/l1/html_page_example.rs`.
- The Input Robustness Matrix does not apply: these are renderer tests, as the plan states.

### Gates

- Plan checkpoint: `cargo nextest run -p biscuit-terminal --features image,serde -E 'test(fragment_) + test(render_to_browser)'` passed 27/27. On the first run, a Full-alignment assertion of mine failed on `y1="50%"`, a test defect rather than a rendering one, and I narrowed it to `width="50%"`.
- `just test` (biscuit-terminal): 3927 run, 3927 passed, 55 skipped (Phase 1 had 3919, so this adds the 8 new tests).
- `just lint` (biscuit-terminal): passed.
- Downstream: I ran no dependent-package tests, because only `#[cfg(test)]` code changed and no dependent can observe the change.
- Cross-OS: I did not run a cross-check. The new tests are pure string assertions with no OS-specific code.
- Skill: no change. The biscuit-terminal skill does not describe HorizontalRule browser output.
- Affected scope (`git status … | xargs python3 scripts/ci/affected_scope.py`): `packages` = `[biscuit-terminal, test-toolkit]`, `change_class` = `package`, so both gates ran (above).

## Phase 3

L1 now selects the `serde` feature, so the four horizontal-rule serialization tests run in every L1 recipe and CI L1 cell. No CI workflow, tier, or harness feature changed.

### Changes

- **`lib/Cargo.toml`, `[package.metadata.ci.tests]`:** `features = ["image", "serde", "terminal-tests", "browser-tests"]` and `local-features = ["image", "serde"]`. The policy comment's first sentence now reads "L1 runs with the `image` and `serde` features"; the rest of the comment is unchanged. `serde = []` is a pure cfg switch (`serde`/`serde_json` are already unconditional dependencies), so the dependency graph and the `--no-default-features` build are unaffected.
- **`justfile`:** the library entries of `sanity`, `test`, `coverage`, and `doctest` pass `--features image,serde`. `test-l2`, `test-browser`, and every CLI entry are unchanged. The comment block above them drops the obsolete "NOT `--all-features`" paragraph (the serde module compiles since Phase 1), no longer says `image` gates horizontal-rule parity suites, and no longer quotes per-feature test counts.

### Requirement → test mapping

- The serialization tests are selected by L1: the `just test` listing contains `serde_roundtrip::test_rule_style_serde_roundtrip`, `test_rule_style_serialized_as_kebab_case`, `test_rule_alignment_serde_roundtrip`, and `test_rule_weight_serde_roundtrip`, all passing. The run total rose from 3927 (Phase 2) to 3931, exactly these four.
- The relocated `render_text_tier_matches_render_when_image_unavailable` appears in the same listing and passes.
- No new test was written: this phase changes build selection only, and the observable result is the run listing above.

### Gates

- `just test` (biscuit-terminal): 3931 run, 3931 passed, 55 skipped.
- `just lint`: passed. `just doctest`: passed (now with `image,serde`).
- `cargo check -p biscuit-terminal --tests --all-features`: compiles clean.
- `just check-tier-coverage biscuit-terminal`: 0 stranded.
- Downstream: no dependent-package tests run. Only this package's test metadata and justfile changed; no compiled code a dependent reads changed.
- Cross-OS: not run; feature selection carries no OS-specific risk.
- Skill: no change. The skill's testing notes mention only the `terminal-tests`/`browser-tests` tier features, which are unchanged.
- Affected scope (`git status … | xargs python3 scripts/ci/affected_scope.py`): `packages` = `[biscuit-terminal, test-toolkit, tree-hugger-cli]`, `change_class` = `package`, so the gates ran (above). `tree-hugger-cli` is selected as a test-input reader: `test_prelude_flag_with_real_prelude_file` runs `hug` inside `biscuit-terminal/lib`. I ran it alone with `cargo nextest run -p tree-hugger-cli -E 'test(test_prelude_flag_with_real_prelude_file)'`, and it passed 1/1.
