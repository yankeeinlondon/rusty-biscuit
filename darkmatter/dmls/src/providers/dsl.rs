//! Layer-3 Darkmatter DSL provider: directives, transclusion, interpolation,
//! shell awareness, and fenced-language diagnostics.
//!
//! Registered last in the chain, so its contributions merge on top of the
//! substrate, wiki, and frontmatter providers (which never match a directive or
//! interpolation offset). Every capability is **passive**: the overlay scanners
//! ([`crate::overlay::directives`] / [`expressions`](crate::overlay::expressions)
//! / [`shell`](crate::overlay::shell)) parse but never execute, the graph answers
//! cross-document transclusion queries, and the only filesystem touch is an
//! existence `stat` for broken-path diagnostics — no process is ever spawned and
//! no socket is ever opened (spec acceptance criterion 7).

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::directives_api::{DirectiveKind, scan_shell_block_commands};
use darkmatter::markdown::compose::{
    FrontmatterShellValue, parse_frontmatter_shell_value_spanned,
};
use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::language_grammar::LanguageGrammar;
use darkmatter::markdown::span::SourceSpan;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Diagnostic, DiagnosticRelatedInformation,
    DiagnosticSeverity, DocumentLink, Documentation, FoldingRange, Hover, HoverContents, Location,
    MarkupContent, MarkupKind, NumberOrString, Range, TextEdit,
};

use super::DocumentContext;
use super::location::line_range;
use crate::diagnostics::codes::{code, source};
use crate::graph::normalize_join;
use crate::overlay::shell::PolicyVerdict;
use crate::overlay::{directives, expressions, shell};
use crate::source_map::SourceMap;
use crate::workspace::file_path_to_uri;

// ── Completion ──────────────────────────────────────────────────────────────

/// Directive-name, directive-option, and interpolation completions.
pub fn completion(ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
    let line_start = ctx.text[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let prefix = &ctx.text[line_start..offset];

    if let Some(partial) = directives::name_partial(prefix) {
        return directive_name_completions(ctx, offset, partial);
    }
    if let Some((token_start, partial)) = expressions::completion_partial(ctx.text, offset) {
        return interpolation_completions(ctx, token_start, offset, partial);
    }
    if let Some(items) = directive_option_completions(ctx, offset, prefix) {
        return items;
    }
    Vec::new()
}

/// `::keyword` completions filtered by the partial typed after `::`.
fn directive_name_completions(ctx: &DocumentContext, offset: usize, partial: &str) -> Vec<CompletionItem> {
    let start = offset - partial.len() - 2; // include the leading `::`
    directives::DIRECTIVE_CATALOG
        .iter()
        .filter(|info| info.keyword[2..].starts_with(partial))
        .filter_map(|info| {
            text_edit_item(
                ctx.source_map,
                start,
                offset,
                CompletionCandidate {
                    label: info.keyword.to_string(),
                    new_text: info.keyword.to_string(),
                    kind: CompletionItemKind::KEYWORD,
                    detail: Some(info.summary.to_string()),
                    documentation: None,
                },
            )
        })
        .collect()
}

/// Option-key and option-value completions on a directive line.
fn directive_option_completions(
    ctx: &DocumentContext,
    offset: usize,
    prefix: &str,
) -> Option<Vec<CompletionItem>> {
    let directive = directives::directive_at(ctx.text, offset)?;
    // Only offer options once the cursor is past the keyword (and its target).
    if offset <= directive.keyword_span.end {
        return None;
    }
    let (token_start, token) = word_under_cursor(prefix, offset);
    match token.split_once('=') {
        Some((key, value_partial)) => {
            let values = directives::option_values(key)?;
            let value_start = offset - value_partial.len();
            Some(
                values
                    .iter()
                    .filter(|value| value.starts_with(value_partial))
                    .filter_map(|value| {
                        text_edit_item(
                            ctx.source_map,
                            value_start,
                            offset,
                            CompletionCandidate {
                                label: value.to_string(),
                                new_text: value.to_string(),
                                kind: CompletionItemKind::VALUE,
                                detail: None,
                                documentation: None,
                            },
                        )
                    })
                    .collect(),
            )
        }
        None => {
            let keys = directives::option_keys(directive.kind);
            if keys.is_empty() {
                return None;
            }
            Some(
                keys.iter()
                    .filter(|key| key.starts_with(token))
                    .filter_map(|key| {
                        text_edit_item(
                            ctx.source_map,
                            token_start,
                            offset,
                            CompletionCandidate {
                                label: key.to_string(),
                                new_text: format!("{key}="),
                                kind: CompletionItemKind::PROPERTY,
                                detail: None,
                                documentation: None,
                            },
                        )
                    })
                    .collect(),
            )
        }
    }
}

/// Frontmatter-key, `ctx.*`, and expression-function completions inside `{{ }}`.
fn interpolation_completions(
    ctx: &DocumentContext,
    token_start: usize,
    offset: usize,
    partial: &str,
) -> Vec<CompletionItem> {
    let frontmatter_keys: Vec<String> = ctx
        .overlay
        .and_then(|overlay| overlay.ast.as_ref())
        .map(|ast| ast.top_level().map(|entry| entry.key.clone()).collect())
        .unwrap_or_default();
    interpolation_candidates(partial, &frontmatter_keys)
        .into_iter()
        .filter_map(|candidate| text_edit_item(ctx.source_map, token_start, offset, candidate))
        .collect()
}

/// One completion candidate before LSP lowering. Pure data so the D3/D4 field
/// mapping is unit-testable without a [`DocumentContext`].
struct CompletionCandidate {
    /// The completion label — the untyped `signature` for functions.
    label: String,
    /// The eagerly inserted text — the bare name for functions (no snippet,
    /// no synthesized parentheses), the fully qualified `ctx.<name>` for
    /// context variables.
    new_text: String,
    kind: CompletionItemKind,
    /// D3/D4: the rendered `display_type` (`ctx.*`) or `typed_signature()`
    /// (functions).
    detail: Option<String>,
    /// D3/D4: the catalog description, lowered to eager Markdown.
    documentation: Option<String>,
}

/// Candidates matching `partial` (prefix-based, case-sensitive): top-level
/// frontmatter keys, fully qualified `ctx.*` variables, and expression
/// functions — every field derived from the one catalog descriptor.
fn interpolation_candidates(
    partial: &str,
    frontmatter_keys: &[String],
) -> Vec<CompletionCandidate> {
    let mut items = Vec::new();

    for key in frontmatter_keys {
        if key.starts_with(partial) {
            items.push(CompletionCandidate {
                label: key.clone(),
                new_text: key.clone(),
                kind: CompletionItemKind::FIELD,
                detail: Some("frontmatter key".to_string()),
                documentation: None,
            });
        }
    }

    for descriptor in expressions::context_descriptors() {
        let label = format!("ctx.{}", descriptor.name);
        if label.starts_with(partial) {
            items.push(CompletionCandidate {
                new_text: label.clone(),
                label,
                kind: CompletionItemKind::VARIABLE,
                detail: Some(descriptor.display_type.to_string()),
                documentation: Some(descriptor.description.to_string()),
            });
        }
    }

    for descriptor in expressions::function_descriptors() {
        let signature = descriptor.signature;
        let name = signature.split('(').next().unwrap_or(signature);
        if name.starts_with(partial) {
            items.push(CompletionCandidate {
                label: signature.to_string(),
                new_text: name.to_string(),
                kind: CompletionItemKind::FUNCTION,
                detail: Some(descriptor.typed_signature()),
                documentation: Some(descriptor.description.to_string()),
            });
        }
    }

    items
}

// ── Hover ─────────────────────────────────────────────────────────────────

/// Hover for a directive, interpolation, frontmatter `$()` value, or
/// interpolation literal.
pub fn hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    if let Some(hover) = directive_hover(ctx, offset) {
        return Some(hover);
    }
    if let Some(hover) = shell_block_hover(ctx, offset) {
        return Some(hover);
    }
    if let Some(hover) = literal_hover(ctx, offset) {
        return Some(hover);
    }
    if let Some(hover) = interpolation_hover(ctx, offset) {
        return Some(hover);
    }
    frontmatter_shell_hover(ctx, offset)
}

