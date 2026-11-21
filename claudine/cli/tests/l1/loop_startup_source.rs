//! Level-1 process coverage for which read of a document a loop keeps
//! (fix `2026-11-20-loop-reread-context-drift`).
//!
//! A document loop keeps the root source that owned its first launch verdict:
//! the read after `initialize` on the staged route, the harness's read on the
//! delegated route, the caller-resolved source on the eager route. Every
//! later ordinary pass composes that source with fresh working-tree values,
//! so an edit to the root made during a pass is seen by the next run, a
//! retry or resume, or a new sequence task invocation, never by the next
//! ordinary pass. The lifecycle actions an event runs use the context and
//! frontmatter of the preparation that produced them.
//!
//! The provider is the compiled `claudine-fake-goose` in plan mode: each
//! launch records its prompt as `launch-<n>.txt` under a directory outside
//! the fixture repository, and `FAKE_FIRST_REWRITE_*` / `FAKE_SECOND_REWRITE_*`
//! let launch 1 or 2 rewrite a file. Edits made before or during `initialize` go through the
//! `claudine-fake-editor` installed as `code` (`edit` lifecycle action) or as
//! `rewrite-source` on `PATH` (a body `::shell` directive). Every effect is a
//! file write the test reads back; nothing waits on time.

use crate::common;

use common::{CliProcessFixture, strip_ansi, write};
use std::fs;
use std::path::{Path, PathBuf};
use test_toolkit::argv_recorder::read_argv_records;
use test_toolkit::fake_editor::{FAKE_EDITOR_DELETE, FAKE_EDITOR_RECORD, FAKE_EDITOR_WRITE};

const EXE: &str = std::env::consts::EXE_SUFFIX;

/// A committed fixture repository with the fake provider and fake editor
/// installed, and the files a run reports into.
struct LoopRun {
    fixture: CliProcessFixture,
    /// The fake editor, outside `PATH`, reached only through `EDITOR`.
    editor: PathBuf,
    env: Vec<(String, String)>,
}

struct Outcome {
    code: Option<i32>,
    output: String,
}

