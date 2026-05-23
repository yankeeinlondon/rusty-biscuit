# Stage 3 High-Confidence Plan

## Scope

This plan implements `stage3-spec.md` as a structural-projection completion
pass for the twelve `biscuit-terminal` components already migrated toward the
render-tree IR.

Primary success criteria:

- Nested IR-aware components project as structural `RenderNode`s instead of
  ANSI-stripped text.
- `FileSystem::render` has an explicit, tested, documented terminal-path
  decision.
- The fallback path for missing component projections becomes observable.
- Public `render_bespoke()` compatibility hooks are either retired or retained
  only for sanctioned terminal-only escape hatches.
- The migration recipe is documented for future components.
- Tests, snapshots, clippy, and the pre-existing CLI failure are cleaned up.

## Assumptions

- Work starts from the repository state described by `stage3-spec.md` and the
  current on-disk code. Before coding, run `git status --short` and protect any
  unrelated user changes.
- The authoritative package list remains `cargo metadata --no-deps
  --format-version 1`; do not infer workspace membership from directories.
- `FileSystem::render` is a decision gate, not a foregone flip. Outcome (i),
  (ii), or (iii) is acceptable only when backed by parity evidence and recorded
  in `lessons-learned.md`.
- No new `NodeKind` variants should be introduced during Stage 3.
- `renderable`, `biscuit-terminal`, and `biscuit-terminal-cli` are the core
  verification packages unless a touched file expands the blast radius.

## Preflight

1. Capture the starting point.

   ```sh
   git status --short
   cargo metadata --no-deps --format-version 1 >/tmp/rusty-biscuit-metadata.json
   ```

2. Read the required references.

   - `renderable/features/2026-05-19-pushing-toward-ir/stage3-spec.md`
   - `renderable/features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md`
   - `renderable/docs/tree-rendering.md`
   - `renderable/docs/layout-and-style.md`
   - `renderable/features/2026-05-19-pushing-toward-ir/components/FileSystem-spec.md`

3. Establish the concrete baseline with `rg`.

   ```sh
   rg -n "fn render_tree_node|fn render_bespoke|fn render_via_tree|has_default_border" \
     biscuit-terminal/lib/src/components
   rg -n "TODO\\(stage-3\\)" biscuit-terminal/lib/tests
   rg -n "KNOWN_DRIFT|VIA_RENDER|TREE" biscuit-terminal/lib/tests
   ```

4. Do not reorganize moved feature documents. The current worktree may already
   contain user changes under
   `renderable/features/2026-05-19-pushing-toward-ir/`; Stage 3 should only
   touch files required by this plan.

## Phase 3a: Complete Structural Projection

### 3a.1 Add missing `render_tree_node` overrides

Files:

- `biscuit-terminal/lib/src/components/block_quote.rs`
- `biscuit-terminal/lib/src/components/status_block.rs`
- `biscuit-terminal/lib/src/components/filesystem/mod.rs`
- Existing or new tests under `biscuit-terminal/lib/tests/`

Implementation:

- Add `TerminalRenderable::render_tree_node` overrides for `BlockQuote`,
  `StatusBlock`, and `FileSystem`.
- Each override should delegate to the same canonical projection as
  `TreeRenderable::render_tree`, preferably:

  ```rust
  fn render_tree_node(&self) -> Option<RenderNode> {
      Some(<Self as TreeRenderable>::render_tree(self))
  }
  ```

- Do not add a centralized downcast chain in
  `biscuit-terminal/lib/src/render_tree/projection.rs`.
- Add parity tests for each missing component that serialize or compare
  `TreeRenderable::render_tree` against
  `TerminalRenderable::render_tree_node`.
- For `BlockQuote`, also add the one-line comment near `has_default_border`
  explaining that arbitrary prefixes stay on the internal bespoke escape hatch
  and no public `render_bespoke` hook exists.

Verification:

```sh
cargo test -p biscuit-terminal --test render_tree_component_parity
cargo test -p biscuit-terminal --test status_block_parity
cargo test -p biscuit-terminal filesystem
```

### 3a.2 Audit the existing nine overrides

Components:

- `Compose`
- `OrderedList`
- `Progress`
- `Section`
- `Table`
- `TextBlock`
- `Todo`
- `TwoColumn`
- `UnorderedList`