/// Hover describing a directive's semantics and (for transclusion) its resolved
/// target; for `::shell`, the parsed command and policy verdict.
fn directive_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let directive = directives::directive_at(ctx.text, offset)?;
    let info = directives::info_for(directive.kind)?;
    let mut lines = vec![format!("**`{}`** — {}", info.keyword, info.summary)];

    if directives::is_transclusion(directive.kind)
        && let Some(target) = &directive.target
    {
        let path = target.value.split('#').next().unwrap_or(&target.value);
        match resolve_local_path(ctx, path) {
            Some(resolved) if resolved.exists() => {
                lines.push(format!("\n→ `{}`", resolved.display()));
            }
            _ => lines.push(format!("\n⚠️ target `{path}` was not found")),
        }
    }

    if directive.kind == DirectiveKind::Shell
        && let Some(command) = &directive.target
    {
        lines.push(shell_verdict_markdown(&command.value, ctx));
    }

    Some(markup_hover(ctx, directive.span.clone(), lines.join("\n")))
}

/// Hover on a `{{ … }}` expression: its parsed form plus, in precedence order,
/// the catalog block for the function call under the cursor (D5), the shared
/// `ctx.*` catalog block with the compose-time note (D2), a frontmatter static
/// value, or a bare function-name description.
fn interpolation_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let body_base = body_base(ctx.text);
    let interpolation = expressions::interpolation_at(ctx.text, body_base, offset)?;
    let expr_offset = offset.saturating_sub(interpolation.inner.start);
    let value = interpolation_hover_markdown(&interpolation.text, expr_offset, |name| {
        frontmatter_scalar(ctx, name)
    });
    Some(markup_hover(ctx, interpolation.outer.clone(), value))
}

/// Hover on a `{{{ … }}}` interpolation literal: identifies the span and shows
/// the composed output (`{{ content }}`) with a note that the content is inert.
fn literal_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let body_base = body_base(ctx.text);
    let literal = expressions::literal_at(ctx.text, body_base, offset)?;
    Some(markup_hover(
        ctx,
        literal.outer.clone(),
        literal_hover_markdown(&literal.content),
    ))
}

/// Pure Markdown body for a literal hover.
fn literal_hover_markdown(content: &str) -> String {
    let mut composed = String::new();
    composed.push_str("{{");
    composed.push_str(content);
    composed.push_str("}}");
    let longest_backtick_run = composed
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest_backtick_run.saturating_add(1).max(1));
    let rendered = if composed.contains('\n') || composed.contains('\r') {
        let fence = if fence.len() < 3 {
            "```".to_string()
        } else {
            fence
        };
        format!("{fence}\n{composed}\n{fence}")
    } else {
        format!("{fence}{composed}{fence}")
    };
    format!(
        "**Interpolation literal**\n\n{rendered}\n\nThe content is rendered as literal `{{{{ … }}}}` text and is not interpolated."
    )
}

/// The Markdown body of an interpolation hover. Pure (no [`DocumentContext`])
/// so the D2/D5 classification is unit-testable; `expr_offset` is the cursor's
/// byte offset into `expression`.
///
/// The D2 classification rule: only an explicitly `ctx.`-qualified root
/// receives context-variable metadata — a bare identifier is a frontmatter
/// variable even when its name matches a known `ctx.*` tail, and an unknown
/// `ctx.<name>` keeps the generic hover without borrowing a similarly named
/// bare key's value. Nothing here evaluates the expression or reads `ctx.*`.
fn interpolation_hover_markdown(
    expression: &str,
    expr_offset: usize,
    frontmatter_scalar: impl Fn(&str) -> Option<String>,
) -> String {
    let parsed = expressions::parse(expression);
    let mut value = match &parsed {
        Ok(expr) => format!("**Expression**\n\n`{}`", expr.erase()),
        Err(_) => format!("**Expression** (unparsed)\n\n`{expression}`"),
    };
    let Ok(expr) = parsed else {
        return value;
    };

    // D5: a cursor on a known function-name identifier wins.
    if let Some(name) = expressions::function_call_at(&expr, expression, expr_offset)
        && let Some(descriptor) = expressions::function_descriptor(name)
    {
        value.push_str(&format!("\n\n{}", expressions::format_function_block(descriptor)));
        return value;
    }

    // Resolve the identifier against the sub-expression under the cursor (e.g.
    // a `ctx.packages` argument inside a call) rather than the top-level
    // expression, so an argument's ctx/frontmatter hover is not shadowed by
    // the enclosing call.
    let Some(sub_expr) = expressions::expression_at(&expr, expr_offset) else {
        return value;
    };
    let Some(name) = expressions::root_identifier(sub_expr) else {
        return value;
    };
    if let Some(tail) = name.strip_prefix("ctx.") {
        if let Some(descriptor) = expressions::ctx_descriptor(tail) {
            value.push_str(&format!(
                "\n\n{}\n\nThe `ctx` variable is evaluated at _compose_ time (rather than now).",
                expressions::format_ctx_hover_block(descriptor)
            ));
        }
    } else if let Some(scalar) = frontmatter_scalar(&name) {
        value.push_str(&format!("\n\nStatic value: `{scalar}` (from frontmatter `{name}`)"));
    } else if let Some(descriptor) = expressions::function_descriptor(&name) {
        value.push_str(&format!("\n\nFunction: {}", descriptor.description));
    }
    value
}

/// Hover on a frontmatter `$(...)` value: the parsed command and policy verdict.
fn frontmatter_shell_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let (value_span, parsed) = frontmatter_shell_at(ctx, offset)?;
    let command = &ctx.text[shift_span(&parsed.inner_span, value_span.start)];
    let mut markdown = format!("**Shell value**\n\n`{}`", command.trim());
    markdown.push_str(&shell_verdict_markdown(command, ctx));
    Some(markup_hover(ctx, value_span, markdown))
}

/// Hover on a `::shell-block` body command line: the parsed command and policy
/// verdict. The opening `::shell-block` line falls outside every body span, so
/// the catalog hover on the keyword still wins there.
fn shell_block_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let (command, span) = shell_block_commands(ctx.text)
        .into_iter()
        .find(|(_, span)| span.start <= offset && offset <= span.end)?;
    let mut markdown = format!("**Shell command**\n\n`{command}`");
    markdown.push_str(&shell_verdict_markdown(&command, ctx));
    Some(markup_hover(ctx, span, markdown))
}

/// A Markdown line describing a command's policy verdict.
fn shell_verdict_markdown(command: &str, ctx: &DocumentContext) -> String {
    let Some(parsed) = shell::parse_command(command) else {
        return String::new();
    };
    let whitelist = shell::load_whitelist(ctx.config);
    match shell::verdict(&parsed, whitelist.as_ref()) {
        PolicyVerdict::Approved => "\n\nPolicy: ✅ approved (whitelisted).".to_string(),
        PolicyVerdict::Denied(reason) => format!("\n\nPolicy: ⛔ denied — {reason}."),
        PolicyVerdict::Unknown => "\n\nPolicy: ❔ unknown (compose would prompt for approval).".to_string(),
    }
}

