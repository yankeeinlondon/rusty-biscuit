use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use inquire::{InquireError, Select, Text};
use sniff::os::{RuntimeEnvironment, detect_runtime_environment};
use worktree::WorktreeError;
use worktree::config::{
    BaseDirSource, config_path, considered_dirs, example_base_dir, resolve_base_dir, save_config,
    validate_base_dir, wsl_drive_hint,
};
use worktree::git::repo_info;
use worktree::include::copy::SkipReason;
use worktree::worktree::{IncludeOutcome, create_worktree};

pub fn run(branch: &str, from: Option<&str>, stay: bool) -> Result<(), WorktreeError> {
    let terminal = Terminal::default();

    // Every other resolution error ends the command; `main` renders it with
    // [`base_dir_error_markup`] and exits 1.
    let base = match resolve_base_dir() {
        Ok(path) => path,
        Err(WorktreeError::BaseDirectoryNotConfigured) if crate::env::is_interactive() => {
            setup_base_dir(&terminal)?
        }
        Err(e) => return Err(e),
    };

    let result = create_worktree(branch, &base, from)?;
    let move_shell = !stay && crate::env::shell_wrapper_active();

    let relative = repo_info().map(|i| i.relative_path).unwrap_or_default();
    let location = if relative.as_os_str().is_empty() {
        "<dim><i>repo root</i></dim>".to_string()
    } else {
        format!("<dim>{}</dim>", Prose::escape_text(&relative.to_string_lossy()))
    };
    let forked = result
        .forked_from
        .as_deref()
        .map(|base| format!(" <dim>(forked from <b>{}</b>)</dim>", Prose::escape_text(base)))
        .unwrap_or_default();
    let mut msg = format!(
        "<green>Created worktree</green> <bold>{}</bold>{forked} at <dim>{}</dim>",
        Prose::escape_text(&result.branch),
        Prose::escape_text(&result.worktree_path.display().to_string()),
    );
    if move_shell {
        msg.push_str(&format!(
            "\n  <dim>- you have been moved <i>into</i> the worktree at the same relative path ({location})</dim>"
        ));
    }
    eprintln!();
    eprintln!("{}", hard_lines(msg).render(&terminal));
    eprintln!();

    if let Some(commit) = &result.reused_branch_at {
        let notice = build_reuse_notice(&result.branch, commit);
        eprintln!("{}", hard_lines(notice).render(&terminal));
    }
    render_include_report(&terminal, &result.include);

    if move_shell {
        println!("cd:{}", result.target_cwd.display());
    } else if !stay {
        let notice = format!(
            "<yellow><b>Could not move your shell into the worktree:</b></yellow> the shell wrapper is not active.\n{}",
            super::go::wrapper_setup_help()
        );
        eprintln!("{}", hard_lines(notice).render(&terminal));
    }

    Ok(())
}

/// Prose whose single newlines are line breaks, for messages laid out one
/// line per `\n`.
fn hard_lines(markup: impl Into<String>) -> Prose {
    Prose::new(markup.into()).with_line_breaks(LineBreaks::Hard)
}

fn display_include_path(path: &[u8]) -> String {
    let mut visible = String::new();
    for byte in path {
        if *byte == b' ' { visible.push_str("\\ "); }
        else { visible.extend(std::ascii::escape_default(*byte).map(char::from)); }
    }
    Prose::escape_text(&visible)
}

