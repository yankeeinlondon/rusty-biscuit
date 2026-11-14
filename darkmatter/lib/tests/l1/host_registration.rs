//! `darkmatter::config::registration`: loading `darkmatter-host` manifests
//! into passive host profiles. The manifest cells of the registration
//! input-robustness matrix (one edit to a valid fixture host per case), the
//! all-or-nothing refusals of each member, duplicate entries, same-host
//! agreement, and the `md`-owned names an identifying trigger may not rely on.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::config::load::{ConfigLoadMode, ConfigRevision, ConfigSnapshot, Registration};
use darkmatter::config::registration::{
    HOST_IDENTIFY_TOO_BROAD_CODE, HOST_REGISTRATION_CODE, HostClaim, LoadedRegistrations, RegistrationDiagnostic,
    agreement, load_entries, load_registrations, md_owned_names,
};
use darkmatter::markdown::compose::expression::Severity;

/// The one refusal of a load that published nothing.
fn refused(loaded: &LoadedRegistrations) -> &RegistrationDiagnostic {
    assert!(loaded.profiles.is_empty(), "nothing is published: {:?}", loaded.profiles);
    assert_eq!(loaded.diagnostics.len(), 1, "{:#?}", loaded.diagnostics);
    let diagnostic = &loaded.diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert!(diagnostic.message.contains("entry 0 of `register`"), "names the entry: {}", diagnostic.message);
    diagnostic
}

/// A located `dm.host.registration` refusal in the manifest itself.
fn refused_in_manifest(fixture: &Fixture, loaded: &LoadedRegistrations) -> RegistrationDiagnostic {
    let diagnostic = refused(loaded).clone();
    assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE, "{diagnostic:#?}");
    assert_eq!(diagnostic.file, fixture.manifest());
    assert!(diagnostic.line.is_some() && diagnostic.column.is_some(), "located: {diagnostic:#?}");
    diagnostic
}

fn without_line(manifest: &str, prefix: &str) -> String {
    manifest.lines().filter(|line| !line.starts_with(prefix)).map(|line| format!("{line}\n")).collect()
}

