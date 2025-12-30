//! Research Library - Automated research on software libraries
//!
//! This library provides tools for automated research on software libraries,
//! running multiple LLM prompts in parallel and saving results.
//!
//! ## Tool Integration
//!
//! Phase 1 prompts (research prompts) have access to web search and scraping tools:
//! - [`BraveSearchTool`](shared::tools::BraveSearchTool) - Web search via Brave Search API
//! - [`ScreenScrapeTool`](shared::tools::ScreenScrapeTool) - Web page content extraction
//!
//! Phase 2 prompts (synthesis) run without tools as they consolidate existing content.

pub mod link;
pub mod list;
pub mod providers;
pub mod utils;
pub mod validation;

use chrono::{DateTime, Utc};
use futures::future::join_all;
use inquire::{InquireError, Select};
use providers::zai;
use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::cmark;
use reqwest::Client as HttpClient;
use rig::agent::{Agent, CancelSignal, PromptHook};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{AssistantContent, CompletionModel, Message, Prompt};
use rig::providers::{gemini, openai};
use serde::{Deserialize, Serialize};
use shared::tools::{BravePlan, BraveSearchTool, ScreenScrapeTool};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;
use thiserror::Error;
use tokio::fs;
use tracing::{Span, debug, info, info_span, instrument, warn};

use crate::validation::parse_and_validate_frontmatter;

/// A PromptHook that emits tracing events for agent interactions.
///
/// This hook is used to trace all tool calls made by agents during research tasks,
/// providing visibility into the agent's decision-making process.
#[derive(Clone)]
pub struct TracingPromptHook {
    span: Span,
}

impl TracingPromptHook {
    /// Create a new TracingPromptHook for the given task name.
    pub fn new(task_name: &str) -> Self {
        Self {
            span: info_span!("agent_task", task = %task_name),
        }
    }
}

impl<M> PromptHook<M> for TracingPromptHook
where
    M: CompletionModel,
{
    async fn on_completion_call(
        &self,
        _prompt: &Message,
        history: &[Message],
        _cancel_sig: CancelSignal,
    ) {
        debug!(
            parent: &self.span,
            history_len = history.len(),
            "Sending prompt to model"
        );
    }

    async fn on_completion_response(
        &self,
        _prompt: &Message,
        response: &rig::completion::CompletionResponse<M::Response>,
        _cancel_sig: CancelSignal,
    ) {
        let tool_call_count = response
            .choice
            .iter()
            .filter(|c| matches!(c, AssistantContent::ToolCall(_)))
            .count();

        debug!(
            parent: &self.span,
            has_tool_calls = tool_call_count > 0,
            tool_call_count,
            "Received model response"
        );
    }

    async fn on_tool_call(
        &self,
        tool_name: &str,
        tool_call_id: Option<String>,
        args: &str,
        _cancel_sig: CancelSignal,
    ) {
        info!(
            parent: &self.span,
            tool.name = %tool_name,
            tool.call_id = ?tool_call_id,
            tool.args = %args,
            "Invoking tool"
        );
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        tool_call_id: Option<String>,
        _args: &str,
        result: &str,
        _cancel_sig: CancelSignal,
    ) {
        // Truncate result for logging (tool results can be large)
        let result_preview: String = result.chars().take(200).collect();
        let truncated = result.len() > 200;

        info!(
            parent: &self.span,
            tool.name = %tool_name,
            tool.call_id = ?tool_call_id,
            tool.result_preview = %result_preview,
            tool.result_truncated = truncated,
            tool.result_len = result.len(),
            "Tool returned result"
        );
    }
}

/// Embedded prompt templates
mod prompts {
    pub const OVERVIEW: &str = include_str!("../prompts/overview.md");
    pub const SIMILAR_LIBRARIES: &str = include_str!("../prompts/similar_libraries.md");
    pub const INTEGRATION_PARTNERS: &str = include_str!("../prompts/integration_partners.md");
    pub const USE_CASES: &str = include_str!("../prompts/use_cases.md");
    pub const CHANGELOG: &str = include_str!("../prompts/changelog.md");
    pub const ADDITIONAL_QUESTION: &str = include_str!("../prompts/additional_question.md");
    pub const CONTEXT: &str = include_str!("../prompts/context.md");
    pub const SKILL: &str = include_str!("../prompts/skill.md");
    pub const DEEP_DIVE: &str = include_str!("../prompts/deep_dive.md");
    pub const BRIEF: &str = include_str!("../prompts/brief.md");
}

/// Standard Phase 1 prompts that should be present for complete research.
/// Each entry is (name, filename, prompt_template).
const STANDARD_PROMPTS: [(&str, &str, &str); 5] = [
    ("overview", "overview.md", prompts::OVERVIEW),
    (
        "similar_libraries",
        "similar_libraries.md",
        prompts::SIMILAR_LIBRARIES,
    ),
    (
        "integration_partners",
        "integration_partners.md",
        prompts::INTEGRATION_PARTNERS,
    ),
    ("use_cases", "use_cases.md", prompts::USE_CASES),
    ("changelog", "changelog.md", prompts::CHANGELOG),
];

/// A standard prompt that is missing from the research output.
#[derive(Debug, Clone)]
pub struct MissingPrompt {
    pub name: &'static str,
    pub filename: &'static str,
    pub template: &'static str,
}

/// Check which standard prompts are missing from the output directory.
///
/// Returns a list of prompts that don't have corresponding output files.
#[deprecated(
    note = "Use research_health() from validation::health module. Note: research_health() requires ResearchType and builds paths internally using RESEARCH_DIR environment variable or current directory."
)]
pub async fn check_missing_standard_prompts(output_dir: &std::path::Path) -> Vec<MissingPrompt> {
    let mut missing = Vec::new();

    for (name, filename, template) in STANDARD_PROMPTS {
        let path = output_dir.join(filename);
        if !path.exists() {
            missing.push(MissingPrompt {
                name,
                filename,
                template,
            });
        }
    }

    missing
}

/// A final output file that is missing from the research directory.
#[derive(Debug, Clone)]
pub struct MissingOutput {
    pub name: &'static str,
    pub filename: &'static str,
}

/// Expected final output files that should be generated
const EXPECTED_OUTPUTS: &[(&str, &str)] = &[
    ("Skill", "skill/SKILL.md"),
    ("Deep Dive", "deep_dive.md"),
    ("Brief", "brief.md"),
];

/// Check which final output files are missing from the output directory.
///
/// This checks for the presence of:
/// - skill/SKILL.md (not just the skill/ directory)
/// - deep_dive.md
/// - brief.md
///
/// Returns a list of outputs that don't exist.
#[deprecated(
    note = "Use research_health() from validation::health module. Note: research_health() requires ResearchType and builds paths internally using RESEARCH_DIR environment variable or current directory."
)]
pub async fn check_missing_outputs(output_dir: &std::path::Path) -> Vec<MissingOutput> {
    let mut missing = Vec::new();

    for (name, filename) in EXPECTED_OUTPUTS {
        let path = output_dir.join(filename);
        if !path.exists() {
            missing.push(MissingOutput {
                name,
                filename,
            });
        }
    }

    missing
}

/// Information about a library found in a package manager
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub package_manager: String,
    pub language: String,
    pub url: String,
    pub repository: Option<String>,
    pub description: Option<String>,
}

impl fmt::Display for LibraryInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.package_manager, self.language)?;
        if let Some(ref desc) = self.description {
            // Truncate long descriptions
            let short_desc: String = desc.chars().take(60).collect();
            if desc.len() > 60 {
                write!(f, " - {}...", short_desc)?;
            } else {
                write!(f, " - {}", short_desc)?;
            }
        }
        Ok(())
    }
}

/// The kind of research being performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResearchKind {
    Library,
    // Future: Software, Standard, Company, etc.
}

/// Metadata for a research output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchMetadata {
    /// Schema version for future evolution (defaults to 0 for backward compatibility)
    #[serde(default)]
    pub schema_version: u32,
    /// The kind of research
    pub kind: ResearchKind,
    /// Information about the library (if kind is Library)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_info: Option<LibraryInfoMetadata>,
    /// Additional files created from user prompts (filename -> prompt)
    #[serde(default)]
    pub additional_files: std::collections::HashMap<String, String>,
    /// When the research was first created
    pub created_at: DateTime<Utc>,
    /// When the research was last updated
    pub updated_at: DateTime<Utc>,
    /// Single-sentence summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,
    /// Paragraph summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Guidance on when to use this research (e.g., "Use when working with X library")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
}

/// Library info stored in metadata (serializable version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInfoMetadata {
    pub package_manager: String,
    pub language: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

impl From<&LibraryInfo> for LibraryInfoMetadata {
    fn from(info: &LibraryInfo) -> Self {
        Self {
            package_manager: info.package_manager.clone(),
            language: info.language.clone(),
            url: info.url.clone(),
            repository: info.repository.clone(),
        }
    }
}

impl ResearchMetadata {
    /// Create new metadata for library research
    pub fn new_library(library_info: Option<&LibraryInfo>) -> Self {
        let now = Utc::now();
        Self {
            schema_version: 0,
            kind: ResearchKind::Library,
            library_info: library_info.map(LibraryInfoMetadata::from),
            additional_files: std::collections::HashMap::new(),
            created_at: now,
            updated_at: now,
            brief: None,
            summary: None,
            when_to_use: None,
        }
    }

