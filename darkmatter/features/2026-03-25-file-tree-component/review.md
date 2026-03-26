# FileTree Feature Review

> **Status**: All 8 findings resolved. See commit history on `feat/darkmatter-graph-features`.

~~`just test` passes for `darkmatter`, but the current implementation is still incomplete against the spec and tech design. The biggest problems are in follow-mode semantics: several transclusion forms that are supposed to expand into nested `FileTree` nodes either never do, or map to the wrong child.~~

## Highest-priority findings

### 1. `::toc-linking` is marked followable, but never becomes a child subtree

- `darkmatter/lib/src/markdown/reference/types.rs:64-77` says `ReferenceSyntax::DirectiveTocLinking` is followable.
- `darkmatter/features/2026-03-25-file-tree-component/tech-design.md:293-300` also lists `::toc-linking` as followable in v1.
- But `darkmatter/lib/src/markdown/reference/graph.rs:324-374` explicitly models `::toc-linking` without creating any child graph node or `child_insertions` entry.
- `darkmatter/lib/src/markdown/reference/file_tree/model.rs:269-299` can only recurse when a matching `child_node_id` exists, so follow mode cannot actually expand the target document.

Impact:

- `md graph foo.md --follow` renders `::toc-linking` as a leaf edge instead of a nested file tree.
- `md graph foo.md --follow --validate` also misses broken references inside the TOC-linked child, because that child never enters the graph.

Reproduced:

- Root: `::toc-linking child.md`
- Child: `## Child Heading` plus `[broken](./missing.md)`
- Current result: `2 references scanned, 2 valid, 0 issues`
- Expected: the child document should render as a nested `FileTree`, and the broken child link should be reported when `--follow --validate` is used.

Recommendation:

- Treat `::toc-linking` like the other followable local markdown transclusions during graph construction.
- Add a `child_insertions` entry and child graph node for the target markdown file, while still keeping the synthesized TOC hyperlinks.

### 2. Epilogues can never follow, because the transclusion record and insertion metadata use different line numbers

- Epilogue records are emitted at line `0` in `darkmatter/lib/src/markdown/reference/graph.rs:428-443`.
- The corresponding child insertion is recorded at `usize::MAX` in `darkmatter/lib/src/markdown/reference/graph.rs:459-471`.
- `FileTree` matches a transclusion row to its child insertion only by `directive_line == record.origin.line` in `darkmatter/lib/src/markdown/reference/file_tree/model.rs:269-278`.

Impact:

- `FrontmatterEpilogue` rows never receive a `child_node_id`.
- `md graph foo.md --follow` shows the epilogue edge, but never renders the nested child subtree.
- The graph/validation backends can still know about the child, so the CLI can report issues that the visual tree does not show. That makes the graph output misleading.

Reproduced:

- Root frontmatter: `epilogue: [a.md]`
- Child `a.md`: contains a broken local link
- Current result: the CLI exits with code `2` and reports `1 issue`, but the nested `a.md` subtree is not rendered at all.

Recommendation:

- Stop matching by raw line number alone.
- Either give epilogues a stable insertion key that matches both the transclusion record and the insertion metadata, or map transclusion records to insertions by `(syntax, insertion_order)` / reference id instead of just the line.

### 3. Multiple prologues on the same line duplicate the first child and drop the rest

- Frontmatter prologues all emit transclusion records on line `0` in `darkmatter/lib/src/markdown/reference/graph.rs:378-393`.
- Their insertions also all use line `0` in `darkmatter/lib/src/markdown/reference/graph.rs:409-418`.
- `FileTree` resolves the insertion with `.find(...)` on the first matching line in `darkmatter/lib/src/markdown/reference/file_tree/model.rs:269-278`.

Impact:

- When there are multiple prologues, every rendered transclusion row binds to the first child insertion.
- In follow mode the first child is rendered repeatedly and later prologue children disappear.

Reproduced:

- Root frontmatter:
  - `prologue: [a.md, b.md]`
- Current `md graph root.md --follow` output renders two transclusion rows (`a.md`, `b.md`) but the nested subtree is `a.md` twice and `b.md` never appears.

Recommendation:

- Preserve a stable mapping between each transclusion record and its insertion metadata.
- Line numbers are not enough once multiple insertions share the same source position.

## Missing or incomplete functionality

### 4. `show_root(false)` hides the entire tree, not just the root file label

