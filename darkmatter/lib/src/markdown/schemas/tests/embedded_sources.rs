//! The embedded schema source: preparation of compiled-in documents whose
//! imports resolve against a table of virtual paths, never the disk.
//!
//! Test-owned tables drive the crate-private preparation operation. The
//! shared `content_policy_fixtures` mutation table runs here against the
//! embedded source; `tests/l1/content_policy_disk_baseline.rs` runs the same
//! table against the disk resolver, so both sources are held to one set of
//! outcomes.

use std::collections::BTreeSet;

use super::content_policy_fixtures::{
    BASELINE_PATH, CATALOG_PATH, INVALID_CONTROL, OTHER_PARTIAL_PATHS, POLICY_SCHEMA_PATH, SCHEMA_MUTATIONS,
    SchemaExpect, SchemaFile, VALID_CONTROL, Verdict, baseline_with_policy,
};
use crate::markdown::Markdown;
use crate::markdown::schemas::embedded::{self, EmbeddedDocument};
use crate::markdown::schemas::catalog::{DARKMATTER_CATALOG_PATH, DARKMATTER_GLOBALS, GlobalCatalog};
use crate::markdown::schemas::resolve::{ResolvedSchema, prepare_embedded_inline_schema, prepare_embedded_schema};
use crate::markdown::schemas::{
    DarkmatterSchemas, PropertyDef, SchemaError, SchemaSourceId, SimplifiedSchema, TypeExpr,
};

const IMPORTER: &str = "schemas/main.yaml";

fn table<'a>(documents: &[(&'a str, &'a str)]) -> Vec<EmbeddedDocument<'a>> {
    documents
        .iter()
        .map(|&(path, text)| EmbeddedDocument { path, text })
        .collect()
}

/// Prepares `IMPORTER`, whose `$schema` body is `schema`, with `others`
/// registered beside it.
fn prepare(schema: &str, others: &[(&str, &str)]) -> Result<ResolvedSchema, SchemaError> {
    let main = format!("$schema:\n{schema}");
    let mut documents = vec![(IMPORTER, main.as_str())];
    documents.extend_from_slice(others);
    prepare_embedded_schema(IMPORTER, &table(&documents))
}

/// The message of a [`SchemaError::SchemaDocument`] naming `IMPORTER`.
fn importer_message(error: SchemaError) -> String {
    match error {
        SchemaError::SchemaDocument { path, message } => {
            assert_eq!(path, std::path::Path::new(IMPORTER), "{message}");
            message
        }
        other => panic!("expected a schema-document error naming the importer, got {other:?}"),
    }
}

fn property<'a>(resolved: &'a ResolvedSchema, name: &str) -> &'a PropertyDef {
    match resolved.simplified.as_ref().expect("simplified") {
        SimplifiedSchema::Single(shape) => &shape.properties[name],
        other => panic!("expected a single shape, got {other:?}"),
    }
}

#[test]
fn the_shipped_baseline_prepares_without_dependencies() {
    // The baseline is the catalog's `fm` declaration, prepared from the
    // embedded table as the catalog prepares it.
    let declaration: serde_yaml_ng::Value =
        serde_yaml_ng::from_str("fm: fm@./partials/fm.yaml").expect("declaration");
    let (resolved, _) = prepare_embedded_inline_schema(&declaration, DARKMATTER_CATALOG_PATH, embedded::SHIPPED, "")
        .expect("the shipped baseline prepares");
    assert!(resolved.imports.is_empty());
    assert!(resolved.examples.is_empty());
    assert!(resolved.referenced_files.is_empty());
    let PropertyDef::Single(fm) = property(&resolved, "fm") else {
        panic!("fm is a single atom");
    };
    let TypeExpr::InlineObject(fm) = &fm.ty else {
        panic!("fm is an object shape");
    };
    let PropertyDef::Single(policy) = &fm.properties["content_policy"] else {
        panic!("content_policy is a list atom");
    };
    assert!(policy.is_array);
    // `policy` flattens into `short_form`'s pattern arms and the closed
    // `long_form` object.
    let TypeExpr::Union(arms) = &policy.ty else {
        panic!("content_policy items are a union: {policy:?}");
    };
    let objects: Vec<_> = arms
        .iter()
        .filter_map(|arm| match &arm.ty {
            TypeExpr::InlineObject(shape) => Some(shape.properties.keys().cloned().collect::<Vec<_>>()),
            _ => None,
        })
        .collect();
    assert_eq!(objects, [["rule", "action"]]);
    assert!(arms.len() > 2, "short_form's alternatives are inlined: {}", arms.len());
}

