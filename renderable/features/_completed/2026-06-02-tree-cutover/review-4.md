---
ready: false
agent: codex
model: ""
---

# Review 4 — Tree Cutover

## Verdict

Not ready for production.

Iteration 4 resolves the review-3 deletion blocker mechanically: the legacy
`RuleProcessor` type is gone, the old `output::as_html` / `for_terminal` /
`write_terminal` exports are gone, and the public HTML and terminal entry
points route through the render tree. The remaining blocker is a regression in
the horizontal-rule option surface: public `HtmlOptions` / `TerminalOptions`
fields and `DarkmatterPage` wiring still promise HR defaults and CSS-variable
overrides, but the tree entry points ignore them.

## Findings

### High — HR defaults and `hr_css_variables` are dropped on the tree path

The spec requires no functional or fidelity regressions versus the bespoke
renderers, and the style-frontmatter work made `style.hr.*` / deprecated
top-level `hr:` defaults part of the public render behavior. That behavior is
still exposed in the option structs:

- `darkmatter/lib/src/markdown/output/html.rs:76`-`91` documents
  `HtmlOptions::hr_css_variables` as emitting a `:root` block and
  `HtmlOptions::hr_defaults` as the default HR style source.
- `darkmatter/lib/src/markdown/output/terminal.rs:708`-`712` documents
  `TerminalOptions::hr_defaults` the same way for terminal rendering.
- `DarkmatterPage::render` still sets `options.hr_defaults = self.hr_defaults()`
  at `darkmatter/lib/src/layout/page.rs:858`, and `render_to_browser` threads
  `hr_defaults: self.hr_defaults()` into `HtmlOptions` at
  `darkmatter/lib/src/layout/page.rs:946`.

The new production tree entry points do not consume those fields. In
`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:83`-`84` and
`591`-`592`, the implementation explicitly says
`HtmlOptions::hr_css_variables` is dropped, and
`browser_options_from_html_options` only maps Mermaid mode, page stylesheet,
raw-HTML policy, and the code renderer (`entrypoints.rs:617`-`634`). A
mechanical search shows no render-tree consumer of `HtmlOptions::hr_defaults`
or `TerminalOptions::hr_defaults`; the only remaining references are field
definitions and assignments.

Concrete regressions:

- `HtmlOptions { hr_css_variables: {"hr-width": "42%"}, .. }` no longer emits
  the documented `:root { --hr-width: 42%; }` override.
- `DarkmatterPage` / `style.hr.*` defaults no longer affect bare `---` rules on
  either terminal or browser output, even though the page still computes and
  passes those defaults.
- Deprecated top-level `hr:` compatibility is no longer verified for the direct
  `Markdown::as_html` / `as_terminal` paths.

The tests moved to the public `Markdown::as_*` calls, but the specific
assertions that would catch this were removed rather than ported: frontmatter
defaults for bare rules, partial rule-attribute overrides of defaults, terminal
bare-rule defaults, `hr_css_variables` override emission, and scalar sibling
coercion cases are gone from `horizontal_rule_integration.rs`. The remaining
test at `darkmatter/lib/tests/horizontal_rule_integration.rs:465` only checks
that non-mapping `hr:` frontmatter does not panic; it does not prove valid
defaults are honored.

Verification level: Level 1 is enough to catch the missing option consumption
and HTML source contract. The user-visible browser effect of CSS variables and
terminal rule width/color should also have browser / Level 2 coverage once
restored. Current strongest verification is effectively none for these
requirements after the deleted assertions, so this is a production blocker.

### Medium — Cutover docs still point at deleted validation surfaces

The implementation deletes `darkmatter/lib/benches/migration_parity.rs` and
`darkmatter/lib/tests/render_tree_parity.rs`, and `darkmatter/lib/Cargo.toml`
no longer declares a `migration_parity` bench (`Cargo.toml:111`-`133` lists the
remaining bench targets).

Several local docs still instruct maintainers to use those deleted surfaces or
describe the old blocked state:

- `renderable/features/2026-06-02-tree-cutover/implementation-notes.md:16`
  tells Phase 5 to run `cargo bench -p darkmatter --bench migration_parity`.
- `implementation-notes.md:270`, `551`, and nearby lines still claim
  `render_tree_parity` is part of the green verification set.
- `implementation-notes.md:555`-`556` still says AC1 is partial and the
  decorated terminal path is legacy, which is now stale.
- `darkmatter/lib/benches/render_pipeline_steps.rs:5` says the bespoke-vs-tree
  comparison lives in `migration_parity.rs`.

This does not by itself break runtime behavior, but it weakens the feature's
handoff: the recorded production-readiness workflow is no longer executable.
Update the notes to describe the final post-deletion validation path and remove
stale legacy-state claims.

Verification level: Level 1 file/API inspection is sufficient.

## Requirement Status

| Requirement | Status | Strongest relevant verification |
|---|---:|---|
| `Markdown::as_html` on tree | Implemented | Level 1 API tests plus browser coverage for selected computed styles |
| default `Markdown::as_terminal` on tree | Implemented | Level 2 public-entry real-terminal coverage exists for selected terminal rendering behavior |
| decorated `DarkmatterPage::render` on tree | Implemented | Level 1 decoration tests plus Level 2 layout/page captures |
| delete bespoke darkmatter serializers / `RuleProcessor` | Implemented | Level 1 mechanical search: old public functions/type are gone |
| `style.hr.*` / `hr:` defaults for bare rules | Broken | No current coverage after removed Level 1 assertions; needs restored Level 1 plus terminal/browser coverage where rendering matters |
| `HtmlOptions::hr_css_variables` override contract | Broken | No current coverage after removed assertion; browser-computed coverage would be appropriate for the CSS effect |
| final validation/perf documentation | Stale | Level 1 inspection |

## Notes

I ran:

```bash
cargo test -p darkmatter --test horizontal_rule_integration -- --nocapture
```

It passed (`23 passed`), but that suite no longer contains the HR-default and
`hr_css_variables` override assertions described above. No Level 3 coverage is
required for this feature because the spec does not assert OS keyboard or mouse
input behavior.