/// `manifest` with the top-level `field` (and its indented continuation
/// lines) replaced by `field: value`.
fn replace_field(manifest: &str, field: &str, value: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in manifest.lines() {
        if line.starts_with(&format!("{field}:")) {
            out.push_str(&format!("{field}: {value}\n"));
            skipping = true;
        } else if skipping && line.starts_with(' ') {
        } else {
            skipping = false;
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

// ── control ─────────────────────────────────────────────────────────────────

#[test]
fn a_valid_host_loads_every_member_into_one_profile() {
    let fixture = Fixture::new();
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let [profile] = loaded.profiles.as_slice() else { panic!("one profile: {:?}", loaded.profiles) };

    assert_eq!((profile.host(), profile.version()), ("acme", "1.2.0"));
    assert_eq!(profile.manifest().path, fixture.manifest());
    assert_eq!(profile.manifest().content_hash, darkmatter::config::migration_receipt::content_hash(MANIFEST.as_bytes()));
    assert_eq!(profile.entries().len(), 1);
    assert_eq!(profile.identify_trigger().path(), fixture.path("host/schemas/identify.yaml"));
    let triggers: Vec<&Path> = profile.triggers().iter().map(|trigger| trigger.path()).collect();
    assert_eq!(triggers, [fixture.path("host/schemas/widget.yaml"), fixture.path("host/schemas/gadget.yaml")]);
    let widget = &profile.triggers()[0].payload.json_schema;
    assert!(widget["properties"]["widget"].is_object(), "the relative import resolved: {widget}");
    assert!(profile.globals().global("run").is_some());
    assert_eq!(profile.globals_path(), fixture.path("host/schemas/globals.yaml"));
    assert_eq!(profile.lifecycle().registry().host(), "acme");
    assert_eq!(profile.lifecycle_path(), fixture.path("host/schemas/globals.lifecycle.json"));
    let [functions] = profile.functions() else { panic!("one function set") };
    assert_eq!(functions.functions.function_names(), ["acme_label"]);
    assert!(functions.functions.incomplete_functions().is_empty());
    assert_eq!(profile.revision().len(), 64);
}

#[test]
fn every_file_read_is_a_dependency_once_with_its_hash() {
    let fixture = Fixture::new();
    let loaded = fixture.load();
    let profile = &loaded.profiles[0];
    let paths: Vec<&Path> = profile.dependencies().iter().map(|dependency| dependency.path.as_path()).collect();
    let library = fixture.path("host/schemas/library.yaml");
    assert_eq!(paths.iter().filter(|path| **path == library).count(), 1, "the shared import is read once: {paths:#?}");
    for member in [
        "darkmatter-host.yaml",
        "schemas/identify.yaml",
        "schemas/widget.yaml",
        "schemas/gadget.yaml",
        "schemas/globals.yaml",
        "schemas/globals.lifecycle.json",
        "schemas/functions.yaml",
        "schemas/functions.facts.json",
    ] {
        let path = fixture.path(&format!("host/{member}"));
        let dependency = profile.dependencies().iter().find(|dependency| dependency.path == path);
        let dependency = dependency.unwrap_or_else(|| panic!("{member} is a dependency: {paths:#?}"));
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(dependency.content_hash, darkmatter::config::migration_receipt::content_hash(&bytes), "{member}");
    }
    let unique: BTreeSet<&Path> = paths.iter().copied().collect();
    assert_eq!(unique.len(), paths.len(), "each file once");
}

#[test]
fn empty_trigger_and_function_lists_are_valid() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| {
        let manifest = replace_field(manifest, "triggers", "[]");
        replace_field(&manifest, "functions", "[]")
    });
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert!(loaded.profiles[0].triggers().is_empty() && loaded.profiles[0].functions().is_empty());
}

#[test]
fn a_registration_from_the_user_configuration_loads_through_the_snapshot() {
    let fixture = Fixture::new();
    fixture.write("home/.darkmatter.yaml", "register:\n  - ../host/darkmatter-host.yaml\n");
    let snapshot = ConfigSnapshot::load(&fixture.context(), None, ConfigLoadMode::Passive).expect("the configuration loads");
    let loaded = load_registrations(&snapshot, &fixture.context(), md_owned_names());
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles[0].entries()[0].declared_in, fixture.path("home/.darkmatter.yaml"));
}

// ── the manifest envelope ───────────────────────────────────────────────────

const FIELDS: [&str; 9] =
    ["kind", "format_version", "host", "version", "identify", "triggers", "globals", "lifecycle", "functions"];

#[test]
fn each_absent_field_refuses_the_registration() {
    for field in FIELDS {
        let fixture = Fixture::new();
        fixture.edit_manifest(|manifest| {
            let manifest = replace_field(manifest, field, "REMOVE");
            without_line(&manifest, &format!("{field}: REMOVE"))
        });
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        assert!(diagnostic.message.contains(&format!("`{field}` is required")), "{field}: {}", diagnostic.message);
        assert_eq!(diagnostic.member, None, "{field}");
    }
}

#[test]
fn each_explicit_null_field_refuses_the_registration() {
    for field in FIELDS {
        let fixture = Fixture::new();
        fixture.edit_manifest(|manifest| replace_field(manifest, field, "null"));
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        let line = MANIFEST.lines().position(|line| line.starts_with(&format!("{field}:"))).unwrap() + 1;
        assert_eq!(diagnostic.line, Some(line), "{field}: {diagnostic:#?}");
        assert!(diagnostic.message.contains(field), "{field}: {}", diagnostic.message);
    }
}

