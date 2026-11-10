//! Undeclared-member warnings in the editor: a constant member read whose
//! base has a known closed object type that does not declare the member is
//! one `dm.expression.undeclared_member` warning, ranged on the member name,
//! with Darkmatter's message, on every surface that shows call diagnostics.
//!
//! The editor checks every branch and callback body, whatever a run would
//! reach, so it reports reads composition may never evaluate. A document
//! whose schema still holds unresolved slots is typed from its template.
//!
//! Boundary: the LSP protocol (an in-memory `Connection` driving the real
//! server, router, overlay, and slot worker) and the filesystem (the
//! workspace's documents and data files). F2, L1: no terminal, no editor
//! process, and no window.

use std::path::{Path, PathBuf};

use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const MEMBER: &str = "dm.expression.undeclared_member";
const WARNING: u64 = 2;
const COMPOSE: &str = "darkmatter.compose";
const FRONTMATTER: &str = "darkmatter.frontmatter";

fn initialize_params(root: &Path, encoding: &str) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": {
            "general": { "positionEncodings": [encoding] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true, "relativePatternSupport": true } }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "members" } ]
    })
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().as_str().to_string()
}

/// `(line, start, end, source, severity, message)` of one member warning.
type Reported = (u64, u64, u64, String, u64, String);

/// Every member warning in `diagnostics`, sorted.
fn members(diagnostics: &[Value]) -> Vec<Reported> {
    let mut found: Vec<Reported> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!(MEMBER))
        .map(|diagnostic| {
            let range = &diagnostic["range"];
            assert_eq!(range["start"]["line"], range["end"]["line"], "{diagnostic:#?}");
            assert_eq!(diagnostic["data"], json!({ "expected_type": null, "actual_type": null }), "{diagnostic:#?}");
            (
                range["start"]["line"].as_u64().unwrap(),
                range["start"]["character"].as_u64().unwrap(),
                range["end"]["character"].as_u64().unwrap(),
                diagnostic["source"].as_str().unwrap().to_string(),
                diagnostic["severity"].as_u64().unwrap(),
                diagnostic["message"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

fn message(member: &str, subject: &str) -> String {
    format!("Member '{member}' is not declared by the known object type of '{subject}'.")
}

/// The expected warning at the `nth` (0-based) `needle` on the line starting
/// with `marker`, in UTF-8 bytes.
fn at(doc: &str, marker: &str, needle: &str, nth: usize, source: &str, subject: &str) -> Reported {
    let (line, text) = doc
        .lines()
        .enumerate()
        .find(|(_, text)| text.starts_with(marker))
        .unwrap_or_else(|| panic!("no line starting with {marker:?}"));
    let start = text.match_indices(needle).nth(nth).unwrap_or_else(|| panic!("{needle} #{nth} not on {marker:?}")).0;
    (line as u64, start as u64, (start + needle.len()) as u64, source.to_string(), WARNING, message(needle, subject))
}

struct Session<'w> {
    fixture: LspFixture<'w>,
    path: PathBuf,
}

impl<'w> Session<'w> {
    fn open(workspace: &'w LspWorkspace, text: &str) -> Self {
        Self::open_with(workspace, text, "utf-8")
    }

    fn open_with(workspace: &'w LspWorkspace, text: &str, encoding: &str) -> Self {
        let path = workspace.path().join("doc.md");
        std::fs::write(&path, text).unwrap();
        let mut fixture = LspFixture::start(workspace);
        fixture.initialize(initialize_params(workspace.path(), encoding));
        fixture.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri(&path), "languageId": "markdown", "version": 1, "text": text } }),
        );
        Self { fixture, path }
    }

    /// The latest published diagnostics, after the server handled everything
    /// sent so far.
    fn diagnostics(&mut self) -> Vec<Value> {
        let uri = uri(&self.path);
        self.fixture.flush_server();
        let mut latest = self.fixture.wait_for_diagnostics_params(&uri);
        while let Some(params) = self.fixture.take_buffered_diagnostics(&uri) {
            latest = params;
        }
        latest["diagnostics"].as_array().cloned().unwrap_or_default()
    }

    fn change(&self, text: &str, version: i32) {
        self.fixture.notify(
            "textDocument/didChange",
            json!({ "textDocument": { "uri": uri(&self.path), "version": version }, "contentChanges": [ { "text": text } ] }),
        );
    }

    fn shutdown(self) {
        self.fixture.shutdown();
    }
}

