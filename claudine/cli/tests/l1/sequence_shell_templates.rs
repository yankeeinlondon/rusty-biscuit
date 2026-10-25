//! Sequence shell approval by task position, through `claudine sequence`.
//!
//! The first task's commands may use resolved-byte or shape approval; every later
//! task's commands with a placeholder, and every command reading a late value
//! (`seq`, `loop`, `group`), are approved only by their typed shape. An
//! approval-time value that is missing fails preflight before step 1; a late
//! value that is missing fails its own step before any of that step's commands
//! runs.
//!
//! Every command is the `claudine-argv-recorder` fixture copied into the
//! fixture `bin` as `argv-recorder`, so "what ran" is the argument vectors the
//! program itself recorded, on every platform. There is no terminal, so a
//! command no policy rule covers fails approval rather than prompting.

use crate::common::{CliProcessFixture, strip_ansi};
use std::path::PathBuf;
use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};

struct Sequence {
    fixture: CliProcessFixture,
    records: PathBuf,
}

struct Outcome {
    success: bool,
    /// stderr with ANSI, box gutters, and wrapping removed.
    stderr: String,
}

impl Outcome {
    /// Whether stderr names `text`. Whitespace is ignored on both sides: the
    /// renderer may wrap a command at a hyphen as well as at a space.
    fn names(&self, text: &str) -> bool {
        let squeeze = |text: &str| text.split_whitespace().collect::<String>();
        squeeze(&self.stderr).contains(&squeeze(text))
    }
}

impl Sequence {
    fn new(name: &str, document: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let suffix = std::env::consts::EXE_SUFFIX;
        std::fs::copy(
            biscuit_test_harness::bin_exe!("claudine-argv-recorder"),
            fixture.bin_dir().join(format!("argv-recorder{suffix}")),
        )
        .expect("the recorder fixture binary is built with `test-fixtures`");
        // Native fake provider also serves prompt-document and body steps.
        std::fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            fixture.bin_dir().join(format!("goose{suffix}")),
        )
        .expect("the fake provider is built with `test-fixtures`");
        std::fs::write(fixture.cwd().join("seq.md"), document).unwrap();
        let records = fixture.cwd().join("argv.bin");
        Self { fixture, records }
    }

    /// The sequence's shell policy: its directory is the policy root.
    fn whitelist(&self, rules: &str) -> &Self {
        std::fs::write(self.fixture.cwd().join(".darkmatter-shell-whitelist"), rules).unwrap();
        self
    }

    fn run(&self, setters: &[&str]) -> Outcome {
        self.run_env(setters, &[])
    }

    fn run_env(&self, setters: &[&str], environment: &[(&str, &str)]) -> Outcome {
        let output = self
            .fixture
            .command()
            .env(ARGV_RECORDER_FILE, &self.records)
            .envs(environment.iter().copied())
            .env("FAKE_DIR", self.fixture.cwd().join("provider-records"))
            .args(["sequence", "--goose", "seq.md"])
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

const BUILD_CLEAN: &str = r#"---
$schema:
    out: string -> the build directory
sequence:
    - name: build
      shell: "argv-recorder build --out {{ out }}"
    - name: clean
      shell: "argv-recorder clean --out {{ out }}/build"
---
Body.
"#;

/// An `exact` rule for the later task's current bytes does not cover it, so
/// with no terminal preflight fails naming its authored command before step 1;
/// a `template` rule for its shape approves it without asking.
#[test]
fn a_later_task_is_approved_only_by_its_shape() {
    let sequence = Sequence::new("sequence-shell-templates", BUILD_CLEAN);
    sequence.whitelist("exact argv-recorder build --out dist\nexact argv-recorder clean --out dist/build\n");
    let refused = sequence.run(&["out=dist"]);
    assert!(!refused.success, "{}", refused.stderr);
    assert!(
        refused.names("argv-recorder clean --out {{ out }}/build")
            && refused.stderr.contains("approval"),
        "{}",
        refused.stderr
    );
    assert!(sequence.runs().is_empty(), "nothing may run before preflight passes");

    sequence.whitelist(
        "exact argv-recorder build --out dist\ntemplate argv-recorder clean --out {{ string }}/build\n",
    );
    let approved = sequence.run(&["out=dist"]);
    assert!(approved.success, "{}", approved.stderr);
    assert_eq!(
        sequence.runs(),
        vec![argv(&["build", "--out", "dist"]), argv(&["clean", "--out", "dist/build"])]
    );
}

#[test]
fn a_nested_container_terminal_template_uses_the_current_value_as_one_argument() {
    let sequence = Sequence::new(
        "nested-container-shell-template",
        r#"---
target: initial
$schema:
    target: string
sequence:
    - name: first
      shell: argv-recorder first
    - name: change
      side_effect: { set: { target: 'two words; $(id) `id`' } }
    - name: nested
      sequence:
        - { name: child, shell: argv-recorder child }
      success:
        - shell: 'argv-recorder terminal {{ target }}'
---
Body.
"#,
    );
    sequence.whitelist(
        "exact argv-recorder first\nexact argv-recorder child\nexact argv-recorder terminal initial\n",
    );
    let refused = sequence.run(&[]);
    assert!(!refused.success, "{}", refused.stderr);
    assert!(refused.stderr.contains("approval"), "{}", refused.stderr);
    assert!(refused.names("argv-recorder terminal {{ target }}"), "{}", refused.stderr);
    assert!(sequence.runs().is_empty(), "container approval must precede the first child effect");

    sequence.whitelist(
        "exact argv-recorder first\nexact argv-recorder child\ntemplate argv-recorder terminal {{ string }}\n",
    );
    let approved = sequence.run(&[]);
    assert!(approved.success, "{}", approved.stderr);
    assert_eq!(
        sequence.runs(),
        vec![argv(&["first"]), argv(&["child"]), argv(&["terminal", "two words; $(id) `id`"])]
    );
}

/// A command reading `last(seq.outputs)` has no bytes at preflight. Without a
/// rule it fails approval before step 1, naming the command; with a template
/// rule, preflight approves it without asking and step 1 runs.
#[test]
fn a_late_value_is_approved_as_a_template_before_step_one() {
    let sequence = Sequence::new(
        "sequence-shell-templates",
        "---\nsequence:\n    - name: first\n      shell: \"argv-recorder first\"\n    - name: second\n      shell: \"argv-recorder {{ last(seq.outputs) }}\"\n---\nBody.\n",
    );
    sequence.whitelist("exact argv-recorder first\n");
    let refused = sequence.run(&[]);
    assert!(!refused.success);
    assert!(
        refused.names("argv-recorder {{ last(seq.outputs) }}") && refused.stderr.contains("approval"),
        "{}",
        refused.stderr
    );
    assert!(sequence.runs().is_empty());

    sequence.whitelist("exact argv-recorder first\ntemplate argv-recorder {{ string }}\n");
    let approved = sequence.run(&[]);
    assert!(approved.success, "{}", approved.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["first"]), argv(&[""])]);
}

/// A caller value a later task reads is an approval-time value: absent, it
/// fails preflight before step 1, naming it; present, the task is approved.
#[test]
fn an_absent_caller_value_fails_preflight_before_step_one() {
    let sequence = Sequence::new(
        "sequence-shell-templates",
        "---\nsequence:\n    - name: first\n      shell: \"argv-recorder first\"\n    - name: second\n      shell: \"argv-recorder {{ target }}\"\n---\nBody.\n",
    );
    sequence.whitelist("prefix argv-recorder\n");
    let refused = sequence.run(&[]);
    assert!(!refused.success);
    assert!(
        refused.stderr.contains("target") && refused.stderr.contains("missing"),
        "{}",
        refused.stderr
    );
    assert!(sequence.runs().is_empty(), "nothing may run before preflight passes");

    let approved = sequence.run(&["target=x"]);
    assert!(approved.success, "{}", approved.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["first"]), argv(&["x"])]);
}

