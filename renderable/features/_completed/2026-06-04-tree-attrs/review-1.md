---
ready: false
agent: codex
model: ""
---

# Review 1 — Tree Attrs

## Findings

### High — The deterministic perf gate does not cover the required targets or allocation invariant

Spec AC5 requires the gate to build the styled corpus, fold it once per target, and assert both zero renderable-owned hint round-trips and zero renderable-owned per-node key allocations while still allowing extension hints. The implementation only asserts a Markdown fold in `renderable`:

- `renderable/src/tree/attrs.rs:2198` documents the gate as Markdown-only.
- `renderable/src/tree/attrs.rs:2207` through `renderable/src/tree/attrs.rs:2213` calls only `render_markdown_document`.
- `renderable/src/tree/attrs.rs:761` through `renderable/src/tree/attrs.rs:796` counts `set_hint` / `get_hint` / `remove_hint` calls by namespace, but does not separately count key allocations.

I found no terminal or browser perf-gate tests (`rg "zero renderable|HINT_ACCESSES|renderable_owned"` only finds `renderable/src/tree/attrs.rs`). Because the counter is behind `#[cfg(test)]` inside `renderable`, downstream `biscuit-terminal` tests would not see it unless `renderable` is compiled in a test-instrumented mode for that dependency. This leaves the terminal fold, browser fragment fold, and browser streaming fold unguarded against reintroducing renderable-owned bag access or key allocation.

Verification level: Level 1 only, and incomplete for this internal structural invariant. No L2/L3 is required because this is not terminal-emulator behavior, but the L1 gate needs all specified folds.

### High — Renderer hot paths still use owned accessors and clone typed attributes

The spec allows owned compatibility accessors to remain, but explicitly requires borrowed accessors to be added and renderers switched to borrowed forms so the folds pay neither serde nor unnecessary typed clones. Borrowed `layout_ref` / `style_ref` exist, but hot paths still call cloning accessors:

- Terminal: `biscuit-terminal/lib/src/render_tree/render.rs:174`, `:191`, `:371`, `:387`, `:389`, `:775`, `:813`, `:1118`, `:1514`, `:1530`, `:1566`, `:1614`, `:2069`.
- Browser: `renderable/src/tree/render/browser.rs:354`, `:361`, `:1004`, `:1286`, `:1329`, `:1337`, `:1790`.
- Markdown: `renderable/src/tree/render/markdown.rs:218`, `:227`, `:265`.

This preserves the biggest storage win but misses the performance contract in AC2. Several of these clone `Style`, `Layout`, table hints, progress hints, column hints, or titles on every fold visit.

Verification level: Level 1 API/perf structural coverage is missing; no L2/L3 required.

### High — `NodeAttrs` is not fully sparse/defaulted on the serde wire shape

AC3 requires sparse serialization with `#[serde(default)]` and `skip_serializing_if`, and the design sample shows default/skip behavior for `classes`, typed fields, and `data`. The current struct only annotates the newly-added typed fields:

- `renderable/src/tree/attrs.rs:737` `id` has no `skip_serializing_if`.
- `renderable/src/tree/attrs.rs:739` `classes` has no `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
- `renderable/src/tree/attrs.rs:758` `data` has no `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`.

That means `NodeAttrs::default()` still serializes identity/class/data noise, and sparse typed payloads that omit these fields are not accepted as defaulted `NodeAttrs`. The added test at `renderable/src/tree/attrs.rs:1477` only checks that new field names are absent; it does not assert the complete sparse shape or deserialization from a sparse typed payload.

Verification level: Level 1 serde tests are present but too weak for the AC3 contract.

### Medium — Validation does not enforce most typed `ComponentHints` kind-placement invariants

The typed `ComponentHints` docs state that variants are matched to `NodeKind` (`renderable/src/tree/attrs.rs:686` through `:690`), and AC8 requires validation to read typed fields directly. Validation currently checks sequence join, list marker policy, task hints, and table title:

- `renderable/src/tree/validate.rs:318` through `:354`.

It does not reject `ComponentHints::List` on a paragraph, `Code` on a list, `Progress` on a table, `Columns` on a non-blockquote, `Table` column/terminal hints on non-table nodes, or `TableCell` hints on non-cell nodes. The test search likewise only shows placement tests for sequence/list-marker/task/title. That leaves malformed typed IR accepted even though the old stringly accessors have been replaced by a discriminated component hint field.

Verification level: Level 1 validation coverage is partial.

## Notes

- The requested `root` skill was not available in this session's skill catalog, so this review used the repo instructions, `renderable`, and `rust-testing`.
- The requested path used `renderable/root/features/...`, but the actual feature directory in this worktree is `renderable/features/2026-06-04-tree-attrs/`; this review is saved there.

## Verification Performed

- `cargo test -p renderable` passed: 392 unit tests, 22 integration tests, and 81 passing doctests with 2 ignored doctests.
- I did not run L2/L3 terminal tests. This feature's reviewed requirements are internal IR/serde/perf-gate behavior; real-terminal or OS-keyboard injection is not the appropriate minimum for these findings.

## Production Readiness

Not ready for production. The core typed storage is present, and the renderable package test suite is green, but the implementation does not yet satisfy the perf-gate, borrowed-hot-path, sparse-serde, or full typed-validation acceptance criteria.
