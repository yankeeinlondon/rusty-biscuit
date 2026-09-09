//! Level 1 PTY tests for interactive schema-property collection.
//!
//! Addresses the High finding in `review-3.md`:
//!
//! > Required level: Level 1 PTY is the minimum for this terminal I/O
//! > behavior.
//!
//! The schema-aware interactive prompt fires only when stdin AND stderr
//! are both TTYs, `prompt_for_missing` is enabled (default), and
//! `--silent` is off. The process tests in `compose_schema_cli.rs` cover
//! the non-TTY denial path; this file drives the prompt through a
//! pseudo-terminal so we can verify:
//!
//! - The widget appears with the correct property label.
//! - Typing a value and pressing Enter satisfies the missing required
//!   property, and the provider stub is launched.
//! - Enum selection (default-highlighted first option) submits the
//!   first member on Enter.
//! - Numeric inputs surface a parse-and-retry loop when the first
//!   value is non-numeric.
//! - `--silent` suppresses the prompt under TTY and emits the typed
//!   `MissingProperties` report instead.
//!
//! This binary owns the direct-`compose` schema-prompt coverage and the
//! interactive `inline-compose` collection coverage. The sequence-overlay
//! coverage lives in `sequence_overlay_pty.rs`; shared PTY harness
//! helpers live in `common::pty`.
//!
//! ## Tier
//!
//! **Level 1.** `expectrl` opens `/dev/ptmx` and every byte the child reads is
//! manufactured by the test, so no terminal emulator participates. Level 2
//! means a real emulator session reached through `biscuit-test-harness`, where
//! the emulator's own capability handshake and rendering are part of what is
//! under test.
//!
//! Gating: `#![cfg(unix)]`, `require_level!(Level::L1, pty_available(),
//! ...)` so the test skips cleanly without a PTY.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test
//! ```

#![cfg(unix)]

use expectrl::Session;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use test_toolkit::{Level, require_level};

mod common;
use common::pty::*;
use common::{CliProcessFixture, pty_available, write_executable};

/// Build a fresh PTY-spawnable `claudine <args>` command against the fixture's
/// own `HOME`, so `prompt_for_missing` reads the default (`true`) instead of any
/// real user config.
fn claudine_command(fixture: &CliProcessFixture, args: &[&str]) -> Command {
    stage_default_config(fixture.home());
    // `expectrl` needs a live `std::process::Command`; the builder's raw
    // surface hands one over carrying the same policy.
    let mut cmd = fixture.command_std();
    cmd.args(args);
    // Give the TUI a deterministic terminal type with truecolor support
    // so the inquire/biscuit-tui rendering path picks a consistent style.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd
}

/// The `claudine compose --goose <file>` shape most tests here drive.
fn compose_command(fixture: &CliProcessFixture, md_file: &Path) -> Command {
    claudine_command(
        fixture,
        &["compose", "--goose", md_file.to_str().unwrap()],
    )
}

/// Write `contents` as the plan document inside the fixture's launch area.
fn write_plan(fixture: &CliProcessFixture, contents: &str) -> std::path::PathBuf {
    let md_file = fixture.cwd().join("plan.md");
    fs::write(&md_file, contents).unwrap();
    md_file
}

/// A fixture with a `goose` stub staged, plus the marker path the stub writes
/// when it is launched.
fn staged_fixture() -> (CliProcessFixture, std::path::PathBuf) {
    let fixture = CliProcessFixture::named("level1-schema-prompt-pty");
    let marker = fixture.cwd().join("launched.flag");
    stage_goose_stub(fixture.bin_dir(), &marker);
    (fixture, marker)
}

/// Drain the PTY until `marker` appears on disk or the deadline elapses,
/// accumulating the transcript.
fn drain_until_marker(
    session: &mut OsSession,
    marker: &Path,
    seed: String,
    deadline: Duration,
) -> String {
    let stop = Instant::now() + deadline;
    let mut transcript = seed;
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(session, Duration::from_millis(200)));
    }
    transcript
}

