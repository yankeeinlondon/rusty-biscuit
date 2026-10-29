//! `content_policy` typed by the compiled-in Darkmatter baseline alone, in
//! DMLS: a schema-free document gets completion, hover, and diagnostics for
//! its policy list from the embedded Content Policy schema, with no schema
//! file on disk. A matching extension baseline, a trigger, and the document's
//! own `$schema` still replace the declaration, in that order; a document's
//! own `policy` type does not collide with the baseline's; and the embedded
//! documents never become watched files.

use crate::common;
use crate::union_item_editor::{
    FIELD, VALUE, completion, hover, initialize_params, labels, offset_of, open_text, request,
};

use std::path::{Path, PathBuf};

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::schemas::format::is_pending_expression_value;
use dmls::context::RepositoryContexts;
use dmls::overlay::SchemaOutcome;
use dmls::{DmlsConfig, OverlayState};
use serde_json::{Value, json};

#[path = "../../../lib/tests/content_policy_fixtures/mod.rs"]
pub(crate) mod content_policy_fixtures;

use content_policy_fixtures::{DECLARATIONS, MIXED_POLICY_FRONTMATTER, RULE_SUGGESTIONS, Verdict, render};

const ENUM_MEMBER: u64 = 20;

/// The labels of `items` that are value completions.
fn values(items: &[(String, u64, Option<String>)]) -> Vec<&str> {
    items
        .iter()
        .filter(|(_, kind, _)| *kind == VALUE)
        .map(|(label, _, _)| label.as_str())
        .collect()
}

/// A session over an empty workspace: no schema file exists anywhere in it.
fn session(workspace: &LspWorkspace) -> LspFixture<'_> {
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(workspace.path()));
    fixture
}

fn document(frontmatter: &str) -> String {
    format!("---\n{frontmatter}---\n\nBody\n")
}

/// `(line, character, instance path)` of each diagnostic: its range start and
/// the JSON Pointer it carries in `data`, sorted.
fn located_paths(diagnostics: &[Value]) -> Vec<(u64, u64, String)> {
    let mut located: Vec<(u64, u64, String)> = diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic["range"]["start"]["line"].as_u64().unwrap(),
                diagnostic["range"]["start"]["character"].as_u64().unwrap(),
                diagnostic["data"]["instance_path"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    located.sort();
    located
}

/// `(line, character, code)` of each diagnostic's range start, sorted.
fn starts(diagnostics: &[Value]) -> Vec<(u64, u64, String)> {
    let mut starts: Vec<(u64, u64, String)> = diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic["range"]["start"]["line"].as_u64().unwrap(),
                diagnostic["range"]["start"]["character"].as_u64().unwrap(),
                diagnostic["code"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    starts.sort();
    starts
}

#[test]
fn completion_offers_rule_forms_actions_and_long_form_keys_from_the_embedded_schema() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let (document, _) = open_text(
        &mut fixture,
        &workspace,
        "complete.md",
        &document(
            "content_policy:\n  - ValidFor(3mo)\n  - \n  - rule: \n    action: archive\n  - rule: TimeSensitive\n    action: \n  - rule: \"ValidFor(3mo)\"\n    \n  - action: remove\n    \n",
        ),
    );

    // A compact item: every shipped rule suggestion and both long-form keys.
    let item = completion(&mut fixture, &document, "ValidFor(3mo)\n  - ");
    assert_eq!(values(&item), RULE_SUGGESTIONS, "{item:?}");
    assert!(item.contains(&("rule".to_string(), FIELD, Some("(required)".to_string()))), "{item:?}");
    assert!(item.contains(&("action".to_string(), FIELD, Some("(required)".to_string()))), "{item:?}");

    // A long-form `rule:` value offers every shipped rule suggestion.
    let rule = completion(&mut fixture, &document, "  - rule: ");
    assert_eq!(values(&rule), RULE_SUGGESTIONS, "{rule:?}");

    // An `action:` value offers exactly the three actions.
    let action = completion(&mut fixture, &document, "rule: TimeSensitive\n    action: ");
    assert_eq!(labels(&action), ["refresh", "archive", "remove"], "{action:?}");
    assert!(action.iter().all(|(_, kind, _)| *kind == ENUM_MEMBER), "{action:?}");

    // Inside a long-form entry, the key not yet written.
    let after_rule = completion(&mut fixture, &document, "rule: \"ValidFor(3mo)\"\n    ");
    assert_eq!(labels(&after_rule), ["action"], "{after_rule:?}");
    let after_action = completion(&mut fixture, &document, "action: remove\n    ");
    assert_eq!(labels(&after_action), ["rule"], "{after_action:?}");

    fixture.shutdown();
}

#[test]
fn hover_on_compact_and_long_form_rules_shows_the_shipped_branch_descriptions() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let (document, diagnostics) =
        open_text(&mut fixture, &workspace, "hover.md", &document(MIXED_POLICY_FRONTMATTER));
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    let compact = hover(&mut fixture, &document, "ValidFor(3mo)");
    assert!(compact.contains("**`content_policy[0]`**"), "{compact}");
    assert!(compact.contains("Matches alternative 3 of 9"), "{compact}");
    assert!(compact.contains("Interval from the default date property"), "{compact}");

    let long_form = hover(&mut fixture, &document, "ValidUntil(2027-01-01)");
    assert!(long_form.contains("**`rule`**"), "{long_form}");
    for description in [
        "Never triggers",
        "Always triggers",
        "Interval from the default date property",
        "Interval from a property",
        "Interval from an inline date",
        "Fixed deadline",
        "Deadline held in a property",
        "Content change of a file, fingerprint held in a property",
    ] {
        assert!(long_form.contains(&format!("- **string** — {description}")), "{description}: {long_form}");
    }
    assert!(long_form.contains("Item `content_policy[1]` matches alternative 9 of 9:"), "{long_form}");

    let property = hover(&mut fixture, &document, "content_policy:");
    assert!(property.contains("Type: **array**, each item one of:"), "{property}");

    fixture.shutdown();
}