/// The member warnings of `text` opened in a fresh workspace holding
/// `files`.
fn reported(text: &str, files: &[(&str, &str)]) -> (Vec<Reported>, Vec<Value>) {
    let workspace = LspWorkspace::new();
    for (name, content) in files {
        std::fs::write(workspace.path().join(name), content).unwrap();
    }
    let mut session = Session::open(&workspace, text);
    let diagnostics = session.diagnostics();
    session.shutdown();
    (members(&diagnostics), diagnostics)
}

/// Every surface that shows call diagnostics, each reading `nmae` from a
/// closed object: body interpolation, a multiline frontmatter
/// interpolation, an `expression`-typed frontmatter value, `::block when`,
/// `::file when`, a `::map` callback, and a `::filter` gate.
fn every_surface(member: &str) -> String {
    format!(
        "---
$schema:
  server: \"{{ name: string, port: number }}\"
  items: \"{{ name: string }}[]\"
  gate: expression
server: {{ name: web, port: 80 }}
items:
  - name: a
gate: server.{member}
multi: |-
  first {{{{ server.name }}}}
  second {{{{ server.{member} }}}}
---

B1 {{{{ server.{member} }}}}
::block when=\"server.{member}\"
inside
::end-block
::file ./other.md when=\"server.{member}\"
::map items \"h => h.{member}\"
Row
::end-map
::filter items \"h => h.{member}\"
Kept
::end-filter
"
    )
}

#[test]
fn reports_on_every_surface() {
    let files = [("other.md", "Other.\n")];
    // Control: the declared member is silent everywhere, so each edit below
    // is load-bearing.
    let (control, diagnostics) = reported(&every_surface("name"), &files);
    assert_eq!(control, [], "{diagnostics:#?}");

    let doc = every_surface("nmae");
    let (found, diagnostics) = reported(&doc, &files);
    let mut expected = vec![
        at(&doc, "gate:", "nmae", 0, FRONTMATTER, "server"),
        at(&doc, "  second", "nmae", 0, FRONTMATTER, "server"),
        at(&doc, "B1", "nmae", 0, COMPOSE, "server"),
        at(&doc, "::block", "nmae", 0, COMPOSE, "server"),
        at(&doc, "::file", "nmae", 0, COMPOSE, "server"),
        at(&doc, "::map", "nmae", 0, COMPOSE, "h"),
        at(&doc, "::filter", "nmae", 0, COMPOSE, "h"),
    ];
    expected.sort();
    assert_eq!(found, expected, "{diagnostics:#?}");
}

/// The acceptance example's expression in the body.
const AC1: &str = "{{ map(load_data(\"hosts.yaml\", \"{ name: string, port: number }[]\"), h => h.nmae) }}";

const THREE_HOSTS: &str = "- name: web\n  port: 80\n- name: db\n  port: 5432\n- name: cache\n  port: 6379\n";

#[test]
fn ac1_editor() {
    let doc = format!("---\ntitle: Hosts\n---\n\nH {AC1}\n");
    let expected = vec![at(&doc, "H ", "nmae", 0, COMPOSE, "h")];
    for hosts in [THREE_HOSTS, "[]\n"] {
        let (found, diagnostics) = reported(&doc, &[("hosts.yaml", hosts)]);
        assert_eq!(found, expected, "hosts = {hosts:?}: {diagnostics:#?}");
    }
    // Control: the declared member is silent.
    let (found, diagnostics) = reported(&doc.replace("h.nmae", "h.name"), &[("hosts.yaml", THREE_HOSTS)]);
    assert_eq!(found, [], "{diagnostics:#?}");

    // A computed contract gives the items no known type.
    let computed = "---\nkind: \"{ name: string }[]\"\n---\n\nH {{ map(load_data(\"hosts.yaml\", kind), h => h.nmae) }}\n";
    let (found, diagnostics) = reported(computed, &[("hosts.yaml", THREE_HOSTS)]);
    assert_eq!(found, [], "{diagnostics:#?}");
}

