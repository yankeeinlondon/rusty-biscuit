//! Lifecycle side effects write into the **launch** workspace, not the
//! repository that holds the prompt.
//!
//! A shared prompt library such as `~/.claudine/prompts` is often its own git
//! repository. When a prompt from that library runs against a different
//! project, its lifecycle actions (`ensure_file`, `append_line`, …) must target
//! the project being worked on. Before this was fixed the effect engine's
//! mutation root was the *prompt's* repository root, so a write into the launch
//! repo failed with `refusing to write outside the mutation root` as soon as
//! the prompt library became a git repo.

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

    /// Run `claudine compose --claude` on `doc_body`, asserting success and
    /// that no lifecycle write was refused. Returns the plain stderr.
    fn compose(&self, doc_body: &str) -> String {
        fs::write(&self.doc, doc_body).unwrap();
        let output = self
            .fixture
            .command()
            .args(["compose", "--claude", self.doc.to_str().unwrap()])
            .output()
            .expect("spawn claudine");
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert!(
            !stderr.contains("outside the mutation root"),
            "lifecycle writes into the launch repo must not be refused; stderr was:\n{stderr}"
        );
        assert!(
            output.status.success(),
            "compose failed ({:?}); stderr was:\n{stderr}",
            output.status
        );
        stderr
    }
}

#[test]
fn lifecycle_relative_writes_land_in_launch_repo_when_prompt_lives_in_another_repo() {
    let staged = CrossRepo::stage();
    let stderr = staged.compose(
        "---\n\
         title: cross-repo lifecycle writes\n\
         initialize:\n  \
           - {append_line: [\"events.log\", \"initialize\"]}\n\
         finalize:\n  \
           - {append_line: [\"events.log\", \"finalize\"]}\n\
         ---\n\
         Body\n",
    );

    let launch_log = staged.fixture.cwd().join("events.log");
    assert_eq!(
        fs::read_to_string(&launch_log).unwrap_or_default().replace("\r\n", "\n"),
        "initialize\nfinalize\n",
        "a new relative lifecycle target is created in the launch repo; stderr was:\n{stderr}"
    );
    assert!(
        !staged.library.join("events.log").exists(),
        "lifecycle writes must not land in the prompt library's repository"
    );
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
