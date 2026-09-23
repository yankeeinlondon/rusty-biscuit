//! Level-1 acceptance coverage for fix `2026-09-15-initialize-after-proxy`.
//!
//! `compose_initialize_staged_boot.rs` pins each entry path's staged boot. This
//! file covers the rest of the spec's acceptance criteria: repeated invocations
//! over a persisted file (AC4), approval integrity of commands reachable only
//! after `initialize` (AC5), mutation visibility (AC6), control flow — proxy
//! chains, `stop`, `error`, retry, and resume (AC7/AC11) — `inline-compose`
//! proxy adoption (AC9), the shipped implementation router with the reported
//! spec spelling (AC10), and resolution parity between `ensure_file` and the
//! transclusion (AC12).
//!
//! Every provider is a fake `claude` (or `goose` for the shipped route) that
//! appends the prompt it received to `prompt.txt`, one attempt per section.
//! Lifecycle markers use `append_line`, which resolves against the fixture
//! repository root.

use crate::common;

use common::{CliProcessFixture, InlineAgentStub, sh_quote, strip_ansi, write, write_executable};
use std::fs;
use std::path::{Path, PathBuf};

const GENERATED: &str = "generated/notes.md";
const GENERATED_MARKER: &str = "GENERATED-BY-INITIALIZE";
const BODY_MARKER: &str = "STAGED-BODY";
const ATTEMPT_SEPARATOR: &str = "=== attempt ===";

/// The `initialize` stack that creates the included file with content.
const CREATING_INITIALIZE: &str = "initialize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'initialize']}\n    \
     - action: {ensure_file: 'generated/notes.md'}\n    \
     - action: {append_line: ['generated/notes.md', 'GENERATED-BY-INITIALIZE']}\n";

struct Acceptance {
    fixture: CliProcessFixture,
    events: PathBuf,
    prompt: PathBuf,
}

impl Acceptance {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.initialize_repository();
        fixture.seed_user_config();
        let events = fixture.cwd().join("events.log");
        let prompt = fixture.workspace_path().join("prompt.txt");
        Self {
            fixture,
            events,
            prompt,
        }
    }

    /// A provider that records each attempt's prompt and launch, then exits with
    /// the code at `exit_codes[attempt - 1]` (0 past the end of the list).
    fn install_provider(&self, binary: &str, exit_codes: &[i32]) {
        let counter = self.fixture.workspace_path().join("attempts");
        let cases: String = exit_codes
            .iter()
            .enumerate()
            .map(|(index, code)| format!("  {}) exit {code} ;;\n", index + 1))
            .collect();
        write_executable(
            &self.fixture.bin_dir().join(binary),
            &format!(
                "#!/bin/sh\nn=$(( $(cat {counter} 2>/dev/null || echo 0) + 1 ))\n\
                 printf '%s' \"$n\" > {counter}\n\
                 {{ printf '%s\\n' {separator}; printf '%s\\n' \"$@\"; cat; }} >> {prompt}\n\
                 printf 'provider-ran\\n' >> {events}\n\
                 case \"$n\" in\n{cases}  *) exit 0 ;;\nesac\n",
                counter = sh_quote(&counter.display().to_string()),
                separator = sh_quote(ATTEMPT_SEPARATOR),
                prompt = sh_quote(&self.prompt.display().to_string()),
                events = sh_quote(&self.events.display().to_string()),
            ),
        );
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.fixture.cwd().join(relative)
    }

    fn write_doc(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path(relative);
        write(&path, text);
        path
    }

    fn run(&self, args: &[&str]) -> (bool, String) {
        let output = self.fixture.command().args(args).output().unwrap();
        let text = format!(
            "{}{}",
            strip_ansi(&String::from_utf8_lossy(&output.stdout)),
            strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        );
        assert!(
            !self.fixture.audio_spool().exists(),
            "no lifecycle audio may be published:\n{text}"
        );
        (output.status.success(), text)
    }

    fn events(&self) -> Vec<String> {
        fs::read_to_string(&self.events)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn count(&self, event: &str) -> usize {
        self.events().iter().filter(|line| *line == event).count()
    }

    /// Each provider attempt's recorded prompt, in launch order.
    fn attempts(&self) -> Vec<String> {
        fs::read_to_string(&self.prompt)
            .unwrap_or_default()
            .split(ATTEMPT_SEPARATOR)
            .skip(1)
            .map(str::to_string)
            .collect()
    }
}

