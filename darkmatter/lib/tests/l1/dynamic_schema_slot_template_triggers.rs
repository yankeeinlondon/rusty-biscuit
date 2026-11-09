//! Template types in trigger payloads and layering (R20): a payload property
//! whose type a slot supplies is pending until a session resolves it, the
//! trigger still matches, and a property the document declares itself is
//! never evaluated from the trigger. Places that never hold slots (a trigger's
//! `match`, a kind definition, the baseline) refuse a template type as they
//! refuse any slot.
//!
//! Each case writes its triggers under `{root}/schemas` of a temp folder
//! outside any repository (the tree schema root of a context anchored at
//! `root`), so no automatic grant applies and only the stated grants do.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::template_diagnostics;
use darkmatter::markdown::compose::{ComposeSource, DataRuntime};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::SlotSession;
use darkmatter::markdown::schemas::slots::codes::{
    SLOT_NOT_ALLOWED_IN_BASELINE_CODE, SLOT_NOT_ALLOWED_IN_KIND_CODE, slot_prohibition_code,
};
use darkmatter::markdown::schemas::slots::grants::{Grant, GrantDecision, GrantScope, GrantTarget};
use serde_json::Value;
use simplified_schema::SchemaError;

use crate::yaml_kind_registry::{Fixture as KindFixture, ROSTER_DEFINITION, ROSTER_DOCUMENT};

const DENIED: &str = "dm.schema.slot_denied";
const TYPES: &str = "kind: schema\ntypes:\n    Person:\n        name: string(required)\n";
const TRIGGER: &str = "schemas/owner.trigger.yaml";

/// The trigger at [`TRIGGER`] matching `topic: sandbox` with `payload` as
/// its `$schema` entries.
fn trigger(payload: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n  topic: literal(sandbox; required)\n$schema:\n{payload}")
}

/// A canonical temp folder with `schemas/types.yaml` (a `Person` type).
struct Case {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Case {
    fn new(files: &[(&str, &str)]) -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        let case = Self { _temp: temp, root };
        case.write("schemas/types.yaml", TYPES);
        for (name, text) in files {
            case.write(name, text);
        }
        case
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("write");
    }

    /// Schemas with every trigger under `schemas/` discovered, and `session`
    /// when given.
    fn schemas(&self, session: Option<SlotSession>) -> DarkmatterSchemas {
        let context = FileResolutionContext::new(&self.root).with_env(HashMap::new());
        let schemas = DarkmatterSchemas::new(context).with_trigger_discovery().expect("triggers");
        match session {
            Some(session) => schemas.with_slot_session(Arc::new(session)),
            None => schemas,
        }
    }

    /// The document `doc.md` with `frontmatter`, its source set.
    fn document(&self, frontmatter: &str) -> Markdown {
        self.write("doc.md", &format!("---\n{frontmatter}---\n\nBody.\n"));
        let path = self.path("doc.md");
        Markdown::try_from(path.as_path()).expect("document").with_source(ComposeSource::File(path))
    }
}

fn grant(decision: GrantDecision, path: &Path) -> Grant {
    Grant { decision, target: GrantTarget::File(path.to_path_buf()), scope: GrantScope::Once }
}

fn effective(schemas: &DarkmatterSchemas, markdown: &Markdown) -> Result<Value, SchemaError> {
    Ok(Value::clone(&schemas.effective_for(markdown)?.expect("a schema applies").json_schema))
}

fn property_names(schema: &Value) -> Vec<String> {
    let mut names: Vec<String> = schema["properties"].as_object().expect("properties").keys().cloned().collect();
    names.sort();
    names
}

