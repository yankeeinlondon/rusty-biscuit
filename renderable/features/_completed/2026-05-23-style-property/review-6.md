---
ready: true
agent: codex
model: ""
---

# Review: `style:` Frontmatter Sub-Spec #6

## Resolution (2026-05-24)

All three High findings are resolved:

1. **Strict-style alias detection** — `merge_deprecated_top_level_hr` now
   emits `Deprecated` warnings on alias *presence* rather than on successful
   typed migration. Non-mapping `hr:`, empty `hr: {}`, and invalid-but-
   legacy-tolerated shapes (e.g. `alignment: true`) all trip strict mode.
   Coverage: `into_strict_fails_on_top_level_hr_alias_with_invalid_typed_field`,
   `into_strict_fails_on_non_mapping_top_level_hr`, and
   `into_strict_fails_on_empty_top_level_hr_mapping` in
   `darkmatter/lib/src/style/parse.rs`; CLI counterparts in
   `darkmatter/cli/tests/cli.rs`.
2. **Browser HR bg-color wiring** — the SVG renderer now emits
   `<svg class="darkmatter-hr">` and `component_selectors(PageComponent::Hr)`
   returns `.darkmatter-hr`, so the generated rule
   `.darkmatter-page .darkmatter-hr { background-color: … }` matches the
   actual rendered element. Coverage: `browser_hr_bg_color_targets_rendered_element`
   and `browser_hr_color_emits_rule_for_svg_target` in
   `darkmatter/lib/src/layout/page.rs`.
3. **HR visual behavior verification depth** — six Level 2 tests added in
   `darkmatter/cli/tests/level2_layout.rs` covering `style.hr.kind` (waves),
   `weight` (thick vs thin), `alignment` (center), `width`, `color`, and
   `bg-color` through the canonical `style.hr.*` frontmatter path via the
   real `md` CLI in a WezTerm pane.

## Original Findings

### High: `--strict-style` can miss deprecated top-level `hr:` alias usage

The spec requires strict mode to reject deprecated top-level `hr:` syntax before producing final output. The parser currently maps the whole top-level `hr` object through typed `HrStyle` deserialization, and if any field fails that typed deserialize it returns without emitting any deprecation warning ([darkmatter/lib/src/style/parse.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/style/parse.rs:299)). That means documents such as:

```yaml
---
hr:
  style: waves
  alignment: true
---
```

can avoid the `Deprecated` warning path entirely, even though the renderer still has the legacy top-level fallback path. Non-mapping `hr:` values also return with no warning. This violates the strict-style requirement because alias presence, not successful typed migration, is what must be rejected.

Verification level present: Level 1 only, and only for a fully typed-compatible alias (`hr: { style: waves }`). Required: Level 1 strict tests for invalid-but-legacy-tolerated top-level `hr:` shapes and non-mapping `hr:`, plus the existing CLI strict-path coverage.

### High: `style.hr.bg-color` does not apply to browser HR output

`style.hr.bg-color` is stored as `PageComponent::Hr` background color and browser page CSS emits component background rules using `component_selectors(PageComponent::Hr) == "hr"` ([darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:1318), [darkmatter/lib/src/layout/page.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/layout/page.rs:1333)). But the HR browser renderer emits raw `<svg>`, not `<hr>` ([biscuit-terminal/lib/src/components/horizontal_rule/browser.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/components/horizontal_rule/browser.rs:204)). The generated `.darkmatter-page hr { background-color: ... }` rule therefore matches no element and the browser-visible HR background is dropped.

This misses acceptance criterion #11 for the browser target. Fix by targeting the emitted SVG via a stable class/wrapper, or by wrapping HR output in an element that the component background mechanism can style.

Verification level present: Level 1 storage tests only. Required: browser computed-style coverage proving `style.hr.bg-color` affects the actual rendered HR element/wrapper.

### High: HR visual behavior is not verified at the required level

Sub-spec #6 is mostly user-observable rendering behavior: `style.hr.kind`, `weight`, `width`/`max-width`, `alignment`, `color`, and `bg-color` must affect terminal and browser HR output; inline `kind` must override frontmatter; deprecated inline `style` must still render while strict mode rejects it. The implementation has useful unit tests for schema parsing, page state, and builder mapping, and there is an older Level 2 render-tree test for legacy inline `--- { style: waves }`. I did not find Level 2 coverage for the canonical `style.hr` frontmatter path, HR weight/width/alignment/color/bg-color in a real terminal, or browser computed-style tests for SVG stroke/fill/background.

Per the requested rigor model, terminal glyph/styling/width/alignment behavior needs Level 2 real-terminal capture, and browser CSS behavior should be verified in a real browser through computed style. Current strongest coverage for many user-facing HR requirements is Level 1, so the feature cannot be marked production-ready.

Suggested coverage:

- Level 2 terminal: `style.hr.kind: waves`, `weight: thick`, `alignment`, `width`/`max-width`, `color`, and `bg-color` through the public CLI/page path.
- Level 1 CLI strict: inline legacy `--- { style: waves }` and top-level `hr:` alias, including invalid legacy-tolerated shapes.
- Browser computed style: canonical `style.hr.color` changes SVG stroke/fill, and `style.hr.bg-color` changes the actual HR wrapper/background.

## Notes

The core schema shape is in place: `HrKind`, `HrWeight`, `HrAlignment`, `PageComponent::Hr`, `apply_hr_style`, and inline `kind`/legacy `style` provenance all exist. The remaining blockers are alias-warning robustness, browser background wiring, and verification depth.

## Verification

Review-only pass. I inspected the spec and implementation; I did not run the full test suite.

## Production Readiness

Not ready. The implementation is close, but strict-style alias rejection has an escape path, browser `bg-color` is not wired to the emitted element, and user-visible HR behavior does not meet the required verification levels.
