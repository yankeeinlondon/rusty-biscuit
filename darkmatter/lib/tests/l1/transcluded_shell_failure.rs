//! An unhandled shell failure fails the composition wherever its span is
//! written: in the root document, in a transcluded file, or two transclusions
//! deep. The lenient transclusion fallback (a notice in place of the file)
//! still applies to failures that are not shell failures.
//!
//! Commands are portable: `git` in a private repository exits non-zero, and
//! this test binary re-executed in a helper mode never exits.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;

/// A positional argument: libtest reads it as a name filter, which under
/// `--exact` matches nothing, so the harness ignores it.
const HELPER_ARG: &str = "dm-transcluded-shell-helper=";

/// What a re-executed copy of this binary does instead of testing.
#[test]
fn helper_process_entrypoint() {
    if std::env::args().any(|arg| arg == format!("{HELPER_ARG}sleep")) {
        loop {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

fn sleeping_command() -> String {
    let exe = std::env::current_exe().expect("current test executable");
    let module = module_path!()
        .split_once("::")
        .map(|(_, rest)| rest)
        .expect("module path has a crate segment");
    format!(
        "'{}' --exact {module}::helper_process_entrypoint --nocapture {HELPER_ARG}sleep",
        exe.display()
    )
}

const FAILS: &str = "git rev-parse --verify nope";
const MISSING: &str = "dm-transcluded-shell-no-such-program --version";

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    git(dir.path(), &["init", "-q"]);
    dir
}

#[derive(Clone, Copy, Debug)]
enum Span {
    Shell,
    ShellBlock,
    Frontmatter,
}

impl Span {
    /// A whole file whose only executable span runs `command`.
    fn file(self, command: &str, handled: bool) -> String {
        match (self, handled) {
            (Self::Shell, false) => format!("Facts:\n\n::shell {command}\n"),
            (Self::Shell, true) => format!("Facts:\n\n::shell --when-error \"(fallback)\" {command}\n"),
            (Self::ShellBlock, false) => format!("Facts:\n\n::shell-block\n{command}\n::end-block\n"),
            (Self::ShellBlock, true) => {
                format!("Facts:\n\n::shell-block when_error=\"(fallback)\"\n{command}\n::end-block\n")
            }
            (Self::Frontmatter, false) => {
                let yaml = format!("$({command})").replace('\'', "''");
                format!("---\nv: '{yaml}'\n---\nFacts: {{{{ v }}}}\n")
            }
            (Self::Frontmatter, true) => unreachable!("frontmatter `$( … )` has no fallback text"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Placement {
    Inline,
    Transcluded,
    Nested,
}

/// Writes `root.md` with the span placed as asked, and composes it with the
/// approval set discovery computes, as a caller that audits first does.
fn compose(
    dir: &Path,
    placement: Placement,
    span_file: &str,
    timeout: Duration,
    allow_timeout: bool,
) -> Result<String, String> {
    let root = dir.join("root.md");
    match placement {
        Placement::Inline => std::fs::write(&root, span_file),
        Placement::Transcluded => {
            std::fs::write(dir.join("part.md"), span_file).expect("write part");
            std::fs::write(&root, "# Root\n\n::file ./part.md\n\nafter\n")
        }
        Placement::Nested => {
            std::fs::write(dir.join("part.md"), span_file).expect("write part");
            std::fs::write(dir.join("mid.md"), "Middle\n\n::file ./part.md\n").expect("write mid");
            std::fs::write(&root, "# Root\n\n::file ./mid.md\n\nafter\n")
        }
    }
    .expect("write root");
    let document = Markdown::try_from(root.as_path()).map_err(|error| error.to_string())?;
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_timeout(timeout)
        .with_allow_shell_timeout(allow_timeout);
    let approved = document
        .compose_preflight(&options)
        .map_err(|error| error.to_string())?
        .approval_set()
        .into_iter()
        .collect();
    document
        .compose_with(options.with_pre_approved_commands(approved))
        .map(|(composed, _report)| composed.content().to_string())
        .map_err(|error| error.to_string())
}

#[test]
fn unhandled_shell_failure_fails_the_composition_in_every_placement() {
    let sleep = sleeping_command();
    let failures = [
        ("non-zero exit", FAILS, Duration::from_secs(60), "exit"),
        ("missing program", MISSING, Duration::from_secs(60), "not found"),
        ("timeout", sleep.as_str(), Duration::from_secs(1), "timed out"),
    ];
    let mut rows = Vec::new();
    for span in [Span::Shell, Span::ShellBlock, Span::Frontmatter] {
        for failure in failures {
            for placement in [Placement::Inline, Placement::Transcluded, Placement::Nested] {
                rows.push((span, failure, placement));
            }
        }
    }
    // Rows run concurrently, each in its own repository: a missing program is
    // looked up as a login-shell alias before it is reported, which takes
    // seconds on a host with a heavy shell profile.
    let problems: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = rows
            .iter()
            .map(|&(span, (failure, command, timeout, fragment), placement)| {
                scope.spawn(move || {
                    let dir = repo();
                    let row = format!("{span:?} / {failure} / {placement:?}");
                    match compose(dir.path(), placement, &span.file(command, false), timeout, false) {
                        Ok(composed) => Some(format!("{row}: composed instead of failing:\n{composed}")),
                        Err(error) if !error.contains(fragment) => {
                            Some(format!("{row}: expected `{fragment}` in: {error}"))
                        }
                        Err(_) => None,
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().expect("row thread"))
            .collect()
    });
    assert!(problems.is_empty(), "{}", problems.join("\n\n"));
}

#[test]
fn handled_shell_failure_in_a_transcluded_file_renders_its_fallback() {
    let dir = repo();
    for span in [Span::Shell, Span::ShellBlock] {
        for placement in [Placement::Inline, Placement::Transcluded, Placement::Nested] {
            let composed = compose(
                dir.path(),
                placement,
                &span.file(FAILS, true),
                Duration::from_secs(60),
                false,
            )
            .unwrap_or_else(|error| panic!("{span:?} / {placement:?}: {error}"));
            assert!(composed.contains("(fallback)"), "{span:?} / {placement:?}: {composed}");
            assert!(!composed.contains("Could not transclude"), "{span:?} / {placement:?}: {composed}");
        }
    }
}

#[test]
fn allowed_timeout_in_a_transcluded_file_is_not_a_failure() {
    let dir = repo();
    let composed = compose(
        dir.path(),
        Placement::Transcluded,
        &Span::ShellBlock.file(&sleeping_command(), false),
        Duration::from_secs(1),
        true,
    )
    .expect("an allowed timeout inserts nothing and composes");
    assert!(composed.contains("Facts:"), "{composed}");
    assert!(!composed.contains("Could not transclude"), "{composed}");
}

/// Control: a child that fails for a reason other than a shell span keeps the
/// lenient fallback, so only shell failures changed.
#[test]
fn non_shell_failure_in_a_transcluded_file_still_becomes_a_notice() {
    let dir = repo();
    let composed = compose(
        dir.path(),
        Placement::Transcluded,
        "child {{ 1 / 0 }}\n",
        Duration::from_secs(60),
        false,
    )
    .expect("a non-shell child failure is tolerated");
    assert!(composed.contains("_Could not transclude `part.md`_"), "{composed}");
    assert!(composed.contains("after"), "{composed}");
}