#[test]
fn a_template_type_payload_matches_and_resolves_to_the_static_equivalent() {
    let case = Case::new(&[(TRIGGER, &trigger("    owner: \"{{ 'Person@./types.yaml' }}\"\n"))]);
    let pending = case.schemas(None);
    assert!(pending.discovery_findings().is_empty(), "a template payload is a valid trigger: {:?}", pending.discovery_findings());
    let markdown = case.document("topic: sandbox\n");
    let error = effective(&pending, &markdown).expect_err("pending without a session");
    assert!(matches!(error, SchemaError::UnresolvedSlots(_)), "the trigger matched and its payload is pending: {error:?}");

    let session =
        SlotSession::new(DataRuntime::new()).with_run_grants([grant(GrantDecision::Allow, &case.path("schemas/types.yaml"))]);
    let resolved = effective(&case.schemas(Some(session)), &markdown).expect("resolves");

    let control = Case::new(&[(TRIGGER, &trigger("    owner: Person@./types.yaml\n"))]);
    let expected = effective(&control.schemas(None), &control.document("topic: sandbox\n")).expect("static resolves");
    assert_eq!(resolved["properties"]["owner"], expected["properties"]["owner"]);
    assert!(resolved["properties"]["owner"].to_string().contains(r#""required":["name"]"#), "{resolved}");
}

/// The document's own declaration wins, so the trigger's slot for the same
/// property never runs: a denied read it would make is neither refused nor
/// read. The control proves the slot would be refused were it evaluated.
#[test]
fn a_property_the_document_declares_never_evaluates_the_trigger_slot() {
    let case = Case::new(&[
        (TRIGGER, &trigger("    owner: \"{{ load_data('./owner-type.yaml', '{ type: string }').type }}\"\n")),
        ("schemas/owner-type.yaml", "type: Person@./types.yaml\n"),
    ]);
    let owner_type = case.path("schemas/owner-type.yaml");
    let session = |data: &DataRuntime| {
        SlotSession::new(data.clone()).with_run_grants([grant(GrantDecision::Deny, &owner_type)])
    };

    let data = DataRuntime::new();
    let markdown = case.document("$schema:\n    owner: string\ntopic: sandbox\nowner: Ann\n");
    let resolved = effective(&case.schemas(Some(session(&data))), &markdown).expect("the document's owner wins");
    let owner = resolved["properties"]["owner"].to_string();
    assert!(owner.contains(r#""type":"string""#) && !owner.contains("properties"), "{resolved}");
    let entry = biscuit_file::to_portable_string(&owner_type);
    assert!(!data.read_set().contains(&entry), "{:?}", data.read_set());

    let markdown = case.document("topic: sandbox\n");
    let error = effective(&case.schemas(Some(session(&DataRuntime::new()))), &markdown).expect_err("control: refused");
    let codes: Vec<String> = template_diagnostics(&error, &case.path("doc.md"))
        .unwrap_or_else(|| panic!("not a template failure: {error:?}"))
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, vec![DENIED.to_string()], "{error}");
}

/// A returned object definition is the property's type: its keys stay
/// nested under the property and never join the document's properties.
#[test]
fn a_returned_object_definition_stays_nested_under_its_property() {
    let case = Case::new(&[(TRIGGER, &trigger("    owner: \"{{ '{ name: string }' }}\"\n"))]);
    let session = SlotSession::new(DataRuntime::new());
    let resolved = effective(&case.schemas(Some(session)), &case.document("topic: sandbox\n")).expect("resolves");
    assert!(!property_names(&resolved).contains(&"name".to_string()), "{resolved}");
    assert!(property_names(&resolved).contains(&"owner".to_string()), "{resolved}");
    let owner = resolved["properties"]["owner"].to_string();
    assert!(owner.contains(r#""properties":{"name""#), "{resolved}");
}

/// A trigger's `match` decides whether the payload applies, so it never
/// holds a slot, a template-type-shaped one included.
#[test]
fn a_template_type_slot_in_a_match_condition_is_an_invalid_trigger() {
    let case = Case::new(&[(
        TRIGGER,
        "kind: schema-trigger\nmatch:\n  topic: \"{{ 'string' }}\"\n$schema:\n    level: number\n",
    )]);
    let schemas = case.schemas(Some(SlotSession::new(DataRuntime::new())));
    let findings = schemas.discovery_findings();
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].code(), "dm.schema.invalid_trigger");
    assert!(findings[0].message().contains("slot"), "{}", findings[0].message());
}

/// A kind definition never holds a slot: a template type written in it, or
/// reached through a static import, breaks its claim with the kind code.
#[test]
fn a_template_type_in_a_kind_definition_is_refused() {
    let template = "    platform:\n        name: \"{{ 'string' }}\"\n";
    let inline = ROSTER_DEFINITION.replace("    platform:\n        name: string(required)\n", template);
    let imported =
        ROSTER_DEFINITION.replace("    platform:\n        name: string(required)\n", "    platform: platform@./types.yaml\n");
    assert_ne!(inline, ROSTER_DEFINITION);
    assert_ne!(imported, ROSTER_DEFINITION);
    for (case, text) in [("inline", inline), ("import", imported)] {
        let fixture = KindFixture::new();
        crate::yaml_kind_registry::write(&fixture.repo().join("schemas/roster.yaml"), &text);
        crate::yaml_kind_registry::write(
            &fixture.repo().join("schemas/types.yaml"),
            &format!("kind: schema\ntypes:\n{template}"),
        );
        let error = fixture.validate(&fixture.package().join("roster.yaml"), ROSTER_DOCUMENT).expect_err(case);
        assert_eq!(slot_prohibition_code(&error), Some(SLOT_NOT_ALLOWED_IN_KIND_CODE), "{case}: {error}");
    }
}

/// The baseline never holds a slot: a template type in a baseline file, or
/// reached through its static import, fails with the baseline code.
#[test]
fn a_template_type_in_the_baseline_is_refused() {
    let case = Case::new(&[
        ("plain.yaml", "$schema:\n    owner: Person@./schemas/types.yaml\n"),
        ("direct.yaml", "$schema:\n    owner: \"{{ 'Person@./schemas/types.yaml' }}\"\n"),
        ("schemas/slotted.yaml", "kind: schema\ntypes:\n    Owner: \"{{ 'Person@./types.yaml' }}\"\n"),
        ("imported.yaml", "$schema:\n    owner: Owner@./schemas/slotted.yaml\n"),
    ]);
    let context = || FileResolutionContext::new(&case.root).with_env(HashMap::new());
    DarkmatterSchemas::new(context()).with_baseline_from_file(case.path("plain.yaml")).expect("the control loads");
    for name in ["direct.yaml", "imported.yaml"] {
        let error = DarkmatterSchemas::new(context())
            .with_baseline_from_file(case.path(name))
            .err()
            .unwrap_or_else(|| panic!("{name} is not a baseline"));
        assert_eq!(slot_prohibition_code(&error), Some(SLOT_NOT_ALLOWED_IN_BASELINE_CODE), "{name}: {error}");
    }
}
