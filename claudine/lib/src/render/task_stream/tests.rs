//! Renderer contract for attributed task frames.
//!
//! Every case here asserts something a reader can see: the left edge, the
//! attribution that survives no-color, or the absence of a torn escape.

use std::time::Duration;

use biscuit_terminal::discovery::detection::color::ColorDepth;
use biscuit_terminal::terminal::Terminal;

use super::*;

/// A color-capable terminal of a chosen width.
fn color_term(width: u32) -> Terminal {
    Terminal::builder()
        .width(width)
        .color_depth(ColorDepth::TrueColor)
        .supports_unicode(true)
        .is_tty(true)
        .build()
}

/// The `NO_COLOR` / non-TTY shape claudine's `log::plain_terminal` produces.
fn plain_term(width: u32) -> Terminal {
    Terminal::builder()
        .width(width)
        .color_depth(ColorDepth::None)
        .supports_unicode(true)
        .is_tty(false)
        .build()
}

/// Visible width after stripping every SGR/OSC sequence.
fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }
        // CSI: consume through the final byte in `@`..=`~`.
        for next in chars.by_ref() {
            if ('\u{40}'..='\u{7e}').contains(&next) && next != '[' {
                break;
            }
        }
    }
    out
}

/// The column `needle` starts at, counting characters rather than bytes.
fn column_of(text: &str, needle: &str) -> Option<usize> {
    text.find(needle).map(|byte| text[..byte].chars().count())
}

mod geometry {
    use super::*;

    #[test]
    fn a_serial_frame_and_a_parallel_frame_share_one_left_edge() {
        let term = color_term(60);
        let parallel = TaskStreamFrame::new(TaskBar::for_index(0), "hello").render(&term);
        let serial = TaskStreamFrame::new(TaskBar::Invisible, "hello").render(&term);

        let parallel_text = strip_ansi(&parallel);
        // Columns, not bytes: `│` is one column and three UTF-8 bytes. The bar
        // occupies exactly the columns the invisible bar leaves blank, so the
        // content starts at the same column in both modes.
        assert_eq!(
            column_of(&parallel_text, "hello"),
            column_of(&serial, "hello")
        );
        assert!(parallel_text.starts_with("│ "), "got {parallel_text:?}");
        assert!(serial.starts_with("  "), "got {serial:?}");
    }

    #[test]
    fn the_invisible_bar_draws_nothing_but_holds_its_columns() {
        let rendered = TaskStreamFrame::new(TaskBar::Invisible, "x").render(&color_term(40));
        assert!(!rendered.contains('│'));
        assert_eq!(TaskBar::Invisible.border().chars().count(), 2);
        assert_eq!(rendered.find('x'), Some(2));
    }

    #[test]
    fn long_content_wraps_inside_the_bar_rather_than_past_it() {
        let text = "alpha bravo charlie delta echo foxtrot golf hotel india juliett";
        let rendered = TaskStreamFrame::new(TaskBar::for_index(1), text).render(&color_term(30));
        let lines: Vec<&str> = rendered.lines().collect();
        assert!(lines.len() > 1, "expected a wrap at width 30: {rendered:?}");
        for line in &lines {
            let visible = strip_ansi(line);
            assert!(
                visible.starts_with("│ "),
                "every wrapped line keeps the bar: {visible:?}"
            );
            assert!(
                visible.chars().count() <= 30,
                "line overflows the terminal: {visible:?}"
            );
        }
    }

    #[test]
    fn a_narrow_terminal_still_renders_a_barred_line() {
        let rendered = TaskStreamFrame::new(TaskBar::for_index(0), "abcdefghij").render(&color_term(8));
        for line in rendered.lines() {
            let visible = strip_ansi(line);
            assert!(visible.starts_with("│ "), "got {visible:?}");
            assert!(visible.chars().count() <= 8, "got {visible:?}");
        }
    }

    #[test]
    fn wide_unicode_content_survives_framing() {
        let rendered =
            TaskStreamFrame::new(TaskBar::for_index(2), "日本語のタスク — ✅ 完了").render(&color_term(40));
        let visible = strip_ansi(&rendered);
        assert!(visible.contains("日本語のタスク"), "got {visible:?}");
        assert!(visible.contains('✅'), "got {visible:?}");
    }
}

mod palette {
    use super::*;

    #[test]
    fn the_palette_cycles_once_tasks_outnumber_it() {
        let len = TASK_PALETTE.len();
        assert_eq!(TaskBar::for_index(0), TaskBar::for_index(len));
        assert_eq!(TaskBar::for_index(1), TaskBar::for_index(len + 1));
        assert_ne!(TaskBar::for_index(0), TaskBar::for_index(1));
    }

    #[test]
    fn adjacent_tasks_never_share_a_color() {
        for index in 0..TASK_PALETTE.len() * 3 {
            assert_ne!(
                TaskBar::for_index(index),
                TaskBar::for_index(index + 1),
                "tasks {index} and {} collide",
                index + 1
            );
        }
    }

    #[test]
    fn distinct_palette_entries_produce_distinct_rendered_bars() {
        let term = color_term(40);
        let first = TaskStreamFrame::new(TaskBar::for_index(0), "x").render(&term);
        let second = TaskStreamFrame::new(TaskBar::for_index(1), "x").render(&term);
        assert_ne!(first, second, "two tasks rendered an identical bar");
    }
}

mod degradation {
    use super::*;

    #[test]
    fn a_no_color_terminal_emits_no_escape_sequences() {
        let term = plain_term(60);
        for bar in [TaskBar::for_index(0), TaskBar::Invisible] {
            let rendered = TaskStreamFrame::new(bar, "<b>bold</b> content").render(&term);
            assert!(
                !rendered.contains('\u{1b}'),
                "{bar:?} leaked an escape under ColorDepth::None: {rendered:?}"
            );
        }
    }

