//! Markdown parsing, rendering, and Mermaid diagram support.
//!
//! This library provides markdown document manipulation with frontmatter support,
//! syntax highlighting, terminal and HTML rendering, and Mermaid diagram theming.
//!
//! ## Rendering
//!
//! The simplified-rendering model exposes **two primary components** for turning
//! a Darkmatter document into terminal or browser output:
//!
//! - [`markdown::code_block::CodeBlock`] — the atomic renderer for one
//!   syntax-highlighted code block. It implements `TerminalRenderable` and
//!   `BrowserRenderable` directly, so a snippet, a Markdown fence, and a
//!   schema example all flow through the same code-block implementation.
//! - [`layout::DarkmatterPage`] — the page assembler that renders a full
//!   Markdown document. It owns page-frame layout (margins, padding,
//!   max-width, page background) and delegates nested fenced code blocks to
//!   `CodeBlock` so a fence inside a `DarkmatterPage` renders byte-for-byte
//!   equal to a direct `CodeBlock` call for the same code, language, and
//!   metadata.
//!
//! Both components share a single theme-resolution boundary
//! ([`markdown::highlighting::themes::ThemePair::resolve_for_surface`]) so
//! the page surface and the code panel never drift: the same
//! `Terminal::color_mode` feeds both, eliminating the dual-color-mode defect
//! the `with_color_mode` option used to introduce.
//!
//! The legacy `YamlBlock` is a thin delegating wrapper around
//! `CodeBlock::yaml(...)` and is deprecated; new code should use
//! `CodeBlock` directly.
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
//! - [`markdown`] - Markdown document manipulation, [`CodeBlock`], and the
//!   parsing/composition/frontmatter/hashing surface
//! - [`diff`] - Reusable diff utilities
//! - [`effects`] - Mutating side-effect engine (library surface only)
//! - [`layout`] - Page-level layout primitive ([`DarkmatterPage`])
//! - [`mermaid`] - Mermaid diagram theming and rendering
//! - [`prelude`] - Curated re-exports of every renderable component defined
//!   in Darkmatter (alongside the render traits)
//! - [`render`] - Hyperlink rendering utilities
//! - [`terminal`] - Terminal color detection utilities
//! - [`testing`] - Testing utilities for terminal output verification

pub mod catalog;
pub mod diff;
pub mod editor;
pub mod effects;
pub mod layout;
pub mod markdown;
pub mod mermaid;
pub mod prelude;
pub mod render;
pub mod style;
pub mod terminal;

pub mod testing;

pub use markdown::schemas::{
    darkmatter_base_json_schema, darkmatter_base_json_schema_ref, darkmatter_base_schema,
};
pub use markdown::span::{SourceSpan, Spanned, line_col_of_offset, line_of_offset};
