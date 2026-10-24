//! Historical JSON is compared as values before any typed deserialization.
#[path = "../../examples/support/definition_snapshot.rs"]
mod definition_snapshot;

use schematic_define::RestApi;
use schematic_gen::pipeline::{resolve_all_apis, resolve_api};
use serde_json::Value;
use definition_snapshot::{canonical, differences, fingerprint, snapshot};

// Historical exceptions name one exact leaf and its cause. Current artifacts
// always compare without exceptions in artifact_drift.
const EXPLAINED: &[(&str, &str)] = &[
    ("lmstudio/definition/endpoints/4:DownloadModel/description",
        "R31 review: the endpoint returns one JSON download job, not SSE progress, so the description drops \"with SSE progress\""),
];

fn unexplained(changed: Vec<String>, explained: &[(&str, &str)]) -> Vec<String> {
    for (path, cause) in explained {
        assert!(!cause.trim().is_empty(), "missing cause for {path}");
        assert!(!path.contains('*'), "wildcard exception: {path}");
        assert!(changed.iter().any(|change| change == path), "stale exception: {path}");
    }
    changed.into_iter().filter(|path| !explained.iter().any(|(allowed, _)| path == allowed)).collect()
}

// The only APIs whose historical `auth: "Basic"` is lowered to the embedded
// lists (contracts.md, historical comparison normalization).
const BASIC_APIS: &[&str] = &["bitbucket", "emqx-basic"];
// A populated `env_username` on a non-Basic API that no output read: it is
// equivalent only while the same snapshot's explicit `basic_user` names
// exactly that variable, which the current builder also requires.
const UNUSED_USERNAME_APIS: &[&str] = &["unfolded-circle-core-rest"];
// Imported before the importer applied OpenAPI's `explode` default, `true`
// for `form` style: their snapshots record `false` for every form-style query
// and cookie parameter, and the vendored document declares none of them.
const FORM_EXPLODE_DEFAULT_APIS: &[&str] = &["openai"];

// Imported before the importer kept value rules: their snapshots carry no
// parameter, form-field, or body `constraints`, and their registered models
// had no Schemars `extend` rules on fields or roots, so every field carried
// the format Schemars implies for its Rust type and a `serde_json::Value`
// field carried no nested rule objects. The current definition is compared
// with all of these undone (see `without_value_rules`); its rules are checked
// by artifact_drift and the import_overlay re-import test.
const VALUE_RULE_APIS: &[&str] = &["openai"];

/// The keywords `import --definitions-out` adds to a model field through
/// `#[schemars(extend(..))]`; schemars writes none of them on a field of the
/// types OpenAI's models use. `format` is not listed: the rebuild replaces it
/// rather than adding it, and [`schemars_format`] restores it.
const FIELD_RULE_KEYWORDS: &[&str] = &[
    "minLength", "maxLength", "pattern", "minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum",
    "multipleOf", "minItems", "maxItems", "uniqueItems", "enum", "const", "x-schematic-exact",
];

/// `api` as a [`VALUE_RULE_APIS`] snapshot recorded it: no endpoint
/// constraints, and no field rules in any component schema. Other APIs are
/// returned unchanged. `source` is the vendored document a
/// [`VALUE_RULE_APIS`] key was imported from.
fn without_value_rules(key: &str, api: &RestApi, source: Option<&std::path::Path>) -> RestApi {
    let mut api = api.clone();
    if !VALUE_RULE_APIS.contains(&key) {
        return api;
    }
    let source = source.unwrap_or_else(|| panic!("the {key} baseline test passes its vendored document"));
    let free_form = free_form_fields(source);
    for endpoint in &mut api.endpoints {
        endpoint.request_constraints = None;
        if let Some(params) = &mut endpoint.params {
            for param in params.path.iter_mut().chain(&mut params.query).chain(&mut params.header).chain(&mut params.cookie) {
                param.constraints = None;
            }
        }
        if let Some(schematic_define::ApiRequest::FormData { fields } | schematic_define::ApiRequest::UrlEncoded { fields }) = &mut endpoint.request {
            for field in fields {
                field.constraints = None;
            }
        }
    }
    for component in &mut api.schemas {
        let fields = free_form.get(&component.name).cloned().unwrap_or_default();
        strip_field_rules(&mut component.schema, &fields);
    }
    api
}

/// The wire names of each struct's fields that the importer types as
/// free-form JSON (`serde_json::Value`, also emitted for an inline
/// composition), by struct. Schemars writes no composition on such a field,
/// so an `anyOf` or `allOf` there is the rebuild's; on any other field it
/// may be Schemars' own `Option` of a model, identical once registered. An
/// `allOf` model is a struct flattening its members, whose properties
/// Schemars merges, so it has every named member's free-form fields.
fn free_form_fields(source: &std::path::Path) -> std::collections::HashMap<String, std::collections::HashSet<String>> {
    use schematic_define::models::{ModelDef, PrimitiveType, TypeRef};
    use schematic_define::openapi::{OpenApiImport, OpenApiSource};

    let imported = OpenApiImport::new(OpenApiSource::path(source)).prefer_json().build().unwrap();
    let mut fields: std::collections::HashMap<String, std::collections::HashSet<String>> = std::collections::HashMap::new();
    for model in &imported.models.types {
        let ModelDef::Struct(def) = model else { continue };
        let names = def.fields.iter().filter_map(|field| {
            let mut field_type = &field.field_type;
            while let TypeRef::Optional(inner) = field_type {
                field_type = inner;
            }
            matches!(field_type, TypeRef::Primitive(PrimitiveType::Json) | TypeRef::Unknown | TypeRef::OneOf(_) | TypeRef::AnyOf(_) | TypeRef::AllOf(_))
                .then(|| field.serde_rename.clone().unwrap_or_else(|| field.name.clone()))
        });
        fields.insert(def.name.clone(), names.collect());
    }
    let composed: Vec<(&str, Vec<&str>)> = imported
        .models
        .types
        .iter()
        .filter_map(|model| match model {
            ModelDef::Alias(alias) => match &alias.target {
                TypeRef::AllOf(members) => Some((
                    alias.name.as_str(),
                    members.iter().filter_map(|member| match member { TypeRef::Named(name) => Some(name.as_str()), _ => None }).collect(),
                )),
                _ => None,
            },
            _ => None,
        })
        .collect();
    loop {
        let mut grew = false;
        for (name, members) in &composed {
            let merged: std::collections::HashSet<String> = members.iter().filter_map(|member| fields.get(*member)).flatten().cloned().collect();
            let entry = fields.entry((*name).to_owned()).or_default();
            let before = entry.len();
            entry.extend(merged);
            grew |= entry.len() != before;
        }
        if !grew {
            return fields;
        }
    }
}

