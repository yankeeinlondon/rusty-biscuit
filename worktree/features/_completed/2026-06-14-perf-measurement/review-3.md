---
ready: true
agent: codex
model: ""
---

# Review 3 — Performance Measurement

Feature is **not ready for production** because the iteration-3 implementation fixes the review-2 runtime diagnostic gap, but the new integration fixture is host-config-dependent. The functionality now attributes the non-image verbose data gather as `verbose gather`, and the existing `--perf` behavior otherwise matches the spec.

## Findings

### Medium — new verbose `--perf` integration test depends on the host's default Git branch

The new fixture in `worktree/cli/tests/perf_flag.rs:9-20` initializes a repo with plain `git init`, while `temp_repo_with_feature_branch` later runs `git checkout main` at `worktree/cli/tests/perf_flag.rs:65-77`. That only works on machines where Git is configured to create `main` by default.

On a clean Git config, `git init` still creates `master`:

```text
HOME=<empty> XDG_CONFIG_HOME=<empty> GIT_CONFIG_NOSYSTEM=1 git init ...
branch --show-current => master
```

So `list_perf_non_image_verbose_includes_verbose_gather` can fail before it verifies the feature. The in-process fixture in `worktree/cli/src/commands/list.rs` already uses `git init -b main`; the integration fixture should do the same, or it should discover the initialized branch instead of hard-coding `main`.

Verification level: Level 1 is still the correct level for this requirement because it verifies CLI process output and stage accounting. The issue is determinism, not missing Level 2 or Level 3 coverage.

## Requirement Coverage

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| R1 Criterion bench for `list_worktrees()` | Bench target compile check: `cargo bench -p worktree --bench list_status --no-run --color=never` | Implemented |
| R2 `bench`, `bench-save`, `bench-compare` recipes | Code inspection | Implemented |
| R3 `wt list --perf` runtime diagnostic | Level 1 unit + binary integration tests | Implemented |
| R4 README performance docs | Code inspection | Implemented |
| R5 performance-testing docs + hash | `md hash worktree/docs/performance-testing.md` matched frontmatter | Implemented |
| R6 `--perf` tests | Level 1 unit + binary integration tests | Mostly implemented; new verbose integration fixture is not host-portable |
| AC4 no non-perf stage timing | Level 1 `run_pipeline_without_perf_produces_no_collector` plus code inspection | Implemented |
| AC5 non-image graph stages omitted | Level 1 integration and in-process tests | Implemented |
| AC6 stdout empty for `--perf` | Level 1 binary integration test | Implemented |

No Level 2 or Level 3 coverage is required for the current `--perf` requirements. The feature does not specify terminal key encoding, OS keyboard injection, glyph-width-sensitive rendering, or SGR color fidelity as acceptance behavior. Level 1 binary and in-process tests are the right tier for the CLI diagnostic content and stage accounting.

## Verification Run

- `cargo test -p worktree-cli perf --color=never` — passed on this host.
- `cargo bench -p worktree --bench list_status --no-run --color=never` — passed.
- `md hash worktree/docs/performance-testing.md` — returned `ef46db3751d8e999-5f1753d5627d5caa`, matching frontmatter.

## Recommendation

Fix the integration fixture to initialize `main` deterministically, then ship. The review-2 functional gap is resolved.
