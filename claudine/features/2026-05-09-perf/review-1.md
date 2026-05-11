---
ready: true
agent: codex
model: ""
---

# Review 1

> **Status:** All five findings addressed (2026-05-10).
>
> - Finding 1: `launch_workspace_context_from_repo_info` now takes a
>   `source_repo_root_hint` and preserves the legacy split contract
>   (`repo_root` follows the source for metadata; `child_cwd` follows the
>   launch repo). Unit tests cover the out-of-repo, in-repo, and no-launch-
>   repo cases (`claudine/cli/src/commands/wrap/env.rs`).
> - Finding 2: `--perf` totals no longer round-trip through formatted
>   strings. CLI Overhead sums only the disjoint rows
>   (`pre-dispatch + prep_phase + environment_setup`); diagnostic
>   sub-buckets and substages are explicitly excluded from the sum.
>   Composition Report TOTAL now uses `compose.total` directly.
> - Finding 3: Level 1 CLI tests added in
>   `claudine/cli/tests/compose_receipt_banner.rs` cover ordering vs the
>   execution header, `--silent` suppression, and `--quiet` retention for
>   both `compose` and `inline-compose`. The implementation was corrected
>   so `--quiet` no longer suppresses the banner (matches spec W1).
> - Finding 4: `resolve_binary_path_direct` snapshot test confirms the
>   `which::which` lookup is skipped when the snapshot has the path; a
>   companion test confirms graceful fallback otherwise. New
>   `prep_context` tests pin the W0 split contract end-to-end.
> - Finding 5: `render_perf_report_snapshot_locks_totals_and_alignment`
>   in `claudine/cli/src/perf.rs` asserts microsecond rendering, long-
>   label spacing, both fixed-section TOTALs, and dry-run agent omission.

## Findings

### High: Reused launch workspace drops the source-repo root for out-of-repo prompt files

The W0 path builds `prep_context.launch_workspace` from only the launch-CWD sniff data (`claudine/cli/src/commands/wrap/composition/prep_context.rs:142-153`) and then `execute_composition_request_inner` uses it whenever present (`claudine/cli/src/commands/wrap/composition/mod.rs:556-560`, `:770-779`). That bypasses the old `resolve_launch_workspace_context(&launch_cwd, source_repo_root)` behavior, whose contract explicitly used `source_repo_root` as metadata for guardrails, MCP defaults, and harness path resolution while keeping `child_cwd` anchored to the launch repo.

For the common same-repo prompt case this is invisible because launch root and source root match. For a supported sibling-clone / external prompt case, `env_plan.repo_root` now points at the launch repo, while `request.prepared.source_repo_root` still points at the prompt repo. Some later call sites compensate with `source_repo_root.or(env_plan.repo_root.as_deref())`, but anything consuming `env_plan.repo_root`, env-derived repo metadata, or the precomputed launch workspace can observe the wrong repo. This is a behavioral regression against the spec's "zero regressions" requirement and the existing env comment at `claudine/cli/src/commands/wrap/env.rs:350-354`.

Recommendation: make the precomputed `LaunchWorkspaceContext` preserve the old split contract: `repo_root = source_repo_root.or(launch_repo_root)` for metadata, `child_cwd = launch_repo_root.unwrap_or(launch_cwd)` for process launch. Add an integration test with launch CWD in repo A and composed markdown in repo B that verifies guardrails/MCP/harness repo resolution keys off repo B and the child process still launches in repo A.

### High: `--perf` section totals are wrong and can underreport or double count

`render_perf_report` computes section totals by parsing already-formatted row strings (`claudine/cli/src/perf.rs:441-455`). This is brittle and currently broken for microsecond rows: `parse_fmt_duration` checks `strip_suffix('µ')` before `strip_suffix('s')`, so a value like `8µs` is not parsed and contributes zero. The checked-in `trace-after-w0.md` shows the result: Composition Report has non-zero microsecond rows but renders `TOTAL: 0µs`.

The CLI Overhead total is also semantically wrong. The renderer sums `pre-dispatch`, `arg parsing`, `config loading`, `tracing init`, `environment setup`, and each substage (`claudine/cli/src/perf.rs:555-565`). Those rows overlap: `pre-dispatch` already includes the Phase A sub-buckets, and `environment setup` already includes the substages. The trace shows `elapsed 419.7ms` but `CLI Overhead TOTAL: 1.38s`, which is not a usable diagnostic total.

