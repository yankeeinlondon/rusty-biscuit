//! Input matrix 1 of `2026-10-18-schematic-fidelity-polish` (contracts.md,
//! "Input matrices (frozen columns)"): every `x-schematic` column, the
//! version controls, and the document-shape rows, each cell run through the
//! public importer from JSON text and from YAML text.
//!
//! Each cell mutates one place of the exported control document. A cell
//! whose asserted outcome is a rule adopted after contracts.md carries an
//! `adopted` note; every other cell asserts the contract.

#![cfg(feature = "openapi")]

mod matrix_support;

use matrix_support::*;
use schematic_define::{AuthStrategy, CrossOrigin, EnvList, EnvMapping, FormFieldKind, PrimitiveType, StreamEnd, TypeRef};
use serde_json::{Value, json};

/// `/x-schematic<rest>`.
fn x(rest: &str) -> String {
    format!("/x-schematic{rest}")
}

/// The `GetItem` operation's `x-schematic<rest>`.
fn op(rest: &str) -> String {
    format!("/paths/~1items~1{{item_id}}/get/x-schematic{rest}")
}

fn upload(rest: &str) -> String {
    format!("/paths/~1uploads/post/x-schematic{rest}")
}

fn stream(rest: &str) -> String {
    format!("/paths/~1stream/post/x-schematic{rest}")
}

fn files(rest: &str) -> String {
    format!("/paths/~1files~1{{path}}/get/x-schematic{rest}")
}

fn loc(pointer: &str) -> String {
    format!("#{pointer}")
}

fn set(name: &'static str, pointer: String, value: Value, expect: Expect) -> Cell {
    Cell::new(name, pointer, Some(value), expect)
}

fn remove(name: &'static str, pointer: String, expect: Expect) -> Cell {
    Cell::new(name, pointer, None, expect)
}

