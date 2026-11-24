//! `darkmatter::config::registration`: loading format 3 `darkmatter-host`
//! bundles into passive host profiles. Each case makes one edit to the valid
//! `acme` bundle: the classification of every `files` member by its own
//! `kind`, the all-or-nothing refusals of each member, registration-wide
//! `name` labels, the YAML reader of member envelopes, what a refused entry
//! keeps watching, duplicate entries, same-host agreement, the `md`-owned
//! names an identifying trigger may not rely on, and grant-independent,
//! write-free loading. The manifest envelope and digest table themselves are
//! `host_registration_snapshot`'s; the namespace catalog is
//! `host_namespaces`'.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::config::load::{ConfigLoadMode, ConfigRevision, ConfigSnapshot, Registration};
use darkmatter::config::registration::{
    HOST_IDENTIFY_TOO_BROAD_CODE, HOST_REGISTRATION_CODE, HostClaim, LoadedRegistrations, RegistrationDiagnostic,
    agreement, load_entries, load_registrations, md_owned_names,
};
use darkmatter::markdown::compose::expression::Severity;

const MANIFEST_REFERENCE: &str = "../host/darkmatter-host.yaml";

/// Every refusal of a load that published nothing.
fn refusals(loaded: &LoadedRegistrations) -> &[RegistrationDiagnostic] {
    assert!(loaded.profiles.is_empty(), "nothing is published: {:?}", loaded.profiles);
    assert!(!loaded.diagnostics.is_empty(), "the entry is refused");
    for diagnostic in &loaded.diagnostics {
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert!(diagnostic.message.contains("entry 1 of `register`"), "names the entry: {}", diagnostic.message);
    }
    &loaded.diagnostics
}

/// The one refusal of a load that published nothing.
fn refused(loaded: &LoadedRegistrations) -> &RegistrationDiagnostic {
    let all = refusals(loaded);
    assert_eq!(all.len(), 1, "{all:#?}");
    &all[0]
}

/// The one `dm.host.registration` refusal, in `file`, at member `member`.
fn refused_at(loaded: &LoadedRegistrations, file: &Path, member: &str) -> RegistrationDiagnostic {
    let diagnostic = refused(loaded).clone();
    assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE, "{diagnostic:#?}");
    assert_eq!(diagnostic.file, file, "{diagnostic:#?}");
    assert_eq!(diagnostic.member.as_deref(), Some(member), "{diagnostic:#?}");
    diagnostic
}

/// The fixture with the `acme` bundle plus the extra `files` member
/// `schemas/extra.yaml` (`files[5]`, after the expression catalog) holding `text`.
fn with_extra(text: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.install_bundle("host", &Bundle::acme().with_file("schemas/extra.yaml", text));
    fixture
}

const EXTRA_MEMBER: &str = "files[5]";

/// The `acme` bundle's expression catalog entry, which its facts companion
/// is reported under.
const CATALOG_MEMBER: &str = "files[4]";

/// Every bundle-relative member path of the `acme` bundle, manifest first.
fn acme_paths() -> Vec<String> {
    let mut paths = vec![MANIFEST_NAME.to_string()];
    paths.extend(Bundle::acme().listed());
    paths
}

// ── control ─────────────────────────────────────────────────────────────────

