use clap::{CommandFactory, Parser};
use clap_complete::{CompleteEnv, Shell};
use sniff::filesystem::blast_radius::{
    ChangeScope, ChangedPathKind, ChangedPathQuery, collect_changed_paths,
    find_blast_radius_documents,
};
use sniff::package::{enrich_dependency, is_major_update, is_owner_repo_shorthand};
use sniff::programs::ProgramsInfo;
use sniff::remote::{DocumentCategory, GitRemote, RemoteRepoProvider, RemoteReport};
use sniff::request::*;
use sniff::services::{ServiceState, detect_services};
use sniff::{SniffResult, detect_with_plan};

use crate::args::{
    BlastRadiusScopeArg, COMPLETIONS_HELP, Cli, Commands, DEFAULT_COMMIT_COUNT, DocsFilter,
    FileListArgs, FilesFilter, ServiceStateArg,
};
use crate::output::{self, OutputFilter, PathListFormat};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Handle dynamic shell completions (invoked by shell completion scripts)
    CompleteEnv::with_factory(Cli::command).complete();

    if wants_completions_help() {
        print_completions_help();
        return Ok(());
    }

    // Pre-scan for --plain to disable clap ANSI styling before parsing
    let is_plain = std::env::args().any(|a| a == "--plain");
    if is_plain {
        // NO_COLOR is a well-established convention for disabling terminal colors
        unsafe { std::env::set_var("NO_COLOR", "1") };
    }

    let cli = Cli::parse();

    // Handle --completions first (prints setup instructions)
    if let Some(shell) = cli.completions {
        print_completions(shell);
        return Ok(());
    }

    // No subcommand and no --json flag: show help
    if cli.command.is_none() && !cli.json {
        Cli::command().print_help()?;
        return Ok(());
    }

    if matches!(cli.command, Some(Commands::Topics)) {
        output::emit_text(&output::render_topics_table(), cli.plain);
        return Ok(());
    }

    let output_filter = cli
        .command
        .as_ref()
        .map(Commands::to_output_filter)
        .unwrap_or(OutputFilter::All);

    // Handle programs mode separately (doesn't use SniffResult)
    if let Some(ref cmd) = cli.command {
        if cmd.is_programs_mode() {
            // Check for install action FIRST
            if cmd.is_install_action() {
                let filter = cmd.to_output_filter();
                return match cmd.install_program_name() {
                    Some(name) => crate::install::direct_install(filter, name),
                    None => crate::install::interactive_install(filter),
                };
            }

            let programs = detect_programs_for_filter(output_filter);
            if cli.json {
                output::print_programs_json(&programs, output_filter)?;
            } else {
                let rendered =
                    output::render_programs_markdown(&programs, cli.verbose, output_filter);
                output::emit_text(&rendered, cli.plain);
            }
            return Ok(());
        }

        // Handle just mode separately (doesn't use SniffResult)
        if let Commands::Just {
            filter,
            with,
            grouped,
        } = cmd
        {
            let base = cli
                .base
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));
            let justfiles = sniff::filesystem::detect_justfiles(&base, filter)?;
            if cli.json {
                let filtered = output::filter_justfiles_for_json(&justfiles, with.as_deref());
                println!("{}", serde_json::to_string_pretty(&filtered)?);
            } else {
                let rendered =
                    output::render_just_text(&justfiles, cli.verbose, with.as_deref(), *grouped);
                output::emit_text(&rendered, cli.plain);
            }
            return Ok(());
        }

        // Handle docs mode separately for split-stream output
        if let Commands::Docs { .. } = cmd {
            let base = cli
                .base
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));
            let docs_filter = cmd.docs_filter();

            // Discover repo root for detect_docs
            let repo = git2::Repository::discover(&base)
                .map_err(|e| format!("Not a git repository: {}", e))?;
            let repo_root = repo
                .workdir()
                .ok_or("Bare repository not supported")?
                .to_path_buf();

            let all_docs = sniff::filesystem::detect_docs(&repo_root).unwrap_or_default();
            let filtered = output::filter_docs(&all_docs, &docs_filter);

            if cli.json {
                let json_val = serde_json::to_value(&filtered).unwrap_or(serde_json::json!([]));
                println!("{}", serde_json::to_string_pretty(&json_val)?);
            } else {
                let text_output = output::render_docs_output(&filtered, cli.verbose);
                output::emit_stderr(&text_output.stderr, cli.plain);
                output::emit_text(&text_output.stdout, cli.plain);
            }
            return Ok(());
        }

        // Handle blast-radius mode separately (doesn't use SniffResult)
        if let Commands::BlastRadius {
            scope,
            package,
            package_area,
            list,
            csv,
            no_path,
            no_error,
            on_error,
        } = cmd
        {
            let base = cli
                .base
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));

            let change_scope = match scope {
                BlastRadiusScopeArg::Dirty => ChangeScope::Dirty,
                BlastRadiusScopeArg::Staged => ChangeScope::Staged,
                BlastRadiusScopeArg::LastCommit => ChangeScope::LastCommit,
            };

            let matched_docs = find_blast_radius_documents(
                &base,
                change_scope,
                package.as_deref(),
                package_area.as_deref(),
            )?;

            if matched_docs.is_empty() {
                return handle_no_results(*no_error, on_error, cli.plain);
            }

            if cli.json {
                let json_val = serde_json::json!({
                    "scope": change_scope,
                    "documents": matched_docs.iter().map(|d| &d.relative).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&json_val)?);
            } else {
                // Extract document paths and render
                let repo = git2::Repository::discover(&base)
                    .map_err(|e| format!("Not a git repository: {}", e))?;
                let repo_root = repo
                    .workdir()
                    .ok_or("Bare repository not supported")?
                    .to_path_buf();

                let doc_paths: Vec<std::path::PathBuf> = matched_docs
                    .iter()
                    .map(|d| std::path::PathBuf::from(&d.relative))
                    .collect();

                let format = if *list {
                    PathListFormat::BulletList
                } else if *csv {
                    PathListFormat::Csv
                } else {
                    PathListFormat::Lines
                };

                let rendered = output::render_path_list(&repo_root, &doc_paths, format, *no_path);
                output::emit_text(&rendered, cli.plain);
            }

            return Ok(());
        }

        // Handle services mode separately (doesn't use SniffResult)
        if let Some(state_arg) = cmd.state() {
            let services_info = detect_services();
            let state_filter = match state_arg {
                ServiceStateArg::All => ServiceState::All,
                ServiceStateArg::Running => ServiceState::Running,
                ServiceStateArg::Stopped => ServiceState::Stopped,
            };
            if cli.json {
                output::print_services_json(&services_info, state_filter)?;
            } else {
                output::emit_text(
                    &output::render_services_text(&services_info, cli.verbose, state_filter),
                    cli.plain,
                );
            }
            return Ok(());
        }
    }

    // Canonicalize path if provided
    let base_dir = cli
        .base
        .clone()
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p));

    // Normalize to RepoAction for unified dispatch
    let repo_action = cli.command.as_ref().and_then(|cmd| cmd.to_repo_action());

    // Handle RepoAction variants that don't need full detection as early returns
    if let Some(ref action) = repo_action {
        match action {
            crate::args::RepoAction::Remote { remote } => {
                if remote.contains("://") || remote.starts_with("git@") {
                    return handle_remote_url(remote, cli.json, cli.plain, cli.verbose).await;
                } else if is_owner_repo_shorthand(remote) {
                    return handle_shorthand(remote, cli.json, cli.plain, cli.verbose).await;
                } else {
                    let url =
                        resolve_remote_name(remote, base_dir.as_deref()).ok_or_else(|| {
                            format!(
                                "Could not find remote '{}' in the current repository",
                                remote
                            )
                        })?;
                    return handle_remote_url(&url, cli.json, cli.plain, cli.verbose).await;
                }
            }
            crate::args::RepoAction::Hash { sha } => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let repo = git2::Repository::discover(dir)
                    .map_err(|e| format!("Not a git repository: {}", e))?;
                let commit = sniff::filesystem::get_commit_by_sha(&repo, sha)
                    .ok_or_else(|| format!("Commit not found: {}", sha))?;
                let files = sniff::filesystem::get_commit_files(&repo, &commit.sha);

                if cli.json {
                    let json = serde_json::json!({
                        "commit": commit,
                        "files": files.iter().map(|(p, k)| serde_json::json!({
                            "path": p,
                            "kind": k,
                        })).collect::<Vec<_>>(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    let commit_url = commit_url_from_repo(&repo, &commit.sha);
                    output::emit_text(
                        &output::render_hash_section(
                            &commit,
                            &files,
                            cli.verbose,
                            commit_url.as_deref(),
                        ),
                        cli.plain,
                    );
                }
                return Ok(());
            }
            crate::args::RepoAction::UnstagedFiles { package: _ }
            | crate::args::RepoAction::UntrackedFiles { package: _ } => {
                let status_filter = match action {
                    crate::args::RepoAction::UnstagedFiles { .. } => {
                        sniff::filesystem::git::FileStatus::Modified
                    }
                    crate::args::RepoAction::UntrackedFiles { .. } => {
                        sniff::filesystem::git::FileStatus::Untracked
                    }
                    _ => unreachable!(),
                };
                // Use the existing file-list logic
                let mut plan = DetectionPlan::new()
                    .without_os()
                    .without_hardware()
                    .without_network();
                if let Some(ref base) = base_dir {
                    plan = plan.base_dir(base.clone());
                }
                let result = detect_with_plan(plan)?;
                if let Some(ref fs) = result.filesystem
                    && let Some(ref git) = fs.git
                {
                    let files: Vec<_> = git
                        .file_changes
                        .iter()
                        .filter(|f| match status_filter {
                            sniff::filesystem::git::FileStatus::Modified => {
                                f.status == sniff::filesystem::git::FileStatus::Modified
                                    || f.status == sniff::filesystem::git::FileStatus::Both
                            }
                            _ => f.status == status_filter,
                        })
                        .collect();
                    if files.is_empty() {
                        std::process::exit(1);
                    }
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&files)?);
                    } else {
                        output::emit_text(
                            &output::render_git_file_list(git, &status_filter, cli.verbose),
                            cli.plain,
                        );
                    }
                } else {
                    std::process::exit(1);
                }
                return Ok(());
            }
            crate::args::RepoAction::StagedFiles(args)
            | crate::args::RepoAction::DirtySourceCode(args)
            | crate::args::RepoAction::StagedSourceCode(args)
            | crate::args::RepoAction::UnstagedSourceCode(args)
            | crate::args::RepoAction::DirtyFiles(args) => {
                let (scope, kind) = match action {
                    crate::args::RepoAction::DirtySourceCode(_) => {
                        (ChangeScope::Dirty, ChangedPathKind::SourceCode)
                    }
                    crate::args::RepoAction::StagedSourceCode(_) => {
                        (ChangeScope::Staged, ChangedPathKind::SourceCode)
                    }
                    crate::args::RepoAction::UnstagedSourceCode(_) => {
                        (ChangeScope::Unstaged, ChangedPathKind::SourceCode)
                    }
                    crate::args::RepoAction::DirtyFiles(_) => {
                        (ChangeScope::Dirty, ChangedPathKind::AllFiles)
                    }
                    crate::args::RepoAction::StagedFiles(_) => {
                        (ChangeScope::Staged, ChangedPathKind::AllFiles)
                    }
                    _ => unreachable!(),
                };
                return handle_file_list_command(
                    args,
                    scope,
                    kind,
                    cli.json,
                    cli.plain,
                    base_dir.as_deref(),
                );
            }
            _ => {
                // Other RepoAction variants (Structure, GitStatus, Deps, etc.)
                // are handled after full detection below
            }
        }
    }

    let refresh_remotes_enabled = match &repo_action {
        Some(crate::args::RepoAction::GitStatus {
            refresh_remotes: true,
            ..
        }) => true,
        _ => cli
            .command
            .as_ref()
            .is_some_and(|command| command.refresh_remotes()),
    };

    // Set commit count from history flag
    let history_count = match &repo_action {
        Some(crate::args::RepoAction::GitStatus { history, .. }) => *history,
        _ => cli
            .command
            .as_ref()
            .map_or(DEFAULT_COMMIT_COUNT, |c| c.history()),
    };

    // Build git request based on deep/commit_count flags
    let git_request = if refresh_remotes_enabled {
        GitRequest::deep().commit_count(history_count)
    } else {
        GitRequest::full().commit_count(history_count)
    };

    // Build a targeted DetectionPlan based on the output filter
    let plan = match output_filter {
        OutputFilter::Os => DetectionPlan::new()
            .without_hardware()
            .without_network()
            .without_filesystem(),
        OutputFilter::Hardware => DetectionPlan::new()
            .without_os()
            .without_network()
            .without_filesystem(),
        OutputFilter::Network => DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_filesystem(),
        OutputFilter::Filesystem => DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(FilesystemRequest::new().git(git_request.clone())),
        OutputFilter::Cpu | OutputFilter::Memory => DetectionPlan::new()
            .without_os()
            .without_network()
            .without_filesystem()
            .hardware(HardwareRequest::summary()),
        OutputFilter::Gpu => DetectionPlan::new()
            .without_os()
            .without_network()
            .without_filesystem()
            .hardware(HardwareRequest::summary().include_gpu(true)),
        OutputFilter::Storage => DetectionPlan::new()
            .without_os()
            .without_network()
            .without_filesystem()
            .hardware(HardwareRequest::summary().include_storage(true)),
        OutputFilter::AudioDevices => DetectionPlan::new()
            .without_os()
            .without_network()
            .without_filesystem()
            .hardware(HardwareRequest::summary().include_audio(true)),
        OutputFilter::Repo => DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(git_request.clone())
                    .without_file_inventory(),
            ),
        OutputFilter::Language => DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(git_request.clone())
                    .without_docs()
                    .without_formatting(),
            ),
        OutputFilter::Files => DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(FilesystemRequest::new().git(git_request.clone())),
        OutputFilter::Docs => DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(git_request.clone())
                    .without_file_inventory()
                    .without_formatting(),
            ),
        OutputFilter::All => DetectionPlan::new()
            .filesystem(FilesystemRequest::new().git(git_request.clone())),
        OutputFilter::Programs
        | OutputFilter::Editors
        | OutputFilter::Utilities
        | OutputFilter::LanguagePackageManagers
        | OutputFilter::OsPackageManagers
        | OutputFilter::TtsClients
        | OutputFilter::TerminalApps
        | OutputFilter::HeadlessAudio
        | OutputFilter::AiClients
        | OutputFilter::Services
        | OutputFilter::Just
        | OutputFilter::BlastRadius => {
            unreachable!(
                "Programs, Services, Just, and BlastRadius mode should be handled before this point"
            )
        }
    };

    let plan = if let Some(ref base) = base_dir {
        plan.base_dir(base.clone())
    } else {
        plan
    };

    let mut result = detect_with_plan(plan)?;

    // Handle package scoping for git actions
    let package_for_git = match &repo_action {
        Some(crate::args::RepoAction::GitStatus { package, .. }) => package.clone(),
        _ => None,
    };
    if let Some(pkg_name) = &package_for_git {
        let path_prefix = resolve_package_path(&result, pkg_name)?;

        if let Some(ref mut filesystem) = result.filesystem
            && filesystem.git.is_some()
        {
            let dir = base_dir
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new("."));
            if let Ok(repo) = git2::Repository::discover(dir) {
                let scoped_commits =
                    sniff::filesystem::get_commits_for_path(&repo, &path_prefix, history_count);
                if let Some(ref mut git) = filesystem.git {
                    git.recent = scoped_commits;

                    // Filter file_changes to the package path
                    git.file_changes
                        .retain(|f| f.path.to_string_lossy().starts_with(&path_prefix));

                    // Filter dirty files
                    git.status
                        .dirty
                        .retain(|f| f.filepath.to_string_lossy().starts_with(&path_prefix));

                    // Filter untracked files
                    git.status
                        .untracked
                        .retain(|f| f.filepath.to_string_lossy().starts_with(&path_prefix));

                    // Update counts to match filtered lists
                    git.status.staged_count = git
                        .file_changes
                        .iter()
                        .filter(|f| {
                            f.status == sniff::filesystem::git::FileStatus::Staged
                                || f.status == sniff::filesystem::git::FileStatus::Both
                        })
                        .count();
                    git.status.unstaged_count = git
                        .file_changes
                        .iter()
                        .filter(|f| {
                            f.status == sniff::filesystem::git::FileStatus::Modified
                                || f.status == sniff::filesystem::git::FileStatus::Both
                        })
                        .count();
                    git.status.untracked_count = git
                        .file_changes
                        .iter()
                        .filter(|f| f.status == sniff::filesystem::git::FileStatus::Untracked)
                        .count();
                    git.status.is_dirty = git.status.staged_count > 0
                        || git.status.unstaged_count > 0
                        || git.status.untracked_count > 0;
                }
            }
        }
    }

    let latest_versions_enabled = match &repo_action {
        Some(crate::args::RepoAction::Structure {
            latest_versions: true,
            ..
        }) => true,
        _ => cli
            .command
            .as_ref()
            .is_some_and(|command| command.latest_versions()),
    };

    if latest_versions_enabled {
        result = enrich_result_dependencies(result).await;
    }

    let docs_filter = cli
        .command
        .as_ref()
        .map_or(DocsFilter::default(), |c| c.docs_filter());
    let files_filter = cli
        .command
        .as_ref()
        .map_or(FilesFilter::default(), |c| c.files_filter());

    // Output logic:
    // - No subcommand + --json: full JSON output (help already handled above)
    // - With subcommand: text by default, --json for JSON
    let use_json = cli.command.is_none() || cli.json;

    if use_json {
        output::print_json(&result, output_filter, &docs_filter, &files_filter)?;
    } else {
        let text = output::render_text(
            &result,
            cli.verbose,
            output_filter,
            history_count,
            &docs_filter,
            &files_filter,
            repo_action.as_ref(),
            base_dir.as_deref(),
            latest_versions_enabled,
        );
        output::emit_text(&text, cli.plain);
    }

    Ok(())
}