#[test]
fn all_branches() {
    let doc = "---
$schema:
  server: \"{ name: string }\"
  empty: \"{ name: string }[]\"
server: { name: web }
empty: []
---

T {{ true ? server.name : server.nmae }}
C {{ map(empty, h => h.nmae) }}
";
    let (found, diagnostics) = reported(doc, &[]);
    assert_eq!(
        found,
        [at(doc, "T ", "nmae", 0, COMPOSE, "server"), at(doc, "C ", "nmae", 0, COMPOSE, "h")],
        "the unchosen branch and a callback over an empty list: {diagnostics:#?}"
    );
}

#[test]
fn positions() {
    const HEAD: &str = "---\n$schema:\n  server: \"{ name: string }\"\nserver: { name: web }\n---\n\n";
    let body = "é {{ server.nmae }}\n😀 {{ server.nmae }}\n";
    let text = format!("{HEAD}{body}");
    let workspace = LspWorkspace::new();
    let mut session = Session::open_with(&workspace, &text, "utf-16");
    let first = HEAD.matches('\n').count() as u64;
    // `é` is one UTF-16 unit and `😀` two; `{{ server.` is 10 more.
    let utf16 = |line: u64, start: u64| (line, start, start + 4, COMPOSE.to_string(), WARNING, message("nmae", "server"));
    let diagnostics = session.diagnostics();
    assert_eq!(members(&diagnostics), [utf16(first, 12), utf16(first + 1, 13)], "{diagnostics:#?}");

    // A body edit above moves both warnings down one line.
    session.change(&format!("{HEAD}Inserted line.\n{body}"), 2);
    let diagnostics = session.diagnostics();
    assert_eq!(members(&diagnostics), [utf16(first + 1, 12), utf16(first + 2, 13)], "{diagnostics:#?}");
    session.shutdown();
}

#[test]
fn strict_independent() {
    let doc = "---
$schema:
  server: \"{ name: string }\"
  maybe: number
server: { name: web }
maybe: 3
---

S {{ server.nmae }} {{ min(maybe, 5) }}
";
    let expected = vec![at(doc, "S ", "nmae", 0, COMPOSE, "server")];
    let mut uncertain = Vec::new();
    for strict in [false, true] {
        let workspace = LspWorkspace::new();
        std::fs::write(workspace.path().join(".dmls.toml"), format!("[expressions]\nstrict = {strict}\n")).unwrap();
        let mut session = Session::open(&workspace, doc);
        let diagnostics = session.diagnostics();
        session.shutdown();
        assert_eq!(members(&diagnostics), expected, "strict = {strict}: {diagnostics:#?}");
        uncertain.push(diagnostics.iter().any(|diagnostic| diagnostic["code"] == json!("dm.expression.uncertain_call")));
    }
    // Control: the setting is live; strict shows the nullable call argument.
    assert_eq!(uncertain, [false, true]);
}

#[test]
fn lifecycle_unchanged() {
    let doc = "---
$schema:
  server: \"{ name: string }\"
server: { name: web }
note: \"{{ server.nmae }}\"
composed:
  - info: \"composed {{ server.nmae }}\"
---

Body.
";
    let (found, diagnostics) = reported(doc, &[]);
    assert_eq!(
        found,
        [at(doc, "note:", "nmae", 0, FRONTMATTER, "server")],
        "only the value outside the lifecycle stack: {diagnostics:#?}"
    );
}

/// The pending-template schema: `role` waits on a slot, `server` is a closed
/// object whose other member is literal.
const PENDING: &str = "---
$schema:
  server: \"{ name: string, role: enum({{ [\\\"web\\\", \\\"db\\\"] }}; required) }\"
server: { name: web, role: web }
---

P {{ server.nmae }}
";

fn slot_params(root: &Path) -> Value {
    initialize_params(root, "utf-8")
}

