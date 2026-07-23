//! Reference graph traversal and flattening.
//!
//! Builds a graph of documents connected by transclusion directives,
//! extracts references at each node, and flattens into composed order.

use std::collections::BTreeMap;

use super::provenance::{
    ReferenceDependencyManifest, ReferenceDocumentIdentity, ReferenceGraphMode,
};
use super::snapshot::PreparedHeadingSnapshot;
use super::types::{
    NodeId, ReferenceGraph, ReferenceGraphNode, ReferenceGraphOptions, ReferenceInsertion,
    ReferenceInsertionContext, ReferenceKind, ReferenceOrigin, ReferenceRecord, ReferenceSet,
    ReferenceSyntax, classify_target, make_reference_id,
};
use crate::markdown::Markdown;
use crate::markdown::compose::cache::RunLocalCache;
use crate::markdown::compose::conditions;
use crate::markdown::compose::toc_linking;
use biscuit_terminal::errors::SourceContext;

use crate::markdown::compose::transclusion::{
    DirectiveKind, TransclusionRuntime, parse_directives, parse_frontmatter_refs,
};
use crate::markdown::compose::{
    ComposeOperation, ComposeOptions, ComposeSource, EffectiveStateBuilder, ResolvingLookup,
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
}

fn source_to_path(source: &ComposeSource) -> std::path::PathBuf {
    match source {
        ComposeSource::File(path) => path.clone(),
        ComposeSource::Url(url) => std::path::PathBuf::from(url.to_string()),
        ComposeSource::Unknown => std::path::PathBuf::from("<unknown>"),
    }
}

impl ReferenceAnalysisRuntime {
    /// Load a markdown document, using the shared cache.
    fn load_markdown(&self, path: &std::path::Path) -> Option<Markdown> {
        self.cache.load_markdown(path).ok()
    }
}

