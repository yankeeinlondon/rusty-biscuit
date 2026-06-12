---
ready: false
agent: codex
model: ""
---

# Review: 2026-05-19 CI/CD

## Findings

### High: `changed-areas` heuristic is not directly covered by Level 1 tests

R2 is now explicitly scoped as a partial, top-level-directory heuristic, which is a reasonable deferral from full dependency-aware mapping. The current behavior still has user-observable consequences: it decides which area tests run before a push. I found Level 1 tests proving that `.githooks/pre-push` falls back when `just changed-areas` returns empty and forwards non-empty output, but those tests use a fake `just` shim and never execute the real `changed-areas` recipe.

- `justfile:119`
- `justfile:122`
- `justfile:127`
- `.githooks/tests/test-pre-push.sh:62`
- `.githooks/tests/test-pre-push.sh:255`
- `.githooks/tests/test-pre-push.sh:265`

Verification level present: Level 1 tests for hook integration with a stubbed `changed-areas` result.

Verification level required: Level 1 tests for the real heuristic using a temporary Git repository, configured upstream branch, and changed files such as `claudine/src/...`, `darkmatter/...`, and an unmapped top-level path.

Impact: a regression in the actual `git rev-parse @{u}` / `git diff "$upstream..HEAD"` / top-level segment matching logic can pass the checked-in hook tests while changing which test suites developers run before push.

Suggested fix: add a focused Level 1 test script or Rust integration test for `just changed-areas`. Cover no-upstream empty output, mapped area output, multiple mapped areas, and unmapped paths returning empty so the hook fallback remains meaningful.

### Medium: New GitHub workflows still need real GitHub Actions validation before production sign-off

The workflow structure now matches R5: both new area workflows have `push` and `pull_request` path filters, include shared orchestration paths, and run the expected `just test <area>` commands. Static YAML parsing also succeeds locally. However, the Definition of Done requires the new `claudine-tests.yml` and `darkmatter-tests.yml` workflows to pass on PR, and there is no in-repo evidence that those GitHub Actions jobs have run successfully with these changes.

- `.github/workflows/claudine-tests.yml:7`
- `.github/workflows/claudine-tests.yml:54`
- `.github/workflows/darkmatter-tests.yml:7`
- `.github/workflows/darkmatter-tests.yml:54`

Verification level present: static review and local YAML parse.

Verification level required: an actual GitHub Actions run for a PR or branch that exercises each new workflow.

Impact: runner dependencies, action compatibility, cache setup, `just` installation, and the Node 24 workaround are not fully proven until GitHub executes the workflows.

Suggested fix: push this implementation to a PR that touches `claudine/**` and `darkmatter/**`, then record the passing workflow runs before marking the feature production-ready.

## Coverage Notes

- R3 hook mode behavior has Level 1 coverage in `.githooks/tests/test-pre-push.sh` for `off`, invalid mode, warn pass/fail, strict pass/fail, area override, fallback, and changed-area forwarding.
- The red failure requirement is covered at Level 1 byte level: tests assert the literal `\033[31mPre-push tests failed` prefix and `\033[0m` reset. I do not think Level 2 is necessary here because this is a shell hook byte-output contract, not a TUI rendering requirement.
- The hook test workflow is now wired into CI through `.github/workflows/hooks-tests.yml`, with path filters for `.githooks/**`, `justfile`, and `just/**`.
- R5 path-filtered area workflows now include both `push` and `pull_request` triggers for `claudine/**` and `darkmatter/**`, plus `Cargo.toml`, `Cargo.lock`, `justfile`, and `just/**`.
- R6 is statically satisfied across current workflow files: every `.github/workflows/*.yml` file contains `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24`.

## Verification Performed

- `just test-pre-push-hook` — passed, 9/9 tests.
- Parsed all `.github/workflows/*.yml` files with Ruby `YAML.load_file` — passed.
- Checked all workflow files for `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` — no omissions found.

## Production Readiness

Not ready for production.

The implementation is much closer than the prior review: the main workflow trigger gaps and hook test wiring gaps are addressed. I would hold production readiness until the real `just changed-areas` heuristic has Level 1 coverage and the new GitHub Actions workflows have passed in GitHub.
