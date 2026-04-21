# FileTree Feature Review, Pass 2

`just test` now passes for `darkmatter`, and the main functional issues from the first review appear to be fixed in the implementation:

- `::toc-linking` now creates child graph nodes and follows correctly.
- epilogues now match their child insertions and render/follow correctly.
- multiple prologues no longer collapse onto the first child.
- `show_root(false)` now preserves the rest of the tree.
- caption rendering now uses the recorded heading level.

I also manually smoke-tested the CLI for:

- `md graph root.md --follow --validate` with `::toc-linking`
- `md graph root.md --follow` with multiple prologues

Both worked as expected.

## Remaining findings

### 1. The CLI addition is still under-tested relative to the feature surface

- The `graph` CLI tests in `darkmatter/cli/tests/cli.rs:1409-1480` are unchanged from the previous pass.
- They still cover only:
  - one plain `::file` follow case
  - one basic valid `--validate` case
  - one basic invalid `--validate` case
- There is still no CLI coverage for:
  - `::toc-linking` with `--follow`
  - `::toc-linking` with `--follow --validate`
  - frontmatter `prologue` with `--follow`
  - frontmatter `epilogue` with `--follow`
  - the validation exit-code path on recursive follow-mode failures

Impact:

- The component/library behavior is much better covered now, but the user-facing CLI contract added by the spec is still not.
- A regression in `run_graph()` output, formatting, or exit semantics for the newly fixed follow cases could slip through without any failing CLI test.

Recommendation:

- Add CLI integration tests for the newly fixed cases, not just library integration tests.
- At minimum add:
  - `graph --follow` with `::toc-linking`
  - `graph --follow --validate` where the followed child has a broken local link
  - `graph --follow` with multiple prologues
  - `graph --follow` with epilogue

### 2. One of the new library tests is still too weak to prove recursive render behavior

- `darkmatter/lib/tests/reference_integration.rs:825-842` adds `file_tree_toc_linking_follow_mode`.
- That test only asserts that the output contains `child.md`.
- But the transclusion edge itself already contains `child.md`, so this assertion can still pass even if the nested subtree is missing.

Impact:

- This test does not actually prove that follow mode rendered the child `FileTree`.
- It would not catch a regression where `::toc-linking` falls back to a leaf edge again but still prints the target path.

Recommendation:

- Tighten this test to assert nested child-only content, for example:
  - the child hyperlink `https://child.example.com`, or
  - a second occurrence pattern that proves both the edge and the nested child node rendered.

## Assessment

I did not find another blocking implementation bug in the current pass. The remaining work is mainly to raise the automated test bar on the CLI path and strengthen one library regression test so it actually proves the recursive render behavior it is meant to cover.
