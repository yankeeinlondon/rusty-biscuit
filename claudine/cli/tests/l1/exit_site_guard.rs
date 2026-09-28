//! Every ordinary exit of the `claudine` binary goes through
//! `cli/src/shutdown.rs`, which drains pending outbound messages first.
//!
//! A direct `std::process::exit`, `libc::_exit`, or `ExitProcess` anywhere else
//! in `claudine/cli/src` would kill a message still being sent, with no
//! warning. So would a call to one of the crate's own diverging exit helpers,
//! `shutdown::exit_before_runtime` and `compose::interrupt::force_exit`, made
//! from anywhere their contracts do not cover. This guard scans the crate's
//! source (comments and string literals blanked by [`source_scan::sanitize`])
//! for both and reconciles every site against [`EXIT_ALLOWLIST`].
//!
//! ## Site identity
//!
//! An entry pins one site by its file, its enclosing function, its position in
//! that function, and its call form, each read by
//! [`site_identity::site_contexts`]; the entry states why the site is safe. The
//! position is the chain of branches around the call (`if …`, `arm …`, …), or
//! `tail`/`body` when there is none, so an exit that moves out of the branch
//! that justifies it, into another function, or ahead of the tail call that
//! follows the drain no longer matches its entry. Line numbers are not part of
//! the identity, so unrelated edits never fail the guard.
//!
//! It fails in both directions: a site no entry matches, and an entry that
//! matches no live site. Entries and sites pair one to one, so a second site
//! with the same identity also fails.
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

use crate::common::{site_identity, source_scan};

use site_identity::{BODY, TAIL, site_contexts};
use source_scan::{is_ident, line_at, sanitize};

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// One approved direct exit.
struct AllowedExit {
    /// Relative to `claudine/cli/src`, with `/` separators.
    file: &'static str,
    /// [`site_identity::SiteContext::function`].
    function: &'static str,
    /// [`site_identity::SiteContext::position`].
    position: &'static str,
    form: &'static str,
    reason: &'static str,
}

/// Every approved direct exit, one entry per site.
const EXIT_ALLOWLIST: &[AllowedExit] = &[
    AllowedExit {
        file: "shutdown.rs",
        function: "finish",
        position: TAIL,
        form: "process::exit",
        reason: "the ordinary exit, after the delivery drain and the flush",
    },
    AllowedExit {
        file: "shutdown.rs",
        function: "exit_before_runtime",
        position: TAIL,
        form: "process::exit",
        reason: "errors raised before the runtime exists, when no delivery can \
                 have started",
    },
    AllowedExit {
        file: "main.rs",
        function: "main",
        position: "if let Some(code) = run_audio_worker_if_requested()?",
        form: "process::exit",
        reason: "the audio worker mode, which runs instead of the CLI and never \
                 starts a delivery",
    },
    AllowedExit {
        file: "main.rs",
        function: "main",
        position: TAIL,
        form: "exit_before_runtime",
        reason: "`run` returned an error before the runtime existed",
    },
    AllowedExit {
        file: "commands/compose/interrupt.rs",
        function: "ensure_grace_exit_watcher",
        position: BODY,
        form: "_exit",
        reason: "the grace-exit watcher, armed only by a repeat Ctrl+C while a \
                 terminal lifecycle runs: the user asked to stop",
    },
    AllowedExit {
        file: "commands/compose/interrupt.rs",
        function: "install_ladder",
        position: "arm PressRung::GraceExit > else of fd >= 0",
        form: "_exit",
        reason: "a repeat Ctrl+C when no grace-exit watcher could start: the user \
                 asked to stop now",
    },
    AllowedExit {
        file: "commands/compose/interrupt.rs",
        function: "install_ladder",
        position: "arm PressRung::ForceExit",
        form: "_exit",
        reason: "a repeat Ctrl+C: the user asked to stop now",
    },
    AllowedExit {
        file: "commands/compose/interrupt.rs",
        function: "on_console_interrupt",
        position: "arm ComposeInterruptEffect::GraceExit",
        form: "force_exit",
        reason: "Windows: the grace after a repeat Ctrl+C expired",
    },
    AllowedExit {
        file: "commands/compose/interrupt.rs",
        function: "on_console_interrupt",
        position: "arm ComposeInterruptEffect::ForceExit",
        form: "force_exit",
        reason: "Windows: a repeat Ctrl+C, the user asked to stop now",
    },
    AllowedExit {
        file: "commands/compose/interrupt.rs",
        function: "force_exit",
        position: TAIL,
        form: "ExitProcess",
        reason: "the Windows forced-exit helper; its callers are pinned above",
    },
    AllowedExit {
        file: "commands/compose/interrupt.rs",
        function: "force_exit",
        position: TAIL,
        form: "process::exit",
        reason: "the non-Windows build of the forced-exit helper, present so the \
                 ladder compiles and is tested; nothing calls it there",
    },
];

