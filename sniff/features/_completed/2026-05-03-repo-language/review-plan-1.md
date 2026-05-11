---
phases: 3
created: 2026-05-03
review: review-1.md
spec: spec.md
package_area: sniff
crate: sniff-cli
start_phase: 1
source_files_during_phase_1: [sniff/cli/src/output/mod.rs, sniff/cli/src/output/filesystem.rs, sniff/cli/src/output/repo_json.rs, sniff/cli/src/args.rs]
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: [sniff/cli/tests/cli.rs]
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
---

# Review-1 Remediation Plan: `sniff repo language`

This plan closes the gaps identified in `review-1.md` for the `sniff repo language`
feature. It is sequenced so that the **behaviour change + help text land first**,
the **Level 1 CLI tests are written against the now-locked behaviour**, and the
**final phase is a clean lint/test sweep** of the `sniff` package area.

The Critical (perf) finding has already been addressed in the working tree:
`sniff/cli/src/commands.rs` now uses
`FilesystemRequest::new().git(GitRequest::summary()).without_repo().without_docs().without_formatting()`
for `RepoAction::Language` (verified via `git diff` at plan-creation time). Phase 3
re-verifies this, but no code change is required for it.

---

## Product Decision: No-language behaviour (Medium finding)

### Decision

**Both text and JSON exit non-zero (status `1`) when no primary language can be
determined. JSON additionally emits the stable shape `{ "language": null }` so
scripts always see a parseable object. Text mode emits empty stdout (no
sentinel).**

Concretely:

| Mode | Found             | Not found                             |
| ---- | ----------------- | ------------------------------------- |
| text | `Rust\n`, exit 0  | empty stdout, **exit 1**              |
| json | `{"language":"Rust"}\n`, exit 0 | `{"language":null}\n`, **exit 1** |

### Rationale

1. **Consistency with the closest existing pattern.** `sniff repo language`
   is, semantically, a single-token locator: "give me the one identifier for
   this repo." The existing locator family — `sniff repo root`,
   `sniff repo package`, `sniff repo package-area`, `sniff repo package-root`,
   `sniff repo package-area-root` — already implements exactly this contract.
   See `sniff/cli/src/output/mod.rs:512-535` (text path: empty rendered →
   `std::process::exit(1)`) and `sniff/cli/src/output/repo_json.rs:196-205`
   (`locator_root_outcome`: empty rendered → `BuildOutcome` with
   `exit_code = Some(1)` while still emitting a stable JSON shape). Reusing
   this shape means `language` slots into a family the user already knows.

2. **No sentinel string in text mode.** A literal `(unknown)` or `none` token
   pollutes the namespace of "real" language names — a future
   detector that returns "Unknown" or "None" as a real value would collide.
   Exit code is the unambiguous signal; stdout stays clean for piping.

3. **Stable JSON shape.** Keeping `{"language":null}` (already implemented
   in `repo_json.rs:180-182`) means JSON consumers never have to handle a
   missing key, only a null value. Adding `exit_code = 1` on null mirrors
   the locator family and lets shells use `$?` without parsing JSON.

4. **No stderr message.** Other locator commands stay silent on the empty
   case; introducing a stderr line would diverge for no scripting benefit.

### What this does NOT change

- The shape of `render_repo_language` (still returns empty string on absence)
  stays the same — the exit-code decision is made by the caller, exactly like
  the existing locator path. This keeps the rendering function pure.
- `--no-error` / `--on-error` are **not** wired into this command in this
  plan. Those flags are only consumed by the dirty/has-source-code-changes
  family today. If they are wanted on `language` later, they can be layered
  on without breaking the contract decided here.

---

## Phase 1: Behaviour change + help text

**Objective:** Lock in the no-language exit-code semantics in both text and
JSON, and add `sniff repo language` to the curated repo help examples.

**Files modified:**

