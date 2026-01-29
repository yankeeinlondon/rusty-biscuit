//! Shared Library - Common utilities for the Dockhand monorepo
//!
//! This library provides shared functionality used across multiple areas
//! of the monorepo.
//!
//! ## Modules
//!
//! - [`api`] - OpenAI-compatible API utilities for model discovery
//! - [`codegen`] - Safe code injection with AST-based manipulation
//! - [`hashing`] - Fast (xxHash) and secure (BLAKE3) content hashing
//! - [`interpolate`] - Content interpolation (string, regex, markdown, HTML)
//! - [`isolate`] - Content isolation from structured documents
//! - [`model`] - Centralized model selection with fallback stacking
//! - [`providers`] - LLM provider discovery and model listing
//! - [`tools`] - Agent tools for rig-core (Brave Search, Screen Scrape)
//!
//! **Note:** TTS functionality has moved to the `biscuit-speaks` crate.
//! **Note:** Markdown and Mermaid functionality has moved to the `darkmatter-lib` crate.

pub mod api;
pub mod codegen;
pub mod hashing;
pub mod interpolate;
pub mod isolate;
pub mod model;
pub mod providers;

pub mod symbols;

pub mod tools;
