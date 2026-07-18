//! Attributed framing for concurrent task output.
//!
//! When a parallel group runs, N tasks write status to one terminal and
//! interleave in arrival order. Attribution therefore cannot come from ordering
//! — it comes from a colored vertical bar plus a stable textual label carried on
//! every frame (spec → *Reporting Concurrency*).
//!
//! Two rules shape the geometry:
//!
//! - **Serial work uses the same geometry with an invisible bar.** A group that
//!   switches between serial and parallel work must not lurch horizontally, so
//!   [`TaskBar::Invisible`] renders whitespace of exactly the bar's visible
//!   width rather than dropping the prefix.
//! - **Color is never the only attribution.** The header and footer always name
//!   the task, so a `NO_COLOR` terminal loses the palette and keeps the meaning.
//!
//! ## Frames, not writers
//!
//! Following the [`StreamRenderable`][super::StreamRenderable] contract, nothing
//! here holds a `W: Write`. Each method returns fully-rendered *complete* lines;
//! a synchronized sink writes them as one unit, which is what keeps a sibling
//! from tearing an ANSI sequence mid-escape.

use std::time::Duration;

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::{Layout, WordWrap};

/// The fixed palette parallel tasks cycle through.
///
/// Fixed rather than derived so the same fixture renders the same colors on
/// every run, and ordered for adjacent-pair contrast — neighbouring bars are the
/// pair a reader must tell apart.
pub const TASK_PALETTE: [Tailwind; 6] = [
    Tailwind::Cyan400,
    Tailwind::Amber400,
    Tailwind::Violet400,
    Tailwind::Emerald400,
    Tailwind::Rose400,
    Tailwind::Sky400,
];

/// The bar glyph plus its trailing gutter. Two visible columns.
const BAR: &str = "│ ";

/// The invisible bar: whitespace of identical visible width, so serial and
/// parallel frames align to the same left edge.
const BAR_INVISIBLE: &str = "  ";

/// The Unicode header marker, and its ASCII fallback for limited-glyph terminals.
const HEADER_GLYPH: &str = "▶";
const HEADER_GLYPH_ASCII: &str = ">";

/// How one task's frames are attributed in the left gutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBar {
    /// A concurrent task: a colored vertical bar drawn from [`TASK_PALETTE`].
    Colored(Tailwind),
    /// Non-concurrent work: the bar's geometry with nothing drawn in it.
    Invisible,
}

impl TaskBar {
    /// The palette entry for a task at `index`, cycling when tasks outnumber the
    /// palette.
    #[must_use]
    pub fn for_index(index: usize) -> Self {
        Self::Colored(TASK_PALETTE[index % TASK_PALETTE.len()])
    }

    /// The prefix string this bar contributes to every rendered line.
    #[must_use]
    pub fn border(self) -> &'static str {
        match self {
            Self::Colored(_) => BAR,
            Self::Invisible => BAR_INVISIBLE,
        }
    }

    /// The bar's color, when it draws one.
    #[must_use]
    pub fn color(self) -> Option<Color> {
        match self {
            Self::Colored(shade) => Some(Color::Tailwind(shade)),
            Self::Invisible => None,
        }
    }
}

/// How a task finished, as the footer reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStreamOutcome {
    Succeeded,
    Failed,
    Interrupted,
}

impl TaskStreamOutcome {
    /// The footer's outcome word, with the Prose color tag that carries it.
    fn markup(self) -> &'static str {
        match self {
            Self::Succeeded => "<green>succeeded</green>",
            Self::Failed => "<red>failed</red>",
            Self::Interrupted => "<yellow>interrupted</yellow>",
        }
    }
}

/// One frame of an attributed task stream.
///
/// The renderable unit of [`TaskStream`]: a bar-prefixed, width-wrapped block
/// carrying one header, body chunk, or footer.
#[derive(Debug, Clone)]
pub struct TaskStreamFrame {
    bar: TaskBar,
    content: String,
    layout: Layout,
}

