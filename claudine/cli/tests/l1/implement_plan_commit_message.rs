//! The frozen `_implement/implement-plan.md` prompt, composed through
//! `claudine compose`, with its guarded `git commit -m "{{commit_message}}"`.
//!
//! The prompt as frozen declares `commit_message: ""`. Each cell stages it with
//! that line kept, removed (so `commit_message` is absent unless the run
//! passes it), or replaced by `commit_message: null`. With dirty files, an
//! empty, absent, or null message takes the `git add` + `just commit` route
//! and a passed message reaches `git commit -m` as one argument; pre-flight
//! refuses the guarded command when only its empty-default bytes are approved.
//! With a clean repository, neither commit route runs.
//!
//! `git` and `just` are the `claudine-argv-recorder` fixture copied into the
//! fixture `bin`, so no test commits anything and every program the prompt
//! started is in the recording. Claudine reads `ctx.dirty_files` and
//! `ctx.repo_root` in process, not through `git`, so any `git` call of its own
//! would also appear in the recording and fail the exact comparisons below.

use crate::common::prompt_staging::stage_frozen_prompts;
use crate::common::{CliProcessFixture, strip_ansi, write};
use std::fs;
use std::path::{Path, PathBuf};
use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};

const PROMPT: &str = "prompts/_implement/implement-plan.md";
const DEFAULT_LINE: &str = "\ncommit_message: \"\"\n";

/// Every lifecycle command the prompt's `success` stack discovers.
const RULES: &str = "exact git add ..\ntemplate git add {{ string }}\nexact just commit\ntemplate git commit -m {{ string }}\n";

/// What the staged prompt declares for `commit_message`.
#[derive(Clone, Copy)]
enum Declared {
    /// `commit_message: ""`, as frozen.
    Empty,
    /// No declaration.
    Absent,
    /// `commit_message: null`.
    Null,
}

/// How the fixture repository's working tree looks to Claudine.
#[derive(Clone, Copy, PartialEq)]
enum Tree {
    /// Every staged file is untracked, so `ctx.dirty_files` is not empty.
    Dirty,
    /// `.git/info/exclude` ignores every file, so `ctx.dirty_files` is empty.
    Clean,
}

struct ImplementPlan {
    fixture: CliProcessFixture,
    records: PathBuf,
    launches: PathBuf,
}

struct Outcome {
    success: bool,
    /// stdout and stderr with ANSI, box gutters, and wrapping removed.
    output: String,
}

impl ImplementPlan {
    /// A repository holding the staged prompt, a one-phase plan with its spec,
    /// and `rules` as its shell whitelist.
    fn new(name: &str, declared: Declared, tree: Tree, rules: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.initialize_repository();
        let suffix = std::env::consts::EXE_SUFFIX;
        for program in ["git", "just"] {
            fs::copy(
                biscuit_test_harness::bin_exe!("claudine-argv-recorder"),
                fixture.bin_dir().join(format!("{program}{suffix}")),
            )
            .expect("the recorder fixture binary is built with `test-fixtures`");
        }
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            fixture.bin_dir().join(format!("goose{suffix}")),
        )
        .expect("the fake provider is built with `test-fixtures`");

        stage_frozen_prompts(
            &fixture.cwd().join("prompts"),
            &["_implement/implement-plan.md"],
        );
        let replacement = match declared {
            Declared::Empty => None,
            Declared::Absent => Some("\n"),
            Declared::Null => Some("\ncommit_message: null\n"),
        };
        if let Some(replacement) = replacement {
            let staged = fixture.cwd().join(PROMPT);
            let prompt = fs::read_to_string(&staged).unwrap();
            assert_eq!(
                prompt.matches(DEFAULT_LINE).count(),
                1,
                "the frozen prompt declares the default once"
            );
            fs::write(&staged, prompt.replacen(DEFAULT_LINE, replacement, 1)).unwrap();
        }
        if tree == Tree::Clean {
            // Ignoring everything keeps the tree clean without a real commit.
            write(&fixture.cwd().join(".git/info/exclude"), "*\n");
        }
        write(
            &fixture.cwd().join("work/spec.md"),
            "---\nimplemented: false\n---\n# Spec\n",
        );
        write(
            &fixture.cwd().join("work/plan.md"),
            "---\ntotal_phases: 1\n---\n# Plan\n",
        );
        write(&fixture.cwd().join(".darkmatter-shell-whitelist"), rules);

