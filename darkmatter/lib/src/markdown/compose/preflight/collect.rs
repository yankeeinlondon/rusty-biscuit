//! Condition-blind shell-command collection across the document graph.
//!
//! Walks frontmatter, body `::shell`/`::shell-block` directives, and the
//! transclusion graph **without** evaluating any `when=`/page-block condition,
//! so the resulting approval set is a superset of anything reachable under any
//! document state. This is the *approval* half of pre-flight; condition-aware
//! *execution* stays in the inline shell-expansion stage.
//!
//! The walk is a single recursive pass per document: each visited file resolves
//! its own inline state (frontmatter interpolation, text replacement, body
//! interpolation) — never page blocks, never transclusion, never shell
//! execution — then contributes its commands and recurses into every
//! referenced child. There is no second discovery compose that runs the full
//! transclusion merge.

use std::collections::HashSet;
use std::ops::Range;
use std::path::PathBuf;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeOperation;
use crate::markdown::compose::ComposeOptions;
use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::context::effective_state as state;
use crate::markdown::compose::frontmatter_interpolation::interpolate_frontmatter_best_effort;
use crate::markdown::compose::frontmatter_shell_expansion::{
    directive_reachable_pipelines, parse_shell_value, scan_frontmatter,
};
use crate::markdown::compose::prepare_frontmatter_for_compose;
use crate::markdown::compose::shell_expansion::parser::parse_directives;
use crate::markdown::compose::expression::ExpressionFinder;
use crate::markdown::compose::shell_expansion::policy::normalize_command;
use crate::markdown::compose::shell_expansion::types::{
    ShellCommandEntry, ShellCommandOrigin, ShellDirective, ShellExpansionError, frontmatter_key_line,
};
use crate::markdown::compose::transclusion;
use crate::markdown::compose::remote_fetch;
use crate::markdown::types::MarkdownResult;

use super::super::block_pairs;
use super::super::parse_utils::strip_blockquote_prefix;
use super::super::shell_blocks::body::split_logical_commands;

/// Renders a single action back to its raw command form for display in
/// discovery output (without any preceding chain operator).
fn render_action(executable: &str, args: &[String]) -> String {
    if args.is_empty() {
        return executable.to_string();
    }
    let mut out = String::from(executable);
    for arg in args {
        out.push(' ');
        if arg.contains(' ') || arg.contains('"') || arg.contains('\'') {
            out.push('"');
            out.push_str(&arg.replace('"', "\\\""));
            out.push('"');
        } else {
            out.push_str(arg);
        }
    }
    out
}

/// Yields one tuple per executable action in a directive: the per-action raw
/// command rendering, the executable, and its args. For non-pipeline
/// directives this yields a single entry.
fn directive_action_iter(directive: &ShellDirective) -> Vec<(String, String, Vec<String>)> {
    if let Some(ref pipeline) = directive.pipeline
        && pipeline.actions.len() > 1
    {
        return pipeline
            .actions
            .iter()
            .map(|a| {
                let exe = a.command.executable.clone();
                let args = a.command.args.clone();
                let raw = render_action(&exe, &args);
                (raw, exe, args)
            })
            .collect();
    }
    vec![(
        directive.raw_command.clone(),
        directive.executable.clone(),
        directive.args.clone(),
    )]
}

