//! Editor surfaces of `::map` / `::filter` regions: header completions,
//! hover, folds, and diagnostics.
//!
//! Every type here comes from the library's shared region analysis through
//! [`crate::overlay::collections`]: this module decides what to show where,
//! never what a binding's type is. It is called from the DSL provider, so the
//! request routers stay thin.

use darkmatter::markdown::compose::collection_blocks::{
    CollectionRegion, ExpressionRole, scan_collection_regions_lenient,
};
use darkmatter::markdown::compose::expression::{
    BindingView, DeclaredTypes, Expr, LexicalTypes, Shape, SourceProjection, TypeScope, TypedLocal, identifier_prefix_start,
};
use lsp_types::{CompletionItem, CompletionItemKind, Diagnostic, DiagnosticSeverity, FoldingRange, Hover};

use super::DocumentContext;
use super::dsl::{
    CompletionCandidate, diagnostic, frontmatter_scalar, host_catalogs, lower_candidates, markup_hover, push_line_fold,
    schema_property_hover, text_edit_item, with_site_scope,
};
use super::frontmatter;
use crate::diagnostics::call_check;
use crate::diagnostics::codes::source;
use crate::overlay::collections::{self, authored_sites};
use crate::overlay::{directives, expressions};

// ── Folding ─────────────────────────────────────────────────────────────────

/// One fold per paired `::map` / `::filter` region, nested ones included.
pub(super) fn folds(ctx: &DocumentContext, folds: &mut Vec<FoldingRange>) {
    if !collections::may_contain_collections(ctx.text) {
        return;
    }
    fn walk(ctx: &DocumentContext, regions: &[CollectionRegion], folds: &mut Vec<FoldingRange>) {
        for region in regions {
            push_line_fold(ctx, folds, region.start_line, region.end_line);
            walk(ctx, &region.children, folds);
        }
    }
    walk(ctx, &scan_collection_regions_lenient(ctx.text).regions, folds);
}

// ── Diagnostics ─────────────────────────────────────────────────────────────

/// The static findings of every collection region: header, structural, and
/// typing defects as the library reports them, then the call, unknown
/// function, and gate checks of every callback under the scope it is
/// evaluated in.
pub(super) fn diagnostics(
    ctx: &DocumentContext,
    view: Option<&BindingView>,
    declared: &DeclaredTypes<'_>,
    strict: bool,
    out: &mut Vec<Diagnostic>,
) {
    if !collections::may_contain_collections(ctx.text) {
        return;
    }
    let site = TypeScope::unguarded(declared);
    for finding in collections::region_findings(ctx.text, &site, view) {
        if let Some(range) = ctx.source_map.byte_range_to_lsp(finding.span) {
            let severity = if finding.warning { DiagnosticSeverity::WARNING } else { DiagnosticSeverity::ERROR };
            out.push(diagnostic(range, severity, &finding.code, source::COMPOSE, finding.message));
        }
    }

    let scopes = collections::scopes(ctx.text);
    let scan = scan_collection_regions_lenient(ctx.text);
    for authored in authored_sites(&scopes, &scan, &site) {
        let text = &authored.site.text;
        for found in call_check::call_diagnostics(&authored.site.expression, &authored.site.scope, strict) {
            if let Some(range) = ctx.source_map.byte_range_to_lsp(authored.authored(found.span.clone())) {
                out.push(call_check::lsp_diagnostic(range, source::COMPOSE, found));
            }
        }
        for (name, span) in expressions::unknown_function_calls(&authored.site.expression, text) {
            if let Some(range) = ctx.source_map.byte_range_to_lsp(authored.authored(span)) {
                out.push(diagnostic(
                    range,
                    DiagnosticSeverity::ERROR,
                    crate::diagnostics::codes::code::EXPRESSION_UNKNOWN_FUNCTION,
                    source::COMPOSE,
                    expressions::unknown_function_message(&name),
                ));
            }
        }
    }
}

// ── Hover ───────────────────────────────────────────────────────────────────

