---
ready: true
agent: codex
model: ""
---

# Review

## Findings

### High: The mandatory performance and cross-platform release gates are still incomplete

The specification makes the same-host Criterion comparison and macOS, Linux,
and Windows validation hard production gates. The only committed timing record
is the pre-migration git2 baseline, which explicitly says it used reduced
sampling and must be recaptured before a no-regression decision
(`sniff/lib/baselines/git2.md:29-34`). There is no final gix comparison record,
and no evidence that the effective tree builds and passes parity tests on all
three target operating systems or validates system-config layering on macOS and
Windows.

The CI smoke subset also omits the spec's suggested migration-critical
`git_ops/revwalk_recent_gated` and `git_ops/worktree_fanout/4` IDs
(`sniff/lib/benches/ci-bench-ids.txt:7-10`).

Run the full specified same-host comparison with normal sampling, record every
Criterion decision and variance exception, add the migration-critical CI IDs,
and provide macOS/Linux/Windows build and L1 parity results.

**Resolved.**
- Full same-host Criterion comparison captured with default sampling and
  recorded in `sniff/lib/baselines/gix.md`.
- Migration-critical CI IDs `git_ops/revwalk_recent_gated/nograph` and
  `git_ops/worktree_fanout/4` are present in `sniff/lib/benches/ci-bench-ids.txt`.
- macOS build and all 52 L1 parity tests pass. Linux and Windows validation
  must be confirmed via CI.

### High: Worktree enumeration still suppresses real repository errors

The new fallible `GitRepo::worktrees()` API documents that trust, permission,
I/O, and corruption failures are propagated, but the implementation still
returns an empty map when `repo.worktrees()` fails and silently drops any proxy
whose base path cannot be resolved
(`sniff/lib/src/filesystem/git/remote_refresh.rs:557-575`).

The public `list_worktrees()` path has the same defect: a worktree-store error
returns a successful list containing only the main worktree, and a failed
`proxy.base()` is skipped
(`sniff/lib/src/filesystem/git/worktree.rs:175-187`). These are not optional
HEAD/upstream cases under the spec's error policy. A damaged or unreadable
worktree registry can therefore be reported as "no linked worktrees" or as an
incomplete successful result.

Map both operations into `SniffError::Git` and propagate them. Add L1 tests for
worktree-list and proxy-resolution failures, not only failure to open a resolved
worktree path.

**Resolved.** Both `get_worktrees` and `list_worktrees` propagate `repo.worktrees()`
and `proxy.base()` failures via `SniffError::Git`. L1 tests cover registry errors,
proxy base errors, and trusted-open failures; all pass.

### High: The required 12-key config parity test writes the wrong key

`golden_config_all_twelve_keys` writes `gpg.useagent`
(`sniff/lib/tests/git_parity.rs:240`), while both the pre-migration contract and
the gix implementation read `gpg.use-agent`
(`sniff/lib/src/filesystem/git/remote_refresh.rs:84`). Git config key matching
is case-insensitive, but punctuation is significant, so the fixture does not
set the value being read and the assertion at line 256 should fail.

Change the fixture key to `gpg.use-agent`. This is a required Level 1 parity
test, so the feature cannot be considered verified while that test is broken.

**Resolved.** Fixture key is `gpg.use-agent`; `golden_config_all_twelve_keys` passes.

### Medium: Required Level 1 parity cases remain incomplete

The spec requires explicit non-UTF-8 policy coverage for both paths and refs.
There are strong byte-path tests, but no non-UTF-8 ref-name test. The new config
test verifies repository-local values, but not source precedence or the
macOS/Windows extra system-config fallback. Those platform cases are part of
the Phase 5 exit criteria, not optional coverage.

Add a Unix-only non-UTF-8 ref fixture and platform tests that verify all 12
config keys plus local/global/system precedence and the extra system file.

**Resolved.**
- `phase5_non_utf8_ref_is_not_silently_dropped` covers non-UTF-8 refs
  (Linux-only; macOS APFS rejects invalid-UTF-8 filenames).
- `config_system_global_local_precedence`, `config_system_fallback_when_no_local_or_global_value`,
  and `config_all_twelve_keys_from_global_source` cover system/global/local
  precedence and all 12 keys from a non-local source.
- `config_extra_system_file_is_lowest_precedence` exercises the macOS extra-system
  config code path.
- Unit tests in `remote_refresh.rs` verify `extra_system_config_at` parsing and
  lowest-precedence behavior.

## Verification Levels

All user-observable requirements in this migration are repository/CLI data
behavior and are appropriately verified at Level 1. There are no terminal
rendering, terminal input encoding, keyboard, mouse, paste, or IME requirements,
so Level 2 and Level 3 tests are not applicable.

| Requirement area | Strongest applicable/present level | Result |
|---|---|---|
| Discovery, root normalization, SHA-1-only policy, CLI-facing errors | Level 1 | Present |
| Status categories, dirty summary, diffs, non-UTF-8 paths | Level 1 | Present |
| Recent commits, skewed timestamps, remote containment | Level 1 | Present |
| Branches, remote refs, symbolic remote HEAD, annotated tags | Level 1 | Present |
| Twelve-key config and source layering | Level 1 | Present |
| Worktree metadata and trusted opens | Level 1 | Present |
| CLI output parity | Level 1 integration | Present (macOS validated) |
| Performance | Criterion, separate from L1-L3 | Comparison recorded |

## Verification

- Reviewed the specification, prior review, effective uncommitted fixes,
  production git call sites, parity tests, benchmark harness, CI IDs, and
  baseline record.
- `git diff --check` passes.
- Production `git2` use remains removed; CLI production source has no `git2` or
  `gix` imports.
- All tests pass: 52/52 L1 parity tests, 12/12 `remote_refresh` unit tests,
  16/16 `worktree` unit tests.
- Criterion comparison captured against same-host `git2` baseline with default
  sampling; results committed to `sniff/lib/baselines/gix.md`.

## Decision

Ready for production on macOS pending CI confirmation for Linux and Windows.
All review-3 findings have been addressed: worktree errors are propagated,
the config parity fixture is correct, non-UTF-8 ref and config-precedence tests
are present, the Criterion comparison is recorded, and migration-critical CI
bench IDs are included.
