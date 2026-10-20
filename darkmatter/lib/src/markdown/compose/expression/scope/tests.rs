use super::facts::PREDICATE_FACTS;
use super::*;
use crate::markdown::compose::expression::functions::{accepts_call_arity, canonical_function_name};

/// The fact table names only registered canonical bindings that accept one
/// argument; it is not a second registry.
#[test]
fn every_predicate_fact_names_a_registered_one_argument_binding() {
    for (name, _) in PREDICATE_FACTS {
        assert_eq!(canonical_function_name(name), Some(*name), "{name} is a canonical binding");
        assert!(accepts_call_arity(name, 1), "{name} accepts one argument");
    }
}

#[test]
fn predicate_aliases_and_case_resolve_through_the_registry() {
    assert_eq!(canonical_function_name("isString"), Some("is_string"));
    assert_eq!(canonical_function_name("IS_NULL"), Some("is_null"));
    assert_eq!(canonical_function_name("fileExists"), Some("file_exists"));
    assert_eq!(canonical_function_name("AND"), Some("and"));
    assert_eq!(canonical_function_name("is_boolean"), None);
}

#[test]
fn subjects_overlap_through_ancestors_descendants_and_any_index() {
    let path = |text: &str| Subject::document_path(text).unwrap();
    assert!(path("a").overlaps(&path("a.b")));
    assert!(path("a.b").overlaps(&path("a")));
    assert!(!path("a.b").overlaps(&path("a.c")));
    assert!(!path("a").overlaps(&path("b")));
    let declared = DeclaredTypes::new(None);
    let subject = |text: &str| declared.subject(&crate::markdown::compose::expression::parse(text).unwrap()).unwrap();
    assert!(subject("a[0]").overlaps(&subject("a[-1]")));
    assert!(subject("a[0]").overlaps(&subject("a['k']")));
    assert!(subject("fm.a").overlaps(&path("a.b")));
}
