//! Human-facing generator reports, rendered through `biscuit-terminal`
//! components (`Prose` for lines, `UnorderedList` for detail groups).
//!
//! Every human-facing output path routes through this module so styling
//! degrades consistently: a real TTY gets colored status keywords, while a
//! pipe / `NO_COLOR` gets bare text and `FORCE_COLOR` / `CLICOLOR_FORCE`
//! forces color on. Machine-facing JSON (the `mapping` document, the
//! `agent-errors` findings file) is emitted raw and never passes through
//! here.
//!
//! The rendered plain-text content is intentionally identical to the
//! pre-Phase-10 `println!` output: color lives only in markup that strips
//! away when the terminal reports no color, and dynamic text is escaped so
//! `<placeholder>` tokens, identifiers, and paths render literally.

use std::io::IsTerminal;
use std::path::Path;

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::prelude::{Prose, TerminalRenderable, UnorderedList};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Length, TargetValue};

use crate::generate::{CheckOutcome, Generation, Provenance};

/// Builds the terminal used for all human-facing output.
///
/// `FORCE_COLOR=1` / `CLICOLOR_FORCE=1` forces color on (matching `bt`'s own
/// escape hatch). Otherwise color is gated on stdout being a TTY — a pipe,
/// redirect, or `NO_COLOR` degrades to plain text — because `COLORTERM`
/// detection alone would otherwise emit SGR into a captured stream.
pub fn output_terminal() -> Terminal {
    let forced = std::env::var_os("FORCE_COLOR").is_some_and(|v| v != "0")
        || std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| v == "1");
    if forced {
        return Terminal::new_forced();
    }
    let mut term = Terminal::new();
    let no_color = std::env::var_os("NO_COLOR").is_some();
    if no_color || !std::io::stdout().is_terminal() {
        term.color_depth = ColorDepth::None;
        term.is_tty = false;
        term.osc_link_support = false;
    }
    term
}

