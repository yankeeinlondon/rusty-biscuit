//! Lifecycle shell approval under `md compose`: what a `set` can and cannot
//! do to an approved command, the typed-template rules, and a guarded
//! command, each matching what `claudine compose` does with the same stack.
//!
//! Every command is the `darkmatter-argv-recorder` fixture copied into the
//! fixture `bin` as `argv-recorder`, so "what ran" is the argument vectors
//! the program itself recorded, on every platform. There is no terminal, so
//! a command no whitelist rule covers is refused rather than prompted for.
//! Each stack marks its events with `info: "event:<name>"` lines.

use crate::common::{CliProcessFixture, unwrapped};
use std::path::PathBuf;
use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};

struct Document {
    fixture: CliProcessFixture,
    records: PathBuf,
}

struct Ran {
    status: Option<i32>,
    stdout: String,
    /// stderr with escape codes, error-block gutters, and wrapping removed.
    stderr: String,
    /// stderr as written, one line per status line.
    raw_stderr: String,
}

impl Ran {
    /// The run's event markers, in the order they reached stderr.
    fn events(&self) -> Vec<&str> {
        self.raw_stderr
            .lines()
            .filter_map(|line| line.split("event:").nth(1))
            .map(|rest| rest.split_whitespace().next().unwrap_or_default())
            .collect()
    }

    /// Whether stderr names `text`, ignoring where the renderer wrapped it.
    fn names(&self, text: &str) -> bool {
        let squeeze = |text: &str| text.split_whitespace().collect::<String>();
        squeeze(&self.stderr).contains(&squeeze(text))
    }
}

impl Document {
    fn new(name: &str, document: &str, whitelist: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let program = fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"), &program)
            .expect("the recorder fixture binary is built with `test-fixtures`");
        fixture.write_file("cwd/doc.md", document);
        fixture.write_file("cwd/.darkmatter-shell-whitelist", whitelist);
        let records = fixture.tmp_dir().join("argv.bin");
        Self { fixture, records }
    }

    fn compose(&self) -> Ran {
        let output = self
            .fixture
            .command()
            .env(ARGV_RECORDER_FILE, &self.records)
            .arg("compose")
            .arg(self.fixture.cwd().join("doc.md"))
            .output()
            .expect("md runs");
        let raw_stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Ran {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: unwrapped(&raw_stderr),
            raw_stderr,
        }
    }

    /// Every argument vector the recorder received, in order; empty when it
    /// never ran.
    fn runs(&self) -> Vec<Vec<String>> {
        match read_argv_records(&self.records) {
            Ok(runs) => runs,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("unreadable recorder file: {error}"),
        }
    }
}

fn argv(words: &[&str]) -> Vec<Vec<String>> {
    vec![words.iter().map(|word| (*word).to_string()).collect()]
}

const CLEANUP: &str = "failed:\n  - info: \"event:failed\"\nfinalize:\n  - info: \"event:finalize\"\n";

// ── A `set` and an exact approval ────────────────────────────────────────

/// `initialize` mutations are input to the shell audit, so an exact approval
/// of the bytes the authored value gives no longer covers the command: it is
/// refused before anything runs, and `failed` then `finalize` follow.
#[test]
fn an_initialize_set_invalidates_an_exact_approval_of_the_authored_bytes() {
    let document = Document::new(
        "lifecycle-approval-initialize-set",
        &format!(
            "---\nx: plain\ninitialize:\n  - set: {{ x: changed }}\ncomposed:\n  - shell: \"argv-recorder {{{{ x }}}}\"\n\
             {CLEANUP}---\nbody\n"
        ),
        "exact argv-recorder plain\n",
    );

    let ran = document.compose();

    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert!(ran.names("Approval required for 'argv-recorder {{ x }}'"), "{}", ran.stderr);
    assert!(ran.names("exact argv-recorder changed"), "the audit saw the mutated bytes: {}", ran.stderr);
    assert_eq!(ran.events(), ["failed", "finalize"], "{}", ran.stderr);
    assert!(document.runs().is_empty(), "nothing ran");
    assert!(!ran.stdout.contains("body"), "{}", ran.stdout);
}

