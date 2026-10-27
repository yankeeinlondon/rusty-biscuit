//! The control fixture and the cell walker shared by the OpenAPI input
//! matrices (`openapi_extension_matrix`, `openapi_standard_matrix`).
//!
//! The control is one `RestApi` built with the public builders so every
//! matrix column is load-bearing, exported once. Each cell mutates exactly
//! one place of that exported document, is serialized to JSON or YAML text,
//! imported permissively and strictly, and checked through the public
//! result only.

#![allow(dead_code)]

use biscuit_file::serde_yaml_ng;
use schematic_define::openapi::{
    DiagnosticSeverity, ExportOptions, OpenApiDiagnostic, OpenApiImport, OpenApiImportResult, OpenApiSource, export,
};
use schematic_define::{
    ApiKeyEnv, AuthMethod, AuthPolicy, AuthStrategy, Backoff, CrossOrigin, DefinitionContext, Endpoint, EnvList,
    EnvMapping, ErrorSet, ErrorShape, FormField, ParamDef, QueryParamType, RateLimit, ResetUnit, RestApi,
    RetryGuidance, StatusMatch, StatusSelector, Success, TypeRef, definition_type,
};
use serde_json::Value;

definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Item { pub name: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Chunk { pub text: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Problem { pub message: String });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct UploadMeta { pub title: String });

/// Text formats a cell is serialized to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Yaml,
}

impl Format {
    pub fn text(self, document: &Value) -> String {
        match self {
            Self::Json => serde_json::to_string_pretty(document).unwrap(),
            Self::Yaml => serde_yaml_ng::to_string(document).unwrap(),
        }
    }

    pub fn source(self, text: impl Into<String>) -> OpenApiSource {
        match self {
            Self::Json => OpenApiSource::json(text.into()),
            Self::Yaml => OpenApiSource::yaml(text.into()),
        }
    }
}

pub const ITEMS: &str = "/items/{item_id}";
pub const ITEMS_PTR: &str = "#/paths/~1items~1{item_id}";
pub const GET_PTR: &str = "#/paths/~1items~1{item_id}/get";
pub const UPLOAD: &str = "/uploads";
pub const UPLOAD_PTR: &str = "#/paths/~1uploads/post";
pub const STREAM: &str = "/stream";
pub const STREAM_PTR: &str = "#/paths/~1stream/post";
pub const FILES: &str = "/files/{+path}";
pub const FILES_KEY: &str = "/files/{path}";
pub const FILES_PTR: &str = "#/paths/~1files~1{path}/get";

pub fn rate_limit(prefix: &str) -> RateLimit {
    RateLimit::new(format!("{prefix}-limit"), format!("{prefix}-remaining"), format!("{prefix}-reset"), ResetUnit::Seconds).unwrap()
}

pub fn retry_guidance(attempts: u8) -> RetryGuidance {
    RetryGuidance::new(
        [StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 503 }],
        true,
        attempts,
        Backoff::Exponential { base_ms: 100, cap_ms: 2000 },
    )
    .unwrap()
}