impl TaskStreamFrame {
    /// Frame `content` — Prose markup — under `bar`.
    #[must_use]
    pub fn new(bar: TaskBar, content: impl Into<String>) -> Self {
        Self {
            bar,
            content: content.into(),
            layout: Layout::default(),
        }
    }

    /// The quote this frame renders through.
    ///
    /// The default `│ ` border routes `BlockQuote` through the render tree,
    /// which lowers the border color against the terminal's own color depth; an
    /// invisible bar is a custom prefix with no color at all. Neither path can
    /// emit an escape a `ColorDepth::None` terminal did not ask for.
    fn quote(&self) -> BlockQuote {
        match self.bar.color() {
            Some(color) => {
                BlockQuote::from(Prose::new(self.content.clone())).with_left_block_color(color)
            }
            None => {
                // The custom-prefix path folds nothing on its own — only the
                // default-border *tree* path wraps a bordered block — and
                // `Layout::default()` ships `word_wrap: None`. Without this
                // opt-in a serial body line runs past the pane edge and its
                // continuation restarts at column 0 with no gutter, which is
                // exactly the sideways lurch the invisible bar exists to
                // prevent.
                let prose = Prose::new(self.content.clone()).with_word_wrap(WordWrap::default());
                let mut quote = BlockQuote::from(prose).with_border(self.bar.border());
                // `BlockQuote::default` ships a gray border color, and the
                // custom-prefix path paints it unconditionally — which would put
                // a truecolor escape around two spaces on a `NO_COLOR` terminal.
                if let Some(style) = quote.style_mut() {
                    style.border = None;
                }
                quote
            }
        }
    }
}

impl TerminalRenderable for TaskStreamFrame {
    fn render(&self, term: &Terminal) -> String {
        self.quote().render(term)
    }

    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        self.quote().render_optimistic(term_width)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

/// A task's attributed output stream.
///
/// Stateful only in the partial-line sense: [`append`](Self::append) holds a
/// trailing fragment until its newline arrives, so no frame is ever emitted with
/// half a line in it.
#[derive(Debug, Clone)]
pub struct TaskStream {
    label: String,
    bar: TaskBar,
    term: Terminal,
    pending: String,
}

impl TaskStream {
    /// A stream for the task named `label`, attributed by `bar`.
    #[must_use]
    pub fn new(label: impl Into<String>, bar: TaskBar, term: Terminal) -> Self {
        Self {
            label: label.into(),
            bar,
            term,
            pending: String::new(),
        }
    }

    /// The task's stable textual label — the attribution that survives no-color.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The header frame announcing the task.
    #[must_use]
    pub fn open(&mut self) -> Vec<String> {
        let glyph = if self.term.supports_unicode {
            HEADER_GLYPH
        } else {
            HEADER_GLYPH_ASCII
        };
        self.frame(&format!(
            "{glyph} <b>{}</b>",
            Prose::escape_text(&self.label)
        ))
    }

    /// Frame every *complete* line in `chunk`, holding any trailing fragment.
    #[must_use]
    pub fn append(&mut self, chunk: &str) -> Vec<String> {
        self.pending.push_str(chunk);
        let Some(last_newline) = self.pending.rfind('\n') else {
            return Vec::new();
        };
        let complete: String = self.pending.drain(..=last_newline).collect();
        complete
            .lines()
            .flat_map(|line| self.frame(&Prose::escape_text(line)))
            .collect()
    }

    /// Frame whatever fragment is still held, without waiting for a newline.
    ///
    /// Separate from [`close`](Self::close) because the two land on different
    /// channels: a held fragment is task *data* and belongs on stdout, while the
    /// footer is status and belongs on stderr.
    #[must_use]
    pub fn flush(&mut self) -> Vec<String> {
        if self.pending.is_empty() {
            return Vec::new();
        }
        let held = std::mem::take(&mut self.pending);
        self.frame(&Prose::escape_text(held.trim_end_matches('\n')))
    }

    /// The footer frame carrying outcome and duration.
    ///
    /// Any fragment still held is *not* emitted here — call
    /// [`flush`](Self::flush) first so it lands on the data channel.
    #[must_use]
    pub fn close(&mut self, outcome: TaskStreamOutcome, duration: Duration) -> Vec<String> {
        self.frame(&format!(
            "<b>{}</b> — {} <dim><i>({:.1}s)</i></dim>",
            Prose::escape_text(&self.label),
            outcome.markup(),
            duration.as_secs_f64()
        ))
    }