/// `err`, `timing`, and `current` are not late values: a command reading one
/// is still refused at preflight, even beside an allowed late root.
#[test]
fn forbidden_roots_are_still_refused() {
    for (read, root) in [
        ("{{ err.msg }}", "err"),
        ("{{ timing.total }}", "timing"),
        ("{{ last(seq.outputs) }} {{ current.branch }}", "current"),
    ] {
        let sequence = Sequence::new(
            "sequence-shell-templates",
            &format!(
                "---\nsequence:\n    - name: first\n      shell: \"argv-recorder first\"\n    - name: second\n      shell: \"argv-recorder {read}\"\n---\nBody.\n"
            ),
        );
        sequence.whitelist("prefix argv-recorder\n");
        let refused = sequence.run(&[]);
        assert!(!refused.success);
        assert!(refused.names(&format!("references `{root}`")), "{root}: {}", refused.stderr);
        assert!(sequence.runs().is_empty());
    }
}

/// A late value that is `null` when its step is reached fails that step
/// before its command runs while the
/// earlier step's command has run.
#[test]
fn a_missing_late_value_fails_its_step_before_its_commands_run() {
    let sequence = Sequence::new(
        "sequence-shell-templates",
        "---\nsequence:\n    - name: first\n      shell: \"argv-recorder first\"\n    - name: second\n      shell: \"argv-recorder {{ seq.outputs[3] }}\"\n---\nBody.\n",
    );
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(!outcome.success);
    assert!(
        outcome.names("outputs[3]") && outcome.stderr.contains("missing"),
        "{}",
        outcome.stderr
    );
    assert_eq!(sequence.runs(), vec![argv(&["first"])], "only the earlier step ran");
}

/// A prompt lifecycle's whole-value `set` `$( … )` is approved at preflight, so it
/// runs at its turn instead of being refused as never resolved.
#[test]
fn a_prompt_lifecycle_set_value_is_approved_and_runs() {
    let sequence = Sequence::new(
        "sequence-shell-templates",
        "---\nsequence:\n    - name: first\n      prompt: child.md\n---\nBody.\n",
    );
    std::fs::write(sequence.fixture.cwd().join("child.md"), "---\nstart:\n  - set:\n      captured: \"$(argv-recorder from-set)\"\n  - shell: argv-recorder primary\n---\nPrompt.\n").unwrap();
    sequence.whitelist("exact argv-recorder from-set\nexact argv-recorder primary\n");
    let outcome = sequence.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["from-set"]), argv(&["primary"])]);
}

#[test]
fn a_prompt_documents_missing_late_value_fails_before_prompt_launch() {
    let sequence = Sequence::new(
        "sequence-document-barrier",
        "---\nsequence:\n    - name: first\n      shell: argv-recorder first\n    - name: prompt\n      prompt: child.md\n---\nBody.\n",
    );
    std::fs::write(sequence.fixture.cwd().join("child.md"), "::shell argv-recorder {{ seq.outputs[3] }}\n").unwrap();
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.names("outputs[3]") && outcome.stderr.contains("missing"), "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["first"])]);
}

#[test]
fn a_prompt_documents_late_parameter_is_filled_as_one_argument() {
    let sequence = Sequence::new(
        "sequence-document-late-parameter",
        "---\nsequence:\n    - name: first\n      shell: argv-recorder first\n    - name: prompt\n      with:\n          prior_output: '{{ last(seq.outputs) }}'\n      prompt: child.md\n---\nBody.\n",
    );
    std::fs::write(sequence.fixture.cwd().join("child.md"), "::shell argv-recorder '{{ prior_output }}suffix with spaces'\n\nPrompt.\n").unwrap();
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["first"]), argv(&["suffix with spaces"])]);
}

#[test]
fn a_prompt_documents_invalid_late_value_fails_before_prompt_launch() {
    let sequence = Sequence::new(
        "sequence-document-invalid-late",
        "---\nsequence:\n    - name: first\n      shell: argv-recorder first\n    - name: prompt\n      prompt: child.md\n---\nBody.\n",
    );
    std::fs::write(sequence.fixture.cwd().join("child.md"), "::shell argv-recorder {{ seq.outputs }}\n").unwrap();
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("array or object"), "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["first"])]);
}

#[test]
fn body_steps_fill_their_current_value_without_preflight_spawns() {
    let sequence = Sequence::new(
        "sequence-body-current-value",
        "---\nsequence:\n    - name: first\n      value: one\n    - name: second\n      value: two words\n---\n::shell argv-recorder '{{ seq.value }}'\n\nPrompt.\n",
    );
    sequence.whitelist("exact argv-recorder one\ntemplate argv-recorder {{ string }}\n");
    let outcome = sequence.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["one"]), argv(&["two words"])]);
}

macro_rules! primary_argument_case {
    ($name:ident, $expression:literal, $expected:literal) => {
        #[test]
        fn $name() {
            let command = format!("argv-recorder {{{{ {} }}}}", $expression);
            let document = format!("---\nsequence:\n    - name: first\n      shell: argv-recorder first\n    - name: second\n      shell: {}\n---\nBody.\n", serde_json::to_string(&command).unwrap());
            let sequence = Sequence::new("sequence-primary-argv", &document);
            sequence.whitelist("exact argv-recorder first\ntemplate argv-recorder {{ string }}\n");
            let outcome = sequence.run(&[]);
            assert!(outcome.success, "{}", outcome.stderr);
            assert_eq!(sequence.runs(), vec![argv(&["first"]), argv(&[$expected])]);
        }
    };
}

primary_argument_case!(a_template_primary_keeps_whitespace_in_one_argument, "'a b'", "a b");
primary_argument_case!(a_template_primary_never_executes_substitution_bytes, "'$(id)'", "$(id)");
primary_argument_case!(a_template_primary_keeps_quotes_as_data, "'a\"b'", "a\"b");
primary_argument_case!(a_template_primary_keeps_newlines_in_one_argument, "'a\\nb'", "a\nb");
primary_argument_case!(a_template_primary_keeps_an_empty_argument, "''", "");

#[test]
fn a_later_prompt_lifecycle_set_value_dispatches_its_template() {
    let sequence = Sequence::new(
        "sequence-later-stack-set",
        "---\nsequence:\n    - name: first\n      shell: argv-recorder first\n    - name: second\n      prompt: child.md\n---\nBody.\n",
    );
    std::fs::write(sequence.fixture.cwd().join("child.md"), "---\nstart:\n  - set:\n      captured: \"$(argv-recorder {{ last(seq.outputs) }}from-set)\"\n  - shell: argv-recorder primary\n---\nPrompt.\n").unwrap();
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["first"]), argv(&["from-set"]), argv(&["primary"])]);
}

#[test]
fn a_transcluded_documents_missing_late_value_fails_before_prompt_launch() {
    let sequence = Sequence::new(
        "sequence-transcluded-document-barrier",
        "---\nsequence:\n    - name: first\n      shell: argv-recorder first\n    - name: prompt\n      prompt: child.md\n---\nBody.\n",
    );
    std::fs::write(sequence.fixture.cwd().join("child.md"), "::file nested.md\n").unwrap();
    std::fs::write(sequence.fixture.cwd().join("nested.md"), "::shell argv-recorder {{ seq.outputs[3] }}\n").unwrap();
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.names("outputs[3]") && outcome.stderr.contains("missing"), "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["first"])]);
}


