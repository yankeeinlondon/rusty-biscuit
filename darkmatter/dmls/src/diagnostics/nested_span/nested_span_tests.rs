//! `dm.expression.nested_span_in_literal` end to end through the frontmatter
//! producer and the `Rewrite with + concatenation` quick fix, with Claudine's
//! catalog (from `claudine/schemas/`) active as a schema extension.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::compose::expression::lint_expression;
use darkmatter::markdown::span::SourceSpan;
use lsp_types::{CodeActionOrCommand, InitializeParams, Uri};

use super::lifecycle::{EVENT_KEYS, LOOP_ITERATION_KEYS, STRUCTURAL_ITEM_KEYS};
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


/// Runs `f` with a context for `text` at client document `version`.
fn with_ctx<R>(text: &str, version: i32, f: impl FnOnce(&DocumentContext) -> R) -> R {
    let path = Path::new("/w/prompts/doc.md");
    let uri: Uri = "file:///w/prompts/doc.md".parse().unwrap();
    let schemas = tempfile::tempdir().expect("temp dir");
    let mut config = DmlsConfig::default();
    config.schema.extensions.insert(
        "claudine".to_string(),
        SchemaExtensionConfig {
            path: crate::context::test_support::claudine_payload_extension(
                schemas.path(),
                &[
                    ("claudine.yaml", CLAUDINE_TRIGGER),
                    ("lifecycle.yaml", CLAUDINE_LIFECYCLE),
                    ("action.yaml", CLAUDINE_ACTION),
                    ("linking.yaml", CLAUDINE_LINKING),
                ],
            ),
            globs: vec!["**/*.md".to_string()],
        },
    );
    let roots = [PathBuf::from("/w")];
    let state = OverlayState::default();
    let overlay = state.for_document(&uri, text, path, &config, &roots, &crate::context::test_support::resolution_for(path));
    let source_map = SourceMap::new(uri.clone(), version, PositionEncoding::Utf16, Arc::from(text));
    let graph = WorkspaceGraph::build(&BTreeMap::new(), 1, &crate::context::test_support::workspace_contexts());
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
        resolution: &crate::context::test_support::resolution_for(path),
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
fn diagnostics_use_decoded_escape_parity_for_literal_openers() {
    // One document, one stack item per case: item `k`'s expression is on
    // line `3 + 2k`.
    let cases = [(1, 0), (2, 0), (3, 1), (4, 1), (5, 0), (6, 0)];
    let mut text = String::from("---\nsuccess:\n");
    let mut expressions = Vec::new();
    for (authored_backslashes, _) in cases {
        let expression = format!(
            "{{{{ '{}{{{{ name }}}}' }}}}",
            "\\".repeat(authored_backslashes)
        );
        text.push_str(&format!("    - say: |-\n        {expression}\n"));
        expressions.push(expression);
    }
    text.push_str("---\n\nbody\n");
    with_ctx(&text, 1, |ctx| {
        let found = nested(ctx);
        for (k, ((_, expected_diagnostics), expression)) in cases.iter().zip(&expressions).enumerate() {
            let line = 3 + 2 * k as u32;
            let on_line: Vec<&Diagnostic> =
                found.iter().filter(|diagnostic| diagnostic.range.start.line == line).collect();
            assert_eq!(on_line.len(), *expected_diagnostics, "{expression}: {found:#?}");
            for diagnostic in on_line {
                assert_eq!(ranged(ctx, diagnostic), "{{ name }}", "{expression}");
            }
        }
        let expected_total: usize = cases.iter().map(|(_, expected)| expected).sum();
        assert_eq!(found.len(), expected_total, "{found:#?}");
    });
}

#[test]
fn mixed_strings_and_non_lifecycle_whole_values_are_not_flagged() {
    let text = concat!(
        "---\n",
        "resides_in: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "success:\n",
        "    say: \"done {{ ok ? 'a {{ x }}' : 'b' }} now\"\n",
        "loop:\n",
        "    action: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "---\n\nbody\n",
    );
    with_ctx(text, 1, |ctx| assert!(nested(ctx).is_empty(), "{:#?}", nested(ctx)));
}

#[test]
fn item_operands_proxy_with_values_and_predicates_are_diagnosed() {
    let text = concat!(
        "---\n",
        "initialize:\n",
        "    - when: \"mode == 'a {{ x }}'\"\n",
        "      then:\n",
        "          - ensure_file: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "          - proxy:\n",
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
            vec![(2, "{{ x }}"), (4, "{{ x }}"), (8, "{{ y }}"), (10, "{{ w }}")],
            "{found:#?}"
        );
        assert!(found.iter().all(|d| d.severity == Some(DiagnosticSeverity::ERROR)));
    });
}