/// The `format` Schemars writes for the Rust type the importer emits for a
/// schema of JSON `type`: every integer is an `i64`, every number an `f64`
/// (`model_gen.rs`, `base_schema`), and no other emitted type carries one.
fn schemars_format(object: &serde_json::Map<String, Value>) -> Option<&'static str> {
    match object.get("type").and_then(Value::as_str) {
        Some("integer") => Some("int64"),
        Some("number") => Some("double"),
        _ => None,
    }
}

/// Whether `key: child` is a nested rule slot as the rebuild writes it onto an
/// untyped (`serde_json::Value`) field once its rules are stripped: a
/// `properties` map or an `items` or `additionalProperties` schema whose own
/// keys are again only such slots, down to `{}` or a reference to a model
/// with rules (its `$ref`, and `nullable: true` beside it), and the
/// [`rebuilt_wrapper`]s around them. Schemars writes no nested schema
/// beneath an untyped field, so every reference and composition there is
/// the rebuild's.
fn rule_only(key: &str, child: &Value) -> bool {
    match (key, child) {
        ("properties", Value::Object(properties)) => properties.values().all(rule_slot),
        ("items" | "additionalProperties", schema) => rule_slot(schema),
        _ => false,
    }
}

/// Whether `schema` holds only [`rule_only`] slots, a reference leaf, and
/// [`rebuilt_wrapper`]s.
fn rule_slot(schema: &Value) -> bool {
    schema.as_object().is_some_and(|object| {
        let reference = object.get("$ref").is_some_and(Value::is_string);
        object.iter().all(|(key, child)| match key.as_str() {
            "$ref" => reference,
            "nullable" => reference && child == &Value::Bool(true),
            _ => rule_only(key, child) || rebuilt_wrapper(key, child),
        })
    })
}

/// Whether `key: child` is a nullable alternative (a [`rule_slot`] beside a
/// null-only member) or a one-member `allOf` of a rule slot, the two
/// compositions the rebuild keeps on a free-form value.
fn rebuilt_wrapper(key: &str, child: &Value) -> bool {
    let null_only = |member: &Value| {
        member.as_object().is_some_and(|object| {
            object.get("enum").and_then(Value::as_array).is_some_and(|values| !values.is_empty() && values.iter().all(Value::is_null))
                && object.keys().all(|key| matches!(key.as_str(), "enum" | "type" | "nullable"))
        })
    };
    match (key, child.as_array().map(Vec::as_slice)) {
        ("anyOf", Some([first, second])) => (null_only(first) && rule_slot(second)) || (rule_slot(first) && null_only(second)),
        ("allOf", Some([member])) => rule_slot(member),
        _ => false,
    }
}

