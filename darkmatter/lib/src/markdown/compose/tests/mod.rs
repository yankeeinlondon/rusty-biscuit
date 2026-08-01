//! Integration tests for the compose pipeline public API and stages.

#[allow(unused_imports)]
use super::HeadingLevel;
use super::*;
use super::super::types::MarkdownError;
use super::transclusion::TransclusionEngine;
use biscuit_terminal::utils::UnicodeWidthStr;

/// A runtime context without the repo-wide capture `ComposeContext::capture()`
/// performs — git, repo, file changes, languages, docs, OS, hardware and GPU
/// detection via sniff, rooted at the real working tree, measured at 1.4s per
/// call on this repository.
///
/// Environment variables are populated either way. A `ctx.*` group some
/// expression actually asks for is still captured on demand during evaluation,
/// so this only drops work nothing reads. Tests that assert the *capture* itself
/// must keep calling `ComposeContext::capture()` / `ComposeOptions::new()`.
fn context_free_context() -> ComposeContext {
    ComposeContext::capture_for_content(std::path::Path::new("."), "")
}

/// Compose options carrying [`context_free_context`] instead of a fresh capture.
fn context_free_options() -> ComposeOptions {
    ComposeOptions::new_with_context(context_free_context())
}

mod caching;
mod fixtures;
mod frontmatter;
mod preflight;
mod provider_network;
mod rendering;
mod schema;
mod shell;
#[path = "transclusion.rs"]
mod transclusion_tests;