#[test]
fn a_valid_host_loads_every_member_into_one_profile() {
    let fixture = Fixture::new();
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert!(loaded.watches.is_empty(), "a loaded entry is watched through its profile: {:?}", loaded.watches);
    let [profile] = loaded.profiles.as_slice() else { panic!("one profile: {:?}", loaded.profiles) };

    assert_eq!((profile.host(), profile.version()), ("acme", "1.2.0"));
    assert_eq!(profile.manifest().path, fixture.manifest());
    let manifest = Bundle::acme().manifest();
    assert_eq!(profile.manifest().content_hash, darkmatter::config::migration_receipt::content_hash(manifest.as_bytes()));
    assert_eq!(profile.entries().len(), 1);
    // Bundle-path order, not `files` order (widget is listed first).
    let triggers: Vec<(&Path, &str)> =
        profile.triggers().iter().map(|trigger| (trigger.path(), trigger.bundle_path.as_str())).collect();
    let gadget = fixture.path("host/schemas/gadget.yaml");
    let widget = fixture.path("host/schemas/widget.yaml");
    assert_eq!(triggers, [(gadget.as_path(), "schemas/gadget.yaml"), (widget.as_path(), "schemas/widget.yaml")]);
    let widget_payload = &profile.triggers()[1].payload.json_schema;
    assert!(widget_payload["properties"]["widget"].is_object(), "the relative import resolved: {widget_payload}");
    assert_eq!(profile.libraries(), ["schemas/library.yaml"]);
    assert!(profile.yaml_kinds().is_empty());
    assert!(profile.globals().global("run").is_some());
    assert_eq!(profile.catalog_hash(), catalog_hash_of(&[("schemas/run.yaml", RUN_NAMESPACE)]));
    assert_eq!(profile.lifecycle().registry().host(), "acme");
    assert_eq!(profile.lifecycle_path(), fixture.path("host/schemas/globals.lifecycle.json"));
    let [functions] = profile.functions() else { panic!("one function set") };
    assert_eq!(functions.functions.function_names(), ["acme_label"]);
    assert!(functions.functions.incomplete_functions().is_empty());
    assert_eq!(profile.revision(), profile.snapshot().revision());
    assert_eq!(profile.revision().len(), 64);
}

#[test]
fn every_file_read_is_a_dependency_once_with_its_hash() {
    let fixture = Fixture::new();
    let loaded = fixture.load();
    let profile = &loaded.profiles[0];
    let paths: Vec<&Path> = profile.dependencies().iter().map(|dependency| dependency.path.as_path()).collect();
    for member in acme_paths() {
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
fn a_loaded_profile_watches_its_manifest_and_every_member() {
    let fixture = Fixture::new();
    let loaded = fixture.load();
    let candidates = loaded.profiles[0].candidates();
    for member in acme_paths() {
        assert!(candidates.contains(&fixture.path(&format!("host/{member}"))), "{member}: {candidates:#?}");
    }
}

/// `files: []` is a legal empty inventory: a host with no namespaces has an
/// empty catalog, and its artifact records that catalog's hash.
#[test]
fn a_host_with_no_files_loads() {
    let fixture = Fixture::new();
    let lifecycle = "schemas/globals.lifecycle.json".to_string();
    let bundle = Bundle {
        files: Vec::new(),
        members: [(lifecycle.clone(), lifecycle_artifact_declaring("acme", &[], &catalog_hash_of(&[])))].into(),
        lifecycle,
        ..Bundle::acme()
    };
    fixture.install_bundle("empty", &bundle);

    let loaded = fixture.load_references(&["../empty/darkmatter-host.yaml"]);

    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let profile = &loaded.profiles[0];
    assert!(profile.triggers().is_empty() && profile.functions().is_empty() && profile.namespaces().is_empty());
    assert_eq!(profile.globals().globals().count(), 0);
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

// ── the manifest, through `load_entries` ────────────────────────────────────

#[test]
fn a_format_1_manifest_is_refused_with_the_upgrade_warning_and_watched() {
    let fixture = Fixture::new();
    fixture.write(
        "host/darkmatter-host.yaml",
        "kind: darkmatter-host\nformat_version: 1\nhost: acme\nversion: 1.2.0\nidentify: ./schemas/widget.yaml\n\
         triggers: []\nglobals: ./schemas/run.yaml\nlifecycle: ./schemas/globals.lifecycle.json\nfunctions: []\n",
    );

    let loaded = fixture.load();

    let diagnostic = refused(&loaded);
    assert_eq!((diagnostic.code, &diagnostic.file), (HOST_REGISTRATION_CODE, &fixture.manifest()));
    assert_eq!(diagnostic.line, Some(2), "at `format_version`: {diagnostic:#?}");
    assert!(
        diagnostic.message.ends_with(
            "unsupported host registration format version 1; update Darkmatter and the host tool, then republish the registration"
        ),
        "{}",
        diagnostic.message
    );
    assert_eq!(loaded.watches, [fixture.manifest()]);
}

/// The frozen format 2 manifest Claudine last wrote, installed where Claudine
/// installs it (`~/.claudine/registrations/darkmatter/`), is refused at its
/// `format_version` with the upgrade instruction, and stays watched.
#[test]
fn the_frozen_format_2_install_is_refused_with_its_version_and_watched() {
    let frozen = biscuit_test_harness::manifest_dir!().join("tests/fixtures/host_registration_v2/darkmatter");
    let fixture = Fixture::new();
    let installed = "home/.claudine/registrations/darkmatter";
    let mut pending = vec![frozen.clone()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(&frozen).unwrap().to_string_lossy().replace('\\', "/");
                fixture.write(&format!("{installed}/{relative}"), &std::fs::read_to_string(&path).unwrap());
            }
        }
    }
    let manifest = fixture.path(&format!("{installed}/manifest.yaml"));
    let text = std::fs::read_to_string(&manifest).unwrap();
    let line = text.lines().position(|line| line.starts_with("format_version:")).unwrap() + 1;

    let loaded = fixture.load_references(&["../home/.claudine/registrations/darkmatter/manifest.yaml"]);

    let diagnostic = refused(&loaded);
    assert_eq!((diagnostic.code, &diagnostic.file), (HOST_REGISTRATION_CODE, &manifest));
    assert_eq!((diagnostic.line, diagnostic.column), (Some(line), Some("format_version: ".len() + 1)), "{diagnostic:#?}");
    assert!(
        diagnostic.message.ends_with(
            "unsupported host registration format version 2; update Darkmatter and the host tool, then republish the registration"
        ),
        "{}",
        diagnostic.message
    );
    assert!(loaded.watches.contains(&manifest), "{:#?}", loaded.watches);
}

#[test]
fn an_absent_manifest_field_is_a_located_refusal() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| manifest.replace("version: 1.2.0\n", ""));
    let diagnostic = refused(&fixture.load()).clone();
    assert_eq!((diagnostic.code, diagnostic.file.clone()), (HOST_REGISTRATION_CODE, fixture.manifest()));
    assert!(diagnostic.line.is_some() && diagnostic.message.contains("version"), "{diagnostic:#?}");
}

