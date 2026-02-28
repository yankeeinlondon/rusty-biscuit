use clap::{CommandFactory, Parser};
use clap_complete::{CompleteEnv, Shell};
use sniff::package::{enrich_dependency, is_major_update, is_owner_repo_shorthand};
use sniff::programs::ProgramsInfo;
use sniff::remote::{DocumentCategory, GitRemote, RemoteRepoProvider, RemoteReport};
use sniff::services::{ServiceState, detect_services};
use sniff::{SniffConfig, SniffResult, detect_with_config};

use crate::args::{COMPLETIONS_HELP, Cli, Commands, DEFAULT_COMMIT_COUNT, DocsFilter, ServiceStateArg};
use crate::output::{self, OutputFilter};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Handle dynamic shell completions (invoked by shell completion scripts)
    CompleteEnv::with_factory(Cli::command).complete();

    if wants_completions_help() {
        print_completions_help();
        return Ok(());
    }

    let cli = Cli::parse();

    // Handle --completions first (prints setup instructions)
    if let Some(shell) = cli.completions {
        print_completions(shell);
        return Ok(());
    }

    if matches!(cli.command, Some(Commands::Topics)) {
        output::print_topics_table();
        return Ok(());
    }

    if matches!(cli.command, Some(Commands::Structure)) {
        output::print_structure();
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
                output::print_programs_markdown(&programs, cli.verbose, output_filter);
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
                output::print_services_text(&services_info, cli.verbose, state_filter);
            }
            return Ok(());
        }
    }

    // Canonicalize path if provided
    let base_dir = cli
        .base
        .clone()
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p));

    // Handle `sniff git <remote>` - resolve name/URL/shorthand, then fetch remote info
    if let Some(ref cmd) = cli.command
        && let Some(remote_ref) = cmd.git_remote()
    {
        if remote_ref.contains("://") || remote_ref.starts_with("git@") {
            // URL: contains :// or starts with git@
            return handle_remote_url(remote_ref, cli.json, cli.verbose).await;
        } else if is_owner_repo_shorthand(remote_ref) {
            // owner/repo shorthand: exactly one slash with non-empty parts
            return handle_shorthand(remote_ref, cli.json, cli.verbose).await;
        } else {
            // Git remote name (e.g., "origin")
            let url =
                resolve_remote_name(remote_ref, base_dir.as_deref()).ok_or_else(|| {
                    format!(
                        "Could not find remote '{}' in the current repository",
                        remote_ref
                    )
                })?;
            return handle_remote_url(&url, cli.json, cli.verbose).await;
        }
    }

    let mut config = SniffConfig::new();

    if let Some(base) = base_dir {
        config = config.base_dir(base);
    }

    let deep_enabled = cli.deep;
    if deep_enabled {
        config = config.deep(true);
    }

    // Set commit count from history flag
    let history_count = cli.command.as_ref().map_or(DEFAULT_COMMIT_COUNT, |c| c.history());
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
        OutputFilter::Git | OutputFilter::Repo | OutputFilter::Language | OutputFilter::Docs => {
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

    // Enrich dependencies with latest versions when --deep is enabled.
    // Only for subcommands that display dependency info (repo, language, all).
    if deep_enabled
        && matches!(
            output_filter,
            OutputFilter::All
                | OutputFilter::Filesystem
                | OutputFilter::Repo
                | OutputFilter::Language
        )
    {
        result = enrich_result_dependencies(result).await;
    }

    let docs_filter = cli
        .command
        .as_ref()
        .map_or(DocsFilter::default(), |c| c.docs_filter());

    let deps = cli.command.as_ref().is_some_and(|c| c.deps());
    let packages = cli.command.as_ref().is_some_and(|c| c.packages());
    let ui = cli.command.as_ref().is_some_and(|c| c.ui());
    let repo_filter = cli.command.as_ref().and_then(|c| c.repo_filter());

    // Output logic:
    // - No subcommand: always JSON
    // - With subcommand: text by default, --json for JSON
    let use_json = cli.command.is_none() || cli.json;

    if use_json {
        output::print_json(&result, output_filter, &docs_filter)?;
    } else {
        output::print_text(
            &result,
            cli.verbose,
            output_filter,
            history_count,
            &docs_filter,
            deps,
            packages,
            ui,
            repo_filter,
        );
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
    verbose: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, repo) = shorthand.split_once('/').expect("already validated");
    let remote = GitRemote::from_shorthand(owner, repo).await?;
    let report = remote.fetch_report(owner, repo).await?;

    if json {
        output::print_remote_json(&report)?;
    } else {
        let readme = fetch_readme(&report, &remote, owner, repo, verbose).await;
        output::print_remote_text(&report, readme.as_deref());
    }

    Ok(())
}

/// Handle remote URL inspection (from `sniff git <remote>`).
///
/// Parses the URL, detects the provider, fetches the report, and outputs it.
async fn handle_remote_url(
    url: &str,
    json: bool,
    verbose: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let remote = GitRemote::from_url(url)?;
    let parsed = GitRemote::parse_url(url)?;
    let report = remote.fetch_report(&parsed.owner, &parsed.repo).await?;

    if json {
        output::print_remote_json(&report)?;
    } else {
        let readme = fetch_readme(&report, &remote, &parsed.owner, &parsed.repo, verbose).await;
        output::print_remote_text(&report, readme.as_deref());
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
