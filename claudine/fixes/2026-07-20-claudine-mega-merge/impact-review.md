# High/Critical Impact Review Queue

| Target | Revision | Risk | Blast radius | Proposed resolution owner | Status | Required before edit |
|---|---|---|---|---|---|---|
| `File:claudine/lib/src/composition/types.rs` | proxy `e348486c810969abe87a6b7209979034f5454b07` | HIGH | 16 direct; 25 through depth 3; no indexed process membership | Shared typed composition contracts (Clusters 1, 3, and 4) | open | Inspect every direct importer, preserve one canonical request/prepared-document contract, and record the conflict rationale before Phase 3 edits |
| `run_harness_loop_inner` | execution seed `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb` | HIGH | 1 direct; 5 through depth 3; 1 affected process across Composition, Wrap, and Lifecycle | Command coordinator / harness orchestration | resolved for Phase 2 | Reviewed `run_harness_loop`, `run_composition_body`, the composition pipeline, and lifecycle-control dispatch on the seed and foundation tip. The foundation retains one coordinator and the prepare → execute → classify ordering; its changes thread task output/file context and preserve typed errors instead of changing transition ownership. Bound by harness loop-control tests, `characterization_error_routes`, `effective_diagnostic_render`, and the focused diagnostic suites. |
| `execute_loop_with_lifecycle` | execution seed `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb` | HIGH | 3 direct; 15 through depth 2 across Tests, Looping, and Compose | Active-document loop / lifecycle protocol | resolved for Phase 2 | Reviewed both test helpers and `run_loop_with_overrides` on the seed and foundation tip. The foundation preserves initialize-once, terminal/finalize, gate, fail-fast, cap, and reset ordering while threading the request-scoped `FileResolutionContext` through proxy/expression/action lookup. Bound by `looping::engine::tests::lifecycle_control`, file-resolution parity tests, and Sequence JIT/output tests. |

Evidence: [`impact/proxy/file-lib-src-composition-types-rs.md`](./impact/proxy/file-lib-src-composition-types-rs.md).

No CRITICAL result was returned. No other hotspot was HIGH. The
foundation view of the same file was MEDIUM (12 direct, 15 total); both branch
views must be considered when the row is reviewed.

The remaining open HIGH row is proxy-tip-only and is intentionally deferred to
Phase 3; Phase 2 does not integrate or resolve that conflict surface.
