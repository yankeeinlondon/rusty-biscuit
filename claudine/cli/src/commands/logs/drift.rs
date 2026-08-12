use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::utils::layout::Alignment;
use claudine::reporting::{DriftReport, DriftSignalSummary};

use crate::log;
use crate::table_utils::base_table;

use super::common::truncate_str;

pub(super) fn render_drift_report(report: &DriftReport) {
    let term = crate::log::terminal();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Model Catalog Drift</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );

    if report.signals.is_empty() && report.aliases.is_empty() {
        log::data(
            &Prose::new(
                "<dim>No model-catalog signals or alias resolutions recorded for this range.</dim>",
            )
            .render(&term),
        );
        return;
    }

    if !report.signals.is_empty() {
        let mut table = base_table(vec![
            TableColumn::new("Provider"),
            TableColumn::new("Signal"),
            TableColumn::new("Sessions").with_alignment(Alignment::Right),
            TableColumn::new("Occurrences").with_alignment(Alignment::Right),
            TableColumn::new("Last Seen"),
            TableColumn::new("Detail"),
        ]);
        for signal in &report.signals {
            table.add_row(vec![
                signal.provider.to_string().into(),
                signal.kind.clone().into(),
                signal.session_count.to_string().into(),
                signal.occurrences.to_string().into(),
                signal
                    .last_seen
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
                    .into(),
                drift_detail(signal).into(),
            ]);
        }
        log::data(&table.render(&term));
    }

    if !report.aliases.is_empty() {
        log::data(
            &Prose::new("<blue><bold>Alias Resolutions</bold></blue>".to_string()).render(&term),
        );
        let mut table = base_table(vec![
            TableColumn::new("Time"),
            TableColumn::new("Provider"),
            TableColumn::new("Session"),
            TableColumn::new("Alias"),
            TableColumn::new("Resolved To"),
            TableColumn::new("Artifact"),
        ]);
        for alias in &report.aliases {
            let artifact = if alias.stale {
                match alias.age_days {
                    Some(days) => format!("stale ({days}d)"),
                    None => "stale".to_string(),
                }
            } else {
                "fresh".to_string()
            };
            table.add_row(vec![
                alias.ended_at.format("%Y-%m-%d %H:%M").to_string().into(),
                alias.provider.to_string().into(),
                alias.session_id.as_deref().unwrap_or("—").to_string().into(),
                alias.alias.clone().into(),
                alias.identity_key.clone().into(),
                artifact.into(),
            ]);
        }
        log::data(&table.render(&term));
    }
}

/// Compact drift summary: counts plus a truncated id sample; empty for
/// the non-drift kinds (their payload is already covered by the count
/// columns).
fn drift_detail(signal: &DriftSignalSummary) -> String {
    if signal.unexpected.is_empty() && signal.missing.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    if !signal.unexpected.is_empty() {
        parts.push(format!(
            "+{} unexpected: {}",
            signal.unexpected.len(),
            truncate_str(&signal.unexpected.join(", "), 60)
        ));
    }
    if !signal.missing.is_empty() {
        parts.push(format!(
            "-{} missing: {}",
            signal.missing.len(),
            truncate_str(&signal.missing.join(", "), 60)
        ));
    }
    if let Some(via) = &signal.observed_via {
        parts.push(format!("via {via}"));
    }
    parts.join(" · ")
}
