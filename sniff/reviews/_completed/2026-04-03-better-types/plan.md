# Sniff Programs Module: Type Safety, DRY, and Test Coverage Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate ~4000 lines of boilerplate in the sniff programs module by replacing 8 hand-written detector structs with a single generic `CategoryDetector<E>`, merging duplicated metadata into a unified `ProgramInfo`, converting `Program` to a tagged union, and closing test coverage gaps.

**Architecture:** A `CategoryEnum` trait bridges each category enum (Editor, Utility, etc.) to a generic `CategoryDetector<E>` that provides detection, serialization, and installation uniformly. `ProgramInfo` absorbs the fields currently in `ProgramDetails`, eliminating the 2700-line `PROGRAM_LOOKUP` HashMap. The `Program` enum becomes a tagged union wrapping category enums, making the mapping structural rather than manually maintained.

**Tech Stack:** Rust, serde, strum (existing dependencies only — no new crates)

---

## File Map

**Modified:**
| File | Responsibility |
|------|---------------|
| `sniff/lib/src/programs/schema.rs` | Extend `ProgramInfo` with installation metadata + alternate binary names |
| `sniff/lib/src/programs/enums.rs` | `CategoryEnum` trait, update all 8 static `*_INFO` arrays with new fields |
| `sniff/lib/src/programs/types.rs` | `CategoryDetector<E>` struct + generic `ProgramDetector` impl; remove `ProgramDetails` |
| `sniff/lib/src/programs/inventory.rs` | `Program` tagged union; remove `PROGRAM_LOOKUP` HashMap + installation arrays |
| `sniff/lib/src/programs/installer.rs` | Add `InstallationMethod::manager_binary()`, simplify `method_available()` |
| `sniff/lib/src/programs/editors.rs` | Replace with type alias + minimal backward-compat |
| `sniff/lib/src/programs/utilities.rs` | Same |
| `sniff/lib/src/programs/ai_cli.rs` | Same + boolean accessors + `with_client()` |
| `sniff/lib/src/programs/tts_clients.rs` | Same + boolean accessors + `with_client()` |
| `sniff/lib/src/programs/pkg_mngrs.rs` | Same (two type aliases) |
| `sniff/lib/src/programs/terminal_apps.rs` | Same |
| `sniff/lib/src/programs/headless_audio.rs` | Same |
| `sniff/lib/src/programs/mod.rs` | Update exports, `ProgramsInfo` struct |
| `sniff/cli/src/install.rs` | Update detector type references if needed |
| `unchained-ai/lib/src/primitives/services/agent_status.rs` | Update boolean accessor calls |
| `biscuit-speaks/lib/src/detection.rs` | Update `with_client` if signature changes |

**New:**
| File | Responsibility |
|------|---------------|
| `sniff/lib/tests/program_serialization.rs` | Serialization roundtrip tests |

---

## Task 1: Extend ProgramInfo with Installation Metadata

**Files:**
- Modify: `sniff/lib/src/programs/schema.rs:94-161`
- Modify: `sniff/lib/src/programs/types.rs:246-292` (ProgramDetails — will be removed in Task 10)

Currently `ProgramInfo` (schema.rs) has version/detection metadata and `ProgramDetails` (types.rs) has installation metadata. These will be merged into `ProgramInfo` so every program's full metadata is accessible via `ProgramMetadata::info()`.

- [ ] **Step 1: Add new fields to ProgramInfo**

In `sniff/lib/src/programs/schema.rs`, add fields to `ProgramInfo`:

```rust
/// Metadata about a program including its name, website, version detection,
/// and installation methods.
#[derive(Debug, Clone)]
pub struct ProgramInfo {
    /// The binary name used to invoke this program.
    pub binary_name: &'static str,

    /// Human-readable display name.
    pub display_name: &'static str,

    /// One-line description of the program.
    pub description: &'static str,

    /// Official website URL.
    pub website: &'static str,

    /// Command-line flag to get version.
    pub version_flag: VersionFlag,

    /// Strategy for parsing version output.
    pub parse_strategy: VersionParseStrategy,

    /// Optional regex pattern for version extraction.
    pub version_regex: Option<&'static str>,

    /// Optional prefix to skip when parsing version.
    pub version_prefix: Option<&'static str>,

    // --- New fields (from ProgramDetails) ---

    /// Alternative binary names to search when primary is not found.
    /// E.g., kimi has alternate "kimi-cli"; sherpa-onnx has "sherpa-onnx-tts".
    pub alternate_binary_names: &'static [&'static str],

    /// Operating systems this program runs on. Empty slice means all OS.
    pub os_availability: &'static [OsType],

    /// Source code repository URL (if available).
    pub repo: Option<&'static str>,

    /// Methods for installing this program (brew, apt, cargo, etc.).
    pub installation_methods: &'static [InstallationMethod],
}
```

- [ ] **Step 2: Add OsType import and update constructors**

Add `use crate::os::OsType;` and `use crate::programs::types::InstallationMethod;` to schema.rs imports.

Update `ProgramInfo::standard()`:
```rust
pub const fn standard(
    binary_name: &'static str,
    display_name: &'static str,
    description: &'static str,
    website: &'static str,
) -> Self {
    Self {
        binary_name,
        display_name,
        description,
        website,
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[],
    }
}
```

Update `ProgramInfo::with_prefix()` similarly (add same 4 new fields with empty defaults).

Add a full constructor:
```rust
/// Creates a ProgramInfo with all fields specified.
pub const fn full(
    binary_name: &'static str,
    display_name: &'static str,
    description: &'static str,
    website: &'static str,
    version_flag: VersionFlag,
    parse_strategy: VersionParseStrategy,
    version_regex: Option<&'static str>,
    version_prefix: Option<&'static str>,
    alternate_binary_names: &'static [&'static str],
    os_availability: &'static [OsType],
    repo: Option<&'static str>,
    installation_methods: &'static [InstallationMethod],
) -> Self {
    Self {
        binary_name,
        display_name,
        description,
        website,
        version_flag,
        parse_strategy,
        version_regex,
        version_prefix,
        alternate_binary_names,
        os_availability,
        repo,
        installation_methods,
    }
}
```

- [ ] **Step 3: Add convenience method to ProgramMetadata trait**

In the `ProgramMetadata` trait, add:
```rust
/// Returns alternate binary names for fallback detection.
fn alternate_binary_names(&self) -> &'static [&'static str] {
    self.info().alternate_binary_names
}

/// Returns the OS availability list.
fn os_availability(&self) -> &'static [OsType] {
    self.info().os_availability
}

/// Returns the installation methods.
fn installation_methods(&self) -> &'static [InstallationMethod] {
    self.info().installation_methods
}

/// Returns the source code repository URL.
fn repo(&self) -> Option<&'static str> {
    self.info().repo
}
```

- [ ] **Step 4: Verify existing tests pass**

Run: `just test` from `sniff/` directory
Expected: All existing tests pass (the new fields have defaults so nothing breaks)

- [ ] **Step 5: Commit**

```
feat(sniff): extend ProgramInfo with installation metadata fields
```

---

## Task 2: Define CategoryEnum Trait

**Files:**
- Modify: `sniff/lib/src/programs/enums.rs` (add trait at top, before enum definitions)

- [ ] **Step 1: Write test for CategoryEnum trait**

Add to the test module in `enums.rs`:
```rust
#[test]
fn test_category_enum_trait_on_editor() {
    assert_eq!(Editor::category_name(), "editors");
    assert_eq!(Editor::Vi.variant_index(), 0);
    assert_eq!(Editor::Kate.variant_index(), Editor::COUNT - 1);

    // Verify all variant indices are unique and in range
    let mut seen = std::collections::HashSet::new();
    for editor in Editor::iter() {
        let idx = editor.variant_index();
        assert!(idx < Editor::COUNT, "{:?} index {} >= COUNT {}", editor, idx, Editor::COUNT);
        assert!(seen.insert(idx), "{:?} has duplicate index {}", editor, idx);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sniff --lib programs::enums::tests::test_category_enum_trait_on_editor`
Expected: FAIL — `category_name` method not found

- [ ] **Step 3: Define CategoryEnum trait**

Add after the imports in `enums.rs`, before the Editor enum:

```rust
use std::fmt;
use std::hash::Hash;

use crate::os::OsType;
use crate::programs::types::InstallationMethod;

/// Trait bridging category enums to the generic `CategoryDetector<E>`.
///
/// Implementors provide category-level metadata and variant indexing
/// that enables a single generic detector struct to work across all
/// program categories.
pub trait CategoryEnum:
    ProgramMetadata
    + strum::IntoEnumIterator
    + strum::EnumCount
    + Copy
    + Clone
    + Eq
    + Hash
    + fmt::Debug
    + fmt::Display
    + Send
    + Sync
    + 'static
{
    /// Human-readable category name (e.g., "editors", "utilities").
    fn category_name() -> &'static str;

    /// Returns the ordinal index of this variant (0-based, contiguous).
    fn variant_index(&self) -> usize;

    /// Serialization key for JSON output (snake_case variant name).
    fn serde_key(&self) -> &'static str;

    /// Platform-specific detection override.
    ///
    /// Returns `Some(...)` to inject a synthetic detection result instead of
    /// searching PATH. Used for Windows SAPI which isn't a real executable.
    fn platform_override(&self) -> Option<(std::path::PathBuf, crate::programs::ExecutableSource)> {
        None
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sniff --lib programs::enums::tests::test_category_enum_trait_on_editor`
Expected: FAIL — `CategoryEnum` not implemented for `Editor` yet. That's expected; implementation is Task 3.

- [ ] **Step 5: Commit**

```
feat(sniff): define CategoryEnum trait for generic program detection
```

---

## Task 3: Implement CategoryEnum for All 8 Enums + Extend INFO Arrays

**Files:**
- Modify: `sniff/lib/src/programs/enums.rs` (all 8 enum sections)

This task has two parts per enum: (A) implement `CategoryEnum`, (B) extend the static `*_INFO` array entries to include the new `ProgramInfo` fields (os_availability, repo, installation_methods, alternate_binary_names). The installation method arrays currently live in `inventory.rs` — they'll be moved to `enums.rs` alongside the `*_INFO` arrays so all metadata is co-located.

- [ ] **Step 1: Move OS availability constants from inventory.rs**

Add these constants near the top of `enums.rs` (after imports):

```rust
// OS availability constants
static ALL_OS: &[OsType] = &[OsType::MacOS, OsType::Linux, OsType::Windows];
static UNIX_ONLY: &[OsType] = &[OsType::MacOS, OsType::Linux];
static MACOS_ONLY: &[OsType] = &[OsType::MacOS];
static LINUX_ONLY: &[OsType] = &[OsType::Linux];
static WINDOWS_ONLY: &[OsType] = &[OsType::Windows];
```

- [ ] **Step 2: Move installation method arrays from inventory.rs to enums.rs**

Move all `static *_INSTALL: &[InstallationMethod]` arrays from `inventory.rs` to `enums.rs`, placing each category's install arrays just before its `*_INFO` array. For example, editor install arrays go before `EDITOR_INFO`.

These arrays keep their exact same content — this is a pure move, no changes to the data.

- [ ] **Step 3: Implement CategoryEnum for Editor**

```rust
impl CategoryEnum for Editor {
    fn category_name() -> &'static str {
        "editors"
    }

    fn variant_index(&self) -> usize {
        *self as usize
    }

    fn serde_key(&self) -> &'static str {
        match self {
            Editor::Vi => "vi",
            Editor::Vim => "vim",
            Editor::Neovim => "neovim",
            Editor::Emacs => "emacs",
            Editor::XEmacs => "xemacs",
            Editor::Nano => "nano",
            Editor::Helix => "helix",
            Editor::VSCode => "vscode",
            Editor::VSCodium => "vscodium",
            Editor::Sublime => "sublime",
            Editor::Zed => "zed",
            Editor::Micro => "micro",
            Editor::Kakoune => "kakoune",
            Editor::Amp => "amp",
            Editor::Lapce => "lapce",
            Editor::PhpStorm => "phpstorm",
            Editor::IntellijIdea => "intellij_idea",
            Editor::PyCharm => "pycharm",
            Editor::WebStorm => "webstorm",
            Editor::CLion => "clion",
            Editor::GoLand => "goland",
            Editor::Rider => "rider",
            Editor::TextMate => "textmate",
            Editor::BBEdit => "bbedit",
            Editor::Geany => "geany",
            Editor::Kate => "kate",
        }
    }
}
```

- [ ] **Step 4: Update EDITOR_INFO array entries with new fields**

Each entry changes from `ProgramInfo::standard(...)` to include the installation metadata. For example, the Vim entry changes from:

```rust
ProgramInfo::standard("vim", "Vim", "Vi IMproved text editor", "https://www.vim.org/"),
```

To a block using `ProgramInfo { ... }` with all fields:

```rust
ProgramInfo {
    binary_name: "vim",
    display_name: "Vim",
    description: "Vi IMproved text editor",
    website: "https://www.vim.org/",
    version_flag: VersionFlag::Long,
    parse_strategy: VersionParseStrategy::FirstLine,
    version_regex: None,
    version_prefix: None,
    alternate_binary_names: &[],
    os_availability: ALL_OS,
    repo: Some("https://github.com/vim/vim"),
    installation_methods: VIM_INSTALL,
},
```

Where previously the os_availability, repo, and installation_methods data lived in `inventory.rs` PROGRAM_LOOKUP entries, it now lives directly in the `EDITOR_INFO` array.

Do this for ALL 26 editor entries. The data comes from the corresponding `PROGRAM_LOOKUP` entry in inventory.rs. Match each entry carefully:
- `os_availability`: from `ProgramDetails::full(...)` os param
- `repo`: from `ProgramDetails::full(...)` repo param
- `installation_methods`: from `ProgramDetails::full(...)` install param

Programs that had no entry in PROGRAM_LOOKUP (if any) use empty defaults.

- [ ] **Step 5: Repeat Steps 3-4 for Utility enum**

Implement `CategoryEnum for Utility` with `category_name() -> "utilities"` and `serde_key()` mapping. Update all 30 `UTILITY_INFO` entries with data from PROGRAM_LOOKUP.

- [ ] **Step 6: Repeat for LanguagePackageManager**

`category_name() -> "language_package_managers"`. Update 18 `LANG_PKG_MGR_INFO` entries.

- [ ] **Step 7: Repeat for OsPackageManager**

`category_name() -> "os_package_managers"`. Update 9 `OS_PKG_MGR_INFO` entries.

- [ ] **Step 8: Repeat for TtsClient (with platform_override)**

`category_name() -> "tts_clients"`. Override `platform_override()`:

```rust
impl CategoryEnum for TtsClient {
    // ... standard methods ...

    fn platform_override(&self) -> Option<(std::path::PathBuf, crate::programs::ExecutableSource)> {
        match self {
            TtsClient::WindowsSapi => {
                if cfg!(target_os = "windows") {
                    Some((
                        std::path::PathBuf::from("sapi"),
                        crate::programs::ExecutableSource::Path,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
```

Update 15 `TTS_CLIENT_INFO` entries. For SherpaOnnx, set `alternate_binary_names: &["sherpa-onnx-tts"]` (primary is `sherpa-onnx-offline-tts`). For CoquiTts, binary_name is `tts`.

- [ ] **Step 9: Repeat for TerminalApp**

`category_name() -> "terminal_apps"`. Update 17 `TERMINAL_APP_INFO` entries.

- [ ] **Step 10: Repeat for HeadlessAudio**

`category_name() -> "headless_audio"`. Update 13 `HEADLESS_AUDIO_INFO` entries.

- [ ] **Step 11: Repeat for AiCli (with alternate binary names)**

`category_name() -> "ai_clients"`. Update 9 `AI_CLI_INFO` entries. For KimiCli, set `alternate_binary_names: &["kimi-cli"]` (primary is `kimi`).

- [ ] **Step 12: Verify all existing tests pass**

Run: `just test` from `sniff/`
Expected: All tests pass — the new fields are additive and existing constructors still work.

- [ ] **Step 13: Commit**

```
feat(sniff): implement CategoryEnum for all 8 program category enums
```

---

## Task 4: Create CategoryDetector<E> Core Implementation

**Files:**
- Modify: `sniff/lib/src/programs/types.rs`

- [ ] **Step 1: Write tests for CategoryDetector**

Add to the test module in `types.rs`:

