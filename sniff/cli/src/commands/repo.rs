use sniff::filesystem::blast_radius::{
    ChangeScope, ChangedPathKind, ChangedPathQuery, collect_changed_paths,
};
use sniff::filesystem::repo::{detect_repo_structure, detect_repo_structure_or_root_package};

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

    // A standalone single-package project (a `Cargo.toml` with no `[workspace]`,
    // a lone `package.json`, etc.) has no workspace structure but is still one
    // package. Synthesize its root-package catalog so `repo packages` lists it
    // instead of erroring, matching the repo-wide `package_manager` /
    // `dependencies` facts.
    let info = match detect_repo_structure_or_root_package(&root)? {
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

/// Subcommand-specific args for `sniff repo version`.
pub(super) struct RepoVersionArgs {
    pub(super) csv: bool,
    pub(super) list: bool,
    pub(super) md: bool,
    pub(super) all: bool,
    pub(super) package: Option<String>,
    pub(super) package_area: Option<String>,
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
        let sync = match (branch.ahead, branch.behind) {
            (Some(ahead), Some(behind)) if ahead > 0 || behind > 0 => {
                format!(" <dim>ahead {ahead} behind {behind}</dim>")
            }
            _ => String::new(),
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
        AggregateScope, collect_external_dependencies, detect_repo_with_request_or_root_package,
        resolve_scope,
    };
    use sniff::request::{RepoDetailRequest, RepoRequest};

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let explicit = base_dir.unwrap_or(&cwd);
    let root = if base_dir.is_some() {
        explicit.to_path_buf()
    } else {
        sniff::filesystem::repo_root(explicit)?.unwrap_or_else(|| explicit.to_path_buf())
    };
    let request = RepoRequest::focused(RepoDetailRequest::dependencies());
    let Some(info) = detect_repo_with_request_or_root_package(&root, &request)? else {
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
        AggregateResult, AggregateScope, aggregate_package_values,
        detect_repo_with_request_or_root_package, resolve_scope,
    };
    use sniff::request::{RepoDetailRequest, RepoRequest};

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let explicit = base_dir.unwrap_or(&cwd);
    let root = if base_dir.is_some() {
        explicit.to_path_buf()
    } else {
        sniff::filesystem::repo_root(explicit)?.unwrap_or_else(|| explicit.to_path_buf())
    };

    let request = RepoRequest::focused(RepoDetailRequest::package_managers());
    let info = detect_repo_with_request_or_root_package(&root, &request)?;
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
        TestRunnerAttribution, aggregate_test_runners, detect_repo_with_request,
        detect_test_runners_for_dir, resolve_scope,
    };
    use sniff::request::{RepoDetailRequest, RepoRequest};

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let explicit = base_dir.unwrap_or(&cwd);
    let root = if base_dir.is_some() {
        explicit.to_path_buf()
    } else {
        sniff::filesystem::repo_root(explicit)?.unwrap_or_else(|| explicit.to_path_buf())
    };

    let request = RepoRequest::focused(RepoDetailRequest::test_runners());
    let info = detect_repo_with_request(&root, &request)?;

    // Resolve the scope relative to the CWD so the command answers
    // "what does THIS directory declare?" rather than always the repo root.
    let dir_for_scope = if base_dir.is_some() { explicit } else { &cwd };

    // Collapse to distinct runner usages, each carrying its attributing
    // packages. Two paths:
    //   1. Monorepo with packages → aggregate across the resolved scope.
    //   2. Non-monorepo / no packages → detect at the directory directly
    //      (no per-package attribution to carry).
    let entries: Vec<TestRunnerAttribution> = match &info {
        Some(info) if info.is_monorepo && info.packages.as_ref().is_some_and(|p| !p.is_empty()) => {
            let packages = info.packages.as_deref().expect("non-empty packages");
            let scope = resolve_scope(info, dir_for_scope);
            aggregate_test_runners(packages, &scope)
        }
        _ => detect_test_runners_for_dir(&root)
            .into_iter()
            .map(|usage| TestRunnerAttribution {
                usage,
                packages: Vec::new(),
            })
            .collect(),
    };

    if json {
        let value = crate::output::test_runner_report::build_test_runner_json(&entries, &root);
        crate::output::print_json_value(value, perf.build_report().as_ref());
        return Ok(());
    }

    if entries.is_empty() {
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

    let rendered = if args.csv || args.list || args.md {
        // `--csv`/`--list`/`--md` select the delimiter. Without -v: distinct
        // runner names only. With -v: each item keeps the same styled
        // evidence/attribution the default CSV shows (`--plain` strips styling).
        let items: Vec<String> = if verbose > 0 {
            let multi = entries.len() > 1;
            let term = Terminal::default();
            entries
                .iter()
                .map(|entry| {
                    crate::output::test_runner_report::render_one(
                        entry, verbose, multi, &root, &term,
                    )
                })
                .collect()
        } else {
            crate::output::test_runner_report::entry_names(&entries)
                .into_iter()
                .map(str::to_string)
                .collect()
        };
        let mut out = String::new();
        use std::fmt::Write;
        if args.csv {
            let _ = writeln!(out, "{}", items.join(", "));
        } else {
            let prefix = if args.md { "- " } else { "" };
            for item in &items {
                let _ = writeln!(out, "{prefix}{item}");
            }
        }
        out
    } else {
        crate::output::test_runner_report::render_entries(
            &entries,
            verbose,
            &root,
            &Terminal::default(),
        )
    };

    crate::output::emit_text(&rendered, plain);
    perf.emit_stderr(None);
    Ok(())
}

/// Handle `sniff repo version`.
///
/// Reports declared package versions for the current repo/package context
/// using the shared library aggregation helper. Mirrors
/// [`handle_repo_test_runner`]: monorepo-with-packages collapses via
/// `aggregate_versions`; non-monorepo / no catalog falls back to reading the
/// directory's own manifest into a single attribution (empty `packages`,
/// one source). `--json` always emits the `{ "versions": [...] }` shape;
/// `--on-error` is text-mode only.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_repo_version(
    base_dir: Option<&std::path::Path>,
    args: RepoVersionArgs,
    json: bool,
    plain: bool,
    verbose: u8,
    no_error: bool,
    on_error: Option<String>,
    perf: &CliPerf,
) -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;
    use sniff::filesystem::repo::{
        VersionAttribution, aggregate_versions, detect_repo_structure_or_root_package,
        resolve_directory_version, resolve_scope_with_overrides,
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    // `--base` (or the CWD) is the *scope* directory, not the detection root.
    // Always discover the enclosing repo root from it so `--all` /
    // `--package` / `--package-area` select across the whole repo even when
    // `--base` points inside a package directory of a monorepo.
    let dir_for_scope = base_dir.unwrap_or(&cwd);
    let root = sniff::filesystem::repo_root(dir_for_scope)?
        .unwrap_or_else(|| dir_for_scope.to_path_buf());

    let info = detect_repo_structure_or_root_package(&root)?;

    // Resolve scope (with overrides) and collapse to distinct version entries.
    // Two paths:
    //   1. Any repo with a package catalog → resolve overrides and aggregate
    //      across the resolved scope. Running through the catalog regardless of
    //      `is_monorepo` is what validates `--package` / `--package-area`
    //      against a synthesized single-package repo.
    //   2. No catalog at all → detect at the directory directly (no
    //      per-package attribution to carry).
    let entries: Vec<VersionAttribution> = match &info {
        Some(info) if info.packages.as_ref().is_some_and(|p| !p.is_empty()) => {
            let scope = resolve_scope_with_overrides(
                info,
                dir_for_scope,
                args.all,
                args.package.as_deref(),
                args.package_area.as_deref(),
            )?;
            let packages = info.packages.as_deref().expect("non-empty packages");
            aggregate_versions(packages, &scope, &root)
        }
        _ => resolve_directory_version(&root).into_iter().collect(),
    };

    if json {
        let value = crate::output::version_report::build_version_json(&entries, &root);
        crate::output::print_json_value(value, perf.build_report().as_ref());
        // JSON mode keeps stdout as valid JSON even on empty; exit code
        // follows the same `--no-error` contract as text mode.
        if entries.is_empty() {
            if no_error {
                return Ok(());
            }
            std::process::exit(1);
        }
        return Ok(());
    }

    if entries.is_empty() {
        // No resolvable version. Emit nothing on stdout; hint on stderr when
        // not plain. `--on-error` text is honored; `--no-error` keeps exit 0.
        if let Some(msg) = on_error.as_deref() {
            let terminal = Terminal::default();
            let rendered = Prose::new(msg.to_string()).render(&terminal);
            let text = if plain {
                biscuit_terminal::prelude::strip_escape_codes(&rendered)
            } else {
                rendered
            };
            eprintln!("{text}");
        } else if !plain {
            eprintln!(
                "{}",
                Prose::new("<dim>No declared version for this context.</dim>")
                    .render(&Terminal::default())
            );
        }
        perf.emit_stderr(None);
        std::process::exit(if no_error { 0 } else { 1 });
    }

    let rendered = if args.csv || args.list || args.md {
        // `--csv`/`--list`/`--md` select the delimiter. Without -v: distinct
        // version strings only. With -v: each item keeps the same styled
        // source attribution the default CSV shows (`--plain` strips styling).
        let items: Vec<String> = if verbose > 0 {
            let multi = entries.len() > 1;
            let term = Terminal::default();
            entries
                .iter()
                .map(|entry| {
                    crate::output::version_report::render_one(entry, verbose, multi, &root, &term)
                })
                .collect()
        } else {
            crate::output::version_report::entry_names(&entries)
                .into_iter()
                .map(str::to_string)
                .collect()
        };
        let mut out = String::new();
        use std::fmt::Write;
        if args.csv {
            let _ = writeln!(out, "{}", items.join(", "));
        } else {
            let prefix = if args.md { "- " } else { "" };
            for item in &items {
                let _ = writeln!(out, "{prefix}{item}");
            }
        }
        out
    } else {
        crate::output::version_report::render_entries(
            &entries,
            verbose,
            &root,
            &Terminal::default(),
        )
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