Recommendation: totals should come from raw duration fields, not parsed display strings. For CLI, render either a fixed total that does not double count, or omit a total and label the rows as overlapping diagnostics. For Composition Report, use `compose.total` directly rather than summing formatted metrics.

### High: Receipt banner behavior is not automatically verified at the required level

W1 requires a user-visible receipt banner immediately after dispatch, before the execution header, with `--silent` suppression and quiet-mode behavior matching the existing header. The implementation emits the banner from `run_compose_inner` / `run_inline_compose_inner` (`claudine/cli/src/commands/compose.rs:285-291`, `:560-566`), but I found no integration test asserting stderr order or suppression behavior.

Verification level present: none for the new receipt banner. Required level: Level 1 is sufficient for ordering and suppression because this is stderr text generated by the process, not terminal input encoding. The trace file is useful evidence, but it is not a regression test. Also note that the implementation suppresses under `--quiet` as well as `--silent`; the spec only says "Skip when `--silent`" and says `--quiet` should follow the existing header's detail rules, so this needs either correction or an explicit spec clarification.

Recommendation: add Level 1 CLI/PTY tests for compose and inline-compose asserting `→ Composing ...` appears before the full `Composition` header, is absent with `--silent`, and has the intended behavior with `--quiet`.

### Medium: W2 and W0 optimization claims are under-tested

The spec asks for a test proving W0 reduces launch workspace discovery to at most one scan and a W2 test proving `resolve_binary_path_direct` consults the snapshot without calling `which`. The implementation threads `installed_snapshot` and `prep_launch_workspace`, and the post-W0 trace strongly suggests the hot path is faster, but I did not find automated tests that would fail if a future change reintroduced either redundant scan.

Verification level present: mostly unit tests around rendering and existing prep helpers, plus a manual trace. Required level: Level 1 instrumentation/unit tests are sufficient for these non-terminal behavior requirements.

Recommendation: add a test-only counter or injectable resolver for launch workspace discovery, and a binary-path resolver test using a snapshot path for a fake provider profile. These would directly protect the performance regression this feature is meant to eliminate.

### Medium: Perf report formatting lacks a precise snapshot regression test

W9 explicitly called for a snapshot-style fixture covering all sections and a long label. Current tests in `claudine/cli/src/perf.rs` mostly assert substrings, which did not catch the broken `TOTAL: 0µs` and inflated `CLI Overhead TOTAL` behavior.

Verification level present: Level 1 substring unit tests. Required level: Level 1 snapshot or equivalent exact text assertion is appropriate. Level 2 is not required unless we want to verify actual terminal glyph width/rendering through a real emulator.

Recommendation: add an exact render fixture with microsecond rows, long labels, substages, dry-run agent omission, and an expected total for each section.

## Test Rigor Matrix

| Requirement | Strongest verification observed | Required | Status |
|---|---:|---:|---|
| W0 avoids redundant launch-workspace scans | Manual `trace-after-w0.md`; no automated counter | Level 1 | Gap |
| W0 preserves source repo metadata and launch CWD behavior | Existing helper tests only cover pieces | Level 1 | Gap |
| W1 receipt banner appears before header | Manual trace only | Level 1 | Gap |
| W1 `--silent` / `--quiet` behavior | No targeted test found | Level 1 | Gap |
| W2 binary path snapshot avoids `which` | No targeted test found | Level 1 | Gap |
| W6 terminal memoization | Unit test checks repeated properties | Level 1 | OK, but limited |
| W8 substage rows render | Unit substring test | Level 1 | Partial |
| W9 totals/alignment/long labels | Unit substring test | Level 1 exact/snapshot | Gap |

## Verification Run

- `cargo check --manifest-path claudine/cli/Cargo.toml --no-default-features` passed.
- `cargo test --manifest-path claudine/cli/Cargo.toml perf --no-default-features` passed.

## Production Readiness

Not ready. The main performance direction is sound, and the post-W0 trace shows the intended hot-path improvement, but the source-repo metadata regression and misleading perf totals are production blockers. The receipt banner and optimization paths also need Level 1 automated coverage before this can be called ready.
