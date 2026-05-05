# Sniff Code Review: Type Safety, DRY, and Test Coverage

**Date**: 2026-04-03
**Scope**: `sniff/lib` and `sniff/cli`
**Focus areas**: (1) Type safety improvements, (2) DRY violations, (3) Test coverage gaps

---

## Summary

The sniff library and CLI form a cross-platform system detection toolkit. The codebase is well-structured with clean separation between detection domains (OS, hardware, network, filesystem, programs). However, the **program detection subsystem** (`sniff/lib/src/programs/`) carries significant boilerplate from a manually-maintained parallel type hierarchy: 8 per-category enums, 8 nearly-identical detector structs, and a unified `Program` enum with a ~2700-line lookup table. This review identifies concrete opportunities to improve type safety, eliminate duplication, and close test gaps.

**Severity key**: High = significant bug risk or maintenance burden; Medium = worthwhile improvement; Low = nice-to-have.

---

## 1. Type Safety Improvements

### 1.1 `InstalledEditors` (and all 8 category detectors) use manual match exhaustiveness instead of indexed storage

**Severity**: High
**Files**: `sniff/lib/src/programs/editors.rs`, `utilities.rs`, `ai_cli.rs`, `language_package_managers.rs`, `os_package_managers.rs`, `tts_clients.rs`, `terminal_apps.rs`, `headless_audio.rs`

Each category detector struct stores individual fields:

```rust
pub struct InstalledEditors {
    pub cursor: Option<(PathBuf, ExecutableSource)>,
    pub vscode: Option<(PathBuf, ExecutableSource)>,
    pub zed: Option<(PathBuf, ExecutableSource)>,
    // ... one field per variant
}
```

Every method (`is_installed()`, `path()`, `path_with_source()`, serialize, deserialize) contains a manual `match` over the `Editor` enum that must enumerate every variant. If someone adds a variant to `Editor` but forgets one of these match arms, the compiler will catch it (thanks to exhaustive matching), but the **runtime semantics** silently degrade: the new variant returns `None` in some methods but not others, and the serialization format breaks.

**Recommendation**: Replace the individual-field struct with a type-indexed map:

```rust
pub struct InstalledPrograms<E: CategoryEnum> {
    inner: EnumMap<E, Option<(PathBuf, ExecutableSource)>>,
}
```

This eliminates all per-variant match statements in one stroke. The `enum_map` crate (or a hand-rolled `Vec` indexed by discriminant) gives O(1) lookup and guarantees compile-time exhaustiveness when `E` gains new variants.

### 1.2 Parallel enum hierarchy (`Editor` / `Utility` / ... vs `Program`) requires manual sync

**Severity**: High
**Files**: `sniff/lib/src/programs/enums.rs`, `sniff/lib/src/programs/inventory.rs`

Adding a new program requires:
1. Adding a variant to the category enum (e.g., `Editor::NewEditor`)
2. Adding a corresponding variant to `Program` (e.g., `Program::NewEditor`)
3. Updating the `editor_details()` mapping function
4. Adding an entry to `PROGRAM_LOOKUP`
5. Adding an entry to the per-category `ProgramInfo` metadata array

Steps 2-5 are all derivable from step 1, yet each is manually maintained.

**Recommendation**: Use a single enum as the source of truth and derive the category grouping via a trait or associated type:

```rust
#[derive(strum::EnumIter)]
pub enum Program {
    // all variants here
}

pub trait ProgramCategory {
    fn programs() -> impl Iterator<Item = Program>;
}
```

Alternatively, use a derive macro that generates both the per-category enums and the unified enum from a single definition.

### 1.3 `editor_details()` and sibling mapping functions are fragile manual match arms

**Severity**: Medium
**Files**: Each detector file (e.g., `editors.rs:22-53`)

```rust
fn editor_details(editor: &Editor) -> Program {
    match editor {
        Editor::Cursor => Program::Cursor,
        Editor::Vscode => Program::Vscode,
        // ... manual 1:1 mapping
    }
}
```

This is a pure identity-style mapping that adds no logic. If the two enums share variant names, a macro can generate it. If they don't always share names, the mapping should at minimum be testable for completeness.

**Recommendation**: Replace with a derive-based mapping or, at minimum, add a test that iterates all category variants and asserts the mapping returns a unique `Program` variant.

### 1.4 `InstallationMethod` methods have large match arms that duplicate structure

**Severity**: Medium
**Files**: `sniff/lib/src/programs/types.rs`

`InstallationMethod` has `package_name()` and `manager_name()` methods with ~20 match arms each. These could be replaced with struct fields:

```rust
pub enum InstallationMethod {
    Brew { package: String },
    Apt { package: String },
    // ...
}
```

This eliminates the need for `package_name()` and `manager_name()` entirely — callers access the field directly. The `method_available()` function in `installer.rs` (35-arm match) could then use a `manager_executable()` method on the enum itself.

