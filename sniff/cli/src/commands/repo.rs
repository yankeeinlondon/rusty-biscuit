use sniff::filesystem::blast_radius::{
    ChangeScope, ChangedPathKind, ChangedPathQuery, collect_changed_paths,
};
use sniff::filesystem::repo::detect_repo_structure;

use crate::args::{FileListArgs, PackagesFormat};
use crate::output::{self, PathListFormat};
use crate::perf::{CliPerf, handle_no_results};

/// Subcommand-specific args for `sniff repo packages`.
pub(super) struct RepoPackagesArgs<'a> {
    pub(super) filter: &'a [String],
    pub(super) package: Option<&'a str>,
    pub(super) package_area: Option<&'a str>,
    pub(super) format: PackagesFormat,
    pub(super) no_error: bool,
    pub(super) on_error: Option<String>,
}

/// Fast-path handler for `sniff repo packages`.
///
/// Skips the full detection pipeline and calls `detect_repo_structure` directly,
/// which avoids git scanning, file inventory, language detection, docs, and
/// formatting work. Typical wall time on a large monorepo: well under 50ms.
pub(super) fn handle_repo_packages(
    base_dir: Option<&std::path::Path>,
    args: RepoPackagesArgs<'_>,
    json: bool,
    plain: bool,
    verbose: u8,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    let RepoPackagesArgs {
        filter,
        package,
        package_area,
        format,
        no_error,
        on_error,
    } = args;
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let explicit = base_dir.unwrap_or(&cwd);
    let root = if base_dir.is_some() {
        explicit.to_path_buf()
    } else {
        sniff::filesystem::repo_root(explicit)?.unwrap_or_else(|| explicit.to_path_buf())
    };

    let info = match detect_repo_structure(&root)? {
        Some(info) => info,
        None => {
            return Err("Not inside a recognized repository".into());
        }
    };

    // Validate that `--package` and `--package-area` resolve and (when both
    // are passed) overlap. The output functions apply the actual filtering;
    // this call surfaces the intersection error before render time.
    super::resolve_package_and_area(info.packages.as_deref(), package, package_area)?;

    let names = output::collect_repo_package_names(&info, filter, package, package_area);
    let is_empty = info.is_monorepo && names.is_empty();

    if is_empty {
        if json {
            println!("{}", serde_json::to_string(&names)?);
            perf.emit_stderr(None);
        }
        return handle_no_results(no_error, &on_error, plain, perf);
    }

    if json {
        println!("{}", serde_json::to_string(&names)?);
        perf.emit_stderr(None);
        return Ok(());
    }

    let rendered = output::render_repo_packages_formatted(
        &info,
        filter,
        package,
        package_area,
        format,
        verbose,
    );
    let with_newline = if rendered.ends_with('\n') {
        rendered
    } else {
        format!("{rendered}\n")
    };
    output::emit_text(&with_newline, plain);
    perf.emit_stderr(None);
    Ok(())
}

/// Subcommand-specific args for `sniff repo package-areas`.
pub(super) struct RepoPackageAreasArgs<'a> {
    pub(super) filter: &'a [String],
    pub(super) package: Option<&'a str>,
    pub(super) package_area: Option<&'a str>,
    pub(super) format: PackagesFormat,
    pub(super) no_error: bool,
    pub(super) on_error: Option<String>,
}