/// A document whose `initialize` creates the file its body includes.
fn creating_doc(extra_frontmatter: &str) -> String {
    format!(
        "---\ntitle: staged\n{extra_frontmatter}{CREATING_INITIALIZE}---\n\
         {BODY_MARKER} phase {{{{phase}}}}.\n\n::file {GENERATED}\n"
    )
}

fn assert_no_early_discovery(output: &str) {
    assert!(
        !output.contains("File not found") && !output.contains("TransclusionError"),
        "body discovery dereferenced an include before initialize ran:\n{output}"
    );
}

// ── AC4: existing file and repeated invocation ──────────────────────────────

/// `ensure_file` keeps a file that already has content, and the prompt includes
/// those exact bytes.
#[test]
fn ensure_file_preserves_an_existing_include_the_prompt_reads() {
    let accept = Acceptance::new("init-accept-existing");
    accept.install_provider("claude", &[]);
    let existing = "PRE-EXISTING-NOTES\nsecond line\n";
    write(&accept.path(GENERATED), existing);
    let doc = accept.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: existing\ninitialize:\n  stack:\n    \
             - action: {{append_line: ['events.log', 'initialize']}}\n    \
             - action: {{ensure_file: '{GENERATED}'}}\n---\n{BODY_MARKER}\n\n::file {GENERATED}\n"
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(success, "{output}");
    assert_eq!(accept.events(), ["initialize", "provider-ran"], "{output}");
    assert_eq!(fs::read_to_string(accept.path(GENERATED)).unwrap(), existing);
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 1, "{attempts:?}");
    assert!(attempts[0].contains("PRE-EXISTING-NOTES\nsecond line"), "{}", attempts[0]);
}

/// A second invocation reads what the first persisted plus what its own
/// `initialize` appended, and each invocation initializes exactly once.
#[test]
fn a_second_invocation_reads_the_persisted_file_and_initializes_once() {
    let accept = Acceptance::new("init-accept-repeat");
    accept.install_provider("claude", &[]);
    let doc = accept.write_doc("doc.md", &creating_doc("phase: 1\n"));

    let (first_success, first) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);
    assert!(first_success, "{first}");
    assert_eq!(fs::read_to_string(accept.path(GENERATED)).unwrap(), format!("{GENERATED_MARKER}\n"));

    let (second_success, second) =
        accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&second);
    assert!(second_success, "{second}");
    assert_eq!(
        accept.events(),
        ["initialize", "provider-ran", "initialize", "provider-ran"],
        "each invocation initializes once, before its provider"
    );
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 2, "{attempts:?}");
    assert_eq!(attempts[0].matches(GENERATED_MARKER).count(), 1, "{}", attempts[0]);
    assert_eq!(
        attempts[1].matches(GENERATED_MARKER).count(),
        2,
        "the second prompt reads the persisted line plus this invocation's append:\n{}",
        attempts[1]
    );
}

// ── AC5: approval integrity after initialize ────────────────────────────────

/// A `::shell` directive that exists only because `initialize` wrote it into an
/// include is audited from the stabilized reread: without an approval handler it
/// is refused and never runs, and the provider never launches.
#[test]
fn a_shell_directive_initialize_writes_into_an_include_is_audited_before_it_runs() {
    let accept = Acceptance::new("init-accept-shell-include");
    accept.install_provider("claude", &[]);
    let effect = accept.fixture.workspace_path().join("include-shell-effect.txt");
    let doc = accept.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: shell include\ninitialize:\n  stack:\n    \
             - action: {{append_line: ['events.log', 'initialize']}}\n    \
             - action: {{ensure_file: '{GENERATED}'}}\n    \
             - action: {{append_line: ['{GENERATED}', '::shell touch {effect}']}}\n\
             ---\n{BODY_MARKER}\n\n::file {GENERATED}\n",
            effect = effect.display(),
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "--claude"]);

    assert!(!success, "an unapproved include shell must refuse the run:\n{output}");
    assert!(output.contains("approval"), "{output}");
    assert_no_early_discovery(&output);
    assert_eq!(accept.count("initialize"), 1, "{output}");
    assert!(
        accept.path(GENERATED).is_file(),
        "initialize has no shell command, so it ran and created the include"
    );
    assert!(!effect.exists(), "the generated include's shell ran unapproved:\n{output}");
    assert_eq!(accept.count("provider-ran"), 0, "{output}");
}

