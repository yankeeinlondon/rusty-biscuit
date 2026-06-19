---
ready: true
agent: codex
model: ""
---

# Review: Browser Tree-Renderer Performance

## Findings

### High: Perf gate is not satisfied until the three remaining browser breaches are signed off

The implementation materially improves the browser tree renderer and records a clean quiescent capture, but the feature still does not meet the spec's production-readiness bar. The spec defines the browser gate as geomean of tree / legacy <= 1.0x, no fixture > 1.5x, or a documented, signed-off exception for each breach (`spec.md:95-102`, `spec.md:357-360`, `spec.md:380-382`). The recorded post-fix baseline still has full-corpus geomean 1.58x and three fixtures above the 1.5x ceiling: `small_prose` 9.74x, `deeply_nested_lists` 3.70x, and `mark_dim_hr` 2.08x (`baselines.md:570-579`, `baselines.md:584-602`). The exception table explicitly says these are only proposed and pending cutover-owner sign-off (`baselines.md:611-622`).

That means the structural work may be done, but this feature cannot be marked production-ready yet. Either get explicit owner sign-off and update `baselines.md` / the tree-cutover spec from "proposed/pending" to accepted, or keep iterating until the full browser corpus geomean is <= 1.0x and every fixture is <= 1.5x without exceptions.

Verification level: performance requirement; verified by Criterion benchmark receipts, not L1/L2/L3 terminal behavior. The strongest current verification is an in-repo quiescent benchmark capture. The missing piece is approval state, not another test tier.

## Coverage Notes

- `migration_parity` now measures the browser tree side as a final `String` via `render_browser_document_html(...).output`, matching the corrected benchmark requirement.
- The direct string renderer has L1 byte-parity tests against `render_browser_document(...).output.render()` for the shared corpus, page options, raw HTML policy, code-hook rollup, and Mermaid modes.
- Browser-tier tests exist for real Chromium DOM/computed-style behavior around styled HR SVG and Mermaid SVG, which is appropriate for the user-observable browser rendering requirements in this area.
- I ran `cargo test -p renderable document_html --lib --color=never`; 7 tests passed.

## Recommendation

Do not ship this as production-ready until the exception status is closed. Once the three fidelity exceptions are explicitly accepted by the cutover owner, this review's blocking finding can be cleared without requiring code changes.

## Resolution (2026-06-03)

The blocking finding is **cleared**. The cutover owner (Ken Snyder) reviewed each
breach one by one against its performance characteristics and **signed off on all
three as accepted added-fidelity exceptions**:

| Fixture | Ratio | Byte ratio | Time/byte | Decision |
|---|---|---|---|---|
| `small_prose` | 9.74× | 6.42× | 1.52× | **Signed off** — small-denominator artifact (≈ +32 µs on a ~1.2 KB body); amortizes to ≤ 1.0× as content grows (`large_prose` 1.19×). |
| `deeply_nested_lists` | 3.70× | 3.06× | 1.21× | **Signed off** — pure added markup (list/item classes, `data-*`); time tracks output volume. |
| `mark_dim_hr` | 2.08× | 1.43× | 1.45× | **Signed off** — genuinely new fidelity (graphics-policy styled-HR `<svg>` + `<mark>` recovery legacy never emitted). |

No code changes were required, consistent with this review's note. The exception
table in `baselines.md` is updated from proposed/pending to accepted, and the
tree-cutover spec records the browser perf gate as cleared. With the five
non-exception fixtures at geomean **0.88× ≤ 1.0×** and structural overhead
eliminated, the browser perf gate is satisfied.
