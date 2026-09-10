//! Level 2 real-terminal capture for the full incomplete-sub-agent diagnostic.
//!
//! ## The gap this closes
//!
//! `wrap_incomplete_subagents.rs` (Level 1) proves the diagnostic reaches
//! stderr with every stopped task named, including past the five-item limit the
//! watchdog ring imposes. It reads a pipe, so it can say nothing about the one
//! property whose contract *is* the terminal: that the heading, its explanatory
//! sentence, and every list entry all survive being rendered into a real pane at
//! a constrained width, where both the heading and `UnorderedList` must fold
//! themselves rather than leave the emulator to chop them mid-word.
//!
//! ## Width
//!
//! 70 columns — the documented minimum-supported-width floor (claudine's own
//! `--side-effects` report drops its `Example` column below it), and the
//! narrowest width any claudine surface is expected to survive. The pane and
//! `COLUMNS` agree, so the emulator's own wrap boundary is the same 70 the
//! renderer sized to and nothing is measured against a width claudine never saw.
//!
//! ## Two wraps, one rule
//!
//! At 70 columns the capture holds both, and the test asserts each on the same
//! terms — the *component* folded the text, not the emulator:
//!
//! - **`UnorderedList` wraps.** Each entry breaks on a space and continues on a
//!   `HANGING_INDENT`ed row, and every list row stays within the reserved 1ch
//!   right margin.
//! - **The heading wraps too.** `IncompleteSubagents` opts its heading `Prose`
//!   into `WordWrap::default()`, because `Layout::default()` ships
//!   `WordWrap::None` (`renderable::layout::Layout`) and the unwrapped
//!   113-cell heading was reaching the pane as one logical line for the
//!   emulator to chop mid-word (`so t` / `his run`).
//!
//! The distinguishing evidence is width: an emulator soft-wrap fills the row to
//! exactly `PANE_COLS`, so a heading that breaks *below* that width — and whose
//! rows rejoin on single spaces into the original sentence — can only have been
//! folded by the component. That is a real-terminal fact a pipe cannot produce.
//!
//! ## Hazards handled
//!
//! - **No scrollback.** `capture-pane` returns only the visible pane, so the
//!   session is 200 rows — comfortably more than the ~95 the replay emits — and
//!   a sentinel printed immediately before the run marks where claudine's own
//!   output begins.
//! - **Styling.** `FORCE_COLOR=1` routes claudine through an optimistic
//!   terminal so the heading's `<b>` reaches the pane; `common::clear_no_color`
//!   removes an ambient `NO_COLOR`, which claudine treats as present rather
//!   than truthy and which would otherwise make the styling assertion vacuous.
//! - **Focus.** tmux sessions are detached and headless: nothing is raised, and
//!   no `SpawnVisibility::Foreground` or `focus_spawned_pane` appears here
//!   (acceptance criterion 11, enforced by `test_placement.rs`).
//! - **Stale binaries.** The binary under test comes from
//!   `common::claudine_bin()` — the run-time-resolved path to the binary this
//!   suite built — never a bare `claudine` off the host `PATH`.
//! - **Probe hygiene.** The provider is a shell script written into the fixture
//!   `bin`, not a production binary.
//!
//! ## Level and platform
//!
//! L2, not L3: this verifies what the terminal *displayed*, and no keypress
//! participates. The binary is `cfg(unix)` because the replay provider is a
//! `/bin/sh` script and every tmux-backed neighbour is gated the same way; the
//! assertions themselves name no signal, path separator, or platform glyph.
//!
//! Skip-clean via `require_level!`; run through `just test-l2`.

#![cfg(unix)]

use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_test_harness::tmux::{
    TmuxHarness, kill_session_by_name, spawn_shell_session_with_env,
};
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::incomplete_subagents::{
    REPLAY_DOCUMENT, incident_stream, task_notification, task_started, write_replay_provider,
};
use common::{CliProcessFixture, claudine_bin, clear_no_color};

/// The documented minimum-supported-width floor. Both the pane and `COLUMNS`.
const PANE_COLS: u32 = 70;

/// Tall enough that the whole replay — two progress lines per task plus the
/// trailer — stays on screen. The harness keeps no scrollback.
const PANE_ROWS: u32 = 200;

