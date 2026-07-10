use biscuit_terminal::utils::UnicodeWidthStr;
use pulldown_cmark::{CowStr, Event, Tag, TagEnd};

pub(super) fn align_tables_in_stream(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut result = Vec::with_capacity(events.len());
    let mut table_buffer = Vec::new();
    let mut in_table = false;

    for event in events {
        match &event {
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                table_buffer.clear();
                table_buffer.push(event);
            }
            Event::End(TagEnd::Table) => {
                table_buffer.push(event);
                in_table = false;

                // Process the buffered table
                let aligned = process_single_table(table_buffer.clone());
                result.extend(aligned);
                table_buffer.clear();
            }
            _ => {
                if in_table {
                    table_buffer.push(event);
                } else {
                    result.push(event);
                }
            }
        }
    }

    result
}

/// Calculates the rendered width of table cell content.
///
/// This struct tracks nested formatting elements (links, emphasis, strong) and
/// calculates the actual visual width of the rendered markdown output.
struct CellWidthCalculator {
    /// Current accumulated width
    width: usize,
    /// Stack of active formatting contexts with their overhead
    /// (start_overhead, end_overhead, url_width for links)
    format_stack: Vec<FormatContext>,
}

/// Represents an active formatting context within a table cell.
#[derive(Debug, Clone)]
enum FormatContext {
    /// Emphasis: *text* or _text_ - adds 2 chars total
    Emphasis,
    /// Strong: **text** or __text__ - adds 4 chars total
    Strong,
    /// Link: [text](url) - adds 4 chars + url length
    Link { url_width: usize },
}

impl CellWidthCalculator {
    fn new() -> Self {
        Self {
            width: 0,
            format_stack: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.width = 0;
        self.format_stack.clear();
    }

    fn add_text(&mut self, text: &str) {
        self.width += UnicodeWidthStr::width(text);
    }

    fn add_code(&mut self, code: &str) {
        // Code spans render with backticks: `code`
        self.width += UnicodeWidthStr::width(code) + 2;
    }

    fn start_emphasis(&mut self) {
        // Opening marker: * or _
        self.width += 1;
        self.format_stack.push(FormatContext::Emphasis);
    }

    fn end_emphasis(&mut self) {
        // Closing marker: * or _
        self.width += 1;
        self.format_stack.pop();
    }

    fn start_strong(&mut self) {
        // Opening marker: ** or __
        self.width += 2;
        self.format_stack.push(FormatContext::Strong);
    }

    fn end_strong(&mut self) {
        // Closing marker: ** or __
        self.width += 2;
        self.format_stack.pop();
    }

    fn start_link(&mut self, url: &str) {
        // Opening bracket: [
        self.width += 1;
        self.format_stack.push(FormatContext::Link {
            url_width: UnicodeWidthStr::width(url),
        });
    }

    fn end_link(&mut self) {
        // Closing: ](url)
        if let Some(FormatContext::Link { url_width }) = self.format_stack.pop() {
            // "](" + url + ")"
            self.width += 2 + url_width + 1;
        }
    }

    fn total_width(&self) -> usize {
        self.width
    }
}

/// Processes a single table's events to align columns.
///
/// This function analyzes all table cells to determine the maximum width
/// for each column and then pads cells accordingly. It preserves the original
/// event structure (keeping Code events as Code, not merging into Text) and
/// adds spacing around cell content for readability: `| content |`
///
/// Width calculation accounts for:
/// - Unicode display width (not just character count)
/// - Link syntax: `[text](url)` adds brackets and URL
/// - Emphasis: `*text*` or `_text_` adds 2 characters
/// - Strong: `**text**` or `__text__` adds 4 characters
/// - Code spans: `` `code` `` adds 2 backticks
fn process_single_table(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    // Pass 1: Measure column widths (visual width of rendered content)
    let mut col_widths: Vec<usize> = Vec::new();
    let mut current_col = 0;
    let mut in_cell = false;
    let mut calc = CellWidthCalculator::new();

    for ev in &events {
        match ev {
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                calc.reset();
            }
            Event::End(TagEnd::TableCell) => {
                let cell_width = calc.total_width();
                if col_widths.len() <= current_col {
                    col_widths.push(cell_width);
                } else {
                    col_widths[current_col] = col_widths[current_col].max(cell_width);
                }
                current_col += 1;
                in_cell = false;
            }
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                current_col = 0;
            }
            Event::Text(t) if in_cell => {
                calc.add_text(t);
            }
            Event::Code(t) if in_cell => {
                calc.add_code(t);
            }
            Event::Start(Tag::Emphasis) if in_cell => {
                calc.start_emphasis();
            }
            Event::End(TagEnd::Emphasis) if in_cell => {
                calc.end_emphasis();
            }
            Event::Start(Tag::Strong) if in_cell => {
                calc.start_strong();
            }
            Event::End(TagEnd::Strong) if in_cell => {
                calc.end_strong();
            }
            Event::Start(Tag::Link { dest_url, .. }) if in_cell => {
                calc.start_link(dest_url);
            }
            Event::End(TagEnd::Link) if in_cell => {
                calc.end_link();
            }
            _ => {}
        }
    }

    // Pass 2: Preserve original events, add leading space and trailing padding
    let mut result = Vec::with_capacity(events.len() + col_widths.len() * 2);
    let mut current_col = 0;
    let mut in_cell = false;
    let mut calc = CellWidthCalculator::new();

    for ev in events {
        match &ev {
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                calc.reset();
                result.push(ev);
                // Add leading space for readability: "|content" -> "| content"
                result.push(Event::Text(CowStr::from(" ")));
            }
            Event::End(TagEnd::TableCell) => {
                // Add trailing padding to align columns, plus one space before |
                let target_width = col_widths.get(current_col).copied().unwrap_or(0);
                let cell_width = calc.total_width();
                let padding_needed = target_width.saturating_sub(cell_width);
                // Add padding + trailing space: "content|" -> "content |"
                let padding = " ".repeat(padding_needed + 1);
                result.push(Event::Text(CowStr::from(padding)));
                current_col += 1;
                in_cell = false;
                result.push(ev);
            }
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                current_col = 0;
                result.push(ev);
            }
            Event::Text(t) if in_cell => {
                calc.add_text(t);
                result.push(ev);
            }
            Event::Code(t) if in_cell => {
                calc.add_code(t);
                result.push(ev);
            }
            Event::Start(Tag::Emphasis) if in_cell => {
                calc.start_emphasis();
                result.push(ev);
            }
            Event::End(TagEnd::Emphasis) if in_cell => {
                calc.end_emphasis();
                result.push(ev);
            }
            Event::Start(Tag::Strong) if in_cell => {
                calc.start_strong();
                result.push(ev);
            }
            Event::End(TagEnd::Strong) if in_cell => {
                calc.end_strong();
                result.push(ev);
            }
            Event::Start(Tag::Link { dest_url, .. }) if in_cell => {
                calc.start_link(dest_url);
                result.push(ev);
            }
            Event::End(TagEnd::Link) if in_cell => {
                calc.end_link();
                result.push(ev);
            }
            _ => {
                result.push(ev);
            }
        }
    }

    result
}
