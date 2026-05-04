# Follow-Up Review

Reviewed on 2026-03-29 against the current `biscuit-terminal` and `biscuit-visualized` code, using [spec.md](./spec.md) for scope and the earlier `data-visualization-review.md` as the checklist.

## Validation

- Ran `just test` in `biscuit-terminal`: passed.
- Confirmed `state-diagram --json --title ...` now injects Mermaid frontmatter title.
- Confirmed `erd --json --title ...` now injects Mermaid frontmatter title.
- Confirmed the CLI test suite now includes coverage for:
  - `bt graph-expression --meta`
  - mixed `->` / `--` rejection
  - state-diagram title wiring
  - ERD title wiring

## Conclusion

I did not find any of the previously reported shipped behavior bugs still open.

The major functional findings from the prior review appear fixed:

1. `bt graph-expression --meta` is now implemented.
   - `display_graph(...)` emits the same `RenderMeta` shape used by Mermaid: `biscuit-terminal/cli/src/commands.rs:356-405`
   - `render_graph_expression(...)` now routes through that helper: `biscuit-terminal/cli/src/commands.rs:1447-1522`
   - CLI regression test exists: `biscuit-terminal/cli/tests/integration_test.rs:1319-1338`

2. `--title` for `state-diagram` and `erd` is now carried into rendered Mermaid instructions.
   - Shared title injection helper: `biscuit-terminal/cli/src/commands.rs:290-295`
   - `state-diagram` uses it before rendering: `biscuit-terminal/cli/src/commands.rs:1406-1429`
   - `erd` uses it before rendering: `biscuit-terminal/cli/src/commands.rs:1585-1622`
   - CLI regression tests exist:
     - `biscuit-terminal/cli/tests/integration_test.rs:1066-1085`
     - `biscuit-terminal/cli/tests/integration_test.rs:1156-1175`

3. Graph inverse rendering is no longer the old white-box fallback.
   - Terminal-aware graph theming is now explicit in `GraphExpression::for_terminal_mode(...)`: `biscuit-terminal/lib/src/components/graph_expression.rs:279-312`
   - Inverse mode selects the opposite theme and disables transparency: `biscuit-terminal/lib/src/components/graph_expression.rs:132-145`
   - Graph rendering now has explicit dark/light surface colors and uses the theme surface for opaque output:
     - `biscuit-visualized/src/src/graph/render.rs:62-105`
     - `biscuit-visualized/src/src/graph/render.rs:471-497`

4. Mixed directed and undirected graph-expression syntax is now rejected instead of silently coerced.
   - Parser rejection: `biscuit-visualized/src/src/graph/expression.rs:94-98`
   - CLI regression test: `biscuit-terminal/cli/tests/integration_test.rs:1341-1349`

5. The documentation drift called out in the prior review appears addressed.
   - Mermaid component docs now describe `png_path` and `cache_hit`, not the removed `render_time_ms`: `biscuit-terminal/docs/components/mermaid_diagram.md:19-25`
   - Graph docs now use the spelled-out orientation values that the CLI accepts: `biscuit-terminal/docs/components/graph_expression.md:56-60`
   - Darkmatter terminal Mermaid docs now describe the pure-Rust `biscuit-visualized` path instead of the old `mmdc` story: `darkmatter/lib/src/mermaid/render_terminal.rs:3-6`

6. The font-database performance suggestion also appears resolved.
   - `biscuit-visualized` now caches the system font database with `OnceLock`: `biscuit-visualized/src/src/raster/mod.rs:4-16`

## Remaining Recommendations

These are the only follow-ups I would still carry forward, and both are lower-priority maintainability/testing items rather than open shipped bugs.

### 1. Shared CLI display abstraction is still duplicated

The earlier recommendation to unify graph and Mermaid terminal display is still reasonable, but it is no longer blocking correctness.

- `display_mermaid(...)` and `display_graph(...)` still exist as parallel helpers with largely duplicated render/meta/layout flow:
  - `biscuit-terminal/cli/src/commands.rs:300-352`
  - `biscuit-terminal/cli/src/commands.rs:356-410`

Recommendation:

- Keep this as a cleanup task, not as an urgent bug fix.
- If touched again, extract a shared render-result display path so future metadata/layout changes land once.

### 2. Inverse graph mode still lacks explicit regression coverage

The implementation now looks correct, but I did not find a focused test that proves inverse graph rendering keeps the intended opaque opposite-theme behavior.

- Implementation exists in:
  - `biscuit-terminal/lib/src/components/graph_expression.rs:132-145`
  - `biscuit-terminal/lib/src/components/graph_expression.rs:279-312`
  - `biscuit-visualized/src/src/graph/render.rs:471-497`
- Existing graph tests currently cover title/fallback/meta/mixed-edge cases, but not inverse-theme assertions:
  - `biscuit-terminal/lib/src/components/graph_expression.rs:363-407`
  - `biscuit-terminal/cli/tests/integration_test.rs:1259-1349`

Recommendation:

- Add one unit test at the adapter/backend level that asserts inverse renders use an opaque surface color from `GraphColorTheme`, plus one CLI-facing contract test if you want end-to-end protection.

## Final Recommendation

Treat this review item as substantially complete. The user-facing bugs from the previous review are fixed. What remains is cleanup: unify the duplicated display path when convenient, and add an inverse-theme regression test so the newer graph theming behavior stays locked in.