/// Construct a [`RunLocalCache`] from graph options, attaching persistent
/// backing when `cache_root` is configured.
///
/// Uses [`FileStore::resolve_cache_root`] to match the compose pipeline's
/// cache-path resolution, honoring `cache_namespace` for branch/profile
/// isolation.
fn make_cache(options: &ReferenceGraphOptions) -> RunLocalCache {
    use crate::markdown::compose::cache::FileStore;

    let cache = RunLocalCache::new(options.compose.cache_access_mode);
    if let Some(ref root) = options.compose.cache_root {
        let resolved =
            FileStore::resolve_cache_root(Some(root), options.compose.cache_namespace.as_deref());
        cache.with_persistent(resolved)
    } else {
        cache
    }
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

    let source = md.source().clone().unwrap_or(ComposeSource::Unknown);
    let compose = super::options_with_reference_resolution_context(&source, &options.compose);
    let compose = match &source {
        ComposeSource::File(path) => compose.with_source_file(path),
        ComposeSource::Url(url) => compose.with_source_url(url.clone()),
        ComposeSource::Unknown => compose,
    };
    let options = ReferenceGraphOptions::with_compose(compose);
    let options = &options;
    let mut runtime = ReferenceAnalysisRuntime {
        transclusion: TransclusionRuntime::new(options.compose.max_transclusion_depth),
        cache: make_cache(options),
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
    )?;

    runtime.transclusion.exit();

    Ok(ReferenceGraph::from_build(
        md,
        options,
        mode,
        root,
        all_nodes,
        dependencies,
        prepared_headings,
    ))
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
    let mut records = Vec::new();
    flatten_node(graph.root(), graph, &mut records);
    ReferenceSet { records }
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
fn flatten_node(node: &ReferenceGraphNode, graph: &ReferenceGraph, out: &mut Vec<ReferenceRecord>) {
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
                    for insertion in insertions {
                        if let Some(child_node) = graph.node_by_id(insertion.child_node_id.as_ref())
                        {
                            flatten_node(child_node, graph, out);
                        }
                    }
                }
            }
        }

        // Emit this reference
        out.push(record.clone());

        // If a child insertion exists at the same line as this reference,
        // emit it immediately after (this is the transclusion point).
        if emitted_insertion_lines.insert(record.origin.line)
            && let Some(insertions) = insertion_map.get(&record.origin.line)
        {
            for insertion in insertions {
                if let Some(child_node) = graph.node_by_id(insertion.child_node_id.as_ref()) {
                    flatten_node(child_node, graph, out);
                }
            }
        }

        last_line = record.origin.line.saturating_add(1);
    }

    // Flush remaining child insertions after all local references
    for (&line, insertions) in insertion_map.range(last_line..) {
        if emitted_insertion_lines.insert(line) {
            for insertion in insertions {
                if let Some(child_node) = graph.node_by_id(insertion.child_node_id.as_ref()) {
                    flatten_node(child_node, graph, out);
                }
            }
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

fn accepted_child_options(
    options: &ReferenceGraphOptions,
    path: &std::path::Path,
) -> ReferenceGraphOptions {
    ReferenceGraphOptions::with_compose(
        options
            .compose
            .clone()
            .with_accepted_source_file(path),
    )
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
    let node_id = source_to_id(source);

    // Always run InlinePre preparation (rec #2): page blocks, interpolation,
    // shell expansion, and text replacement can affect references even in
    // leaf documents with no transclusions.
    let prepared_content = prepare_content(md, options)?;

    let ctx = {
        let base = md.source_context_for_errors();
        SourceContext::new(base.absolute, base.display, prepared_content.clone())
    };

    // Build heading index for section context on transclusion insertions
    let (heading_index, prepared_slugs) = build_heading_index(&prepared_content);

    if let ComposeSource::File(path) = source {
        prepared_headings.record(path, prepared_slugs);
    }

    // Extract local references if requested
    let mut local_references = if extract_references {
        extract_all_references(&prepared_content, source)
    } else {
        ReferenceSet::default()
    };

    // Parse transclusion directives and build child nodes
    let mut child_insertions = Vec::new();
    let mut all_descendant_nodes: Vec<ReferenceGraphNode> = Vec::new();
    let mut insertion_order = 0;

    // Build effective state for evaluating `when=` conditions.
    // Uses the document's frontmatter merged with any external state from
    // options, plus runtime context (including environment variables).
    let effective_state = {
        let fm: std::collections::HashMap<String, serde_json::Value> = md
            .frontmatter()
            .as_map()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut builder = EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(options.compose.context().clone());
        if let Some(ref ext) = options.compose.external_state {
            builder = builder.with_external_state(ext.clone());
        }
        builder.build().unwrap_or_else(|_| {
            // Context merge failure during reference analysis: fall back to
            // a state without user ctx so reference extraction can proceed.
            EffectiveStateBuilder::new()
                .with_context(options.compose.context().clone())
                .build()
                .expect("empty frontmatter has no user ctx")
        })
    };

    let when_options = options.compose.clone();
    let when_lookup = ResolvingLookup::new(
        &effective_state,
        when_options.local_expression_resolution_context(),
    );

    // Block directives
    if let Ok(directives) = parse_directives(&prepared_content, ctx.clone()) {
        for directive in &directives {
            // Evaluate `when=` condition — skip directive entirely if false.
            if let Some(ref when_expr) = directive.options.when_expr {
                let condition_met = conditions::evaluate_condition(
                    when_expr,
                    &when_lookup,
                    directive.line,
                    ctx.clone(),
                )
                .unwrap_or(false);
                if !condition_met {
                    continue;
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
                    target: classify_target(&directive.raw_target),
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line: directive.line,
                        span: directive.span.clone(),
                        syntax,
                    },
                    attributes: serde_json::Map::new(),
                });
            }

            if directive.kind == DirectiveKind::File {
                // Try to resolve and recurse into the child
                if let Some(child_path) = resolve_local_target(
                    &directive.raw_target,
                    source,
                    &options.compose,
                )? {
                    let child_source = ComposeSource::File(child_path.clone());
                    let child_id = source_to_id(&child_source);

                    // Cycle/depth check
                    if runtime
                        .transclusion
                        .enter(child_id.to_string(), child_path.clone(), directive.line)
                        .is_ok()
                    {
                        if let Some(child_md) = runtime.load_markdown(&child_path) {
                            // Record this visited local child from the value we
                            // just loaded (deduped per unique resolved source).
                            dependencies.record(
                                child_source.clone(),
                                ReferenceDocumentIdentity::capture(&child_md),
                            );
                            let (child_node, mut descendants) = build_node(
                                &child_md,
                                &child_source,
                                &accepted_child_options(options, &child_path),
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
    }

    // ::toc-linking directives generate synthesized TOC hyperlinks AND
    // create child graph nodes for follow-mode expansion.
    if let Ok(toc_directives) =
        crate::markdown::compose::toc_linking::parse_directives(&prepared_content)
    {
        let transclusion_options = options.compose.transclusion_options();

        for directive in &toc_directives {
            let ref_id = make_reference_id(source, directive.line, directive.span.start);

            // Section context is always available from the parent document's
            // heading index, regardless of whether the target can be loaded.
            let (sec_text, sec_level) = section_at_line(&heading_index, directive.line)
                .map(|(t, l)| (Some(t.to_string()), Some(l)))
                .unwrap_or((None, None));

            if let Some((display_target, path)) =
                resolve_toc_linking_target(directive, source, &transclusion_options, ctx.clone())
            {
                if extract_references {
                    local_references.records.push(ReferenceRecord {
                        id: ref_id.clone(),
                        kind: ReferenceKind::Transclusion,
                        target: classify_target(&display_target),
                        origin: ReferenceOrigin {
                            source: source.clone(),
                            line: directive.line,
                            span: directive.span.clone(),
                            syntax: ReferenceSyntax::DirectiveTocLinking,
                        },
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
                if extract_references {
                    for mut record in
                        generate_toc_link_references(&display_target, &path, directive, source, runtime)
                    {
                        record
                            .attributes
                            .insert("toc_synthesized".to_string(), serde_json::Value::Bool(true));
                        local_references.records.push(record);
                    }
                }

                if runtime
                    .transclusion
                    .enter(child_id.to_string(), path.clone(), directive.line)
                    .is_ok()
                {
                    if let Some(child_md) = runtime.load_markdown(&path) {
                        dependencies.record(
                            child_source.clone(),
                            ReferenceDocumentIdentity::capture(&child_md),
                        );
                        let (child_node, mut descendants) = build_node(
                            &child_md,
                            &child_source,
                            &accepted_child_options(options, &path),
                            runtime,
                            extract_references,
                            dependencies,
                            prepared_headings,
                        )?;

                        child_insertions.push(ReferenceInsertion {
                            child_node_id: child_node.node_id.clone(),
                            directive_line: directive.line,
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
                // a context-only insertion for the caption.
                if extract_references && let Some(raw_target) = directive.targets.first() {
                    local_references.records.push(ReferenceRecord {
                        id: ref_id.clone(),
                        kind: ReferenceKind::Transclusion,
                        target: classify_target(raw_target),
                        origin: ReferenceOrigin {
                            source: source.clone(),
                            line: directive.line,
                            span: directive.span.clone(),
                            syntax: ReferenceSyntax::DirectiveTocLinking,
                        },
                        attributes: serde_json::Map::new(),
                    });
                }

                child_insertions.push(ReferenceInsertion {
                    child_node_id: NodeId::from(""),
                    directive_line: directive.line,
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
    }

    // ::file-links directives emit reference records but do not create
    // child insertions (they are not followable transclusions).
    if let Ok(file_links_directives) =
        crate::markdown::compose::file_links::parse_file_links_directives(&prepared_content)
    {
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
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line: directive.line,
                        span: directive.span.clone(),
                        syntax: ReferenceSyntax::DirectiveFileLinks,
                    },
                    attributes: serde_json::Map::new(),
                });
            }
        }
    }

    // Frontmatter prologue/epilogue
    if let Ok(fm_refs) = parse_frontmatter_refs(md.frontmatter().as_map(), ctx.clone()) {
        for (idx, prologue) in fm_refs.prologue.iter().enumerate() {
            let is_literal = is_literal_content(prologue);

            if extract_references {
                let mut attrs = serde_json::Map::new();
                if is_literal {
                    attrs.insert("fm_literal".to_string(), serde_json::Value::Bool(true));
                    // Extract links from literal content for follow mode
                    let links = super::local::extract_markdown_links(prologue, source);
                    for mut link in links {
                        link.attributes
                            .insert("toc_synthesized".to_string(), serde_json::Value::Bool(true));
                        local_references.records.push(link);
                    }
                }
                local_references.records.push(ReferenceRecord {
                    id: make_reference_id(source, 0, idx),
                    kind: ReferenceKind::Transclusion,
                    target: if is_literal {
                        classify_target("")
                    } else {
                        classify_target(prologue)
                    },
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line: 0,
                        span: 0..0,
                        syntax: ReferenceSyntax::FrontmatterPrologue,
                    },
                    attributes: attrs,
                });
            }

            if !is_literal
                && let Some(child_path) =
                    resolve_local_target(prologue, source, &options.compose)?
            {
                let child_source = ComposeSource::File(child_path.clone());
                let child_id = source_to_id(&child_source);

                if runtime
                    .transclusion
                    .enter(child_id.to_string(), child_path.clone(), 0)
                    .is_ok()
                {
                    if let Some(child_md) = runtime.load_markdown(&child_path) {
                        dependencies.record(
                            child_source.clone(),
                            ReferenceDocumentIdentity::capture(&child_md),
                        );
                        let (child_node, mut descendants) = build_node(
                            &child_md,
                            &child_source,
                            &accepted_child_options(options, &child_path),
                            runtime,
                            extract_references,
                            dependencies,
                            prepared_headings,
                        )?;

                        let ref_id = make_reference_id(source, 0, idx);
                        child_insertions.push(ReferenceInsertion {
                            child_node_id: child_node.node_id.clone(),
                            directive_line: 0, // prologue goes at the start
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
            let is_literal = is_literal_content(epilogue);

            if extract_references {
                let mut attrs = serde_json::Map::new();
                if is_literal {
                    attrs.insert("fm_literal".to_string(), serde_json::Value::Bool(true));
                    // Extract links from literal content for follow mode
                    let links = super::local::extract_markdown_links(epilogue, source);
                    for mut link in links {
                        link.attributes
                            .insert("toc_synthesized".to_string(), serde_json::Value::Bool(true));
                        local_references.records.push(link);
                    }
                }
                local_references.records.push(ReferenceRecord {
                    id: make_reference_id(source, usize::MAX, idx),
                    kind: ReferenceKind::Transclusion,
                    target: if is_literal {
                        classify_target("")
                    } else {
                        classify_target(epilogue)
                    },
                    origin: ReferenceOrigin {
                        source: source.clone(),
                        line: usize::MAX,
                        span: 0..0,
                        syntax: ReferenceSyntax::FrontmatterEpilogue,
                    },
                    attributes: attrs,
                });
            }

            if !is_literal
                && let Some(child_path) =
                    resolve_local_target(epilogue, source, &options.compose)?
            {
                let child_source = ComposeSource::File(child_path.clone());
                let child_id = source_to_id(&child_source);

                if runtime
                    .transclusion
                    .enter(child_id.to_string(), child_path.clone(), 0)
                    .is_ok()
                {
                    if let Some(child_md) = runtime.load_markdown(&child_path) {
                        dependencies.record(
                            child_source.clone(),
                            ReferenceDocumentIdentity::capture(&child_md),
                        );
                        let (child_node, mut descendants) = build_node(
                            &child_md,
                            &child_source,
                            &accepted_child_options(options, &child_path),
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
    }

    // Re-sort after adding transclusion records
    if extract_references {
        local_references
            .records
            .sort_by_key(|r| (r.origin.line, r.origin.span.start));
    }

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
    records.extend(super::html::extract_html_script_blocks(content, source));
    records.extend(super::html::extract_html_link_tags(content, source));
    records.extend(super::html::extract_html_meta_tags(content, source));

    // Extract style blocks once and reuse for CSS/font extraction (rec #17)
    let style_blocks = super::html::extract_html_style_blocks(content, source);
    for style_record in &style_blocks {
        if let Some(css_content) = style_record
            .attributes
            .get("css_content")
            .and_then(|v| v.as_str())
        {
            records.extend(super::css::extract_css_imports(
                css_content,
                source,
                style_record.origin.line,
            ));
            records.extend(super::css::extract_font_face_sources(
                css_content,
                source,
                style_record.origin.line,
            ));
        }
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
        ComposeSource::File(path) if options.compose.source != *source => {
            accepted_child_options(options, path)
        }
        ComposeSource::Url(url) if options.compose.source != *source => {
            ReferenceGraphOptions::with_compose(
                options.compose.clone().with_source_url(url.clone()),
            )
        }
        _ => options.clone(),
    };
    prepare_content(md, &node_options)
}

/// Prepare content by running only InlinePre operations.
///
/// Starts from the caller's `ComposeOptions` (rec #3) to preserve
/// external state, overrides, cache settings, shell settings, and per-node
/// source provenance, then restricts to InlinePre-only operations.
fn prepare_content(
    md: &Markdown,
    options: &ReferenceGraphOptions,
) -> MarkdownResult<String> {
    let inline_pre_options = options.compose.clone().only(&[
        ComposeOperation::TextReplacement,
        ComposeOperation::PageBlocks,
        ComposeOperation::Interpolation,
        ComposeOperation::ShellExpansion,
    ]);

    let (result, _report) = md.compose_with(inline_pre_options)?;
    Ok(result.content().to_string())
}

/// Detect literal content in frontmatter prologue/epilogue values.
///
/// Returns `true` when the value is inline markdown content rather than
/// a file path reference. Literal content contains newlines or starts
/// with markdown syntax that cannot be a valid file path.
fn is_literal_content(value: &str) -> bool {
    value.contains('\n') || value.starts_with("---")
}

/// Resolve a local path target through the request-scoped detailed resolver.
fn resolve_local_target(
    raw_target: &str,
    source: &ComposeSource,
    options: &ComposeOptions,
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
        ComposeSource::File(p) => NodeId::from(
            p.canonicalize()
                .unwrap_or_else(|_| p.clone())
                .to_string_lossy()
                .to_string(),
        ),
        ComposeSource::Url(u) => NodeId::from(u.to_string()),
    }
}

fn resolve_toc_linking_target(
    directive: &toc_linking::TocLinkingDirective,
    source: &ComposeSource,
    transclusion_options: &crate::markdown::compose::TransclusionOptions,
    ctx: SourceContext,
) -> Option<(String, std::path::PathBuf)> {
    toc_linking::resolve_target_chain(directive, source, transclusion_options, ctx)
        .ok()
        .flatten()
}

fn generate_toc_link_references(
    display_target: &str,
    path: &std::path::Path,
    directive: &toc_linking::TocLinkingDirective,
    source: &ComposeSource,
    runtime: &ReferenceAnalysisRuntime,
) -> Vec<ReferenceRecord> {
    // 35.4: route the TOC-heading read through the run's shared cache instead
    // of a second `Markdown::try_from` on the same file. The target is also
    // loaded (cached) by the follow-mode traversal below, so a single owner
    // serves both graph-discovery and composition reads. A load failure
    // reproduces the previous empty-result behavior.
    let Some(target_md) = runtime.load_markdown(path) else {
        return Vec::new();
    };

    let toc = target_md.toc();
    let headings = toc.all_headings();

    headings
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
                },
                attributes: serde_json::Map::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::{ComposeContext, ComposeOptions};
    use crate::markdown::reference::ReferenceGraphMismatchKind;
    use crate::markdown::reference::validate::ReferenceValidationOptions;

    #[test]
    fn single_document_graph() {
        let md = Markdown::new("# Hello\n\n[link](./file.md)\n\n![img](./photo.png)");
        let options = ReferenceGraphOptions::default();
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
        let options = ReferenceGraphOptions::default();
        let graph = build_reference_graph(&md, &options).unwrap();
        let flat = flatten_graph(&graph);
        assert_eq!(flat.len(), 2);
    }

    #[test]
    fn transclusion_graph_no_references() {
        let md = Markdown::new("# Just text\n\nNo links here.");
        let options = ReferenceGraphOptions::default();
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
        let graph = build_reference_graph(&root_md, &ReferenceGraphOptions::default()).unwrap();
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
             ::file child-3.md when=\"file_exists('!package.flag')\"\n",
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
        let options = ReferenceGraphOptions::with_compose(
            ComposeOptions::new().with_file_resolution_context(snapshot),
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
        let graph = build_reference_graph(&root_md, &ReferenceGraphOptions::default()).unwrap();
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
        let options = ReferenceGraphOptions::default();
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
        let compose = ComposeOptions::new()
            .with_source_file(&root_path)
            .with_file_resolution_context(snapshot);
        let graph_options = ReferenceGraphOptions::with_compose(compose.clone());
        let root_md = Markdown::try_from(root_path.as_path()).unwrap();

        let (composed, _) = root_md.compose_with(compose.clone()).unwrap();
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

        let child_md = Markdown::try_from(child_path.as_path()).unwrap();
        let ordinary_external = compose.with_source_file(&child_path);
        assert!(child_md.transclusions_with_options(&ordinary_external).is_err());
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
        let options = ReferenceGraphOptions::default();
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
        let options = ReferenceGraphOptions::default();
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

    #[test]
    fn cycle_composed_references_are_finite() {
        let dir = tempfile::TempDir::new().unwrap();

        let a_path = dir.path().join("a.md");
        let b_path = dir.path().join("b.md");
        std::fs::write(&a_path, "[a-link](https://a.example.com)\n\n::file b.md").unwrap();
        std::fs::write(&b_path, "[b-link](https://b.example.com)\n\n::file a.md").unwrap();

        let root_md = Markdown::try_from(a_path.as_path()).unwrap();
        let options = ReferenceGraphOptions::default();
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
        let options = ReferenceGraphOptions::default();
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
                },
                ReferenceInsertion {
                    child_node_id: "child_b".into(),
                    directive_line: 0,
                    insertion_order: 1,
                    reference_id: None,
                    context: ReferenceInsertionContext::default(),
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
                    },
                    attributes: serde_json::Map::new(),
                }],
            },
            child_insertions: vec![],
        };

        let graph = ReferenceGraph::from_build(
            &Markdown::new(""),
            &ReferenceGraphOptions::default(),
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
        let options = ReferenceGraphOptions::default();
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
        let options = ReferenceGraphOptions::default();
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
        let graph = build_reference_graph(&root_md, &ReferenceGraphOptions::default()).unwrap();

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
        let graph = build_reference_graph(&root_md, &ReferenceGraphOptions::default()).unwrap();

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
        let graph = build_reference_graph(&md, &ReferenceGraphOptions::default()).unwrap();
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
        let options = ReferenceGraphOptions::default();
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
    /// persistent-cache `context_hash` drops it — this is the case the
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
            ComposeOptions::new_with_context(ComposeContext::fixed_for_testing_with([(
                "timestamp",
                serde_json::json!(stamp),
            )]))
        };
        let compose_a = compose_for("1000");
        let compose_b = compose_for("2000");

        let graph_a = md
            .reference_graph(ReferenceGraphOptions::with_compose(compose_a.clone()))
            .unwrap();
        let graph_b = md
            .reference_graph(ReferenceGraphOptions::with_compose(compose_b.clone()))
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
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
                    compose_a,
                )),
            )
            .expect("same-context reuse must be accepted");
        assert_eq!(report.references_scanned, 1);

        // Cross-context reuse is rejected on the options dimension.
        let err = md
            .validate_references_with_graph(
                &graph_a,
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
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
            ComposeOptions::new_with_context(ComposeContext::fixed_for_testing_with([(
                "memory_used",
                serde_json::json!(memory_used),
            )]))
        };
        let compose_excluded = compose_for(1024);
        let compose_included = compose_for(4096);

        let graph_excluded = md
            .reference_graph(ReferenceGraphOptions::with_compose(
                compose_excluded.clone(),
            ))
            .unwrap();
        let graph_included = md
            .reference_graph(ReferenceGraphOptions::with_compose(
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
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
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
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
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
                ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(
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
        assert_eq!(
            runtime.strong_count(),
            strong_alone + 1,
            "the build options hold the second strong reference"
        );

        let graph = md
            .reference_graph(ReferenceGraphOptions::with_compose(compose))
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
        assert_eq!(
            Arc::strong_count(&preflight),
            strong_alone + 1,
            "the build options hold one additional strong reference"
        );

        let graph = md
            .reference_graph(ReferenceGraphOptions::with_compose(compose))
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
}
