//! The expression-family severity ladder (spec D7) across the frontmatter and
//! body producers, and pending-value deferral before parsing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lsp_types::{CodeActionOrCommand, InitializeParams, Uri};

use super::*;
use crate::capabilities::ClientProfile;
use crate::config::DmlsConfig;
use crate::graph::WorkspaceGraph;
use crate::overlay::OverlayState;
use crate::source_map::PositionEncoding;

fn with_ctx<R>(text: &str, f: impl FnOnce(&DocumentContext) -> R) -> R {
    let path = Path::new("/w/doc.md");
    let uri: Uri = "file:///w/doc.md".parse().unwrap();
    let config = DmlsConfig::default();
    let roots = [PathBuf::from("/w")];
    let state = OverlayState::default();
    let overlay = state.for_document(&uri, text, path, &config, &roots);
    let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
    let graph = WorkspaceGraph::build(&BTreeMap::new(), 1);
    let profile =
        ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
    let ctx = DocumentContext {
        uri: &uri,
        path,
        text,
        source_map: &source_map,
        graph: &graph,
        doc_id: None,
        config: &config,
        profile: &profile,
        overlay: overlay.as_ref(),
    };
    f(&ctx)
}

/// `(code, severity)` for every expression-family diagnostic from both producers.
fn expression_severities(ctx: &DocumentContext) -> Vec<(String, DiagnosticSeverity, &'static str)> {
    let frontmatter = diagnostics(ctx).into_iter().map(|d| (d, "frontmatter"));
    let body = crate::providers::dsl::diagnostics(ctx).into_iter().map(|d| (d, "body"));
    frontmatter
        .chain(body)
        .filter_map(|(diagnostic, producer)| match diagnostic.code {
            Some(NumberOrString::String(code)) if code.starts_with("dm.expression.") => {
                Some((code, diagnostic.severity.expect("severity"), producer))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn ladder_separates_schema_typed_frontmatter_from_body_inference() {
    let text = concat!(
        "---\n",
        "$schema:\n  when: expression\n  guard: expression\n",
        "when: '1 +'\n",
        "guard: mystery\n",
        "---\n\n{{ 1 + }} and {{ other_mystery }}\n",
    );
    with_ctx(text, |ctx| {
        let mut found = expression_severities(ctx);
        found.sort_by_key(|(code, _, producer)| (producer.to_string(), code.clone()));
        assert_eq!(
            found,
            vec![
                (code::EXPRESSION_MALFORMED.to_string(), DiagnosticSeverity::WARNING, "body"),
                (code::EXPRESSION_UNKNOWN_IDENTIFIER.to_string(), DiagnosticSeverity::WARNING, "body"),
                (code::EXPRESSION_MALFORMED.to_string(), DiagnosticSeverity::ERROR, "frontmatter"),
                (
                    code::EXPRESSION_UNKNOWN_IDENTIFIER.to_string(),
                    DiagnosticSeverity::WARNING,
                    "frontmatter"
                ),
            ]
        );
    });
}

#[test]
fn foreign_template_syntax_in_a_body_is_at_most_a_warning() {
    let text = concat!(
        "---\ntitle: templates\n---\n\n",
        "Handlebars: {{#each items}}{{this}}{{/each}} and {{> partial}}\n\n",
        "Liquid: {{ name | upcase }} and {% if user %}hi{% endif %}\n\n",
        "Jinja: {{ user.name|default('guest') }} and {{ items|length }}\n",
    );
    with_ctx(text, |ctx| {
        let found = expression_severities(ctx);
        assert!(
            found.iter().any(|(code, _, _)| code == code::EXPRESSION_MALFORMED),
            "the foreign syntax is still reported: {found:#?}"
        );
        assert!(found.iter().all(|(_, severity, _)| *severity != DiagnosticSeverity::ERROR), "{found:#?}");
    });
}

#[test]
fn pending_values_are_deferred_before_parsing() {
    for value in ["when: '{{ ctx.area }}'", "when: \"$(git branch) == 'main'\""] {
        let text = format!("---\n$schema:\n  when: expression\n{value}\n---\n\nbody\n");
        with_ctx(&text, |ctx| {
            let all = diagnostics(ctx);
            assert!(all.is_empty(), "{value}: {all:#?}");

            // Independent of the accepted-validation guard: hand the expression
            // pass a report that rejects `/when`, as a malformed value would.
            let ast = ctx.overlay.and_then(|overlay| overlay.ast.as_deref()).unwrap();
            let bundle = ctx.overlay.and_then(|overlay| overlay.bundle()).unwrap();
            let rejecting = bundle.effective.validate_with_options(
                &serde_json::json!({ "when": "1 +" }),
                &PositionMap::new(),
                &ValidationOptions::default(),
            );
            assert!(rejecting.problems.iter().any(|problem| problem.path == "/when"));
            let mut out = Vec::new();
            expression_diagnostics(ctx, ast, &rejecting, &mut out);
            assert!(out.is_empty(), "{value}: {out:#?}");
        });
    }
}

#[test]
fn existing_malformed_code_action_is_offered_at_error_severity() {
    let text = "# Doc\n\n{{ > invalid }}\n";
    with_ctx(text, |ctx| {
        let mut malformed = crate::providers::dsl::diagnostics(ctx)
            .into_iter()
            .find(|d| matches!(&d.code, Some(NumberOrString::String(c)) if c == code::EXPRESSION_MALFORMED))
            .expect("malformed diagnostic");
        malformed.severity = Some(DiagnosticSeverity::ERROR);
        let titles: Vec<String> = crate::providers::code_actions::code_actions(ctx, &[malformed])
            .into_iter()
            .filter_map(|action| match action {
                CodeActionOrCommand::CodeAction(action) => Some(action.title),
                CodeActionOrCommand::Command(_) => None,
            })
            .collect();
        assert_eq!(titles, vec!["Wrap in interpolation literal".to_string()]);
    });
}
