//! The frozen recursive-schema measurement fixtures
//! (`tests/fixtures/recursive_schema_frozen/`): the copies still match the
//! committed spike schemas, the recursive grammar stays small and
//! independent of instance depth, it agrees with the depth-5 unroll wherever
//! the unroll reaches, and the spike's generator runs from any directory.

use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::resolve::resolve_yaml_schema;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;

const RECURSIVE: &str = "lifecycle-recursive.yaml";
const UNROLLED: &str = "lifecycle.yaml";

/// The seven trigger-payload keys the spike measured, every event and `loop`.
const EVENTS: [&str; 6] = ["initialize", "start", "blocked", "success", "failure", "finalize"];

/// The effective schema of a document whose `$schema` types `success` as the
/// fixture's `stack`, so only one consumer of the grammar is resolved.
fn effective(dir: &Path, schema_file: &str, frontmatter: &Value) -> EffectiveSchema {
    let document = format!("---\n$schema:\n  success: stack@./{schema_file}\nsuccess: {frontmatter}\n---\n# fixture\n");
    let md: Markdown = document.into();
    DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .effective_for(&md)
        .expect("the fixture resolves")
        .expect("the document declares a schema")
}

/// A `success` stack whose `when`/`then` items nest `depth` levels deep, with
/// `leaf` as the one action at the bottom. Depth 0 is the bare leaf list.
fn nested(depth: usize, leaf: &Value) -> Value {
    if depth == 0 {
        return json!([leaf]);
    }
    json!([
        { "info": "level" },
        {
            "when": format!("ctx.level == {depth}"),
            "then": nested(depth - 1, leaf),
            "else": [{ "warn": format!("level {depth} false") }],
        },
    ])
}

fn frontmatter(stack: Value) -> Value {
    json!({ "success": stack })
}

/// `success` typed as the fixture's `stack`, resolved and compiled directly:
/// the grammar's own verdict, without the effective schema's diagnostics.
fn stack_validator(dir: &Path, schema_file: &str) -> jsonschema::Validator {
    let wiring: YamlValue =
        serde_yaml_ng::from_str(&format!("success: stack@./{schema_file}")).expect("the wiring parses");
    let resolved = resolve_yaml_schema(&wiring, dir, &FileResolutionContext::new(dir)).expect("the fixture resolves");
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&resolved.json_schema)
        .expect("the lowered schema compiles")
}

#[test]
fn frozen_action_table_matches_the_committed_spike_schema() {
    let frozen: &[u8] = include_bytes!("../fixtures/recursive_schema_frozen/action.yaml");
    let committed: &[u8] =
        include_bytes!("../../../../claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/schemas/action.yaml");
    assert!(frozen == committed, "the frozen action.yaml must stay a byte-for-byte copy of the spike's");
}

#[test]
fn frozen_unrolled_lifecycle_matches_the_committed_spike_schema() {
    let frozen: &[u8] = include_bytes!("../fixtures/recursive_schema_frozen/lifecycle.yaml");
    let committed: &[u8] = include_bytes!(
        "../../../../claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/schemas/lifecycle.yaml"
    );
    assert!(frozen == committed, "the frozen lifecycle.yaml must stay a byte-for-byte copy of the spike's");
}

/// Acceptance check: the recursive grammar wired into the same seven keys the
/// spike measured (39.9 MB unrolled at depth 5) serializes to at most 2 MB.
#[test]
fn recursive_fixture_resolves_to_at_most_two_megabytes() {
    let dir = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_frozen");
    let mut wiring = serde_yaml_ng::Mapping::new();
    for event in EVENTS {
        wiring.insert(event.into(), format!("stack@./{RECURSIVE}").into());
    }
    wiring.insert("loop".into(), format!("loop@./{RECURSIVE}").into());

    let resolved = resolve_yaml_schema(&YamlValue::Mapping(wiring), &dir, &FileResolutionContext::new(&dir))
        .expect("the recursive fixture resolves");
    let bytes = serde_json::to_string(&resolved.json_schema).expect("the schema serializes").len();

    println!("lifecycle-recursive.yaml, seven consumers: {bytes} bytes of JSON");
    assert!(!resolved.definitions.is_empty(), "the recursive item must lower to a definition");
    assert!(bytes <= 2_000_000, "the recursive fixture serialized to {bytes} bytes, above 2 MB");
}

