//! Standalone schema documents written with an indentless block sequence or a
//! plain scalar continued across physical lines, seen through the editor:
//! malformed-definition diagnostics, hover, and completion, plus inline
//! frontmatter `$schema` definition errors ranged the same way.
//!
//! Every range is asserted in the negotiated position encoding against the
//! original authored buffer (LF and CRLF alike), so a byte offset reported as
//! an editor column, or a range anchored at the start of a folded scalar
//! instead of its offending token, fails here.

use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

/// The specification's example: an unknown constraint on the continuation
/// line of a folded type expression.
const BOGUS: &str = "kind: schema\ntypes:\n  t: string(min(1);\n    bogus(2))\n";

fn initialize_params(root: &std::path::Path, encoding: &str) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "clientInfo": { "name": "Neovim", "version": "0.11.0" },
        "capabilities": {
            "general": { "positionEncodings": [encoding] },
            "workspace": { "configuration": true }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
    })
}

fn start<'w>(workspace: &'w LspWorkspace, encoding: &str) -> LspFixture<'w> {
    let mut fixture = LspFixture::start(workspace);
    let result = fixture.initialize(initialize_params(workspace.path(), encoding));
    assert_eq!(result["capabilities"]["positionEncoding"], json!(encoding));
    fixture
}

/// Writes `text` to `name` in the workspace and opens it, returning its URI.
fn open(fixture: &LspFixture<'_>, workspace: &LspWorkspace, name: &str, text: &str) -> String {
    let path = workspace.path().join(name);
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap().to_string();
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": { "uri": uri, "languageId": "yaml", "version": 1, "text": text }
        }),
    );
    uri
}

/// The LSP position of byte `offset` in `text`, in the named encoding.
fn position(text: &str, offset: usize, encoding: &str) -> Value {
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line = text[..offset].matches('\n').count();
    let prefix = &text[line_start..offset];
    let character = match encoding {
        "utf-8" => prefix.len(),
        "utf-16" => prefix.encode_utf16().count(),
        other => panic!("unsupported encoding {other}"),
    };
    json!({ "line": line, "character": character })
}

/// The LSP range of the `occurrence`-th (zero-based) `needle` in `text`.
fn range_of(text: &str, needle: &str, occurrence: usize, encoding: &str) -> Value {
    let start = text
        .match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("fixture lacks occurrence {occurrence} of {needle:?}"))
        .0;
    json!({
        "start": position(text, start, encoding),
        "end": position(text, start + needle.len(), encoding),
    })
}

fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

fn definition_diagnostics(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture
        .wait_for_diagnostics(uri)
        .into_iter()
        .filter(|diagnostic| diagnostic["source"] == json!("darkmatter.schema"))
        .collect()
}

