//! A format 3 host's expression catalogs through the public `load_entries`:
//! the facts companion's input robustness matrix (each row one edit to the
//! valid `acme` companion, republished so its digest matches), the shared
//! name space of one host's catalogs and the separation of two hosts, the
//! refused `kind: mutation-schema` member, and that a catalog no manifest
//! lists supplies nothing. Every refusal names the companion or catalog it
//! is in, the 1-based line and character column the reader judged, the
//! catalog's `files` entry, and publishes no profile.

use std::path::Path;
use std::sync::Arc;

use crate::host_fixtures::*;
use darkmatter::config::registration::{
    HOST_REGISTRATION_CODE, LoadedRegistrations, RegistrationDiagnostic, bundle::load_snapshot, load_verified, md_owned_names,
};
use darkmatter::markdown::compose::expression::{HostFunctionCatalog, HostFunctions, HostHandler};
use serde_json::json;

const FACTS: &str = "schemas/functions.facts.json";
/// The `acme` bundle's expression catalog entry, which its companion's
/// refusals are reported under.
const CATALOG_MEMBER: &str = "files[4]";

/// The one refusal of a load that published nothing.
fn refused(loaded: &LoadedRegistrations) -> RegistrationDiagnostic {
    assert!(loaded.profiles.is_empty(), "nothing is published: {:?}", loaded.profiles);
    let [diagnostic] = loaded.diagnostics.as_slice() else { panic!("one refusal: {:#?}", loaded.diagnostics) };
    assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE, "{diagnostic:#?}");
    diagnostic.clone()
}

/// `diagnostic`'s file, member, and position.
fn site(diagnostic: &RegistrationDiagnostic) -> (&Path, Option<&str>, Option<(usize, usize)>) {
    (diagnostic.file.as_path(), diagnostic.member.as_deref(), diagnostic.line.zip(diagnostic.column))
}

/// The 1-based line and character column of byte `offset` in `text`.
fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |at| at + 1);
    (before.matches('\n').count() + 1, before[line_start..].chars().count() + 1)
}

/// The position of the `nth` (0-based) `needle` in `text`, plus `skip`
/// bytes.
fn nth_position(text: &str, needle: &str, nth: usize, skip: usize) -> (usize, usize) {
    let (offset, _) = text.match_indices(needle).nth(nth).unwrap_or_else(|| panic!("`{needle}` #{nth} is in {text}"));
    line_column(text, offset + skip)
}

/// The position of the value of the first `"key": ` in `text`.
fn value_of(text: &str, key: &str) -> (usize, usize) {
    let needle = format!("\"{key}\": ");
    nth_position(text, &needle, 0, needle.len())
}

/// `text` with its first `from` replaced by `to`.
fn edited(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "`{from}` is in {text}");
    text.replacen(from, to, 1)
}

/// The `acme` install with its companion republished as `facts`.
fn with_facts(facts: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write_member("host", FACTS, facts);
    fixture
}

// ── the facts companion ─────────────────────────────────────────────────────

/// The control: the companion as the host's registration writes it records
/// explicit nulls and explicit empty lists, and loads.
#[test]
fn the_written_companion_records_explicit_nulls_and_empties_and_loads() {
    let facts = facts_artifact();
    for field in [
        "\"conversion_recovery\": null",
        "\"malformed_reference\": null",
        "\"aliases\": []",
        "\"facts\": []",
        "\"source_exclusions\": []",
    ] {
        assert!(facts.contains(field), "{field}: {facts}");
    }
    let fixture = with_facts(&facts);
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let [functions] = loaded.profiles[0].functions() else { panic!("one catalog") };
    assert!(functions.functions.incomplete_functions().is_empty());
    assert_eq!(functions.facts, fixture.path(&format!("host/{FACTS}")));
}

/// A matrix row: the cell, the companion's text, where its refusal is in
/// that text, and a phrase of its reason.
type Cell = (&'static str, String, fn(&str) -> (usize, usize), &'static str);

