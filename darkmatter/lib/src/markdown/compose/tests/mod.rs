//! Integration tests for the compose pipeline public API and stages.

#[allow(unused_imports)]
use super::HeadingLevel;
use super::*;
use super::super::types::MarkdownError;
use super::transclusion::TransclusionEngine;
use biscuit_terminal::utils::UnicodeWidthStr;

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
