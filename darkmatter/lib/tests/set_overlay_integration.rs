//! End-to-end integration tests for the `set` overlay feature in transclusion.
//!
//! Validates that `::file ... set.NAME=<value>` and `::file ... set=<dict>`
//! correctly apply a three-layer overlay on child frontmatter BEFORE any
//! child pipeline stages observe it.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};
use tempfile::TempDir;

fn write_files(dir: &TempDir, files: &[(&str, &str)]) {
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
    }
}

fn compose_file(dir: &TempDir, root_name: &str, extra_options: Option<ComposeOptions>) -> String {
    let root = dir.path().join(root_name);
    let base = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path());
    let options = extra_options.unwrap_or(base);
    let (composed, _) = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .unwrap();
    composed.content().to_string()
}

fn compose_file_result(
    dir: &TempDir,
    root_name: &str,
    extra_options: Option<ComposeOptions>,
) -> Result<String, darkmatter::markdown::types::MarkdownError> {
    let root = dir.path().join(root_name);
    let base = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path());
    let options = extra_options.unwrap_or(base);
    Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .map(|(md, _)| md.content().to_string())
}

// ── Overlay-first page-block evaluation ────────────────────────────

#[test]
fn child_page_block_sees_set_property_override() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.role="admin""#,
            ),
            (
                "child.md",
                r#"---
role: guest
---

::block when="role == 'admin'"

Admin content

::end-block

::block when="role == 'guest'"

Guest content

::end-block
"#,
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Admin content"),
        "child page block should see overridden role='admin'"
    );
    assert!(
        !output.contains("Guest content"),
        "child page block for guest should be removed since role is overridden to admin"
    );
}

// ── Child interpolation through set.* ──────────────────────────────

#[test]
fn child_interpolation_sees_set_property() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.name="Bob""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\nHello {{ name }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Hello Bob"),
        "child interpolation should see set.name='Bob', got: {output:?}"
    );
    assert!(
        !output.contains("Alice"),
        "child's original name='Alice' should be overridden"
    );
}

#[test]
fn child_interpolation_sees_set_object() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set='{name: "Bob", age: 42}'"#,
            ),
            (
                "child.md",
                "---\nname: Alice\nage: 25\n---\n\n{{ name }} is {{ age }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Bob is 42"),
        "child interpolation should see set object values, got: {output:?}"
    );
}

// ── replace: observing overrides ───────────────────────────────────

#[test]
fn child_replace_sees_set_overrides() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.greeting="Hello""#,
            ),
            (
                "child.md",
                r#"---
greeting: Hi
replace:
  GREETING: '{{ greeting }}'
---

GREETING world
"#,
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Hello world"),
        "child replace should observe overridden greeting, got: {output:?}"
    );
    assert!(
        !output.contains("Hi world"),
        "original greeting should not appear"
    );
}

// ── Grandchild isolation ───────────────────────────────────────────

#[test]
fn set_overlay_does_not_propagate_to_grandchild() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.name="Bob""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\nChild sees: {{ name }}\n\n::file grandchild.md\n",
            ),
            (
                "grandchild.md",
                "---\nname: Charlie\n---\n\nGrandchild sees: {{ name }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Child sees: Bob"),
        "child should see overridden name='Bob', got: {output:?}"
    );
    assert!(
        output.contains("Grandchild sees: Charlie"),
        "grandchild should see its own name='Charlie', not parent's overlay, got: {output:?}"
    );
    assert!(
        !output.contains("Grandchild sees: Bob"),
        "parent overlay must not propagate to grandchild"
    );
}

// ── Worked example from spec ───────────────────────────────────────

#[test]
fn spec_worked_example_alice_bob() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file foo.md set='{author: {handle: "@bob"}, tags: ["blue"]}' set.name="Bob""#,
            ),
            (
                "foo.md",
                r#"---
name: Alice
author:
  name: Alice
  handle: "@alice"
tags:
  - red
  - green
---

Name: {{ name }}
Handle: {{ author.handle }}
"#,
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Name: Bob"),
        "set.name should override name to Bob, got: {output:?}"
    );
    assert!(
        output.contains("Handle: @bob"),
        "set object should deep-merge author.handle to @bob, got: {output:?}"
    );
    assert!(
        !output.contains("Alice"),
        "original Alice values should be overridden"
    );
    assert!(
        !output.contains("@alice"),
        "original @alice should be overridden by @bob"
    );
}