#[test]
fn a_missing_manifest_is_reported_at_the_declaring_file() {
    let fixture = Fixture::new();
    let diagnostic = refused(&fixture.load_references(&["../host/absent.yaml"])).clone();
    assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE);
    assert_eq!(diagnostic.file, fixture.path("home/.darkmatter.yaml"));
    assert!(diagnostic.message.contains("names no existing file"), "{}", diagnostic.message);
}

// ── member classification ───────────────────────────────────────────────────

#[test]
fn a_second_type_library_is_a_valid_member() {
    let fixture = with_extra("kind: schema\n# a comment\ntypes:\n    bolt: \"string -> A 'quoted' bolt.\"\n");
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles[0].libraries(), ["schemas/extra.yaml", "schemas/library.yaml"]);
}

#[test]
fn a_member_that_is_no_registered_kind_is_refused_at_its_entry() {
    for (text, needle) in [
        ("kind: widget-thing\ntypes:\n    bolt: string\n", "its `kind` is `widget-thing`"),
        ("types:\n    bolt: string\n", "declares no `kind`"),
        ("kind: [schema]\ntypes:\n    bolt: string\n", "`kind` must be a string"),
        ("kind: example\nvalue: 1\n", "its `kind` is `example`"),
        ("kind: schema\n$schema:\n    bolt: string\ntypes:\n    bolt: string\n", "may not declare `$schema`"),
        ("kind: schema\n$schema:\n    bolt: string\n", "may not declare `$schema`"),
        ("kind: schema\ntypes: {}\n", "non-empty `types`"),
        ("kind: schema\ntypes:\n    bolt: string\nextra: 1\n", "`extra` is not a field of a `kind: schema` member"),
    ] {
        let fixture = with_extra(text);
        let diagnostic = refused_at(&fixture.load(), &fixture.path("host/schemas/extra.yaml"), EXTRA_MEMBER);
        assert!(diagnostic.message.contains(needle), "{text}: {}", diagnostic.message);
    }
}

