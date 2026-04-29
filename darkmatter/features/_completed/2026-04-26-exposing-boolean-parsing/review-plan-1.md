---
phases: 5
created: 2026-04-28
source: darkmatter/features/2026-04-26-exposing-boolean-parsing/review-1.md
scope: darkmatter package area
---

# Review Response Plan: Exposing Boolean Parsing

This plan covers every recommendation from `review-1.md`:

- remove duplicate condition tests from the transclusion adapter
- rewrite the boolean conditional logic documentation introduction
- avoid the per-capture `HashSet` allocation in the shortcut lookup path
- add ternary lazy-loading regression coverage
- verify the darkmatter package area has passing tests and clean lints

The review uses `translocation/conditions.rs` once, but the affected file is `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs`.

## Phase 1: Preflight And Current-State Audit

**Scope**

Confirm the implementation state before editing. Some review items may already be partially or fully applied in the working tree; if so, verify and record that instead of reintroducing churn.

**Files to inspect**

- `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs`
- `darkmatter/lib/src/markdown/compose/conditions.rs`
- `darkmatter/lib/src/markdown/compose/context/capture.rs`
- `darkmatter/docs/topics/boolean-conditional-logic.md`

**Commands**

```bash
cargo metadata --no-deps --format-version 1
rg -n "cfg\\(test\\)|mod tests|HashSet::new|capture_runtime_context_for_groups|shortcut_ternary_short_circuits|Darkmatter relies|beyond directives|uses a shared condition evaluator" \
  darkmatter/lib/src/markdown/compose/transclusion/conditions.rs \
  darkmatter/lib/src/markdown/compose/conditions.rs \
  darkmatter/lib/src/markdown/compose/context/capture.rs \
  darkmatter/docs/topics/boolean-conditional-logic.md
```

**Done Criteria**

- The darkmatter workspace package is confirmed via `cargo metadata`.
- The implementer knows which of the four review fixes still need code or docs edits.
- No unrelated files are changed.

## Phase 2: Remove Duplicate Transclusion Condition Tests

**Scope**

`transclusion/conditions.rs` should remain a thin adapter over `compose::conditions`; duplicated evaluator unit tests belong in `compose/conditions.rs` and `compose/expression/*`.

**Implementation**

1. In `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs`, delete any `#[cfg(test)] mod tests` block that duplicates core condition behavior.
2. Keep the public adapter function and `From<ConditionError> for TransclusionError` mapping intact.
3. Do not delete transclusion integration coverage elsewhere. The adapter is still exercised by transclusion tests that use `when="..."`.

**Tests To Run**

```bash
cargo test -p darkmatter conditions
cargo test -p darkmatter transclusion
```

**Lint Command**

```bash
cargo clippy -p darkmatter --lib -- -D warnings
```

**Done Criteria**

- `transclusion/conditions.rs` contains only adapter logic and no duplicate core evaluator tests.
- Core condition tests still pass.
- Transclusion tests still pass, proving `when="..."` behavior remains covered through the integration path.
- No new clippy warnings.

## Phase 3: Fix Boolean Conditional Logic Documentation

**Scope**

Clean up the opening of `darkmatter/docs/topics/boolean-conditional-logic.md` so the public docs read clearly and reflect the shared evaluator architecture.

**Implementation**

1. Replace any fragmented introduction with this structure:
   - a one-sentence description of boolean expressions conditionally including or excluding content
   - a short list of surfaces: page blocks, transclusion directives, reference-graph conditional extraction
   - a statement that all surfaces share syntax, truthiness rules, helper functions, and the evaluator
2. Preserve the existing sections on expression engine, value resolution, truthiness, operators, and shortcut API.
3. Keep links relative and existing-doc compatible.

**Tests To Run**

```bash
cargo test -p darkmatter --doc
```

**Lint Command**

```bash
cargo clippy -p darkmatter --all-targets -- -D warnings
```

**Done Criteria**

- The first 15-20 lines of the topic doc contain no sentence fragments.
- Public docs mention the shared evaluator accurately.
- Doctests and clippy pass.