- `sniff/cli/src/output/mod.rs`
- `sniff/cli/src/output/repo_json.rs`
- `sniff/cli/src/output/filesystem.rs` (doc comment only)
- `sniff/cli/src/args.rs` (`REPO_AFTER_HELP`)

### Step 1.1 — Text path: exit `1` when no primary language

In `sniff/cli/src/output/mod.rs`, the current arm reads:

```rust
Some(RepoAction::Language) => {
    out.push_str(&render_repo_language(result, base_dir));
}
```

Replace it with the locator-style empty-check, mirroring `RepoAction::Root`
on lines 528-535:

```rust
Some(RepoAction::Language) => {
    let rendered = render_repo_language(result, base_dir);
    if rendered.is_empty() {
        // Mirror locator family: empty == not detected → exit 1, no stdout.
        // JSON path emits `{ "language": null }` with the same exit code.
        std::process::exit(1);
    }
    out.push_str(&rendered);
}
```

Notes:

- Do **not** add a trailing `out.push('\n')` — `render_repo_language` already
  appends `\n` to the language name (see `filesystem.rs:2085`). The locator
  variants don't, which is why those arms push `'\n'` themselves.
- The `exit(1)` happens *before* the buffered output is flushed, which is
  identical to how `RepoAction::Root` etc. handle it today. Other commands
  in the same `match` (e.g. `print_current_package_area_dirty`) also exit
  early; this is an established pattern, not a new one.

### Step 1.2 — JSON path: attach `exit_code = 1` when language is null

In `sniff/cli/src/output/repo_json.rs`, replace the current arm:

```rust
Some(RepoAction::Language) => BuildOutcome::pure(json!({
    "language": filesystem::primary_language_name(result),
})),
```

with:

```rust
// `repo language --json` emits `{ "language": "Rust" }` (or
// `{ "language": null }` when no primary language can be detected).
// Exit code mirrors the text path: 0 on success, 1 on null, so scripts
// can branch on `$?` without parsing the JSON body.
Some(RepoAction::Language) => {
    let name = filesystem::primary_language_name(result);
    let exit_code = if name.is_none() { Some(1) } else { None };
    BuildOutcome {
        value: json!({ "language": name }),
        exit_code,
    }
}
```

`BuildOutcome` is the same struct used by `locator_root_outcome` (lines
196-205 of `repo_json.rs`); we mirror its pattern rather than calling that
helper directly because the JSON shape here uses key `"language"` not
`"root"`.

### Step 1.3 — Update `render_repo_language` doc comment

In `sniff/cli/src/output/filesystem.rs:2078-2087`, update the doc comment to
describe the new contract precisely:

```rust
/// Render the repository's primary programming language as plain text.
///
/// Returns the language name (e.g. `"Rust"`) followed by a newline,
/// suitable for piping. When no primary language can be determined,
/// returns an empty string; the caller in `output/mod.rs` treats an
/// empty result as "not detected" and exits with status `1` — the same
/// contract used by `repo root` / `repo package-root` / `repo package-area-root`.
///
/// ## Notes
///
/// The JSON path uses [`primary_language_name`] directly and emits
/// `{ "language": null }` with `exit_code = 1` for the same case.
```

No code change in this function — it stays pure, returns `String`.

### Step 1.4 — Add `sniff repo language` to `REPO_AFTER_HELP`

In `sniff/cli/src/args.rs:1447`, add a new section to the `REPO_AFTER_HELP`
constant. Place it after the `Packages:` block and before `Dependencies:`,
since "language" is a property of the repo as a whole, alongside
`packages` / `package`. Use this exact addition:

```text
Languages:
  sniff repo language                 Primary programming language for the repository
  sniff repo language --json          Same, as { "language": "Rust" }

```

(Note the trailing blank line, matching the spacing of every other section
in this constant.)

### Phase 1 verification

Run from the repo root:

```bash
cargo build -p sniff-cli
cargo test  -p sniff-cli --quiet
cargo clippy -p sniff-cli --all-targets -- -D warnings
cargo clippy -p sniff       --all-targets -- -D warnings
```

