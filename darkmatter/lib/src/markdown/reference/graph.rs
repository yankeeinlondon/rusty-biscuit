//! Reference graph traversal and flattening.
//!
//! Builds a graph of documents connected by transclusion directives,
//! extracts references at each node, and flattens into composed order.

use std::collections::BTreeMap;

use super::provenance::{
    ReferenceDependencyManifest, ReferenceDocumentIdentity, ReferenceGraphMode,
};
use super::snapshot::{PreparedHeading, PreparedHeadingSnapshot, heading_at, prepared_headings_of};
use super::types::{
    AuthoredReferenceLocation, NodeId, ReferenceGraph, ReferenceGraphNode, ReferenceGraphOptions, ReferenceInsertion,
    ReferenceInsertionContext, ReferenceKind, ReferenceOrigin, ReferenceRecord,
    ReferenceReportDocument, ReferenceSet, ReferenceSyntax, ReferenceTarget, ReferenceTrailStep, classify_target,
    make_reference_id,
};
use crate::markdown::{Markdown, MarkdownError};
use crate::markdown::compose::body_origin::BodyOrigin;
use crate::markdown::compose::cache::RunLocalCache;
use crate::markdown::compose::conditions;
use crate::markdown::compose::context::options::ProseInterpolation;
use crate::markdown::compose::site_scope::{SiteHook, SiteLookup};
use crate::markdown::compose::toc_linking;
use biscuit_terminal::errors::SourceContext;

use crate::markdown::compose::transclusion::{
    DirectiveKind, InclusionSite, TransclusionRuntime, frontmatter_value_is_inline, parse_frontmatter_refs,
};
use crate::markdown::compose::{
    ComposeOperation, ComposeSource, ComposeWarning, EffectiveStateBuilder, ResolvingLookup,
};
use crate::markdown::normalize::HeadingLevel;
use crate::markdown::types::MarkdownResult;

/// Runtime state for reference graph analysis.
///
/// Shares a [`RunLocalCache`] across the entire graph traversal (rec #16),
/// so repeated loads of the same child document hit the cache instead of disk.
struct ReferenceAnalysisRuntime {
    transclusion: TransclusionRuntime,
    cache: RunLocalCache,
    /// The skipped-trigger warnings each node's preparation reported, once
    /// per trigger, in discovery order: a child prepares in its own context,
    /// whose schema roots may hold triggers the root's never see.
    trigger_warnings: Vec<ComposeWarning>,
    /// The passive warnings of each finished preparation and everything the
    /// failed one reported, in order, for a build that fails before a report
    /// can carry them.
    earlier_warnings: Vec<ComposeWarning>,
    /// The distinct prepared copies of each source built so far, by source
    /// id, in build order: what [`prepared_copy_id`](Self::prepared_copy_id)
    /// compares a new copy with.
    prepared_copies: std::collections::HashMap<NodeId, Vec<PreparedCopy>>,
    /// The copies that tolerate an invalid inclusion they write.
    tolerant: std::collections::HashSet<NodeId>,
}

/// What a node's consumers read of one prepared copy of a document.
struct PreparedCopy {
    references: ReferenceSet,
    insertions: Vec<ReferenceInsertion>,
    tolerant: bool,
}

fn source_to_path(source: &ComposeSource) -> std::path::PathBuf {
    match source {
        ComposeSource::File(path) => path.clone(),
        ComposeSource::Url(url) => std::path::PathBuf::from(url.to_string()),
        ComposeSource::Unknown => std::path::PathBuf::from("<unknown>"),
    }
}

impl ReferenceAnalysisRuntime {
    /// Loads the document an inclusion resolved to, through the shared cache.
    ///
    /// `None` when the file vanished after it resolved (validation then
    /// reports the target missing) or when a reference check running ahead of
    /// composition defers the failure to composition, which tolerates it with
    /// a notice. Any other failure (the target cannot be read, or its
    /// frontmatter does not parse) fails the graph, so no reader reports a
    /// document whose inclusion it could not inspect as clean.
    ///
    /// `site` is the inclusion that named `path`: a file that cannot be read
    /// is reported there, as composition reports it.
    fn load_inclusion(
        &self,
        path: &std::path::Path,
        options: &ReferenceGraphOptions,
        site: Option<InclusionSite>,
    ) -> MarkdownResult<Option<Markdown>> {
        // A dry run's file the run's initializer appends to holds other
        // references by then; composition leaves what reads it unchecked.
        if options.compose.passive_session().is_some_and(|session| session.initializer_changes(path).is_some()) {
            return Ok(None);
        }
        match self.cache.load_markdown(path) {
            Ok(markdown) => Ok(Some(markdown)),
            Err(MarkdownError::FileLoad(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) if options.compose.defer_inclusion_parse_failures => Ok(None),
            Err(error) => Err(read_failure(error, path, site)),
        }
    }

    /// Enters a local inclusion on the transclusion stack, `false` when the
    /// graph must not descend into it.
    ///
    /// A cycle is not a failure here: every document on it is already a node,
    /// so the graph inspects all of them. A depth refusal leaves a subtree
    /// unread, so it fails the graph as composition fails, unless the request
    /// leaves inclusion failures to composition.
    fn enter_inclusion(
        &mut self,
        id: String,
        path: std::path::PathBuf,
        line: usize,
        options: &ReferenceGraphOptions,
    ) -> MarkdownResult<bool> {
        match self.transclusion.enter(id, path, line) {
            Ok(()) => Ok(true),
            Err(crate::markdown::compose::transclusion::TransclusionError::CycleDetected { .. }) => Ok(false),
            Err(_) if options.compose.defer_inclusion_parse_failures => Ok(false),
            Err(error) => Err(MarkdownError::from(error)),
        }
    }

    /// The node id of a prepared copy of the source `source_id` names.
    ///
    /// Each inclusion prepares its child under its own values, and a child
    /// insertion selects its copy by this id, so copies whose references or
    /// insertions differ must not share one; an equal copy reuses the id of
    /// the copy it equals. Source identity (`source_id`) stays the key for
    /// raw loads, dependency freshness, cycle detection, and heading slugs.
    fn prepared_copy_id(
        &mut self,
        source_id: NodeId,
        references: &ReferenceSet,
        insertions: &[ReferenceInsertion],
        tolerant: bool,
    ) -> NodeId {
        let copies = self.prepared_copies.entry(source_id.clone()).or_default();
        let ordinal = match copies.iter().position(|copy| {
            copy.references == *references && copy.insertions == insertions && copy.tolerant == tolerant
        }) {
            Some(index) => index,
            None => {
                copies.push(PreparedCopy { references: references.clone(), insertions: insertions.to_vec(), tolerant });
                copies.len() - 1
            }
        };
        let id = match ordinal {
            0 => source_id,
            later => NodeId(format!("{source_id} ({})", later + 1)),
        };
        if tolerant {
            self.tolerant.insert(id.clone());
        }
        id
    }
}

/// `error`, raised reading `path`, the target the inclusion `site` selected:
/// placed at `site` as composition places it, or, with no site, naming
/// `path`.
fn read_failure(error: MarkdownError, path: &std::path::Path, site: Option<InclusionSite>) -> MarkdownError {
    match site {
        Some(_) => crate::markdown::compose::transclusion::target_failure_at(error, path, site),
        None => named_read_failure(error, path),
    }
}

/// `error` with `path` named when it is a bare I/O error, which names no file.
fn named_read_failure(error: MarkdownError, path: &std::path::Path) -> MarkdownError {
    match error {
        MarkdownError::FileLoad(error) => MarkdownError::FileLoad(std::io::Error::new(
            error.kind(),
            format!("cannot read {}: {error}", path.display()),
        )),
        other => other,
    }
}

/// Construct a run-local [`RunLocalCache`] from graph options.
///
/// Memory-only: `cache_root` reaches only the remote transport cache, so
/// graph construction, like the compose pipeline, persists no semantic-result
/// artifact (R18).
///
/// A dry run's graph reads each local source through the run's one
/// observation of local files, which its composition shares.
fn make_cache(options: &ReferenceGraphOptions) -> RunLocalCache {
    RunLocalCache::new(options.compose.cache_access_mode).with_shared_reads(shared_reads(options))
}

/// The dry run's observation of local files, when the graph is part of one.
fn shared_reads(options: &ReferenceGraphOptions) -> Option<simplified_schema::resolve::buffers::SharedReads> {
    options.compose.options().passive_session().map(|session| session.observations().local_reads().clone())
}

/// Shared graph construction. The build [`ReferenceGraphMode`] is the sole
/// input controlling reference extraction.
fn build_graph_inner(
    md: &Markdown,
    options: &ReferenceGraphOptions,
    mode: ReferenceGraphMode,
) -> MarkdownResult<ReferenceGraph> {
    // Graph mode is the single source of truth for reference extraction.
    let extract_references = matches!(mode, ReferenceGraphMode::Full);

    // The root as the request admits it: the document's own path (a
    // relative one is the file under the request directory), or the
    // request's file when the document names none.
    let admitted = options.compose.admitted_document(md);
    let md = admitted.as_ref();
    let compose = options.compose.clone().with_root_source(&md.source().clone().unwrap_or(ComposeSource::Unknown));
    let source = match md.source() {
        Some(ComposeSource::File(_)) => compose.source.clone(),
        other => other.clone().unwrap_or(ComposeSource::Unknown),
    };
    let options = ReferenceGraphOptions::from_compose_options(compose);
    let options = &options;
    // Every schema file and document the build reads on this thread.
    let _shared_reads = shared_reads(options).map(simplified_schema::resolve::buffers::enter_shared_reads);
    let mut runtime = ReferenceAnalysisRuntime {
        transclusion: TransclusionRuntime::new(options.compose.max_transclusion_depth),
        cache: make_cache(options),
        trigger_warnings: Vec::new(),
        earlier_warnings: Vec::new(),
        prepared_copies: std::collections::HashMap::new(),
        tolerant: std::collections::HashSet::new(),
    };

    // Seed the runtime with the root node so child documents that
    // transclude the root are detected as cycles immediately.
    let root_id = source_to_id(&source);
    let _ = runtime
        .transclusion
        .enter(root_id.to_string(), source_to_path(&source), 1);

    // Accumulates one compact identity per unique visited local child.
    let mut dependencies = ReferenceDependencyManifest::default();
    let mut prepared_headings = PreparedHeadingSnapshot::default();

    let (root, all_nodes) = build_node(
        md,
        &source,
        options,
        &mut runtime,
        extract_references,
        &mut dependencies,
        &mut prepared_headings,
    )
    .inspect_err(|_| {
        options.compose.options().preflight_failure_warnings.record(std::mem::take(&mut runtime.earlier_warnings));
    })?;

    runtime.transclusion.exit();

    Ok(ReferenceGraph::from_build(
        md,
        options,
        mode,
        root,
        all_nodes,
        dependencies,
        prepared_headings,
    )
    .with_trigger_warnings(runtime.trigger_warnings)
    .with_tolerant_nodes(runtime.tolerant)
    .with_earlier_warnings(runtime.earlier_warnings))
}

/// Build a transclusion-only graph (no link/image extraction at leaf nodes).
pub(crate) fn build_transclusion_graph(
    md: &Markdown,
    options: &ReferenceGraphOptions,
) -> MarkdownResult<ReferenceGraph> {
    build_graph_inner(md, options, ReferenceGraphMode::TransclusionOnly)
}

/// Build a full reference graph (transclusions + all reference types at each node).
pub(crate) fn build_reference_graph(
    md: &Markdown,
    options: &ReferenceGraphOptions,
) -> MarkdownResult<ReferenceGraph> {
    build_graph_inner(md, options, ReferenceGraphMode::Full)
}

/// Flatten a reference graph into composed-order [`ReferenceSet`].
pub(crate) fn flatten_graph(graph: &ReferenceGraph) -> ReferenceSet {
    ReferenceSet {
        records: flatten_occurrences(graph)
            .records
            .into_iter()
            .map(|(record, _)| record)
            .collect(),
    }
}

/// A graph walked in composed order, one entry per traversal occurrence.
#[derive(Default)]
pub(crate) struct ReferenceOccurrences {
    /// Each reference with the index in `documents` of the document copy it
    /// was reached through.
    pub(crate) records: Vec<(ReferenceRecord, usize)>,
    /// Each inclusion of a document, depth-first in insertion order, root
    /// first. A child transcluded twice is here twice with different trails.
    pub(crate) documents: Vec<ReferenceReportDocument>,
}

/// Walks the graph, keeping the inclusion trail of every document copy.
pub(crate) fn flatten_occurrences(graph: &ReferenceGraph) -> ReferenceOccurrences {
    let mut out = ReferenceOccurrences::default();
    flatten_node(graph.root(), graph, &mut Vec::new(), &mut out);
    out
}

/// Recursively flatten a node's references in composed order.
///
/// Supports multiple insertions on the same directive line by grouping
/// them in a `BTreeMap<usize, Vec<&ReferenceInsertion>>` and sorting
/// each group by `insertion_order`.
///
/// When a transclusion record and its child insertion share the same line
/// (which is the normal case), the child subtree is emitted immediately
/// after the transclusion record.
fn flatten_node(
    node: &ReferenceGraphNode,
    graph: &ReferenceGraph,
    trail: &mut Vec<ReferenceTrailStep>,
    out: &mut ReferenceOccurrences,
) {
    let occurrence = out.documents.len();
    out.documents.push(ReferenceReportDocument {
        source: node.source.clone(),
        included_via: trail.clone(),
        tolerates_invalid_references: graph.tolerates_invalid(&node.node_id),
    });

    // Build an index of child insertions by directive line for interleaving.
    let mut insertion_map: BTreeMap<usize, Vec<&ReferenceInsertion>> = BTreeMap::new();
    for insertion in &node.child_insertions {
        insertion_map
            .entry(insertion.directive_line)
            .or_default()
            .push(insertion);
    }
    for insertions in insertion_map.values_mut() {
        insertions.sort_by_key(|i| i.insertion_order);
    }

    // Track which insertion lines have been emitted to avoid double-processing.
    let mut emitted_insertion_lines = std::collections::HashSet::new();

    // Yield local references, interleaving child subtrees at insertion points.
    let mut last_line: usize = 0;
    for record in &node.local_references.records {
        // Insert child subtrees whose directive_line falls before this reference.
        // Guard: only create range when last_line <= record line (multiple records
        // at the same line can cause last_line to exceed the current line).
        if last_line < record.origin.line {
            for (&line, insertions) in insertion_map.range(last_line..record.origin.line) {
                if emitted_insertion_lines.insert(line) {
                    flatten_insertions(node, insertions, graph, trail, out);
                }
            }
        }

        // Emit this reference
        out.records.push((record.clone(), occurrence));

        // If a child insertion exists at the same line as this reference,
        // emit it immediately after (this is the transclusion point).
        if emitted_insertion_lines.insert(record.origin.line)
            && let Some(insertions) = insertion_map.get(&record.origin.line)
        {
            flatten_insertions(node, insertions, graph, trail, out);
        }

        last_line = record.origin.line.saturating_add(1);
    }

    // Flush remaining child insertions after all local references
    for (&line, insertions) in insertion_map.range(last_line..) {
        if emitted_insertion_lines.insert(line) {
            flatten_insertions(node, insertions, graph, trail, out);
        }
    }
}

/// Walks `node`'s children at one directive line, each under its own trail
/// step.
fn flatten_insertions(
    node: &ReferenceGraphNode,
    insertions: &[&ReferenceInsertion],
    graph: &ReferenceGraph,
    trail: &mut Vec<ReferenceTrailStep>,
    out: &mut ReferenceOccurrences,
) {
    for insertion in insertions {
        if let Some(child_node) = graph.node_by_id(insertion.child_node_id.as_ref()) {
            trail.push(ReferenceTrailStep {
                file: node.source.clone(),
                line: insertion.authored_directive_line,
            });
            flatten_node(child_node, graph, trail, out);
            trail.pop();
        }
    }
}

/// Build a heading index from prepared content.
///
/// Returns `(start_line, heading_text, heading_level)` tuples sorted by line
/// number for section-context lookups, plus the lowercased heading slugs in
/// document order. Both projections come from the build's single TOC parse;
/// the slugs are exactly the set fragment validation checks against.
fn build_heading_index(
    prepared_content: &str,
) -> (Vec<(usize, String, HeadingLevel)>, Vec<String>) {
    let temp_md = Markdown::new(prepared_content);
    let toc = temp_md.toc();
    let headings = toc.all_headings();
    let slugs: Vec<String> = headings.iter().map(|h| h.slug.to_lowercase()).collect();
    let mut index: Vec<(usize, String, HeadingLevel)> = headings
        .into_iter()
        .map(|h| (h.line_range.0, h.title.clone(), h.level))
        .collect();
    index.sort_by_key(|(line, _, _)| *line);
    (index, slugs)
}

/// Look up the active section for a given line number.
///
/// Finds the heading with the greatest start line that is `<= target_line`.
fn section_at_line(
    heading_index: &[(usize, String, HeadingLevel)],
    target_line: usize,
) -> Option<(&str, HeadingLevel)> {
    heading_index
        .iter()
        .rev()
        .find(|(line, _, _)| *line <= target_line)
        .map(|(_, text, level)| (text.as_str(), *level))
}

/// Maps one document's prepared-body offsets to its loaded file.
///
/// Exists only for a document with a file behind it whose map describes the
/// prepared body, so an in-memory or remote document has no authored position
/// rather than a prepared one.
pub(crate) struct AuthoredLocator<'a> {
    prepared: &'a str,
    loaded: &'a str,
    origin: &'a BodyOrigin,
    /// Byte offset of each line of `loaded`.
    line_starts: Vec<usize>,
}

