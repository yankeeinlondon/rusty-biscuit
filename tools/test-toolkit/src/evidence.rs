//! Machine-readable proof that backend-gated tests actually executed.
//!
//! Availability is not execution. A CI leg can install `tmux`, run a tier that
//! happens to contain no tmux-backed tests, and exit 0 — a green cell that
//! verified nothing, indistinguishable from a real pass. The gate therefore
//! records what it decided, per test, and a post-run check asserts that every
//! required backend produced at least one [`ExecutionDecision::Run`].
//!
//! ## Notes
//!
//! nextest runs each test in its own process, so an in-process counter cannot
//! aggregate. Records are appended to a shared JSON Lines file instead, one
//! `O_APPEND` `write` per line so concurrent test processes cannot interleave
//! partial lines. Nothing ever reads-modifies-writes the file.
//!
//! Recording is off unless [`BISCUIT_TEST_REQUIRED_BACKENDS`] is set, so local
//! development does no file I/O.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::backend::{BISCUIT_TEST_REQUIRED_BACKENDS, Backend};

/// Environment variable overriding the JUnit/report staging directory.
///
/// When unset, the staging root is `target/nextest/ci-reports` under the
/// workspace root.
pub const BISCUIT_JUNIT_STAGE_DIR: &str = "BISCUIT_JUNIT_STAGE_DIR";

/// File name, inside the staging directory, of the execution-evidence log.
pub const BACKEND_EXECUTIONS_FILE: &str = "backend-executions.jsonl";

/// Staging-root path relative to the workspace root, used when
/// [`BISCUIT_JUNIT_STAGE_DIR`] is unset.
const DEFAULT_STAGE_SUBPATH: &str = "target/nextest/ci-reports";

/// What [`require_level!`](crate::require_level) decided for one gated test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionDecision {
    /// The gate passed and the test body ran.
    Run,
    /// The harness was unavailable and not required; the test returned early.
    Skip,
    /// The harness was unavailable and required; the test panicked.
    Panic,
}

impl ExecutionDecision {
    /// The stable string written to the evidence file.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ExecutionDecision::Run => "run",
            ExecutionDecision::Skip => "skip",
            ExecutionDecision::Panic => "panic",
        }
    }
}

impl fmt::Display for ExecutionDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One line of the evidence file.
///
/// `backend` and `decision` are kept as raw strings rather than parsed enums so
/// a file written by a newer toolkit does not make an older checker fail with a
/// parse error instead of the verdict it was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRecord {
    /// Stable backend identifier, as written by [`Backend::as_str`].
    pub backend: String,
    /// Fully qualified name of the gated test.
    pub test: String,
    /// One of `run`, `skip`, `panic`.
    pub decision: String,
}

/// Resolve the staging directory.
///
/// [`BISCUIT_JUNIT_STAGE_DIR`] wins when set to a non-empty value; a relative
/// value resolves against the **workspace root**, not the current directory,
/// matching `just/devops.just`. Otherwise the path is
/// [`DEFAULT_STAGE_SUBPATH`] under the workspace root.
#[must_use]
pub fn stage_dir() -> PathBuf {
    match std::env::var(BISCUIT_JUNIT_STAGE_DIR) {
        Ok(raw) if !raw.trim().is_empty() => {
            let candidate = PathBuf::from(raw);
            if candidate.is_absolute() {
                candidate
            } else {
                workspace_root().join(candidate)
            }
        }
        _ => workspace_root().join(DEFAULT_STAGE_SUBPATH),
    }
}

/// Full path of the execution-evidence file.
#[must_use]
pub fn backend_executions_path() -> PathBuf {
    stage_dir().join(BACKEND_EXECUTIONS_FILE)
}

/// The workspace root containing this crate.
///
/// Derived from this crate's compile-time manifest directory rather than the
/// current directory: nextest runs each test with its *own* package as the
/// working directory, so every test process would otherwise resolve a different
/// staging root and the evidence would scatter.
#[must_use]
pub fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .ancestors()
            .find(|candidate| is_workspace_root(candidate))
            .unwrap_or(manifest_dir)
            .to_path_buf()
    })
    .as_path()
}