/// The approved counterpart: with `-y` the generated include's command runs and
/// its output reaches the prompt.
#[test]
fn an_approved_shell_directive_initialize_writes_into_an_include_runs() {
    let accept = Acceptance::new("init-accept-shell-include-approved");
    accept.install_provider("claude", &[]);
    let doc = accept.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: shell include\ninitialize:\n  stack:\n    \
             - action: {{ensure_file: '{GENERATED}'}}\n    \
             - action: {{append_line: ['{GENERATED}', '::shell echo GENERATED-SHELL-OUTPUT']}}\n\
             ---\n{BODY_MARKER}\n\n::file {GENERATED}\n"
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(success, "{output}");
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 1, "{output}");
    assert!(attempts[0].contains("GENERATED-SHELL-OUTPUT"), "{}", attempts[0]);
}

/// Discovery stays condition-blind after initialize: a command inside an
/// existing include under a false condition still needs approval, and is not
/// executed either way.
#[test]
fn a_false_condition_include_still_contributes_its_command_to_the_approval_set() {
    let accept = Acceptance::new("init-accept-false-condition");
    accept.install_provider("claude", &[]);
    let effect = accept.fixture.workspace_path().join("false-branch-effect.txt");
    write(
        &accept.path("partials/guarded.md"),
        &format!("::shell touch {}\n", effect.display()),
    );
    let doc = accept.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: false branch\n{CREATING_INITIALIZE}---\n{BODY_MARKER}\n\n::file {GENERATED}\n\n\
             ::block when=\"false\"\n::file partials/guarded.md\n::end-block\n"
        ),
    );

    let (refused, refused_output) = accept.run(&["compose", doc.to_str().unwrap(), "--claude"]);

    assert!(!refused, "the false branch's command still needs approval:\n{refused_output}");
    assert!(refused_output.contains("approval"), "{refused_output}");
    assert_no_early_discovery(&refused_output);
    assert_eq!(accept.count("provider-ran"), 0, "{refused_output}");

    let (approved, approved_output) =
        accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(approved, "{approved_output}");
    assert_eq!(accept.count("provider-ran"), 1, "{approved_output}");
    assert!(!effect.exists(), "a false-condition command is approved, never executed");
}

// ── AC6: mutation visibility ────────────────────────────────────────────────

/// `initialize` rewrites an include that already existed at bootstrap: the
/// prompt carries the rewritten bytes, not the bootstrap-time content.
#[test]
fn an_include_initialize_rewrites_reaches_the_prompt_with_its_new_content() {
    let accept = Acceptance::new("init-accept-rewrite-include");
    accept.install_provider("claude", &[]);
    write(&accept.path(GENERATED), "---\ncontent: BOOTSTRAP-TIME-CONTENT\n---\n{{content}}\n");
    let doc = accept.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: rewrite\ninitialize:\n  stack:\n    \
             - action: {{set_frontmatter: ['{path}', 'content', 'REWRITTEN-BY-INITIALIZE']}}\n\
             ---\n{BODY_MARKER}\n\n::file {GENERATED}\n",
            path = accept.path(GENERATED).display(),
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(success, "{output}");
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 1, "{output}");
    assert!(attempts[0].contains("REWRITTEN-BY-INITIALIZE"), "{}", attempts[0]);
    assert!(!attempts[0].contains("BOOTSTRAP-TIME-CONTENT"), "{}", attempts[0]);
}

