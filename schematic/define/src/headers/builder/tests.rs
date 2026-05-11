use super::super::env::{ApiKeyEnv, EnvList, EnvMapping};
use super::super::error::HeaderError;
use super::*;

#[test]
fn headers_default_is_empty() {
    let headers = Headers::default();
    let result = headers.build().unwrap();
    assert!(result.is_empty());
}

#[test]
fn headers_bearer_token_formatting() {
    let headers = Headers::default().use_bearer_token("my-secret-token");
    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Authorization");
    assert_eq!(result[0].1, "Bearer my-secret-token");
}

#[test]
fn headers_basic_auth_encoding() {
    let headers = Headers::default().use_basic_auth("username", "password");
    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Authorization");
    assert_eq!(result[0].1, "Basic dXNlcm5hbWU6cGFzc3dvcmQ=");
}

#[test]
fn headers_basic_auth_special_chars() {
    let headers = Headers::default().use_basic_auth("user@example.com", "p@ssw0rd!");
    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Authorization");
    assert_eq!(result[0].1, "Basic dXNlckBleGFtcGxlLmNvbTpwQHNzdzByZCE=");
}

#[test]
fn headers_api_key_custom_header() {
    let headers = Headers::default().use_api_key("my-api-key", "X-API-Key");
    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "X-API-Key");
    assert_eq!(result[0].1, "my-api-key");
}

#[test]
fn headers_builder_chaining() {
    let headers = Headers::default()
        .use_bearer_token("token123")
        .content_type("application/json")
        .accept("application/json")
        .user_agent("MyClient/1.0");

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 4);

    assert!(result.contains(&("Authorization".to_string(), "Bearer token123".to_string())));
    assert!(result.contains(&("Content-Type".to_string(), "application/json".to_string())));
    assert!(result.contains(&("Accept".to_string(), "application/json".to_string())));
    assert!(result.contains(&("User-Agent".to_string(), "MyClient/1.0".to_string())));
}

#[test]
fn headers_accept_json_convenience() {
    let headers = Headers::default().accept_json();
    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Accept");
    assert_eq!(result[0].1, "application/json");
}

#[test]
fn headers_content_type_json_convenience() {
    let headers = Headers::default().content_type_json();
    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Content-Type");
    assert_eq!(result[0].1, "application/json");
}

#[test]
fn headers_custom_header() {
    let headers = Headers::default()
        .header("X-Custom-1", "value1")
        .header("X-Custom-2", "value2");

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 2);
    assert!(result.contains(&("X-Custom-1".to_string(), "value1".to_string())));
    assert!(result.contains(&("X-Custom-2".to_string(), "value2".to_string())));
}

#[test]
fn headers_remove_header() {
    let headers = Headers::default()
        .header("X-Custom-1", "value1")
        .header("X-Custom-2", "value2")
        .remove("X-Custom-1");

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "X-Custom-2");
    assert_eq!(result[0].1, "value2");
}

#[test]
fn headers_remove_standard_header() {
    let headers = Headers::default()
        .content_type("text/plain")
        .accept("text/html")
        .remove("Content-Type");

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Accept");
    assert_eq!(result[0].1, "text/html");
}

#[test]
fn headers_remove_nonexistent_does_nothing() {
    let headers = Headers::default()
        .header("X-Custom", "value")
        .remove("X-NonExistent");

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "X-Custom");
}

#[test]
fn headers_invalid_header_name_non_ascii() {
    let headers = Headers::default().header("Inválid-Héader", "value");
    let result = headers.build();

    assert!(result.is_err());
    match result {
        Err(HeaderError::InvalidHeaderName(name)) => {
            assert_eq!(name, "Inválid-Héader");
        }
        _ => panic!("Expected InvalidHeaderName error"),
    }
}

#[test]
fn headers_invalid_header_name_with_space() {
    let headers = Headers::default().header("Invalid Header", "value");
    let result = headers.build();

    assert!(result.is_err());
    match result {
        Err(HeaderError::InvalidHeaderName(name)) => {
            assert_eq!(name, "Invalid Header");
        }
        _ => panic!("Expected InvalidHeaderName error"),
    }
}

#[test]
fn headers_valid_ascii_header_names() {
    let headers = Headers::default()
        .header("X-Custom-123", "value")
        .header("Accept-Language", "en-US")
        .header("cache-control", "no-cache");

    let result = headers.build();
    assert!(result.is_ok());
}

#[test]
fn headers_multiple_auth_strategies_last_wins() {
    let headers = Headers::default()
        .use_bearer_token("token1")
        .use_basic_auth("user", "pass");

    let result = headers.build().unwrap();

    let auth_headers: Vec<_> = result
        .iter()
        .filter(|(k, _)| k == "Authorization")
        .collect();

    assert_eq!(auth_headers.len(), 1);
    assert!(auth_headers[0].1.starts_with("Basic "));
}

