//! Editor support for the core schema types `unknown` (alias `any`), `null`,
//! `alphanumeric`, and `shell`: completion at exact cursor positions, hover,
//! schema diagnostics with exact ranges, and a lifecycle-shaped schema whose
//! `initialize` and `start` action unions DMLS judges the way the library's
//! validation (and therefore `md schema validate`) does.
//!
//! Every document under test is an unsaved buffer: the file on disk holds
//! different text, so each answer comes from the open buffer.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde_json::{Value, json};

fn initialize_params(root: &Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "clientInfo": { "name": "Core Types", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": [{ "uri": root_uri.as_str(), "name": "scratch" }]
    })
}

/// An open document whose buffer differs from what is saved on disk.
struct Buffer {
    uri: String,
    text: String,
}

impl Buffer {
    /// Saves `saved` to `name` and opens the buffer with `text` instead.
    fn open(fixture: &LspFixture<'_>, workspace: &LspWorkspace, name: &str, saved: &str, text: &str) -> Self {
        let path = workspace.path().join(name);
        std::fs::write(&path, saved).unwrap();
        let uri = url::Url::from_file_path(&path).unwrap().to_string();
        fixture.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text }
            }),
        );
        Self { uri, text: text.to_string() }
    }

    /// The byte offset just past the `occurrence`-th (0-based) `needle`.
    fn after(&self, needle: &str, occurrence: usize) -> usize {
        self.start_of(needle, occurrence) + needle.len()
    }

    fn start_of(&self, needle: &str, occurrence: usize) -> usize {
        self.text
            .match_indices(needle)
            .nth(occurrence)
            .unwrap_or_else(|| panic!("`{needle}` #{occurrence} in {:?}", self.text))
            .0
    }

    /// The LSP range of the `occurrence`-th `needle`.
    fn range_of(&self, needle: &str, occurrence: usize) -> Value {
        let start = self.start_of(needle, occurrence);
        range(&self.text, start, start + needle.len())
    }
}

/// The `(line, character)` of byte `offset` in ASCII `text`.
fn position(text: &str, offset: usize) -> Value {
    let before = &text[..offset];
    let line = before.matches('\n').count();
    let character = offset - before.rfind('\n').map_or(0, |index| index + 1);
    json!({ "line": line, "character": character })
}

fn range(text: &str, start: usize, end: usize) -> Value {
    json!({ "start": position(text, start), "end": position(text, end) })
}

fn request_at(fixture: &mut LspFixture<'_>, method: &str, buffer: &Buffer, offset: usize) -> Value {
    fixture
        .request(
            method,
            json!({
                "textDocument": { "uri": buffer.uri },
                "position": position(&buffer.text, offset)
            }),
        )
        .result
        .unwrap_or(Value::Null)
}

fn completion_items(fixture: &mut LspFixture<'_>, buffer: &Buffer, offset: usize) -> Vec<Value> {
    request_at(fixture, "textDocument/completion", buffer, offset)
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn labels(items: &[Value]) -> Vec<&str> {
    items.iter().filter_map(|item| item["label"].as_str()).collect()
}

fn hover_text(fixture: &mut LspFixture<'_>, buffer: &Buffer, offset: usize) -> String {
    request_at(fixture, "textDocument/hover", buffer, offset)["contents"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

const OPTIONS: [&str; 9] = [
    "allow-dashes",
    "allow-underscores",
    "allow-slashes",
    "allow-periods",
    "allow-whitespace",
    "block-double-periods",
    "block-leading-dash",
    "block-leading-slash",
    "leading-alpha",
];

fn start(workspace: &LspWorkspace) -> LspFixture<'_> {
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(workspace.path()));
    fixture
}

fn item<'a>(items: &'a [Value], label: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item["label"] == json!(label))
        .unwrap_or_else(|| panic!("no `{label}` in {:?}", labels(items)))
}