/// Prints shell completions setup instructions.
///
/// With dynamic completions, the shell sources a command that calls back to the CLI.
/// This outputs the appropriate setup command for each shell.
fn print_completions(shell: Shell) {
    let (setup_cmd, config_file) = match shell {
        Shell::Bash => ("source <(COMPLETE=bash sniff)", "~/.bashrc"),
        Shell::Zsh => ("source <(COMPLETE=zsh sniff)", "~/.zshrc"),
        Shell::Fish => ("COMPLETE=fish sniff | source", "~/.config/fish/config.fish"),
        Shell::PowerShell => (
            r#"$env:COMPLETE = "powershell"; sniff | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE"#,
            "$PROFILE",
        ),
        Shell::Elvish => ("eval (E:COMPLETE=elvish sniff | slurp)", "~/.elvish/rc.elv"),
        _ => {
            eprintln!("Shell {:?} is not supported for dynamic completions", shell);
            return;
        }
    };

    println!("# Add this line to {}:", config_file);
    println!("{}", setup_cmd);
}

fn wants_completions_help() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    wants_completions_help_with_args(&args)
}

fn wants_completions_help_with_args(args: &[String]) -> bool {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--completions"
            && let Some(next) = args.get(index + 1)
        {
            return next == "--help" || next == "-h";
        }
    }

    false
}

