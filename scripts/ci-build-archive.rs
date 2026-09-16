//! Producing and verifying one run-scoped build record's archive.
//!
//! `fixes/2026-09-12-single-os-compile/spec.md` splits a test cell in two: a
//! native **producer** compiles one immutable Nextest archive per planned build
//! key, and every **consumer** verifies and executes those exact outputs without
//! a compiler. This module is both halves —
//! [`produce`](fn@run_produce) writes the archive, its sidecars, and the
//! manifest that describes them; [`verify`](fn@run_verify) is what a consumer
//! runs before it extracts anything.
//!
//! ## Why a manifest, and not just a checksum
//!
//! A checksum proves an artifact arrived intact. It cannot say whether it is
//! the *right* artifact: the same bytes are a valid archive of the wrong
//! revision, the wrong feature graph, or the wrong libc. The manifest carries
//! the planner's identity fields verbatim alongside the producer's discovered
//! ones, so a consumer refuses a mismatch by name instead of failing later,
//! inside a test, as something that reads like a product bug.
//!
//! ## Notes
//!
//! Every rejection is drawn from [`REJECTIONS`], which
//! `scripts/ci/schema.py::BUILD_REJECTIONS` mirrors. A verifier never compiles
//! a replacement for anything it refuses: falling back to a build is precisely
//! the behavior the specification exists to remove.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use biscuit_hash::{blake3_hash_reader, xx_hash};
use biscuit_terminal::prelude::{Prose, Table, TableColumn, Terminal, TerminalRenderable};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[path = "ci-build-runtime.rs"]
mod runtime;

/// Bumped whenever a manifest's field set changes. A consumer refuses a
/// generation it does not understand rather than verifying the fields it
/// happens to recognize.
pub const MANIFEST_SCHEMA_VERSION: u32 = 3;

/// Bumped whenever a build status document's field set changes. The status is
/// the only thing a failed producer emits, so a rollup that cannot read one
/// must say so by name rather than treat the cell as unexplained.
pub const BUILD_STATUS_SCHEMA_VERSION: u32 = 1;

/// Bumped whenever the `verify --json` document's field set changes.
pub const VERDICT_SCHEMA_VERSION: u32 = 1;

/// The Nextest profile the producer archives under.
///
/// A *named* profile, not `default`: nextest resolves a profile-specific key
/// ahead of `profile.default` from a higher-priority config file, which is the
/// only layering that lets the generated tool config supply
/// `archive.include` while the repository's own `.config/nextest.toml` keeps
/// owning timeouts, leak windows, and per-test overrides.
pub const ARCHIVE_PROFILE: &str = "ci-build-archive";

/// Why a consumer refused its inputs. Mirrored by
/// `scripts/ci/schema.py::BUILD_REJECTIONS`; adding a code is a contract change.
///
/// Every one of them is an *infrastructure* verdict. None of them is a test
/// result, and none of them may be repaired by compiling something.
pub const REJECTIONS: [&str; 15] = [
    "build-manifest-missing",
    "build-manifest-malformed",
    "build-manifest-schema",
    "build-digest-mismatch",
    "build-key-mismatch",
    "build-source-mismatch",
    "build-environment-incompatible",
    "build-runtime-incompatible",
    "build-archive-missing",
    "build-archive-corrupt",
    "build-sidecar-missing",
    "build-sidecar-corrupt",
    "build-asset-missing",
    "build-inventory-incomplete",
    "build-inventory-unexpected",
];

// ---------------------------------------------------------------------------
// The resolved plan's build slice
// ---------------------------------------------------------------------------

/// Every plan-known compile-affecting input of one archive.
///
/// Field for field `scripts/ci/schema.py::BUILD_IDENTITY_FIELDS`. `deny_unknown_fields`
/// is deliberate: a planner that grew a field this producer ignores would
/// otherwise compile something the key no longer describes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub source_commit: String,
    pub lockfile: String,
    pub rust: String,
    pub nextest: String,
    pub host: String,
    pub target: String,
    pub profile: String,
    pub rustflags: String,
    pub cargo_config: Vec<String>,
    pub linker: String,
    pub archive_format: String,
    pub package: String,
    pub target_kinds: Vec<String>,
    /// The package's declared CI feature arguments, as one command-line
    /// fragment (`--features a,b`, `--all-features`, or empty) — a string
    /// rather than a list because that is what the planner writes and what
    /// `schema.py::BUILD_IDENTITY_FIELDS` validates. It is split on whitespace
    /// where it is handed to Cargo, and digested as written everywhere else.
    pub features: String,
    pub native: Vec<String>,
    pub archive_includes: Vec<String>,
    pub sidecars: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Consumer {
    pub environment: String,
    pub gate: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRecord {
    pub key: String,
    pub package: String,
    pub producer: String,
    pub artifact: String,
    pub compatible_environments: Vec<String>,
    pub compatibility_reason: String,
    pub consumers: Vec<Consumer>,
    pub identity: Identity,
}

/// The execution predicates one environment's binaries must satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Runtime {
    pub arch: String,
    pub abi: String,
    pub libc: String,
    #[serde(default)]
    pub native_libraries: Vec<String>,
}

/// The subset of the plan this module reads.
///
/// The plan is the scheduling authority and this is a reader: nothing here
/// decides what to build, only how to build what was already decided.
#[derive(Debug)]
pub struct Plan {
    pub head: String,
    pub builds: Vec<BuildRecord>,
    /// `environments[].build.runtime`, by environment name.
    pub runtimes: BTreeMap<String, Runtime>,
}

pub fn read_plan(path: &Path) -> Result<Plan> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading the plan {}", path.display()))?;
    let document: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let head = document
        .get("head")
        .and_then(Value::as_str)
        .context("the resolved plan has no 'head'")?
        .to_owned();
    let builds: Vec<BuildRecord> = serde_json::from_value(
        document
            .get("builds")
            .cloned()
            .context("the resolved plan has no 'builds' list")?,
    )
    .context("reading the plan's build records")?;

    let mut runtimes = BTreeMap::new();
    if let Some(entries) = document.get("environments").and_then(Value::as_array) {
        for entry in entries {
            let Some(name) = entry.get("name").and_then(Value::as_str) else {
                continue;
            };
            if let Some(runtime) = entry.pointer("/build/runtime") {
                let parsed: Runtime = serde_json::from_value(runtime.clone())
                    .with_context(|| format!("reading {name}'s runtime predicates"))?;
                runtimes.insert(name.to_owned(), parsed);
            }
        }
    }
    Ok(Plan {
        head,
        builds,
        runtimes,
    })
}

// ---------------------------------------------------------------------------
// The sidecar table
// ---------------------------------------------------------------------------

/// One named build sidecar: another package's binaries, compiled by the
/// producer and shipped beside the archive.
///
/// A closed, data-declared vocabulary rather than a shell-command field. A
/// package names a sidecar; it never says how to build one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sidecar {
    pub package: String,
    #[serde(default)]
    pub features: Vec<String>,
    pub bins: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SidecarTable {
    schema_version: u32,
    sidecars: BTreeMap<String, Sidecar>,
}

/// Bumped whenever `.github/ci/sidecars.json`'s field set changes.
pub const SIDECAR_SCHEMA_VERSION: u32 = 1;

pub fn read_sidecars(path: &Path) -> Result<BTreeMap<String, Sidecar>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading the sidecar table {}", path.display()))?;
    let table: SidecarTable =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    if table.schema_version != SIDECAR_SCHEMA_VERSION {
        bail!(
            "{} is sidecar schema version {}, this tool reads {}",
            path.display(),
            table.schema_version,
            SIDECAR_SCHEMA_VERSION
        );
    }
    Ok(table.sidecars)
}

// ---------------------------------------------------------------------------
// The manifest
// ---------------------------------------------------------------------------