/// Walks the full document graph and returns every shell command that *could*
/// run under any document state.
///
/// Collection is **condition-blind**: commands inside `when=`-false page blocks
/// and false-condition transclusions are still collected, so the approval set
/// covers every branch. Interpolation and text replacement resolve normally so
/// commands have their final shape; page blocks, transclusion, and shell
/// execution never run. Entries are deduped by normalized command string.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::compose::ComposeOptions;
/// use darkmatter::markdown::compose::preflight::collect_shell_commands;
///
/// let md: Markdown = "# Test\n::shell echo hello\n".into();
/// let options = ComposeOptions::new();
/// let entries = collect_shell_commands(&md, &options).unwrap();
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries[0].executable, "echo");
/// ```
///
/// ## Errors
///
/// - Propagates interpolation/transclusion-resolution failures.
/// - Propagates `::shell` directive parse errors.
/// - Returns [`ShellExpansionError::DynamicCommandShape`] when a body command
///   embeds a frontmatter value still pending frontmatter-shell expansion.
pub fn collect_shell_commands(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> MarkdownResult<Vec<ShellCommandEntry>> {
    let (entries, _graph) = collect_shell_commands_with_graph(markdown, options)?;
    Ok(entries)
}

/// Like [`collect_shell_commands`], but also returns the hierarchical graph
/// metadata the walk discovered.
///
/// The graph is the reusable artifact the v2 design calls for (tech-design §
/// "Reusing the collection walk"): the resolved child sources, the per-child
/// shell entries, and the nested child tree. The flat `entries` vector is
/// still returned for callers that only need the deduped approval set.
pub fn collect_shell_commands_with_graph(
    markdown: &Markdown,
    options: &ComposeOptions,
) -> MarkdownResult<(Vec<ShellCommandEntry>, super::PreflightGraphNode)> {
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    let mut visited = HashSet::new();
    // Reuse the caller-supplied shared runtime when present so the terminal
    // compose pass that follows fetches each remote URL once (single-flight),
    // not twice. Absent a shared runtime, build a private one.
    let remote_fetch = options.remote_fetch_runtime();
    let graph = collect_recursive(
        markdown,
        options,
        &mut seen,
        &mut entries,
        &mut visited,
        &remote_fetch,
    )?;
    Ok((entries, graph))
}

/// Collects every command from one document and recurses into its referenced
/// children, condition-blind.
///
/// Returns a [`super::PreflightGraphNode`] describing this document's resolved
/// source, its locally-discovered shell entries, and its nested children.
fn collect_recursive(
    markdown: &Markdown,
    options: &ComposeOptions,
    seen: &mut HashSet<String>,
    entries: &mut Vec<ShellCommandEntry>,
    visited: &mut HashSet<String>,
    remote_fetch: &remote_fetch::RemoteFetchRuntime,
) -> MarkdownResult<super::PreflightGraphNode> {
    let source_file = match &options.source {
        ComposeSource::File(p) => p.clone(),
        ComposeSource::Url(u) => PathBuf::from(u.as_str()),
        _ => PathBuf::from("<unknown>"),
    };

    let source_key = match &options.source {
        ComposeSource::File(path) => Some(
            std::fs::canonicalize(path)
                .unwrap_or_else(|_| path.clone())
                .to_string_lossy()
                .to_string(),
        ),
        ComposeSource::Url(url) => Some(url.to_string()),
        ComposeSource::Unknown => None,
    };
    if let Some(key) = source_key
        && !visited.insert(key)
    {
        return Ok(super::PreflightGraphNode::default());
    }

    // Per-document accumulator: shell entries first discovered *here* (so the
    // returned graph node can attribute them to this document). The global
    // `entries` and `seen` are still updated so the flat list stays deduped
    // across the whole graph.
    let mut local_entries: Vec<ShellCommandEntry> = Vec::new();
    let mut edges: Vec<super::PreflightGraphEdge> = Vec::new();

    // ── Frontmatter commands ───────────────────────────────────────
    scan_one_frontmatter(markdown, options, &source_file, seen, entries, &mut local_entries)?;

    // ── Resolve this document's inline state ───────────────────────
    // Interpolation + text replacement only: never page blocks (so
    // `when=`-false regions survive), never transclusion (we walk children
    // ourselves), never shell execution.
    let inline_ops: Vec<_> = [
        ComposeOperation::FrontmatterInterpolation,
        ComposeOperation::TextReplacement,
        ComposeOperation::Interpolation,
    ]
    .into_iter()
    .filter(|op| options.is_enabled(*op))
    .collect();

    // Discovery is a non-terminal pass: it strips FrontmatterShellExpansion to
    // avoid executing commands. Run schema validation so a genuinely-invalid
    // frontmatter value (e.g. an empty required string) fails fast *before* the
    // approval gate or any shell execution — matching the design's
    // "schema validation → approval → shell" ordering. Defer problems on
    // still-literal `$(...)` values: the terminal compose pass expands and
    // re-validates them, so they are not yet final violations here.
    let mut inline_exclude_keys = options.exclude_keys.clone();
    inline_exclude_keys.insert("$schema".to_string());
    let mut inline_options = options
        .clone()
        .only(&inline_ops)
        .with_exclude_keys(inline_exclude_keys);
    inline_options.defer_shell_pending_schema_problems = true;
    let (prepared, _) = markdown.compose_with(inline_options)?;
    let line_offset = prepared.frontmatter_line_count();
    let prepared_ctx = prepared.full_source_context_for_errors();

    // ── Dynamic command shape (chicken-and-egg) ────────────────────
    // A body command whose text embeds a frontmatter value still pending
    // frontmatter-shell expansion cannot be approved condition-blind: its
    // approved shape (with `$(...)`) differs from its executed shape. Reject it
    // here rather than letting it surface as a late `NotPreApproved`.
    let pending = pending_shell_literals(&prepared, &prepared_ctx);
    detect_dynamic_command_shape(prepared.content(), &pending, &prepared_ctx, line_offset)?;

    // ── Body `::shell` directives ──────────────────────────────────
    let directives = parse_directives(prepared.content(), prepared_ctx.clone(), line_offset)?;
    for directive in directives {
        let line = directive.origin.line_number();
        for (raw_action, executable, args) in directive_action_iter(&directive) {
            let normalized = normalize_command(&executable, &args);
            if seen.insert(normalized.clone()) {
                let entry = ShellCommandEntry {
                    raw_command: raw_action,
                    executable,
                    args,
                    normalized,
                    source_file: source_file.clone(),
                    origin: ShellCommandOrigin::Body { line },
                };
                local_entries.push(entry.clone());
                entries.push(entry);
            }
        }
    }

    // ── Body `::shell-block` commands ──────────────────────────────
    // A malformed block (e.g. an unterminated `::block`) is surfaced richly by
    // the terminal compose pass (page-blocks / shell-blocks stage) with a source
    // link and excerpt. Pre-flight discovery must not pre-empt it with a
    // flattened Transform string — and a document that fails to scan has no
    // well-formed shell block to approve anyway. Skip discovery on a scan
    // failure and let the compose pass produce the real, styled error.
    let pairs = block_pairs::scan_block_pairs(prepared.content()).unwrap_or_default();
    for pair in pairs {
        if !matches!(pair.kind, block_pairs::BlockOpenKind::Shell) {
            continue;
        }
        let body_text = &prepared.content()[pair.body_span.clone()];
        let commands = split_logical_commands(body_text, pair.start_line + 1)
            .map_err(|e| crate::markdown::types::MarkdownError::Transform(e.to_string()))?;
        for command in commands {
            for action in &command.pipeline.actions {
                let executable = action.command.executable.clone();
                let args = action.command.args.clone();
                let raw_command = render_action(&executable, &args);
                let normalized = normalize_command(&executable, &args);
                if !seen.insert(normalized.clone()) {
                    continue;
                }
                let entry = ShellCommandEntry {
                    raw_command,
                    executable,
                    args,
                    normalized,
                    source_file: source_file.clone(),
                    origin: ShellCommandOrigin::ShellBlock {
                        start_line: pair.start_line + line_offset,
                        command_line: command.start_line + line_offset,
                    },
                };
                local_entries.push(entry.clone());
                entries.push(entry);
            }
        }
    }

    // ── Recurse into referenced children (condition-blind) ─────────
    let transclusion_opts = options.transclusion_options();

    for directive in transclusion::parse_directives(prepared.content(), prepared_ctx.clone())? {
        // `::file` and `::url` can both reference Markdown children that
        // contain shell directives. `::code` inserts literal code (no shell
        // directives), so it is excluded.
        if !matches!(
            directive.kind,
            transclusion::DirectiveKind::File | transclusion::DirectiveKind::Url
        ) {
            continue;
        }

        // No `when=` evaluation: a false-condition transclusion still
        // contributes its commands to the approval set.
        let resolved = match transclusion::resolve_target(
            directive.kind,
            &directive.raw_target,
            &transclusion_opts,
            &options.source,
            directive.line,
            prepared_ctx.clone(),
        ) {
            Ok(r) => r,
            // Remote transclusion disabled: the child cannot appear in the
            // composed output, so its commands can never execute.
            Err(transclusion::TransclusionError::UrlExecutionDisabled { .. }) => continue,
            Err(e) => return Err(e.into()),
        };

        match resolved {
            transclusion::ResolvedTarget::File { path, .. } => {
                let mut child = Markdown::try_from(path.as_path())?;
                apply_child_overrides(&mut child, &directive.options);
                let child = collect_recursive(
                    &child,
                    &options.clone().with_accepted_source_file(path.clone()),
                    seen,
                    entries,
                    visited,
                    remote_fetch,
                )?;
                edges.push(super::PreflightGraphEdge {
                    directive: directive.clone(),
                    resolved_target: super::PreflightResolvedTarget::File(path),
                    child: std::sync::Arc::new(child),
                });
            }
            transclusion::ResolvedTarget::Url { url, .. } => {
                let Some(body) = fetch_remote_child_body(&url, remote_fetch)? else {
                    continue;
                };
                let mut child = Markdown::from(body);
                apply_child_overrides(&mut child, &directive.options);
                let child = collect_recursive(
                    &child,
                    &options.clone().with_source_url(url.clone()),
                    seen,
                    entries,
                    visited,
                    remote_fetch,
                )?;
                edges.push(super::PreflightGraphEdge {
                    directive: directive.clone(),
                    resolved_target: super::PreflightResolvedTarget::Url(url),
                    child: std::sync::Arc::new(child),
                });
            }
        }
    }

    let refs =
        transclusion::parse_frontmatter_refs(prepared.frontmatter().as_map(), prepared_ctx.clone())?;
    // Refcount bumps, not subtree deep-clones (Finding 16): each entry is the
    // same `Arc` already held by the corresponding edge.
    let mut children: Vec<std::sync::Arc<super::PreflightGraphNode>> =
        edges.iter().map(|e| std::sync::Arc::clone(&e.child)).collect();
    for reference in refs.prologue.iter().chain(refs.epilogue.iter()) {
        let file_ref = match transclusion::classify_frontmatter_reference(reference) {
            transclusion::FrontmatterReference::Inline => continue,
            transclusion::FrontmatterReference::Parsed(file_ref) => file_ref,
            transclusion::FrontmatterReference::ParseError(error) => {
                return Err(transclusion::TransclusionError::from(error).into());
            }
        };

        let resolved = match transclusion::resolve_parsed_target(
            transclusion::DirectiveKind::File,
            &file_ref,
            &transclusion_opts,
            &options.source,
            0,
            prepared_ctx.clone(),
        ) {
            Ok(r) => r,
            Err(transclusion::TransclusionError::UrlExecutionDisabled { .. }) => continue,
            Err(e) => return Err(e.into()),
        };

        let child = match resolved {
            transclusion::ResolvedTarget::File { path, .. } => {
                let child = Markdown::try_from(path.as_path())?;
                collect_recursive(
                    &child,
                    &options.clone().with_accepted_source_file(path),
                    seen,
                    entries,
                    visited,
                    remote_fetch,
                )?
            }
            transclusion::ResolvedTarget::Url { url, .. } => {
                let Some(body) = fetch_remote_child_body(&url, remote_fetch)? else {
                    continue;
                };
                let child = Markdown::from(body);
                collect_recursive(
                    &child,
                    &options.clone().with_source_url(url),
                    seen,
                    entries,
                    visited,
                    remote_fetch,
                )?
            }
        };
        // Frontmatter prologue/epilogue references have no parsed
        // directive; they appear in the `children` view (so callers that
        // want a flat list of referenced documents still get them) but are
        // NOT added to `edges` — the transclusion engine treats them as
        // section slots rather than replace-target directives, and
        // resolving them a second time is cheap.
        children.push(std::sync::Arc::new(child));
    }

    let source = match &options.source {
        ComposeSource::File(p) => Some(p.clone()),
        ComposeSource::Url(u) => Some(PathBuf::from(u.to_string())),
        ComposeSource::Unknown => None,
    };

    Ok(super::PreflightGraphNode {
        source,
        entries: local_entries,
        edges,
        children,
    })
}

/// Applies `set_object`/`set_properties` overrides from a transclusion
/// directive's options to a child document's frontmatter.
fn apply_child_overrides(child: &mut Markdown, opts: &transclusion::BlockOptions) {
    if opts.set_object.is_some() || !opts.set_properties.is_empty() {
        let base_indexmap = std::mem::take(child.frontmatter_mut().as_map_mut());
        let base_map: serde_json::Map<String, serde_json::Value> =
            base_indexmap.into_iter().collect();
        let overlaid =
            state::apply_set_overrides(&base_map, opts.set_object.as_ref(), &opts.set_properties);
        *child.frontmatter_mut().as_map_mut() = overlaid.into_iter().collect();
    }
}

/// Fetches a remote Markdown child for pre-flight collection.
///
/// Registers the URL on demand (the eager pre-scan only covers the root) and
/// blocks until the fetch completes. Returns `None` when the URL was not
/// registered or produced no body.
fn fetch_remote_child_body(
    url: &url::Url,
    remote_fetch: &remote_fetch::RemoteFetchRuntime,
) -> MarkdownResult<Option<String>> {
    remote_fetch.register_nested(url.clone());
    match remote_fetch.get_content(url) {
        Ok(body) => Ok(body),
        Err(e) => Err(crate::markdown::types::MarkdownError::Transform(format!(
            "pre-flight: failed to fetch remote transclusion '{}': {e}",
            url
        ))),
    }
}

/// Collects the frontmatter `$(...)` commands for a single document.
fn scan_one_frontmatter(
    markdown: &Markdown,
    options: &ComposeOptions,
    source_file: &std::path::Path,
    seen: &mut HashSet<String>,
    entries: &mut Vec<ShellCommandEntry>,
    local_entries: &mut Vec<ShellCommandEntry>,
) -> MarkdownResult<()> {
    let mut fm_clone = markdown.clone();
    let pre_interpolation_snapshot = prepare_frontmatter_for_compose(&mut fm_clone, options, true);
    let mut preflight_exclude_keys = options.exclude_keys.clone();
    preflight_exclude_keys.insert("$schema".to_string());
    if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
        // Defer templated keys that reference a shell-pending (`$(...)`) value.
        // Without this, a key like `review: "{{ dir + '/x' }}"` resolves against
        // `dir`'s still-literal `$(...)` text and becomes `$(...)/x`, which then
        // trips `scan_frontmatter`'s "trailing content" guard. Deferral keeps it
        // as template text so only the real `$(...)` directive (`dir`) is scanned.
        //
        // Preflight only: shell-command discovery enumerates the reachable
        // pipelines for the approval workflow. It must use the same local
        // read-side resolution context as the real frontmatter pass so
        // request-scoped inputs supplied via set overrides (for example
        // `spec=reviews/x/spec.md`) resolve during derived-key interpolation.
        //
        // Best-effort: a key that still cannot resolve is left for the real
        // compose pass. This keeps command discovery resilient while avoiding a
        // false schema failure when a required property is derived through
        // `file_exists()` or `frontmatter()`.
        let _ = interpolate_frontmatter_best_effort(
            fm_clone.frontmatter_mut(),
            options.context(),
            false,
            true,
            Some(options.frontmatter_resolution_context()),
            &preflight_exclude_keys,
            &options.name_coercion_keys,
        );
    }

    let scan_ctx = fm_clone.full_source_context_for_errors();

    // ── Dynamic frontmatter command shape ──────────────────────────
    // A top-level `$(...)` value that still carries `{{ … }}` after best-effort
    // interpolation depends on a key that could not be resolved context-free —
    // either a context-requiring function (`file_exists`, `frontmatter`, …) or
    // another shell-pending `$(...)` sibling. Its executed shape (e.g.
    // `echo true`) differs from any statically-collectable form, and collecting
    // it verbatim would leak template syntax into the approval set. Reject it
    // here with a clear dynamic-shape error rather than letting it surface later
    // as the misleading "command not pre-approved … bug in the pre-flight
    // scanner". This mirrors `detect_dynamic_command_shape` for body directives.
    detect_dynamic_frontmatter_command_shape(fm_clone.frontmatter(), &scan_ctx)?;

    let candidates = scan_frontmatter(
        fm_clone.frontmatter(),
        pre_interpolation_snapshot.as_ref(),
        &scan_ctx,
        &preflight_exclude_keys,
    )?;

    for candidate in candidates {
        // Walk every pipeline this directive could run — for a plain pipeline
        // there is exactly one; for a ternary, every non-empty branch
        // contributes a pipeline so both reachable command sets surface. The
        // legacy `executable`/`args` fields on the candidate are placeholders
        // for the ternary case and cannot be used directly.
        let pipelines = directive_reachable_pipelines(
            &candidate,
            fm_clone.frontmatter(),
            options,
            &scan_ctx,
        )?;

        for pipeline in pipelines {
            let executable = pipeline
                .actions
                .first()
                .map(|a| a.command.executable.clone())
                .unwrap_or_default();
            let args = pipeline
                .actions
                .first()
                .map(|a| a.command.args.clone())
                .unwrap_or_default();
            let raw_command = pipeline.display_string();

            let directive = ShellDirective {
                raw_command,
                executable,
                args,
                span: 0..0,
                indent: String::new(),
                origin: ShellCommandOrigin::Frontmatter {
                    key: candidate.key.clone(),
                    line: candidate.line,
                },
                error_handling: Default::default(),
                timeout_override: candidate.timeout_override,
                // Collection only enumerates commands for approval; it never
                // executes, so the cache flag is irrelevant here.
                no_cache: false,
                pipeline: Some(pipeline),
                ctx: scan_ctx.clone(),
            };

            for (raw_action, executable, args) in directive_action_iter(&directive) {
                let normalized = normalize_command(&executable, &args);
                if seen.insert(normalized.clone()) {
                    let entry = ShellCommandEntry {
                        raw_command: raw_action,
                        executable,
                        args,
                        normalized,
                        source_file: source_file.to_path_buf(),
                        origin: ShellCommandOrigin::Frontmatter {
                            key: candidate.key.clone(),
                            line: candidate.line,
                        },
                    };
                    local_entries.push(entry.clone());
                    entries.push(entry);
                }
            }
        }
    }

    Ok(())
}

