//! Level 2 reproduction: a proxied target's `initialize` must run before its
//! body transclusions are discovered.
//!
//! Fix: `claudine/fixes/2026-09-15-initialize-after-proxy/spec.md` (AC1, AC2, AC5).
//!
//! A router's `initialize` stack proxies to a target whose own `initialize`
//! stack creates `implementation-log.md` with `ensure_file`, and whose body
//! includes that file inside the reported nested `file_exists(log)` /
//! `phase > 1` guards. The target's `log` uses `dirname(spec)`, a valid
//! directory path, so the failure cannot be attributed to the shipped prompt's
//! separate `parent_dir(spec)` defect.
//!
//! Before the fix, command-level template discovery dereferences the body's
//! `::file` transclusion (condition-blind by design) before `initialize` runs,
//! so the run fails with `TransclusionError: I/O failure` / `File not found`
//! and the file that initialization would have created is never written.
//!
//! A third test pins the R2 approval boundary on the direct route: without
//! `-y` and without an interactive approval handler, an `initialize` shell
//! command runs before the lifecycle audit refuses it.
//!
//! A fourth test pins the sequence boundary (OQ1 Option A): sequence static
//! preflight still fails on an include that `initialize` would create, and the
//! pane shows the note telling the author to create the file first.
//!
//! ## Hermetic launch
//!
//! The pane's shell belongs to the harness, not to the L1 command builder, so
//! the launcher starts `claudine` under `env -i` with only the fixture's
//! variables: fixture home, fixture `bin` plus the minimal system `PATH`,
//! silenced lifecycle audio, and no rendezvous reporting. The provider is a
//! fake `claude`; no terminal window is opened or focused (detached tmux).
//!
//! Run via the canonical recipe:
//! `just test-l2 initialize_generated_transclusion`.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::{CliProcessFixture, claudine_bin, minimal_system_path, sh_quote, wait_for_exit_marker, write, write_executable};

const SPEC_ARGUMENT: &str = "fixes/2026-09-14-demo/spec.md";
const LOG_RELATIVE: &str = "fixes/2026-09-14-demo/implementation-log.md";
const LOG_MARKER: &str = "PRIOR-PHASE-LOG-MARKER";
const BODY_MARKER: &str = "IMPLEMENT-TARGET-BODY";

const ROUTER_DOC: &str = "\
---
spec: \"\"
initialize:
  stack:
    - when: \"spec && !frontmatter(spec, 'implemented')\"
      action:
        - proxy: ./_implement/implement-plan.md
    - action:
        - error: \"router fell through\"
---
Router body (never composed: the run proxies away at initialize).
";