/// One edit per cell, each refused at the field the reader judged.
#[test]
fn every_facts_cell_is_refused_where_the_reader_judged_it() {
    let base = facts_artifact();
    let cells: Vec<Cell> = vec![
        (
            "absent conversion_recovery",
            edited(&base, "\"conversion_recovery\": null,\n", ""),
            |text| value_of(text, "acme_label"),
            "has no `conversion_recovery` field (write `null` for none)",
        ),
        (
            "absent malformed_reference",
            edited(&base, "\"malformed_reference\": null,\n", ""),
            |text| value_of(text, "acme_label"),
            "has no `malformed_reference` field (write `null` for none)",
        ),
        ("absent aliases", edited(&base, "\"aliases\": [],\n", ""), |text| value_of(text, "acme_label"), "has no `aliases` field"),
        ("null aliases", edited(&base, "\"aliases\": []", "\"aliases\": null"), |text| value_of(text, "aliases"), "has `aliases: null`"),
        ("numeric aliases", edited(&base, "\"aliases\": []", "\"aliases\": 1"), |text| value_of(text, "aliases"), "a number instead of an array"),
        (
            "every alias is numeric",
            edited(&base, "\"aliases\": []", "\"aliases\": [1]"),
            |text| nth_position(text, "[1]", 0, 1),
            "has `aliases[0]` a number instead of a string",
        ),
        ("null facts", edited(&base, "\"facts\": []", "\"facts\": null"), |text| value_of(text, "facts"), "has `facts: null`"),
        (
            "absent source_exclusions",
            edited(&base, ",\n      \"source_exclusions\": []", ""),
            |text| value_of(text, "acme_label"),
            "has no `source_exclusions` field",
        ),
        (
            "empty exclusion kinds",
            edited(&base, "\"source_exclusions\": []", "\"source_exclusions\": [{\"kinds\": [], \"parameter\": \"name\"}]"),
            |text| value_of(text, "kinds"),
            "has an empty `source_exclusions[0].kinds`; an exclusion names at least one kind",
        ),
        (
            "empty functions",
            {
                let start = base.find("\"functions\": ").unwrap() + "\"functions\": ".len();
                format!("{}{{}}\n}}\n", &base[..start])
            },
            |text| value_of(text, "functions"),
            "`acme_label`: the facts artifact has no entry for it",
        ),
        (
            "stray entry",
            edited(&base, "\"functions\": {\n", "\"functions\": {\n    \"stray\": {},\n"),
            |text| value_of(text, "stray"),
            "has an entry for `stray`, which the catalog does not declare",
        ),
        (
            "a narrowing the catalog does not declare",
            edited(
                &base,
                "\"facts\": []",
                "\"facts\": [{\"kind\": \"true_implies\", \"parameter\": \"name\", \"position\": 0, \"type\": \"string\"}]",
            ),
            |text| value_of(text, "facts"),
            "records a fact for `name`, but the catalog declares none",
        ),
        (
            "an exclusion naming no declared parameter",
            edited(&base, "\"source_exclusions\": []", "\"source_exclusions\": [{\"kinds\": [\"number\"], \"parameter\": \"nope\"}]"),
            |text| value_of(text, "acme_label"),
            "its facts artifact entry does not register",
        ),
        (
            "unsupported format_version",
            edited(&base, "\"format_version\": 1", "\"format_version\": 2"),
            |text| value_of(text, "format_version"),
            "has unknown `format_version` 2; this reader understands 1",
        ),
        (
            "nested duplicate key",
            edited(&base, "\"lazy\": false", "\"lazy\": false,\n      \"lazy\": false"),
            // The parser stops at the repeated key's closing quote.
            |text| nth_position(text, "\"lazy\"", 1, "\"lazy\"".len() - 1),
            "is not valid JSON: duplicate key `lazy`",
        ),
        (
            "trailing JSON",
            format!("{base}{{}}\n"),
            |text| nth_position(text, "{}\n", 0, 0),
            "has content after its JSON value",
        ),
        (
            "a JSON5 comment",
            edited(&base, "\"lazy\": false", "// eager\n      \"lazy\": false"),
            |text| nth_position(text, "// eager", 0, 0),
            "is not valid JSON",
        ),
        (
            "a JSON5 trailing comma",
            edited(&base, "\"source_exclusions\": []", "\"source_exclusions\": [],"),
            |text| nth_position(text, "}", 0, 0),
            "is not valid JSON: trailing comma",
        ),
        (
            "a syntax error after a non-ASCII character",
            edited(&base, "\"aliases\": []", "\"aliases\": [\"überall\", ]"),
            |text| nth_position(text, "\", ]", 0, 3),
            "is not valid JSON: trailing comma",
        ),
        (
            "a field error after a non-ASCII character",
            edited(&base, "\"aliases\": []", "\"aliases\": [\"überall\", 1]"),
            |text| nth_position(text, "\", 1]", 0, 3),
            "has `aliases[1]` a number instead of a string",
        ),
    ];
    for (cell, facts, expected, needle) in cells {
        let fixture = with_facts(&facts);
        let diagnostic = refused(&fixture.load());
        let companion = fixture.path(&format!("host/{FACTS}"));
        assert_eq!(site(&diagnostic), (companion.as_path(), Some(CATALOG_MEMBER), Some(expected(&facts))), "{cell}: {diagnostic:#?}\n{facts}");
        assert!(diagnostic.message.contains(needle), "{cell}: {}", diagnostic.message);
        assert!(diagnostic.message.contains("its functions are incomplete"), "{cell}: {}", diagnostic.message);
    }
}

