//! Repeatable normalization of query-bearing provider paths before operation mapping.

use std::collections::{HashMap, HashSet};
use openapiv3::{OpenAPI, ReferenceOr};
use serde_json::{Value, json};
use super::diagnostics::OpenApiDiagnostic;
use crate::openapi::OpenApiError;

fn invalid(path: &str, message: impl Into<String>) -> OpenApiError {
    OpenApiError::Validation { path: path.to_string(), message: message.into() }
}

pub(super) fn normalize(doc: &mut OpenAPI, diagnostics: &mut Vec<OpenApiDiagnostic>) -> Result<(), OpenApiError> {
    let original = serde_json::to_value(&*doc).map_err(|e| invalid("paths", e.to_string()))?;
    let mut normalized = original.clone();
    let paths = normalized["paths"].as_object_mut().ok_or_else(|| invalid("paths", "Expected paths object"))?;
    let query_paths: Vec<_> = paths.keys().filter(|p| p.contains('?')).cloned().collect();
    let mut changed: HashMap<(String, String), openapiv3::Operation> = HashMap::new();
    for path in query_paths {
        let (base, query) = path.split_once('?').unwrap();
        let mut item = paths.remove(&path).unwrap();
        let object = item.as_object_mut().ok_or_else(|| invalid(&path, "Expected path item"))?;
        let inherited = object.get("parameters").cloned().unwrap_or_else(|| json!([]));
        for method in ["get", "post", "put", "delete", "patch", "head", "options", "trace"] {
            let Some(operation) = object.get_mut(method) else { continue };
            let id = operation["operationId"].as_str().unwrap_or(method).to_string();
            let context = format!("{} {id} {} {path}", doc.info.title, method.to_uppercase());
            let parameters = parse_query(query, &context)?;
            let op = operation.as_object_mut().ok_or_else(|| invalid(&context, "Expected operation object"))?;
            let mut declarations = inherited.as_array().cloned().ok_or_else(|| invalid(&context, "Expected parameters array"))?;
            declarations.extend(op.get("parameters").and_then(Value::as_array).cloned().unwrap_or_default());
            for declaration in &mut declarations {
                let mut depth = 0;
                while let Some(reference) = declaration.get("$ref") {
                    let reference = reference.as_str().and_then(|r| r.strip_prefix('#')).ok_or_else(|| invalid(&context, "Parameter reference must be a local string reference"))?;
                    depth += 1;
                    if depth > super::MAX_NESTING_DEPTH { return Err(invalid(&context, "Cyclic parameter reference")); }
                    *declaration = original.pointer(reference).cloned().ok_or_else(|| invalid(&context, "Unresolved parameter reference"))?;
                }
            }
            let mut rust_names = serde_json::Map::new();
            for (wire_name, value) in &parameters {
                let placeholder = value.strip_prefix('{').and_then(|v| v.strip_suffix('}'));
                let mut declaration = if let Some(name) = placeholder {
                    let index = declarations.iter().position(|p| p["name"] == name && p["in"] == "path")
                        .ok_or_else(|| invalid(&context, format!("Unsupported query placeholder {{{name}}}: missing path parameter")))?;
                    let mut declaration = declarations.remove(index);
                    declaration["name"] = json!(wire_name);
                    declaration["in"] = json!("query");
                    declaration["style"] = json!("form");
                    rust_names.insert(wire_name.clone(), json!(name));
                    declaration
                } else {
                    let kind = if value == "true" || value == "false" { "boolean" }
                        else if value.parse::<i64>().is_ok() { "integer" } else { "string" };
                    json!({"name":wire_name,"in":"query","required":false,"schema":{"type":kind}})
                };
                // Constants are caller controls, never pinned defaults or singleton enums.
                if placeholder.is_none() { declaration["required"] = json!(false); }
                let typed: openapiv3::Parameter = serde_json::from_value(declaration).map_err(|e| invalid(&context, e.to_string()))?;
                let declaration = serde_json::to_value(typed).map_err(|e| invalid(&context, e.to_string()))?;
                if let Some(existing) = declarations.iter().find(|p| p["name"] == wire_name.as_str() && p["in"] == "query") {
                    if existing.get("required").and_then(Value::as_bool).unwrap_or(false)
                        || !compatible(&existing["schema"], &declaration["schema"], &original, &mut HashSet::new(), 0) {
                        return Err(invalid(&context, format!("Conflicting query parameter {wire_name}")));
                    }
                } else { declarations.push(declaration); }
            }
            op.insert("parameters".to_string(), json!(declarations));
            if !rust_names.is_empty() { op.insert("x-schematic-query-rust-names".to_string(), Value::Object(rust_names)); }
            if let Some(twin) = paths.get_mut(base).and_then(|p| p.get_mut(method)) {
                let mut merged = twin.clone();
                let mut seen = HashSet::new();
                for field in ["requestBody", "responses"] {
                    let ordinary = twin.get(field).cloned().unwrap_or(Value::Null);
                    let candidate = operation.get(field).cloned().unwrap_or(Value::Null);
                    if !compatible(&candidate, &ordinary, &original, &mut seen, 0) {
                        return Err(invalid(&context, format!("Cannot merge into {base}: {field} schema is not a superset")));
                    }
                    let equal = compatible(&ordinary, &candidate, &original, &mut HashSet::new(), 0);
                    diagnostics.push(OpenApiDiagnostic::info(context.clone(), format!("{field} flattened schema: {}; merged side uses candidate type", if equal { "equal" } else { "candidate superset" })));
                    if !candidate.is_null() { merged[field] = candidate; }
                }
                let mut merged_params = merged.get("parameters").and_then(Value::as_array).cloned().unwrap_or_default();
                for param in operation["parameters"].as_array().unwrap() {
                    if let Some(existing) = merged_params.iter_mut().find(|p| p["name"] == param["name"] && p["in"] == param["in"]) {
                        if !compatible(param, existing, &original, &mut HashSet::new(), 0) { return Err(invalid(&context, format!("Conflicting twin parameter {}", param["name"]))); }
                        *existing = param.clone();
                    } else { merged_params.push(param.clone()); }
                }
                merged["parameters"] = json!(merged_params);
                if let Some(names) = operation.get("x-schematic-query-rust-names") { merged["x-schematic-query-rust-names"] = names.clone(); }
                diagnostics.push(OpenApiDiagnostic::info(context, format!("Merged into {base}; flattened request/response schema gate passed (candidate superset or equal)")));
                *twin = merged;
            } else {
                paths.entry(base.to_string()).or_insert_with(|| json!({}))[method] = operation.clone();
            }
            let source_item = match &doc.paths.paths[&path] { ReferenceOr::Item(item) => item, _ => unreachable!() };
            let source_operation = source_item.iter().find(|(name, _)| *name == method).unwrap().1;
            let ordinary_operation = doc.paths.paths.get(base).and_then(|item| match item {
                ReferenceOr::Item(item) => item.iter().find(|(name, _)| *name == method).map(|(_, operation)| operation),
                _ => None,
            });
            let key = (base.to_string(), method.to_string());
            let mut preserved = changed.remove(&key).unwrap_or_else(|| ordinary_operation.unwrap_or(source_operation).clone());
            preserved.request_body = source_operation.request_body.clone();
            preserved.responses = source_operation.responses.clone();
            preserved.parameters = serde_json::from_value(paths[base][method]["parameters"].clone()).map_err(|e| invalid(base, e.to_string()))?;
            if let Some(names) = paths[base][method].get("x-schematic-query-rust-names") {
                preserved.extensions.insert("x-schematic-query-rust-names".to_string(), names.clone());
            }
            changed.insert(key, preserved);
        }
    }
    let mut rebuilt = indexmap::IndexMap::new();
    for original_path in doc.paths.paths.keys() {
        let base = original_path.split_once('?').map(|(base, _)| base).unwrap_or(original_path);
        if original_path.contains('?') && doc.paths.paths.contains_key(base) { continue; }
        if !rebuilt.contains_key(base) {
            // Re-deserializing an untouched operation sorts media-type maps and can
            // change fallback selection (for example multipart versus application/sdp).
            let mut item = doc.paths.paths.get(base).unwrap_or(&doc.paths.paths[original_path]).clone();
            if let ReferenceOr::Item(item) = &mut item {
                for method in ["get", "post", "put", "delete", "patch", "head", "options", "trace"] {
                    if let Some(operation) = changed.remove(&(base.to_string(), method.to_string())) {
                        match method {
                            "get" => item.get = Some(operation), "post" => item.post = Some(operation),
                            "put" => item.put = Some(operation), "delete" => item.delete = Some(operation),
                            "patch" => item.patch = Some(operation), "head" => item.head = Some(operation),
                            "options" => item.options = Some(operation), "trace" => item.trace = Some(operation),
                            _ => unreachable!(),
                        }
                    }
                }
            }
            rebuilt.insert(base.to_string(), item);
        }
    }
    doc.paths.paths = rebuilt;
    for (path, item) in &doc.paths.paths {
        if path.is_empty() || !path.starts_with('/') { return Err(invalid(path, "Path must start with /")); }
        if let ReferenceOr::Item(item) = item {
            for (method, operation) in item.iter() {
                let id = operation.operation_id.as_deref().unwrap_or(method);
                if method == "get" && operation.request_body.is_some() {
                    return Err(invalid(path, format!("{} endpoint {id} GET cannot declare a request body", doc.info.title)));
                }
                for parameters in [&item.parameters, &operation.parameters] {
                    let mut seen = HashSet::new();
                    for parameter in parameters {
                    if let ReferenceOr::Item(parameter) = parameter {
                        let data = parameter.parameter_data_ref();
                        if data.name.is_empty() { return Err(invalid(path, format!("{id}: parameter name cannot be empty"))); }
                        let location = match parameter { openapiv3::Parameter::Query { .. } => "query", openapiv3::Parameter::Path { .. } => "path", openapiv3::Parameter::Header { .. } => "header", openapiv3::Parameter::Cookie { .. } => "cookie" };
                        if !seen.insert((location, &data.name)) { return Err(invalid(path, format!("{id}: duplicate {location} parameter {}", data.name))); }
                    }
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_query(query: &str, context: &str) -> Result<Vec<(String, String)>, OpenApiError> {
    let mut result = Vec::new();
    let mut names = HashSet::new();
    for part in query.split('&') {
        let (name, value) = part.split_once('=').ok_or_else(|| invalid(context, "Malformed query expression: expected name=value"))?;
        if name.is_empty() || value.is_empty() || name.contains(['{', '}', '#', '?']) || value.contains(['#', '?', '=']) {
            return Err(invalid(context, "Malformed query expression"));
        }
        for encoded in [name, value] {
            let bytes = encoded.as_bytes();
            for (index, byte) in bytes.iter().enumerate() {
                if *byte == b'%' && (index + 2 >= bytes.len() || !bytes[index + 1].is_ascii_hexdigit() || !bytes[index + 2].is_ascii_hexdigit()) {
                    return Err(invalid(context, "Malformed percent escape in query"));
                }
            }
        }
        if value.contains(['{', '}']) && !(value.starts_with('{') && value.ends_with('}') && !value[1..value.len()-1].contains(['{', '}'])) {
            return Err(invalid(context, "Unsupported query placeholder"));
        }
        let pair = url::form_urlencoded::parse(part.as_bytes()).next().unwrap();
        if !names.insert(pair.0.to_string()) { return Err(invalid(context, format!("Duplicate query key {}", pair.0))); }
        result.push((pair.0.into_owned(), pair.1.into_owned()));
    }
    Ok(result)
}

// Compare wire schemas after resolving refs and flattening allOf, not Rust names.
// Annotation-only differences do not affect the set of representable values.
fn flatten(value: &Value, document: &Value, depth: usize) -> Value {
    if depth > 64 { return value.clone(); }
    if let Some(reference) = value.get("$ref").and_then(Value::as_str).and_then(|r| r.strip_prefix('#'))
        && let Some(target) = document.pointer(reference) { return flatten(target, document, depth + 1); }
    let mut result = value.clone();
    if let Some(parts) = result.as_object_mut().and_then(|v| v.remove("allOf")) {
        for part in parts.as_array().into_iter().flatten() {
            let part = flatten(part, document, depth + 1);
            if let Some(properties) = part.get("properties").and_then(Value::as_object) {
                if result.get("properties").is_none() { result["properties"] = json!({}); }
                result["properties"].as_object_mut().unwrap().extend(properties.clone());
            }
            if let Some(required) = part.get("required").and_then(Value::as_array) {
                if result.get("required").is_none() { result["required"] = json!([]); }
                for name in required { if !result["required"].as_array().unwrap().contains(name) { result["required"].as_array_mut().unwrap().push(name.clone()); } }
            }
            if let Some(object) = part.as_object() {
                for (key, value) in object { if key != "properties" && key != "required" { result[key] = value.clone(); } }
            }
        }
    }
    result
}

fn compatible(candidate: &Value, ordinary: &Value, document: &Value, seen: &mut HashSet<(String, String)>, depth: usize) -> bool {
    if candidate == ordinary { return true; }
    if depth > 128 { return false; }
    if let (Some(c), Some(o)) = (candidate.get("$ref").and_then(Value::as_str), ordinary.get("$ref").and_then(Value::as_str))
        && !seen.insert((c.to_string(), o.to_string())) { return true; }
    let candidate = flatten(candidate, document, 0);
    let ordinary = flatten(ordinary, document, 0);
    match (&candidate, &ordinary) {
        (Value::Object(c), Value::Object(o)) => {
            let ignored = ["description", "title", "example", "examples", "externalDocs", "deprecated", "default", "x-oaiMeta", "x-stainless-const", "x-stainless-skip", "x-stainless-naming", "discriminator"];
            for (key, value) in o {
                if ignored.contains(&key.as_str()) || key.starts_with("x-") { continue; }
                if key == "required" && value.is_array() {
                    let ordinary_required = value.as_array();
                    if c.get(key).and_then(Value::as_array).is_some_and(|required| required.iter().any(|name| !ordinary_required.is_some_and(|v| v.contains(name)))) { return false; }
                } else if ["minimum", "exclusiveMinimum", "minLength", "minItems", "minProperties"].contains(&key.as_str()) && value.is_number() {
                    if c.get(key).is_some_and(|candidate| candidate.as_f64().zip(value.as_f64()).is_none_or(|(c, o)| c > o)) { return false; }
                } else if ["maximum", "exclusiveMaximum", "maxLength", "maxItems", "maxProperties"].contains(&key.as_str()) && value.is_number() {
                    if c.get(key).is_some_and(|candidate| candidate.as_f64().zip(value.as_f64()).is_none_or(|(c, o)| c < o)) { return false; }
                } else if key == "nullable" {
                    if value == &json!(true) && c.get(key) != Some(&json!(true)) { return false; }
                } else if key == "properties" {
                    let Some(properties) = c.get(key).and_then(Value::as_object) else { return false };
                    let Some(ordinary_properties) = value.as_object() else { return false };
                    for (name, schema) in ordinary_properties {
                        if !properties.get(name).is_some_and(|candidate| compatible(candidate, schema, document, seen, depth + 1)) { return false; }
                    }
                } else if key == "additionalProperties" {
                    match (c.get(key), value) {
                        (None | Some(Value::Bool(true)), _) => {},
                        (Some(Value::Bool(false)), Value::Bool(false)) => {},
                        (Some(candidate @ Value::Object(_)), ordinary @ Value::Object(_)) => {
                            if !compatible(candidate, ordinary, document, seen, depth + 1) { return false; }
                        }
                        _ => return false,
                    }
                } else if key == "enum" {
                    if let Some(variants) = c.get(key).and_then(Value::as_array)
                        && !value.as_array().is_some_and(|base| base.iter().all(|v| variants.contains(v))) { return false; }
                } else if key == "anyOf" || key == "oneOf" {
                    let Some(variants) = c.get(key).and_then(Value::as_array) else { return false };
                    if !value.as_array().is_some_and(|base| base.iter().all(|v| variants.iter().any(|variant| compatible(variant, v, document, &mut seen.clone(), depth + 1)))) { return false; }
                } else if !c.get(key).is_some_and(|v| compatible(v, value, document, seen, depth + 1)) { return false; }
            }
            let schema = c.get("type").is_some_and(Value::is_string) || o.get("type").is_some_and(Value::is_string)
                || c.get("properties").is_some_and(Value::is_object) || o.get("properties").is_some_and(Value::is_object)
                || c.get("anyOf").is_some_and(Value::is_array) || o.get("anyOf").is_some_and(Value::is_array);
            for (key, value) in c {
                if o.contains_key(key) || ignored.contains(&key.as_str()) || key.starts_with("x-") || key == "properties" { continue; }
                if key == "required" && (value == &json!(false) || value.as_array().is_some_and(Vec::is_empty)) { continue; }
                if key == "nullable" || (key == "additionalProperties" && value == &json!(true)) { continue; }
                if schema { return false; }
                if key == "required" && value == &json!(true) { return false; }
            }
            // A candidate cannot make formerly optional fields required.
            if c.get("required").and_then(Value::as_array).is_some_and(|required| required.iter().any(|name| !o.get("required").and_then(Value::as_array).is_some_and(|v| v.contains(name)))) { return false; }
            true
        }
        _ => candidate == ordinary,
    }
}

/// Reject duplicate authored keys before serde's maps can discard them, and
/// distinguish omitted parameter declarations from explicit null.
pub(super) fn check_text(content: &str, json: bool) -> Result<(), OpenApiError> {
    let value: UniqueValue = if json {
        serde_json::from_str(content).map_err(|error| OpenApiError::Parse { message: error.to_string(), location: None })?
    } else {
        biscuit_file::serde_yaml_ng::from_str(content).map_err(|error| OpenApiError::Parse { message: error.to_string(), location: None })?
    };
    // `openapiv3` reads a null `security` as absent and YAML `version: ~` as
    // the string "~"; both are malformed, not omissions.
    if value.0.get("security").is_some_and(|security| !security.is_array()) {
        return Err(invalid("#/security", "Expected an array of security requirements; null is not omission"));
    }
    if value.0.get("info").and_then(|info| info.get("version")).is_some_and(Value::is_null) {
        return Err(invalid("#/info/version", "Expected a version string; null is not a version"));
    }
    if let Some(paths) = value.0.get("paths") {
        let paths = paths.as_object().ok_or_else(|| invalid("paths", "Expected paths object"))?;
        for (path, item) in paths {
            if path.is_empty() || !path.starts_with('/') { return Err(invalid(path, "Path must start with /")); }
            let object = item.as_object().ok_or_else(|| invalid(path, "Expected path item object"))?;
            check_parameters(object.get("parameters"), path)?;
            for method in ["get", "post", "put", "delete", "patch", "head", "options", "trace"] {
                if let Some(operation) = object.get(method) {
                    let operation = operation.as_object().ok_or_else(|| invalid(path, "Expected operation object"))?;
                    if method == "get" && operation.contains_key("requestBody") {
                        let id = operation.get("operationId").and_then(Value::as_str).unwrap_or(method);
                        let api = value.0["info"]["title"].as_str().unwrap_or("API");
                        return Err(invalid(path, format!("{api} endpoint {id} GET cannot declare a request body")));
                    }
                    check_parameters(operation.get("parameters"), path)?;
                    if operation.get("security").is_some_and(|security| !security.is_array()) {
                        return Err(invalid(path, format!("{method} `security` must be an array of security requirements; null is not omission")));
                    }
                    let id = operation.get("operationId").and_then(Value::as_str).unwrap_or(method);
                    let api = value.0["info"]["title"].as_str().unwrap_or("API");
                    check_responses(operation.get("responses"), &format!("API {api} endpoint {id} {} {path} responses", method.to_uppercase()))?;
                }
            }
        }
    }
    if let Some(responses) = value.0.get("components").and_then(|components| components.get("responses")) {
        let responses = responses.as_object().ok_or_else(|| invalid("components.responses", "Expected responses object"))?;
        for (name, response) in responses {
            check_response(response, &format!("components.responses.{name}"))?;
        }
    }
    Ok(())
}

/// The response members the v2 classification reads, which `openapiv3`
/// would otherwise accept silently: it drops unknown status keys, merges
/// `4XX` with `4xx`, reads a null `schema` or `discriminator` as absent, and
/// keeps an empty `$ref`, `oneOf`, or `propertyName`. Media and status keys
/// are compared normalized, so differently spelled duplicates are rejected.
fn check_responses(responses: Option<&Value>, context: &str) -> Result<(), OpenApiError> {
    let Some(responses) = responses else { return Ok(()) };
    let responses = responses.as_object().ok_or_else(|| invalid(context, "Expected responses object; null is not omission"))?;
    let mut seen = HashSet::new();
    for (key, response) in responses {
        if key.starts_with("x-") { continue; }
        let normalized = key.to_ascii_uppercase();
        let valid = normalized == "DEFAULT"
            || normalized.parse::<u16>().is_ok_and(|code| key.len() == 3 && (100..=599).contains(&code))
            || matches!(normalized.as_bytes(), [b'1'..=b'5', b'X', b'X']);
        if !valid { return Err(invalid(context, format!("Response key `{key}` is not `default`, a status 100-599, or a class 1XX-5XX"))); }
        if !seen.insert(normalized) { return Err(invalid(context, format!("Response key `{key}` duplicates another spelling of the same status"))); }
        check_response(response, &format!("{context}.{key}"))?;
    }
    Ok(())
}

fn check_response(response: &Value, context: &str) -> Result<(), OpenApiError> {
    let response = response.as_object().ok_or_else(|| invalid(context, "Expected response object"))?;
    if let Some(reference) = response.get("$ref") {
        return check_reference(reference, context);
    }
    let Some(content) = response.get("content") else { return Ok(()) };
    let content = content.as_object().ok_or_else(|| invalid(context, "Expected content object; null is not omission"))?;
    let mut seen = HashSet::new();
    for (key, media) in content {
        let normalized = match crate::response::normalize_media_type(key) {
            Ok(normalized) => normalized,
            // Media ranges are well formed but unrepresentable; the importer reports them.
            Err(_) if is_media_range(key) => key.trim().to_ascii_lowercase(),
            Err(message) => return Err(invalid(context, message)),
        };
        if !seen.insert(normalized) { return Err(invalid(context, format!("Media type `{key}` duplicates another spelling of the same media type"))); }
        let context = format!("{context}.content.{key}");
        let media = media.as_object().ok_or_else(|| invalid(&context, "Expected media type object"))?;
        if let Some(schema) = media.get("schema") { check_schema(schema, &format!("{context}.schema"))?; }
    }
    Ok(())
}

fn is_media_range(key: &str) -> bool {
    let key = key.split(';').next().unwrap_or_default().trim();
    key == "*/*" || key.strip_suffix("/*").is_some_and(|kind| !kind.is_empty() && kind.bytes().all(|c| c.is_ascii_alphanumeric() || b"!#$%&'+-.^_`|~".contains(&c)))
}

fn check_reference(reference: &Value, context: &str) -> Result<(), OpenApiError> {
    match reference.as_str() {
        Some(reference) if !reference.is_empty() => Ok(()),
        _ => Err(invalid(context, "`$ref` must be a nonempty string")),
    }
}

fn check_schema(schema: &Value, context: &str) -> Result<(), OpenApiError> {
    let schema = schema.as_object().ok_or_else(|| invalid(context, "Expected schema object; null is not omission"))?;
    if let Some(reference) = schema.get("$ref") { check_reference(reference, context)?; }
    if let Some(members) = schema.get("oneOf") {
        let members = members.as_array().filter(|members| !members.is_empty())
            .ok_or_else(|| invalid(context, "`oneOf` must be a nonempty array"))?;
        for member in members {
            let member = member.as_object().ok_or_else(|| invalid(context, "`oneOf` member must be a schema or reference object"))?;
            if let Some(reference) = member.get("$ref") { check_reference(reference, context)?; }
        }
    }
    if let Some(discriminator) = schema.get("discriminator") {
        let discriminator = discriminator.as_object().ok_or_else(|| invalid(context, "Expected discriminator object; null is not omission"))?;
        if !discriminator.get("propertyName").and_then(Value::as_str).is_some_and(|name| !name.is_empty()) {
            return Err(invalid(context, "`discriminator.propertyName` must be a nonempty string"));
        }
        if let Some(mapping) = discriminator.get("mapping")
            && !mapping.as_object().is_some_and(|mapping| mapping.values().all(|target| target.as_str().is_some_and(|target| !target.is_empty()))) {
            return Err(invalid(context, "`discriminator.mapping` must map values to nonempty reference strings"));
        }
    }
    Ok(())
}

fn check_parameters(parameters: Option<&Value>, path: &str) -> Result<(), OpenApiError> {
    if let Some(parameters) = parameters {
        let parameters = parameters.as_array().ok_or_else(|| invalid(path, "Expected parameters array; null is not omission"))?;
        for parameter in parameters {
            let parameter = parameter.as_object().ok_or_else(|| invalid(path, "Expected parameter object"))?;
            if parameter.contains_key("$ref") { continue; }
            if let Some(schema) = parameter.get("schema").and_then(Value::as_object)
                && let Some(kind) = schema.get("type")
                && !kind.as_str().is_some_and(|kind| ["string", "integer", "number", "boolean", "object", "array"].contains(&kind)) {
                return Err(invalid(path, "Parameter schema type must be a supported nonempty string"));
            }
            for key in ["name", "in"] {
                if !parameter.get(key).and_then(Value::as_str).is_some_and(|s| !s.is_empty()) {
                    return Err(invalid(path, format!("Parameter {key} must be a nonempty string")));
                }
            }
        }
    }
    Ok(())
}

struct UniqueValue(Value);
impl<'de> serde::Deserialize<'de> for UniqueValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueValue;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { formatter.write_str("a document without duplicate mapping keys") }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> { Ok(UniqueValue(json!(value))) }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> { Ok(UniqueValue(json!(value))) }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> { Ok(UniqueValue(json!(value))) }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> { Ok(UniqueValue(json!(value))) }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> { Ok(UniqueValue(json!(value))) }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> { Ok(UniqueValue(json!(value))) }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> { Ok(UniqueValue(Value::Null)) }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> { self.visit_unit() }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<UniqueValue>()? { values.push(value.0); }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut mapping: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = mapping.next_entry::<String, UniqueValue>()? {
                    if values.insert(key.clone(), value.0).is_some() { return Err(serde::de::Error::custom(format!("Duplicate key {key}"))); }
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
