---
ready: false
agent: codex
model: ""
---

# Review: 2026-05-19 CI/CD

## Findings

### High: the new area workflows are active but currently fail in GitHub Actions

R5 and the Definition of Done require the new `claudine-tests.yml` and
`darkmatter-tests.yml` workflows to run the corresponding area tests and pass
on GitHub. The workflow shape is correct locally: both have `push` and
`pull_request` path filters, the Node 24 environment workaround, and a final
`just test <area>` step.

- `.github/workflows/claudine-tests.yml:7`
- `.github/workflows/claudine-tests.yml:27`
- `.github/workflows/claudine-tests.yml:54`
- `.github/workflows/darkmatter-tests.yml:7`
- `.github/workflows/darkmatter-tests.yml:27`
- `.github/workflows/darkmatter-tests.yml:54`

The authoritative GitHub runs are failing, though:

- `claudine-tests` run #4 on `364cb5b` failed in `Run Claudine tests` with
  exit code 127:
  https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/26416836673/job/77763011364
- `darkmatter-tests` run #5 on `364cb5b` failed in `Run Darkmatter tests` with
  exit code 127:
  https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/26416836646/job/77763011372

Verification level present: GitHub Actions did execute the workflows, but the
area test step failed.

Verification level required: passing GitHub Actions runs for both workflows,
because CI is explicitly the authoritative cross-platform gate for this
feature.

Impact: this does not satisfy the primary CI/CD goal of restoring trustworthy
CI signals. The Node 20 deprecation is now reduced to a warning under the
`FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` workaround, but the new required area
workflows still produce red CI.

Suggested fix: inspect the failed `just test claudine` and `just test
darkmatter` job logs with authenticated GitHub access, reproduce the missing
command or environment issue locally in an Ubuntu-like environment, and add any
missing runner setup before marking the feature ready.

## Coverage Notes

- R2 `changed-areas` now has Level 1 coverage against the real `just
  changed-areas` recipe using temporary Git repositories. The tests cover no
  upstream, a single mapped area, multiple mapped areas in curated order,
  unmapped paths, and mixed mapped/unmapped paths.
- R3 hook mode behavior has Level 1 coverage in `.githooks/tests/test-pre-push.sh`
  for `off`, invalid mode, warn pass/fail, strict pass/fail, area override,
  fallback, and forwarding non-empty `changed-areas` output.
- The red warning behavior is byte-level Level 1 coverage. I do not think
  Level 2 is required here because this is a shell hook output contract, not a
  terminal-emulator rendering requirement.
- R5 path filters are now present for both `push` and `pull_request` in the
  `claudine` and `darkmatter` workflows.
- R6 is statically satisfied across current workflow files: every
  `.github/workflows/*.yml` file contains
  `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24`.

## Verification Performed

- `just test-githooks` - passed: 9/9 pre-push hook tests and 5/5
  `changed-areas` tests.
- Parsed all `.github/workflows/*.yml` files with Ruby `YAML.load_file` -
  passed.
- Checked all workflow files for `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` - no
  omissions found.
- Queried the public GitHub Actions API for recent runs - latest
  `claudine-tests` and `darkmatter-tests` runs are completed failures.
- `shellcheck .githooks/pre-push .githooks/tests/test-pre-push.sh
  .githooks/tests/test-changed-areas.sh` - reported only test-harness warnings
  about indirect function invocation and `cd` guards; no production hook issue
  found.

## Production Readiness

Not ready for production.

The local hook behavior and the `changed-areas` regression coverage are now in
good shape. The remaining blocker is the authoritative CI gate: both new area
workflows currently fail in GitHub Actions, so the feature does not yet meet
the Definition of Done.