fn print_completions_help() {
    println!("{}", COMPLETIONS_HELP);
}

/// Handle `owner/repo` shorthand by probing configured providers.
async fn handle_shorthand(
    shorthand: &str,
    json: bool,
    plain: bool,
    verbose: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, repo) = shorthand.split_once('/').expect("already validated");
    let remote = GitRemote::from_shorthand(owner, repo).await?;
    let report = remote.fetch_report(owner, repo).await?;

    if json {
        output::print_remote_json(&report)?;
    } else {
        let readme = fetch_readme(&report, &remote, owner, repo, verbose).await;
        let rendered = output::render_remote_text(&report, readme.as_deref());
        output::emit_text(&rendered, plain);
    }

    Ok(())
}

/// Handle remote URL inspection (from `sniff git <remote>`).
///
/// Parses the URL, detects the provider, fetches the report, and outputs it.
async fn handle_remote_url(
    url: &str,
    json: bool,
    plain: bool,
    verbose: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let remote = GitRemote::from_url(url)?;
    let parsed = GitRemote::parse_url(url)?;
    let report = remote.fetch_report(&parsed.owner, &parsed.repo).await?;

    if json {
        output::print_remote_json(&report)?;
    } else {
        let readme = fetch_readme(&report, &remote, &parsed.owner, &parsed.repo, verbose).await;
        let rendered = output::render_remote_text(&report, readme.as_deref());
        output::emit_text(&rendered, plain);
    }

    Ok(())
}

