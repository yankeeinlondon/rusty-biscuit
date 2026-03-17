use clap::{CommandFactory, Parser};
use clap_complete::{CompleteEnv, Shell};
use sniff::package::{enrich_dependency, is_major_update, is_owner_repo_shorthand};
use sniff::programs::ProgramsInfo;
use sniff::remote::{DocumentCategory, GitRemote, RemoteRepoProvider, RemoteReport};
use sniff::services::{ServiceState, detect_services};
use sniff::{SniffConfig, SniffResult, detect_with_config};

use crate::args::{
    COMPLETIONS_HELP, Cli, Commands, DEFAULT_COMMIT_COUNT, DocsFilter, FilesFilter, ServiceStateArg,
};
use crate::output::{self, OutputFilter};

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

            let programs = ProgramsInfo::detect();
            if cli.json {
                output::print_programs_json(&programs, output_filter)?;
            } else {
                let rendered =
                    output::render_programs_markdown(&programs, cli.verbose, output_filter);
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

    // Emit deprecation warning for `sniff git` (text mode only)
    if matches!(cli.command, Some(Commands::Git { .. })) && !cli.json {
        eprintln!("note: 'sniff git' is deprecated, use 'sniff repo' subcommands instead");
    }

    // Normalize to RepoAction for unified dispatch
    let repo_action = cli
        .command
        .as_ref()
        .and_then(|cmd| cmd.to_repo_action().or_else(|| cmd.git_to_repo_action()));

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
            crate::args::RepoAction::StagedFiles { package: _ }
            | crate::args::RepoAction::UnstagedFiles { package: _ }
            | crate::args::RepoAction::UntrackedFiles { package: _ } => {
                let status_filter = match action {
                    crate::args::RepoAction::StagedFiles { .. } => {
                        sniff::filesystem::git::FileStatus::Staged
                    }
                    crate::args::RepoAction::UnstagedFiles { .. } => {
                        sniff::filesystem::git::FileStatus::Modified
                    }
                    crate::args::RepoAction::UntrackedFiles { .. } => {
                        sniff::filesystem::git::FileStatus::Untracked
                    }
                    _ => unreachable!(),
                };
                // Use the existing file-list logic
                let mut config = SniffConfig::new().skip_os().skip_hardware().skip_network();
                if let Some(ref base) = base_dir {
                    config = config.base_dir(base.clone());
                }
                let result = sniff::detect_with_config(config)?;
                if let Some(ref fs) = result.filesystem
                    && let Some(ref git) = fs.git
                {
                    if cli.json {
                        let files: Vec<_> = git
                            .file_changes
                            .iter()
                            .filter(|f| match status_filter {
                                sniff::filesystem::git::FileStatus::Staged => {
                                    f.status == sniff::filesystem::git::FileStatus::Staged
                                        || f.status == sniff::filesystem::git::FileStatus::Both
                                }
                                sniff::filesystem::git::FileStatus::Modified => {
                                    f.status == sniff::filesystem::git::FileStatus::Modified
                                        || f.status == sniff::filesystem::git::FileStatus::Both
                                }
                                _ => f.status == status_filter,
                            })
                            .collect();
                        println!("{}", serde_json::to_string_pretty(&files)?);
                    } else {
                        output::emit_text(
                            &output::render_git_file_list(git, &status_filter, cli.verbose),
                            cli.plain,
                        );
                    }
                }
                return Ok(());
            }
            _ => {
                // Other RepoAction variants (Structure, GitStatus, Deps, etc.)
                // are handled after full detection below
            }
        }
    }

    let mut config = SniffConfig::new();

    if let Some(ref base) = base_dir {
        config = config.base_dir(base.clone());
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
    if refresh_remotes_enabled {
        config = config.deep(true);
    }

    // Set commit count from history flag
    let history_count = match &repo_action {
        Some(crate::args::RepoAction::GitStatus { history, .. }) => *history,
        _ => cli
            .command
            .as_ref()
            .map_or(DEFAULT_COMMIT_COUNT, |c| c.history()),
    };
    config = config.commit_count(history_count);

    // Apply skip logic based on filter mode
    match output_filter {
        OutputFilter::Os => {
            config = config.skip_hardware().skip_network().skip_filesystem();
        }
        OutputFilter::Hardware => {
            config = config.skip_os().skip_network().skip_filesystem();
        }
        OutputFilter::Network => {
            config = config.skip_os().skip_hardware().skip_filesystem();
        }
        OutputFilter::Filesystem => {
            config = config.skip_os().skip_hardware().skip_network();
        }
        OutputFilter::Cpu
        | OutputFilter::Gpu
        | OutputFilter::Memory
        | OutputFilter::Storage
        | OutputFilter::AudioDevices => {
            config = config.skip_os().skip_network().skip_filesystem();
        }
        OutputFilter::Git
        | OutputFilter::Repo
        | OutputFilter::Language
        | OutputFilter::Files
        | OutputFilter::Docs => {
            config = config.skip_os().skip_hardware().skip_network();
        }
        OutputFilter::All => {
            // No filtering - detect everything
        }
        OutputFilter::Programs
        | OutputFilter::Editors
        | OutputFilter::Utilities
        | OutputFilter::LanguagePackageManagers
        | OutputFilter::OsPackageManagers
        | OutputFilter::TtsClients
        | OutputFilter::TerminalApps
        | OutputFilter::HeadlessAudio
        | OutputFilter::AiClients
        | OutputFilter::Services => {
            unreachable!("Programs and Services mode should be handled before this point")
        }
    }

    let mut result = detect_with_config(config)?;

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
