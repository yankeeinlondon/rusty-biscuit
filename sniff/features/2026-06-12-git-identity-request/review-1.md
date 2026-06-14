---
ready: false
agent: codex
model: ""
---

# Review: Git Identity Request

## Findings

### High: Existing GitInfo JSON shape changes for non-identity requests

The spec says existing presets and CLI commands must keep their current JSON
shapes, with the exception that identity-only results omit `status`. The
implementation adds `head_id` to `GitInfo` and populates it for every
non-identity request in `GitRepo::detect_with_request`
(`sniff/lib/src/filesystem/git/types.rs:956-962`). Because the field is only
`skip_serializing_if = "Option::is_none"` (`types.rs:1009-1011`), normal
`summary`/`full`/`deep` results now serialize an extra top-level `head_id`.

This changes existing outputs such as `sniff repo git-status --json`, even
though the feature was scoped to preserve current command JSON shapes. The
identity path does need HEAD id (`types.rs:800-808`), but that does not require
adding the field to every existing status-bearing payload. Either keep
`head_id` unset outside identity mode, add a dedicated identity response shape,
or explicitly update the spec/tests if the JSON schema expansion is intended.

Verification level: Level 1 JSON/API tests are appropriate here, but no test
asserts that existing non-identity JSON has not gained new fields.

### High: The status-walk proof test is not isolated under `cargo test`

The status-walk counter is a process-global static
(`sniff/lib/src/filesystem/git/status.rs:20-26`) and
`identity_request_does_not_walk_status` resets it before asserting zero
(`sniff/lib/src/filesystem/git/types.rs:1512-1515`). Running the identity tests
with the standard Rust runner fails because other matching unit tests in the
same binary run concurrently and increment the shared counter.

Observed failure:

```text
cargo test --color=never -p sniff identity_request --lib
identity_request_does_not_walk_status ... FAILED
left: 1
right: 0
```

`cargo nextest run -p sniff identity_request` passes because nextest isolates
tests into separate processes, but the test itself is still fragile outside
that runner. The project supports canonical nextest recipes, but a feature proof
for "identity never invokes status" should not be invalidated by ordinary unit
test concurrency. Make the test serial, isolate the counter per test/thread, or
structure the assertion around a test-specific instrumentation guard.

Verification level: Level 1 is the correct level for this requirement. The
current Level 1 proof exists, but it is not robust across supported Rust test
execution modes.

### Medium: Status-oriented render helpers panic on valid identity-only data

Several output helpers now unwrap `GitInfo.status` with `expect(...)`, for
example `render_git_section` (`sniff/cli/src/output/filesystem/mod.rs:866-870`),
package-area dirty/change helpers (`mod.rs:1188-1191`, `mod.rs:1266-1269`),
`render_filesystem_section` (`sniff/cli/src/output/filesystem/repo.rs:706-709`),
and `dirty_package_names`
(`sniff/cli/src/output/filesystem/packages.rs:142-145`).

The spec allows status-oriented paths to reject identity-only data, but asks for
an explicit internal error rather than silently treating it as clean. A panic is
not an ergonomic error boundary for CLI rendering helpers, especially now that
`GitInfo { status: None }` is a valid library state. Prefer returning a
`Result`, skipping only clearly non-status sections, or routing identity data to
a dedicated renderer before these helpers are called.

Verification level: Level 1 unit tests are sufficient. There is no test that
passes an identity-only `GitInfo` through the public render/selection helpers
and verifies a clear non-panic outcome.

## Requirement Coverage

- `GitRequest::identity()` exists and is checked before status collection:
  implemented at `sniff/lib/src/request.rs` and
  `sniff/lib/src/filesystem/git/types.rs:828-831`.
- `GitInfo.status` is optional and identity JSON omits it:
  implemented at `types.rs:1021-1026` with Level 1 serialization coverage.
- Existing presets still compute `status`: covered by Level 1 tests.
- Plan-level identity plus repo structure is expressible and skips status:
  covered by Level 1 test
  `identity_plan_does_not_walk_status_end_to_end`.
- Main worktree, linked worktree, detached HEAD, and unborn HEAD identity cases:
  covered by Level 1 unit tests.
- Existing `sniff repo git-status --json` keeps a top-level `status`: covered by
  existing Level 1 CLI tests, but the tests do not check the full shape for new
  unexpected fields.
- L2/L3: not required for this feature as implemented. The user-observable
  behavior is API/JSON/CLI data shape, not terminal encoder behavior, keyboard
  input, or real-terminal styling.

## Verification Performed

- `cargo check --color=never -p sniff-cli --tests` passed.
- `cargo nextest run --color=never -p sniff identity_request` passed.
- `cargo test --color=never -p sniff identity_request --lib` failed as described
  above.
- `cargo test --color=never -p sniff --lib` built and ran, but failed on
  `filesystem::git::worktree::tests::list_worktrees_main_only` because the local
  git default branch was `master` while the assertion expected `main`; this
  appears unrelated to the identity feature.