/// Escapes Prose markup sigils in dynamic text so identifiers, paths, and
/// `<placeholder>` tokens render literally in both color and plain modes.
fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' | '{') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Renders one Prose line (markup interpreted, trailing newline appended).
fn line(term: &Terminal, markup: impl AsRef<str>) -> String {
    let mut out = Prose::new(markup.as_ref()).render(term);
    out.push('\n');
    out
}

/// Renders a flat detail group as an indented `UnorderedList`; empty groups
/// render nothing.
///
/// `UnorderedList` items are rendered as literal text (no Prose markup
/// pass), so callers pass raw strings — `<placeholder>` tokens, identifiers,
/// and `&["..."]` fragments all survive verbatim without escaping.
fn detail_list(term: &Terminal, items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let list = UnorderedList::new(items.to_vec()).left_margin(TargetValue::universal(Length::ch(2)));
    let mut out = list.render(term);
    out.push('\n');
    out
}

/// One provider's `check` outcome line plus any drift detail.
pub fn provider_check(term: &Terminal, slug: &str, outcome: &CheckOutcome) -> String {
    match outcome {
        CheckOutcome::Clean => line(
            term,
            format!("{slug}: <green>clean</green> (inputs match the committed data.rs)"),
        ),
        CheckOutcome::Drift { details } => {
            let mut out = line(
                term,
                format!("{slug}: <red>DRIFT</red> — regenerate with `claudine-gen generate`:"),
            );
            out.push_str(&capped_diff(term, details));
            out
        }
        CheckOutcome::MissingCommitted { path } => line(
            term,
            format!(
                "{slug}: <yellow>committed data.rs missing</yellow> at {} — run `claudine-gen generate`",
                esc(&biscuit_file::to_portable_string(path))
            ),
        ),
    }
}

/// A full-scope artifact's `check` outcome (catalog / signals / vocabulary /
/// families), whose clean detail differs per artifact.
pub fn artifact_check(
    term: &Terminal,
    label: &str,
    clean_detail: &str,
    outcome: &CheckOutcome,
) -> String {
    match outcome {
        CheckOutcome::Clean => line(term, format!("{label}: <green>clean</green> {clean_detail}")),
        CheckOutcome::Drift { details } => {
            let mut out = line(
                term,
                format!("{label}: <red>DRIFT</red> — regenerate with `claudine-gen generate`:"),
            );
            out.push_str(&capped_diff(term, details));
            out
        }
        CheckOutcome::MissingCommitted { path } => line(
            term,
            format!(
                "{label} <yellow>missing</yellow> at {} — run `claudine-gen generate`",
                esc(&biscuit_file::to_portable_string(path))
            ),
        ),
    }
}

/// Roster ↔ wired-set cross-check line.
pub fn roster(term: &Terminal, unwired_active: &[String]) -> String {
    if unwired_active.is_empty() {
        line(term, "roster: every active entry has a wired Provider variant")
    } else {
        line(
            term,
            format!(
                "roster: {} researched but not wired (no Provider variant) — informational",
                esc(&unwired_active.join(", "))
            ),
        )
    }
}

/// The models-catalog staleness warning (artifact-global, printed once).
pub fn artifact_warning(term: &Terminal, generations: &[Generation]) -> String {
    match generations
        .iter()
        .find_map(|generation| generation.artifact_warning.as_deref())
    {
        Some(warning) => line(term, format!("<yellow>WARNING:</yellow> {}", esc(warning))),
        None => String::new(),
    }
}

/// Per-provider provenance: offering-join coverage, coercion skips, and
/// active/stale overrides.
pub fn provenance(term: &Terminal, generation: &Generation) -> String {
    let join = &generation.offering_join;
    let mut out = line(
        term,
        format!(
            "  expected offerings: {}/{} joined to the models-catalog artifact",
            join.joined, join.total
        ),
    );
    let mut unjoined: Vec<String> = join
        .unjoined
        .iter()
        .map(|id| format!("unjoined: {id}"))
        .collect();
    for alias in &join.ambiguous_aliases {
        unjoined.push(format!(
            "ambiguous alias: `{alias}` spans multiple families — resolves mark dropped"
        ));
    }
    out.push_str(&detail_list(term, &unjoined));

    for skip in &generation.skips {
        out.push_str(&line(
            term,
            format!(
                "  coercion skipped {} record{} for {} ({}):",
                skip.records.len(),
                if skip.records.len() == 1 { "" } else { "s" },
                esc(skip.field),
                esc(skip.reason)
            ),
        ));
        let records: Vec<String> = skip
            .records
            .iter()
            .map(|record| format!("{record:?}"))
            .collect();
        out.push_str(&detail_list(term, &records));
    }

    for field in &generation.fields {
        if let Provenance::Override {
            reason,
            suppressed,
            stale,
        } = &field.provenance
        {
            let suppressed = suppressed
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "<source unresolvable>".to_string());
            out.push_str(&line(
                term,
                format!(
                    "  override active: {} (suppressing {}) — {}",
                    esc(field.field),
                    esc(&suppressed),
                    esc(reason)
                ),
            ));
            if *stale {
                out.push_str(&line(
                    term,
                    format!(
                        "  <yellow>STALE override</yellow>: {} now equals the source value — delete it",
                        esc(field.field)
                    ),
                ));
            }
        }
    }
    out
}

/// Compact diff: the first lines verbatim (as an indented list), then an
/// elision count.
pub fn capped_diff(term: &Terminal, details: &[String]) -> String {
    const MAX_LINES: usize = 20;
    let mut items: Vec<String> = details.iter().take(MAX_LINES).cloned().collect();
    if details.len() > MAX_LINES {
        items.push(format!(
            "… {} more differing lines",
            details.len() - MAX_LINES
        ));
    }
    detail_list(term, &items)
}

/// The `families` compile summary printed before the generate/apply flow.
pub fn families_count(term: &Terminal, count: usize) -> String {
    line(
        term,
        format!("families: {count} family keys compiled from expected-offering joins"),
    )
}

/// The non-TTY report-only notice.
pub fn report_only_notice(term: &Terminal) -> String {
    line(
        term,
        "stdin is not a TTY — report-only (pass --yes to write unconditionally)",
    )
}

/// The per-file drift header shown before each confirmation decision.
pub fn decide_header(term: &Terminal, path: &Path, diff: &[String]) -> String {
    let mut out = line(
        term,
        format!(
            "{}: {} differing line{}",
            esc(&biscuit_file::to_portable_string(path)),
            diff.len(),
            if diff.len() == 1 { "" } else { "s" }
        ),
    );
    out.push_str(&capped_diff(term, diff));
    out
}