        let records = fixture.workspace_path().join("argv.bin");
        let launches = fixture.workspace_path().join("launches.txt");
        Self {
            fixture,
            records,
            launches,
        }
    }

    /// `claudine compose <prompt> spec=work/spec.md --goose <setters>`.
    fn run(&self, setters: &[&str]) -> Outcome {
        let output = self
            .fixture
            .command()
            .env(ARGV_RECORDER_FILE, &self.records)
            .env("PROVIDER_MARKER", &self.launches)
            .args(["compose", PROMPT, "spec=work/spec.md", "--goose"])
            .args(setters)
            .output()
            .expect("claudine runs");
        assert!(
            !self.fixture.audio_spool().exists(),
            "the prompt's `say` must publish no audio"
        );
        let text = strip_ansi(&format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
        Outcome {
            success: output.status.success(),
            output: text
                .replace(['┃', '│'], " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// Every argument vector `git` or `just` received, in order.
    fn runs(&self) -> Vec<Vec<String>> {
        if !self.records.exists() {
            return Vec::new();
        }
        read_argv_records(&self.records).expect("the recorder writes whole records")
    }

    fn provider_launches(&self) -> usize {
        fs::read_to_string(&self.launches).map_or(0, |log| log.lines().count())
    }

    /// The `!commit_message` branch: `git add {{ctx.repo_root}}`, then
    /// `just commit`, and nothing from the guarded commit branch.
    fn assert_took_the_just_commit_branch(&self, outcome: &Outcome) {
        let runs = self.runs();
        assert_eq!(runs.len(), 2, "{runs:?}\n{}", outcome.output);
        assert_eq!(runs[0].len(), 2, "{runs:?}");
        assert_eq!(runs[0][0], "add", "{runs:?}");
        assert_same_directory(Path::new(&runs[0][1]), self.fixture.cwd());
        assert_eq!(
            runs[1],
            ["commit"],
            "`just commit`, and no `git commit -m`: {runs:?}"
        );
    }

    fn assert_no_commit(&self) {
        let heads = self.fixture.cwd().join(".git/refs/heads");
        assert_eq!(
            fs::read_dir(heads).map_or(0, Iterator::count),
            0,
            "no real commit was made"
        );
    }
}

fn assert_same_directory(actual: &Path, expected: &Path) {
    assert_eq!(
        actual.canonicalize().unwrap(),
        expected.canonicalize().unwrap()
    );
}

/// Control: the prompt as frozen. Its `commit_message: ""` default is present
/// and falsy, so the unguarded `just commit` branch runs.
#[test]
fn the_frozen_prompt_with_its_empty_default_takes_the_just_commit_branch() {
    let plan = ImplementPlan::new(
        "implement-plan-default",
        Declared::Empty,
        Tree::Dirty,
        RULES,
    );

    let outcome = plan.run(&[]);

    assert!(outcome.success, "{}", outcome.output);
    assert_eq!(plan.provider_launches(), 1, "{}", outcome.output);
    plan.assert_took_the_just_commit_branch(&outcome);
    plan.assert_no_commit();
}

/// AC 1: without the default, `commit_message` is absent. The guarded
/// command passes pre-flight by its approved shape, and its false guard
/// launches nothing.
#[test]
fn an_absent_commit_message_passes_pre_flight_and_its_false_guard_launches_nothing() {
    let plan = ImplementPlan::new(
        "implement-plan-absent",
        Declared::Absent,
        Tree::Dirty,
        RULES,
    );

    let outcome = plan.run(&[]);

    assert!(outcome.success, "{}", outcome.output);
    assert_eq!(plan.provider_launches(), 1, "{}", outcome.output);
    plan.assert_took_the_just_commit_branch(&outcome);
    plan.assert_no_commit();
}

/// An absent value has no bytes, so the guarded command is approvable only
/// by its shape. The `exact` rule for the empty default's bytes approves the
/// frozen prompt, but approves nothing here, and the run stops at pre-flight
/// before the provider or any command starts.
#[test]
fn an_absent_commit_message_without_a_template_rule_is_refused_at_pre_flight() {
    const BYTES_ONLY: &str =
        "exact git add ..\ntemplate git add {{ string }}\nexact just commit\nexact git commit -m\n";
    let plan = ImplementPlan::new(
        "implement-plan-unapproved",
        Declared::Absent,
        Tree::Dirty,
        BYTES_ONLY,
    );

    let outcome = plan.run(&[]);

    assert!(!outcome.success, "{}", outcome.output);
    assert!(
        outcome.output.contains("git commit -m"),
        "{}",
        outcome.output
    );
    assert!(
        outcome.output.contains("requires approval"),
        "{}",
        outcome.output
    );
    assert!(
        !outcome.output.contains("missing"),
        "not a missing-value refusal: {}",
        outcome.output
    );
    assert_eq!(plan.provider_launches(), 0, "{}", outcome.output);
    assert!(plan.runs().is_empty(), "{:?}", plan.runs());
}

/// AC 1: a passed message selects the guarded branch, and `git` receives it
/// as exactly one argument, spaces and shell characters included.
#[test]
fn a_passed_commit_message_reaches_git_as_one_argument() {
    const MESSAGE: &str = "msg with spaces; and $chars";
    let plan = ImplementPlan::new(
        "implement-plan-selected",
        Declared::Absent,
        Tree::Dirty,
        RULES,
    );

    let outcome = plan.run(&[&format!("commit_message={MESSAGE}")]);

    assert!(outcome.success, "{}", outcome.output);
    assert_eq!(plan.provider_launches(), 1, "{}", outcome.output);
    assert_eq!(
        plan.runs(),
        [vec!["add", ".."], vec!["commit", "-m", MESSAGE]],
        "{}",
        outcome.output
    );
    plan.assert_no_commit();
}

/// An explicit `null` default is falsy, so it takes the `just commit` branch
/// like the empty default and the absent value.
#[test]
fn a_null_commit_message_takes_the_just_commit_branch() {
    let plan = ImplementPlan::new("implement-plan-null", Declared::Null, Tree::Dirty, RULES);

    let outcome = plan.run(&[]);

    assert!(outcome.success, "{}", outcome.output);
    assert_eq!(plan.provider_launches(), 1, "{}", outcome.output);
    plan.assert_took_the_just_commit_branch(&outcome);
    plan.assert_no_commit();
}

/// With no dirty files only the `!ctx.dirty_files` branch runs, so no `git`
/// or `just` starts.
fn assert_a_clean_tree_runs_neither_commit_route(name: &str, setters: &[&str]) {
    let plan = ImplementPlan::new(name, Declared::Empty, Tree::Clean, RULES);

    let outcome = plan.run(setters);

    assert!(outcome.success, "{}", outcome.output);
    assert_eq!(plan.provider_launches(), 1, "{}", outcome.output);
    assert!(plan.runs().is_empty(), "{:?}", plan.runs());
    assert!(
        outcome.output.contains("made no file changes"),
        "{}",
        outcome.output
    );
    plan.assert_no_commit();
}

#[test]
fn a_clean_tree_with_the_empty_default_runs_neither_commit_route() {
    assert_a_clean_tree_runs_neither_commit_route("implement-plan-clean-default", &[]);
}

/// A passed message would select the supplied-message route on a dirty tree.
#[test]
fn a_clean_tree_with_a_passed_commit_message_runs_neither_commit_route() {
    assert_a_clean_tree_runs_neither_commit_route(
        "implement-plan-clean-passed",
        &["commit_message=msg"],
    );
}
