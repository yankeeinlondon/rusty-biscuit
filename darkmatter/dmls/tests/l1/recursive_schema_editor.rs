//! Editing below a recursive named type in DMLS, over the in-memory LSP
//! harness: completion and hover at recursion levels 1 to 3 (empty items and
//! key-less objects included), lazy definition lookup, whole-type hover with
//! its named back-reference, go-to-definition across a two-file cycle, a deep
//! validation failure at its node, non-descending cycles as schema
//! diagnostics on their references, and same-named definitions in two
//! triggers' inline payloads kept apart. The same surfaces cover an acyclic
//! type stored once because several uses share it: an imported object at two
//! properties and, through an alias, a list item, and a shared union.
//!
//! Every completion and hover assertion below a recursion point offers
//! nothing if DMLS does not open the `TypeExpr::Ref` it stands on.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

use crate::union_item_editor::{Document, initialize_params, labels, open_text, position, request};

/// `item` recurses through `children` and, with `meta`, through `meta.owner`;
/// `tag` recurses on its own. A cursor path through `children` reaches only
/// `item`.
const SCHEMA: &str = concat!(
    "$schema:\n",
    "  item: \"{ children: item[](required), meta: meta, ",
    "value: string(required; suggest(alpha, beta)) -> The node text }\"\n",
    "  meta: \"{ owner: item, tags: tag[] }\"\n",
    "  tag: \"{ label: string, sub: tag[] }\"\n",
    "  tree: item(required) -> The root node\n",
);

/// A tree four items deep: `a` is level 0, `b` level 1, `c` level 2, and `d`
/// level 3.
const TREE: &str = concat!(
    "tree:\n",
    "  value: a\n",
    "  children:\n",
    "    - value: b\n",
    "      children:\n",
    "        - value: c\n",
    "          children:\n",
    "            - value: d\n",
    "              children: []\n",
);

fn markdown(frontmatter: &str) -> String {
    format!("---\n{frontmatter}---\n\nBody\n")
}

fn session(workspace: &LspWorkspace) -> LspFixture<'_> {
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(workspace.path()));
    fixture
}

/// The byte offset of the `occurrence`-th (0-based) `needle`, plus `skip`.
fn offset(text: &str, needle: &str, occurrence: usize, skip: usize) -> usize {
    text.match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("`{needle}` #{occurrence} in {text:?}"))
        .0
        + skip
}

/// The LSP range of the `occurrence`-th `needle` in `text`.
fn range_of(text: &str, needle: &str, occurrence: usize) -> Value {
    let start = offset(text, needle, occurrence, 0);
    json!({ "start": position(text, start), "end": position(text, start + needle.len()) })
}

/// The range of the type name the first `needle` starts with.
fn name_range(text: &str, needle: &str) -> Value {
    let start = offset(text, needle, 0, 0);
    let name = needle
        .find(|character: char| !(character.is_alphanumeric() || character == '_'))
        .unwrap_or(needle.len());
    json!({ "start": position(text, start), "end": position(text, start + name) })
}

/// `(label, kind, detail)` of every completion item at `offset`.
fn completion_at(fixture: &mut LspFixture<'_>, document: &Document, offset: usize) -> Vec<(String, u64, Option<String>)> {
    let items = request(fixture, "textDocument/completion", document, offset);
    items
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|item| {
            (
                item["label"].as_str().unwrap().to_string(),
                item["kind"].as_u64().unwrap_or_default(),
                item["detail"].as_str().map(str::to_string),
            )
        })
        .collect()
}

fn hover_at(fixture: &mut LspFixture<'_>, document: &Document, offset: usize) -> String {
    let hover = request(fixture, "textDocument/hover", document, offset);
    hover["contents"]["value"]
        .as_str()
        .unwrap_or_else(|| panic!("no hover at {offset}: {hover:?}"))
        .to_string()
}

/// The item keys with their `(required)` detail, sorted by label.
fn item_keys(items: &[(String, u64, Option<String>)]) -> Vec<(&str, Option<&str>)> {
    let mut keys: Vec<(&str, Option<&str>)> = items
        .iter()
        .map(|(label, _, detail)| (label.as_str(), detail.as_deref()))
        .collect();
    keys.sort_unstable();
    keys
}

const ITEM_KEYS: [(&str, Option<&str>); 3] =
    [("children", Some("(required)")), ("meta", None), ("value", Some("(required)"))];

/// `items` offers `meta` and nothing but item keys, each with its presence.
fn assert_item_keys(items: &[(String, u64, Option<String>)], case: &str) {
    let keys = item_keys(items);
    assert!(keys.contains(&("meta", None)), "{case}: {items:?}");
    assert!(keys.iter().all(|key| ITEM_KEYS.contains(key)), "{case}: {items:?}");
}

/// The value of item `level` (`a` … `d`).
fn level_value(level: usize) -> &'static str {
    ["a", "b", "c", "d"][level]
}

