//! Regression tests for the Level 2 output-region detector: a command's
//! output is found by the markers it prints, whatever the shell does to the
//! echoed command line.

use crate::output_region::{OutputRow, marked_command, rows_between_markers};

const ID: &str = "4242x7";

fn plain_rows(rows: &[OutputRow]) -> Vec<&str> {
    rows.iter().map(|row| row.plain.as_str()).collect()
}

/// A pane whose raw and plain captures are the same text.
fn rows(pane: &str) -> Option<Vec<OutputRow>> {
    rows_between_markers(pane, pane, ID)
}

#[test]
fn finds_output_after_an_echo_shortened_with_a_leading_angle_bracket() {
    // readline's horizontal scrolling dropped the start of the line,
    // including the begin marker's `printf`.
    let pane = "\
<x-biscuit/target/debug/bt' prose --force-color 'See `md hash` here'; printf 'BTOUT-%s-END\\n' 4242x7
BTOUT-4242x7-BEGIN
See md hash here
BTOUT-4242x7-END
bash-3.2$";
    assert_eq!(rows(pane).map(|r| plain_rows(&r).join("|")), Some("See md hash here".into()));
}

#[test]
fn ignores_an_echo_wrapped_across_rows_at_any_byte() {
    let echo = format!(
        "bash-3.2$ {}",
        marked_command("'/very/long/checkout/target/debug/bt' quote 'see `md hash` here'", ID)
    );
    for width in 1..=echo.len() {
        let wrapped: Vec<String> = echo
            .as_bytes()
            .chunks(width)
            .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
            .collect();
        let pane = format!(
            "{}\nBTOUT-4242x7-BEGIN\n│ see md hash here\nBTOUT-4242x7-END\nbash-3.2$",
            wrapped.join("\n")
        );
        assert_eq!(
            rows(&pane).map(|r| plain_rows(&r).join("|")),
            Some("│ see md hash here".into()),
            "echo wrapped every {width} bytes"
        );
    }
}

#[test]
fn keeps_output_rows_that_look_like_a_prompt_or_repeat_the_command() {
    let pane = "\
bash-3.2$ printf 'BTOUT-%s-BEGIN\\n' 4242x7; cat '/tmp/l2show'; printf 'BTOUT-%s-END\\n' 4242x7
BTOUT-4242x7-BEGIN
bash-3.2$
cat '/tmp/l2show'

total: 100%
root #
BTOUT-4242x7-END
bash-3.2$";
    assert_eq!(
        rows(pane).map(|r| plain_rows(&r).join("|")),
        Some("bash-3.2$|cat '/tmp/l2show'||total: 100%|root #".into())
    );
}

#[test]
fn is_not_ready_until_the_end_marker_is_displayed() {
    let pane = "\
bash-3.2$ printf 'BTOUT-%s-BEGIN\\n' 4242x7; bt prose x; printf 'BTOUT-%s-END\\n' 4242x7
BTOUT-4242x7-BEGIN
x";
    assert_eq!(rows(pane), None);
    assert_eq!(rows("bash-3.2$ printf 'BTOUT-%s-BEGIN\\n' 4242x7; bt prose x"), None);
}

#[test]
fn ignores_markers_left_by_an_earlier_command() {
    let pane = "\
BTOUT-4242x6-BEGIN
stale
BTOUT-4242x6-END
bash-3.2$";
    assert_eq!(rows(pane), None);
}

#[test]
fn pairs_raw_rows_and_splits_output_sharing_the_end_marker_row() {
    let plain = "BTOUT-4242x7-BEGIN\nred\ntailBTOUT-4242x7-END\nbash-3.2$";
    let raw = "BTOUT-4242x7-BEGIN\n\x1b[31mred\x1b[0m\ntail\x1b[0mBTOUT-4242x7-END\nbash-3.2$";
    let found = rows_between_markers(raw, plain, ID).expect("both markers shown");
    assert_eq!(
        found,
        [
            OutputRow { plain: "red".into(), raw: "\x1b[31mred\x1b[0m".into() },
            OutputRow { plain: "tail".into(), raw: "tail\x1b[0m".into() },
        ]
    );
}

/// The command a POSIX shell runs prints the markers around the output, even
/// when the command fails.
#[cfg(unix)]
#[test]
fn a_posix_shell_prints_the_markers_around_the_output() {
    let command = marked_command("printf 'one\\n\\ntwo\\n'; false", ID);
    let output = std::process::Command::new("sh").arg("-c").arg(&command).output().expect("run sh");
    let printed = String::from_utf8(output.stdout).expect("utf-8 output");
    let pane = format!("$ {command}\n{printed}$");
    assert_eq!(rows(&pane).map(|r| plain_rows(&r).join("|")), Some("one||two".into()));
}

#[test]
fn the_command_line_never_spells_out_a_marker() {
    // However the echo wraps or shortens, no row of it can equal a marker.
    let command = marked_command("bt prose 'BTOUT-%s-END'", ID);
    for marker in ["BTOUT-4242x7-BEGIN", "BTOUT-4242x7-END"] {
        assert!(!command.contains(marker), "{marker} is spelled out in the echo: {command}");
    }
}
