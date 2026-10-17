//! `CrossOrigin` constructors and the API-level-only policy surface.

use schematic_define::{ApiResponse, CrossOrigin, Endpoint, RestMethod};

#[test]
fn default_is_same_origin() {
    assert_eq!(CrossOrigin::default(), CrossOrigin::same_origin());
    assert_eq!(CrossOrigin::default(), CrossOrigin::SameOrigin);
}

#[test]
fn each_constructor_yields_its_variant_and_name() {
    let cases = [
        (
            CrossOrigin::same_origin(),
            CrossOrigin::SameOrigin,
            "same_origin",
        ),
        (
            CrossOrigin::allow_origins(["https://a.example.com"]),
            CrossOrigin::AllowOrigins(vec!["https://a.example.com".to_string()]),
            "allow_origins",
        ),
        (
            CrossOrigin::allow_without_credentials(),
            CrossOrigin::AllowWithoutCredentials,
            "allow_without_credentials",
        ),
        (
            CrossOrigin::danger_allow_any(),
            CrossOrigin::DangerAllowAny,
            "danger_allow_any",
        ),
        (CrossOrigin::deny(), CrossOrigin::Deny, "deny"),
    ];

    for (constructed, variant, name) in cases {
        assert_eq!(constructed, variant);
        assert_eq!(constructed.name(), name);
    }
}

#[test]
fn allow_origins_keeps_order_and_exact_strings() {
    let policy = CrossOrigin::allow_origins(vec![
        "https://z.example.com:8443".to_string(),
        "HTTPS://A.example.com".to_string(),
        "https://a.example.com/".to_string(),
    ]);

    assert_eq!(
        policy,
        CrossOrigin::AllowOrigins(vec![
            "https://z.example.com:8443".to_string(),
            "HTTPS://A.example.com".to_string(),
            "https://a.example.com/".to_string(),
        ])
    );
}

/// Frozen to today's surface: `Endpoint` has no cross-origin field or setter,
/// and the crate exports no GraphQL primitive.
///
/// A later spec that adds a per-endpoint override (spec 4a) or a GraphQL
/// primitive (spec 5) retires or updates this test.
#[test]
fn endpoints_have_no_cross_origin_override_and_no_graphql_export() {
    let endpoint = Endpoint {
        id: "Ping".to_string(),
        method: RestMethod::Get,
        path: "/ping".to_string(),
        description: "Ping".to_string(),
        request: None,
        response: ApiResponse::Empty,
        headers: vec![],
        params: None,
        oauth_scopes: None,
    };

    // Exhaustive, without `..`: a new `Endpoint` field fails to compile here.
    let Endpoint {
        id,
        method,
        path,
        description,
        request,
        response,
        headers,
        params,
        oauth_scopes,
    } = endpoint;
    assert_eq!(id, "Ping");
    assert_eq!(method, RestMethod::Get);
    assert_eq!(path, "/ping");
    assert_eq!(description, "Ping");
    assert!(request.is_none());
    assert_eq!(response, ApiResponse::Empty);
    assert!(headers.is_empty());
    assert!(params.is_none());
    assert!(oauth_scopes.is_none());

    let lib_source = include_str!("../src/lib.rs");
    let exported = lib_source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    assert!(
        !exported.contains("graphql"),
        "the crate root exports a GraphQL name"
    );

    let endpoint_source = include_str!("../src/types.rs");
    assert!(
        !endpoint_source.contains("fn cross_origin"),
        "`Endpoint` (src/types.rs) gained a cross-origin setter"
    );
}