All four must pass with zero warnings. No new tests are expected to pass yet —
that is Phase 2's job — but **nothing existing must regress**, and `--help`
output must include the new `Languages:` block (manually verify with
`cargo run -p sniff-cli -- repo --help | grep -A2 Languages:`).

### Phase 1 stop signal

Stop after Phase 1 verification passes. Do not start Phase 2 in the same
session.

---

## Phase 2: Level 1 CLI integration tests

**Objective:** Pin every behaviour decided above behind tests that spawn the
real `sniff` binary against temporary git repos. After this phase, `cargo
test -p sniff-cli` must cover all six requirement bullets from the review.

**Files modified:**

- `sniff/cli/tests/cli.rs` (additions only — no edits to existing tests)

### Step 2.1 — Add a section header and tests

Add a new section near the existing language tests (around `cli.rs:736`, the
`test_language_subcommand_*` block for the older top-level `sniff language`).
Insert immediately **after** that block so related tests cluster, with the
header:

```rust
// ============================================================================
// `sniff repo language` Subcommand Tests (review-plan-1, Phase 2)
// Pins:
//   - text output exact contract: `Rust\n` / empty + exit 1
//   - JSON output exact contract: `{"language":"Rust"}` / `{"language":null}` + exit 1
//   - `--base` works in all three placements (global pre, repo-nested, leaf)
// ============================================================================
```

The tests below all use the existing `create_test_repo()` /
`test_commit_file()` helpers from `cli.rs:1276-1318`. Do **not** add new
helpers — those already do exactly what's needed.

### Step 2.2 — Test: text output is exactly `Rust\n`

```rust
#[test]
fn test_repo_language_text_returns_rust_for_rust_repo() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "language"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout, "Rust\n", "expected exact `Rust\\n` output");
}
```

### Step 2.3 — Test: JSON output is exactly `{"language":"Rust"}`

```rust
#[test]
fn test_repo_language_json_returns_rust_for_rust_repo() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");

    let output = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "language", "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = std::str::from_utf8(&output).unwrap().trim_end();
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .expect("repo language --json must emit valid JSON");

    // Exact shape contract: object with single key "language" → "Rust".
    assert_eq!(parsed, serde_json::json!({ "language": "Rust" }));
}
```

### Step 2.4 — Test: `--base` works in all three placements

This test is the heart of the High finding. Use a single helper closure to
keep the three invocations terse and to make a regression in any one
placement trivially visible:

```rust
#[test]
fn test_repo_language_base_flag_all_three_placements() {
    let (_dir, path) = create_test_repo();
    test_commit_file(&path, "src/main.rs", "fn main() {}");
    let base = path.to_str().unwrap();

    // Placement A: `sniff --base <repo> repo language` (global, before subcommand)
    let a = cargo_bin_cmd!("sniff")
        .args(["--base", base, "repo", "language"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(a).unwrap(), "Rust\n", "placement A failed");

    // Placement B: `sniff repo --base <repo> language` (between repo and leaf)
    let b = cargo_bin_cmd!("sniff")
        .args(["repo", "--base", base, "language"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(b).unwrap(), "Rust\n", "placement B failed");

    // Placement C: `sniff repo language --base <repo>` (after the leaf subcommand)
    let c = cargo_bin_cmd!("sniff")
        .args(["repo", "language", "--base", base])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(String::from_utf8(c).unwrap(), "Rust\n", "placement C failed");
}
```

If any placement fails, the failing assertion identifies it directly. This
is the regression sentinel for the `--base` fix that the original spec called
out.

### Step 2.5 — Test: empty repo, text mode → empty stdout + exit 1

```rust
#[test]
fn test_repo_language_text_empty_repo_exits_one_with_no_stdout() {
    // create_test_repo creates a git repo with one empty initial commit
    // and no source files — primary language detection returns None.
    let (_dir, path) = create_test_repo();

    let assert = cargo_bin_cmd!("sniff")
        .args(["--base", path.to_str().unwrap(), "repo", "language"])
        .assert()
        .failure() // exit 1 by Phase 1 contract
        .code(1);

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout, "", "text mode must emit no stdout when no language detected");
}
```