    /// Load metadata from a directory
    pub async fn load(output_dir: &std::path::Path) -> Option<Self> {
        let path = output_dir.join("metadata.json");
        let content = fs::read_to_string(&path).await.ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save metadata to a directory
    pub async fn save(&self, output_dir: &std::path::Path) -> Result<(), std::io::Error> {
        let path = output_dir.join("metadata.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&path, content).await
    }

    /// Add an additional file to the metadata
    pub fn add_additional_file(&mut self, filename: String, prompt: String) {
        self.additional_files.insert(filename, prompt);
        self.updated_at = Utc::now();
    }

    /// Check if a prompt is similar to an existing one (simple text-based comparison)
    pub fn check_overlap(&self, prompt: &str) -> Option<String> {
        let prompt_lower = prompt.to_lowercase();
        for (filename, existing_prompt) in &self.additional_files {
            let existing_lower = existing_prompt.to_lowercase();
            // Simple overlap detection: check if significant words overlap
            let prompt_words: std::collections::HashSet<_> = prompt_lower
                .split_whitespace()
                .filter(|w| w.len() > 3)
                .collect();
            let existing_words: std::collections::HashSet<_> = existing_lower
                .split_whitespace()
                .filter(|w| w.len() > 3)
                .collect();
            let overlap: usize = prompt_words.intersection(&existing_words).count();
            let min_words = prompt_words.len().min(existing_words.len());
            if min_words > 0 && overlap as f32 / min_words as f32 > 0.5 {
                return Some(filename.clone());
            }
        }
        None
    }

    /// Get the next question number for additional files
    pub fn next_question_number(&self) -> usize {
        self.additional_files
            .keys()
            .filter_map(|k| {
                k.strip_prefix("question_")
                    .and_then(|s| s.strip_suffix(".md"))
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0)
            + 1
    }
}

/// Result of overlap detection for a prompt
#[derive(Debug, Clone)]
pub struct PromptOverlap {
    /// The original prompt text
    pub prompt: String,
    /// The filename that would be generated
    pub filename: String,
    /// Whether this prompt overlaps with existing research
    pub verdict: OverlapVerdict,
    /// The conflicting file if there's overlap
    pub conflict: Option<String>,
}

/// Verdict on whether a prompt overlaps with existing research
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlapVerdict {
    /// No overlap detected, safe to proceed
    New,
    /// Potential overlap with existing document
    Conflict,
}

/// Response from crates.io API
#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    krate: Option<CratesIoCrate>,
}

#[derive(Debug, Deserialize)]
struct CratesIoCrate {
    description: Option<String>,
    repository: Option<String>,
}

/// Response from npm registry API
#[derive(Debug, Deserialize)]
struct NpmResponse {
    description: Option<String>,
    repository: Option<NpmRepository>,
}

#[derive(Debug, Deserialize)]
struct NpmRepository {
    url: Option<String>,
}

/// Response from PyPI API
#[derive(Debug, Deserialize)]
struct PyPIResponse {
    info: Option<PyPIInfo>,
}

#[derive(Debug, Deserialize)]
struct PyPIInfo {
    summary: Option<String>,
    project_urls: Option<std::collections::HashMap<String, String>>,
}

/// Response from Packagist API
#[derive(Debug, Deserialize)]
struct PackagistSearchResponse {
    results: Option<Vec<PackagistResult>>,
}

#[derive(Debug, Deserialize)]
struct PackagistResult {
    name: String,
    description: Option<String>,
    url: Option<String>,
}

/// Find a library across multiple package managers concurrently.
///
/// Checks the following package managers:
/// - crates.io (Rust)
/// - npm (JavaScript/TypeScript)
/// - PyPI (Python)
/// - Packagist (PHP)
/// - LuaRocks (Lua)
/// - pkg.go.dev (Go)
///
/// Returns a list of `LibraryInfo` for each package manager where the library was found.
pub async fn find_library(name: &str) -> Vec<LibraryInfo> {
    let client = HttpClient::builder()
        .user_agent("research-lib/0.1.0")
        .build()
        .unwrap_or_default();

    let name = name.to_string();

    // Check all package managers concurrently
    let (crates_io, npm, pypi, packagist, luarocks, go) = tokio::join!(
        check_crates_io(&client, &name),
        check_npm(&client, &name),
        check_pypi(&client, &name),
        check_packagist(&client, &name),
        check_luarocks(&client, &name),
        check_go(&client, &name),
    );

    // Collect all found libraries (no printing here - select_library handles display)
    [crates_io, npm, pypi, packagist, luarocks, go]
        .into_iter()
        .flatten()
        .collect()
}

async fn check_crates_io(client: &HttpClient, name: &str) -> Option<LibraryInfo> {
    let url = format!("https://crates.io/api/v1/crates/{}", name);
    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let data: CratesIoResponse = response.json().await.ok()?;
    let description = data.krate.as_ref().and_then(|c| c.description.clone());
    let repository = data.krate.as_ref().and_then(|c| c.repository.clone());

    Some(LibraryInfo {
        package_manager: "crates.io".to_string(),
        language: "Rust".to_string(),
        url: format!("https://crates.io/crates/{}", name),
        repository,
        description,
    })
}

async fn check_npm(client: &HttpClient, name: &str) -> Option<LibraryInfo> {
    let url = format!("https://registry.npmjs.org/{}", name);
    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let data: NpmResponse = response.json().await.ok()?;

    // Extract repository URL and clean git+ prefix
    let repository = data
        .repository
        .as_ref()
        .and_then(|r| r.url.as_ref())
        .map(|url| {
            // Clean git+https:// prefix to just https://
            url.strip_prefix("git+").unwrap_or(url).to_string()
        });

    Some(LibraryInfo {
        package_manager: "npm".to_string(),
        language: "JavaScript/TypeScript".to_string(),
        url: format!("https://www.npmjs.com/package/{}", name),
        repository,
        description: data.description,
    })
}

async fn check_pypi(client: &HttpClient, name: &str) -> Option<LibraryInfo> {
    let url = format!("https://pypi.org/pypi/{}/json", name);
    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let data: PyPIResponse = response.json().await.ok()?;
    let description = data.info.as_ref().and_then(|i| i.summary.clone());

    // Extract repository URL from project_urls (try "Repository" first, then "Source")
    let repository = data
        .info
        .as_ref()
        .and_then(|i| i.project_urls.as_ref())
        .and_then(|urls| {
            urls.get("Repository")
                .or_else(|| urls.get("Source"))
                .cloned()
        });

    Some(LibraryInfo {
        package_manager: "PyPI".to_string(),
        language: "Python".to_string(),
        url: format!("https://pypi.org/project/{}", name),
        repository,
        description,
    })
}

async fn check_packagist(client: &HttpClient, name: &str) -> Option<LibraryInfo> {
    // Packagist requires vendor/package format, so we search instead
    let url = format!("https://packagist.org/search.json?q={}", name);
    let response = client.get(&url).send().await.ok()?;

    if !response.status().is_success() {
        return None;
    }

    let data: PackagistSearchResponse = response.json().await.ok()?;
    let results = data.results?;

    // Look for an exact match in the package name
    let matching = results.into_iter().find(|r| {
        let package_name = r.name.split('/').next_back().unwrap_or(&r.name);
        package_name == name
    })?;

    Some(LibraryInfo {
        package_manager: "Packagist".to_string(),
        language: "PHP".to_string(),
        url: matching
            .url
            .unwrap_or_else(|| format!("https://packagist.org/packages/{}", matching.name)),
        repository: None,
        description: matching.description,
    })
}

async fn check_luarocks(client: &HttpClient, name: &str) -> Option<LibraryInfo> {
    // LuaRocks doesn't have a formal API, but we can check if the package page exists
    let url = format!("https://luarocks.org/modules/{}", name);
    let response = client.head(&url).send().await.ok()?;

    if response.status().is_success() {
        return Some(LibraryInfo {
            package_manager: "LuaRocks".to_string(),
            language: "Lua".to_string(),
            url,
            repository: None,
            description: None,
        });
    }

    // Try searching
    let search_url = format!("https://luarocks.org/search?q={}", name);
    let response = client.get(&search_url).send().await.ok()?;

    if response.status().is_success() {
        let body = response.text().await.ok()?;
        // Simple check if the package name appears in search results
        if body.contains(&format!("\"/{}/", name)) || body.contains(&format!(">{}<", name)) {
            return Some(LibraryInfo {
                package_manager: "LuaRocks".to_string(),
                language: "Lua".to_string(),
                url: format!("https://luarocks.org/modules/{}", name),
                repository: None,
                description: None,
            });
        }
    }

    None
}

async fn check_go(client: &HttpClient, name: &str) -> Option<LibraryInfo> {
    // pkg.go.dev requires full module path, but we can search
    // First try as a potential GitHub path
    let common_prefixes = ["github.com/", "golang.org/x/", ""];

    for prefix in common_prefixes {
        let module = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}{}", prefix, name)
        };

        let url = format!("https://pkg.go.dev/{}", module);
        let response = client.head(&url).send().await.ok();

        if let Some(resp) = response
            && resp.status().is_success()
        {
            return Some(LibraryInfo {
                package_manager: "pkg.go.dev".to_string(),
                language: "Go".to_string(),
                url,
                repository: None,
                description: None,
            });
        }
    }

    None
}

/// Result of library selection
#[derive(Debug)]
pub enum LibrarySelection {
    /// User selected a specific library
    Selected(LibraryInfo),
    /// Only one library was found (no selection needed)
    Single(LibraryInfo),
    /// No libraries were found
    NotFound,
    /// User cancelled the selection (pressed ESC)
    Cancelled,
}

/// Interactively select a library if multiple package managers match.
///
/// - If no matches: returns `LibrarySelection::NotFound`
/// - If one match: returns `LibrarySelection::Single` with info message
/// - If multiple matches: prompts user to select one
/// - If user cancels: returns `LibrarySelection::Cancelled`
pub fn select_library(libraries: Vec<LibraryInfo>, topic: &str) -> LibrarySelection {
    match libraries.len() {
        0 => {
            println!(
                "  ⚠ '{}' not found on any package manager (may be a general topic)\n",
                topic
            );
            LibrarySelection::NotFound
        }
        1 => {
            let lib = libraries.into_iter().next().unwrap();
            println!(
                "  ✓ Found '{}' on {} ({})\n",
                topic, lib.package_manager, lib.language
            );
            LibrarySelection::Single(lib)
        }
        _ => {
            println!(
                "\n  Found '{}' on {} package managers. Please select the intended one:\n",
                topic,
                libraries.len()
            );

            let selection = Select::new("Which package manager?", libraries)
                .with_help_message("↑↓ to move, enter to select, ESC to skip")
                .prompt();

            match selection {
                Ok(lib) => {
                    println!(
                        "\n  → Selected: {} ({})\n",
                        lib.package_manager, lib.language
                    );
                    LibrarySelection::Selected(lib)
                }
                Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                    println!("\n  ⚠ Selection skipped, continuing as general topic\n");
                    LibrarySelection::Cancelled
                }
                Err(_) => {
                    println!("\n  ⚠ Selection error, continuing as general topic\n");
                    LibrarySelection::Cancelled
                }
            }
        }
    }
}

/// Errors that can occur during research operations
#[derive(Error, Debug)]
pub enum ResearchError {
    #[error("Failed to create output directory: {0}")]
    OutputDirCreation(#[from] std::io::Error),

    #[error("All prompts failed")]
    AllPromptsFailed,

    #[error("Too many prompts failed ({succeeded}/{total} succeeded)")]
    TooManyPromptsFailed { succeeded: usize, total: usize },

    #[error("Operation cancelled by user")]
    Cancelled,
}

/// Metrics from a completed prompt
#[derive(Debug, Default, Clone)]
pub struct PromptMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub elapsed_secs: f32,
}

/// Result of a research operation
#[derive(Debug)]
pub struct ResearchResult {
    pub topic: String,
    pub output_dir: PathBuf,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: bool,
    pub total_time_secs: f32,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
}

/// Split multi-file LLM output into separate files.
/// Handles the implicit first file (SKILL.md) that doesn't have a separator before it.
///
/// LLM output format expected:
/// ```text
/// [Content for SKILL.md]
/// --- FILE: advanced-usage.md ---
/// [Content for advanced-usage.md]
/// --- FILE: examples.md ---
/// [Content for examples.md]
/// ```
fn split_into_files(content: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut current_filename = "SKILL.md".to_string();  // First file is implicitly SKILL.md
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with("--- FILE:") && line.ends_with("---") {
            // Save previous file
            if !current_content.trim().is_empty() {
                files.push((current_filename.clone(), current_content.trim().to_string()));
            }

            // Extract new filename from separator
            current_filename = line.trim_start_matches("--- FILE:")
                .trim_end_matches("---")
                .trim()
                .to_string();
            current_content = String::new();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Don't forget the last file
    if !current_content.trim().is_empty() {
        files.push((current_filename, current_content.trim().to_string()));
    }

    files
}

/// Normalize markdown by parsing and re-serializing it.
/// This produces consistent formatting regardless of LLM output style.
/// Also strips empty HTML anchor tags that LLMs sometimes generate for navigation.
fn normalize_markdown(input: &str) -> String {
    use pulldown_cmark::{CowStr, Event};

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(input, options);

    // Filter out empty anchor tags like <a name="..."></a> or <a id="..."></a>
    // but preserve other useful inline HTML
    let is_empty_anchor = |html: &CowStr| {
        let s = html.trim();
        (s.starts_with("<a ") && s.ends_with("></a>"))
            || (s.starts_with("<a ") && s.ends_with("/>"))
    };

    let filtered = parser.filter(|event| match event {
        Event::Html(html) | Event::InlineHtml(html) => !is_empty_anchor(html),
        _ => true,
    });

    let mut output = String::new();

    if cmark(filtered, &mut output).is_err() {
        return input.to_string();
    }

    output
}

/// Parse the brief response into (brief, summary) tuple
fn parse_brief_response(response: &str) -> (Option<String>, Option<String>) {
    let mut brief = None;
    let mut summary = None;

    for line in response.lines() {
        let line = line.trim();
        if line.starts_with("BRIEF:") {
            brief = Some(line.trim_start_matches("BRIEF:").trim().to_string());
        } else if line.starts_with("SUMMARY:") {
            // Summary might span multiple lines until end
            if let Some(start) = response.find("SUMMARY:") {
                summary = Some(response[start + 8..].trim().to_string());
            }
            break;
        }
    }

    (brief, summary)
}

/// Library context for building prompts
struct LibraryContext<'a> {
    package_manager: &'a str,
    language: &'a str,
    url: &'a str,
}

impl<'a> From<&'a LibraryInfo> for LibraryContext<'a> {
    fn from(info: &'a LibraryInfo) -> Self {
        Self {
            package_manager: &info.package_manager,
            language: &info.language,
            url: &info.url,
        }
    }
}

impl<'a> From<&'a LibraryInfoMetadata> for LibraryContext<'a> {
    fn from(info: &'a LibraryInfoMetadata) -> Self {
        Self {
            package_manager: &info.package_manager,
            language: &info.language,
            url: &info.url,
        }
    }
}

/// Build a prompt by replacing topic and library context placeholders.
///
/// Replaces:
/// - `{{topic}}` - The library/topic name
/// - `{{package_manager}}` - The package manager name (e.g., "crates.io", "npm")
/// - `{{language}}` - The programming language (e.g., "Rust", "JavaScript")
/// - `{{url}}` - The URL to the package on the package manager
fn build_prompt(template: &str, topic: &str, library_info: Option<&LibraryInfo>) -> String {
    let ctx = library_info.map(LibraryContext::from);
    build_prompt_with_context(template, topic, ctx.as_ref())
}

/// Internal helper to build prompts with optional library context
fn build_prompt_with_context(template: &str, topic: &str, ctx: Option<&LibraryContext>) -> String {
    let (package_manager, language, url) = match ctx {
        Some(c) => (c.package_manager, c.language, c.url),
        None => ("unknown", "unknown", "N/A"),
    };

    template
        .replace("{{topic}}", topic)
        .replace("{{package_manager}}", package_manager)
        .replace("{{language}}", language)
        .replace("{{url}}", url)
}

/// Result of a single prompt task
struct PromptTaskResult {
    metrics: Option<PromptMetrics>,
}