fn render_include_report(terminal: &Terminal, include: &IncludeOutcome) {
    if !include.copied.is_empty() {
        let names = include.copied.iter().map(|path| display_include_path(path)).collect::<Vec<_>>().join(", ");
        let source = Prose::escape_text(&include.source_label);
        eprintln!("{}", Prose::new(format!("Copied from <b>{source}</b>: {names}")).render(terminal));
    }
    let mut warnings = UnorderedList::empty();
    let mut has_warnings = false;
    for (path, reason) in &include.skipped {
        let reason = match reason {
            SkipReason::Existing => "already exists in the destination",
            SkipReason::IndexTracked => "tracked in the destination",
            SkipReason::LinkUnsupported => "symbolic link could not be created",
            SkipReason::UnsafePath => "path is unsafe to copy",
        };
        warnings.add(Prose::new(format!("Skipped {}: {reason}", display_include_path(path))));
        has_warnings = true;
    }
    for (path, reason) in &include.failed {
        warnings.add(Prose::new(format!("Could not copy {}: {}", display_include_path(path), Prose::escape_text_outside_code_spans(reason))));
        has_warnings = true;
    }
    for warning in &include.warnings {
        warnings.add(Prose::new(Prose::escape_text_outside_code_spans(warning)));
        has_warnings = true;
    }
    if has_warnings { eprintln!("{}", warnings.render(terminal)); }
}

/// Notice shown when `create` re-attached a pre-existing branch instead of
/// forking a fresh one. Surfaces the commit so a stale branch is not reused
/// silently.
fn build_reuse_notice(branch: &str, commit: &str) -> String {
    let branch = Prose::escape_text(branch);
    let commit = Prose::escape_text(commit);
    format!(
        "<yellow><b>Note:</b></yellow> branch <bold>{branch}</bold> already existed and was \
        reused at <dim>{commit}</dim> — <b>not</b> forked from your current branch.\n  \
        <dim>Delete it first (<i>git branch -d {branch}</i>) if you wanted a fresh fork.</dim>"
    )
}

/// The full markup for a base-directory or `~/.worktree.json` error, or
/// `None` for any other error.
///
/// User-controlled values are escaped. An error from the config file links to
/// it; a Windows drive path rejected under WSL gets the `/mnt` suggestion.
/// WSL is detected only for that rejection.
pub fn base_dir_error_markup(error: &WorktreeError) -> Option<String> {
    base_dir_error_markup_with(error, detect_runtime_environment, considered_dirs)
}

fn base_dir_error_markup_with(
    error: &WorktreeError,
    runtime: impl FnOnce() -> RuntimeEnvironment,
    candidates: impl FnOnce() -> Vec<PathBuf>,
) -> Option<String> {
    let origin = match error {
        WorktreeError::ConfigInvalidFormat { config_path, message } => {
            return Some(invalid_format_markup(config_path, message));
        }
        WorktreeError::ConfigBaseDirIsGitRepo { config_path, .. } => {
            return Some(config_repo_root_markup(error, config_path, candidates()));
        }
        WorktreeError::BaseDirectoryEmpty { origin }
        | WorktreeError::BaseDirectoryForeignPath { origin, .. }
        | WorktreeError::BaseDirectoryNotAbsolute { origin, .. }
        | WorktreeError::BaseDirectoryNotDirectory { origin, .. }
        | WorktreeError::BaseDirectoryInvalidEncoding { origin, .. }
        | WorktreeError::BaseDirectoryNotFound { origin, .. }
        | WorktreeError::BaseDirectoryUnreadable { origin, .. }
        | WorktreeError::BaseDirectoryIsGitRepo { origin, .. } => origin,
        _ => return None,
    };
    let mut markup = format!("<red><b>Error:</b></red> {}", Prose::escape_text(&error.to_string()));
    if let WorktreeError::BaseDirectoryForeignPath { value, .. } = error
        && let Some(hint) = wsl_drive_hint(value, runtime())
    {
        markup.push_str(&format!("\n\n{}", Prose::escape_text(&hint)));
    }
    if let BaseDirSource::ConfigFile(config_path) = origin {
        markup.push_str(&format!("\n\n{}", edit_config_link(config_path)));
    }
    Some(markup)
}

fn edit_config_link(config_path: &Path) -> String {
    let href = Prose::quoted_attr(&config_path.display().to_string());
    format!("<a href={href}>Edit ~/.worktree.json</a>")
}

