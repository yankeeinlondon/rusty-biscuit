//! `sniff filesystem query`: resolve the target, run the library query once,
//! and project the captured report to text or JSON.

use std::ffi::OsStr;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use biscuit_file::{
    DetailedOutcome, FileReference, FileReferenceKind, FileResolutionContext, ProbeDisposition,
    ResolutionFailure, find_git_root,
};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::query::{Outcome, PathUsageOptions, PathUsageReport, query_path_usage};

use crate::args::FilesystemQueryArgs;
use crate::output;
use crate::perf::CliPerf;

/// Hidden test seam: a file holding a serialized report to render instead of
/// querying the host, so tests can assert both projections of one retained
/// observation and outcomes a live host cannot produce on demand.
pub(crate) const REPLAY_ENV: &str = "SNIFF_FILESYSTEM_QUERY_REPLAY";

/// A failure that leaves no report: target resolution or a typed query error.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct QueryFailure {
    pub kind: &'static str,
    pub message: String,
}

/// Runs the query and writes its output.
///
/// ## Returns
///
/// The process exit code: 0 for a usable report, 1 for an unavailable or
/// unsupported report or a failure that left no report. Match presence never
/// changes it.
pub(crate) fn run(
    args: &FilesystemQueryArgs,
    json: bool,
    plain: bool,
    perf: &CliPerf,
) -> Result<i32, Box<dyn std::error::Error>> {
    let (report, reference) = match std::env::var_os(REPLAY_ENV) {
        Some(file) => (Ok(replay(Path::new(&file))?), None),
        None => {
            let reference = args.path.to_str().map(str::to_owned);
            let report = perf.collect(|| live(args));
            // Only a file reference resolves to a different spelling; an
            // ordinary path reaches the library as written.
            let reference = reference.filter(|_| {
                report
                    .as_ref()
                    .is_ok_and(|r| r.target.requested.as_os_str() != args.path)
            });
            (report, reference)
        }
    };
    let performance = perf.build_report();

    let code = match &report {
        Ok(report) if report.outcome == Outcome::Usable => 0,
        _ => 1,
    };
    let stdout = match report {
        Ok(report) if json => {
            let mut value = serde_json::to_value(&report)?;
            if let Some(performance) = &performance {
                value = output::attach_performance(value, performance);
            }
            format!("{}\n", serde_json::to_string_pretty(&value)?)
        }
        Ok(report) => {
            let term = Terminal::default();
            let width = if std::io::stdout().is_terminal() {
                term.width()
            } else {
                output::UNWRAPPED_WIDTH
            };
            let view = output::PathUsageView {
                term: &term,
                width,
                reference: reference.as_deref(),
            };
            let text = output::render_path_usage(&report, &view);
            if plain {
                biscuit_terminal::prelude::strip_escape_codes(&text)
            } else {
                text
            }
        }
        Err(failure) => {
            eprintln!(
                "Error: {}",
                output::neutralize_control_text(&failure.message)
            );
            if !json {
                perf.emit_stderr(performance.as_ref());
                return Ok(code);
            }
            let value = serde_json::json!({
                "error": { "kind": failure.kind, "message": failure.message }
            });
            format!("{}\n", serde_json::to_string_pretty(&value)?)
        }
    };
    let mut handle = std::io::stdout().lock();
    handle.write_all(stdout.as_bytes())?;
    handle.flush()?;
    perf.emit_stderr(performance.as_ref());
    Ok(code)
}

fn live(args: &FilesystemQueryArgs) -> Result<PathUsageReport, QueryFailure> {
    let path = resolve_target(&args.path)?;
    let mut options = PathUsageOptions::default();
    if args.target_only {
        options = options.target_only();
    }
    if let Some(timeout) = args.timeout {
        options = options.with_deadline(timeout);
    }
    query_path_usage(&path, &options).map_err(|error| QueryFailure {
        kind: error.kind(),
        message: error.to_string(),
    })
}

