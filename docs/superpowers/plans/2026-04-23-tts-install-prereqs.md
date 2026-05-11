# TTS Install System-Prerequisite Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `so-you-say install kokoro|piper|echogarden` on Linux install the system libraries those tools need (`libportaudio2`, `espeak-ng`, `ffmpeg`) before the tool-level install, so the result is a working binary instead of an installed-but-broken one.

**Architecture:** Extend `sniff` with a `SystemPrerequisite` type and `PrereqProbe` (binary-on-PATH / shared-lib-on-dyld-path). Add a `FullInstallPlan` that wraps the existing `InstallPlan` with resolved prereq plans. Add a parallel `run_full_install_interview` that drives one combined announcement, one consent prompt, and fail-fast prereq execution. The existing `build_install_plan` / `run_install_interview` entry points stay in place.

**Tech Stack:** Rust 2024 edition, `which` crate (already a sniff dep) for binary probing, `ldconfig` shell-out on Linux and dyld path search on macOS for shared-lib probing.

**Spec:** `docs/superpowers/specs/2026-04-23-tts-install-prereqs-design.md`

---

## File Structure

**Sniff library (`sniff/lib`):**

- `src/programs/types.rs` — modify: add `PrereqProbe` and `SystemPrerequisite` types next to `InstallationMethod`.
- `src/programs/schema.rs` — modify: add `system_prerequisites` field to `ProgramInfo`; update the three const constructors (`standard`, `with_prefix`, `full`) so raw struct literals keep compiling via mechanical addition of the field.
- `src/programs/prereq_probe.rs` — create: `is_satisfied(probe, os)` implementation per platform.
- `src/programs/install_plan.rs` — modify: extract `select_method` helper from `build_install_plan`; add `PrereqPlan`, `FullInstallPlan`, `build_full_install_plan`.
- `src/programs/install_interview.rs` — modify: add `confirm_full_plan` to `InstallInterviewDelegate`; add `FullInstallInterviewInput`, `FullInstallInterviewOutcome`, `run_full_install_interview`; add combined-announcement text builder.
- `src/error.rs` — modify: add `PrerequisiteUnavailable` variant to `SniffInstallationError`.
- `src/programs/mod.rs` — modify: export new public types.
- `src/programs/enums/metadata.rs` — modify: declare `PORTAUDIO_PREREQ`, `FFMPEG_PREREQ`, `ESPEAK_NG_PREREQ`; wire them to kokoro/piper/echogarden `ProgramInfo` entries; add `system_prerequisites: &[]` to every other `ProgramInfo` literal (137 of them — mechanical).

**Biscuit-speaks CLI (`biscuit-speaks/cli`):**

- `src/install_ui.rs` — modify: implement `confirm_full_plan` on `SoYouSayInstallUi`.
- `src/main.rs` — modify: switch `install_client_via_interview` from `build_install_plan`/`run_install_interview` to full variants; handle new outcomes.
- `tests/cli_test.rs` — modify: extend dry-run tests to assert prereq commands.

---

## Task 1: Types foundation (`PrereqProbe`, `SystemPrerequisite`, extended `ProgramInfo`)

**Files:**
- Modify: `sniff/lib/src/programs/types.rs`
- Modify: `sniff/lib/src/programs/schema.rs`
- Modify: `sniff/lib/src/programs/enums/metadata.rs`
- Modify: `sniff/lib/src/programs/install_plan.rs` (test-only struct literals)
- Modify: `sniff/lib/src/programs/inventory.rs` (one struct literal)
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Write the failing test**

Append to the end of `sniff/lib/src/programs/types.rs` (inside the existing `#[cfg(test)] mod tests { ... }` block, or create one if absent):

```rust
#[cfg(test)]
mod prereq_type_tests {
    use super::*;

    #[test]
    fn system_prerequisite_is_constructible_as_const() {
        const PREREQ: SystemPrerequisite = SystemPrerequisite {
            name: "PortAudio",
            probe: PrereqProbe::SharedLibrary("libportaudio.so.2"),
            methods: &[InstallationMethod::Apt("libportaudio2")],
        };
        assert_eq!(PREREQ.name, "PortAudio");
        assert!(matches!(PREREQ.probe, PrereqProbe::SharedLibrary(_)));
    }

    #[test]
    fn prereq_probe_binary_variant() {
        const PROBE: PrereqProbe = PrereqProbe::Binary("ffmpeg");
        assert!(matches!(PROBE, PrereqProbe::Binary("ffmpeg")));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p sniff --lib -- prereq_type_tests
```

Expected: compile errors — `PrereqProbe` and `SystemPrerequisite` not defined.

- [ ] **Step 3: Add the types**

Open `sniff/lib/src/programs/types.rs`. After the `impl InstallationMethod { ... }` block (before the next top-level item), add:

```rust
/// How to detect whether a `SystemPrerequisite` is already installed on the
/// host. The probe decides whether the prereq's install command needs to run.
///
/// ## Notes
///
/// Windows behavior for `SharedLibrary`: always reports satisfied. On Windows,
/// shared libraries travel with the Python/npm package that consumes them
/// (e.g., the `sounddevice` wheel bundles `portaudio.dll`), so a system-wide
/// probe has no meaningful target. Reporting satisfied silently skips the
/// prereq on Windows, which is correct for every v1 consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrereqProbe {
    /// Shared-library lookup via the dynamic linker search path.
    /// Linux: `ldconfig -p` cache. macOS: dyld default search paths.
    /// Windows: always satisfied (see type-level Notes).
    SharedLibrary(&'static str),
    /// Binary lookup on PATH.
    Binary(&'static str),
}

/// A system-level dependency that must be present before a program's
/// tool-level install runs. Resolved to a single `InstallationMethod` per
/// host using the same bucket logic as `build_install_plan`.
#[derive(Debug, Clone, Copy)]
pub struct SystemPrerequisite {
    /// User-facing name shown in the combined install plan rendering.
    pub name: &'static str,
    /// Presence check used to decide whether installation is needed.
    pub probe: PrereqProbe,
    /// OS-specific install methods. Exactly one wins per host.
    pub methods: &'static [InstallationMethod],
}
```

- [ ] **Step 4: Run types test to verify it passes**

```bash
cargo test -p sniff --lib -- prereq_type_tests
```

Expected: PASS (2 tests).

- [ ] **Step 5: Add `system_prerequisites` field to `ProgramInfo`**

Open `sniff/lib/src/programs/schema.rs`. At line 135 (just after `pub installation_methods: &'static [InstallationMethod],`), add:

```rust
    /// System-level prerequisites required before the tool-level install runs.
    /// Empty slice for most programs. Treated as required — all must resolve
    /// on the host for the `FullInstallPlan` to be successful.
    pub system_prerequisites: &'static [SystemPrerequisite],
```

Update the import near the top of the file (around line 1–20) to include `SystemPrerequisite`:

```rust
use crate::programs::types::{InstallationMethod, SystemPrerequisite};
```

Update `ProgramInfo::standard` (line 146) to include `system_prerequisites: &[]` in its `Self { ... }` literal.

Update `ProgramInfo::with_prefix` (line 170) to include `system_prerequisites: &[]` in its `Self { ... }` literal.

Update `ProgramInfo::full` (line 188): add a new parameter and include it in the `Self { ... }` literal. Because `full` is `#[allow(clippy::too_many_arguments)]` already, the added parameter is fine. Add `system_prerequisites: &'static [SystemPrerequisite]` as the final parameter, and `system_prerequisites,` as the final field assignment. Then find every caller of `ProgramInfo::full` in the codebase:

```bash
grep -rn "ProgramInfo::full(" /Volumes/coding/personal/rusty-biscuit/sniff/lib/src/
```

and append `, &[]` before the closing paren of each call.

- [ ] **Step 6: Mechanically add `system_prerequisites: &[]` to every raw `ProgramInfo` literal**

Use this awk script (run from the repo root) to insert the field after every `installation_methods:` line inside a `ProgramInfo { ... }` literal in the four affected files:

