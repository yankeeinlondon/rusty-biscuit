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
/// stream (the capture path has no section stream and passes `None`).
pub(crate) fn emit_final_message(
    text: &str,
    term: &Terminal,
    section_stream: Option<&SectionStream>,
) -> std::io::Result<()> {
    if let Some(stream) = section_stream {
        stream.enter_final_stdout();
    }
    let mut stdout = std::io::stdout();
    if stdout.is_terminal() {
        let rendered = FinalMessage::new(text).render(term);
        stdout.write_all(rendered.as_bytes())?;
        if !rendered.ends_with('\n') {
            stdout.write_all(b"\n")?;
        }
    } else {
        stdout.write_all(text.as_bytes())?;
        if !text.ends_with('\n') {
            stdout.write_all(b"\n")?;
        }
    }
    stdout.flush()
}