#[test]
fn pending_template() {
    let workspace = LspWorkspace::new();
    let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
    let doc = root.join("doc.md");
    std::fs::write(&doc, PENDING).unwrap();
    let snapshot =
        darkmatter::markdown::compose::RequestSnapshot::new(&root).with_home(Some(root.join("home")));
    let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot);
    fixture.initialize(slot_params(&root));
    let before = darkmatter::work_counters::snapshot().get(darkmatter::work_counters::Work::SlotResolution);
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(&doc), "languageId": "markdown", "version": 1, "text": PENDING } }),
    );
    fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);
    fixture.flush_server();
    let pending = fixture.wait_for_diagnostics(&uri(&doc));
    let codes: Vec<_> = pending.iter().filter_map(|diagnostic| diagnostic["code"].as_str()).collect();
    assert!(codes.contains(&"dm.schema.slot_pending"), "no snapshot is published yet: {pending:#?}");
    let expected = vec![at(PENDING, "P ", "nmae", 0, COMPOSE, "server")];
    assert_eq!(members(&pending), expected, "typed from the template: {pending:#?}");
    assert_eq!(
        darkmatter::work_counters::snapshot().get(darkmatter::work_counters::Work::SlotResolution),
        before,
        "no request evaluated a slot"
    );

    // Resolved: the same one warning from the published snapshot.
    fixture.release_slot_job();
    assert!(fixture.wait_for_slot_prepared());
    fixture.flush_server();
    let resolved = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(
        !resolved.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.slot_pending")),
        "{resolved:#?}"
    );
    assert_eq!(members(&resolved), expected, "{resolved:#?}");
    fixture.shutdown();
}

#[test]
fn pending_template_refresh_reports_once() {
    let workspace = LspWorkspace::new();
    let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
    let data = root.join("roles.yaml");
    std::fs::write(&data, "- web\n").unwrap();
    let store = root.join("home").join(".config").join("darkmatter").join("schema-read-policy");
    std::fs::create_dir_all(store.parent().unwrap()).unwrap();
    std::fs::write(&store, format!("allow file {}\n", data.display())).unwrap();
    let text = "---
$schema:
  server: \"{ name: string, role: enum({{ load_data(\\\"./roles.yaml\\\", \\\"string[]\\\") }}) }\"
server: { name: web, role: web }
---

P {{ server.nmae }}
";
    let doc = root.join("doc.md");
    std::fs::write(&doc, text).unwrap();
    let snapshot =
        darkmatter::markdown::compose::RequestSnapshot::new(&root).with_home(Some(root.join("home")));
    let mut fixture = LspFixture::start_with_slot_gate(&workspace, snapshot);
    fixture.initialize(slot_params(&root));
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(&doc), "languageId": "markdown", "version": 1, "text": text } }),
    );
    fixture.release_slot_job();
    assert!(fixture.wait_for_slot_prepared());
    fixture.flush_server();
    let expected = vec![at(text, "P ", "nmae", 0, COMPOSE, "server")];
    let first = fixture.wait_for_diagnostics(&uri(&doc));
    assert!(!first.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.slot_pending")), "{first:#?}");
    assert_eq!(members(&first), expected, "{first:#?}");

    // A refresh is held: the last-good snapshot is served, marked stale, and
    // the warning is still reported once.
    std::fs::write(&data, "- web\n- db\n").unwrap();
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": [ { "uri": uri(&data), "type": 2 } ] }));
    fixture.wait_for_event(dmls::StartupEvent::SlotPreparing);
    fixture.round_trip();
    let held = fixture.wait_for_diagnostics(&uri(&doc));
    assert_eq!(
        held.iter().filter(|diagnostic| diagnostic["code"] == json!("dm.schema.slot_pending")).count(),
        1,
        "control: the snapshot is refreshing: {held:#?}"
    );
    assert_eq!(members(&held), expected, "no duplicate while refreshing: {held:#?}");
    fixture.release_slot_job();
    assert!(fixture.wait_for_slot_prepared());
    fixture.shutdown();
}

/// Approves every shell command, so a frontmatter `$( … )` value composes.
struct AllowAll;

impl darkmatter::markdown::compose::shell_expansion::types::ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: darkmatter::markdown::compose::shell_expansion::types::ShellApprovalRequest,
    ) -> Result<
        darkmatter::markdown::compose::shell_expansion::types::ShellApprovalDecision,
        darkmatter::markdown::compose::ShellExpansionError,
    > {
        Ok(darkmatter::markdown::compose::shell_expansion::types::ShellApprovalDecision::AllowOnce)
    }
}

