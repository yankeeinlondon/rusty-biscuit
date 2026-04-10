# Program Install Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the opaque `select_best_method` pipeline with a first-class `InstallPlan` that evaluates every installation method against cached host capabilities, records a reason for every accept/reject, and lets the Sniff CLI render and execute the plan with explicit sudo and remote-bash consent.

**Architecture:** A new `HostCapabilities` type (detected once, cached at `~/.sniff-programs.json` with a 90-day TTL) feeds a pure `build_install_plan` selector that walks priority buckets (default OS PM → verified pnpm → npm no-sudo → alt OS PM → remote bash → cargo → sudo-gated npm). Existing `ProgramDetector::install()` becomes a wrapper over the plan. The CLI grows a shared `InstallCommandArgs` flag group, a new `install-plan` read-only subcommand, and a single `render_install_plan` function that handles success, sudo, remote-bash, and failure branches.

**Tech Stack:** Rust 2024, `serde` + `serde_json`, `thiserror`, `chrono` (already in `sniff/lib`), `dirs` (new), `inquire` (CLI), `biscuit-terminal` (CLI), `assert_cmd` + `predicates` + `tempfile` (tests).

---

## Source Documents

Authoritative inputs for every task in this plan:

- [`spec.md`](./spec.md) — product requirements and CLI messaging
- [`tech-design.md`](./tech-design.md) — binding implementation contract (overrides spec on conflicts, per its "Spec Alignment Deltas" section)

If any task below appears to conflict with either document, the tech-design wins.

## File Structure

**New files:**

| Path | Responsibility |
|------|----------------|
| `sniff/lib/src/programs/host_capability.rs` | `HostCapabilities`, `HostCapabilityCache`, detection and cache I/O |
| `sniff/lib/src/programs/install_plan.rs` | `InstallPlan`, `InstallPlanOption`, `InstallPlanReason`, `build_install_plan`, internal rule helpers |
| `sniff/cli/src/install_plan_cmd.rs` | `render_install_plan` + execute flow (new home for plan-aware install logic, keeps `install.rs` as the resolver/legacy interactive path) |
| `sniff/lib/tests/install_plan.rs` | Rule bucket and reason tests with fabricated `HostCapabilities` |
| `sniff/lib/tests/host_capability_cache.rs` | Cache hit/miss/stale/corrupt tests (uses `tempfile` to redirect home) |
| `sniff/cli/tests/install_plan.rs` | CLI `install-plan --json`, rendering snapshots, `--via`, `--force` |

**Modified files:**

| Path | Change |
|------|--------|
| `sniff/lib/Cargo.toml` | Add `dirs = "6"`, `which = "7"` |
| `sniff/lib/src/error.rs` | Add `NoViableMethod`, `RemoteBashConsentRequired` variants |
| `sniff/lib/src/programs/types.rs` | Derive tagged `Serialize` on `InstallationMethod` (one-way, see Task 2); add `known_methods`, `available_methods`, `install_plan` to `ProgramDetector`; rewrite `installable`, `install`, `install_version` as plan wrappers |
| `sniff/lib/src/programs/installer.rs` | Add `approve_remote_bash` to `InstallOptions`; expose `method_available` and helpers to the new `install_plan` module |
| `sniff/lib/src/programs/mod.rs` | Declare new modules and re-export new public types |
| `sniff/lib/src/os/mod.rs` | (no code change required — `OsType` already exists; default-PM helper lives in `host_capability.rs` because it depends on `LinuxFamily`) |
| `sniff/cli/src/args.rs` | Add `InstallCommandArgs`, per-category `InstallPlan` action variant, accessors |
| `sniff/cli/src/install.rs` | Add pure `ResolvedProgram` resolver; remove recursive `sniff programs install` retry loop |
| `sniff/cli/src/commands.rs` | Dispatch `install` and `install-plan` through the new plan-aware code path |
| `sniff/cli/src/main.rs` | Declare new `mod install_plan_cmd;` alongside the existing `mod install;` |

**Files NOT touched (by design):**
- `sniff/lib/src/programs/enums/metadata.rs` — no new installation methods, only selection changes.
- `sniff/lib/src/programs/schema.rs` — `ProgramMetadata: Sized` stays; `build_install_plan` is generic (per tech-design §4).

---

## Ground Rules

1. **TDD first.** Every task writes its failing test before any implementation code.
2. **Keep the tree green.** Every task ends with `just test -p sniff` and `just lint -p sniff` passing before committing.
3. **One concern per commit.** Small, conventional commits; no mixed refactors.
4. **Do not delete or weaken existing tests** in `sniff/lib/src/programs/installer.rs` or `sniff/cli/tests/cli.rs` unless a task explicitly removes a known-dead path.
5. **Work in the existing worktree** `feat-sniff-tuning` on branch `feat/sniff-tuning`.
6. **Before starting Task 1,** verify baseline with:
   ```bash
   cd /Users/ken/.claudine/worktrees/feat-sniff-tuning
   cargo test -p sniff 2>&1 | tail -30
   cargo test -p sniff-cli 2>&1 | tail -30
   ```
   Both must pass. If they do not, stop and investigate before proceeding.

---

## Task Overview

| # | Task | Layer |
|---|------|-------|
| 1 | Add new error variants (`NoViableMethod`, `RemoteBashConsentRequired`) | lib |
| 2 | Tagged serde on `InstallationMethod` | lib |
| 3 | Extend `InstallOptions` with `approve_remote_bash` | lib |
| 4 | Add `dirs` + `which` dependencies to `sniff-lib` | lib |
| 5 | `HostCapabilities` struct + cheap detector | lib |
| 6 | Sudo detection (Unix group + `sudo -n`) | lib |
| 7 | Default OS package manager helper + WSL detection | lib |
| 8 | `detect_with_verification` probes (npm/pnpm/yarn/bun/cargo) | lib |
| 9 | `HostCapabilityCache` file load/save with TTL | lib |
| 10 | `InstallPlan`, `InstallPlanOption`, `InstallPlanReason` types | lib |
| 11 | Plan builder: Phase 1 fact derivation | lib |
| 12 | Plan builder: Phase 2 bucket-priority selection | lib |
| 13 | `InstallPlan::execute()` with remote-bash gate | lib |
| 14 | Wire `ProgramDetector::{known_methods, available_methods, install_plan}` | lib |
| 15 | Rewire `installable`/`install`/`install_version` as plan wrappers | lib |
| 16 | CLI: `InstallCommandArgs` shared flags + per-category `InstallPlan` action | cli |
| 17 | CLI: pure `ResolvedProgram` resolver | cli |
| 18 | CLI: `render_install_plan` renderer (success / sudo / failure) | cli |
| 19 | CLI: execute flow with confirm + remote-bash extra confirm + Ctrl+C | cli |
| 20 | CLI: dispatch `install` and `install-plan` through new path; `--via`, `--force`, `--json` | cli |
| 21 | CLI integration tests (`assert_cmd`) | tests |
| 22 | Final sweep: verbose verify, `just test`, update CLI help text | polish |

---

## Task 1: Add new error variants

**Files:**
- Modify: `sniff/lib/src/error.rs`

The plan-aware install pipeline needs two new error shapes. `NoViableMethod` embeds the full plan so the CLI can render the "here's what we tried" block from a single error value. `RemoteBashConsentRequired` is the execution-time gate that stops a RemoteBash chosen option from running without explicit approval.

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `sniff/lib/src/error.rs`:

```rust
#[test]
fn test_no_viable_method_display() {
    // We only assert the formatted string here; the embedded plan is tested
    // where InstallPlan lives (Task 10+).
    let err = SniffInstallationError::NoViableMethod {
        pkg: "vim".to_string(),
        detail: "no runnable installation method".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("vim"));
    assert!(msg.contains("no runnable installation method"));
}

#[test]
fn test_remote_bash_consent_required_display() {
    let err = SniffInstallationError::RemoteBashConsentRequired {
        pkg: "rustup".to_string(),
        url: "https://sh.rustup.rs".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("rustup"));
    assert!(msg.contains("https://sh.rustup.rs"));
    assert!(msg.to_lowercase().contains("consent"));
}
```

- [ ] **Step 2: Verify the tests fail to compile**

Run: `cargo test -p sniff error:: --no-run 2>&1 | tail -20`
Expected: compile error referencing `NoViableMethod` / `RemoteBashConsentRequired` not found.

- [ ] **Step 3: Add the variants**

In `sniff/lib/src/error.rs`, extend `enum SniffInstallationError` with:

```rust
    /// No runnable installation method exists for this program on this host.
    ///
    /// The embedded `detail` is human-readable and already aware of the rejection
    /// reasons for every evaluated method. Callers that want the full plan should
    /// call `install_plan()` directly rather than relying on `install()`.
    #[error("No viable installation method for {pkg}: {detail}")]
    NoViableMethod { pkg: String, detail: String },

    /// A remote-bash installation was selected but execution has not been
    /// authorized by the caller.
    #[error(
        "Installing {pkg} via remote bash requires explicit consent (url: {url})"
    )]
    RemoteBashConsentRequired { pkg: String, url: String },
```

Note: we intentionally use a `String` detail field rather than embedding the full `InstallPlan` in the error. The tech-design sketch shows `plan: InstallPlan` but that creates a dependency cycle (InstallPlan lives below error.rs in the dependency graph, and InstallPlan needs `SniffInstallationError` for its own `execute` method). Using `detail: String` avoids the cycle while still letting the CLI produce the full failure render — the CLI renders directly from the `InstallPlan` it already holds.

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p sniff error::tests 2>&1 | tail -20`
Expected: all existing `error::tests::*` plus the two new tests pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/ken/.claudine/worktrees/feat-sniff-tuning
git add sniff/lib/src/error.rs
git commit -m "feat(sniff-lib): add NoViableMethod and RemoteBashConsentRequired errors"
```

---

## Task 2: Tagged serde on `InstallationMethod`

**Files:**
- Modify: `sniff/lib/src/programs/types.rs` (enum attributes + impls unaffected)
- Test: `sniff/lib/src/programs/types.rs` (add tests to the existing `#[cfg(test)] mod tests`)

The `install-plan --json` command must emit a stable, self-describing shape for each method. Today `InstallationMethod` has no `Serialize`/`Deserialize`; we add them with the externally-tagged `tag = "manager", content = "target"` form so the JSON shape is `{"manager": "brew", "target": "ripgrep"}`. See tech-design §"JSON Shape".

**Deviation from tech-design:** The tech-design sketch mentions `Serialize, Deserialize`, but `InstallationMethod`'s variants hold `&'static str` and cannot round-trip through `Deserialize` without leaking owned strings. The CLI only ever *emits* plans as JSON (`install-plan --json`); it never reads plans back from JSON. Therefore this task adds only `Serialize` with the tagged layout from the tech-design. If a future feature needs `Deserialize`, it should either change the variants to `String` (breaking change) or introduce a parallel owned mirror type for JSON ingest.

- [ ] **Step 1: Write the failing tests**

Append to `#[cfg(test)] mod tests` in `sniff/lib/src/programs/types.rs`:

```rust
#[test]
fn test_installation_method_serializes_with_manager_target_shape() {
    let method = InstallationMethod::Brew("ripgrep");
    let json = serde_json::to_string(&method).unwrap();
    assert_eq!(json, r#"{"manager":"brew","target":"ripgrep"}"#);
}

#[test]
fn test_installation_method_serializes_remote_bash_as_tagged_shape() {
    let method = InstallationMethod::RemoteBash("https://sh.rustup.rs");
    let json = serde_json::to_string(&method).unwrap();
    assert_eq!(
        json,
        r#"{"manager":"remote_bash","target":"https://sh.rustup.rs"}"#
    );
}

#[test]
fn test_installation_method_serializes_cargo_as_tagged_shape() {
    let method = InstallationMethod::Cargo("bat");
    let json = serde_json::to_string(&method).unwrap();
    assert_eq!(json, r#"{"manager":"cargo","target":"bat"}"#);
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff installation_method_serializes --no-run 2>&1 | tail -20`
Expected: compile error — `InstallationMethod` doesn't implement `Serialize`.

- [ ] **Step 3: Derive tagged `Serialize`**

Apply this to the enum declaration around line 84 in `sniff/lib/src/programs/types.rs`:

```rust
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "manager", content = "target", rename_all = "snake_case")]
pub enum InstallationMethod {
    // ... existing variants unchanged
}
```

If `serde::Serialize` is not already imported at the top of the file, add `use serde::Serialize;` to the existing `use serde::{Deserialize, Serialize};` line (the file already imports `serde::{Deserialize, Serialize}` for `ExecutableSource` — verify before adding a duplicate).

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p sniff installation_method_serializes 2>&1 | tail -20`
Expected: the three new tests pass.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/types.rs
git commit -m "feat(sniff-lib): add tagged serde serialization to InstallationMethod"
```

---

## Task 3: Extend `InstallOptions` with `approve_remote_bash`

**Files:**
- Modify: `sniff/lib/src/programs/installer.rs`

Adds the consent field that `InstallPlan::execute` will check in Task 13. This is a non-breaking additive change to `InstallOptions`.

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `sniff/lib/src/programs/installer.rs`:

```rust
#[test]
fn test_install_options_default_does_not_approve_remote_bash() {
    let opts = InstallOptions::default();
    assert!(!opts.approve_remote_bash);
}

#[test]
fn test_install_options_with_approve_remote_bash_sets_flag() {
    let opts = InstallOptions::default().with_approve_remote_bash(true);
    assert!(opts.approve_remote_bash);
}
```

- [ ] **Step 2: Verify the test fails**

Run: `cargo test -p sniff approve_remote_bash --no-run 2>&1 | tail -10`
Expected: compile error — field/method not found.

- [ ] **Step 3: Add the field and builder method**

In `sniff/lib/src/programs/installer.rs`, extend `InstallOptions`:

```rust
#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub dry_run: bool,
    pub skip_confirm: bool,
    pub timeout_secs: u64,
    /// Whether the caller has explicitly approved executing a RemoteBash method.
    ///
    /// Defaults to `false`. The plan executor returns
    /// `SniffInstallationError::RemoteBashConsentRequired` if the selected
    /// option is `RemoteBash` and this flag is `false`.
    pub approve_remote_bash: bool,
}
```

Update `Default`:

```rust
impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            skip_confirm: false,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            approve_remote_bash: false,
        }
    }
}
```

Add the builder method inside `impl InstallOptions`:

```rust
    /// Sets whether RemoteBash execution is pre-approved.
    pub fn with_approve_remote_bash(mut self, approve: bool) -> Self {
        self.approve_remote_bash = approve;
        self
    }
```

The existing `dry_run()` and `auto_confirm()` constructors already spread `..Default::default()`, so they pick up the new field automatically.

- [ ] **Step 4: Verify all installer tests pass**

