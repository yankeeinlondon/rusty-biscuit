//! The public surface of [`OriginPolicy`]: constructors, names, default, and
//! equality. Redirect and link decisions are covered by the unit tests in
//! `src/origin.rs`.

use schematic_runtime::OriginPolicy;

#[test]
fn constructors_build_their_named_variant() {
    let cases = [
        (OriginPolicy::same_origin(), "same_origin"),
        (OriginPolicy::allow_origins(["https://b.test"]), "allow_origins"),
        (OriginPolicy::allow_without_credentials(), "allow_without_credentials"),
        (OriginPolicy::danger_allow_any(), "danger_allow_any"),
        (OriginPolicy::deny(), "deny"),
    ];

    for (policy, name) in cases {
        assert_eq!(policy.name(), name);
    }
}

#[test]
fn default_is_same_origin() {
    assert_eq!(OriginPolicy::default(), OriginPolicy::same_origin());
}

#[test]
fn allow_origins_accepts_owned_and_borrowed_entries() {
    let borrowed = OriginPolicy::allow_origins(["https://b.test", "https://c.test"]);
    let owned = OriginPolicy::allow_origins(vec![String::from("https://b.test"), String::from("https://c.test")]);

    assert_eq!(borrowed, owned);
    assert_eq!(
        borrowed,
        OriginPolicy::AllowOrigins(vec!["https://b.test".to_owned(), "https://c.test".to_owned()])
    );
}

#[test]
fn allow_origins_equality_respects_entries() {
    assert_ne!(
        OriginPolicy::allow_origins(["https://b.test"]),
        OriginPolicy::allow_origins(["https://c.test"])
    );
    assert_ne!(OriginPolicy::allow_origins(Vec::<String>::new()), OriginPolicy::same_origin());
    assert_ne!(OriginPolicy::deny(), OriginPolicy::danger_allow_any());
}