/// The proxied target. `phase` comes from the caller so one document covers
/// the phase-1 (guard excludes the log) and later-phase (guard includes it)
/// shapes. Non-shell file effects record that `initialize` ran and give the log
/// observable content once it exists.
fn target_doc(events_log: &Path) -> String {
    format!(
        "\
---
spec: \"\"
phase: 1
log: \"{{{{ dirname(spec) + '/implementation-log.md' }}}}\"
initialize:
  stack:
    - action:
        - ensure_file: \"{{{{log}}}}\"
        - append_line: [\"{events}\", initialize]
        - append_line: [\"{{{{log}}}}\", \"{LOG_MARKER}\"]
---
{BODY_MARKER} phase {{{{phase}}}}.

::block when=\"file_exists(log)\"
::block when=\"phase > 1\"
::file {{{{log}}}}
::end-block
::end-block
",
        events = events_log.display(),
    )
}

/// Records the prompt it received on stdin, then its launch.
const FAKE_CLAUDE: &str = "#!/bin/sh\n\
cat > \"$CLAUDINE_PROMPT_FILE\"\n\
printf 'provider-ran\\n' >> \"$CLAUDINE_EVENTS_FILE\"\n\
exit 0\n";

struct Staged {
    fixture: CliProcessFixture,
    events_log: PathBuf,
    prompt_file: PathBuf,
}

fn stage(name: &str) -> Staged {
    let fixture = CliProcessFixture::named(name);
    fixture.initialize_repository();
    fixture.seed_user_config();
    let cwd = fixture.cwd();
    let events_log = fixture.cwd().join("events.log");
    let prompt_file = fixture.workspace_path().join("prompt.txt");

    write(&cwd.join("prompts/router.md"), ROUTER_DOC);
    write(&cwd.join("prompts/_implement/implement-plan.md"), &target_doc(&events_log));
    write(&cwd.join(SPEC_ARGUMENT), "---\nimplemented: false\n---\n# Demo spec\n");
    write(&cwd.join("fixes/2026-09-14-demo/plan.md"), "---\ntotal_phases: 2\n---\n# Demo plan\n");
    write_executable(&fixture.bin_dir().join("claude"), FAKE_CLAUDE);

    Staged {
        fixture,
        events_log,
        prompt_file,
    }
}

fn unique_marker() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("INIT_GEN_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Runs `claudine compose prompts/router.md spec=… phase=… -y --claude` in a
/// tmux pane and returns the rendered frame and exit status.
fn run_router(staged: &Staged, phase: u32) -> (String, String) {
    let arguments = format!(
        "compose prompts/router.md spec={spec} phase={phase} -y --claude",
        spec = sh_quote(SPEC_ARGUMENT),
    );
    run_claudine(staged, &arguments)
}

/// Runs `claudine <arguments>` from the fixture `cwd` with stdin closed, in a
/// tmux pane, and returns the rendered frame and exit status.
///
/// The exit marker is printed by the script rather than typed with `$?`: on a
/// host that loads Atuin AI, a typed `?` opens its overlay and wedges the pane.
fn run_claudine(staged: &Staged, arguments: &str) -> (String, String) {
    let fixture = &staged.fixture;
    let marker = unique_marker();
    let path = std::env::join_paths(std::iter::once(fixture.bin_dir().to_path_buf()).chain(minimal_system_path()))
        .expect("join fixture PATH");
    let quoted = |value: &Path| sh_quote(&value.display().to_string());
    let variables = [
        ("HOME", quoted(fixture.home())),
        ("PATH", quoted(Path::new(&path))),
        ("TERM", "xterm-256color".to_string()),
        ("NO_COLOR", "1".to_string()),
        ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
        ("PLAYA_DRY_RUN", "1".to_string()),
        ("PLAYA_SPOOL_DIR", quoted(&fixture.audio_spool())),
        ("CLAUDINE_PROMPT_FILE", quoted(&staged.prompt_file)),
        ("CLAUDINE_EVENTS_FILE", quoted(&staged.events_log)),
    ]
    .map(|(name, value)| format!("{name}={value}"))
    .join(" ");
    let script = format!(
        "#!/bin/sh\nprintf '\\033[2J\\033[H'\ncd {cwd} || exit 1\n\
         if /usr/bin/env -i {variables} {claudine} {arguments} < /dev/null; then\n  \
         printf '\\n{marker}:0\\n'\nelse\n  printf '\\n{marker}:1\\n'\nfi\n",
        cwd = quoted(fixture.cwd()),
        claudine = sh_quote(claudine_bin()),
    );
    let launcher = fixture.workspace_path().join("launch.sh");
    write_executable(&launcher, &script);

    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    harness
        .send_text(format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string())).as_bytes())
        .expect("send launcher");
    let (frame, status) = wait_for_exit_marker(&mut harness, &marker, Duration::from_secs(60));
    (frame.plain, status)
}

fn event_lines(staged: &Staged) -> Vec<String> {
    fs::read_to_string(&staged.events_log)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// Everything both phases assert: the run succeeded, `initialize` ran exactly
/// once and before the provider, the generated log exists, and the provider
/// received the composed target body.
fn assert_generated_flow(staged: &Staged, frame: &str, status: &str) -> String {
    assert!(
        !frame.contains("TransclusionError") && !frame.contains("File not found"),
        "body discovery dereferenced the generated transclusion before initialize ran:\n{frame}"
    );
    assert_eq!(status, "0", "the proxied compose run failed:\n{frame}");

    let events = event_lines(staged);
    assert_eq!(
        events,
        ["initialize", "provider-ran"],
        "initialize must run exactly once and precede the provider launch:\n{frame}"
    );
    assert!(
        staged.fixture.cwd().join(LOG_RELATIVE).is_file(),
        "initialize's ensure_file never created {LOG_RELATIVE}:\n{frame}"
    );

    let prompt = fs::read_to_string(&staged.prompt_file)
        .unwrap_or_else(|error| panic!("the fake provider received no prompt ({error}):\n{frame}"));
    assert!(
        prompt.contains(BODY_MARKER),
        "the provider did not receive the target's composed body:\n{prompt}"
    );
    prompt
}

#[test]
#[serial(level2_terminal)]
fn level2_proxied_initialize_creates_log_before_guarded_phase_one_body_is_discovered() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage("l2-init-gen-phase-one");
    let (frame, status) = run_router(&staged, 1);
    let prompt = assert_generated_flow(&staged, &frame, &status);

    assert!(prompt.contains(&format!("{BODY_MARKER} phase 1.")), "{prompt}");
    assert!(
        !prompt.contains(LOG_MARKER),
        "phase 1 must exclude the log through the `phase > 1` guard:\n{prompt}"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_proxied_initialize_creates_log_before_guarded_later_phase_body_includes_it() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage("l2-init-gen-phase-two");
    let (frame, status) = run_router(&staged, 2);
    let prompt = assert_generated_flow(&staged, &frame, &status);

    assert!(prompt.contains(&format!("{BODY_MARKER} phase 2.")), "{prompt}");
    assert!(
        prompt.contains(LOG_MARKER),
        "a later phase must include the log initialize wrote:\n{prompt}"
    );
}

/// Initialization shell actions are forbidden before preflight, including when
/// no approval handler is available. No shell effect or provider launch occurs.
#[test]
#[serial(level2_terminal)]
fn level2_direct_initialize_shell_has_no_effect_when_approval_is_unavailable() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage("l2-init-gen-unapproved");
    let effect = staged.fixture.workspace_path().join("initialize-effect.txt");
    let document = format!(
        "---\ninitialize:\n  stack:\n    - action:\n        - shell: \"touch {effect}\"\n---\n{BODY_MARKER}\n",
        effect = sh_quote(&effect.display().to_string()),
    );
    write(&staged.fixture.cwd().join("direct.md"), &document);

    let (frame, status) = run_claudine(&staged, "compose direct.md --claude");

    assert_eq!(status, "1", "an unapprovable initialize command must fail the run:\n{frame}");
    assert!(
        frame.contains("shell") && frame.contains("initialize"),
        "the refusal must name the forbidden initialization shell:\n{frame}"
    );
    assert!(
        !effect.exists(),
        "the unapproved initialize shell command ran before the approval gate:\n{frame}"
    );
    assert!(
        event_lines(&staged).is_empty() && !staged.prompt_file.exists(),
        "no provider may launch after a refused initialize command:\n{frame}"
    );
}

/// OQ1 Option A in a real terminal: a sequence keeps its static preflight, so a
/// prompt step whose own `initialize` would create an included file fails
/// before any step starts, and the pane shows the note telling the author to
/// create the file first ahead of the unchanged typed error.
#[test]
#[serial(level2_terminal)]
fn level2_sequence_preflight_note_renders_for_an_include_initialize_would_create() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let staged = stage("l2-init-gen-sequence");
    let cwd = staged.fixture.cwd();
    write(
        &cwd.join("target.md"),
        "---\ninitialize:\n  stack:\n    - action:\n        - ensure_file: generated/notes.md\n---\n\
         TARGET\n\n::file generated/notes.md\n",
    );
    write(
        &cwd.join("seq.md"),
        "---\nsequence:\n    - name: one\n      prompt: target.md\n---\nBody.\n",
    );

    let (frame, status) = run_claudine(&staged, "sequence seq.md -y --claude");
    // The note is one logical line; the pane soft-wraps it mid-word, so compare
    // with all whitespace removed.
    let squash = |text: &str| text.split_whitespace().collect::<String>();
    let flat = squash(&frame.replace('\u{2503}', " "));

    assert_eq!(status, "1", "sequence preflight must fail:\n{frame}");
    let note = flat
        .find(&squash("note: a sequence checks every prompt document before its first step runs"))
        .unwrap_or_else(|| panic!("the preflight note is missing from the pane:\n{frame}"));
    assert!(flat.contains(&squash("Create the file before starting the sequence.")), "{frame}");
    let error = flat
        .find("TransclusionError")
        .unwrap_or_else(|| panic!("the typed error is missing from the pane:\n{frame}"));
    assert!(note < error, "the note precedes the error block:\n{frame}");
    assert!(flat.contains(&squash("File not found: generated/notes.md")), "{frame}");
    assert!(
        !cwd.join("generated").exists() && event_lines(&staged).is_empty(),
        "no step, initialize, or provider may run:\n{frame}"
    );
}
