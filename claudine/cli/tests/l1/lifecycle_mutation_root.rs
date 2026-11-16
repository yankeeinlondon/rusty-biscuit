//! Lifecycle side effects are bounded by the **caller's** tree, while their
//! targets are selected relative to the document that authored them.
//!
//! A shared prompt library such as `~/.claudine/prompts` is often its own git
//! repository. When a prompt from that library runs against a different
//! project, its lifecycle actions (`ensure_file`, `append_line`, …) may write
//! only inside the project being worked on: an absolute target there is
//! allowed, while a relative target resolves beside the prompt, in the
//! library, and is refused before anything is created.
//!
//! The same boundary holds on the sequence routes: a sequence's own
//! `initialize` and its side-effect tasks write inside the caller's tree, not
//! inside the directory that holds the sequence document.
//!
//! Outside any repository the boundary is the launch directory, even when the
//! prompt lives inside a repository. A failed discovery of the launch
//! repository is not "outside any repository": it never widens the boundary to
//! that repository's top, and every lifecycle host stops the run with the
//! discovery diagnostic before any file action.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;
use common::mutation_paths::{HEALTHY_GIT_CONFIG, write_git_dir};
use common::{CliProcessFixture, strip_ansi, write_executable};

#[cfg(windows)]
const PROVIDER_SHIM: (&str, &str) = ("claude.cmd", "@echo off\r\nexit /b 0\r\n");

#[cfg(not(windows))]
const PROVIDER_SHIM: (&str, &str) = ("claude", "#!/bin/sh\ncat > /dev/null\nexit 0\n");

/// A fixture whose prompt lives in its own git repository under `home`,
/// separate from the git repository the run is launched from (`cwd`).
struct CrossRepo {
    fixture: CliProcessFixture,
    library: PathBuf,
    doc: PathBuf,
}

impl CrossRepo {
    fn stage() -> Self {
        let staged = Self::stage_outside_a_repository();
        write_git_dir(staged.fixture.cwd(), HEALTHY_GIT_CONFIG);
        staged
    }

    /// [`Self::stage`] without a repository at `cwd`: the launch directory is
    /// in no working tree, so it is the mutation root itself.
    fn stage_outside_a_repository() -> Self {
        let fixture = CliProcessFixture::named("lifecycle-mutation-root");
        fixture.seed_user_config();

        let library = fixture.home().join("prompt-library");
        fs::create_dir_all(library.join("prompts")).unwrap();
        write_git_dir(&library, HEALTHY_GIT_CONFIG);

        let (shim_name, shim_body) = PROVIDER_SHIM;
        write_executable(&fixture.bin_dir().join(shim_name), shim_body);

        let doc = library.join("prompts").join("router.md");
        Self { fixture, library, doc }
    }

    /// Run `claudine compose --claude` on `doc_body`; returns its status and
    /// plain stderr.
    fn run(&self, doc_body: &str) -> (std::process::ExitStatus, String) {
        self.run_at("compose", &self.doc, doc_body)
    }

    /// Run `claudine <command> --claude` on `doc_body` written at `path`;
    /// returns its status and plain stderr.
    fn run_at(&self, command: &str, path: &Path, doc_body: &str) -> (std::process::ExitStatus, String) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, doc_body).unwrap();
        let output = self
            .fixture
            .command()
            .args([command, "--claude", path.to_str().unwrap()])
            .output()
            .expect("spawn claudine");
        (output.status, strip_ansi(&String::from_utf8_lossy(&output.stderr)))
    }

    /// Run `claudine sequence --claude` on a sequence document at `path`;
    /// returns its status and plain output (the sequence route reports on
    /// both streams).
    fn sequence(&self, path: &std::path::Path, body: &str) -> (std::process::ExitStatus, String) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
        let output = self
            .fixture
            .command()
            .args(["sequence", "--claude", path.to_str().unwrap()])
            .output()
            .expect("spawn claudine");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status, strip_ansi(&combined))
    }

    /// [`Self::run`], asserting success and that no lifecycle write was
    /// refused. Returns the plain stderr.
    fn compose(&self, doc_body: &str) -> String {
        let (status, stderr) = self.run(doc_body);
        assert!(
            !stderr.contains("outside the mutation root"),
            "lifecycle writes into the launch repo must not be refused; stderr was:\n{stderr}"
        );
        assert!(
            status.success(),
            "compose failed ({status:?}); stderr was:\n{stderr}"
        );
        stderr
    }
}

