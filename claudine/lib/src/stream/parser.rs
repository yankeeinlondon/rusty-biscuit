use super::summary::StreamExecutionSummary;

/// Errors during stream parsing.
#[derive(Debug, thiserror::Error)]
pub enum StreamParseError {
    #[error("Malformed JSON on line {line_num}: {message}")]
    MalformedLine { line_num: usize, message: String },
    #[error("Stream unusable: {0}")]
    Fatal(String),
}

/// Line-by-line structured stream parser built around
/// [`super::semantic::SemanticEvent`].
///
/// Each successfully-parsed line yields zero or more semantic events delivered
/// through the parser's owned [`super::semantic::SemanticEventSink`].
/// Malformed JSON lines emit [`super::semantic::SemanticEvent::Warning`] and
/// return `Ok(())` rather than propagating an error.
pub trait SemanticStreamParser: Send {
    /// Process one line of provider output, emitting zero or more
    /// [`super::semantic::SemanticEvent`]s via the parser's owned sink.
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError>;

    /// Finalize parsing and return the accumulated summary.
    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary;
}
