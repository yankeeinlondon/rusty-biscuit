---
feature: incorrect-json
review: review-2.md
created: 2026-04-29
phases: 5
start_phase: 3
source_files_during_phase_1:
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4:
  - sniff/cli/src/output/filesystem.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
source_files_during_phase_5:
  - sniff/cli/src/output/repo_json.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase5: []
packages:
  - sniff-cli
---

# Implementation Plan — Address `review-2.md` Gaps

This plan addresses every gap identified in `review-2.md`. The bulk of the
`incorrect-json` feature is already implemented and well-tested; the remaining
work focuses on:

1. **BLOCKER** — `--perf` corrupts JSON for early-return subcommands
2. Stable JSON shapes for `package`/`package-area` on empty results
3. `repo structure --filter` / `--latest-versions` in JSON mode
4. Targeted integration-test coverage for untested paths
5. Minor ergonomic cleanups

The plan is organized into **5 phases** that can be implemented sequentially.
Each phase is self-contained with specific files, line numbers, and test
expectations.

## Working assumptions

- Repo root: `/Users/ken/.claudine/worktrees/rusty-biscuit/sniff`
- Sniff lib crate: `sniff/lib/` (`-p sniff`)
- Sniff CLI crate: `sniff/cli/` (`-p sniff-cli`)
- All `cargo` invocations MUST be targeted (no bare `cargo build` / `cargo test` at repo root).
- All shape decisions match `spec.md`.

---

## Phase 1 — Central `emit_perf` helper + fix all `--perf` corruption (BLOCKER)

**Goal.** Eliminate every `perf.emit_stdout(None)` / `perf.emit_stdout(detailed)`
call that fires after JSON has been printed to stdout. All early-return JSON
paths must route perf to stderr so stdout stays valid JSON.

### 1A. Add a central helper on `CliPerf`

**File:** `sniff/cli/src/commands.rs`

Add a new method to `CliPerf` (around line 62, after the existing `emit` method):

```rust
/// Emit perf output, routing to stderr when JSON has been printed to
/// stdout so the JSON payload stays machine-parseable.
pub fn emit_for_json(&self, detailed: Option<&PerformanceReport>) {
    self.emit(detailed, true)
}
```

This is a semantic alias that makes every call site's intent self-documenting.
(It always emits to stderr, which is correct for JSON mode — stdout must be
reserved for the JSON payload.)

### 1B. Replace every corrupting `emit_stdout` call in early-return JSON paths

Every call site listed in review-2 Gap #1's table must be updated. The pattern
is: if the code just printed JSON to stdout (`println!` of `serde_json`), the
perf emit must go to stderr.

**File:** `sniff/cli/src/commands.rs`

| Location | Current | Change to |
|----------|---------|-----------|
| `Root` JSON branch (line ~497) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |
| `Hash` JSON branch (line ~393) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |
| `Pr` after JSON/text output (line ~1122) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |
| `Remote` URL handlers (lines ~328, ~345) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |
| `BlastRadius` JSON branch (line ~260) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |
| `handle_file_list_command` JSON branch (line ~1572) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |
| `Package` JSON branch (line ~815) | `perf.emit_stdout(result.performance.as_ref())` | `perf.emit_for_json(result.performance.as_ref())` |
| `PackageArea` JSON branch (line ~835) | `perf.emit_stdout(result.performance.as_ref())` | `perf.emit_for_json(result.performance.as_ref())` |
| `HasMergeConflict` JSON branch (line ~523) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |

**File:** `sniff/cli/src/output/recent_commits.rs`

| Location | Current | Change to |
|----------|---------|-----------|
| `RecentCommits` JSON branch (line ~94) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |
| SourceCodeChanges / DocumentationChanges JSON branch (line ~117) | `perf.emit_stdout(None)` | `perf.emit_for_json(None)` |

### 1C. Update the `print_json` path's perf emit to use the helper too

**File:** `sniff/cli/src/commands.rs`, lines 893-897

Replace the existing conditional:

```rust
if result.performance.is_some() {
    perf.emit_stderr(result.performance.as_ref());
} else {
    perf.emit_stdout(result.performance.as_ref());
}
```

With:

```rust
perf.emit_for_json(result.performance.as_ref());
```

