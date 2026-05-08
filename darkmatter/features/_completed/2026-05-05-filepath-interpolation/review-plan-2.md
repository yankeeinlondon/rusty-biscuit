---
phases: 6
start_phase: 4
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/link_normalization.rs
  - darkmatter/lib/src/markdown/compose/link_resolve.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - darkmatter/docs/operations/link-resolve.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - darkmatter
  - darkmatter-cli
---

# Implementation Plan: Filepath Interpolation — Review 2

## Objective
Address all findings from Review 2 to bring the filepath interpolation feature to production readiness. The plan ensures no debug output leaks to stdout, spaced HTML attributes are fully supported, warnings are emitted exactly once, and the feature has comprehensive CLI-level test coverage.

---

## Phase 1: Remove Debug Traces from Production Paths (HIGH)

**Goal:** Eliminate unconditional `println!` calls that corrupt CLI stdout during `md compose`.

### 1.1 Files to modify
- `darkmatter/lib/src/markdown/compose/link_resolve.rs`
  - Lines 52, 54, 75, 129, 139, 148, 152
- `darkmatter/lib/src/markdown/compose/mod.rs`
  - Lines 151, 166, 176, 185 (inside `find_target_range`)

### 1.2 Changes
- Replace all `println!` calls in these locations with `tracing::trace!` or `tracing::debug!`.
- Add `use tracing::trace;` / `use tracing::debug;` imports where needed.
- Ensure no unconditional stdout writes remain in link_resolve, link_normalization, or find_target_range.

### 1.3 Verification
- Run `cargo test -p darkmatter link_resolve -- --nocapture` and confirm no diagnostics appear on stdout.
- Run `cargo test -p darkmatter --test link_interpolation_integration -- --nocapture` and confirm stdout is clean.

---

## Phase 2: Fix HTML Spaced-Attribute Pre-Scan (HIGH)

**Goal:** Ensure documents containing `<a href = "...">` or `<img src = "...">` are not skipped by the early-exit check.

### 2.1 Root cause
Both `link_resolve` and `normalize_links` return early unless the content contains the exact substrings `](`, `href=`, or `src=`. The HTML extractor in `reference/html.rs` supports `href = "` and `src = "` (with spaces around `=`), but the pre-scan does not, so these documents are skipped before extraction even runs.

### 2.2 Changes
- In `link_resolve.rs` line 31 and `link_normalization.rs` line 41, replace the brittle substring check with a broader pre-scan that matches the extractor's accepted syntax.
- Accept:
  - `](` for Markdown links/images
  - `href=` and `href =`
  - `src=` and `src =`
- The simplest robust fix: remove the early-exit optimisation entirely. The extraction functions already handle documents with no matches efficiently. If performance is a concern, use a regex or broader substring check (`href` and `src` together with `](`), but the safest path is removal.

### 2.3 Unit tests to add
In `link_resolve.rs` tests:
- `test_link_resolve_html_spaced_attributes` — `<a href = "./b.md">`, `<img src = "b.md">`, `<video src = "./movie.mp4">`, `<link href = "styles.css">`

In `link_normalization.rs` tests:
- `test_normalize_links_html_spaced_attributes` — same tags with absolute paths, asserting they normalize correctly.

### 2.4 Verification
- Run the new tests: `cargo test -p darkmatter link_resolve_html_spaced` and `cargo test -p darkmatter normalize_links_html_spaced`.
- Confirm they pass.

---

## Phase 3: Deduplicate ENV-Var Warning Emission (MEDIUM)

**Goal:** Ensure ENV path substitution warnings appear exactly once on stderr.

### 3.1 Root cause
`link_normalization.rs` (lines 185–190) both:
1. Records a `ComposeWarning` in the report.
2. Directly renders a `Status` and writes it to stderr via `eprintln!`.

The CLI (`commands.rs` lines 759–769) then iterates over `report.warnings` and renders each one to stderr again, producing duplicates.

### 3.2 Changes
- In `link_normalization.rs`, remove the direct `eprintln!` of the `Status`.
- Keep the `report.add_warning(...)` call so the warning is programmatically visible.
- The CLI already renders all `report.warnings` to stderr; no CLI changes needed.

### 3.3 Verification
- Existing unit test `test_normalize_links_env_var` should still pass (it asserts on content and report, not stderr).
- CLI test in Phase 4 will verify exactly one warning appears on stderr.