#[test]
fn relative_imports_and_bare_local_names_resolve_inside_the_table() {
    let types = "kind: schema\ntypes:\n  label: string(min(2))\n  tag: label\n";
    let resolved = prepare(
        "  a: \"tag@./types.yaml\"\n  b: \"label@types.yaml\"\n  c: \"tag@../schemas/types.yaml\"\n",
        &[("schemas/types.yaml", types)],
    )
    .expect("prepares");
    for name in ["a", "b", "c"] {
        let PropertyDef::Single(atom) = property(&resolved, name) else {
            panic!("{name} is a single atom");
        };
        assert!(
            matches!(atom.ty, TypeExpr::Primitive(_)),
            "{name} expands to `label`: {atom:?}"
        );
    }
    assert!(resolved.imports.is_empty(), "embedded imports are not dependency edges");
}

#[test]
fn invalid_virtual_paths_are_rejected_with_the_importer_named() {
    let types = "kind: schema\ntypes:\n  label: string\n";
    for reference in [
        "../../types.yaml",
        "/schemas/types.yaml",
        "C:/schemas/types.yaml",
        "\\\\\\\\server\\\\share\\\\types.yaml",
        "https://example.com/types.yaml",
        "&schemas/types.yaml",
        "~/types.yaml",
        "*.yaml",
    ] {
        let error = prepare(&format!("  a: \"label@{reference}\"\n"), &[("types.yaml", types)])
            .expect_err(reference);
        let message = importer_message(error);
        assert!(message.contains("embedded import"), "{reference}: {message}");
    }
}

#[test]
fn a_missing_document_is_an_error_without_disk_fallback() {
    // The repository holds this file on disk, but the table does not.
    let message = importer_message(
        prepare("  a: \"policy@../content-policy/schemas/content-policy.yaml\"\n", &[])
            .expect_err("unregistered target"),
    );
    assert!(message.contains("`content-policy/schemas/content-policy.yaml` is not an embedded schema document"), "{message}");

    let message = importer_message(
        prepare_embedded_schema(IMPORTER, &[]).expect_err("unregistered document"),
    );
    assert!(message.contains("not a registered embedded schema document"), "{message}");
}

#[test]
fn a_missing_type_names_both_virtual_documents() {
    let error = prepare(
        "  a: \"nope@./types.yaml\"\n",
        &[("schemas/types.yaml", "kind: schema\ntypes:\n  label: string\n")],
    )
    .expect_err("unknown type");
    let SchemaError::UnknownNamedType { name, namespace, site } = error else {
        panic!("expected an unknown named type, got {error:?}");
    };
    assert_eq!(name, "nope");
    assert_eq!(namespace, SchemaSourceId::Embedded("schemas/types.yaml".into()));
    assert_eq!(site.source, SchemaSourceId::Embedded(IMPORTER.into()));
    assert_eq!(site.path.to_string(), "a");
}

