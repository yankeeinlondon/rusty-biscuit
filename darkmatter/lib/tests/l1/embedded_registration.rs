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