/// Removes [`FIELD_RULE_KEYWORDS`] from every property schema below
/// `schema`, from its `items`, and from the members of a nullable `anyOf`.
/// An `enum` of only `null` is schemars' own null branch and stays. On the
/// same schemas an existing `format` is reset to [`schemars_format`] (or
/// removed where it implies none), and an untyped schema without `$ref`
/// loses the `properties`, `items`, and `additionalProperties` that are
/// [`rule_only`] once stripped. A [`rebuilt_wrapper`] is removed there too
/// where Schemars writes no composition: on the items of an array and on a
/// top-level property named in `free_form` (see [`free_form_fields`]).
fn strip_field_rules(schema: &mut Value, free_form: &std::collections::HashSet<String>) {
    fn strip_property(property: &mut Value, composed: bool) {
        if let Some(object) = property.as_object_mut() {
            object.retain(|key, value| {
                let null_branch = key == "enum" && value.as_array().is_some_and(|values| values.iter().all(Value::is_null));
                null_branch || !FIELD_RULE_KEYWORDS.contains(&key.as_str())
            });
            if object.contains_key("format") {
                match schemars_format(object) {
                    Some(format) => { object.insert("format".into(), Value::from(format)); }
                    None => { object.remove("format"); }
                }
            }
            if let Some(items) = object.get_mut("items") {
                strip_property(items, true);
            }
            for composition in ["anyOf", "allOf"] {
                if let Some(Value::Array(members)) = object.get_mut(composition) {
                    members.iter_mut().for_each(|member| strip_property(member, false));
                }
            }
        }
        strip_field_rules(property, &Default::default());
        if let Some(object) = property.as_object_mut()
            && !object.contains_key("type") && !object.contains_key("$ref") {
            object.retain(|key, child| !rule_only(key, child) && !(composed && rebuilt_wrapper(key, child)));
        }
    }
    match schema {
        Value::Object(object) => {
            for (key, child) in object.iter_mut() {
                match (key.as_str(), child) {
                    ("properties", Value::Object(properties)) => {
                        properties.iter_mut().for_each(|(name, property)| strip_property(property, free_form.contains(name)));
                    }
                    (_, child) => strip_field_rules(child, &Default::default()),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| strip_field_rules(item, &Default::default())),
        _ => {}
    }
}

/// Deliberate definition adoptions made after the snapshots were taken: one
/// API, one top-level definition field, and the exact value adopted. A
/// snapshot that already has the field, or a current value that differs, is
/// still reported at its own leaf.
fn adoptions() -> [(&'static str, &'static str, Value); 1] {
    [(
        "github",
        "rate_limit",
        serde_json::json!({
            "limit_header": "x-ratelimit-limit",
            "remaining_header": "x-ratelimit-remaining",
            "reset_header": "x-ratelimit-reset",
            "reset_unit": "epoch_seconds",
            "retry_after_header": "retry-after"
        }),
    )]
}

/// Deliberate response-model migrations made after the snapshots were taken:
/// one API, one endpoint, and why. Only that endpoint's `response` and
/// `errors` and its fingerprint may differ; every entry must still differ.
const RESPONSE_MIGRATIONS: &[(&str, &str, &str)] = &[
    ("elevenlabs", "StreamSpeech", "R33: an incremental byte stream instead of a buffered binary body"),
    ("elevenlabs", "StreamSpeechWithTimestamps", "R33: one JSON object per line (NDJSON), read incrementally"),
    ("ollama-native", "Generate", "R30: NDJSON chunks ending at a kept done=true line, or with `stream` false the JSON completion"),
    ("ollama-native", "Chat", "R30: NDJSON chunks ending at a kept done=true line, or with `stream` false the JSON completion"),
    ("ollama-native", "PullModel", "R30: NDJSON progress ending at a kept status=success line, or with `stream` false the JSON status"),
    ("ollama-native", "PushModel", "R30: NDJSON progress ending at a kept status=success line, or with `stream` false the JSON status"),
    ("ollama-native", "CreateModel", "R30: NDJSON progress ending at a kept status=success line, or with `stream` false the JSON status"),
    ("ollama-openai", "ChatCompletions", "R30: the JSON completion, or with `stream` true SSE chunks ending at data [DONE]"),
    ("ollama-openai", "Completions", "R30: the JSON completion, or with `stream` true SSE chunks ending at data [DONE]"),
    ("anthropic", "CreateMessage", "R32: the JSON message, or with `stream` true the SSE Messages events ending at message_stop"),
    ("lmstudio", "Chat", "R31: the JSON chat result, or with `stream` true the SSE chat events ending at a kept chat.end"),
    ("lmstudio", "DownloadModel", "R31 review: one JSON download job (the provider documents no stream), no longer binary"),
    ("bitbucket", "GetSource", "R32a: a JSON directory listing (Directory) or the file's bytes as the binary fallback (File)"),
    ("openai", "GetResponse", "R32b: a JSON reply or, with the stream query parameter, server-sent response events"),
    ("openai", "CreateRealtimeCall", "R28 import: the application/sdp answer is a binary body; the earlier importer \
        did not recognize the media type and declared no body"),
];

/// Components the response-model migrations register or correct: one API, one
/// component, and why. They are compared nowhere else; every entry must name
/// a current component.
const MIGRATED_COMPONENTS: &[(&str, &str, &str)] = &[
    ("ollama-native", "OpenAIChatCompletionChoice", "R30: a choice of the JSON chat completion"),
    ("ollama-native", "OpenAIChatCompletionResponse", "R30: the JSON reply of ChatCompletions"),
    ("ollama-native", "OpenAIDelta", "R30: the delta of a chat stream choice"),
    ("ollama-native", "OpenAIStreamChoice", "R30: a choice of a chat stream chunk"),
    ("ollama-native", "OpenAIChatStreamChunk", "R30: the SSE item of ChatCompletions"),
    ("ollama-native", "OpenAICompletionChoice", "R30: a choice of a completion"),
    ("ollama-native", "OpenAICompletionResponse", "R30: the JSON reply of Completions"),
    ("ollama-native", "OpenAICompletionStreamChunk", "R30: the SSE item of Completions"),
    ("ollama-native", "StreamErrorLine", "R11c: the typed mid-stream error line"),
    ("ollama-native", "GenerateStreamChunk", "R30: a Generate NDJSON chunk"),
    ("ollama-native", "GenerateStreamItem", "R30: the Generate NDJSON item: chunk or error line"),
    ("ollama-native", "GenerateCompletion", "R30: the buffered Generate reply (renamed from GenerateResponse, the D-15 enum name)"),
    ("ollama-native", "ChatStreamChunk", "R30: a Chat NDJSON chunk"),
    ("ollama-native", "ChatStreamItem", "R30: the Chat NDJSON item: chunk or error line"),
    ("ollama-native", "ChatCompletion", "R30: the buffered Chat reply (renamed from ChatResponse, the D-15 enum name)"),
    ("ollama-native", "PullModelProgress", "R30: a pull or push progress line"),
    ("ollama-native", "PullModelStreamItem", "R30: the pull/push NDJSON item: progress or error line"),
    ("ollama-native", "CreateModelProgress", "R30: a create progress line"),
    ("ollama-native", "CreateModelStreamItem", "R30: the create NDJSON item: progress or error line"),
    ("ollama-native", "StatusResponse", "R30: the buffered pull/push/create reply"),
    ("ollama-openai", "OpenAIChatCompletionChoice", "R30: a choice of the JSON chat completion"),
    ("ollama-openai", "OpenAIChatCompletionResponse", "R30: the JSON reply of ChatCompletions"),
    ("ollama-openai", "OpenAIDelta", "R30: the delta of a chat stream choice"),
    ("ollama-openai", "OpenAIStreamChoice", "R30: a choice of a chat stream chunk"),
    ("ollama-openai", "OpenAIChatStreamChunk", "R30: the SSE item of ChatCompletions"),
    ("ollama-openai", "OpenAICompletionChoice", "R30: a choice of a completion"),
    ("ollama-openai", "OpenAICompletionResponse", "R30: the JSON reply of Completions"),
    ("ollama-openai", "OpenAICompletionStreamChunk", "R30: the SSE item of Completions"),
    ("ollama-openai", "StreamErrorLine", "R11c: the typed mid-stream error line"),
    ("ollama-openai", "GenerateStreamChunk", "R30: a Generate NDJSON chunk"),
    ("ollama-openai", "GenerateStreamItem", "R30: the Generate NDJSON item: chunk or error line"),
    ("ollama-openai", "GenerateCompletion", "R30: the buffered Generate reply (renamed from GenerateResponse, the D-15 enum name)"),
    ("ollama-openai", "ChatStreamChunk", "R30: a Chat NDJSON chunk"),
    ("ollama-openai", "ChatStreamItem", "R30: the Chat NDJSON item: chunk or error line"),
    ("ollama-openai", "ChatCompletion", "R30: the buffered Chat reply (renamed from ChatResponse, the D-15 enum name)"),
    ("ollama-openai", "PullModelProgress", "R30: a pull or push progress line"),
    ("ollama-openai", "PullModelStreamItem", "R30: the pull/push NDJSON item: progress or error line"),
    ("ollama-openai", "CreateModelProgress", "R30: a create progress line"),
    ("ollama-openai", "CreateModelStreamItem", "R30: the create NDJSON item: progress or error line"),
    ("ollama-openai", "StatusResponse", "R30: the buffered pull/push/create reply"),
    ("anthropic", "MessageStreamEvent", "R32: the SSE item type of CreateMessage"),
    ("anthropic", "ContentBlockDelta", "R32: a content_block_delta payload"),
    ("anthropic", "MessageDelta", "R32: a message_delta payload"),
    ("anthropic", "MessageDeltaUsage", "R32: a message_delta usage payload"),
    ("anthropic", "StreamErrorDetail", "R11c: the typed error event of CreateMessage"),
    ("anthropic", "StreamMessage", "R32: the message_start payload, whose stop_reason is null"),
    ("lmstudio", "ChatBody", "R31: the `stream` doc comment now says it defaults to false, as the provider documents"),
    ("lmstudio", "DownloadModelBody", "R31 review: the doc comment no longer claims SSE progress"),
    ("lmstudio", "ChatResult", "R31: the JSON reply of Chat"),
    ("lmstudio", "ChatOutputItem", "R31: an output item of the chat result"),
    ("lmstudio", "ChatStats", "R31: the statistics of the chat result"),
    ("lmstudio", "InvalidToolCallMetadata", "R31: metadata of an invalid tool call"),
    ("lmstudio", "ToolProviderInfo", "R31: the provider of a tool call"),
    ("lmstudio", "ChatStreamEvent", "R31: the SSE item type of Chat"),
    ("lmstudio", "ChatStreamErrorDetail", "R11c: the typed error event of Chat"),
    ("lmstudio", "DownloadModelResponse", "R31 review: the JSON download job, now registered with the documented fields"),
    ("elevenlabs", "SpeechWithTimestampsResponse", "R33: the provider sends snake_case members (audio_base64, normalized_alignment)"),
    ("elevenlabs", "HttpAlignment", "R33: the provider sends snake_case members (character_start_times_seconds, ...)"),
];

/// Rewrites a historical snapshot from before `RestApi::env_username` was
/// removed and Basic carried its own lists, and adds the named
/// [`adoptions`]. Touches only `definition/auth`, `definition/env_auth`,
/// `definition/env_username` and the adopted fields, and then recomputes
/// `fingerprints/api_metadata` exactly as `snapshot` does; anything it does
/// not recognize is left in place to be reported at its own leaf.
fn normalize_historical(key: &str, mut historical: Value) -> Value {
    lower_single_responses(&mut historical);
    let definition = historical["definition"].as_object_mut().unwrap();
    let mut changed = false;
    for (api, field, value) in adoptions() {
        if api == key && !definition.contains_key(field) {
            definition.insert(field.to_string(), value);
            changed = true;
        }
    }
    if definition.get("env_username").is_some_and(Value::is_null) {
        definition.remove("env_username");
        changed = true;
    }
    let env_auth = definition.get("env_auth").and_then(Value::as_array).cloned().unwrap_or_default();
    if BASIC_APIS.contains(&key) && definition.get("auth") == Some(&Value::from("Basic")) && env_auth.len() <= 1 {
        // Historical username: zero or one `env_username`; historical password:
        // what the legacy mapping selected, the first `env_auth` element.
        let username: Vec<Value> = match definition.remove("env_username") {
            Some(Value::String(name)) => vec![Value::String(name)],
            None => vec![],
            Some(other) => { definition.insert("env_username".into(), other); return historical; }
        };
        definition.insert("auth".into(), serde_json::json!({"Basic": {
            "username_env": {"names": username}, "password_env": {"names": env_auth}}}));
        definition.insert("env_auth".into(), serde_json::json!([]));
        changed = true;
    } else if UNUSED_USERNAME_APIS.contains(&key)
        && let Some(Value::String(name)) = definition.get("env_username")
        && definition.get("auth").is_some_and(|auth| auth != "Basic")
        && definition.get("env_mapping").and_then(|mapping| mapping.get("basic_user"))
            == Some(&serde_json::json!({"names": [name]})) {
        definition.remove("env_username");
        changed = true;
    }
    if changed {
        let mut metadata = historical["definition"].clone();
        metadata.as_object_mut().unwrap().remove("endpoints");
        historical["fingerprints"]["api_metadata"] = Value::String(fingerprint(&metadata));
    }
    if FORM_EXPLODE_DEFAULT_APIS.contains(&key) {
        apply_form_explode_default(&mut historical);
    }
    historical
}

/// Rewrites every historical single `response` (`{"Json": …}`, `"Text"`,
/// `"Binary"`, `"Empty"`) into the one-shape list `[{"shape": …}]` that
/// stores it now, and recomputes the fingerprint of each endpoint it changes
/// exactly as `snapshot` does. A response that is already a list is left in
/// place.
fn lower_single_responses(historical: &mut Value) {
    let count = historical["definition"]["endpoints"].as_array().map_or(0, Vec::len);
    for index in 0..count {
        let endpoint = &mut historical["definition"]["endpoints"][index];
        if endpoint["response"].is_array() {
            continue;
        }
        let single = endpoint["response"].take();
        endpoint["response"] = serde_json::json!([{"shape": single}]);
        let blake3 = fingerprint(endpoint);
        historical["fingerprints"]["endpoints"][index]["blake3"] = Value::String(blake3);
    }
}

/// Removes the [`MIGRATED_COMPONENTS`] of `key` from a snapshot's registry.
fn without_migrated_components(key: &str, mut snapshot: Value) -> Value {
    if let Some(registry) = snapshot["registry"].as_array_mut() {
        registry.retain(|entry| !MIGRATED_COMPONENTS.iter().any(|(api, name, _)| *api == key && entry["name"] == *name));
    }
    snapshot
}

/// Drops the differences a [`RESPONSE_MIGRATIONS`] entry of `key` allows, and
/// fails on an entry that allows none.
fn without_migrated_responses(key: &str, changed: Vec<String>) -> Vec<String> {
    let migrations: Vec<_> = RESPONSE_MIGRATIONS.iter().filter(|(api, ..)| *api == key).collect();
    let allowed = |path: &str, id: &str| {
        let endpoint = |root: &str| path.strip_prefix(&format!("{key}/{root}/")).and_then(|rest| rest.split_once(':'))
            .and_then(|(index, rest)| index.parse::<usize>().ok().map(|_| rest));
        endpoint("definition/endpoints").is_some_and(|rest| {
            rest == format!("{id}/response") || rest.starts_with(&format!("{id}/response/"))
                || rest == format!("{id}/errors") || rest.starts_with(&format!("{id}/errors/"))
        }) || endpoint("fingerprints/endpoints").is_some_and(|rest| rest == format!("{id}/blake3"))
    };
    for (_, id, reason) in &migrations {
        assert!(!reason.trim().is_empty(), "missing reason for {key} {id}");
        assert!(changed.iter().any(|path| allowed(path, id)), "stale response migration: {key} {id}");
    }
    changed.into_iter().filter(|path| !migrations.iter().any(|(_, id, _)| allowed(path, id))).collect()
}

/// Sets `explode` to `true` on every form-style query and cookie parameter
/// that records `false`, and recomputes the fingerprint of each endpoint it
/// changes exactly as `snapshot` does.
fn apply_form_explode_default(historical: &mut Value) {
    let count = historical["definition"]["endpoints"].as_array().map_or(0, Vec::len);
    for index in 0..count {
        let endpoint = &mut historical["definition"]["endpoints"][index];
        let mut changed = false;
        for location in ["query", "cookie"] {
            let Some(params) = endpoint.pointer_mut(&format!("/params/{location}")).and_then(Value::as_array_mut)
            else {
                continue;
            };
            for param in params {
                if param["style"] == "Form" && param["explode"] == false {
                    param["explode"] = Value::Bool(true);
                    changed = true;
                }
            }
        }
        if changed {
            let blake3 = fingerprint(endpoint);
            historical["fingerprints"]["endpoints"][index]["blake3"] = Value::String(blake3);
        }
    }
}

/// The components `api`'s rebuilt value rules reference from inside an
/// untyped field (the references [`without_value_rules`] removes from
/// `stripped`), with every component they reach in `stripped`. Schemars
/// registers a component where it is first reached, so these references can
/// move these components, and only these, earlier in the registry.
fn rebuild_reached(api: &RestApi, stripped: &RestApi) -> std::collections::HashSet<String> {
    fn references(value: &Value, found: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                if let Some(name) = object.get("$ref").and_then(Value::as_str).and_then(|reference| reference.strip_prefix("#/components/schemas/")) {
                    found.push(name.to_owned());
                }
                object.values().for_each(|child| references(child, found));
            }
            Value::Array(values) => values.iter().for_each(|child| references(child, found)),
            _ => {}
        }
    }
    let counts = |api: &RestApi| {
        let mut found = Vec::new();
        api.schemas.iter().for_each(|component| references(&component.schema, &mut found));
        found.into_iter().fold(std::collections::HashMap::<String, usize>::new(), |mut counts, name| {
            *counts.entry(name).or_default() += 1;
            counts
        })
    };
    let kept = counts(stripped);
    let mut reached: Vec<String> =
        counts(api).into_iter().filter(|(name, count)| kept.get(name).copied().unwrap_or(0) < *count).map(|(name, _)| name).collect();
    let mut seen: std::collections::HashSet<String> = reached.iter().cloned().collect();
    while let Some(name) = reached.pop() {
        let Some(component) = stripped.component(&name) else { continue };
        let mut found = Vec::new();
        references(&component.schema, &mut found);
        for target in found {
            if seen.insert(target.clone()) {
                reached.push(target);
            }
        }
    }
    seen
}