fn hover(fixture: &mut LspFixture<'_>, uri: &str, at: Value) -> String {
    fixture
        .request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": uri }, "position": at }),
        )
        .result
        .expect("hover result")["contents"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The completion item labelled `label` at `at`, or `None` when none is offered.
fn completion(fixture: &mut LspFixture<'_>, uri: &str, at: Value, label: &str) -> Option<Value> {
    let items = fixture
        .request(
            "textDocument/completion",
            json!({ "textDocument": { "uri": uri }, "position": at }),
        )
        .result
        .expect("completion result");
    items
        .as_array()
        .expect("completion array")
        .iter()
        .find(|item| item["label"] == json!(label))
        .cloned()
}

// ── Malformed definitions ──────────────────────────────────────────────────

/// `(file, text, offending token, occurrence)` for every malformed folded
/// definition: both envelopes, a nested mapping property, an indentless union
/// arm, and a sibling definition after the fold that is itself valid.
fn malformed_cases() -> Vec<(&'static str, String, &'static str, usize)> {
    vec![
        ("spec.yaml", BOGUS.to_string(), "bogus", 0),
        (
            "single-line.yaml",
            "kind: schema\ntypes:\n  t: string(min(1); bogus(2))\n".to_string(),
            "bogus",
            0,
        ),
        ("spec-crlf.yaml", crlf(BOGUS), "bogus", 0),
        (
            "sibling.yaml",
            "kind: schema\ntypes:\n  t: string(min(1);\n    bogus(2)) -> a folded\n    description\n  next: boolean\n"
                .to_string(),
            "bogus",
            0,
        ),
        (
            "pure-nested.yaml",
            "$schema:\n  meta:\n    level: number(min(1);\n      bogus(2))\n  next: boolean\n"
                .to_string(),
            "bogus",
            0,
        ),
        (
            "indentless-arm.yaml",
            "kind: schema\ntypes:\n  t:\n  - string\n  - number(min(1);\n    bogus(2))\n  next: boolean\n"
                .to_string(),
            "bogus",
            0,
        ),
        (
            // The same text appears earlier in a comment and in a valid
            // definition; only the folded definition's token is the error.
            "repeated.yaml",
            "# bogus(2)\nkind: schema\ntypes:\n  a: string(min(1);\n    max(2))\n  t: string(min(1);\n    bogus(2))\n"
                .to_string(),
            "bogus",
            1,
        ),
    ]
}

/// A grammar error inside a folded scalar is ranged at the offending token on
/// its continuation line, with the inner-definition code and error severity,
/// rather than at the whole value starting on the definition's line.
#[test]
fn a_grammar_error_on_a_continuation_line_ranges_the_offending_token() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-8");
    for (name, text, token, occurrence) in malformed_cases() {
        let uri = open(&fixture, &workspace, name, &text);
        let diagnostics = definition_diagnostics(&mut fixture, &uri);
        let [diagnostic] = diagnostics.as_slice() else {
            panic!("{name}: expected one schema diagnostic: {diagnostics:#?}");
        };
        assert_eq!(
            diagnostic["code"],
            json!("dm.schema.invalid_type_definition"),
            "{name}: {diagnostic:#?}"
        );
        assert_eq!(diagnostic["severity"], json!(1), "{name}: {diagnostic:#?}");
        assert!(
            diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains("bogus")),
            "{name}: {diagnostic:#?}"
        );
        assert_eq!(
            diagnostic["range"],
            range_of(&text, token, occurrence, "utf-8"),
            "{name}: {diagnostic:#?}"
        );
    }
    fixture.shutdown();
}

/// An invalid `narrows(...)` clause inside a `|-` block scalar is ranged at
/// the clause's offending token, with the grammar's own message, and a
/// control whose clause is valid reports nothing.
#[test]
fn an_invalid_narrows_clause_in_a_block_scalar_is_ranged_at_its_token() {
    let function = |clause: &str| {
        format!(
            "kind: schema\ntypes:\n  pred: |-\n    function(\n      parameters(x: unknown | null);\n      returns(boolean);\n      {clause};\n    )\n"
        )
    };
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-8");

    let control = function("narrows(x: string; true-iff)");
    let uri = open(&fixture, &workspace, "narrows-valid.yaml", &control);
    assert_eq!(definition_diagnostics(&mut fixture, &uri), Vec::<Value>::new(), "control");

    for (name, clause, token, message) in [
        ("kind.yaml", "narrows(x: string; true-sometimes)", "true-sometimes", "true-sometimes"),
        ("parameter.yaml", "narrows(y: string; true-iff)", "y", "does not declare"),
    ] {
        let text = function(clause);
        let uri = open(&fixture, &workspace, name, &text);
        let diagnostics = definition_diagnostics(&mut fixture, &uri);
        let [diagnostic] = diagnostics.as_slice() else {
            panic!("{name}: expected one schema diagnostic: {diagnostics:#?}");
        };
        assert_eq!(diagnostic["code"], json!("dm.schema.invalid_type_definition"), "{name}: {diagnostic:#?}");
        assert!(diagnostic["message"].as_str().is_some_and(|text| text.contains(message)), "{name}: {diagnostic:#?}");
        let clause_start = text.find(clause).unwrap();
        let token_start = clause_start + clause.find(token).unwrap();
        assert_eq!(
            diagnostic["range"],
            json!({
                "start": position(&text, token_start, "utf-8"),
                "end": position(&text, token_start + token.len(), "utf-8"),
            }),
            "{name}: {diagnostic:#?}"
        );
    }
    fixture.shutdown();
}