/// Fetch the README content when verbose mode is enabled.
///
/// Looks for the first `DocumentCategory::Readme` entry in the report's
/// documents and fetches its content via the provider API.
async fn fetch_readme(
    report: &RemoteReport,
    remote: &GitRemote,
    owner: &str,
    repo: &str,
    verbose: u8,
) -> Option<String> {
    if verbose == 0 {
        return None;
    }
    let readme_path = report
        .documents
        .iter()
        .find(|d| d.category == DocumentCategory::Readme)
        .map(|d| d.path.as_str())?;
    remote.get_file_content(owner, repo, readme_path).await.ok()
}

/// Resolve a remote name to a URL by looking it up in the local git repository.
///
/// Returns `None` if not in a git repo or if the remote doesn't exist.
fn resolve_remote_name(name: &str, base_dir: Option<&std::path::Path>) -> Option<String> {
    let dir = base_dir.unwrap_or_else(|| std::path::Path::new("."));
    let repo = git2::Repository::discover(dir).ok()?;
    let remote = repo.find_remote(name).ok()?;
    remote.url().map(String::from)
}

/// Build a commit URL from a `git2::Repository` by reading the origin remote.
fn commit_url_from_repo(repo: &git2::Repository, sha: &str) -> Option<String> {
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url()?;
    let provider = sniff::filesystem::git::GitHostingProvider::from_url(url);
    let base = provider.browser_base_url()?;

    // Extract owner/repo from URL
    let owner_repo = if url.contains('@') && url.contains(':') {
        url.split(':')
            .next_back()
            .map(|s| s.trim_end_matches(".git").to_string())
    } else if url.contains("://") {
        let path = url.split('/').skip(3).collect::<Vec<_>>().join("/");
        Some(path.trim_end_matches(".git").to_string())
    } else {
        None
    }?;

    Some(format!(
        "{base}/{owner_repo}/{}/{sha}",
        provider.commit_path_segment()
    ))
}

