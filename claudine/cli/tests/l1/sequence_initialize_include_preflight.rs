//! Level-1 process coverage for the sequence static-preflight boundary of fix
//! `2026-09-15-initialize-after-proxy` (OQ1, Option A).
//!
//! Direct `compose`, `inline-compose`, and proxy adoption run a document's
//! `initialize` before its body is discovered. A sequence deliberately does
//! not: its static preflight reads every prompt document before the first step
//! runs, so an include that the document's own `initialize` would create is a
//! preflight failure. These rows pin that failure — typed, side-effect free,
//! and carrying a note that tells the author to create the file first — and
//! pin that a sequence whose include already exists runs unchanged.
//!
//! The failure rows need no provider: preflight aborts before a target is
//! resolved. The success rows use a `#!/bin/sh` fake provider and are
//! `unix`-only.

use crate::common;

use common::{CliProcessFixture, strip_ansi, write};
use std::fs;
use std::path::PathBuf;

const GENERATED: &str = "generated/notes.md";
const GENERATED_MARKER: &str = "GENERATED-BY-INITIALIZE";
const NOTE: &str = "note: a sequence checks every prompt document before its first step runs, \
     so a file that target.md includes must already exist. Neither an earlier step nor the \
     document's own initialize can create it. Create the file before starting the sequence.";

/// A prompt document whose `initialize` creates the file its body includes.
const TARGET: &str = "---\ntitle: target\ninitialize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'initialize']}\n    \
     - action: {ensure_file: 'generated/notes.md'}\n    \
     - action: {append_line: ['generated/notes.md', 'GENERATED-BY-INITIALIZE']}\n\
     ---\nTARGET-BODY\n\n::file generated/notes.md\n";

/// An earlier step that would create the include, then the prompt step.
///
/// The shell step writes a marker too, so a test can prove no step started.
const SEQUENCE: &str = "---\nsequence:\n    \
     - name: make\n      shell: \"mkdir -p generated && echo made > step-ran.txt\"\n    \
     - name: one\n      prompt: target.md\n---\nBody.\n";

struct Case {
    fixture: CliProcessFixture,
    sequence: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.initialize_repository();
        fixture.seed_user_config();
        write(&fixture.cwd().join("target.md"), TARGET);
        let sequence = fixture.cwd().join("seq.md");
        write(&sequence, SEQUENCE);
        Self { fixture, sequence }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.fixture.cwd().join(relative)
    }

    /// Run `claudine <args>`, returning success, flattened stderr, and the
    /// selected diagnostic's code when one was rendered.
    fn run(&self, args: &[&str]) -> (bool, String, Option<String>) {
        let snapshot = self.fixture.workspace_path().join("diagnostic.json");
        let _ = fs::remove_file(&snapshot);
        let output = self
            .fixture
            .command()
            .env("CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT", &snapshot)
            .args(args)
            .output()
            .unwrap();
        assert!(
            !self.fixture.audio_spool().exists(),
            "sequence preflight tests must not publish audio"
        );
        let code = fs::read_to_string(&snapshot).ok().map(|text| {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            value["code"].as_str().unwrap().to_string()
        });
        (
            output.status.success(),
            flatten(&strip_ansi(&String::from_utf8_lossy(&output.stderr))),
            code,
        )
    }

    fn run_sequence(&self, extra: &[&str]) -> (bool, String, Option<String>) {
        let mut args = vec!["sequence", self.sequence.to_str().unwrap()];
        args.extend_from_slice(extra);
        self.run(&args)
    }

    /// No step started and `initialize` never ran.
    fn assert_no_side_effects(&self, output: &str) {
        assert!(!self.path("events.log").exists(), "initialize must not run:\n{output}");
        assert!(!self.path("generated").exists(), "nothing may be created:\n{output}");
        assert!(!self.path("step-ran.txt").exists(), "no step may start:\n{output}");
        assert_eq!(fs::read_to_string(&self.sequence).unwrap(), SEQUENCE);
        assert_eq!(fs::read_to_string(self.path("target.md")).unwrap(), TARGET);
    }
}