/// Places the `moved` entries of `current`'s registry where `historical`
/// lists them; every other entry keeps its current relative order, which is
/// still compared slot by slot. An entry `historical` does not list stays at
/// the end, to be reported at its own leaf.
fn in_historical_order(historical: &Value, current: &mut Value, moved: &std::collections::HashSet<String>) {
    if moved.is_empty() {
        return;
    }
    let mut entries: Vec<Value> = current["registry"].as_array_mut().map(std::mem::take).unwrap_or_default();
    let name = |entry: &Value| entry["name"].as_str().unwrap_or_default().to_owned();
    let mut ordered = Vec::with_capacity(entries.len());
    for slot in historical["registry"].as_array().into_iter().flatten() {
        let wanted = name(slot);
        let position = if moved.contains(&wanted) {
            entries.iter().position(|entry| name(entry) == wanted)
        } else {
            entries.iter().position(|entry| !moved.contains(&name(entry)))
        };
        if let Some(position) = position {
            ordered.push(entries.remove(position));
        }
    }
    ordered.append(&mut entries);
    current["registry"] = Value::Array(ordered);
}

/// The unexplained-before-[`EXPLAINED`] differences between a historical
/// snapshot and `api`, after normalization and the migration allow-lists.
fn historical_differences(key: &str, historical: Value, api: &RestApi) -> Vec<String> {
    historical_differences_from(key, historical, api, None)
}