/// A trigger's and a library's `types` keep their envelope rules through the
/// later declared-type check: a trigger may write an empty table but never a
/// blank or numeric one; a library needs a non-empty table. Each refusal is
/// at the member's `types`.
#[test]
fn trigger_and_library_types_tables_keep_their_envelope_rules() {
    const TRIGGER: &str = "kind: schema-trigger\nmatch:\n    bolt: string(required)\n";
    let fixture = with_extra(&format!("{TRIGGER}types: {{}}\n"));
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "an empty trigger table: {:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles[0].triggers().len(), 3);
    // A library's admitted `$schema: null` is the absent payload there too.
    let loaded = with_extra("kind: schema\n$schema:\ntypes:\n    bolt: string\n").load();
    assert!(loaded.diagnostics.is_empty(), "a library's null payload: {:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles[0].libraries(), ["schemas/extra.yaml", "schemas/library.yaml"]);

    for text in [
        format!("{TRIGGER}types:\n"),
        format!("{TRIGGER}types: null\n"),
        format!("{TRIGGER}types: 7\n"),
        "kind: schema\ntypes:\n".to_string(),
        "kind: schema\ntypes: 7\n".to_string(),
    ] {
        let fixture = with_extra(&text);
        let diagnostic = refused_at(&fixture.load(), &fixture.path("host/schemas/extra.yaml"), EXTRA_MEMBER);
        let line = text.lines().position(|line| line.starts_with("types:")).unwrap() + 1;
        assert_eq!(diagnostic.line, Some(line), "{text:?}: {diagnostic:#?}");
        assert!(diagnostic.message.contains("`types`"), "{text:?}: {}", diagnostic.message);
    }
}

/// The YAML reader of member envelopes: a duplicate key (top level or
/// nested), trailing garbage, a second document, and a bare null are each a
/// refusal of the member; comments and quoted values are ordinary YAML.
#[test]
fn the_member_yaml_reader_refuses_every_malformed_document() {
    for text in [
        "kind: schema\ntypes:\n    bolt: string\ntypes:\n    nut: string\n",
        "kind: schema\ntypes:\n    bolt: string\n    bolt: number\n",
        "kind: schema\ntypes:\n    bolt: string\n: : ]\n",
        "kind: schema\ntypes:\n    bolt: string\n---\nkind: schema\n",
        "null\n",
        "~\n",
    ] {
        let fixture = with_extra(text);
        refused_at(&fixture.load(), &fixture.path("host/schemas/extra.yaml"), EXTRA_MEMBER);
    }
}

#[test]
fn a_matcher_only_trigger_loads_with_an_empty_payload() {
    let fixture = with_extra("kind: schema-trigger\nmatch:\n    bolt: string(required)\n");
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let trigger = loaded.profiles[0].triggers().iter().find(|trigger| trigger.bundle_path == "schemas/extra.yaml").expect("registered");
    assert!(trigger.payload.shape.as_ref().is_none_or(|shape| shape.properties.is_empty()), "{:?}", trigger.payload.shape);
    assert!(
        trigger.payload.json_schema.get("properties").is_none_or(|properties| properties.as_object().is_some_and(serde_json::Map::is_empty)),
        "contributes nothing: {}",
        trigger.payload.json_schema
    );
}

#[test]
fn an_invalid_trigger_refuses_the_registration_at_its_entry() {
    for text in [
        "kind: schema-trigger\nmatch: {}\n",
        "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    gadget: nope@./library.yaml\n",
        "kind: schema-trigger\nmatch:\n    gadget: string(required)\ndescription: null\n",
    ] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", text);
        refused_at(&fixture.load(), &fixture.path("host/schemas/gadget.yaml"), "files[1]");
    }
}

