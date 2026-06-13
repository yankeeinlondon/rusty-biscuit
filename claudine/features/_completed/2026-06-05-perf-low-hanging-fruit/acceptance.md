# Acceptance Verification — Eliminate Redundant Repo-Root Detection

Captured 2026-06-05 in response to `review-1.md`, updated for `review-2.md`. All
commands run from the repo root
(`/Users/ken/.claudine/worktrees/rusty-biscuit/claudine`).

## review-2.md: `None` fallback now proves repo-root resolution

`review-2.md` flagged that the old `build_repo_home_env_fallback_to_cwd_when_no_effective_root`
test placed `.claude/commands` directly under the directory it passed as `cwd`,
so it would still pass if the fallback degraded from `resolve_repo_root(cwd)` to
`cwd.to_path_buf()`. Replaced with
`build_repo_home_env_fallback_resolves_repo_root_from_nested_cwd`: it `git init`s
a real temporary repo, writes `.claude/commands/review.md` at the **repo root**,
and passes a nested subdirectory (`repo/crate/src/deep`) as `cwd`. The root-level
prompt only materializes if `resolve_repo_root` walks up to the git root.

Verified the test fails against the degraded behavior: temporarily swapping the
fallback to `cwd.to_path_buf()` made the test panic
(`expected root-level review.md to materialize via resolve_repo_root from nested
cwd`); restoring `resolve_repo_root(cwd)` returns it to green. The test
self-skips when `git` is unavailable, matching the existing
`resolve_repo_root_returns_nested_git_root` pattern in `linking/paths.rs`.

## Targeted unit tests

`claudine-cli` is a binary crate (no `--lib` target), so module tests run via
`--bins`.

```text
$ cargo test -p claudine-cli --bins repo_home --color=never
running 11 tests
test commands::wrap::repo_home::tests::needs_shadow_home_supplied_effective_root_used_for_codex_detection ... ok
test commands::wrap::repo_home::tests::needs_shadow_home_repo_only_short_circuits_regardless_of_effective_root ... ok
test commands::wrap::repo_home::tests::build_repo_home_env_uses_supplied_effective_root_not_cwd ... ok
test commands::wrap::repo_home::tests::build_repo_home_env_fallback_resolves_repo_root_from_nested_cwd ... ok
... (7 more)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1104 filtered out
```

## Source-repo vs launch-repo regression (real env wiring)

The new L1 test exercises `build_child_env_with_launch -> needs_shadow_home ->
build_repo_home_env` with `repo_root != child_cwd` and asserts the Codex
shadow-HOME prompts come from `child_cwd`, not the source metadata root.

```text
$ cargo test -p claudine-cli --bins commands::wrap::env --color=never
running 18 tests
test commands::wrap::env::tests::build_child_env_codex_shadow_home_uses_child_cwd_not_source_repo_root ... ok
... (17 more)
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 1097 filtered out
```

## Compile check (both crates)

```text
$ cargo check -p claudine -p claudine-cli --color=never
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

## Perf smoke — redundant shadow-HOME repo detection collapsed

`claudine compose --perf --dry-run --repo <file>` against a Codex composition
document inside a repo with `.claude/commands` (so the shadow-HOME branch runs):

```text
▌ ├─ environment setup                     210.5ms   19%
▌ │  ├─ child env build                      3.5ms   <1%
▌ │  │  ├─ env sanitize                      442µs   <1%
▌ │  │  └─ shadow home sync                  2.9ms   <1%
▌ │  │     └─ repo root detect                 0µs   <1%
```

`repo root detect` is **0µs** (microsecond-scale, known-root reuse) versus the
previous 660ms–2s `resolve_repo_root` sniff git walk. `shadow home sync` now
reflects only the filesystem linking work, and the overall perf tree shape is
preserved.