#[test]
fn installed_and_verified_callable_names_are_refused_at_the_authored_key_or_alias() {
    let fixture = Fixture::new();
    let context = fixture.context();
    let base = facts_artifact();
    let bundle = Bundle::acme();
    let control = fixture.load();
    assert!(control.diagnostics.is_empty(), "{:#?}", control.diagnostics);
    let [set] = control.profiles[0].functions() else { panic!("one catalog") };
    assert_eq!(set.functions.function_names(), ["acme_label"]);

    let assert_refusal = |relative: &str, expected, name: &str| {
        let file = fixture.path(&format!("host/{relative}"));
        let diagnostic = refused(&fixture.load());
        assert_eq!(site(&diagnostic), (file.as_path(), Some(CATALOG_MEMBER), Some(expected)), "{name:?}: {diagnostic:#?}");
        assert!(diagnostic.message.contains("cannot be used as an expression call name"), "{diagnostic:#?}");
        assert!(diagnostic.related.is_empty(), "{diagnostic:#?}");

        let snapshot = load_snapshot(&fixture.manifest(), &context).expect("the digests verify");
        let refusals = load_verified(snapshot, &context, md_owned_names()).expect_err("the verified inventory is refused");
        let [refusal] = refusals.as_slice() else { panic!("one refusal: {refusals:#?}") };
        assert_eq!(refusal.file, file, "{refusal:#?}");
        assert_eq!(refusal.line.zip(refusal.column), Some(expected), "{refusal:#?}");
        assert_eq!(refusal.member.as_deref(), Some(CATALOG_MEMBER), "{refusal:#?}");
        assert!(diagnostic.message.ends_with(&refusal.message), "{refusal:#?}\n{diagnostic:#?}");
    };
    for name in ["acme label", "2fa_label", "true", "", " acme_label", "acme_label "] {
        let key = json!(name).to_string();
        let catalog = edited(FUNCTIONS, "acme_label:", &format!("{key}:"));
        let facts = edited(&base, &biscuit_hash::blake3_hash(FUNCTIONS), &biscuit_hash::blake3_hash(&catalog));
        let facts = edited(&facts, "\"acme_label\":", &format!("{key}:"));
        fixture.install_bundle("host", &bundle.clone().with_member("schemas/functions.yaml", &catalog).with_member(FACTS, &facts));
        assert_refusal("schemas/functions.yaml", nth_position(&catalog, &key, 0, 0), name);
    }

    for name in ["acme label", "2fa_label", "true", "", " acme_label", "acme_label "] {
        let aliases = json!(["acme.alias", name]).to_string();
        let facts = edited(&base, "\"aliases\": []", &format!("\"aliases\": {aliases}"));
        fixture.install_bundle("host", &bundle.clone().with_member(FACTS, &facts));
        let site = nth_position(&facts, &format!("{}]", json!(name)), 0, 0);
        assert_refusal(FACTS, site, name);
    }

    let facts = edited(&base, "\"aliases\": []", "\"aliases\": [\"acme.alias\"]");
    fixture.install_bundle("host", &bundle.with_member(FACTS, &facts));
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let [set] = loaded.profiles[0].functions() else { panic!("one catalog") };
    assert_eq!(set.functions.function_names(), ["acme_label"]);
    assert!(set.functions.registration("ACME.ALIAS").is_some());
}

#[test]
fn installed_reference_errors_preserve_the_failing_signature_and_related_location() {
    let fixture = Fixture::new();
    let catalog_path = "schemas/functions.yaml";
    let file = fixture.path(&format!("host/{catalog_path}"));
    let earlier = "function(parameters(Missing: string); returns(string); category(Acme))";
    let control_signature = "function(parameters(name: number); returns(Label@this); category(Acme))";
    let source = format!("kind: expression-schema\ncategories: [Acme]\ntypes:\n    Label: string\n    acme_label:\n        - |-\n            {earlier} -> Mentions Missing, ./helpers.yaml, and acme_label.\n        - |- # Missing ./helpers.yaml acme_label\n            {control_signature}\n");
    let control_facts = facts_for(&source, &[("acme_label", &[])]);
    let bundle = Bundle::acme();
    let install = |source: &str, facts: &str| {
        fixture.install_bundle("host", &bundle.clone().with_member(catalog_path, source).with_member(FACTS, facts));
    };
    install(&source, &control_facts);
    let accepted = fixture.load();
    assert!(accepted.diagnostics.is_empty(), "{:#?}", accepted.diagnostics);
    assert_eq!(accepted.profiles.len(), 1);
    assert_eq!(accepted.profiles[0].functions()[0].functions.function_names(), ["acme_label"]);

    for (reference, reason, is_callable) in [
        ("Missing@this", "which this catalog does not declare", false),
        ("Label@./helpers.yaml", "resolves only its own helper types", false),
        ("acme_label@this", "uses the callable", true),
    ] {
        let invalid = source.replace("Label@this", reference);
        let facts = control_facts.replace(&biscuit_hash::blake3_hash(&source), &biscuit_hash::blake3_hash(&invalid));
        install(&invalid, &facts);
        let diagnostic = refused(&fixture.load());
        let start = nth_position(&invalid, "function(parameters(name: number)", 0, 0);
        assert_eq!(site(&diagnostic), (file.as_path(), Some(CATALOG_MEMBER), Some(start)), "{diagnostic:#?}");
        assert!(diagnostic.message.contains(reason), "{diagnostic:#?}");
        if is_callable {
            let [related] = diagnostic.related.as_slice() else { panic!("{diagnostic:#?}") };
            assert_eq!(related.file, file, "{diagnostic:#?}");
            assert_eq!(related.line.zip(related.column), Some((5, 5)), "{diagnostic:#?}");
        } else {
            assert!(diagnostic.related.is_empty(), "{diagnostic:#?}");
        }
    }
}

