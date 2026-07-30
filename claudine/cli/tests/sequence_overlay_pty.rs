//! Level 1 PTY tests for `claudine sequence` interactive schema-property
//! collection.
//!
//! Split out of `level2_schema_prompt_pty.rs`: this binary owns the
//! sequence-overlay coverage (cross-step prompt deduplication, per-step
//! overlay satisfaction, status reporting, and the pre-prompt /
//! agent-resolution gate that must fire before any provider launches).
//! The schema-prompt and inline-compose coverage stays in
//! `level2_schema_prompt_pty.rs`. Shared PTY harness helpers live in
//! `common::pty`.
//!
//! ## Tier
//!
//! **Level 1, not Level 2**, despite the file's former `level2_` name. These
//! tests drive an `expectrl` pseudo-terminal, which is in-process I/O plumbing
//! — `test_toolkit::Level::L1` is defined as "in-process or PTY-based tests".
//! Level 2 means a *real terminal emulator* reached through
//! `biscuit-test-harness` (tmux/WezTerm/Kitty), where the emulator's own
//! capability handshake, folding, and SGR re-emission are part of what is under
//! test. A PTY reproduces none of that. Terminal-visible task-stream rendering
//! is covered at the real Level 2 in
//! `level2_sequence_task_stream_capture.rs`.
//!
//! The name carries no tier prefix, which is what puts them in `just test`:
//! `_test` and `_sanity` both filter out `level2_`/`level3_`/`browser_`/
//! `real_`/`slow_`, so any of those prefixes would have left this binary
//! running in no canonical recipe at all.
//!
//! Gating: `#![cfg(unix)]`, `require_level!(Level::L1, pty_available(), ...)`
//! so the test skips cleanly without a PTY and panics under
//! `BISCUIT_TEST_LEVEL_REQUIRED=1`.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test
//! ```

#![cfg(unix)]

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use expectrl::Session;
use expectrl::session::OsSession;
use std::fs;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Level, require_level};