/// The control `RestApi`; every matrix column has a non-default value.
pub fn control_api() -> RestApi {
    let mut ctx = DefinitionContext::types_in("::fixture");
    let problem = ctx.register::<Problem>().unwrap();
    let chunk = ctx.register::<Chunk>().unwrap();
    let item = ctx.register::<Item>().unwrap();
    ctx.register::<UploadMeta>().unwrap();

    let get = Endpoint::get("GetItem", ITEMS)
        .describes("Get one item")
        .returns(Success::json(item.clone()))
        .path_param(ParamDef::path("item_id", QueryParamType::String).rust_name("id"))
        .query(ParamDef::new("api_token", QueryParamType::String).secret())
        .query(ParamDef::new("verbose", QueryParamType::Boolean))
        .header("X-Endpoint", "endpoint-value")
        .rate_limit(rate_limit("x-op"))
        .retry_guidance(retry_guidance(5))
        .cross_origin(CrossOrigin::Deny)
        .errors(ErrorSet::new().on(StatusMatch::class(5), ErrorShape::text()))
        .auth_with_env_mapping(
            AuthPolicy { explicit: vec![AuthMethod::BearerToken { header: Some("x-op-auth".into()) }], env_fallback: None },
            EnvMapping { bearer_token: Some(EnvList::from_strs(&["OP_TOKEN"])), ..Default::default() },
        )
        .build()
        .unwrap();
    let upload = Endpoint::post("Upload", UPLOAD)
        .describes("Upload files")
        .request_form(vec![
            FormField::file_accept("cover", vec!["image/png".into(), "image/jpeg".into()]),
            FormField::files_with_constraints("pages", vec!["application/pdf".into()], Some(1), Some(4)),
            FormField::json("meta", TypeRef::Named("UploadMeta".into())),
            FormField::text("note").optional().with_description("A free note"),
        ])
        .returns_empty()
        .build()
        .unwrap();
    let stream = Endpoint::post("Stream", STREAM)
        .describes("Stream chunks")
        .returns(Success::json(item.clone()).note("when not streaming"))
        .returns(Success::sse(chunk).ends_at_data_line("[DONE]").note("when streaming"))
        .build()
        .unwrap();
    let files = Endpoint::get("GetFile", FILES).describes("Get a file").returns_binary().build().unwrap();

    RestApi::builder(&ctx, "Fixture", "https://api.fixture.test")
        .describes("Matrix control fixture")
        .authentication(AuthStrategy::ApiKey { header: "x-api-key".into(), value_prefix: Some("Key ".into()) }, ["FIX_KEY"])
        .env_mapping(EnvMapping {
            api_key: Some(ApiKeyEnv { names: EnvList::from_strs(&["FIX_KEY"]), header: "x-api-key".into() }),
            ..Default::default()
        })
        .version("2.3.4")
        .module_path("fixture_api")
        .request_suffix("Req")
        .header("X-Client", "matrix")
        .rate_limit(rate_limit("x-api"))
        .retry_guidance(retry_guidance(3))
        .default_errors(ErrorSet::new().on(StatusMatch::class(4), ErrorShape::json(problem)))
        .cross_origin(CrossOrigin::AllowOrigins(vec!["https://cdn.fixture.test".into()]))
        .request_id_header("X-Request-Id")
        .secret_header("X-Session")
        .endpoints([get, upload, stream, files])
        .build()
        .unwrap()
}

/// The control document, exported once.
pub fn control_document() -> Value {
    serde_json::to_value(export(&control_api(), &ExportOptions::new()).unwrap()).unwrap()
}

pub fn import_text(format: Format, text: &str) -> Result<OpenApiImportResult, String> {
    OpenApiImport::new(format.source(text)).build().map_err(|error| error.to_string())
}

pub fn import_strict(format: Format, text: &str) -> Result<OpenApiImportResult, String> {
    OpenApiImport::new(format.source(text)).strict().build().map_err(|error| error.to_string())
}

/// Diagnostics at WARN or above, as `(severity, location)`.
pub fn problems(result: &OpenApiImportResult) -> Vec<(DiagnosticSeverity, String)> {
    result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity >= DiagnosticSeverity::Warn)
        .map(|OpenApiDiagnostic { severity, location, .. }| (*severity, location.clone()))
        .collect()
}

pub fn infos(result: &OpenApiImportResult) -> Vec<String> {
    result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Info)
        .map(|diagnostic| diagnostic.location.clone())
        .collect()
}

/// Sets (`Some`) or removes (`None`) the value at RFC 6901 `pointer` (no
/// leading `#`) of `document`; every parent must already exist.
pub fn mutate(document: &mut Value, pointer: &str, value: Option<Value>) {
    let (parent, last) = pointer.rsplit_once('/').expect("pointer has a parent");
    let last = last.replace("~1", "/").replace("~0", "~");
    let parent = document.pointer_mut(parent).unwrap_or_else(|| panic!("no parent at {parent}"));
    match (parent, value) {
        (Value::Object(object), Some(value)) => {
            object.insert(last, value);
        }
        (Value::Object(object), None) => {
            object.remove(&last).unwrap_or_else(|| panic!("nothing to remove at {pointer}"));
        }
        (Value::Array(items), Some(value)) => {
            let index: usize = last.parse().unwrap();
            if index == items.len() { items.push(value) } else { items[index] = value }
        }
        (Value::Array(items), None) => {
            items.remove(last.parse::<usize>().unwrap());
        }
        (other, _) => panic!("cannot mutate under {other}"),
    }
}

pub fn endpoint<'a>(api: &'a RestApi, id: &str) -> &'a Endpoint {
    api.endpoints.iter().find(|endpoint| endpoint.id == id).unwrap_or_else(|| panic!("endpoint {id} imported"))
}