/// Run a prompt task and save result, printing progress as it completes
#[allow(clippy::too_many_arguments)]
async fn run_prompt_task<M>(
    name: &'static str,
    filename: &'static str,
    output_dir: PathBuf,
    model: M,
    prompt: String,
    counter: Arc<AtomicUsize>,
    total: usize,
    start_time: Instant,
    cancelled: Arc<AtomicBool>,
) -> PromptTaskResult
where
    M: CompletionModel,
{
    // Check if already cancelled before starting
    if cancelled.load(Ordering::SeqCst) {
        return PromptTaskResult { metrics: None };
    }

    println!("  [{}] Starting...", name);

    let result = model.completion_request(&prompt).send().await;

    // Check if cancelled after the request completed
    if cancelled.load(Ordering::SeqCst) {
        println!("  [{}] Cancelled (response discarded)", name);
        return PromptTaskResult { metrics: None };
    }

    let elapsed = start_time.elapsed().as_secs_f32();
    let completed = counter.fetch_add(1, Ordering::SeqCst) + 1;

    let metrics = match result {
        Ok(response) => {
            let content: String = response
                .choice
                .into_iter()
                .filter_map(|c| match c {
                    AssistantContent::Text(text) => Some(text.text),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            let usage = &response.usage;
            let metrics = PromptMetrics {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                total_tokens: usage.total_tokens,
                elapsed_secs: elapsed,
            };

            // Write raw content without normalization
            // Normalization happens selectively later (e.g., SKILL.md preserves frontmatter)
            let path = output_dir.join(filename);
            match fs::write(&path, &content).await {
                Ok(_) => {
                    println!(
                        "  [{}/{}] ✓ {} ({:.1}s) | tokens: {} in, {} out, {} total",
                        completed,
                        total,
                        name,
                        elapsed,
                        metrics.input_tokens,
                        metrics.output_tokens,
                        metrics.total_tokens,
                    );
                    Some(metrics)
                }
                Err(e) => {
                    eprintln!(
                        "  [{}/{}] ✗ {} write failed: {} ({:.1}s)",
                        completed, total, name, e, elapsed
                    );
                    None
                }
            }
        }
        Err(e) => {
            eprintln!(
                "  [{}/{}] ✗ {} failed: {} ({:.1}s)",
                completed, total, name, e, elapsed
            );
            None
        }
    };

    PromptTaskResult { metrics }
}

/// Check if web research tools are available (BRAVE_API_KEY is set).
///
/// Returns `true` if the environment is configured for tool usage.
pub fn tools_available() -> bool {
    std::env::var("BRAVE_API_KEY").is_ok()
}

/// Run a prompt task using an agent with tools, printing progress as it completes.
///
/// This function is used for Phase 1 prompts that benefit from web search
/// and scraping capabilities. If tools are not available (no BRAVE_API_KEY),
/// it falls back to a standard completion request without tools.
#[instrument(
    name = "prompt_task",
    skip(output_dir, agent, prompt, counter, cancelled),
    fields(
        task = name,
        filename = filename,
        prompt_len = prompt.len()
    )
)]
#[allow(clippy::too_many_arguments)]
async fn run_agent_prompt_task<M>(
    name: &'static str,
    filename: &'static str,
    output_dir: PathBuf,
    agent: Agent<M>,
    prompt: String,
    counter: Arc<AtomicUsize>,
    total: usize,
    start_time: Instant,
    cancelled: Arc<AtomicBool>,
) -> PromptTaskResult
where
    M: CompletionModel,
{
    // Check if already cancelled before starting
    if cancelled.load(Ordering::SeqCst) {
        debug!(task = name, "Task cancelled before starting");
        return PromptTaskResult { metrics: None };
    }

    info!(task = name, "Starting prompt task with tools");
    println!("  [{}] Starting (with tools)...", name);

    // Create a tracing hook for this task to emit tool call events
    let hook = TracingPromptHook::new(name);

    // Use multi_turn(15) to allow up to 15 rounds of tool calls before final response
    // Higher limit needed as research tasks may require multiple search + scrape operations
    // If this still hits the limit, the preamble should guide the agent to synthesize earlier
    let result = agent.prompt(&prompt).multi_turn(15).with_hook(hook).await;

    // Check if cancelled after the request completed
    if cancelled.load(Ordering::SeqCst) {
        println!("  [{}] Cancelled (response discarded)", name);
        return PromptTaskResult { metrics: None };
    }

    let elapsed = start_time.elapsed().as_secs_f32();
    let completed = counter.fetch_add(1, Ordering::SeqCst) + 1;

    let metrics = match result {
        Ok(content) => {
            debug!(
                task = name,
                content_len = content.len(),
                "Agent returned content"
            );

            // Agent returns the content directly as a string
            let metrics = PromptMetrics {
                input_tokens: 0, // Agent API doesn't expose token counts
                output_tokens: 0,
                total_tokens: 0,
                elapsed_secs: elapsed,
            };

            let normalized = normalize_markdown(&content);

            let path = output_dir.join(filename);
            match fs::write(&path, &normalized).await {
                Ok(_) => {
                    info!(
                        task = name,
                        elapsed_secs = elapsed,
                        content_len = normalized.len(),
                        "Task completed successfully"
                    );
                    println!("  [{}/{}] ✓ {} ({:.1}s)", completed, total, name, elapsed,);
                    Some(metrics)
                }
                Err(e) => {
                    warn!(
                        task = name,
                        error = %e,
                        "Failed to write output file"
                    );
                    eprintln!(
                        "  [{}/{}] ✗ {} write failed: {} ({:.1}s)",
                        completed, total, name, e, elapsed
                    );
                    None
                }
            }
        }
        Err(e) => {
            warn!(
                task = name,
                error = %e,
                elapsed_secs = elapsed,
                "Task failed"
            );
            eprintln!(
                "  [{}/{}] ✗ {} failed: {} ({:.1}s)",
                completed, total, name, e, elapsed
            );
            None
        }
    };

    PromptTaskResult { metrics }
}

