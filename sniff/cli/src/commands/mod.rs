use clap::{CommandFactory, Parser};
use clap_complete::{CompleteEnv, Shell};
use sniff::filesystem::blast_radius::{
    ChangeScope, ChangedPathKind, find_blast_radius_documents,
};
use sniff::package::{enrich_dependency, is_major_update, is_owner_repo_shorthand};
use sniff::programs::ProgramsInfo;

use sniff::request::*;
use sniff::services::{ServiceState, detect_services};
use sniff::{SniffResult, detect_with_plan};
use tracing::info_span;

use crate::args::{
    BlastRadiusScopeArg, COMPLETIONS_HELP, Cli, Commands, DEFAULT_COMMIT_COUNT, DocsFilter,
    FilesFilter, ServiceStateArg,
};
use crate::output::{self, OutputFilter, PathListFormat};
use crate::perf::{CliPerf, handle_no_results};

mod repo;
mod remote;

use repo::{
    RepoPackageAreasArgs, RepoPackagesArgs, handle_file_list_command,
    handle_repo_package_areas, handle_repo_packages,
};
use remote::{
    commit_url_from_repo, handle_pr_command, handle_remote_url,
    handle_shorthand, resolve_remote_name,
};

/// Main CLI entrypoint.
///
/// ## Async model
///
/// The CLI is a single-shot command runner: parse args, run detection, print
/// output, and exit.  Most code paths are synchronous (git, filesystem,
/// subprocess).  The `async` signature exists because a minority of paths
/// (remote provider calls, dependency enrichment) use async HTTP.  There is
/// no concurrent async work that would justify `spawn_blocking` for the
/// synchronous parts — adding it would only add scheduling overhead without
/// improving throughput.  If future modes add progress UI, cancellation, or
/// true concurrency, `spawn_blocking` should be introduced selectively for
/// paths that mix blocking local work with async network awaits.
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

    crate::init_tracing(cli.debug);

    let _root = info_span!("sniff",
        command = ?cli.command.as_ref().map(|c| format!("{c:?}")),
        json = cli.json,
        plain = cli.plain,
        perf = cli.perf,
    )
    .entered();

    // Handle --completions first (prints setup instructions)
    if let Some(shell) = cli.completions {
        print_completions(shell);
        return Ok(());
    }

    let perf = CliPerf::new(cli.perf, cli.plain);

    // No subcommand and no --json flag: show help
    if cli.command.is_none() && !cli.json {
        Cli::command().print_help()?;
        return Ok(());
    }

    if matches!(cli.command, Some(Commands::Topics)) {
        output::emit_text(&output::render_topics_table(), cli.plain);
        perf.emit_stdout(None);
        return Ok(());
    }

    let output_filter = cli
        .command
        .as_ref()
        .map(Commands::to_output_filter)
        .unwrap_or(OutputFilter::All);

    // Handle programs mode separately (doesn't use SniffResult)
    if let Some(ref cmd) = cli.command {
        // Handle notification-helpers before generic programs mode
        if matches!(cmd, Commands::NotificationHelpers) {
            let programs = sniff::programs::ProgramsInfo::detect();
            if cli.json {
                output::print_notification_helpers_json(
                    &programs.notification_helpers,
                    perf.build_report().as_ref(),
                )?;
            } else {
                let rendered = output::render_notification_helpers_markdown(
                    &programs.notification_helpers,
                    cli.verbose,
                );
                output::emit_text(&rendered, cli.plain);
                perf.emit_stdout(None);
            }
            return Ok(());
        }

        if cmd.is_programs_mode() {
            // Plan-aware install or install-plan
            if let Some((kind, args)) = cmd.install_command_args() {
                if args.program.is_some() {
                    let filter = cmd.to_output_filter();
                    return crate::install_plan_cmd::dispatch(
                        kind,
                        args,
                        filter,
                        cli.json,
                        cli.plain,
                        perf.build_report().as_ref(),
                    );
                }
                // No name: fall through to interactive (Install) or error (InstallPlan)
                if kind == crate::args::InstallCommandKind::InstallPlan {
                    return Err("install-plan requires a program name".into());
                }
                let filter = cmd.to_output_filter();
                return crate::install::interactive_install(filter);
            }

            let programs = detect_programs_for_filter(output_filter);
            if cli.json {
                let json = output::build_programs_json(&programs, output_filter)?;
                output::print_json_value(json, perf.build_report().as_ref());
            } else {
                let rendered =
                    output::render_programs_markdown(&programs, cli.verbose, output_filter);
                output::emit_text(&rendered, cli.plain);
                perf.emit_stdout(None);
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
                let json_val = serde_json::to_value(&filtered)?;
                output::print_json_value(json_val, perf.build_report().as_ref());
            } else {
                let rendered =
                    output::render_just_text(&justfiles, cli.verbose, with.as_deref(), *grouped);
                output::emit_text(&rendered, cli.plain);
            }
            perf.emit_stdout(None);
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
                output::print_json_value(json_val, perf.build_report().as_ref());
            } else {
                let text_output = output::render_docs_output(&filtered, cli.verbose);
                output::emit_stderr(&text_output.stderr, cli.plain);
                output::emit_text(&text_output.stdout, cli.plain);
            }
            perf.emit_stdout(None);
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
                return handle_no_results(*no_error, on_error, cli.plain, &perf);
            }

            if cli.json {
                let json_val = serde_json::json!({
                    "scope": change_scope,
                    "documents": matched_docs.iter().map(|d| &d.relative).collect::<Vec<_>>(),
                });
                output::print_json_value(json_val, perf.build_report().as_ref());
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
                perf.emit_stderr(None);
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
                output::print_services_json(
                    &services_info,
                    state_filter,
                    perf.build_report().as_ref(),
                )?;
            } else {
                output::emit_text(
                    &output::render_services_text(&services_info, cli.verbose, state_filter),
                    cli.plain,
                );
            }
            perf.emit_stdout(None);
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
                    return handle_remote_url(remote, cli.json, cli.plain, cli.verbose, &perf)
                        .await;
                } else if is_owner_repo_shorthand(remote) {
                    return handle_shorthand(remote, cli.json, cli.plain, cli.verbose, &perf).await;
                } else {
                    let url =
                        resolve_remote_name(remote, base_dir.as_deref()).ok_or_else(|| {
                            format!(
                                "Could not find remote '{}' in the current repository",
                                remote
                            )
                        })?;
                    return handle_remote_url(&url, cli.json, cli.plain, cli.verbose, &perf,
                    ).await;
                }
            }
            crate::args::RepoAction::Pr { status, .. } => {
                let result = handle_pr_command(
                    *status,
                    cli.json,
                    cli.plain,
                    cli.verbose,
                    base_dir.as_deref(),
                    &perf,
                )
                .await;
                perf.emit_stdout(None);
                return result;
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
                    output::print_json_value(json, perf.build_report().as_ref());
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
                    perf.emit_for_json(None);
                }
                return Ok(());
            }
            crate::args::RepoAction::UnstagedFiles { package } => {
                let args = crate::args::FileListArgs {
                    package: package.clone(),
                    package_area: None,
                    list: false,
                    csv: false,
                    no_path: false,
                    no_error: false,
                    on_error: None,
                    filter: Vec::new(),
                };
                return handle_file_list_command(
                    &args,
                    ChangeScope::Unstaged,
                    ChangedPathKind::AllFiles,
                    cli.json,
                    cli.plain,
                    base_dir.as_deref(),
                    &perf,
                );
            }
            crate::args::RepoAction::UntrackedFiles { package } => {
                let args = crate::args::FileListArgs {
                    package: package.clone(),
                    package_area: None,
                    list: false,
                    csv: false,
                    no_path: false,
                    no_error: false,
                    on_error: None,
                    filter: Vec::new(),
                };
                return handle_file_list_command(
                    &args,
                    ChangeScope::Untracked,
                    ChangedPathKind::AllFiles,
                    cli.json,
                    cli.plain,
                    base_dir.as_deref(),
                    &perf,
                );
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
                    &perf,
                );
            }
            crate::args::RepoAction::Root => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let repo = match git2::Repository::discover(dir) {
                    Ok(r) => r,
                    Err(e) => {
                        if cli.json {
                            let json = serde_json::json!({ "root": "" });
                            output::print_json_value(json, perf.build_report().as_ref());
                            std::process::exit(1);
                        }
                        return Err(format!("Not a git repository: {}", e).into());
                    }
                };
                let Some(workdir) = repo.workdir() else {
                    if cli.json {
                        let json = serde_json::json!({ "root": "" });
                        output::print_json_value(json, perf.build_report().as_ref());
                        std::process::exit(1);
                    }
                    perf.emit_stderr(None);
                    std::process::exit(1);
                };
                if cli.json {
                    let json = serde_json::json!({
                        "root": workdir.display().to_string(),
                    });
                    output::print_json_value(json, perf.build_report().as_ref());
                } else {
                    println!("{}", workdir.display());
                    perf.emit_stderr(None);
                }
                return Ok(());
            }
            crate::args::RepoAction::HasMergeConflict => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let repo = git2::Repository::discover(dir)
                    .map_err(|e| format!("Not a git repository: {}", e))?;
                let conflicted = sniff::filesystem::git::detect_merge_conflicts(&repo);
                let has_conflict = !conflicted.is_empty();
                // Verbose conflict listing goes to stderr in BOTH json and
                // text modes — it's diagnostic, not part of the contract
                // shape consumers parse.
                if cli.verbose > 0 {
                    for path in &conflicted {
                        eprintln!("{}", path.display());
                    }
                }
                if cli.json {
                    let outcome = output::repo_json::has_merge_conflict_outcome(has_conflict);
                    output::print_json_value(outcome.value, perf.build_report().as_ref());
                    std::process::exit(outcome.exit_code.unwrap_or(0));
                }
                perf.emit_stderr(None);
                if has_conflict {
                    std::process::exit(0);
                }
                std::process::exit(1);
            }
            crate::args::RepoAction::RecentCommits { .. }
            | crate::args::RepoAction::SourceCodeChanges { .. }
            | crate::args::RepoAction::DocumentationChanges { .. } => {
                return crate::output::recent_commits::handle_recent_commits_command(
                    action,
                    base_dir.as_deref(),
                    cli.json,
                    cli.plain,
                    cli.verbose,
                    &perf,
                );
            }
            crate::args::RepoAction::Packages {
                filter,
                package_area,
                format,
            } => {
                return handle_repo_packages(
                    base_dir.as_deref(),
                    RepoPackagesArgs {
                        filter,
                        package_area: package_area.as_deref(),
                        format: *format,
                    },
                    cli.json,
                    cli.plain,
                    cli.verbose,
                    &perf,
                );
            }
            crate::args::RepoAction::PackageAreas {
                filter,
                package_area,
                format,
            } => {
                return handle_repo_package_areas(
                    base_dir.as_deref(),
                    RepoPackageAreasArgs {
                        filter,
                        package_area: package_area.as_deref(),
                        format: *format,
                    },
                    cli.json,
                    cli.plain,
                    cli.verbose,
                    &perf,
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

    // Lightweight repo subcommands only need the repo root and package list —
    // no worktrees, commits, or per-file change stats. Using `summary` skips the
    // expensive worktree enumeration (opens each worktree, runs status scan).
    let lightweight_repo_action = matches!(
        &repo_action,
        Some(
            crate::args::RepoAction::Root
                | crate::args::RepoAction::PackageRoot
                | crate::args::RepoAction::PackageAreaRoot
                | crate::args::RepoAction::Package { .. }
                | crate::args::RepoAction::PackageArea { .. }
        )
    );

    // Build git request based on deep/commit_count flags
    let git_request = if refresh_remotes_enabled {
        GitRequest::deep().commit_count(history_count)
    } else if lightweight_repo_action {
        GitRequest::summary()
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
        OutputFilter::Repo => match &repo_action {
            Some(crate::args::RepoAction::Language { breakdown: true }) => {
                DetectionPlan::new()
                    .without_os()
                    .without_hardware()
                    .without_network()
                    .filesystem(
                        FilesystemRequest::new()
                            .git(GitRequest::summary())
                            .without_docs()
                            .without_formatting(),
                    )
            }
            Some(crate::args::RepoAction::Language { breakdown: false }) => {
                DetectionPlan::new()
                    .without_os()
                    .without_hardware()
                    .without_network()
                    .filesystem(
                        FilesystemRequest::new()
                            .git(GitRequest::summary())
                            .without_repo()
                            .without_docs()
                            .without_formatting(),
                    )
            }
            _ => DetectionPlan::new()
                .without_os()
                .without_hardware()
                .without_network()
                .filesystem(
                    FilesystemRequest::new()
                        .git(git_request.clone())
                        .without_file_inventory(),
                ),
        },
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
        OutputFilter::All => {
            DetectionPlan::new().filesystem(FilesystemRequest::new().git(git_request.clone()))
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
        | OutputFilter::NotificationHelpers
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
    }
    .performance(cli.perf);

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
                        || git.status.untracked_count > 0
                        || git
                            .file_changes
                            .iter()
                            .any(|f| f.status == sniff::filesystem::git::FileStatus::Conflicted);
                }
            }
        }
    }

    // Handle Package/PackageArea early returns (need detection result but not enrichment)
    if let Some(ref action) = repo_action {
        match action {
            crate::args::RepoAction::Package { no_error, on_error } => {
                // Resolve plain name for both modes; verbose styling is a
                // text-only concern, JSON consumers only want the bare name.
                let plain_name = output::render_repo_package(&result, base_dir.as_deref(), 0);
                if plain_name.is_empty() {
                    if cli.json {
                        let outcome = output::repo_json::name_outcome(String::new());
                        output::print_json_value(outcome.value, result.performance.as_ref());
                        std::process::exit(if *no_error { 0 } else { 1 });
                    }
                    return handle_no_results(*no_error, on_error, cli.plain, &perf);
                }
                if cli.json {
                    let outcome = output::repo_json::name_outcome(plain_name);
                    output::print_json_value(outcome.value, result.performance.as_ref());
                    return Ok(());
                }
                let rendered = if cli.verbose > 0 {
                    output::render_repo_package(&result, base_dir.as_deref(), cli.verbose)
                } else {
                    plain_name
                };
                println!("{rendered}");
                perf.emit_stderr(result.performance.as_ref());
                return Ok(());
            }
            crate::args::RepoAction::PackageArea { no_error, on_error } => {
                let rendered = output::render_repo_package_area(&result, base_dir.as_deref());
                if rendered.is_empty() {
                    if cli.json {
                        let outcome = output::repo_json::name_outcome(String::new());
                        output::print_json_value(outcome.value, result.performance.as_ref());
                        std::process::exit(if *no_error { 0 } else { 1 });
                    }
                    return handle_no_results(*no_error, on_error, cli.plain, &perf);
                }
                if cli.json {
                    let outcome = output::repo_json::name_outcome(rendered);
                    output::print_json_value(outcome.value, result.performance.as_ref());
                    return Ok(());
                }
                println!("{rendered}");
                perf.emit_stderr(result.performance.as_ref());
                return Ok(());
            }
            _ => {}
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

    let is_scriptable_text = !use_json && is_scriptable_repo_action(repo_action.as_ref());

    if use_json {
        let exit_code = output::print_json(
            &result,
            output_filter,
            &docs_filter,
            &files_filter,
            repo_action.as_ref(),
            base_dir.as_deref(),
        )?;
        // When `--perf` was set, `attach_performance` already injected the
        // `performance` field into the JSON object (or wrapped non-objects
        // as `{ data, performance }`). Emitting an additional Markdown
        // `## Performance` block to stdout would corrupt the JSON output,
        // so we route the human-readable text to stderr instead — keeping
        // stdout strictly machine-parseable while preserving the legacy
        // verbose-style perf section for terminals.
        perf.emit_for_json(result.performance.as_ref());
        if let Some(code) = exit_code {
            std::process::exit(code);
        }
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
        if is_scriptable_text {
            perf.emit_stderr(result.performance.as_ref());
        } else {
            perf.emit_stdout(result.performance.as_ref());
        }
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

/// Determine if a repo action produces machine-readable/pipable text output.
///
/// Scriptable commands emit perf to stderr so stdout stays clean for pipelines.
fn is_scriptable_repo_action(action: Option<&crate::args::RepoAction>) -> bool {
    use crate::args::RepoAction;
    match action {
        None => false,
        Some(
            RepoAction::Structure { .. }
            | RepoAction::GitStatus { .. }
            | RepoAction::Deps { .. }
            | RepoAction::Remote { .. }
            | RepoAction::Hash { .. },
        ) => false,
        Some(_) => true,
    }
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
