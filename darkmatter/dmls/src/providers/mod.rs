//! The provider registry (AD-5).
//!
//! Each LSP capability is answered by an ordered chain of providers — the
//! Markdown substrate first, the Darkmatter overlay after (overlay lands in
//! Phases 7/9). A single [`Provider`] trait carries one defaulted method per
//! capability; [`ProviderRegistry`] holds the ordered `Vec` and applies the
//! design's per-capability merge policy. Every provider call is wrapped in a
//! panic boundary so one misbehaving provider degrades to an empty
//! contribution and a logged warning instead of taking down the request loop.

pub mod code_actions;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod dsl;
pub mod edits;
pub mod folding;
pub mod formatting;
pub mod frontmatter;
pub mod hover;
pub mod location;
pub mod references;
pub mod rename;
pub mod semantic_tokens;
pub mod symbols;
pub mod wiki;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use lsp_types::{
    CompletionItem, Diagnostic, DocumentHighlight, DocumentLink, DocumentSymbol, FoldingRange,
    Hover, Location, Position, Uri, WorkspaceSymbol,
};

use crate::capabilities::ClientProfile;
use crate::config::DmlsConfig;
use crate::graph::{DocumentId, WorkspaceGraph};
use crate::overlay::DocumentOverlay;
use crate::source_map::SourceMap;

/// Everything a provider needs to answer a request about one open document.
///
/// Borrows the live open-buffer snapshot (`text` + `source_map`, both
/// version-stamped) and the current immutable workspace graph; `doc_id` is the
/// current document's node in that graph (`None` when a fresh buffer has not
/// yet been indexed into the loaded snapshot).
pub struct DocumentContext<'a> {
    /// The document URI.
    pub uri: &'a Uri,
    /// The document's resolved filesystem path (the graph's logical key).
    pub path: &'a Path,
    /// The exact buffer text.
    pub text: &'a str,
    /// Byte ↔ position map for this `(uri, version)`.
    pub source_map: &'a SourceMap,
    /// The current workspace graph snapshot.
    pub graph: &'a WorkspaceGraph,
    /// This document's id in `graph`, if indexed.
    pub doc_id: Option<DocumentId>,
    /// Effective configuration.
    pub config: &'a DmlsConfig,
    /// Per-client capability gates.
    pub profile: &'a ClientProfile,
    /// The frontmatter overlay (AST + effective schema) for this document, when
    /// it has a frontmatter block. `None` for documents without frontmatter.
    pub overlay: Option<&'a DocumentOverlay>,
}

/// One member of a capability chain.
///
/// Every method defaults to an empty contribution, so a provider implements
/// only the capabilities it participates in.
#[allow(unused_variables)]
pub trait Provider: Send + Sync {
    /// Stable name for logging.
    fn name(&self) -> &'static str;

    /// Heading-hierarchy document symbols.
    fn document_symbols(&self, ctx: &DocumentContext) -> Vec<DocumentSymbol> {
        Vec::new()
    }

    /// Workspace-wide symbols matching `query`.
    fn workspace_symbols(&self, query: &str, graph: &WorkspaceGraph) -> Vec<WorkspaceSymbol> {
        Vec::new()
    }

    /// Definition locations for the construct under `offset`.
    fn definition(&self, ctx: &DocumentContext, offset: usize) -> Vec<Location> {
        Vec::new()
    }

    /// Document links (resolved eagerly; targets are cheap path joins).
    fn document_links(&self, ctx: &DocumentContext) -> Vec<DocumentLink> {
        Vec::new()
    }

    /// Reference locations for the construct under `offset`.
    fn references(&self, ctx: &DocumentContext, offset: usize, include_declaration: bool) -> Vec<Location> {
        Vec::new()
    }

    /// Same-document highlights for the construct under `offset`.
    fn document_highlights(&self, ctx: &DocumentContext, offset: usize) -> Vec<DocumentHighlight> {
        Vec::new()
    }