#[test]
fn headers_override_content_type() {
    let headers = Headers::default()
        .content_type("text/plain")
        .content_type("application/json");

    let result = headers.build().unwrap();

    let ct_headers: Vec<_> = result.iter().filter(|(k, _)| k == "Content-Type").collect();

    assert_eq!(ct_headers.len(), 1);
    assert_eq!(ct_headers[0].1, "application/json");
}

#[test]
fn headers_has_authorization_false_by_default() {
    let headers = Headers::default();
    assert!(!headers.has_authorization());
}

#[test]
fn headers_has_authorization_true_after_bearer_token() {
    let headers = Headers::default().use_bearer_token("my-token");
    assert!(headers.has_authorization());
}

#[test]
fn headers_has_authorization_true_after_basic_auth() {
    let headers = Headers::default().use_basic_auth("user", "pass");
    assert!(headers.has_authorization());
}

#[test]
fn headers_has_authorization_false_after_api_key() {
    let headers = Headers::default().use_api_key("key", "X-API-Key");
    assert!(!headers.has_authorization());
}

#[test]
fn headers_has_explicit_auth_true_after_api_key() {
    let headers = Headers::default().use_api_key("key", "X-API-Key");
    assert!(headers.has_explicit_auth());
}

#[test]
fn headers_has_explicit_auth_false_after_api_key_remove() {
    let headers = Headers::default()
        .use_api_key("key", "X-API-Key")
        .remove("X-API-Key");
    assert!(!headers.has_explicit_auth());
}

#[test]
fn headers_has_authorization_false_after_remove() {
    let headers = Headers::default()
        .use_bearer_token("token")
        .remove("Authorization");
    assert!(!headers.has_authorization());
}

#[test]
fn headers_from_string_conversions() {
    let headers = Headers::default()
        .use_bearer_token(String::from("token"))
        .content_type(String::from("text/plain"))
        .header(String::from("X-Custom"), String::from("value"));

    let result = headers.build().unwrap();
    assert_eq!(result.len(), 3);
}

#[test]
fn headers_empty_values_allowed() {
    let headers = Headers::default().header("X-Empty", "").content_type("");

    let result = headers.build();
    assert!(result.is_ok());
}

#[test]
#[serial_test::serial]
fn from_env_resolves_bearer_token_from_first_matching_var() {
    unsafe {
        std::env::set_var("OPENAI_API_KEY", "sk-first-token");
        std::env::set_var("OPENAI_KEY", "sk-second-token");
    }

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"])),
        basic_user: None,
        basic_pass: None,
        api_key: None,
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    }
    .from_env();

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Authorization");
    assert_eq!(result[0].1, "Bearer sk-first-token");

    unsafe {
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OPENAI_KEY");
    }
}

#[test]
#[serial_test::serial]
fn from_env_skips_when_no_env_vars_set() {
    unsafe {
        std::env::remove_var("MISSING_KEY_1");
        std::env::remove_var("MISSING_KEY_2");
    }

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::from_strs(&["MISSING_KEY_1", "MISSING_KEY_2"])),
        basic_user: None,
        basic_pass: None,
        api_key: None,
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    }
    .from_env();

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 0);
}

#[test]
#[serial_test::serial]
fn from_env_respects_fallback_chain_order() {
    unsafe {
        std::env::remove_var("PRIMARY_KEY");
        std::env::set_var("SECONDARY_KEY", "fallback-token");
        std::env::set_var("TERTIARY_KEY", "third-token");
    }

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::from_strs(&[
            "PRIMARY_KEY",
            "SECONDARY_KEY",
            "TERTIARY_KEY",
        ])),
        basic_user: None,
        basic_pass: None,
        api_key: None,
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    }
    .from_env();

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Authorization");
    assert_eq!(result[0].1, "Bearer fallback-token");

    unsafe {
        std::env::remove_var("SECONDARY_KEY");
        std::env::remove_var("TERTIARY_KEY");
    }
}

#[test]
#[serial_test::serial]
fn from_env_does_not_overwrite_preset_authorization() {
    unsafe {
        std::env::set_var("OPENAI_API_KEY", "sk-from-env");
    }

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::single("OPENAI_API_KEY")),
        basic_user: None,
        basic_pass: None,
        api_key: None,
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    }
    .use_bearer_token("sk-preset-token")
    .from_env();

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Authorization");
    assert_eq!(result[0].1, "Bearer sk-preset-token");

    unsafe {
        std::env::remove_var("OPENAI_API_KEY");
    }
}

