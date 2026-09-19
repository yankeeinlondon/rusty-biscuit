//! Level-1 process coverage for the staged boot of a document that authors
//! `initialize` (fix `2026-09-15-initialize-after-proxy`).
//!
//! `initialize` may create a file the document's own body includes, so no body
//! dependency may be dereferenced before it runs. Each entry path — direct
//! `compose`, `inline-compose`, an adopted proxy target, a loop-owning target's
//! first iteration, and a target a sequence task adopts inside the provider
//! harness — must run `initialize` exactly once, before body
//! discovery and before the provider launches, and must deliver a prompt that
//! includes what `initialize` created.
//!
//! The negative rows pin the boundaries that must not move: a file still
//! missing after `initialize` fails once through the document's own
//! `blocked`/`finalize`; a document without `initialize` fails eagerly exactly
//! as before; a dry run fires no lifecycle event; a `skip` ends the run before
//! any body read; and every `initialize` shell command is refused
//! before it can run.
//!
//! The provider is a fake `claude` that records the prompt it received and its
//! launch in the fixture workspace. Lifecycle markers use `append_line`, which
//! resolves against the fixture repository root.

#![cfg(unix)]

mod common;

use common::{CliProcessFixture, InlineAgentStub, sh_quote, strip_ansi, write, write_executable};
use std::fs;
use std::path::PathBuf;

const GENERATED: &str = "generated/notes.md";
const GENERATED_MARKER: &str = "GENERATED-BY-INITIALIZE";
const BODY_MARKER: &str = "STAGED-BODY";

/// The `initialize` stack that creates the included file with content.
const CREATING_INITIALIZE: &str = "initialize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'initialize']}\n    \
     - action: {ensure_file: 'generated/notes.md'}\n    \
     - action: {append_line: ['generated/notes.md', 'GENERATED-BY-INITIALIZE']}\n";

/// `blocked`/`finalize` markers, so a test can count the catch events.
const CATCH_MARKERS: &str = "blocked:\n  stack:\n    \
     - action: {append_line: ['events.log', 'blocked']}\n\
     finalize:\n  stack:\n    \
     - action: {append_line: ['events.log', 'finalize']}\n";

struct Staged {
    fixture: CliProcessFixture,
    events: PathBuf,
    prompt: PathBuf,
}

impl Staged {
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

    /// A compose provider that appends the prompt it received (argv and stdin)
    /// and a launch marker.
    fn install_compose_provider(&self) {
        write_executable(
            &self.fixture.bin_dir().join("claude"),
            &format!(
                "#!/bin/sh\n{{ printf '%s\\n' \"$@\"; cat; }} >> {prompt}\n\
                 printf 'provider-ran\\n' >> {events}\nexit 0\n",
                prompt = sh_quote(&self.prompt.display().to_string()),
                events = sh_quote(&self.events.display().to_string()),
            ),
        );
    }

    fn write_doc(&self, name: &str, text: &str) -> PathBuf {
        let path = self.fixture.cwd().join(name);
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
        (output.status.success(), text)
    }

    fn events(&self) -> Vec<String> {
        fs::read_to_string(&self.events)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn prompt(&self) -> String {
        fs::read_to_string(&self.prompt).unwrap_or_default()
    }

    fn generated_exists(&self) -> bool {
        self.fixture.cwd().join(GENERATED).is_file()
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
        "body discovery dereferenced the generated include before initialize ran:\n{output}"
    );
}

#[test]
fn compose_initialize_creates_a_file_the_body_includes() {
    let staged = Staged::new("staged-boot-direct");
    staged.install_compose_provider();
    let doc = staged.write_doc("doc.md", &creating_doc("phase: 1\n"));

    let (success, output) = staged.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "the staged compose run failed:\n{output}");
    assert_eq!(
        staged.events(),
        ["initialize", "provider-ran"],
        "initialize must run exactly once, before the provider:\n{output}"
    );
    let prompt = staged.prompt();
    assert!(prompt.contains(&format!("{BODY_MARKER} phase 1.")), "{prompt}");
    assert!(
        prompt.contains(GENERATED_MARKER),
        "the prompt must include the file initialize created:\n{prompt}"
    );
    assert!(
        !output.contains("approval"),
        "`-y` approves every staged command without a prompt:\n{output}"
    );
    assert!(!staged.fixture.audio_spool().exists());
}