#[test]
fn a_null_caller_value_fails_before_step_one_and_provider_launch() {
    let sequence = Sequence::new("sequence-null-caller", BUILD_CLEAN);
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&["out=null"]);
    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.names("out") && outcome.stderr.contains("missing"), "{}", outcome.stderr);
    assert!(sequence.runs().is_empty());
    assert!(!sequence.fixture.cwd().join("provider-records").exists());
}

#[test]
fn an_invalid_late_primary_value_prevents_the_step_commands() {
    let sequence = Sequence::new(
        "sequence-invalid-late-primary",
        "---\nsequence:\n  - name: first\n    shell: argv-recorder first\n  - name: second\n    shell: argv-recorder {{ seq.outputs }}\n---\nBody.\n",
    );
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(!outcome.success, "{}", outcome.stderr);
    assert!(outcome.stderr.contains("array or object"), "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["first"])]);
}

macro_rules! group_position_case {
    ($name:ident, $predecessor:literal, $rules:literal, $success:literal, $expected:expr) => {
        #[test]
        fn $name() {
            let document = format!("---\nout: a b\nsequence:\n{}  - name: grouped\n    group:\n      name: bundle\n      tasks:\n        - name: alpha\n          shell: argv-recorder alpha {{{{ out }}}}\n        - name: beta\n          shell: argv-recorder beta {{{{ out }}}}\n---\nBody.\n", $predecessor);
            let sequence = Sequence::new("sequence-group-position", &document);
            sequence.whitelist($rules);
            let outcome = sequence.run(&[]);
            assert_eq!(outcome.success, $success, "{}", outcome.stderr);
            let mut actual = sequence.runs();
            let mut expected = $expected;
            if !$predecessor.is_empty() && $success {
                assert_eq!(actual.first(), expected.first(), "the predecessor runs before the group");
            }
            // Concurrent group members have no guaranteed completion order.
            actual.sort();
            expected.sort();
            assert_eq!(actual, expected);
        }
    };
}

group_position_case!(a_first_group_offers_resolved_bytes_for_every_member, "",
    "exact argv-recorder alpha a b\nexact argv-recorder beta a b\ntemplate argv-recorder alpha {{ string }}\ntemplate argv-recorder beta {{ string }}\n", true,
    vec![argv(&["alpha", "a", "b"]), argv(&["beta", "a", "b"])]);
group_position_case!(a_later_group_keeps_template_mode_for_every_member, "  - name: first\n    shell: argv-recorder first\n",
    "exact argv-recorder first\ntemplate argv-recorder alpha {{ string }}\ntemplate argv-recorder beta {{ string }}\n", true,
    vec![argv(&["first"]), argv(&["alpha", "a b"]), argv(&["beta", "a b"])]);
group_position_case!(exact_rules_cannot_authorize_a_later_groups_members, "  - name: first\n    shell: argv-recorder first\n",
    "exact argv-recorder first\nexact argv-recorder alpha a b\nexact argv-recorder beta a b\n", false,
    Vec::<Vec<String>>::new());

#[test]
fn a_group_value_is_late_even_in_the_first_group() {
    let sequence = Sequence::new(
        "sequence-group-late-value",
        "---\nsequence:\n  - name: grouped\n    group:\n      name: bundle\n      variables:\n        label: a b\n      tasks:\n        - name: member\n          shell: argv-recorder {{ group.label }}\n---\nBody.\n",
    );
    sequence.whitelist("template argv-recorder {{ string }}\n");
    let outcome = sequence.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["a b"])]);
}


macro_rules! invalid_template_case {
    ($name:ident, $command:literal) => {
        #[test]
        fn $name() {
            let document = format!("---\nout: safe\nsequence:\n  - name: first\n    shell: argv-recorder first\n  - name: invalid\n    shell: {}\n---\nBody.\n", serde_json::to_string($command).unwrap());
            let sequence = Sequence::new("sequence-invalid-template", &document);
            sequence.whitelist("prefix argv-recorder\n");
            let outcome = sequence.run(&[]);
            assert!(!outcome.success, "{}", outcome.stderr);
            assert!(outcome.stderr.contains("template"), "{}", outcome.stderr);
            assert!(sequence.runs().is_empty(), "invalid template must fail before step 1");
            assert!(!sequence.fixture.cwd().join("provider-records").exists());
        }
    };
}

invalid_template_case!(a_prefix_cannot_authorize_a_pipe_in_a_template, "argv-recorder {{ out }} | argv-recorder second");
invalid_template_case!(a_prefix_cannot_authorize_a_semicolon_in_a_template, "argv-recorder {{ out }}; argv-recorder second");
invalid_template_case!(a_prefix_cannot_authorize_a_variable_program_word, "{{ out }} first");
invalid_template_case!(a_prefix_cannot_authorize_a_variable_redirect_target, "argv-recorder {{ out }} > {{ out }}");
invalid_template_case!(a_prefix_cannot_authorize_an_append_redirect, "argv-recorder {{ out }} >> trace.txt");
invalid_template_case!(a_prefix_cannot_authorize_an_input_redirect, "argv-recorder {{ out }} < trace.txt");
invalid_template_case!(a_prefix_cannot_authorize_a_background_template, "argv-recorder {{ out }} &");

#[test]
fn a_later_tasks_parameter_reads_the_mutation_from_its_predecessor() {
    let sequence = Sequence::new(
        "sequence-changed-task-parameter",
        "---\nvalue: before\nsequence:\n  - name: change\n    side_effect:\n      set:\n        value: two words\n  - name: consume\n    with:\n      word: '{{ value }}'\n    prompt: child.md\n---\nBody.\n",
    );
    sequence.whitelist("template argv-recorder {{ string }}\n");
    std::fs::write(sequence.fixture.cwd().join("child.md"), "::shell argv-recorder {{ word }}\n\nPrompt.\n").unwrap();
    let outcome = sequence.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["two words"])]);
}


macro_rules! native_task_redirect_case {
    ($name:ident, $redirect:literal, $expected:literal) => {
        #[test]
        fn $name() {
            let document = format!("---\nword: a b\nsequence:\n  - name: quiet\n    side_effect:\n      set:\n        seen: true\n  - name: probe\n    shell: argv-recorder {{{{ word }}}} {}\n  - name: consume\n    shell: argv-recorder {{{{ last(seq.outputs) }}}}\n---\nBody.\n", $redirect);
            let sequence = Sequence::new("sequence-native-redirection", &document);
            sequence.whitelist("template argv-recorder {{ string }} 2>&1 > /dev/null\ntemplate argv-recorder {{ string }} > /dev/null 2>&1\ntemplate argv-recorder {{ string }} 2> /dev/null\ntemplate argv-recorder {{ string }}\n");
            let outcome = sequence.run_env(&[], &[("ARGV_RECORDER_STDOUT", "stdout-probe"), ("ARGV_RECORDER_STDERR", "stderr-probe")]);
            assert!(outcome.success, "{}", outcome.stderr);
            assert_eq!(sequence.runs(), vec![argv(&["a b"]), argv(&[$expected])]);
        }
    };
}

native_task_redirect_case!(native_task_merge_then_null_retains_stderr_capture, "2>&1 > /dev/null", "stderr-probe");
native_task_redirect_case!(native_task_null_then_merge_discards_both, "> /dev/null 2>&1", "");
native_task_redirect_case!(native_task_stderr_null_retains_stdout_capture, "2> /dev/null", "stdout-probe");