/// [`historical_differences`] for a [`VALUE_RULE_APIS`] key, whose vendored
/// document is `source`.
fn historical_differences_from(key: &str, historical: Value, api: &RestApi, source: Option<&std::path::Path>) -> Vec<String> {
    let historical = normalize_historical(key, historical);
    let stripped = without_value_rules(key, api, source);
    let mut current = snapshot(&stripped);
    in_historical_order(&historical, &mut current, &rebuild_reached(api, &stripped));
    let expected = canonical(without_migrated_components(key, historical));
    for (api, name, reason) in MIGRATED_COMPONENTS.iter().filter(|(api, ..)| *api == key) {
        assert!(!reason.trim().is_empty(), "missing reason for {api} {name}");
        assert!(current["registry"].as_array().unwrap().iter().any(|entry| entry["name"] == *name), "stale migrated component: {api} {name}");
    }
    let actual = canonical(without_migrated_components(key, current));
    let mut changed = Vec::new();
    differences(&expected, &actual, key, &mut changed);
    without_migrated_responses(key, changed)
}

fn assert_baseline(key: &str, text: &str) {
    assert_baseline_from(key, text, None);
}

fn assert_baseline_from(key: &str, text: &str, source: Option<&std::path::Path>) {
    let changed = historical_differences_from(key, serde_json::from_str(text).unwrap(), &resolve_api(key).unwrap(), source);
    let prefix = format!("{key}/");
    let explained: Vec<_> = EXPLAINED.iter().copied().filter(|(path, _)| path.starts_with(&prefix)).collect();
    let changed = unexplained(changed, &explained);
    assert!(changed.is_empty(), "unexplained historical differences:\n{}", changed.join("\n"));
}