/// A catalog whose own bytes changed and were republished, but whose
/// companion was not regenerated, passes every digest and is refused at the
/// companion's `catalog_hash`; so is a companion whose recorded hash was
/// edited and re-addressed.
#[test]
fn a_stale_catalog_hash_behind_valid_digests_is_refused_at_the_hash() {
    let stale_catalog = Fixture::new();
    stale_catalog.write_member("host", "schemas/functions.yaml", &FUNCTIONS.replace("The acme label of a name.", "The label."));
    let edited_hash = with_facts(&edited(&facts_artifact(), "\"catalog_hash\": \"", "\"catalog_hash\": \"0"));
    for fixture in [stale_catalog, edited_hash] {
        let facts = std::fs::read_to_string(fixture.path(&format!("host/{FACTS}"))).unwrap();
        let diagnostic = refused(&fixture.load());
        let companion = fixture.path(&format!("host/{FACTS}"));
        assert_eq!(site(&diagnostic), (companion.as_path(), Some(CATALOG_MEMBER), Some(value_of(&facts, "catalog_hash"))), "{diagnostic:#?}");
        assert!(diagnostic.message.contains("regenerate the artifact"), "{}", diagnostic.message);
    }
}

/// With two functions, a companion missing one entry leaves only that one
/// incomplete, and the refusal is at `functions`.
#[test]
fn a_missing_entry_is_refused_at_the_functions_map() {
    let catalog = catalog_declaring(&["acme_label", "acme_count"]);
    let full = facts_for(&catalog, &[("acme_label", &[]), ("acme_count", &[])]);
    let start = full.find("    \"acme_count\": {").unwrap();
    let end = full[start..].find("    },\n").unwrap() + start + "    },\n".len();
    let facts = format!("{}{}", &full[..start], &full[end..]);
    let fixture = Fixture::new();
    fixture.install_bundle(
        "host",
        &Bundle::acme().with_member("schemas/functions.yaml", &catalog).with_member(FACTS, &facts),
    );
    let diagnostic = refused(&fixture.load());
    let companion = fixture.path(&format!("host/{FACTS}"));
    assert_eq!(site(&diagnostic), (companion.as_path(), Some(CATALOG_MEMBER), Some(value_of(&facts, "functions"))), "{diagnostic:#?}");
    assert!(diagnostic.message.contains("`acme_count`: the facts artifact has no entry for it"), "{}", diagnostic.message);
    assert!(!diagnostic.message.contains("`acme_label`"), "the other function is complete: {}", diagnostic.message);
}

// ── names across one host's catalogs ────────────────────────────────────────

/// A catalog in the acme style declaring `names`.
fn catalog_declaring(names: &[&str]) -> String {
    let mut text = String::from("kind: expression-schema\ncategories:\n    - Acme -> Ask the acme host.\ntypes:\n");
    for name in names {
        text.push_str(&format!(
            "    {name}: |-\n        function(\n            parameters(name: string);\n            returns(string);\n            category(Acme);\n        ) -> {name}.\n"
        ));
    }
    text
}

/// The companion the host's registration writes for `catalog`, each
/// function registered with its aliases.
fn facts_for(catalog: &str, functions: &[(&str, &[&str])]) -> String {
    let loaded = HostFunctionCatalog::load(catalog, "catalog.yaml").expect("the catalog loads");
    let mut builder = HostFunctions::builder(loaded);
    for (name, aliases) in functions {
        let handler: HostHandler = Arc::new(|_| Ok(json!("label")));
        builder = builder.function(*name, aliases, |registration| registration, handler);
    }
    builder.build().expect("the host registers").facts_artifact()
}

/// The `acme` bundle plus the catalogs `schemas/a.yaml` and `schemas/b.yaml`
/// (`files[5]`, `files[6]`), each `(name, aliases)` list with its companion.
fn two_catalogs(a: &[(&str, &[&str])], b: &[(&str, &[&str])]) -> Bundle {
    let mut bundle = Bundle::acme();
    for (path, functions) in [("schemas/a.yaml", a), ("schemas/b.yaml", b)] {
        let names: Vec<&str> = functions.iter().map(|(name, _)| *name).collect();
        let catalog = catalog_declaring(&names);
        bundle = bundle.with_file(path, &catalog).with_member(&companion_of(path), &facts_for(&catalog, functions));
    }
    bundle
}

#[test]
fn a_registered_catalog_with_inline_comments_supplies_its_callable_but_not_its_helpers() {
    let fixture = Fixture::new();
    let path = "schemas/labels.yaml";
    let catalog = "kind: expression-schema
categories:
    - Acme -> Look up labels.
types:
    default_label: function(parameters(void); returns(Label@this); category(Acme)) # Look up a label.
    Label: string # A label.
    Nested:
        id: string # A nested field.
    Sequence:
        - string # Text alternative.
        - number
    Pipe: string | number # Text or a number.
";
    let facts = facts_for(catalog, &[("default_label", &[])]);
    fixture.install_bundle(
        "host",
        &Bundle::acme().with_file(path, catalog).with_member(&companion_of(path), &facts),
    );
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles.len(), 1);
    let functions = loaded.profiles[0].functions().iter()
        .find(|set| set.bundle_path == path).expect("the commented catalog is installed");
    assert_eq!(functions.functions.function_names(), ["default_label"]);
    assert!(functions.functions.incomplete_functions().is_empty());

    let changed = catalog.replace("Label: string", "Label: strung");
    let error = HostFunctionCatalog::load(&changed, path).expect_err("an undeclared type");
    assert!(error.to_string().contains("which this catalog does not declare"), "{error}");
    assert_eq!(&changed[error.span().unwrap().clone()], "strung");
}