#[test]
fn each_field_of_the_wrong_type_refuses_the_registration() {
    for (field, value) in [
        ("kind", "[darkmatter-host]"),
        ("format_version", "{one: 1}"),
        ("host", "[acme]"),
        ("version", "{v: 1}"),
        ("identify", "7"),
        ("triggers", "./schemas/widget.yaml"),
        ("globals", "[./schemas/globals.yaml]"),
        ("lifecycle", "true"),
        ("functions", "{catalog: ./a.yaml, facts: ./a.facts.json}"),
    ] {
        let fixture = Fixture::new();
        fixture.edit_manifest(|manifest| replace_field(manifest, field, value));
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        let line = MANIFEST.lines().position(|line| line.starts_with(&format!("{field}:"))).unwrap() + 1;
        assert_eq!(diagnostic.line, Some(line), "{field}: {diagnostic:#?}");
    }
}

#[test]
fn a_list_element_of_the_wrong_type_refuses_the_registration() {
    for (field, value, needle) in [
        ("triggers", "[./schemas/widget.yaml, 3]", "/triggers/1"),
        ("triggers", "[3]", "/triggers/0"),
        ("functions", "[3]", "/functions/0"),
        ("functions", "[{catalog: ./schemas/functions.yaml}]", "`facts` is required"),
        ("functions", "[{catalog: ./schemas/functions.yaml, facts: ./schemas/functions.facts.json, extra: 1}]", "`extra`"),
    ] {
        let fixture = Fixture::new();
        fixture.edit_manifest(|manifest| replace_field(manifest, field, value));
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        assert!(diagnostic.message.contains(needle), "{value}: {}", diagnostic.message);
    }
}

#[test]
fn an_empty_string_field_refuses_the_registration() {
    for field in ["host", "version", "identify", "globals", "lifecycle"] {
        let fixture = Fixture::new();
        fixture.edit_manifest(|manifest| replace_field(manifest, field, "\"\""));
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        assert!(diagnostic.message.contains(field), "{field}: {}", diagnostic.message);
    }
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| replace_field(manifest, "triggers", "[\"\"]"));
    refused_in_manifest(&fixture, &fixture.load());
}

#[test]
fn an_unknown_key_refuses_the_closed_envelope_at_the_key() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| format!("{manifest}handlers: ./run.sh\n"));
    let diagnostic = refused_in_manifest(&fixture, &fixture.load());
    assert!(diagnostic.message.contains("`handlers` is not a manifest field"), "{}", diagnostic.message);
    assert_eq!(diagnostic.line, Some(MANIFEST.lines().count() + 1));
}

#[test]
fn only_the_integer_format_version_one_is_accepted() {
    for value in ["2", "\"1\"", "1.0", "0"] {
        let fixture = Fixture::new();
        fixture.edit_manifest(|manifest| replace_field(manifest, "format_version", value));
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        assert_eq!(diagnostic.line, Some(2), "{value}: {diagnostic:#?}");
    }
}

#[test]
fn another_kind_refuses_the_registration() {
    for value in ["schema", "darkmatter-hosts", "\"\""] {
        let fixture = Fixture::new();
        fixture.edit_manifest(|manifest| replace_field(manifest, "kind", value));
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        assert_eq!(diagnostic.line, Some(1), "{value}: {diagnostic:#?}");
    }
}

#[test]
fn a_document_the_yaml_loader_refuses_is_a_located_refusal() {
    let after = MANIFEST.lines().count() + 1;
    for (edit, line) in [
        // The loader locates a duplicate key at its mapping.
        (format!("{MANIFEST}host: other\n"), None),
        (format!("{MANIFEST}---\nkind: darkmatter-host\n"), Some(after)),
        (format!("{MANIFEST}: : ]\n"), Some(after)),
    ] {
        let fixture = Fixture::new();
        fixture.write("host/darkmatter-host.yaml", &edit);
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        assert!(diagnostic.message.contains("not valid YAML"), "{}", diagnostic.message);
        if line.is_some() {
            assert_eq!(diagnostic.line, line, "{edit}: {diagnostic:#?}");
        }
    }
}

#[test]
fn an_empty_or_scalar_manifest_is_refused() {
    for text in ["", "darkmatter-host\n", "[]\n"] {
        let fixture = Fixture::new();
        fixture.write("host/darkmatter-host.yaml", text);
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        assert_eq!(diagnostic.line, Some(1), "{text:?}");
    }
}

