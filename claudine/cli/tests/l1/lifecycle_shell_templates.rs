//! Shell commands approved by their typed shape in a single document, through
//! `claudine compose`: a body `::shell` across loop iterations, and lifecycle
//! `shell` actions and `set` values.
//!
//! Every command is the `claudine-argv-recorder` fixture copied into the
//! fixture `bin` as `argv-recorder`, so "what ran" is the argument vectors the
//! program itself recorded, on every platform. There is no terminal, so a
//! command no policy rule covers fails approval rather than prompting.

use crate::common::{CliProcessFixture, strip_ansi};
use std::path::PathBuf;
use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};

struct Document {
    fixture: CliProcessFixture,
    records: PathBuf,
}

struct Outcome {
    success: bool,
    stdout: String,
    /// stderr with ANSI, box gutters, and wrapping removed.
    stderr: String,
}

impl Outcome {
    /// Whether stderr names `text`, ignoring whitespace on both sides: the
    /// renderer may wrap a command at a hyphen as well as at a space.
    fn names(&self, text: &str) -> bool {
        let squeeze = |text: &str| text.split_whitespace().collect::<String>();
        squeeze(&self.stderr).contains(&squeeze(text))
    }
}

impl Document {
    fn new(document: &str) -> Self {
        let fixture = CliProcessFixture::named("lifecycle-shell-templates");
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
        let records = fixture.cwd().join("argv.bin");
        Self { fixture, records }
    }

    /// The document's shell policy: its directory is the policy root.
    fn whitelist(&self, rules: &str) -> &Self {
        std::fs::write(self.fixture.cwd().join(".darkmatter-shell-whitelist"), rules).unwrap();
        self
    }

    fn blacklist(&self, rules: &str) -> &Self {
        std::fs::write(self.fixture.cwd().join(".darkmatter-shell-blacklist"), rules).unwrap();
        self
    }

    fn run(&self, setters: &[&str]) -> Outcome {
        let mut args = vec!["compose", "--goose", "doc.md"];
        args.extend_from_slice(setters);
        self.run_args(&args, None)
    }

    fn run_args(&self, args: &[&str], provider_plan: Option<&str>) -> Outcome {
        self.run_env(args, provider_plan, &[])
    }