#[test]
fn level1_pty_schema_prompt_collects_string_and_launches_provider() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        "---\n$schema:\n  topic: 'string(required)'\n---\nPlan for {{topic}}.\n",
    );

    let cmd = compose_command(&fixture, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // The property name appears in the pre-prompt status report. Wait for
    // it, then block until raw mode is on before sending input (see
    // `wait_for_raw_mode`).
    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));

    // Type a value and submit. `\r` is the carriage-return byte that
    // crossterm's raw-mode reader translates into `KeyCode::Enter`.
    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    // Drain until the child exits. The stub writes the marker file once
    // it has been spawned, so polling for that file is a robust signal.
    let transcript = drain_until_marker(&mut session, &marker, pre, Duration::from_secs(15));

    assert!(
        marker.exists(),
        "provider stub should have been launched after the prompt submitted; \
         the interactive collection loop must have satisfied `topic`.\n\
         transcript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
fn level1_pty_schema_prompt_collects_enum_selection() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\n---\nPlan with {{tier}}.\n",
    );

    let cmd = compose_command(&fixture, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "tier", Duration::from_secs(10));
    wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));

    // ChooseOne defaults to the first option highlighted. Pressing Enter
    // submits the current selection (`small`).
    session.write_all(b"\r").expect("submit default enum choice");
    session.flush().ok();

    drain_until_marker(
        &mut session,
        &marker,
        String::new(),
        Duration::from_secs(15),
    );

    assert!(
        marker.exists(),
        "provider stub should have been launched after the enum selection \
         submitted the default choice"
    );
}

#[test]
fn level1_pty_schema_prompt_collects_boolean() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        "---\n$schema:\n  ready: 'boolean(required)'\n---\nReady = {{ready}}.\n",
    );

    let cmd = compose_command(&fixture, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "ready", Duration::from_secs(10));
    wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));

    // BooleanSwitch defaults to false; Enter submits the current state.
    session.write_all(b"\r").expect("submit boolean default");
    session.flush().ok();

    drain_until_marker(
        &mut session,
        &marker,
        String::new(),
        Duration::from_secs(15),
    );

    assert!(
        marker.exists(),
        "provider stub should have been launched after the boolean prompt \
         submitted its default value"
    );
}

#[test]
fn level1_pty_schema_prompt_number_retries_on_invalid_input() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        "---\n$schema:\n  count: 'number(required; integer)'\n---\nCount = {{count}}.\n",
    );

    let cmd = compose_command(&fixture, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "count", Duration::from_secs(10));
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));

    // First, submit a non-numeric value. The parse-and-retry loop in
    // `collect_number` must reject it and re-prompt, keeping the previous
    // buffer.
    session.write_all(b"not-a-number\r").expect("write bad value");
    session.flush().ok();

    // Wait for the prompt to re-enter raw mode: a fresh `run_standalone`
    // iteration is the deterministic proof the invalid value was rejected
    // and `collect_number` looped. (Scraping the re-rendered validation
    // error glyphs is unreliable here — a bare PTY has no emulator to
    // answer the inline viewport's cursor probe with a real position, so
    // the error/hint rows cannot be laid out deterministically.)
    let _ = wait_for_raw_mode_reentry(&mut session, pre, Duration::from_secs(5));

    // Clear the previous buffer so the next submission contains only the
    // good value. Backspace (`\x7f` DEL) is what crossterm reports as
    // `KeyCode::Backspace` in raw mode; send enough to wipe
    // `not-a-number`.
    for _ in 0.."not-a-number".len() {
        session.write_all(&[0x7f]).expect("backspace");
    }
    session.write_all(b"42\r").expect("write good value");
    session.flush().ok();

    drain_until_marker(
        &mut session,
        &marker,
        String::new(),
        Duration::from_secs(15),
    );

    assert!(
        marker.exists(),
        "provider stub should have launched after the retry loop accepted \
         the valid integer"
    );
}

#[test]
fn level1_pty_schema_silent_suppresses_prompt_under_tty() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        "---\n$schema:\n  topic: 'string(required)'\n---\nPlan for {{topic}}.\n",
    );

    // With both stdin and stderr as TTYs, the prompt would normally fire;
    // `--silent` must suppress it and surface the typed `MissingProperties`
    // error instead.
    let cmd = claudine_command(
        &fixture,
        &["compose", "--goose", "--silent", md_file.to_str().unwrap()],
    );

    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");
    let transcript = read_for(&mut session, Duration::from_secs(8));
    let plain = common::strip_ansi(&transcript);

    assert!(
        !marker.exists(),
        "provider stub must NOT be launched when `--silent` denies the prompt; \
         transcript:\n{plain}"
    );
    // The typed CompositionError surface must appear: --silent denies
    // interactive collection, so the missing-required surfaces as the
    // hard error.
    assert!(
        plain.to_lowercase().contains("missing"),
        "expected a missing-properties report on stderr when --silent denied \
         the interactive prompt; transcript:\n{plain}"
    );
    assert!(
        plain.contains("topic"),
        "expected the missing property name `topic` in the report; \
         transcript:\n{plain}"
    );
}

