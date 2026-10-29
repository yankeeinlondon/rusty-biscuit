//! Eager-file rewriting through recursive references.
//!
//! A recursive type lowers to a root `$defs` entry used through `$ref`, so an
//! eager `file` field below the first level is reachable only by following
//! the reference. Normalization follows it under the original policy: only
//! the eager marker triggers, a union rewrites only through its one
//! validating eager arm, passive validation rewrites and runs nothing, and a
//! reference that names nothing never authorizes a rewrite.
//!
//! Every test pairs its claim with a value that is rewritten only when the
//! walker dereferences, so a walker that stops at `$ref` fails each one.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeSource};
use darkmatter::markdown::schemas::rewrite::rewrite_eager_file_values;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use serde_json::{Value, json};

use crate::request_support::context_at;

const DEPTH: usize = 7;

/// `node` carries an eager file and `lazy_node` a lazy one. `twin` is a union
/// of two recursive types that both accept a `{ doc, children }` value, so
/// neither arm can be chosen without guessing.
const TYPES: &str = "kind: schema\ntypes:\n  \
    node: \"{ label: string, doc: file(eager), children: node[](required) }\"\n  \
    lazy_node: \"{ doc: file, children: lazy_node[](required) }\"\n  \
    twin: [eager_a, eager_b]\n  \
    eager_a: \"{ doc: file(eager), children: eager_a[](required) }\"\n  \
    eager_b: \"{ doc: file(eager), size: number, children: eager_b[](required) }\"\n";

const AUTHORED: &str = "./sub/../sub/b.md";
const RESOLVED: &str = "sub/b.md";

/// A temp root with the type library and the eager target `sub/b.md`.
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::write(root.join("types.yaml"), TYPES).expect("write types");
    std::fs::create_dir_all(root.join("sub")).expect("create sub");
    std::fs::write(root.join("sub/b.md"), "# B\n").expect("write target");
    (dir, root)
}

/// A `{ doc, children }` chain `DEPTH` levels deep whose only `doc` is on the
/// deepest node, with `extra` merged into every node.
fn tree(leaf_doc: &str, extra: &Value) -> Value {
    let node = |mut body: Value| {
        for (key, value) in extra.as_object().expect("extra is a mapping") {
            body[key] = value.clone();
        }
        body
    };
    (1..DEPTH).fold(node(json!({"doc": leaf_doc, "children": []})), |child, _| {
        node(json!({"children": [child]}))
    })
}

fn deep_doc(value: &Value) -> &Value {
    let pointer = format!("{}/doc", "/children/0".repeat(DEPTH - 1));
    value.pointer(&pointer).unwrap_or_else(|| panic!("no node at {pointer}: {value}"))
}

/// Writes `doc.md` with `schema` and `frontmatter` (JSON is valid YAML) and
/// loads it from disk.
fn document(root: &Path, schema: &str, frontmatter: &Value) -> (PathBuf, Markdown) {
    let path = root.join("doc.md");
    let text = format!("---\n$schema:\n{schema}{}\n---\nBody\n", serde_yaml_ng::to_string(frontmatter).expect("yaml"));
    std::fs::write(&path, &text).expect("write document");
    let markdown = Markdown::try_from(path.as_path()).expect("document loads").with_source(ComposeSource::File(path.clone()));
    (path, markdown)
}

fn effective(root: &Path, markdown: &Markdown) -> EffectiveSchema {
    let schemas = DarkmatterSchemas::new(context_at(root));
    let report = schemas.validate(markdown).expect("schema resolves");
    assert!(report.valid, "{:#?}", report.problems);
    schemas.effective_for(markdown).expect("schema").expect("declared schema")
}

fn instance(markdown: &Markdown) -> Value {
    let mut value = serde_json::to_value(markdown.frontmatter().as_map()).expect("frontmatter as JSON");
    value.as_object_mut().expect("mapping").remove("$schema");
    value
}

#[test]
fn composition_rewrites_an_eager_file_seven_levels_into_a_recursive_type() {
    let (_dir, root) = fixture();
    let frontmatter = json!({"tree": tree(AUTHORED, &json!({"label": "x"}))});
    let (path, _) = document(&root, "  tree: node(required)@./types.yaml\n", &frontmatter);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&root, ""))
        .with_source_file(path.clone());

    let (composed, _) = Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with(&crate::request_support::request(options))
        .unwrap_or_else(|error| panic!("{error}"));

    let tree = composed.frontmatter().as_map().get("tree").cloned().expect("tree survives composition");
    assert_eq!(deep_doc(&tree), &json!(RESOLVED));
    assert_eq!(tree["label"], json!("x"), "only the eager field changes");
}

