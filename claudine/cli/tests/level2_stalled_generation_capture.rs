//! Level 2 real-terminal capture test for the visible stalled-generation
//! ("live-but-dead guard") output.
//!
//! ## What this covers (the gap unit tests do not)
//!
//! The stalled-generation backstop's terminal-visible output was previously
//! only unit-tested (Level 1):
//! - `claudine/lib/src/stream/logs/opencode/reasoning.rs`
//!   (`stalled_generation_emits_agent_native_terminal_event_with_safe_context`)
//!   proves the emitted [`SemanticEvent::Error`] shape (label / classification /
//!   safe context) against an injected `now`.
//! - `claudine/cli/src/commands/wrap/exec/termination.rs`
//!   (`apply_early_termination_stalled_generation_sets_summary_fields_and_context`)
//!   proves the summary/error-message mapping.
//!
//! Nothing proved that the `AgentNative` error block — the `Agent Error` label
//! and the `stalled generation: N attempts over Ms ...` message with its safe
//! context — actually **renders** during a real wrapped OpenCode run. This
//! closes that gap end-to-end: it drives the real `claudine opencode` wrapper
//! inside a tmux pane against a fake OpenCode provider whose stderr emits the
//! retry-churn fingerprint (repeated streamed `llm_call_start` with no
//! progress), forces the guard to trip, and asserts the rendered pane.
//!
//! ## Level: L2 (rendering), not L3 (input encoder)
//!
//! This test verifies what the terminal *displayed* — the rendered
//! stalled-generation error block — not what bytes a keypress *encodes*, so it
//! is L2. No OS keyboard input participates; the trip is driven entirely by the
//! fake provider's structured stderr stream. (Level 3 is not applicable: this
//! feature does not depend on OS keyboard input encoding.)
//!
//! ## Forcing the trip without real sleeping
//!
//! The guard trips only when **both** conditions hold: the streamed
//! `llm_call_start` churn count reaches `MAX_GENERATIONS_WITHOUT_PROGRESS` (4)
//! **and** progress silence reaches the `stall_timeout` budget. Rather than
//! wait the production 10m default, the run sets
//! `CLAUDINE_OPENCODE_STALL_TIMEOUT=1s` — a small, bounded budget. (`0.5s`
//! would also arm the guard for 500ms; `1s` is used purely for a comfortable,
//! deterministic window.)
//! The fake provider emits one `step_loop` (establishing safe `step` context
//! and the progress baseline), then a steady stream of `llm_call_start` lines
//! ~150ms apart with no intervening progress. Once both the churn count and the
//! 1s progress-silence budget are crossed the guard trips deterministically —
//! the count condition is what makes this proof, not a timing race; the 1s
//! window is a fixed, bounded wait, not a flaky sleep. `CLAUDINE_STEP_TIMEOUT=0s`
//! disables stream-silence so only the stalled-generation guard can terminate
//! the run.
//!
//! ## Styling (L2 styling-capture pattern)
//!
//! `FORCE_COLOR=1` routes claudine through an optimistic terminal so the
//! capture reflects production styling. The `AgentNative` error block renders
//! its `Agent Error` label as `<red>` prose, which emits the 3-bit red
//! foreground (`\x1b[31m`) — asserted from `frame.raw`. Per the L2 capture
//! caveats, the assertion targets stable visible substrings on `frame.plain`
//! and the semantic 3-bit red SGR on `frame.raw`, never byte-equality across
//! captures or a broad truecolor (`38;2`) matcher.
//!
//! ## Pane height (no scrollback)
//!
//! `capture-pane` returns only the visible pane. A 50-row pane keeps the
//! rendered error block on screen after the short churn burst (per the
//! "Harness capture has no scrollback" caveat).
//!
//! Skip-clean: `TmuxHarness::available()` is checked via `require_level!`; the
//! test skips when tmux is absent. `BISCUIT_TEST_LEVEL_REQUIRED=2` flips a
//! missing backend into a hard failure. Run via `just test-l2`.

#![cfg(unix)]

mod common;
use common::{augmented_path, clear_no_color, write_executable};

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name};
use serial_test::serial;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Backend, Level, require_level};

/// Whether `raw` selects the basic or bright red foreground (`31`/`91`),
/// matching both the bare `\x1b[31m` form and the combined `\x1b[1;31m`
/// (bold+red) the bold label can emit. Per the "<red> emits 3-bit `\x1b[31m`"
/// note the prose `<red>` label always lands on the 3-bit code, so this
/// deliberately carries **no** broad `38;2` truecolor branch (which would
/// false-match an unrelated truecolor span).
fn has_3bit_red(raw: &str) -> bool {
    raw.contains("\x1b[31m")
        || raw.contains("\x1b[91m")
        || raw.contains(";31m")
        || raw.contains(";91m")
}