/// The specification's literal example: `bogus` on physical line 5, columns
/// 5–9 when displayed one-based.
#[test]
fn the_specified_bogus_example_underlines_line_five_columns_five_to_nine() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-16");
    let uri = open(&fixture, &workspace, "bogus.yaml", BOGUS);
    let diagnostics = definition_diagnostics(&mut fixture, &uri);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0]["range"],
        json!({ "start": { "line": 3, "character": 4 }, "end": { "line": 3, "character": 9 } }),
    );
    fixture.shutdown();
}

/// Non-BMP text before the offending token on its continuation line separates
/// byte, Unicode-scalar, and UTF-16 columns; each negotiated encoding reports
/// its own.
#[test]
fn a_continuation_line_grammar_error_converts_through_the_negotiated_encoding() {
    let text = "kind: schema\ntypes:\n  t: enum(a,\n    '😀é', b; bogus(2))\n";
    for encoding in ["utf-8", "utf-16"] {
        let workspace = LspWorkspace::new();
        let mut fixture = start(&workspace, encoding);
        let uri = open(&fixture, &workspace, "astral.yaml", text);
        let diagnostics = definition_diagnostics(&mut fixture, &uri);
        assert_eq!(diagnostics.len(), 1, "{encoding}: {diagnostics:#?}");
        assert_eq!(
            diagnostics[0]["range"],
            range_of(text, "bogus", 0, encoding),
            "{encoding}: {diagnostics:#?}"
        );
        fixture.shutdown();
    }
}

/// When the strict locator cannot read the document (here, a quoted scalar
/// continued across lines for a sibling definition), the token cannot be
/// projected, so the diagnostic keeps its existing whole-value fallback and
/// code. It is never an empty or fabricated range.
#[test]
fn an_unprojectable_grammar_error_keeps_the_whole_value_fallback() {
    let text = "kind: schema\ntypes:\n  t: string(min(1);\n    bogus(2))\n  next: \"boolean -> a\n    b\"\n";
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-8");
    let uri = open(&fixture, &workspace, "block-sibling.yaml", text);
    let diagnostics = definition_diagnostics(&mut fixture, &uri);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("expected one schema diagnostic: {diagnostics:#?}");
    };
    assert_eq!(diagnostic["code"], json!("dm.schema.invalid_type_definition"));
    assert_eq!(diagnostic["severity"], json!(1));
    let value = "string(min(1);\n    bogus(2))";
    let start = text.find(value).unwrap();
    assert_eq!(
        diagnostic["range"]["start"],
        position(text, start, "utf-8"),
        "{diagnostic:#?}"
    );
    assert_ne!(diagnostic["range"], range_of(text, "bogus", 0, "utf-8"));
    assert_ne!(diagnostic["range"]["start"], diagnostic["range"]["end"]);
    fixture.shutdown();
}