fn invalid_format_markup(config_path: &Path, message: &str) -> String {
    let message = Prose::escape_text(message);
    // The example is shown as JSON, so a Windows path's backslashes are doubled.
    let example = Prose::escape_text(&example_base_dir().replace('\\', r"\\"));
    format!(
        "<red><b>Error:</b></red> <b>~/.worktree.json</b> has an invalid format: {message}\n\n\
        The file must be structured as:\n\n\
        <dim>{{</dim>\n\
        <dim>  \"base_dir\": \"{example}\"</dim>\n\
        <dim>}}</dim>\n\n\
        {} to fix the format, or delete the file and run <dim>wt create</dim> again \
        in an interactive terminal to choose a directory.",
        edit_config_link(config_path)
    )
}

fn config_repo_root_markup(error: &WorktreeError, config_path: &Path, candidates: Vec<PathBuf>) -> String {
    let suggestions: Vec<String> = candidates
        .into_iter()
        .filter(|p| !p.join(".git").exists())
        .map(|p| format!("  • <dim>{}</dim>", Prose::escape_text(&p.display().to_string())))
        .collect();
    let suggestions = if suggestions.is_empty() {
        String::new()
    } else {
        format!("\n\nCommon choices:\n{}", suggestions.join("\n"))
    };
    format!(
        "<red><b>Error:</b></red> {}\n\n{} to point to a directory without .git directly inside it.{suggestions}",
        Prose::escape_text(&error.to_string()),
        edit_config_link(config_path)
    )
}

/// Interactive setup: asks for a base directory, prepares it, and saves it
/// to `~/.worktree.json`.
fn setup_base_dir(terminal: &Terminal) -> Result<PathBuf, WorktreeError> {
    let intro = "<b>No worktree base directory configured.</b> Let's set one up.\n\n\
        This is where <dim>wt</dim> will store all worktrees, organized as \
        <dim>{base}/{repo}/{branch}/</dim>. Your choice will be saved to \
        <dim>~/.worktree.json</dim>.";
    eprintln!();
    eprintln!("{}", Prose::new(intro).render(terminal));

    let candidates = setup_candidates(considered_dirs());
    let path = choose_base_dir(&mut InquirePrompt { terminal }, &candidates, save_config)?;

    let config_str = config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.worktree.json".to_string());
    let confirm = format!(
        "<green>Saved</green> base directory <dim>{}</dim> to <dim>{}</dim>.",
        Prose::escape_text(&path.display().to_string()),
        Prose::escape_text(&config_str)
    );
    eprintln!();
    eprintln!("{}", Prose::new(confirm).render(terminal));

    Ok(path)
}

/// The suggested directories worth offering: not a repository root, and
/// spelled in Unicode so the choice can be saved exactly as shown.
fn setup_candidates(dirs: Vec<PathBuf>) -> Vec<String> {
    dirs.into_iter()
        .filter(|p| !p.join(".git").exists())
        .filter_map(|p| p.to_str().map(str::to_owned))
        .collect()
}

/// A setup-menu answer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SetupChoice {
    /// One of the suggested directories, as offered.
    Candidate(String),
    /// "Other": the user types a path.
    Other,
}

/// The questions setup asks; [`InquirePrompt`] asks them in the terminal.
/// A cancelled question returns [`WorktreeError::Cancelled`].
trait SetupPrompt {
    fn select(&mut self, candidates: &[String]) -> Result<SetupChoice, WorktreeError>;
    fn enter_path(&mut self) -> Result<String, WorktreeError>;
    /// Shows why a choice cannot be used, before asking again.
    fn reject(&mut self, reason: &WorktreeError);
}

