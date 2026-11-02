//! `claimed_values`: the non-empty strings a resolved schema claims for one
//! property, checked on the resolved lowering so inline arms, aliases,
//! `Name@file` imports, and file arms are judged alike.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use simplified_schema::embedded::ShippedSources;
use simplified_schema::resolve::{ResolvedSchema, resolve_yaml_schema};
use simplified_schema::simplified::SchemaSourcePath;
use simplified_schema::{ClaimCause, ClaimProblem, SchemaError, claimed_values};

#[cfg(feature = "file")]
const DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";

#[cfg(feature = "file")]
fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

/// Resolves the `$schema` declaration `yaml` from `dir`.
fn resolve_in(dir: &Path, yaml: &str) -> Result<ResolvedSchema, SchemaError> {
    let declaration: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("test YAML parses");
    resolve_yaml_schema(&declaration, dir, &FileResolutionContext::new(dir), ShippedSources::NONE)
}

#[track_caller]
fn claims_in(dir: &Path, yaml: &str) -> Result<Vec<String>, ClaimProblem> {
    let resolved = resolve_in(dir, yaml).unwrap_or_else(|error| panic!("`{yaml}` resolves: {error}"));
    claimed_values(&resolved, "kind")
}

#[track_caller]
fn claims(yaml: &str) -> Result<Vec<String>, ClaimProblem> {
    let dir = tempfile::tempdir().unwrap();
    claims_in(dir.path(), yaml)
}

#[track_caller]
fn single_root_problem(yaml: &str, cause: ClaimCause) {
    assert_eq!(
        claims(yaml),
        Err(ClaimProblem {
            property: "kind".into(),
            arm_index: None,
            cause,
            path: SchemaSourcePath::root().property("kind"),
        }),
        "{yaml}"
    );
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

mod inline {
    use super::*;

    #[test]
    fn a_single_root_claims_its_literal() {
        assert_eq!(claims("kind: literal(roster; required)\nname: string\n"), Ok(strings(&["roster"])));
    }

    #[test]
    fn an_enum_claims_every_member_in_order() {
        assert_eq!(claims("kind: enum(b, a; required)\n"), Ok(strings(&["b", "a"])));
    }

    #[test]
    fn a_literal_union_claims_each_literal() {
        assert_eq!(claims("kind: literal(a; required) | literal(b; required)\n"), Ok(strings(&["a", "b"])));
    }

    #[test]
    fn a_root_union_claims_the_union_of_its_arms_without_repeats() {
        let yaml = "- kind: enum(a, b; required)\n- kind: literal(c; required)\n- kind: literal(a; required)\n";
        assert_eq!(claims(yaml), Ok(strings(&["a", "b", "c"])));
    }

    /// Repeating a choice within one definition is harmless.
    #[test]
    fn repeated_claims_within_one_definition_collapse() {
        assert_eq!(claims("kind: literal(a; required) | enum(a, a; required)\n"), Ok(strings(&["a"])));
        assert_eq!(claims("- kind: literal(a; required)\n- kind: literal(a; required)\n"), Ok(strings(&["a"])));
    }
}

mod problems {
    use super::*;

    #[test]
    fn a_missing_property() {
        single_root_problem("name: string\n", ClaimCause::Missing);
    }

    #[test]
    fn an_optional_declaration_including_one_made_optional_by_null() {
        single_root_problem("kind: literal(a)\n", ClaimCause::NotRequired);
        single_root_problem("kind: null | literal(a; required)\n", ClaimCause::NotRequired);
    }

    #[test]
    fn a_declaration_that_is_not_a_finite_set() {
        single_root_problem("kind: string(required)\n", ClaimCause::NotEnumerable);
        single_root_problem("kind: string(required) | literal(a; required)\n", ClaimCause::NotEnumerable);
    }

    #[test]
    fn numeric_and_boolean_literals() {
        single_root_problem("kind: literal(1; required)\n", ClaimCause::NonStringChoice);
        single_root_problem("kind: literal(true; required)\n", ClaimCause::NonStringChoice);
    }

    #[test]
    fn an_empty_string_choice() {
        single_root_problem("kind: literal(\"\"; required)\n", ClaimCause::EmptyChoice);
    }

    /// Null admission and non-object arms cannot be authored in
    /// SimplifiedSchema with `required`; a raw JSON Schema arm can.
    #[cfg(feature = "file")]
    #[test]
    fn raw_json_arms_that_admit_null_have_no_enum_members_or_are_not_objects() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "text.json", &format!(r#"{{"$schema": "{DRAFT}", "type": "string"}}"#));
        write(
            dir.path(),
            "nullable.json",
            &format!(
                r#"{{"$schema": "{DRAFT}", "type": "object", "required": ["kind"],
                    "properties": {{"kind": {{"anyOf": [{{"const": "a"}}, {{"type": "null"}}]}}}}}}"#
            ),
        );
        write(
            dir.path(),
            "empty.json",
            &format!(
                r#"{{"$schema": "{DRAFT}", "type": "object", "required": ["kind"],
                    "properties": {{"kind": {{"enum": []}}}}}}"#
            ),
        );
        for (file, cause, path) in [
            ("text.json", ClaimCause::NotObject, SchemaSourcePath::root().union_arm(1)),
            ("nullable.json", ClaimCause::NullAdmitted, SchemaSourcePath::root().union_arm(1).property("kind")),
            ("empty.json", ClaimCause::EmptyChoice, SchemaSourcePath::root().union_arm(1).property("kind")),
        ] {
            let yaml = format!("- kind: literal(a; required)\n- ./{file}\n");
            assert_eq!(
                claims_in(dir.path(), &yaml),
                Err(ClaimProblem { property: "kind".into(), arm_index: Some(1), cause, path }),
                "{file}"
            );
        }
    }

    #[test]
    fn the_message_names_the_property_and_the_arm() {
        let single = claims("kind: string(required)\n").unwrap_err();
        assert_eq!(single.to_string(), "`kind` must be declared as `literal(...)` or `enum(...)`");
        let arm = claims("- kind: literal(a; required)\n- kind: string\n").unwrap_err();
        assert_eq!(arm.cause, ClaimCause::NotRequired);
        assert_eq!(arm.path, SchemaSourcePath::root().union_arm(1).property("kind"));
        assert_eq!(arm.to_string(), "root-union arm 1: `kind` must be declared as `literal(...)` or `enum(...)`");
    }
}