/// Enriches all dependencies in a SniffResult with latest versions from package registries.
///
/// Collects all dependencies across all packages, deduplicates by (name, package_manager),
/// enriches in parallel with bounded concurrency, then distributes results back.
async fn enrich_result_dependencies(mut result: SniffResult) -> SniffResult {
    use futures::stream::{self, StreamExt};
    use std::collections::HashMap;

    let Some(ref mut filesystem) = result.filesystem else {
        return result;
    };

    let Some(ref mut repo) = filesystem.repo else {
        return result;
    };

    let mut unique_deps: HashMap<(String, Option<String>), sniff::filesystem::DependencyEntry> =
        HashMap::new();

    let mut collect = |deps: &Option<Vec<sniff::filesystem::DependencyEntry>>| {
        if let Some(dep_list) = deps {
            for dep in dep_list {
                let key = (dep.name.clone(), dep.package_manager.clone());
                unique_deps.entry(key).or_insert_with(|| dep.clone());
            }
        }
    };

    collect(&repo.dependencies);
    collect(&repo.dev_dependencies);
    collect(&repo.peer_dependencies);
    collect(&repo.optional_dependencies);

    if let Some(ref packages) = repo.packages {
        for pkg in packages {
            collect(&pkg.dependencies);
            collect(&pkg.dev_dependencies);
            collect(&pkg.peer_dependencies);
            collect(&pkg.optional_dependencies);
        }
    }

    let unique_entries: Vec<_> = unique_deps.into_values().collect();
    let enriched: Vec<sniff::filesystem::DependencyEntry> = stream::iter(unique_entries)
        .map(|dep| async move { enrich_dependency(dep).await })
        .buffer_unordered(20)
        .collect()
        .await;

    let lookup: HashMap<(String, Option<String>), Option<String>> = enriched
        .into_iter()
        .map(|dep| {
            let key = (dep.name.clone(), dep.package_manager.clone());
            (key, dep.latest_version)
        })
        .collect();

    let apply = |deps: &mut Option<Vec<sniff::filesystem::DependencyEntry>>| {
        if let Some(dep_list) = deps {
            for dep in dep_list.iter_mut() {
                let key = (dep.name.clone(), dep.package_manager.clone());
                if let Some(latest) = lookup.get(&key).and_then(|v| v.clone()) {
                    dep.latest_version = Some(latest.clone());
                    if let Some(ref actual) = dep.actual_version {
                        dep.is_updatable = *actual != latest;
                        if dep.is_updatable {
                            dep.has_major_update = is_major_update(actual, &latest);
                        }
                    }
                }
            }
        }
    };

    apply(&mut repo.dependencies);
    apply(&mut repo.dev_dependencies);
    apply(&mut repo.peer_dependencies);
    apply(&mut repo.optional_dependencies);

    if let Some(ref mut packages) = repo.packages {
        for pkg in packages.iter_mut() {
            apply(&mut pkg.dependencies);
            apply(&mut pkg.dev_dependencies);
            apply(&mut pkg.peer_dependencies);
            apply(&mut pkg.optional_dependencies);

            let all_deps = [
                &pkg.dependencies,
                &pkg.dev_dependencies,
                &pkg.peer_dependencies,
                &pkg.optional_dependencies,
            ];

            let any_updatable = all_deps
                .iter()
                .filter_map(|deps| deps.as_ref())
                .flat_map(|deps| deps.iter())
                .any(|d| d.is_updatable);

            let any_major = all_deps
                .iter()
                .filter_map(|deps| deps.as_ref())
                .flat_map(|deps| deps.iter())
                .any(|d| d.has_major_update);

            pkg.is_updatable = Some(any_updatable);
            pkg.has_major_update = Some(any_major);
        }
    }

    result
}

