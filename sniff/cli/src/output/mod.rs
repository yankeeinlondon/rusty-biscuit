//! Output formatting for sniff CLI.
//!
//! This module handles text and JSON output formatting for all detection results.
//! Each major section (OS, Hardware, Network, Filesystem, Programs, Services)
//! has its own submodule.

mod commit_blocks;
mod filesystem;
mod hardware;
mod just;
mod network;
mod notification_helpers;
mod os;
mod programs;
pub(crate) mod recent_commits;
mod remote;
mod render;
pub(crate) mod repo_json;
mod services;
pub(crate) mod test_runner_report;
mod test_runners;
mod topics;
pub(crate) mod version_report;

use sniff::{PerformanceReport, SniffResult};

use crate::args::{DocsFilter, FilesFilter, RepoAction};

/// Split-stream output for commands that write to both stdout and stderr.
pub struct TextOutput {
    pub stdout: String,
    pub stderr: String,
}

pub use filesystem::{
    PathListFormat, render_docs_output, render_git_section, render_hash_section, render_path_list,
};
pub use just::{filter_justfiles_for_json, render_just_text};
pub use notification_helpers::{
    print_notification_helpers_json, render_notification_helpers_markdown,
};
pub use programs::{build_programs_json, render_programs_markdown};
pub use remote::{
    print_remote_json, render_cicd, render_pull_requests_empty, render_pull_requests_table,
    render_pull_requests_verbose, render_remote_text,
};
pub use services::{print_services_json, render_services_text};
pub use test_runners::{
    HintMode, build_test_runners_json, render_test_runners_markdown, test_runners_search_hint,
};
pub use topics::render_topics_table;

// Re-export types needed by submodules
pub(crate) use filesystem::format_monorepo_label;
pub(crate) use filesystem::{
    collect_repo_package_area_names, collect_repo_package_names, print_current_package_area_dirty,
    print_package_area_has_source_code_changes, render_dirty_package_areas, render_dirty_packages,
    render_files_section, render_filesystem_section, render_language_section, render_repo_area,
    render_repo_default_verbose, render_repo_deps_svg, render_repo_deps_text,
    render_repo_deps_visual, render_repo_language, render_repo_name, render_repo_package,
    render_repo_package_area, render_repo_package_area_root, render_repo_package_areas_formatted,
    render_repo_package_root, render_repo_packages_formatted, render_repo_root,
    render_repo_section, render_staged_package_areas, render_staged_packages,
    render_unstaged_package_areas, render_unstaged_packages,
};
pub(crate) use hardware::{
    render_audio_devices_section, render_cpu_section, render_gpu_section, render_hardware_section,
    render_memory_section, render_storage_section,
};
pub(crate) use network::render_network_section;
pub(crate) use os::{render_os_section, render_runtime_environment};

/// Filter mode for output - determines which subsection to display.
///
/// When a single top-level section is requested (Os, Hardware, Network, Filesystem),
/// the JSON output is flattened - the section's fields appear at the top level without
/// a wrapper object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFilter {
    /// Show all sections (no filtering)
    #[default]
    All,
    /// Show only OS section (flattened in JSON)
    Os,
    /// Show only hardware section (flattened in JSON)
    Hardware,
    /// Show only network section (flattened in JSON)
    Network,
    /// Show only filesystem section (flattened in JSON)
    Filesystem,
    /// Show only CPU info (hardware subsection, flattened in JSON)
    Cpu,
    /// Show only GPU info (hardware subsection, flattened in JSON)
    Gpu,
    /// Show only memory info (hardware subsection, flattened in JSON)
    Memory,
    /// Show only storage info (hardware subsection, flattened in JSON)
    Storage,
    /// Show only audio devices (hardware subsection, flattened in JSON)
    AudioDevices,
    /// Show only repo/monorepo info (filesystem subsection, flattened in JSON)
    Repo,
    /// Show only broad file associations (filesystem subsection, flattened in JSON)
    Files,
    /// Show only markdown document metadata (filesystem subsection, flattened in JSON)
    Docs,
    /// Show only programs info (installed programs detection)
    Programs,
    /// Show only editors (programs subsection)
    Editors,
    /// Show only utilities (programs subsection)
    Utilities,
    /// Show only language package managers (programs subsection)
    LanguagePackageManagers,
    /// Show only OS package managers (programs subsection)
    OsPackageManagers,
    /// Show only TTS clients (programs subsection)
    TtsClients,
    /// Show only terminal apps (programs subsection)
    TerminalApps,
    /// Show only headless audio players (programs subsection)
    HeadlessAudio,
    /// Show only AI CLI tools (programs subsection)
    AiClients,
    /// Show only desktop notification helpers (programs subsection)
    NotificationHelpers,
    /// Show resolved test runners with availability discriminators
    /// (programs subsection). Distinct from the other categories because
    /// runners live in many places (PATH, project-local bin, parent binary).
    TestRunners,
    /// Show only system services (init system and service list)
    Services,
    /// Show justfiles and their recipes
    Just,
    /// Show documents whose blast_radius intersects with changed files
    BlastRadius,
}