/// Acceptance 3: at every recursion level the keys of the referenced
/// definition are offered (with their presence) and its leaf's suggestions
/// are offered as values. `meta` is the one key no item has written yet;
/// whether written keys are also offered is the ordinary nested-key rule.
#[test]
fn completion_offers_the_definition_at_recursion_levels_one_two_and_three() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "tree.md", &markdown(&format!("{SCHEMA}{TREE}")));

    for level in 0..=3 {
        // The start of the item's `children:` key line: the key position of
        // an object at this level.
        let key_line = format!("value: {}\n", level_value(level));
        let column = 2 + 4 * level;
        let at = offset(&document.text, &key_line, 0, key_line.len() + column);
        let items = completion_at(&mut fixture, &document, at);
        assert_item_keys(&items, &format!("level {level}"));

        let at = offset(&document.text, &format!("value: {}", level_value(level)), 0, "value: ".len());
        let items = completion_at(&mut fixture, &document, at);
        assert_eq!(labels(&items), ["alpha", "beta"], "level {level} value suggestions: {items:?}");
    }

    fixture.shutdown();
}

/// Acceptance 3 and spec design 8: an empty `- ` item and an object with no
/// keys yet offer the next keys of the recursive definition, at levels 0
/// through 3.
#[test]
fn an_empty_item_and_a_key_less_object_offer_the_next_keys() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let frontmatter = concat!(
        "tree:\n",
        "  value: a\n",
        "  children:\n",
        "    - \n",
        "    - value: b\n",
        "      children:\n",
        "        - \n",
        "        - value: c\n",
        "          children:\n",
        "            - \n",
        "other: \n",
    );
    let (document, _) = open_text(&mut fixture, &workspace, "empty.md", &markdown(&format!("{SCHEMA}{frontmatter}")));

    for (level, dash) in [(1, "\n    - \n"), (2, "\n        - \n"), (3, "\n            - \n")] {
        let at = offset(&document.text, dash, 0, dash.len() - 1);
        let items = completion_at(&mut fixture, &document, at);
        assert_eq!(item_keys(&items), ITEM_KEYS, "empty item at level {level}: {items:?}");
    }

    // `tree:` with no keys yet, the cursor on the indented line below it.
    let (document, _) = open_text(
        &mut fixture,
        &workspace,
        "keyless.md",
        &markdown(&format!("{SCHEMA}tree:\n  \nother: 1\n")),
    );
    let at = offset(&document.text, "tree:\n  ", 0, "tree:\n  ".len());
    let items = completion_at(&mut fixture, &document, at);
    assert_eq!(item_keys(&items), ITEM_KEYS, "a key-less object: {items:?}");

    fixture.shutdown();
}

/// Acceptance 3: hover on a leaf below recursion describes it from the
/// referenced definition, and hover on a recursive property at each level
/// names its type and shows the definition once.
#[test]
fn hover_describes_the_definition_at_recursion_levels_one_two_and_three() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "tree.md", &markdown(&format!("{SCHEMA}{TREE}")));

    for level in 1..=3 {
        let markup = hover_at(&mut fixture, &document, offset(&document.text, &format!("value: {}", level_value(level)), 0, 0));
        assert!(markup.starts_with("**`value`**"), "level {level}: {markup}");
        assert!(markup.contains("Type: **string**"), "level {level}: {markup}");
        assert!(markup.contains("Required"), "level {level}: {markup}");
        assert!(markup.contains("The node text"), "level {level}: {markup}");

        let markup = hover_at(&mut fixture, &document, offset(&document.text, "children:", level, 0));
        assert!(markup.contains("Type: **item[](required)**"), "level {level}: {markup}");
        assert!(markup.contains("`item` is defined as: `{children: item[](required)"), "level {level}: {markup}");
    }

    fixture.shutdown();
}

