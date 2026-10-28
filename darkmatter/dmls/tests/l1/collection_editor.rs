//! DMLS reads `::map` / `::filter` regions through Darkmatter's one scope
//! analysis ([`BodyScopes`], `analyze_collection_blocks`): completions offer
//! the locals visible at the cursor, hover shows the library's own types,
//! folds and semantic tokens cover the four markers, and diagnostics carry
//! the library's codes at the library's authored spans, including through
//! escaped and multibyte callback values and CRLF line endings.
//!
//! Every request runs in one in-process LSP session; the zero-I/O proof is
//! the in-crate tripwire test in `providers/collections.rs`.

use crate::common::{LspFixture, LspWorkspace};

use darkmatter::markdown::compose::collection_blocks::BodyScopes;
use darkmatter::markdown::compose::expression::{DeclaredTypes, Expr, SourceProjection, TypeScope};
use darkmatter::markdown::compose::ComposeOperationSet;
use darkmatter::markdown::schemas::{SimplifiedSchema, parse_yaml_schema};
use dmls::overlay::expressions::{shape_text, type_text};
use serde_json::{Value, json};

/// Properties under `$schema:`.
const SCHEMA: &str = concat!(
    "  team: '{ name: string(required), age: number(required), active: boolean(required) }[](required)'\n",
    "  people: '{ name: string(required), age: number }[](required)'\n",
    "  tags: string[](required)\n",
    "  flag: boolean(required)\n",
    "  note: string\n",
);

fn schema() -> SimplifiedSchema {
    let yaml: String = SCHEMA.lines().map(|line| format!("{}\n", line.trim_start())).collect();
    parse_yaml_schema(&serde_yaml_ng::from_str(&yaml).unwrap()).unwrap()
}

/// A document declaring [`SCHEMA`] followed by `body`, with `eol` line
/// endings throughout.
fn document(body: &str, eol: &str) -> String {
    format!("---\n$schema:\n{SCHEMA}---\n\n{body}").replace('\n', eol)
}

fn initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "textDocument": {
                "foldingRange": { "lineFoldingOnly": true },
                "semanticTokens": {
                    "requests": { "full": true },
                    "tokenTypes": [],
                    "tokenModifiers": [],
                    "formats": ["relative"]
                }
            }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
    })
}

/// `(line, UTF-16 character)` of a byte offset.
fn position(text: &str, offset: usize) -> (u32, u32) {
    let line = text[..offset].matches('\n').count();
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    (line as u32, text[line_start..offset].encode_utf16().count() as u32)
}

/// Byte offset of the `nth` occurrence of `needle` after the first `after`.
fn offset_of(text: &str, after: &str, needle: &str, nth: usize) -> usize {
    let base = text.find(after).unwrap_or_else(|| panic!("no `{after}`"));
    text[base..]
        .match_indices(needle)
        .nth(nth)
        .map(|(index, _)| base + index)
        .unwrap_or_else(|| panic!("no occurrence {nth} of `{needle}` after `{after}`"))
}

/// One open document in an in-process session.
struct Session<'w> {
    fixture: LspFixture<'w>,
    uri: String,
    text: String,
    diagnostics: Vec<Value>,
}