/// Fast-path handler for `sniff repo package-areas`.
///
/// Uses `detect_repo_structure` (same fast path as `sniff repo packages`) so
/// the command returns well under 100 ms even on large monorepos.
pub(super) fn handle_repo_package_areas(
    base_dir: Option<&std::path::Path>,
    args: RepoPackageAreasArgs<'_>,
    json: bool,
    plain: bool,
    verbose: u8,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    let RepoPackageAreasArgs {
        filter,
        package,
        package_area,
        format,
        no_error,
        on_error,
    } = args;
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let explicit = base_dir.unwrap_or(&cwd);
    let root = if base_dir.is_some() {
        explicit.to_path_buf()
    } else {
        sniff::filesystem::repo_root(explicit)?.unwrap_or_else(|| explicit.to_path_buf())
    };

    let info = match detect_repo_structure(&root)? {
        Some(info) => info,
        None => {
            return Err("Not inside a recognized repository".into());
        }
    };

    // Validate that `--package` and `--package-area` resolve and (when both
    // are passed) overlap. The output functions apply the actual filtering;
    // this call surfaces the intersection error before render time.
    super::resolve_package_and_area(info.packages.as_deref(), package, package_area)?;

    let names = output::collect_repo_package_area_names(&info, filter, package, package_area);
    let is_empty = info.is_monorepo && names.is_empty();

    if is_empty {
        if json {
            println!("{}", serde_json::to_string(&names)?);
            perf.emit_stderr(None);
        }
        return handle_no_results(no_error, &on_error, plain, perf);
    }

    if json {
        println!("{}", serde_json::to_string(&names)?);
        perf.emit_stderr(None);
        return Ok(());
    }

    let rendered = output::render_repo_package_areas_formatted(
        &info,
        filter,
        package,
        package_area,
        format,
        verbose,
    );
    let with_newline = if rendered.ends_with('\n') {
        rendered
    } else {
        format!("{rendered}\n")
    };
    output::emit_text(&with_newline, plain);
    perf.emit_stderr(None);
    Ok(())
}

/// Resolve `FileListArgs` flags into a `PathListFormat`.
pub(super) fn path_list_format(args: &FileListArgs) -> PathListFormat {
    if args.list {
        PathListFormat::BulletList
    } else if args.csv {
        PathListFormat::Csv
    } else {
        PathListFormat::Lines
    }
}

/// Subcommand-specific args for `sniff repo test-runner`.
pub(super) struct RepoTestRunnerArgs {
    pub(super) csv: bool,
    pub(super) list: bool,
    pub(super) md: bool,
}

/// Subcommand-specific args for `sniff repo package-manager`.
pub(super) struct RepoPackageManagerArgs {
    pub(super) csv: bool,
    pub(super) list: bool,
    pub(super) md: bool,
}

/// Handle `sniff repo branches`.
pub(super) fn handle_repo_branches(
    base_dir: Option<&std::path::Path>,
    refresh_remotes: bool,
    json: bool,
    plain: bool,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;

    let dir = base_dir.unwrap_or_else(|| std::path::Path::new("."));
    let Some(branches) = sniff::filesystem::git::branches_at(dir, refresh_remotes)? else {
        return Err("Not a git repository".into());
    };

    if json {
        crate::output::print_json_value(
            serde_json::to_value(&branches)?,
            perf.build_report().as_ref(),
        );
        return Ok(());
    }

    let term = Terminal::default();
    let mut out = String::new();
    for branch in branches {
        use std::fmt::Write;
        let marker = if branch.current { "* " } else { "  " };
        let upstream = branch
            .upstream
            .as_deref()
            .map(|upstream| format!(" <dim>{upstream}</dim>"))
            .unwrap_or_default();
        let sync = if branch.ahead > 0 || branch.behind > 0 {
            format!(" <dim>ahead {} behind {}</dim>", branch.ahead, branch.behind)
        } else {
            String::new()
        };
        let remote = if branch.remote_represented {
            " <dim>remote</dim>"
        } else {
            ""
        };
        let markup = format!(
            "{marker}<b>{}</b> <dim>{}</dim>{upstream}{sync}{remote}",
            branch.name,
            &branch.sha[..8.min(branch.sha.len())]
        );
        writeln!(out, "{}", Prose::new(markup).render(&term)).unwrap();
    }
    crate::output::emit_text(&out, plain);
    perf.emit_stderr(None);
    Ok(())
}