/// Acceptance 3 laziness, at the DMLS level: completion and hover at level
/// `n` open only `item`, never `meta` or `tag`, which every item declares and
/// an eager expansion of `item` would reach. Beyond the per-request constant
/// (each request rebuilds the overlay's authoring state, which walks the
/// authored mapping keys once), the completion walk opens `item` at least
/// once and at most twice per reference on the cursor path: once to reach the
/// item, once for the keys offered there. Hover grows by at most two lookups
/// per level.
///
/// DMLS opens every reference through its bundle's `DefinitionLookup`, whose
/// lookups `work_counters::record_definition_lookups` records; the library
/// function behind each step (`resolved_alternatives`) is proven to open one
/// definition per reference by the library's
/// `recursive_schema_completion::descent_opens_only_the_definitions_on_the_cursor_path`.
#[test]
fn completion_and_hover_open_only_the_definitions_on_the_cursor_path() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "tree.md", &markdown(&format!("{SCHEMA}{TREE}")));
    fixture.flush_server();

    let record = |fixture: &mut LspFixture<'_>, request: &mut dyn FnMut(&mut LspFixture<'_>)| {
        dmls::work_counters::record_definition_lookups();
        request(fixture);
        let opened = dmls::work_counters::definition_lookups();
        assert!(opened.iter().all(|name| name == "item"), "only `item` is opened: {opened:?}");
        opened.len()
    };
    // A top-level key completion descends nothing: the per-request constant.
    let top_level = offset(&document.text, "tree:\n", 0, 0);
    let constant = record(&mut fixture, &mut |fixture| {
        assert!(!completion_at(fixture, &document, top_level).is_empty());
    });

    let mut previous_hover = None;
    for level in 0..=3 {
        let key_line = format!("value: {}\n", level_value(level));
        let at = offset(&document.text, &key_line, 0, key_line.len() + 2 + 4 * level);
        let opened = record(&mut fixture, &mut |fixture| {
            assert_item_keys(&completion_at(fixture, &document, at), &format!("level {level}"));
        });
        let path = level + 1;
        assert!(
            (path..=2 * path).contains(&(opened - constant)),
            "completion at level {level} opened {opened} definitions over a constant {constant}"
        );

        let at = offset(&document.text, &format!("value: {}", level_value(level)), 0, 0);
        let opened = record(&mut fixture, &mut |fixture| {
            let markup = hover_at(fixture, &document, at);
            assert!(markup.contains("The node text"), "level {level}: {markup}");
        });
        // The item definition of `children` is one recursive node at every
        // depth, synthesized once per snapshot, so a deeper hover may open
        // nothing the level above did not.
        if let Some(previous) = previous_hover {
            assert!(
                opened >= previous && opened - previous <= 2,
                "hover at level {level} opened {opened} definitions, {previous} one level up"
            );
        }
        previous_hover = Some(opened);
    }

    fixture.shutdown();
}

/// A recursive type declared by a configured extension baseline is
/// descended too: the extension's definitions are carried beside the
/// effective schema's, which cannot see them once the extension is merged
/// into the baseline.
#[test]
fn an_extension_baseline_recursive_type_is_descended() {
    let workspace = LspWorkspace::new();
    write(
        &workspace,
        ".dmls.toml",
        "[schema.extensions.tree]\npath = \"tree-extension.yaml\"\nglobs = [\"tree-*.md\"]\n",
    );
    let (_, extension_schema) = SCHEMA.split_once('\n').unwrap();
    write(&workspace, "tree-extension.yaml", &format!("$schema:\n{extension_schema}"));
    let mut fixture = session(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "tree-doc.md", &markdown(TREE));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    for level in 1..=3 {
        let key_line = format!("value: {}\n", level_value(level));
        let at = offset(&document.text, &key_line, 0, key_line.len() + 2 + 4 * level);
        assert_item_keys(&completion_at(&mut fixture, &document, at), &format!("level {level}"));
    }

    fixture.shutdown();
}

/// Whole-type hover renders the recursive type once, with its recursive uses
/// as named back-references: on the property, on a mutually recursive
/// definition, and from a body interpolation.
#[test]
fn whole_type_hover_names_the_recursive_back_reference() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let tree = TREE.replace("    - value: b\n", "    - value: b\n      meta:\n        tags: []\n");
    let text = markdown(&format!("{SCHEMA}{tree}")).replace("Body", "{{ tree }}");
    let (document, _) = open_text(&mut fixture, &workspace, "tree.md", &text);

    let markup = hover_at(&mut fixture, &document, offset(&document.text, "tree:\n", 0, 0));
    assert_eq!(
        markup,
        concat!(
            "**`tree`**\n\n",
            "Type: **item(required)**\n\n",
            "`item` is defined as: `{children: item[](required),meta: meta,",
            "value: string(required;suggest(alpha,beta)) -> The node text}`\n\n",
            "Required\n\n\n\n",
            "The root node",
        )
    );

    let markup = hover_at(&mut fixture, &document, offset(&document.text, "meta:\n        tags", 0, 0));
    assert!(markup.contains("`meta` is defined as: `{owner: item,tags: tag[]}`"), "{markup}");

    let markup = hover_at(&mut fixture, &document, offset(&document.text, "{{ tree }}", 0, 3));
    assert!(markup.contains("`item` is defined as: `{children: item[](required)"), "{markup}");

    fixture.shutdown();
}

/// A union whose arm is a recursive reference lists that arm by name with
/// the keys of the object it defines, rather than as an opaque name; an item
/// of the union list is judged and described through the definition.
#[test]
fn a_recursive_union_arm_is_listed_with_its_keys() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let frontmatter = concat!(
        "$schema:\n",
        "  item: \"{ children: item[], value: string }\"\n",
        "  choice: [item, number]\n",
        "  picks: choice[]\n",
        "picks:\n",
        "  - value: a\n",
        "    children: []\n",
    );
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "union.md", &markdown(frontmatter));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let markup = hover_at(&mut fixture, &document, offset(&document.text, "picks:\n", 0, 0));
    assert!(markup.contains("- **item** with `children`, `value`"), "{markup}");
    let markup = hover_at(&mut fixture, &document, offset(&document.text, "value: a", 0, 0));
    assert!(markup.contains("matches alternative 1 of 2:\n- **object** with `children`, `value`"), "{markup}");

    fixture.shutdown();
}