/// One file the producer emitted, by size and content digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    /// Path relative to the manifest's own directory, so the whole set of
    /// emitted files relocates as a unit.
    pub file: String,
    pub bytes: u64,
    pub blake3: String,
}

/// A sidecar binary, or a target-relative file the archive was made to carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedFile {
    pub name: String,
    #[serde(flatten)]
    pub record: FileRecord,
}

/// What the producer observed about itself, as opposed to what the plan
/// asserted. A disagreement between the two is a rejection, not a warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Realized {
    pub rustc: String,
    pub cargo: String,
    pub nextest: String,
    pub host: String,
    pub target: String,
    pub linker: String,
    pub runtime: Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Timings {
    pub setup_ms: u64,
    /// `cargo nextest archive` compiles and archives in one command, so the two
    /// are not separable from outside it. Named for what it measures.
    pub compile_archive_ms: u64,
    pub sidecars_ms: u64,
    pub inventory_ms: u64,
    pub checksum_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub key: String,
    /// Digest of everything above the observational fields — see
    /// [`realized_digest`]. A consumer recomputes it, so an edited manifest
    /// fails before an edited archive ever gets read.
    pub digest: String,
    pub package: String,
    pub producer: String,
    pub artifact: String,
    pub compatible_environments: Vec<String>,
    pub source_commit: String,
    /// The producer's working-tree identity. `head` alone cannot distinguish a
    /// clean checkout from a locally modified one.
    pub source_tree: String,
    /// The absolute workspace root the producer compiled at, forward-slashed.
    ///
    /// `--workspace-remap` rewrites the *run-time* `CARGO_MANIFEST_DIR`, but a
    /// test that reads the compile-time `env!("CARGO_MANIFEST_DIR")` still
    /// opens this path. 150-odd such sites remain in the workspace, so a
    /// consumer that cannot place its checkout freely — the WSL2 guest, which
    /// clones rather than sharing the producer's disk — reads the path out of
    /// here instead of hardcoding one that drifts. `fixes/2026-09-12-single-os-compile`
    /// Task 6.4 moves those sites to runtime lookup and retires this use; the
    /// field stays as the producer's own record of where it built.
    pub producer_workspace: String,
    pub identity: Identity,
    pub realized: Realized,
    pub archive: FileRecord,
    pub sidecars: Vec<NamedFile>,
    /// Target-relative paths the producer added to the archive from each
    /// declared `archive-includes` entry, with the size and digest they had
    /// when they went in.
    pub runtime_assets: Vec<NamedFile>,
    /// Every binary id the archive contains, sorted. A consumer that finds
    /// fewer has an incomplete archive; more means it is not this build.
    pub test_binaries: Vec<String>,
    /// The `ci-build report --json` document for this build, when the producer
    /// was measured. Absent on an ordinary run: `RUSTC_WRAPPER` is empty by
    /// default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_work: Option<Value>,
    pub timings: Timings,
}

/// The canonical byte form a digest is taken over.
///
/// `serde_json`'s object representation is a `BTreeMap`, so serializing a
/// `Value` is already key-sorted and separator-tight — the same shape
/// `scripts/ci/schema.py::canonical` produces, which is what lets the two sides
/// of this contract compare digests at all.
fn canonical(value: &Value) -> String {
    serde_json::to_string(value).expect("a Value always serializes")
}

/// The digest of everything in `manifest` except the observational fields.
///
/// `digest`, `timings`, and `compiler_work` are excluded: the first is the
/// output, and the other two legitimately differ between two runs that produced
/// byte-identical artifacts. Everything else is the claim being made.
pub fn realized_digest(manifest: &Manifest) -> String {
    let material = json!({
        "schema_version": manifest.schema_version,
        "key": manifest.key,
        "package": manifest.package,
        "producer": manifest.producer,
        "artifact": manifest.artifact,
        "compatible_environments": manifest.compatible_environments,
        "source_commit": manifest.source_commit,
        "source_tree": manifest.source_tree,
        "producer_workspace": manifest.producer_workspace,
        "identity": manifest.identity,
        "realized": manifest.realized,
        "archive": manifest.archive,
        "sidecars": manifest.sidecars,
        "runtime_assets": manifest.runtime_assets,
        "test_binaries": manifest.test_binaries,
    });
    format!("{:016x}", xx_hash(&canonical(&material)))
}

pub fn read_manifest(path: &Path) -> std::result::Result<Manifest, Rejection> {
    let text = fs::read_to_string(path).map_err(|error| {
        Rejection::new(
            "build-manifest-missing",
            format!("{}: {error}", path.display()),
        )
    })?;
    // Version first, and from the raw document: a manifest from another
    // generation must miss cleanly by name rather than fail as a typed parse
    // error that reads like corruption.
    let raw: Value = serde_json::from_str(&text).map_err(|error| {
        Rejection::new(
            "build-manifest-malformed",
            format!("{}: {error}", path.display()),
        )
    })?;
    match raw.get("schema_version").and_then(Value::as_u64) {
        Some(version) if version == u64::from(MANIFEST_SCHEMA_VERSION) => {}
        Some(version) => {
            return Err(Rejection::new(
                "build-manifest-schema",
                format!(
                    "{} is manifest schema version {version}, this consumer reads {}",
                    path.display(),
                    MANIFEST_SCHEMA_VERSION
                ),
            ));
        }
        None => {
            return Err(Rejection::new(
                "build-manifest-malformed",
                format!("{} declares no schema_version", path.display()),
            ));
        }
    }
    serde_json::from_value(raw).map_err(|error| {
        Rejection::new(
            "build-manifest-malformed",
            format!("{}: {error}", path.display()),
        )
    })
}

// ---------------------------------------------------------------------------
// File digests
// ---------------------------------------------------------------------------

fn file_record(root: &Path, path: &Path) -> Result<FileRecord> {
    let mut file =
        fs::File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let bytes = file
        .metadata()
        .with_context(|| format!("sizing {}", path.display()))?
        .len();
    let blake3 = blake3_hash_reader(&mut file)
        .with_context(|| format!("checksumming {}", path.display()))?;
    Ok(FileRecord {
        file: relative_to(root, path),
        bytes,
        blake3,
    })
}

/// `path` spelled relative to `root`, with forward slashes.
///
/// Manifest paths are compared across machines, and a Windows producer that
/// wrote `sidecars\md.exe` would not match a Linux consumer's reading of the
/// same field. One spelling everywhere.
fn relative_to(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

// ---------------------------------------------------------------------------
// Producing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProduceOptions {
    pub plan: PathBuf,
    pub producer: String,
    /// Optional single-key selection for local diagnosis. Hosted owners omit
    /// this so all their records share one target tree.
    pub key: Option<String>,
    pub out_dir: PathBuf,
    pub workspace: PathBuf,
    pub target_dir: Option<PathBuf>,
    pub sidecars: Option<PathBuf>,
    pub source_tree: Option<String>,
    /// Setting this turns the compiler-work counter on for the Cargo and
    /// Nextest commands each record drives, and each record's manifest then
    /// carries the counts from its own `<counter-dir>/<artifact>` slice. Left
    /// unset, no wrapper is set anywhere and no manifest records counts.
    pub counter_dir: Option<PathBuf>,
    pub cargo: String,
    /// The nextest driver, split on whitespace. `cargo nextest` on a host with
    /// Cargo; `cargo-nextest nextest` where only the standalone binary exists.
    pub nextest: Vec<String>,
    pub json: bool,
}

/// Cargo's output directory for `profile`.
///
/// `test` and `bench` are the profiles Cargo *derives* for test targets; their
/// artifacts land in `debug` and `release` respectively, and a producer that
/// looked for them under their own names would find nothing.
fn profile_dir(profile: &str) -> &str {
    match profile {
        "test" | "dev" => "debug",
        "bench" => "release",
        other => other,
    }
}