fn is_workspace_root(candidate: &Path) -> bool {
    fs::read_to_string(candidate.join("Cargo.toml"))
        .is_ok_and(|manifest| manifest.lines().any(|line| line.trim() == "[workspace]"))
}

/// Whether backend decisions are being recorded in this process.
///
/// Recording follows [`BISCUIT_TEST_REQUIRED_BACKENDS`] being set to a
/// non-blank value. Validity of that value is not checked here — an invalid
/// value is reported by the gate itself, which panics.
#[must_use]
pub fn recording_enabled() -> bool {
    std::env::var(BISCUIT_TEST_REQUIRED_BACKENDS)
        .is_ok_and(|raw| !raw.trim().is_empty())
}

/// Record one backend decision, if recording is enabled.
///
/// I/O failures are reported on stderr rather than propagated: a test must not
/// fail because the evidence directory is unwritable. The consequence surfaces
/// at the end of the tier instead, when `backend-proof verify` finds no `run`
/// record for the backend.
pub fn record_backend_execution(backend: Backend, test: &str, decision: ExecutionDecision) {
    if !recording_enabled() {
        return;
    }
    let path = backend_executions_path();
    if let Err(err) = append_backend_execution(&path, backend, test, decision) {
        eprintln!(
            "warning: could not record {} execution evidence to {}: {err}",
            backend.as_str(),
            path.display()
        );
    }
}

/// Append one record to `path`, creating the file and its parents as needed.
///
/// The line is emitted in a single `write_all` on an `O_APPEND` handle so
/// concurrent writers from separate test processes append whole lines. There is
/// no read-modify-write and no locking.
///
/// ## Errors
///
/// Any I/O error from creating the parent directory, opening the file, or
/// writing the line.
pub fn append_backend_execution(
    path: &Path,
    backend: Backend,
    test: &str,
    decision: ExecutionDecision,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = format!(
        "{{\"backend\":\"{}\",\"test\":\"{}\",\"decision\":\"{}\"}}\n",
        escape_json(backend.as_str()),
        escape_json(test),
        escape_json(decision.as_str()),
    );
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())
}

/// Read every record from an evidence file.
///
/// A missing file yields an empty vector: "the tier ran and produced nothing"
/// and "the tier never ran" are the same verdict here, and both are failures
/// once a backend is required.
///
/// ## Errors
///
/// I/O errors other than "not found", and any line that is not a well-formed
/// record object.
pub fn read_backend_executions(path: &Path) -> io::Result<Vec<ExecutionRecord>> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    let mut records = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let record = parse_record(line).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: line {} is not a backend execution record: {line}",
                    path.display(),
                    index + 1
                ),
            )
        })?;
        records.push(record);
    }
    Ok(records)
}

/// Backends in `required` for which `records` contains no `run` entry.
///
/// The returned set is the failure itself: empty means the tier proved every
/// requirement.
#[must_use]
pub fn unproven_backends(
    required: &BTreeSet<Backend>,
    records: &[ExecutionRecord],
) -> BTreeSet<Backend> {
    required
        .iter()
        .copied()
        .filter(|backend| {
            !records.iter().any(|record| {
                record.backend == backend.as_str()
                    && record.decision == ExecutionDecision::Run.as_str()
            })
        })
        .collect()
}

/// Count records for `backend` with each decision, as `(run, skip, panic)`.
#[must_use]
pub fn decision_counts(backend: Backend, records: &[ExecutionRecord]) -> (usize, usize, usize) {
    let mut counts = (0, 0, 0);
    for record in records.iter().filter(|r| r.backend == backend.as_str()) {
        match record.decision.as_str() {
            "run" => counts.0 += 1,
            "skip" => counts.1 += 1,
            "panic" => counts.2 += 1,
            _ => {}
        }
    }
    counts
}