/// The URI the server reports for a file it resolved itself: the resolver
/// identifies schema files by canonical path.
fn canonical_uri(path: &Path) -> String {
    let canonical = biscuit_file::canonicalize_simplified(path).unwrap();
    url::Url::from_file_path(canonical).unwrap().to_string()
}

fn write(workspace: &LspWorkspace, name: &str, text: &str) {
    std::fs::write(workspace.path().join(name), text).unwrap();
}

/// `node` and `link` recurse through each other across two files.
const NODES: &str = "kind: schema\ntypes:\n  node: \"{ label: string(required), next: link[]@./links.yaml }\"\n";
const LINKS: &str = "kind: schema\ntypes:\n  link: \"{ weight: number, target: node@./nodes.yaml }\"\n";

/// Go-to-definition on either reference of a two-file cycle reaches the
/// other file's defining key, and the editor descends the cycle in a
/// document that uses it.
#[test]
fn definition_and_descent_follow_a_two_file_cycle() {
    let workspace = LspWorkspace::new();
    write(&workspace, "nodes.yaml", NODES);
    write(&workspace, "links.yaml", LINKS);
    let mut fixture = session(&workspace);
    let nodes = open_text(&mut fixture, &workspace, "nodes.yaml", NODES).0;
    let links = open_text(&mut fixture, &workspace, "links.yaml", LINKS).0;
    let nodes_uri = canonical_uri(&workspace.path().join("nodes.yaml"));
    let links_uri = canonical_uri(&workspace.path().join("links.yaml"));

    let locations = request(&mut fixture, "textDocument/definition", &nodes, offset(NODES, "link[]@", 0, 1));
    assert_eq!(locations, json!([{ "uri": links_uri, "range": range_of(LINKS, "link", 0) }]));
    // The cycle-closing reference, back into the first file.
    let locations = request(&mut fixture, "textDocument/definition", &links, offset(LINKS, "node@", 0, 1));
    assert_eq!(locations, json!([{ "uri": nodes_uri, "range": range_of(NODES, "node", 0) }]));

    let frontmatter = concat!(
        "$schema:\n",
        "  root: \"node(required)@./nodes.yaml\"\n",
        "root:\n",
        "  label: a\n",
        "  next:\n",
        "    - target:\n",
        "        label: b\n",
        "        next:\n",
        "          - target:\n",
        "              label: c\n",
        "              next: []\n",
    );
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "doc.md", &markdown(frontmatter));
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    // `label: c` is node level 2, reached through `link` twice.
    let at = offset(&document.text, "label: c\n", 0, "label: c\n".len() + 14);
    let items = completion_at(&mut fixture, &document, at);
    let mut keys = labels(&items);
    keys.sort_unstable();
    assert_eq!(keys, ["label", "next"], "{items:?}");
    let at = offset(&document.text, "- target:\n              ", 0, "- ".len());
    let markup = hover_at(&mut fixture, &document, at);
    assert!(markup.contains("`node` is defined as: `{label: string(required),next: link[]}`"), "{markup}");

    fixture.shutdown();
}

/// Acceptance 2 in the editor: a failure seven items deep is published at
/// the offending value's line and column, with its JSON path.
#[test]
fn a_deep_validation_failure_is_published_at_its_node() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let mut frontmatter = String::from("tree:\n  value: root\n  children:\n");
    let depth = 7;
    for level in 1..=depth {
        let indent = "    ".repeat(level - 1);
        let value = if level == depth { "[1]" } else { "ok" };
        frontmatter.push_str(&format!("{indent}    - value: {value}\n{indent}      children:"));
        frontmatter.push_str(if level == depth { " []\n" } else { "\n" });
    }
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "deep.md", &markdown(&format!("{SCHEMA}{frontmatter}")));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic["code"], json!("dm.schema.type_mismatch"), "{diagnostic:#?}");
    let pointer = format!("/tree{}/value", "/children/0".repeat(depth));
    assert_eq!(diagnostic["data"]["instance_path"], json!(pointer), "{diagnostic:#?}");
    assert_eq!(diagnostic["range"], range_of(&document.text, "[1]", 0), "{diagnostic:#?}");

    fixture.shutdown();
}

fn type_definition_errors(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_type_definition"))
        .collect()
}

fn related_locations(diagnostic: &Value) -> Vec<Value> {
    diagnostic["relatedInformation"]
        .as_array()
        .map(|related| related.iter().map(|related| related["location"].clone()).collect())
        .unwrap_or_default()
}