// ── Three-layer precedence ─────────────────────────────────────────

#[test]
fn three_layer_precedence_property_wins_over_object() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set='{name: "Carol"}' set.name="Bob""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\n{{ name }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Bob"),
        "property-form set.name should win over object-form, got: {output:?}"
    );
    assert!(
        !output.contains("Carol"),
        "Carol from object-form should be overridden by Bob from property-form"
    );
    assert!(
        !output.contains("Alice"),
        "Alice from child frontmatter should be overridden"
    );
}

// ── Null semantics ─────────────────────────────────────────────────

#[test]
fn null_rhs_is_literal_not_deletion() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.x=null"#,
            ),
            (
                "child.md",
                "---\nx: 5\n---\n\nValue: {{ x }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        !output.contains("Value: 5"),
        "original x=5 should be overridden by null"
    );
}

#[test]
fn null_in_quoted_object_deep_merge_is_literal() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.author='{name: null}'"#,
            ),
            (
                "child.md",
                r#"---
author:
  name: Alice
  handle: "@alice"
---

Handle: {{ author.handle }}
"#,
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Handle: @alice"),
        "deep-merge should preserve author.handle, got: {output:?}"
    );
}

// ── Strict mode (default) errors ───────────────────────────────────

#[test]
fn strict_mode_rejects_invalid_assignment() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set=42"#,
            ),
            ("child.md", "body\n"),
        ],
    );

    let result = compose_file_result(&dir, "parent.md", None);
    assert!(result.is_err(), "strict mode should reject set=42");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Invalid frontmatter assignment"),
        "expected InvalidFrontmatterAssignment, got: {err}"
    );
}

#[test]
fn strict_mode_rejects_reassigned_property() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.name="Bob" set.name="Mary""#,
            ),
            ("child.md", "---\n---\nbody\n"),
        ],
    );

    let result = compose_file_result(&dir, "parent.md", None);
    assert!(
        result.is_err(),
        "strict mode should reject duplicate set.name"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Invalid reassigned frontmatter property"),
        "expected InvalidReassignedFrontmatterProperty, got: {err}"
    );
}

// ── Permissive mode ────────────────────────────────────────────────

#[test]
fn permissive_invalid_assignment_warns_and_keeps_siblings() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set=42 set.name="Bob""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\n{{ name }}\n",
            ),
        ],
    );

    let root = dir.path().join("parent.md");
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path())
        .with_allow_invalid_frontmatter_assignment(true);
    let (composed, report) = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .unwrap();

    let output = composed.content().to_string();
    assert!(
        output.contains("Bob"),
        "sibling set.name should still apply under permissive mode, got: {output:?}"
    );
    assert!(
        !output.contains("Alice"),
        "original name should be overridden"
    );
    assert!(
        !report.warnings.is_empty(),
        "permissive mode should emit a warning for set=42"
    );
}

#[test]
fn permissive_reassigned_property_warns_and_rightmost_wins() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.name="Bob" set.name="Mary""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\n{{ name }}\n",
            ),
        ],
    );

    let root = dir.path().join("parent.md");
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path())
        .with_allow_reassigned_frontmatter_property(true);
    let (composed, report) = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .unwrap();

    let output = composed.content().to_string();
    assert!(
        output.contains("Mary"),
        "rightmost assignment should win under permissive mode, got: {output:?}"
    );
    assert!(
        !output.contains("Bob"),
        "Bob should be overridden by Mary"
    );
    assert!(
        !output.contains("Alice"),
        "original Alice should be overridden"
    );
    assert!(
        !report.warnings.is_empty(),
        "permissive mode should emit a warning for duplicate set.name"
    );
}

// ── Cache-key partitioning by overlay ──────────────────────────────

#[test]
fn different_set_overlays_produce_different_outputs() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                "::file child.md set.name=\"Alice\"\n\n::file child.md set.name=\"Bob\"\n",
            ),
            (
                "child.md",
                "---\nname: default\n---\n\n{{ name }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Alice"),
        "first transclusion with set.name=Alice should appear, got: {output:?}"
    );
    assert!(
        output.contains("Bob"),
        "second transclusion with set.name=Bob should appear, got: {output:?}"
    );
    assert!(
        !output.contains("default"),
        "neither transclusion should show default name"
    );
}