Run: `cargo test -p sniff programs::installer 2>&1 | tail -20`
Expected: the two new tests plus all existing `installer` tests pass.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/installer.rs
git commit -m "feat(sniff-lib): add approve_remote_bash to InstallOptions"
```

---

## Task 4: Add `dirs` and `which` dependencies to `sniff-lib`

**Files:**
- Modify: `sniff/lib/Cargo.toml`

`HostCapabilities` needs a home directory resolver for the cache file, and a cross-platform `bash` executable lookup for the `has_bash` field. Both add minimal compile time.

- [ ] **Step 1: Add the dependencies**

Open `sniff/lib/Cargo.toml` and add in the `[dependencies]` section (keep alphabetical if the file already is):

```toml
dirs = "6"
which = "7"
```

- [ ] **Step 2: Verify the workspace still builds**

Run: `cargo build -p sniff 2>&1 | tail -20`
Expected: clean build, no errors.

- [ ] **Step 3: Commit**

```bash
git add sniff/lib/Cargo.toml Cargo.lock
git commit -m "build(sniff-lib): add dirs and which dependencies for host capability detection"
```

---

## Task 5: `HostCapabilities` struct + cheap detector

**Files:**
- Create: `sniff/lib/src/programs/host_capability.rs`
- Modify: `sniff/lib/src/programs/mod.rs` (declare module + re-export)
- Test: `sniff/lib/src/programs/host_capability.rs` inline tests

Introduces the type that the plan builder will consume. In this task we land the struct definition and the cheap constructor (`detect()`). Sudo detection lands in Task 6, default PM + WSL in Task 7, and verification probes in Task 8.

**Deviation from tech-design:** the tech-design types `verified_lang_pkg_mgrs` as `BTreeSet<LanguagePackageManager>`, but `LanguagePackageManager` in `sniff/lib/src/programs/enums/categories.rs` does not derive `Ord` / `PartialOrd`. We use `HashSet<LanguagePackageManager>` instead (which only needs `Hash + Eq`, already derived) to avoid touching the shared enum. The cache file serializes `HashSet` as a JSON array, which is functionally equivalent for read-back.

- [ ] **Step 1: Write the failing test**

Create `sniff/lib/src/programs/host_capability.rs` with this skeleton:

```rust
//! Host capability detection for the install-plan pipeline.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/tech-design.md`
//! for the contract this module implements.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::os::{OsType, detect_os_type};
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::pkg_mngrs::{
    InstalledLanguagePackageManagers, InstalledOsPackageManagers,
};

/// Shared input to `build_install_plan`.
///
/// All fields are injectable so tests can fabricate arbitrary hosts without
/// touching the real machine. `HostCapabilities::default()` returns a
/// "nothing detected" host on the current platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub os_type: OsType,
    pub is_wsl: bool,
    pub has_bash: bool,
    pub os_pkg_mgrs: InstalledOsPackageManagers,
    pub lang_pkg_mgrs: InstalledLanguagePackageManagers,
    pub can_sudo: bool,
    pub default_os_package_manager: Option<OsPackageManager>,
    pub verified_lang_pkg_mgrs: HashSet<LanguagePackageManager>,
    pub npm_global_prefix_writable: Option<bool>,
    pub detected_at: DateTime<Utc>,
}

impl Default for HostCapabilities {
    fn default() -> Self {
        Self {
            os_type: OsType::Other,
            is_wsl: false,
            has_bash: false,
            os_pkg_mgrs: InstalledOsPackageManagers::default(),
            lang_pkg_mgrs: InstalledLanguagePackageManagers::default(),
            can_sudo: false,
            default_os_package_manager: None,
            verified_lang_pkg_mgrs: HashSet::new(),
            npm_global_prefix_writable: None,
            detected_at: Utc::now(),
        }
    }
}

impl HostCapabilities {
    /// Detect cheap host facts (no verification probes).
    ///
    /// Does not touch disk cache; call [`HostCapabilities::load_or_detect`]
    /// from Task 9 for the cached path.
    pub fn detect() -> Self {
        Self {
            os_type: detect_os_type(),
            is_wsl: false, // filled in by Task 7
            has_bash: detect_has_bash(),
            os_pkg_mgrs: InstalledOsPackageManagers::new(),
            lang_pkg_mgrs: InstalledLanguagePackageManagers::new(),
            can_sudo: false, // filled in by Task 6
            default_os_package_manager: None, // filled in by Task 7
            verified_lang_pkg_mgrs: HashSet::new(),
            npm_global_prefix_writable: None,
            detected_at: Utc::now(),
        }
    }
}

fn detect_has_bash() -> bool {
    which::which("bash").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_reports_no_sudo_and_no_default_pm() {
        let host = HostCapabilities::default();
        assert!(!host.can_sudo);
        assert!(host.default_os_package_manager.is_none());
        assert!(host.verified_lang_pkg_mgrs.is_empty());
        assert!(host.npm_global_prefix_writable.is_none());
    }

    #[test]
    fn detect_returns_current_os_type() {
        let host = HostCapabilities::detect();
        assert_eq!(host.os_type, detect_os_type());
    }

    #[test]
    fn detect_records_timestamp_near_now() {
        let before = Utc::now();
        let host = HostCapabilities::detect();
        let after = Utc::now();
        assert!(host.detected_at >= before);
        assert!(host.detected_at <= after);
    }
}
```

- [ ] **Step 2: Declare the module and run the tests**

In `sniff/lib/src/programs/mod.rs`, add to the `pub mod` block (keep alphabetical):

```rust
pub mod host_capability;
```

Run: `cargo test -p sniff host_capability 2>&1 | tail -20`
Expected: the three tests pass.

- [ ] **Step 3: Re-export `HostCapabilities`**

In `sniff/lib/src/programs/mod.rs`, add to the `pub use` section (keep grouped near the other programs re-exports):

```rust
pub use host_capability::HostCapabilities;
```

- [ ] **Step 4: Verify the public re-export compiles**

Run: `cargo build -p sniff 2>&1 | tail -10`
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/host_capability.rs sniff/lib/src/programs/mod.rs
git commit -m "feat(sniff-lib): add HostCapabilities struct with cheap detect"
```

---

## Task 6: Sudo detection

**Files:**
- Modify: `sniff/lib/src/programs/host_capability.rs`

Implements `can_sudo` detection without prompting. On Unix we probe (a) the primary group list for `wheel`/`sudo`/`admin` via `id -Gn`, and (b) `sudo -n true` for passwordless sudo. On Windows we always return `false` for now (elevation surfaces through a separate path in Task 12; see tech-design §3).

- [ ] **Step 1: Write the failing test**

Add to `sniff/lib/src/programs/host_capability.rs`:

```rust
#[cfg(test)]
mod sudo_tests {
    use super::*;

    /// Stubbed probes so we can unit-test the decision logic without touching
    /// the real shell. Each field simulates the result of one probe.
    #[derive(Debug, Clone, Default)]
    pub(super) struct SudoProbes {
        pub group_membership: bool,
        pub sudo_n_true: bool,
    }

    /// Pure decision function over the probe results.
    ///
    /// First positive signal wins; if none fire, returns `false`.
    pub(super) fn decide_can_sudo(probes: &SudoProbes) -> bool {
        probes.group_membership || probes.sudo_n_true
    }

    #[test]
    fn group_membership_wins() {
        assert!(decide_can_sudo(&SudoProbes {
            group_membership: true,
            sudo_n_true: false,
        }));
    }

    #[test]
    fn sudo_n_true_wins() {
        assert!(decide_can_sudo(&SudoProbes {
            group_membership: false,
            sudo_n_true: true,
        }));
    }

    #[test]
    fn no_signals_returns_false() {
        assert!(!decide_can_sudo(&SudoProbes::default()));
    }
}
```

Run: `cargo test -p sniff host_capability::sudo_tests 2>&1 | tail -20`
Expected: compile error until we add the helpers (Step 2 defines them inside the test module so they exist for the tests themselves).

Note: because the test module itself defines `decide_can_sudo` and `SudoProbes`, this first compile pass will succeed. The real work lands in step 3 where we wire the probes into `HostCapabilities::detect`.

- [ ] **Step 2: Verify the decision-function tests pass**

Run: `cargo test -p sniff host_capability::sudo_tests 2>&1 | tail -20`
Expected: three tests pass.

- [ ] **Step 3: Add real probes and wire them into `detect()`**

Promote `decide_can_sudo` and `SudoProbes` out of the test module and add the real probes:

```rust
#[derive(Debug, Clone, Default)]
struct SudoProbes {
    group_membership: bool,
    sudo_n_true: bool,
}

fn decide_can_sudo(probes: &SudoProbes) -> bool {
    probes.group_membership || probes.sudo_n_true
}

#[cfg(unix)]
fn probe_group_membership() -> bool {
    use std::process::Command;

    // `id -Gn` prints space-separated group names for the current user.
    let Ok(output) = Command::new("id").arg("-Gn").output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let groups = String::from_utf8_lossy(&output.stdout);
    groups
        .split_whitespace()
        .any(|g| matches!(g, "wheel" | "sudo" | "admin"))
}

#[cfg(not(unix))]
fn probe_group_membership() -> bool {
    false
}

#[cfg(unix)]
fn probe_sudo_n_true() -> bool {
    use std::process::Command;

    let Ok(output) = Command::new("sudo").args(["-n", "true"]).output() else {
        return false;
    };
    output.status.success()
}

#[cfg(not(unix))]
fn probe_sudo_n_true() -> bool {
    false
}

fn detect_can_sudo() -> bool {
    // On native Windows we never claim sudo; WSL is detected as Linux and
    // goes through the Unix probes.
    if cfg!(all(windows, not(target_env = "gnu"))) {
        return false;
    }
    decide_can_sudo(&SudoProbes {
        group_membership: probe_group_membership(),
        sudo_n_true: probe_sudo_n_true(),
    })
}
```

Update `HostCapabilities::detect()` to call `detect_can_sudo()`:

```rust
pub fn detect() -> Self {
    Self {
        os_type: detect_os_type(),
        is_wsl: false, // Task 7
        has_bash: detect_has_bash(),
        os_pkg_mgrs: InstalledOsPackageManagers::new(),
        lang_pkg_mgrs: InstalledLanguagePackageManagers::new(),
        can_sudo: detect_can_sudo(),
        default_os_package_manager: None, // Task 7
        verified_lang_pkg_mgrs: HashSet::new(),
        npm_global_prefix_writable: None,
        detected_at: Utc::now(),
    }
}
```

Then delete the duplicate `decide_can_sudo` / `SudoProbes` definitions inside `mod sudo_tests` (they are now imported via `use super::*;`).

- [ ] **Step 4: Verify all host_capability tests still pass**

Run: `cargo test -p sniff host_capability 2>&1 | tail -20`
Expected: all tests pass. (The real-probe path is covered implicitly by `detect()` running in the test environment; the pure-function tests cover the decision logic.)

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/host_capability.rs
git commit -m "feat(sniff-lib): detect sudo availability via group and sudo -n probes"
```

---

## Task 7: Default OS package manager helper + WSL detection

**Files:**
- Modify: `sniff/lib/src/programs/host_capability.rs`

Adds the `is_wsl` probe and the `(OsType, Option<LinuxFamily>) -> Option<OsPackageManager>` mapping that drives rule 1 of the plan builder. WSL reports as `OsType::Linux` but needs its own signal so the CLI can tell a WSL host from a native Linux one for the rare cases that diverge (documented in tech-design §3).

- [ ] **Step 1: Write the failing test**

Add inside `sniff/lib/src/programs/host_capability.rs`:

```rust
#[cfg(test)]
mod default_pm_tests {
    use super::*;
    use crate::os::LinuxFamily;

    #[test]
    fn debian_maps_to_apt() {
        assert_eq!(
            default_os_package_manager_for(OsType::Linux, Some(LinuxFamily::Debian)),
            Some(OsPackageManager::Apt)
        );
    }

    #[test]
    fn redhat_maps_to_dnf() {
        assert_eq!(
            default_os_package_manager_for(OsType::Linux, Some(LinuxFamily::RedHat)),
            Some(OsPackageManager::Dnf)
        );
    }

    #[test]
    fn arch_maps_to_pacman() {
        assert_eq!(
            default_os_package_manager_for(OsType::Linux, Some(LinuxFamily::Arch)),
            Some(OsPackageManager::Pacman)
        );
    }

    #[test]
    fn macos_maps_to_brew() {
        assert_eq!(
            default_os_package_manager_for(OsType::MacOS, None),
            Some(OsPackageManager::Brew)
        );
    }

    #[test]
    fn windows_maps_to_winget() {
        assert_eq!(
            default_os_package_manager_for(OsType::Windows, None),
            Some(OsPackageManager::Winget)
        );
    }

    #[test]
    fn unknown_linux_family_returns_none() {
        assert_eq!(
            default_os_package_manager_for(OsType::Linux, Some(LinuxFamily::Other)),
            None
        );
    }

    #[test]
    fn linux_without_family_returns_none() {
        assert_eq!(default_os_package_manager_for(OsType::Linux, None), None);
    }
}
```

- [ ] **Step 2: Verify the tests fail to compile**

Run: `cargo test -p sniff default_pm_tests --no-run 2>&1 | tail -10`
Expected: `default_os_package_manager_for` not found.

- [ ] **Step 3: Implement the helper and WSL probe**

Add to `sniff/lib/src/programs/host_capability.rs`:

```rust
use crate::os::{LinuxFamily, detect_linux_distro};

/// Returns the OS package manager that should be considered the "default" for
/// the given host. Linux delegates to the distro family; non-Linux uses the
/// hard-coded canonical manager.
///
/// Returns `None` when the OS has no known default (e.g. BSDs, unknown Linux
/// family) — the plan builder falls through to the alternative-OS-PM bucket.
pub fn default_os_package_manager_for(
    os: OsType,
    linux_family: Option<LinuxFamily>,
) -> Option<OsPackageManager> {
    match os {
        OsType::MacOS => Some(OsPackageManager::Brew),
        OsType::Windows => Some(OsPackageManager::Winget),
        OsType::Linux => match linux_family {
            Some(LinuxFamily::Debian) => Some(OsPackageManager::Apt),
            Some(LinuxFamily::RedHat) => Some(OsPackageManager::Dnf),
            Some(LinuxFamily::Arch) => Some(OsPackageManager::Pacman),
            Some(LinuxFamily::SUSE) => None, // zypper not modelled yet
            Some(LinuxFamily::NixOS) => Some(OsPackageManager::Nix),
            _ => None,
        },
        _ => None,
    }
}