/// Returns the default output directory for a given topic.
///
/// Uses the `RESEARCH_DIR` environment variable if set, otherwise falls back to `$HOME`.
/// The full path is: `${RESEARCH_DIR:-$HOME}/.research/library/{topic}`
pub fn default_output_dir(topic: &str) -> PathBuf {
    let base = std::env::var("RESEARCH_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
    base.join(".research").join("library").join(topic)
}

/// Run a dynamic question task and save result
#[allow(clippy::too_many_arguments)]
async fn run_question_task<M>(
    question_num: usize,
    topic: String,
    question: String,
    package_manager: String,
    language: String,
    url: String,
    output_dir: PathBuf,
    model: M,
    counter: Arc<AtomicUsize>,
    total: usize,
    start_time: Instant,
    cancelled: Arc<AtomicBool>,
) -> PromptTaskResult
where
    M: CompletionModel,
{
    // Check if already cancelled before starting
    if cancelled.load(Ordering::SeqCst) {
        return PromptTaskResult { metrics: None };
    }

    let name = format!("question_{}", question_num);
    println!("  [{}] Starting...", name);

    let ctx = LibraryContext {
        package_manager: &package_manager,
        language: &language,
        url: &url,
    };
    let prompt = build_prompt_with_context(prompts::ADDITIONAL_QUESTION, &topic, Some(&ctx))
        .replace("{{question}}", &question);

    let result = model.completion_request(&prompt).send().await;

    // Check if cancelled after the request completed
    if cancelled.load(Ordering::SeqCst) {
        println!("  [{}] Cancelled (response discarded)", name);
        return PromptTaskResult { metrics: None };
    }

    let elapsed = start_time.elapsed().as_secs_f32();
    let completed = counter.fetch_add(1, Ordering::SeqCst) + 1;

    let metrics = match result {
        Ok(response) => {
            let content: String = response
                .choice
                .into_iter()
                .filter_map(|c| match c {
                    AssistantContent::Text(text) => Some(text.text),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            let usage = &response.usage;
            let metrics = PromptMetrics {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                total_tokens: usage.total_tokens,
                elapsed_secs: elapsed,
            };

            let normalized = normalize_markdown(&content);

            let filename = format!("question_{}.md", question_num);
            let path = output_dir.join(&filename);
            match fs::write(&path, &normalized).await {
                Ok(_) => {
                    println!(
                        "  [{}/{}] ✓ {} ({:.1}s) | tokens: {} in, {} out, {} total",
                        completed,
                        total,
                        name,
                        elapsed,
                        metrics.input_tokens,
                        metrics.output_tokens,
                        metrics.total_tokens,
                    );
                    Some(metrics)
                }
                Err(e) => {
                    eprintln!(
                        "  [{}/{}] ✗ {} write failed: {} ({:.1}s)",
                        completed, total, name, e, elapsed
                    );
                    None
                }
            }
        }
        Err(e) => {
            eprintln!(
                "  [{}/{}] ✗ {} failed: {} ({:.1}s)",
                completed, total, name, e, elapsed
            );
            None
        }
    };

    PromptTaskResult { metrics }
}

/// Run incremental research by regenerating missing prompts and/or adding new questions.
///
/// This is called when metadata.json exists and either:
/// - Some standard prompts are missing (need to be regenerated)
/// - New questions are provided
///
/// It runs the missing prompt tasks and question tasks in parallel, then re-synthesizes Phase 2.
async fn run_incremental_research(
    topic: &str,
    output_dir: PathBuf,
    mut existing_metadata: ResearchMetadata,
    questions: Vec<(usize, String)>,
    missing_prompts: Vec<MissingPrompt>,
    missing_outputs: Vec<MissingOutput>,
) -> Result<ResearchResult, ResearchError> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let has_missing_prompts = !missing_prompts.is_empty();
    let has_missing_outputs = !missing_outputs.is_empty();
    let has_questions = !questions.is_empty();

    // Print what we're doing
    if !has_missing_prompts && !has_missing_outputs && !has_questions {
        // Nothing to do - should not reach here, but handle gracefully
        return Ok(ResearchResult {
            topic: topic.to_string(),
            output_dir,
            succeeded: 0,
            failed: 0,
            cancelled: false,
            total_time_secs: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_tokens: 0,
        });
    }

    // Build status message
    let mut parts = Vec::new();
    if has_missing_prompts {
        parts.push(format!("Regenerating {} missing prompt(s)", missing_prompts.len()));
    }
    if has_missing_outputs {
        parts.push(format!("Regenerating {} missing output(s)", missing_outputs.len()));
    }
    if has_questions {
        parts.push(format!("Adding {} new question(s)", questions.len()));
    }
    println!("\nIncremental research: {}...\n", parts.join(" and "));

    // Set up cancellation flag for SIGINT handling
    let cancelled = Arc::new(AtomicBool::new(false));

    // Spawn SIGINT handler
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            eprintln!("\n⚠ Received SIGINT, exiting now");
            std::process::exit(130);
        }
    });

    // Initialize providers
    let gemini = gemini::Client::from_env();
    let openai = openai::Client::from_env();
    let zai = providers::zai::Client::from_env().ok();

    // Check if research tools are available
    let use_tools = tools_available();
    if use_tools {
        let plan = std::env::var("BRAVE_PLAN")
            .map(|s| BravePlan::from_string(&s))
            .unwrap_or_default();
        println!(
            "  ✓ Web research tools enabled (BRAVE_API_KEY found, {:?} plan)\n",
            plan
        );
    } else {
        println!("  ⚠ Web research tools disabled (set BRAVE_API_KEY to enable)\n");
    }

    // Extract library context from metadata (clone to owned strings for futures)
    let (package_manager, language, url) = match &existing_metadata.library_info {
        Some(info) => (
            info.package_manager.clone(),
            info.language.clone(),
            info.url.clone(),
        ),
        None => (
            "unknown".to_string(),
            "unknown".to_string(),
            "N/A".to_string(),
        ),
    };

    // Build library info for prompt building
    let library_info = existing_metadata
        .library_info
        .as_ref()
        .map(|info| LibraryInfo {
            package_manager: info.package_manager.clone(),
            language: info.language.clone(),
            url: info.url.clone(),
            repository: info.repository.clone(),
            description: None,
        });
    let lib_info_ref = library_info.as_ref();

    // Clone topic for use in futures
    let topic_owned = topic.to_string();

    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let total = missing_prompts.len() + questions.len();

    // Create tasks for missing standard prompts - with or without tools
    type BoxedFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = PromptTaskResult> + Send>>;
    let mut phase1_futures: Vec<BoxedFuture> = Vec::new();

    if use_tools {
        // Create agents with web research tools
        let search_tool = BraveSearchTool::from_env();
        let scrape_tool = ScreenScrapeTool::new();

        for mp in &missing_prompts {
            let prompt = build_prompt(mp.template, topic, lib_info_ref);
            let task_name = mp.name;
            let filename = mp.filename;

            match mp.name {
                "overview" => {
                    // Use GLM-4.7 if available, otherwise fall back to Gemini
                    if let Some(ref z) = zai {
                        let agent = z
                            .agent(providers::zai::GLM_4_7)
                            .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer.")
                            .tool(search_tool.clone())
                            .tool(scrape_tool.clone())
                            .build();
                        phase1_futures.push(Box::pin(run_agent_prompt_task(
                            task_name,
                            filename,
                            output_dir.clone(),
                            agent,
                            prompt,
                            counter.clone(),
                            total,
                            start_time,
                            cancelled.clone(),
                        )));
                    } else {
                        let agent = gemini
                            .agent("gemini-3-flash-preview")
                            .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer.")
                            .tool(search_tool.clone())
                            .tool(scrape_tool.clone())
                            .build();
                        phase1_futures.push(Box::pin(run_agent_prompt_task(
                            task_name,
                            filename,
                            output_dir.clone(),
                            agent,
                            prompt,
                            counter.clone(),
                            total,
                            start_time,
                            cancelled.clone(),
                        )));
                    }
                }
                "changelog" => {
                    let agent = openai
                        .agent("gpt-5.2")
                        .preamble("You are a research assistant with web search and scraping tools. Search for recent releases, changelogs, and version history. Use 1-3 targeted searches, then synthesize your findings. Do not make excessive tool calls - write your final answer after gathering sufficient information.")
                        .tool(search_tool.clone())
                        .tool(scrape_tool.clone())
                        .build();
                    phase1_futures.push(Box::pin(run_agent_prompt_task(
                        task_name,
                        filename,
                        output_dir.clone(),
                        agent,
                        prompt,
                        counter.clone(),
                        total,
                        start_time,
                        cancelled.clone(),
                    )));
                }
                _ => {
                    let agent = gemini
                        .agent("gemini-3-flash-preview")
                        .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer.")
                        .tool(search_tool.clone())
                        .tool(scrape_tool.clone())
                        .build();
                    phase1_futures.push(Box::pin(run_agent_prompt_task(
                        task_name,
                        filename,
                        output_dir.clone(),
                        agent,
                        prompt,
                        counter.clone(),
                        total,
                        start_time,
                        cancelled.clone(),
                    )));
                }
            }
        }

        // Create question tasks with tools
        for (num, question) in questions.iter() {
            let question_agent = gemini
                .agent("gemini-3-flash-preview")
                .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to find relevant information, then provide a comprehensive answer. Do not make excessive tool calls - synthesize your findings efficiently.")
                .tool(search_tool.clone())
                .tool(scrape_tool.clone())
                .build();

            let prompt = prompts::ADDITIONAL_QUESTION
                .replace("{{topic}}", &topic_owned)
                .replace("{{package_manager}}", &package_manager)
                .replace("{{language}}", &language)
                .replace("{{url}}", &url)
                .replace("{{question}}", question);

            let filename: &'static str = Box::leak(format!("question_{}.md", num).into_boxed_str());
            let name: &'static str = Box::leak(format!("question_{}", num).into_boxed_str());

            phase1_futures.push(Box::pin(run_agent_prompt_task(
                name,
                filename,
                output_dir.clone(),
                question_agent,
                prompt,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )));
        }
    } else {
        // Fallback: Use raw completion models without tools
        for mp in &missing_prompts {
            let prompt = build_prompt(mp.template, topic, lib_info_ref);
            let task_name = mp.name;
            let filename = mp.filename;

            match mp.name {
                "overview" => {
                    // Use GLM-4.7 if available, otherwise fall back to Gemini
                    if let Some(ref z) = zai {
                        let model = z.completion_model(providers::zai::GLM_4_7);
                        phase1_futures.push(Box::pin(run_prompt_task(
                            task_name,
                            filename,
                            output_dir.clone(),
                            model,
                            prompt,
                            counter.clone(),
                            total,
                            start_time,
                            cancelled.clone(),
                        )));
                    } else {
                        let model = gemini.completion_model("gemini-3-flash-preview");
                        phase1_futures.push(Box::pin(run_prompt_task(
                            task_name,
                            filename,
                            output_dir.clone(),
                            model,
                            prompt,
                            counter.clone(),
                            total,
                            start_time,
                            cancelled.clone(),
                        )));
                    }
                }
                "changelog" => {
                    let model = openai.completion_model("gpt-5.2");
                    phase1_futures.push(Box::pin(run_prompt_task(
                        task_name,
                        filename,
                        output_dir.clone(),
                        model,
                        prompt,
                        counter.clone(),
                        total,
                        start_time,
                        cancelled.clone(),
                    )));
                }
                _ => {
                    let model = gemini.completion_model("gemini-3-flash-preview");
                    phase1_futures.push(Box::pin(run_prompt_task(
                        task_name,
                        filename,
                        output_dir.clone(),
                        model,
                        prompt,
                        counter.clone(),
                        total,
                        start_time,
                        cancelled.clone(),
                    )));
                }
            }
        }

        // Create question tasks without tools
        for (num, question) in questions.iter() {
            let question_model = gemini.completion_model("gemini-3-flash-preview");
            phase1_futures.push(Box::pin(run_question_task(
                *num,
                topic_owned.clone(),
                question.clone(),
                package_manager.clone(),
                language.clone(),
                url.clone(),
                output_dir.clone(),
                question_model,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )));
        }
    }

    // Run all Phase 1 tasks in parallel
    let all_results = join_all(phase1_futures).await;

    let succeeded: Vec<_> = all_results
        .iter()
        .filter_map(|r| r.metrics.as_ref())
        .collect();
    let failed = all_results.len() - succeeded.len();

    let was_cancelled = cancelled.load(Ordering::SeqCst);

    println!(
        "\nPhase 1 complete: {}/{} succeeded{}\n",
        succeeded.len(),
        all_results.len(),
        if was_cancelled { " (cancelled)" } else { "" }
    );

    // If cancelled, return early with partial results
    if was_cancelled {
        let total_time = start_time.elapsed().as_secs_f32();
        let total_input: u64 = succeeded.iter().map(|m| m.input_tokens).sum();
        let total_output: u64 = succeeded.iter().map(|m| m.output_tokens).sum();
        let total_tokens: u64 = succeeded.iter().map(|m| m.total_tokens).sum();

        return Ok(ResearchResult {
            topic: topic.to_string(),
            output_dir,
            succeeded: succeeded.len(),
            failed,
            cancelled: true,
            total_time_secs: total_time,
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_tokens,
        });
    }

    // If all prompts failed, return error
    if succeeded.is_empty() && !all_results.is_empty() {
        return Err(ResearchError::AllPromptsFailed);
    }

    // Check if too many prompts failed (require at least 50% success for incremental)
    let min_required = (all_results.len() / 2).max(1);
    if succeeded.len() < min_required && all_results.len() > 1 {
        println!(
            "⚠ Too many prompts failed ({}/{}). Stopping before Phase 2.",
            failed,
            all_results.len()
        );
        return Err(ResearchError::TooManyPromptsFailed {
            succeeded: succeeded.len(),
            total: all_results.len(),
        });
    }

    // Update metadata with new questions
    for (num, question) in &questions {
        let filename = format!("question_{}.md", num);
        existing_metadata.add_additional_file(filename, question.clone());
    }

    // === Phase 2: Re-synthesize with expanded corpus ===
    println!("Phase 2: Re-generating consolidated outputs with new content...\n");

    // Read back all documents
    let overview_content = fs::read_to_string(output_dir.join("overview.md"))
        .await
        .unwrap_or_default();
    let similar_libraries_content = fs::read_to_string(output_dir.join("similar_libraries.md"))
        .await
        .unwrap_or_default();
    let integration_partners_content =
        fs::read_to_string(output_dir.join("integration_partners.md"))
            .await
            .unwrap_or_default();
    let use_cases_content = fs::read_to_string(output_dir.join("use_cases.md"))
        .await
        .unwrap_or_default();
    let changelog_content = fs::read_to_string(output_dir.join("changelog.md"))
        .await
        .unwrap_or_default();

    // Read all additional question files
    let mut additional_content = String::new();
    for filename in existing_metadata.additional_files.keys() {
        if let Ok(content) = fs::read_to_string(output_dir.join(filename)).await
            && !content.is_empty()
        {
            let num = filename
                .strip_prefix("question_")
                .and_then(|s| s.strip_suffix(".md"))
                .unwrap_or("?");
            additional_content.push_str(&format!(
                "\n## Additional Research: Question {}\n\n{}\n",
                num, content
            ));
        }
    }

    // Build context from all phase 1 results
    let combined_context = prompts::CONTEXT
        .replace("{{topic}}", topic)
        .replace("{{overview}}", &overview_content)
        .replace("{{similar_libraries}}", &similar_libraries_content)
        .replace("{{integration_partners}}", &integration_partners_content)
        .replace("{{use_cases}}", &use_cases_content)
        .replace("{{changelog}}", &changelog_content)
        .replace("{{additional_content}}", &additional_content);

    // Build prompts for phase 2
    let skill_prompt = prompts::SKILL
        .replace("{{topic}}", topic)
        .replace("{{context}}", &combined_context);

    let deep_dive_prompt = prompts::DEEP_DIVE
        .replace("{{topic}}", topic)
        .replace("{{context}}", &combined_context);

    // Create skill subdirectory
    let skill_dir = output_dir.join("skill");
    fs::create_dir_all(&skill_dir).await?;

    // Get models for phase 2
    let skill_gen = openai.completion_model("gpt-5.2");
    let deep_dive_gen = openai.completion_model("gpt-5.2");

    let phase2_counter = Arc::new(AtomicUsize::new(0));
    let phase2_start = Instant::now();

    // Run phase 2 prompts in parallel
    let (skill_result, deep_dive_result) = tokio::join!(
        run_prompt_task(
            "skill",
            "SKILL.md",
            skill_dir.clone(),
            skill_gen,
            skill_prompt,
            phase2_counter.clone(),
            2,
            phase2_start,
            cancelled.clone(),
        ),
        run_prompt_task(
            "deep_dive",
            "deep_dive.md",
            output_dir.clone(),
            deep_dive_gen,
            deep_dive_prompt,
            phase2_counter.clone(),
            2,
            phase2_start,
            cancelled.clone(),
        ),
    );

    // Parse skill output and split into multiple files if needed
    if skill_result.metrics.is_some()
        && let Ok(skill_content) = fs::read_to_string(skill_dir.join("SKILL.md")).await
    {
            // Step 1: Split files BEFORE normalization
            let files = if skill_content.contains("--- FILE:") {
                split_into_files(&skill_content)
            } else {
                vec![("SKILL.md".to_string(), skill_content)]
            };

            // Step 2: Selectively normalize files (skip SKILL.md, normalize others)
            for (filename, content) in files {
                let final_content = if filename == "SKILL.md" {
                    // Don't normalize SKILL.md - preserve frontmatter exactly as LLM generated it
                    content
                } else {
                    // Normalize supporting documentation files
                    normalize_markdown(&content)
                };

                let file_path = skill_dir.join(&filename);
                if let Err(e) = fs::write(&file_path, final_content).await {
                    tracing::error!("Failed to write {}: {}", filename, e);
                }
            }

            // Validate SKILL.md frontmatter and extract when_to_use
            let skill_md_path = skill_dir.join("SKILL.md");
            if let Ok(skill_content) = fs::read_to_string(&skill_md_path).await {
                match parse_and_validate_frontmatter(&skill_content) {
                    Ok((frontmatter, _body)) => {
                        tracing::info!("✓ SKILL.md frontmatter is valid");

                        // Update metadata with when_to_use
                        existing_metadata.when_to_use = Some(frontmatter.description.clone());
                        existing_metadata.updated_at = Utc::now();

                        if let Err(e) = existing_metadata.save(&output_dir).await {
                            tracing::error!("Failed to save metadata: {}", e);
                        } else {
                            tracing::info!("✓ Updated metadata.when_to_use");
                        }
                    }
                    Err(e) => {
                        tracing::error!("✗ SKILL.md frontmatter validation failed: {}", e);
                        tracing::error!("  File: {}", skill_md_path.display());
                        tracing::error!("  Please manually fix the frontmatter in SKILL.md");

                        eprintln!("\n⚠️  Warning: SKILL.md frontmatter is invalid");
                        eprintln!("   {}", e);
                        eprintln!("   File: {}", skill_md_path.display());
                        eprintln!(
                            "   The skill may not activate correctly until this is fixed.\n"
                        );
                    }
                }
            }
    }

    // Normalize deep_dive.md if it was generated
    if deep_dive_result.metrics.is_some() {
        let deep_dive_path = output_dir.join("deep_dive.md");
        if let Ok(content) = fs::read_to_string(&deep_dive_path).await {
            let normalized = normalize_markdown(&content);
            if let Err(e) = fs::write(&deep_dive_path, normalized).await {
                tracing::error!("Failed to normalize deep_dive.md: {}", e);
            }
        }
    }

    // === Phase 2b: Generate brief from deep_dive (if successful) ===
    let (brief_text, summary_text) = if deep_dive_result.metrics.is_some() {
        println!("Generating brief summary...\n");

        // Read the deep_dive content
        let deep_dive_content = fs::read_to_string(output_dir.join("deep_dive.md"))
            .await
            .unwrap_or_default();

        let brief_prompt = prompts::BRIEF
            .replace("{{topic}}", topic)
            .replace("{{deep_dive}}", &deep_dive_content);

        let brief_model = gemini.completion_model("gemini-3-flash-preview");

        match brief_model.completion_request(&brief_prompt).send().await {
            Ok(response) => {
                let content: String = response
                    .choice
                    .into_iter()
                    .filter_map(|c| match c {
                        AssistantContent::Text(text) => Some(text.text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let (brief, summary) = parse_brief_response(&content);

                // Write brief.md file
                if let (Some(b), Some(s)) = (&brief, &summary) {
                    let repo_url = library_info
                        .as_ref()
                        .and_then(|li| li.repository.as_ref())
                        .map(|r| format!("repo: {}\n", r))
                        .unwrap_or_default();

                    let brief_content = format!("---\nsummary: {}\n{}---\n\n{}", b, repo_url, s);
                    let _ = fs::write(output_dir.join("brief.md"), brief_content).await;
                    println!("[3/3] brief ✓");
                }

                (brief, summary)
            }
            Err(e) => {
                eprintln!("Warning: Failed to generate brief: {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let phase2_results = [skill_result, deep_dive_result];
    let phase2_succeeded: Vec<_> = phase2_results
        .iter()
        .filter_map(|r| r.metrics.as_ref())
        .collect();
    let phase2_failed = phase2_results.len() - phase2_succeeded.len();

    println!(
        "\nPhase 2 complete: {}/{} succeeded",
        phase2_succeeded.len(),
        phase2_results.len()
    );

    // Update metadata with brief/summary
    existing_metadata.brief = brief_text;
    existing_metadata.summary = summary_text;

    // Save updated metadata
    if let Err(e) = existing_metadata.save(&output_dir).await {
        eprintln!("Warning: Failed to update metadata.json: {}", e);
    }

    // Aggregate all metrics
    let total_time = start_time.elapsed().as_secs_f32();
    let all_metrics: Vec<_> = succeeded.iter().chain(phase2_succeeded.iter()).collect();
    let total_input: u64 = all_metrics.iter().map(|m| m.input_tokens).sum();
    let total_output: u64 = all_metrics.iter().map(|m| m.output_tokens).sum();
    let total_tokens: u64 = all_metrics.iter().map(|m| m.total_tokens).sum();

    Ok(ResearchResult {
        topic: topic.to_string(),
        output_dir,
        succeeded: succeeded.len() + phase2_succeeded.len(),
        failed: failed + phase2_failed,
        cancelled: was_cancelled,
        total_time_secs: total_time,
        total_input_tokens: total_input,
        total_output_tokens: total_output,
        total_tokens,
    })
}

/// Research a topic by running multiple LLM prompts in parallel.
///
/// Generates the following files in the output directory:
/// - `overview.md` - Comprehensive analysis of the topic
/// - `similar_libraries.md` - Comparable libraries and alternatives
/// - `integration_partners.md` - Libraries commonly used with the topic
/// - `use_cases.md` - Common use cases and examples
/// - `question_N.md` - Answers to additional user questions (if provided)
///
/// ## Arguments
/// * `topic` - The Rust crate or topic to research
/// * `output_dir` - Directory where output files will be written. If `None`,
///   defaults to `research/{topic}` relative to the current directory.
/// * `questions` - Additional questions to research in parallel with Phase 1
///
/// List all research topics from the filesystem.
///
/// This function discovers all topics in the research library directory,
/// applies filters, and outputs results in either terminal or JSON format.
///
/// ## Arguments
/// * `filters` - Glob patterns to filter topics (e.g., ["foo*", "bar"])
/// * `types` - Research types to filter (e.g., ["library", "software"])
/// * `verbose` - If true, show detailed sub-bullets with metadata issues
/// * `json` - If true, output as JSON; otherwise use terminal format
///
/// ## Environment Variables
/// * `RESEARCH_DIR` - Base directory for research library (default: `$HOME`)
///
/// ## Returns
/// Returns `Ok(())` on success, or an error if discovery/formatting fails.
///
/// ## Errors
/// Returns an error if:
/// - Neither `RESEARCH_DIR` nor `HOME` environment variable is set
/// - The library directory cannot be read
/// - JSON serialization fails (when `json` is true)
#[instrument(
    name = "list",
    skip(filters, types),
    fields(
        filter_count = filters.len(),
        type_count = types.len(),
        verbose = verbose,
        json = json
    )
)]
pub async fn list(filters: Vec<String>, types: Vec<String>, verbose: bool, json: bool) -> Result<(), String> {
    use list::{apply_filters, discover_topics, format_json, format_terminal};

    // Get RESEARCH_DIR from env (default to HOME)
    let research_dir = std::env::var("RESEARCH_DIR").unwrap_or_else(|_| {
        std::env::var("HOME").expect("Neither RESEARCH_DIR nor HOME environment variable is set")
    });

    // Construct library path: $RESEARCH_DIR/.research/library/
    let library_path = PathBuf::from(research_dir)
        .join(".research")
        .join("library");

    debug!("Searching for topics in: {:?}", library_path);

    // Discover topics
    let topics =
        discover_topics(library_path).map_err(|e| format!("Failed to discover topics: {}", e))?;

    debug!("Found {} topics before filtering", topics.len());

    // Apply filters
    let filtered_topics = apply_filters(topics, &filters, &types)
        .map_err(|e| format!("Failed to apply filters: {}", e))?;

    debug!("Found {} topics after filtering", filtered_topics.len());

    // Determine if we're filtering to a single type (for format_terminal)
    let filter_single_type = types.len() == 1;

    // Format and output to stdout
    if json {
        let output =
            format_json(&filtered_topics).map_err(|e| format!("Failed to format JSON: {}", e))?;
        println!("{}", output);
    } else {
        let output = format_terminal(&filtered_topics, filter_single_type, verbose);
        println!("{}", output);
    }

    Ok(())
}

/// Create symbolic links from research topic skill directories to Claude Code
/// and OpenCode user-scoped skill locations.
///
/// This is a wrapper function that delegates to the link module implementation.
/// It outputs results to stdout in either terminal or JSON format.
///
/// # Arguments
///
/// * `filters` - Glob patterns to filter topics (e.g., "foo", "foo*", "bar")
/// * `types` - Topic types to filter by (e.g., "library", "software")
/// * `json` - If true, output JSON format; otherwise use terminal format
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error string on failure.
///
/// # Example
///
/// ```no_run
/// # use research_lib::link;
/// # async fn example() {
/// // Link all library topics
/// link(vec![], vec!["library".to_string()], false).await.unwrap();
///
/// // Link topics matching "clap*" in JSON format
/// link(vec!["clap*".to_string()], vec![], true).await.unwrap();
/// # }
/// ```
#[instrument(
    skip(filters, types, json),
    fields(
        filter_count = filters.len(),
        type_count = types.len(),
        json = json
    )
)]
pub async fn link(filters: Vec<String>, types: Vec<String>, json: bool) -> Result<(), String> {
    // Delegate to the link module implementation
    let result = link::link(filters, types, json)
        .await
        .map_err(|e| format!("Link operation failed: {}", e))?;

    // TODO: Phase 5 - Format and output results
    // For now, just acknowledge success
    debug!(
        "Link completed: {} processed, {} created, {} failed",
        result.total_processed(),
        result.total_created(),
        result.total_failed()
    );

    Ok(())
}

/// ## Returns
/// A `ResearchResult` containing metrics about the operation
///
/// ## Errors
/// Returns `ResearchError` if the output directory cannot be created
/// or if all prompts fail.
#[instrument(
    name = "research",
    skip(output_dir, questions),
    fields(
        topic = %topic,
        question_count = questions.len(),
        tools_enabled = tracing::field::Empty
    )
)]
pub async fn research(
    topic: &str,
    output_dir: Option<PathBuf>,
    questions: &[String],
) -> Result<ResearchResult, ResearchError> {
    info!("Starting research session");

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Use provided output_dir or default to research/{topic}
    let output_dir = output_dir.unwrap_or_else(|| default_output_dir(topic));

    // Create output directory
    fs::create_dir_all(&output_dir).await?;

    // Check for existing metadata (incremental mode)
    if let Some(existing_metadata) = ResearchMetadata::load(&output_dir).await {
        println!("Found existing research for '{}'", topic);

        // Check for missing standard prompts
        // NOTE: Using deprecated function because research() accepts custom output_dir
        // and doesn't have ResearchType context. This function should be kept until
        // research() is refactored to require ResearchType parameter or can infer it.
        #[allow(deprecated)]
        let missing_prompts = check_missing_standard_prompts(&output_dir).await;
        if !missing_prompts.is_empty() {
            println!("  ⚠ Missing {} standard prompt(s):", missing_prompts.len());
            for mp in &missing_prompts {
                println!("    - {}", mp.filename);
            }
        }

        // Check for missing output files
        // NOTE: Using deprecated function for same reason as above
        #[allow(deprecated)]
        let missing_outputs = check_missing_outputs(&output_dir).await;
        if !missing_outputs.is_empty() {
            println!("  ⚠ Missing {} output file(s):", missing_outputs.len());
            for mo in &missing_outputs {
                println!("    - {}", mo.filename);
            }
        }

        // Check for overlaps and filter questions
        let mut questions_to_run: Vec<(usize, String)> = Vec::new();
        let mut next_num = existing_metadata.next_question_number();

        for question in questions {
            if let Some(conflict_file) = existing_metadata.check_overlap(question) {
                println!(
                    "  ⚠ Question overlaps with existing {}: \"{}\"",
                    conflict_file, question
                );

                // Ask user if they want to include anyway
                let confirm =
                    inquire::Confirm::new(&format!("Include anyway as question_{}?", next_num))
                        .with_default(false)
                        .prompt();

                match confirm {
                    Ok(true) => {
                        questions_to_run.push((next_num, question.clone()));
                        next_num += 1;
                    }
                    Ok(false) => {
                        println!("    Skipping overlapping question");
                    }
                    Err(_) => {
                        println!("    Skipping (cancelled)");
                    }
                }
            } else {
                questions_to_run.push((next_num, question.clone()));
                next_num += 1;
            }
        }

        // If nothing to do (no missing prompts, no missing outputs, and no new questions), return early
        if missing_prompts.is_empty() && missing_outputs.is_empty() && questions_to_run.is_empty() {
            println!("  Research is complete. Use additional prompts to expand research.");
            return Ok(ResearchResult {
                topic: topic.to_string(),
                output_dir,
                succeeded: 0,
                failed: 0,
                cancelled: false,
                total_time_secs: 0.0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_tokens: 0,
            });
        }

        // Run incremental research with missing prompts, missing outputs, and/or new questions
        return run_incremental_research(
            topic,
            output_dir,
            existing_metadata,
            questions_to_run,
            missing_prompts,
            missing_outputs,
        )
        .await;
    }

    // Find the library across package managers and let user select if multiple
    println!("Checking package managers for '{}'...", topic);
    let library_matches = find_library(topic).await;
    let selected = select_library(library_matches, topic);

    // Extract library info for metadata
    let library_info = match &selected {
        LibrarySelection::Selected(info) | LibrarySelection::Single(info) => Some(info.clone()),
        _ => None,
    };

    // Set up cancellation flag for SIGINT handling
    let cancelled = Arc::new(AtomicBool::new(false));

    // Spawn SIGINT handler - exit immediately on Ctrl+C
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            eprintln!("\n⚠ Received SIGINT, exiting now");
            std::process::exit(130); // 128 + SIGINT(2)
        }
    });

    // Initialize providers
    let openai = openai::Client::from_env();
    let gemini = gemini::Client::from_env();
    let zai = zai::Client::from_env().ok();

    // Check if research tools are available
    let use_tools = tools_available();
    Span::current().record("tools_enabled", use_tools);
    if use_tools {
        let plan = std::env::var("BRAVE_PLAN")
            .map(|s| BravePlan::from_string(&s))
            .unwrap_or_default();
        info!(?plan, "Web research tools enabled");
        println!(
            "  ✓ Web research tools enabled (BRAVE_API_KEY found, {:?} plan)\n",
            plan
        );
    } else {
        warn!("Web research tools disabled - set BRAVE_API_KEY to enable");
        println!("  ⚠ Web research tools disabled (set BRAVE_API_KEY to enable)\n");
    }

    // Build prompts from templates with library context
    let lib_info_ref = library_info.as_ref();
    let overview_prompt = build_prompt(prompts::OVERVIEW, topic, lib_info_ref);
    let similar_libraries_prompt = build_prompt(prompts::SIMILAR_LIBRARIES, topic, lib_info_ref);
    let integration_partners_prompt =
        build_prompt(prompts::INTEGRATION_PARTNERS, topic, lib_info_ref);
    let use_cases_prompt = build_prompt(prompts::USE_CASES, topic, lib_info_ref);
    let changelog_prompt = build_prompt(prompts::CHANGELOG, topic, lib_info_ref);

    // Extract library context strings for question tasks (owned for boxed futures)
    let (pkg_mgr, lang, pkg_url) = match &library_info {
        Some(info) => (
            info.package_manager.clone(),
            info.language.clone(),
            info.url.clone(),
        ),
        None => (
            "unknown".to_string(),
            "unknown".to_string(),
            "N/A".to_string(),
        ),
    };
    let topic_owned = topic.to_string();

    let num_questions = questions.len();
    let total = 5 + num_questions; // 5 default prompts + user questions

    // Phase 1 span
    let _phase1_guard =
        info_span!("phase_1", prompt_count = total, tools_enabled = use_tools).entered();

    info!(prompt_count = total, "Beginning parallel prompt execution");
    println!(
        "Phase 1: Running {} research prompts in parallel to {:?}...\n",
        total, output_dir
    );
    println!("  (Press Ctrl+C to cancel and save completed results)\n");

    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));

    // Create Phase 1 tasks - with or without tools
    type BoxedFuture =
        std::pin::Pin<Box<dyn std::future::Future<Output = PromptTaskResult> + Send>>;
    let mut phase1_futures: Vec<BoxedFuture> = Vec::new();

    if use_tools {
        // Create agents with web research tools
        let search_tool = BraveSearchTool::from_env();
        let scrape_tool = ScreenScrapeTool::new();

        // Overview agent (using zai GLM if available, otherwise Gemini)
        if let Some(ref z) = zai {
            let overview_agent = z
                .agent(zai::GLM_4_7)
                .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer.")
                .tool(search_tool.clone())
                .tool(scrape_tool.clone())
                .build();
            phase1_futures.push(Box::pin(run_agent_prompt_task(
                "overview",
                "overview.md",
                output_dir.clone(),
                overview_agent,
                overview_prompt,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )));
        } else {
            let overview_agent = gemini
                .agent("gemini-3-flash-preview")
                .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer.")
                .tool(search_tool.clone())
                .tool(scrape_tool.clone())
                .build();
            phase1_futures.push(Box::pin(run_agent_prompt_task(
                "overview",
                "overview.md",
                output_dir.clone(),
                overview_agent,
                overview_prompt,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )));
        }

        // Similar libraries agent (using Gemini)
        let similar_agent = gemini
            .agent("gemini-3-flash-preview")
            .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer.")
            .tool(search_tool.clone())
            .tool(scrape_tool.clone())
            .build();
        phase1_futures.push(Box::pin(run_agent_prompt_task(
            "similar_libraries",
            "similar_libraries.md",
            output_dir.clone(),
            similar_agent,
            similar_libraries_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        )));

        // Integration partners agent (using Gemini)
        let integration_agent = gemini
            .agent("gemini-3-flash-preview")
            .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer.")
            .tool(search_tool.clone())
            .tool(scrape_tool.clone())
            .build();
        phase1_futures.push(Box::pin(run_agent_prompt_task(
            "integration_partners",
            "integration_partners.md",
            output_dir.clone(),
            integration_agent,
            integration_partners_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        )));

        // Use cases agent (using Gemini)
        let use_cases_agent = gemini
            .agent("gemini-3-flash-preview")
            .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to gather key information, then synthesize your findings into a comprehensive response. Do not make excessive tool calls - gather what you need efficiently and write your final answer.")
            .tool(search_tool.clone())
            .tool(scrape_tool.clone())
            .build();
        phase1_futures.push(Box::pin(run_agent_prompt_task(
            "use_cases",
            "use_cases.md",
            output_dir.clone(),
            use_cases_agent,
            use_cases_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        )));

        // Changelog agent (using OpenAI GPT)
        let changelog_agent = openai
            .agent("gpt-5.2")
            .preamble("You are a research assistant with web search and scraping tools. Search for recent releases, changelogs, and version history. Use 1-3 targeted searches, then synthesize your findings. Do not make excessive tool calls - write your final answer after gathering sufficient information.")
            .tool(search_tool.clone())
            .tool(scrape_tool.clone())
            .build();
        phase1_futures.push(Box::pin(run_agent_prompt_task(
            "changelog",
            "changelog.md",
            output_dir.clone(),
            changelog_agent,
            changelog_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        )));

        // Question agents (using Gemini)
        for (i, question) in questions.iter().enumerate() {
            let question_agent = gemini
                .agent("gemini-3-flash-preview")
                .preamble("You are a research assistant with web search and scraping tools. Use 1-3 targeted searches to find relevant information, then provide a comprehensive answer. Do not make excessive tool calls - synthesize your findings efficiently.")
                .tool(search_tool.clone())
                .tool(scrape_tool.clone())
                .build();

            let ctx = LibraryContext {
                package_manager: &pkg_mgr,
                language: &lang,
                url: &pkg_url,
            };
            let prompt = build_prompt_with_context(prompts::ADDITIONAL_QUESTION, topic, Some(&ctx))
                .replace("{{question}}", question);

            let question_num = i + 1;
            let filename: &'static str =
                Box::leak(format!("question_{}.md", question_num).into_boxed_str());
            let name: &'static str =
                Box::leak(format!("question_{}", question_num).into_boxed_str());

            phase1_futures.push(Box::pin(run_agent_prompt_task(
                name,
                filename,
                output_dir.clone(),
                question_agent,
                prompt,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )));
        }
    } else {
        // Fallback: Use raw completion models without tools
        let gemini1 = gemini.completion_model("gemini-3-flash-preview");
        let gemini2 = gemini.completion_model("gemini-3-flash-preview");
        let gemini3 = gemini.completion_model("gemini-3-flash-preview");
        let changelog_model = openai.completion_model("gpt-5.2");

        // Use GLM-4.7 if available, otherwise fall back to Gemini
        if let Some(ref z) = zai {
            let overview_model = z.completion_model(zai::GLM_4_7);
            phase1_futures.push(Box::pin(run_prompt_task(
                "overview",
                "overview.md",
                output_dir.clone(),
                overview_model,
                overview_prompt,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )));
        } else {
            let overview_model = gemini.completion_model("gemini-3-flash-preview");
            phase1_futures.push(Box::pin(run_prompt_task(
                "overview",
                "overview.md",
                output_dir.clone(),
                overview_model,
                overview_prompt,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )));
        }
        phase1_futures.push(Box::pin(run_prompt_task(
            "similar_libraries",
            "similar_libraries.md",
            output_dir.clone(),
            gemini1,
            similar_libraries_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        )));
        phase1_futures.push(Box::pin(run_prompt_task(
            "integration_partners",
            "integration_partners.md",
            output_dir.clone(),
            gemini2,
            integration_partners_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        )));
        phase1_futures.push(Box::pin(run_prompt_task(
            "use_cases",
            "use_cases.md",
            output_dir.clone(),
            gemini3,
            use_cases_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        )));
        phase1_futures.push(Box::pin(run_prompt_task(
            "changelog",
            "changelog.md",
            output_dir.clone(),
            changelog_model,
            changelog_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        )));

        // Question tasks without tools
        for (i, question) in questions.iter().enumerate() {
            let question_model = gemini.completion_model("gemini-3-flash-preview");
            phase1_futures.push(Box::pin(run_question_task(
                i + 1,
                topic_owned.clone(),
                question.clone(),
                pkg_mgr.clone(),
                lang.clone(),
                pkg_url.clone(),
                output_dir.clone(),
                question_model,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )));
        }
    }

    // Run all Phase 1 tasks in parallel
    let phase1_results = join_all(phase1_futures).await;

    let phase1_succeeded: Vec<_> = phase1_results
        .iter()
        .filter_map(|r| r.metrics.as_ref())
        .collect();
    let phase1_failed = phase1_results.len() - phase1_succeeded.len();

    // Check if cancelled
    let was_cancelled = cancelled.load(Ordering::SeqCst);

    info!(
        succeeded = phase1_succeeded.len(),
        failed = phase1_failed,
        cancelled = was_cancelled,
        "Phase 1 complete"
    );

    // Exit the phase 1 span
    drop(_phase1_guard);

    println!(
        "\nPhase 1 complete: {}/{} succeeded{}\n",
        phase1_succeeded.len(),
        phase1_results.len(),
        if was_cancelled { " (cancelled)" } else { "" }
    );

    if phase1_succeeded.is_empty() {
        return Err(ResearchError::AllPromptsFailed);
    }

    // Check if too many Phase 1 prompts failed (require at least 50% success or all 5 core prompts)
    let core_prompts = 5; // overview, similar_libraries, integration_partners, use_cases, changelog
    let min_required = core_prompts.min(phase1_results.len() / 2 + 1);
    if phase1_succeeded.len() < min_required {
        println!(
            "⚠ Too many Phase 1 prompts failed ({}/{}). Stopping before Phase 2.",
            phase1_failed,
            phase1_results.len()
        );
        return Err(ResearchError::TooManyPromptsFailed {
            succeeded: phase1_succeeded.len(),
            total: phase1_results.len(),
        });
    }

    // If cancelled, skip phase 2 and return partial results
    if was_cancelled {
        let total_time = start_time.elapsed().as_secs_f32();
        let total_input: u64 = phase1_succeeded.iter().map(|m| m.input_tokens).sum();
        let total_output: u64 = phase1_succeeded.iter().map(|m| m.output_tokens).sum();
        let total_tokens: u64 = phase1_succeeded.iter().map(|m| m.total_tokens).sum();

        return Ok(ResearchResult {
            topic: topic.to_string(),
            output_dir,
            succeeded: phase1_succeeded.len(),
            failed: phase1_failed,
            cancelled: true,
            total_time_secs: total_time,
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_tokens,
        });
    }

    // === Phase 2: Read initial documents and generate consolidated outputs ===
    let _phase2_guard = info_span!("phase_2").entered();
    info!("Generating consolidated outputs");
    println!("Phase 2: Generating consolidated outputs...\n");

    // Read back the initial documents
    let overview_content = fs::read_to_string(output_dir.join("overview.md"))
        .await
        .unwrap_or_default();
    let similar_libraries_content = fs::read_to_string(output_dir.join("similar_libraries.md"))
        .await
        .unwrap_or_default();
    let integration_partners_content =
        fs::read_to_string(output_dir.join("integration_partners.md"))
            .await
            .unwrap_or_default();
    let use_cases_content = fs::read_to_string(output_dir.join("use_cases.md"))
        .await
        .unwrap_or_default();
    let changelog_content = fs::read_to_string(output_dir.join("changelog.md"))
        .await
        .unwrap_or_default();

    // Read additional question files and build additional content
    let mut additional_content = String::new();
    for i in 1..=num_questions {
        let filename = format!("question_{}.md", i);
        if let Ok(content) = fs::read_to_string(output_dir.join(&filename)).await
            && !content.is_empty()
        {
            additional_content.push_str(&format!(
                "\n## Additional Research: Question {}\n\n{}\n",
                i, content
            ));
        }
    }

    // Build context from phase 1 results
    let combined_context = prompts::CONTEXT
        .replace("{{topic}}", topic)
        .replace("{{overview}}", &overview_content)
        .replace("{{similar_libraries}}", &similar_libraries_content)
        .replace("{{integration_partners}}", &integration_partners_content)
        .replace("{{use_cases}}", &use_cases_content)
        .replace("{{changelog}}", &changelog_content)
        .replace("{{additional_content}}", &additional_content);

    // Build prompts for phase 2 from templates
    let skill_prompt = prompts::SKILL
        .replace("{{topic}}", topic)
        .replace("{{context}}", &combined_context);

    let deep_dive_prompt = prompts::DEEP_DIVE
        .replace("{{topic}}", topic)
        .replace("{{context}}", &combined_context);

    // Create skill subdirectory
    let skill_dir = output_dir.join("skill");
    fs::create_dir_all(&skill_dir).await?;

    // Get fresh model instances for phase 2
    let skill_gen = openai.completion_model("gpt-5.2");
    let deep_dive_gen = openai.completion_model("gpt-5.2");

    let phase2_counter = Arc::new(AtomicUsize::new(0));
    let phase2_start = Instant::now();

    // Run phase 2 prompts in parallel
    let (skill_result, deep_dive_result) = tokio::join!(
        run_prompt_task(
            "skill",
            "SKILL.md",
            skill_dir.clone(),
            skill_gen,
            skill_prompt,
            phase2_counter.clone(),
            2,
            phase2_start,
            cancelled.clone(),
        ),
        run_prompt_task(
            "deep_dive",
            "deep_dive.md",
            output_dir.clone(),
            deep_dive_gen,
            deep_dive_prompt,
            phase2_counter.clone(),
            2,
            phase2_start,
            cancelled.clone(),
        ),
    );

    // Parse skill output and split into multiple files if needed
    // Also validate frontmatter and extract when_to_use for metadata
    let mut when_to_use: Option<String> = None;

    if skill_result.metrics.is_some()
        && let Ok(skill_content) = fs::read_to_string(skill_dir.join("SKILL.md")).await
    {
        // Step 1: Split files BEFORE normalization
        let files = if skill_content.contains("--- FILE:") {
                split_into_files(&skill_content)
            } else {
                vec![("SKILL.md".to_string(), skill_content)]
            };

            // Step 2: Selectively normalize files (skip SKILL.md, normalize others)
            for (filename, content) in files {
                let final_content = if filename == "SKILL.md" {
                    // Don't normalize SKILL.md - preserve frontmatter exactly as LLM generated it
                    content
                } else {
                    // Normalize supporting documentation files
                    normalize_markdown(&content)
                };

                let file_path = skill_dir.join(&filename);
                if let Err(e) = fs::write(&file_path, final_content).await {
                    tracing::error!("Failed to write {}: {}", filename, e);
                }
            }

            // Validate SKILL.md frontmatter and extract when_to_use
            let skill_md_path = skill_dir.join("SKILL.md");
            if let Ok(skill_content) = fs::read_to_string(&skill_md_path).await {
                match parse_and_validate_frontmatter(&skill_content) {
                    Ok((frontmatter, _body)) => {
                        tracing::info!("✓ SKILL.md frontmatter is valid");
                        when_to_use = Some(frontmatter.description.clone());
                        tracing::info!("✓ Extracted when_to_use from frontmatter");
                    }
                    Err(e) => {
                        tracing::error!("✗ SKILL.md frontmatter validation failed: {}", e);
                        tracing::error!("  File: {}", skill_md_path.display());
                        tracing::error!("  Please manually fix the frontmatter in SKILL.md");

                        eprintln!("\n⚠️  Warning: SKILL.md frontmatter is invalid");
                        eprintln!("   {}", e);
                        eprintln!("   File: {}", skill_md_path.display());
                        eprintln!(
                            "   The skill may not activate correctly until this is fixed.\n"
                        );
                    }
                }
            }
    }

    // Normalize deep_dive.md if it was generated
    if deep_dive_result.metrics.is_some() {
        let deep_dive_path = output_dir.join("deep_dive.md");
        if let Ok(content) = fs::read_to_string(&deep_dive_path).await {
            let normalized = normalize_markdown(&content);
            if let Err(e) = fs::write(&deep_dive_path, normalized).await {
                tracing::error!("Failed to normalize deep_dive.md: {}", e);
            }
        }
    }

    // === Phase 2b: Generate brief from deep_dive (if successful) ===
    let (brief_text, summary_text) = if deep_dive_result.metrics.is_some() {
        println!("Generating brief summary...\n");

        // Read the deep_dive content
        let deep_dive_content = fs::read_to_string(output_dir.join("deep_dive.md"))
            .await
            .unwrap_or_default();

        let brief_prompt = prompts::BRIEF
            .replace("{{topic}}", topic)
            .replace("{{deep_dive}}", &deep_dive_content);

        let brief_model = gemini.completion_model("gemini-3-flash-preview");

        match brief_model.completion_request(&brief_prompt).send().await {
            Ok(response) => {
                let content: String = response
                    .choice
                    .into_iter()
                    .filter_map(|c| match c {
                        AssistantContent::Text(text) => Some(text.text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let (brief, summary) = parse_brief_response(&content);

                // Write brief.md file
                if let (Some(b), Some(s)) = (&brief, &summary) {
                    let repo_url = library_info
                        .as_ref()
                        .and_then(|li| li.repository.as_ref())
                        .map(|r| format!("repo: {}\n", r))
                        .unwrap_or_default();

                    let brief_content = format!("---\nsummary: {}\n{}---\n\n{}", b, repo_url, s);
                    let _ = fs::write(output_dir.join("brief.md"), brief_content).await;
                    println!("[3/3] brief ✓");
                }

                (brief, summary)
            }
            Err(e) => {
                eprintln!("Warning: Failed to generate brief: {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let phase2_results = [skill_result, deep_dive_result];
    let phase2_succeeded: Vec<_> = phase2_results
        .iter()
        .filter_map(|r| r.metrics.as_ref())
        .collect();
    let phase2_failed = phase2_results.len() - phase2_succeeded.len();

    // Check if cancelled during phase 2
    let was_cancelled = cancelled.load(Ordering::SeqCst);

    println!(
        "\nPhase 2 complete: {}/{} succeeded{}",
        phase2_succeeded.len(),
        phase2_results.len(),
        if was_cancelled { " (cancelled)" } else { "" }
    );

    // Aggregate all metrics
    let total_time = start_time.elapsed().as_secs_f32();
    let all_metrics: Vec<_> = phase1_succeeded
        .iter()
        .chain(phase2_succeeded.iter())
        .collect();
    let total_input: u64 = all_metrics.iter().map(|m| m.input_tokens).sum();
    let total_output: u64 = all_metrics.iter().map(|m| m.output_tokens).sum();
    let total_tokens: u64 = all_metrics.iter().map(|m| m.total_tokens).sum();

    // Write metadata.json
    let mut metadata = ResearchMetadata::new_library(library_info.as_ref());
    metadata.brief = brief_text;
    metadata.summary = summary_text;
    metadata.when_to_use = when_to_use;
    for (i, question) in questions.iter().enumerate() {
        let filename = format!("question_{}.md", i + 1);
        metadata.add_additional_file(filename, question.clone());
    }
    if let Err(e) = metadata.save(&output_dir).await {
        eprintln!("Warning: Failed to write metadata.json: {}", e);
    } else if metadata.when_to_use.is_some() {
        tracing::info!("✓ Updated metadata.when_to_use");
    }

    // Exit the phase 2 span
    drop(_phase2_guard);

    info!(
        total_time_secs = total_time,
        total_tokens,
        succeeded = phase1_succeeded.len() + phase2_succeeded.len(),
        failed = phase1_failed + phase2_failed,
        "Research complete"
    );

    Ok(ResearchResult {
        topic: topic.to_string(),
        output_dir,
        succeeded: phase1_succeeded.len() + phase2_succeeded.len(),
        failed: phase1_failed + phase2_failed,
        cancelled: was_cancelled,
        total_time_secs: total_time,
        total_input_tokens: total_input,
        total_output_tokens: total_output,
        total_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ===========================================
    // Tests for ResearchMetadata
    // ===========================================

    #[test]
    fn test_metadata_new_library() {
        let lib_info = LibraryInfo {
            package_manager: "crates.io".to_string(),
            language: "Rust".to_string(),
            url: "https://crates.io/crates/tokio".to_string(),
            repository: Some("https://github.com/tokio-rs/tokio".to_string()),
            description: Some("Async runtime".to_string()),
        };

        let metadata = ResearchMetadata::new_library(Some(&lib_info));

        assert_eq!(metadata.kind, ResearchKind::Library);
        assert!(metadata.library_info.is_some());
        let info = metadata.library_info.unwrap();
        assert_eq!(info.package_manager, "crates.io");
        assert_eq!(info.language, "Rust");
        assert!(metadata.additional_files.is_empty());
    }

    #[test]
    fn test_metadata_new_library_without_info() {
        let metadata = ResearchMetadata::new_library(None);

        assert_eq!(metadata.kind, ResearchKind::Library);
        assert!(metadata.library_info.is_none());
        assert!(metadata.additional_files.is_empty());
    }

    #[test]
    fn test_metadata_add_additional_file() {
        let mut metadata = ResearchMetadata::new_library(None);
        let original_updated = metadata.updated_at;

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        metadata.add_additional_file(
            "question_1.md".to_string(),
            "What are the best practices?".to_string(),
        );

        assert_eq!(metadata.additional_files.len(), 1);
        assert!(metadata.additional_files.contains_key("question_1.md"));
        assert!(metadata.updated_at >= original_updated);
    }

    #[test]
    fn test_metadata_next_question_number_empty() {
        let metadata = ResearchMetadata::new_library(None);
        assert_eq!(metadata.next_question_number(), 1);
    }

    #[test]
    fn test_metadata_next_question_number_sequential() {
        let mut metadata = ResearchMetadata::new_library(None);
        metadata.add_additional_file("question_1.md".to_string(), "Q1".to_string());
        metadata.add_additional_file("question_2.md".to_string(), "Q2".to_string());

        assert_eq!(metadata.next_question_number(), 3);
    }

    #[test]
    fn test_metadata_next_question_number_gaps() {
        let mut metadata = ResearchMetadata::new_library(None);
        metadata.add_additional_file("question_1.md".to_string(), "Q1".to_string());
        metadata.add_additional_file("question_5.md".to_string(), "Q5".to_string());

        // Should return max + 1, even with gaps
        assert_eq!(metadata.next_question_number(), 6);
    }

    #[test]
    fn test_metadata_next_question_number_ignores_non_questions() {
        let mut metadata = ResearchMetadata::new_library(None);
        metadata.add_additional_file("question_1.md".to_string(), "Q1".to_string());
        metadata.add_additional_file("overview.md".to_string(), "Overview".to_string());
        metadata.add_additional_file("random_file.md".to_string(), "Random".to_string());

        assert_eq!(metadata.next_question_number(), 2);
    }

    #[test]
    fn test_metadata_check_overlap_no_overlap() {
        let mut metadata = ResearchMetadata::new_library(None);
        metadata.add_additional_file(
            "question_1.md".to_string(),
            "What are the performance characteristics of async runtimes?".to_string(),
        );

        let result = metadata.check_overlap("How do I handle errors in database connections?");
        assert!(result.is_none());
    }

    #[test]
    fn test_metadata_check_overlap_with_overlap() {
        let mut metadata = ResearchMetadata::new_library(None);
        // Use words without punctuation for accurate matching
        metadata.add_additional_file(
            "question_1.md".to_string(),
            "performance characteristics async runtimes handling".to_string(),
        );

        // This has significant word overlap (performance, async, runtimes)
        // 3 out of 5 words match = 60% > 50% threshold
        let result = metadata.check_overlap("async runtimes performance features testing");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "question_1.md");
    }

    #[test]
    fn test_metadata_check_overlap_short_words_ignored() {
        let mut metadata = ResearchMetadata::new_library(None);
        metadata.add_additional_file(
            "question_1.md".to_string(),
            "the and but for are".to_string(), // All words <= 3 chars
        );

        // Short words (<= 3 chars) should be ignored, resulting in empty sets
        let result = metadata.check_overlap("the and but for are");
        assert!(result.is_none()); // No overlap because short words are filtered out
    }

    // ===========================================
    // Tests for normalize_markdown
    // ===========================================

    #[test]
    fn test_normalize_markdown_basic() {
        let input = "# Hello\n\nWorld";
        let output = normalize_markdown(input);
        assert!(output.contains("# Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn test_normalize_markdown_handles_html() {
        // HTML blocks are preserved by pulldown-cmark as raw blocks
        // The normalize function processes them but may preserve HTML depending on structure
        let input = "<a name=\"section\"></a>\n\n# Title\n\nContent";
        let output = normalize_markdown(input);

        // Verify the markdown structure is preserved
        assert!(output.contains("# Title"));
        assert!(output.contains("Content"));
    }

    #[test]
    fn test_normalize_markdown_preserves_links() {
        let input = "Check out [this link](https://example.com)";
        let output = normalize_markdown(input);
        assert!(output.contains("[this link](https://example.com)"));
    }

    #[test]
    fn test_normalize_markdown_handles_tables() {
        let input = "| Col1 | Col2 |\n|------|------|\n| A | B |";
        let output = normalize_markdown(input);
        assert!(output.contains("Col1"));
        assert!(output.contains("Col2"));
    }

    #[test]
    fn test_normalize_markdown_handles_code_blocks() {
        let input = "```rust\nfn main() {}\n```";
        let output = normalize_markdown(input);
        assert!(output.contains("fn main()"));
    }

    // ===========================================
    // Tests for build_prompt
    // ===========================================

    #[test]
    fn test_build_prompt_replaces_topic() {
        let template = "Research the {{topic}} library.";
        let result = build_prompt(template, "tokio", None);
        assert_eq!(result, "Research the tokio library.");
    }

    #[test]
    fn test_build_prompt_with_library_info() {
        let template =
            "{{topic}} is available on {{package_manager}} for {{language}}. URL: {{url}}";
        let lib_info = LibraryInfo {
            package_manager: "crates.io".to_string(),
            language: "Rust".to_string(),
            url: "https://crates.io/crates/tokio".to_string(),
            repository: None,
            description: None,
        };

        let result = build_prompt(template, "tokio", Some(&lib_info));
        assert_eq!(
            result,
            "tokio is available on crates.io for Rust. URL: https://crates.io/crates/tokio"
        );
    }

    #[test]
    fn test_build_prompt_without_library_info() {
        let template = "{{topic}} from {{package_manager}} ({{language}})";
        let result = build_prompt(template, "something", None);
        assert_eq!(result, "something from unknown (unknown)");
    }

    // ===========================================
    // Tests for check_missing_standard_prompts
    // ===========================================

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_check_missing_prompts_all_missing() {
        let temp = tempdir().unwrap();
        let missing = check_missing_standard_prompts(temp.path()).await;

        // All 5 standard prompts should be missing
        assert_eq!(missing.len(), 5);

        let names: Vec<_> = missing.iter().map(|m| m.name).collect();
        assert!(names.contains(&"overview"));
        assert!(names.contains(&"similar_libraries"));
        assert!(names.contains(&"integration_partners"));
        assert!(names.contains(&"use_cases"));
        assert!(names.contains(&"changelog"));
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_check_missing_prompts_some_present() {
        let temp = tempdir().unwrap();

        // Create some of the files
        std::fs::write(temp.path().join("overview.md"), "# Overview").unwrap();
        std::fs::write(temp.path().join("changelog.md"), "# Changelog").unwrap();

        let missing = check_missing_standard_prompts(temp.path()).await;

        // Only 3 should be missing
        assert_eq!(missing.len(), 3);

        let names: Vec<_> = missing.iter().map(|m| m.name).collect();
        assert!(!names.contains(&"overview"));
        assert!(!names.contains(&"changelog"));
        assert!(names.contains(&"similar_libraries"));
        assert!(names.contains(&"integration_partners"));
        assert!(names.contains(&"use_cases"));
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_check_missing_prompts_all_present() {
        let temp = tempdir().unwrap();

        // Create all standard files
        std::fs::write(temp.path().join("overview.md"), "# Overview").unwrap();
        std::fs::write(temp.path().join("similar_libraries.md"), "# Similar").unwrap();
        std::fs::write(temp.path().join("integration_partners.md"), "# Partners").unwrap();
        std::fs::write(temp.path().join("use_cases.md"), "# Use Cases").unwrap();
        std::fs::write(temp.path().join("changelog.md"), "# Changelog").unwrap();

        let missing = check_missing_standard_prompts(temp.path()).await;

        // None should be missing
        assert!(missing.is_empty());
    }

    // ===========================================
    // Tests for STANDARD_PROMPTS constant
    // ===========================================

    #[test]
    fn test_standard_prompts_count() {
        assert_eq!(STANDARD_PROMPTS.len(), 5);
    }

    #[test]
    fn test_standard_prompts_all_have_content() {
        for (name, filename, template) in STANDARD_PROMPTS {
            assert!(!name.is_empty(), "Name should not be empty");
            assert!(filename.ends_with(".md"), "Filename should end with .md");
            assert!(!template.is_empty(), "Template should not be empty");
            assert!(
                template.contains("{{topic}}"),
                "Template for {} should contain {{{{topic}}}}",
                name
            );
        }
    }

    // ===========================================
    // Tests for MissingPrompt struct
    // ===========================================

    #[test]
    fn test_missing_prompt_clone() {
        let mp = MissingPrompt {
            name: "overview",
            filename: "overview.md",
            template: "Template content",
        };

        let cloned = mp.clone();
        assert_eq!(cloned.name, mp.name);
        assert_eq!(cloned.filename, mp.filename);
        assert_eq!(cloned.template, mp.template);
    }

    // ===========================================
    // Tests for LibraryInfo Display
    // ===========================================

    #[test]
    fn test_library_info_display_without_description() {
        let info = LibraryInfo {
            package_manager: "crates.io".to_string(),
            language: "Rust".to_string(),
            url: "https://crates.io/crates/test".to_string(),
            repository: None,
            description: None,
        };

        let display = format!("{}", info);
        assert_eq!(display, "crates.io (Rust)");
    }

    #[test]
    fn test_library_info_display_with_short_description() {
        let info = LibraryInfo {
            package_manager: "npm".to_string(),
            language: "JavaScript".to_string(),
            url: "https://npmjs.com/package/test".to_string(),
            repository: None,
            description: Some("A test package".to_string()),
        };

        let display = format!("{}", info);
        assert_eq!(display, "npm (JavaScript) - A test package");
    }

    #[test]
    fn test_library_info_display_with_long_description() {
        let long_desc = "A".repeat(100);
        let info = LibraryInfo {
            package_manager: "PyPI".to_string(),
            language: "Python".to_string(),
            url: "https://pypi.org/project/test".to_string(),
            repository: None,
            description: Some(long_desc),
        };

        let display = format!("{}", info);
        // Should be truncated to 60 chars with "..."
        assert!(display.contains("..."));
        assert!(display.len() < 100);
    }

    // ===========================================
    // Tests for default_output_dir
    // ===========================================

    #[test]
    fn test_default_output_dir_structure() {
        let dir = default_output_dir("tokio");
        let path_str = dir.to_string_lossy();

        assert!(path_str.contains(".research"));
        assert!(path_str.contains("library"));
        assert!(path_str.contains("tokio"));
    }

    // ===========================================
    // Tests for ResearchMetadata serialization
    // ===========================================

    #[tokio::test]
    async fn test_metadata_save_and_load() {
        let temp = tempdir().unwrap();

        let lib_info = LibraryInfo {
            package_manager: "crates.io".to_string(),
            language: "Rust".to_string(),
            url: "https://crates.io/crates/tokio".to_string(),
            repository: None,
            description: None,
        };

        let mut metadata = ResearchMetadata::new_library(Some(&lib_info));
        metadata.add_additional_file("question_1.md".to_string(), "Test question".to_string());

        // Save
        metadata.save(temp.path()).await.unwrap();

        // Load
        let loaded = ResearchMetadata::load(temp.path()).await;
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.kind, ResearchKind::Library);
        assert!(loaded.library_info.is_some());
        assert_eq!(loaded.additional_files.len(), 1);
    }

    #[tokio::test]
    async fn test_metadata_load_nonexistent() {
        let temp = tempdir().unwrap();
        let loaded = ResearchMetadata::load(temp.path()).await;
        assert!(loaded.is_none());
    }

    // ===========================================
    // Tests for PromptMetrics
    // ===========================================

    #[test]
    fn test_prompt_metrics_default() {
        let metrics = PromptMetrics::default();
        assert_eq!(metrics.input_tokens, 0);
        assert_eq!(metrics.output_tokens, 0);
        assert_eq!(metrics.total_tokens, 0);
        assert_eq!(metrics.elapsed_secs, 0.0);
    }

    // ===========================================
    // Tests for ResearchResult
    // ===========================================

    #[test]
    fn test_research_result_debug() {
        let result = ResearchResult {
            topic: "test".to_string(),
            output_dir: PathBuf::from("/tmp/test"),
            succeeded: 5,
            failed: 0,
            cancelled: false,
            total_time_secs: 10.5,
            total_input_tokens: 1000,
            total_output_tokens: 2000,
            total_tokens: 3000,
        };

        let debug = format!("{:?}", result);
        assert!(debug.contains("test"));
        assert!(debug.contains("5"));
    }

    // ===========================================
    // Tests for LibraryInfoMetadata conversion
    // ===========================================

    #[test]
    fn test_library_info_metadata_from() {
        let lib_info = LibraryInfo {
            package_manager: "npm".to_string(),
            language: "TypeScript".to_string(),
            url: "https://npmjs.com/package/test".to_string(),
            repository: Some("https://github.com/test/test".to_string()),
            description: Some("Test description".to_string()),
        };

        let metadata: LibraryInfoMetadata = (&lib_info).into();

        assert_eq!(metadata.package_manager, "npm");
        assert_eq!(metadata.language, "TypeScript");
        assert_eq!(metadata.url, "https://npmjs.com/package/test");
        assert_eq!(
            metadata.repository,
            Some("https://github.com/test/test".to_string())
        );
        // Note: description is not included in metadata
    }

    // ===========================================
    // Tests for OverlapVerdict
    // ===========================================

    #[test]
    fn test_overlap_verdict_equality() {
        assert_eq!(OverlapVerdict::New, OverlapVerdict::New);
        assert_eq!(OverlapVerdict::Conflict, OverlapVerdict::Conflict);
        assert_ne!(OverlapVerdict::New, OverlapVerdict::Conflict);
    }

    // ===========================================
    // Tests for split_into_files
    // ===========================================

    #[test]
    fn test_split_into_files_single_file_no_separators() {
        let content = "---\ntitle: Test Skill\n---\n\n# Test Content\n\nSome content here.";
        let files = split_into_files(content);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "SKILL.md");
        assert!(files[0].1.contains("Test Content"));
    }

    #[test]
    fn test_split_into_files_multiple_files() {
        let content = r#"---
title: Test Skill
---

# Main Content

--- FILE: advanced-usage.md ---

# Advanced Usage

Some advanced content.

--- FILE: examples.md ---

# Examples

Example content here."#;

        let files = split_into_files(content);

        assert_eq!(files.len(), 3);
        assert_eq!(files[0].0, "SKILL.md");
        assert!(files[0].1.contains("Main Content"));

        assert_eq!(files[1].0, "advanced-usage.md");
        assert!(files[1].1.contains("Advanced Usage"));

        assert_eq!(files[2].0, "examples.md");
        assert!(files[2].1.contains("Examples"));
    }

    #[test]
    fn test_split_into_files_empty_content_between_separators() {
        let content = r#"---
title: Test Skill
---

# Main Content

--- FILE: empty.md ---

--- FILE: real.md ---

# Real Content"#;

        let files = split_into_files(content);

        // Empty file should be skipped
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "SKILL.md");
        assert_eq!(files[1].0, "real.md");
        assert!(files[1].1.contains("Real Content"));
    }

    #[test]
    fn test_split_into_files_separator_at_end() {
        let content = r#"---
title: Test Skill
---

# Main Content

--- FILE: additional.md ---

# Additional Content

Last line."#;

        let files = split_into_files(content);

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "SKILL.md");
        assert_eq!(files[1].0, "additional.md");
        assert!(files[1].1.contains("Last line"));
    }

    #[test]
    fn test_split_into_files_starting_with_separator() {
        // This is the current bug scenario where LLM output starts with a separator
        let content = r#"--- FILE: SKILL.md ---
---
title: Test Skill
---

# Main Content

--- FILE: advanced.md ---

# Advanced"#;

        let files = split_into_files(content);

        // First file should still be SKILL.md (the implicit one)
        // But it will be empty, so it gets skipped
        // Then we get SKILL.md from the separator
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "SKILL.md");
        assert!(files[0].1.contains("Main Content"));
        assert_eq!(files[1].0, "advanced.md");
    }

    #[test]
    fn test_split_into_files_whitespace_handling() {
        let content = r#"

---
title: Test
---

# Content

--- FILE:   spaces.md   ---

Content with spaces in separator."#;

        let files = split_into_files(content);

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "SKILL.md");
        // Filename should be trimmed
        assert_eq!(files[1].0, "spaces.md");
    }

    // ===========================================
    // Tests for Tool Integration
    // ===========================================

    #[test]
    fn test_tools_available_returns_true_when_api_key_set() {
        // Save original value if set
        let original = std::env::var("BRAVE_API_KEY").ok();

        // SAFETY: This is a single-threaded test, no concurrent access to env vars
        unsafe {
            std::env::set_var("BRAVE_API_KEY", "test-key");
        }
        assert!(
            tools_available(),
            "tools_available should return true when BRAVE_API_KEY is set"
        );

        // Restore original value
        // SAFETY: This is a single-threaded test, no concurrent access to env vars
        unsafe {
            match original {
                Some(val) => std::env::set_var("BRAVE_API_KEY", val),
                None => std::env::remove_var("BRAVE_API_KEY"),
            }
        }
    }

    #[test]
    fn test_tools_available_returns_false_when_api_key_not_set() {
        // Save original value if set
        let original = std::env::var("BRAVE_API_KEY").ok();

        // SAFETY: This is a single-threaded test, no concurrent access to env vars
        unsafe {
            std::env::remove_var("BRAVE_API_KEY");
        }
        assert!(
            !tools_available(),
            "tools_available should return false when BRAVE_API_KEY is not set"
        );

        // Restore original value
        // SAFETY: This is a single-threaded test, no concurrent access to env vars
        if let Some(val) = original {
            unsafe {
                std::env::set_var("BRAVE_API_KEY", val);
            }
        }
    }

    #[test]
    fn test_tools_available_handles_empty_api_key() {
        // Save original value if set
        let original = std::env::var("BRAVE_API_KEY").ok();

        // SAFETY: This is a single-threaded test, no concurrent access to env vars
        unsafe {
            // Set to empty string - this should still count as "set" in Rust's env::var
            std::env::set_var("BRAVE_API_KEY", "");
        }
        assert!(
            tools_available(),
            "tools_available should return true for empty BRAVE_API_KEY (env var exists)"
        );

        // Restore original value
        // SAFETY: This is a single-threaded test, no concurrent access to env vars
        unsafe {
            match original {
                Some(val) => std::env::set_var("BRAVE_API_KEY", val),
                None => std::env::remove_var("BRAVE_API_KEY"),
            }
        }
    }
}