/// `initialize` appends to the target document itself: the appended text reaches
/// the prompt, and a `::shell` it appends is audited from the reread (refused
/// without approval, so it never runs).
#[test]
fn initialize_mutating_the_document_itself_is_prompted_and_audited_from_the_reread() {
    let accept = Acceptance::new("init-accept-self-mutation");
    accept.install_provider("claude", &[]);
    let appended_doc = |effect: Option<&Path>| {
        let shell = effect
            .map(|path| format!("    - action: {{append_line: ['doc.md', '::shell touch {}']}}\n", path.display()))
            .unwrap_or_default();
        format!(
            "---\ntitle: self mutation\ninitialize:\n  stack:\n    \
             - action: {{append_line: ['events.log', 'initialize']}}\n    \
             - action: {{append_line: ['doc.md', 'APPENDED-BY-INITIALIZE']}}\n{shell}\
             ---\n{BODY_MARKER}\n"
        )
    };

    let doc = accept.write_doc("doc.md", &appended_doc(None));
    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);
    assert!(success, "{output}");
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 1, "{output}");
    assert!(attempts[0].contains("APPENDED-BY-INITIALIZE"), "{}", attempts[0]);
    assert_eq!(accept.count("initialize"), 1, "the reread must not re-initialize");

    let effect = accept.fixture.workspace_path().join("self-shell-effect.txt");
    let doc = accept.write_doc("doc.md", &appended_doc(Some(&effect)));
    let (refused, refused_output) = accept.run(&["compose", doc.to_str().unwrap(), "--claude"]);
    assert!(!refused, "{refused_output}");
    assert!(refused_output.contains("approval"), "{refused_output}");
    assert!(!effect.exists(), "a command initialize appended ran unapproved:\n{refused_output}");
    assert_eq!(accept.count("provider-ran"), 1, "no second launch:\n{refused_output}");
}

// ── AC7 / AC11: control flow ────────────────────────────────────────────────

/// A three-document chain: each adopted document initializes exactly once, and
/// the abandoned middle document's missing include is never dereferenced.
#[test]
fn an_initialize_proxy_chain_never_reads_an_abandoned_body() {
    let accept = Acceptance::new("init-accept-chain");
    accept.install_provider("claude", &[]);
    accept.write_doc("target.md", &creating_doc("phase: 4\n"));
    accept.write_doc(
        "middle.md",
        "---\ntitle: middle\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'middle-initialize']}\n    \
         - action: {proxy: 'target.md'}\n---\nMIDDLE-BODY\n\n::file abandoned/never.md\n",
    );
    let router = accept.write_doc(
        "router.md",
        "---\ntitle: router\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'router-initialize']}\n    \
         - action: {proxy: 'middle.md'}\n---\nROUTER-BODY\n\n::file abandoned/router.md\n",
    );

    let (success, output) = accept.run(&["compose", router.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "{output}");
    assert_eq!(
        accept.events(),
        ["router-initialize", "middle-initialize", "initialize", "provider-ran"],
        "{output}"
    );
    let attempts = accept.attempts();
    assert!(attempts[0].contains(&format!("{BODY_MARKER} phase 4.")), "{}", attempts[0]);
    assert!(attempts[0].contains(GENERATED_MARKER), "{}", attempts[0]);
    assert!(!attempts[0].contains("MIDDLE-BODY") && !attempts[0].contains("ROUTER-BODY"));
}

/// `stop` ends the `initialize` stack cleanly: later actions do not run, and the
/// run continues into the stabilized reread with what ran before the stop.
#[test]
fn an_initialize_stop_ends_the_stack_and_the_run_continues_from_the_reread() {
    let accept = Acceptance::new("init-accept-stop");
    accept.install_provider("claude", &[]);
    let doc = accept.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: stop\ninitialize:\n  stack:\n    \
             - action: {{append_line: ['events.log', 'initialize']}}\n    \
             - action: {{ensure_file: '{GENERATED}'}}\n    \
             - action: {{append_line: ['{GENERATED}', '{GENERATED_MARKER}']}}\n    \
             - action: stop\n    \
             - action: {{append_line: ['events.log', 'after-stop']}}\n\
             ---\n{BODY_MARKER}\n\n::file {GENERATED}\n"
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "{output}");
    assert_eq!(accept.events(), ["initialize", "provider-ran"], "{output}");
    assert!(accept.attempts()[0].contains(GENERATED_MARKER));
}

