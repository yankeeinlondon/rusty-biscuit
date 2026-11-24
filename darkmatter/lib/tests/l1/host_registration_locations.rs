//! Where a refused registration is reported: every `dm.host.registration`
//! refusal names the file its problem is in and, when the problem has a
//! source position, the 1-based line and column of the failed field. Each
//! case makes one edit to the valid `acme` bundle and republishes it, so the
//! only refusal is the one under test. An error in a file another member
//! imports is reported in that file, at the failed declaration, and names
//! the member that imported it; a JSON artifact's refusal is reported at the
//! field its reader judged.

use std::path::Path;

use crate::host_fixtures::*;
use darkmatter::config::registration::{HOST_REGISTRATION_CODE, LoadedRegistrations, RegistrationDiagnostic};

/// Every refusal of a load that published nothing.
fn refusals(loaded: &LoadedRegistrations) -> Vec<RegistrationDiagnostic> {
    assert!(loaded.profiles.is_empty(), "nothing is published");
    assert!(!loaded.diagnostics.is_empty(), "the entry is refused");
    for diagnostic in &loaded.diagnostics {
        assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE, "{diagnostic:#?}");
    }
    loaded.diagnostics.clone()
}

/// The one refusal of a load that published nothing.
fn refused(loaded: &LoadedRegistrations) -> RegistrationDiagnostic {
    let all = refusals(loaded);
    assert_eq!(all.len(), 1, "{all:#?}");
    all[0].clone()
}

/// `diagnostic`'s file, member, and position.
fn site(diagnostic: &RegistrationDiagnostic) -> (&Path, Option<&str>, Option<(usize, usize)>) {
    (diagnostic.file.as_path(), diagnostic.member.as_deref(), diagnostic.line.zip(diagnostic.column))
}

/// The 1-based line and column of the first `needle` in `text`.
fn position_of(text: &str, needle: &str) -> (usize, usize) {
    let offset = text.find(needle).unwrap_or_else(|| panic!("`{needle}` is in the text"));
    let before = &text[..offset];
    (before.matches('\n').count() + 1, before.len() - before.rfind('\n').map_or(0, |at| at + 1) + 1)
}

/// The 1-based line and column of the declaration that writes the first
/// `reference` in `text`: the value of the mapping entry on its line, where
/// a refusal of the reference points.
pub(crate) fn declaration_of(text: &str, reference: &str) -> (usize, usize) {
    let offset = text.find(reference).unwrap_or_else(|| panic!("`{reference}` is in the text"));
    let line = text[..offset].rfind('\n').map_or(0, |at| at + 1);
    let value = line + text[line..offset].rfind(": ").expect("the reference is a mapping entry's value") + 2;
    let before = &text[..value];
    (before.matches('\n').count() + 1, before.len() - line + 1)
}

// ── trigger members ─────────────────────────────────────────────────────────

#[test]
fn an_invalid_trigger_label_is_located_at_its_name() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/gadget.yaml", &format!("name: Bad_Name\n{GADGET_TRIGGER}"));
    let diagnostic = refused(&fixture.load());
    assert_eq!(site(&diagnostic), (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), Some((1, 7))));
}

#[test]
fn invalid_match_syntax_is_located_at_the_condition() {
    for (condition, needle) in [("gadget: strin(required)", "strin("), ("gadget: [1, 2]", "[1, 2]")] {
        let text = format!("kind: schema-trigger\nmatch:\n    {condition}\n");
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", &text);
        let diagnostic = refused(&fixture.load());
        let expected = Some(position_of(&text, needle));
        assert_eq!(site(&diagnostic), (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), expected), "{condition}");
    }
}

#[test]
fn a_grouped_trigger_loads_unedited() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/gadget.yaml", GROUPED_GADGET_TRIGGER);
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles.len(), 1);
}

/// A condition that fails its type grammar, or uses a constraint a match may
/// not, is located at its own value: in a flat `match` and in a nested
/// `all`/`group` list alike, never at the payload property of the same name
/// or at the whole `match` block.
#[test]
fn a_failing_match_condition_is_located_at_its_value_flat_or_grouped() {
    let cases = [
        ("flat typo", GADGET_TRIGGER, "gadget: string(required)", "gadget: string(requird)", "string(requird)"),
        ("grouped typo", GROUPED_GADGET_TRIGGER, "gadget: string(required)", "gadget: string(requird)", "string(requird)"),
        ("flat import", GADGET_TRIGGER, "gadget: string(required)", "gadget: \"gadget(required)@./library.yaml\"", "\"gadget(required)@"),
        (
            "grouped import",
            GROUPED_GADGET_TRIGGER,
            "gadget: string(required)",
            "gadget: \"gadget(required)@./library.yaml\"",
            "\"gadget(required)@",
        ),
    ];
    for (case, trigger, condition, edited, needle) in cases {
        let text = trigger.replacen(condition, edited, 1);
        assert_ne!(text, trigger, "{case}: the edit applies");
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", &text);
        let diagnostic = refused(&fixture.load());
        let expected = Some(position_of(&text, needle));
        assert_eq!(
            site(&diagnostic),
            (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), expected),
            "{case}: {diagnostic:#?}"
        );
    }
}

#[test]
fn an_unlisted_import_is_located_at_its_reference() {
    let fixture = Fixture::new();
    fixture.write("host/schemas/extra.yaml", LIBRARY);
    let text = GADGET_TRIGGER.replace("./library.yaml", "./extra.yaml");
    fixture.write_member("host", "schemas/gadget.yaml", &text);
    let diagnostic = refused(&fixture.load());
    let expected = Some(declaration_of(&text, "./extra.yaml"));
    assert_eq!(site(&diagnostic), (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), expected));
}

// ── imported libraries ──────────────────────────────────────────────────────

/// The library's own refusal and each importer's are all reported at the
/// library's `name`; an importer's names the importer.
#[test]
fn an_invalid_library_label_is_reported_in_the_library_for_every_importer() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/library.yaml", &format!("name: Parts_List\n{LIBRARY}"));
    let library = fixture.path("host/schemas/library.yaml");

    let all = refusals(&fixture.load());

    assert_eq!(all.len(), 3, "{all:#?}");
    for diagnostic in &all {
        assert_eq!(site(diagnostic), (library.as_path(), Some("files[2]"), Some((1, 7))), "{diagnostic:#?}");
    }
    let mut importers: Vec<bool> = ["imported by `files[0]`", "imported by `files[1]`"]
        .iter()
        .map(|importer| all.iter().any(|diagnostic| diagnostic.message.contains(importer)))
        .collect();
    importers.dedup();
    assert_eq!(importers, [true], "both importers are named: {all:#?}");
}

/// The library's own refusal (every declaration it makes is checked) and
/// its importer's are both at the failed declaration.
#[test]
fn an_unknown_type_in_an_imported_library_is_located_at_its_declaration() {
    let fixture = Fixture::new();
    let text = LIBRARY.replace("number ->", "numbr ->");
    fixture.write_member("host", "schemas/library.yaml", &text);
    let all = refusals(&fixture.load());
    assert_eq!(all.len(), 2, "{all:#?}");
    let expected = Some(position_of(&text, "\"numbr"));
    for diagnostic in &all {
        assert_eq!(site(diagnostic), (fixture.path("host/schemas/library.yaml").as_path(), Some("files[2]"), expected), "{diagnostic:#?}");
    }
    assert!(
        all.iter().any(|diagnostic| diagnostic.message.contains("imported by `files[0]` (`schemas/widget.yaml`)")),
        "{all:#?}"
    );
}

#[test]
fn a_yaml_kind_importing_a_malformed_library_is_located_in_the_library() {
    let library = LIBRARY.replace("number ->", "numbr ->");
    let kind = "kind: yaml-schema\n$schema:\n    kind: literal(part; required)\n    w: widget@./library.yaml\n";
    let fixture = Fixture::new();
    fixture.install_bundle(
        "host",
        &Bundle::acme().with_file("schemas/part.yaml", kind).with_member("schemas/library.yaml", &library),
    );
    let all = refusals(&fixture.load());
    let from_kind = all.iter().find(|diagnostic| diagnostic.message.contains("imported by `files[5]`")).expect("the kind's refusal");
    let expected = Some(position_of(&library, "\"numbr"));
    assert_eq!(site(from_kind), (fixture.path("host/schemas/library.yaml").as_path(), Some("files[2]"), expected));
}

// ── member YAML and namespaces ──────────────────────────────────────────────

#[test]
fn a_duplicate_kind_is_located_at_the_repeated_key() {
    let fixture = Fixture::new();
    let text = RUN_NAMESPACE.replacen("kind: namespace-schema\n", "kind: namespace-schema\nkind: namespace-schema\n", 1);
    fixture.write_member("host", "schemas/run.yaml", &text);
    let all = refusals(&fixture.load());
    let run = all.iter().find(|diagnostic| diagnostic.member.as_deref() == Some("files[3]")).expect("the namespace's refusal");
    assert_eq!(site(run), (fixture.path("host/schemas/run.yaml").as_path(), Some("files[3]"), Some((2, 1))));
}