/// Acceptance 11: an alias cycle in a document's inline `$schema` is a schema
/// diagnostic on one of its references, with the other reference related.
#[test]
fn an_inline_alias_cycle_is_reported_on_its_references() {
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace);
    let (document, diagnostics) =
        open_text(&mut fixture, &workspace, "alias.md", &markdown("$schema:\n  a: b\n  b: a\n  title: string\ntitle: x\n"));

    let errors = type_definition_errors(&diagnostics);
    assert_eq!(errors.len(), 1, "{diagnostics:#?}");
    let message = errors[0]["message"].as_str().unwrap();
    assert!(message.contains("never descends into a child value"), "{message}");
    let first = name_range(&document.text, "b\n  b:");
    let second = name_range(&document.text, "a\n  title");
    let primary = errors[0]["range"].clone();
    assert!(primary == first || primary == second, "ranged on a reference: {:#?}", errors[0]);
    let other = if primary == first { second } else { first };
    assert_eq!(related_locations(errors[0]), vec![json!({ "uri": document.uri, "range": other })], "{:#?}", errors[0]);

    fixture.shutdown();
}

/// Acceptance 11: a non-descending cycle across two files is reported with
/// related spans in both files, and fixing either file clears it without
/// reopening the document.
#[test]
fn a_two_file_alias_cycle_relates_both_files_and_clears_when_fixed() {
    let workspace = LspWorkspace::new();
    let first = "kind: schema\ntypes:\n  a: 'b@./second.yaml'\n";
    let second = "kind: schema\ntypes:\n  b: 'a@./first.yaml'\n";
    std::fs::create_dir(workspace.path().join("lib")).unwrap();
    write(&workspace, "lib/first.yaml", first);
    write(&workspace, "lib/second.yaml", second);
    let mut fixture = session(&workspace);
    let (document, diagnostics) =
        open_text(&mut fixture, &workspace, "doc.md", &markdown("$schema:\n  title: 'a@./lib/first.yaml'\ntitle: x\n"));

    let errors = type_definition_errors(&diagnostics);
    assert_eq!(errors.len(), 1, "{diagnostics:#?}");
    assert_eq!(errors[0]["range"], name_range(&document.text, "a@"), "the document's reference into the cycle");
    let mut related = related_locations(errors[0]);
    related.sort_by_key(|location| location["uri"].as_str().unwrap().to_string());
    assert_eq!(
        related,
        vec![
            json!({ "uri": canonical_uri(&workspace.path().join("lib/first.yaml")), "range": name_range(first, "b@") }),
            json!({ "uri": canonical_uri(&workspace.path().join("lib/second.yaml")), "range": name_range(second, "a@") }),
        ],
        "{:#?}",
        errors[0]
    );

    // Fixing the file reached only through the cycle clears the document's
    // diagnostic at its next edit, even with no watched-file event (an edit
    // made outside the editor): the failed assembly records every file of the
    // cycle as a dependency, not only the file the document names.
    let mut version = 1;
    let mut edit_elsewhere = |fixture: &mut LspFixture<'_>, text: &str| {
        write(&workspace, "lib/second.yaml", text);
        version += 1;
        fixture.notify(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": document.uri, "version": version },
                "contentChanges": [{ "text": document.text }]
            }),
        );
        fixture.wait_for_diagnostics(&document.uri)
    };
    let diagnostics = edit_elsewhere(&mut fixture, "kind: schema\ntypes:\n  b: string\n");
    assert!(type_definition_errors(&diagnostics).is_empty(), "{diagnostics:#?}");
    let diagnostics = edit_elsewhere(&mut fixture, second);
    assert_eq!(type_definition_errors(&diagnostics).len(), 1, "{diagnostics:#?}");

    // Opened on its own, the file reports the cycle on its own reference.
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "lib/second.yaml", second);
    let errors = type_definition_errors(&diagnostics);
    assert_eq!(errors.len(), 1, "{diagnostics:#?}");
    assert_eq!(errors[0]["range"], name_range(second, "a@"));

    fixture.shutdown();
}

/// A schema-trigger matching any document with a `prompt`, whose inline
/// payload declares its own recursive `node` as `property`.
fn node_trigger(property: &str, node: &str) -> String {
    format!(
        "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  node: \"{node}\"\n  {property}: node\n"
    )
}