/// Where a claim is written: its bundle path and 1-based line and column.
type Claim = (String, (usize, usize));

/// The canonical name `name`'s declaration in `catalog`: its `types` value.
fn declared(bundle: &Bundle, catalog: &str, name: &str) -> Claim {
    let needle = format!("    {name}: ");
    (catalog.to_string(), nth_position(&bundle.members[catalog], &needle, 0, needle.len()))
}

/// The first alias of `function` in `catalog`'s companion.
fn aliased(bundle: &Bundle, catalog: &str, function: &str) -> Claim {
    let path = companion_of(catalog);
    let text = &bundle.members[&path];
    let from = text.find(&format!("\"{function}\": {{")).unwrap();
    let aliases = from + text[from..].find("\"aliases\": [").unwrap() + "\"aliases\": [".len();
    let first = aliases + text[aliases..].find('"').unwrap();
    (path, line_column(text, first))
}

/// A row: the case, the functions (with aliases) of `schemas/a.yaml` and
/// of `schemas/b.yaml`, the later claim that is refused, and the earlier
/// claim it is related to.
type NameCase = (&'static str, Functions, Functions, fn(&Bundle) -> Claim, fn(&Bundle) -> Claim);
type Functions = &'static [(&'static str, &'static [&'static str])];

#[test]
fn a_later_catalog_claiming_an_earlier_name_is_refused_at_its_declaration() {
    let cases: [NameCase; 4] = [
        (
            "canonical names differing in case",
            &[("shared_name", &[])],
            &[("Shared_Name", &[])],
            |bundle| declared(bundle, "schemas/b.yaml", "Shared_Name"),
            |bundle| declared(bundle, "schemas/a.yaml", "shared_name"),
        ),
        (
            "alias against alias",
            &[("alpha", &["al"])],
            &[("beta", &["AL"])],
            |bundle| aliased(bundle, "schemas/b.yaml", "beta"),
            |bundle| aliased(bundle, "schemas/a.yaml", "alpha"),
        ),
        (
            "alias against an earlier canonical name",
            &[("gamma", &[])],
            &[("delta", &["Gamma"])],
            |bundle| aliased(bundle, "schemas/b.yaml", "delta"),
            |bundle| declared(bundle, "schemas/a.yaml", "gamma"),
        ),
        (
            "canonical name against an earlier alias",
            &[("epsilon", &["zeta"])],
            &[("zeta", &[])],
            |bundle| declared(bundle, "schemas/b.yaml", "zeta"),
            |bundle| aliased(bundle, "schemas/a.yaml", "epsilon"),
        ),
    ];
    for (case, a, b, later, earlier) in cases {
        let bundle = two_catalogs(a, b);
        let ((refused_file, refused_at), (related_file, related_at)) = (later(&bundle), earlier(&bundle));
        let fixture = Fixture::new();
        fixture.install_bundle("host", &bundle);

        let diagnostic = refused(&fixture.load());

        let file = fixture.path(&format!("host/{refused_file}"));
        assert_eq!(site(&diagnostic), (file.as_path(), Some("files[6]"), Some(refused_at)), "{case}: {diagnostic:#?}");
        assert!(diagnostic.message.contains("share one namespace"), "{case}: {}", diagnostic.message);
        let [related] = diagnostic.related.as_slice() else { panic!("{case}: one related location: {diagnostic:#?}") };
        assert_eq!(related.file, fixture.path(&format!("host/{related_file}")), "{case}");
        assert_eq!((related.line, related.column), (Some(related_at.0), Some(related_at.1)), "{case}: {related:#?}");
    }
}

/// Two hosts may each publish the same canonical name and alias: each
/// host's catalogs are a name space of their own.
#[test]
fn another_host_may_publish_the_same_name_and_alias() {
    let fixture = Fixture::new();
    let catalog = catalog_declaring(&["acme_label"]);
    let aliased = facts_for(&catalog, &[("acme_label", &["label"])]);
    for (folder, host) in [("host", "acme"), ("other", "other")] {
        fixture.install_bundle(folder, &Bundle::acme_named(host).with_member(FACTS, &aliased).with_member("schemas/functions.yaml", &catalog));
    }

    let alone = fixture.load();
    let both = fixture.load_references(&["../host/darkmatter-host.yaml", "../other/darkmatter-host.yaml"]);

    assert!(both.diagnostics.is_empty(), "{:#?}", both.diagnostics);
    let hosts: Vec<&str> = both.profiles.iter().map(|profile| profile.host()).collect();
    assert_eq!(hosts, ["acme", "other"]);
    for profile in &both.profiles {
        let [functions] = profile.functions() else { panic!("one catalog") };
        assert_eq!(functions.functions.aliases("acme_label"), Some(&["label".to_string()][..]), "{}", profile.host());
    }
    assert_eq!(alone.profiles[0].revision(), both.profiles[0].revision(), "the first host is unchanged by the second");
}