- The public API says `show_root` controls whether to show the root file label in `darkmatter/lib/src/markdown/reference/file_tree/mod.rs:164-167`.
- Both render paths return `String::new()` immediately when `show_root == false` in `darkmatter/lib/src/markdown/reference/file_tree/render.rs:25-47`.
- The test at `darkmatter/lib/src/markdown/reference/file_tree/render.rs:397-401` currently codifies this behavior.

Impact:

- The method name and docs imply “hide the head row”, but the implementation means “render nothing”.
- That makes the option unusable for embedding a subtree into another render context.

Recommendation:

- Render the references/transclusions/children even when the root head row is hidden.
- Rename the option if the intent really is “suppress all output”, but that would be a surprising API.

### 5. Section captions ignore the actual heading level

- `ReferenceInsertionContext` stores `section_heading_level`.
- `graph.rs` populates it for both `::file` and epilogue insertions in `darkmatter/lib/src/markdown/reference/graph.rs:300-310` and `:459-469`.
- `transclusion_caption()` ignores that field and hardcodes `##` in `darkmatter/lib/src/markdown/reference/file_tree/model.rs:377-382`.

Impact:

- A transclusion inside `### Details` is rendered as if it were inside `## Details`.
- The data model already has the correct level, so this is an avoidable presentation error.

Recommendation:

- Render heading markers from `section_heading_level` instead of hardcoding H2.

## Test coverage gaps

The current tests are too narrow for a feature with multiple transclusion modes.

- Library integration coverage only exercises a plain `::file` case in `darkmatter/lib/tests/reference_integration.rs:778-820`.
- CLI coverage only checks the happy-path `::file` follow case and basic validation in `darkmatter/cli/tests/cli.rs:1409-1480`.
- There are no tests for:
  - `::toc-linking` in follow mode
  - `::toc-linking --follow --validate` catching issues in the child document
  - frontmatter `prologue` follow mode with multiple entries
  - frontmatter `epilogue` follow mode
  - `show_root(false)` preserving the rest of the tree
  - section captions for non-H2 headings

Recommendation:

- Add both unit and integration tests for each followable transclusion form separately: `::file`, `::toc-linking`, `prologue`, `epilogue`.
- Add CLI integration tests for `--follow --validate`, not just `--follow` and `--validate` independently.
- Add a regression test for “multiple insertions at the same logical position” so frontmatter lists cannot silently duplicate the wrong child again.

## Ergonomics and performance opportunities

### 6. `FileTree` silently renders an empty string until `ensure_built()` is called

- `ensure_built()` is explicit in `darkmatter/lib/src/markdown/reference/file_tree/mod.rs:178-212`.
- But `render()` and `render_optimistic()` just return `String::new()` when the model has not been built in `darkmatter/lib/src/markdown/reference/file_tree/mod.rs:226-245`.

Impact:

- This is an easy API footgun for library consumers.
- A missed `ensure_built()` call looks like “the file has no graph” instead of an actionable error.

Recommendation:

- At minimum, document this more aggressively.
- Better options would be: build eagerly on construction, return a diagnostic placeholder in debug/test builds, or expose a fallible render API that can build on demand.

### 7. Recursive model building does repeated linear `node_by_id()` lookups

- `FileTree` looks up every child with `graph.node_by_id()` in `darkmatter/lib/src/markdown/reference/file_tree/model.rs:293-296`.
- `ReferenceGraph::node_by_id()` is a linear scan over `nodes` in `darkmatter/lib/src/markdown/reference/types.rs:303-310`.

Impact:

- Deep or broad transclusion trees turn model construction into avoidable O(n^2) lookup work.

Recommendation:

- Build an `id -> &ReferenceGraphNode` map once per `build_file_tree_model()` call and reuse it during recursion.

### 8. Line truncation is not robust for ANSI or Unicode-heavy output

- `truncate_line()` slices by byte count in `darkmatter/lib/src/markdown/reference/file_tree/render.rs:236-245`.
- The same function is fed strings that already contain ANSI escapes and multi-byte Unicode icons in `render_reference_groups()` and `render_transclusion_edges()`.

Impact:

- Narrow-width rendering can mis-measure visible width.
- Byte slicing on formatted Unicode/ANSI strings is brittle and can eventually panic or corrupt styling.

Recommendation:

- Keep connector prefix and content width accounting separate until the final write step, as the tech design recommends.
- Use display-width-aware truncation instead of `line[..width]`.