/// Two disk triggers whose inline payloads each declare a recursive `node`:
/// completion, hover, and validation under `right` use the right-hand
/// trigger's body, never the left-hand one that shares its name.
#[test]
fn same_named_inline_trigger_definitions_keep_their_own_bodies() {
    let workspace = LspWorkspace::new();
    std::fs::create_dir(workspace.path().join("schemas")).unwrap();
    write(
        &workspace,
        "schemas/left.trigger.yaml",
        &node_trigger("left", "{ left_value: string, children: node[] }"),
    );
    write(
        &workspace,
        "schemas/right.trigger.yaml",
        &node_trigger("right", "{ children: node[], right_value: number -> The right-hand number }"),
    );
    let mut fixture = session(&workspace);

    // `right:` with no keys yet, the cursor on the indented line below it.
    let (document, _) = open_text(
        &mut fixture,
        &workspace,
        "keyless.md",
        &markdown("prompt: x\nleft: {}\nright:\n  \nother: 1\n"),
    );
    let at = offset(&document.text, "right:\n  ", 0, "right:\n  ".len());
    let items = completion_at(&mut fixture, &document, at);
    let labels: Vec<&str> = item_keys(&items).into_iter().map(|(label, _)| label).collect();
    assert_eq!(labels, ["children", "right_value"], "{items:?}");

    let frontmatter = concat!(
        "prompt: x\n",
        "left:\n",
        "  left_value: a\n",
        "right:\n",
        "  right_value: 2\n",
        "  children:\n",
        "    - right_value: 3\n",
        "      children: []\n",
    );
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "valid.md", &markdown(frontmatter));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    for occurrence in 0..2 {
        let markup = hover_at(&mut fixture, &document, offset(&document.text, "right_value", occurrence, 0));
        assert!(markup.contains("Type: **number**"), "#{occurrence}: {markup}");
        assert!(markup.contains("The right-hand number"), "#{occurrence}: {markup}");
    }

    let invalid = frontmatter.replace("    - right_value: 3\n", "    - right_value: \"x\"\n");
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "invalid.md", &markdown(&invalid));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["code"], json!("dm.schema.type_mismatch"), "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["data"]["instance_path"], json!("/right/children/0/right_value"), "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["range"], range_of(&document.text, "\"x\"", 0), "{diagnostics:#?}");

    fixture.shutdown();
}

/// `entry` is an acyclic object declared in an imported file, and `item` an
/// alias of it there. The document names it at three sites (`first`,
/// `second`, and the list item of `items` through the alias), so the resolver
/// stores it once and references it at each use; see
/// `the_shared_fixture_stores_entry_once_and_references_it_at_every_use`.
/// `spare` is shared too, by two properties no document writes, so a walk
/// that opens it has left the cursor path.
const SHARED_TYPES: &str = concat!(
    "kind: schema\n",
    "types:\n",
    "  entry: \"{ name: string(required) -> The entry name, size: number(min(0)) -> The entry size }\"\n",
    "  item: entry\n",
    "  spare: \"{ unused: string }\"\n",
);

const SHARED_SCHEMA: &str = concat!(
    "$schema:\n",
    "  first: \"entry@./types.yaml\"\n",
    "  second: \"entry@./types.yaml\"\n",
    "  items: \"item[]@./types.yaml\"\n",
    "  spare_a: \"spare@./types.yaml\"\n",
    "  spare_b: \"spare@./types.yaml\"\n",
);

const SHARED_VALUES: &str = concat!(
    "first:\n",
    "  name: a\n",
    "  size: 1\n",
    "second:\n",
    "  name: b\n",
    "  size: 2\n",
    "items:\n",
    "  - name: c\n",
    "    size: 3\n",
    "  - name: d\n",
    "    size: 4\n",
);

/// A session whose client watches files, over a workspace holding
/// `types.yaml`.
fn shared_session(workspace: &LspWorkspace) -> LspFixture<'_> {
    write(workspace, "types.yaml", SHARED_TYPES);
    let mut params = initialize_params(workspace.path());
    params["capabilities"]["workspace"] = json!({ "didChangeWatchedFiles": { "dynamicRegistration": true } });
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(params);
    fixture
}

/// Every `$ref` below `value`.
fn refs_below(value: &Value) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(pointer)) = map.get("$ref") {
                    found.push(pointer.clone());
                }
                pending.extend(map.values());
            }
            Value::Array(items) => pending.extend(items),
            _ => {}
        }
    }
    found
}

/// Guards the fixture: the library stores `entry` as one `$defs` entry and
/// every use references it, so the editor tests below stand on a `$ref` at
/// each use rather than on three inlined copies.
#[test]
fn the_shared_fixture_stores_entry_once_and_references_it_at_every_use() {
    let workspace = LspWorkspace::new();
    write(&workspace, "types.yaml", SHARED_TYPES);
    let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(SHARED_SCHEMA).unwrap();
    let context = biscuit_file::FileResolutionContext::new(workspace.path());

    let resolved = darkmatter::markdown::schemas::resolve::resolve_yaml_schema(&document["$schema"], workspace.path(), &context)
        .unwrap();

    let schema = &resolved.json_schema;
    let pointer = refs_below(&schema["properties"]["first"]);
    assert_eq!(pointer.len(), 1, "{schema:#}");
    for property in ["second", "items"] {
        assert_eq!(refs_below(&schema["properties"][property]), pointer, "{property}: {schema:#}");
    }
    let key = pointer[0].strip_prefix("#/$defs/").unwrap();
    let mut keys: Vec<&String> = schema["$defs"][key]["properties"].as_object().unwrap().keys().collect();
    keys.sort_unstable();
    assert_eq!(keys, ["name", "size"], "{schema:#}");
}