/// Fails with [`ShellExpansionError::DynamicCommandShape`] when a top-level
/// `$(...)` frontmatter value still carries a `{{ … }}` expression after
/// best-effort interpolation.
///
/// Such a value depends on a key the context-free pre-flight pass could not
/// resolve (a context-requiring function, or a shell-pending sibling), so its
/// executed shape is not yet known. The error names the unresolved key the
/// command depends on so the failure is actionable instead of surfacing later
/// as a `NotPreApproved` "bug in the pre-flight scanner".
fn detect_dynamic_frontmatter_command_shape(
    frontmatter: &crate::markdown::frontmatter::Frontmatter,
    ctx: &biscuit_terminal::errors::SourceContext,
) -> MarkdownResult<()> {
    for (key, value) in frontmatter.as_map().iter() {
        let serde_json::Value::String(s) = value else {
            continue;
        };
        if !s.trim_start().starts_with("$(") {
            continue;
        }
        let locations = ExpressionFinder::find_all_plain(s);
        let Some(first) = locations.first() else {
            continue;
        };
        // Name the dependency root the surviving template references; fall back
        // to the raw expression text when it is not a simple variable.
        let dependency = first
            .expression
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .find(|t| !t.is_empty())
            .unwrap_or(first.expression.as_str())
            .to_string();
        return Err(ShellExpansionError::DynamicCommandShape {
            ctx: Box::new(ctx.clone()),
            command: s.clone(),
            key: dependency,
            origin: ShellCommandOrigin::Frontmatter {
                key: key.clone(),
                line: frontmatter_key_line(ctx, key),
            },
        }
        .into());
    }
    Ok(())
}

