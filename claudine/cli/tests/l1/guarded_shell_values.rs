//! Shell values a `when:` guard proves present, through `claudine compose`
//! and `claudine sequence`: pre-flight accepts a guarded command whose value
//! is missing now once its template is approved, a false guard launches
//! nothing, a selected command reads its live value as one argument, and the
//! same command outside any guard still fails pre-flight.
//!
//! Every command is the `claudine-argv-recorder` fixture copied into the
//! fixture `bin` as `argv-recorder`, so "what ran" is the argument vectors the
//! program itself recorded, on every platform. There is no terminal, so a
//! command no policy rule covers fails approval rather than prompting.

use crate::common::{CliProcessFixture, strip_ansi};
use std::path::PathBuf;
use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};

struct Run {
    fixture: CliProcessFixture,
    records: PathBuf,
}

struct Outcome {
    success: bool,
    /// stderr with ANSI, box gutters, and wrapping removed.
    stderr: String,
}

impl Run {
    /// `document` as `doc.md`, with `rules` as its shell whitelist.
    fn new(name: &str, document: &str, rules: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let suffix = std::env::consts::EXE_SUFFIX;
        std::fs::copy(
            biscuit_test_harness::bin_exe!("claudine-argv-recorder"),
            fixture.bin_dir().join(format!("argv-recorder{suffix}")),
        )
        .expect("the recorder fixture binary is built with `test-fixtures`");
        std::fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            fixture.bin_dir().join(format!("goose{suffix}")),
        )
        .expect("the fake provider is built with `test-fixtures`");
        std::fs::write(fixture.cwd().join("doc.md"), document).unwrap();
        std::fs::write(fixture.cwd().join(".darkmatter-shell-whitelist"), rules).unwrap();
        let records = fixture.cwd().join("argv.bin");
        Self { fixture, records }
    }

    /// `claudine <command> --goose doc.md <setters>`; the recorder is cleared
    /// first, so [`Self::runs`] holds this run's commands only.
    fn run(&self, command: &str, setters: &[&str]) -> Outcome {
        let _ = std::fs::remove_file(&self.records);
        let output = self
            .fixture
            .command()
            .env(ARGV_RECORDER_FILE, &self.records)
            .env("FAKE_DIR", self.fixture.cwd().join("provider-records"))
            .env("PROVIDER_MARKER", self.fixture.cwd().join("launches.txt"))
            .args([command, "--goose", "doc.md"])
            .args(setters)
            .output()
            .expect("claudine runs");
        assert!(!self.fixture.audio_spool().exists(), "shell acceptance must publish no audio");
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        Outcome {
            success: output.status.success(),
            stderr: stderr.replace(['┃', '│'], " ").split_whitespace().collect::<Vec<_>>().join(" "),
        }
    }

    /// Every argument vector the recorder received, in order.
    fn runs(&self) -> Vec<Vec<String>> {
        if !self.records.exists() {
            return Vec::new();
        }
        read_argv_records(&self.records).expect("the recorder writes whole records")
    }
}

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_string()).collect()
}

const COMMIT_RULE: &str = "template argv-recorder commit -m {{ string }}\n";

/// The motivating lifecycle guard: `commit_message` absent passes pre-flight
/// once the template is approved and the false branch launches nothing; set,
/// the selected branch passes it as one argument.
#[test]
fn a_lifecycle_guard_proves_its_commit_message_present() {
    let document = Run::new(
        "guarded-lifecycle-commit",
        r#"---
dirty: true
start:
    - when: "dirty && commit_message"
      then:
          - shell: argv-recorder commit -m "{{commit_message}}"
---
Body.
"#,
        COMMIT_RULE,
    );

    let skipped = document.run("compose", &[]);
    assert!(skipped.success, "{}", skipped.stderr);
    assert!(document.runs().is_empty(), "the false branch launches nothing");

    let selected = document.run("compose", &["commit_message=a b; c"]);
    assert!(selected.success, "{}", selected.stderr);
    assert_eq!(document.runs(), vec![argv(&["commit", "-m", "a b; c"])]);
}

#[test]
fn the_same_lifecycle_command_outside_a_guard_still_fails_pre_flight() {
    let document = Run::new(
        "unguarded-lifecycle-commit",
        "---\nstart:\n    - shell: argv-recorder commit -m \"{{commit_message}}\"\n---\nBody.\n",
        COMMIT_RULE,
    );

    let outcome = document.run("compose", &[]);

    assert!(!outcome.success);
    assert!(outcome.stderr.contains("commit_message") && outcome.stderr.contains("which is missing"), "{}", outcome.stderr);
    assert!(document.runs().is_empty());
}