#[test]
fn a_namespace_payload_failure_is_located_at_the_failed_declaration() {
    let fixture = Fixture::new();
    let text = RUN_NAMESPACE.replace("\"number ->", "\"numbr ->");
    fixture.write_member("host", "schemas/run.yaml", &text);
    let diagnostic = refused(&fixture.load());
    let expected = Some(position_of(&text, "\"numbr"));
    assert_eq!(site(&diagnostic), (fixture.path("host/schemas/run.yaml").as_path(), Some("files[3]"), expected));
}

#[test]
fn a_namespace_importing_a_malformed_library_is_located_in_the_library() {
    let library = LIBRARY.replace("number ->", "numbr ->");
    let namespace = "kind: namespace-schema\nnamespace: run\n$schema:\n    part: widget@./library.yaml\n";
    let fixture = Fixture::new();
    fixture.install_bundle(
        "host",
        &Bundle::acme().with_member("schemas/run.yaml", namespace).with_member("schemas/library.yaml", &library),
    );
    let all = refusals(&fixture.load());
    let from_namespace =
        all.iter().find(|diagnostic| diagnostic.message.contains("imported by `files[3]`")).expect("the namespace's refusal");
    let expected = Some(position_of(&library, "\"numbr"));
    assert_eq!(site(from_namespace), (fixture.path("host/schemas/library.yaml").as_path(), Some("files[2]"), expected));
}

/// A member that fails before its `kind` is known might have been a
/// namespace, so the catalog is not prepared without it: the member's own
/// refusal is the only one, never a stale-lifecycle refusal at
/// `catalog_hash` caused by the namespace missing from the catalog.
#[test]
fn a_member_of_unknown_kind_is_the_only_refusal() {
    let cases = [
        ("schemas/run.yaml", "files[3]", "kind: namespace-schema\nnamespace: [run\n", "not one YAML mapping"),
        ("schemas/run.yaml", "files[3]", "namespace: run\n$schema:\n    count: number\n", "declares no `kind`"),
        ("schemas/run.yaml", "files[3]", "kind: namespace-schem\nnamespace: run\n", "`namespace-schem`"),
        ("schemas/gadget.yaml", "files[1]", "kind: schema-trigger\nmatch: {gadget\n", "not one YAML mapping"),
    ];
    for (path, field, text, reason) in cases {
        let fixture = Fixture::new();
        fixture.write_member("host", path, text);
        let diagnostic = refused(&fixture.load());
        assert_eq!(site(&diagnostic).0, fixture.path(&format!("host/{path}")).as_path(), "{text}: {diagnostic:#?}");
        assert_eq!(diagnostic.member.as_deref(), Some(field), "{text}");
        assert!(diagnostic.message.contains(reason), "{text}: {}", diagnostic.message);
    }
}

/// A namespace that is not UTF-8 text is refused at its manifest entry, and
/// nothing is refused on the catalog's behalf.
#[test]
fn a_namespace_that_is_not_text_is_the_only_refusal() {
    let fixture = Fixture::new();
    std::fs::write(fixture.path("host/schemas/run.yaml"), b"kind: namespace-schema\n\xff\n").unwrap();
    fixture.readdress("host");
    let diagnostic = refused(&fixture.load());
    let expected = Some(position_of(&fixture.manifest_text("host"), "./schemas/run.yaml"));
    assert_eq!(site(&diagnostic), (fixture.manifest().as_path(), Some("files[3]"), expected), "{diagnostic:#?}");
}

/// A library refused before its `kind` is known still fails each importer;
/// every importer's refusal is reported in the library and names the
/// importer, never in the importer as a problem of its own.
#[test]
fn importers_of_a_library_of_unknown_kind_are_reported_in_the_library() {
    for (text, parse_error) in [
        ("kind: schema\ntypes: [widget\n", true),
        ("types:\n    widget:\n        size: number\n", false),
        ("kind: schemax\ntypes:\n    widget:\n        size: number\n", false),
    ] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/library.yaml", text);
        let library = fixture.path("host/schemas/library.yaml");
        let all = refusals(&fixture.load());
        assert_eq!(all.len(), 3, "the library and both importers: {all:#?}");
        for diagnostic in &all {
            assert_eq!(site(diagnostic).0, library.as_path(), "{text}: {diagnostic:#?}");
            assert_eq!(diagnostic.member.as_deref(), Some("files[2]"), "{text}: {diagnostic:#?}");
        }
        for importer in ["imported by `files[0]`", "imported by `files[1]`"] {
            assert!(all.iter().any(|diagnostic| diagnostic.message.contains(importer)), "{text}: {importer}: {all:#?}");
        }
        if parse_error {
            let positions: Vec<_> = all.iter().map(|diagnostic| diagnostic.line.zip(diagnostic.column)).collect();
            assert!(positions.iter().all(|at| at.is_some() && *at == positions[0]), "one parser position: {positions:?}");
        }
    }
}

/// A member that is not UTF-8 has no text to locate in: it is reported at
/// its manifest entry, by itself and by each member importing it.
#[test]
fn an_unreadable_member_is_located_at_its_manifest_entry() {
    let fixture = Fixture::new();
    std::fs::write(fixture.path("host/schemas/library.yaml"), b"kind: schema\n\xff\n").unwrap();
    fixture.readdress("host");
    let all = refusals(&fixture.load());
    let expected = Some(position_of(&fixture.manifest_text("host"), "./schemas/library.yaml"));
    assert_eq!(all.len(), 3, "the library and both importers: {all:#?}");
    for diagnostic in &all {
        assert_eq!(site(diagnostic), (fixture.manifest().as_path(), Some("files[2]"), expected), "{diagnostic:#?}");
    }
}

/// A namespace whose bytes no longer match the manifest's digest refuses the
/// snapshot before any member is prepared, so it is the only refusal.
#[test]
fn a_namespace_that_fails_its_digest_is_the_only_refusal() {
    let fixture = Fixture::new();
    fixture.corrupt_member("host", "schemas/run.yaml", &RUN_NAMESPACE.replace("The current pass.", "The pass."));
    let diagnostic = refused(&fixture.load());
    assert_eq!(site(&diagnostic).1, Some("files[3]"), "{diagnostic:#?}");
    assert!(diagnostic.message.contains("does not match its digest"), "{}", diagnostic.message);
}

// ── manifest and artifacts ──────────────────────────────────────────────────

#[test]
fn a_host_mismatch_is_located_at_the_manifest_host() {
    let fixture = Fixture::new();
    fixture.edit_manifest(|manifest| manifest.replace("host: acme", "host: other"));
    let diagnostic = refused(&fixture.load());
    let expected = Some(position_of(&fixture.manifest_text("host"), "other"));
    assert_eq!(site(&diagnostic), (fixture.manifest().as_path(), Some("host"), expected));
}

#[test]
fn a_stale_lifecycle_artifact_is_located_at_its_catalog_hash() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/run.yaml", &RUN_NAMESPACE.replace("The current pass.", "The pass."));
    let diagnostic = refused(&fixture.load());
    let artifact = std::fs::read_to_string(fixture.path("host/schemas/globals.lifecycle.json")).unwrap();
    let (line, _) = position_of(&artifact, "\"catalog_hash\"");
    assert_eq!(diagnostic.member.as_deref(), Some("lifecycle"));
    assert_eq!(diagnostic.line, Some(line), "{diagnostic:#?}");
}

/// With `global_catalog: true` the reader refuses a `host_globals` name the
/// catalog lacks; the refusal is at that name.
#[test]
fn a_host_global_the_attached_catalog_lacks_is_located_at_the_name() {
    let fixture = Fixture::new();
    let hash = catalog_hash_of(&[("schemas/run.yaml", RUN_NAMESPACE)]);
    let artifact = lifecycle_artifact_declaring("acme", &["run", "stray"], &hash);
    assert!(artifact.contains("\"global_catalog\": true"), "the catalog is attached");
    fixture.write_member("host", "schemas/globals.lifecycle.json", &artifact);
    let diagnostic = refused(&fixture.load());
    assert!(diagnostic.message.contains("binds host global `stray`"), "{}", diagnostic.message);
    assert_eq!(diagnostic.line.zip(diagnostic.column), Some(position_of(&artifact, "\"stray\"")), "{diagnostic:#?}");
}

#[test]
fn a_bad_facts_artifact_is_located_at_the_field_its_reader_judged() {
    let stale = facts_artifact().replace("\"catalog_hash\": \"", "\"catalog_hash\": \"0");
    let entry = facts_artifact().replace("\"lazy\": false", "\"lazy\": 1");
    for (facts, needle) in [(stale.as_str(), "\"0"), (entry.as_str(), "1"), ("{}", "{}"), ("{\n  \"a\": 1,\n  \"a\": 2\n}\n", "\n}")] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/functions.facts.json", facts);
        let diagnostic = refused(&fixture.load());
        assert_eq!(diagnostic.member.as_deref(), Some("files[4]"), "{facts}");
        let expected = match needle {
            "1" => position_of(facts, "\"lazy\": 1").0,
            other => position_of(facts, other).0,
        };
        assert_eq!(diagnostic.line, Some(expected), "{facts}: {diagnostic:#?}");
    }
}

