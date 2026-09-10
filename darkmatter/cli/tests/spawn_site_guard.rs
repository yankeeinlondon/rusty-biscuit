//! Two structural gates over the L1 test binaries: the `md` process comes
//! from the fixture builder, and the isolation the builder gave it survives
//! `build()`.
//!
//! ## The spawn gate
//!
//! A raw `assert_cmd::Command::cargo_bin("md")` — or a call to `md_cmd()`
//! (the shared `common::md_cmd`, or one of the private per-file
//! redefinitions), or a `bin_exe!("md")` handed to `std::process::Command` —
//! inherits the runner's environment: the rusty-biscuit checkout as the launch
//! context (the whole monorepo is an ancestor of the package dir nextest
//! launches from), the host `PATH` with every installed tool on it, and the
//! developer's `$HOME`. That is what makes the slow tests slow and the CI legs
//! divergent, and no per-test `.env(...)` chain can be relied on to have
//! covered all of it. `CliProcessFixture`'s builder is the one supported
//! spawn; this guard keeps the alternative from regrowing.
//!
//! [`SPAWN_ALLOWLIST`] was a burn-down list, not a permanent exemption table:
//! every entry named a file and stated which Phase 6 batch would migrate it.
//! An entry that matches no live site fails the guard, so a migration cannot
//! leave the list stale, and a file with a live site but no entry fails it
//! too. The list was seeded with all **38** files that carried the population
//! when the guard landed: 502 `md_cmd()` call sites across 37 files plus 2
//! inline `cargo_bin("md")` spawns in `schema_validate_baseline.rs`, with the
//! seven private `fn md_cmd()` redefinitions adding one `md_cmd` site for
//! their definition line and one `cargo_bin` site for their body each —
//! **518 sites across 38 files** in the detector's census. Phase 6A/6B/6C
//! deleted every entry; no generic migration exemption remains.
//!
//! The former shared `common::md_cmd()` lived in `common/`, which both gates
//! exclude (that is where the builder lives). It was deleted when the final
//! caller migrated; `scanned_sites` reaching zero in the census artifact is
//! the signal.
//!
//! ### The sanctioned forms
//!
//! There are two, and the detector recognizes neither as a site because
//! neither names a binary: `CliProcessFixture::command()` for a run to
//! completion, and `command_std()` / `command_builder().build_std()` for a
//! test that has to keep the child. The raw path carries the same environment
//! policy as the `assert_cmd` one (`common/fixture.rs` § "Two command
//! surfaces, one policy"), which is what makes it sanctioned; a
//! `std::process::Command::new(bin_exe!("md"))` written out by hand carries
//! none of it and stays a violation.
//!
//! ## The isolation gate
//!
//! `build()` hands back a bare `assert_cmd::Command`, so the supported spawn
//! form is not by itself the contract: a call site can write
//! `.current_dir(repository_root())` or `.env("PATH", host_path)` on the
//! returned command and reinstate both leaks the migration removed, while the
//! spawn gate above stays perfectly happy. The second gate scans for those
//! post-`build()` escapes and reconciles them against [`ISOLATION_ALLOWLIST`]
//! on the same mechanics.
//!
//! `PATH` and the launch directory carry their own four forms —
//! `.current_dir(…)`, `.env("PATH", …)`, `.env_remove("PATH")`, and
//! `.env_clear()`. (The claudine reference has a fifth, a reach for its
//! `augmented_path` helper; the darkmatter builder has no equivalent, because
//! `PATH` composition is internal to `MdCommandBuilder`.) Every *other*
//! variable the contract pins is classified by `common/protected_env.rs` —
//! the same table the builder validates a declaration against — and a
//! post-`build()` `.env`/`.env_remove` naming one reports under its class:
//! home/config/cache/temp containment, Git plumbing containment, a rendering
//! input, or a darkmatter application input. Without that widening a plain
//! `.env("HOME", host_home)` or `.env("GIT_DIR", checkout)` restored exactly
//! the contamination the fixture exists to prevent with both gates green.
//!
//! The two classes are not the same defect and the failure message says so.
//! Containment has no legitimate post-`build()` form at all. A behavior input
//! usually does have a legitimate reason behind it — the test's subject *is*
//! that input — and the fix is to declare it on the builder
//! (`rendering_input`, `application_input`, `plain_terminal`) so the claim is
//! visible at construction beside `ambient_context`, `host_path`,
//! `fake_only_path`, and `inherit_no_env`.
//!
//! The allow-list carries exactly one entry: `md_process_fixture.rs`, whose
//! `.current_dir` sites pin **parent-side `git()` helper commands**
//! (hostile-state construction and config readback), not builder-built `md`
//! commands. The escapes are spelled on the builder, not on the command.
//!
//! The scan is textual, so it reads a raw `std::process::Command` from
//! `build_std()` exactly as it reads an `assert_cmd::Command` from `build()`:
//! the same forms, the same allow-list. What the raw surface adds —
//! `.spawn()`, `.stdout(…)`, a `Child` held across a deadline — is not an
//! escape and is not flagged, because keeping the child is the reason that
//! surface exists.
//!
//! ## What each gate governs
//!
//! `level2_*`, `level3_*`, `browser_*`, `real_*`, and `slow_*` files are out
//! of both scans — the same prefixes the area's `_tier_filter` drops from L1.
//! They drive real terminals and real host tooling on purpose, so the
//! hermetic default is the wrong contract for them (`common/level2.rs`'s
//! `md_bin()` is a `bin_exe!("md")` and is deliberately not this gate's
//! business). `common/` is excluded because that is where the builder lives.
//!
//! The isolation gate narrows that population twice more:
//!
//! - [`SPAWN_ALLOWLIST`] files are out. The list is empty after Phase 6, but
//!   keeping this relationship explicit makes any future technical exemption
//!   narrow both gates in one reviewed place.
//! - A file that never names `CliProcessFixture` in executable code is out.
//!   It holds no builder-produced command, so it has no isolation to defeat.
//!
//! The residual blind spot is the mirror of that last rule: a `.current_dir`
//! on a non-`md` command *inside* a governed file reads the same as one on a
//! fixture command, because the scan is textual and does not resolve
//! receivers. `md_process_fixture.rs` is today's one genuine case, and its
//! allow-list entry says which commands its sites target. The file that
//! acquires another takes an entry saying the same.
//!
//! ## Guard scope beyond this crate
//!
//! The guard governs `darkmatter/cli/tests` only. `dmls`'s single raw spawn
//! (`stdio_subprocess.rs`'s `bin_exe!("dmls")` — a live-child LSP stdio
//! session, Phase 8's remediation) and `zed-dmls-cli`'s six
//! `Command::cargo_bin("zed-dmls")` spawns (Phase 6D's disposition) are
//! recorded in `inventory.md` § Guard scope with why they are governed
//! differently; this file's detectors name `md` and nobody else.
//!
//! ## Reading the burn-down
//!
//! Each gate prints its census on stderr and writes the same census to a JSON
//! Lines artifact in the staging directory — `$BISCUIT_JUNIT_STAGE_DIR`, else
//! `target/nextest/ci-reports`, resolved by `test_toolkit::stage_dir`. The
//! spawn gate writes [`SPAWN_REPORT_FILE`], the isolation gate
//! [`ISOLATION_REPORT_FILE`]. The `md-` prefix keeps them distinct from the
//! claudine guard's identically-shaped artifacts in a shared stage directory.
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
//! $ cat target/nextest/ci-reports/md-spawn-site-burn-down.jsonl
//! {"kind":"total","gate":"spawn","files":38,"sites":518,"scanned_sites":518,"governed_files":41}
//! ```