This simplifies the logic: when JSON is on stdout, perf always goes to stderr.
The conditional was redundant.

### 1D. Add integration tests for `--perf --json` on early-return paths

**File:** `sniff/cli/tests/cli.rs`

Add the following tests:

```rust
#[test]
fn test_repo_root_json_perf_stdout_is_valid_json() {
    // `repo root --json --perf` must produce parseable JSON on stdout.
    let assert = cargo_bin_cmd!("sniff")
        .args(["repo", "root", "--json", "--perf"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert!(stdout.contains("root"), "should contain root key");
}
```

```rust
#[test]
fn test_repo_dirty_files_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    std::fs::write(path.join("src/main.rs"), "fn main() { dirty }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "dirty-files", "--json", "--perf",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
}
```

```rust
#[test]
fn test_repo_recent_commits_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "recent-commits", "--json", "--perf",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert!(stdout.contains("commits"), "should contain commits key");
}
```

```rust
#[test]
fn test_repo_has_merge_conflict_json_perf_stdout_is_valid_json() {
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "has-merge-conflict", "--json", "--perf",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let _: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
}
```

### Verification

```
cargo build -p sniff-cli
cargo test -p sniff-cli --test cli json_perf_stdout_is_valid_json
cargo test -p sniff-cli --test cli json_shapes_are_distinct
```

---

## Phase 2 — Stable JSON shape for `package`/`package-area` on no-result

**Goal.** When `package` or `package-area` resolves to empty, JSON consumers
must still see a stable object (`{ "name": "" }`) instead of prose error text
or no output at all. Same for `Root` when workdir discovery fails.

### 2A. Fix `Package` and `PackageArea` empty-name JSON path

**File:** `sniff/cli/src/commands.rs`, lines 805-841

Current behavior: when `plain_name.is_empty()`, the code calls
`handle_no_results` which emits Prose text even in JSON mode. Fix by emitting
the stable JSON shape *before* delegating to `handle_no_results` when `cli.json`
is true.

For `Package` (lines 805-826):

```rust
crate::args::RepoAction::Package { no_error, on_error } => {
    let plain_name = output::render_repo_package(&result, base_dir.as_deref(), 0);
    if plain_name.is_empty() {
        if cli.json {
            let outcome = output::repo_json::name_outcome(String::new());
            println!("{}", serde_json::to_string_pretty(&outcome.value)?);
            perf.emit_for_json(result.performance.as_ref());
            return handle_no_results_exit_code(*no_error);
        }
        return handle_no_results(*no_error, on_error, cli.plain, &perf);
    }
    // ... rest unchanged
}
```

Wait — `handle_no_results` calls `std::process::exit` internally, which
means we need a variant that returns the exit code instead of calling exit.
But that's a bigger refactor. The simpler approach:

**Approach:** Emit the JSON shape, then let `handle_no_results` handle exit
codes. Since `handle_no_results` also calls `perf.emit_stderr` internally, we
should skip the perf emit in our JSON branch and let `handle_no_results` do it.

Actually, looking more carefully at `handle_no_results` (line 1506-1537): it
always renders Prose text via `on_error` message, then calls
`perf.emit_stderr(None)` and `std::process::exit()`. The Prose rendering is
only to stderr when `on_error` is set (line 1527: `eprintln!("{text}")`), and
when `no_error` is false with no `on_error`, there's no text output at all
(lines 1529-1536 just emit perf and exit).

So the cleanest fix is:

1. In the `Package` and `PackageArea` arms, when `cli.json` is true and the
   name is empty, print the stable JSON shape, emit perf to stderr, then exit
   with the appropriate code — *before* reaching `handle_no_results`.

**File:** `sniff/cli/src/commands.rs`

Replace the `Package` arm (lines 805-826) with:

```rust
crate::args::RepoAction::Package { no_error, on_error } => {
    let plain_name = output::render_repo_package(&result, base_dir.as_deref(), 0);
    if plain_name.is_empty() {
        if cli.json {
            println!("{}", serde_json::to_string_pretty(
                &output::repo_json::name_outcome(String::new()).value
            )?);
            perf.emit_for_json(result.performance.as_ref());
            std::process::exit(if *no_error { 0 } else { 1 });
        }
        return handle_no_results(*no_error, on_error, cli.plain, &perf);
    }
    if cli.json {
        let outcome = output::repo_json::name_outcome(plain_name);
        println!("{}", serde_json::to_string_pretty(&outcome.value)?);
        perf.emit_for_json(result.performance.as_ref());
        return Ok(());
    }
    let rendered = if cli.verbose > 0 {
        output::render_repo_package(&result, base_dir.as_deref(), cli.verbose)
    } else {
        plain_name
    };
    println!("{rendered}");
    perf.emit_stderr(result.performance.as_ref());
    Ok(())
}
```

Same pattern for `PackageArea` (lines 827-841):

```rust
crate::args::RepoAction::PackageArea { no_error, on_error } => {
    let rendered = output::render_repo_package_area(&result, base_dir.as_deref());
    if rendered.is_empty() {
        if cli.json {
            println!("{}", serde_json::to_string_pretty(
                &output::repo_json::name_outcome(String::new()).value
            )?);
            perf.emit_for_json(result.performance.as_ref());
            std::process::exit(if *no_error { 0 } else { 1 });
        }
        return handle_no_results(*no_error, on_error, cli.plain, &perf);
    }
    if cli.json {
        let outcome = output::repo_json::name_outcome(rendered);
        println!("{}", serde_json::to_string_pretty(&outcome.value)?);
        perf.emit_for_json(result.performance.as_ref());
        return Ok(());
    }
    println!("{rendered}");
    perf.emit_stderr(result.performance.as_ref());
    Ok(())
}
```

### 2B. Fix `Root` when workdir cannot be discovered

**File:** `sniff/cli/src/commands.rs`, lines 482-503

The current code already handles the `None` workdir case, but only for text
mode. When `cli.json` is true and `repo.workdir()` returns `None`, we need to
emit `{ "root": "" }` instead of silently exiting.

Replace the `Root` arm with:

```rust
crate::args::RepoAction::Root => {
    let dir = base_dir
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new("."));
    let repo = match git2::Repository::discover(dir) {
        Ok(r) => r,
        Err(_) => {
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "root": ""
                }))?);
                perf.emit_for_json(None);
                std::process::exit(1);
            }
            perf.emit_stderr(None);
            std::process::exit(1);
        }
    };
    let Some(workdir) = repo.workdir() else {
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "root": ""
            }))?);
            perf.emit_for_json(None);
            std::process::exit(1);
        }
        perf.emit_stderr(None);
        std::process::exit(1);
    };
    if cli.json {
        let json = serde_json::json!({
            "root": workdir.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
        perf.emit_for_json(None);
    } else {
        println!("{}", workdir.display());
        perf.emit_stderr(None);
    }
    Ok(())
}
```

### 2C. Update `name_outcome` to carry exit-code semantics

**File:** `sniff/cli/src/output/repo_json.rs`, lines 200-202

Currently `name_outcome` always returns `exit_code: None`. Update it to set
`exit_code: Some(1)` when the name is empty:

```rust
pub(crate) fn name_outcome(rendered: String) -> BuildOutcome {
    let exit_code = if rendered.is_empty() { Some(1) } else { None };
    BuildOutcome {
        value: json!({ "name": rendered }),
        exit_code,
    }
}
```

This doesn't change the `commands.rs` callers (they still handle exit codes
directly), but it makes the function's contract clearer and lets future callers
rely on it.

### 2D. Add integration tests

**File:** `sniff/cli/tests/cli.rs`

```rust
#[test]
fn test_package_json_empty_name_stable_shape() {
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "package", "--json",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String(String::new()));
}
```

```rust
#[test]
fn test_package_area_json_empty_name_stable_shape() {
    let (_dir, path) = create_test_repo();
    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "package-area", "--json",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String(String::new()));
}
```

### Verification

```
cargo test -p sniff-cli --test cli package_json_empty
cargo test -p sniff-cli --test cli package_area_json_empty
cargo test -p sniff-cli output::repo_json
```

---

## Phase 3 — `repo structure --filter` / `--latest-versions` in JSON mode

