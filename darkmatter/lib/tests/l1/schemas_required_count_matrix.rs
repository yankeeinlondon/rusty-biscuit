//! Missing-required reporting must not depend on how many properties the
//! effective schema happens to declare, nor on how many of them are required.
//!
//! `jsonschema` 0.42 compiled *no* `required` validator whenever a `required`
//! array held exactly two names and its parent object also carried
//! `properties` — it assumed the `properties` compiler would emit a fused
//! properties-plus-required validator. That fusion only happened below an
//! internal 15-property threshold, and never when `additionalProperties` was a
//! schema object, so a two-name `required` on any larger or
//! `additionalProperties`-bearing object was silently enforced by nothing.
//! Upstream fixed the two shapes in 0.46.1 and 0.46.2.
//!
//! Every Darkmatter document merged against the shipped baseline is over that
//! threshold (the baseline alone declares 16 top-level properties), so the
//! defect reached every author with exactly two required properties: DMLS and
//! `md schema validate` both reported a false-clean document.
//!
//! These cases run through the ordinary `DarkmatterSchemas` baseline-merge and
//! `EffectiveSchema` validation path rather than a hand-built
//! `jsonschema::Validator`, so they pin the seam Darkmatter actually ships.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, ValidationProblem, ValidationProblemCode,
};

/// Property names used by the matrix. None collide with the shipped baseline.
const CANDIDATES: [&str; 4] = ["alpha", "bravo", "charlie", "delta"];

/// Builds a document declaring the first `count` [`CANDIDATES`] as required
/// strings and supplying none of them.
fn doc_with_omitted_required(count: usize) -> Markdown {
    let mut body = String::from("---\n$schema:\n");
    for name in &CANDIDATES[..count] {
        body.push_str(&format!("    {name}: 'string(required)'\n"));
    }
    body.push_str("title: Matrix\n---\n\nbody\n");
    body.as_str().into()
}

/// The missing-required problems of a baseline-merged validation, sorted by
/// property name.
fn missing_required(md: &Markdown, api: &DarkmatterSchemas) -> Vec<ValidationProblem> {
    let report = api.validate(md).expect("schema resolution must succeed");
    let mut missing: Vec<_> = report
        .problems
        .into_iter()
        .filter(|problem| problem.code == ValidationProblemCode::MissingRequired)
        .collect();
    missing.sort_by(|a, b| a.property.cmp(&b.property));
    missing
}

fn baseline_api() -> DarkmatterSchemas {
    DarkmatterSchemas::new()
        .with_darkmatter_baseline_json_schema()
        .expect("darkmatter baseline must attach")
}

/// The regression proper: one omitted required property must produce one
/// diagnostic, two must produce two, and so on — with the shipped baseline
/// merged in, which is what pushes the merged property count past the
/// upstream fusion threshold.
#[test]
fn every_omitted_required_property_is_reported_over_the_baseline() {
    let api = baseline_api();

    for count in 1..=CANDIDATES.len() {
        let md = doc_with_omitted_required(count);
        let missing = missing_required(&md, &api);

        let names: Vec<&str> = missing
            .iter()
            .map(|problem| {
                problem
                    .property
                    .as_deref()
                    .expect("a missing-required problem must name its property")
            })
            .collect();
        let mut expected: Vec<&str> = CANDIDATES[..count].to_vec();
        expected.sort_unstable();

        assert_eq!(
            names, expected,
            "{count} omitted required properties must each be reported",
        );
        for problem in &missing {
            let name = problem.property.as_deref().expect("property name");
            assert!(
                problem.message.contains(name),
                "message must name the missing property, got: {}",
                problem.message,
            );
        }
    }
}

/// The exactly-two case is the one upstream lost. Supplying one of the pair
/// must leave the other reported, not silence both.
#[test]
fn one_of_two_required_properties_supplied_still_reports_the_other() {
    let api = baseline_api();
    let md: Markdown = concat!(
        "---\n",
        "$schema:\n",
        "    alpha: 'string(required)'\n",
        "    bravo: 'string(required)'\n",
        "alpha: supplied\n",
        "---\n",
        "\n",
        "body\n",
    )
    .into();

    let missing = missing_required(&md, &api);
    let names: Vec<&str> = missing
        .iter()
        .filter_map(|problem| problem.property.as_deref())
        .collect();
    assert_eq!(names, vec!["bravo"]);
}

/// The 0.46.2 shape: a two-entry `required` on an object that also declares
/// `additionalProperties` as a *schema* rather than `false`. Upstream compiled
/// no `required` validator for this regardless of property count, so this case
/// deliberately stays small — only a referenced raw JSON Schema can author it,
/// since a SimplifiedSchema never emits an `additionalProperties` schema
/// object.
#[test]
fn two_required_properties_are_reported_under_a_schema_additional_properties() {
    let dir = tempfile::tempdir().expect("create temp dir");
    std::fs::write(
        dir.path().join("schema.json"),
        r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "alpha": { "type": "string" },
    "bravo": { "type": "string" }
  },
  "required": ["alpha", "bravo"],
  "additionalProperties": { "type": ["string", "number", "boolean", "null"] }
}"#,
    )
    .expect("write schema");

    let document = "---\n$schema: ./schema.json\ntitle: Matrix\n---\n\nbody\n";
    let doc_path = dir.path().join("doc.md");
    std::fs::write(&doc_path, document).expect("write doc");

    let md: Markdown = document.into();
    let md = md.with_source(ComposeSource::File(doc_path));

    let missing = missing_required(&md, &DarkmatterSchemas::new());
    let names: Vec<&str> = missing
        .iter()
        .filter_map(|problem| problem.property.as_deref())
        .collect();
    assert_eq!(names, vec!["alpha", "bravo"]);
}
