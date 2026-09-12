//! Level 2 real-terminal capture for provided-partial resolution ahead of
//! `initialize`.
//!
//! `fixes/2026-09-10-no-interactive-completion` asked for its interactive
//! coverage to run through the shared terminal harness. The PTY suite in
//! `level2_provided_partial_file_pty.rs` proves the ordering and the data flow
//! of that flow with bytes the test process manufactures; this binary is its
//! rendering complement. It runs the shipped `prompts/review.md` router with
//! the reported `spec=` partial inside a real terminal emulator (tmux and
//! WezTerm) and reads back what the emulator drew:
//!
//! - The `Use this file? (Y/n)` confirmation is on screen, styled, before the
//!   router's `initialize` guard can dereference the partial — no provider has
//!   been reached and no lifecycle error has been printed.
//! - Accepting it through the emulator's own key path carries the selected
//!   spec across the proxy hop to the provider stub, and the run exits `0`.
//!
//! What a terminal application does with the widget is the only thing this
//! file adds over the PTY suite; the decline, chooser, and proxy-target
//! variants stay there, where they are cheaper and already discriminate.
//!
//! Gating: `#![cfg(unix)]`, `require_level!(Level::L2, ...)` so the tests skip
//! cleanly when the backend is unavailable and fail under
//! `BISCUIT_TEST_REQUIRED_BACKENDS`.
//!
//! Run via the canonical recipe:
//!
//! ```text
//! just test-l2 provided_partial_file_capture
//! ```

#![cfg(unix)]

use std::fs;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{TerminalHarness, wait_for_prompt};
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

mod common;
use common::review_router::{
    PARTIAL, assert_provider_received, launch_area, names_selected_spec, provider_prompt,
    review_router_fixture,
};
use common::{
    assert_row_is_styled, claudine_bin, clear_no_color, minimal_system_path, sh_quote,
    wait_for_exit_marker, wait_for_pane_text,
};

const CONFIRMATION_PROMPT: &str = "Use this file? (Y/n)";
const PARTIAL_NOTICE: &str = "did not match a file directly";
const CANDIDATE_BADGE: &str = " FILE ";
const LIFECYCLE_ERROR: &str = "lifecycle evaluation error";
const SELECTED_SPEC: &str = "2026-09-10-local-a/spec.md";

/// The one key this file sends, encoded by each backend's own input path.
trait KeySender: TerminalHarness {
    fn send_yes(&mut self) -> io::Result<()>;
}

impl KeySender for TmuxHarness {
    fn send_yes(&mut self) -> io::Result<()> {
        self.send_key("y")
    }
}

impl KeySender for WezTermHarness {
    fn send_yes(&mut self) -> io::Result<()> {
        self.send_text(b"y")
    }
}

fn run_accept_path<H: KeySender>(harness: &mut H) {
    // The styling assertion below needs the dialog to reach the pane colored;
    // an ambient `NO_COLOR` would turn it into a false failure.
    clear_no_color(harness);

    let (fixture, router) = review_router_fixture(false);
    let launch = launch_area(&fixture);
    let home = fixture.home().display().to_string();
    let path = std::env::join_paths(
        std::iter::once(fixture.bin_dir().to_path_buf()).chain(minimal_system_path()),
    )
    .expect("join fixture PATH")
    .to_string_lossy()
    .into_owned();

    harness
        .send_command_with_env(&format!("cd {}", sh_quote(&launch.display().to_string())), &[("HOME", &home)])
        .expect("cd into launch area");
    let _ = wait_for_prompt(harness);

    // Shared broker panes retain previous tests' output. Clear the viewport
    // before this command and wait for its own exit marker so neither an old
    // dialog nor an old shell prompt can satisfy this invocation's capture.
    //
    // The marker reports success through `&&`/`||` rather than `"$?"`: a `?`
    // typed into the pane's interactive shell is Atuin AI's trigger key on a
    // host that loads it, and its onboarding overlay then wedges the pane.
    static NEXT_CAPTURE: AtomicUsize = AtomicUsize::new(0);
    let marker = format!(
        "PARTIAL_{}_{}",
        std::process::id(),
        NEXT_CAPTURE.fetch_add(1, Ordering::Relaxed)
    );
    let script = format!(
        "printf '\\033[2J\\033[H'; {} compose --goose {} spec={PARTIAL} -y token=retained && printf '\\n{marker}:0\\n' || printf '\\n{marker}:1\\n'",
        claudine_bin(),
        sh_quote(&router.display().to_string()),
    );
    harness
        .send_command_with_env(
            &format!("/bin/sh -c {}", sh_quote(&script)),
            &[
                ("HOME", &home),
                ("PATH", &path),
                ("TERM", "xterm-256color"),
                ("COLORTERM", "truecolor"),
                ("FORCE_COLOR", "1"),
                ("CLAUDINE_RENDEZVOUS_REPORT", "false"),
            ],
        )
        .expect("send review router command");

    let dialog = wait_for_pane_text(harness, CONFIRMATION_PROMPT, Duration::from_secs(20));
    assert!(
        !provider_prompt(&fixture).exists(),
        "provider was reached before the confirmation was drawn; plain:\n{}",
        dialog.plain
    );
    assert!(
        !dialog.plain.contains(LIFECYCLE_ERROR),
        "initialize dereferenced the partial before the confirmation; plain:\n{}",
        dialog.plain
    );
    assert!(
        !dialog.plain.contains(&format!("{marker}:")),
        "command exited before the confirmation was answered; plain:\n{}",
        dialog.plain
    );
    assert!(
        dialog.plain.contains(PARTIAL_NOTICE),
        "the partial notice must precede the confirmation; plain:\n{}",
        dialog.plain
    );
    // The candidate path is longer than the pane is wide and wraps wherever
    // the emulator's column count puts the break, so match it unwrapped.
    let unwrapped: String = dialog.plain.lines().collect();
    assert!(
        names_selected_spec(&unwrapped, SELECTED_SPEC) && !unwrapped.contains("local-decoy"),
        "the confirmation must name the launch-area candidate only; plain:\n{}",
        dialog.plain
    );
    // The notice is plain prose; the candidate card beneath it is where the
    // dialog's styling lives (bold badge, hyperlinked path, dim schema note).
    assert_row_is_styled(&dialog.raw, CANDIDATE_BADGE, "candidate file badge");

    harness.send_yes().expect("accept the candidate");

    let (done, status) = wait_for_exit_marker(harness, &marker, Duration::from_secs(20));
    assert_eq!(status, "0", "review router failed after acceptance; plain:\n{}", done.plain);
    assert!(
        done.plain.contains("provider reached"),
        "proxy target never reached the provider stub; plain:\n{}",
        done.plain
    );
    assert!(!done.plain.contains(LIFECYCLE_ERROR), "plain:\n{}", done.plain);
    // The shipped router also requires `plan` and `review`; collecting either
    // ahead of `initialize` announces itself with this sentence.
    assert!(
        !done.plain.contains("requires a valid file reference"),
        "an unrelated property was collected before initialize; plain:\n{}",
        done.plain
    );
    let prompt = fs::read_to_string(provider_prompt(&fixture)).expect("provider stub recorded its prompt");
    assert_provider_received(&prompt, "alpha", SELECTED_SPEC);
}

#[test]
#[serial(level2_terminal)]
fn level2_tmux_review_router_partial_confirms_before_initialize() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
    run_accept_path(&mut harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_wezterm_review_router_partial_confirms_before_initialize() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let mut harness = WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm");
    run_accept_path(&mut harness);
}