```rust
use crate::programs::enums::{CategoryEnum, Editor};

#[test]
fn test_category_detector_default_has_nothing_installed() {
    let detector = CategoryDetector::<Editor>::default();
    assert!(detector.installed().is_empty());
    assert!(!detector.is_installed(Editor::Vim));
    assert!(detector.path(Editor::Vim).is_none());
    assert!(detector.path_with_source(Editor::Vim).is_none());
}

#[test]
fn test_category_detector_with_program_marks_installed() {
    let detector = CategoryDetector::<Editor>::default()
        .with_program(Editor::Vim, PathBuf::from("/usr/bin/vim"), ExecutableSource::Path);
    assert!(detector.is_installed(Editor::Vim));
    assert!(!detector.is_installed(Editor::Neovim));
    assert_eq!(
        detector.path(Editor::Vim),
        Some(PathBuf::from("/usr/bin/vim"))
    );
}

#[test]
fn test_category_detector_installed_returns_only_installed() {
    let detector = CategoryDetector::<Editor>::default()
        .with_program(Editor::Vim, PathBuf::from("/usr/bin/vim"), ExecutableSource::Path)
        .with_program(Editor::Neovim, PathBuf::from("/usr/bin/nvim"), ExecutableSource::Path);
    let installed = detector.installed();
    assert_eq!(installed.len(), 2);
    assert!(installed.contains(&Editor::Vim));
    assert!(installed.contains(&Editor::Neovim));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sniff --lib programs::types::tests::test_category_detector`
Expected: FAIL — `CategoryDetector` not found

- [ ] **Step 3: Implement CategoryDetector struct**

Add to `types.rs` (after imports, before `ProgramDetector` trait):

```rust
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use crate::programs::enums::CategoryEnum;
use crate::programs::find_program::{
    ExecutableIndex, find_programs_with_source_from_index, find_programs_with_source_parallel,
};
use crate::programs::schema::{ProgramEntry, ProgramError, ProgramMetadata};

/// Generic program detector for any category enum.
///
/// Stores detection results (path + source) indexed by enum variant ordinal.
/// Replaces the per-category `InstalledEditors`, `InstalledUtilities`, etc.
/// structs with a single generic implementation.
///
/// ## Examples
///
/// ```ignore
/// use sniff::programs::{CategoryDetector, Editor, ProgramDetector};
///
/// let editors = CategoryDetector::<Editor>::new();
/// if editors.is_installed(Editor::Vim) {
///     println!("Vim found at {:?}", editors.path(Editor::Vim));
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CategoryDetector<E: CategoryEnum> {
    results: Vec<Option<(PathBuf, ExecutableSource)>>,
    _phantom: PhantomData<E>,
}

impl<E: CategoryEnum> Default for CategoryDetector<E> {
    fn default() -> Self {
        Self {
            results: vec![None; E::COUNT],
            _phantom: PhantomData,
        }
    }
}

impl<E: CategoryEnum> PartialEq for CategoryDetector<E> {
    fn eq(&self, other: &Self) -> bool {
        self.results == other.results
    }
}

impl<E: CategoryEnum> Eq for CategoryDetector<E> {}

impl<E: CategoryEnum> CategoryDetector<E> {
    /// Detect installed programs by scanning PATH.
    pub fn new() -> Self {
        let mut names_to_search: Vec<&'static str> = Vec::new();
        for variant in E::iter() {
            let info = variant.info();
            names_to_search.push(info.binary_name);
            names_to_search.extend_from_slice(info.alternate_binary_names);
        }

        let found = find_programs_with_source_parallel(&names_to_search);
        Self::from_search_results(&found)
    }

    /// Detect installed programs using a pre-built executable index.
    pub fn new_with_index(index: &ExecutableIndex) -> Self {
        let mut names_to_search: Vec<&'static str> = Vec::new();
        for variant in E::iter() {
            let info = variant.info();
            names_to_search.push(info.binary_name);
            names_to_search.extend_from_slice(info.alternate_binary_names);
        }

        let found = find_programs_with_source_from_index(index, &names_to_search);
        Self::from_search_results(&found)
    }

    /// Construct from search results HashMap.
    fn from_search_results(
        found: &std::collections::HashMap<String, Option<(PathBuf, ExecutableSource)>>,
    ) -> Self {
        let mut results = vec![None; E::COUNT];

        for variant in E::iter() {
            let idx = variant.variant_index();

            // Check platform override first (e.g., Windows SAPI)
            if let Some(override_result) = variant.platform_override() {
                results[idx] = Some(override_result);
                continue;
            }

            // Try primary binary name
            let info = variant.info();
            if let Some(result) = found.get(info.binary_name).and_then(|r| r.clone()) {
                results[idx] = Some(result);
                continue;
            }

            // Try alternate binary names
            for alt in info.alternate_binary_names {
                if let Some(result) = found.get(*alt).and_then(|r| r.clone()) {
                    results[idx] = Some(result);
                    break;
                }
            }
        }

        Self {
            results,
            _phantom: PhantomData,
        }
    }

    /// Re-check program availability and update internal state.
    pub fn refresh(&mut self) {
        *self = Self::new();
    }

    /// Returns true if the specified program is installed.
    pub fn is_installed(&self, program: E) -> bool {
        self.results[program.variant_index()].is_some()
    }

    /// Returns the path to the specified program's binary if installed.
    pub fn path(&self, program: E) -> Option<PathBuf> {
        self.results[program.variant_index()]
            .as_ref()
            .map(|(p, _)| p.clone())
    }

    /// Returns the path and source of the specified program if installed.
    pub fn path_with_source(&self, program: E) -> Option<(PathBuf, ExecutableSource)> {
        self.results[program.variant_index()].clone()
    }

    /// Returns the version of the specified program if available.
    pub fn version(&self, program: E) -> Result<String, ProgramError> {
        if !self.is_installed(program) {
            return Err(ProgramError::NotFound(program.binary_name().to_string()));
        }
        program.version()
    }

    /// Returns the official website URL for the specified program.
    pub fn website(&self, program: E) -> &'static str {
        program.website()
    }

    /// Returns a one-line description of the specified program.
    pub fn description(&self, program: E) -> &'static str {
        program.description()
    }

    /// Returns a list of all installed programs in this category.
    pub fn installed(&self) -> Vec<E> {
        E::iter().filter(|p| self.is_installed(*p)).collect()
    }

    /// Builder method: mark a program as installed with given path and source.
    ///
    /// Useful for testing.
    pub fn with_program(mut self, program: E, path: PathBuf, source: ExecutableSource) -> Self {
        self.results[program.variant_index()] = Some((path, source));
        self
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p sniff --lib programs::types::tests::test_category_detector`
Expected: PASS

- [ ] **Step 5: Commit**

```
feat(sniff): add generic CategoryDetector<E> struct
```

---

## Task 5: Add Serialize/Deserialize to CategoryDetector<E>

**Files:**
- Modify: `sniff/lib/src/programs/types.rs`

- [ ] **Step 1: Write serialization roundtrip test**

```rust
#[test]
fn test_category_detector_serialize_produces_program_entries() {
    let detector = CategoryDetector::<Editor>::default();
    let json = serde_json::to_string(&detector).unwrap();
    // Should produce ProgramEntry objects with full metadata
    assert!(json.contains("\"installed\":false"));
    assert!(json.contains("\"vim\":{"));
    assert!(json.contains("\"name\":\"Vim\""));
}

#[test]
fn test_category_detector_deserialize_from_booleans() {
    let json = r#"{"vim": true, "vscode": false}"#;
    let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
    assert!(detector.is_installed(Editor::Vim));
    assert!(!detector.is_installed(Editor::VSCode));
}

#[test]
fn test_category_detector_deserialize_partial_json() {
    let json = r#"{"vim": true}"#;
    let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
    assert!(detector.is_installed(Editor::Vim));
    assert!(!detector.is_installed(Editor::Neovim));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p sniff --lib programs::types::tests::test_category_detector_serialize`
Expected: FAIL — Serialize not implemented

- [ ] **Step 3: Implement Serialize**

```rust
use serde::ser::SerializeMap;

impl<E: CategoryEnum> Serialize for CategoryDetector<E> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(E::COUNT))?;
        for variant in E::iter() {
            let key = variant.serde_key();
            let info = variant.info();
            let entry = match self.path_with_source(variant) {
                Some((path, source)) => ProgramEntry::installed(info, path, source),
                None => ProgramEntry::not_installed(info),
            };
            map.serialize_entry(key, &entry)?;
        }
        map.end()
    }
}
```

- [ ] **Step 4: Implement Deserialize**

```rust
use serde::de::{MapAccess, Visitor};

/// Helper for deserializing both boolean and ProgramEntry values.
#[derive(Deserialize)]
#[serde(untagged)]
enum BoolOrEntry {
    Bool(bool),
    Entry { installed: bool },
}

impl<'de, E: CategoryEnum> Deserialize<'de> for CategoryDetector<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(CategoryDetectorVisitor::<E>(PhantomData))
    }
}

struct CategoryDetectorVisitor<E>(PhantomData<E>);

