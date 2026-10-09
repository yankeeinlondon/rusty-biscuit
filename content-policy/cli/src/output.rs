//! Terminal rendering of reports and renewal plans.
//!
//! `--json` output is the library's own serialization; this module renders
//! the default (styled) and `--plain` forms from the same values.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::discovery::eval::strip_ansi_codes;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::wrap_policy::WordWrap;
use content_policy::{
    BaselineTarget, ChangeKind, DateEvidence, DateSource, EntryOutcome, EntryResult,
    PolicySource, RenewalPlan, Report, Status, UnknownReason,
};

/// Styled or plain text output.
pub struct TextOutput {
    terminal: Terminal,
    plain: bool,
}

impl TextOutput {
    pub fn new(plain: bool) -> Self {
        let terminal = if plain {
            Terminal {
                color_depth: ColorDepth::None,
                ..Terminal::new()
            }
        } else if std::env::var_os("FORCE_COLOR").is_some()
            || std::env::var_os("CLICOLOR_FORCE").is_some()
        {
            Terminal::new_forced()
        } else {
            Terminal::new()
        };
        Self { terminal, plain }
    }

    fn render(&self, component: &impl TerminalRenderable) -> String {
        let rendered = component.render(&self.terminal);
        // A colorless terminal still gets bold and italic; `--plain` promises
        // no styling at all.
        if self.plain { strip_ansi_codes(&rendered) } else { rendered }
    }

    /// Wrapping for the rule and evidence columns. A `FileChanged` rule and
    /// a fingerprint are long words with no spaces: prose wrapping either
    /// cannot fit them in 80 columns or splits them mid-word, so a table that
    /// holds one also breaks at `/` and `_`.
    fn wrap(long_words: bool) -> WordWrap {
        if long_words {
            WordWrap::BespokeProse(None, vec![' ', '-', '/', '_'], None)
        } else {
            WordWrap::WrapProse(None, None)
        }
    }

    pub fn report(&self, report: &Report) -> String {
        let mut out = String::new();
        out.push_str(&self.render(&Prose::new(summary(report))));
        out.push('\n');
        let long_words = report.results.iter().any(|entry| entry.file.is_some());
        out.push_str(&self.render(&entries_table(&report.results, Self::wrap(long_words))));
        for warning in &report.warnings {
            out.push('\n');
            out.push_str(&self.render(&Prose::new(format!(
                "<yellow><bold>warning:</bold></yellow> {}",
                Prose::escape_text(&warning.message)
            ))));
        }
        ensure_trailing_newline(out)
    }

    pub fn plan(&self, plan: &RenewalPlan, written: bool) -> String {
        let document = document_label(plan.document.as_deref());
        if plan.nothing_to_renew() {
            return ensure_trailing_newline(self.render(&Prose::new(format!(
                "<bold>{document}</bold>: nothing to renew; the policy has no rule with a \
                 baseline to advance"
            ))));
        }
        let state = if written {
            "<green>written</green>".to_string()
        } else {
            "<dim>preview; pass --write to apply</dim>".to_string()
        };
        let mut out = self.render(&Prose::new(format!(
            "<bold>{document}</bold>: renewal on <bold>{}</bold> ({state})",
            plan.update_date.0
        )));
        out.push('\n');
        let long_words = plan.changes.iter().any(|change| change.value.contains(':'));
        out.push_str(&self.render(&changes_table(plan, Self::wrap(long_words))));
        if !plan.tab_repair.is_empty() {
            out.push('\n');
            out.push_str(&self.render(&Prose::new(format!(
                "<yellow><bold>tab repair:</bold></yellow> {} frontmatter line(s) indented with \
                 tabs, which YAML forbids, are re-indented with two spaces per tab",
                plan.tab_repair.len()
            ))));
        }
        ensure_trailing_newline(out)
    }
}

/// The `--needs-action` answer: a confirmed trigger is `true` even when
/// another entry is unknown.
pub fn needs_action(report: &Report) -> &'static str {
    match report.status {
        Status::Stale | Status::Expired => "true",
        Status::Fresh => "false",
        Status::Unknown => "unknown",
    }
}

fn summary(report: &Report) -> String {
    let document = document_label(report.document.as_deref());
    let status = match report.status {
        Status::Fresh => "<green>fresh</green>",
        Status::Stale => "<yellow>stale</yellow>",
        Status::Expired => "<red>expired</red>",
        Status::Unknown => "<magenta>unknown</magenta>",
    };
    let action = match report.action {
        Some(action) => format!("action <bold>{action}</bold>"),
        None => "no action".to_string(),
    };
    let resolution = if report.action_resolution_complete {
        ""
    } else {
        "; an unknown entry could raise the action"
    };
    let source = match report.policy.source {
        PolicySource::Declared => "declared policy",
        PolicySource::Defaulted => "default policy",
    };
    format!(
        "<bold>{document}</bold>: {status}, {action}{resolution} <dim>({source}, evaluated {})</dim>",
        report.evaluated_at.format("%Y-%m-%d %H:%M UTC")
    )
}

