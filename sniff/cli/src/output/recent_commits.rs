//! `repo recent-commits`, `repo source-code-changes`, and
//! `repo documentation-changes`: presets over the library's recent-commits
//! pipeline. Selection, filtering, linking, and every report byte come from
//! `sniff`; this module only chooses the output form and writes it.

use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::prelude::WordWrap;
use biscuit_terminal::terminal::Terminal;
use sniff::SniffError;
use sniff::filesystem::git::{GitRepo, RecentCommits, RecentCommitsProjection};

use crate::args::RepoAction;
use crate::perf::CliPerf;

pub(crate) fn handle_recent_commits_command(
    action: &RepoAction,
    base_dir: Option<&Path>,
    json: bool,
    plain: bool,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    let (args, projection) = match action {
        RepoAction::RecentCommits(args) => (args, RecentCommitsProjection::All),
        RepoAction::SourceCodeChanges(args) => (args, RecentCommitsProjection::SourceCode),
        RepoAction::DocumentationChanges(args) => (args, RecentCommitsProjection::Documentation),
        _ => unreachable!("dispatched only for commit-family repo actions"),
    };
    let options = args.to_options(projection)?;

    let dir = base_dir.unwrap_or_else(|| Path::new("."));
    let repo =
        GitRepo::discover(dir)?.ok_or_else(|| SniffError::NotARepository(dir.to_path_buf()))?;
    let report = RecentCommits::collect(&repo, &options)?.projected(projection);

    if json {
        crate::output::print_json_value(report.to_json(), perf.build_report().as_ref());
        return Ok(());
    }

    if report.is_empty() {
        // Stdout stays empty so `$(sniff repo recent-commits)` is empty too.
        eprintln!("No commits matched.");
    } else if plain {
        print!("{}", report.to_plain(&options));
    } else {
        let rendered = Prose::new(report.to_prose(&options))
            .with_word_wrap(WordWrap::WrapProse(None, None))
            .render(&Terminal::default());
        println!("{}", rendered.trim_end_matches('\n'));
    }
    perf.emit_stderr(None);
    Ok(())
}
