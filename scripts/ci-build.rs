//! Count the compiler work a CI cell actually performs.
//!
//! `fixes/2026-09-12-single-os-compile/spec.md` claims that L1, L2, and browser
//! cells recompile the same test programs. A warm `Swatinem/rust-cache` restore
//! makes a rebuilt job look fast, so wall-clock time cannot distinguish "reused
//! the producer's binaries" from "recompiled them quickly". The only honest
//! measurement is how many times rustc ran, which is what this binary records.
//!
//! ## Wrapper re-entry
//!
//! Cargo invokes `$RUSTC_WRAPPER <rustc> <args…>`, with no room for a
//! subcommand. Rather than ship a shell script that native Windows cannot run,
//! the binary re-enters itself: when `BISCUIT_CI_BUILD_WRAP` is set to a
//! non-empty value, `argv[1..]` is the rustc command line and everything else
//! here is skipped. A measured step therefore sets `RUSTC_WRAPPER`,
//! `BISCUIT_CI_BUILD_WRAP`, and `BISCUIT_CI_BUILD_COUNTER_DIR` together, in one
//! step's `env:` block. `RUSTC_WRAPPER` is empty everywhere else — mixing
//! wrapped and unwrapped builds in one `target/` is a documented repository
//! hazard, and a host-global wrapper would poison every later build.
//!
//! ## Notes
//!
//! Behind the `local-tools` feature because it links `biscuit-terminal` and
//! `biscuit-hash`. `ci-rollup`, the always-runs merge-gate binary, links none
//! of the monorepo's crates and must stay that way.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use biscuit_hash::xx_hash;
use biscuit_terminal::prelude::{
    Prose, Table, TableColumn, Terminal, TerminalRenderable, UnorderedList,
};
use serde::{Deserialize, Serialize};

#[path = "ci-build-archive.rs"]
mod archive;

use archive::{ProduceOptions, VerifyOptions};

/// Bumped whenever an event file's field set changes. A report refuses a
/// generation it does not understand rather than under-counting silently.
const EVENT_SCHEMA_VERSION: u32 = 1;

/// Bumped whenever the `--json` report's field set changes. This is the
/// versioned machine interface workflow status artifacts embed.
const REPORT_SCHEMA_VERSION: u32 = 1;

/// Bumped whenever the `key` request or response field set changes. The
/// planner refuses a generation it does not understand rather than writing a
/// build key nothing else can reproduce.
const KEY_SCHEMA_VERSION: u32 = 1;

const COUNTER_DIR_ENV: &str = "BISCUIT_CI_BUILD_COUNTER_DIR";
const WRAP_ENV: &str = "BISCUIT_CI_BUILD_WRAP";
const PACKAGE_ENV: &str = "BISCUIT_CI_BUILD_PACKAGE";
const CONFIGURATION_ENV: &str = "BISCUIT_CI_BUILD_CONFIGURATION";

const USAGE: &str = "\
usage:
  ci-build report  [--counter-dir <dir>] [--json]
  ci-build reset   [--counter-dir <dir>]
  ci-build key     [--input <file>|-]
  ci-build produce --plan <file> --producer <env> --out-dir <dir>
                   [--key <key>] [--workspace <dir>] [--target-dir <dir>] [--sidecars <file>]
                   [--source-tree <id>] [--counter-dir <dir>] [--cargo <cmd>]
                   [--nextest <cmd>] [--json]
  ci-build verify  --manifest <file> --environment <env>
                   [--plan <file>] [--root <dir>] [--workspace <dir>]
                   [--nextest <cmd>] [--json] [--verdict-out <file>]

  As a rustc wrapper, set BISCUIT_CI_BUILD_WRAP=1 and RUSTC_WRAPPER=<this bin>
  for ONE measured command; argv[1..] is then the rustc command line.

environment:
  BISCUIT_CI_BUILD_COUNTER_DIR   where event files are written (required)
  BISCUIT_CI_BUILD_WRAP          non-empty selects wrapper re-entry
  BISCUIT_CI_BUILD_PACKAGE       measurement label for the package under test
  BISCUIT_CI_BUILD_CONFIGURATION measurement label for the configuration
