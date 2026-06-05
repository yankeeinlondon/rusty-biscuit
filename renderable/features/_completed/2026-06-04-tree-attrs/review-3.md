---
ready: true
agent: codex
model: ""
---

# Review 3 — Tree Attrs

## Findings

No blocking findings.

Iteration 3 closes the remaining issues from review 2:

- The structural perf-gate corpus now exercises progress hints, columns hints, table terminal hints, table-cell hints, code hints, list/task hints, typed layout/style, nested inline style inheritance, and an extension namespace.
- The perf gate folds the corpus through Markdown, browser fragment, browser streaming, and terminal renderers, and asserts zero renderable-owned `data`-bag accesses.
- Public `HintNamespace` / `set_hint` / `get_hint` / `remove_hint` docs now use custom extension namespaces and warn that `renderable.*` keys are compatibility/testing-only and rejected by validation.

## Verification Level Mapping

- Typed `NodeAttrs` storage, sparse serde shape, stale `renderable.*` validation, accessor round-trips, and component-hint placement: Level 1 unit/doctest coverage. This is the appropriate level because the behavior is in-process IR/API behavior.
- Perf-gate invariant: Level 1 structural tests. This is the appropriate level because the requirement is not terminal-emulator rendering or keyboard input; it is an internal guarantee that first-class attrs do not route through the JSON bag during folds.
- Terminal-renderer participation in the perf gate: Level 1 crate integration test in `biscuit-terminal/lib/tests/perf_gate.rs`. No Level 2 or Level 3 test is required for this storage/perf invariant.

## Verification Performed

- `cargo metadata --no-deps --format-version 1`
- `cargo test -p renderable --color=never`
  - 405 unit tests passed.
  - 22 integration tests passed.
  - 81 doctests passed, 2 ignored.
- `cargo test -p biscuit-terminal --test perf_gate --color=never`
  - 2 tests passed.

## Notes

- The requested `root` skill was not available in this session's skill catalog, so this review used the repo instructions plus the `renderable` and `rust-testing` skills.
- The requested path used `renderable/root/features/...`; the actual feature directory in this worktree is `renderable/features/2026-06-04-tree-attrs/`.

## Production Readiness

Ready for production. The implementation satisfies the reviewed acceptance criteria, and the remaining verification is at the right level for the behavior under review.