#[test]
fn level1_pty_schema_status_does_not_report_templated_enum_as_invalid() {
    // Regression test for review-4 medium finding. When the schema status
    // report renders before Interactive Mode prompts for a missing
    // required property, it must not flag a separately-templated
    // (provider-derived) enum value as Invalid. The preflight + prepare
    // pipeline composes `{{ env.AGENT }}` into the resolved provider slug,
    // so the status display must agree.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        concat!(
            "---\n",
            "$schema:\n",
            "  runtime_agent: 'enum(goose; required)'\n",
            "  topic: 'string(required)'\n",
            "runtime_agent: '{{ env.AGENT }}'\n",
            "---\n",
            "Plan for {{topic}} via {{runtime_agent}}.\n",
        ),
    );

    let cmd = compose_command(&fixture, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // Drive the prompt: wait for the missing-required label, type a
    // value, and submit. The provider-derived `runtime_agent` value is
    // never user-supplied — the test asserts it is NOT flagged invalid
    // in the report transcript.
    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    let transcript = drain_until_marker(&mut session, &marker, pre, Duration::from_secs(15));

    assert!(
        marker.exists(),
        "provider stub should have launched after the prompt satisfied `topic`; \
         transcript:\n{}",
        common::strip_ansi(&transcript)
    );

    let plain = common::strip_ansi(&transcript);
    // The "was defined but with the wrong type" phrasing is the
    // Invalid-state body of `render_required_line`. If it appears
    // anywhere paired with `runtime_agent`, the status report is
    // misrepresenting the provider-derived templated value.
    let mentions_runtime_agent_as_invalid = plain.lines().any(|line| {
        let lower = line.to_lowercase();
        lower.contains("runtime_agent") && lower.contains("wrong type")
    });
    assert!(
        !mentions_runtime_agent_as_invalid,
        "status report must not mark templated `runtime_agent` as Invalid; \
         transcript:\n{plain}"
    );
}

// ============================================================================
// Session-interactivity independence (2026-06-14-interactive Phase 4)
// ============================================================================
//
// The schema collection gate depends only on the four `InteractiveSchemaOptions`
// signals and must run before the provider session launches, regardless of how
// the resolved session interactivity value is derived.

