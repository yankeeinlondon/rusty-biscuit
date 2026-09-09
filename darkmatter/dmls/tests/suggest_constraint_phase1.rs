//! Level-1 in-memory LSP session tests for `suggest(...)` diagnostics/completion.
//!
//! These tests use `Connection::memory()` and an in-process server thread —
//! no real terminal or terminal harness is involved, so they are Level 1
//! integration tests and run under `just test`.

mod common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const INLINE: &str = include_str!("fixtures/suggest_constraint/inline.md");
const PURE: &str = include_str!("fixtures/suggest_constraint/pure.yaml");
const TAGGED: &str = include_str!("fixtures/suggest_constraint/tagged.yaml");
const COMPLETION: &str = include_str!("fixtures/suggest_constraint/completion.md");
const UNIONS: &str = include_str!("fixtures/suggest_constraint/unions.md");
const RAW_SCHEMA: &str = include_str!("fixtures/suggest_constraint/raw-schema.json");
const RAW_CONSUMER: &str = include_str!("fixtures/suggest_constraint/raw-consumer.md");

/// Initialize params for the suggestion sessions: a Neovim-shaped client that
/// advertises workspace configuration support.
fn suggestions_initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "clientInfo": { "name": "Neovim", "version": "0.11.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-8", "utf-16"] },
            "workspace": { "configuration": true }
        },
        "workspaceFolders": [
            { "uri": root_uri.as_str(), "name": "suggestions" }
        ]
    })
}

/// Completion labels at `line`/`character`, as the suggestion tests assert them.
fn completion(fixture: &mut LspFixture<'_>, uri: &str, line: u32, character: u32) -> Vec<Value> {
    fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
        .result
        .expect("completion response")
        .as_array()
        .cloned()
        .expect("completion array")
}

fn open(fixture: &LspFixture<'_>, uri: &str, language_id: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": text
            }
        }),
    );
}

fn assert_invalid_suggestion(diagnostic: &Value, line: u32, start: u32, end: u32) {
    assert_eq!(diagnostic["severity"], json!(2));
    assert_eq!(diagnostic["source"], json!("darkmatter.schema"));
    assert_eq!(diagnostic["code"], json!("dm.schema.invalid_suggestion"));
    assert_eq!(
        diagnostic["range"],
        json!({
            "start": { "line": line, "character": start },
            "end": { "line": line, "character": end }
        })
    );
}

fn labels(items: &[Value]) -> Vec<&str> {
    items.iter().filter_map(|item| item["label"].as_str()).collect()
}

#[test]
fn suggest_phase1_inline_warning_has_exact_argument_range() {
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("inline.md");
    std::fs::write(&path, INLINE).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", INLINE);
    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let suggestion = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_suggestion"))
        .expect("invalid suggestion diagnostic");
    assert_invalid_suggestion(suggestion, 2, 35, 39);
    fixture.shutdown();
}

#[test]
fn suggest_phase1_decoy_field_does_not_steal_diagnostic_span() {
    let text = "---\ndecoy: number(suggest(1, many, 2))\n$schema:\n  count: number(min(0); suggest(1, many, 2))\ncount: 1\n---\n\nbody\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("decoy.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);
    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());

    let suggestions: Vec<&Value> = diagnostics
        .iter()
        .filter(|d| d["code"] == json!("dm.schema.invalid_suggestion"))
        .collect();
    assert_eq!(
        suggestions.len(),
        1,
        "expected exactly one invalid suggestion diagnostic, got {diagnostics:?}"
    );

    // The decoy `many` sits at line 1, chars 25..29. The `$schema` `many`
    // sits at line 3, chars 35..39. The diagnostic must point at the latter.
    assert_invalid_suggestion(suggestions[0], 3, 35, 39);

    fixture.shutdown();
}

