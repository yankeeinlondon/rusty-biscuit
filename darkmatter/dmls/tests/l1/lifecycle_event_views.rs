//! An expression inside a lifecycle stack value reads its event's binding
//! view from the `md` event registry: diagnostics, hover, and completion all
//! agree with `registry.globals(id)`, never with a list kept in DMLS. A
//! registered host's globals are execution-dependent everywhere; a catalog
//! in a schema root declares none.

use darkmatter::lifecycle::md_host;
use darkmatter::markdown::compose::expression::Availability;
use serde_json::{Value, json};

use crate::common::{LspFixture, LspWorkspace};
use crate::union_item_editor::{Document, initialize_params, offset_of, open_text, request};

/// A repository whose tree-root `schemas/` holds `catalogs`.
fn session<'w>(workspace: &'w LspWorkspace, catalogs: &[(&str, &str)]) -> LspFixture<'w> {
    let root = workspace.path();
    for dir in [".git/objects", ".git/refs/heads", "schemas"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    for (name, text) in catalogs {
        std::fs::write(root.join("schemas").join(name), text).unwrap();
    }
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(root));
    fixture
}

fn with_code<'d>(diagnostics: &'d [Value], code: &str) -> Vec<&'d Value> {
    diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!(code)).collect()
}

fn line_of(diagnostic: &Value) -> u64 {
    diagnostic["range"]["start"]["line"].as_u64().unwrap()
}

fn line_containing(text: &str, needle: &str) -> u64 {
    text[..offset_of(text, needle, false)].matches('\n').count() as u64
}

/// The reason code the registry gives `err` in `event`, so the test asserts
/// the registry's reason rather than a spelling of its own.
fn err_reason(event: &str) -> String {
    let registry = md_host::registry();
    let id = registry.id(event).unwrap_or_else(|| panic!("`{event}` is an md event"));
    match registry.err_availability(id) {
        Some(Availability::Unavailable(reason)) => reason.code().to_string(),
        other => panic!("`err` in `{event}` is {other:?}"),
    }
}

fn err_is_unavailable(event: &str) -> bool {
    let registry = md_host::registry();
    let id = registry.id(event).unwrap();
    matches!(registry.err_availability(id), Some(Availability::Unavailable(_)))
}

fn hover_text(fixture: &mut LspFixture<'_>, document: &Document, offset: usize) -> String {
    let hover = request(fixture, "textDocument/hover", document, offset);
    hover["contents"]["value"].as_str().unwrap_or_default().to_string()
}

fn labels(result: &Value) -> Vec<String> {
    let items = result.get("items").unwrap_or(result);
    items
        .as_array()
        .map(|items| items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// `err` in a stack whose event has no error is a diagnostic carrying the
/// view's scope and reason, in an operand and a `when` guard alike, even
/// with an `err` frontmatter key to fall back to. In `failed` and `finalize`
/// it is not, and outside every stack nothing changes.
#[test]
fn err_reads_follow_each_events_view() {
    for event in ["initialize", "composed"] {
        assert!(err_is_unavailable(event), "{event}");
    }
    for event in ["failed", "finalize"] {
        assert!(!err_is_unavailable(event), "{event}");
    }
    let text = concat!(
        "---\n",
        "title: Views\n",
        "err: shadow\n",
        "notes: \"{{ err.message }}\"\n",
        "initialize:\n",
        "  - when: err\n",
        "    then:\n",
        "      - info: guarded\n",
        "composed:\n",
        "  - info: \"composed {{ err.message }}\"\n",
        "failed:\n",
        "  - info: \"failed {{ err.message }}\"\n",
        "finalize:\n",
        "  - when: err\n",
        "    then:\n",
        "      - info: \"finalize {{ err.message }} {{ timing.document_ms }}\"\n",
        "---\n\n",
        "Body {{ title }}\n",
    );
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, &[]);
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "views.md", text);

    let unavailable = with_code(&diagnostics, "dm.expression.unavailable_global");
    assert_eq!(unavailable.len(), 2, "{diagnostics:#?}");
    for (needle, event) in [("  - when: err\n    then:\n      - info: guarded", "initialize"), ("composed {{", "composed")] {
        let line = line_containing(text, needle);
        let found = unavailable.iter().find(|diagnostic| line_of(diagnostic) == line);
        let message = found.unwrap_or_else(|| panic!("{event}: {unavailable:#?}"))["message"].as_str().unwrap();
        assert!(message.contains(&format!("md.{event}")) && message.contains(&err_reason(event)), "{message}");
        assert_eq!(found.unwrap()["severity"], json!(1), "an unavailable read is an error");
    }
    // `failed`, `finalize`, and the plain `notes` value report nothing about
    // `err`: no unavailable read and no undeclared property.
    let expression_codes: Vec<&Value> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"].as_str().is_some_and(|code| code.starts_with("dm.expression.")))
        .collect();
    assert_eq!(expression_codes.len(), 2, "{expression_codes:#?}");
    fixture.shutdown();
}