// ── Definition & document links ─────────────────────────────────────────────

/// Definition: transclusion target → file; bare interpolation variable →
/// frontmatter key.
pub fn definition(ctx: &DocumentContext, offset: usize) -> Vec<Location> {
    if let Some(directive) = directives::directive_at(ctx.text, offset)
        && directives::is_transclusion(directive.kind)
        && let Some(target) = &directive.target
        && target.span.start <= offset
        && offset <= target.span.end
    {
        let path = target.value.split('#').next().unwrap_or(&target.value);
        if let Some(location) = resolve_local_path(ctx, path).and_then(|p| file_location(&p)) {
            return vec![location];
        }
    }

    if let Some(location) = interpolation_definition(ctx, offset) {
        return vec![location];
    }
    Vec::new()
}

/// A bare interpolation variable's frontmatter-key declaration location.
fn interpolation_definition(ctx: &DocumentContext, offset: usize) -> Option<Location> {
    let body_base = body_base(ctx.text);
    let interpolation = expressions::interpolation_at(ctx.text, body_base, offset)?;
    let expr = expressions::parse(&interpolation.text).ok()?;
    let name = expressions::root_identifier(&expr)?;
    let ast = ctx.overlay.and_then(|overlay| overlay.ast.as_ref())?;
    let entry = ast.entry_by_dotted(&name)?;
    let range = ctx.source_map.byte_range_to_lsp(entry.key_span.clone())?;
    Some(Location::new(ctx.uri.clone(), range))
}

/// Document links for `::file`/`::code` targets and `prologue`/`epilogue`.
pub fn document_links(ctx: &DocumentContext) -> Vec<DocumentLink> {
    let mut links = Vec::new();
    for directive in directives::directives(ctx.text) {
        if !directives::is_transclusion(directive.kind) {
            continue;
        }
        let Some(target) = directive.target else {
            continue;
        };
        let path = target.value.split('#').next().unwrap_or(&target.value);
        if let Some(link) = transclusion_link(ctx, target.span.clone(), path) {
            links.push(link);
        }
    }
    for span in frontmatter_prologue_epilogue(ctx) {
        let path = ctx.text[span.clone()].trim().to_string();
        if let Some(link) = transclusion_link(ctx, span, &path) {
            links.push(link);
        }
    }
    links
}

/// A document link over `span` targeting the resolved local path, if it exists.
fn transclusion_link(ctx: &DocumentContext, span: SourceSpan, path: &str) -> Option<DocumentLink> {
    let range = ctx.source_map.byte_range_to_lsp(span)?;
    let resolved = resolve_local_path(ctx, path)?;
    if !resolved.exists() {
        return None;
    }
    let target = url::Url::from_file_path(&resolved).ok()?;
    Some(DocumentLink {
        range,
        target: target.as_str().parse().ok(),
        tooltip: None,
        data: None,
    })
}

// ── References ──────────────────────────────────────────────────────────────

/// References on a transclusion target: every document that transcludes the same
/// file ("who transcludes this file").
pub fn references(ctx: &DocumentContext, offset: usize, _include_declaration: bool) -> Vec<Location> {
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    // Find the transclusion node under the cursor and its resolved target.
    let target_doc = ctx.graph.transclusions(doc_id).find_map(|(node_id, node)| {
        node.span.contains(&offset).then(|| {
            ctx.graph
                .outgoing(node_id, crate::graph::EdgeKind::Transcludes)
                .find_map(|edge| edge.target.node())
                .and_then(|target| ctx.graph.node(target))
                .map(|node| node.document)
        })
    });
    let Some(Some(target_doc)) = target_doc else {
        return Vec::new();
    };
    ctx.graph
        .transcluded_by(target_doc)
        .into_iter()
        .filter_map(|node_id| super::location::node_location(ctx.graph, node_id))
        .collect()
}

// ── Folding ─────────────────────────────────────────────────────────────────

/// Folds for `::block` / `::shell-block` regions and `::disclosure` blocks.
pub fn folding_ranges(ctx: &DocumentContext) -> Vec<FoldingRange> {
    if !ctx.profile.supports_folding {
        return Vec::new();
    }
    let mut folds = Vec::new();
    if let Ok(blocks) = directives::blocks(ctx.text) {
        for block in blocks {
            push_line_fold(ctx, &mut folds, block.start_line, block.end_line);
        }
    }
    if let Ok(disclosures) = directives::disclosures(ctx.text) {
        for disclosure in disclosures {
            push_span_fold(ctx, &mut folds, disclosure.block_span);
        }
    }
    folds
}

// ── Diagnostics ─────────────────────────────────────────────────────────────

/// All Layer-3 DSL diagnostics for the document.
pub fn diagnostics(ctx: &DocumentContext) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    unknown_directive_diagnostics(ctx, &mut out);
    malformed_option_diagnostics(ctx, &mut out);
    block_pairing_diagnostics(ctx, &mut out);
    disclosure_diagnostics(ctx, &mut out);
    transclusion_diagnostics(ctx, &mut out);
    expression_diagnostics(ctx, &mut out);
    shell_diagnostics(ctx, &mut out);
    fence_language_diagnostics(ctx, &mut out);
    out
}

/// `::word` lines whose keyword names no directive.
fn unknown_directive_diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    let mut in_fence: Option<char> = None;
    let mut offset = 0;
    for line in ctx.text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim_start();
        if let Some(marker) = in_fence {
            if is_fence(trimmed, marker) {
                in_fence = None;
            }
        } else if let Some(marker) = fence_marker(trimmed) {
            in_fence = Some(marker);
        } else if directives::looks_like_directive(trimmed) {
            let keyword = directives::keyword_token(trimmed);
            if !directives::is_known_keyword(keyword) {
                let indent = content.len() - trimmed.len();
                let start = offset + indent;
                let span = start..start + keyword.len();
                if let Some(range) = ctx.source_map.byte_range_to_lsp(span) {
                    out.push(diagnostic(
                        range,
                        DiagnosticSeverity::WARNING,
                        code::DIRECTIVE_UNKNOWN,
                        source::COMPOSE,
                        format!("unknown directive `{keyword}`"),
                    ));
                }
            }
        }
        offset += line.len();
    }
}

/// Option keys a directive family does not recognize.
fn malformed_option_diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    for directive in directives::directives(ctx.text) {
        for option in &directive.options {
            if directives::option_recognized(directive.kind, &option.key.value) {
                continue;
            }
            if let Some(range) = ctx.source_map.byte_range_to_lsp(option.key.span.clone()) {
                out.push(diagnostic(
                    range,
                    DiagnosticSeverity::WARNING,
                    code::DIRECTIVE_MALFORMED_OPTION,
                    source::COMPOSE,
                    format!(
                        "`{}` is not a recognized option for `{}`",
                        option.key.value,
                        directives::info_for(directive.kind).map_or("", |info| info.keyword)
                    ),
                ));
            }
        }
    }
}

/// Unbalanced `::block` / `::shell-block` pairs.
fn block_pairing_diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    use darkmatter::markdown::compose::directives_api::BlockScanError;
    let Err(error) = directives::blocks(ctx.text) else {
        return;
    };
    let (line, message, code_value) = match error {
        BlockScanError::UnmatchedEnd { line } => {
            (line, "`::end-block` has no matching opener".to_string(), code::DIRECTIVE_UNMATCHED_END)
        }
        BlockScanError::UnterminatedBlock { line, .. } => (
            line,
            "block opened here is never closed with `::end-block`".to_string(),
            code::DIRECTIVE_UNCLOSED_BLOCK,
        ),
        BlockScanError::TrailingContent { line, content } => (
            line,
            format!("unexpected content after `::end-block`: `{content}`"),
            code::DIRECTIVE_UNMATCHED_END,
        ),
    };
    out.push(diagnostic(
        line_range(line),
        DiagnosticSeverity::ERROR,
        code_value,
        source::COMPOSE,
        message,
    ));
}

