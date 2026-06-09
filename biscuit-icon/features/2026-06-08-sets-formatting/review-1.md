---
ready: false
agent: codex
model: ""
---

# Icon Sets Formatting Review

The core metadata, cache-count, and table-layout implementation is present, but
the feature is not ready for production. Its terminal-visible acceptance
criteria have no Level 2 verification, and the empty-result and migration
contracts are not fully implemented.

## Findings

### 1. High: The table output has no Level 2 verification

The acceptance criteria require a striped, aligned table with Unicode borders,
right-aligned formatted numbers, wrapping, and a side-by-side layout. All new
checks are Level 1 string tests using a manufactured `Terminal`; none run
`icon sets` in a real terminal or inspect the resulting pane text and SGR
output. The existing Level 2 suite covers icon glyphs and errors only.

Evidence:

- `cli/src/sets_table.rs:223-280` verifies rendered substrings in process.
- `cli/tests/cli.rs:910-1000` verifies layout from captured subprocess bytes.
- `cli/tests/level2_terminal.rs:60-329` contains no `sets` table test.

Add Level 2 coverage through `biscuit-test-harness` that captures a single and
split table in a real terminal. Verify border glyphs and column positions from
plain pane text, and verify alternating-row SGR background styling from raw
capture. This is a required verification-level mismatch, not optional
additional coverage.

### 2. High: An empty successful search renders an empty table

When `/collections` succeeds but no online, built-in, or cached set matches the
filter, `offline` remains empty. The command then builds and prints a table with
headers but no data rows. The specification explicitly requires preserving the
command's no-result error behavior instead of rendering an empty table.

Evidence:

- `cli/src/commands.rs:195-224` does not reject an empty final row set.
- `spec.md:232-233` requires an error rather than an empty table.
- No CLI test exercises a successful online response with an unmatched filter.

Check the merged result before querying counts or rendering, return the
established no-match error, and add online and offline no-result tests.

### 3. Medium: The version-0 migration is not transactional

The specification requires every version step to run in a transaction so a
failed migration cannot leave partially modified schema or data. The new
version-2 step is transactional, but the version-0-to-1 step still performs
table creation, column additions, origin conversion, column removal, and the
version bump across separate autocommit operations.

Evidence:

- `lib/src/cache/store.rs:53-157` performs most v1 migration operations outside
  one transaction.
- `lib/src/cache/store.rs:159-177` correctly uses a transaction for v2.
- `spec.md:93-95` requires each version step to be transactional.

Wrap the complete v1 step in one transaction, including its `user_version`
update. Add a failure-injection or deliberately invalid legacy-row test that
proves rollback leaves both schema and version unchanged.

### 4. Medium: This feature changes unrelated no-argument CLI behavior

The staged change makes bare `icon` print help and exit instead of dispatching
the documented default `icons` command with an empty filter. This behavior
change is unrelated to sets formatting and removes the existing offline-listing
path for a bare invocation.

Evidence:

- `cli/src/main.rs:31-40` adds the help-only branch.
- `cli/src/args.rs:25-30` still documents the no-subcommand default command.
- The feature specification contains no change to command dispatch.

Restore the prior dispatch behavior or move this change into a separately
specified feature with explicit tests and documentation.

### 5. Medium: The cached-count CLI test does not verify the required values

The test claims to cover zero and nonzero cached counts, but it only checks that
the entire output contains the character `3`. That character can occur in
borders, titles, totals, or unrelated rows, and the test never asserts that the
`empty` row displays `0`.

Evidence:

- `cli/tests/cli.rs:844-880` seeds counts of three and zero.
- `cli/tests/cli.rs:879` uses only `stdout.contains('3')`.

Parse or normalize the table rows and assert the `test` row has `Cached = 3`
and the `empty` row has `Cached = 0`.

## Verification Matrix

| Requirement | Strongest verification | Assessment |
|---|---:|---|
| Collection totals parse as present, zero, or missing and remain prefix-sorted | Level 1 wiremock | Appropriate |
| Schema v2 and total persistence | Level 1 SQLite | Coverage present; v1 transaction contract is not implemented |
| Batched cached counts exclude embedded icons | Level 1 SQLite | Appropriate |
| Online totals persist for offline display | Level 1 CLI subprocess | Appropriate |
| Missing totals show `Unknown` | Level 1 CLI subprocess | Appropriate |
| Cached zero and nonzero values display on the correct rows | Level 1 CLI subprocess | Inadequate assertion |
| Thousands separators | Level 1 renderer/CLI | Necessary but not sufficient for terminal rendering |
| Narrow, split, and wide-tall layout selection | Level 1 renderer/CLI | Logic covered |
| Balanced column-major split ordering | Level 1 renderer | Appropriate for distribution logic |
| Unicode borders, widths, wrapping, right alignment, and striping | Level 1 only | **Level mismatch: requires Level 2** |
| Keyboard, mouse, paste, or IME behavior | Not applicable | No Level 3 requirement |

## Validation

- `git diff --cached --check`: passed.
- `cargo test -p biscuit-icon`: not run; `rustup` reports no installed/default
  toolchain.
- `cargo test -p biscuit-icon-cli`: not run for the same reason.
- `cargo clippy -p biscuit-icon -p biscuit-icon-cli --all-targets -- -D warnings`:
  not run for the same reason.
- Level 2 tests were not run because the required Rust toolchain is unavailable;
  inspection also confirms there is no Level 2 test for this feature.

The requested `biscuit-icon` skill is not present in the repository's
authoritative skill catalog or configured skill roots. The review used the
package specification, repository conventions, prior package reviews, and the
`rust-testing` skill.