/// The call forms the detector matches: the three OS exits, then the crate's
/// diverging helpers, whose every call is itself an exit that skips the drain.
const FORMS: &[&str] = &[
    "process::exit",
    "_exit",
    "ExitProcess",
    "exit_before_runtime",
    "force_exit",
];

/// One direct exit call in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Site {
    line: usize,
    form: &'static str,
    function: String,
    position: String,
}

/// The direct exit calls in `source`, ignoring comments, string literals, and
/// a helper's own `fn` definition.
fn exit_sites(source: &str) -> Vec<Site> {
    let sanitized = sanitize(source);
    let mut found = Vec::new();
    for &form in FORMS {
        let needle = form.as_bytes();
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
            if sanitized[..offset].trim_ascii_end().ends_with(b"fn") {
                continue;
            }
            // `std::process::exit` also contains `::exit`, not `_exit`, so the
            // forms cannot double-count one call.
            found.push((offset, form));
        }
    }
    found.sort_by_key(|(offset, _)| *offset);
    if found.is_empty() {
        return Vec::new();
    }
    let offsets: Vec<usize> = found.iter().map(|(offset, _)| *offset).collect();
    found
        .into_iter()
        .zip(site_contexts(source, &offsets))
        .map(|((offset, form), context)| Site {
            line: line_at(source, offset),
            form,
            function: context.function,
            position: context.position,
        })
        .collect()
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

fn matches(entry: &AllowedExit, file: &str, site: &Site) -> bool {
    entry.file == file
        && entry.function == site.function
        && entry.position == site.position
        && entry.form == site.form
}

/// Every mismatch between `census` and `allowlist`, one line each.
fn violations(census: &BTreeMap<String, Vec<Site>>, allowlist: &[AllowedExit]) -> Vec<String> {
    let mut used = vec![false; allowlist.len()];
    let mut problems = Vec::new();
    for (file, sites) in census {
        for site in sites {
            let entry = (0..allowlist.len())
                .find(|&index| !used[index] && matches(&allowlist[index], file, site));
            if let Some(index) = entry {
                used[index] = true;
                continue;
            }
            let described = format!(
                "{file}:{} ({}) in `{}` at `{}`",
                site.line, site.form, site.function, site.position
            );
            if allowlist.iter().any(|entry| entry.file == file) {
                problems.push(format!(
                    "direct exit at an unapproved site in an allowlisted file: {described}; \
                     an approved exit moved or a new one appeared"
                ));
            } else {
                problems.push(format!(
                    "direct exit outside the shutdown path: {described}; return the exit code \
                     to `async_main` so `shutdown::finish` drains deliveries first"
                ));
            }
        }
    }
    for (entry, used) in allowlist.iter().zip(used) {
        if !used {
            problems.push(format!(
                "stale allowlist entry: {} ({}) in `{}` at `{}` matches no live site",
                entry.file, entry.form, entry.function, entry.position
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
    for entry in EXIT_ALLOWLIST {
        assert!(FORMS.contains(&entry.form), "{}: unknown form {}", entry.file, entry.form);
        assert!(
            !entry.reason.trim().is_empty(),
            "{} `{}`: an entry needs a reason",
            entry.file,
            entry.function
        );
    }
}

#[test]
fn the_detector_finds_every_exit_form() {
    let source = "\
fn a() { std::process::exit(1); }
fn b() { unsafe { libc::_exit(130) } }
fn c() { unsafe { windows::Win32::System::Threading::ExitProcess(130) } }
use std::process::exit;
fn d() { shutdown::exit_before_runtime(1) }
fn e() { force_exit(); }
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
            (5, "exit_before_runtime"),
            (6, "force_exit"),
        ]
    );
}

#[test]
fn the_detector_ignores_comments_strings_and_similar_names() {
    let source = r##"
// std::process::exit(1) in a comment
/// libc::_exit(130) in a doc comment
fn force_exit() -> ! { loop {} }
pub(crate) fn exit_before_runtime(code: i32) -> ! { loop {} }
fn a(err: clap::Error) { err.exit() }
const TEXT: &str = "std::process::exit(2)";
const RAW: &str = r#"ExitProcess(3)"#;
fn exit_code() -> i32 { 0 }
fn b() { let _ = ExitProcessLater; }
"##;
    assert_eq!(exit_sites(source), Vec::new());
}

#[test]
fn the_detector_identifies_each_site_by_function_and_position() {
    let source = r#"
use std::process::exit;
fn finish(code: i32) -> ! {
    drain();
    std::process::exit(code)
}
fn early(code: i32) {
    std::process::exit(code);
    drain();
}
fn main() {
    if let Some(code) = worker() {
        std::process::exit(code);
    }
}
mod ladder {
    impl Guard {
        fn press(&self) {
            match rung() {
                Rung::Notice => {}
                Rung::Force => unsafe { libc::_exit(130) },
            }
            let Some(fd) = watcher() else { unsafe { libc::_exit(130) } };
        }
    }
}
"#;
    let identities: Vec<_> = exit_sites(source)
        .into_iter()
        .map(|site| (site.function, site.position))
        .collect();
    let expected = [
        ("<module>", "body"),
        ("finish", TAIL),
        ("early", BODY),
        ("main", "if let Some(code) = worker()"),
        ("ladder::Guard::press", "arm Rung::Force"),
        ("ladder::Guard::press", "let-else Some(fd)"),
    ];
    assert_eq!(
        identities,
        expected
            .iter()
            .map(|(function, position)| (function.to_string(), position.to_string()))
            .collect::<Vec<_>>()
    );
}

fn site(line: usize, form: &'static str, function: &str, position: &str) -> Site {
    Site {
        line,
        form,
        function: function.to_string(),
        position: position.to_string(),
    }
}

fn approved(file: &'static str, function: &'static str, position: &'static str) -> AllowedExit {
    AllowedExit {
        file,
        function,
        position,
        form: "process::exit",
        reason: "approved",
    }
}

#[test]
fn a_site_in_an_unlisted_file_fails() {
    let mut census = BTreeMap::new();
    census.insert(
        "commands/wrap/mod.rs".to_string(),
        vec![site(245, "process::exit", "dispatch", BODY)],
    );
    let problems = violations(&census, &[]);
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0].contains("outside the shutdown path: commands/wrap/mod.rs:245 (process::exit)"),
        "{problems:?}"
    );
}

