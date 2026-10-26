# gitGraph layout fixtures

Read by `gitgraph_layout_tests` with `include_str!` inside each test.

- `layout.mmd` and `layout.json`: the loader's control. `layout.json` is
  Merman's unedited `RenderRequest::layout_json` output for `layout.mmd`
  at the pinned rev (site config `{"theme": "dark"}`, measured with the
  bundled Inter). A test regenerates it and fails when the `layout` section
  drifts.
- `tag_collision_adjacent.mmd`, `labeled_merge.mmd`: this repository's
  Merman comparison corpus (colliding tags on consecutive commits; a labeled
  merge).
- `long_labels.mmd`, `observation_1.mmd`: the Mermaid text that `worktree`'s
  `gathered_graph_lays_out_long_labels` and `gathered_graph_lays_out_observation_1`
  tests lay out.
- `empty_branch_merge.mmd`: a merge of a branch with no commit of its own.

All were written in this repository, so no third-party license applies.