/// Returns the `(key, "$(...)")` frontmatter values still pending
/// frontmatter-shell expansion after pass-1 interpolation.
///
/// These are the only values whose literal `$(...)` text can leak into a body
/// command via `{{ doc.key }}` interpolation, which is the dynamic-shape case.
fn pending_shell_literals(
    prepared: &Markdown,
    ctx: &biscuit_terminal::errors::SourceContext,
) -> Vec<(String, String)> {
    let mut pending = Vec::new();
    for (key, value) in prepared.frontmatter().as_map().iter() {
        if let serde_json::Value::String(s) = value
            && s.starts_with("$(")
            && matches!(parse_shell_value(s, key, None, ctx), Ok(Some(_)))
        {
            pending.push((key.clone(), s.clone()));
        }
    }
    pending
}

/// Fails with [`ShellExpansionError::DynamicCommandShape`] when a pending
/// frontmatter shell value has been interpolated into a body `::shell` directive
/// or `::shell-block` command.
fn detect_dynamic_command_shape(
    content: &str,
    pending: &[(String, String)],
    ctx: &biscuit_terminal::errors::SourceContext,
    line_offset: usize,
) -> MarkdownResult<()> {
    if pending.is_empty() {
        return Ok(());
    }

    let shell_block_spans: Vec<Range<usize>> = block_pairs::scan_block_pairs(content)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| matches!(p.kind, block_pairs::BlockOpenKind::Shell))
        .map(|p| p.body_span)
        .collect();

    for (key, literal) in pending {
        let mut from = 0usize;
        while let Some(rel) = content[from..].find(literal.as_str()) {
            let at = from + rel;
            from = at + literal.len();

            let line_start = content[..at].rfind('\n').map(|p| p + 1).unwrap_or(0);
            let line_end = content[at..]
                .find('\n')
                .map(|p| at + p)
                .unwrap_or(content.len());
            let line = strip_blockquote_prefix(content[line_start..line_end].trim());
            let in_shell_directive =
                line == "::shell" || line.starts_with("::shell ") || line.starts_with("::shell\t");
            let in_shell_block = shell_block_spans
                .iter()
                .any(|span| at >= span.start && at < span.end);

            if in_shell_directive || in_shell_block {
                let line_number = content[..at].matches('\n').count() + 1 + line_offset;
                return Err(ShellExpansionError::DynamicCommandShape {
                    ctx: Box::new(ctx.clone()),
                    command: line.to_string(),
                    key: key.clone(),
                    origin: ShellCommandOrigin::Body { line: line_number },
                }
                .into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;
    use crate::markdown::compose::ComposeOptions;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn discovers_shell_directives_in_simple_document() {
        let content = "# Test\n::shell echo hello\n::shell ls -la\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].executable, "echo");
        assert_eq!(entries[0].args, vec!["hello"]);
        assert_eq!(entries[1].executable, "ls");
        assert_eq!(entries[1].args, vec!["-la"]);
    }

    #[test]
    fn discovers_directives_in_transcluded_files() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "::shell echo child").unwrap();

        let root_path = temp_dir.path().join("root.md");
        let mut root_file = std::fs::File::create(&root_path).unwrap();
        writeln!(root_file, "::shell echo root").unwrap();
        writeln!(root_file, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        let raw_commands: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw_commands.contains(&"echo root"));
        assert!(raw_commands.contains(&"echo child"));
    }

    #[test]
    fn deduplicates_by_normalized_form() {
        let content = "::shell echo hello\n::shell echo hello\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].normalized, "echo hello");
    }

    #[test]
    fn defers_keys_referencing_shell_pending_values() {
        let content = "---\ndir: \"$(dirname '{{spec}}')\"\nreview: \"{{ dir + '/review-' + iteration + '.md' }}\"\n---\nbody\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new().with_set_overrides(serde_json::json!({
            "spec": "features/rough-edges/spec.md",
            "iteration": "1",
        }));

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].executable, "dirname");
        assert_eq!(entries[0].args, vec!["features/rough-edges/spec.md"]);
    }

    /// Regression for the `review-feature.md` "command not pre-approved" bug.
    ///
    /// `dir` is a shell-pending directive whose template (`{{ spec || design }}`)
    /// is transitively blocked because `design` references `dir`, so it resolves
    /// only in the post-main fallback pass. A sibling `iteration` key calls
    /// `file_exists()` — a filesystem function that errors in the context-free
    /// pre-flight pass. Before the fix, that error aborted the interpolation
    /// before the fallback ran, so the command was collected with its raw
    /// `{{ … }}` template; execution then ran the resolved form, yielding a
    /// spurious "command not pre-approved". The collected argument must be the
    /// resolved path.
    #[test]
    fn collects_resolved_command_despite_context_requiring_sibling_key() {
        let content = "---\n\
dir: \"$(dirname '{{ spec || design }}')\"\n\
design: \"{{ file_exists(dir + '/design.md') ? dir + '/design.md' : null }}\"\n\
iteration: \"{{ file_exists('design.md') ? 2 : 1 }}\"\n\
---\nbody\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new().with_set_overrides(serde_json::json!({
            "spec": "fixes/x/spec.md",
        }));

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1, "exactly one shell command (dir)");
        assert_eq!(entries[0].executable, "dirname");
        assert_eq!(
            entries[0].args,
            vec!["fixes/x/spec.md"],
            "must collect the resolved argument, not the raw {{{{ }}}} template"
        );
        assert!(
            !entries[0].normalized.contains("{{"),
            "no template syntax should survive into the approval set: {:?}",
            entries[0].normalized
        );
    }

    #[test]
    fn preflight_resolves_launch_relative_derived_required_file() {
        let launch_dir = TempDir::new().unwrap();
        std::fs::write(launch_dir.path().join(".git"), "gitdir: /tmp/fake\n").unwrap();
        let prompt_dir = launch_dir.path().join("prompts");
        std::fs::create_dir_all(&prompt_dir).unwrap();
        let review_dir = launch_dir.path().join("reviews/2026-06-30-replace-expression");
        std::fs::create_dir_all(&review_dir).unwrap();
        std::fs::write(review_dir.join("spec.md"), "# Spec\n").unwrap();
        std::fs::write(
            review_dir.join("plan.md"),
            "---\nstart_phase: 2\ntotal_phases: 3\n---\n# Plan\n",
        )
        .unwrap();

        let prompt_path = prompt_dir.join("implement-plan.md");
        let yaml = "\
$schema:
  phase: 'number(required)'
  total_phases: 'number(required)'
  plan: 'file(required)'
  spec: file
plan: \"{{ file_exists(spec) ? dirname(spec) + '/plan.md' : null }}\"
phase: \"{{ file_exists(plan) ? frontmatter(plan, 'start_phase') || 1 : null }}\"
total_phases: \"{{ file_exists(plan) ? frontmatter(plan, 'total_phases') : 0 }}\"
spec: \"{{ file_exists(plan) ? dirname(plan) + '/spec.md' : null }}\"
";
        let content = format!("---\n{yaml}---\nbody\n");
        std::fs::write(&prompt_path, &content).unwrap();
        let md = Markdown::try_from_content(content.to_string()).unwrap();
        assert!(
            md.frontmatter().as_map().get("$schema").is_some_and(|v| v.is_object()),
            "fixture must parse $schema as an object, got {:?}",
            md.frontmatter().as_map().get("$schema")
        );
        let options = ComposeOptions::new()
            .with_source_file(&prompt_path)
            .with_file_ref_fallback_dir(launch_dir.path())
            .with_set_overrides(serde_json::json!({
                "spec": "reviews/2026-06-30-replace-expression/spec.md",
            }));

        md.compose_preflight(&options)
            .expect("preflight should resolve the derived plan through the request context");
    }

    /// Regression for the review-4 "approves the WRONG command" bug.
    ///
    /// `cmd` is a `$(...)` directive whose argument interpolates `exists`, a
    /// sibling that calls `frontmatter()` on a file that does not exist — a
    /// context-requiring filesystem function that errors in the pre-flight
    /// resolution pass. Before the fix, the best-effort interpolation
    /// substituted the errored key as empty, so the collector approved `echo`
    /// (empty arg) while execution ran the resolved form, yielding the
    /// misleading "command not pre-approved … bug in the pre-flight scanner".
    /// The collector must instead reject the command as a dynamic shape and
    /// never put the wrong (or template-bearing) command in the approval set.
    ///
    /// `file_exists()` no longer drives this path: since pre-flight gained a
    /// resolution context (launch-relative derived-file support), a missing
    /// local path resolves to `false` — the same value execution produces — so
    /// such a command is correctly approved (see
    /// `collects_resolved_command_despite_context_requiring_sibling_key`).
    /// `frontmatter()` on an absent file still errors, keeping the
    /// errored-dependency deferral live.
    #[test]
    fn rejects_command_depending_on_context_requiring_sibling_key() {
        let content = "---\n\
exists: \"{{ frontmatter('missing-sibling.md', 'x') }}\"\n\
cmd: \"$(echo '{{ exists }}')\"\n\
---\nbody\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let err = collect_shell_commands(&md, &options).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("frontmatter.cmd") && msg.contains("'exists'"),
            "error must name the dynamic command and its dependency: {msg}"
        );
        assert!(
            !msg.contains("not pre-approved"),
            "must not surface as the late NotPreApproved scanner-bug error: {msg}"
        );
        assert!(
            !msg.contains("bug in the pre-flight scanner"),
            "must be a clear dynamic-shape error, not the misleading scanner-bug message: {msg}"
        );
    }

    /// The dynamic-shape rejection must be scoped to keys that genuinely cannot
    /// resolve context-free. A `$(...)` command whose argument interpolates a
    /// sibling that IS context-free-resolvable (here a plain templated key that
    /// folds to a literal) must still collect its fully-resolved command — the
    /// fix must not over-reject.
    #[test]
    fn still_collects_command_depending_on_context_free_sibling_key() {
        let content = "---\n\
name: \"{{ 'wor' + 'ld' }}\"\n\
cmd: \"$(echo '{{ name }}')\"\n\
---\nbody\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();
        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].executable, "echo");
        assert_eq!(entries[0].args, vec!["world"]);
        assert!(
            !entries[0].normalized.contains("{{"),
            "no template syntax should survive: {:?}",
            entries[0].normalized
        );
    }

    #[test]
    fn resolves_interpolated_variables_before_scanning() {
        let content = "---\ncmd_arg: world\n---\n::shell echo {{cmd_arg}}\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo world");
    }

    #[test]
    fn ignores_directives_in_code_blocks() {
        let content = "::shell echo outside\n\n```bash\n::shell echo inside\n```\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo outside");
    }

    #[test]
    fn empty_document_returns_empty_vec() {
        let md: Markdown = "# Just a heading\n\nSome text.\n".into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert!(entries.is_empty());
    }

    /// T1: approval is condition-blind — a command in a `when=`-false page block
    /// is still collected.
    #[test]
    fn includes_directives_inside_false_page_blocks() {
        let content = "\
---
include_shell: false
---
::shell echo always
::block when=\"include_shell\"
::shell echo conditional
::end-block
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo always"), "raw: {raw:?}");
        assert!(raw.contains(&"echo conditional"), "raw: {raw:?}");
    }

    /// T1: a command inside a `when=`-false page block nested in a shell block
    /// is still collected.
    #[test]
    fn includes_shell_block_commands_inside_false_page_blocks() {
        let content = "\
---
include_shell: false
---
::shell echo always
::block when=\"include_shell\"
::shell-block
echo conditional
::end-block
::end-block
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo always"), "raw: {raw:?}");
        assert!(raw.contains(&"echo conditional"), "raw: {raw:?}");
    }

    /// T1: a command in a false-condition transclusion is still collected.
    #[test]
    fn includes_frontmatter_shell_commands_in_false_transclusions() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        std::fs::write(&child_path, "---\ncmd: \"$(echo hidden)\"\n---\n# Child\n").unwrap();

        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\ninclude_child: false\n---\n::file ./child.md when=\"include_child\"\n",
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].raw_command, "echo hidden");
    }

    /// T1: a body command in a false-condition transclusion is still collected.
    #[test]
    fn includes_body_shell_commands_in_false_transclusions() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        std::fs::write(&child_path, "# Child\n::shell echo hidden-body\n").unwrap();

        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\ninclude_child: false\n---\n::file ./child.md when=\"include_child\"\n",
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].raw_command, "echo hidden-body");
    }

    #[test]
    fn discovers_directives_introduced_by_text_replacement() {
        let content = "\
---
replace:
  PLACEHOLDER: \"echo replaced\"
---
::shell PLACEHOLDER
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo replaced");
    }

    #[test]
    fn transclusion_provenance_attributes_to_child_file() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "# Child").unwrap();
        writeln!(child_file, "::shell echo from-child").unwrap();

        let root_path = temp_dir.path().join("root.md");
        let mut root_file = std::fs::File::create(&root_path).unwrap();
        writeln!(root_file, "::shell echo from-root").unwrap();
        writeln!(root_file, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);

        let root_entry = entries
            .iter()
            .find(|e| e.raw_command == "echo from-root")
            .unwrap();
        assert_eq!(
            root_entry.source_file.canonicalize().unwrap(),
            root_path.canonicalize().unwrap()
        );

        let child_entry = entries
            .iter()
            .find(|e| e.raw_command == "echo from-child")
            .unwrap();
        assert_eq!(
            child_entry.source_file.canonicalize().unwrap(),
            child_path.canonicalize().unwrap()
        );
        assert_eq!(
            child_entry.origin,
            ShellCommandOrigin::Body { line: 2 },
            "line should be 2 in the child file"
        );
    }

    #[test]
    fn discovers_frontmatter_shell_commands() {
        let content = "---\nfiles: \"$(sniff repo dirty-files)\"\n---\n# Doc\n::shell echo body\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2, "entries: {entries:?}");
        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"sniff"), "Missing sniff: {executables:?}");
        assert!(executables.contains(&"echo"), "Missing echo: {executables:?}");
    }

    #[test]
    fn discovers_frontmatter_command_when_schema_constrains_the_value() {
        let content = "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\ntier: $(echo small)\n---\n# Doc\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1, "entries: {entries:?}");
        assert_eq!(entries[0].executable, "echo");
        assert_eq!(entries[0].raw_command, "echo small");
    }

    #[test]
    fn discovers_frontmatter_shell_commands_in_transcluded_files() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        std::fs::write(
            &child_path,
            "---\nchild_cmd: \"$(echo child-frontmatter)\"\n---\n# Child\n",
        )
        .unwrap();

        let root_path = temp_dir.path().join("root.md");
        std::fs::write(&root_path, "::file ./child.md\n").unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo child-frontmatter");
        assert_eq!(
            entries[0].source_file.canonicalize().unwrap(),
            child_path.canonicalize().unwrap()
        );
        assert_eq!(
            entries[0].origin,
            ShellCommandOrigin::Frontmatter {
                key: "child_cmd".to_string(),
                line: Some(2),
            }
        );
    }

    #[test]
    fn frontmatter_shell_commands_use_frontmatter_origin() {
        let content = "---\nfiles: \"$(echo fm-cmd)\"\n---\n# Doc\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        match &entries[0].origin {
            ShellCommandOrigin::Frontmatter { key, .. } => assert_eq!(key, "files"),
            other => panic!("Expected Frontmatter origin, got: {other:?}"),
        }
    }

    #[test]
    fn frontmatter_and_body_commands_deduplicate() {
        let content = "---\nval: \"$(echo hello)\"\n---\n::shell echo hello\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn frontmatter_interpolation_resolves_before_discovery() {
        let content = "---\nname: world\ncmd: \"$(echo {{name}})\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo world");
    }

    #[test]
    fn discovery_uses_external_state_before_frontmatter_scan() {
        let content = "---\ncmd: \"{{tool}}\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new()
            .with_external_state(serde_json::json!({"tool": "$(echo external)"}));

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo external");
    }

    #[test]
    fn discovery_uses_set_overrides_before_frontmatter_scan() {
        let content = "---\ncmd: \"$(echo before)\"\n---\n";
        let md: Markdown = content.into();
        let options =
            ComposeOptions::new().with_set_overrides(serde_json::json!({"cmd": "$(echo after)"}));

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo after");
    }

    #[test]
    fn discovery_rejects_interpolated_frontmatter_executable() {
        let content = "---\ncmd_name: echo\ncmd: \"$({{cmd_name}} hello)\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let err = collect_shell_commands(&md, &options).unwrap_err();
        assert!(
            err.to_string()
                .contains("Frontmatter shell executable may not come from interpolation")
        );
    }

    /// T10: a body command embedding a shell-pending frontmatter value is a hard
    /// pre-flight error, not a late `NotPreApproved`.
    #[test]
    fn dynamic_command_shape_in_body_directive_is_rejected() {
        let content = "---\nbranch: $(git branch --show-current)\n---\n::shell git log {{ doc.branch }} -1\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let err = collect_shell_commands(&md, &options).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("depends on frontmatter key 'branch'"),
            "unexpected error: {msg}"
        );
        assert!(
            !msg.contains("not pre-approved"),
            "must not surface as NotPreApproved: {msg}"
        );
    }

    /// A pending frontmatter value referenced only as prose content is fine;
    /// dynamic-shape detection is scoped to shell directives.
    #[test]
    fn dynamic_value_in_prose_is_not_rejected() {
        let content = "---\nbranch: $(git branch --show-current)\n---\nThe branch is {{ doc.branch }}.\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();
        // Only the frontmatter command itself is collected.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].executable, "git");
    }

    #[test]
    fn body_chain_emits_one_entry_per_action() {
        let content = "::shell echo ok && pwd || ls\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo ok"), "missing echo ok: {raw:?}");
        assert!(raw.contains(&"pwd"), "missing pwd: {raw:?}");
        assert!(raw.contains(&"ls"), "missing ls: {raw:?}");
    }

    #[test]
    fn frontmatter_chain_emits_one_entry_per_action() {
        let content = "---\nfiles: \"$(echo first || pwd)\"\n---\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"echo"), "missing echo: {executables:?}");
        assert!(executables.contains(&"pwd"), "missing pwd: {executables:?}");
    }

    #[test]
    fn body_chain_with_redirection_emits_per_action_entries() {
        let content = "::shell echo silent > /dev/null && pwd\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"echo"));
        assert!(executables.contains(&"pwd"));
    }

    #[test]
    fn discovers_shell_block_commands() {
        let content = "::shell-block\necho hello\necho world\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].raw_command, "echo hello");
        assert_eq!(entries[1].raw_command, "echo world");
        assert_eq!(
            entries[0].origin,
            ShellCommandOrigin::ShellBlock {
                start_line: 1,
                command_line: 2,
            }
        );
    }

    #[test]
    fn shell_block_chain_emits_one_entry_per_action() {
        let content = "::shell-block\necho ok && pwd || ls\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo ok"), "missing echo ok: {raw:?}");
        assert!(raw.contains(&"pwd"), "missing pwd: {raw:?}");
        assert!(raw.contains(&"ls"), "missing ls: {raw:?}");

        let blocks: Vec<_> = entries
            .iter()
            .filter(|e| matches!(e.origin, ShellCommandOrigin::ShellBlock { .. }))
            .collect();
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn discovers_mixed_shell_and_shell_block_commands() {
        let content = "::shell echo standalone\n::shell-block\necho block\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);
        let standalones: Vec<_> = entries
            .iter()
            .filter(|e| matches!(e.origin, ShellCommandOrigin::Body { .. }))
            .collect();
        let blocks: Vec<_> = entries
            .iter()
            .filter(|e| matches!(e.origin, ShellCommandOrigin::ShellBlock { .. }))
            .collect();
        assert_eq!(standalones.len(), 1);
        assert_eq!(blocks.len(), 1);
        assert_eq!(standalones[0].raw_command, "echo standalone");
        assert_eq!(blocks[0].raw_command, "echo block");
    }

    #[test]
    fn shell_block_commands_deduplicate_with_shell_directives() {
        let content = "::shell echo hello\n::shell-block\necho hello\n::end-block\n";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
    }

    /// T1: both branches of a frontmatter ternary surface, condition-blind.
    #[test]
    fn frontmatter_ternary_emits_both_branch_commands() {
        let content = "\
---
flag: true
out: \"$(flag ? echo yes : basename README.md)\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo yes"), "missing echo yes: {raw:?}");
        assert!(
            raw.contains(&"basename README.md"),
            "missing basename README.md: {raw:?}"
        );
    }

    #[test]
    fn frontmatter_ternary_empty_branch_emits_nothing() {
        let content = "\
---
flag: true
out: \"$(flag ? echo only : '')\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].raw_command, "echo only");
    }

    #[test]
    fn frontmatter_ternary_branch_chain_emits_one_entry_per_action() {
        let content = "\
---
flag: true
out: \"$(flag ? echo a && pwd : ls)\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        let executables: Vec<&str> = entries.iter().map(|e| e.executable.as_str()).collect();
        assert!(executables.contains(&"echo"), "missing echo: {executables:?}");
        assert!(executables.contains(&"pwd"), "missing pwd: {executables:?}");
        assert!(executables.contains(&"ls"), "missing ls: {executables:?}");
    }

    #[test]
    fn frontmatter_ternary_branch_arguments_are_interpolated() {
        let content = "\
---
flag: true
name: README.md
out: \"$(flag ? basename {{name}} : '')\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].executable, "basename");
        assert_eq!(entries[0].args, vec!["README.md"]);
    }

    #[test]
    fn frontmatter_ternary_branch_rejects_interpolated_executable() {
        let content = "\
---
flag: true
cmd_name: echo
out: \"$(flag ? {{cmd_name}} hi : '')\"
---
";
        let md: Markdown = content.into();
        let options = ComposeOptions::new();

        let err = collect_shell_commands(&md, &options).unwrap_err();
        assert!(
            err.to_string().contains("may not come from interpolation"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn discovers_shell_block_commands_in_transcluded_files() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        let mut child_file = std::fs::File::create(&child_path).unwrap();
        writeln!(child_file, "# Child").unwrap();
        writeln!(child_file, "::shell-block").unwrap();
        writeln!(child_file, "echo from-child").unwrap();
        writeln!(child_file, "::end-block").unwrap();

        let root_path = temp_dir.path().join("root.md");
        let mut root_file = std::fs::File::create(&root_path).unwrap();
        writeln!(root_file, "::shell echo from-root").unwrap();
        writeln!(root_file, "::file ./child.md").unwrap();

        let root_content = std::fs::read_to_string(&root_path).unwrap();
        let md: Markdown = root_content.into();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let entries = collect_shell_commands(&md, &options).unwrap();

        assert_eq!(entries.len(), 2);

        let child_entry = entries
            .iter()
            .find(|e| e.raw_command == "echo from-child")
            .unwrap();
        assert_eq!(
            child_entry.source_file.canonicalize().unwrap(),
            child_path.canonicalize().unwrap()
        );
        match &child_entry.origin {
            ShellCommandOrigin::ShellBlock {
                start_line,
                command_line,
            } => {
                assert_eq!(*start_line, 2);
                assert_eq!(*command_line, 3);
            }
            other => panic!("Expected ShellBlock origin, got: {other:?}"),
        }
    }

    /// T1 (remote): a `::url` remote transclusion child contributes its commands
    /// to the approval set. The directive is `::url`-prefixed, not `::file`, so
    /// the collector must traverse both shapes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_url_transclusion_child_participates_in_preflight() {
        use crate::markdown::compose::remote::RemoteReadConfig;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/child.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "# Remote child\n::shell echo remote-body\n::shell echo remote-body-2\n",
            ))
            .mount(&server)
            .await;

        let remote_url = format!("{}/child.md", server.uri());

        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            format!("# Root\n::url {remote_url}\n"),
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let config = RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        };
        let options = ComposeOptions::new()
            .with_source_file(&root_path)
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(config);

        let entries = collect_shell_commands(&md, &options).unwrap();
        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(
            raw.contains(&"echo remote-body"),
            "remote child body command missing from approval set: {raw:?}"
        );
        assert!(
            raw.contains(&"echo remote-body-2"),
            "remote child body command 2 missing from approval set: {raw:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mixed_case_http_file_target_participates_in_preflight() {
        use crate::markdown::compose::remote::RemoteReadConfig;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/mixed-child.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "# Remote child\n::shell echo mixed-case-remote\n",
            ))
            .mount(&server)
            .await;

        let remote_url = server.uri().replacen("http://", "HtTp://", 1);
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            format!("# Root\n::file {remote_url}/mixed-child.md\n"),
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let config = RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        };
        let options = ComposeOptions::new()
            .with_source_file(&root_path)
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(config);

        let entries = collect_shell_commands(&md, &options).unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.raw_command == "echo mixed-case-remote"),
            "mixed-case HTTP ::file child must participate in preflight: {entries:?}"
        );
    }

    /// T1 (remote, false-`when`): the collector must ignore `when=` even on a
    /// `::url` directive so a dead-branch remote child still appears in the
    /// approval set. Without this, the v2 condition-blind invariant is broken
    /// for remote transclusion.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_url_transclusion_false_when_still_collected() {
        use crate::markdown::compose::remote::RemoteReadConfig;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/hidden.md"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "# Remote child\n::shell echo hidden-remote\n",
            ))
            .mount(&server)
            .await;

        let remote_url = format!("{}/hidden.md", server.uri());

        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            format!("---\ninclude_remote: false\n---\n# Root\n::url {remote_url} when=\"include_remote\"\n"),
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let config = RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        };
        let options = ComposeOptions::new()
            .with_source_file(&root_path)
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(config);

        let entries = collect_shell_commands(&md, &options).unwrap();
        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(
            raw.contains(&"echo hidden-remote"),
            "false-when remote child command must still be in the approval set: {raw:?}"
        );
    }

    /// T1 (remote, frontmatter `prologue`): a `prologue:` URL reference is
    /// resolved by the collector the same way a body `::url` directive is —
    /// the child document's commands are part of the approval set.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_url_frontmatter_prologue_participates_in_preflight() {
        use crate::markdown::compose::remote::RemoteReadConfig;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/prologue-child.md"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string("# Prologue child\n::shell echo prologue-cmd\n"))
            .mount(&server)
            .await;

        let remote_url = format!("{}/prologue-child.md", server.uri());

        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            format!("---\nprologue: \"{remote_url}\"\n---\n# Root\n"),
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let config = RemoteReadConfig {
            allowed_hosts: vec!["127.0.0.1".into()],
            ..Default::default()
        };
        let options = ComposeOptions::new()
            .with_source_file(&root_path)
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(config);

        let entries = collect_shell_commands(&md, &options).unwrap();
        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(
            raw.contains(&"echo prologue-cmd"),
            "remote prologue child command missing from approval set: {raw:?}"
        );
    }

    /// The graph metadata captures the resolved child sources, per-child shell
    /// entries, and the nested child tree. The flat `entries` list is
    /// unchanged — this is additive metadata for downstream reuse.
    #[test]
    fn preflight_graph_attributes_entries_to_their_source_document() {
        let temp_dir = TempDir::new().unwrap();

        let child_path = temp_dir.path().join("child.md");
        std::fs::write(
            &child_path,
            "---\nchild_cmd: \"$(echo child-frontmatter)\"\n---\n# Child\n::shell echo child-body\n",
        )
        .unwrap();

        let root_path = temp_dir.path().join("root.md");
        std::fs::write(
            &root_path,
            "---\nroot_cmd: \"$(echo root-frontmatter)\"\n---\n::shell echo root-body\n::file ./child.md\n",
        )
        .unwrap();

        let md = Markdown::try_from(root_path.as_path()).unwrap();
        let options = ComposeOptions::new().with_source_file(&root_path);

        let (entries, graph) = collect_shell_commands_with_graph(&md, &options).unwrap();

        // The flat approval set is the union, deduped.
        let raw: Vec<&str> = entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(raw.contains(&"echo child-frontmatter"));
        assert!(raw.contains(&"echo child-body"));
        assert!(raw.contains(&"echo root-frontmatter"));
        assert!(raw.contains(&"echo root-body"));

        // The graph node for the root documents its resolved source.
        assert_eq!(
            graph
                .source
                .as_ref()
                .and_then(|p| std::fs::canonicalize(p).ok()),
            std::fs::canonicalize(&root_path).ok(),
            "root graph node must carry the root's resolved source"
        );

        // The root's own entries include only what the root document contains.
        let root_raw: Vec<&str> = graph.entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(root_raw.contains(&"echo root-frontmatter"));
        assert!(root_raw.contains(&"echo root-body"));
        assert!(
            !root_raw.contains(&"echo child-body"),
            "child entries must not be attributed to the root"
        );

        // Exactly one nested child, and its entries are attributed to itself.
        assert_eq!(graph.children.len(), 1, "children: {:?}", graph.children);
        let child = &graph.children[0];
        assert_eq!(
            child
                .source
                .as_ref()
                .and_then(|p| std::fs::canonicalize(p).ok()),
            std::fs::canonicalize(&child_path).ok(),
            "child graph node must carry the child's resolved source"
        );
        let child_raw: Vec<&str> = child.entries.iter().map(|e| e.raw_command.as_str()).collect();
        assert!(child_raw.contains(&"echo child-frontmatter"));
        assert!(child_raw.contains(&"echo child-body"));
        assert!(
            child.children.is_empty(),
            "leaf transclusion should have no nested children"
        );

        // `flattened_entries` gives the union through the graph — the caller
        // gets the same deduped set the flat `entries` carries, just from the
        // hierarchical view.
        let flattened_raw: Vec<&str> = graph
            .flattened_entries()
            .iter()
            .map(|e| e.raw_command.as_str())
            .collect();
        for cmd in ["echo child-frontmatter", "echo child-body", "echo root-frontmatter", "echo root-body"] {
            assert!(flattened_raw.contains(&cmd), "missing {cmd} in flattened: {flattened_raw:?}");
        }
    }
}
