//! `darkmatter::config::registration` over a bundle a host compiles into its
//! binary: `VerifiedSnapshot::from_embedded` verifies the manifest and member
//! bytes from memory under the virtual root `<name>`, and `load_embedded`
//! loads the snapshot as a registered host profile whose triggers name their
//! bundle path, never a host file. The bundle is the valid `acme` bundle of
//! the shared host fixtures, held in memory.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::host_fixtures::*;
use crate::host_registration_locations::declaration_of;
use darkmatter::config::registration::bundle::{VerifiedSnapshot, load_snapshot};
use darkmatter::config::registration::{HOST_REGISTRATION_CODE, load_embedded, md_owned_names};
use simplified_schema::triggers::TriggerSource;

/// The manifest bytes and the member bytes by bundle path of `bundle`.
fn in_memory(bundle: &Bundle) -> (Vec<u8>, BTreeMap<String, Arc<[u8]>>) {
    let members = bundle.members.iter().map(|(path, text)| (path.clone(), Arc::from(text.as_bytes()))).collect();
    (bundle.manifest().into_bytes(), members)
}

fn embedded(bundle: &Bundle) -> Result<VerifiedSnapshot, String> {
    let (manifest, members) = in_memory(bundle);
    VerifiedSnapshot::from_embedded("acme", MANIFEST_NAME, &manifest, &members).map_err(|refusal| {
        assert_eq!(refusal.code, HOST_REGISTRATION_CODE, "{refusal:?}");
        refusal.message
    })
}