#[test]
fn malformed_imports_cycles_and_examples_fail_preparation() {
    let message = importer_message(
        prepare("  a: \"label[](min(1)@./types.yaml\"\n", &[]).expect_err("malformed import"),
    );
    assert!(message.contains("invalid SimplifiedSchema"), "{message}");

    let cycle = prepare(
        "  a: \"one@./types.yaml\"\n",
        &[("schemas/types.yaml", "kind: schema\ntypes:\n  one: two\n  two: one\n")],
    );
    // An alias cycle never descends into a child value.
    assert!(matches!(cycle, Err(SchemaError::NonDescendingCycle { .. })), "{cycle:?}");

    let message = importer_message(
        prepare("  a: \"string(example(./a.md))\"\n", &[]).expect_err("example"),
    );
    assert!(message.contains("example"), "{message}");

    let union = prepare_embedded_schema(IMPORTER, &table(&[(IMPORTER, "$schema:\n  - a: string\n  - b: string\n")]));
    assert!(importer_message(union.expect_err("root union")).contains("root union"));
    let reference = prepare_embedded_schema(IMPORTER, &table(&[(IMPORTER, "$schema: ./other.yaml\n")]));
    assert!(importer_message(reference.expect_err("whole-file reference")).contains("whole-file reference"));
}

/// The same virtual spelling under two identities: a document importing
/// `policy` from an embedded table and one importing it from a same-named
/// disk file each see their own definition.
#[test]
fn an_embedded_namespace_is_distinct_from_a_disk_file_with_the_same_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let disk_types = dir.path().join("schemas/types.yaml");
    std::fs::create_dir_all(disk_types.parent().expect("parent")).expect("dir");
    std::fs::write(&disk_types, "kind: schema\ntypes:\n  label: number\n").expect("write");

    let embedded = prepare(
        "  a: \"label@./types.yaml\"\n",
        &[("schemas/types.yaml", "kind: schema\ntypes:\n  label: string\n")],
    )
    .expect("embedded prepares");
    let embedded_json = embedded.json_schema.clone();
    let api = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path()))
        .with_baseline_json_schema(embedded.json_schema)
        .expect("baseline");
    let check = |frontmatter: &str| {
        let markdown: Markdown = format!("---\n{frontmatter}---\nbody\n").as_str().into();
        api.validate(&markdown).expect("validates").valid
    };
    assert_eq!(embedded_json["properties"]["a"]["anyOf"][1], serde_json::json!({"type": "string"}));
    assert!(check("a: text\n"));
    assert!(!check("a: [1]\n"));
    // A document schema importing the disk file's same-named type.
    assert!(check("$schema:\n  b: \"label@./schemas/types.yaml\"\na: text\nb: 4\n"));
    assert!(!check("$schema:\n  b: \"label@./schemas/types.yaml\"\na: text\nb: text\n"));
}

// ── shared mutation table ──────────────────────────────────────────────────

/// The shipped catalog bundle's texts, in [`CATALOG_PATH`], [`BASELINE_PATH`],
/// [`OTHER_PARTIAL_PATHS`], [`POLICY_SCHEMA_PATH`] order.
struct Shipped<'a> {
    catalog: &'a str,
    baseline: &'a str,
    others: [&'a str; 4],
    policy: &'a str,
}

