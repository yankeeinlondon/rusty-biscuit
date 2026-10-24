//! Level-1 process coverage for mixed-text span validation: a malformed
//! `{{ … }}` span inside literal text (`"status {{ fm. }}"`) in an unselected
//! branch fails preparation at every command boundary and stack home, naming
//! the operand, and the provider is never launched. Two controls launch: valid
//! mixed text, and a span that parses but would fail only if evaluated.
//!
//! The reader-by-reader table lives in the library
//! (`composition::lifecycle::tests::mixed_span`); these rows prove each
//! boundary reaches that validation before anything runs.

use crate::common;

use common::CliProcessFixture;
use common::launch_recorder::{install, launches};
use common::write;

const BAD: &str = "status {{ fm. }}";

struct Run {
    code: Option<i32>,
    output: String,
    launches: usize,
}

impl Run {
    /// The output with gutter glyphs and whitespace runs collapsed, so a match
    /// does not depend on where the terminal width wrapped a line.
    fn collapsed(&self) -> String {
        self.output
            .replace(['┃', '│'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Writes `files` into a fresh fixture with a recording `goose`, runs
/// `claudine {command} --goose {target}`, and counts provider launches.
fn run(name: &str, files: &[(&str, &str)], command: &str, target: &str) -> Run {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    let record = install(&fixture, "goose", "exit 0");
    for (file, content) in files {
        write(&fixture.cwd().join(file), content);
    }
    let output = fixture
        .command()
        .args([command, "--goose", target])
        .output()
        .expect("claudine runs");
    Run {
        code: output.status.code(),
        output: common::strip_ansi(&format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )),
        launches: launches(&record).len(),
    }
}

/// A guard that never selects `branch`, which holds `info: {text}`.
fn conditional(indent: &str, branch: &str, text: &str) -> String {
    let other = if branch == "then" { "else" } else { "then" };
    format!(
        "{indent}- when: \"pick == 'neither'\"\n\
         {indent}  {branch}:\n\
         {indent}    - info: \"{text}\"\n\
         {indent}  {other}:\n\
         {indent}    - info: \"selected branch\"\n"
    )
}

/// Either boundary may report the span first: the launch's passive schema
/// verdict names the operand by JSON pointer (`/failure/0/else/0/info`), the
/// lifecycle parser by its lifecycle path (`failure[0].else[0].info.text`).
fn assert_rejected(run: &Run, path: &str) {
    assert_ne!(run.code, Some(0), "{}", run.output);
    assert_eq!(run.launches, 0, "the provider must not launch\n{}", run.output);
    let collapsed = run.collapsed();
    let pointer = format!(
        "/{}",
        path.trim_end_matches(".text").replace('[', ".").replace(']', "").replace('.', "/")
    );
    let by_parser = collapsed.contains("invalid lifecycle action") && collapsed.contains(&format!("`{path}`"));
    let by_schema = collapsed.contains("malformed template") && collapsed.contains(&format!("`{pointer}`"));
    assert!(by_parser || by_schema, "neither `{path}` nor `{pointer}` was rejected: {collapsed}");
}

fn event_document(event: &str, branch: &str) -> String {
    format!("---\npick: other\n{event}:\n{}---\nBody.\n", conditional("  ", branch, BAD))
}

macro_rules! compose_rejects {
    ($($test:ident: $event:literal, $branch:literal;)*) => {$(
        #[test]
        fn $test() {
            let run = run(
                stringify!($test),
                &[("doc.md", &event_document($event, $branch))],
                "compose",
                "doc.md",
            );
            assert_rejected(&run, &format!("{}[0].{}[0].info.text", $event, $branch));
        }
    )*};
}

compose_rejects! {
    compose_rejects_a_malformed_mixed_span_in_an_unselected_start_then: "start", "then";
    compose_rejects_a_malformed_mixed_span_in_an_unselected_failure_else: "failure", "else";
}

#[test]
fn compose_rejects_a_malformed_mixed_span_in_an_unselected_loop_gate_branch() {
    let document = format!(
        "---\npick: other\nloop:\n  while: \"false\"\n  gate:\n{}---\nBody.\n",
        conditional("    ", "else", BAD)
    );
    let run = run("mixed-span-loop-gate", &[("doc.md", &document)], "compose", "doc.md");
    assert_rejected(&run, "loop.gate[0].else[0].info.text");
}

#[test]
fn inline_compose_rejects_a_malformed_mixed_span_before_launch() {
    let document = format!(
        "---\npick: other\nprompt: \"Summarize the body.\"\nstart:\n{}---\nBody.\n",
        conditional("  ", "then", BAD)
    );
    let run = run("mixed-span-inline", &[("doc.md", &document)], "inline-compose", "doc.md");
    assert_rejected(&run, "start[0].then[0].info.text");
}

#[test]
fn sequence_rejects_a_malformed_mixed_span_in_its_own_stack_before_any_step() {
    let sequence = format!(
        "---\npick: other\nstart:\n{}sequence:\n  - name: only\n    prompt: \"./step.md\"\n---\n",
        conditional("  ", "else", BAD)
    );
    let run = run(
        "mixed-span-sequence",
        &[("sequence.md", &sequence), ("step.md", "---\ntitle: step\n---\nstep body\n")],
        "sequence",
        "sequence.md",
    );
    assert_rejected(&run, "start[0].else[0].info.text");
}

macro_rules! task_stack_rejects {
    ($($test:ident: $stage:literal;)*) => {$(
        #[test]
        fn $test() {
            // Validate the authored prompt lifecycle through an external task.
            let task = "kind: task\nprompt: ./step.md\n";
            let prompt = format!("---\ntitle: step\n{}:\n{}---\nstep body\n", $stage, conditional("  ", "then", BAD));
            let run = run(
                stringify!($test),
                &[
                    ("sequence.md", "---\nsequence:\n  - name: only\n    task: ./task.yaml\n---\n"),
                    ("task.yaml", task),
                    ("step.md", &prompt),
                ],
                "sequence",
                "sequence.md",
            );
            // A failed step reports one truncated line, so it names the action;
            // the library table asserts the operand path for task stacks.
            assert_rejected(&run, &format!("{}[0].then[0].info", $stage));
        }
    )*};
}

task_stack_rejects! {
    sequence_rejects_a_malformed_mixed_span_in_prompt_start_before_the_task_launches: "start";
    sequence_rejects_a_malformed_mixed_span_in_prompt_finalize_before_the_task_launches: "finalize";
}

macro_rules! compose_accepts {
    ($($test:ident: $text:literal;)*) => {$(
        #[test]
        fn $test() {
            let document = format!("---\npick: other\nstart:\n{}---\nBody.\n", conditional("  ", "then", $text));
            let run = run(stringify!($test), &[("doc.md", &document)], "compose", "doc.md");
            assert_eq!(run.code, Some(0), "{}", run.output);
            assert_eq!(run.launches, 1, "{}", run.output);
            assert!(run.collapsed().contains("selected branch"), "{}", run.output);
        }
    )*};
}

compose_accepts! {
    compose_accepts_valid_mixed_text_in_an_unselected_branch: "status {{ fm.title }}";
    compose_accepts_an_evaluation_only_failure_in_an_unselected_branch_without_evaluating_it: "status {{ no_such_fn(fm.missing.deep) }}";
}