// Included directly rather than through `mod common;`: this binary needs the
// sanitizer and nothing else, and `common/mod.rs` drags in the fixture's
// consumers (`mock_http_server`'s listener threads, the `baseline`/`layout`
// helpers) that a text scanner has no use for.
#[path = "common/source_scan.rs"]
mod source_scan;
// The classification the fixture builder validates declarations against, so
// the two cannot disagree about what "protected" means.
#[path = "common/protected_env.rs"]
mod protected_env;

use protected_env::{ProtectedClass, protected_class};
use source_scan::{is_ident, line_at, sanitize};

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// `assert_cmd::Command::cargo_bin("md")` — the raw spawn, including the
/// bodies of private `fn md_cmd()` redefinitions.
const FORM_CARGO_BIN: &str = "cargo_bin";
/// A call to `md_cmd()` — the shared `common::md_cmd` at a call site, or a
/// private redefinition's own `fn md_cmd()` line, which opens a call
/// syntactically. This is the 502-site bulk of the population.
const FORM_MD_CMD: &str = "md_cmd";
/// `bin_exe!("md")` fed to a raw `Command::new`. Zero live L1 sites today
/// (`common/level2.rs`'s `md_bin()` is the excluded-tier user); the form
/// exists so the leak cannot open silently.
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

/// `.env`/`.env_remove` naming a home, config, cache, or temp anchor — the
/// accident that hands the child the developer's own directories back.
const FORM_HOME_CONTAINMENT: &str = ".env/.env_remove of a home/config/cache/temp anchor";
/// `.env`/`.env_remove` naming Git plumbing, which overrides cwd-based
/// repository discovery and so defeats the pinned launch directory.
const FORM_GIT_CONTAINMENT: &str = ".env/.env_remove of a Git plumbing variable";
/// `.env`/`.env_remove` of a rendering input that was never declared on the
/// builder — the claim is real, but it belongs at construction.
const FORM_RENDERING_INPUT: &str = ".env/.env_remove of an undeclared rendering input";
/// `.env`/`.env_remove` of an undeclared darkmatter application input.
const FORM_APPLICATION_INPUT: &str = ".env/.env_remove of an undeclared application input";

/// The form a protected key's post-`build()` override reports as.
fn form_for(class: ProtectedClass) -> &'static str {
    match class {
        ProtectedClass::HomeContainment => FORM_HOME_CONTAINMENT,
        ProtectedClass::GitContainment => FORM_GIT_CONTAINMENT,
        ProtectedClass::RenderingInput => FORM_RENDERING_INPUT,
        ProtectedClass::ApplicationInput => FORM_APPLICATION_INPUT,
    }
}

/// File-name prefixes whose spawn contract is deliberately not hermetic — the
/// same prefixes the area's `_tier_filter` drops from the L1 population.
const EXCLUDED_PREFIXES: &[&str] = &["level2_", "level3_", "browser_", "real_", "slow_"];

/// One flagged site: a raw spawn, or a post-`build()` isolation escape.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    /// Path relative to `darkmatter/cli/tests`, `/`-separated.
    file: String,
    line: usize,
    form: &'static str,
}

/// One file a gate still exempts, with the reason it is exempt.
struct AllowEntry {
    /// Path relative to `darkmatter/cli/tests`, `/`-separated.
    file: &'static str,
    reason: &'static str,
}

