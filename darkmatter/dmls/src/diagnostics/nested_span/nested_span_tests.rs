//! `dm.expression.nested_span_in_literal` end to end through the frontmatter
//! producer and the `Rewrite with + concatenation` quick fix, with the shipped
//! Claudine extension active.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::expression::lint_expression;
use darkmatter::markdown::span::SourceSpan;
use lsp_types::{CodeActionOrCommand, InitializeParams, Uri};

use super::lifecycle::{COMMUNICATION_FIELDS, EVENT_KEYS};
use super::*;
use crate::capabilities::ClientProfile;
use crate::config::{DmlsConfig, SchemaExtensionConfig};
use crate::graph::WorkspaceGraph;
use crate::overlay::FmPathSegment::{Index, Key};
use crate::overlay::OverlayState;
use crate::providers::code_actions::code_actions;
use crate::source_map::{PositionEncoding, SourceMap};

const INCIDENT: &str = include_str!(
    "../../../../../claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md"
);

fn schemas_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/schemas")
}

/// Runs `f` with a context for `text` at client document `version`.
fn with_ctx<R>(text: &str, version: i32, f: impl FnOnce(&DocumentContext) -> R) -> R {
    let path = Path::new("/w/prompts/doc.md");
    let uri: Uri = "file:///w/prompts/doc.md".parse().unwrap();
    let mut config = DmlsConfig::default();
    config.schema.extensions.insert(
        "claudine".to_string(),
        SchemaExtensionConfig {
            path: schemas_dir().join("claudine.yaml"),
            globs: vec!["**/*.md".to_string()],
        },
    );
    let roots = [PathBuf::from("/w")];
    let state = OverlayState::default();
    let overlay = state.for_document(&uri, text, path, &config, &roots);
    let source_map = SourceMap::new(uri.clone(), version, PositionEncoding::Utf16, Arc::from(text));
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

fn is_nested(diagnostic: &Diagnostic) -> bool {
    matches!(
        &diagnostic.code,
        Some(NumberOrString::String(code)) if code == code::EXPRESSION_NESTED_SPAN_IN_LITERAL
    )
}

/// Nested-span diagnostics from every producer (frontmatter and body).
fn nested(ctx: &DocumentContext) -> Vec<Diagnostic> {
    let mut all = crate::diagnostics::frontmatter::diagnostics(ctx);
    all.extend(crate::providers::dsl::diagnostics(ctx));
    all.into_iter().filter(is_nested).collect()
}

fn ranged<'t>(ctx: &DocumentContext<'t>, diagnostic: &Diagnostic) -> &'t str {
    &ctx.text[ctx.source_map.lsp_range_to_byte(diagnostic.range).expect("range")]
}

/// The quick-fix titles offered for one diagnostic.
fn titles(ctx: &DocumentContext, diagnostic: &Diagnostic) -> Vec<String> {
    code_actions(ctx, std::slice::from_ref(diagnostic))
        .into_iter()
        .map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.title,
            CodeActionOrCommand::Command(command) => command.title,
        })
        .collect()
}

/// Applies every offered rewrite to the document and returns the result.
fn apply_quick_fixes(ctx: &DocumentContext) -> String {
    let mut edits: Vec<(SourceSpan, String)> = Vec::new();
    for diagnostic in nested(ctx) {
        for action in code_actions(ctx, &[diagnostic]) {
            let CodeActionOrCommand::CodeAction(action) = action else {
                continue;
            };
            assert_eq!(action.title, "Rewrite with + concatenation");
            assert_eq!(action.kind, Some(lsp_types::CodeActionKind::QUICKFIX));
            let edit = action.edit.expect("edit");
            for edit in edit.changes.as_ref().and_then(|c| c.get(ctx.uri)).expect("document edits") {
                let span = ctx.source_map.lsp_range_to_byte(edit.range).expect("edit range");
                edits.push((span, edit.new_text.clone()));
            }
        }
    }
    edits.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
    let mut edited = ctx.text.to_string();
    for (span, new_text) in edits {
        edited.replace_range(span, &new_text);
    }
    edited
}

// ── Where the diagnostic fires ──────────────────────────────────────────────

#[test]
fn incident_yields_one_error_per_say_on_the_inner_span() {
    with_ctx(INCIDENT, 1, |ctx| {
        let found = nested(ctx);
        assert_eq!(found.len(), 2, "{found:#?}");
        let ast = ctx.overlay.and_then(|overlay| overlay.ast.as_deref()).unwrap();
        for (diagnostic, key, inner) in [
            (&found[0], "success.say", "{{ctx.area}}"),
            (&found[1], "failure.say", "{{ title_case(without_date(parent_dir(spec))) }}"),
        ] {
            assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
            assert_eq!(diagnostic.source.as_deref(), Some(source::FRONTMATTER));
            assert_eq!(ranged(ctx, diagnostic), inner);
            let block = ast.entry_by_dotted(key).unwrap().value_span.clone();
            let span = ctx.source_map.lsp_range_to_byte(diagnostic.range).unwrap();
            assert!(block.start < span.start && span.end <= block.end, "{key}: {span:?}");
        }
        assert_eq!(
            found[0].message,
            "`{{ctx.area}}` inside a quoted string is literal text and is never interpolated here; concatenate with `+` instead"
        );
    });
}

