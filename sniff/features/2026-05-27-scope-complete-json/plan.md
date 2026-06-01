# Scope-Complete JSON Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the `sniff` CLI's `--json` output into compliance with the scope-complete JSON principle ([`sniff/docs/topics/json-output.md`](../../docs/topics/json-output.md)) by fixing two HIGH-severity violations in the `repo` tree, wiring three commit-family leaves that currently leak parent scope, adding three new sibling leaves to preserve information removed from `repo name`, and adding the `--with-network` global flag the principle doc references.

**Architecture:** Five focused changes across `args`, `commands`, and `output` modules. JSON shape changes are accompanied by terminal-form changes wherever the JSON scope narrows (reciprocal contract: terminal output must be a subset of the JSON scope at the same level). The implementation order: foundational `--with-network` flag → narrow `repo name` → add three new leaves → wire commit families → add parent `repo` aggregator → update help text. Each task is TDD: failing test → minimal impl → passing test → commit.

**Tech Stack:** Rust, clap v4 (CLI parsing), serde_json (JSON), assert_cmd + predicates (integration tests), `BuildOutcome` pattern (already established in `output/repo_json.rs`).

---

## File Structure

Files modified by this plan:

- **`sniff/cli/src/args/mod.rs`** — add `--with-network` field to `Cli` struct; update `REPO_AFTER_HELP` (in `args/mod.rs:1104`) to document the new leaves.
- **`sniff/cli/src/args/repo.rs`** — add three new variants to `RepoSubcommand` (`IsMonorepo`, `PackageCount`, `Version`) and three matching variants to `RepoAction`; extend `Commands::to_repo_action()` mapping (in `args/mod.rs`).
- **`sniff/cli/src/commands/mod.rs`** — fix `RepoAction::Name` handler (currently at lines 703-717) to route through `name_outcome()` for JSON; add handlers for three new leaves; add parent-aggregator dispatch for `cli.json && repo_subcommand.is_none()`.
- **`sniff/cli/src/output/repo_json.rs`** — add `is_monorepo_outcome()`, `package_count_outcome()`, `version_outcome()`, `commits_outcome()`, and `repo_aggregate_outcome()` builders; replace the wildcard `_` arm at line 288 with explicit arms for the three commit families.
- **`sniff/cli/src/output/filesystem/repo.rs`** — narrow `render_repo_name()` (lines 22-53) to emit only the name regardless of verbose level.
- **`sniff/cli/tests/cli.rs`** — add integration tests for every changed JSON contract, the three new leaves, and the `--with-network` flag.

No new files are created. No existing file is split.

---

## Task 1: Add `--with-network` global flag

**Files:**
- Modify: `sniff/cli/src/args/mod.rs:107` (insert after `--plain`, before `--completions`)
- Test: `sniff/cli/tests/cli.rs` (add new test in `subcommand_parsing` mod or top-level)

- [ ] **Step 1: Write the failing integration test**

Add at the end of `sniff/cli/tests/cli.rs` (or alongside existing flag tests):

```rust
#[test]
fn with_network_flag_parses() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("sniff").expect("sniff binary");
    // The flag should be accepted globally; pair with a fast subcommand
    // so the test doesn't pay full-detection cost.
    cmd.args(["--with-network", "repo", "name"])
        .assert()
        .success();
}

#[test]
fn with_network_flag_parses_before_json() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("sniff").expect("sniff binary");
    cmd.args(["--with-network", "repo", "name", "--json"])
        .assert()
        .success();
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sniff-cli --test cli with_network_flag -- --nocapture`

Expected: FAIL with clap error about unrecognized argument `--with-network`.

- [ ] **Step 3: Add the field to the `Cli` struct**

In `sniff/cli/src/args/mod.rs`, insert this block between the `plain: bool` field (currently at line 107) and the `completions: Option<Shell>` field (currently at line 109):

```rust
    /// Include network-dependent supplemental data (requires connectivity).
    /// Affects both terminal and JSON output. No effect on subcommands that
    /// do not currently consume network data — included as the well-known
    /// surface for future opt-in.
    #[arg(long, global = true)]
    pub with_network: bool,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p sniff-cli --test cli with_network_flag -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run the full CLI test suite to verify nothing else broke**

Run: `cargo test -p sniff-cli`

Expected: PASS (no regressions; the new global flag is invisible to other commands).

- [ ] **Step 6: Commit**

```bash
git add sniff/cli/src/args/mod.rs sniff/cli/tests/cli.rs
git commit -m "feat(sniff-cli): add --with-network global flag