/// Hover on a collection opener: its source (declared and entry types), a
/// callback expression (the shared expression hover at the callback's own
/// scope), or a callback parameter. `None` on the keyword and elsewhere on
/// the line, which the catalog hover describes.
pub(super) fn hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    if !collections::may_contain_collections(ctx.text) {
        return None;
    }
    let scan = scan_collection_regions_lenient(ctx.text);
    let header = scan.headers.iter().find(|header| {
        header.directive.span.start <= offset && offset <= header.directive.span.end
    })?;
    let directive = &header.directive;
    if directive.keyword_span.contains(&offset) {
        return None;
    }
    let scopes = collections::scopes(ctx.text);

    if let Some(target) = directive.target.as_ref().filter(|target| target.span.contains(&offset)) {
        let lines = with_site_scope(ctx, Some(directive.span.start), |outer| source_hover_lines(&target.value, outer));
        return Some(markup_hover(ctx, target.span.clone(), lines));
    }

    with_site_scope(ctx, None, |unguarded| {
        let sites = authored_sites(&scopes, &scan, unguarded);
        let authored = sites.iter().find(|authored| authored.quoted.span.contains(&offset))?;
        let decoded = authored.decoded_offset(offset)?;
        let site = &authored.site;
        let body = &site.expression.span;
        if decoded >= body.start && decoded <= body.end {
            let host = super::dsl::host_catalogs(ctx);
            let value = expressions::hover_markdown_parsed(
                site.expression.clone(),
                &site.text,
                decoded,
                &site.scope,
                |name| frontmatter_scalar(ctx, name),
                |name| expressions::host_global_hover(name, &host).or_else(|| schema_property_hover(ctx, name)),
            );
            return Some(markup_hover(ctx, authored.quoted.span.clone(), value));
        }
        let word = identifier_at(&site.text, decoded)?;
        let ty = site.scope.type_of_expr(&Expr::Variable(word.to_string()))?;
        site.scope.lexical()?.names().contains(&word).then(|| {
            markup_hover(
                ctx,
                authored.authored(decoded..decoded + word.len().max(1)),
                format!("**Parameter** `{word}`\n\nLocal `{word}`: `{}`", expressions::type_text(&ty)),
            )
        })
    })
}

/// The Markdown for a source reference: its type at the opener and the type
/// of one entry.
fn source_hover_lines(reference: &str, outer: &TypeScope<'_>) -> String {
    let source = Expr::Variable(reference.to_string());
    let mut lines = vec![format!("**Source** `{reference}`")];
    if let Some(ty) = outer.type_of_expr(&source) {
        lines.push(format!("Type: `{}`", expressions::type_text(&ty)));
    }
    lines.push(match outer.project_source(None, &source) {
        SourceProjection::Entries(entry) => format!("Entry: `{}`", expressions::shape_text(&entry)),
        SourceProjection::UnknownShape => "No declared iterable type.".to_string(),
        SourceProjection::Unsuitable(arms) => format!(
            "Not iterable: includes {}.",
            arms.iter().map(|arm| format!("`{arm}`")).collect::<Vec<_>>().join(", ")
        ),
    });
    lines.join("\n\n")
}

/// The identifier containing byte `at` of `text`.
fn identifier_at(text: &str, at: usize) -> Option<&str> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let start = text[..at.min(text.len())].rfind(|c: char| !is_word(c)).map_or(0, |index| {
        index + text[index..].chars().next().map_or(1, char::len_utf8)
    });
    let end = text[at.min(text.len())..].find(|c: char| !is_word(c)).map_or(text.len(), |index| at + index);
    (start < end).then(|| &text[start..end])
}

// ── Completion ──────────────────────────────────────────────────────────────

/// Where the cursor is in a collection opener's header.
#[derive(Debug, PartialEq, Eq)]
enum HeaderCursor<'a> {
    /// The source token.
    Source { partial: &'a str, start: usize },
    /// An option name, with the option names already written (`""` for the
    /// positional callback).
    Option { partial: &'a str, start: usize, present: Vec<&'a str> },
    /// An unquoted option value.
    Value { key: &'a str, partial: &'a str, start: usize },
    /// Inside an unterminated quoted value: the option it belongs to, or the
    /// positional callback.
    Quoted { key: Option<&'a str>, content: &'a str, start: usize },
}

/// The end of a quoted value that starts at `open`, or `None` when the quote
/// is still open at the end of `text`.
fn quoted_end(text: &str, open: usize) -> Option<usize> {
    let quote = text[open..].chars().next()?;
    let mut escaped = false;
    for (index, character) in text[open + quote.len_utf8()..].char_indices() {
        let at = open + quote.len_utf8() + index;
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == quote {
            return Some(at + character.len_utf8());
        }
    }
    None
}