/// `error` in `initialize` fails the run without reading the body (whose include
/// nothing creates) and without launching the provider.
#[test]
fn an_initialize_error_fails_without_reading_the_body_or_launching() {
    let accept = Acceptance::new("init-accept-error");
    accept.install_provider("claude", &[]);
    let doc = accept.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: error\ninitialize:\n  stack:\n    \
             - action: {{append_line: ['events.log', 'initialize']}}\n    \
             - action: {{error: 'INITIALIZE-REFUSED'}}\n\
             ---\n{BODY_MARKER}\n\n::file {GENERATED}\n"
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(!success, "{output}");
    assert!(output.contains("INITIALIZE-REFUSED"), "the initialize failure is not obscured:\n{output}");
    assert_no_early_discovery(&output);
    assert_eq!(accept.count("initialize"), 1, "{output}");
    assert_eq!(accept.count("provider-ran"), 0, "{output}");
}

/// A `failure` → `retry` re-enters the provider attempt from a fresh read of the
/// body — it sees what the failure stack appended to the include — without a
/// second `initialize`.
#[test]
fn a_retry_rereads_the_include_without_initializing_again() {
    let accept = Acceptance::new("init-accept-retry");
    accept.install_provider("claude", &[1]);
    let doc = accept.write_doc(
        "doc.md",
        &creating_doc(
            "phase: 1\nfailure:\n  stack:\n    \
             - action: {append_line: ['events.log', 'failure']}\n    \
             - action: {append_line: ['generated/notes.md', 'APPENDED-BEFORE-RETRY']}\n    \
             - action: {retry: 1}\n",
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "the retried attempt succeeds:\n{output}");
    assert_eq!(
        accept.events(),
        ["initialize", "provider-ran", "failure", "provider-ran"],
        "{output}"
    );
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 2, "{output}");
    assert!(!attempts[0].contains("APPENDED-BEFORE-RETRY"), "{}", attempts[0]);
    assert!(
        attempts[1].contains(GENERATED_MARKER) && attempts[1].contains("APPENDED-BEFORE-RETRY"),
        "the retry composes from a fresh read of the include:\n{}",
        attempts[1]
    );
}