impl<'a> AuthoredLocator<'a> {
    pub(crate) fn new(
        md: &'a Markdown,
        source: &ComposeSource,
        prepared: &'a str,
        origin: Option<&'a BodyOrigin>,
    ) -> Option<Self> {
        if !matches!(source, ComposeSource::File(_)) {
            return None;
        }
        let loaded = md.loaded_source()?.text.as_ref();
        let origin = origin.filter(|origin| origin.describes(prepared));
        debug_assert!(
            origin.is_some(),
            "a document loaded from a file keeps a map of its prepared body",
        );
        Some(Self {
            prepared,
            loaded,
            origin: origin?,
            line_starts: std::iter::once(0)
                .chain(loaded.match_indices('\n').map(|(index, _)| index + 1))
                .collect(),
        })
    }

    /// `range` of the loaded file as a location.
    fn location(&self, range: std::ops::Range<usize>, inserted: bool) -> AuthoredReferenceLocation {
        let line = self.line_starts.partition_point(|start| *start <= range.start);
        let line_start = self.line_starts[line - 1];
        // A byte-order mark is an encoding signature, not a column.
        let counted = self.loaded[line_start..range.start].trim_start_matches('\u{FEFF}');
        AuthoredReferenceLocation { line, column: counted.chars().count() + 1, span: range, inserted }
    }

    /// The location of the prepared-body `span`, from its first character past
    /// any indentation: the whole span when every byte of it is authored, else
    /// that character, or the construct that wrote the character. `None` when
    /// neither is known.
    pub(crate) fn body_location(&self, span: &std::ops::Range<usize>) -> Option<AuthoredReferenceLocation> {
        let text = self.prepared.get(span.clone())?;
        let start = span.end - text.trim_start_matches([' ', '\t']).len();
        let first = start..start + self.prepared.get(start..)?.chars().next()?.len_utf8();
        if let Some(first) = self.origin.project(&first) {
            let whole = (start < span.end).then(|| self.origin.project_whole(&(start..span.end), self.loaded)).flatten();
            return Some(self.location(whole.unwrap_or(first), false));
        }
        self.origin.producer_of(start).map(|construct| self.location(construct, true))
    }

    /// The construct that wrote the first stage-written byte of the
    /// prepared-body `span`; `None` when every byte of it was copied.
    pub(crate) fn produced_location(&self, span: &std::ops::Range<usize>) -> Option<AuthoredReferenceLocation> {
        span.clone().find_map(|offset| self.origin.producer_of(offset)).map(|construct| self.location(construct, true))
    }

    /// The location of a `prologue:` or `epilogue:` value reading `expected`
    /// (`index` for one item of a list); `None` when it cannot be proven.
    fn frontmatter_location(&self, key: &str, index: Option<usize>, expected: &str) -> Option<AuthoredReferenceLocation> {
        let range = crate::markdown::compose::transclusion::frontmatter_value_range(self.loaded, key, index, expected)?;
        Some(self.location(range, false))
    }
}

/// `origin`, whose span is in the prepared body, with its authored position
/// and section. `locator` is `None` when the document has no authored
/// positions.
fn placed_in_body(
    origin: ReferenceOrigin,
    locator: Option<&AuthoredLocator<'_>>,
    headings: &[PreparedHeading],
) -> ReferenceOrigin {
    let authored = locator.and_then(|locator| locator.body_location(&origin.span));
    let section = heading_at(headings, origin.span.start).map(|heading| heading.slug.clone());
    ReferenceOrigin { authored, section, ..origin }
}

/// The file line of the directive occupying the prepared-body `span`.
fn authored_directive_line(locator: Option<&AuthoredLocator<'_>>, span: &std::ops::Range<usize>) -> Option<usize> {
    locator.and_then(|locator| locator.body_location(span)).map(|location| location.line)
}

/// The options for a document the graph prepares that composition never
/// includes as a child (a `::toc-linking` target, whose headings are read
/// raw, or a fragment target outside the graph): the source and prose setting
/// change, no parent state is inherited, and no inclusion's `replace=` option
/// (one-off map or parent-wins mode) applies. Inclusion children use
/// [`inclusion_child_options`].
///
/// `opening` is the reference that resolved to `path`, when the caller has it
/// (see [`SourceOpening`](crate::markdown::compose::context::options::SourceOpening)).
fn accepted_child_options(
    options: &ReferenceGraphOptions,
    path: &std::path::Path,
    opening: Option<crate::markdown::compose::context::options::SourceOpening>,
    prose: ProseInterpolation,
) -> ReferenceGraphOptions {
    ReferenceGraphOptions::from_compose_options(
        options
            .compose
            .clone()
            .with_accepted_source_file(path, opening)
            .derive(|compose| {
                compose
                    .with_prose_interpolation(prose)
                    .with_one_off_replace(None)
                    .with_replace_parent_wins(false)
            }),
    )
}

/// The options an inclusion child (a body `::file`, `prologue:`, or
/// `epilogue:` target) is prepared with: the ones composition gives it
/// ([`markdown_child_options`](crate::markdown::compose::transclusion::markdown_child_options)),
/// with the parent's effective state (`parent_data`) as inherited data.
fn inclusion_child_options(
    options: &ReferenceGraphOptions,
    parent_data: &crate::markdown::compose::SharedFrontmatter,
    directive_options: &crate::markdown::compose::transclusion::BlockOptions,
    path: &std::path::Path,
    opening: Option<crate::markdown::compose::context::options::SourceOpening>,
) -> ReferenceGraphOptions {
    ReferenceGraphOptions::from_compose_options(crate::markdown::compose::transclusion::markdown_child_options(
        &options.compose,
        parent_data,
        directive_options,
        path,
        opening,
    ))
}

/// The inclusions one parser found in a document. A parser failure (a body
/// directive with no target, an invalid `::toc-linking` level or
/// `::file-links` depth, a `prologue`/`epilogue` that is not a string or list
/// of strings) fails the graph in both build modes, so no reference reader
/// reports a document it could not read as clean. Only a reference check
/// running ahead of composition
/// ([`ComposeOptions::for_reference_check`](crate::markdown::compose::ComposeOptions::for_reference_check))
/// leaves the failure to composition and lists nothing.
fn parsed_inclusions<T: Default>(options: &ReferenceGraphOptions, parsed: MarkdownResult<T>) -> MarkdownResult<T> {
    match parsed {
        Err(_) if options.compose.defer_inclusion_parse_failures => Ok(T::default()),
        parsed => parsed,
    }
}

