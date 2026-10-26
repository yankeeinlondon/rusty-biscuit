//! Input matrix 2 of `2026-10-18-schematic-fidelity-polish` (contracts.md,
//! "Input matrices (frozen columns)"): the standard OpenAPI fields the
//! reconciliation reads, each cell run through the public importer from
//! JSON text and from YAML text.
//!
//! The base is the exported control document augmented once: `GetItem`'s
//! path item gains an inherited `limit` and an overridden `verbose`, and its
//! own `verbose` becomes a `$ref` to `components.parameters`. Each cell then
//! mutates one place. A cell whose asserted outcome is a rule adopted after
//! contracts.md carries an `adopted` note; every other cell asserts the
//! contract.

#![cfg(feature = "openapi")]

mod matrix_support;

use matrix_support::*;
use schematic_define::openapi::OpenApiImportResult;
use schematic_define::{ApiRequest, AuthStrategy, EndpointAuth, FormFieldKind, ParamDef, ParamStyle, QueryParamType, RestApi};
use serde_json::{Value, json};

const PATH_ITEM: &str = "/paths/~1items~1{item_id}";
const GET: &str = "/paths/~1items~1{item_id}/get";
const UPLOAD_OP: &str = "/paths/~1uploads/post";
const MULTIPART: &str = "/paths/~1uploads/post/requestBody/content/multipart~1form-data";

fn at(base: &str, rest: &str) -> String {
    format!("{base}{rest}")
}

fn loc(pointer: &str) -> String {
    format!("#{pointer}")
}

/// The control document plus `GetItem`'s path-item parameters and a
/// `components.parameters` reference; the one augmentation every cell
/// shares.
fn base() -> Value {
    let mut document = control_document();
    let verbose = document.pointer(&at(GET, "/parameters/1")).unwrap().clone();
    assert_eq!(verbose["name"], "verbose");
    mutate(&mut document, "/components/parameters", Some(json!({"Verbose": verbose})));
    mutate(&mut document, &at(GET, "/parameters/1"), Some(json!({"$ref": "#/components/parameters/Verbose"})));
    let path_item = document.pointer_mut(&at(PATH_ITEM, "/parameters")).unwrap().as_array_mut().unwrap();
    path_item.push(json!({"name": "limit", "in": "query", "schema": {"type": "integer"}}));
    path_item.push(json!({"name": "verbose", "in": "query", "description": "inherited", "schema": {"type": "string"}}));
    document
}

fn query(api: &RestApi) -> Vec<ParamDef> {
    endpoint(api, "GetItem").params.as_ref().map(|params| params.query.clone()).unwrap_or_default()
}

fn query_names(api: &RestApi) -> Vec<String> {
    query(api).into_iter().map(|param| param.name).collect()
}

fn param<'a>(params: &'a [ParamDef], name: &str) -> Option<&'a ParamDef> {
    params.iter().find(|param| param.name == name)
}

type Check = Box<dyn Fn(&OpenApiImportResult) -> Result<(), String>>;

fn check(what: &'static str, holds: impl Fn(&RestApi) -> bool + 'static) -> Check {
    Box::new(move |result| if holds(&result.api) { Ok(()) } else { Err(format!("check failed: {what}")) })
}

fn cell(name: &'static str, pointer: String, value: Option<Value>, expect: Expect) -> Cell {
    Cell::new(name, pointer, value, expect)
}

fn e(name: &'static str, pointer: String, value: Option<Value>) -> Cell {
    cell(name, pointer, value, Expect::BuildError)
}

/// The control query: the inherited `limit`, the operation's `$ref`
/// `verbose` overriding the path item's in place, then `api_token`.
fn control_query(api: &RestApi) -> bool {
    let params = query(api);
    query_names(api) == ["limit", "verbose", "api_token"]
        && param(&params, "limit").is_some_and(|limit| limit.param_type == QueryParamType::Integer && !limit.required)
        && param(&params, "verbose").is_some_and(|verbose| verbose.param_type == QueryParamType::Boolean && verbose.description.is_none())
}