macro_rules! eligible_site_case {
    ($name:ident, $position:literal, $surface:literal) => {
        #[test]
        fn $name() {
            let probe = "argv-recorder probe {{ out }}";
            let task = if $surface == "primary" {
                format!("name: probe\nshell: '{probe}'\n")
            } else { "name: probe\nprompt: child.md\n".to_string() };
            let mut document = "---\nout: a b\nsequence:\n".to_string();
            let indent = if $position == "group" {
                document.push_str("  - name: bundle\n    group:\n      name: bundle\n      tasks:\n");
                "        "
            } else {
                if $position == "later" { document.push_str("  - name: prior\n    shell: argv-recorder prior\n"); }
                "  "
            };
            for (index, line) in task.lines().enumerate() {
                document.push_str(indent);
                document.push_str(if index == 0 { "- " } else { "  " });
                document.push_str(line);
                document.push('\n');
            }
            document.push_str("---\nBody.\n");
            let sequence = Sequence::new("sequence-either-mode", &document);
            if $surface != "primary" {
                let action = if $surface.ends_with("_set") {
                    format!("  - set:\n      captured: '$({probe})'\n")
                } else { format!("  - shell: '{probe}'\n") };
                let lifecycle = if $surface.starts_with("setup") {
                    format!("start:\n{action}  - shell: argv-recorder primary\n")
                } else { format!("start:\n  - shell: argv-recorder primary\nsuccess:\n{action}") };
                std::fs::write(sequence.fixture.cwd().join("child.md"), format!("---\nout: a b\n{lifecycle}---\nPrompt.\n")).unwrap();
            }
            sequence.whitelist("exact argv-recorder primary\nexact argv-recorder prior\ntemplate argv-recorder probe {{ string }}\n");
            let outcome = sequence.run(&[]);
            assert!(outcome.success, "{}", outcome.stderr);
            let mut expected = Vec::new();
            if $position == "later" { expected.push(argv(&["prior"])); }
            if $surface.starts_with("teardown") { expected.push(argv(&["primary"])); }
            expected.push(argv(&["probe", "a b"]));
            if $surface.starts_with("setup") { expected.push(argv(&["primary"])); }
            assert_eq!(sequence.runs(), expected);
        }
    };
}
eligible_site_case!(first_primary_accepts_shape, "first", "primary");
eligible_site_case!(first_prompt_start_accepts_shape, "first", "setup");
eligible_site_case!(first_prompt_success_accepts_shape, "first", "teardown");
eligible_site_case!(first_prompt_start_set_accepts_shape, "first", "setup_set");
eligible_site_case!(first_prompt_success_set_accepts_shape, "first", "teardown_set");
eligible_site_case!(group_primary_accepts_shape, "group", "primary");
eligible_site_case!(group_prompt_start_accepts_shape, "group", "setup");
eligible_site_case!(group_prompt_success_accepts_shape, "group", "teardown");
eligible_site_case!(group_prompt_start_set_accepts_shape, "group", "setup_set");
eligible_site_case!(group_prompt_success_set_accepts_shape, "group", "teardown_set");
eligible_site_case!(later_primary_accepts_shape, "later", "primary");
eligible_site_case!(later_prompt_start_accepts_shape, "later", "setup");
eligible_site_case!(later_prompt_success_accepts_shape, "later", "teardown");
eligible_site_case!(later_prompt_start_set_accepts_shape, "later", "setup_set");
eligible_site_case!(later_prompt_success_set_accepts_shape, "later", "teardown_set");

macro_rules! eligible_document_case {
    ($name:ident, $standalone:literal, $surface:literal) => {
        #[test]
        fn $name() {
            let sequence = Sequence::new("sequence-known-document-shape", "---\nsequence:\n  - name: prompt\n    prompt: child.md\n---\nBody.\n");
            let (metadata, body) = match $surface {
                "directive" => (String::new(), "::shell argv-recorder probe {{ out }}\n\nPrompt.\n"),
                "block" => (String::new(), "::shell-block\nargv-recorder probe {{ out }}\n::end-block\n\nPrompt.\n"),
                "frontmatter" => ("captured: '$(argv-recorder probe {{ out }})'\n".to_string(), "Prompt.\n"),
                "shell" => ("start:\n  - shell: 'argv-recorder probe {{ out }}'\n".to_string(), "Prompt.\n"),
                _ => ("start:\n  - set:\n      captured: '$(argv-recorder probe {{ out }})'\n".to_string(), "Prompt.\n"),
            };
            std::fs::write(sequence.fixture.cwd().join("child.md"), format!("---\nout: a b\n{metadata}---\n{body}")).unwrap();
            sequence.whitelist("template argv-recorder probe {{ string }}\n");
            if $standalone {
                let output = sequence.fixture.command().env(ARGV_RECORDER_FILE, &sequence.records)
                    .env("FAKE_DIR", sequence.fixture.cwd().join("provider-records"))
                    .args(["compose", "--goose", "child.md"]).output().unwrap();
                assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            } else {
                let outcome = sequence.run(&[]);
                assert!(outcome.success, "{}", outcome.stderr);
            }
            assert_eq!(sequence.runs(), vec![argv(&["probe", "a b"])]);
        }
    };
}
eligible_document_case!(known_prompt_directive_accepts_shape, false, "directive");
eligible_document_case!(known_prompt_block_accepts_shape, false, "block");
eligible_document_case!(known_prompt_frontmatter_accepts_shape, false, "frontmatter");
eligible_document_case!(known_prompt_shell_accepts_shape, false, "shell");
eligible_document_case!(known_prompt_set_accepts_shape, false, "set");
eligible_document_case!(known_standalone_directive_accepts_shape, true, "directive");
eligible_document_case!(known_standalone_block_accepts_shape, true, "block");
eligible_document_case!(known_standalone_frontmatter_accepts_shape, true, "frontmatter");
eligible_document_case!(known_standalone_shell_accepts_shape, true, "shell");
eligible_document_case!(known_standalone_set_accepts_shape, true, "set");

fn referenced_lifecycle_child(event: &str, form: &str, read: &str) -> String {
    let action = if form == "shell" {
        format!("- shell: 'argv-recorder probe {{{{ {read} }}}}'")
    } else {
        format!("- set:\n    captured: '$(argv-recorder probe {{{{ {read} }}}})'")
    };
    let (head, indent) = if event == "loop" {
        ("loop:\n  until: 'loop.count >= 1'\n  gate:\n".to_string(), "    ")
    } else { (format!("{event}:\n"), "  ") };
    let action = action.lines().map(|line| format!("{indent}{line}\n")).collect::<String>();
    format!("---\nout: a b\n{head}{action}---\nPrompt.\n")
}

fn referenced_lifecycle_sequence() -> Sequence {
    Sequence::new("sequence-referenced-lifecycle", "---\nsequence:\n  - name: prior\n    shell: argv-recorder prior\n  - name: consume\n    prompt: child.md\n---\nBody.\n")
}

fn assert_referenced_lifecycle_mode(event: &str, form: &str, policy: &str) {
    let sequence = referenced_lifecycle_sequence();
    std::fs::write(sequence.fixture.cwd().join("child.md"), referenced_lifecycle_child(event, form, "out")).unwrap();
    sequence.whitelist(policy);
    let outcome = sequence.run(&[]);
    if policy.contains("probe a b") {
        assert!(!outcome.success, "{}", outcome.stderr);
        assert!(sequence.runs().is_empty(), "exact-only refusal precedes the sequence");
    } else {
        assert!(outcome.success, "{}", outcome.stderr);
        // A looping prompt document runs its loop gate as a sequence step.
        let expected = if matches!(event, "start" | "success" | "finalize" | "loop") {
            vec![argv(&["prior"]), argv(&["probe", "a b"])]
        } else { vec![argv(&["prior"])] };
        assert_eq!(sequence.runs(), expected);
    }
}