/// Build a single graph node for a document.
///
/// Returns the node itself plus all descendant nodes collected during
/// recursive traversal. This ensures the caller can assemble a complete
/// flat node list for the graph.
///
/// Every file-sourced node's prepared heading slugs are recorded into
/// `prepared_headings`: fragment validation reads descendant headings from
/// that snapshot rather than rereading children from disk, so a post-build
/// heading edit cannot leak into a one-step report.
fn build_node(
    md: &Markdown,
    source: &ComposeSource,
    options: &ReferenceGraphOptions,
    runtime: &mut ReferenceAnalysisRuntime,
    extract_references: bool,
    dependencies: &mut ReferenceDependencyManifest,
    prepared_headings: &mut PreparedHeadingSnapshot,
) -> MarkdownResult<(ReferenceGraphNode, Vec<ReferenceGraphNode>)> {
    // Always run InlinePre preparation (rec #2): page blocks, interpolation,
    // shell expansion, and text replacement can affect references even in
    // leaf documents with no transclusions.
    let PreparedReferenceContent {
        content: prepared_content,
        document: prepared_document,
        body_data,
        body_origin,
        scopes,
        warnings,
        passive_warnings,
    } = prepare_content(md, options).map_err(|failure| {
        runtime.earlier_warnings.extend(failure.warnings);
        failure.error
    })?;
    // The rest of a lenient check's warnings stand in for failures the
    // composition would raise (as in the shell-approval walk).
    runtime.earlier_warnings.extend(warnings.iter().take(passive_warnings).cloned());
    let body_data = body_data.as_ref();
    let locator = AuthoredLocator::new(md, source, &prepared_content, body_origin.as_ref());
    let locator = locator.as_ref();
    for warning in warnings {
        if let Some(trigger) = warning.invalid_trigger_source()
            && !runtime
                .trigger_warnings
                .iter()
                .any(|kept| kept.invalid_trigger_source().as_ref() == Some(&trigger))
        {
            runtime.trigger_warnings.push(warning);
        }
    }

    let ctx = {
        let base = md.source_context_for_errors();
        SourceContext::new(base.absolute, base.display, prepared_content.clone())
    };

    // Build heading index for section context on transclusion insertions
    let (heading_index, prepared_slugs) = build_heading_index(&prepared_content);
    let headings = prepared_headings_of(&prepared_content);

    if let ComposeSource::File(path) = source {
        prepared_headings.record(path, prepared_slugs, headings.clone());
    }

    // Extract local references if requested
    let mut local_references = if extract_references {
        extract_all_references(&prepared_content, source)
    } else {
        ReferenceSet::default()
    };
    // A reference an expression inserted is data: validation still checks it,
    // but a missing target warns instead of failing the run.
    if let Some(data) = body_data {
        for record in &mut local_references.records {
            if data.intersects(&record.origin.span) {
                record.attributes.insert(
                    DATA_ORIGIN_ATTRIBUTE.to_string(),
                    serde_json::Value::String("data".to_string()),
                );
            }
        }
    }
    for record in &mut local_references.records {
        record.origin = placed_in_body(record.origin.clone(), locator, &headings);
    }

    // Parse transclusion directives and build child nodes
    let mut child_insertions = Vec::new();
    let mut all_descendant_nodes: Vec<ReferenceGraphNode> = Vec::new();
    let mut insertion_order = 0;

    // The state composition's transclusion stage builds from the resolved
    // frontmatter: `when=` conditions read it, and every inclusion child
    // inherits it as data.
    let effective_state = crate::markdown::compose::pipeline::document_state_builder(
        &prepared_document,
        &options.compose,
    )
    .build()
    .unwrap_or_else(|_| {
        // Context merge failure during reference analysis: fall back to
        // a state without user ctx so reference extraction can proceed.
        EffectiveStateBuilder::new()
            .with_context(options.compose.context().clone())
            .build()
            .expect("empty frontmatter has no user ctx")
    });
    // Less the parent's `$schema`, as pre-flight discovery hands it on: the
    // parent already coerced these values, and re-judging them would resolve
    // its relative file values from the child's directory.
    let mut child_data = effective_state.data().clone();
    child_data.remove("$schema");

    // Validation evaluates the same `when=` conditions compose will, against
    // the request's one repository observation: fixed at request creation, or
    // by the root pipeline entry `prepare_content` already ran above.
    // With the run's remote-fetch runtime, as composition evaluates them, so
    // a condition reading an allowed URL decides here as it does there.
    let when_options = options.compose.clone();
    let remote_fetch = when_options.remote_fetch_runtime();
    let document_lookup = ResolvingLookup::new(
        &effective_state,
        when_options.expression_resolution_context(&remote_fetch),
    );
    // A directive in a repeated body reads the frame of its copy.
    let when_lookup = SiteLookup::new(&document_lookup, &scopes, None);
    let when_origin = body_origin.as_ref().filter(|origin| origin.describes(&prepared_content));

    // Block directives
    let directives = parsed_inclusions(
        options,
        // A dry run's directive whose line holds a value it left as written
        // names no target, as composition leaves it unresolved.
        crate::markdown::compose::transclusion::parse_directives_deferring(
            &prepared_content,
            body_data,
            ctx.clone(),
            0,
            &|line| options.compose.passive_session().and_then(|session| session.deferred_span_in(line)),
        )
        .map(|(directives, _)| directives)
        .map_err(|error| error.at_authored_directive(&prepared_document, body_origin.as_ref(), 0).into()),
    )?;
    for directive in &directives {
        // A false `when=` leaves the directive out, as composition does. A
        // condition that fails is composition's error, located as composition
        // locates it, unless this is the reference check running ahead of
        // composition, which leaves the verdict to it and skips the directive.
        if let Some(ref when_expr) = directive.options.when_expr {
            when_lookup.enter(directive.span.start);
            let site = conditions::ConditionSite {
                span: &directive.span,
                body_line: directive.line,
                opener: &|span| crate::markdown::compose::site_scope::authored_opener(&prepared_document, when_origin, span),
                current: &|| ctx.clone(),
            };
            match site.evaluate(when_expr, &when_lookup, &mut ()) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(_) if options.compose.defer_expression_failures => continue,
                Err(error) => {
                    return Err(crate::markdown::compose::TransclusionError::from(error).into());
                }
            }
        }

        // Emit transclusion reference records (rec #4)
        if extract_references {
            let syntax = match directive.kind {
                DirectiveKind::File => ReferenceSyntax::DirectiveFile,
                DirectiveKind::Code => ReferenceSyntax::DirectiveCode,
                DirectiveKind::Url => ReferenceSyntax::DirectiveUrl,
            };
            local_references.records.push(ReferenceRecord {
                id: make_reference_id(source, directive.line, directive.span.start),
                kind: ReferenceKind::Transclusion,
                target: match directive.kind {
                    // `::url` opens nothing but a URL; validation applies its
                    // admission rule before any hyperlink or remote check.
                    DirectiveKind::Url => ReferenceTarget::RemoteUrl { raw: directive.raw_target.clone() },
                    DirectiveKind::File | DirectiveKind::Code => opened_target(&directive.raw_target),
                },
                origin: placed_in_body(
                    ReferenceOrigin {
                        source: source.clone(),
                        line: directive.line,
                        span: directive.span.clone(),
                        syntax,
                        authored: None,
                        section: None,
                    },
                    locator,
                    &headings,
                ),
                attributes: serde_json::Map::new(),
            });
        }

        if directive.kind == DirectiveKind::File {
            // A target failure names the directive's opener in its file.
            let inclusion = || {
                let site = crate::markdown::compose::site_scope::DirectiveSite::of_span(
                    &prepared_document,
                    body_origin.as_ref(),
                    &directive.span,
                );
                InclusionSite::of_directive(&prepared_document, &site)
            };
            // Try to resolve and recurse into the child
            if let Some(child_path) = resolve_local_target(&directive.raw_target, source, &options.compose)
                .map_err(|error| super::at_inclusion(error, inclusion()))?
                && admits_markdown_child(&child_path, options, inclusion())?
            {
                let child_source = ComposeSource::File(child_path.clone());
                let child_id = source_to_id(&child_source);

                // Cycle/depth check
                if runtime.enter_inclusion(child_id.to_string(), child_path.clone(), directive.line, options)? {
                    if let Some(child_md) = runtime.load_inclusion(&child_path, options, inclusion())? {
                        // Record this visited local child from the value we
                        // just loaded (deduped per unique resolved source).
                        dependencies.record(
                            child_source.clone(),
                            ReferenceDocumentIdentity::capture(&child_md),
                        );
                        // The child is prepared with the directive's `set`
                        // values, as composition runs it, so its branches
                        // select the references composition keeps.
                        let mut child_md = child_md;
                        crate::markdown::compose::transclusion::apply_directive_set_overlay(
                            &mut child_md,
                            &directive.options,
                        );
                        let child_options = inclusion_child_options(
                            options,
                            &child_data,
                            &directive.options,
                            &child_path,
                            crate::markdown::compose::transclusion::source_opening(&directive.raw_target, &child_path),
                        );
                        let (child_node, mut descendants) = build_node(
                            &child_md,
                            &child_source,
                            &child_options,
                            runtime,
                            extract_references,
                            dependencies,
                            prepared_headings,
                        )?;

                        let (sec_text, sec_level) =
                            section_at_line(&heading_index, directive.line)
                                .map(|(t, l)| (Some(t.to_string()), Some(l)))
                                .unwrap_or((None, None));
                        let ref_id =
                            make_reference_id(source, directive.line, directive.span.start);
                        child_insertions.push(ReferenceInsertion {
                            child_node_id: child_node.node_id.clone(),
                            directive_line: directive.line,
                            authored_directive_line: authored_directive_line(locator, &directive.span),
                            insertion_order,
                            reference_id: Some(ref_id),
                            context: ReferenceInsertionContext {
                                directive_kind: Some(ReferenceSyntax::DirectiveFile),
                                section_heading_text: sec_text,
                                section_heading_level: sec_level,
                            },
                        });
                        all_descendant_nodes.push(child_node);
                        all_descendant_nodes.append(&mut descendants);
                        insertion_order += 1;
                    }
                    runtime.transclusion.exit();
                }
            }
        }
    }

    // ::toc-linking directives generate synthesized TOC hyperlinks AND
    // create child graph nodes for follow-mode expansion.
    let toc_directives = parsed_inclusions(
        options,
        crate::markdown::compose::toc_linking::parse_directives_in(&prepared_content, body_data).map_err(|error| {
            error.with_document(md.error_document()).at_authored_line(&prepared_document, body_origin.as_ref()).into()
        }),
    )?;
    let transclusion_options = options.compose.transclusion_options();

    for directive in &toc_directives {
        // A dry run's target the run's initializer creates or changes first
        // is selected, and its headings read, only by the run.
        if let Some(session) = options.compose.passive_session()
            && toc_linking::changed_at_launch(directive, source, &transclusion_options, ctx.clone(), session).is_some()
        {
            continue;
        }
        let ref_id = make_reference_id(source, directive.line, directive.span.start);

        // Section context is always available from the parent document's
        // heading index, regardless of whether the target can be loaded.
        let (sec_text, sec_level) = section_at_line(&heading_index, directive.line)
            .map(|(t, l)| (Some(t.to_string()), Some(l)))
            .unwrap_or((None, None));

        // Composition's selection: the first existing alternative, a
        // suppressed chain (trailing `false`), or a required failure.
        let selection =
            toc_linking::resolve_target_chain(directive, source, &transclusion_options, ctx.clone());
        let suppressed = matches!(selection, Ok(None));
        if let Ok(Some((display_target, path))) = selection {
            if extract_references {
                local_references.records.push(ReferenceRecord {
                    id: ref_id.clone(),
                    kind: ReferenceKind::Transclusion,
                    // The selector chose this alternative as a local file.
                    target: ReferenceTarget::LocalPath { raw: display_target.clone().into() },
                    origin: placed_in_body(
                        ReferenceOrigin {
                            source: source.clone(),
                            line: directive.line,
                            span: directive.span.clone(),
                            syntax: ReferenceSyntax::DirectiveTocLinking,
                            authored: None,
                            section: None,
                        },
                        locator,
                        &headings,
                    ),
                    attributes: serde_json::Map::new(),
                });
            }

            // Create a child graph node so follow mode can expand the
            // target document as a nested FileTree subtree.
            let child_source = ComposeSource::File(path.clone());
            let child_id = source_to_id(&child_source);

            // Generate synthesized TOC link references on the parent
            // node (composed view). Tagged so the model builder only
            // includes them when follow=true.
            // The directive that selected the target: where a target that
            // cannot be read is reported, both reading its headings and
            // loading it as a child.
            let inclusion = InclusionSite::of_directive(
                &prepared_document,
                &crate::markdown::compose::site_scope::DirectiveSite::of_span(
                    &prepared_document,
                    body_origin.as_ref(),
                    &directive.span,
                ),
            );
            if extract_references {
                let (records, raw_slugs) = generate_toc_link_references(
                    &display_target,
                    &path,
                    directive,
                    source,
                    runtime,
                    options,
                    inclusion.clone(),
                )?;
                for mut record in records {
                    record
                        .attributes
                        .insert("toc_synthesized".to_string(), serde_json::Value::Bool(true));
                    record.origin = placed_in_body(record.origin, locator, &headings);
                    prepared_headings.record_toc_reference(&record, std::sync::Arc::clone(&raw_slugs));
                    local_references.records.push(record);
                }
            }

            if runtime.enter_inclusion(child_id.to_string(), path.clone(), directive.line, options)? {
                if let Some(child_md) = runtime.load_inclusion(&path, options, inclusion)? {
                    dependencies.record(
                        child_source.clone(),
                        ReferenceDocumentIdentity::capture(&child_md),
                    );
                    let (child_node, mut descendants) = build_node(
                        &child_md,
                        &child_source,
                        &accepted_child_options(options, &path, None, options.compose.prose_interpolation),
                        runtime,
                        extract_references,
                        dependencies,
                        prepared_headings,
                    )?;

                    child_insertions.push(ReferenceInsertion {
                        child_node_id: child_node.node_id.clone(),
                        directive_line: directive.line,
                        authored_directive_line: authored_directive_line(locator, &directive.span),
                        insertion_order,
                        reference_id: Some(ref_id.clone()),
                        context: ReferenceInsertionContext {
                            directive_kind: Some(ReferenceSyntax::DirectiveTocLinking),
                            section_heading_text: sec_text.clone(),
                            section_heading_level: sec_level,
                        },
                    });
                    all_descendant_nodes.push(child_node);
                    all_descendant_nodes.append(&mut descendants);
                    insertion_order += 1;
                }
                runtime.transclusion.exit();
            }

            // If no child insertion was created (target couldn't load),
            // still create a context-only insertion so the caption has
            // section information.
            if !child_insertions
                .iter()
                .any(|ins| ins.reference_id.as_deref() == Some(&ref_id))
            {
                child_insertions.push(ReferenceInsertion {
                    child_node_id: NodeId::from(""),
                    directive_line: directive.line,
                    authored_directive_line: authored_directive_line(locator, &directive.span),
                    insertion_order,
                    reference_id: Some(ref_id),
                    context: ReferenceInsertionContext {
                        directive_kind: Some(ReferenceSyntax::DirectiveTocLinking),
                        section_heading_text: sec_text,
                        section_heading_level: sec_level,
                    },
                });
                insertion_order += 1;
            }
        } else {
            // Target didn't resolve — still emit reference record and
            // a context-only insertion for the caption. The record carries
            // the selector's outcome, suppressed or a required failure, so
            // validation reports exactly that and never re-reads the first
            // alternative as a link, a URL, or a file.
            if extract_references && let Some(raw_target) = directive.targets.first() {
                let mut attributes = serde_json::Map::new();
                let outcome = if suppressed { TOC_SUPPRESSED_ATTRIBUTE } else { TOC_UNRESOLVED_ATTRIBUTE };
                attributes.insert(outcome.to_string(), serde_json::Value::Bool(true));
                local_references.records.push(ReferenceRecord {
                    id: ref_id.clone(),
                    kind: ReferenceKind::Transclusion,
                    // The selector reads every alternative as a local file,
                    // so a URL, `#…`, or scheme-like alternative it rejected
                    // is a missing file, never a hyperlink to check.
                    target: ReferenceTarget::LocalPath { raw: raw_target.into() },
                    origin: placed_in_body(
                        ReferenceOrigin {
                            source: source.clone(),
                            line: directive.line,
                            span: directive.span.clone(),
                            syntax: ReferenceSyntax::DirectiveTocLinking,
                            authored: None,
                            section: None,
                        },
                        locator,
                        &headings,
                    ),
                    attributes,
                });
            }

            child_insertions.push(ReferenceInsertion {
                child_node_id: NodeId::from(""),
                directive_line: directive.line,
                authored_directive_line: authored_directive_line(locator, &directive.span),
                insertion_order,
                reference_id: Some(ref_id),
                context: ReferenceInsertionContext {
                    directive_kind: Some(ReferenceSyntax::DirectiveTocLinking),
                    section_heading_text: sec_text,
                    section_heading_level: sec_level,
                },
            });
            insertion_order += 1;
        }
    }

    // ::file-links directives emit reference records but do not create
    // child insertions (they are not followable transclusions).
    let file_links_directives = parsed_inclusions(
        options,
        crate::markdown::compose::file_links::parse_file_links_directives_in(&prepared_content, body_data).map_err(|error| {
            error.with_document(md.error_document()).at_authored_line(&prepared_document, body_origin.as_ref()).into()
        }),
    )?;
    for directive in &file_links_directives {
        if extract_references {
            let raw_target = match &directive.mode {
                crate::markdown::compose::file_links::FileLinksMode::Glob(g) => g.clone(),
                crate::markdown::compose::file_links::FileLinksMode::Dir { path, depth } => {
                    format!("{} --depth {}", path, depth)
                }
            };
            local_references.records.push(ReferenceRecord {
                id: make_reference_id(source, directive.line, directive.span.start),
                kind: ReferenceKind::Transclusion,
                target: classify_target(&raw_target),
                origin: placed_in_body(
                    ReferenceOrigin {
                        source: source.clone(),
                        line: directive.line,
                        span: directive.span.clone(),
                        syntax: ReferenceSyntax::DirectiveFileLinks,
                        authored: None,
                        section: None,
                    },
                    locator,
                    &headings,
                ),
                attributes: serde_json::Map::new(),
            });
        }
    }

    // Frontmatter prologue/epilogue, read from the prepared frontmatter as
    // composition reads them. An omitted field or `[]` lists nothing.
    let fm_refs = parsed_inclusions(
        options,
        parse_frontmatter_refs(prepared_document.frontmatter().as_map(), md.frontmatter_error_context())
            .map_err(MarkdownError::from),
    )?;
    // A dry run's value it could not produce names no target, as composition
    // leaves it unread.
    let deferred_reference =
        |value: &str| options.compose.passive_session().is_some_and(|session| session.deferred_reference(value).is_some());
    let value_location = |key: &str, idx: usize, expected: &str| {
        let in_list = md.frontmatter().as_map().get(key).is_some_and(serde_json::Value::is_array);
        let index = in_list.then_some(idx);
        match locator {
            Some(locator) => locator.frontmatter_location(key, index, expected),
            // A generated body beneath a file's frontmatter has no body map,
            // but its frontmatter values are still the file's.
            None => md
                .frontmatter_origin()
                .filter(|origin| matches!(source, ComposeSource::File(file) if *file == origin.file))
                .and_then(|origin| {
                    let range =
                        crate::markdown::compose::transclusion::frontmatter_value_range(&origin.text, key, index, expected)?;
                    let span = crate::markdown::types::AuthoredSpan::locate(&origin.text, range)?;
                    Some(AuthoredReferenceLocation {
                        line: span.line(),
                        column: span.column(),
                        span: span.range(),
                        inserted: false,
                    })
                }),
        }
    };
    for (idx, prologue) in fm_refs.prologue.iter().enumerate() {
        if deferred_reference(prologue) {
            continue;
        }
        let is_literal = frontmatter_value_is_inline(prologue);
        let location = value_location("prologue", idx, prologue);

        if extract_references {
            let mut attrs = serde_json::Map::new();
            if is_literal {
                attrs.insert("fm_literal".to_string(), serde_json::Value::Bool(true));
                // Inline text is inserted as body Markdown, so every
                // reference kind the body reads is read from it too.
                let links = extract_all_references(prologue, source).records;
                for mut link in links {
                    link.attributes
                        .insert("toc_synthesized".to_string(), serde_json::Value::Bool(true));
                    link.origin.authored.clone_from(&location);
                    local_references.records.push(link);
                }
            }
            local_references.records.push(ReferenceRecord {
                id: make_reference_id(source, 0, idx),
                kind: ReferenceKind::Transclusion,
                // Inline text has no target of its own; its links are the
                // records extracted above.
                target: if is_literal {
                    ReferenceTarget::Inline
                } else {
                    opened_target(prologue)
                },
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line: 0,
                    span: 0..0,
                    syntax: ReferenceSyntax::FrontmatterPrologue,
                    authored: location.clone(),
                    section: None,
                },
                attributes: attrs,
            });
        }

        let inclusion = || InclusionSite::of_frontmatter_value(md, "prologue", idx, prologue);
        if !is_literal
            && let Some(child_path) = resolve_local_target(prologue, source, &options.compose)
                .map_err(|error| super::at_inclusion(error, inclusion()))?
            && admits_markdown_child(&child_path, options, inclusion())?
        {
            let child_source = ComposeSource::File(child_path.clone());
            let child_id = source_to_id(&child_source);

            if runtime.enter_inclusion(child_id.to_string(), child_path.clone(), 0, options)? {
                if let Some(child_md) = runtime.load_inclusion(&child_path, options, inclusion())? {
                    dependencies.record(
                        child_source.clone(),
                        ReferenceDocumentIdentity::capture(&child_md),
                    );
                    let (child_node, mut descendants) = build_node(
                        &child_md,
                        &child_source,
                        &inclusion_child_options(
                            options,
                            &child_data,
                            &crate::markdown::compose::transclusion::BlockOptions::default(),
                            &child_path,
                            crate::markdown::compose::transclusion::source_opening(prologue, &child_path),
                        ),
                        runtime,
                        extract_references,
                        dependencies,
                        prepared_headings,
                    )?;

                    let ref_id = make_reference_id(source, 0, idx);
                    child_insertions.push(ReferenceInsertion {
                        child_node_id: child_node.node_id.clone(),
                        directive_line: 0, // prologue goes at the start
                        authored_directive_line: location.as_ref().map(|location| location.line),
                        insertion_order,
                        reference_id: Some(ref_id),
                        context: ReferenceInsertionContext {
                            directive_kind: Some(ReferenceSyntax::FrontmatterPrologue),
                            section_heading_text: None,
                            section_heading_level: None,
                        },
                    });
                    all_descendant_nodes.push(child_node);
                    all_descendant_nodes.append(&mut descendants);
                    insertion_order += 1;
                }
                runtime.transclusion.exit();
            }
        }
    }

    for (idx, epilogue) in fm_refs.epilogue.iter().enumerate() {
        if deferred_reference(epilogue) {
            continue;
        }
        let is_literal = frontmatter_value_is_inline(epilogue);
        let location = value_location("epilogue", idx, epilogue);

        if extract_references {
            let mut attrs = serde_json::Map::new();
            if is_literal {
                attrs.insert("fm_literal".to_string(), serde_json::Value::Bool(true));
                // Inline text is inserted as body Markdown, so every
                // reference kind the body reads is read from it too.
                let links = extract_all_references(epilogue, source).records;
                for mut link in links {
                    link.attributes
                        .insert("toc_synthesized".to_string(), serde_json::Value::Bool(true));
                    link.origin.authored.clone_from(&location);
                    local_references.records.push(link);
                }
            }
            local_references.records.push(ReferenceRecord {
                id: make_reference_id(source, usize::MAX, idx),
                kind: ReferenceKind::Transclusion,
                // Inline text has no target of its own; its links are the
                // records extracted above.
                target: if is_literal {
                    ReferenceTarget::Inline
                } else {
                    opened_target(epilogue)
                },
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line: usize::MAX,
                    span: 0..0,
                    syntax: ReferenceSyntax::FrontmatterEpilogue,
                    authored: location.clone(),
                    section: None,
                },
                attributes: attrs,
            });
        }

        let inclusion = || InclusionSite::of_frontmatter_value(md, "epilogue", idx, epilogue);
        if !is_literal
            && let Some(child_path) = resolve_local_target(epilogue, source, &options.compose)
                .map_err(|error| super::at_inclusion(error, inclusion()))?
            && admits_markdown_child(&child_path, options, inclusion())?
        {
            let child_source = ComposeSource::File(child_path.clone());
            let child_id = source_to_id(&child_source);

            if runtime.enter_inclusion(child_id.to_string(), child_path.clone(), 0, options)? {
                if let Some(child_md) = runtime.load_inclusion(&child_path, options, inclusion())? {
                    dependencies.record(
                        child_source.clone(),
                        ReferenceDocumentIdentity::capture(&child_md),
                    );
                    let (child_node, mut descendants) = build_node(
                        &child_md,
                        &child_source,
                        &inclusion_child_options(
                            options,
                            &child_data,
                            &crate::markdown::compose::transclusion::BlockOptions::default(),
                            &child_path,
                            crate::markdown::compose::transclusion::source_opening(epilogue, &child_path),
                        ),
                        runtime,
                        extract_references,
                        dependencies,
                        prepared_headings,
                    )?;

                    let (sec_text, sec_level) = heading_index
                        .last()
                        .map(|(_, t, l)| (Some(t.clone()), Some(*l)))
                        .unwrap_or((None, None));
                    let ref_id = make_reference_id(source, usize::MAX, idx);
                    child_insertions.push(ReferenceInsertion {
                        child_node_id: child_node.node_id.clone(),
                        directive_line: usize::MAX, // epilogue goes at the end
                        authored_directive_line: location.as_ref().map(|location| location.line),
                        insertion_order,
                        reference_id: Some(ref_id),
                        context: ReferenceInsertionContext {
                            directive_kind: Some(ReferenceSyntax::FrontmatterEpilogue),
                            section_heading_text: sec_text,
                            section_heading_level: sec_level,
                        },
                    });
                    all_descendant_nodes.push(child_node);
                    all_descendant_nodes.append(&mut descendants);
                    insertion_order += 1;
                }
                runtime.transclusion.exit();
            }
        }
    }

    // Re-sort after adding transclusion records
    if extract_references {
        local_references
            .records
            .sort_by_key(|r| (r.origin.line, r.origin.span.start));
    }

    // Composition asks the document that writes an inclusion, as prepared
    // (its parent's state passed down), whether a failing target is removed.
    let tolerant = crate::markdown::compose::transclusion::ignores_invalid(&options.compose, &prepared_document);
    let node_id = runtime.prepared_copy_id(source_to_id(source), &local_references, &child_insertions, tolerant);
    let main_node = ReferenceGraphNode {
        node_id,
        source: source.clone(),
        local_references,
        child_insertions,
    };

    Ok((main_node, all_descendant_nodes))
}

