//! The inventory walk and member checks `build.rs` runs, over temporary
//! folders. The embedded table's agreement with the real folder is
//! `tests/l1/registration_inventory.rs`.

use std::path::Path;

use super::walk::{self, InventoryError, MemberKind};

const LIBRARY: &str = "kind: schema\ntypes:\n    thing: string\n";
const TRIGGER: &str = "kind: schema-trigger\nname: a-trigger\nmatch:\n    - prompt: string(required)\n";
const NAMESPACE: &str = "kind: namespace-schema\nnamespace: seq\n$schema:\n    name: string\n";
const KIND: &str = "kind: yaml-schema\n$schema:\n    kind: literal(thing; required)\n";

fn write(root: &Path, path: &str, bytes: &[u8]) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn refusals(root: &Path) -> Vec<InventoryError> {
    walk::walk(root).expect_err("the inventory is refused")
}

fn assert_refused_at(root: &Path, path: &str, needle: &str) {
    let errors = refusals(root);
    assert!(
        errors.iter().any(|error| error.path == path && error.message.contains(needle)),
        "expected a refusal at `{path}` containing {needle:?}, got {errors:#?}"
    );
}

#[test]
fn members_of_every_kind_sort_by_relative_path_including_nested_files() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "zeta.yaml", LIBRARY.as_bytes());
    write(dir.path(), "nested/deeper/b.yaml", NAMESPACE.as_bytes());
    write(dir.path(), "nested/a.yaml", TRIGGER.as_bytes());
    write(dir.path(), "alpha.yaml", KIND.as_bytes());

    let (members, directories) = walk::walk(dir.path()).expect("a valid inventory");

    let listed: Vec<(&str, MemberKind, Option<&str>)> =
        members.iter().map(|member| (member.path.as_str(), member.kind, member.name.as_deref())).collect();
    assert_eq!(
        listed,
        [
            ("alpha.yaml", MemberKind::YamlSchema, None),
            ("nested/a.yaml", MemberKind::Trigger, Some("a-trigger")),
            ("nested/deeper/b.yaml", MemberKind::Namespace, None),
            ("zeta.yaml", MemberKind::Library, None),
        ]
    );
    assert_eq!(members[1].text, TRIGGER);
    assert_eq!(directories.len(), 3, "the root and both nested folders are tracked: {directories:?}");
}

#[test]
fn a_file_that_is_not_yaml_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "types.yaml", LIBRARY.as_bytes());
    write(dir.path(), "README.md", b"# notes\n");
    write(dir.path(), "nested/types.yml", LIBRARY.as_bytes());
    assert_refused_at(dir.path(), "README.md", "only `.yaml` files");
    assert_refused_at(dir.path(), "nested/types.yml", "only `.yaml` files");
}

#[test]
fn a_symlink_is_refused_and_its_target_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(dir.path(), "types.yaml", LIBRARY.as_bytes());
    write(outside.path(), "target.yaml", LIBRARY.as_bytes());
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(outside.path().join("target.yaml"), dir.path().join("link.yaml")).is_ok();
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_file(outside.path().join("target.yaml"), dir.path().join("link.yaml")).is_ok();
    // Windows refuses an unprivileged symlink; the reparse-point check is
    // then proven only where symlinks can be made.
    if !created {
        return;
    }
    assert_refused_at(dir.path(), "link.yaml", "symlink or junction");
}

#[test]
fn a_linked_directory_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(dir.path(), "types.yaml", LIBRARY.as_bytes());
    write(outside.path(), "inner.yaml", LIBRARY.as_bytes());
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(outside.path(), dir.path().join("linked")).is_ok();
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_dir(outside.path(), dir.path().join("linked")).is_ok();
    if !created {
        return;
    }
    let errors = refusals(dir.path());
    assert!(errors.iter().any(|error| error.path == "linked"), "{errors:#?}");
    assert!(!errors.iter().any(|error| error.path.starts_with("linked/")), "nothing behind the link is walked: {errors:#?}");
}

#[test]
fn paths_that_differ_only_in_case_collide_at_any_depth() {
    let collisions = walk::case_collisions(&[
        "Types/a.yaml".to_string(),
        "types/b.yaml".to_string(),
        "other.yaml".to_string(),
        "OTHER.yaml".to_string(),
    ]);
    let at: Vec<&str> = collisions.iter().map(|error| error.path.as_str()).collect();
    assert_eq!(at, ["OTHER.yaml", "Types", "other.yaml", "types"]);
    assert!(collisions[0].message.contains("`other.yaml`"), "{collisions:#?}");
    assert!(walk::case_collisions(&["a/b.yaml".to_string(), "a/c.yaml".to_string()]).is_empty());
}