macro_rules! referenced_lifecycle_case {
    ($name:ident, $event:literal, $form:literal) => {
        mod $name {
            use super::*;
            #[test]
            fn prefix_preserves_one_argument() {
                assert_referenced_lifecycle_mode($event, $form, "prefix argv-recorder\n");
            }
            #[test]
            fn shape_preserves_one_argument() {
                assert_referenced_lifecycle_mode($event, $form, "exact argv-recorder prior\nexact argv-recorder setup\ntemplate argv-recorder probe {{ string }}\n");
            }
            #[test]
            fn exact_only_refused_before_execution() {
                assert_referenced_lifecycle_mode($event, $form, "exact argv-recorder prior\nexact argv-recorder setup\nexact argv-recorder probe a b\n");
            }
        }
    };
}
referenced_lifecycle_case!(referenced_start_shell_modes, "start", "shell");
referenced_lifecycle_case!(referenced_start_set_modes, "start", "set");
referenced_lifecycle_case!(referenced_success_shell_modes, "success", "shell");
referenced_lifecycle_case!(referenced_success_set_modes, "success", "set");
referenced_lifecycle_case!(referenced_finalize_shell_modes, "finalize", "shell");
referenced_lifecycle_case!(referenced_finalize_set_modes, "finalize", "set");
referenced_lifecycle_case!(referenced_blocked_shell_modes, "blocked", "shell");
referenced_lifecycle_case!(referenced_blocked_set_modes, "blocked", "set");
referenced_lifecycle_case!(referenced_failure_shell_modes, "failure", "shell");
referenced_lifecycle_case!(referenced_failure_set_modes, "failure", "set");
referenced_lifecycle_case!(referenced_loop_shell_modes, "loop", "shell");
referenced_lifecycle_case!(referenced_loop_set_modes, "loop", "set");

macro_rules! referenced_lifecycle_barrier {
    ($name:ident, $event:literal, $form:literal, $read:literal, $error:literal) => {
        #[test]
        fn $name() {
            let sequence = referenced_lifecycle_sequence();
            std::fs::write(sequence.fixture.cwd().join("child.md"), referenced_lifecycle_child($event, $form, $read)).unwrap();
            sequence.whitelist("prefix argv-recorder\n");
            let outcome = sequence.run(&[]);
            assert!(!outcome.success, "{}", outcome.stderr);
            assert!(outcome.stderr.contains($error), "{}", outcome.stderr);
            assert_eq!(sequence.runs(), vec![argv(&["prior"])]);
        }
    };
}
referenced_lifecycle_barrier!(referenced_shell_missing_barrier, "start", "shell", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_set_missing_barrier, "start", "set", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_shell_array_barrier, "start", "shell", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_set_array_barrier, "start", "set", "seq.outputs", "array or object");

macro_rules! referenced_group_lifecycle {
    ($name:ident, $form:literal, $read:literal, $success:literal) => {
        #[test]
        fn $name() {
            let sequence = Sequence::new("sequence-group-lifecycle", "---\nsequence:\n  - name: bundle\n    group:\n      name: bundle\n      variables:\n        label: a b\n      tasks:\n        - name: prompt\n          prompt: child.md\n---\nBody.\n");
            std::fs::write(sequence.fixture.cwd().join("child.md"), referenced_lifecycle_child("start", $form, $read)).unwrap();
            sequence.whitelist("prefix argv-recorder\n");
            let outcome = sequence.run(&[]);
            assert_eq!(outcome.success, $success, "{}", outcome.stderr);
            let expected = if $success { vec![argv(&["probe", "a b"])] } else { Vec::new() };
            assert_eq!(sequence.runs(), expected);
            if !$success { assert!(outcome.stderr.contains("missing"), "{}", outcome.stderr); }
        }
    };
}
referenced_group_lifecycle!(referenced_group_shell_scope, "shell", "group.label", true);
referenced_group_lifecycle!(referenced_group_set_scope, "set", "group.label", true);
referenced_group_lifecycle!(referenced_group_shell_missing, "shell", "group.missing", false);
referenced_group_lifecycle!(referenced_group_set_missing, "set", "group.missing", false);

macro_rules! referenced_lifecycle_known_refusal {
    ($name:ident, $form:literal, $read:literal, $metadata:literal) => {
        #[test]
        fn $name() {
            let sequence = referenced_lifecycle_sequence();
            let child = referenced_lifecycle_child("start", $form, $read).replace("out: a b\n", $metadata);
            std::fs::write(sequence.fixture.cwd().join("child.md"), child).unwrap();
            sequence.whitelist("prefix argv-recorder\n");
            let outcome = sequence.run(&[]);
            assert!(!outcome.success, "{}", outcome.stderr);
            assert!(sequence.runs().is_empty(), "approval-time refusals precede prior: {:?}; {}", sequence.runs(), outcome.stderr);
        }
    };
}
referenced_lifecycle_known_refusal!(referenced_shell_absent_known, "shell", "out", "");
referenced_lifecycle_known_refusal!(referenced_set_absent_known, "set", "out", "");
referenced_lifecycle_known_refusal!(referenced_shell_null_known, "shell", "out", "out: null\n");
referenced_lifecycle_known_refusal!(referenced_set_null_known, "set", "out", "out: null\n");
referenced_lifecycle_known_refusal!(referenced_shell_compound_err, "shell", "last(seq.outputs) }} {{ err.msg", "");
referenced_lifecycle_known_refusal!(referenced_set_compound_err, "set", "last(seq.outputs) }} {{ err.msg", "");
referenced_lifecycle_known_refusal!(referenced_shell_compound_current, "shell", "last(seq.outputs) }} {{ current.branch", "");
referenced_lifecycle_known_refusal!(referenced_set_compound_current, "set", "last(seq.outputs) }} {{ current.branch", "");

#[test]
fn referenced_lifecycle_derived_late_value_retains_its_provenance() {
    let sequence = referenced_lifecycle_sequence();
    let child = referenced_lifecycle_child("start", "set", "derived")
        .replace("out: a b\n", "derived: '{{ last(seq.outputs) }}'\n");
    std::fs::write(sequence.fixture.cwd().join("child.md"), child).unwrap();
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["prior"]), argv(&["probe", ""])]);
}

#[test]
fn referenced_lifecycle_parameter_overrides_the_child_default() {
    let sequence = Sequence::new("sequence-lifecycle-with", "---\nsequence:\n  - name: prior\n    shell: argv-recorder prior\n  - name: consume\n    with:\n      out: task value\n    prompt: child.md\n---\nBody.\n");
    std::fs::write(sequence.fixture.cwd().join("child.md"), referenced_lifecycle_child("start", "shell", "out")).unwrap();
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run(&[]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["prior"]), argv(&["probe", "task value"])]);
}

referenced_lifecycle_barrier!(referenced_success_shell_missing_barrier, "success", "shell", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_success_shell_array_barrier, "success", "shell", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_success_set_missing_barrier, "success", "set", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_success_set_array_barrier, "success", "set", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_finalize_shell_missing_barrier, "finalize", "shell", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_finalize_shell_array_barrier, "finalize", "shell", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_finalize_set_missing_barrier, "finalize", "set", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_finalize_set_array_barrier, "finalize", "set", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_blocked_shell_missing_barrier, "blocked", "shell", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_blocked_shell_array_barrier, "blocked", "shell", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_blocked_set_missing_barrier, "blocked", "set", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_blocked_set_array_barrier, "blocked", "set", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_failure_shell_missing_barrier, "failure", "shell", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_failure_shell_array_barrier, "failure", "shell", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_failure_set_missing_barrier, "failure", "set", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_failure_set_array_barrier, "failure", "set", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_loop_shell_missing_barrier, "loop", "shell", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_loop_shell_array_barrier, "loop", "shell", "seq.outputs", "array or object");
referenced_lifecycle_barrier!(referenced_loop_set_missing_barrier, "loop", "set", "last(seq.outputs).missing", "missing");
referenced_lifecycle_barrier!(referenced_loop_set_array_barrier, "loop", "set", "seq.outputs", "array or object");

