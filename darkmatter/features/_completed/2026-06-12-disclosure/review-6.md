---
ready: false
agent: codex
model: ""
---

# Review 6

## Findings

### High: Inline `width` does not override frontmatter `max-width`

The spec requires disclosure layout precedence to be `instance-level param=value` > `style.disclosure` frontmatter > broadcast/default (`spec.md:329`, `spec.md:342`). The implementation starts from the frontmatter `ComponentPolicy` and overlays inline fields one by one in `apply_disclosure_policy` (`darkmatter/lib/src/markdown/render_tree/build_context.rs:273-287`). When an instance specifies `width=60ch` and frontmatter specifies `style.disclosure.max-width: 24ch`, the inline width is copied into `layout.width`, but the lower-priority `layout.max_width` remains in place.

That stale cap is user-visible in both terminal and browser rendering:

- Terminal rendering resolves `layout.width` first and then applies `layout.max_width` as a cap (`biscuit-terminal/lib/src/render_tree/render.rs:401-417`), so the instance `width=60ch` still renders at 24 columns.
- Browser rendering emits both `width:60ch` and `max-width:24ch` (`renderable/src/tree/render/browser.rs:2785-2795`), so CSS clamps the instance width to the lower-priority frontmatter cap.

Requirement classification:

- Instance-level disclosure layout precedence: strongest present coverage is Level 1 for inline `max-width` alone and frontmatter `max-width` alone; no test covers cross-property precedence between inline `width` and frontmatter `max-width`.
- Terminal visible layout precedence: strongest present coverage is Level 2 for inline `max-width` and frontmatter `max-width` separately; no Level 2 test covers the conflict/override case.
- Browser visible layout precedence: strongest present coverage is string/DOM coverage for disclosure behavior, but no browser-tier computed-width assertion covers this precedence case.

Fix by treating `width` and `max-width` as a mutually exclusive layout choice across precedence layers, not just within a single bucket. If inline `width` is present, clear inherited `max_width`; if inline `max-width` is present, reset inherited fixed `width` back to `Auto` before applying the cap. Add a focused Level 1 policy test asserting the resulting node layout, plus either a Level 2 terminal width assertion or browser-tier computed-width assertion for the user-visible behavior.

## Notes

The prior review's four verification-level findings appear resolved:

- Browser disclosure click behavior now has browser-tier Chrome coverage in `darkmatter/lib/tests/browser_render.rs`.
- Frontmatter `style.disclosure.*` terminal behavior now has Level 2 coverage in `darkmatter/cli/tests/level2_layout.rs`.
- Disclosure `width` + `max-width` bucket conflict now has direct Level 1 coverage.
- Malformed disclosure page-render error mapping now has direct Level 1 coverage.

I did not find gaps in compose invariance, Markdown/MarkdownPlus/HTML/JSON target lowering, nested disclosure rendering, output aliases, strict-style parsing, or transclusion unification.

Verification run during this review:

- `cargo test --color=never -p darkmatter disclosure --tests` passed.
- `cargo test --color=never -p darkmatter-cli markdown_plus --test cli` passed.

I did not run `just test-l2`; the review inspected the Level 2 tests that exist, and the local targeted disclosure suite already exercised the browser-tier tests that matched the `disclosure` filter.