// ── Arrays as leaves ───────────────────────────────────────────────

#[test]
fn array_valued_set_replaces_entire_array() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.tags='["blue"]'"#,
            ),
            (
                "child.md",
                "---\ntags:\n  - red\n  - green\n---\n\nTags: {{ tags }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        !output.contains("red"),
        "original tags should be entirely replaced"
    );
    assert!(
        output.contains("blue"),
        "new tag from set should appear, got: {output:?}"
    );
}

// ── Deep merge preserves sibling keys ──────────────────────────────

#[test]
fn deep_merge_preserves_unoverlapped_keys() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.a='{y: 2}'"#,
            ),
            (
                "child.md",
                "---\na:\n  x: 1\n---\n\na.x={{ a.x }} a.y={{ a.y }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("a.x=1"),
        "child's a.x should be preserved by deep merge, got: {output:?}"
    );
    assert!(
        output.contains("a.y=2"),
        "set overlay's a.y should be added by deep merge, got: {output:?}"
    );
}

// ── set with quoted JSON5 object in property form ──────────────────

#[test]
fn quoted_property_object_form_deep_merge() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.author='{name: null}'"#,
            ),
            (
                "child.md",
                r#"---
author:
  name: Alice
  handle: "@alice"
---

Name: {{ author.name }}
Handle: {{ author.handle }}
"#,
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Handle: @alice"),
        "deep merge should preserve author.handle, got: {output:?}"
    );
}

// ── Multiple properties on single directive ────────────────────────

#[test]
fn multiple_set_properties_all_apply() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.x=1 set.y=2 set.z=3"#,
            ),
            (
                "child.md",
                "---\nx: 0\ny: 0\n---\n\n{{ x }} {{ y }} {{ z }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("1 2 3"),
        "all set properties should apply, got: {output:?}"
    );
}
        std::fs::write(&path, content).unwrap();
    }
}

fn compose_file(dir: &TempDir, root_name: &str, extra_options: Option<ComposeOptions>) -> String {
    let root = dir.path().join(root_name);
    let base = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path());
    let options = extra_options.unwrap_or(base);
    let (composed, _) = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .unwrap();
    composed.content().to_string()
}

fn compose_file_result(
    dir: &TempDir,
    root_name: &str,
    extra_options: Option<ComposeOptions>,
) -> Result<String, darkmatter::markdown::types::MarkdownError> {
    let root = dir.path().join(root_name);
    let base = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path());
    let options = extra_options.unwrap_or(base);
    Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .map(|(md, _)| md.content().to_string())
}

// ── Overlay-first page-block evaluation ────────────────────────────

#[test]
fn child_page_block_sees_set_property_override() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.role="admin""#,
            ),
            (
                "child.md",
                "---\nrole: guest\n---\n\n::block when="role == 'admin'"\n\nAdmin content\n\n::end-block\n\n::block when="role == 'guest'"\n\nGuest content\n\n::end-block\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Admin content"),
        "child page block should see overridden role='admin'"
    );
    assert!(
        !output.contains("Guest content"),
        "child page block for guest should be removed since role is overridden to admin"
    );
}

// ── Child interpolation through set.* ──────────────────────────────

#[test]
fn child_interpolation_sees_set_property() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.name="Bob""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\nHello {{ name }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Hello Bob"),
        "child interpolation should see set.name='Bob', got: {output:?}"
    );
    assert!(
        !output.contains("Alice"),
        "child's original name='Alice' should be overridden"
    );
}

#[test]
fn child_interpolation_sees_set_object() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set='{name: "Bob", age: 42}'"#,
            ),
            (
                "child.md",
                "---\nname: Alice\nage: 25\n---\n\n{{ name }} is {{ age }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Bob is 42"),
        "child interpolation should see set object values, got: {output:?}"
    );
}

// ── replace: observing overrides ───────────────────────────────────

#[test]
fn child_replace_sees_set_overrides() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.greeting="Hello""#,
            ),
            (
                "child.md",
                "---\ngreeting: Hi\nreplace:\n  GREETING: '{{ greeting }}'\n---\n\nGREETING world\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Hello world"),
        "child replace should observe overridden greeting, got: {output:?}"
    );
    assert!(
        !output.contains("Hi world"),
        "original greeting should not appear"
    );
}

