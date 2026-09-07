//! Two structural gates over the L1 test binaries: the `claudine` process comes
//! from the fixture builder, and the isolation the builder gave it survives
//! `build()`.
//!
//! ## The spawn gate
//!
//! A raw `assert_cmd::Command::cargo_bin("claudine")` — or a shell-out through
//! `common::claudine_bin`, or a `biscuit_test_harness::bin_exe!("claudine")`
//! handed to `std::process::Command::new` — inherits the runner's environment: the
//! rusty-biscuit checkout as the launch context, the host `PATH` with every
//! installed agentic CLI on it, and the developer's `$HOME`. That is what makes
//! the slow tests slow and the CI legs divergent, and no per-test `.env(...)`
//! chain can be relied on to have covered all of it. `CliProcessFixture`'s
//! builder is the one supported spawn; this guard keeps the alternative from
//! regrowing.
//!
//! [`SPAWN_ALLOWLIST`] is a burn-down list, not a permanent exemption table:
//! every entry names a file and states why it still spawns raw. An entry that
//! matches no live site fails the guard, so a migration cannot leave the list
//! stale, and a file with a live site but no entry fails it too.
//!
//! ## The isolation gate
//!
//! `build()` hands back a bare `assert_cmd::Command`, so the supported spawn
//! form is not by itself the contract: a call site can write
//! `.current_dir(repository_root())` or `.env("PATH", augmented_path(&dir))` on
//! the returned command and reinstate both leaks the migration removed, while
//! the spawn gate above stays perfectly happy. The second gate scans for those
//! post-`build()` escapes — `.current_dir`, `.env("PATH", …)`,
//! `.env_remove("PATH")`, `.env_clear()`, and any reach for
//! `common::augmented_path` — and reconciles them against
//! [`ISOLATION_ALLOWLIST`] on the same mechanics.
//!
//! Every legitimate need is already a named builder method
//! (`ambient_context`, `host_path`, `fake_only_path`, `inherit_no_env`), which
//! is why the allow-list is empty: the escapes are spelled on the builder, not
//! on the command.
//!
//! ## What each gate governs
//!
//! `level2_*`, `level3_*`, and `real_*` files are out of both scans. They drive
//! real terminals, real providers, and real host tooling on purpose, so the
//! hermetic default is the wrong contract for them. `common/` is excluded
//! because that is where the builder — and `augmented_path` itself — lives.
//!
//! The isolation gate narrows that population twice more:
//!
//! - [`SPAWN_ALLOWLIST`] files are out. They still spawn raw and set their own
//!   environment by hand, so flagging them would be noise on code the burn-down
//!   has not reached. Deleting a file's spawn entry is the single edit that
//!   turns the isolation contract on for it.
//! - A file that never names `CliProcessFixture` is out. It holds no
//!   builder-produced command, so it has no isolation to defeat —
//!   `system_prompt_perf_bench.rs` pins `.current_dir` on a
//!   `std::process::Command::new("git")`, which is not this contract's
//!   business.
//!
//! The residual blind spot is the mirror of that last rule: a `.current_dir` on
//! a non-claudine command *inside* a governed file reads the same as one on a
//! fixture command, because the scan is textual and does not resolve receivers.
//! No governed file has one today; the file that acquires one takes an
//! allow-list entry saying which command it targets.
//!
//! ## Reading the burn-down
//!
//! Each gate prints its census on stderr and writes the same census to a JSON
//! Lines artifact in the staging directory — `$BISCUIT_JUNIT_STAGE_DIR`, else
//! `target/nextest/ci-reports`, resolved by [`test_toolkit::stage_dir`]. The
//! spawn gate writes [`SPAWN_REPORT_FILE`], the isolation gate
//! [`ISOLATION_REPORT_FILE`].
//!
//! The stderr copy is what makes a *failing* run legible, but nextest's
//! `success-output` defaults to `never`, so on a green run it goes nowhere —
//! and a green run is exactly when the burn-down is worth reading. The artifact
//! is the durable half. CI already uploads the whole staging directory as
//! `junit-claudine-cli-L1-<environment>`, so the two files ride along with the
//! JUnit reports without any workflow change.
//!
//! Three record kinds, discriminated by `kind` and tagged with the `gate` that
//! wrote them:
//!
//! - `file` — one allow-listed file that still holds live sites, with its
//!   count and the allow-list reason it carries.
//! - `reason` — the roll-up for one reason: how many files and how many sites
//!   are still exempt under it. This is the number that has to reach zero.
//! - `total` — `files`/`sites` still allow-listed, `scanned_sites` the
//!   detector found in total, and `governed_files` the size of the population
//!   scanned. `sites` below `scanned_sites` means live sites are *unlisted*,
//!   which is the failure the gate asserts on.
//!
//! Each file is rewritten whole by the single test that owns it, so the
//! artifact is the last run's census and never accumulates across runs. Two
//! files rather than one for the same reason: nextest gives each test its own
//! process, and two processes truncating one path would clobber each other.
//!
//! ```console
//! $ cat target/nextest/ci-reports/spawn-site-burn-down.jsonl
//! {"kind":"file","gate":"spawn","file":"compose_cli.rs","reason":"outside this fix's scope","sites":4}
//! …
//! {"kind":"reason","gate":"spawn","reason":"outside this fix's scope","files":34,"sites":168}
//! {"kind":"total","gate":"spawn","files":36,"sites":170,"scanned_sites":170,"governed_files":83}
//! ```