/// Hover on a stack's `err` shows the event's declaration and availability,
/// not the shadowed frontmatter value; outside a stack it is unchanged.
#[test]
fn hover_on_a_stack_global_shows_the_events_availability() {
    let text = concat!(
        "---\n",
        "err: shadow\n",
        "notes: \"{{ err }}\"\n",
        "composed:\n",
        "  - info: \"composed {{ err.msg }}\"\n",
        "failed:\n",
        "  - when: err\n",
        "    then:\n",
        "      - info: \"failed {{ err.msg }}\"\n",
        "---\n\n",
        "Body {{ err }}\n",
    );
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, &[]);
    let (document, _) = open_text(&mut fixture, &workspace, "hover.md", text);

    let composed = hover_text(&mut fixture, &document, offset_of(text, "composed {{ e", true));
    assert!(composed.contains("In `composed`: unavailable") && composed.contains(&err_reason("composed")), "{composed}");
    assert!(!composed.contains("shadow"), "{composed}");
    let member = hover_text(&mut fixture, &document, offset_of(text, "composed {{ err.m", true));
    assert!(member.contains("**`err.msg`**"), "{member}");

    let failed = hover_text(&mut fixture, &document, offset_of(text, "failed {{ e", true));
    assert!(failed.contains("In `failed`: available"), "{failed}");
    let guard = hover_text(&mut fixture, &document, offset_of(text, "  - when: e", true));
    assert!(guard.contains("In `failed`: available"), "{guard}");

    let body = hover_text(&mut fixture, &document, offset_of(text, "Body {{ e", true));
    assert!(body.contains("Static value: `shadow`") && !body.contains("In `"), "{body}");
    fixture.shutdown();
}

/// Completion inside a stack offers the globals its event may read: `err` in
/// `failed` with its catalog members, not in `composed`; never outside a stack.
#[test]
fn completion_offers_only_the_globals_an_event_may_read() {
    let text = concat!(
        "---\n",
        "title: Completion\n",
        "composed:\n",
        "  - info: \"composed {{ er }} {{ tim }}\"\n",
        "failed:\n",
        "  - info: \"failed {{ er }} {{ err.ms }}\"\n",
        "---\n\n",
        "Body {{ er }}\n",
    );
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, &[]);
    let (document, _) = open_text(&mut fixture, &workspace, "complete.md", text);
    let mut at = |needle: &str| labels(&request(&mut fixture, "textDocument/completion", &document, offset_of(text, needle, true)));

    assert!(!at("composed {{ er").contains(&"err".to_string()));
    assert!(at("{{ tim").contains(&"timing".to_string()));
    assert!(at("failed {{ er").contains(&"err".to_string()));
    assert!(at("{{ err.ms").contains(&"err.msg".to_string()));
    assert!(!at("Body {{ er").contains(&"err".to_string()));
    fixture.shutdown();
}

const HOST_READS: &str = concat!(
    "---\n",
    "title: Hosts\n",
    "composed:\n",
    "  - when: seq.index > 1\n",
    "    then:\n",
    "      - info: \"{{ seq.index }} {{ loop.count }} {{ group.failed }}\"\n",
    "---\n\n",
    "Body {{ seq.index }} {{ loop.count }} {{ group.failed }}\n",
);

fn host_findings(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic["code"].as_str(),
                Some("dm.expression.undeclared_property" | "dm.expression.unavailable_global")
            )
        })
        .collect()
}

/// A registered host's global is execution-dependent, so it is neither
/// undeclared nor unavailable, inside a stack or not. Without the
/// registration the body reads stay undeclared document properties, even
/// with a catalog declaring `seq` in the schema root.
#[test]
fn registered_host_globals_are_never_undeclared_or_unavailable() {
    let workspace = LspWorkspace::new();
    crate::claudine_host::install(workspace.path(), &crate::claudine_host::bundle());
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(workspace.path()));
    // Claudine names `composed` `start`.
    let text = crate::claudine_host::marked(&HOST_READS.replace("composed:", "start:"));
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "hosts.md", &text);
    assert!(host_findings(&diagnostics).is_empty(), "{diagnostics:#?}");
    fixture.shutdown();

    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, &[("globals.yaml", crate::registered_host_views::ROOT_CATALOG)]);
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "hosts.md", HOST_READS);
    let body = line_containing(HOST_READS, "Body");
    let body_reports: Vec<&str> = host_findings(&diagnostics)
        .into_iter()
        .filter(|diagnostic| line_of(diagnostic) == body)
        .map(|diagnostic| diagnostic["message"].as_str().unwrap())
        .collect();
    for root in ["seq", "loop", "group"] {
        assert!(body_reports.iter().any(|message| message.starts_with(&format!("`{root}`"))), "{root}: {body_reports:#?}");
    }
    fixture.shutdown();
}