/// More than the five terminal observations the watchdog ring retains, so the
/// capture also witnesses the ledger-completeness contract (criterion 7).
const STOPPED_TASK_COUNT: usize = 7;

/// `UnorderedList`'s continuation indent, in columns.
const HANGING_INDENT: &str = "  ";

/// The widest a list row may be. `UnorderedList` reserves a 1ch right margin,
/// so a list row never reaches the pane's last cell — which is also what keeps
/// the emulator from soft-wrapping it and disguising a component wrap.
const LIST_WIDTH_LIMIT: u32 = PANE_COLS - 1;

/// The bullet `IncompleteSubagents` configures. Only the full diagnostic uses
/// it, so anchoring on it keeps the concise headline — which names the same
/// tasks, truncated — from satisfying a list assertion.
const BULLET: &str = "\u{2022} ";

/// Printed by the shell immediately before claudine runs.
const SENTINEL: &str = "__L2_INCOMPLETE_SUBAGENTS_BEGIN__";

/// The exit-code marker echoed after the run. Bracket- and glob-free so no
/// login shell mangles it before the code is printed.
const EXIT_MARKER: &str = "claudine_rc:";

/// A stopped task whose rendered entry cannot fit on one 70-column row.
///
/// `SubagentOutcome::describe` renders `<name> (stopped)`, so each entry is 91
/// cells against a 68-cell content budget. The name breaks on spaces rather
/// than being one unbreakable token, so the wrap under test is
/// `UnorderedList`'s hanging indent and not the emulator's mid-word chop.
fn wrapping_task_name(index: usize) -> String {
    format!(
        "commit-agent-{index} stage the pending watchdog and ledger changes then push the branch"
    )
}

/// The rendered list entry for one stopped task, as the component composes it.
fn rendered_entry(index: usize) -> String {
    format!("{BULLET}{} (stopped)", wrapping_task_name(index))
}

/// The captured rows that belong to claudine's own output.
///
/// Anchors on the *last* sentinel: the shell also echoes the typed command line,
/// which at 70 columns wraps wherever the prompt length puts it and may split
/// the echoed copy across rows.
fn rows_after_sentinel(frame: &CapturedFrame) -> Vec<String> {
    let rows: Vec<&str> = frame.plain.lines().map(str::trim_end).collect();
    let start = rows
        .iter()
        .rposition(|row| row.contains(SENTINEL))
        .map_or(0, |index| index + 1);
    rows[start..].iter().map(|row| (*row).to_string()).collect()
}