#[test]
fn a_function_catalog_signature_failure_is_located_at_the_signature() {
    let fixture = Fixture::new();
    let text = FUNCTIONS.replace("parameters(name: string)", "parameters(name: string(min(abc)))");
    fixture.write_member("host", "schemas/functions.yaml", &text);
    let diagnostic = refused(&fixture.load());
    assert_eq!(diagnostic.member.as_deref(), Some("files[4]"), "{diagnostic:#?}");
    assert_eq!(diagnostic.line, Some(position_of(&text, "min(abc)").0), "{diagnostic:#?}");
}

// ── omitted required member fields ──────────────────────────────────────────

/// A comment header, so a member's mapping does not start at its first line.
const HEADER: &str = "# A member published by the acme host.\n\n";

/// `text` behind [`HEADER`], without its top-level `key` and that key's
/// indented lines.
fn omitting(text: &str, key: &str) -> String {
    let mut kept = String::from(HEADER);
    let mut skipping = false;
    for line in text.lines() {
        if line.starts_with(&format!("{key}:")) {
            skipping = true;
            continue;
        }
        if skipping && line.starts_with(' ') {
            continue;
        }
        skipping = false;
        kept.push_str(line);
        kept.push('\n');
    }
    assert_ne!(kept, format!("{HEADER}{text}"), "`{key}` is in the member");
    kept
}

/// A member that omits a field its kind requires is refused, and the whole
/// host with it, at the mapping that lacks the field (line 3, after the
/// header), never without a position.
#[test]
fn an_omitted_required_member_field_is_located_at_its_mapping() {
    let yaml_kind = "kind: yaml-schema\ndescription: A part.\n$schema:\n    kind: literal(part; required)\n";
    let cases = [
        ("trigger without match", "schemas/gadget.yaml", "files[1]", omitting(GADGET_TRIGGER, "match"), None),
        ("namespace without namespace", "schemas/run.yaml", "files[3]", omitting(RUN_NAMESPACE, "namespace"), None),
        ("namespace without $schema", "schemas/run.yaml", "files[3]", omitting(RUN_NAMESPACE, "$schema"), None),
        ("member without kind", "schemas/run.yaml", "files[3]", omitting(RUN_NAMESPACE, "kind"), None),
        ("yaml-schema without $schema", "schemas/part.yaml", "files[5]", omitting(yaml_kind, "$schema"), Some(())),
    ];
    for (case, path, member, text, listed_as_new) in cases {
        let fixture = Fixture::new();
        match listed_as_new {
            Some(()) => fixture.install_bundle("host", &Bundle::acme().with_file(path, &text)),
            None => fixture.write_member("host", path, &text),
        }
        let diagnostic = refused(&fixture.load());
        assert_eq!(
            site(&diagnostic),
            (fixture.path(&format!("host/{path}")).as_path(), Some(member), Some((3, 1))),
            "{case}: {diagnostic:#?}\n{text}"
        );
    }
}

/// A library without `types` is refused at its mapping, and so is each
/// trigger importing it; nothing is published.
#[test]
fn a_library_without_types_is_located_at_its_mapping_for_itself_and_importers() {
    let fixture = Fixture::new();
    let text = omitting(&format!("name: parts\n{LIBRARY}"), "types");
    fixture.write_member("host", "schemas/library.yaml", &text);
    let library = fixture.path("host/schemas/library.yaml");

    let all = refusals(&fixture.load());

    assert_eq!(all.len(), 3, "the library and both importers: {all:#?}");
    for diagnostic in &all {
        assert_eq!(site(diagnostic), (library.as_path(), Some("files[2]"), Some((3, 1))), "{diagnostic:#?}");
    }
    assert!(all.iter().any(|diagnostic| !diagnostic.message.contains("imported by")), "the library's own refusal: {all:#?}");
}

/// A present `match` whose value is invalid keeps its value's position.
#[test]
fn a_present_invalid_match_is_still_located_at_its_value() {
    for (written, needle) in [("match:\n", "match:"), ("match: 5\n", "5\n"), ("match: {}\n", "{}"), ("match: [7]\n", "7]")] {
        let text = format!("{HEADER}kind: schema-trigger\n{written}");
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", &text);
        let diagnostic = refused(&fixture.load());
        let expected = match needle {
            // A bare `match:` is a null value at the end of its line.
            "match:" => (position_of(&text, needle).0, "match:".len() + 1),
            other => position_of(&text, other),
        };
        assert_eq!(site(&diagnostic).2, Some(expected), "{written:?}: {diagnostic:#?}");
    }
}

// ── input robustness of the artifact projection ─────────────────────────────

/// One array field of the lifecycle artifact (`effects`) in every shape
/// through the same projection: absent at the object that lacks it, null
/// and a wrong whole type at the field, mixed and wholly invalid elements at
/// the first bad element, and an explicit empty array loads.
#[test]
fn every_shape_of_a_collection_field_is_located_through_one_projection() {
    let original = Bundle::acme().members["schemas/globals.lifecycle.json"].clone();
    let cases: [(&str, Option<serde_json::Value>, Option<&str>); 6] = [
        ("absent", None, Some("{")),
        ("null", Some(serde_json::Value::Null), Some("\"effects\": null")),
        ("wrong type", Some(serde_json::json!(1)), Some("\"effects\": 1")),
        ("mixed elements", Some(serde_json::json!(["ok", 2])), Some("2\n")),
        ("wholly invalid", Some(serde_json::json!([1, 2])), Some("1,")),
        ("empty", Some(serde_json::json!([])), None),
    ];
    for (shape, value, needle) in cases {
        let mut artifact: serde_json::Value = serde_json::from_str(&original).unwrap();
        match value {
            Some(value) => artifact["effects"] = value,
            None => {
                artifact.as_object_mut().unwrap().remove("effects");
            }
        }
        let text = serde_json::to_string_pretty(&artifact).unwrap() + "\n";
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/globals.lifecycle.json", &text);
        let loaded = fixture.load();
        let Some(needle) = needle else {
            assert!(loaded.diagnostics.is_empty(), "{shape}: {:#?}", loaded.diagnostics);
            continue;
        };
        let diagnostic = refused(&loaded);
        let expected = match shape {
            // The element's own line in the pretty array.
            "mixed elements" | "wholly invalid" => {
                let effects = text.find("\"effects\"").unwrap();
                let offset = effects + text[effects..].find(needle).unwrap();
                let before = &text[..offset];
                (before.matches('\n').count() + 1, before.len() - before.rfind('\n').map_or(0, |at| at + 1) + 1)
            }
            _ => position_of(&text, needle),
        };
        let expected = if needle.starts_with('"') { (expected.0, expected.1 + needle.find(": ").unwrap() + 2) } else { expected };
        assert_eq!(diagnostic.line.zip(diagnostic.column), Some(expected), "{shape}: {diagnostic:#?}\n{text}");
    }
}


// ── declarations nothing uses ───────────────────────────────────────────────

/// An unused named type importing `./options.yaml`, a file beside the
/// members that the manifest does not list.
const UNUSED_IMPORT: &str = "    optional-host-settings: Options@./options.yaml\n";
/// The same unused declaration importing a remote file.
const REMOTE_IMPORT: &str = "    optional-host-settings: Options@https://example.com/options.yaml\n";
/// An unused named type whose example is the unlisted `./options.yaml`.
const UNUSED_EXAMPLE: &str = "    review-note: string(example(./options.yaml))\n";
const OPTIONS: &str = "kind: schema\ntypes:\n    Options:\n        verbose: boolean\n";
const PART_KIND: &str = "kind: yaml-schema\n$schema:\n    kind: literal(part; required)\n    name: string\n";

/// Each member kind of the `acme` bundle with `declaration` added to its
/// `types` table, as (case, member path, member field, edited text). The
/// YAML kind is an added member, `files[5]`.
fn with_unused(declaration: &str) -> [(&'static str, &'static str, &'static str, String); 4] {
    [
        ("library", "schemas/library.yaml", "files[2]", format!("{LIBRARY}{declaration}")),
        ("trigger", "schemas/gadget.yaml", "files[1]", format!("{GADGET_TRIGGER}types:\n{declaration}")),
        ("yaml kind", "schemas/part.yaml", "files[5]", format!("{PART_KIND}types:\n{declaration}")),
        ("namespace", "schemas/run.yaml", "files[3]", format!("{RUN_NAMESPACE}types:\n{declaration}")),
    ]
}

/// The `acme` bundle with `path` replaced by `text`, republished: a new YAML
/// kind is listed, and an edited namespace gets the lifecycle artifact
/// recorded against its new catalog hash, so the edit is the only change.
pub(crate) fn republished(path: &str, text: &str) -> Bundle {
    let bundle = Bundle::acme().with_file("schemas/part.yaml", PART_KIND).with_file(path, text);
    if path != "schemas/run.yaml" {
        return bundle;
    }
    bundle.with_member("schemas/globals.lifecycle.json", &lifecycle_artifact_for("acme", &[("schemas/run.yaml", text)]))
}

#[test]
fn a_bundle_with_every_member_kind_loads_unedited() {
    let fixture = Fixture::new();
    fixture.install_bundle("host", &republished("schemas/part.yaml", PART_KIND));
    fixture.write("host/schemas/options.yaml", OPTIONS);
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles[0].yaml_kinds().len(), 1);
}