/// Asks until a choice passes [`prepare_base_dir`], then saves it.
///
/// An unusable suggested choice returns to the menu and an unusable typed
/// path asks for another path; nothing is created or saved for either. An I/O
/// failure or a cancelled question ends setup without saving.
fn choose_base_dir(
    prompt: &mut impl SetupPrompt,
    candidates: &[String],
    save: impl FnOnce(&Path) -> Result<(), WorktreeError>,
) -> Result<PathBuf, WorktreeError> {
    let path = 'chosen: loop {
        match prompt.select(candidates)? {
            SetupChoice::Candidate(raw) => match prepare_base_dir(&raw) {
                Ok(path) => break path,
                Err(e) if can_choose_again(&e) => prompt.reject(&e),
                Err(e) => return Err(e),
            },
            SetupChoice::Other => loop {
                let typed = prompt.enter_path()?;
                match prepare_base_dir(typed.trim()) {
                    Ok(path) => break 'chosen path,
                    Err(e) if can_choose_again(&e) => prompt.reject(&e),
                    Err(e) => return Err(e),
                }
            },
        }
    };
    save(&path)?;
    Ok(path)
}

/// Whether the error is about the chosen value, so another choice may work,
/// rather than a failure to read or create the filesystem.
fn can_choose_again(error: &WorktreeError) -> bool {
    matches!(
        error,
        WorktreeError::BaseDirectoryEmpty { .. }
            | WorktreeError::BaseDirectoryForeignPath { .. }
            | WorktreeError::BaseDirectoryNotAbsolute { .. }
            | WorktreeError::BaseDirectoryNotDirectory { .. }
            | WorktreeError::BaseDirectoryIsGitRepo { .. }
    )
}

/// Makes `raw` a usable base directory: valid syntax first, then an existing
/// directory, or a new one when nothing is there, without `.git` directly
/// inside it.
///
/// Syntax is checked before anything is created, so a relative or foreign
/// value never becomes a directory under the current one.
fn prepare_base_dir(raw: &str) -> Result<PathBuf, WorktreeError> {
    let path = validate_base_dir(raw, BaseDirSource::Prompt)?;
    let unreadable = |cause| WorktreeError::BaseDirectoryUnreadable {
        value: raw.to_owned(),
        origin: BaseDirSource::Prompt,
        cause,
    };
    let is_dir = match std::fs::metadata(&path) {
        Ok(metadata) => metadata.is_dir(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(&path).map_err(|e| {
                WorktreeError::Io(std::io::Error::new(
                    e.kind(),
                    format!("could not create base directory '{raw}': {e}"),
                ))
            })?;
            std::fs::metadata(&path).map_err(unreadable)?.is_dir()
        }
        Err(cause) => return Err(unreadable(cause)),
    };
    if !is_dir {
        return Err(WorktreeError::BaseDirectoryNotDirectory { value: raw.to_owned(), origin: BaseDirSource::Prompt });
    }
    if path.join(".git").exists() {
        return Err(WorktreeError::BaseDirectoryIsGitRepo { value: raw.to_owned(), origin: BaseDirSource::Prompt });
    }
    Ok(path)
}

struct InquirePrompt<'a> {
    terminal: &'a Terminal,
}

impl SetupPrompt for InquirePrompt<'_> {
    fn select(&mut self, candidates: &[String]) -> Result<SetupChoice, WorktreeError> {
        let mut options = candidates.to_vec();
        options.push("Other (specify manually)".to_string());
        let picked = Select::new("Choose a base directory:", options)
            .raw_prompt()
            .map_err(map_inquire_err)?;
        Ok(match candidates.get(picked.index) {
            Some(candidate) => SetupChoice::Candidate(candidate.clone()),
            None => SetupChoice::Other,
        })
    }

    fn enter_path(&mut self) -> Result<String, WorktreeError> {
        Text::new("Base directory path (absolute):").prompt().map_err(map_inquire_err)
    }

    fn reject(&mut self, reason: &WorktreeError) {
        let markup = base_dir_error_markup(reason).unwrap_or_else(|| Prose::escape_text(&reason.to_string()));
        eprintln!();
        eprintln!("{}", hard_lines(markup).render(self.terminal));
        eprintln!();
    }
}