#[test]
fn nonportable_names_are_refused() {
    for name in ["has space.yaml", "ümlaut.yaml", ".hidden.yaml", "-dash.yaml", "trailing.", "con.yaml", "LPT1.yaml", "a:b.yaml", ""] {
        assert!(walk::check_portable_component(name).is_err(), "`{name}` is refused");
    }
    for name in ["seq.yaml", "content-policy.yaml", "formal_sequence.v2.yaml", "console.yaml"] {
        assert!(walk::check_portable_component(name).is_ok(), "`{name}` is portable");
    }
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "has space.yaml", LIBRARY.as_bytes());
    assert_refused_at(dir.path(), "has space.yaml", "not a portable name");
}

#[test]
fn a_file_that_is_not_utf8_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "types.yaml", b"kind: schema\ntypes:\n    thing: \xff\xfe\n");
    assert_refused_at(dir.path(), "types.yaml", "not UTF-8");
}

#[test]
fn ill_formed_member_envelopes_are_refused() {
    let cases: &[(&str, &str)] = &[
        ("kind: document\n", "its `kind` is `document`"),
        ("types:\n    a: string\n", "declares no `kind`"),
        ("- kind: schema\n", "not a YAML mapping"),
        ("kind: [schema]\n", "must be a string"),
        ("kind: schema\n$schema:\n    a: string\ntypes:\n    b: string\n", "may not declare `$schema`"),
        ("kind: schema\ntypes: {}\n", "non-empty `types`"),
        ("kind: schema-trigger\n$schema:\n    a: string\n", "declares a non-null `match`"),
        ("kind: namespace-schema\n$schema:\n    a: string\n", "string `namespace`"),
        ("kind: namespace-schema\nnamespace: seq\n", "`$schema` mapping"),
        ("kind: yaml-schema\n", "`$schema` mapping"),
        ("kind: schema\nname: Not_Kebab\ntypes:\n    a: string\n", "not a kebab-case identifier"),
        ("kind: schema\ndescription: [x]\ntypes:\n    a: string\n", "`description` must be a string"),
        ("kind: schema\ntypes:\n  a: [\n", "not one YAML document"),
        // The shared member admission's closed envelope and null rules
        // (each field's full matrix is Simplified Schema's
        // `member_envelope` test).
        ("kind: schema\ntypes:\n    a: string\nextra: 1\n", "`extra` is not a field of a `kind: schema` member"),
        ("kind: schema-trigger\nmatch:\n    a: string\ndescription:\n", "`description` must not be null"),
        ("kind: schema-trigger\nmatch:\n", "declares a non-null `match`"),
    ];
    for (text, needle) in cases {
        let error = walk::check_member(text.as_bytes()).expect_err(text);
        assert!(error.contains(needle), "{text:?}: expected {needle:?} in {error:?}");
    }
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "nested/bad.yaml", b"kind: document\n");
    assert_refused_at(dir.path(), "nested/bad.yaml", "its `kind` is `document`");
}

/// A refused member that declares a valid `name` is named by its label
/// beside its path, as the build failure prints it; an unnamed member, or
/// one whose `name` is itself the problem, by its path alone.
#[test]
fn a_named_member_refusal_shows_its_label_beside_its_path() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "inline-composition.yaml", format!("{TRIGGER}review-typo: 1\n").as_bytes());
    write(dir.path(), "library.yaml", format!("{LIBRARY}review-typo: 1\n").as_bytes());
    write(dir.path(), "named-badly.yaml", b"kind: schema\nname: Not_Kebab\ntypes:\n    a: string\n");
    let errors: Vec<String> = refusals(dir.path()).iter().map(ToString::to_string).collect();
    assert_eq!(errors.len(), 3, "{errors:#?}");
    assert!(errors[0].starts_with("`a-trigger (inline-composition.yaml)`: `review-typo` is not a field"), "{errors:#?}");
    assert!(errors[1].starts_with("`library.yaml`: `review-typo` is not a field"), "{errors:#?}");
    assert!(errors[2].starts_with("`named-badly.yaml`: its `name` `Not_Kebab`"), "{errors:#?}");
}

#[test]
fn a_missing_or_empty_root_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    assert_refused_at(dir.path(), "", "holds no member");
    assert_refused_at(&dir.path().join("absent"), "", "cannot be read");
}