referenced_lifecycle_known_refusal!(referenced_shell_group_outside, "shell", "group.label", "");
referenced_lifecycle_known_refusal!(referenced_set_group_outside, "set", "group.label", "");

fn group_barrier_fixture(execution: &str, surface: &str, read: &str) -> Sequence {
    let probe = format!("argv-recorder probe {{{{ {read} }}}}");
    let task = match surface {
        "primary" => format!("name: beta\nshell: '{probe}'\n"),
        stage @ ("setup" | "teardown") => format!("name: beta\n{stage}:\n  - shell: '{probe}'\nshell: argv-recorder beta\n"),
        "setup_set" | "teardown_set" => format!("name: beta\n{}:\n  - set:\n      captured: '$({probe})'\nshell: argv-recorder beta\n", surface.trim_end_matches("_set")),
        _ => "name: beta\nprompt: child.md\n".to_string(),
    };
    let execution_key = if execution == "serial" { "      execution: serial\n" } else { "" };
    let mut document = format!("---\nsequence:\n  - name: prior\n    shell: argv-recorder prior\n  - name: bundle\n    group:\n      name: bundle\n{execution_key}      tasks:\n        - name: alpha\n          shell: argv-recorder alpha\n");
    for (index, line) in task.lines().enumerate() {
        document.push_str(if index == 0 { "        - " } else { "          " });
        document.push_str(line);
        document.push('\n');
    }
    document.push_str("---\nBody.\n");
    let sequence = Sequence::new("sequence-group-entry-barrier", &document);
    let child = match surface {
        "directive" | "transcluded" => format!("::shell {probe}\n\nPrompt.\n"),
        "block" => format!("::shell-block\n{probe}\n::end-block\n\nPrompt.\n"),
        "frontmatter" => format!("---\nresult: '$({probe})'\n---\nPrompt.\n"),
        surface if surface.contains('.') => {
            let (event, form) = surface.split_once('.').unwrap();
            referenced_lifecycle_child(event, form, read)
        }
        _ => "Prompt.\n".to_string(),
    };
    if surface == "transcluded" {
        std::fs::write(sequence.fixture.cwd().join("nested.md"), child).unwrap();
        std::fs::write(sequence.fixture.cwd().join("child.md"), "::file nested.md\n").unwrap();
    } else {
        std::fs::write(sequence.fixture.cwd().join("child.md"), child).unwrap();
    }
    sequence.whitelist("prefix argv-recorder\n");
    sequence
}

fn assert_group_entry_barrier(execution: &str, surface: &str, read: &str, error: &str) {
    let sequence = group_barrier_fixture(execution, surface, read);
    let outcome = sequence.run(&[]);
    assert!(!outcome.success, "{}", outcome.stderr);
    if execution == "serial" || matches!(surface, "setup" | "teardown" | "setup_set" | "teardown_set") {
        assert!(outcome.stderr.contains("removed"), "{}", outcome.stderr);
        assert!(sequence.runs().is_empty(), "retired syntax is refused before prior executes");
    } else {
        assert!(outcome.stderr.contains(error), "{}", outcome.stderr);
        assert_eq!(sequence.runs(), vec![argv(&["prior"])], "every member must remain unrun");
    }
}

macro_rules! group_barrier_case {
    ($name:ident, $surface:literal) => {
        mod $name {
            use super::*;
            #[test]
            fn serial_execution_with_missing_input_is_rejected() { assert_group_entry_barrier("serial", $surface, "group.missing", "missing"); }
            #[test]
            fn parallel_missing() { assert_group_entry_barrier("parallel", $surface, "group.missing", "missing"); }
            #[test]
            fn serial_execution_with_array_input_is_rejected() { assert_group_entry_barrier("serial", $surface, "seq.outputs", "array or object"); }
            #[test]
            fn parallel_array() { assert_group_entry_barrier("parallel", $surface, "seq.outputs", "array or object"); }
        }
    };
}
group_barrier_case!(group_barrier_primary, "primary");
group_barrier_case!(group_barrier_setup, "setup");
group_barrier_case!(group_barrier_teardown, "teardown");
group_barrier_case!(group_barrier_setup_set, "setup_set");
group_barrier_case!(group_barrier_teardown_set, "teardown_set");
group_barrier_case!(group_barrier_directive, "directive");
group_barrier_case!(group_barrier_block, "block");
group_barrier_case!(group_barrier_frontmatter, "frontmatter");
group_barrier_case!(group_barrier_transcluded, "transcluded");
group_barrier_case!(group_barrier_start_shell, "start.shell");
group_barrier_case!(group_barrier_start_set, "start.set");
group_barrier_case!(group_barrier_success_shell, "success.shell");
group_barrier_case!(group_barrier_success_set, "success.set");
group_barrier_case!(group_barrier_finalize_shell, "finalize.shell");
group_barrier_case!(group_barrier_finalize_set, "finalize.set");
group_barrier_case!(group_barrier_blocked_shell, "blocked.shell");
group_barrier_case!(group_barrier_blocked_set, "blocked.set");
group_barrier_case!(group_barrier_failure_shell, "failure.shell");
group_barrier_case!(group_barrier_failure_set, "failure.set");
group_barrier_case!(group_barrier_loop_shell, "loop.shell");
group_barrier_case!(group_barrier_loop_set, "loop.set");

fn assert_group_late_snapshot(surface: &str) {
    let sequence = group_barrier_fixture("parallel", surface, "last(seq.outputs) }} {{ out");
    let path = sequence.fixture.cwd().join("seq.md");
    let source = std::fs::read_to_string(&path).unwrap()
        .replacen("---\n", "---\nout: before\n", 1)
        .replace("      tasks:\n", "      max_parallel: 1\n      tasks:\n")
        .replace("          shell: argv-recorder alpha\n", "          prompt: alpha.md\n");
    std::fs::write(sequence.fixture.cwd().join("alpha.md"), "---\nstart:\n  - set:\n      out: after\n  - shell: argv-recorder alpha-one\n  - shell: argv-recorder alpha-two\n---\nAlpha.\n").unwrap();
    std::fs::write(path, source).unwrap();
    let child_path = sequence.fixture.cwd().join(if surface == "transcluded" { "nested.md" } else { "child.md" });
    let child = std::fs::read_to_string(&child_path).unwrap();
    let child = if child.contains("out: a b\n") {
        child.replace("out: a b\n", "out: before\n")
    } else if child.starts_with("---\n") {
        child.replacen("---\n", "---\nout: before\n", 1)
    } else { format!("---\nout: before\n---\n{child}") };
    std::fs::write(child_path, child).unwrap();
    let outcome = sequence.run_env(&[], &[("ARGV_RECORDER_STDOUT", "seed")]);
    if matches!(surface, "setup" | "teardown" | "setup_set" | "teardown_set") {
        assert!(!outcome.success && outcome.stderr.contains("removed"), "{}", outcome.stderr);
        assert!(sequence.runs().is_empty());
        return;
    }
    assert!(outcome.success, "{}", outcome.stderr);
    let records = sequence.runs();
    let probes = records.iter().filter(|args| args.first().is_some_and(|arg| arg == "probe")).collect::<Vec<_>>();
    assert_eq!(probes, vec![&argv(&["probe", "seed", "before"])], "late outputs stay at group entry and sibling mutations stay private: {records:?}");
    assert_eq!(&records[..3], &[argv(&["prior"]), argv(&["alpha-one"]), argv(&["alpha-two"])]);
}