/// A `failure` → `resume` continues the session without a second `initialize`.
#[test]
fn a_resume_continues_the_session_without_initializing_again() {
    let accept = Acceptance::new("init-accept-resume");
    let session = "init-accept-session";
    write_executable(
        &accept.fixture.bin_dir().join("claude"),
        &format!(
            "#!/bin/sh\nprompt=$(cat)\nprintf 'provider-ran\\n' >> {events}\n\
             printf '%s\\n' '{{\"type\":\"init\",\"session_id\":\"{session}\",\"model\":\"claude-test\"}}'\n\
             case \" $* \" in\n  *\" -r {session} \"*) printf 'resumed\\n' >> {events}; exit 0 ;;\n\
             esac\ncase \"$prompt\" in\n  *{GENERATED_MARKER}*) printf 'first-prompt-included\\n' >> {events} ;;\n\
             esac\nexit 1\n",
            events = sh_quote(&accept.events.display().to_string()),
        ),
    );
    let doc = accept.write_doc(
        "doc.md",
        &creating_doc(
            "phase: 1\nfailure:\n  stack:\n    \
             - action: {append_line: ['events.log', 'failure']}\n    \
             - action: {resume: 'please continue'}\n",
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "the resumed attempt succeeds:\n{output}");
    assert_eq!(
        accept.events(),
        ["initialize", "provider-ran", "first-prompt-included", "failure", "provider-ran", "resumed"],
        "{output}"
    );
}

// ── AC9: inline-compose proxy adoption ──────────────────────────────────────

#[test]
fn inline_compose_proxy_target_initialize_creates_a_file_its_prompt_includes() {
    let accept = Acceptance::new("init-accept-inline-proxy");
    let target = accept.write_doc(
        "target.md",
        &format!(
            "---\ntitle: inline target\nprompt: |\n  Summarize the notes.\n\n  ::file {GENERATED}\n\
             {CREATING_INITIALIZE}---\noriginal target body\n"
        ),
    );
    let router = accept.write_doc(
        "router.md",
        "---\ntitle: router\nprompt: ROUTER-PROMPT\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'router-initialize']}\n    \
         - action: {proxy: 'target.md'}\n---\nrouter body\n",
    );
    let prelude = format!(
        "{{ printf '%s\\n' {separator}; printf '%s\\n' \"$@\"; cat; }} >> {prompt}\n\
         printf 'provider-ran\\n' >> {events}\n",
        separator = sh_quote(ATTEMPT_SEPARATOR),
        prompt = sh_quote(&accept.prompt.display().to_string()),
        events = sh_quote(&accept.events.display().to_string()),
    );
    InlineAgentStub::new(&target)
        .prelude(&prelude)
        .body("Agent-written target body.\n")
        .install(accept.fixture.bin_dir(), "claude");

    let (success, output) =
        accept.run(&["inline-compose", router.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "{output}");
    assert_eq!(
        accept.events(),
        ["router-initialize", "initialize", "provider-ran"],
        "{output}"
    );
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 1, "{output}");
    assert!(attempts[0].contains(GENERATED_MARKER), "{}", attempts[0]);
    assert!(!attempts[0].contains("ROUTER-PROMPT"), "{}", attempts[0]);
    let written = fs::read_to_string(&target).unwrap();
    assert!(written.contains("Agent-written target body."), "{written}");
    assert!(fs::read_to_string(&router).unwrap().contains("router body"));
}

// ── AC10: the shipped implementation router ─────────────────────────────────

/// Stage the shipped router and the drift-guarded `implement-plan.md` fixture
/// under `prompts/`, with fixture-owned spec/plan data at the reported
/// `fixes/2026-09-14-cicd-improvements/` location and no log file.
fn stage_shipped_route(accept: &Acceptance, plan_phase: u32, target_suffix: &str) {
    let manifest = biscuit_test_harness::manifest_dir!();
    let repository = manifest.ancestors().nth(2).expect("repository root");
    write(
        &accept.path("prompts/implement.md"),
        &fs::read_to_string(repository.join("prompts/implement.md")).unwrap(),
    );
    let target = fs::read_to_string(
        manifest.join("tests/fixtures/shipped_implement_route/_implement/implement-plan.md"),
    )
    .unwrap();
    write(
        &accept.path("prompts/_implement/implement-plan.md"),
        &format!("{target}{target_suffix}"),
    );
    for snippet in ["_no_formatting.md", "_os.md", "_set_spec_schema.md"] {
        write(
            &accept.path(&format!("prompts/{snippet}")),
            &fs::read_to_string(repository.join("prompts").join(snippet)).unwrap(),
        );
    }
    write(
        &accept.path("fixes/2026-09-14-cicd-improvements/spec.md"),
        "---\nimplemented: false\n---\n# Spec\n",
    );
    write(
        &accept.path("fixes/2026-09-14-cicd-improvements/plan.md"),
        &format!("---\ntotal_phases: {plan_phase}\nstart_phase: {plan_phase}\n---\n# Plan\n"),
    );
    accept.install_provider("goose", &[]);
}

/// The original argument spelling, run from the repository root.
const REPORTED_SPEC_ARG: &str = "spec=fixes/2026-09-14-cicd-improvements/spec.md";
const REPORTED_LOG: &str = "fixes/2026-09-14-cicd-improvements/implementation-log.md";

/// The reported invocation through the shipped router: the target's
/// `initialize` creates the log at the full `dirname(spec)` path, and the
/// provider receives the completed prompt that starts the log.
#[test]
fn the_shipped_router_implements_a_spec_whose_log_does_not_exist_yet() {
    let accept = Acceptance::new("init-accept-shipped-router");
    stage_shipped_route(&accept, 1, "");
    assert!(!accept.path(REPORTED_LOG).exists());

    let (success, output) =
        accept.run(&["compose", "prompts/implement.md", REPORTED_SPEC_ARG, "-y", "--goose"]);

    assert_no_early_discovery(&output);
    assert!(success, "the shipped route must succeed:\n{output}");
    assert_eq!(accept.count("provider-ran"), 1, "{output}");
    assert_eq!(fs::read_to_string(accept.path(REPORTED_LOG)).unwrap(), "");
    assert!(
        !accept.path("2026-09-14-cicd-improvements").exists(),
        "the log path must keep its `fixes/` component"
    );
    let attempts = accept.attempts();
    assert!(attempts[0].contains(REPORTED_LOG), "{}", attempts[0]);
    assert!(
        attempts[0].contains("the log file for this implementation has not been started yet"),
        "{}",
        attempts[0]
    );
    assert!(!attempts[0].contains("Implementation Router"), "{}", attempts[0]);
}

/// The reported body shape — the log included inside nested `file_exists(log)`
/// and `phase > 1` blocks — appended to the shipped target and driven through
/// the shipped router. Phase 1 excludes the log; a later phase includes the log
/// `initialize` just created. Before the fix both failed with `File not found`.
#[test]
fn the_shipped_router_reaches_the_reported_guarded_log_include_at_each_phase() {
    const REPORTED_BLOCK: &str = "\n::block when=\"file_exists(log)\"\n::block when=\"phase > 1\"\n\
                                  LOG-INCLUDE-START\n::file {{log}}\nLOG-INCLUDE-END\n\
                                  ::end-block\n::end-block\n";

    for (phase, included) in [(1, false), (2, true)] {
        let accept = Acceptance::new(&format!("init-accept-shipped-guard-{phase}"));
        stage_shipped_route(&accept, phase, REPORTED_BLOCK);

        let (success, output) =
            accept.run(&["compose", "prompts/implement.md", REPORTED_SPEC_ARG, "-y", "--goose"]);

        assert_no_early_discovery(&output);
        assert!(success, "phase {phase}:\n{output}");
        assert!(accept.path(REPORTED_LOG).is_file(), "phase {phase}: initialize creates the log");
        let attempts = accept.attempts();
        assert_eq!(attempts.len(), 1, "phase {phase}:\n{output}");
        assert_eq!(
            attempts[0].contains("LOG-INCLUDE-START"),
            included,
            "phase {phase}: the guarded include tracks the phase:\n{}",
            attempts[0]
        );
    }
}

// ── AC12: resolution parity ─────────────────────────────────────────────────
//
// Lifecycle file mutations and `::file` both resolve document-authored values
// through the invocation's captured `FileResolutionContext`. Existing targets
// follow normal precedence; an absent creation target uses the first candidate.

/// A document below the repository root whose `initialize` ensures `reference`,
/// appends a marker to it, and whose body includes the same `reference`. An
/// optional decoy sits where a divergent resolution would look, so success
/// proves both sides named one file: the ensured file carries the marker, the
/// prompt carries the marker, and the decoy never reaches the prompt.
fn assert_resolution_parity(name: &str, reference: &str, resolved: &str, decoy: Option<&str>) {
    let accept = Acceptance::new(name);
    accept.install_provider("claude", &[]);
    if let Some(decoy) = decoy {
        write(&accept.path(decoy), "DECOY-CONTENT\n");
    }
    let doc = accept.write_doc(
        "docs/area/doc.md",
        &format!(
            "---\ntitle: parity\ninitialize:\n  stack:\n    \
             - action: {{ensure_file: \"{reference}\"}}\n    \
             - action: {{append_line: [\"{reference}\", PARITY-MARKER]}}\n\
             ---\n{BODY_MARKER}\n\n::file \"{reference}\"\n"
        ),
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "{reference}:\n{output}");
    let resolved_content = fs::read_to_string(accept.path(resolved)).unwrap_or_default();
    let source_content =
        fs::read_to_string(accept.path("docs/area/generated/notes.md")).unwrap_or_default();
    let root_content = fs::read_to_string(accept.path("generated/notes.md")).unwrap_or_default();
    assert_eq!(
        resolved_content,
        "PARITY-MARKER\n",
        "{reference} must be ensured at {resolved}; source={source_content:?}; root={root_content:?}"
    );
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 1, "{output}");
    assert!(
        attempts[0].contains("PARITY-MARKER") && !attempts[0].contains("DECOY-CONTENT"),
        "{reference}: the include resolved to a different file than ensure_file:\n{}",
        attempts[0]
    );
}

