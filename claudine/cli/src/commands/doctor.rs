//! `claudine doctor [--fix] [--json]`: renders the rows of
//! [`claudine::doctor::run`] on stdout. Every finding, severity, and repair
//! is decided by the library; this adapter only chooses the format and the
//! exit status. User guide: `claudine/docs/cli/doctor.md`.
//!
//! `main.rs` dispatches it before Darkmatter's configuration is read and
//! before first-run setup, so a missing or broken configuration can neither
//! stop the diagnosis nor start a setup prompt, a conversion, or a write.

use std::io::{IsTerminal as _, Write as _};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use clap::Args;
use color_eyre::eyre::Result;

use claudine::doctor::{DoctorRow, Severity};

use crate::{log, request};

/// Arguments for `claudine doctor`.
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Attempt the available automatic repairs, then check the affected
    /// rows again.
    #[arg(long)]
    pub fix: bool,
    /// Print the report as one JSON array.
    #[arg(long)]
    pub json: bool,
}

/// Runs the checks and prints the report; returns `1` when any row is an
/// `error`, else `0`. A failed write to stdout is returned as an error.
pub fn run(args: &DoctorArgs) -> Result<i32> {
    let rows = claudine::doctor::run(request::snapshot(), args.fix);
    let text = if args.json {
        let mut json = serde_json::to_string_pretty(&rows)?;
        json.push('\n');
        json
    } else {
        // Redirected output is plain, as `--plain` is.
        if !std::io::stdout().is_terminal() {
            log::set_plain(true);
        }
        let term = if log::is_plain() {
            log::optimistic_terminal(Some(log::forced_width(80)))
        } else {
            log::terminal()
        };
        render(&rows, &term)
    };
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()?;
    Ok(exit_status(&rows))
}

/// `1` when any row is an error, else `0`.
fn exit_status(rows: &[DoctorRow]) -> i32 {
    i32::from(rows.iter().any(|row| row.severity == Severity::Error))
}

/// The human report: one bulleted line per row, its detail lines listed
/// below it, and a closing count.
fn render(rows: &[DoctorRow], term: &Terminal) -> String {
    let mut out = String::new();
    for row in rows {
        push_block(&mut out, &UnorderedList::from(vec![row_head(row)]).with_bullet("• ").render(term));
        if !row.detail.is_empty() {
            let lines: Vec<RenderableTerminalContent> = row
                .detail
                .lines()
                .map(|line| RenderableTerminalContent::from(Prose::new(Prose::escape_text_outside_code_spans(line))))
                .collect();
            push_block(&mut out, &UnorderedList::from(lines).with_bullet("    - ").render(term));
        }
    }
    out.push('\n');
    push_block(&mut out, &Prose::new(tally(rows)).render(term));
    out
}

fn push_block(out: &mut String, block: &str) {
    out.push_str(block.trim_end_matches('\n'));
    out.push('\n');
}

fn row_head(row: &DoctorRow) -> RenderableTerminalContent {
    let title = claudine::doctor::title_of(row.id).unwrap_or(row.id);
    RenderableTerminalContent::from(Prose::new(format!(
        "{} <b>{}</b> ({}): {}",
        severity_label(row.severity),
        Prose::escape_text(title),
        Prose::escape_text(row.id),
        Prose::escape_text_outside_code_spans(&row.summary),
    )))
}

/// The severity as bracketed text, so it reads the same without color.
fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Ok => "<green>[ok]</green>",
        Severity::Info => "<blue>[info]</blue>",
        Severity::Warn => "<orange>[warn]</orange>",
        Severity::Error => "<red><b>[error]</b></red>",
    }
}

fn tally(rows: &[DoctorRow]) -> String {
    let count = |severity| rows.iter().filter(|row| row.severity == severity).count();
    let (errors, warnings) = (count(Severity::Error), count(Severity::Warn));
    let plural = |n: usize, word: &str| if n == 1 { format!("{n} {word}") } else { format!("{n} {word}s") };
    if errors == 0 && warnings == 0 {
        "No problems found.".to_string()
    } else {
        format!("{}, {}.", plural(errors, "error"), plural(warnings, "warning"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &'static str, severity: Severity, detail: &str) -> DoctorRow {
        DoctorRow { id, severity, summary: "a `code` summary".into(), detail: detail.into(), fixable: false }
    }

    fn plain() -> Terminal {
        log::set_plain(true);
        log::optimistic_terminal(Some(200))
    }

    #[test]
    fn the_exit_status_is_one_only_when_an_error_remains() {
        assert_eq!(exit_status(&[row("config", Severity::Warn, ""), row("tts", Severity::Info, "")]), 0);
        assert_eq!(exit_status(&[row("config", Severity::Ok, ""), row("path", Severity::Error, "")]), 1);
    }

    #[test]
    fn every_row_and_detail_line_is_listed_in_order_without_ansi() {
        let rows = [
            row("config", Severity::Error, "first <line>\nRemedy: run `claudine setup`"),
            row("path", Severity::Ok, ""),
        ];
        let text = render(&rows, &plain());
        assert!(!text.contains('\u{1b}'), "{text}");
        let config = text.find("[error] Configuration files (config): a `code` summary").expect(&text);
        let first = text.find("first <line>").expect(&text);
        let remedy = text.find("Remedy: run `claudine setup`").expect(&text);
        let path = text.find("[ok] Executable path (path)").expect(&text);
        assert!(config < first && first < remedy && remedy < path, "{text}");
        assert!(text.trim_end().ends_with("1 error, 0 warnings."), "{text}");
    }

    #[test]
    fn a_report_without_problems_says_so() {
        let text = render(&[row("tts", Severity::Info, "")], &plain());
        assert!(text.trim_end().ends_with("No problems found."), "{text}");
    }
}