/// A definition inside a pure root-union mapping arm is located through its
/// indexed arm, at the offending token, as an inner-definition error; an arm
/// of the wrong type stays a declaration-shape error on that arm.
#[test]
fn a_definition_inside_a_root_union_arm_is_ranged_at_its_token() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-8");
    let text = "$schema:\n- title: string\n- count: number(bogus(1))\n";
    let uri = open(&fixture, &workspace, "arm.yaml", text);
    let diagnostics = definition_diagnostics(&mut fixture, &uri);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("expected one schema diagnostic: {diagnostics:#?}");
    };
    assert_eq!(diagnostic["code"], json!("dm.schema.invalid_type_definition"));
    assert_eq!(diagnostic["range"], range_of(text, "bogus", 0, "utf-8"));

    let text = "$schema:\n- title: string\n- 7\n";
    let uri = open(&fixture, &workspace, "shape.yaml", text);
    let diagnostics = definition_diagnostics(&mut fixture, &uri);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("expected one schema diagnostic: {diagnostics:#?}");
    };
    assert_eq!(diagnostic["code"], json!("dm.schema.invalid_schema_shape"));
    assert_eq!(diagnostic["range"], range_of(text, "7", 0, "utf-8"));
    fixture.shutdown();
}

/// Opens `text` as a Markdown document named `name`.
fn open_markdown(fixture: &LspFixture<'_>, workspace: &LspWorkspace, name: &str, text: &str) -> String {
    let path = workspace.path().join(name);
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(path).unwrap().to_string();
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text }
        }),
    );
    uri
}

/// A grammar error in an inline frontmatter `$schema` is underlined at its
/// token, in a nested mapping and in a sequence member, through the
/// source-aware declaration parser: LF and CRLF, non-BMP text before the
/// token, and both negotiated encodings.
#[test]
fn an_inline_schema_definition_error_is_ranged_at_its_token() {
    let cases = [
        (
            "nested",
            "---\n$schema:\n  meta:\n    'a.b': string(suggest('😀'); bogus(1))\n---\n\nBody\n",
        ),
        (
            "member",
            "---\n$schema:\n  tags: [string, 'number(suggest(\"😀\"); bogus(1))']\n---\n\nBody\n",
        ),
        (
            "block-member",
            "---\n$schema:\n  tags:\n    - string\n    - number(suggest('😀');\n        bogus(1))\n---\n\nBody\n",
        ),
    ];
    for encoding in ["utf-8", "utf-16"] {
        let workspace = LspWorkspace::new();
        let mut fixture = start(&workspace, encoding);
        for (name, body) in cases {
            for (text, ending) in [(body.to_string(), "lf"), (crlf(body), "crlf")] {
                let uri = open_markdown(&fixture, &workspace, &format!("{ending}-{name}.md"), &text);
                let diagnostics: Vec<Value> = fixture
                    .wait_for_diagnostics(&uri)
                    .into_iter()
                    .filter(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_type_definition"))
                    .collect();
                let [diagnostic] = diagnostics.as_slice() else {
                    panic!("{encoding} {ending} {name}: expected one definition error: {diagnostics:#?}");
                };
                assert_eq!(
                    diagnostic["range"],
                    range_of(&text, "bogus", 0, encoding),
                    "{encoding} {ending} {name}: {diagnostic:#?}"
                );
            }
        }
        fixture.shutdown();
    }
}

/// A definition error in a referenced schema file stays on the document's
/// `$schema` reference: its byte offsets belong to that file and are never
/// applied to this buffer.
#[test]
fn a_referenced_definition_error_keeps_the_local_reference_range() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-8");
    std::fs::write(
        workspace.path().join("bad.yaml"),
        "kind: schema\ntypes:\n  title: string(min(1); bogus(1))\n",
    )
    .unwrap();
    let text = "---\n$schema: ./bad.yaml\ntitle: x\n---\n\nBody text that is long enough to hold the offsets\n";
    let uri = open_markdown(&fixture, &workspace, "doc.md", text);
    let diagnostics = definition_diagnostics(&mut fixture, &uri);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("expected one schema diagnostic: {diagnostics:#?}");
    };
    assert_eq!(diagnostic["code"], json!("dm.schema.prepare"), "{diagnostic:#?}");
    assert!(diagnostic["message"].as_str().is_some_and(|message| message.contains("bogus")));
    assert_eq!(diagnostic["range"], range_of(text, "./bad.yaml", 0, "utf-8"));
    fixture.shutdown();
}

