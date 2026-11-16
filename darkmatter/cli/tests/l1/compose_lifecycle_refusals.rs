//! A refused lifecycle write reported by `md compose` names the whole target
//! and mutation root, in its terminal and its JSON output alike, however long
//! the paths are.
//!
//! Messages were once cut to 240 characters. Each run launches from a
//! 50-character project directory under the fixture's temp workspace and
//! writes to a target in a 50-character directory beside it, so the refusal is
//! always longer than that old limit. The `edit` rows install the fake editor as `code`; a refused target
//! never reaches it.

use std::path::PathBuf;

use serde_json::Value;
use test_toolkit::fake_editor::FAKE_EDITOR_RECORD;

use crate::common::CliProcessFixture;
use crate::editor_support::{editor_dir, install_fake_editor};

const LONG_PROJECT: &str = "a-project-directory-with-a-fifty-character-name-xy";
const LONG_OUTSIDE: &str = "an-outside-directory-with-a-fifty-character-name-x";

/// A launch directory with a long name, the refused target outside it, and
/// the fake editor.
struct LongRoot {
    fixture: CliProcessFixture,
    _editors: tempfile::TempDir,
    editor: PathBuf,
    record: PathBuf,
    root: PathBuf,
    target: PathBuf,
}

impl LongRoot {
    fn stage(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let root = fixture.cwd().join(LONG_PROJECT);
        std::fs::create_dir_all(&root).unwrap();
        let root = biscuit_file::canonicalize_simplified(&root).expect("the launch directory exists");
        let target = fixture.workspace_path().join(LONG_OUTSIDE).join("refused-target-file.md");
        let editors = editor_dir();
        let editor = install_fake_editor(editors.path(), "code");
        let record = fixture.tmp_dir().join("editor-record");
        Self { fixture, _editors: editors, editor, record, root, target }
    }

    /// Compose a document whose `initialize` runs `action` on the target,
    /// with `extra` arguments; returns the exit status, stdout, and stderr.
    fn compose(&self, action: &str, extra: &[&str]) -> (Option<i32>, String, String) {
        let target = serde_json::to_string(&self.target.to_string_lossy()).unwrap();
        let action = action.replace("TARGET", &target);
        let document = self.root.join("doc.md");
        std::fs::write(&document, format!("---\ncount: 1\ninitialize:\n  - {action}\n---\nbody\n")).unwrap();
        let output = self
            .fixture
            .command_builder()
            .ambient_context(&self.root)
            // Wide enough that a status line naming both paths never wraps.
            .rendering_input("COLUMNS", "4000")
            .build()
            .arg("compose")
            .arg(&document)
            .args(extra)
            .env("EDITOR", &self.editor)
            .env_remove("VISUAL")
            .env("DISPLAY", ":0")
            .env_remove("WAYLAND_DISPLAY")
            .env(FAKE_EDITOR_RECORD, &self.record)
            .output()
            .expect("md runs");
        (
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// `text` names the whole target and root, the refusal is longer than
    /// the old 240-character limit, and nothing ran or was written.
    fn assert_full_refusal(&self, text: &str) {
        let flat = prose(text);
        let target = prose(&self.target.display().to_string());
        let root = prose(&self.root.display().to_string());
        assert_eq!(flat.matches("outsidethemutationroot").count(), 1, "one refusal; output was:\n{text}");
        assert!(flat.contains(&format!("{target}(root:{root})")), "names the whole target and root; output was:\n{text}");
        let refusal = format!("refusing to write outside the mutation root: {} (root: {})", self.target.display(), self.root.display());
        assert!(refusal.chars().count() > 240, "the fixture exceeds the old limit: {refusal}");
        assert!(!self.target.exists(), "the refused target was written");
        assert!(!self.record.exists(), "the editor never runs for a refused target");
    }
}

/// `text` without whitespace, so a line wrapped by the renderer still matches.
fn prose(text: &str) -> String {
    text.split_whitespace().collect()
}

/// The one failure diagnostic's message in a JSON run's envelope.
fn json_failure(status: Option<i32>, stdout: &str, stderr: &str) -> String {
    assert_eq!(status, Some(1), "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert_eq!(stderr, "", "a data output keeps stderr empty");
    let envelope: Value = serde_json::from_str(stdout).expect("one JSON envelope");
    let diagnostics = envelope["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1, "{envelope}");
    diagnostics[0]["message"].as_str().expect("a failure message").to_string()
}

#[test]
fn an_effect_refusal_names_the_whole_long_root_on_the_terminal() {
    let staged = LongRoot::stage("refusal-effect-md");

    let (status, _stdout, stderr) = staged.compose("ensure_file: TARGET", &[]);

    assert_eq!(status, Some(1), "{stderr}");
    staged.assert_full_refusal(&stderr);
}

#[test]
fn an_effect_refusal_names_the_whole_long_root_in_json() {
    let staged = LongRoot::stage("refusal-effect-json");

    let (status, stdout, stderr) = staged.compose("ensure_file: TARGET", &["--output", "json"]);

    let message = json_failure(status, &stdout, &stderr);
    assert_eq!(
        message,
        format!("refusing to write outside the mutation root: {} (root: {})", staged.target.display(), staged.root.display())
    );
    staged.assert_full_refusal(&message);
}

#[test]
fn a_frontmatter_writer_refusal_names_the_whole_long_root_in_json() {
    let staged = LongRoot::stage("refusal-frontmatter-json");

    let (status, stdout, stderr) = staged.compose("set_frontmatter: [TARGET, count, 2]", &["--output", "json"]);

    let message = json_failure(status, &stdout, &stderr);
    staged.assert_full_refusal(&message);
}

#[test]
fn an_edit_refusal_names_the_whole_long_target_and_root_on_the_terminal() {
    let staged = LongRoot::stage("refusal-edit-md");

    let (status, _stdout, stderr) = staged.compose("edit: TARGET", &[]);

    assert_eq!(status, Some(1), "{stderr}");
    assert!(prose(&stderr).contains("`initialize[0].edit`couldnoteditthefile:"), "{stderr}");
    staged.assert_full_refusal(&stderr);
}

#[test]
fn an_edit_refusal_names_the_whole_long_target_and_root_in_json() {
    let staged = LongRoot::stage("refusal-edit-json");

    let (status, stdout, stderr) = staged.compose("edit: TARGET", &["--output", "json"]);

    let message = json_failure(status, &stdout, &stderr);
    assert!(message.starts_with("`initialize[0].edit` could not edit the file: "), "{message}");
    staged.assert_full_refusal(&message);
}