/// Spec acceptance 3 and 7 in the editor: a failure reached through each use
/// of the shared type, a list item included, is published at its own value
/// with its full instance path.
#[test]
fn a_failure_through_each_shared_use_is_published_at_its_value() {
    let workspace = LspWorkspace::new();
    let mut fixture = shared_session(&workspace);
    let invalid = SHARED_VALUES
        .replace("  size: 1\n", "  size: -1\n")
        .replace("  size: 2\n", "  size: [2]\n")
        .replace("  - name: d\n", "  - name: [1]\n");
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "doc.md", &markdown(&format!("{SHARED_SCHEMA}{invalid}")));

    let mut published: Vec<(Value, Value, Value, Value)> = diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic["data"]["instance_path"].clone(),
                diagnostic["code"].clone(),
                diagnostic["message"].clone(),
                diagnostic["range"].clone(),
            )
        })
        .collect();
    published.sort_by_key(|(path, ..)| path.to_string());
    assert_eq!(
        published,
        [
            (
                json!("/first/size"),
                json!("dm.schema.constraint"),
                json!("-1 is less than the minimum of 0"),
                range_of(&document.text, "-1", 0),
            ),
            (
                json!("/items/1/name"),
                json!("dm.schema.type_mismatch"),
                json!("[1] is not of type \"string\""),
                range_of(&document.text, "[1]", 0),
            ),
            (
                json!("/second/size"),
                json!("dm.schema.type_mismatch"),
                json!("[2] is not of type \"number\""),
                range_of(&document.text, "[2]", 0),
            ),
        ],
        "{diagnostics:#?}"
    );

    fixture.shutdown();
}

/// Key completion inside an incomplete object at either use and in an empty
/// list item offers the shared type's keys, opening only `entry`: never
/// `spare`, which the schema stores too but the cursor path does not reach.
#[test]
fn completion_offers_the_shared_keys_at_each_use_and_in_an_empty_item() {
    let workspace = LspWorkspace::new();
    let mut fixture = shared_session(&workspace);
    let incomplete = "first:\n  \nsecond:\n  name: b\n  \nitems:\n  - \nother: 1\n";
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", &markdown(&format!("{SHARED_SCHEMA}{incomplete}")));
    fixture.flush_server();

    let both = [("name", Some("(required)")), ("size", None)];
    for (case, needle, expected) in [
        ("key-less first", "first:\n  ", &both[..]),
        ("second with name written", "name: b\n  ", &both[1..]),
        ("empty item", "items:\n  - ", &both[..]),
    ] {
        dmls::work_counters::record_definition_lookups();
        let items = completion_at(&mut fixture, &document, offset(&document.text, needle, 0, needle.len()));
        let opened = dmls::work_counters::definition_lookups();
        assert_eq!(item_keys(&items), expected, "{case}: {items:?}");
        assert!(opened.contains(&"entry".to_string()), "{case} opens the reference: {opened:?}");
        assert!(opened.iter().all(|name| name == "entry"), "{case} opens only `entry`: {opened:?}");
    }

    fixture.shutdown();
}

