use schematic_define::RestApi;
use schematic_define::pagination::{caller_knobs, param_adoptions, resolve};
use schematic_definitions::registry::{get_registry, registry_key_for};
use serde_json::{Value, json};

pub fn snapshot(api: &RestApi) -> Value {
    let registry = get_registry(&registry_key_for(&api.name)).expect("shipped registry");
    let entries: Vec<_> = registry.names().map(|name| json!({
        "name": name, "blake3": fingerprint(registry.get(name).unwrap().as_value()),
    })).collect();
    let pagination: Vec<_> = api.endpoints.iter().map(|endpoint| {
        let resolution = resolve(api, endpoint);
        json!({"endpoint": endpoint.id, "resolution": resolution,
            "caller_knobs": caller_knobs(&resolution),
            "param_adoptions": resolution.strategy().map(|strategy| param_adoptions(endpoint, strategy)).unwrap_or_default()})
    }).collect();
    let mut metadata = serde_json::to_value(api).unwrap();
    metadata.as_object_mut().unwrap().remove("endpoints");
    let endpoints: Vec<_> = api.endpoints.iter().map(|endpoint| json!({
        "id": endpoint.id, "blake3": fingerprint(&serde_json::to_value(endpoint).unwrap()),
    })).collect();
    json!({"definition": api, "registry": entries, "pagination": pagination,
        "fingerprints": {"api_metadata": fingerprint(&metadata), "endpoints": endpoints}})
}

pub fn fingerprint(value: &Value) -> String {
    biscuit_hash::blake3_hash(&serde_json::to_string(&canonical(value.clone())).unwrap())
}

// Object keys are canonical; arrays retain endpoint and registration order.
pub fn canonical(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let sorted: std::collections::BTreeMap<_, _> = object.into_iter()
                .map(|(key, value)| (key, canonical(value))).collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        value => value,
    }
}

pub fn differences(expected: &Value, actual: &Value, path: &str, out: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Object(left), Value::Object(right)) => {
            let keys: std::collections::BTreeSet<_> = left.keys().chain(right.keys()).collect();
            for key in keys {
                let child = format!("{path}/{key}");
                match (left.get(key), right.get(key)) {
                    (Some(a), Some(b)) => differences(a, b, &child, out),
                    _ => out.push(child),
                }
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            for index in 0..left.len().max(right.len()) {
                let label = left.get(index).or_else(|| right.get(index)).and_then(|value|
                    value.get("id").or_else(|| value.get("endpoint")).or_else(|| value.get("name")))
                    .and_then(Value::as_str);
                let child = match label {
                    Some(name) => format!("{path}/{index}:{name}"),
                    None => format!("{path}/{index}"),
                };
                match (left.get(index), right.get(index)) {
                    (Some(a), Some(b)) => differences(a, b, &child, out),
                    _ => out.push(child),
                }
            }
        }
        (a, b) if a != b => out.push(path.to_owned()),
        _ => {}
    }
}