mod common;
use common::pty::*;
use common::{augmented_path, pty_available, write_executable};

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
    let mut cmd = Command::new(cargo_bin("claudine"));
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
fn pty_sequence_prompt_dedupes_and_launches_all_steps() {
    // Both steps require `topic` but the schema only declares it once.
    // The interactive collector must prompt for `topic` EXACTLY ONCE
    // (dedupe), reuse the answer for every step, and only launch the
    // provider AFTER the prompt has been satisfied.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

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

    // Submit the answer once, after raw mode is confirmed. The deduper
    // should apply this value to every step that declared the same
    // missing property.
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
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
fn pty_sequence_step_overlay_satisfies_required_property() {
    // The reserved overlay key `state` is set to each step's generated
    // `step_state` **object** (`name`/`id`/`index`/`count`/…). A schema that
    // requires `state` is therefore satisfied by every step's overlay, so the
    // interactive prompt should NOT fire for `state` — only for the
    // genuinely-missing `topic`.
    //
    // The schema declares `state` as `object`, not `string`: the
    // string-coercion rule (`{{state}}` renders `state.name`) is a *render*
    // contract, so the body below still composes "Step alpha", but the value
    // being validated is the object itself.
    //
    // This exercises the review-5 contract that the per-step status
    // report must honor the per-step effective override map: `state`
    // must appear as Valid for every step (because the overlay supplies
    // it) even while the user is being prompted for the missing `topic`.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

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
            "  state: 'object(required)'\n",
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
    let pre = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
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
fn pty_sequence_status_report_honors_setter_supplied_required() {
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
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

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
    let mut cmd = Command::new(cargo_bin("claudine"));
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
    wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
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

// ============================================================================
// `claudine sequence` agent-resolution pre-prompt (review-4 High finding)
// ============================================================================
//
// A live sequence (no `--dry-run`, no explicit `--<provider>`) gates agent
// resolution on `stderr` only — the prompting/status channel — exactly like
// direct compose. On a terminal, a prompting state must emit the same styled
// pre-prompt message direct compose shows *before* the review screen renders:
//
//   - a scalar invalid `agent`        -> the imperative `Invalid Agent:` line,
//   - an all-uninstallable `agent` list -> the zero-installed-list breakdown.
//
// These drive `claudine sequence` (no provider flag) through a real PTY so the
// message is observed before the review UI. A third test redirects stdout to a
// file to prove the gate keys off `stderr`, not `stdout`: under the old
// `stdin && stdout` gate, `sequence doc.md > out.md` wrongly aborted; under the
// stderr gate it prompts.

/// Build a `claudine sequence <file>` command with NO explicit provider flag,
/// so the live agent-resolution gate runs. `PATH` is restricted to `bin_dir`
/// alone (not [`augmented_path`]) so only the staged stub providers count as
/// installed and the classified agent state is deterministic on any host.
fn sequence_command_no_provider(
    workspace_dir: &std::path::Path,
    bin_dir: &std::path::Path,
    md_file: &std::path::Path,
) -> Command {
    stage_default_config(workspace_dir);
    let mut cmd = Command::new(cargo_bin("claudine"));
    cmd.args(["sequence", md_file.to_str().unwrap()]);
    cmd.env("HOME", workspace_dir);
    cmd.env("PATH", bin_dir);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd.current_dir(workspace_dir);
    cmd
}

/// Stage a stub provider `slug` that records a launch by writing `marker_file`.
/// The agent-resolution tests assert this marker never appears, proving the
/// pre-prompt fires strictly before any provider session starts.
fn stage_provider_launch_stub(
    bin_dir: &std::path::Path,
    slug: &str,
    marker_file: &std::path::Path,
) {
    write_executable(
        &bin_dir.join(slug),
        &format!(
            "#!/bin/sh\necho launched > {marker}\nexit 0\n",
            marker = marker_file.display()
        ),
    );
}

#[test]
#[serial_test::serial(pty)]
fn pty_sequence_invalid_agent_shows_preprompt_before_review() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");
    stage_provider_launch_stub(&bin_dir, "goose", &marker);

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "agent: not-real\n",
            "sequence:\n",
            "  - alpha\n",
            "  - beta\n",
            "---\n",
            "Step {{state}}.\n",
        ),
    )
    .unwrap();

    let cmd = sequence_command_no_provider(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // The styled `Invalid Agent:` pre-prompt must render BEFORE the review
    // table. Under the old code the TTY branch jumped straight into the
    // review screen and never emitted this message.
    let pre = wait_for_marker(&mut session, "Invalid Agent", Duration::from_secs(10));
    let plain = common::strip_ansi(&pre);
    assert!(
        plain.contains("not-real"),
        "the invalid hint must be named in the pre-prompt; transcript:\n{plain}"
    );
    assert!(
        !marker.exists(),
        "no provider may launch before the agent-resolution prompt is shown; \
         transcript:\n{plain}"
    );

    // Cancel the review screen (Esc) so the child exits instead of blocking
    // on the picker. Gate the keystroke on raw mode like the schema tests.
    let _ = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    session.write_all(b"\x1b").expect("send Esc to cancel review");
    session.flush().ok();

    let _ = read_for(&mut session, Duration::from_secs(5));
    assert!(
        !marker.exists(),
        "a cancelled review must never launch a provider"
    );
}

#[test]
#[serial_test::serial(pty)]
fn pty_sequence_zero_installed_list_shows_preprompt_before_review() {
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");
    // A runnable provider must exist so the review picker has an option to
    // choose from once the zero-installed-list breakdown is shown.
    stage_provider_launch_stub(&bin_dir, "goose", &marker);

    let md_file = workspace.path().join("seq.md");
    // An all-invalid list resolves to zero installed providers regardless of
    // host, mirroring the L1 `sequence_dry_run_zero_installed_list_*` fixture.
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "agent: [not-real, also-fake]\n",
            "sequence:\n",
            "  - alpha\n",
            "  - beta\n",
            "---\n",
            "Step {{state}}.\n",
        ),
    )
    .unwrap();

    let cmd = sequence_command_no_provider(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // `installed/valid` is the single-token signature of the zero-installed
    // breakdown; it must render before the review table.
    let pre = wait_for_marker(&mut session, "installed/valid", Duration::from_secs(10));
    let plain = common::strip_ansi(&pre);
    assert!(
        !plain.contains("Invalid Agent"),
        "a list state must not render the single-invalid scalar message; \
         transcript:\n{plain}"
    );
    assert!(
        !marker.exists(),
        "no provider may launch before the agent-resolution prompt is shown; \
         transcript:\n{plain}"
    );

    let _ = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    session.write_all(b"\x1b").expect("send Esc to cancel review");
    session.flush().ok();

    let _ = read_for(&mut session, Duration::from_secs(5));
    assert!(
        !marker.exists(),
        "a cancelled review must never launch a provider"
    );
}