/// A `set` that clears the guarded value before the selected command fails
/// that command with a missing value; the earlier command has run.
#[test]
fn a_value_a_set_clears_inside_the_branch_fails_the_reached_command() {
    let document = Run::new(
        "guarded-lifecycle-cleared",
        r#"---
msg: present
start:
    - when: msg
      then:
          - shell: argv-recorder first
          - set:
                msg: null
          - shell: argv-recorder second {{ msg }}
---
Body.
"#,
        "exact argv-recorder first\ntemplate argv-recorder second {{ string }}\n",
    );

    let outcome = document.run("compose", &[]);

    assert!(!outcome.success);
    assert!(outcome.stderr.contains("missing"), "{}", outcome.stderr);
    assert_eq!(document.runs(), vec![argv(&["first"])], "only the earlier action ran");
}

/// A sequence task's `when:` proves its `shell:` value present: missing, the
/// task is skipped after its template is approved; set, it runs.
#[test]
fn a_task_guard_proves_its_shell_value_present() {
    let sequence = Run::new(
        "guarded-sequence-task",
        "---\nsequence:\n    - name: commit\n      when: msg\n      shell: argv-recorder commit -m {{ msg }}\n---\nBody.\n",
        COMMIT_RULE,
    );

    let skipped = sequence.run("sequence", &[]);
    assert!(skipped.success, "{}", skipped.stderr);
    assert!(sequence.runs().is_empty());

    let selected = sequence.run("sequence", &["msg=a b"]);
    assert!(selected.success, "{}", selected.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["commit", "-m", "a b"])]);
}

const GROUP: &str = r#"---
sequence:
    - name: first
      shell: argv-recorder first
    - name: grouped
      group:
          name: bundle
          tasks:
              - name: reader
                GUARD
                shell: argv-recorder reader {{ seq.outputs[5] }}
              - name: plain
                shell: argv-recorder plain
---
Body.
"#;

const GROUP_RULES: &str =
    "exact argv-recorder first\nexact argv-recorder plain\ntemplate argv-recorder reader {{ string }}\n";

/// A group checks its members' late values before any member starts. A
/// member whose guard proves its late value present is not checked there,
/// so its false guard skips it; the same member without a guard still fails
/// the barrier before any member runs.
#[test]
fn a_false_task_guard_does_not_fail_the_late_value_barrier() {
    let guarded = Run::new("guarded-group-barrier", &GROUP.replace("GUARD", "when: \"seq.outputs[5]\""), GROUP_RULES);

    let outcome = guarded.run("sequence", &[]);

    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(guarded.runs(), vec![argv(&["first"]), argv(&["plain"])]);

    let unguarded = Run::new("unguarded-group-barrier", &GROUP.replace("                GUARD\n", ""), GROUP_RULES);

    let outcome = unguarded.run("sequence", &[]);

    assert!(!outcome.success);
    assert!(outcome.stderr.contains("outputs[5]") && outcome.stderr.contains("missing"), "{}", outcome.stderr);
    assert_eq!(unguarded.runs(), vec![argv(&["first"])], "no member ran");
}

/// A prompt document's guarded lifecycle command is approved with the
/// sequence and settled again, closed, when the document composes at its
/// turn: missing, it waits as a template and its false guard runs nothing;
/// passed through `with:`, it runs with that value as one argument.
#[test]
fn a_prompt_documents_guarded_command_settles_under_the_sequence_approval() {
    for (with, expected) in [("", Vec::<Vec<String>>::new()), ("      with:\n          msg: a b\n", vec![argv(&["guarded", "a b"])])] {
        let sequence = Run::new(
            "guarded-prompt-lifecycle",
            &format!("---\nsequence:\n    - name: review\n      prompt: prompt.md\n{with}---\nBody.\n"),
            "template argv-recorder guarded {{ string }}\n",
        );
        std::fs::write(
            sequence.fixture.cwd().join("prompt.md"),
            "---\nstart:\n    - when: msg\n      then:\n          - shell: argv-recorder guarded {{ msg }}\n---\nPrompt body.\n",
        )
        .unwrap();

        let outcome = sequence.run("sequence", &[]);

        assert!(outcome.success, "{with:?}: {}", outcome.stderr);
        assert_eq!(sequence.runs(), expected, "{with:?}");
    }
}

/// A `start` stack whose first action runs before `guarded`, so an empty
/// recording proves pre-flight refused the document before anything ran.
fn after_a_first_action(frontmatter: &str, guarded: &str) -> String {
    format!("---\n{frontmatter}start:\n    - shell: argv-recorder first\n{guarded}---\nBody.\n")
}

