use chrono::Utc;
use darkmatter::markdown::output::terminal::{for_terminal, TerminalOptions};
use darkmatter::markdown::Markdown;

use sniff::filesystem::git::{parse_period, PeriodSpecifier};

use crate::args::RepoAction;
use crate::commands::handle_no_results;
use crate::output::emit_text;

pub(crate) fn handle_recent_commits_command(
    action: &RepoAction,
    base_dir: Option<&std::path::Path>,
    json: bool,
    plain: bool,
    _verbose: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let (period, package, package_area, no_error, on_error, mode) = extract_action_params(action);

    let period_str = period.as_deref().unwrap_or("3d");
    let specifier = parse_period(period_str)?;

    let dir = base_dir.unwrap_or_else(|| std::path::Path::new("."));

    let mut commit_set = match &specifier {
        PeriodSpecifier::Duration(_) => {
            let duration = match &specifier {
                PeriodSpecifier::Duration(d) => *d,
                _ => unreachable!(),
            };
            let label = format!("last {}", period_str);
            sniff::filesystem::get_recent_commits_by_duration(dir, duration, &label)?
        }
        PeriodSpecifier::Today => {
            let label = "today".to_string();
            let now = Utc::now();
            let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
            sniff::filesystem::get_recent_commits_in_range(dir, today_start, now, &label)?
        }
        PeriodSpecifier::Yesterday => {
            let label = "yesterday".to_string();
            let now = Utc::now();
            let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
            let yesterday_start = today_start - chrono::Duration::days(1);
            sniff::filesystem::get_recent_commits_in_range(
                dir,
                yesterday_start,
                today_start,
                &label,
            )?
        }
        PeriodSpecifier::Date(date) => sniff::filesystem::get_recent_commits_by_date(dir, *date)?,
        PeriodSpecifier::Hash(hash) => sniff::filesystem::get_recent_commits_by_hash(dir, hash)?,
    };

    if let Some(ref pkg) = package {
        commit_set.filter_by_package(pkg);
    }
    if let Some(ref area) = package_area {
        commit_set.filter_by_package_area(area);
    }

    if commit_set.commits.is_empty() {
        return handle_no_results(no_error, &on_error, plain);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&commit_set)?);
        return Ok(());
    }

    let markdown = match mode {
        RecentCommitsMode::RecentCommits => commit_set.describe(plain),
        RecentCommitsMode::SourceCodeChanges => commit_set.source_code_changes(plain),
        RecentCommitsMode::DocumentationChanges => commit_set.documentation_changes(plain),
    };

    if markdown.is_empty() {
        return handle_no_results(no_error, &on_error, plain);
    }

    if plain {
        emit_text(&markdown, true);
    } else {
        let md = Markdown::from(markdown.as_str());
        match for_terminal(&md, TerminalOptions::default()) {
            Ok(rendered) => emit_text(&rendered, false),
            Err(_) => emit_text(&markdown, true),
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum RecentCommitsMode {
    RecentCommits,
    SourceCodeChanges,
    DocumentationChanges,
}

fn extract_action_params(
    action: &RepoAction,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    Option<String>,
    RecentCommitsMode,
) {
    match action {
        RepoAction::RecentCommits {
            period,
            package,
            package_area,
            no_error,
            on_error,
        } => (
            period.clone(),
            package.clone(),
            package_area.clone(),
            *no_error,
            on_error.clone(),
            RecentCommitsMode::RecentCommits,
        ),
        RepoAction::SourceCodeChanges {
            period,
            package,
            package_area,
            no_error,
            on_error,
        } => (
            period.clone(),
            package.clone(),
            package_area.clone(),
            *no_error,
            on_error.clone(),
            RecentCommitsMode::SourceCodeChanges,
        ),
        RepoAction::DocumentationChanges {
            period,
            package,
            package_area,
            no_error,
            on_error,
        } => (
            period.clone(),
            package.clone(),
            package_area.clone(),
            *no_error,
            on_error.clone(),
            RecentCommitsMode::DocumentationChanges,
        ),
        _ => unreachable!(),
    }
}