fn run_column(column: &str, shipped: Shipped<'_>) {
    let baseline = baseline_with_policy(shipped.baseline);
    let dir = tempfile::tempdir().expect("temp dir");
    let cells: Vec<_> = SCHEMA_MUTATIONS.iter().filter(|m| m.column == column).collect();
    assert!(!cells.is_empty(), "column `{column}` has cells");
    assert_eq!(cells[0].expect, SchemaExpect::Control, "the first `{column}` row is its control");

    let mut failures = Vec::new();
    for cell in cells {
        let (catalog, baseline, policy) = match cell.file {
            SchemaFile::Catalog => (cell.edit.apply(shipped.catalog), baseline.clone(), shipped.policy.to_string()),
            SchemaFile::Baseline => (shipped.catalog.to_string(), cell.edit.apply(&baseline), shipped.policy.to_string()),
            SchemaFile::Policy => (shipped.catalog.to_string(), baseline.clone(), cell.edit.apply(shipped.policy)),
        };
        let mut documents = vec![(CATALOG_PATH, catalog.as_str()), (BASELINE_PATH, baseline.as_str())];
        documents.extend(OTHER_PARTIAL_PATHS.into_iter().zip(shipped.others));
        documents.push((POLICY_SCHEMA_PATH, policy.as_str()));
        let outcome = GlobalCatalog::from_documents(CATALOG_PATH, &documents, DARKMATTER_GLOBALS)
            .map_err(|error| error.to_string())
            .and_then(|catalog| {
                let (baseline, definitions) = catalog.baseline().expect("the catalog declares `fm`");
                let json = crate::markdown::schemas::to_json_schema_with_definitions(&baseline, &definitions)
                    .map_err(|error| error.to_string())?;
                DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path()))
                    .with_baseline_json_schema(json)
                    .map_err(|error| error.to_string())
            })
            .and_then(|api| {
                let verdict = |frontmatter: &str| -> Result<bool, String> {
                    let markdown: Markdown = format!("---\n{frontmatter}---\nBody\n").as_str().into();
                    Ok(api.validate(&markdown).map_err(|error| error.to_string())?.valid)
                };
                Ok((
                    verdict(VALID_CONTROL)?,
                    verdict(INVALID_CONTROL)?,
                    cell.probe.map(|(frontmatter, _)| verdict(frontmatter)).transpose()?,
                ))
            });
        let actual = match &outcome {
            Err(_) => SchemaExpect::PrepareFails,
            Ok((true, false, _)) => SchemaExpect::Control,
            Ok((true, true, _)) => SchemaExpect::Weakened,
            Ok((false, _, _)) => SchemaExpect::RejectsValid,
        };
        let probe_ok = match (cell.probe, &outcome) {
            (Some((_, expected)), Ok((_, _, Some(valid)))) => (expected == Verdict::Accept) == *valid,
            (Some(_), _) => false,
            (None, _) => true,
        };
        if actual != cell.expect || !probe_ok {
            failures.push(format!(
                "{column} / {}: expected {:?} (probe {:?}), got {actual:?} from {outcome:?}",
                cell.cell, cell.expect, cell.probe
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

macro_rules! mutation_columns {
    ($($test:ident => $column:literal),+ $(,)?) => {
        $(
            #[test]
            fn $test() {
                run_column(
                    $column,
                    Shipped {
                        catalog: include_str!("../../../../../schemas/darkmatter.yaml"),
                        baseline: include_str!("../../../../../schemas/partials/fm.yaml"),
                        others: [
                            include_str!("../../../../../schemas/partials/style.yaml"),
                            include_str!("../../../../../schemas/partials/context-variables.yaml"),
                            include_str!("../../../../../schemas/partials/error.yaml"),
                            include_str!("../../../../../schemas/partials/timing.yaml"),
                        ],
                        policy: include_str!("../../../../../../content-policy/schemas/content-policy.yaml"),
                    },
                );
            }
        )+

        /// Every column in the shared table has a test above.
        #[test]
        fn every_embedded_mutation_column_has_a_test() {
            let tested: BTreeSet<&str> = [$($column),+].into_iter().collect();
            let declared: BTreeSet<&str> = SCHEMA_MUTATIONS.iter().map(|m| m.column).collect();
            assert_eq!(tested, declared);
        }
    };
}

mutation_columns! {
    embedded_baseline_envelope_cells => "$schema",
    embedded_policy_kind_cells => "kind",
    embedded_policy_types_cells => "types",
    embedded_baseline_declaration_cells => "$schema.content_policy",
    embedded_short_form_cells => "types.short_form",
    embedded_long_form_cells => "types.long_form",
    embedded_policy_definition_cells => "types.policy",
    embedded_long_form_rule_cells => "long_form.rule",
    embedded_long_form_action_cells => "long_form.action",
}