    /// Folding ranges for the whole document.
    fn folding_ranges(&self, ctx: &DocumentContext) -> Vec<FoldingRange> {
        Vec::new()
    }

    /// Hover for the construct under `offset`.
    fn hover(&self, ctx: &DocumentContext, offset: usize) -> Option<Hover> {
        None
    }

    /// Completion items at `offset`.
    fn completion(&self, ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
        Vec::new()
    }

    /// Push diagnostics for the whole document.
    fn diagnostics(&self, ctx: &DocumentContext) -> Vec<Diagnostic> {
        Vec::new()
    }
}

/// The Markdown substrate provider — the only Layer-0 provider.
///
/// Each capability delegates to a focused module (`symbols`, `definition`, …)
/// so the design's "one module per capability" shape holds even though a
/// single trait impl registers them all.
pub struct SubstrateProvider;

impl Provider for SubstrateProvider {
    fn name(&self) -> &'static str {
        "substrate"
    }

    fn document_symbols(&self, ctx: &DocumentContext) -> Vec<DocumentSymbol> {
        symbols::document_symbols(ctx)
    }

    fn workspace_symbols(&self, query: &str, graph: &WorkspaceGraph) -> Vec<WorkspaceSymbol> {
        symbols::workspace_symbols(query, graph)
    }

    fn definition(&self, ctx: &DocumentContext, offset: usize) -> Vec<Location> {
        definition::definition(ctx, offset)
    }

    fn document_links(&self, ctx: &DocumentContext) -> Vec<DocumentLink> {
        definition::document_links(ctx)
    }

    fn references(&self, ctx: &DocumentContext, offset: usize, include_declaration: bool) -> Vec<Location> {
        references::references(ctx, offset, include_declaration)
    }

    fn document_highlights(&self, ctx: &DocumentContext, offset: usize) -> Vec<DocumentHighlight> {
        references::document_highlights(ctx, offset)
    }

    fn folding_ranges(&self, ctx: &DocumentContext) -> Vec<FoldingRange> {
        folding::folding_ranges(ctx)
    }

    fn hover(&self, ctx: &DocumentContext, offset: usize) -> Option<Hover> {
        hover::hover(ctx, offset)
    }

    fn completion(&self, ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
        completion::completion(ctx, offset)
    }

    fn diagnostics(&self, ctx: &DocumentContext) -> Vec<Diagnostic> {
        diagnostics::diagnostics(ctx)
    }
}

/// The Layer-1 wiki-link provider (R-8).
///
/// Registered after the substrate so its contributions merge on top of plain
/// Markdown. Every method self-gates on `wiki.enable` and reads the resolution
/// pre-computed on each wiki-link node — no filesystem access at request time.
pub struct WikiProvider;

impl Provider for WikiProvider {
    fn name(&self) -> &'static str {
        "wiki"
    }

    fn definition(&self, ctx: &DocumentContext, offset: usize) -> Vec<Location> {
        wiki::definition(ctx, offset)
    }

    fn document_links(&self, ctx: &DocumentContext) -> Vec<DocumentLink> {
        wiki::document_links(ctx)
    }

    fn references(&self, ctx: &DocumentContext, offset: usize, include_declaration: bool) -> Vec<Location> {
        wiki::references(ctx, offset, include_declaration)
    }

    fn document_highlights(&self, ctx: &DocumentContext, offset: usize) -> Vec<DocumentHighlight> {
        wiki::document_highlights(ctx, offset)
    }

    fn hover(&self, ctx: &DocumentContext, offset: usize) -> Option<Hover> {
        wiki::hover(ctx, offset)
    }

    fn completion(&self, ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
        wiki::completion(ctx, offset)
    }

    fn diagnostics(&self, ctx: &DocumentContext) -> Vec<Diagnostic> {
        wiki::diagnostics(ctx)
    }
}