// Included directly rather than through `mod common;`: this binary needs the
// sanitizer and nothing else, and `common/mod.rs` drags in the fixture surface
// (`common::wrap`'s `claudine::mcp::types`/`chrono`/`serde_json`, and
// `common::pty`'s expectrl on Unix) that a text scanner has no use for.
#[path = "common/source_scan.rs"]
mod source_scan;

use source_scan::{is_ident, line_at, sanitize};

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// `assert_cmd::Command::cargo_bin("claudine")`.
const FORM_CARGO_BIN: &str = "cargo_bin";
/// A shell-out through `common::claudine_bin()`.
const FORM_CLAUDINE_BIN: &str = "claudine_bin";
/// `biscuit_test_harness::bin_exe!("claudine")` fed to a raw `Command::new`.
const FORM_BIN_EXE: &str = "bin_exe";

/// `.current_dir(…)` on a built command — the `ambient_context` escape, taken
/// without the containment check that escape enforces.
const FORM_CURRENT_DIR: &str = ".current_dir(…)";
/// `.env("PATH", …)` on a built command — the `host_path`/`fake_only_path`
/// escapes, taken without saying which of the two is meant.
const FORM_PATH_ENV: &str = r#".env("PATH", …)"#;
/// `.env_remove("PATH")` — the child loses the fixture `bin` and falls back to
/// whatever the platform hands a `PATH`-less process.
const FORM_PATH_ENV_REMOVE: &str = r#".env_remove("PATH")"#;
/// `.env_clear()` on a built command, which drops the defaults `build()` set.
const FORM_ENV_CLEAR: &str = ".env_clear()";
/// A direct reach for `common::augmented_path`, bypassing `host_path()`.
const FORM_AUGMENTED_PATH: &str = "augmented_path";

/// File-name prefixes whose spawn contract is deliberately not hermetic.
const EXCLUDED_PREFIXES: &[&str] = &["level2_", "level3_", "real_"];

/// One flagged site: a raw spawn, or a post-`build()` isolation escape.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    /// Path relative to `claudine/cli/tests`, `/`-separated.
    file: String,
    line: usize,
    form: &'static str,
}

/// One file a gate still exempts, with the reason it is exempt.
struct AllowEntry {
    /// Path relative to `claudine/cli/tests`, `/`-separated.
    file: &'static str,
    reason: &'static str,
}

/// Reason shared by the spawn-shaped files this fix does not touch.
const OUT_OF_SCOPE: &str = "outside this fix's scope";

/// Reason shared by the two Windows console-control tests.
///
/// Both hold the spawned `claudine` as a live [`std::process::Child`] — they
/// inject `GenerateConsoleCtrlEvent` while it runs and then poll `try_wait` —
/// and both need `CREATE_NEW_PROCESS_GROUP` so the event reaches only the
/// wrapper subtree and not the test runner's own console group.
/// `assert_cmd::Command` offers neither: it has no `spawn`, only the blocking
/// `output`/`assert` family, and it never hands back the inner
/// `std::process::Command` that `CommandExt::creation_flags` extends. Closing
/// these two needs a fixture builder that can yield a `std::process::Command`,
/// not a call-site migration.
const NEEDS_LIVE_CHILD: &str =
    "needs a live Child + CREATE_NEW_PROCESS_GROUP; assert_cmd has neither";

/// Seeded from the raw-spawn census taken when this guard landed (Phase 2 of
/// `claudine/fixes/2026-08-01-cli-slow-tests`). Each migration phase deletes
/// its file's entry; the list shrinking is how the burn-down is observed.
const SPAWN_ALLOWLIST: &[AllowEntry] = &[
    AllowEntry {
        file: "completion_contract.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "completion_perf.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "completion_resolution_round_trip.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "compose_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "compose_interactive_timeout_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "compose_removed_validation_keys.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "compose_schema_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "compose_system_prompt_lifetime.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "compose_ttff_perf.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "composition_outputs.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "context_command.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "effective_diagnostic_render.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "errors_command.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "handle_blocking_output.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "handle_deadline.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "inline_compose_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "level1_compose_autocomplete_failure_pty.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "level1_inline_compose_mismatch_pty.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "level1_structured_error_message.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "loop_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "protect_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "provider_error_finalize.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "sequence_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "sequence_ctrl_c_windows.rs",
        reason: NEEDS_LIVE_CHILD,
    },
    AllowEntry {
        file: "sequence_errors_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "sequence_groups.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "sequence_jit.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "sequence_magic_reference.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "sequence_overlay_pty.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "sequence_prompt_property.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "sequence_sources_cli.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "shipped_prompts.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "skills_integration.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "wrap_ctrl_c_windows.rs",
        reason: NEEDS_LIVE_CHILD,
    },
    AllowEntry {
        file: "wrap_sequence_composition.rs",
        reason: OUT_OF_SCOPE,
    },
    AllowEntry {
        file: "wrap_sigint.rs",
        reason: OUT_OF_SCOPE,
    },
];

/// Files allowed to undo the builder's isolation after `build()`.
///
/// Empty, and expected to stay that way: every legitimate need is a named
/// builder method, so an entry here is a claim that the builder is missing one.
/// The mechanics are [`SPAWN_ALLOWLIST`]'s — a reason is mandatory, and an entry
/// naming a file with no live escape fails the guard as stale.
const ISOLATION_ALLOWLIST: &[AllowEntry] = &[];

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

fn identifier_at(code: &[u8], index: usize) -> Option<(&[u8], usize)> {
    if !is_ident(code[index]) || code[index].is_ascii_digit() {
        return None;
    }
    if index > 0 && is_ident(code[index - 1]) {
        return None;
    }
    let mut end = index;
    while end < code.len() && is_ident(code[end]) {
        end += 1;
    }
    Some((&code[index..end], end))
}