fn is_quote(character: char) -> bool {
    matches!(character, '"' | '\'')
}

/// Classifies the cursor at the end of `header` (the opener line from just
/// after the keyword to the cursor). `None` right after a closed quote.
fn header_cursor(header: &str) -> Option<HeaderCursor<'_>> {
    let mut at = 0;
    let mut tokens = 0usize;
    let mut present: Vec<&str> = Vec::new();
    loop {
        at += header[at..].len() - header[at..].trim_start().len();
        if at >= header.len() {
            return Some(if tokens == 0 {
                HeaderCursor::Source { partial: "", start: at }
            } else {
                HeaderCursor::Option { partial: "", start: at, present }
            });
        }
        let start = at;
        let word_end = header[start..]
            .find(|c: char| c.is_whitespace() || c == '=' || is_quote(c))
            .map_or(header.len(), |index| start + index);
        if header[word_end..].starts_with('=') {
            let key = &header[start..word_end];
            let value = word_end + 1;
            if header[value..].starts_with(is_quote) {
                match quoted_end(header, value) {
                    None => {
                        let open = header[value..].chars().next().map_or(1, char::len_utf8);
                        return Some(HeaderCursor::Quoted {
                            key: Some(key),
                            content: &header[value + open..],
                            start: value + open,
                        });
                    }
                    Some(end) => {
                        present.push(key);
                        tokens += 1;
                        at = end;
                    }
                }
            } else {
                let end = header[value..].find(char::is_whitespace).map_or(header.len(), |index| value + index);
                if end >= header.len() {
                    return Some(HeaderCursor::Value { key, partial: &header[value..], start: value });
                }
                present.push(key);
                tokens += 1;
                at = end;
            }
            if at >= header.len() {
                return None;
            }
        } else if word_end == start && header[start..].starts_with(is_quote) {
            match quoted_end(header, start) {
                None => {
                    let open = header[start..].chars().next().map_or(1, char::len_utf8);
                    return Some(HeaderCursor::Quoted { key: None, content: &header[start + open..], start: start + open });
                }
                Some(end) => {
                    present.push("");
                    tokens += 1;
                    at = end;
                    if at >= header.len() {
                        return None;
                    }
                }
            }
        } else {
            let end = header[start..].find(char::is_whitespace).map_or(header.len(), |index| start + index);
            if end >= header.len() {
                return Some(if tokens == 0 {
                    HeaderCursor::Source { partial: &header[start..], start }
                } else {
                    HeaderCursor::Option { partial: &header[start..], start, present }
                });
            }
            tokens += 1;
            at = end;
        }
    }
}

/// The opener keyword at the start of `line` (behind any block-quote
/// markers) and the text after it, when the line is a `::map` / `::filter`
/// opener that the cursor is past.
fn opener_header(line: &str) -> Option<(&str, &str)> {
    let mut rest = line.trim_start();
    while let Some(stripped) = rest.strip_prefix('>') {
        rest = stripped.trim_start();
    }
    ["::map", "::filter"].into_iter().find_map(|keyword| {
        let tail = rest.strip_prefix(keyword)?;
        tail.starts_with(char::is_whitespace).then_some((keyword, tail))
    })
}