/// The Layer-2 frontmatter provider (schema intelligence).
///
/// Registered after the substrate and wiki providers so its frontmatter
/// contributions merge on top. Every method self-gates on the presence of the
/// [`DocumentOverlay`](crate::overlay::DocumentOverlay) in the context — a
/// document with no frontmatter block never reaches the overlay logic.
pub struct FrontmatterProvider;

impl Provider for FrontmatterProvider {
    fn name(&self) -> &'static str {
        "frontmatter"
    }

    fn document_symbols(&self, ctx: &DocumentContext) -> Vec<DocumentSymbol> {
        frontmatter::document_symbols(ctx)
    }

    fn definition(&self, ctx: &DocumentContext, offset: usize) -> Vec<Location> {
        frontmatter::definition(ctx, offset)
    }

    fn document_links(&self, ctx: &DocumentContext) -> Vec<DocumentLink> {
        frontmatter::document_links(ctx)
    }

    fn folding_ranges(&self, ctx: &DocumentContext) -> Vec<FoldingRange> {
        frontmatter::folding_ranges(ctx)
    }

    fn hover(&self, ctx: &DocumentContext, offset: usize) -> Option<Hover> {
        frontmatter::hover(ctx, offset)
    }

    fn completion(&self, ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
        frontmatter::completion(ctx, offset)
    }

    fn diagnostics(&self, ctx: &DocumentContext) -> Vec<Diagnostic> {
        frontmatter::diagnostics(ctx)
    }
}

/// The Layer-3 Darkmatter DSL provider: directives, transclusion,
/// interpolation, shell awareness, and fenced-language diagnostics.
///
/// Registered last so its contributions merge on top of every other provider;
/// each capability re-scans the buffer with the passive Phase-8 library scanners
/// and never executes a directive, expression, or command.
pub struct DslProvider;

impl Provider for DslProvider {
    fn name(&self) -> &'static str {
        "dsl"
    }

    fn definition(&self, ctx: &DocumentContext, offset: usize) -> Vec<Location> {
        dsl::definition(ctx, offset)
    }

    fn document_links(&self, ctx: &DocumentContext) -> Vec<DocumentLink> {
        dsl::document_links(ctx)
    }

    fn references(&self, ctx: &DocumentContext, offset: usize, include_declaration: bool) -> Vec<Location> {
        dsl::references(ctx, offset, include_declaration)
    }

    fn folding_ranges(&self, ctx: &DocumentContext) -> Vec<FoldingRange> {
        dsl::folding_ranges(ctx)
    }

    fn hover(&self, ctx: &DocumentContext, offset: usize) -> Option<Hover> {
        dsl::hover(ctx, offset)
    }

    fn completion(&self, ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
        dsl::completion(ctx, offset)
    }

    fn diagnostics(&self, ctx: &DocumentContext) -> Vec<Diagnostic> {
        dsl::diagnostics(ctx)
    }
}

/// Result cap for workspace symbol queries (design "workspace symbols: …
/// result cap").
const WORKSPACE_SYMBOL_CAP: usize = 256;

/// The ordered provider chain and its per-capability merge policies.
pub struct ProviderRegistry {
    providers: Vec<Box<dyn Provider>>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::with_substrate()
    }
}

impl ProviderRegistry {
    /// The default registry: the Markdown substrate followed by the Layer-1
    /// wiki provider (self-gated on `wiki.enable`). Later overlay providers
    /// (Phases 7/9) append after these, per AD-5.
    pub fn with_substrate() -> Self {
        Self {
            providers: vec![
                Box::new(SubstrateProvider),
                Box::new(WikiProvider),
                Box::new(FrontmatterProvider),
                Box::new(DslProvider),
            ],
        }
    }

    /// Runs `call` on each provider inside a panic boundary, discarding (and
    /// logging) a provider that panics.
    fn collect<T>(&self, capability: &str, mut call: impl FnMut(&dyn Provider) -> Vec<T>) -> Vec<T> {
        let mut merged = Vec::new();
        for provider in &self.providers {
            match catch_unwind(AssertUnwindSafe(|| call(provider.as_ref()))) {
                Ok(items) => merged.extend(items),
                Err(_) => {
                    tracing::error!(provider = provider.name(), capability, "provider panicked");
                }
            }
        }
        merged
    }