/// Type-keyword positions offer the four core types, with `any` labeled as
/// the alias of `unknown`: a standalone schema value, a block union arm, a
/// flow union arm, and a frontmatter `type-definition` (meta-type) value.
#[test]
fn type_keyword_positions_offer_the_core_types_and_the_any_alias() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    let standalone = Buffer::open(
        &fixture,
        &workspace,
        "types.yaml",
        "kind: schema\ntypes: {}\n",
        "kind: schema\ntypes:\n  a: \n  b:\n    - string\n    - \n  c: '[string, '\n",
    );
    let meta = Buffer::open(
        &fixture,
        &workspace,
        "meta.md",
        "---\n---\n",
        "---\n$schema:\n  definition: type-definition\ndefinition: \n---\n\nBody\n",
    );
    let cursors = [
        (&standalone, standalone.after("a: ", 0)),
        (&standalone, standalone.after("    - ", 1)),
        (&standalone, standalone.after("[string, ", 0)),
        (&meta, meta.after("definition: ", 1)),
    ];
    for (buffer, offset) in cursors {
        let items = completion_items(&mut fixture, buffer, offset);
        let offered = labels(&items);
        for keyword in ["unknown", "any", "null", "alphanumeric", "shell"] {
            assert!(offered.contains(&keyword), "{keyword} at {offset} in {}: {offered:?}", buffer.uri);
        }
        let any = item(&items, "any");
        assert!(
            any["detail"].as_str().unwrap().starts_with("Alias of `unknown`"),
            "{any}"
        );
        assert_eq!(any["textEdit"]["newText"], json!("any"));
        assert_eq!(any["textEdit"]["range"], range(&buffer.text, offset, offset));
    }
    fixture.shutdown();
}

/// Inside `alphanumeric(…)`, the leading group before the first `;` offers
/// the options not yet written there (with their effect), alongside the
/// constraints only while the group is empty; after `;` only constraints.
#[test]
fn alphanumeric_option_completion_follows_the_leading_group() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    let text = concat!(
        "kind: schema\n",
        "types:\n",
        "  empty: alphanumeric(\n",
        "  partial: alphanumeric(allow-da\n",
        "  second: alphanumeric(allow-dashes, \n",
        "  after: alphanumeric(allow-dashes; \n",
        "  arm: '[string, alphanumeric(leading-alpha, '\n",
        "  other: string(\n",
    );
    let buffer = Buffer::open(&fixture, &workspace, "types.yaml", "kind: schema\ntypes: {}\n", text);
    let options = |items: &[Value]| -> Vec<String> {
        labels(items)
            .into_iter()
            .filter(|label| OPTIONS.contains(label))
            .map(str::to_string)
            .collect()
    };
    let all: Vec<String> = OPTIONS.iter().map(|option| option.to_string()).collect();
    let without = |written: &str| -> Vec<String> {
        all.iter().filter(|option| *option != written).cloned().collect()
    };

    let offset = buffer.after("empty: alphanumeric(", 0);
    let items = completion_items(&mut fixture, &buffer, offset);
    assert_eq!(options(&items), all);
    assert!(labels(&items).contains(&"required"), "{:?}", labels(&items));
    let dashes = item(&items, "allow-dashes");
    assert_eq!(dashes["detail"], json!("`alphanumeric` option: also allows `-`"));
    assert_eq!(dashes["textEdit"]["range"], range(&buffer.text, offset, offset));

    let offset = buffer.after("allow-da", 0);
    let items = completion_items(&mut fixture, &buffer, offset);
    assert_eq!(labels(&items), ["allow-dashes"]);
    assert_eq!(items[0]["textEdit"]["range"], range(&buffer.text, offset - "allow-da".len(), offset));

    let items = completion_items(&mut fixture, &buffer, buffer.after("second: alphanumeric(allow-dashes, ", 0));
    assert_eq!(options(&items), without("allow-dashes"));
    assert!(!labels(&items).contains(&"required"), "a constraint here is a mixed list: {:?}", labels(&items));

    let items = completion_items(&mut fixture, &buffer, buffer.after("after: alphanumeric(allow-dashes; ", 0));
    assert!(options(&items).is_empty(), "{:?}", labels(&items));
    for constraint in ["required", "min", "max", "pattern"] {
        assert!(labels(&items).contains(&constraint), "{constraint}: {:?}", labels(&items));
    }

    let items = completion_items(&mut fixture, &buffer, buffer.after("alphanumeric(leading-alpha, ", 0));
    assert_eq!(options(&items), without("leading-alpha"));

    let items = completion_items(&mut fixture, &buffer, buffer.after("string(", 0));
    assert!(options(&items).is_empty(), "{:?}", labels(&items));
    assert!(labels(&items).contains(&"required"), "{:?}", labels(&items));
    fixture.shutdown();
}