**Goal.** When `--filter` is passed to `repo structure --json`, the JSON output
should only contain packages matching the filter — matching text mode behavior.
When `--latest-versions` is set, the enrichment must carry through to the
filtered JSON.

### 3A. Add a focused `Structure` JSON builder in `repo_json.rs`

**File:** `sniff/cli/src/output/repo_json.rs`

Add a new function and wire it into `build_with_outcome`:

```rust
fn structure_value(result: &SniffResult, filter: &[String]) -> Value {
    let Some(fs) = result.filesystem.as_ref() else {
        return json!({});
    };
    let Some(repo) = fs.repo.as_ref() else {
        return json!({});
    };

    let mut value = serde_json::to_value(repo).unwrap_or(Value::Null);

    if !filter.is_empty() {
        if let Some(packages_val) = value.get_mut("packages").and_then(|p| p.as_array_mut()) {
            let filtered: Vec<Value> = packages_val
                .iter()
                .filter(|pkg| {
                    let name = pkg.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let area = pkg.get("package_area").and_then(|a| a.as_str()).unwrap_or("");
                    filter.iter().any(|f| {
                        let parsed = filesystem::RepoFilter::parse(f);
                        parsed.matches(&sniff::filesystem::repo::Package {
                            name: name.to_string(),
                            package_area: area.to_string(),
                            ..default_package()
                        })
                    })
                })
                .cloned()
                .collect();
            *packages_val = serde_json::Value::Array(filtered);
        }
    }

    value
}
```

Actually, a cleaner approach: reconstruct a filtered `RepoInfo` value using
serde, filtering the `packages` array. But we need the `RepoFilter` matching
which operates on `Package` structs, not JSON values. The simplest correct
approach:

1. Get the `RepoInfo` from the result.
2. Filter `repo.packages` using `filter_packages`.
3. Clone the repo, replace packages with the filtered set.
4. Serialize the clone.

```rust
fn structure_value(result: &SniffResult, filter: &[String]) -> Value {
    let Some(fs) = result.filesystem.as_ref() else {
        return json!({});
    };
    let Some(repo) = fs.repo.as_ref() else {
        return json!({});
    };

    if filter.is_empty() {
        return serde_json::to_value(repo).unwrap_or(Value::Null);
    }

    let packages = repo.packages.as_deref().unwrap_or(&[]);
    let filtered = filesystem::filter_packages(packages, filter);
    let filtered_packages: Vec<_> = filtered.into_iter().cloned().collect();

    let mut repo_clone = repo.clone();
    repo_clone.packages = Some(filtered_packages);
    serde_json::to_value(&repo_clone).unwrap_or(Value::Null)
}
```

### 3B. Wire into `build_with_outcome`

**File:** `sniff/cli/src/output/repo_json.rs`, line 93

Replace:

```rust
None | Some(RepoAction::Structure { .. }) => {
    BuildOutcome::pure(fallback_repo_value(result))
}
```

With:

```rust
None => BuildOutcome::pure(fallback_repo_value(result)),
Some(RepoAction::Structure { filter, .. }) => {
    BuildOutcome::pure(structure_value(result, filter))
}
```

This makes bare `sniff repo --json` (action = `None`) still use the unfiltered
fallback, while `sniff repo structure --filter X --json` applies the filter.

### 3C. `--latest-versions` preservation

The `enrich_result_dependencies` call happens in `commands.rs` at line 858,
*before* the JSON output path. Since `structure_value` serializes whatever is
in `result.filesystem.repo` at that point, the enrichment is automatically
preserved — the `DependencyEntry` fields (`latest_version`, `is_updatable`,
`has_major_update`) are already on the packages.

Add a code comment in `structure_value` to document this:

```rust
// `--latest-versions` enrichment is applied in commands.rs before
// this function is called, so `repo.packages` already carries the
// enriched `DependencyEntry` fields. We serialize the repo as-is.
```

### 3D. Add integration test for `structure --filter --json`

**File:** `sniff/cli/tests/cli.rs`