/// A relative target authored by a foreign prompt resolves beside the prompt
/// (its first candidate), outside the caller's tree, so it is refused before
/// anything is created on either side.
#[test]
fn a_relative_write_from_a_prompt_in_another_repo_is_refused_and_creates_nothing() {
    let staged = CrossRepo::stage();
    let (status, stderr) = staged.run(
        "---\n\
         title: cross-repo lifecycle writes\n\
         initialize:\n  \
           - {append_line: [\"events.log\", \"initialize\"]}\n\
         ---\n\
         Body\n",
    );

    assert!(!status.success(), "the refused write fails the run; stderr was:\n{stderr}");
    let prose = stderr.replace('┃', " ").split_whitespace().collect::<String>();
    assert!(prose.contains("outsidethemutationroot"), "stderr was:\n{stderr}");
    assert!(prose.contains("events.log"), "names the target; stderr was:\n{stderr}");
    // The status line carries the complete message, so the root survives to
    // its last component. Only that component is compared: the root's full
    // spelling (short names, prefixes) is the host's, and the library tests
    // pin it.
    let root_name = staged.fixture.cwd().file_name().unwrap().to_string_lossy().into_owned();
    assert!(prose.contains("(root:"), "names the root; stderr was:\n{stderr}");
    assert!(prose.contains(&format!("{root_name})")), "keeps the root's end; stderr was:\n{stderr}");
    assert!(!prose.contains('…'), "nothing is shortened; stderr was:\n{stderr}");
    assert!(!staged.library.join("prompts/events.log").exists(), "nothing beside the prompt");
    assert!(!staged.library.join("events.log").exists(), "nothing at the library root");
    assert!(!staged.fixture.cwd().join("events.log").exists(), "never root-joined in the caller");
}

/// The incident shape: an `initialize` `ensure_file` targeting an absolute path
/// inside the launch repo (the original router prompt derives it from a `spec`
/// parameter).
#[test]
fn initialize_ensure_file_into_launch_repo_is_allowed_from_foreign_prompt_repo() {
    let staged = CrossRepo::stage();
    let target = staged.fixture.cwd().join("features").join("implementation-log.md");
    // Single-quoted YAML so Windows backslashes are not read as escapes.
    staged.compose(&format!(
        "---\n\
         title: cross-repo ensure_file\n\
         initialize:\n  \
           - ensure_file: '{target}'\n\
         ---\n\
         Body\n",
        target = target.display(),
    ));

    assert!(target.exists(), "ensure_file must create {}", target.display());
}

/// A sequence container's `initialize` and a side-effect task both writing
/// `target`, relative to the sequence document.
fn sequence_writing(target: &str) -> String {
    format!(
        "---\n\
         initialize:\n  \
           - append_line: [\"{target}\", \"initialize\"]\n\
         sequence:\n  \
           - name: first\n    \
             side_effect:\n      \
               append_line:\n        \
                 - {target}\n        \
                 - task\n\
         ---\n"
    )
}

/// A sequence in a subdirectory of the launch repository reaches the
/// repository top through `../`: the boundary is the caller's tree, not the
/// sequence's directory, for its container event and for its task alike.
#[test]
fn a_sequence_in_a_subdirectory_writes_to_the_launch_root_from_initialize_and_a_task() {
    let staged = CrossRepo::stage();
    let sequence = staged.fixture.cwd().join("sequences").join("seq.md");
    let (status, output) = staged.sequence(&sequence, &sequence_writing("../events.log"));

    assert!(status.success(), "the run succeeds; output was:\n{output}");
    assert!(!output.contains("outside the mutation root"), "output was:\n{output}");
    let log = fs::read_to_string(staged.fixture.cwd().join("events.log")).expect("the launch root holds the log");
    assert_eq!(log.lines().collect::<Vec<_>>(), ["initialize", "task"]);
    assert!(!staged.fixture.cwd().join("sequences/events.log").exists());
}

