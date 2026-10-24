#![cfg(feature = "openapi")]
//! Acceptance: every supported value-constraint keyword survives a direct
//! OpenAPI import, an export, and a re-import, compared by meaning.
//!
//! The emitted-definitions route (`import --definitions-out` → assembly →
//! registration) is `schematic-gen`'s
//! `constraint_preservation::slow_definitions_out_rebuilds_every_rule_through_registration`.

#[path = "support/constraint_meaning.rs"]
mod meaning;

use schematic_define::RestApi;
use schematic_define::openapi::{ExportFormat, ExportOptions, OpenApiImport, OpenApiSource, export, serialize};

/// Components the fixture defines that every route keeps under their own name.
const COMPONENTS: &[&str] = &["Item", "Node", "Stats", "Owner", "Settings", "Extras"];

fn import(source: OpenApiSource) -> RestApi {
    OpenApiImport::new(source).api_name("ConstraintPreservation").build().expect("fixture imports").api
}

#[test]
fn the_fixture_imports_every_rule_exactly() {
    let api = import(OpenApiSource::yaml(include_str!("fixtures/constraint_preservation.yaml")));
    let rules = meaning::rule_map(&api, COMPONENTS);
    let get = |key: &str| rules.get(key).unwrap_or_else(|| panic!("no rules at {key}: {:#?}", rules.keys().collect::<Vec<_>>()));

    let limit = &api.endpoints.iter().find(|endpoint| endpoint.id == "ListItems").unwrap().params.as_ref().unwrap().query[0];
    assert_eq!(limit.constraints.as_ref().unwrap().maximum.as_ref().unwrap().as_str(), "18446744073709551616");
    assert_eq!(get("param:ListItems:query.score")["exclusiveMinimum"], "0");
    assert_eq!(get("param:ListItems:query.score")["multipleOf"], "10000000000000000001e-20");
    assert_eq!(get("param:ListItems:query.step")["multipleOf"], "1e-1");
    assert_eq!(get("param:ListItems:query.tags")["uniqueItems"], "true");
    assert!(get("param:ListItems:query.tags")["items"].contains("pattern"));
    assert_eq!(get("param:ListItems:query.order")["enum"], "1e0,-1e0");
    assert_eq!(get("param:ListItems:header.x-trace")["format"], "uuid");
    assert_eq!(get("param:ReplaceItem:path.id")["maxLength"], "64");
    assert_eq!(get("body:CreateNote")["maxLength"], "280");
    assert_eq!(get("body:CreateBatch")["maxItems"], "100");
    assert_eq!(get("form:Search:query")["minLength"], "1");
    assert_eq!(get("schema:Item/properties/display-name")["maxLength"], "100");
    assert_eq!(get("schema:Item/properties/kind")["enum"], "\"item\"");
    assert_eq!(get("schema:Item/properties/price")["exclusiveMinimum"], "0");
    assert_eq!(get("schema:Item/properties/price")["multipleOf"], "1e-2");
    assert_eq!(get("schema:Item/properties/quantity")["maximum"], "9223372036854775808e0");
    assert_eq!(get("schema:Item/properties/page_size")["maximum"], "5e2", "the alias' rules apply where it is used");
    assert_eq!(get("schema:Item/properties/labels/items")["maxLength"], "20");
    assert_eq!(get("schema:Node/properties/name")["pattern"], "^[A-Za-z]+$");
    assert_eq!(get("schema:Node/properties/children")["maxItems"], "8");
    assert_eq!(get("schema:Stats/properties/ratio")["maximum"], "1e0", "a response-only component keeps its rules");
    assert_eq!(get("schema:Item/properties/attributes/additionalProperties")["maxLength"], "3", "a map's value rule");
    assert_eq!(get("schema:Item/properties/free_value")["maxLength"], "3", "a free-form value's rule");
    assert_eq!(get("schema:Item/properties/identifier")["format"], "uuid");
    assert_eq!(get("schema:Item/properties/palette")["enum"], r#"{"primary":"red"}"#, "a map's whole-value rule");
    assert_eq!(get("schema:Item/properties/owner")["enum"], r#"{"name":"boss"}"#, "a rule beside a model reference");
    assert!(!rules.contains_key("schema:Owner"), "the referenced component is not constrained by the occurrence");
    assert_eq!(get("schema:Settings")["enum"], r#"{"version":"1"}"#, "a model root's rule");
    assert_eq!(get("schema:Extras/additionalProperties")["maxLength"], "3", "a model's extra-property rule");
    assert_eq!(get("schema:Item/properties/entries/items")["enum"], r#"{"code":"x"}"#, "an inline-object item's rule");
    assert_eq!(get("schema:Item/properties/owners/items")["enum"], r#"{"name":"boss"}"#, "a rule beside a model item reference");
}

#[test]
fn export_and_reimport_keep_every_rule_by_meaning_in_both_formats() {
    let api = import(OpenApiSource::yaml(include_str!("fixtures/constraint_preservation.yaml")));
    let original = meaning::rule_map(&api, COMPONENTS);
    let doc = export(&api, &ExportOptions::new()).expect("fixture exports");

    for format in [ExportFormat::Json, ExportFormat::Yaml] {
        let text = serialize(&doc, format).unwrap();
        for exact in ["18446744073709551616", "0.10000000000000000001", "9223372036854775808"] {
            assert!(text.contains(exact), "{format:?}: {exact} is written exactly");
        }
        assert!(!text.contains("x-schematic-exact"), "{format:?}: the exact-number extension is resolved, never written");
        let source = match format {
            ExportFormat::Json => OpenApiSource::json(text),
            ExportFormat::Yaml => OpenApiSource::yaml(text),
        };
        let reimported = import(source);
        assert_eq!(meaning::rule_map(&reimported, COMPONENTS), original, "{format:?}");

        // A second round trip is stable too.
        let again = serialize(&export(&reimported, &ExportOptions::new()).unwrap(), format).unwrap();
        let source = match format {
            ExportFormat::Json => OpenApiSource::json(again),
            ExportFormat::Yaml => OpenApiSource::yaml(again),
        };
        assert_eq!(meaning::rule_map(&import(source), COMPONENTS), original, "{format:?}: second round trip");
    }
}

#[test]
fn exported_spelling_is_openapi_3_0() {
    let api = import(OpenApiSource::yaml(include_str!("fixtures/constraint_preservation.yaml")));
    let text = serialize(&export(&api, &ExportOptions::new()).unwrap(), ExportFormat::Json).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    let price = &doc["components"]["schemas"]["Item"]["properties"]["price"];
    // A `number` schema's bounds are `f64` in OpenAPI 3.0 output (`0.0`).
    assert_eq!((price["minimum"].as_f64(), &price["exclusiveMinimum"]), (Some(0.0), &serde_json::json!(true)));
    let kind = &doc["components"]["schemas"]["Item"]["properties"]["kind"];
    assert_eq!(kind["enum"], serde_json::json!(["item"]), "`const` is lowered to a singleton `enum`");
    assert!(kind.get("const").is_none());
}