/// A `set` in `composed` cannot change an exactly approved command: it runs
/// the bytes the audit approved, and the value the `set` wrote never reaches
/// it. This is Claudine's rule for an exact approval.
#[test]
fn an_exact_approval_runs_its_approved_bytes_whatever_a_composed_set_wrote() {
    let document = Document::new(
        "lifecycle-approval-composed-set-exact",
        &format!(
            "---\nx: plain\ncomposed:\n  - set: {{ x: changed }}\n  - shell: \"argv-recorder {{{{ x }}}}\"\n{CLEANUP}---\nbody\n"
        ),
        "exact argv-recorder plain\n",
    );

    let ran = document.compose();

    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(document.runs(), argv(&["plain"]));
    assert_eq!(ran.events(), ["finalize"], "{}", ran.stderr);
}

/// Approving the bytes a later `set` would produce authorizes nothing: the
/// audit judges the command as the document stands before `composed`, so the
/// run is refused before any event can write the value.
#[test]
fn an_exact_approval_of_the_bytes_a_composed_set_would_produce_is_refused() {
    let document = Document::new(
        "lifecycle-approval-composed-set-refused",
        &format!(
            "---\nx: plain\ncomposed:\n  - set: {{ x: changed }}\n  - shell: \"argv-recorder {{{{ x }}}}\"\n{CLEANUP}---\nbody\n"
        ),
        "exact argv-recorder changed\n",
    );

    let ran = document.compose();

    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert!(ran.names("Approval required for 'argv-recorder {{ x }}'"), "{}", ran.stderr);
    assert_eq!(ran.events(), ["failed", "finalize"], "{}", ran.stderr);
    assert!(document.runs().is_empty(), "nothing ran");
}

// ── Typed templates ──────────────────────────────────────────────────────

/// A shape-approved command reads the value current when it runs, so a
/// `set` earlier in `composed` is what it receives, and the value is one
/// argument whatever shell syntax it holds.
#[test]
fn a_shape_approved_lifecycle_shell_fills_the_value_a_composed_set_wrote_as_one_argument() {
    let document = Document::new(
        "lifecycle-approval-template-fill",
        &format!(
            "---\nx: plain\ncomposed:\n  - set: {{ x: \"a b; c\" }}\n  - shell: \"argv-recorder {{{{ x }}}}\"\n{CLEANUP}---\nbody\n"
        ),
        "template argv-recorder {{ string }}\n",
    );

    let ran = document.compose();

    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(document.runs(), argv(&["a b; c"]));
}

/// A value outside the approved shape stops the event before anything
/// starts, even under `no_error`; `finalize` still runs, `failed` does not
/// (composition already succeeded), and the refusal never shows the value.
fn assert_refused_fill(name: &str, schema: &str, written: &str, reason: &str) {
    let document = Document::new(
        name,
        &format!(
            "---\n{schema}word: long-enough\ncomposed:\n  - set: {{ word: {written} }}\n\
             \x20 - shell: {{ command: \"argv-recorder {{{{ word }}}}\", no_error: true }}\n{CLEANUP}---\nbody\n"
        ),
        "template argv-recorder {{ string }}\n",
    );

    let ran = document.compose();

    assert_eq!(ran.status, Some(1), "{name}: {}", ran.stderr);
    assert!(ran.names("argv-recorder {{ word }}"), "{name}: {}", ran.stderr);
    assert!(ran.names(reason), "{name}: {}", ran.stderr);
    assert!(!ran.stderr.contains("SECRET"), "{name}: the value is never shown: {}", ran.stderr);
    assert_eq!(ran.events(), ["finalize"], "{name}: {}", ran.stderr);
    assert!(document.runs().is_empty(), "{name}: nothing may run");
}

