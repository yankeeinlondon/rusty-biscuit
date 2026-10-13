//! One command's output, cut out of a shared terminal pane's history.
//!
//! The Level 2 tests run several reports on one pane, and the pane keeps the
//! earlier ones in its scrollback (a `clear` is not guaranteed to remove them,
//! and a shared pane also holds other tests' output). An assertion over the
//! whole capture can therefore be satisfied, or failed, by an earlier report.
//! So every command is wrapped in a unique begin and end marker printed by the
//! pane's shell, and every assertion reads only the rows strictly between the
//! two ([`Report`]). A capture whose markers are missing, repeated or out of
//! order is an error, never an empty or whole-pane report.
//!
//! The helpers need no terminal: the `symbols` L1 binary tests them on
//! synthetic captures, and the `level2_*` binaries drive them on real panes.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use biscuit_test_harness::{TerminalHarness, strip_ansi};

/// The begin and end lines printed around one command.
///
/// The shell prints them with `printf '%s-%s\n' HUG-BEGIN <id>`, so the typed
/// command line (`HUG-BEGIN <id>`) never reads as the printed marker
/// (`HUG-BEGIN-<id>`). The id carries the process id and a per-process
/// counter, so no other report on a shared pane prints the same marker.
pub struct ReportMarkers {
    id: String,
}

impl ReportMarkers {
    /// Markers no other report in this or another test process prints.
    pub fn unique() -> Self {
        static RUN: AtomicUsize = AtomicUsize::new(0);
        let run = RUN.fetch_add(1, Ordering::Relaxed);
        Self::with_id(&format!("{}-{run}", std::process::id()))
    }

    /// Markers with a fixed id, for synthetic captures.
    pub fn with_id(id: &str) -> Self {
        Self { id: id.to_string() }
    }

    pub fn begin(&self) -> String {
        format!("HUG-BEGIN-{}", self.id)
    }

    pub fn end(&self) -> String {
        format!("HUG-END-{}", self.id)
    }

    /// `command` wrapped in the shell commands that print the markers.
    pub fn wrap(&self, command: &str) -> String {
        format!(
            "printf '%s-%s\\n' HUG-BEGIN {id}; {command}; printf '%s-%s\\n' HUG-END {id}\n",
            id = self.id,
        )
    }
}

/// Why a capture does not (yet) hold exactly one report.
#[derive(Debug)]
pub enum ScopeError {
    /// The end marker has not been printed; the command may still be running.
    Unfinished,
    /// The markers cannot delimit one report: missing begin (scrolled out of
    /// the captured history), repeated, or in the wrong order.
    Malformed(String),
}

/// One rendered row: its escapes kept (`raw`) and stripped (`plain`).
pub struct Row {
    pub raw: String,
    pub plain: String,
    /// The SGR state in effect where the row starts. Captures re-emit only
    /// attribute changes and carry them across rows (tmux), so a row's own
    /// escapes do not describe its cells.
    entry: Style,
}

/// The rows a single command printed, between its markers.
pub struct Report {
    rows: Vec<Row>,
}

/// `raw` split into rows, each with the style in effect where it starts.
fn rows(raw: &str) -> Vec<Row> {
    let mut style = Style::default();
    raw.lines()
        .map(|line| {
            let entry = style.clone();
            style = cells(line, style.clone()).1;
            Row {
                raw: line.to_string(),
                plain: strip_ansi(line),
                entry,
            }
        })
        .collect()
}

