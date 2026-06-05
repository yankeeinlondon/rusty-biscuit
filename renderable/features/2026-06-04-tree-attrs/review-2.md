---
ready: false
agent: codex
model: ""
---

# Review 2 — Tree Attrs

## Findings

### High — The perf-gate corpus still does not exercise several first-class hint branches

The implementation now folds Markdown, browser fragment, browser streaming, and terminal targets, and the terminal crate can observe renderable's test counter. That closes a large part of review 1. The remaining gap is corpus coverage: the structural gate still does not walk all branches that read first-class typed hints.

Spec AC5 requires a representative corpus with block + inline + table + list + code + nested inheritance + extension hints, and the design says the gate should prevent any first-class attr from being routed back through `data` during a fold (`renderable/features/2026-06-04-tree-attrs/spec.md:221`). The current renderable gate builds only a heading, a list/task item, a table with column/title hints, and a code block (`renderable/src/tree/attrs.rs:2253`). The terminal gate repeats the same shape (`biscuit-terminal/lib/tests/perf_gate.rs:26`).

That leaves these active renderer branches outside the structural invariant:

- progress hints in Markdown/browser/terminal (`renderable/src/tree/render/markdown.rs:218`, `renderable/src/tree/render/browser.rs:354`, `biscuit-terminal/lib/src/render_tree/render.rs:371`)
- columns hints in Markdown/browser/terminal (`renderable/src/tree/render/markdown.rs:227`, `renderable/src/tree/render/browser.rs:361`, `biscuit-terminal/lib/src/render_tree/render.rs:387`)
- table terminal hints in the terminal fold (`biscuit-terminal/lib/src/render_tree/render.rs:1541`)
- table-cell hints in the terminal table path (`biscuit-terminal/lib/src/render_tree/render.rs:2083`)
- an inline styled span / nested style inheritance case in the gate corpus

The gate is Level 1, which is the right verification level for this internal structural invariant. It is not rigorous enough yet because branch coverage is incomplete. No Level 2 or Level 3 terminal test is required for this specific perf invariant.

### Medium — Public `set_hint` docs still show first-class renderable-owned namespaces

`NodeAttrs::data` is now extension-only, and validation rejects stale `renderable.*` keys (`renderable/src/tree/validate.rs:1002`). The public docs for `HintNamespace`, `set_hint`, `get_hint`, and `remove_hint` still demonstrate `HintNamespace::LAYOUT` (`renderable/src/tree/attrs.rs:20`, `renderable/src/tree/attrs.rs:824`, `renderable/src/tree/attrs.rs:844`, `renderable/src/tree/attrs.rs:873`), which creates exactly the stale renderable-owned data shape the validator now reports as invalid.

This is not a runtime correctness bug in the folds, but it is an API ergonomics/docs gap. The examples should use a custom extension namespace such as `HintNamespace("myapp.custom")`, and the docs should explicitly say the `renderable.*` namespace constants are compatibility/testing only or otherwise not for new first-class attrs.

Verification level: Level 1 doctests currently prove the stale examples compile, but they verify the wrong usage contract.

## Resolved Since Review 1

- Renderer hot paths now use borrowed attrs/hint accessors in the searched render folds; the previous owned-accessor clone pattern was not found.
- `NodeAttrs` serde sparsity is implemented and tested; `NodeAttrs::default()` serializes to `{}`.
- Typed component hint placement validation now covers list, code, progress, columns, task, table, and table-cell hints, plus stale `renderable.*` data keys.
- The perf counter is enabled for the downstream terminal perf-gate test through the `hint-access-counter` feature, and the terminal gate passes.

## Verification Performed

- `cargo test -p renderable --color=never` passed: 405 unit tests, 22 integration tests, and 81 passing doctests with 2 ignored doctests.
- `cargo test -p biscuit-terminal --test perf_gate --color=never` passed: 2 tests.
- I did not run Level 2 or Level 3 tests. The reviewed requirements are internal IR storage, validation, serde shape, and structural perf gating; Level 1 is the appropriate verification level for these findings.

## Notes

- The requested `root` skill was not available in this session's skill catalog, so this review used the repo instructions, `renderable`, and `rust-testing`.
- The requested path used `renderable/root/features/...`, but the actual feature directory in this worktree is `renderable/features/2026-06-04-tree-attrs/`; this review is saved there.

## Production Readiness

Not ready for production. The implementation is close, but the perf gate is still too narrow to enforce the promised invariant across the first-class hint branches it is meant to protect.