/// A canonical name or a recorded alias spelled as a built-in is refused:
/// the catalog at its declaration, the alias at its entry in the companion.
#[test]
fn a_name_spelled_as_a_built_in_is_refused_where_it_is_written() {
    let catalog = FUNCTIONS.replace("acme_label", "MIN");
    let fixture = Fixture::new();
    fixture.install_bundle("host", &Bundle::acme().with_member("schemas/functions.yaml", &catalog));
    let diagnostic = refused(&fixture.load());
    let file = fixture.path("host/schemas/functions.yaml");
    assert_eq!(site(&diagnostic).0, file.as_path(), "{diagnostic:#?}");
    assert_eq!(site(&diagnostic).1, Some(CATALOG_MEMBER), "{diagnostic:#?}");
    assert_eq!(diagnostic.line, Some(nth_position(&catalog, "MIN", 0, 0).0), "at the declaration: {diagnostic:#?}");
    assert!(diagnostic.message.contains("MIN"), "{}", diagnostic.message);
    let built_in = include_str!("../../../schemas/partials/functions.yaml");
    let min = nth_position(built_in, "\n    min:", 0, "\n    ".len());
    assert_eq!(diagnostic.related.len(), 1, "{diagnostic:#?}");
    assert_eq!(diagnostic.related[0].line.zip(diagnostic.related[0].column), Some(min), "{diagnostic:#?}");

    let facts = edited(&facts_artifact(), "\"aliases\": []", "\"aliases\": [\"min\"]");
    let diagnostic = refused(&with_facts(&facts).load());
    let expected = nth_position(&facts, "\"min\"", 0, 0);
    assert_eq!(diagnostic.line.zip(diagnostic.column), Some(expected), "at the alias: {diagnostic:#?}");
    assert!(diagnostic.message.contains("min"), "{}", diagnostic.message);
    assert_eq!(diagnostic.related.len(), 1, "{diagnostic:#?}");
    assert_eq!(diagnostic.related[0].line.zip(diagnostic.related[0].column), Some(min), "{diagnostic:#?}");

    let facts = edited(&facts_artifact(), "\"aliases\": []", "\"aliases\": [\"KEBABCASE\"]");
    let diagnostic = refused(&with_facts(&facts).load());
    let kebab = nth_position(built_in, "\n    kebab_case:", 0, "\n    ".len());
    assert_eq!(diagnostic.line.zip(diagnostic.column), Some(nth_position(&facts, "\"KEBABCASE\"", 0, 0)));
    assert_eq!(diagnostic.related.len(), 1, "{diagnostic:#?}");
    assert_eq!(diagnostic.related[0].line.zip(diagnostic.related[0].column), Some(kebab), "{diagnostic:#?}");
}

#[test]
fn installed_catalog_errors_keep_their_primary_and_earlier_declarations() {
    let first = "function(parameters(x: string); returns(string); category(Acme))";
    let at = |text: &str, needle: &str| Some(nth_position(text, needle, 0, 0));
    let path = "schemas/functions.yaml";
    let mut cases = Vec::new();
    for (sequence, ty) in [(true, "string"), (false, "string"), (true, "Label@this"), (false, "Label@this")] {
        let second = format!("function(parameters(other: {ty}); returns(string); category(Acme))");
        let body = if sequence {
            format!("\n        - '{first}'\n        - '{second}'")
        } else {
            format!(" >-\n        {first} |\n        {second}")
        };
        cases.push((
            format!("kind: expression-schema\ncategories: [Acme]\ntypes:\n    Label: string\n    acme_label:{body}\n"),
            second,
            Some(first),
            "same binding contract",
        ));
    }
    for (entry, needle, earlier, reason) in [
        ("    - Acme -> Again.\n", "Acme -> Again.", Some("Acme ->"), "duplicate category"),
        ("    - ' -> Blank.'\n", "' -> Blank.'", None, "cannot be blank"),
        ("    - 'Extra ->'\n", "'Extra ->'", None, "must be followed by a description"),
        ("", "function(", None, "is not declared in"),
    ] {
        let catalog = if entry.is_empty() {
            FUNCTIONS.replacen("category(Acme)", "category(Missing)", 1)
        } else {
            FUNCTIONS.replace("types:\n", &format!("{entry}types:\n"))
        };
        cases.push((catalog, needle.to_string(), earlier, reason));
    }
    let fixture = Fixture::new();
    for (catalog, needle, earlier, reason) in cases {
        let facts = edited(&facts_artifact(), &biscuit_hash::blake3_hash(FUNCTIONS), &biscuit_hash::blake3_hash(&catalog));
        fixture.install_bundle("host", &Bundle::acme().with_member(path, &catalog).with_member(FACTS, &facts));
        let diagnostic = refused(&fixture.load());
        let file = fixture.path(&format!("host/{path}"));
        assert_eq!(site(&diagnostic), (file.as_path(), Some(CATALOG_MEMBER), at(&catalog, &needle)), "{diagnostic:#?}");
        assert!(diagnostic.message.contains(reason), "{diagnostic:#?}");
        assert_eq!(diagnostic.related.len(), usize::from(earlier.is_some()), "{diagnostic:#?}");
        if let Some(earlier) = earlier {
            assert_eq!(diagnostic.related[0].file, file);
            assert_eq!(diagnostic.related[0].line.zip(diagnostic.related[0].column), at(&catalog, earlier));
        }
    }
    fixture.install_bundle("host", &Bundle::acme());
    let control = fixture.load();
    assert_eq!(control.profiles.len(), 1);
    assert!(control.diagnostics.is_empty(), "{:#?}", control.diagnostics);
}

