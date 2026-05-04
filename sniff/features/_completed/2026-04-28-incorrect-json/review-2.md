---
ready: false
agent: ""
model: ""
---

# Feature Review #2: Correct `sniff repo --json` Output

Review of the implementation of the `incorrect-json` feature defined in
[spec.md](./spec.md) against the current state of the code on the `sniff`
branch. The previous review (`review-1.md`) found the implementation was
not yet started; this review confirms it has now been substantially
completed and identifies the remaining gaps.

## Summary

The bulk of the spec is implemented, well-factored, and well-tested.
A new `sniff/cli/src/output/repo_json.rs` module dispatches on
`RepoAction` to produce focused JSON shapes, and the unit and
integration tests pin the contract effectively (including a
distinctness matrix that proves no two repo subcommands collapse to the
same JSON payload). All 186 `sniff-cli` tests pass on this branch.

However the feature is **not yet ready for production**. One of the
spec's acceptance criteria — `--perf` continuing to work alongside the
new JSON shapes (criterion #9) — is broken for every repo subcommand
that takes the early-return path in `commands.rs`. There are also a
handful of smaller no-result / filter behaviours that don't yet honour
the "JSON consumers always see a stable shape" promise.

## What Is Implemented Correctly

- `output/repo_json.rs` dispatches on `RepoAction` and returns a
  `BuildOutcome { value, exit_code }` so locator and boolean families
  can both emit a stable shape and influence the process exit code.
- `git-status --json` returns the focused `GitInfo` directly
  (`repo_json.rs:319`); `RepoInfo` fields are no longer leaked.
- `deps --json` is hand-built with an explicit field allowlist
  (`name`, `depends_on`, `used_by`, `dependencies`, `dev_dependencies`,
  plus optional `peer_dependencies` / `optional_dependencies` only when
  populated). Forbidden fields like `path`, `relative`, `package_area`,
  `languages`, `documentation`, `configuration`, `package_managers`,
  `version`, `is_excluded` are explicitly asserted absent in
  `repo_json.rs:1101-1128`.
- Dirty / staged / unstaged package + package-area families all return
  `{ scope, kind, names }` with empty `names` for non-monorepo repos
  instead of the legacy prose error string.
- Locator family (`package-root`, `package-area-root`) returns
  `{ root: "..." }` and exits `1` when the path is empty.
- Boolean family (`is-current-package-area-dirty`,
  `package-area-has-source-code-changes`, `has-merge-conflict`) returns
  descriptive boolean keys and preserves exit-code semantics.
- `source-code-changes --json` and `documentation-changes --json`
  filter commits and per-commit files using
  `commit_blocks::filter_commit_set` and tag the payload with a
  top-level `"filter"` field. `recent-commits --json` deliberately
  remains unchanged and a regression test
  (`test_recent_commits_json_unchanged`) pins that.
- `test_repo_subcommand_json_shapes_are_distinct` cross-products 17
  subcommand JSON outputs and asserts no two are equal — an effective
  guardrail against future regressions.
- `--perf` for `git-status --json` and
  `is-current-package-area-dirty --json` correctly attaches a sibling
  `performance` field via `attach_performance`.

## Gaps in Functionality

### 1. `--perf` corrupts JSON for every early-return subcommand (BLOCKER)

The spec's acceptance criterion #9 explicitly requires `--perf` to keep
working alongside the new JSON shapes. Today this works for the
subcommands that route through `output::print_json` (which calls
`attach_performance`), but **every early-return JSON path emits a
human-readable Markdown `## Performance` block to stdout after the
JSON**, which corrupts the JSON payload for any consumer that pipes it
to `jq`, `serde_json`, or similar.

Reproduced from this worktree:

```
$ sniff repo root --json --perf
{
  "root": "/.../sniff/"
}

## Performance

Total: 2.84 ms
```

```
$ sniff repo recent-commits --json --perf
... (valid JSON) ...
}

## Performance

Total: 577.94 ms
```

```
$ sniff repo dirty-files --json --perf
... (valid JSON) ...
}

## Performance

Total: 65.38 ms
```

Affected JSON paths (all in `sniff/cli/src/commands.rs` unless noted):

| Path | Location | Behaviour today |
|------|----------|-----------------|
| `repo root --json` | `commands.rs:482-503` | `perf.emit_stdout(None)` after JSON |
| `repo hash --json` | `commands.rs:362-394` | `perf.emit_stdout(None)` after JSON |
| `repo pr --json` | `commands.rs:1107-1124` | `perf.emit_stdout(None)` after JSON |
| `repo remote --json` (URL & shorthand) | `commands.rs:324-347` + handlers | `perf.emit_stdout(None)` after JSON |
| `repo blast-radius --json` | `commands.rs:254-260` | `perf.emit_stdout(None)` after JSON |
| `repo dirty-files`, `staged-files`, `dirty-source-code`, `staged-source-code`, `unstaged-source-code` (all `--json`) | `commands.rs:1565-1572` (`handle_file_list_command`) | `perf.emit_stdout(None)` after JSON |
| `repo packages --json` | `commands.rs:1409-1414` | `perf.emit_stderr(None)` (safe but inconsistent) |
| `repo package-areas --json` | `commands.rs:1470-1474` | `perf.emit_stderr(None)` (safe but inconsistent) |
| `repo package --json`, `repo package-area --json` | `commands.rs:812-816, 832-836` | `perf.emit_stdout(result.performance.as_ref())` after JSON |
| `repo recent-commits/source-code-changes/documentation-changes --json` | `output/recent_commits.rs:93, 117` | `perf.emit_stdout(None)` after JSON |
| `repo unstaged-files/untracked-files --json` | `commands.rs:431-446` | `perf.emit_stderr(...)` (safe) |
| `repo has-merge-conflict --json` | `commands.rs:521-525` | `perf.emit_stdout(None)` after JSON, then `exit` |

The commands that do `emit_stderr` are safe (perf goes to stderr,
stdout stays valid JSON). The ones that do `emit_stdout` produce
syntactically invalid JSON output. The fix should mirror what
`commands.rs:892-897` already does for the `print_json` path:

```rust
if result.performance.is_some() {
    perf.emit_stderr(result.performance.as_ref());
} else {
    perf.emit_stdout(result.performance.as_ref());
}
```

…except in JSON mode the perf section should always go to stderr (or
be attached to the JSON via `attach_performance`) regardless of whether
`result.performance` is `Some`. A central helper would also avoid the
copy-paste between roughly a dozen call sites.

There is **no integration test** that catches this. `--perf` is
exercised only against the `print_json` path
(`test_git_status_json_perf_attaches_performance_field`,
`test_is_current_package_area_dirty_json_perf_attaches_performance_field`,
`test_repo_package_areas_json_perf_stdout_is_valid_json`). A
regression test that runs `--json --perf` for `repo root`,
`repo dirty-files`, and `repo recent-commits` and asserts
`serde_json::from_str(stdout).is_ok()` would have caught this.

### 2. `package` / `package-area` JSON loses its shape on no-result

`commands.rs:807-816` resolves the package name via
`render_repo_package`, then short-circuits to `handle_no_results` when
the result is empty. `handle_no_results` always renders the (text)
`on_error` message via `Prose` — even in JSON mode — and does not emit
a JSON object. The result is that the documented stable shape
(`{ "name": "" }` or a `--no-error` JSON success object) is not
produced. Compare with `repo_json::locator_root_outcome`, which
correctly emits `{ "root": "" }` plus exit 1 when the rendered locator
is empty.

This is observable today: `cd /tmp && sniff repo package --json` exits
with no JSON output at all. JSON consumers should always see an
object.

Suggested fix: emit `{ "name": "" }` (or, with `--no-error`, an empty
object plus exit 0) before delegating exit-code handling to
`handle_no_results`. The same fix applies to `PackageArea`. The
`Root` early-return at `commands.rs:482-503` has the same issue — when
the workdir cannot be discovered the path exits without emitting JSON.

### 3. `repo structure --filter <pat> --json` ignores the filter

`repo_json::build_with_outcome` matches `Some(RepoAction::Structure { .. })`
and falls through to `fallback_repo_value(result)`, which serializes
the entire `RepoInfo`. Text mode honours `--filter` via
`render_repo_section(... &filter ...)`, so JSON mode now diverges from
the text mode contract. The spec leaves this implicit (it documents
`structure --json` as "the full RepoInfo blob"), so this is a
borderline gap — but it is the only remaining repo subcommand whose
text and JSON outputs answer different questions for the same flags.

A focused JSON builder for `Structure` that filters `repo.packages`
the same way `render_repo_section` does (and preserves all other
top-level `RepoInfo` fields) would close the gap. If the spec
intentionally mandates the unfiltered shape, that should be stated
explicitly in `repo_json.rs` so future readers don't try to "fix" it.

### 4. `repo structure --latest-versions --json` enriches but doesn't expose

`enrich_result_dependencies` runs at `commands.rs:858`, but the
`Structure { latest_versions: true }` JSON path is the same
fallback that just serializes `RepoInfo`. Because
`DependencyEntry` already serializes `latest_version` /
`is_updatable` / `has_major_update`, this does work today — but
only because `serde` reflects the entire struct. Once #3 above is
addressed with a focused builder, those fields must be carried
through deliberately rather than incidentally.

## Test Coverage Gaps

The unit and integration tests in this feature are unusually thorough
— in particular `test_repo_subcommand_json_shapes_are_distinct` is a
durable regression guardrail. The remaining gaps are:

1. **No `--perf --json` test for any early-return path** (see Gap #1).
   The corruption is invisible to the existing test suite because
   every `--perf --json` test asserts on a path that goes through
   `print_json`.
2. **No integration test for `package-area --json`** when it resolves
   to a real area (only `package` has one in
   `test_package_name_json`). The distinctness matrix reaches it but
   doesn't assert on the shape.
3. **No integration test for `package-area-root --json`** when it
   resolves to a real area (only `package-root` has one in
   `test_package_root_json_when_present`).
4. **No integration test for `git-status --package <name> --json`**.
   The package-scoping logic at `commands.rs:735-799` mutates
   `result.filesystem.git` in place; a JSON-mode assertion that
   `recent`, `file_changes`, `status.dirty`, and `status.staged_count`
   are all scoped would catch any future regression in that block.
5. **No integration test for the JSON output of `package` /
   `package-area` when the path resolves to an empty name** (Gap
   #2). A test that exercises `--no-error` + `--json` + outside-area
   behaviour is needed to pin the stable-shape contract.
6. **`is-current-package-area-dirty --json` and
   `package-area-has-source-code-changes --json` don't cover the
   `true` branch at the integration layer.** The pure helpers cover
   it, but the end-to-end CLI exit-code wiring (`exit 0 on dirty`)
   is only validated in the false branch. A test using
   `--refresh-remotes` (or a fixture that pre-populates
   `RepoStatus.dirty`) would close this.
7. **`repo structure --filter <pat> --json` has no test** at all,
   contributing to Gap #3 going unnoticed.

## Ergonomic / Performance Notes

- `name_outcome` always returns `exit_code: None` and is used purely
  to wrap `{ "name": ... }`. Either remove the wrapper and return a
  bare `Value`, or actually use the exit-code field to carry empty-
  name semantics (which would also help close Gap #2).
- The 12 `perf.emit_stdout(None)` / `perf.emit_stderr(None)` call
  sites in `commands.rs` could be consolidated into a single
  `emit_perf(use_json, &result, &perf)` helper. That would make Gap
  #1 a one-line fix and prevent future drift.
- `serde_json::to_value(...).unwrap_or_else(|_| Value::Array(vec![]))`
  in `build_deps_package_entry` (`repo_json.rs:273-289`) silently
  masks serialization errors. `DependencyEntry` and `Vec<String>`
  serialize cleanly so the fallback is unreachable in practice;
  consider `.expect("DependencyEntry serializes")` to convert a
  silent bug into a panic, or just use `serde_json::to_value(...)?`
  if the function returns `Result`.
- `build_with_outcome` re-derives `(scope, kind)` strings for each
  package family arm. A small enum (`PackageFamilyKind::Packages`,
  `PackageFamilyAreas`) plus a single arm per scope would eliminate
  the duplication. Minor.
- `output/recent_commits.rs:91-120` constructs the JSON value, mutates
  it in place to add the `filter` field, and prints it — all inside
  the handler. Moving this into a `repo_json::commit_centric_value`
  helper would put the entire repo JSON contract in one module and
  let the unit tests in `repo_json::tests` cover it directly.

## Documentation

`sniff/cli/README.md`, `sniff/docs/cli/repo_*.md`, and the existing
per-command markdown pages are listed as modified in `git status` —
worth a final read to confirm the new JSON shapes are reflected
consistently and that the `--perf` interaction is documented.

## Verdict

**Ready for production:** No.

The functional core of the feature (the JSON shapes themselves) is
implemented to spec, well-factored, and well-tested. What blocks
release is acceptance criterion #9: `--perf --json` corrupts JSON
output for the majority of `sniff repo` subcommands today (Gap #1).
Gap #2 (no-result `name`/`root` shape) is a smaller but real contract
violation. Gap #3 (`structure --filter --json`) is the only remaining
text/JSON divergence for repo subcommands. None of these require deep
changes — they each fit within a focused follow-up commit, and the
existing test infrastructure makes durable coverage straightforward to
add.