### 1.5 `method_available()` in `installer.rs` manually maps `InstallationMethod` to manager executables

**Severity**: Medium
**Files**: `sniff/lib/src/programs/installer.rs`

```rust
fn method_available(method: &InstallationMethod) -> Option<InstallationMethod> {
    let available = match method {
        InstallationMethod::Brew(_) => which::which("brew").is_ok(),
        InstallationMethod::Apt(_) => which::which("apt").is_ok(),
        // ... 35 arms
    };
    // ...
}
```

**Recommendation**: Add a `fn manager_binary(&self) -> &str` method to `InstallationMethod` and collapse this to a single `which::which(method.manager_binary())` call.

---

## 2. DRY Violations

### 2.1 Eight nearly-identical category detector files (THE BIG ONE)

**Severity**: High
**Files**: `editors.rs`, `utilities.rs`, `ai_cli.rs`, `language_package_managers.rs`, `os_package_managers.rs`, `tts_clients.rs`, `terminal_apps.rs`, `headless_audio.rs`

Each file contains the same structural pattern:
1. A struct with individual `Option<(PathBuf, ExecutableSource)>` fields
2. `new()` — iterates the category enum, calls `find_program()`, stores results
3. `new_with_index()` — same but uses a pre-built `ExecutableIndex`
4. `is_installed()` — match on category enum, return field's `is_some()`
5. `path()` / `path_with_source()` — match on category enum, return field
6. `Serialize` / `Deserialize` — manual match to/from string keys
7. `ProgramDetector` impl — calls `*_details()` for the mapping

Estimated duplication: **~150-200 lines per file × 8 files = ~1200-1600 lines** of pure boilerplate.

**Recommendation**: Create a generic `CategoryDetector<E>` struct that implements all of this once:

```rust
pub struct CategoryDetector<E: CategoryEnum> {
    programs: EnumMap<E, Option<(PathBuf, ExecutableSource)>>,
}

impl<E: CategoryEnum> CategoryDetector<E> {
    pub fn new(index: &ExecutableIndex) -> Self { /* ... */ }
    pub fn is_installed(&self, program: E) -> bool { /* ... */ }
    pub fn path(&self, program: E) -> Option<&Path> { /* ... */ }
}
```

This would reduce each detector file to just the category-specific metadata and any custom detection logic.

### 2.2 `PROGRAM_LOOKUP` in `inventory.rs` (~2700 lines) duplicates metadata from `enums.rs`

**Severity**: High
**Files**: `sniff/lib/src/programs/inventory.rs`, `sniff/lib/src/programs/enums.rs`

`enums.rs` defines `ProgramInfo` arrays per category (name, description, website, repo). `inventory.rs` defines `ProgramDetails` entries per `Program` variant with overlapping metadata (name, description, website, repo). The name/description/website/repo data appears in **both** places.

**Recommendation**: Make `ProgramDetails` reference or compose `ProgramInfo` rather than duplicating its fields. Alternatively, generate `PROGRAM_LOOKUP` at compile time from a single source of truth (e.g., a macro that reads both the enum variants and their metadata).

### 2.3 `ProgramDetector` trait blanket impls are boilerplate

**Severity**: Medium
**Files**: Each detector file's `ProgramDetector` impl

Every `ProgramDetector` impl follows the identical pattern:

```rust
impl ProgramDetector for InstalledEditors {
    fn install(&self, program: &Program) -> Result<String, SniffError> {
        let details = self.editor_details(/* extract variant */);
        execute_install(details, &InstallOptions::default())
    }
    // install_version() is identical pattern
}
```

**Recommendation**: If the generic `CategoryDetector<E>` from 2.1 is adopted, the `ProgramDetector` impl becomes a single blanket implementation.

### 2.4 Serialization boilerplate in each detector

**Severity**: Medium
**Files**: Each detector file's `Serialize`/`Deserialize` impls

Each file manually serializes/deserializes by iterating enum variants and mapping to string keys. This is ~30-40 lines per file.

**Recommendation**: The generic `CategoryDetector<E>` approach handles this uniformly. Alternatively, `serde` attributes on an enum-map-based structure would handle it automatically.

---

## 3. Test Coverage Gaps

### 3.1 No tests for `ProgramDetector::install()` or `install_version()`

**Severity**: High
**Files**: `sniff/lib/src/programs/installer.rs`, `sniff/lib/src/programs/types.rs`

The installation code paths have **zero test coverage**. These execute system commands (`brew install`, `apt install`, etc.). Even with mocking, the argument construction logic is untested.

**Recommendation**: Use `wiremock` or a mock command runner to test:
- Argument construction for each `InstallationMethod`
- `select_best_method()` prioritization
- `install_version()` version flag injection per manager
- Error handling when package manager not found

### 3.2 No tests for serialization roundtrips of full `ProgramsInfo`

**Severity**: Medium
**Files**: `sniff/lib/tests/`