```rust
#[test]
fn test_repo_structure_filter_json_filters_packages() {
    let (_dir, path) = create_cli_monorepo();

    let assert_all = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "structure", "--json",
        ])
        .assert()
        .success();
    let stdout_all = String::from_utf8_lossy(&assert_all.get_output().stdout);
    let json_all: Value = serde_json::from_str(stdout_all.trim()).unwrap();
    let all_packages = json_all["packages"].as_array().unwrap();
    assert_eq!(all_packages.len(), 2, "unfiltered should have 2 packages");

    let assert_filtered = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "structure", "--json", "--filter", "pkg-a",
        ])
        .assert()
        .success();
    let stdout_filtered = String::from_utf8_lossy(&assert_filtered.get_output().stdout);
    let json_filtered: Value = serde_json::from_str(stdout_filtered.trim()).unwrap();
    let filtered_packages = json_filtered["packages"].as_array().unwrap();
    assert_eq!(
        filtered_packages.len(), 1,
        "filtered should have 1 package, got: {json_filtered}"
    );
    assert_eq!(filtered_packages[0]["name"], "pkg-a");
}
```

### Verification

```
cargo test -p sniff-cli --test cli structure_filter_json
cargo test -p sniff-cli output::repo_json::structure
cargo test -p sniff-cli --test cli json_shapes_are_distinct
```

---

## Phase 4 — Targeted integration-test coverage

**Goal.** Close every test-coverage gap listed in review-2 (items 1-7). Phase 1
already addressed item 1 (`--perf --json` tests for early-return paths). This
phase covers items 2-7.

**File:** `sniff/cli/tests/cli.rs`

### 4A. `package-area --json` resolving to a real area (item 2)

```rust
#[test]
fn test_package_area_json_resolves_to_real_area() {
    let (_dir, path) = create_cli_monorepo_distinct_area_and_package();
    let cwd = path.join("alpha/core");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", cwd.to_str().unwrap(),
            "repo", "package-area", "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(value["name"], Value::String("alpha".to_string()));
}
```

### 4B. `package-area-root --json` resolving to a real area (item 3)

```rust
#[test]
fn test_package_area_root_json_when_present() {
    let (_dir, path) = create_cli_monorepo_distinct_area_and_package();
    let cwd = path.join("alpha/core");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", cwd.to_str().unwrap(),
            "repo", "package-area-root", "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not JSON: {e}\n---\n{stdout}\n---"));
    let root = value["root"].as_str().expect("root must be string");
    assert!(root.contains("alpha"), "area root should contain 'alpha': {root}");
}
```

### 4C. `git-status --package <name> --json` (item 4)

```rust
#[test]
fn test_git_status_json_with_package_scope() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a2() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "git-status", "--package", "pkg-a", "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not JSON: {e}\n---\n{stdout}\n---"));

    assert!(
        value.get("repo_root").is_some(),
        "must have repo_root: {value}"
    );

    if let Some(file_changes) = value["file_changes"].as_array() {
        for fc in file_changes {
            let p = fc["path"].as_str().unwrap_or("");
            assert!(
                !p.starts_with("pkg-b/"),
                "pkg-scoped git-status must not contain pkg-b files: {p}"
            );
        }
    }
}
```

### 4D. `package`/`package-area` JSON when path resolves to empty name (item 5)

Covered by Phase 2D tests (`test_package_json_empty_name_stable_shape`,
`test_package_area_json_empty_name_stable_shape`).

### 4E. Boolean `true` branch tests (item 6)

The `false` branch is already tested. To test `true`, we need to create dirty
files in the package area and use `--refresh-remotes` to populate
`RepoStatus.dirty`.

```rust
#[test]
fn test_is_current_package_area_dirty_json_true_branch() {
    let (_dir, path) = create_cli_monorepo();
    test_commit_file(&path, "pkg-a/lib/src/lib.rs", "pub fn a() {}");

    std::fs::write(path.join("pkg-a/lib/src/lib.rs"), "pub fn a() { dirty }").unwrap();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.join("pkg-a/lib").to_str().unwrap(),
            "repo", "is-current-package-area-dirty", "--json",
        ])
        .assert();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not JSON: {e}\n---\n{stdout}\n---"));
    assert_eq!(
        value["dirty"], Value::Bool(true),
        "dirty area should emit dirty: true: {value}"
    );
}
```

