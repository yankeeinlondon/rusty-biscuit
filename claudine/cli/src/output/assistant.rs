use std::io::{IsTerminal, Write};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::render::FinalMessage;

use crate::commands::wrap::section::SectionStream;

/// Write the agent's final message to stdout: rendered through
/// [`FinalMessage`] when stdout is a TTY, raw bytes otherwise. Guarantees a
/// trailing newline and flushes.
///
/// When `section_stream` is provided, the `FinalStdout` section transition
/// is recorded first so section spacing stays consistent with the live
/// stream (the capture path has no section stream and passes `None`). With a
/// section stream the bytes are queued on the run's output worker, like the
/// rest of the run's output; without one they are written directly.
pub(crate) fn emit_final_message(
    text: &str,
    term: &Terminal,
    section_stream: Option<&SectionStream>,
) -> std::io::Result<()> {
    if let Some(stream) = section_stream {
        stream.enter_final_stdout();
    }
    let is_terminal = std::io::stdout().is_terminal();
    let mut out: Box<dyn Write> = match section_stream {
        Some(stream) => Box::new(stream.stdout_writer()),
        None => Box::new(std::io::stdout()),
    };
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