// ── Hover ──────────────────────────────────────────────────────────────────

/// Hover on continuation text and on indentless items selects the same node,
/// and reports the same meaning, as the conventional spelling of the schema.
#[test]
fn hover_on_continuation_text_and_indentless_items_selects_the_conventional_node() {
    let conventional = concat!(
        "kind: schema\n",
        "types:\n",
        "  a: string(min(1); max(2))\n",
        "  t:\n",
        "    - string\n",
        "    - number\n",
        "  b: string(min(1); max(9))\n",
        "  next: boolean\n",
    );
    let presented = concat!(
        "kind: schema\n",
        "types:\n",
        "  a: string(min(1);\n",
        "    max(2))\n",
        "  t:\n",
        "  - string\n",
        "  - number\n",
        "  b: string(min(1);\n",
        "    max(9))\n",
        "  next: boolean\n",
    );
    // (needle, occurrence in conventional, occurrence in presented): the
    // second `max` is the repeated identical-shape definition `b`.
    let probes = [
        ("max(2)", 0, 0),
        ("max(9)", 0, 0),
        ("number", 0, 0),
        ("boolean", 0, 0),
    ];
    for (text, label) in [(presented.to_string(), "lf"), (crlf(presented), "crlf")] {
        let workspace = LspWorkspace::new();
        let mut fixture = start(&workspace, "utf-16");
        let conventional_uri = open(&fixture, &workspace, "conventional.yaml", conventional);
        let presented_uri = open(&fixture, &workspace, "presented.yaml", &text);
        for (needle, conventional_index, presented_index) in probes {
            let expected = hover(
                &mut fixture,
                &conventional_uri,
                range_of(conventional, needle, conventional_index, "utf-16")["start"].clone(),
            );
            let actual = hover(
                &mut fixture,
                &presented_uri,
                range_of(&text, needle, presented_index, "utf-16")["start"].clone(),
            );
            assert!(
                expected.contains("Type: **type-definition**"),
                "{label} {needle}: conventional hover {expected}"
            );
            assert_eq!(actual, expected, "{label} {needle}");
        }
        fixture.shutdown();
    }
}

// ── Completion ─────────────────────────────────────────────────────────────

/// Completion on a continuation line and on an indentless item, for a
/// complete expression and a partially typed one, replaces exactly the
/// authored token.
#[test]
fn completion_on_continuation_text_and_indentless_items_replaces_the_authored_token() {
    // (file, text, cursor after this needle, label, token the edit replaces)
    let cases = [
        (
            // Partially typed: the document does not load.
            "partial-continuation.yaml",
            "kind: schema\ntypes:\n  t: string(min(1);\n    ma\n  next: boolean\n",
            "    ma",
            "max",
            "ma",
        ),
        (
            "complete-continuation.yaml",
            "kind: schema\ntypes:\n  t: string(min(1);\n    max(2))\n  next: boolean\n",
            "    max",
            "max",
            "max",
        ),
        (
            "partial-indentless.yaml",
            "kind: schema\ntypes:\n  t:\n  - number\n  - str\n  next: boolean\n",
            "  - str",
            "string",
            "str",
        ),
        (
            "complete-indentless.yaml",
            "kind: schema\ntypes:\n  t:\n  - number\n  - string\n  next: boolean\n",
            "  - string",
            "string",
            "string",
        ),
        (
            "partial-pure-root-union.yaml",
            "$schema:\n- title: string\n  next: boolean\n- count: num\n",
            "count: num",
            "number",
            "num",
        ),
    ];
    for encoding in ["utf-8", "utf-16"] {
        let workspace = LspWorkspace::new();
        let mut fixture = start(&workspace, encoding);
        for (name, body, after, label, token) in cases {
            for (text, ending) in [(body.to_string(), "lf"), (crlf(body), "crlf")] {
                let uri = open(&fixture, &workspace, &format!("{ending}-{name}"), &text);
                let cursor = text.find(after).expect("cursor needle") + after.len();
                let item = completion(&mut fixture, &uri, position(&text, cursor, encoding), label)
                    .unwrap_or_else(|| panic!("{encoding} {name}: no {label} completion"));
                let token_start = cursor - token.len();
                assert_eq!(
                    item["textEdit"]["range"]["start"],
                    position(&text, token_start, encoding),
                    "{encoding} {name}: {item:#?}"
                );
                assert_eq!(
                    item["textEdit"]["range"]["end"],
                    position(&text, cursor, encoding),
                    "{encoding} {name}: replacement must stop before arguments or siblings"
                );
                let replacement = if label == "max" { "max(number)" } else { label };
                assert_eq!(item["textEdit"]["newText"], replacement, "{encoding} {name}");
            }
        }
        fixture.shutdown();
    }
}