#[test]
fn the_same_defect_in_a_body_produces_no_diagnostic() {
    let text = "---\ntitle: body only\n---\n\n{{ ok ? \"done in {{ ctx.repo }}\" : \"no\" }}\n";
    with_ctx(text, 1, |ctx| assert!(nested(ctx).is_empty()));
}

#[test]
fn mixed_strings_and_non_lifecycle_whole_values_are_not_flagged() {
    let text = concat!(
        "---\n",
        "resides_in: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "success:\n",
        "    say: \"done {{ ok ? 'a {{ x }}' : 'b' }} now\"\n",
        "    effect: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "loop:\n",
        "    action: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "---\n\nbody\n",
    );
    with_ctx(text, 1, |ctx| assert!(nested(ctx).is_empty(), "{:#?}", nested(ctx)));
}

#[test]
fn stack_operands_proxy_with_values_and_predicates_are_diagnosed() {
    let text = concat!(
        "---\n",
        "initialize:\n",
        "    stack:\n",
        "        - when: \"mode == 'a {{ x }}'\"\n",
        "          action:\n",
        "              - ensure_file: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "              - action: proxy\n",
        "                target: ./next.md\n",
        "                with:\n",
        "                    review: \"{{ ok ? 'c {{ y }}' : 'd' }}\"\n",
        "loop:\n",
        "    until: \"mode == 'z {{ w }}'\"\n",
        "---\n\nbody\n",
    );
    with_ctx(text, 1, |ctx| {
        let found = nested(ctx);
        // Predicates are reported by the schema pass, whole values after it.
        let mut summary: Vec<(u32, &str)> =
            found.iter().map(|d| (d.range.start.line, ranged(ctx, d))).collect();
        summary.sort();
        assert_eq!(
            summary,
            vec![(3, "{{ x }}"), (5, "{{ x }}"), (9, "{{ y }}"), (11, "{{ w }}")],
            "{found:#?}"
        );
        assert!(found.iter().all(|d| d.severity == Some(DiagnosticSeverity::ERROR)));
    });
}

#[test]
fn ranges_are_inner_for_exact_styles_and_whole_for_folded_and_tagged() {
    let text = concat!(
        "---\n",
        "start:\n",
        "    say: '{{ ok ? \"a {{ x }}\" : ''b'' }}'\n",
        "    message: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "    info: |-\n        {{ ok ? 'a {{ x }}' : 'b' }}\n",
        "    warn: >-\n        {{ ok ? 'a {{ x }}' : 'b' }}\n",
        "    notify: !!str \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "---\n\nbody\n",
    );
    with_ctx(text, 1, |ctx| {
        let found = nested(ctx);
        let ranges: Vec<&str> = found.iter().map(|d| ranged(ctx, d)).collect();
        assert_eq!(
            ranges,
            vec![
                "{{ x }}",
                "{{ x }}",
                "{{ x }}",
                ">-\n        {{ ok ? 'a {{ x }}' : 'b' }}\n",
                "\"{{ ok ? 'a {{ x }}' : 'b' }}\"",
            ]
        );
        let offered: Vec<bool> = found.iter().map(|d| d.data.is_some()).collect();
        assert_eq!(offered, vec![true, true, true, false, false]);
    });
}

// ── Lifecycle inventory ─────────────────────────────────────────────────────

fn schema_mapping(file: &str) -> serde_yaml_ng::Mapping {
    let text = std::fs::read_to_string(schemas_dir().join(file)).expect("read schema");
    let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("schema YAML");
    document.get("$schema").and_then(|schema| schema.as_mapping()).expect("$schema").clone()
}

fn keys_of(mapping: &serde_yaml_ng::Mapping) -> BTreeSet<String> {
    mapping.keys().filter_map(|key| key.as_str()).map(str::to_string).collect()
}