/// Extract all reference types from prepared content in a single pass.
///
/// Avoids repeated extraction passes over the same HTML (rec #17) by
/// extracting style blocks once and reusing them for CSS/font extraction.
pub(super) fn extract_all_references(content: &str, source: &ComposeSource) -> ReferenceSet {
    let mut records = Vec::new();

    // Markdown-native references
    records.extend(super::local::extract_markdown_links(content, source));
    records.extend(super::local::extract_markdown_images(content, source));

    // HTML references
    records.extend(super::html::extract_html_links(content, source));
    records.extend(super::html::extract_html_images(content, source));
    records.extend(super::html::extract_html_videos(content, source));
    records.extend(super::html::extract_html_audio(content, source));
    records.extend(super::html::extract_html_sources(content, source));
    records.extend(super::html::extract_html_iframes(content, source));
    records.extend(super::html::extract_html_script_blocks(content, source));
    records.extend(super::html::extract_html_link_tags(content, source));
    records.extend(super::html::extract_html_meta_tags(content, source));

    // Extract style blocks once and reuse for CSS/font extraction (rec #17)
    let style_blocks = super::html::extract_html_style_blocks(content, source);
    for style_record in &style_blocks {
        records.extend(super::css::style_references(content, style_record, super::css::extract_css_imports));
        records.extend(super::css::style_references(content, style_record, super::css::extract_font_face_sources));
    }
    records.extend(style_blocks);

    // Sort by line number for composed-order interleaving
    records.sort_by_key(|r| (r.origin.line, r.origin.span.start));
    ReferenceSet { records }
}

