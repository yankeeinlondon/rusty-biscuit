//! A refused registration member is named by its display label: a trigger
//! or types-only library that declares a valid `name` is shown as
//! `name (bundle path)` beside its manifest field, in the installed
//! warning and in the embedded loader's refusal, and an importer named in
//! a library's refusal is shown the same way. A member without a valid
//! `name` (none, an invalid one, or a kind that has none) keeps its path
//! alone, and an invalid `name` is still the problem reported.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::config::registration::bundle::{SnapshotRefusal, VerifiedSnapshot};
use darkmatter::config::registration::{
    HOST_IDENTIFY_TOO_BROAD_CODE, HOST_REGISTRATION_CODE, RegistrationDiagnostic, load_embedded, md_owned_names,
};

const GADGET_LABEL: &str = "`files[1]` (`gadget-schema (schemas/gadget.yaml)`)";

/// The named gadget trigger with one edit, as (case, text, expected code).
fn named_gadget_refusals() -> Vec<(&'static str, String, &'static str)> {
    let named = format!("name: gadget-schema\n{GADGET_TRIGGER}");
    vec![
        ("match typo", named.replacen("string(required)", "string(requird)", 1), HOST_REGISTRATION_CODE),
        (
            "identification",
            named.replacen("gadget: string(required)", "title: string(required)", 1),
            HOST_IDENTIFY_TOO_BROAD_CODE,
        ),
        ("payload", named.replacen("gadget: gadget@./library.yaml", "gadget: \"Missing(required)\"", 1), HOST_REGISTRATION_CODE),
        (
            "file-backed example",
            format!("{named}    review-entry: \"string(example(./library.yaml))\"\n"),
            HOST_REGISTRATION_CODE,
        ),
    ]
}

/// The one diagnostic of a load that published nothing.
fn refused(fixture: &Fixture) -> RegistrationDiagnostic {
    let loaded = fixture.load();
    assert!(loaded.profiles.is_empty(), "nothing is published");
    assert_eq!(loaded.diagnostics.len(), 1, "{:#?}", loaded.diagnostics);
    loaded.diagnostics[0].clone()
}

fn embedded_refusals(bundle: &Bundle) -> Vec<SnapshotRefusal> {
    let members: BTreeMap<String, Arc<[u8]>> =
        bundle.members.iter().map(|(path, text)| (path.clone(), Arc::from(text.as_bytes()))).collect();
    let snapshot = VerifiedSnapshot::from_embedded("acme", MANIFEST_NAME, bundle.manifest().as_bytes(), &members).expect("verifies");
    load_embedded(snapshot, md_owned_names(), &|_| None).expect_err("refused")
}

#[test]
fn a_named_trigger_refusal_shows_its_label_beside_its_field() {
    for (case, text, code) in named_gadget_refusals() {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", &text);
        let diagnostic = refused(&fixture);
        assert_eq!(diagnostic.code, code, "{case}: {diagnostic:#?}");
        assert_eq!(diagnostic.member.as_deref(), Some("files[1]"), "{case}: the field is kept");
        assert!(diagnostic.message.contains(&format!("is not loaded at {GADGET_LABEL}: ")), "{case}: {}", diagnostic.message);

        let refusals = embedded_refusals(&Bundle::acme().with_member("schemas/gadget.yaml", &text));
        assert_eq!(refusals.len(), 1, "{case}: {refusals:#?}");
        assert_eq!(refusals[0].member.as_deref(), Some("files[1]"), "{case}");
        assert_eq!(refusals[0].label.as_deref(), Some("gadget-schema (schemas/gadget.yaml)"), "{case}: {refusals:#?}");
    }
}