/// Every completion label offered at `at`, sorted, with the `max` item's edit.
fn completion_labels(fixture: &mut LspFixture<'_>, uri: &str, at: Value) -> Vec<String> {
    let items = fixture
        .request(
            "textDocument/completion",
            json!({ "textDocument": { "uri": uri }, "position": at }),
        )
        .result
        .expect("completion result");
    let mut labels: Vec<String> = items
        .as_array()
        .expect("completion array")
        .iter()
        .map(|item| item["label"].as_str().unwrap_or_default().to_string())
        .collect();
    labels.sort();
    labels
}

/// An anchored or tagged plain value continued onto the next line completes
/// the same way whether the document loads or is still being typed: the same
/// candidates, and an edit that replaces only the token, never the `&e` or
/// `!!str` before the value.
#[test]
fn an_anchored_or_tagged_continuation_completes_like_the_loaded_document() {
    // (name, loaded text, incomplete text); the cursor follows the `max` on
    // the continuation line in both.
    let cases = [
        (
            "anchor",
            "kind: schema\ntypes:\n  entry: &e string(min(1);\n    max(2))\n  next: boolean\n",
            "kind: schema\ntypes:\n  entry: &e string(min(1);\n    max\n  next: boolean\n",
        ),
        (
            "tag",
            "kind: schema\ntypes:\n  entry: !!str string(min(1);\n    max(2))\n  next: boolean\n",
            "kind: schema\ntypes:\n  entry: !!str string(min(1);\n    max\n  next: boolean\n",
        ),
        (
            "blank-fold",
            "kind: schema\ntypes:\n  entry: &e string(min(1);\n\n    max(2))\n  next: boolean\n",
            "kind: schema\ntypes:\n  entry: &e string(min(1);\n\n    max\n  next: boolean\n",
        ),
        (
            "sequence-item",
            "kind: schema\ntypes:\n  entry:\n    - &e string(min(1);\n      max(2))\n    - number\n",
            "kind: schema\ntypes:\n  entry:\n    - &e string(min(1);\n      max\n    - number\n",
        ),
    ];
    for encoding in ["utf-8", "utf-16"] {
        let workspace = LspWorkspace::new();
        let mut fixture = start(&workspace, encoding);
        for (name, loaded, incomplete) in cases {
            for ending in ["lf", "crlf"] {
                let mut offered = Vec::new();
                for (state, body) in [("loaded", loaded), ("incomplete", incomplete)] {
                    let text = if ending == "crlf" { crlf(body) } else { body.to_string() };
                    let uri = open(&fixture, &workspace, &format!("{ending}-{state}-{name}.yaml"), &text);
                    let token_start = text.rfind("max").expect("continued token");
                    let cursor = token_start + "max".len();
                    let at = position(&text, cursor, encoding);
                    let item = completion(&mut fixture, &uri, at.clone(), "max")
                        .unwrap_or_else(|| panic!("{encoding} {ending} {state} {name}: no max"));
                    assert_eq!(
                        item["textEdit"]["range"],
                        json!({ "start": position(&text, token_start, encoding), "end": at }),
                        "{encoding} {ending} {state} {name}"
                    );
                    assert_eq!(item["textEdit"]["newText"], "max(number)");
                    offered.push(completion_labels(&mut fixture, &uri, at));
                }
                assert_eq!(offered[0], offered[1], "{encoding} {ending} {name}: loaded vs incomplete");
            }
        }
        fixture.shutdown();
    }
}