#[test]
fn suggest_phase1_standalone_ranges() {
    let workspace = LspWorkspace::new();
    let pure_path = workspace.path().join("pure.yaml");
    let tagged_path = workspace.path().join("tagged.yaml");
    std::fs::write(&pure_path, PURE).unwrap();
    std::fs::write(&tagged_path, TAGGED).unwrap();
    let pure_uri = url::Url::from_file_path(&pure_path).unwrap();
    let tagged_uri = url::Url::from_file_path(&tagged_path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, pure_uri.as_str(), "yaml", PURE);
    open(&fixture, tagged_uri.as_str(), "yaml", TAGGED);

    let pure_diagnostics = fixture.wait_for_diagnostics(pure_uri.as_str());
    let pure_suggestion = pure_diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_suggestion"))
        .expect("pure-envelope warning");
    assert_invalid_suggestion(pure_suggestion, 2, 35, 39);

    let tagged_diagnostics = fixture.wait_for_diagnostics(tagged_uri.as_str());
    let tagged_suggestion = tagged_diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_suggestion"))
        .expect("tagged-envelope warning");
    assert_invalid_suggestion(tagged_suggestion, 3, 35, 39);
    fixture.shutdown();
}

fn assert_whole_file_envelope_completion(schema_name: &str, schema: &str) {
    let workspace = LspWorkspace::new();
    let schema_path = workspace.path().join(schema_name);
    let consumer_path = workspace.path().join("consumer.md");
    let consumer = format!("---\n$schema: ./{schema_name}\ncolor: gr\ncount: \n---\n");
    std::fs::write(schema_path, schema).unwrap();
    std::fs::write(&consumer_path, &consumer).unwrap();
    let consumer_uri = url::Url::from_file_path(consumer_path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, consumer_uri.as_str(), "markdown", &consumer);

    let color = completion(&mut fixture, consumer_uri.as_str(), 2, 9);
    assert_eq!(labels(&color), vec!["green"]);
    assert_eq!(color[0]["textEdit"]["newText"], json!("\"green\""));
    assert_eq!(
        color[0]["textEdit"]["range"],
        json!({
            "start": { "line": 2, "character": 7 },
            "end": { "line": 2, "character": 9 }
        })
    );

    let count = completion(&mut fixture, consumer_uri.as_str(), 3, 7);
    assert_eq!(labels(&count), vec!["1", "2"]);
    assert_eq!(count[0]["textEdit"]["newText"], json!("1"));
    assert_eq!(count[1]["textEdit"]["newText"], json!("2"));
    for item in &count {
        assert_eq!(
            item["textEdit"]["range"],
            json!({
                "start": { "line": 3, "character": 7 },
                "end": { "line": 3, "character": 7 }
            })
        );
    }
    fixture.shutdown();
}

#[test]
fn suggest_phase1_pure_whole_file_reference_completion() {
    assert_whole_file_envelope_completion("pure.yaml", PURE);
}

#[test]
fn suggest_phase1_tagged_whole_file_reference_completion() {
    assert_whole_file_envelope_completion("tagged.yaml", TAGGED);
}

#[test]
fn suggest_phase1_completion_positions() {
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("completion.md");
    std::fs::write(&path, COMPLETION).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", COMPLETION);

    let scalar = completion(&mut fixture, uri.as_str(), 6, 9);
    assert_eq!(labels(&scalar), vec!["green"]);
    assert_eq!(scalar[0]["textEdit"]["newText"], json!("\"green\""));

    let nested = completion(&mut fixture, uri.as_str(), 8, 10);
    assert_eq!(labels(&nested), vec!["slow"]);
    assert_eq!(nested[0]["textEdit"]["newText"], json!("\"slow\""));

    let block_array = completion(&mut fixture, uri.as_str(), 10, 6);
    assert_eq!(labels(&block_array), vec!["alpha"]);
    assert_eq!(block_array[0]["textEdit"]["newText"], json!("\"alpha\""));

    let flow_array = completion(&mut fixture, uri.as_str(), 11, 12);
    assert_eq!(labels(&flow_array), vec!["0.25", "0.5", "1"]);
    assert_eq!(flow_array[0]["textEdit"]["newText"], json!("0.25"));
    fixture.shutdown();
}