#[test]
fn a_missing_manifest_is_reported_at_the_declaring_file() {
    let fixture = Fixture::new();
    let diagnostic = refused(&fixture.load_references(&["../host/absent.yaml"])).clone();
    assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE);
    assert_eq!(diagnostic.file, fixture.path("home/.darkmatter.yaml"));
    assert!(diagnostic.message.contains("names no existing file"), "{}", diagnostic.message);
}

// ── members ─────────────────────────────────────────────────────────────────

#[test]
fn a_remote_member_is_refused_without_fetching() {
    let fixture = Fixture::new();
    // A reserved, unroutable address: a fetch would fail slowly or
    // differently, never with the remote refusal.
    fixture.edit_manifest(|manifest| manifest.replace("./schemas/gadget.yaml", "https://192.0.2.1/gadget.yaml"));
    let diagnostic = refused_in_manifest(&fixture, &fixture.load());
    assert_eq!(diagnostic.member.as_deref(), Some("triggers[1]"));
    assert!(diagnostic.message.contains("is remote"), "{}", diagnostic.message);
    assert_eq!(diagnostic.line, Some(8));
}

#[test]
fn a_missing_member_is_refused_at_its_field() {
    for (member, field, line) in [
        ("schemas/identify.yaml", "identify", 5),
        ("schemas/widget.yaml", "triggers[0]", 7),
        ("schemas/globals.yaml", "globals", 9),
        ("schemas/globals.lifecycle.json", "lifecycle", 10),
        ("schemas/functions.yaml", "functions[0].catalog", 12),
        ("schemas/functions.facts.json", "functions[0].facts", 13),
    ] {
        let fixture = Fixture::new();
        std::fs::remove_file(fixture.path(&format!("host/{member}"))).unwrap();
        let diagnostic = refused_in_manifest(&fixture, &fixture.load());
        assert_eq!(diagnostic.member.as_deref(), Some(field), "{member}");
        assert_eq!(diagnostic.line, Some(line), "{member}: {diagnostic:#?}");
        assert!(diagnostic.message.contains("names no existing file"), "{}", diagnostic.message);
    }
}

#[test]
fn a_missing_import_of_a_trigger_refuses_the_registration() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.path("host/schemas/library.yaml")).unwrap();
    let diagnostic = refused(&fixture.load()).clone();
    assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE);
    assert_eq!(diagnostic.file, fixture.path("host/schemas/widget.yaml"));
    assert_eq!(diagnostic.member.as_deref(), Some("triggers[0]"));
}

#[test]
fn an_invalid_trigger_refuses_the_registration() {
    for text in [
        "kind: schema-trigger\nmatch: {}\n$schema: {}\n",
        "kind: schema\ntypes: {}\n",
        "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    gadget: nope@./library.yaml\n",
    ] {
        let fixture = Fixture::new();
        fixture.write("host/schemas/gadget.yaml", text);
        let diagnostic = refused(&fixture.load()).clone();
        assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE);
        assert_eq!(diagnostic.file, fixture.path("host/schemas/gadget.yaml"), "{text}");
        assert_eq!(diagnostic.member.as_deref(), Some("triggers[1]"));
    }
}

#[test]
fn a_stale_lifecycle_artifact_refuses_the_registration() {
    let fixture = Fixture::new();
    fixture.write("host/schemas/globals.yaml", &GLOBALS.replace("the acme host's globals", "edited"));
    let diagnostic = refused(&fixture.load()).clone();
    assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE);
    assert_eq!(diagnostic.file, fixture.path("host/schemas/globals.lifecycle.json"));
    assert_eq!(diagnostic.member.as_deref(), Some("lifecycle"));
    assert!(diagnostic.message.contains("regenerate the artifact"), "{}", diagnostic.message);
}