impl Report {
    /// The rows strictly between `markers` in a whole-history capture.
    ///
    /// ## Errors
    ///
    /// [`ScopeError::Unfinished`] while no end marker is present;
    /// [`ScopeError::Malformed`] when either marker is not present exactly
    /// once or the begin marker is not before the end marker.
    pub fn scope(raw: &str, markers: &ReportMarkers) -> Result<Self, ScopeError> {
        let mut rows = rows(raw);
        let (begin, end) = (markers.begin(), markers.end());
        let at = |marker: &str| -> Vec<usize> {
            rows.iter()
                .enumerate()
                .filter(|(_, row)| row.plain.trim() == marker)
                .map(|(index, _)| index)
                .collect()
        };
        let (begins, ends) = (at(&begin), at(&end));
        match (begins.as_slice(), ends.as_slice()) {
            (_, []) => Err(ScopeError::Unfinished),
            (&[first], &[last]) if first < last => {
                rows.truncate(last);
                rows.drain(..=first);
                Ok(Self { rows })
            }
            _ => Err(ScopeError::Malformed(format!(
                "expected {begin:?} once before {end:?} once; found begin rows {begins:?}, \
                 end rows {ends:?}"
            ))),
        }
    }

    /// The report's text, one captured row per line.
    pub fn plain(&self) -> String {
        self.rows.iter().map(|row| format!("{}\n", row.plain)).collect()
    }

    /// Each captured row's text, escapes stripped, untrimmed.
    pub fn lines(&self) -> Vec<&str> {
        self.rows.iter().map(|row| row.plain.as_str()).collect()
    }

    /// The report's escaped text, one captured row per line.
    pub fn raw(&self) -> String {
        self.rows.iter().map(|row| format!("{}\n", row.raw)).collect()
    }

    /// Whether some row, trimmed, is exactly `text`.
    pub fn has_row(&self, text: &str) -> bool {
        self.rows.iter().any(|row| row.plain.trim() == text)
    }

    /// Whether some logical row, trimmed, is exactly `text`, for a capture
    /// that splits soft-wrapped rows (WezTerm's `get-text`) of a pane
    /// `cols` wide: a row that fills the pane is joined to the next, and a
    /// row one column short also joins, with the space the capture trimmed
    /// put back. Widths are measured without trailing spaces, because the
    /// capture pads a wrapped line's last piece to the pane width, and that
    /// piece ends the line. Only a whole-line check can use this: a row that
    /// happens to be one column short is joined too. A row `hug` itself broke
    /// never matches, since its continuation starts with the list's hanging
    /// indent.
    pub fn has_logical_row(&self, text: &str, cols: usize) -> bool {
        let mut joined = String::new();
        for row in &self.rows {
            let shown = row.plain.trim_end();
            joined.push_str(shown);
            match shown.chars().count() {
                width if width >= cols => {}
                width if width + 1 == cols => joined.push(' '),
                _ => joined.push('\n'),
            }
        }
        joined.lines().any(|row| row.trim() == text)
    }

    /// Asserts that `needles` match rows in order (see [`row_matches`]).
    pub fn assert_rows_in_order(&self, needles: &[&str]) {
        let mut from = 0;
        for needle in needles {
            match self.rows[from..]
                .iter()
                .position(|row| row_matches(&row.plain, needle))
            {
                Some(offset) => from += offset + 1,
                None => panic!(
                    "expected a row matching {needle:?} after report row {from}.\nreport:\n{}",
                    self.plain()
                ),
            }
        }
    }

    /// The one row matching `needle` (see [`row_matches`]).
    ///
    /// ## Panics
    ///
    /// When no row or more than one row matches, so a style or link check
    /// never lands on an arbitrary occurrence.
    pub fn row(&self, needle: &str) -> &Row {
        let mut matching = self
            .rows
            .iter()
            .filter(|row| row_matches(&row.plain, needle));
        match (matching.next(), matching.next()) {
            (Some(row), None) => row,
            (None, _) => panic!(
                "expected a row matching {needle:?}.\nreport:\n{}",
                self.plain()
            ),
            (Some(_), Some(_)) => panic!(
                "expected one row matching {needle:?}, found several.\nreport:\n{}",
                self.plain()
            ),
        }
    }

    /// The SGR parameters in effect on the first character of `text` in the
    /// row matching `row`; `None` when the row does not show `text`.
    pub fn style_in_row(&self, row: &str, text: &str) -> Option<Vec<String>> {
        let row = self.row(row);
        style_of(&row.raw, row.entry.clone(), text)
    }