#[test]
fn suggest_phase1_union_selection_and_raw_schema_exclusion() {
    let workspace = LspWorkspace::new();
    let union_path = workspace.path().join("unions.md");
    let raw_path = workspace.path().join("raw-schema.json");
    let consumer_path = workspace.path().join("raw-consumer.md");
    std::fs::write(&union_path, UNIONS).unwrap();
    std::fs::write(&raw_path, RAW_SCHEMA).unwrap();
    std::fs::write(&consumer_path, RAW_CONSUMER).unwrap();
    let union_uri = url::Url::from_file_path(union_path).unwrap();
    let consumer_uri = url::Url::from_file_path(consumer_path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, union_uri.as_str(), "markdown", UNIONS);
    open(&fixture, consumer_uri.as_str(), "markdown", RAW_CONSUMER);

    let property_union = completion(&mut fixture, union_uri.as_str(), 7, 10);
    assert_eq!(labels(&property_union), vec!["second"]);
    // `root` is declared in both discriminant-less root-union arms, so the
    // effective shape merges the arms per-key: the shared property becomes the
    // union of both arms' atoms and its suggestions merge in arm-declaration
    // order.
    let root_union = completion(&mut fixture, union_uri.as_str(), 8, 8);
    assert_eq!(labels(&root_union), vec!["arm-one", "arm-two"]);
    assert!(
        completion(&mut fixture, consumer_uri.as_str(), 2, 9).is_empty(),
        "raw JSON Schema annotations must not activate suggestion completion"
    );
    fixture.shutdown();
}

#[test]
fn suggest_phase1_root_union_filters_invalid_from_later_arm() {
    let text = "---\n$schema:\n  - root: string\n  - root: number(suggest(1, many, 2))\nroot: \n---\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("root-union.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);

    let items = completion(&mut fixture, uri.as_str(), 4, 6);
    let labels: Vec<&str> = items.iter().filter_map(|item| item["label"].as_str()).collect();
    assert_eq!(labels, vec!["1", "2"]);
    fixture.shutdown();
}

/// Typing a leading-zero prefix (`00`) must complete a numeric candidate
/// whose decoded text is `003.5` even though its canonical label/insert text
/// is `3.5`. Prefix filtering uses the decoded (authored) spelling; insertion
/// uses the canonical decimal.
#[test]
fn suggest_phase1_numeric_prefix_uses_decoded_text() {
    let text = "---\n$schema:\n  val: number(suggest(003.5))\nval: 00\n---\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("numeric-prefix.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);

    let items = completion(&mut fixture, uri.as_str(), 3, 7);
    assert_eq!(labels(&items), vec!["3.5"]);
    assert_eq!(items[0]["textEdit"]["newText"], json!("3.5"));
    fixture.shutdown();
}

/// A `suggest(...)` list with both valid and invalid candidates omits the
/// invalid ones from completion while keeping their valid siblings.
#[test]
fn suggest_phase1_invalid_sibling_omitted_from_completion() {
    let text = "---\n$schema:\n  count: number(min(0); suggest(1, many, 2))\ncount: \n---\n\nbody\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("invalid-sibling.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);

    let items = completion(&mut fixture, uri.as_str(), 3, 7);
    assert_eq!(labels(&items), vec!["1", "2"]);
    fixture.shutdown();
}

/// Completion immediately after a literal block-array dash inserts after the
/// dash instead of constructing an out-of-bounds edit range.
#[test]
fn suggest_phase1_bare_block_array_dash() {
    let text = "---\ntitle: café\n$schema:\n  tags: string(suggest(alpha, beta))[]\ntags:\n  -\n---\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("bare-dash.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);

    let items = completion(&mut fixture, uri.as_str(), 5, 3);
    assert_eq!(labels(&items), vec!["alpha", "beta"]);
    assert_eq!(items[0]["textEdit"]["newText"], json!("\"alpha\""));
    assert_eq!(
        items[0]["textEdit"]["range"],
        json!({
            "start": { "line": 5, "character": 3 },
            "end": { "line": 5, "character": 3 }
        })
    );
    fixture.shutdown();
}

