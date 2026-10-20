//! Eager-file normalization through union items (`U[]`) and the same union
//! used as a property.
//!
//! An array of a union lowers to `items.anyOf`, inside the optional
//! `anyOf: [null, …]` wrapper. Normalization must apply the property-union
//! rule to each item: a value is rewritten only when exactly one validating
//! arm carries an eager `file` marker, and an object arm's nested eager file
//! fields are rewritten only when that object arm is the one such arm.
//! `eager` on the union or the container is property timing and never makes a
//! lazy `file` arm eligible.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde_json::{Value, json};

use crate::request_support::context_at;
use crate::union_fixtures::{FILE_TYPES, write_files};

/// Union types beyond the shared fixture: object arms carrying a nested
/// eager file field, tagged (one arm selected) and overlapping (both arms
/// accept `{ path }`), and a union whose two arms are both eager files.
const OBJECT_FILE_TYPES: &str = "kind: schema\ntypes:\n  \
    tagged_doc:\n    \
    - \"{ kind: literal(doc; required), path: file(eager; required) }\"\n    \
    - \"{ kind: literal(note; required), text: string(required) }\"\n  \
    overlapping_doc:\n    \
    - \"{ path: file(eager; required) }\"\n    \
    - \"{ path: file(eager; required), size: number }\"\n  \
    twin_files: [\"file(eager)\", \"file(eager; required)\"]\n";

fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// A root holding the type files and two documents' worth of targets:
/// `a.md` beside the document and `sub/b.md` below it.
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let (dir, root) = temp_root();
    write_files(
        &root,
        &[("files.yaml", FILE_TYPES), ("objects.yaml", OBJECT_FILE_TYPES), ("a.md", "# A\n")],
    );
    std::fs::create_dir_all(root.join("sub")).expect("create sub");
    std::fs::write(root.join("sub/b.md"), "# B\n").expect("write sub/b.md");
    (dir, root)
}

/// Writes `doc.md` with the inline `$schema` body and frontmatter, loads it
/// from disk, asserts it validates without touching the file, and returns
/// its normalized frontmatter.
fn normalize(root: &Path, schema: &str, frontmatter: &str) -> Value {
    let path = root.join("doc.md");
    let text = format!("---\n$schema:\n{schema}{frontmatter}---\nBody\n");
    std::fs::write(&path, &text).expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    let schemas = DarkmatterSchemas::new(context_at(root));

    let report = schemas.validate(&markdown).expect("schema resolves");
    assert!(report.valid, "{frontmatter:?}: {:?}", report.problems);
    let effective = schemas.effective_for(&markdown).expect("schema").expect("declared schema");
    let mut instance =
        serde_json::to_value(markdown.frontmatter().as_map()).expect("frontmatter as JSON");
    instance.as_object_mut().expect("mapping").remove("$schema");
    let first = effective.normalize_frontmatter(&instance, &HashSet::new());
    assert_eq!(first.changed, first.value != instance, "`changed` reports the rewrite");

    let second = effective.normalize_frontmatter(&first.value, &HashSet::new());
    assert!(!second.changed, "normalization is idempotent: {:?}", first.value);
    assert_eq!(
        serde_json::to_vec(&second.value).expect("serialize"),
        serde_json::to_vec(&first.value).expect("serialize"),
    );
    assert_eq!(std::fs::read_to_string(&path).expect("read document"), text, "validation is read-only");
    first.value
}

// ── AC 4: an unambiguous eager `file` arm inside a union item ─────────────

#[test]
fn eager_file_arm_in_union_item_is_rewritten_like_the_property() {
    let (_dir, root) = fixture();
    let normalized = normalize(
        &root,
        "  assets: \"asset[]@./files.yaml\"\n  one: \"asset@./files.yaml\"\n",
        "assets: [\"./a.md\", 3, \"./sub/../sub/b.md\"]\none: \"./sub/../sub/b.md\"\n",
    );

    assert_eq!(normalized["assets"], json!(["a.md", 3, "sub/b.md"]));
    assert_eq!(normalized["one"], json!("sub/b.md"), "item and property agree");
}

