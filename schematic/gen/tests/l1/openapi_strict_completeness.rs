//! Strict schema completeness of OpenAPI export, over each API's own
//! components (`RestApi::schemas`).
//!
//! A JSON body reference names components the API itself carries. These
//! tests assert that:
//!
//! 1. A reference that does not resolve (response, request body, or JSON
//!    form part) is an `invalid-schema-reference` error naming the API, the
//!    endpoint, and the dangling name, so no `$ref` to it is ever written.
//! 2. A grouped-module export rejects the entire module when one member API
//!    has a dangling reference, and writes nothing.
//! 3. Every shipped API resolves every reference, and every module's grouped
//!    document carries exactly its members' components.

use std::collections::BTreeSet;

use schematic_define::openapi::ExportOptions;
use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, CrossOrigin, Endpoint, FormField, RestApi, RestMethod,
    TypeRef,
};
use schematic_gen::diagnostics::Feature;
use schematic_gen::errors::GeneratorError;
use schematic_gen::export::openapi::{build_openapi_grouped, write_openapi_grouped};
use schematic_gen::validation::validate_openapi;

/// Builds a tiny `RestApi`, carrying no components, whose single endpoint
/// references the supplied JSON response type by name.
fn synthetic_api(name: &str, response_type: &str) -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = name.to_string(); fixture.description = format!("Synthetic API for {} regression test", name); fixture.base_url = "https://example.test".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_username = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "DanglingResponse".to_string(); fixture.method = RestMethod::Get; fixture.path = "/dangling".to_string(); fixture.description = String::new(); fixture.request = None; fixture.response = ApiResponse::json_type(response_type); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.env_mapping = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; fixture }
}

/// The OpenAPI validator's single diagnostic for `api`, as the generator
/// prints it.
fn render_error(api: &RestApi) -> String {
    let diagnostics = validate_openapi(&[api]);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(diagnostics[0].is_error());
    assert_eq!(diagnostics[0].feature, Feature::InvalidSchemaReference);
    diagnostics[0].to_string()
}

#[test]
fn single_api_with_missing_schema_fails_validation() {
    let api = synthetic_api("SyntheticOne", "MissingResponseType");

    let rendered = render_error(&api);
    assert!(rendered.contains("api=SyntheticOne "), "expected API name in: {rendered}");
    assert!(rendered.contains("endpoint=DanglingResponse "), "expected endpoint in: {rendered}");
    assert!(rendered.contains("`MissingResponseType`"), "expected missing name in: {rendered}");
    assert!(
        rendered.contains("feature=invalid-schema-reference output=openapi"),
        "rendered error must use the canonical diagnostic scope, got: {rendered}",
    );
}

#[test]
fn every_dangling_name_of_a_wrapped_reference_is_reported() {
    let mut api = synthetic_api("Wrapped", "Unused");
    api.endpoints[0].response = ApiResponse::json(TypeRef::KeyedMap {
        key: "crate::Key".into(),
        value: Box::new(TypeRef::Optional(Box::new(TypeRef::Array(Box::new(TypeRef::Named(
            "Deep".into(),
        )))))),
    });
    api.endpoints[0].request = Some(ApiRequest::FormData {
        fields: vec![FormField::json("metadata", TypeRef::Named("Part".into()))],
    });

    let diagnostics = validate_openapi(&[&api]);
    let messages: Vec<&str> = diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(messages[0].contains("`Deep`"), "{messages:?}");
    assert!(messages[1].contains("`Part`"), "{messages:?}");
    assert!(diagnostics.iter().all(|d| d.endpoint.as_deref() == Some("DanglingResponse")));
}