### Step 2.6 — Test: empty repo, JSON mode → `{"language":null}` + exit 1

```rust
#[test]
fn test_repo_language_json_empty_repo_emits_null_and_exits_one() {
    let (_dir, path) = create_test_repo();

    let assert = cargo_bin_cmd!("sniff")
        .args([
            "--base", path.to_str().unwrap(),
            "repo", "language", "--json",
        ])
        .assert()
        .failure()
        .code(1);

    let stdout = assert.get_output().stdout.clone();
    let json_str = std::str::from_utf8(&stdout).unwrap().trim_end();
    let parsed: serde_json::Value = serde_json::from_str(json_str)
        .expect("repo language --json must emit valid JSON even when null");
    assert_eq!(parsed, serde_json::json!({ "language": null }));
}
```

### Step 2.7 — Optional: pin help-text discoverability

Add one tiny test so the Low finding can't silently regress:

```rust
#[test]
fn test_repo_help_lists_language_subcommand() {
    cargo_bin_cmd!("sniff")
        .args(["repo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff repo language"));
}
```

### Phase 2 verification

```bash
cargo test  -p sniff-cli --quiet
cargo clippy -p sniff-cli --all-targets -- -D warnings
```

All seven new tests must pass; no existing test may regress. Clippy must be
clean. Specifically confirm the six new `test_repo_language_*` tests + the
one help test all show up in the test report.

If `test_repo_language_text_empty_repo_exits_one_with_no_stdout` fails with
exit 0, Phase 1 was not actually applied — go fix Phase 1 first, do not paper
over it in tests.

### Phase 2 stop signal

Stop after Phase 2 verification passes.

---

## Phase 3: Final lint/test sweep + perf sanity check

**Objective:** Confirm the entire `sniff` package area (lib + cli) is clean
and that the Critical perf fix is still in place. Pure verification phase —
no code changes expected.

### Step 3.1 — Confirm the Critical perf fix is intact

Inspect `sniff/cli/src/commands.rs` around line 705. The `RepoAction::Language`
arm of `OutputFilter::Repo` must read (or be equivalent to):

```rust
Some(crate::args::RepoAction::Language) => DetectionPlan::new()
    .without_os()
    .without_hardware()
    .without_network()
    .filesystem(
        FilesystemRequest::new()
            .git(GitRequest::summary())
            .without_repo()
            .without_docs()
            .without_formatting(),
    ),
```

Specifically check:

- `git(GitRequest::summary())` — **must** be `summary()`, not `full()` /
  default.
- `.without_repo()` — **must** be present.
- `.without_docs()` and `.without_formatting()` — **must** be present.

If any of these have drifted, restore them before running the rest of
Phase 3. This is the only "code might need a touch" step in Phase 3.

### Step 3.2 — Smoke: wall-time sanity check

In a known-Rust repo (the rusty-biscuit checkout itself is fine), confirm
`sniff repo language` is fast:

```bash
cargo build -p sniff-cli --release
time ./target/release/sniff repo language
```

Acceptable: well under 0.5s wall time on a laptop. The review measured
~0.13s after the fix; anything above ~0.5s suggests the perf regression has
crept back in and Step 3.1 should be revisited.

This is a sanity check, not a CI gate — do **not** add a timing assertion to
`cli.rs`. Timing tests are flaky on shared CI runners.

### Step 3.3 — Full sniff-area lint/test

```bash
cargo test  -p sniff      --quiet
cargo test  -p sniff-cli  --quiet
cargo clippy -p sniff      --all-targets -- -D warnings
cargo clippy -p sniff-cli  --all-targets -- -D warnings
cargo doc   -p sniff      --no-deps --quiet
cargo doc   -p sniff-cli  --no-deps --quiet
```

