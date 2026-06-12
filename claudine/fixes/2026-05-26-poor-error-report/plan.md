---
phases: 4
created: 2026-06-10
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/schemas/format.rs
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/validate.rs
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/tests/error_snapshots/markdown_error.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - darkmatter/lib/src/markdown/schemas/format.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/tests/error_snapshots/markdown_error.rs
documentation: []
packages:
  - darkmatter
---

# Execution Plan: Schema Validation Error Reporting

## Phase 1: Core Failure Mode Abstraction and Helper

**Objective:** Define the new file-reference failure abstraction in `format.rs` and introduce a typed resolution helper to distinguish parse, resolution, and non-existence errors.

- [x] **Define Failure Enum (`format.rs`)**
  - Implement `FileReferenceFailure` enum with variants: `InvalidSyntax { raw: String, err: FileReferenceError }`, `Resolution { raw: String, err: FileReferenceError }`, and `NoMatch { raw: String, cwd: Option<PathBuf> }`.
  - Implement `std::fmt::Display` for the enum to perfectly match the three message contracts specified:
    - `` `<value>` is not a valid file reference: <error> ``
    - `` could not resolve file reference `<value>`: <error> ``
    - `` no existing file matched reference `<value>` while resolving from `<cwd>` `` (omit `while resolving from <cwd>` if the `cwd` is unavailable).
- [x] **Implement Typed Resolution Helper (`format.rs`)**
  - Add `pub(crate) fn resolve_file_reference(value: &str) -> Result<PathBuf, FileReferenceFailure>`.
  - Call `FileReference::new(value)` and map errors to `InvalidSyntax`.
  - Call `resolve()` and map errors to `Resolution`.
  - Validate the path exists; if not, return `NoMatch` capturing `value.to_string()` and `std::env::current_dir().ok()`.
- [x] **Refactor Existing Format Validator (`format.rs`)**
  - Update `validate_file_reference(value: &str) -> bool` to return `resolve_file_reference(value).is_ok()`.
- [x] **Validation Checkpoint (Phase 1)**
  - Add unit tests for `resolve_file_reference` in `format.rs` covering an invalid reference, a missing file, and an existing file.
  - Run `cargo test -p darkmatter --lib` to ensure existing `format.rs` tests pass without failure.

## Phase 2: Intercepting Errors in Validation

**Objective:** Update the central JSON schema error mapping in `validate.rs` to substitute the detailed `FileReferenceFailure` messages when `format: darkmatter-file` fails.

- [x] **Intercept `darkmatter-file` Errors (`validate.rs`)**
  - In `build_problem`, intercept `ValidationErrorKind::Format`.
  - Check if `format` equals `format::DARKMATTER_FILE_FORMAT` and if `err.instance()` is a string.
  - Rerun `format::resolve_file_reference` on the instance string.
  - If it returns an `Err(failure)`, substitute the message of the resulting `ValidationProblem` with `failure.to_string()`.
  - Note: Ensure other format failures or cases where the string subsequently resolves successfully retain their original `err.to_string()`.
- [x] **Validation Checkpoint (Phase 2)**
  - Add unit tests in `validate.rs` covering a direct JSON schema setup using `format: darkmatter-file`. Ensure that validating a missing file produces the new typed error messages.

## Phase 3: Narrowing Glob Responsibility and Compile-Time Guards

**Objective:** Refactor the `x-darkmatter-match` keyword to only fail when a file exists but doesn't match the glob, and prevent bypassing existence checks by adding a compile-time schema guard.

- [x] **Refactor `DarkmatterMatchKeyword::check` (`format.rs`)**
  - Modify `check` to return `true` (valid) if parsing, resolving, or file existence fails.
  - This delegates "missing file" reporting exclusively to the `darkmatter-file` format validator to avoid duplicate or misleading errors.
- [x] **Add Schema-Build Guard (`format.rs`)**
  - In `match_keyword_factory`, inspect the `_parent` map for the `"format"` key.
  - Reject the schema construction with `ValidationError::schema` if the format is not exactly `"darkmatter-file"`.
- [x] **Validation Checkpoint (Phase 3)**
  - Add test in `format.rs` to verify that schema generation fails if `x-darkmatter-match` is present but `format: darkmatter-file` is absent.
  - Add test confirming `x-darkmatter-match` returns valid when the target file does not exist, verifying the format validator takes precedence.

## Phase 4: Integration Testing and Snapshot Verification

**Objective:** Update and verify all test snapshots and ensure the overall system correctly handles and displays the improved error diagnostics.

- [x] **Update Snapshots (`markdown_error.rs` and conversion tests)**
  - Run the `darkmatter` test suite: `cargo test -p darkmatter --lib`.
  - Identify snapshot failures in `darkmatter/lib/tests/error_snapshots/` and `darkmatter/lib/tests/snapshots/schemas_convert_snapshots`.
  - For `markdown_error.rs` which manually constructs the error object using the legacy generic string, update the mock string in the test to reflect the new structure.
  - Update all affected snapshots via manual updates or automated snapshot sync.
- [x] **Validation Checkpoint (Phase 4)**
  - Run all tests to confirm 100% pass rate: `cargo test -p darkmatter`.
  - Verify that no duplicate errors (glob + existence) are surfaced for missing files subject to `file(match(...))` constraints.
