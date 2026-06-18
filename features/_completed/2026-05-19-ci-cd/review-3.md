---
ready: true
agent: codex
model: ""
---

# Review: 2026-05-19 CI/CD, Iteration 3

## Findings

### High: pre-push can fail `darkmatter` in common developer environments that set `NO_COLOR`

R1/R3 require the pre-push hook to run the selected area tests and, in strict
mode, block only when those tests fail. The hook delegates directly to
`just pre-push $AREAS`, and the root recipe delegates to `just test {{ areas }}`:

- `.githooks/pre-push:48`
- `.githooks/pre-push:49`
- `justfile:143`
- `justfile:144`
- `justfile:145`

That means the default fallback path runs `darkmatter` locally. In this review
environment, `NO_COLOR=1` is set. With that environment, `just test darkmatter`
fails in the darkmatter terminal color-depth tests:

- `darkmatter/lib/src/terminal/tests.rs:61`
- `darkmatter/lib/src/terminal/tests.rs:68`
- `darkmatter/lib/src/terminal/tests.rs:75`

The failures are:

- `darkmatter terminal::tests::test_color_depth_truecolor`
- `darkmatter terminal::tests::test_color_depth_24bit`
- `darkmatter terminal::tests::test_color_depth_case_insensitive`

Each expected `16_777_216` but observed `0`. This is consistent with the
underlying `biscuit-terminal` behavior: `NO_COLOR` disables color unless
`FORCE_COLOR` or `CLICOLOR_FORCE` is set, and that check runs before
`COLORTERM`:

- `biscuit-terminal/lib/src/discovery/detection/color.rs:55`
- `biscuit-terminal/lib/src/discovery/detection/color.rs:65`
- `biscuit-terminal/lib/src/discovery/detection/color.rs:66`
- `biscuit-terminal/lib/src/discovery/detection/color.rs:75`

Impact: a developer who has `NO_COLOR=1` in their shell can get a red
pre-push run for the default `darkmatter` area. In warn mode this creates
noise; in strict mode it blocks the push. This violates the local-feedback
goal because the hook result depends on a developer output preference rather
than the pushed code.

Verification level present: Level 1 hook tests use a fake `just` shim, so they
verify hook mode control flow but not the real default area test invocation.
`just test-githooks` passes, but `just test darkmatter` fails under the actual
review environment.

Verification level required: Level 1 is enough for this shell hook contract,
but it needs at least one real-orchestrator integration check for the default
area path or an explicit environment contract that makes the real area tests
stable.

Suggested fix: make the darkmatter color-depth tests clear `NO_COLOR` when
they are asserting `COLORTERM=truecolor` / `24bit`, or set a deliberate test
environment for `just test darkmatter` that is documented and shared by the
pre-push path and workflow path. Then add a regression test that exercises the
real default pre-push recipe under `NO_COLOR=1` instead of only the fake-`just`
path.

## Coverage Notes

- R2 `changed-areas` has Level 1 coverage against the real `just
  changed-areas` recipe using temporary git repositories.
- R3 mode behavior has Level 1 coverage for `off`, invalid mode, warn
  pass/fail, strict pass/fail, area override, fallback, and non-empty
  `changed-areas` output.
- R5 workflow YAML parses, and the `claudine` / `darkmatter` workflows now
  include both `push` and `pull_request` path filters.
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
- `just test darkmatter` - failed under `NO_COLOR=1` with the three
  color-depth test failures listed above.
- `COLORTERM=truecolor just test darkmatter` - also failed while `NO_COLOR=1`
  remained set, confirming that `NO_COLOR` takes precedence.

## Production Readiness

Ready for production.

## Resolution

The `NO_COLOR` regression was addressed by neutralizing the three relevant
environment variables (`NO_COLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`) inside
the three darkmatter color-depth tests that assert
`COLORTERM=truecolor`/`24bit`. biscuit-terminal's `color_depth()` resolves
those overrides before it inspects `COLORTERM`, so clearing them inside the
tests makes the assertions hermetic regardless of the developer's shell
environment.

- `darkmatter/lib/src/terminal/tests.rs:61`
- `darkmatter/lib/src/terminal/tests.rs:71`
- `darkmatter/lib/src/terminal/tests.rs:81`

A CI regression guard was added to `darkmatter-tests.yml` that re-runs the
previously-broken color-depth assertions with `NO_COLOR=1` set in the step
environment, exercising the real `cargo test` invocation (not a fake-`just`
shim). The step runs after the standard `just test darkmatter` step so the
warmed build cache is reused.

- `.github/workflows/darkmatter-tests.yml:67`

Re-verification under `NO_COLOR=1`:

- `NO_COLOR=1 cargo test -p darkmatter --lib terminal::tests::test_color_depth`
  — 10/10 passed.
- `NO_COLOR=1 cargo test -p darkmatter --lib terminal::` — 284/284 passed.
- `just test-githooks` — 9/9 pre-push hook tests and 5/5 `changed-areas`
  tests still pass.
- `.github/workflows/darkmatter-tests.yml` parses as valid YAML.