/// Hover describes each core type from the catalog: `any` as `unknown` with
/// its alias, native `null`, `shell`, and `alphanumeric` with its canonical
/// options and effective character sets. A valid buffer, including native
/// null spellings, publishes no diagnostic.
#[test]
fn hover_describes_the_core_types_and_alphanumeric_character_sets() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    let text = concat!(
        "kind: schema\n",
        "types:\n",
        "  anything: unknown\n",
        "  whatever: any\n",
        "  nothing: null\n",
        "  tilde: ~\n",
        "  empty:\n",
        "  maybe: [string, null]\n",
        "  slug: alphanumeric(allow-dashes, block-leading-dash, block-double-periods; max(40))\n",
        "  word: alphanumeric\n",
        "  command: shell\n",
    );
    let buffer = Buffer::open(&fixture, &workspace, "types.yaml", "kind: schema\ntypes: {}\n", text);
    assert_eq!(fixture.wait_for_diagnostics(&buffer.uri), Vec::<Value>::new());

    for key in ["anything", "whatever"] {
        let hover = hover_text(&mut fixture, &buffer, buffer.start_of(key, 0));
        assert!(hover.contains("Declares: **unknown**"), "{key}: {hover}");
        assert!(hover.contains("**`unknown`** (alias `any`): Every value"), "{key}: {hover}");
    }
    for key in ["nothing", "tilde", "empty"] {
        let hover = hover_text(&mut fixture, &buffer, buffer.start_of(key, 0));
        assert!(hover.contains("Declares: **null**"), "{key}: {hover}");
        assert!(hover.contains("**`null`**: Only an explicit null value"), "{key}: {hover}");
    }
    let maybe = hover_text(&mut fixture, &buffer, buffer.start_of("maybe", 0));
    assert!(maybe.contains("Declares: **string | null**"), "{maybe}");

    let slug = hover_text(&mut fixture, &buffer, buffer.start_of("slug", 0));
    assert!(slug.contains("**`alphanumeric`**: A string of ASCII letters and digits"), "{slug}");
    // `block-double-periods` is dropped: periods are not allowed anyway.
    assert!(slug.contains("Options: `allow-dashes`, `block-leading-dash`\n"), "{slug}");
    assert!(slug.contains("Characters: `A-Z a-z 0-9 -`"), "{slug}");
    assert!(slug.contains("First character: `A-Z a-z 0-9`"), "{slug}");
    assert!(slug.contains("`..`: forbidden"), "{slug}");

    let word = hover_text(&mut fixture, &buffer, buffer.start_of("word", 0));
    assert!(word.contains("Options: none"), "{word}");
    assert!(word.contains("Characters: `A-Z a-z 0-9`"), "{word}");

    let command = hover_text(&mut fixture, &buffer, buffer.start_of("command", 0));
    assert!(command.contains("**`shell`**: A string holding a shell command"), "{command}");

    // A document property typed `alphanumeric` shows the same effective sets.
    let document = Buffer::open(
        &fixture,
        &workspace,
        "doc.md",
        "---\n---\n",
        "---\n$schema:\n  id: alphanumeric(leading-alpha, allow-periods)\nid: a.b\n---\n\nBody\n",
    );
    assert_eq!(fixture.wait_for_diagnostics(&document.uri), Vec::<Value>::new());
    let hover = hover_text(&mut fixture, &document, document.start_of("id: a.b", 0));
    assert!(hover.contains("Type: **alphanumeric**"), "{hover}");
    assert!(hover.contains("Characters: `A-Z a-z 0-9 .`"), "{hover}");
    assert!(hover.contains("First character: `A-Z a-z`"), "{hover}");
    assert!(hover.contains("`..`: allowed"), "{hover}");
    fixture.shutdown();
}