impl<'de, E: CategoryEnum> Visitor<'de> for CategoryDetectorVisitor<E> {
    type Value = CategoryDetector<E>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a map of program names to booleans or program entries"
        )
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        // Build key -> variant index lookup
        let key_to_index: std::collections::HashMap<&'static str, usize> = E::iter()
            .map(|v| (v.serde_key(), v.variant_index()))
            .collect();

        let mut results = vec![None; E::COUNT];

        while let Some(key) = map.next_key::<String>()? {
            if let Some(&idx) = key_to_index.get(key.as_str()) {
                let value: BoolOrEntry = map.next_value()?;
                let installed = match value {
                    BoolOrEntry::Bool(b) => b,
                    BoolOrEntry::Entry { installed } => installed,
                };
                if installed {
                    results[idx] = Some((PathBuf::new(), ExecutableSource::Path));
                }
            } else {
                let _: serde::de::IgnoredAny = map.next_value()?;
            }
        }

        Ok(CategoryDetector {
            results,
            _phantom: PhantomData,
        })
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p sniff --lib programs::types::tests::test_category_detector`
Expected: PASS

- [ ] **Step 6: Commit**

```
feat(sniff): add Serialize/Deserialize for CategoryDetector<E>
```

---

## Task 6: Add ProgramDetector Blanket Impl for CategoryDetector<E>

**Files:**
- Modify: `sniff/lib/src/programs/types.rs`

- [ ] **Step 1: Write test for ProgramDetector trait on CategoryDetector**

```rust
#[test]
fn test_category_detector_program_detector_trait() {
    use crate::programs::types::ProgramDetector;

    let detector = CategoryDetector::<Editor>::default()
        .with_program(Editor::Vim, PathBuf::from("/usr/bin/vim"), ExecutableSource::Path);

    // Test through trait interface
    assert!(ProgramDetector::is_installed(&detector, Editor::Vim));
    assert!(!ProgramDetector::is_installed(&detector, Editor::Neovim));
    assert_eq!(
        ProgramDetector::path(&detector, Editor::Vim),
        Some(PathBuf::from("/usr/bin/vim"))
    );
    let installed = ProgramDetector::installed(&detector);
    assert_eq!(installed, vec![Editor::Vim]);
}
```

- [ ] **Step 2: Implement ProgramDetector for CategoryDetector<E>**

```rust
impl<E: CategoryEnum> ProgramDetector for CategoryDetector<E> {
    type Program = E;

    fn refresh(&mut self) {
        *self = Self::new();
    }

    fn is_installed(&self, program: E) -> bool {
        CategoryDetector::is_installed(self, program)
    }

    fn path(&self, program: E) -> Option<PathBuf> {
        CategoryDetector::path(self, program)
    }

    fn path_with_source(&self, program: E) -> Option<(PathBuf, ExecutableSource)> {
        CategoryDetector::path_with_source(self, program)
    }

    fn version(&self, program: E) -> Result<String, ProgramError> {
        CategoryDetector::version(self, program)
    }

    fn website(&self, program: E) -> &'static str {
        CategoryDetector::website(self, program)
    }

    fn description(&self, program: E) -> &'static str {
        CategoryDetector::description(self, program)
    }

    fn installed(&self) -> Vec<E> {
        CategoryDetector::installed(self)
    }

    fn installable(&self, program: E) -> bool {
        let info = program.info();
        if info.installation_methods.is_empty() {
            return false;
        }

        let os_availability = info.os_availability;
        if !os_availability.is_empty() {
            let os_type = crate::os::detect_os_type();
            if !os_availability.contains(&os_type) {
                return false;
            }
        }

        let os_pkg_mgrs = crate::programs::InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = crate::programs::InstalledLanguagePackageManagers::new();

        info.installation_methods
            .iter()
            .any(|method| {
                crate::programs::installer::method_available(method, &os_pkg_mgrs, &lang_pkg_mgrs)
            })
    }

    fn install(&self, program: E) -> Result<(), SniffInstallationError> {
        let info = program.info();

        if info.installation_methods.is_empty() {
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
            });
        }

        let os_availability = info.os_availability;
        if !os_availability.is_empty() {
            let os_type = crate::os::detect_os_type();
            if !os_availability.contains(&os_type) {
                return Err(SniffInstallationError::NotInstallableOnOs {
                    pkg: program.display_name().to_string(),
                    os: os_type.to_string(),
                });
            }
        }

        let os_pkg_mgrs = crate::programs::InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = crate::programs::InstalledLanguagePackageManagers::new();
        let method = crate::programs::installer::select_best_method(
            info.installation_methods,
            &os_pkg_mgrs,
            &lang_pkg_mgrs,
        )
        .ok_or_else(|| SniffInstallationError::MissingPackageManager {
            pkg: program.display_name().to_string(),
            manager: "package manager".to_string(),
        })?;

        let _result = crate::programs::installer::execute_install(
            method,
            &crate::programs::InstallOptions::default(),
        )?;
        Ok(())
    }

    fn install_version(
        &self,
        program: E,
        version: &str,
    ) -> Result<(), SniffInstallationError> {
        let info = program.info();

        if info.installation_methods.is_empty() {
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: program.display_name().to_string(),
                os: "unknown".to_string(),
            });
        }

        let os_availability = info.os_availability;
        if !os_availability.is_empty() {
            let os_type = crate::os::detect_os_type();
            if !os_availability.contains(&os_type) {
                return Err(SniffInstallationError::NotInstallableOnOs {
                    pkg: program.display_name().to_string(),
                    os: os_type.to_string(),
                });
            }
        }

        let os_pkg_mgrs = crate::programs::InstalledOsPackageManagers::new();
        let lang_pkg_mgrs = crate::programs::InstalledLanguagePackageManagers::new();
        let method = crate::programs::installer::select_best_method(
            info.installation_methods,
            &os_pkg_mgrs,
            &lang_pkg_mgrs,
        )
        .ok_or_else(|| SniffInstallationError::MissingPackageManager {
            pkg: program.display_name().to_string(),
            manager: "package manager".to_string(),
        })?;

        let _result = crate::programs::installer::execute_versioned_install(
            method,
            version,
            &crate::programs::InstallOptions::default(),
        )?;
        Ok(())
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p sniff --lib programs::types::tests`
Expected: PASS

- [ ] **Step 4: Commit**

```
feat(sniff): add ProgramDetector blanket impl for CategoryDetector<E>
```

---

## Task 7: Replace 8 Detector Files with Type Aliases

**Files:**
- Modify: All 7 detector files + `mod.rs`

Each detector file (~150-700 lines) is replaced by a type alias and optional backward-compat methods. The pattern is the same for all 8 detectors.

- [ ] **Step 1: Replace editors.rs**

Replace the entire contents of `editors.rs` with:

```rust
//! Editor detection — type alias for `CategoryDetector<Editor>`.

use crate::programs::enums::Editor;
use crate::programs::types::CategoryDetector;

/// Popular text editors and IDEs found on the system.
pub type InstalledEditors = CategoryDetector<Editor>;
```

- [ ] **Step 2: Replace utilities.rs**

```rust
//! Utility detection — type alias for `CategoryDetector<Utility>`.

use crate::programs::enums::Utility;
use crate::programs::types::CategoryDetector;

/// Modern command-line utilities found on the system.
pub type InstalledUtilities = CategoryDetector<Utility>;
```

- [ ] **Step 3: Replace terminal_apps.rs**

```rust
//! Terminal app detection — type alias for `CategoryDetector<TerminalApp>`.

use crate::programs::enums::TerminalApp;
use crate::programs::types::CategoryDetector;

/// Terminal emulator applications found on the system.
pub type InstalledTerminalApps = CategoryDetector<TerminalApp>;
```

- [ ] **Step 4: Replace headless_audio.rs**

```rust
//! Headless audio detection — type alias for `CategoryDetector<HeadlessAudio>`.

use crate::programs::enums::HeadlessAudio;
use crate::programs::types::CategoryDetector;

/// Headless audio players found on the system.
pub type InstalledHeadlessAudio = CategoryDetector<HeadlessAudio>;
```

- [ ] **Step 5: Replace pkg_mngrs.rs**

```rust
//! Package manager detection — type aliases for language and OS package managers.

use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::types::CategoryDetector;

/// Language-specific package managers found on the system.
pub type InstalledLanguagePackageManagers = CategoryDetector<LanguagePackageManager>;

/// Operating system package managers found on the system.
pub type InstalledOsPackageManagers = CategoryDetector<OsPackageManager>;
```

- [ ] **Step 6: Replace ai_cli.rs with type alias + backward-compat methods**

```rust
//! AI CLI detection — type alias with backward-compatible accessors.

