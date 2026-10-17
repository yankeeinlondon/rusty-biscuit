//! Rehearsal of the `just fix` review/fix loop (`just/review.just`) against
//! stub providers in a disposable repository.
//!
//! The real recipe runs the real `claudine`, which composes the shipped
//! `feature-review.md` and `implement-suggestions.md` and launches whatever
//! `codex` (reviews) and `claude` (repairs) it finds on `PATH`. Both are stubs
//! that re-execute this test binary in a helper mode: it recognizes the stage
//! by the prompt's heading, writes the review file or marks it repaired the
//! way an agent following the prompt would, and appends the stage to a trail
//! the assertions read. Each prompt is saved, numbered by its place in the
//! trail, so a test can assert which branch of a prompt Claudine composed.

use crate::common;

use biscuit_file::serde_yaml_ng::{Mapping, Value};
use common::prompt_staging::{stage_shipped_prompts, workspace_root};
use common::{CliProcessFixture, write, write_executable};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A positional argument: libtest reads it as a name filter, which under
/// `--exact` matches nothing, so the harness ignores it.
const HELPER_ARG: &str = "review-fix-stub=";

const PROMPTS: &[&str] = &["prompts/_implement/implement-suggestions.md", "prompts/_reviews/feature-review.md"];

/// What one review stage writes.
#[derive(Clone, Serialize, Deserialize)]
struct Review {
    ready: bool,
    findings: Vec<String>,
    observations: Vec<String>,
}

impl Review {
    fn ready(findings: &[&str], observations: &[&str]) -> Self {
        Self { ready: true, findings: strings(findings), observations: strings(observations) }
    }

    fn not_ready(findings: &[&str]) -> Self {
        Self { ready: false, findings: strings(findings), observations: Vec::new() }
    }
}

#[derive(Serialize, Deserialize)]
struct Scenario {
    spec: PathBuf,
    /// One entry per review stage, in launch order.
    reviews: Vec<Review>,
    trail: PathBuf,
    /// Where each prompt is saved, as `{trail position}-{stage}.md`.
    prompts: PathBuf,
}

// ── Stub provider ──────────────────────────────────────────────────────────

/// What a re-executed copy of this binary does instead of testing.
#[test]
fn stub_entrypoint() {
    let Some(scenario) = std::env::args().find_map(|arg| arg.strip_prefix(HELPER_ARG).map(PathBuf::from)) else {
        return;
    };
    let scenario: Scenario =
        serde_json::from_str(&std::fs::read_to_string(scenario).expect("read scenario")).expect("parse scenario");
    let mut prompt = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut prompt).expect("read prompt");
    let stage = if prompt.lines().any(|line| line.starts_with("# Review of ")) {
        "review"
    } else if prompt.lines().any(|line| line.starts_with("# Implement Review Suggestions for ")) {
        "repair"
    } else {
        "unknown"
    };
    let trail = std::fs::read_to_string(&scenario.trail).unwrap_or_default();
    let position = trail.lines().count() + 1;
    std::fs::create_dir_all(&scenario.prompts).expect("create prompt directory");
    std::fs::write(scenario.prompts.join(format!("{position:02}-{stage}.md")), &prompt).expect("save prompt");
    std::fs::write(&scenario.trail, format!("{trail}{stage}\n")).expect("append trail");

    let iterations = spec_iterations(&scenario.spec);
    match stage {
        "review" => {
            let index = trail.lines().filter(|line| *line == "review").count();
            let review = scenario.reviews.get(index).expect("the scenario scripts every review stage").clone();
            let number = iterations + 1;
            write_review(&review_path(&scenario.spec, number), &review);
            write_spec(&scenario.spec, number, review.ready);
        }
        "repair" => set_implemented(&review_path(&scenario.spec, iterations)),
        other => panic!("the stub did not recognize the stage it was launched for: {other}"),
    }
}

// ── Fixture ────────────────────────────────────────────────────────────────

struct Rehearsal {
    fixture: CliProcessFixture,
    scenario: Scenario,
    just: PathBuf,
}

struct Outcome {
    code: Option<i32>,
    output: String,
    trail: Vec<String>,
}

impl Rehearsal {
    fn new(name: &str, reviews: Vec<Review>) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.initialize_repository();
        fixture.seed_user_config();
        let repo = fixture.cwd().to_path_buf();
        let repository = workspace_root();
        stage_shipped_prompts(&repository, &repo.join("prompts"), PROMPTS);
        // The recipe calls itself back (`just _link`), so it needs a justfile
        // in the launch directory that brings in the shipped one.
        write(
            &repo.join("justfile"),
            &format!("import \"{}\"\n", portable(&repository.join("just/review.just"))),
        );