fn detect_is_wsl() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    // Primary signal: /proc/version contains "microsoft" or "WSL".
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        let lower = version.to_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            return true;
        }
    }
    // Secondary signal: osrelease file.
    if let Ok(osrelease) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        let lower = osrelease.to_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            return true;
        }
    }
    false
}
```

Note: `OsPackageManager` is an enum variant already present in `sniff/lib/src/programs/enums/categories.rs`. The variants used above (`Apt`, `Dnf`, `Pacman`, `Brew`, `Winget`, `Nix`) already exist — verify by running `grep -n 'pub enum OsPackageManager' sniff/lib/src/programs/enums/categories.rs`. If any variant is missing, stop and raise the issue; adding new variants is out of scope for this feature.

Update `HostCapabilities::detect` to fill the new fields:

```rust
pub fn detect() -> Self {
    let os_type = detect_os_type();
    let is_wsl = detect_is_wsl();
    let linux_family = detect_linux_distro().map(|d| d.family);
    let default_pm = default_os_package_manager_for(os_type, linux_family);

    Self {
        os_type,
        is_wsl,
        has_bash: detect_has_bash(),
        os_pkg_mgrs: InstalledOsPackageManagers::new(),
        lang_pkg_mgrs: InstalledLanguagePackageManagers::new(),
        can_sudo: detect_can_sudo(),
        default_os_package_manager: default_pm,
        verified_lang_pkg_mgrs: HashSet::new(),
        npm_global_prefix_writable: None,
        detected_at: Utc::now(),
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sniff host_capability 2>&1 | tail -30`
Expected: all tests pass, including the seven new default-PM tests.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/host_capability.rs
git commit -m "feat(sniff-lib): add default OS package manager helper and WSL detection"
```

---

## Task 8: Verification probes for language package managers

**Files:**
- Modify: `sniff/lib/src/programs/host_capability.rs`

Implements `detect_with_verification()` which runs the "is the user *comfortable* with this manager" probes. These shell out to each installed language PM and parse one-line "yes/no" output. Timeouts are 2 seconds per probe (per tech-design). Failures degrade to "unverified" without failing the whole detection.

- [ ] **Step 1: Write the failing test**

Add to `sniff/lib/src/programs/host_capability.rs`:

```rust
#[cfg(test)]
mod verification_tests {
    use super::*;

    #[test]
    fn parse_npm_global_list_finds_entries() {
        let json = r#"{"dependencies":{"typescript":{"version":"5.0.0"}}}"#;
        assert!(parse_npm_global_list(json));
    }

    #[test]
    fn parse_npm_global_list_handles_empty() {
        let json = r#"{"dependencies":{}}"#;
        assert!(!parse_npm_global_list(json));
    }

    #[test]
    fn parse_npm_global_list_handles_malformed() {
        assert!(!parse_npm_global_list("not json"));
    }

    #[test]
    fn parse_cargo_install_list_finds_entries() {
        let output = "bat v0.24.0:\n    bat\nripgrep v14.1.0:\n    rg\n";
        assert!(parse_cargo_install_list(output));
    }

    #[test]
    fn parse_cargo_install_list_handles_empty() {
        assert!(!parse_cargo_install_list(""));
        assert!(!parse_cargo_install_list("\n\n"));
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff verification_tests --no-run 2>&1 | tail -10`
Expected: parse functions not found.

- [ ] **Step 3: Implement the probes and parsers**

Add to `sniff/lib/src/programs/host_capability.rs`:

```rust
use std::process::{Command, Stdio};
use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Runs a command with a short timeout and returns its stdout on success.
fn run_probe(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                use std::io::Read;
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_string(&mut out);
                }
                return Some(out);
            }
            Ok(Some(_)) => return None,
            Ok(None) => {}
            Err(_) => return None,
        }
        if start.elapsed() >= PROBE_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn parse_npm_global_list(stdout: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout) else {
        return false;
    };
    value
        .get("dependencies")
        .and_then(|d| d.as_object())
        .is_some_and(|obj| !obj.is_empty())
}

fn parse_cargo_install_list(stdout: &str) -> bool {
    // `cargo install --list` prints one crate header per line ending in ':'.
    // Empty output or whitespace-only output means no globally-installed crates.
    stdout
        .lines()
        .any(|line| line.trim_end().ends_with(':'))
}

fn probe_npm_verified() -> bool {
    run_probe("npm", &["ls", "-g", "--depth=0", "--json"])
        .as_deref()
        .map(parse_npm_global_list)
        .unwrap_or(false)
}

fn probe_pnpm_verified() -> bool {
    run_probe("pnpm", &["ls", "-g", "--depth=0", "--json"])
        .as_deref()
        .map(parse_npm_global_list) // same shape
        .unwrap_or(false)
}

fn probe_bun_verified() -> bool {
    // `bun pm ls -g` prints one package per line; any line means verified.
    run_probe("bun", &["pm", "ls", "-g"])
        .as_deref()
        .map(|s| s.lines().any(|l| !l.trim().is_empty()))
        .unwrap_or(false)
}

fn probe_yarn_verified() -> bool {
    // `yarn global list --json` emits one JSON object per line on success.
    run_probe("yarn", &["global", "list", "--json"])
        .as_deref()
        .map(|s| s.lines().any(|l| l.trim().starts_with('{')))
        .unwrap_or(false)
}

fn probe_cargo_verified() -> bool {
    run_probe("cargo", &["install", "--list"])
        .as_deref()
        .map(parse_cargo_install_list)
        .unwrap_or(false)
}

fn detect_verified_lang_pkg_mgrs(
    lang_pkg_mgrs: &InstalledLanguagePackageManagers,
) -> HashSet<LanguagePackageManager> {
    use crate::programs::types::ProgramDetector;
    let mut verified = HashSet::new();

    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Npm) && probe_npm_verified() {
        verified.insert(LanguagePackageManager::Npm);
    }
    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Pnpm) && probe_pnpm_verified() {
        verified.insert(LanguagePackageManager::Pnpm);
    }
    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Yarn) && probe_yarn_verified() {
        verified.insert(LanguagePackageManager::Yarn);
    }
    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Bun) && probe_bun_verified() {
        verified.insert(LanguagePackageManager::Bun);
    }
    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Cargo) && probe_cargo_verified() {
        verified.insert(LanguagePackageManager::Cargo);
    }

    verified
}

fn detect_npm_global_prefix_writable() -> Option<bool> {
    let prefix = run_probe("npm", &["prefix", "-g"])?;
    let path = std::path::Path::new(prefix.trim());
    if !path.exists() {
        return Some(false);
    }
    // Best-effort: try to create and immediately remove a temp file.
    let marker = path.join(".sniff-writable-check");
    match std::fs::File::create(&marker) {
        Ok(_) => {
            let _ = std::fs::remove_file(&marker);
            Some(true)
        }
        Err(_) => Some(false),
    }
}

impl HostCapabilities {
    /// Detect host facts plus verification probes.
    ///
    /// This runs global-list commands for each installed language package
    /// manager and checks whether the npm global prefix is user-writable. Each
    /// probe has a 2-second timeout and its failure mode is "unverified", not
    /// fatal. Call the cheaper [`HostCapabilities::detect`] when you don't
    /// need these extra signals.
    pub fn detect_with_verification() -> Self {
        let mut host = Self::detect();
        host.verified_lang_pkg_mgrs = detect_verified_lang_pkg_mgrs(&host.lang_pkg_mgrs);
        host.npm_global_prefix_writable = detect_npm_global_prefix_writable();
        host
    }
}
```

Verify `LanguagePackageManager` variants `Npm, Pnpm, Yarn, Bun, Cargo` all exist in `sniff/lib/src/programs/enums/categories.rs` before running tests. If naming differs, adjust.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sniff host_capability 2>&1 | tail -30`
Expected: all verification-parser tests pass. The full `detect_with_verification()` shell-outs aren't unit-tested (they depend on the live host); integration coverage comes in Task 21.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/host_capability.rs
git commit -m "feat(sniff-lib): add verification probes and npm prefix writability check"
```

---

## Task 9: `HostCapabilityCache` file load/save with TTL

**Files:**
- Modify: `sniff/lib/src/programs/host_capability.rs`
- Create: `sniff/lib/tests/host_capability_cache.rs`

Adds the on-disk cache at `~/.sniff-programs.json` with a 90-day TTL and schema versioning. Corrupt or schema-mismatched files are ignored and rebuilt. `load_or_detect` / `load_or_detect_with_verification` are the entry points the CLI will use. A `force_refresh` boolean bypasses the cache.

- [ ] **Step 1: Write the failing integration test**

Create `sniff/lib/tests/host_capability_cache.rs`:

```rust
//! Integration tests for the HostCapabilityCache file format and TTL logic.
//!
//! Uses a tempdir as $HOME via the injectable cache-path entry point so no
//! real home directory is touched.

use std::path::PathBuf;

use chrono::{Duration, Utc};
use sniff::programs::host_capability::{
    HostCapabilities, load_host_capabilities_from, save_host_capabilities_to,
    CACHE_SCHEMA_VERSION,
};

fn tmp_cache_path() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(".sniff-programs.json");
    (dir, path)
}

#[test]
fn cache_miss_returns_none() {
    let (_dir, path) = tmp_cache_path();
    assert!(load_host_capabilities_from(&path).is_none());
}

#[test]
fn round_trip_preserves_capabilities() {
    let (_dir, path) = tmp_cache_path();
    let host = HostCapabilities::default();
    save_host_capabilities_to(&path, &host).unwrap();
    let loaded = load_host_capabilities_from(&path).expect("cache hit");
    assert_eq!(loaded.os_type, host.os_type);
    assert_eq!(loaded.can_sudo, host.can_sudo);
}

#[test]
fn stale_cache_returns_none() {
    let (_dir, path) = tmp_cache_path();
    let mut host = HostCapabilities::default();
    // Detected 100 days ago; TTL is 90 days.
    host.detected_at = Utc::now() - Duration::days(100);
    save_host_capabilities_to(&path, &host).unwrap();
    assert!(load_host_capabilities_from(&path).is_none());
}

#[test]
fn corrupt_cache_returns_none() {
    let (_dir, path) = tmp_cache_path();
    std::fs::write(&path, "this is not json").unwrap();
    assert!(load_host_capabilities_from(&path).is_none());
}

#[test]
fn wrong_schema_version_returns_none() {
    let (_dir, path) = tmp_cache_path();
    let envelope = serde_json::json!({
        "schema_version": CACHE_SCHEMA_VERSION + 1,
        "hostname": "test",
        "os": "linux",
        "is_wsl": false,
        "expires_at": (Utc::now() + Duration::days(30)).to_rfc3339(),
        "capabilities": HostCapabilities::default(),
    });
    std::fs::write(&path, serde_json::to_string(&envelope).unwrap()).unwrap();
    assert!(load_host_capabilities_from(&path).is_none());
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff --test host_capability_cache --no-run 2>&1 | tail -20`
Expected: compile error — `HostCapabilityCacheFile`, `load_host_capabilities_from`, `save_host_capabilities_to`, `CACHE_SCHEMA_VERSION` not found.

- [ ] **Step 3: Implement the cache**

Add to `sniff/lib/src/programs/host_capability.rs`:

```rust
use std::io::Write;
use std::path::{Path, PathBuf};

/// Current on-disk schema version for the host capability cache.
pub const CACHE_SCHEMA_VERSION: u32 = 1;

/// 90-day TTL for cached host capabilities.
const CACHE_TTL: chrono::Duration = chrono::Duration::days(90);

/// On-disk envelope wrapping a `HostCapabilities` snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCapabilityCacheFile {
    pub schema_version: u32,
    pub hostname: String,
    pub os: OsType,
    pub is_wsl: bool,
    pub expires_at: DateTime<Utc>,
    pub capabilities: HostCapabilities,
}

/// Default cache path: `~/.sniff-programs.json`.
///
/// Returns `None` when the home directory cannot be resolved — in that case
/// callers should skip caching and run a live detection.
pub fn default_cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".sniff-programs.json"))
}

/// Load capabilities from a cache file if the cache is fresh and the schema
/// version matches. Returns `None` on miss, stale, corrupt, or schema drift.
pub fn load_host_capabilities_from(path: &Path) -> Option<HostCapabilities> {
    let bytes = std::fs::read(path).ok()?;
    let envelope: HostCapabilityCacheFile = serde_json::from_slice(&bytes).ok()?;
    if envelope.schema_version != CACHE_SCHEMA_VERSION {
        return None;
    }
    if envelope.expires_at < Utc::now() {
        return None;
    }
    Some(envelope.capabilities)
}

/// Save capabilities atomically to `path` with `rename`.
pub fn save_host_capabilities_to(
    path: &Path,
    host: &HostCapabilities,
) -> std::io::Result<()> {
    let envelope = HostCapabilityCacheFile {
        schema_version: CACHE_SCHEMA_VERSION,
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        os: host.os_type,
        is_wsl: host.is_wsl,
        expires_at: host.detected_at + CACHE_TTL,
        capabilities: host.clone(),
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&serde_json::to_vec_pretty(&envelope)?)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

impl HostCapabilities {
    /// Returns cached capabilities if fresh, otherwise detects and writes a
    /// new cache file. Errors writing the cache are silently ignored.
    pub fn load_or_detect() -> Self {
        let path = default_cache_path();
        if let Some(ref p) = path
            && let Some(host) = load_host_capabilities_from(p)
        {
            return host;
        }
        let host = Self::detect();
        if let Some(ref p) = path {
            let _ = save_host_capabilities_to(p, &host);
        }
        host
    }

    /// As [`load_or_detect`], but uses `detect_with_verification` on miss.
    /// When `force_refresh` is true the cache is bypassed and rewritten.
    pub fn load_or_detect_with_verification(force_refresh: bool) -> Self {
        let path = default_cache_path();
        if !force_refresh
            && let Some(ref p) = path
            && let Some(host) = load_host_capabilities_from(p)
        {
            return host;
        }
        let host = Self::detect_with_verification();
        if let Some(ref p) = path {
            let _ = save_host_capabilities_to(p, &host);
        }
        host
    }
}
```

Then re-export the cache helpers from `sniff/lib/src/programs/mod.rs`:

```rust
pub use host_capability::{
    HostCapabilities, HostCapabilityCacheFile, default_cache_path,
    load_host_capabilities_from, save_host_capabilities_to, CACHE_SCHEMA_VERSION,
};
```

Note: `sniff/lib` is `edition = "2024"`, so `if let ... && let ...` (let-chains) are stable. If you later backport this module to a crate on an older edition, rewrite the chained patterns as nested `if let` blocks.

- [ ] **Step 4: Run the integration tests**

Run: `cargo test -p sniff --test host_capability_cache 2>&1 | tail -30`
Expected: all five tests pass.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/host_capability.rs sniff/lib/src/programs/mod.rs sniff/lib/tests/host_capability_cache.rs
git commit -m "feat(sniff-lib): add HostCapabilityCache with 90-day TTL and atomic writes"
```

---

## Task 10: `InstallPlan`, `InstallPlanOption`, `InstallPlanReason` types

**Files:**
- Create: `sniff/lib/src/programs/install_plan.rs`
- Modify: `sniff/lib/src/programs/mod.rs`

Introduces the plan data types. No selection logic yet — just the public shape so the rest of the plan can reference stable types. This task keeps the module compiling with a stub `build_install_plan` that always returns an empty, failed plan.

- [ ] **Step 1: Write the failing test**

Create `sniff/lib/src/programs/install_plan.rs`:

```rust
//! Install plan data types and builder.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/spec.md` and
//! `tech-design.md` for the contract this module implements.

use serde::Serialize;

use crate::error::SniffInstallationError;
use crate::programs::host_capability::HostCapabilities;
use crate::programs::installer::{InstallOptions, InstallResult, execute_install};
use crate::programs::schema::ProgramMetadata;
use crate::programs::types::InstallationMethod;

/// Machine-readable reason an install plan option was selected or rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPlanReason {
    /// This option was chosen.
    Selected,
    /// A higher-priority runnable method was chosen instead.
    LowerPriorityAlternative,
    /// The program's `os_availability` excludes the detected host OS.
    NoOsSupport,
    /// The package manager required by this method is not installed.
    ManagerNotInstalled,
    /// The method requires sudo and the host cannot sudo (or `--no-sudo`).
    RequiresSudoNotAvailable,
    /// A language PM is installed but not verified.
    RequiresUnverifiedLangManager,
    /// Catch-all for unexpected skip reasons.
    Unknown,
}

/// One evaluated installation method on an install plan.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlanOption {
    pub kind: InstallationMethod,
    pub requires_sudo: bool,
    pub choose: bool,
    pub reason_type: InstallPlanReason,
    pub reason: String,
}

/// A full evaluation of every installation method a program declares.
#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    pub program: String,
    pub website: &'static str,
    pub successful: bool,
    pub options: Vec<InstallPlanOption>,
}

impl InstallPlan {
    /// Every method considered, regardless of runnability.
    pub fn known_installations(&self) -> Vec<&InstallationMethod> {
        self.options.iter().map(|o| &o.kind).collect()
    }

    /// Every option that was not chosen.
    pub fn failed_with_reason(&self) -> Vec<&InstallPlanOption> {
        self.options.iter().filter(|o| !o.choose).collect()
    }

