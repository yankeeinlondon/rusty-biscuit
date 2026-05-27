---
ready: true
agent: codex
model: ""
---

# Review: 2026-05-19 CI/CD

## Findings

### High: New area test workflows do not run on pull requests

Requirement R5 says CI shall use path filters to run relevant tests on **PR and push**, and the Definition of Done says the new `claudine-tests.yml` and `darkmatter-tests.yml` workflows must pass on PR. The implemented workflows only define `on.push.paths` plus `workflow_dispatch`; there is no `pull_request` trigger.

- `.github/workflows/claudine-tests.yml:7`
- `.github/workflows/darkmatter-tests.yml:7`

Impact: changes to `claudine/**` or `darkmatter/**` in a PR will not run these path-filtered area gates. That weakens the stated CI source-of-truth model and fails the PR half of R5.

Verification level: workflow routing is not a terminal UX requirement. Static review shows the trigger is missing; production readiness should also be validated by opening or simulating a PR touching each path.

Suggested fix: add matching `pull_request.paths` blocks to both workflows, keeping the same paths as `push`.

### High: Pre-push hook behavior is not covered by checked-in Level 1 tests

R3 defines user-observable hook behavior for `warn`, `strict`, `off`, invalid modes, failure messaging, red failure output, and bypass guidance. I found no checked-in tests for `.githooks/pre-push`, `just pre-push`, or `just changed-areas` behavior.

- `.githooks/pre-push:27`
- `.githooks/pre-push:49`
- `justfile:119`
- `justfile:138`

Manual smoke check with a fake failing `just` confirms `warn` currently exits zero, but that is not durable coverage. These requirements are appropriate for Level 1 script tests: execute the hook with a temp `PATH` containing a fake `just`, set `RUSTY_BISCUIT_PRE_PUSH` / `RUSTY_BISCUIT_PRE_PUSH_AREAS`, and assert exit code plus stderr/stdout snippets.

Verification level present: ad hoc Level 1 manual smoke only, not committed.

Verification level required: Level 1 checked-in tests.

Suggested fix: add a small shell-test harness or Rust integration test that exercises `off`, `warn` failure, `strict` failure, invalid mode, area override, and fallback behavior. Include an assertion that warn-mode failure text mentions `--no-verify`, because the spec requires that reminder.

### Medium: Dynamic change detection does not meet the specified dependency-mapping design

R2 says the hook shall inspect the commits being pushed, map changed files to workspace members via `Cargo.toml` path dependencies, and run tests only for affected areas. The implementation uses the configured upstream branch and maps only the first path segment against the curated area list.

- `justfile:122`
- `justfile:127`
- `justfile:129`

Impact: first pushes with no upstream fall back to `claudine darkmatter`; dependency changes outside those directories are not propagated through `Cargo.toml` path dependencies; and the hook ignores the remote ref/sha pairs that Git passes to pre-push on stdin. This is better than a static-only hook, but it is not the dependency-aware behavior the spec describes.

Verification level present: none found.

Verification level required: Level 1 tests with temporary Git repositories and synthetic changed files are sufficient.

Suggested fix: either mark R2 as deferred in the spec/README or implement a proper changed-file-to-workspace-member resolver from `cargo metadata --no-deps --format-version 1`, using the pre-push stdin refs as the source of the pushed commit ranges.

### Medium: README overstates dynamic detection as complete

The README says the hook dynamically detects changed monorepo areas, but the implementation falls back to `claudine darkmatter` whenever there is no upstream and does not perform dependency mapping.

- `README.md:126`
- `justfile:122`
- `justfile:127`

Impact: users may assume the hook is precise when it is still a coarse top-level-directory detector with broad fallback behavior. This matters because one of the spec risks is hook latency and developer trust.

Suggested fix: either update the docs to describe the current top-level-directory heuristic and fallback, or complete R2 and then document the dependency-aware behavior.

## Positive Coverage

- All current workflow files include `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24`, satisfying the immediate Node.js 20 deprecation workaround.
- `.githooks/pre-push` is executable in Git (`100755`).
- `warn` mode allows a failing fake `just` command to proceed in manual Level 1 smoke testing.
- The root `pre-push` recipe delegates to `just test`, keeping the local orchestration centralized.

## Production Readiness

Not ready for production.

The feature misses the PR trigger requirement for the new CI gates, lacks committed Level 1 verification for the hook’s user-observable behavior, and only partially implements the dynamic change-detection requirement.
