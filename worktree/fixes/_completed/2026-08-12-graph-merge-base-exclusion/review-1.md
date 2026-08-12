---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-08-12T19:44:28+01:00
spec: 2026-08-12-graph-merge-base-exclusion/spec.md
implemented: false
description: "A **fix** review of `2026-08-12-graph-merge-base-exclusion/spec.md`"
fix: 2026-08-12-graph-merge-base-exclusion/review-1.md
---

# Review 1: Graph Merge-Base Exclusion

## Verdict

The fix is **ready for production**. The implementation changes every affected
lane, hidden-count, and verbose-detail query to exclude the opposite tip while
preserving the selected merge-base solely for shared context, anchor placement,
and merge-base detail. The criss-cross fixture is non-vacuous, independent of
which incomparable best base plain `git merge-base` selects, and fails the old
query model by construction.

I found no correctness, performance, ergonomics, or test-rigor defect that
should block release. One pre-existing branch-cap mismatch remains in the
documentation touched by this fix; it is recorded below as non-blocking because
the implementation intentionally preserves the existing five-commit overview
cap.

## Findings

### Low: base-overview documentation claims a ten-commit branch cap

[`docs/cli/list.md`](../../docs/cli/list.md) and the forward-looking
[`docs/git-graph.md`](../../docs/git-graph.md) say the base overview shows up to
10 commits per worktree branch. `gather_branch_for_base` still calls
`commits_since(branch, default_branch, 5)`, so the actual cap is five. The
implementation did not introduce this behavioral mismatch, and preserving caps
is part of this fix's scope, but the edited documentation now restates an
incorrect public/reference contract. Change both documents to five, or change
the implementation and its subprocess/rendering expectations in a separately
specified behavior change.

## Requirement Verification

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1: criss-cross focused graph uses disjoint tip-unique lanes with no in-window elision | Level 1: `criss_cross_fixture_has_two_incomparable_best_bases` and `criss_cross_focused_graph_uses_tip_unique_commits` use a real temporary Git repository and structurally parse the Mermaid lane segments | Sufficient. The tests compare full SHAs and independent opposite-tip Git queries, prove two incomparable best bases, and prove common history remains outside the selected base. |
| AC2: each `+N` equals unique count minus the five displayed commits and is fork-adjacent | Level 1: `worktree_graph_marks_elided_commits` | Sufficient. This is deterministic Git-set and Mermaid-instruction behavior; no terminal encoder or rendering semantics determine the count. |
| AC3: verbose detail is the oldest-first feature-unique sequence | Level 1 semantic regression: `criss_cross_verbose_details_use_feature_unique_commits`; Level 2 generic text path: `level2_list_verbose_renders_table_and_verbose_in_tmux` | Sufficient. Level 1 proves exact sequence membership/order through real Git; existing Level 2 coverage proves the unchanged verbose terminal presentation path. |
| AC4: base overview contains only branch-unique commits and anchors at the selected base | Level 1: `criss_cross_base_graph_uses_unique_commits_at_selected_base` | Sufficient. The test derives rather than assumes the selected base and verifies the rendered branch block structurally. |
| AC5: linear topologies remain byte-identical | Level 1: `linear_focused_graph_is_unchanged_by_opposite_tip_exclusion` and `over_window_focused_graph_is_unchanged_by_opposite_tip_exclusion` | Sufficient. Both the ordinary and over-window fixtures compare complete Mermaid output under legacy and opposite-tip exclusion. |
| AC6: subprocess budget is unchanged | Level 1: `gather_branch_uses_one_merge_base_and_no_short_sha` and `base_graph_subprocess_count_is_bounded` | Sufficient. Exact `merge-base`, `log`, `rev-list`, and `rev-parse --short` counts are asserted. |
| AC7: source and affected documentation use unique-tip terminology | Level 1/static inspection of `git_graph.rs` and the three specified documents | Implemented for exclusion semantics. The unrelated five-versus-ten cap drift is the low finding above. |
| AC8: behavior does not depend on merge-base choice, hashes, timestamps, output order, or host OS | Level 1: unordered full-SHA merge-base set, ancestry assertions, and dynamically derived selected-base index | Sufficient. Fixture construction uses non-interactive `Command` arguments and conflict-free files, with no shell quoting or platform-specific path assumptions. |
| User-visible inline graph reaches a real image terminal | Level 2: existing `level2_graph_emits_image_protocol_bytes_in_kitty` | Sufficient for the unchanged Mermaid-to-image/Kitty transport. It does not prove criss-cross lane membership itself; that semantic contract is proven at Level 1 before the renderer. |

No Level 3 verification is applicable. This fix changes no keyboard, mouse,
paste, IME, hotkey, or terminal input-encoder behavior.

## Implementation Assessment

- `gather_branch_impl` consistently uses `default --not branch` for the default
  lane and `branch --not default` for the branch lane, including hidden counts
  and verbose details.
- `default_unique` and `branch_unique` accurately encode the revised data
  contract; their field and accessor documentation no longer implies a single
  merge-base defines lane membership.
- `GatherScope::BaseOverview` retains its reduced query set, so correctness does
  not add subprocesses or redundant default-lane work.
- The implementation remains portable across macOS, Windows, and Linux: Git is
  invoked with `std::process::Command`, test merges are non-interactive, and
  fixture paths are passed as arguments rather than shell fragments.
- The change adds no dependencies, cache invalidation requirements, or new
  public API surface.

## Validation Performed

- `worktree/just build`: passed for `biscuit-terminal`, `worktree`, and
  `worktree-cli` on macOS.
- `just test worktree-cli` from the repository root: 113/113 Level 1 tests
  passed; 14 higher-tier tests were excluded by the canonical filter.
- `cargo nextest run -p worktree --no-tests=fail`: 37/37 tests passed. This was
  used after the package wrapper's captured output ended before its final
  summary on two attempts.
- `worktree/just lint`: passed for `worktree` and `worktree-cli`.
- `git diff --check` on the implementation and affected documentation: passed.

The existing Level 2 suite was inspected for requirement classification but
was not rerun because this fix does not change terminal rendering or input
behavior.