#[test]
fn an_interpolated_repository_root_reference_ensures_and_includes_one_file() {
    assert_resolution_parity(
        "init-accept-parity-ctx-root",
        "{{ ctx.repo_root }}/generated/notes.md",
        "generated/notes.md",
        Some("docs/area/generated/notes.md"),
    );
}

#[test]
fn an_unshadowed_implicit_reference_ensures_and_includes_one_file() {
    assert_resolution_parity(
        "init-accept-parity-implicit",
        "generated/notes.md",
        "generated/notes.md",
        None,
    );
}

#[test]
fn an_explicit_relative_reference_ensures_and_includes_one_file() {
    assert_resolution_parity(
        "init-accept-parity-explicit",
        "./generated/notes.md",
        "docs/area/generated/notes.md",
        Some("generated/notes.md"),
    );
}

#[test]
fn a_repository_root_reference_ensures_and_includes_one_file() {
    assert_resolution_parity(
        "init-accept-parity-repository",
        "&generated/notes.md",
        "generated/notes.md",
        Some("docs/area/generated/notes.md"),
    );
}

#[test]
fn a_repository_scoped_reference_ensures_and_includes_one_file() {
    assert_resolution_parity(
        "init-accept-parity-repository-scoped",
        "^generated/notes.md",
        "generated/notes.md",
        Some("docs/area/generated/notes.md"),
    );
}

