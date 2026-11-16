//! A refused lifecycle action is reported to the author without a `failure`
//! stack that prints `err.msg`, whichever host recorded it.
//!
//! A root sequence's own events fold a refusal into exit 1 and report it; a
//! nested container's step reports every refusal its events recorded; a
//! document's terminal-phase events and a loop gate on any of the four
//! looping owners (a document, the root sequence, a referenced sequence, a
//! referenced group) leave the outcome unchanged and warn. Each report names
//! the refused target and the mutation root, once.
//!
//! The provider is the compiled `claudine-fake-goose` and the editor the
//! compiled `claudine-fake-editor` installed as `code`; a refused `edit`
//! never reaches it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use test_toolkit::fake_editor::FAKE_EDITOR_RECORD;

use crate::common::{CliProcessFixture, init_git_repo, strip_ansi};

/// A repository at the fixture `cwd`, an outside directory beside it that
/// the mutation root does not contain, the fake provider, and the fake
/// editor.
struct Refusals {
    fixture: CliProcessFixture,
    outside: PathBuf,
    editor: PathBuf,
    record: PathBuf,
}

impl Refusals {
    fn stage() -> Self {
        Self::stage_named("lrr")
    }

    /// [`Self::stage`] in a workspace whose name starts with `prefix`.
    fn stage_named(prefix: &str) -> Self {
        let fixture = CliProcessFixture::named(prefix);
        fixture.seed_user_config();
        fixture.initialize_repository();
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
        )
        .expect("the fake provider is built with `test-fixtures`");
        // Not on `PATH`: only `EDITOR` finds it.
        let editors = fixture.workspace_path().join("editors");
        fs::create_dir_all(&editors).unwrap();
        let editor = editors.join(format!("code{}", std::env::consts::EXE_SUFFIX));
        fs::copy(biscuit_test_harness::bin_exe!("claudine-fake-editor"), &editor)
            .expect("the fake editor is built with `test-fixtures`");
        let outside = fixture.workspace_path().join("o");
        fs::create_dir_all(&outside).unwrap();
        let record = fixture.workspace_path().join("editor-record");
        Self { fixture, outside, editor, record }
    }

    fn cwd(&self) -> &Path {
        self.fixture.cwd()
    }

    /// The refused target, in the outside directory.
    fn refused(&self, name: &str) -> PathBuf {
        self.outside.join(name)
    }

    fn write(&self, path: &Path, body: &str) -> PathBuf {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
        path.to_path_buf()
    }

    /// Run `claudine <command> --goose --yolo <entry>`; returns the output
    /// and its plain stdout and stderr together.
    fn run(&self, command: &str, entry: &Path) -> (Output, String) {
        let output = self
            .fixture
            .command()
            .args([command, "--goose", "--yolo", entry.to_str().unwrap()])
            // Wide enough that a report naming temporary paths never wraps.
            .env("COLUMNS", "4000")
            .env("EDITOR", &self.editor)
            .env_remove("VISUAL")
            // A display session, so Linux admits a GUI editor.
            .env("DISPLAY", ":0")
            .env_remove("WAYLAND_DISPLAY")
            .env(FAKE_EDITOR_RECORD, &self.record)
            .env("PROVIDER_MARKER", self.fixture.workspace_path().join("provider-launches"))
            .output()
            .expect("spawn claudine");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output, strip_ansi(&text))
    }

    /// The canonical mutation root a refusal names.
    fn root(&self) -> PathBuf {
        biscuit_file::canonicalize_simplified(self.cwd()).expect("the launch directory exists")
    }

    /// Assert `text` reports the refusal of `target` against the root
    /// exactly once, and that nothing was written there.
    fn assert_refusal_reported(&self, text: &str, target: &Path) {
        let flat = prose(text);
        let target_text = prose(&target.display().to_string());
        assert_eq!(flat.matches("outsidethemutationroot").count(), 1, "one refusal report; output was:\n{text}");
        assert!(flat.contains(&target_text), "names the target {}; output was:\n{text}", target.display());
        assert!(names_root(&flat, &prose(&self.root().display().to_string())), "names the root; output was:\n{text}");
        assert!(!target.exists(), "the refused target was written: {}", target.display());
        assert!(!self.record.exists(), "the editor never runs for a refused target");
    }
}

/// Whether `flat` names the complete `root` after `(root:`: the report shows
/// the full refusal, never the clamped `err.msg`.
fn names_root(flat: &str, root: &str) -> bool {
    flat.contains(&format!("(root:{root})"))
}

