//! Shared Library - Common utilities for the Dockhand monorepo
//!
//! This library provides shared functionality used across multiple areas
//! of the monorepo.
//!
//! ## Modules
//!
//! - [`api`] - OpenAI-compatible API utilities for model discovery
//! - [`codegen`] - Safe code injection with AST-based manipulation
//! - [`markdown`] - Markdown document manipulation with frontmatter support (Phase 3)
//! - [`model`] - Centralized model selection with fallback stacking (Phase 3 - in progress)
//! - [`providers`] - LLM provider discovery and model listing (Phase 1 - in progress)
//! - [`terminal`] - Terminal color detection utilities (Phase 1)
//! - [`testing`] - Testing utilities for terminal output verification (Phase 2)
//! - [`tts`] - Text-to-speech utilities for announcing task completion
//! - [`tools`] - Agent tools for rig-core (Brave Search, Screen Scrape)

pub mod api;
pub mod codegen;
pub mod markdown;
pub mod model;
pub mod providers;
pub mod render;
pub mod terminal;

pub mod symbols;

pub mod testing;
pub mod tools;
pub mod tts;
