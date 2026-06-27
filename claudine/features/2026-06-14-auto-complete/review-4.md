---
ready: false
agent: "codex/default"
created: "2026-06-26T16:13:34"
implemented: true
---

# Review 4 - Auto Complete

Not ready for production. Review 4 confirms that the prior operation-file YAML sequence coverage gap has been addressed with Level 2 real-terminal tests and Level 3 OS-keyboard tests. The remaining blockers are narrower: two observable failure modes are still not verified through the CLI path, and the performance harness can report success-compatible output while missing the stated p95 target.

## Findings

### High: no-match and over-cap autocomplete failures are not verified through the CLI

Requirement: the ENTER autocomplete path has three observable failure modes: no matches -> error, over `MAX_CANDIDATES` -> visible "narrow your query" error, and non-TTY -> error ([spec.md](spec.md:178), [spec.md](spec.md:207)).

Coverage present: non-TTY has a subprocess test in [wrap_compose_validation.rs](../../cli/tests/wrap_compose_validation.rs:346). The no-match and over-cap paths are only exercised below the user-facing route: private `gather_candidates` unit tests in [operation_file.rs](../../cli/src/completion/operation_file.rs:294) and [operation_file.rs](../../cli/src/completion/operation_file.rs:337), plus error rendering unit tests in [error.rs](../../lib/src/composition/error.rs:3258) and [error.rs](../../lib/src/composition/error.rs:3303).

Why this is a gap: the spec calls these failures observable. Unit tests prove the helper and status block can produce the right variants, but they do not prove `claudine compose <partial>` routes from a failed `FileReference` lookup into autocomplete, preserves the variant, renders the expected stderr text, and exits cleanly without launching the provider. A regression in the compose/sequence CLI plumbing could leave the current tests green.

Fix direction: add Level 1 CLI/PTY tests for interactive no-match and over-cap. They can run under `expectrl` or the existing PTY helper, stage an empty/no-match prompt scope and a 501-match prompt scope, invoke `claudine compose <query>`, and assert stderr contains the no-match or narrow-query message with a non-zero exit and no provider launch. Level 2/3 is not required here because these are immediate error surfaces, not terminal layout or key-encoder behavior.

### High: the latency assertion allows p95 values above the stated target

Requirement: the acceptance criteria say the autocomplete latency assertion reuses `completion_perf.rs` and confirms p95 stays within the same ~100 ms-class budget as completion ([spec.md](spec.md:206)).

Implementation: `TARGET_P95_MS` is 100, but `assert_perf_target` returns successfully for `100 ms < p95 <= 150 ms` and only prints a warning ([completion_perf.rs](../../cli/tests/completion_perf.rs:290), [completion_perf.rs](../../cli/tests/completion_perf.rs:308)). The ENTER-path performance test uses the same helper ([completion_perf.rs](../../cli/tests/completion_perf.rs:429)).

Impact: an explicit acceptance criterion can be missed without failing the test harness. A run with p95 at 149 ms would not confirm the ~100 ms budget, yet the test would pass if someone ran the ignored performance suite in CI or before release.

Fix direction: make the assertion fail when `stats.p95 > TARGET_P95_MS`. If the team wants a 150 ms cache-trigger diagnostic, keep it as an additional message after the hard target failure, not as a passing warning range.

### Low: completion modules still carry stale scaffolding comments

Several production files still say the code is "scaffolding" or "not wired yet" even though it is now used by the runtime autocomplete path: [walker.rs](../../cli/src/completion/walker.rs:91), [walker.rs](../../cli/src/completion/walker.rs:128), [default_glob.rs](../../cli/src/completion/default_glob.rs:19), and [autocomplete_ui.rs](../../cli/src/completion/autocomplete_ui.rs:15).

Per the repo comment-quality rules, I treated the code as correct and the comments as drifted. This is not a functional blocker by itself, but it should be cleaned up with the next implementation pass so future reviewers do not misread integrated production code as unused feature staging.

## Verification Matrix

- Shared bounded walker, query-filtered cap counting, mode contract filtering: Level 1 unit coverage present.
- Dynamic completion contract, bootstrap `claudine completions <shell>`, `@` sigil stripping, bare `file`/`file[]` fallback, comma continuation: Level 1 subprocess/unit coverage present.
- Operation-file single-match confirmation and multi-match chooser for Markdown compose: Level 2 rendering and Level 3 OS-keyboard coverage present.
- Operation-file YAML sequence confirmation/chooser/detail: Level 2 rendering and Level 3 OS-keyboard coverage present.
- Missing `file` and `file[]` frontmatter property chooser behavior: Level 2 rendering and Level 3 OS-keyboard coverage present.
- Autocomplete failure modes: non-TTY has Level 1 subprocess coverage; no-match and over-cap are only unit-level helper/render coverage and need CLI-path Level 1 verification.
- Latency: performance harness exists, but the assertion does not enforce the documented 100 ms p95 target.

## Notes

I did not run the full test suite for this review. The assessment is based on source and test inspection, with special attention to the required Level 1/2/3 verification mapping.