#[test]
fn a_shadowed_implicit_reference_mutates_the_file_the_include_prefers() {
    let accept = Acceptance::new("init-accept-parity-shadowed");
    accept.install_provider("claude", &[]);
    write(
        &accept.path("docs/area/generated/notes.md"),
        "SOURCE-PREEXISTING\n",
    );
    write(&accept.path("generated/notes.md"), "ROOT-DECOY\n");
    let doc = accept.write_doc(
        "docs/area/doc.md",
        "---\ntitle: parity\ninitialize:\n  stack:\n    \
         - action: {ensure_file: generated/notes.md}\n    \
         - action: {append_line: [generated/notes.md, PARITY-MARKER]}\n\
         ---\nSTAGED-BODY\n\n::file generated/notes.md\n",
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(success, "{output}");
    assert_eq!(
        fs::read_to_string(accept.path("docs/area/generated/notes.md")).unwrap(),
        "SOURCE-PREEXISTING\nPARITY-MARKER\n",
    );
    assert_eq!(
        fs::read_to_string(accept.path("generated/notes.md")).unwrap(),
        "ROOT-DECOY\n",
    );
    let attempts = accept.attempts();
    assert!(attempts[0].contains("SOURCE-PREEXISTING\nPARITY-MARKER"));
    assert!(!attempts[0].contains("ROOT-DECOY"));
}

#[test]
fn every_filesystem_effect_uses_the_document_reference_identity() {
    let accept = Acceptance::new("init-accept-parity-all-fs-effects");
    accept.install_provider("claude", &[]);
    let doc = accept.write_doc(
        "docs/area/doc.md",
        "---\ntitle: parity\npayload:\n  event: ready\ninitialize:\n  stack:\n    \
         - action: {ensure_dir: ./generated/nested}\n    \
         - action: {ensure_file: [./generated/content.md, SEEDED-CONTENT]}\n    \
         - action: {append_jsonl: [./generated/events.md, \"{{ payload }}\"]}\n\
         ---\n::file \"./generated/content.md\"\n::file \"./generated/events.md\"\n",
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(success, "{output}");
    assert!(accept.path("docs/area/generated/nested").is_dir());
    assert_eq!(
        fs::read_to_string(accept.path("docs/area/generated/content.md")).unwrap(),
        "SEEDED-CONTENT",
    );
    assert_eq!(
        fs::read_to_string(accept.path("docs/area/generated/events.md")).unwrap(),
        "{\"event\":\"ready\"}\n",
    );
    assert!(!accept.path("generated").exists());
    let attempts = accept.attempts();
    assert_eq!(attempts.len(), 1, "{output}");
    assert!(attempts[0].contains("SEEDED-CONTENT"));
    assert!(attempts[0].contains("{\"event\":\"ready\"}"));
}

#[test]
fn a_repository_escape_is_rejected_before_any_file_or_provider_effect() {
    let accept = Acceptance::new("init-accept-parity-escape");
    accept.install_provider("claude", &[]);
    let doc = accept.write_doc(
        "docs/area/doc.md",
        "---\ntitle: invalid reference\ninitialize:\n  stack:\n    \
         - action: {ensure_file: '&../outside.md'}\n---\nBODY\n",
    );

    let (success, output) = accept.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(!success, "{output}");
    assert!(output.contains("lifecycle initialize failed"), "{output}");
    assert_eq!(accept.count("provider-ran"), 0, "{output}");
    assert!(!accept.fixture.workspace_path().join("outside.md").exists());
}