#[test]
fn an_identifying_trigger_on_md_owned_keys_is_too_broad_at_its_match() {
    for gadget in [
        "kind: schema-trigger\nmatch:\n    title: string(required)\n",
        "kind: schema-trigger\nmatch:\n    initialize: object(required)\n",
        "kind: schema-trigger\nmatch:\n    group:\n        - gadget: string(required)\n        - composed: object(required)\n",
        "kind: schema-trigger\nmatch:\n    group:\n        - gadget: string(required)\n        - $path: \"**/*.md\"\n",
    ] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", gadget);
        let diagnostic = refused(&fixture.load()).clone();
        assert_eq!(diagnostic.code, HOST_IDENTIFY_TOO_BROAD_CODE, "{gadget}");
        assert_eq!(diagnostic.file, fixture.path("host/schemas/gadget.yaml"));
        assert_eq!(diagnostic.member.as_deref(), Some("files[1]"));
        assert_eq!(diagnostic.line, Some(3), "at the `match` value: {diagnostic:#?}");
    }
}

// ── labels ──────────────────────────────────────────────────────────────────

#[test]
fn distinct_labels_are_valid() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/widget.yaml", &format!("name: widget\n{WIDGET_TRIGGER}"));
    fixture.write_member("host", "schemas/library.yaml", &format!("name: parts\n{LIBRARY}"));
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
}

#[test]
fn a_label_shared_by_a_trigger_and_a_library_is_refused_on_both() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/widget.yaml", &format!("name: parts\n{WIDGET_TRIGGER}"));
    fixture.write_member("host", "schemas/library.yaml", &format!("name: parts\n{LIBRARY}"));

    let loaded = fixture.load();

    let mut located: Vec<(PathBuf, Option<usize>, Option<String>)> =
        refusals(&loaded).iter().map(|diagnostic| (diagnostic.file.clone(), diagnostic.line, diagnostic.member.clone())).collect();
    located.sort();
    assert_eq!(
        located,
        [
            (fixture.path("host/schemas/library.yaml"), Some(1), Some("files[2]".to_string())),
            (fixture.path("host/schemas/widget.yaml"), Some(1), Some("files[0]".to_string())),
        ]
    );
    assert!(loaded.diagnostics.iter().all(|diagnostic| diagnostic.message.contains("the name `parts` is also declared by")));
}

#[test]
fn a_label_outside_the_name_pattern_is_refused() {
    let fixture = with_extra("name: Parts_List\nkind: schema\ntypes:\n    bolt: string\n");
    let diagnostic = refused_at(&fixture.load(), &fixture.path("host/schemas/extra.yaml"), EXTRA_MEMBER);
    assert!(diagnostic.message.contains("`Parts_List` is not a kebab-case identifier"), "{}", diagnostic.message);
    assert_eq!(diagnostic.line, Some(1), "located at `name`: {diagnostic:#?}");
}

// ── members, and what a refused entry watches ──────────────────────────────

/// `loaded.watches` holds the manifest and every listed member.
fn assert_watches_the_bundle(fixture: &Fixture, loaded: &LoadedRegistrations) {
    for member in acme_paths() {
        assert!(loaded.watches.contains(&fixture.path(&format!("host/{member}"))), "{member}: {:#?}", loaded.watches);
    }
}

#[test]
fn a_missing_member_is_refused_and_watched() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.path("host/schemas/gadget.yaml")).unwrap();
    let loaded = fixture.load();
    let diagnostic = refused_at(&loaded, &fixture.manifest(), "files[1]");
    assert_eq!(diagnostic.line, Some(7), "{diagnostic:#?}");
    assert!(diagnostic.message.contains("missing"), "{}", diagnostic.message);
    assert_watches_the_bundle(&fixture, &loaded);
}

