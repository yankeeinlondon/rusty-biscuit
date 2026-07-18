//! The wrapper's destination for attributed group-task frames.
//!
//! The library renders a task's header and footer; this is where those lines
//! meet the terminal. Two contracts hold here:
//!
//! - **Stderr, not stdout.** Headers, footers, status, and warnings are status
//!   rendering; provider and task *data* stays on stdout undecorated, which is
//!   also what keeps the `outputs` capture boundary clean.
//! - **One coordinator.** Frames go through the process-wide
//!   [`StreamOutput`], so a sibling task cannot land a line — or half an ANSI
//!   escape — inside another task's frame group.

use std::sync::Arc;

use claudine::render::TaskStreamSink;

use crate::commands::wrap::stream_io::StreamOutput;

/// Writes group-task frames to stderr through the shared coordinator.
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
}