#[test]
fn required_union_array_without_null_wrapper_is_rewritten() {
    let (_dir, root) = fixture();
    let normalized = normalize(
        &root,
        "  assets: \"asset[](required; min(1))@./files.yaml\"\n",
        "assets: [\"./a.md\"]\n",
    );

    assert_eq!(normalized["assets"], json!(["a.md"]));
}

// ── `eager` on the union or container is property timing ─────────────────

#[test]
fn lazy_file_arm_is_preserved_under_union_and_container_eager() {
    let (_dir, root) = fixture();
    let schema = "  items_eager: \"lazy_asset[](eager)@./files.yaml\"\n  \
                  plain: \"lazy_asset[]@./files.yaml\"\n  \
                  one: \"lazy_asset(eager)@./files.yaml\"\n";
    let frontmatter =
        "items_eager: [\"./a.md\", 3]\nplain: [\"./a.md\"]\none: \"./sub/../sub/b.md\"\n";

    let normalized = normalize(&root, schema, frontmatter);

    assert_eq!(normalized["items_eager"], json!(["./a.md", 3]));
    assert_eq!(normalized["plain"], json!(["./a.md"]));
    assert_eq!(normalized["one"], json!("./sub/../sub/b.md"));
}

// ── Object arms: nested eager file fields ────────────────────────────────

#[test]
fn nested_file_field_of_selected_object_arm_is_rewritten() {
    let (_dir, root) = fixture();
    let normalized = normalize(
        &root,
        "  docs: \"tagged_doc[]@./objects.yaml\"\n  one: \"tagged_doc(required)@./objects.yaml\"\n",
        "docs:\n  - {kind: doc, path: ./a.md}\n  - {kind: note, text: ./a.md}\n\
         one: {kind: doc, path: ./sub/../sub/b.md}\n",
    );

    assert_eq!(
        normalized["docs"],
        json!([{ "kind": "doc", "path": "a.md" }, { "kind": "note", "text": "./a.md" }]),
        "only the eager field of the selected arm is rewritten",
    );
    assert_eq!(normalized["one"], json!({ "kind": "doc", "path": "sub/b.md" }));
}

// ── Ambiguous alternatives are not rewritten ─────────────────────────────

#[test]
fn ambiguous_item_and_property_unions_are_left_unchanged() {
    let (_dir, root) = fixture();
    let schema = "  docs: \"overlapping_doc[]@./objects.yaml\"\n  \
                  doc: \"overlapping_doc@./objects.yaml\"\n  \
                  twins: \"twin_files[]@./objects.yaml\"\n  \
                  twin: \"twin_files@./objects.yaml\"\n";
    let frontmatter = "docs: [{path: ./a.md}]\ndoc: {path: ./a.md}\n\
                       twins: [\"./a.md\"]\ntwin: \"./a.md\"\n";

    let normalized = normalize(&root, schema, frontmatter);

    assert_eq!(normalized["docs"], json!([{ "path": "./a.md" }]));
    assert_eq!(normalized["doc"], json!({ "path": "./a.md" }));
    assert_eq!(normalized["twins"], json!(["./a.md"]));
    assert_eq!(normalized["twin"], json!("./a.md"));
}

#[test]
fn overlapping_object_arms_still_rewrite_when_only_one_validates() {
    let (_dir, root) = fixture();
    let normalized = normalize(
        &root,
        "  docs: \"overlapping_doc[]@./objects.yaml\"\n",
        "docs: [{path: ./a.md}, {path: ./a.md, size: 2}]\n",
    );

    // `{path, size}` fails the closed first arm, so the second arm is the one
    // eligible arm; `{path}` alone matches both and stays.
    assert_eq!(normalized["docs"], json!([{ "path": "./a.md" }, { "path": "a.md", "size": 2 }]));
}
