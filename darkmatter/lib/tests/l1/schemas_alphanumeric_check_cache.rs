//! Lifetime of the `alphanumeric` default/suggestion check: equal complete
//! constraints (the option pattern plus every string constraint) share one
//! compiled validator across resolutions, different options compile and
//! judge apart, and every default or suggestion is judged afresh, so a
//! rejected value stays rejected with the same diagnostic.
//!
//! The process-wide validator cache is shared by every test in a process, so
//! each case declares constraints no other test uses. Compile counts are
//! asserted only with the `work-counters` feature; acceptance, rejection, and
//! diagnostics are asserted either way.

use std::path::Path;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::SchemaError;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use serde_yaml_ng::Value as YamlValue;

/// Writes `types.yaml` declaring `types` and resolves a schema importing the
/// type `slug` from it into `imports` properties.
fn resolve_imports(dir: &Path, types: &str, imports: usize) -> Result<ResolvedSchema, SchemaError> {
    std::fs::write(dir.join("types.yaml"), format!("kind: schema\ntypes:\n{types}")).expect("write types");
    let mut wiring = serde_yaml_ng::Mapping::new();
    for index in 0..imports {
        wiring.insert(format!("p{index}").into(), "slug@./types.yaml".into());
    }
    resolve_yaml_schema(&YamlValue::Mapping(wiring), dir, &FileResolutionContext::new(dir))
}

/// Resolves an inline schema declaring one property `p` as `definition`.
fn resolve_inline(dir: &Path, definition: &str) -> Result<ResolvedSchema, SchemaError> {
    let schema: YamlValue = serde_yaml_ng::from_str(&format!("p: \"{definition}\"\n")).expect("fixture YAML parses");
    resolve_yaml_schema(&schema, dir, &FileResolutionContext::new(dir))
}

/// The counters a compile count is measured from; nothing without the
/// `work-counters` feature, where [`assert_compiles`] checks nothing.
#[cfg(feature = "work-counters")]
type Compiles = darkmatter::work_counters::WorkSnapshot;
#[cfg(not(feature = "work-counters"))]
struct Compiles;

fn compiles_now() -> Compiles {
    #[cfg(feature = "work-counters")]
    return darkmatter::work_counters::snapshot();
    #[cfg(not(feature = "work-counters"))]
    Compiles
}

/// Asserts that `expected` alphanumeric checks were compiled since `before`.
#[cfg_attr(not(feature = "work-counters"), allow(unused_variables))]
#[track_caller]
fn assert_compiles(label: &str, before: &Compiles, expected: u64) {
    #[cfg(feature = "work-counters")]
    assert_eq!(
        darkmatter::work_counters::snapshot()
            .since(before)
            .get(darkmatter::work_counters::Work::AlphanumericCheckCompile),
        expected,
        "{label}: alphanumeric checks compiled"
    );
}

/// The conversion failure a rejected `default`/`suggest` value reports for
/// `property`.
fn rejected(property: &str, keyword: &str, value: &str) -> SchemaError {
    SchemaError::Convert {
        property: property.to_string(),
        message: format!("`{keyword}({value:?})` is not a valid `alphanumeric` value for this type"),
    }
}

/// An imported type's rejected suggestion: reported by the lint of the file
/// that declares the type, naming that file and the type.
fn rejected_in_types_file(dir: &Path, keyword: &str, value: &str) -> String {
    let lint = SchemaError::SchemaDocument {
        path: dir.join("types.yaml"),
        message: format!("could not lint SimplifiedSchema payload: {}", rejected("slug", keyword, value)),
    };
    format!("{lint:?}")
}

#[test]
fn suggested_imports_compile_one_alphanumeric_check_whatever_the_import_count() {
    for imports in [1usize, 10, 100] {
        // A bound unique to this count keeps each batch cold.
        let types = format!("  slug: \"alphanumeric(allow-dashes; max({}); suggest(qa, qb-c))\"\n", 4_100 + imports);
        let dir = tempfile::tempdir().expect("temp dir");

        let cold = compiles_now();
        let resolved = resolve_imports(dir.path(), &types, imports).expect("valid suggestions load");
        assert_compiles(&format!("{imports} imports, cold"), &cold, 1);
        assert_eq!(resolved.json_schema["properties"].as_object().map(|map| map.len()), Some(imports));

        // A later resolution in a fresh directory reuses the validator.
        let other = tempfile::tempdir().expect("temp dir");
        let warm = compiles_now();
        resolve_imports(other.path(), &types, imports).expect("valid suggestions load again");
        assert_compiles(&format!("{imports} imports, warm"), &warm, 0);
    }
}

#[test]
fn different_alphanumeric_options_compile_and_judge_distinctly() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dashed = "  slug: \"alphanumeric(allow-dashes; max(4201); suggest(ra, rb-c))\"\n";
    let plain = "  slug: \"alphanumeric(max(4201); suggest(ra, rb-c))\"\n";

    let before = compiles_now();
    resolve_imports(dir.path(), dashed, 3).expect("`allow-dashes` accepts `rb-c`");
    assert_compiles("dashed options", &before, 1);

    let before = compiles_now();
    let error = resolve_imports(dir.path(), plain, 3).expect_err("without `allow-dashes`, `rb-c` is rejected");
    assert_compiles("plain options", &before, 1);
    assert_eq!(format!("{error:?}"), rejected_in_types_file(dir.path(), "suggest", "rb-c"));

    // Both validators are warm now; neither borrows the other's verdict.
    let before = compiles_now();
    resolve_imports(dir.path(), dashed, 3).expect("`allow-dashes` still accepts `rb-c`");
    let again = resolve_imports(dir.path(), plain, 3).expect_err("the plain type still rejects `rb-c`");
    assert_eq!(format!("{again:?}"), rejected_in_types_file(dir.path(), "suggest", "rb-c"));
    assert_compiles("both warm", &before, 0);
}

#[test]
fn a_rejected_alphanumeric_default_stays_rejected_with_the_same_diagnostic() {
    let dir = tempfile::tempdir().expect("temp dir");
    let types = "  slug: \"alphanumeric(leading-alpha; max(4301); default(1s))\"\n";
    // A default is reported by each importing property.
    let expected = format!(
        "{:?}",
        SchemaError::Aggregate { errors: vec![rejected("p0", "default", "1s"), rejected("p1", "default", "1s")] }
    );

    let before = compiles_now();
    let first = resolve_imports(dir.path(), types, 2).expect_err("`leading-alpha` rejects `1s`");
    assert_eq!(format!("{first:?}"), expected);
    let second = resolve_imports(dir.path(), types, 2).expect_err("the warm check rejects `1s` again");
    assert_eq!(format!("{second:?}"), expected);
    assert_compiles("cold then warm", &before, 1);

    // An accepted default under the same constraints shares the validator
    // and is judged on its own value.
    let before = compiles_now();
    resolve_imports(dir.path(), "  slug: \"alphanumeric(leading-alpha; max(4301); default(s1))\"\n", 2)
        .expect("`leading-alpha` accepts `s1`");
    assert_compiles("same constraints, another default", &before, 0);
}

#[test]
fn inline_alphanumeric_checks_keep_their_property_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    for _ in 0..2 {
        let error = resolve_inline(dir.path(), "alphanumeric(min(4401); suggest(tt))").expect_err("`tt` is too short");
        assert_eq!(format!("{error:?}"), format!("{:?}", rejected("p", "suggest", "tt")));
        assert!(error.to_string().contains("for `p`"), "{error}");
    }
}
