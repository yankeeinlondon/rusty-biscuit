# Stage 3 Implementation Review

## Findings

### 1. BlockQuote still flattens nested block components, contrary to the Stage 3 structural-projection goal

**Severity:** Medium  
**Files:** `biscuit-terminal/lib/src/components/block_quote.rs:526`, `biscuit-terminal/lib/tests/render_tree_component_parity.rs:345`

Stage 3's core problem statement and S3-2 companion tests call out `Section-in-BlockQuote` and `List-in-BlockQuote` as cases that should move from "text survives" to "structural kind survives." The implementation does the opposite: `BlockQuote::paragraph_children` forces `ProjectionMode::InlineOnly`, and the new tests assert that nested `Section`, `OrderedList`, and `UnorderedList` must *not* appear structurally.

That means a `BlockQuote` containing a non-`Prose` IR-aware component still degrades that child into ANSI-stripped text. This is exactly the class of loss Stage 3 was intended to close.

**Suggested fix:** Either change `BlockQuote` projection to preserve structural children for non-inline content, or explicitly amend `stage3-spec.md` and the acceptance criteria if `BlockQuote` is intentionally exempt. If keeping the current architecture, at minimum the completion note should not claim all container nested tests now assert structural `NodeKind`s.

### 2. Warn-once fallback is bypassed for the most common terminal-context projection path

**Severity:** Medium  
**File:** `biscuit-terminal/lib/src/render_tree/projection.rs:304`

S3-3 says a future component that forgets `render_tree_node` should be observable via a warn-once fallback. The direct `RenderableTerminalContent::to_tree_nodes` path now does that, but `project_renderable_content(..., ProjectionMode::Structural { terminal_hint: Some(term) })` short-circuits first: it calls `component.render_tree_node().is_none()`, renders to text, and returns without emitting the warn/debug event or diagnostic.

Containers that render with a terminal context, such as `Compose::render(term)` and list rendering paths that pass a terminal hint, can therefore still silently flatten bespoke-only children. This keeps the original footgun alive in production entry points.

**Suggested fix:** Route the terminal-hint fallback through the same warn-once helper, or add a small shared fallback function that both `to_tree_nodes` and the terminal-hint branch call. Add a regression test that projects an un-overridden component through `project_renderable_content` with `terminal_hint: Some(_)` and asserts the first call warns and the second debugs.

### 3. Compose fixture tests are weaker than the spec and can pass with misplaced or nested matches

**Severity:** Low  
**File:** `biscuit-terminal/lib/tests/compose_parity.rs:118`

The S3-2 fixture table requires each Compose row to assert that the corresponding child exists at the expected top-level index and that the child's `NodeKind` matches the expected discriminant. The tests currently use `children.iter().any(|c| walk_has_kind(...))`, which searches anywhere under any child.

That means these tests would still pass if the component appeared under the wrong sibling, if order regressed, or if an unrelated nested node happened to have the expected kind. This is especially loose for `Paragraph` cases (`Progress`, `TextBlock`, `StatusBlock`) and `BlockQuote` cases (`BlockQuote`, `TwoColumn`).

**Suggested fix:** Replace the `any + walk_has_kind` assertions with direct checks against `children[0]` for single-fixture Compose cases. Keep recursive checks only where the expected behavior is explicitly "flatten nested Root into top-level children," and then assert the resulting top-level sequence.

### 4. FileSystem gitignore coverage does not actually exercise gitignored entries, and the Stage 4 gate omits that latent gap

**Severity:** Low  
**Files:** `biscuit-terminal/lib/tests/filesystem_parity.rs:194`, `renderable/features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md:1720`

S3-1c required the `FileSystem` decision to be based on parity results across the documented variants, including gitignore styling. The current fixture creates a `.gitignore`, but the scanner still hardcodes `is_ignored: false`, so the test only asserts that neither path emits dim styling.

That is useful documentation, but it is not coverage of gitignore styling. The recorded Stage 4 acceptance criterion then lists only connector-list style lowering, icon-name spacing, and three named divergence fixtures to invert; it does not require a real ignored-entry fixture to become active once ignore support lands.

**Suggested fix:** Add a direct projection fixture using a manually constructed ignored `TreeNode` if possible, or record gitignore as an explicit Stage 4/Phase 8 acceptance item. The future gate should name `fixture_gitignore_styling_records_divergence` alongside the other divergence fixtures instead of treating it as already green.

### 5. `tree-rendering.md` still documents pre-Stage-3 state after the implementation

**Severity:** Low  
**File:** `renderable/docs/tree-rendering.md:141`

The implementation added `render_tree_node` overrides for `BlockQuote`, `StatusBlock`, and `FileSystem`, but `renderable/docs/tree-rendering.md` still says those three components are missing the override and still describes the old `BlockQuote`-inside-`TwoColumn` flattening behavior as pending Stage 3 work.

This is now misleading, especially because the same implementation added new Stage 3 documentation elsewhere.

**Suggested fix:** Update or remove the stale caveats in `tree-rendering.md`. If `BlockQuote` remains intentionally `InlineOnly`, document that as the remaining exception instead of saying the missing overrides are the issue.

## Verification Notes

This review was based on source inspection against `stage3-spec.md`; I did not rerun the full 3,465-test gate.