/// Completion after a block-array dash and space inserts at the cursor while
/// preserving the marker and its separator.
#[test]
fn suggest_phase1_block_array_dash_space() {
    let text = "---\ntitle: café\n$schema:\n  tags: string(suggest(alpha, beta))[]\ntags:\n  - \n---\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("dash-space.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);

    let items = completion(&mut fixture, uri.as_str(), 5, 4);
    assert_eq!(labels(&items), vec!["alpha", "beta"]);
    assert_eq!(items[0]["textEdit"]["newText"], json!("\"alpha\""));
    assert_eq!(
        items[0]["textEdit"]["range"],
        json!({
            "start": { "line": 5, "character": 4 },
            "end": { "line": 5, "character": 4 }
        })
    );
    fixture.shutdown();
}

/// Completion after a partially typed block-array item replaces only the
/// partial value, leaving the dash and separating space intact.
#[test]
fn suggest_phase1_block_array_partial_value() {
    let text = "---\ntitle: café\n$schema:\n  tags: string(suggest(alpha, beta))[]\ntags:\n  - al\n---\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("dash-partial.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);

    let items = completion(&mut fixture, uri.as_str(), 5, 6);
    assert_eq!(labels(&items), vec!["alpha"]);
    assert_eq!(items[0]["textEdit"]["newText"], json!("\"alpha\""));
    assert_eq!(
        items[0]["textEdit"]["range"],
        json!({
            "start": { "line": 5, "character": 4 },
            "end": { "line": 5, "character": 6 }
        })
    );
    fixture.shutdown();
}

/// A nested inline-object property that exists only in a later root-union arm
/// still resolves suggestion completion.
#[test]
fn suggest_phase1_nested_property_from_later_root_arm() {
    let text = "---\n$schema:\n  - settings: string\n  - settings: \"{ mode: string(suggest(fast, slow)) }\"\nsettings:\n  mode: \n---\n\nbody\n";
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("nested-later-arm.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);

    let items = completion(&mut fixture, uri.as_str(), 5, 8);
    assert_eq!(labels(&items), vec!["fast", "slow"]);
    fixture.shutdown();
}

// ── Phase 5 session tests: lifecycle, ownership, correction ──

/// Opening an inline schema document, then correcting the invalid candidate,
/// removes the warning.
#[test]
fn suggest_phase5_inline_warning_removed_after_correction() {
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("inline.md");
    let uri = url::Url::from_file_path(&path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", INLINE);

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let suggestion = diagnostics
        .iter()
        .find(|d| d["code"] == json!("dm.schema.invalid_suggestion"))
        .expect("invalid suggestion diagnostic before correction");
    assert_invalid_suggestion(suggestion, 2, 35, 39);

    // Correct the candidate: replace `many` with `3`.
    let corrected = INLINE.replace("many", "3");
    fixture.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri.as_str(), "version": 2 },
            "contentChanges": [{ "text": corrected }]
        }),
    );

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let has_suggestion = diagnostics
        .iter()
        .any(|d| d["code"] == json!("dm.schema.invalid_suggestion"));
    assert!(!has_suggestion, "warning must be removed after correction: {diagnostics:?}");

    fixture.shutdown();
}