/// Phase 6A migrates the seven private `fn md_cmd()` redefinitions plus the
/// two orphan inline spawns the spec's evidence missed.
const PENDING_6A: &str =
    "awaiting Phase 6A migration (private md_cmd() helpers + orphan inline spawns)";
/// Phase 6B migrates the clean / hash / frontmatter family.
const PENDING_6B: &str = "awaiting Phase 6B migration (clean/hash/frontmatter family)";
/// The Phase 6 closure state: no generic migration exemption remains.
const SPAWN_ALLOWLIST: &[AllowEntry] = &[];

/// Files allowed to undo the builder's isolation after `build()`.
///
/// Expected to stay at one entry: every legitimate need is a named builder
/// method, so an entry here is a claim that the site targets a command the
/// fixture did not build. The mechanics are [`SPAWN_ALLOWLIST`]'s — a reason
/// is mandatory, and an entry naming a file with no live escape fails the
/// guard as stale.
const ISOLATION_ALLOWLIST: &[AllowEntry] = &[AllowEntry {
    file: "md_process_fixture.rs",
    reason: "`.current_dir` sites pin parent-side `git()` helper commands (hostile-state construction, config readback), not builder-built `md` commands",
}];

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
/// it names — and `cargo_bin("claudine")` is a legitimate claudine-guard
/// shape that must not enrol a darkmatter file.
fn names_literal(source: &[u8], after_name: usize, literal: &str) -> bool {
    let cursor = skip_whitespace(source, after_name);
    if source.get(cursor) != Some(&b'(') {
        return false;
    }
    let cursor = skip_whitespace(source, cursor + 1);
    let quoted = format!("\"{literal}\"");
    source[cursor..].starts_with(quoted.as_bytes())
}

fn names_md(source: &[u8], after_name: usize) -> bool {
    names_literal(source, after_name, "md")
}

/// The first argument of the call opening at `after_name`, when it is a plain
/// string literal.
///
/// Read from the *original* source for the same reason [`names_literal`] is:
/// [`sanitize`] blanks the literal, so only the original says which variable a
/// site names. A literal carrying an escape is rejected rather than decoded —
/// no environment variable this contract owns is spelled with one, and a
/// half-decoded key would classify wrongly.
fn first_string_argument(source: &[u8], after_name: usize) -> Option<String> {
    let cursor = skip_whitespace(source, after_name);
    if source.get(cursor) != Some(&b'(') {
        return None;
    }
    let start = skip_whitespace(source, cursor + 1) + 1;
    if source.get(start - 1) != Some(&b'"') {
        return None;
    }
    let end = start + source[start..].iter().position(|&byte| byte == b'"')?;
    let literal = std::str::from_utf8(&source[start..end]).ok()?;
    (!literal.contains('\\')).then(|| literal.to_string())
}