#[test]
#[serial_test::serial]
fn from_env_does_not_apply_bearer_fallback_when_api_key_is_explicit() {
    unsafe {
        std::env::set_var("OPENAI_API_KEY", "sk-from-env");
    }

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::single("OPENAI_API_KEY")),
        basic_user: None,
        basic_pass: None,
        api_key: None,
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    }
    .use_api_key("hf-explicit", "X-API-Key")
    .from_env();

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "X-API-Key");
    assert_eq!(result[0].1, "hf-explicit");

    unsafe {
        std::env::remove_var("OPENAI_API_KEY");
    }
}

#[test]
#[serial_test::serial]
fn from_env_does_not_overwrite_explicit_api_key() {
    unsafe {
        std::env::set_var("HF_TOKEN", "hf-from-env");
    }

    let mapping = EnvMapping {
        bearer_token: None,
        basic_user: None,
        basic_pass: None,
        api_key: Some(ApiKeyEnv {
            names: EnvList::single("HF_TOKEN"),
            header: "X-API-Key".to_string(),
        }),
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    }
    .use_api_key("hf-explicit", "X-API-Key")
    .from_env();

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "X-API-Key");
    assert_eq!(result[0].1, "hf-explicit");

    unsafe {
        std::env::remove_var("HF_TOKEN");
    }
}

#[test]
#[serial_test::serial]
fn from_env_with_uses_custom_mapping() {
    unsafe {
        std::env::set_var("CUSTOM_TOKEN_VAR", "custom-token");
    }

    let custom_mapping = EnvMapping {
        bearer_token: Some(EnvList::single("CUSTOM_TOKEN_VAR")),
        basic_user: None,
        basic_pass: None,
        api_key: None,
        ..Default::default()
    };

    let headers = Headers::default().from_env_with(custom_mapping);

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Authorization");
    assert_eq!(result[0].1, "Bearer custom-token");

    unsafe {
        std::env::remove_var("CUSTOM_TOKEN_VAR");
    }
}

#[test]
#[serial_test::serial]
fn try_from_env_returns_error_when_required_vars_missing() {
    unsafe {
        std::env::remove_var("REQUIRED_KEY_1");
        std::env::remove_var("REQUIRED_KEY_2");
    }

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::from_strs(&["REQUIRED_KEY_1", "REQUIRED_KEY_2"])),
        basic_user: None,
        basic_pass: None,
        api_key: None,
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    };

    let result = headers.try_from_env();

    assert!(result.is_err());
    match result {
        Err(HeaderError::MissingCredential(vars)) => {
            assert_eq!(vars.len(), 2);
            assert!(vars.contains(&"REQUIRED_KEY_1".to_string()));
            assert!(vars.contains(&"REQUIRED_KEY_2".to_string()));
        }
        _ => panic!("Expected MissingCredential error"),
    }
}

#[test]
#[serial_test::serial]
fn try_from_env_succeeds_when_all_required_vars_present() {
    unsafe {
        std::env::set_var("API_TOKEN", "valid-token");
    }

    let mapping = EnvMapping {
        bearer_token: Some(EnvList::single("API_TOKEN")),
        basic_user: None,
        basic_pass: None,
        api_key: None,
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    };

    let result = headers.try_from_env();

    assert!(result.is_ok());
    let headers = result.unwrap();
    let built = headers.build().unwrap();

    assert_eq!(built.len(), 1);
    assert_eq!(built[0].0, "Authorization");
    assert_eq!(built[0].1, "Bearer valid-token");

    unsafe {
        std::env::remove_var("API_TOKEN");
    }
}

#[test]
#[serial_test::serial]
fn from_env_basic_auth_resolution() {
    unsafe {
        std::env::set_var("API_USERNAME", "myuser");
        std::env::set_var("API_PASSWORD", "mypass");
    }

    let mapping = EnvMapping {
        bearer_token: None,
        basic_user: Some(EnvList::single("API_USERNAME")),
        basic_pass: Some(EnvList::single("API_PASSWORD")),
        api_key: None,
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    }
    .from_env();

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "Authorization");
    assert!(result[0].1.starts_with("Basic "));

    unsafe {
        std::env::remove_var("API_USERNAME");
        std::env::remove_var("API_PASSWORD");
    }
}

#[test]
#[serial_test::serial]
fn from_env_api_key_resolution() {
    unsafe {
        std::env::set_var("HF_TOKEN", "hf-api-key-12345");
    }

    let mapping = EnvMapping {
        bearer_token: None,
        basic_user: None,
        basic_pass: None,
        api_key: Some(ApiKeyEnv {
            names: EnvList::from_strs(&["HF_TOKEN", "HUGGINGFACE_KEY"]),
            header: "X-API-Key".to_string(),
        }),
        ..Default::default()
    };

    let headers = Headers {
        env_mapping: mapping,
        ..Default::default()
    }
    .from_env();

    let result = headers.build().unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "X-API-Key");
    assert_eq!(result[0].1, "hf-api-key-12345");

    unsafe {
        std::env::remove_var("HF_TOKEN");
    }
}