/// A sequence from a foreign library writing beside itself from its
/// container `initialize` is refused before anything is created; the run
/// reports the container's own refusal, so no `failure` stack is needed to
/// learn why it failed. One run per test, so the two hosts run in parallel.
#[test]
fn a_foreign_sequence_cannot_write_beside_itself_from_initialize() {
    let staged = CrossRepo::stage();
    let sequence = staged.library.join("prompts").join("seq.md");

    let (status, output) = staged.sequence(&sequence, &sequence_writing("events.log"));

    assert!(!status.success(), "the refused container write fails the run; output was:\n{output}");
    let prose = prose(&output);
    assert!(prose.contains("sequencefailed:refusingtowriteoutsidethemutationroot"), "output was:\n{output}");
    assert!(prose.contains("events.log"), "names the target; output was:\n{output}");
    assert!(prose.contains("(root:"), "names the root; output was:\n{output}");
    assert!(!staged.library.join("prompts/events.log").exists(), "nothing beside the sequence");
    assert!(!staged.fixture.cwd().join("events.log").exists(), "never root-joined in the caller");
}

/// A side-effect task of a foreign sequence writing beside the sequence is
/// refused before anything is created.
#[test]
fn a_foreign_sequence_cannot_write_beside_itself_from_a_task() {
    let staged = CrossRepo::stage();
    let sequence = staged.library.join("prompts").join("seq.md");
    let task_only = "---\n\
         sequence:\n  \
           - name: first\n    \
             side_effect:\n      \
               append_line:\n        \
                 - events.log\n        \
                 - task\n\
         ---\n";

    let (status, output) = staged.sequence(&sequence, task_only);

    assert!(!status.success(), "the refused task write fails the run; output was:\n{output}");
    assert!(prose(&output).contains("outsidethemutationroot"), "output was:\n{output}");
    assert!(!staged.library.join("prompts/events.log").exists(), "nothing beside the sequence");
    assert!(!staged.fixture.cwd().join("events.log").exists(), "never root-joined in the caller");
}

/// A refusal's stderr with the status-line gutter and wrapping removed, so a
/// phrase split across lines still matches.
fn prose(stderr: &str) -> String {
    stderr.replace('┃', " ").split_whitespace().collect()
}

/// Launched outside any repository, the launch directory is the boundary even
/// though the prompt sits inside its library's repository: a relative target
/// beside the prompt is refused, and a `{{ ctx.cwd }}` target is written.
#[test]
fn outside_a_repository_the_launch_directory_bounds_a_prompt_from_inside_one() {
    let staged = CrossRepo::stage_outside_a_repository();
    let cwd = staged.fixture.cwd();
    // The precondition the case depends on: nothing above the launch
    // directory is a working tree, so no repository can widen the boundary.
    if let Some(repository) = cwd.ancestors().find(|dir| dir.join(".git").exists()) {
        panic!("fixture precondition: {} is inside the repository {}", cwd.display(), repository.display());
    }

    let (status, stderr) = staged.run(
        "---\n\
         initialize:\n  \
           - append_line: [\"events.log\", \"initialize\"]\n\
         ---\n\
         Body\n",
    );
    assert!(!status.success(), "the refused write fails the run; stderr was:\n{stderr}");
    let refusal = prose(&stderr);
    assert!(refusal.contains("outsidethemutationroot"), "stderr was:\n{stderr}");
    assert!(refusal.contains("events.log"), "names the target; stderr was:\n{stderr}");
    assert!(refusal.contains("(root:"), "names the root; stderr was:\n{stderr}");
    assert!(!staged.library.join("prompts/events.log").exists(), "nothing beside the prompt");
    assert!(!cwd.join("events.log").exists(), "never root-joined in the launch directory");

    staged.compose(
        "---\n\
         initialize:\n  \
           - append_line: [\"{{ ctx.cwd }}/events.log\", \"initialize\"]\n\
         ---\n\
         Body\n",
    );
    let log = fs::read_to_string(cwd.join("events.log")).expect("the launch directory holds the log");
    assert_eq!(log.lines().collect::<Vec<_>>(), ["initialize"]);
    assert!(!staged.library.join("prompts/events.log").exists(), "still nothing beside the prompt");
}

