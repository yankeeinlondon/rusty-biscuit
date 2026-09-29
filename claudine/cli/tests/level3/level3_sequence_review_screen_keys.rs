//! Level 3: the `claudine sequence` review screen's key promises, pressed as
//! OS key events (Linux).
//!
//! The review screen reads its keys itself, in raw mode: `Ctrl+S` submits
//! every row, and `Esc` and `Ctrl+C` leave it with exit `130` before any step
//! runs. Each press here is an XTEST key event on a private X display
//! (`biscuit_test_harness::xvfb::XvfbKitty`), so it enters where a physical
//! keyboard's events enter and kitty's own encoder turns it into the bytes the
//! screen decodes. The host user's display and focus are never touched.
//!
//! The fixture is `common::review_screen`: a sequence interleaving eligible
//! steps with hidden `shell` and `side_effect` steps, fake `claude`/`goose`
//! executables recording each launch, and hidden steps writing `events.log`.
//! Layout, per-row choices, and the free-text model editor are covered in a
//! real terminal by `level2_sequence_review_screen`.
//!
//! ## Other operating systems
//!
//! macOS and Windows have no private desktop that can receive key events
//! (see `level3_drain_ctrl_c.rs`); an OS key press there takes the user's
//! focus, so this module is Linux-only.
//!
//! Skip-clean without `Xvfb` and `kitty`, and unless `RUN_LEVEL3=1`.

use biscuit_test_harness::xvfb::XvfbKitty;
use test_toolkit::{Level, require_level};

use crate::common::review_screen::{
    HIDDEN_STEP_EVENTS, ModelEditor, ReviewFixture, STEP_TOKENS,
};

const REQUIREMENT: &str = "Xvfb + kitty (private X display)";

/// Open the screen on a private display, press once, and return the fixture
/// with the final frame and exit status.
fn press_on_screen(name: &str, press: impl FnOnce(&XvfbKitty)) -> (ReviewFixture, String, String) {
    let review = ReviewFixture::stage(name, ModelEditor::CatalogChoice);
    let instance = XvfbKitty::launch(120, 40).expect("launch kitty on a private X display");
    let mut harness = instance.harness();
    review.open(&mut harness);
    press(&instance);
    let (frame, status) = review.wait_for_exit(&mut harness);
    (review, frame, status)
}

/// A pressed `Ctrl+S` submits the rows as drawn: every eligible step runs on
/// the default provider and model, and the hidden steps run.
#[test]
fn level3_os_ctrl_s_submits_the_sequence_review_screen() {
    require_level!(Level::L3, XvfbKitty::can_launch(), REQUIREMENT);
    let (review, frame, status) = press_on_screen("review-keys-ctrl-s", |instance| {
        instance.press_ctrl('s').expect("XTEST Ctrl+S on the private display");
    });
    assert_eq!(status, "0", "Ctrl+S must submit and the sequence succeed:\n{frame}");
    let expected: Vec<(&str, String, Option<String>)> = STEP_TOKENS
        .into_iter()
        .map(|token| (token, "claude".to_string(), None))
        .collect();
    assert_eq!(review.targets_by_step(), expected, "targets per step:\n{frame}");
    assert_eq!(review.events(), HIDDEN_STEP_EVENTS, "hidden steps:\n{frame}");
    review.assert_no_audio();
}

/// A pressed `Esc` leaves the screen, starts no step, and exits `130`.
#[test]
fn level3_os_escape_cancels_the_sequence_review_screen() {
    require_level!(Level::L3, XvfbKitty::can_launch(), REQUIREMENT);
    let (review, frame, status) = press_on_screen("review-keys-escape", |instance| {
        instance.press_escape().expect("XTEST Escape on the private display");
    });
    assert_eq!(status, "130", "Esc must exit 130:\n{frame}");
    review.assert_no_work(&frame);
}

/// A pressed `Ctrl+C` leaves the screen, starts no step, and exits `130`.
#[test]
fn level3_os_ctrl_c_cancels_the_sequence_review_screen() {
    require_level!(Level::L3, XvfbKitty::can_launch(), REQUIREMENT);
    let (review, frame, status) = press_on_screen("review-keys-ctrl-c", |instance| {
        instance.press_ctrl('c').expect("XTEST Ctrl+C on the private display");
    });
    assert_eq!(status, "130", "Ctrl+C must exit 130:\n{frame}");
    review.assert_no_work(&frame);
}