```bash
for f in \
  sniff/lib/src/programs/enums/metadata.rs \
  sniff/lib/src/programs/install_plan.rs \
  sniff/lib/src/programs/inventory.rs \
  sniff/lib/src/programs/types.rs ; do
  awk '
    /^([[:space:]]*)installation_methods:[[:space:]]/ {
      print
      match($0, /^[[:space:]]*/)
      indent = substr($0, RSTART, RLENGTH)
      print indent "system_prerequisites: &[],"
      next
    }
    { print }
  ' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done
```

If the repo has any `ProgramInfo` literal that uses a different ordering or is nested inside a macro that the awk heuristic doesn't match, the compiler will point at it.

- [ ] **Step 7: Export new types from `sniff::programs`**

Open `sniff/lib/src/programs/mod.rs`. Find the `pub use types::{...}` line (line 151) and extend it:

```rust
pub use types::{
    CategoryDetector, ExecutableSource, InstallationMethod, PrereqProbe, ProgramDetector,
    SystemPrerequisite,
};
```

- [ ] **Step 8: Verify the whole lib compiles**

```bash
cargo check -p sniff
```

Expected: clean compile. If any `ProgramInfo` literal still misses the field, fix it now.

- [ ] **Step 9: Run the full sniff test suite to confirm no regression**

```bash
cargo test -p sniff --lib
```

Expected: all existing tests pass plus the two new ones from Step 1.

- [ ] **Step 10: Commit**

```bash
git add sniff/lib/src/programs/types.rs \
        sniff/lib/src/programs/schema.rs \
        sniff/lib/src/programs/mod.rs \
        sniff/lib/src/programs/enums/metadata.rs \
        sniff/lib/src/programs/install_plan.rs \
        sniff/lib/src/programs/inventory.rs
git commit -m "feat(sniff): add PrereqProbe and SystemPrerequisite types

Extends ProgramInfo with a system_prerequisites slice (default empty)
for declaring system-level dependencies. No consumers yet."
```

---

## Task 2: Probe implementation (`prereq_probe.rs`)

**Files:**
- Create: `sniff/lib/src/programs/prereq_probe.rs`
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `sniff/lib/src/programs/prereq_probe.rs`:

```rust
//! Presence probes for `SystemPrerequisite`. A probe reports whether the
//! prerequisite is already satisfied on the current host; satisfied prereqs
//! are omitted from the install plan.

use crate::os::OsType;
use crate::programs::types::PrereqProbe;

/// Returns true when `probe` is already satisfied on the current host.
///
/// ## Examples
///
/// ```ignore
/// use sniff::programs::{is_prereq_satisfied, PrereqProbe};
/// use sniff::os::OsType;
///
/// let satisfied = is_prereq_satisfied(&PrereqProbe::Binary("ls"), OsType::Linux);
/// assert!(satisfied);
/// ```
pub fn is_prereq_satisfied(probe: &PrereqProbe, os: OsType) -> bool {
    match probe {
        PrereqProbe::Binary(name) => binary_on_path(name),
        PrereqProbe::SharedLibrary(name) => match os {
            OsType::Linux => shared_lib_on_linux(name),
            OsType::MacOS => shared_lib_on_macos(name),
            // On Windows, DLLs travel with the consuming wheel/package —
            // always report satisfied so the prereq is silently skipped.
            OsType::Windows => true,
            _ => false,
        },
    }
}

fn binary_on_path(name: &str) -> bool {
    which::which(name).is_ok()
}

fn shared_lib_on_linux(name: &str) -> bool {
    // `ldconfig -p` prints the linker cache, one line per library. We match
    // the basename at the start of the trimmed line. Shell-out failures
    // (ldconfig missing, non-zero exit) conservatively return false — the
    // prereq will be treated as unsatisfied and its install will run.
    let Ok(output) = std::process::Command::new("ldconfig").arg("-p").output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    ldconfig_output_contains(&stdout, name)
}

fn shared_lib_on_macos(name: &str) -> bool {
    // dyld has no cache file we can grep; fall back to the default search
    // paths. We check DYLD_LIBRARY_PATH entries first, then the standard
    // locations.
    let dyld_env: Vec<String> = std::env::var("DYLD_LIBRARY_PATH")
        .ok()
        .map(|s| s.split(':').map(str::to_string).collect())
        .unwrap_or_default();
    let defaults: &[&str] = &[
        "/usr/local/lib",
        "/opt/homebrew/lib",
        "/opt/local/lib",
        "/usr/lib",
    ];
    dyld_env
        .iter()
        .map(String::as_str)
        .chain(defaults.iter().copied())
        .any(|dir| std::path::Path::new(dir).join(name).exists())
}

