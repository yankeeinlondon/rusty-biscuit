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
use biscuit_terminal::utils::layout::Layout;

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
        let prose = Prose::new(self.content.clone());
        let quote = BlockQuote::from(prose);
        match self.bar.color() {
            Some(color) => quote.with_left_block_color(color),
            None => {
                let mut quote = quote.with_border(self.bar.border());
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

    /// Flush any held fragment, then emit the footer carrying outcome and
    /// duration.
    #[must_use]
    pub fn close(&mut self, outcome: TaskStreamOutcome, duration: Duration) -> Vec<String> {
        let mut frames = Vec::new();
        if !self.pending.is_empty() {
            let held = std::mem::take(&mut self.pending);
            frames.extend(self.frame(&Prose::escape_text(held.trim_end_matches('\n'))));
        }
        frames.extend(self.frame(&format!(
            "<b>{}</b> — {} <dim><i>({:.1}s)</i></dim>",
            Prose::escape_text(&self.label),
            outcome.markup(),
            duration.as_secs_f64()
        )));
        frames
    }

    /// Render one Prose-markup chunk into complete, bar-prefixed lines.
    fn frame(&self, markup: &str) -> Vec<String> {
        TaskStreamFrame::new(self.bar, markup)
            .render(&self.term)
            .lines()
            .map(str::to_string)
            .collect()
    }
}

/// A synchronized destination for rendered task frames.
///
/// The whole point of the seam is atomicity: an implementation must write all
/// the frames of one call as an uninterrupted unit, so a sibling task cannot
/// land a line — or half an ANSI escape — inside another's frame.
pub trait TaskStreamSink: Sync {
    /// Write one task's already-rendered lines as one indivisible unit.
    fn write_frames(&self, frames: &[String]);
}

#[cfg(test)]
mod tests;