#[test]
fn an_embedded_bundle_verifies_from_memory_under_its_virtual_root() {
    let snapshot = embedded(&Bundle::acme()).expect("the acme bundle verifies");
    assert_eq!(snapshot.embedded_bundle(), Some("acme"));
    assert_eq!(snapshot.host(), "acme");
    assert_eq!(snapshot.bundle_root(), Path::new("<acme>"));
    assert_eq!(snapshot.manifest(), Path::new("<acme>").join(MANIFEST_NAME));
    let files: Vec<&str> = snapshot.files().iter().map(|member| member.path.as_str()).collect();
    assert_eq!(files, Bundle::acme().files.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn an_embedded_member_that_does_not_match_its_digest_is_refused() {
    let bundle = Bundle::acme();
    let (manifest, mut members) = in_memory(&bundle);
    members.insert("schemas/library.yaml".to_string(), Arc::from(&b"kind: schema\ntypes:\n    other: string\n"[..]));
    let refusal = VerifiedSnapshot::from_embedded("acme", MANIFEST_NAME, &manifest, &members).expect_err("a digest mismatch");
    assert!(refusal.message.contains("schemas/library.yaml") && refusal.message.contains("does not match its digest"), "{refusal:?}");
}

#[test]
fn an_embedded_member_missing_from_the_bytes_is_refused() {
    let bundle = Bundle::acme();
    let (manifest, mut members) = in_memory(&bundle);
    members.remove("schemas/gadget.yaml");
    let refusal = VerifiedSnapshot::from_embedded("acme", MANIFEST_NAME, &manifest, &members).expect_err("a missing member");
    assert!(refusal.message.contains("schemas/gadget.yaml") && refusal.message.contains("missing from the embedded bundle"), "{refusal:?}");
}

#[test]
fn an_embedded_bundle_name_must_be_one_plain_segment() {
    let (manifest, members) = in_memory(&Bundle::acme());
    for name in ["", "a/b", "..", "<acme>"] {
        let refusal = VerifiedSnapshot::from_embedded(name, MANIFEST_NAME, &manifest, &members)
            .expect_err("a name that is not one plain segment");
        assert!(refusal.message.contains("is not one plain path segment"), "`{name}`: {refusal:?}");
    }
}

#[test]
fn an_embedded_member_outside_the_bundle_is_refused_without_a_read() {
    // A member reference that leaves the virtual root never reaches the
    // process file system.
    let bundle = Bundle::acme().with_file("schemas/../../outside.yaml", "kind: schema\ntypes:\n    a: string\n");
    let message = embedded(&bundle).expect_err("a member outside the bundle");
    assert!(message.contains("leaves the bundle"), "{message}");
}

#[test]
fn load_embedded_refuses_a_snapshot_read_from_disk() {
    let fixture = Fixture::new();
    let snapshot = load_snapshot(&fixture.manifest(), &fixture.context()).expect("the installed acme bundle verifies");
    assert_eq!(snapshot.embedded_bundle(), None);
    let refusals = load_embedded(snapshot, md_owned_names(), &|_| None).expect_err("an installed snapshot");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(refusals[0].message.contains("read from disk"), "{refusals:?}");
}

#[test]
fn embedded_triggers_name_their_bundle_path_and_no_host_file() {
    let snapshot = embedded(&Bundle::acme()).expect("the acme bundle verifies");
    let profile = load_embedded(snapshot, md_owned_names(), &|_| None).expect("the acme bundle loads");
    let paths: BTreeSet<&str> = profile.triggers().iter().map(|trigger| trigger.bundle_path.as_str()).collect();
    assert_eq!(paths, BTreeSet::from(["schemas/widget.yaml", "schemas/gadget.yaml"]));
    for trigger in profile.triggers() {
        assert!(trigger.is_embedded(), "{}", trigger.bundle_path);
        assert_eq!(
            trigger.trigger.source,
            TriggerSource::Embedded { bundle: "acme".to_string(), name: trigger.bundle_path.clone() }
        );
        assert_eq!(trigger.trigger.source.to_string(), format!("embedded:acme/{}", trigger.bundle_path));
        assert_eq!(trigger.location, PathBuf::from("<acme>").join(&trigger.bundle_path));
        assert!(trigger.payload.dependencies.is_empty(), "{}: {:?}", trigger.bundle_path, trigger.payload.dependencies);
        assert!(trigger.payload.searched.is_empty(), "{}: {:?}", trigger.bundle_path, trigger.payload.searched);
    }
}

#[test]
fn a_prepared_payload_is_served_instead_of_resolved() {
    let reference = load_embedded(embedded(&Bundle::acme()).unwrap(), md_owned_names(), &|_| None).expect("loads");
    let widget = reference.triggers().iter().find(|trigger| trigger.bundle_path == "schemas/widget.yaml").expect("widget");
    let mut marked = widget.payload.clone();
    marked.json_schema = Arc::new(serde_json::json!({ "type": "object", "properties": { "served_marker": {} } }));
    marked.shape = None;
    marked.digest = None;
    let served = std::cell::RefCell::new(Vec::new());
    let profile = load_embedded(embedded(&Bundle::acme()).unwrap(), md_owned_names(), &|path| {
        served.borrow_mut().push(path.to_string());
        (path == "schemas/widget.yaml").then(|| marked.clone())
    })
    .expect("loads");
    assert!(served.borrow().contains(&"schemas/widget.yaml".to_string()), "{:?}", served.borrow());
    let widget = profile.triggers().iter().find(|trigger| trigger.bundle_path == "schemas/widget.yaml").expect("widget");
    assert!(widget.payload.json_schema["properties"].get("served_marker").is_some(), "the prepared payload is served");
}

/// An embedded trigger's failing `group` condition is refused at that
/// condition's value in the bundle's virtual file, as an installed one is,
/// never at the payload's property of the same name.
#[test]
fn an_embedded_grouped_condition_failure_is_located_at_its_value() {
    let text = GROUPED_GADGET_TRIGGER.replacen("gadget: string(required)", "gadget: string(requird)", 1);
    let bundle = Bundle::acme().with_member("schemas/gadget.yaml", &text);
    let refusals = load_embedded(embedded(&bundle).expect("verifies"), md_owned_names(), &|_| None).expect_err("refused");
    assert_eq!(refusals.len(), 1, "{refusals:#?}");
    let refusal = &refusals[0];
    let offset = text.find("string(requird)").unwrap();
    let line = text[..offset].matches('\n').count() + 1;
    let column = offset - text[..offset].rfind('\n').map_or(0, |at| at + 1) + 1;
    assert_eq!(
        (refusal.file.as_path(), refusal.line, refusal.column),
        (Path::new("<acme>/schemas/gadget.yaml"), Some(line), Some(column)),
        "{refusal:#?}"
    );
}

/// An embedded trigger that omits `match` is refused, with nothing loaded,
/// at the mapping that lacks it in the bundle's virtual file.
#[test]
fn an_embedded_trigger_without_match_is_located_at_its_mapping() {
    let text = "# The gadget trigger.\n\nkind: schema-trigger\n$schema:\n    gadget: gadget@./library.yaml\n";
    let bundle = Bundle::acme().with_member("schemas/gadget.yaml", text);
    let refusals = load_embedded(embedded(&bundle).expect("verifies"), md_owned_names(), &|_| None).expect_err("refused");
    assert_eq!(refusals.len(), 1, "{refusals:#?}");
    let refusal = &refusals[0];
    assert_eq!(
        (refusal.file.as_path(), refusal.line, refusal.column),
        (Path::new("<acme>/schemas/gadget.yaml"), Some(3), Some(1)),
        "{refusal:#?}"
    );
}

/// An embedded member's unused declaration faces the bundle boundary as an
/// installed one does: an unlisted import or example in a library's or a
/// trigger's `types` refuses the whole bundle at the declaration that
/// writes it, in the member's virtual file, even when the trigger's payload
/// is served prepared.
#[test]
fn an_embedded_unused_unlisted_reference_refuses_the_bundle_at_the_reference() {
    let cases = [
        ("schemas/library.yaml", format!("{LIBRARY}    optional-host-settings: Options@./options.yaml\n")),
        ("schemas/gadget.yaml", format!("{GADGET_TRIGGER}types:\n    optional-host-settings: Options@./options.yaml\n")),
        ("schemas/gadget.yaml", format!("{GADGET_TRIGGER}types:\n    review-note: string(example(./options.yaml))\n")),
    ];
    let reference = load_embedded(embedded(&Bundle::acme()).unwrap(), md_owned_names(), &|_| None).expect("the unedited bundle loads");
    let gadget = reference.triggers().iter().find(|trigger| trigger.bundle_path == "schemas/gadget.yaml").expect("gadget").payload.clone();
    for (path, text) in &cases {
        let bundle = Bundle::acme().with_member(path, text).with_member("schemas/options.yaml", "kind: schema\ntypes:\n    Options: string\n");
        let refusals = load_embedded(embedded(&bundle).expect("verifies"), md_owned_names(), &|served| {
            (served == "schemas/gadget.yaml").then(|| gadget.clone())
        })
        .expect_err("refused");
        assert_eq!(refusals.len(), 1, "{path}: {refusals:#?}");
        let refusal = &refusals[0];
        let (line, column) = declaration_of(text, "./options.yaml");
        assert_eq!(
            (refusal.file.as_path(), refusal.line, refusal.column),
            (Path::new("<acme>").join(path).as_path(), Some(line), Some(column)),
            "{path}: {refusal:#?}"
        );
    }
}

/// An embedded bundle locates a failure inside a union arm, and an unlisted
/// reference a shared library type makes, as an installed one does: in the
/// arm, and in the library's virtual file for the importer too.
#[test]
fn an_embedded_failure_inside_an_arm_or_a_shared_type_is_located_where_it_is_written() {
    let union_trigger = "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    - $extends: widget@./library.yaml\n      gadget: gadget@./library.yaml\n    - $extends: widget@./library.yaml\n      gadget: \"Missing(required)\"\n";
    let library_arm = LIBRARY.replacen(
        "    gadget: \"string -> The gadget's name.\"\n",
        "    gadget:\n        - \"string -> The gadget's name.\"\n        - \"Missing[] -> Or its parts.\"\n",
        1,
    );
    let library_import = LIBRARY.replacen("    gadget: \"string -> The gadget's name.\"\n", "    gadget: Options@./options.yaml\n", 1);
    let cases = [
        ("trigger arm", "schemas/gadget.yaml", union_trigger.to_string(), "\"Missing("),
        ("library arm", "schemas/library.yaml", library_arm, "\"Missing[]"),
        ("library import", "schemas/library.yaml", library_import, "Options@./options.yaml"),
    ];
    for (case, path, text, needle) in cases {
        let bundle = Bundle::acme().with_member(path, &text).with_member("schemas/options.yaml", "kind: schema\ntypes:\n    Options: string\n");
        let refusals = load_embedded(embedded(&bundle).expect("verifies"), md_owned_names(), &|_| None).expect_err("refused");
        let offset = text.find(needle).unwrap();
        let line = text[..offset].matches('\n').count() + 1;
        let column = offset - text[..offset].rfind('\n').map_or(0, |at| at + 1) + 1;
        for refusal in &refusals {
            assert_eq!(
                (refusal.file.as_path(), refusal.line, refusal.column),
                (Path::new("<acme>").join(path).as_path(), Some(line), Some(column)),
                "{case}: {refusal:#?}"
            );
        }
        if path == "schemas/library.yaml" {
            assert!(
                refusals.iter().any(|refusal| refusal.message.contains("imported by `files[1]`")),
                "{case}: the importer's refusal is in the library: {refusals:#?}"
            );
        }
    }
}

/// An embedded bundle locates an unlisted import or example that follows a
/// sibling declaration whose description names the same file, as an
/// installed one does: at the declaration that writes it, in every member
/// kind, used or not, and for a shared library's importers too.
#[test]
fn an_embedded_reference_after_a_sibling_description_is_located_at_the_reference() {
    const PROSE: &str = "    review-setting: 'string -> Read options from ./options.yaml.'\n";
    const PART: &str = "kind: yaml-schema\n$schema:\n    kind: literal(part; required)\n";
    for (entry, outer) in [("    review-entry: Options@./options.yaml\n", "@./options.yaml"), ("    review-entry: string(example(./options.yaml))\n", "example(./options.yaml")] {
        let declarations = format!("{PROSE}{entry}");
        let cases = [
            ("unused library", "schemas/library.yaml", format!("{LIBRARY}{declarations}")),
            ("unused trigger", "schemas/gadget.yaml", format!("{GADGET_TRIGGER}types:\n{declarations}")),
            ("unused yaml kind", "schemas/part.yaml", format!("{PART}types:\n{declarations}")),
            ("unused namespace", "schemas/run.yaml", format!("{RUN_NAMESPACE}types:\n{declarations}")),
            ("used trigger", "schemas/gadget.yaml", GADGET_TRIGGER.replacen("$schema:\n", &format!("$schema:\n{declarations}"), 1)),
            ("used yaml kind", "schemas/part.yaml", format!("{PART}{declarations}")),
            ("used namespace", "schemas/run.yaml", format!("{RUN_NAMESPACE}{declarations}")),
            (
                "used library",
                "schemas/library.yaml",
                LIBRARY.replacen("    gadget: \"string -> The gadget's name.\"\n", &declarations.replace("review-entry", "gadget"), 1),
            ),
        ];
        for (case, path, text) in cases {
            let mut bundle = Bundle::acme().with_file(path, &text).with_member("schemas/options.yaml", "kind: schema\ntypes:\n    Options: string\n");
            if path == "schemas/run.yaml" {
                bundle = bundle.with_member("schemas/globals.lifecycle.json", &lifecycle_artifact_for("acme", &[(path, &text)]));
            }
            let refusals = load_embedded(embedded(&bundle).expect("verifies"), md_owned_names(), &|_| None).expect_err("refused");
            let (line, column) = declaration_of(&text, outer);
            if !case.ends_with("library") {
                assert_eq!(refusals.len(), 1, "{case}: {refusals:#?}");
            }
            for refusal in &refusals {
                assert_eq!(
                    (refusal.file.as_path(), refusal.line, refusal.column),
                    (Path::new("<acme>").join(path).as_path(), Some(line), Some(column)),
                    "{case} {outer}: {refusal:#?}"
                );
            }
            if case == "used library" {
                assert!(refusals.iter().any(|refusal| refusal.message.contains("imported by `files[1]`")), "{case}: {refusals:#?}");
            }
        }
    }
}

/// An embedded `kind: yaml-schema` definition's optional `types` table is
/// judged as its member envelope judges it, as installed: absent, blank or
/// `null`, and populated load a profile with the kind and both triggers;
/// numeric and empty refuse the profile at `types`.
#[test]
fn an_embedded_definition_s_optional_types_table_keeps_the_envelope_s_rules() {
    const ROSTER: &str = "kind: yaml-schema\n$schema:\n    kind: literal(roster; required)\n    platforms: string[](required)\n";
    for (types, refused) in [
        ("", false),
        ("types:\n", false),
        ("types: null\n", false),
        ("types:\n    platform: string\n", false),
        ("types: 7\n", true),
        ("types: {}\n", true),
    ] {
        let text = format!("{ROSTER}{types}");
        let snapshot = embedded(&Bundle::acme().with_file("schemas/roster.yaml", &text)).expect("the bundle verifies");
        let loaded = load_embedded(snapshot, md_owned_names(), &|_| None);
        if refused {
            let refusals = loaded.expect_err(types);
            let [refusal] = refusals.as_slice() else { panic!("{types:?}: {refusals:#?}") };
            assert_eq!(refusal.file, Path::new("<acme>/schemas/roster.yaml"), "{types:?}");
            let line = text.lines().position(|line| line.starts_with("types:")).unwrap() + 1;
            assert_eq!(refusal.line.zip(refusal.column), Some((line, "types: ".len() + 1)), "{types:?}: {refusal:#?}");
        } else {
            let profile = loaded.unwrap_or_else(|refusals| panic!("{types:?}: {refusals:#?}"));
            let kinds: Vec<&str> = profile.yaml_kinds().iter().map(|kind| kind.source().path.as_str()).collect();
            assert_eq!(kinds, ["schemas/roster.yaml"], "{types:?}");
            assert_eq!(profile.triggers().len(), 2, "{types:?}: both Markdown triggers");
        }
    }
}

/// An embedded `kind: yaml-schema` definition whose `kind` claim goes
/// through a named type too broad to claim a finite set is refused, with
/// nothing loaded, at the `kind` declaration's value in its virtual file,
/// as an installed one is.
#[test]
fn an_embedded_named_kind_claim_that_claims_nothing_is_located_at_its_declaration() {
    let text = "kind: yaml-schema\n$schema:\n    kind: kind-token(required)\n    name: string\ntypes:\n    kind-token: string\n";
    let snapshot = embedded(&Bundle::acme().with_file("schemas/part.yaml", text)).expect("the bundle verifies");
    let refusals = load_embedded(snapshot, md_owned_names(), &|_| None).expect_err("refused");
    let [refusal] = refusals.as_slice() else { panic!("{refusals:#?}") };
    assert_eq!(
        (refusal.file.as_path(), refusal.line, refusal.column),
        (Path::new("<acme>/schemas/part.yaml"), Some(3), Some(11)),
        "{refusal:#?}"
    );
}

/// An embedded bundle locates an example beside a working import of its
/// file, and one inherited from a shared library declaration, as an
/// installed one does: at the declaration that writes the example, in the
/// file that writes it, naming each importer of a library's.
#[test]
fn an_embedded_example_is_located_at_the_declaration_that_writes_it() {
    use crate::host_registration_locations::{BAD_EXAMPLE, example_beside_an_import, last_position_of, republished, shared_example_bundle};
    for (case, path, _, text) in example_beside_an_import() {
        let refusals = load_embedded(embedded(&republished(path, &text)).expect("verifies"), md_owned_names(), &|_| None).expect_err("refused");
        let [refusal] = refusals.as_slice() else { panic!("{case}: {refusals:#?}") };
        let (line, column) = last_position_of(&text, BAD_EXAMPLE);
        assert_eq!(
            (refusal.file.as_path(), refusal.line, refusal.column),
            (Path::new("<acme>").join(path).as_path(), Some(line), Some(column)),
            "{case}: {refusal:#?}"
        );
    }

    let (bundle, library) = shared_example_bundle();
    let refusals = load_embedded(embedded(&bundle).expect("verifies"), md_owned_names(), &|_| None).expect_err("refused");
    let (line, column) = last_position_of(&library, "\"string(example(./options.yaml))\"");
    for refusal in &refusals {
        assert_eq!(
            (refusal.file.as_path(), refusal.line, refusal.column),
            (Path::new("<acme>/schemas/library.yaml"), Some(line), Some(column)),
            "{refusal:#?}"
        );
    }
    for importer in ["imported by `files[0]`", "imported by `files[1]`"] {
        assert!(refusals.iter().any(|refusal| refusal.message.contains(importer)), "{importer}: {refusals:#?}");
    }
}

/// A payload naming another trigger, or a member of another role, as its
/// document schema is refused at the same declaration by the embedded
/// loader as by the installed one: the scalar or arm that names it.
#[test]
fn an_embedded_target_refused_for_its_role_is_located_at_the_declaration_that_names_it() {
    use crate::host_registration_locations::republished;
    let head = "kind: schema-trigger\nmatch:\n    gadget: string(required)\n";
    for (target, reason) in [("./widget.yaml", "resolved to a schema-trigger envelope"), ("./library.yaml", "it declares only `types`")] {
        for text in [format!("{head}$schema: {target}\n"), format!("{head}$schema:\n    - gadget: string\n    - {target}\n")] {
            let bundle = republished("schemas/gadget.yaml", &text);
            let refusals = load_embedded(embedded(&bundle).expect("verifies"), md_owned_names(), &|_| None).expect_err("refused");
            let [refusal] = refusals.as_slice() else { panic!("{target}: {refusals:#?}") };
            let offset = text.find(target).unwrap();
            let line = text[..offset].matches('\n').count() + 1;
            let column = offset - text[..offset].rfind('\n').map_or(0, |at| at + 1) + 1;
            assert_eq!(
                (refusal.file.as_path(), refusal.line, refusal.column),
                (Path::new("<acme>/schemas/gadget.yaml"), Some(line), Some(column)),
                "{target}: {refusal:#?}"
            );
            assert!(refusal.message.contains(reason), "{target}: {}", refusal.message);
        }
    }
}