use std::path::PathBuf;

use crate::programs::enums::AiCli;
use crate::programs::types::{CategoryDetector, ExecutableSource};

/// AI-powered CLI coding tools found on the system.
pub type InstalledAiClients = CategoryDetector<AiCli>;

impl InstalledAiClients {
    /// Returns true if Claude Code is installed.
    pub fn claude(&self) -> bool {
        self.is_installed(AiCli::Claude)
    }

    /// Returns true if OpenCode is installed.
    pub fn opencode(&self) -> bool {
        self.is_installed(AiCli::Opencode)
    }

    /// Returns true if Roo Code is installed.
    pub fn roo(&self) -> bool {
        self.is_installed(AiCli::Roo)
    }

    /// Returns true if Gemini CLI is installed.
    pub fn gemini_cli(&self) -> bool {
        self.is_installed(AiCli::GeminiCli)
    }

    /// Returns true if Aider is installed.
    pub fn aider(&self) -> bool {
        self.is_installed(AiCli::Aider)
    }

    /// Returns true if Codex CLI is installed.
    pub fn codex(&self) -> bool {
        self.is_installed(AiCli::Codex)
    }

    /// Returns true if Goose is installed.
    pub fn goose(&self) -> bool {
        self.is_installed(AiCli::Goose)
    }

    /// Returns true if Kimi Code CLI is installed.
    pub fn kimi_cli(&self) -> bool {
        self.is_installed(AiCli::KimiCli)
    }

    /// Returns true if Qwen Code CLI is installed.
    pub fn qwen_cli(&self) -> bool {
        self.is_installed(AiCli::QwenCli)
    }

    /// Mark a client as installed (for testing purposes).
    pub fn with_client(self, client: AiCli) -> Self {
        let info = client.info();
        let fake_path = PathBuf::from(format!("/usr/bin/{}", info.binary_name));
        self.with_program(client, fake_path, ExecutableSource::Path)
    }
}
```

- [ ] **Step 7: Replace tts_clients.rs with type alias + backward-compat methods**

```rust
//! TTS client detection — type alias with backward-compatible accessors.

use std::path::PathBuf;

use crate::programs::enums::TtsClient;
use crate::programs::types::{CategoryDetector, ExecutableSource};

/// Text-to-speech clients found on the system.
pub type InstalledTtsClients = CategoryDetector<TtsClient>;

impl InstalledTtsClients {
    /// Returns true if macOS Say is installed.
    pub fn say(&self) -> bool {
        self.is_installed(TtsClient::Say)
    }

    /// Returns true if eSpeak is installed.
    pub fn espeak(&self) -> bool {
        self.is_installed(TtsClient::Espeak)
    }

    /// Returns true if eSpeak-NG is installed.
    pub fn espeak_ng(&self) -> bool {
        self.is_installed(TtsClient::EspeakNg)
    }

    /// Returns true if Festival is installed.
    pub fn festival(&self) -> bool {
        self.is_installed(TtsClient::Festival)
    }

    /// Returns true if Mimic is installed.
    pub fn mimic(&self) -> bool {
        self.is_installed(TtsClient::Mimic)
    }

    /// Returns true if Mimic3 is installed.
    pub fn mimic3(&self) -> bool {
        self.is_installed(TtsClient::Mimic3)
    }

    /// Returns true if Piper is installed.
    pub fn piper(&self) -> bool {
        self.is_installed(TtsClient::Piper)
    }

    /// Returns true if Echogarden is installed.
    pub fn echogarden(&self) -> bool {
        self.is_installed(TtsClient::Echogarden)
    }

    /// Returns true if Balcon is installed.
    pub fn balcon(&self) -> bool {
        self.is_installed(TtsClient::Balcon)
    }

    /// Returns true if Windows SAPI is available.
    pub fn windows_sapi(&self) -> bool {
        self.is_installed(TtsClient::WindowsSapi)
    }

    /// Returns true if gTTS CLI is installed.
    pub fn gtts_cli(&self) -> bool {
        self.is_installed(TtsClient::GttsCli)
    }

    /// Returns true if Coqui TTS is installed.
    pub fn coqui_tts(&self) -> bool {
        self.is_installed(TtsClient::CoquiTts)
    }

    /// Returns true if Sherpa-ONNX is installed.
    pub fn sherpa_onnx(&self) -> bool {
        self.is_installed(TtsClient::SherpaOnnx)
    }

    /// Returns true if Kokoro TTS is installed.
    pub fn kokoro_tts(&self) -> bool {
        self.is_installed(TtsClient::KokoroTts)
    }

    /// Returns true if Pico2Wave is installed.
    pub fn pico2wave(&self) -> bool {
        self.is_installed(TtsClient::Pico2Wave)
    }

    /// Mark a client as installed (for testing purposes).
    pub fn with_client(self, client: TtsClient) -> Self {
        let info = client.info();
        let fake_path = PathBuf::from(format!("/usr/bin/{}", info.binary_name));
        self.with_program(client, fake_path, ExecutableSource::Path)
    }
}
```

- [ ] **Step 8: Update mod.rs exports**

Update `sniff/lib/src/programs/mod.rs` to export `CategoryDetector` and `CategoryEnum`:

```rust
pub use enums::CategoryEnum;
pub use types::CategoryDetector;
```

Remove the `ProgramDetails` export from the `pub use types::` line (it's removed in Task 10).

- [ ] **Step 9: Build and run ALL tests**

Run: `just test` from `sniff/`
Expected: All existing tests pass. The type aliases preserve the public API.

Note: Some tests in the old detector files may need to be moved to the types.rs test module or to a new test file since the detector files are now thin aliases. Check that all test assertions still hold.

- [ ] **Step 10: Commit**

```
refactor(sniff): replace 8 detector structs with CategoryDetector type aliases

Eliminates ~1400 lines of boilerplate. Each detector file is now a thin
type alias with optional backward-compatible convenience methods.
```

---

## Task 8: Convert Program to Tagged Union

**Files:**
- Modify: `sniff/lib/src/programs/inventory.rs`
- Modify: `sniff/lib/src/programs/enums.rs` (add to_program to CategoryEnum impls)

- [ ] **Step 1: Write tests for tagged union**

Add to `inventory.rs` test module:

```rust
#[test]
fn test_program_from_category_enums() {
    use crate::programs::enums::*;

    let p = Program::from(Editor::Vim);
    assert_eq!(p.display_name(), "Vim");

    let p = Program::from(Utility::Ripgrep);
    assert_eq!(p.display_name(), "ripgrep");
}

#[test]
fn test_program_display_uses_binary_name() {
    use crate::programs::enums::*;

    let p = Program::from(Editor::Vim);
    assert_eq!(p.to_string(), "vim");
}

#[test]
fn test_program_serde_roundtrip() {
    use crate::programs::enums::*;

    let p = Program::from(Editor::Vim);
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(json, "\"vim\"");
    let p2: Program = serde_json::from_str(&json).unwrap();
    assert_eq!(p, p2);
}

#[test]
fn test_program_iter_covers_all_categories() {
    let all: Vec<Program> = Program::iter().collect();
    // Total should match sum of all category enum counts
    use strum::EnumCount;
    let expected = Editor::COUNT + Utility::COUNT + LanguagePackageManager::COUNT
        + OsPackageManager::COUNT + TtsClient::COUNT + TerminalApp::COUNT
        + HeadlessAudio::COUNT + AiCli::COUNT;
    assert_eq!(all.len(), expected);
}
```

- [ ] **Step 2: Redefine Program as tagged union**

Replace the flat `Program` enum in `inventory.rs` with:

```rust
use crate::programs::enums::{
    AiCli, CategoryEnum, Editor, HeadlessAudio, LanguagePackageManager,
    OsPackageManager, TerminalApp, TtsClient, Utility,
};
use crate::programs::schema::{ProgramInfo, ProgramMetadata};

