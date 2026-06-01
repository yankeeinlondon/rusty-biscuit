//! Level 2 PTY tests for interactive schema-property collection.
//!
//! Addresses the High finding in `review-3.md`:
//!
//! > Required level: Level 1 PTY is the minimum for this terminal I/O
//! > behavior.
//!
//! The schema-aware interactive prompt fires only when stdin AND stderr
//! are both TTYs, `prompt_for_missing` is enabled (default), and
//! `--silent` is off. The Level 1 process tests in
//! `compose_schema_cli.rs` cover the non-TTY denial path; this file
//! drives the prompt through a real pseudo-terminal so we can verify:
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
//! Gating: `#![cfg(unix)]`, `require_level!(Level::L2, pty_available(),
//! ...)` so the test skips cleanly without a PTY and panics under
//! `BISCUIT_TEST_LEVEL_REQUIRED=2`.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test-l2
//! ```

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use expectrl::session::OsSession;
use expectrl::Session;
use std::fs;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, require_level};

mod common;
use common::{augmented_path, pty_available, write_executable};

/// Drain bytes from the PTY for a generous window so each `try_read`
/// call collects whatever the child has flushed so far. Returns the
/// concatenated transcript bytes as `String::from_utf8_lossy` so stray
/// ANSI fragments never panic.
fn read_for(session: &mut OsSession, total_deadline: Duration) -> String {
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    let deadline = Instant::now() + total_deadline;
    session.set_expect_timeout(Some(Duration::from_millis(150)));
    while Instant::now() < deadline {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&scratch[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Wait for a substring (after ANSI stripping) to appear in the cumulative
/// transcript, draining the PTY incrementally. Returns the full ANSI-bearing
/// transcript on success; panics with the accumulated transcript on
/// timeout for easier diagnosis.
fn wait_for_marker(session: &mut OsSession, marker: &str, deadline: Duration) -> String {
    let stop = Instant::now() + deadline;
    let mut transcript = String::new();
    let mut scratch = [0u8; 4096];
    session.set_expect_timeout(Some(Duration::from_millis(100)));
    while Instant::now() < stop {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                transcript.push_str(&String::from_utf8_lossy(&scratch[..n]));
                if common::strip_ansi(&transcript).contains(marker) {
                    return transcript;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
    panic!(
        "marker {marker:?} did not appear within {deadline:?}; transcript:\n{transcript}"
    );
}

/// Pre-stage a minimal claudine config at `$HOME/.claudine/config.json`.
///
/// Under a PTY, claudine detects an interactive stdin and runs the
/// first-run setup wizard when no user config exists. The wizard would
/// intercept any input we send to the schema prompt and block the test
/// indefinitely. Writing an empty JSON object satisfies the loader (every
/// `ClaudineConfig` field has a `#[serde(default)]` derivation) and
/// inherits the defaults — notably `prompt_for_missing = true`.
fn stage_default_config(home_dir: &std::path::Path) {
    let claudine_dir = home_dir.join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();
    fs::write(claudine_dir.join("config.json"), "{}").unwrap();
}

/// Build a fresh PTY-spawnable `Command` for `claudine compose --goose <file>`
/// with the workspace's `bin` dir on PATH and HOME set to the workspace so
/// `prompt_for_missing` reads the default (`true`) instead of any real
/// user config.
fn compose_command(workspace_dir: &std::path::Path, bin_dir: &std::path::Path, md_file: &std::path::Path) -> Command {
    stage_default_config(workspace_dir);
    let mut cmd = Command::new(cargo_bin!("claudine"));
    cmd.args(["compose", "--goose", md_file.to_str().unwrap()]);
    cmd.env("HOME", workspace_dir);
    cmd.env("PATH", augmented_path(bin_dir));
    // Give the TUI a deterministic terminal type with truecolor support
    // so the inquire/tui-chrome rendering path picks a consistent style.
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd.current_dir(workspace_dir);
    cmd
}

/// Stage a stub `goose` binary that records that the provider was
/// launched (so the test can prove the interactive prompt successfully
/// satisfied the schema and execution proceeded).
///
/// The stub writes the marker file FIRST and then exits without reading
/// stdin. Under PTY, the wrapper's stdin remains attached to the master
/// side, so a `cat > /dev/null` stub would block on EOF that never
/// arrives. Writing the marker first lets the test observe the launch
/// even if the wrapper still has stdin open.
fn stage_goose_stub(bin_dir: &std::path::Path, marker_file: &std::path::Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho 'launched' > {marker}\nexit 0\n",
            marker = marker_file.display()
        ),
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_schema_prompt_collects_string_and_launches_provider() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n  topic: 'string(required)'\n---\nPlan for {{topic}}.\n",
    )
    .unwrap();

    let cmd = compose_command(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // The prompt label is built from the property name plus type hint. Wait
    // for the label to render before sending input.
    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));

    // Type a value and submit. `\r` is the carriage-return byte that
    // crossterm's raw-mode reader translates into `KeyCode::Enter`.
    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    // Drain until the child exits. The stub writes the marker file once
    // it has been spawned, so polling for that file is a robust signal.
    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = pre;
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }

    assert!(
        marker.exists(),
        "provider stub should have been launched after the prompt submitted; \
         the interactive collection loop must have satisfied `topic`.\n\
         transcript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_schema_prompt_collects_enum_selection() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\n---\nPlan with {{tier}}.\n",
    )
    .unwrap();

    let cmd = compose_command(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    wait_for_marker(&mut session, "tier", Duration::from_secs(10));

    // ChooseOne defaults to the first option highlighted. Pressing Enter
    // submits the current selection (`small`).
    session.write_all(b"\r").expect("submit default enum choice");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        let _ = read_for(&mut session, Duration::from_millis(200));
    }

    assert!(
        marker.exists(),
        "provider stub should have been launched after the enum selection \
         submitted the default choice"
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_schema_prompt_collects_boolean() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n  ready: 'boolean(required)'\n---\nReady = {{ready}}.\n",
    )
    .unwrap();

    let cmd = compose_command(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    wait_for_marker(&mut session, "ready", Duration::from_secs(10));

    // BooleanSwitch defaults to false; Enter submits the current state.
    session.write_all(b"\r").expect("submit boolean default");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        let _ = read_for(&mut session, Duration::from_millis(200));
    }

    assert!(
        marker.exists(),
        "provider stub should have been launched after the boolean prompt \
         submitted its default value"
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_schema_prompt_number_retries_on_invalid_input() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n  count: 'number(required; integer)'\n---\nCount = {{count}}.\n",
    )
    .unwrap();

    let cmd = compose_command(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    wait_for_marker(&mut session, "count", Duration::from_secs(10));

    // First, submit a non-numeric value. The parse-and-retry loop in
    // `collect_number` must re-prompt with an inline validation error
    // and keep the previous buffer.
    session.write_all(b"not-a-number\r").expect("write bad value");
    session.flush().ok();

    // Wait for the re-prompt to render the validation error. The error
    // text is built from the parse_number helper as `\`<raw>\` is not a
    // valid integer`.
    wait_for_marker(&mut session, "not a valid integer", Duration::from_secs(5));

    // Clear the previous buffer so the next submission contains only the
    // good value. Backspace (`\x7f` DEL) is what crossterm reports as
    // `KeyCode::Backspace` in raw mode; send enough to wipe
    // `not-a-number`.
    for _ in 0.."not-a-number".len() {
        session.write_all(&[0x7f]).expect("backspace");
    }
    session.write_all(b"42\r").expect("write good value");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(15);
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        let _ = read_for(&mut session, Duration::from_millis(200));
    }

    assert!(
        marker.exists(),
        "provider stub should have launched after the retry loop accepted \
         the valid integer"
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_schema_silent_suppresses_prompt_under_tty() {
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n$schema:\n  topic: 'string(required)'\n---\nPlan for {{topic}}.\n",
    )
    .unwrap();

    // Same as `compose_command`, but adds `--silent`. With both stdin and
    // stderr as TTYs, the prompt would normally fire; `--silent` must
    // suppress it and surface the typed `MissingProperties` error
    // instead.
    stage_default_config(workspace.path());
    let mut cmd = Command::new(cargo_bin!("claudine"));
    cmd.args([
        "compose",
        "--goose",
        "--silent",
        md_file.to_str().unwrap(),
    ]);
    cmd.env("HOME", workspace.path());
    cmd.env("PATH", augmented_path(&bin_dir));
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd.current_dir(workspace.path());

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
#[serial_test::serial(pty)]
fn level2_pty_schema_status_does_not_report_templated_enum_as_invalid() {
    // Regression test for review-4 medium finding. When the schema status
    // report renders before Interactive Mode prompts for a missing
    // required property, it must not flag a separately-templated
    // (provider-derived) enum value as Invalid. The preflight + prepare
    // pipeline composes `{{ env.AGENT }}` into the resolved provider slug,
    // so the status display must agree.
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  runtime_agent: 'enum(goose; required)'\n",
            "  topic: 'string(required)'\n",
            "runtime_agent: '{{ env.AGENT }}'\n",
            "---\n",
            "Plan for {{topic}} via {{runtime_agent}}.\n",
        ),
    )
    .unwrap();

    let cmd = compose_command(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // Drive the prompt: wait for the missing-required label, type a
    // value, and submit. The provider-derived `runtime_agent` value is
    // never user-supplied — the test asserts it is NOT flagged invalid
    // in the report transcript.
    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));
    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    // Drain until the stub records its launch.
    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = pre;
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }

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
// `claudine sequence` Interactive Mode (review-5 High finding)
// ============================================================================
//
// The sequence subcommand drives the same `biscuit-tui` prompt loop as
// direct `compose`, but with two extra contracts:
//
// 1. Missing required properties are deduplicated across steps — the user
//    is prompted once and the answer applies to every step that needs it.
// 2. Prompting happens BEFORE any provider session is launched, so a
//    malformed sequence never spawns the agent.
//
// These tests drive `claudine sequence` through a real pseudo-terminal so
// both contracts can be observed end-to-end.

