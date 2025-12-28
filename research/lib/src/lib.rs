//! Research Library - Automated research on software libraries
//!
//! This library provides tools for automated research on software libraries,
//! running multiple LLM prompts in parallel and saving results.

pub mod providers;

use futures::future::join_all;
use inquire::{InquireError, Select};
use providers::zai;
use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::cmark;
use reqwest::Client as HttpClient;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{AssistantContent, CompletionModel};
use rig::providers::{gemini, openai};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::fs;

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
}

/// Information about a library found in a package manager
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub package_manager: String,
    pub language: String,
    pub url: String,
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
}

/// Library info stored in metadata (serializable version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInfoMetadata {
    pub package_manager: String,
    pub language: String,
    pub url: String,
}

impl From<&LibraryInfo> for LibraryInfoMetadata {
    fn from(info: &LibraryInfo) -> Self {
        Self {
            package_manager: info.package_manager.clone(),
            language: info.language.clone(),
            url: info.url.clone(),
        }
    }
}

impl ResearchMetadata {
    /// Create new metadata for library research
    pub fn new_library(library_info: Option<&LibraryInfo>) -> Self {
        let now = Utc::now();
        Self {
            kind: ResearchKind::Library,
            library_info: library_info.map(LibraryInfoMetadata::from),
            additional_files: std::collections::HashMap::new(),
            created_at: now,
            updated_at: now,
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
}

/// Response from npm registry API
#[derive(Debug, Deserialize)]
struct NpmResponse {
    description: Option<String>,
}

/// Response from PyPI API
#[derive(Debug, Deserialize)]
struct PyPIResponse {
    info: Option<PyPIInfo>,
}

#[derive(Debug, Deserialize)]
struct PyPIInfo {
    summary: Option<String>,
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

    Some(LibraryInfo {
        package_manager: "crates.io".to_string(),
        language: "Rust".to_string(),
        url: format!("https://crates.io/crates/{}", name),
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

    Some(LibraryInfo {
        package_manager: "npm".to_string(),
        language: "JavaScript/TypeScript".to_string(),
        url: format!("https://www.npmjs.com/package/{}", name),
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
    let description = data.info.and_then(|i| i.summary);

    Some(LibraryInfo {
        package_manager: "PyPI".to_string(),
        language: "Python".to_string(),
        url: format!("https://pypi.org/project/{}", name),
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
        let package_name = r.name.split('/').last().unwrap_or(&r.name);
        package_name == name
    })?;

    Some(LibraryInfo {
        package_manager: "Packagist".to_string(),
        language: "PHP".to_string(),
        url: matching
            .url
            .unwrap_or_else(|| format!("https://packagist.org/packages/{}", matching.name)),
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

        if let Some(resp) = response {
            if resp.status().is_success() {
                return Some(LibraryInfo {
                    package_manager: "pkg.go.dev".to_string(),
                    language: "Go".to_string(),
                    url,
                    description: None,
                });
            }
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
            println!("  ⚠ '{}' not found on any package manager (may be a general topic)\n", topic);
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
                    println!("\n  → Selected: {} ({})\n", lib.package_manager, lib.language);
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

/// Result of a single prompt task
struct PromptTaskResult {
    metrics: Option<PromptMetrics>,
}

/// Run a prompt task and save result, printing progress as it completes
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

            let normalized = normalize_markdown(&content);

            let path = output_dir.join(filename);
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
async fn run_question_task<M>(
    question_num: usize,
    topic: &str,
    question: &str,
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

    let prompt = prompts::ADDITIONAL_QUESTION
        .replace("{{topic}}", topic)
        .replace("{{question}}", question);

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
                        completed, total, name, elapsed, metrics.input_tokens, metrics.output_tokens, metrics.total_tokens,
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

/// Run incremental research by adding new questions to existing research.
///
/// This is called when metadata.json exists and new questions are provided.
/// It runs only the new question tasks, then re-synthesizes Phase 2.
async fn run_incremental_research(
    topic: &str,
    output_dir: PathBuf,
    mut existing_metadata: ResearchMetadata,
    questions: Vec<(usize, String)>,
) -> Result<ResearchResult, ResearchError> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    println!(
        "\nIncremental research: Adding {} new question(s)...\n",
        questions.len()
    );

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

    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let total = questions.len();

    // Create question tasks
    let question_tasks: Vec<_> = questions
        .iter()
        .map(|(num, question)| {
            let question_model = gemini.completion_model("gemini-3-flash-preview");
            run_question_task(
                *num,
                topic,
                question,
                output_dir.clone(),
                question_model,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )
        })
        .collect();

    // Run all question tasks in parallel
    let question_results = join_all(question_tasks).await;

    let succeeded: Vec<_> = question_results
        .iter()
        .filter_map(|r| r.metrics.as_ref())
        .collect();
    let failed = question_results.len() - succeeded.len();

    let was_cancelled = cancelled.load(Ordering::SeqCst);

    println!(
        "\nNew questions complete: {}/{} succeeded{}\n",
        succeeded.len(),
        question_results.len(),
        if was_cancelled { " (cancelled)" } else { "" }
    );

    if was_cancelled || succeeded.is_empty() {
        let total_time = start_time.elapsed().as_secs_f32();
        let total_input: u64 = succeeded.iter().map(|m| m.input_tokens).sum();
        let total_output: u64 = succeeded.iter().map(|m| m.output_tokens).sum();
        let total_tokens: u64 = succeeded.iter().map(|m| m.total_tokens).sum();

        return Ok(ResearchResult {
            topic: topic.to_string(),
            output_dir,
            succeeded: succeeded.len(),
            failed,
            cancelled: was_cancelled,
            total_time_secs: total_time,
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_tokens,
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
        if let Ok(content) = fs::read_to_string(output_dir.join(filename)).await {
            if !content.is_empty() {
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
    if skill_result.metrics.is_some() {
        if let Ok(skill_content) = fs::read_to_string(skill_dir.join("SKILL.md")).await {
            if skill_content.contains("--- FILE:") {
                let mut current_file: Option<String> = None;
                let mut current_content = String::new();

                for line in skill_content.lines() {
                    if line.starts_with("--- FILE:") && line.ends_with("---") {
                        if let Some(ref filename) = current_file {
                            let file_path = skill_dir.join(filename);
                            let normalized = normalize_markdown(&current_content);
                            let _ = fs::write(&file_path, normalized).await;
                        }
                        let filename = line
                            .trim_start_matches("--- FILE:")
                            .trim_end_matches("---")
                            .trim();
                        current_file = Some(filename.to_string());
                        current_content = String::new();
                    } else if current_file.is_some() {
                        current_content.push_str(line);
                        current_content.push('\n');
                    }
                }
                if let Some(ref filename) = current_file {
                    let file_path = skill_dir.join(filename);
                    let normalized = normalize_markdown(&current_content);
                    let _ = fs::write(&file_path, normalized).await;
                }
            }
        }
    }

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
/// ## Returns
/// A `ResearchResult` containing metrics about the operation
///
/// ## Errors
/// Returns `ResearchError` if the output directory cannot be created
/// or if all prompts fail.
pub async fn research(
    topic: &str,
    output_dir: Option<PathBuf>,
    questions: &[String],
) -> Result<ResearchResult, ResearchError> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Use provided output_dir or default to research/{topic}
    let output_dir = output_dir.unwrap_or_else(|| default_output_dir(topic));

    // Create output directory
    fs::create_dir_all(&output_dir).await?;

    // Check for existing metadata (incremental mode)
    if let Some(existing_metadata) = ResearchMetadata::load(&output_dir).await {
        println!("Found existing research for '{}'", topic);

        if questions.is_empty() {
            println!("  No new questions provided. Use additional prompts to expand research.");
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

        // Check for overlaps and filter questions
        let mut questions_to_run: Vec<(usize, String)> = Vec::new();
        let mut next_num = existing_metadata.next_question_number();

        for question in questions {
            if let Some(conflict_file) = existing_metadata.check_overlap(question) {
                println!("  ⚠ Question overlaps with existing {}: \"{}\"", conflict_file, question);

                // Ask user if they want to include anyway
                let confirm = inquire::Confirm::new(&format!(
                    "Include anyway as question_{}?",
                    next_num
                ))
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

        if questions_to_run.is_empty() {
            println!("  No new questions to run after overlap check.");
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

        // Run incremental research with just the new questions
        return run_incremental_research(
            topic,
            output_dir,
            existing_metadata,
            questions_to_run,
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
    let zai = zai::Client::from_env();

    // Initialize models
    let glm = zai.completion_model(zai::GLM_4_7);
    let gemini1 = gemini.completion_model("gemini-3-flash-preview");
    let gemini2 = gemini.completion_model("gemini-3-flash-preview");
    let gemini3 = gemini.completion_model("gemini-3-flash-preview");
    let changelog_model = openai.completion_model("gpt-5.2"); // Use stronger model for changelog

    // Build prompts from templates
    let overview_prompt = prompts::OVERVIEW.replace("{{topic}}", topic);
    let similar_libraries_prompt = prompts::SIMILAR_LIBRARIES.replace("{{topic}}", topic);
    let integration_partners_prompt = prompts::INTEGRATION_PARTNERS.replace("{{topic}}", topic);
    let use_cases_prompt = prompts::USE_CASES.replace("{{topic}}", topic);
    let changelog_prompt = prompts::CHANGELOG.replace("{{topic}}", topic);

    let num_questions = questions.len();
    let total = 5 + num_questions; // 5 default prompts + user questions

    println!(
        "Phase 1: Running {} research prompts in parallel to {:?}...\n",
        total, output_dir
    );
    println!("  (Press Ctrl+C to cancel and save completed results)\n");

    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));

    // Create question tasks dynamically
    let question_tasks: Vec<_> = questions
        .iter()
        .enumerate()
        .map(|(i, question)| {
            let question_model = gemini.completion_model("gemini-3-flash-preview");
            run_question_task(
                i + 1,
                topic,
                question,
                output_dir.clone(),
                question_model,
                counter.clone(),
                total,
                start_time,
                cancelled.clone(),
            )
        })
        .collect();

    // Run main 5 tasks and all question tasks in parallel
    let (r1, r2, r3, r4, r5, question_results) = tokio::join!(
        run_prompt_task(
            "overview",
            "overview.md",
            output_dir.clone(),
            glm,
            overview_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        ),
        run_prompt_task(
            "similar_libraries",
            "similar_libraries.md",
            output_dir.clone(),
            gemini1,
            similar_libraries_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        ),
        run_prompt_task(
            "integration_partners",
            "integration_partners.md",
            output_dir.clone(),
            gemini2,
            integration_partners_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        ),
        run_prompt_task(
            "use_cases",
            "use_cases.md",
            output_dir.clone(),
            gemini3,
            use_cases_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        ),
        run_prompt_task(
            "changelog",
            "changelog.md",
            output_dir.clone(),
            changelog_model,
            changelog_prompt,
            counter.clone(),
            total,
            start_time,
            cancelled.clone(),
        ),
        join_all(question_tasks),
    );

    let mut phase1_results = vec![r1, r2, r3, r4, r5];
    phase1_results.extend(question_results);
    let phase1_succeeded: Vec<_> = phase1_results
        .iter()
        .filter_map(|r| r.metrics.as_ref())
        .collect();
    let phase1_failed = phase1_results.len() - phase1_succeeded.len();

    // Check if cancelled
    let was_cancelled = cancelled.load(Ordering::SeqCst);

    println!(
        "\nPhase 1 complete: {}/{} succeeded{}\n",
        phase1_succeeded.len(),
        phase1_results.len(),
        if was_cancelled { " (cancelled)" } else { "" }
    );

    if phase1_succeeded.is_empty() {
        return Err(ResearchError::AllPromptsFailed);
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
        if let Ok(content) = fs::read_to_string(output_dir.join(&filename)).await {
            if !content.is_empty() {
                additional_content.push_str(&format!(
                    "\n## Additional Research: Question {}\n\n{}\n",
                    i, content
                ));
            }
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
    if skill_result.metrics.is_some() {
        if let Ok(skill_content) = fs::read_to_string(skill_dir.join("SKILL.md")).await {
            // Check if the output contains multiple files
            if skill_content.contains("--- FILE:") {
                let mut current_file: Option<String> = None;
                let mut current_content = String::new();

                for line in skill_content.lines() {
                    if line.starts_with("--- FILE:") && line.ends_with("---") {
                        // Save previous file if any
                        if let Some(ref filename) = current_file {
                            let file_path = skill_dir.join(filename);
                            let normalized = normalize_markdown(&current_content);
                            let _ = fs::write(&file_path, normalized).await;
                        }
                        // Start new file
                        let filename = line
                            .trim_start_matches("--- FILE:")
                            .trim_end_matches("---")
                            .trim();
                        current_file = Some(filename.to_string());
                        current_content = String::new();
                    } else if current_file.is_some() {
                        current_content.push_str(line);
                        current_content.push('\n');
                    }
                }
                // Save last file
                if let Some(ref filename) = current_file {
                    let file_path = skill_dir.join(filename);
                    let normalized = normalize_markdown(&current_content);
                    let _ = fs::write(&file_path, normalized).await;
                }
            }
        }
    }

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
    for (i, question) in questions.iter().enumerate() {
        let filename = format!("question_{}.md", i + 1);
        metadata.add_additional_file(filename, question.clone());
    }
    if let Err(e) = metadata.save(&output_dir).await {
        eprintln!("Warning: Failed to write metadata.json: {}", e);
    }

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