/// The member warnings composition reports for `text` as `doc.md` in a
/// fresh workspace holding `files`, and those DMLS publishes for it.
fn library_and_editor(text: &str, files: &[(&str, &str)]) -> (Vec<darkmatter::markdown::compose::expression::Diagnostic>, Vec<Value>) {
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};

    let workspace = LspWorkspace::new();
    let dir = workspace.path();
    for (name, content) in files {
        std::fs::write(dir.join(name), content).unwrap();
    }
    let path = dir.join("doc.md");
    std::fs::write(&path, text).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone())
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(std::sync::Arc::new(AllowAll));
    let snapshot = RequestSnapshot::new(dir).with_home(Some(dir.join("home")));
    let request = ComposeRequest::prepare(options, &snapshot).expect("request");
    let (_, report) = Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with(&request)
        .unwrap_or_else(|error| panic!("{text:?} failed to compose: {error}"));
    let composed: Vec<_> = report
        .warnings
        .iter()
        .filter(|warning| warning.code.as_deref() == Some(MEMBER))
        .map(|warning| *warning.diagnostic.clone().expect("a member warning carries its diagnostic"))
        .collect();

    let mut session = Session::open(&workspace, text);
    let diagnostics = session.diagnostics();
    session.shutdown();
    let edited = diagnostics.into_iter().filter(|diagnostic| diagnostic["code"] == json!(MEMBER)).collect();
    (composed, edited)
}

/// Composition's warning and DMLS's diagnostic for `text` are the same
/// finding: code, message, severity, and start position (one-based line and
/// character column against zero-based LSP positions; the fixtures are
/// ASCII, so the editor's UTF-8 offsets are characters).
#[track_caller]
fn assert_agree(text: &str, files: &[(&str, &str)]) {
    use darkmatter::markdown::compose::expression::Severity;

    // Control: the declared member is silent on both sides.
    let (composed, edited) = library_and_editor(&text.replace("nmae", "name"), files);
    assert!(composed.is_empty() && edited.is_empty(), "control: {composed:#?} {edited:#?}");

    let (composed, edited) = library_and_editor(text, files);
    let ([library], [editor]) = (composed.as_slice(), edited.as_slice()) else {
        panic!("one reached member on each side: composition {composed:#?}, editor {edited:#?}");
    };
    assert_eq!(library.code, editor["code"].as_str().unwrap(), "{library:#?} {editor:#?}");
    assert_eq!(library.message, editor["message"].as_str().unwrap(), "{library:#?} {editor:#?}");
    assert_eq!(library.severity, Severity::Warning, "{library:#?}");
    assert_eq!(editor["severity"], json!(WARNING), "{editor:#?}");
    let source = library.source.as_ref().expect("composition locates the member");
    let range = &editor["range"];
    let (line, start, end) = (
        range["start"]["line"].as_u64().unwrap() as usize,
        range["start"]["character"].as_u64().unwrap() as usize,
        range["end"]["character"].as_u64().unwrap() as usize,
    );
    assert_eq!(source.line.map(|line| line - 1), Some(line), "line: {library:#?} {editor:#?}");
    assert_eq!(source.column.map(|column| column - 1), Some(start), "column: {library:#?} {editor:#?}");
    // Composition's diagnostic carries only the start (`span` is `None` for
    // a member warning), so the extent is the editor's alone.
    assert_eq!(end - start, "nmae".len(), "{editor:#?}");
}

/// Library/editor agreement (AC7): each surface holds one reached
/// undeclared member, which composition and DMLS report identically.
mod library_and_editor_agree {
    use super::assert_agree;

    /// Three rows matching the closed `{ name: string }` contract.
    const NAMED_HOSTS: &str = "- name: web\n- name: db\n- name: cache\n";

    const HEAD: &str = "---\n$schema:\n  server: \"{ name: string, port: number }\"\n  hosts: \"{ name: string }[]\"\nserver: { name: web, port: 80 }\nhosts: [{ name: a }, { name: b }, { name: c }]\n";

    fn doc(extra: &str, body: &str) -> String {
        format!("{HEAD}{extra}---\n\n{body}")
    }

    macro_rules! surfaces {
        ($($name:ident: $extra:expr, $body:expr, $files:expr;)+) => {
            $(
                #[test]
                fn $name() {
                    assert_agree(&doc($extra, $body), $files);
                }
            )+
        };
    }