        let spec = repo.join("area/features/2026-01-01-demo/spec.md");
        write_spec(&spec, 0, false);
        let scenario = Scenario {
            spec,
            reviews,
            trail: fixture.workspace_path().join("trail.txt"),
            prompts: fixture.workspace_path().join("prompts"),
        };
        let just = which::which("just").expect("the review/fix rehearsal needs just");
        let rehearsal = Self { fixture, scenario, just };
        rehearsal.install_command_stubs();
        rehearsal
    }

    /// Puts the tools the recipe runs into the fixture `bin`, ahead of the
    /// minimal system `PATH`: the real `just`, `md`, `bash`, `git`, and the
    /// `claudine` under test, plus the two provider stubs.
    fn install_command_stubs(&self) {
        let bin = self.fixture.bin_dir();
        let claudine = PathBuf::from(self.fixture.command_std().get_program());
        // `md` is the recipe's frontmatter reader; `bash` because the recipe's
        // `/usr/bin/env bash` must not find the host's oldest bash first.
        for (name, program) in [
            ("just", self.just.clone()),
            ("md", which::which("md").expect("the review/fix rehearsal needs md")),
            ("bash", which::which("bash").expect("the review/fix rehearsal needs bash")),
            ("git", which::which("git").expect("the review/fix rehearsal needs git")),
            ("claudine", claudine),
        ] {
            std::os::unix::fs::symlink(&program, bin.join(name)).expect("link a tool into the fixture bin");
        }

        let exe = portable(&std::env::current_exe().expect("current test executable"));
        let module = module_path!()
            .split_once("::")
            .map(|(_, rest)| rest)
            .expect("module path has a crate segment");
        let scenario = portable(&self.fixture.workspace_path().join("scenario.json"));
        let log = portable(&self.fixture.workspace_path().join("stub.log"));
        let helper = format!("'{exe}' --exact {module}::stub_entrypoint --nocapture '{HELPER_ARG}{scenario}' >/dev/null 2>>'{log}' || exit 1");

        let init = r#"{"type":"system","subtype":"init","session_id":"rehearsal","model":"stub"}"#;
        let result = r#"{"type":"result","subtype":"success","result":"done","session_id":"rehearsal","is_error":false}"#;
        write_executable(
            &bin.join("claude"),
            &format!("#!/bin/sh\n{helper}\nprintf '%s\\n' '{init}'\nprintf '%s\\n' '{result}'\n"),
        );
        // Codex's own event stream, and its last-message file when asked for.
        // Refusing `app-server` makes Claudine fall back to `codex exec`, whose
        // prompt arrives on stdin; the app-server session would hold stdin open.
        write_executable(
            &bin.join("codex"),
            &format!(
                "#!/bin/sh\ncase \"$1\" in --version|-V|-v|version|models) exit 0;; app-server) exit 2;; esac\n\
                 last=\"\"\nprev=\"\"\nfor arg in \"$@\"; do\n  [ \"$prev\" = \"--output-last-message\" ] && last=\"$arg\"\n  prev=\"$arg\"\ndone\n\
                 {helper}\n\
                 printf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"rehearsal\"}}'\n\
                 printf '%s\\n' '{{\"type\":\"turn.started\"}}'\n\
                 printf '%s\\n' '{{\"type\":\"item.completed\",\"item\":{{\"id\":\"msg-1\",\"type\":\"agent_message\",\"text\":\"done\"}}}}'\n\
                 printf '%s\\n' '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":1,\"output_tokens\":1}}}}'\n\
                 [ -n \"$last\" ] && printf '%s\\n' 'done' > \"$last\"\nexit 0\n"
            ),
        );
    }

    fn run(&self) -> Outcome {
        write(
            &self.fixture.workspace_path().join("scenario.json"),
            &serde_json::to_string(&self.scenario).expect("serialize scenario"),
        );
        let spec = self.scenario.spec.strip_prefix(self.fixture.cwd()).expect("spec inside the repository");
        let mut command = Command::new(&self.just);
        command.current_dir(self.fixture.cwd()).args(["fix", &portable(spec), "--use-defaults"]);
        // `claudine` is a grandchild here (just → bash → claudine), so the
        // fixture's environment policy is applied to the outermost process.
        self.fixture.command_builder().apply_policy_to(&mut command);
        let output = command.output().expect("just runs");
        assert!(
            !self.fixture.audio_spool().exists(),
            "a lifecycle sound escaped PLAYA_DRY_RUN into {}",
            self.fixture.audio_spool().display()
        );
        Outcome {
            code: output.status.code(),
            output: format!(
                "{}{}\n--- stub log ---\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                std::fs::read_to_string(self.fixture.workspace_path().join("stub.log")).unwrap_or_default()
            ),
            trail: std::fs::read_to_string(&self.scenario.trail)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect(),
        }
    }

    /// The prompt saved at `position` in the trail (1-based).
    fn prompt(&self, position: usize, stage: &str) -> String {
        std::fs::read_to_string(self.scenario.prompts.join(format!("{position:02}-{stage}.md"))).expect("read saved prompt")
    }

    fn review(&self, number: u64) -> Mapping {
        frontmatter(&review_path(&self.scenario.spec, number)).0
    }
}

// ── Files the stages read and write ────────────────────────────────────────

