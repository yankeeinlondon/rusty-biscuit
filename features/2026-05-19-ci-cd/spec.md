## Broken Actions

Currently we've not been leveraging CI-CD but that is going to change soon. One immediate problem is that every push to Github results in a failed action because of this error:

```sh
Node.js 20 actions are deprecated. The following actions are running on Node.js 20 and may not work as expected: actions/checkout@v4. Actions will be forced to run with Node.js 24 by default starting June 2nd, 2026. Node.js 20 will be removed from the runner on September 16th, 2026. Please check if updated versions of these actions are available that support Node.js 24. To opt into Node.js 24 now, set the FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true environment variable on the runner or in your workflow file. Once Node.js 24 becomes the default, you can temporarily opt out by setting ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION=true. For more information see: https://github.blog/changelog/2025-09-19-deprecation-of-node-20-on-github-actions-runners/
```

This needs to be fixed right away.

### Problem Statement

The monorepo has existing GitHub Actions workflows that are currently broken due to the Node.js 20 deprecation. Every push triggers a failed action, creating noise and eroding trust in CI signals. This must be resolved before any new CI/CD investment can be considered reliable.

### Goals

1. Eliminate deprecation failures on every push so CI status is trustworthy.
2. Provide fast local feedback to developers before they push.
3. Keep CI as the authoritative cross-platform gate.

### Non-Goals

- Replacing CI with local hooks.
- Running the full workspace test suite in the pre-push hook.
- Blocking developers who intentionally bypass checks.

## Leveraging Local

Testing for this monorepo is quite laborious so I want to start by leveraging as much local testing as possible so what I'd like to do is:

1. I want to create a hook that runs prior to pushing commits that runs all tests in the "claudine" and "darkmatter" areas.

### Requirements

#### R1. Pre-Push Hook — Coverage

The hook shall run tests for a hardcoded list of workspace areas. The initial list is:

- `claudine`
- `darkmatter`

The hook script must be structured so that adding a new area is a one-line change (e.g., an array or list variable at the top of the script).

#### R2. Pre-Push Hook — Change Detection (Future)

Within one sprint after initial hook deployment, the hook shall migrate from the hardcoded list to dynamic change detection. It shall:

- Inspect the commits being pushed.
- Map changed files to workspace members via `Cargo.toml` path dependencies.
- Run tests only for the areas that contain modified files.

This gives fast feedback for small changes without punishing developers who touch documentation or unrelated crates.

#### R3. Pre-Push Hook — Enforcement Model

The hook shall read an environment variable or local config file:

- `RUSTY_BISCUIT_PRE_PUSH=strict` — runs tests; blocks the push (exit non-zero) if any test fails.
- `RUSTY_BISCUIT_PRE_PUSH=warn` (default for new clones) — runs tests; prints failures prominently in red; exits zero so the push proceeds.
- `RUSTY_BISCUIT_PRE_PUSH=off` — skips the hook entirely.

Failure messages in `warn` mode shall include a reminder that `--no-verify` can bypass the hook if the developer chooses to proceed.

#### R4. Pre-Push Hook — Cross-Platform Execution

The hook shall be a thin wrapper that delegates to `just` as the orchestrator. This avoids maintaining parallel POSIX shell and PowerShell implementations.

- The hook itself can be a POSIX shell script (sufficient for macOS and Linux developers).
- The actual test invocation logic lives in the root `justfile` (e.g., a `pre-push` recipe).
- Windows developers can run `just pre-push` manually or create an equivalent local wrapper.

#### R5. GitHub Actions CI — Path-Filtered Workflows

CI shall use path filters to run relevant tests on PR and push, consistent with existing patterns in workflows such as `messenger-desktop-tests.yml` and `sniff-performance.yml`.

New workflows needed:

- `.github/workflows/claudine-tests.yml`
- `.github/workflows/darkmatter-tests.yml`

Each workflow runs the corresponding area's tests when files under that area's path are modified.

#### R6. GitHub Actions CI — Node.js 20 Deprecation Fix

**Immediate:** Add `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24=true` to the environment of all existing workflows (or set it at the repository level via GitHub settings) to stop the deprecation failures.

**Follow-up:** Once GitHub publishes `actions/checkout@v5` (or a pinned Node-24-compatible SHA), migrate all workflows from the environment-variable workaround to the updated action version.

### Implementation Approach

1. **Fix Node.js 20 deprecation** — apply the environment variable to all workflow files in `.github/workflows/`.
2. **Add just recipe** — create a `pre-push` recipe in the root `justfile` that accepts an optional area list and runs `just test` for each.
3. **Create hook script** — add a `pre-push` script under a new `.githooks/` directory (or similar) that:
   - Sources developer config (environment variable or file).
   - Resolves the list of areas (hardcoded initially).
   - Calls `just pre-push <areas...>`.
   - Exits according to the `strict` / `warn` / `off` setting.
4. **Document setup** — add a "Local Development" section to the README explaining how to enable the hook and configure the enforcement mode.
5. **Add CI workflows** — create `claudine-tests.yml` and `darkmatter-tests.yml` with path filters and the same `just test` invocation used locally.
6. **Future sprint** — replace the hardcoded area list with file-change detection using `git diff --name-only` and `cargo metadata --no-deps --format-version 1` to map files to workspace members.

### Acceptance Criteria

- [ ] No push to `main` or PR triggers a Node.js 20 deprecation failure.
- [ ] A developer with `RUSTY_BISCUIT_PRE_PUSH=warn` sees test output and failures in red, but can still push.
- [ ] A developer with `RUSTY_BISCUIT_PRE_PUSH=strict` cannot push if claudine or darkmatter tests fail.
- [ ] A developer with `RUSTY_BISCUIT_PRE_PUSH=off` pushes without any local test run.
- [ ] Adding a third area to the hook requires editing exactly one line in the hook script.
- [ ] CI runs claudine tests when `claudine/**` files change.
- [ ] CI runs darkmatter tests when `darkmatter/**` files change.
- [ ] CI remains the source of truth: a green hook does not imply a green CI run.

### Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Hook adds noticeable latency to every push | Medium | Start with two small areas; migrate to change detection within one sprint. |
| Developers disable the hook because it is too noisy | Medium | Default to `warn` mode so failures are visible but non-blocking. |
| Windows developers cannot use the POSIX hook script | Low | Document that they can run `just pre-push` manually; the script is a thin wrapper around `just`. |
| CI and local test commands diverge | Medium | Both invoke the same `just` recipe; no separate shell logic in CI. |
| Change detection is complex to implement correctly | Medium | Time-box the follow-up sprint; fall back to the hardcoded list if detection is not ready. |

### Definition of Done

- All existing workflows pass without deprecation warnings.
- New `claudine-tests.yml` and `darkmatter-tests.yml` workflows exist and pass on PR.
- A developer can clone the repo, set `RUSTY_BISCUIT_PRE_PUSH=warn`, install the hook, and get fast feedback before pushing.
- The spec is updated to reflect any deviations discovered during implementation.
