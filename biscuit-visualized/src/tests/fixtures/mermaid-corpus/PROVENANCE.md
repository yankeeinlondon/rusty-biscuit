# Mermaid comparison corpus

A small, frozen set of Mermaid inputs used to compare renderer and rasterizer
versions. Each `.mmd` file holds the source exactly as taken from its origin;
provenance lives here rather than in a `%%` comment so the input bytes match
the origin and a comment cannot change how a parser treats them.

Upstream inputs come from
[mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer) at
`3726ccbffe0e8032361eb9668694b24f77858060`; `v0.3.1` is
`2f993bd79a55235eb59a34d807852276ba25bea7`. None of these inputs is a
diagram a user reported as failing: no such diagram has been reproduced yet.

| File | Origin | Upstream change it exercises | Expected behavior |
| ---- | ------ | ---------------------------- | ----------------- |
| `flowchart_decision_fan.mmd` | upstream `tests/layout_suite.rs`, `decision_branches_use_lower_sides_independent_of_declaration_order` (`screenshot_repro`) | `3e431ec` fan decision branches from diamond sides (issue #129) | each decision's two branches leave the diamond symmetrically from its lower sides, without crossing |
| `flowchart_cycles.mmd` | upstream `tests/fixtures/layout_regressions/flowchart_cycles.mmd` | `90a1bfc`, `7578b7b` layout and routing audit fixes | back edges and the `LR` subgraph cycle route without passing through nodes; every edge label visible |
| `radar_long_names.mmd` | upstream `tests/fixtures/radar/long_names.mmd` | `b783fbb` axis label anchoring and canvas growth; `92807b4` legend fixtures | every curve name and axis label fully visible inside the canvas |
| `radar_short_curve.mmd` | upstream `tests/fixtures/radar/short_curve.mmd` | `a560e0c` radar data pipeline; `e9f5e19` missing values pad to the center | `Short` plots 2, 3, 4 on A–C and the center (0) on D and E |
| `state_fork_join.mmd` | upstream `benches/fixtures/state_forkjoin.mmd` | `ac37239` pseudostate marker collapse with fork/join bars | `Fork` and `Join` draw as bars; start and end markers stay distinct; `Alpha` and `Beta` sit between them |
| `requirement_basic.mmd` | upstream `tests/fixtures/requirement/basic.mmd` | `1da35e5` divider struck through the ID row; `75b0963` header band height from font size | the type header sits inside the box; the divider runs between the name and the `ID:` row without touching text |
| `gitgraph_labeled_merge.mmd` | `src/src/tests/gitgraph_tests.rs`, `a_labeled_merge_gets_its_source_tip_as_second_parent` | local labeled-merge repair | `M` has parents `B` and `S2`, sits on `main`, and carries the `origin/main` tag |
| `gitgraph_tag_spacing.mmd` | `src/src/tests/gitgraph_tests.rs`, `long_labels_on_neighboring_commits_do_not_overlap` | local tag spacing | the three tags do not overlap each other, and each tag's text stays inside its tag box |
