//! Every ordinary exit of the `claudine` binary goes through
//! `cli/src/shutdown.rs`, which drains pending outbound messages first.
//!
//! A direct `std::process::exit`, `libc::_exit`, or `ExitProcess` anywhere else
//! in `claudine/cli/src` would kill a message still being sent, with no
//! warning. This guard scans the crate's source (comments and string literals
//! blanked by [`source_scan::sanitize`]) and reconciles every direct exit
//! against [`EXIT_ALLOWLIST`], which names each permitted file, its exact site
//! count, and why. It fails in both directions: a site in an unlisted file or a
//! count that grew, and an entry whose file no longer has that many sites.
//!
//! ## Out of scope
//!
//! These exits happen before any delivery can start, and the detector does not
//! match them, so they carry no entry:
//!
//! - clap's `err.exit()` and `Cli::parse_from` (help, version, parse errors);
//! - `completion::maybe_complete()`, which exits from inside the completion
//!   engine when invoked as a shell-completion subprocess.
//!
//! Text that only *looks* like an exit is not a site either: the child program
//! that `wrap/exec/termination/windows.rs` writes out for a test is a raw string
//! literal, which the sanitizer blanks.

use crate::common::source_scan;

use source_scan::{is_ident, line_at, sanitize};

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Files allowed to exit directly, with their exact site count and reason.
///
/// Paths are relative to `claudine/cli/src`, with `/` separators.
const EXIT_ALLOWLIST: &[(&str, usize, &str)] = &[
    (
        "shutdown.rs",
        2,
        "the ordinary exit after the delivery drain, and the exit for errors \
         raised before the runtime exists",
    ),
    (
        "main.rs",
        1,
        "the audio worker mode, which runs instead of the CLI and never starts \
         a delivery",
    ),
    (
        "commands/compose/interrupt.rs",
        5,
        "forced exits on a repeat Ctrl+C: the user asked to stop now",
    ),
];

/// One direct exit call in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Site {
    line: usize,
    form: &'static str,
}

/// The direct exit calls in `source`, ignoring comments and string literals.
fn exit_sites(source: &str) -> Vec<Site> {
    let sanitized = sanitize(source);
    let mut sites = Vec::new();
    for (form, needle) in [
        ("process::exit", b"process::exit".as_slice()),
        ("_exit", b"_exit".as_slice()),
        ("ExitProcess", b"ExitProcess".as_slice()),
    ] {
        let mut from = 0;
        while let Some(offset) = find(&sanitized[from..], needle).map(|at| from + at) {
            from = offset + needle.len();
            let before = offset.checked_sub(1).map(|at| sanitized[at]);
            let after = sanitized.get(from).copied();
            // `force_exit` and `exit_code` are not `_exit`; `ExitProcessFoo`
            // is not `ExitProcess`.
            if before.is_some_and(is_ident) || after.is_some_and(is_ident) {
                continue;
            }
            // `std::process::exit` also contains `::exit`, not `_exit`, so the
            // forms cannot double-count one call.
            sites.push(Site {
                line: line_at(source, offset),
                form,
            });
        }
    }
    sites.sort_by_key(|site| site.line);
    sites
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

/// Direct exit sites per file, keyed by the `/`-separated path under `src`.
fn exit_census(src: &Path) -> BTreeMap<String, Vec<Site>> {
    let mut files = Vec::new();
    rust_files(src, &mut files);
    let mut census = BTreeMap::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let sites = exit_sites(&source);
        if sites.is_empty() {
            continue;
        }
        let relative = path
            .strip_prefix(src)
            .expect("file under src")
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        census.insert(relative, sites);
    }
    census
}

/// Every mismatch between `census` and `allowlist`, one line each.
fn violations(
    census: &BTreeMap<String, Vec<Site>>,
    allowlist: &[(&str, usize, &str)],
) -> Vec<String> {
    let mut problems = Vec::new();
    for (file, sites) in census {
        let listed = allowlist.iter().find(|(path, _, _)| path == file);
        let lines = sites
            .iter()
            .map(|site| format!("{}:{} ({})", file, site.line, site.form))
            .collect::<Vec<_>>()
            .join(", ");
        match listed {
            None => problems.push(format!(
                "direct exit outside the shutdown path: {lines}; return the exit code \
                 to `async_main` so `shutdown::finish` drains deliveries first"
            )),
            Some((_, expected, _)) if *expected != sites.len() => problems.push(format!(
                "{file} has {} direct exit sites but its allowlist entry says {expected}: {lines}",
                sites.len()
            )),
            Some(_) => {}
        }
    }
    for (file, expected, _) in allowlist {
        if !census.contains_key(*file) {
            problems.push(format!(
                "stale allowlist entry: {file} is listed with {expected} direct exit \
                 sites but has none"
            ));
        }
    }
    problems
}

#[test]
fn direct_exits_occur_only_at_allowlisted_sites() {
    let src = biscuit_test_harness::manifest_dir!().join("src");
    let census = exit_census(&src);
    let problems = violations(&census, EXIT_ALLOWLIST);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_allowlist_entry_states_a_reason() {
    for (file, sites, reason) in EXIT_ALLOWLIST {
        assert!(*sites > 0, "{file}: an entry must allow at least one site");
        assert!(!reason.trim().is_empty(), "{file}: an entry needs a reason");
    }
}

#[test]
fn the_detector_finds_every_exit_form() {
    let source = "\
fn a() { std::process::exit(1); }
fn b() { unsafe { libc::_exit(130) } }
fn c() { unsafe { windows::Win32::System::Threading::ExitProcess(130) } }
use std::process::exit;
";
    let forms: Vec<_> = exit_sites(source)
        .into_iter()
        .map(|site| (site.line, site.form))
        .collect();
    assert_eq!(
        forms,
        vec![
            (1, "process::exit"),
            (2, "_exit"),
            (3, "ExitProcess"),
            (4, "process::exit"),
        ]
    );
}

#[test]
fn the_detector_ignores_comments_strings_and_similar_names() {
    let source = r##"
// std::process::exit(1) in a comment
/// libc::_exit(130) in a doc comment
fn force_exit() -> ! { loop {} }
fn a(err: clap::Error) { err.exit() }
const TEXT: &str = "std::process::exit(2)";
const RAW: &str = r#"ExitProcess(3)"#;
fn exit_code() -> i32 { 0 }
fn b() { let _ = ExitProcessLater; }
"##;
    assert_eq!(exit_sites(source), Vec::new());
}

#[test]
fn a_site_in_an_unlisted_file_fails() {
    let mut census = BTreeMap::new();
    census.insert(
        "commands/wrap/mod.rs".to_string(),
        vec![Site {
            line: 245,
            form: "process::exit",
        }],
    );
    let problems = violations(&census, &[]);
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0].contains("commands/wrap/mod.rs:245 (process::exit)"),
        "{problems:?}"
    );
}

#[test]
fn an_extra_site_in_a_listed_file_fails() {
    let mut census = BTreeMap::new();
    census.insert(
        "main.rs".to_string(),
        vec![
            Site {
                line: 10,
                form: "process::exit",
            },
            Site {
                line: 20,
                form: "process::exit",
            },
        ],
    );
    let problems = violations(&census, &[("main.rs", 1, "audio worker")]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("says 1"), "{problems:?}");
}

#[test]
fn an_entry_with_no_live_site_fails() {
    let problems = violations(&BTreeMap::new(), &[("sequence.rs", 1, "old exit")]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("stale allowlist entry: sequence.rs"), "{problems:?}");
}
