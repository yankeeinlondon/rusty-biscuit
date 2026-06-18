---
ready: true
agent: codex
model: ""
---

# Review: Poor Error Report Fix

## Findings

### High: Required Level 1 coverage is incomplete for the specified file-reference failure matrix

The implementation adds the core typed helper and message substitution, but the tests only cover a subset of the required behavior. Current coverage includes invalid syntax, missing relative files, existing files, direct `format: darkmatter-file`, unrelated format preservation, and the `x-darkmatter-match` schema guard. The spec also requires verification for resolution errors, missing absolute/magic/package/recursive references without fabricated candidate paths, nested/array paths, root-union attribution, and escaped rendering of reference/source-error text. I do not see tests for those cases in the new coverage around [format.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/schemas/format.rs:447), [validate.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/schemas/validate.rs:715), or the renderer snapshot at [markdown_error.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/tests/error_snapshots/markdown_error.rs:171).

Verification level: Level 1 is the right level for these schema diagnostics; no Level 2 or Level 3 terminal testing is required for this feature. The issue is that several Level 1 cases listed by the spec are absent, so this is not production-ready under the requested test rigor.

Suggested coverage to add:

- `{{UNSET_ENV_VAR}}` or `vault:...` produces `could not resolve file reference` with the source error.
- missing absolute, magic, package, and recursive references report no fabricated candidate path.
- `file(match(...))` missing file produces exactly one file-reference diagnostic through the full validator, not only `Keyword::is_valid`.
- existing file that violates `file(match(...))` produces exactly one glob diagnostic through the full validator.
- nested object or array item paths retain JSON-pointer attribution.
- root-union validation preserves the selected arm attribution for the improved message.
- rendered diagnostics containing markup-like or escape-sensitive text are escaped by the renderer.

### Medium: CWD-mutating tests use separate locks, so the unit suite can race under normal parallel test execution

The new tests mutate the process working directory in both `format.rs` and `validate.rs`, but each module defines its own mutex: [format.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/schemas/format.rs:380) and [validate.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/schemas/validate.rs:680). Those tests run in the same `darkmatter` unit-test binary, and Rust runs test functions in parallel by default. Because the locks are not shared, one test can change CWD while the other is resolving `./does-not-exist.md` or `./exists.md`.

Use the repository's shared serialization pattern for process-global state, such as a single shared CWD guard or `serial_test` on every CWD-mutating test with the same serial key. This is especially important here because the diagnostic message explicitly captures the current directory.

## Notes

The main implementation shape is sound: `format::resolve_file_reference` centralizes parse/resolution/no-match classification, `validate::build_problem` substitutes only `darkmatter-file` format failures, and `x-darkmatter-match` now delegates parse/resolution/no-match failures to the format validator. The public SimplifiedSchema conversion appears unchanged.

I ran:

```text
cargo test -p darkmatter markdown::schemas --color=never
```

Result: passed, with 224 schema-related unit tests run. This does not cover the missing cases above.