/// A declared type that no export uses still faces the bundle boundary: its
/// unlisted import refuses the whole host, at the declaration that writes
/// it, in the one member that declares it. A namespace's refusal is the only one, never
/// a lifecycle `catalog_hash` refusal.
#[test]
fn an_unused_unlisted_import_refuses_the_host_at_the_reference_in_every_member_kind() {
    for (case, path, field, text) in with_unused(UNUSED_IMPORT) {
        let fixture = Fixture::new();
        fixture.install_bundle("host", &republished(path, &text));
        fixture.write("host/schemas/options.yaml", OPTIONS);
        let diagnostic = refused(&fixture.load());
        let expected = Some(declaration_of(&text, "./options.yaml"));
        assert_eq!(site(&diagnostic), (fixture.path(&format!("host/{path}")).as_path(), Some(field), expected), "{case}: {diagnostic:#?}");
        assert!(diagnostic.message.contains("options.yaml"), "{case}: {}", diagnostic.message);
    }
}

#[test]
fn an_unused_remote_import_in_a_library_refuses_the_host_at_the_reference() {
    let text = format!("{LIBRARY}{REMOTE_IMPORT}");
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/library.yaml", &text);
    let diagnostic = refused(&fixture.load());
    let expected = Some(declaration_of(&text, "https://"));
    assert_eq!(site(&diagnostic), (fixture.path("host/schemas/library.yaml").as_path(), Some("files[2]"), expected));
}

#[test]
fn an_unused_unlisted_example_refuses_the_host_at_the_reference() {
    for (case, path, field, text) in with_unused(UNUSED_EXAMPLE).into_iter().take(2) {
        let fixture = Fixture::new();
        fixture.write_member("host", path, &text);
        fixture.write("host/schemas/options.yaml", OPTIONS);
        let diagnostic = refused(&fixture.load());
        let expected = Some(declaration_of(&text, "./options.yaml"));
        assert_eq!(site(&diagnostic), (fixture.path(&format!("host/{path}")).as_path(), Some(field), expected), "{case}: {diagnostic:#?}");
    }
}

/// The control: the same unused import of a listed member loads.
#[test]
fn an_unused_import_of_a_listed_member_loads() {
    for (case, path, _, text) in with_unused(UNUSED_IMPORT) {
        let fixture = Fixture::new();
        fixture.install_bundle("host", &republished(path, &text).with_file("schemas/options.yaml", OPTIONS));
        let loaded = fixture.load();
        assert!(loaded.diagnostics.is_empty(), "{case}: {:#?}", loaded.diagnostics);
        assert_eq!(loaded.profiles.len(), 1, "{case}");
    }
}

// ── declarations inside union arms and across files ─────────────────────────

/// `gadget.yaml` with a root-union payload: both arms extend the library's
/// `widget` and declare `gadget`, so a locator that stops at the arm, or
/// searches by property name, lands on the first arm or its `$extends`.
const UNION_GADGET_TRIGGER: &str = "\
kind: schema-trigger
match:
    gadget: string(required)
$schema:
    - $extends: widget@./library.yaml
      gadget: \"gadget(required)@./library.yaml\"
    - $extends: widget@./library.yaml
      gadget: \"string(required)\"
";

/// A failure inside the second root-union arm of a trigger payload is
/// located in that arm: an unknown named type at its declaration, a
/// constraint typo at the parser's rejected token.
#[test]
fn a_failure_in_a_root_union_arm_is_located_at_that_arms_declaration() {
    for (case, edited, needle) in [
        ("unknown type", "      gadget: \"Missing(required)\"\n", "\"Missing("),
        ("constraint typo", "      gadget: \"string(requird)\"\n", "requird)"),
    ] {
        let text = UNION_GADGET_TRIGGER.replacen("      gadget: \"string(required)\"\n", edited, 1);
        assert_ne!(text, UNION_GADGET_TRIGGER, "{case}: the edit applies");
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", &text);
        let diagnostic = refused(&fixture.load());
        let expected = Some(position_of(&text, needle));
        assert_eq!(
            site(&diagnostic),
            (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), expected),
            "{case}: {diagnostic:#?}"
        );
    }
}

/// The unedited root-union trigger loads, and an object trigger's unknown
/// `$extends` type keeps its location at the `$extends` value.
#[test]
fn a_root_union_trigger_loads_and_an_object_extends_failure_keeps_its_location() {
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/gadget.yaml", UNION_GADGET_TRIGGER);
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);

    let text = "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    $extends: Missing@./library.yaml\n    gadget: gadget@./library.yaml\n";
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/gadget.yaml", text);
    let diagnostic = refused(&fixture.load());
    assert_eq!(site(&diagnostic).2, Some(position_of(text, "Missing@")), "{diagnostic:#?}");
}

/// A namespace payload failure is located at its own value: in the second
/// arm of a property union, and in the payload when a local type has the
/// same name as the failing property.
#[test]
fn a_namespace_payload_failure_keeps_its_arm_and_section() {
    let cases = [
        (
            "second union arm",
            "kind: namespace-schema\nnamespace: run\n$schema:\n    count:\n        - \"number -> The pass.\"\n        - \"Missing -> Or this.\"\n",
            "\"Missing",
        ),
        (
            "local type of the same name",
            "kind: namespace-schema\nnamespace: run\ntypes:\n    count: \"number -> A count.\"\n$schema:\n    count: Missing\n",
            "Missing\n",
        ),
    ];
    for (case, text, needle) in cases {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/run.yaml", text);
        let diagnostic = refused(&fixture.load());
        let expected = Some(position_of(text, needle));
        assert_eq!(
            site(&diagnostic),
            (fixture.path("host/schemas/run.yaml").as_path(), Some("files[3]"), expected),
            "{case}: {diagnostic:#?}"
        );
    }
}

/// A failure in a shared library type is located in the library for the
/// library's own refusal and for each importer, whose message names it: an
/// unknown type in the second arm of a property union, an unlisted
/// transitive import, and an unlisted example file.
#[test]
fn a_failure_inside_a_shared_type_is_located_in_the_library_for_every_importer() {
    let cases = [
        (
            "second union arm",
            "    gadget:\n        - \"string -> The gadget's name.\"\n        - \"Missing[] -> Or its parts.\"\n",
            "\"Missing[]",
        ),
        ("unlisted transitive import", "    gadget: Options@./options.yaml\n", "./options.yaml"),
        ("unlisted example", "    gadget: string(example(./options.yaml))\n", "./options.yaml"),
    ];
    for (case, gadget, needle) in cases {
        let text = LIBRARY.replacen("    gadget: \"string -> The gadget's name.\"\n", gadget, 1);
        assert_ne!(text, LIBRARY, "{case}: the edit applies");
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/library.yaml", &text);
        fixture.write("host/schemas/options.yaml", OPTIONS);
        let library = fixture.path("host/schemas/library.yaml");

        let all = refusals(&fixture.load());

        // An arm is a sequence item, located at itself; a reference at the
        // declaration that writes it.
        let expected = Some(if needle.starts_with('"') { position_of(&text, needle) } else { declaration_of(&text, needle) });
        let named = needle.trim_start_matches('"').trim_end_matches("[]");
        for diagnostic in &all {
            assert_eq!(site(diagnostic), (library.as_path(), Some("files[2]"), expected), "{case}: {diagnostic:#?}");
            assert!(diagnostic.message.contains(named), "{case}: {}", diagnostic.message);
        }
        assert!(all.iter().any(|diagnostic| !diagnostic.message.contains("imported by")), "{case}: the library's own: {all:#?}");
        assert!(
            all.iter().any(|diagnostic| diagnostic.message.contains("imported by `files[1]` (`schemas/gadget.yaml`)")),
            "{case}: the importer is named: {all:#?}"
        );
    }
}

/// The control: a registered YAML kind keeps its precise positions, for an
/// unknown sequence item type and a constraint typo in a nested mapping.
#[test]
fn a_yaml_kind_failure_keeps_its_precise_position() {
    let base = "kind: yaml-schema\n$schema:\n    kind: literal(part; required)\n    name: string\n    parts:\n        size: number\n";
    for (case, added, needle) in [
        ("sequence type", "    steps: \"Missing[](required)\"\n", "\"Missing[]"),
        ("nested typo", "        label: string(requird)\n", "requird)"),
    ] {
        let text = format!("{base}{added}");
        let fixture = Fixture::new();
        fixture.install_bundle("host", &Bundle::acme().with_file("schemas/part.yaml", &text));
        let diagnostic = refused(&fixture.load());
        let expected = Some(position_of(&text, needle));
        assert_eq!(site(&diagnostic), (fixture.path("host/schemas/part.yaml").as_path(), Some("files[5]"), expected), "{case}: {diagnostic:#?}");
    }
}

// ── references whose path the member's prose repeats ────────────────────────

/// Prose naming `./options.yaml` ahead of every declaration, so a locator
/// that searches the whole text for the path lands in the description.
const PROSE: &str = "description: Settings may come from ./options.yaml.\n";