impl LoopRun {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.seed_user_config();
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-goose"),
            fixture.bin_dir().join(format!("goose{EXE}")),
        )
        .expect("the fake provider is built with `test-fixtures`");
        // No space in the path: `$EDITOR` is split on whitespace.
        let editors = fixture.workspace_path().join("editors");
        fs::create_dir_all(&editors).unwrap();
        let editor = editors.join(format!("code{EXE}"));
        fs::copy(biscuit_test_harness::bin_exe!("claudine-fake-editor"), &editor)
            .expect("the fake editor is built with `test-fixtures`");
        git(&fixture, &["init", "-q", "--initial-branch=main"]);
        write(&fixture.cwd().join("README.md"), "fixture\n");
        write(&fixture.cwd().join(".gitignore"), "events.log\n.claudine/\n");
        git(&fixture, &["add", "README.md", ".gitignore"]);
        git(&fixture, &["commit", "-q", "-m", "initial"]);
        Self { fixture, editor, env: Vec::new() }
    }

    fn cwd(&self) -> &Path {
        self.fixture.cwd()
    }

    fn with(mut self, key: &str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        self.env.push((key.to_string(), value.as_ref().to_string_lossy().into_owned()));
        self
    }

    /// Bytes the fake editor (as `code` or `rewrite-source`) writes into its
    /// target.
    fn editor_writes(self, bytes: &str) -> Self {
        let content = self.fixture.workspace_path().join("editor-content.md");
        fs::write(&content, bytes).unwrap();
        self.with(FAKE_EDITOR_WRITE, content)
    }

    /// Bytes launch 1 of the provider writes over `document`.
    fn first_launch_rewrites(self, document: &str, bytes: &str) -> Self {
        let content = self.fixture.workspace_path().join("first-launch-rewrite.md");
        fs::write(&content, bytes).unwrap();
        let target = self.cwd().join(document);
        self.with("FAKE_FIRST_REWRITE_PATH", target).with("FAKE_FIRST_REWRITE_FROM", content)
    }

    /// Bytes launch 2 of the provider writes over `file`.
    fn second_launch_rewrites(self, file: &str, bytes: &str) -> Self {
        let content = self.fixture.workspace_path().join("second-launch-rewrite.md");
        fs::write(&content, bytes).unwrap();
        let target = self.cwd().join(file);
        self.with("FAKE_SECOND_REWRITE_PATH", target).with("FAKE_SECOND_REWRITE_FROM", content)
    }

    /// Installs the fake editor on `PATH` as `rewrite-source`.
    fn install_rewrite_source(&self) {
        fs::copy(
            biscuit_test_harness::bin_exe!("claudine-fake-editor"),
            self.fixture.bin_dir().join(format!("rewrite-source{EXE}")),
        )
        .expect("the fake editor is built with `test-fixtures`");
    }

    /// Writes and commits `name`, so the working tree's untracked list is
    /// only what the run itself creates.
    fn committed(&self, name: &str, content: &str) {
        write(&self.cwd().join(name), content);
        git(&self.fixture, &["add", name]);
        git(&self.fixture, &["commit", "-q", "-m", name]);
    }

    /// Leaves `before.txt` untracked, so the working tree's untracked list
    /// observed before `initialize` is not empty.
    fn untracked_before_run(&self) {
        write(&self.cwd().join("before.txt"), "untracked before the run\n");
    }

    /// Records every fake editor invocation into `editor-record`.
    fn recording_editor(self) -> Self {
        let record = self.fixture.workspace_path().join("editor-record");
        self.with(FAKE_EDITOR_RECORD, record)
    }

    /// How many times the fake editor ran, under any name.
    fn editor_runs(&self) -> usize {
        let record = self.fixture.workspace_path().join("editor-record");
        if !record.exists() {
            return 0;
        }
        read_argv_records(&record).expect("the editor writes whole records").len()
    }

    fn launches_dir(&self, label: &str) -> PathBuf {
        self.fixture.workspace_path().join(format!("launches-{label}"))
    }

    /// Runs `claudine` with the provider recording into `launches-<label>`.
    fn run(&self, label: &str, plan: &str, args: &[&str]) -> Outcome {
        let mut command = self.fixture.command_std();
        command
            .args(args)
            .env("COLUMNS", "4000")
            .env("FAKE_DIR", self.launches_dir(label))
            .env("FAKE_PLAN", plan)
            .env("EDITOR", &self.editor)
            .env_remove("VISUAL")
            // A display session, so Linux admits the GUI editor.
            .env("DISPLAY", ":0")
            .env_remove("WAYLAND_DISPLAY");
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
        }
    }

    /// Runs with the provider rewrite variables removed, for a fresh run
    /// after the rewrite.
    fn fresh_run(&self, label: &str, args: &[&str]) -> Outcome {
        let mut command = self.fixture.command_std();
        command
            .args(args)
            .env("COLUMNS", "4000")
            .env("FAKE_DIR", self.launches_dir(label))
            .env("FAKE_PLAN", "ok");
        let output = command.output().expect("claudine runs");
        Outcome {
            code: output.status.code(),
            output: strip_ansi(&format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )),
        }
    }

    /// Every prompt the provider received under `label`, in launch order.
    fn prompts(&self, label: &str) -> Vec<String> {
        let dir = self.launches_dir(label);
        (1..)
            .map(|n| dir.join(format!("launch-{n}.txt")))
            .take_while(|path| path.is_file())
            .map(|path| fs::read_to_string(path).unwrap())
            .collect()
    }

    /// The lines lifecycle actions appended to `events.log`.
    fn events(&self) -> Vec<String> {
        fs::read_to_string(self.cwd().join("events.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
}

/// `git` in the fixture repository, with an identity and no signing so the
/// host's configuration can neither change nor block the fixture history.
fn git(fixture: &CliProcessFixture, args: &[&str]) {
    let output = common::helper_command("git")
        .arg("-C")
        .arg(fixture.cwd())
        .args(["-c", "user.name=Fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("run git in the fixture");
    assert!(output.status.success(), "git {args:?}: {output:?}");
}

/// The value of `name=` in a space-separated record line, up to the next `|`.
fn field<'a>(line: &'a str, name: &str) -> &'a str {
    let start = line.find(&format!("{name}=")).map(|at| at + name.len() + 1).unwrap_or(line.len());
    let rest = &line[start..];
    rest.split('|').next().unwrap_or_default().trim()
}

fn lines_with<'a>(lines: &'a [String], prefix: &str) -> Vec<&'a str> {
    lines.iter().map(String::as_str).filter(|line| line.starts_with(prefix)).collect()
}