#[test]
fn a_member_that_does_not_match_its_digest_is_refused_and_watched() {
    let fixture = Fixture::new();
    fixture.corrupt_member("host", "schemas/library.yaml", &LIBRARY.replace("The gadget's name.", "Its name."));
    let loaded = fixture.load();
    let diagnostic = refused_at(&loaded, &fixture.manifest(), "files[2]");
    assert!(diagnostic.message.contains("does not match its digest"), "{}", diagnostic.message);
    assert_watches_the_bundle(&fixture, &loaded);
}

#[test]
fn an_unlisted_import_of_a_trigger_is_refused_and_its_target_watched() {
    let fixture = Fixture::new();
    // `extra.yaml` exists beside the trigger but is no member of the bundle.
    fixture.write("host/schemas/extra.yaml", LIBRARY);
    fixture.write_member("host", "schemas/gadget.yaml", &GADGET_TRIGGER.replace("./library.yaml", "./extra.yaml"));
    let loaded = fixture.load();
    refused_at(&loaded, &fixture.path("host/schemas/gadget.yaml"), "files[1]");
    assert_watches_the_bundle(&fixture, &loaded);
    assert!(loaded.watches.contains(&fixture.path("host/schemas/extra.yaml")), "{:#?}", loaded.watches);
}

#[test]
fn a_stale_lifecycle_artifact_refuses_the_registration() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/run.yaml", &RUN_NAMESPACE.replace("The current pass.", "The pass."));
    let diagnostic = refused_at(&fixture.load(), &fixture.path("host/schemas/globals.lifecycle.json"), "lifecycle");
    assert!(diagnostic.message.contains("regenerate the artifact"), "{}", diagnostic.message);
}

#[test]
fn a_format_1_lifecycle_artifact_is_refused() {
    let fixture = Fixture::new();
    let artifact = std::fs::read_to_string(fixture.path("host/schemas/globals.lifecycle.json")).unwrap();
    fixture.write_member("host", "schemas/globals.lifecycle.json", &artifact.replace("\"format_version\": 2", "\"format_version\": 1"));
    let diagnostic = refused_at(&fixture.load(), &fixture.path("host/schemas/globals.lifecycle.json"), "lifecycle");
    assert!(diagnostic.message.contains("unknown `format_version` 1"), "{}", diagnostic.message);
}

#[test]
fn a_host_that_differs_from_the_lifecycle_profile_refuses_the_registration() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| manifest.replace("host: acme", "host: other"));
    let diagnostic = refused_at(&fixture.load(), &fixture.manifest(), "host");
    assert!(diagnostic.message.contains("records host `acme`"), "{}", diagnostic.message);
}

#[test]
fn a_bad_or_stale_facts_artifact_refuses_the_registration() {
    for facts in ["{}", "not json", &facts_artifact().replace("\"catalog_hash\": \"", "\"catalog_hash\": \"0")] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/functions.facts.json", facts);
        let diagnostic = refused_at(&fixture.load(), &fixture.path("host/schemas/functions.facts.json"), CATALOG_MEMBER);
        assert!(!diagnostic.message.contains("imported by"), "a companion is its catalog's entry: {}", diagnostic.message);
    }
}

// ── passivity ───────────────────────────────────────────────────────────────

/// Every file under `root` with its bytes.
fn tree(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    files
}

/// Loading is the same with no `permissions` block and with one that
/// pre-approves nothing, and writes nothing: registration reads no grant.
#[test]
fn loading_is_independent_of_permissions_and_writes_nothing() {
    let fixture = Fixture::new();
    let load_with = |config: &str| {
        fixture.write("home/.darkmatter.yaml", config);
        let snapshot = ConfigSnapshot::load(&fixture.context(), None, ConfigLoadMode::Passive).expect("the configuration loads");
        let before = tree(&fixture.root);
        let loaded = load_registrations(&snapshot, &fixture.context(), md_owned_names());
        assert_eq!(tree(&fixture.root), before, "loading wrote nothing");
        assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
        loaded.profiles[0].revision().to_string()
    };
    let ungranted = load_with("register:\n  - ../host/darkmatter-host.yaml\n");
    let denied = load_with(
        "permissions:\n  file-read: []\n  shell-commands: []\nregister:\n  - ../host/darkmatter-host.yaml\n",
    );
    assert_eq!(ungranted, denied);
}

