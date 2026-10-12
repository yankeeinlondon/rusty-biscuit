//! DMLS completes and validates `style.markdown` from the same baseline
//! declarations the library ships, and agrees with the runtime rule
//! validator (`darkmatter::markdown::cleanup::rules`) about which values are
//! usable.

use crate::common;

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::cleanup::rules::{RULE_NAMES, validate_rule};
use serde_json::{Value, json};

/// The shipped control document: every rule set to a valid non-default value.
const CONTROL: &str = include_str!("../../../lib/tests/fixtures/clean/style-markdown-control.md");

fn initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": {
            "general": { "positionEncodings": ["utf-8", "utf-16"] }
        },
        "workspaceFolders": [{ "uri": root_uri.as_str(), "name": "scratch" }]
    })
}

fn open(fixture: &LspFixture<'_>, uri: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text }
        }),
    );
}

fn labels(fixture: &mut LspFixture<'_>, uri: &str, line: u32, character: u32) -> Vec<String> {
    fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
        .result
        .expect("completion result")
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|item| item["label"].as_str().map(str::to_string))
        .collect()
}

fn diagnostics_on_line(diagnostics: &[Value], line: u64) -> Vec<&Value> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["range"]["start"]["line"] == json!(line))
        .collect()
}

#[test]
fn style_markdown_completion_and_validation_agree_with_the_rules() {
    let workspace = LspWorkspace::new();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));

    // The control document is clean: no diagnostic anywhere in its
    // frontmatter, so valid rules are neither unknown keys nor schema errors.
    let control_uri = url::Url::from_file_path(workspace.path().join("control.md")).unwrap();
    open(&fixture, control_uri.as_str(), CONTROL);
    let diagnostics = fixture.wait_for_diagnostics(control_uri.as_str());
    let frontmatter_lines = CONTROL.lines().skip(1).position(|line| line == "---").unwrap() as u64 + 1;
    let in_frontmatter: Vec<&Value> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["range"]["start"]["line"].as_u64().unwrap() <= frontmatter_lines)
        .collect();
    assert!(in_frontmatter.is_empty(), "control rules must validate cleanly: {in_frontmatter:?}");

    // Key completion under `style.markdown` offers every rule not yet
    // written.
    let keys_doc = "---\nstyle:\n  markdown:\n    unwrap: true\n    \n---\n\n# Doc\n";
    let keys_uri = url::Url::from_file_path(workspace.path().join("keys.md")).unwrap();
    open(&fixture, keys_uri.as_str(), keys_doc);
    let offered = labels(&mut fixture, keys_uri.as_str(), 4, 4);
    for rule in RULE_NAMES.into_iter().filter(|rule| *rule != "unwrap") {
        assert!(offered.iter().any(|label| label == rule), "missing `{rule}` completion: {offered:?}");
    }
    assert!(!offered.iter().any(|label| label == "unwrap"), "present key offered: {offered:?}");

    // Value completion offers the enum members.
    let values_doc = "---\nstyle:\n  markdown:\n    list-interior-padding: \n---\n\n# Doc\n";
    let values_uri = url::Url::from_file_path(workspace.path().join("values.md")).unwrap();
    open(&fixture, values_uri.as_str(), values_doc);
    let offered = labels(&mut fixture, values_uri.as_str(), 3, 27);
    for member in ["preserve", "none", "except-nested-lists", "all"] {
        assert!(offered.iter().any(|label| label == member), "missing `{member}`: {offered:?}");
    }

    // Every value the runtime validator rejects is also flagged by DMLS
    // validation, on its own line; every value it accepts is not.
    let cases = [
        ("indentation", "3", false),
        ("indentation", "\"4\"", false),
        ("indentation", "preserve", true),
        ("unwrap", "\"true\"", false),
        ("unwrap", "true", true),
        ("fixed-width", "\"80\"", false),
        ("fixed-width", "0", false),
        ("fixed-width", "true", true),
        ("default-language", "5", false),
        ("default-language", "\"\"", true),
        ("list-item-padding", "two", false),
        ("bogus", "true", false),
    ];
    for (index, (rule, value, usable)) in cases.into_iter().enumerate() {
        let parsed: serde_json::Value =
            serde_json::to_value(serde_yaml_ng::from_str::<serde_yaml_ng::Value>(value).unwrap()).unwrap();
        assert_eq!(validate_rule(rule, &parsed).is_ok(), usable, "runtime: {rule}: {value}");

        let document = format!("---\nstyle:\n  markdown:\n    {rule}: {value}\n---\n\n# Doc\n");
        let uri = url::Url::from_file_path(workspace.path().join(format!("case-{index}.md"))).unwrap();
        open(&fixture, uri.as_str(), &document);
        let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
        let flagged = !diagnostics_on_line(&diagnostics, 3).is_empty();
        assert_eq!(flagged, !usable, "DMLS validation of {rule}: {value}: {diagnostics:?}");
    }

    fixture.shutdown();
}
