use super::summary::StreamExecutionSummary;

/// Line-by-line structured stream parser built around
/// [`super::semantic::SemanticEvent`].
///
/// Each successfully-parsed line yields zero or more semantic events delivered
/// through the parser's owned [`super::semantic::SemanticEventSink`].
///
/// Parsing a line is infallible by contract: a line the parser cannot make
/// sense of becomes a [`super::semantic::SemanticEvent::Warning`], not an
/// error return. Every provider parser already worked this way, so the former
/// `StreamParseError` channel had no producer and its reader — a raw-echo
/// fallback in the stdout reader — was unreachable. A future parser that needs
/// to declare a stream unusable should reintroduce a failure return here rather
/// than signalling it out of band.
pub trait SemanticStreamParser: Send {
    /// Process one line of provider output, emitting zero or more
    /// [`super::semantic::SemanticEvent`]s via the parser's owned sink.
    fn feed_line(&mut self, line: &str);

    /// Finalize parsing and return the accumulated summary.
    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary;
}