/// `text` without whitespace, so a phrase split across lines still matches.
fn prose(text: &str) -> String {
    text.split_whitespace().collect()
}

/// A path as a single-quoted YAML scalar, so Windows backslashes are not
/// read as escapes.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display())
}

// --- root sequence events --------------------------------------------------

/// A file-only sequence whose own `event` refuses `target`, with one
/// successful side-effect task.
fn root_with_event(staged: &Refusals, event: &str, target: &Path) -> PathBuf {
    let ran = quoted(&staged.cwd().join("ran.md"));
    staged.write(
        &staged.cwd().join("seq.md"),
        &format!(
            "---\n{event}:\n  - ensure_file: {}\nsequence:\n  - name: first\n    side_effect:\n      ensure_file: {ran}\n---\n",
            quoted(target),
        ),
    )
}

macro_rules! a_root_event_refusal_is_reported {
    ($($name:ident: $event:literal;)+) => {$(
        #[test]
        fn $name() {
            let staged = Refusals::stage();
            let target = staged.refused("r.md");
            let entry = root_with_event(&staged, $event, &target);

            let (output, text) = staged.run("sequence", &entry);

            assert_eq!(output.status.code(), Some(1), "the refusal fails the sequence; output was:\n{text}");
            assert!(prose(&text).contains("sequencefailed:"), "output was:\n{text}");
            staged.assert_refusal_reported(&text, &target);
        }
    )+};
}

a_root_event_refusal_is_reported! {
    a_root_initialize_refusal_is_reported: "initialize";
    a_root_start_refusal_is_reported: "start";
    a_root_success_refusal_is_reported: "success";
    a_root_finalize_refusal_is_reported: "finalize";
}

/// A child's refusal is reported by its step, and the root `failure` stack's
/// own refusal of another target beside it; neither twice.
#[test]
fn a_root_failure_refusal_is_reported_beside_the_child_refusal_that_led_to_it() {
    let staged = Refusals::stage();
    let (child, cleanup) = (staged.refused("c.md"), staged.refused("f.md"));
    let entry = staged.write(
        &staged.cwd().join("seq.md"),
        &format!(
            "---\nfailure:\n  - ensure_file: {}\nsequence:\n  - name: first\n    side_effect:\n      ensure_file: {}\n---\n",
            quoted(&cleanup),
            quoted(&child),
        ),
    );

    let (output, text) = staged.run("sequence", &entry);

    assert_eq!(output.status.code(), Some(1), "output was:\n{text}");
    let flat = prose(&text);
    assert_eq!(flat.matches("outsidethemutationroot").count(), 2, "each refusal once; output was:\n{text}");
    assert_eq!(flat.matches(&prose(&child.display().to_string())).count(), 1, "output was:\n{text}");
    assert_eq!(flat.matches(&prose(&cleanup.display().to_string())).count(), 1, "output was:\n{text}");
    assert!(flat.contains("step1/1failed:"), "the child's step reports its refusal; output was:\n{text}");
    assert!(flat.contains("sequencefailed:"), "the root reports its own; output was:\n{text}");
    assert!(!child.exists() && !cleanup.exists());
}

/// A referenced group from a foreign library selects a relative target
/// beside itself, outside the caller's tree; its `initialize` refusal is
/// reported by the step that ran it, naming that target.
#[test]
fn a_foreign_referenced_group_initialize_refusal_is_reported_by_its_step() {
    let staged = Refusals::stage();
    let library = staged.fixture.home().join("lib");
    fs::create_dir_all(&library).unwrap();
    // The refusal names the target canonically.
    let library = biscuit_file::canonicalize_simplified(&library).unwrap();
    assert!(init_git_repo(&library), "git init of the prompt library failed");
    let group = staged.write(
        &library.join("groups/foreign.yaml"),
        "kind: group\nname: foreign\ninitialize:\n  - ensure_file: e.md\ntasks:\n  - name: leg\n    side_effect:\n      ensure_file: leg.md\n",
    );
    let entry = staged.write(
        &staged.cwd().join("seq.md"),
        &format!("---\nsequence:\n  - name: foreign\n    group: {}\n---\n", quoted(&group)),
    );

    let (output, text) = staged.run("sequence", &entry);

    assert_eq!(output.status.code(), Some(1), "output was:\n{text}");
    assert!(prose(&text).contains("step1/1failed:"), "output was:\n{text}");
    staged.assert_refusal_reported(&text, &library.join("groups/e.md"));
    assert!(!library.join("groups/leg.md").exists(), "the leg never ran");
}