/// The line of `text` that starts with `prefix`.
fn line_starting<'a>(text: &'a str, prefix: &str) -> &'a str {
    text.lines().find(|line| line.trim_start().starts_with(prefix)).map(str::trim_start).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Staged proxy target: the read after `initialize` owns the loop
// ---------------------------------------------------------------------------

/// The stabilized target: what `initialize`'s `edit` installs. It adds
/// `required_passes` (which the loop condition reads) and a `start` that
/// names `ctx.hostname`, a group the original root never named.
const STAGED_STABILIZED: &str = r#"---
$schema:
  caller_label: string(required)
required_passes: 2
initialize:
  - append_line: ["events.log", "target-initialize"]
start:
  - append_line: ["events.log", "start-v2 pass={{ loop.count }}| host={{ ctx.hostname }}| files={{ ctx.untracked_files }}|"]
loop:
  until: "loop.count >= required_passes"
  max: 3
---
STABILIZED-BODY pass={{ loop.count }}| caller={{ caller_label }}| proxy={{ proxy_label }}| host={{ ctx.hostname }}| files={{ ctx.untracked_files }}|
"#;

/// What the first provider launch writes over the target: a different body,
/// frontmatter, `start`/`failure`, schema, and a one-pass loop.
const STAGED_THIRD: &str = r#"---
$schema:
  third_label: string(required)
start:
  - append_line: ["events.log", "start-v3 label={{ third_label }}"]
failure:
  - append_line: ["events.log", "failure-v3"]
loop:
  until: "true"
  max: 3
---
THIRD-BODY label={{ third_label }}
"#;

#[test]
fn staged_proxy_loop_retains_its_first_validated_root() {
    let run = LoopRun::new("loop-startup-staged-proxy").editor_writes(STAGED_STABILIZED);
    let run = run.first_launch_rewrites("target.md", STAGED_THIRD);
    run.committed(
        "target.md",
        r#"---
$schema:
  caller_label: string(required)
initialize:
  - append_line: ["events.log", "target-initialize"]
  - ensure_file: "init-created.txt"
  - edit:
      file: "target.md"
loop:
  until: "loop.count >= required_passes"
  max: 3
---
ORIGINAL-BODY files={{ ctx.untracked_files }}
"#,
    );
    run.committed(
        "router.md",
        "---\ninitialize:\n  - proxy:\n      target: './target.md'\n      with:\n        proxy_label: proxy\n---\nROUTER-BODY\n",
    );
    run.untracked_before_run();

    let outcome = run.run("staged", "ok,ok,ok", &["compose", "--goose", "router.md", "caller_label=caller"]);

    let prompts = run.prompts("staged");
    let events = run.events();
    assert_eq!(outcome.code, Some(0), "{}\nevents: {events:?}", outcome.output);
    assert_eq!(prompts.len(), 2, "required_passes from the stabilized root ends the loop at 2: {events:?}");
    assert_eq!(
        events.iter().filter(|line| *line == "target-initialize").count(),
        1,
        "the target initializes once: {events:?}"
    );
    let starts = lines_with(&events, "start-v2 ");
    assert_eq!(starts.len(), 2, "both passes run the stabilized start: {events:?}");
    assert!(lines_with(&events, "start-v3").is_empty(), "the provider's rewrite is not a pass source: {events:?}");
    for (index, prompt) in prompts.iter().enumerate() {
        let pass = index + 1;
        let body = line_starting(prompt, "STABILIZED-BODY");
        assert!(!body.is_empty(), "pass {pass} composes the stabilized body:\n{prompt}");
        assert!(!prompt.contains("ORIGINAL-BODY") && !prompt.contains("THIRD-BODY"), "{prompt}");
        assert_eq!(field(body, "pass"), pass.to_string(), "{body}");
        assert_eq!(field(body, "caller"), "caller", "{body}");
        assert_eq!(field(body, "proxy"), "proxy", "{body}");
        let start = starts[index];
        assert_eq!(field(start, "start-v2 pass"), pass.to_string(), "{start}");
        let host = field(body, "host");
        assert!(!host.is_empty(), "the prompt renders the host: {body}");
        assert_eq!(field(start, "host"), host, "start renders the host composition used: {start} vs {body}");
        assert_eq!(field(start, "files"), field(body, "files"), "{start} vs {body}");
    }
    let first_files = field(line_starting(&prompts[0], "STABILIZED-BODY"), "files");
    assert!(first_files.contains("before.txt"), "{first_files}");
    assert!(
        !first_files.contains("init-created.txt"),
        "pass 1 keeps the file list observed before initialize: {first_files}"
    );
    let second_files = field(line_starting(&prompts[1], "STABILIZED-BODY"), "files");
    assert!(
        second_files.contains("init-created.txt"),
        "pass 2 observes the working tree pass 1 left: {second_files}"
    );
    assert_eq!(fs::read_to_string(run.cwd().join("target.md")).unwrap(), STAGED_THIRD);

    // The provider's rewrite takes effect on a new run.
    let fresh = run.fresh_run("fresh", &["compose", "--goose", "target.md", "third_label=fresh"]);
    let fresh_prompts = run.prompts("fresh");
    assert_eq!(fresh.code, Some(0), "{}", fresh.output);
    assert_eq!(fresh_prompts.len(), 1, "{}", fresh.output);
    assert!(fresh_prompts[0].contains("THIRD-BODY label=fresh"), "{}", fresh_prompts[0]);
    assert!(run.events().contains(&"start-v3 label=fresh".to_string()), "{:?}", run.events());
}