#[test]
fn level1_pty_schema_prompt_precedes_provider_launch_with_interactive_flag() {
    // `compose -i` requests an interactive session via CLI flag. The missing
    // required property must still be collected before the provider stub
    // launches.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        "---\n$schema:\n  topic: 'string(required)'\n---\nPlan for {{topic}}.\n",
    );

    let cmd = claudine_command(
        &fixture,
        &["compose", "-i", "--goose", md_file.to_str().unwrap()],
    );

    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));

    // Ordering guarantee: the prompt has rendered but no input has been sent,
    // so the provider session must NOT have started yet. This is the assertion
    // that proves schema collection precedes provider launch — without it a
    // regression that launched immediately after rendering would still pass.
    assert!(
        !marker.exists(),
        "provider launched before the `-i` schema prompt was satisfied; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));

    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    let transcript = drain_until_marker(&mut session, &marker, pre, Duration::from_secs(15));

    assert!(
        marker.exists(),
        "provider stub should have launched after the `-i` prompt submitted; \
         transcript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
fn level1_pty_schema_prompt_precedes_provider_launch_with_frontmatter_interactive() {
    // A document with `interactive: true` in frontmatter selects interactive
    // session mode without a CLI flag. Schema collection must still complete
    // before the provider session starts.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "interactive: true\n",
            "---\n",
            "Plan for {{topic}}.\n",
        ),
    );

    let cmd = compose_command(&fixture, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));

    // Ordering guarantee: the prompt has rendered but no input has been sent,
    // so the frontmatter-driven interactive session must NOT have launched the
    // provider yet. This proves schema collection precedes provider launch.
    assert!(
        !marker.exists(),
        "provider launched before the frontmatter-interactive schema prompt was \
         satisfied; transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));

    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    let transcript = drain_until_marker(&mut session, &marker, pre, Duration::from_secs(15));

    assert!(
        marker.exists(),
        "provider stub should have launched after frontmatter-driven interactive \
         prompt submitted; transcript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
fn level1_pty_schema_prompt_appears_even_when_no_interactive_overrides_frontmatter() {
    // `--no-interactive` overrides a document's `interactive: true` frontmatter,
    // so the resolved session mode is non-interactive. The schema collection
    // prompt must still appear under a TTY because the collection gate is
    // independent of the resolved session mode. Adding `--timeout` proves the
    // resolved mode is non-interactive (it would be rejected in interactive mode).
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let (fixture, marker) = staged_fixture();
    let md_file = write_plan(
        &fixture,
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "interactive: true\n",
            "---\n",
            "Plan for {{topic}}.\n",
        ),
    );

    let cmd = claudine_command(
        &fixture,
        &[
            "compose",
            "--no-interactive",
            "--timeout",
            "30s",
            "--goose",
            md_file.to_str().unwrap(),
        ],
    );

    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));

    // Ordering guarantee: the prompt has rendered but no input has been sent,
    // so the provider must NOT have launched yet — even though the resolved
    // session mode is non-interactive, collection still precedes launch.
    assert!(
        !marker.exists(),
        "provider launched before the `--no-interactive` schema prompt was \
         satisfied; transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));

    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    let transcript = drain_until_marker(&mut session, &marker, pre, Duration::from_secs(15));

    assert!(
        marker.exists(),
        "provider stub should have launched after `--no-interactive` prompt \
         submitted; the schema prompt must fire even when the resolved session \
         mode is non-interactive.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

/// Stage a Codex stub for interactive inline composition. It records its
/// launch by writing `marker_file`, then satisfies Claudine's
/// `--output-last-message <path>` capture by writing a replacement body
/// there. Recording the launch first lets the test observe it even though
/// the wrapper keeps the PTY stdin attached.
fn stage_codex_inline_stub(bin_dir: &Path, marker_file: &Path) {
    write_executable(
        &bin_dir.join("codex"),
        &format!(
            "#!/bin/sh\necho 'launched' > {marker}\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"--output-last-message\" ]; then\n    shift\n    printf 'Inline body from codex\\n' > \"$1\"\n    exit 0\n  fi\n  shift\ndone\nexit 0\n",
            marker = marker_file.display()
        ),
    );
}

/// Drive a single `inline-compose` interactive run through the PTY: wait for
/// the schema prompt, assert collection has not yet launched the provider nor
/// emitted the inline-unsupported diagnostic, submit the value, and confirm
/// the provider launches afterward.
fn drive_inline_compose_collection(cmd: Command, marker: &Path) {
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // The schema prompt renders the property name before raw mode is entered.
    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));

    // Collection must precede BOTH the provider launch and the inline-compose
    // interactive-unsupported diagnostic. Neither may have happened yet.
    assert!(
        !marker.exists(),
        "provider launched before the schema prompt was satisfied; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );
    assert!(
        !common::strip_ansi(&pre)
            .to_lowercase()
            .contains("not supported"),
        "inline-unsupported diagnostic appeared before schema collection completed; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    let transcript = drain_until_marker(&mut session, marker, pre, Duration::from_secs(15));

    assert!(
        marker.exists(),
        "Codex inline-compose provider should have launched after the schema \
         prompt submitted; collection must complete before launch.\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
fn level1_pty_inline_compose_interactive_flag_collects_before_launch() {
    // `inline-compose -i --codex` requests an interactive session via flag.
    // The missing required `topic` must be collected before Codex launches.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-schema-prompt-pty");
    let marker = fixture.cwd().join("launched.flag");
    stage_codex_inline_stub(fixture.bin_dir(), &marker);

    let md_file = write_plan(
        &fixture,
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "prompt: Generate notes about {{topic}}.\n",
            "---\n",
            "Original body.\n",
        ),
    );

    let cmd = claudine_command(
        &fixture,
        &["inline-compose", "-i", "--codex", md_file.to_str().unwrap()],
    );

    drive_inline_compose_collection(cmd, &marker);
}

#[test]
fn level1_pty_inline_compose_frontmatter_interactive_collects_before_launch() {
    // `interactive: true` frontmatter selects an interactive session for
    // `inline-compose` with no CLI flag. The missing required `topic` must
    // still be collected before Codex launches.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let fixture = CliProcessFixture::named("level1-schema-prompt-pty");
    let marker = fixture.cwd().join("launched.flag");
    stage_codex_inline_stub(fixture.bin_dir(), &marker);

    let md_file = write_plan(
        &fixture,
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "interactive: true\n",
            "prompt: Generate notes about {{topic}}.\n",
            "---\n",
            "Original body.\n",
        ),
    );

    let cmd = claudine_command(
        &fixture,
        &["inline-compose", "--codex", md_file.to_str().unwrap()],
    );

    drive_inline_compose_collection(cmd, &marker);
}