// ============================================================================
// Shared utility functions
// ============================================================================

pub use render::render_performance_section;
pub(crate) use render::{format_bytes, format_number, format_uptime, relative_path};

// ============================================================================
// Docs filtering
// ============================================================================

/// Apply docs filter flags to a list of markdown documents.
///
/// When multiple flags are combined, results are intersected (AND logic).
/// Within `--package-area` and `--package`, values are OR'd.
/// When no flags are set, all documents are returned.
pub fn filter_docs(
    docs: &[sniff::filesystem::docs::MarkdownMeta],
    filter: &DocsFilter,
) -> Vec<sniff::filesystem::docs::MarkdownMeta> {
    if !filter.readme
        && !filter.plan
        && !filter.src
        && !filter.has_prompt
        && !filter.blast_radius
        && filter.package_area.is_empty()
        && filter.package.is_empty()
        && filter.filter.is_empty()
    {
        return docs.to_vec();
    }

    docs.iter()
        .filter(|doc| {
            let path_lower = doc.relative.to_lowercase();

            if filter.readme && !path_lower.ends_with("/readme.md") && path_lower != "readme.md" {
                return false;
            }
            if filter.plan && !path_lower.contains("plan") {
                return false;
            }
            if filter.src && !path_lower.contains("/src/") {
                return false;
            }
            if filter.has_prompt && doc.prompt.is_none() {
                return false;
            }
            if filter.blast_radius && !doc.has_blast_radius {
                return false;
            }
            if !filter.package_area.is_empty()
                && !filter.package_area.iter().any(|area| {
                    let area_lower = area.to_lowercase();
                    let prefix = format!("{}/", area_lower);
                    path_lower == area_lower || path_lower.starts_with(&prefix)
                })
            {
                return false;
            }
            if !filter.package.is_empty()
                && !filter.package.iter().any(|name| {
                    doc.package
                        .as_ref()
                        .is_some_and(|p| p.eq_ignore_ascii_case(name))
                })
            {
                return false;
            }
            if !filter.filter.is_empty()
                && !filter
                    .filter
                    .iter()
                    .any(|s| path_lower.contains(&s.to_lowercase()))
            {
                return false;
            }

            true
        })
        .cloned()
        .collect()
}

// ============================================================================
// Main render/emit functions
// ============================================================================

/// Emit rendered text to stdout, optionally stripping ANSI escape codes.
pub fn emit_text(text: &str, plain: bool) {
    if plain {
        print!("{}", biscuit_terminal::prelude::strip_escape_codes(text));
    } else {
        print!("{text}");
    }
}

/// Emit rendered text to stderr, optionally stripping ANSI escape codes.
pub fn emit_stderr(text: &str, plain: bool) {
    if plain {
        eprint!("{}", biscuit_terminal::prelude::strip_escape_codes(text));
    } else {
        eprint!("{text}");
    }
}

