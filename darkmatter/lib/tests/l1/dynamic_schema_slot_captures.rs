//! A schema slot calling a captured-context function (`ipv4`, `ipv6`,
//! `has_agentic_cli`, `package`, `package_area`) resolves wherever its
//! schema is written and whoever prepares it: the request supplies the
//! capture the slot's own expression needs, never only the captures the
//! document's text happens to name.
//!
//! Each row is one function at one site. A site is a public preparation
//! path: composition of a document whose schema is inline, referenced,
//! imported, a selected trigger's payload, or a `load_data` contract a slot
//! reaches; and `DarkmatterSchemas::effective_for` with a validating
//! command's session (`SlotSession::for_context`), full or reference-check;
//! and a reference validation with a reference-check request session.
//! The expressions are chosen so the default is correctly typed on any host
//! (an address list's length, a boolean, a package name that is `""` outside
//! a repository). Every case is a temp folder with an empty home and no
//! repository, except the trigger case (compose discovers triggers only
//! inside one). The clock and command-probe rows are controls: the clock is
//! captured lazily and the probes read the context's own `PATH`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, ComposeSource, DataRuntime, RequestSnapshot};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::{SessionMode, SlotSession};
use serde_json::Value;

use crate::dynamic_slots_fixtures::git;

/// A temp folder and an empty home.
struct Case {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: tempfile::TempDir,
}

impl Case {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        Self { _temp: temp, root, home: tempfile::tempdir().expect("temp home") }
    }

    fn path(&self, name: &str) -> PathBuf {
        name.split('/').fold(self.root.clone(), |path, part| path.join(part))
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(&path, text).expect("write");
        path
    }

    /// Persists `allow dir <root>`, so a contract slot's data and contract
    /// reads are granted.
    fn grant_folder(&self) {
        let store = self.home.path().join(".config/darkmatter/schema-read-policy");
        std::fs::create_dir_all(store.parent().expect("store folder")).expect("store folder");
        std::fs::write(&store, format!("allow dir {}\n", biscuit_file::to_portable_string(&self.root))).expect("store");
    }

    fn request(&self, document: &Path) -> ComposeRequest {
        let snapshot =
            RequestSnapshot::new(&self.root).with_home(Some(self.home.path().to_path_buf())).with_env(HashMap::new());
        let options = ComposeOptions::new().with_source_file(document).with_trigger_schemas(true);
        ComposeRequest::prepare(options, &snapshot).expect("request")
    }

    /// A validating command's context: the folder, no repository, the home.
    fn context(&self) -> FileResolutionContext {
        FileResolutionContext::new(&self.root).with_env(HashMap::new()).with_home_dir(self.home.path())
    }

    /// Composes `document`, which must succeed: composition validates
    /// against the resolved schema without writing defaults, so success is
    /// the slot's resolution.
    fn compose(&self, document: &Path) {
        match Markdown::try_from(document).expect("document").compose_with_warnings(&self.request(document)) {
            Ok(_) => {}
            Err(failure) => {
                let codes: Vec<String> = error_diagnostics(&failure.error).into_iter().map(|record| record.code).collect();
                panic!("composition failed with {codes:?}: {}", failure.error)
            }
        }
    }

    /// Checks `document`'s references as `md validate refs` does: its
    /// request with a reference-check slot session, which must succeed.
    fn validate_references(&self, document: &Path) {
        let request = self.request(document);
        let session = Arc::new(SlotSession::for_request(&request).with_mode(SessionMode::ReferenceCheck));
        let checked = request.map_options(|options| options.with_slot_session(session));
        Markdown::try_from(document)
            .expect("document")
            .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&checked)))
            .unwrap_or_else(|error| {
                let codes: Vec<String> = error_diagnostics(&error).into_iter().map(|record| record.code).collect();
                panic!("the reference check failed with {codes:?}: {error}")
            });
    }

    /// Prepares `document`'s effective schema with a validating command's
    /// session in `mode`, returning the default its slot supplied.
    fn prepared_default(&self, document: &Path, mode: SessionMode) -> Value {
        let context = self.context();
        let session = SlotSession::for_context(DataRuntime::new(), &context).with_mode(mode);
        let markdown =
            Markdown::try_from(document).expect("document").with_source(ComposeSource::File(document.to_path_buf()));
        let effective = DarkmatterSchemas::new(context)
            .with_slot_session(Arc::new(session))
            .effective_for(&markdown)
            .unwrap_or_else(|error| panic!("the schema resolves: {error}"))
            .expect("a schema applies");
        first_default(&effective.json_schema).unwrap_or_else(|| panic!("a default: {}", effective.json_schema))
    }
}

/// The first `default` anywhere in `schema`.
fn first_default(schema: &Value) -> Option<Value> {
    match schema {
        Value::Object(map) => map.get("default").cloned().or_else(|| map.values().find_map(first_default)),
        Value::Array(items) => items.iter().find_map(first_default),
        _ => None,
    }
}

/// `type` holds a `default` from `expression`.
fn declaration(type_name: &str, expression: &str) -> String {
    format!("{type_name}(default({{{{ {expression} }}}}))")
}

