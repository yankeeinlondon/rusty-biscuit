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
mod os;
mod programs;
pub(crate) mod recent_commits;
mod remote;
mod services;
mod topics;

use std::path::Path;

use sniff::{PerformanceReport, SniffResult};

use crate::args::{DocsFilter, FilesFilter, RepoAction};

/// Split-stream output for commands that write to both stdout and stderr.
pub struct TextOutput {
    pub stdout: String,
    pub stderr: String,
}

pub use filesystem::{
    PathListFormat, render_docs_output, render_git_file_list, render_git_section,
    render_hash_section, render_path_list,
};
pub use just::{filter_justfiles_for_json, render_just_text};
pub use programs::{print_programs_json, render_programs_markdown};
pub use remote::{print_remote_json, render_remote_text};
pub use services::{print_services_json, render_services_text};
pub use topics::render_topics_table;

// Re-export types needed by submodules
pub(crate) use filesystem::{
    collect_repo_package_names, print_current_package_area_dirty,
    print_package_area_has_source_code_changes, render_dirty_package_areas, render_dirty_packages,
    render_files_section, render_filesystem_section, render_language_section,
    render_repo_deps_text, render_repo_deps_visual, render_repo_package, render_repo_package_area,
    render_repo_package_area_root, render_repo_package_root, render_repo_packages_formatted,
    render_repo_root, render_repo_section, render_staged_package_areas, render_staged_packages,
    render_unstaged_package_areas, render_unstaged_packages,
};
pub(crate) use hardware::{
    render_audio_devices_section, render_cpu_section, render_gpu_section, render_hardware_section,
    render_memory_section, render_storage_section,
};
pub(crate) use network::render_network_section;
pub(crate) use os::render_os_section;

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
    /// Show only language detection (filesystem subsection, flattened in JSON)
    Language,
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

/// Format bytes into human-readable units (KB, MB, GB, TB)
pub(crate) fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Format large numbers with comma separators
pub(crate) fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

/// Convert absolute path to relative path from repo root
pub(crate) fn relative_path(path: &Path, repo_root: Option<&Path>) -> String {
    if let Some(root) = repo_root
        && let Ok(rel) = path.strip_prefix(root)
    {
        return rel.display().to_string();
    }
    path.display().to_string()
}

/// Format uptime in seconds to a human-readable string
pub(crate) fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut parts = Vec::new();

    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }
    if hours > 0 {
        parts.push(format!(
            "{} hour{}",
            hours,
            if hours == 1 { "" } else { "s" }
        ));
    }
    if minutes > 0 || (days == 0 && hours == 0 && secs == 0) {
        parts.push(format!(
            "{} minute{}",
            minutes,
            if minutes == 1 { "" } else { "s" }
        ));
    }
    if secs > 0 && days == 0 && hours == 0 {
        parts.push(format!(
            "{} second{}",
            secs,
            if secs == 1 { "" } else { "s" }
        ));
    }

    if parts.is_empty() {
        "0 seconds".to_string()
    } else {
        parts.join(", ")
    }
}

fn render_performance_section(report: &PerformanceReport) -> String {
    let mut out = String::new();
    out.push_str("\n## Performance\n\n");
    out.push_str(&format!("Total: {:.2} ms\n", report.total_duration_ms));

    if !report.stages.is_empty() {
        out.push_str("\nStages:\n");
        let mut stages: Vec<_> = report.stages.iter().collect();
        stages.sort_by(|a, b| {
            b.1.total_duration_ms
                .total_cmp(&a.1.total_duration_ms)
                .then_with(|| a.0.cmp(b.0))
        });
        for (name, stage) in stages {
            out.push_str(&format!(
                "- {}: {:.2} ms total ({} call{}, max {:.2} ms, last {:.2} ms)\n",
                name,
                stage.total_duration_ms,
                stage.calls,
                if stage.calls == 1 { "" } else { "s" },
                stage.max_duration_ms,
                stage.last_duration_ms
            ));
        }
    }

    if !report.counters.is_empty() {
        out.push_str("\nCounters:\n");
        let mut counters: Vec<_> = report.counters.iter().collect();
        counters.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        for (name, value) in counters {
            out.push_str(&format!("- {}: {}\n", name, value));
        }
    }

    out
}

// ============================================================================
// Docs filtering
// ============================================================================