// ── duplicates and agreement ────────────────────────────────────────────────

#[test]
fn entries_naming_one_manifest_load_it_once() {
    let fixture = Fixture::new();
    fixture.write("repo/.darkmatter.yaml", "register: []\n");
    let entries = [
        fixture.entry(MANIFEST_REFERENCE, 0),
        fixture.entry("../host/./schemas/../darkmatter-host.yaml", 1),
        Registration {
            reference: MANIFEST_REFERENCE.to_string(),
            index: 0,
            declared_in: fixture.path("repo/.darkmatter.yaml"),
            revision: ConfigRevision::RepositoryWorktree,
            at: None,
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
    let loaded = fixture.load_references(&[MANIFEST_REFERENCE, MANIFEST_REFERENCE]);
    refused(&loaded);
}

#[test]
fn distinct_manifests_with_the_same_content_agree() {
    let fixture = Fixture::new();
    fixture.install("copy");
    let loaded = fixture.load_references(&[MANIFEST_REFERENCE, "../copy/darkmatter-host.yaml"]);
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
fn a_comment_does_not_change_the_revision_but_member_content_does() {
    let fixture = Fixture::new();
    let before = fixture.load().profiles[0].revision().to_string();
    let manifest = fixture.manifest_text("host");
    fixture.write("host/darkmatter-host.yaml", &format!("# written by acme\n{manifest}"));
    assert_eq!(fixture.load().profiles[0].revision(), before);
    fixture.write_member("host", "schemas/library.yaml", &LIBRARY.replace("The gadget's name.", "Its name."));
    assert_ne!(fixture.load().profiles[0].revision(), before, "a member's content is part of the revision");
}

#[test]
fn distinct_manifests_with_different_content_conflict() {
    let fixture = Fixture::new();
    fixture.install("copy");
    fixture.write("copy/darkmatter-host.yaml", &fixture.manifest_text("copy").replace("version: 1.2.0", "version: 1.3.0"));
    let loaded = fixture.load_references(&[MANIFEST_REFERENCE, "../copy/darkmatter-host.yaml"]);
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
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (Some(3), Some(7)), "at the manifest's `host`: {diagnostics:#?}");
    assert!(diagnostics[0].message.contains("copy"), "{}", diagnostics[0].message);
}

#[test]
fn different_hosts_are_separate_claims() {
    let fixture = Fixture::new();
    fixture.install_host("other", "other");
    let loaded = fixture.load_references(&[MANIFEST_REFERENCE, "../other/darkmatter-host.yaml"]);
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let claims = agreement(&loaded.profiles);
    let hosts: Vec<&str> = claims.iter().map(HostClaim::host).collect();
    assert_eq!(hosts, ["acme", "other"]);
}

// ── md-owned names and projection ───────────────────────────────────────────

#[test]
fn md_owned_names_cover_the_baseline_events_and_globals_but_no_claudine_key() {
    let owned = md_owned_names();
    for name in ["initialize", "composed", "failed", "cancelled", "finalize", "fm", "ctx", "env", "err", "timing", "$schema", "title"] {
        assert!(owned.contains(name), "`{name}` is md-owned: {owned:?}");
    }
    for key in ["sequence", "start", "blocked", "success", "failure", "loop", "prompt", "agent", "model"] {
        assert!(!owned.contains(key), "Claudine's identifying key `{key}` is not md-owned");
    }
}

#[test]
fn a_refusal_projects_to_a_located_warning_record() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| manifest.replace("host: acme", "host: null"));
    let loaded = fixture.load();
    let record = darkmatter::markdown::compose::report_diagnostics::registration_diagnostic(&loaded.diagnostics[0]);
    assert_eq!((record.code.as_str(), record.severity), (HOST_REGISTRATION_CODE, Severity::Warning));
    let source = record.source.expect("a source");
    assert_eq!(source.file.as_deref(), Some(fixture.manifest().as_path()));
    assert_eq!(source.line, Some(3));
}
