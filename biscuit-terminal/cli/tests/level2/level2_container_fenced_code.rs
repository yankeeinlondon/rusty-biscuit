//! Level-2 tests for the `bt quote` / `bt list` container paths embedding a
//! Prose body that carries a fenced code block.
//!
//! Regression for review-2: a `Prose` body with a fenced code block embedded
//! in a quote or list container projected to an invalid render tree
//! (`Paragraph([…, Code, …])`), which validation rejected — the terminal
//! renderer then emitted empty output. These tests prove the container paths
//! render the styled text and the dim code block in a real terminal.
//!
//! The same containers are also checked for inline code, a link around an
//! explicit code block, and hard breaks in list items, as displayed.
//!
//! ## Skip-clean contract
//!
//! Every test checks `*Harness::available()` before spawning. When the
//! required terminal is absent the test skips clean via `require_level!`. No
//! `#[ignore]` markers are used.

use crate::common;

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use common::OutputRow;
use crate::prose_cells::{attr_run_cells, osc8_run_cells};
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// Process-shared WezTerm pane reused across the container tests.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Process-shared Kitty window reused across the container tests.
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// A styled Prose body carrying an inline fenced code block. The
/// `<code-block>` form (rather than a triple-backtick fence) keeps the payload
/// single-line and shell-safe while still producing the block-level `Code`
/// node embedded in the styled span that tripped the container projection.
const FENCED_PROSE: &str = "<red>before <code-block lang=rust>code</code-block> after</red>";

/// Drives `bt <subcommand> "<FENCED_PROSE>"` with `FORCE_COLOR=1` and asserts
/// the container rendered the styled text and the dim code block — proving the
/// embedded-Prose-with-code projection no longer collapses to empty output.
fn assert_container_renders_fenced_code<H: TerminalHarness>(harness: &mut H, subcommand: &str) {
    let frame = common::run_bt_output(harness, &format!("{subcommand} \"{FENCED_PROSE}\""), FORCED);

    // Visible text survived. A validation failure would emit empty output, so
    // none of `before` / `code` / `after` would appear in the rendered rows.
    for needle in ["before", "code", "after"] {
        assert!(
            output_region_contains(&frame, needle),
            "expected visible `{needle}` in the `bt {subcommand}` output region.\n\
             plain:\n{}\nraw:\n{}",
            frame.plain,
            frame.raw,
        );
    }

    // The enclosing red style lowered through the container's tree path: the
    // `before` output row selects red (`31`/`91`).
    let before_row = find_output_row(&frame, "before").unwrap_or_else(|| {
        panic!(
            "could not locate the `before` output row.\nraw:\n{}",
            frame.raw
        )
    });
    assert!(
        row_selects_red(before_row),
        "expected the `before` text to be red through the `bt {subcommand}` \
         container path.\nrow: {before_row:?}",
    );
}

/// Returns the raw (escape-bearing) row of `frame`, a command's output frame,
/// whose visible text contains `needle`. The output frame holds no echo, so
/// the literal markup the user typed cannot satisfy the search.
fn find_output_row<'a>(frame: &'a CapturedFrame, needle: &str) -> Option<&'a str> {
    frame
        .plain
        .lines()
        .zip(frame.raw.lines())
        .find_map(|(plain, raw)| plain.contains(needle).then_some(raw))
}

/// Whether `needle` appears in any row of `frame`, a command's output frame.
fn output_region_contains(frame: &CapturedFrame, needle: &str) -> bool {
    find_output_row(frame, needle).is_some()
}

/// Whether `segment` contains an SGR sequence that selects basic red (`31`) or
/// bright red (`91`) foreground. Accepts both the semicolon and ITU colon SGR
/// sub-parameter forms the two emulators emit.
fn row_selects_red(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len()
                && (bytes[j].is_ascii_digit() || bytes[j] == b';' || bytes[j] == b':')
            {
                j += 1;
            }
            if bytes.get(j) == Some(&b'm') {
                let selects_red = segment[start..j]
                    .split([';', ':'])
                    .any(|p| p == "31" || p == "91");
                if selects_red {
                    return true;
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    false
}

// ------------------------------------------------------------------
// WezTerm
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_quote_fenced_code_prose_renders_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    assert_container_renders_fenced_code(harness, "quote");
}

#[test]
#[serial(level2_terminal)]
fn level2_list_fenced_code_prose_renders_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    assert_container_renders_fenced_code(harness, "list");
}

// ------------------------------------------------------------------
// Kitty
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_quote_fenced_code_prose_renders_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    assert_container_renders_fenced_code(harness, "quote");
}

#[test]
#[serial(level2_terminal)]
fn level2_list_fenced_code_prose_renders_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    assert_container_renders_fenced_code(harness, "list");
}

// ------------------------------------------------------------------
// Inline code, links around block code, and hard breaks in containers
// ------------------------------------------------------------------

/// Process-shared tmux session for the container display tests; `capture-pane
/// -e` re-serializes each cell's SGR and OSC 8 state.
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