#[test]
fn inline_compose_initialize_creates_a_file_the_prompt_includes() {
    let staged = Staged::new("staged-boot-inline");
    let doc_text = format!(
        "---\ntitle: staged inline\nprompt: |\n  Summarize the notes.\n\n  ::file {GENERATED}\n\
         {CREATING_INITIALIZE}---\noriginal body\n"
    );
    let doc = staged.write_doc("doc.md", &doc_text);
    let prelude = format!(
        "{{ printf '%s\\n' \"$@\"; cat; }} >> {prompt}\nprintf 'provider-ran\\n' >> {events}\n",
        prompt = sh_quote(&staged.prompt.display().to_string()),
        events = sh_quote(&staged.events.display().to_string()),
    );
    InlineAgentStub::new(&doc)
        .prelude(&prelude)
        .body("Agent-written body.\n")
        .install(staged.fixture.bin_dir(), "claude");

    let (success, output) =
        staged.run(&["inline-compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "the staged inline-compose run failed:\n{output}");
    assert_eq!(staged.events(), ["initialize", "provider-ran"], "{output}");
    assert!(
        staged.prompt().contains(GENERATED_MARKER),
        "the inline prompt must include the file initialize created:\n{}",
        staged.prompt()
    );
    let written = fs::read_to_string(&doc).unwrap();
    assert!(written.contains("Agent-written body."), "{written}");
    assert!(
        written.contains("initialize:"),
        "the inline closure must keep the authored lifecycle:\n{written}"
    );
}

#[test]
fn a_proxied_target_initialize_creates_a_file_the_target_body_includes() {
    let staged = Staged::new("staged-boot-proxy");
    staged.install_compose_provider();
    staged.write_doc("target.md", &creating_doc("phase: 2\n"));
    let router = staged.write_doc(
        "router.md",
        "---\ntitle: router\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'router-initialize']}\n    \
         - action: {proxy: 'target.md'}\n---\nROUTER-BODY\n",
    );

    let (success, output) = staged.run(&["compose", router.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "the proxied staged run failed:\n{output}");
    assert_eq!(
        staged.events(),
        ["router-initialize", "initialize", "provider-ran"],
        "each document's initialize runs once, before the provider:\n{output}"
    );
    let prompt = staged.prompt();
    assert!(prompt.contains(&format!("{BODY_MARKER} phase 2.")), "{prompt}");
    assert!(prompt.contains(GENERATED_MARKER), "{prompt}");
    assert!(!prompt.contains("ROUTER-BODY"), "{prompt}");
}

#[test]
fn a_looping_target_first_iteration_includes_what_initialize_created() {
    let staged = Staged::new("staged-boot-loop");
    staged.install_compose_provider();
    staged.write_doc(
        "target.md",
        &creating_doc(
            "phase: 1\nloop:\n  until: \"phase > 1\"\n  action: \"increment(phase)\"\n  max: 5\n",
        ),
    );
    let router = staged.write_doc(
        "router.md",
        "---\ntitle: router\ninitialize:\n  stack:\n    \
         - action: {proxy: 'target.md'}\n---\nROUTER-BODY\n",
    );

    let (success, output) = staged.run(&["compose", router.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "the looping staged run failed:\n{output}");
    let events = staged.events();
    assert_eq!(
        events.iter().filter(|event| *event == "initialize").count(),
        1,
        "a loop emits initialize once across its iterations: {events:?}\n{output}"
    );
    assert_eq!(events.first().map(String::as_str), Some("initialize"), "{events:?}");
    assert_eq!(
        events.iter().filter(|event| *event == "provider-ran").count(),
        2,
        "the loop runs phase 1 and phase 2: {events:?}\n{output}"
    );
    let prompt = staged.prompt();
    assert!(prompt.contains(&format!("{BODY_MARKER} phase 1.")), "{prompt}");
    assert!(prompt.contains(&format!("{BODY_MARKER} phase 2.")), "{prompt}");
    assert_eq!(
        prompt.matches(GENERATED_MARKER).count(),
        2,
        "both iterations include the file initialize created:\n{prompt}"
    );
}

/// A sequence task has no command-level coordinator, so its prompt's `initialize`
/// proxy is adopted inside the provider harness. The adopted target's boot must
/// read only its lifecycle surface before its own `initialize`, exactly like a
/// target the command coordinator adopts.
#[test]
fn a_sequence_task_proxy_target_initialize_creates_a_file_its_body_includes() {
    let staged = Staged::new("staged-boot-sequence-task");
    staged.install_compose_provider();
    staged.write_doc("target.md", &creating_doc("phase: 3\n"));
    staged.write_doc(
        "router.md",
        "---\ntitle: router\ninitialize:\n  stack:\n    \
         - action: {append_line: ['events.log', 'router-initialize']}\n    \
         - action: {proxy: 'target.md'}\n---\nROUTER-BODY\n",
    );
    let sequence = staged.write_doc(
        "seq.md",
        "---\nsequence:\n    - name: one\n      prompt: router.md\n---\nBody.\n",
    );

    let (success, output) =
        staged.run(&["sequence", sequence.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "the sequence task's staged target failed:\n{output}");
    assert_eq!(
        staged.events(),
        ["router-initialize", "initialize", "provider-ran"],
        "{output}"
    );
    let prompt = staged.prompt();
    assert!(prompt.contains(&format!("{BODY_MARKER} phase 3.")), "{prompt}");
    assert!(prompt.contains(GENERATED_MARKER), "{prompt}");
}

/// AC8: a file still missing after `initialize` fails through its typed
/// diagnostic once, via the document's own `blocked`/`finalize`, and the
/// provider never launches.
#[test]
fn a_file_still_missing_after_initialize_blocks_once_without_launching() {
    let staged = Staged::new("staged-boot-still-missing");
    staged.install_compose_provider();
    let doc = staged.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: never creates\ninitialize:\n  stack:\n    \
             - action: {{append_line: ['events.log', 'initialize']}}\n\
             {CATCH_MARKERS}---\n{BODY_MARKER}\n\n::file {GENERATED}\n"
        ),
    );

    let (success, output) = staged.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(!success, "a missing include after initialize must fail:\n{output}");
    assert_eq!(
        output.matches("File not found").count(),
        1,
        "the typed missing-file diagnostic renders exactly once:\n{output}"
    );
    assert!(output.contains(GENERATED), "{output}");
    assert_eq!(
        staged.events(),
        ["initialize", "blocked", "finalize"],
        "the stabilized-reread failure routes through blocked/finalize once:\n{output}"
    );
    assert!(!staged.prompt.exists(), "the provider must not launch:\n{output}");
}

/// AC11: a document without `initialize` keeps its eager discovery — the
/// missing include fails before any lifecycle event, as it always has.
#[test]
fn a_document_without_initialize_still_fails_eagerly_on_a_missing_include() {
    let staged = Staged::new("staged-boot-eager");
    staged.install_compose_provider();
    let doc = staged.write_doc(
        "doc.md",
        &format!("---\ntitle: eager\n{CATCH_MARKERS}---\n{BODY_MARKER}\n\n::file {GENERATED}\n"),
    );

    let (success, output) = staged.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert!(!success, "{output}");
    assert!(
        output.contains("TransclusionError") && output.contains(&format!("File not found: {GENERATED}")),
        "the eager boundary keeps its typed diagnostic:\n{output}"
    );
    assert!(
        staged.events().is_empty(),
        "eager discovery fails before any lifecycle event:\n{output}"
    );
    assert!(!staged.prompt.exists(), "{output}");
}

/// Dry run is unchanged: no `initialize` side effect, and the include only
/// `initialize` would create is still reported missing.
#[test]
fn a_dry_run_never_runs_initialize_and_still_reports_the_missing_include() {
    let staged = Staged::new("staged-boot-dry-run");
    staged.install_compose_provider();
    let doc = staged.write_doc("doc.md", &creating_doc("phase: 1\n"));

    let (success, output) =
        staged.run(&["compose", doc.to_str().unwrap(), "--dry-run", "--claude"]);

    assert!(!success, "{output}");
    assert!(output.contains("File not found"), "{output}");
    assert!(staged.events().is_empty(), "dry run fires no lifecycle event:\n{output}");
    assert!(!staged.generated_exists(), "dry run must not create the file:\n{output}");
    assert!(!staged.prompt.exists(), "{output}");
}

/// A `skip` in `initialize` ends the run cleanly before any body read, so an
/// include nothing will ever create is never dereferenced.
#[test]
fn an_initialize_skip_ends_the_run_before_the_body_is_discovered() {
    let staged = Staged::new("staged-boot-skip");
    staged.install_compose_provider();
    let doc = staged.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: skipped\ninitialize:\n  stack:\n    \
             - action: {{append_line: ['events.log', 'initialize']}}\n    \
             - action: skip\n{CATCH_MARKERS}---\n{BODY_MARKER}\n\n::file {GENERATED}\n"
        ),
    );

    let (success, output) = staged.run(&["compose", doc.to_str().unwrap(), "-y", "--claude"]);

    assert_no_early_discovery(&output);
    assert!(success, "a skip ends the run successfully:\n{output}");
    assert_eq!(staged.events(), ["initialize"], "{output}");
    assert!(!staged.prompt.exists(), "{output}");
}

/// R2/AC5: an initialization shell is an authoring error before body discovery,
/// so the refusal names the forbidden action rather than the missing include.
#[test]
fn an_unapproved_initialize_shell_is_refused_before_the_body_is_discovered() {
    let staged = Staged::new("staged-boot-gate");
    staged.install_compose_provider();
    let effect = staged.fixture.workspace_path().join("initialize-effect.txt");
    let doc = staged.write_doc(
        "doc.md",
        &format!(
            "---\ntitle: gated\ninitialize:\n  stack:\n    \
             - action: {{shell: \"touch {effect}\"}}\n\
             {CATCH_MARKERS}---\n{BODY_MARKER}\n\n::file {GENERATED}\n",
            effect = effect.display(),
        ),
    );

    let (success, output) = staged.run(&["compose", doc.to_str().unwrap(), "--claude"]);

    assert!(!success, "{output}");
    assert!(output.contains("is not valid in") && output.contains("initialize"), "{output}");
    assert_no_early_discovery(&output);
    assert!(!effect.exists(), "the unapproved command ran:\n{output}");
    assert!(
        staged.events().is_empty(),
        "a refused gate precedes the document's lifecycle, so nothing is caught:\n{output}"
    );
    assert!(!staged.prompt.exists(), "{output}");
}

#[test]
fn initialize_shells_are_forbidden_even_with_yolo_across_entry_paths() {
    for (command, proxy, looping) in [
        ("compose", false, false),
        ("inline-compose", false, false),
        ("compose", true, false),
        ("inline-compose", true, false),
        ("compose", false, true),
        ("sequence", true, false),
    ] {
        let staged = Staged::new("initialize-shell-prohibited");
        staged.install_compose_provider();
        let effect = staged.fixture.workspace_path().join("forbidden.txt");
        staged.write_doc("target.md", &format!(
            "---\nprompt: Do the task\n{}initialize:\n  stack:\n    - when: 'false'\n      action: {{shell: \"touch {}\"}}\n---\nBody\n",
            if looping { "loop:\n  until: 'true'\n  max: 2\n" } else { "" }, effect.display(),
        ));
        staged.write_doc("router.md", "---\nprompt: Route the task\ninitialize:\n  stack:\n    - action: {proxy: './target.md'}\n---\nRouter\n");
        staged.write_doc("seq.md", "---\nsequence:\n  - name: one\n    prompt: router.md\n---\nSequence\n");
        staged.write_doc(".darkmatter-shell-whitelist", "prefix touch\n");
        let entry = if command == "sequence" { "seq.md" } else if proxy { "router.md" } else { "target.md" };
        let (success, output) = staged.run(&[command, entry, "--claude", "-y"]);
        assert!(!success, "{command}, proxy={proxy}, looping={looping}: {output}");
        assert!(output.contains("is not valid in") && output.contains("initialize"), "{output}");
        assert!(!effect.exists(), "{output}");
        assert!(!staged.prompt.exists(), "provider launched: {output}");
    }
}

#[test]
fn early_catch_shells_cannot_run_or_enable_another_catch_shell() {
    for trigger in ["missing", "error", "proxy", "evaluation", "schema", "audit", "adopted"] {
        for yolo in [false, true] {
            let staged = Staged::new("early-catch-shell-prohibited");
            staged.install_compose_provider();
            let action = match trigger {
                "error" => "{error: 'INITIALIZE-ERROR'}",
                "proxy" => "{proxy: './missing-target.md'}",
                _ => "{ensure_file: 'initialized.md'}",
            };
            let body = match trigger {
                "schema" => "Body",
                "audit" => "::shell rm forbidden.txt",
                _ => "::file ./missing.md",
            };
            let schema = if trigger == "schema" { "$schema:\n  required_value: string(required)\n" } else { "" };
            let mut catches = String::new();
            for event in ["blocked", "failure", "finalize"] {
                let marker = staged.fixture.workspace_path().join(format!("{event}-shell"));
                catches.push_str(&format!(
                    "{event}:\n  stack:\n    - action: {{append_line: ['events.log', '{event}']}}\n",
                ));
                if trigger == "evaluation" && event == "blocked" {
                    catches.push_str("    - action: {info: '{{ definitely_missing_value }}'}\n");
                }
                catches.push_str(&format!(
                    "    - action:\n        action: shell\n        command: \"touch {}\"\n        no_error: true\n", marker.display(),
                ));
            }
            staged.write_doc("doc.md", &format!(
                "---\n{schema}initialize:\n  stack:\n    - action: {action}\n{catches}---\n{body}\n",
            ));
            staged.write_doc("router.md", "---\ninitialize:\n  stack:\n    - action: {proxy: './doc.md'}\n---\nRouter\n");
            staged.write_doc("seq.md", "---\nsequence:\n  - name: one\n    prompt: router.md\n---\nSequence\n");
            let mut args = if trigger == "adopted" {
                vec!["sequence", "seq.md", "--claude"]
            } else {
                vec!["compose", "doc.md", "--claude"]
            };
            if yolo { args.push("-y"); }
            let (success, output) = staged.run(&args);
            assert!(!success, "{trigger}, yolo={yolo}: {output}");
            assert!(output.contains("shell commands are forbidden"), "{trigger}: {output}");
            for event in ["blocked", "failure", "finalize"] {
                assert!(!staged.fixture.workspace_path().join(format!("{event}-shell")).exists(), "{trigger}: {output}");
                assert!(staged.events().iter().filter(|value| *value == event).count() <= 1, "catch repeated: {output}");
            }
            let expected = if trigger == "error" {
                vec!["failure", "finalize"]
            } else {
                vec!["blocked", "failure", "finalize"]
            };
            assert_eq!(staged.events(), expected, "{trigger}: {output}");
            assert!(!staged.prompt.exists(), "provider launched: {output}");
        }
    }
}

/// Review 1's reproduction: a non-shell `initialize`, a still-missing body
/// include, and plain shell actions in `blocked`/`finalize`. It once ran both
/// shells before any approval. Each entry that reaches the shared staged boot —
/// direct, an adopted proxy target, and a loop-owning target — must refuse
/// both, route each catch once, and never launch the provider.
#[test]
fn the_reviewed_early_catch_shell_trigger_is_refused_on_every_entry_path() {
    const LOOP: &str = "phase: 1\nloop:\n  until: 'phase > 1'\n  action: 'increment(phase)'\n  max: 5\n";
    for (entry, looping) in [("doc.md", false), ("router.md", false), ("doc.md", true), ("router.md", true)] {
        for yolo in [false, true] {
            let staged = Staged::new("reviewed-early-catch-shell");
            staged.install_compose_provider();
            let blocked = staged.fixture.workspace_path().join("unapproved-blocked");
            let finalize = staged.fixture.workspace_path().join("unapproved-finalize");
            staged.write_doc("doc.md", &format!(
                "---\n{}initialize:\n  stack:\n    - action: {{ensure_file: marker.md}}\n\
                 blocked:\n  stack:\n    - action: {{append_line: ['events.log', 'blocked']}}\n    \
                 - action: {{shell: \"touch {}\"}}\n\
                 finalize:\n  stack:\n    - action: {{append_line: ['events.log', 'finalize']}}\n    \
                 - action: {{shell: \"touch {}\"}}\n---\n::file ./missing.md\n",
                if looping { LOOP } else { "" }, blocked.display(), finalize.display(),
            ));
            staged.write_doc("router.md", "---\ninitialize:\n  stack:\n    - action: {proxy: './doc.md'}\n---\nRouter\n");
            let mut args = vec!["compose", entry, "--claude"];
            if yolo {
                args.push("-y");
            }

            let (success, output) = staged.run(&args);

            let case = format!("{entry}, looping={looping}, yolo={yolo}");
            assert!(!success, "{case}: {output}");
            assert_eq!(output.matches("shell commands are forbidden").count(), 2, "{case}: {output}");
            assert!(!blocked.exists() && !finalize.exists(), "{case}: a catch shell ran:\n{output}");
            assert_eq!(staged.events(), ["blocked", "finalize"], "{case}: {output}");
            assert!(!staged.prompt.exists(), "{case}: provider launched:\n{output}");
        }
    }
}

#[test]
fn approved_start_shell_runs_once_after_shell_free_initialization() {
    let staged = Staged::new("post-preflight-shell");
    staged.install_compose_provider();
    let effect = staged.fixture.workspace_path().join("start-shell.txt");
    staged.write_doc("doc.md", &format!(
        "---\n{CREATING_INITIALIZE}start:\n  stack:\n    - action: {{shell: \"printf 'ran\\n' >> {}\"}}\n---\n::file {GENERATED}\n",
        effect.display(),
    ));
    let (success, output) = staged.run(&["compose", "doc.md", "--claude", "-y"]);
    assert!(success, "{output}");
    assert_eq!(fs::read_to_string(effect).unwrap(), "ran\n", "{output}");
    assert_eq!(staged.events(), ["initialize", "provider-ran"], "{output}");
}
