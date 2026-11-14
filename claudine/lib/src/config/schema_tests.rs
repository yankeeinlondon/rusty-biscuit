use super::*;
use serde_json::{Value, json};

fn defaulted(scope: ConfigScope, document: Value) -> Value {
    let success = contract(scope).validate_strict(&document).expect("valid");
    let mut value = document.clone();
    for default in &success.defaults {
        let mut target = &mut value;
        let (last, parents) = default.path.segments().split_last().expect("a property");
        for segment in parents {
            target = target.get_mut(segment).expect("parent before child");
        }
        target[last] = default.value.clone();
    }
    value
}

#[test]
fn both_scopes_prepare_and_accept_an_empty_file() {
    for scope in [ConfigScope::User, ConfigScope::Repository] {
        assert!(contract(scope).validate_strict(&json!({})).is_ok(), "{scope:?}");
    }
}

/// A parent default is judged whole, so each mapping default restates its
/// children's defaults; they must agree.
#[test]
fn each_mapping_default_equals_its_children_defaults() {
    let whole = defaulted(ConfigScope::User, json!({}));
    let parts = defaulted(
        ConfigScope::User,
        json!({ "guard_settings": { "repetition": {}, "volume": {} }, "steering": { "automatic": {} }, "permissions": {} }),
    );
    for key in ["guard_settings", "steering", "permissions"] {
        assert_eq!(whole[key], parts[key], "{key}");
    }
    let partial = defaulted(ConfigScope::User, json!({ "guard_settings": {} }));
    assert_eq!(whole["guard_settings"], partial["guard_settings"]);
}

#[test]
fn a_repository_file_gets_no_inserted_overrides() {
    let value = defaulted(ConfigScope::Repository, json!({}));
    let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["permissions"], "only Darkmatter's block has a repository default");
}