    #[test]
    fn attribution_survives_without_color() {
        let mut stream = TaskStream::new("build docs", TaskBar::for_index(0), plain_term(60));
        let header = stream.open().join("\n");
        let footer = stream
            .close(TaskStreamOutcome::Succeeded, Duration::from_millis(1500))
            .join("\n");

        // With no color at all, the task name is the only attribution left — so
        // it must be on both ends of the stream.
        assert!(header.contains("build docs"), "got {header:?}");
        assert!(footer.contains("build docs"), "got {footer:?}");
        assert!(footer.contains("succeeded"), "got {footer:?}");
        assert!(footer.contains("1.5s"), "got {footer:?}");
    }

    #[test]
    fn a_limited_glyph_terminal_uses_the_ascii_header_marker() {
        let term = Terminal::builder()
            .width(60)
            .color_depth(ColorDepth::None)
            .supports_unicode(false)
            .build();
        let header = TaskStream::new("task", TaskBar::Invisible, term)
            .open()
            .join("\n");
        assert!(header.contains("> task"), "got {header:?}");
        assert!(!header.contains('▶'), "got {header:?}");
    }
}

mod streaming {
    use super::*;

    #[test]
    fn a_partial_line_is_held_until_its_newline_arrives() {
        let mut stream = TaskStream::new("t", TaskBar::Invisible, plain_term(60));
        assert!(stream.append("half a ").is_empty());
        let frames = stream.append("line\nsecond line\n");
        assert_eq!(frames.len(), 2, "got {frames:?}");
        assert!(frames[0].contains("half a line"), "got {frames:?}");
        assert!(frames[1].contains("second line"), "got {frames:?}");
    }

    #[test]
    fn close_flushes_a_held_fragment_before_the_footer() {
        let mut stream = TaskStream::new("t", TaskBar::Invisible, plain_term(60));
        assert!(stream.append("trailing fragment").is_empty());
        let frames = stream.close(TaskStreamOutcome::Failed, Duration::from_secs(2));
        assert!(frames[0].contains("trailing fragment"), "got {frames:?}");
        assert!(
            frames.last().is_some_and(|line| line.contains("failed")),
            "got {frames:?}"
        );
    }

    #[test]
    fn every_emitted_frame_is_a_complete_line() {
        let mut stream = TaskStream::new("t", TaskBar::for_index(0), color_term(30));
        let mut frames = stream.open();
        frames.extend(stream.append("one two three four five six seven eight\n"));
        frames.extend(stream.close(TaskStreamOutcome::Succeeded, Duration::from_secs(1)));

        for frame in &frames {
            assert!(!frame.contains('\n'), "frame carries a newline: {frame:?}");
            // A torn ANSI sequence is an ESC without its terminating byte.
            let escapes = frame.matches('\u{1b}').count();
            let terminators = frame.matches('m').count();
            assert!(
                escapes == 0 || terminators >= escapes / 2,
                "possible torn escape in {frame:?}"
            );
        }
    }

    #[test]
    fn interrupted_tasks_report_their_own_outcome_word() {
        let mut stream = TaskStream::new("t", TaskBar::Invisible, plain_term(60));
        let footer = stream
            .close(TaskStreamOutcome::Interrupted, Duration::from_millis(300))
            .join("\n");
        assert!(footer.contains("interrupted"), "got {footer:?}");
        assert!(footer.contains("0.3s"), "got {footer:?}");
    }

    #[test]
    fn prose_markup_in_task_output_is_escaped_not_interpreted() {
        let mut stream = TaskStream::new("t", TaskBar::Invisible, plain_term(60));
        let frames = stream.append("<b>not bold</b>\n").join("\n");
        assert!(frames.contains("<b>not bold</b>"), "got {frames:?}");
    }
}

mod sink {
    use std::sync::Mutex;

    use super::*;

    /// A sink that records whole write calls, so a torn write is visible as a
    /// split group rather than needing a timing assertion.
    #[derive(Default)]
    struct RecordingSink {
        writes: Mutex<Vec<Vec<String>>>,
    }

    impl TaskStreamSink for RecordingSink {
        fn write_frames(&self, frames: &[String]) {
            self.writes.lock().expect("sink poisoned").push(frames.to_vec());
        }
    }

    #[test]
    fn concurrent_writers_never_interleave_within_one_frame_group() {
        let sink = RecordingSink::default();
        std::thread::scope(|scope| {
            for index in 0..8 {
                let sink = &sink;
                scope.spawn(move || {
                    let mut stream =
                        TaskStream::new(format!("task {index}"), TaskBar::for_index(index), color_term(60));
                    sink.write_frames(&stream.open());
                    sink.write_frames(&stream.append("line one\nline two\n"));
                    sink.write_frames(&stream.close(
                        TaskStreamOutcome::Succeeded,
                        Duration::from_millis(10),
                    ));
                });
            }
        });

        let writes = sink.writes.into_inner().expect("sink poisoned");
        assert_eq!(writes.len(), 24, "expected 3 writes per task");
        // Attribution is per-write, not per-position: every group of frames
        // belongs to exactly one task.
        for group in &writes {
            let labelled: Vec<&String> = group
                .iter()
                .filter(|line| line.contains("task "))
                .collect();
            let names: std::collections::HashSet<String> = labelled
                .iter()
                .map(|line| {
                    let stripped = strip_ansi(line);
                    let start = stripped.find("task ").expect("label present");
                    stripped[start..start + 6].to_string()
                })
                .collect();
            assert!(names.len() <= 1, "a frame group mixed tasks: {group:?}");
        }
    }
}
