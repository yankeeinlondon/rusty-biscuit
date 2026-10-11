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
    let mut rows = Vec::new();
    for (i, plain) in plain_lines.iter().enumerate().skip(start + 1) {
        let plain = plain.trim_end();
        let raw = raw_lines.get(i).copied().unwrap_or("");
        if plain.trim_start() == end {
            return Some(rows);
        }
        if let Some(text) = plain.strip_suffix(end.as_str()) {
            let raw = raw.rfind(end.as_str()).map_or(raw, |at| &raw[..at]);
            rows.push(OutputRow { plain: text.trim_end().to_string(), raw: raw.to_string() });
            return Some(rows);
        }
        rows.push(OutputRow { plain: plain.to_string(), raw: raw.to_string() });
    }
    None
}