#[test]
fn anthropic() {
    assert_baseline("anthropic", include_str!("../fixtures/definition-baseline/anthropic.json"));
}

#[test]
fn artificial_analysis_critpt() {
    assert_baseline("artificial-analysis-critpt", include_str!("../fixtures/definition-baseline/artificial-analysis-critpt.json"));
}

#[test]
fn artificial_analysis_data() {
    assert_baseline("artificial-analysis-data", include_str!("../fixtures/definition-baseline/artificial-analysis-data.json"));
}

#[test]
fn bitbucket() {
    assert_baseline("bitbucket", include_str!("../fixtures/definition-baseline/bitbucket.json"));
}

#[test]
fn elevenlabs() {
    assert_baseline("elevenlabs", include_str!("../fixtures/definition-baseline/elevenlabs.json"));
}

#[test]
fn emqx_basic() {
    assert_baseline("emqx-basic", include_str!("../fixtures/definition-baseline/emqx-basic.json"));
}

#[test]
fn emqx_bearer() {
    assert_baseline("emqx-bearer", include_str!("../fixtures/definition-baseline/emqx-bearer.json"));
}

#[test]
fn eversolo() {
    assert_baseline("eversolo", include_str!("../fixtures/definition-baseline/eversolo.json"));
}

#[test]
fn gitea() {
    assert_baseline("gitea", include_str!("../fixtures/definition-baseline/gitea.json"));
}

#[test]
fn github() {
    assert_baseline("github", include_str!("../fixtures/definition-baseline/github.json"));
}

#[test]
fn gitlab() {
    assert_baseline("gitlab", include_str!("../fixtures/definition-baseline/gitlab.json"));
}

#[test]
fn huggingface() {
    assert_baseline("huggingface", include_str!("../fixtures/definition-baseline/huggingface.json"));
}

#[test]
fn lmstudio() {
    assert_baseline("lmstudio", include_str!("../fixtures/definition-baseline/lmstudio.json"));
}

#[test]
fn ollama_native() {
    assert_baseline("ollama-native", include_str!("../fixtures/definition-baseline/ollama-native.json"));
}

#[test]
fn ollama_openai() {
    assert_baseline("ollama-openai", include_str!("../fixtures/definition-baseline/ollama-openai.json"));
}

#[test]
fn openai() {
    let source = biscuit_test_harness::manifest_dir!().join("../specs/openai/openapi.yaml");
    assert_baseline_from("openai", include_str!("../fixtures/definition-baseline/openai.json"), Some(&source));
}

#[test]
fn samsung_smart_tv() {
    assert_baseline("samsung-smart-tv", include_str!("../fixtures/definition-baseline/samsung-smart-tv.json"));
}

#[test]
fn unfolded_circle_core_rest() {
    assert_baseline("unfolded-circle-core-rest", include_str!("../fixtures/definition-baseline/unfolded-circle-core-rest.json"));
}

#[test]
fn baseline_catalog_matches_shipped_apis() {
    let cases: &[(&str, fn())] = &[
        ("anthropic", anthropic),
        ("artificial-analysis-critpt", artificial_analysis_critpt),
        ("artificial-analysis-data", artificial_analysis_data),
        ("bitbucket", bitbucket),
        ("elevenlabs", elevenlabs),
        ("emqx-basic", emqx_basic),
        ("emqx-bearer", emqx_bearer),
        ("eversolo", eversolo),
        ("gitea", gitea),
        ("github", github),
        ("gitlab", gitlab),
        ("huggingface", huggingface),
        ("lmstudio", lmstudio),
        ("ollama-native", ollama_native),
        ("ollama-openai", ollama_openai),
        ("openai", openai),
        ("samsung-smart-tv", samsung_smart_tv),
        ("unfolded-circle-core-rest", unfolded_circle_core_rest),
    ];
    let expected: Vec<_> = cases.iter().map(|(key, _)| key.to_string()).collect();
    let mut actual: Vec<_> = resolve_all_apis().iter().map(|api|
        schematic_definitions::registry::registry_key_for(&api.name)).collect();
    actual.sort();
    assert_eq!(expected, actual);
    for (path, _) in EXPLAINED {
        assert!(expected.iter().any(|key| path.starts_with(&format!("{key}/"))), "unknown API: {path}");
    }
}

#[test]
fn shipped_definitions_round_trip() {
    let temp = tempfile::TempDir::new().unwrap();
    for api in resolve_all_apis() {
        let original = canonical(snapshot(&api));
        let encoded = serde_json::to_string(&api).unwrap();
        let path = temp.path().join(format!("{}.json", api.name));
        std::fs::write(&path, &encoded).unwrap();
        let restored: RestApi = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let rewritten = serde_json::to_string(&restored).unwrap();
        std::fs::write(&path, &rewritten).unwrap();
        let reread: RestApi = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(api, reread, "{}", api.name);
        assert_eq!(encoded, rewritten);
        assert_eq!(original, canonical(snapshot(&reread)), "{} dependent outputs", api.name);
    }
}