/// Structurally malformed `::disclosure` triples.
fn disclosure_diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    use darkmatter::markdown::MarkdownError;
    if let Err(MarkdownError::MalformedDisclosure { reason, range }) = directives::disclosures(ctx.text)
        && let Some(lsp_range) = ctx.source_map.byte_range_to_lsp(range)
    {
        out.push(diagnostic(
            lsp_range,
            DiagnosticSeverity::ERROR,
            code::DIRECTIVE_MALFORMED_DISCLOSURE,
            source::COMPOSE,
            format!("malformed disclosure: {reason}"),
        ));
    }
}

/// Broken transclusion paths and transclusion cycles.
fn transclusion_diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    for directive in directives::directives(ctx.text) {
        if !directives::is_transclusion(directive.kind) {
            continue;
        }
        let Some(target) = directive.target else {
            continue;
        };
        let path = target.value.split('#').next().unwrap_or(&target.value);
        if is_remote(path) {
            continue;
        }
        let resolved = resolve_local_path(ctx, path);
        let broken = resolved.as_ref().is_none_or(|resolved| !resolved.exists());
        if broken
            && let Some(range) = ctx.source_map.byte_range_to_lsp(target.span.clone())
        {
            out.push(diagnostic(
                range,
                DiagnosticSeverity::WARNING,
                code::TRANSCLUSION_BROKEN_PATH,
                source::COMPOSE,
                format!("broken transclusion: no file matches `{path}`"),
            ));
        }
    }

    // Cycle detection over the workspace `transcludes` graph.
    if let Some(doc_id) = ctx.doc_id
        && let Some(cycle) = ctx.graph.transclusion_cycle(doc_id)
    {
        let related = cycle
            .iter()
            .filter_map(|&doc| ctx.graph.document(doc))
            .filter_map(|record| file_path_to_uri(&record.path))
            .map(|uri| DiagnosticRelatedInformation {
                location: Location::new(uri, line_range(1)),
                message: "on the transclusion cycle".to_string(),
            })
            .collect();
        let range = ctx
            .graph
            .transclusions(doc_id)
            .next()
            .and_then(|(_, node)| ctx.source_map.byte_range_to_lsp(node.span.clone()))
            .unwrap_or_else(|| line_range(1));
        out.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(code::TRANSCLUSION_CYCLE.to_string())),
            source: Some(source::COMPOSE.to_string()),
            message: "transclusion cycle detected".to_string(),
            related_information: Some(related),
            ..Default::default()
        });
    }
}

/// Malformed body interpolations and unknown bare identifiers.
fn expression_diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    let body_base = body_base(ctx.text);
    for interpolation in expressions::interpolations(ctx.text, body_base) {
        match expressions::parse(&interpolation.text) {
            Err(error) => {
                // The parser's `position` is a byte offset into the expression
                // text (Phase 8); range it inside the interpolation.
                let at = (interpolation.inner.start + error.position).min(interpolation.inner.end);
                let span = at..interpolation.inner.end.max(at);
                if let Some(range) = ctx.source_map.byte_range_to_lsp(span) {
                    out.push(diagnostic(
                        range,
                        DiagnosticSeverity::WARNING,
                        code::EXPRESSION_MALFORMED,
                        source::COMPOSE,
                        format!("malformed expression: {}", error.message),
                    ));
                }
            }
            Ok(expr) => {
                if let Some(name) = expressions::root_identifier(&expr)
                    && is_unknown_identifier(ctx, &name)
                    && let Some(range) = ctx.source_map.byte_range_to_lsp(interpolation.inner.clone())
                {
                    out.push(diagnostic(
                        range,
                        DiagnosticSeverity::INFORMATION,
                        code::EXPRESSION_UNKNOWN_IDENTIFIER,
                        source::COMPOSE,
                        format!("`{name}` matches no frontmatter key, `ctx.*`, `env.*`, or function"),
                    ));
                }
            }
        }
    }
}

/// Shell commands the built-in policy blacklist disallows (`::shell`,
/// `::shell-block` body lines, and frontmatter `$()`).
fn shell_diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    let whitelist = shell::load_whitelist(ctx.config);
    for directive in directives::directives(ctx.text) {
        if directive.kind != DirectiveKind::Shell {
            continue;
        }
        let Some(command) = directive.target else {
            continue;
        };
        if let Some(parsed) = shell::parse_command(&command.value)
            && let PolicyVerdict::Denied(reason) = shell::verdict(&parsed, whitelist.as_ref())
            && let Some(range) = ctx.source_map.byte_range_to_lsp(command.span.clone())
        {
            out.push(diagnostic(
                range,
                DiagnosticSeverity::ERROR,
                code::SECURITY_DISALLOWED_COMMAND,
                source::SECURITY,
                format!("disallowed command: {reason}"),
            ));
        }
    }

    for (value_span, parsed) in frontmatter_shell_values(ctx) {
        let command = &ctx.text[shift_span(&parsed.inner_span, value_span.start)];
        if let Some(command) = shell::parse_command(command)
            && let PolicyVerdict::Denied(reason) = shell::verdict(&command, whitelist.as_ref())
            && let Some(range) = ctx.source_map.byte_range_to_lsp(value_span)
        {
            out.push(diagnostic(
                range,
                DiagnosticSeverity::ERROR,
                code::SECURITY_DISALLOWED_COMMAND,
                source::SECURITY,
                format!("disallowed command: {reason}"),
            ));
        }
    }

    // `::shell-block` bodies are shell scripts; scan each command line like a
    // `::shell` target. Block-pairing errors are surfaced by
    // `block_pairing_diagnostics`, so an unpaired block simply yields no lines.
    for (command, span) in shell_block_commands(ctx.text) {
        if let Some(parsed) = shell::parse_command(&command)
            && let PolicyVerdict::Denied(reason) = shell::verdict(&parsed, whitelist.as_ref())
            && let Some(range) = ctx.source_map.byte_range_to_lsp(span)
        {
            out.push(diagnostic(
                range,
                DiagnosticSeverity::ERROR,
                code::SECURITY_DISALLOWED_COMMAND,
                source::SECURITY,
                format!("disallowed command: {reason}"),
            ));
        }
    }
}

/// Fenced-code info strings whose language token no grammar recognizes.
fn fence_language_diagnostics(ctx: &DocumentContext, out: &mut Vec<Diagnostic>) {
    let mut in_fence: Option<char> = None;
    let mut offset = 0;
    for line in ctx.text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim_start();
        if let Some(marker) = in_fence {
            if is_fence(trimmed, marker) {
                in_fence = None;
            }
            offset += line.len();
            continue;
        }
        if let Some(marker) = fence_marker(trimmed) {
            in_fence = Some(marker);
            let info = trimmed.trim_start_matches(marker).trim_start();
            let token = info.split_whitespace().next().unwrap_or("");
            if !token.is_empty() && LanguageGrammar::from_token(token).is_err() {
                let token_offset = offset + (content.len() - info.len());
                let span = token_offset..token_offset + token.len();
                if let Some(range) = ctx.source_map.byte_range_to_lsp(span) {
                    let mut message = format!("unknown fenced-code language `{token}`");
                    if let Some(suggestion) = nearest_language(token) {
                        message.push_str(&format!(" — did you mean `{suggestion}`?"));
                    }
                    out.push(diagnostic(
                        range,
                        DiagnosticSeverity::WARNING,
                        code::FENCE_UNKNOWN_LANGUAGE,
                        source::COMPOSE,
                        message,
                    ));
                }
            }
        }
        offset += line.len();
    }
}