/// Handle `sniff repo dependencies`.
pub(super) fn handle_repo_dependencies(
    base_dir: Option<&std::path::Path>,
    filter: sniff::filesystem::repo::ExternalDependencyFilter,
    json: bool,
    plain: bool,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    use sniff::filesystem::repo::{
        AggregateScope, collect_external_dependencies, detect_repo_structure, resolve_scope,
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let explicit = base_dir.unwrap_or(&cwd);
    let root = if base_dir.is_some() {
        explicit.to_path_buf()
    } else {
        sniff::filesystem::repo_root(explicit)?.unwrap_or_else(|| explicit.to_path_buf())
    };
    let Some(info) = detect_repo_structure(&root)? else {
        return Err("Not inside a recognized repository".into());
    };
    let dir_for_scope = if base_dir.is_some() { explicit } else { &cwd };
    let scope = if info.packages.as_ref().is_some_and(|p| !p.is_empty()) {
        resolve_scope(&info, dir_for_scope)
    } else {
        AggregateScope::Repo
    };
    let deps = collect_external_dependencies(&info, &scope, filter);

    if json {
        crate::output::print_json_value(
            serde_json::json!({ "dependencies": deps }),
            perf.build_report().as_ref(),
        );
        return Ok(());
    }

    if deps.is_empty() {
        perf.emit_stderr(None);
        std::process::exit(1);
    }

    let term = Terminal::default();
    let mut out = String::new();
    for dep in deps {
        use std::fmt::Write;
        let version = dep.dependency.targeted_version;
        let manager = dep.dependency.package_manager.unwrap_or_default();
        let markup = format!(
            "<b>{}</b> <dim>{version}</dim> <dim>{manager}</dim> <dim>({:?} in {})</dim>",
            dep.dependency.name, dep.family, dep.package
        );
        writeln!(out, "{}", Prose::new(markup).render(&term)).unwrap();
    }
    crate::output::emit_text(&out, plain);
    perf.emit_stderr(None);
    Ok(())
}

/// Handle `sniff repo package-manager`.
///
/// Reports package manager usage for the current repo/package context using
/// the shared library aggregation helper also used by `repo test-runner`.
pub(super) fn handle_repo_package_manager(
    base_dir: Option<&std::path::Path>,
    args: RepoPackageManagerArgs,
    json: bool,
    plain: bool,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    use sniff::filesystem::repo::{
        AggregateResult, AggregateScope, aggregate_package_values, detect_repo_structure,
        resolve_scope,
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let explicit = base_dir.unwrap_or(&cwd);
    let root = if base_dir.is_some() {
        explicit.to_path_buf()
    } else {
        sniff::filesystem::repo_root(explicit)?.unwrap_or_else(|| explicit.to_path_buf())
    };

    let info = detect_repo_structure(&root)?;
    let dir_for_scope = if base_dir.is_some() { explicit } else { &cwd };

    let (result, scope_kind): (AggregateResult<String>, &'static str) = match &info {
        Some(info) if info.is_monorepo && info.packages.as_ref().is_some_and(|p| !p.is_empty()) => {
            let packages = info.packages.as_deref().expect("non-empty packages");
            let scope = resolve_scope(info, dir_for_scope);
            let kind = match &scope {
                AggregateScope::Package(_) => "package",
                AggregateScope::PackageArea(_) => "package-area",
                AggregateScope::Repo => "repo",
            };
            let result = aggregate_package_values(
                packages,
                &scope,
                |pkg| pkg.package_managers.clone(),
                |manager: &String| manager.clone(),
            );
            (result, kind)
        }
        Some(info) => {
            let values = info
                .packages
                .as_deref()
                .and_then(|packages| packages.first())
                .map(|pkg| pkg.package_managers.clone())
                .unwrap_or_default();
            (aggregate_strings(values), "package")
        }
        None => (AggregateResult::Empty, "package"),
    };

    if json {
        let value = build_string_aggregate_json(&result, "package_manager", "package_managers");
        crate::output::print_json_value(value, perf.build_report().as_ref());
        return Ok(());
    }

    if matches!(result, AggregateResult::Empty) {
        if !plain {
            eprintln!(
                "{}",
                Prose::new("<dim>No package manager for this context.</dim>")
                    .render(&Terminal::default())
            );
        }
        perf.emit_stderr(None);
        std::process::exit(1);
    }

    let rendered = match &result {
        AggregateResult::Singular(manager) => {
            render_string_values(std::slice::from_ref(manager), &args, false)
        }
        AggregateResult::Multiple(managers) => {
            let mut rendered = render_string_values(managers, &args, true);
            if !args.csv && !args.list && !args.md && scope_kind != "package" {
                let term = Terminal::default();
                let hint = match scope_kind {
                    "package-area" => " (across the current package-area)",
                    "repo" => " (across all packages)",
                    _ => "",
                };
                if !hint.is_empty() {
                    rendered.push_str(
                        &Prose::new(format!("<dim>distinct package managers{hint}</dim>\n"))
                            .render(&term),
                    );
                }
            }
            rendered
        }
        AggregateResult::Empty => unreachable!("handled above"),
    };

    crate::output::emit_text(&rendered, plain);
    perf.emit_stderr(None);
    Ok(())
}

fn aggregate_strings(values: Vec<String>) -> sniff::filesystem::repo::AggregateResult<String> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    match unique.len() {
        0 => sniff::filesystem::repo::AggregateResult::Empty,
        1 => sniff::filesystem::repo::AggregateResult::Singular(
            unique.pop().expect("exactly one value"),
        ),
        _ => sniff::filesystem::repo::AggregateResult::Multiple(unique),
    }
}

fn build_string_aggregate_json(
    result: &sniff::filesystem::repo::AggregateResult<String>,
    singular_key: &str,
    plural_key: &str,
) -> serde_json::Value {
    match result {
        sniff::filesystem::repo::AggregateResult::Singular(value) => {
            serde_json::json!({ singular_key: value })
        }
        sniff::filesystem::repo::AggregateResult::Multiple(values) => {
            serde_json::json!({ plural_key: values })
        }
        sniff::filesystem::repo::AggregateResult::Empty => {
            serde_json::json!({ singular_key: serde_json::Value::Null })
        }
    }
}

fn render_string_values(values: &[String], args: &RepoPackageManagerArgs, multiple: bool) -> String {
    if args.csv {
        return format!("{}\n", values.join(", "));
    }
    if args.md {
        let mut out = String::new();
        for value in values {
            use std::fmt::Write;
            let _ = writeln!(out, "- {value}");
        }
        return out;
    }
    if args.list || multiple {
        let mut out = String::new();
        for value in values {
            use std::fmt::Write;
            let _ = writeln!(out, "{value}");
        }
        return out;
    }
    format!("{}\n", values.first().expect("non-empty values"))
}

/// Handle `sniff repo test-runner`.
///
/// Reports declared test runner usage for the current repo/package context
/// using the shared library aggregation helper. Non-monorepo contexts (or
/// monorepo contexts outside any package/area) detect at the resolved
/// directory; monorepo package/area/repo scopes collapse per the spec.
pub(super) fn handle_repo_test_runner(
    base_dir: Option<&std::path::Path>,
    args: RepoTestRunnerArgs,
    json: bool,
    plain: bool,
    verbose: u8,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    use sniff::filesystem::repo::{
        AggregateResult, AggregateScope, TestRunnerUsage, aggregate_package_values,
        detect_repo_structure, detect_test_runners_for_dir, resolve_scope,
    };
    use sniff::programs::schema::ProgramMetadata;

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let explicit = base_dir.unwrap_or(&cwd);
    let root = if base_dir.is_some() {
        explicit.to_path_buf()
    } else {
        sniff::filesystem::repo_root(explicit)?.unwrap_or_else(|| explicit.to_path_buf())
    };

    let info = detect_repo_structure(&root)?;

    // Resolve the scope relative to the CWD so the command answers
    // "what does THIS directory declare?" rather than always the repo root.
    let dir_for_scope = if base_dir.is_some() { explicit } else { &cwd };

    // Build the aggregate result. Two paths:
    //   1. Monorepo with packages → collapse via the shared helper.
    //   2. Non-monorepo / no packages → detect at the directory directly.
    let (result, scope_kind): (AggregateResult<TestRunnerUsage>, &'static str) = match &info {
        Some(info) if info.is_monorepo && info.packages.as_ref().is_some_and(|p| !p.is_empty()) => {
            let packages = info.packages.as_deref().expect("non-empty packages");
            let scope = resolve_scope(info, dir_for_scope);
            let kind = match &scope {
                AggregateScope::Package(_) => "package",
                AggregateScope::PackageArea(_) => "package-area",
                AggregateScope::Repo => "repo",
            };
            let result = aggregate_package_values(
                packages,
                &scope,
                |pkg| pkg.test_runners.clone(),
                |usage: &TestRunnerUsage| usage.runner,
            );
            (result, kind)
        }
        _ => {
            // Non-monorepo (or no discovered packages): detect at the
            // resolved root directory directly. The result is trivially
            // singular or empty.
            let usages = detect_test_runners_for_dir(&root);
            let result = match usages.len() {
                0 => AggregateResult::Empty,
                1 => AggregateResult::Singular(usages.into_iter().next().expect("one")),
                _ => AggregateResult::Multiple(usages),
            };
            (result, "package")
        }
    };

    if json {
        let value = crate::output::test_runner_report::build_test_runner_json(&result);
        crate::output::print_json_value(value, perf.build_report().as_ref());
        return Ok(());
    }

    if matches!(result, AggregateResult::Empty) {
        // No declared runner. Emit nothing on stdout; hint on stderr when not plain.
        if !plain {
            eprintln!(
                "{}",
                Prose::new("<dim>No declared test runner for this context.</dim>")
                    .render(&Terminal::default())
            );
        }
        perf.emit_stderr(None);
        std::process::exit(1);
    }

    let term = Terminal::default();
    let display = |usage: &TestRunnerUsage| usage.runner.info().display_name;

    let rendered = match &result {
        AggregateResult::Singular(usage) => {
            if args.csv || args.list {
                format!("{}\n", display(usage))
            } else if args.md {
                format!("- {}\n", display(usage))
            } else {
                crate::output::test_runner_report::render_singular(usage, verbose, &term)
            }
        }
        AggregateResult::Multiple(usages) => {
            let names: Vec<&str> = usages.iter().map(display).collect();
            if args.csv {
                format!("{}\n", names.join(", "))
            } else if args.list {
                let mut out = String::new();
                for name in names {
                    use std::fmt::Write;
                    let _ = writeln!(out, "{name}");
                }
                out
            } else if args.md {
                let mut out = String::new();
                for name in names {
                    use std::fmt::Write;
                    let _ = writeln!(out, "- {name}");
                }
                out
            } else {
                crate::output::test_runner_report::render_multiple(usages, scope_kind, verbose, &term)
            }
        }
        AggregateResult::Empty => unreachable!("handled above"),
    };

    crate::output::emit_text(&rendered, plain);
    perf.emit_stderr(None);
    Ok(())
}

/// Handle `sniff repo dirty-source-code`, `staged-source-code`, `unstaged-source-code`,
/// and `dirty-files` commands.
pub(super) fn handle_file_list_command(
    args: &FileListArgs,
    scope: ChangeScope,
    kind: ChangedPathKind,
    json: bool,
    plain: bool,
    base_dir: Option<&std::path::Path>,
    perf: &CliPerf,
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
        return handle_no_results(args.no_error, &args.on_error, plain, perf);
    }

    if json {
        let json_val = crate::output::repo_json::file_list_value(scope, kind, &result.paths);
        output::print_json_value(json_val, perf.build_report().as_ref());
    } else {
        let format = path_list_format(args);
        let rendered =
            output::render_path_list(&result.repo_root, &result.paths, format, args.no_path);
        output::emit_text(&rendered, plain);
        perf.emit_stderr(None);
    }

    Ok(())
}