fn entries_table(results: &[EntryResult], wrap: WordWrap) -> Table {
    let rows = results
        .iter()
        .map(|entry| {
            let evidence = match &entry.file {
                Some(file) => file.stored.as_deref().map_or_else(|| "missing".to_string(), abbreviate),
                None => entry
                    .baseline
                    .as_ref()
                    .or(entry.deadline.as_ref())
                    .map(date_label)
                    .unwrap_or_default(),
            };
            vec![
                TableCellContent::from((entry.index + 1).to_string()),
                entry.rule.clone().into(),
                entry.action.as_str().into(),
                outcome_label(entry.outcome).into(),
                evidence.into(),
                entry
                    .due
                    .map(|due| due.0.to_string())
                    .unwrap_or_default()
                    .into(),
            ]
        })
        .collect();
    Table::new()
        .with_columns(vec![
            TableColumn::new("#"),
            TableColumn::new("Rule").with_word_wrap(wrap),
            TableColumn::new("Action"),
            TableColumn::new("Result").with_word_wrap(WordWrap::WrapProse(None, None)),
            TableColumn::new("Evidence").with_word_wrap(WordWrap::WrapProse(None, None)),
            TableColumn::new("Due").with_word_wrap(WordWrap::None),
        ])
        .with_data(rows)
}

fn changes_table(plan: &RenewalPlan, wrap: WordWrap) -> Table {
    // Dates never split at a hyphen while there is room; fingerprints may.
    let value_wrap = match wrap {
        WordWrap::WrapProse(..) => WordWrap::None,
        _ => wrap.clone(),
    };
    let rows = plan
        .changes
        .iter()
        .map(|change| {
            let target = match &change.target {
                BaselineTarget::Inline { entry } => format!("entry {} inline date", entry + 1),
                BaselineTarget::Property { name } => name.clone(),
            };
            let entries = change
                .entries
                .iter()
                .map(|entry| (entry + 1).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let kind = match change.kind {
                ChangeKind::Renewed => "renewed",
                ChangeKind::NewBaseline => "new baseline",
                ChangeKind::Unchanged => "unchanged",
            };
            vec![
                TableCellContent::from(target),
                entries.into(),
                kind.into(),
                change
                    .previous
                    .as_deref()
                    .map_or_else(|| "(none)".to_string(), abbreviate)
                    .into(),
                abbreviate(&change.value).into(),
            ]
        })
        .collect();
    Table::new()
        .with_columns(vec![
            TableColumn::new("Baseline").with_word_wrap(wrap.clone()),
            TableColumn::new("Entries"),
            TableColumn::new("Change").with_word_wrap(WordWrap::None),
            TableColumn::new("From").with_word_wrap(value_wrap.clone()),
            TableColumn::new("To").with_word_wrap(value_wrap),
        ])
        .with_data(rows)
}

fn outcome_label(outcome: EntryOutcome) -> &'static str {
    match outcome {
        EntryOutcome::Triggered => "triggered",
        EntryOutcome::NotTriggered => "not triggered",
        EntryOutcome::Unknown(UnknownReason::MissingBaseline) => "unknown (missing baseline)",
        EntryOutcome::Unknown(UnknownReason::InconsistentBaseline) => {
            "unknown (future baseline)"
        }
        EntryOutcome::Unknown(UnknownReason::MissingProvider) => "unknown (no file provider)",
        EntryOutcome::Unknown(UnknownReason::NotAFile) => "unknown (not a file)",
        EntryOutcome::Unknown(UnknownReason::UnreadableFile) => "unknown (unreadable file)",
        EntryOutcome::Unknown(UnknownReason::IncompatibleFingerprint) => {
            "unknown (unrecognized fingerprint scheme)"
        }
    }
}

/// A date as is; a content fingerprint cut to its scheme and 8 hex digits,
/// which is enough to tell two apart in a table. `--json` keeps the full
/// value.
fn abbreviate(value: &str) -> String {
    match value.split_once(':') {
        Some((scheme, hex)) if hex.len() > 8 => format!("{scheme}:{}…", &hex[..8]),
        _ => value.to_string(),
    }
}

fn date_label(evidence: &DateEvidence) -> String {
    let value = evidence
        .value
        .map(|date| date.0.to_string())
        .unwrap_or_else(|| "missing".to_string());
    match (&evidence.property, evidence.source) {
        (Some(name), DateSource::Property | DateSource::DefaultProperty) => {
            format!("{value} (@{name})")
        }
        _ => format!("{value} (inline)"),
    }
}

fn document_label(document: Option<&str>) -> String {
    Prose::escape_text(document.unwrap_or("document"))
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}