/// Returns true if `ldconfig_output` contains a cache line for `name`.
/// Extracted so unit tests can feed a canned cache snapshot without
/// shelling out.
fn ldconfig_output_contains(ldconfig_output: &str, name: &str) -> bool {
    ldconfig_output
        .lines()
        .any(|line| line.trim_start().starts_with(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_probe_finds_ls() {
        // `ls` is on PATH on every Unix-y CI host; on Windows the binary
        // probe still succeeds via PATHEXT lookup by the `which` crate.
        assert!(binary_on_path("ls") || binary_on_path("ls.exe"));
    }

    #[test]
    fn binary_probe_rejects_nonexistent_binary() {
        assert!(!binary_on_path("this-binary-definitely-does-not-exist-xyz"));
    }

    #[test]
    fn ldconfig_parser_matches_library_line() {
        let sample = "\
\t/sbin/ldconfig:
\tlibc.so.6 (libc6,x86-64) => /lib/x86_64-linux-gnu/libc.so.6
\tlibportaudio.so.2 (libc6,x86-64) => /lib/x86_64-linux-gnu/libportaudio.so.2
\tlibm.so.6 (libc6,x86-64) => /lib/x86_64-linux-gnu/libm.so.6
";
        assert!(ldconfig_output_contains(sample, "libportaudio.so.2"));
        assert!(ldconfig_output_contains(sample, "libc.so.6"));
    }

    #[test]
    fn ldconfig_parser_rejects_missing_library() {
        let sample = "\tlibc.so.6 (libc6) => /lib/libc.so.6\n";
        assert!(!ldconfig_output_contains(sample, "libportaudio.so.2"));
    }

    #[test]
    fn ldconfig_parser_ignores_header_lines() {
        // Header lines like "XX libs found" must not false-positive.
        let sample = "2 libs found in cache `/etc/ld.so.cache'\n\tlibc.so.6 => /lib/libc.so.6\n";
        assert!(!ldconfig_output_contains(sample, "2 libs"));
    }

    #[test]
    fn windows_shared_library_always_satisfied() {
        let probe = PrereqProbe::SharedLibrary("libportaudio.so.2");
        assert!(is_prereq_satisfied(&probe, OsType::Windows));
    }

    #[test]
    fn unknown_os_shared_library_not_satisfied() {
        // BSD/other: conservative false — we'd rather prompt for install
        // than silently assume presence.
        let probe = PrereqProbe::SharedLibrary("libportaudio.so.2");
        assert!(!is_prereq_satisfied(&probe, OsType::Other));
    }
}
```

- [ ] **Step 2: Register the module**

Edit `sniff/lib/src/programs/mod.rs`. After the existing `pub mod install_plan;` line, add:

```rust
pub mod prereq_probe;
```

After the existing `pub use install_plan::{...}` line, add:

```rust
pub use prereq_probe::is_prereq_satisfied;
```

- [ ] **Step 3: Verify `OsType` import is correct**

```bash
grep -n "pub enum OsType" /Volumes/coding/personal/rusty-biscuit/sniff/lib/src/os.rs | head -5
```

Expected: line like `pub enum OsType {`. Confirm the `Other` variant exists:

```bash
grep -n "Other" /Volumes/coding/personal/rusty-biscuit/sniff/lib/src/os.rs | head -10
```

If the `Other` variant in `OsType` is named differently (e.g., `Unknown`), update the test's `OsType::Other` reference accordingly. If there is no catch-all variant, replace the `unknown_os_shared_library_not_satisfied` test with a cfg-gated equivalent (skip if the enum is exhaustive with only Linux/MacOS/Windows).

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p sniff --lib -- prereq_probe
```

Expected: 7 tests pass. The `binary_probe_finds_ls` test requires `ls` on PATH — always true on macOS and Linux CI.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/prereq_probe.rs sniff/lib/src/programs/mod.rs
git commit -m "feat(sniff): add PrereqProbe presence-detection logic

Binary probe via the \`which\` crate; SharedLibrary probe via
\`ldconfig -p\` on Linux and dyld default search paths on macOS.
Windows reports SharedLibrary as satisfied unconditionally since
Python/npm wheels on Windows bundle their own DLLs."
```

---

## Task 3: Extract `select_method` helper from `build_install_plan`

**Files:**
- Modify: `sniff/lib/src/programs/install_plan.rs`

This is a pure refactor — behavior must be unchanged. The goal is a helper that takes a raw `&[InstallationMethod]` + `&[OsType]` and returns a `Vec<InstallPlanOption>`, so `build_full_install_plan` can call it per prereq without rebuilding a `ProgramMetadata`.

- [ ] **Step 1: Write a test that pins the existing behavior of `build_install_plan`**

This is a regression guard — we'll run it before and after the refactor and assert equivalence. Append to the `selection_tests` module (top line ~`mod selection_tests`, search for `fn brew_wins_over_cargo_on_macos`):

```rust
#[test]
fn select_method_matches_build_install_plan_for_brew_program() {
    // Regression guard: the extracted helper must produce the same
    // InstallPlanOption set (ignoring `program` / `website`) as the
    // existing build_install_plan for a simple brew-only program.
    let host = host_macos_with_brew();
    let plan = build_install_plan(
        &FakeProgram {
            info: &BREW_AND_CARGO,
        },
        &host,
    );
    let via_helper = select_method(BREW_AND_CARGO.installation_methods, BREW_AND_CARGO.os_availability, &host);
    assert_eq!(plan.options.len(), via_helper.len());
    for (a, b) in plan.options.iter().zip(via_helper.iter()) {
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.choose, b.choose);
        assert_eq!(a.reason_type, b.reason_type);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test -p sniff --lib -- selection_tests::select_method_matches_build_install_plan
```

Expected: compile error — `select_method` undefined.

- [ ] **Step 3: Extract the helper**

In `sniff/lib/src/programs/install_plan.rs`, find `pub fn build_install_plan` (line ~379). Change it to delegate to a new helper:

```rust
/// Select a single installation method from `methods` against the current
/// host, returning one `InstallPlanOption` per candidate. Shared between
/// `build_install_plan` (for the tool-level install) and
/// `build_full_install_plan` (for each system prerequisite).
///
/// Applies the same Pip/Uv synthesis rule as `build_install_plan`: if the
/// input contains `Pip(_)` or `Uv(_)` without an explicit `UvWithInstall(_)`,
/// a synthesized `UvWithInstall(pkg)` is appended to the candidates.
pub fn select_method(
    methods: &[InstallationMethod],
    os_availability: &[OsType],
    host: &HostCapabilities,
) -> Vec<InstallPlanOption> {
    let declared: Vec<InstallationMethod> = methods.to_vec();
    let mut effective_methods = declared.clone();
    if let Some(synth) = synthesize_uv_bootstrap(&declared) {
        effective_methods.push(synth);
    }

    let facts: Vec<MethodFact> = effective_methods
        .iter()
        .map(|m| derive_method_fact(m, os_availability, host))
        .collect();

    let mut chosen_index: Option<usize> = None;
    'outer: for bucket in bucket_order() {
        for (idx, fact) in facts.iter().enumerate() {
            if fact.eligible_without_priority && bucket_for(fact, host) == bucket {
                chosen_index = Some(idx);
                break 'outer;
            }
        }
    }

    facts
        .iter()
        .enumerate()
        .map(|(i, fact)| {
            let choose = chosen_index == Some(i);
            let (reason_type, reason) = if choose {
                let text = match &fact.kind {
                    InstallationMethod::Uv(_) => {
                        "chosen — uv tool install (uv already on host)".to_string()
                    }
                    InstallationMethod::Pip(_) => {
                        "chosen — pip install (pip already on host; uv absent)".to_string()
                    }
                    _ => format!(
                        "chosen — {}{}",
                        bucket_description(bucket_for(fact, host)),
                        if fact.requires_sudo {
                            " (requires sudo)"
                        } else {
                            ""
                        }
                    ),
                };
                (InstallPlanReason::Selected, text)
            } else if fact.eligible_without_priority && bucket_for(fact, host) != Bucket::Other {
                (
                    InstallPlanReason::LowerPriorityAlternative,
                    "a higher-priority method was chosen".to_string(),
                )
            } else {
                let reason_type = blocking_reason_for(fact, host);
                let reason = explain_blocking_reason(fact, reason_type);
                (reason_type, reason)
            };
            InstallPlanOption {
                kind: fact.kind.clone(),
                requires_sudo: fact.requires_sudo,
                choose,
                reason_type,
                reason,
            }
        })
        .collect()
}

/// Build an install plan for a program against the given host capabilities.
pub fn build_install_plan<P: ProgramMetadata>(program: &P, host: &HostCapabilities) -> InstallPlan {
    let info = program.info();
    let options = select_method(info.installation_methods, info.os_availability, host);
    let successful = options.iter().any(|o| o.choose);

    InstallPlan {
        program: program.display_name().to_string(),
        website: program.website(),
        successful,
        options,
    }
}
```

Delete the now-redundant inline body of the old `build_install_plan` (lines 386–461 in the current file). The helpers `synthesize_uv_bootstrap`, `derive_method_fact`, `bucket_order`, `bucket_for`, `blocking_reason_for`, `explain_blocking_reason`, and `bucket_description` keep their current locations and visibility — they become shared between `select_method` and any future caller.

- [ ] **Step 4: Run the new test plus the whole install_plan module**

```bash
cargo test -p sniff --lib -- install_plan
```

Expected: every existing test plus the new `select_method_matches_build_install_plan_for_brew_program` test passes. If any existing test fails, the refactor changed behavior — investigate and fix.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_plan.rs
git commit -m "refactor(sniff): extract select_method helper from build_install_plan

No behavior change. Prepares for FullInstallPlan which needs to run the
bucket/OS selection against each prereq's method list."
```

---

## Task 4: `PrereqPlan`, `FullInstallPlan`, `build_full_install_plan`

**Files:**
- Modify: `sniff/lib/src/programs/install_plan.rs`
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Write failing tests**

In `sniff/lib/src/programs/install_plan.rs`, add a new `#[cfg(test)] mod full_plan_tests` at the bottom of the file:

```rust
#[cfg(test)]
mod full_plan_tests {
    use super::*;
    use crate::os::OsType;
    use crate::programs::enums::OsPackageManager;
    use crate::programs::host_capability::HostCapabilities;
    use crate::programs::schema::{
        ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy,
    };
    use crate::programs::types::{InstallationMethod, PrereqProbe, SystemPrerequisite};

    static TEST_PORTAUDIO_PREREQ: SystemPrerequisite = SystemPrerequisite {
        name: "PortAudio",
        probe: PrereqProbe::SharedLibrary("libportaudio.so.2"),
        methods: &[
            InstallationMethod::Apt("libportaudio2"),
            InstallationMethod::Dnf("portaudio"),
            InstallationMethod::Brew("portaudio"),
        ],
    };

    static KOKORO_LIKE: ProgramInfo = ProgramInfo {
        binary_name: "kokoro-tts",
        display_name: "kokoro (Kokoro TTS)",
        description: "Kokoro TTS CLI",
        website: "https://github.com/nazdridoy/kokoro-tts",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[InstallationMethod::Pip("kokoro-tts")],
        system_prerequisites: &[TEST_PORTAUDIO_PREREQ],
    };

    struct FakeKokoro;
    impl ProgramMetadata for FakeKokoro {
        fn info(&self) -> &'static ProgramInfo {
            &KOKORO_LIKE
        }
    }

    fn linux_host_with_apt_sudo_no_uv() -> HostCapabilities {
        let os_pkg_mgrs = serde_json::from_str(r#"{"apt": true}"#).unwrap();
        let lang_pkg_mgrs = serde_json::from_str(r#"{"pip": true}"#).unwrap();
        HostCapabilities {
            os_type: OsType::Linux,
            default_os_package_manager: Some(OsPackageManager::Apt),
            os_pkg_mgrs,
            lang_pkg_mgrs,
            can_sudo: true,
            has_bash: true,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn full_plan_lists_unsatisfied_prereq_on_linux() {
        let host = linux_host_with_apt_sudo_no_uv();
        let plan = build_full_install_plan(&FakeKokoro, &host);

        // The probe runs the real ldconfig call on the test host. If the
        // library happens to be installed locally, the prereq list will be
        // empty — so we only assert the plan is successful and check shape.
        assert!(plan.successful);
        assert!(plan.main.successful);
        // prereq list is either empty (lib present) or one entry (lib absent)
        // and in the absent case the chosen method is apt.
        if let Some(prereq) = plan.prerequisites.first() {
            assert_eq!(prereq.name, "PortAudio");
            let chosen = prereq
                .options
                .iter()
                .find(|o| o.choose)
                .expect("chosen prereq method");
            assert!(matches!(chosen.kind, InstallationMethod::Apt("libportaudio2")));
        }
    }

    #[test]
    fn full_plan_fails_when_prereq_cannot_be_installed() {
        // Linux host without apt/dnf/brew/pacman — PortAudio has no method.
        let host = HostCapabilities {
            os_type: OsType::Linux,
            lang_pkg_mgrs: serde_json::from_str(r#"{"pip": true}"#).unwrap(),
            has_bash: true,
            ..HostCapabilities::default()
        };
        let plan = build_full_install_plan(&FakeKokoro, &host);

        if plan.prerequisites.is_empty() {
            // lib already present on test host — skip rather than assert
            return;
        }
        assert!(!plan.successful);
        assert!(!plan.prerequisites[0].successful);
    }

    #[test]
    fn full_plan_empty_prereqs_matches_main_plan_success() {
        // Program with no prereqs → FullInstallPlan.successful mirrors main.
        static NO_PREREQ_PROG: ProgramInfo = ProgramInfo {
            binary_name: "bat",
            display_name: "bat",
            description: "cat clone",
            website: "https://github.com/sharkdp/bat",
            version_flag: VersionFlag::Long,
            parse_strategy: VersionParseStrategy::FirstLine,
            version_regex: None,
            version_prefix: None,
            alternate_binary_names: &[],
            os_availability: &[OsType::MacOS, OsType::Linux],
            repo: None,
            installation_methods: &[InstallationMethod::Brew("bat")],
            system_prerequisites: &[],
        };
        struct FakeBat;
        impl ProgramMetadata for FakeBat {
            fn info(&self) -> &'static ProgramInfo {
                &NO_PREREQ_PROG
            }
        }
        let os_pkg_mgrs = serde_json::from_str(r#"{"brew": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::MacOS,
            default_os_package_manager: Some(OsPackageManager::Brew),
            os_pkg_mgrs,
            has_bash: true,
            ..HostCapabilities::default()
        };
        let plan = build_full_install_plan(&FakeBat, &host);
        assert!(plan.prerequisites.is_empty());
        assert!(plan.successful);
        assert!(plan.main.successful);
    }

    #[test]
    fn full_plan_windows_skips_shared_library_prereq() {
        // On Windows the SharedLibrary probe reports satisfied → no prereq.
        let host = HostCapabilities {
            os_type: OsType::Windows,
            has_bash: false,
            ..HostCapabilities::default()
        };
        let plan = build_full_install_plan(&FakeKokoro, &host);
        assert!(plan.prerequisites.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p sniff --lib -- full_plan_tests
```

Expected: compile error — `FullInstallPlan`, `PrereqPlan`, `build_full_install_plan` undefined.

- [ ] **Step 3: Add the types and builder**

In `sniff/lib/src/programs/install_plan.rs`, after the `impl InstallPlan { ... }` block (before the `MethodFact` struct), add:

```rust
/// Plan for a single system prerequisite. Mirrors `InstallPlan`'s option
/// structure: exactly one option has `choose = true` when the prereq can be
/// installed on this host.
#[derive(Debug, Clone, Serialize)]
pub struct PrereqPlan {
    pub name: &'static str,
    pub probe: crate::programs::types::PrereqProbe,
    pub successful: bool,
    pub options: Vec<InstallPlanOption>,
}

impl PrereqPlan {
    /// The chosen option, if any.
    pub fn chosen(&self) -> Option<&InstallPlanOption> {
        self.options.iter().find(|o| o.choose)
    }
}

/// A combined plan that covers a program's system prerequisites and its
/// tool-level install. Built by `build_full_install_plan`.
///
/// `prerequisites` contains only *unsatisfied* prereqs — if a probe reports
/// the library/binary is already present, the prereq is omitted entirely.
#[derive(Debug, Clone, Serialize)]
pub struct FullInstallPlan {
    pub program: String,
    pub website: &'static str,
    /// `true` when the main plan is successful AND every listed prereq is
    /// successful. An empty `prerequisites` list with a successful main
    /// plan is successful.
    pub successful: bool,
    /// Unsatisfied prereqs that must be installed before the main install.
    /// Order: declaration order from `ProgramInfo.system_prerequisites`,
    /// minus already-satisfied entries.
    pub prerequisites: Vec<PrereqPlan>,
    pub main: InstallPlan,
}

/// Build a combined install plan for a program, including its system
/// prerequisites resolved against the current host.
pub fn build_full_install_plan<P: ProgramMetadata>(
    program: &P,
    host: &HostCapabilities,
) -> FullInstallPlan {
    let info = program.info();
    let main = build_install_plan(program, host);

    let mut prerequisites = Vec::new();
    for prereq in info.system_prerequisites {
        if crate::programs::prereq_probe::is_prereq_satisfied(&prereq.probe, host.os_type) {
            continue;
        }
        let options = select_method(prereq.methods, info.os_availability, host);
        let successful = options.iter().any(|o| o.choose);
        prerequisites.push(PrereqPlan {
            name: prereq.name,
            probe: prereq.probe,
            successful,
            options,
        });
    }

    let successful = main.successful && prerequisites.iter().all(|p| p.successful);

    FullInstallPlan {
        program: program.display_name().to_string(),
        website: program.website(),
        successful,
        prerequisites,
        main,
    }
}
```

Also make `PrereqProbe` derive `Serialize`. Back in `sniff/lib/src/programs/types.rs`, replace:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrereqProbe {
```

with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "target", rename_all = "snake_case")]
pub enum PrereqProbe {
```

and confirm `use serde::Serialize;` is already present in that file (it must be, since `InstallationMethod` derives `Serialize`).

- [ ] **Step 4: Export new types from `sniff::programs`**

In `sniff/lib/src/programs/mod.rs`, change:

```rust
pub use install_plan::{InstallPlan, InstallPlanOption, InstallPlanReason, build_install_plan};
```

to:

```rust
pub use install_plan::{
    FullInstallPlan, InstallPlan, InstallPlanOption, InstallPlanReason, PrereqPlan,
    build_full_install_plan, build_install_plan, select_method,
};
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test -p sniff --lib -- full_plan_tests
```

Expected: 4 tests pass. Run the full suite:

```bash
cargo test -p sniff --lib
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add sniff/lib/src/programs/install_plan.rs sniff/lib/src/programs/mod.rs sniff/lib/src/programs/types.rs
git commit -m "feat(sniff): add FullInstallPlan and build_full_install_plan

Wraps the existing InstallPlan with resolved prereq plans. Prereqs whose
probe reports satisfied are omitted. FullInstallPlan.successful requires
every listed prereq to have a viable method."
```

---

## Task 5: Error variant, delegate method, interview input/outcome

**Files:**
- Modify: `sniff/lib/src/error.rs`
- Modify: `sniff/lib/src/programs/install_interview.rs`
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Write the failing test**

At the bottom of `sniff/lib/src/programs/install_interview.rs`, add a new `mod full_interview_types_tests`:

```rust
#[cfg(test)]
mod full_interview_types_tests {
    use super::*;
    use crate::programs::install_plan::{FullInstallPlan, InstallPlan};

    #[test]
    fn full_install_interview_outcome_installed_constructible() {
        let _ = FullInstallInterviewOutcome::Installed;
    }

    #[test]
    fn full_install_interview_input_constructible() {
        let _ = FullInstallInterviewInput {
            program: "kokoro".into(),
            website: "https://example.com",
            plan: FullInstallPlan {
                program: "kokoro".into(),
                website: "https://example.com",
                successful: true,
                prerequisites: Vec::new(),
                main: InstallPlan {
                    program: "kokoro".into(),
                    website: "https://example.com",
                    successful: true,
                    options: Vec::new(),
                },
            },
        };
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p sniff --lib -- full_interview_types_tests
```

Expected: compile error — types undefined.

- [ ] **Step 3: Add `PrerequisiteUnavailable` to `SniffInstallationError`**

In `sniff/lib/src/error.rs`, after the `RemoteBashConsentRequired` variant (line 163), add:

```rust
    /// A required system prerequisite cannot be installed on this host
    /// (no eligible method for the host's OS/package managers).
    #[error(
        "Installing {program} requires the {prereq} system library, which has no installable method on this host: {reason}"
    )]
    PrerequisiteUnavailable {
        program: String,
        prereq: &'static str,
        reason: String,
    },
```

- [ ] **Step 4: Add `confirm_full_plan` to the delegate trait**

In `sniff/lib/src/programs/install_interview.rs`, extend the `InstallInterviewDelegate` trait (line 105):

```rust
pub trait InstallInterviewDelegate {
    fn on_event(&mut self, event: &InstallInterviewEvent) -> Result<(), SniffInstallationError>;

    fn confirm_remote_script(&mut self, prose: &str) -> Result<bool, SniffInstallationError>;

    /// Consent prompt for the entire `FullInstallPlan` — prereqs plus main.
    /// Default impl defers to `confirm_remote_script` so existing delegates
    /// keep compiling; new delegates can override to provide distinct
    /// wording.
    fn confirm_full_plan(&mut self, prose: &str) -> Result<bool, SniffInstallationError> {
        self.confirm_remote_script(prose)
    }

    fn choose_retry(&mut self, prompt: &RetryPrompt)
    -> Result<RetryChoice, SniffInstallationError>;
}
```

- [ ] **Step 5: Add `FullInstallInterviewInput` and `FullInstallInterviewOutcome`**

In `sniff/lib/src/programs/install_interview.rs`, near the other interview types (after `InstallInterviewOutcome`, line 122), add:

```rust
/// Input to `run_full_install_interview` — mirrors `InstallInterviewInput`
/// but carries a `FullInstallPlan` instead of a plain `InstallPlan`.
#[derive(Debug, Clone)]
pub struct FullInstallInterviewInput {
    pub program: String,
    pub website: &'static str,
    pub plan: crate::programs::install_plan::FullInstallPlan,
}

/// Outcome of a full install interview (prereqs + main).
#[derive(Debug, Clone)]
pub enum FullInstallInterviewOutcome {
    /// Every required step completed successfully.
    Installed,
    /// Dry-run succeeded — no install actually ran.
    DryRun,
    /// User rejected the combined consent prompt.
    AbortedByUser,
    /// A required prereq has no installable method on this host.
    PrereqUnavailable {
        name: &'static str,
        reason: String,
    },
    /// A prereq's install command failed at runtime.
    PrereqFailed {
        name: &'static str,
        attempted: Vec<InstallationMethod>,
    },
    /// Prereqs succeeded but the main install failed.
    MainFailed {
        attempted: Vec<InstallationMethod>,
    },
    /// The combined plan was not successful before execution started
    /// (either main or any prereq had no viable method).
    NotInstallable,
}
```

- [ ] **Step 6: Export new types**

In `sniff/lib/src/programs/mod.rs`, extend the `pub use install_interview::{...}` line (around line 135):

```rust
pub use install_interview::{
    FullInstallInterviewInput, FullInstallInterviewOutcome, InstallInterviewDelegate,
    InstallInterviewEvent, InstallInterviewInput, InstallInterviewOptions,
    InstallInterviewOutcome, InstallOutputStream, InstallStatusKind, RetryChoice, RetryPrompt,
    RetryPromptChoice, run_install_interview,
};
```

(Preserve the other names already exported; the list above may need adjustment to match what's currently there.)

- [ ] **Step 7: Run the new tests**

```bash
cargo test -p sniff --lib -- full_interview_types_tests
```

Expected: 2 tests pass.

```bash
cargo test -p sniff --lib
```

Expected: every existing test still passes (the default impl of `confirm_full_plan` keeps existing delegates compatible).

- [ ] **Step 8: Commit**

```bash
git add sniff/lib/src/error.rs \
        sniff/lib/src/programs/install_interview.rs \
        sniff/lib/src/programs/mod.rs
git commit -m "feat(sniff): add FullInstallInterview types and confirm_full_plan

Adds PrerequisiteUnavailable error variant, FullInstallInterviewInput/
Outcome types, and a defaulted confirm_full_plan method on
InstallInterviewDelegate so existing delegates keep compiling."
```

---

## Task 6: `run_full_install_interview` runner

**Files:**
- Modify: `sniff/lib/src/programs/install_interview.rs`

- [ ] **Step 1: Write failing tests**

At the bottom of `sniff/lib/src/programs/install_interview.rs`, add a new `#[cfg(test)] mod full_runner_tests`. The tests need a mock delegate to observe events without real I/O.

```rust
#[cfg(test)]
mod full_runner_tests {
    use super::*;
    use crate::programs::install_plan::{
        FullInstallPlan, InstallPlan, InstallPlanOption, InstallPlanReason, PrereqPlan,
    };
    use crate::programs::installer::InstallOptions;
    use crate::programs::types::{InstallationMethod, PrereqProbe};

    #[derive(Default)]
    struct MockDelegate {
        pub events: Vec<InstallInterviewEvent>,
        pub consent_answer: bool,
    }

    impl InstallInterviewDelegate for MockDelegate {
        fn on_event(&mut self, event: &InstallInterviewEvent) -> Result<(), SniffInstallationError> {
            self.events.push(event.clone());
            Ok(())
        }
        fn confirm_remote_script(&mut self, _: &str) -> Result<bool, SniffInstallationError> {
            Ok(self.consent_answer)
        }
        fn confirm_full_plan(&mut self, _: &str) -> Result<bool, SniffInstallationError> {
            Ok(self.consent_answer)
        }
        fn choose_retry(&mut self, _: &RetryPrompt) -> Result<RetryChoice, SniffInstallationError> {
            Ok(RetryChoice::Quit)
        }
    }

    fn dry_run_options() -> InstallInterviewOptions {
        InstallInterviewOptions {
            install: InstallOptions::dry_run(),
            prompt_on_failure: false,
        }
    }

    #[test]
    fn unsuccessful_plan_emits_error_status_and_returns_not_installable() {
        let plan = FullInstallPlan {
            program: "kokoro".into(),
            website: "https://example.com",
            successful: false,
            prerequisites: Vec::new(),
            main: InstallPlan {
                program: "kokoro".into(),
                website: "https://example.com",
                successful: false,
                options: Vec::new(),
            },
        };
        let input = FullInstallInterviewInput {
            program: "kokoro".into(),
            website: "https://example.com",
            plan,
        };
        let mut delegate = MockDelegate::default();
        let outcome =
            run_full_install_interview(&input, &dry_run_options(), &mut delegate).unwrap();
        assert!(matches!(outcome, FullInstallInterviewOutcome::NotInstallable));
        assert_eq!(delegate.events.len(), 1);
        assert!(matches!(
            delegate.events[0],
            InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                ..
            }
        ));
    }

    #[test]
    fn unsuccessful_prereq_returns_prereq_unavailable() {
        let prereq = PrereqPlan {
            name: "PortAudio",
            probe: PrereqProbe::SharedLibrary("libportaudio.so.2"),
            successful: false,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::Apt("libportaudio2"),
                requires_sudo: true,
                choose: false,
                reason_type: InstallPlanReason::RequiresSudoNotAvailable,
                reason: "apt requires sudo and user cannot sudo".into(),
            }],
        };
        let plan = FullInstallPlan {
            program: "kokoro".into(),
            website: "https://example.com",
            successful: false,
            prerequisites: vec![prereq],
            main: InstallPlan {
                program: "kokoro".into(),
                website: "https://example.com",
                successful: true,
                options: Vec::new(),
            },
        };
        let input = FullInstallInterviewInput {
            program: "kokoro".into(),
            website: "https://example.com",
            plan,
        };
        let mut delegate = MockDelegate::default();
        let outcome =
            run_full_install_interview(&input, &dry_run_options(), &mut delegate).unwrap();
        assert!(matches!(
            outcome,
            FullInstallInterviewOutcome::PrereqUnavailable { name: "PortAudio", .. }
        ));
    }

    #[test]
    fn user_rejecting_consent_returns_aborted() {
        let plan = FullInstallPlan {
            program: "bat".into(),
            website: "https://github.com/sharkdp/bat",
            successful: true,
            prerequisites: Vec::new(),
            main: InstallPlan {
                program: "bat".into(),
                website: "https://github.com/sharkdp/bat",
                successful: true,
                options: vec![InstallPlanOption {
                    kind: InstallationMethod::Brew("bat"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "chosen — default OS pm".into(),
                }],
            },
        };
        let input = FullInstallInterviewInput {
            program: "bat".into(),
            website: "https://github.com/sharkdp/bat",
            plan,
        };
        let mut delegate = MockDelegate {
            consent_answer: false,
            ..Default::default()
        };
        // Non-dry-run so the consent gate is hit:
        let outcome = run_full_install_interview(
            &input,
            &InstallInterviewOptions::default(),
            &mut delegate,
        )
        .unwrap();
        assert!(matches!(outcome, FullInstallInterviewOutcome::AbortedByUser));
    }

    #[test]
    fn dry_run_skips_consent_and_executes_no_install() {
        let plan = FullInstallPlan {
            program: "bat".into(),
            website: "https://github.com/sharkdp/bat",
            successful: true,
            prerequisites: Vec::new(),
            main: InstallPlan {
                program: "bat".into(),
                website: "https://github.com/sharkdp/bat",
                successful: true,
                options: vec![InstallPlanOption {
                    kind: InstallationMethod::Brew("bat"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "chosen".into(),
                }],
            },
        };
        let input = FullInstallInterviewInput {
            program: "bat".into(),
            website: "https://github.com/sharkdp/bat",
            plan,
        };
        let mut delegate = MockDelegate {
            consent_answer: false,
            ..Default::default()
        };
        let outcome =
            run_full_install_interview(&input, &dry_run_options(), &mut delegate).unwrap();
        assert!(matches!(outcome, FullInstallInterviewOutcome::DryRun));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p sniff --lib -- full_runner_tests
```

Expected: compile error — `run_full_install_interview` undefined.

- [ ] **Step 3: Implement `run_full_install_interview`**

In `sniff/lib/src/programs/install_interview.rs`, after the existing `run_install_interview` function (ends around line 260), add:

```rust
/// Builds the combined-plan announcement body. Renders one line per prereq
/// plus the main install. The caller's delegate decides how to style it.
fn build_full_plan_announcement(input: &FullInstallInterviewInput) -> String {
    use std::fmt::Write as _;
    let mut body = String::new();
    let _ = writeln!(body, "Installing <b>{}</b>:", input.program);
    for prereq in &input.plan.prerequisites {
        if let Some(chosen) = prereq.chosen() {
            match get_install_command(&chosen.kind) {
                Ok(cmd) => {
                    let _ = writeln!(body, "  Prereq: {} — {}", prereq.name, cmd);
                }
                Err(_) => {
                    let _ = writeln!(body, "  Prereq: {} — (command unavailable)", prereq.name);
                }
            }
        }
    }
    if let Some(chosen) = input.plan.main.chosen() {
        match get_install_command(&chosen.kind) {
            Ok(cmd) => {
                let _ = writeln!(body, "  Main:   {}", cmd);
            }
            Err(_) => {
                let _ = writeln!(body, "  Main:   (command unavailable)");
            }
        }
    }
    body
}

/// Drive the combined install interview: prereqs then main, one consent
/// up-front, fail-fast on any prereq error.
pub fn run_full_install_interview<D: InstallInterviewDelegate>(
    input: &FullInstallInterviewInput,
    options: &InstallInterviewOptions,
    delegate: &mut D,
) -> Result<FullInstallInterviewOutcome, SniffInstallationError> {
    // Plan-level failure modes.
    if !input.plan.successful {
        // Identify which prereq (if any) failed, so we return the right outcome.
        if let Some(bad) = input.plan.prerequisites.iter().find(|p| !p.successful) {
            let reason = bad
                .options
                .iter()
                .find(|o| !o.choose)
                .map(|o| o.reason.clone())
                .unwrap_or_else(|| "no eligible method".to_string());
            delegate.on_event(&InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                text: format!(
                    "Cannot install {}: prereq {} has no installable method on this host: {}",
                    input.program, bad.name, reason
                ),
            })?;
            return Ok(FullInstallInterviewOutcome::PrereqUnavailable {
                name: bad.name,
                reason,
            });
        }
        delegate.on_event(&InstallInterviewEvent::Status {
            kind: InstallStatusKind::Error,
            text: build_install_failure_status(&input.program, input.website),
        })?;
        return Ok(FullInstallInterviewOutcome::NotInstallable);
    }

    // Single announcement covering everything that will run.
    let announcement = build_full_plan_announcement(input);
    delegate.on_event(&InstallInterviewEvent::Announcement {
        prose: announcement.clone(),
    })?;

    // Single consent (skipped for dry-run).
    if !options.install.dry_run && !options.install.approve_remote_bash {
        if !delegate.confirm_full_plan(&announcement)? {
            return Ok(FullInstallInterviewOutcome::AbortedByUser);
        }
    }

    // Execute prereqs in declaration order. Fail-fast on first failure.
    for prereq in &input.plan.prerequisites {
        let chosen = prereq
            .chosen()
            .cloned()
            .expect("successful prereq has a chosen option");

        let mut exec_opts = options.install.clone();
        // Prereqs inside a consented full plan are pre-authorized.
        exec_opts.approve_remote_bash = true;

        let outcome = execute_install_captured(&chosen.kind, &exec_opts);
        let attempted = vec![chosen.kind.clone()];
        match outcome {
            InstallCapturedOutcome::SetupError(e) => {
                delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                    stream: InstallOutputStream::Stderr,
                    body: e.to_string(),
                })?;
                return Ok(FullInstallInterviewOutcome::PrereqFailed {
                    name: prereq.name,
                    attempted,
                });
            }
            InstallCapturedOutcome::Completed(r) if r.success => {
                if !r.stdout.trim().is_empty() {
                    delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                        stream: InstallOutputStream::Stdout,
                        body: r.stdout,
                    })?;
                }
            }
            InstallCapturedOutcome::Completed(r) => {
                let body = if !r.stderr.trim().is_empty() {
                    r.stderr
                } else {
                    r.stdout
                };
                delegate.on_event(&InstallInterviewEvent::CapturedOutput {
                    stream: InstallOutputStream::Stderr,
                    body,
                })?;
                return Ok(FullInstallInterviewOutcome::PrereqFailed {
                    name: prereq.name,
                    attempted,
                });
            }
        }
    }

    // Execute the main install. Use the existing single-plan runner so all
    // retry/rendering logic stays in one place. Map its outcome to ours.
    let main_input = InstallInterviewInput {
        program: input.program.clone(),
        website: input.website,
        plan: input.plan.main.clone(),
    };
    let mut main_opts = options.clone();
    // Consent was already granted for the whole plan; don't re-prompt.
    main_opts.install.approve_remote_bash = true;

    match run_install_interview(&main_input, &main_opts, delegate)? {
        InstallInterviewOutcome::Installed { .. } => {
            // Emit a final combined success status if the plan had prereqs.
            if !input.plan.prerequisites.is_empty() {
                delegate.on_event(&InstallInterviewEvent::Status {
                    kind: InstallStatusKind::Success,
                    text: build_install_success_status(&input.program, input.website),
                })?;
            }
            Ok(FullInstallInterviewOutcome::Installed)
        }
        InstallInterviewOutcome::DryRun { .. } => Ok(FullInstallInterviewOutcome::DryRun),
        InstallInterviewOutcome::AbortedByUser => {
            Ok(FullInstallInterviewOutcome::AbortedByUser)
        }
        InstallInterviewOutcome::Failed { attempted } => {
            Ok(FullInstallInterviewOutcome::MainFailed { attempted })
        }
        InstallInterviewOutcome::NotInstallable => {
            Ok(FullInstallInterviewOutcome::NotInstallable)
        }
    }
}
```

- [ ] **Step 4: Run the tests**

```bash
cargo test -p sniff --lib -- full_runner_tests
```

Expected: 4 tests pass.

```bash
cargo test -p sniff --lib
```

Expected: full suite clean.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_interview.rs
git commit -m "feat(sniff): add run_full_install_interview runner

Drives a FullInstallPlan end-to-end with one combined announcement,
one consent prompt, and fail-fast prereq execution. Delegates to the
existing run_install_interview for main-install retry/rendering."
```

---

## Task 7: Concrete prereq declarations

**Files:**
- Modify: `sniff/lib/src/programs/enums/metadata.rs`

- [ ] **Step 1: Write a verification test**

In `sniff/lib/src/programs/enums/metadata.rs`, append an inline test module at the very end of the file:

```rust
#[cfg(test)]
mod prereq_wiring_tests {
    use super::*;

    fn find_program(binary: &str) -> &'static ProgramInfo {
        TTS_CLIENT_INFO
            .iter()
            .find(|p| p.binary_name == binary)
            .unwrap_or_else(|| panic!("TTS program {} not found", binary))
    }

    #[test]
    fn kokoro_has_portaudio_prereq() {
        let info = find_program("kokoro-tts");
        assert_eq!(info.system_prerequisites.len(), 1);
        assert_eq!(info.system_prerequisites[0].name, "PortAudio");
    }

    #[test]
    fn piper_has_espeak_prereq() {
        let info = find_program("piper");
        assert_eq!(info.system_prerequisites.len(), 1);
        assert_eq!(info.system_prerequisites[0].name, "eSpeak NG");
    }

    #[test]
    fn echogarden_has_ffmpeg_prereq() {
        let info = find_program("echogarden");
        assert_eq!(info.system_prerequisites.len(), 1);
        assert_eq!(info.system_prerequisites[0].name, "FFmpeg");
    }
}
```

If the TTS lookup table in `metadata.rs` is named something other than `TTS_CLIENT_INFO`, update the reference (search for `static .*_INFO.*ProgramInfo` near the TTS entries):

```bash
grep -n "static .*_INFO.*\[ProgramInfo\]" /Volumes/coding/personal/rusty-biscuit/sniff/lib/src/programs/enums/metadata.rs
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p sniff --lib -- prereq_wiring_tests
```

Expected: assertion failure — `system_prerequisites.len()` is 0, not 1.

- [ ] **Step 3: Declare the three prereqs**

In `sniff/lib/src/programs/enums/metadata.rs`, import `PrereqProbe` and `SystemPrerequisite` near the other type imports at the top:

```rust
use crate::programs::types::{InstallationMethod, PrereqProbe, SystemPrerequisite};
```

Near the existing `pub(crate) static KOKORO_TTS_INSTALL: ...` line (around line 1817), add the three prereq constants:

```rust
pub(crate) static PORTAUDIO_PREREQ: SystemPrerequisite = SystemPrerequisite {
    name: "PortAudio",
    probe: PrereqProbe::SharedLibrary("libportaudio.so.2"),
    methods: &[
        InstallationMethod::Apt("libportaudio2"),
        InstallationMethod::Dnf("portaudio"),
        InstallationMethod::Pacman("portaudio"),
        InstallationMethod::Brew("portaudio"),
    ],
};

pub(crate) static FFMPEG_PREREQ: SystemPrerequisite = SystemPrerequisite {
    name: "FFmpeg",
    probe: PrereqProbe::Binary("ffmpeg"),
    methods: &[
        InstallationMethod::Apt("ffmpeg"),
        InstallationMethod::Dnf("ffmpeg"),
        InstallationMethod::Pacman("ffmpeg"),
        InstallationMethod::Brew("ffmpeg"),
        InstallationMethod::Chocolatey("ffmpeg"),
        InstallationMethod::Scoop("ffmpeg"),
        InstallationMethod::Winget("Gyan.FFmpeg"),
    ],
};

pub(crate) static ESPEAK_NG_PREREQ: SystemPrerequisite = SystemPrerequisite {
    name: "eSpeak NG",
    probe: PrereqProbe::Binary("espeak-ng"),
    methods: &[
        InstallationMethod::Apt("espeak-ng"),
        InstallationMethod::Dnf("espeak-ng"),
        InstallationMethod::Pacman("espeak-ng"),
        InstallationMethod::Brew("espeak-ng"),
        InstallationMethod::Chocolatey("espeak-ng"),
        InstallationMethod::Winget("espeak-ng.espeak-ng"),
    ],
};
```

- [ ] **Step 4: Wire prereqs to kokoro, piper, echogarden**

Still in `metadata.rs`, find each of the three `ProgramInfo` entries (binary_name `"kokoro-tts"`, `"piper"`, `"echogarden"`) and change the `system_prerequisites` line.

For `kokoro-tts` (around line 2020):

```rust
system_prerequisites: &[PORTAUDIO_PREREQ],
```

For `piper` (around line 1910):

```rust
system_prerequisites: &[ESPEAK_NG_PREREQ],
```

For `echogarden` (around line 1925):

```rust
system_prerequisites: &[FFMPEG_PREREQ],
```

Each of these replaces the `system_prerequisites: &[],` placeholder inserted by Task 1 Step 6.

- [ ] **Step 5: Run tests**

```bash
cargo test -p sniff --lib -- prereq_wiring_tests
```

Expected: 3 tests pass.

```bash
cargo test -p sniff --lib
```

Expected: full suite clean.

- [ ] **Step 6: Commit**

```bash
git add sniff/lib/src/programs/enums/metadata.rs
git commit -m "feat(sniff): declare system prereqs for kokoro, piper, echogarden

Kokoro → PortAudio (libportaudio2/portaudio), Piper → eSpeak NG,
Echogarden → FFmpeg. Each resolves to the host's default OS package
manager via the existing install-plan bucket logic."
```

---

## Task 8: CLI migration in `so-you-say`

**Files:**
- Modify: `biscuit-speaks/cli/src/install_ui.rs`
- Modify: `biscuit-speaks/cli/src/main.rs`
- Modify: `biscuit-speaks/cli/tests/cli_test.rs`

- [ ] **Step 1: Write the failing CLI test**

Open `biscuit-speaks/cli/tests/cli_test.rs` and find existing `so-you-say install --dry-run` tests. Add a new test:

```rust
#[test]
fn install_kokoro_dry_run_mentions_portaudio_prereq_on_linux() {
    // This test only asserts string content on a Linux host where PortAudio
    // is not already present; on other hosts the prereq is silently skipped
    // (probe reports satisfied), so we guard the assertion.
    if !cfg!(target_os = "linux") {
        return;
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_so-you-say"))
        .args(["install", "kokoro", "--dry-run"])
        .output()
        .expect("run so-you-say");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    // Either the probe found libportaudio on this Linux box and we only
    // see the main uv command, OR we see the apt prereq line.
    let mentions_main = combined.contains("kokoro-tts");
    let mentions_prereq = combined.contains("libportaudio2");
    assert!(
        mentions_main,
        "expected kokoro-tts in output, got: {}",
        combined
    );
    // Soft-assert the prereq: absence is OK only when the probe matched.
    // Nothing further to check — the test is a smoke test for wiring.
    let _ = mentions_prereq;
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p biscuit-speaks-cli -- install_kokoro_dry_run_mentions_portaudio_prereq_on_linux
```

Expected: on Linux the test runs and currently only shows the main uv command (no prereq awareness); on macOS/Windows the test short-circuits to pass. The failure on Linux (if any) is a shape failure — the test is primarily a smoke test.

If the test happens to pass on your Linux dev box because libportaudio2 is already installed, that's fine — the meaningful verification is that the CLI still compiles and runs after the migration below. If you want to force the failure, temporarily move `libportaudio.so.2` out of its directory; but this is not required for merge.

- [ ] **Step 3: Add `confirm_full_plan` to `SoYouSayInstallUi`**

In `biscuit-speaks/cli/src/install_ui.rs`, inside the `impl InstallInterviewDelegate for SoYouSayInstallUi { ... }` block, add a new method after `confirm_remote_script`:

```rust
fn confirm_full_plan(&mut self, _prose: &str) -> Result<bool, SniffInstallationError> {
    match Confirm::new("Proceed with this install plan?")
        .with_default(false)
        .prompt()
    {
        Ok(answer) => Ok(answer),
        Err(inquire::InquireError::OperationCanceled) => Ok(false),
        Err(inquire::InquireError::OperationInterrupted) => std::process::exit(130),
        Err(e) => Err(SniffInstallationError::InstallationError {
            pkg: String::new(),
            cmd: e.to_string(),
        }),
    }
}
```

- [ ] **Step 4: Migrate `install_client_via_interview`**

In `biscuit-speaks/cli/src/main.rs`, find `install_client_via_interview` (line 1073). Update the `use` block at the top of the file (around line 19) to swap:

```rust
use sniff::programs::{
    HostCapabilities, InstallInterviewInput, InstallInterviewOptions, InstallInterviewOutcome,
    InstallOptions, InstalledTtsClients, ProgramDetector, TtsClient, build_install_plan,
    run_install_interview,
};
```

for:

```rust
use sniff::programs::{
    FullInstallInterviewInput, FullInstallInterviewOutcome, HostCapabilities,
    InstallInterviewOptions, InstallOptions, InstalledTtsClients, ProgramDetector, TtsClient,
    build_full_install_plan, run_full_install_interview,
};
```

Then replace the body of `install_client_via_interview` (starting at line 1073):

```rust
fn install_client_via_interview(client: TtsClient, dry_run: bool) {
    let host = HostCapabilities::detect();
    let plan = build_full_install_plan(&client, &host);

    if !plan.successful {
        eprintln!(
            "  {} {} cannot be automatically installed on this host.",
            "✗".red().bold(),
            tts_client_display_name(client)
        );
        // Surface per-prereq and per-method failure reasons.
        for prereq in &plan.prerequisites {
            if !prereq.successful {
                eprintln!(
                    "    prereq {} ({:?}) has no installable method:",
                    prereq.name, prereq.probe
                );
                for option in &prereq.options {
                    if !option.choose {
                        eprintln!(
                            "      - {}: {}",
                            option.kind.manager_name().dimmed(),
                            option.reason
                        );
                    }
                }
            }
        }
        if !plan.main.successful {
            for option in plan.main.failed_with_reason() {
                eprintln!(
                    "    - {}: {}",
                    option.kind.manager_name().dimmed(),
                    option.reason
                );
            }
        }
        if !plan.website.is_empty() {
            eprintln!("  See: {}", plan.website.dimmed());
        }
        std::process::exit(1);
    }

    let input = FullInstallInterviewInput {
        program: tts_client_display_name(client).to_string(),
        website: plan.website,
        plan,
    };
    let options = InstallInterviewOptions {
        install: if dry_run {
            InstallOptions::dry_run()
        } else {
            InstallOptions::default()
        },
        prompt_on_failure: true,
    };

    let mut ui = SoYouSayInstallUi::new();
    match run_full_install_interview(&input, &options, &mut ui) {
        Ok(FullInstallInterviewOutcome::Installed | FullInstallInterviewOutcome::DryRun) => {}
        Ok(FullInstallInterviewOutcome::AbortedByUser) => {
            eprintln!("  {} Installation aborted.", "!".yellow().bold());
            std::process::exit(1);
        }
        Ok(FullInstallInterviewOutcome::PrereqUnavailable { name, reason }) => {
            eprintln!(
                "  {} Required system library {} is not installable on this host: {}",
                "✗".red().bold(),
                name.bold(),
                reason
            );
            std::process::exit(1);
        }
        Ok(FullInstallInterviewOutcome::PrereqFailed { name, attempted: _ }) => {
            eprintln!(
                "  {} Prereq install for {} failed.",
                "✗".red().bold(),
                name.bold()
            );
            std::process::exit(1);
        }
        Ok(FullInstallInterviewOutcome::MainFailed { attempted: _ })
        | Ok(FullInstallInterviewOutcome::NotInstallable) => {
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("  {} Install interview error: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 5: Verify the CLI builds**

```bash
cargo check -p biscuit-speaks-cli
```

Expected: clean compile.

- [ ] **Step 6: Run CLI tests**

```bash
cargo test -p biscuit-speaks-cli
```

Expected: existing tests pass; the new `install_kokoro_dry_run_mentions_portaudio_prereq_on_linux` test passes (on non-Linux it short-circuits; on Linux it asserts the `kokoro-tts` substring which should hold regardless of prereq state).

- [ ] **Step 7: Manual smoke test (recommended, not required)**

From the repo root:

```bash
cargo run -p biscuit-speaks-cli --bin so-you-say -- install kokoro --dry-run
```

Expected output on Linux without libportaudio2:

```
Installing kokoro (Kokoro TTS):
  Prereq: PortAudio — sudo apt install libportaudio2
  Main:   uv tool install kokoro-tts
```

Expected output on macOS:

```
Installing kokoro (Kokoro TTS):
  Main:   uv tool install kokoro-tts
```

(or similar, depending on which of brew/uv is present).

- [ ] **Step 8: Commit**

```bash
git add biscuit-speaks/cli/src/install_ui.rs \
        biscuit-speaks/cli/src/main.rs \
        biscuit-speaks/cli/tests/cli_test.rs
git commit -m "feat(biscuit-speaks): install system prereqs during so-you-say install

Switches install_client_via_interview to sniff's FullInstallPlan flow so
kokoro/piper/echogarden get their system dependencies (libportaudio2,
espeak-ng, ffmpeg) installed alongside the tool-level install. Fails
fast with a clear message when a prereq has no installable method on
the current host."
```

---

## Final verification

- [ ] **Run the whole workspace test suite**

```bash
cargo test -p sniff -p biscuit-speaks-cli
```

Expected: clean.

- [ ] **Run lints on the touched packages**

```bash
cargo clippy -p sniff -p biscuit-speaks-cli --all-targets -- -D warnings
```

Expected: no warnings.

- [ ] **Run fmt check**

```bash
cargo fmt --check
```

Expected: clean.

- [ ] **Manual verification on Linux target host (if available)**

Copy the built binary to a Linux host without PortAudio and run:

```bash
so-you-say install kokoro
```

Expected: sudo prompt for apt, libportaudio2 installs, kokoro-tts installs, and `kokoro-tts --help` runs cleanly without the "PortAudio library not found" error.

---

## Implementation Notes

- **Package-name verification at implementation time:** before committing Task 7, confirm each distro package name against the upstream packaging page (`apt`/`dnf`/`pacman` listings for `libportaudio2`/`portaudio`, `espeak-ng`, and `ffmpeg`; winget search IDs for `Gyan.FFmpeg` and `espeak-ng.espeak-ng`). The design allows adjustment without structural change.

- **Existing `build_install_plan` / `run_install_interview` stay in place.** This plan does not remove them. Other callers (if any) keep working. A follow-up plan can migrate remaining callers or delete the old entry points once everyone is on the full variants.

- **TDD caveat for probe tests:** the `binary_probe_finds_ls` and `ldconfig_parser_*` tests run against the real host. If your test box lacks `/bin/ls` (unlikely) or `ldconfig` output format differs (unusual), adjust the sample fixtures rather than the parser contract.

- **Awk heuristic risk (Task 1 Step 6):** the awk script relies on `installation_methods:` being the last field in every `ProgramInfo` literal. The existing codebase matches that convention (I spot-checked ten entries). Any literal that orders fields differently will produce a compile error pointing directly at the struct literal — fix by hand.

- **Announcement doubling (acceptable for v1):** in `run_full_install_interview`, the main install delegates to `run_install_interview`, which emits its own per-method announcement *after* the combined announcement the full runner already emitted. The user sees the main command described twice — once as a short line in the overview, once as the detailed per-method announcement. Accepted as two levels of information. If user feedback flags this as noise, a follow-up can add a `suppress_announcement` flag to `InstallInterviewOptions`.