/// A local type named like a built-in stays a local reference when spelled
/// `Name@this`: completion offers that spelling, and hover and go-to-definition
/// select the local definition, while the bare keyword is the built-in.
#[test]
fn a_local_type_named_like_a_builtin_resolves_through_its_explicit_reference() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    let text = concat!(
        "kind: schema\n",
        "types:\n",
        "  shell: string(min(2))\n",
        "  local: shell@this\n",
        "  builtin: shell\n",
        "  fresh: \n",
    );
    let buffer = Buffer::open(&fixture, &workspace, "types.yaml", "kind: schema\ntypes: {}\n", text);

    let reference = buffer.start_of("shell@this", 0);
    let hover = hover_text(&mut fixture, &buffer, reference);
    assert!(hover.contains("Declares: **string**"), "{hover}");
    assert!(hover.contains("Defined in: this schema"), "{hover}");
    let definition = request_at(&mut fixture, "textDocument/definition", &buffer, reference);
    let location = if definition.is_array() { definition[0].clone() } else { definition };
    assert_eq!(location["uri"], json!(buffer.uri));
    assert_eq!(location["range"], buffer.range_of("shell", 0));

    let builtin = hover_text(&mut fixture, &buffer, buffer.start_of("builtin", 0));
    assert!(builtin.contains("Declares: **shell**"), "{builtin}");
    assert!(builtin.contains("**`shell`**: A string holding a shell command"), "{builtin}");

    let items = completion_items(&mut fixture, &buffer, buffer.after("fresh: ", 0));
    assert_eq!(item(&items, "shell@this")["kind"], json!(18), "a named-type reference");
    assert_eq!(item(&items, "shell")["kind"], json!(25), "the built-in keyword");
    fixture.shutdown();
}

/// Option grammar errors and a non-null `null` default are schema
/// diagnostics ranged on the offending token of an unsaved buffer.
#[test]
fn option_and_null_default_errors_are_ranged_on_the_offending_token() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    // (definition, offending token, its occurrence, message fragment)
    let cases = [
        ("alphanumeric(allow-dash)", "allow-dash", 0, "unknown `alphanumeric` option `allow-dash`"),
        ("alphanumeric(allow-dashes, allow-dashes)", "allow-dashes", 1, "repeated `alphanumeric` option"),
        ("alphanumeric(min(1); allow-dashes)", "allow-dashes", 0, "must be written before the first `;`"),
        ("alphanumeric(allow-dashes, min(1))", "min", 0, "separate `alphanumeric` options from constraints"),
        ("null(default(1))", "1", 0, "a `null` default must be null"),
    ];
    for (index, (definition, token, occurrence, message)) in cases.into_iter().enumerate() {
        let text = format!("kind: schema\ntypes:\n  a: {definition}\n");
        let buffer = Buffer::open(&fixture, &workspace, &format!("types{index}.yaml"), "kind: schema\ntypes: {}\n", &text);
        let diagnostics = fixture.wait_for_diagnostics(&buffer.uri);
        assert_eq!(diagnostics.len(), 1, "{definition}: {diagnostics:?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic["code"], json!("dm.schema.invalid_type_definition"), "{definition}");
        assert!(diagnostic["message"].as_str().unwrap().contains(message), "{definition}: {diagnostic}");
        // Offsets are found in the line after `a: `, never in `types:`.
        let line_start = buffer.start_of("  a: ", 0);
        let start = line_start
            + buffer.text[line_start..]
                .match_indices(token)
                .nth(occurrence)
                .unwrap()
                .0;
        assert_eq!(diagnostic["range"], range(&buffer.text, start, start + token.len()), "{definition}");
    }
    fixture.shutdown();
}

/// A named type declared as bare `alphanumeric` takes part in an options
/// conflict as the empty set. It authors no option token, so its related
/// location is the type's definition.
#[test]
fn a_bare_alphanumeric_conflict_participant_relates_its_definition() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    let text = "kind: schema\ntypes:\n  code: alphanumeric(min(1))\n  id: code(allow-dashes)\n";
    let buffer = Buffer::open(&fixture, &workspace, "bare.yaml", "kind: schema\ntypes: {}\n", text);
    let diagnostics = fixture.wait_for_diagnostics(&buffer.uri);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let related: Vec<(Value, &str)> = diagnostics[0]["relatedInformation"]
        .as_array()
        .expect("related locations")
        .iter()
        .map(|related| (related["location"].clone(), related["message"].as_str().unwrap()))
        .collect();
    assert_eq!(
        related,
        vec![
            (
                json!({ "uri": buffer.uri, "range": buffer.range_of("alphanumeric(min(1))", 0) }),
                "`alphanumeric` is declared here",
            ),
            (
                json!({ "uri": buffer.uri, "range": buffer.range_of("allow-dashes", 0) }),
                "`alphanumeric(allow-dashes)` is declared here",
            ),
        ]
    );
    fixture.shutdown();
}