// ---------------------------------------------------------------------------
// Delegated proxy target: the harness's read owns the loop
// ---------------------------------------------------------------------------

/// What the body directive installs, immediately before the harness's read.
const DELEGATED_HARNESS_READ: &str = r#"---
$schema:
  proxy_label: string(required)
start:
  - append_line: ["events.log", "start HARNESS-READ pass={{ loop.count }} proxy={{ proxy_label }}"]
loop:
  until: "loop.count >= 2"
  max: 3
---
HARNESS-READ pass={{ loop.count }} proxy={{ proxy_label }}
"#;

const DELEGATED_THIRD: &str = r#"---
start:
  - append_line: ["events.log", "start THIRD"]
loop:
  until: "loop.count >= 2"
  max: 3
---
THIRD-BODY
"#;

#[test]
fn delegated_proxy_loop_retains_the_harness_read() {
    let run = LoopRun::new("loop-startup-delegated-proxy")
        .editor_writes(DELEGATED_HARNESS_READ)
        .first_launch_rewrites("target.md", DELEGATED_THIRD)
        .recording_editor();
    run.install_rewrite_source();
    run.committed(
        "target.md",
        r#"---
start:
  - append_line: ["events.log", "start ORIGINAL pass={{ loop.count }}"]
loop:
  until: "loop.count >= 2"
  max: 3
---
ORIGINAL-BODY

::shell rewrite-source target.md
"#,
    );
    run.committed(
        "router.md",
        "---\ninitialize:\n  - proxy:\n      target: './target.md'\n      with:\n        proxy_label: proxy\n---\nROUTER-BODY\n",
    );

    let outcome = run.run("delegated", "ok,ok,ok", &["compose", "--goose", "--yolo", "router.md"]);

    let prompts = run.prompts("delegated");
    let events = run.events();
    assert_eq!(outcome.code, Some(0), "{}\nevents: {events:?}", outcome.output);
    assert_eq!(prompts.len(), 2, "{events:?}\n{}", outcome.output);
    for (index, prompt) in prompts.iter().enumerate() {
        let pass = index + 1;
        assert!(
            prompt.contains(&format!("HARNESS-READ pass={pass} proxy=proxy")),
            "pass {pass} composes the source the harness validated:\n{prompt}"
        );
        assert!(!prompt.contains("ORIGINAL-BODY") && !prompt.contains("THIRD-BODY"), "{prompt}");
    }
    assert_eq!(
        lines_with(&events, "start "),
        ["start HARNESS-READ pass=1 proxy=proxy", "start HARNESS-READ pass=2 proxy=proxy"],
        "{}",
        outcome.output
    );
    assert_eq!(
        run.editor_runs(),
        1,
        "only the first canonical body composition runs the original directive:\n{}",
        outcome.output
    );
    assert_eq!(fs::read_to_string(run.cwd().join("target.md")).unwrap(), DELEGATED_THIRD);
}

// ---------------------------------------------------------------------------
// Staged single run: the rewritten lifecycle uses the rewritten context
// ---------------------------------------------------------------------------