fn skip_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

/// Whether the call opening at `after_name` passes `literal` as its first
/// argument.
///
/// Read from the *original* source: [`sanitize`] blanks the literal, so the
/// sanitized buffer can say a `cargo_bin(…)` call exists but not which binary
/// it names — and `cargo_bin("md")` is a legitimate darkmatter shim spawn.
fn names_literal(source: &[u8], after_name: usize, literal: &str) -> bool {
    let cursor = skip_whitespace(source, after_name);
    if source.get(cursor) != Some(&b'(') {
        return false;
    }
    let cursor = skip_whitespace(source, cursor + 1);
    let quoted = format!("\"{literal}\"");
    source[cursor..].starts_with(quoted.as_bytes())
}

fn names_claudine(source: &[u8], after_name: usize) -> bool {
    names_literal(source, after_name, "claudine")
}

fn opens_a_call(code: &[u8], after_name: usize) -> bool {
    code.get(skip_whitespace(code, after_name)) == Some(&b'(')
}

/// Whether the identifier starting at `index` sits in method position — the
/// nearest non-whitespace byte before it is a `.`.
///
/// This is what separates `command.env("PATH", …)` from `std::env::var("PATH")`
/// and `.current_dir(dir)` from `std::env::current_dir()`: the path forms are
/// preceded by a `:`, never a dot.
fn is_method_call(code: &[u8], index: usize) -> bool {
    let mut cursor = index;
    while cursor > 0 && code[cursor - 1].is_ascii_whitespace() {
        cursor -= 1;
    }
    cursor > 0 && code[cursor - 1] == b'.'
}

/// Whether the macro whose name ends at `after_name` is invoked with the
/// literal `"claudine"` — `bin_exe!("claudine")`.
///
/// Only the `!` separates this from the `cargo_bin` arm's shape, so it steps
/// over the bang and reuses [`names_claudine`] for the argument itself.
fn macro_names_claudine(source: &[u8], after_name: usize) -> bool {
    let cursor = skip_whitespace(source, after_name);
    source.get(cursor) == Some(&b'!') && names_claudine(source, cursor + 1)
}

/// Every raw `claudine` spawn site in `source`, as `(line, form)` pairs.
fn spawn_sites(source: &str) -> Vec<(usize, &'static str)> {
    let code = sanitize(source);
    let bytes = source.as_bytes();
    let mut sites = Vec::new();
    let mut index = 0;
    while index < code.len() {
        let Some((name, end)) = identifier_at(&code, index) else {
            index += 1;
            continue;
        };
        match name {
            b"cargo_bin" if names_claudine(bytes, end) => {
                sites.push((line_at(source, index), FORM_CARGO_BIN));
            }
            b"claudine_bin" if opens_a_call(&code, end) => {
                sites.push((line_at(source, index), FORM_CLAUDINE_BIN));
            }
            b"bin_exe" if macro_names_claudine(bytes, end) => {
                sites.push((line_at(source, index), FORM_BIN_EXE));
            }
            _ => {}
        }
        index = end;
    }
    sites
}

/// Every post-`build()` isolation escape in `source`, as `(line, form)` pairs.
///
/// The four command forms are recognized in method position only. The fifth,
/// `augmented_path`, is flagged wherever it is named in executable code —
/// including a `use` — because importing it is already the reach-around the
/// `host_path()` escape exists to replace.
fn isolation_sites(source: &str) -> Vec<(usize, &'static str)> {
    let code = sanitize(source);
    let bytes = source.as_bytes();
    let mut sites = Vec::new();
    let mut index = 0;
    while index < code.len() {
        let Some((name, end)) = identifier_at(&code, index) else {
            index += 1;
            continue;
        };
        let method = is_method_call(&code, index);
        match name {
            b"current_dir" if method && opens_a_call(&code, end) => {
                sites.push((line_at(source, index), FORM_CURRENT_DIR));
            }
            b"env" if method && names_literal(bytes, end, "PATH") => {
                sites.push((line_at(source, index), FORM_PATH_ENV));
            }
            b"env_remove" if method && names_literal(bytes, end, "PATH") => {
                sites.push((line_at(source, index), FORM_PATH_ENV_REMOVE));
            }
            b"env_clear" if method && opens_a_call(&code, end) => {
                sites.push((line_at(source, index), FORM_ENV_CLEAR));
            }
            b"augmented_path" => {
                sites.push((line_at(source, index), FORM_AUGMENTED_PATH));
            }
            _ => {}
        }
        index = end;
    }
    sites
}