/// The `--yes` unconditional-write acknowledgement under a decide header.
pub fn writing_yes(term: &Terminal) -> String {
    line(term, "  <green>writing</green> (--yes)")
}

/// A `wrote <path>` acknowledgement line.
pub fn wrote(term: &Terminal, path: &Path) -> String {
    line(
        term,
        format!(
            "<green>wrote</green> {}",
            esc(&biscuit_file::to_portable_string(path))
        ),
    )
}

/// The trailing report-only / unreconciled-drift summary plus declined
/// files with their override snippets.
pub fn declined_summary(
    term: &Terminal,
    report_only: bool,
    declined: &[crate::apply::DeclinedDrift],
) -> String {
    let mut out = String::from("\n");
    if report_only {
        out.push_str(&line(
            term,
            format!(
                "{} file(s) drifted — rerun on a TTY to confirm per file, or pass --yes",
                declined.len()
            ),
        ));
    } else {
        out.push_str(&line(
            term,
            format!(
                "{} declined file(s) remain unreconciled — pin the committed value with an \
                 override, or revert the offending input, then regenerate:",
                declined.len()
            ),
        ));
    }
    for entry in declined {
        out.push_str(&line(
            term,
            format!(
                "  <red>declined:</red> {}",
                esc(&biscuit_file::to_portable_string(&entry.path))
            ),
        ));
        if let Some(snippet) = &entry.override_snippet {
            let items: Vec<String> = snippet.lines().map(str::to_string).collect();
            out.push_str(&detail_list(term, &items));
        }
    }
    out
}

/// A scaffold-stub write/skip acknowledgement.
pub fn scaffold_stub(term: &Terminal, written: bool, rel: &str) -> String {
    if written {
        line(term, format!("<green>scaffolded</green> {}", esc(rel)))
    } else {
        line(term, format!("{} already present — left untouched", esc(rel)))
    }
}

/// The facts-skeleton scaffolding notice.
pub fn scaffold_facts_notice(term: &Terminal, slug: &str) -> String {
    line(
        term,
        format!(
            "<green>scaffolded</green> docs/providers/facts/{slug}.yaml — fill the TODO(required) \
             fields, then rerun `claudine-gen generate {slug} --scaffold`"
        ),
    )
}

/// The interactive `[y/N/q]` prompt text (no trailing newline — the answer
/// follows on the same line).
pub fn prompt(term: &Terminal, name: &str) -> String {
    Prose::new(format!("write {}? [y/N/q] ", esc(name))).render(term)
}

/// The unrecognized-answer reprompt hint.
pub fn prompt_unrecognized(term: &Terminal, other: &str) -> String {
    line(
        term,
        format!("unrecognized `{}` — expected y, n, or q", esc(other)),
    )
}

/// The `agent-errors check` clean outcome.
pub fn agent_errors_clean(term: &Terminal, slug: &str) -> String {
    line(
        term,
        format!("{slug}: <green>agent-errors research clean</green> (explicit outcome)"),
    )
}

/// The `agent-errors check` findings outcome plus each finding detail.
pub fn agent_errors_findings(
    term: &Terminal,
    slug: &str,
    findings: &[crate::agent_errors_check::Finding],
    findings_path: &Path,
) -> String {
    let mut out = line(
        term,
        format!(
            "{slug}: {} deterministic finding(s) written to {}",
            findings.len(),
            esc(&biscuit_file::to_portable_string(findings_path))
        ),
    );
    let items: Vec<String> = findings.iter().map(|f| f.detail.clone()).collect();
    out.push_str(&detail_list(term, &items));
    out
}

/// The `agent-errors check` gate-error outcome.
pub fn agent_errors_gate_error(
    term: &Terminal,
    slug: &str,
    findings_path: &Path,
    error: Option<&str>,
) -> String {
    line(
        term,
        format!(
            "{slug}: deterministic gate error written to {}: {}",
            esc(&biscuit_file::to_portable_string(findings_path)),
            esc(error.unwrap_or("unknown gate error"))
        ),
    )
}

/// The top-level fatal-error line (stderr).
pub fn fatal(term: &Terminal, err: &crate::errors::GenError) -> String {
    line(
        term,
        format!("<red>claudine-gen error:</red> {}", esc(&err.to_string())),
    )
}

#[cfg(test)]
mod tests;
