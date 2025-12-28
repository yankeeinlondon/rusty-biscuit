//! Dockhand Library - Research automation for Rust crates
//!
//! This library provides tools for automated research on Rust crates,
//! running multiple LLM prompts in parallel and saving results.

pub mod providers;

use providers::zai;
use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::cmark;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{AssistantContent, CompletionModel};
use rig::providers::{gemini, openai};
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
    pub const CONTEXT: &str = include_str!("../prompts/context.md");
    pub const SKILL: &str = include_str!("../prompts/skill.md");
    pub const DEEP_DIVE: &str = include_str!("../prompts/deep_dive.md");
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

/// Returns the default output directory for a given topic: `research/{topic}`
pub fn default_output_dir(topic: &str) -> PathBuf {
    PathBuf::from("research").join(topic)
}



/// Research a topic by running multiple LLM prompts in parallel.
///
/// Generates the following files in the output directory:
/// - `overview.md` - Comprehensive analysis of the topic
/// - `similar_libraries.md` - Comparable libraries and alternatives
/// - `integration_partners.md` - Libraries commonly used with the topic
/// - `use_cases.md` - Common use cases and examples
///
/// ## Arguments
/// * `topic` - The Rust crate or topic to research
/// * `output_dir` - Directory where output files will be written. If `None`,
///   defaults to `research/{topic}` relative to the current directory.
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
) -> Result<ResearchResult, ResearchError> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Use provided output_dir or default to research/{topic}
    let output_dir = output_dir.unwrap_or_else(|| default_output_dir(topic));

    // Create output directory
    fs::create_dir_all(&output_dir).await?;

    // Set up cancellation flag for graceful SIGINT handling
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_for_handler = cancelled.clone();

    // Spawn SIGINT handler
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            eprintln!("\n⚠ Received SIGINT - completing in-progress tasks and saving results...");
            cancelled_for_handler.store(true, Ordering::SeqCst);
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

    // Build prompts from templates
    let overview_prompt = prompts::OVERVIEW.replace("{{topic}}", topic);
    let similar_libraries_prompt = prompts::SIMILAR_LIBRARIES.replace("{{topic}}", topic);
    let integration_partners_prompt = prompts::INTEGRATION_PARTNERS.replace("{{topic}}", topic);
    let use_cases_prompt = prompts::USE_CASES.replace("{{topic}}", topic);

    println!("Phase 1: Running 4 research prompts in parallel to {:?}...\n", output_dir);
    println!("  (Press Ctrl+C to cancel and save completed results)\n");

    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let total = 4;

    // Spawn all tasks in parallel
    let (r1, r2, r3, r4) = tokio::join!(
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
    );

    let phase1_results = [r1, r2, r3, r4];
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

    // Build context from phase 1 results
    let combined_context = prompts::CONTEXT
        .replace("{{topic}}", topic)
        .replace("{{overview}}", &overview_content)
        .replace("{{similar_libraries}}", &similar_libraries_content)
        .replace("{{integration_partners}}", &integration_partners_content)
        .replace("{{use_cases}}", &use_cases_content);

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
    let skill_gen = openai.completion_model("gpt-5.2-codex");
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
