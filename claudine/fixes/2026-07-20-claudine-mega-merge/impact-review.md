# High/Critical Impact Review Queue

| Target | Revision | Risk | Blast radius | Proposed resolution owner | Status | Required before edit |
|---|---|---|---|---|---|---|
| `File:claudine/lib/src/composition/types.rs` | proxy `e348486c810969abe87a6b7209979034f5454b07` | HIGH | 16 direct; 25 through depth 3; no indexed process membership | Shared typed composition contracts (Clusters 1, 3, and 4) | open | Inspect every direct importer, preserve one canonical request/prepared-document contract, and record the conflict rationale before Phase 3 edits |
| `run_harness_loop_inner` | execution seed `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb` | HIGH | 1 direct; 5 through depth 3; 1 affected process across Composition, Wrap, and Lifecycle | Command coordinator / harness orchestration | open | Review the full composition-body flow and all three affected modules before Phase 2 edits |
| `execute_loop_with_lifecycle` | execution seed `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb` | HIGH | 3 direct; 15 through depth 2 across Tests, Looping, and Compose | Active-document loop / lifecycle protocol | open | Preserve loop/lifecycle invariants and review every direct caller before Phase 2 edits |

Evidence: [`impact/proxy/file-lib-src-composition-types-rs.md`](./impact/proxy/file-lib-src-composition-types-rs.md).

No CRITICAL result was returned. No other hotspot was HIGH. The
foundation view of the same file was MEDIUM (12 direct, 15 total); both branch
views must be considered when the row is reviewed.