#[test]
fn diagnostics_address_the_offending_item_field_or_value() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let (_, diagnostics) = open_text(
        &mut fixture,
        &workspace,
        "invalid.md",
        &document(
            "content_policy:\n  - Duration(3mo)\n  - rule: \"ValidFor(3mo)\"\n    action: delete\n  - action: archive\n  - rule: null\n    action: archive\n",
        ),
    );
    let constraint = "dm.schema.constraint".to_string();
    assert_eq!(
        starts(&diagnostics),
        [
            // The unknown rule, at its item.
            (2, 4, constraint.clone()),
            // `/content_policy/1/action`, at the value.
            (4, 12, constraint.clone()),
            // No `rule`: the containing mapping.
            (5, 4, constraint.clone()),
            // `rule: null`, at the value.
            (6, 10, constraint.clone()),
        ],
        "{diagnostics:#?}"
    );
    assert_eq!(
        located_paths(&diagnostics),
        [
            (2, 4, "/content_policy/0".to_string()),
            (4, 12, "/content_policy/1/action".to_string()),
            (5, 4, "/content_policy/2".to_string()),
            (6, 10, "/content_policy/3/rule".to_string()),
        ],
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| diagnostic["source"] == json!("darkmatter.schema")));

    // An empty list, at the `content_policy` value.
    let (_, empty) = open_text(&mut fixture, &workspace, "empty.md", &document("content_policy: []\n"));
    assert_eq!(starts(&empty), [(1, 16, constraint)], "{empty:#?}");
    assert_eq!(located_paths(&empty), [(1, 16, "/content_policy".to_string())], "{empty:#?}");

    fixture.shutdown();
}

#[test]
fn the_authoring_example_and_a_policy_free_document_are_clean() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let (_, example) = open_text(&mut fixture, &workspace, "example.md", &document(MIXED_POLICY_FRONTMATTER));
    assert!(example.is_empty(), "{example:?}");
    let (_, plain) = open_text(&mut fixture, &workspace, "plain.md", &document("title: Notes\n"));
    assert!(plain.is_empty(), "{plain:?}");
    fixture.shutdown();
}

