---
ready: false
implemented: true
agent: codex/default
created: 2026-06-21T09:23:06
---

# Review 2

## Findings

### High: The explicit performance target is not met

The implementation does remove the per-directory marker probes in `walk_for_nested_markers` and therefore materially reduces the syscall surface ([nested.rs:248](../../lib/src/filesystem/repo/nested.rs#L248)). However, the feature spec defines the warm-cache target as `~10-20ms` for the worktree path ([spec.md:105](spec.md#L105)), and the feature's own validation records `~60-80ms wall / ~61-66ms internal`, with pre- and post-refactor timings essentially unchanged ([plan.md:129](plan.md#L129), [commit-message.txt:17](commit-message.txt#L17)).

That leaves the feature short of a stated acceptance criterion for "faster package list". The plan argues the value is reduced contention surface rather than warm latency, which is plausible, but then the spec should be updated to make that the production criterion and remove the 10-20ms target. Otherwise, continue optimizing the remaining dominant work (`ignore` walk, Cargo workspace expansion, structure assembly) until the stated target is met.

Strongest verification present: manual L1/host timing via CLI. That is an appropriate level for a performance requirement, but it currently verifies that the stated latency target was not achieved.

### Medium: The original contention failure mode was not reproduced in validation

The spec says the production risk is write-heavy contention from Claudine running `sniff repo packages` while subagents commit in parallel and a background build may be active ([spec.md:9](spec.md#L9)). The validation only reports read-only `find target/` load and explicitly says the >1.8s spike was not reproducible on this host ([plan.md:131](plan.md#L131), [commit-message.txt:31](commit-message.txt#L31)). The syscall-count reduction is the right structural change, but there is no direct evidence here that the write-contention timeout scenario is fixed.

Before marking this production-ready, either capture a repeatable validation under the motivating write-heavy load, or revise the spec/acceptance language to state that this change only removes the known amplification mechanism and does not claim observed latency improvement under the original scenario.

Strongest verification present: manual L1/host timing under a weaker synthetic load. This is not a terminal-rendering/input requirement, so L2/L3 is not applicable.

## Notes

The previous review's three test-sensitivity findings appear addressed. The root-marker case now has a direct unit test on `walk_for_nested_markers`, and the `node_modules` prune and gitignored-marker fixtures now contain markers that would produce real layers if the old or regressed behavior were present ([nested.rs:394](../../lib/src/filesystem/repo/nested.rs#L394), [fixtures.rs:979](../../lib/tests/fixtures.rs#L979), [fixtures.rs:1017](../../lib/tests/fixtures.rs#L1017)).

For user-observable CLI output, there were no CLI code changes and the feature artifacts report byte-identical output checks for text, plain, markdown/list, JSON, and aggregate repo JSON modes. For the nested detection semantics, L1 in-process tests are the right verification level; no requirement here needs L2 real-terminal capture or L3 OS keyboard injection.

Focused checks run during this review:

```text
cargo test --color=never --manifest-path sniff/lib/Cargo.toml --test integration test_nested_pnpm_and_dotnet_both_discovered_via_single_pass
cargo test --color=never --manifest-path sniff/lib/Cargo.toml --test integration test_root_marker_is_not_registered_as_nested_candidate
cargo test --color=never --manifest-path sniff/lib/Cargo.toml --test integration test_node_modules_package_json_is_pruned
cargo test --color=never --manifest-path sniff/lib/Cargo.toml --test integration test_gitignored_nested_marker_is_not_detected
cargo test --color=never --manifest-path sniff/lib/Cargo.toml marker_name_matches_is_exact_on_unix_and_case_insensitive_on_windows
cargo test --color=never --manifest-path sniff/lib/Cargo.toml root_marker_does_not_register_a_candidate
```

All focused checks passed. Production readiness: **not ready** until the performance acceptance criteria and validation evidence are reconciled.