fn map_inquire_err(e: InquireError) -> WorktreeError {
    match e {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => {
            WorktreeError::Cancelled
        }
        e => WorktreeError::Io(std::io::Error::other(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuse_notice_names_branch_commit_and_flags_no_fork() {
        let msg = build_reuse_notice("w-cli-heretic", "d4ea98a");
        assert!(msg.contains("w-cli-heretic"));
        assert!(msg.contains("d4ea98a"));
        assert!(msg.contains("not"));
    }

    #[test]
    fn reuse_notice_shows_a_markup_like_branch_literally() {
        let msg = build_reuse_notice("feat/<red>x_y_z", "d4ea98a");
        let rendered = biscuit_test_harness::strip_ansi(&Prose::new(msg).render_optimistic(Some(400)));
        assert!(rendered.contains("branch feat/<red>x_y_z already existed"), "{rendered:?}");
        assert!(rendered.contains("git branch -d feat/<red>x_y_z"), "{rendered:?}");
    }

    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// A value in the other operating system's syntax: the incident's `b:\wt`
    /// on POSIX, a POSIX path on Windows.
    const FOREIGN: &str = if cfg!(windows) { "/home/me/wt" } else { r"b:\wt" };

    enum Answer {
        Select(SetupChoice),
        Type(&'static str),
        TypePath(PathBuf),
        Cancel,
    }

    /// Answers setup's questions from a script and records every rejection.
    struct ScriptedPrompt {
        answers: VecDeque<Answer>,
        rejected: Vec<String>,
    }

    impl ScriptedPrompt {
        fn new(answers: impl IntoIterator<Item = Answer>) -> Self {
            Self { answers: answers.into_iter().collect(), rejected: Vec::new() }
        }

        fn next(&mut self) -> Answer {
            self.answers.pop_front().expect("setup asked more questions than the script answers")
        }
    }

    impl SetupPrompt for ScriptedPrompt {
        fn select(&mut self, _candidates: &[String]) -> Result<SetupChoice, WorktreeError> {
            match self.next() {
                Answer::Select(choice) => Ok(choice),
                Answer::Cancel => Err(WorktreeError::Cancelled),
                _ => panic!("expected a menu answer"),
            }
        }

        fn enter_path(&mut self) -> Result<String, WorktreeError> {
            match self.next() {
                Answer::Type(text) => Ok(text.to_owned()),
                Answer::TypePath(path) => Ok(path.to_str().unwrap().to_owned()),
                Answer::Cancel => Err(WorktreeError::Cancelled),
                Answer::Select(_) => panic!("expected a typed path"),
            }
        }

        fn reject(&mut self, reason: &WorktreeError) {
            self.rejected.push(reason.to_string());
        }
    }

    fn run_setup(prompt: &mut ScriptedPrompt, candidates: &[String]) -> (Result<PathBuf, WorktreeError>, Vec<PathBuf>) {
        let saved = RefCell::new(Vec::new());
        let result = choose_base_dir(prompt, candidates, |path| {
            saved.borrow_mut().push(path.to_path_buf());
            Ok(())
        });
        (result, saved.into_inner())
    }

    fn other() -> Answer {
        Answer::Select(SetupChoice::Other)
    }

    #[test]
    fn invalid_typed_path_then_valid_one_saves_only_the_valid_one() {
        let root = tempfile::tempdir().unwrap();
        let chosen = root.path().join("wt");
        let mut prompt = ScriptedPrompt::new([other(), Answer::Type(FOREIGN), Answer::TypePath(chosen.clone())]);

        let (result, saved) = run_setup(&mut prompt, &[]);

        assert_eq!(result.unwrap(), chosen);
        assert_eq!(saved, std::slice::from_ref(&chosen));
        assert!(chosen.is_dir(), "the valid choice is created before it is saved");
        assert_eq!(prompt.rejected.len(), 1, "{:?}", prompt.rejected);
        assert!(prompt.rejected[0].contains(FOREIGN) && prompt.rejected[0].contains("the setup prompt"), "{:?}", prompt.rejected);
        assert!(!Path::new(FOREIGN).exists(), "a foreign value is never created relative to the current directory");
    }

    #[test]
    fn invalid_typed_path_then_cancel_saves_and_creates_nothing() {
        for typed in [FOREIGN, "wt", "./wt", "~/wt", "   "] {
            let mut prompt = ScriptedPrompt::new([other(), Answer::Type(typed), Answer::Cancel]);

            let (result, saved) = run_setup(&mut prompt, &[]);

            assert!(matches!(result, Err(WorktreeError::Cancelled)), "{typed:?}: {result:?}");
            assert!(saved.is_empty(), "{typed:?}");
            assert_eq!(prompt.rejected.len(), 1, "{typed:?}");
            if !typed.trim().is_empty() {
                assert!(!Path::new(typed).exists(), "{typed:?} was created relative to the current directory");
            }
        }
    }

    #[test]
    fn a_home_relative_typed_path_gets_the_full_path_hint() {
        let mut prompt = ScriptedPrompt::new([other(), Answer::Type("~/wt"), Answer::Cancel]);

        let _ = run_setup(&mut prompt, &[]);

        assert!(prompt.rejected[0].contains("full home-directory path"), "{:?}", prompt.rejected);
    }

    #[test]
    fn unusable_suggestions_return_to_the_menu_before_anything_is_saved() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("file");
        std::fs::write(&file, b"").unwrap();
        let repo_root = root.path().join("repo");
        std::fs::create_dir_all(repo_root.join(".git")).unwrap();
        let fresh = root.path().join("fresh").join("wt");
        let candidates: Vec<String> =
            [&file, &repo_root, &fresh].iter().map(|p| p.to_str().unwrap().to_owned()).collect();
        let mut prompt = ScriptedPrompt::new([
            Answer::Select(SetupChoice::Candidate(candidates[0].clone())),
            Answer::Select(SetupChoice::Candidate(candidates[1].clone())),
            Answer::Select(SetupChoice::Candidate(candidates[2].clone())),
        ]);

        let (result, saved) = run_setup(&mut prompt, &candidates);

        assert_eq!(result.unwrap(), fresh);
        assert_eq!(saved, std::slice::from_ref(&fresh));
        assert!(fresh.is_dir());
        assert_eq!(prompt.rejected.len(), 2, "{:?}", prompt.rejected);
        assert!(prompt.rejected[0].contains("is not a directory"), "{:?}", prompt.rejected);
        assert!(prompt.rejected[1].contains("root of a Git repository"), "{:?}", prompt.rejected);
        assert_eq!(std::fs::read(&file).unwrap(), b"", "the rejected file is left alone");
    }

    #[test]
    fn a_path_that_cannot_be_created_stops_setup_without_saving() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("file");
        std::fs::write(&file, b"").unwrap();
        let mut prompt = ScriptedPrompt::new([other(), Answer::TypePath(file.join("wt"))]);

        let (result, saved) = run_setup(&mut prompt, &[]);

        let error = result.unwrap_err();
        assert!(!can_choose_again(&error) && !matches!(error, WorktreeError::Cancelled), "{error:?}");
        assert!(saved.is_empty());
        assert!(prompt.rejected.is_empty());
        assert!(prompt.answers.is_empty());
    }

    #[test]
    fn setup_candidates_skip_repository_roots() {
        let root = tempfile::tempdir().unwrap();
        let repo_root = root.path().join("repo");
        std::fs::create_dir_all(repo_root.join(".git")).unwrap();
        let plain = root.path().join("wt");

        assert_eq!(setup_candidates(vec![repo_root, plain.clone()]), [plain.to_str().unwrap().to_owned()]);
    }

    fn render(markup: &str) -> String {
        biscuit_test_harness::strip_ansi(&hard_lines(markup).render_optimistic(Some(400)))
    }

    fn markup(error: &WorktreeError, runtime: RuntimeEnvironment) -> String {
        base_dir_error_markup_with(error, || runtime, Vec::new).expect("a base-directory error")
    }

    #[test]
    fn config_errors_link_the_config_file_and_show_values_literally() {
        let config = PathBuf::from(if cfg!(windows) { r"C:\h\<b>.worktree.json" } else { "/h/<b>.worktree.json" });
        let error = WorktreeError::BaseDirectoryNotAbsolute {
            value: "<red>w_t</red>".into(),
            origin: BaseDirSource::ConfigFile(config.clone()),
        };

        let markup = markup(&error, RuntimeEnvironment::Native);

        assert!(markup.contains(&format!("<a href={}>", Prose::quoted_attr(&config.display().to_string()))), "{markup}");
        let rendered = render(&markup);
        assert!(rendered.contains("Base directory '<red>w_t</red>' from the config file"), "{rendered}");
        assert!(rendered.contains("Edit ~/.worktree.json"), "{rendered}");
    }

    #[test]
    fn env_and_prompt_errors_have_no_config_link() {
        for origin in [BaseDirSource::Env, BaseDirSource::Prompt] {
            let error = WorktreeError::BaseDirectoryNotFound { value: "/srv/wt".into(), origin };
            assert!(!markup(&error, RuntimeEnvironment::Native).contains("<a href"));
        }
    }

    #[test]
    fn foreign_drive_paths_get_the_wsl_hint_only_under_wsl() {
        let error = |value: &str| WorktreeError::BaseDirectoryForeignPath {
            value: value.into(),
            origin: BaseDirSource::ConfigFile("/h/.worktree.json".into()),
        };

        let rendered = render(&markup(&error(r"b:\wt"), RuntimeEnvironment::Wsl2));
        assert!(rendered.contains(r"Base directory 'b:\wt'"), "{rendered}");
        assert!(rendered.contains("If it is mounted there, use /mnt/b/wt."), "{rendered}");

        assert!(!markup(&error(r"b:\wt"), RuntimeEnvironment::Native).contains("/mnt/"));
        for value in ["b:wt", "b:", r"\\server\share\wt", r"\\?\C:\wt"] {
            assert!(!markup(&error(value), RuntimeEnvironment::Wsl2).contains("/mnt/"), "{value}");
        }
    }

    #[test]
    fn runtime_is_detected_only_for_a_foreign_path() {
        let error = WorktreeError::BaseDirectoryNotAbsolute { value: "wt".into(), origin: BaseDirSource::Env };
        let markup = base_dir_error_markup_with(&error, || panic!("runtime detected"), Vec::new);
        assert!(markup.is_some());
    }

    #[test]
    fn invalid_format_markup_names_the_file_and_offers_no_unconditional_setup() {
        let rendered = render(&invalid_format_markup(Path::new("/h/.worktree.json"), "expected `<x>`"));
        assert!(rendered.contains("~/.worktree.json has an invalid format: expected `<x>`"), "{rendered}");
        let example = example_base_dir().replace('\\', r"\\");
        assert!(rendered.contains(&format!("\"base_dir\": \"{example}\"")), "{rendered}");
        assert!(rendered.contains("in an interactive terminal"), "{rendered}");
    }

    #[test]
    fn other_errors_are_left_to_the_plain_renderer() {
        assert!(base_dir_error_markup_with(&WorktreeError::NotInGitRepo, || panic!(), || panic!()).is_none());
        assert!(base_dir_error_markup_with(&WorktreeError::BaseDirectoryNotConfigured, || panic!(), || panic!()).is_none());
    }
}