/// A line that does not continue the holder's plain scalar offers no
/// constraint completions from it: after a comment line or a trailing comment,
/// under a quoted or block value, an alias, an empty holder, and on a dedented
/// line that is the holder's sibling rather than its continuation.
#[test]
fn a_line_the_plain_scalar_does_not_reach_offers_none_of_its_completions() {
    let cases = [
        ("comment-line", "kind: schema\ntypes:\n  entry: string(min(1);\n    # note\n    ma\n"),
        ("trailing-comment", "kind: schema\ntypes:\n  entry: string(min(1) # note\n    ma\n"),
        ("single-quoted", "kind: schema\ntypes:\n  entry: 'string(min(1);'\n    ma\n"),
        ("double-quoted", "kind: schema\ntypes:\n  entry: \"string(min(1);\"\n    ma\n"),
        ("block", "kind: schema\ntypes:\n  entry: >\n    string(min(1);\n  ma\n"),
        ("alias", "kind: schema\ntypes:\n  base: &e string\n  entry: *e\n    ma\n"),
        ("flow", "kind: schema\ntypes:\n  entry: [string(min(1)]\n    ma\n"),
        ("dedented-sibling", "kind: schema\ntypes:\n  entry: string(min(1);\n  ma\n"),
        ("sequence-sibling", "kind: schema\ntypes:\n  entry:\n    - string(min(1);\n    ma\n"),
    ];
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-16");
    for (name, body) in cases {
        for (text, ending) in [(body.to_string(), "lf"), (crlf(body), "crlf")] {
            let uri = open(&fixture, &workspace, &format!("{ending}-{name}.yaml"), &text);
            let cursor = text.rfind("ma").expect("cursor token") + 2;
            assert!(
                completion(&mut fixture, &uri, position(&text, cursor, "utf-16"), "max").is_none(),
                "{ending} {name}: a line outside the scalar must not complete its constraints"
            );
        }
    }
    fixture.shutdown();
}

/// Recognizing a continued value is passive: a command substitution in the
/// value being typed and an import it names are neither run nor loaded.
#[test]
fn continuation_completion_runs_and_loads_nothing() {
    let workspace = LspWorkspace::new();
    let marker = workspace.path().join("ran.txt");
    let mut fixture = start(&workspace, "utf-8");
    let text = format!(
        "kind: schema\ntypes:\n  entry: &e string(suggest('$(touch {}) {{{{ ctx.today }}}}');\n    ma\n  other: Other@./missing.yaml\n",
        marker.display()
    );
    let uri = open(&fixture, &workspace, "passive.yaml", &text);
    let cursor = text.rfind("ma").unwrap() + 2;
    assert!(completion(&mut fixture, &uri, position(&text, cursor, "utf-8"), "max").is_some());
    fixture.shutdown();
    assert!(!marker.exists(), "completion must not run the value");
    assert!(!workspace.path().join("missing.yaml").exists());
}

#[test]
fn completion_after_non_bmp_text_on_a_continuation_uses_negotiated_columns() {
    let body = "kind: schema\ntypes:\n  t: string(min(1);\n    suggest('😀'); ma\n  next: boolean\n";
    for encoding in ["utf-8", "utf-16"] {
        let workspace = LspWorkspace::new();
        let mut fixture = start(&workspace, encoding);
        for (text, ending) in [(body.to_string(), "lf"), (crlf(body), "crlf")] {
            let uri = open(&fixture, &workspace, &format!("{ending}-unicode.yaml"), &text);
            let start = text.find("; ma").expect("partial token") + 2;
            let item = completion(&mut fixture, &uri, position(&text, start + 2, encoding), "max")
                .expect("max completion after the non-BMP suggestion");
            assert_eq!(
                item["textEdit"]["range"],
                json!({ "start": position(&text, start, encoding), "end": position(&text, start + 2, encoding) })
            );
            assert_eq!(item["textEdit"]["newText"], "max(number)");
        }
        fixture.shutdown();
    }
}