Implementation:

- Confirm each has a `render_tree_node` override.
- Confirm each override shares one projection with `TreeRenderable::render_tree`
  via a parity test.
- If any component fails the audit, fix it in the same pattern as 3a.1.
- Append a concise audit note to
  `renderable/features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md`.

Verification:

```sh
cargo test -p biscuit-terminal --test ordered_list_parity
cargo test -p biscuit-terminal --test unordered_list_parity
cargo test -p biscuit-terminal --test progress_parity
cargo test -p biscuit-terminal --test section_parity
cargo test -p biscuit-terminal --test table_parity
cargo test -p biscuit-terminal --test text_block_parity
cargo test -p biscuit-terminal --test todo_parity
cargo test -p biscuit-terminal --test two_column_parity
```

### 3a.3 Decide `FileSystem::render`

Files:

- `biscuit-terminal/lib/src/components/filesystem/mod.rs`
- Existing or new `biscuit-terminal/lib/tests/filesystem_parity.rs`
- `renderable/features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md`

Decision procedure:

1. Build parity fixtures from `components/FileSystem-spec.md`:
   connector geometry, gitignore styling, errors and permissions, depth limits,
   highlight precedence, metric annotations, dotfile italic, symlink styling,
   and link behavior.
2. Compare bespoke terminal output against direct tree rendering through
   `render_terminal_node`.
3. Choose and record exactly one outcome:
   - Outcome (i): flip `FileSystem::render` to `render_via_tree`.
   - Outcome (ii): keep bespoke permanently and document why the tree cannot
     express the terminal behavior.
   - Outcome (iii): defer to Stage 4 and name the exact missing capability as
     the Stage 4 acceptance criterion.

Guardrails:

- Do not begin S3-4d work for `FileSystem` until this decision is recorded.
- If outcome (i) is chosen, add `FileSystem` to the layout matrix default-case
  coverage in Phase 3c.
- If outcome (ii) or (iii) is chosen, exclude `FileSystem` from
  `via_render == via_tree_direct` matrix parity and record it as a sanctioned
  terminal-only escape hatch.

Verification:

```sh
cargo test -p biscuit-terminal filesystem
cargo test -p biscuit-terminal --test filesystem_parity
```

## Phase 3a: Tighten Nested-Component Tests

Files:

- `biscuit-terminal/lib/tests/two_column_parity.rs`
- `biscuit-terminal/lib/tests/section_parity.rs`
- `biscuit-terminal/lib/tests/render_tree_component_parity.rs` or
  `biscuit-terminal/lib/tests/block_quote_parity.rs`
- `biscuit-terminal/lib/tests/ordered_list_parity.rs`
- `biscuit-terminal/lib/tests/unordered_list_parity.rs`
- `biscuit-terminal/lib/src/components/compose.rs` tests or a dedicated
  `compose` integration test if the existing coverage is not accessible

Implementation:

- Replace relaxed "text survived somewhere" assertions with structural
  `NodeKind` assertions.
- In `two_column_parity.rs`, tighten
  `render_via_tree_preserves_nested_block_content` to require
  `NodeKind::BlockQuote` in the left column.
- Add or tighten:
  - Section-in-Section
  - BlockQuote-in-Section
  - Table-in-Section
  - Section-in-BlockQuote
  - List-in-BlockQuote
  - Nested block components inside ordered and unordered list items
- Expand Compose nested-component coverage for every valid fixture in the spec:
  `BlockQuote`, `Section`, `Table`, `OrderedList`, `UnorderedList`, `Progress`,
  `StatusBlock`, `TextBlock`, `Todo`, `TwoColumn`, and nested `Compose`.
- Use `matches!` on discriminants and avoid over-pinning renderer-shaped
  details.
- Remove all `TODO(stage-3)` markers from `biscuit-terminal/lib/tests/`.

Verification:

```sh
rg -n "TODO\\(stage-3\\)" biscuit-terminal/lib/tests && false || true
cargo test -p biscuit-terminal --test two_column_parity
cargo test -p biscuit-terminal --test section_parity
cargo test -p biscuit-terminal --test ordered_list_parity
cargo test -p biscuit-terminal --test unordered_list_parity
cargo test -p biscuit-terminal compose
```