/// Every shared declaration case, in every presentation, publishes exactly
/// the baseline problems the library reports, at the authored anchor and
/// carrying the problem's instance path.
#[test]
fn every_shared_declaration_publishes_the_baseline_verdict_at_its_anchor() {
    let _ = include_str!("../../../lib/tests/content_policy_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    // One open buffer, edited per case: opening a document per case slows
    // the session superlinearly (about 15 s for this table).
    let (document, _) = open_text(&mut fixture, &workspace, "declaration.md", &document("title: start\n"));
    let mut version = 1;
    let mut checked = 0;
    for case in DECLARATIONS {
        for rendered in render(case.declaration) {
            let label = format!("{} ({:?})", case.name, rendered.presentation);
            version += 1;
            fixture.notify(
                "textDocument/didChange",
                json!({
                    "textDocument": { "uri": document.uri, "version": version },
                    "contentChanges": [{ "text": rendered.document() }]
                }),
            );
            let params = fixture.wait_for_diagnostics_params(&document.uri);
            assert_eq!(params["version"], json!(version), "{label}: the publication describes this edit");
            let diagnostics = params["diagnostics"].as_array().cloned().unwrap_or_default();
            let published = located_paths(&diagnostics);
            // DMLS validates as compose does, deferring a value that still holds
            // `{{ … }}` or `$(…)` to after interpolation, so such a declaration
            // publishes nothing here though the passive validator rejects it.
            let pending = is_pending_expression_value(&rendered.frontmatter);
            let mut expected: Vec<(u64, u64, String)> = match case.schema {
                Verdict::Reject if !pending => case
                    .problems
                    .iter()
                    .map(|problem| {
                        let (line, column) = rendered.start(problem.at);
                        (u64::from(line) - 1, u64::from(column) - 1, problem.path.to_string())
                    })
                    .collect(),
                _ => Vec::new(),
            };
            expected.sort();
            assert_eq!(published, expected, "{label}: {diagnostics:#?}");
            assert!(
                diagnostics.iter().all(|diagnostic| diagnostic["source"] == json!("darkmatter.schema")),
                "{label}: {diagnostics:#?}"
            );
            checked += 1;
        }
    }
    assert!(checked > DECLARATIONS.len(), "both presentations of the list cases ran");
    fixture.shutdown();
}

/// Layering: built-in baseline, then a matching extension baseline, then a
/// trigger, then the document's `$schema`. Each layer retypes
/// `content_policy` and the one above wins.
#[test]
fn extensions_triggers_and_document_schemas_replace_the_baseline_declaration() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    std::fs::write(
        root.join(".dmls.toml"),
        "[schema.extensions.policy]\npath = \"policy-extension.yaml\"\nglobs = [\"policy-*.md\"]\n",
    )
    .unwrap();
    std::fs::write(root.join("policy-extension.yaml"), "$schema:\n  content_policy: string\n").unwrap();
    // The trigger sits in the documents' `base_dir` schema root.
    std::fs::create_dir_all(root.join("schemas")).unwrap();
    std::fs::write(
        root.join("schemas/numbered.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  numbered: string(required)\n$schema: numbered.yaml\n",
    )
    .unwrap();
    std::fs::write(root.join("schemas/numbered.yaml"), "$schema:\n  content_policy: number\n").unwrap();
    let mut fixture = session(&workspace);
    // The `(line, character, message)` of each published diagnostic.
    let mut published = |name: &str, frontmatter: &str| -> Vec<(u64, u64, String)> {
        let (_, diagnostics) = open_text(&mut fixture, &workspace, name, &document(frontmatter));
        diagnostics
            .iter()
            .map(|diagnostic| {
                assert_eq!(diagnostic["code"], json!("dm.schema.type_mismatch"), "{diagnostic:?}");
                (
                    diagnostic["range"]["start"]["line"].as_u64().unwrap(),
                    diagnostic["range"]["start"]["character"].as_u64().unwrap(),
                    diagnostic["message"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    };
    let not_of_type = |line: u64, value: &str, ty: &str| vec![(line, 16, format!("{value} is not of type \"{ty}\""))];

    // Outside the extension's globs the built-in list type applies.
    assert_eq!(published("plain.md", "content_policy: Evergreen\n"), not_of_type(1, "\"Evergreen\"", "array"));
    // The extension's `string` replaces it.
    assert_eq!(published("policy-string.md", "content_policy: Evergreen\n"), []);
    assert_eq!(
        published("policy-list.md", "content_policy: [Evergreen]\n"),
        not_of_type(1, "[\"Evergreen\"]", "string")
    );
    // A matching trigger's `number` replaces the extension's.
    assert_eq!(published("policy-trigger.md", "content_policy: 5\nnumbered: one\n"), []);
    assert_eq!(
        published("policy-trigger-string.md", "content_policy: Evergreen\nnumbered: one\n"),
        not_of_type(1, "\"Evergreen\"", "number")
    );
    // The document's own `$schema` replaces all three.
    let declared = "$schema:\n  content_policy: boolean\nnumbered: one\n";
    assert_eq!(published("policy-document.md", &format!("{declared}content_policy: true\n")), []);
    assert_eq!(
        published("policy-document-number.md", &format!("{declared}content_policy: [Evergreen]\n")),
        not_of_type(4, "[\"Evergreen\"]", "boolean")
    );

    fixture.shutdown();
}

/// A document schema importing its own `policy`, `short_form`, and
/// `long_form` types leaves the baseline's `content_policy` typed by the
/// embedded Content Policy schema.
#[test]
fn a_document_schema_with_the_same_type_names_does_not_collide_with_the_baseline() {
    let workspace = LspWorkspace::new();
    std::fs::write(
        workspace.path().join("local-types.yaml"),
        "kind: schema\ntypes:\n  short_form: \"enum(low, high)\"\n  long_form:\n    level: short_form\n  policy:\n    - short_form\n    - long_form\n",
    )
    .unwrap();
    let mut fixture = session(&workspace);
    let schema = "$schema:\n  mine: \"policy[]@./local-types.yaml\"\n";
    let (_, valid) = open_text(
        &mut fixture,
        &workspace,
        "valid.md",
        &document(&format!("{schema}mine:\n  - low\n  - level: high\ncontent_policy:\n  - ValidFor(3mo)\n")),
    );
    assert!(valid.is_empty(), "{valid:#?}");

    let (_, crossed) = open_text(
        &mut fixture,
        &workspace,
        "crossed.md",
        &document(&format!("{schema}mine:\n  - ValidFor(3mo)\ncontent_policy:\n  - low\n")),
    );
    let constraint = "dm.schema.constraint".to_string();
    assert_eq!(starts(&crossed), [(4, 4, constraint.clone()), (6, 4, constraint)], "{crossed:#?}");

    fixture.shutdown();
}

/// The schema files a schema-free document's overlay would hash for cache
/// invalidation; none may be a virtual embedded path.
fn overlay_dependencies(workspace: &LspWorkspace, document: &Path, text: &str) -> Vec<PathBuf> {
    let contexts = RepositoryContexts::new(LspFixture::fixture_snapshot(workspace));
    let resolution = contexts.for_document(document);
    let uri: lsp_types::Uri = url::Url::from_file_path(document).unwrap().as_str().parse().unwrap();
    let overlay = OverlayState::default()
        .for_document(&uri, text, document, &DmlsConfig::default(), &[workspace.path().to_path_buf()], &resolution)
        .expect("overlay");
    let SchemaOutcome::Ready(Some(bundle)) = &overlay.schema else {
        panic!("a document with frontmatter assembles a schema");
    };
    assert!(
        bundle.effective.json_schema["properties"]["content_policy"].is_object(),
        "the built-in baseline types `content_policy`"
    );
    bundle
        .effective
        .dependencies()
        .iter()
        .chain(&bundle.extension_dependencies)
        .chain(overlay.named_types.dependencies())
        .cloned()
        .collect()
}

#[test]
fn embedded_baseline_documents_are_never_watched_dependencies() {
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("policy.md");
    let text = document(MIXED_POLICY_FRONTMATTER);
    std::fs::write(&path, &text).unwrap();

    let dependencies = overlay_dependencies(&workspace, &path, &text);
    assert!(
        dependencies.iter().all(|dependency| {
            !dependency.ends_with("content-policy/schemas/content-policy.yaml")
                && !dependency.to_string_lossy().contains("darkmatter/schemas/")
        }),
        "{dependencies:?}"
    );
    assert!(dependencies.is_empty(), "a schema-free document reads no schema file: {dependencies:?}");

    // Through the protocol as well: the session asks to watch no embedded path.
    let mut fixture = session(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "policy.md", &text);
    let _ = request(&mut fixture, "textDocument/hover", &document, offset_of(&document.text, "ValidFor", false));
    let registrations = serde_json::to_string(
        &fixture
            .buffered_server_requests()
            .iter()
            .map(|request| request.params.clone())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(
        !registrations.contains("content-policy.yaml") && !registrations.contains("darkmatter.yaml"),
        "{registrations}"
    );
    fixture.shutdown();
}