impl<'w> Session<'w> {
    fn open(workspace: &'w LspWorkspace, text: &str) -> Self {
        let path = workspace.path().join("doc.md");
        std::fs::write(&path, text).unwrap();
        let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
        let mut fixture = LspFixture::start(workspace);
        fixture.initialize(initialize_params(workspace.path()));
        fixture.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
        );
        fixture.flush_server();
        let mut latest = fixture.wait_for_diagnostics_params(&uri);
        while let Some(params) = fixture.take_buffered_diagnostics(&uri) {
            latest = params;
        }
        let diagnostics = latest["diagnostics"].as_array().cloned().unwrap_or_default();
        Self { fixture, uri, text: text.to_string(), diagnostics }
    }

    fn params(&self, offset: usize) -> Value {
        let (line, character) = position(&self.text, offset);
        json!({ "textDocument": { "uri": self.uri }, "position": { "line": line, "character": character } })
    }

    fn hover(&mut self, offset: usize) -> String {
        let params = self.params(offset);
        let response = self.fixture.request("textDocument/hover", params);
        response
            .result
            .and_then(|result| result["contents"]["value"].as_str().map(str::to_string))
            .unwrap_or_default()
    }

    /// The completion items at `offset`.
    fn completion(&mut self, offset: usize) -> Vec<Value> {
        let params = self.params(offset);
        let response = self.fixture.request("textDocument/completion", params);
        match response.result {
            Some(Value::Array(items)) => items,
            Some(result) => result["items"].as_array().cloned().unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// `(start line, end line)` of every fold, sorted.
    fn folds(&mut self) -> Vec<(u64, u64)> {
        let response = self
            .fixture
            .request("textDocument/foldingRange", json!({ "textDocument": { "uri": self.uri } }));
        let mut folds: Vec<(u64, u64)> = response
            .result
            .and_then(|result| result.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .map(|fold| (fold["startLine"].as_u64().unwrap(), fold["endLine"].as_u64().unwrap()))
            .collect();
        folds.sort();
        folds.dedup();
        folds
    }

    /// Decoded `(line, character, length, type, modifiers)` tokens.
    fn tokens(&mut self) -> Vec<(u32, u32, u32, u32, u32)> {
        let response = self
            .fixture
            .request("textDocument/semanticTokens/full", json!({ "textDocument": { "uri": self.uri } }));
        let result = response.result.expect("semantic tokens");
        let data = result["data"].as_array().expect("token data");
        let (mut line, mut character) = (0u32, 0u32);
        data.chunks(5)
            .map(|group| {
                let field = |index: usize| group[index].as_u64().unwrap() as u32;
                if field(0) == 0 {
                    character += field(1);
                } else {
                    line += field(0);
                    character = field(1);
                }
                (line, character, field(2), field(3), field(4))
            })
            .collect()
    }

    /// The diagnostics with `code` as `(start, end)` position pairs, sorted.
    fn ranges(&self, code: &str) -> Vec<((u64, u64), (u64, u64))> {
        let mut ranges: Vec<_> = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["code"] == json!(code))
            .map(|diagnostic| {
                let at = |edge: &str| {
                    (
                        diagnostic["range"][edge]["line"].as_u64().unwrap(),
                        diagnostic["range"][edge]["character"].as_u64().unwrap(),
                    )
                };
                (at("start"), at("end"))
            })
            .collect();
        ranges.sort();
        ranges
    }

    fn shutdown(self) {
        self.fixture.shutdown();
    }
}

/// The labels of `items`.
fn labels(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect()
}

fn item<'a>(items: &'a [Value], label: &str) -> &'a Value {
    items.iter().find(|item| item["label"] == json!(label)).unwrap_or_else(|| panic!("no `{label}` in {:?}", labels(items)))
}

/// The library's scope at `offset`: the one answer every editor surface must
/// match.
fn library_type(text: &str, offset: usize, path: &str) -> String {
    let schema = schema();
    let declared = DeclaredTypes::new(Some(&schema));
    let scopes = BodyScopes::scan(text, &ComposeOperationSet::all());
    let scope = scopes.scope_at(TypeScope::unguarded(&declared), offset);
    type_text(&scope.type_of_expr(&Expr::Variable(path.to_string())).unwrap_or_else(|| panic!("`{path}` has no type")))
}

// ── completion ────────────────────────────────────────────────────────────

const NESTED: &str = concat!(
    "::filter team \"p => p.active\" sort=\"q => q.age\"\n",
    "Row {{ }} {{ note }}\n",
    "::filter tags\n",
    "Inner {{ }}\n",
    "::end-filter\n",
    "Later {{ }}\n",
    "::end-filter\n",
    "After {{ }}\n",
);