## Phase 3b: Make Projection Fallback Observable

Files:

- `biscuit-terminal/lib/src/components/renderable.rs`
- `biscuit-terminal/lib/src/render_tree/projection.rs`
- `biscuit-terminal/lib/Cargo.toml` if a log-capture dev dependency is needed

Implementation:

1. Add a default method to `TerminalRenderable`:

   ```rust
   fn type_name(&self) -> &'static str {
       std::any::type_name::<Self>()
   }
   ```

2. Replace the current `Debug`-based label heuristic in
   `RenderableTerminalContent::to_tree_nodes` with `component.type_name()`.
3. Add warn-once behavior keyed by `&'static str` type name:
   - first fallback for a concrete type logs `tracing::warn!`
   - subsequent fallback for the same concrete type logs `tracing::debug!`
4. Keep `Strict` behavior as an error and `Lossy` behavior as silent output
   unless the spec explicitly requires debug logging there. The loud policy is
   for the `Warn` fallback footgun.
5. Add a negative-path test with a deliberately un-overridden
   `TerminalRenderable` stub. Assert:
   - projected node is text fallback
   - diagnostic includes the stable Rust type name
   - first call emits exactly one warning
   - second call emits debug, not warn

Risk note:

- Global warn-once state can make tests order-dependent. Use a unique test-only
  stub type and, if needed, expose a `#[cfg(test)]` reset helper local to
  `projection.rs`.

Verification:

```sh
cargo test -p biscuit-terminal projection
cargo test -p biscuit-terminal --lib render_tree::projection
```

## Phase 3c: Retire or Retain Bespoke Hooks

### 3c.1 Retire no-escape-hatch public hooks

Files:

- `biscuit-terminal/lib/src/components/list.rs`
- `biscuit-terminal/lib/src/components/progress.rs`
- `biscuit-terminal/lib/src/components/section.rs`
- `biscuit-terminal/lib/src/components/text_block.rs`
- `biscuit-terminal/lib/src/components/todo.rs`
- `biscuit-terminal/lib/tests/ordered_list_parity.rs`
- `biscuit-terminal/lib/tests/unordered_list_parity.rs`
- `biscuit-terminal/lib/tests/progress_parity.rs`
- `biscuit-terminal/lib/tests/section_parity.rs`
- `biscuit-terminal/lib/tests/text_block_parity.rs`
- `biscuit-terminal/lib/tests/todo_parity.rs`

Implementation:

- Remove `pub fn render_bespoke` and any `_optimistic` public compatibility
  hook from `OrderedList`, `Progress`, `Section`, `TextBlock`, `Todo`, and
  `UnorderedList`.
- Remove now-dead bespoke helper functions only when they have no remaining
  caller.
- Collapse parity tests to structural tree-path checks, or delete tests whose
  only purpose was bespoke comparison.
- Grep for monorepo callers before deletion:

  ```sh
  rg -n "render_bespoke|render_bespoke_optimistic" .
  ```

- Add a one-line note to `lessons-learned.md` for each retired parity scaffold.

### 3c.2 Retain sanctioned escape-hatch hooks

Files:

- `biscuit-terminal/lib/src/components/status_block.rs`
- `biscuit-terminal/lib/src/components/table/table.rs`
- `biscuit-terminal/lib/src/components/two_column.rs`
- `biscuit-terminal/lib/tests/status_block_parity.rs`
- `biscuit-terminal/lib/tests/table_parity.rs`
- `biscuit-terminal/lib/tests/two_column_parity.rs`

Implementation:

- Keep `#[doc(hidden)] pub fn render_bespoke` for:
  - `StatusBlock`: arbitrary border prefix
  - `Table`: `prefer_cursor_alignment` plus TTY path
  - `TwoColumn`: image overlay / `Unsupported` fallback
- Do not demote these to `pub(crate)` because integration tests need access.
- Add top-of-file comments to each retained parity file naming the escape-hatch
  knob under test.

### 3c.3 Apply `FileSystem` action from 3a.3

- Outcome (i): use a temporary bespoke comparator only as long as parity needs
  it, then retire or hide it according to the evidence.
