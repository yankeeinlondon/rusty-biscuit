//! Level-1 process coverage for where an inline-compose sequence step writes
//! its closure (review finding R1-F8 of `2026-11-22-better-errors-and-dry-run`).
//!
//! A sequence document with a document-level `prompt:` runs every body step as
//! inline-compose: the agent edits the source and the closure restores
//! `prompt`, stamps `hash` and `last_updated`, and writes once. The write must
//! keep the source's own format. A YAML `kind: sequence` file stays a YAML
//! mapping, so the next step's reread of it still loads.
//!
//! The provider is the compiled `claudine-fake-goose` in plan mode: each
//! launch records its prompt as `launch-<n>.txt`, and `FAKE_FIRST_REWRITE_*` /
//! `FAKE_SECOND_REWRITE_*` make launch 1 or 2 overwrite the source with a
//! prepared edit, as an agent editing an authored field would.

use crate::common;

use common::{CliProcessFixture, strip_ansi, write};
use std::fs;
use std::path::PathBuf;

const EXE: &str = std::env::consts::EXE_SUFFIX;

struct Run {
    fixture: CliProcessFixture,
    env: Vec<(String, PathBuf)>,
}

struct Outcome {
    code: Option<i32>,
    output: String,
    launches: usize,
}

impl Run {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.seed_user_config();
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            fixture.bin_dir().join(format!("goose{EXE}")),
        )
        .expect("the fake provider is built with `test-fixtures`");
        Self { fixture, env: Vec::new() }
    }

    fn source(&self, name: &str, content: &str) -> &Self {
        write(&self.fixture.cwd().join(name), content);
        self
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.fixture.cwd().join(name)).unwrap()
    }

    /// Launch `n` (1 or 2) overwrites `document` with `bytes`.
    fn launch_rewrites(mut self, n: u8, document: &str, bytes: &str) -> Self {
        let label = if n == 1 { "FIRST" } else { "SECOND" };
        let content = self.fixture.workspace_path().join(format!("rewrite-{n}"));
        fs::write(&content, bytes).unwrap();
        self.env.push((format!("FAKE_{label}_REWRITE_PATH"), self.fixture.cwd().join(document)));
        self.env.push((format!("FAKE_{label}_REWRITE_FROM"), content));
        self
    }

    fn run(&self, args: &[&str]) -> Outcome {
        let launches = self.fixture.workspace_path().join("launches");
        let mut command = self.fixture.command_std();
        command
            .args(args)
            .env("COLUMNS", "4000")
            .env("FAKE_DIR", &launches)
            .env("FAKE_PLAN", "ok");
        for (key, value) in &self.env {
            command.env(key, value);
        }
        let output = command.output().expect("claudine runs");
        Outcome {
            code: output.status.code(),
            output: strip_ansi(&format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )),
            launches: (1..).take_while(|n| launches.join(format!("launch-{n}.txt")).is_file()).count(),
        }
    }
}

impl Run {
    /// The mapping the YAML file `name` parses to.
    fn yaml_root(&self, name: &str) -> serde_json::Map<String, serde_json::Value> {
        let text = self.read(name);
        let value = biscuit_file::Yaml::new(self.fixture.cwd().join(name))
            .and_then(|yaml| yaml.as_json())
            .unwrap_or_else(|error| panic!("the source must stay one YAML document ({error}):\n{text}"));
        value.as_object().cloned().expect("a mapping root")
    }
}

const YAML_SEQUENCE: &str = "kind: sequence\nprompt: \"Step {{ seq.name }}.\"\nstatus: draft\nsequence:\n  - first\n  - second\n";

fn yaml_edited(status: &str) -> String {
    YAML_SEQUENCE.replace("status: draft", &format!("status: {status}"))
}

#[test]
fn a_yaml_body_step_edit_is_kept_and_the_source_stays_yaml() {
    let run = Run::new("seq-inline-yaml-one")
        .launch_rewrites(1, "seq.yaml", &yaml_edited("first-done"))
        .launch_rewrites(2, "seq.yaml", &yaml_edited("second-done"));
    run.source("seq.yaml", YAML_SEQUENCE);

    let outcome = run.run(&["sequence", "seq.yaml", "--goose", "--yolo"]);

    let text = run.read("seq.yaml");
    assert!(!text.starts_with("---"), "no Markdown fence is published:\n{text}");
    let root = run.yaml_root("seq.yaml");
    assert_eq!(root["status"], "second-done", "the agent's edits are kept");
    assert_eq!(root["prompt"], "Step {{ seq.name }}.");
    assert!(root.contains_key("hash") && root.contains_key("last_updated"), "stamped in YAML:\n{text}");
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
}

