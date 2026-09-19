//! `alpha` must observe the feature graph `alpha` declared, and no other.

#[test]
fn alpha_observes_the_extra_feature_it_asked_for() {
    assert!(
        shared_deps_alpha::divergent_has_extra(),
        "alpha declares `features = [\"extra\"]` on its divergent dependency"
    );
}

#[test]
fn alpha_links_the_identically_configured_dependency() {
    assert_eq!(shared_deps_alpha::common_mark(), "shared-deps-common");
}