- Outcome (ii) or (iii): leave `FileSystem::render` bespoke and avoid adding a
  public `render_bespoke` hook.
- Record the final action next to the `FileSystem::render` decision in
  `lessons-learned.md`.

Verification:

```sh
rg -n "pub fn render_bespoke|render_bespoke_optimistic" biscuit-terminal/lib/src/components
cargo test -p biscuit-terminal --tests
```

## Phase 3c: Simplify Layout Matrix Harness

Files:

- `biscuit-terminal/lib/tests/layout_matrix_support/mod.rs`
- `biscuit-terminal/lib/tests/layout_matrix.rs`
- `biscuit-terminal/lib/tests/render_comparison.rs`
- `biscuit-terminal/lib/tests/snapshots/layout_matrix__*.snap`

Implementation:

- Rename the right column from `TREE` to `VIA_TREE_DIRECT` in helpers and
  snapshots.
- Use `TreeRenderable::render_tree(component)` for the right column wherever
  possible, not `TerminalRenderable::render_tree_node`, so the matrix compares
  the two public trait entry points:
  - left: `component.render(&term)`
  - right: `render_terminal_node(TreeRenderable::render_tree(component), opts)`
- Keep default-case rows for:
  - `BlockQuote`
  - `Compose`
  - `OrderedList`
  - `Progress`
  - `Section`
  - `StatusBlock`
  - `Table`
  - `TextBlock`
  - `Todo`
  - `TwoColumn`
  - `UnorderedList`
  - `FileSystem` only if 3a.3 chooses outcome (i)
- Add missing rows currently called out by the spec: `OrderedList`,
  `TextBlock`, and `Todo`. Also add `Compose` and `StatusBlock` if absent.
- Exclude escape-hatch scenarios from the default matrix:
  - `BlockQuote::with_border(arbitrary)`
  - `StatusBlock::border(arbitrary)`
  - `Table::prefer_cursor_alignment`
  - `TwoColumn` image overlay
  - `FileSystem::render` if it stays bespoke
- Update the `KNOWN_DRIFT` ledger comment in `render_comparison.rs` so it
  describes `via_render` vs `via_tree_direct`, not bespoke vs tree.
- Regenerate snapshots with:

  ```sh
  INSTA_UPDATE=always cargo test -p biscuit-terminal --test layout_matrix
  ```

Verification:

```sh
cargo test -p biscuit-terminal --test layout_matrix
cargo test -p biscuit-terminal --test render_comparison
```

## Phase 3d: Publish the Migration Checklist

Files:

- `renderable/docs/migrate-component-to-ir.md`
- `renderable/README.md`
- `.claude/skills/renderable/SKILL.md`

Implementation:

- Create `migrate-component-to-ir.md` with:
  - flip-from-bespoke variant
  - born-on-the-tree variant
  - escape-hatch rules
  - `render_bespoke()` retention rules after Stage 3
  - `project_renderable_content(content, ProjectionMode)` guidance
  - CLI helper guidance at `biscuit-terminal/cli/src/commands/shared.rs`
  - terminal, Markdown, MarkdownPlus, and Browser error-fallback policy
  - documentation-update obligations
- Reference the new doc from `renderable/README.md`.
- Reference the new doc from the renderable skill so future agents see it.
- Add a closing "Stage 3 complete" section to `lessons-learned.md` that points
  to this checklist as the canonical onward-path document.

Verification:

```sh
rg -n "migrate-component-to-ir" renderable/README.md .claude/skills/renderable/SKILL.md
cargo test -p renderable --doc
```

## Phase 3e: Cleanup Pre-Existing Failures and Warnings

Files:

- `biscuit-terminal/cli/tests/integration_test.rs`
- `biscuit-terminal/lib/src/components/prose/mod.rs`
- `biscuit-terminal/lib/src/discovery/detection/color.rs`

Implementation:

- Update `test_prose_empty_errors_to_stderr` to assert the clap parse-time
  empty positional argument error, or remove it if the runtime guard is
  unreachable and already covered elsewhere.
- Fix clippy warnings:
  - `needless_borrows_for_generic_args` at prose lines around 409 and 421
  - `clone_on_copy` at color detection around 255

Verification:

```sh
cargo test -p biscuit-terminal-cli --test integration_test test_prose_empty_errors_to_stderr
cargo clippy -p renderable -p biscuit-terminal -p biscuit-terminal-cli --all-targets -- -D warnings
```

## Phase 3e: Verify `NO_COLOR`

Files:

- `biscuit-terminal/cli/tests/integration_test.rs` or the CLI test module that
  owns `bt` command assertions
- Shared terminal detection files only if the new test fails

Implementation:

1. Add one CLI integration test against a tree-rendered command, for example:

   ```sh
   NO_COLOR=1 bt quote "text"
   ```

   or:

   ```sh
   NO_COLOR=1 bt progress 50 --fill-color green
   ```

2. Assert stdout contains no `\x1b[` bytes.
3. If it passes, record verified behavior in `lessons-learned.md`.
4. If it fails, fix at the shared layer, preferably terminal detection:
   `NO_COLOR` set should downgrade to `ColorDepth::None` unless `FORCE_COLOR`
   explicitly overrides it.
5. Do not add per-component stripping or command-local patches.

Verification:

```sh
cargo test -p biscuit-terminal-cli no_color
cargo test -p biscuit-terminal-cli --test integration_test
```

## Final Verification Gate

Run the narrow tests first while iterating, then finish with:

```sh
cargo test -p renderable -p biscuit-terminal -p biscuit-terminal-cli
cargo clippy -p renderable -p biscuit-terminal -p biscuit-terminal-cli --all-targets -- -D warnings
rg -n "TODO\\(stage-3\\)" biscuit-terminal/lib/tests && false || true
rg -n "TREE\\b|bespoke output|KNOWN_DRIFT" biscuit-terminal/lib/tests/layout_matrix_support biscuit-terminal/lib/tests/render_comparison.rs
```

If snapshots were updated:

```sh
git diff -- biscuit-terminal/lib/tests/snapshots
```

Review snapshot changes for renamed headers and newly added default-case rows
only. Any default-case content drift between `via_render` and
`via_tree_direct` is a component regression until proven otherwise.

## Risk Register

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `FileSystem` parity reveals connector, link, or metric behavior that cannot be represented by the tree | Medium | Treat 3a.3 as a decision gate; record outcome (ii) or (iii) with the exact missing capability instead of forcing a flip. |
| Warn-once state makes fallback tests flaky | Medium | Use a unique test-only type and a `#[cfg(test)]` reset helper if needed. |
| Removing hidden bespoke hooks breaks integration tests or monorepo callers | Medium | `rg` callers before deletion; retain `#[doc(hidden)] pub` for sanctioned escape hatches. |
| Layout matrix accidentally includes escape-hatch cases in default parity | Medium | Audit each `ComponentCase`; default rows must avoid arbitrary borders, cursor alignment, image overlays, and bespoke `FileSystem` render. |
| Snapshot churn hides real rendering drift | Medium | Regenerate only after the harness rename and missing rows are correct; inspect diffs component by component. |
| `NO_COLOR` failure tempts command-local ANSI stripping | Medium | Fix shared terminal detection or tree-renderer style application only. |

## Completion Checklist

- [ ] Three missing `render_tree_node` overrides added and parity-tested.
- [ ] Nine existing overrides audited and recorded.
- [ ] `FileSystem::render` decision made from parity evidence and recorded.
- [ ] Nested-component tests assert structural `NodeKind`s.
- [ ] No `TODO(stage-3)` markers remain under `biscuit-terminal/lib/tests/`.
- [ ] Fallback diagnostics use `TerminalRenderable::type_name`.
- [ ] Warn-once then debug fallback behavior is tested.
- [ ] No-escape-hatch `render_bespoke()` hooks are retired.
- [ ] Sanctioned escape-hatch hooks are retained as `#[doc(hidden)] pub`.
- [ ] Layout matrix compares `via_render` to `via_tree_direct`.
- [ ] Missing default-case matrix rows are added.
- [ ] Migration checklist doc exists and is linked.
- [ ] Pre-existing CLI test failure and clippy warnings are fixed.
- [ ] `NO_COLOR` behavior is verified or fixed at the shared layer.
- [ ] Final cargo test and clippy gates pass.