/// Build a fresh PTY-spawnable `Command` for `claudine sequence --goose <file>`
/// with the workspace's `bin` dir on PATH and HOME set to the workspace so
/// `prompt_for_missing` reads the default (`true`) instead of any real
/// user config.
fn sequence_command(
    workspace_dir: &std::path::Path,
    bin_dir: &std::path::Path,
    md_file: &std::path::Path,
) -> Command {
    stage_default_config(workspace_dir);
    let mut cmd = Command::new(cargo_bin!("claudine"));
    cmd.args(["sequence", "--goose", md_file.to_str().unwrap()]);
    cmd.env("HOME", workspace_dir);
    cmd.env("PATH", augmented_path(bin_dir));
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd.current_dir(workspace_dir);
    cmd
}

/// Stage a stub `goose` binary that records each invocation in a counter
/// file. The stub writes the count atomically (last-writer-wins) and
/// exits successfully so the wrapper continues to the next sequence step.
fn stage_goose_counter_stub(bin_dir: &std::path::Path, count_path: &std::path::Path) {
    write_executable(
        &bin_dir.join("goose"),
        &format!(
            "#!/bin/sh\ncount=0\nif [ -f {count} ]; then\n  IFS= read -r count < {count}\nfi\ncount=$((count + 1))\nprintf '%s' \"$count\" > {count}\nexit 0\n",
            count = count_path.display()
        ),
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_sequence_prompt_dedupes_and_launches_all_steps() {
    // Both steps require `topic` but the schema only declares it once.
    // The interactive collector must prompt for `topic` EXACTLY ONCE
    // (dedupe), reuse the answer for every step, and only launch the
    // provider AFTER the prompt has been satisfied.
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    stage_goose_counter_stub(&bin_dir, &count_path);

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "sequence:\n",
            "  - alpha\n",
            "  - beta\n",
            "---\n",
            "Step {{state}} about {{topic}}.\n",
        ),
    )
    .unwrap();

    let cmd = sequence_command(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // The label is built from the property name plus type hint. Wait for
    // the prompt to render before sending input.
    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));

    // Sanity assertion: before we type, the stub must NOT have been
    // launched. This proves the prompt runs strictly before any provider
    // session starts.
    assert!(
        !count_path.exists(),
        "provider stub was launched before the prompt was satisfied; \
         transcript so far:\n{}",
        common::strip_ansi(&pre)
    );

    // Submit the answer once. The deduper should apply this value to
    // every step that declared the same missing property.
    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();

    // Drain until BOTH steps have launched. The counter file is created
    // by the stub on first invocation; the count must reach 2.
    let stop = Instant::now() + Duration::from_secs(20);
    let mut transcript = pre;
    let mut final_count = String::new();
    while Instant::now() < stop {
        if let Ok(content) = fs::read_to_string(&count_path)
            && content.trim() == "2"
        {
            final_count = content;
            break;
        }
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }

    assert!(
        count_path.exists(),
        "provider stub was never launched; transcript:\n{}",
        common::strip_ansi(&transcript)
    );
    let count = if final_count.is_empty() {
        fs::read_to_string(&count_path).unwrap_or_default()
    } else {
        final_count
    };
    assert_eq!(
        count.trim(),
        "2",
        "expected both sequence steps to launch after the deduped prompt was \
         satisfied; counter content: {count:?}; transcript:\n{}",
        common::strip_ansi(&transcript)
    );

    // The prompt label should appear exactly once in the transcript even
    // though two steps declared the same missing property. The
    // status-report rendering also includes the property name in the
    // per-step header, so we look for the prompt-specific label glyph
    // `string` to count actual prompt instances. We tolerate
    // re-rendering by the TUI (raw-mode cursor moves can re-emit the
    // label) but the count must be small — never proportional to the
    // step count.
    let plain = common::strip_ansi(&transcript);
    let prompt_label_hits = plain.matches("topic (string)").count();
    assert!(
        prompt_label_hits <= 4,
        "expected the deduped prompt to render once (with TUI re-render \
         tolerance up to 4); saw {prompt_label_hits} hits in transcript:\n{plain}"
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_sequence_step_overlay_satisfies_required_property() {
    // The reserved overlay key `state` is set to each step's raw value
    // (the step name for string-form steps). A schema that requires
    // `state` is therefore satisfied by every step's overlay, so the
    // interactive prompt should NOT fire for `state` — only for the
    // genuinely-missing `topic`.
    //
    // This exercises the review-5 contract that the per-step status
    // report must honor the per-step effective override map: `state`
    // must appear as Valid for every step (because the overlay supplies
    // it) even while the user is being prompted for the missing `topic`.
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let prompts_path = workspace.path().join("all-prompts.txt");

    // Goose stub: append the `-t <prompt>` argument from each invocation
    // to a single file so the test can inspect each step's composed
    // prompt afterward.
    write_executable(
        &bin_dir.join("goose"),
        r#"#!/bin/sh
prev=""
for arg in "$@"; do
  if [ "$prev" = "-t" ]; then
    {
      printf -- '--- invocation ---\n'
      printf '%s\n' "$arg"
    } >> "$CLAUDINE_PROMPTS_FILE"
  fi
  prev="$arg"
done
exit 0
"#,
    );

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  state: 'string(required)'\n",
            "  topic: 'string(required)'\n",
            "sequence:\n",
            "  - alpha\n",
            "  - beta\n",
            "---\n",
            "Step {{state}}: topic={{topic}}\n",
        ),
    )
    .unwrap();

    let mut cmd = sequence_command(workspace.path(), &bin_dir, &md_file);
    cmd.env("CLAUDINE_PROMPTS_FILE", &prompts_path);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));
    let plain_pre = common::strip_ansi(&pre);

    // The status report must NOT flag `state` as Missing — the overlay
    // supplies it per step. If this assertion fires, the pre-prompt
    // status report was built without the per-step effective override
    // map (the review-5 medium finding).
    let state_missing_line = plain_pre
        .lines()
        .find(|line| line.contains("state") && line.contains("was not defined but is required"));
    assert!(
        state_missing_line.is_none(),
        "`state` must NOT appear as Missing in the pre-prompt status report \
         when the per-step overlay supplies it; offending line: {state_missing_line:?}\n\
         transcript:\n{plain_pre}"
    );

    // Drive the prompt to completion. The deduper should fire `topic`
    // exactly once and reuse the answer for both steps.
    session.write_all(b"collected\r").expect("write topic value");
    session.flush().ok();

    let stop = Instant::now() + Duration::from_secs(20);
    let mut transcript = pre;
    while Instant::now() < stop {
        if prompts_path.exists()
            && fs::read_to_string(&prompts_path)
                .map(|s| s.matches("invocation").count() >= 2)
                .unwrap_or(false)
        {
            break;
        }
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }

    let captured = fs::read_to_string(&prompts_path).unwrap_or_default();
    // Each step's `state` came from its own overlay so the prompts must
    // contain the step name verbatim, and both steps must show the
    // collected `topic`.
    assert!(
        captured.contains("Step alpha: topic=collected"),
        "step 1 must compose its own overlay-supplied state (alpha) with the \
         deduped collected topic; captured:\n{captured}\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
    assert!(
        captured.contains("Step beta: topic=collected"),
        "step 2 must compose its own overlay-supplied state (beta) with the \
         deduped collected topic; captured:\n{captured}\ntranscript:\n{}",
        common::strip_ansi(&transcript)
    );
}