#[test]
fn instance_depth_does_not_change_the_recursive_schema() {
    let dir = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_frozen");
    let leaf = json!({ "say": "leaf" });
    let shapes: Vec<(usize, String, usize)> = [3, 6, 10]
        .into_iter()
        .map(|depth| {
            let document = frontmatter(nested(depth, &leaf));
            let schema = effective(&dir, RECURSIVE, &document);
            let report = schema.validate(&document);
            assert!(report.valid, "depth {depth} must be valid: {:?}", report.problems);
            let json = serde_json::to_string(&*schema.json_schema).expect("the schema serializes");
            (depth, json, schema.definitions().len())
        })
        .collect();

    let (_, first_json, first_definitions) = &shapes[0];
    assert!(*first_definitions > 0, "the recursive item must lower to a definition");
    for (depth, json, definitions) in &shapes[1..] {
        assert_eq!(definitions, first_definitions, "depth {depth} changed the definition count");
        assert!(json == first_json, "depth {depth} changed the resolved schema");
    }
}

/// Within the unroll's five levels both grammars give every document the same
/// verdict. Invalid leaves are values no conversion could repair, so the
/// grammar's verdict is the effective schema's too.
#[test]
fn unrolled_and_recursive_fixtures_agree_within_five_levels() {
    let dir = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_frozen");
    // The unroll lowers to about 5 MB; going through `effective_for` and its
    // first rejection's diagnostics would take this test past its time budget.
    let unrolled = stack_validator(&dir, UNROLLED);
    let recursive = stack_validator(&dir, RECURSIVE);

    let valid_leaves = [
        json!({ "say": "leaf" }),
        json!({ "retry": { "max_attempts": 2, "backoff": "exponential" } }),
        json!({ "say": "a", "warn": "b", "no_error": true }),
    ];
    let invalid_leaves = [
        json!({ "say": [42] }),
        json!({ "say": { "not": "text" } }),
        json!({ "retry": { "backoff": "linear" } }),
    ];
    let cases = (1..=5).flat_map(|depth| {
        let valid = valid_leaves.iter().map(move |leaf| (depth, leaf, true));
        let invalid = invalid_leaves.iter().map(move |leaf| (depth, leaf, false));
        valid.chain(invalid)
    });

    for (depth, leaf, expected) in cases {
        let document = frontmatter(nested(depth, leaf));
        assert_eq!(unrolled.is_valid(&document), expected, "unrolled, depth {depth}, leaf {leaf}");
        assert_eq!(recursive.is_valid(&document), expected, "recursive, depth {depth}, leaf {leaf}");
    }
}

/// Past the unroll's reach only the recursive grammar is checked.
#[test]
fn recursive_fixture_checks_levels_beyond_the_unroll() {
    let dir = biscuit_test_harness::manifest_dir!().join("tests/fixtures/recursive_schema_frozen");
    let valid = frontmatter(nested(7, &json!({ "say": "leaf" })));
    let invalid = frontmatter(nested(7, &json!({ "say": [42] })));
    let schema = effective(&dir, RECURSIVE, &valid);

    let report = schema.validate(&valid);
    assert!(report.valid, "depth 7 must be valid: {:?}", report.problems);
    assert!(!schema.validate(&invalid).valid, "a bad value at depth 7 must be rejected");
}

#[test]
fn generator_names_no_absolute_path() {
    let generator =
        include_str!("../../../../claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/gen-action-schema.py");

    for marker in ["/Volumes/", "/Users/", "/home/", "/tmp/"] {
        assert!(!generator.contains(marker), "the generator names an absolute path containing {marker}");
    }
    let drive_letter = generator.as_bytes().windows(3).any(|window| {
        window[0].is_ascii_alphabetic() && window[1] == b':' && (window[2] == b'\\' || window[2] == b'/')
    });
    assert!(!drive_letter, "the generator names a drive-letter path");
    assert!(
        generator.contains("Path(__file__).resolve()"),
        "the generator must derive its paths from its own location"
    );
}

