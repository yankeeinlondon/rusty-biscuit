//! DMLS — the Darkmatter Language Server.
//!
//! Speaks standard LSP 3.17 over stdio (`lsp-server` + `lsp-types`) with full
//! document sync. The Darkmatter library remains the semantic authority for
//! Markdown, frontmatter, schemas, and compose semantics; this crate owns the
//! protocol loop, source maps, client profiles, and workspace state.
//!
//! Architecture: `darkmatter/features/2026-07-04-dmls/design.md`. The router
//! shape is adapted from the Apache-2.0 `iwes` language server (IWE project,
//! Dmytro Halichenko) per the AD-1 decision record; see module-level notes in
//! [`router`].

pub mod bench;
pub mod capabilities;
pub mod config;
pub mod corpus;
pub mod diagnostics;
pub mod graph;
pub mod overlay;
pub mod providers;
pub mod router;
pub mod semantic_legend;
pub mod source_map;
pub mod wiki;
pub mod workspace;

pub use bench::{BenchReport, bench_index};
pub use capabilities::{ClientProfile, negotiate_position_encoding, server_capabilities};
pub use config::{ConfigState, DmlsConfig};
pub use corpus::{CorpusTier, generate_corpus};
pub use diagnostics::{DiagnosticsScheduler, DiagnosticTier};
pub use overlay::{DocumentOverlay, OverlayState};
pub use providers::{DocumentContext, Provider, ProviderRegistry};
pub use router::{RunOptions, ServerError, run_server};
pub use source_map::{PositionEncoding, Region, SourceMap};