    /// Render one Prose-markup chunk into complete, bar-prefixed lines.
    fn frame(&self, markup: &str) -> Vec<String> {
        TaskStreamFrame::new(self.bar, markup)
            .render(&self.term)
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// The rendered gutter every frame of this stream begins with.
    ///
    /// Taken from a real frame rather than rebuilt, so a line prefixed with it
    /// is byte-identical in the gutter to one this stream framed itself — which
    /// is what lets a reader match a body line to its header by color.
    ///
    /// ## Panics
    ///
    /// Panics if the sentinel does not survive framing, which would mean
    /// [`TaskStreamFrame`] no longer renders its content verbatim.
    #[must_use]
    pub fn gutter(&self) -> String {
        const SENTINEL: &str = "x";
        let framed = TaskStreamFrame::new(self.bar, SENTINEL).render(&self.term);
        let at = framed
            .find(SENTINEL)
            .expect("a task frame must render its content");
        framed[..at].to_string()
    }
}

/// An owned, thread-safe handle that frames **already-rendered** lines.
///
/// The provider stdout path needs this rather than [`TaskStream`] for two
/// reasons, both consequences of the text arriving pre-rendered:
///
/// - **No escaping.** `TaskStream::append` runs its input through Prose, which
///   would show a rendered line's ANSI as literal text.
/// - **No re-wrapping.** The upstream renderer already wrapped to its terminal
///   width. Re-wrapping would fight it; instead the caller narrows *that*
///   renderer by the gutter's width so the framed result still fits.
///
/// Owned and `Send` because the stdout reader runs on its own thread, so a
/// borrow of the task's [`TaskLiveOutput`] cannot reach it.
#[derive(Clone)]
pub struct TaskFrameWriter {
    gutter: String,
    sink: std::sync::Arc<dyn TaskStreamSink>,
    /// Held partial line. Cloning starts a fresh buffer: two writers must never
    /// share one half-written line.
    pending: String,
}

impl TaskFrameWriter {
    /// The visible width the gutter occupies, for narrowing the renderer that
    /// feeds this writer.
    #[must_use]
    pub fn gutter_width(&self) -> usize {
        BAR.chars().count()
    }

    /// The rendered gutter this writer prefixes every line with.
    ///
    /// Exposed so a task's *status* channel can be decorated with the identical
    /// bytes its data channel uses. Attribution only reads as one stream if the
    /// gutter matches exactly — a rebuilt one would differ in color depth.
    #[must_use]
    pub fn gutter(&self) -> &str {
        &self.gutter
    }

    /// Frame every *complete* line in `chunk`, holding any trailing fragment.
    pub fn write(&mut self, chunk: &str) {
        self.pending.push_str(chunk);
        let Some(last_newline) = self.pending.rfind('\n') else {
            return;
        };
        let complete: String = self.pending.drain(..=last_newline).collect();
        let frames: Vec<String> = complete
            .lines()
            .map(|line| format!("{}{line}", self.gutter))
            .collect();
        if !frames.is_empty() {
            self.sink.write_data_frames(&frames);
        }
    }

    /// Emit whatever fragment is still held.
    pub fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let held = std::mem::take(&mut self.pending);
        let frames = vec![format!("{}{}", self.gutter, held.trim_end_matches('\n'))];
        self.sink.write_data_frames(&frames);
    }
}

impl std::fmt::Debug for TaskFrameWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskFrameWriter")
            .field("pending_len", &self.pending.len())
            .finish_non_exhaustive()
    }
}