/// The same probe `scripts/ci-rollup-tests.rs` uses: a Windows Store alias
/// named `python3` exists but fails `--version`, so it is not an interpreter.
fn python_interpreter() -> Option<&'static str> {
    ["python3", "python"].into_iter().find(|candidate| {
        Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|probe| probe.status.success())
    })
}

/// Just the PyYAML surface the generator touches, emitting JSON (valid YAML).
const YAML_STUB: &str = r#"import json

class SafeDumper:
    def ignore_aliases(self, data):
        return False

    def increase_indent(self, flow=False, indentless=False):
        return None


def dump(doc, Dumper=SafeDumper, **_options):
    dumper = Dumper()
    dumper.ignore_aliases(doc)
    dumper.increase_indent(False, True)
    return json.dumps(doc, indent=2)


def safe_load(text):
    return json.loads(text)
"#;

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read temp dir") {
            let path = entry.expect("read temp entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path.strip_prefix(root).expect("under root").to_path_buf());
            }
        }
    }
    files.sort();
    files
}

/// In a fresh-checkout-shaped workspace, run from an unrelated directory, the
/// generator writes into its own adjacent `schemas/` and nowhere else.
#[test]
fn generator_writes_beside_itself_from_an_unrelated_cwd() {
    let Some(python) = python_interpreter() else {
        eprintln!("no Python interpreter is available; skipping the generator portability check");
        return;
    };
    let generator =
        include_str!("../../../../claudine/features/_completed/2026-10-20-lifecycle-grammar/spike/gen-action-schema.py");

    let temp = tempfile::tempdir().expect("temp dir");
    let repo = temp.path().join("repo");
    let launch = temp.path().join("launch");
    let stub = temp.path().join("stub");
    // Same depth below the repository root as the real generator.
    let spike = repo.join("claudine/features/_completed/x/spike");
    let inputs = [
        (spike.join("gen-action-schema.py"), generator.to_string()),
        (
            repo.join("darkmatter/lib/src/effects/catalog.rs"),
            "signature: \"notify_user(message)\",\nsignature: \"notify_user(message, level)\",\n".to_string(),
        ),
        (
            repo.join("darkmatter/docs/schemas/expression-functions.yaml"),
            json!({ "functions": [{
                "name": "upper",
                "description": "uppercase",
                "overloads": [{ "parameters": [{ "name": "text", "type": "string" }] }],
            }] })
            .to_string(),
        ),
        (stub.join("yaml.py"), YAML_STUB.to_string()),
    ];
    for (path, body) in &inputs {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create input dir");
        std::fs::write(path, body).expect("write input");
    }
    std::fs::create_dir_all(&launch).expect("create launch dir");

    let output = Command::new(python)
        .current_dir(&launch)
        .arg(spike.join("gen-action-schema.py"))
        .env("PYTHONPATH", &stub)
        .env("PYTHONUTF8", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("run the generator");
    assert!(
        output.status.success(),
        "the generator failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut expected: Vec<PathBuf> = [
        "action.yaml",
        "action-collapsed.yaml",
        "lifecycle.yaml",
        "lifecycle-d1.yaml",
        "lifecycle-d2.yaml",
        "lifecycle-d3.yaml",
        "lifecycle-d4.yaml",
        "lifecycle-collapsed.yaml",
        "lifecycle-collapsed-d3.yaml",
    ]
    .into_iter()
    .map(|name| Path::new("claudine/features/_completed/x/spike/schemas").join(name))
    .chain(inputs[..3].iter().map(|(path, _)| path.strip_prefix(&repo).expect("under repo").to_path_buf()))
    .collect();
    expected.sort();

    assert_eq!(files_under(&repo), expected, "the generator wrote outside its adjacent schemas/");
    assert!(files_under(&launch).is_empty(), "the generator wrote into its launch directory");
}