/// Drive a wrapped OpenCode run whose fake provider emits the stalled-generation
/// retry-churn fingerprint, force the guard to trip via a tiny `stall_timeout`,
/// and assert the rendered `AgentNative` stalled-generation block.
#[test]
#[serial(level2_terminal)]
fn level2_stalled_generation_renders_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    static SEQ: AtomicU32 = AtomicU32::new(0);

    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    common::wrap::seed_minimal_config(workspace.path());

    // Fake OpenCode provider: respond to the `models` validation refresh, emit
    // an stdout `init` event (so the wrapper's stdout-seen path engages), record
    // a readiness marker, then emit one `step_loop` (establishing safe `step`
    // context and the progress baseline) followed by a burst of streamed
    // `llm_call_start` lines to STDERR with no intervening progress — the exact
    // live-but-dead retry-churn fingerprint. The stderr `service=llm ... stream`
    // line is what the bridge counts; the trailing `sleep` keeps the child alive
    // so the guard (not an exit) drives termination. Per the "L2 probe must not
    // be a production bin" note, the provider is a shell script on `PATH`.
    let ready_marker = workspace.path().join("opencode-started");
    write_executable(
        &path_dir.join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"ses_l2_stall","model":"test-model"}'
: > "$CLAUDINE_READY_MARKER"
# Establish the progress baseline and safe `step` context.
printf '%s\n' 'INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_l2_stall step=70 logSpan.http.span.4=55ms loop' >&2
# Retry-churn: repeated streamed generations with zero intervening progress.
# ~150ms apart so progress silence crosses the 1s budget within the burst while
# the churn count climbs well past MAX_GENERATIONS_WITHOUT_PROGRESS.
i=0
while [ "$i" -lt 40 ]; do
  printf '%s\n' 'INFO  2026-05-12T20:00:13 +0ms service=llm providerID=zai-coding-plan modelID=glm-5.2 session.id=ses_l2_stall small=false agent=rust-developer mode=all stream' >&2
  i=$((i + 1))
  /bin/sleep 0.15