// ── Grandchild isolation ───────────────────────────────────────────

#[test]
fn set_overlay_does_not_propagate_to_grandchild() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.name="Bob""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\nChild sees: {{ name }}\n\n::file grandchild.md\n",
            ),
            (
                "grandchild.md",
                "---\nname: Charlie\n---\n\nGrandchild sees: {{ name }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Child sees: Bob"),
        "child should see overridden name='Bob', got: {output:?}"
    );
    assert!(
        output.contains("Grandchild sees: Charlie"),
        "grandchild should see its own name='Charlie', not parent's overlay, got: {output:?}"
    );
    assert!(
        !output.contains("Grandchild sees: Bob"),
        "parent overlay must not propagate to grandchild"
    );
}

// ── Worked example from spec ───────────────────────────────────────

#[test]
fn spec_worked_example_alice_bob() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file foo.md set='{author: {handle: "@bob"}, tags: ["blue"]}' set.name="Bob""#,
            ),
            (
                "foo.md",
                "---\nname: Alice\nauthor:\n  name: Alice\n  handle: \"@alice\"\ntags:\n  - red\n  - green\n---\n\nName: {{ name }}\nHandle: {{ author.handle }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Name: Bob"),
        "set.name should override name to Bob, got: {output:?}"
    );
    assert!(
        output.contains("Handle: @bob"),
        "set object should deep-merge author.handle to @bob, got: {output:?}"
    );
    assert!(
        !output.contains("Alice"),
        "original Alice values should be overridden"
    );
    assert!(
        !output.contains("@alice"),
        "original @alice should be overridden by @bob"
    );
}

// ── Three-layer precedence ─────────────────────────────────────────

#[test]
fn three_layer_precedence_property_wins_over_object() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set='{name: "Carol"}' set.name="Bob""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\n{{ name }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Bob"),
        "property-form set.name should win over object-form, got: {output:?}"
    );
    assert!(
        !output.contains("Carol"),
        "Carol from object-form should be overridden by Bob from property-form"
    );
    assert!(
        !output.contains("Alice"),
        "Alice from child frontmatter should be overridden"
    );
}

// ── Null semantics ─────────────────────────────────────────────────

#[test]
fn null_rhs_is_literal_not_deletion() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.x=null"#,
            ),
            (
                "child.md",
                "---\nx: 5\n---\n\nValue: {{ x }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        !output.contains("Value: 5"),
        "original x=5 should be overridden by null"
    );
}

#[test]
fn null_in_quoted_object_deep_merge_is_literal() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.author='{name: null}'"#,
            ),
            (
                "child.md",
                "---\nauthor:\n  name: Alice\n  handle: \"@alice\"\n---\n\nHandle: {{ author.handle }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Handle: @alice"),
        "deep-merge should preserve author.handle, got: {output:?}"
    );
}

// ── Strict mode (default) errors ───────────────────────────────────

#[test]
fn strict_mode_rejects_invalid_assignment() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set=42"#,
            ),
            ("child.md", "body\n"),
        ],
    );

    let result = compose_file_result(&dir, "parent.md", None);
    assert!(result.is_err(), "strict mode should reject set=42");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Invalid frontmatter assignment"),
        "expected InvalidFrontmatterAssignment, got: {err}"
    );
}

#[test]
fn strict_mode_rejects_reassigned_property() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.name="Bob" set.name="Mary""#,
            ),
            ("child.md", "---\n---\nbody\n"),
        ],
    );

    let result = compose_file_result(&dir, "parent.md", None);
    assert!(
        result.is_err(),
        "strict mode should reject duplicate set.name"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Invalid reassigned frontmatter property"),
        "expected InvalidReassignedFrontmatterProperty, got: {err}"
    );
}

// ── Permissive mode ────────────────────────────────────────────────