// ── Advisory suggestion lint ───────────────────────────────────────────────

/// A lint-invalid suggestion on a continuation line is ranged at its authored
/// token in LF and CRLF buffers, stays a warning, and does not stop the schema
/// loading: the sibling after the fold still hovers as a definition.
#[test]
fn a_lint_invalid_suggestion_on_a_continuation_line_stays_advisory_and_exact() {
    let body = concat!(
        "kind: schema\n",
        "types:\n",
        "  count: number(min(0); suggest(1,\n",
        "    many, 2)) -> how many\n",
        "  next: boolean\n",
    );
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-16");
    for (text, ending) in [(body.to_string(), "lf"), (crlf(body), "crlf")] {
        let uri = open(&fixture, &workspace, &format!("{ending}-suggest.yaml"), &text);
        let diagnostics = definition_diagnostics(&mut fixture, &uri);
        let [diagnostic] = diagnostics.as_slice() else {
            panic!("{ending}: expected one schema diagnostic: {diagnostics:#?}");
        };
        assert_eq!(diagnostic["code"], json!("dm.schema.invalid_suggestion"), "{ending}");
        assert_eq!(diagnostic["severity"], json!(2), "{ending}");
        assert_eq!(diagnostic["range"], range_of(&text, "many", 0, "utf-16"), "{ending}");
        let next = hover(&mut fixture, &uri, range_of(&text, "boolean", 0, "utf-16")["start"].clone());
        assert!(next.contains("Declares: **boolean**"), "{ending}: {next}");
    }
    fixture.shutdown();
}

/// A lint-invalid candidate whose own range crosses a fold is reported once,
/// ranged from its opening quote on the first line to just after its closing
/// quote on the continuation line. Non-ASCII text on both lines makes the
/// UTF-16 columns differ from the byte columns.
#[test]
fn a_lint_invalid_suggestion_crossing_a_fold_is_one_two_line_range() {
    let body = concat!(
        "kind: schema\n",
        "types:\n",
        "  mode: string(min(40); suggest('fïrst\n",
        "    sécond')) -> mode\n",
        "  next: boolean\n",
    );
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace, "utf-16");
    for (text, ending) in [(body.to_string(), "lf"), (crlf(body), "crlf")] {
        let uri = open(&fixture, &workspace, &format!("{ending}-folded-suggest.yaml"), &text);
        let diagnostics = definition_diagnostics(&mut fixture, &uri);
        let [diagnostic] = diagnostics.as_slice() else {
            panic!("{ending}: expected one schema diagnostic: {diagnostics:#?}");
        };
        assert_eq!(diagnostic["code"], json!("dm.schema.invalid_suggestion"), "{ending}");
        assert_eq!(diagnostic["severity"], json!(2), "{ending}");
        let start = text.find("'fïrst").expect("opening quote");
        let end = text.find("sécond'").expect("closing quote") + "sécond'".len();
        assert_eq!(
            diagnostic["range"],
            json!({ "start": { "line": 2, "character": 32 }, "end": { "line": 3, "character": 11 } }),
            "{ending}"
        );
        assert_eq!(
            diagnostic["range"],
            json!({ "start": position(&text, start, "utf-16"), "end": position(&text, end, "utf-16") }),
            "{ending}"
        );
        let next = hover(&mut fixture, &uri, range_of(&text, "boolean", 0, "utf-16")["start"].clone());
        assert!(next.contains("Declares: **boolean**"), "{ending}: {next}");
    }
    fixture.shutdown();
}
