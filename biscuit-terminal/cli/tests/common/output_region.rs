// Locating a command's output in a captured pane without reading its echo.
//
// A shell may display the typed command line wrapped, or shortened with a
// leading `<` (readline's horizontal scrolling, seen in tmux), so the echo
// cannot reliably mark where output starts. The command instead prints a
// begin marker before itself and an end marker after itself, and the output
// is exactly the rows between them.
//
// The marker text never appears in the echo: `printf` assembles it from a
// format string and the id, which the echo shows apart (`'BTOUT-%s-BEGIN\n'
// 4242x7`). Pure text in and out, so the L1 binary tests it without a
// terminal.

/// One displayed row of a command's output: the escape-free text the
/// terminal shows (trailing padding trimmed) and the raw capture row with
/// the terminal's re-serialized SGR and OSC 8 state.
///
/// `raw` starts with the state carried into the row: a terminal re-emits only
/// changed attributes, so a dim set on one row and still on for the next
/// arrives once (WezTerm on Windows, tmux). The prefix is the SGR parameters
/// since the last full reset and the open OSC 8 link, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRow {
    pub plain: String,
    pub raw: String,
}

fn marker(id: &str, edge: &str) -> String {
    format!("BTOUT-{id}-{edge}")
}

/// The shell line that runs `line` between the begin and end markers for
/// `id`. `id` must need no shell quoting (digits, letters, `x`).
///
/// The markers print even when `line` fails, and the syntax is POSIX, so
/// every shell the harness launches (bash, zsh, sh) runs it the same way.
pub fn marked_command(line: &str, id: &str) -> String {
    format!("printf 'BTOUT-%s-BEGIN\\n' {id}; {line}; printf 'BTOUT-%s-END\\n' {id}")
}

/// The rows displayed between the begin and end markers for `id`, blank rows
/// included so paragraph spacing stays observable. `None` until the frame
/// shows both markers.
///
/// `raw` and `plain` are the same capture's rows, paired by index. Output
/// that ends without a newline shares its last row with the end marker; that
/// row's text before the marker is the last output row.
pub fn rows_between_markers(raw: &str, plain: &str, id: &str) -> Option<Vec<OutputRow>> {
    let begin = marker(id, "BEGIN");
    let end = marker(id, "END");
    let raw_lines: Vec<&str> = raw.lines().collect();
    let plain_lines: Vec<&str> = plain.lines().collect();
    let start = plain_lines.iter().rposition(|line| line.trim() == begin)?;
    let mut carried = CarriedState::default();
    for line in &raw_lines[..(start + 1).min(raw_lines.len())] {
        carried.advance(line);
    }
    let mut rows = Vec::new();
    for (i, plain) in plain_lines.iter().enumerate().skip(start + 1) {
        let plain = plain.trim_end();
        let raw = raw_lines.get(i).copied().unwrap_or("");
        if plain.trim_start() == end {
            return Some(rows);
        }
        let prefix = carried.prefix();
        carried.advance(raw);
        if let Some(text) = plain.strip_suffix(end.as_str()) {
            let raw = raw.rfind(end.as_str()).map_or(raw, |at| &raw[..at]);
            rows.push(OutputRow { plain: text.trim_end().to_string(), raw: format!("{prefix}{raw}") });
            return Some(rows);
        }
        rows.push(OutputRow { plain: plain.to_string(), raw: format!("{prefix}{raw}") });
    }
    None
}

/// The SGR and OSC 8 state in force after the raw rows seen so far.
#[derive(Default)]
struct CarriedState {
    /// SGR parameters since the last full reset, in order.
    sgr: Vec<String>,
    /// The URI of the OSC 8 link left open, if any.
    link: Option<String>,
}

impl CarriedState {
    /// Escapes that re-establish the state; empty when it is the default.
    fn prefix(&self) -> String {
        let mut out = String::new();
        if !self.sgr.is_empty() {
            out.push_str(&format!("\x1b[{}m", self.sgr.join(";")));
        }
        if let Some(uri) = &self.link {
            out.push_str(&format!("\x1b]8;;{uri}\x1b\\"));
        }
        out
    }

    fn advance(&mut self, raw: &str) {
        let mut rest = raw;
        while let Some(at) = rest.find('\x1b') {
            rest = &rest[at + 1..];
            if let Some(csi) = rest.strip_prefix('[') {
                let Some(end) = csi.find(|c: char| ('\u{40}'..='\u{7e}').contains(&c)) else {
                    return;
                };
                if csi[end..].starts_with('m') {
                    self.apply_sgr(&csi[..end]);
                }
                rest = &csi[end + 1..];
            } else if let Some(osc) = rest.strip_prefix(']') {
                let (body, after) = match (osc.find('\u{07}'), osc.find("\x1b\\")) {
                    (Some(bel), Some(st)) if st < bel => (&osc[..st], &osc[st + 2..]),
                    (Some(bel), _) => (&osc[..bel], &osc[bel + 1..]),
                    (None, Some(st)) => (&osc[..st], &osc[st + 2..]),
                    (None, None) => return,
                };
                if let Some(link) = body.strip_prefix("8;") {
                    let uri = link.split_once(';').map_or("", |(_, uri)| uri);
                    self.link = (!uri.is_empty()).then(|| uri.to_string());
                }
                rest = after;
            }
        }
    }

    /// A reset (`0` or empty) drops everything before it; other parameters
    /// are replayed in order, so `2` then `22` still ends with dim off.
    fn apply_sgr(&mut self, params: &str) {
        let mut params = params.split(';');
        while let Some(param) = params.next() {
            if param.is_empty() || param == "0" {
                self.sgr.clear();
                continue;
            }
            self.sgr.push(param.to_string());
            // A semicolon-form color carries its arguments as further
            // parameters, whose `0` components are not resets.
            if matches!(param, "38" | "48" | "58")
                && let Some(kind) = params.next()
            {
                self.sgr.push(kind.to_string());
                let count = if kind == "5" { 1 } else { 3 };
                self.sgr.extend(params.by_ref().take(count).map(str::to_string));
            }
        }
    }
}