#[test]
fn changed_description_is_detected_at_its_exact_field() {
    let expected: Value = serde_json::from_str(include_str!("../fixtures/definition-baseline/github.json")).unwrap();
    let mut api = resolve_api("github").unwrap();
    api.endpoints[0].description.push_str(" deliberately changed");
    let mut changed = Vec::new();
    differences(&canonical(normalize_historical("github", expected)), &canonical(snapshot(&api)), "github", &mut changed);
    assert_eq!(changed, [format!("github/definition/endpoints/0:{}/description", api.endpoints[0].id),
        format!("github/fingerprints/endpoints/0:{}/blake3", api.endpoints[0].id)]);
}

fn historical(file: &str) -> Value {
    let path = biscuit_test_harness::manifest_dir!().join(format!("tests/fixtures/definition-baseline/{file}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn adoptions_accept_only_the_named_api_field_and_exact_value() {
    let mut github = resolve_api("github").unwrap();
    github.rate_limit = Some(
        schematic_define::RateLimit::new("x-ratelimit-limit", "x-ratelimit-remaining", "x-ratelimit-used",
            schematic_define::ResetUnit::EpochSeconds).unwrap().with_retry_after_header("retry-after").unwrap(),
    );
    let changed = historical_differences("github", historical("github"), &github);
    assert!(changed.contains(&"github/definition/rate_limit/reset_header".to_string()), "{changed:?}");

    let mut gitea = resolve_api("gitea").unwrap();
    gitea.rate_limit = github.rate_limit.clone();
    let changed = historical_differences("gitea", historical("gitea"), &gitea);
    assert!(changed.contains(&"gitea/definition/rate_limit".to_string()), "{changed:?}");
}

#[test]
fn historical_normalization_lowers_basic_and_unused_usernames_only() {
    for key in ["bitbucket", "emqx-basic", "unfolded-circle-core-rest", "github"] {
        let raw = historical(key);
        let normalized = normalize_historical(key, raw.clone());
        let mut touched = Vec::new();
        differences(&canonical(raw.clone()), &canonical(normalized.clone()), key, &mut touched);
        let adopted: Vec<String> = adoptions().into_iter().filter(|(api, ..)| *api == key)
            .map(|(_, field, _)| format!("{key}/definition/{field}")).collect();
        let endpoint_leaf = |path: &str, root: &str, leaf: &str| path.strip_prefix(&format!("{key}/{root}/"))
            .and_then(|rest| rest.split_once('/')).is_some_and(|(_, rest)| rest == leaf);
        touched.retain(|path| !path.starts_with(&format!("{key}/definition/auth/"))
            && ![format!("{key}/definition/auth"), format!("{key}/definition/env_auth/0"),
                format!("{key}/definition/env_username"), format!("{key}/fingerprints/api_metadata")].contains(path)
            && !adopted.contains(path)
            && !endpoint_leaf(path, "definition/endpoints", "response")
            && !endpoint_leaf(path, "fingerprints/endpoints", "blake3"));
        assert!(touched.is_empty(), "{key}: normalization touched {touched:?}");
        for (before, after) in raw["definition"]["endpoints"].as_array().unwrap().iter()
            .zip(normalized["definition"]["endpoints"].as_array().unwrap()) {
            assert_eq!(after["response"], serde_json::json!([{"shape": before["response"]}]), "{key} {}", before["id"]);
        }
    }
}

#[test]
fn a_historical_single_response_is_lowered_and_a_changed_one_is_still_reported() {
    let api = resolve_api("github").unwrap();
    assert!(historical_differences("github", historical("github"), &api).is_empty());
    let mut mutated = historical("github");
    let index = api.endpoints.iter().position(|endpoint| endpoint.response[0].plain() == Some(&schematic_define::ResponseShape::Text)).unwrap();
    mutated["definition"]["endpoints"][index]["response"] = Value::from("Binary");
    let id = &api.endpoints[index].id;
    assert_eq!(historical_differences("github", mutated, &api), [format!("github/definition/endpoints/{index}:{id}/response/0/shape"),
        format!("github/fingerprints/endpoints/{index}:{id}/blake3")]);
    let mut listed = historical("github");
    let lowered = serde_json::json!([{"shape": listed["definition"]["endpoints"][index]["response"].clone()}]);
    listed["definition"]["endpoints"][index]["response"] = lowered.clone();
    assert_eq!(normalize_historical("github", listed)["definition"]["endpoints"][index]["response"], lowered, "a list is left in place");
}

#[test]
fn populated_username_outside_the_named_equivalences_is_reported() {
    let api = resolve_api("unfolded-circle-core-rest").unwrap();
    let mut mutated = historical("unfolded-circle-core-rest");
    mutated["definition"]["env_username"] = Value::from("UCR_OTHER_USER");
    assert_eq!(historical_differences("unfolded-circle-core-rest", mutated, &api),
        ["unfolded-circle-core-rest/definition/env_username", "unfolded-circle-core-rest/fingerprints/api_metadata"]);
    let api = resolve_api("github").unwrap();
    let mut mutated = historical("github");
    mutated["definition"]["env_username"] = Value::from("GITHUB_USER");
    assert_eq!(historical_differences("github", mutated, &api),
        ["github/definition/env_username", "github/fingerprints/api_metadata"]);
}

#[test]
fn basic_lowering_reports_new_password_names_and_unlowerable_history() {
    let mut api = resolve_api("bitbucket").unwrap();
    let schematic_define::AuthStrategy::Basic { password_env, .. } = &mut api.auth else { panic!("Bitbucket is Basic") };
    *password_env = schematic_define::EnvList::from_strs(&["BITBUCKET_APP_PASSWORD", "BITBUCKET_TOKEN"]);
    assert_eq!(historical_differences("bitbucket", historical("bitbucket"), &api),
        ["bitbucket/definition/auth/Basic/password_env/names/1", "bitbucket/fingerprints/api_metadata"]);
    let api = resolve_api("bitbucket").unwrap();
    let mut mutated = historical("bitbucket");
    mutated["definition"]["env_auth"] = serde_json::json!(["BITBUCKET_APP_PASSWORD", "BITBUCKET_TOKEN"]);
    let changed = historical_differences("bitbucket", mutated, &api);
    assert!(changed.contains(&"bitbucket/definition/env_username".to_string()), "{changed:?}");
    assert!(changed.contains(&"bitbucket/definition/auth".to_string()), "{changed:?}");
}

#[test]
fn normalization_leaves_other_auth_leaves_reported() {
    let api = resolve_api("github").unwrap();
    let mut mutated = historical("github");
    mutated["definition"]["env_auth"][0] = Value::from("GITHUB_RENAMED");
    assert_eq!(historical_differences("github", mutated, &api),
        ["github/definition/env_auth/0", "github/fingerprints/api_metadata"]);
}

#[test]
fn shipped_json_field_matrix_preserves_required_and_optional_shapes() {
    // `optional` fields may be absent; `strict` ones among them reject an
    // explicit null instead of reading it as absent.
    fn matrix<T>(control: Value, optional: &[&str], strict: &[&str])
    where T: serde::de::DeserializeOwned + serde::Serialize {
        assert!(serde_json::from_value::<T>(control.clone()).is_ok());
        for (field, value) in control.as_object().unwrap() {
            let is_optional = optional.contains(&field.as_str());
            for null in [false, true] {
                let mut edited = control.clone();
                if null { edited[field] = Value::Null; }
                else { edited.as_object_mut().unwrap().remove(field); }
                let result = serde_json::from_value::<T>(edited);
                let accepted = is_optional && !(null && strict.contains(&field.as_str()));
                assert_eq!(result.is_ok(), accepted, "{field}: null={null}");
                if let Ok(restored) = result {
                    assert!(serde_json::to_value(restored).unwrap()[field].is_null(), "{field}");
                }
            }
            let wrong = if value.is_boolean() { serde_json::json!("true") }
                else { serde_json::json!(123) };
            let mut edited = control.clone();
            edited[field] = wrong;
            assert!(serde_json::from_value::<T>(edited).is_err(), "{field}: wrong type");
            if let Some(values) = value.as_array() {
                for bad in [serde_json::json!([123]), {
                    let mut mixed = values.clone(); mixed.push(serde_json::json!(123));
                    Value::Array(mixed)
                }] {
                    let mut edited = control.clone(); edited[field] = bad;
                    assert!(serde_json::from_value::<T>(edited).is_err(), "{field}: bad elements");
                }
                let mut empty = control.clone(); empty[field] = serde_json::json!([]);
                let restored: T = serde_json::from_value(empty).unwrap();
                assert_eq!(serde_json::to_value(restored).unwrap()[field], serde_json::json!([]));
            }
            let enum_fields = ["auth", "cross_origin", "method", "response", "param_type", "style"];
            let optional_strings = ["docs_url", "module_path", "request_suffix",
                "version", "request_id_header", "description"];
            if !value.is_boolean() {
                let (empty, accepts) = if value.is_array() {
                    (serde_json::json!([]), true)
                } else if field == "auth_policy" || field == "env_mapping" {
                    (serde_json::json!({}), true)
                } else if (value.is_string() && !enum_fields.contains(&field.as_str()))
                    || optional_strings.contains(&field.as_str()) {
                    (serde_json::json!(""), true)
                } else {
                    (serde_json::json!({}), false)
                };
                let mut edited = control.clone(); edited[field] = empty;
                let result = serde_json::from_value::<T>(edited);
                assert_eq!(result.is_ok(), accepts, "{field}: empty native");
                if let Ok(restored) = result {
                    assert!(!serde_json::to_value(restored).unwrap()[field].is_null(), "{field}: empty became absent");
                }
            }
            let encoded = serde_json::to_string(&control).unwrap();
            let duplicate = format!("{{\"{field}\":{},{}", serde_json::to_string(value).unwrap(), &encoded[1..]);
            assert!(serde_json::from_str::<T>(&duplicate).is_err(), "{field}: duplicate");
        }
        let trailing = serde_json::to_string(&control).unwrap() + " garbage";
        assert!(serde_json::from_str::<T>(&trailing).is_err());
    }
    let api = resolve_api("github").unwrap();
    matrix::<RestApi>(serde_json::to_value(&api).unwrap(), &[
        "docs_url", "auth_policy", "module_path", "request_suffix",
        "version", "env_mapping", "request_id_header", "default_pagination", "rate_limit",
    ], &["rate_limit"]);
    for endpoint in &api.endpoints {
        matrix::<schematic_define::Endpoint>(serde_json::to_value(endpoint).unwrap(), &[
            "request", "params", "cross_origin", "pagination",
        ], &[]);
        if let Some(params) = &endpoint.params {
            matrix::<schematic_define::EndpointParams>(serde_json::to_value(params).unwrap(), &[

            ], &[]);
            for param in params.query.iter().chain(&params.header).chain(&params.cookie) {
                matrix::<schematic_define::ParamDef>(serde_json::to_value(param).unwrap(), &["description"], &[]);
            }
        }
    }
}

#[test]
fn historical_exceptions_only_accept_the_exact_leaf_with_a_cause() {
    let changed = vec!["github/definition/endpoints/0:GetRepository/description".to_string(),
        "github/registry/0:Repository/blake3".to_string()];
    let remaining = unexplained(changed, &[("github/definition/endpoints/0:GetRepository/description",
        "Example fixture cause: a description was corrected")]);
    assert_eq!(remaining, ["github/registry/0:Repository/blake3"]);
}

#[test]
#[should_panic(expected = "missing cause")]
fn historical_exception_requires_a_cause() {
    unexplained(vec!["github/registry/0:Repository/blake3".into()], &[("github/registry/0:Repository/blake3", " ")]);
}

#[test]
#[should_panic(expected = "wildcard exception")]
fn historical_exception_rejects_wildcards() {
    unexplained(vec![], &[("github/*", "Too broad")]);
}