/// Render all text output for a given filter mode into a single String.
///
/// This is the central render function that delegates to per-section renderers.
/// The caller is responsible for emitting the result (via `emit_text`).
///
/// Some `RepoAction` variants trigger side effects (e.g., `std::process::exit`)
/// and cannot be rendered to a string — these are handled inline.
#[allow(clippy::too_many_arguments)]
pub fn render_text(
    result: &SniffResult,
    verbose: u8,
    filter: OutputFilter,
    history_count: usize,
    docs_filter: &DocsFilter,
    files_filter: &FilesFilter,
    repo_action: Option<&RepoAction>,
    base_dir: Option<&std::path::Path>,
    latest_versions_requested: bool,
) -> String {
    // Get repo root for relative paths
    let repo_root = result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.git.as_ref())
        .map(|git| git.repo_root.as_path());

    let mut out = String::new();

    match filter {
        OutputFilter::All => {
            if let Some(ref os) = result.os {
                out.push_str(&render_os_section(os, verbose));
            }
            if let Some(ref hardware) = result.hardware {
                out.push_str(&render_hardware_section(hardware, verbose, repo_root));
            }
            if let Some(ref network) = result.network {
                out.push_str(&render_network_section(network, verbose));
            }
            if let Some(ref filesystem) = result.filesystem {
                out.push_str(&render_filesystem_section(
                    filesystem,
                    verbose,
                    repo_root,
                    latest_versions_requested,
                ));
            }
        }
        OutputFilter::Os => {
            if let Some(ref os) = result.os {
                out.push_str(&render_os_section(os, verbose));
            }
        }
        OutputFilter::Hardware => {
            if let Some(ref hardware) = result.hardware {
                out.push_str(&render_hardware_section(hardware, verbose, repo_root));
            }
        }
        OutputFilter::Network => {
            if let Some(ref network) = result.network {
                out.push_str(&render_network_section(network, verbose));
            }
        }
        OutputFilter::Filesystem => {
            if let Some(ref filesystem) = result.filesystem {
                out.push_str(&render_filesystem_section(
                    filesystem,
                    verbose,
                    repo_root,
                    latest_versions_requested,
                ));
            }
        }
        OutputFilter::Cpu => {
            if let Some(ref hardware) = result.hardware {
                out.push_str(&render_cpu_section(&hardware.cpu, verbose));
            }
        }
        OutputFilter::Gpu => {
            if let Some(ref hardware) = result.hardware {
                out.push_str(&render_gpu_section(&hardware.gpu, verbose));
            }
        }
        OutputFilter::Memory => {
            if let Some(ref hardware) = result.hardware {
                out.push_str(&render_memory_section(&hardware.memory));
            }
        }
        OutputFilter::Storage => {
            if let Some(ref hardware) = result.hardware {
                out.push_str(&render_storage_section(
                    &hardware.storage,
                    verbose,
                    repo_root,
                ));
            }
        }
        OutputFilter::AudioDevices => {
            if let Some(ref hardware) = result.hardware {
                out.push_str(&render_audio_devices_section(
                    &hardware.audio_devices,
                    verbose,
                ));
            }
        }
        OutputFilter::Repo => {
            match repo_action {
                Some(RepoAction::Package { .. }) => {
                    unreachable!("Package is handled as an early return in commands.rs")
                }
                Some(RepoAction::PackageArea { .. }) => {
                    unreachable!("PackageArea is handled as an early return in commands.rs")
                }
                Some(RepoAction::Worktree { .. }) => {
                    unreachable!("Worktree is handled as an early return in commands.rs")
                }
                Some(RepoAction::Packages { .. }) => {
                    unreachable!("Packages is handled as an early return in commands.rs")
                }
                Some(RepoAction::PackageAreas { .. }) => {
                    unreachable!("PackageAreas is handled as an early return in commands.rs")
                }
                Some(RepoAction::DirtyPackages {
                    filter,
                    package,
                    package_area,
                }) => {
                    let rendered = render_dirty_packages(
                        result,
                        filter,
                        package.as_deref(),
                        package_area.as_deref(),
                    );
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::DirtyPackageAreas {
                    filter,
                    package,
                    package_area,
                }) => {
                    let rendered = render_dirty_package_areas(
                        result,
                        filter,
                        package.as_deref(),
                        package_area.as_deref(),
                    );
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::StagedPackages {
                    filter,
                    package,
                    package_area,
                }) => {
                    let rendered = render_staged_packages(
                        result,
                        filter,
                        package.as_deref(),
                        package_area.as_deref(),
                    );
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::StagedPackageAreas {
                    filter,
                    package,
                    package_area,
                }) => {
                    let rendered = render_staged_package_areas(
                        result,
                        filter,
                        package.as_deref(),
                        package_area.as_deref(),
                    );
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::UnstagedPackages {
                    filter,
                    package,
                    package_area,
                }) => {
                    let rendered = render_unstaged_packages(
                        result,
                        filter,
                        package.as_deref(),
                        package_area.as_deref(),
                    );
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::UnstagedPackageAreas {
                    filter,
                    package,
                    package_area,
                }) => {
                    let rendered = render_unstaged_package_areas(
                        result,
                        filter,
                        package.as_deref(),
                        package_area.as_deref(),
                    );
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::PackageDependencies {
                    ui,
                    svg,
                    filter,
                    package,
                    package_area,
                    width,
                    orientation,
                }) => {
                    if let Some(ref filesystem) = result.filesystem
                        && let Some(ref repo) = filesystem.repo
                    {
                        if *svg {
                            out.push_str(&render_repo_deps_svg(
                                repo,
                                filter,
                                package.as_deref(),
                                package_area.as_deref(),
                                orientation.as_deref(),
                            ));
                        } else if *ui {
                            out.push_str(&render_repo_deps_visual(
                                repo,
                                filter,
                                package.as_deref(),
                                package_area.as_deref(),
                                width.as_deref(),
                                orientation.as_deref(),
                            ));
                        } else {
                            out.push_str(&render_repo_deps_text(
                                repo,
                                filter,
                                package.as_deref(),
                                package_area.as_deref(),
                            ));
                        }
                    }
                }
                Some(RepoAction::PackageRoot) => {
                    let rendered = render_repo_package_root(result, base_dir);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::PackageAreaRoot) => {
                    let rendered = render_repo_package_area_root(result, base_dir);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::Root) => {
                    let rendered = render_repo_root(result);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::IsCurrentPackageAreaDirty) => {
                    // Side-effect only: calls std::process::exit
                    print_current_package_area_dirty(result, base_dir);
                }
                Some(RepoAction::PackageAreaHasSourceCodeChanges) => {
                    // Side-effect only: calls std::process::exit
                    print_package_area_has_source_code_changes(result, base_dir, verbose);
                }
                Some(RepoAction::GitStatus {
                    compact,
                    branch,
                    worktree,
                    ..
                }) => {
                    if let Some(ref filesystem) = result.filesystem
                        && let Some(ref git) = filesystem.git
                    {
                        // `--worktree` takes precedence; the worktree handler
                        // upstream already replaced `current_branch` with the
                        // worktree's branch, so use that for the heading.
                        let target_worktree =
                            worktree.as_deref().zip(git.current_branch.as_deref());

                        // Annotate the Status heading only when the user
                        // explicitly named a branch different from the
                        // currently checked-out one. `--branch` with no
                        // value (Some(None)) resolves to current and is a
                        // no-op for both data and heading.
                        let target_branch = match branch {
                            Some(Some(name))
                                if !name.is_empty()
                                    && git.current_branch.as_deref() != Some(name.as_str()) =>
                            {
                                Some(name.as_str())
                            }
                            _ => None,
                        };
                        out.push_str(&render_git_section(
                            git,
                            history_count,
                            verbose,
                            *compact,
                            target_branch,
                            target_worktree,
                        ));
                    }
                }
                Some(RepoAction::Language { breakdown: true }) => {
                    out.push_str(&render_language_section(result, verbose, base_dir));
                }
                Some(RepoAction::Language { breakdown: false }) => {
                    let rendered = render_repo_language(result, base_dir);
                    if rendered.is_empty() {
                        // Mirror locator family: empty == not detected → exit 1, no stdout.
                        // JSON path emits `{ "language": null }` with the same exit code.
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                }
                Some(RepoAction::Structure {
                    filter,
                    package,
                    package_area,
                    ..
                }) => {
                    if let Some(ref filesystem) = result.filesystem
                        && let Some(ref repo) = filesystem.repo
                    {
                        out.push_str(&render_repo_section(
                            repo,
                            verbose,
                            repo_root,
                            filter,
                            package.as_deref(),
                            package_area.as_deref(),
                            latest_versions_requested,
                        ));
                    }
                }
                None => {
                    if let Some(ref filesystem) = result.filesystem
                        && let Some(ref repo) = filesystem.repo
                    {
                        out.push_str(&render_repo_section(
                            repo,
                            verbose,
                            repo_root,
                            &[],
                            None,
                            None,
                            latest_versions_requested,
                        ));
                    }
                }
                _ => {
                    // Hash, Remote, StagedFiles, UnstagedFiles, UntrackedFiles
                    // are handled as early returns in commands.rs
                }
            }
        }
        OutputFilter::Files => {
            if let Some(ref filesystem) = result.filesystem
                && let Some(ref files) = filesystem.files
            {
                out.push_str(&render_files_section(files, verbose, files_filter));
            }
        }
        OutputFilter::Docs => {
            // Docs is handled as an early return in commands.rs with split-stream output.
            // This branch is kept for the All filter case.
            if let Some(ref filesystem) = result.filesystem
                && let Some(ref docs) = filesystem.docs
            {
                let filtered = filter_docs(docs, docs_filter);
                let text_output = filesystem::render_docs_output(&filtered, verbose);
                out.push_str(&text_output.stderr);
                out.push_str(&text_output.stdout);
            }
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
        | OutputFilter::TestRunners
        | OutputFilter::Services
        | OutputFilter::Just
        | OutputFilter::BlastRadius => {
            unreachable!(
                "Programs, Services, Just, BlastRadius, and Remote filters should be handled separately"
            )
        }
    }

    out
}

/// Apply output filter to create a custom JSON value with only the requested fields.
///
/// This ensures that filter flags like --cpu, --gpu work consistently for both
/// text and JSON output modes, and that filtered JSON only contains the relevant fields.
///
/// For subsection filters (--cpu, --gpu, --memory, --storage, --git, --repo, --language),
/// the output is flattened to the top level without the parent container.
///
/// ## Returns
///
/// A tuple of `(value, exit_code)`. `exit_code` is `None` for almost every
/// filter; only the boolean / locator repo-action arms set it (so the caller
/// can `std::process::exit(code)` after the JSON is flushed).
fn apply_filter_to_json(
    result: &SniffResult,
    filter: OutputFilter,
    docs_filter: &DocsFilter,
    files_filter: &FilesFilter,
    repo_action: Option<&RepoAction>,
    base_dir: Option<&std::path::Path>,
) -> (serde_json::Value, Option<i32>) {
    use serde_json::{Value, json};

    let value = match filter {
        OutputFilter::All => {
            // No filtering - serialize everything
            serde_json::to_value(result).unwrap_or(Value::Null)
        }
        OutputFilter::Os => {
            // Flatten: return OS fields at top level (name, version, kernel, etc.)
            if let Some(ref os) = result.os {
                serde_json::to_value(os).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Network => {
            // Flatten: return network fields at top level (interfaces, primary_interface, etc.)
            if let Some(ref network) = result.network {
                serde_json::to_value(network).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Hardware => {
            // Flatten: return hardware fields at top level (cpu, gpu, memory, storage)
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(hw).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Filesystem => {
            // Flatten: return filesystem fields at top level (git, languages, repo, formatting)
            if let Some(ref fs) = result.filesystem {
                serde_json::to_value(fs).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Cpu => {
            // Flatten: return CPU data at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.cpu).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Gpu => {
            // Flatten: return GPU array at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.gpu).unwrap_or(Value::Null)
            } else {
                json!([])
            }
        }
        OutputFilter::Memory => {
            // Flatten: return memory data at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.memory).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Storage => {
            // Flatten: return storage array at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.storage).unwrap_or(Value::Null)
            } else {
                json!([])
            }
        }
        OutputFilter::AudioDevices => {
            // Flatten: return audio devices array at top level
            if let Some(ref hw) = result.hardware {
                serde_json::to_value(&hw.audio_devices).unwrap_or(Value::Null)
            } else {
                json!([])
            }
        }
        OutputFilter::Repo => {
            // Dispatch on RepoAction so each subcommand can return a focused
            // JSON shape. Locator and boolean families also surface an
            // explicit exit code; we capture both and return-early below.
            let outcome = repo_json::build_with_outcome(result, repo_action, base_dir);
            return (outcome.value, outcome.exit_code);
        }
        OutputFilter::Files => {
            if let Some(ref fs) = result.filesystem
                && let Some(ref files) = fs.files
            {
                let filtered = filesystem::filter_file_breakdown(files, files_filter);
                serde_json::to_value(&filtered).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Docs => {
            // Flatten: return docs array at top level
            if let Some(ref fs) = result.filesystem
                && let Some(ref docs) = fs.docs
            {
                let filtered = filter_docs(docs, docs_filter);
                serde_json::to_value(&filtered).unwrap_or(json!([]))
            } else {
                json!([])
            }
        }
        // Programs, Services, and Just filters are handled separately
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
        | OutputFilter::TestRunners
        | OutputFilter::Services
        | OutputFilter::Just
        | OutputFilter::BlastRadius => {
            unreachable!(
                "Programs, Services, Just, and BlastRadius filters should be handled separately"
            )
        }
    };
    (value, None)
}

pub fn attach_performance(
    mut filtered_json: serde_json::Value,
    performance: &PerformanceReport,
) -> serde_json::Value {
    let performance_json = serde_json::to_value(performance).unwrap_or(serde_json::Value::Null);
    match &mut filtered_json {
        serde_json::Value::Object(map) => {
            map.insert("performance".to_string(), performance_json);
            filtered_json
        }
        _ => serde_json::json!({
            "data": filtered_json,
            "performance": performance_json,
        }),
    }
}

fn attach_performance_from_result(
    filtered_json: serde_json::Value,
    result: &SniffResult,
) -> serde_json::Value {
    match result.performance.as_ref() {
        Some(perf) => attach_performance(filtered_json, perf),
        None => filtered_json,
    }
}

/// Print a JSON value to stdout, optionally injecting performance data.
///
/// When `performance` is `Some`, the data is injected into the JSON value
/// (as a sibling field for objects, or wrapped as `{ data, performance }`
/// for non-objects), matching the behavior of [`attach_performance`].
pub fn print_json_value(value: serde_json::Value, performance: Option<&PerformanceReport>) {
    let output = match performance {
        Some(perf) => attach_performance(value, perf),
        None => value,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
}

/// Print a filtered JSON view of `result` to stdout.
///
/// ## Returns
///
/// `Ok(Some(code))` when the underlying repo-action JSON wants the process
/// to exit with `code` after stdout has been flushed (boolean and locator
/// families). `Ok(None)` when no special exit code is needed and the caller
/// can return normally from `main`.
pub fn print_json(
    result: &SniffResult,
    filter: OutputFilter,
    docs_filter: &DocsFilter,
    files_filter: &FilesFilter,
    repo_action: Option<&RepoAction>,
    base_dir: Option<&std::path::Path>,
) -> serde_json::Result<Option<i32>> {
    let (filtered, exit_code) = apply_filter_to_json(
        result,
        filter,
        docs_filter,
        files_filter,
        repo_action,
        base_dir,
    );
    let with_perf = attach_performance_from_result(filtered, result);
    println!("{}", serde_json::to_string_pretty(&with_perf)?);
    Ok(exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sniff::{PerformanceReport, performance::PerformanceStage};

    mod docs_filter {
        use super::*;
        use chrono::Utc;
        use sniff::filesystem::docs::MarkdownMeta;
        use std::path::PathBuf;

        fn make_doc(relative: &str) -> MarkdownMeta {
            MarkdownMeta {
                filepath: PathBuf::from(relative),
                relative: relative.to_string(),
                package: None,
                title: String::new(),
                title_source: sniff::filesystem::TitleSource::None,
                model: None,
                prompt: None,
                last_updated: Utc::now(),
                updated_source: sniff::filesystem::UpdatedSource::FileMetadata,
                content_hash: String::new(),
                has_blast_radius: false,
                blast_radius: None,
                frontmatter_keys: Vec::new(),
            }
        }

        fn make_doc_with_prompt(relative: &str, prompt: &str) -> MarkdownMeta {
            let mut doc = make_doc(relative);
            doc.prompt = Some(prompt.to_string());
            doc
        }

        fn make_doc_with_package(relative: &str, package: &str) -> MarkdownMeta {
            let mut doc = make_doc(relative);
            doc.package = Some(package.to_string());
            doc
        }

        fn sample_docs() -> Vec<MarkdownMeta> {
            vec![
                make_doc("README.md"),
                make_doc("sniff/README.md"),
                make_doc_with_package("sniff/lib/src/notes.md", "sniff-lib"),
                make_doc(".ai/plans/2026-02-07.plan-for-feature.md"),
                make_doc("homelab/docs/planning.md"),
                make_doc_with_package("darkmatter/lib/src/README.md", "darkmatter-lib"),
                make_doc_with_prompt("research/docs/overview.md", "Summarize this library"),
                make_doc_with_prompt("homelab/docs/setup.md", "How to configure"),
            ]
        }

        #[test]
        fn no_flags_returns_all() {
            let docs = sample_docs();
            let filter = DocsFilter::default();
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), docs.len());
        }

        #[test]
        fn readme_flag_filters_readme_files() {
            let docs = sample_docs();
            let filter = DocsFilter {
                readme: true,
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 3);
            assert!(
                result
                    .iter()
                    .all(|d| d.relative.to_lowercase().ends_with("/readme.md")
                        || d.relative.to_lowercase() == "readme.md")
            );
        }

        #[test]
        fn readme_flag_is_case_insensitive() {
            let docs = vec![
                make_doc("readme.md"),
                make_doc("pkg/Readme.md"),
                make_doc("other.md"),
            ];
            let filter = DocsFilter {
                readme: true,
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn plan_flag_matches_filename_and_path() {
            let docs = sample_docs();
            let filter = DocsFilter {
                plan: true,
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
            assert!(
                result
                    .iter()
                    .all(|d| d.relative.to_lowercase().contains("plan"))
            );
        }

        #[test]
        fn src_flag_matches_src_in_path() {
            let docs = sample_docs();
            let filter = DocsFilter {
                src: true,
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
            assert!(result.iter().all(|d| d.relative.contains("/src/")));
        }

        #[test]
        fn has_prompt_flag_filters_to_prompted_docs() {
            let docs = sample_docs();
            let filter = DocsFilter {
                has_prompt: true,
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
            assert!(result.iter().all(|d| d.prompt.is_some()));
        }

        #[test]
        fn positional_filter_matches_substring() {
            let docs = sample_docs();
            let filter = DocsFilter {
                filter: vec!["homelab".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
            assert!(
                result
                    .iter()
                    .all(|d| d.relative.to_lowercase().contains("homelab"))
            );
        }

        #[test]
        fn positional_filter_is_case_insensitive() {
            let docs = sample_docs();
            let filter = DocsFilter {
                filter: vec!["HOMELAB".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn has_prompt_with_positional_filter_intersects() {
            let docs = sample_docs();
            let filter = DocsFilter {
                has_prompt: true,
                filter: vec!["homelab".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            // Only homelab/docs/setup.md has both a prompt and "homelab" in path
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].relative, "homelab/docs/setup.md");
        }

        #[test]
        fn combined_flags_intersect() {
            let docs = sample_docs();
            let filter = DocsFilter {
                readme: true,
                src: true,
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            // Only darkmatter/lib/src/README.md matches both --readme and --src
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].relative, "darkmatter/lib/src/README.md");
        }

        #[test]
        fn blast_radius_flag_filters_to_blast_radius_docs() {
            let mut doc_with_br = make_doc("sniff/docs/cli/repo.md");
            doc_with_br.has_blast_radius = true;
            doc_with_br.blast_radius = Some(vec![PathBuf::from("sniff/cli/src/args.rs")]);

            let doc_without_br = make_doc("sniff/README.md");

            let mut doc_empty_br = make_doc("sniff/docs/overview.md");
            doc_empty_br.has_blast_radius = true;
            doc_empty_br.blast_radius = Some(Vec::new());

            let docs = vec![doc_with_br, doc_without_br, doc_empty_br];
            let filter = DocsFilter {
                blast_radius: true,
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            // Should return both docs that have blast_radius key, even the empty one
            assert_eq!(result.len(), 2);
            assert!(result.iter().all(|d| d.has_blast_radius));
            assert!(
                result
                    .iter()
                    .any(|d| d.relative == "sniff/docs/cli/repo.md")
            );
            assert!(
                result
                    .iter()
                    .any(|d| d.relative == "sniff/docs/overview.md")
            );
        }

        #[test]
        fn blast_radius_flag_with_filter_intersects() {
            let mut doc_br_homelab = make_doc("homelab/docs/api.md");
            doc_br_homelab.has_blast_radius = true;

            let mut doc_br_sniff = make_doc("sniff/docs/cli/repo.md");
            doc_br_sniff.has_blast_radius = true;

            let doc_no_br = make_doc("homelab/README.md");

            let docs = vec![doc_br_homelab, doc_br_sniff, doc_no_br];
            let filter = DocsFilter {
                blast_radius: true,
                filter: vec!["homelab".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].relative, "homelab/docs/api.md");
        }

        #[test]
        fn package_area_filters_by_path_prefix() {
            let docs = sample_docs();
            let filter = DocsFilter {
                package_area: vec!["sniff".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
            assert!(result.iter().all(|d| d.relative.starts_with("sniff/")));
        }

        #[test]
        fn package_area_or_logic_with_multiple_areas() {
            let docs = sample_docs();
            let filter = DocsFilter {
                package_area: vec!["sniff".to_string(), "homelab".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 4);
            assert!(result.iter().all(|d| {
                d.relative.starts_with("sniff/") || d.relative.starts_with("homelab/")
            }));
        }

        #[test]
        fn package_area_is_case_insensitive() {
            let docs = sample_docs();
            let filter = DocsFilter {
                package_area: vec!["SNIFF".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn package_filters_by_package_name() {
            let docs = sample_docs();
            let filter = DocsFilter {
                package: vec!["sniff-lib".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].relative, "sniff/lib/src/notes.md");
        }

        #[test]
        fn package_or_logic_with_multiple_packages() {
            let docs = sample_docs();
            let filter = DocsFilter {
                package: vec!["sniff-lib".to_string(), "darkmatter-lib".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn package_is_case_insensitive() {
            let docs = sample_docs();
            let filter = DocsFilter {
                package: vec!["SNIFF-LIB".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 1);
        }

        #[test]
        fn package_area_intersects_with_readme() {
            let docs = sample_docs();
            let filter = DocsFilter {
                package_area: vec!["sniff".to_string()],
                readme: true,
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].relative, "sniff/README.md");
        }

        #[test]
        fn package_excludes_docs_without_package() {
            let docs = sample_docs();
            let filter = DocsFilter {
                package: vec!["sniff-lib".to_string()],
                ..Default::default()
            };
            let result = filter_docs(&docs, &filter);
            assert!(result.iter().all(|d| d.package.is_some()));
        }
    }

    #[test]
    fn attach_performance_to_object_filter_output() {
        let result = SniffResult {
            os: None,
            hardware: None,
            network: None,
            filesystem: None,
            performance: Some(PerformanceReport {
                total_duration_ms: 12.5,
                stages: std::collections::BTreeMap::from([(
                    "detect.total".to_string(),
                    PerformanceStage {
                        calls: 1,
                        total_duration_ms: 12.5,
                        max_duration_ms: 12.5,
                        last_duration_ms: 12.5,
                    },
                )]),
                counters: std::collections::BTreeMap::from([("files".to_string(), 3)]),
            }),
        };

        let filtered =
            attach_performance_from_result(serde_json::json!({"name": "sniff"}), &result);
        assert_eq!(filtered["name"], "sniff");
        assert_eq!(filtered["performance"]["total_duration_ms"], 12.5);
    }

    #[test]
    fn attach_performance_wraps_non_object_output() {
        let result = SniffResult {
            os: None,
            hardware: None,
            network: None,
            filesystem: None,
            performance: Some(PerformanceReport {
                total_duration_ms: 8.0,
                stages: std::collections::BTreeMap::new(),
                counters: std::collections::BTreeMap::from([("files".to_string(), 2)]),
            }),
        };

        let filtered = attach_performance_from_result(serde_json::json!(["a", "b"]), &result);
        assert_eq!(filtered["data"], serde_json::json!(["a", "b"]));
        assert_eq!(filtered["performance"]["counters"]["files"], 2);
    }
}
