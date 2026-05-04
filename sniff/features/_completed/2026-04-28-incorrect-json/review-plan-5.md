---
ready: true
agent: "planner"
model: "codex"
---

# Review 5 Implementation Plan

## Scope

Address the remaining review-5 blocker for the incorrect JSON feature:

- `sniff repo packages --json` must remain a JSON array of strings.
- `sniff repo package-areas --json` must remain a JSON array of strings.
- The `{ "names": [...] }` object shape must remain limited to the dirty/staged/unstaged package and package-area families, where the spec explicitly requires `{ scope, kind, names }`.

No changes are needed for the review-4 `package-area-has-source-code-changes --json` fix or for the other repo JSON subcommands that review-5 marked appropriate.

## Phase 1: Restore the Production JSON Contracts

Edit `sniff/cli/src/commands.rs`.

1. In `handle_repo_packages`, keep using:
   - `output::collect_repo_package_names(&info, filter, package_area)`
   - the existing fast path through `sniff::filesystem::repo::detect_repo_structure`
   - the existing `perf.emit_stderr(None)` behavior

2. Change only the JSON serialization branch from wrapping the names in an object to serializing the names vector directly:

   ```rust
   println!("{}", serde_json::to_string(&names)?);
   ```

3. In `handle_repo_package_areas`, make the same focused change:
   - keep `output::collect_repo_package_area_names(&info, filter, package_area)`
   - serialize the returned names vector directly
   - preserve the existing performance stderr behavior

4. Do not change the dirty/staged/unstaged package-family handlers. Those should continue to emit `{ scope, kind, names }`.

Expected production result:

- `sniff --base <repo> repo packages --json` emits `["pkg-a","pkg-b"]`.
- `sniff --base <repo> repo package-areas --json` emits `["pkg-a","pkg-b"]`.
- `sniff --base <repo> repo dirty-packages --json` still emits an object with `scope`, `kind`, and `names`.

## Phase 2: Correct Level 1 CLI Regression Tests

Edit `sniff/cli/tests/cli.rs`.

There are already local modifications in this file in the current worktree, so implementers must preserve unrelated edits and change only the relevant assertions.

Update these tests:

- `test_repo_packages_json_output`
- `test_repo_package_areas_json_output`
- `test_repo_package_areas_json_perf_stdout_is_valid_json`

For each test:

1. Parse stdout into `serde_json::Value` as it does today.
2. Assert that the top-level value is an array:

   ```rust
   let names = json.as_array().expect("top-level JSON must be an array");
   ```

3. Compare the array contents directly to the expected package or area names:

   ```rust
   assert_eq!(
       names
           .iter()
           .map(|v| v.as_str().unwrap())
           .collect::<Vec<_>>(),
       vec!["pkg-a", "pkg-b"]
   );
   ```

4. Keep the `--perf --json` test's stderr assertion unchanged. It should continue to prove that stdout is valid JSON and perf output goes to stderr.

5. Leave the package-family tests around `dirty-packages`, `dirty-package-areas`, `staged-*`, and `unstaged-*` unchanged. Those tests should continue asserting `json["names"]`, `json["scope"]`, and `json["kind"]` because those subcommands intentionally return objects.

Expected test coverage after this phase:

- Level 1 CLI integration coverage proves the two already-correct public JSON contracts remain arrays.
- Existing Level 1 package-family tests continue proving the object shape is limited to the subcommands that require it.
- Existing `--perf --json` coverage continues proving performance output does not corrupt JSON stdout.

## Phase 3: Focused Verification

Run the narrow tests first:

```bash
cargo test -p sniff-cli test_repo_packages_json_output
cargo test -p sniff-cli test_repo_package_areas_json_output
cargo test -p sniff-cli test_repo_package_areas_json_perf_stdout_is_valid_json
```

Then run the broader feature-relevant CLI filters from review-5 to catch accidental regressions:

```bash
cargo test -p sniff-cli repo_json
cargo test -p sniff-cli test_repo_subcommand_json_shapes_are_distinct
```

Run the package-family shape tests if the implementation touched any nearby shared helper:

```bash
cargo test -p sniff-cli dirty_packages_json
cargo test -p sniff-cli dirty_package_areas_json
cargo test -p sniff-cli staged_packages_json
cargo test -p sniff-cli staged_package_areas_json
cargo test -p sniff-cli unstaged_packages_json
cargo test -p sniff-cli unstaged_package_areas_json
```

Run lint/format checks for the touched package:

```bash
cargo fmt --check -p sniff-cli
cargo clippy -p sniff-cli --all-targets -- -D warnings
```

If `cargo fmt --check -p sniff-cli` is not supported by the installed Cargo version, use:

```bash
cargo fmt --check --package sniff-cli
```

Manual smoke checks after building `sniff`:

```bash
target/debug/sniff --base . repo packages --json
target/debug/sniff --base . repo package-areas --json
target/debug/sniff --base . repo dirty-packages --json
target/debug/sniff --base . repo package-areas --json --perf --plain
```

Manual expectations:

- `packages --json` stdout starts with `[` and parses as a JSON array.
- `package-areas --json` stdout starts with `[` and parses as a JSON array.
- `dirty-packages --json` remains an object with `scope`, `kind`, and `names`.
- `package-areas --json --perf --plain` keeps JSON on stdout and writes perf output to stderr.

## Completion Criteria

- Review-5's only high finding is resolved.
- The tests that previously asserted `json["names"]` for `packages` and `package-areas` now assert top-level arrays.
- The object shape for dirty/staged/unstaged package-family subcommands is unchanged.
- Focused CLI tests pass.
- Feature-relevant regression tests pass.
- `cargo fmt --check` and `cargo clippy -p sniff-cli --all-targets -- -D warnings` pass, or any environmental blocker is documented with exact command output.