#[test]
fn lazy_and_ambiguous_alternatives_through_a_reference_are_unchanged() {
    let (_dir, root) = fixture();
    let schema = "  tree: node(required)@./types.yaml\n  \
                  lazy: lazy_node(required)@./types.yaml\n  \
                  pair: twin(required)@./types.yaml\n  \
                  sized: twin(required)@./types.yaml\n";
    let frontmatter = json!({
        "tree": tree(AUTHORED, &json!({})),
        "lazy": tree(AUTHORED, &json!({})),
        "pair": tree(AUTHORED, &json!({})),
        "sized": tree(AUTHORED, &json!({"size": 1})),
    });
    let (_, markdown) = document(&root, schema, &frontmatter);

    let outcome = effective(&root, &markdown).normalize_frontmatter(&instance(&markdown), &HashSet::new());

    assert_eq!(deep_doc(&outcome.value["lazy"]), &json!(AUTHORED), "a lazy file is never a trigger");
    assert_eq!(outcome.value["pair"], frontmatter["pair"], "both eager arms accept, so neither is chosen");
    assert_eq!(deep_doc(&outcome.value["tree"]), &json!(RESOLVED));
    assert_eq!(deep_doc(&outcome.value["sized"]), &json!(RESOLVED), "only one eager arm accepts `size`");
}

#[test]
fn passive_validation_rewrites_and_runs_nothing() {
    let (_dir, root) = fixture();
    let marker = root.join("ran.txt");
    let label = format!("$(touch {})", biscuit_file::to_portable_string(&marker));
    let frontmatter = json!({"tree": tree(AUTHORED, &json!({"label": label}))});
    let (path, markdown) = document(&root, "  tree: node(required)@./types.yaml\n", &frontmatter);
    let text = std::fs::read_to_string(&path).expect("read document");
    #[cfg(feature = "effects-instrumentation")]
    let before = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());

    let effective = effective(&root, &markdown);
    let report = effective.validate(&instance(&markdown));

    assert!(report.valid, "{:#?}", report.problems);
    assert_eq!(std::fs::read_to_string(&path).expect("read document"), text, "the document is not written");
    assert_eq!(instance(&markdown), frontmatter, "the frontmatter is not rewritten");
    assert!(!marker.exists(), "the shell expression is not executed");
    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(
        (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()),
        before,
        "validation builds no effect engine",
    );
    let normalized = effective.normalize_frontmatter(&instance(&markdown), &HashSet::new());
    assert_eq!(deep_doc(&normalized.value["tree"]), &json!(RESOLVED), "normalization had a value to rewrite");
}

#[test]
fn repeated_normalization_through_a_recursive_type_is_idempotent() {
    let (_dir, root) = fixture();
    let frontmatter = json!({"tree": tree(AUTHORED, &json!({}))});
    let (_, markdown) = document(&root, "  tree: node(required)@./types.yaml\n", &frontmatter);
    let effective = effective(&root, &markdown);

    let first = effective.normalize_frontmatter(&instance(&markdown), &HashSet::new());
    let second = effective.normalize_frontmatter(&first.value, &HashSet::new());

    assert!(first.changed);
    assert_eq!(deep_doc(&first.value["tree"]), &json!(RESOLVED));
    assert!(!second.changed, "{}", first.value);
    assert_eq!(second.value, first.value);
}

#[test]
fn a_missing_reference_target_never_authorizes_a_rewrite() {
    let (_dir, root) = fixture();
    let eager = json!({"type": "string", "format": "darkmatter-file"});
    let schema = json!({
        "type": "object",
        "properties": {
            "found": {"$ref": "#/$defs/node"},
            "missing": {"$ref": "#/$defs/absent", "format": "darkmatter-file"},
            "hop": {"$ref": "#/$defs/hop"},
            "arm": {"anyOf": [{"$ref": "#/$defs/absent", "format": "darkmatter-file"}, {"type": "number"}]},
        },
        "$defs": {
            "node": {"type": "object", "properties": {"doc": eager, "next": {"$ref": "#/$defs/node"}}},
            "hop": {"$ref": "#/$defs/absent", "properties": {"doc": eager}},
        },
    });
    let chain = |depth: usize| {
        (1..depth).fold(json!({"doc": AUTHORED}), |next, _| json!({"next": next}))
    };
    let input = json!({"found": chain(DEPTH), "missing": AUTHORED, "hop": {"doc": AUTHORED}, "arm": AUTHORED});

    let outcome = rewrite_eager_file_values(&schema, &input, &root, None, &HashSet::new(), &context_at(&root));

    let found = format!("/found{}/doc", "/next".repeat(DEPTH - 1));
    assert_eq!(outcome.value.pointer(&found), Some(&json!(RESOLVED)));
    assert_eq!(outcome.value["missing"], json!(AUTHORED), "a sibling marker beside a dangling reference");
    assert_eq!(outcome.value["hop"], input["hop"], "a chain that dangles after an eager hop");
    assert_eq!(outcome.value["arm"], json!(AUTHORED), "an eager union arm whose reference dangles");
}
