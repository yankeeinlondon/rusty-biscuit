mod providers;

use providers::zai;
use pulldown_cmark::{Options, Parser};
use pulldown_cmark_to_cmark::cmark;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::CompletionModel;
use rig::providers::{gemini, openai};
use tts::Tts;
use std::env;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;

/// Metrics from a completed prompt
#[derive(Debug, Default, Clone)]
pub struct PromptMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub elapsed_secs: f32,
}

/// Normalize markdown by parsing and re-serializing it
/// This produces consistent formatting regardless of LLM output style
fn normalize_markdown(input: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(input, options);
    let mut output = String::new();

    // cmark returns Result<State, Error> - we just need to handle any error
    if cmark(parser, &mut output).is_err() {
        // If normalization fails, return original content
        return input.to_string();
    }

    output
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
) -> Option<PromptMetrics>
where
    M: CompletionModel,
{
    println!("  [{}] Starting...", name);

    // Use completion_request to get full response with usage metrics
    let result = model.completion_request(&prompt).send().await;

    let elapsed = start_time.elapsed().as_secs_f32();
    let completed = counter.fetch_add(1, Ordering::SeqCst) + 1;

    match result {
        Ok(response) => {
            // Extract content from the response
            use rig::completion::AssistantContent;
            let content: String = response
                .choice
                .into_iter()
                .filter_map(|c| match c {
                    AssistantContent::Text(text) => Some(text.text),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            // Extract usage metrics
            let usage = &response.usage;
            let metrics = PromptMetrics {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                total_tokens: usage.total_tokens,
                elapsed_secs: elapsed,
            };

            // Normalize markdown formatting before writing
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
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let topic: String = "rig-core".to_string();

    // providers
    let openai = openai::Client::from_env();
    let gemini = gemini::Client::from_env();
    let zai = zai::Client::from_env();

    // models - use completion_model for low-level API with metrics
    let glm = zai.completion_model(zai::GLM_4_7);
    let gemini1 = gemini.completion_model("gemini-3-flash-preview");
    let gemini2 = gemini.completion_model("gemini-3-flash-preview");
    let gemini3 = gemini.completion_model("gemini-3-flash-preview");

    // prompts
    let deep_dive_prompt = format!(
        "Do a deep dive on the Rust crate {topic}. Provide a structured view of its functional footprint, using code examples where possible. Add a section for 'gotchas' that people run into when they use this crate along with solutions to avoid these issues. Mention any/all licenses this software is available under. Finally discuss where this crate is a good fit and where it is not."
    );

    let similar_libraries_prompt = format!(
        "Find a list of comparable libraries to the '{topic}' crate. For each library create a summary description, a few pros and cons bullets, and URLs to the repo, docs.rs reference, and documentation site (if exists)."
    );

    let integration_partners_prompt = format!(
        "Find 2-3 libraries which are commonly used with/integrated with the '{topic}' crate. For each library, describe how and why these two libraries are used together and give a code example of how they may be used together."
    );

    let use_cases_prompt = format!(
        "List out at least 4-5 common use cases which might benefit from the '{topic}' crate. For each use case: describe the use case, the benefit which '{topic}' would provide and a code example of what that might look like."
    );

    let skill_prompt = format!(
        "Use the following documents about '{topic}' as context and once you've formed a clear view about this library. Build a \"Claude Code Skill\" where this skills starts with a document named `{topic}/SKILL.md` and then link out to a tree of other markdown documents located in the same output directory as SKILL.md.  The intention is that "
    );

    let deep_dive_consolidated = format!(
        "Consolidate all the documents into a well structured cohesive \"deep dive\" document on the '{topic}' crate. The goal is to not loss any information but restructure the content in a way that the overall "
    );

    // Create output directory
    let output_dir = env::current_dir()
        .expect("Failed to get current directory")
        .join("output");
    fs::create_dir_all(&output_dir)
        .await
        .expect("Failed to create output directory");

    println!("Running 4 prompts in parallel to {:?}...\n", output_dir);

    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let total = 4;

    // Spawn all tasks in parallel
    let (r1, r2, r3, r4) = tokio::join!(
        run_prompt_task(
            "deep_dive",
            "deep_dive.md",
            output_dir.clone(),
            glm,
            deep_dive_prompt,
            counter.clone(),
            total,
            start_time,
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
        ),
    );

    let results = [r1, r2, r3, r4];
    let succeeded: Vec<_> = results.iter().filter_map(|r| r.as_ref()).collect();
    let failed = results.len() - succeeded.len();
    let total_time = start_time.elapsed().as_secs_f32();

    // Aggregate metrics
    let total_input: u64 = succeeded.iter().map(|m| m.input_tokens).sum();
    let total_output: u64 = succeeded.iter().map(|m| m.output_tokens).sum();
    let total_tokens: u64 = succeeded.iter().map(|m| m.total_tokens).sum();

    println!("\n{}", "=".repeat(60));
    println!(
        "Complete: {} succeeded, {} failed in {:.1}s",
        succeeded.len(),
        failed,
        total_time
    );
    println!(
        "Total tokens: {} in, {} out, {} total",
        total_input, total_output, total_tokens
    );
    println!("{}", "=".repeat(60));

    // Announce completion via TTS
    if let Ok(mut tts) = Tts::default() {
        // Try to select a higher-quality voice (prefer non-compact voices)
        if let Ok(voices) = tts.voices() {
            // Prefer voices without "compact" in the ID (higher quality)
            // Also prefer English voices
            if let Some(voice) = voices.iter().find(|v| {
                !v.id().contains("compact")
                    && !v.id().contains("eloquence")
                    && v.language().starts_with("en")
            }) {
                let _ = tts.set_voice(voice);
            }
        }

        let message = format!("Research for the {} crate has completed", topic);
        if tts.speak(&message, false).is_ok() {
            // Small delay for speech to start
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Wait for speech to complete before exiting
            while tts.is_speaking().unwrap_or(false) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}
