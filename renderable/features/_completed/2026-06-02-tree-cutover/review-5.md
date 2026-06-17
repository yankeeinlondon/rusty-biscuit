---
ready: false
agent: codex
model: ""
---

# Review 5 — Tree Cutover

## Verdict

Not ready for production.

Iteration 5 fixes the review-4 `DarkmatterPage` / `HtmlOptions` / `TerminalOptions`
HR wiring blocker: page-projected `style.hr.*` defaults now reach the tree
HTML and terminal entry points, `hr_css_variables` is emitted through the page
CSS-variable channel, and the terminal tree renderer accepts both `center` and
`centered` HR alignment spellings.

The remaining blocker is a narrower direct-API regression: the old public
`Markdown::as_html(HtmlOptions::default())` and
`Markdown::as_terminal(TerminalOptions::default())` paths honored deprecated
top-level `hr:` frontmatter defaults. The new tree-backed paths only apply
defaults already supplied in the options, so direct callers lose that
compatibility unless they route through `DarkmatterPage`.

## Findings

### High — Direct `Markdown::as_html` / `as_terminal` no longer apply top-level `hr:` frontmatter defaults

The spec requires no functional or fidelity regressions versus the bespoke
renderers, and review 4 explicitly called out deprecated top-level `hr:`
compatibility for the direct `Markdown::as_html` / `as_terminal` paths.

The deleted bespoke serializers derived an option fallback from the markdown
frontmatter when no explicit `hr_defaults` option was supplied:

- `darkmatter/lib/src/markdown/output/html.rs` at the pre-cutover baseline used
  `hr_defaults_from_frontmatter(md)` and then
  `options.hr_defaults.as_ref().or(hr_fallback.as_ref())` before rendering both
  attributed and bare rules.
- `darkmatter/lib/src/markdown/output/terminal.rs` did the same before writing
  terminal horizontal rules.

The current tree-backed direct paths do not do that fallback:

- `darkmatter/lib/src/markdown/mod.rs:604`-`611` folds the document, validates
  code directives, and passes the original `HtmlOptions` straight to
  `render_tree_html_from_document`.
- `darkmatter/lib/src/markdown/mod.rs:637`-`660` passes the original
  `TerminalOptions` through to `render_tree_terminal`.
- `darkmatter/lib/src/markdown/render_tree/entrypoints.rs:201`-`208`,
  `577`-`584`, and `618`-`625` apply HR defaults only when
  `options.hr_defaults` is already `Some`.

Concrete regression:

```markdown
---
hr:
  style: waves
  weight: thick
  width: "50%"
---

---
```

`md.as_html(HtmlOptions::default())` should render the bare rule with the
frontmatter defaults, and `md.as_terminal(TerminalOptions { image_mode: Never,
.. })` should likewise render the styled text-tier rule. On the current tree
path, those calls render the default rule because the fallback never populates
`options.hr_defaults`.

The new tests cover `DarkmatterPage`-mediated frontmatter application
(`horizontal_rule_integration.rs:536`-`608`) and explicit option defaults in
the render-tree entrypoint unit tests, but the direct public `Markdown::as_*`
compatibility tests that existed before the cutover were not restored.

Verification level: Level 1 is sufficient to catch the missing direct option
fallback and should be restored for direct HTML and terminal calls, including
bare-rule defaults, partial inline-attribute override, numeric/bool scalar
sibling preservation, and blockquote-contained bare rules. Browser computed
coverage is appropriate once restored for the visible CSS/SVG effect, and
Level 2 terminal coverage is appropriate for the rendered glyph/width behavior.
Current strongest verification for this direct-API requirement is none, so this
remains a production blocker.

**Resolution (2026-06-03).** The bespoke serializer's frontmatter fallback is
restored on the tree path. A `hr_defaults_from_frontmatter` helper (with the
original scalar-coercion / sibling-preservation / warn-on-unknown-key contract)
was reinstated in `render_tree/entrypoints.rs`, and the direct entry points
(`Markdown::as_html`, the standalone `render_tree_html`, `render_tree_terminal`,
and `render_tree_terminal_with_layout`) now seed bare-rule defaults from the
deprecated top-level `hr:` block whenever `options.hr_defaults` is unset. The
`is_none()` guard preserves the legacy `.or()` precedence, so an explicit option
(including a `DarkmatterPage` `style.hr.*` projection) still wins outright and
the page path is unchanged. Level 1 coverage was restored: direct `Markdown::as_*`
integration tests for bare-rule defaults, partial inline-attribute override,
numeric/bool scalar sibling preservation, and blockquote-contained bare rules
(`horizontal_rule_integration.rs`), plus entrypoint unit tests for the coercion
contract, the fallback wiring, and explicit-option precedence.

## Requirement Status

| Requirement | Status | Strongest relevant verification |
|---|---:|---|
| `Markdown::as_html` routes through the tree | Implemented | Level 1 direct API tests plus browser computed-style coverage for selected browser behavior |
| `Markdown::as_terminal` and decorated `DarkmatterPage::render` route through the tree | Implemented | Level 1 entrypoint/page tests plus Level 2 WezTerm captures for selected terminal behavior |
| Delete bespoke darkmatter serializers / `RuleProcessor` | Implemented | Level 1 mechanical/API inspection |
| Page-projected `style.hr.*` defaults for `DarkmatterPage` | Implemented | Level 1 integration tests; browser/Level 2 coverage still useful for visual fidelity |
| `HtmlOptions::hr_css_variables` override contract | Implemented | Level 1 entrypoint tests; browser computed coverage still useful for CSS application |
| Direct deprecated top-level `hr:` defaults for `Markdown::as_html` / `as_terminal` | Implemented | Level 1 direct-API integration tests (`horizontal_rule_integration.rs` `direct_as_*`) plus entrypoint unit tests for the `hr_defaults_from_frontmatter` coercion contract and fallback wiring |

## Notes

I ran:

```bash
cargo test -p darkmatter --test horizontal_rule_integration -- --nocapture
cargo test -p darkmatter --lib markdown::render_tree::entrypoints::tests:: -- --nocapture
cargo test -p biscuit-terminal horizontal_rule_from_attrs_accepts_center_and_centered -- --nocapture
```

All three passed. No Level 3 coverage is required for this feature because the
spec does not assert OS keyboard or mouse input behavior.
