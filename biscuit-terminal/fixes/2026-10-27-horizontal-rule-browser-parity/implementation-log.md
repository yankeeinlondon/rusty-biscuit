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
  - |-
    Phase 4 was planned as documentation only, but its checkpoint (and the
    Phase 5 drift sweep) requires that searching the library source for the
    old method name `render_to_browser` finds nothing. The only remaining
    matches were the names of ten tests that earlier phases kept after
    migrating them to the new fragment API. I renamed them from
    `test_render_to_browser*` to `test_render_html_fragment*` (and
    `test_snapshot_render_to_browser_all_styles` to
    `test_snapshot_render_html_fragment_all_styles`); their bodies and
    snapshot files are unchanged. The alternative was to leave the names and
    accept a failing checkpoint, or loosen the search pattern. If a reviewer
    prefers the old names, revert the rename in
    `lib/src/components/horizontal_rule/mod.rs`; nothing else depends on it.
  - |-
    The plan's last task said to create the implementation log and not to
    modify the spec. The log already existed (earlier phases wrote it), so
    Phase 5 appended to it instead. The pipeline's final-phase instructions
    require marking the spec as implemented, so the spec's `implemented` flag
    was set to true and `implemented_by` was added; no spec content changed.
    If the reviewer wants the spec untouched, revert those two frontmatter
    keys; nothing else depends on them.
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
source_files_during_phase_4:
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
docs_updated_during_phase_4:
  - biscuit-terminal/docs/components/horizontal-rule.md
  - biscuit-terminal/docs/components/browser-renderable-trait.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - biscuit-terminal/lib/src/components/horizontal_rule/browser.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/color.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/snapshots/
  - biscuit-terminal/lib/Cargo.toml
  - biscuit-terminal/justfile
documentation:
  - biscuit-terminal/docs/components/horizontal-rule.md
  - biscuit-terminal/docs/components/browser-renderable-trait.md
completed_phase: 5
implemented: true
source_files_during_phase_5: []
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
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

## Phase 4

Both component docs now describe the single path that actually exists: the rule is projected into the render tree and drawn by the shared browser renderer. All facts were checked against the code: the trait in `renderable/src/browser/renderable.rs`, the margins and validation in `renderable/src/tree/graphics.rs`, and `render_image_tier`/`render_image_svg` in `horizontal_rule/mod.rs`.

### Changes

- **`docs/components/horizontal-rule.md`:**
  - both usage examples call `rule.render_html_fragment().render()`
  - the nonexistent `render_to_browser_with_inline_variables` subsection is deleted
  - the BrowserRenderable section explains the delegation, with a Mermaid flow (rule → `ThematicBreak` node → `BrowserTreeComponent` → optional wrapper `<div>` → inline SVG), the three-line delegation, and the `GraphicsMode::Off` `<hr>` alternative
  - a new "Alignment" table gives the SVG left/right margins and width for each `RuleAlignment`, and a new "`Full` ignores the authored width" subsection covers the default `Full` (always `100%`, no side margins) and how to keep an authored width
  - a new "Rule Alignment vs Outer Layout" section: `.alignment()` sets the SVG margins, while `with_layout` adds exactly one wrapper (none for a default layout). Per-target values resolve for the browser.
  - a new "Invalid Width and Color Values" section lists the accepted forms (mirroring `is_safe_css_dimension`/`is_safe_css_color`), the `100%`/`currentColor` fallbacks, and the injection examples. It notes that `tomato` still renders in the browser.
  - the "raw string is preserved for the browser target" sentence is replaced
  - `Renderable` → `TerminalRenderable`
  - Tier 1: the gate is now "Kitty or ITerm", and the doc says the terminal builds its own pixel-sized SVG with a white/black default color rather than reusing the browser SVG
  - the CSS Variables section is kept, and its table now says "if set and valid"