    fn run_env(&self, args: &[&str], provider_plan: Option<&str>, environment: &[(&str, &str)]) -> Outcome {
        let mut command = self.fixture.command();
        command.env(ARGV_RECORDER_FILE, &self.records).envs(environment.iter().copied());
        if let Some(plan) = provider_plan {
            command.env("FAKE_DIR", self.fixture.cwd().join("provider"));
            command.env("FAKE_PLAN", plan);
        } else {
            command.env("PROVIDER_MARKER", self.fixture.cwd().join("launches.txt"));
        }
        let output = command.args(args).output().expect("claudine runs");
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        Outcome {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
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

const LOOP_DOCUMENT: &str = r#"---
counter: 0
loop:
    until: "loop.count >= 3"
    action: increment(counter)
---
::shell argv-recorder run-{{ loop.count }}

Counter is {{ counter }}
"#;

/// A body command approved by its shape runs on every iteration of a
/// single-document loop: each iteration's bytes are new, and the shape covers
/// them without asking. Each value arrives as one argument.
#[test]
fn a_shape_approval_covers_every_loop_iteration() {
    let document = Document::new(LOOP_DOCUMENT);
    document.whitelist("template argv-recorder run-{{ string }}\n");

    let outcome = document.run(&[]);

    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(document.runs(), vec![argv(&["run-1"]), argv(&["run-2"]), argv(&["run-3"])]);
}

/// An exact approval of the first iteration's bytes does not cover the next
/// iteration's: after the first launch the prompt is frozen, so iteration 2
/// is refused naming its command, and nothing of it runs.
#[test]
fn an_exact_approval_of_the_first_iteration_refuses_the_second() {
    let document = Document::new(LOOP_DOCUMENT);
    document.whitelist("exact argv-recorder run-1\n");

    let outcome = document.run(&[]);

    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.names("argv-recorder run-2"), "{}", outcome.stderr);
    assert_eq!(document.runs(), vec![argv(&["run-1"])]);
}

/// A lifecycle `shell` action approved by its shape passes each value as
/// exactly one argument: whitespace, `;`, `$( … )`, and backticks are data,
/// never shell syntax.
#[test]
fn a_lifecycle_shell_template_passes_each_value_as_one_argument() {
    let document = Document::new(
        r#"---
spaced: "a b"
chained: "x; rm -rf ~"
ticked: "`id`"
substituted: "x$(id)"
start:
    - shell: "argv-recorder {{ spaced }} {{ chained }} {{ ticked }} {{ substituted }}"
---
Body.
"#,
    );
    document.whitelist("template argv-recorder {{ string }} {{ string }} {{ string }} {{ string }}\n");

    let outcome = document.run(&[]);

    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(document.runs(), vec![argv(&["a b", "x; rm -rf ~", "`id`", "x$(id)"])]);
}

/// A template-mode `shell` reads the state current when it runs, so a `set`
/// earlier in the same event is visible; a `set` value's own command reads
/// the snapshot taken before that `set` writes anything.
#[test]
fn a_shell_reads_the_current_state_and_a_set_value_its_pre_write_snapshot() {
    let document = Document::new(
        r#"---
word: before
start:
    - set:
          word: after
          seen: "$( argv-recorder set {{ word }} )"
    - shell: "argv-recorder shell {{ word }}"
---
Body.
"#,
    );
    document.whitelist("template argv-recorder set {{ string }}\ntemplate argv-recorder shell {{ string }}\n");

    let outcome = document.run(&[]);

    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(document.runs(), vec![argv(&["set", "before"]), argv(&["shell", "after"])]);
}

/// A value its declaration refuses, and a value an earlier `set` made
/// missing, stop the event before anything starts, even under `no_error`;
/// the error names the placeholder and never the value.
fn assert_refused_value(write: &str, reason: &str) {
        let document = Document::new(&format!(
            r#"---
$schema:
    word: "string(min(3))"
word: long-enough
start:
    - set:
          word: {write}
    - shell:
          command: "argv-recorder {{{{ word }}}}"
          no_error: true
---
Body.
"#
        ));
        document.whitelist("template argv-recorder {{ string }}\n");

        let outcome = document.run(&[]);

        assert!(!outcome.success, "{reason}: {}", outcome.stderr);
        assert!(outcome.names("argv-recorder {{ word }}"), "{reason}: {}", outcome.stderr);
        assert!(outcome.stderr.contains("refused before it ran"), "{reason}: {}", outcome.stderr);
        assert!(!outcome.stderr.contains("Q7"), "{reason}: the value is never shown: {}", outcome.stderr);
        assert!(document.runs().is_empty(), "{reason}: nothing may run");
}

macro_rules! refused_values {
    ($( $name:ident: $write:literal => $reason:literal ),+ $(,)?) => {
        $(#[test]
        fn $name() {
            assert_refused_value($write, $reason);
        })+
    };
}

refused_values! {
    a_rejected_value_fails_the_event_whatever_no_error_says: "Q7" => "rejected",
    a_missing_value_fails_the_event_whatever_no_error_says: "null" => "missing",
}

const LIFECYCLE_LOOP_DOCUMENT: &str = r#"---
counter: 0
loop:
    until: "loop.count >= 3"
    action: increment(counter)
start:
    - shell: "argv-recorder start-{{ loop.count }}"
---
Counter is {{ counter }}
"#;

/// A lifecycle command approved by its shape runs on every iteration; one
/// approved as the first iteration's exact bytes is refused at the second,
/// because an exact approval never covers a template and the reverse.
#[test]
fn a_lifecycle_shape_approval_covers_three_iterations() {
    let shaped = Document::new(LIFECYCLE_LOOP_DOCUMENT);
    shaped.whitelist("template argv-recorder start-{{ string }}\n");
    let outcome = shaped.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(shaped.runs(), vec![argv(&["start-1"]), argv(&["start-2"]), argv(&["start-3"])]);
}

#[test]
fn a_lifecycle_exact_approval_refuses_iteration_two() {
    let exact = Document::new(LIFECYCLE_LOOP_DOCUMENT);
    exact.whitelist("exact argv-recorder start-1\n");
    let outcome = exact.run(&[]);
    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.names("argv-recorder start-2"), "{}", outcome.stderr);
    assert_eq!(exact.runs(), vec![argv(&["start-1"])]);
}

/// The filled command meets both blacklists after its approval is found:
/// pre-flight approved the bytes the document started with, then a `set`
/// changed the value, and the `template` rule cannot authorize the new fill
/// the built-in blacklist refuses, nor one a user `exact` rule names.
#[test]
fn the_builtin_blacklist_judges_the_filled_command_after_approval() {
    let builtin = Document::new(
        "---\nsub: status\nstart:\n    - set: { sub: reset }\n    - shell: { command: \"git {{ sub }}\", no_error: true }\n---\nBody.\n",
    );
    builtin.whitelist("template git {{ string }}\n");
    let outcome = builtin.run(&[]);
    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.names("git {{ sub }}") && outcome.stderr.contains("refused before it ran"), "{}", outcome.stderr);
}