/// One column's control effect, observed on an imported API against the
/// control `RestApi`. `extension` marks effects only `x-schematic` carries;
/// they are what a disabled (D) document loses.
pub struct Probe {
    pub name: &'static str,
    pub extension: bool,
    pub holds: fn(&RestApi, &RestApi) -> bool,
}

macro_rules! probe {
    ($name:literal, $extension:literal, |$imported:ident, $control:ident| $body:expr) => {
        Probe { name: $name, extension: $extension, holds: |$imported: &RestApi, $control: &RestApi| $body }
    };
}

/// Every control column, document level then operation level.
pub const PROBES: &[Probe] = &[
    probe!("module_path", true, |i, c| i.module_path == c.module_path),
    probe!("request_suffix", true, |i, c| i.request_suffix == c.request_suffix),
    probe!("env_mapping", true, |i, c| i.env_mapping == c.env_mapping),
    probe!("env_auth", true, |i, c| i.env_auth == c.env_auth),
    probe!("headers", true, |i, c| i.headers == c.headers),
    probe!("auth", true, |i, c| i.auth == c.auth),
    probe!("rate_limit", true, |i, c| i.rate_limit == c.rate_limit),
    probe!("retry_guidance", true, |i, c| i.retry_guidance == c.retry_guidance),
    probe!("errors", true, |i, c| i.errors == c.errors),
    probe!("cross_origin", true, |i, c| i.cross_origin == c.cross_origin),
    probe!("request_id_header", true, |i, c| i.request_id_header == c.request_id_header),
    probe!("secret_names", true, |i, c| i.secret_names == c.secret_names),
    probe!("version", false, |i, c| i.version == c.version),
    probe!("op.headers", true, |i, c| endpoint(i, "GetItem").headers == endpoint(c, "GetItem").headers),
    probe!("op.rate_limit", true, |i, c| endpoint(i, "GetItem").rate_limit == endpoint(c, "GetItem").rate_limit),
    probe!("op.retry_guidance", true, |i, c| endpoint(i, "GetItem").retry_guidance == endpoint(c, "GetItem").retry_guidance),
    probe!("op.errors", true, |i, c| endpoint(i, "GetItem").errors == endpoint(c, "GetItem").errors),
    probe!("op.cross_origin", true, |i, c| endpoint(i, "GetItem").cross_origin == endpoint(c, "GetItem").cross_origin),
    probe!("op.auth", true, |i, c| auth_methods(endpoint(i, "GetItem")) == auth_methods(endpoint(c, "GetItem"))),
    probe!("op.env_mapping", true, |i, c| op_env_mapping(endpoint(i, "GetItem")) == op_env_mapping(endpoint(c, "GetItem"))),
    probe!("op.params", true, |i, c| param_details(endpoint(i, "GetItem")) == param_details(endpoint(c, "GetItem"))),
    probe!("op.request", false, |i, c| sorted_fields(endpoint(i, "Upload")) == sorted_fields(endpoint(c, "Upload"))),
    probe!("op.success", true, |i, c| endpoint(i, "Stream").response == endpoint(c, "Stream").response),
    probe!("op.path_template", true, |i, c| endpoint(i, "GetFile").path == endpoint(c, "GetFile").path),
];

pub fn probe(name: &str) -> &'static Probe {
    PROBES.iter().find(|probe| probe.name == name).unwrap_or_else(|| panic!("no probe {name}"))
}

/// The form fields by name: a document round-tripped through
/// `serde_json::Value` sorts `properties`, and the import follows the
/// standard schema's order, not the extension's.
fn sorted_fields(endpoint: &Endpoint) -> Option<Vec<FormField>> {
    match &endpoint.request {
        Some(schematic_define::ApiRequest::FormData { fields }) => {
            let mut fields = fields.clone();
            fields.sort_by(|a, b| a.name.cmp(&b.name));
            Some(fields)
        }
        _ => None,
    }
}

fn auth_methods(endpoint: &Endpoint) -> Option<Vec<AuthMethod>> {
    match &endpoint.auth {
        Some(schematic_define::EndpointAuth::Policy { policy, .. }) => Some(policy.explicit.clone()),
        _ => None,
    }
}