- **`docs/components/browser-renderable-trait.md`:** the trait definition is replaced with the real one (`render_html_fragment() -> BrowserFragment<Ready>`, default `render_html_page`, `as_any`) along with per-method notes. The HorizontalRule example is the actual delegated impl. The guidelines now add "prefer the render tree" and "validate untrusted values" and drop the CSS-variable/perf guidance that described the old hand-written SVG. The CSS-variable table is kept.
- **`lib/src/components/horizontal_rule/mod.rs` (test names only):** I renamed ten tests from `test_render_to_browser*` / `test_snapshot_render_to_browser_all_styles` to `test_render_html_fragment*` / `test_snapshot_render_html_fragment_all_styles` so that the phase checkpoint's search finds nothing (see `decisions_for_review`). Test bodies and snapshot files are unchanged, since the snapshots use explicit names.
- The plan said to "keep the `blast_radius` frontmatter" on `horizontal-rule.md`, but the page had no frontmatter, so there was nothing to keep and none was added.

### Requirement → test mapping

- These are docs only, so they have no behavior to test. Every documented claim has existing tests behind it:
  - alignment margins: `test_fragment_alignment_controls_svg_margins`
  - `Full` width: `test_fragment_full_alignment_forces_full_width`
  - wrapper once / no wrapper: `test_fragment_outer_layout_wraps_rule` and `test_fragment_default_layout_adds_no_wrapper`
  - fallbacks: `test_fragment_unsafe_width_falls_back_to_full` and `test_fragment_unsafe_color_falls_back_to_current_color`
  - `tomato`: `test_fragment_safe_authored_values_pass_through`
  - delegation: `test_fragment_routes_through_tree_adapter`
- Renamed tests: `cargo nextest run -p biscuit-terminal --features image,serde -E 'test(render_html_fragment)'` passed 12/12 (the 10 renamed plus 2 filesystem tests with matching names).
- The shipped docs are read by darkmatter's corpus tests, so I ran `cargo nextest run -p darkmatter -E 'test(shipped_markdown_inline_schemas_parse_passively) + test(shipped_markdown_named_type_references_resolve)'`: 2/2 passed.

### Gates

- Checkpoint: `rg -n "render_to_browser|render_browser_svg|MarginToCss" biscuit-terminal/docs/ biscuit-terminal/lib/src/` finds nothing. Before the test rename it matched the 10 test names.
- Skill check: `rg -n -i "horizontalrule|darkmatter-hr|--hr"` over both skill copies matches only `image-rendering.md:223` (the Kitty `q=2` note for the graphical HorizontalRule). The Phase 1 log already found that note, and it is still accurate, so no skill edit was made.
- Affected scope (`git status … | xargs python3 scripts/ci/affected_scope.py`): `packages` = `[biscuit-terminal, test-toolkit, darkmatter]`, `change_class` = `package`, so the gates ran. `darkmatter` is selected as a test-input reader of `biscuit-terminal/docs` (the corpus tests above). `test-toolkit` is selected only for its lint companion suite, as in earlier phases.
- `just test` (biscuit-terminal): 3931 run, 3931 passed, 55 skipped, the same count as Phase 3.
- `just lint` (biscuit-terminal): passed.
- Downstream: no other dependent tests. The source change is test names under `#[cfg(test)]`, which no dependent can observe.
- Cross-OS: not run. Docs and test renames carry no OS risk.

## Phase 5

This phase verified the work and wrote the log. It changed no source, doc, or skill file; the only edits are the plan checkboxes and the frontmatter on the plan, log, and spec.

### Verification gate