/// Strip the `::{{closure}}` segments that appear when a test body is wrapped
/// by an attribute macro such as `#[serial_test::serial]`.
///
/// Without this the same test yields a different identity depending on which
/// attributes it happens to carry.
#[must_use]
pub fn normalize_test_name(raw: &str) -> &str {
    let mut name = raw;
    while let Some(trimmed) = name.strip_suffix("::{{closure}}") {
        name = trimmed;
    }
    name
}

fn escape_json(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Minimal reader for the flat `{"key":"value",...}` objects this module
/// writes. Returns `None` for anything else rather than guessing.
fn parse_record(line: &str) -> Option<ExecutionRecord> {
    let body = line.trim().strip_prefix('{')?.strip_suffix('}')?;
    let mut backend = None;
    let mut test = None;
    let mut decision = None;

    let mut rest = body;
    while !rest.trim().is_empty() {
        let (key, after_key) = take_json_string(rest.trim_start())?;
        let after_colon = after_key.trim_start().strip_prefix(':')?;
        let (value, after_value) = take_json_string(after_colon.trim_start())?;
        match key.as_str() {
            "backend" => backend = Some(value),
            "test" => test = Some(value),
            "decision" => decision = Some(value),
            _ => {}
        }
        let tail = after_value.trim_start();
        rest = match tail.strip_prefix(',') {
            Some(next) => next,
            None if tail.is_empty() => tail,
            None => return None,
        };
    }

    Some(ExecutionRecord {
        backend: backend?,
        test: test?,
        decision: decision?,
    })
}

/// Consume one JSON string literal from the front of `input`, returning the
/// unescaped contents and the remainder.
fn take_json_string(input: &str) -> Option<(String, &str)> {
    let mut chars = input.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }
    let mut out = String::new();
    while let Some((index, ch)) = chars.next() {
        match ch {
            '"' => return Some((out, &input[index + 1..])),
            '\\' => match chars.next()?.1 {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000c}'),
                'u' => {
                    let mut code = String::new();
                    for _ in 0..4 {
                        code.push(chars.next()?.1);
                    }
                    let value = u32::from_str_radix(&code, 16).ok()?;
                    out.push(char::from_u32(value)?);
                }
                _ => return None,
            },
            c => out.push(c),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        Backend, ExecutionDecision, ExecutionRecord, escape_json, parse_record, take_json_string,
        unproven_backends,
    };
    use std::collections::BTreeSet;

    #[test]
    fn round_trips_a_record_with_awkward_characters() {
        let line = format!(
            "{{\"backend\":\"{}\",\"test\":\"{}\",\"decision\":\"{}\"}}",
            escape_json(Backend::Tmux.as_str()),
            escape_json("mod::weird\"name\\here"),
            escape_json(ExecutionDecision::Run.as_str()),
        );
        let record = parse_record(&line).expect("parses");
        assert_eq!(record.backend, "tmux");
        assert_eq!(record.test, "mod::weird\"name\\here");
        assert_eq!(record.decision, "run");
    }

    #[test]
    fn parse_record_rejects_non_records() {
        assert!(parse_record("not json").is_none());
        assert!(parse_record("{\"backend\":\"tmux\"}").is_none());
        assert!(parse_record("{\"backend\":\"tmux\",\"test\":\"t\"}").is_none());
    }

    #[test]
    fn take_json_string_decodes_unicode_escapes() {
        let (value, rest) = take_json_string("\"a\\u0041b\",tail").expect("parses");
        assert_eq!(value, "aAb");
        assert_eq!(rest, ",tail");
    }

    #[test]
    fn unproven_ignores_skip_and_panic_records() {
        let required: BTreeSet<Backend> = [Backend::Tmux, Backend::Kitty].into_iter().collect();
        let records = vec![
            ExecutionRecord {
                backend: "tmux".into(),
                test: "a".into(),
                decision: "run".into(),
            },
            ExecutionRecord {
                backend: "kitty".into(),
                test: "b".into(),
                decision: "skip".into(),
            },
        ];

        assert_eq!(
            unproven_backends(&required, &records)
                .into_iter()
                .collect::<Vec<_>>(),
            vec![Backend::Kitty]
        );
    }
}
