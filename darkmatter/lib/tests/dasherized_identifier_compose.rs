//! Compose-level coverage for Requirement 1 of the dasherized-identifiers spec
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`): a kebab-case
//! frontmatter key is reachable bare, as `doc.<key>`, and as `doc['<key>']`,
//! while unspaced subtraction after a non-identifier operand still evaluates.

use darkmatter::markdown::Markdown;
use serde_json::{Value, json};

fn compose(content: &str) -> Markdown {
    let markdown: Markdown = content.into();
    markdown.compose().expect("document composes").0
}

fn frontmatter_value(document: &Markdown, key: &str) -> Value {
    document
        .frontmatter()
        .as_map()
        .get(key)
        .cloned()
        .unwrap_or_else(|| panic!("composed frontmatter lost `{key}`"))
}

#[test]
fn kebab_key_resolves_the_same_bare_dotted_and_bracketed() {
    let composed = compose(
        "---\nspec-name: alpha\n---\nBare: [{{ spec-name }}] Doc: [{{ doc.spec-name }}] Bracket: [{{ doc['spec-name'] }}]\n",
    );
    assert!(
        composed.content().contains("Bare: [alpha] Doc: [alpha] Bracket: [alpha]"),
        "{}",
        composed.content()
    );
}

#[test]
fn native_and_quoted_yaml_keys_and_scalar_types_all_resolve() {
    let composed = compose(
        "---\n\"quoted-key\": 7\nis-draft: false\nnative-key: 'text'\n---\n\
         Q: [{{ quoted-key }}|{{ doc.quoted-key }}|{{ doc[\"quoted-key\"] }}]\n\
         N: [{{ native-key }}] B: [{{ is-draft ? 'yes' : 'no' }}] Sum: [{{ quoted-key + 1 }}]\n",
    );
    let content = composed.content();
    assert!(content.contains("Q: [7|7|7]"), "{content}");
    assert!(content.contains("N: [text] B: [no] Sum: [8]"), "{content}");
}

#[test]
fn kebab_key_resolves_in_frontmatter_mixed_text_and_survives_a_round_trip() {
    let source = "---\nspec-name: alpha\nlabel: \"v={{ spec-name }}/{{ doc.spec-name }}\"\n---\nLabel: {{ label }}\n";
    let composed = compose(source);
    assert_eq!(frontmatter_value(&composed, "label"), json!("v=alpha/alpha"));
    assert!(composed.content().contains("Label: v=alpha/alpha"), "{}", composed.content());

    // Written back out and composed again, the resolved value is stable.
    let rewritten = composed.as_string();
    let recomposed = compose(&rewritten);
    assert_eq!(frontmatter_value(&recomposed, "label"), json!("v=alpha/alpha"));
    assert_eq!(frontmatter_value(&recomposed, "spec-name"), json!("alpha"));
    assert_eq!(recomposed.as_string(), rewritten);
}

#[test]
fn kebab_key_drives_a_when_condition() {
    let composed = compose(
        "---\nis-draft: false\nhas-review: true\n---\n\
         ::block when=\"is-draft\"\nDRAFT\n::end-block\n\
         ::block when=\"!is-draft && has-review\"\nFINAL\n::end-block\n",
    );
    let content = composed.content();
    assert!(content.contains("FINAL"), "{content}");
    assert!(!content.contains("DRAFT"), "{content}");
}

#[test]
fn subtraction_still_evaluates_where_the_left_operand_cannot_continue_a_name() {
    let composed = compose(
        "---\n_loop_count: 3\nphase: 5\nphase-2: two\nitems: [10, 20]\n---\n\
         Arith: [{{ _loop_count - 1 }}|{{ 4-2 }}|{{ length(items)-1 }}|{{ items[0]-1 }}|{{ (phase)-1 }}|{{ phase - 2 }}|{{ phase-2 }}]\n",
    );
    assert!(
        composed.content().contains("Arith: [2|2|1|9|4|3|two]"),
        "{}",
        composed.content()
    );
}

#[test]
fn unspaced_identifier_subtraction_now_names_a_kebab_key() {
    // The breaking row: `iteration-1` is one identifier now. With no such key
    // it resolves to nothing rather than to `iteration` minus one.
    let composed = compose("---\niteration: 3\n---\nPrev: [{{ iteration-1 }}] Spaced: [{{ iteration - 1 }}]\n");
    assert!(
        composed.content().contains("Prev: [] Spaced: [2]"),
        "{}",
        composed.content()
    );
}

#[test]
fn kebab_key_works_as_an_unquoted_object_literal_key() {
    let composed = compose("---\nx: 1\n---\nObj: [{{ { spec-name: 'k' }.spec-name }}]\n");
    assert!(composed.content().contains("Obj: [k]"), "{}", composed.content());
}