const SINGLE_ORIGINAL: &str = r#"---
initialize:
  - append_line: ["events.log", "initialize"]
  - ensure_file: "init-created.txt"
  - edit:
      file: "doc.md"
---
ORIGINAL-BODY files={{ ctx.untracked_files }}|
"#;

#[test]
fn staged_single_uses_the_rewritten_lifecycle_context() {
    let run = LoopRun::new("loop-startup-staged-single").editor_writes(
        r#"---
start:
  - append_line: ["events.log", "start-v2 branch={{ ctx.branch }}| files={{ ctx.untracked_files }}|"]
---
REWRITTEN-BODY branch={{ ctx.branch }}| files={{ ctx.untracked_files }}|
"#,
    );
    run.committed("doc.md", SINGLE_ORIGINAL);
    run.untracked_before_run();

    let outcome = run.run("single", "ok", &["compose", "--goose", "doc.md"]);

    let prompts = run.prompts("single");
    let events = run.events();
    assert_eq!(outcome.code, Some(0), "{}\nevents: {events:?}", outcome.output);
    assert_eq!(prompts.len(), 1, "{events:?}");
    let body = line_starting(&prompts[0], "REWRITTEN-BODY");
    assert!(!body.is_empty(), "the rewritten body is composed:\n{}", prompts[0]);
    let starts = lines_with(&events, "start-v2 ");
    assert_eq!(starts.len(), 1, "start fires once: {events:?}");
    assert_eq!(field(starts[0], "branch"), "main", "{}", starts[0]);
    assert_eq!(field(starts[0], "branch"), field(body, "branch"), "{} vs {body}", starts[0]);
    assert_eq!(field(starts[0], "files"), field(body, "files"), "{} vs {body}", starts[0]);
    assert!(field(starts[0], "files").contains("before.txt"), "{}", starts[0]);
    assert!(
        !field(starts[0], "files").contains("init-created.txt"),
        "the file-change observation made before initialize is kept: {}",
        starts[0]
    );
}

#[test]
fn staged_single_denied_audit_routes_through_the_rewritten_catch_context() {
    let run = LoopRun::new("loop-startup-staged-denied");
    let effect = run.fixture.workspace_path().join("catch-shell-effect.txt");
    let rewritten = format!(
        r#"---
start:
  - shell: "echo unauthorized"
blocked:
  - append_line: ["events.log", "blocked-v2 branch={{{{ ctx.branch }}}}|"]
  - shell: "touch {effect}"
finalize:
  - append_line: ["events.log", "finalize-v2 branch={{{{ ctx.branch }}}}|"]
---
REWRITTEN-BODY
"#,
        effect = effect.display().to_string().replace('\\', "/"),
    );
    let run = run.editor_writes(&rewritten);
    run.committed("doc.md", SINGLE_ORIGINAL);

    let outcome = run.run("denied", "ok", &["compose", "--goose", "doc.md"]);

    let events = run.events();
    assert_ne!(outcome.code, Some(0), "the unapproved start shell is refused:\n{}", outcome.output);
    assert!(run.prompts("denied").is_empty(), "no provider launches:\n{}", outcome.output);
    assert_eq!(
        events,
        ["initialize", "blocked-v2 branch=main|", "finalize-v2 branch=main|"],
        "the rewritten catch events render the rewritten context once each:\n{}",
        outcome.output
    );
    assert!(!effect.exists(), "an early catch shell never runs:\n{}", outcome.output);
}

// ---------------------------------------------------------------------------
// Eager direct loop: a later ordinary pass needs no root on disk
// ---------------------------------------------------------------------------

#[test]
fn ordinary_direct_pass_uses_retained_root_after_removal() {
    let run = LoopRun::new("loop-startup-removed-root").with(FAKE_EDITOR_DELETE, "1");
    run.committed(
        "root.md",
        r#"---
success:
  - when: "loop.count == 1"
    then:
      - edit:
          file: "root.md"
loop:
  until: "loop.count >= 2"
  max: 3
---
RETAINED-BODY pass={{ loop.count }}
"#,
    );

    let outcome = run.run("removed", "ok,ok,ok", &["compose", "--goose", "root.md"]);

    let prompts = run.prompts("removed");
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert!(!run.cwd().join("root.md").exists(), "pass 1's success removed the root");
    assert_eq!(prompts.len(), 2, "{}", outcome.output);
    assert!(prompts[0].contains("RETAINED-BODY pass=1"), "{}", prompts[0]);
    assert!(prompts[1].contains("RETAINED-BODY pass=2"), "{}", prompts[1]);

    let fresh = run.fresh_run("fresh", &["compose", "--goose", "root.md"]);
    assert_ne!(fresh.code, Some(0), "a new run reads the root and finds it gone:\n{}", fresh.output);
    assert!(run.prompts("fresh").is_empty(), "{}", fresh.output);
}