/// Apply docs filter flags to a list of markdown documents.
///
/// When multiple flags are combined, results are intersected (AND logic).
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
                Some(RepoAction::Packages { .. }) => {
                    unreachable!("Packages is handled as an early return in commands.rs")
                }
                Some(RepoAction::DirtyPackages { filter }) => {
                    let rendered = render_dirty_packages(result, filter);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::DirtyPackageAreas { filter }) => {
                    let rendered = render_dirty_package_areas(result, filter);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::StagedPackages { filter }) => {
                    let rendered = render_staged_packages(result, filter);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::StagedPackageAreas { filter }) => {
                    let rendered = render_staged_package_areas(result, filter);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::UnstagedPackages { filter }) => {
                    let rendered = render_unstaged_packages(result, filter);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::UnstagedPackageAreas { filter }) => {
                    let rendered = render_unstaged_package_areas(result, filter);
                    if rendered.is_empty() {
                        std::process::exit(1);
                    }
                    out.push_str(&rendered);
                    out.push('\n');
                }
                Some(RepoAction::Deps { ui, filter }) => {
                    if let Some(ref filesystem) = result.filesystem
                        && let Some(ref repo) = filesystem.repo
                    {
                        if *ui {
                            out.push_str(&render_repo_deps_visual(repo, filter));
                        } else {
                            out.push_str(&render_repo_deps_text(repo, filter));
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
                Some(RepoAction::GitStatus { compact, .. }) => {
                    if let Some(ref filesystem) = result.filesystem
                        && let Some(ref git) = filesystem.git
                    {
                        out.push_str(&render_git_section(git, history_count, verbose, *compact));
                    }
                }
                Some(RepoAction::Structure { filter, .. }) => {
                    if let Some(ref filesystem) = result.filesystem
                        && let Some(ref repo) = filesystem.repo
                    {
                        out.push_str(&render_repo_section(
                            repo,
                            verbose,
                            repo_root,
                            filter,
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
        OutputFilter::Language => {
            out.push_str(&render_language_section(result, verbose, base_dir));
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
        | OutputFilter::Services
        | OutputFilter::Just
        | OutputFilter::BlastRadius => {
            unreachable!(
                "Programs, Services, Just, BlastRadius, and Remote filters should be handled separately"
            )
        }
    }

    if let Some(ref performance) = result.performance {
        out.push_str(&render_performance_section(performance));
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
fn apply_filter_to_json(
    result: &SniffResult,
    filter: OutputFilter,
    docs_filter: &DocsFilter,
    files_filter: &FilesFilter,
) -> serde_json::Value {
    use serde_json::{Value, json};

    match filter {
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
            // Flatten: return repo data at top level
            if let Some(ref fs) = result.filesystem {
                serde_json::to_value(&fs.repo).unwrap_or(Value::Null)
            } else {
                json!({})
            }
        }
        OutputFilter::Language => {
            // Flatten: return languages data at top level
            if let Some(ref fs) = result.filesystem {
                serde_json::to_value(&fs.languages).unwrap_or(Value::Null)
            } else {
                json!({})
            }
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
        | OutputFilter::Services
        | OutputFilter::Just
        | OutputFilter::BlastRadius => {
            unreachable!(
                "Programs, Services, Just, and BlastRadius filters should be handled separately"
            )
        }
    }
}

fn attach_performance(
    mut filtered_json: serde_json::Value,
    result: &SniffResult,
) -> serde_json::Value {
    let Some(performance) = result.performance.as_ref() else {
        return filtered_json;
    };

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

pub fn print_json(
    result: &SniffResult,
    filter: OutputFilter,
    docs_filter: &DocsFilter,
    files_filter: &FilesFilter,
) -> serde_json::Result<()> {
    let filtered_json = attach_performance(
        apply_filter_to_json(result, filter, docs_filter, files_filter),
        result,
    );
    println!("{}", serde_json::to_string_pretty(&filtered_json)?);
    Ok(())
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

        fn sample_docs() -> Vec<MarkdownMeta> {
            vec![
                make_doc("README.md"),
                make_doc("sniff/README.md"),
                make_doc("sniff/lib/src/notes.md"),
                make_doc(".ai/plans/2026-02-07.plan-for-feature.md"),
                make_doc("homelab/docs/planning.md"),
                make_doc("darkmatter/lib/src/README.md"),
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
    }

    #[test]
    fn test_format_uptime_zero() {
        assert_eq!(format_uptime(0), "0 minutes");
    }

    #[test]
    fn test_format_uptime_seconds() {
        assert_eq!(format_uptime(30), "30 seconds");
        assert_eq!(format_uptime(1), "1 second");
    }

    #[test]
    fn test_format_uptime_minutes() {
        assert_eq!(format_uptime(60), "1 minute");
        assert_eq!(format_uptime(120), "2 minutes");
        assert_eq!(format_uptime(90), "1 minute, 30 seconds");
    }

    #[test]
    fn test_format_uptime_hours() {
        assert_eq!(format_uptime(3600), "1 hour");
        assert_eq!(format_uptime(3660), "1 hour, 1 minute");
        assert_eq!(format_uptime(7200), "2 hours");
        assert_eq!(format_uptime(7320), "2 hours, 2 minutes");
    }

    #[test]
    fn test_format_uptime_days() {
        assert_eq!(format_uptime(86400), "1 day");
        assert_eq!(format_uptime(86400 + 3600), "1 day, 1 hour");
        assert_eq!(format_uptime(86400 + 3660), "1 day, 1 hour, 1 minute");
        assert_eq!(
            format_uptime(2 * 86400 + 5 * 3600 + 30 * 60),
            "2 days, 5 hours, 30 minutes"
        );
    }

    #[test]
    fn test_format_uptime_long() {
        // 16 days, 13 hours, 26 minutes
        assert_eq!(
            format_uptime(16 * 86400 + 13 * 3600 + 26 * 60),
            "16 days, 13 hours, 26 minutes"
        );
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

        let filtered = attach_performance(serde_json::json!({"name": "sniff"}), &result);
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

        let filtered = attach_performance(serde_json::json!(["a", "b"]), &result);
        assert_eq!(filtered["data"], serde_json::json!(["a", "b"]));
        assert_eq!(filtered["performance"]["counters"]["files"], 2);
    }
}