    /// Document symbols: union, containment order preserved by the provider.
    pub fn document_symbols(&self, ctx: &DocumentContext) -> Vec<DocumentSymbol> {
        self.collect("documentSymbol", |p| p.document_symbols(ctx))
    }

    /// Workspace symbols: union across providers, capped.
    pub fn workspace_symbols(&self, query: &str, graph: &WorkspaceGraph) -> Vec<WorkspaceSymbol> {
        let mut merged = self.collect("workspaceSymbol", |p| p.workspace_symbols(query, graph));
        merged.truncate(WORKSPACE_SYMBOL_CAP);
        merged
    }

    /// Definition: union of locations, deduplicated.
    pub fn definition(&self, ctx: &DocumentContext, offset: usize) -> Vec<Location> {
        dedup_locations(self.collect("definition", |p| p.definition(ctx, offset)))
    }

    /// Document links: union, deduplicated by range.
    pub fn document_links(&self, ctx: &DocumentContext) -> Vec<DocumentLink> {
        let mut merged = self.collect("documentLink", |p| p.document_links(ctx));
        merged.sort_by_key(|link| (link.range.start.line, link.range.start.character));
        merged.dedup_by_key(|link| link.range);
        merged
    }

    /// References: union, deduplicated by location.
    pub fn references(&self, ctx: &DocumentContext, offset: usize, include_declaration: bool) -> Vec<Location> {
        dedup_locations(self.collect("references", |p| p.references(ctx, offset, include_declaration)))
    }

    /// Document highlights: union within the current document, deduplicated.
    pub fn document_highlights(&self, ctx: &DocumentContext, offset: usize) -> Vec<DocumentHighlight> {
        let mut merged = self.collect("documentHighlight", |p| p.document_highlights(ctx, offset));
        merged.sort_by_key(|h| (h.range.start.line, h.range.start.character));
        merged.dedup_by_key(|h| h.range);
        merged
    }

    /// Folding: union, containment-sorted by start line.
    pub fn folding_ranges(&self, ctx: &DocumentContext) -> Vec<FoldingRange> {
        let mut merged = self.collect("foldingRange", |p| p.folding_ranges(ctx));
        merged.sort_by_key(|f| (f.start_line, f.end_line));
        merged
    }

    /// Hover: first non-empty wins.
    pub fn hover(&self, ctx: &DocumentContext, offset: usize) -> Option<Hover> {
        for provider in &self.providers {
            match catch_unwind(AssertUnwindSafe(|| provider.hover(ctx, offset))) {
                Ok(Some(hover)) => return Some(hover),
                Ok(None) => {}
                Err(_) => tracing::error!(provider = provider.name(), "hover provider panicked"),
            }
        }
        None
    }

    /// Completion: union, deduplicated by label + kind.
    pub fn completion(&self, ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
        let mut merged = self.collect("completion", |p| p.completion(ctx, offset));
        merged.dedup_by(|a, b| a.label == b.label && a.kind == b.kind);
        merged
    }

    /// Diagnostics: union (namespaced sources prevent collisions).
    pub fn diagnostics(&self, ctx: &DocumentContext) -> Vec<Diagnostic> {
        self.collect("diagnostics", |p| p.diagnostics(ctx))
    }
}

/// Deduplicates locations by `(uri, range)`, preserving first-seen order.
fn dedup_locations(locations: Vec<Location>) -> Vec<Location> {
    let mut seen: Vec<(String, Position, Position)> = Vec::new();
    let mut out = Vec::new();
    for location in locations {
        let key = (
            location.uri.as_str().to_string(),
            location.range.start,
            location.range.end,
        );
        if !seen.contains(&key) {
            seen.push(key);
            out.push(location);
        }
    }
    out
}
