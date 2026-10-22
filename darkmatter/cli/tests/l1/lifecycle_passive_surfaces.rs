//! `md` commands that read a document never run its lifecycle: only
//! `md compose` on a root that authors an event key drives a run.
//!
//! The document's stacks leave evidence if any of them executes:
//! `initialize` and `finalize` each create a file, and `composed` runs the
//! `darkmatter-argv-recorder` fixture (which writes its records file) and
//! prints a line of its own. The control row composes the same document and
//! finds all of it, so an empty result elsewhere is not a broken fixture.

use crate::common::CliProcessFixture;
use std::path::PathBuf;
use test_toolkit::argv_recorder::ARGV_RECORDER_FILE;

/// The line the `composed` stack prints; it appears in the frontmatter only
/// after `stdout: `, never on a line of its own.
const MARKER: &str = "LIFECYCLE-RAN";

const STACKS: &str = "\
initialize:
  - ensure_file: [\"sentinel-a.md\", \"initialize ran\"]
composed:
  - shell: \"argv-recorder composed\"
  - stdout: LIFECYCLE-RAN
finalize:
  - ensure_file: [\"sentinel-c.md\", \"finalize ran\"]
";

struct Workspace {
    fixture: CliProcessFixture,
    records: PathBuf,
}

struct Ran {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let program = fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"), &program)
            .expect("the recorder fixture binary is built with `test-fixtures`");
        // Approved, so the control row can run it; nothing else may.
        fixture.write_file("cwd/.darkmatter-shell-whitelist", "exact argv-recorder composed\n");
        let records = fixture.tmp_dir().join("argv.bin");
        Self { fixture, records }
    }

    fn write(&self, name: &str, content: &str) {
        self.fixture.write_file(format!("cwd/{name}"), content);
    }

    /// `md` with `args`, from the directory holding the documents.
    fn md(&self, args: &[&str]) -> Ran {
        let output = self
            .fixture
            .command()
            .env(ARGV_RECORDER_FILE, &self.records)
            .args(args)
            .output()
            .expect("md runs");
        Ran {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Which pieces of lifecycle evidence exist.
    fn evidence(&self, ran: &Ran) -> Vec<&'static str> {
        let printed = |text: &str| text.lines().any(|line| line.trim() == MARKER);
        [
            ("initialize", self.fixture.cwd().join("sentinel-a.md").exists()),
            ("composed shell", self.records.exists()),
            ("composed stdout", printed(&ran.stdout) || printed(&ran.stderr)),
            ("finalize", self.fixture.cwd().join("sentinel-c.md").exists()),
        ]
        .into_iter()
        .filter_map(|(piece, present)| present.then_some(piece))
        .collect()
    }
}

/// One `md` invocation on the lifecycle document that must leave no
/// evidence and succeed.
fn assert_passive(name: &str, args: &[&str]) {
    let workspace = Workspace::new(name);
    workspace.write("doc.md", &format!("---\ntitle: Passive\n{STACKS}---\n# Passive body\n"));

    let ran = workspace.md(args);

    assert_eq!(ran.status, Some(0), "{args:?}: {}{}", ran.stdout, ran.stderr);
    assert_eq!(workspace.evidence(&ran), Vec::<&str>::new(), "{args:?} ran a stack:\n{}\n{}", ran.stdout, ran.stderr);
}

macro_rules! passive_surfaces {
    ($( $name:ident: [$($arg:literal),+] ),+ $(,)?) => {
        $(#[test]
        fn $name() {
            assert_passive(stringify!($name), &[$($arg),+]);
        })+
    };
}

passive_surfaces! {
    schema_validate_runs_no_stack: ["schema", "validate", "doc.md"],
    render_runs_no_stack: ["render", "doc.md"],
    the_default_render_runs_no_stack: ["doc.md"],
    get_runs_no_stack: ["get", "doc.md", "title"],
    toc_runs_no_stack: ["toc", "doc.md"],
    hash_runs_no_stack: ["hash", "doc.md"],
    graph_runs_no_stack: ["graph", "doc.md"],
    // Report-only `--shell` lists the lifecycle command without running it.
    compose_shell_report_runs_no_stack: ["compose", "doc.md", "--shell"],
}

/// The control: composing the same document leaves every piece of evidence.
#[test]
fn composing_the_root_runs_every_stack_the_passive_rows_did_not() {
    let workspace = Workspace::new("passive-control");
    workspace.write("doc.md", &format!("---\ntitle: Passive\n{STACKS}---\n# Passive body\n"));

    let ran = workspace.md(&["compose", "doc.md"]);

    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(workspace.evidence(&ran), ["initialize", "composed shell", "composed stdout", "finalize"]);
}

/// A child's stacks never fire, even when the root that transcludes it
/// authors no lifecycle of its own; its body is still composed.
#[test]
fn a_child_transcluded_by_a_root_without_a_lifecycle_fires_nothing() {
    let workspace = Workspace::new("passive-child");
    workspace.write("child.md", &format!("---\n{STACKS}---\nchild-body\n"));
    workspace.write("doc.md", "---\ntitle: Root\n---\n::file ./child.md\n");

    let ran = workspace.md(&["compose", "doc.md"]);

    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(ran.stdout.contains("child-body"), "{}", ran.stdout);
    assert_eq!(workspace.evidence(&ran), Vec::<&str>::new(), "{}\n{}", ran.stdout, ran.stderr);
}
