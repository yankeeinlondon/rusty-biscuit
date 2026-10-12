//! Fixtures for references to union-typed named types (`U[]`, `U(required)`,
//! `U -> text`).
//!
//! Std-only, so the darkmatter lib, CLI, and DMLS suites can all include it
//! with `#[path]`. Each caller reads the shipped Content Policy schema with
//! its own `include_str!` inside the `#[test]` function and passes the text
//! in. CI's test-input index maps a literal inside a test to that test; the
//! same literal in this helper would widen the cell to the whole binary.

#![allow(dead_code, reason = "each including suite uses a different subset")]

use std::path::Path;

/// File name the Content Policy fixture is written under.
pub const CONTENT_POLICY_FILE: &str = "content-policy.yaml";

/// The document property type the acceptance fixture declares.
pub const CONTENT_POLICY_LIST: &str = "policy[](min(1))@./content-policy.yaml";

/// Replaces `from` with `to`, requiring exactly one match. A shipped-schema
/// edit that moves the line fails here instead of silently testing the
/// unmodified schema.
fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "the shipped content-policy schema no longer contains `{from}` exactly once; update the fixture edit",
    );
    text.replacen(from, to, 1)
}

/// The shipped schema with `long_form.rule` made optional (`short_form`
/// instead of `short_form(required)`). It resolves without `U(required)`, so
/// it is the control for [`content_policy_union_schema`].
pub fn content_policy_bare_schema(shipped: &str) -> String {
    replace_once(shipped, "    rule: \"short_form(required)\"", "    rule: short_form")
}

/// The acceptance fixture: the shipped schema, which writes same-file
/// references bare and makes `long_form.rule` `short_form(required)`.
pub fn content_policy_union_schema(shipped: &str) -> String {
    assert_eq!(
        shipped.matches("    rule: \"short_form(required)\"").count(),
        1,
        "the shipped content-policy schema no longer requires `long_form.rule`; update the fixture",
    );
    shipped.to_string()
}

/// Acceptance criterion 1 for a `content_policy: policy[](min(1))@…`
/// property: each frontmatter body (empty for a missing property) and whether
/// it validates.
pub const CONTENT_POLICY_CASES: &[(&str, bool)] = &[
    (
        "content_policy: [\"ValidFor(3mo)\", {rule: \"ValidUntil(2027-01-01)\", action: archive}]\n",
        true,
    ),
    ("", true),
    ("content_policy: null\n", true),
    ("content_policy: [null]\n", false),
    ("content_policy: \"ValidFor(3mo)\"\n", false),
    ("content_policy: []\n", false),
    ("content_policy: [\"Duration(3mo)\"]\n", false),
    ("content_policy: [{action: archive}]\n", false),
    ("content_policy: [{rule: null, action: archive}]\n", false),
    ("content_policy: [{rule: \"ValidFor(3mo)\", action: delete}]\n", false),
];

/// A literal-tagged union of closed object arms. `kind` selects one arm.
pub const TAGGED_TYPES: &str = "kind: schema\ntypes:\n  step:\n    - \"{ kind: literal(shell; required), run: string(required) }\"\n    - \"{ kind: literal(say; required), text: string(required) }\"\n";

/// An untagged union whose object arms overlap: `{ name: x }` matches both.
pub const OVERLAPPING_TYPES: &str = "kind: schema\ntypes:\n  entry:\n    - \"{ name: string(required) }\"\n    - \"{ name: string(required), size: number }\"\n";

/// A scalar/object union carrying an eager and a lazy `file` arm.
pub const FILE_TYPES: &str = "kind: schema\ntypes:\n  asset: [\"file(eager)\", number]\n  lazy_asset: [file, number]\n";

/// Direct and postfixed nested unions in pairs: `nested` and `nested_tag`
/// postfix their inner union reference, `direct` and `direct_tag` are the
/// unpostfixed controls. Both forms flatten into one list of alternatives.
pub const NESTED_UNION_TYPES: &str = "kind: schema\ntypes:\n  toggle: [boolish, number]\n  direct: [toggle, object]\n  nested: [toggle(required), object]\n  tagged:\n    - \"{kind: literal(a; required), a: string(required)}\"\n    - \"{kind: literal(b; required), b: string(required)}\"\n  direct_tag: [tagged, number]\n  nested_tag: [tagged(required), number]\n";

/// The file name [`NESTED_UNION_TYPES`] is written to.
pub const NESTED_UNION_FILE: &str = "types.yaml";

/// A union reached through two imports: `outer.yaml` → `middle.yaml`, whose
/// arm is a bare name defined in `middle.yaml` itself.
pub const MULTI_HOP_FILES: &[(&str, &str)] = &[
    (
        "outer.yaml",
        "kind: schema\ntypes:\n  choice: \"middle_choice@./middle.yaml\"\n",
    ),
    (
        "middle.yaml",
        "kind: schema\ntypes:\n  word: string(pattern(^[a-z]+$))\n  middle_choice: [word, number]\n",
    ),
];

/// A property that `md clean` repairs by quoting (`label: 1.20` → `"1.20"`),
/// to sit beside an unrepairable union item.
pub const REPAIRABLE_SIBLING: (&str, &str) = ("label: string", "label: 1.20\n");

/// Writes `files` (name, contents) below `dir`.
pub fn write_files(dir: &Path, files: &[(&str, &str)]) {
    for (name, contents) in files {
        std::fs::write(dir.join(name), contents).expect("write union fixture file");
    }
}
