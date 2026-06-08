//! Markdown parsing, rendering, and Mermaid diagram support.
//!
//! This library provides markdown document manipulation with frontmatter support,
//! syntax highlighting, terminal and HTML rendering, and Mermaid diagram theming.
//!
//! ## Public Metadata Catalogs
//!
//! Darkmatter exposes three static, typed descriptor catalogs that describe its
//! runtime surface. These are compile-time metadata — no host probing, I/O, or
//! context capture is performed when accessing them.
//!
//! - **Context variables** — [`markdown::compose::context::ContextVariableDescriptor`]
//!   and [`markdown::compose::context::context_variable_descriptors()`] describe
//!   every variable available to `ComposeContext` (date/time, repository, file
//!   changes, languages, documents, OS, hardware).
//!
//! - **Expression functions** — [`markdown::compose::expression::ExpressionFunctionDescriptor`]
//!   and [`markdown::compose::expression::expression_function_descriptors()`] describe
//!   every callable function in the expression engine (type predicates, math,
//!   collection operations, string predicates/mutations, date formatting/validators,
//!   logical operators, filesystem queries).
//!
//! - **Side-effect capabilities** — [`effects::EffectDescriptor`] and
//!   [`effects::effect_descriptors()`] describe every mutating verb exposed by
//!   `EffectEngine` (frontmatter mutations, file/directory operations, network
//!   requests), including all overloaded signatures and their safety constraints.
//!
//! The Markdown topic docs (`context-variables.md`, `darkmatter-expressions.md`,
//! `side-effects.md`) remain explanatory documentation only. They are not an API
//! and are not parsed at runtime.
//!
//! ## Modules
//!
//! - [`markdown`] - Markdown document manipulation with frontmatter support
//! - [`diff`] - Reusable diff utilities
//! - [`effects`] - Mutating side-effect engine (library surface only)
//! - [`layout`] - Page-level layout primitive (`DarkmatterPage`)
//! - [`mermaid`] - Mermaid diagram theming and rendering
//! - [`render`] - Hyperlink rendering utilities
//! - [`terminal`] - Terminal color detection utilities
//! - [`testing`] - Testing utilities for terminal output verification

pub mod diff;
pub mod editor;
pub mod effects;
pub mod layout;
pub mod markdown;
pub mod mermaid;
pub mod render;
pub mod style;
pub mod terminal;

pub mod testing;