fn review_path(spec: &Path, number: u64) -> PathBuf {
    spec.parent().expect("spec directory").join(format!("review-{number}.md"))
}

fn frontmatter(path: &Path) -> (Mapping, String) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let rest = text.strip_prefix("---\n").expect("frontmatter opens the file");
    let (yaml, body) = rest.split_once("\n---\n").expect("frontmatter is closed");
    (biscuit_file::serde_yaml_ng::from_str(yaml).expect("frontmatter is YAML"), body.to_string())
}

fn write_frontmatter(path: &Path, map: &Mapping, body: &str) {
    let yaml = biscuit_file::serde_yaml_ng::to_string(map).expect("serialize frontmatter");
    write(path, &format!("---\n{yaml}---\n{body}"));
}

fn spec_iterations(spec: &Path) -> u64 {
    frontmatter(spec).0.get("review_iterations").and_then(Value::as_u64).unwrap_or(0)
}

fn write_spec(spec: &Path, iterations: u64, completed: bool) {
    let mut map = Mapping::new();
    map.insert("implemented".into(), true.into());
    map.insert("review_iterations".into(), iterations.into());
    map.insert("completed".into(), completed.into());
    write_frontmatter(spec, &map, "# Demo spec\n");
}

fn write_review(path: &Path, review: &Review) {
    let mut map = Mapping::new();
    map.insert("ready".into(), review.ready.into());
    map.insert("recurrence".into(), false.into());
    map.insert("implemented".into(), false.into());
    let list = |items: &[String]| Value::Sequence(items.iter().map(|item| item.as_str().into()).collect());
    map.insert("findings".into(), list(&review.findings));
    map.insert("observations".into(), list(&review.observations));
    write_frontmatter(path, &map, "# Review\n");
}

fn set_implemented(path: &Path) {
    let (mut map, body) = frontmatter(path);
    map.insert("implemented".into(), true.into());
    write_frontmatter(path, &map, &body);
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn portable(path: &Path) -> String {
    biscuit_file::to_portable_string(path)
}

// ── Routes ─────────────────────────────────────────────────────────────────

#[test]
fn a_ready_review_gets_one_follow_up_pass_and_no_further_review() {
    let rehearsal = Rehearsal::new("review-fix-follow-up", vec![Review::ready(&["R1-F1 (medium): absent cap reads as zero"], &[])]);

    let outcome = rehearsal.run();

    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert_eq!(outcome.trail, ["review", "repair"], "{}", outcome.output);
    let repair = rehearsal.prompt(2, "repair");
    assert!(repair.contains("### Follow-up pass"), "the repair is the follow-up pass:\n{repair}");
    assert!(!repair.contains("### Inventory before implementation"), "not a full repair:\n{repair}");
    assert_eq!(rehearsal.review(1).get("implemented"), Some(&Value::Bool(true)));

    let rerun = rehearsal.run();
    assert_eq!(rerun.code, Some(0), "{}", rerun.output);
    assert_eq!(rerun.trail, ["review", "repair"], "a finished follow-up pass launches nothing:\n{}", rerun.output);
    assert!(rerun.output.contains("nothing to do"), "{}", rerun.output);
}

#[test]
fn a_ready_review_with_nothing_open_launches_no_repair() {
    let rehearsal = Rehearsal::new("review-fix-nothing-open", vec![Review::ready(&[], &[])]);

    let outcome = rehearsal.run();

    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert_eq!(outcome.trail, ["review"], "{}", outcome.output);
}

#[test]
fn a_not_ready_review_is_repaired_and_the_ready_one_gets_its_follow_up_pass() {
    let rehearsal = Rehearsal::new(
        "review-fix-repair-then-ready",
        vec![
            Review::not_ready(&["R1-F1 (high): the cap is never applied"]),
            Review::ready(&["R2-F1 (low): a message names the wrong key"], &[]),
        ],
    );

    let outcome = rehearsal.run();

    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert_eq!(outcome.trail, ["review", "repair", "review", "repair"], "{}", outcome.output);
    let full_repair = rehearsal.prompt(2, "repair");
    assert!(full_repair.contains("### Inventory before implementation"), "a full repair:\n{full_repair}");
    let follow_up = rehearsal.prompt(4, "repair");
    assert!(follow_up.contains("### Follow-up pass"), "the follow-up pass:\n{follow_up}");
}

#[test]
fn a_rerun_on_a_ready_spec_runs_its_open_follow_up_pass() {
    let rehearsal = Rehearsal::new("review-fix-startup-follow-up", Vec::new());
    write_review(&review_path(&rehearsal.scenario.spec, 1), &Review::ready(&[], &["the old parser ignores tabs"]));
    write_spec(&rehearsal.scenario.spec, 1, true);

    let outcome = rehearsal.run();

    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert_eq!(outcome.trail, ["repair"], "only the follow-up pass, and no review:\n{}", outcome.output);
    assert!(rehearsal.prompt(1, "repair").contains("### Follow-up pass"));
}