/// A nested sequence's `failure` refusal is reported by the step after the
/// `initialize` refusal that failed it, each once.
#[test]
fn a_nested_sequence_failure_refusal_is_reported_beside_its_initialize_refusal() {
    let staged = Refusals::stage();
    let (first, cleanup) = (staged.refused("c.md"), staged.refused("f.md"));
    staged.write(
        &staged.cwd().join("parts/child.yaml"),
        &format!(
            "initialize:\n  - ensure_file: {}\nfailure:\n  - ensure_file: {}\nsequence:\n  - name: inner\n    side_effect:\n      ensure_file: {}\n",
            quoted(&first),
            quoted(&cleanup),
            quoted(&staged.cwd().join("ran.md")),
        ),
    );
    let entry = staged.write(&staged.cwd().join("seq.md"), "---\nsequence:\n  - name: child\n    sequence: parts/child.yaml\n---\n");

    let (output, text) = staged.run("sequence", &entry);

    assert_eq!(output.status.code(), Some(1), "output was:\n{text}");
    let flat = prose(&text);
    assert_eq!(flat.matches("step1/1failed:").count(), 2, "the step reports both; output was:\n{text}");
    assert_eq!(flat.matches(&prose(&first.display().to_string())).count(), 1, "output was:\n{text}");
    assert_eq!(flat.matches(&prose(&cleanup.display().to_string())).count(), 1, "output was:\n{text}");
    assert!(!first.exists() && !cleanup.exists());
}

// --- document terminal events --------------------------------------------

macro_rules! a_document_terminal_refusal_warns {
    ($($name:ident: $event:literal;)+) => {$(
        #[test]
        fn $name() {
            let staged = Refusals::stage();
            let target = staged.refused("r.md");
            let entry = staged.write(
                &staged.cwd().join("doc.md"),
                &format!("---\n{}:\n  - ensure_file: {}\n---\nBody\n", $event, quoted(&target)),
            );

            let (output, text) = staged.run("compose", &entry);

            assert_eq!(output.status.code(), Some(0), "a terminal-phase refusal leaves the outcome; output was:\n{text}");
            assert!(prose(&text).contains(&format!("`{}`actionfailed:", $event)), "output was:\n{text}");
            staged.assert_refusal_reported(&text, &target);
        }
    )+};
}

a_document_terminal_refusal_warns! {
    a_document_success_refusal_warns: "success";
    a_document_finalize_refusal_warns: "finalize";
}

/// A `start` refusal fails the document; its `failure` stack's own refusal
/// is reported beside it, not instead of it.
#[test]
fn a_document_failure_refusal_warns_beside_the_start_refusal_that_failed_it() {
    let staged = Refusals::stage();
    let (first, cleanup) = (staged.refused("s.md"), staged.refused("f.md"));
    let entry = staged.write(
        &staged.cwd().join("doc.md"),
        &format!(
            "---\nstart:\n  - ensure_file: {}\nfailure:\n  - ensure_file: {}\n---\nBody\n",
            quoted(&first),
            quoted(&cleanup),
        ),
    );

    let (output, text) = staged.run("compose", &entry);

    assert_eq!(output.status.code(), Some(1), "output was:\n{text}");
    let flat = prose(&text);
    assert!(flat.contains("`failure`actionfailed:"), "output was:\n{text}");
    assert_eq!(flat.matches(&prose(&first.display().to_string())).count(), 1, "output was:\n{text}");
    assert_eq!(flat.matches(&prose(&cleanup.display().to_string())).count(), 1, "output was:\n{text}");
}

/// A `blocked` refusal is reported beside the error that selected `blocked`
/// (here a `proxy` target that does not resolve), which the run reports.
#[test]
fn a_document_blocked_refusal_warns_beside_the_error_that_selected_it() {
    let staged = Refusals::stage();
    let target = staged.refused("b.md");
    let entry = staged.write(
        &staged.cwd().join("doc.md"),
        &format!("---\nstart:\n  - proxy: ./missing.md\nblocked:\n  - ensure_file: {}\n---\nBody\n", quoted(&target)),
    );

    let (output, text) = staged.run("compose", &entry);

    assert_eq!(output.status.code(), Some(1), "output was:\n{text}");
    assert!(prose(&text).contains("`blocked`actionfailed:"), "output was:\n{text}");
    assert!(text.contains("Unresolvable file reference"), "the selecting error is still the run's; output was:\n{text}");
    staged.assert_refusal_reported(&text, &target);
}