There are no tests verifying that `ProgramsInfo` serializes to JSON and deserializes back correctly. Given the manual `Serialize`/`Deserialize` impls in each detector, this is a regression risk.

**Recommendation**: Add a roundtrip test:

```rust
#[test]
fn programs_info_serialization_roundtrip() {
    let info = ProgramsInfo::default(); // or constructed
    let json = serde_json::to_string(&info).unwrap();
    let decoded: ProgramsInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(info, decoded);
}
```

### 3.3 No tests verifying `Program` enum stays in sync with category enums

**Severity**: Medium
**Files**: `sniff/lib/src/programs/enums.rs`, `sniff/lib/src/programs/inventory.rs`

The `program_lookup_has_all_variants()` test checks that `PROGRAM_LOOKUP` contains all `Program` variants, but there is no test verifying that the category enums (`Editor`, `Utility`, etc.) collectively cover all `Program` variants, or that `editor_details()` etc. are surjective.

**Recommendation**: Add a test that:
1. Iterates all `Program` variants
2. Asserts each maps back to exactly one category enum variant
3. Asserts the mapping is bijective (no two category variants map to the same `Program`)

### 3.4 No tests for `SniffError` variants

**Severity**: Medium
**Files**: `sniff/lib/src/error.rs`

Several `SniffError` variants have no test coverage:
- `ShorthandNotFound`
- `InvalidCredentials`
- `RateLimited`
- `InstallationFailed`

**Recommendation**: Add unit tests for each variant's Display output and any relevant conversion impls.

### 3.5 No tests for the services module

**Severity**: Medium
**Files**: `sniff/lib/src/programs/services.rs` (if it exists)

No test file was found for service detection. If service detection logic exists, it needs coverage for:
- Service name normalization
- Detection of running vs stopped vs not-installed states
- Platform-specific service managers (launchd, systemd, etc.)

### 3.6 No negative tests for `FileAssociationArg` and CLI argument parsing

**Severity**: Low
**Files**: `sniff/cli/src/args.rs`

The CLI argument types have implicit `From` conversions but no tests for invalid input handling (e.g., unknown file association types, invalid program names as positional args).

### 3.7 No tests for `ProgramDetails` equality/clone behavior

**Severity**: Low
**Files**: `sniff/lib/src/programs/types.rs`

`ProgramDetails` derives `PartialEq` and `Clone` but these are never exercised in tests. Given the large struct, a constructed equality test would catch accidental field reordering or future `PartialEq` customization bugs.

---

## 4. Priority Recommendations

Ordered by impact-to-effort ratio:

| Priority | Item | Effort | Impact |
|----------|------|--------|--------|
| 1 | Generic `CategoryDetector<E>` (2.1) | Medium | Eliminates ~1200-1600 lines of boilerplate, improves type safety |
| 2 | Unify enum hierarchy (1.2) | Medium-High | Prevents sync bugs, reduces maintenance burden |
| 3 | `InstallationMethod` struct fields (1.4 + 1.5) | Low | Eliminates ~60 lines of match arms, improves correctness |
| 4 | Installation code path tests (3.1) | Medium | Covers critical untested functionality |
| 5 | Serialization roundtrip tests (3.2) | Low | Catches serialization regressions early |
| 6 | Enum sync tests (3.3) | Low | Prevents enum hierarchy drift |
| 7 | `PROGRAM_LOOKUP` dedup (2.2) | Medium | Eliminates ~2700 lines of duplicated metadata |

---

## Appendix: Files Reviewed

### Library
- `sniff/lib/src/lib.rs`
- `sniff/lib/src/request.rs`
- `sniff/lib/src/error.rs`
- `sniff/lib/src/os/mod.rs`
- `sniff/lib/src/hardware/mod.rs`
- `sniff/lib/src/network/mod.rs`
- `sniff/lib/src/filesystem/mod.rs`
- `sniff/lib/src/programs/mod.rs`
- `sniff/lib/src/programs/types.rs`
- `sniff/lib/src/programs/schema.rs`
- `sniff/lib/src/programs/enums.rs`
- `sniff/lib/src/programs/inventory.rs`
- `sniff/lib/src/programs/editors.rs`
- `sniff/lib/src/programs/utilities.rs`
- `sniff/lib/src/programs/ai_cli.rs`
- `sniff/lib/src/programs/language_package_managers.rs`
- `sniff/lib/src/programs/os_package_managers.rs`
- `sniff/lib/src/programs/tts_clients.rs`
- `sniff/lib/src/programs/terminal_apps.rs`
- `sniff/lib/src/programs/headless_audio.rs`
- `sniff/lib/src/programs/find_program.rs`
- `sniff/lib/src/programs/installer.rs`

### CLI
- `sniff/cli/src/commands.rs`
- `sniff/cli/src/args.rs`

### Tests
- `sniff/lib/tests/integration.rs`
- `sniff/lib/tests/program_installable.rs`