#[test]
fn the_user_blacklist_judges_the_filled_command_after_approval() {
    let user = Document::new(
        "---\nword: allowed\nstart:\n    - set: { word: forbidden }\n    - shell: { command: \"argv-recorder {{ word }}\", no_error: true }\n---\nBody.\n",
    );
    user.whitelist("template argv-recorder {{ string }}\n");
    user.blacklist("exact argv-recorder forbidden\n");
    let outcome = user.run(&[]);
    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("user blacklist"), "{}", outcome.stderr);
    assert!(user.runs().is_empty(), "a blacklisted fill never starts");
}

/// A program an interpreter would re-parse the arguments for is refused
/// before it starts, never run through a shell: a script without `#!` on
/// Unix, a batch file on Windows.
#[test]
fn an_interpreter_target_is_refused_before_it_starts() {
    let document = Document::new(
        "---\nword: value\nstart:\n    - shell: { command: \"interpreted {{ word }}\", no_error: true }\n---\nBody.\n",
    );
    let marker = document.fixture.cwd().join("interpreted-ran");
    #[cfg(unix)]
    crate::common::write_executable(
        &document.fixture.bin_dir().join("interpreted"),
        &format!("touch '{}'\n", marker.display()),
    );
    #[cfg(windows)]
    std::fs::write(
        document.fixture.bin_dir().join("interpreted.cmd"),
        format!("@echo off\r\ntype nul > \"{}\"\r\n", marker.display()),
    )
    .unwrap();
    document.whitelist("template interpreted {{ string }}\n");

    let outcome = document.run(&[]);

    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("refused before it ran"), "{}", outcome.stderr);
    assert!(!marker.exists(), "the interpreter target never ran");
}

#[test]
fn a_retry_reaudits_the_shape_and_reads_its_one_attempt_overlay() {
    let document = Document::new(
        r#"---
word: before
start:
    - shell: "argv-recorder {{ word }}"
failure:
    - retry:
          max_attempts: 1
          with:
              word: after
---
Body.
"#,
    );
    document.whitelist("template argv-recorder {{ string }}\n");

    let outcome = document.run_args(&["compose", "--goose", "doc.md"], Some("fail,ok"));

    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(document.runs(), vec![argv(&["before"]), argv(&["after"])]);
    assert!(document.fixture.cwd().join("provider/launch-2.txt").exists());
    assert!(!document.fixture.cwd().join("provider/launch-3.txt").exists());
}

