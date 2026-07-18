//! The wrapper's destination for attributed task frames.
//!
//! The library renders a task's header, body, and footer; this is where those
//! lines meet the terminal. Two contracts hold here:
//!
//! - **Two channels, one contract.** Headers, footers, and warnings are status
//!   rendering and go to stderr; task and provider *data* goes to stdout. The
//!   bar the data carries is decoration added on the way out — the payload
//!   captured for `outputs` is the undecorated text it was rendered from, which
//!   is what keeps the capture boundary clean.
//! - **One coordinator.** Both channels go through the process-wide
//!   [`StreamOutput`], so a sibling task cannot land a line — or half an ANSI
//!   escape — inside another task's frame group.

use std::sync::Arc;

use claudine::render::TaskStreamSink;

use crate::commands::wrap::stream_io::StreamOutput;

/// Writes task frames to their channel through the shared coordinator.
pub(crate) struct SequenceTaskSink {
    output: Arc<StreamOutput>,
}

impl SequenceTaskSink {
    /// A sink bound to the process-wide stdout/stderr coordinator.
    pub(crate) fn new() -> Self {
        Self {
            output: StreamOutput::shared(),
        }
    }
}

impl TaskStreamSink for SequenceTaskSink {
    fn write_frames(&self, frames: &[String]) {
        if frames.is_empty() {
            return;
        }
        self.output.emit_stderr_frames(frames);
    }

    fn write_data_frames(&self, frames: &[String]) {
        if frames.is_empty() {
            return;
        }
        self.output.emit_stdout_frames(frames);
    }
}