---

## Phase 4: Add CLI-Level Integration Tests (MEDIUM)

**Goal:** Add Level-1 `assert_cmd` tests in `darkmatter/cli/tests/cli.rs` covering the binary's observable contract for filepath interpolation.

### 4.1 Test: `test_compose_link_relative_same_repo`
- Create a temp directory with a `.git` subdirectory (repo root), a `docs/` directory, and an `assets/` directory.
- Write `docs/source.md` containing `![img](../assets/logo.png)`.
- Run `md compose docs/source.md`.
- Assert stdout contains a portable relative path (e.g., `../assets/logo.png`), not an absolute path.
- Assert stdout does NOT contain diagnostics, `Total records`, `Record kind`, etc.

### 4.2 Test: `test_compose_link_transcluded_child`
- Create a repo with `parent.md` containing `::file child.md` and `child.md` containing `[link](./sibling.md)`.
- Run `md compose parent.md`.
- Assert the transcluded child's link is resolved relative to `child.md` and then normalized relative to `parent.md`.

### 4.3 Test: `test_compose_env_var_substitution_one_warning`
- Set a whitelisted env var (e.g., `PROJECT_ROOT`) pointing to a temp directory.
- Write a markdown file referencing a file under that directory with an absolute path.
- Run `md compose file.md`.
- Assert stderr contains exactly one warning about the env-var abstraction (use `String::from_utf8` on output and count occurrences, or use predicate logic).
- Assert stdout does NOT contain the warning text.

### 4.4 Test: `test_compose_html_spaced_attributes`
- Create a markdown file with `<a href = "./page.md">` and `<img src = "./img.png">`.
- Run `md compose file.md`.
- Assert stdout contains normalized relative paths, proving the spaced attributes were not skipped.

### 4.5 Verification
- Run `cargo test -p darkmatter-cli --test cli -- test_compose_link_`.
- All four tests must pass.

---

## Phase 5: Fix Broken Documentation Link (LOW)

**Goal:** Update `darkmatter/docs/operations/link-resolve.md` to point to the correct location of the Link Normalization doc.

### 5.1 Changes
- In `link-resolve.md` line 28, change:
  - From: `[Link Normalization](./link-normalization.md)`
  - To: `[Link Normalization](../inline/link-normalization.md)`

### 5.2 Verification
- Confirm the target file `darkmatter/docs/inline/link-normalization.md` exists.

---

## Phase 6: Final Verification & Lint

**Goal:** Ensure all tests pass and no lint warnings exist in the darkmatter package area.

### 6.1 Test commands to run
```bash
# Library tests
cargo test -p darkmatter link_resolve
cargo test -p darkmatter link_normalization
cargo test -p darkmatter --test link_interpolation_integration

# CLI tests
cargo test -p darkmatter-cli --test cli -- test_compose_link_relative
cargo test -p darkmatter-cli --test cli -- test_compose_link_transcluded
cargo test -p darkmatter-cli --test cli -- test_compose_env_var
cargo test -p darkmatter-cli --test cli -- test_compose_html_spaced

# Full package-area test suite (if available)
just test   # or cargo test -p darkmatter -p darkmatter-cli
```

### 6.2 Lint commands to run
```bash
cargo clippy -p darkmatter -- -D warnings
cargo clippy -p darkmatter-cli -- -D warnings
cargo fmt -- --check
```

### 6.3 Acceptance criteria
- All tests pass.
- No clippy warnings or errors in `darkmatter` or `darkmatter-cli`.
- `cargo fmt --check` reports no formatting issues.
- Running `md compose` on a file with links produces clean stdout (only composed markdown) and clean stderr (only expected warnings, no duplicates, no debug traces).

---

## Summary

| Phase | Focus | Priority | Tests Added |
|---|---|---|---|
| 1 | Remove `println!` debug traces from production code | HIGH | 0 (behavioural fix) |
| 2 | Fix spaced-HTML-attribute pre-scan skip | HIGH | 2 unit tests |
| 3 | Remove duplicate ENV-var warning emission | MEDIUM | 0 (behavioural fix) |
| 4 | Add CLI `assert_cmd` integration tests | MEDIUM | 4 CLI tests |
| 5 | Fix broken docs cross-reference | LOW | 0 |
| 6 | Full test & lint verification | — | — |