fn parameter_cells() -> Vec<Cell> {
    let path_params = || at(PATH_ITEM, "/parameters");
    let op_params = || at(GET, "/parameters");
    let limit = |rest: &str| at(PATH_ITEM, &format!("/parameters/1{rest}"));
    vec![
        cell("path-item parameters absent", path_params(), None, clean(&[]))
            .check(check("nothing inherited", |api| query_names(api) == ["api_token", "verbose"])),
        e("path-item parameters null", path_params(), Some(Value::Null)),
        e("path-item parameters wrong type", path_params(), Some(json!({}))),
        e("path-item parameters wrong element", limit(""), Some(json!(5))),
        e("path-item parameters all wrong elements", path_params(), Some(json!([5, "x"]))),
        cell("path-item parameters empty", path_params(), Some(json!([])), clean(&[]))
            .check(check("nothing inherited", |api| query_names(api) == ["api_token", "verbose"])),
        // The control's `params.query.api_token` detail then names no
        // standard parameter: the extension's own W, not this column's.
        cell("operation parameters absent", op_params(), None, warn(loc(&at(GET, "/x-schematic/params/query/api_token")), &["op.params"]))
            .check(check("inherits the path item's", inherits_path_item)),
        e("operation parameters null", op_params(), Some(Value::Null)),
        e("operation parameters wrong type", op_params(), Some(json!({}))),
        e("operation parameters wrong element", at(GET, "/parameters/0"), Some(json!(5))),
        cell("operation parameters empty", op_params(), Some(json!([])), warn(loc(&at(GET, "/x-schematic/params/query/api_token")), &["op.params"]))
            .check(check("inherits the path item's", inherits_path_item)),
        e("parameter name absent", limit("/name"), None),
        e("parameter name null", limit("/name"), Some(Value::Null)),
        e("parameter name wrong type", limit("/name"), Some(json!(5))),
        e("parameter name empty", limit("/name"), Some(json!(""))),
        e("parameter in absent", limit("/in"), None),
        e("parameter in null", limit("/in"), Some(Value::Null)),
        e("parameter in unknown", limit("/in"), Some(json!("body"))),
        e("parameter in wrong type", limit("/in"), Some(json!(5))),
        cell("parameter required absent is false", limit("/required"), Some(json!(false)), clean(&[])).check(check("limit optional", |api| {
            param(&query(api), "limit").is_some_and(|limit| !limit.required)
        })),
        cell("path parameter required absent", at(PATH_ITEM, "/parameters/0/required"), None, path_required()),
        cell("path parameter required false", at(PATH_ITEM, "/parameters/0/required"), Some(json!(false)), path_required()),
        e("parameter required null", limit("/required"), Some(Value::Null)),
        e("parameter required wrong type", limit("/required"), Some(json!("yes"))),
        cell("parameter $ref empty", at(GET, "/parameters/1/$ref"), Some(json!("")), Expect::ErrorDiagnostic { at: loc(&at(GET, "/parameters/1")), warns: vec![] }),
        e("parameter $ref null", at(GET, "/parameters/1/$ref"), Some(Value::Null)),
        cell(
            "parameter $ref unresolved",
            at(GET, "/parameters/1/$ref"),
            Some(json!("#/components/parameters/Missing")),
            Expect::ErrorDiagnostic { at: loc(&at(GET, "/parameters/1")), warns: vec![] },
        ),
        cell(
            "parameter $ref remote",
            at(GET, "/parameters/1/$ref"),
            Some(json!("https://example.com/params.json#/Verbose")),
            Expect::ErrorDiagnostic { at: loc(&at(GET, "/parameters/1")), warns: vec![] },
        ),
        cell("parameter $ref inline", at(GET, "/parameters/1"), Some(json!({"name": "verbose", "in": "query", "schema": {"type": "boolean"}})), clean(&[]))
            .check(check("inline override", control_query)),
        cell("parameter schema type absent", limit("/schema"), Some(json!({})), clean(&[]))
            .check(check("limit is untyped JSON", |api| param(&query(api), "limit").is_some_and(|limit| limit.param_type == QueryParamType::Json)))
            .adopted("tests/import_normalization.rs pins a type-less schema as JSON, superseding contracts.md's `String`"),
        cell("parameter style and explode absent", at(GET, "/parameters/0"), Some(json!({"name": "api_token", "in": "query", "schema": {"type": "string"}})), clean(&[]))
            .check(check("query defaults: form, exploded", |api| {
                param(&query(api), "api_token").is_some_and(|token| token.style == ParamStyle::Form && token.explode && token.secret)
            })),
        cell("parameter deprecated", limit("/deprecated"), Some(json!(true)), clean(&[]))
            .check(check("limit deprecated", |api| param(&query(api), "limit").is_some_and(|limit| limit.deprecated))),
        cell("parameter description empty", limit("/description"), Some(json!("")), clean(&[]))
            .check(check("empty description kept", |api| param(&query(api), "limit").is_some_and(|limit| limit.description.as_deref() == Some("")))),
    ]
}