// --- loop gates ------------------------------------------------------------

/// The looping owner a gate belongs to.
#[derive(Clone, Copy)]
enum Owner {
    Document,
    RootSequence,
    ReferencedSequence,
    ReferencedGroup,
}

/// The gate's one item.
#[derive(Clone, Copy)]
enum Gate {
    /// `ensure_file` outside the root.
    EnsureOutside,
    /// `edit` outside the root.
    EditOutside,
    /// `ensure_file` inside the root.
    EnsureInside,
    /// `ensure_file` outside the root under `no_error: true`.
    EnsureOutsideTolerated,
    /// An operand that divides by zero.
    DivisionByZero,
}

/// Write `owner`'s files with `gate` as its loop's one gate item; returns
/// the entry document and the gate's target.
fn stage_gate(staged: &Refusals, owner: Owner, gate: Gate) -> (&'static str, PathBuf, PathBuf) {
    let target = match gate {
        Gate::EnsureInside => staged.cwd().join("gate.md"),
        _ => staged.refused("r.md"),
    };
    let item = match gate {
        Gate::EnsureOutside | Gate::EnsureInside => format!("{{ensure_file: {}}}", quoted(&target)),
        Gate::EditOutside => format!("{{edit: {}}}", quoted(&target)),
        Gate::EnsureOutsideTolerated => format!("{{ensure_file: {}, no_error: true}}", quoted(&target)),
        Gate::DivisionByZero => "{ensure_file: \"{{ 1 / 0 }}\"}".to_string(),
    };
    let looping = format!("loop:\n  until: \"loop.count >= 1\"\n  max: 2\n  gate:\n    - {item}\n");
    let task = format!("  - name: task\n    side_effect:\n      ensure_file: {}\n", quoted(&staged.cwd().join("ran.md")));
    let seq = staged.cwd().join("seq.md");
    let entry = match owner {
        Owner::Document => {
            return ("compose", staged.write(&staged.cwd().join("doc.md"), &format!("---\n{looping}---\nBody\n")), target);
        }
        Owner::RootSequence => staged.write(&seq, &format!("---\n{looping}sequence:\n{task}---\n")),
        Owner::ReferencedSequence => {
            staged.write(&staged.cwd().join("parts/child.yaml"), &format!("{looping}sequence:\n{task}"));
            staged.write(&seq, "---\nsequence:\n  - name: child\n    sequence: parts/child.yaml\n---\n")
        }
        Owner::ReferencedGroup => {
            staged.write(&staged.cwd().join("parts/group.yaml"), &format!("kind: group\nname: bundle\n{looping}tasks:\n{task}"));
            staged.write(&seq, "---\nsequence:\n  - name: grouped\n    group: parts/group.yaml\n---\n")
        }
    };
    ("sequence", entry, target)
}

/// Run `owner`'s loop with `gate` and assert the outcome policy: a refused
/// gate action warns and leaves exit 0, `no_error` silences it, an in-root
/// target is written, and an evaluation error fails the owner.
fn check_gate(owner: Owner, gate: Gate) {
    let staged = Refusals::stage();
    let (command, entry, target) = stage_gate(&staged, owner, gate);

    let (output, text) = staged.run(command, &entry);

    match gate {
        Gate::EnsureOutside | Gate::EditOutside => {
            assert_eq!(output.status.code(), Some(0), "a gate refusal leaves the outcome; output was:\n{text}");
            assert!(prose(&text).contains("`loop`actionfailed:"), "output was:\n{text}");
            staged.assert_refusal_reported(&text, &target);
        }
        Gate::EnsureInside => {
            assert_eq!(output.status.code(), Some(0), "output was:\n{text}");
            assert!(target.exists(), "the in-root gate target is written; output was:\n{text}");
            assert!(!text.contains("action failed"), "output was:\n{text}");
        }
        Gate::EnsureOutsideTolerated => {
            assert_eq!(output.status.code(), Some(0), "output was:\n{text}");
            assert!(!prose(&text).contains("outsidethemutationroot"), "`no_error` reports nothing; output was:\n{text}");
            assert!(!target.exists());
        }
        Gate::DivisionByZero => {
            assert_eq!(output.status.code(), Some(1), "an evaluation error fails the owner; output was:\n{text}");
            assert!(text.to_lowercase().contains("evaluation"), "output was:\n{text}");
            assert!(!text.contains("action failed"), "an evaluation error is not an action failure; output was:\n{text}");
        }
    }
}

