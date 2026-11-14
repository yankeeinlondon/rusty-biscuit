//! `md init`: create or complete the user configuration file and, with
//! `--repo`, the repository's.
//!
//! The work is [`darkmatter::config::init`]'s; this leaf picks the scopes,
//! checks the repository before anything is written, and reports each file
//! and what happened to it. User guide: `darkmatter/docs/cli/init.md`.

use crate::args::Cli;
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::output::output_diagnostic;
use crate::output::report::ClassifiedFailure;
use crate::request::MdRequest;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Report, Result};
use darkmatter::config::init::{InitError, InitOptions, InitReport, init};
use darkmatter::config::schema::ConfigScope;
use darkmatter::markdown::compose::expression::{DiagnosticSource, Severity};
use darkmatter::markdown::compose::report_diagnostics::config_error_diagnostics;
use darkmatter::markdown::output::command::{CommandValue, prose_document};
use serde::Serialize;

/// `md init`'s data value.
#[derive(Debug, Clone, Serialize)]
struct InitValue {
    files: Vec<FileRow>,
    notices: Vec<NoticeRow>,
}

#[derive(Debug, Clone, Serialize)]
struct FileRow {
    path: String,
    scope: &'static str,
    action: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct NoticeRow {
    file: String,
    message: String,
}

/// Runs `md init`.
///
/// ## Errors
///
/// A usage error from `--width`; [`crate::output::report::ReportedExit`]
/// when the run failed or its result cannot be written; for a formatted
/// output, the setup error itself, for `main` to report.
pub(crate) fn run_init(repo: bool, inline: bool, output: OutputTarget, cli: &Cli, request: &MdRequest) -> Result<()> {
    let run = CommandRun::begin(
        output,
        cli.width,
        WidthRequest::Auto,
        cli.verbose,
        DataShape::typed(InitValue { files: Vec::new(), notices: Vec::new() }),
    )?;
    let result = initialize(repo, inline, request).map(|report| {
        let value = value_of(&report);
        let markdown = report_markdown(&value);
        let terminal_value = value.clone();
        Completed::success(
            CommandValue::new(value, prose_document(&markdown))
                .with_terminal(move |options| report_terminal(&terminal_value, &Terminal::from_output_options(options))),
        )
    });
    run.finish(result)
}

fn initialize(repo: bool, inline: bool, request: &MdRequest) -> Result<InitReport> {
    let options = InitOptions { inline };
    // The launch context names the repository, if any; `init` checks every
    // scope before it reads or writes a file, and the user scope reads only
    // the home.
    let context = request.launch_context()?;
    let scopes: &[ConfigScope] =
        if repo { &[ConfigScope::User, ConfigScope::Repository] } else { &[ConfigScope::User] };
    let outcome = init(context, scopes, options);
    outcome.map_err(failure)
}

/// The setup error with its records: the configuration's own located ones
/// for an invalid file, otherwise one `dm.config.not_saved` record.
fn failure(error: InitError) -> Report {
    let diagnostics = match &error {
        InitError::Invalid(config) => config_error_diagnostics(config),
        other => {
            let mut record = output_diagnostic(other.code(), Severity::Error, &other.to_string());
            let file = match other {
                InitError::Read { path, .. } => Some(path.clone()),
                _ => None,
            };
            record.source = file.map(|file| DiagnosticSource::file_only(Some(file)));
            vec![record]
        }
    };
    ClassifiedFailure::located(Report::new(error), diagnostics)
}

fn value_of(report: &InitReport) -> InitValue {
    InitValue {
        files: report
            .files
            .iter()
            .map(|file| FileRow {
                path: file.path.display().to_string(),
                scope: match file.scope {
                    ConfigScope::User => "user",
                    ConfigScope::Repository => "repository",
                },
                action: file.action.as_str(),
            })
            .collect(),
        notices: report
            .notices
            .iter()
            .map(|notice| NoticeRow { file: notice.file.display().to_string(), message: notice.message.clone() })
            .collect(),
    }
}

fn status_line(file: &FileRow) -> String {
    let verb = match file.action {
        "created" => "Created",
        "updated" => "Updated",
        _ => "Unchanged",
    };
    format!("{verb} `{}` ({} configuration)", file.path, file.scope)
}

fn report_markdown(value: &InitValue) -> String {
    let mut out = String::new();
    for file in &value.files {
        out.push_str(&format!("- {}\n", status_line(file)));
    }
    for notice in &value.notices {
        out.push_str(&format!("\nNote: `{}`: {}\n", notice.file, notice.message));
    }
    out
}

fn report_terminal(value: &InitValue, terminal: &Terminal) -> String {
    let mut list = UnorderedList::empty();
    for file in &value.files {
        list.add(Prose::new(Prose::escape_text_outside_code_spans(&status_line(file))).with_line_breaks(LineBreaks::Hard));
    }
    let mut out = list.render(terminal).trim_end().to_string();
    for notice in &value.notices {
        let text = format!("<b>Note:</b> {}: {}", Prose::escape_text(&notice.file), Prose::escape_text(&notice.message));
        out.push_str("\n\n");
        out.push_str(&Prose::new(text).render(terminal));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_markdown_names_each_file_its_scope_and_what_happened() {
        let value = InitValue {
            files: vec![
                FileRow { path: "/h/.darkmatter.yaml".into(), scope: "user", action: "created" },
                FileRow { path: "/r/.darkmatter.yaml".into(), scope: "repository", action: "unchanged" },
            ],
            notices: vec![NoticeRow { file: "/h/.darkmatter.yaml".into(), message: "defaults used".into() }],
        };
        let markdown = report_markdown(&value);
        assert!(markdown.contains("- Created `/h/.darkmatter.yaml` (user configuration)"), "{markdown}");
        assert!(markdown.contains("- Unchanged `/r/.darkmatter.yaml` (repository configuration)"), "{markdown}");
        assert!(markdown.contains("Note: `/h/.darkmatter.yaml`: defaults used"), "{markdown}");
    }
}