/// Arms the parse-only meta-check skips or passes provisionally are verified
/// here, on the resolved declaration.
#[cfg(feature = "file")]
mod references {
    use super::*;

    #[test]
    fn a_named_type_import_is_checked_as_resolved() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "kinds.yaml", "kind: schema\ntypes:\n  Good: enum(a, b)\n  Bad: string\n");
        assert_eq!(claims_in(dir.path(), "kind: Good(required)@./kinds.yaml\n"), Ok(strings(&["a", "b"])));
        assert_eq!(
            claims_in(dir.path(), "kind: Bad(required)@./kinds.yaml\n").map_err(|problem| problem.cause),
            Err(ClaimCause::NotEnumerable)
        );
    }

    /// A local file arm whose `kind` is a local alias.
    #[test]
    fn a_local_file_arm_and_its_alias_are_checked_as_resolved() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "roster.yaml", "types:\n  Kind: literal(roster)\n$schema:\n  kind: Kind(required)\n");
        write(dir.path(), "loose.yaml", "types:\n  Kind: string\n$schema:\n  kind: Kind(required)\n");
        assert_eq!(
            claims_in(dir.path(), "- kind: literal(team; required)\n- ./roster.yaml\n"),
            Ok(strings(&["team", "roster"]))
        );
        assert_eq!(claims_in(dir.path(), "./roster.yaml"), Ok(strings(&["roster"])));
        assert_eq!(
            claims_in(dir.path(), "- ./roster.yaml\n- ./loose.yaml\n"),
            Err(ClaimProblem {
                property: "kind".into(),
                arm_index: Some(1),
                cause: ClaimCause::NotEnumerable,
                path: SchemaSourcePath::root().union_arm(1).property("kind"),
            })
        );
    }

    /// A file arm that is itself a root union contributes each of its arms
    /// under the referencing arm's index.
    #[test]
    fn a_file_arm_holding_a_root_union_is_flattened() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "pair.yaml", "$schema:\n  - kind: literal(a; required)\n  - kind: literal(b; required)\n");
        write(dir.path(), "broken.yaml", "$schema:\n  - kind: literal(a; required)\n  - name: string\n");
        assert_eq!(claims_in(dir.path(), "- kind: literal(c; required)\n- ./pair.yaml\n"), Ok(strings(&["c", "a", "b"])));
        let problem = claims_in(dir.path(), "- kind: literal(c; required)\n- ./broken.yaml\n").unwrap_err();
        assert_eq!((problem.arm_index, problem.cause), (Some(1), ClaimCause::Missing));
    }

    /// Cycles never reach `claimed_values`: the resolver rejects them first.
    #[test]
    fn a_reference_cycle_is_the_resolver_diagnostic() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.yaml", "$schema:\n  - ./b.yaml\n");
        write(dir.path(), "b.yaml", "$schema:\n  - ./a.yaml\n");
        let error = resolve_in(dir.path(), "- kind: literal(c; required)\n- ./a.yaml\n").unwrap_err();
        assert!(matches!(error, SchemaError::ReferenceCycle { .. }), "{error:?}");
    }
}
