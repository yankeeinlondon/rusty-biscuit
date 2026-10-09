use std::io::{IsTerminal, Write};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::render::{FinalMessage, TaskFrameWriter};

use crate::commands::wrap::section::SectionStream;
use crate::commands::wrap::stream_io::StreamOutput;

/// Queue the agent's final message for stdout: rendered through
/// [`FinalMessage`] when stdout is a TTY, raw bytes otherwise. Guarantees a
/// trailing newline.
///
/// When `section_stream` is provided, the `FinalStdout` section transition
/// is recorded first so section spacing stays consistent with the live
/// stream (the capture path has no section stream and passes `None`). The
/// bytes go to the run's output worker either way, the process-wide one
/// without a section stream, so the caller waits for the terminal with a
/// bounded drain instead of a write that can block.
pub(crate) fn emit_final_message(
    text: &str,
    term: &Terminal,
    section_stream: Option<&SectionStream>,
) -> std::io::Result<()> {
    if let Some(stream) = section_stream {
        stream.enter_final_stdout();
    }
    let mut out = match section_stream {
        Some(stream) => stream.stdout_writer(),
        None => StreamOutput::shared().stdout_writer(),
    };
    write_final_message(text, term, std::io::stdout().is_terminal(), &mut out)
}

/// [`emit_final_message`] for a sequence task: every line goes through the
/// task's `writer`, so it carries the task gutter, and a TTY render is folded
/// to the terminal width less that gutter.
///
/// Returns nothing because the writer's sink queues frames on the output
/// worker, which tracks its own delivery loss, exactly as for the streamed
/// text above the answer.
pub(crate) fn emit_framed_final_message(
    text: &str,
    term: &Terminal,
    section_stream: &SectionStream,
    mut writer: TaskFrameWriter,
) {
    section_stream.enter_final_stdout();
    let rendered = if std::io::stdout().is_terminal() {
        let mut inset = term.clone();
        inset.fixed_width = Some(
            term.width()
                .saturating_sub(writer.gutter_width() as u32)
                .max(1),
        );
        FinalMessage::new(text).render(&inset)
    } else {
        text.to_string()
    };
    writer.write(&rendered);
    writer.flush();
}

/// The bytes of [`emit_final_message`], written to `out`.
pub(crate) fn write_final_message(
    text: &str,
    term: &Terminal,
    is_terminal: bool,
    out: &mut dyn Write,
) -> std::io::Result<()> {
    if is_terminal {
        let rendered = FinalMessage::new(text).render(term);
        out.write_all(rendered.as_bytes())?;
        if !rendered.ends_with('\n') {
            out.write_all(b"\n")?;
        }
    } else {
        out.write_all(text.as_bytes())?;
        if !text.ends_with('\n') {
            out.write_all(b"\n")?;
        }
    }
    out.flush()
}