/// A scheme conflict after an earlier successful narrowing relates the
/// effective inherited list, worded as effective rather than authored, at the
/// declaration that introduced it.
#[test]
fn a_narrowed_scheme_conflict_relates_the_effective_constraint() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    let text = "kind: schema\ntypes:\n  link: url(scheme(http, https))\n  narrow: link(scheme(https, ftp); scheme(ftp))\n";
    let buffer = Buffer::open(&fixture, &workspace, "scheme.yaml", "kind: schema\ntypes: {}\n", text);
    let diagnostics = fixture.wait_for_diagnostics(&buffer.uri);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let related: Vec<(Value, &str)> = diagnostics[0]["relatedInformation"]
        .as_array()
        .expect("related locations")
        .iter()
        .map(|related| (related["location"].clone(), related["message"].as_str().unwrap()))
        .collect();
    assert_eq!(
        related,
        vec![
            (
                json!({ "uri": buffer.uri, "range": buffer.range_of("scheme(http, https)", 0) }),
                "`scheme(https)` is the effective constraint from this declaration",
            ),
            (
                json!({ "uri": buffer.uri, "range": buffer.range_of("scheme(ftp)", 0) }),
                "`scheme(ftp)` is the effective constraint from this declaration",
            ),
        ]
    );
    fixture.shutdown();
}

/// A use site whose options differ from its named definition's is a schema
/// diagnostic on the reference, relating the definition's option group and
/// the use site's options.
#[test]
fn an_options_conflict_relates_the_definition_and_the_use_site() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    let text = "kind: schema\ntypes:\n  slug: alphanumeric(allow-dashes; max(9))\n  id: slug(allow-periods)\n";
    let buffer = Buffer::open(&fixture, &workspace, "types.yaml", "kind: schema\ntypes: {}\n", text);
    let diagnostics = fixture.wait_for_diagnostics(&buffer.uri);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic["code"], json!("dm.schema.invalid_type_definition"));
    assert!(
        diagnostic["message"].as_str().unwrap().contains("contradictory `alphanumeric` options"),
        "{diagnostic}"
    );
    assert_eq!(diagnostic["range"], buffer.range_of("slug", 1));
    let related: Vec<Value> = diagnostic["relatedInformation"]
        .as_array()
        .expect("related locations")
        .iter()
        .map(|related| related["location"].clone())
        .collect();
    assert_eq!(
        related,
        vec![
            json!({ "uri": buffer.uri, "range": buffer.range_of("allow-dashes", 0) }),
            json!({ "uri": buffer.uri, "range": buffer.range_of("allow-periods", 0) }),
        ]
    );

    let messages: Vec<&str> = diagnostic["relatedInformation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|related| related["message"].as_str().unwrap())
        .collect();
    assert_eq!(
        messages,
        [
            "`alphanumeric(allow-dashes)` is declared here",
            "`alphanumeric(allow-periods)` is declared here",
        ]
    );

    // The same options repeated at the use site are no conflict.
    let agreeing = Buffer::open(
        &fixture,
        &workspace,
        "agreeing.yaml",
        "kind: schema\ntypes: {}\n",
        "kind: schema\ntypes:\n  slug: alphanumeric(allow-dashes)\n  id: slug(allow-dashes; required)\n",
    );
    assert_eq!(fixture.wait_for_diagnostics(&agreeing.uri), Vec::<Value>::new());
    fixture.shutdown();
}

/// Native YAML null spellings declare `null` without a false diagnostic in
/// an inline `$schema`, accept null values, and reject a non-null value at
/// the value's exact range.
#[test]
fn native_null_declarations_validate_without_false_diagnostics() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    let schema = "---\n$schema:\n  review: null\n  tilde: ~\n  empty:\n";
    let valid = Buffer::open(
        &fixture,
        &workspace,
        "valid.md",
        "---\n---\n",
        &format!("{schema}review: ~\ntilde: null\nempty:\n---\n\nBody\n"),
    );
    assert_eq!(fixture.wait_for_diagnostics(&valid.uri), Vec::<Value>::new());

    let invalid = Buffer::open(
        &fixture,
        &workspace,
        "invalid.md",
        "---\n---\n",
        &format!("{schema}review: 3\n---\n\nBody\n"),
    );
    let diagnostics = fixture.wait_for_diagnostics(&invalid.uri);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0]["code"], json!("dm.schema.type_mismatch"));
    assert_eq!(diagnostics[0]["range"], invalid.range_of("3", 0));
    fixture.shutdown();
}