macro_rules! refused_fills {
    ($( $name:ident: $schema:literal, $written:literal => $reason:literal ),+ $(,)?) => {
        $(#[test]
        fn $name() {
            assert_refused_fill(stringify!($name), $schema, $written, $reason);
        })+
    };
}

refused_fills! {
    a_set_array_is_refused_before_the_shape_approved_shell_runs: "", "[SECRET]" => "is an array or object",
    a_set_null_is_refused_before_the_shape_approved_shell_runs: "", "null" => "reads 'word', which is missing",
    // The document's own schema types the placeholder, as it does for a body
    // `::shell` and under Claudine.
    a_value_the_document_schema_rejects_is_refused_before_the_shell_runs:
        "$schema:\n  word: \"string(min(8))\"\n", "SECRET" => "does not satisfy its declared type",
}

// ── A guarded command (Claudine's `guarded-shell-*` traces) ──────────────

/// `when: note` guards a shape-approved command that reads `note`.
fn guarded(name: &str, note: &str, then: &str) -> Document {
    Document::new(
        name,
        &format!(
            "---\nnote: {note}\ncomposed:\n  - info: \"event:composed\"\n  - when: \"note\"\n    then:\n{then}\
             \x20     - shell: \"argv-recorder start {{{{ note }}}}\"\n{CLEANUP}---\nbody\n"
        ),
        "template argv-recorder start {{ string }}\n",
    )
}

/// `guarded-shell-present`: the value is present, so the command runs once
/// with it.
#[test]
fn a_guarded_lifecycle_shell_runs_with_its_value_when_present() {
    let document = guarded("lifecycle-guarded-present", "hello", "");

    let ran = document.compose();

    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert_eq!(document.runs(), argv(&["start", "hello"]));
    assert_eq!(ran.events(), ["composed", "finalize"], "{}", ran.stderr);
    assert!(ran.stdout.contains("body"), "{}", ran.stdout);
}

/// A null value the guard proves present only when selected passes the
/// audit, because of the guard; the guard is false, so nothing runs and the
/// run succeeds.
#[test]
fn a_guarded_lifecycle_shell_with_a_null_value_passes_preflight_and_never_runs() {
    let document = guarded("lifecycle-guarded-null", "null", "");

    let ran = document.compose();

    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(!ran.names("which is missing"), "{}", ran.stderr);
    assert_eq!(ran.events(), ["composed", "finalize"], "{}", ran.stderr);
    assert!(document.runs().is_empty(), "the unselected command never runs");
    assert!(ran.stdout.contains("body"), "{}", ran.stdout);
}

/// The control: without the guard, the same null value is refused at
/// preflight, so the guard is what admitted it above.
#[test]
fn an_unguarded_lifecycle_shell_with_a_null_value_is_refused_at_preflight() {
    let document = Document::new(
        "lifecycle-unguarded-null",
        &format!("---\nnote: null\ncomposed:\n  - shell: \"argv-recorder start {{{{ note }}}}\"\n{CLEANUP}---\nbody\n"),
        "template argv-recorder start {{ string }}\n",
    );

    let ran = document.compose();

    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert!(ran.names("reads 'note', which is missing"), "{}", ran.stderr);
    assert_eq!(ran.events(), ["failed", "finalize"], "{}", ran.stderr);
    assert!(document.runs().is_empty());
}

/// `guarded-shell-null`: the guard holds, then a `set` in the selected branch
/// clears the value, so the reached command fails its missing-value check
/// before it starts. Under `md` a `composed` error runs `finalize` without
/// `failed` and exits non-zero.
#[test]
fn a_guarded_value_a_set_cleared_fails_the_reached_shell_before_it_runs() {
    let document = guarded("lifecycle-guarded-cleared", "hello", "      - set: { note: null }\n");

    let ran = document.compose();

    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    assert!(ran.names("reads 'note', which is missing"), "{}", ran.stderr);
    assert_eq!(ran.events(), ["composed", "finalize"], "{}", ran.stderr);
    assert!(document.runs().is_empty(), "nothing launched");
    assert!(!ran.stdout.contains("body"), "{}", ran.stdout);
}