## Phase 4: Optimize Shortcut Context Capture And Add Ternary Lazy Tests

**Scope**

Address the performance recommendation and the missing regression coverage in `darkmatter/lib/src/markdown/compose/conditions.rs`, with any needed helper signature adjustment in `darkmatter/lib/src/markdown/compose/context/capture.rs`.

**Implementation**

1. Inspect `capture_runtime_context_for_groups` in `context/capture.rs`.
2. Prefer the zero-extra-allocation shape:

```rust
pub(crate) fn capture_runtime_context_for_groups(
    base_dir: &Path,
    groups: &[ContextGroup],
) -> CaptureResult
```

3. Update `ShortcutLookup::capture_group` to pass a single-element slice:

```rust
let (values, _diagnostics, _timings) =
    super::context::capture::capture_runtime_context_for_groups(self.work_dir, &[group]);
```

4. If the slice-based signature is already present, make no additional code change for this item.
5. Keep `captured_groups: RefCell<HashSet<ContextGroup>>`; that set is still useful as the per-evaluation cache guard and is not the allocation called out by the review.
6. Add or verify these two tests in the `#[cfg(test)] mod tests` in `conditions.rs`:
   - `shortcut_ternary_short_circuits_then_branch`: `false_flag ? ctx.repo : 'default'` must not capture `ContextGroup::Repo`.
   - `shortcut_ternary_short_circuits_else_branch`: `true_flag ? 'default' : ctx.repo` must not capture `ContextGroup::Repo`.
7. Implement those tests through `ShortcutLookup`, `expression::parse_condition`, and `expression::evaluate` so they can inspect `lookup.captured_groups()` directly. Assert both returned branch value and captured group state.

**Tests To Run**

```bash
cargo test -p darkmatter shortcut_ternary
cargo test -p darkmatter shortcut_ternary_short_circuits
cargo test -p darkmatter shortcut_and_short_circuits_prevents_ctx_capture
cargo test -p darkmatter shortcut_or_short_circuits_prevents_ctx_capture
cargo test -p darkmatter expression
cargo test -p darkmatter conditions
```

**Lint Command**

```bash
cargo clippy -p darkmatter --all-targets -- -D warnings
```

**Done Criteria**

- `ShortcutLookup::capture_group` no longer builds a new `HashSet` for each capture.
- Existing lazy capture semantics for `&&`, `||`, unknown `ctx.*` keys, and repeated same-group lookups still pass.
- Ternary tests prove the unevaluated branch does not trigger context capture.
- Clippy has no warnings.

## Phase 5: Final Darkmatter Area Verification

**Scope**

Run the full verification expected before marking the review ready. The review focused on the library, but the darkmatter package area includes both library and CLI packages, so lint both.

**Tests To Run**

```bash
cargo test -p darkmatter conditions
cargo test -p darkmatter interpolation
cargo test -p darkmatter expression
cargo test -p darkmatter --lib
cargo test -p darkmatter --doc
cargo test -p darkmatter-cli
```

**Lint And Format Commands**

```bash
cargo fmt --check
cargo clippy -p darkmatter -p darkmatter-cli --all-targets -- -D warnings
```

Alternative area-level commands, if the developer prefers the local justfile:

```bash
just -f darkmatter/justfile test
just -f darkmatter/justfile lint
```

**Done Criteria**

- All review recommendations are either implemented or confirmed already present.
- All targeted and full darkmatter library tests pass.
- Darkmatter CLI tests pass.
- `cargo fmt --check` passes.
- `cargo clippy -p darkmatter -p darkmatter-cli --all-targets -- -D warnings` passes with no warnings or errors.
- The final review response can state that `review-1.md` is addressed and production readiness is no longer blocked by the listed cleanup tasks.

## Expected Changed Files

- `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs`
- `darkmatter/docs/topics/boolean-conditional-logic.md`
- `darkmatter/lib/src/markdown/compose/conditions.rs`
- `darkmatter/lib/src/markdown/compose/context/capture.rs` only if the slice-based capture helper is not already present

No dependency changes are expected. No README or skill updates are required unless implementation reveals public behavior different from the spec.
