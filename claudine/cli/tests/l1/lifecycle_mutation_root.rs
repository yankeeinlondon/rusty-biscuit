//! Lifecycle side effects are bounded by the **caller's** tree, while their
//! targets are selected relative to the document that authored them.
//!
//! A shared prompt library such as `~/.claudine/prompts` is often its own git
//! repository. When a prompt from that library runs against a different
//! project, its lifecycle actions (`ensure_file`, `append_line`, …) may write
//! only inside the project being worked on: an absolute target there is
//! allowed, while a relative target resolves beside the prompt, in the
//! library, and is refused before anything is created.

use std::fs;
use std::path::PathBuf;

use crate::common;
use common::{CliProcessFixture, init_git_repo, strip_ansi, write_executable};

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
        let fixture = CliProcessFixture::named("lifecycle-mutation-root");
        fixture.seed_user_config();
        fixture.initialize_repository();

        let library = fixture.home().join("prompt-library");
        fs::create_dir_all(library.join("prompts")).unwrap();
        assert!(init_git_repo(&library), "git init of the prompt library failed");

        let (shim_name, shim_body) = PROVIDER_SHIM;
        write_executable(&fixture.bin_dir().join(shim_name), shim_body);

        let doc = library.join("prompts").join("router.md");
        Self { fixture, library, doc }
    }

    /// Run `claudine compose --claude` on `doc_body`; returns its status and
    /// plain stderr.
    fn run(&self, doc_body: &str) -> (std::process::ExitStatus, String) {
        fs::write(&self.doc, doc_body).unwrap();
        let output = self
            .fixture
            .command()
            .args(["compose", "--claude", self.doc.to_str().unwrap()])
            .output()
            .expect("spawn claudine");
        (output.status, strip_ansi(&String::from_utf8_lossy(&output.stderr)))
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
    // The status line is length-clamped, so only the root's label survives
    // here; the library tests pin the full root text.
    assert!(prose.contains("(root:"), "names the root; stderr was:\n{stderr}");
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