/// `text` with [`PROSE`] as its description, right after its `kind` line.
fn with_prose(text: &str) -> String {
    let text = text.replacen("description: The current pass.\n", "", 1);
    let (first, rest) = text.split_once('\n').expect("a `kind` line");
    format!("{first}\n{PROSE}{rest}")
}

/// The 1-based position of the first `inner` inside the first `outer`.
fn position_within(text: &str, outer: &str, inner: &str) -> (usize, usize) {
    let (line, column) = position_of(text, outer);
    (line, column + outer.find(inner).unwrap_or_else(|| panic!("`{inner}` is in `{outer}`")))
}

/// An unused unlisted example or import is refused at the declaration that
/// writes it in every member kind, even when the member's description
/// mentions the same path first.
#[test]
fn an_unlisted_reference_repeated_in_prose_is_located_at_the_reference_in_every_member_kind() {
    for (declaration, outer) in [(UNUSED_EXAMPLE, "example(./options.yaml"), (UNUSED_IMPORT, "@./options.yaml")] {
        for (case, path, field, text) in with_unused(declaration) {
            let text = with_prose(&text);
            let fixture = Fixture::new();
            fixture.install_bundle("host", &republished(path, &text));
            fixture.write("host/schemas/options.yaml", OPTIONS);
            let diagnostic = refused(&fixture.load());
            let expected = Some(declaration_of(&text, outer));
            assert_eq!(
                site(&diagnostic),
                (fixture.path(&format!("host/{path}")).as_path(), Some(field), expected),
                "{case} {outer}: {diagnostic:#?}"
            );
        }
    }
}

/// A used shared-library type whose unlisted reference the library's prose
/// repeats is located at its declaration for the library and each importer,
/// whose message still names it.
#[test]
fn a_shared_type_reference_repeated_in_prose_is_located_at_the_reference_for_every_importer() {
    for (gadget, outer) in [
        ("    gadget: string(example(./options.yaml))\n", "example(./options.yaml"),
        ("    gadget: Options@./options.yaml\n", "@./options.yaml"),
    ] {
        let text = with_prose(&LIBRARY.replacen("    gadget: \"string -> The gadget's name.\"\n", gadget, 1));
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/library.yaml", &text);
        fixture.write("host/schemas/options.yaml", OPTIONS);
        let library = fixture.path("host/schemas/library.yaml");

        let all = refusals(&fixture.load());

        let expected = Some(declaration_of(&text, outer));
        for diagnostic in &all {
            assert_eq!(site(diagnostic), (library.as_path(), Some("files[2]"), expected), "{outer}: {diagnostic:#?}");
        }
        assert!(
            all.iter().any(|diagnostic| diagnostic.message.contains("imported by `files[1]` (`schemas/gadget.yaml`)")),
            "{outer}: the importer is named: {all:#?}"
        );
    }
}

/// A trigger payload that is a bare unlisted file reference is located at
/// its `$schema` carrier, not at the description that mentions the path.
#[test]
fn a_bare_payload_reference_repeated_in_prose_is_located_at_its_carrier() {
    let text = with_prose(&GADGET_TRIGGER.replacen("$schema:\n    gadget: gadget@./library.yaml\n", "$schema: ./options.yaml\n", 1));
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/gadget.yaml", &text);
    fixture.write("host/schemas/options.yaml", OPTIONS);
    let diagnostic = refused(&fixture.load());
    let expected = Some(position_within(&text, "$schema: ./options.yaml", "./options.yaml"));
    assert_eq!(site(&diagnostic), (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), expected), "{diagnostic:#?}");
}

// ── a YAML kind's claim ─────────────────────────────────────────────────────

/// A YAML kind whose `kind` claim goes through a named type too broad to
/// claim a finite set is refused at the `kind` declaration's value in
/// `$schema`, the node `md schema validate` reports. The controls: a direct
/// open claim is refused earlier, by the envelope, at the `$schema` payload
/// it judged; the literal claim loads
/// (`a_bundle_with_every_member_kind_loads_unedited`).
#[test]
fn a_yaml_kind_claim_that_names_no_kind_is_located_at_its_declaration() {
    let named = "kind: yaml-schema\n$schema:\n    kind: kind-token(required)\n    name: string\ntypes:\n    kind-token: string\n";
    let direct = PART_KIND.replacen("literal(part; required)", "string(required)", 1);
    for (case, text, needle) in [("named claim", named.to_string(), "kind-token(required)"), ("direct string", direct, "kind: string(required)")] {
        let fixture = Fixture::new();
        fixture.install_bundle("host", &Bundle::acme().with_file("schemas/part.yaml", &text));
        let diagnostic = refused(&fixture.load());
        let expected = Some(position_of(&text, needle));
        assert_eq!(site(&diagnostic), (fixture.path("host/schemas/part.yaml").as_path(), Some("files[5]"), expected), "{case}: {diagnostic:#?}");
    }
}

// ── references whose path a sibling declaration's description repeats ─────

/// A declaration whose description names `./options.yaml` but whose type
/// references nothing: the text after `->` is prose, so a locator that
/// searches every declaration scalar for the path lands here first.
const SIBLING_PROSE: &str = "    review-setting: 'string -> Read options from ./options.yaml.'\n";
/// The declaration after [`SIBLING_PROSE`] that imports the unlisted file.
const SIBLING_IMPORT: &str = "    review-entry: Options@./options.yaml\n";
/// The same declaration naming the unlisted file as its example.
const SIBLING_EXAMPLE: &str = "    review-entry: string(example(./options.yaml))\n";

/// The two declarations of each repaired shape, with the authored text the
/// reference must be found inside.
fn sibling_cases() -> [(String, &'static str); 2] {
    [(format!("{SIBLING_PROSE}{SIBLING_IMPORT}"), "@./options.yaml"), (format!("{SIBLING_PROSE}{SIBLING_EXAMPLE}"), "example(./options.yaml")]
}

/// An unused unlisted import or example is refused at the declaration that
/// writes it, in every member kind, even when an
/// earlier sibling's description names the same path. A namespace's refusal
/// stays the only one.
#[test]
fn an_unused_reference_after_a_sibling_description_is_located_at_the_reference_in_every_member_kind() {
    for (declarations, outer) in sibling_cases() {
        for (case, path, field, text) in with_unused(&declarations) {
            let fixture = Fixture::new();
            fixture.install_bundle("host", &republished(path, &text));
            fixture.write("host/schemas/options.yaml", OPTIONS);
            let diagnostic = refused(&fixture.load());
            let expected = Some(declaration_of(&text, outer));
            assert_eq!(
                site(&diagnostic),
                (fixture.path(&format!("host/{path}")).as_path(), Some(field), expected),
                "{case} {outer}: {diagnostic:#?}\n{text}"
            );
        }
    }
}

/// The same declarations as used `$schema` properties: in a trigger's
/// object payload and in the first arm of its root union, in a YAML kind,
/// and in a namespace payload.
#[test]
fn a_used_reference_after_a_sibling_description_is_located_at_the_reference_in_every_payload() {
    for (declarations, outer) in sibling_cases() {
        let arm = declarations.replace("\n    ", "\n      ").replacen("    review-setting", "    - review-setting", 1);
        let cases = [
            ("trigger object", "schemas/gadget.yaml", "files[1]", GADGET_TRIGGER.replacen("$schema:\n", &format!("$schema:\n{declarations}"), 1)),
            (
                "trigger root-union arm",
                "schemas/gadget.yaml",
                "files[1]",
                format!("kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n{arm}      gadget: string\n    - gadget: string\n"),
            ),
            ("yaml kind", "schemas/part.yaml", "files[5]", format!("{PART_KIND}{declarations}")),
            ("namespace", "schemas/run.yaml", "files[3]", format!("{RUN_NAMESPACE}{declarations}")),
        ];
        for (case, path, field, text) in cases {
            let fixture = Fixture::new();
            fixture.install_bundle("host", &republished(path, &text));
            fixture.write("host/schemas/options.yaml", OPTIONS);
            let diagnostic = refused(&fixture.load());
            let expected = Some(declaration_of(&text, outer));
            assert_eq!(
                site(&diagnostic),
                (fixture.path(&format!("host/{path}")).as_path(), Some(field), expected),
                "{case} {outer}: {diagnostic:#?}\n{text}"
            );
        }
    }
}

/// A used shared-library declaration whose sibling's description names the
/// same file is located at that declaration for the library's own refusal and
/// for each importer, whose message still names it: a `types` entry, and a
/// property inside the `widget` object both triggers' shapes reach.
#[test]
fn a_used_shared_reference_after_a_sibling_description_is_located_at_the_reference_for_every_importer() {
    for (declarations, outer) in sibling_cases() {
        let entry = declarations.replace("review-entry", "gadget");
        let property = declarations.replace("    ", "        ").replace("review-entry", "size");
        for (case, text) in [
            ("types entry", LIBRARY.replacen("    gadget: \"string -> The gadget's name.\"\n", &entry, 1)),
            ("object property", LIBRARY.replacen("        size: \"number -> The widget's size.\"\n", &property, 1)),
        ] {
            assert_ne!(text, LIBRARY, "{case}: the edit applies");
            let fixture = Fixture::new();
            fixture.write_member("host", "schemas/library.yaml", &text);
            fixture.write("host/schemas/options.yaml", OPTIONS);
            let library = fixture.path("host/schemas/library.yaml");

            let all = refusals(&fixture.load());

            let expected = Some(declaration_of(&text, outer));
            for diagnostic in &all {
                assert_eq!(site(diagnostic), (library.as_path(), Some("files[2]"), expected), "{case} {outer}: {diagnostic:#?}\n{text}");
            }
            assert!(
                all.iter().any(|diagnostic| diagnostic.message.contains("imported by `files[")),
                "{case} {outer}: an importer is named: {all:#?}"
            );
        }
    }
}

/// A trigger arm's `$extends` operand importing the unlisted file is
/// located at the `$extends` declaration, after a sibling description that
/// names the same file; so are a missing listed-looking file and a remote
/// import, at the declaration that writes each.
#[test]
fn a_composition_operand_and_other_reference_failures_after_a_sibling_description_are_located_at_the_reference() {
    let extends = "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    - review-setting: 'string -> Read options from ./options.yaml.'\n      $extends: Options@./options.yaml\n      gadget: string\n    - gadget: string\n".to_string();
    let remote = format!(
        "{GADGET_TRIGGER}types:\n    review-setting: 'string -> Read options from https://example.com/options.yaml.'\n    review-entry: Options@https://example.com/options.yaml\n"
    );
    let missing = format!("{GADGET_TRIGGER}types:\n    review-setting: 'string -> Read options from ./absent.yaml.'\n    review-entry: Options@./absent.yaml\n");
    for (case, text, outer) in [("$extends operand", extends, "@./options.yaml"), ("remote import", remote, "@https://"), ("missing file", missing, "@./absent.yaml")] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", &text);
        fixture.write("host/schemas/options.yaml", OPTIONS);
        let diagnostic = refused(&fixture.load());
        let expected = Some(declaration_of(&text, outer));
        assert_eq!(site(&diagnostic), (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), expected), "{case}: {diagnostic:#?}\n{text}");
    }
}