/// Expand the platform placeholders a declared archive include may carry.
///
/// A dynamic library is spelled `libfoo.so`, `libfoo.dylib`, and `foo.dll` on
/// the three producers; making each package declare three paths would be three
/// chances to get one wrong.
pub fn expand_placeholders(entry: &str) -> String {
    entry
        .replace("{DLL_PREFIX}", std::env::consts::DLL_PREFIX)
        .replace("{DLL_SUFFIX}", std::env::consts::DLL_SUFFIX)
        .replace("{EXE_SUFFIX}", std::env::consts::EXE_SUFFIX)
}

/// The example target one declared archive include names, if it names one.
///
/// `examples/discovery_probe{EXE_SUFFIX}` is the one include class the archive
/// build does not produce for itself, so the producer has to recognize it.
/// Only a direct child of `examples/` qualifies: Cargo has no nested example
/// targets, so a deeper path is some other build output that happens to live
/// there and must not be built as one.
pub fn example_target(entry: &str) -> Option<String> {
    let name = expand_placeholders(entry).strip_prefix("examples/")?.to_owned();
    let stem = name
        .strip_suffix(std::env::consts::EXE_SUFFIX)
        .filter(|_| !std::env::consts::EXE_SUFFIX.is_empty())
        .unwrap_or(&name);
    (!stem.is_empty() && !stem.contains('/')).then(|| stem.to_owned())
}

/// The target-relative path of one declared archive include.
///
/// Declarations are relative to the *profile output* directory, so a package
/// says `examples/discovery_probe` once and the producer supplies the
/// `<triple>/<profile>` prefix its own invocation created.
pub fn include_path(identity: &Identity, entry: &str) -> String {
    let mut parts = Vec::new();
    if !identity.target.is_empty() {
        parts.push(identity.target.clone());
    }
    parts.push(profile_dir(&identity.profile).to_owned());
    parts.push(expand_placeholders(entry));
    parts.join("/")
}

/// The nextest tool config that carries this build's archive includes.
///
/// The repository's own `profile.default.archive.include` is merged in rather
/// than replaced: those entries are still the contract for every `cargo nextest
/// archive` this producer did not drive, and silently dropping them would
/// remove files from the archive that tests already depend on.
pub fn tool_config(identity: &Identity, inherited: &[toml::Value]) -> String {
    let mut include: Vec<toml::Value> = inherited.to_vec();
    for entry in &identity.archive_includes {
        let mut declared = toml::map::Map::new();
        declared.insert(
            "path".to_owned(),
            toml::Value::String(include_path(identity, entry)),
        );
        declared.insert(
            "relative-to".to_owned(),
            toml::Value::String("target".to_owned()),
        );
        // `error`, not `ignore`: a declared include that the build did not
        // produce is a broken declaration, and discovering that as a missing
        // file inside a consumer's test is far more expensive than here.
        declared.insert(
            "on-missing".to_owned(),
            toml::Value::String("error".to_owned()),
        );
        include.push(toml::Value::Table(declared));
    }

    let mut archive = toml::map::Map::new();
    archive.insert("include".to_owned(), toml::Value::Array(include));
    let mut profile = toml::map::Map::new();
    profile.insert("archive".to_owned(), toml::Value::Table(archive));
    let mut profiles = toml::map::Map::new();
    profiles.insert(ARCHIVE_PROFILE.to_owned(), toml::Value::Table(profile));
    let mut document = toml::map::Map::new();
    document.insert("profile".to_owned(), toml::Value::Table(profiles));

    format!(
        "# Generated by `ci-build produce`. Do not edit or commit.\n{}",
        toml::to_string(&toml::Value::Table(document))
            .expect("a table of strings always serializes")
    )
}