#[test]
fn an_inline_proxy_target_audits_its_shape_and_reads_the_overlay() {
    let document = Document::new(
        "---\nprompt: Router\nstart:\n    - proxy: { target: './target.md', with: { word: 'a b; c' } }\n---\nRouter.\n",
    );
    std::fs::write(
        document.fixture.cwd().join("target.md"),
        "---\nprompt: Target\nword: default\nstart:\n    - shell: \"argv-recorder {{ word }}\"\n---\nTarget.\n",
    )
    .unwrap();
    document.whitelist("template argv-recorder {{ string }}\n");

    let target = document.fixture.cwd().join("target.md");
    let output = document.fixture.command()
        .env(ARGV_RECORDER_FILE, &document.records)
        .env("PROVIDER_MARKER", document.fixture.cwd().join("launches.txt"))
        .env("FAKE_INLINE_DOCUMENT", &target)
        .args(["inline-compose", "--goose", "doc.md"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(document.runs(), vec![argv(&["a b; c"])]);
    assert!(std::fs::read_to_string(target).unwrap().contains("Agent-written body."));
}

#[test]
fn a_persisted_broad_shape_keeps_the_current_owner_constraints() {
    let source = |minimum| format!(
        "---\n$schema:\n    word: 'string(min({minimum}))'\nword: long-enough\nstart:\n    - set: {{ word: four }}\n    - shell: {{ command: \"argv-recorder {{{{ word }}}}\", no_error: true }}\n---\nBody.\n"
    );
    let document = Document::new(&source(3));
    document.whitelist("template argv-recorder {{ string }}\n");
    let first = document.run(&[]);
    assert!(first.success, "{}", first.stderr);
    assert_eq!(document.runs(), vec![argv(&["four"])]);

    std::fs::write(document.fixture.cwd().join("doc.md"), source(5)).unwrap();
    let second = document.run(&[]);
    assert!(!second.success, "{}", second.stderr);
    assert!(second.stderr.contains("refused before it ran"), "{}", second.stderr);
    assert_eq!(document.runs(), vec![argv(&["four"])], "the changed constraint blocks a second spawn");
    assert_eq!(
        std::fs::read_to_string(document.fixture.cwd().join(".darkmatter-shell-whitelist")).unwrap(),
        "template argv-recorder {{ string }}\n"
    );
}

#[test]
fn a_resume_reaudits_the_shape_after_the_document_changes() {
    let document = Document::new(
        r#"---
word: before
start:
    - shell: "argv-recorder {{ word }}"
success:
    - when: "word == 'before'"
      then:
          - set_frontmatter: ["doc.md", "word", "after"]
          - resume: continue
---
Body.
"#,
    );
    std::fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-codex"),
        document.fixture.bin_dir().join(format!("codex{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    document.whitelist("template argv-recorder {{ string }}\n");
    let provider = document.fixture.cwd().join("codex");

    let output = document.fixture.command()
        .env(ARGV_RECORDER_FILE, &document.records)
        .env("FAKE_CODEX_DIR", &provider)
        .args(["compose", "--codex", "doc.md"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(document.runs(), vec![argv(&["before"]), argv(&["after"])]);
    let resumed: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(provider.join("argv-2.json")).unwrap(),
    )
    .unwrap();
    assert!(resumed.iter().any(|arg| arg == "resume"), "{resumed:?}");
    assert!(!provider.join("argv-3.json").exists());
}


macro_rules! native_lifecycle_redirect_case {
    ($name:ident, $redirect:literal, $out:literal, $err:literal) => {
        #[test]
        fn $name() {
            let command = format!("argv-recorder {{{{ word }}}} {}", $redirect);
            let document = Document::new(&format!("---\nword: a b\nstart:\n  - shell: {}\n---\nPrompt.\n", serde_json::to_string(&command).unwrap()));
            document.whitelist(&format!("template argv-recorder {{{{ string }}}} {}\n", $redirect));
            let outcome = document.run_env(&["compose", "--goose", "doc.md"], None,
                &[("ARGV_RECORDER_STDOUT", "stdout-probe"), ("ARGV_RECORDER_STDERR", "stderr-probe")]);
            assert!(outcome.success, "{}", outcome.stderr);
            assert_eq!(outcome.stdout.contains("stderr-probe"), $out, "{} / {}", outcome.stdout, outcome.stderr);
            assert_eq!(outcome.stderr.contains("stdout-probe"), $err, "{} / {}", outcome.stdout, outcome.stderr);
            assert!(!outcome.stdout.contains("stdout-probe"));
            assert!(!outcome.stderr.contains("stderr-probe"));
            assert_eq!(document.runs(), vec![argv(&["a b"])]);
            assert!(!document.fixture.audio_spool().exists());
        }
    };
}

native_lifecycle_redirect_case!(native_lifecycle_merge_then_null_keeps_the_original_stdout_destination, "2>&1 > /dev/null", true, false);
native_lifecycle_redirect_case!(native_lifecycle_route_then_null_keeps_the_original_stderr_destination, ">&2 2> /dev/null", false, true);