macro_rules! gate_cells {
    ($($name:ident: $owner:ident, $gate:ident;)+) => {$(
        #[test]
        fn $name() {
            check_gate(Owner::$owner, Gate::$gate);
        }
    )+};
}

gate_cells! {
    a_document_gate_refusal_is_reported: Document, EnsureOutside;
    a_document_gate_edit_refusal_is_reported: Document, EditOutside;
    a_document_gate_writes_inside_the_root: Document, EnsureInside;
    a_document_gate_refusal_under_no_error_is_silent: Document, EnsureOutsideTolerated;
    a_document_gate_evaluation_error_fails_the_document: Document, DivisionByZero;
    a_root_sequence_gate_refusal_is_reported: RootSequence, EnsureOutside;
    a_root_sequence_gate_edit_refusal_is_reported: RootSequence, EditOutside;
    a_root_sequence_gate_writes_inside_the_root: RootSequence, EnsureInside;
    a_root_sequence_gate_refusal_under_no_error_is_silent: RootSequence, EnsureOutsideTolerated;
    a_root_sequence_gate_evaluation_error_fails_the_sequence: RootSequence, DivisionByZero;
    a_referenced_sequence_gate_refusal_is_reported: ReferencedSequence, EnsureOutside;
    a_referenced_sequence_gate_edit_refusal_is_reported: ReferencedSequence, EditOutside;
    a_referenced_sequence_gate_writes_inside_the_root: ReferencedSequence, EnsureInside;
    a_referenced_sequence_gate_refusal_under_no_error_is_silent: ReferencedSequence, EnsureOutsideTolerated;
    a_referenced_sequence_gate_evaluation_error_fails_the_sequence: ReferencedSequence, DivisionByZero;
    a_referenced_group_gate_refusal_is_reported: ReferencedGroup, EnsureOutside;
    a_referenced_group_gate_edit_refusal_is_reported: ReferencedGroup, EditOutside;
    a_referenced_group_gate_writes_inside_the_root: ReferencedGroup, EnsureInside;
    a_referenced_group_gate_refusal_under_no_error_is_silent: ReferencedGroup, EnsureOutsideTolerated;
    a_referenced_group_gate_evaluation_error_fails_the_group: ReferencedGroup, DivisionByZero;
}

// --- long paths ------------------------------------------------------------

/// A 50-character project name: the workspace, and so the mutation root, is
/// the system temp directory plus this component, and the refused target sits
/// in a 50-character directory beside the root, so every refusal is longer
/// than the 240-character `err.msg` clamp.
const LONG_PROJECT: &str = "a-project-directory-with-a-fifty-character-name-xy";
const LONG_OUTSIDE: &str = "an-outside-directory-with-a-fifty-character-name-x";

/// Compose a document whose `start` runs `action` (with `TARGET` replaced by
/// a long outside target) and assert the failure names the whole target and
/// root.
fn check_long_start_refusal(action: &str) -> String {
    let staged = Refusals::stage_named(LONG_PROJECT);
    let target = staged.refused(LONG_OUTSIDE).join("refused-target-file.md");
    let entry = staged.write(
        &staged.cwd().join("doc.md"),
        &format!("---\ncount: 1\nstart:\n  - {}\n---\nBody\n", action.replace("TARGET", &quoted(&target))),
    );

    let (output, text) = staged.run("compose", &entry);

    assert_eq!(output.status.code(), Some(1), "a `start` refusal fails the document; output was:\n{text}");
    staged.assert_refusal_reported(&text, &target);
    let refusal = format!(
        "refusing to write outside the mutation root: {} (root: {})",
        target.display(),
        staged.root().display()
    );
    assert!(refusal.chars().count() > 240, "the fixture exercises the clamp: {refusal}");
    text
}

#[test]
fn a_long_root_survives_in_a_composition_effect_refusal() {
    check_long_start_refusal("ensure_file: TARGET");
}

#[test]
fn a_long_root_survives_in_a_composition_frontmatter_writer_refusal() {
    check_long_start_refusal("set_frontmatter: [TARGET, count, 2]");
}

#[test]
fn a_long_target_and_root_survive_in_a_composition_edit_refusal() {
    let text = check_long_start_refusal("edit: TARGET");
    assert!(prose(&text).contains("couldnoteditthefile"), "output was:\n{text}");
}