/// The control: a declaration whose example precedes its own description
/// naming the same file is located at that declaration.
#[test]
fn an_example_ahead_of_its_own_description_is_located_at_the_example() {
    let text = format!("{GADGET_TRIGGER}types:\n    review-entry: 'string(example(./options.yaml)) -> Read options from ./options.yaml.'\n");
    let fixture = Fixture::new();
    fixture.write_member("host", "schemas/gadget.yaml", &text);
    fixture.write("host/schemas/options.yaml", OPTIONS);
    let diagnostic = refused(&fixture.load());
    let expected = Some(declaration_of(&text, "example(./options.yaml"));
    assert_eq!(site(&diagnostic).2, expected, "{diagnostic:#?}");
}

// ── columns after non-ASCII text ────────────────────────────────────────────

/// The 1-based line and character column of the first `needle` in `text`,
/// with the line's text before it.
fn character_position_of(text: &str, needle: &str) -> (usize, usize, String) {
    let offset = text.find(needle).unwrap_or_else(|| panic!("`{needle}` is in the text"));
    let before = &text[..offset];
    let prefix = &before[before.rfind('\n').map_or(0, |at| at + 1)..];
    (before.matches('\n').count() + 1, prefix.chars().count() + 1, prefix.to_string())
}

fn located(diagnostic: &RegistrationDiagnostic) -> Option<(usize, usize, String)> {
    Some((diagnostic.line?, diagnostic.column?, diagnostic.line_prefix.clone()?))
}

/// A refusal after ASCII prose, `café`, or `🧪 café` on its line keeps the
/// 1-based character column, and the line's text before it for an editor
/// to count in its own encoding.
#[test]
fn a_refusal_after_non_ascii_text_keeps_its_character_column_and_line_text() {
    for prose in ["plain prose", "café", "🧪 café"] {
        let text = format!(
            "{GADGET_TRIGGER}types: {{label: 'string -> {prose}', review-entry: 'string(example(./options.yaml))'}}\n"
        );
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/gadget.yaml", &text);
        let diagnostic = refused(&fixture.load());
        assert_eq!(diagnostic.file, fixture.path("host/schemas/gadget.yaml"), "{prose}: {diagnostic:#?}");
        assert_eq!(located(&diagnostic), Some(character_position_of(&text, "'string(example(./options.yaml")), "{prose}: {diagnostic:#?}");
    }
}

/// `serde_json` reports a syntax error's column in bytes; the refusal's
/// column is still a character count.
#[test]
fn a_json_artifact_syntax_error_after_non_ascii_text_is_a_character_column() {
    let fixture = Fixture::new();
    let artifact = "{\"note\": \"🧪 café\"} trailing\n";
    fixture.write_member("host", "schemas/globals.lifecycle.json", artifact);
    let diagnostic = refused(&fixture.load());
    assert_eq!(diagnostic.member.as_deref(), Some("lifecycle"), "{diagnostic:#?}");
    assert_eq!(located(&diagnostic), Some(character_position_of(artifact, "trailing")), "{diagnostic:#?}");
}

// ── an example whose file the member also imports ──────────────────────────

/// A listed member that is no example artifact, named as an `example(...)`.
pub(crate) const BAD_EXAMPLE: &str = "\"string(example(./library.yaml))\"";

/// The bad example beside a working import of the same file, in every
/// payload that can hold one, as (case, member path, member field, edited
/// text): the import is written first, so a locator that searches the text
/// for the file's spelling lands on the import.
pub(crate) fn example_beside_an_import() -> [(&'static str, &'static str, &'static str, String); 4] {
    [
        (
            "trigger object",
            "schemas/gadget.yaml",
            "files[1]",
            format!("{GADGET_TRIGGER}    review-entry: {BAD_EXAMPLE}\n"),
        ),
        (
            "trigger root-union arm after `$extends`",
            "schemas/gadget.yaml",
            "files[1]",
            format!(
                "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    - $extends: widget@./library.yaml\n      gadget: string\n    - $extends: widget@./library.yaml\n      gadget: string\n      review-entry: {BAD_EXAMPLE}\n"
            ),
        ),
        ("yaml kind", "schemas/part.yaml", "files[5]", format!("{PART_KIND}    gadget: gadget@./library.yaml\n    review-entry: {BAD_EXAMPLE}\n")),
        ("namespace", "schemas/run.yaml", "files[3]", format!("{RUN_NAMESPACE}    label: gadget@./library.yaml\n    review-entry: {BAD_EXAMPLE}\n")),
    ]
}

/// The 1-based position of the last `needle` in `text`: the declaration a
/// case adds is written after the import it must not be confused with.
pub(crate) fn last_position_of(text: &str, needle: &str) -> (usize, usize) {
    let offset = text.rfind(needle).unwrap_or_else(|| panic!("`{needle}` is in the text"));
    let before = &text[..offset];
    (before.matches('\n').count() + 1, before.len() - before.rfind('\n').map_or(0, |at| at + 1) + 1)
}

/// An example naming a file the same payload imports successfully is
/// refused at the declaration that writes the example, in its own arm,
/// never at the working import; a namespace's refusal is the only one.
#[test]
fn an_example_beside_a_working_import_of_its_file_is_located_at_its_declaration() {
    for (case, path, field, text) in example_beside_an_import() {
        let fixture = Fixture::new();
        fixture.install_bundle("host", &republished(path, &text));
        let diagnostic = refused(&fixture.load());
        let expected = Some(last_position_of(&text, BAD_EXAMPLE));
        assert_eq!(
            site(&diagnostic),
            (fixture.path(&format!("host/{path}")).as_path(), Some(field), expected),
            "{case}: {diagnostic:#?}\n{text}"
        );
        assert!(diagnostic.message.contains("invalid example artifact"), "{case}: {}", diagnostic.message);
    }
}

/// `OPTIONS` as a listed library, the library's `widget` importing its
/// `Options` and naming the same file as the example of `last_updated`,
/// and the gadget trigger using `widget` too, so both triggers import the
/// failing declaration.
pub(crate) fn shared_example_bundle() -> (Bundle, String) {
    let library = LIBRARY.replacen(
        "        size: \"number -> The widget's size.\"\n",
        "        size: \"number -> The widget's size.\"\n        settings: Options@./options.yaml\n        last_updated: \"string(example(./options.yaml))\"\n",
        1,
    );
    let gadget = format!("{GADGET_TRIGGER}    widget: widget@./library.yaml\n");
    let bundle = Bundle::acme()
        .with_file("schemas/options.yaml", OPTIONS)
        .with_member("schemas/library.yaml", &library)
        .with_member("schemas/gadget.yaml", &gadget);
    (bundle, library)
}

/// An example a used shared-library declaration writes is refused in the
/// library, at that declaration, for each trigger that imports it, and each
/// refusal names its importer; the importers' payloads are never blamed.
#[test]
fn an_inherited_example_is_located_in_the_library_that_wrote_it_for_every_importer() {
    let (bundle, library) = shared_example_bundle();
    let fixture = Fixture::new();
    fixture.install_bundle("host", &bundle);
    let all = refusals(&fixture.load());
    let expected = Some(position_of(&library, "\"string(example(./options.yaml))\""));
    let file = fixture.path("host/schemas/library.yaml");
    for diagnostic in &all {
        assert_eq!(site(diagnostic), (file.as_path(), Some("files[2]"), expected), "{diagnostic:#?}");
        assert!(diagnostic.message.contains("invalid example artifact"), "{}", diagnostic.message);
    }
    for importer in ["imported by `files[0]` (`schemas/widget.yaml`)", "imported by `files[1]` (`schemas/gadget.yaml`)"] {
        assert!(all.iter().any(|diagnostic| diagnostic.message.contains(importer)), "{importer}: {all:#?}");
    }
}

/// The controls: a sole bad example, with no import of its file, is refused
/// at its declaration; an unused example naming a listed file passes the
/// bundle boundary without its contents being judged.
#[test]
fn a_sole_bad_example_is_located_at_its_declaration_and_an_unused_listed_example_loads() {
    let text = format!("{RUN_NAMESPACE}    review-entry: {BAD_EXAMPLE}\n");
    let fixture = Fixture::new();
    fixture.install_bundle("host", &republished("schemas/run.yaml", &text));
    let diagnostic = refused(&fixture.load());
    assert_eq!(
        site(&diagnostic),
        (fixture.path("host/schemas/run.yaml").as_path(), Some("files[3]"), Some(position_of(&text, BAD_EXAMPLE))),
        "{diagnostic:#?}"
    );

    let unused = format!("{LIBRARY}    review-note: \"string(example(./options.yaml))\"\n");
    let fixture = Fixture::new();
    fixture.install_bundle("host", &Bundle::acme().with_file("schemas/options.yaml", OPTIONS).with_member("schemas/library.yaml", &unused));
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles.len(), 1);
}