/// The protected key a `.env`/`.env_remove` call names, if it names one.
fn protected_argument(source: &[u8], after_name: usize) -> Option<ProtectedClass> {
    protected_class(&first_string_argument(source, after_name)?)
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
/// literal `"md"` — `bin_exe!("md")`.
///
/// Only the `!` separates this from the `cargo_bin` arm's shape, so it steps
/// over the bang and reuses [`names_md`] for the argument itself.
fn macro_names_md(source: &[u8], after_name: usize) -> bool {
    let cursor = skip_whitespace(source, after_name);
    source.get(cursor) == Some(&b'!') && names_md(source, cursor + 1)
}

/// Every raw `md` spawn site in `source`, as `(line, form)` pairs.
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
            b"cargo_bin" if names_md(bytes, end) => {
                sites.push((line_at(source, index), FORM_CARGO_BIN));
            }
            b"md_cmd" if opens_a_call(&code, end) => {
                sites.push((line_at(source, index), FORM_MD_CMD));
            }
            b"bin_exe" if macro_names_md(bytes, end) => {
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
/// Every form is recognized in method position only. `PATH` keeps its two
/// dedicated forms; every other protected key reports under its class, so a
/// failure says which contract the site broke.
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
            b"env" | b"env_remove" if method => {
                if let Some(class) = protected_argument(bytes, end) {
                    sites.push((line_at(source, index), form_for(class)));
                }
            }
            b"env_clear" if method && opens_a_call(&code, end) => {
                sites.push((line_at(source, index), FORM_ENV_CLEAR));
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

/// Whether `relative` is named by `allowlist`.
fn listed(relative: &str, allowlist: &[AllowEntry]) -> bool {
    allowlist.iter().any(|entry| entry.file == relative)
}

/// Whether a file is in the isolation gate's population — the migrated L1
/// files that actually hold a builder-produced command. See the module docs
/// for why each narrowing is there.
///
/// Parameterized by the spawn allow-list so the widening rule stays testable
/// after [`SPAWN_ALLOWLIST`] empties: an assertion written against the live
/// list can only say "no file is exempt", which is the same thing a broken
/// predicate says.
fn governs_isolation_against(relative: &str, source: &str, spawn_allowlist: &[AllowEntry]) -> bool {
    !excluded(relative) && !listed(relative, spawn_allowlist) && obtains_a_fixture_command(source)
}

fn governs_isolation(relative: &str, source: &str) -> bool {
    governs_isolation_against(relative, source, SPAWN_ALLOWLIST)
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
/// Split out of the gates so a gate that also needs the population size does
/// not have to read every file twice to get it.
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
const SPAWN_REPORT_FILE: &str = "md-spawn-site-burn-down.jsonl";
/// The isolation gate's roll-up, inside the staging directory.
const ISOLATION_REPORT_FILE: &str = "md-isolation-burn-down.jsonl";

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

// ---------------------------------------------------------------------------
// The gates
// ---------------------------------------------------------------------------

/// The guard proper: raw `md` spawns exist only where the allow-list says so,
/// and every allow-list entry still describes a live site.
#[test]
fn l1_tests_spawn_md_through_the_fixture_builder() {
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
        "Raw `md` spawns outside the fixture builder. These inherit the runner's \
         launch context, host PATH, and $HOME. Build the command with \
         `CliProcessFixture::command()` (or `command_builder()` plus a named escape), or — \
         if the file genuinely cannot — give it a SPAWN_ALLOWLIST entry in \
         cli/tests/spawn_site_guard.rs naming the specific technical necessity.\n\
         Unlisted sites:\n{}",
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
         `assert_cmd::Command`, so these silently reinstate host state the fixture removed. \
         Say it on the builder instead — `ambient_context(dir)` for the launch CWD, \
         `host_path()` or `fake_only_path()` for PATH, `inherit_no_env()` for a cleared \
         environment, `rendering_input(…)`/`plain_terminal(…)` for a rendering claim, \
         `application_input(…)` for a darkmatter one. A home/config/cache/temp anchor or a \
         Git plumbing variable has no declared form at all: it is containment, and handing \
         it back re-contaminates the child. Or, if the site targets a command the fixture \
         did not build, give the file an ISOLATION_ALLOWLIST entry in \
         cli/tests/spawn_site_guard.rs saying which command it targets.\nEscapes:\n{}",
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

// ---------------------------------------------------------------------------
// Detector unit tests
// ---------------------------------------------------------------------------

#[test]
fn detector_finds_every_spawn_form_as_executable_code() {
    assert_eq!(
        spawn_sites("let mut c = assert_cmd::Command::cargo_bin(\"md\").unwrap();\n"),
        [(1, FORM_CARGO_BIN)]
    );
    assert_eq!(spawn_sites("let mut c = md_cmd();\n"), [(1, FORM_MD_CMD)]);
    // A private redefinition's own `fn` line opens a call syntactically…
    assert_eq!(
        spawn_sites("fn md_cmd() -> assert_cmd::Command {\n"),
        [(1, FORM_MD_CMD)]
    );
    // …and its body is a cargo_bin site on top of it.
    assert_eq!(
        spawn_sites(
            "fn md_cmd() -> assert_cmd::Command {\n    assert_cmd::Command::cargo_bin(\"md\").unwrap()\n}\n"
        ),
        [(1, FORM_MD_CMD), (2, FORM_CARGO_BIN)]
    );
    assert_eq!(
        spawn_sites("let bin = biscuit_test_harness::bin_exe!(\"md\");\n"),
        [(1, FORM_BIN_EXE)]
    );
    // The shared helper through its crate path is the same site.
    assert_eq!(
        spawn_sites("let mut c = common::md_cmd();\n"),
        [(1, FORM_MD_CMD)]
    );
    // Whitespace between the callee and its argument list still opens a call.
    assert_eq!(
        spawn_sites("Command::cargo_bin (\n    \"md\",\n).unwrap()\n"),
        [(1, FORM_CARGO_BIN)]
    );
    assert_eq!(spawn_sites("bin_exe ! ( \"md\" )\n"), [(1, FORM_BIN_EXE)]);
    assert_eq!(spawn_sites("md_cmd (\n)\n"), [(1, FORM_MD_CMD)]);
    // Line attribution points at the site, not at the file start.
    assert_eq!(
        spawn_sites("fn a() {}\nfn b() {}\nlet c = md_cmd();\n"),
        [(3, FORM_MD_CMD)]
    );
}

#[test]
fn detector_ignores_prose_neighbors_and_other_binaries() {
    // This guard's own docs, and every migrated file's explanatory comment,
    // name the forbidden call. Only executable use counts.
    assert!(
        spawn_sites("//! A raw `Command::cargo_bin(\"md\")` inherits the runner env.\n").is_empty()
    );
    assert!(spawn_sites("/* Command::cargo_bin(\"md\") */\n").is_empty());
    assert!(spawn_sites("let doc = r#\"Command::cargo_bin(\"md\")\"#;\n").is_empty());
    assert!(spawn_sites("//! `md_cmd()` remains for the binaries Phase 6 migrates.\n").is_empty());
    assert!(spawn_sites("let s = \"md_cmd()\";\n").is_empty());
    assert!(spawn_sites("//! Obtained with `bin_exe!(\"md\")` and spawned raw.\n").is_empty());
    assert!(spawn_sites("let s = \"bin_exe!(\\\"md\\\")\";\n").is_empty());

    // Other binaries spawn legitimately — claudine's guard catches its own,
    // and dmls/zed-dmls-cli are other packages with their own contracts.
    assert!(spawn_sites("Command::cargo_bin(\"claudine\").unwrap();\n").is_empty());
    assert!(spawn_sites("Command::cargo_bin(\"zed-dmls\").unwrap();\n").is_empty());
    assert!(spawn_sites("bin_exe!(\"dmls\");\n").is_empty());
    assert!(spawn_sites("Command::new(bin_exe!(\"dmls\")).spawn();\n").is_empty());
    // A bare import of the helper is not a shell-out; the call is the site.
    assert!(spawn_sites("use common::md_cmd;\n").is_empty());
    // Identifier boundaries hold on both sides.
    assert!(spawn_sites("let x = my_md_cmd();\n").is_empty());
    assert!(spawn_sites("let x = md_cmd_of(\"md\");\n").is_empty());
    assert!(spawn_sites("Command::cargo_bin_of(\"md\");\n").is_empty());
    assert!(spawn_sites("let p = harness_bin_exe!(\"md\");\n").is_empty());
    assert!(spawn_sites("let p = bin_exe_path!(\"md\");\n").is_empty());
    // The macro form needs its `!`: a plain call of that name is something else.
    assert!(spawn_sites("let p = bin_exe(\"md\");\n").is_empty());
}

/// The builder's raw-command path: sanctioned where it comes from the builder,
/// a violation where it is hand-rolled.
#[test]
fn detector_treats_the_builders_raw_command_path_as_a_sanctioned_form() {
    // Sanctioned: neither spelling names a binary, so neither can inherit the
    // runner's launch context. The policy came from the builder.
    assert!(spawn_sites("let mut child = fixture.command_std().spawn().unwrap();\n").is_empty());
    assert!(
        spawn_sites("let command = fixture.command_builder().host_path().build_std();\n")
            .is_empty()
    );

    // Still a violation: reaching for the binary directly is what the raw path
    // replaces, and `.spawn()` does not launder it.
    assert_eq!(
        spawn_sites("Command::new(biscuit_test_harness::bin_exe!(\"md\")).spawn();\n"),
        [(1, FORM_BIN_EXE)]
    );
    assert_eq!(
        spawn_sites("let mut c = assert_cmd::Command::cargo_bin(\"md\").unwrap();\n"),
        [(1, FORM_CARGO_BIN)]
    );

    // Negatives: another binary, prose, and a string literal naming the form.
    assert!(spawn_sites("Command::new(bin_exe!(\"dmls\")).spawn();\n").is_empty());
    assert!(
        spawn_sites("//! Prefer `command_std()` to `Command::new(bin_exe!(\"md\"))`.\n").is_empty()
    );
    assert!(spawn_sites("let doc = \"Command::new(bin_exe!(\\\"md\\\"))\";\n").is_empty());
}

/// The isolation gate reads a raw fixture command exactly as it reads an
/// `assert_cmd` one — and does not mistake the raw surface's own methods for
/// escapes.
#[test]
fn isolation_detector_reads_a_raw_fixture_command_like_an_assert_cmd_one() {
    assert_eq!(
        isolation_sites("let mut child = fixture.command_std();\nchild.current_dir(root);\n"),
        [(2, FORM_CURRENT_DIR)]
    );
    assert_eq!(
        isolation_sites("fixture.command_std().env_remove(\"PATH\");\n"),
        [(1, FORM_PATH_ENV_REMOVE)]
    );
    assert_eq!(
        isolation_sites("let mut c = fixture.command_builder().build_std();\nc.env_clear();\n"),
        [(2, FORM_ENV_CLEAR)]
    );

    // Keeping the child is the raw surface's purpose, not an escape from the
    // isolation the builder gave it.
    assert!(
        isolation_sites("let child = fixture.command_std().stdout(Stdio::piped()).spawn()?;\n")
            .is_empty()
    );
    assert!(isolation_sites("let output = child.wait_with_output()?;\n").is_empty());
}

#[test]
fn isolation_detector_finds_every_escape_as_executable_code() {
    assert_eq!(
        isolation_sites("command.current_dir(repository_root());\n"),
        [(1, FORM_CURRENT_DIR)]
    );
    assert_eq!(
        isolation_sites("command.env(\"PATH\", host_path_string);\n"),
        [(1, FORM_PATH_ENV)]
    );
    assert_eq!(
        isolation_sites("command.env_remove(\"PATH\");\n"),
        [(1, FORM_PATH_ENV_REMOVE)]
    );
    assert_eq!(
        isolation_sites("command.env_clear();\n"),
        [(1, FORM_ENV_CLEAR)]
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

/// The first containment namespace: home, config, cache, and temp. Handing
/// any of these back points the child at the developer's own directories.
#[test]
fn isolation_detector_flags_a_home_config_cache_anchor_taken_back_after_build() {
    // The accident the review named: the fixture home replaced by the host's.
    assert_eq!(
        isolation_sites("command.env(\"HOME\", host_home);\n"),
        [(1, FORM_HOME_CONTAINMENT)]
    );
    for source in [
        "command.env(\"USERPROFILE\", host_home);\n",
        "command.env(\"APPDATA\", roaming);\n",
        "command.env(\"LOCALAPPDATA\", local);\n",
        "command.env(\"XDG_CONFIG_HOME\", host_config);\n",
        "command.env(\"XDG_CACHE_HOME\", host_cache);\n",
        "command.env(\"TMPDIR\", host_tmp);\n",
        "command.env(\"TEMP\", host_tmp);\n",
        "command.env(\"TMP\", host_tmp);\n",
        // Removal is the same defect: the child falls back to the platform's
        // own answer, which is the host's.
        "command.env_remove(\"HOME\");\n",
        "command.env_remove(\"HOMEDRIVE\");\n",
        "command.env_remove(\"HOMEPATH\");\n",
    ] {
        assert_eq!(
            isolation_sites(source),
            [(1, FORM_HOME_CONTAINMENT)],
            "{source:?} must be flagged as home containment"
        );
    }
}

/// The second containment namespace: Git plumbing overrides cwd-based
/// repository discovery, so it defeats the pinned launch directory outright.
#[test]
fn isolation_detector_flags_git_plumbing_taken_back_after_build() {
    // The second accident the review named.
    assert_eq!(
        isolation_sites("command.env(\"GIT_DIR\", checkout);\n"),
        [(1, FORM_GIT_CONTAINMENT)]
    );
    for source in [
        "command.env(\"GIT_WORK_TREE\", checkout);\n",
        "command.env(\"GIT_INDEX_FILE\", index);\n",
        "command.env(\"GIT_COMMON_DIR\", common);\n",
        "command.env(\"GIT_OBJECT_DIRECTORY\", objects);\n",
        "command.env(\"GIT_CONFIG_GLOBAL\", host_gitconfig);\n",
        "command.env(\"GIT_CONFIG_COUNT\", \"1\");\n",
        "command.env_remove(\"GIT_CONFIG_NOSYSTEM\");\n",
    ] {
        assert_eq!(
            isolation_sites(source),
            [(1, FORM_GIT_CONTAINMENT)],
            "{source:?} must be flagged as Git plumbing containment"
        );
    }
}

/// The rendering namespace is a behavior input, so the claim is legitimate and
/// the defect is only that it was never declared.
#[test]
fn isolation_detector_flags_an_undeclared_rendering_input() {
    assert_eq!(
        isolation_sites("command.env(\"COLUMNS\", \"44\");\n"),
        [(1, FORM_RENDERING_INPUT)]
    );
    for source in [
        "command.env(\"LINES\", \"24\");\n",
        "command.env(\"TERM\", \"dumb\");\n",
        "command.env(\"COLORTERM\", \"truecolor\");\n",
        "command.env(\"FORCE_COLOR\", \"1\");\n",
        "command.env(\"THEME\", theme);\n",
        "command.env(\"CODE_THEME\", theme);\n",
        "command.env_remove(\"NO_COLOR\");\n",
        "command.env_remove(\"COLORTERM\");\n",
        "command.env_remove(\"DARK_MODE\");\n",
    ] {
        assert_eq!(
            isolation_sites(source),
            [(1, FORM_RENDERING_INPUT)],
            "{source:?} must be flagged as an undeclared rendering input"
        );
    }
}

/// The darkmatter application namespace, by prefix and by the names `md`
/// reads directly.
#[test]
fn isolation_detector_flags_an_undeclared_darkmatter_application_input() {
    assert_eq!(
        isolation_sites("command.env(\"DARKMATTER_NO_BASELINE_SCHEMA\", \"1\");\n"),
        [(1, FORM_APPLICATION_INPUT)]
    );
    for source in [
        "command.env(\"DM_TEST_VAR\", value);\n",
        "command.env(\"MD_DRY_RUN\", \"1\");\n",
        "command.env(\"HASH_PROPERTY\", \"fingerprint\");\n",
        "command.env(\"HASH_IGNORE_PROPERTIES\", \"draft\");\n",
        "command.env(\"BASELINE_SCHEMA\", &baseline);\n",
        "command.env(\"RUST_LOG\", \"biscuit_terminal=debug\");\n",
        "command.env(\"AGENT\", \"claude\");\n",
        "command.env(\"MODEL\", \"opus\");\n",
        "command.env_remove(\"DARKMATTER_NO_BASELINE_SCHEMA\");\n",
    ] {
        assert_eq!(
            isolation_sites(source),
            [(1, FORM_APPLICATION_INPUT)],
            "{source:?} must be flagged as an undeclared application input"
        );
    }
}

/// A declaration is not an escape: it happens before `build()`, on the
/// builder, where the guard's method-call detector has nothing to find.
#[test]
fn a_declared_input_is_not_an_isolation_escape() {
    assert!(
        isolation_sites(
            "let command = fixture\n    .command_builder()\n    \
             .rendering_input(\"COLUMNS\", \"80\")\n    \
             .rendering_input_removed(\"NO_COLOR\")\n    \
             .application_input(\"MD_DRY_RUN\", \"1\")\n    \
             .plain_terminal(80, 24)\n    .build();\n"
        )
        .is_empty()
    );
}

#[test]
fn isolation_detector_ignores_prose_neighbors_and_path_qualified_lookalikes() {
    // This guard's own docs, and `md_process_fixture.rs`'s, name every form.
    assert!(isolation_sites("//! A `.current_dir(root)` undoes the pin.\n").is_empty());
    assert!(isolation_sites("/// survives its own `env_clear()`\n").is_empty());
    assert!(isolation_sites("/* command.env(\"PATH\", value) */\n").is_empty());
    assert!(isolation_sites("let doc = \"command.env_clear()\";\n").is_empty());
    assert!(isolation_sites("let doc = r#\"cmd.env(\"PATH\", x)\"#;\n").is_empty());

    // Path-qualified lookalikes are not method calls on a built command.
    assert!(isolation_sites("let here = std::env::current_dir().unwrap();\n").is_empty());
    assert!(isolation_sites("let path = std::env::var(\"PATH\").unwrap();\n").is_empty());
    assert!(isolation_sites("let path = env::var_os(\"PATH\");\n").is_empty());
    // The parent process setting its own hostile environment is not a child
    // escape — `md_process_fixture.rs`'s hostile-environment test does exactly
    // this to *prove* the defaults hold.
    assert!(isolation_sites("std::env::set_var(\"PATH\", poisoned);\n").is_empty());
    // An index, not a call.
    assert!(isolation_sites("let value = &recorded[\"PATH\"];\n").is_empty());
    // `.env` naming a variable the spawn contract does not pin is the ordinary
    // supported form: there is no default for it to undo.
    assert!(isolation_sites("command.env(\"PROJECT_ROOT\", &abs_root);\n").is_empty());
    assert!(isolation_sites("command.env(\"FIXTURE_PROBE_CAPTURE\", capture);\n").is_empty());
    // A computed key cannot be classified, so it is not reported; the guard
    // reads literals only, and every protected key in this suite is one.
    assert!(isolation_sites("command.env(key, value);\n").is_empty());

    // Identifier boundaries hold on both sides.
    assert!(isolation_sites("command.set_current_dir(root);\n").is_empty());
    assert!(isolation_sites("command.current_dir_of(root);\n").is_empty());
    assert!(isolation_sites("command.env_clear_all();\n").is_empty());
    // A bare mention with no call is not an escape.
    assert!(isolation_sites("command.current_dir;\n").is_empty());
}

#[test]
fn a_file_that_never_builds_a_fixture_command_is_outside_the_isolation_gate() {
    assert!(obtains_a_fixture_command(
        "let fixture = CliProcessFixture::named(\"x\");\n"
    ));
    assert!(obtains_a_fixture_command(
        "use common::CliProcessFixture;\n"
    ));
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

/// The tier taxonomy this guard mirrors: the same prefixes `_tier_filter`
/// drops from L1, plus `common/` where the builder lives.
#[test]
fn tier_naming_decides_what_the_guard_governs() {
    assert!(excluded("level2_errors.rs"));
    assert!(excluded("level2_schema_about.rs"));
    assert!(excluded("level3_popover.rs"));
    assert!(excluded("browser_layout.rs"));
    assert!(excluded("real_provider_call.rs"));
    assert!(excluded("slow_compose_warmup.rs"));
    assert!(excluded("common/mod.rs"));
    assert!(excluded("common/fixture.rs"));
    assert!(excluded("common/level2.rs"));
    assert!(excluded("common/source_scan.rs"));
    // The L1 population stays governed after migration.
    assert!(!excluded("clean.rs"));
    assert!(!excluded("schema_validate_baseline.rs"));
    assert!(!excluded("md_process_fixture.rs"));
    assert!(!excluded("spawn_site_guard.rs"));
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
            form: FORM_MD_CMD,
        },
        Site {
            file: "migrated_but_regressed.rs".into(),
            line: 7,
            form: FORM_MD_CMD,
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
                reason: PENDING_6B,
            },
            AllowEntry {
                file: "allowed.rs",
                reason: "   ",
            },
        ],
    );

    assert_eq!(result.unlisted, ["migrated_but_regressed.rs:7 md_cmd"]);
    assert_eq!(result.stale, ["already_migrated.rs"]);
    assert_eq!(result.unexplained, ["allowed.rs"]);
    assert_eq!(result.burn_down, BTreeMap::from([("allowed.rs".into(), 2)]));
}

/// The isolation gate's population widens by one edit: deleting a file's
/// [`SPAWN_ALLOWLIST`] entry. Phase 6 does that per batch, and this is what
/// makes the widening automatic rather than a second list to maintain.
#[test]
fn deleting_a_spawn_entry_is_what_widens_the_isolation_population() {
    let migrated = "let fixture = CliProcessFixture::named(\"x\");\n";
    // A synthetic list, not the live one: the rule has to stay provable after
    // the burn-down empties `SPAWN_ALLOWLIST`, and it is the *entry* that is
    // the input, not whichever file happens still to carry one.
    const BEFORE: &[AllowEntry] = &[AllowEntry {
        file: "not_yet_migrated.rs",
        reason: PENDING_6B,
    }];
    const AFTER: &[AllowEntry] = &[];

    // With the entry: out of the isolation gate whatever its source says.
    assert!(!governs_isolation_against(
        "not_yet_migrated.rs",
        migrated,
        BEFORE
    ));
    // Without it — the single edit a migration batch makes — the same file and
    // the same source are governed.
    assert!(governs_isolation_against(
        "not_yet_migrated.rs",
        migrated,
        AFTER
    ));
    // A file holding no fixture command stays out either way — it has no
    // builder isolation to undo.
    assert!(!governs_isolation_against(
        "not_yet_migrated.rs",
        "fn main() {}\n",
        AFTER
    ));
    // And the tier exclusions still win over both.
    assert!(!governs_isolation_against(
        "level2_schema_about.rs",
        migrated,
        AFTER
    ));
    // The live gate delegates to exactly this predicate. With the burn-down
    // empty, migrated files are now inside the isolation gate.
    assert_eq!(
        governs_isolation("not_yet_migrated.rs", migrated),
        governs_isolation_against("not_yet_migrated.rs", migrated, SPAWN_ALLOWLIST)
    );
    assert!(governs_isolation("clean.rs", migrated));
}

/// A second reason, used only by the synthetic census below.
///
/// The artifact groups by reason, so proving that roll-up needs two distinct
/// reasons in one census.
const SAMPLE_SECOND_REASON: &str = "needs a live Child; assert_cmd has no spawn";

/// A synthetic two-file census, shared by the artifact tests.
fn sample_burn_down() -> BurnDown {
    const ALLOWLIST: &[AllowEntry] = &[
        AllowEntry {
            file: "allowed.rs",
            reason: PENDING_6A,
        },
        AllowEntry {
            file: "windows_ctrl_c.rs",
            reason: SAMPLE_SECOND_REASON,
        },
        AllowEntry {
            file: "already_migrated.rs",
            reason: PENDING_6A,
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
            form: FORM_MD_CMD,
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
            r#"{{"kind":"file","gate":"spawn","file":"allowed.rs","reason":"{p6a}","sites":2}}"#,
            "\n",
            r#"{{"kind":"file","gate":"spawn","file":"windows_ctrl_c.rs","reason":"{live}","sites":1}}"#,
            "\n",
            r#"{{"kind":"reason","gate":"spawn","reason":"{p6a}","files":1,"sites":2}}"#,
            "\n",
            r#"{{"kind":"reason","gate":"spawn","reason":"{live}","files":1,"sites":1}}"#,
            "\n",
            r#"{{"kind":"total","gate":"spawn","files":2,"sites":3,"scanned_sites":3,"governed_files":9}}"#,
            "\n",
        ),
        p6a = PENDING_6A,
        live = SAMPLE_SECOND_REASON,
    );

    assert_eq!(sample_burn_down().to_jsonl(), expected);
}

#[test]
fn the_artifact_is_written_even_when_the_staging_directory_does_not_exist() {
    let stage = tempfile::tempdir().expect("temp dir");
    // Two levels deep and never created: the local `cargo nextest` case, where
    // nothing has staged a report yet.
    let path = stage
        .path()
        .join("nextest/ci-reports")
        .join(SPAWN_REPORT_FILE);
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

// ---------------------------------------------------------------------------
// Non-vacuity over the real population
// ---------------------------------------------------------------------------

/// Non-vacuity for the spawn gate. The gate passes while 38 files are exempt,
/// which is indistinguishable from a gate that reads no files at all — so the
/// population is asserted directly, its census shape is asserted against the
/// seed counts, and a planted raw form is asserted to still be found. When
/// Phase 6 empties the list this same test carries the zero: `sites` empty is
/// then a fact this population assertion backs, not a blind spot.
#[test]
fn the_spawn_gate_reads_a_real_population_and_still_finds_a_planted_site() {
    let governed = governed_files(&tests_root(), governs_spawn).expect("spawn-site population");
    assert!(
        governed.len() > 30,
        "the spawn gate scanned only {} file(s); the L1 population is ~40, so the scan root \
         or the predicate is wrong",
        governed.len()
    );
    // The population is the real L1 suite, not a stray directory.
    let names: BTreeSet<&str> = governed.iter().map(|(file, _)| file.as_str()).collect();
    for expected in [
        "clean.rs",
        "code_block.rs",
        "schema_validate.rs",
        "schema_validate_baseline.rs",
        "toc.rs",
        "md_process_fixture.rs",
        "spawn_site_guard.rs",
    ] {
        assert!(
            names.contains(expected),
            "{expected} must be governed: {names:?}"
        );
    }
    // A site-free L1 file is governed too — governed means scanned, not
    // flagged (`layout_alignment.rs` resolves layout in-process).
    assert!(names.contains("layout_alignment.rs"));
    assert!(!names.contains("level2_errors.rs"));

    let sites = sites_in(&governed, spawn_sites);
    // Phase 6 migrated all 518 seed sites across the 38 allow-listed files.
    // A nonzero census means a raw spawn has regrown.
    assert_eq!(
        sites.len(),
        0,
        "the Phase 6 closure census must contain no raw md spawn"
    );
    let site_files: BTreeSet<&str> = sites.iter().map(|site| site.file.as_str()).collect();
    assert_eq!(
        site_files.len(),
        0,
        "the Phase 6 closure census must contain no raw-spawn files: {site_files:?}"
    );
    for entry in SPAWN_ALLOWLIST {
        assert!(
            site_files.contains(entry.file),
            "{} carries a SPAWN_ALLOWLIST entry but no live site — the stale-entry arm of the \
             gate test reports the same failure",
            entry.file
        );
    }

    // …and the detector that reported the census is the one that still finds a
    // site when there is one to find, so the numbers above are facts, not a
    // blind spot. The base is a real governed file's source that carries no
    // site of its own (`layout_alignment.rs` resolves layout in-process), so
    // the one hit is unambiguously the planted one.
    let site_free = governed
        .iter()
        .find(|(file, source)| file == "layout_alignment.rs" && spawn_sites(source).is_empty())
        .expect("layout_alignment.rs is governed and site-free");
    let planted = spawn_sites(&format!(
        "{}\nlet c = assert_cmd::Command::cargo_bin(\"md\").unwrap();\n",
        site_free.1
    ));
    assert_eq!(
        planted.len(),
        1,
        "a planted raw spawn must still be detected"
    );
    let result = reconcile(
        &[Site {
            file: "planted.rs".into(),
            line: planted[0].0,
            form: planted[0].1,
        }],
        SPAWN_ALLOWLIST,
    );
    assert_eq!(result.unlisted.len(), 1, "and must reconcile as unlisted");
}

/// Non-vacuity for the isolation gate. It governs the migrated L1 population
/// plus the fixture's self-test binary, whose two `.current_dir` sites are
/// allow-listed parent-side `git()` helpers.
#[test]
fn the_isolation_gate_governs_the_fixture_self_tests_and_only_those() {
    let governed: BTreeSet<String> = governed_files(&tests_root(), governs_isolation)
        .expect("isolation population")
        .into_iter()
        .map(|(file, _)| file)
        .collect();

    assert!(
        governed.contains("md_process_fixture.rs"),
        "the fixture self-test binary holds builder-produced commands and must be governed: \
         {governed:?}"
    );
    assert!(governed.contains("clean.rs"));
    assert!(
        governed.len() > 30,
        "the isolation gate must govern the migrated L1 population: {governed:?}"
    );
    // The tier and `common/` exclusions still apply.
    assert!(!governed.contains("level2_errors.rs"));
    assert!(!governed.contains("common/fixture.rs"));
    // This guard names the fixture in prose only.
    assert!(!governed.contains("spawn_site_guard.rs"));
}