macro_rules! group_snapshot_case {
    ($name:ident, $surface:literal) => {
        #[test]
        fn $name() { assert_group_late_snapshot($surface); }
    };
}
group_snapshot_case!(group_snapshot_primary, "primary");
group_snapshot_case!(group_snapshot_setup, "setup");
group_snapshot_case!(group_snapshot_teardown, "teardown");
group_snapshot_case!(group_snapshot_setup_set, "setup_set");
group_snapshot_case!(group_snapshot_teardown_set, "teardown_set");
group_snapshot_case!(group_snapshot_directive, "directive");
group_snapshot_case!(group_snapshot_block, "block");
group_snapshot_case!(group_snapshot_frontmatter, "frontmatter");
group_snapshot_case!(group_snapshot_transcluded, "transcluded");
group_snapshot_case!(group_snapshot_lifecycle_shell, "start.shell");
group_snapshot_case!(group_snapshot_lifecycle_set, "start.set");

#[test]
fn group_snapshot_parallel_prompt_mutations_stay_private_until_merge() {
    let sequence = Sequence::new("sequence-group-private-prompt", "---\nout: before\nsequence:\n  - name: prior\n    shell: argv-recorder prior\n  - name: bundle\n    group:\n      name: bundle\n      max_parallel: 1\n      tasks:\n        - name: writer\n          prompt: writer.md\n        - name: reader\n          prompt: reader.md\n  - name: after\n    shell: 'argv-recorder after {{ out }}'\n---\nBody.\n");
    std::fs::write(sequence.fixture.cwd().join("writer.md"), "---\nout: before\nstart:\n  - set:\n      out: after\n  - shell: 'argv-recorder writer {{ last(seq.outputs) }} {{ out }}'\n---\nWriter.\n").unwrap();
    std::fs::write(sequence.fixture.cwd().join("reader.md"), "---\nout: before\nstart:\n  - shell: 'argv-recorder reader {{ last(seq.outputs) }} {{ out }}'\n---\nReader.\n").unwrap();
    sequence.whitelist("prefix argv-recorder\n");
    let outcome = sequence.run_env(&[], &[("ARGV_RECORDER_STDOUT", "seed")]);
    assert!(outcome.success, "{}", outcome.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["prior"]), argv(&["writer", "seed", "after"]), argv(&["reader", "seed", "before"]), argv(&["after", "after"])]);
}


fn byte_grammar_fixture(position: &str) -> Sequence {
    let tasks = match position {
        "group" => "  - name: bundle\n    group:\n      name: bundle\n      tasks:\n        - name: prompt\n          prompt: child.md\n",
        "later" => "  - name: prior\n    shell: argv-recorder prior\n  - name: prompt\n    prompt: child.md\n",
        _ => "  - name: prompt\n    prompt: child.md\n",
    };
    Sequence::new("sequence-byte-grammar", &format!("---\nsequence:\n{tasks}---\nBody.\n"))
}

fn byte_grammar_child(event: &str, form: &str, command: &str) -> String {
    let action = match form {
        "long" => format!("- shell:\n    command: '{command}'"),
        "set" => format!("- set:\n    captured: '$({command})'"),
        _ => format!("- shell: '{command}'"),
    };
    let (head, indent) = if event == "loop" {
        ("loop:\n  until: 'loop.count >= 1'\n  gate:\n".to_string(), "    ")
    } else { (format!("{event}:\n"), "  ") };
    format!("---\ntool: argv-recorder\nout: dist\n{head}{}---\nPrompt.\n",
        action.lines().map(|line| format!("{indent}{line}\n")).collect::<String>())
}

fn byte_grammar_run(sequence: &Sequence, position: &str) -> Outcome {
    if position != "standalone" { return sequence.run(&[]) }
    let output = sequence.fixture.command().env(ARGV_RECORDER_FILE, &sequence.records)
        .env("FAKE_DIR", sequence.fixture.cwd().join("provider-records"))
        .args(["compose", "--goose", "child.md"]).output().unwrap();
    assert!(!sequence.fixture.audio_spool().exists());
    Outcome { success: output.status.success(), stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)) }
}

fn assert_byte_grammar(event: &str, form: &str, position: &str) {
    let sequence = byte_grammar_fixture(position);
    std::fs::write(sequence.fixture.cwd().join("child.md"), byte_grammar_child(event, form, "{{ tool }} probe")).unwrap();
    sequence.whitelist("exact argv-recorder prior\nexact argv-recorder probe\n");
    let outcome = byte_grammar_run(&sequence, position);
    if position == "later" || form == "set" {
        assert!(!outcome.success, "{}", outcome.stderr);
        if position == "later" && form != "set" { assert!(outcome.names("invalid shell template grammar"), "{}", outcome.stderr); }
        if form == "set" { assert!(outcome.names("executable"), "{}", outcome.stderr); }
        assert!(sequence.runs().is_empty(), "refusal must precede all sequence effects");
    } else {
        assert!(outcome.success, "{}", outcome.stderr);
        // A looping prompt document runs its loop gate wherever it runs.
        let expected = if matches!(event, "start" | "success" | "finalize" | "loop") { vec![argv(&["probe"])] } else { vec![] };
        assert_eq!(sequence.runs(), expected, "inactive stacks are audited without execution");
    }
}

macro_rules! byte_grammar_event {
    ($module:ident, $event:literal) => {
        mod $module {
            use super::*;
            #[test] fn first_short() { assert_byte_grammar($event, "shell", "first"); }
            #[test] fn group_short() { assert_byte_grammar($event, "shell", "group"); }
            #[test] fn first_long() { assert_byte_grammar($event, "long", "first"); }
            #[test] fn group_long() { assert_byte_grammar($event, "long", "group"); }
            #[test] fn standalone_short() { assert_byte_grammar($event, "shell", "standalone"); }
            #[test] fn standalone_long() { assert_byte_grammar($event, "long", "standalone"); }
            #[test] fn later_short_refused() { assert_byte_grammar($event, "shell", "later"); }
            #[test] fn later_long_refused() { assert_byte_grammar($event, "long", "later"); }
            #[test] fn first_set_refused() { assert_byte_grammar($event, "set", "first"); }
            #[test] fn group_set_refused() { assert_byte_grammar($event, "set", "group"); }
        }
    };
}
byte_grammar_event!(byte_grammar_start, "start");
byte_grammar_event!(byte_grammar_success, "success");
byte_grammar_event!(byte_grammar_finalize, "finalize");
byte_grammar_event!(byte_grammar_failure, "failure");
byte_grammar_event!(byte_grammar_blocked, "blocked");
byte_grammar_event!(byte_grammar_loop, "loop");

#[test]
fn byte_grammar_standalone_set_refused() { assert_byte_grammar("start", "set", "standalone"); }