/// Prepare content for validation heading extraction.
///
/// This is a `pub(super)` wrapper around [`prepare_content`] so the
/// validator can run InlinePre preparation before extracting headings.
pub(super) fn prepare_content_for_validation(
    md: &Markdown,
    source: &ComposeSource,
    options: &ReferenceGraphOptions,
) -> MarkdownResult<String> {
    let node_options = match source {
        // A fragment target outside the graph is read as its own document,
        // not through an inclusion, so its prose interpolates.
        ComposeSource::File(path) if options.compose.source != *source => {
            accepted_child_options(options, path, None, ProseInterpolation::Enabled)
        }
        ComposeSource::Url(url) if options.compose.source != *source => {
            ReferenceGraphOptions::from_compose_options(
                options.compose.clone().with_source_url(url.clone()),
            )
        }
        _ => options.clone(),
    };
    prepare_content(md, &node_options).map(|prepared| prepared.content).map_err(|failure| failure.error)
}

/// A document after InlinePre preparation.
struct PreparedReferenceContent {
    content: String,
    /// The prepared document, whose frontmatter is resolved as composition
    /// resolves it: the graph's own conditions and its children's inherited
    /// state are read from it, never from the authored frontmatter.
    document: Markdown,
    /// Which bytes of `content` are data.
    body_data: Option<crate::markdown::compose::body_origin::DataRanges>,
    /// Where each byte of `content` came from.
    body_origin: Option<BodyOrigin>,
    /// The item frame of each `::map` / `::filter` copy in `content`.
    scopes: crate::markdown::compose::body_origin::ScopeRanges,
    warnings: Vec<ComposeWarning>,
    /// How many of `warnings` the document's passive checks found.
    passive_warnings: usize,
}

/// Prepare content by running only InlinePre operations.
///
/// Starts from the caller's `ComposeOptions` (rec #3) to preserve
/// external state, overrides, cache settings, shell settings, and per-node
/// source provenance, then restricts it to the InlinePre operations it
/// enables.
fn prepare_content(
    md: &Markdown,
    options: &ReferenceGraphOptions,
) -> Result<PreparedReferenceContent, Box<crate::markdown::compose::ComposeFailure>> {
    // Restricted, never widened: an operation the request disabled (the
    // passive `md compose` reference check's `ShellExpansion`) stays off.
    // A `::map` / `::filter` body is expanded as composition expands it, one
    // copy per retained item, so each copy's directive inputs and links read
    // their item's frame; condition-blind discovery would instead take every
    // `::block` gate and every filtered-out item, and report references
    // composition never produces. Shell expansion is left out of such a
    // document, so the check never runs a command a copy repeats.
    let repeats_bodies = md.content().contains("::map") || md.content().contains("::filter");
    let inline_pre_options = options.compose.clone().derive(|compose| {
        let expands_bodies = repeats_bodies && compose.is_enabled(ComposeOperation::CollectionBlocks);
        let enabled: Vec<ComposeOperation> = [
            // A schema and a condition judge the resolved values.
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::TextReplacement,
            ComposeOperation::PageBlocks,
            ComposeOperation::CollectionBlocks,
            ComposeOperation::Interpolation,
            ComposeOperation::ShellExpansion,
        ]
        .into_iter()
        .filter(|operation| compose.is_enabled(*operation))
        .filter(|operation| match operation {
            ComposeOperation::CollectionBlocks => expands_bodies,
            ComposeOperation::ShellExpansion => !expands_bodies,
            _ => true,
        })
        .collect();
        compose.only(&enabled)
    });

    let mut result = md.clone();
    let report = match result.run_root_pipeline_collecting(inline_pre_options, None) {
        (Ok(report), _) => report,
        (Err(error), warnings) => {
            return Err(Box::new(crate::markdown::compose::ComposeFailure {
                error,
                warnings,
                read_set: options.compose.read_set(),
            }));
        }
    };
    Ok(PreparedReferenceContent {
        content: result.content().to_string(),
        document: result,
        body_data: report.body_data,
        body_origin: report.body_origin,
        scopes: report.body_sites.scopes,
        warnings: report.warnings,
        passive_warnings: report.passive_warnings,
    })
}

/// The attribute that marks a reference found in data (text an expression
/// or shell command inserted) rather than in authored text.
pub(crate) const DATA_ORIGIN_ATTRIBUTE: &str = "darkmatter-origin";

/// The attribute that marks a `::toc-linking` record whose fallback chain
/// ends in `false` and found no existing alternative: composition omits the
/// directive, so the record names no required file.
pub(crate) const TOC_SUPPRESSED_ATTRIBUTE: &str = "toc_suppressed";

/// The attribute that marks a `::toc-linking` record whose fallback chain
/// has no `false` and found no alternative composition's selector accepts: a
/// required inclusion failure, whatever its first alternative looks like.
pub(crate) const TOC_UNRESOLVED_ATTRIBUTE: &str = "toc_unresolved";

/// The target a `::file`, `::code`, `prologue`, or `epilogue` value opens,
/// read as composition's resolver reads it: an `http(s)` file reference is a
/// remote URL, and anything else is one local filename. A `#` and what
/// follows it are part of that filename (`./side.md#side` opens the file
/// `side.md#side`), so validation never treats it as a fragment.
fn opened_target(raw: &str) -> ReferenceTarget {
    match biscuit_file::FileReference::new(raw) {
        Ok(reference) if reference.class().kind == biscuit_file::FileReferenceKind::Url => {
            ReferenceTarget::RemoteUrl { raw: raw.into() }
        }
        // A target that does not parse is resolved, and refused, where the
        // graph follows it.
        _ => ReferenceTarget::LocalPath { raw: raw.into() },
    }
}

/// Whether the graph follows `path`, the file a `::file`, `prologue`, or
/// `epilogue` resolved to, as a Markdown child: composition's own admission
/// rule ([`admit_local_target`](crate::markdown::compose::transclusion::admit_local_target)),
/// applied to the canonical path before anything is read, so a plain-text
/// target is composition's unsupported-file-type error at `site`. A
/// reference check running ahead of composition leaves the refusal to it
/// (`false`, no child).
fn admits_markdown_child(
    path: &std::path::Path,
    options: &ReferenceGraphOptions,
    site: Option<InclusionSite>,
) -> MarkdownResult<bool> {
    let canonical = biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf());
    let admitted = crate::markdown::compose::transclusion::admit_local_target(
        DirectiveKind::File,
        &canonical,
        &options.compose.transclusion_options(),
    );
    match admitted {
        Ok(()) => Ok(true),
        Err(_) if options.compose.defer_inclusion_parse_failures => Ok(false),
        Err(error) => Err(error.at_inclusion(site).into()),
    }
}

/// Resolve a local path target through the request-scoped detailed resolver.
fn resolve_local_target(
    raw_target: &str,
    source: &ComposeSource,
    options: &crate::markdown::compose::ComposeRequest,
) -> Result<Option<std::path::PathBuf>, super::ReferenceError> {
    super::resolve_transclusion_target(raw_target, source, options)
}

/// Convert a compose source to a stable, canonicalized node ID.
///
/// File paths are canonicalized to avoid duplicate nodes when the same
/// physical file is referenced via different path spellings (e.g.,
/// `/var/...` vs `/private/var/...` on macOS).
fn source_to_id(source: &ComposeSource) -> NodeId {
    match source {
        ComposeSource::Unknown => NodeId::from("unknown"),
        // `canonicalize_simplified`, not `canonicalize`: a Windows verbatim
        // `\\?\` prefix would leak into every id and rendered graph.
        ComposeSource::File(p) => NodeId::from(
            biscuit_file::canonicalize_simplified(p)
                .unwrap_or_else(|_| p.clone())
                .to_string_lossy()
                .to_string(),
        ),
        ComposeSource::Url(u) => NodeId::from(u.to_string()),
    }
}