/// Collapse block-quote prefixes and terminal-width wrapping into one line.
fn flatten(text: &str) -> String {
    text.replace('\u{2503}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn assert_missing_include_failure(success: bool, output: &str, code: Option<&str>) {
    assert!(!success, "a generated include must fail sequence preflight:\n{output}");
    let error = output
        .find("TransclusionError")
        .unwrap_or_else(|| panic!("the typed transclusion error renders:\n{output}"));
    assert!(output.contains(&format!("File not found: {GENERATED}")), "{output}");
    let note = output
        .find(NOTE)
        .unwrap_or_else(|| panic!("the preflight note renders:\n{output}"));
    assert!(note < error, "the note precedes the error block:\n{output}");
    assert_eq!(output.matches("note:").count(), 1, "{output}");
    assert_eq!(output.matches("File not found").count(), 1, "{output}");
    // The note adds guidance only: the selected diagnostic is the same one
    // direct compose reports for a missing include.
    assert_eq!(code, Some("composition.failed"), "{output}");
}

#[test]
fn a_prompt_whose_initialize_would_create_its_include_fails_sequence_preflight() {
    let case = Case::new("sequence-generated-include");

    let (success, output, code) = case.run_sequence(&["-y", "--claude"]);

    assert_missing_include_failure(success, &output, code.as_deref());
    case.assert_no_side_effects(&output);
}

#[test]
fn a_dry_run_reports_the_same_preflight_failure_without_side_effects() {
    let case = Case::new("sequence-generated-include-dry");

    let (success, output, code) = case.run_sequence(&["--dry-run", "--claude"]);

    assert_missing_include_failure(success, &output, code.as_deref());
    case.assert_no_side_effects(&output);
}

/// The note is scoped to sequence preflight: direct `compose` of a document
/// without `initialize` keeps its eager failure and gets no sequence note.
#[test]
fn a_direct_compose_missing_include_carries_no_sequence_note() {
    let case = Case::new("sequence-generated-include-direct");
    let document = case.path("plain.md");
    write(&document, "---\ntitle: plain\n---\nBODY\n\n::file generated/notes.md\n");

    let (success, output, code) =
        case.run(&["compose", document.to_str().unwrap(), "-y", "--claude"]);

    assert!(!success, "{output}");
    assert!(output.contains(&format!("File not found: {GENERATED}")), "{output}");
    assert!(!output.contains("note:"), "{output}");
    assert_eq!(code.as_deref(), Some("composition.failed"), "{output}");
}

#[cfg(unix)]
mod existing_include {
    use super::*;
    use common::{sh_quote, write_executable};

    /// A fake `claude` that records the prompt it received.
    fn install_provider(case: &Case) -> PathBuf {
        let prompt = case.fixture.workspace_path().join("prompt.txt");
        write_executable(
            &case.fixture.bin_dir().join("claude"),
            &format!(
                "#!/bin/sh\n{{ printf '%s\\n' \"$@\"; cat; }} >> {prompt}\nexit 0\n",
                prompt = sh_quote(&prompt.display().to_string()),
            ),
        );
        prompt
    }

    fn seed_include(case: &Case) {
        write(&case.path(GENERATED), "PRE-EXISTING\n");
    }

    #[test]
    fn a_sequence_whose_include_exists_runs_unchanged() {
        let case = Case::new("sequence-existing-include");
        let prompt = install_provider(&case);
        seed_include(&case);

        let (success, output, _) = case.run_sequence(&["-y", "--claude"]);

        assert!(success, "{output}");
        assert!(!output.contains("note:"), "{output}");
        assert!(case.path("step-ran.txt").is_file(), "{output}");
        assert_eq!(
            fs::read_to_string(case.path("events.log")).unwrap(),
            "initialize\n",
            "the step's initialize runs once, at its turn:\n{output}"
        );
        let prompt = fs::read_to_string(prompt).unwrap();
        assert!(prompt.contains("TARGET-BODY"), "{prompt}");
        assert!(prompt.contains("PRE-EXISTING"), "{prompt}");
        // Option A keeps the sequence contract: the step's prompt is composed
        // before its `initialize` runs, so what `initialize` appends is not in
        // this prompt. Supporting that belongs to a separate feature (OQ1).
        assert!(!prompt.contains(GENERATED_MARKER), "{prompt}");
    }

    #[test]
    fn a_dry_run_whose_include_exists_still_succeeds_without_side_effects() {
        let case = Case::new("sequence-existing-include-dry");
        let prompt = install_provider(&case);
        seed_include(&case);

        let (success, output, _) = case.run_sequence(&["--dry-run", "--yolo", "--claude"]);

        assert!(success, "{output}");
        assert!(!output.contains("note:"), "{output}");
        assert!(!case.path("events.log").exists(), "initialize must not run:\n{output}");
        // A dry run may probe the installed binary, but never delivers a prompt.
        let delivered = fs::read_to_string(prompt).unwrap_or_default();
        assert!(!delivered.contains("TARGET-BODY"), "{delivered}\n{output}");
        assert_eq!(fs::read_to_string(case.path(GENERATED)).unwrap(), "PRE-EXISTING\n");
    }
}
