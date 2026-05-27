use biscuit_terminal::components::renderable::TerminalRenderable;
use clap::{CommandFactory, Parser};
use clap_complete::{CompleteEnv, Shell};
use sniff::filesystem::blast_radius::{ChangeScope, ChangedPathKind, find_blast_radius_documents};
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
use std::fmt::Write as FmtWrite;

mod remote;
mod repo;

use remote::{
    commit_url_from_repo, handle_pr_command, handle_remote_url, handle_shorthand,
    resolve_origin_or_first_remote, resolve_remote_name,
};
use repo::{
    RepoPackageAreasArgs, RepoPackagesArgs, handle_file_list_command, handle_repo_package_areas,
    handle_repo_packages,
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

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => exit_with_parse_error(err),
    };

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
        if let Commands::Docs { paths_only, .. } = cmd {
            let base = cli
                .base
                .clone()
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));
            let docs_filter = cmd.docs_filter();

            let repo = git2::Repository::discover(&base)
                .map_err(|e| format!("Not a git repository: {}", e))?;
            let repo_root = repo
                .workdir()
                .ok_or("Bare repository not supported")?
                .to_path_buf();

            let has_area_filter = !docs_filter.package_area.is_empty();
            let has_pkg_filter = !docs_filter.package.is_empty();
            let needs_full_parse = cli.verbose > 0 || cli.json;

            let target_dirs: Vec<std::path::PathBuf> = if has_area_filter {
                docs_filter
                    .package_area
                    .iter()
                    .map(|area| repo_root.join(area))
                    .collect()
            } else if has_pkg_filter {
                let pkgs = sniff::filesystem::docs::detect_repo_packages(&repo_root);
                let lowered: Vec<String> = docs_filter
                    .package
                    .iter()
                    .map(|p| p.to_lowercase())
                    .collect();
                pkgs.iter()
                    .filter(|(name, _)| lowered.iter().any(|p| name.eq_ignore_ascii_case(p)))
                    .map(|(_, rel)| repo_root.join(rel))
                    .collect()
            } else {
                vec![repo_root.to_path_buf()]
            };

            if *paths_only {
                let paths = if has_area_filter || has_pkg_filter {
                    sniff::filesystem::docs::collect_markdown_paths_fast(&target_dirs)
                } else {
                    sniff::filesystem::docs::collect_markdown_paths_from_dirs(&target_dirs)
                };
                let filtered: Vec<String> = paths
                    .iter()
                    .map(|p| {
                        p.strip_prefix(&repo_root)
                            .unwrap_or(p)
                            .to_string_lossy()
                            .into_owned()
                    })
                    .filter(|rel| {
                        if !docs_filter.filter.is_empty() {
                            let path_lower = rel.to_lowercase();
                            return docs_filter
                                .filter
                                .iter()
                                .any(|s| path_lower.contains(&s.to_lowercase()));
                        }
                        true
                    })
                    .collect();

                if cli.json {
                    let json_val = serde_json::json!(filtered);
                    output::print_json_value(json_val, perf.build_report().as_ref());
                } else {
                    for path in &filtered {
                        println!("{}", path);
                    }
                }
                perf.emit_stdout(None);
                return Ok(());
            }

            let has_metadata_filter = docs_filter.has_prompt
                || docs_filter.blast_radius
                || docs_filter.readme
                || docs_filter.plan
                || docs_filter.src;

            let mut all_docs;

            if needs_full_parse && has_metadata_filter {
                let root_for_pkgs = repo_root.clone();
                let pkg_handle = std::thread::spawn(move || {
                    sniff::filesystem::docs::detect_repo_packages(&root_for_pkgs)
                });

                let phase1 = sniff::filesystem::docs::collect_markdown_files_from_dirs(
                    &target_dirs,
                    &repo_root,
                    &[],
                    sniff::filesystem::docs::DocParseMode::FrontmatterOnly,
                    |_| true,
                );

                let narrowed: Vec<std::path::PathBuf> = output::filter_docs(&phase1, &docs_filter)
                    .into_iter()
                    .map(|d| d.filepath.clone())
                    .collect();

                let packages = if has_area_filter && !has_pkg_filter {
                    sniff::filesystem::docs::detect_packages_in_dirs(&repo_root, &target_dirs)
                } else {
                    pkg_handle.join().unwrap_or_default()
                };

                if narrowed.is_empty() {
                    all_docs = vec![];
                } else {
                    all_docs = sniff::filesystem::docs::parse_markdown_files(
                        &narrowed,
                        &repo_root,
                        &packages,
                        sniff::filesystem::docs::DocParseMode::Full,
                    );
                }
            } else if has_area_filter && !has_pkg_filter {
                let packages =
                    sniff::filesystem::docs::detect_packages_in_dirs(&repo_root, &target_dirs);
                let mode = if needs_full_parse {
                    sniff::filesystem::docs::DocParseMode::Full
                } else {
                    sniff::filesystem::docs::DocParseMode::FrontmatterOnly
                };
                all_docs = sniff::filesystem::docs::collect_markdown_files_from_dirs(
                    &target_dirs,
                    &repo_root,
                    &packages,
                    mode,
                    |_| true,
                );
            } else if !has_area_filter && !has_pkg_filter && needs_full_parse {
                let root_for_pkgs = repo_root.clone();
                let pkg_handle = std::thread::spawn(move || {
                    sniff::filesystem::docs::detect_repo_packages(&root_for_pkgs)
                });
                all_docs = sniff::filesystem::docs::collect_markdown_files_from_dirs(
                    &target_dirs,
                    &repo_root,
                    &[],
                    sniff::filesystem::docs::DocParseMode::Full,
                    |_| true,
                );
                let packages = pkg_handle.join().unwrap_or_default();
                sniff::filesystem::docs::assign_packages(&mut all_docs, &packages, &repo_root);
            } else if has_pkg_filter || needs_full_parse {
                let packages = sniff::filesystem::docs::detect_repo_packages(&repo_root);
                let mode = if needs_full_parse {
                    sniff::filesystem::docs::DocParseMode::Full
                } else {
                    sniff::filesystem::docs::DocParseMode::FrontmatterOnly
                };
                all_docs = sniff::filesystem::docs::collect_markdown_files_from_dirs(
                    &target_dirs,
                    &repo_root,
                    &packages,
                    mode,
                    |_| true,
                );
            } else {
                all_docs = sniff::filesystem::docs::collect_markdown_files_from_dirs(
                    &target_dirs,
                    &repo_root,
                    &[],
                    sniff::filesystem::docs::DocParseMode::FrontmatterOnly,
                    |_| true,
                );
            }

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
                let remote = match remote {
                    Some(r) => r.clone(),
                    None => {
                        let dir = base_dir
                            .as_deref()
                            .unwrap_or_else(|| std::path::Path::new("."));
                        let repo = git2::Repository::discover(dir)
                            .map_err(|_| "No git repository found — provide a REMOTE argument or run from inside a git repo".to_string())?;
                        resolve_origin_or_first_remote(&repo).ok_or(
                            "No git remotes found for this repository — provide a REMOTE argument",
                        )?
                    }
                };
                if remote.contains("://") || remote.starts_with("git@") {
                    return handle_remote_url(&remote, cli.json, cli.plain, cli.verbose, &perf)
                        .await;
                } else if is_owner_repo_shorthand(&remote) {
                    return handle_shorthand(&remote, cli.json, cli.plain, cli.verbose, &perf)
                        .await;
                } else {
                    let url =
                        resolve_remote_name(&remote, base_dir.as_deref()).ok_or_else(|| {
                            format!(
                                "Could not find remote '{}' in the current repository",
                                remote
                            )
                        })?;
                    return handle_remote_url(&url, cli.json, cli.plain, cli.verbose, &perf).await;
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
            crate::args::RepoAction::UnstagedFiles {
                package,
                package_area,
            } => {
                let args = crate::args::FileListArgs {
                    package: package.clone(),
                    package_area: package_area.clone(),
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
            crate::args::RepoAction::UntrackedFiles {
                package,
                package_area,
            } => {
                let args = crate::args::FileListArgs {
                    package: package.clone(),
                    package_area: package_area.clone(),
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
                // git2's workdir() yields a trailing separator; collecting
                // components drops it so downstream consumers get a clean path.
                let workdir: std::path::PathBuf = workdir.components().collect();
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
            crate::args::RepoAction::Name => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let identity = sniff::filesystem::repo::detect_repo_identity(dir)?;
                if cli.json {
                    let json = serde_json::to_value(&identity)?;
                    output::print_json_value(json, perf.build_report().as_ref());
                    return Ok(());
                }
                let rendered = output::render_repo_name(&identity, cli.verbose);
                output::emit_text(&rendered, cli.plain);
                perf.emit_stderr(None);
                return Ok(());
            }
            crate::args::RepoAction::Worktree { no_error, on_error } => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let name = sniff::filesystem::git::get_current_worktree_name(dir)?;
                if let Some(name) = name {
                    if cli.json {
                        let outcome = output::repo_json::worktree_outcome(Some(&name), *no_error);
                        output::print_json_value(outcome.value, perf.build_report().as_ref());
                        return Ok(());
                    }
                    if cli.verbose > 0 {
                        if let Some(info) = sniff::filesystem::git::get_current_worktree_info(dir)?
                        {
                            println!("{} [{}]", info.0, info.1);
                        } else {
                            println!("{name}");
                        }
                    } else {
                        println!("{name}");
                    }
                    perf.emit_stderr(None);
                    return Ok(());
                }
                // Not in a worktree (or not in a repo)
                if cli.json {
                    let outcome = output::repo_json::worktree_outcome(None, *no_error);
                    output::print_json_value(outcome.value, perf.build_report().as_ref());
                    std::process::exit(if *no_error { 0 } else { 1 });
                }
                return handle_no_results(*no_error, on_error, cli.plain, &perf);
            }
            crate::args::RepoAction::Worktrees { list, csv, .. } => {
                let dir = base_dir
                    .as_deref()
                    .unwrap_or_else(|| std::path::Path::new("."));
                let worktrees = sniff::filesystem::git::list_worktrees(dir)?;
                let Some(entries) = worktrees else {
                    return Err("Not a git repository".into());
                };

                if cli.json {
                    let json = output::repo_json::worktrees_value(&entries);
                    output::print_json_value(json, perf.build_report().as_ref());
                } else if *csv {
                    let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
                    println!("{}", names.join(", "));
                } else if cli.verbose > 0 {
                    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
                    let terminal = biscuit_terminal::terminal::Terminal::default();
                    let mut lines = Vec::new();
                    for entry in &entries {
                        let marker = if entry.is_current { "* " } else { "  " };
                        let branch_text = if entry.is_detached {
                            "detached HEAD".to_string()
                        } else {
                            entry
                                .branch
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string())
                        };
                        let display_path = if let Some(ref home) = home {
                            if let Ok(stripped) = entry.path.strip_prefix(home) {
                                format!("~/{}", stripped.display())
                            } else {
                                entry.path.display().to_string()
                            }
                        } else {
                            entry.path.display().to_string()
                        };
                        let absolute_path = entry.path.display().to_string();
                        let markup = format!(
                            "{marker}<b>{}</b> (on <green>{}</green> branch, located at <a href=\"{}\">{}</a>)",
                            entry.name, branch_text, absolute_path, display_path
                        );
                        let prose = biscuit_terminal::components::prose::Prose::new(&markup);
                        lines.push(prose.render(&terminal));
                    }
                    let rendered = lines.join("\n") + "\n";
                    output::emit_text(&rendered, cli.plain);
                } else if *list {
                    let mut out = String::new();
                    for entry in &entries {
                        let marker = if entry.is_current { "* " } else { "" };
                        writeln!(out, "- {}{}", marker, entry.name).unwrap();
                    }
                    output::emit_text(&out, cli.plain);
                } else {
                    let mut out = String::new();
                    for entry in &entries {
                        let marker = if entry.is_current { "* " } else { "  " };
                        writeln!(out, "{}{}", marker, entry.name).unwrap();
                    }
                    output::emit_text(&out, cli.plain);
                }
                perf.emit_stderr(None);
                return Ok(());
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
                package,
                package_area,
                format,
                no_error,
                on_error,
            } => {
                return handle_repo_packages(
                    base_dir.as_deref(),
                    RepoPackagesArgs {
                        filter,
                        package: package.as_deref(),
                        package_area: package_area.as_deref(),
                        format: *format,
                        no_error: *no_error,
                        on_error: on_error.clone(),
                    },
                    cli.json,
                    cli.plain,
                    cli.verbose,
                    &perf,
                );
            }
            crate::args::RepoAction::PackageAreas {
                filter,
                package,
                package_area,
                format,
                no_error,
                on_error,
            } => {
                return handle_repo_package_areas(
                    base_dir.as_deref(),
                    RepoPackageAreasArgs {
                        filter,
                        package: package.as_deref(),
                        package_area: package_area.as_deref(),
                        format: *format,
                        no_error: *no_error,
                        on_error: on_error.clone(),
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
                | crate::args::RepoAction::Area { .. }
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
            Some(crate::args::RepoAction::Language { breakdown: true }) => DetectionPlan::new()
                .without_os()
                .without_hardware()
                .without_network()
                .filesystem(
                    FilesystemRequest::new()
                        .git(GitRequest::summary())
                        .without_docs()
                        .without_formatting(),
                ),
            Some(crate::args::RepoAction::Language { breakdown: false }) => DetectionPlan::new()
                .without_os()
                .without_hardware()
                .without_network()
                .filesystem(
                    FilesystemRequest::new()
                        .git(GitRequest::summary())
                        .without_repo()
                        .without_docs()
                        .without_formatting(),
                ),
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

    // Handle package scoping for git actions. Both `--package` and
    // `--package-area` are honored; when both are passed, the resolved package
    // path must lie within the resolved area (intersection error fires here).
    let git_scope = match &repo_action {
        Some(crate::args::RepoAction::GitStatus {
            package,
            package_area,
            ..
        }) => resolve_package_and_area(
            packages_from_result(&result),
            package.as_deref(),
            package_area.as_deref(),
        )?,
        _ => None,
    };
    if let Some(path_prefix) = git_scope.as_deref()
        && let Some(ref mut filesystem) = result.filesystem
        && filesystem.git.is_some()
    {
        let dir = base_dir
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new("."));
        if let Ok(repo) = git2::Repository::discover(dir) {
            let scoped_commits =
                sniff::filesystem::get_commits_for_path(&repo, path_prefix, history_count);
            if let Some(ref mut git) = filesystem.git {
                git.recent = scoped_commits;

                // Filter file_changes to the package path
                git.file_changes
                    .retain(|f| f.path.to_string_lossy().starts_with(path_prefix));

                // Filter dirty files
                git.status
                    .dirty
                    .retain(|f| f.filepath.to_string_lossy().starts_with(path_prefix));

                // Filter untracked files
                git.status
                    .untracked
                    .retain(|f| f.filepath.to_string_lossy().starts_with(path_prefix));

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

    // Branch filter for `repo git-status --branch [<name>]`. When the flag is
    // given without a value, the current branch is used (which is a no-op vs
    // the default HEAD walk). When a different branch is given, replace the
    // recent commit list with one walked from that branch's tip.
    if let Some(crate::args::RepoAction::GitStatus {
        branch: Some(branch_opt),
        ..
    }) = &repo_action
        && let Some(ref mut filesystem) = result.filesystem
        && let Some(ref mut git) = filesystem.git
    {
        let explicit = branch_opt.as_deref().filter(|s| !s.is_empty());
        let target_branch = explicit
            .map(str::to_string)
            .or_else(|| git.current_branch.clone());

        if let Some(branch_name) = target_branch
            && git.current_branch.as_deref() != Some(branch_name.as_str())
        {
            let dir = base_dir
                .as_deref()
                .unwrap_or_else(|| std::path::Path::new("."));
            if let Ok(repo) = git2::Repository::discover(dir) {
                git.recent =
                    sniff::filesystem::get_commits_for_branch(&repo, &branch_name, history_count);
            }

            // Working-tree state belongs to the currently checked-out branch,
            // not the one being inspected — clear it so the output doesn't
            // imply those files are dirty on the target branch.
            git.file_changes.clear();
            git.status.dirty.clear();
            git.status.untracked.clear();
            git.status.staged_count = 0;
            git.status.unstaged_count = 0;
            git.status.untracked_count = 0;
            git.status.is_dirty = false;
        }
    }

    // Worktree filter for `repo git-status --worktree <name>`. Replaces both
    // the recent commit list and the working-tree status with those of the
    // selected worktree. The selector matches against either the worktree's
    // branch (the `worktrees` HashMap key) or its directory basename.
    if let Some(crate::args::RepoAction::GitStatus {
        worktree: Some(wt_selector),
        ..
    }) = &repo_action
        && let Some(ref mut filesystem) = result.filesystem
        && let Some(ref mut git) = filesystem.git
    {
        let matched = git
            .worktrees
            .values()
            .find(|info| {
                info.branch == *wt_selector
                    || info
                        .filepath
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|basename| basename == *wt_selector)
            })
            .cloned();

        match matched {
            Some(info) => {
                if let Some(wt_repo) = sniff::filesystem::git::GitRepo::discover(&info.filepath)? {
                    git.recent = wt_repo.recent_commits(history_count);

                    let file_changes = wt_repo.file_changes().unwrap_or_default();
                    let repo_status = wt_repo.repo_status().unwrap_or_default();
                    git.file_changes = file_changes;
                    git.status = repo_status;
                }
                // Reflect the worktree's location so absolute file paths in
                // the rendered output resolve to the worktree, and the queried
                // branch so downstream code sees it as the current branch.
                git.repo_root = info.filepath.clone();
                git.current_branch = Some(info.branch.clone());
            }
            None => {
                let names: Vec<String> = git
                    .worktrees
                    .values()
                    .map(|info| info.branch.clone())
                    .collect();
                let listing = if names.is_empty() {
                    "no linked worktrees found".to_string()
                } else {
                    format!("available: {}", names.join(", "))
                };
                return Err(format!("Worktree not found: {} ({})", wt_selector, listing).into());
            }
        }
    }

    // Tier 2/3 scoping: validate that `--package` and `--package-area` resolve
    // and (when both supplied) overlap. The output functions apply the actual
    // filtering; this call surfaces the intersection error before render time.
    if let Some(
        crate::args::RepoAction::Structure {
            package,
            package_area,
            ..
        }
        | crate::args::RepoAction::Deps {
            package,
            package_area,
            ..
        }
        | crate::args::RepoAction::DirtyPackages {
            package,
            package_area,
            ..
        }
        | crate::args::RepoAction::DirtyPackageAreas {
            package,
            package_area,
            ..
        }
        | crate::args::RepoAction::StagedPackages {
            package,
            package_area,
            ..
        }
        | crate::args::RepoAction::StagedPackageAreas {
            package,
            package_area,
            ..
        }
        | crate::args::RepoAction::UnstagedPackages {
            package,
            package_area,
            ..
        }
        | crate::args::RepoAction::UnstagedPackageAreas {
            package,
            package_area,
            ..
        },
    ) = &repo_action
    {
        resolve_package_and_area(
            packages_from_result(&result),
            package.as_deref(),
            package_area.as_deref(),
        )?;
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
            crate::args::RepoAction::Area { no_error, on_error } => {
                let fs = result.filesystem.as_ref();
                let repo = fs.and_then(|fs| fs.repo.as_ref());
                let is_monorepo = repo.is_some_and(|r| r.is_monorepo);
                if is_monorepo {
                    let rendered = output::render_repo_area(&result, base_dir.as_deref());
                    if cli.json {
                        let outcome = output::repo_json::name_outcome(rendered);
                        output::print_json_value(outcome.value, result.performance.as_ref());
                        return Ok(());
                    }
                    println!("{rendered}");
                    perf.emit_stderr(result.performance.as_ref());
                    return Ok(());
                }
                // Not a monorepo. Distinguish "in a git repo" from "not in a
                // repo at all" using the git detection result.
                if cli.verbose > 0 {
                    let in_git_repo = fs.and_then(|fs| fs.git.as_ref()).is_some();
                    let msg = if in_git_repo {
                        "'area' is a term that holds meaning only in a monorepo; you are in a repo but not a monorepo!"
                    } else {
                        "'area' is a term that holds meaning only in a monorepo; you are not in a repo!"
                    };
                    eprintln!("{msg}");
                }
                if cli.json {
                    let outcome = output::repo_json::name_outcome(String::new());
                    output::print_json_value(outcome.value, result.performance.as_ref());
                    std::process::exit(if *no_error { 0 } else { 1 });
                }
                return handle_no_results(*no_error, on_error, cli.plain, &perf);
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

/// Print a clap parse error and exit.
///
/// For `sniff repo <unknown>` errors, append the categorized list of valid
/// repo subcommands so users do not have to re-run with `--help`.
fn exit_with_parse_error(err: clap::Error) -> ! {
    use clap::error::ErrorKind;

    let kind = err.kind();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let under_repo = argv.iter().any(|a| a == "repo");
    let is_unknown_sub = matches!(
        kind,
        ErrorKind::InvalidSubcommand | ErrorKind::UnknownArgument
    );

    if under_repo && is_unknown_sub {
        let _ = err.print();
        eprintln!("\n{}", crate::args::REPO_AFTER_HELP);
        std::process::exit(2);
    }
    err.exit();
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
/// Performs an exact case-insensitive match on `Package.name`. The returned
/// path always ends with a trailing `/` so `starts_with` filtering matches
/// whole-segment boundaries.
///
/// ## Errors
///
/// Returns an error listing valid package names when no package matches.
pub(super) fn resolve_package_path(
    packages: Option<&[sniff::filesystem::repo::Package]>,
    pkg_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(packages) = packages else {
        return Err("No packages found in this repository".into());
    };

    let lower = pkg_name.to_lowercase();

    if let Some(pkg) = packages.iter().find(|p| p.name.to_lowercase() == lower) {
        return Ok(format!("{}/", pkg.relative));
    }

    let mut names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    names.sort();
    names.dedup();

    Err(format!(
        "Package '{}' not found.\n\nValid package names: {}",
        pkg_name,
        names.join(", "),
    )
    .into())
}

/// Resolve a package-area name to its path prefix for git filtering.
///
/// Performs a case-insensitive **prefix** match on `Package.package_area`.
/// The returned path always ends with a trailing `/` so `starts_with`
/// filtering matches whole-segment boundaries. The canonical area casing
/// from the package list is preserved.
///
/// ## Errors
///
/// Returns an error listing valid package areas when no area matches.
pub(super) fn resolve_package_area_path(
    packages: Option<&[sniff::filesystem::repo::Package]>,
    area: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let Some(packages) = packages else {
        return Err("No packages found in this repository".into());
    };

    let lower = area.to_lowercase();

    if let Some(pkg) = packages
        .iter()
        .find(|p| p.package_area.to_lowercase().starts_with(&lower))
    {
        return Ok(format!("{}/", pkg.package_area));
    }

    let mut areas: Vec<&str> = packages.iter().map(|p| p.package_area.as_str()).collect();
    areas.sort();
    areas.dedup();

    Err(format!(
        "Package area '{}' not found.\n\nValid package areas: {}",
        area,
        areas.join(", "),
    )
    .into())
}

/// Resolve and intersect optional `--package` and `--package-area` inputs.
///
/// Returns `Ok(None)` when both inputs are `None`. Otherwise resolves the
/// inputs that are `Some` via [`resolve_package_path`] and
/// [`resolve_package_area_path`] respectively. When **both** are `Some`, the
/// resolved package path must start with the resolved area path; otherwise
/// the function returns a hard error. The narrower prefix (the package path)
/// is returned in the overlap case.
///
/// ## Errors
///
/// - Either resolver propagates its error when an input does not match.
/// - When both resolve but the package is in a different area, the error
///   message names the package, its real area, and the requested area.
pub(super) fn resolve_package_and_area(
    packages: Option<&[sniff::filesystem::repo::Package]>,
    package: Option<&str>,
    area: Option<&str>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    match (package, area) {
        (None, None) => Ok(None),
        (Some(name), None) => resolve_package_path(packages, name).map(Some),
        (None, Some(area)) => resolve_package_area_path(packages, area).map(Some),
        (Some(name), Some(area)) => {
            let pkg_path = resolve_package_path(packages, name)?;
            let area_path = resolve_package_area_path(packages, area)?;
            if !pkg_path.starts_with(&area_path) {
                let lower_name = name.to_lowercase();
                let real_area = packages
                    .and_then(|pkgs| pkgs.iter().find(|p| p.name.to_lowercase() == lower_name))
                    .map(|p| p.package_area.clone())
                    .unwrap_or_else(|| pkg_path.trim_end_matches('/').to_string());
                let requested = area_path_label(packages, area).unwrap_or_else(|| area.to_string());
                return Err(format!(
                    "Package '{name}' is in area '{real_area}', not '{requested}'"
                )
                .into());
            }
            Ok(Some(pkg_path))
        }
    }
}

/// Return the canonical package-area label for an input string, if any
/// package's area starts with the (case-insensitive) input.
fn area_path_label(
    packages: Option<&[sniff::filesystem::repo::Package]>,
    area: &str,
) -> Option<String> {
    let packages = packages?;
    let lower = area.to_lowercase();
    packages
        .iter()
        .find(|p| p.package_area.to_lowercase().starts_with(&lower))
        .map(|p| p.package_area.clone())
}

/// Extract the package list from a [`SniffResult`] for resolver consumption.
fn packages_from_result(result: &SniffResult) -> Option<&[sniff::filesystem::repo::Package]> {
    result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.repo.as_ref())
        .and_then(|repo| repo.packages.as_deref())
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