";

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// One rustc invocation, as observed from inside the wrapper.
///
/// `package` and `configuration` are the *measurement* labels supplied by the
/// step being measured — which cell's compile window this invocation belongs
/// to. `cargo_package` is what Cargo said it was compiling, which for a
/// dependency is not the package under test. Keeping both is what lets a report
/// say "this cell's build ran rustc 412 times, 9 of them for its own crates".
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Event {
    schema_version: u32,
    package: String,
    configuration: String,
    #[serde(default)]
    crate_name: Option<String>,
    #[serde(default)]
    cargo_package: Option<String>,
    /// Cargo sets `CARGO_PRIMARY_PACKAGE` only for the workspace members the
    /// command selected, so this separates the package's own crates from the
    /// dependency closure compiled underneath it.
    primary: bool,
    #[serde(default)]
    crate_types: Vec<String>,
    #[serde(default)]
    target: Option<String>,
    /// Cargo's version and target-information queries produce no package
    /// artifacts and are excluded from compiler-work counts.
    probe: bool,
    argv_digest: String,
    started_ms: u128,
    finished_ms: u128,
    duration_ms: u64,
    #[serde(default)]
    exit_code: Option<i32>,
    pid: u32,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default()
}

/// The value of `flag`, whether spelled `--flag value` or `--flag=value`.
fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let prefix = format!("{flag}=");
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(rest) = arg.strip_prefix(&prefix) {
            return Some(rest.to_owned());
        }
        if arg == flag {
            return iter.next().cloned();
        }
    }
    None
}

/// Every `--crate-type` value, in command-line order.
///
/// rustc accepts the flag more than once and also accepts a comma-separated
/// list, so a single `arg_value` lookup would report `lib` for a
/// `--crate-type lib --crate-type cdylib` invocation.
fn crate_types(args: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let raw = if let Some(rest) = arg.strip_prefix("--crate-type=") {
            Some(rest.to_owned())
        } else if arg == "--crate-type" {
            iter.next().cloned()
        } else {
            None
        };
        if let Some(raw) = raw {
            found.extend(raw.split(',').map(str::to_owned));
        }
    }
    found
}

fn is_probe(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "-vV" || arg == "--version" || arg == "-V")
        || (arg_value(args, "--crate-name").as_deref() == Some("___")
            && args.iter().any(|arg| arg == "-")
            && args.iter().any(|arg| arg == "--print" || arg.starts_with("--print="))
            && !args.iter().any(|arg| arg == "--emit" || arg.starts_with("--emit=")))
}

/// The digest that makes an event file name collision-free.
///
/// xxHash through `biscuit-hash`, never a second implementation: the planned
/// build key in Phase 2 is computed through this same boundary, and two hashing
/// implementations in one contract is how digests silently diverge.
fn argv_digest(command: &str, args: &[String], package: &str, configuration: &str) -> String {
    let mut material = String::with_capacity(128);
    material.push_str(package);
    material.push('\u{1f}');
    material.push_str(configuration);
    material.push('\u{1f}');
    material.push_str(command);
    for arg in args {
        material.push('\u{1f}');
        material.push_str(arg);
    }
    format!("{:016x}", xx_hash(&material))
}

/// Write `event` into `dir` under a name no concurrent writer can take.
///
/// Cargo runs many rustc processes at once, so the counter must not need a
/// lock. `{nanos}-{pid}-{digest}` is already unique in practice; the file is
/// still created with `create_new`, and a collision retries with a suffix
/// rather than overwriting another process's measurement.
fn write_event(dir: &Path, event: &Event) -> Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let body = serde_json::to_string(event)?;
    let stem = format!(
        "{:039}-{}-{}",
        event.started_ms, event.pid, event.argv_digest
    );
    for attempt in 0..64u32 {
        let name = if attempt == 0 {
            format!("{stem}.json")
        } else {
            format!("{stem}-{attempt}.json")
        };
        let path = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(body.as_bytes())?;
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| format!("writing {}", path.display()));
            }
        }
    }
    bail!("could not find a free event file name in {}", dir.display())
}

