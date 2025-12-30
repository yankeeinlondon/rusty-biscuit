//! Shared Library - Common utilities for the Dockhand monorepo
//!
//! This library provides shared functionality used across multiple areas
//! of the monorepo.
//!
//! ## Modules
//!
//! - [`api`] - OpenAI-compatible API utilities for model discovery
//! - [`codegen`] - Safe code injection with AST-based manipulation
//! - [`model`] - Centralized model selection with fallback stacking (Phase 3 - in progress)
//! - [`providers`] - LLM provider discovery and model listing (Phase 1 - in progress)
//! - [`tts`] - Text-to-speech utilities for announcing task completion
//! - [`tools`] - Agent tools for rig-core (Brave Search, Screen Scrape)

pub mod api;
pub mod codegen;
pub mod model;
pub mod providers;
pub mod tools;
pub mod tts;