/// `in: path` without `required: true`: ERROR, and the permissive import
/// still succeeds.
fn path_required() -> Expect {
    Expect::ErrorDiagnostic { at: loc(&at(PATH_ITEM, "/parameters/0/required")), warns: vec![] }
}

fn inherits_path_item(api: &RestApi) -> bool {
    let params = query(api);
    query_names(api) == ["limit", "verbose"]
        && param(&params, "verbose").is_some_and(|verbose| verbose.param_type == QueryParamType::String && verbose.description.as_deref() == Some("inherited"))
}

fn all_parts_optional(api: &RestApi) -> bool {
    matches!(&endpoint(api, "Upload").request, Some(ApiRequest::FormData { fields }) if fields.iter().all(|field| !field.required))
}

/// A multipart body with no parts: the schema WARN, one per `encoding`
/// entry, and the extension's `request` disagreeing.
fn no_parts() -> Expect {
    let encoding = |part: &str| loc(&at(MULTIPART, &format!("/encoding/{part}")));
    Expect::Warn {
        at: vec![loc(&at(MULTIPART, "/schema")), encoding("cover"), encoding("meta"), encoding("pages"), loc(&at(UPLOAD_OP, "/x-schematic/request"))],
        absent: vec!["op.request"],
    }
}

fn form_field<'a>(api: &'a RestApi, name: &str) -> Option<&'a schematic_define::FormField> {
    match &endpoint(api, "Upload").request {
        Some(ApiRequest::FormData { fields }) => fields.iter().find(|field| field.name == name),
        _ => None,
    }
}

/// `requestBody` without content: ERROR at `content`, plus the extension's
/// `request` WARN since no body is left to match.
fn no_content(request: &str) -> Expect {
    Expect::ErrorDiagnostic { at: loc(&at(UPLOAD_OP, "/requestBody/content")), warns: vec![request.to_string()] }
}

