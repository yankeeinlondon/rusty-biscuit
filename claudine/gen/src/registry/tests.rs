use super::*;

#[test]
fn registry_fields_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for entry in REGISTRY {
        assert!(seen.insert(entry.field), "duplicate field {}", entry.field);
    }
}

#[test]
fn registry_matches_matrix_source_counts() {
    let count = |kind: &str| {
        REGISTRY
            .iter()
            .filter(|entry| entry.source.kind() == kind)
            .count()
    };
    assert_eq!(count("roster"), 10, "roster rows");
    assert_eq!(count("research"), 11, "research rows");
    assert_eq!(count("facts"), 22, "facts rows");
    assert_eq!(REGISTRY.len(), 43, "total serialized fields");
}

#[test]
fn mapping_json_covers_every_entry() {
    let value = mapping_json();
    assert_eq!(value["fields"].as_array().unwrap().len(), REGISTRY.len());
    assert_eq!(value["provider_scope"], "all");
}

/// `supports_skills` graduated facts → research (Open question 3): the
/// registry must never declare it facts again without a matrix ruling.
#[test]
fn supports_skills_is_research_declared() {
    let entry = entry_for("supports_skills").expect("registered");
    assert!(matches!(
        entry.source,
        DeclaredSource::Research {
            topic: "skills",
            path: "support"
        }
    ));
}