#[test]
#[serial_test::serial(pty)]
fn pty_sequence_stderr_tty_with_stdout_redirected_prompts() {
    // The core gate fix: `sequence doc.md > out.md` keeps `stderr` on the
    // terminal but redirects `stdout` to a file. The agent-resolution gate
    // keys off `stderr` only, so the prompting state must reach the review
    // screen (emitting the pre-prompt) rather than aborting with the no-TTY
    // `agent resolution failed` error the old `stdin && stdout` gate produced.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");
    stage_provider_launch_stub(&bin_dir, "goose", &marker);
    stage_default_config(workspace.path());

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "agent: not-real\n",
            "sequence:\n",
            "  - alpha\n",
            "---\n",
            "Step {{state}}.\n",
        ),
    )
    .unwrap();

    let out_file = workspace.path().join("out.md");
    let claudine_bin = cargo_bin("claudine");

    // Drive through `/bin/sh -c` so the shell, not expectrl, owns the `>`
    // redirect: the child claudine inherits stderr+stdin from the PTY but
    // stdout points at `out.md`.
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c").arg(format!(
        "'{}' sequence '{}' > '{}'",
        claudine_bin.display(),
        md_file.display(),
        out_file.display(),
    ));
    cmd.env("HOME", workspace.path());
    cmd.env("PATH", &bin_dir);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env_remove("NO_COLOR");
    cmd.env_remove("CLAUDINE_PLAIN");
    cmd.env_remove("CI");
    cmd.current_dir(workspace.path());

    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // The pre-prompt is written to stderr (the PTY) even though stdout is
    // redirected. Observing it proves the run reached the prompt path.
    let pre = wait_for_marker(&mut session, "Invalid Agent", Duration::from_secs(10));
    let plain = common::strip_ansi(&pre);
    assert!(
        !plain.contains("agent resolution failed"),
        "stderr-gated resolution must NOT abort when only stdout is redirected; \
         transcript:\n{plain}"
    );
    assert!(
        !marker.exists(),
        "no provider may launch before the agent-resolution prompt is shown; \
         transcript:\n{plain}"
    );

    let _ = wait_for_raw_mode(&mut session, pre, Duration::from_secs(10));
    session.write_all(b"\x1b").expect("send Esc to cancel review");
    session.flush().ok();

    let _ = read_for(&mut session, Duration::from_secs(5));
    assert!(
        !marker.exists(),
        "a cancelled review must never launch a provider"
    );
}

#[test]
#[serial_test::serial(pty)]
fn pty_sequence_auto_selectable_skips_review_and_launches() {
    // The review-5 High finding: auto-selectable states (`Selected` /
    // `ListOneInstalled`) must bypass the review screen on a TTY, exactly like
    // direct compose's `resolve_live_target_with_tty` returns before the picker.
    // A one-installed-list hint (`agent: [goose, gemini]` with only `goose`
    // staged) classifies as `ListOneInstalled`, so the provider must launch with
    // no alternate-screen review UI and no keyboard input.
    require_level!(Level::L1, pty_available(), "PTY (/dev/ptmx)");

    let workspace = tempdir().unwrap();
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let marker = workspace.path().join("launched.flag");
    stage_provider_launch_stub(&bin_dir, "goose", &marker);

    let md_file = workspace.path().join("seq.md");
    fs::write(
        &md_file,
        concat!(
            "---\n",
            "agent: [goose, gemini]\n",
            "sequence:\n",
            "  - alpha\n",
            "---\n",
            "Auto-select body.\n",
        ),
    )
    .unwrap();

    let cmd = sequence_command_no_provider(workspace.path(), &bin_dir, &md_file);
    let mut session: OsSession = Session::spawn(cmd).expect("spawn PTY session");

    // Drain the PTY until the stub records a launch, accumulating the full raw
    // transcript so the alternate-screen assertion below sees everything the
    // review UI would have emitted. No keystroke is ever sent.
    let stop = Instant::now() + Duration::from_secs(15);
    let mut transcript = String::new();
    while Instant::now() < stop {
        if marker.exists() {
            break;
        }
        transcript.push_str(&read_for(&mut session, Duration::from_millis(200)));
    }
    // Drain a final window so any trailing alt-screen bytes are captured even
    // if the marker landed before the picker would have rendered.
    transcript.push_str(&read_for(&mut session, Duration::from_millis(300)));

    assert!(
        marker.exists(),
        "an auto-selectable (one-installed-list) sequence must launch the \
         provider without prompting; transcript:\n{}",
        common::strip_ansi(&transcript)
    );
    assert!(
        !transcript.contains(ALT_SCREEN_ENTER),
        "auto-selectable states must bypass the review screen — the picker's \
         alternate-screen enter must never appear; transcript:\n{}",
        common::strip_ansi(&transcript)
    );
}

// ============================================================================
// `claudine inline-compose` interactive schema collection
// (2026-06-14-interactive, review-1 High finding)
// ============================================================================
//
// `inline-compose` has its own preparation path (`prepare_inline_with_schema`)
// and an extra interactive-closure gate that rejects providers which cannot
// capture the final assistant message. The schema-collection invariant —
// missing required values are collected BEFORE any provider session starts or
// the inline-unsupported diagnostic is reached — must therefore be verified
// for `inline-compose` specifically, not just `compose`.
//
// These tests use Codex, the one provider that supports interactive inline
// closure (`supports_interactive_inline_closure() == true`), so the run
// proceeds to a real provider launch after collection rather than the
// unsupported diagnostic. The stub captures Claudine's
// `--output-last-message <file>` write-back path exactly like the non-PTY
// `inline_compose_interactive_codex_uses_captured_last_message` test.