/// Two declarations of one namespace payload writing the same bad example
/// leave the writer unknown, so the refusal names the namespace file without
/// a position rather than guessing its `$schema` key or either declaration.
#[test]
fn an_example_written_by_two_namespace_declarations_is_refused_without_a_position() {
    let text = format!("{RUN_NAMESPACE}    review-entry: {BAD_EXAMPLE}\n    review-other: {BAD_EXAMPLE}\n");
    let fixture = Fixture::new();
    fixture.install_bundle("host", &republished("schemas/run.yaml", &text));
    let all = refusals(&fixture.load());
    assert_eq!(all.len(), 1, "{all:#?}");
    assert_eq!(site(&all[0]), (fixture.path("host/schemas/run.yaml").as_path(), Some("files[3]"), None), "{all:#?}");
    assert!(all[0].message.contains("invalid example artifact"), "{}", all[0].message);
}

// ── a listed file refused for the role a declaration names it in ────────────

/// `gadget.yaml` with its payload naming `target` as its whole `$schema`,
/// and as a root union's second arm: (form, edited text).
fn naming_as_document_schema(target: &str) -> [(&'static str, String); 2] {
    let head = "kind: schema-trigger\nmatch:\n    gadget: string(required)\n";
    [
        ("scalar", format!("{head}$schema: {target}\n")),
        ("second arm", format!("{head}$schema:\n    - gadget: string\n    - {target}\n")),
    ]
}

/// A trigger payload naming a trigger file (itself or another) as its
/// document schema is refused as a payload cycle, at the scalar or arm that
/// names the trigger.
#[test]
fn a_payload_naming_a_trigger_is_refused_as_a_cycle_at_the_declaration_that_names_it() {
    for target in ["./widget.yaml", "./gadget.yaml"] {
        for (form, text) in naming_as_document_schema(target) {
            let fixture = Fixture::new();
            fixture.install_bundle("host", &republished("schemas/gadget.yaml", &text));
            let diagnostic = refused(&fixture.load());
            assert_eq!(
                site(&diagnostic),
                (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), Some(position_of(&text, target))),
                "{target} {form}: {diagnostic:#?}"
            );
            assert!(diagnostic.message.contains("resolved to a schema-trigger envelope"), "{target} {form}: {}", diagnostic.message);
        }
    }
}

/// A valid member of another role (a types-only library, a YAML kind, a
/// namespace) named as a document schema is refused at the scalar or arm
/// that names it; the message names the target and its role.
#[test]
fn a_member_of_another_role_named_as_a_document_schema_is_refused_at_the_declaration() {
    for (target, role) in [
        ("./library.yaml", "it declares only `types`"),
        ("./part.yaml", "it is a `yaml-schema`"),
        ("./run.yaml", "it is a `namespace-schema`"),
    ] {
        for (form, text) in naming_as_document_schema(target) {
            let fixture = Fixture::new();
            fixture.install_bundle("host", &republished("schemas/gadget.yaml", &text));
            let diagnostic = refused(&fixture.load());
            assert_eq!(
                site(&diagnostic),
                (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), Some(position_of(&text, target))),
                "{target} {form}: {diagnostic:#?}"
            );
            let name = target.trim_start_matches("./");
            assert!(diagnostic.message.contains(name) && diagnostic.message.contains(role), "{target} {form}: {}", diagnostic.message);
        }
    }
}

/// A `Name@file` import naming a trigger file is refused at the import, in
/// the YAML kind and the namespace that write it.
#[test]
fn an_import_naming_a_trigger_is_refused_at_the_import_in_a_kind_and_a_namespace() {
    let kind = format!("{PART_KIND}    step: \"step[]@./widget.yaml\"\n");
    let namespace = RUN_NAMESPACE.replace("count: \"number -> The pass.\"", "count: count@./widget.yaml");
    for (case, path, field, text, reference) in [
        ("yaml kind", "schemas/part.yaml", "files[5]", kind, "\"step[]@./widget.yaml\""),
        ("namespace", "schemas/run.yaml", "files[3]", namespace, "count@./widget.yaml"),
    ] {
        let fixture = Fixture::new();
        fixture.install_bundle("host", &republished(path, &text));
        let all = refusals(&fixture.load());
        let expected = Some(position_of(&text, reference));
        for diagnostic in &all {
            assert_eq!(site(diagnostic), (fixture.path(&format!("host/{path}")).as_path(), Some(field), expected), "{case}: {all:#?}");
            assert!(diagnostic.message.contains("it is a `schema-trigger`"), "{case}: {}", diagnostic.message);
        }
    }
}

/// A shared library's import naming a trigger file is refused in the
/// library, at that import: the library's own check, and each trigger
/// importing the declaration, whose refusal is a payload cycle that names
/// its importer.
#[test]
fn a_shared_import_naming_a_trigger_is_refused_in_the_library_for_every_importer() {
    let library = LIBRARY.replacen(
        "        size: \"number -> The widget's size.\"\n",
        "        size: \"number -> The widget's size.\"\n        last_updated: stamp@./widget.yaml\n",
        1,
    );
    let gadget = format!("{GADGET_TRIGGER}    widget: widget@./library.yaml\n");
    let bundle = Bundle::acme().with_member("schemas/library.yaml", &library).with_member("schemas/gadget.yaml", &gadget);
    let fixture = Fixture::new();
    fixture.install_bundle("host", &bundle);
    let all = refusals(&fixture.load());
    let expected = Some(position_of(&library, "stamp@./widget.yaml"));
    let file = fixture.path("host/schemas/library.yaml");
    for diagnostic in &all {
        assert_eq!(site(diagnostic), (file.as_path(), Some("files[2]"), expected), "{diagnostic:#?}");
    }
    for importer in ["imported by `files[0]` (`schemas/widget.yaml`)", "imported by `files[1]` (`schemas/gadget.yaml`)"] {
        let diagnostic = all.iter().find(|diagnostic| diagnostic.message.contains(importer)).unwrap_or_else(|| panic!("{importer}: {all:#?}"));
        assert!(diagnostic.message.contains("resolved to a schema-trigger envelope"), "{}", diagnostic.message);
    }
    assert!(all.iter().any(|diagnostic| !diagnostic.message.contains("imported by")), "the library's own check: {all:#?}");
}