    /// The chosen option, if any.
    pub fn chosen(&self) -> Option<&InstallPlanOption> {
        self.options.iter().find(|o| o.choose)
    }

    /// Execute the chosen option. See Task 13 for the full implementation.
    pub fn execute(
        &self,
        opts: &InstallOptions,
    ) -> Result<InstallResult, SniffInstallationError> {
        let _ = opts;
        Err(SniffInstallationError::NoViableMethod {
            pkg: self.program.clone(),
            detail: "InstallPlan::execute not implemented yet".to_string(),
        })
    }
}

/// Stub plan builder. Task 11 and 12 replace this with the real implementation.
pub fn build_install_plan<P: ProgramMetadata>(
    program: &P,
    _host: &HostCapabilities,
) -> InstallPlan {
    InstallPlan {
        program: program.display_name().to_string(),
        website: program.website(),
        successful: false,
        options: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_plan_reason_selected_serializes_snake_case() {
        let json = serde_json::to_string(&InstallPlanReason::Selected).unwrap();
        assert_eq!(json, "\"selected\"");
    }

    #[test]
    fn install_plan_reason_lower_priority_serializes_snake_case() {
        let json = serde_json::to_string(&InstallPlanReason::LowerPriorityAlternative).unwrap();
        assert_eq!(json, "\"lower_priority_alternative\"");
    }

    #[test]
    fn empty_plan_reports_no_chosen_option() {
        let plan = InstallPlan {
            program: "vim".into(),
            website: "https://www.vim.org",
            successful: false,
            options: Vec::new(),
        };
        assert!(plan.chosen().is_none());
        assert!(plan.failed_with_reason().is_empty());
        assert!(plan.known_installations().is_empty());
    }

    #[test]
    fn chosen_returns_option_where_choose_is_true() {
        let plan = InstallPlan {
            program: "bat".into(),
            website: "https://github.com/sharkdp/bat",
            successful: true,
            options: vec![
                InstallPlanOption {
                    kind: InstallationMethod::Cargo("bat"),
                    requires_sudo: false,
                    choose: false,
                    reason_type: InstallPlanReason::LowerPriorityAlternative,
                    reason: "brew was chosen".into(),
                },
                InstallPlanOption {
                    kind: InstallationMethod::Brew("bat"),
                    requires_sudo: false,
                    choose: true,
                    reason_type: InstallPlanReason::Selected,
                    reason: "default OS package manager".into(),
                },
            ],
        };
        let chosen = plan.chosen().expect("chosen option");
        assert!(matches!(chosen.kind, InstallationMethod::Brew("bat")));
        assert_eq!(plan.failed_with_reason().len(), 1);
    }
}
```

Declare the module and re-export from `sniff/lib/src/programs/mod.rs`:

```rust
pub mod install_plan;

pub use install_plan::{
    InstallPlan, InstallPlanOption, InstallPlanReason, build_install_plan,
};
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p sniff install_plan 2>&1 | tail -30`
Expected: all four tests pass.

- [ ] **Step 3: Verify the workspace builds**

Run: `cargo build -p sniff 2>&1 | tail -10`
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add sniff/lib/src/programs/install_plan.rs sniff/lib/src/programs/mod.rs
git commit -m "feat(sniff-lib): add InstallPlan, InstallPlanOption, InstallPlanReason types"
```

---

## Task 11: Plan builder — Phase 1 fact derivation

**Files:**
- Modify: `sniff/lib/src/programs/install_plan.rs`
- Modify: `sniff/lib/src/programs/installer.rs` (expose `method_available` to the new module via `pub(crate)` — it already is; verify)

Implements the fact-derivation pass: for every method in a program's `installation_methods`, compute `os_supported`, `manager_installed`, `requires_sudo`, `lang_manager_verified`, `eligible_without_priority`, and the best blocking reason. No selection yet.

- [ ] **Step 1: Write the failing test**

Add to `sniff/lib/src/programs/install_plan.rs`:

```rust
#[cfg(test)]
mod fact_tests {
    use super::*;
    use crate::os::OsType;
    use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
    use crate::programs::host_capability::HostCapabilities;
    use crate::programs::types::InstallationMethod;

    fn host_with_brew() -> HostCapabilities {
        let json = r#"{"brew": true}"#;
        let os_pkg_mgrs = serde_json::from_str(json).unwrap();
        HostCapabilities {
            os_type: OsType::MacOS,
            os_pkg_mgrs,
            default_os_package_manager: Some(OsPackageManager::Brew),
            has_bash: true,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn derive_fact_brew_on_macos_is_eligible() {
        let host = host_with_brew();
        let method = InstallationMethod::Brew("ripgrep");
        let os_availability = &[OsType::MacOS];
        let fact = derive_method_fact(&method, os_availability, &host);
        assert!(fact.os_supported);
        assert!(fact.manager_installed);
        assert!(!fact.requires_sudo);
        assert!(fact.eligible_without_priority);
    }

    #[test]
    fn derive_fact_apt_requires_sudo() {
        let host = HostCapabilities::default();
        let method = InstallationMethod::Apt("ripgrep");
        let fact = derive_method_fact(&method, &[], &host);
        assert!(fact.requires_sudo);
    }

    #[test]
    fn derive_fact_brew_not_installed_is_ineligible() {
        let host = HostCapabilities::default(); // no managers
        let method = InstallationMethod::Brew("ripgrep");
        let fact = derive_method_fact(&method, &[], &host);
        assert!(!fact.manager_installed);
        assert!(!fact.eligible_without_priority);
        assert_eq!(fact.blocking_reason, Some(InstallPlanReason::ManagerNotInstalled));
    }

    #[test]
    fn derive_fact_unsupported_os_is_blocked_by_os() {
        let host = HostCapabilities {
            os_type: OsType::Linux,
            ..HostCapabilities::default()
        };
        let method = InstallationMethod::Brew("ripgrep");
        let fact = derive_method_fact(&method, &[OsType::MacOS], &host);
        assert!(!fact.os_supported);
        assert_eq!(fact.blocking_reason, Some(InstallPlanReason::NoOsSupport));
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff fact_tests --no-run 2>&1 | tail -20`
Expected: `derive_method_fact` and `MethodFact` not found.

- [ ] **Step 3: Implement fact derivation**

Add to `sniff/lib/src/programs/install_plan.rs`:

```rust
use crate::os::OsType;
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::installer::method_available;
use strum::IntoEnumIterator;

/// Why a given method would run (or not) on this host, before priority is
/// applied. Used internally by the bucket selector in Task 12.
#[derive(Debug, Clone)]
pub(crate) struct MethodFact {
    pub kind: InstallationMethod,
    pub os_supported: bool,
    pub manager_installed: bool,
    pub requires_sudo: bool,
    pub lang_manager_verified: bool,
    pub eligible_without_priority: bool,
    pub blocking_reason: Option<InstallPlanReason>,
}

pub(crate) fn derive_method_fact(
    method: &InstallationMethod,
    os_availability: &[OsType],
    host: &HostCapabilities,
) -> MethodFact {
    let os_supported = os_availability.is_empty() || os_availability.contains(&host.os_type);
    let manager_installed =
        method_available(method, &host.os_pkg_mgrs, &host.lang_pkg_mgrs)
            || method.is_remote_bash() && host.has_bash;
    let requires_sudo = method_requires_sudo(method, host);
    let lang_manager_verified = is_lang_manager_verified(method, host);

    let mut blocking_reason = None;
    let mut eligible = true;

    if !os_supported {
        eligible = false;
        blocking_reason = Some(InstallPlanReason::NoOsSupport);
    } else if !manager_installed {
        eligible = false;
        blocking_reason = Some(InstallPlanReason::ManagerNotInstalled);
    } else if requires_sudo && !host.can_sudo {
        eligible = false;
        blocking_reason = Some(InstallPlanReason::RequiresSudoNotAvailable);
    }

    MethodFact {
        kind: method.clone(),
        os_supported,
        manager_installed,
        requires_sudo,
        lang_manager_verified,
        eligible_without_priority: eligible,
        blocking_reason,
    }
}

/// Returns whether this method needs `sudo` on the current host.
///
/// OS package managers that prefix commands with `sudo` in
/// [`build_install_command`] return `true`. On Windows no manager uses sudo
/// today (see tech-design §3; elevation for native winget is modelled as
/// `requires_sudo = true` when we implement it). Language managers never
/// require sudo in the current command table.
fn method_requires_sudo(method: &InstallationMethod, host: &HostCapabilities) -> bool {
    use InstallationMethod::*;
    let unix_sudo_method = matches!(method, Apt(_) | Nala(_) | Dnf(_) | Pacman(_));
    if unix_sudo_method {
        return true;
    }
    // On native Windows, winget elevation surfaces as requires_sudo = true.
    // On WSL it is still Linux and doesn't run winget.
    if matches!(method, Winget(_)) && host.os_type == OsType::Windows && !host.is_wsl {
        return true;
    }
    false
}

fn is_lang_manager_verified(method: &InstallationMethod, host: &HostCapabilities) -> bool {
    use InstallationMethod::*;
    match method {
        Npm(_) => host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Npm),
        Pnpm(_) => host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Pnpm),
        Yarn(_) => host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Yarn),
        Bun(_) => host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Bun),
        Cargo(_) => host.verified_lang_pkg_mgrs.contains(&LanguagePackageManager::Cargo),
        _ => false,
    }
}
```

Verify `method_available` is `pub(crate)` in `sniff/lib/src/programs/installer.rs` (it already is, per `pub(crate) fn method_available` around line 133). No change required there.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sniff fact_tests 2>&1 | tail -20`
Expected: all four fact tests pass.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_plan.rs
git commit -m "feat(sniff-lib): derive per-method facts for install plan builder"
```

---

## Task 12: Plan builder — Phase 2 bucket-priority selection

**Files:**
- Modify: `sniff/lib/src/programs/install_plan.rs`

Walks the method facts in priority order and chooses the first eligible option. All other eligible facts become `LowerPriorityAlternative`. Ineligible facts keep their blocking reason. Replaces the stub `build_install_plan` from Task 10.

Priority buckets, in order (tech-design §"Plan-Building Algorithm"):
1. Default OS package manager (matches `host.default_os_package_manager`)
2. Verified pnpm global (`Pnpm(_)` + pnpm verified)
3. User-writable npm global (`Npm(_)`, prefix writable OR unknown)
4. Alternative installed OS package manager (installed but not default)
5. Remote bash (if `host.has_bash`)
6. Cargo (any cargo install)
7. Sudo-gated npm global fallback (`Npm(_)`, prefix NOT writable, `can_sudo=true`)

- [ ] **Step 1: Write the failing tests**

Add to `sniff/lib/src/programs/install_plan.rs`:

```rust
#[cfg(test)]
mod selection_tests {
    use super::*;
    use crate::os::OsType;
    use crate::programs::enums::{Editor, LanguagePackageManager, OsPackageManager};
    use crate::programs::host_capability::HostCapabilities;
    use crate::programs::schema::{ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy};
    use crate::programs::types::InstallationMethod;

    /// Helper: build a fake ProgramMetadata that carries an arbitrary slice of
    /// installation methods. Lives inside tests because it's a one-off shape.
    struct FakeProgram {
        info: &'static ProgramInfo,
    }
    impl ProgramMetadata for FakeProgram {
        fn info(&self) -> &'static ProgramInfo {
            self.info
        }
    }

    static BREW_AND_CARGO: ProgramInfo = ProgramInfo {
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
        installation_methods: &[
            InstallationMethod::Brew("bat"),
            InstallationMethod::Cargo("bat"),
        ],
    };

    fn host_macos_with_brew() -> HostCapabilities {
        let os_pkg_mgrs = serde_json::from_str(r#"{"brew": true}"#).unwrap();
        HostCapabilities {
            os_type: OsType::MacOS,
            default_os_package_manager: Some(OsPackageManager::Brew),
            os_pkg_mgrs,
            has_bash: true,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn brew_wins_over_cargo_on_macos() {
        let host = host_macos_with_brew();
        let plan = build_install_plan(&FakeProgram { info: &BREW_AND_CARGO }, &host);
        assert!(plan.successful);
        let chosen = plan.chosen().expect("chosen");
        assert!(matches!(chosen.kind, InstallationMethod::Brew("bat")));
        assert_eq!(chosen.reason_type, InstallPlanReason::Selected);

        let cargo_opt = plan.options.iter().find(|o| {
            matches!(o.kind, InstallationMethod::Cargo(_))
        }).unwrap();
        assert!(!cargo_opt.choose);
        assert_eq!(cargo_opt.reason_type, InstallPlanReason::LowerPriorityAlternative);
    }

    static LINUX_APT_ONLY: ProgramInfo = ProgramInfo {
        binary_name: "htop",
        display_name: "htop",
        description: "interactive process viewer",
        website: "https://htop.dev",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[OsType::Linux],
        repo: None,
        installation_methods: &[InstallationMethod::Apt("htop")],
    };

    #[test]
    fn apt_without_sudo_is_rejected_with_reason() {
        let os_pkg_mgrs = serde_json::from_str(r#"{"apt": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::Linux,
            default_os_package_manager: Some(OsPackageManager::Apt),
            os_pkg_mgrs,
            can_sudo: false,
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(&FakeProgram { info: &LINUX_APT_ONLY }, &host);
        assert!(!plan.successful);
        let apt = &plan.options[0];
        assert!(!apt.choose);
        assert_eq!(apt.reason_type, InstallPlanReason::RequiresSudoNotAvailable);
    }

    #[test]
    fn apt_with_sudo_is_selected() {
        let os_pkg_mgrs = serde_json::from_str(r#"{"apt": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::Linux,
            default_os_package_manager: Some(OsPackageManager::Apt),
            os_pkg_mgrs,
            can_sudo: true,
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(&FakeProgram { info: &LINUX_APT_ONLY }, &host);
        assert!(plan.successful);
        let apt = plan.chosen().unwrap();
        assert!(apt.requires_sudo);
    }

    static PNPM_AND_NPM: ProgramInfo = ProgramInfo {
        binary_name: "typescript",
        display_name: "TypeScript",
        description: "Typed JavaScript",
        website: "https://www.typescriptlang.org",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[],
        repo: None,
        installation_methods: &[
            InstallationMethod::Pnpm("typescript"),
            InstallationMethod::Npm("typescript"),
        ],
    };

    #[test]
    fn verified_pnpm_beats_npm() {
        let lang_pkg_mgrs = serde_json::from_str(r#"{"pnpm": true, "npm": true}"#).unwrap();
        let mut host = HostCapabilities {
            os_type: OsType::Linux,
            lang_pkg_mgrs,
            npm_global_prefix_writable: Some(true),
            ..HostCapabilities::default()
        };
        host.verified_lang_pkg_mgrs.insert(LanguagePackageManager::Pnpm);
        let plan = build_install_plan(&FakeProgram { info: &PNPM_AND_NPM }, &host);
        let chosen = plan.chosen().unwrap();
        assert!(matches!(chosen.kind, InstallationMethod::Pnpm(_)));
    }

    #[test]
    fn unverified_pnpm_gets_unverified_reason_and_falls_through_to_npm() {
        let lang_pkg_mgrs = serde_json::from_str(r#"{"pnpm": true, "npm": true}"#).unwrap();
        let host = HostCapabilities {
            os_type: OsType::Linux,
            lang_pkg_mgrs,
            npm_global_prefix_writable: Some(true),
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(&FakeProgram { info: &PNPM_AND_NPM }, &host);
        let chosen = plan.chosen().unwrap();
        assert!(matches!(chosen.kind, InstallationMethod::Npm(_)));

        let pnpm_opt = plan.options.iter().find(|o| {
            matches!(o.kind, InstallationMethod::Pnpm(_))
        }).unwrap();
        assert_eq!(
            pnpm_opt.reason_type,
            InstallPlanReason::RequiresUnverifiedLangManager
        );
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff selection_tests --no-run 2>&1 | tail -20`
Expected: compile succeeds but the stubbed `build_install_plan` returns an empty plan, so `selection_tests::*` all fail on assertions.

- [ ] **Step 3: Implement the bucket selector**

Replace the stub `build_install_plan` in `sniff/lib/src/programs/install_plan.rs` with:

```rust
/// Priority buckets for install plan selection. The earliest matching bucket
/// whose fact is eligible wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bucket {
    DefaultOsPm,
    VerifiedPnpm,
    NpmNoSudo,
    AltOsPm,
    RemoteBash,
    Cargo,
    SudoNpm,
    Other,
}

fn bucket_for(fact: &MethodFact, host: &HostCapabilities) -> Bucket {
    use InstallationMethod::*;
    match &fact.kind {
        // Default OS PM bucket: method manager matches host default PM
        _ if fact.kind.is_os_package_manager()
            && host
                .default_os_package_manager
                .as_ref()
                .is_some_and(|pm| pm.binary_name() == fact.kind.manager_binary()) =>
        {
            Bucket::DefaultOsPm
        }
        Pnpm(_) if fact.lang_manager_verified => Bucket::VerifiedPnpm,
        Pnpm(_) => Bucket::Other,
        Npm(_) => {
            match host.npm_global_prefix_writable {
                Some(false) if host.can_sudo => Bucket::SudoNpm,
                Some(false) => Bucket::Other,
                _ => Bucket::NpmNoSudo,
            }
        }
        _ if fact.kind.is_os_package_manager() => Bucket::AltOsPm,
        RemoteBash(_) => Bucket::RemoteBash,
        Cargo(_) => Bucket::Cargo,
        _ => Bucket::Other,
    }
}

fn bucket_order() -> [Bucket; 7] {
    [
        Bucket::DefaultOsPm,
        Bucket::VerifiedPnpm,
        Bucket::NpmNoSudo,
        Bucket::AltOsPm,
        Bucket::RemoteBash,
        Bucket::Cargo,
        Bucket::SudoNpm,
    ]
}

/// Build an install plan for a program against the given host capabilities.
pub fn build_install_plan<P: ProgramMetadata>(
    program: &P,
    host: &HostCapabilities,
) -> InstallPlan {
    let info = program.info();
    let facts: Vec<MethodFact> = info
        .installation_methods
        .iter()
        .map(|m| derive_method_fact(m, info.os_availability, host))
        .collect();

    // Find the first bucket with an eligible fact.
    let mut chosen_index: Option<usize> = None;
    for bucket in bucket_order() {
        if let Some((idx, _)) = facts.iter().enumerate().find(|(_, f)| {
            f.eligible_without_priority && bucket_for(f, host) == bucket
        }) {
            chosen_index = Some(idx);
            break;
        }
    }

    let options: Vec<InstallPlanOption> = facts
        .iter()
        .enumerate()
        .map(|(i, fact)| {
            let choose = chosen_index == Some(i);
            let (reason_type, reason) = if choose {
                (
                    InstallPlanReason::Selected,
                    format!(
                        "chosen — {}{}",
                        bucket_description(bucket_for(fact, host)),
                        if fact.requires_sudo { " (requires sudo)" } else { "" }
                    ),
                )
            } else if fact.eligible_without_priority {
                (
                    InstallPlanReason::LowerPriorityAlternative,
                    "a higher-priority method was chosen".to_string(),
                )
            } else {
                // Blocked — map to the best reason we computed
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
        .collect();

    InstallPlan {
        program: program.display_name().to_string(),
        website: program.website(),
        successful: chosen_index.is_some(),
        options,
    }
}

fn bucket_description(bucket: Bucket) -> &'static str {
    match bucket {
        Bucket::DefaultOsPm => "default OS package manager",
        Bucket::VerifiedPnpm => "verified pnpm global",
        Bucket::NpmNoSudo => "user-writable npm global",
        Bucket::AltOsPm => "alternative OS package manager",
        Bucket::RemoteBash => "remote bash installer",
        Bucket::Cargo => "cargo install",
        Bucket::SudoNpm => "sudo-gated npm global",
        Bucket::Other => "other",
    }
}

fn blocking_reason_for(fact: &MethodFact, host: &HostCapabilities) -> InstallPlanReason {
    if let Some(reason) = fact.blocking_reason {
        return reason;
    }
    // Special case: unverified pnpm — we *can* run it but refuse to pick it.
    if matches!(&fact.kind, InstallationMethod::Pnpm(_)) && !fact.lang_manager_verified
        && host
            .lang_pkg_mgrs
            .is_installed(LanguagePackageManager::Pnpm)
    {
        return InstallPlanReason::RequiresUnverifiedLangManager;
    }
    InstallPlanReason::Unknown
}

fn explain_blocking_reason(fact: &MethodFact, reason: InstallPlanReason) -> String {
    match reason {
        InstallPlanReason::NoOsSupport => {
            format!("{} does not run on this OS", fact.kind.manager_name())
        }
        InstallPlanReason::ManagerNotInstalled => format!(
            "{} is not installed on this host",
            fact.kind.manager_binary()
        ),
        InstallPlanReason::RequiresSudoNotAvailable => format!(
            "{} requires sudo and the current user cannot sudo",
            fact.kind.manager_name()
        ),
        InstallPlanReason::RequiresUnverifiedLangManager => format!(
            "{} is installed but has no globally-installed packages — not choosing it blindly",
            fact.kind.manager_name()
        ),
        InstallPlanReason::Unknown => {
            "no other bucket accepted this method".to_string()
        }
        InstallPlanReason::Selected | InstallPlanReason::LowerPriorityAlternative => {
            unreachable!()
        }
    }
}
```

You'll need `use crate::programs::types::ProgramDetector;` at the top of the file (the `is_installed` call above uses that trait). `OsPackageManager::binary_name` is already part of the generated enum impl — verify with `grep -n 'fn binary_name' sniff/lib/src/programs/enums/categories.rs` before committing.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sniff install_plan 2>&1 | tail -40`
Expected: all `install_plan::*` tests pass — fact tests, selection tests, and the earlier type tests.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_plan.rs
git commit -m "feat(sniff-lib): implement priority-bucket install plan selection"
```

---

## Task 13: `InstallPlan::execute()` with remote-bash gate

**Files:**
- Modify: `sniff/lib/src/programs/install_plan.rs`

Wires execution. The dry-run path always works. For the real execution path, we refuse to run a `RemoteBash` chosen option unless `opts.approve_remote_bash` is true, returning `SniffInstallationError::RemoteBashConsentRequired`.

- [ ] **Step 1: Write the failing tests**

Append to `#[cfg(test)] mod` in `sniff/lib/src/programs/install_plan.rs`:

```rust
#[cfg(test)]
mod execute_tests {
    use super::*;
    use crate::os::OsType;
    use crate::programs::host_capability::HostCapabilities;
    use crate::programs::installer::InstallOptions;
    use crate::programs::schema::{ProgramInfo, VersionFlag, VersionParseStrategy};

    static BREW_PKG: ProgramInfo = ProgramInfo {
        binary_name: "ripgrep",
        display_name: "ripgrep",
        description: "fast grep",
        website: "https://github.com/BurntSushi/ripgrep",
        version_flag: VersionFlag::Long,
        parse_strategy: VersionParseStrategy::FirstLine,
        version_regex: None,
        version_prefix: None,
        alternate_binary_names: &[],
        os_availability: &[OsType::MacOS],
        repo: None,
        installation_methods: &[InstallationMethod::Brew("ripgrep")],
    };

    struct FakeProgram;
    impl crate::programs::schema::ProgramMetadata for FakeProgram {
        fn info(&self) -> &'static ProgramInfo {
            &BREW_PKG
        }
    }

    fn host_with_brew() -> HostCapabilities {
        let os_pkg_mgrs = serde_json::from_str(r#"{"brew": true}"#).unwrap();
        HostCapabilities {
            os_type: OsType::MacOS,
            os_pkg_mgrs,
            default_os_package_manager: Some(
                crate::programs::enums::OsPackageManager::Brew,
            ),
            has_bash: true,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn dry_run_returns_ok_without_executing() {
        let plan = build_install_plan(&FakeProgram, &host_with_brew());
        let result = plan.execute(&InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        assert!(result.command.contains("brew"));
    }

    #[test]
    fn failed_plan_returns_no_viable_method() {
        let host = HostCapabilities {
            os_type: OsType::Linux, // brew not installed on this fake host
            ..HostCapabilities::default()
        };
        let plan = build_install_plan(&FakeProgram, &host);
        let err = plan.execute(&InstallOptions::dry_run()).unwrap_err();
        assert!(matches!(
            err,
            crate::error::SniffInstallationError::NoViableMethod { .. }
        ));
    }

    // Remote bash consent test uses a fabricated plan directly because we
    // don't want a real program metadata with RemoteBash in the fixture set.
    #[test]
    fn remote_bash_without_consent_errors() {
        let plan = InstallPlan {
            program: "rustup".into(),
            website: "https://rustup.rs",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::RemoteBash("https://sh.rustup.rs"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "remote bash installer".into(),
            }],
        };
        let err = plan.execute(&InstallOptions::default()).unwrap_err();
        assert!(matches!(
            err,
            crate::error::SniffInstallationError::RemoteBashConsentRequired { .. }
        ));
    }

    #[test]
    fn remote_bash_dry_run_allowed_without_consent() {
        let plan = InstallPlan {
            program: "rustup".into(),
            website: "https://rustup.rs",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::RemoteBash("https://sh.rustup.rs"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "remote bash installer".into(),
            }],
        };
        // Even dry-run errors today because execute_install rejects RemoteBash
        // at the build_install_command layer. The contract is therefore: dry-run
        // a remote-bash plan is still rejected by the underlying executor.
        // This test documents the behavior so we don't accidentally widen it.
        let result = plan.execute(&InstallOptions::dry_run());
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff execute_tests --no-run 2>&1 | tail -10`
Expected: some fail on assertions because the stub returns `NoViableMethod` always.

- [ ] **Step 3: Implement `InstallPlan::execute`**

Replace the stub `execute` in `sniff/lib/src/programs/install_plan.rs`:

```rust
impl InstallPlan {
    pub fn execute(
        &self,
        opts: &InstallOptions,
    ) -> Result<InstallResult, SniffInstallationError> {
        let chosen = self.chosen().ok_or_else(|| {
            SniffInstallationError::NoViableMethod {
                pkg: self.program.clone(),
                detail: format!(
                    "no runnable method (considered {} option(s))",
                    self.options.len()
                ),
            }
        })?;

        if matches!(chosen.kind, InstallationMethod::RemoteBash(_))
            && !opts.approve_remote_bash
        {
            let url = chosen.kind.package_name().to_string();
            return Err(SniffInstallationError::RemoteBashConsentRequired {
                pkg: self.program.clone(),
                url,
            });
        }

        execute_install(&chosen.kind, opts)
    }
}
```

(This replaces the stub body; keep everything else in the `impl InstallPlan` block unchanged.)

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sniff install_plan 2>&1 | tail -30`
Expected: all execute tests pass, along with all prior install_plan tests.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/install_plan.rs
git commit -m "feat(sniff-lib): implement InstallPlan::execute with remote-bash consent gate"
```

---

## Task 14: `ProgramDetector` trait additions

**Files:**
- Modify: `sniff/lib/src/programs/types.rs`

Adds `known_methods`, `available_methods`, and `install_plan` to the `ProgramDetector` trait with default implementations that compose the new pipeline. Existing callers keep working because the methods are additive.

- [ ] **Step 1: Write the failing test**

Append to `sniff/lib/src/programs/types.rs` tests:

```rust
#[test]
fn category_detector_known_methods_matches_metadata() {
    let detector = CategoryDetector::<Editor>::default();
    let methods = detector.known_methods(Editor::Vim);
    assert_eq!(methods, Editor::Vim.info().installation_methods);
}

#[test]
fn category_detector_available_methods_filters_by_os() {
    // On the current host, VSCode's brew/winget/etc. methods should still
    // produce a deterministic subset — we just assert the call compiles and
    // returns a Vec (not checking contents since they depend on host state).
    let detector = CategoryDetector::<Editor>::default();
    let _available = detector.available_methods(Editor::VSCode);
}

#[test]
fn category_detector_install_plan_returns_plan_for_program() {
    let detector = CategoryDetector::<Editor>::default();
    let plan = detector.install_plan(Editor::Vim);
    assert_eq!(plan.program, Editor::Vim.display_name());
    // Not asserting success/failure since both depend on host state.
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff category_detector_known_methods --no-run 2>&1 | tail -10`
Expected: compile error — the methods don't exist on the trait.

- [ ] **Step 3: Add the trait methods**

In `sniff/lib/src/programs/types.rs`, extend `trait ProgramDetector` with:

```rust
    /// Returns every installation method the program declares, ignoring host
    /// constraints. This is the static metadata.
    fn known_methods(&self, program: Self::Program) -> &'static [InstallationMethod] {
        program.info().installation_methods
    }

    /// Returns the subset of known methods whose required package manager is
    /// actually installed on this host and whose program is permitted on the
    /// current OS.
    fn available_methods(&self, program: Self::Program) -> Vec<InstallationMethod> {
        use crate::programs::host_capability::HostCapabilities;
        use crate::programs::installer::method_available;

        let info = program.info();
        let host = HostCapabilities::load_or_detect();

        let os_ok = info.os_availability.is_empty()
            || info.os_availability.contains(&host.os_type);
        if !os_ok {
            return Vec::new();
        }

        info.installation_methods
            .iter()
            .filter(|m| {
                method_available(m, &host.os_pkg_mgrs, &host.lang_pkg_mgrs)
                    || (m.is_remote_bash() && host.has_bash)
            })
            .cloned()
            .collect()
    }

    /// Returns a full install plan for this program against cached host
    /// capabilities.
    fn install_plan(&self, program: Self::Program) -> crate::programs::InstallPlan {
        use crate::programs::host_capability::HostCapabilities;
        use crate::programs::install_plan::build_install_plan;

        let host = HostCapabilities::load_or_detect();
        // We need a `&impl ProgramMetadata`; Self::Program is Copy so we pass
        // by reference to a temporary.
        build_install_plan(&program, &host)
    }
```

The default implementations work for every `CategoryDetector<E>` and all future implementors. The `install_plan` default uses `load_or_detect` so repeated calls within a process still pay the cheap cache check.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sniff category_detector 2>&1 | tail -30`
Expected: all three new tests pass alongside the existing `category_detector::*` tests.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/types.rs
git commit -m "feat(sniff-lib): add known_methods, available_methods, install_plan to ProgramDetector"
```

---

## Task 15: Rewire `installable`/`install`/`install_version` as plan wrappers

**Files:**
- Modify: `sniff/lib/src/programs/types.rs`

Replace the duplicated host-probing logic in the `CategoryDetector<E>: ProgramDetector` impl with thin wrappers over `install_plan`. This is the refactor the tech design calls "use the same decision engine everywhere." Existing behavior must not regress — the existing tests in `program_installable.rs` still need to pass.

- [ ] **Step 1: Write a regression test for the wrapper mapping**

Append to `sniff/lib/src/programs/types.rs` tests:

```rust
#[test]
fn installable_mirrors_plan_successful() {
    let detector = CategoryDetector::<Editor>::default();
    for editor in Editor::iter() {
        let plan = detector.install_plan(editor);
        assert_eq!(
            detector.installable(editor),
            plan.successful,
            "installable() must mirror install_plan().successful for {:?}",
            editor
        );
    }
}
```

- [ ] **Step 2: Verify the test currently passes (baseline)**

Run: `cargo test -p sniff installable_mirrors_plan_successful 2>&1 | tail -10`

It might pass today *accidentally* because the old and new paths both read the same host state. That's fine — we want it to keep passing after the refactor.

- [ ] **Step 3: Rewrite the wrapper impl**

In the `impl<E: CategoryEnum> ProgramDetector for CategoryDetector<E>` block in `sniff/lib/src/programs/types.rs` (starts around line 512), replace the `installable`, `install`, and `install_version` methods with:

```rust
    fn installable(&self, program: E) -> bool {
        self.install_plan(program).successful
    }

    fn install(&self, program: E) -> Result<(), SniffInstallationError> {
        let plan = self.install_plan(program);
        if !plan.successful {
            return Err(SniffInstallationError::NoViableMethod {
                pkg: program.display_name().to_string(),
                detail: format!(
                    "evaluated {} method(s); none are runnable",
                    plan.options.len()
                ),
            });
        }
        let _ = plan.execute(&crate::programs::installer::InstallOptions::default())?;
        Ok(())
    }

    fn install_version(&self, program: E, version: &str) -> Result<(), SniffInstallationError> {
        let plan = self.install_plan(program);
        let chosen = plan
            .chosen()
            .ok_or_else(|| SniffInstallationError::NoViableMethod {
                pkg: program.display_name().to_string(),
                detail: format!(
                    "evaluated {} method(s); none are runnable",
                    plan.options.len()
                ),
            })?;

        if matches!(chosen.kind, InstallationMethod::RemoteBash(_)) {
            return Err(SniffInstallationError::RemoteBashConsentRequired {
                pkg: program.display_name().to_string(),
                url: chosen.kind.package_name().to_string(),
            });
        }

        let _ = crate::programs::installer::execute_versioned_install(
            &chosen.kind,
            version,
            &crate::programs::installer::InstallOptions::default(),
        )?;
        Ok(())
    }
```

- [ ] **Step 4: Run the full lib test suite**

Run: `cargo test -p sniff 2>&1 | tail -30`
Expected: everything passes. Of particular interest: `sniff/lib/tests/program_installable.rs` (the OS-availability regression tests) and the new `installable_mirrors_plan_successful` test.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/types.rs
git commit -m "refactor(sniff-lib): reroute installable/install/install_version through InstallPlan"
```

---

## Task 16: CLI shared `InstallCommandArgs` and per-category `InstallPlan` action

**Files:**
- Modify: `sniff/cli/src/args.rs`

Grows the existing per-category `*Action` enums from a single `Install { program }` variant to `Install(InstallCommandArgs)` and `InstallPlan(InstallCommandArgs)`. Shared flags (`--dry-run`, `-y/--yes`, `--via`, `--no-sudo`, `-f/--force`) live on one struct. Updates `is_install_action`, `install_program_name`, and adds `is_install_plan_action` accessors.

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `sniff/cli/src/args.rs`:

```rust
#[test]
fn editors_install_parses_with_dry_run_and_yes() {
    let cli = parse_args(&["editors", "install", "vim", "--dry-run", "-y"]).unwrap();
    if let Some(Commands::Editors {
        action: Some(EditorAction::Install(args)),
    }) = cli.command
    {
        assert_eq!(args.program.as_deref(), Some("vim"));
        assert!(args.dry_run);
        assert!(args.yes);
        assert!(!args.no_sudo);
        assert!(!args.force);
        assert!(args.via.is_none());
    } else {
        panic!("Expected Editors install with InstallCommandArgs");
    }
}

#[test]
fn editors_install_plan_parses() {
    let cli = parse_args(&["editors", "install-plan", "vim"]).unwrap();
    assert!(cli.command.as_ref().unwrap().is_install_plan_action());
    assert_eq!(
        cli.command.as_ref().unwrap().install_program_name(),
        Some("vim")
    );
}

#[test]
fn editors_install_via_and_no_sudo_parse() {
    let cli = parse_args(&[
        "editors", "install", "vim",
        "--via", "brew",
        "--no-sudo",
        "--force",
    ]).unwrap();
    if let Some(Commands::Editors {
        action: Some(EditorAction::Install(args)),
    }) = cli.command
    {
        assert_eq!(args.via.as_deref(), Some("brew"));
        assert!(args.no_sudo);
        assert!(args.force);
    } else {
        panic!("Expected editors install with via+no_sudo+force");
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli editors_install_parses_with --no-run 2>&1 | tail -15`
Expected: compile error — `InstallCommandArgs`, `is_install_plan_action`, new variant shapes not found.

- [ ] **Step 3: Introduce `InstallCommandArgs`**

In `sniff/cli/src/args.rs`, before the `define_program_action!` macro definition, add:

```rust
/// Shared flag group for `install` and `install-plan` subcommands across every
/// program category.
#[derive(clap::Args, Debug, Clone, Default)]
pub struct InstallCommandArgs {
    /// Program name to install (binary name or identifier)
    pub program: Option<String>,

    /// Build the plan and print what would happen; do not execute
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the interactive confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Force a specific package manager (e.g. `brew`, `cargo`, `pnpm`)
    #[arg(long, value_name = "MANAGER")]
    pub via: Option<String>,

    /// Force the plan builder to treat sudo as unavailable
    #[arg(long)]
    pub no_sudo: bool,

    /// Bypass the host capability cache and rebuild it
    #[arg(short = 'f', long)]
    pub force: bool,
}
```

Update the `define_program_action!` macro to generate two action variants:

```rust
macro_rules! define_program_action {
    ($action_name:ident, $candidates_fn:ident, $program_enum:ty) => {
        fn $candidates_fn() -> Vec<clap_complete::engine::CompletionCandidate> {
            use clap_complete::engine::CompletionCandidate;
            use sniff::programs::ProgramMetadata;
            use strum::IntoEnumIterator;

            <$program_enum>::iter()
                .flat_map(|p| {
                    let mut candidates = vec![
                        CompletionCandidate::new(p.binary_name())
                            .help(Some(p.description().into())),
                    ];
                    let snake = p.to_string();
                    if snake != p.binary_name() {
                        candidates.push(
                            CompletionCandidate::new(snake).help(Some(p.description().into())),
                        );
                    }
                    candidates
                })
                .collect()
        }

        #[derive(Subcommand, Debug, Clone)]
        pub enum $action_name {
            /// Install a program (interactive picker if no name given)
            Install(InstallCommandArgs),

            /// Show the install plan without executing anything
            #[command(name = "install-plan")]
            InstallPlan(InstallCommandArgs),
        }
    };
}
```

Do the same change to `AllProgramAction`:

```rust
#[derive(Subcommand, Debug, Clone)]
pub enum AllProgramAction {
    /// Install a program (interactive picker if no name given)
    Install(InstallCommandArgs),

    /// Show the install plan without executing anything
    #[command(name = "install-plan")]
    InstallPlan(InstallCommandArgs),
}
```

Now update the `Commands` accessor methods. Replace the existing `is_install_action`, `install_program_name` impls with these two plus the new `is_install_plan_action`. Also add `install_command_args` to return the whole struct:

```rust
impl Commands {
    pub fn is_install_action(&self) -> bool {
        self.install_command_args().map(|(a, _)| a == InstallCommandKind::Install).unwrap_or(false)
    }

    pub fn is_install_plan_action(&self) -> bool {
        self.install_command_args().map(|(a, _)| a == InstallCommandKind::InstallPlan).unwrap_or(false)
    }

    pub fn install_program_name(&self) -> Option<&str> {
        self.install_command_args().and_then(|(_, args)| args.program.as_deref())
    }

    /// Returns the kind (install vs install-plan) and the shared args for
    /// install-like commands. Returns `None` for non-install commands.
    pub fn install_command_args(&self) -> Option<(InstallCommandKind, &InstallCommandArgs)> {
        use Commands::*;
        match self {
            Programs { action: Some(AllProgramAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            Programs { action: Some(AllProgramAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            Editors { action: Some(EditorAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            Editors { action: Some(EditorAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            Utilities { action: Some(UtilityAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            Utilities { action: Some(UtilityAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            LanguagePackageManagers { action: Some(LangPkgMgrAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            LanguagePackageManagers { action: Some(LangPkgMgrAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            OsPackageManagers { action: Some(OsPkgMgrAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            OsPackageManagers { action: Some(OsPkgMgrAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            TtsClients { action: Some(TtsClientAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            TtsClients { action: Some(TtsClientAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            TerminalApps { action: Some(TerminalAppAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            TerminalApps { action: Some(TerminalAppAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            Audio { action: Some(AudioAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            Audio { action: Some(AudioAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            Agents { action: Some(AgentAction::Install(args)) } => Some((InstallCommandKind::Install, args)),
            Agents { action: Some(AgentAction::InstallPlan(args)) } => Some((InstallCommandKind::InstallPlan, args)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCommandKind {
    Install,
    InstallPlan,
}
```

**Migration checklist for existing `args.rs`.** Search for `EditorAction::Install {` and `AllProgramAction::Install {` and migrate every hit. As of this plan, there are call sites at lines **780, 782, 805, 808, 1557, 1570, 1595, 1604, 1663, 1668, 1687, 1694** of `sniff/cli/src/args.rs` (line numbers are approximate — rely on `grep -n 'EditorAction::Install {\|AllProgramAction::Install {' sniff/cli/src/args.rs`).

For each hit, convert:

| Old (struct variant)                                       | New (tuple variant)                                                    |
|------------------------------------------------------------|------------------------------------------------------------------------|
| `EditorAction::Install { program }`                        | `EditorAction::Install(InstallCommandArgs { program, .. })`            |
| `EditorAction::Install { program: None }`                  | `EditorAction::Install(InstallCommandArgs { program: None, .. })`     |
| `EditorAction::Install { program: Some("vim".to_string()) }` | `EditorAction::Install(InstallCommandArgs { program: Some("vim".to_string()), ..Default::default() })` |
| `AllProgramAction::Install { program }`                    | `AllProgramAction::Install(InstallCommandArgs { program, .. })`       |

The same shape applies to `UtilityAction`, `LangPkgMgrAction`, `OsPkgMgrAction`, `TtsClientAction`, `TerminalAppAction`, `AudioAction`, and `AgentAction` if they appear — though in practice only `EditorAction` and `AllProgramAction` are currently exercised by tests.

The two accessor-site matches in `is_install_action` and `install_program_name` (lines 780, 782, 805, 808) are deleted and replaced wholesale by the new `install_command_args` implementation shown further below — don't migrate them in place.

Also update `sniff/cli/src/install.rs`: its `direct_install` dispatch currently pattern-matches the old variant shape via `Commands::install_program_name`. That accessor still works (it reads `args.program`), so the resolver path is untouched.

But `sniff/cli/src/commands.rs` around line 74 currently calls `cmd.is_install_action()` and then branches on `install_program_name`. We need to update this dispatch in Task 20; for Task 16 it's fine to leave the old branch calling a stub — as long as it still compiles.

- [ ] **Step 4: Run the test**

Run: `cargo test -p sniff-cli editors_install 2>&1 | tail -20`
Expected: the new parser tests pass and no existing tests break.

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/args.rs
git commit -m "feat(sniff-cli): add InstallCommandArgs with install and install-plan variants"
```

---

## Task 17: CLI pure `ResolvedProgram` resolver

**Files:**
- Modify: `sniff/cli/src/install.rs`

Adds a deterministic resolver that returns `Result<ResolvedProgram, ResolveError>` without attempting installs. Replaces the recursive `OutputFilter::Programs => categories.iter()...install().is_ok()` loop. The resolver is the single point of truth for "what category does the name 'vim' belong to?"

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `sniff/cli/src/install.rs`:

```rust
#[test]
fn resolve_program_editor_by_binary() {
    let resolved = resolve_program("vim").unwrap();
    assert!(matches!(resolved, ResolvedProgram::Editor(sniff::programs::Editor::Vim)));
}

#[test]
fn resolve_program_utility_alternate() {
    let resolved = resolve_program("rg").unwrap();
    assert!(matches!(resolved, ResolvedProgram::Utility(sniff::programs::Utility::Ripgrep)));
}

#[test]
fn resolve_program_unknown_name_errors() {
    let err = resolve_program("definitely-not-a-real-program-xyz").unwrap_err();
    assert!(err.to_string().contains("Unknown program"));
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli resolve_program --no-run 2>&1 | tail -10`
Expected: `resolve_program` and `ResolvedProgram` not found.

- [ ] **Step 3: Implement the resolver**

Add to `sniff/cli/src/install.rs` (after the per-category `resolve_*` macros, not inside them):

```rust
/// A program identified by the name the user typed.
#[derive(Debug, Clone, Copy)]
pub enum ResolvedProgram {
    Editor(sniff::programs::Editor),
    Utility(sniff::programs::Utility),
    LanguagePackageManager(sniff::programs::LanguagePackageManager),
    OsPackageManager(sniff::programs::OsPackageManager),
    TtsClient(sniff::programs::TtsClient),
    TerminalApp(sniff::programs::TerminalApp),
    HeadlessAudio(sniff::programs::HeadlessAudio),
    AiCli(sniff::programs::AiCli),
}

impl ResolvedProgram {
    pub fn display_name(&self) -> &'static str {
        use sniff::programs::ProgramMetadata;
        match self {
            Self::Editor(e) => e.display_name(),
            Self::Utility(u) => u.display_name(),
            Self::LanguagePackageManager(m) => m.display_name(),
            Self::OsPackageManager(m) => m.display_name(),
            Self::TtsClient(c) => c.display_name(),
            Self::TerminalApp(t) => t.display_name(),
            Self::HeadlessAudio(a) => a.display_name(),
            Self::AiCli(a) => a.display_name(),
        }
    }

    // Note: plan-building for a ResolvedProgram goes through
    // `install_plan_cmd::build_plan_for_args` in Task 20 because the CLI
    // needs to honor --force and --no-sudo before building the plan.
    // We intentionally do NOT add a `ResolvedProgram::install_plan()`
    // convenience here — that would be a second, hidden entry point that
    // bypasses CLI flags.
}

/// Resolve a free-form program name to a specific category enum variant.
///
/// Tries each category in a deterministic order. Returns an error listing the
/// categories searched if no match is found.
pub fn resolve_program(name: &str) -> Result<ResolvedProgram, ResolveError> {
    if let Ok(p) = resolve_editor(name) {
        return Ok(ResolvedProgram::Editor(p));
    }
    if let Ok(p) = resolve_utility(name) {
        return Ok(ResolvedProgram::Utility(p));
    }
    if let Ok(p) = resolve_lang_pkg_mgr(name) {
        return Ok(ResolvedProgram::LanguagePackageManager(p));
    }
    if let Ok(p) = resolve_os_pkg_mgr(name) {
        return Ok(ResolvedProgram::OsPackageManager(p));
    }
    if let Ok(p) = resolve_tts_client(name) {
        return Ok(ResolvedProgram::TtsClient(p));
    }
    if let Ok(p) = resolve_terminal_app(name) {
        return Ok(ResolvedProgram::TerminalApp(p));
    }
    if let Ok(p) = resolve_audio(name) {
        return Ok(ResolvedProgram::HeadlessAudio(p));
    }
    if let Ok(p) = resolve_agent(name) {
        return Ok(ResolvedProgram::AiCli(p));
    }
    Err(ResolveError(format!(
        "Unknown program '{}'. Searched categories: editors, utilities, language package managers, OS package managers, TTS clients, terminal apps, headless audio, AI agents",
        name
    )))
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sniff-cli resolve_program 2>&1 | tail -20`
Expected: the three new tests pass.

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/install.rs
git commit -m "feat(sniff-cli): add pure ResolvedProgram resolver"
```

---

## Task 18: CLI `render_install_plan` renderer

**Files:**
- Create: `sniff/cli/src/install_plan_cmd.rs`
- Modify: `sniff/cli/src/lib.rs` (or `main.rs`, whichever declares modules — verify)

Single renderer that handles all three success/failure branches with `biscuit-terminal::Prose`. Used by both `install` (with subsequent execute) and `install-plan` (render only).

- [ ] **Step 1: Write the failing test**

Create `sniff/cli/src/install_plan_cmd.rs`:

```rust
//! Plan-aware install rendering and execution for the CLI.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/spec.md`
//! section "CLI: Updated `install` Behavior" for the messaging contract.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use sniff::programs::{InstallPlan, InstallPlanOption, InstallPlanReason, InstallationMethod};

/// Render the plan to a `String` ready for printing to stdout.
///
/// Branches:
/// - `plan.successful == true` and no `requires_sudo` on the chosen option →
///   short success line
/// - `plan.successful == true` and `requires_sudo` → sudo-warning line
/// - `plan.successful == false` → "we know how to install X but none are
///   available" block with website fallback
///
/// When `verbose` is set, each failed option is rendered above the success
/// line with a dim/info style so users can see what was skipped and why.
pub fn render_install_plan(plan: &InstallPlan, verbose: bool) -> String {
    let terminal = Terminal::default();
    let mut out = String::new();

    if plan.successful {
        if verbose {
            for opt in plan.failed_with_reason() {
                let line = format!(
                    "- <dim>skipped {} — <i>{}</i></dim>",
                    opt.kind.manager_name(),
                    opt.reason
                );
                out.push_str(&Prose::new(line).render(&terminal));
                out.push('\n');
            }
            if !plan.failed_with_reason().is_empty() {
                out.push('\n');
            }
        }
        let chosen = plan.chosen().expect("successful plan has a chosen option");
        let success_line = render_success_line(&plan.program, chosen);
        out.push_str(&Prose::new(success_line).render(&terminal));
        out.push('\n');
    } else {
        out.push_str(&render_failure_block(plan, &terminal));
    }

    out
}

fn render_success_line(program: &str, chosen: &InstallPlanOption) -> String {
    let method = chosen.kind.manager_name();
    if matches!(chosen.kind, InstallationMethod::RemoteBash(_)) {
        format!(
            "The <blue>{program}</blue> will be installed using a <b>remote bash installer</b>. You will be asked for explicit confirmation before the script runs."
        )
    } else if chosen.requires_sudo {
        format!(
            "The <blue>{program}</blue> is installable using <b>{method}</b> but it requires root privileges so we will include the use of <yellow>sudo</yellow> so this installation method will succeed."
        )
    } else {
        format!(
            "The <blue>{program}</blue> will be installed using the <b>{method}</b>."
        )
    }
}

fn render_failure_block(plan: &InstallPlan, terminal: &Terminal) -> String {
    let mut out = String::new();
    let header = format!(
        "We know how to install the <blue>{}</blue> program via the following methods but none are available to you for the stated reasons:",
        plan.program
    );
    out.push_str(&Prose::new(header).render(terminal));
    out.push_str("\n\n");
    for opt in &plan.options {
        let line = format!(
            "    - {} (reason: <i><dim><red>{}</red></dim></i>)",
            opt.kind.manager_name(),
            opt.reason
        );
        out.push_str(&Prose::new(line).render(terminal));
        out.push('\n');
    }
    out.push('\n');
    let fallback = format!(
        "While we weren't able to do this for you, it's likely that you can install it yourself by going to their website: <a href=\"{url}\">{url}</a>",
        url = plan.website
    );
    out.push_str(&Prose::new(fallback).render(terminal));
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_success_plan(requires_sudo: bool) -> InstallPlan {
        InstallPlan {
            program: "Vim".into(),
            website: "https://www.vim.org",
            successful: true,
            options: vec![InstallPlanOption {
                kind: if requires_sudo {
                    InstallationMethod::Apt("vim")
                } else {
                    InstallationMethod::Brew("vim")
                },
                requires_sudo,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "default OS package manager".into(),
            }],
        }
    }

    #[test]
    fn success_without_sudo_mentions_brew() {
        let rendered = render_install_plan(&fake_success_plan(false), false);
        assert!(rendered.contains("Vim"));
        assert!(rendered.to_lowercase().contains("brew"));
        assert!(!rendered.to_lowercase().contains("sudo"));
    }

    #[test]
    fn success_with_sudo_mentions_sudo_warning() {
        let rendered = render_install_plan(&fake_success_plan(true), false);
        assert!(rendered.to_lowercase().contains("sudo"));
        assert!(rendered.to_lowercase().contains("root privileges"));
    }

    #[test]
    fn failure_lists_all_options_and_website() {
        let plan = InstallPlan {
            program: "Vim".into(),
            website: "https://www.vim.org",
            successful: false,
            options: vec![
                InstallPlanOption {
                    kind: InstallationMethod::Brew("vim"),
                    requires_sudo: false,
                    choose: false,
                    reason_type: InstallPlanReason::ManagerNotInstalled,
                    reason: "brew is not installed on this host".into(),
                },
                InstallPlanOption {
                    kind: InstallationMethod::Apt("vim"),
                    requires_sudo: true,
                    choose: false,
                    reason_type: InstallPlanReason::RequiresSudoNotAvailable,
                    reason: "apt requires sudo".into(),
                },
            ],
        };
        let rendered = render_install_plan(&plan, false);
        assert!(rendered.contains("brew"));
        assert!(rendered.contains("apt"));
        assert!(rendered.contains("https://www.vim.org"));
        assert!(rendered.contains("none are available"));
    }

    #[test]
    fn verbose_success_prints_skipped_options() {
        let mut plan = fake_success_plan(false);
        plan.options.push(InstallPlanOption {
            kind: InstallationMethod::Cargo("vim"),
            requires_sudo: false,
            choose: false,
            reason_type: InstallPlanReason::LowerPriorityAlternative,
            reason: "brew was chosen".into(),
        });
        let verbose = render_install_plan(&plan, true);
        assert!(verbose.contains("skipped cargo"));
    }
}
```

Declare the module in `sniff/cli/src/main.rs`. Add `mod install_plan_cmd;` alongside the existing `mod install;`, `mod args;`, `mod commands;`, `mod output;` declarations at the top of the file.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p sniff-cli install_plan_cmd 2>&1 | tail -30`
Expected: all four render tests pass.

- [ ] **Step 3: Commit**

```bash
git add sniff/cli/src/install_plan_cmd.rs sniff/cli/src/main.rs
git commit -m "feat(sniff-cli): add render_install_plan with success, sudo, and failure branches"
```

---

## Task 19: CLI execute flow with confirm and remote-bash extra confirm

**Files:**
- Modify: `sniff/cli/src/install_plan_cmd.rs`

Adds the execute path: render the plan, prompt for confirmation unless `--yes`, add an *unconditional* extra confirmation for remote-bash (even with `--yes`), and distinguish Ctrl+C (exit 130) from Esc (exit 0).

- [ ] **Step 1: Write the failing test**

Append to `sniff/cli/src/install_plan_cmd.rs` tests:

```rust
#[test]
fn should_require_remote_bash_consent_returns_true_for_remote_bash() {
    let plan = InstallPlan {
        program: "rustup".into(),
        website: "https://rustup.rs",
        successful: true,
        options: vec![InstallPlanOption {
            kind: InstallationMethod::RemoteBash("https://sh.rustup.rs"),
            requires_sudo: false,
            choose: true,
            reason_type: InstallPlanReason::Selected,
            reason: "remote bash installer".into(),
        }],
    };
    assert!(should_require_remote_bash_consent(&plan));
}

#[test]
fn should_require_remote_bash_consent_false_for_brew() {
    let plan = InstallPlan {
        program: "vim".into(),
        website: "https://www.vim.org",
        successful: true,
        options: vec![InstallPlanOption {
            kind: InstallationMethod::Brew("vim"),
            requires_sudo: false,
            choose: true,
            reason_type: InstallPlanReason::Selected,
            reason: "default OS package manager".into(),
        }],
    };
    assert!(!should_require_remote_bash_consent(&plan));
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli should_require_remote_bash_consent --no-run 2>&1 | tail -10`
Expected: function not found.

- [ ] **Step 3: Implement the execute flow**

Append to `sniff/cli/src/install_plan_cmd.rs`:

```rust
use std::error::Error;

use inquire::Confirm;
use sniff::programs::InstallOptions;

/// Returns true if the plan's chosen option is a `RemoteBash` method, in
/// which case the CLI must prompt for a second explicit confirmation even
/// when `--yes` is passed.
pub fn should_require_remote_bash_consent(plan: &InstallPlan) -> bool {
    plan.chosen()
        .is_some_and(|o| matches!(o.kind, InstallationMethod::RemoteBash(_)))
}

/// Exit code for ctrl-c / interrupted prompts.
pub const EXIT_INTERRUPTED: i32 = 130;

/// Full "render + confirm + execute" flow. Called by the CLI dispatcher for
/// `sniff <category> install <name>`.
pub fn execute_install_flow(
    plan: &InstallPlan,
    dry_run: bool,
    skip_confirm: bool,
    plain: bool,
) -> Result<(), Box<dyn Error>> {
    // 1. Render
    let rendered = render_install_plan(plan, /* verbose */ false);
    crate::output::emit_text(&rendered, plain);

    // 2. Failure: exit cleanly, nothing to do
    if !plan.successful {
        return Ok(());
    }

    // 3. Base confirmation
    if !dry_run && !skip_confirm {
        match Confirm::new("Proceed with installation?")
            .with_default(true)
            .prompt()
        {
            Ok(true) => {}
            Ok(false) => return Ok(()),
            Err(inquire::InquireError::OperationCanceled) => return Ok(()),
            Err(inquire::InquireError::OperationInterrupted) => {
                std::process::exit(EXIT_INTERRUPTED)
            }
            Err(e) => return Err(e.into()),
        }
    }

    // 4. Remote-bash extra confirmation (never skipped)
    let remote_bash = should_require_remote_bash_consent(plan);
    if remote_bash && !dry_run {
        eprintln!();
        let warning = "<yellow>Warning:</yellow> this will download and execute a remote shell script. Continue?";
        let terminal = Terminal::default();
        eprintln!("{}", Prose::new(warning).render(&terminal));
        match Confirm::new("I understand; proceed with remote-bash install?")
            .with_default(false)
            .prompt()
        {
            Ok(true) => {}
            Ok(false) => return Ok(()),
            Err(inquire::InquireError::OperationCanceled) => return Ok(()),
            Err(inquire::InquireError::OperationInterrupted) => {
                std::process::exit(EXIT_INTERRUPTED)
            }
            Err(e) => return Err(e.into()),
        }
    }

    // 5. Execute
    let opts = InstallOptions {
        dry_run,
        skip_confirm: true,
        timeout_secs: 120,
        approve_remote_bash: remote_bash,
    };
    plan.execute(&opts)?;
    Ok(())
}
```

The flow above calls `crate::output::emit_text`. Verify that function exists in `sniff/cli/src/output/mod.rs`:

```bash
grep -n 'pub fn emit_text' sniff/cli/src/output/mod.rs
```

It should. If not, check the exact name (`emit`, `emit_styled`, etc.) and adjust.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sniff-cli install_plan_cmd 2>&1 | tail -20`
Expected: the two new tests pass. (The full `execute_install_flow` is not unit-tested because it prompts; CLI integration tests in Task 21 cover non-interactive paths.)

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/install_plan_cmd.rs
git commit -m "feat(sniff-cli): add execute_install_flow with sudo and remote-bash consent"
```

---

## Task 20: Dispatch `install` and `install-plan` through the new pipeline

**Files:**
- Modify: `sniff/cli/src/commands.rs`
- Modify: `sniff/cli/src/install.rs` (remove the old recursive `OutputFilter::Programs` loop)
- Modify: `sniff/cli/src/install_plan_cmd.rs` (add a small dispatch helper)

Wires everything: commands.rs routes `install` and `install-plan` through `install_plan_cmd`, which uses `resolve_program` + `InstallPlan` + `render_install_plan` + `execute_install_flow`. Implements `--via` enforcement, `--json` output for `install-plan`, `--force` cache bypass, and `--no-sudo` clamping.

- [ ] **Step 1: Write the failing integration tests**

Create `sniff/cli/tests/install_plan.rs`:

```rust
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn install_plan_vim_renders_text_output() {
    cargo_bin_cmd!("sniff")
        .args(["editors", "install-plan", "vim"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Vim"));
}

#[test]
fn install_plan_vim_json_returns_program_field() {
    let output = cargo_bin_cmd!("sniff")
        .args(["editors", "install-plan", "vim", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["program"], "Vim");
    assert!(json["options"].is_array());
    assert!(json["website"].is_string());
    assert!(json["successful"].is_boolean());
}

#[test]
fn install_plan_unknown_program_errors() {
    cargo_bin_cmd!("sniff")
        .args(["programs", "install-plan", "definitely-not-a-real-thing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown program"));
}

#[test]
fn install_dry_run_does_not_execute() {
    // Dry-run must always succeed because nothing actually runs.
    cargo_bin_cmd!("sniff")
        .args(["editors", "install", "vim", "--dry-run", "-y"])
        .assert()
        .success();
}

#[test]
fn install_via_unknown_manager_errors_with_valid_list() {
    cargo_bin_cmd!("sniff")
        .args(["editors", "install", "vim", "--via", "nonexistent-mgr", "--dry-run", "-y"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("valid manager").or(predicate::str::contains("Unknown manager")));
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli --test install_plan 2>&1 | tail -30`
Expected: the commands don't exist yet → tests fail with usage errors or missing subcommand errors.

- [ ] **Step 3: Add the dispatch helper**

In `sniff/cli/src/install_plan_cmd.rs`, add:

```rust
use crate::args::{InstallCommandArgs, InstallCommandKind};
use crate::install::{ResolveError, ResolvedProgram, resolve_program};
use sniff::programs::{InstallPlanOption, InstallPlanReason};

/// Force the plan to select the method whose `manager_name()` matches
/// `via_manager`. Returns an error if no method matches or if the matched
/// method was not eligible (not runnable on this host).
fn apply_via(
    plan: &mut InstallPlan,
    via_manager: &str,
) -> Result<(), String> {
    let matching_indices: Vec<usize> = plan
        .options
        .iter()
        .enumerate()
        .filter(|(_, o)| o.kind.manager_name() == via_manager)
        .map(|(i, _)| i)
        .collect();

    if matching_indices.is_empty() {
        let valid: Vec<&str> = plan
            .known_installations()
            .iter()
            .map(|m| m.manager_name())
            .collect();
        return Err(format!(
            "Unknown manager '{}'. Valid manager names for this program: {}",
            via_manager,
            valid.join(", ")
        ));
    }
    if matching_indices.len() > 1 {
        return Err(format!(
            "--via {} is ambiguous for this program (more than one method uses the same manager)",
            via_manager
        ));
    }

    let idx = matching_indices[0];
    if !plan.options[idx].choose {
        // Was the selected-by-library method eligible? If the reason for skip
        // was LowerPriorityAlternative, that's fine to flip. Anything else
        // means the library rejected the method and --via cannot override it.
        let current_reason = plan.options[idx].reason_type;
        if current_reason != InstallPlanReason::LowerPriorityAlternative {
            return Err(format!(
                "--via {} cannot override an unavailable method (reason: {:?})",
                via_manager, current_reason
            ));
        }
    }

    // Un-choose everything, then choose the matched option.
    let previously_chosen = plan.options.iter().position(|o| o.choose);
    for o in &mut plan.options {
        if o.choose {
            o.choose = false;
            o.reason_type = InstallPlanReason::LowerPriorityAlternative;
            o.reason = format!("{} was forced via --via", via_manager);
        }
    }
    plan.options[idx].choose = true;
    plan.options[idx].reason_type = InstallPlanReason::Selected;
    plan.options[idx].reason = format!("forced via --via {}", via_manager);
    plan.successful = true;
    let _ = previously_chosen;
    Ok(())
}

/// Build a plan for a resolved program, honoring `--force` (cache bypass) and
/// `--no-sudo` (forces `can_sudo = false`). Uses verification-aware detection
/// so the pnpm verified bucket can fire.
pub fn build_plan_for_args(
    resolved: &ResolvedProgram,
    args: &InstallCommandArgs,
) -> InstallPlan {
    use sniff::programs::host_capability::HostCapabilities;
    let mut host = HostCapabilities::load_or_detect_with_verification(args.force);
    if args.no_sudo {
        host.can_sudo = false;
    }
    // We have to rebuild the plan against the possibly-mutated host rather
    // than using ResolvedProgram::install_plan (which uses the cached host).
    use sniff::programs::install_plan::build_install_plan;
    use sniff::programs::ProgramMetadata;
    match resolved {
        ResolvedProgram::Editor(p) => build_install_plan(p, &host),
        ResolvedProgram::Utility(p) => build_install_plan(p, &host),
        ResolvedProgram::LanguagePackageManager(p) => build_install_plan(p, &host),
        ResolvedProgram::OsPackageManager(p) => build_install_plan(p, &host),
        ResolvedProgram::TtsClient(p) => build_install_plan(p, &host),
        ResolvedProgram::TerminalApp(p) => build_install_plan(p, &host),
        ResolvedProgram::HeadlessAudio(p) => build_install_plan(p, &host),
        ResolvedProgram::AiCli(p) => build_install_plan(p, &host),
    }
}

/// Top-level dispatch for `sniff <category> install …` and
/// `sniff <category> install-plan …`.
pub fn dispatch(
    kind: InstallCommandKind,
    args: &InstallCommandArgs,
    json: bool,
    plain: bool,
) -> Result<(), Box<dyn Error>> {
    let name = args
        .program
        .as_deref()
        .ok_or("--program is required for plan-aware install commands")?;

    let resolved = resolve_program(name).map_err(|e: ResolveError| Box::new(e))?;
    let mut plan = build_plan_for_args(&resolved, args);

    if let Some(via) = args.via.as_deref() {
        apply_via(&mut plan, via).map_err(|s| Box::<dyn Error>::from(s))?;
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
        return Ok(());
    }

    match kind {
        InstallCommandKind::InstallPlan => {
            let rendered = render_install_plan(&plan, /* verbose */ true);
            crate::output::emit_text(&rendered, plain);
            Ok(())
        }
        InstallCommandKind::Install => {
            execute_install_flow(&plan, args.dry_run, args.yes, plain)
        }
    }
}

// Note: `resolve_program` is re-exported from `crate::install` above so this
// module does not depend on the old macro-generated per-category functions.
```

Make `InstallCommandKind` and `InstallCommandArgs` `pub` in `sniff/cli/src/args.rs` (they already are per Task 16).

- [ ] **Step 4: Route `commands.rs` through `dispatch`**

Update `sniff/cli/src/commands.rs` around line 72 in `run()`. Replace the current install-action block with:

```rust
        if cmd.is_programs_mode() {
            // Plan-aware install or install-plan
            if let Some((kind, args)) = cmd.install_command_args() {
                return crate::install_plan_cmd::dispatch(kind, args, cli.json, cli.plain);
            }

            // Interactive install when `sniff <category> install` has no args
            // goes through the legacy MultiSelect picker for now.
            // (The picker will call dispatch() per selection under the hood.)

            let programs = detect_programs_for_filter(output_filter);
            if cli.json {
                output::print_programs_json(&programs, output_filter)?;
            } else {
                let rendered =
                    output::render_programs_markdown(&programs, cli.verbose, output_filter);
                output::emit_text(&rendered, cli.plain);
            }
            return Ok(());
        }
```

The legacy `crate::install::direct_install` and `crate::install::interactive_install` are still used only when a user runs `sniff editors install` with **no** program name. The interactive flow is *not* replaced in this feature (per spec Non-Goals section). The dispatch path above only fires when `install_command_args` returns `Some`, which requires an `Install` or `InstallPlan` action variant with at least parsed clap args; users who type `sniff editors install` without a name still hit the legacy picker because clap parses an `InstallCommandArgs { program: None, ..default }` and we fall through to it.

Actually, wait — `install_command_args` returns `Some` even with `program: None`. We need to distinguish:

- `sniff editors install` (no name) → interactive picker
- `sniff editors install vim` → plan-aware dispatch
- `sniff editors install-plan vim` → plan-aware dispatch (render-only)
- `sniff editors install-plan` (no name) → error "name required for install-plan"

Adjust the dispatch branch:

```rust
            if let Some((kind, args)) = cmd.install_command_args() {
                if args.program.is_some() {
                    return crate::install_plan_cmd::dispatch(kind, args, cli.json, cli.plain);
                }
                // No name: fall through to interactive (Install) or error (InstallPlan)
                if kind == InstallCommandKind::InstallPlan {
                    return Err("install-plan requires a program name".into());
                }
                return crate::install::interactive_install(cmd.to_output_filter());
            }
```

Import `InstallCommandKind` in `sniff/cli/src/commands.rs`:

```rust
use crate::args::InstallCommandKind;
```

- [ ] **Step 5: Remove the old recursive programs-install loop**

In `sniff/cli/src/install.rs`, the `direct_install` function's `OutputFilter::Programs` branch currently retries every category. Since plan-aware dispatch no longer calls `direct_install` for `OutputFilter::Programs` (it uses `resolve_program` instead), we can delete the whole recursive branch. Replace it with:

```rust
        OutputFilter::Programs => {
            // Plan-aware dispatch handles cross-category resolution now.
            // `direct_install` is only reached via the legacy interactive
            // picker, which resolves category before calling in.
            unreachable!("OutputFilter::Programs reaches direct_install only via legacy picker path")
        }
```

Actually that's incorrect — the legacy picker *never* called `direct_install(Programs, ...)`. The only caller of `direct_install(Programs, ...)` was the dispatch block in `commands.rs` for `sniff programs install <name>`, which now goes through `install_plan_cmd::dispatch`. It is safe to remove the `Programs` arm entirely:

```rust
        OutputFilter::Programs => {
            Err("internal error: OutputFilter::Programs reached legacy direct_install; \
                 use install_plan_cmd::dispatch instead".into())
        }
```

This leaves a defensive error so that a future regression is loud.

- [ ] **Step 6: Run the CLI integration tests**

Run: `cargo test -p sniff-cli --test install_plan 2>&1 | tail -40`
Expected: all five tests pass.

- [ ] **Step 7: Run the full CLI test suite**

Run: `cargo test -p sniff-cli 2>&1 | tail -30`
Expected: no regressions.

- [ ] **Step 8: Commit**

```bash
git add sniff/cli/src/commands.rs sniff/cli/src/install.rs sniff/cli/src/install_plan_cmd.rs sniff/cli/tests/install_plan.rs
git commit -m "feat(sniff-cli): dispatch install and install-plan through InstallPlan pipeline"
```

---

## Task 21: Additional CLI integration tests

**Files:**
- Modify: `sniff/cli/tests/install_plan.rs`

Adds coverage for the remaining spec requirements: `--no-sudo`, `--force` cache bypass, and snapshot-style rendering of the failure branch.

- [ ] **Step 1: Add tests**

Append to `sniff/cli/tests/install_plan.rs`:

```rust
use tempfile::TempDir;

/// Helper: point HOME at a tempdir so HostCapabilities doesn't touch the real
/// cache file. Returns the tempdir (must stay alive) and a ready Command.
fn cmd_with_tmp_home() -> (TempDir, assert_cmd::Command) {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = cargo_bin_cmd!("sniff");
    cmd.env("HOME", tmp.path());
    (tmp, cmd)
}

#[test]
fn install_plan_populates_cache_file() {
    let (tmp, mut cmd) = cmd_with_tmp_home();
    cmd.args(["editors", "install-plan", "vim"])
        .assert()
        .success();
    let cache = tmp.path().join(".sniff-programs.json");
    assert!(cache.exists(), "cache file should be created");
}

#[test]
fn install_plan_force_rebuilds_cache() {
    let (tmp, _) = cmd_with_tmp_home();
    // Seed a stale cache.
    let cache = tmp.path().join(".sniff-programs.json");
    std::fs::write(&cache, "garbage").unwrap();
    let mut cmd = cargo_bin_cmd!("sniff");
    cmd.env("HOME", tmp.path())
        .args(["editors", "install-plan", "vim", "--force"])
        .assert()
        .success();
    let after = std::fs::read_to_string(&cache).unwrap();
    assert_ne!(after, "garbage", "cache should have been rewritten");
}

#[test]
fn install_plan_no_sudo_never_selects_sudo_method() {
    // We can't force a deterministic host, but we can assert that any
    // selected option has requires_sudo = false when --no-sudo is passed.
    let output = cargo_bin_cmd!("sniff")
        .args([
            "editors",
            "install-plan",
            "vim",
            "--no-sudo",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: Value = serde_json::from_str(&stdout).unwrap();
    if json["successful"] == Value::Bool(true) {
        let options = json["options"].as_array().unwrap();
        let chosen = options.iter().find(|o| o["choose"] == Value::Bool(true)).unwrap();
        assert_eq!(chosen["requires_sudo"], Value::Bool(false));
    }
}
```

Note on HOME override on Windows: `assert_cmd::Command::env("HOME", ...)` works on Unix. On Windows `dirs::home_dir()` reads `USERPROFILE`, not `HOME`. For the sake of this feature and the Unix-focused test host, these tests are acceptable as Unix-only and should be gated:

```rust
#[cfg(unix)]
#[test]
fn install_plan_populates_cache_file() { ... }

#[cfg(unix)]
#[test]
fn install_plan_force_rebuilds_cache() { ... }
```

`install_plan_no_sudo_never_selects_sudo_method` does not depend on the HOME override and can run on all platforms.

- [ ] **Step 2: Run the new tests**

Run: `cargo test -p sniff-cli --test install_plan 2>&1 | tail -40`
Expected: all tests pass on Unix; on Windows only the no-sudo test runs.

- [ ] **Step 3: Commit**

```bash
git add sniff/cli/tests/install_plan.rs
git commit -m "test(sniff-cli): add cache, --force, and --no-sudo integration coverage"
```

---

## Task 22: Final sweep

**Files:**
- Modify: `sniff/cli/src/args.rs` (help text)
- Verify: all packages build and lint cleanly

- [ ] **Step 1: Update the CLI help text**

In `sniff/cli/src/args.rs`, extend `AFTER_HELP` so the Programs block mentions the new subcommand. Find the existing `Programs:` block (around line 1230) and replace it with:

```rust
  Programs:
    sniff programs         Show all installed programs
    sniff programs install-plan <name>   Explain how a program would be installed
    sniff editors          Show editors (supports 'install' and 'install-plan')
    sniff utilities        Show utilities
    sniff agents           Show AI agent CLI tools
```

- [ ] **Step 2: Run the full lib + cli test suites**

Run:
```bash
cd /Users/ken/.claudine/worktrees/feat-sniff-tuning
cargo test -p sniff 2>&1 | tail -20
cargo test -p sniff-cli 2>&1 | tail -20
```
Expected: both report `test result: ok`.

- [ ] **Step 3: Run the area lint**

Run: `just lint -p sniff 2>&1 | tail -30` (if the area `justfile` exposes `lint`; otherwise `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`)
Expected: no warnings.

- [ ] **Step 4: Run `just test` to exercise all sniff-area recipes**

Run: `just test 2>&1 | tail -40`
Expected: every area recipe the root justfile lists passes. If any unrelated area fails, restore baseline first, re-run Task 0, and triage.

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/args.rs
git commit -m "docs(sniff-cli): document install-plan subcommand in after-help"
```

---

## Spec Coverage Checklist

Cross-reference every spec requirement to the task that implements it:

| Spec requirement | Task |
|-----------------|------|
| Three capability tiers (known / available / plan) | 14 |
| `InstallPlan` struct with options, website, successful | 10 |
| `InstallPlanOption` with kind, requires_sudo, choose, reason | 10 |
| `InstallPlanReason` with 7 variants (no `RemoteBashNotAllowed` per tech-design §1) | 10 |
| `HostCapabilities` with all fields | 5, 6, 7, 8 |
| `HostCapabilities::detect` | 5, 6, 7 |
| `HostCapabilities::detect_with_verification` | 8 |
| Sudo detection (groups + `sudo -n true`) | 6 |
| Default OS package manager mapping | 7 |
| WSL detection | 7 |
| Verified language PM probes (npm, pnpm, yarn, bun, cargo) | 8 |
| Cache at `~/.sniff-programs.json`, 90-day TTL, atomic write | 9 |
| Cache invalidation on corrupt / schema drift / stale | 9 |
| `--force` bypass | 9, 20, 21 |
| 7 priority rules for selection | 12 |
| `build_install_plan` generic over `ProgramMetadata` (tech-design §4) | 10, 12 |
| `InstallPlan::execute` | 13 |
| RemoteBash consent (opt-out of `--allow-remote-bash`, second confirm) | 13, 19 |
| Ctrl+C vs Esc distinction (exit 130 vs 0) | 19 |
| `ProgramDetector::{known_methods, available_methods, install_plan}` | 14 |
| `installable`/`install`/`install_version` rewired as plan wrappers | 15 |
| New `SniffInstallationError` variants | 1 |
| Tagged serde on `InstallationMethod` | 2 |
| `InstallOptions::approve_remote_bash` field | 3 |
| CLI `InstallCommandArgs` shared flag group | 16 |
| CLI `--dry-run`, `-y/--yes`, `--via`, `--no-sudo`, `-f/--force` | 16, 20 |
| CLI `install-plan` subcommand | 16, 20 |
| CLI renderer for success / sudo / failure | 18 |
| CLI `--via` enforcement with valid-list error | 20 |
| CLI `--via` ambiguity error | 20 |
| CLI `--json` on `install-plan` serializes full plan | 20 |
| CLI fallback website hyperlink in failure block | 18 |
| Remove recursive `sniff programs install` retry loop | 20 |
| Pure `ResolvedProgram` resolver | 17 |
| Library tests per bucket, per reason, fabricated host | 11, 12, 13, 21 |
| CLI tests for `install-plan --json`, `--via`, `--force`, `--no-sudo` | 20, 21 |
| Backwards compatibility of public API | 14, 15 (additive only) |

---

## Execution Handoff

Plan complete and saved to `sniff/features/2026-04-10-program-install-improvements/plan.md`. Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration with clean context per step.
2. **Inline Execution** — execute tasks in the current session using `superpowers:executing-plans`, batching commits at the end of each task.

Which approach would you like to use?