fn body_cells() -> Vec<Cell> {
    let schema = |rest: &str| at(MULTIPART, &format!("/schema{rest}"));
    let encoding = |rest: &str| at(MULTIPART, &format!("/encoding{rest}"));
    let request = loc(&at(UPLOAD_OP, "/x-schematic/request"));
    vec![
        cell("requestBody absent", at(UPLOAD_OP, "/requestBody"), None, Expect::Warn { at: vec![request.clone()], absent: vec!["op.request"] })
            .check(check("no body", |api| endpoint(api, "Upload").request.is_none())),
        cell("requestBody content absent", at(UPLOAD_OP, "/requestBody/content"), None, no_content(&request))
            .check(check("no body", |api| endpoint(api, "Upload").request.is_none())),
        e("requestBody content null", at(UPLOAD_OP, "/requestBody/content"), Some(Value::Null)),
        cell("requestBody content empty", at(UPLOAD_OP, "/requestBody/content"), Some(json!({})), no_content(&request))
            .check(check("no body", |api| endpoint(api, "Upload").request.is_none())),
        // With no parts, each `encoding` entry names an unknown part and the
        // extension's fields disagree; all diagnosed, as the contract asks.
        cell("multipart properties absent", schema("/properties"), None, no_parts()),
        cell("multipart properties empty", schema("/properties"), Some(json!({})), no_parts()),
        cell("multipart object-valued part", schema("/properties/note"), Some(json!({"type": "object"})), Expect::Warn {
            at: vec![loc(&schema("/properties/note")), request.clone()],
            absent: vec!["op.request"],
        }),
        cell("multipart array of non-binary items", schema("/properties/pages/items"), Some(json!({"type": "string"})), Expect::Warn {
            at: vec![loc(&schema("/properties/pages")), request.clone()],
            absent: vec!["op.request"],
        }),
        cell("multipart minItems absent", schema("/properties/pages/minItems"), None, clean(&[]))
            .check(check("extension min fills", |api| {
                matches!(form_field(api, "pages").map(|field| &field.kind), Some(FormFieldKind::Files { min: Some(1), max: Some(4), .. }))
            })),
        // Standard wins over the extension's `required: true`, once per part.
        cell("multipart required absent", schema("/required"), None, Expect::Warn { at: vec![request.clone(); 3], absent: vec!["op.request"] })
            .check(check("all optional", all_parts_optional)),
        cell("multipart required empty", schema("/required"), Some(json!([])), Expect::Warn { at: vec![request.clone(); 3], absent: vec!["op.request"] })
            .check(check("all optional", all_parts_optional)),
        cell("encoding contentType absent", encoding("/cover/contentType"), None, clean(&[]))
            .check(check("extension accept fills", |api| {
                matches!(form_field(api, "cover").map(|field| &field.kind), Some(FormFieldKind::File { accept }) if accept == &["image/png", "image/jpeg"])
            })),
        cell("encoding contentType empty", encoding("/cover/contentType"), Some(json!("")), Expect::ErrorDiagnostic { at: loc(&encoding("/cover/contentType")), warns: vec![] }),
        cell("encoding for an unknown part", encoding("/ghost"), Some(json!({"contentType": "text/plain"})), warn(loc(&encoding("/ghost")), &[])),
        cell("encoding headers", encoding("/cover/headers"), Some(json!({"X-Part": {"schema": {"type": "string"}}})), warn(loc(&encoding("/cover/headers")), &[])),
        cell("encoding style", encoding("/cover/style"), Some(json!("form")), warn(loc(&encoding("/cover/style")), &[])),
    ]
}

fn op_policy(api: &RestApi) -> Option<&EndpointAuth> {
    endpoint(api, "GetItem").auth.as_ref()
}