/// `inline-compose` selects relative targets beside its document like
/// `compose` does: a document at the launch directory creates a bare target
/// there, and a nested target together with its missing parents.
#[test]
fn inline_compose_creates_bare_and_nested_relative_targets_beside_a_document_at_the_launch_directory() {
    let staged = CrossRepo::stage();
    let cwd = staged.fixture.cwd();
    let doc = cwd.join("status.md");
    // `inline-compose` fails a run whose agent leaves the body empty, so this
    // provider writes one.
    let (shim_name, _) = PROVIDER_SHIM;
    let shim_body = if cfg!(windows) {
        format!("@echo off\r\necho Body>>\"{}\"\r\nexit /b 0\r\n", doc.display())
    } else {
        format!("#!/bin/sh\ncat > /dev/null\nprintf 'Body\\n' >> '{}'\n", doc.display())
    };
    write_executable(&staged.fixture.bin_dir().join(shim_name), &shim_body);
    let (status, stderr) = staged.run_at(
        "inline-compose",
        &doc,
        "---\n\
         prompt: Summarize the status.\n\
         initialize:\n  \
           - ensure_file: [\"notes.md\", \"x\"]\n  \
           - ensure_file: [\"a/b/c.md\", \"y\"]\n\
         ---\n",
    );

    assert!(!stderr.contains("outside the mutation root"), "stderr was:\n{stderr}");
    assert!(status.success(), "inline-compose failed ({status:?}); stderr was:\n{stderr}");
    assert_eq!(fs::read_to_string(cwd.join("notes.md")).unwrap(), "x");
    assert_eq!(fs::read_to_string(cwd.join("a/b/c.md")).unwrap(), "y");
}

/// Launched from a subdirectory of a repository whose discovery fails (a
/// `.git/config` that git and sniff refuse to load), a write to the
/// repository top, outside the launch directory, never happens: a failed
/// discovery can only narrow the boundary, never widen it.
#[test]
fn a_failed_launch_discovery_never_widens_the_boundary_to_the_repository_top() {
    let staged = CrossRepo::stage();
    let top = staged.fixture.cwd();
    let launch = top.join("package");
    fs::create_dir_all(&launch).unwrap();
    let target = top.join("events.log");
    // Single-quoted YAML so Windows backslashes are not read as escapes.
    fs::write(
        &staged.doc,
        format!(
            "---\n\
             initialize:\n  \
               - ensure_file: '{target}'\n\
             ---\n\
             Body\n",
            target = target.display(),
        ),
    )
    .unwrap();
    let run = || {
        let output = staged
            .fixture
            .command_builder()
            .ambient_context(&launch)
            .build()
            .args(["compose", "--claude", staged.doc.to_str().unwrap()])
            .output()
            .expect("spawn claudine");
        (output.status, strip_ansi(&String::from_utf8_lossy(&output.stderr)))
    };

    // Control: with a healthy repository the top is inside the boundary.
    let (status, stderr) = run();
    assert!(status.success(), "a healthy repository admits the write; stderr was:\n{stderr}");
    assert!(target.exists(), "the control run writes at the repository top");
    fs::remove_file(&target).unwrap();

    fs::write(top.join(".git/config"), "[core\n garbage = = =\n").unwrap();
    let (status, stderr) = run();
    assert!(!status.success(), "the run must not succeed; stderr was:\n{stderr}");
    assert!(!target.exists(), "nothing written at the repository top; stderr was:\n{stderr}");
}

