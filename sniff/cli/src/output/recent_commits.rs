use biscuit_terminal::terminal::Terminal;
use chrono::Utc;
use serde_json::{Value, json};

use sniff::filesystem::git::recent_commits::CommitDescSet;
use sniff::filesystem::git::{PeriodSpecifier, parse_period};

use crate::args::{RecentCommitActionArg, RepoAction};
use crate::output::commit_blocks::{
    CommitCentricFilter, filter_commit_set, render_commit_set_styled,
};
use crate::output::emit_text;
use crate::perf::{CliPerf, handle_no_results};

pub(crate) fn handle_recent_commits_command(
    action: &RepoAction,
    base_dir: Option<&std::path::Path>,
    json: bool,
    plain: bool,
    _verbose: u8,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    let RecentCommitsParams {
        period,
        actions,
        package,
        package_area,
        no_error,
        on_error,
        mode,
    } = extract_action_params(action);

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
        PeriodSpecifier::Count(count) => {
            sniff::filesystem::get_recent_commits_by_count(dir, *count)?
        }
    };

    if let Some(ref pkg) = package {
        commit_set.filter_by_package(pkg)?;
    }
    if let Some(ref area) = package_area {
        commit_set.filter_by_package_area(area)?;
    }
    if !actions.is_empty() {
        commit_set.filter_by_actions(actions.iter().copied().map(RecentCommitActionArg::as_str));
    }

    if commit_set.commits.is_empty() {
        let msg = on_error.clone().unwrap_or_else(|| "none found".to_string());
        return handle_no_results(no_error, &Some(msg), plain, perf);
    }

    if json {
        // The two filtered modes still honor the no-results policy at the
        // command level; the value builder itself always returns a stable
        // object and is reused by the `repo --json` aggregate.
        match mode {
            RecentCommitsMode::SourceCodeChanges | RecentCommitsMode::DocumentationChanges => {
                let centric_filter = match mode {
                    RecentCommitsMode::SourceCodeChanges => CommitCentricFilter::SourceCode,
                    RecentCommitsMode::DocumentationChanges => CommitCentricFilter::Documentation,
                    RecentCommitsMode::RecentCommits => unreachable!(),
                };
                let filtered = filter_commit_set(&commit_set, centric_filter);
                if filtered.commits.is_empty() {
                    let msg = on_error.clone().unwrap_or_else(|| "none found".to_string());
                    return handle_no_results(no_error, &Some(msg), plain, perf);
                }
            }
            _ => {}
        }

        let value = commit_family_value(&commit_set, mode);
        crate::output::print_json_value(value, perf.build_report().as_ref());
        return Ok(());
    }

    if plain {
        // Plain mode delegates to the library's markdown emitter so the
        // `--plain` output stays faithful to what would land in a file.
        let markdown = match mode {
            RecentCommitsMode::RecentCommits => commit_set.describe(true),
            RecentCommitsMode::SourceCodeChanges => commit_set.source_code_changes(true),
            RecentCommitsMode::DocumentationChanges => commit_set.documentation_changes(true),
        };

        if markdown.is_empty() {
            let msg = on_error.clone().unwrap_or_else(|| "none found".to_string());
            return handle_no_results(no_error, &Some(msg), plain, perf);
        }
        emit_text(&markdown, true);
        perf.emit_stderr(None);
        return Ok(());
    }

    // Styled terminal mode — render directly from the commit data so we
    // can apply fine-grained Prose-based formatting (colored conventional
    // commit prefix, bold hash, italic `at`, bold time, OSC8 file links).
    let filter = match mode {
        RecentCommitsMode::RecentCommits => CommitCentricFilter::All,
        RecentCommitsMode::SourceCodeChanges => CommitCentricFilter::SourceCode,
        RecentCommitsMode::DocumentationChanges => CommitCentricFilter::Documentation,
    };
    let terminal = Terminal::default();
    let rendered = render_commit_set_styled(&commit_set, filter, &terminal);

    if rendered.is_empty() {
        let msg = on_error.clone().unwrap_or_else(|| "none found".to_string());
        return handle_no_results(no_error, &Some(msg), plain, perf);
    }

    emit_text(&rendered, false);
    perf.emit_stderr(None);
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum RecentCommitsMode {
    RecentCommits,
    SourceCodeChanges,
    DocumentationChanges,
}

/// Build the JSON value for a commit-family subcommand.
///
/// Returns the focused shape (`commits`, `period_label`, `repo_root`, plus
/// `packages`/`filter`) even when `commit_set.commits` is empty, so the
/// `repo --json` aggregate can include a stable object for
/// `recent-commits`, `source-code-changes`, and `documentation-changes`
/// without triggering the leaf command's no-results exit path.
pub(crate) fn commit_family_value(commit_set: &CommitDescSet, mode: RecentCommitsMode) -> Value {
    match mode {
        RecentCommitsMode::RecentCommits => serde_json::to_value(commit_set).unwrap_or(Value::Null),
        RecentCommitsMode::SourceCodeChanges | RecentCommitsMode::DocumentationChanges => {
            let (filter, filter_label) = match mode {
                RecentCommitsMode::SourceCodeChanges => {
                    (CommitCentricFilter::SourceCode, "source_code")
                }
                RecentCommitsMode::DocumentationChanges => {
                    (CommitCentricFilter::Documentation, "documentation")
                }
                RecentCommitsMode::RecentCommits => unreachable!(),
            };
            let mut filtered = filter_commit_set(commit_set, filter);
            // Trim full package metadata for brevity in filtered commit views.
            filtered.packages = None;
            let mut value = serde_json::to_value(&filtered).unwrap_or(Value::Null);
            if let Some(obj) = value.as_object_mut() {
                obj.insert("filter".into(), json!(filter_label));
            }
            value
        }
    }
}

struct RecentCommitsParams {
    period: Option<String>,
    actions: Vec<RecentCommitActionArg>,
    package: Option<String>,
    package_area: Option<String>,
    no_error: bool,
    on_error: Option<String>,
    mode: RecentCommitsMode,
}

fn extract_action_params(action: &RepoAction) -> RecentCommitsParams {
    match action {
        RepoAction::RecentCommits {
            period,
            actions,
            package,
            package_area,
            no_error,
            on_error,
        } => RecentCommitsParams {
            period: period.clone(),
            actions: actions.clone(),
            package: package.clone(),
            package_area: package_area.clone(),
            no_error: *no_error,
            on_error: on_error.clone(),
            mode: RecentCommitsMode::RecentCommits,
        },
        RepoAction::SourceCodeChanges {
            period,
            actions,
            package,
            package_area,
            no_error,
            on_error,
        } => RecentCommitsParams {
            period: period.clone(),
            actions: actions.clone(),
            package: package.clone(),
            package_area: package_area.clone(),
            no_error: *no_error,
            on_error: on_error.clone(),
            mode: RecentCommitsMode::SourceCodeChanges,
        },
        RepoAction::DocumentationChanges {
            period,
            actions,
            package,
            package_area,
            no_error,
            on_error,
        } => RecentCommitsParams {
            period: period.clone(),
            actions: actions.clone(),
            package: package.clone(),
            package_area: package_area.clone(),
            no_error: *no_error,
            on_error: on_error.clone(),
            mode: RecentCommitsMode::DocumentationChanges,
        },
        _ => unreachable!(),
    }
}