/// Completions inside a `::map` / `::filter` opener line, or `None` when the
/// cursor is not in one (the other completion routes then apply).
pub(super) fn completion(ctx: &DocumentContext, offset: usize, prefix: &str) -> Option<Vec<CompletionItem>> {
    let (_, header) = opener_header(prefix)?;
    let header_start = offset - header.len();
    let line_start = offset - prefix.len();
    let cursor = header_cursor(header)?;
    let keys = frontmatter::expression_property_keys(ctx);
    let host_catalogs = host_catalogs(ctx);

    let candidates = match cursor {
        HeaderCursor::Source { partial, start } => {
            let items = with_site_scope(ctx, Some(line_start), |outer| {
                let host = expressions::host_global_candidates(partial, &host_catalogs);
                expressions::scoped_completion_candidates(partial, &keys, host, outer)
                    .into_iter()
                    .filter(|item| item.kind != expressions::ExprCompletionKind::Function)
                    .filter(|item| {
                        item.kind == expressions::ExprCompletionKind::ReservedRoot
                            || matches!(
                                outer.project_source(None, &Expr::Variable(item.insert_text.clone())),
                                SourceProjection::Entries(_)
                            )
                    })
                    .collect::<Vec<_>>()
            });
            return Some(edit_items(ctx, header_start + start, offset, lower_candidates(items)));
        }
        HeaderCursor::Option { partial, start, present } => {
            let names = directives::COLLECTION_OPTIONS
                .iter()
                .filter(|name| name.starts_with(partial))
                .filter(|name| !present.contains(name))
                .filter(|name| match **name {
                    "while" => !present.contains(&"until"),
                    "until" => !present.contains(&"while"),
                    "sort-order" => present.contains(&"sort"),
                    _ => true,
                })
                .map(|name| CompletionCandidate {
                    label: (*name).to_string(),
                    new_text: format!("{name}="),
                    kind: CompletionItemKind::PROPERTY,
                    detail: None,
                    documentation: None,
                })
                .collect();
            return Some(edit_items(ctx, header_start + start, offset, names));
        }
        HeaderCursor::Value { key, partial, start } => {
            let values: Vec<CompletionCandidate> = directives::option_values(key)
                .unwrap_or_default()
                .iter()
                .filter(|value| value.starts_with(partial))
                .map(|value| CompletionCandidate {
                    label: (*value).to_string(),
                    new_text: format!("\"{value}\""),
                    kind: CompletionItemKind::VALUE,
                    detail: None,
                    documentation: None,
                })
                .collect();
            return Some(edit_items(ctx, header_start + start, offset, values));
        }
        HeaderCursor::Quoted { key, content, start } => (key, content, header_start + start),
    };

    let (key, content, content_start) = candidates;
    if key == Some("sort-order") {
        let values = directives::option_values("sort-order")
            .unwrap_or_default()
            .iter()
            .filter(|value| value.starts_with(content))
            .map(|value| CompletionCandidate {
                label: (*value).to_string(),
                new_text: (*value).to_string(),
                kind: CompletionItemKind::VALUE,
                detail: None,
                documentation: None,
            })
            .collect();
        return Some(edit_items(ctx, content_start, offset, values));
    }

    // Every other quoted value is an expression: `when` is whole, a callback
    // is `pattern => body` and only its body completes.
    let role = match key {
        None => ExpressionRole::Positional,
        Some("when") => ExpressionRole::Gate,
        Some("filter") => ExpressionRole::Filter,
        Some("sort") => ExpressionRole::Sort,
        Some("while") => ExpressionRole::While,
        Some("until") => ExpressionRole::Until,
        Some(_) => return Some(Vec::new()),
    };
    let (expression_start, pattern) = if role == ExpressionRole::Gate {
        (0, "")
    } else {
        match content.rfind("=>").or_else(|| content.rfind("->")) {
            Some(arrow) => (arrow + 2, &content[..arrow]),
            None => return Some(Vec::new()),
        }
    };
    let expression = &content[expression_start..];
    let partial_start = identifier_prefix_start(expression);
    let partial = &expression[partial_start..];
    let token_start = content_start + expression_start + partial_start;

    let line = ctx.text[..line_start].matches('\n').count() + 1;
    let items = with_site_scope(ctx, None, |unguarded| {
        let scope = callback_scope(ctx, unguarded, line, line_start, role, pattern);
        let host = expressions::host_global_candidates(partial, &host_catalogs);
        expressions::scoped_completion_candidates(partial, &keys, host, &scope)
    });
    Some(edit_items(ctx, token_start, offset, lower_candidates(items)))
}