/// The repository's own `profile.default.archive.include`, or an empty list.
fn inherited_includes(workspace: &Path) -> Result<Vec<toml::Value>> {
    let path = workspace.join(".config/nextest.toml");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let document: toml::Table =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(document
        .get("profile")
        .and_then(|profiles| profiles.get("default"))
        .and_then(|profile| profile.get("archive"))
        .and_then(|archive| archive.get("include"))
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// `fs::canonicalize`, with Windows's verbatim prefix removed where it is safe.
///
/// Windows canonicalization answers `\\?\C:\…`. Two of the arguments this
/// module builds are parsed rather than merely opened: nextest splits
/// `--tool-config-file` on the colon after the tool name, and compares
/// `--workspace-remap` against ordinary paths. Neither is written for a
/// verbatim spelling. A long path or a UNC one still needs the prefix and keeps
/// it — the failure mode there is an unopenable path, which is louder than a
/// silently mismatched one.
fn canonical_path(path: &Path) -> Result<PathBuf> {
    let resolved =
        fs::canonicalize(path).with_context(|| format!("resolving {}", path.display()))?;
    #[cfg(windows)]
    {
        let text = resolved.to_string_lossy().into_owned();
        if let Some(stripped) = text.strip_prefix(r"\\?\") {
            let drive_qualified = stripped.as_bytes().get(1) == Some(&b':');
            if drive_qualified && stripped.len() < 260 {
                return Ok(PathBuf::from(stripped));
            }
        }
    }
    Ok(resolved)
}

fn command_line(command: &str, args: &[String]) -> String {
    std::iter::once(command.to_owned())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run `command`, failing loudly with everything it printed.
///
/// A producer that swallowed a compile failure would upload an archive of the
/// previous build, which is the one failure mode a checksum cannot detect.
fn run(command: &str, args: &[String], cwd: &Path, env: &[(String, OsString)]) -> Result<()> {
    let mut process = Command::new(command);
    process.args(args).current_dir(cwd);
    for (name, value) in env {
        process.env(name, value);
    }
    let status = process
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("running `{}`", command_line(command, args)))?;
    if !status.success() {
        bail!(
            "`{}` failed with {}",
            command_line(command, args),
            status
                .code()
                .map_or_else(|| "a signal".to_owned(), |code| format!("exit code {code}"))
        );
    }
    Ok(())
}

fn capture(command: &str, args: &[String], cwd: &Path) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("running `{}`", command_line(command, args)))?;
    if !output.status.success() {
        bail!(
            "`{}` failed: {}",
            command_line(command, args),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The first line of a `--version` answer, or a stable placeholder.
///
/// Tool-version display fallback. Linker provenance uses a strict probe instead.
fn version_of(command: &str, args: &[String], cwd: &Path) -> String {
    capture(command, args, cwd)
        .ok()
        .and_then(|text| text.lines().next().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn rustc_host(cwd: &Path) -> String {
    capture("rustc", &["-vV".to_owned()], cwd)
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// Prove this machine's toolchain is the one the key was computed for.
///
/// The compiler host and target are inputs to the planned build key, so an
/// archive compiled by the wrong toolchain still *verifies*: the consumer
/// compares against the key that was planned, not against what actually ran.
/// The only authority on what is about to run is the toolchain itself, which is
/// why this asks `rustc` rather than trusting the runner label that selected
/// the leg — a matrix that inferred compatibility from `runner.os` would hand
/// an `aarch64` consumer `x86_64` binaries and call it a pass.
///
/// ## Errors
///
/// Returns an error when `rustc` cannot be interrogated, when its host triple
/// is not the one the plan resolved, or when it has no standard library for the
/// resolved target. Every one of them is refused *before* the compile, so the
/// record's status names the preflight rather than a compile that never ran.
fn preflight_toolchain(identity: &Identity, workspace: &Path) -> Result<()> {
    let host = rustc_host(workspace);
    if host == "unknown" {
        bail!(
            "this producer has no usable rustc: `rustc -vV` reported no host triple, so nothing \
             here can be shown to match the plan's compiler host {}",
            identity.host
        );
    }
    if host != identity.host {
        bail!(
            "the plan resolved this build for compiler host {} and this producer's rustc is {}: \
             the host is an input to the build key, so compiling it here would hand every \
             consumer binaries of a toolchain the key does not describe",
            identity.host,
            host
        );
    }
    // A target equal to the host needs no second question: its standard library
    // is the one `rustc` just answered from.
    if !identity.target.is_empty() && identity.target != host {
        let libdir = capture(
            "rustc",
            &[
                "--print".to_owned(),
                "target-libdir".to_owned(),
                "--target".to_owned(),
                identity.target.clone(),
            ],
            workspace,
        )
        .with_context(|| {
            format!(
                "the plan resolved this build for target {} and this producer's rustc cannot \
                 describe it",
                identity.target
            )
        })?;
        // `--print target-libdir` answers for any recognized triple, installed
        // or not; only the directory says whether the standard library is here.
        if !Path::new(libdir.trim()).is_dir() {
            bail!(
                "the plan resolved this build for target {} and this producer's toolchain has no \
                 standard library for it at {}",
                identity.target,
                libdir.trim()
            );
        }
    }
    Ok(())
}

/// Bind a clean tracked checkout to the resolved commit. Never infer content
/// identity from a caller-supplied label or tolerate an unreadable Git tree.
fn source_tree(workspace: &Path, commit: &str) -> Result<String> {
    let git = |args: &[&str]| capture("git", &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(), workspace);
    let head = git(&["rev-parse", "HEAD"])?;
    if head != commit {
        bail!("build-source-mismatch: checkout {head} is not planned commit {commit}");
    }
    if !git(&["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        bail!("build-source-mismatch: tracked source differs from the planned commit; use a clean checkout");
    }
    git(&["rev-parse", &format!("{commit}^{{tree}}")])
}

/// What `nextest list --message-format json` says the archive contains.
pub struct Inventory {
    pub test_binaries: Vec<String>,
    /// Non-test binaries, build-script output directories, and linked paths —
    /// every payload class that is neither a test binary nor a declared
    /// include, reported so a later phase can see what an archive actually
    /// carries rather than inferring it.
    pub assets: Vec<String>,
}

fn parse_inventory(document: &Value) -> Result<Inventory> {
    let binaries = document
        .get("rust-binaries")
        .and_then(Value::as_object)
        .context("nextest's listing has no 'rust-binaries' object")?;
    let mut test_binaries: Vec<String> = binaries.keys().cloned().collect();
    test_binaries.sort();

    let mut assets = BTreeSet::new();
    if let Some(meta) = document.get("rust-build-meta") {
        if let Some(non_test) = meta.get("non-test-binaries").and_then(Value::as_object) {
            for entries in non_test.values() {
                for entry in entries.as_array().into_iter().flatten() {
                    if let Some(path) = entry.get("path").and_then(Value::as_str) {
                        assets.insert(path.to_owned());
                    }
                }
            }
        }
        for field in ["build-script-out-dirs"] {
            if let Some(map) = meta.get(field).and_then(Value::as_object) {
                for value in map.values() {
                    if let Some(path) = value.as_str() {
                        assets.insert(path.to_owned());
                    }
                }
            }
        }
        if let Some(paths) = meta.get("linked-paths").and_then(Value::as_array) {
            for path in paths {
                if let Some(path) = path.as_str() {
                    assets.insert(path.to_owned());
                }
            }
        }
    }
    Ok(Inventory {
        test_binaries,
        assets: assets.into_iter().collect(),
    })
}

/// List an archive's contents without running anything in it.
///
/// Extraction goes to `scratch`, which the caller discards: this is a read of
/// the archive, not the consumer's real extraction.
pub fn archive_inventory(
    nextest: &[String],
    archive: &Path,
    workspace: &Path,
    scratch: &Path,
) -> Result<Inventory> {
    fs::create_dir_all(scratch)
        .with_context(|| format!("creating {}", scratch.display()))?;
    let (command, head) = nextest.split_first().context("no nextest command")?;
    let mut args: Vec<String> = head.to_vec();
    args.extend(
        [
            "list",
            "--archive-file",
            &archive.to_string_lossy(),
            "--workspace-remap",
            &workspace.to_string_lossy(),
            "--extract-to",
            &scratch.to_string_lossy(),
            "--extract-overwrite",
            "--list-type",
            "binaries-only",
            "--message-format",
            "json",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    let text = capture(command, &args, workspace)?;
    // nextest prints its extraction progress on stderr and the document on
    // stdout, but a stray line has been observed ahead of it on some hosts;
    // take the last line that parses rather than assuming the whole stream is
    // one document.
    let document: Value = text
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str(line).ok())
        .context("nextest's listing was not JSON")?;
    parse_inventory(&document)
}

/// The measurement label one record's compiles are counted under.
///
/// Parallel to the tier cells' `L1 <environment>` / `check <environment>`
/// labels, so one run's compiler work reads as one list of named
/// configurations rather than an archive plus unattributed compiles.
fn measurement_label(producer: &str) -> String {
    format!("build {producer}")
}

/// Where one record's compiler events are written under `--counter-dir`.
fn measurement_dir(counter_dir: &Path, artifact: &str) -> PathBuf {
    counter_dir.join(artifact)
}

/// Point this record's Cargo/Nextest children at the compiler-work counter.
///
/// Command-scoped and plan-identified, which is the only wrapper the
/// specification permits: the value is set on the child commands this function
/// is measuring, never in `.cargo/config.toml` and never in the producer's own
/// environment. Without `--counter-dir` nothing is set at all, so the default
/// producer run is byte-identical to an uninstrumented one.
///
/// Each record counts into its own directory named for its artifact, because a
/// flat directory would fold two records compiled in one leg into each other's
/// manifests — and proving "this dependency compiled once across both records"
/// needs the two to stay separable.
///
/// ## Returns
///
/// The directory this record's events land in, or `None` when the run is
/// unmeasured.
///
/// ## Errors
///
/// Returns an error when the counter directory cannot be created or this
/// executable cannot name itself, because a measured run that silently records
/// nothing reads as proof of reuse.
fn measurement_env(
    options: &ProduceOptions,
    record: &BuildRecord,
    env: &mut Vec<(String, OsString)>,
) -> Result<Option<PathBuf>> {
    let Some(counter_dir) = options.counter_dir.as_deref() else {
        return Ok(None);
    };
    let dir = measurement_dir(counter_dir, &record.artifact);
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    // Absolute: the wrapper is re-entered from a Cargo child whose working
    // directory is the workspace, not this process's.
    let dir = canonical_path(&dir)?;
    let wrapper = std::env::current_exe().context(
        "a measured producer has to name its own executable to serve as the rustc wrapper",
    )?;
    env.push(("RUSTC_WRAPPER".to_owned(), wrapper.into_os_string()));
    env.push((super::WRAP_ENV.to_owned(), OsString::from("1")));
    env.push((
        super::COUNTER_DIR_ENV.to_owned(),
        OsString::from(dir.as_os_str()),
    ));
    env.push((
        super::PACKAGE_ENV.to_owned(),
        OsString::from(&record.package),
    ));
    env.push((
        super::CONFIGURATION_ENV.to_owned(),
        OsString::from(measurement_label(&record.producer)),
    ));
    Ok(Some(dir))
}

/// Build one record's archive, sidecars, and manifest.
fn produce_one(
    record: &BuildRecord,
    plan: &Plan,
    sidecars: &BTreeMap<String, Sidecar>,
    options: &ProduceOptions,
    inherited: &[toml::Value],
    tree: &str,
) -> Result<Manifest> {
    let started = Instant::now();
    let identity = &record.identity;

    // Absolute throughout: `--tool-config-file` requires an absolute path,
    // `--workspace-remap` is resolved by a consumer in another directory, and
    // every emitted file is looked up again from the manifest's own location.
    fs::create_dir_all(&options.out_dir)
        .with_context(|| format!("creating {}", options.out_dir.display()))?;
    let out_dir = canonical_path(&options.out_dir)?;
    let out_dir = out_dir.as_path();
    let workspace = canonical_path(&options.workspace)?;
    let workspace = workspace.as_path();
    let target_dir = match &options.target_dir {
        Some(path) if path.is_absolute() => path.clone(),
        Some(path) => workspace.join(path),
        None => workspace.join("target"),
    };

    let mut env: Vec<(String, OsString)> = vec![(
        "CARGO_TARGET_DIR".to_owned(),
        OsString::from(&target_dir),
    )];
    let observed_linker = runtime::linker(identity, workspace, &mut env)?;
    let events = measurement_env(options, record, &mut env)?;

    let config_path = out_dir.join(format!("{}.nextest.toml", record.artifact));
    fs::write(&config_path, tool_config(identity, inherited))
        .with_context(|| format!("writing {}", config_path.display()))?;

    let archive_path = out_dir.join(format!("{}.{}", record.artifact, identity.archive_format));

    // An include COPIES what the build produced, and `cargo nextest archive`
    // builds lib, bin, and test targets — never examples. A package that
    // declares one therefore has to have it built first, or the include errors
    // on a file that was never going to exist.
    for name in identity.archive_includes.iter().filter_map(|entry| example_target(entry)) {
        let mut build: Vec<String> = vec![
            "build".to_owned(),
            "--quiet".to_owned(),
            "--profile".to_owned(),
            identity.profile.clone(),
            "--package".to_owned(),
            identity.package.clone(),
            "--example".to_owned(),
            name,
        ];
        if !identity.target.is_empty() {
            build.push("--target".to_owned());
            build.push(identity.target.clone());
        }
        build.extend(identity.features.split_whitespace().map(str::to_owned));
        run(&options.cargo, &build, workspace, &env)?;
    }
    let setup_ms = elapsed_ms(started);

    let (command, head) = options.nextest.split_first().context("no nextest command")?;
    let mut args: Vec<String> = head.to_vec();
    args.extend(
        [
            "archive",
            "--package",
            &identity.package,
            "--archive-file",
            &archive_path.to_string_lossy(),
            "--tool-config-file",
            &format!("ci-build:{}", config_path.display()),
            "--cargo-profile",
            &identity.profile,
            "--profile",
            ARCHIVE_PROFILE,
        ]
        .into_iter()
        .map(str::to_owned),
    );
    if !identity.target.is_empty() {
        args.push("--target".to_owned());
        args.push(identity.target.clone());
    }
    for entry in &identity.cargo_config {
        args.push("--config".to_owned());
        args.push(entry.clone());
    }
    args.extend(identity.features.split_whitespace().map(str::to_owned));

    let compile_started = Instant::now();
    run(command, &args, workspace, &env)?;
    let compile_archive_ms = elapsed_ms(compile_started);

    // Sidecars
    let sidecar_started = Instant::now();
    let sidecar_dir = out_dir.join(format!("{}-sidecars", record.artifact));
    let mut emitted = Vec::new();
    if !identity.sidecars.is_empty() {
        fs::create_dir_all(&sidecar_dir)
            .with_context(|| format!("creating {}", sidecar_dir.display()))?;
    }
    for name in &identity.sidecars {
        let spec = sidecars.get(name).with_context(|| {
            format!("sidecar '{name}' is declared by {} but is not in the sidecar table", identity.package)
        })?;
        let mut build: Vec<String> = vec![
            "build".to_owned(),
            "--quiet".to_owned(),
            "--profile".to_owned(),
            identity.profile.clone(),
            "--package".to_owned(),
            spec.package.clone(),
        ];
        if !identity.target.is_empty() {
            build.push("--target".to_owned());
            build.push(identity.target.clone());
        }
        for feature in &spec.features {
            build.push("--features".to_owned());
            build.push(feature.clone());
        }
        for bin in &spec.bins {
            build.push("--bin".to_owned());
            build.push(bin.clone());
        }
        run(&options.cargo, &build, workspace, &env)?;

        for bin in &spec.bins {
            let file = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
            let mut built = target_dir.clone();
            if !identity.target.is_empty() {
                built.push(&identity.target);
            }
            built.push(profile_dir(&identity.profile));
            built.push(&file);
            let destination = sidecar_dir.join(&file);
            fs::copy(&built, &destination).with_context(|| {
                format!(
                    "copying the '{name}' sidecar {} to {}",
                    built.display(),
                    destination.display()
                )
            })?;
            emitted.push(NamedFile {
                name: name.clone(),
                record: file_record(out_dir, &destination)?,
            });
        }
    }
    emitted.sort_by(|left, right| {
        (&left.name, &left.record.file).cmp(&(&right.name, &right.record.file))
    });
    let sidecars_ms = elapsed_ms(sidecar_started);

    // Declared archive includes, digested where they sat before archiving.
    let mut runtime_assets = Vec::new();
    for entry in &identity.archive_includes {
        let relative = include_path(identity, entry);
        let path = target_dir.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        let mut record = file_record(&target_dir, &path).with_context(|| {
            format!("the declared archive include '{entry}' was not produced")
        })?;
        record.file = relative;
        runtime_assets.push(NamedFile {
            name: entry.clone(),
            record,
        });
    }

    let inventory_started = Instant::now();
    let scratch = out_dir.join(format!("{}-inventory", record.artifact));
    let inventory = archive_inventory(&options.nextest, &archive_path, workspace, &scratch)?;
    let observed_runtime = runtime::observe(&[scratch.clone(), sidecar_dir.clone()]);
    let _ = fs::remove_dir_all(&scratch);
    let observed_runtime = observed_runtime?;
    let inventory_ms = elapsed_ms(inventory_started);

    let checksum_started = Instant::now();
    let archive = file_record(out_dir, &archive_path)?;
    let checksum_ms = elapsed_ms(checksum_started);

    for asset in inventory.assets {
        if runtime_assets.iter().any(|entry| entry.record.file == asset) {
            continue;
        }
        runtime_assets.push(NamedFile {
            name: asset.clone(),
            // A build-script output *directory* has no single size or digest;
            // the archive's own checksum covers its contents. Zeroes here mean
            // "carried by the archive", never "empty file".
            record: FileRecord {
                file: asset,
                bytes: 0,
                blake3: String::new(),
            },
        });
    }
    runtime_assets.sort_by(|left, right| left.record.file.cmp(&right.record.file));

    let compiler_work = events
        .as_deref()
        .map(super::read_events)
        .transpose()?
        .map(|events| serde_json::to_value(super::summarize(&events)))
        .transpose()?;

    if source_tree(workspace, &plan.head)? != tree {
        bail!("build-source-mismatch: source changed during production");
    }
    let mut manifest = Manifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        key: record.key.clone(),
        digest: String::new(),
        package: record.package.clone(),
        producer: record.producer.clone(),
        artifact: record.artifact.clone(),
        compatible_environments: record.compatible_environments.clone(),
        source_commit: plan.head.clone(),
        source_tree: tree.to_owned(),
        producer_workspace: workspace
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/"),
        identity: identity.clone(),
        realized: Realized {
            rustc: version_of("rustc", &["--version".to_owned()], workspace),
            cargo: version_of(&options.cargo, &["--version".to_owned()], workspace),
            nextest: version_of(
                command,
                &head
                    .iter()
                    .cloned()
                    .chain(std::iter::once("--version".to_owned()))
                    .collect::<Vec<_>>(),
                workspace,
            ),
            host: rustc_host(workspace),
            target: if identity.target.is_empty() {
                rustc_host(workspace)
            } else {
                identity.target.clone()
            },
            linker: observed_linker,
            runtime: observed_runtime,
        },
        archive,
        sidecars: emitted,
        runtime_assets,
        test_binaries: inventory.test_binaries,
        compiler_work,
        timings: Timings {
            setup_ms,
            compile_archive_ms,
            sidecars_ms,
            inventory_ms,
            checksum_ms,
            total_ms: elapsed_ms(started),
        },
    };
    manifest.digest = realized_digest(&manifest);

    let manifest_path = out_dir.join(format!("{}.manifest.json", record.artifact));
    fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )
    .with_context(|| format!("writing {}", manifest_path.display()))?;
    Ok(manifest)
}

fn elapsed_ms(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// One planned key's outcome, written whether it compiled or not.
///
/// The manifest is the *successful* producer's statement. A record that never
/// reached one still owes its consumers an account of why, and a rollup that
/// can only see "no artifact" cannot tell a compile failure from a key that was
/// never scheduled. `stage` names where it stopped so a reader is sent to the
/// right log; `digest` and `timings` appear when the producer got far enough to
/// have them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildStatus {
    pub schema_version: u32,
    pub key: String,
    pub package: String,
    pub producer: String,
    pub artifact: String,
    /// `success` or `failure`. `cancelled` is reachable only from the workflow,
    /// which synthesizes it for a key this tool never reported on.
    pub result: String,
    /// `preflight`, `produce`, `compile`, or — from the workflow — `upload`.
    pub stage: String,
    pub consumers: Vec<Consumer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings: Option<Timings>,
}

impl BuildStatus {
    fn of(record: &BuildRecord, result: &str, stage: &str) -> Self {
        Self {
            schema_version: BUILD_STATUS_SCHEMA_VERSION,
            key: record.key.clone(),
            package: record.package.clone(),
            producer: record.producer.clone(),
            artifact: record.artifact.clone(),
            result: result.to_owned(),
            stage: stage.to_owned(),
            consumers: record.consumers.clone(),
            digest: None,
            detail: None,
            timings: None,
        }
    }
}

/// Where a produced record's status document is written.
pub fn status_path(out_dir: &Path, artifact: &str) -> PathBuf {
    out_dir.join(format!("{artifact}.status.json"))
}

/// Produce every archive this producer owns, in deterministic order.
///
/// Each record is attempted independently and every one leaves a status
/// document behind, so a failing key neither hides its own reason nor stops an
/// unrelated key from completing (Task 4.1).
///
/// ## Errors
///
/// Returns `Ok` with one entry per record; the caller decides the exit code
/// from the statuses. Only a failure to read the plan, the sidecar table, or
/// the repository's own nextest config — which is common to every record and
/// therefore not attributable to one — is an error.
pub fn run_produce(
    options: &ProduceOptions,
    term: &Terminal,
) -> Result<Vec<(BuildStatus, Option<Manifest>)>> {
    let plan = read_plan(&options.plan)?;
    let sidecar_path = options
        .sidecars
        .clone()
        .unwrap_or_else(|| options.workspace.join(".github/ci/sidecars.json"));
    let sidecars = read_sidecars(&sidecar_path)?;
    let inherited = inherited_includes(&options.workspace)?;
    let tree = source_tree(&options.workspace, &plan.head)?;
    if options.source_tree.as_ref().is_some_and(|expected| expected != &tree) {
        bail!("build-source-mismatch: --source-tree differs from the observed checkout");
    }

    let mut mine: Vec<&BuildRecord> = plan
        .builds
        .iter()
        .filter(|record| record.producer == options.producer)
        .filter(|record| options.key.as_ref().is_none_or(|key| &record.key == key))
        .collect();
    mine.sort_by(|left, right| (&left.package, &left.key).cmp(&(&right.package, &right.key)));

    if let (true, Some(key)) = (mine.is_empty(), &options.key) {
        bail!(
            "the plan has no build {key} owned by {}; the owner matrix and the plan disagree",
            options.producer
        );
    }

    fs::create_dir_all(&options.out_dir)
        .with_context(|| format!("creating {}", options.out_dir.display()))?;

    let mut outcomes = Vec::new();
    for record in mine {
        // Per record, not once per leg: two records owned by one producer can
        // name different targets, and a refusal must be attributable to the key
        // it refused.
        let outcome = if let Err(error) = preflight_toolchain(&record.identity, &options.workspace)
        {
            let mut status = BuildStatus::of(record, "failure", "preflight");
            status.detail = Some(format!("{error:#}"));
            (status, None)
        } else {
            match produce_one(record, &plan, &sidecars, options, &inherited, &tree) {
                Ok(manifest) => {
                    let mut status = BuildStatus::of(record, "success", "produce");
                    status.digest = Some(manifest.digest.clone());
                    status.timings = Some(manifest.timings.clone());
                    (status, Some(manifest))
                }
                Err(error) => {
                    let mut status = BuildStatus::of(record, "failure", "compile");
                    status.detail = Some(format!("{error:#}"));
                    (status, None)
                }
            }
        };
        let path = status_path(&options.out_dir, &record.artifact);
        fs::write(
            &path,
            format!("{}\n", serde_json::to_string_pretty(&outcome.0)?),
        )
        .with_context(|| format!("writing {}", path.display()))?;
        outcomes.push(outcome);
    }

    if options.json {
        let document: Vec<Value> = outcomes
            .iter()
            .map(|(status, manifest)| {
                json!({"status": status, "manifest": manifest})
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&document)?);
    } else {
        print!("{}", render_produced(&outcomes, &options.producer, term));
    }
    Ok(outcomes)
}

fn render_produced(
    outcomes: &[(BuildStatus, Option<Manifest>)],
    producer: &str,
    term: &Terminal,
) -> String {
    let mut out = String::new();
    if outcomes.is_empty() {
        out.push_str(
            &Prose::new(format!(
                "**No build records for `{producer}`.** An all-reused plan schedules no owner; \
                 this is the expected shape of a run that pays for no compilation."
            ))
            .render(term),
        );
        out.push('\n');
        return out;
    }
    let failed = outcomes
        .iter()
        .filter(|(status, _)| status.result != "success")
        .count();
    out.push_str(
        &Prose::new(if failed == 0 {
            format!(
                "**Produced {} archive(s)** on `{producer}`.",
                outcomes.len()
            )
        } else {
            format!(
                "**{} of {} build(s) failed** on `{producer}`. Each key is attempted \
                 on its own, so the rows below report every one; a failed key blocks \
                 only the cells that named it.",
                failed,
                outcomes.len()
            )
        })
        .render(term),
    );
    out.push('\n');
    let columns = ["Package", "Key", "Result", "Digest", "Binaries", "Bytes"]
        .into_iter()
        .map(TableColumn::new)
        .collect::<Vec<_>>();
    let data = outcomes
        .iter()
        .map(|(status, manifest)| {
            vec![
                status.package.clone().into(),
                status.key.clone().into(),
                status.result.clone().into(),
                manifest
                    .as_ref()
                    .map(|entry| entry.digest.clone())
                    .unwrap_or_else(|| status.stage.clone())
                    .into(),
                manifest
                    .as_ref()
                    .map(|entry| entry.test_binaries.len().to_string())
                    .unwrap_or_else(|| "-".to_owned())
                    .into(),
                manifest
                    .as_ref()
                    .map(|entry| entry.archive.bytes.to_string())
                    .unwrap_or_else(|| "-".to_owned())
                    .into(),
            ]
        })
        .collect::<Vec<_>>();
    out.push_str(&Table::new().with_columns(columns).with_data(data).render(term));
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// Verifying
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rejection {
    pub code: String,
    pub detail: String,
}

impl Rejection {
    pub fn new(code: &str, detail: impl Into<String>) -> Self {
        debug_assert!(
            REJECTIONS.contains(&code),
            "{code} is not in the rejection vocabulary"
        );
        Self {
            code: code.to_owned(),
            detail: detail.into(),
        }
    }
}

/// What one consumer's pre-execution stages cost, in the order they ran.
///
/// The reporting contract requires the consumer's extraction to be reported
/// apart from its test execution, and `cargo nextest run --archive-file`
/// extracts inside the run. `extract_ms` is the verifier's own extraction of
/// the same archive on the same host — a full `--extract-to` of every binary,
/// performed because listing the inventory requires it — so the stage has a
/// measured cost of its own rather than one folded into test time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct VerifyTimings {
    /// Identity, compatibility, and every archive/sidecar checksum.
    pub identity_ms: u64,
    /// Extracting the archive and listing what came out.
    pub extract_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Verdict {
    pub schema_version: u32,
    pub accepted: bool,
    pub environment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
    /// Whether the archive's own contents were listed and compared. False when
    /// no workspace was supplied, which a consumer job must never do.
    pub inventory_checked: bool,
    #[serde(default)]
    pub timings: VerifyTimings,
    pub rejections: Vec<Rejection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyOptions {
    pub manifest: PathBuf,
    pub environment: String,
    pub plan: Option<PathBuf>,
    /// Where the emitted files actually sit, when that is not the manifest's
    /// own directory. Every `FileRecord.file` is resolved against it.
    pub root: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
    pub nextest: Vec<String>,
    pub json: bool,
    /// Where to write the verdict document in addition to rendering it.
    ///
    /// A consumer job needs both: the rendered verdict is what a reader sees in
    /// the log, and the document is where its stage timings reach the cell's
    /// status artifact without a second verification run.
    pub verdict_out: Option<PathBuf>,
}

/// Check one emitted file against its record.
fn check_file(root: &Path, record: &FileRecord, missing: &str, corrupt: &str) -> Vec<Rejection> {
    let path = root.join(record.file.replace('/', std::path::MAIN_SEPARATOR_STR));
    let observed = match file_record(root, &path) {
        Ok(observed) => observed,
        Err(error) => {
            return vec![Rejection::new(missing, format!("{}: {error:#}", record.file))];
        }
    };
    let mut problems = Vec::new();
    if observed.bytes != record.bytes {
        problems.push(Rejection::new(
            corrupt,
            format!(
                "{}: manifest declares {} byte(s), found {}",
                record.file, record.bytes, observed.bytes
            ),
        ));
    }
    if observed.blake3 != record.blake3 {
        problems.push(Rejection::new(
            corrupt,
            format!(
                "{}: manifest declares blake3 {}, found {}",
                record.file, record.blake3, observed.blake3
            ),
        ));
    }
    problems
}

/// The host facts a compiled consumer knows about itself.
///
/// Read from `cfg!`, not from a runner label: Task 5.1 requires the actual
/// execution platform to be validated against the plan rather than inferred
/// from `runner.os`, and a `cfg` is the one answer that cannot be mislabeled.
pub fn host_runtime() -> (&'static str, &'static str, &'static str) {
    let abi = if cfg!(target_env = "msvc") {
        "msvc"
    } else if cfg!(target_env = "musl") {
        "musl"
    } else if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
        "darwin"
    } else {
        "gnu"
    };
    let libc = match abi {
        "msvc" => "msvc",
        "musl" => "musl",
        "darwin" => "libSystem",
        _ => "glibc",
    };
    (std::env::consts::ARCH, abi, libc)
}

/// Everything wrong with one consumer's inputs, in the order a reader should
/// see it: identity first, then integrity, then contents.
pub fn verify(
    manifest: &Manifest,
    options: &VerifyOptions,
    plan: Option<&Plan>,
    host: (&str, &str, &str),
) -> Vec<Rejection> {
    let mut problems = Vec::new();

    let recomputed = realized_digest(manifest);
    if recomputed != manifest.digest {
        problems.push(Rejection::new(
            "build-digest-mismatch",
            format!(
                "manifest declares digest {}, its own fields hash to {recomputed}",
                manifest.digest
            ),
        ));
    }

    if let Some(plan) = plan {
        match plan
            .builds
            .iter()
            .find(|record| record.key == manifest.key)
        {
            None => problems.push(Rejection::new(
                "build-key-mismatch",
                format!("the plan has no build record for key {}", manifest.key),
            )),
            Some(record) => {
                if record.identity != manifest.identity {
                    problems.push(Rejection::new(
                        "build-key-mismatch",
                        format!(
                            "build {} was produced from a different identity than the plan declares",
                            manifest.key
                        ),
                    ));
                }
                if record.package != manifest.package || record.producer != manifest.producer {
                    problems.push(Rejection::new(
                        "build-key-mismatch",
                        format!(
                            "build {} is {}/{} in the plan and {}/{} in the manifest",
                            manifest.key,
                            record.package,
                            record.producer,
                            manifest.package,
                            manifest.producer
                        ),
                    ));
                }
                if record.artifact != manifest.artifact {
                    problems.push(Rejection::new(
                        "build-key-mismatch",
                        format!(
                            "build {} is artifact {} in the plan and {} in the manifest",
                            manifest.key, record.artifact, manifest.artifact
                        ),
                    ));
                }
            }
        }
        if let Some(workspace) = options.workspace.as_deref() {
            match source_tree(workspace, &plan.head) {
                Ok(tree) if tree == manifest.source_tree => {},
                Ok(tree) => problems.push(Rejection::new("build-source-mismatch",
                    format!("archive tree {} differs from checkout tree {tree}", manifest.source_tree))),
                Err(error) => problems.push(Rejection::new("build-source-mismatch", format!("{error:#}"))),
            }
        }
        if plan.head != manifest.source_commit {
            problems.push(Rejection::new(
                "build-source-mismatch",
                format!(
                    "the archive was built at {} and the plan resolves {}",
                    manifest.source_commit, plan.head
                ),
            ));
        }
        if let Some(expected) = plan.runtimes.get(&options.environment) {
            let observed = &manifest.realized.runtime;
            if expected.arch != observed.arch
                || expected.abi != observed.abi
                || expected.libc != observed.libc
            {
                problems.push(Rejection::new(
                    "build-runtime-incompatible",
                    format!(
                        "{} executes {}/{}/{}; this archive was built for {}/{}/{}",
                        options.environment,
                        expected.arch,
                        expected.abi,
                        expected.libc,
                        observed.arch,
                        observed.abi,
                        observed.libc
                    ),
                ));
            }

        }
    }

    if !manifest
        .compatible_environments
        .iter()
        .any(|name| name == &options.environment)
    {
        problems.push(Rejection::new(
            "build-environment-incompatible",
            format!(
                "{} is not one of this build's compatible environments ({})",
                options.environment,
                manifest.compatible_environments.join(", ")
            ),
        ));
    }

    let (arch, abi, libc) = host;
    let declared = &manifest.realized.runtime;
    if !declared.arch.is_empty()
        && (declared.arch != arch || declared.abi != abi || declared.libc != libc)
    {
        problems.push(Rejection::new(
            "build-runtime-incompatible",
            format!(
                "this host is {arch}/{abi}/{libc}; the archive was built for {}/{}/{}",
                declared.arch, declared.abi, declared.libc
            ),
        ));
    }

    let root = options
        .root
        .clone()
        .or_else(|| options.manifest.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    problems.extend(check_file(
        &root,
        &manifest.archive,
        "build-archive-missing",
        "build-archive-corrupt",
    ));
    for sidecar in &manifest.sidecars {
        problems.extend(check_file(
            &root,
            &sidecar.record,
            "build-sidecar-missing",
            "build-sidecar-corrupt",
        ));
    }
    if manifest.identity.sidecars.len() > manifest.sidecars.len() {
        problems.push(Rejection::new(
            "build-sidecar-missing",
            format!(
                "the plan declares sidecar(s) {} and the manifest emitted {} file(s)",
                manifest.identity.sidecars.join(", "),
                manifest.sidecars.len()
            ),
        ));
    }
    for entry in &manifest.identity.archive_includes {
        if !manifest
            .runtime_assets
            .iter()
            .any(|asset| &asset.name == entry)
        {
            problems.push(Rejection::new(
                "build-asset-missing",
                format!("the declared archive include '{entry}' is not in the manifest"),
            ));
        }
    }

    problems
}

/// Compare the archive's real contents with the inventory the manifest claims.
fn check_inventory(manifest: &Manifest, options: &VerifyOptions, archive: &Path) -> Vec<Rejection> {
    let Some(workspace) = options.workspace.as_deref() else {
        return Vec::new();
    };
    let scratch = std::env::temp_dir().join(format!(
        "ci-build-verify-{}-{}",
        std::process::id(),
        manifest.key
    ));
    let listed = archive_inventory(&options.nextest, archive, workspace, &scratch);
    let root = options.root.as_deref().unwrap_or_else(|| options.manifest.parent().unwrap());
    let mut roots = vec![scratch.clone()];
    roots.extend(manifest.sidecars.iter().filter_map(|sidecar| root.join(&sidecar.record.file).parent().map(Path::to_path_buf)));
    roots.sort();
    roots.dedup();
    let observed = listed.as_ref().ok().map(|_| runtime::observe(&roots));
    let _ = fs::remove_dir_all(&scratch);
    if let Some(observed) = observed {
        match observed {
            Ok(runtime) if runtime == manifest.realized.runtime => {},
            Ok(runtime) => return vec![Rejection::new("build-runtime-incompatible",
                format!("observed native libraries {:?} differ from producer {:?}", runtime.native_libraries, manifest.realized.runtime.native_libraries))],
            Err(error) => return vec![Rejection::new("build-runtime-incompatible", format!("{error:#}"))],
        }
    }
    let inventory = match listed {
        Ok(inventory) => inventory,
        Err(error) => {
            return vec![Rejection::new(
                "build-archive-corrupt",
                format!("the archive could not be listed: {error:#}"),
            )];
        }
    };
    let expected: BTreeSet<&str> = manifest
        .test_binaries
        .iter()
        .map(String::as_str)
        .collect();
    let observed: BTreeSet<&str> = inventory
        .test_binaries
        .iter()
        .map(String::as_str)
        .collect();
    let mut problems = Vec::new();
    let missing: Vec<&str> = expected.difference(&observed).copied().collect();
    if !missing.is_empty() {
        problems.push(Rejection::new(
            "build-inventory-incomplete",
            format!("the archive is missing {}", missing.join(", ")),
        ));
    }
    let extra: Vec<&str> = observed.difference(&expected).copied().collect();
    if !extra.is_empty() {
        problems.push(Rejection::new(
            "build-inventory-unexpected",
            format!("the archive also contains {}", extra.join(", ")),
        ));
    }
    problems
}

/// Verify one consumer's inputs and render the verdict.
///
/// ## Returns
///
/// `true` when every check passed. The caller turns that into the process exit
/// status; a rejection is not an `Err`, because it is a *result* — an
/// infrastructure verdict a workflow reports — rather than a tool failure.
pub fn run_verify(options: &VerifyOptions, term: &Terminal) -> Result<bool> {
    let started = Instant::now();
    let manifest = match read_manifest(&options.manifest) {
        Ok(manifest) => manifest,
        Err(rejection) => {
            let verdict = Verdict {
                schema_version: VERDICT_SCHEMA_VERSION,
                accepted: false,
                environment: options.environment.clone(),
                key: None,
                digest: None,
                package: None,
                producer: None,
                inventory_checked: false,
                timings: VerifyTimings {
                    identity_ms: elapsed_ms(started),
                    extract_ms: 0,
                    total_ms: elapsed_ms(started),
                },
                rejections: vec![rejection],
            };
            emit(&verdict, options, term)?;
            return Ok(false);
        }
    };

    let plan = options.plan.as_deref().map(read_plan).transpose()?;
    let mut rejections = verify(&manifest, options, plan.as_ref(), host_runtime());
    let identity_ms = elapsed_ms(started);

    let root = options
        .root
        .clone()
        .or_else(|| options.manifest.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    let archive = root.join(
        manifest
            .archive
            .file
            .replace('/', std::path::MAIN_SEPARATOR_STR),
    );
    // Only when the archive itself is intact: listing a corrupt archive adds a
    // second, derived complaint about the same defect.
    let inventory_checked = options.workspace.is_some()
        && !rejections
            .iter()
            .any(|entry| entry.code.starts_with("build-archive-"));
    let extraction = Instant::now();
    if inventory_checked {
        rejections.extend(check_inventory(&manifest, options, &archive));
    }

    let verdict = Verdict {
        schema_version: VERDICT_SCHEMA_VERSION,
        accepted: rejections.is_empty(),
        environment: options.environment.clone(),
        key: Some(manifest.key.clone()),
        digest: Some(manifest.digest.clone()),
        package: Some(manifest.package.clone()),
        producer: Some(manifest.producer.clone()),
        inventory_checked,
        timings: VerifyTimings {
            identity_ms,
            extract_ms: if inventory_checked {
                elapsed_ms(extraction)
            } else {
                0
            },
            total_ms: elapsed_ms(started),
        },
        rejections,
    };
    emit(&verdict, options, term)?;
    Ok(verdict.accepted)
}

fn emit(verdict: &Verdict, options: &VerifyOptions, term: &Terminal) -> Result<()> {
    if options.json {
        println!("{}", serde_json::to_string_pretty(verdict)?);
    } else {
        print!("{}", render_verdict(verdict, term));
    }
    // Written even for a refusal: a rejected consumer still reports how long
    // its transfer and verification took, and a cell that never started is
    // exactly the one whose stage costs a reader wants to see.
    if let Some(path) = options.verdict_out.as_deref() {
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(path, format!("{}\n", serde_json::to_string_pretty(verdict)?))
            .with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

fn render_verdict(verdict: &Verdict, term: &Terminal) -> String {
    let mut out = String::new();
    if verdict.accepted {
        out.push_str(
            &Prose::new(format!(
                "**Accepted** build `{}` for `{}` — digest `{}`, produced on `{}`.",
                verdict.key.as_deref().unwrap_or("?"),
                verdict.environment,
                verdict.digest.as_deref().unwrap_or("?"),
                verdict.producer.as_deref().unwrap_or("?")
            ))
            .render(term),
        );
        out.push('\n');
        if !verdict.inventory_checked {
            out.push_str(
                &Prose::new(
                    "_The archive's own contents were not listed: no workspace was supplied. \
                     A CI consumer always supplies one._",
                )
                .render(term),
            );
            out.push('\n');
        }
        return out;
    }

    out.push_str(
        &Prose::new(format!(
            "**Refused** the build inputs for `{}`. Nothing is compiled to replace them — a \
             consumer that built its own binaries would be reporting on a different program \
             than the one under test.",
            verdict.environment
        ))
        .render(term),
    );
    out.push('\n');
    let columns = ["Code", "Detail"]
        .into_iter()
        .map(TableColumn::new)
        .collect::<Vec<_>>();
    let data = verdict
        .rejections
        .iter()
        .map(|rejection| vec![rejection.code.clone().into(), rejection.detail.clone().into()])
        .collect::<Vec<_>>();
    out.push_str(&Table::new().with_columns(columns).with_data(data).render(term));
    out.push('\n');
    out
}

#[cfg(test)]
#[path = "ci-build-archive-tests.rs"]
mod tests;