/// The hyperlinks a `::toc-linking` directive generates, one per raw
/// heading of `path` its levels select, and every raw heading slug of
/// `path`.
///
/// Like composition, this reads the target's raw source: no replacement,
/// interpolation, or conditional block of the target applies. Fragment
/// validation checks these anchors against the returned slugs, not the
/// target's prepared headings.
fn generate_toc_link_references(
    display_target: &str,
    path: &std::path::Path,
    directive: &toc_linking::TocLinkingDirective,
    source: &ComposeSource,
    runtime: &ReferenceAnalysisRuntime,
    options: &ReferenceGraphOptions,
    site: Option<InclusionSite>,
) -> MarkdownResult<(Vec<ReferenceRecord>, std::sync::Arc<[String]>)> {
    // The headings composition reads (the run's shared TOC-heading cache):
    // the raw text, so a target whose frontmatter does not parse still lists
    // its headings. A target that cannot be read is composition's read
    // failure at `site`, the directive that selected it, unless it vanished
    // after resolving (validation reports it missing) or the reference check
    // defers it to composition.
    let headings = match runtime.cache.load_toc_headings(path) {
        Ok(headings) => headings,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(_) if options.compose.defer_inclusion_parse_failures => Vec::new(),
        Err(error) => return Err(read_failure(MarkdownError::FileLoad(error), path, site)),
    };
    let raw_slugs: std::sync::Arc<[String]> = headings.iter().map(|h| h.slug.to_lowercase()).collect();

    let records = headings
        .iter()
        .filter(|h| directive.options.levels.includes(h.level))
        .enumerate()
        .map(|(idx, h)| {
            let fragment_url = format!("{display_target}#{}", h.slug);
            ReferenceRecord {
                id: make_reference_id(source, directive.line, directive.span.start + idx + 1),
                kind: ReferenceKind::Hyperlink,
                target: classify_target(&fragment_url),
                origin: ReferenceOrigin {
                    source: source.clone(),
                    line: directive.line,
                    span: directive.span.clone(),
                    syntax: ReferenceSyntax::MarkdownLink,
                    authored: None,
                    section: None,
                },
                attributes: serde_json::Map::new(),
            }
        })
        .collect();
    Ok((records, raw_slugs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::{ComposeContext, ComposeOptions};
    use crate::markdown::reference::ReferenceGraphMismatchKind;
    use crate::markdown::reference::validate::ReferenceValidationOptions;

    fn graph_options() -> ReferenceGraphOptions {
        ReferenceGraphOptions::with_compose(&crate::markdown::compose::test_request(
            ComposeOptions::new(),
        ))
    }

    /// Graph options whose request is anchored at the fixture directory the
    /// documents live in; a graph refuses a document outside its request's
    /// repository.
    fn graph_options_at(dir: &std::path::Path) -> ReferenceGraphOptions {
        ReferenceGraphOptions::with_compose(&crate::markdown::compose::test_request_in(
            ComposeOptions::new(),
            biscuit_file::FileResolutionContext::new(dir),
        ))
    }

    #[test]
    fn single_document_graph() {
        let md = Markdown::new("# Hello\n\n[link](./file.md)\n\n![img](./photo.png)");
        let options = graph_options();
        let graph = build_reference_graph(&md, &options).unwrap();

        assert_eq!(graph.node_count(), 1);
        assert!(!graph.root().local_references.is_empty());

        let links = graph.root().local_references.hyperlinks();
        assert_eq!(links.len(), 1);

        let images = graph.root().local_references.images();
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn flatten_single_node() {
        let md = Markdown::new("[a](./a.md)\n[b](./b.md)");
        let options = graph_options();
        let graph = build_reference_graph(&md, &options).unwrap();
        let flat = flatten_graph(&graph);
        assert_eq!(flat.len(), 2);
    }

    #[test]
    fn transclusion_graph_no_references() {
        let md = Markdown::new("# Just text\n\nNo links here.");
        let options = graph_options();
        let graph = build_transclusion_graph(&md, &options).unwrap();

        assert_eq!(graph.node_count(), 1);
        assert!(graph.root().local_references.is_empty());
    }

    #[test]
    fn source_to_id_unknown() {
        assert_eq!(
            source_to_id(&ComposeSource::Unknown),
            NodeId::from("unknown")
        );
    }

    #[test]
    fn when_condition_uses_read_side_functions_against_document_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("child.md"), "# Child\n").unwrap();

        // `when=` references a read-side function rooted at the document's
        // directory. With the resolution context wired in, the condition
        // evaluates true and the child is traversed.
        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "::file child.md when=\"file_exists('child.md')\"\n",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let graph = build_reference_graph(&root_md, &graph_options_at(dir.path())).unwrap();
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    #[serial_test::serial(reference_graph_snapshot)]
    fn when_condition_reuses_request_home_magic_and_package_roots() {
        let request = tempfile::tempdir().unwrap();
        let home = request.path().join("home");
        let magic = request.path().join("magic");
        let package = request.path().join("package");
        for dir in [&home, &magic, &package] {
            std::fs::create_dir_all(dir).unwrap();
        }
        for path in [
            home.join("home.flag"),
            magic.join("magic.flag"),
            package.join("package.flag"),
        ] {
            std::fs::write(path, "ready").unwrap();
        }
        for index in 1..=3 {
            std::fs::write(request.path().join(format!("child-{index}.md")), "# Child\n").unwrap();
        }
        let root_path = request.path().join("root.md");
        std::fs::write(
            &root_path,
            "::file child-1.md when=\"file_exists('~/home.flag')\"\n\
             ::file child-2.md when=\"file_exists('@magic.flag')\"\n\
             ::file child-3.md when=\"file_exists('^package.flag')\"\n",
        )
        .unwrap();
        let snapshot = biscuit_file::FileResolutionContext::from_snapshot(
            request.path(),
            Some(home),
            std::collections::HashMap::new(),
        )
        .with_repository_root(request.path())
        .with_package_area(package)
        .add_magic_path(magic, biscuit_file::PathPosition::Start);
        let prior = std::env::var_os("HOME");
        // SAFETY: the test is serialized while process-global state is changed.
        let options = ReferenceGraphOptions::from_compose_options(
            crate::markdown::compose::test_request_in(ComposeOptions::new(), snapshot),
        );
        let ambient = tempfile::tempdir().unwrap();
        // SAFETY: the test is serialized while process-global state is changed.
        unsafe { std::env::set_var("HOME", ambient.path()) };

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let graph = build_reference_graph(&root_md, &options).unwrap();
        assert_eq!(
            graph.node_count(),
            4,
            "resolved nodes: {:?}",
            graph.nodes().iter().map(|node| &node.source).collect::<Vec<_>>()
        );

        match prior {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    #[test]
    fn when_condition_false_excludes_directive_via_read_side_function() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("child.md"), "# Child\n").unwrap();

        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "::file child.md when=\"file_exists('does-not-exist.md')\"\n",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let graph = build_reference_graph(&root_md, &graph_options_at(dir.path())).unwrap();
        // Condition false → directive skipped, child not traversed.
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn recursive_file_traversal() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create child document with a link
        let child_path = dir.path().join("child.md");
        std::fs::write(
            &child_path,
            "# Child\n\n[child-link](https://child.example.com)",
        )
        .unwrap();

        // Create root document that transcludes the child
        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "[root-link](https://root.example.com)\n\n::file child.md\n\n[after](https://after.example.com)",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = graph_options_at(dir.path());
        let graph = build_reference_graph(&root_md, &options).unwrap();

        // Root + child = 2 nodes
        assert_eq!(graph.node_count(), 2);

        // Root should have links + transclusion record
        let root_links = graph.root().local_references.hyperlinks();
        assert_eq!(root_links.len(), 2); // root-link + after
        let root_transclusions = graph.root().local_references.transclusions();
        assert_eq!(root_transclusions.len(), 1);

        // Child node should have its link
        let child_node = &graph.nodes()[0];
        let child_links = child_node.local_references.hyperlinks();
        assert_eq!(child_links.len(), 1);

        // Flattened should contain all references in composed order
        let flat = flatten_graph(&graph);
        assert!(flat.len() >= 4); // root-link, transclusion, child-link, after
    }

    #[test]
    fn accepted_external_child_keeps_authoring_provenance_across_surfaces() {
        let request = tempfile::tempdir().unwrap();
        let repository = request.path().join("repository");
        let external = request.path().join("external-magic");
        std::fs::create_dir_all(&repository).unwrap();
        std::fs::create_dir_all(&external).unwrap();

        let root_path = repository.join("root.md");
        let child_path = external.join("child.md");
        let grandchild_path = external.join("grandchild.md");
        std::fs::write(&root_path, "# Root\n\n::file @child.md\n").unwrap();
        std::fs::write(
            &child_path,
            "# External child\n\n::file ./grandchild.md\n",
        )
        .unwrap();
        std::fs::write(&grandchild_path, "# External grandchild\n").unwrap();

        let snapshot = biscuit_file::FileResolutionContext::from_snapshot(
            &repository,
            None,
            std::collections::HashMap::new(),
        )
        .with_repository_root(&repository)
        .add_magic_path(&external, biscuit_file::PathPosition::Start);
        let compose = crate::markdown::compose::test_request_in(
            ComposeOptions::new().with_source_file(&root_path),
            snapshot,
        );
        let graph_options = ReferenceGraphOptions::from_compose_options(compose.clone());
        let root_md = Markdown::try_from(root_path.as_path()).unwrap();

        let (composed, _) = root_md.compose_with(&compose).unwrap();
        assert!(composed.content().contains("External child"));
        assert!(composed.content().contains("External grandchild"));

        let references = root_md.composed_references(graph_options.clone()).unwrap();
        assert!(references.records.iter().any(|record| {
            record.origin.source == ComposeSource::File(child_path.clone())
                && record.target.raw() == Some("./grandchild.md")
        }));

        let graph = root_md.reference_graph(graph_options.clone()).unwrap();
        let sources: Vec<_> = std::iter::once(graph.root())
            .chain(graph.nodes().iter())
            .map(|node| node.source.clone())
            .collect();
        assert!(sources.contains(&ComposeSource::File(root_path.clone())));
        assert!(sources.contains(&ComposeSource::File(child_path.clone())));
        assert!(sources.contains(&ComposeSource::File(grandchild_path.clone())));

        let validation = root_md
            .validate_references(super::super::validate::ReferenceValidationOptions::with_graph(
                graph_options,
            ))
            .unwrap();
        assert!(validation.is_valid(), "issues: {:?}", validation.issues);

        // Queried as a root of its own, the child is admitted the way a
        // request admits its own source, so its sibling resolves there.
        let child_md = Markdown::try_from(child_path.as_path()).unwrap();
        let refs = child_md.transclusions_with_options(&compose).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0].resolved_target.as_deref().map(std::path::Path::new),
            Some(grandchild_path.as_path()),
        );
    }

    #[test]
    fn prologue_and_epilogue_traversal() {
        let dir = tempfile::TempDir::new().unwrap();

        let prologue_path = dir.path().join("prologue.md");
        std::fs::write(
            &prologue_path,
            "[prologue-link](https://prologue.example.com)",
        )
        .unwrap();

        let epilogue_path = dir.path().join("epilogue.md");
        std::fs::write(
            &epilogue_path,
            "[epilogue-link](https://epilogue.example.com)",
        )
        .unwrap();

        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\nprologue: prologue.md\nepilogue: epilogue.md\n---\n\n[main](https://main.example.com)",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = graph_options_at(dir.path());
        let graph = build_reference_graph(&root_md, &options).unwrap();

        // Root + prologue + epilogue = 3 nodes
        assert_eq!(graph.node_count(), 3);

        // Root should have transclusion records for prologue and epilogue
        let transclusions = graph.root().local_references.transclusions();
        assert_eq!(transclusions.len(), 2);
    }

    #[test]
    fn cycle_detection() {
        let dir = tempfile::TempDir::new().unwrap();

        let a_path = dir.path().join("a.md");
        let b_path = dir.path().join("b.md");
        std::fs::write(&a_path, "::file b.md").unwrap();
        std::fs::write(&b_path, "::file a.md").unwrap();

        let root_md = Markdown::try_from(a_path.as_path()).unwrap();
        let options = graph_options_at(dir.path());
        // Should not infinite loop — cycle detection stops recursion
        let graph = build_reference_graph(&root_md, &options).unwrap();

        // A two-node cycle should produce exactly 2 unique nodes
        assert_eq!(
            graph.node_count(),
            2,
            "two-node cycle should produce exactly 2 nodes"
        );

        // Verify no duplicate node IDs
        let mut ids: Vec<&str> = vec![graph.root().node_id.as_ref()];
        ids.extend(graph.nodes().iter().map(|n| n.node_id.as_ref()));
        let unique: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len(), "all node IDs should be unique");
    }

    /// Copies of one file prepared under different values are distinct
    /// nodes, each insertion selects its own, and an equal copy keeps the
    /// first copy's id; Mermaid and DOT draw an edge to each distinct copy.
    #[test]
    fn differing_copies_of_one_file_are_distinct_nodes() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("leaf.md"), "[Link](./{{ word }}.md)\n").unwrap();
        let root = dir.path().join("root.md");
        std::fs::write(
            &root,
            "::file ./leaf.md set.word=\"a\"\n\n::file ./leaf.md set.word=\"b\"\n\n::file ./leaf.md set.word=\"a\"\n",
        )
        .unwrap();
        let graph = build_reference_graph(&Markdown::try_from(root.as_path()).unwrap(), &graph_options_at(dir.path())).unwrap();

        let ids: Vec<&str> = graph.root().child_insertions.iter().map(|ins| ins.child_node_id.as_ref()).collect();
        let leaf = source_to_id(&ComposeSource::File(dir.path().join("leaf.md")));
        let second = format!("{leaf} (2)");
        assert_eq!(ids, [leaf.as_ref(), second.as_str(), leaf.as_ref()]);
        let links: Vec<String> = ids
            .iter()
            .map(|id| graph.node_by_id(id).unwrap().local_references.hyperlinks()[0].target.raw().unwrap().to_string())
            .collect();
        assert_eq!(links, ["./a.md", "./b.md", "./a.md"]);

        let distinct_targets = |diagram: &str, arrow: &str| {
            let targets: std::collections::HashSet<String> =
                diagram.lines().filter_map(|line| line.split_once(arrow)).map(|(_, to)| to.to_string()).collect();
            targets.len()
        };
        let mermaid = graph.to_mermaid();
        assert_eq!(distinct_targets(&mermaid, " --> "), 2, "{mermaid}");
        assert!(mermaid.contains("leaf.md (2)<br/>1 refs"), "{mermaid}");
        let dot = graph.to_dot();
        assert_eq!(distinct_targets(&dot, " -> "), 2, "{dot}");
    }

    #[test]
    fn cycle_composed_references_are_finite() {
        let dir = tempfile::TempDir::new().unwrap();

        let a_path = dir.path().join("a.md");
        let b_path = dir.path().join("b.md");
        std::fs::write(&a_path, "[a-link](https://a.example.com)\n\n::file b.md").unwrap();
        std::fs::write(&b_path, "[b-link](https://b.example.com)\n\n::file a.md").unwrap();

        let root_md = Markdown::try_from(a_path.as_path()).unwrap();
        let options = graph_options_at(dir.path());
        let graph = build_reference_graph(&root_md, &options).unwrap();
        let flat = flatten_graph(&graph);

        // composed_references should be finite and non-duplicative
        assert!(
            flat.len() <= 10,
            "flattened refs should be bounded, got {}",
            flat.len()
        );
    }

    #[test]
    fn transclusion_records_emitted() {
        let md = Markdown::new("::file child.md\n::code example.rs\n::url https://example.com");
        let options = graph_options();
        let graph = build_reference_graph(&md, &options).unwrap();

        let transclusions = graph.root().local_references.transclusions();
        assert_eq!(transclusions.len(), 3);

        // Verify syntax types
        let syntaxes: Vec<_> = transclusions.iter().map(|t| t.origin.syntax).collect();
        assert!(syntaxes.contains(&ReferenceSyntax::DirectiveFile));
        assert!(syntaxes.contains(&ReferenceSyntax::DirectiveCode));
        assert!(syntaxes.contains(&ReferenceSyntax::DirectiveUrl));
    }

    #[test]
    fn multiple_insertions_same_line() {
        // Simulate multiple child_insertions on directive_line = 0 (e.g., two prologues)
        let node = ReferenceGraphNode {
            node_id: "root".into(),
            source: ComposeSource::Unknown,
            local_references: ReferenceSet::default(),
            child_insertions: vec![
                ReferenceInsertion {
                    child_node_id: "child_a".into(),
                    directive_line: 0,
                    insertion_order: 0,
                    reference_id: None,
                    context: ReferenceInsertionContext::default(),
                    authored_directive_line: None,
                },
                ReferenceInsertion {
                    child_node_id: "child_b".into(),
                    directive_line: 0,
                    insertion_order: 1,
                    reference_id: None,
                    context: ReferenceInsertionContext::default(),
                    authored_directive_line: None,
                },
            ],
        };

        let child_a = ReferenceGraphNode {
            node_id: "child_a".into(),
            source: ComposeSource::Unknown,
            local_references: ReferenceSet {
                records: vec![ReferenceRecord {
                    id: "a_ref".into(),
                    kind: ReferenceKind::Hyperlink,
                    target: super::super::types::ReferenceTarget::RemoteUrl {
                        raw: "https://a.example.com".into(),
                    },
                    origin: ReferenceOrigin {
                        source: ComposeSource::Unknown,
                        line: 1,
                        span: 0..10,
                        syntax: ReferenceSyntax::MarkdownLink,
                        authored: None,
                        section: None,
                    },
                    attributes: serde_json::Map::new(),
                }],
            },
            child_insertions: vec![],
        };

        let child_b = ReferenceGraphNode {
            node_id: "child_b".into(),
            source: ComposeSource::Unknown,
            local_references: ReferenceSet {
                records: vec![ReferenceRecord {
                    id: "b_ref".into(),
                    kind: ReferenceKind::Hyperlink,
                    target: super::super::types::ReferenceTarget::RemoteUrl {
                        raw: "https://b.example.com".into(),
                    },
                    origin: ReferenceOrigin {
                        source: ComposeSource::Unknown,
                        line: 1,
                        span: 0..10,
                        syntax: ReferenceSyntax::MarkdownLink,
                        authored: None,
                        section: None,
                    },
                    attributes: serde_json::Map::new(),
                }],
            },
            child_insertions: vec![],
        };

        let graph = ReferenceGraph::from_build(
            &Markdown::new(""),
            &graph_options(),
            ReferenceGraphMode::Full,
            node,
            vec![child_a, child_b],
            ReferenceDependencyManifest::default(),
            PreparedHeadingSnapshot::default(),
        );

        let flat = flatten_graph(&graph);
        // Both children should appear in insertion_order (a before b)
        assert_eq!(flat.len(), 2);
        assert_eq!(flat.records[0].id, "a_ref");
        assert_eq!(flat.records[1].id, "b_ref");
    }

    #[test]
    fn when_infix_and_true_follows_transclusion() {
        let dir = tempfile::TempDir::new().unwrap();

        let child_path = dir.path().join("child.md");
        std::fs::write(&child_path, "[link](https://example.com)").unwrap();

        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\nenabled: true\nvisible: true\n---\n\n::file child.md when=\"enabled && visible\"\n",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = graph_options_at(dir.path());
        let graph = build_reference_graph(&root_md, &options).unwrap();

        // Root + child = 2 nodes when the condition is true
        assert_eq!(graph.node_count(), 2);
        let transclusions = graph.root().local_references.transclusions();
        assert_eq!(transclusions.len(), 1);
    }

    #[test]
    fn when_infix_and_false_skips_transclusion() {
        let dir = tempfile::TempDir::new().unwrap();

        let child_path = dir.path().join("child.md");
        std::fs::write(&child_path, "[link](https://example.com)").unwrap();

        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\nenabled: true\nvisible: false\n---\n\n::file child.md when=\"enabled && visible\"\n",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = graph_options_at(dir.path());
        let graph = build_reference_graph(&root_md, &options).unwrap();

        // Child is skipped, so only the root node remains
        assert_eq!(graph.node_count(), 1);
        let transclusions = graph.root().local_references.transclusions();
        assert_eq!(transclusions.len(), 0);
    }

    #[test]
    fn dependency_manifest_records_one_entry_per_unique_local_child() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("child.md"), "# Child\n").unwrap();

        // The same local child is reached through two `::file` insertions; a
        // remote `::url` target is present but must not enter the manifest.
        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "::file child.md\n\n::file child.md\n\n::url https://example.com\n",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let graph = build_reference_graph(&root_md, &graph_options_at(dir.path())).unwrap();

        let manifest = graph.provenance().dependencies();
        assert_eq!(
            manifest.len(),
            1,
            "one child reached twice yields exactly one entry; remote ::url is excluded"
        );
        let recorded = &manifest.documents()[0];
        match &recorded.source {
            ComposeSource::File(p) => assert_eq!(p.file_name().unwrap(), "child.md"),
            other => panic!("expected a local file source, got {other:?}"),
        }
    }

    #[test]
    fn dependency_manifest_covers_prologue_and_epilogue_children() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("prologue.md"), "# Prologue\n").unwrap();
        std::fs::write(dir.path().join("epilogue.md"), "# Epilogue\n").unwrap();

        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\nprologue: prologue.md\nepilogue: epilogue.md\n---\n\n# Body\n",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let graph = build_reference_graph(&root_md, &graph_options_at(dir.path())).unwrap();

        let manifest = graph.provenance().dependencies();
        assert_eq!(
            manifest.len(),
            2,
            "prologue and epilogue children each record one manifest entry"
        );
    }

    #[test]
    fn dependency_manifest_empty_without_local_children() {
        // A remote-only transclusion never materializes a local child.
        let md = Markdown::new("::url https://example.com\n[link](https://example.com)");
        let graph = build_reference_graph(&md, &graph_options()).unwrap();
        assert!(graph.provenance().dependencies().is_empty());
    }

    #[test]
    fn when_infix_or_follows_on_either_true() {
        let dir = tempfile::TempDir::new().unwrap();

        let child_path = dir.path().join("child.md");
        std::fs::write(&child_path, "[link](https://example.com)").unwrap();

        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\nprimary: false\nbackup: true\n---\n\n::file child.md when=\"primary || backup\"\n",
        )
        .unwrap();

        let root_md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = graph_options_at(dir.path());
        let graph = build_reference_graph(&root_md, &options).unwrap();

        assert_eq!(graph.node_count(), 2);
        let transclusions = graph.root().local_references.transclusions();
        assert_eq!(transclusions.len(), 1);
    }

    // ── Volatile-context reuse regressions ─────────────────────────────
    //
    // The provenance unit tests in `provenance.rs` assert that two option
    // identities differ. That is necessary but not sufficient: it never shows
    // the identity delta corresponds to a *real* difference in what the graph
    // describes. These two build real graphs through `Markdown::reference_graph`
    // under two contexts, prove the built graphs differ in contents, and then
    // drive the public `validate_references_with_graph` entry point to show
    // cross-context reuse is rejected while same-context reuse still succeeds.

    /// Unwraps the typed prebuilt-graph mismatch classification, panicking on
    /// any other error shape. Mirrors the public integration-test helper so the
    /// assertion pins the classification rather than message text.
    fn graph_mismatch_kind(err: &crate::markdown::MarkdownError) -> ReferenceGraphMismatchKind {
        match err {
            crate::markdown::MarkdownError::Reference(inner) => match inner.as_ref() {
                crate::markdown::reference::ReferenceError::ReferenceGraphMismatch(mismatch) => {
                    mismatch.kind()
                }
                other => panic!("expected a graph-mismatch reference error, got: {other}"),
            },
            other => panic!("expected a reference error, got: {other}"),
        }
    }

    /// Every hyperlink target recorded on the graph's root node, in order.
    fn root_link_targets(graph: &ReferenceGraph) -> Vec<String> {
        graph
            .root()
            .local_references
            .hyperlinks()
            .iter()
            .map(|record| {
                record
                    .target
                    .raw()
                    .expect("a link target is never inline")
                    .to_string()
            })
            .collect()
    }

    /// A reference target produced by `{{ ctx.* }}` interpolation resolves to a
    /// *different file* under a different context, so the two graphs record
    /// different link targets. Reusing the first graph to validate the second
    /// context's request would report on links the request never contained.
    ///
    /// `timestamp` is the volatile value here specifically because the
    /// compose-cache `context_hash` drops it — this is the case the
    /// complete graph-context fingerprint exists to catch.
    #[test]
    fn prebuilt_graph_rejects_reuse_across_volatile_interpolated_link_target() {
        let dir = tempfile::TempDir::new().unwrap();
        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "# Root\n\n[snapshot](./snapshot-{{ ctx.timestamp }}.md)\n",
        )
        .unwrap();
        let md = Markdown::try_from(root_path.as_path()).unwrap();

        let compose_for = |stamp: &str| {
            crate::markdown::compose::test_request_in(
                ComposeOptions::new_with_context(ComposeContext::fixed_for_testing_with([(
                    "timestamp", serde_json::json!(stamp),
                )])),
                biscuit_file::FileResolutionContext::new(dir.path()),
            )
        };
        let compose_a = compose_for("1000");
        let compose_b = compose_for("2000");

        let graph_a = md
            .reference_graph(ReferenceGraphOptions::from_compose_options(compose_a.clone()))
            .unwrap();
        let graph_b = md
            .reference_graph(ReferenceGraphOptions::from_compose_options(compose_b.clone()))
            .unwrap();

        // The graphs differ in *contents*, not merely in provenance: each root
        // node recorded the interpolated target its own context produced.
        assert_eq!(root_link_targets(&graph_a), ["./snapshot-1000.md"]);
        assert_eq!(root_link_targets(&graph_b), ["./snapshot-2000.md"]);
        assert_ne!(
            root_link_targets(&graph_a),
            root_link_targets(&graph_b),
            "the volatile context must actually change the resolved link target, \
             otherwise this fixture no longer exercises cross-context reuse"
        );

        // Positive control: same-context reuse is accepted, so the rejection
        // below is attributable to the context delta and not to the graph being
        // refused unconditionally.
        let report = md
            .validate_references_with_graph(
                &graph_a,
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::from_compose_options(
                    compose_a,
                )),
            )
            .expect("same-context reuse must be accepted");
        assert_eq!(report.references_scanned, 1);

        // Cross-context reuse is rejected on the options dimension.
        let err = md
            .validate_references_with_graph(
                &graph_a,
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::from_compose_options(
                    compose_b,
                )),
            )
            .unwrap_err();
        assert_eq!(
            graph_mismatch_kind(&err),
            ReferenceGraphMismatchKind::Options
        );
    }

    /// A `when=`-conditional transclusion gated on a volatile context value
    /// materializes a child node under one context and not the other, so the
    /// two graphs differ in node count and in the dependency manifest. Reusing
    /// the child-bearing graph for a request whose context excludes the child
    /// would validate references the request never composed.
    #[test]
    fn prebuilt_graph_rejects_reuse_across_volatile_when_controlled_transclusion() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("child.md"),
            "# Child\n\n[c](https://child.example.com)\n",
        )
        .unwrap();
        let root_path = dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "# Root\n\n::file child.md when=\"ctx.memory_used > 2000\"\n",
        )
        .unwrap();
        let md = Markdown::try_from(root_path.as_path()).unwrap();

        let compose_for = |memory_used: u64| {
            crate::markdown::compose::test_request_in(
                ComposeOptions::new_with_context(ComposeContext::fixed_for_testing_with([(
                    "memory_used", serde_json::json!(memory_used),
                )])),
                biscuit_file::FileResolutionContext::new(dir.path()),
            )
        };
        let compose_excluded = compose_for(1024);
        let compose_included = compose_for(4096);

        let graph_excluded = md
            .reference_graph(ReferenceGraphOptions::from_compose_options(
                compose_excluded.clone(),
            ))
            .unwrap();
        let graph_included = md
            .reference_graph(ReferenceGraphOptions::from_compose_options(
                compose_included.clone(),
            ))
            .unwrap();

        // Contents differ: the condition decided whether the child became a node.
        assert_eq!(
            graph_excluded.node_count(),
            1,
            "condition false → child is not materialized"
        );
        assert_eq!(
            graph_included.node_count(),
            2,
            "condition true → child is materialized"
        );
        assert!(
            graph_excluded.provenance().dependencies().is_empty(),
            "no child was visited, so nothing enters the dependency manifest"
        );
        assert_eq!(
            graph_included.provenance().dependencies().len(),
            1,
            "the visited child records exactly one dependency entry"
        );

        // Positive control: same-context reuse is accepted for both graphs.
        for (label, graph, compose) in [
            ("excluded", &graph_excluded, compose_excluded.clone()),
            ("included", &graph_included, compose_included.clone()),
        ] {
            md.validate_references_with_graph(
                graph,
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::from_compose_options(
                    compose,
                )),
            )
            .unwrap_or_else(|e| panic!("same-context {label} reuse must be accepted: {e}"));
        }

        // Cross-context reuse is rejected on the options dimension in both
        // directions: neither graph describes the other context's document.
        let err = md
            .validate_references_with_graph(
                &graph_included,
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::from_compose_options(
                    compose_excluded,
                )),
            )
            .unwrap_err();
        assert_eq!(
            graph_mismatch_kind(&err),
            ReferenceGraphMismatchKind::Options
        );

        let err = md
            .validate_references_with_graph(
                &graph_excluded,
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::from_compose_options(
                    compose_included,
                )),
            )
            .unwrap_err();
        assert_eq!(
            graph_mismatch_kind(&err),
            ReferenceGraphMismatchKind::Options
        );
    }

    // ── Graph non-retention of stateful runtimes (AC6, invariant 7) ─────
    //
    // `prebuilt_graph_rejects_recreated_shared_remote_fetch` and the
    // preflight fresh-instance rejection prove only that a *different* live
    // instance mismatches — which is true whether the graph held the original
    // strongly or held only a `Weak` (a distinct allocation mismatches either
    // way). These two tests own the stateful instance externally so the strong
    // count is directly observable, then build and drop a *real* graph to prove
    // the graph captures only a `Weak` and never extends the instance lifetime.

    /// Graph construction captures only a `Weak` identity handle for the shared
    /// remote-fetch runtime, so building — and dropping — a real graph never
    /// raises the runtime's strong count, and the runtime is released the
    /// instant the last *external* strong handle is dropped.
    #[test]
    fn graph_build_does_not_retain_remote_fetch_runtime() {
        use crate::markdown::compose::remote_fetch::RemoteFetchRuntime;
        use biscuit_file::file_reference::fetch::FetchPolicy;

        let dir = tempfile::TempDir::new().unwrap();
        let root_path = dir.path().join("root.md");
        std::fs::write(&root_path, "# Root\n\n[ok](https://example.com)\n").unwrap();
        let md = Markdown::try_from(root_path.as_path()).unwrap();

        // Own the runtime externally so its strong count is observable; the
        // public `with_shared_remote_fetch()` builder hides the `Arc` inside the
        // options where a graph-level test cannot reach it.
        let runtime = RemoteFetchRuntime::with_policy(FetchPolicy::deny_all());
        let weak = runtime.weak_id();
        let strong_alone = runtime.strong_count();
        assert_eq!(strong_alone, 1, "the external handle is the only owner");

        let mut compose = ComposeOptions::new();
        compose.remote_fetch = Some(runtime.clone());
        let compose = crate::markdown::compose::test_request(compose);
        assert_eq!(
            runtime.strong_count(),
            strong_alone + 1,
            "the build options hold the second strong reference"
        );

        let graph = md
            .reference_graph(ReferenceGraphOptions::from_compose_options(compose))
            .unwrap();
        // Building the graph moved and dropped the build options, and provenance
        // captured only a `Weak`: the runtime is back to a single owner.
        assert_eq!(
            runtime.strong_count(),
            strong_alone,
            "graph construction must not retain a strong reference to the runtime"
        );

        drop(graph);
        assert_eq!(
            runtime.strong_count(),
            strong_alone,
            "dropping the graph leaves the runtime's strong count at baseline"
        );

        // Drop-probe: releasing the last external strong handle actually frees
        // the runtime, proving nothing (least of all the already-dropped graph)
        // pinned it alive.
        assert!(weak.is_alive(), "the external handle still keeps it alive");
        drop(runtime);
        assert!(
            !weak.is_alive(),
            "with no strong owner left the runtime is dropped: the graph held only a Weak"
        );
    }

    /// The same graph-level ownership proof for the shared preflight graph: an
    /// externally owned `Arc<PreflightGraphNode>` makes the strong count
    /// observable across a real graph build and drop, so non-retention is
    /// proven rather than inferred from a fresh-instance rejection.
    #[test]
    fn graph_build_does_not_retain_preflight_graph() {
        use crate::markdown::compose::preflight::PreflightGraphNode;
        use std::sync::Arc;

        let dir = tempfile::TempDir::new().unwrap();
        let root_path = dir.path().join("root.md");
        std::fs::write(&root_path, "# Root\n\n[ok](https://example.com)\n").unwrap();
        let md = Markdown::try_from(root_path.as_path()).unwrap();

        // Own the preflight `Arc` externally; the public `with_preflight_graph`
        // builder moves the node in and wraps it in a private `Arc`.
        let preflight = Arc::new(PreflightGraphNode::default());
        let strong_alone = Arc::strong_count(&preflight);

        let mut compose = ComposeOptions::new();
        compose.preflight_graph = Some(Arc::clone(&preflight));
        let compose = crate::markdown::compose::test_request(compose);
        assert_eq!(
            Arc::strong_count(&preflight),
            strong_alone + 1,
            "the build options hold one additional strong reference"
        );

        let graph = md
            .reference_graph(ReferenceGraphOptions::from_compose_options(compose))
            .unwrap();
        // Graph construction consumed and dropped the build options and captured
        // only a `Weak`, so the count is back to the external handle alone.
        assert_eq!(
            Arc::strong_count(&preflight),
            strong_alone,
            "graph construction must not retain a strong reference to the preflight graph"
        );

        drop(graph);
        assert_eq!(
            Arc::strong_count(&preflight),
            strong_alone,
            "dropping the graph leaves the preflight strong count at baseline"
        );

        // With the graph gone, the external handle is the sole owner; dropping
        // it frees the node, proving the graph never pinned it.
        let weak = Arc::downgrade(&preflight);
        drop(preflight);
        assert!(
            weak.upgrade().is_none(),
            "with no strong owner left the preflight node is dropped: the graph held only a Weak"
        );
    }

    // ── `::file … interpolate=false` ────────────────────────────────

    /// A child whose prose holds interpolation examples, a malformed
    /// expression, and links built from examples.
    const PROSE_EXAMPLES_CHILD: &str = "# Child\n\nSee [the {{ name }} page](./{{name}}.md) and [guide](./guide.md).\nUnknown {{ undeclared }}. Malformed {{ 1 + }}.\n";

    /// Builds the graph of `root_text` (written as `root.md` beside
    /// `PROSE_EXAMPLES_CHILD` and `guide.md`) under a deny-all policy.
    fn prose_example_graph(
        dir: &std::path::Path,
        root_text: &str,
    ) -> (Markdown, ReferenceGraphOptions, MarkdownResult<ReferenceGraph>) {
        std::fs::write(dir.join("child.md"), PROSE_EXAMPLES_CHILD).unwrap();
        std::fs::write(dir.join("guide.md"), "# Guide\n").unwrap();
        let root_path = dir.join("root.md");
        std::fs::write(&root_path, root_text).unwrap();
        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ReferenceGraphOptions::with_compose(&crate::markdown::compose::test_request_in(
            ComposeOptions::new()
                .with_source_file(&root_path)
                .with_blocking_policy(crate::markdown::compose::policy::BlockingPolicy::default().deny_all()),
            biscuit_file::FileResolutionContext::new(dir),
        ));
        let graph = build_reference_graph(&md, &options);
        (md, options, graph)
    }

    fn child_link_targets(graph: &ReferenceGraph) -> Vec<String> {
        let child = graph
            .nodes()
            .iter()
            .find(|node| matches!(&node.source, ComposeSource::File(path) if path.ends_with("child.md")))
            .expect("the child is a graph node");
        child
            .local_references
            .hyperlinks()
            .iter()
            .map(|record| record.target.raw().expect("a link target is never inline").to_string())
            .collect()
    }

    /// Preparation composes an `interpolate=false` child under the setting
    /// composition gives it: its prose (link targets included) stays as
    /// written, its examples raise no warning under a deny-all policy, and
    /// its links are still references.
    #[test]
    fn a_disabled_child_prepares_its_prose_literally_and_keeps_its_links() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_, _, graph) =
            prose_example_graph(dir.path(), "---\nname: Caller\n---\n::file ./child.md interpolate=false\n");
        let graph = graph.expect("suppressed prose cannot fail preparation");

        assert_eq!(graph.node_count(), 2);
        assert_eq!(child_link_targets(&graph), ["./{{name}}.md", "./guide.md"]);
        assert!(graph.earlier_warnings().is_empty(), "{:?}", graph.earlier_warnings());
    }

    /// Links in suppressed prose are still validated: a missing target is a
    /// located issue that names the inclusion it came through.
    #[test]
    fn links_in_suppressed_prose_are_still_validated() {
        let dir = tempfile::TempDir::new().unwrap();
        // A destination spelled `{{VAR}}` is biscuit-file variable syntax, so
        // this case keeps its interpolation examples in the label.
        let (md, options, _) = prose_example_graph(dir.path(), "::file ./child.md interpolate=false\n");
        std::fs::write(
            dir.path().join("child.md"),
            "See [the {{ name }} page](./gone.md) and [{{ 1 + }}](./guide.md).\n",
        )
        .unwrap();
        let report = md.validate_references(ReferenceValidationOptions::with_graph(options)).expect("validation runs");
        let broken: Vec<(&str, usize)> = report
            .issues
            .iter()
            .map(|issue| (issue.reference_display.as_str(), issue.included_via.len()))
            .collect();
        assert_eq!(broken, [("./gone.md", 1)], "{:?}", report.issues);
        assert_eq!(report.references_valid, 2, "the inclusion and `./guide.md` are valid");
    }

    /// Control: the same child included normally evaluates its prose, so the
    /// malformed example fails preparation.
    #[test]
    fn an_enabled_child_still_evaluates_its_prose_in_preparation() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_, _, graph) = prose_example_graph(dir.path(), "---\nname: Caller\n---\n::file ./child.md\n");
        assert!(graph.is_err(), "the malformed example is evaluated without the option");
    }

    /// The setting is inherited through a nested child (even one that writes
    /// `interpolate=true`) and a prologue child, and never reaches a sibling.
    #[test]
    fn the_disabled_setting_reaches_descendants_but_not_siblings() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("leaf.md"), "[leaf {{ who }}](./leaf-{{who}}.md)\n").unwrap();
        std::fs::write(dir.path().join("intro.md"), "[intro](./intro-{{who}}.md)\n").unwrap();
        std::fs::write(
            dir.path().join("middle.md"),
            "---\nwho: Ann\nprologue: ./intro.md\n---\n::file ./leaf.md interpolate=true\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("sibling.md"), "---\nwho: Bo\n---\n[sib](./sib-{{who}}.md)\n").unwrap();
        let root_path = dir.path().join("root.md");
        std::fs::write(&root_path, "::file ./middle.md interpolate=false\n\n::file ./sibling.md\n").unwrap();
        let md = Markdown::try_from(root_path.as_path()).unwrap();

        let graph = build_reference_graph(&md, &graph_options_at(dir.path())).unwrap();
        let mut targets: Vec<String> = graph
            .nodes()
            .iter()
            .flat_map(|node| node.local_references.hyperlinks())
            .map(|record| record.target.raw().unwrap().to_string())
            .collect();
        targets.sort();
        assert_eq!(targets, ["./intro-{{who}}.md", "./leaf-{{who}}.md", "./sib-Bo.md"]);
    }

    /// Two requests differing only in the setting have different graph
    /// options identities, so a graph built under one is never reused for
    /// the other.
    #[test]
    fn a_graph_built_with_prose_disabled_is_not_reused_with_it_enabled() {
        use crate::markdown::compose::context::options::ProseInterpolation;

        let dir = tempfile::TempDir::new().unwrap();
        let root_path = dir.path().join("root.md");
        std::fs::write(&root_path, "[a](./a.md)\n").unwrap();
        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let enabled = graph_options_at(dir.path());
        let disabled = ReferenceGraphOptions::from_compose_options(
            enabled.compose.clone().derive(|compose| compose.with_prose_interpolation(ProseInterpolation::DisabledByInclusion)),
        );

        let graph = md.reference_graph(disabled.clone()).unwrap();
        md.validate_references_with_graph(&graph, ReferenceValidationOptions::with_graph(disabled))
            .expect("same-setting reuse is accepted");
        let err = md
            .validate_references_with_graph(&graph, ReferenceValidationOptions::with_graph(enabled))
            .unwrap_err();
        assert_eq!(graph_mismatch_kind(&err), ReferenceGraphMismatchKind::Options);
    }
}