/// A launch repository whose discovery fails (`.git/config` cannot be
/// parsed), and a prompt library in its own healthy repository.
struct UnreadableLaunch {
    fixture: CliProcessFixture,
    library: PathBuf,
}

impl UnreadableLaunch {
    fn stage() -> Self {
        let fixture = CliProcessFixture::named("lifecycle-discovery-failure");
        fixture.seed_user_config();
        write_git_dir(fixture.cwd(), "[core\n garbage = = =\n");
        let library = fixture.home().join("prompt-library");
        write_git_dir(&library, HEALTHY_GIT_CONFIG);
        let (shim_name, shim_body) = PROVIDER_SHIM;
        write_executable(&fixture.bin_dir().join(shim_name), shim_body);
        Self { fixture, library }
    }

    /// An absolute marker path below the launch directory, which the
    /// launch-directory fallback would admit.
    fn marker(&self, name: &str) -> PathBuf {
        self.fixture.cwd().join("markers").join(name)
    }

    /// Run `claudine <command> --claude <library>/<file>` once; returns its
    /// status and plain combined output.
    fn run(&self, command: &str, file: &str, body: &str) -> (std::process::ExitStatus, String) {
        let path = self.library.join(file);
        fs::write(&path, body).unwrap();
        let output = self
            .fixture
            .command()
            .args([command, "--claude", path.to_str().unwrap()])
            .output()
            .expect("spawn claudine");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status, strip_ansi(&combined))
    }
}

/// One host family per row: `(command, file, body)` where `body` names the
/// markers `{first}` and `{second}` with absolute targets under the launch
/// directory. Each run must fail with the discovery diagnostic before either
/// marker is written.
macro_rules! a_failed_launch_discovery_stops_the_run {
    ($($name:ident: $command:literal, $file:literal, $body:literal;)+) => {$(
        #[test]
        fn $name() {
            let staged = UnreadableLaunch::stage();
            let (first, second) = (staged.marker("first.md"), staged.marker("second.md"));
            // Single-quoted YAML so Windows backslashes are not read as escapes.
            let body = $body
                .replace("{first}", &first.display().to_string())
                .replace("{second}", &second.display().to_string());
            let (status, output) = staged.run($command, $file, &body);

            assert!(!status.success(), "the run must fail; output was:\n{output}");
            let prose = prose(&output);
            assert!(prose.contains("lifecycleactionscannotrun"), "output was:\n{output}");
            assert!(prose.contains("repositorydiscoveryatthelaunchdirectory"), "output was:\n{output}");
            assert!(!first.exists(), "no file action ran; output was:\n{output}");
            assert!(!second.exists(), "no file action ran; output was:\n{output}");
        }
    )+};
}

a_failed_launch_discovery_stops_the_run! {
    compose_with_initialize_stops_before_the_staged_boot: "compose", "staged.md",
        "---\ninitialize:\n  - ensure_file: '{first}'\nstart:\n  - ensure_file: '{second}'\n---\nBody\n";
    compose_without_initialize_stops_before_start: "compose", "start.md",
        "---\nstart:\n  - ensure_file: '{first}'\nfinalize:\n  - ensure_file: '{second}'\n---\nBody\n";
    inline_compose_stops_before_initialize_and_start: "inline-compose", "inline.md",
        "---\nprompt: Summarize.\ninitialize:\n  - ensure_file: '{first}'\nstart:\n  - ensure_file: '{second}'\n---\n";
    a_looping_compose_stops_before_its_first_pass: "compose", "loop.md",
        "---\nloop:\n  max: 1\n  until: 'loop.count >= 1'\ninitialize:\n  - ensure_file: '{first}'\nstart:\n  - ensure_file: '{second}'\n---\nBody\n";
    a_sequence_stops_before_initialize_and_its_task: "sequence", "seq.md",
        "---\ninitialize:\n  - ensure_file: '{first}'\nsequence:\n  - name: first\n    side_effect:\n      ensure_file: '{second}'\n---\n";
}