Note: This test may need `--refresh-remotes` or a fixture that pre-populates
`RepoStatus.dirty`. If the detection path doesn't populate dirty files for the
non-deep plan, this test will need adjustment. The underlying
`current_package_area_is_dirty` checks `git.status.dirty` and
`git.status.untracked`, which should be populated even in non-deep mode
(basic `GitRequest::summary()` still fills `status`). Verify during
implementation.

### 4F. `repo structure --filter <pat> --json` test (item 7)

Covered by Phase 3D (`test_repo_structure_filter_json_filters_packages`).

### Verification

```
cargo test -p sniff-cli --test cli package_area_json_resolves
cargo test -p sniff-cli --test cli package_area_root_json_when
cargo test -p sniff-cli --test cli git_status_json_with_package_scope
cargo test -p sniff-cli --test cli is_current_package_area_dirty_json_true
cargo test -p sniff-cli --test cli structure_filter_json
```

---

## Phase 5 — Ergonomic cleanups and final validation

**Goal.** Address the minor ergonomic notes from review-2 and run the full
validation suite.

### 5A. Replace silent fallback in `build_deps_package_entry` with `.expect()`

**File:** `sniff/cli/src/output/repo_json.rs`, lines 273-289

Change the six `.unwrap_or_else(|_| Value::Array(vec![]))` calls to
`.expect("DependencyEntry serializes")` and the two `Vec<String>` ones to
`.expect("Vec<String> serializes")`:

```rust
serde_json::to_value(&pkg.depends_on).expect("Vec<String> serializes")
```

This converts a silent bug into a panic, which is correct for serialization of
types that are known to serialize cleanly.

### 5B. Move commit-family JSON construction into `repo_json.rs` (optional)

**Files:** `sniff/cli/src/output/recent_commits.rs`, `sniff/cli/src/output/repo_json.rs`

The review suggests moving the JSON construction from `recent_commits.rs` into
`repo_json.rs` so all repo JSON contract lives in one module. This is a
*nice-to-have* refactoring, not a blocker. If time permits:

1. Add `pub(crate) fn commit_centric_value(...)` to `repo_json.rs`
2. Call it from `recent_commits.rs`

**Decision:** Defer this to a follow-up if it would make the diff too large.
The current working code in `recent_commits.rs` is correct and well-tested.

### 5C. Consolidate `build_with_outcome` scope/kind strings (minor)

**File:** `sniff/cli/src/output/repo_json.rs`, lines 105-138

Consider a small enum:

```rust
enum PackageFamilyKind {
    Packages,
    PackageAreas,
}

impl PackageFamilyKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Packages => "packages",
            Self::PackageAreas => "package_areas",
        }
    }
}
```

Then the six arms collapse to two per scope. This is minor and can be deferred.

### 5D. Final validation

Run the full test and lint suite:

```
cargo test -p sniff -p sniff-cli
cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings
cargo fmt -p sniff -p sniff-cli --check
```

Verify all spec acceptance criteria:

1. Every `sniff repo` subcommand with `--json` returns JSON matching text mode
2. No two different subcommands return identical JSON (distinctness matrix)
3. Dirty/staged/unstaged package families return `{ scope, kind, names }`
4. `source-code-changes --json` filters to source-code commits/files
5. `documentation-changes --json` filters to documentation commits/files
6. Boolean subcommands return descriptive boolean keys with exit codes
7. Locator subcommands return focused JSON objects
8. All existing correct subcommands unchanged
9. `--perf` output works alongside all new JSON shapes

---

## Summary

| Phase | Focus | Blocker? | Files |
|-------|-------|----------|-------|
| **1** | `--perf` corruption fix + central helper | **Yes** | `commands.rs`, `recent_commits.rs`, `cli.rs` |
| **2** | Stable JSON for empty `package`/`package-area`/`root` | No | `commands.rs`, `repo_json.rs`, `cli.rs` |
| **3** | `structure --filter --json` + `--latest-versions` | No | `repo_json.rs`, `cli.rs` |
| **4** | Integration test coverage (7 gaps) | No | `cli.rs` |
| **5** | Ergonomic cleanups + final validation | No | `repo_json.rs` |

**Total: 5 phases.** Phase 1 is the blocker and should land first. Phases 2-4
are independent of each other and can be implemented in any order. Phase 5 is
cleanup and final gate.