macro_rules! byte_grammar_chain {
    ($name:ident, $position:literal) => {
        #[test]
        fn $name() {
            let sequence = byte_grammar_fixture($position);
            // cmd uses `&` for the same authored, exact-approved two-command chain.
            let separator = if cfg!(windows) { "&" } else { ";" };
            let command = format!("argv-recorder {{{{ out }}}}{separator} argv-recorder tail");
            std::fs::write(sequence.fixture.cwd().join("child.md"), byte_grammar_child("start", "shell", &command)).unwrap();
            sequence.whitelist(&format!("exact argv-recorder dist{separator} argv-recorder tail\n"));
            let outcome = byte_grammar_run(&sequence, $position);
            assert!(outcome.success, "{}", outcome.stderr);
            assert_eq!(sequence.runs(), vec![argv(&["dist"]), argv(&["tail"])]);
        }
    };
}
byte_grammar_chain!(byte_grammar_first_chain, "first");
byte_grammar_chain!(byte_grammar_group_chain, "group");
byte_grammar_chain!(byte_grammar_standalone_chain, "standalone");

macro_rules! byte_grammar_task {
    ($name:ident, $position:literal, $surface:literal) => {
        #[test]
        fn $name() {
            let command = "{{ tool }} probe";
            let task = if $surface == "primary" {
                format!("name: probe\nshell: '{command}'\n")
            } else { "name: probe\nprompt: child.md\n".to_string() };
            let mut document = "---\ntool: argv-recorder\nsequence:\n".to_string();
            let indent = if $position == "group" {
                document.push_str("  - name: bundle\n    group:\n      name: bundle\n      tasks:\n");
                "        "
            } else { "  " };
            for (index, line) in task.lines().enumerate() {
                document.push_str(&format!("{indent}{}{line}\n", if index == 0 { "- " } else { "  " }));
            }
            document.push_str("---\nBody.\n");
            let sequence = Sequence::new("sequence-byte-task", &document);
            if $surface != "primary" {
                let action = if $surface.ends_with("_set") {
                    format!("  - set:\n      captured: '$({command})'\n")
                } else { format!("  - shell: '{command}'\n") };
                let lifecycle = if $surface.starts_with("setup") {
                    format!("start:\n{action}  - shell: argv-recorder primary\n")
                } else { format!("start:\n  - shell: argv-recorder primary\nsuccess:\n{action}") };
                std::fs::write(sequence.fixture.cwd().join("child.md"), format!("---\ntool: argv-recorder\n{lifecycle}---\nPrompt.\n")).unwrap();
            }
            sequence.whitelist("exact argv-recorder primary\nexact argv-recorder probe\n");
            let outcome = sequence.run(&[]);
            if $surface.ends_with("_set") {
                assert!(!outcome.success, "{}", outcome.stderr);
                assert!(sequence.runs().is_empty());
                assert!(outcome.names("executable"), "{}", outcome.stderr);
            } else {
                assert!(outcome.success, "{}", outcome.stderr);
                let expected = match $surface {
                    "primary" => vec![argv(&["probe"])],
                    "setup" => vec![argv(&["probe"]), argv(&["primary"])],
                    _ => vec![argv(&["primary"]), argv(&["probe"])],
                };
                assert_eq!(sequence.runs(), expected);
            }
        }
    };
}
byte_grammar_task!(byte_grammar_first_primary, "first", "primary");
byte_grammar_task!(byte_grammar_first_prompt_start, "first", "setup");
byte_grammar_task!(byte_grammar_first_prompt_success, "first", "teardown");
byte_grammar_task!(byte_grammar_group_primary, "group", "primary");
byte_grammar_task!(byte_grammar_group_prompt_start, "group", "setup");
byte_grammar_task!(byte_grammar_group_prompt_success, "group", "teardown");
byte_grammar_task!(byte_grammar_first_prompt_start_set, "first", "setup_set");
byte_grammar_task!(byte_grammar_first_prompt_success_set, "first", "teardown_set");
byte_grammar_task!(byte_grammar_group_prompt_start_set, "group", "setup_set");
byte_grammar_task!(byte_grammar_group_prompt_success_set, "group", "teardown_set");

macro_rules! byte_grammar_document_refusal {
    ($name:ident, $position:literal, $surface:literal) => {
        #[test]
        fn $name() {
            let sequence = byte_grammar_fixture($position);
            let (metadata, body) = match $surface {
                "directive" => ("", "::shell {{ tool }} probe\n\nPrompt.\n"),
                "block" => ("", "::shell-block\n{{ tool }} probe\n::end-block\n\nPrompt.\n"),
                _ => ("captured: '$({{ tool }} probe)'\n", "Prompt.\n"),
            };
            std::fs::write(sequence.fixture.cwd().join("child.md"), format!("---\ntool: argv-recorder\n{metadata}---\n{body}")).unwrap();
            sequence.whitelist("exact argv-recorder probe\n");
            let outcome = byte_grammar_run(&sequence, $position);
            assert!(!outcome.success, "{}", outcome.stderr);
            assert!(sequence.runs().is_empty(), "inserted executables remain refused");
        }
    };
}
byte_grammar_document_refusal!(byte_grammar_first_directive, "first", "directive");
byte_grammar_document_refusal!(byte_grammar_first_block, "first", "block");
byte_grammar_document_refusal!(byte_grammar_first_frontmatter, "first", "frontmatter");
byte_grammar_document_refusal!(byte_grammar_standalone_directive, "standalone", "directive");
byte_grammar_document_refusal!(byte_grammar_standalone_block, "standalone", "block");
byte_grammar_document_refusal!(byte_grammar_standalone_frontmatter, "standalone", "frontmatter");


macro_rules! byte_grammar_first_late_refusal {
    ($name:ident, $command:literal, $diagnostic:literal) => {
        #[test]
        fn $name() {
            let sequence = byte_grammar_fixture("first");
            std::fs::write(sequence.fixture.cwd().join("child.md"), byte_grammar_child("start", "shell", $command)).unwrap();
            sequence.whitelist("prefix argv-recorder\n");
            let outcome = sequence.run(&[]);
            assert!(!outcome.success, "{}", outcome.stderr);
            assert!(outcome.names($diagnostic), "{}", outcome.stderr);
            assert!(sequence.runs().is_empty());
        }
    };
}
byte_grammar_first_late_refusal!(byte_grammar_first_late_program, "{{ last(seq.outputs) }} probe", "invalid shell template grammar");
byte_grammar_first_late_refusal!(byte_grammar_first_late_chain, "argv-recorder {{ last(seq.outputs) }}; argv-recorder tail", "invalid shell template grammar");
byte_grammar_first_late_refusal!(byte_grammar_first_forbidden_error, "{{ tool }} {{ last(seq.outputs) }} {{ err.msg }}", "err");
byte_grammar_first_late_refusal!(byte_grammar_first_forbidden_current, "{{ tool }} {{ current.branch }}", "current");

/// A sequence task's quoted empty words are empty arguments: the first
/// task's quoted hole filled with an empty string and a later task's
/// authored `''` both run as resolved bytes under keys that spell them `""`,
/// and a key without them approves nothing.
#[test]
fn sequence_task_exact_keys_keep_quoted_empty_arguments() {
    let document = r#"---
commit_message: ""
sequence:
    - name: first
      shell: "argv-recorder -m \"{{ commit_message }}\""
    - name: second
      shell: "argv-recorder second ''"
---
Body.
"#;
    let sequence = Sequence::new("sequence-quoted-empty", document);
    sequence.whitelist("exact argv-recorder -m\nexact argv-recorder second\n");
    let refused = sequence.run(&[]);
    assert!(!refused.success, "{}", refused.stderr);
    assert!(sequence.runs().is_empty(), "nothing may run before preflight passes");

    sequence.whitelist("exact argv-recorder -m \"\"\nexact argv-recorder second \"\"\n");
    let approved = sequence.run(&[]);
    assert!(approved.success, "{}", approved.stderr);
    assert_eq!(sequence.runs(), vec![argv(&["-m", ""]), argv(&["second", ""])]);
}