- `just test` (biscuit-terminal): 3931 run, 3931 passed, 55 skipped. The listing includes all five tests that were hidden before: `serde_roundtrip::test_rule_style_serde_roundtrip`, `test_rule_style_serialized_as_kebab_case`, `test_rule_alignment_serde_roundtrip`, `test_rule_weight_serde_roundtrip`, and `render_text_tier_matches_render_when_image_unavailable`.
- Targeted: `cargo nextest run -p biscuit-terminal --features image,serde -E 'test(test_rule_style_serde_roundtrip) + test(test_rule_style_serialized_as_kebab_case) + test(test_rule_alignment_serde_roundtrip) + test(test_rule_weight_serde_roundtrip) + test(render_text_tier_matches_render_when_image_unavailable)'`: 5/5 passed, with each name in the output.
- `cargo check -p biscuit-terminal --tests --all-features`: compiles clean. This is the combination that failed with 28 errors before the fix.
- `cargo check -p biscuit-terminal --no-default-features`: compiles clean.
- `just lint`: passed. `just doctest`: 204 passed, 1 ignored. `just sanity`: 2086 + 83 passed.
- `just check-tier-coverage biscuit-terminal`: 0 stranded.
- `just test-l2` and `just test-browser` were not run, as the plan directs: no harness test changed.

### Acceptance sweep

- Files changed across the whole fix, compared with the commit before Phase 1: `horizontal_rule/{mod.rs,browser.rs,color.rs}`, `lib/Cargo.toml`, `justfile`, the two docs pages, the plan/log/spec, and 22 `browser_*` snapshots. Nothing else changed.
- Snapshots: 22 browser snapshots changed (21 `browser_{style}_{weight}` plus `browser_custom_attributes_2`). `browser_custom_attributes` renders the same as before and is untouched; see `decisions_for_review`. None of the 93 `terminal_*` snapshots changed.
- Drift: `rg -n "render_browser_svg|MarginToCss|to_css_value|render_to_browser"` over `biscuit-terminal/{lib,cli,docs,README.md}` and both biscuit-terminal skill copies finds nothing. The plan's command with `--glob '!fixes/**' --glob '!reviews/**'` still matches files under `fixes/`, `reviews/_completed/`, and `features/_completed/`, because those globs do not exclude nested paths. Every match is in a spec, plan, or review snapshot, which is never updated after it closes, so none was edited.

### Summary of the fix (all phases)

- Phase 1 deleted the legacy browser SVG (`render_browser_svg`, `MarginToCss`) and pointed `BrowserRenderable` at the render tree. It moved the terminal color helpers to `color.rs`, migrated the browser tests to the public fragment, deleted the parity tests, and moved the terminal test that had been misplaced under the serde gate.
- Phase 2 added eight fragment contract tests covering routing, alignment margins, `Full` width, the outer wrapper, the no-wrapper default, unsafe width and color fallbacks, and safe pass-through.
- Phase 3 enabled `serde` in L1, in both the Cargo CI metadata and the justfile.
- Phase 4 rewrote both docs pages to describe the single tree path and renamed the migrated tests.
- Departures from the plan are recorded in `decisions_for_review`: test migration moved into Phase 1, 22 snapshots changed rather than 23, the Phase 4 test rename, and the existing log and spec flag in Phase 5.

### Gates and scope

- Affected scope (`git status … | xargs python3 scripts/ci/affected_scope.py`): `packages` = `[biscuit-terminal, test-toolkit, darkmatter]`, `change_class` = `package`, so the gates ran (above). The working tree still holds Phase 4's uncommitted changes.
- Downstream darkmatter: `cargo nextest run -p darkmatter -E 'test(shipped_markdown_inline_schemas_parse_passively) + test(shipped_markdown_named_type_references_resolve) + test(/horizontal_rule/) + test(/hr_/)'`: 110/110 passed.
- `test-toolkit` is selected only for its `archive-path-guard` lint companion suite, because the justfile changed. It has no L1 tier, so no test was run for it.
- Cross-OS: not run. The fix changes renderer output and test selection only, with no paths, processes, or `cfg(windows)` code.
- Skill: no change. The skill's only HorizontalRule mention (`image-rendering.md`, the terminal image tier) is still accurate.
