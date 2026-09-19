//! `beta` must observe the feature graph `beta` declared, and no other.
//!
//! This assertion is the one that fails if the two packages are ever compiled
//! in a single combined Cargo invocation: `alpha`'s `extra` would unify onto
//! `beta` and this test would see a library it never asked for.

#[test]
fn beta_observes_no_feature_it_did_not_ask_for() {
    assert!(
        !shared_deps_beta::divergent_has_extra(),
        "beta declares its divergent dependency without `extra`; observing it \
         means the two packages' feature graphs were unified"
    );
}

#[test]
fn beta_links_the_identically_configured_dependency() {
    assert_eq!(shared_deps_beta::common_mark(), "shared-deps-common");
}
