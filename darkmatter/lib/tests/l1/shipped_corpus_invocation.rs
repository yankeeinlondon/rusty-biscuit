//! The shipped corpus through Darkmatter: a real consumer sidecar (Claudine's
//! agent-cli research contract, with the `_types.yaml` files it imports) and a
//! real research document that binds it with `$schema: ./_schema.yaml`. The
//! files are copied into a temp directory, then judged twice: by Darkmatter's
//! document path (`DarkmatterSchemas`, as `md schema validate` and Messenger's
//! research loader call it) and by `simplified-schema`'s schema object built
//! from the same sidecar file. The two lower the sidecar identically and
//! agree on the authored document and on a one-edit invalid copy.
//!
//! `simplified-schema`'s `shipped_corpus` checks the corpus passively; this is
//! the integration case.

use std::collections::HashMap;
use std::path::Path;

use biscuit_file::canonicalize_simplified;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde_json::{Map, Value};
use simplified_schema::{SchemaSource, SimplifiedSchema};

/// The frontmatter without `$schema`, the object both judges see.
fn frontmatter(markdown: &Markdown) -> Value {
    let mut map: Map<String, Value> =
        markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
    map.remove("$schema");
    Value::Object(map)
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("fixture dir");
    std::fs::write(path, text).expect("write fixture");
}

#[test]
fn a_shipped_sidecar_judges_its_document_alike_in_darkmatter_and_the_library() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let research = root.join("research");
    write(&research.join("_types.yaml"), include_str!("../../../../claudine/docs/research/_types.yaml"));
    write(&research.join("agent-cli/_types.yaml"), include_str!("../../../../claudine/docs/research/agent-cli/_types.yaml"));
    write(&research.join("agent-cli/_schema.yaml"), include_str!("../../../../claudine/docs/research/agent-cli/_schema.yaml"));
    let authored = include_str!("../../../../claudine/docs/research/agent-cli/kimi.md");
    assert!(authored.contains("\nschema_revision: 2\n"), "the control document is revision 2");
    let page = research.join("agent-cli/kimi.md");
    let broken_page = research.join("agent-cli/broken.md");
    write(&page, authored);
    write(&broken_page, &authored.replacen("\nschema_revision: 2\n", "\nschema_revision: 3\n", 1));

    std::fs::create_dir_all(root.join("home")).expect("home dir");
    let snapshot = RequestSnapshot::new(&root).with_home(Some(root.join("home"))).with_env(HashMap::new());
    let context = build_resolution_context(&snapshot).expect("fixture context");
    let api = DarkmatterSchemas::new(context.clone());
    let library = SimplifiedSchema::try_from_yaml(SchemaSource::from(research.join("agent-cli/_schema.yaml")).bound(context))
        .expect("the sidecar loads through the library");

    let markdown = Markdown::try_from(page.as_path()).expect("the document loads");
    let effective = api.effective_for(&markdown).expect("effective schema").expect("the sidecar applies");
    assert_eq!(*effective.json_schema, *library.to_json_schema().expect("lowered"), "one lowering");

    let (report, summary) = api.validate_summarized(&markdown).expect("report");
    assert!(summary.has_schema);
    assert!(report.valid, "{:#?}", report.problems);
    let data = frontmatter(&markdown);
    assert!(library.validate_soft(&data).is_ok(), "{:?}", library.validate_soft(&data).err());

    let broken = Markdown::try_from(broken_page.as_path()).expect("the edited copy loads");
    let report = api.validate(&broken).expect("report");
    assert!(!report.valid, "a revision-3 document fails its revision-2 contract");
    assert!(report.problems.iter().any(|problem| problem.path.contains("schema_revision")), "{:#?}", report.problems);
    assert!(library.validate_soft(&frontmatter(&broken)).is_err());
}
