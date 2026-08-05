---
status: implemented
created: 2026-08-05
---

# `--refresh-cache` Rewrites the Caller's Real Cache From a Test

## Summary

`test_cli_background_with_refresh_cache_is_allowed`
(`biscuit-speaks/cli/tests/cli_test.rs`) runs
`so-you-say --background --refresh-cache test`. That deleted and rebuilt the
developer's or CI runner's real `~/.biscuit-speaks-cache.json` on every
`just test` — **after** the test had already reported green.

Found while fixing the library-side race in
`2026-08-05-cache-tests-race-shared-file`, which that spec recorded as out of
scope. Investigation showed it to be a live defect rather than housekeeping.

## Mechanism

In `cli/src/main.rs` the `--background` branch re-spawns the process detached and
**returns at line 1236, before** the `--refresh-cache` branch at line 1240:

1. The parent spawns a detached `so-you-say --refresh-cache test` and exits 0.
2. The test asserts that exit 0 and completes.
3. The orphaned grandchild then calls `bust_host_capability_cache()` (deleting
   the real file), re-enumerates every installed provider, rewrites the file, and
   speaks.

This delegation is **intended behavior**, ratified 2026-08-05:
`--background --refresh-cache` means "refresh in the background", so the parent
returning before the refresh is correct and the test's exit-0 assertion — that
the flag pair is *allowed* — is exactly what it should assert.

The defect is therefore scoped to one thing: **the side effect outlives the
run.** A green `just test` with a detached process still mutating shared state
outside the build directory is the shape that produces unattributable cross-test
interference later. Measured: the cache file's mtime moved *after* the suite
reported success, not during it.

## Why the obvious fix does not work

Overriding `HOME` on the child `Command` fails the monorepo's cross-platform
requirement. `dirs 6` resolves `home_dir()` as `dirs_sys::home_dir()` on
macOS/Linux, which honors `$HOME`, but as `dirs_sys::known_folder_profile()` on
Windows — a `SHGetKnownFolderPath` call that ignores `USERPROFILE`. The test
would look sandboxed locally and still rewrite the profile cache on the Windows
runner.

## Fix

`cache_file_path()` now honors `BISCUIT_SPEAKS_CACHE`, naming the cache **file**
(not a directory), with an empty value treated as unset. The test sets it on the
child `Command`.

Environment is inherited across `--background`'s re-spawn, so the override
reaches the detached grandchild. That inheritance is the specific property this
defect requires; a mechanism scoped to the parent process would not have
contained it.

The precedence rule is split into a pure `resolve_cache_path(Option<OsString>)`
so it can be asserted without mutating process-global environment state — the
same seam discipline applied to the file operations in the companion fix, and
for the same reason.

Documented in `biscuit-speaks/README.md` (Environment Variables) and
`biscuit-speaks/docs/tts-caching.md` (Provider Capability Cache → Location).

## Verification

- `just lint` clean; `just test` 100 CLI + 342 lib tests pass.
- **Containment, measured end to end.** Running the built binary as
  `BISCUIT_SPEAKS_CACHE=$TMP/relocated.json so-you-say --background
  --refresh-cache test` produced a full 166 KB cache at the override path, while
  `~/.biscuit-speaks-cache.json` kept its prior mtime. This is direct evidence
  that the override survives the detached re-spawn.
- **Suite-level invariant.** A full-area `just test`, plus an 8 s settle for the
  orphan, leaves the real cache file's mtime unchanged. Before this fix the same
  run moved it.
- **Non-vacuity, measured.** `resolve_cache_path`'s override arm was
  short-circuited → `test_cache_path_override_wins_over_home` FAILED; its
  `is_empty` guard was short-circuited →
  `test_cache_path_falls_back_when_override_absent_or_empty` FAILED. Both
  restored.

## Ratified semantics

`--background --refresh-cache` delegating the refresh to a detached child is
correct and deliberate (2026-08-05). The flag ordering in `main.rs` is not to be
"fixed", and the parent exiting 0 before the refresh completes is the contract,
not a reporting bug. Only the escape of the side effect into the caller's real
cache was a defect, and that is what this change closes.