#[test]
fn a_yaml_body_step_rereads_the_edited_source_for_the_next_step() {
    let run = Run::new("seq-inline-yaml-two")
        .launch_rewrites(1, "seq.yaml", &yaml_edited("first-done"))
        .launch_rewrites(2, "seq.yaml", &yaml_edited("second-done"));
    run.source("seq.yaml", YAML_SEQUENCE);

    let outcome = run.run(&["sequence", "seq.yaml", "--goose", "--yolo"]);

    assert_eq!(outcome.launches, 2, "task two launches:\n{}", outcome.output);
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
}

#[test]
fn a_yaml_body_step_restores_an_edited_prompt_in_yaml() {
    let edited = yaml_edited("first-done").replace("Step {{ seq.name }}.", "Changed.");
    let run = Run::new("seq-inline-yaml-prompt")
        .launch_rewrites(1, "seq.yaml", &edited)
        .launch_rewrites(2, "seq.yaml", &yaml_edited("second-done"));
    run.source("seq.yaml", YAML_SEQUENCE);

    let outcome = run.run(&["sequence", "seq.yaml", "--goose", "--yolo"]);

    let root = run.yaml_root("seq.yaml");
    assert_eq!(root["prompt"], "Step {{ seq.name }}.", "the owned prompt is restored");
    assert_eq!(outcome.launches, 2, "{}", outcome.output);
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
}

#[test]
fn a_yaml_body_step_that_changes_nothing_is_refused_and_left_as_written() {
    let run = Run::new("seq-inline-yaml-unchanged");
    run.source("seq.yaml", YAML_SEQUENCE);

    let outcome = run.run(&["sequence", "seq.yaml", "--goose", "--yolo"]);

    assert_eq!(run.read("seq.yaml"), YAML_SEQUENCE, "nothing is stamped or written");
    assert_eq!(outcome.launches, 1, "{}", outcome.output);
    assert_ne!(outcome.code, Some(0), "{}", outcome.output);
}

const MARKDOWN_SEQUENCE: &str =
    "---\nprompt: \"Step {{ seq.name }}.\"\nsequence:\n  - first\n  - second\n---\nOriginal body.\n";

#[test]
fn a_markdown_body_step_still_writes_markdown_and_reaches_the_next_step() {
    let run = Run::new("seq-inline-markdown")
        .launch_rewrites(1, "seq.md", &MARKDOWN_SEQUENCE.replace("Original body.", "First body."))
        .launch_rewrites(2, "seq.md", &MARKDOWN_SEQUENCE.replace("Original body.", "Second body."));
    run.source("seq.md", MARKDOWN_SEQUENCE);

    let outcome = run.run(&["sequence", "seq.md", "--goose", "--yolo"]);

    let text = run.read("seq.md");
    assert!(text.starts_with("---\n") && text.ends_with("Second body.\n"), "got:\n{text}");
    assert!(text.contains("\nhash: ") && text.contains("\nlast_updated: "), "got:\n{text}");
    assert_eq!(outcome.launches, 2, "{}", outcome.output);
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
}

#[test]
fn a_yaml_sequence_of_markdown_prompts_leaves_the_yaml_root_untouched() {
    let source = "kind: sequence\nsequence:\n  - name: first\n    prompt: ./task.md\n  - name: second\n    prompt: ./task.md\n";
    let run = Run::new("seq-yaml-prompts");
    run.source("seq.yaml", source).source("task.md", "Do {{ seq.name }}.\n");

    let outcome = run.run(&["sequence", "seq.yaml", "--goose", "--yolo"]);

    assert_eq!(run.read("seq.yaml"), source, "the YAML root is never published");
    assert_eq!(outcome.launches, 2, "{}", outcome.output);
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
}

#[test]
fn a_dry_run_of_a_yaml_inline_sequence_writes_nothing() {
    let run = Run::new("seq-inline-yaml-dry");
    run.source("seq.yaml", YAML_SEQUENCE);

    let outcome = run.run(&["sequence", "seq.yaml", "--goose", "--dry-run"]);

    assert_eq!(run.read("seq.yaml"), YAML_SEQUENCE);
    assert_eq!(outcome.launches, 0, "{}", outcome.output);
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert!(outcome.output.contains("Step first."), "previews:\n{}", outcome.output);
}