#[test]
fn an_extra_site_with_an_approved_identity_fails() {
    let mut census = BTreeMap::new();
    census.insert(
        "shutdown.rs".to_string(),
        vec![
            site(66, "process::exit", "finish", TAIL),
            site(80, "process::exit", "finish", TAIL),
        ],
    );
    let problems = violations(&census, &[approved("shutdown.rs", "finish", TAIL)]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("shutdown.rs:80"), "{problems:?}");
}

/// Same file, same count: an approved exit replaced by one in another place.
/// Each case reports the new site as unapproved and the entry it displaced as
/// stale.
#[test]
fn a_replaced_site_in_an_allowlisted_file_fails() {
    let allowlist = [
        approved("shutdown.rs", "finish", TAIL),
        approved("shutdown.rs", "exit_before_runtime", TAIL),
        approved("main.rs", "main", "if let Some(code) = run_audio_worker_if_requested()?"),
    ];
    let replacements = [
        // An exit ahead of the drain is no longer `finish`'s tail call.
        ("shutdown.rs", site(62, "process::exit", "finish", BODY), "finish"),
        // A new early-exit helper, with `exit_before_runtime` routed through it.
        (
            "shutdown.rs",
            site(75, "process::exit", "exit_now", TAIL),
            "exit_before_runtime",
        ),
        // The audio worker's exit moved into ordinary dispatch.
        ("main.rs", site(190, "process::exit", "main", BODY), "main"),
        // Same place, different call form.
        ("shutdown.rs", site(66, "_exit", "finish", TAIL), "finish"),
    ];
    for (file, replacement, displaced) in replacements {
        let mut census: BTreeMap<String, Vec<Site>> = BTreeMap::new();
        for entry in &allowlist {
            let sites = census.entry(entry.file.to_string()).or_default();
            if entry.file == file && entry.function == displaced {
                sites.push(replacement.clone());
            } else {
                sites.push(site(1, entry.form, entry.function, entry.position));
            }
        }
        let problems = violations(&census, &allowlist);
        assert_eq!(problems.len(), 2, "{replacement:?}: {problems:?}");
        assert!(
            problems[0].contains("unapproved site in an allowlisted file")
                && problems[0].contains(&format!("{file}:{}", replacement.line)),
            "{problems:?}"
        );
        assert!(
            problems[1].contains(&format!("stale allowlist entry: {file}"))
                && problems[1].contains(&format!("`{displaced}`")),
            "{problems:?}"
        );
    }
}

/// A forced exit that leaves its forced-interrupt arm for another arm of the
/// same `match` keeps its function and count but not its identity.
#[test]
fn a_forced_exit_moved_out_of_its_branch_fails() {
    let source = r#"
fn install_ladder() {
    match press_rung() {
        PressRung::Notice => unsafe { libc::_exit(130) },
        PressRung::ForceExit => {}
    }
}
"#;
    let mut census = BTreeMap::new();
    census.insert("commands/compose/interrupt.rs".to_string(), exit_sites(source));
    let allowlist = [AllowedExit {
        file: "commands/compose/interrupt.rs",
        function: "install_ladder",
        position: "arm PressRung::ForceExit",
        form: "_exit",
        reason: "forced exit",
    }];
    let problems = violations(&census, &allowlist);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].contains("at `arm PressRung::Notice`"), "{problems:?}");
    assert!(problems[1].starts_with("stale allowlist entry"), "{problems:?}");
}

#[test]
fn an_entry_with_no_live_site_fails() {
    let problems = violations(&BTreeMap::new(), &[approved("sequence.rs", "run", BODY)]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("stale allowlist entry: sequence.rs"), "{problems:?}");
}