fn security_cells() -> Vec<Cell> {
    let op_auth = loc(&at(GET, "/x-schematic/auth/bearerAuth"));
    let op_env = loc(&at(GET, "/x-schematic/env_mapping"));
    let scheme = |rest: &str| format!("/components/securitySchemes/apiKeyAuth{rest}");
    vec![
        cell("operation security absent", at(GET, "/security"), None, Expect::Warn {
            at: vec![op_auth.clone(), op_env.clone()],
            absent: vec!["op.auth", "op.env_mapping"],
        })
        .check(check("inherits root", |api| op_policy(api).is_none())),
        cell("operation security empty", at(GET, "/security"), Some(json!([])), Expect::Warn {
            at: vec![op_auth.clone(), op_env.clone()],
            absent: vec!["op.auth", "op.env_mapping"],
        })
        .check(check("public", |api| op_policy(api) == Some(&EndpointAuth::Public))),
        cell("operation security AND requirement", at(GET, "/security/0"), Some(json!({"bearerAuth": [], "apiKeyAuth": []})), Expect::Warn {
            at: vec![loc(&at(GET, "/security/0")), op_auth.clone()],
            absent: vec!["op.auth", "op.env_mapping"],
        }),
        e("operation security null", at(GET, "/security"), Some(Value::Null)),
        e("operation security wrong type", at(GET, "/security"), Some(json!({}))),
        cell("root security absent", "/security".into(), None, clean(&[]))
            .check(check("first scheme", |api| matches!(&api.auth, AuthStrategy::ApiKey { header, value_prefix: Some(prefix) } if header == "x-api-key" && prefix == "Key "))),
        cell("root security empty", "/security".into(), Some(json!([])), Expect::Warn {
            at: vec![loc("/x-schematic/auth/apiKeyAuth")],
            absent: vec!["auth"],
        })
        .check(check("anonymous", |api| api.auth == AuthStrategy::None)),
        e("root security null", "/security".into(), Some(Value::Null)),
        e("securityScheme type absent", scheme("/type"), None),
        e("securityScheme type null", scheme("/type"), Some(Value::Null)),
        e("securityScheme in absent", scheme("/in"), None),
        e("securityScheme name absent", scheme("/name"), None),
        cell("securityScheme name empty", scheme("/name"), Some(json!("")), Expect::ErrorDiagnostic { at: loc(&scheme("/name")), warns: vec![] })
            .check(check("scheme not mapped", |api| !matches!(&api.auth, AuthStrategy::ApiKey { .. }))),
    ]
}

fn document_cells() -> Vec<Cell> {
    vec![
        e("info.version null", "/info/version".into(), Some(Value::Null)),
        cell("operation deprecated", at(GET, "/deprecated"), Some(json!(true)), clean(&[]))
            .check(check("GetItem deprecated", |api| endpoint(api, "GetItem").deprecated)),
        e("info.version absent", "/info/version".into(), None),
        cell("info.version empty", "/info/version".into(), Some(json!("")), Expect::ErrorDiagnostic { at: loc("/info/version"), warns: vec![] })
            .check(check("no version", |api| api.version.is_none())),
        cell("servers absent", "/servers".into(), None, warn(loc("/servers"), &[]))
            .check(check("default base URL", |api| api.base_url == "https://api.example.com")),
        cell("servers url empty", "/servers/0/url".into(), Some(json!("")), Expect::ErrorDiagnostic { at: loc("/servers/0/url"), warns: vec![] }),
        cell("externalDocs url empty", "/externalDocs".into(), Some(json!({"url": ""})), Expect::ErrorDiagnostic { at: loc("/externalDocs/url"), warns: vec![] })
            .check(check("no docs_url", |api| api.docs_url.is_none())),
        cell("externalDocs url", "/externalDocs".into(), Some(json!({"url": "https://docs.fixture.test"})), clean(&[]))
            .check(check("docs_url", |api| api.docs_url.as_deref() == Some("https://docs.fixture.test"))),
    ]
}

fn table() -> Vec<Cell> {
    let mut cells = parameter_cells();
    cells.extend(body_cells());
    cells.extend(security_cells());
    cells.extend(document_cells());
    cells
}

#[test]
fn augmented_control_row_imports_without_problems() {
    let control = control_api();
    for format in [Format::Json, Format::Yaml] {
        let text = format.text(&base());
        let failures = check_outcome(format, &text, &clean(&[]), Some(&|result: &OpenApiImportResult| {
            if control_query(&result.api) { Ok(()) } else { Err(format!("query {:?}", query(&result.api))) }
        }), &control);
        assert!(failures.is_empty(), "{format:?}: {failures:?}");
    }
}

#[test]
fn standard_matrix_json() {
    walk(Format::Json, &base(), &control_api(), &table());
}

#[test]
fn standard_matrix_yaml() {
    walk(Format::Yaml, &base(), &control_api(), &table());
}