fn replay(file: &Path) -> Result<PathUsageReport, String> {
    let bytes = std::fs::read(file)
        .map_err(|error| format!("cannot read {REPLAY_ENV} file {}: {error}", file.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("{REPLAY_ENV} file {} is not a report: {error}", file.display()))
}

/// Resolves the positional argument to the native path handed to the library.
///
/// An ordinary path (relative, absolute, or not valid UTF-8) is passed
/// through unchanged, so the library resolves a relative one against the
/// invocation directory and no lossy string conversion happens. A file
/// reference (`@`, `&`, `^`, `~`, `vault:`) resolves locally through
/// `biscuit-file` to an existing file or directory; a URL is rejected before
/// anything is probed. A reference that resolves to nothing is an error,
/// never a fallback to the current directory.
pub(crate) fn resolve_target(raw: &OsStr) -> Result<PathBuf, QueryFailure> {
    let Some(text) = raw.to_str() else {
        return Ok(PathBuf::from(raw));
    };
    let reference = FileReference::new(text).map_err(|error| QueryFailure {
        kind: "invalid_reference",
        message: error.to_string(),
    })?;
    match reference.class().kind {
        FileReferenceKind::ExplicitRelative
        | FileReferenceKind::ImplicitRelative
        | FileReferenceKind::Absolute => return Ok(PathBuf::from(raw)),
        FileReferenceKind::Url => {
            return Err(QueryFailure {
                kind: "nonlocal_reference",
                message: format!("`{text}` is a remote URL; query a local file or directory"),
            });
        }
        FileReferenceKind::Magic
        | FileReferenceKind::RepositoryRoot
        | FileReferenceKind::RepositoryScoped
        | FileReferenceKind::Home
        | FileReferenceKind::Vault => {}
    }

    let cwd = std::env::current_dir().map_err(|error| QueryFailure {
        kind: "current_directory",
        message: format!("cannot read the current directory: {error}"),
    })?;
    let mut context = FileResolutionContext::new(&cwd);
    if matches!(
        reference.class().kind,
        FileReferenceKind::Magic
            | FileReferenceKind::RepositoryRoot
            | FileReferenceKind::RepositoryScoped
    ) {
        // Only the enclosing Git root: package scopes would need repository
        // inventory, which this command never runs.
        let root = find_git_root(&cwd).map_err(|error| QueryFailure {
            kind: "reference_repository",
            message: format!("cannot resolve `{text}`: {error}"),
        })?;
        if let Some(root) = root {
            context = context.with_repository_root(root);
        }
    }
    let resolution = reference.resolve_detailed(&context);
    // Resolution matches regular files only; a directory is the first
    // candidate, in the reference's own order, that exists as a non-file.
    // The library rejects sockets, devices, and FIFOs.
    for probed in resolution.candidates() {
        if matches!(
            probed.disposition(),
            ProbeDisposition::Matched | ProbeDisposition::NonFile
        ) {
            return Ok(cwd.join(probed.candidate().path()));
        }
    }
    let failure = match resolution.outcome() {
        DetailedOutcome::Failed(failure) => *failure,
        DetailedOutcome::Matched(path) => return Ok(path.clone()),
    };
    let kind = match failure {
        ResolutionFailure::InvalidReference => "invalid_reference",
        ResolutionFailure::MissingContext => "reference_context_missing",
        ResolutionFailure::NoMatch => "reference_not_found",
        ResolutionFailure::Io => "reference_io",
        ResolutionFailure::UnsupportedRemote => "nonlocal_reference",
    };
    let mut message = match resolution.error() {
        Some(error) => format!("cannot resolve `{text}`: {error}"),
        None => format!("no file or directory matches `{text}`"),
    };
    if let Some(hint) = resolution.glob_hint() {
        message.push_str(". ");
        message.push_str(hint);
    }
    Err(QueryFailure { kind, message })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_paths_pass_through_unchanged() {
        for raw in ["./checkout", "checkout", "../x/y", "/abs/path", "missing"] {
            assert_eq!(
                resolve_target(OsStr::new(raw)).unwrap(),
                PathBuf::from(raw),
                "{raw}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_path_passes_through_without_conversion() {
        use std::os::unix::ffi::OsStrExt;
        let raw = OsStr::from_bytes(b"dir-\xff");
        assert_eq!(resolve_target(raw).unwrap().as_os_str(), raw);
    }

    #[test]
    fn a_url_is_rejected_before_probing() {
        let failure = resolve_target(OsStr::new("https://example.com/x")).unwrap_err();
        assert_eq!(failure.kind, "nonlocal_reference");
    }

    #[test]
    fn a_reference_without_a_match_is_an_error_not_the_current_directory() {
        let failure =
            resolve_target(OsStr::new("@no-such-file-for-sniff-query-tests.txt")).unwrap_err();
        assert_eq!(failure.kind, "reference_not_found");
        assert!(failure.message.contains("@no-such-file"), "{}", failure.message);
    }
}