done
/bin/sleep 30
"#,
    );

    let md_session_provider = "zai-coding-plan";
    let md_session_model = "glm-5.2";

    let session = format!(
        "biscuit_l2_stall_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    // POSIX shell (bash/sh), not the developer's `$SHELL`: a custom login prompt
    // (e.g. Starship's `❯`) never ends in `$`/`#`/`%`, so `wait_for_prompt`
    // would never match and burn its full timeout.
    let shell = biscuit_test_harness::detect_shell();
    let spawned = std::process::Command::new("tmux")
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "120",
            "-y",
            "50",
            &format!("{shell} -l"),
        ])
        .env("FORCE_COLOR", "1")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(spawned, "failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    // This fixture asserts a *colored* surface under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);

    let claudine = common::claudine_bin();
    let env_pairs: [(&str, String); 7] = [
        ("FORCE_COLOR", "1".to_string()),
        ("HOME", workspace.path().display().to_string()),
        (
            "PATH",
            augmented_path(&path_dir).to_string_lossy().into_owned(),
        ),
        ("OPENCODE_MODEL", "test-model".to_string()),
        ("CLAUDINE_READY_MARKER", ready_marker.display().to_string()),
        // A small, bounded budget. The churn count is what makes the trip a
        // proof, not the 1s window — the run only needs a budget short enough
        // to cross quickly and deterministically.
        ("CLAUDINE_OPENCODE_STALL_TIMEOUT", "1s".to_string()),
        // Disable stream-silence so only the stalled-generation guard (not a
        // step_timeout) can terminate the run.
        ("CLAUDINE_STEP_TIMEOUT", "0s".to_string()),
    ];
    let env_prefix: String = env_pairs
        .iter()
        .map(|(k, v)| format!("{k}='{}' ", v.replace('\'', "'\\''")))
        .collect();
    let cmd = format!("{env_prefix}{claudine} opencode 'describe the thing'");
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send wrapper command");

    // Wait until the fake provider reaches its run loop — it has begun emitting
    // the churn stream and the per-run bridge is wired. Polling the marker keeps
    // the test from racing the wrapper's spawn.
    let marker_deadline = Instant::now() + Duration::from_secs(30);
    while !ready_marker.exists() {
        if Instant::now() >= marker_deadline {
            kill_session_by_name(&session);
            panic!("fake provider never reached its run loop within 30s");
        }
        std::thread::sleep(Duration::from_millis(25));
    }

    // Poll the captured frame for the rendered stalled-generation block. The
    // guard trips after the fourth-plus streamed generation crosses the 0.1s
    // budget, so the block appears within a few churn intervals. Poll on the
    // single word `stalled` (the message and label both word-wrap inside the
    // rendered BlockQuote, so a multi-word needle could straddle a wrap).
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last_raw = String::new();
    let mut last_plain = String::new();
    let mut rendered = false;
    while Instant::now() < deadline {
        if let Ok(frame) = harness.capture() {
            last_raw = frame.raw;
            last_plain = frame.plain;
            if last_plain.contains("stalled") {
                rendered = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Hosted runners have produced fully colorless captures here that no
    // local simulation (fresh tmux server, ambient NO_COLOR, stripped
    // COLORTERM, bogus TERM) reproduces. When the styling is absent, probe
    // the live pane's effective environment before the session dies so the
    // failure itself carries the discriminating evidence.
    let color_probe = if has_3bit_red(&last_raw) {
        String::new()
    } else {
        let _ = harness.send_text(
            b"echo __PROBE__ NO_COLOR=$NO_COLOR FORCE_COLOR=$FORCE_COLOR \
              COLORTERM=$COLORTERM TERM=$TERM; tmux -V\n",
        );
        std::thread::sleep(Duration::from_millis(600));
        harness.capture().map(|f| f.plain).unwrap_or_default()
    };

    kill_session_by_name(&session);

    assert!(
        rendered,
        "the stalled-generation block (word `stalled`) must render in the real \
         terminal once the guard trips.\nplain:\n{last_plain}",
    );

    // The rendered block is a word-wrapped BlockQuote: the message and label are
    // split across lines, each prefixed with the `┃` border glyph, and a long
    // hyphenated word (`live-but-dead`) can hard-wrap at the hyphen. Flatten the
    // border glyphs, drop a soft `- ` hyphen-wrap artifact, then collapse all
    // whitespace (including wrap newlines) so phrases and hyphenated words that
    // straddle a wrap boundary rejoin for matching.
    let collapsed: String = last_plain
        .replace('\u{2503}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    // Rejoin a hyphenated word the terminal hard-wrapped (`live- but-dead` →
    // `live-but-dead`). Safe for this message: its only `- ` runs are wrap
    // artifacts of `live-but-dead`.
    let flattened = collapsed.replace("- ", "-");

    // The `AgentNative` block renders a distinct `Agent Error` label
    // (recommendation A: a distinct presentation kept on the agent-native
    // color/style). The body names the live-but-dead retry loop.
    for needle in [
        "Agent Error",
        "stalled generation:",
        "attempts over",
        "no progress",
        "live-but-dead retry loop",
    ] {
        assert!(
            flattened.contains(needle),
            "expected '{needle}' in the rendered stalled-generation block.\nflattened:\n{flattened}\nplain:\n{last_plain}",
        );
    }
    // The attempt count must be a number >= MAX_GENERATIONS_WITHOUT_PROGRESS
    // (4): the message reads "stalled generation: <N> attempts over <M>s ...".
    let attempt_count: u32 = flattened
        .split("stalled generation:")
        .nth(1)
        .and_then(|tail| tail.split("attempts").next())
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or_else(|| {
            panic!("could not parse the attempt count from the block.\nflattened:\n{flattened}")
        });
    assert!(
        attempt_count >= 4,
        "the stalled-generation block must name a generation-attempt count at \
         or beyond the trip threshold (>=4); got {attempt_count}.\nflattened:\n{flattened}",
    );

    // Safe context only: identity metadata is surfaced, never prompt text or
    // tool payloads. Each `key=value` token is a single word, so it survives
    // word-wrapping intact in the flattened view.
    for needle in [
        "session=ses_l2_stall",
        "step=70",
        "agent=rust-developer",
        &format!("provider={md_session_provider}"),
        &format!("model={md_session_model}"),
        "mode=all",
    ] {
        assert!(
            flattened.contains(needle),
            "expected safe context '{needle}' in the rendered block.\nflattened:\n{flattened}",
        );
    }

    // AgentNative styling: the `<red>` label lands on the 3-bit red foreground
    // (`\x1b[31m`), asserted semantically from the captured escape bytes.
    assert!(
        has_3bit_red(&last_raw),
        "the AgentNative stalled-generation block must render with the 3-bit \
         red foreground SGR.\nraw:\n{last_raw}\npane env probe:\n{color_probe}",
    );
}