Adds the surface referenced by the scope-complete JSON principle
(sniff/docs/topics/json-output.md). No subcommand currently consumes
the flag; included as the well-known opt-in for future
network-touching children."
```

---

## Task 2: Narrow `sniff repo name --json` to leaf scope

**Files:**
- Modify: `sniff/cli/src/commands/mod.rs:703-717` (the `RepoAction::Name` arm)
- Test: `sniff/cli/tests/cli.rs` (new test)
- Test: `sniff/cli/src/output/repo_json.rs` (existing unit test confirms `name_outcome()` shape; no change needed)

- [ ] **Step 1: Write the failing integration test**

Add to `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_name_json_is_leaf_only() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "name", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).expect("utf8");
    let json: serde_json::Value =
        serde_json::from_str(json_str).expect("valid json");

    let obj = json.as_object().expect("json should be an object");

    // The only key allowed is `name`. No version, language, is_monorepo,
    // or package_count may appear at the leaf level.
    assert_eq!(
        obj.len(),
        1,
        "repo name --json must contain exactly one key; got: {json}"
    );
    assert!(
        obj.contains_key("name"),
        "repo name --json must contain `name`: {json}"
    );
    assert!(
        obj.get("name").and_then(|v| v.as_str()).is_some(),
        "`name` must be a string: {json}"
    );

    for forbidden in ["version", "language", "is_monorepo", "package_count"] {
        assert!(
            !obj.contains_key(forbidden),
            "repo name --json must NOT contain `{forbidden}`: {json}"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sniff-cli --test cli repo_name_json_is_leaf_only -- --nocapture`

Expected: FAIL — the current handler serializes the full `RepoIdentity` struct, which contains `is_monorepo` and `package_count`. The assertion `obj.len() == 1` will fail.

- [ ] **Step 3: Fix the Name handler to use `name_outcome()`**

In `sniff/cli/src/commands/mod.rs`, replace the `RepoAction::Name` arm (currently at lines 703-717):

```rust
            crate::args::RepoAction::Name => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let identity = sniff::filesystem::repo::detect_repo_identity(dir)?;
                if cli.json {
                    let outcome = output::repo_json::name_outcome(identity.name.clone());
                    output::print_json_value(outcome.value, perf.build_report().as_ref());
                    if let Some(code) = outcome.exit_code {
                        std::process::exit(code);
                    }
                    return Ok(());
                }
                let rendered = output::render_repo_name(&identity, cli.verbose);
                output::emit_text(&rendered, cli.plain);
                perf.emit_stderr(None);
                return Ok(());
            }
```

The only change from today: replace `let json = serde_json::to_value(&identity)?;` (which serialized the full struct) with the focused `name_outcome()` builder. Exit-code handling preserves the `name_outcome()` contract (`exit_code = Some(1)` when the rendered name is empty).

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p sniff-cli --test cli repo_name_json_is_leaf_only -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS. Watch for any existing test that depended on `is_monorepo`/`package_count` appearing in `repo name --json` — if any exist, the test was asserting the bug and must be updated. (Audit suggests there are none, but verify.)

- [ ] **Step 6: Commit**

```bash
git add sniff/cli/src/commands/mod.rs sniff/cli/tests/cli.rs
git commit -m "fix(sniff-cli): narrow 'repo name --json' to leaf scope

The Name leaf was serializing the full RepoIdentity struct, leaking
version/language/is_monorepo/package_count fields outside its scope.
Routes through the existing name_outcome() builder which already
produces the correct { \"name\": ... } shape.

Violation of scope-complete JSON rule 4 (leaf returns only its own
scope). See sniff/docs/topics/json-output.md."
```

---

## Task 3: Narrow `sniff repo name -v` terminal output (reciprocal contract)

**Files:**
- Modify: `sniff/cli/src/output/filesystem/repo.rs:22-53` (the `render_repo_name` function)
- Test: `sniff/cli/tests/cli.rs` (new test)

- [ ] **Step 1: Write the failing terminal-form test**

Add to `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_name_verbose_terminal_shows_only_name() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "name", "-v", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = std::str::from_utf8(&output)
        .expect("utf8")
        .trim()
        .to_string();

    // After scope-complete narrowing: verbose form shows only the name
    // (no version suffix, no [language] suffix, no [N package monorepo]).
    // The name is a single token with no whitespace.
    assert!(
        !text.contains(' '),
        "repo name -v should be a single token (just the name), got: {text:?}"
    );
    assert!(
        !text.contains('['),
        "repo name -v should not contain '[' (suffix indicator), got: {text:?}"
    );
    assert!(
        !text.starts_with('v') && !text.contains(" v"),
        "repo name -v should not contain a version, got: {text:?}"
    );
    assert!(!text.is_empty(), "repo name -v should not be empty");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sniff-cli --test cli repo_name_verbose_terminal_shows_only_name -- --nocapture`

Expected: FAIL — current output decorates with version, language, or monorepo package count.

- [ ] **Step 3: Narrow `render_repo_name`**

In `sniff/cli/src/output/filesystem/repo.rs`, replace the entire `render_repo_name` function body (lines 22-53):

```rust
pub fn render_repo_name(identity: &RepoIdentity, _verbose: u8) -> String {
    format!("{}\n", identity.name)
}
```

The function signature is preserved (it has callers that pass `verbose`); `_verbose` is now unused. The `RepoIdentity` import remains; `Prose`, `Terminal`, and `format_number` imports may become unused — remove any that the compiler reports as unused.

- [ ] **Step 4: Clean up newly-unused imports**

Run: `cargo build -p sniff-cli 2>&1 | grep "unused import"`

Remove any imports the compiler flags. Typically expected: `Prose`, `Terminal`, and the helper used to format the package count (likely `format_number`).

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p sniff-cli --test cli repo_name_verbose_terminal_shows_only_name -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS. Existing tests of `render_repo_name` may fail if they asserted the decorated form — update them to assert the bare name. (Specifically look for any test in `sniff/cli/src/output/filesystem/repo.rs`'s `#[cfg(test)]` block.)

- [ ] **Step 7: Commit**

```bash
git add sniff/cli/src/output/filesystem/repo.rs sniff/cli/tests/cli.rs
git commit -m "fix(sniff-cli): narrow 'repo name -v' terminal to bare name

Reciprocal contract: terminal output must be a subset of the JSON
scope at the same level. JSON narrowed in previous commit;
terminal now also drops version/language/monorepo-count decorations.
Those fields are accessible via the new sibling leaves
(repo version, repo language, repo is-monorepo, repo package-count).

See sniff/docs/topics/terminal-output.md."
```

---

## Task 4: Add `sniff repo is-monorepo` leaf

**Files:**
- Modify: `sniff/cli/src/args/repo.rs` (add to `RepoSubcommand` and `RepoAction` enums)
- Modify: `sniff/cli/src/args/mod.rs` (extend `Commands::to_repo_action()` to map the new variant)
- Modify: `sniff/cli/src/output/repo_json.rs` (add `is_monorepo_outcome()` builder + unit test)
- Modify: `sniff/cli/src/commands/mod.rs` (add handler for `RepoAction::IsMonorepo`)
- Test: `sniff/cli/tests/cli.rs` (integration test)

- [ ] **Step 1: Write failing tests**

Unit test — add inside `#[cfg(test)] mod tests` block in `sniff/cli/src/output/repo_json.rs` (in the `locators_and_booleans` submodule alongside the existing builders):

```rust
        #[test]
        fn is_monorepo_outcome_true_exits_zero() {
            let outcome = is_monorepo_outcome(true);
            assert_eq!(outcome.value, json!({ "is-monorepo": true }));
            assert_eq!(outcome.exit_code, Some(0));
        }

        #[test]
        fn is_monorepo_outcome_false_exits_one() {
            let outcome = is_monorepo_outcome(false);
            assert_eq!(outcome.value, json!({ "is-monorepo": false }));
            assert_eq!(outcome.exit_code, Some(1));
        }
```

Integration test — add to `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_is_monorepo_subcommand_parses() {
    use assert_cmd::Command;
    Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "is-monorepo"])
        .assert()
        .success(); // rusty-biscuit IS a monorepo, so exit code 0
}

#[test]
fn repo_is_monorepo_json_shape() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "is-monorepo", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    let obj = json.as_object().expect("object");
    assert_eq!(obj.len(), 1, "leaf shape: only one key; got {json}");
    assert_eq!(
        obj.get("is-monorepo"),
        Some(&serde_json::Value::Bool(true))
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sniff-cli is_monorepo`

Expected: FAIL — neither the variant nor the builder exists yet.

- [ ] **Step 3: Add the `RepoSubcommand` variant**

In `sniff/cli/src/args/repo.rs`, inside the `RepoSubcommand` enum (anywhere before the closing `}`; conventional placement is right before `Name` at the end, since these are identity-adjacent leaves):

```rust
    /// Exit 0 if the repository is a monorepo, exit 1 otherwise
    #[command(name = "is-monorepo")]
    IsMonorepo,
```

- [ ] **Step 4: Add the `RepoAction` variant**

In the same file, in the `RepoAction` enum (parallel placement, before `Name`):

```rust
    IsMonorepo,
```

- [ ] **Step 5: Wire `to_repo_action()` mapping**

In `sniff/cli/src/args/mod.rs`, inside the `match repo_subcommand` block of `to_repo_action()`, add this arm (alongside the other simple no-arg arms like `Some(RepoSubcommand::Root) => RepoAction::Root,`):

```rust
                Some(RepoSubcommand::IsMonorepo) => RepoAction::IsMonorepo,
```

- [ ] **Step 6: Add the `is_monorepo_outcome()` builder**

In `sniff/cli/src/output/repo_json.rs`, add this function alongside the other simple outcome builders (e.g. near `name_outcome` at line 317):

```rust
pub(crate) fn is_monorepo_outcome(is_monorepo: bool) -> BuildOutcome {
    BuildOutcome::with_exit(
        json!({ "is-monorepo": is_monorepo }),
        if is_monorepo { 0 } else { 1 },
    )
}
```

- [ ] **Step 7: Add the handler in the dispatcher**

In `sniff/cli/src/commands/mod.rs`, alongside the other simple boolean-leaf handlers (e.g. near the `IsCurrentPackageAreaDirty` arm), add:

```rust
            crate::args::RepoAction::IsMonorepo => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let info = sniff::filesystem::repo::detect_repo_structure(dir)?
                    .ok_or("Not inside a recognized repository")?;
                let outcome = output::repo_json::is_monorepo_outcome(info.is_monorepo);
                if cli.json {
                    output::print_json_value(outcome.value, perf.build_report().as_ref());
                } else {
                    let text = if info.is_monorepo { "yes\n" } else { "no\n" };
                    output::emit_text(text, cli.plain);
                }
                perf.emit_stderr(None);
                std::process::exit(outcome.exit_code.unwrap_or(0));
            }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p sniff-cli is_monorepo`

Expected: PASS.

- [ ] **Step 9: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add sniff/cli/src/args/repo.rs sniff/cli/src/args/mod.rs \
        sniff/cli/src/output/repo_json.rs sniff/cli/src/commands/mod.rs \
        sniff/cli/tests/cli.rs
git commit -m "feat(sniff-cli): add 'sniff repo is-monorepo' leaf

New boolean leaf. Text: yes/no. JSON: { \"is-monorepo\": bool }.
Exit code 0 when true, 1 when false (standard boolean-leaf
convention matching is-current-package-area-dirty and
has-merge-conflict).

Surfaces information no longer in 'repo name' scope after the
leaf narrowing."
```

---

## Task 5: Add `sniff repo package-count` leaf

**Files:**
- Modify: `sniff/cli/src/args/repo.rs` (add `PackageCount` to `RepoSubcommand` and `RepoAction`)
- Modify: `sniff/cli/src/args/mod.rs` (extend `to_repo_action()`)
- Modify: `sniff/cli/src/output/repo_json.rs` (add `package_count_outcome()` + unit test)
- Modify: `sniff/cli/src/commands/mod.rs` (add handler)
- Test: `sniff/cli/tests/cli.rs` (integration test)

- [ ] **Step 1: Write failing tests**

Unit test in `sniff/cli/src/output/repo_json.rs`:

```rust
        #[test]
        fn package_count_outcome_with_count() {
            let outcome = package_count_outcome(Some(63));
            assert_eq!(outcome.value, json!({ "package-count": 63 }));
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn package_count_outcome_null_for_non_monorepo() {
            let outcome = package_count_outcome(None);
            assert_eq!(
                outcome.value,
                json!({ "package-count": serde_json::Value::Null })
            );
            assert!(outcome.exit_code.is_none());
        }
```

Integration test in `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_package_count_json_shape() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "package-count", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    let obj = json.as_object().expect("object");
    assert_eq!(obj.len(), 1, "leaf shape: only one key; got {json}");
    assert!(
        obj.contains_key("package-count"),
        "missing key: {json}"
    );
    assert!(
        obj.get("package-count").and_then(|v| v.as_u64()).is_some(),
        "package-count should be a number in this monorepo: {json}"
    );
}

#[test]
fn repo_package_count_text_form_is_numeric() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "package-count", "--plain"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = std::str::from_utf8(&output).unwrap().trim();
    assert!(
        text.chars().all(|c| c.is_ascii_digit()),
        "package-count text should be plain digits, got: {text:?}"
    );
    let _: usize = text.parse().expect("must parse as usize");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sniff-cli package_count`

Expected: FAIL — variant and builder do not exist.

- [ ] **Step 3: Add `RepoSubcommand::PackageCount` variant**

In `sniff/cli/src/args/repo.rs`, inside `RepoSubcommand` (placement: right after `IsMonorepo`):

```rust
    /// Output the number of packages in the monorepo
    #[command(name = "package-count")]
    PackageCount,
```

- [ ] **Step 4: Add `RepoAction::PackageCount` variant**

In the same file, in `RepoAction` (right after `IsMonorepo`):

```rust
    PackageCount,
```

- [ ] **Step 5: Wire `to_repo_action()` mapping**

In `sniff/cli/src/args/mod.rs`:

```rust
                Some(RepoSubcommand::PackageCount) => RepoAction::PackageCount,
```

- [ ] **Step 6: Add the `package_count_outcome()` builder**

In `sniff/cli/src/output/repo_json.rs`, near `name_outcome`:

```rust
pub(crate) fn package_count_outcome(count: Option<usize>) -> BuildOutcome {
    let value = match count {
        Some(n) => json!({ "package-count": n }),
        None => json!({ "package-count": serde_json::Value::Null }),
    };
    BuildOutcome::pure(value)
}
```

- [ ] **Step 7: Add the handler**

In `sniff/cli/src/commands/mod.rs`, alongside the other simple value-leaf handlers:

```rust
            crate::args::RepoAction::PackageCount => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let info = sniff::filesystem::repo::detect_repo_structure(dir)?
                    .ok_or("Not inside a recognized repository")?;
                let count = info
                    .packages
                    .as_ref()
                    .map(|p| p.len());
                if cli.json {
                    let outcome = output::repo_json::package_count_outcome(count);
                    output::print_json_value(outcome.value, perf.build_report().as_ref());
                } else {
                    let text = match count {
                        Some(n) => format!("{n}\n"),
                        None => "0\n".to_string(),
                    };
                    output::emit_text(&text, cli.plain);
                }
                perf.emit_stderr(None);
                return Ok(());
            }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p sniff-cli package_count`

Expected: PASS.

- [ ] **Step 9: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add sniff/cli/src/args/repo.rs sniff/cli/src/args/mod.rs \
        sniff/cli/src/output/repo_json.rs sniff/cli/src/commands/mod.rs \
        sniff/cli/tests/cli.rs
git commit -m "feat(sniff-cli): add 'sniff repo package-count' leaf

New value leaf. Text: count as plain digits. JSON:
{ \"package-count\": <number> | null }.

Non-monorepo repos return null (and text \"0\") rather than
erroring, matching the convention that scope-complete data
surfaces the answer even when uninteresting."
```

---

## Task 6: Add `sniff repo version` leaf

**Files:**
- Modify: `sniff/cli/src/args/repo.rs` (add `Version` to `RepoSubcommand` and `RepoAction`)
- Modify: `sniff/cli/src/args/mod.rs` (extend `to_repo_action()`)
- Modify: `sniff/cli/src/output/repo_json.rs` (add `version_outcome()` + unit test)
- Modify: `sniff/cli/src/commands/mod.rs` (add handler)
- Test: `sniff/cli/tests/cli.rs` (integration test)

- [ ] **Step 1: Write failing tests**

Unit test in `sniff/cli/src/output/repo_json.rs`:

```rust
        #[test]
        fn version_outcome_with_string() {
            let outcome = version_outcome(Some("0.1.0".to_string()));
            assert_eq!(outcome.value, json!({ "version": "0.1.0" }));
            assert!(outcome.exit_code.is_none());
        }

        #[test]
        fn version_outcome_null_when_absent() {
            let outcome = version_outcome(None);
            assert_eq!(
                outcome.value,
                json!({ "version": serde_json::Value::Null })
            );
            assert!(outcome.exit_code.is_none());
        }
```

Integration test in `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_version_json_shape() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "version", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    let obj = json.as_object().expect("object");
    assert_eq!(obj.len(), 1, "leaf shape: only one key; got {json}");
    assert!(obj.contains_key("version"), "missing key: {json}");
    // Value can be a string or null; both are valid.
    let v = obj.get("version").unwrap();
    assert!(
        v.is_string() || v.is_null(),
        "version must be string or null: {json}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sniff-cli version_outcome` and `cargo test -p sniff-cli repo_version`

Expected: FAIL.

- [ ] **Step 3: Add `RepoSubcommand::Version` variant**

In `sniff/cli/src/args/repo.rs`, inside `RepoSubcommand` (placement: right after `PackageCount`):

```rust
    /// Output the version string from the root manifest, if present
    Version,
```

- [ ] **Step 4: Add `RepoAction::Version` variant**

In the same file, in `RepoAction` (right after `PackageCount`):

```rust
    Version,
```

- [ ] **Step 5: Wire `to_repo_action()` mapping**

In `sniff/cli/src/args/mod.rs`:

```rust
                Some(RepoSubcommand::Version) => RepoAction::Version,
```

- [ ] **Step 6: Add the `version_outcome()` builder**

In `sniff/cli/src/output/repo_json.rs`, near `name_outcome`:

```rust
pub(crate) fn version_outcome(version: Option<String>) -> BuildOutcome {
    let value = match version {
        Some(v) => json!({ "version": v }),
        None => json!({ "version": serde_json::Value::Null }),
    };
    BuildOutcome::pure(value)
}
```

- [ ] **Step 7: Add the handler**

In `sniff/cli/src/commands/mod.rs`:

```rust
            crate::args::RepoAction::Version => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let identity = sniff::filesystem::repo::detect_repo_identity(dir)?;
                if cli.json {
                    let outcome = output::repo_json::version_outcome(identity.version.clone());
                    output::print_json_value(outcome.value, perf.build_report().as_ref());
                } else {
                    let text = identity
                        .version
                        .map(|v| format!("{v}\n"))
                        .unwrap_or_default(); // empty text when no version
                    output::emit_text(&text, cli.plain);
                }
                perf.emit_stderr(None);
                return Ok(());
            }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p sniff-cli version_outcome` and `cargo test -p sniff-cli repo_version`

Expected: PASS.

- [ ] **Step 9: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add sniff/cli/src/args/repo.rs sniff/cli/src/args/mod.rs \
        sniff/cli/src/output/repo_json.rs sniff/cli/src/commands/mod.rs \
        sniff/cli/tests/cli.rs
git commit -m "feat(sniff-cli): add 'sniff repo version' leaf

New value leaf. Text: version string or empty. JSON:
{ \"version\": <string> | null }.

Surfaces the version field that used to be bundled into the
'repo name' verbose output and JSON."
```

---

## Task 7: Wire focused JSON builder for `repo recent-commits`

**Files:**
- Modify: `sniff/cli/src/output/repo_json.rs` (add `commits_outcome()` helper; replace the wildcard `_` arm at line 288 with an explicit `RecentCommits` arm)
- Test: `sniff/cli/tests/cli.rs` (integration test asserts the JSON does NOT contain `RepoInfo` top-level keys)

- [ ] **Step 1: Write the failing integration test**

Add to `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_recent_commits_json_is_scope_complete() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "recent-commits", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    let obj = json.as_object().expect("object");

    // The new focused shape must contain `commits`. May also contain
    // `period` and any filter context. Must NOT contain RepoInfo-only
    // fields (is_monorepo, packages, dependencies, etc.).
    assert!(
        obj.contains_key("commits"),
        "recent-commits JSON must contain `commits` key: {json}"
    );
    for leaked in [
        "is_monorepo",
        "packages",
        "dependencies",
        "dev_dependencies",
        "monorepo_tool",
        "workspace_tools",
    ] {
        assert!(
            !obj.contains_key(leaked),
            "recent-commits JSON must NOT leak RepoInfo key `{leaked}`: {json}"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sniff-cli repo_recent_commits_json_is_scope_complete -- --nocapture`

Expected: FAIL — current wildcard arm falls through to `fallback_repo_value`, which returns the entire `RepoInfo` blob (including `is_monorepo`, `packages`, etc.).

- [ ] **Step 3: Add the `commits_outcome()` builder**

In `sniff/cli/src/output/repo_json.rs`, near the other outcome builders. This builder is shared across `RecentCommits`, `SourceCodeChanges`, and `DocumentationChanges` (DRY — they differ only in which commits are filtered upstream):

```rust
pub(crate) fn commits_outcome(
    period: Option<&str>,
    actions: &[String],
    commits: Vec<serde_json::Value>,
) -> BuildOutcome {
    BuildOutcome::pure(json!({
        "period": period,
        "actions": actions,
        "commits": commits,
    }))
}
```

- [ ] **Step 4: Replace the wildcard arm with the `RecentCommits` arm**

In `sniff/cli/src/output/repo_json.rs`, the current match block has (at line 285-289):

```rust
        // All other actions fall through to today's behavior. Later phases
        // (6) replace these fall-throughs with focused builders for the
        // commit families.
        _ => BuildOutcome::pure(fallback_repo_value(result)),
    }
```

Replace this with an explicit arm for `RecentCommits` (the other two follow in subsequent tasks; for now the wildcard `_` stays to catch them):

```rust
        Some(RepoAction::RecentCommits { period, actions, .. }) => {
            // Source of commits comes from result.filesystem.git (already
            // populated by the upstream detection plan). Map each commit
            // to a focused JSON object — see output/recent_commits.rs for
            // the parallel terminal renderer's data shape.
            let commits = result
                .filesystem
                .as_ref()
                .and_then(|fs| fs.git.as_ref())
                .map(|git| git.recent.iter().map(|c| serde_json::to_value(c).unwrap_or(json!({}))).collect())
                .unwrap_or_default();
            let actions_str: Vec<String> = actions.iter().map(|a| a.as_str().to_string()).collect();
            commits_outcome(period.as_deref(), &actions_str, commits)
        }
        // All other actions fall through to today's behavior. Later phases
        // replace these fall-throughs with focused builders.
        _ => BuildOutcome::pure(fallback_repo_value(result)),
    }
```

**Note for implementer:** the exact field path (`fs.git.recent`) is the audit's best guess based on existing patterns. Verify by reading `output/recent_commits.rs` to see which field of `git` (or `result`) the terminal renderer reads from. If the field name differs, use the same source the terminal renderer uses — they must agree.

If the recent-commits data is filtered by `actions` upstream, the focused builder can pass the already-filtered list through. If filtering happens at render time, port that filtering here.

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p sniff-cli repo_recent_commits_json_is_scope_complete -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add sniff/cli/src/output/repo_json.rs sniff/cli/tests/cli.rs
git commit -m "fix(sniff-cli): scope 'repo recent-commits --json' to commits

Was falling through to fallback_repo_value which serializes the
entire RepoInfo blob (Rule 4 violation: leaf leaking parent scope).

Adds the shared commits_outcome() builder and an explicit
RecentCommits arm. SourceCodeChanges and DocumentationChanges
(also currently in the fall-through) are wired in subsequent
commits."
```

---

## Task 8: Wire focused JSON builder for `repo source-code-changes`

**Files:**
- Modify: `sniff/cli/src/output/repo_json.rs` (add the `SourceCodeChanges` arm)
- Test: `sniff/cli/tests/cli.rs` (integration test)

- [ ] **Step 1: Write the failing integration test**

Add to `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_source_code_changes_json_is_scope_complete() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "source-code-changes", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    let obj = json.as_object().expect("object");

    assert!(obj.contains_key("commits"), "missing commits key: {json}");
    for leaked in ["is_monorepo", "packages", "dependencies", "monorepo_tool"] {
        assert!(
            !obj.contains_key(leaked),
            "source-code-changes JSON must NOT leak RepoInfo key `{leaked}`: {json}"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sniff-cli repo_source_code_changes_json_is_scope_complete -- --nocapture`

Expected: FAIL — still in the wildcard fall-through.

- [ ] **Step 3: Add the `SourceCodeChanges` arm**

In `sniff/cli/src/output/repo_json.rs`, immediately after the `RecentCommits` arm added in Task 7:

```rust
        Some(RepoAction::SourceCodeChanges { period, actions, .. }) => {
            // Same shape as RecentCommits — upstream is responsible for
            // filtering to source-code-touching commits.
            let commits = result
                .filesystem
                .as_ref()
                .and_then(|fs| fs.git.as_ref())
                .map(|git| {
                    git.source_code_changes
                        .iter()
                        .map(|c| serde_json::to_value(c).unwrap_or(json!({})))
                        .collect()
                })
                .unwrap_or_default();
            let actions_str: Vec<String> =
                actions.iter().map(|a| a.as_str().to_string()).collect();
            commits_outcome(period.as_deref(), &actions_str, commits)
        }
```

**Note for implementer:** verify the exact field name (`source_code_changes` vs whatever the lib actually exposes) by reading `output/recent_commits.rs` for the data source the terminal renderer uses. Mirror that.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p sniff-cli repo_source_code_changes_json_is_scope_complete -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add sniff/cli/src/output/repo_json.rs sniff/cli/tests/cli.rs
git commit -m "fix(sniff-cli): scope 'repo source-code-changes --json' to commits

Mirrors the previous RecentCommits fix. Reuses commits_outcome()
shared builder."
```

---

## Task 9: Wire focused JSON builder for `repo documentation-changes`

**Files:**
- Modify: `sniff/cli/src/output/repo_json.rs` (add the `DocumentationChanges` arm)
- Test: `sniff/cli/tests/cli.rs` (integration test)

- [ ] **Step 1: Write the failing integration test**

Add to `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_documentation_changes_json_is_scope_complete() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "documentation-changes", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    let obj = json.as_object().expect("object");

    assert!(obj.contains_key("commits"), "missing commits key: {json}");
    for leaked in ["is_monorepo", "packages", "dependencies", "monorepo_tool"] {
        assert!(
            !obj.contains_key(leaked),
            "documentation-changes JSON must NOT leak RepoInfo key `{leaked}`: {json}"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sniff-cli repo_documentation_changes_json_is_scope_complete -- --nocapture`

Expected: FAIL.

- [ ] **Step 3: Add the `DocumentationChanges` arm**

In `sniff/cli/src/output/repo_json.rs`, immediately after the `SourceCodeChanges` arm:

```rust
        Some(RepoAction::DocumentationChanges { period, actions, .. }) => {
            let commits = result
                .filesystem
                .as_ref()
                .and_then(|fs| fs.git.as_ref())
                .map(|git| {
                    git.documentation_changes
                        .iter()
                        .map(|c| serde_json::to_value(c).unwrap_or(json!({})))
                        .collect()
                })
                .unwrap_or_default();
            let actions_str: Vec<String> =
                actions.iter().map(|a| a.as_str().to_string()).collect();
            commits_outcome(period.as_deref(), &actions_str, commits)
        }
```

Same field-name verification note as Task 8.

After this task, the wildcard `_ => BuildOutcome::pure(fallback_repo_value(result))` at the bottom of the match should no longer hide the three commit families. It will still catch any other action variants that don't have explicit arms — leave it in place as the catch-all (other future leaves will be wired explicitly).

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p sniff-cli repo_documentation_changes_json_is_scope_complete -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add sniff/cli/src/output/repo_json.rs sniff/cli/tests/cli.rs
git commit -m "fix(sniff-cli): scope 'repo documentation-changes --json' to commits

Completes the trio of commit-family fixes. All three families
(recent-commits, source-code-changes, documentation-changes) now
emit { period, actions, commits } only — no longer leak RepoInfo
parent scope via the fallback path."
```

---

## Task 10: Add `sniff repo --json` parent aggregator

**Files:**
- Modify: `sniff/cli/src/output/repo_json.rs` (add `repo_aggregate_outcome()` builder)
- Modify: `sniff/cli/src/commands/mod.rs` (intercept `cli.json && repo_subcommand.is_none()` BEFORE dispatching to `RepoAction::Name`)
- Test: `sniff/cli/tests/cli.rs` (integration tests for the aggregate shape and round-trip)

- [ ] **Step 1: Write the failing integration tests**

Add to `sniff/cli/tests/cli.rs`:

```rust
#[test]
fn repo_json_returns_aggregate_not_name() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    let obj = json.as_object().expect("object");

    // The aggregate must contain all the core repo-scope keys. Keys are
    // kebab-case to match subcommand names.
    for required in [
        "name",
        "is-monorepo",
        "package-count",
        "version",
        "language",
        "structure",
        "packages",
        "package-areas",
        "worktrees",
        "root",
    ] {
        assert!(
            obj.contains_key(required),
            "repo --json aggregate must contain `{required}`: {json}"
        );
    }

    // The bug: the old behavior dispatched to `name` and returned
    // RepoIdentity fields at top level. Now `name` is just a key
    // pointing to the leaf scope (`{ \"name\": ... }` or the string).
    // Make sure the top level is not a single-key Name leaf.
    assert!(
        obj.len() > 1,
        "repo --json must not be a single-key Name leaf; got {json}"
    );
}

#[test]
fn repo_json_aggregate_keys_round_trip_to_subcommands() {
    use assert_cmd::Command;
    let output = Command::cargo_bin("sniff")
        .expect("sniff binary")
        .args(["repo", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&output).unwrap()).unwrap();
    let obj = json.as_object().expect("object");

    // For each top-level key, `sniff repo <key>` must parse as a valid
    // subcommand (success exit). This is the round-trip contract:
    // aggregate keys map back 1:1 to drillable subcommands.
    for key in obj.keys() {
        let mut cmd = Command::cargo_bin("sniff").expect("sniff binary");
        let assertion = cmd.args(["repo", key, "--help"]).assert();
        assertion.success();
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sniff-cli repo_json_returns_aggregate_not_name -- --nocapture`

Expected: FAIL — currently `sniff repo --json` dispatches to `RepoAction::Name` and returns a focused `{ "name": ... }` after Task 2's fix (or the bundled identity before Task 2). Either way, it does not contain the broader aggregate keys.

- [ ] **Step 3: Add the `repo_aggregate_outcome()` builder**

In `sniff/cli/src/output/repo_json.rs`, near the other outcome builders. This is the central change of the task:

```rust
/// Builds the aggregate JSON for `sniff repo --json` (no subcommand).
///
/// Combines the scope of every direct `repo` child into one object,
/// keyed by kebab-case subcommand name. Each key maps 1:1 to a
/// subcommand the user could invoke directly.
///
/// Scope boundaries: this aggregate includes only local children.
/// Network-touching children (e.g. `pr`) are excluded unless
/// `with_network` is true.
pub(crate) fn repo_aggregate_outcome(
    result: &SniffResult,
    base_dir: Option<&std::path::Path>,
    _with_network: bool,
) -> BuildOutcome {
    let dir = base_dir.unwrap_or_else(|| std::path::Path::new("."));

    // Identity-side leaves — call detect_repo_identity directly because
    // these fields are not all present on RepoInfo.
    let identity = sniff::filesystem::repo::detect_repo_identity(dir).ok();

    // Structure-side data — taken from the already-populated SniffResult
    // to avoid redundant detection work.
    let repo_info = result.filesystem.as_ref().map(|fs| &fs.repo);

    // Compose each child scope using the existing focused builders where
    // available, falling back to direct JSON construction for fields
    // exposed only by RepoInfo or RepoIdentity.
    let name = identity.as_ref().map(|i| i.name.clone()).unwrap_or_default();
    let is_monorepo = repo_info.map(|r| r.is_monorepo).unwrap_or(false);
    let package_count = repo_info
        .and_then(|r| r.packages.as_ref().map(|p| p.len()));
    let version = identity.as_ref().and_then(|i| i.version.clone());
    let language = identity.as_ref().and_then(|i| i.language.clone());

    let structure = repo_info
        .map(|r| serde_json::to_value(r).unwrap_or(json!({})))
        .unwrap_or(json!({}));

    let packages: Vec<String> = repo_info
        .and_then(|r| r.packages.as_ref())
        .map(|pkgs| pkgs.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default();

    let package_areas: Vec<String> = repo_info
        .and_then(|r| r.packages.as_ref())
        .map(|pkgs| {
            let mut set: std::collections::BTreeSet<String> =
                pkgs.iter().map(|p| p.package_area.clone()).collect();
            set.into_iter().collect()
        })
        .unwrap_or_default();

    let worktrees: Vec<serde_json::Value> = sniff::filesystem::git::list_worktrees(dir)
        .unwrap_or_default()
        .into_iter()
        .map(|wt| serde_json::to_value(wt).unwrap_or(json!({})))
        .collect();

    let root = repo_info
        .map(|r| r.root.display().to_string())
        .unwrap_or_default();

    BuildOutcome::pure(json!({
        "name": name,
        "is-monorepo": is_monorepo,
        "package-count": package_count,
        "version": version,
        "language": language,
        "structure": structure,
        "packages": packages,
        "package-areas": package_areas,
        "worktrees": worktrees,
        "root": root,
    }))
}
```

**Note for implementer:** the exact function names for worktree listing (`list_worktrees`) and any unexposed fields may differ. The audit confirmed `sniff::filesystem::git::*` is the right module; verify the precise function name by reading the existing `Worktrees` action handler. The shape above is the *contract*; the *data sources* may require minor tweaks for compilation. If a data source is genuinely unavailable, return `null`/empty for that key rather than failing the whole aggregate.

**Scope note:** the network flag is accepted as `_with_network` but does not currently change behavior because no aggregated child requires network. When a future child opts in, replace the underscore and gate that child's inclusion.

- [ ] **Step 4: Intercept the dispatcher**

In `sniff/cli/src/commands/mod.rs`, find the section where `Commands::Repo { repo_subcommand }` is handled and `to_repo_action()` is called. Before that dispatch (or as an early arm), add:

```rust
        // Scope-complete JSON: at the bare `repo` level, JSON ignores the
        // default subcommand and returns the aggregate of all child scopes.
        // Terminal output (no --json) continues to dispatch to the default
        // `name` subcommand for human-friendly output.
        Commands::Repo {
            repo_subcommand: None,
        } if cli.json => {
            // Run the full detection plan so the aggregate has access to
            // all the data its children need.
            let dir = cli
                .base
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new("."));
            let plan = sniff::DetectionPlan::default(); // adjust if a specific
                                                       // plan is conventional
                                                       // — see the Structure
                                                       // handler for reference
            let result = sniff::detect_with_plan(dir, &plan).await?;
            let outcome = output::repo_json::repo_aggregate_outcome(
                &result,
                cli.base.as_deref(),
                cli.with_network,
            );
            output::print_json_value(outcome.value, perf.build_report().as_ref());
            perf.emit_stderr(None);
            return Ok(());
        }
```

**Note for implementer:** the exact placement depends on the dispatcher's current `match cli.command` shape. Place the new arm *before* the general `Commands::Repo { repo_subcommand }` arm so it matches first. The `cli.json` guard ensures it only applies to the JSON path; the text path falls through to the existing dispatcher logic.

**Note on `DetectionPlan`:** the existing `Structure` handler shows the conventional plan setup. Mirror it. If the codebase uses `sniff::detect()` (no plan) for "everything", use that instead.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p sniff-cli repo_json_returns_aggregate_not_name -- --nocapture`
Run: `cargo test -p sniff-cli repo_json_aggregate_keys_round_trip_to_subcommands -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS.

If any pre-existing test asserted the *old* `sniff repo --json` behavior (i.e. asserted it returned `{ is_monorepo, name, package_count }`), it was asserting the bug. Update it to assert the new aggregate shape — and add a comment noting the change.

- [ ] **Step 7: Commit**

```bash
git add sniff/cli/src/output/repo_json.rs sniff/cli/src/commands/mod.rs \
        sniff/cli/tests/cli.rs
git commit -m "fix(sniff-cli): aggregate child scopes for 'sniff repo --json'

The canonical scope-complete JSON bug: 'sniff repo --json' was
dispatching to the default 'name' subcommand and returning leaf-scope
data instead of the parent's aggregate. Default subcommands are
human-output-only; JSON must return the scope of the node typed.

Adds repo_aggregate_outcome() which composes name, is-monorepo,
package-count, version, language, structure, packages, package-areas,
worktrees, and root into one keyed object. Top-level keys are
kebab-case and map 1:1 to subcommands users can invoke directly to
drill in.

Round-trip tested: each aggregate key resolves as a valid subcommand.

See sniff/docs/topics/json-output.md for the principle and
sniff/features/2026-05-27-scope-complete-json/spec.md for the design."
```

---

## Task 11: Update help text — REPO_AFTER_HELP and main AFTER_HELP

**Files:**
- Modify: `sniff/cli/src/args/mod.rs:1104` (the `REPO_AFTER_HELP` const) — add the three new leaves under "Identity:"

- [ ] **Step 1: Update `REPO_AFTER_HELP`**

In `sniff/cli/src/args/mod.rs`, find the `REPO_AFTER_HELP` constant (starts at line 1104) and modify the `Identity:` section to include the new leaves:

```rust
pub const REPO_AFTER_HELP: &str = "\
Identity:
  sniff repo name                     Repository name (plain text)
  sniff repo version                  Repository version, if present
  sniff repo is-monorepo              Exit 0 if monorepo, 1 otherwise
  sniff repo package-count            Number of packages in the monorepo
  sniff repo --json                   Full repo scope as JSON (aggregate)

Structure:
  sniff repo structure                Show repository/monorepo structure
...
";
```

Keep the rest of the help text unchanged. Note the addition of `sniff repo --json` to make the aggregate behavior discoverable.

- [ ] **Step 2: Build and verify help text renders**

Run: `cargo build -p sniff-cli`

Run: `cargo run -p sniff-cli -- repo --help`

Expected: PASS. The new leaves appear in the Identity section.

- [ ] **Step 3: Run the full CLI test suite**

Run: `cargo test -p sniff-cli`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add sniff/cli/src/args/mod.rs
git commit -m "docs(sniff-cli): document new repo leaves and aggregate JSON

Adds the three new leaves (version, is-monorepo, package-count) and
calls out the aggregate JSON form of 'sniff repo --json' in
REPO_AFTER_HELP."
```

---

## Task 12: Final verification

- [ ] **Step 1: Run the full test suite**

Run: `cargo test -p sniff-cli`
Run: `cargo test -p sniff`

Expected: PASS.

- [ ] **Step 2: Run lint**

Run: `cargo clippy -p sniff-cli -- -D warnings`
Run: `cargo clippy -p sniff -- -D warnings`

Expected: no warnings.

- [ ] **Step 3: Run fmt**

Run: `cargo fmt --check`

Expected: no diffs.

- [ ] **Step 4: Manual smoke test**

Run the following commands and visually verify the output matches the spec:

```bash
cargo run -p sniff-cli -- repo --json | jq 'keys'
# Expected: array containing "name", "is-monorepo", "package-count",
# "version", "language", "structure", "packages", "package-areas",
# "worktrees", "root"

cargo run -p sniff-cli -- repo name --json
# Expected: {"name": "rusty-biscuit"}

cargo run -p sniff-cli -- repo name -v
# Expected: rusty-biscuit (just the name; no decorations)

cargo run -p sniff-cli -- repo is-monorepo
# Expected: yes
echo "exit=$?"
# Expected: exit=0

cargo run -p sniff-cli -- repo package-count
# Expected: 63 (or similar numeric)

cargo run -p sniff-cli -- repo version --json
# Expected: {"version": null} (rusty-biscuit root has no version) or string

cargo run -p sniff-cli -- repo recent-commits --json | jq 'keys'
# Expected: ["actions", "commits", "period"] — no is_monorepo, no packages

cargo run -p sniff-cli -- --with-network repo --json | jq 'keys'
# Expected: same keys as repo --json (flag accepted; no network children yet)
```

- [ ] **Step 5: Final commit (if any drift fixes needed)**

If any test, lint, or fmt fix is needed, fix and commit:

```bash
git add -A
git commit -m "chore(sniff-cli): post-feature cleanup (lint/fmt)"
```

---

## Out of scope (explicit deferrals, captured in spec)

- Broader CLI-wide audit for terminal-output subset compliance (every command not just the ones we touched).
- Design-philosophy rollout for terminal output (renderable adoption, two-tier guidance, biscuit-terminal/darkmatter component patterns).
- A richer `sniff repo -v` aggregate verbose terminal form built from the new aggregate scope.
- Adding network-touching children that actually consume `--with-network`.
- Adding the file-list family (`dirty-files`, `staged-files`, etc.) and the commit families to the `sniff repo --json` aggregate. They remain reachable as direct subcommands. Adding them to the aggregate is straightforward (extend `repo_aggregate_outcome()`) and can be done as a separate small change once we see usage patterns.