/// The scope a callback of role `role` of the opener at `line` is evaluated
/// in. A region whose header validates answers with the library's own scope
/// (only that callback's parameters); one that does not yet validate falls
/// back to the enclosing scope plus the parameter names written so far,
/// untyped.
fn callback_scope<'d>(
    ctx: &DocumentContext,
    unguarded: &TypeScope<'d>,
    line: usize,
    line_start: usize,
    role: ExpressionRole,
    pattern: &str,
) -> TypeScope<'d> {
    let scopes = collections::scopes(ctx.text);
    let scan = scan_collection_regions_lenient(ctx.text);
    let outer = scopes.scope_at(unguarded.clone(), line_start);
    if role == ExpressionRole::Gate {
        return outer;
    }
    let sites = authored_sites(&scopes, &scan, unguarded);
    if let Some(region) = find_region_at_line(&scan.regions, line)
        && let Some(found) = sites.into_iter().find(|authored| authored.site.region == region.span && authored.site.role == role)
    {
        return found.site.scope;
    }
    let names: Vec<TypedLocal> = pattern
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .filter(|name| !name.is_empty())
        .map(|name| TypedLocal::new(name, Shape::Unknown { non_null: false }))
        .collect();
    let layer = match outer.lexical() {
        Some(parent) => parent.child(names),
        None => LexicalTypes::new(names),
    };
    outer.within(layer)
}

fn find_region_at_line(regions: &[CollectionRegion], line: usize) -> Option<&CollectionRegion> {
    regions.iter().find_map(|region| {
        if region.start_line == line { Some(region) } else { find_region_at_line(&region.children, line) }
    })
}