fn op_env_mapping(endpoint: &Endpoint) -> Option<EnvMapping> {
    match &endpoint.auth {
        Some(schematic_define::EndpointAuth::Policy { env_mapping, .. }) => env_mapping.clone(),
        _ => None,
    }
}

/// `(location, name, secret, rust_name)` of every parameter that carries a
/// detail.
fn param_details(endpoint: &Endpoint) -> Vec<(&'static str, String, bool, Option<String>)> {
    let Some(params) = &endpoint.params else { return vec![] };
    let mut details = Vec::new();
    for (location, list) in [("path", &params.path), ("query", &params.query), ("header", &params.header), ("cookie", &params.cookie)] {
        for param in list.iter().filter(|param| param.secret || param.rust_name.is_some()) {
            details.push((location, param.name.clone(), param.secret, param.rust_name.clone()));
        }
    }
    details
}

/// A cell's expected public outcome.
pub enum Expect {
    /// I, O, or V: no WARN or ERROR, strict import passes, the `absent`
    /// probes fail (the column's control effect is gone) and every other
    /// probe holds. `infos` are the exact INFO locations.
    Clean { absent: &'static [&'static str], infos: Vec<String> },
    /// W: exactly the WARNs at `at`, the `absent` probes fail, every other
    /// probe (the unrelated siblings) holds, and strict import fails.
    Warn { at: Vec<String>, absent: Vec<&'static str> },
    /// D: exactly one WARN at `at`, every extension probe fails, and strict
    /// import fails.
    Disabled { at: String },
    /// E as a failed build in both modes.
    BuildError,
    /// E as an ERROR diagnostic at `at` (with any WARNs in `warns`), which
    /// fails strict import.
    ErrorDiagnostic { at: String, warns: Vec<String> },
}

/// An extra assertion on a permissive import result.
pub type ResultCheck = dyn Fn(&OpenApiImportResult) -> Result<(), String>;

/// One mutation of the base document and its outcome.
pub struct Cell {
    pub name: &'static str,
    /// RFC 6901 pointer without `#`.
    pub pointer: String,
    /// `None` removes the member.
    pub value: Option<Value>,
    pub expect: Expect,
    /// An extra assertion on the permissive result (V values and the like).
    pub check: Option<Box<ResultCheck>>,
    /// `Some("<rule>")` when the cell asserts an outcome adopted by ruling
    /// that differs from contracts.md; every other cell asserts the contract.
    pub adopted: Option<&'static str>,
}

impl Cell {
    pub fn new(name: &'static str, pointer: impl Into<String>, value: Option<Value>, expect: Expect) -> Self {
        Self { name, pointer: pointer.into(), value, expect, check: None, adopted: None }
    }

    pub fn check(mut self, check: impl Fn(&OpenApiImportResult) -> Result<(), String> + 'static) -> Self {
        self.check = Some(Box::new(check));
        self
    }

    /// Adds one more expected WARN at `at` (a `Clean` cell becomes `Warn`),
    /// with the probes it additionally loses.
    pub fn plus_warn(mut self, at: impl Into<String>, lost: &[&'static str]) -> Self {
        self.expect = match self.expect {
            Expect::Clean { absent, infos } => {
                assert!(infos.is_empty(), "{}: INFO with an added WARN", self.name);
                Expect::Warn { at: vec![at.into()], absent: absent.iter().chain(lost).copied().collect() }
            }
            Expect::Warn { at: mut locations, mut absent } => {
                locations.push(at.into());
                absent.extend(lost);
                Expect::Warn { at: locations, absent }
            }
            _ => panic!("{}: plus_warn on a non-WARN cell", self.name),
        };
        self
    }

    /// Adds probes a `Warn` cell also loses, with no further WARN.
    pub fn losing(mut self, lost: &[&'static str]) -> Self {
        match &mut self.expect {
            Expect::Warn { absent, .. } => absent.extend(lost),
            _ => panic!("{}: losing on a non-WARN cell", self.name),
        }
        self
    }

    /// Marks the asserted outcome as an adopted rule that departs from
    /// contracts.md.
    pub fn adopted(mut self, rule: &'static str) -> Self {
        self.adopted = Some(rule);
        self
    }
}

pub fn clean(absent: &'static [&'static str]) -> Expect {
    Expect::Clean { absent, infos: vec![] }
}

pub fn warn(at: impl Into<String>, absent: &'static [&'static str]) -> Expect {
    Expect::Warn { at: vec![at.into()], absent: absent.to_vec() }
}