/// AC 9: a guard proves only what it reads, only inside its branch. `when: x`
/// leaves a missing `y` in the same command, and a read of `x` outside the
/// branch, failing pre-flight; neither document launches anything.
#[test]
fn a_guard_on_one_value_defers_neither_another_value_nor_an_unguarded_read() {
    for (row, frontmatter, guarded, placeholder) in [
        (
            "other value",
            "x: present\n",
            "    - when: x\n      then:\n          - shell: argv-recorder tool {{ x }} {{ y }}\n",
            "y",
        ),
        (
            "unguarded read",
            "",
            "    - when: x\n      then:\n          - shell: argv-recorder tool {{ x }} x\n    - shell: argv-recorder tool {{ x }} x\n",
            "x",
        ),
    ] {
        let document = Run::new(
            "guarded-scope-limits",
            &after_a_first_action(frontmatter, guarded),
            "exact argv-recorder first\ntemplate argv-recorder tool {{ string }} {{ string }}\ntemplate argv-recorder tool {{ string }} x\n",
        );

        let outcome = document.run("compose", &[]);

        assert!(!outcome.success, "{row}: {}", outcome.stderr);
        assert!(
            outcome.stderr.contains(&format!("reads `{placeholder}`, which is missing")),
            "{row}: {}",
            outcome.stderr
        );
        assert!(document.runs().is_empty(), "{row}: {:?}", document.runs());
    }
}

/// AC 9: a declaration that rules out `null` does not make an actual `null`
/// present. `string` leaves it to the missing-value rule; `string(required)`
/// is refused earlier by schema validation. Either way nothing runs.
#[test]
fn a_non_null_declaration_does_not_hide_an_actual_null() {
    for (declaration, refusal) in [
        ("string", "reads `msg`, which is missing"),
        ("string(required)", "null is not of type \"string\""),
    ] {
        let document = Run::new(
            "declared-null",
            &after_a_first_action(
                &format!("$schema:\n    msg: {declaration}\nmsg: null\n"),
                "    - shell: argv-recorder commit -m {{ msg }}\n",
            ),
            &format!("exact argv-recorder first\n{COMMIT_RULE}"),
        );

        let outcome = document.run("compose", &[]);

        assert!(!outcome.success, "{declaration}: {}", outcome.stderr);
        assert!(outcome.stderr.contains(refusal), "{declaration}: {}", outcome.stderr);
        assert!(document.runs().is_empty(), "{declaration}: {:?}", document.runs());
    }
}

/// AC 12: a write ends the guard's proof for the property it writes, through
/// either spelling, through an ancestor, and after an inner branch rejoins.
/// The value is absent now, so each command is a missing value at pre-flight;
/// a proof that outlived the write would let the command run with the written
/// value instead.
#[test]
fn a_write_through_an_alias_an_ancestor_or_an_inner_branch_ends_the_guards_proof() {
    for (row, guarded, placeholder) in [
        (
            "fm. write, bare read",
            "    - when: msg\n      then:\n          - set:\n                fm.msg: replaced\n          - shell: argv-recorder second {{ msg }}\n",
            "msg",
        ),
        (
            "bare write, fm. read",
            "    - when: fm.msg\n      then:\n          - set:\n                msg: replaced\n          - shell: argv-recorder second {{ fm.msg }}\n",
            "fm.msg",
        ),
        (
            "ancestor write",
            "    - when: obj.field\n      then:\n          - set:\n                obj:\n                    field: replaced\n          - shell: argv-recorder second {{ obj.field }}\n",
            "obj.field",
        ),
        (
            "inner branch write",
            "    - when: msg\n      then:\n          - when: other\n            then:\n                - set:\n                      msg: replaced\n          - shell: argv-recorder second {{ msg }}\n",
            "msg",
        ),
    ] {
        let document = Run::new(
            "guarded-write-ends-proof",
            &after_a_first_action("other: true\n", guarded),
            "exact argv-recorder first\ntemplate argv-recorder second {{ string }}\n",
        );

        let outcome = document.run("compose", &[]);

        assert!(!outcome.success, "{row}: {}", outcome.stderr);
        assert!(
            outcome.stderr.contains(&format!("reads `{placeholder}`, which is missing")),
            "{row}: {}",
            outcome.stderr
        );
        assert!(document.runs().is_empty(), "{row}: {:?}", document.runs());
    }
}

/// AC 3 / AC 12: a value present at approval that a `set` inside the selected
/// branch turns into a list is refused when its command is reached, before
/// any segment of that command's chain starts; the earlier action has run,
/// and the refusal never shows the value.
#[test]
fn a_value_a_set_turns_into_a_list_refuses_the_whole_reached_chain() {
    let document = Run::new(
        "guarded-lifecycle-retyped",
        r#"---
msg: approved
start:
    - when: msg
      then:
          - shell: argv-recorder first
          - set:
                msg: [SECRET-ITEM]
          - shell: argv-recorder chained && argv-recorder second {{ msg }}
---
Body.
"#,
        "exact argv-recorder first\ntemplate argv-recorder chained && argv-recorder second {{ string }}\n",
    );

    let outcome = document.run("compose", &[]);

    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("was refused before it ran"), "{}", outcome.stderr);
    assert!(!outcome.stderr.contains("SECRET-ITEM"), "{}", outcome.stderr);
    assert_eq!(document.runs(), vec![argv(&["first"])], "no segment of the refused chain ran");
}