fn edit_items(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    candidates: Vec<CompletionCandidate>,
) -> Vec<CompletionItem> {
    candidates.into_iter().filter_map(|candidate| text_edit_item(ctx, start, offset, candidate)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor(header: &str) -> Option<HeaderCursor<'_>> {
        header_cursor(header)
    }

    #[test]
    fn the_cursor_is_classified_at_every_header_position() {
        assert_eq!(cursor(" "), Some(HeaderCursor::Source { partial: "", start: 1 }));
        assert_eq!(cursor(" team.mem"), Some(HeaderCursor::Source { partial: "team.mem", start: 1 }));
        assert_eq!(
            cursor(" items "),
            Some(HeaderCursor::Option { partial: "", start: 7, present: Vec::new() })
        );
        assert_eq!(
            cursor(" items so"),
            Some(HeaderCursor::Option { partial: "so", start: 7, present: Vec::new() })
        );
        assert_eq!(
            cursor(" items \"p => p\" sort=\"q => q.a\" wh"),
            Some(HeaderCursor::Option { partial: "wh", start: 32, present: vec!["", "sort"] })
        );
        assert_eq!(
            cursor(" items sort-order=a"),
            Some(HeaderCursor::Value { key: "sort-order", partial: "a", start: 18 })
        );
        assert_eq!(
            cursor(" items \"é => é.na"),
            Some(HeaderCursor::Quoted { key: None, content: "é => é.na", start: 8 })
        );
        assert_eq!(
            cursor(" items when=\"a == \\\"x\\\" && b"),
            Some(HeaderCursor::Quoted { key: Some("when"), content: "a == \\\"x\\\" && b", start: 13 })
        );
        // Right after a closed quote nothing completes.
        assert_eq!(cursor(" items \"p => p\""), None);
    }

    #[test]
    fn openers_are_recognized_behind_quote_markers_and_only_with_a_boundary() {
        assert_eq!(opener_header("::map items"), Some(("::map", " items")));
        assert_eq!(opener_header("  > > ::filter x"), Some(("::filter", " x")));
        assert_eq!(opener_header("::map"), None);
        assert_eq!(opener_header("::mapping x"), None);
        assert_eq!(opener_header("::file x"), None);
    }

    #[test]
    fn identifiers_are_found_around_a_byte() {
        assert_eq!(identifier_at("{ name, é } => x", 3), Some("name"));
        assert_eq!(identifier_at("p => p", 0), Some("p"));
        assert_eq!(identifier_at("p => p", 2), None);
    }

    /// The tripwire: every editor surface over regions whose gates,
    /// callbacks, and bodies would act if run (a registered function, a
    /// shell command, a transclusion) enters no handler, spawns nothing,
    /// opens no socket, and builds no effect engine.
    #[test]
    fn collection_analysis_runs_no_callback_and_no_effect() {
        use std::collections::BTreeMap;
        use std::path::Path;
        use std::sync::Arc;

        use lsp_types::{InitializeParams, Uri};

        use crate::capabilities::ClientProfile;
        use crate::config::DmlsConfig;
        use crate::graph::WorkspaceGraph;
        use crate::overlay::OverlayState;
        use crate::source_map::{PositionEncoding, SourceMap};

        #[cfg(feature = "effects-instrumentation")]
        let before = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());
        #[cfg(feature = "effects-instrumentation")]
        let _recorded = {
            use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
            use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
            let registration = FunctionRegistration::builder("recorded", "function(parameters(void); returns(boolean))")
                .build()
                .expect("the test signature compiles");
            let guard = hooks::register(registration, Arc::new(|_| Ok(serde_json::json!(true))));
            hooks::reset_handler_entries();
            guard
        };

        let sentinel_dir = tempfile::TempDir::new().expect("a scratch directory");
        let sentinel = sentinel_dir.path().join("SENTINEL_SHOULD_NOT_EXIST").to_string_lossy().replace('\\', "/");
        let text = format!(
            concat!(
                "---\n$schema:\n  team: '{{ name: string(required), active: boolean(required) }}[](required)'\n---\n\n",
                "::map team \"p => recorded()\" when=\"recorded()\" filter=\"q => recorded()\" sort=\"r => recorded() ? 1 : 2\" until=\"u => recorded()\"\n",
                "{{{{ recorded() }}}} {{{{ p }}}}\n",
                "{{{{ map([1], item => recorded()) }}}} {{{{ filter(null, item -> recorded()) }}}}\n",
                "::shell touch {sentinel}\n",
                "::file ./missing.md\n",
                "::filter team \"t => recorded()\" while=\"w => recorded()\"\n",
                "{{{{ recorded() ? t.name : 'none' }}}}\n",
                "::end-filter\n",
                "::end-map\n",
            ),
            sentinel = sentinel,
        );

        let path = Path::new("/w/doc.md");
        let uri: Uri = "file:///w/doc.md".parse().unwrap();
        let config = DmlsConfig::default();
        let roots = [std::path::PathBuf::from("/w")];
        let resolution = crate::context::test_support::resolution_for(path);
        let overlay = OverlayState::default().for_document(&uri, &text, path, &config, &roots, &resolution);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text.as_str()));
        let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts());
        let params: InitializeParams = serde_json::from_value(serde_json::json!({
            "capabilities": { "textDocument": {
                "foldingRange": { "lineFoldingOnly": true },
                "semanticTokens": { "requests": { "full": true }, "tokenTypes": [], "tokenModifiers": [], "formats": ["relative"] }
            } }
        }))
        .unwrap();
        let profile = ClientProfile::from_initialize(&params, PositionEncoding::Utf16);
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text: &text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: overlay.as_ref(),
            resolution: &resolution,
        };

        let mut answered = 0usize;
        let mut found = crate::diagnostics::frontmatter::diagnostics(&ctx);
        found.extend(crate::providers::dsl::diagnostics(&ctx));
        for (offset, _) in text.char_indices().filter(|(offset, _)| offset % 3 == 0) {
            answered += usize::from(crate::providers::dsl::hover(&ctx, offset).is_some());
            answered += crate::providers::dsl::completion(&ctx, offset).len().min(1);
        }
        let folds = crate::providers::dsl::folding_ranges(&ctx);
        assert!(folds.iter().any(|fold| fold.start_line < fold.end_line), "{folds:?}");
        let tokens = crate::providers::semantic_tokens::semantic_tokens_full(&ctx);
        assert!(!tokens.data.is_empty());
        assert!(answered > 10, "the editor requests reached the regions: {answered}");

        assert!(!Path::new(&sentinel).exists(), "a shell command ran");
        #[cfg(feature = "effects-instrumentation")]
        {
            use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
            assert_eq!(hooks::handler_entries("recorded"), 0, "a callback or body was evaluated");
            struct NoValues;
            impl darkmatter::markdown::compose::expression::EvaluationLookup for NoValues {
                fn get(&self, _: &str) -> Option<serde_json::Value> {
                    None
                }
            }
            use darkmatter::markdown::compose::expression::{evaluate, parse};
            assert_eq!(evaluate(&parse("recorded()").expect("parses"), &NoValues).expect("evaluates"), serde_json::json!(true));
            assert_eq!(hooks::handler_entries("recorded"), 1, "the recording seam observes an evaluation");
            assert_eq!(
                (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()),
                before,
                "analysis built no effect engine and attempted no network access",
            );
        }
    }
}