const FORCED: &[(&str, &str)] = &[("FORCE_COLOR", "1")];

fn dim_runs(row: &OutputRow) -> Vec<String> {
    common::active_runs(&attr_run_cells(&row.raw, "2"), true)
}

fn link_runs(row: &OutputRow) -> Vec<String> {
    common::active_runs(&osc8_run_cells(&row.raw), false)
}

fn non_blank(rows: &[OutputRow]) -> Vec<&OutputRow> {
    rows.iter().filter(|row| !row.plain.trim().is_empty()).collect()
}

/// The text of a container row without its bullet or quote border.
fn content(row: &OutputRow) -> &str {
    row.plain
        .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '•' | '│'))
        .trim_end()
}

/// Inline code inside a quote and a list displays dim with its delimiters
/// gone; a link around an explicit code block displays as linked paragraphs
/// around the dim, unlinked code; and a backslash hard break in a list item
/// keeps its second row under the item's hanging indent.
fn assert_container_inline_display<H: TerminalHarness>(harness: &mut H) {
    for subcommand in ["quote", "list"] {
        let (frame, rows) = common::run_bt_rows(harness, &format!("{subcommand} 'see `md hash` here'"), FORCED);
        let shown = non_blank(&rows);
        assert_eq!(shown.len(), 1, "`bt {subcommand}`.\nplain:\n{}", frame.plain);
        assert_eq!(content(shown[0]), "see md hash here", "{:?}", shown[0]);
        assert_eq!(dim_runs(shown[0]), ["md hash"], "{:?}", shown[0]);

        let (frame, rows) = common::run_bt_rows(
            harness,
            &format!("{subcommand} '[a<code-block>x</code-block>b](https://e.io)'"),
            FORCED,
        );
        let shown = non_blank(&rows);
        let texts: Vec<&str> = shown.iter().map(|row| content(row)).collect();
        assert!(
            texts.first() == Some(&"a") && texts.last() == Some(&"b") && texts.contains(&"x"),
            "`bt {subcommand}` must show the linked paragraphs around the code: {texts:?}\nplain:\n{}",
            frame.plain,
        );
        for row in [shown[0], shown[shown.len() - 1]] {
            assert_eq!(link_runs(row), [content(row)], "{row:?}");
            assert_eq!(common::osc8_destinations(&row.raw), ["https://e.io"], "{row:?}");
        }
        let code = shown.iter().find(|row| content(row) == "x").expect("code row");
        assert_eq!(dim_runs(code), ["x"], "{code:?}");
        assert!(link_runs(code).is_empty(), "{code:?}");
    }

    let (frame, rows) = common::run_bt_rows(harness, r#"list "$(printf 'one\\\ntwo')""#, FORCED);
    let shown = non_blank(&rows);
    let plain: Vec<&str> = shown.iter().map(|row| row.plain.as_str()).collect();
    assert_eq!(plain, ["• one", "  two"], "plain:\n{}", frame.plain);
}

/// A `Hard`-mode Prose list item keeps each authored line on its own row
/// under the hanging indent. `bt list` has no line-break flag, so the list is
/// rendered in-process and its bytes are shown in the pane.
fn assert_hard_mode_list_display<H: TerminalHarness>(harness: &mut H) {
    use biscuit_terminal::components::list::UnorderedList;
    use biscuit_terminal::components::prose::{LineBreaks, Prose};
    use biscuit_terminal::components::renderable::TerminalRenderable;

    let mut list = UnorderedList::empty();
    list.add(Prose::new("Head <b>one</b>\n  - Details: `x`").with_line_breaks(LineBreaks::Hard));
    list.add(Prose::new("Next"));
    let (frame, rows) = common::display_bytes_rows(harness, list.render(&common::styled_terminal()).as_bytes());
    let shown = non_blank(&rows);
    let plain: Vec<&str> = shown.iter().map(|row| row.plain.trim_end()).collect();
    assert_eq!(plain.len(), 3, "{plain:?}\nplain:\n{}", frame.plain);
    assert!(plain[0].ends_with("Head one"), "{plain:?}");
    assert_eq!(plain[1].trim_start(), "- Details: x", "{plain:?}");
    assert!(plain[2].ends_with("Next"), "{plain:?}");
    assert_eq!(
        common::active_runs(&attr_run_cells(&shown[0].raw, "1"), true),
        ["one"],
        "{:?}",
        shown[0]
    );
    assert_eq!(dim_runs(shown[1]), ["x"], "{:?}", shown[1]);
}

#[test]
#[serial(level2_terminal)]
fn level2_container_inline_code_and_breaks_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    assert_container_inline_display(harness);
    assert_hard_mode_list_display(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_container_inline_code_and_breaks_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);
    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    assert_container_inline_display(harness);
    assert_hard_mode_list_display(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_container_inline_code_and_breaks_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut guard =
        SHARED_TMUX.get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    assert_container_inline_display(harness);
    assert_hard_mode_list_display(harness);
}