/// A synchronized destination for rendered task frames.
///
/// The whole point of the seam is atomicity: an implementation must write all
/// the frames of one call as an uninterrupted unit, so a sibling task cannot
/// land a line — or half an ANSI escape — inside another's frame.
///
/// The two methods are two *channels*, not two formats. Claudine's command
/// contract puts task and provider data on stdout and status rendering on
/// stderr, so a sink that collapsed them would break `2>/dev/null` and
/// `| jq` alike (spec → *Reporting Concurrency*).
/// `Send` as well as `Sync` because a provider session's stdout is drained on a
/// spawned reader thread, and that thread owns the handle that frames its lines.
pub trait TaskStreamSink: Send + Sync {
    /// Status framing — headers, footers, warnings. Stderr.
    fn write_frames(&self, frames: &[String]);

    /// Task/provider data. Stdout.
    ///
    /// The bar is display decoration only: the payload captured for `outputs`
    /// is the undecorated text these frames were rendered *from*, never the
    /// rendered line (spec → *Output capture boundary*).
    fn write_data_frames(&self, frames: &[String]);
}

/// One task's live output: a [`TaskStream`] bound to a [`TaskStreamSink`].
///
/// This is the seam that puts a task's *body* inside its own framing. Without
/// it a task's header and footer are attributed but everything between them is
/// not, which is the whole point of the color bar.
///
/// Interior mutability because the executor holds it behind a shared reference
/// while stages run; the mutex is also what makes the held-fragment state safe
/// when a runner emits from a helper thread.
pub struct TaskLiveOutput {
    stream: std::sync::Mutex<TaskStream>,
    /// Shared rather than borrowed so [`rendered_writer`](Self::rendered_writer)
    /// can hand an owned handle to a provider's stdout reader thread.
    sink: std::sync::Arc<dyn TaskStreamSink>,
}

/// A poisoned stream means the emitting task panicked; later frames from it
/// would be attributed to a task that is no longer running.
const STREAM_POISONED: &str = "task live output poisoned by a panicking task";

impl TaskLiveOutput {
    /// Bind `stream` to `sink`.
    #[must_use]
    pub fn new(stream: TaskStream, sink: std::sync::Arc<dyn TaskStreamSink>) -> Self {
        Self {
            stream: std::sync::Mutex::new(stream),
            sink,
        }
    }

    /// An owned handle that frames already-rendered lines under this task's bar.
    ///
    /// The gutter is captured from this stream's own framing, so lines written
    /// through the returned writer carry the same bar — and the same color — as
    /// the header and footer that bracket them.
    #[must_use]
    pub fn rendered_writer(&self) -> TaskFrameWriter {
        TaskFrameWriter {
            gutter: self.locked().gutter(),
            sink: std::sync::Arc::clone(&self.sink),
            pending: String::new(),
        }
    }

    /// Announce the task on the status channel.
    pub fn open(&self) {
        let frames = self.locked().open();
        self.status(&frames);
    }

    /// Frame every complete line of `chunk` onto the data channel.
    pub fn append(&self, chunk: &str) {
        let frames = self.locked().append(chunk);
        self.data(&frames);
    }

    /// Emit a captured payload that will receive no further chunks.
    ///
    /// Captured stdout arrives with its transport newline already removed, so
    /// [`append`](Self::append) alone would hold the final line forever.
    pub fn emit(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        let mut stream = self.locked();
        let mut frames = stream.append(text);
        frames.extend(stream.flush());
        drop(stream);
        self.data(&frames);
    }

    /// Flush any held fragment to the data channel, then close on the status
    /// channel with the outcome footer.
    pub fn close(&self, outcome: TaskStreamOutcome, duration: Duration) {
        let mut stream = self.locked();
        let held = stream.flush();
        let footer = stream.close(outcome, duration);
        drop(stream);
        self.data(&held);
        self.status(&footer);
    }

    /// The task's stable textual label.
    #[must_use]
    pub fn label(&self) -> String {
        self.locked().label().to_string()
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, TaskStream> {
        self.stream.lock().expect(STREAM_POISONED)
    }

    fn status(&self, frames: &[String]) {
        if !frames.is_empty() {
            self.sink.write_frames(frames);
        }
    }

    fn data(&self, frames: &[String]) {
        if !frames.is_empty() {
            self.sink.write_data_frames(frames);
        }
    }
}

impl std::fmt::Debug for TaskLiveOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskLiveOutput")
            .field("label", &self.label())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