#[test]
#[serial_test::serial(pty)]
fn level2_pty_sequence_status_report_honors_setter_supplied_required() {
    // The schema requires TWO properties, `topic` and `tier`. The user
    // supplies `tier` via a CLI `--set` / shorthand setter, so the
    // interactive prompt should only fire for `topic`. The status report
    // shown before the prompt must reflect this — `tier` must NOT be
    // listed as Missing.
    //
    // Regression target (review-5 medium): the previous implementation
    // called `build_schema_status_report(&source, None)`, discarding both
    // user `--set` overrides and the per-step overlay, so `tier` would
    // appear as missing in the pre-prompt diagnostic even though every
    // step's prepare step had already accepted the supplied value.
    require_level!(Level::L2, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");

    stage_goose_stub(&bin_dir, &marker);

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "$schema:\n",
            "  topic: 'string(required)'\n",
            "  tier: 'string(required)'\n",
            "sequence:\n",
            "  - alpha\n",
            "  - beta\n",
            "---\n",
            "Step {{state}}: topic={{topic}} tier={{tier}}\n",
        ),
    )
    .unwrap();

    // Same as `sequence_command`, but appends a shorthand setter
    // `tier=gold` after the file argument so only `topic` should remain
    // missing once the overlay/setter merge has run.
    stage_default_config(workspace.path());
    let mut cmd = Command::new(cargo_bin!("claudine"));
    cmd.args([
        "sequence",
        "--goose",
        md_file.to_str().unwrap(),
        "tier=gold",
    ]);
    cmd.env("HOME", workspace.path());
    cmd.env("PATH", augmented_path(&bin_dir));
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd.current_dir(workspace.path());

    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    let pre = wait_for_marker(&mut session, "topic", Duration::from_secs(10));
    let plain_pre = common::strip_ansi(&pre);

    // Targeted assertion: the per-property status line for `tier` must
    // NOT carry the "was not defined but is required" missing-state body
    // from `render_required_line`. If it does, the status report was
    // built without the CLI setter overrides applied.
    let tier_missing_line = plain_pre
        .lines()
        .find(|line| line.contains("tier") && line.contains("was not defined but is required"));
    assert!(
        tier_missing_line.is_none(),
        "setter-supplied `tier` must NOT appear as Missing in the pre-prompt \
         status report; offending line: {tier_missing_line:?}\nfull transcript:\n{plain_pre}"
    );

    // Drive the prompt to completion so the test exits cleanly.
    session.write_all(b"async\r").expect("write topic value");
    session.flush().ok();
    let stop = Instant::now() + Duration::from_secs(15);
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        let _ = read_for(&mut session, Duration::from_millis(200));
    }
}