/// A standalone pure-envelope schema document publishes its own warnings; a
/// consuming Markdown document referencing it gets no suggestion diagnostics.
#[test]
fn suggest_phase5_standalone_warning_not_duplicated_on_consumer() {
    let workspace = LspWorkspace::new();
    let schema_path = workspace.path().join("pure.yaml");
    let consumer_path = workspace.path().join("consumer.md");
    std::fs::write(&schema_path, PURE).unwrap();
    let consumer_text = "---\n$schema: ./pure.yaml\ncount: 1\n---\n\nbody\n";
    std::fs::write(&consumer_path, consumer_text).unwrap();
    let schema_uri = url::Url::from_file_path(&schema_path).unwrap();
    let consumer_uri = url::Url::from_file_path(&consumer_path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, consumer_uri.as_str(), "markdown", consumer_text);
    open(&fixture, schema_uri.as_str(), "yaml", PURE);

    // The schema document has the warning.
    let schema_diagnostics = fixture.wait_for_diagnostics(schema_uri.as_str());
    let schema_suggestion = schema_diagnostics
        .iter()
        .find(|d| d["code"] == json!("dm.schema.invalid_suggestion"))
        .expect("standalone schema document owns the warning");
    assert_invalid_suggestion(schema_suggestion, 2, 35, 39);

    // The consuming Markdown document does NOT duplicate it.
    let consumer_diagnostics = fixture.wait_for_diagnostics(consumer_uri.as_str());
    let consumer_has_suggestion = consumer_diagnostics
        .iter()
        .any(|d| d["code"] == json!("dm.schema.invalid_suggestion"));
    assert!(
        !consumer_has_suggestion,
        "consumer must not carry the standalone schema's suggestion warning: {consumer_diagnostics:?}"
    );

    fixture.shutdown();
}

/// A malformed tagged envelope publishes a `dm.schema.document_malformed` error.
#[test]
fn suggest_phase5_malformed_tagged_envelope_error() {
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("bad.yaml");
    std::fs::write(&path, "kind: schema\n").unwrap();
    let uri = url::Url::from_file_path(&path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "yaml", "kind: schema\n");

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let malformed = diagnostics
        .iter()
        .find(|d| d["code"] == json!("dm.schema.document_malformed"))
        .expect("malformed envelope diagnostic");
    assert_eq!(malformed["severity"], json!(1), "must be ERROR severity");
    assert_eq!(malformed["source"], json!("darkmatter.schema"));

    fixture.shutdown();
}

/// Closing a standalone schema document clears its diagnostics.
#[test]
fn suggest_phase5_close_clears_diagnostics() {
    let workspace = LspWorkspace::new();
    let path = workspace.path().join("pure.yaml");
    std::fs::write(&path, PURE).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "yaml", PURE);

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == json!("dm.schema.invalid_suggestion")),
        "expected warning before close"
    );

    fixture.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": uri.as_str() } }),
    );

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert!(
        diagnostics.is_empty(),
        "closing must clear diagnostics: {diagnostics:?}"
    );

    fixture.shutdown();
}

/// The effective schema stays available after suggestion warnings so key
/// completion, hover, and validation continue operating.
#[test]
fn suggest_phase5_schema_remains_active_alongside_warnings() {
    let workspace = LspWorkspace::new();
    let text = "---\n$schema:\n  color: string(suggest(red, green, blue))\n  count: number(min(0); suggest(1, many, 2))\ncolor: red\ncount: 1\n---\n\nbody\n";
    let path = workspace.path().join("doc.md");
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap();

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(suggestions_initialize_params(workspace.path()));
    open(&fixture, uri.as_str(), "markdown", text);

    // The invalid suggestion warning is present.
    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == json!("dm.schema.invalid_suggestion")),
        "expected invalid suggestion warning"
    );

    // But schema validation is still active: violating the min(0) constraint
    // on `count` produces a constraint diagnostic.
    let changed = text.replace("count: 1\n---", "count: -5\n---");
    fixture.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri.as_str(), "version": 2 },
            "contentChanges": [{ "text": changed }]
        }),
    );
    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == json!("dm.schema.constraint")
                || d["code"] == json!("dm.schema.type_mismatch")),
        "schema validation must remain active alongside warnings: {diagnostics:?}"
    );

    // And the suggestion warning is still there.
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == json!("dm.schema.invalid_suggestion")),
        "suggestion warning must persist alongside validation: {diagnostics:?}"
    );

    fixture.shutdown();
}