// ── Shared helpers ──────────────────────────────────────────────────────────

/// Common language tokens for the fence nearest-match suggestion.
const COMMON_LANGUAGES: &[&str] = &[
    "rust", "javascript", "typescript", "python", "ruby", "go", "java", "kotlin", "swift", "php",
    "bash", "sh", "shell", "zsh", "html", "css", "scss", "json", "yaml", "toml", "markdown", "sql",
    "c", "cpp", "csharp", "text",
];

/// The closest common language token within edit distance 2, if any.
fn nearest_language(token: &str) -> Option<&'static str> {
    let lower = token.to_ascii_lowercase();
    COMMON_LANGUAGES
        .iter()
        .map(|candidate| (candidate, levenshtein(&lower, candidate)))
        .filter(|(_, distance)| *distance <= 2)
        .min_by_key(|(_, distance)| *distance)
        .map(|(candidate, _)| *candidate)
}

/// Bounded Levenshtein edit distance.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current.push(
                (previous[j + 1] + 1)
                    .min(current[j] + 1)
                    .min(previous[j] + cost),
            );
        }
        previous = current;
    }
    previous[b.len()]
}

/// The document byte offset where the body begins (after any frontmatter block).
pub(crate) fn body_base(text: &str) -> usize {
    match extract_frontmatter_block(text) {
        Ok(Some(extraction)) => extraction.body_span.start,
        _ => 0,
    }
}

/// Lexically resolves a workspace-local `path` against the document's directory.
fn resolve_local_path(ctx: &DocumentContext, path: &str) -> Option<PathBuf> {
    if is_remote(path) {
        return None;
    }
    let base_dir = ctx.path.parent()?;
    Some(normalize_join(base_dir, path))
}

/// A line-based location for a resolved local file.
fn file_location(path: &Path) -> Option<Location> {
    let uri = file_path_to_uri(path)?;
    Some(Location::new(uri, line_range(1)))
}

/// The frontmatter scalar value for a dotted key, from the overlay AST.
fn frontmatter_scalar(ctx: &DocumentContext, dotted: &str) -> Option<String> {
    ctx.overlay
        .and_then(|overlay| overlay.ast.as_ref())
        .and_then(|ast| ast.entry_by_dotted(dotted))
        .and_then(|entry| entry.scalar.clone())
}

/// Whether a bare identifier resolves to nothing DMLS can name.
///
/// Only fires when the document has frontmatter (so the intended variable set is
/// known); on a frontmatter-less document every bare identifier could be a
/// `--set` value, so none is flagged. Namespaced roots (`ctx.*`, `env.*`,
/// `doc.*`) and expression functions are always known.
fn is_unknown_identifier(ctx: &DocumentContext, name: &str) -> bool {
    if matches!(name, "ctx" | "env" | "doc") || name.contains('.') {
        return false;
    }
    if expressions::function_description(name).is_some() {
        return false;
    }
    let Some(ast) = ctx.overlay.and_then(|overlay| overlay.ast.as_ref()) else {
        return false;
    };
    ast.entry_by_dotted(name).is_none()
}

/// The document spans of top-level `prologue`/`epilogue` frontmatter values.
fn frontmatter_prologue_epilogue(ctx: &DocumentContext) -> Vec<SourceSpan> {
    let Some(ast) = ctx.overlay.and_then(|overlay| overlay.ast.as_ref()) else {
        return Vec::new();
    };
    ["prologue", "epilogue"]
        .into_iter()
        .filter_map(|key| ast.entry_by_dotted(key))
        .filter(|entry| entry.scalar.is_some())
        .map(|entry| entry.value_span.clone())
        .collect()
}

/// The frontmatter `$(...)` value whose span contains `offset`.
fn frontmatter_shell_at(
    ctx: &DocumentContext,
    offset: usize,
) -> Option<(SourceSpan, FrontmatterShellValue)> {
    frontmatter_shell_values(ctx)
        .into_iter()
        .find(|(span, _)| span.start <= offset && offset <= span.end)
}

/// Every frontmatter scalar that is a whole-value `$(...)`, with its document
/// value span and parsed (value-relative) form.
fn frontmatter_shell_values(ctx: &DocumentContext) -> Vec<(SourceSpan, FrontmatterShellValue)> {
    let Some(ast) = ctx.overlay.and_then(|overlay| overlay.ast.as_ref()) else {
        return Vec::new();
    };
    ast.entries()
        .iter()
        .filter_map(|entry| {
            let value_span = entry.value_span.clone();
            let raw = ctx.text.get(value_span.clone())?;
            let parsed = parse_frontmatter_shell_value_spanned(raw)?;
            Some((value_span, parsed))
        })
        .collect()
}

/// Every executable command inside a `::shell-block` body, as
/// `(command, document_byte_span)`. Delegates to the library's compose-parity
/// splitter ([`scan_shell_block_commands`]) so hover and the disallowed-command
/// diagnostic classify exactly the commands `md compose` would run — including
/// backslash-folded continuations and block-quote-stripped commands — without
/// executing any. A block-pairing error (reported elsewhere) yields no lines.
fn shell_block_commands(text: &str) -> Vec<(String, SourceSpan)> {
    scan_shell_block_commands(text)
        .into_iter()
        .map(|command| (command.command.value, command.command.span))
        .collect()
}

/// Shifts a value-relative span into document coordinates.
fn shift_span(span: &SourceSpan, base: usize) -> SourceSpan {
    base + span.start..base + span.end
}

/// Whether a target is a remote (`http(s)://`) reference.
fn is_remote(raw: &str) -> bool {
    raw.starts_with("http://") || raw.starts_with("https://")
}

/// The word (and its start offset) under the cursor on a directive line.
fn word_under_cursor(prefix: &str, offset: usize) -> (usize, &str) {
    let token_start = prefix
        .rfind(char::is_whitespace)
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_start = offset - prefix.len();
    (line_start + token_start, &prefix[token_start..])
}

/// A fence's marker character if `trimmed` opens a fenced code block.
fn fence_marker(trimmed: &str) -> Option<char> {
    if trimmed.starts_with("```") {
        Some('`')
    } else if trimmed.starts_with("~~~") {
        Some('~')
    } else {
        None
    }
}

/// Whether `trimmed` closes a fence of `marker`.
fn is_fence(trimmed: &str, marker: char) -> bool {
    let fence = trimmed.trim_end();
    fence.len() >= 3 && fence.chars().all(|c| c == marker)
}

/// Pushes a line-only fold for a 1-indexed `[start_line, end_line]` block.
fn push_line_fold(ctx: &DocumentContext, folds: &mut Vec<FoldingRange>, start_line: usize, end_line: usize) {
    let _ = ctx;
    if end_line > start_line {
        folds.push(FoldingRange {
            start_line: (start_line - 1) as u32,
            start_character: None,
            end_line: (end_line - 1) as u32,
            end_character: None,
            kind: None,
            collapsed_text: None,
        });
    }
}

/// Pushes a line-only fold spanning a byte range's start/end lines.
fn push_span_fold(ctx: &DocumentContext, folds: &mut Vec<FoldingRange>, span: SourceSpan) {
    let Some(start) = ctx.source_map.byte_to_lsp(span.start) else {
        return;
    };
    let end_offset = span.end.saturating_sub(1);
    let Some(end) = ctx.source_map.byte_to_lsp(end_offset) else {
        return;
    };
    if end.line > start.line {
        folds.push(FoldingRange {
            start_line: start.line,
            start_character: None,
            end_line: end.line,
            end_character: None,
            kind: None,
            collapsed_text: None,
        });
    }
}