/// Locals are offered where the library says they are visible, bare and as
/// `local.*`: the body's own binding and outer captures inside, none of a
/// sibling callback's, a closed region's, or a later one's.
#[test]
fn interpolation_completions_offer_exactly_the_cursor_visible_locals() {
    let text = document(NESTED, "\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);
    let inside = |prefix: &str| offset_of(&text, prefix, "{{ ", 0) + 3;

    let row = labels(&session.completion(inside("Row")));
    for expected in ["p", "local.p", "note", "tags", "flag"] {
        assert!(row.contains(&expected.to_string()), "{expected} in {row:?}");
    }
    for excluded in ["q", "local.q", "i", "local.i"] {
        assert!(!row.contains(&excluded.to_string()), "callback sibling or inner local `{excluded}` in {row:?}");
    }

    let inner = labels(&session.completion(inside("Inner")));
    for expected in ["i", "local.i", "p", "local.p"] {
        assert!(inner.contains(&expected.to_string()), "{expected} in {inner:?}");
    }
    assert!(!inner.contains(&"q".to_string()), "{inner:?}");

    let later = labels(&session.completion(inside("Later")));
    assert!(later.contains(&"p".to_string()) && !later.contains(&"i".to_string()), "expired local: {later:?}");

    let after = labels(&session.completion(inside("After")));
    assert!(!after.contains(&"p".to_string()) && !after.contains(&"local".to_string()), "closed region: {after:?}");
    session.shutdown();
}

/// A local that shadows a frontmatter key replaces it: the key is offered
/// once, as the local, typed by the library; `fm.<key>` still reaches it.
#[test]
fn a_local_shadows_the_outer_binding_in_completions() {
    let body = "::filter team \"tags => tags.active\"\nBody {{ }}\n::end-filter\n";
    let text = document(body, "\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);

    let items = session.completion(offset_of(&text, "Body", "{{ ", 0) + 3);
    let tags: Vec<&Value> = items.iter().filter(|item| item["label"] == json!("tags")).collect();
    assert_eq!(tags.len(), 1, "{:?}", labels(&items));
    assert!(tags[0]["detail"].as_str().unwrap().starts_with("local: "), "{tags:?}");
    session.shutdown();
}

/// Header completions: iterable sources only, option names without the ones
/// already written or excluded by another, `sort-order` values, and
/// callback bodies seeing only their own parameters.
#[test]
fn header_completions_offer_sources_options_and_callback_scopes() {
    let text = document(
        concat!(
            "::map \n",
            "::map team \"p => p\" so\n",
            "::map team sort=\"p => p.age\" \n",
            "::map team while=\"p => p.active\" \n",
            "::map team sort=\"p => p.age\" sort-order=\"\n",
            "::filter team \"p => p.active\" sort=\"q => \n",
            "::filter team when=\"\n",
            "::filter team \"p => p.name == 'Zoë' && ta\n",
        ),
        "\n",
    );
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);
    let line_end = |needle: &str| offset_of(&text, needle, "\n", 0);

    let sources = labels(&session.completion(offset_of(&text, "::map \n", "\n", 0)));
    assert!(sources.contains(&"team".to_string()) && sources.contains(&"tags".to_string()), "{sources:?}");
    assert!(!sources.contains(&"flag".to_string()) && !sources.contains(&"note".to_string()), "scalars: {sources:?}");

    let started = labels(&session.completion(line_end("\"p => p\" so")));
    assert_eq!(started, ["sort"], "`sort-order` needs `sort`");

    let after_sort = labels(&session.completion(offset_of(&text, "p.age\" \n", "\n", 0)));
    assert!(after_sort.contains(&"sort-order".to_string()), "{after_sort:?}");
    assert!(!after_sort.contains(&"sort".to_string()), "already written: {after_sort:?}");

    let after_while = labels(&session.completion(offset_of(&text, "while=\"p => p.active\" \n", "\n", 0)));
    assert!(!after_while.contains(&"until".to_string()) && !after_while.contains(&"while".to_string()), "{after_while:?}");
    assert!(after_while.contains(&"filter".to_string()), "{after_while:?}");

    let order = session.completion(offset_of(&text, "sort-order=\"\n", "\n", 0));
    assert_eq!(labels(&order), ["asc", "desc"]);

    let sibling = labels(&session.completion(line_end("sort=\"q => ")));
    assert!(sibling.contains(&"q".to_string()) && !sibling.contains(&"p".to_string()), "callback sibling: {sibling:?}");

    let gate = labels(&session.completion(line_end("::filter team when=\"")));
    assert!(gate.contains(&"tags".to_string()) && !gate.contains(&"i".to_string()), "gate sees the enclosing scope: {gate:?}");

    // The edit replaces the partial after a multibyte literal, in UTF-16 units.
    let multibyte = session.completion(line_end("&& ta"));
    let tags = item(&multibyte, "tags");
    let line = text.lines().position(|line| line.contains("&& ta")).unwrap() as u64;
    let start = text.lines().nth(line as usize).unwrap().encode_utf16().count() as u64 - 2;
    assert_eq!(tags["textEdit"]["range"]["start"], json!({ "line": line, "character": start }));
    session.shutdown();
}

// ── hover ─────────────────────────────────────────────────────────────────

/// Hover shows the library's types: the source's declared and entry types, a
/// body local, a callback parameter, and a refined field.
#[test]
fn hover_shows_the_declared_result_and_refined_types() {
    let body = concat!(
        "::map team \"p => p\"\n",
        "Row {{ p.age }} {{ p }}\n",
        "::end-map\n",
        "::filter people \"p => p.age != null\" sort=\"q => q.name\"\n",
        "Kept {{ p.age }}\n",
        "::end-filter\n",
    );
    let text = document(body, "\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);

    // The source: the declared type and one entry's type.
    let schema = schema();
    let declared = DeclaredTypes::new(Some(&schema));
    let outer = TypeScope::unguarded(&declared);
    let source = Expr::Variable("team".into());
    let SourceProjection::Entries(entry) = outer.project_source(None, &source) else { panic!("team iterates") };
    let hover = session.hover(offset_of(&text, "::map team", "team", 0));
    assert!(hover.contains(&format!("Type: `{}`", type_text(&outer.type_of_expr(&source).unwrap()))), "{hover}");
    assert!(hover.contains(&format!("Entry: `{}`", shape_text(&entry))), "{hover}");

    // A body read matches the library at that offset.
    let age = offset_of(&text, "Row {{", "p.age", 0);
    let hover = session.hover(age);
    assert!(hover.contains(&format!("Local `p.age`: `{}`", library_type(&text, age, "p.age"))), "{hover}");
    assert_eq!(library_type(&text, age, "p.age"), "number");

    // The filter's retained side refines the optional field.
    let kept = offset_of(&text, "Kept {{", "p.age", 0);
    let hover = session.hover(kept);
    assert_eq!(library_type(&text, kept, "p.age"), "number", "retained items have a non-null age");
    assert!(hover.contains("Local `p.age`: `number`"), "{hover}");

    // A callback body and a parameter, each at the callback's own scope.
    let sort = offset_of(&text, "sort=\"q => q.name\"", "q.name", 0);
    assert!(session.hover(sort).contains("Local `q.name`: `string`"), "{}", session.hover(sort));
    let parameter = offset_of(&text, "sort=\"q => q.name\"", "q", 0);
    assert!(session.hover(parameter).contains("**Parameter** `q`"), "{}", session.hover(parameter));
    // The keyword still describes the directive.
    assert!(session.hover(offset_of(&text, "::map team", "::map", 0) + 2).contains("::map"));
    session.shutdown();
}

/// A local shadows a same-named frontmatter key in hover and in the
/// undeclared-property diagnostic, whichever spelling reads it.
#[test]
fn locals_are_not_undeclared_properties_and_shadow_frontmatter() {
    let body = concat!(
        "::filter team \"person => person.active\"\n",
        "{{ person.name }} {{ local.person.name }} {{ tags }}\n",
        "::end-filter\n",
        "{{ person.name }}\n",
    );
    let text = document(body, "\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);
    let undeclared = session.ranges("dm.expression.undeclared_property");
    let outside = position(&text, offset_of(&text, "::end-filter", "person", 0));
    assert_eq!(undeclared.len(), 1, "only the read outside the body: {:#?}", session.diagnostics);
    assert_eq!(undeclared[0].0.0, u64::from(outside.0), "{undeclared:?}");

    let name = offset_of(&text, "{{ person.name", "person.name", 0);
    assert!(session.hover(name).contains("Local `person.name`: `string`"), "{}", session.hover(name));
    session.shutdown();
}

/// An escaped callback value hovers at the right authored offsets.
#[test]
fn hover_inside_an_escaped_multibyte_callback_maps_its_offsets() {
    let body = "::filter team \"p => p.name == \\\"é\\\" && p.age > 3\"\nx\n::end-filter\n";
    let text = document(body, "\r\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);
    let age = offset_of(&text, "&&", "p.age", 0);
    let hover = session.hover(age);
    assert!(hover.contains("Local `p.age`: `number`"), "{hover}");
    session.shutdown();
}

// ── folds ─────────────────────────────────────────────────────────────────

/// Both markers fold, nested regions fold, and CRLF changes nothing.
#[test]
fn collection_regions_fold_including_nested_ones_and_under_crlf() {
    let body = "::map team\nrow\n::filter tags\ninner\n::end-filter\n::end-map\n";
    let mut folds = Vec::new();
    for eol in ["\n", "\r\n"] {
        let text = document(body, eol);
        let workspace = LspWorkspace::new();
        let mut session = Session::open(&workspace, &text);
        let first = text[..text.find("::map").unwrap()].matches('\n').count() as u64;
        let found = session.folds();
        for expected in [(first, first + 5), (first + 2, first + 4)] {
            assert!(found.contains(&expected), "{eol:?}: {expected:?} in {found:?}");
        }
        folds.push(found);
        session.shutdown();
    }
    assert_eq!(folds[0], folds[1], "line endings do not change the folds");
}

// ── semantic tokens ───────────────────────────────────────────────────────

const MACRO: u32 = 0;
const PROPERTY: u32 = 3;
const STRING: u32 = 4;
const DIRECTIVE: u32 = 1 << 2;
const CLOSER: u32 = 1 << 3;

/// Keywords, closers, sources, callbacks, and options are classified, with
/// UTF-16 lengths over escaped and multibyte callback values.
#[test]
fn semantic_tokens_classify_the_four_markers_and_header_parts() {
    let callback = "\"é => é.name == \\\"ü\\\"\"";
    let body = format!("::map team {callback} sort=\"p => p.age\" sort-order=\"desc\"\nx\n::end-map\n::filter tags\ny\n::end-filter\n");
    let text = document(&body, "\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);
    let tokens = session.tokens();
    let (map_line, _) = position(&text, text.find("::map").unwrap());
    let utf16 = |text: &str| text.encode_utf16().count() as u32;

    let at = |line: u32, character: u32| tokens.iter().find(|token| token.0 == line && token.1 == character).copied();
    assert_eq!(at(map_line, 0), Some((map_line, 0, 5, MACRO, DIRECTIVE)));
    assert_eq!(at(map_line, 6), Some((map_line, 6, 4, STRING, DIRECTIVE)), "the source");
    assert_eq!(
        at(map_line, 11),
        Some((map_line, 11, utf16(callback), STRING, DIRECTIVE)),
        "the whole authored callback, escapes and multibyte included"
    );
    let sort = 11 + utf16(callback) + 1;
    assert_eq!(at(map_line, sort), Some((map_line, sort, 4, PROPERTY, DIRECTIVE)), "an option key");
    assert_eq!(at(map_line + 2, 0), Some((map_line + 2, 0, 9, MACRO, DIRECTIVE | CLOSER)), "::end-map closes");
    assert_eq!(at(map_line + 3, 0), Some((map_line + 3, 0, 8, MACRO, DIRECTIVE)));
    assert_eq!(at(map_line + 3, 9), Some((map_line + 3, 9, 4, STRING, DIRECTIVE)));
    assert_eq!(at(map_line + 5, 0), Some((map_line + 5, 0, 12, MACRO, DIRECTIVE | CLOSER)), "::end-filter closes");
    session.shutdown();
}

// ── diagnostics ───────────────────────────────────────────────────────────

/// `(name, region text, code, the authored text the finding points at)`: one
/// static defect per document, so each is the first strict defect of its own.
const STATIC_DEFECTS: &[(&str, &str, &str, &str)] = &[
    ("untyped_map", "::map nope \"é => é\"\nx\n::end-map\n", "dm.collection.untyped_map_source", "nope"),
    ("untyped_filter", "::filter nope\nx\n::end-filter\n", "dm.collection.untyped_filter_source", "nope"),
    ("scalar_source", "::filter note\nx\n::end-filter\n", "dm.collection.invalid_source", "note"),
    ("protected_name", "::map team \"ctx => ctx\"\nx\n::end-map\n", "dm.expression.invalid_callback", "ctx"),
    ("pattern", "::map team \"{ nope } => nope\"\nx\n::end-map\n", "dm.collection.pattern_mismatch", "nope"),
    ("unknown_option", "::map team bogus=\"x\"\nx\n::end-map\n", "dm.collection.malformed", "bogus"),
    ("unclosed", "::map team\nx\n", "dm.collection.malformed", "::map team"),
    ("sort_key", "::map team sort=\"p => p.active\"\nx\n::end-map\n", "dm.collection.invalid_sort_key", "p.active"),
    ("escaped_callback", "::map team \"é => é.name == \\\"ü\\\" )\"\nx\n::end-map\n", "dm.expression.invalid_callback", ")"),
];

/// Opens every `(name, text)` in one session and returns each document's
/// published diagnostics.
fn published(workspace: &LspWorkspace, documents: &[(String, String)]) -> Vec<Vec<Value>> {
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(workspace.path()));
    let found = documents
        .iter()
        .map(|(name, text)| {
            let path = workspace.path().join(format!("{name}.md"));
            std::fs::write(&path, text).unwrap();
            let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
            fixture.notify(
                "textDocument/didOpen",
                json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
            );
            fixture.flush_server();
            let mut latest = fixture.wait_for_diagnostics_params(&uri);
            while let Some(params) = fixture.take_buffered_diagnostics(&uri) {
                latest = params;
            }
            latest["diagnostics"].as_array().cloned().unwrap_or_default()
        })
        .collect();
    fixture.shutdown();
    found
}

/// The code and one-based `(line, column)` composition reports for the
/// document at `path`, or `None` when it composes.
fn composed(workspace: &LspWorkspace, path: &std::path::Path) -> Option<(String, Option<(usize, usize)>)> {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};

    let dir = workspace.path();
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.to_path_buf());
    let snapshot = RequestSnapshot::new(dir).with_home(Some(dir.join("home")));
    let request = ComposeRequest::prepare(options, &snapshot).expect("request");
    let error = Markdown::try_from(path).expect("document loads").compose_with(&request).err()?;
    let first = error_diagnostics(&error).into_iter().next()?;
    Some((first.code, first.source.map(|source| (source.line, source.column))))
}

/// Library and editor agree on every static defect: the same code, at the
/// location composition reports, once; the editor's range covers exactly the
/// authored text (escaped and multibyte callbacks included).
fn static_defects_agree(eol: &str) {
    let workspace = LspWorkspace::new();
    let documents: Vec<(String, String)> = STATIC_DEFECTS
        .iter()
        .map(|(name, region, _, _)| ((*name).to_string(), document(region, eol)))
        .collect();
    let found = published(&workspace, &documents);
    let mut disagreements = Vec::new();
    for (((name, text), diagnostics), (_, region, code, authored)) in documents.iter().zip(&found).zip(STATIC_DEFECTS) {
        let collection: Vec<&Value> = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic["code"].as_str().is_some_and(|found| found.starts_with("dm.collection.") || found == "dm.expression.invalid_callback")
            })
            .collect();
        let library = composed(&workspace, &workspace.path().join(format!("{name}.md")));
        let editor: Vec<(String, (usize, usize))> = collection
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic["code"].as_str().unwrap().to_string(),
                    (
                        diagnostic["range"]["start"]["line"].as_u64().unwrap() as usize + 1,
                        diagnostic["range"]["start"]["character"].as_u64().unwrap() as usize + 1,
                    ),
                )
            })
            .collect();
        let region_start = text.find(&region.replace('\n', eol)).expect("the region is in the document");
        let expected_start = region.find(authored).map(|offset| region_start + region[..offset].replace('\n', eol).len()).map(|offset| {
            let (line, character) = position(text, offset);
            (line as usize + 1, character as usize + 1)
        });
        let library_code = library.as_ref().map(|(code, _)| code.as_str());
        let library_at = library.as_ref().and_then(|(_, at)| *at);
        if library_code != Some(code)
            || editor.len() != 1
            || editor[0].0 != *code
            || Some(editor[0].1) != library_at
            || Some(editor[0].1) != expected_start
        {
            disagreements.push(format!("{name}: library {library:?}, editor {editor:?}, expected {code} at {expected_start:?}"));
        }
    }
    assert!(disagreements.is_empty(), "{}", disagreements.join("\n"));
}