#[test]
fn an_invalid_global_catalog_refuses_the_registration() {
    let fixture = Fixture::new();
    fixture.write("host/schemas/globals.yaml", "kind: schema\n$schema:\n    fm:\n        x: string\n");
    let diagnostic = refused(&fixture.load()).clone();
    assert_eq!(diagnostic.file, fixture.path("host/schemas/globals.yaml"));
    assert_eq!(diagnostic.member.as_deref(), Some("globals"));
}

#[test]
fn a_host_that_differs_from_the_lifecycle_profile_refuses_the_registration() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| manifest.replace("host: acme", "host: other"));
    let diagnostic = refused_in_manifest(&fixture, &fixture.load());
    assert_eq!(diagnostic.member.as_deref(), Some("host"));
    assert_eq!(diagnostic.line, Some(3));
    assert!(diagnostic.message.contains("records host `acme`"), "{}", diagnostic.message);
}

#[test]
fn a_bad_or_stale_facts_artifact_refuses_the_registration() {
    for facts in ["{}", "not json", &facts_artifact().replace("\"catalog_hash\": \"", "\"catalog_hash\": \"0")] {
        let fixture = Fixture::new();
        fixture.write("host/schemas/functions.facts.json", facts);
        let diagnostic = refused(&fixture.load()).clone();
        assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE);
        assert_eq!(diagnostic.file, fixture.path("host/schemas/functions.facts.json"), "{facts}");
        assert_eq!(diagnostic.member.as_deref(), Some("functions[0].facts"));
    }
}

#[test]
fn an_identifying_trigger_on_md_owned_keys_is_too_broad() {
    for identify in [
        "kind: schema-trigger\nmatch:\n    initialize: object(required)\n$schema: {}\n",
        "kind: schema-trigger\nmatch:\n    group:\n        - widget: object(required)\n        - composed: object(required)\n$schema: {}\n",
        "kind: schema-trigger\nmatch:\n    group:\n        - widget: object(required)\n        - $path: \"**/*.md\"\n$schema: {}\n",
    ] {
        let fixture = Fixture::new();
        fixture.write("host/schemas/identify.yaml", identify);
        let diagnostic = refused(&fixture.load()).clone();
        assert_eq!(diagnostic.code, HOST_IDENTIFY_TOO_BROAD_CODE, "{identify}");
        assert_eq!(diagnostic.file, fixture.path("host/schemas/identify.yaml"));
        assert_eq!(diagnostic.member.as_deref(), Some("identify"));
        assert_eq!(diagnostic.line, Some(3), "at the `match` value: {diagnostic:#?}");
        assert!(diagnostic.message.contains("non-null top-level key only this host owns"), "{}", diagnostic.message);
    }
}

// ── duplicates and agreement ────────────────────────────────────────────────

#[test]
fn entries_naming_one_manifest_load_it_once() {
    let fixture = Fixture::new();
    fixture.write("repo/.darkmatter.yaml", "register: []\n");
    let entries = [
        fixture.entry("../host/darkmatter-host.yaml", 0),
        fixture.entry("../host/./schemas/../darkmatter-host.yaml", 1),
        Registration {
            reference: "../host/darkmatter-host.yaml".to_string(),
            index: 0,
            declared_in: fixture.path("repo/.darkmatter.yaml"),
            revision: ConfigRevision::RepositoryWorktree,
        },
    ];
    let loaded = load_entries(&entries, &fixture.context(), md_owned_names());
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let [profile] = loaded.profiles.as_slice() else { panic!("one profile: {:?}", loaded.profiles) };
    assert_eq!(profile.entries(), entries.as_slice());
}

#[test]
fn a_refused_manifest_named_twice_is_reported_once() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| manifest.replace("host: acme", "host: \"\""));
    let loaded = fixture.load_references(&["../host/darkmatter-host.yaml", "../host/darkmatter-host.yaml"]);
    refused(&loaded);
}