#[test]
fn grouped_module_with_missing_schema_in_one_member_fails() {
    // Two APIs sharing a module. The first carries its schema; the second
    // references a component it does not carry.
    let mut context = schematic_define::DefinitionContext::types_in("::fixture");
    let endpoint = Endpoint::get("GoodResponse", "/good").describes("Good response")
        .returns_json::<schematic_definitions::openai::Model>(&mut context).build().unwrap();
    let good = RestApi::builder(&context, "MemberA", "https://example.test")
        .describes("Good member").no_auth().endpoints([endpoint]).build().unwrap();
    let bad = synthetic_api("MemberB", "TotallyMissing");

    assert!(validate_openapi(&[&good]).is_empty(), "MemberA should validate");
    let rendered = render_error(&bad);
    assert!(rendered.contains("api=MemberB "));
    assert!(rendered.contains("`TotallyMissing`"));

    let temp = tempfile::TempDir::new().unwrap();
    let err = write_openapi_grouped(
        &[&good, &bad],
        "synthetic_group",
        &ExportOptions::new(),
        temp.path(),
    )
    .expect_err("one dangling member blocks the whole module");
    assert!(matches!(err, GeneratorError::Validation { .. }), "{err}");
    assert!(std::fs::read_dir(temp.path()).unwrap().next().is_none(), "nothing is written");
}

#[test]
fn dangling_json_ref_cannot_be_emitted() {
    // A real API loses one of its components (as if a registration were
    // removed). Export must reject it before any `$ref` is written.
    let mut api = schematic_definitions::openai::define_openai_api();
    assert!(validate_openapi(&[&api]).is_empty());
    api.schemas.retain(|component| component.name != "ListModelsResponse");

    let rendered = render_error(&api);
    assert!(
        rendered.contains("api=OpenAI endpoint=ListModels "),
        "rendered error must name the API and endpoint, got: {rendered}",
    );
    assert!(
        rendered.contains("`ListModelsResponse`"),
        "rendered error must name the dangling type, got: {rendered}",
    );
    build_openapi_grouped(&[&api], "openai", &ExportOptions::new())
        .expect_err("dangling $ref must be rejected");
}

#[test]
fn request_body_types_must_be_registered() {
    let api = { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "RequestBodyTest".to_string(); fixture.description = "Test API with missing request body type".to_string(); fixture.base_url = "https://example.test".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_username = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "Create".to_string(); fixture.method = RestMethod::Post; fixture.path = "/create".to_string(); fixture.description = String::new(); fixture.request = Some(ApiRequest::json_type("SomeBody")); fixture.response = ApiResponse::Empty; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.env_mapping = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; fixture };

    let rendered = render_error(&api);
    assert!(rendered.contains("api=RequestBodyTest endpoint=Create "), "{rendered}");
    assert!(rendered.contains("`SomeBody`"), "{rendered}");
}

#[test]
fn real_apis_still_pass_strict_completeness() {
    // Every API exposed by the generator resolves every reference in its own
    // components. If this fails, the generator cannot emit OpenAPI for that
    // API and a human needs to fix its registrations before merging.
    for api in schematic_definitions::all_apis() {
        api.validate_references()
            .unwrap_or_else(|error| panic!("{}: {error}", api.name));
        let diagnostics: Vec<_> = validate_openapi(&[&api])
            .into_iter()
            .filter(|d| d.feature == Feature::InvalidSchemaReference)
            .collect();
        assert!(diagnostics.is_empty(), "{}: {diagnostics:#?}", api.name);
    }
}

#[test]
fn real_module_documents_carry_exactly_their_components() {
    // Mirrors what the OpenAPI output does: one grouped document per module,
    // whose `components.schemas` are exactly its members' components.
    for (module_name, member_apis) in schematic_definitions::apis_by_module() {
        let refs: Vec<&RestApi> = member_apis.iter().collect();
        let document = build_openapi_grouped(&refs, &module_name, &ExportOptions::new())
            .unwrap_or_else(|e| panic!("module {module_name:?}: {e}"));
        let exported: BTreeSet<&str> = document
            .components
            .as_ref()
            .map(|components| components.schemas.keys().map(String::as_str).collect())
            .unwrap_or_default();
        let carried: BTreeSet<&str> = member_apis
            .iter()
            .flat_map(|api| api.schemas.iter().map(|component| component.name.as_str()))
            .collect();
        assert_eq!(exported, carried, "module {module_name:?}");
    }
}