// ---------------------------------------------------------------------------
// Sequence: each prompt task invocation reads its own source
// ---------------------------------------------------------------------------

#[test]
fn sequence_task_reopens_root_between_invocations() {
    let run = LoopRun::new("loop-startup-sequence").first_launch_rewrites(
        "loop.md",
        "---\nloop:\n  until: \"loop.count >= 2\"\n  max: 3\n---\nSECOND-SOURCE pass={{ loop.count }}\n",
    );
    run.committed(
        "loop.md",
        "---\nloop:\n  until: \"loop.count >= 2\"\n  max: 3\n---\nFIRST-SOURCE pass={{ loop.count }}\n",
    );
    run.committed(
        "seq.md",
        "---\nsequence:\n  - name: one\n    prompt: loop.md\n  - name: two\n    prompt: loop.md\n---\nBody.\n",
    );

    let outcome = run.run("sequence", "ok,ok,ok,ok,ok", &["sequence", "--goose", "seq.md"]);

    let prompts = run.prompts("sequence");
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert_eq!(prompts.len(), 4, "{}", outcome.output);
    let expected = [
        "FIRST-SOURCE pass=1",
        "FIRST-SOURCE pass=2",
        "SECOND-SOURCE pass=1",
        "SECOND-SOURCE pass=2",
    ];
    for (prompt, marker) in prompts.iter().zip(expected) {
        assert!(prompt.contains(marker), "expected {marker}:\n{prompt}");
    }
}

// ---------------------------------------------------------------------------
// Retry: recovery reads disk, the next ordinary pass does not
// ---------------------------------------------------------------------------

#[test]
fn document_loop_retry_does_not_replace_startup_source() {
    let run = LoopRun::new("loop-startup-retry").first_launch_rewrites(
        "root.md",
        r#"---
start:
  - append_line: ["events.log", "start RECOVERY pass={{ loop.count }}"]
failure:
  - retry: 1
loop:
  until: "true"
  max: 3
---
RECOVERY-BODY pass={{ loop.count }}
"#,
    );
    run.committed(
        "root.md",
        r#"---
initialize:
  - append_line: ["events.log", "initialize"]
start:
  - append_line: ["events.log", "start ORIGINAL pass={{ loop.count }}"]
failure:
  - retry: 1
loop:
  until: "loop.count >= 2"
  max: 3
---
ORIGINAL-BODY pass={{ loop.count }}
"#,
    );

    let outcome = run.run("retry", "fail,ok,ok,ok", &["compose", "--goose", "root.md"]);

    let prompts = run.prompts("retry");
    let events = run.events();
    assert_eq!(outcome.code, Some(0), "{}\nevents: {events:?}", outcome.output);
    assert_eq!(
        events,
        [
            "initialize",
            "start ORIGINAL pass=1",
            "start RECOVERY pass=1",
            "start ORIGINAL pass=2",
        ],
        "{}\nprompts: {prompts:#?}",
        outcome.output
    );
    assert_eq!(prompts.len(), 3, "{}", outcome.output);
    assert!(prompts[0].contains("ORIGINAL-BODY pass=1"), "{}", prompts[0]);
    assert!(prompts[1].contains("RECOVERY-BODY pass=1"), "the retry reads disk:\n{}", prompts[1]);
    assert!(
        prompts[2].contains("ORIGINAL-BODY pass=2"),
        "the next ordinary pass returns to the startup source:\n{}",
        prompts[2]
    );
}

// ---------------------------------------------------------------------------
// The looping guide's example runs as written
// ---------------------------------------------------------------------------