#[test]
fn a_duplicate_catalog_key_refuses_registration_with_both_yaml_locations() {
    let fixture = Fixture::new();
    let path = "schemas/functions.yaml";
    let catalog = format!("{FUNCTIONS}    acme_label: string\n");
    fixture.install_bundle("host", &Bundle::acme().with_member(path, &catalog));
    let diagnostic = refused(&fixture.load());
    let file = fixture.path(&format!("host/{path}"));
    let (first_line, first_column) = nth_position(&catalog, "acme_label:", 0, 0);
    assert_eq!(site(&diagnostic), (file.as_path(), Some(CATALOG_MEMBER), Some(nth_position(&catalog, "acme_label:", 1, 0))), "{diagnostic:#?}");
    assert!(diagnostic.message.contains(&format!("the key is first written at line {first_line} column {first_column}")), "{diagnostic:#?}");
}

#[test]
fn aliases_in_one_facts_file_keep_both_locations_for_each_claim_order() {
    const PATH: &str = "schemas/a.yaml";
    let catalog = catalog_declaring(&["alpha", "beta"]);
    let base = facts_for(&catalog, &[("alpha", &[]), ("beta", &[])]);
    let fixture = Fixture::new();
    for (alpha, beta, earlier_alias, later_alias) in [
        (vec![], vec!["ALPHA"], None, Some(("beta", 0))),
        (vec!["shared"], vec!["SHARED"], Some("alpha"), Some(("beta", 0))),
        (vec!["BETA"], vec![], Some("alpha"), None),
        (vec!["ALPHA"], vec![], None, Some(("alpha", 0))),
        (vec!["shared", "SHARED"], vec![], Some("alpha"), Some(("alpha", 1))),
    ] {
        let mut facts: serde_json::Value = serde_json::from_str(&base).unwrap();
        facts["functions"]["alpha"]["aliases"] = json!(alpha);
        facts["functions"]["beta"]["aliases"] = json!(beta);
        let facts = serde_json::to_string_pretty(&facts).unwrap();
        let bundle = Bundle::acme().with_file(PATH, &catalog).with_member(&companion_of(PATH), &facts);
        let earlier = earlier_alias.map_or_else(|| declared(&bundle, PATH, "alpha"), |name| aliased(&bundle, PATH, name));
        let later = match later_alias {
            Some((name, 0)) => aliased(&bundle, PATH, name),
            Some((_, 1)) => (companion_of(PATH), nth_position(&facts, "\"SHARED\"", 0, 0)),
            None => declared(&bundle, PATH, "beta"),
            _ => unreachable!(),
        };
        fixture.install_bundle("host", &bundle);
        let diagnostic = refused(&fixture.load());
        let later_file = fixture.path(&format!("host/{}", later.0));
        assert_eq!(site(&diagnostic), (later_file.as_path(), Some("files[5]"), Some(later.1)), "{diagnostic:#?}");
        assert_eq!(diagnostic.related.len(), 1, "{diagnostic:#?}");
        assert_eq!(diagnostic.related[0].file, fixture.path(&format!("host/{}", earlier.0)));
        assert_eq!(diagnostic.related[0].line.zip(diagnostic.related[0].column), Some(earlier.1), "{diagnostic:#?}");
    }
    let control = Bundle::acme().with_file(PATH, &catalog).with_member(&companion_of(PATH), &base);
    fixture.install_bundle("host", &control);
    let loaded = fixture.load();
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles.len(), 1);
}

// ── other members ───────────────────────────────────────────────────────────

const MUTATION: &str = "kind: mutation-schema
categories:
    - Files -> Update a Markdown file.
types:
    set_frontmatter: |-
        function(
            parameters(file: string, prop: string, value: unknown | null);
            returns(unknown | null);
            category(Files);
            fallible;
        ) -> Sets a frontmatter property and returns its prior value, or null.
";

#[test]
fn a_mutation_catalog_member_with_inline_comments_is_refused_at_its_kind() {
    let fixture = Fixture::new();
    let commented = format!("{MUTATION}    Label: string # A label.\n");
    fixture.install_bundle("host", &Bundle::acme().with_file("schemas/actions.yaml", &commented));

    let diagnostic = refused(&fixture.load());

    let file = fixture.path("host/schemas/actions.yaml");
    assert_eq!(site(&diagnostic), (file.as_path(), Some("files[5]"), Some((1, 7))), "at the `kind` value: {diagnostic:#?}");
    assert!(diagnostic.message.ends_with(": host-defined actions are not supported"), "{}", diagnostic.message);
}

