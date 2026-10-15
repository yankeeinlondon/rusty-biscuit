//! Historical JSON is compared as values before any typed deserialization.
#[path = "../../examples/support/definition_snapshot.rs"]
mod definition_snapshot;

use schematic_define::RestApi;
use schematic_gen::pipeline::{resolve_all_apis, resolve_api};
use serde_json::Value;
use definition_snapshot::{canonical, differences, snapshot};

// Historical exceptions name one exact leaf and its cause. Current artifacts
// always compare without exceptions in artifact_drift.
const EXPLAINED: &[(&str, &str)] = &[];

fn unexplained(changed: Vec<String>, explained: &[(&str, &str)]) -> Vec<String> {
    for (path, cause) in explained {
        assert!(!cause.trim().is_empty(), "missing cause for {path}");
        assert!(!path.contains('*'), "wildcard exception: {path}");
        assert!(changed.iter().any(|change| change == path), "stale exception: {path}");
    }
    changed.into_iter().filter(|path| !explained.iter().any(|(allowed, _)| path == allowed)).collect()
}

fn assert_baseline(key: &str, text: &str) {
    let expected: Value = serde_json::from_str(text).unwrap();
    let api = resolve_api(key).unwrap();
    let actual = canonical(snapshot(&api));
    let mut changed = Vec::new();
    differences(&expected, &actual, key, &mut changed);
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
    assert_baseline("openai", include_str!("../fixtures/definition-baseline/openai.json"));
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
    differences(&expected, &canonical(snapshot(&api)), "github", &mut changed);
    assert_eq!(changed, [format!("github/definition/endpoints/0:{}/description", api.endpoints[0].id),
        format!("github/fingerprints/endpoints/0:{}/blake3", api.endpoints[0].id)]);
}

#[test]
fn shipped_json_field_matrix_preserves_required_and_optional_shapes() {
    fn matrix<T>(control: Value, optional: &[&str])
    where T: serde::de::DeserializeOwned + serde::Serialize {
        assert!(serde_json::from_value::<T>(control.clone()).is_ok());
        for (field, value) in control.as_object().unwrap() {
            let is_optional = optional.contains(&field.as_str());
            for null in [false, true] {
                let mut edited = control.clone();
                if null { edited[field] = Value::Null; }
                else { edited.as_object_mut().unwrap().remove(field); }
                let result = serde_json::from_value::<T>(edited);
                assert_eq!(result.is_ok(), is_optional, "{field}: null={null}");
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
            let optional_strings = ["docs_url", "env_username", "module_path", "request_suffix",
                "version", "request_id_header", "description"];
            if !value.is_boolean() {
                let (empty, accepts) = if value.is_array() || field == "oauth_scopes" {
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
        "docs_url", "auth_policy", "env_username", "module_path", "request_suffix",
        "version", "env_mapping", "request_id_header", "default_pagination",
    ]);
    for endpoint in &api.endpoints {
        matrix::<schematic_define::Endpoint>(serde_json::to_value(endpoint).unwrap(), &[
            "request", "params", "oauth_scopes",
        ]);
        if let Some(params) = &endpoint.params {
            matrix::<schematic_define::EndpointParams>(serde_json::to_value(params).unwrap(), &[
                "pagination", "response_pagination",
            ]);
            for param in params.query.iter().chain(&params.header).chain(&params.cookie) {
                matrix::<schematic_define::ParamDef>(serde_json::to_value(param).unwrap(), &["description"]);
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