pub fn disabled(at: impl Into<String>) -> Expect {
    Expect::Disabled { at: at.into() }
}

/// Imports `text` permissively and strictly and checks `expect`; returns
/// every mismatch.
pub fn check_outcome(
    format: Format,
    text: &str,
    expect: &Expect,
    extra: Option<&ResultCheck>,
    control: &RestApi,
) -> Vec<String> {
    let mut failures = Vec::new();
    let permissive = import_text(format, text);
    let strict = import_strict(format, text);
    let probes = |result: &OpenApiImportResult, absent: &[&str], every_extension: bool, failures: &mut Vec<String>| {
        for probe in PROBES {
            let holds = (probe.holds)(&result.api, control);
            let should_hold = if every_extension { !probe.extension } else { !absent.contains(&probe.name) };
            if every_extension && !probe.extension {
                continue;
            }
            if holds != should_hold {
                failures.push(format!("probe `{}` {} but should {}", probe.name, if holds { "holds" } else { "fails" }, if should_hold { "hold" } else { "fail" }));
            }
        }
    };
    match expect {
        Expect::BuildError => {
            if let Ok(result) = &permissive {
                failures.push(format!("expected a build error, imported with {:?}", problems(result)));
            }
            if strict.is_ok() {
                failures.push("strict import succeeded".into());
            }
            return failures;
        }
        _ if permissive.is_err() => {
            failures.push(format!("build failed: {}", permissive.unwrap_err()));
            return failures;
        }
        _ => {}
    }
    let result = permissive.unwrap();
    let found = problems(&result);
    match expect {
        Expect::Clean { absent, infos: expected_infos } => {
            if !found.is_empty() {
                failures.push(format!("unexpected problems {found:?}"));
            }
            if &infos(&result) != expected_infos {
                failures.push(format!("INFO at {:?}, expected {expected_infos:?}", infos(&result)));
            }
            if let Err(error) = &strict {
                failures.push(format!("strict failed: {error}"));
            }
            probes(&result, absent, false, &mut failures);
        }
        Expect::Warn { at, absent } => {
            let mut expected: Vec<_> = at.iter().map(|at| (DiagnosticSeverity::Warn, at.clone())).collect();
            let mut sorted = found.clone();
            sorted.sort();
            expected.sort();
            if sorted != expected {
                failures.push(format!("problems {found:?}, expected {expected:?}"));
            }
            if strict.is_ok() {
                failures.push("strict import succeeded".into());
            }
            probes(&result, absent, false, &mut failures);
        }
        Expect::Disabled { at } => {
            if found != [(DiagnosticSeverity::Warn, at.clone())] {
                failures.push(format!("problems {found:?}, expected one WARN at {at}"));
            }
            if strict.is_ok() {
                failures.push("strict import succeeded".into());
            }
            probes(&result, &[], true, &mut failures);
        }
        Expect::ErrorDiagnostic { at, warns } => {
            let mut expected: Vec<_> = warns.iter().map(|at| (DiagnosticSeverity::Warn, at.clone())).collect();
            expected.push((DiagnosticSeverity::Error, at.clone()));
            let mut sorted = found.clone();
            sorted.sort();
            expected.sort();
            if sorted != expected {
                failures.push(format!("problems {found:?}, expected {expected:?}"));
            }
            if strict.is_ok() {
                failures.push("strict import succeeded".into());
            }
        }
        Expect::BuildError => unreachable!(),
    }
    if let Some(extra) = extra
        && let Err(problem) = extra(&result)
    {
        failures.push(problem);
    }
    failures
}

/// Runs every cell against `base` in `format` and panics listing every
/// failing cell.
pub fn walk(format: Format, base: &Value, control: &RestApi, cells: &[Cell]) {
    let mut failures = Vec::new();
    let mut names = std::collections::HashSet::new();
    for cell in cells {
        assert!(names.insert(cell.name), "duplicate cell name {}", cell.name);
        let mut document = base.clone();
        mutate(&mut document, &cell.pointer, cell.value.clone());
        let text = format.text(&document);
        for failure in check_outcome(format, &text, &cell.expect, cell.check.as_deref(), control) {
            failures.push(format!("[{format:?}] {}: {failure}", cell.name));
        }
    }
    assert!(failures.is_empty(), "{} failing cell checks:\n{}", failures.len(), failures.join("\n"));
}