/// The controls: a YAML kind's or a namespace's scalar `$schema` is refused
/// by its envelope at the authored field before any target is read; a
/// target that does not parse keeps its own parse position.
#[test]
fn neighboring_role_and_parse_refusals_keep_their_own_positions() {
    let kind = PART_KIND.replace("$schema:\n    kind: literal(part; required)\n    name: string\n", "$schema: ./library.yaml\n");
    let namespace = RUN_NAMESPACE.replace("$schema:\n    count: \"number -> The pass.\"\n", "$schema: ./library.yaml\n");
    for (case, path, field, text) in [("yaml kind", "schemas/part.yaml", "files[5]", kind), ("namespace", "schemas/run.yaml", "files[3]", namespace)] {
        let fixture = Fixture::new();
        fixture.install_bundle("host", &republished(path, &text));
        let all = refusals(&fixture.load());
        let expected = Some(position_of(&text, "./library.yaml"));
        assert!(
            all.iter().all(|diagnostic| site(diagnostic) == (fixture.path(&format!("host/{path}")).as_path(), Some(field), expected)),
            "{case}: {all:#?}"
        );
        assert!(all.iter().all(|diagnostic| !diagnostic.message.contains("is not a valid schema definition")), "{case}: {all:#?}");
    }

    let broken = "$schema:\n    title: [\n";
    let text = "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema: ./broken.yaml\n";
    let fixture = Fixture::new();
    fixture.install_bundle("host", &republished("schemas/gadget.yaml", text).with_file("schemas/broken.yaml", broken));
    let all = refusals(&fixture.load());
    let file = fixture.path("host/schemas/broken.yaml");
    assert!(all.iter().all(|diagnostic| diagnostic.file == file && diagnostic.line == Some(3)), "{all:#?}");

    // A target that claims the trigger envelope but does not parse is
    // refused for its own content at its parse position, never turned into
    // an unlocated payload cycle.
    let broken = "kind: schema-trigger\nmatch:\n    gizmo: string(required)\n$schema:\n    title: [\n";
    let fixture = Fixture::new();
    fixture.install_bundle("host", &republished("schemas/gadget.yaml", text).with_file("schemas/broken.yaml", broken));
    let all = refusals(&fixture.load());
    let file = fixture.path("host/schemas/broken.yaml");
    assert!(!all.is_empty());
    assert!(all.iter().all(|diagnostic| diagnostic.file == file && diagnostic.line == Some(6)), "{all:#?}");
    assert!(all.iter().all(|diagnostic| !diagnostic.message.contains("schema-trigger envelope")), "{all:#?}");
}

/// A payload's `$extends` operand naming a trigger is the same payload
/// cycle, reported at the operand.
#[test]
fn an_extends_operand_naming_a_trigger_is_refused_at_the_operand() {
    let text = "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    $extends: widget@./widget.yaml\n    gadget: string\n";
    let fixture = Fixture::new();
    fixture.install_bundle("host", &republished("schemas/gadget.yaml", text));
    let diagnostic = refused(&fixture.load());
    assert_eq!(
        site(&diagnostic),
        (fixture.path("host/schemas/gadget.yaml").as_path(), Some("files[1]"), Some(position_of(text, "widget@./widget.yaml"))),
        "{diagnostic:#?}"
    );
    assert!(diagnostic.message.contains("resolved to a schema-trigger envelope"), "{}", diagnostic.message);
}

// ── which `register` entry a refusal names ──────────────────────────────────
//
// A warning names its entry by the 1-based position a person counts in the
// file, never the list offset `Registration::index` keeps: moving an entry
// down one line moves its number by one, an equivalent spelling at the same
// position keeps it, and the refusal's own file and position are untouched.

const VALID_ENTRY: &str = "../host/darkmatter-host.yaml";

/// The refusals of the `register` entries the user `config` (and, when
/// given, the working copy `repository` of a repository at `work/` with an
/// unborn `HEAD`) lists, loaded through the configuration snapshot.
fn snapshot_refusals(fixture: &Fixture, config: &str, repository: Option<&str>) -> Vec<RegistrationDiagnostic> {
    use darkmatter::config::load::{ConfigLoadMode, ConfigSnapshot};
    use darkmatter::config::registration::{load_registrations, md_owned_names};

    fixture.write("home/.darkmatter.yaml", config);
    let (context, tree) = match repository {
        Some(text) => {
            let work = fixture.path("work");
            git2::Repository::init(&work).unwrap();
            fixture.write("work/.darkmatter.yaml", text);
            let tree = sniff::filesystem::git::GitRepo::discover(&work).unwrap().expect("a repository").committed_tree().unwrap();
            (fixture.context().with_repository_root(&work), Some(tree))
        }
        None => (fixture.context(), None),
    };
    let snapshot = ConfigSnapshot::load(&context, tree.as_ref(), ConfigLoadMode::Passive).expect("the configuration loads");
    load_registrations(&snapshot, &context, md_owned_names()).diagnostics
}

/// `diagnostic` names entry `ordinal` of the `register` list in `declared_in`.
fn assert_names_entry(diagnostic: &RegistrationDiagnostic, ordinal: usize, declared_in: &Path) {
    let named = format!("(entry {ordinal} of `register` in `{}`)", declared_in.display());
    assert!(diagnostic.message.contains(&named), "expected {named}: {}", diagnostic.message);
}

#[test]
fn a_refused_entry_is_numbered_by_its_position_and_not_by_its_spelling() {
    let cases = [
        ("first", "register:\n  - ~/.acme/darkmatter-host.yaml\n", 1, (2, 5)),
        ("second", "register:\n  - ../host/darkmatter-host.yaml\n  - ~/.acme/darkmatter-host.yaml\n", 2, (3, 5)),
        ("second, respelled", "register:\n  - ../host/darkmatter-host.yaml\n  - ./.acme/darkmatter-host.yaml\n", 2, (3, 5)),
    ];
    for (case, config, ordinal, at) in cases {
        let fixture = Fixture::new();
        let all = snapshot_refusals(&fixture, config, None);
        let [diagnostic] = all.as_slice() else { panic!("{case}: one refusal: {all:#?}") };
        let declared_in = fixture.path("home/.darkmatter.yaml");
        assert_names_entry(diagnostic, ordinal, &declared_in);
        assert_eq!(site(diagnostic), (declared_in.as_path(), None, Some(at)), "{case}: {diagnostic:#?}");
        assert_eq!(diagnostic.entry.index, ordinal - 1, "{case}: the structured index stays the list offset");
    }
}

#[test]
fn a_refused_entry_is_numbered_within_the_configuration_that_lists_it() {
    let fixture = Fixture::new();
    let all = snapshot_refusals(
        &fixture,
        &format!("register:\n  - {VALID_ENTRY}\n  - ./other/darkmatter-host.yaml\n"),
        Some("register:\n  - ./.acme/darkmatter-host.yaml\n"),
    );
    let [user, repository] = all.as_slice() else { panic!("one refusal per file: {all:#?}") };
    let (user_file, repository_file) = (fixture.path("home/.darkmatter.yaml"), fixture.path("work/.darkmatter.yaml"));
    assert_names_entry(user, 2, &user_file);
    assert_eq!(site(user), (user_file.as_path(), None, Some((3, 5))), "{user:#?}");
    assert_names_entry(repository, 1, &repository_file);
    assert_eq!(site(repository), (repository_file.as_path(), None, Some((2, 5))), "{repository:#?}");
}

/// A manifest or member refusal names the entry that declared the manifest
/// while keeping its own file and position.
#[test]
fn a_manifest_or_member_refusal_names_its_declaring_entry() {
    let format_1 = "kind: darkmatter-host\nformat_version: 1\nhost: acme\nversion: 1.2.0\nidentify: ./schemas/widget.yaml\n\
                    triggers: []\nglobals: ./schemas/run.yaml\nlifecycle: ./schemas/globals.lifecycle.json\nfunctions: []\n";

    let fixture = Fixture::new();
    fixture.write("retired/darkmatter-host.yaml", format_1);
    let all = snapshot_refusals(&fixture, &format!("register:\n  - {VALID_ENTRY}\n  - ../retired/darkmatter-host.yaml\n"), None);
    let [retired] = all.as_slice() else { panic!("present retired manifest: {all:#?}") };
    assert_names_entry(retired, 2, &fixture.path("home/.darkmatter.yaml"));
    assert_eq!((retired.file.clone(), retired.line), (fixture.path("retired/darkmatter-host.yaml"), Some(2)), "{retired:#?}");

    let fixture = Fixture::new();
    fixture.install_host("other", "other");
    fixture.edit_manifest(|manifest| manifest.replace("version: 1.2.0\n", ""));
    let expected = site(&refused(&fixture.load())).0.to_path_buf();
    let all = snapshot_refusals(&fixture, &format!("register:\n  - {VALID_ENTRY}\n  - ../other/darkmatter-host.yaml\n"), None);
    let [malformed] = all.as_slice() else { panic!("malformed current manifest: {all:#?}") };
    assert_names_entry(malformed, 1, &fixture.path("home/.darkmatter.yaml"));
    assert_eq!(malformed.file, expected, "{malformed:#?}");
    assert!(malformed.line.is_some(), "{malformed:#?}");

    let fixture = Fixture::new();
    std::fs::remove_file(fixture.path("host/schemas/gadget.yaml")).unwrap();
    let owned_site = |diagnostic: &RegistrationDiagnostic| {
        let (file, member, at) = site(diagnostic);
        (file.to_path_buf(), member.map(str::to_string), at)
    };
    let expected: Vec<_> = refusals(&fixture.load()).iter().map(owned_site).collect();
    let all = snapshot_refusals(&fixture, &format!("register:\n  - {VALID_ENTRY}\n"), None);
    assert!(!all.is_empty(), "missing member");
    for diagnostic in &all {
        assert_names_entry(diagnostic, 1, &fixture.path("home/.darkmatter.yaml"));
        assert!(diagnostic.member.is_some(), "{diagnostic:#?}");
    }
    let found: Vec<_> = all.iter().map(owned_site).collect();
    assert_eq!(found, expected, "the member refusals keep their own locations");
}