/// Unified enum spanning all program categories.
///
/// Each variant wraps a category-specific enum, making the relationship
/// between categories and the unified type structural rather than manual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Program {
    Editor(Editor),
    Utility(Utility),
    LanguagePackageManager(LanguagePackageManager),
    OsPackageManager(OsPackageManager),
    TtsClient(TtsClient),
    TerminalApp(TerminalApp),
    HeadlessAudio(HeadlessAudio),
    AiCli(AiCli),
}
```

- [ ] **Step 3: Implement ProgramMetadata for Program**

```rust
impl ProgramMetadata for Program {
    fn info(&self) -> &'static ProgramInfo {
        match self {
            Program::Editor(e) => e.info(),
            Program::Utility(u) => u.info(),
            Program::LanguagePackageManager(l) => l.info(),
            Program::OsPackageManager(o) => o.info(),
            Program::TtsClient(t) => t.info(),
            Program::TerminalApp(t) => t.info(),
            Program::HeadlessAudio(h) => h.info(),
            Program::AiCli(a) => a.info(),
        }
    }
}
```

- [ ] **Step 4: Implement From conversions**

```rust
impl From<Editor> for Program {
    fn from(e: Editor) -> Self { Program::Editor(e) }
}
impl From<Utility> for Program {
    fn from(u: Utility) -> Self { Program::Utility(u) }
}
impl From<LanguagePackageManager> for Program {
    fn from(l: LanguagePackageManager) -> Self { Program::LanguagePackageManager(l) }
}
impl From<OsPackageManager> for Program {
    fn from(o: OsPackageManager) -> Self { Program::OsPackageManager(o) }
}
impl From<TtsClient> for Program {
    fn from(t: TtsClient) -> Self { Program::TtsClient(t) }
}
impl From<TerminalApp> for Program {
    fn from(t: TerminalApp) -> Self { Program::TerminalApp(t) }
}
impl From<HeadlessAudio> for Program {
    fn from(h: HeadlessAudio) -> Self { Program::HeadlessAudio(h) }
}
impl From<AiCli> for Program {
    fn from(a: AiCli) -> Self { Program::AiCli(a) }
}
```

- [ ] **Step 5: Implement Display, Serialize, Deserialize for backward compatibility**

```rust
impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.binary_name())
    }
}

impl Serialize for Program {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.binary_name())
    }
}

impl<'de> Deserialize<'de> for Program {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Program::from_binary_name(&name).ok_or_else(|| {
            serde::de::Error::custom(format!("unknown program: {}", name))
        })
    }
}

impl Program {
    /// Look up a Program by binary name.
    pub fn from_binary_name(name: &str) -> Option<Self> {
        // Search each category
        for e in Editor::iter() {
            if e.binary_name() == name { return Some(Program::Editor(e)); }
        }
        for u in Utility::iter() {
            if u.binary_name() == name { return Some(Program::Utility(u)); }
        }
        for l in LanguagePackageManager::iter() {
            if l.binary_name() == name { return Some(Program::LanguagePackageManager(l)); }
        }
        for o in OsPackageManager::iter() {
            if o.binary_name() == name { return Some(Program::OsPackageManager(o)); }
        }
        for t in TtsClient::iter() {
            if t.binary_name() == name { return Some(Program::TtsClient(t)); }
        }
        for t in TerminalApp::iter() {
            if t.binary_name() == name { return Some(Program::TerminalApp(t)); }
        }
        for h in HeadlessAudio::iter() {
            if h.binary_name() == name { return Some(Program::HeadlessAudio(h)); }
        }
        for a in AiCli::iter() {
            if a.binary_name() == name { return Some(Program::AiCli(a)); }
        }
        None
    }