/// Lowers a [`CompletionCandidate`] to an LSP item with an eager text edit
/// replacing `start..offset` and eager Markdown `documentation` (D3/D4 —
/// nothing is deferred to `completionItem/resolve`, which Zed never uses for
/// `textEdit`).
fn text_edit_item(
    source_map: &SourceMap,
    start: usize,
    offset: usize,
    candidate: CompletionCandidate,
) -> Option<CompletionItem> {
    let range = source_map.byte_range_to_lsp(start..offset)?;
    Some(CompletionItem {
        label: candidate.label,
        kind: Some(candidate.kind),
        detail: candidate.detail,
        documentation: candidate.documentation.map(|value| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            })
        }),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range,
            new_text: candidate.new_text,
        })),
        ..Default::default()
    })
}

/// Builds a Markdown hover over `span`.
fn markup_hover(ctx: &DocumentContext, span: SourceSpan, value: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: ctx.source_map.byte_range_to_lsp(span),
    }
}

/// Builds a diagnostic with a namespaced source and stable code.
fn diagnostic(
    range: Range,
    severity: DiagnosticSeverity,
    code_value: &str,
    source_value: &str,
    message: String,
) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(code_value.to_string())),
        source: Some(source_value.to_string()),
        message,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::Arc;

    use lsp_types::{Position, Uri};

    use crate::capabilities::{ClientProfile, HoverMediaProfile};
    use crate::config::DmlsConfig;
    use crate::graph::WorkspaceGraph;
    use crate::providers::ProviderRegistry;
    use crate::source_map::PositionEncoding;

    #[test]
    fn shell_block_commands_enumerates_body_lines() {
        let text = "# Doc\n\n::shell-block\nrm -rf /tmp/x\necho hi\n::end-block\n";
        let commands = shell_block_commands(text);
        let strings: Vec<&str> = commands.iter().map(|(cmd, _)| cmd.as_str()).collect();
        assert_eq!(strings, vec!["rm -rf /tmp/x", "echo hi"]);
        // Spans point at the trimmed command text in document coordinates.
        for (cmd, span) in &commands {
            assert_eq!(&text[span.clone()], cmd);
        }
    }

    #[test]
    fn shell_block_commands_skips_blank_lines() {
        let text = "::shell-block\n\n  git status\n\n::end-block\n";
        let commands = shell_block_commands(text);
        let strings: Vec<&str> = commands.iter().map(|(cmd, _)| cmd.as_str()).collect();
        assert_eq!(strings, vec!["git status"]);
        // The span excludes the leading indentation.
        assert_eq!(&text[commands[0].1.clone()], "git status");
    }

    #[test]
    fn shell_block_commands_ignores_page_blocks() {
        let text = "::block when=\"true\"\nrm -rf /\n::end-block\n";
        assert!(shell_block_commands(text).is_empty());
    }

    /// A frontmatter lookup that knows no keys.
    fn no_frontmatter(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn ctx_hover_shares_catalog_block_and_appends_compose_note() {
        let descriptor = expressions::ctx_descriptor("packages").unwrap();
        let block = expressions::format_ctx_hover_block(descriptor);
        let markdown = interpolation_hover_markdown("ctx.packages", 0, no_frontmatter);
        // The catalog-backed block is the shared formatter's output verbatim —
        // the same bytes the frontmatter `ctx_hover` renders.
        assert!(markdown.contains(&block));
        // The interpolation-specific passive note follows the block, and the
        // hover carries no captured value — `ctx.*` is never evaluated here.
        assert!(markdown
            .ends_with("The `ctx` variable is evaluated at _compose_ time (rather than now)."));
        assert!(!markdown.contains("Static value"));
    }

    #[test]
    fn bare_identifier_matching_ctx_tail_is_frontmatter_not_context() {
        // `today` is a known `ctx.*` tail, but a bare `{{ today }}` is a
        // frontmatter variable (D2 classification rule).
        let markdown = interpolation_hover_markdown("today", 0, |name| {
            (name == "today").then(|| "2026-07-09".to_string())
        });
        assert!(markdown.contains("Static value: `2026-07-09` (from frontmatter `today`)"));
        assert!(!markdown.contains("Darkmatter-owned"));
        assert!(!markdown.contains("_compose_ time"));
        // Without a matching frontmatter key it still borrows nothing from the
        // ctx catalog.
        let markdown = interpolation_hover_markdown("today", 0, no_frontmatter);
        assert!(!markdown.contains("Darkmatter-owned"));
        assert!(!markdown.contains("_compose_ time"));
    }

    #[test]
    fn unknown_ctx_name_keeps_generic_hover() {
        // An unknown `ctx.<name>` keeps the generic expression hover and does
        // not borrow a similarly named bare frontmatter key's value.
        let markdown = interpolation_hover_markdown("ctx.nope", 0, |name| {
            (name == "nope").then(|| "shadow".to_string())
        });
        assert!(markdown.starts_with("**Expression**"));
        assert!(!markdown.contains("Darkmatter-owned"));
        assert!(!markdown.contains("_compose_ time"));
        assert!(!markdown.contains("Static value"));
    }

    #[test]
    fn function_call_hover_renders_catalog_block() {
        // A formatting function.
        let as_csv = expressions::function_descriptor("as_csv").unwrap();
        let markdown = interpolation_hover_markdown("as_csv(items)", 2, no_frontmatter);
        assert!(markdown.contains(&expressions::format_function_block(as_csv)));
        // A pre-existing function; its fallible typed signature carries `| error`.
        let markdown = interpolation_hover_markdown("length(items)", 3, no_frontmatter);
        let length = expressions::function_descriptor("length").unwrap();
        assert!(markdown.contains(&length.typed_signature()));
        assert!(markdown.contains("| error"));
        // An unknown function keeps the generic parsed-expression hover.
        let markdown = interpolation_hover_markdown("mystery(items)", 3, no_frontmatter);
        assert!(markdown.starts_with("**Expression**"));
        assert!(!markdown.contains("**`mystery"));
    }

    #[test]
    fn function_call_hover_resolves_deepest_call_under_cursor() {
        let source = "as_csv(length(items))";
        let inner = source.find("length").unwrap() + 1;
        let markdown = interpolation_hover_markdown(source, inner, no_frontmatter);
        let length = expressions::function_descriptor("length").unwrap();
        assert!(markdown.contains(&length.typed_signature()));
        // On the outer name the outer call answers.
        let markdown = interpolation_hover_markdown(source, 1, no_frontmatter);
        let as_csv = expressions::function_descriptor("as_csv").unwrap();
        assert!(markdown.contains(&as_csv.typed_signature()));
    }

    #[test]
    fn function_identifier_hover_shows_catalog_block() {
        // Offset 1 is on `s` in the `as_csv` name identifier.
        let as_csv = expressions::function_descriptor("as_csv").unwrap();
        let markdown = interpolation_hover_markdown("as_csv(ctx.packages)", 1, no_frontmatter);
        assert!(markdown.contains(&expressions::format_function_block(as_csv)));
        let packages = expressions::ctx_descriptor("packages").unwrap();
        assert!(!markdown.contains(&expressions::format_ctx_hover_block(packages)));
        assert!(!markdown.contains("_compose_ time"));
    }

    #[test]
    fn ctx_variable_inside_function_call_shows_ctx_hover() {
        // Offset 8 is on `t` in `ctx` — the argument, not the name.
        let packages = expressions::ctx_descriptor("packages").unwrap();
        let markdown = interpolation_hover_markdown("as_csv(ctx.packages)", 8, no_frontmatter);
        assert!(markdown.contains(&expressions::format_ctx_hover_block(packages)));
        assert!(markdown.contains("_compose_ time"));
        let as_csv = expressions::function_descriptor("as_csv").unwrap();
        assert!(
            !markdown.contains(&expressions::format_function_block(as_csv)),
            "no function hover on an argument: {markdown}"
        );
    }

    #[test]
    fn ctx_hover_surfaces_through_index_access() {
        // `ctx.packages[0]` parses as
        // `Index { base: Variable("ctx.packages"), index: NumberLiteral(0) }` —
        // the lexer keeps the dotted `ctx.packages` as one Variable token, so the
        // ctx variable is the root through index access (D2). The catalog block,
        // the rendered type, and the compose-time note must all surface, with no
        // captured frontmatter value.
        let packages = expressions::ctx_descriptor("packages").unwrap();
        let block = expressions::format_ctx_hover_block(packages);

        // Offset 5 lands on `a` inside `packages` — within the ctx-rooted base.
        let markdown = interpolation_hover_markdown("ctx.packages[0]", 5, no_frontmatter);
        assert!(markdown.contains(&block), "catalog block on base: {markdown}");
        assert!(markdown.contains("string[]"), "rendered array type: {markdown}");
        assert!(markdown.contains("_compose_ time"), "compose-time note: {markdown}");
        assert!(!markdown.contains("Static value"), "no captured value: {markdown}");

        // Cursor on the `[` bracket (byte 12) — a structural position of the
        // Index construct rather than the `ctx.packages` text — still resolves to
        // the ctx-rooted expression, confirming the Index-rooted expression is
        // handled end-to-end at the bracket offset.
        let markdown = interpolation_hover_markdown("ctx.packages[0]", 12, no_frontmatter);
        assert!(markdown.contains(&block), "catalog block on `[`: {markdown}");
        assert!(markdown.contains("_compose_ time"), "compose-time note on `[`: {markdown}");
    }

    #[test]
    fn ctx_hover_surfaces_through_member_access_chain() {
        // `ctx.packages[0].first` parses as
        // `MemberAccess { base: Index { base: Variable("ctx.packages"), index: 0 },
        //                  name: "first" }` — the postfix dot only follows `]`, so
        // the chain is rooted at the ctx variable (D2).
        let packages = expressions::ctx_descriptor("packages").unwrap();
        let block = expressions::format_ctx_hover_block(packages);

        // Cursor on the `ctx.packages` base (offset 5 on `a`): the catalog block
        // and compose-time note surface.
        let markdown = interpolation_hover_markdown("ctx.packages[0].first", 5, no_frontmatter);
        assert!(markdown.contains(&block), "catalog block on base: {markdown}");
        assert!(markdown.contains("string[]"), "rendered array type: {markdown}");
        assert!(markdown.contains("_compose_ time"), "compose-time note: {markdown}");

        // Cursor on the `first` member name (offset 17 on `i`): the deepest
        // sub-expression under the cursor is the MemberAccess node, so
        // `root_identifier` must recurse MemberAccess -> Index -> Variable to
        // reach `ctx.packages`. This is the genuine root-recursion proof through
        // both member-access and index nodes.
        let markdown = interpolation_hover_markdown("ctx.packages[0].first", 17, no_frontmatter);
        assert!(markdown.contains(&block), "catalog block on member name: {markdown}");
        assert!(markdown.contains("_compose_ time"), "compose-time note on member name: {markdown}");
    }

    #[test]
    fn frontmatter_variable_inside_function_call_shows_static_value() {
        // Offset 7 is on `i` in `title` — the argument, not the name.
        let markdown = interpolation_hover_markdown("upper(title)", 7, |name| {
            (name == "title").then(|| "Guide".to_string())
        });
        assert!(markdown.contains("Static value: `Guide`"));
        let upper = expressions::function_descriptor("upper").unwrap();
        assert!(
            !markdown.contains(&expressions::format_function_block(upper)),
            "no function hover on an argument: {markdown}"
        );
    }

    #[test]
    fn parentheses_and_commas_show_no_function_hover() {
        let as_csv = expressions::function_descriptor("as_csv").unwrap();
        let block = expressions::format_function_block(as_csv);
        // Offset 6 = `(`, offset 8 = `,`, offset 11 = `)` in `as_csv(a, b)`.
        for (label, offset) in [("(", 6), (",", 8), (")", 11)] {
            let markdown = interpolation_hover_markdown("as_csv(a, b)", offset, no_frontmatter);
            assert!(
                markdown.starts_with("**Expression**"),
                "cursor on {label} keeps the generic hover: {markdown}"
            );
            assert!(
                !markdown.contains(&block),
                "no function docs on {label}: {markdown}"
            );
        }
    }

    #[test]
    fn ctx_completion_derives_detail_and_documentation_from_descriptor() {
        let candidates = interpolation_candidates("ctx.pa", &[]);
        let packages = candidates
            .iter()
            .find(|candidate| candidate.label == "ctx.packages")
            .expect("`ctx.pa` offers `ctx.packages`");
        let descriptor = expressions::ctx_descriptor("packages").unwrap();
        assert_eq!(packages.kind, CompletionItemKind::VARIABLE);
        // D3: the rendered type rides in `detail`, the description in
        // `documentation`, and the inserted text is the fully qualified name.
        assert_eq!(packages.detail.as_deref(), Some("string[]"));
        assert_eq!(packages.documentation.as_deref(), Some(descriptor.description));
        assert_eq!(packages.new_text, "ctx.packages");
        // Removed `*_list` aliases are gone from the catalog, so none is offered.
        assert!(candidates.iter().all(|candidate| !candidate.label.ends_with("_list")));
    }

    #[test]
    fn function_completion_derives_all_fields_from_one_descriptor() {
        let candidates = interpolation_candidates("len", &[]);
        let length = candidates
            .iter()
            .find(|candidate| candidate.new_text == "length")
            .expect("`len` offers `length`");
        let descriptor = expressions::function_descriptor("length").unwrap();
        assert_eq!(length.kind, CompletionItemKind::FUNCTION);
        // D4: untyped signature label, typed-signature detail, description
        // documentation, bare-name insertion (no snippet, no parentheses).
        assert_eq!(length.label, descriptor.signature);
        assert_eq!(length.detail, Some(descriptor.typed_signature()));
        assert_eq!(length.documentation.as_deref(), Some(descriptor.description));
        assert!(!length.new_text.contains('('));
    }

    #[test]
    fn all_formatting_functions_present_with_fallible_error_detail() {
        let candidates = interpolation_candidates("", &[]);
        for name in [
            "as_csv",
            "as_tsv",
            "as_space_separated",
            "as_line_separated",
            "as_unordered_list",
            "as_ordered_list",
        ] {
            assert!(
                candidates.iter().any(|candidate| candidate.new_text == name),
                "formatting function `{name}` missing from completion"
            );
        }
        // At least one fallible function's `detail` carries the `| error` suffix.
        let length = candidates
            .iter()
            .find(|candidate| candidate.new_text == "length")
            .unwrap();
        assert!(length.detail.as_deref().unwrap().ends_with("| error"));
    }

    #[test]
    fn completion_matching_is_prefix_based_and_case_sensitive() {
        let keys = vec!["title".to_string()];
        // Frontmatter keys remain available alongside `ctx.*` and functions.
        let candidates = interpolation_candidates("ti", &keys);
        assert!(candidates
            .iter()
            .any(|candidate| candidate.label == "title" && candidate.kind == CompletionItemKind::FIELD));
        // Case-sensitive: an uppercase partial matches nothing.
        assert!(interpolation_candidates("CTX.", &keys).is_empty());
        assert!(interpolation_candidates("Ti", &keys).is_empty());
    }

    #[test]
    fn period_outside_open_interpolation_offers_nothing() {
        // The `completion` router only reaches interpolation candidates through
        // `completion_partial`, whose open-interpolation guard rejects both a
        // period in plain prose and one after a closed `{{ … }}`.
        let prose = "plain prose ctx.";
        assert!(expressions::completion_partial(prose, prose.len()).is_none());
        let closed = "x {{ a }} ctx.";
        assert!(expressions::completion_partial(closed, closed.len()).is_none());
    }

    #[test]
    fn literal_provider_diagnostics_and_hover_respect_utf16_and_body_boundary() {
        let text = "---\nexample: '{{{ frontmatter }}}'\n---\n\n😀 See {{{ > invalid }}} here.\n";
        let uri: Uri = "file:///literal.md".parse().unwrap();
        let path = Path::new("/literal.md");
        let source_map = SourceMap::new(
            uri.clone(),
            7,
            PositionEncoding::Utf16,
            Arc::from(text),
        );
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1);
        let config = DmlsConfig::default();
        let profile = ClientProfile {
            client_name: None,
            client_version: None,
            position_encoding: PositionEncoding::Utf16,
            supports_resource_operations: false,
            supports_change_annotations: false,
            supports_code_action_resolve: false,
            supports_completion_resolve: false,
            resolve_provides_text_edit: false,
            supports_snippets: false,
            client_watches_files: false,
            needs_watch_fallback: false,
            supports_file_operations: false,
            supports_workspace_configuration: false,
            supports_folding: false,
            folding_line_only: false,
            supports_selection_range: false,
            supports_linked_editing: false,
            supports_work_done_progress: false,
            hover_media: HoverMediaProfile::default(),
            helix_one_char_selection_is_empty: false,
        };
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: None,
        };
        let registry = ProviderRegistry::with_substrate();

        let diagnostics = registry.diagnostics(&ctx);
        assert!(
            diagnostics.iter().all(|diagnostic| !matches!(
                &diagnostic.code,
                Some(NumberOrString::String(code)) if code.starts_with("dm.expression.")
            )),
            "literal produced expression diagnostics: {diagnostics:#?}"
        );

        let frontmatter_cursor = text.find("frontmatter").unwrap();
        assert!(
            crate::providers::dsl::hover(&ctx, frontmatter_cursor).is_none(),
            "DSL literal hover must not cross the frontmatter/body boundary"
        );

        let literal_start = text.find("{{{ > invalid }}}").unwrap();
        let cursor = source_map
            .lsp_to_byte(Position::new(4, 13))
            .expect("UTF-16 cursor inside the body literal");
        let hover = registry.hover(&ctx, cursor).expect("literal hover");
        assert_eq!(
            hover.range,
            source_map.byte_range_to_lsp(literal_start..literal_start + "{{{ > invalid }}}".len())
        );
        assert_eq!(hover.range.unwrap().start, Position::new(4, 7));
        match hover.contents {
            HoverContents::Markup(MarkupContent { kind, value }) => {
                assert_eq!(kind, MarkupKind::Markdown);
                assert!(value.contains("**Interpolation literal**"));
                assert!(value.contains("`{{ > invalid }}`"));
                assert!(value.contains("not interpolated"));
            }
            other => panic!("literal hover must use Markdown markup, got {other:?}"),
        }
    }

    #[test]
    fn literal_provider_hover_safely_preserves_backticks_multiline_and_unicode() {
        let text = "Backticks: {{{ use `x` and ``y`` }}}\n\nMultiline: {{{ first\nsecond }}}\n\nUnicode: {{{ café 😀 東京 }}}\n";
        let uri: Uri = "file:///literal-content.md".parse().unwrap();
        let path = Path::new("/literal-content.md");
        let source_map = SourceMap::new(
            uri.clone(),
            8,
            PositionEncoding::Utf16,
            Arc::from(text),
        );
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1);
        let config = DmlsConfig::default();
        let profile = ClientProfile {
            client_name: None,
            client_version: None,
            position_encoding: PositionEncoding::Utf16,
            supports_resource_operations: false,
            supports_change_annotations: false,
            supports_code_action_resolve: false,
            supports_completion_resolve: false,
            resolve_provides_text_edit: false,
            supports_snippets: false,
            client_watches_files: false,
            needs_watch_fallback: false,
            supports_file_operations: false,
            supports_workspace_configuration: false,
            supports_folding: false,
            folding_line_only: false,
            supports_selection_range: false,
            supports_linked_editing: false,
            supports_work_done_progress: false,
            hover_media: HoverMediaProfile::default(),
            helix_one_char_selection_is_empty: false,
        };
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: None,
        };
        let registry = ProviderRegistry::with_substrate();
        let hover_value = |literal: &str| {
            let cursor = text.find(literal).unwrap() + 3;
            let hover = registry.hover(&ctx, cursor).expect("literal hover");
            match hover.contents {
                HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }) => value,
                other => panic!("literal hover must use Markdown markup, got {other:?}"),
            }
        };

        let backticks = hover_value("{{{ use `x` and ``y`` }}}");
        assert!(backticks.contains("```{{ use `x` and ``y`` }}```"));

        let multiline = hover_value("{{{ first\nsecond }}}");
        assert!(multiline.contains("```\n{{ first\nsecond }}\n```"));

        let unicode = hover_value("{{{ café 😀 東京 }}}");
        assert!(unicode.contains("`{{ café 😀 東京 }}`"));
    }

    #[test]
    fn literal_hover_markdown_identifies_span_and_composed_output() {
        let markdown = literal_hover_markdown(" name ");
        assert!(markdown.contains("**Interpolation literal**"));
        assert!(markdown.contains("`{{ name }}`"));
        assert!(markdown.contains("not interpolated"));
    }

    #[test]
    fn literal_hover_markdown_preserves_content_with_inner_expression() {
        let markdown = literal_hover_markdown(" {{ x }} ");
        assert!(markdown.contains("**Interpolation literal**"));
        assert!(markdown.contains("`{{ {{ x }} }}`"));
        assert!(markdown.contains("not interpolated"));
    }

    #[test]
    fn text_edit_item_carries_eager_edit_and_markdown_documentation() {
        use crate::source_map::PositionEncoding;
        let text = "hello {{ ctx.pa";
        let source_map = SourceMap::new(
            "file:///t.md".parse().unwrap(),
            1,
            PositionEncoding::Utf16,
            text.into(),
        );
        let token_start = text.find("ctx.pa").unwrap();
        let item = text_edit_item(
            &source_map,
            token_start,
            text.len(),
            CompletionCandidate {
                label: "ctx.packages".to_string(),
                new_text: "ctx.packages".to_string(),
                kind: CompletionItemKind::VARIABLE,
                detail: Some("string[]".to_string()),
                documentation: Some("All package names.".to_string()),
            },
        )
        .unwrap();
        // The eager `textEdit` replaces exactly the typed token.
        let Some(CompletionTextEdit::Edit(edit)) = item.text_edit else {
            panic!("expected an eager text edit");
        };
        assert_eq!(edit.new_text, "ctx.packages");
        assert_eq!(edit.range.start.character as usize, token_start);
        assert_eq!(edit.range.end.character as usize, text.len());
        // Documentation is eager Markdown, not deferred to resolve.
        let Some(Documentation::MarkupContent(content)) = item.documentation else {
            panic!("expected eager Markdown documentation");
        };
        assert_eq!(content.kind, MarkupKind::Markdown);
        assert_eq!(content.value, "All package names.");
        assert_eq!(item.detail.as_deref(), Some("string[]"));
    }
}