    /// Asserts that `text`, in the row matching `row`, is rendered with every
    /// SGR parameter in `expected`.
    pub fn assert_styled(&self, row: &str, text: &str, expected: &[&str], what: &str) {
        let parameters = self.style_in_row(row, text);
        assert!(
            parameters.as_ref().is_some_and(|parameters| expected
                .iter()
                .all(|want| parameters.iter().any(|p| p == want))),
            "expected {what} (SGR {expected:?}) on {text:?} in row {row:?}; found \
             {parameters:?}.\nrow:\n{:?}",
            self.row(row).raw,
        );
    }
}

/// A list-item needle (`- …`) matches the start of a trimmed row, so a long
/// item is found by its beginning; any other needle is a whole heading row.
pub fn row_matches(plain: &str, needle: &str) -> bool {
    let row = plain.trim();
    if needle.starts_with("- ") {
        row.starts_with(needle)
    } else {
        row == needle
    }
}

/// The SGR parameters in effect on the first character of the first visible
/// occurrence of `text` in `raw`, replaying every escape from the start of
/// `raw` (an occurrence inside an OSC 8 URL is not visible, so never found).
pub fn style_at(raw: &str, text: &str) -> Option<Vec<String>> {
    style_of(raw, Style::default(), text)
}

fn style_of(raw: &str, entry: Style, text: &str) -> Option<Vec<String>> {
    let (cells, _) = cells(raw, entry);
    let shown: String = cells.iter().map(|(ch, _)| *ch).collect();
    let at = shown.find(text)?;
    let index = shown[..at].chars().count();
    Some(cells[index].1.parameters())
}

/// The SGR attributes in effect on a cell, one canonical parameter per
/// attribute (`1`, `34`, `38:5:12`, …).
///
/// Backends spell the same cell differently: merged (`[0;1;34m`) or separate
/// (`[1m[34m`), legacy or colon colors, and only the attributes that changed
/// since the previous cell, even across rows.
#[derive(Clone, Default, Debug, PartialEq)]
struct Style {
    bold: bool,
    dim: bool,
    italic: bool,
    underline: Option<String>,
    blink: bool,
    inverse: bool,
    hidden: bool,
    strike: bool,
    foreground: Option<String>,
    background: Option<String>,
}

impl Style {
    fn parameters(&self) -> Vec<String> {
        let flags = [
            (self.bold, "1"),
            (self.dim, "2"),
            (self.italic, "3"),
            (self.blink, "5"),
            (self.inverse, "7"),
            (self.hidden, "8"),
            (self.strike, "9"),
        ];
        flags
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, code)| code.to_string())
            .chain(self.underline.clone())
            .chain(self.foreground.clone())
            .chain(self.background.clone())
            .collect()
    }

    /// Applies one SGR sequence's parameter list (`1;38;5;12`).
    fn apply(&mut self, list: &str) {
        let mut parameters = list.split(';');
        while let Some(parameter) = parameters.next() {
            let mut colon = parameter.split(':');
            let code = colon.next().unwrap_or_default();
            let sub: Vec<&str> = colon.collect();
            match code {
                "" | "0" => *self = Self::default(),
                "1" => self.bold = true,
                "2" => self.dim = true,
                "3" => self.italic = true,
                "4" if sub.first() == Some(&"0") => self.underline = None,
                "4" | "21" => self.underline = Some(parameter.to_string()),
                "5" | "6" => self.blink = true,
                "7" => self.inverse = true,
                "8" => self.hidden = true,
                "9" => self.strike = true,
                "22" => (self.bold, self.dim) = (false, false),
                "23" => self.italic = false,
                "24" => self.underline = None,
                "25" => self.blink = false,
                "27" => self.inverse = false,
                "28" => self.hidden = false,
                "29" => self.strike = false,
                "39" => self.foreground = None,
                "49" => self.background = None,
                "38" | "48" => {
                    let color = if sub.is_empty() {
                        // Legacy form: the color's parts are the next parameters.
                        let take = match parameters.next() {
                            Some("5") => vec!["5", parameters.next().unwrap_or_default()],
                            Some("2") => {
                                let mut parts = vec!["2"];
                                parts.extend(parameters.by_ref().take(3));
                                parts
                            }
                            other => other.into_iter().collect(),
                        };
                        format!("{code}:{}", take.join(":"))
                    } else {
                        parameter.to_string()
                    };
                    if code == "38" {
                        self.foreground = Some(color);
                    } else {
                        self.background = Some(color);
                    }
                }
                _ => match code.parse::<u16>() {
                    Ok(30..=37 | 90..=97) => self.foreground = Some(code.to_string()),
                    Ok(40..=47 | 100..=107) => self.background = Some(code.to_string()),
                    _ => {}
                },
            }
        }
    }
}