    /// Iterate over all programs across all categories.
    pub fn iter() -> impl Iterator<Item = Program> {
        Editor::iter().map(Program::from)
            .chain(Utility::iter().map(Program::from))
            .chain(LanguagePackageManager::iter().map(Program::from))
            .chain(OsPackageManager::iter().map(Program::from))
            .chain(TtsClient::iter().map(Program::from))
            .chain(TerminalApp::iter().map(Program::from))
            .chain(HeadlessAudio::iter().map(Program::from))
            .chain(AiCli::iter().map(Program::from))
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p sniff --lib programs::inventory`
Expected: PASS

- [ ] **Step 7: Commit**

```
refactor(sniff): convert Program to tagged union wrapping category enums
```

---

## Task 9: Add InstallationMethod::manager_binary() and Simplify Installer

**Files:**
- Modify: `sniff/lib/src/programs/types.rs:142-234`
- Modify: `sniff/lib/src/programs/installer.rs:130-177`

- [ ] **Step 1: Write test for manager_binary()**

Add to `types.rs` test module:

```rust
#[test]
fn test_installation_method_manager_binary() {
    assert_eq!(InstallationMethod::Brew("vim").manager_binary(), "brew");
    assert_eq!(InstallationMethod::Apt("vim").manager_binary(), "apt");
    assert_eq!(InstallationMethod::Cargo("ripgrep").manager_binary(), "cargo");
    assert_eq!(InstallationMethod::Npm("typescript").manager_binary(), "npm");
    assert_eq!(InstallationMethod::RemoteBash("url").manager_binary(), "bash");
}
```

- [ ] **Step 2: Add manager_binary() method**

Add to `impl InstallationMethod` in `types.rs`:

```rust
/// Returns the binary name of the package manager.
///
/// This is the executable that must be present on the system
/// to use this installation method.
pub fn manager_binary(&self) -> &'static str {
    match self {
        InstallationMethod::Npm(_) => "npm",
        InstallationMethod::Pnpm(_) => "pnpm",
        InstallationMethod::Yarn(_) => "yarn",
        InstallationMethod::Bun(_) => "bun",
        InstallationMethod::Cargo(_) => "cargo",
        InstallationMethod::GoModules(_) => "go",
        InstallationMethod::Composer(_) => "composer",
        InstallationMethod::SwiftPm(_) => "swift",
        InstallationMethod::LuaRocks(_) => "luarocks",
        InstallationMethod::VcPkg(_) => "vcpkg",
        InstallationMethod::Conan(_) => "conan",
        InstallationMethod::Nuget(_) => "nuget",
        InstallationMethod::Hex(_) => "mix",
        InstallationMethod::Pip(_) => "pip",
        InstallationMethod::Uv(_) => "uv",
        InstallationMethod::Poetry(_) => "poetry",
        InstallationMethod::Cpan(_) => "cpan",
        InstallationMethod::Cpanm(_) => "cpanm",
        InstallationMethod::Apt(_) => "apt",
        InstallationMethod::Nala(_) => "nala",
        InstallationMethod::Brew(_) => "brew",
        InstallationMethod::Dnf(_) => "dnf",
        InstallationMethod::Pacman(_) => "pacman",
        InstallationMethod::Winget(_) => "winget",
        InstallationMethod::Chocolatey(_) => "choco",
        InstallationMethod::Scoop(_) => "scoop",
        InstallationMethod::Nix(_) => "nix",
        InstallationMethod::RemoteBash(_) => "bash",
    }
}
```

- [ ] **Step 3: Simplify method_available() in installer.rs**

Replace the 28-arm match in `method_available()` with:

```rust
pub(crate) fn method_available(
    method: &InstallationMethod,
    os_pkg_mgrs: &InstalledOsPackageManagers,
    lang_pkg_mgrs: &InstalledLanguagePackageManagers,
) -> bool {
    if method.is_remote_bash() {
        return false;
    }

    let binary = method.manager_binary();

    if method.is_os_package_manager() {
        // Check OS package manager detectors
        OsPackageManager::iter().any(|mgr| {
            mgr.binary_name() == binary && os_pkg_mgrs.is_installed(mgr)
        })
    } else {
        // Check language package manager detectors
        LanguagePackageManager::iter().any(|mgr| {
            mgr.binary_name() == binary && lang_pkg_mgrs.is_installed(mgr)
        })
    }
}
```

Note: `manager_binary()` returns "choco" for Chocolatey but the binary_name for the enum is "choco" too. Similarly "mix" for Hex. Verify all mappings align by checking each pair. If any mismatch, use `manager_name()` comparison instead:

```rust
// Alternative if binary names don't match 1:1:
if method.is_os_package_manager() {
    OsPackageManager::iter().any(|mgr| {
        mgr.info().binary_name == method.manager_binary() && os_pkg_mgrs.is_installed(mgr)
    })
} else {
    LanguagePackageManager::iter().any(|mgr| {
        mgr.info().binary_name == method.manager_binary() && lang_pkg_mgrs.is_installed(mgr)
    })
}
```

Verify by checking enums.rs binary names match types.rs manager_binary() return values. Key ones to check:
- Chocolatey: binary_name "choco", manager_binary "choco" ✓
- Hex: binary_name "mix", manager_binary "mix" ✓
- GoModules: binary_name "go", manager_binary "go" ✓

- [ ] **Step 4: Run installer tests**

Run: `cargo test -p sniff --lib programs::installer`
Expected: PASS

- [ ] **Step 5: Commit**

```
refactor(sniff): add InstallationMethod::manager_binary(), simplify method_available()
```

---

## Task 10: Eliminate PROGRAM_LOOKUP and Clean Up

**Files:**
- Modify: `sniff/lib/src/programs/inventory.rs` (remove PROGRAM_LOOKUP, installation arrays)
- Modify: `sniff/lib/src/programs/types.rs` (remove ProgramDetails)
- Modify: `sniff/lib/src/programs/mod.rs` (update exports)

- [ ] **Step 1: Remove PROGRAM_LOOKUP from inventory.rs**

Delete the entire `pub static PROGRAM_LOOKUP: LazyLock<HashMap<...>>` block (~1700 lines, lines 955-2623).

Delete all the `static *_INSTALL` arrays (lines 172-935) — these have been moved to `enums.rs` in Task 3.

Delete the OS availability constants (lines 941-945) — these have been moved to `enums.rs` in Task 3.

Remove the `LazyLock`, `HashMap` imports. Remove `OsType`, `InstallationMethod`, `ProgramDetails` imports.

- [ ] **Step 2: Update inventory.rs tests**

Update or replace tests that referenced `PROGRAM_LOOKUP`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::programs::enums::*;
    use crate::programs::schema::ProgramMetadata;

    #[test]
    fn test_program_from_all_editors() {
        for editor in Editor::iter() {
            let p = Program::from(editor);
            assert_eq!(p.display_name(), editor.display_name());
        }
    }

    #[test]
    fn test_program_vim_has_metadata() {
        let p = Program::from(Editor::Vim);
        assert_eq!(p.display_name(), "Vim");
        assert!(!p.info().website.is_empty());
    }

    #[test]
    fn test_program_iter_count() {
        let count = Program::iter().count();
        let expected = Editor::COUNT + Utility::COUNT + LanguagePackageManager::COUNT
            + OsPackageManager::COUNT + TtsClient::COUNT + TerminalApp::COUNT
            + HeadlessAudio::COUNT + AiCli::COUNT;
        assert_eq!(count, expected);
    }

    #[test]
    fn test_all_programs_have_valid_metadata() {
        for program in Program::iter() {
            let info = program.info();
            assert!(!info.display_name.is_empty(), "{:?} has empty display_name", program);
            assert!(!info.description.is_empty(), "{:?} has empty description", program);
            assert!(!info.website.is_empty(), "{:?} has empty website", program);
        }
    }

    #[test]
    fn test_program_serde_roundtrip() {
        for program in Program::iter() {
            let json = serde_json::to_string(&program).unwrap();
            let decoded: Program = serde_json::from_str(&json).unwrap();
            assert_eq!(program, decoded, "Roundtrip failed for {:?}", program);
        }
    }

    #[test]
    fn test_program_copy_derive() {
        let p = Program::from(Editor::Vim);
        let p2 = p;
        assert_eq!(p, p2);
    }
}
```

- [ ] **Step 3: Remove ProgramDetails from types.rs**

Delete the `ProgramDetails` struct and its `impl` block (lines 246-292).

- [ ] **Step 4: Update mod.rs exports**

In `mod.rs`, remove:
```rust
pub use inventory::PROGRAM_LOOKUP;
pub use types::ProgramDetails;
```

Add (if not already):
```rust
pub use enums::CategoryEnum;
pub use types::CategoryDetector;
```

- [ ] **Step 5: Run full test suite**

Run: `just test` from `sniff/`
Expected: PASS

- [ ] **Step 6: Commit**

```
refactor(sniff): remove PROGRAM_LOOKUP HashMap and ProgramDetails type

All metadata now lives in ProgramInfo static arrays accessed via
ProgramMetadata::info(). Eliminates ~2700 lines of duplicated data.
```

---

## Task 11: Update External Consumers

**Files:**
- Modify: `unchained-ai/lib/src/primitives/services/agent_status.rs`
- Modify: `biscuit-speaks/lib/src/detection.rs`
- Modify: `sniff/cli/src/install.rs` (if needed)

- [ ] **Step 1: Check if CLI install.rs needs changes**

The CLI uses macros that reference `sniff::programs::InstalledEditors`, etc. These are now type aliases for `CategoryDetector<Editor>`, which implements the same `ProgramDetector` trait with the same methods. The macros should work unchanged.

Run: `cargo build -p sniff-cli`
Expected: PASS — if not, fix any compilation errors.

- [ ] **Step 2: Update unchained-ai agent_status.rs if needed**

The code at `unchained-ai/lib/src/primitives/services/agent_status.rs:130` uses:
```rust
let claude_installed = ai_clients.claude();
let codex_installed = ai_clients.codex();
```

These boolean accessors are preserved in our `InstalledAiClients` type alias impl block (Task 7 Step 6). Should work unchanged.

Run: `cargo build -p unchained-ai`
Expected: PASS

- [ ] **Step 3: Update biscuit-speaks detection.rs if needed**

The code uses `InstalledTtsClients::default().with_client(TtsClient::Say)`. The `with_client` method is preserved in our type alias impl block (Task 7 Step 7). Should work unchanged.

Run: `cargo build -p biscuit-speaks`
Expected: PASS

- [ ] **Step 4: Fix any remaining compilation errors across workspace**

Run: `cargo build -p sniff -p sniff-cli -p biscuit-speaks -p unchained-ai`
Expected: PASS

- [ ] **Step 5: Commit**

```
fix(sniff): update external consumers for refactored program detection API
```

---

## Task 12: Add Test Coverage for Gaps

**Files:**
- Create: `sniff/lib/tests/program_serialization.rs`
- Modify: `sniff/lib/src/programs/enums.rs` (add enum sync tests)
- Modify: `sniff/lib/src/programs/installer.rs` (add installation code tests)
- Modify: `sniff/lib/src/error.rs` (add error variant tests)

### 12a: Serialization Roundtrip Tests

- [ ] **Step 1: Create serialization roundtrip test file**

Create `sniff/lib/tests/program_serialization.rs`:

```rust
//! Serialization roundtrip tests for ProgramsInfo and CategoryDetector.

use sniff::programs::{
    CategoryDetector, Editor, Utility, LanguagePackageManager, OsPackageManager,
    TtsClient, TerminalApp, HeadlessAudio, AiCli, ProgramsInfo,
};

#[test]
fn programs_info_serialization_roundtrip() {
    let info = ProgramsInfo::default();
    let json = serde_json::to_string(&info).unwrap();
    let decoded: ProgramsInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(info, decoded);
}

#[test]
fn editors_serialization_roundtrip() {
    let detector = CategoryDetector::<Editor>::default();
    let json = serde_json::to_string(&detector).unwrap();
    let decoded: CategoryDetector<Editor> = serde_json::from_str(&json).unwrap();
    assert_eq!(detector, decoded);
}

#[test]
fn utilities_serialization_roundtrip() {
    let detector = CategoryDetector::<Utility>::default();
    let json = serde_json::to_string(&detector).unwrap();
    let decoded: CategoryDetector<Utility> = serde_json::from_str(&json).unwrap();
    assert_eq!(detector, decoded);
}

#[test]
fn ai_clients_serialization_roundtrip() {
    let detector = CategoryDetector::<AiCli>::default();
    let json = serde_json::to_string(&detector).unwrap();
    let decoded: CategoryDetector<AiCli> = serde_json::from_str(&json).unwrap();
    assert_eq!(detector, decoded);
}

#[test]
fn tts_clients_serialization_roundtrip() {
    let detector = CategoryDetector::<TtsClient>::default();
    let json = serde_json::to_string(&detector).unwrap();
    let decoded: CategoryDetector<TtsClient> = serde_json::from_str(&json).unwrap();
    assert_eq!(detector, decoded);
}

#[test]
fn all_categories_serialization_roundtrip() {
    // Test all 8 categories in one test
    macro_rules! roundtrip {
        ($t:ty) => {{
            let detector = CategoryDetector::<$t>::default();
            let json = serde_json::to_string(&detector).unwrap();
            let decoded: CategoryDetector<$t> = serde_json::from_str(&json).unwrap();
            assert_eq!(detector, decoded, "Roundtrip failed for {}", std::any::type_name::<$t>());
        }};
    }
    roundtrip!(Editor);
    roundtrip!(Utility);
    roundtrip!(LanguagePackageManager);
    roundtrip!(OsPackageManager);
    roundtrip!(TtsClient);
    roundtrip!(TerminalApp);
    roundtrip!(HeadlessAudio);
    roundtrip!(AiCli);
}

#[test]
fn deserialize_from_legacy_boolean_format() {
    // Backward compatibility: old format used simple booleans
    let json = r#"{"vim": true, "neovim": true, "emacs": false}"#;
    let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
    assert!(detector.is_installed(Editor::Vim));
    assert!(detector.is_installed(Editor::Neovim));
    assert!(!detector.is_installed(Editor::Emacs));
}

#[test]
fn deserialize_from_rich_entry_format() {
    // New format uses ProgramEntry objects
    let json = r#"{"vim": {"installed": true, "name": "Vim", "description": "test", "website": "test"}}"#;
    let detector: CategoryDetector<Editor> = serde_json::from_str(json).unwrap();
    assert!(detector.is_installed(Editor::Vim));
}
```

### 12b: Enum Sync Tests

- [ ] **Step 2: Add enum sync tests to enums.rs**

```rust
#[test]
fn test_all_category_enums_cover_all_programs() {
    use crate::programs::inventory::Program;

    // Count variants across all categories
    let category_total = Editor::COUNT + Utility::COUNT + LanguagePackageManager::COUNT
        + OsPackageManager::COUNT + TtsClient::COUNT + TerminalApp::COUNT
        + HeadlessAudio::COUNT + AiCli::COUNT;

    let program_total = Program::iter().count();
    assert_eq!(
        category_total, program_total,
        "Category enum total ({}) != Program::iter() count ({})",
        category_total, program_total
    );
}

#[test]
fn test_program_mapping_is_bijective() {
    use crate::programs::inventory::Program;
    use std::collections::HashSet;

    let mut seen = HashSet::new();

    // Check that each category variant maps to a unique Program
    macro_rules! check_category {
        ($enum_type:ty) => {
            for variant in <$enum_type>::iter() {
                let program = Program::from(variant);
                assert!(
                    seen.insert(format!("{:?}", program)),
                    "Duplicate Program mapping from {:?}",
                    variant
                );
            }
        };
    }

    check_category!(Editor);
    check_category!(Utility);
    check_category!(LanguagePackageManager);
    check_category!(OsPackageManager);
    check_category!(TtsClient);
    check_category!(TerminalApp);
    check_category!(HeadlessAudio);
    check_category!(AiCli);
}

#[test]
fn test_category_variant_indices_are_contiguous() {
    macro_rules! check_indices {
        ($enum_type:ty) => {{
            let mut indices: Vec<usize> = <$enum_type>::iter()
                .map(|v| v.variant_index())
                .collect();
            indices.sort();
            let expected: Vec<usize> = (0..<$enum_type>::COUNT).collect();
            assert_eq!(
                indices, expected,
                "{} variant indices are not contiguous",
                std::any::type_name::<$enum_type>()
            );
        }};
    }
    check_indices!(Editor);
    check_indices!(Utility);
    check_indices!(LanguagePackageManager);
    check_indices!(OsPackageManager);
    check_indices!(TtsClient);
    check_indices!(TerminalApp);
    check_indices!(HeadlessAudio);
    check_indices!(AiCli);
}
```

### 12c: Installation Code Path Tests

- [ ] **Step 3: Add installation code path tests to installer.rs**

```rust
#[test]
fn test_build_install_command_brew() {
    let method = InstallationMethod::Brew("ripgrep");
    let cmd = build_install_command(&method).unwrap();
    assert_eq!(cmd, vec!["brew", "install", "ripgrep"]);
}

#[test]
fn test_build_install_command_cargo() {
    let method = InstallationMethod::Cargo("ripgrep");
    let cmd = build_install_command(&method).unwrap();
    assert_eq!(cmd, vec!["cargo", "install", "ripgrep"]);
}

#[test]
fn test_build_install_command_apt() {
    let method = InstallationMethod::Apt("ripgrep");
    let cmd = build_install_command(&method).unwrap();
    assert_eq!(cmd, vec!["apt", "install", "-y", "ripgrep"]);
}

#[test]
fn test_build_versioned_install_command_brew() {
    let method = InstallationMethod::Brew("ripgrep");
    let cmd = build_versioned_install_command(&method, "14.0.0").unwrap();
    assert!(cmd.contains(&"14.0.0".to_string()) || cmd.contains(&"ripgrep@14.0.0".to_string()));
}

#[test]
fn test_build_install_command_rejects_shell_metacharacters() {
    let method = InstallationMethod::Brew("ripgrep; rm -rf /");
    let result = build_install_command(&method);
    assert!(result.is_err());
}

#[test]
fn test_select_best_method_prefers_os_over_language() {
    let methods = &[
        InstallationMethod::Cargo("ripgrep"),
        InstallationMethod::Brew("ripgrep"),
    ];
    let os_mgrs = os_pkg_mgrs_with_brew();
    let lang_mgrs = lang_pkg_mgrs_with_cargo();
    let best = select_best_method(methods, &os_mgrs, &lang_mgrs);
    assert!(best.unwrap().is_os_package_manager());
}

#[test]
fn test_select_best_method_returns_none_when_nothing_available() {
    let methods = &[
        InstallationMethod::Brew("ripgrep"),
        InstallationMethod::Apt("ripgrep"),
    ];
    let os_mgrs = empty_os_pkg_mgrs();
    let lang_mgrs = empty_lang_pkg_mgrs();
    let best = select_best_method(methods, &os_mgrs, &lang_mgrs);
    assert!(best.is_none());
}

#[test]
fn test_dry_run_does_not_execute() {
    let method = InstallationMethod::Brew("ripgrep");
    let opts = InstallOptions::dry_run();
    let result = execute_install(&method, &opts).unwrap();
    assert!(!result.executed);
    assert!(result.command.contains("brew"));
}

#[test]
fn test_get_install_command_returns_string() {
    let method = InstallationMethod::Brew("ripgrep");
    let cmd = get_install_command(&method).unwrap();
    assert!(cmd.contains("brew install ripgrep"));
}
```

### 12d: SniffError Variant Tests

- [ ] **Step 4: Add error variant tests to error.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shorthand_not_found_display() {
        let err = SniffError::ShorthandNotFound {
            owner: "user".to_string(),
            repo: "repo".to_string(),
            providers_tried: "GitHub, GitLab".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("user/repo"));
        assert!(msg.contains("GitHub, GitLab"));
    }