#[test]
fn permissive_invalid_assignment_warns_and_keeps_siblings() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set=42 set.name="Bob""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\n{{ name }}\n",
            ),
        ],
    );

    let root = dir.path().join("parent.md");
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path())
        .with_allow_invalid_frontmatter_assignment(true);
    let (composed, report) = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .unwrap();

    let output = composed.content().to_string();
    assert!(
        output.contains("Bob"),
        "sibling set.name should still apply under permissive mode, got: {output:?}"
    );
    assert!(
        !output.contains("Alice"),
        "original name should be overridden"
    );
    assert!(
        !report.warnings().is_empty(),
        "permissive mode should emit a warning for set=42"
    );
}

#[test]
fn permissive_reassigned_property_warns_and_rightmost_wins() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.name="Bob" set.name="Mary""#,
            ),
            (
                "child.md",
                "---\nname: Alice\n---\n\n{{ name }}\n",
            ),
        ],
    );

    let root = dir.path().join("parent.md");
    let options = ComposeOptions::new()
        .with_source_file(&root)
        .with_shell_policy_root(dir.path())
        .with_allow_reassigned_frontmatter_property(true);
    let (composed, report) = Markdown::try_from(root.as_path())
        .unwrap()
        .compose_with(options)
        .unwrap();

    let output = composed.content().to_string();
    assert!(
        output.contains("Mary"),
        "rightmost assignment should win under permissive mode, got: {output:?}"
    );
    assert!(
        !output.contains("Bob"),
        "Bob should be overridden by Mary"
    );
    assert!(
        !output.contains("Alice"),
        "original Alice should be overridden"
    );
    assert!(
        !report.warnings().is_empty(),
        "permissive mode should emit a warning for duplicate set.name"
    );
}

// ── Cache-key partitioning by overlay ──────────────────────────────

#[test]
fn different_set_overlays_produce_different_outputs() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                concat!(
                    "::file child.md set.name=\"Alice\"\n",
                    "\n",
                    "::file child.md set.name=\"Bob\"\n",
                ),
            ),
            (
                "child.md",
                "---\nname: default\n---\n\n{{ name }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Alice"),
        "first transclusion with set.name=Alice should appear, got: {output:?}"
    );
    assert!(
        output.contains("Bob"),
        "second transclusion with set.name=Bob should appear, got: {output:?}"
    );
    assert!(
        !output.contains("default"),
        "neither transclusion should show default name"
    );
}

// ── Arrays as leaves ───────────────────────────────────────────────

#[test]
fn array_valued_set_replaces_entire_array() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.tags='["blue"]'"#,
            ),
            (
                "child.md",
                "---\ntags:\n  - red\n  - green\n---\n\nTags: {{ tags }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        !output.contains("red"),
        "original tags should be entirely replaced"
    );
    assert!(
        output.contains("blue"),
        "new tag from set should appear, got: {output:?}"
    );
}

// ── Deep merge preserves sibling keys ──────────────────────────────

#[test]
fn deep_merge_preserves_unoverlapped_keys() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.a='{y: 2}'"#,
            ),
            (
                "child.md",
                "---\na:\n  x: 1\n---\n\na.x={{ a.x }} a.y={{ a.y }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("a.x=1"),
        "child's a.x should be preserved by deep merge, got: {output:?}"
    );
    assert!(
        output.contains("a.y=2"),
        "set overlay's a.y should be added by deep merge, got: {output:?}"
    );
}

// ── set with quoted JSON5 object in property form ──────────────────

#[test]
fn quoted_property_object_form_deep_merge() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.author='{name: null}'"#,
            ),
            (
                "child.md",
                "---\nauthor:\n  name: Alice\n  handle: \"@alice\"\n---\n\nName: {{ author.name }}\nHandle: {{ author.handle }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("Handle: @alice"),
        "deep merge should preserve author.handle, got: {output:?}"
    );
}

// ── Multiple properties on single directive ────────────────────────

#[test]
fn multiple_set_properties_all_apply() {
    let dir = TempDir::new().unwrap();
    write_files(
        &dir,
        &[
            (
                "parent.md",
                r#"::file child.md set.x=1 set.y=2 set.z=3"#,
            ),
            (
                "child.md",
                "---\nx: 0\ny: 0\n---\n\n{{ x }} {{ y }} {{ z }}\n",
            ),
        ],
    );

    let output = compose_file(&dir, "parent.md", None);
    assert!(
        output.contains("1 2 3"),
        "all set properties should apply, got: {output:?}"
    );
}