/// An expression catalog and its companion in a schema root, or beside the
/// manifest in the bundle folder, that the manifest does not list supply no
/// function: the host publishes only what it lists, and a host listing no
/// catalog has none.
#[test]
fn a_catalog_no_manifest_lists_supplies_no_function() {
    let fixture = Fixture::new();
    let catalog = catalog_declaring(&["stray_label"]);
    let facts = facts_for(&catalog, &[("stray_label", &[])]);
    fixture.root_catalog("home/schemas", "acme");
    fixture.root_catalog("work/schemas", "acme");
    for folder in ["host/schemas", "bare/schemas"] {
        fixture.write(&format!("{folder}/stray.yaml"), &catalog);
        fixture.write(&format!("{folder}/stray.facts.json"), &facts);
    }
    // `bare` lists no catalog, though the acme companion is still on disk.
    fixture.install_bundle("bare", &Bundle::acme_named("bare").without_file("schemas/functions.yaml"));

    let loaded = fixture.load_references(&["../host/darkmatter-host.yaml", "../bare/darkmatter-host.yaml"]);

    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let [acme, bare] = loaded.profiles.as_slice() else { panic!("two profiles") };
    let names: Vec<&str> = acme.functions().iter().flat_map(|set| set.functions.function_names()).collect();
    assert_eq!(names, ["acme_label"], "only the listed catalog");
    assert!(bare.functions().is_empty(), "a host listing no catalog has none");
    for profile in [acme, bare] {
        assert!(
            !profile.candidates().iter().any(|path| path.ends_with("stray.yaml") || path.ends_with("stray.facts.json")),
            "an unlisted catalog is not even watched: {:#?}",
            profile.candidates()
        );
    }
}

#[test]
fn installed_and_verified_catalogs_preserve_alias_arms_and_conflict_locations() {
    let fixture = Fixture::new();
    let context = fixture.context();
    let catalog_path = "schemas/functions.yaml";
    let catalog_file = fixture.path("host/schemas/functions.yaml");
    let string = "function(parameters(x: string); returns(string); category(Acme))";
    let number = "function(parameters(x: number); returns(string); category(Acme))";
    let resolved = string.replace("x: string", "x: Label@this");
    let other_category = string.replace("category(Acme)", "category(Extra)");
    let missing = number.replace("returns(string)", "returns(Missing@this)");
    let catalog = |shared: &str, arms: &str| {
        format!("kind: expression-schema\ndescription: &shared '{shared}'\ncategories: [Acme, Extra]\ntypes:\n    Label: string\n    acme_label:{arms}\n")
    };
    let written = catalog(number, &format!(" ['{string}', '{number}']"));
    let base_facts = facts_for(&written, &[("acme_label", &[])]);
    let publish = |text: &str| {
        let facts = edited(&base_facts, &biscuit_hash::blake3_hash(&written), &biscuit_hash::blake3_hash(text));
        fixture.install_bundle("host", &Bundle::acme().with_member(catalog_path, text).with_member(FACTS, &facts));
    };
    let verify = || {
        let snapshot = load_snapshot(&fixture.manifest(), &context).expect("the member digests match");
        load_verified(snapshot, &context, md_owned_names())
    };
    for text in [
        written.clone(),
        catalog(number, &format!("\n        - &string |- # The original arm.\n            {string}\n        - *shared")),
    ] {
        publish(&text);
        let loaded = fixture.load();
        assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
        let [profile] = loaded.profiles.as_slice() else { panic!("{text}") };
        let [functions] = profile.functions() else { panic!("{text}") };
        assert_eq!(functions.functions.function_names(), ["acme_label"]);
        let registration = functions.functions.registration("acme_label").unwrap();
        assert_eq!(registration.overloads().len(), 2, "{text}");
        assert!(functions.functions.incomplete_functions().is_empty(), "{text}");
        assert!(verify().is_ok(), "{text}");
    }

    for (text, primary, earlier, reason) in [
        (catalog(number, &format!("\n        - &string '{string}'\n        - *string")), "*string", Some(string), "same binding contract"),
        (catalog(string, &format!(" [*shared, '{resolved}']")), resolved.as_str(), Some("*shared"), "same binding contract"),
        (catalog(number, &format!(" [*shared, '{other_category}']")), &format!("'{other_category}'"), Some("*shared"), "different categories"),
        (catalog(&missing, &format!(" ['{string}', *shared]")), "*shared", None, "which this catalog does not declare"),
    ] {
        publish(&text);
        let diagnostic = refused(&fixture.load());
        let arms = text.find("    acme_label:").unwrap();
        let position = |needle: &str| line_column(&text, arms + text[arms..].find(needle).unwrap());
        assert_eq!(site(&diagnostic), (catalog_file.as_path(), Some(CATALOG_MEMBER), Some(position(primary))), "{diagnostic:#?}");
        assert!(diagnostic.message.contains(reason), "{diagnostic:#?}");
        let refusals = verify().expect_err("semantic refusal despite matching digests");
        let [verified] = refusals.as_slice() else { panic!("{refusals:#?}") };
        assert_eq!(verified.file, catalog_file);
        assert_eq!(verified.line.zip(verified.column), Some(position(primary)));
        assert!(verified.message.contains(reason), "{verified:#?}");
        for notes in [&diagnostic.related, &verified.related] {
            assert_eq!(notes.len(), usize::from(earlier.is_some()), "{notes:#?}");
            if let Some(earlier) = earlier {
                assert_eq!(notes[0].file, catalog_file);
                assert_eq!(notes[0].line.zip(notes[0].column), Some(position(earlier)));
            }
        }
    }
}
