//! Output formatting for sniff CLI.
//!
//! This module handles text and JSON output formatting for all detection results.
//! Each major section (OS, Hardware, Network, Filesystem, Programs, Services)
//! has its own submodule.

mod filesystem;
mod hardware;
mod network;
mod os;
mod programs;
mod remote;
mod services;
mod structure;
mod topics;

use std::path::Path;

use sniff::SniffResult;

use crate::DocsFilter;

pub use filesystem::print_git_section;
pub use programs::{print_programs_json, print_programs_markdown};
pub use remote::{print_remote_json, print_remote_text};
pub use services::{print_services_json, print_services_text};
pub use structure::print_structure;
pub use topics::print_topics_table;

// Re-export types needed by submodules
pub(crate) use filesystem::{
    print_docs_section, print_filesystem_section, print_language_section, print_repo_deps,
    print_repo_packages, print_repo_section,
};
pub(crate) use hardware::{
    print_cpu_section, print_gpu_section, print_hardware_section, print_memory_section,
    print_storage_section,
};
pub(crate) use network::print_network_section;
pub(crate) use os::print_os_section;

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
    /// Show only git info (filesystem subsection, flattened in JSON)
    Git,
    /// Show only repo/monorepo info (filesystem subsection, flattened in JSON)
    Repo,
    /// Show only language detection (filesystem subsection, flattened in JSON)
    Language,
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

// ============================================================================
// Docs filtering
// ============================================================================

/// Apply docs filter flags to a list of markdown documents.
///
/// When multiple flags are combined, results are intersected (AND logic).
/// When no flags are set, all documents are returned.
fn filter_docs(
    docs: &[sniff::filesystem::docs::MarkdownMeta],
    filter: &DocsFilter,
) -> Vec<sniff::filesystem::docs::MarkdownMeta> {
    if !filter.readme
        && !filter.plan
        && !filter.src
        && !filter.has_prompt
        && filter.filter.is_none()
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
            if let Some(ref substring) = filter.filter {
                if !path_lower.contains(&substring.to_lowercase()) {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect()
}

// ============================================================================
// Main print functions
// ============================================================================

pub fn print_text(
    result: &SniffResult,
    verbose: u8,
    filter: OutputFilter,
    history_count: usize,
    docs_filter: &DocsFilter,
    deps: bool,
    packages: bool,
) {
    // Get repo root for relative paths
    let repo_root = result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.git.as_ref())
        .map(|git| git.repo_root.as_path());

    match filter {
        OutputFilter::All => {
            // Print all sections that are present
            if let Some(ref os) = result.os {
                print_os_section(os, verbose);
            }
            if let Some(ref hardware) = result.hardware {
                print_hardware_section(hardware, verbose, repo_root);
            }
            if let Some(ref network) = result.network {
                print_network_section(network);
            }
            if let Some(ref filesystem) = result.filesystem {
                print_filesystem_section(filesystem, verbose, repo_root);
            }
        }
        // Top-level section filters (used for single-section requests)
        OutputFilter::Os => {
            if let Some(ref os) = result.os {
                print_os_section(os, verbose);
            }
        }
        OutputFilter::Hardware => {
            if let Some(ref hardware) = result.hardware {
                print_hardware_section(hardware, verbose, repo_root);
            }
        }
        OutputFilter::Network => {
            if let Some(ref network) = result.network {
                print_network_section(network);
            }
        }
        OutputFilter::Filesystem => {
            if let Some(ref filesystem) = result.filesystem {
                print_filesystem_section(filesystem, verbose, repo_root);
            }
        }
        OutputFilter::Cpu => {
            if let Some(ref hardware) = result.hardware {
                print_cpu_section(&hardware.cpu, verbose);
            }
        }
        OutputFilter::Gpu => {
            if let Some(ref hardware) = result.hardware {
                print_gpu_section(&hardware.gpu, verbose);
            }
        }
        OutputFilter::Memory => {
            if let Some(ref hardware) = result.hardware {
                print_memory_section(&hardware.memory);
            }
        }
        OutputFilter::Storage => {
            if let Some(ref hardware) = result.hardware {
                print_storage_section(&hardware.storage, verbose, repo_root);
            }
        }
        OutputFilter::Git => {
            if let Some(ref filesystem) = result.filesystem
                && let Some(ref git) = filesystem.git
            {
                print_git_section(git, history_count);
            }
        }
        OutputFilter::Repo => {
            if packages {
                print_repo_packages(result);
            } else if let Some(ref filesystem) = result.filesystem
                && let Some(ref repo) = filesystem.repo
            {
                if deps {
                    print_repo_deps(repo);
                } else {
                    print_repo_section(repo, verbose, repo_root);
                }
            }
        }
        OutputFilter::Language => {
            if let Some(ref filesystem) = result.filesystem
                && let Some(ref langs) = filesystem.languages
            {
                print_language_section(langs, verbose);
            }
        }
        OutputFilter::Docs => {
            if let Some(ref filesystem) = result.filesystem
                && let Some(ref docs) = filesystem.docs
            {
                let filtered = filter_docs(docs, docs_filter);
                print_docs_section(&filtered, verbose);
            }
        }
        // Programs and Services filters are handled separately in main.rs
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
            // These are handled separately, should not reach here
            unreachable!("Programs, Services, and Remote filters should be handled separately")
        }
    }
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
        OutputFilter::Git => {
            // Flatten: return git data at top level
            if let Some(ref fs) = result.filesystem {
                serde_json::to_value(&fs.git).unwrap_or(Value::Null)
            } else {
                json!({})
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
        // Programs and Services filters are handled separately
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
            unreachable!("Programs and Services filters should be handled separately")
        }
    }
}

pub fn print_json(
    result: &SniffResult,
    filter: OutputFilter,
    docs_filter: &DocsFilter,
) -> serde_json::Result<()> {
    let filtered_json = apply_filter_to_json(result, filter, docs_filter);
    println!("{}", serde_json::to_string_pretty(&filtered_json)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
                model: None,
                prompt: None,
                last_updated: Utc::now(),
                content_hash: String::new(),
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
                filter: Some("homelab".to_string()),
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
                filter: Some("HOMELAB".to_string()),
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
                filter: Some("homelab".to_string()),
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
}