#[test]
fn lifecycle_inventory_matches_the_authored_claudine_schema() {
    let root = schema_mapping("claudine.yaml");
    let events: BTreeSet<String> = root
        .iter()
        .filter(|(_, value)| {
            value.as_str().is_some_and(|definition| {
                definition.starts_with("lifecycle-event@") || definition.starts_with("loop-event@")
            })
        })
        .filter_map(|(key, _)| key.as_str().map(str::to_string))
        .collect();
    assert_eq!(events, EVENT_KEYS.iter().map(|key| key.to_string()).collect());

    let types = schema_mapping("claudine-types.yaml");
    let object = |name: &str| types.get(name).and_then(|value| value.as_mapping()).unwrap().clone();
    let communication: BTreeSet<String> =
        COMMUNICATION_FIELDS.iter().map(|field| field.to_string()).collect();
    let mut event_fields = keys_of(&object("lifecycle-event"));
    for structural in ["effect", "stack"] {
        assert!(event_fields.remove(structural), "{structural}");
    }
    assert_eq!(event_fields, communication);
    assert!(keys_of(&object("loop-event")).is_superset(&communication));

    let is_expression = |name: &str, key: &str| {
        object(name).get(key).and_then(|value| value.as_str()).is_some_and(|d| d.starts_with("expression"))
    };
    assert!(is_expression("lifecycle-stack-item", "when"));
    assert!(is_expression("loop-event", "while"));
    assert!(is_expression("loop-event", "until"));
}

#[test]
fn path_matcher_classifies_representative_lifecycle_paths() {
    let single_pass = [
        vec![Key("success"), Key("say")],
        vec![Key("loop"), Key("stdout")],
        vec![Key("initialize"), Key("stack"), Index(0), Key("action")],
        vec![Key("start"), Key("stack"), Index(1), Key("action"), Index(2), Key("ensure_file")],
        vec![Key("failure"), Key("stack"), Index(0), Key("action"), Index(0), Key("with"), Key("log")],
        vec![Key("finalize"), Key("stack"), Index(0), Key("stdout")],
    ];
    for path in &single_pass {
        assert!(lifecycle::is_single_pass_value(path), "{path:?}");
        assert!(lifecycle::is_beneath_event(path), "{path:?}");
    }
    let not_single_pass = [
        vec![Key("success")],
        vec![Key("success"), Key("effect")],
        vec![Key("resides_in")],
        vec![Key("loop"), Key("action")],
        vec![Key("initialize"), Key("stack"), Index(0)],
        vec![Key("initialize"), Key("stack"), Index(0), Key("when")],
        vec![Key("initialize"), Key("stack"), Index(0), Key("no_error")],
        vec![Key("meta"), Key("success"), Key("say")],
    ];
    for path in &not_single_pass {
        assert!(!lifecycle::is_single_pass_value(path), "{path:?}");
    }
    assert!(lifecycle::is_predicate(&[Key("blocked"), Key("stack"), Index(3), Key("when")]));
    assert!(lifecycle::is_predicate(&[Key("loop"), Key("while")]));
    assert!(!lifecycle::is_predicate(&[Key("when")]));
    assert!(!lifecycle::is_predicate(&[Key("success"), Key("until")]));
    assert!(!lifecycle::is_beneath_event(&[Key("success")]));
}

// ── Late-binding roots ──────────────────────────────────────────────────────

fn unknown_identifiers(ctx: &DocumentContext) -> Vec<String> {
    let mut all = crate::diagnostics::frontmatter::diagnostics(ctx);
    all.extend(crate::providers::dsl::diagnostics(ctx));
    all.into_iter()
        .filter(|d| {
            matches!(&d.code, Some(NumberOrString::String(c)) if c == code::EXPRESSION_UNKNOWN_IDENTIFIER)
        })
        .map(|d| d.message)
        .collect()
}

#[test]
fn late_binding_roots_are_known_beneath_lifecycle_keys() {
    let text = concat!(
        "---\n",
        "failure:\n",
        "    stack:\n",
        "        - when: err\n",
        "          action: stop\n",
        "        - when: timing\n",
        "          action: stop\n",
        "loop:\n",
        "    while: current\n",
        "---\n\nbody\n",
    );
    with_ctx(text, 1, |ctx| assert_eq!(unknown_identifiers(ctx), Vec::<String>::new()));
}

#[test]
fn late_binding_roots_are_unknown_outside_lifecycle_keys() {
    let text = concat!(
        "---\n",
        "$schema:\n",
        "    check: expression\n",
        "    meta:\n",
        "        when: expression\n",
        "check: err\n",
        "meta:\n",
        "    when: timing\n",
        "---\n\n{{ current }}\n",
    );
    with_ctx(text, 1, |ctx| {
        let found = unknown_identifiers(ctx);
        for root in ["err", "timing", "current"] {
            assert!(found.iter().any(|message| message.starts_with(&format!("`{root}`"))), "{root}: {found:#?}");
        }
    });
}

// ── Quick fix ───────────────────────────────────────────────────────────────