    #[test]
    fn test_invalid_credentials_display() {
        let err = SniffError::InvalidCredentials {
            provider: "GitHub".to_string(),
            message: "token expired".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("GitHub"));
        assert!(msg.contains("token expired"));
    }

    #[test]
    fn test_rate_limited_display_with_retry() {
        let err = SniffError::RateLimited {
            provider: "GitHub".to_string(),
            retry_after: Some(60),
        };
        let msg = err.to_string();
        assert!(msg.contains("rate limited"));
        assert!(msg.contains("60s"));
    }

    #[test]
    fn test_rate_limited_display_without_retry() {
        let err = SniffError::RateLimited {
            provider: "GitHub".to_string(),
            retry_after: None,
        };
        let msg = err.to_string();
        assert!(msg.contains("rate limited"));
        assert!(!msg.contains("retry"));
    }

    #[test]
    fn test_installation_error_display() {
        let err = SniffInstallationError::InstallationError {
            pkg: "vim".to_string(),
            cmd: "brew install vim".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("vim"));
        assert!(msg.contains("brew install vim"));
    }

    #[test]
    fn test_not_installable_on_os_display() {
        let err = SniffInstallationError::NotInstallableOnOs {
            pkg: "winget".to_string(),
            os: "macos".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("winget"));
        assert!(msg.contains("macos"));
    }

    #[test]
    fn test_missing_package_manager_display() {
        let err = SniffInstallationError::MissingPackageManager {
            pkg: "ripgrep".to_string(),
            manager: "brew".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("ripgrep"));
        assert!(msg.contains("brew"));
    }
}
```

- [ ] **Step 5: Run all tests**

Run: `just test` from `sniff/`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```
test(sniff): add serialization roundtrip, enum sync, installation, and error tests
```

---

## Summary of Estimated Impact

| Metric | Before | After |
|--------|--------|-------|
| Detector boilerplate | ~1400 lines across 7 files | ~150 lines (type aliases + compat methods) |
| PROGRAM_LOOKUP | ~2700 lines | 0 (merged into ProgramInfo arrays) |
| ProgramDetails type | 47 lines + referenced everywhere | Removed |
| *_details() functions | 8 functions × ~30 lines = ~240 lines | 0 (structural via tagged union) |
| method_available() match arms | 28 arms | ~5 lines (uses manager_binary()) |
| Test coverage gaps | 7 identified | All addressed |
| **Net lines removed** | | **~4000+** |