/// Each visible character of `raw` with the style it is drawn in, starting
/// from `entry`, and the style in effect after `raw`.
fn cells(raw: &str, entry: Style) -> (Vec<(char, Style)>, Style) {
    let mut style = entry;
    let mut cells = Vec::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\x1b' {
            cells.push((ch, style.clone()));
            continue;
        }
        match chars.next() {
            Some('[') => {
                let mut body = String::new();
                for next in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&next) {
                        if next == 'm' {
                            style.apply(&body);
                        }
                        break;
                    }
                    body.push(next);
                }
            }
            // OSC (hyperlinks, titles): ends at ST (`ESC \`) or BEL.
            Some(']') => {
                while let Some(next) = chars.next() {
                    if next == '\x07' || (next == '\x1b' && chars.next_if_eq(&'\\').is_some()) {
                        break;
                    }
                }
            }
            // Charset designation (`ESC ( B`).
            Some('(' | ')') => {
                chars.next();
            }
            _ => {}
        }
    }
    (cells, style)
}

/// Types `command` into the pane between fresh markers and returns its
/// report once the end marker is printed.
///
/// `capture` must return the pane's whole history (scrollback included), so
/// the begin marker of a report taller than the pane is still present, and
/// in it the markers must be whole rows: they are short, so they never wrap.
///
/// ## Panics
///
/// When the markers are malformed in a capture, or the end marker does not
/// appear within `timeout`.
pub fn run_scoped<H: TerminalHarness>(
    harness: &mut H,
    command: &str,
    timeout: Duration,
    mut capture: impl FnMut(&mut H) -> String,
) -> Report {
    let markers = ReportMarkers::unique();
    harness
        .send_text(markers.wrap(command).as_bytes())
        .expect("send_text failed");
    let deadline = Instant::now() + timeout;
    loop {
        let raw = capture(harness);
        match Report::scope(&raw, &markers) {
            Ok(report) => return report,
            Err(ScopeError::Malformed(why)) => {
                panic!("cannot isolate `{command}`: {why}.\ncapture:\n{}", strip_ansi(&raw))
            }
            Err(ScopeError::Unfinished) => assert!(
                Instant::now() < deadline,
                "`{command}` did not finish within {timeout:?}.\ncapture:\n{}",
                strip_ansi(&raw),
            ),
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Captured terminals may insert SGR between an OSC 8 destination and its
/// label. Styling is checked separately; the link must still name this text.
pub fn has_hyperlink(raw: &str, destination_suffix: &str, text_prefix: &str) -> bool {
    raw.split("\x1b]8;;").skip(1).any(|link| {
        let Some((destination, mut text)) = link.split_once("\x1b\\") else { return false };
        if !destination.ends_with(destination_suffix) { return false; }
        while let Some(sgr) = text.strip_prefix("\x1b[") {
            let Some(end) = sgr.find('m') else { return false };
            if !sgr[..end].bytes().all(|byte| byte.is_ascii_digit() || matches!(byte, b';' | b':')) {
                return false;
            }
            text = &sgr[end + 1..];
        }
        text.starts_with(text_prefix)
    })
}