/// Resolve a package name to its path prefix for git filtering.
///
/// Tries exact match on `Package.name` first, then falls back to `Package.package_area`.
/// Returns the relative path prefix (e.g., "homelab/server" or "homelab").
fn resolve_package_path(
    result: &SniffResult,
    pkg_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let packages = result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.repo.as_ref())
        .and_then(|repo| repo.packages.as_ref());

    let Some(packages) = packages else {
        return Err("No packages found in this repository".into());
    };

    let lower = pkg_name.to_lowercase();

    // Try exact match on package name
    if let Some(pkg) = packages.iter().find(|p| p.name.to_lowercase() == lower) {
        return Ok(format!("{}/", pkg.relative));
    }

    // Try match on package_area
    if let Some(pkg) = packages
        .iter()
        .find(|p| p.package_area.to_lowercase() == lower)
    {
        return Ok(format!("{}/", pkg.package_area));
    }

    // No match — list valid options
    let mut names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    names.sort();
    names.dedup();

    let mut areas: Vec<&str> = packages.iter().map(|p| p.package_area.as_str()).collect();
    areas.sort();
    areas.dedup();

    Err(format!(
        "Package '{}' not found.\n\nValid package names: {}\nValid package areas: {}",
        pkg_name,
        names.join(", "),
        areas.join(", ")
    )
    .into())
}

