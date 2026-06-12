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
        let json_val = serde_json::json!({
            "scope": scope,
            "kind": kind,
            "paths": result.paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });
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