fn inline(case: &Case, declaration: &str) -> PathBuf {
    case.write("doc.md", &format!("---\n$schema:\n  value: \"{declaration}\"\n---\nBody.\n"))
}

fn referenced(case: &Case, declaration: &str) -> PathBuf {
    case.write("schema.yaml", &format!("kind: schema\n$schema:\n  value: \"{declaration}\"\n"));
    case.write("doc.md", "---\n$schema: ./schema.yaml\n---\nBody.\n")
}

fn imported(case: &Case, declaration: &str) -> PathBuf {
    case.write("types.yaml", &format!("kind: schema\ntypes:\n  Value: \"{declaration}\"\n"));
    case.write("doc.md", "---\n$schema:\n  value: Value@./types.yaml\n---\nBody.\n")
}

/// Compose discovers triggers only for a request inside a repository.
fn trigger(case: &Case, declaration: &str) -> PathBuf {
    git(&case.root, &["init", "-q", "-b", "main"]);
    case.write(
        "schemas/value.trigger.yaml",
        &format!("kind: schema-trigger\nmatch:\n  topic: literal(captures; required)\n$schema:\n  value: \"{declaration}\"\n"),
    );
    case.write("doc.md", "---\ntopic: captures\n---\nBody.\n")
}

/// An outer enum whose slot loads an empty list under an imported
/// object-array contract holding the declaration: the contract's slots are
/// first reached inside the outer slot.
fn contract(case: &Case, declaration: &str) -> PathBuf {
    case.grant_folder();
    case.write("rows.yaml", "[]\n");
    case.write("contract.yaml", &format!("kind: schema\ntypes:\n  Row:\n    value: \"{declaration}\"\n"));
    case.write(
        "doc.md",
        "---\n$schema:\n  outer: \"enum({{ length(load_data('./rows.yaml', 'Row[]@./contract.yaml')) == 0 ? ['empty'] : ['rows'] }})\"\n---\nBody.\n",
    )
}

/// Asserts `value` has the JSON type `type_name` declares.
fn assert_typed(type_name: &str, value: &Value) {
    let typed = match type_name {
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "string" => value.is_string(),
        other => panic!("no check for {other}"),
    };
    assert!(typed, "a {type_name} default, got {value}");
}

/// One `#[test]` per function and site, so each row fails on its own.
macro_rules! rows {
    ($($function:ident: $type_name:literal, $expression:literal;)*) => {
        $(
            mod $function {
                use super::*;

                fn declared() -> String {
                    declaration($type_name, $expression)
                }

                #[test]
                fn inline_composition() {
                    let case = Case::new();
                    let document = inline(&case, &declared());
                    case.compose(&document);
                }

                #[test]
                fn referenced_schema_composition() {
                    let case = Case::new();
                    let document = referenced(&case, &declared());
                    case.compose(&document);
                }

                #[test]
                fn imported_type_composition() {
                    let case = Case::new();
                    let document = imported(&case, &declared());
                    case.compose(&document);
                }

                #[test]
                fn selected_trigger_composition() {
                    let case = Case::new();
                    let document = trigger(&case, &declared());
                    case.compose(&document);
                }

                #[test]
                fn nested_contract_composition() {
                    let case = Case::new();
                    let document = contract(&case, &declared());
                    case.compose(&document);
                }

                #[test]
                fn validating_session() {
                    let case = Case::new();
                    let document = inline(&case, &declared());
                    assert_typed($type_name, &case.prepared_default(&document, SessionMode::Full));
                }

                #[test]
                fn validating_session_for_a_referenced_schema() {
                    let case = Case::new();
                    let document = referenced(&case, &declared());
                    assert_typed($type_name, &case.prepared_default(&document, SessionMode::Full));
                }

                #[test]
                fn reference_validation_of_a_referenced_schema() {
                    let case = Case::new();
                    let document = referenced(&case, &declared());
                    case.validate_references(&document);
                }

                #[test]
                fn reference_check_session() {
                    let case = Case::new();
                    let document = inline(&case, &declared());
                    assert_typed($type_name, &case.prepared_default(&document, SessionMode::ReferenceCheck));
                }
            }
        )*
    };
}

rows! {
    ipv4: "number", "length(ipv4())";
    ipv6: "number", "length(ipv6())";
    has_agentic_cli: "boolean", "has_agentic_cli('kimi')";
    package: "string", "package('./child')";
    package_area: "string", "package_area('./child')";
    // Controls: the clock is captured lazily, and the command probes read
    // `PATH` from the slot's file-resolution context.
    is_today: "boolean", "is_today('2024-06-16')";
    has_command: "boolean", "has_command('git')";
    has_binary: "boolean", "has_binary('git')";
}


/// The trigger rows are not vacuous: the case's trigger is selected and
/// judges the document.
#[test]
fn the_trigger_case_selects_its_payload() {
    let case = Case::new();
    let document = trigger(&case, "number");
    std::fs::write(&document, "---\ntopic: captures\nvalue: text\n---\nBody.\n").expect("document");
    let result = Markdown::try_from(document.as_path()).expect("document").compose_with_warnings(&case.request(&document));
    assert!(result.is_err(), "the trigger's `value: number` rejects `text`");
}
