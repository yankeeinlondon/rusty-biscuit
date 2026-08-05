---
status: implemented
created: 2026-08-05
issue: 29
---

# biscuit-speaks Cache Tests Race the Shared Cache File

## Summary

`biscuit-speaks::cache::tests::test_populate_cache_for_provider_success` has
failed intermittently on three unrelated PRs across two operating systems (#25
macOS, #22 macOS, #28 Windows). None of those changes has a dependency path to
`biscuit-speaks`, so each failure cost a manual `cargo tree` check to rule out a
real regression.

The cause is that the cache tests share one file: the caller's real
`~/.biscuit-speaks-cache.json`.

## Mechanism

`cache_file_path()` resolves to `dirs::home_dir().join(".biscuit-speaks-cache.json")`
with no seam for a caller to point elsewhere, so any test exercising
`read_from_cache`, `update_provider_in_cache`, `bust_host_capability_cache`, or
either `populate_cache_*` function operated on that single real file.

Four tests did so concurrently:

| Test | Operation on the shared file |
|---|---|
| `test_populate_cache_for_provider_success` | bust, write Piper, read back |
| `test_populate_cache_for_provider_updates_existing` | **bust**, write Festival twice, read back |
| `test_populate_cache_for_all_providers_runs_without_panic` | enumerate real system providers, write |
| `test_read_from_cache_file_not_found` | read |

The failing assertion is `read_from_cache().unwrap()` in the first test. It goes
red when the *second* test's opening `bust_host_capability_cache()` — which
deletes the file outright — lands between the first test's write and its
read-back. That is why exactly one assertion fails while the other ~362 tests
pass.

Two guards were in place and neither held:

1. `#[serial_test::serial]` on two of the four tests. `serial_test` takes an
   **in-process** lock. Nextest is process-per-test, so under the runner this
   repo actually uses, the attribute serialized nothing. It also did not cover
   `test_populate_cache_for_all_providers_runs_without_panic`, which was never
   marked serial and writes the same file.
2. Provider-key disjointness — `TtsProvider::Host(HostTtsProvider::Piper)`, with
   the comment *"Use Piper as unlikely to conflict"*. Choosing a distinct key
   does not help when a competing test deletes the whole file.

`.config/nextest.toml` constrains threads for `package(biscuit-speaks-cli)` only,
so the library's tests ran at full parallelism.

## Fix

Give every cache operation an explicit-path inner layer and keep the public
functions as thin wrappers that resolve `cache_file_path()` and delegate:

| Public (unchanged signature) | Inner |
|---|---|
| `read_from_cache` | `read_cache_at(&Path)` |
| `update_provider_in_cache` | `update_provider_at(&Path, ...)` |
| `bust_host_capability_cache` | `bust_cache_at(&Path)` |
| `populate_cache_for_provider` | `populate_provider_at(&Path, ...)` |
| `populate_cache_for_all_providers` | `populate_all_at(&Path)` |

Tests then drive the inner layer against a per-test `TempDir` and own their file
outright. No global state, no environment mutation, no `unsafe`, and no reliance
on the test runner's process model.

The public API is untouched, so `biscuit-speaks-cli` and
`lib/examples/list_voices.rs` are unaffected.

`cache_file_path()` gains a `BISCUIT_SPEAKS_CACHE` override alongside the seam,
with the precedence rule split into a pure `resolve_cache_path(Option<OsString>)`
so it is testable without mutating process-global environment state. In-process
callers need only the seam; the override exists for callers that cross a process
boundary and is consumed by the companion CLI fix.

`serial_test` is removed from `biscuit-speaks/lib`'s dev-dependencies; it had no
remaining call sites and, as established above, was not providing isolation
under nextest in the first place.

### Tests recovered as a side effect

The absence of a path seam had also driven several tests to re-implement the
logic they claimed to cover rather than call it — `test_bust_cache_removes_file`
called `fs::remove_file` directly, and the two `read_from_cache` error-path tests
hand-parsed JSON with `serde_json`. Those tests could not fail if the functions
they name were deleted. They now call `bust_cache_at` and `read_cache_at`, and
assert on the typed `TtsError::CacheReadError` and its message.

## Verification

- `just test` in `biscuit-speaks`: 99 CLI + 340 lib tests pass, 0 failures.
- `just lint`: clean.
- **Isolation, measured.** `~/.biscuit-speaks-cache.json` mtime is unchanged
  across a full 340-test library run. Before the fix, that run rewrote and at
  points deleted the developer's own cache file.
- **Non-vacuity, measured.** Each rewritten guard was neutered in turn and
  confirmed to go red, then restored:
  - `bust_cache_at` short-circuited to a no-op → `test_bust_cache_removes_file`
    FAILED.
  - the `capabilities.providers.push(capability)` line dropped from
    `update_provider_at` → both `populate_cache_for_provider` tests FAILED.
  - the schema-version comparison in `read_cache_at` short-circuited →
    `test_read_from_cache_schema_mismatch` FAILED.

## Follow-on

A full-area `just test` still moved the real cache file after this fix landed.
That was traced to a single CLI test whose `--refresh-cache` invocation escapes
into a detached grandchild, and is fixed separately in
`2026-08-05-cli-refresh-cache-escapes-test-sandbox`. With both in place, no leg
of `just test` touches `~/.biscuit-speaks-cache.json`.