#[test]
fn distinct_manifests_with_the_same_content_agree() {
    let fixture = Fixture::new();
    fixture.install("copy");
    let loaded = fixture.load_references(&["../host/darkmatter-host.yaml", "../copy/darkmatter-host.yaml"]);
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles.len(), 2);
    assert_eq!(loaded.profiles[0].revision(), loaded.profiles[1].revision());
    let claims = agreement(&loaded.profiles);
    let [HostClaim::Agreed { profile, collapsed }] = claims.as_slice() else {
        panic!("one agreed claim")
    };
    assert!(Arc::ptr_eq(profile, &loaded.profiles[0]));
    assert_eq!(collapsed.len(), 1);
}

#[test]
fn a_comment_does_not_change_the_revision_but_content_does() {
    let fixture = Fixture::new();
    let before = fixture.load().profiles[0].revision().to_string();
    fixture.write("host/darkmatter-host.yaml", &format!("# written by acme\n{MANIFEST}"));
    assert_eq!(fixture.load().profiles[0].revision(), before);
    fixture.write("host/schemas/library.yaml", &LIBRARY.replace("The gadget's name.", "Its name."));
    assert_ne!(fixture.load().profiles[0].revision(), before, "an import's content is part of the revision");
}

#[test]
fn distinct_manifests_with_different_content_conflict() {
    let fixture = Fixture::new();
    fixture.install("copy");
    fixture.write("copy/darkmatter-host.yaml", &MANIFEST.replace("version: 1.2.0", "version: 1.3.0"));
    let loaded = fixture.load_references(&["../host/darkmatter-host.yaml", "../copy/darkmatter-host.yaml"]);
    assert_eq!(loaded.profiles.len(), 2);
    let claims = agreement(&loaded.profiles);
    let [claim @ HostClaim::Conflict { host, claims: conflicting }] = claims.as_slice() else {
        panic!("one conflict: {claims:?}")
    };
    assert_eq!((host.as_str(), conflicting.len()), ("acme", 2));
    let diagnostics = claim.diagnostics();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|diagnostic| diagnostic.code == HOST_REGISTRATION_CODE));
    assert_eq!(diagnostics[0].file, fixture.manifest());
    assert!(diagnostics[0].message.contains("copy"), "{}", diagnostics[0].message);
}

#[test]
fn different_hosts_are_separate_claims() {
    let fixture = Fixture::new();
    fixture.install("other");
    fixture.write("other/darkmatter-host.yaml", &MANIFEST.replace("host: acme", "host: other"));
    fixture.write("other/schemas/globals.lifecycle.json", &lifecycle_artifact_for("other", GLOBALS));
    let loaded = fixture.load_references(&["../host/darkmatter-host.yaml", "../other/darkmatter-host.yaml"]);
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let claims = agreement(&loaded.profiles);
    let hosts: Vec<&str> = claims.iter().map(HostClaim::host).collect();
    assert_eq!(hosts, ["acme", "other"]);
}

// ── md-owned names and projection ───────────────────────────────────────────

#[test]
fn md_owned_names_cover_the_baseline_events_and_globals_but_no_claudine_key() {
    let owned = md_owned_names();
    for name in ["initialize", "composed", "failed", "cancelled", "finalize", "fm", "ctx", "env", "err", "timing", "$schema"] {
        assert!(owned.contains(name), "`{name}` is md-owned: {owned:?}");
    }
    for key in ["sequence", "start", "blocked", "success", "failure", "loop", "prompt", "agent", "model"] {
        assert!(!owned.contains(key), "Claudine's identifying key `{key}` is not md-owned");
    }
}

#[test]
fn a_refusal_projects_to_a_located_warning_record() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| replace_field(manifest, "host", "null"));
    let loaded = fixture.load();
    let record = darkmatter::markdown::compose::report_diagnostics::registration_diagnostic(&loaded.diagnostics[0]);
    assert_eq!((record.code.as_str(), record.severity), (HOST_REGISTRATION_CODE, Severity::Warning));
    let source = record.source.expect("a source");
    assert_eq!(source.file.as_deref(), Some(fixture.manifest().as_path()));
    assert_eq!(source.line, Some(3));
}
