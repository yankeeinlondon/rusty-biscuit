//! Frontmatter key order through the compose pipeline's merges.
//!
//! The receiving document fixes key order: its keys keep their authored
//! positions and keys it receives (external state, a parent's inherited state,
//! a directive's `set` values) are appended in the sender's order. Value
//! precedence is unchanged. Each case composes more than once, because an
//! unordered map would return a different order on each run.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use serde_json::json;
use tempfile::TempDir;

const RUNS: usize = 4;

fn write(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn compose(root: &std::path::Path, options: ComposeOptions) -> Markdown {
    let options = options.with_source_file(root);
    let (composed, _) = Markdown::try_from(root)
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .unwrap();
    composed
}

fn top_keys(markdown: &Markdown) -> Vec<String> {
    markdown.frontmatter().as_map().keys().cloned().collect()
}

#[test]
fn external_state_is_appended_after_the_documents_own_keys() {
    let dir = TempDir::new().unwrap();
    let root = write(
        &dir,
        "doc.md",
        "---\nzeta: 1\nalpha: 2\nnested:\n  y: 1\n  b: 2\nomega: 3\n---\n{{ fm }}\n",
    );
    let external = json!({"mid": 4, "alpha": 99, "nested": {"c": 3, "b": 98}, "beta": 5});

    for _ in 0..RUNS {
        let composed = compose(&root, ComposeOptions::new().with_external_state(external.clone()));

        assert_eq!(top_keys(&composed), ["zeta", "alpha", "nested", "omega", "mid", "beta"]);
        let fm = composed.frontmatter().as_map();
        let nested: Vec<&String> = fm["nested"].as_object().unwrap().keys().collect();
        assert_eq!(nested, ["y", "b", "c"]);
        assert_eq!(fm["alpha"], json!(2), "the document's value still wins");
        assert_eq!(fm["nested"]["b"], json!(2));
        assert_eq!(
            composed.content().trim(),
            r#"{"zeta":1,"alpha":2,"nested":{"y":1,"b":2,"c":3},"omega":3,"mid":4,"beta":5}"#,
        );
    }
}

#[test]
fn transcluded_child_keeps_its_order_and_appends_inherited_parent_keys() {
    let dir = TempDir::new().unwrap();
    let root = write(
        &dir,
        "parent.md",
        "---\nzeta: p\nalpha: p\nmid: p\nprologue: never inherited\nbeta: p\n---\n::file child.md\n",
    );
    write(&dir, "child.md", "---\nown: c\nalpha: c\n---\n{{ fm }}\n");

    for _ in 0..RUNS {
        let composed = compose(&root, ComposeOptions::new());
        assert!(
            composed
                .content()
                .contains(r#"{"own":"c","alpha":"c","zeta":"p","mid":"p","beta":"p"}"#),
            "{}",
            composed.content()
        );
    }
}

#[test]
fn directive_set_values_follow_the_childs_keys_and_precede_inherited_ones() {
    let dir = TempDir::new().unwrap();
    let root = write(
        &dir,
        "parent.md",
        "---\nzeta: p\nalpha: p\n---\n::file child.md set.new_b=1 set.own=\"set\" set.new_a=2\n",
    );
    write(&dir, "child.md", "---\nown: c\nalpha: c\n---\n{{ fm }}\n");

    for _ in 0..RUNS {
        let composed = compose(&root, ComposeOptions::new());
        assert!(
            composed
                .content()
                .contains(r#"{"own":"set","alpha":"c","new_b":1,"new_a":2,"zeta":"p"}"#),
            "{}",
            composed.content()
        );
    }
}