/// Every new-grammar stack shape reaches the diagnostic: a root dictionary, a
/// `then` dictionary body, an `else` list body with a nested guard, a
/// long-form parameter, and `loop.gate` (guard and body). A long-form
/// `no_error` flag is never a surface.
#[test]
fn root_dictionaries_branch_bodies_and_the_loop_gate_are_diagnosed() {
    let text = concat!(
        "---\n",
        "start:\n",
        "    say: \"{{ ok ? 'a {{ r }}' : 'b' }}\"\n",
        "failure:\n",
        "    - when: \"err.code == 'cap'\"\n",
        "      then:\n",
        "          warn: \"{{ ok ? 'a {{ t }}' : 'b' }}\"\n",
        "      else:\n",
        "          - when: \"mode == 'n {{ n }}'\"\n",
        "            then:\n",
        "                - shell:\n",
        "                      command: \"{{ ok ? 'a {{ s }}' : 'b' }}\"\n",
        "                      no_error: true\n",
        "loop:\n",
        "    while: \"fm.i < 3\"\n",
        "    gate:\n",
        "        - when: \"mode == 'g {{ g }}'\"\n",
        "          then:\n",
        "              - info: \"{{ ok ? 'a {{ i }}' : 'b' }}\"\n",
        "---\n\nbody\n",
    );
    with_ctx(text, 1, |ctx| {
        let found = nested(ctx);
        let mut summary: Vec<(u32, &str)> =
            found.iter().map(|d| (d.range.start.line, ranged(ctx, d))).collect();
        summary.sort();
        assert_eq!(
            summary,
            vec![
                (2, "{{ r }}"),
                (6, "{{ t }}"),
                (8, "{{ n }}"),
                (11, "{{ s }}"),
                (16, "{{ g }}"),
                (18, "{{ i }}"),
            ],
            "{found:#?}"
        );
    });
}

