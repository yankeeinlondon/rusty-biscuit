//! Recording sink for asserting that a task's bar reaches a given emission path.
//!
//! The bypass tests all ask the same question — did this line arrive framed? —
//! so they share one sink that keeps the two channels apart, which is also what
//! lets them assert data did not leak onto the status channel or back.

use std::sync::{Arc, Mutex};

use biscuit_terminal::discovery::detection::color::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use claudine::render::{TaskBar, TaskFrameWriter, TaskLiveOutput, TaskStream, TaskStreamSink};
use biscuit_terminal::utils::color::Tailwind;

/// Frames captured per channel.
#[derive(Default)]
pub(crate) struct RecordedFrames {
    pub(crate) status: Vec<String>,
    pub(crate) data: Vec<String>,
}

/// A [`TaskStreamSink`] that records instead of writing.
pub(crate) struct RecordingSink {
    frames: Arc<Mutex<RecordedFrames>>,
}

impl RecordingSink {
    pub(crate) fn new() -> (Arc<Self>, Arc<Mutex<RecordedFrames>>) {
        let frames = Arc::new(Mutex::new(RecordedFrames::default()));
        (
            Arc::new(Self {
                frames: Arc::clone(&frames),
            }),
            frames,
        )
    }
}

impl TaskStreamSink for RecordingSink {
    fn write_frames(&self, frames: &[String]) {
        self.frames
            .lock()
            .unwrap()
            .status
            .extend(frames.iter().cloned());
    }

    fn write_data_frames(&self, frames: &[String]) {
        self.frames
            .lock()
            .unwrap()
            .data
            .extend(frames.iter().cloned());
    }
}

/// A colored task writer plus the buffer it records into and its gutter.
///
/// Colored rather than [`TaskBar::Invisible`] so an assertion that a line
/// carries the bar cannot pass on whitespace alone.
pub(crate) fn colored_writer() -> (TaskFrameWriter, Arc<Mutex<RecordedFrames>>, String) {
    let term = Terminal::builder()
        .width(80)
        .color_depth(ColorDepth::TrueColor)
        .supports_unicode(true)
        .is_tty(true)
        .build();
    let (sink, frames) = RecordingSink::new();
    let live = TaskLiveOutput::new(
        TaskStream::new("alpha", TaskBar::Colored(Tailwind::Cyan400), term),
        sink as Arc<dyn TaskStreamSink>,
    );
    let writer = live.rendered_writer();
    let gutter = writer.gutter().to_string();
    (writer, frames, gutter)
}
