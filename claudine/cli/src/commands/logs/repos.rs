use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::utils::layout::Alignment;
use claudine::reporting::ReposReport;

use crate::log;
use crate::table_utils::base_table;

use super::common::{render_labeled_counts, repo_label};

pub(super) fn render_repos_report(report: &ReposReport) {
    let term = crate::log::terminal();
    log::data("");
    log::data(
        &Prose::new(format!(
            "<blue><bold>Repos</bold></blue> <dim>{} → {}</dim>",
            report.range.from, report.range.to
        ))
        .render(&term),
    );

    let mut table = base_table(vec![
        TableColumn::new("Repo"),
        TableColumn::new("Events").with_alignment(Alignment::Right),
        TableColumn::new("Sessions").with_alignment(Alignment::Right),
        TableColumn::new("Branches"),
        TableColumn::new("SHAs").with_alignment(Alignment::Right),
        TableColumn::new("Dirty flips").with_alignment(Alignment::Right),
    ]);

    for repo in &report.repos {
        let label = repo_label(repo.repo_org.as_deref(), Some(repo.repo_name.as_str()));
        table.add_row(vec![
            label.into(),
            repo.event_count.to_string().into(),
            repo.session_count.to_string().into(),
            render_labeled_counts(&repo.branches).into(),
            repo.head_sha_count.to_string().into(),
            repo.dirty_transitions.to_string().into(),
        ]);
    }

    log::data(&table.render(&term));
}