/// The "What each pass reads" example in the looping guide: a `set` carries a
/// value into the next pass, the gate reads `progress.md` again, and an edit
/// to the root during pass 1 reaches only the next run.
#[test]
fn looping_guide_example_keeps_the_root_and_carries_set_values() {
    let guide = include_str!("../../../docs/topics/flow-control/looping.md");
    let example = guide
        .split_once("## What each pass reads")
        .and_then(|(_, section)| section.split_once("```markdown\n"))
        .and_then(|(_, rest)| rest.split_once("```"))
        .expect("looping.md keeps its what-each-pass-reads example")
        .0;
    let run = LoopRun::new("loop-startup-guide-example")
        .first_launch_rewrites("loop.md", "---\nloop:\n  until: \"true\"\n---\nEDITED-BODY\n")
        .second_launch_rewrites("progress.md", "---\ndone: true\n---\n");
    run.committed("progress.md", "---\ndone: false\n---\n");
    run.committed("loop.md", example);

    let outcome = run.run("guide", "ok,ok,ok", &["compose", "--goose", "loop.md"]);

    let prompts = run.prompts("guide");
    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert_eq!(prompts.len(), 2, "the gate after pass 2 reads `done: true`\n{}", outcome.output);
    assert!(prompts[0].contains("Pass 1; the previous pass was none."), "{}", prompts[0]);
    assert!(
        prompts[1].contains("Pass 2; the previous pass was 1."),
        "pass 1's `set` carries into pass 2:\n{}",
        prompts[1]
    );
    assert!(
        !prompts[1].contains("EDITED-BODY"),
        "pass 2 composes the kept root, not pass 1's rewrite:\n{}",
        prompts[1]
    );

    let fresh = run.fresh_run("fresh", &["compose", "--goose", "loop.md"]);
    let fresh_prompts = run.prompts("fresh");
    assert_eq!(fresh.code, Some(0), "{}", fresh.output);
    assert_eq!(fresh_prompts.len(), 1, "{}", fresh.output);
    assert!(fresh_prompts[0].contains("EDITED-BODY"), "a new run reads the rewrite:\n{}", fresh_prompts[0]);
}

/// The looping guide's `inline-compose` sentinel example: an explicit
/// `frontmatter('task.md', 'done')` read sees the agent's edit, so the loop
/// ends after the pass that set it.
#[test]
fn looping_guide_inline_sentinel_reads_the_live_file() {
    let guide = include_str!("../../../docs/topics/flow-control/looping.md");
    let snippet = guide
        .split_once("then loop on an explicit read of that property")
        .and_then(|(_, rest)| rest.split_once("```yaml\n"))
        .and_then(|(_, rest)| rest.split_once("```"))
        .expect("looping.md keeps its inline sentinel example")
        .0;
    let frontmatter: String =
        snippet.lines().map(|line| format!("{}\n", line.strip_prefix("  ").unwrap_or(line))).collect();
    let finished = frontmatter.replace("done: false", "done: true");
    let run = LoopRun::new("loop-startup-inline-sentinel")
        .first_launch_rewrites("task.md", &format!("---\n{finished}---\nFinished body.\n"));
    run.committed("task.md", &format!("---\n{frontmatter}---\nInitial body.\n"));

    let outcome = run.run("sentinel", "ok,ok,ok", &["inline-compose", "--goose", "task.md"]);

    assert_eq!(outcome.code, Some(0), "{}", outcome.output);
    assert_eq!(run.prompts("sentinel").len(), 1, "the gate after pass 1 reads `done: true`\n{}", outcome.output);
}

/// The control for the example above: a bare `done` reads the document the
/// loop kept at startup, so the agent's `done: true` never ends the loop.
#[test]
fn a_bare_inline_sentinel_reads_the_kept_document() {
    let document = |done: bool, body: &str| {
        format!(
            "---\nloop:\n  until: \"done == true\"\n  max: 2\ndone: {done}\nprompt: \"Set `done: true` when finished.\"\n---\n{body}\n"
        )
    };
    let run = LoopRun::new("loop-startup-bare-sentinel")
        .first_launch_rewrites("task.md", &document(true, "Pass one body."))
        .second_launch_rewrites("task.md", &document(true, "Pass two body."));
    run.committed("task.md", &document(false, "Initial body."));

    let outcome = run.run("bare", "ok,ok,ok", &["inline-compose", "--goose", "task.md"]);

    assert_eq!(run.prompts("bare").len(), 2, "{}", outcome.output);
    assert_ne!(outcome.code, Some(0), "the cap is reached with `done` still false\n{}", outcome.output);
    assert!(outcome.output.contains("loop limit exceeded"), "the cap failure is reported:\n{}", outcome.output);
}
