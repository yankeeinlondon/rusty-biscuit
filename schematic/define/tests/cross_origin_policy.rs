//! `CrossOrigin` constructors and the API-level-only policy surface.

use schematic_define::{SuccessShape, CrossOrigin, Endpoint, RestMethod};

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

#[test]
fn endpoints_support_cross_origin_override() {
    let endpoint = { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "Ping".to_string(); fixture.method = RestMethod::Get; fixture.path = "/ping".to_string(); fixture.description = "Ping".to_string(); fixture.request = None; fixture.response = vec![SuccessShape::empty()]; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture };

    // The public transfer fields remain readable after construction.
    let Endpoint {
        id,
        method,
        path,
        description,
        request,
        response,
        headers,
        params,
        cross_origin,
        pagination,
        ..
    } = endpoint;
    assert_eq!(id, "Ping");
    assert_eq!(method, RestMethod::Get);
    assert_eq!(path, "/ping");
    assert_eq!(description, "Ping");
    assert!(request.is_none());
    assert_eq!(response, vec![SuccessShape::empty()]);
    assert!(headers.is_empty());
    assert!(params.is_none());
    assert!(cross_origin.is_none());
    assert!(pagination.is_none());
    assert_eq!(Endpoint::get("Ping", "/ping").describes("Ping").returns_empty().cross_origin(CrossOrigin::deny()).build().unwrap().cross_origin, Some(CrossOrigin::deny()));
}