// ---------------------------------------------------------------------------
// Wrapper mode
// ---------------------------------------------------------------------------

/// Run the wrapped compiler and record one event.
///
/// The compiler's exit status is the only thing that reaches Cargo. A counter
/// that cannot write its event must not turn a green build red, so recording
/// failures are reported on stderr and otherwise ignored — an under-counted
/// measurement is recoverable, a broken build is not.
fn run_wrapper(argv: Vec<String>) -> Result<i32> {
    let mut parts = argv.into_iter();
    let command = parts.next().context("rustc wrapper invoked with no command")?;
    let args: Vec<String> = parts.collect();

    let started_ms = now_ms();
    let status = Command::new(&command)
        .args(&args)
        .status()
        .with_context(|| format!("running {command}"))?;
    let finished_ms = now_ms();

    if let Some(dir) = counter_dir_from_env() {
        let event = Event {
            schema_version: EVENT_SCHEMA_VERSION,
            package: env::var(PACKAGE_ENV).unwrap_or_else(|_| "unlabeled".to_owned()),
            configuration: env::var(CONFIGURATION_ENV).unwrap_or_else(|_| "unlabeled".to_owned()),
            crate_name: arg_value(&args, "--crate-name"),
            cargo_package: env::var("CARGO_PKG_NAME").ok(),
            primary: env::var("CARGO_PRIMARY_PACKAGE").is_ok(),
            crate_types: crate_types(&args),
            target: arg_value(&args, "--target"),
            probe: is_probe(&args),
            argv_digest: argv_digest(
                &command,
                &args,
                &env::var(PACKAGE_ENV).unwrap_or_default(),
                &env::var(CONFIGURATION_ENV).unwrap_or_default(),
            ),
            started_ms,
            finished_ms,
            duration_ms: u64::try_from(finished_ms.saturating_sub(started_ms)).unwrap_or(u64::MAX),
            exit_code: status.code(),
            pid: std::process::id(),
        };
        if let Err(error) = write_event(&dir, &event) {
            eprintln!("ci-build: could not record a compiler event: {error:#}");
        }
    }

    // A compiler killed by a signal reports no code. 101 is rustc's own
    // internal-error status, which is the closest honest thing to report and
    // keeps Cargo failing rather than succeeding on a lost process.
    Ok(status.code().unwrap_or(101))
}