All six commands must succeed with zero warnings.

### Step 3.4 — Format check

```bash
cargo fmt --check -p sniff
cargo fmt --check -p sniff-cli
```

Both must be clean. If they aren't, run `cargo fmt -p <pkg>` and re-run
Phase 3 from Step 3.3.

### Phase 3 verification (final report)

Report all of the following as passing:

- [ ] Step 3.1: `commands.rs` `RepoAction::Language` arm uses
      `GitRequest::summary()` + `.without_repo()` + `.without_docs()` +
      `.without_formatting()`
- [ ] Step 3.2: release binary `sniff repo language` runs in well under
      0.5s on a non-empty Rust repo
- [ ] Step 3.3: all six `cargo test` / `cargo clippy` / `cargo doc`
      invocations pass with zero warnings
- [ ] Step 3.4: `cargo fmt --check` passes for `sniff` and `sniff-cli`
- [ ] All seven Phase 2 tests appear in the test list and pass
- [ ] `sniff repo --help` lists `sniff repo language` (manually grep the
      output)

Once all six are green, the review is fully addressed.

---

## Risks and assumptions

1. **Behaviour break for any current consumer of `repo language` exit code.**
   The decision flips empty-repo from exit 0 to exit 1. The feature was
   merged so recently (this branch) that real consumers are extremely
   unlikely, but if there is a script in the user's dotfiles that relied on
   the old "always succeed" behaviour, it will need a one-character update
   (`|| true`). This is the right break to take now, before the contract
   leaks further.

2. **Locator-family pattern reuse, not extraction.** Phase 1 inlines the
   empty-check in `output/mod.rs` rather than refactoring `RepoAction::Root`
   /`PackageRoot` / `PackageAreaRoot` / `Language` into a shared helper.
   That refactor is worthwhile but is **out of scope** for this review-fix
   plan — it would expand the diff and risk breaking the existing locator
   tests. Leaving it as a follow-up keeps this plan reviewable.

3. **`create_test_repo()` produces an empty git repo with one commit.**
   Phase 2's "no language" tests (Steps 2.5 / 2.6) rely on the empty-repo
   path returning `None` for primary language. If the underlying
   `sniff::SniffResult` ever decides to report something like "Markdown" for
   a repo containing only a README (which `create_test_repo` does not
   create — it commits an empty tree), the empty-repo tests will need a
   different fixture. Spot-checked: `create_test_repo` writes no files, so
   the languages summary will be empty — assumption holds.

4. **`--base` test placements depend on `global = true` on the `--base`
   arg.** Already verified at plan-creation time:
   `sniff/cli/src/args.rs:227` has `#[arg(short, long, global = true)]` on
   the `base` field. If a future refactor removes `global = true`,
   placements A and B will stop working — and Phase 2's
   `test_repo_language_base_flag_all_three_placements` will catch it
   immediately. That is the test's purpose.

5. **The Critical fix is in working-tree commits already.** Verified via
   `git diff` that `commands.rs:705` already uses `GitRequest::summary()` +
   `.without_repo()`. Phase 3 Step 3.1 only confirms; it should not need
   to write code. If Phase 3 finds drift, that is a separate problem and
   Step 3.1's instructions handle it.

6. **No skill or doc changes.** None of the user-facing skill docs or repo
   READMEs reference exit-code semantics for `repo language`, so they don't
   need updating. The skill (`.claude/skills/sniff/SKILL.md:131`) already
   lists `sniff repo language` in its CLI examples and is correct as-is.

7. **`exit(1)` mid-output is intentional.** The text path at
   `output/mod.rs` runs accumulated `out` to stdout *after* the `match`
   block. Calling `std::process::exit(1)` inside the empty arm short-circuits
   that flush, which is exactly what the existing `RepoAction::Root` /
   `PackageRoot` / `PackageAreaRoot` arms already do (see lines 514-535).
   No buffered output is lost because `out` is empty in the failure case
   anyway (no `repo language` arm has appended to it).