/// Loose verb keys at the loop root are a dictionary stack; the loop's
/// iteration controls are not lifecycle surfaces.
#[test]
fn loose_loop_verbs_are_diagnosed_and_iteration_controls_are_not() {
    let text = concat!(
        "---\n",
        "loop:\n",
        "    while: \"fm.i < 3\"\n",
        "    action: \"{{ ok ? 'a {{ x }}' : 'b' }}\"\n",
        "    stderr: \"{{ ok ? 'a {{ l }}' : 'b' }}\"\n",
        "---\n\nbody\n",
    );
    with_ctx(text, 1, |ctx| {
        let found = nested(ctx);
        let summary: Vec<(u32, &str)> =
            found.iter().map(|d| (d.range.start.line, ranged(ctx, d))).collect();
        assert_eq!(summary, vec![(4, "{{ l }}")], "{found:#?}");
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

// Claudine's authoritative schemas; DMLS has no Claudine dependency, so the
// static inventory is held against the YAML itself.
const CLAUDINE_TRIGGER: &str = include_str!("../../../../../claudine/schemas/claudine.yaml");
const CLAUDINE_LIFECYCLE: &str = include_str!("../../../../../claudine/schemas/lifecycle.yaml");
const CLAUDINE_ACTION: &str = include_str!("../../../../../claudine/schemas/action.yaml");
const CLAUDINE_LINKING: &str = include_str!("../../../../../claudine/schemas/linking.yaml");

/// The mapping under `section` (`$schema` of a trigger, `types` of a library).
fn schema_section(text: &str, section: &str) -> serde_yaml_ng::Mapping {
    let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(text).expect("schema YAML");
    document.get(section).and_then(|value| value.as_mapping()).expect(section).clone()
}

/// Literal property names of an object type, without `$constraints`.
fn keys_of(mapping: &serde_yaml_ng::Mapping) -> BTreeSet<String> {
    mapping
        .keys()
        .filter_map(|key| key.as_str())
        .filter(|key| !key.starts_with(['<', '$']))
        .map(str::to_string)
        .collect()
}

#[test]
fn lifecycle_inventory_matches_the_authored_claudine_schema() {
    let payload = schema_section(CLAUDINE_TRIGGER, "$schema");
    let events: BTreeSet<String> = payload
        .iter()
        .filter(|(_, value)| {
            value.as_str().is_some_and(|definition| {
                ["stack@", "initialize-stack@", "loop@"].iter().any(|prefix| definition.starts_with(prefix))
            })
        })
        .filter_map(|(key, _)| key.as_str().map(str::to_string))
        .collect();
    assert_eq!(events, EVENT_KEYS.iter().map(|key| key.to_string()).collect());

    let actions = schema_section(CLAUDINE_ACTION, "types");
    let lifecycle = schema_section(CLAUDINE_LIFECYCLE, "types");
    let object = |types: &serde_yaml_ng::Mapping, name: &str| -> serde_yaml_ng::Mapping {
        types.get(name).and_then(|value| value.as_mapping()).unwrap_or_else(|| panic!("{name} is an object")).clone()
    };
    let definition = |types: &serde_yaml_ng::Mapping, name: &str, key: &str| -> String {
        object(types, name).get(key).and_then(|value| value.as_str()).unwrap_or_else(|| panic!("{name}.{key}")).to_string()
    };
    let verbs: BTreeSet<String> = ["communicate", "flow", "shell", "expression"]
        .into_iter()
        .flat_map(|category| keys_of(&object(&actions, category)))
        .collect();

    // A conditional item contributes `when`/`then`/`else`; a dictionary bundle
    // contributes `when` and `no_error` beside its verbs.
    let structural: BTreeSet<String> = STRUCTURAL_ITEM_KEYS.iter().map(|key| key.to_string()).collect();
    let mut authored_structural = BTreeSet::new();
    for conditional in ["conditional", "initialize-conditional", "cancelled-conditional"] {
        let keys = keys_of(&object(&lifecycle, conditional));
        assert!(keys.is_subset(&structural), "{conditional}: {keys:?}");
        assert!(definition(&lifecycle, conditional, "when").starts_with("expression"), "{conditional}");
        authored_structural.extend(keys);
    }
    for bundle in ["bundle", "initialize-bundle"] {
        let keys = keys_of(&object(&actions, bundle));
        let grammar: BTreeSet<String> = keys.difference(&verbs).cloned().collect();
        assert!(grammar.is_subset(&structural), "{bundle}: {grammar:?}");
        assert!(definition(&actions, bundle, "when").starts_with("expression"), "{bundle}");
        authored_structural.extend(grammar);
    }
    assert_eq!(authored_structural, structural);

    let loop_keys = keys_of(&object(&lifecycle, "loop"));
    // `fail_fast` is removed grammar: the parser still owns it so it can reject
    // it, so the schema must not offer it.
    let removed = "fail_fast";
    assert!(LOOP_ITERATION_KEYS.contains(&removed) && !loop_keys.contains(removed), "{loop_keys:?}");
    let mut loop_grammar: BTreeSet<String> =
        LOOP_ITERATION_KEYS.iter().filter(|key| **key != removed).map(|key| key.to_string()).collect();
    loop_grammar.extend(["gate".to_string(), "no_error".to_string()]);
    // `max` is both an iteration key and an expression-function verb name, so
    // the grammar is checked in both directions rather than as a difference.
    assert!(loop_grammar.is_subset(&loop_keys), "{loop_keys:?}");
    let loose: BTreeSet<String> = loop_keys.difference(&loop_grammar).cloned().collect();
    assert!(loose.is_subset(&verbs), "loop keys that are neither grammar nor verbs: {:?}", loose.difference(&verbs));
    assert!(definition(&lifecycle, "loop", "gate").starts_with("stack "), "loop.gate is a lifecycle stack");
    assert!(definition(&lifecycle, "loop", "while").starts_with("expression"));
    assert!(definition(&lifecycle, "loop", "until").starts_with("expression"));

    // Every communication verb is a single-pass surface wherever an event can
    // hold it: short form, long-form `text`, dictionary bundle, branch body,
    // and loose loop verb. Its long-form `no_error` flag is not text.
    let communicate = keys_of(&object(&actions, "communicate"));
    assert!(communicate.contains("say") && communicate.len() >= 10, "{communicate:?}");
    for verb in &communicate {
        let verb = verb.as_str();
        assert!(loop_keys.contains(verb), "loop accepts {verb}");
        for event in EVENT_KEYS.iter().copied().filter(|event| *event != "loop") {
            for path in [
                vec![Key(event), Index(0), Key(verb)],
                vec![Key(event), Index(0), Key(verb), Key("text")],
                vec![Key(event), Key(verb)],
                vec![Key(event), Index(0), Key("then"), Index(0), Key(verb)],
                vec![Key(event), Index(0), Key("else"), Key(verb)],
            ] {
                assert!(lifecycle::is_single_pass_value(&path), "{path:?}");
            }
            let flag = [Key(event), Index(0), Key(verb), Key("no_error")];
            assert!(!lifecycle::is_single_pass_value(&flag), "{flag:?}");
        }
        assert!(lifecycle::is_single_pass_value(&[Key("loop"), Key(verb)]), "loop.{verb}");
        assert!(lifecycle::is_single_pass_value(&[Key("loop"), Key("gate"), Index(0), Key(verb)]), "loop.gate.{verb}");
    }
}

#[test]
fn path_matcher_classifies_representative_lifecycle_paths() {
    let single_pass = [
        // Root dictionary.
        vec![Key("success"), Key("say")],
        vec![Key("success"), Key("effect")],
        vec![Key("start"), Key("shell"), Key("command")],
        // List items: short form, argument list, long form, `set`, `with`.
        vec![Key("initialize"), Index(0), Key("message")],
        vec![Key("start"), Index(1), Key("ensure_file"), Index(2)],
        vec![Key("start"), Index(1), Key("shell"), Key("command")],
        vec![Key("success"), Index(0), Key("set"), Key("no_error")],
        vec![Key("failure"), Index(0), Key("proxy"), Key("with"), Key("log")],
        vec![Key("failure"), Index(0), Key("retry"), Key("with"), Key("reason")],
        // Branch bodies: list and dictionary, nested.
        vec![Key("initialize"), Index(0), Key("then"), Index(0), Key("message")],
        vec![Key("failure"), Index(0), Key("else"), Index(1), Key("shell"), Key("command")],
        vec![Key("failure"), Index(0), Key("then"), Key("warn")],
        vec![
            Key("failure"), Index(0), Key("else"), Index(0), Key("else"), Index(0), Key("then"), Index(0),
            Key("stop"),
        ],
        // Loop: loose verbs and `gate`.
        vec![Key("loop"), Key("stdout")],
        vec![Key("loop"), Key("gate"), Index(0), Key("effect")],
        vec![Key("loop"), Key("gate"), Index(0), Key("then"), Index(0), Key("warn")],
    ];
    for path in &single_pass {
        assert!(lifecycle::is_single_pass_value(path), "{path:?}");
        assert!(!lifecycle::is_predicate(path), "{path:?}");
    }
    let not_single_pass = [
        vec![Key("success")],
        vec![Key("resides_in")],
        vec![Key("loop"), Key("action")],
        vec![Key("loop"), Key("max")],
        vec![Key("loop"), Key("gate")],
        vec![Key("initialize"), Index(0)],
        vec![Key("initialize"), Index(0), Key("when")],
        vec![Key("initialize"), Index(0), Key("no_error")],
        vec![Key("initialize"), Index(0), Key("then")],
        vec![Key("start"), Index(0), Key("shell"), Key("no_error")],
        vec![Key("start"), Key("no_error")],
        vec![Key("failure"), Index(0), Key("else"), Index(0), Key("when")],
        vec![Key("meta"), Key("success"), Key("say")],
    ];
    for path in &not_single_pass {
        assert!(!lifecycle::is_single_pass_value(path), "{path:?}");
    }
    for predicate in [
        vec![Key("blocked"), Index(3), Key("when")],
        vec![Key("failure"), Index(0), Key("else"), Index(1), Key("when")],
        vec![Key("failure"), Index(0), Key("then"), Index(0), Key("else"), Index(0), Key("when")],
        vec![Key("loop"), Key("gate"), Index(0), Key("when")],
        vec![Key("loop"), Key("gate"), Index(0), Key("then"), Index(2), Key("when")],
        vec![Key("loop"), Key("while")],
        vec![Key("loop"), Key("until")],
    ] {
        assert!(lifecycle::is_predicate(&predicate), "{predicate:?}");
    }
    assert!(!lifecycle::is_predicate(&[Key("when")]));
    assert!(!lifecycle::is_predicate(&[Key("success"), Key("until")]));
    // A dictionary of verbs never carries `when`.
    assert!(!lifecycle::is_predicate(&[Key("success"), Key("when")]));
    assert!(!lifecycle::is_predicate(&[Key("loop"), Key("when")]));
}

// ── Host globals at the Darkmatter baseline ─────────────────────────────────

fn undeclared_properties(ctx: &DocumentContext) -> Vec<String> {
    let mut all = crate::diagnostics::frontmatter::diagnostics(ctx);
    all.extend(crate::providers::dsl::diagnostics(ctx));
    all.into_iter()
        .filter(|d| {
            matches!(&d.code, Some(NumberOrString::String(c)) if c == code::EXPRESSION_UNDECLARED_PROPERTY)
        })
        .map(|d| d.message)
        .collect()
}

/// DMLS supplies no host descriptors (R7b; Claudine's are parked with R7c), so
/// a Claudine lifecycle global is an undeclared document property wherever it
/// appears, beneath a lifecycle event or not: one advisory, never an error.
/// `current` is a Darkmatter reserved namespace, so it is never reported.
#[test]
fn host_globals_are_undeclared_properties_beneath_lifecycle_keys_and_elsewhere() {
    let lifecycle = concat!(
        "---\n",
        "failure:\n",
        "    - when: err\n",
        "      then: [stop]\n",
        "    - when: timing\n",
        "      then: [stop]\n",
        "loop:\n",
        "    while: current\n",
        "---\n\nbody\n",
    );
    let elsewhere = concat!(
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
    for text in [lifecycle, elsewhere] {
        with_ctx(text, 1, |ctx| {
            let found = undeclared_properties(ctx);
            for root in ["err", "timing"] {
                let reports: Vec<_> =
                    found.iter().filter(|message| message.starts_with(&format!("`{root}`"))).collect();
                assert_eq!(reports.len(), 1, "{root}: {found:#?}");
                assert!(reports[0].contains("is an undeclared document property"), "{reports:?}");
            }
            assert!(!found.iter().any(|message| message.starts_with("`current`")), "{found:#?}");
            assert_eq!(found.len(), 2, "{found:#?}");
        });
    }
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