fn counter_dir_from_env() -> Option<PathBuf> {
    env::var_os(COUNTER_DIR_ENV)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn in_wrapper_mode() -> bool {
    env::var_os(WRAP_ENV).is_some_and(|value| !value.is_empty())
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

/// Compiler work for one `{package, configuration}` measurement label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Slice {
    package: String,
    configuration: String,
    /// rustc invocations that were not compiler-information probes. This is the number the
    /// specification's "compile once" claim is measured against.
    compiler_invocations: u64,
    /// The subset Cargo marked `CARGO_PRIMARY_PACKAGE`.
    primary_invocations: u64,
    probes: u64,
    distinct_crates: u64,
    /// Summed wall time of every non-probe invocation. Exceeds `window_ms`
    /// whenever Cargo compiled in parallel, which is the normal case.
    compiler_ms: u64,
    /// First start to last finish across every non-probe invocation — the
    /// elapsed build window this label occupied.
    window_ms: u64,
    /// Non-zero rustc exits. A green build normally has a few: `autocfg` and
    /// `version_check` build scripts detect a feature by compiling a probe they
    /// expect to fail. This is a diagnostic beside the gate's outcome, never a
    /// substitute for it.
    failed_invocations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Report {
    schema_version: u32,
    event_schema_version: u32,
    events_read: u64,
    compiler_invocations: u64,
    primary_invocations: u64,
    probes: u64,
    compiler_ms: u64,
    window_ms: u64,
    slices: Vec<Slice>,
}

fn read_events(dir: &Path) -> Result<Vec<Event>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut events = Vec::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();
    for path in entries {
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let event: Event = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        if event.schema_version != EVENT_SCHEMA_VERSION {
            bail!(
                "{} is event schema version {}, this tool reads {}",
                path.display(),
                event.schema_version,
                EVENT_SCHEMA_VERSION
            );
        }
        events.push(event);
    }
    Ok(events)
}

/// Fold raw events into the versioned report document.
///
/// Separated from rendering so the fixtures assert on counts rather than on
/// wrapped terminal text.
fn summarize(events: &[Event]) -> Report {
    #[derive(Default)]
    struct Acc {
        compiler_invocations: u64,
        primary_invocations: u64,
        probes: u64,
        compiler_ms: u64,
        failed_invocations: u64,
        first_start: Option<u128>,
        last_finish: Option<u128>,
        crates: BTreeSet<String>,
    }

    let mut by_label: BTreeMap<(String, String), Acc> = BTreeMap::new();
    let mut overall = Acc::default();

    for event in events {
        let key = (event.package.clone(), event.configuration.clone());
        let acc = by_label.entry(key).or_default();
        for target in [&mut *acc, &mut overall] {
            if event.probe {
                target.probes += 1;
                continue;
            }
            target.compiler_invocations += 1;
            if event.primary {
                target.primary_invocations += 1;
            }
            if event.exit_code != Some(0) {
                target.failed_invocations += 1;
            }
            target.compiler_ms = target.compiler_ms.saturating_add(event.duration_ms);
            target.first_start = Some(match target.first_start {
                Some(existing) => existing.min(event.started_ms),
                None => event.started_ms,
            });
            target.last_finish = Some(match target.last_finish {
                Some(existing) => existing.max(event.finished_ms),
                None => event.finished_ms,
            });
        }
        if !event.probe {
            if let Some(name) = &event.crate_name {
                acc.crates.insert(name.clone());
            }
        }
    }

    fn window(acc: &Acc) -> u64 {
        match (acc.first_start, acc.last_finish) {
            (Some(start), Some(finish)) => {
                u64::try_from(finish.saturating_sub(start)).unwrap_or(u64::MAX)
            }
            _ => 0,
        }
    }

    let slices = by_label
        .iter()
        .map(|((package, configuration), acc)| Slice {
            package: package.clone(),
            configuration: configuration.clone(),
            compiler_invocations: acc.compiler_invocations,
            primary_invocations: acc.primary_invocations,
            probes: acc.probes,
            distinct_crates: acc.crates.len() as u64,
            compiler_ms: acc.compiler_ms,
            window_ms: window(acc),
            failed_invocations: acc.failed_invocations,
        })
        .collect();

    Report {
        schema_version: REPORT_SCHEMA_VERSION,
        event_schema_version: EVENT_SCHEMA_VERSION,
        events_read: events.len() as u64,
        compiler_invocations: overall.compiler_invocations,
        primary_invocations: overall.primary_invocations,
        probes: overall.probes,
        compiler_ms: overall.compiler_ms,
        window_ms: window(&overall),
        slices,
    }
}

const COLUMNS: [&str; 5] = ["Package", "Configuration", "rustc", "own", "window"];

fn rows(report: &Report) -> Vec<Vec<String>> {
    report
        .slices
        .iter()
        .map(|slice| {
            vec![
                slice.package.clone(),
                slice.configuration.clone(),
                slice.compiler_invocations.to_string(),
                slice.primary_invocations.to_string(),
                format!("{:.1}s", slice.window_ms as f64 / 1000.0),
            ]
        })
        .collect()
}

fn render(report: &Report, term: &Terminal) -> String {
    let mut out = String::new();

    if report.events_read == 0 {
        out.push_str(
            &Prose::new(
                "**No compiler work recorded.** Either the measured command compiled \
                 nothing, or `RUSTC_WRAPPER` and `BISCUIT_CI_BUILD_WRAP` were not set \
                 on it — an empty counter is never proof of reuse.",
            )
            .render(term),
        );
        out.push('\n');
        return out;
    }

    out.push_str(
        &Prose::new(format!(
            "**Compiler work** — {} rustc invocation(s) ({} for the selected package's own \
             crates) across {} label(s), {:.1}s of compiler time in a {:.1}s window.",
            report.compiler_invocations,
            report.primary_invocations,
            report.slices.len(),
            report.compiler_ms as f64 / 1000.0,
            report.window_ms as f64 / 1000.0
        ))
        .render(term),
    );
    out.push('\n');

    let columns = COLUMNS
        .into_iter()
        .map(|header| {
            let column = TableColumn::new(header);
            // The per-label window is repeated in the totals prose above, so
            // it is the column that yields on a narrow terminal.
            if header == "window" {
                column.drop_when_space_is_limited(Some("window totals shown above"))
            } else {
                column
            }
        })
        .collect::<Vec<_>>();
    let data = rows(report)
        .into_iter()
        .map(|row| row.into_iter().map(Into::into).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    out.push_str(&Table::new().with_columns(columns).with_data(data).render(term));
    out.push('\n');

    let failures = report
        .slices
        .iter()
        .filter(|slice| slice.failed_invocations > 0)
        .map(|slice| {
            format!(
                "`{}` / `{}` — {} invocation(s) exited non-zero",
                slice.package, slice.configuration, slice.failed_invocations
            )
        })
        .collect::<Vec<_>>();
    if !failures.is_empty() {
        out.push_str(&UnorderedList::new(failures).render(term));
        out.push_str(
            &Prose::new(
                "_A green build normally reports a few: `autocfg` and `version_check` \
                 build scripts detect a feature by compiling a probe they expect to \
                 fail. Read this beside the gate's own outcome, not instead of it._",
            )
            .render(term),
        );
        out.push('\n');
    }

    if report.probes > 0 {
        out.push_str(
            &Prose::new(format!(
                "_{} version/target probe(s) excluded; Cargo queries compiler information and \
                 that is not compiler work._",
                report.probes
            ))
            .render(term),
        );
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Planned build keys
// ---------------------------------------------------------------------------

/// A batch of already-canonical strings to digest.
///
/// The caller canonicalizes: `scripts/ci/schema.py::canonical` is the one
/// serializer the repository's CI documents round-trip through, and asking two
/// languages to agree on JSON key order, float spelling, and non-ASCII escaping
/// is how two "identical" digests drift. This side hashes bytes.
#[derive(Debug, Deserialize)]
struct KeyRequest {
    schema_version: u32,
    material: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct KeyResponse {
    schema_version: u32,
    keys: Vec<String>,
}

/// The planned build key of one canonical input string.
///
/// Sixteen lowercase hex digits of XXH64 through `biscuit-hash`. The same
/// boundary already names the counter's event files, so a build key and an
/// event digest cannot come from two hashing implementations.
fn planned_key(material: &str) -> String {
    format!("{:016x}", xx_hash(material))
}

fn keys(request: &KeyRequest) -> Result<KeyResponse> {
    if request.schema_version != KEY_SCHEMA_VERSION {
        bail!(
            "key request is schema version {}, this tool reads {}",
            request.schema_version,
            KEY_SCHEMA_VERSION
        );
    }
    Ok(KeyResponse {
        schema_version: KEY_SCHEMA_VERSION,
        keys: request.material.iter().map(|item| planned_key(item)).collect(),
    })
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum Invocation {
    Report { dir: PathBuf, json: bool },
    Reset { dir: PathBuf },
    /// `None` reads stdin, which is how the planner passes a batch without a
    /// temporary file.
    Key { input: Option<PathBuf> },
    Produce(Box<ProduceOptions>),
    Verify(Box<VerifyOptions>),
    Usage,
}

/// The default Nextest driver, split the way `BISCUIT_NEXTEST_BIN` is.
///
/// `cargo nextest` needs Cargo on `PATH`. A producer always has one; a consumer
/// deliberately does not, and spells `cargo-nextest nextest` instead.
fn default_nextest() -> Vec<String> {
    env::var("BISCUIT_NEXTEST_BIN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_else(|| vec!["cargo".to_owned(), "nextest".to_owned()])
}

/// Every option the non-wrapper command line can carry, before a subcommand
/// decides which of them it requires.
#[derive(Default)]
struct Flags {
    dir: Option<PathBuf>,
    json: bool,
    input: Option<PathBuf>,
    plan: Option<PathBuf>,
    producer: Option<String>,
    key: Option<String>,
    environment: Option<String>,
    manifest: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    workspace: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    sidecars: Option<PathBuf>,
    source_tree: Option<String>,
    root: Option<PathBuf>,
    cargo: Option<String>,
    nextest: Option<Vec<String>>,
    verdict_out: Option<PathBuf>,
}

/// Parse the non-wrapper command line.
///
/// `dir_from_env` is the resolved `BISCUIT_CI_BUILD_COUNTER_DIR`, passed in
/// rather than read here so the fixtures do not have to mutate process
/// environment shared with every other test in the binary.
fn parse(args: &[String], dir_from_env: Option<PathBuf>) -> Result<Invocation> {
    let mut iter = args.iter();
    let Some(subcommand) = iter.next() else {
        return Ok(Invocation::Usage);
    };
    let mut flags = Flags {
        dir: dir_from_env,
        ..Flags::default()
    };
    while let Some(arg) = iter.next() {
        // `--flag value` and `--flag=value` are interchangeable everywhere, so
        // a caller never has to remember which spelling a flag takes.
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_owned())),
            _ => (arg.as_str(), None),
        };
        let mut value = || -> Result<String> {
            match &inline {
                Some(value) => Ok(value.clone()),
                None => iter
                    .next()
                    .cloned()
                    .with_context(|| format!("{name} needs a value")),
            }
        };
        match name {
            "--counter-dir" => flags.dir = Some(PathBuf::from(value()?)),
            "--input" => flags.input = Some(PathBuf::from(value()?)),
            "--plan" => flags.plan = Some(PathBuf::from(value()?)),
            "--producer" => flags.producer = Some(value()?),
            "--key" => flags.key = Some(value()?),
            "--environment" => flags.environment = Some(value()?),
            "--manifest" => flags.manifest = Some(PathBuf::from(value()?)),
            "--out-dir" => flags.out_dir = Some(PathBuf::from(value()?)),
            "--workspace" => flags.workspace = Some(PathBuf::from(value()?)),
            "--target-dir" => flags.target_dir = Some(PathBuf::from(value()?)),
            "--sidecars" => flags.sidecars = Some(PathBuf::from(value()?)),
            "--source-tree" => flags.source_tree = Some(value()?),
            "--root" => flags.root = Some(PathBuf::from(value()?)),
            "--verdict-out" => flags.verdict_out = Some(PathBuf::from(value()?)),
            "--cargo" => flags.cargo = Some(value()?),
            "--nextest" => {
                flags.nextest =
                    Some(value()?.split_whitespace().map(str::to_owned).collect::<Vec<_>>());
            }
            "--json" => flags.json = true,
            "-h" | "--help" => return Ok(Invocation::Usage),
            other => bail!("unknown argument {other}"),
        }
    }
    match subcommand.as_str() {
        "report" => Ok(Invocation::Report {
            dir: flags
                .dir
                .with_context(|| format!("report needs --counter-dir or {COUNTER_DIR_ENV}"))?,
            json: flags.json,
        }),
        "reset" => Ok(Invocation::Reset {
            dir: flags
                .dir
                .with_context(|| format!("reset needs --counter-dir or {COUNTER_DIR_ENV}"))?,
        }),
        "key" => Ok(Invocation::Key {
            input: flags.input.filter(|path| path.as_os_str() != "-"),
        }),
        "produce" => Ok(Invocation::Produce(Box::new(ProduceOptions {
            plan: flags.plan.context("produce needs --plan")?,
            producer: flags.producer.context("produce needs --producer")?,
            key: flags.key,
            out_dir: flags.out_dir.context("produce needs --out-dir")?,
            workspace: flags
                .workspace
                .unwrap_or_else(|| PathBuf::from(".")),
            target_dir: flags.target_dir,
            sidecars: flags.sidecars,
            source_tree: flags.source_tree,
            counter_dir: flags.dir,
            cargo: flags.cargo.unwrap_or_else(|| "cargo".to_owned()),
            nextest: flags.nextest.unwrap_or_else(default_nextest),
            json: flags.json,
        }))),
        "verify" => Ok(Invocation::Verify(Box::new(VerifyOptions {
            manifest: flags.manifest.context("verify needs --manifest")?,
            environment: flags.environment.context("verify needs --environment")?,
            plan: flags.plan,
            root: flags.root,
            workspace: flags.workspace,
            nextest: flags.nextest.unwrap_or_else(default_nextest),
            json: flags.json,
            verdict_out: flags.verdict_out,
        }))),
        "-h" | "--help" | "help" => Ok(Invocation::Usage),
        other => bail!("unknown subcommand {other}"),
    }
}

/// Empty the counter directory without removing it.
///
/// The directory itself survives so a measured step can export
/// `BISCUIT_CI_BUILD_COUNTER_DIR` before this runs, and so a reset on a
/// never-used directory is not an error.
fn reset(dir: &Path) -> Result<u64> {
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut removed = 0;
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Run one invocation.
///
/// ## Returns
///
/// The process exit code. `verify` answers `3` for a refused build: a rejection
/// is a *verdict* about infrastructure, not a tool failure, and a workflow has
/// to be able to tell the two apart — `2` still means `ci-build` itself could
/// not do its job.
fn run(invocation: Invocation, term: &Terminal) -> Result<u8> {
    match invocation {
        Invocation::Report { dir, json } => {
            let report = summarize(&read_events(&dir)?);
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render(&report, term));
            }
        }
        Invocation::Reset { dir } => {
            let removed = reset(&dir)?;
            print!(
                "{}",
                Prose::new(format!(
                    "Cleared **{removed}** recorded compiler event(s) from `{}`.",
                    dir.display()
                ))
                .render(term)
            );
        }
        Invocation::Key { input } => {
            let text = match &input {
                Some(path) => fs::read_to_string(path)
                    .with_context(|| format!("reading {}", path.display()))?,
                None => {
                    use std::io::Read as _;
                    let mut buffer = String::new();
                    std::io::stdin()
                        .read_to_string(&mut buffer)
                        .context("reading the key request from stdin")?;
                    buffer
                }
            };
            let request: KeyRequest =
                serde_json::from_str(&text).context("parsing the key request")?;
            println!("{}", serde_json::to_string(&keys(&request)?)?);
        }
        Invocation::Produce(options) => {
            // Exit 1 when any key failed, after every key has been attempted
            // and has left a status document: the owner leg must be red, and
            // the reason must already be on disk for the rollup to read.
            let outcomes = archive::run_produce(&options, term)?;
            if outcomes
                .iter()
                .any(|(status, _)| status.result != "success")
            {
                return Ok(1);
            }
        }
        Invocation::Verify(options) => {
            if !archive::run_verify(&options, term)? {
                return Ok(3);
            }
        }
        Invocation::Usage => print!("{USAGE}"),
    }
    Ok(0)
}

fn main() -> ExitCode {
    // `args_os`, not `args`: a Windows rustc command line can carry a path that
    // is not valid Unicode, and `env::args` panics on one. The wrapper must
    // never be the reason a build dies.
    let argv: Vec<String> = env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    if in_wrapper_mode() {
        return match run_wrapper(argv) {
            Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
            Err(error) => {
                eprintln!("ci-build: {error:#}");
                ExitCode::from(101)
            }
        };
    }

    let term = Terminal::new();
    match parse(&argv, counter_dir_from_env()).and_then(|invocation| run(invocation, &term)) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("ci-build: {error:#}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[path = "ci-build-tests.rs"]
mod tests;