/// A W cell whose WARN is at the mutated pointer itself.
fn w(name: &'static str, pointer: String, value: Value, absent: &'static [&'static str]) -> Cell {
    let at = loc(&pointer);
    set(name, pointer, value, warn(at, absent))
}

/// A W cell whose WARN is at `at` rather than at the mutated pointer.
fn w_at(name: &'static str, pointer: String, value: Value, at: String, absent: &'static [&'static str]) -> Cell {
    set(name, pointer, value, warn(loc(&at), absent))
}

fn w_removed(name: &'static str, pointer: String, at: String, absent: &'static [&'static str]) -> Cell {
    remove(name, pointer, warn(loc(&at), absent))
}

fn api_check(check: impl Fn(&schematic_define::RestApi) -> bool + 'static, what: &'static str) -> impl Fn(&schematic_define::openapi::OpenApiImportResult) -> Result<(), String> {
    move |result| if check(&result.api) { Ok(()) } else { Err(format!("check failed: {what}")) }
}

fn version_cells() -> Vec<Cell> {
    let version = x("/version");
    let d = || disabled(loc(&x("/version")));
    vec![
        remove("version absent is major 1", version.clone(), clean(&[])),
        set("version 1", version.clone(), json!(1), clean(&[])),
        set("version 2", version.clone(), json!(2), d()),
        set("version 0", version.clone(), json!(0), d()),
        set("version fractional 1.0", version.clone(), json!(1.0), d()),
        set("version string", version.clone(), json!("1"), d()),
        set("version empty string", version.clone(), json!(""), d()),
        set("version null", version.clone(), Value::Null, d()),
        set("version bool", version.clone(), json!(true), d()),
        set("version array", version, json!([1]), d()),
        set("x-schematic null", x(""), Value::Null, disabled(loc(&x("")))),
        set("x-schematic array", x(""), json!([]), disabled(loc(&x("")))),
        set("x-schematic string", x(""), json!("x"), disabled(loc(&x("")))),
        remove("x-schematic absent", x(""), clean(DOC_MEMBERS)).plus_warn(loc(&op("/errors")), &["op.errors"]).adopted(OWN_TABLE),
        set("x-schematic empty object", x(""), json!({}), clean(DOC_MEMBERS))
            .plus_warn(loc(&op("/errors")), &["op.errors"])
            .adopted(OWN_TABLE),
        set(
            "unknown top-level member",
            x("/future_member"),
            json!({"any": 1}),
            Expect::Clean { absent: &[], infos: vec![loc(&x("/future_member"))] },
        ),
        w("malformed member beside valid sibling", x("/rate_limit"), json!("fast"), &["rate_limit"]),
    ]
}

/// Every document-level effect `x-schematic` carries.
const DOC_MEMBERS: &[&str] = &[
    "module_path", "request_suffix", "env_mapping", "env_auth", "headers", "auth", "rate_limit", "retry_guidance",
    "errors", "cross_origin", "request_id_header", "secret_names",
];

fn scalar_cells() -> Vec<Cell> {
    vec![
        remove("module_path absent", x("/module_path"), clean(&["module_path"])),
        w("module_path null", x("/module_path"), Value::Null, &["module_path"]),
        w("module_path wrong type", x("/module_path"), json!(5), &["module_path"]),
        w("module_path wrong element", x("/module_path"), json!(["fixture_api"]), &["module_path"]),
        w("module_path empty", x("/module_path"), json!(""), &["module_path"]),
        remove("request_suffix absent", x("/request_suffix"), clean(&["request_suffix"])),
        w("request_suffix null", x("/request_suffix"), Value::Null, &["request_suffix"]),
        w("request_suffix wrong type", x("/request_suffix"), json!(1), &["request_suffix"]),
        w("request_suffix empty", x("/request_suffix"), json!(""), &["request_suffix"]),
        w("request_suffix invalid identifier", x("/request_suffix"), json!("Re-q"), &["request_suffix"]),
        remove("request_id_header absent", x("/request_id_header"), clean(&["request_id_header"])),
        w("request_id_header null", x("/request_id_header"), Value::Null, &["request_id_header"]),
        w("request_id_header wrong type", x("/request_id_header"), json!(5), &["request_id_header"]),
        w("request_id_header empty", x("/request_id_header"), json!(""), &["request_id_header"]),
        w("request_id_header invalid name", x("/request_id_header"), json!("x request"), &["request_id_header"]),
        set("request_id_header upper case normalized", x("/request_id_header"), json!("X-REQUEST-ID"), clean(&[])),
        remove("secret_names absent", x("/secret_names"), clean(&["secret_names"])),
        w("secret_names null", x("/secret_names"), Value::Null, &["secret_names"]),
        w("secret_names wrong type", x("/secret_names"), json!("x-session"), &["secret_names"]),
        w("secret_names wrong element", x("/secret_names"), json!(["x-session", 5]), &["secret_names"]),
        w("secret_names all wrong elements", x("/secret_names"), json!([5, true]), &["secret_names"]),
        w("secret_names empty element", x("/secret_names"), json!([""]), &["secret_names"]),
        w("secret_names invalid header name", x("/secret_names"), json!(["x session"]), &["secret_names"]),
        set("secret_names empty", x("/secret_names"), json!([]), clean(&["secret_names"]))
            .check(api_check(|api| api.secret_names.is_empty(), "secret_names is []")),
    ]
}

fn env_cells() -> Vec<Cell> {
    let mapping = || x("/env_mapping");
    let key_names = || x("/env_mapping/api_key/names/names");
    vec![
        remove("env_mapping absent", mapping(), clean(&["env_mapping"])),
        w("env_mapping null", mapping(), Value::Null, &["env_mapping"]),
        w("env_mapping wrong type", mapping(), json!([]), &["env_mapping"]),
        set("env_mapping empty", mapping(), json!({}), clean(&["env_mapping"]))
            .check(api_check(|api| api.env_mapping == Some(EnvMapping::default()), "env_mapping is {}")),
        set(
            "env_mapping unknown sub-key",
            x("/env_mapping/future"),
            json!({"names": ["X"]}),
            Expect::Clean { absent: &[], infos: vec![loc(&x("/env_mapping/future"))] },
        ),
        w_at("env_mapping conflicts with auth", key_names(), json!(["OTHER_KEY"]), mapping(), &["env_mapping"]),
        // `.bearer_token`, `.basic_user`, `.basic_pass`: the writer emits null.
        remove("env_mapping.bearer_token absent", x("/env_mapping/bearer_token"), clean(&[])),
        set("env_mapping.bearer_token null", x("/env_mapping/bearer_token"), Value::Null, clean(&[])),
        w_at("env_mapping.bearer_token wrong type", x("/env_mapping/bearer_token"), json!("TOKEN"), mapping(), &["env_mapping"]),
        set("env_mapping.basic_user null", x("/env_mapping/basic_user"), Value::Null, clean(&[])),
        w_at("env_mapping.basic_user wrong type", x("/env_mapping/basic_user"), json!(["USER"]), mapping(), &["env_mapping"]),
        remove("env_mapping.basic_pass absent", x("/env_mapping/basic_pass"), clean(&[])),
        w_at("env_mapping.basic_pass wrong type", x("/env_mapping/basic_pass"), json!(5), mapping(), &["env_mapping"]),
        set("env_mapping.bearer_token empty list", x("/env_mapping/bearer_token"), json!({"names": []}), clean(&["env_mapping"]))
            .check(api_check(
                |api| api.env_mapping.as_ref().and_then(|mapping| mapping.bearer_token.clone()) == Some(EnvList::new(vec![])),
                "bearer_token is {names: []}",
            )),
        // `.oauth_*`: the writer never emits null.
        w_at("env_mapping.oauth_client_id null", x("/env_mapping/oauth_client_id"), Value::Null, x("/env_mapping/oauth_client_id"), &["env_mapping"]),
        w_at("env_mapping.oauth_client_secret wrong type", x("/env_mapping/oauth_client_secret"), json!("S"), mapping(), &["env_mapping"]),
        set("env_mapping.oauth_redirect_uri empty list", x("/env_mapping/oauth_redirect_uri"), json!({"names": []}), clean(&["env_mapping"])),
        // `EnvList.names`.
        w_removed("EnvList.names absent", key_names(), mapping(), &["env_mapping"]),
        w("EnvList.names null", key_names(), Value::Null, &["env_mapping"]),
        w_at("EnvList.names wrong type", key_names(), json!("FIX_KEY"), mapping(), &["env_mapping"]),
        w_at("EnvList.names wrong element", key_names(), json!(["FIX_KEY", 5]), mapping(), &["env_mapping"]),
        w_at("EnvList.names empty element", key_names(), json!([""]), mapping(), &["env_mapping"]),
        // An empty list is V where nothing else names the variables; the
        // API-key list is also the auth declaration's, so emptying it is the
        // conflict W.
        set("EnvList.names empty", x("/env_mapping/oauth_client_id"), json!({"names": []}), clean(&["env_mapping"])),
        w_at("EnvList.names empty conflicts with auth", key_names(), json!([]), mapping(), &["env_mapping"]),
        // `.api_key`.
        remove("env_mapping.api_key absent", x("/env_mapping/api_key"), clean(&["env_mapping"])),
        set("env_mapping.api_key null", x("/env_mapping/api_key"), Value::Null, clean(&["env_mapping"])),
        w_at("env_mapping.api_key wrong type", x("/env_mapping/api_key"), json!("x-api-key"), mapping(), &["env_mapping"]),
        w_at("env_mapping.api_key empty", x("/env_mapping/api_key"), json!({}), mapping(), &["env_mapping"]),
        w_removed("api_key.header absent", x("/env_mapping/api_key/header"), mapping(), &["env_mapping"]),
        w("api_key.header null", x("/env_mapping/api_key/header"), Value::Null, &["env_mapping"]),
        w_at("api_key.header wrong type", x("/env_mapping/api_key/header"), json!(5), mapping(), &["env_mapping"]),
        w_at("api_key.header empty", x("/env_mapping/api_key/header"), json!(""), mapping(), &["env_mapping"]),
        w_removed("api_key.names absent", x("/env_mapping/api_key/names"), mapping(), &["env_mapping"]),
        w("api_key.names null", x("/env_mapping/api_key/names"), Value::Null, &["env_mapping"]),
        w_at("api_key.names wrong type", x("/env_mapping/api_key/names"), json!(5), mapping(), &["env_mapping"]),
        w_at("api_key.names empty conflicts with auth", x("/env_mapping/api_key/names"), json!({"names": []}), mapping(), &["env_mapping"]),
        // `env_auth`.
        remove("env_auth absent", x("/env_auth"), clean(&["env_auth"])),
        w("env_auth null", x("/env_auth"), Value::Null, &["env_auth"]),
        w("env_auth wrong type", x("/env_auth"), json!("FIX_KEY"), &["env_auth"]),
        w("env_auth wrong element", x("/env_auth"), json!(["FIX_KEY", 1]), &["env_auth"]),
        w("env_auth empty element", x("/env_auth"), json!([""]), &["env_auth"]),
        set("env_auth empty", x("/env_auth"), json!([]), clean(&["env_auth"])),
    ]
}

fn header_cells() -> Vec<Cell> {
    let headers = || x("/headers");
    let mut cells = vec![
        remove("headers absent", headers(), clean(&["headers"])),
        w("headers null", headers(), Value::Null, &["headers"]),
        w("headers wrong type", headers(), json!({"X-Client": "matrix"}), &["headers"]),
        w("headers entry not a pair", headers(), json!([["X-Client"]]), &["headers"]),
        w("headers entry non-string element", headers(), json!([["X-Client", 5]]), &["headers"]),
        w("headers one wrong element", headers(), json!([["X-Client", "matrix"], "X-Other"]), &["headers"]),
        set("headers empty", headers(), json!([]), clean(&["headers"])),
        w_at("headers name null", x("/headers/0/0"), Value::Null, x("/headers/0/0"), &["headers"]),
        w_at("headers name wrong type", x("/headers/0/0"), json!(5), headers(), &["headers"]),
        w_at("headers name empty", x("/headers/0/0"), json!(""), headers(), &["headers"]),
        w_at("headers name invalid token", x("/headers/0/0"), json!("X Client"), headers(), &["headers"]),
        w_at("headers duplicate name", x("/headers/1"), json!(["x-client", "again"]), headers(), &["headers"]),
        w_at("headers value null", x("/headers/0/1"), Value::Null, x("/headers/0/1"), &["headers"]),
        w_at("headers value wrong type", x("/headers/0/1"), json!(5), headers(), &["headers"]),
        set("headers value empty", x("/headers/0/1"), json!(""), clean(&["headers"]))
            .check(api_check(|api| api.headers == [("X-Client".to_string(), String::new())], "header value is \"\"")),
    ];
    // Operation headers, with the same sub-columns.
    cells.extend([
        remove("op headers absent", op("/headers"), clean(&["op.headers"])),
        w("op headers null", op("/headers"), Value::Null, &["op.headers"]),
        w("op headers wrong type", op("/headers"), json!("X-Endpoint"), &["op.headers"]),
        set("op headers empty", op("/headers"), json!([]), clean(&["op.headers"])),
        w_at("op headers name invalid", op("/headers/0/0"), json!("X Endpoint"), op("/headers"), &["op.headers"]),
    ]);
    cells
}

fn auth_cells() -> Vec<Cell> {
    let entry = || x("/auth/apiKeyAuth");
    vec![
        remove("auth absent", x("/auth"), clean(&["auth"]))
            .check(api_check(|api| api.auth == AuthStrategy::ApiKey { header: "x-api-key".into(), value_prefix: None }, "standard api key")),
        w("auth null", x("/auth"), Value::Null, &["auth"]),
        w("auth wrong type", x("/auth"), json!([]), &["auth"]),
        set("auth empty", x("/auth"), json!({}), clean(&["auth"])),
        w_at("auth names an unapplied scheme", x("/auth/noSuchScheme"), json!({"bearer": true}), x("/auth/noSuchScheme"), &[]),
        w("auth entry null", entry(), Value::Null, &["auth"]),
        w("auth entry wrong type", entry(), json!("Key "), &["auth"]),
        w_at("value_prefix null", x("/auth/apiKeyAuth/value_prefix"), Value::Null, entry(), &["auth"]),
        w_at("value_prefix wrong type", x("/auth/apiKeyAuth/value_prefix"), json!(5), entry(), &["auth"]),
        set("value_prefix empty", x("/auth/apiKeyAuth/value_prefix"), json!(""), clean(&["auth"]))
            .check(api_check(|api| api.auth == AuthStrategy::ApiKey { header: "x-api-key".into(), value_prefix: Some(String::new()) }, "empty prefix kept")),
        remove("value_prefix absent", x("/auth/apiKeyAuth/value_prefix"), clean(&["auth"]))
            .check(api_check(|api| api.auth == AuthStrategy::ApiKey { header: "x-api-key".into(), value_prefix: None }, "no detail")),
        set("bearer true", entry(), json!({"bearer": true}), clean(&["auth"]))
            .check(api_check(|api| api.auth == AuthStrategy::BearerToken { header: Some("x-api-key".into()) }, "bearer on x-api-key")),
        w("bearer false", entry(), json!({"bearer": false}), &["auth"]),
        w("bearer null", entry(), json!({"bearer": null}), &["auth"]),
        w("bearer with value_prefix", entry(), json!({"bearer": true, "value_prefix": "Key "}), &["auth"]),
        w("auth entry extra key", entry(), json!({"value_prefix": "Key ", "header": "x"}), &["auth"]),
        // Operation `auth` applies only to an operation `security` override.
        remove("op auth absent", op("/auth"), clean(&["op.auth"])),
        w("op auth null", op("/auth"), Value::Null, &["op.auth"]),
        w("op auth wrong type", op("/auth"), json!(["bearerAuth"]), &["op.auth"]),
        set("op auth empty", op("/auth"), json!({}), clean(&["op.auth"])),
        w_at("op auth unapplied scheme", op("/auth/apiKeyAuth"), json!({"value_prefix": "K "}), op("/auth/apiKeyAuth"), &[]),
        remove("op env_mapping absent", op("/env_mapping"), clean(&["op.env_mapping"])),
        w("op env_mapping null", op("/env_mapping"), Value::Null, &["op.env_mapping"]),
        w("op env_mapping wrong type", op("/env_mapping"), json!(5), &["op.env_mapping"]),
        set("op env_mapping empty", op("/env_mapping"), json!({}), clean(&["op.env_mapping"])),
    ]
}

fn metadata_cells() -> Vec<Cell> {
    let rl = |rest: &str| x(&format!("/rate_limit{rest}"));
    let rg = |rest: &str| x(&format!("/retry_guidance{rest}"));
    vec![
        remove("rate_limit absent", rl(""), clean(&["rate_limit"])),
        w("rate_limit null", rl(""), Value::Null, &["rate_limit"]),
        w("rate_limit wrong type", rl(""), json!([]), &["rate_limit"]),
        w("rate_limit empty", rl(""), json!({}), &["rate_limit"]),
        w_at("rate_limit unknown sub-key", rl("/window"), json!(60), rl(""), &["rate_limit"]),
        w_removed("rate_limit.limit_header absent", rl("/limit_header"), rl(""), &["rate_limit"]),
        w("rate_limit.limit_header null", rl("/limit_header"), Value::Null, &["rate_limit"]),
        w_at("rate_limit.limit_header wrong type", rl("/limit_header"), json!(5), rl(""), &["rate_limit"]),
        w_at("rate_limit.limit_header empty", rl("/limit_header"), json!(""), rl(""), &["rate_limit"]),
        w_at("rate_limit.remaining_header invalid", rl("/remaining_header"), json!("x remaining"), rl(""), &["rate_limit"]),
        w_removed("rate_limit.reset_header absent", rl("/reset_header"), rl(""), &["rate_limit"]),
        w("rate_limit.retry_after_header null", rl("/retry_after_header"), Value::Null, &["rate_limit"]),
        w_at("rate_limit.retry_after_header wrong type", rl("/retry_after_header"), json!(5), rl(""), &["rate_limit"]),
        w_at("rate_limit.retry_after_header empty", rl("/retry_after_header"), json!(""), rl(""), &["rate_limit"]),
        w_removed("rate_limit.reset_unit absent", rl("/reset_unit"), rl(""), &["rate_limit"]),
        w("rate_limit.reset_unit null", rl("/reset_unit"), Value::Null, &["rate_limit"]),
        w_at("rate_limit.reset_unit unknown", rl("/reset_unit"), json!("weeks"), rl(""), &["rate_limit"]),
        w_at("rate_limit.reset_unit empty", rl("/reset_unit"), json!(""), rl(""), &["rate_limit"]),
        remove("retry_guidance absent", rg(""), clean(&["retry_guidance"])),
        w("retry_guidance null", rg(""), Value::Null, &["retry_guidance"]),
        w("retry_guidance wrong type", rg(""), json!("often"), &["retry_guidance"]),
        w("retry_guidance empty", rg(""), json!({}), &["retry_guidance"]),
        w_at("retry_guidance unknown sub-key", rg("/jitter"), json!(true), rg(""), &["retry_guidance"]),
        w_removed("retry_on absent", rg("/retry_on"), rg(""), &["retry_guidance"]),
        w("retry_on null", rg("/retry_on"), Value::Null, &["retry_guidance"]),
        w_at("retry_on wrong type", rg("/retry_on"), json!(429), rg(""), &["retry_guidance"]),
        w_at("retry_on wrong element", rg("/retry_on/1"), json!(503), rg(""), &["retry_guidance"]),
        set("retry_on empty", rg("/retry_on"), json!([]), clean(&["retry_guidance"]))
            .check(api_check(|api| api.retry_guidance.as_ref().is_some_and(|guidance| guidance.retry_on.is_empty()), "retry_on is []")),
        w_at("retry_on code out of range", rg("/retry_on/0"), json!({"code": 99}), rg(""), &["retry_guidance"]),
        w_at("retry_on reversed range", rg("/retry_on/1"), json!({"range": {"start": 503, "end": 500}}), rg(""), &["retry_guidance"]),
        w_at("retry_on empty selector", rg("/retry_on/0"), json!({}), rg(""), &["retry_guidance"]),
        w("retry_on code null", rg("/retry_on/0/code"), Value::Null, &["retry_guidance"]),
        w_removed("honor_retry_after absent", rg("/honor_retry_after"), rg(""), &["retry_guidance"]),
        w("honor_retry_after null", rg("/honor_retry_after"), Value::Null, &["retry_guidance"]),
        w_at("honor_retry_after quoted", rg("/honor_retry_after"), json!("true"), rg(""), &["retry_guidance"]),
        w_removed("max_attempts absent", rg("/max_attempts"), rg(""), &["retry_guidance"]),
        w("max_attempts null", rg("/max_attempts"), Value::Null, &["retry_guidance"]),
        w_at("max_attempts over 255", rg("/max_attempts"), json!(300), rg(""), &["retry_guidance"]),
        w_at("max_attempts zero", rg("/max_attempts"), json!(0), rg(""), &["retry_guidance"]),
        w_at("max_attempts quoted", rg("/max_attempts"), json!("3"), rg(""), &["retry_guidance"]),
        w_removed("backoff absent", rg("/backoff"), rg(""), &["retry_guidance"]),
        w("backoff null", rg("/backoff"), Value::Null, &["retry_guidance"]),
        w_at("backoff unknown", rg("/backoff"), json!("linear"), rg(""), &["retry_guidance"]),
        w_at("backoff empty", rg("/backoff"), json!({}), rg(""), &["retry_guidance"]),
        w_at("backoff zero delay", rg("/backoff"), json!({"fixed": {"delay_ms": 0}}), rg(""), &["retry_guidance"]),
        w_at("backoff cap below base", rg("/backoff"), json!({"exponential": {"base_ms": 100, "cap_ms": 50}}), rg(""), &["retry_guidance"]),
        set("backoff none", rg("/backoff"), json!("none"), clean(&["retry_guidance"])),
        // Operation level: absent inherits; null is W.
        remove("op rate_limit absent", op("/rate_limit"), clean(&["op.rate_limit"])),
        w("op rate_limit null", op("/rate_limit"), Value::Null, &["op.rate_limit"]),
        w("op rate_limit wrong type", op("/rate_limit"), json!(5), &["op.rate_limit"]),
        w("op rate_limit empty", op("/rate_limit"), json!({}), &["op.rate_limit"]),
        remove("op retry_guidance absent", op("/retry_guidance"), clean(&["op.retry_guidance"])),
        w("op retry_guidance null", op("/retry_guidance"), Value::Null, &["op.retry_guidance"]),
        w("op retry_guidance wrong type", op("/retry_guidance"), json!([]), &["op.retry_guidance"]),
        w("op retry_guidance empty", op("/retry_guidance"), json!({}), &["op.retry_guidance"]),
        w_at("op retry_guidance zero attempts", op("/retry_guidance/max_attempts"), json!(0), op("/retry_guidance"), &["op.retry_guidance"]),
    ]
}

/// R12: an operation whose own error table plus the document defaults no
/// longer reproduces its standard responses WARNs at its own `errors`, and
/// the standard-derived table stands.
const OWN_TABLE: &str = "R12: GetItem's own table no longer reproduces its standard responses; W at its errors, standard wins";

fn error_and_origin_cells() -> Vec<Cell> {
    let errors = || x("/errors");
    let op_errors = || loc(&op("/errors"));
    // A rejected document table WARNs at its own member only; every
    // operation keeps its standard-derived table.
    let doc = |cell: Cell| cell.losing(&["op.errors"]);
    let mut cells: Vec<Cell> = vec![
        remove("errors absent", errors(), clean(&["errors"])).plus_warn(op_errors(), &["op.errors"]).adopted(OWN_TABLE),
        set("errors empty", errors(), json!([]), clean(&["errors"])).plus_warn(op_errors(), &["op.errors"]).adopted(OWN_TABLE),
    ];
    cells.extend([
        w("errors null", errors(), Value::Null, &["errors"]),
        w("errors wrong type", errors(), json!({}), &["errors"]),
        w("errors wrong element", errors(), json!([5]), &["errors"]),
        w_removed("errors status absent", x("/errors/0/status"), errors(), &["errors"]),
        w("errors status null", x("/errors/0/status"), Value::Null, &["errors"]),
        w_at("errors status success", x("/errors/0/status"), json!({"Exact": 200}), errors(), &["errors"]),
        w_at("errors status reversed", x("/errors/0/status"), json!({"Range": {"lo": 499, "hi": 400}}), errors(), &["errors"]),
        w_removed("errors shape absent", x("/errors/0/shape"), errors(), &["errors"]),
        w("errors shape null", x("/errors/0/shape"), Value::Null, &["errors"]),
        w_at("errors shape unknown", x("/errors/0/shape"), json!("Nope"), errors(), &["errors"]),
        w_at(
            "errors shape stream",
            x("/errors/0/shape"),
            json!({"Stream": {"framing": "Bytes", "end": "ConnectionClose"}}),
            errors(),
            &["errors"],
        ),
        w("errors content_type null", x("/errors/0/content_type"), Value::Null, &["errors"]),
        w_at("errors content_type wrong type", x("/errors/0/content_type"), json!(5), errors(), &["errors"]),
        w_at("errors content_type empty", x("/errors/0/content_type"), json!(""), errors(), &["errors"]),
        w_at("errors content_type invalid", x("/errors/0/content_type"), json!("not a media type"), errors(), &["errors"]),
    ]
    .map(doc));
    cells.extend([
        // One WARN at the document member; no operation inherits the
        // disagreeing table.
        set("errors disagree with standard (R12)", x("/errors/0/shape"), json!("Text"), warn(loc(&errors()), &["errors", "op.errors"]))
            .check(api_check(|api| api.errors.is_empty(), "RestApi::errors is empty"))
            .adopted("R12: standard wins, attributed to the document `errors` member"),
        remove("op errors absent", op("/errors"), warn(op_errors(), &["op.errors"])).adopted(OWN_TABLE),
        w("op errors null", op("/errors"), Value::Null, &["op.errors"]),
        w("op errors wrong element", op("/errors"), json!([5]), &["op.errors"]),
        w("op errors empty", op("/errors"), json!([]), &["op.errors"]).adopted(OWN_TABLE),
        w_at("op errors disagree with standard (R12)", op("/errors/0/shape"), json!("Binary"), op("/errors"), &["op.errors"]),
        w("no_declared_errors null", op("/no_declared_errors"), Value::Null, &["op.errors"]),
        w("no_declared_errors wrong type", op("/no_declared_errors"), json!("yes"), &["op.errors"]),
        w_at("no_declared_errors disagrees with standard", op("/no_declared_errors"), json!(true), op("/errors"), &["op.errors"]),
        // cross_origin.
        remove("cross_origin absent", x("/cross_origin"), clean(&["cross_origin"]))
            .check(api_check(|api| api.cross_origin == CrossOrigin::SameOrigin, "SameOrigin")),
        w("cross_origin null", x("/cross_origin"), Value::Null, &["cross_origin"]),
        w("cross_origin wrong type", x("/cross_origin"), json!(5), &["cross_origin"]),
        w("cross_origin unknown variant", x("/cross_origin"), json!("Anything"), &["cross_origin"]),
        w("cross_origin empty", x("/cross_origin"), json!(""), &["cross_origin"]),
        set("cross_origin empty allow list", x("/cross_origin"), json!({"AllowOrigins": []}), clean(&["cross_origin"]))
            .check(api_check(|api| api.cross_origin == CrossOrigin::AllowOrigins(vec![]), "AllowOrigins([])")),
        w("cross_origin invalid origin", x("/cross_origin"), json!({"AllowOrigins": ["not an origin"]}), &["cross_origin"]),
        remove("op cross_origin absent", op("/cross_origin"), clean(&["op.cross_origin"])),
        w("op cross_origin null", op("/cross_origin"), Value::Null, &["op.cross_origin"]),
        w("op cross_origin unknown variant", op("/cross_origin"), json!("Bogus"), &["op.cross_origin"]),
        w("op cross_origin empty", op("/cross_origin"), json!(""), &["op.cross_origin"]),
    ]);
    cells
}

fn request_cells() -> Vec<Cell> {
    let request = || upload("/request");
    let field = |index: usize, rest: &str| upload(&format!("/request/FormData/fields/{index}{rest}"));
    vec![
        // The standard multipart schema alone yields the control request.
        remove("request absent", request(), clean(&[])),
        w("request null", request(), Value::Null, &[]),
        w("request wrong type", request(), json!(5), &[]),
        w("request unknown variant", request(), json!({"Protobuf": {}}), &[]),
        w("request empty", request(), json!({}), &[]),
        w_removed("request fields absent", upload("/request/FormData/fields"), request(), &[]),
        w("request fields null", upload("/request/FormData/fields"), Value::Null, &[]),
        w_at("request fields empty", upload("/request/FormData/fields"), json!([]), request(), &[]),
        w_at("request fields wrong element", field(3, ""), json!(5), request(), &[]),
        w_removed("field name absent", field(3, "/name"), request(), &[]),
        w("field name null", field(3, "/name"), Value::Null, &[]),
        w_at("field name empty", field(3, "/name"), json!(""), request(), &[]),
        w_at("field name not a standard part", field(3, "/name"), json!("memo"), request(), &[]),
        w_removed("field kind absent", field(3, "/kind"), request(), &[]),
        w("field kind null", field(3, "/kind"), Value::Null, &[]),
        w_at("field kind unknown", field(3, "/kind"), json!("Blob"), request(), &[]),
        w_at("field kind disagrees", field(3, "/kind"), json!({"File": {"accept": []}}), request(), &[]),
        w_removed("File.accept absent", field(0, "/kind/File/accept"), request(), &[]),
        w("File.accept null", field(0, "/kind/File/accept"), Value::Null, &[]),
        w_at("File.accept wrong element", field(0, "/kind/File/accept"), json!(["image/png", 5]), request(), &[]),
        set("File.accept empty", field(0, "/kind/File/accept"), json!([]), clean(&[])),
        w_at("File.accept disagrees with encoding", field(0, "/kind/File/accept"), json!(["image/gif"]), request(), &[]),
        set("Files.min null", field(1, "/kind/Files/min"), Value::Null, clean(&[])),
        remove("Files.max absent", field(1, "/kind/Files/max"), clean(&[])),
        w_at("Files.min wrong type", field(1, "/kind/Files/min"), json!("one"), request(), &[]),
        w_at("Files.min above max", field(1, "/kind/Files/min"), json!(9), request(), &[]),
        w_at("Files.max disagrees with maxItems", field(1, "/kind/Files/max"), json!(3), request(), &[]),
        w("Json reference null", field(2, "/kind/Json"), Value::Null, &[]),
        w_at("Json reference wrong type", field(2, "/kind/Json"), json!(5), request(), &[]),
        w_at("Json reference unresolved", field(2, "/kind/Json"), json!({"Named": "Missing"}), request(), &[])
            .adopted("R12: the standard part already names a schema, so the disagreeing reference is W and standard wins"),
        w_removed("field required absent", field(0, "/required"), request(), &[]),
        w("field required null", field(0, "/required"), Value::Null, &[]),
        w_at("field required wrong type", field(0, "/required"), json!("yes"), request(), &[]),
        w_at("field required disagrees", field(0, "/required"), json!(false), request(), &[]),
        remove("field description absent", field(0, "/description"), clean(&[])),
        set("field description null", field(0, "/description"), Value::Null, clean(&[])),
        w_at("field description wrong type", field(0, "/description"), json!(5), request(), &[]),
        set("field description empty", field(0, "/description"), json!(""), clean(&[])),
        w_at("field description disagrees", field(3, "/description"), json!("Another note"), request(), &[]),
    ]
}

fn response_cells() -> Vec<Cell> {
    let success = || stream("/success");
    let shape = |index: usize, rest: &str| stream(&format!("/success/{index}{rest}"));
    let sse = |rest: &str| shape(1, &format!("/shape/Stream{rest}"));
    // A rejected `success` never falls back to the legacy `response`.
    let mut cells: Vec<Cell> = vec![
        // The legacy `response` holds only the first shape, so it does not
        // reproduce the standard responses and is dropped without a WARN.
        remove("success absent", success(), clean(&["op.success"])),
        w("success null", success(), Value::Null, &["op.success"]),
        w("success wrong type", success(), json!({}), &["op.success"]),
        w("success empty", success(), json!([]), &["op.success"]),
        w_at("success wrong element", shape(1, ""), json!(5), success(), &["op.success"]),
        w_at("success all wrong elements", success(), json!([5, "x"]), success(), &["op.success"]),
        w_removed("success shape absent", shape(0, "/shape"), success(), &["op.success"]),
        w("success shape null", shape(0, "/shape"), Value::Null, &["op.success"]),
        w_at("success shape unknown", shape(0, "/shape"), json!("Nope"), success(), &["op.success"]),
        w_at("success disagrees with standard (R12)", shape(0, "/shape"), json!("Text"), success(), &["op.success"]),
        w_removed("Stream.framing absent", sse("/framing"), success(), &["op.success"]),
        w("Stream.framing null", sse("/framing"), Value::Null, &["op.success"]),
        w_at("Stream.framing unknown", sse("/framing"), json!({"Lines": {"Named": "Chunk"}}), success(), &["op.success"]),
        w("Stream.framing.Sse null", sse("/framing/Sse"), Value::Null, &["op.success"]),
        w_at("Stream.framing.Sse wrong type", sse("/framing/Sse"), json!(5), success(), &["op.success"]),
        w_at("Stream.framing.Sse unresolved", sse("/framing/Sse"), json!({"Named": "Missing"}), success(), &["op.success"])
            .adopted("R12: the re-exported `success` disagrees with the standard responses, so W and standard wins"),
        w_removed("Stream.end absent", sse("/end"), success(), &["op.success"]),
        w("Stream.end null", sse("/end"), Value::Null, &["op.success"]),
        w_at("Stream.end unknown", sse("/end"), json!("Never"), success(), &["op.success"]),
        w_at("Stream.end empty", sse("/end"), json!({}), success(), &["op.success"]),
        w("DataLineEquals null", sse("/end/DataLineEquals"), Value::Null, &["op.success"]),
        w_at("DataLineEquals wrong type", sse("/end/DataLineEquals"), json!(5), success(), &["op.success"]),
        set("DataLineEquals empty", sse("/end/DataLineEquals"), json!(""), clean(&["op.success"])).check(|result| {
            match &endpoint(&result.api, "Stream").response[1].shape {
                schematic_define::ResponseShape::Stream { end: StreamEnd::DataLineEquals(data), .. } if data.is_empty() => Ok(()),
                other => Err(format!("expected an empty DataLineEquals, got {other:?}")),
            }
        }),
        w_at("EventNamed empty", sse("/end"), json!({"EventNamed": ""}), success(), &["op.success"]),
        w("EventNamed null", sse("/end/EventNamed"), Value::Null, &["op.success"]),
        w_at("FieldEquals field empty", sse("/end"), json!({"FieldEquals": {"field": "", "value": "done"}}), success(), &["op.success"]),
        w_at("FieldEquals value array", sse("/end"), json!({"FieldEquals": {"field": "kind", "value": [1]}}), success(), &["op.success"]),
        set("Stream.keep absent is false", sse("/keep"), json!(false), clean(&[])),
        w("Stream.keep null", sse("/keep"), Value::Null, &["op.success"]),
        w_at("Stream.keep wrong type", sse("/keep"), json!("yes"), success(), &["op.success"]),
        w("content_type null", shape(0, "/content_type"), Value::Null, &["op.success"]),
        w_at("content_type wrong type", shape(0, "/content_type"), json!(5), success(), &["op.success"]),
        w_at("content_type empty", shape(0, "/content_type"), json!(""), success(), &["op.success"]),
        w_at("content_type disagrees with standard", shape(0, "/content_type"), json!("application/vnd.item+json"), success(), &["op.success"]),
        w("statuses null", shape(0, "/statuses"), Value::Null, &["op.success"]),
        w_at("statuses not 2xx", shape(0, "/statuses"), json!([404]), success(), &["op.success"]),
        set("statuses empty", shape(0, "/statuses"), json!([]), clean(&[])),
        w("json_tag null", shape(0, "/json_tag"), Value::Null, &["op.success"]),
        w_at("json_tag unknown", shape(0, "/json_tag"), json!("Scalar"), success(), &["op.success"]),
        w_at("json_tag field value object", shape(0, "/json_tag"), json!({"Field": {"name": "k", "value": {}}}), success(), &["op.success"]),
        w_at("Bytes framing with a terminator", sse("/framing"), json!("Bytes"), success(), &["op.success"]),
        w_at("more than one fallback", success(), json!([
            {"shape": {"Json": {"Named": "Item"}}, "fallback": true},
            {"shape": {"Stream": {"framing": {"Sse": {"Named": "Chunk"}}, "end": {"DataLineEquals": "[DONE]"}}}, "fallback": true},
        ]), success(), &["op.success"]),
        set("json_tag Object", shape(0, "/json_tag"), json!("Object"), clean(&["op.success"])),
        w("fallback null", shape(0, "/fallback"), Value::Null, &["op.success"]),
        w_at("fallback wrong type", shape(0, "/fallback"), json!("yes"), success(), &["op.success"]),
        set("fallback false", shape(0, "/fallback"), json!(false), clean(&[])),
        w("note null", shape(0, "/note"), Value::Null, &["op.success"]),
        w_at("note wrong type", shape(0, "/note"), json!(5), success(), &["op.success"]),
        set("note empty", shape(0, "/note"), json!(""), clean(&["op.success"])),
        w("variant null", shape(0, "/variant"), Value::Null, &["op.success"]),
        w_at("variant invalid identifier", shape(0, "/variant"), json!("not ident"), success(), &["op.success"]),
        w_at("variant empty", shape(0, "/variant"), json!(""), success(), &["op.success"]),
    ];
    cells.extend([
        // Legacy `response` (on `GetItem`, beside no `success`).
        remove("response absent", op("/response"), clean(&[])),
        w("response null", op("/response"), Value::Null, &[]),
        w("response wrong type", op("/response"), json!(5), &[]),
        w("response empty", op("/response"), json!(""), &[]),
    ]);
    cells
}

fn operation_cells() -> Vec<Cell> {
    vec![
        w("operation x-schematic not an object", op(""), json!([]), &[
            "op.headers", "op.rate_limit", "op.retry_guidance", "op.errors", "op.cross_origin", "op.auth", "op.env_mapping", "op.params",
        ]),
        remove("path_template absent", files("/path_template"), clean(&["op.path_template"])),
        w("path_template null", files("/path_template"), Value::Null, &["op.path_template"]),
        w("path_template wrong type", files("/path_template"), json!(5), &["op.path_template"]),
        w("path_template empty", files("/path_template"), json!(""), &["op.path_template"]),
        w("path_template other path", files("/path_template"), json!("/blobs/{+path}"), &["op.path_template"]),
        remove("params absent", op("/params"), clean(&["op.params"])),
        w("params null", op("/params"), Value::Null, &["op.params"]),
        w("params wrong type", op("/params"), json!([]), &["op.params"]),
        set("params empty", op("/params"), json!({}), clean(&["op.params"])),
        set(
            "params unknown location",
            op("/params/body"),
            json!({"x": {"secret": true}}),
            Expect::Clean { absent: &[], infos: vec![loc(&op("/params/body"))] },
        ),
        w("params entry null", op("/params/query/api_token"), Value::Null, &["op.params"]),
        w("params entry wrong type", op("/params/query/api_token"), json!(true), &["op.params"]),
        w("params entry empty", op("/params/query/api_token"), json!({}), &["op.params"]),
        w("params entry names no standard parameter", op("/params/query/nope"), json!({"secret": true}), &[]),
        set(
            "params entry unknown key",
            op("/params/query/api_token/masked"),
            json!(true),
            Expect::Clean { absent: &[], infos: vec![loc(&op("/params/query/api_token/masked"))] },
        ),
        w("params secret null", op("/params/query/api_token/secret"), Value::Null, &["op.params"]),
        w("params secret wrong type", op("/params/query/api_token/secret"), json!("yes"), &["op.params"]),
        w("params rust_name null", op("/params/path/item_id/rust_name"), Value::Null, &["op.params"]),
        w("params rust_name wrong type", op("/params/path/item_id/rust_name"), json!(5), &["op.params"]),
        w("params rust_name empty", op("/params/path/item_id/rust_name"), json!(""), &["op.params"]),
        w("params rust_name invalid", op("/params/path/item_id/rust_name"), json!("item-id"), &["op.params"]),
    ]
}

fn table() -> Vec<Cell> {
    let mut cells = version_cells();
    for group in [scalar_cells(), env_cells(), header_cells(), auth_cells(), metadata_cells(), error_and_origin_cells(), request_cells(), response_cells(), operation_cells()] {
        cells.extend(group);
    }
    cells
}

#[test]
fn control_row_imports_every_column_without_problems() {
    let control = control_api();
    for format in [Format::Json, Format::Yaml] {
        let text = format.text(&control_document());
        let failures = check_outcome(format, &text, &clean(&[]), None, &control);
        assert!(failures.is_empty(), "{format:?}: {failures:?}");
    }
}

#[test]
fn extension_matrix_json() {
    walk(Format::Json, &control_document(), &control_api(), &table());
}

#[test]
fn extension_matrix_yaml() {
    walk(Format::Yaml, &control_document(), &control_api(), &table());
}

/// An unresolved `Json` reference over an untyped standard JSON part is
/// ERROR (reference closure) and the part stays untyped; two mutations, so
/// outside the one-place table.
#[test]
fn unresolved_json_reference_over_an_untyped_part_is_error() {
    let mut document = control_document();
    mutate(&mut document, "/paths/~1uploads/post/requestBody/content/multipart~1form-data/schema/properties/meta", Some(json!({})));
    mutate(&mut document, &upload("/request/FormData/fields/2/kind/Json"), Some(json!({"Named": "Missing"})));
    let untyped = |result: &schematic_define::openapi::OpenApiImportResult| match &endpoint(&result.api, "Upload").request {
        Some(schematic_define::ApiRequest::FormData { fields }) => match fields.iter().find(|field| field.name == "meta") {
            Some(field) if field.kind == FormFieldKind::Json(TypeRef::Primitive(PrimitiveType::Json)) => Ok(()),
            other => Err(format!("meta part {other:?}")),
        },
        other => Err(format!("request {other:?}")),
    };
    let control = control_api();
    for format in [Format::Json, Format::Yaml] {
        let expect = Expect::ErrorDiagnostic { at: loc(&upload("/request")), warns: vec![] };
        let failures = check_outcome(format, &format.text(&document), &expect, Some(&untyped), &control);
        assert!(failures.is_empty(), "{format:?}: {failures:?}");
    }
}

#[test]
fn strict_import_passes_an_info_only_document() {
    let mut document = control_document();
    mutate(&mut document, &x("/future_member"), Some(json!(1)));
    for format in [Format::Json, Format::Yaml] {
        let result = import_strict(format, &format.text(&document)).expect("INFO alone passes strict");
        assert_eq!(infos(&result), [loc(&x("/future_member"))], "{format:?}");
    }
}

// Document-shape rows: what a generic `Value` conversion would hide.

fn assert_build_error(format: Format, text: &str, what: &str) {
    let error = import_text(format, text).err().unwrap_or_else(|| panic!("{format:?} {what}: imported"));
    assert!(import_strict(format, text).is_err(), "{format:?} {what}: strict imported");
    assert!(error.contains("line") || error.contains("at "), "{format:?} {what}: error has no location: {error}");
}

fn json_text() -> String {
    Format::Json.text(&control_document())
}

fn yaml_text() -> String {
    Format::Yaml.text(&control_document())
}

/// Inserts `line` after the first line of `text` containing `anchor`.
fn after_line(text: &str, anchor: &str, line: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let index = lines.iter().position(|candidate| candidate.contains(anchor)).unwrap_or_else(|| panic!("no line with {anchor}"));
    let indent: String = lines[index].chars().take_while(|c| *c == ' ').collect();
    lines.insert(index + 1, format!("{indent}{line}"));
    lines.join("\n")
}

#[test]
fn json_duplicate_key_inside_x_schematic_fails() {
    let text = after_line(&json_text(), "\"module_path\": \"fixture_api\"", "\"module_path\": \"other\",");
    assert_build_error(Format::Json, &text, "duplicate x-schematic key");
}

#[test]
fn yaml_duplicate_key_inside_x_schematic_fails() {
    let text = after_line(&yaml_text(), "module_path: fixture_api", "module_path: other");
    assert_build_error(Format::Yaml, &text, "duplicate x-schematic key");
}

#[test]
fn json_duplicate_key_inside_a_standard_field_fails() {
    let text = after_line(&json_text(), "\"title\": \"Fixture\"", "\"title\": \"Other\",");
    assert_build_error(Format::Json, &text, "duplicate info key");
}

#[test]
fn yaml_duplicate_key_inside_a_standard_field_fails() {
    let text = after_line(&yaml_text(), "title: Fixture", "title: Other");
    assert_build_error(Format::Yaml, &text, "duplicate info key");
}

#[test]
fn trailing_content_after_a_valid_document_fails() {
    assert_build_error(Format::Json, &format!("{}\n{{\"x\": 1}}", json_text()), "trailing JSON value");
    assert_build_error(Format::Json, &format!("{} garbage", json_text()), "trailing JSON garbage");
    assert_build_error(Format::Yaml, &format!("{}\n]]] garbage", yaml_text()), "trailing YAML garbage");
}

/// Adopted: E without a location, since `serde_yaml_ng`'s error names no
/// line or pointer ("deserializing from YAML containing more than one
/// document is not supported").
#[test]
fn a_second_yaml_document_fails() {
    let text = format!("{}\n---\nopenapi: 3.0.3\n", yaml_text());
    let Err(error) = import_text(Format::Yaml, &text) else { panic!("a second document imported") };
    assert!(import_strict(Format::Yaml, &text).is_err());
    assert!(error.contains("more than one document"), "{error}");
    assert!(!error.contains("line"), "the error now carries a location; assert it instead: {error}");
}

#[test]
fn yaml_bare_key_is_null_at_a_warn_position() {
    let text = yaml_text().replace("module_path: fixture_api", "module_path:");
    assert_ne!(text, yaml_text());
    let failures = check_outcome(Format::Yaml, &text, &warn(loc(&x("/module_path")), &["module_path"]), None, &control_api());
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn every_probe_is_load_bearing() {
    // A probe that holds on a document without any `x-schematic` would not
    // detect a lost member.
    let mut document = control_document();
    mutate(&mut document, &x(""), Some(json!(2)));
    let result = import_text(Format::Json, &Format::Json.text(&document)).unwrap();
    let control = control_api();
    let holding: Vec<&str> =
        PROBES.iter().filter(|probe| probe.extension && (probe.holds)(&result.api, &control)).map(|probe| probe.name).collect();
    assert!(holding.is_empty(), "extension probes that hold without x-schematic: {holding:?}");
}