/// Resolve `FileListArgs` flags into a `PathListFormat`.
fn path_list_format(args: &FileListArgs) -> PathListFormat {
    if args.list {
        PathListFormat::BulletList
    } else if args.csv {
        PathListFormat::Csv
    } else {
        PathListFormat::Lines
    }
}

/// Handle the no-results exit behavior for file-list and blast-radius commands.
///
/// - Default: exit 1 with no output.
/// - `--no-error`: exit 0 with no output.
/// - `--on-error <msg>`: render message to stderr.
/// - `--on-error` + `--no-error`: render message to stdout, exit 0.
fn handle_no_results(
    no_error: bool,
    on_error: &Option<String>,
    plain: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable;
    use biscuit_terminal::terminal::Terminal;

    if let Some(msg) = on_error {
        let terminal = Terminal::default();
        let rendered = Prose::new(msg).render(&terminal);
        let text = if plain {
            biscuit_terminal::prelude::strip_escape_codes(&rendered)
        } else {
            rendered
        };
        if no_error {
            print!("{text}");
        } else {
            eprint!("{text}");
        }
    }

    if no_error {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

/// Handle `sniff repo dirty-source-code`, `staged-source-code`, `unstaged-source-code`,
/// and `dirty-files` commands.
fn handle_file_list_command(
    args: &FileListArgs,
    scope: ChangeScope,
    kind: ChangedPathKind,
    json: bool,
    plain: bool,
    base_dir: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base_dir.unwrap_or_else(|| std::path::Path::new("."));
    let query = ChangedPathQuery {
        scope,
        kind,
        package: args.package.clone(),
        package_area: args.package_area.clone(),
        filters: args.filter.clone(),
    };

    let result = collect_changed_paths(dir, &query)?;

    if result.paths.is_empty() {
        return handle_no_results(args.no_error, &args.on_error, plain);
    }

    if json {
        let json_val = serde_json::json!({
            "scope": scope,
            "kind": kind,
            "paths": result.paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json_val)?);
    } else {
        let format = path_list_format(args);
        let rendered =
            output::render_path_list(&result.repo_root, &result.paths, format, args.no_path);
        output::emit_text(&rendered, plain);
    }

    Ok(())
}

/// Detect only the program category needed for the given output filter.
///
/// Builds the shared executable index once, then detects only the requested
/// category instead of all 8. Falls back to full detection for `OutputFilter::Programs`.
fn detect_programs_for_filter(filter: OutputFilter) -> ProgramsInfo {
    use sniff::programs::*;
    use std::sync::Arc;

    if filter == OutputFilter::Programs {
        return ProgramsInfo::detect();
    }

    let index = Arc::new(ExecutableIndex::build());
    let mut programs = ProgramsInfo::default();
    match filter {
        OutputFilter::Editors => {
            programs.editors = InstalledEditors::new_with_index(&index);
        }
        OutputFilter::Utilities => {
            programs.utilities = InstalledUtilities::new_with_index(&index);
        }
        OutputFilter::LanguagePackageManagers => {
            programs.language_package_managers =
                InstalledLanguagePackageManagers::new_with_index(&index);
        }
        OutputFilter::OsPackageManagers => {
            programs.os_package_managers = InstalledOsPackageManagers::new_with_index(&index);
        }
        OutputFilter::TtsClients => {
            programs.tts_clients = InstalledTtsClients::new_with_index(&index);
        }
        OutputFilter::TerminalApps => {
            programs.terminal_apps = InstalledTerminalApps::new_with_index(&index);
        }
        OutputFilter::HeadlessAudio => {
            programs.headless_audio = InstalledHeadlessAudio::new_with_index(&index);
        }
        OutputFilter::AiClients => {
            programs.ai_clients = InstalledAiClients::new_with_index(&index);
        }
        _ => return ProgramsInfo::detect(),
    }
    programs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_help_detection() {
        assert!(wants_completions_help_with_args(&[
            "--completions".to_string(),
            "--help".to_string(),
        ]));
        assert!(wants_completions_help_with_args(&[
            "--completions".to_string(),
            "-h".to_string(),
        ]));
        assert!(!wants_completions_help_with_args(&[
            "--completions".to_string(),
            "zsh".to_string(),
        ]));
    }
}