/// Hover on a field reads the same description at both uses and in a list
/// item; hover on a use's key names the shared type and shows its body once.
#[test]
fn hover_describes_the_shared_type_at_each_use() {
    let workspace = LspWorkspace::new();
    let mut fixture = shared_session(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", &markdown(&format!("{SHARED_SCHEMA}{SHARED_VALUES}")));

    let name = "**`name`**\n\nType: **string**\n\nRequired\n\n\n\nThe entry name";
    let size = "**`size`**\n\nType: **number**\n\n\n\nThe entry size";
    for (field, expected) in [("name: ", name), ("size: ", size)] {
        for occurrence in 0..3 {
            let markup = hover_at(&mut fixture, &document, offset(&document.text, field, occurrence, 0));
            assert!(markup.starts_with(expected), "{field}#{occurrence}: {markup}");
        }
    }

    let body = "`entry` is defined as: `{name: string(required) -> The entry name,size: number(min(0)) -> The entry size}`";
    for (key, spelled) in [("\nfirst:", "entry"), ("\nsecond:", "entry"), ("\nitems:", "entry[]")] {
        let markup = hover_at(&mut fixture, &document, offset(&document.text, key, 0, 1));
        assert_eq!(markup, format!("**`{}`**\n\nType: **{spelled}**\n\n{body}", &key[1..key.len() - 1]));
    }

    fixture.shutdown();
}

/// Go-to-definition from each reference to the shared type, the alias
/// included, lands on its declaration in the imported file, and the alias's
/// own reference lands on `entry`.
#[test]
fn definition_from_each_shared_reference_reaches_the_imported_declaration() {
    let workspace = LspWorkspace::new();
    let mut fixture = shared_session(&workspace);
    let types_uri = canonical_uri(&workspace.path().join("types.yaml"));
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", &markdown(&format!("{SHARED_SCHEMA}{SHARED_VALUES}")));

    let entry = json!([{ "uri": types_uri, "range": range_of(SHARED_TYPES, "entry", 0) }]);
    let item = json!([{ "uri": types_uri, "range": range_of(SHARED_TYPES, "item", 0) }]);
    for (needle, occurrence, expected) in [("entry@", 0, &entry), ("entry@", 1, &entry), ("item[]@", 0, &item)] {
        let at = offset(&document.text, needle, occurrence, 1);
        let locations = request(&mut fixture, "textDocument/definition", &document, at);
        assert_eq!(&locations, expected, "{needle}#{occurrence}");
    }

    let types = open_text(&mut fixture, &workspace, "types.yaml", SHARED_TYPES).0;
    let locations = request(&mut fixture, "textDocument/definition", &types, offset(SHARED_TYPES, "item: entry", 0, 7));
    assert_eq!(locations, json!([{ "uri": types.uri, "range": range_of(SHARED_TYPES, "entry", 0) }]));

    fixture.shutdown();
}

/// Editing the imported file that declares the shared type refreshes the
/// document's diagnostics at every use without reopening it, and restoring
/// the file clears them.
#[test]
fn editing_the_imported_shared_type_refreshes_every_use() {
    let workspace = LspWorkspace::new();
    let mut fixture = shared_session(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "doc.md", &markdown(&format!("{SHARED_SCHEMA}{SHARED_VALUES}")));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let types_uri = url::Url::from_file_path(workspace.path().join("types.yaml")).unwrap().to_string();
    let edit_types = |fixture: &mut LspFixture<'_>, text: &str| {
        write(&workspace, "types.yaml", text);
        fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": [{ "uri": types_uri, "type": 2 }] }));
        fixture.wait_for_diagnostics(&document.uri)
    };

    let diagnostics = edit_types(&mut fixture, &SHARED_TYPES.replace("number(min(0))", "number(min(5))"));
    let mut paths: Vec<&str> = diagnostics
        .iter()
        .map(|diagnostic| {
            assert_eq!(diagnostic["code"], json!("dm.schema.constraint"), "{diagnostic:#?}");
            diagnostic["data"]["instance_path"].as_str().unwrap()
        })
        .collect();
    paths.sort_unstable();
    assert_eq!(paths, ["/first/size", "/items/0/size", "/items/1/size", "/second/size"], "{diagnostics:#?}");

    let diagnostics = edit_types(&mut fixture, SHARED_TYPES);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    fixture.shutdown();
}

/// A union with an object arm, declared in `types.yaml` beside the
/// sharing fixture.
const SHAPE_TYPES: &str = concat!(
    "kind: schema\n",
    "types:\n",
    "  shape:\n",
    "    - \"{ side: number -> The side length }\"\n",
    "    - \"enum(none) -> No shape\"\n",
);

/// A stored union hovers exactly as the same union inlined at a single use
/// does: its alternatives are listed rather than its body spelled out.
#[test]
fn a_shared_union_hovers_as_its_inlined_alternatives() {
    let workspace = LspWorkspace::new();
    write(&workspace, "types.yaml", SHAPE_TYPES);
    let shared = "$schema:\n  left: \"shape(required)@./types.yaml\"\n  right: \"shape(required)@./types.yaml\"\n";
    let single = "$schema:\n  left: \"shape(required)@./types.yaml\"\n";
    let values = "left:\n  side: 1\nright: none\n";
    let context = biscuit_file::FileResolutionContext::new(workspace.path());
    for (schema, stored) in [(shared, true), (single, false)] {
        let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(schema).unwrap();
        let resolved =
            darkmatter::markdown::schemas::resolve::resolve_yaml_schema(&document["$schema"], workspace.path(), &context)
                .unwrap();
        assert_eq!(!refs_below(&resolved.json_schema["properties"]["left"]).is_empty(), stored, "{schema}");
    }
    let mut fixture = session(&workspace);

    let (shared, _) = open_text(&mut fixture, &workspace, "shared.md", &markdown(&format!("{shared}{values}")));
    let (single, _) = open_text(&mut fixture, &workspace, "single.md", &markdown(&format!("{single}{values}")));
    let inlined = hover_at(&mut fixture, &single, offset(&single.text, "\nleft:", 0, 1));
    assert_eq!(
        inlined,
        "**`left`**\n\nType: one of:\n- **object** with `side`\n- **enum** — No shape\n\nRequired"
    );
    for key in ["\nleft:", "\nright:"] {
        let markup = hover_at(&mut fixture, &shared, offset(&shared.text, key, 0, 1));
        assert_eq!(markup, inlined.replace("left", &key[1..key.len() - 1]), "{key}");
    }

    fixture.shutdown();
}