#[test]
fn static_defects_agree_with_composition_on_code_and_location() {
    static_defects_agree("\n");
}

#[test]
fn static_defects_agree_with_composition_under_crlf() {
    static_defects_agree("\r\n");
}

/// A call error in an escaped, multibyte callback is ranged on the argument
/// in authored UTF-16 columns.
#[test]
fn call_errors_in_callbacks_are_ranged_through_the_escapes() {
    let body = "::map team \"p => p.name == \\\"é\\\" && min(p.active, 5) > 0\"\nx\n::end-map\n";
    let text = document(body, "\r\n");
    let workspace = LspWorkspace::new();
    let session = Session::open(&workspace, &text);
    let argument = offset_of(&text, "min(", "p.active", 0);
    let expected = (position(&text, argument), position(&text, argument + "p.active".len()));
    let ranges = session.ranges("dm.expression.incompatible_call");
    assert_eq!(
        ranges,
        vec![(
            (u64::from(expected.0.0), u64::from(expected.0.1)),
            (u64::from(expected.1.0), u64::from(expected.1.1))
        )],
        "{:#?}",
        session.diagnostics
    );
    session.shutdown();
}

/// The editor reads the library's one identity for an original item's field:
/// a guard fact recorded through `p.members` proves a destructured `members`
/// and the reverse, so neither document publishes a collection finding, while
/// an inner region's own `members` stays unproven.
#[test]
fn guard_facts_cross_destructured_and_object_spellings_in_editor_diagnostics() {
    let head = "---\n$schema:\n  data: '{ members: any(required) }[](required)'\ndata: [{members: [a, b]}]\n---\n";
    let documents: Vec<(String, String)> = [
        ("object_guard", "::filter data \"{members} => true\" filter=\"p => is_array(p.members)\"\n::map members\n- {{i}}\n::end-map\n::end-filter\n"),
        ("destructured_guard", "::filter data \"p => true\" until=\"{members} => !is_array(members)\"\n::map p.members\n- {{i}}\n::end-map\n::end-filter\n"),
        (
            "shadowed",
            "::filter data \"p => true\" filter=\"p => is_array(p.members)\"\n::map data \"{members} => {members}\"\n::map members\n- {{i}}\n::end-map\n::end-map\n::end-filter\n",
        ),
    ]
    .iter()
    .map(|(name, body)| ((*name).to_string(), format!("{head}{body}")))
    .collect();
    let workspace = LspWorkspace::new();
    let found = published(&workspace, &documents);
    let collection_codes = |diagnostics: &[Value]| -> Vec<String> {
        diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic["code"].as_str())
            .filter(|code| code.starts_with("dm.collection."))
            .map(str::to_string)
            .collect()
    };
    assert!(collection_codes(&found[0]).is_empty(), "{:#?}", found[0]);
    assert!(collection_codes(&found[1]).is_empty(), "{:#?}", found[1]);
    assert_eq!(collection_codes(&found[2]), ["dm.collection.untyped_map_source"], "{:#?}", found[2]);
}