    surfaces! {
        body_interpolation: "", "B {{ server.nmae }}\n", &[];
        frontmatter_interpolation: "title: \"x {{ server.nmae }}\"\n", "{{ title }}\n", &[];
        block_condition: "", "::block when=\"server.nmae\"\ninside\n::end-block\n", &[];
        file_condition: "", "::file ./child.md when=\"server.nmae\"\n", &[("child.md", "Child.\n")];
        file_condition_beside_placeholder: "expected: web\n",
            "::file ./child.md when=\"server.nmae != '{{ expected }}'\"\n", &[("child.md", "Child.\n")];
        map_callback: "", "::map hosts \"h => h.nmae\"\n{{ h }}\n::end-map\n", &[];
        filter_callback: "", "::filter hosts \"h => h.nmae\"\n{{ h.name }}\n::end-filter\n", &[];
        map_region_body: "", "::map hosts\n{{ i.nmae }}\n::end-map\n", &[];
        load_data_callback: "",
            "H {{ map(load_data(\"hosts.yaml\", \"{ name: string }[]\"), h => h.nmae) }}\n",
            &[("hosts.yaml", NAMED_HOSTS)];
        shell_placeholder: "pick: \"$( echo {{ server.nmae }} )\"\n", "{{ pick }}\n", &[];
        shell_condition: "pick: \"$( server.nmae == 'x' ? echo yes : '' )\"\n", "{{ pick }}\n", &[];
        shell_then: "pick: \"$( server.name ? fm.server.nmae : echo no )\"\n", "{{ pick }}\n", &[];
        shell_else: "pick: \"$( server.port < 0 ? echo no : fm.server.nmae )\"\n", "{{ pick }}\n", &[];
        shell_condition_beside_placeholder:
            "expected: web\npick: \"$( server.nmae == '{{ expected }}' ? echo yes : '' )\"\n", "{{ pick }}\n", &[];
        shell_then_beside_placeholder:
            "expected: web\npick: \"$( server.name ? length([fm.server.nmae, '{{ expected }}']) : echo no )\"\n",
            "{{ pick }}\n", &[];
        shell_else_beside_placeholder:
            "expected: web\npick: \"$( server.port < 0 ? echo no : length([fm.server.nmae, '{{ expected }}']) )\"\n",
            "{{ pick }}\n", &[];
        // The read inside the placeholder is the placeholder's site alone;
        // the piece sees it as string text.
        shell_member_inside_piece_placeholder:
            "pick: \"$( server.name == '{{ server.nmae }}' ? echo yes : '' )\"\n", "{{ pick }}\n", &[];
    }
}

/// The editor checks both value branches of a frontmatter `$( … )`
/// ternary, whichever a run selects, without running a command; a piece
/// whose placeholder lies outside a string is not checked, because filling
/// it changes the expression composition evaluates.
#[test]
fn shell_ternary_pieces_in_the_editor() {
    let doc = "---
$schema:
  server: \"{ name: string, port: number }\"
server: { name: web, port: 80 }
expected: web
unselected_then: \"$( server.port < 0 ? fm.server.nmae : echo no )\"
unselected_else: \"$( server.port > 0 ? echo yes : fm.server.nmae )\"
unstructured: \"$( server.nmae == {{ expected }} ? echo yes : echo no )\"
pipeline: \"$( echo server.nmae )\"
---

Body.
";
    let (found, diagnostics) = reported(doc, &[]);
    assert_eq!(
        found,
        [
            at(doc, "unselected_then:", "nmae", 0, FRONTMATTER, "fm.server"),
            at(doc, "unselected_else:", "nmae", 0, FRONTMATTER, "fm.server"),
        ],
        "{diagnostics:#?}"
    );
    let (composed, _) = library_and_editor(doc, &[]);
    assert!(composed.is_empty(), "composition reaches neither unselected branch: {composed:#?}");
}

/// An unchosen ternary branch is checked by the editor and never reached by
/// composition.
#[test]
fn reachability_differs() {
    let text = "---\n$schema:\n  server: \"{ name: string, port: number }\"\nserver: { name: web, port: 80 }\n---\n\nT {{ server.port > 0 ? server.name : server.nmae }}\n";
    let (composed, edited) = library_and_editor(text, &[]);
    assert!(composed.is_empty(), "composition never evaluates the unchosen branch: {composed:#?}");
    assert_eq!(members(&edited), [at(text, "T ", "nmae", 0, COMPOSE, "server")], "{edited:#?}");
}