/// The controls: an unnamed trigger keeps its field alone, and an invalid
/// `name` is the problem reported, never shown as a label.
#[test]
fn an_unnamed_or_invalidly_named_trigger_keeps_its_field_alone() {
    for (case, text, needle) in [
        ("unnamed", GADGET_TRIGGER.replacen("string(required)", "string(requird)", 1), "requird"),
        ("invalid name", format!("name: Bad_Name\n{GADGET_TRIGGER}"), "Bad_Name"),
    ] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", &text);
        let diagnostic = refused(&fixture);
        assert!(diagnostic.message.contains("is not loaded at `files[1]`: "), "{case}: {}", diagnostic.message);
        assert!(diagnostic.message.contains(needle), "{case}: the problem is reported: {}", diagnostic.message);
        assert!(!diagnostic.message.contains("(schemas/gadget.yaml)"), "{case}: no label: {}", diagnostic.message);

        let refusals = embedded_refusals(&Bundle::acme().with_member("schemas/gadget.yaml", &text));
        assert!(refusals.iter().all(|refusal| refusal.label.is_none()), "{case}: {refusals:#?}");
    }
}

/// A named library with an unlisted import, used by the named widget
/// trigger and the unnamed gadget trigger: every refusal is the library's,
/// under its label, and each importer is named by its own label, or its
/// path alone.
#[test]
fn a_named_library_refusal_shows_its_label_and_each_importers() {
    let library = format!(
        "name: parts-list\n{}",
        LIBRARY.replacen(
            "        size: \"number -> The widget's size.\"\n",
            "        size: \"number -> The widget's size.\"\n        part: Part@./extra.yaml\n",
            1
        )
    );
    let bundle = Bundle::acme()
        .with_member("schemas/library.yaml", &library)
        .with_member("schemas/widget.yaml", &format!("name: widget-schema\n{WIDGET_TRIGGER}"))
        .with_member("schemas/gadget.yaml", &format!("{GADGET_TRIGGER}    widget: widget@./library.yaml\n"));
    let fixture = Fixture::new();
    fixture.install_bundle("host", &bundle);
    let loaded = fixture.load();
    let all = &loaded.diagnostics;
    assert!(!all.is_empty() && loaded.profiles.is_empty(), "{all:#?}");
    for diagnostic in all {
        assert_eq!(diagnostic.file, fixture.path("host/schemas/library.yaml"), "{diagnostic:#?}");
        assert!(diagnostic.message.contains("is not loaded at `files[2]` (`parts-list (schemas/library.yaml)`): "), "{}", diagnostic.message);
    }
    for importer in ["imported by `files[0]` (`widget-schema (schemas/widget.yaml)`)", "imported by `files[1]` (`schemas/gadget.yaml`)"] {
        assert!(all.iter().any(|diagnostic| diagnostic.message.contains(importer)), "{importer}: {all:#?}");
    }

    let refusals = embedded_refusals(&bundle);
    assert!(refusals.iter().all(|refusal| refusal.label.as_deref() == Some("parts-list (schemas/library.yaml)")), "{refusals:#?}");
    for importer in ["imported by `files[0]` (`widget-schema (schemas/widget.yaml)`)", "imported by `files[1]` (`schemas/gadget.yaml`)"] {
        assert!(refusals.iter().any(|refusal| refusal.message.contains(importer)), "{importer}: {refusals:#?}");
    }
}

/// The controls: a namespace member has no display name, and a refusal
/// before the bundle verifies names the manifest entry alone.
#[test]
fn namespace_and_unverified_refusals_carry_no_label() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/run.yaml", &RUN_NAMESPACE.replacen("kind: namespace-schema", "kind: namespace-schema\nname: run-schema", 1));
    let diagnostic = refused(&fixture);
    assert!(diagnostic.message.contains("is not loaded at `files[3]`: "), "{}", diagnostic.message);

    let fixture = Fixture::new();
    fixture.corrupt_member("host", "schemas/gadget.yaml", &format!("name: gadget-schema\n{GADGET_TRIGGER}    extra: string\n"));
    let diagnostic = refused(&fixture);
    assert!(!diagnostic.message.contains("gadget-schema"), "{}", diagnostic.message);
}