#[test]
fn quick_fix_rewrites_the_incident_literal_blocks_and_nothing_else() {
    let edited = with_ctx(INCIDENT, 7, apply_quick_fixes);
    assert_ne!(edited, INCIDENT);
    with_ctx(&edited, 8, |ctx| {
        let overlay = ctx.overlay.unwrap();
        assert!(overlay.parse_error.is_none());
        assert!(nested(ctx).is_empty(), "{edited}");
        let say = overlay.ast.as_deref().unwrap().entry_by_dotted("success.say").unwrap();
        let authored = &edited[say.value_span.clone()];
        assert!(authored.starts_with("|-\n        {{\n        ctx.area\n"), "{authored}");
    });
    let prefix = INCIDENT.find("success:").unwrap();
    assert_eq!(&edited[..prefix], &INCIDENT[..prefix]);
    let between = INCIDENT.find("    message: \"✅").unwrap()..INCIDENT.find("failure:").unwrap();
    assert!(edited.contains(&INCIDENT[between]));
    let suffix = INCIDENT.find("    message: \"💥").unwrap();
    assert!(edited.ends_with(&INCIDENT[suffix..]));
}

#[test]
fn incident_rewrite_is_the_darkmatter_lint_suggestion_verbatim() {
    with_ctx(INCIDENT, 1, |ctx| {
        let found = nested(ctx);
        for diagnostic in &found {
            let payload = NestedSpanRewrite::from_data(diagnostic.data.as_ref().unwrap()).unwrap();
            let inner = &ctx.text[ctx.source_map.lsp_range_to_byte(payload.range).unwrap()];
            let lints = lint_expression(inner, ParseMode::Interpolation);
            assert!(!lints.is_empty(), "{inner}");
            assert_eq!(Some(payload.new_text.as_str()), lints[0].suggestion.as_deref());
        }
    });
}

#[test]
fn quick_fix_encodes_quoted_replacements_for_their_style() {
    for (authored, expected) in [
        (
            "---\nstart:\n    say: '{{ ok ? \"a {{ x }}\" : ''b'' }}'\n---\n\nbody\n",
            "---\nstart:\n    say: '{{ ok ? \"a \" + x : ''b'' }}'\n---\n\nbody\n",
        ),
        (
            "---\nstart:\n    say: \"{{ ok ? 'a {{ x }}' : \\\"b\\\" }}\"\n---\n\nbody\n",
            "---\nstart:\n    say: \"{{ ok ? 'a ' + x : \\\"b\\\" }}\"\n---\n\nbody\n",
        ),
    ] {
        let edited = with_ctx(authored, 1, apply_quick_fixes);
        assert_eq!(edited, expected);
        with_ctx(&edited, 2, |ctx| {
            assert!(ctx.overlay.unwrap().parse_error.is_none());
            assert!(nested(ctx).is_empty());
        });
    }
}

#[test]
fn folded_tagged_and_line_spanning_literals_offer_no_action() {
    let text = concat!(
        "---\n",
        "start:\n",
        "    warn: >-\n        {{ ok ? 'a {{ x }}' : 'b' }}\n",
        "    notify: !!str \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "    say: |-\n        {{ ok ? \"done {{ x }}\n            later\" : \"b\" }}\n",
        "---\n\nbody\n",
    );
    with_ctx(text, 1, |ctx| {
        let found = nested(ctx);
        assert_eq!(found.len(), 3, "{found:#?}");
        for diagnostic in &found {
            assert!(diagnostic.data.is_none());
            assert!(titles(ctx, diagnostic).is_empty());
        }
    });
}

#[test]
fn stale_versions_and_foreign_payloads_are_declined() {
    let text = "---\nstart:\n    say: '{{ ok ? \"a {{ x }}\" : ''b'' }}'\n---\n\nbody\n";
    let diagnostic = with_ctx(text, 3, |ctx| {
        let diagnostic = nested(ctx).remove(0);
        assert_eq!(titles(ctx, &diagnostic), vec!["Rewrite with + concatenation".to_string()]);
        diagnostic
    });
    with_ctx(text, 4, |ctx| assert!(titles(ctx, &diagnostic).is_empty()));
    with_ctx(text, 3, |ctx| {
        let data = diagnostic.data.clone().unwrap();
        for (field, value) in [
            ("version", serde_json::json!(REWRITE_VERSION + 1)),
            ("action", serde_json::json!("dm.expression.other")),
        ] {
            let mut foreign = diagnostic.clone();
            let mut payload = data.clone();
            payload[field] = value;
            foreign.data = Some(payload);
            assert!(titles(ctx, &foreign).is_empty(), "{field}");
        }
        // The message alone is never enough.
        let mut bare = diagnostic.clone();
        bare.data = None;
        assert!(titles(ctx, &bare).is_empty());
    });
}