/// The library's validation (what `md schema validate` reports) for one
/// document written to `path`.
fn library_accepts(dir: &Path, path: &Path) -> bool {
    let context = build_resolution_context(&RequestSnapshot::new(dir).with_home(Some(dir.join("home"))))
        .expect("library context");
    DarkmatterSchemas::new(context)
        .validate(&Markdown::try_from(path).expect("read document"))
        .expect("schema resolves")
        .valid
}

/// AC7: a `shell` action is rejected under `initialize` and accepted under
/// `start`, as are the mixed and empty action objects, with each rejected
/// item ranged exactly; DMLS's decisions equal the library's for the same
/// text.
#[test]
fn a_shell_action_is_diagnosed_under_initialize_and_accepted_under_start() {
    let workspace = LspWorkspace::new();
    let mut fixture = start(&workspace);
    std::fs::write(workspace.path().join("actions.yaml"), ACTION_TYPES).unwrap();
    std::fs::write(workspace.path().join("lifecycle.yaml"), LIFECYCLE_SCHEMA).unwrap();
    for (index, (body, expected, offending)) in LIFECYCLE_ROWS.into_iter().enumerate() {
        let text = format!("---\n$schema: ./lifecycle.yaml\n{body}---\nBody\n");
        let buffer = Buffer::open(&fixture, &workspace, &format!("doc{index}.md"), "---\n---\n", &text);
        let diagnostics = fixture.wait_for_diagnostics(&buffer.uri);

        let saved = workspace.path().join(format!("saved{index}.md"));
        std::fs::write(&saved, &text).unwrap();
        assert_eq!(library_accepts(workspace.path(), &saved), expected, "library: {body}");
        assert_eq!(diagnostics.is_empty(), expected, "DMLS agrees with the library: {body}{diagnostics:?}");

        if let Some(item) = offending {
            assert_eq!(diagnostics.len(), 1, "{body}{diagnostics:?}");
            assert_eq!(diagnostics[0]["code"], json!("dm.schema.constraint"), "{body}");
            let start = buffer.after("  - ", 0);
            let end = buffer.start_of(item, 0) + item.len();
            assert_eq!(diagnostics[0]["range"], range(&buffer.text, start, end), "{body}");
        }
    }
    fixture.shutdown();
}

/// Closed action shapes, as in `md`'s and the library's lifecycle tests:
/// `initialize` takes a union without the `shell` action, `start` a union
/// with it. A tagged `kind: schema` file cannot carry `$schema`, so the
/// lifecycle schema is a plain schema file importing these types.
const ACTION_TYPES: &str = "kind: schema\ntypes:\n  \
    say_action: '{ say: string(required) }'\n  \
    set_action: '{ set: object(required) }'\n  \
    shell_action: '{ shell: shell(required) }'\n  \
    initialize_action: [say_action, set_action]\n  \
    start_action: [say_action, set_action, shell_action]\n  \
    initialize_actions: initialize_action[]\n  \
    start_actions: start_action[]\n";

const LIFECYCLE_SCHEMA: &str = "$schema:\n  \
    initialize: initialize_actions@./actions.yaml\n  \
    start: start_actions@./actions.yaml\n";

/// `(frontmatter after the $schema line, valid, end of the rejected item)`.
/// A rejected block mapping item runs from after its `- ` through its last
/// line's line break; a rejected `{}` is exactly the braces.
const LIFECYCLE_ROWS: [(&str, bool, Option<&str>); 8] = [
    ("initialize:\n  - say: hi\n  - set: { a: 1 }\n", true, None),
    ("initialize:\n  - shell: echo hi\n", false, Some("echo hi\n")),
    ("initialize:\n  - say: hi\n    shell: echo hi\n", false, Some("echo hi\n")),
    ("initialize:\n  - {}\n", false, Some("{}")),
    ("start:\n  - shell: ls | wc -l; echo done\n", true, None),
    ("start:\n  - say: hi\n  - shell: echo hi\n", true, None),
    ("start:\n  - say: hi\n    shell: echo hi\n", false, Some("echo hi\n")),
    ("start:\n  - {}\n", false, Some("{}")),
];
