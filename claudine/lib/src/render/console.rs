//! Where the library's own terminal lines go.
//!
//! A few library paths print straight to the terminal: lifecycle `stderr`,
//! `info`, `warn`, `success`, and `stdout` actions, harness status reports,
//! messaging-failure warnings, overlay recovery notices, and hook `report`
//! actions. An application that delivers terminal output from a writer thread
//! of its own installs a [`ConsoleWriter`] once, so these lines join that
//! output instead of writing to a terminal its writer may be stuck on. The
//! Claudine CLI does this, and queues them during a wrapped run.
//!
//! Without a writer, each line is printed with `eprintln!` or `println!`, as
//! the library always has.
//!
//! ## Examples
//!
//! ```
//! use claudine::render::console::{ConsoleStream, set_console_writer, write_stderr_line};
//!
//! fn to_my_writer(stream: ConsoleStream, text: &str) {
//!     let _ = (stream, text); // queue `text`, which ends in a newline
//! }
//!
//! set_console_writer(to_my_writer);
//! write_stderr_line("shown through to_my_writer");
//! ```

use std::sync::OnceLock;

/// Which standard stream a console line belongs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleStream {
    Stdout,
    Stderr,
}

/// Receives one whole line, newline included. Must not block on the terminal.
pub type ConsoleWriter = fn(ConsoleStream, &str);

static WRITER: OnceLock<ConsoleWriter> = OnceLock::new();

/// Send every later console line to `writer`; returns `false`, changing
/// nothing, when a writer was already installed.
pub fn set_console_writer(writer: ConsoleWriter) -> bool {
    WRITER.set(writer).is_ok()
}

/// Print `line` and a newline to stderr, through the installed writer if any.
pub fn write_stderr_line(line: &str) {
    match WRITER.get() {
        Some(writer) => writer(ConsoleStream::Stderr, &format!("{line}\n")),
        None => eprintln!("{line}"),
    }
}

/// Print `line` and a newline to stdout, through the installed writer if any.
pub fn write_stdout_line(line: &str) {
    match WRITER.get() {
        Some(writer) => writer(ConsoleStream::Stdout, &format!("{line}\n")),
        None => println!("{line}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static RECEIVED: Mutex<Vec<(ConsoleStream, String)>> = Mutex::new(Vec::new());

    fn record(stream: ConsoleStream, text: &str) {
        RECEIVED.lock().unwrap().push((stream, text.to_string()));
    }

    #[test]
    fn an_installed_writer_receives_each_whole_line_and_cannot_be_replaced() {
        assert!(set_console_writer(record));
        assert!(!set_console_writer(record), "the first writer stays installed");

        write_stderr_line("status");
        write_stdout_line("data");

        assert_eq!(
            *RECEIVED.lock().unwrap(),
            [
                (ConsoleStream::Stderr, "status\n".to_string()),
                (ConsoleStream::Stdout, "data\n".to_string()),
            ]
        );
    }
}