/// Every captured row collapsed to single-spaced text.
///
/// Both the heading and `UnorderedList` break across rows on purpose, so a whole
/// sentence or entry is only visible once its opening row and continuations are
/// rejoined. Collapsing whitespace does that without needing to model the indent
/// width — and it is only sound because every break lands on a word boundary,
/// which the heading and list assertions below establish independently.
fn flatten(rows: &[String]) -> String {
    rows
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Drive one wrapped `compose --claude` run of the incident replay inside a
/// 70-column tmux pane and return the settled capture plus claudine's exit code.
fn capture_replay() -> (CapturedFrame, i32) {
    static SEQ: AtomicU32 = AtomicU32::new(0);

    let fixture = CliProcessFixture::named("claudine-l2-incomplete-subagents");
    fixture.seed_user_config();
    let document = fixture.cwd().join("replay.md");
    fs::write(&document, REPLAY_DOCUMENT).unwrap();

    let starts: Vec<String> = (0..STOPPED_TASK_COUNT)
        .map(|index| task_started(&format!("sa_{index}"), &wrapping_task_name(index)))
        .collect();
    let stops: Vec<String> = (0..STOPPED_TASK_COUNT)
        .map(|index| {
            let name = wrapping_task_name(index);
            task_notification(&format!("sa_{index}"), &name, "stopped")
        })
        .collect();
    write_replay_provider(fixture.bin_dir(), &incident_stream(&starts, &stops));

    let session = format!(
        "biscuit_l2_incomplete_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    // A POSIX shell rather than the developer's `$SHELL`: a custom login prompt
    // (Starship's `❯`, say) never ends in `$`/`#`/`%`, so `wait_for_prompt` would
    // never match and would burn its whole timeout.
    spawn_shell_session_with_env(&session, PANE_COLS, PANE_ROWS, &[("FORCE_COLOR", "1")])
        .expect("failed to spawn tmux session");

    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    // The heading's `<b>` is asserted below under `FORCE_COLOR=1`, which an
    // ambient `NO_COLOR` out-votes — see `common::clear_no_color`.
    clear_no_color(&mut harness);

    // The child's `PATH` is the fixture `bin` ahead of the minimal system set,
    // matching the L1 spawn contract: the replay provider resolves, and no
    // agentic CLI installed on this host does.
    let child_path = std::env::join_paths(
        std::iter::once(fixture.bin_dir().to_path_buf()).chain(common::minimal_system_path()),
    )
    .expect("child PATH entries must join");

    let env_pairs: [(&str, String); 5] = [
        ("HOME", fixture.home().display().to_string()),
        ("PATH", child_path.to_string_lossy().into_owned()),
        ("COLUMNS", PANE_COLS.to_string()),
        ("FORCE_COLOR", "1".to_string()),
        ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
    ];
    let env_prefix: String = env_pairs
        .iter()
        .map(|(key, value)| format!("{key}='{}' ", value.replace('\'', "'\\''")))
        .collect();
    let cmd = format!(
        "cd '{cwd}' && printf '{SENTINEL}\\n' && {env_prefix}{claudine} compose --claude '{doc}'; printf '{EXIT_MARKER}%s\\n' \"$?\"",
        cwd = fixture.cwd().display(),
        claudine = claudine_bin(),
        doc = document.display(),
    );
    harness
        .send_command_with_env(&cmd, &[])
        .expect("send the wrapped compose command");

    // Poll for the content the assertions read — the exit marker *and* every
    // rendered entry — in one loop, so a half-painted frame can never be the
    // frame under test.
    //
    // The deadline sits under the 90s termination ceiling this package's L2
    // tier carries in `.config/nextest.toml`, so a run that never renders fails
    // on this assertion (with the pane in the message, and the session torn
    // down) rather than being killed mid-poll and leaking it.
    const RENDER_DEADLINE: Duration = Duration::from_secs(40);
    let deadline = Instant::now() + RENDER_DEADLINE;
    let mut frame = CapturedFrame::from_raw(String::new());
    let mut missing: Vec<String> = Vec::new();
    loop {
        if let Ok(current) = harness.capture() {
            frame = current;
            let body = rows_after_sentinel(&frame);
            let flat = flatten(&body);
            missing = (0..STOPPED_TASK_COUNT)
                .map(rendered_entry)
                .filter(|entry| !flat.contains(entry))
                .collect();
            if missing.is_empty() && body.iter().any(|line| line.starts_with(EXIT_MARKER)) {
                break;
            }
        }
        if Instant::now() >= deadline {
            kill_session_by_name(&session);
            panic!(
                "the incomplete-sub-agent diagnostic did not finish rendering within \
                 {RENDER_DEADLINE:?}; still missing {missing:?}.\nplain:\n{}",
                frame.plain
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let exit_code = rows_after_sentinel(&frame)
        .iter()
        .find_map(|line| line.strip_prefix(EXIT_MARKER)?.trim().parse::<i32>().ok())
        .expect("the exit marker must carry a numeric code");

    kill_session_by_name(&session);
    (frame, exit_code)
}

/// The full diagnostic, rendered into a real 70-column terminal.
///
/// Review 2 finding 5: the heading, the sentence that explains why a zero exit
/// is still a failure, and **every** task entry must survive real-terminal
/// rendering at a constrained width.
#[test]
#[serial(level2_terminal)]
fn level2_incomplete_subagent_diagnostic_renders_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let (frame, exit_code) = capture_replay();
    let body = rows_after_sentinel(&frame);
    let flat = flatten(&body);
    let plain = frame.plain.as_str();

    // The run reached the pane as the semantic failure the fix projects, so the
    // diagnostic under test is the one a real operator sees.
    assert_eq!(
        exit_code, 1,
        "a run with unresolved sub-agent work must exit 1.\nplain:\n{plain}"
    );

    // --- The heading, and the sentence that explains the verdict ------------

    let heading = format!("{STOPPED_TASK_COUNT} sub-agent tasks did not complete.");
    let explanation =
        "The provider exited normally, so this run is a failure despite its exit code.";
    assert!(
        flat.contains(&heading),
        "the diagnostic heading must survive real-terminal rendering.\nplain:\n{plain}"
    );
    assert!(
        flat.contains(explanation),
        "the sentence explaining why exit 0 is still a failure must survive real-terminal \
         rendering.\nplain:\n{plain}"
    );
    assert!(
        flat.contains(&format!("{heading} {explanation}")),
        "the heading and its explanation must arrive as one sentence pair, not as two \
         fragments separated by other output.\nplain:\n{plain}"
    );

    // --- The heading's wrap is the component's, on word boundaries ----------

    // The heading occupies the rows immediately above the list, so it is bounded
    // below by the first bullet row and opened by the last row before it that
    // begins the count sentence — an anchor the concise lifecycle headline,
    // which names the same tasks, cannot satisfy.
    let first_bullet = body
        .iter()
        .position(|row| row.trim_start().starts_with(BULLET))
        .unwrap_or_else(|| panic!("no captured row opens the list.\nplain:\n{plain}"));
    let heading_open = body[..first_bullet]
        .iter()
        .rposition(|row| row.trim_start().starts_with(&heading))
        .unwrap_or_else(|| panic!("no captured row opens the heading.\nplain:\n{plain}"));
    let heading_rows = &body[heading_open..first_bullet];

    assert!(
        heading_rows.len() > 1,
        "the heading is longer than {PANE_COLS} cells, so it must occupy more than one \
         row.\nplain:\n{plain}"
    );
    for row in heading_rows {
        let width = visible_width(row);
        assert!(
            width < PANE_COLS,
            "a heading row filling the pane is the emulator's mid-word soft wrap, not the \
             component's word wrap; row used {width} of {PANE_COLS} cells: \
             {row:?}.\nplain:\n{plain}"
        );
    }
    // Rejoining on a single space reproduces the sentence pair only if every
    // break fell between words: a mid-word chop leaves the halves apart
    // (`so t` + `his run`).
    assert_eq!(
        heading_rows
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        format!("{heading} {explanation}"),
        "the heading must break on word boundaries, like the list beneath \
         it.\nplain:\n{plain}"
    );

    // --- Every task entry, including past the five-item ring ----------------

    for index in 0..STOPPED_TASK_COUNT {
        assert!(
            flat.contains(&rendered_entry(index)),
            "every stopped task must survive real-terminal rendering as its own list entry; \
             {:?} is missing.\nplain:\n{plain}",
            rendered_entry(index)
        );
    }

    // --- `UnorderedList` wrapping, and the widths it must respect -----------

    let rows: Vec<&str> = plain.lines().map(str::trim_end).collect();
    let marker_rows: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.trim_start().starts_with(BULLET))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        marker_rows.len(),
        STOPPED_TASK_COUNT,
        "the list must open exactly one row per stopped task.\nplain:\n{plain}"
    );
    for index in &marker_rows {
        let continuation = rows
            .get(index + 1)
            .unwrap_or_else(|| panic!("list entry at row {index} has no continuation row"));
        assert!(
            continuation.starts_with(HANGING_INDENT) && !continuation.trim().is_empty(),
            "each entry must wrap onto a hanging-indented continuation row; row {} was \
             {continuation:?}.\nplain:\n{plain}",
            index + 1
        );
    }
    for index in marker_rows.iter().copied().flat_map(|row| [row, row + 1]) {
        let width = visible_width(rows[index]);
        assert!(
            width <= LIST_WIDTH_LIMIT,
            "list rows must stay inside the reserved 1ch right margin ({LIST_WIDTH_LIMIT} cells); \
             row {index} used {width}: {:?}.\nplain:\n{plain}",
            rows[index]
        );
    }

    // --- Styling ------------------------------------------------------------

    // The heading opens with `<b>`. Anchored on the row that carries the count
    // sentence, so unrelated colored output in the pane cannot satisfy it.
    let styled_row = frame
        .raw
        .lines()
        .find(|row| common::strip_ansi(row).contains(&heading))
        .unwrap_or_else(|| panic!("no raw row carries the heading.\nraw:\n{}", frame.raw));
    assert!(
        styled_row.contains("\u{1b}[1m"),
        "the heading's bold must survive the emulator's SGR re-emission.\nrow: {styled_row:?}"
    );
}