/// Whether `source` names `CliProcessFixture` in executable code.
///
/// The isolation gate's population test: a file that never obtains a
/// builder-produced command cannot undo one. Sanitized, so this guard's own
/// prose — and its assertion messages — do not enrol a file that has none.
fn obtains_a_fixture_command(source: &str) -> bool {
    let code = sanitize(source);
    let mut index = 0;
    while index < code.len() {
        match identifier_at(&code, index) {
            Some((b"CliProcessFixture", _)) => return true,
            Some((_, end)) => index = end,
            None => index += 1,
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Scanning
// ---------------------------------------------------------------------------

fn tests_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether `relative` is outside this guard's contract.
///
/// Kept a pure function of the relative path so the tier-naming rules are
/// testable without a filesystem.
fn excluded(relative: &str) -> bool {
    if relative.starts_with("common/") {
        return true;
    }
    let Some(name) = relative.rsplit('/').next() else {
        return true;
    };
    EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Whether a file is in the spawn gate's population.
fn governs_spawn(relative: &str, _source: &str) -> bool {
    !excluded(relative)
}

/// Whether `relative` is named by [`SPAWN_ALLOWLIST`].
fn spawn_allowlisted(relative: &str) -> bool {
    SPAWN_ALLOWLIST.iter().any(|entry| entry.file == relative)
}

/// Whether a file is in the isolation gate's population — the migrated L1 files
/// that actually hold a builder-produced command. See the module docs for why
/// each narrowing is there.
fn governs_isolation(relative: &str, source: &str) -> bool {
    !excluded(relative) && !spawn_allowlisted(relative) && obtains_a_fixture_command(source)
}

/// Every file under `root` a gate governs, as `(relative path, source)`.
fn governed_files(
    root: &Path,
    governs: fn(&str, &str) -> bool,
) -> Result<Vec<(String, String)>, String> {
    let mut files = Vec::new();
    collect_rust_files(root, &mut files)
        .map_err(|error| format!("failed to scan {}: {error}", root.display()))?;
    files.sort();

    let mut governed = Vec::new();
    for file in files {
        let relative_path = relative(root, &file);
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
        if governs(&relative_path, &source) {
            governed.push((relative_path, source));
        }
    }
    Ok(governed)
}

/// Every site `detect` finds across an already-read population.
///
/// Split out of [`scan`] so a gate that also needs the population size does not
/// have to read all ~200 files twice to get it.
fn sites_in(
    governed: &[(String, String)],
    detect: fn(&str) -> Vec<(usize, &'static str)>,
) -> Vec<Site> {
    let mut sites = Vec::new();
    for (file, source) in governed {
        sites.extend(detect(source).into_iter().map(|(line, form)| Site {
            file: file.clone(),
            line,
            form,
        }));
    }
    sites
}

fn scan(
    root: &Path,
    governs: fn(&str, &str) -> bool,
    detect: fn(&str) -> Vec<(usize, &'static str)>,
) -> Result<Vec<Site>, String> {
    Ok(sites_in(&governed_files(root, governs)?, detect))
}

// ---------------------------------------------------------------------------
// Allow-list reconciliation
// ---------------------------------------------------------------------------

#[derive(Debug, Default, PartialEq, Eq)]
struct Reconciliation {
    /// Live sites in files no entry covers, as `file:line form`.
    unlisted: Vec<String>,
    /// Entries covering no live site.
    stale: Vec<String>,
    /// Entries with a blank reason.
    unexplained: Vec<String>,
    /// Live site count per allow-listed file.
    burn_down: BTreeMap<String, usize>,
}

fn reconcile(sites: &[Site], allowlist: &[AllowEntry]) -> Reconciliation {
    let allowed: BTreeSet<&str> = allowlist.iter().map(|entry| entry.file).collect();
    let mut result = Reconciliation::default();

    for site in sites {
        if allowed.contains(site.file.as_str()) {
            *result.burn_down.entry(site.file.clone()).or_default() += 1;
        } else {
            result
                .unlisted
                .push(format!("{}:{} {}", site.file, site.line, site.form));
        }
    }
    for entry in allowlist {
        if !sites.iter().any(|site| site.file == entry.file) {
            result.stale.push(entry.file.to_string());
        }
        if entry.reason.trim().is_empty() {
            result.unexplained.push(entry.file.to_string());
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Burn-down artifact
// ---------------------------------------------------------------------------

/// The spawn gate's roll-up, inside the staging directory.
const SPAWN_REPORT_FILE: &str = "spawn-site-burn-down.jsonl";
/// The isolation gate's roll-up, inside the staging directory.
const ISOLATION_REPORT_FILE: &str = "isolation-burn-down.jsonl";

/// `gate` tag on every record [`SPAWN_REPORT_FILE`] carries.
const GATE_SPAWN: &str = "spawn";
/// `gate` tag on every record [`ISOLATION_REPORT_FILE`] carries.
const GATE_ISOLATION: &str = "isolation";

/// One gate's census, in the shape both the stderr copy and the artifact are
/// rendered from.
struct BurnDown {
    gate: &'static str,
    /// Size of the population the gate scanned.
    governed_files: usize,
    /// Every site the detector found, allow-listed or not.
    scanned_sites: usize,
    /// `(file, live sites, allow-list reason)` for each allow-listed file that
    /// still holds sites, ordered by file.
    per_file: Vec<(String, usize, &'static str)>,
    /// Allow-list reason -> `(files, sites)` still exempt under it.
    per_reason: BTreeMap<&'static str, (usize, usize)>,
}

impl BurnDown {
    fn new(
        gate: &'static str,
        governed_files: usize,
        scanned_sites: usize,
        allowlist: &[AllowEntry],
        result: &Reconciliation,
    ) -> Self {
        let mut per_file = Vec::new();
        let mut per_reason: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
        for (file, count) in &result.burn_down {
            let reason = allowlist
                .iter()
                .find(|entry| entry.file == file)
                .map(|entry| entry.reason)
                .unwrap_or_default();
            let rollup = per_reason.entry(reason).or_default();
            rollup.0 += 1;
            rollup.1 += count;
            per_file.push((file.clone(), *count, reason));
        }
        Self {
            gate,
            governed_files,
            scanned_sites,
            per_file,
            per_reason,
        }
    }

    /// Allow-listed files that still hold at least one live site.
    fn files(&self) -> usize {
        self.per_file.len()
    }

    /// Live sites those files hold between them.
    fn sites(&self) -> usize {
        self.per_file.iter().map(|(_, count, _)| count).sum()
    }

    /// The census as JSON Lines, one record per line, trailing newline included.
    fn to_jsonl(&self) -> String {
        let mut records = Vec::new();
        for (file, sites, reason) in &self.per_file {
            records.push(BurnDownRecord::File {
                gate: self.gate,
                file,
                reason,
                sites: *sites,
            });
        }
        for (reason, (files, sites)) in &self.per_reason {
            records.push(BurnDownRecord::Reason {
                gate: self.gate,
                reason,
                files: *files,
                sites: *sites,
            });
        }
        records.push(BurnDownRecord::Total {
            gate: self.gate,
            files: self.files(),
            sites: self.sites(),
            scanned_sites: self.scanned_sites,
            governed_files: self.governed_files,
        });

        let mut out = String::new();
        for record in &records {
            // Infallible: every field is a `&str` or a `usize`.
            out.push_str(&serde_json::to_string(record).expect("burn-down record serializes"));
            out.push('\n');
        }
        out
    }
}

/// One line of a burn-down artifact. See the module docs for how to read them.
#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum BurnDownRecord<'a> {
    File {
        gate: &'a str,
        file: &'a str,
        reason: &'a str,
        sites: usize,
    },
    Reason {
        gate: &'a str,
        reason: &'a str,
        files: usize,
        sites: usize,
    },
    Total {
        gate: &'a str,
        files: usize,
        sites: usize,
        scanned_sites: usize,
        governed_files: usize,
    },
}

/// Write `contents` to `path`, creating the staging directory if it is absent.
///
/// The write is a whole-file truncate, so re-running a gate replaces its census
/// instead of appending to the last run's.
///
/// ## Errors
///
/// Any I/O error from creating the parent directory or writing the file.
fn write_report(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

/// Emit one gate's census as `file_name` inside the staging directory —
/// `$BISCUIT_JUNIT_STAGE_DIR`, else `target/nextest/ci-reports`.
fn emit_report(file_name: &str, burn_down: &BurnDown) {
    emit_report_to(&test_toolkit::stage_dir().join(file_name), burn_down);
}

/// Write one gate's census to `path`, degrading to a warning on failure.
///
/// I/O failures are reported on stderr and otherwise swallowed: a structural
/// guard must not fail because it could not write its own telemetry, and the
/// staging directory is not guaranteed to exist or be writable on a developer's
/// machine.
fn emit_report_to(path: &Path, burn_down: &BurnDown) {
    if let Err(error) = write_report(path, &burn_down.to_jsonl()) {
        eprintln!(
            "warning: could not write the {} burn-down to {}: {error}",
            burn_down.gate,
            path.display()
        );
    }
}

/// The guard proper: raw `claudine` spawns exist only where the allow-list
/// says so, and every allow-list entry still describes a live site.
#[test]
fn l1_tests_spawn_claudine_through_the_fixture_builder() {
    let root = tests_root();
    let governed = governed_files(&root, governs_spawn).expect("spawn-site population");
    let sites = sites_in(&governed, spawn_sites);
    let result = reconcile(&sites, SPAWN_ALLOWLIST);
    let burn_down = BurnDown::new(
        GATE_SPAWN,
        governed.len(),
        sites.len(),
        SPAWN_ALLOWLIST,
        &result,
    );

    eprintln!(
        "spawn-site burn-down: {} raw sites in {} allow-listed files (of {} sites scanned)",
        burn_down.sites(),
        burn_down.files(),
        burn_down.scanned_sites
    );
    for (file, count, reason) in &burn_down.per_file {
        eprintln!("  {file}: {count} site(s) — {reason}");
    }
    for (reason, (files, site_count)) in &burn_down.per_reason {
        eprintln!("  == {reason}: {files} file(s), {site_count} site(s)");
    }
    // Before the assertions: a failing run wants the census too.
    emit_report(SPAWN_REPORT_FILE, &burn_down);

    assert!(
        result.unlisted.is_empty(),
        "Raw `claudine` spawns outside the fixture builder. These inherit the runner's \
         launch context, host PATH, and $HOME. Build the command with \
         `CliProcessFixture::command()` (or `command_builder()` plus a named escape), or — \
         if the file genuinely cannot — add it to SPAWN_ALLOWLIST in \
         cli/tests/spawn_site_guard.rs with a one-line reason.\nUnlisted sites:\n{}",
        result.unlisted.join("\n")
    );
    assert!(
        result.stale.is_empty(),
        "Stale SPAWN_ALLOWLIST entries name files with no raw spawn site left. Delete them \
         so the burn-down stays honest.\nStale entries:\n{}",
        result.stale.join("\n")
    );
    assert!(
        result.unexplained.is_empty(),
        "SPAWN_ALLOWLIST entries without a reason:\n{}",
        result.unexplained.join("\n")
    );
}

/// The second gate: a migrated L1 file does not undo, after `build()`, the
/// isolation the builder gave it.
#[test]
fn migrated_l1_tests_keep_the_isolation_the_builder_gave_them() {
    let root = tests_root();
    let governed = governed_files(&root, governs_isolation).expect("isolation population");
    let sites = sites_in(&governed, isolation_sites);
    let result = reconcile(&sites, ISOLATION_ALLOWLIST);
    let burn_down = BurnDown::new(
        GATE_ISOLATION,
        governed.len(),
        sites.len(),
        ISOLATION_ALLOWLIST,
        &result,
    );

    eprintln!(
        "post-build isolation: {} escape(s) across {} governed file(s)",
        burn_down.scanned_sites, burn_down.governed_files
    );
    for (file, count, reason) in &burn_down.per_file {
        eprintln!("  {file}: {count} escape(s) — {reason}");
    }
    emit_report(ISOLATION_REPORT_FILE, &burn_down);

    assert!(
        result.unlisted.is_empty(),
        "Post-`build()` escapes from the L1 isolation contract. `build()` returns a bare \
         `assert_cmd::Command`, so these silently reinstate the launch context or the host \
         PATH the fixture removed. Use the named builder escape instead — \
         `ambient_context(dir)` for the launch CWD, `host_path()` or `fake_only_path()` for \
         PATH, `inherit_no_env()` for a cleared environment — or, if the site targets a \
         command the fixture did not build, add the file to ISOLATION_ALLOWLIST in \
         cli/tests/spawn_site_guard.rs with a one-line reason.\nEscapes:\n{}",
        result.unlisted.join("\n")
    );
    assert!(
        result.stale.is_empty(),
        "Stale ISOLATION_ALLOWLIST entries name files with no live escape left. Delete \
         them.\nStale entries:\n{}",
        result.stale.join("\n")
    );
    assert!(
        result.unexplained.is_empty(),
        "ISOLATION_ALLOWLIST entries without a reason:\n{}",
        result.unexplained.join("\n")
    );
}

#[test]
fn detector_finds_every_spawn_form_as_executable_code() {
    assert_eq!(
        spawn_sites("let mut c = assert_cmd::Command::cargo_bin(\"claudine\").unwrap();\n"),
        [(1, FORM_CARGO_BIN)]
    );
    assert_eq!(
        spawn_sites("let bin = common::claudine_bin();\n"),
        [(1, FORM_CLAUDINE_BIN)]
    );
    assert_eq!(
        spawn_sites("let bin = biscuit_test_harness::bin_exe!(\"claudine\");\n"),
        [(1, FORM_BIN_EXE)]
    );
    // Whitespace between the callee and its argument list still opens a call.
    assert_eq!(
        spawn_sites("Command::cargo_bin (\n    \"claudine\",\n).unwrap()\n"),
        [(1, FORM_CARGO_BIN)]
    );
    assert_eq!(
        spawn_sites("bin_exe ! ( \"claudine\" )\n"),
        [(1, FORM_BIN_EXE)]
    );
    // Line attribution points at the site, not at the file start.
    assert_eq!(
        spawn_sites("fn a() {}\nfn b() {}\nlet c = Command::cargo_bin(\"claudine\");\n"),
        [(3, FORM_CARGO_BIN)]
    );
}

#[test]
fn detector_ignores_prose_neighbors_and_other_binaries() {
    // This guard's own docs, and every migrated file's explanatory comment,
    // name the forbidden call. Only executable use counts.
    assert!(
        spawn_sites("//! A raw `Command::cargo_bin(\"claudine\")` inherits the runner env.\n")
            .is_empty()
    );
    assert!(spawn_sites("/* Command::cargo_bin(\"claudine\") */\n").is_empty());
    assert!(spawn_sites("let doc = r#\"Command::cargo_bin(\"claudine\")\"#;\n").is_empty());
    assert!(spawn_sites("let s = \"claudine_bin()\";\n").is_empty());
    assert!(
        spawn_sites("//! Obtained with `bin_exe!(\"claudine\")` and spawned raw.\n").is_empty()
    );
    assert!(spawn_sites("let s = \"bin_exe!(\\\"claudine\\\")\";\n").is_empty());

    // Other binaries spawn legitimately — darkmatter's `md` shim, most of all.
    assert!(spawn_sites("Command::cargo_bin(\"md\").unwrap();\n").is_empty());
    assert!(spawn_sites("biscuit_test_harness::bin_exe!(\"md\");\n").is_empty());
    // A bare mention of the helper is not a shell-out.
    assert!(spawn_sites("use common::claudine_bin;\n").is_empty());
    // Identifier boundaries hold on both sides.
    assert!(spawn_sites("let x = my_claudine_bin();\n").is_empty());
    assert!(spawn_sites("Command::cargo_bin_of(\"claudine\");\n").is_empty());
    assert!(spawn_sites("let p = harness_bin_exe!(\"claudine\");\n").is_empty());
    assert!(spawn_sites("let p = bin_exe_path!(\"claudine\");\n").is_empty());
    // The macro form needs its `!`: a plain call of that name is something else.
    assert!(spawn_sites("let p = bin_exe(\"claudine\");\n").is_empty());
}

#[test]
fn isolation_detector_finds_every_escape_as_executable_code() {
    assert_eq!(
        isolation_sites("command.current_dir(repository_root());\n"),
        [(1, FORM_CURRENT_DIR)]
    );
    assert_eq!(
        isolation_sites("command.env(\"PATH\", augmented_path(&dir));\n"),
        [(1, FORM_PATH_ENV), (1, FORM_AUGMENTED_PATH)]
    );
    assert_eq!(
        isolation_sites("command.env_remove(\"PATH\");\n"),
        [(1, FORM_PATH_ENV_REMOVE)]
    );
    assert_eq!(
        isolation_sites("command.env_clear();\n"),
        [(1, FORM_ENV_CLEAR)]
    );
    assert_eq!(
        isolation_sites("use common::augmented_path;\n"),
        [(1, FORM_AUGMENTED_PATH)]
    );
    // The chained form, which is how a call site would actually write it.
    assert_eq!(
        isolation_sites("fixture\n    .command()\n    .current_dir(root)\n    .assert();\n"),
        [(3, FORM_CURRENT_DIR)]
    );
    // Whitespace around the dot and before the argument list still reads as a
    // method call.
    assert_eq!(
        isolation_sites("command . env (\n    \"PATH\",\n    value,\n);\n"),
        [(1, FORM_PATH_ENV)]
    );
}

#[test]
fn isolation_detector_ignores_prose_neighbors_and_path_qualified_lookalikes() {
    // This guard's own docs, and `cli_process_fixture.rs`'s, name every form.
    assert!(isolation_sites("//! A `.current_dir(root)` undoes the pin.\n").is_empty());
    assert!(isolation_sites("/// survives its own `env_clear()`\n").is_empty());
    assert!(isolation_sites("/* command.env(\"PATH\", augmented_path(&d)) */\n").is_empty());
    assert!(isolation_sites("let doc = \"command.env_clear()\";\n").is_empty());
    assert!(isolation_sites("let doc = r#\"cmd.env(\"PATH\", x)\"#;\n").is_empty());
    assert!(isolation_sites("let doc = \"see augmented_path\";\n").is_empty());

    // Path-qualified lookalikes are not method calls on a built command.
    assert!(isolation_sites("let here = std::env::current_dir().unwrap();\n").is_empty());
    assert!(isolation_sites("let path = std::env::var(\"PATH\").unwrap();\n").is_empty());
    assert!(isolation_sites("let path = env::var_os(\"PATH\");\n").is_empty());
    // An index, not a call.
    assert!(isolation_sites("let value = &recorded[\"PATH\"];\n").is_empty());
    // `.env` naming some other variable is the ordinary, supported form.
    assert!(isolation_sites("command.env(\"HOME\", home);\n").is_empty());
    assert!(isolation_sites("command.env_remove(\"HOMEDRIVE\");\n").is_empty());

    // Identifier boundaries hold on both sides.
    assert!(isolation_sites("command.set_current_dir(root);\n").is_empty());
    assert!(isolation_sites("command.current_dir_of(root);\n").is_empty());
    assert!(isolation_sites("command.env_clear_all();\n").is_empty());
    assert!(isolation_sites("let p = host_augmented_path(&dir);\n").is_empty());
    assert!(isolation_sites("let p = augmented_path_of(&dir);\n").is_empty());
    // A bare mention with no call is not an escape.
    assert!(isolation_sites("command.current_dir;\n").is_empty());
}

#[test]
fn a_file_that_never_builds_a_fixture_command_is_outside_the_isolation_gate() {
    assert!(obtains_a_fixture_command(
        "let fixture = CliProcessFixture::named(\"x\");\n"
    ));
    assert!(obtains_a_fixture_command("use common::CliProcessFixture;\n"));
    // Prose naming the fixture must not enrol a file that holds none — this
    // guard's own module docs and assertion messages do exactly that.
    assert!(!obtains_a_fixture_command(
        "//! `CliProcessFixture`'s builder is the one supported spawn.\n"
    ));
    assert!(!obtains_a_fixture_command(
        "panic!(\"build it with CliProcessFixture::command()\");\n"
    ));
    // Identifier boundaries hold.
    assert!(!obtains_a_fixture_command("struct MyCliProcessFixture;\n"));
    assert!(!obtains_a_fixture_command(
        "let x = CliProcessFixtureBuilder::new();\n"
    ));
}

#[test]
fn tier_naming_decides_what_the_guard_governs() {
    assert!(excluded("level2_perf_capture.rs"));
    assert!(excluded("level3_wrap_ctrl_c.rs"));
    assert!(excluded("real_opencode_yolo_subagent.rs"));
    assert!(excluded("common/mod.rs"));
    assert!(excluded("common/wrap.rs"));
    // `level1_*` files are ordinary L1 binaries and stay governed.
    assert!(!excluded("level1_structured_error_message.rs"));
    assert!(!excluded("wrap_basics.rs"));
    assert!(!excluded("sequence_overlay_pty.rs"));
}

#[test]
fn reconciliation_reports_unlisted_sites_stale_and_unexplained_entries() {
    let sites = vec![
        Site {
            file: "allowed.rs".into(),
            line: 12,
            form: FORM_CARGO_BIN,
        },
        Site {
            file: "allowed.rs".into(),
            line: 30,
            form: FORM_CLAUDINE_BIN,
        },
        Site {
            file: "migrated_but_regressed.rs".into(),
            line: 7,
            form: FORM_CARGO_BIN,
        },
    ];
    let result = reconcile(
        &sites,
        &[
            AllowEntry {
                file: "allowed.rs",
                reason: "a reason, any reason",
            },
            AllowEntry {
                file: "already_migrated.rs",
                reason: OUT_OF_SCOPE,
            },
            AllowEntry {
                file: "allowed.rs",
                reason: "   ",
            },
        ],
    );

    assert_eq!(result.unlisted, ["migrated_but_regressed.rs:7 cargo_bin"]);
    assert_eq!(result.stale, ["already_migrated.rs"]);
    assert_eq!(result.unexplained, ["allowed.rs"]);
    assert_eq!(result.burn_down, BTreeMap::from([("allowed.rs".into(), 2)]));
}

/// A synthetic two-file census, shared by the artifact tests.
fn sample_burn_down() -> BurnDown {
    const ALLOWLIST: &[AllowEntry] = &[
        AllowEntry {
            file: "allowed.rs",
            reason: OUT_OF_SCOPE,
        },
        AllowEntry {
            file: "windows_ctrl_c.rs",
            reason: NEEDS_LIVE_CHILD,
        },
        AllowEntry {
            file: "already_migrated.rs",
            reason: OUT_OF_SCOPE,
        },
    ];
    let sites = vec![
        Site {
            file: "allowed.rs".into(),
            line: 12,
            form: FORM_CARGO_BIN,
        },
        Site {
            file: "allowed.rs".into(),
            line: 30,
            form: FORM_CLAUDINE_BIN,
        },
        Site {
            file: "windows_ctrl_c.rs".into(),
            line: 7,
            form: FORM_BIN_EXE,
        },
    ];
    let result = reconcile(&sites, ALLOWLIST);
    BurnDown::new(GATE_SPAWN, 9, sites.len(), ALLOWLIST, &result)
}

#[test]
fn the_artifact_carries_per_file_counts_the_reason_rollup_and_the_totals() {
    let expected = format!(
        concat!(
            r#"{{"kind":"file","gate":"spawn","file":"allowed.rs","reason":"{out}","sites":2}}"#,
            "\n",
            r#"{{"kind":"file","gate":"spawn","file":"windows_ctrl_c.rs","reason":"{live}","sites":1}}"#,
            "\n",
            r#"{{"kind":"reason","gate":"spawn","reason":"{live}","files":1,"sites":1}}"#,
            "\n",
            r#"{{"kind":"reason","gate":"spawn","reason":"{out}","files":1,"sites":2}}"#,
            "\n",
            r#"{{"kind":"total","gate":"spawn","files":2,"sites":3,"scanned_sites":3,"governed_files":9}}"#,
            "\n",
        ),
        out = OUT_OF_SCOPE,
        live = NEEDS_LIVE_CHILD,
    );

    assert_eq!(sample_burn_down().to_jsonl(), expected);
}

#[test]
fn the_artifact_is_written_even_when_the_staging_directory_does_not_exist() {
    let stage = tempfile::tempdir().expect("temp dir");
    // Two levels deep and never created: the local `cargo nextest` case, where
    // nothing has staged a report yet.
    let path = stage.path().join("nextest/ci-reports").join(SPAWN_REPORT_FILE);
    let burn_down = sample_burn_down();

    emit_report_to(&path, &burn_down);

    assert_eq!(
        fs::read_to_string(&path).expect("artifact written"),
        burn_down.to_jsonl()
    );
}

#[test]
fn a_second_run_replaces_the_previous_census_instead_of_appending() {
    let stage = tempfile::tempdir().expect("temp dir");
    let path = stage.path().join(SPAWN_REPORT_FILE);
    let burn_down = sample_burn_down();

    emit_report_to(&path, &burn_down);
    emit_report_to(&path, &burn_down);

    assert_eq!(
        fs::read_to_string(&path).expect("artifact written"),
        burn_down.to_jsonl()
    );
}

#[test]
fn an_unwritable_destination_warns_instead_of_failing_the_guard() {
    let stage = tempfile::tempdir().expect("temp dir");
    // A regular file where the artifact's parent directory would go, so
    // `create_dir_all` cannot succeed on any platform.
    let blocker = stage.path().join("occupied");
    fs::write(&blocker, "not a directory").expect("blocker written");
    let path = blocker.join(SPAWN_REPORT_FILE);

    assert!(write_report(&path, "irrelevant").is_err());
    // The guard's own contract: telemetry failure is a warning, never a panic.
    emit_report_to(&path, &sample_burn_down());
}

#[test]
fn an_empty_allowlist_leaves_every_live_site_unlisted() {
    // Non-vacuity: the guard's population is real, so deleting the allow-list
    // wholesale must surface it rather than pass on an empty scan.
    let sites = scan(&tests_root(), governs_spawn, spawn_sites).expect("spawn-site scan");
    assert!(
        !sites.is_empty(),
        "the scan found no spawn sites at all — the detector or the scan root is wrong"
    );
    let result = reconcile(&sites, &[]);
    assert_eq!(result.unlisted.len(), sites.len());
    assert!(result.burn_down.is_empty());
}

/// Non-vacuity for the isolation gate. It reports zero escapes, which is
/// indistinguishable from a gate that reads no files at all — so the population
/// itself is what gets asserted.
#[test]
fn the_isolation_gate_governs_the_migrated_files_and_only_those() {
    let governed: BTreeSet<String> = governed_files(&tests_root(), governs_isolation)
        .expect("isolation population")
        .into_iter()
        .map(|(file, _)| file)
        .collect();

    assert!(
        governed.len() > 20,
        "the isolation gate governs only {} file(s); the migrated population is ~30, so the \
         scan root or the predicate is wrong: {governed:?}",
        governed.len()
    );
    for migrated in [
        "wrap_basics.rs",
        "cli_process_fixture.rs",
        "ctx_launch_anchor.rs",
        "propagated_context_fixtures.rs",
    ] {
        assert!(
            governed.contains(migrated),
            "{migrated} is a migrated L1 file and must be governed: {governed:?}"
        );
    }
    // Still on the spawn burn-down: it sets its own environment by hand.
    assert!(!governed.contains("sequence_cli.rs"));
    // Never obtains a fixture command; its `.current_dir` pins a `git` child.
    assert!(!governed.contains("system_prompt_perf_bench.rs"));
    // The tier and `common/` exclusions still apply.
    assert!(!governed.contains("level2_context_capture.rs"));
    assert!(!governed.contains("common/mod.rs"));
    // This guard names the fixture in prose only.
    assert!(!governed.contains("spawn_site_guard.rs"));
}
