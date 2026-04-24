---
phases: 4
created: 2026-04-23
status: ready
feature: sniff repo package-areas (review 2 follow-up)
spec: spec.md
review: review-2.md
packages:
  - sniff-cli
  - sniff
blast_radius:
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/filesystem.rs
  - sniff/cli/tests/cli.rs
  - sniff/lib/src/filesystem/repo/detection.rs
addresses_findings:
  - json_perf_stdout_corruption
  - verbose_test_space_mismatch
  - missing_filter_and_root_tests
  - duplicated_package_area_selection_logic
  - missing_make_package_area_unit_tests
  - missing_package_area_root_unit_tests
---

# Review Plan 2: `sniff repo package-areas` Production Follow-Up

## Goal

Address every recommendation in `review-2.md` and leave `sniff repo package-areas` with valid JSON in `--json --perf`, a green focused test suite, and direct coverage for the filtering/root-derivation helpers that now carry the command's edge-case behavior.

## Current State Confirmed

- `sniff/cli/src/commands.rs::handle_repo_package_areas` prints a JSON array, then calls `perf.emit_stdout(None)` in JSON mode. With `--perf`, this appends human-readable performance output to stdout and corrupts the JSON stream.
- `sniff/cli/src/output/filesystem.rs` already has both `select_repo_package_areas` and `select_repo_package_areas_with_roots`, but they duplicate scope/filter parsing and predicate logic.
- `sniff/cli/tests/cli.rs::test_repo_package_areas_verbose_shows_root_dir` still asserts `pkg-a(./pkg-a)` and `pkg-b(./pkg-b)` without the required space.
- Missing review-plan-1 coverage is still absent for positional filters, negated filters, and root-area rendering.
- `sniff/lib/src/filesystem/repo/detection.rs::make_package_area` and `sniff/cli/src/output/filesystem.rs::package_area_root` are private helpers, so unit tests should live in their existing same-file `#[cfg(test)]` modules.

## Phase 1: Fix JSON/Perf Stream Separation

**Files:** `sniff/cli/src/commands.rs`, `sniff/cli/tests/cli.rs`

Change `handle_repo_package_areas` so the JSON branch emits performance data to stderr:

```rust
if json {
    let names: Vec<&str> = output::collect_repo_package_area_names(&info, filter, package_area);
    println!("{}", serde_json::to_string(&names)?);
    perf.emit_stderr(None);
    return Ok(());
}
```

This is the narrowest fix for the critical defect. Do not route this command through `output::print_json` unless also introducing a deliberate wrapper type/result shape, because the current command contract is a JSON array of package-area names and the fast path does not build a `SniffResult`.

Add an integration test:

- `test_repo_package_areas_json_perf_stdout_is_valid_json`
- Run `sniff --base <fixture> repo package-areas --json --perf --plain`
- Assert stdout parses as `Vec<String>` and equals `["pkg-a", "pkg-b"]`
- Assert stderr is non-empty and contains performance text/timing, so the test proves the perf report did not disappear

Also audit `handle_repo_packages`, which currently has the same JSON/perf shape. If a matching test already fails or a consistency patch is desired, apply the same stderr change there in the same phase; otherwise record it as a follow-up outside this review's scope.

## Phase 2: Unify Package-Area Selection Logic

**File:** `sniff/cli/src/output/filesystem.rs`

Refactor the duplicated helpers so `select_repo_package_areas` derives names from `select_repo_package_areas_with_roots`, making the root-aware path the single source of filtering behavior:

```rust
fn select_repo_package_areas<'a>(
    packages: &'a [Package],
    repo_filter: &[String],
    package_area: Option<&str>,
) -> Vec<&'a str> {
    select_repo_package_areas_with_roots(packages, repo_filter, package_area)
        .into_iter()
        .map(|(area, _)| area)
        .collect()
}
```

Keep `collect_repo_package_area_names` unchanged at the public boundary: it must still return a names-only `Vec<&str>` for JSON output.

To avoid simply moving duplication elsewhere, keep scope/filter parsing in `select_repo_package_areas_with_roots`, or extract a tiny private predicate helper only if it makes the code clearer. The important invariant is that JSON names and formatted verbose output use identical filter semantics.

## Phase 3: Repair and Expand CLI Integration Tests

**File:** `sniff/cli/tests/cli.rs`

Fix the regressed verbose test:

- Update `test_repo_package_areas_verbose_shows_root_dir` to assert `pkg-a (./pkg-a)` and `pkg-b (./pkg-b)`.
- Keep `--plain` in the command so ANSI markup cannot affect substring assertions.

Add or update fixture support:

- Extend `create_cli_monorepo` only if doing so does not disturb existing tests. Safer option: add `create_cli_monorepo_with_root_package()` that includes `root-tool` at repo root and workspace members `["pkg-a/lib", "pkg-b/lib", "."]` or an equivalent valid Cargo workspace shape.
- Commit the fixture repo as existing tests do, because commands discover repo structure from an initialized git repo.

Add missing tests from the review:

- `test_repo_package_areas_positional_filter`: `repo package-areas pkg-a --list --plain` returns exactly `pkg-a`.
- `test_repo_package_areas_positional_filter_negation`: `repo package-areas !pkg-a --list --plain` returns `pkg-b` and not `pkg-a`.
- `test_repo_package_areas_root_area_verbose_renders_dot_slash`: on the root-package fixture, `repo package-areas --list --verbose --plain` includes `root (./)` and does not include `root (./root)`.

Optional but useful coverage while touching this block:

- `test_repo_package_areas_json_perf_stdout_is_valid_json` from Phase 1 belongs near `test_repo_package_areas_json_output`.
- Keep exact `--package-area` coverage already present; add a case-insensitive assertion only if the refactor changes that path.

## Phase 4: Add Helper Unit Tests

**File:** `sniff/lib/src/filesystem/repo/detection.rs`

Add unit tests for `make_package_area` inside the existing `#[cfg(test)] mod tests`:

- root package path: `make_package_area("model_id") == "root"`
- top-level lib/cli split: `make_package_area("sniff/lib") == "sniff"`
- nested package area: `make_package_area("apps/browser/my_package") == "apps/browser"`
- optional defensive case: `make_package_area("./pkg")` documents current `Path::parent` behavior only if that shape is expected from callers; otherwise skip to avoid codifying unused input.

**File:** `sniff/cli/src/output/filesystem.rs`

Add unit tests for `package_area_root` inside the existing test module, reusing or extending `make_package`:

- root sentinel: package with `package_area = "root"` and `relative = "model_id"` returns `"."`
- normal area: `package_area = "sniff"`, `relative = "sniff/cli"` returns `"sniff"`
- multi-segment area: `package_area = "apps/browser"`, `relative = "apps/browser/my_package"` returns `"apps/browser"`
- fallback path: mismatched `package_area = "weird"`, `relative = "actual/path"` returns `"actual"`

If the current `make_package` helper always derives `relative` as `{area}/{name}`, add a local constructor or mutate `relative` after construction in the specific unit tests.

## Validation Commands

Focused tests while implementing:

```bash
cargo test -p sniff-cli --test cli -- repo_package_areas
cargo test -p sniff-cli output::filesystem::tests::package_area_root
cargo test -p sniff filesystem::repo::detection::tests::make_package_area
```

Manual smoke checks:

```bash
cargo run -p sniff-cli --quiet -- repo package-areas --json --perf --plain > /tmp/package-areas.json 2> /tmp/package-areas.perf
jq . /tmp/package-areas.json
test -s /tmp/package-areas.perf
cargo run -p sniff-cli --quiet -- repo package-areas --list --verbose --plain
cargo run -p sniff-cli --quiet -- repo package-areas sniff --list --plain
cargo run -p sniff-cli --quiet -- repo package-areas '!sniff' --list --plain
```

Final gates:

```bash
cargo test -p sniff-cli --test cli -- repo_package_areas
cargo test -p sniff-cli
cargo test -p sniff
cargo clippy -p sniff-cli -p sniff -- -D warnings
```

If time is available after the focused gates are green, run the area-level command:

```bash
just sniff test
just sniff lint
```

## Completion Criteria

- `sniff repo package-areas --json --perf` writes parseable JSON to stdout and all perf text to stderr.
- The verbose output contract is `area (./dir)`, including the required space.
- Root packages render as `root (./)` in verbose mode.
- Positional include filters, negated filters, `--package-area`, JSON output, and JSON+perf output are covered by integration tests.
- `make_package_area` and `package_area_root` have direct unit coverage for the root and nested-area cases.
- `select_repo_package_areas` no longer duplicates filter logic; it delegates to the root-aware selection path.
