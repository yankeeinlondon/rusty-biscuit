//! Shared Content Policy declarations and schema mutations for the
//! `content_policy: "policy[](min(1))@…"` baseline declaration.
//!
//! Std-only, so the darkmatter lib, CLI, and DMLS suites can all include it
//! with `#[path]`. Each caller reads the real schema artifacts with its own
//! `include_str!` inside the `#[test]` function and passes the text in, as
//! `union_fixtures` explains.
//!
//! - [`DECLARATIONS`] pairs each complete `content_policy` declaration with
//!   two separately stated expectations: Content Policy's parser
//!   (`Policy::from_declaration`) and the baseline validator. Rows where they
//!   differ are the editor schema's documented limitations.
//! - [`LOAD_FAILURES`] are frontmatter texts the document reader must reject
//!   before either expectation applies (duplicate keys, malformed YAML).
//! - [`SCHEMA_MUTATIONS`] edit one cell of the baseline or policy schema and
//!   state the public outcome, so a disk and an embedded source can be held to
//!   the same table.

#![allow(dead_code, reason = "each including suite uses a different subset")]

/// The specification's authoring example: a schema-free document whose
/// mixed compact/long-form policy is valid for both the parser and the
/// baseline.
pub const MIXED_POLICY_FRONTMATTER: &str = "last_updated: 2026-10-04\n\
content_policy:\n\
\x20 - ValidFor(3mo)\n\
\x20 - rule: ValidUntil(2027-01-01)\n\
\x20   action: archive\n";

/// The rule suggestions Content Policy's editor schema ships, in declaration
/// order. Completion offers all of them wherever a rule is written (a compact
/// item or a long-form `rule:` value).
pub const RULE_SUGGESTIONS: [&str; 7] = [
    "Evergreen",
    "TimeSensitive",
    "ValidFor(3mo)",
    "ValidFor(3mo, @last_updated)",
    "ValidFor(3mo, 2026-09-28)",
    "ValidUntil(2027-01-01)",
    "FileChanged(src/config.rs, @config_fingerprint)",
];

/// The baseline declaration the `fm` partial carries, with its import
/// written relative to `darkmatter/schemas/partials/fm.yaml`.
pub const BASELINE_DECLARATION_LINE: &str =
    "        content_policy: \"policy[](min(1))@../../../content-policy/schemas/content-policy.yaml\"\n";

/// Where the runtime baseline's `fm` definition lives, relative to the
/// repository root.
pub const BASELINE_PATH: &str = "darkmatter/schemas/partials/fm.yaml";

/// Where Darkmatter's global catalog lives, relative to the repository root.
/// The runtime baseline is its resolved `fm` declaration.
pub const CATALOG_PATH: &str = "darkmatter/schemas/darkmatter.yaml";

/// The catalog's other partials, relative to the repository root, in the
/// order a caller passes their texts.
pub const OTHER_PARTIAL_PATHS: [&str; 4] = [
    "darkmatter/schemas/partials/style.yaml",
    "darkmatter/schemas/partials/context-variables.yaml",
    "darkmatter/schemas/partials/error.yaml",
    "darkmatter/schemas/partials/timing.yaml",
];

/// Where Content Policy's editor schema lives, relative to the repository
/// root.
pub const POLICY_SCHEMA_PATH: &str = "content-policy/schemas/content-policy.yaml";

/// The `fm` partial's text, after asserting it carries
/// [`BASELINE_DECLARATION_LINE`] exactly once. The schema mutations edit that
/// line, so a baseline that moved or respelled it fails here first.
pub fn baseline_with_policy(shipped_baseline: &str) -> String {
    assert_eq!(
        shipped_baseline.matches(BASELINE_DECLARATION_LINE).count(),
        1,
        "the shipped baseline declares `content_policy` exactly as the fixture expects"
    );
    shipped_baseline.to_string()
}

// ── declarations ───────────────────────────────────────────────────────────

/// The value written under `content_policy`.
#[derive(Debug, Clone, Copy)]
pub enum Declaration {
    /// No `content_policy` key at all.
    Absent,
    /// A value written exactly as given after `content_policy:` (empty means
    /// `content_policy:` with no value, YAML's null).
    Whole(&'static str),
    /// A list whose items are YAML flow values, rendered both as a block list
    /// and as a flow list.
    List(&'static [&'static str]),
}

/// How a [`Declaration::List`] is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    /// `content_policy:` followed by one `  - item` line per item.
    Block,
    /// `content_policy: [item, item]` on one line.
    Flow,
}

/// Accepted or rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Accept,
    Reject,
}

/// The authored node a problem's position must address.
#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    /// The `content_policy` value.
    Value,
    /// List item `n` (a mapping's start for a mapping item).
    Item(usize),
    /// The value of field `name` in list item `n`.
    Field(usize, &'static str),
}

/// One expected baseline validation problem.
#[derive(Debug, Clone, Copy)]
pub struct ExpectedProblem {
    /// The problem's JSON Pointer.
    pub path: &'static str,
    /// The CLI's JSON `kind`: `missing`, `type`, or `invalid`.
    pub kind: &'static str,
    pub at: Anchor,
}

/// One complete declaration and both consumers' expectations.
#[derive(Debug, Clone, Copy)]
pub struct DeclarationCase {
    pub name: &'static str,
    pub declaration: Declaration,
    /// `Policy::from_declaration`'s verdict; `None` when there is no
    /// declaration to parse (a caller's default policy is never substituted).
    pub parser: Option<Verdict>,
    /// The baseline validator's verdict.
    pub schema: Verdict,
    /// Every baseline problem, when `schema` rejects.
    pub problems: &'static [ExpectedProblem],
}

const fn agree(name: &'static str, items: &'static [&'static str]) -> DeclarationCase {
    DeclarationCase {
        name,
        declaration: Declaration::List(items),
        parser: Some(Verdict::Accept),
        schema: Verdict::Accept,
        problems: &[],
    }
}

const fn both_reject(
    name: &'static str,
    declaration: Declaration,
    problems: &'static [ExpectedProblem],
) -> DeclarationCase {
    DeclarationCase {
        name,
        declaration,
        parser: Some(Verdict::Reject),
        schema: Verdict::Reject,
        problems,
    }
}

/// A documented limitation: the parser rejects what the schema accepts.
const fn limitation(name: &'static str, declaration: Declaration) -> DeclarationCase {
    DeclarationCase {
        name,
        declaration,
        parser: Some(Verdict::Reject),
        schema: Verdict::Accept,
        problems: &[],
    }
}

const fn problem(path: &'static str, kind: &'static str, at: Anchor) -> ExpectedProblem {
    ExpectedProblem { path, kind, at }
}

const ITEM_0: &[ExpectedProblem] = &[problem("/content_policy/0", "invalid", Anchor::Item(0))];
const ITEM_1: &[ExpectedProblem] = &[problem("/content_policy/1", "invalid", Anchor::Item(1))];
const VALUE_TYPE: &[ExpectedProblem] = &[problem("/content_policy", "type", Anchor::Value)];
const VALUE_INVALID: &[ExpectedProblem] = &[problem("/content_policy", "invalid", Anchor::Value)];
const RULE: &[ExpectedProblem] =
    &[problem("/content_policy/0/rule", "invalid", Anchor::Field(0, "rule"))];
const ACTION: &[ExpectedProblem] =
    &[problem("/content_policy/0/action", "invalid", Anchor::Field(0, "action"))];

/// Every shared declaration case. Names are unique.
pub const DECLARATIONS: &[DeclarationCase] = &[
    // Absent: nothing to parse, nothing for the baseline to check.
    DeclarationCase {
        name: "absent",
        declaration: Declaration::Absent,
        parser: None,
        schema: Verdict::Accept,
        problems: &[],
    },
    // ── both accept ──
    agree(
        "every_rule_form_and_action",
        &[
            "TimeSensitive",
            "\"ValidFor(3mo)\"",
            "\"ValidFor(10d, @last_updated)\"",
            "\"ValidFor(2wk,2026-09-28)\"",
            "\"ValidFor( 1yr , @reviewed-on )\"",
            "\"ValidUntil(2027-01-01)\"",
            "\"ValidUntil(@retire_on)\"",
            "\"FileChanged(src/config.rs, @config_fingerprint)\"",
            "{rule: \"ValidFor(6mo)\", action: archive}",
            "{rule: TimeSensitive, action: remove}",
            "{rule: \"ValidUntil(2027-01-01)\", action: refresh}",
        ],
    ),
    agree("evergreen_alone", &["Evergreen"]),
    agree("evergreen_long_form_alone", &["{rule: Evergreen, action: refresh}"]),
    agree(
        "file_changed_paths_the_grammar_permits",
        &[
            "\"FileChanged(notes/a(b.md, @fp)\"",
            "\"FileChanged(&Cargo.toml, @fp)\"",
            "\"FileChanged(^README.md, @fp)\"",
            "\"FileChanged(../x.md, @fp)\"",
            "\"FileChanged(does/not/exist.md, @missing_fingerprint)\"",
        ],
    ),
    // ── both reject: rule grammar ──
    both_reject("duration_rule_name", Declaration::List(&["\"Duration(3mo)\""]), ITEM_0),
    both_reject("unknown_unit", Declaration::List(&["\"ValidFor(3w)\""]), ITEM_0),
    both_reject("capitalized_unit", Declaration::List(&["\"ValidFor(3MO)\""]), ITEM_0),
    both_reject("lowercase_rule_name", Declaration::List(&["evergreen"]), ITEM_0),
    both_reject("zero_duration", Declaration::List(&["\"ValidFor(0mo)\""]), ITEM_0),
    both_reject("leading_zero_duration", Declaration::List(&["\"ValidFor(03mo)\""]), ITEM_0),
    both_reject("nested_property", Declaration::List(&["\"ValidFor(3mo, @a.b)\""]), ITEM_0),
    both_reject(
        "padded_rule_is_not_trimmed",
        Declaration::List(&["\" ValidFor(3mo)\""]),
        ITEM_0,
    ),
    both_reject("malformed_date", Declaration::List(&["\"ValidUntil(2027-1-1)\""]), ITEM_0),
    // ── both reject: `FileChanged` grammar ──
    both_reject(
        "file_changed_without_fingerprint",
        Declaration::List(&["\"FileChanged(src/config.rs)\""]),
        ITEM_0,
    ),
    both_reject(
        "file_changed_comma_in_path",
        Declaration::List(&["\"FileChanged(src,config.rs, @fp)\""]),
        ITEM_0,
    ),
    both_reject(
        "file_changed_backslash_in_path",
        Declaration::List(&["'FileChanged(src\\config.rs, @fp)'"]),
        ITEM_0,
    ),
    both_reject(
        "file_changed_leading_space_is_not_trimmed",
        Declaration::List(&["\"FileChanged( src/a.rs, @fp)\""]),
        ITEM_0,
    ),
    both_reject(
        "file_changed_trailing_space_is_not_trimmed",
        Declaration::List(&["\"FileChanged(src/a.rs , @fp)\""]),
        ITEM_0,
    ),
    both_reject(
        "file_changed_absolute_path",
        Declaration::List(&["\"FileChanged(/etc/hosts, @fp)\""]),
        ITEM_0,
    ),
    both_reject("file_changed_home_prefix", Declaration::List(&["\"FileChanged(~/x, @fp)\""]), ITEM_0),
    both_reject("file_changed_at_prefix", Declaration::List(&["\"FileChanged(@x, @fp)\""]), ITEM_0),
    both_reject(
        "file_changed_percent_prefix",
        Declaration::List(&["\"FileChanged(%x, @fp)\""]),
        ITEM_0,
    ),
    both_reject(
        "file_changed_leading_template",
        Declaration::List(&["\"FileChanged({{HOME}}/x, @fp)\""]),
        ITEM_0,
    ),
    both_reject(
        "file_changed_nested_fingerprint_property",
        Declaration::List(&["\"FileChanged(src/config.rs, @a.b)\""]),
        ITEM_0,
    ),
    // ── both reject: long-form entries ──
    both_reject(
        "unknown_action",
        Declaration::List(&["{rule: \"ValidFor(3mo)\", action: delete}"]),
        ACTION,
    ),
    both_reject(
        "missing_action",
        Declaration::List(&["{rule: \"ValidFor(3mo)\"}"]),
        ITEM_0,
    ),
    both_reject("missing_rule", Declaration::List(&["{action: archive}"]), ITEM_0),
    both_reject("null_rule", Declaration::List(&["{rule: null, action: archive}"]), RULE),
    both_reject(
        "missing_rule_after_a_valid_entry",
        Declaration::List(&["\"ValidFor(3mo)\"", "{action: archive}"]),
        ITEM_1,
    ),
    both_reject(
        "null_rule_after_a_valid_entry",
        Declaration::List(&["\"ValidFor(3mo)\"", "{rule: null, action: archive}"]),
        &[problem("/content_policy/1/rule", "invalid", Anchor::Field(1, "rule"))],
    ),
    both_reject(
        "capitalized_action",
        Declaration::List(&["{rule: TimeSensitive, action: Archive}"]),
        ACTION,
    ),
    both_reject(
        "null_action",
        Declaration::List(&["{rule: \"ValidFor(3mo)\", action: null}"]),
        ACTION,
    ),
    both_reject(
        "extra_long_form_field",
        Declaration::List(&["{rule: \"ValidFor(3mo)\", action: archive, note: x}"]),
        ITEM_0,
    ),
    both_reject("rule_number", Declaration::List(&["{rule: 5, action: archive}"]), RULE),
    both_reject(
        "rule_list",
        Declaration::List(&["{rule: [\"ValidFor(3mo)\"], action: archive}"]),
        RULE,
    ),
    both_reject(
        "rule_mixed_list",
        Declaration::List(&["{rule: [\"ValidFor(3mo)\", 1], action: archive}"]),
        RULE,
    ),
    both_reject("rule_number_list", Declaration::List(&["{rule: [1], action: archive}"]), RULE),
    both_reject("rule_object", Declaration::List(&["{rule: {a: 1}, action: archive}"]), RULE),
    both_reject("rule_empty_string", Declaration::List(&["{rule: '', action: archive}"]), RULE),
    both_reject("rule_empty_list", Declaration::List(&["{rule: [], action: archive}"]), RULE),
    both_reject("rule_empty_object", Declaration::List(&["{rule: {}, action: archive}"]), RULE),
    both_reject("action_number", Declaration::List(&["{rule: TimeSensitive, action: 5}"]), ACTION),
    both_reject(
        "action_list",
        Declaration::List(&["{rule: TimeSensitive, action: [archive]}"]),
        ACTION,
    ),
    both_reject(
        "action_mixed_list",
        Declaration::List(&["{rule: TimeSensitive, action: [archive, 1]}"]),
        ACTION,
    ),
    both_reject(
        "action_number_list",
        Declaration::List(&["{rule: TimeSensitive, action: [1]}"]),
        ACTION,
    ),
    both_reject(
        "action_object",
        Declaration::List(&["{rule: TimeSensitive, action: {a: 1}}"]),
        ACTION,
    ),
    both_reject(
        "action_empty_string",
        Declaration::List(&["{rule: TimeSensitive, action: ''}"]),
        ACTION,
    ),
    both_reject(
        "action_empty_list",
        Declaration::List(&["{rule: TimeSensitive, action: []}"]),
        ACTION,
    ),
    both_reject(
        "action_empty_object",
        Declaration::List(&["{rule: TimeSensitive, action: {}}"]),
        ACTION,
    ),
    // ── both reject: list shape ──
    both_reject("null_item", Declaration::List(&["null"]), ITEM_0),
    both_reject("valid_then_number", Declaration::List(&["\"ValidFor(3mo)\"", "123"]), ITEM_1),
    both_reject("valid_then_null", Declaration::List(&["\"ValidFor(3mo)\"", "null"]), ITEM_1),
    both_reject("every_item_number", Declaration::List(&["123"]), ITEM_0),
    both_reject("whole_string", Declaration::Whole("\"ValidFor(3mo)\""), VALUE_TYPE),
    both_reject("whole_number", Declaration::Whole("5"), VALUE_TYPE),
    both_reject(
        "whole_object",
        Declaration::Whole("{rule: \"ValidFor(3mo)\", action: archive}"),
        VALUE_TYPE,
    ),
    both_reject("empty_list", Declaration::Whole("[]"), VALUE_INVALID),
    both_reject("empty_object", Declaration::Whole("{}"), VALUE_TYPE),
    // ── documented limitations: the parser rejects, the schema accepts ──
    limitation("impossible_date", Declaration::List(&["\"ValidUntil(2027-02-30)\""])),
    limitation(
        "duration_overflow",
        Declaration::List(&["\"ValidFor(99999999999999999999d)\""]),
    ),
    limitation(
        "unsupported_path_prefix",
        Declaration::List(&["\"FileChanged(vault:notes.md, @fp)\""]),
    ),
    limitation(
        "url_path",
        Declaration::List(&["\"FileChanged(https://example.com/a.md, @fp)\""]),
    ),
    limitation(
        "template_inside_path",
        Declaration::List(&["\"FileChanged(docs/{{HOME}}.md, @fp)\""]),
    ),
    limitation("evergreen_with_another_rule", Declaration::List(&["Evergreen", "\"ValidFor(3mo)\""])),
    limitation("whole_list_null", Declaration::Whole("null")),
    limitation("whole_list_empty_value", Declaration::Whole("")),
];

/// The [`DECLARATIONS`] rows the specification names as rejections every
/// entry point must report: an unknown rule, an unknown action, an empty
/// list, a value that is not a list, and a long-form entry without `rule` or
/// with `rule: null`.
pub const ACCEPTANCE_REJECTIONS: [&str; 6] = [
    "duration_rule_name",
    "unknown_action",
    "empty_list",
    "whole_string",
    "missing_rule",
    "null_rule",
];

/// The [`DECLARATIONS`] row called `name`.
///
/// ## Panics
///
/// When no row has that name.
pub fn declaration(name: &str) -> &'static DeclarationCase {
    DECLARATIONS
        .iter()
        .find(|case| case.name == name)
        .unwrap_or_else(|| panic!("no declaration case `{name}`"))
}

/// A declaration written into frontmatter, with the authored start of every
/// node an [`Anchor`] can name. Lines count the whole document (the opening
/// `---` is line 1); columns are 1-based UTF-8 byte columns.
#[derive(Debug, Clone)]
pub struct Rendered {
    pub presentation: Presentation,
    /// The frontmatter lines (no delimiters).
    pub frontmatter: String,
    items: Vec<&'static str>,
    item_starts: Vec<(u32, u32)>,
    value_start: Option<(u32, u32)>,
}

impl Rendered {
    /// The complete Markdown document: the frontmatter between `---` lines,
    /// then a body.
    pub fn document(&self) -> String {
        format!("---\n{}---\nBody\n", self.frontmatter)
    }

    /// The line and column `anchor` names.
    pub fn start(&self, anchor: Anchor) -> (u32, u32) {
        match anchor {
            Anchor::Value => self.value_start.expect("this presentation pins its value start"),
            Anchor::Item(index) => self.item_starts[index],
            Anchor::Field(index, name) => {
                let (line, column) = self.item_starts[index];
                let needle = format!("{name}: ");
                let offset = self.items[index]
                    .find(&needle)
                    .unwrap_or_else(|| panic!("item {index} has no `{needle}`"));
                (line, column + (offset + needle.len()) as u32)
            }
        }
    }
}

/// Every presentation of `declaration`: block and flow for a list, the one
/// authored form otherwise, and nothing for [`Declaration::Absent`] beyond an
/// empty frontmatter.
pub fn render(declaration: Declaration) -> Vec<Rendered> {
    const KEY: &str = "content_policy:";
    match declaration {
        Declaration::Absent => vec![Rendered {
            presentation: Presentation::Flow,
            frontmatter: "title: no policy\n".into(),
            items: Vec::new(),
            item_starts: Vec::new(),
            value_start: None,
        }],
        Declaration::Whole(value) => vec![Rendered {
            presentation: Presentation::Flow,
            frontmatter: if value.is_empty() { format!("{KEY}\n") } else { format!("{KEY} {value}\n") },
            items: Vec::new(),
            item_starts: Vec::new(),
            value_start: (!value.is_empty()).then_some((2, KEY.len() as u32 + 2)),
        }],
        Declaration::List(items) => {
            let mut block = format!("{KEY}\n");
            let mut block_starts = Vec::new();
            for (index, item) in items.iter().enumerate() {
                block.push_str(&format!("  - {item}\n"));
                block_starts.push((3 + index as u32, 5));
            }
            let mut flow = format!("{KEY} [");
            let mut flow_starts = Vec::new();
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    flow.push_str(", ");
                }
                flow_starts.push((2, flow.len() as u32 + 1));
                flow.push_str(item);
            }
            flow.push_str("]\n");
            vec![
                Rendered {
                    presentation: Presentation::Block,
                    frontmatter: block,
                    items: items.to_vec(),
                    item_starts: block_starts,
                    // No case pins a block list's own start yet.
                    value_start: None,
                },
                Rendered {
                    presentation: Presentation::Flow,
                    frontmatter: flow,
                    items: items.to_vec(),
                    item_starts: flow_starts,
                    value_start: Some((2, KEY.len() as u32 + 2)),
                },
            ]
        }
    }
}

// ── reader-boundary failures ───────────────────────────────────────────────

/// Frontmatter the document reader rejects, by the field it damages. Neither
/// the parser nor the baseline sees these: a deduplicated or partially read
/// mapping would hide the defect.
pub const LOAD_FAILURES: &[(&str, &str)] = &[
    (
        "duplicate_content_policy",
        "content_policy: [\"ValidFor(3mo)\"]\ncontent_policy: [TimeSensitive]\n",
    ),
    (
        "duplicate_rule",
        "content_policy:\n  - rule: \"ValidFor(3mo)\"\n    rule: TimeSensitive\n    action: archive\n",
    ),
    (
        "duplicate_action",
        "content_policy:\n  - rule: \"ValidFor(3mo)\"\n    action: archive\n    action: remove\n",
    ),
    ("malformed_content_policy", "content_policy: [\"ValidFor(3mo)\"\n"),
    ("malformed_rule", "content_policy:\n  - {rule: \"ValidFor(3mo), action: archive}\n"),
    ("malformed_action", "content_policy:\n  - {rule: TimeSensitive, action: [archive}\n"),
];

// ── schema mutations ───────────────────────────────────────────────────────

/// Which schema file a mutation edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaFile {
    /// Darkmatter's global catalog, whose `fm` declaration is the baseline.
    Catalog,
    /// The baseline's `fm` partial, after [`baseline_with_policy`].
    Baseline,
    /// Content Policy's shipped editor schema.
    Policy,
}

/// One edit to a schema file's text.
#[derive(Debug, Clone, Copy)]
pub enum Edit {
    /// Replace `from`, which occurs exactly once.
    Replace(&'static str, &'static str),
    /// Replace the text from `start` (inclusive) up to `end` (exclusive), each
    /// occurring exactly once.
    Span(&'static str, &'static str, &'static str),
    /// Replace the text from `start` (inclusive, occurring exactly once) to
    /// the end of the file.
    Tail(&'static str, &'static str),
    /// Append text to the file.
    Append(&'static str),
    /// Replace the whole file.
    Whole(&'static str),
}

impl Edit {
    /// Applies the edit to `text`.
    pub fn apply(self, text: &str) -> String {
        let once = |needle: &str| {
            assert_eq!(text.matches(needle).count(), 1, "`{needle}` must occur exactly once");
            text.find(needle).expect("needle")
        };
        match self {
            Edit::Replace(from, to) => {
                once(from);
                text.replacen(from, to, 1)
            }
            Edit::Span(start, end, to) => {
                let from = once(start);
                let until = once(end);
                assert!(from < until, "`{start}` precedes `{end}`");
                format!("{}{to}{}", &text[..from], &text[until..])
            }
            Edit::Tail(start, to) => format!("{}{to}", &text[..once(start)]),
            Edit::Append(extra) => format!("{text}{extra}"),
            Edit::Whole(replacement) => replacement.to_string(),
        }
    }
}

/// The public outcome a schema mutation must produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaExpect {
    /// The unedited control: [`VALID_CONTROL`] passes, [`INVALID_CONTROL`]
    /// fails.
    Control,
    /// Schema preparation fails; no baseline, empty or partial, is used.
    PrepareFails,
    /// A legal schema that no longer enforces the policy: both controls pass.
    Weakened,
    /// A legal schema whose typing changed: [`VALID_CONTROL`] now fails.
    RejectsValid,
}

/// The valid control document's frontmatter for schema mutations.
pub const VALID_CONTROL: &str = MIXED_POLICY_FRONTMATTER;

/// The invalid control document's frontmatter: an unknown rule name.
pub const INVALID_CONTROL: &str = "content_policy:\n  - Duration(3mo)\n";

/// One mutated cell.
#[derive(Debug, Clone, Copy)]
pub struct SchemaMutation {
    /// The field column (`$schema`, `kind`, `types`, `$schema.content_policy`,
    /// `types.short_form`, `types.long_form`, `types.policy`,
    /// `long_form.rule`, `long_form.action`).
    pub column: &'static str,
    /// The matrix row.
    pub cell: &'static str,
    pub file: SchemaFile,
    pub edit: Edit,
    pub expect: SchemaExpect,
    /// An extra frontmatter whose verdict proves a changed decision.
    pub probe: Option<(&'static str, Verdict)>,
}

const fn mutation(
    column: &'static str,
    cell: &'static str,
    file: SchemaFile,
    edit: Edit,
    expect: SchemaExpect,
) -> SchemaMutation {
    SchemaMutation { column, cell, file, edit, expect, probe: None }
}

const fn probed(
    column: &'static str,
    cell: &'static str,
    file: SchemaFile,
    edit: Edit,
    expect: SchemaExpect,
    probe: (&'static str, Verdict),
) -> SchemaMutation {
    SchemaMutation { column, cell, file, edit, expect, probe: Some(probe) }
}

use SchemaExpect::{Control, PrepareFails, RejectsValid, Weakened};
use SchemaFile::{Baseline, Catalog, Policy};

const DECLARATION: &str = BASELINE_DECLARATION_LINE;
const SHORT_FORM: &str = "  short_form:\n";
const LONG_FORM: &str = "  long_form:\n";
const POLICY: &str = "  policy:\n    - short_form\n    - long_form\n";
const RULE_LINE: &str = "    rule: \"short_form(required)\"\n";
const ACTION_LINE: &str = "    action: \"enum(refresh, archive, remove; required)\"\n";

/// Every schema mutation, grouped by column. Each column's first row is its
/// unedited control.
pub const SCHEMA_MUTATIONS: &[SchemaMutation] = &[
    // ── catalog `$schema` envelope ──
    mutation("$schema", "control", Catalog, Edit::Append(""), Control),
    mutation("$schema", "absent", Catalog, Edit::Replace("\n$schema:\n", "\nnot_schema:\n"), PrepareFails),
    mutation("$schema", "null", Catalog, Edit::Whole("kind: schema\n$schema: null\n"), PrepareFails),
    mutation("$schema", "empty-value null", Catalog, Edit::Whole("kind: schema\n$schema:\n"), PrepareFails),
    mutation("$schema", "wrong type", Catalog, Edit::Whole("kind: schema\n$schema: 5\n"), PrepareFails),
    mutation(
        "$schema",
        "one wrong element",
        Catalog,
        Edit::Whole("kind: schema\n$schema:\n  - {title: string}\n  - 5\n"),
        PrepareFails,
    ),
    mutation("$schema", "every element wrong", Catalog, Edit::Whole("kind: schema\n$schema: [1, 2]\n"), PrepareFails),
    // The catalog declares no globals: a load failure, not an empty baseline.
    mutation("$schema", "empty mapping", Catalog, Edit::Whole("kind: schema\n$schema: {}\n"), PrepareFails),
    mutation("$schema", "empty union", Catalog, Edit::Whole("kind: schema\n$schema: []\n"), PrepareFails),
    mutation("$schema", "duplicate key", Catalog, Edit::Append("$schema:\n    title: string\n"), PrepareFails),
    mutation("$schema", "trailing document", Catalog, Edit::Append("---\nextra: 1\n"), PrepareFails),
    mutation("$schema", "invalid YAML", Catalog, Edit::Append("  : [\n"), PrepareFails),
    // ── policy `kind` ──
    mutation("kind", "control", Policy, Edit::Append(""), Control),
    mutation("kind", "absent", Policy, Edit::Replace("\nkind: schema\n", "\n"), PrepareFails),
    mutation("kind", "null", Policy, Edit::Replace("\nkind: schema\n", "\nkind: null\n"), PrepareFails),
    mutation("kind", "wrong type", Policy, Edit::Replace("\nkind: schema\n", "\nkind: 1\n"), PrepareFails),
    mutation(
        "kind",
        "one wrong element",
        Policy,
        Edit::Replace("\nkind: schema\n", "\nkind: [schema, 1]\n"),
        PrepareFails,
    ),
    mutation(
        "kind",
        "every element wrong",
        Policy,
        Edit::Replace("\nkind: schema\n", "\nkind: [1, 2]\n"),
        PrepareFails,
    ),
    mutation("kind", "empty string", Policy, Edit::Replace("\nkind: schema\n", "\nkind: ''\n"), PrepareFails),
    mutation("kind", "empty mapping", Policy, Edit::Replace("\nkind: schema\n", "\nkind: {}\n"), PrepareFails),
    mutation(
        "kind",
        "duplicate key",
        Policy,
        Edit::Replace("\nkind: schema\n", "\nkind: schema\nkind: schema\n"),
        PrepareFails,
    ),
    mutation("kind", "trailing document", Policy, Edit::Append("---\nkind: schema\n"), PrepareFails),
    // ── policy `types` ──
    mutation("types", "control", Policy, Edit::Append(""), Control),
    mutation("types", "absent", Policy, Edit::Tail("types:\n", ""), PrepareFails),
    mutation("types", "null", Policy, Edit::Tail("types:\n", "types: null\n"), PrepareFails),
    mutation("types", "wrong type", Policy, Edit::Tail("types:\n", "types: 1\n"), PrepareFails),
    mutation(
        "types",
        "one wrong element",
        Policy,
        Edit::Tail("types:\n", "types: [{policy: string}, 1]\n"),
        PrepareFails,
    ),
    mutation(
        "types",
        "every element wrong",
        Policy,
        Edit::Tail("types:\n", "types: [1, 2]\n"),
        PrepareFails,
    ),
    mutation("types", "empty", Policy, Edit::Tail("types:\n", "types: {}\n"), PrepareFails),
    mutation("types", "duplicate key", Policy, Edit::Append("types:\n  other: string\n"), PrepareFails),
    mutation("types", "invalid YAML", Policy, Edit::Append("  : [\n"), PrepareFails),
    // ── baseline `$schema.content_policy` ──
    mutation("$schema.content_policy", "control", Baseline, Edit::Append(""), Control),
    mutation("$schema.content_policy", "absent", Baseline, Edit::Replace(DECLARATION, ""), Weakened),
    mutation(
        "$schema.content_policy",
        "null",
        Baseline,
        Edit::Replace(DECLARATION, "        content_policy: null\n"),
        RejectsValid,
    ),
    mutation(
        "$schema.content_policy",
        "empty-value null",
        Baseline,
        Edit::Replace(DECLARATION, "        content_policy:\n"),
        RejectsValid,
    ),
    mutation(
        "$schema.content_policy",
        "wrong type",
        Baseline,
        Edit::Replace(DECLARATION, "        content_policy: 5\n"),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "wrong type bool",
        Baseline,
        Edit::Replace(DECLARATION, "        content_policy: true\n"),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "one wrong element",
        Baseline,
        Edit::Replace(
            DECLARATION,
            "        content_policy: [\"policy[](min(1))@../../../content-policy/schemas/content-policy.yaml\", 5]\n",
        ),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "every element wrong",
        Baseline,
        Edit::Replace(DECLARATION, "        content_policy: [1, 2]\n"),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "empty string",
        Baseline,
        Edit::Replace(DECLARATION, "        content_policy: ''\n"),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "empty list",
        Baseline,
        Edit::Replace(DECLARATION, "        content_policy: []\n"),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "empty object",
        Baseline,
        Edit::Replace(DECLARATION, "        content_policy: {}\n"),
        RejectsValid,
    ),
    mutation(
        "$schema.content_policy",
        "duplicate key",
        Baseline,
        Edit::Replace(DECLARATION, concat!(
            "        content_policy: \"policy[](min(1))@../../../content-policy/schemas/content-policy.yaml\"\n",
            "        content_policy: \"policy[](min(1))@../../../content-policy/schemas/content-policy.yaml\"\n",
        )),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "malformed constraint",
        Baseline,
        Edit::Replace(
            DECLARATION,
            "        content_policy: \"policy[](min(1)@../../../content-policy/schemas/content-policy.yaml\"\n",
        ),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "misspelled imported type",
        Baseline,
        Edit::Replace(
            DECLARATION,
            "        content_policy: \"polcy[](min(1))@../../../content-policy/schemas/content-policy.yaml\"\n",
        ),
        PrepareFails,
    ),
    mutation(
        "$schema.content_policy",
        "import target missing",
        Baseline,
        Edit::Replace(
            DECLARATION,
            "        content_policy: \"policy[](min(1))@../../content-policy/schemas/content-policy.yaml\"\n",
        ),
        PrepareFails,
    ),
    probed(
        "$schema.content_policy",
        "minimum removed",
        Baseline,
        Edit::Replace(
            DECLARATION,
            "        content_policy: \"policy[]@../../../content-policy/schemas/content-policy.yaml\"\n",
        ),
        Control,
        ("content_policy: []\n", Verdict::Accept),
    ),
    mutation(
        "$schema.content_policy",
        "invalid YAML",
        Baseline,
        Edit::Replace(
            DECLARATION,
            "        content_policy: \"policy[](min(1))@../../../content-policy/schemas/content-policy.yaml\n",
        ),
        PrepareFails,
    ),
    // ── policy `types.short_form` ──
    mutation("types.short_form", "control", Policy, Edit::Append(""), Control),
    mutation("types.short_form", "absent", Policy, Edit::Span(SHORT_FORM, LONG_FORM, ""), PrepareFails),
    mutation(
        "types.short_form",
        "null",
        Policy,
        Edit::Span(SHORT_FORM, LONG_FORM, "  short_form: null\n"),
        RejectsValid,
    ),
    mutation(
        "types.short_form",
        "wrong type",
        Policy,
        Edit::Span(SHORT_FORM, LONG_FORM, "  short_form: 5\n"),
        PrepareFails,
    ),
    mutation(
        "types.short_form",
        "wrong type bool",
        Policy,
        Edit::Span(SHORT_FORM, LONG_FORM, "  short_form: true\n"),
        PrepareFails,
    ),
    mutation(
        "types.short_form",
        "one wrong element",
        Policy,
        Edit::Span(SHORT_FORM, LONG_FORM, "  short_form:\n    - 'string(pattern(^Evergreen$))'\n    - 5\n"),
        PrepareFails,
    ),
    mutation(
        "types.short_form",
        "every element wrong",
        Policy,
        Edit::Span(SHORT_FORM, LONG_FORM, "  short_form: [1, 2]\n"),
        PrepareFails,
    ),
    mutation(
        "types.short_form",
        "empty union",
        Policy,
        Edit::Span(SHORT_FORM, LONG_FORM, "  short_form: []\n"),
        PrepareFails,
    ),
    mutation(
        "types.short_form",
        "empty object",
        Policy,
        Edit::Span(SHORT_FORM, LONG_FORM, "  short_form: {}\n"),
        RejectsValid,
    ),
    mutation(
        "types.short_form",
        "duplicate definition",
        Policy,
        Edit::Append("  short_form: string\n"),
        PrepareFails,
    ),
    mutation(
        "types.short_form",
        "malformed definition",
        Policy,
        Edit::Span(SHORT_FORM, LONG_FORM, "  short_form:\n    - 'string(pattern(^Evergreen$)'\n"),
        PrepareFails,
    ),
    // ── policy `types.long_form` ──
    mutation("types.long_form", "control", Policy, Edit::Append(""), Control),
    mutation("types.long_form", "absent", Policy, Edit::Span(LONG_FORM, "  policy:\n", ""), PrepareFails),
    mutation(
        "types.long_form",
        "null",
        Policy,
        Edit::Span(LONG_FORM, "  policy:\n", "  long_form: null\n"),
        RejectsValid,
    ),
    mutation(
        "types.long_form",
        "wrong type",
        Policy,
        Edit::Span(LONG_FORM, "  policy:\n", "  long_form: 5\n"),
        PrepareFails,
    ),
    mutation(
        "types.long_form",
        "one wrong field",
        Policy,
        Edit::Span(LONG_FORM, "  policy:\n", "  long_form:\n    rule: short_form(required)\n    action: 5\n"),
        PrepareFails,
    ),
    mutation(
        "types.long_form",
        "every field wrong",
        Policy,
        Edit::Span(LONG_FORM, "  policy:\n", "  long_form:\n    rule: 5\n    action: 5\n"),
        PrepareFails,
    ),
    mutation(
        "types.long_form",
        "empty union",
        Policy,
        Edit::Span(LONG_FORM, "  policy:\n", "  long_form: []\n"),
        PrepareFails,
    ),
    probed(
        "types.long_form",
        "empty object",
        Policy,
        Edit::Span(LONG_FORM, "  policy:\n", "  long_form: {}\n"),
        RejectsValid,
        ("content_policy:\n  - {}\n", Verdict::Accept),
    ),
    mutation(
        "types.long_form",
        "duplicate field",
        Policy,
        Edit::Replace(RULE_LINE, "    rule: short_form(required)\n    rule: short_form(required)\n"),
        PrepareFails,
    ),
    mutation(
        "types.long_form",
        "duplicate definition",
        Policy,
        Edit::Append("  long_form: string\n"),
        PrepareFails,
    ),
    mutation(
        "types.long_form",
        "malformed field definition",
        Policy,
        Edit::Span(LONG_FORM, "  policy:\n", "  long_form:\n    rule: \"short_form(required\"\n    action: \"enum(refresh, archive, remove; required)\"\n"),
        PrepareFails,
    ),
    mutation(
        "types.long_form",
        "invalid YAML",
        Policy,
        Edit::Span(LONG_FORM, "  policy:\n", "  long_form:\n    rule: [short_form\n"),
        PrepareFails,
    ),
    // ── policy `types.policy` ──
    mutation("types.policy", "control", Policy, Edit::Append(""), Control),
    mutation("types.policy", "absent", Policy, Edit::Replace(POLICY, ""), PrepareFails),
    mutation("types.policy", "null", Policy, Edit::Replace(POLICY, "  policy: null\n"), RejectsValid),
    mutation("types.policy", "wrong type", Policy, Edit::Replace(POLICY, "  policy: 1\n"), PrepareFails),
    mutation(
        "types.policy",
        "one wrong element",
        Policy,
        Edit::Replace(POLICY, "  policy:\n    - short_form\n    - 1\n"),
        PrepareFails,
    ),
    mutation("types.policy", "every element wrong", Policy, Edit::Replace(POLICY, "  policy: [1, 2]\n"), PrepareFails),
    mutation("types.policy", "empty union", Policy, Edit::Replace(POLICY, "  policy: []\n"), PrepareFails),
    mutation("types.policy", "empty object", Policy, Edit::Replace(POLICY, "  policy: {}\n"), RejectsValid),
    mutation("types.policy", "duplicate definition", Policy, Edit::Append("  policy: [short_form]\n"), PrepareFails),
    mutation(
        "types.policy",
        "misspelled local name",
        Policy,
        Edit::Replace(POLICY, "  policy:\n    - short_fom\n    - long_form\n"),
        PrepareFails,
    ),
    mutation(
        "types.policy",
        "malformed reference",
        Policy,
        Edit::Replace(POLICY, "  policy:\n    - short_form(\n    - long_form\n"),
        PrepareFails,
    ),
    // ── policy `long_form.rule` ──
    mutation("long_form.rule", "control", Policy, Edit::Append(""), Control),
    mutation("long_form.rule", "absent", Policy, Edit::Replace(RULE_LINE, ""), RejectsValid),
    mutation("long_form.rule", "null", Policy, Edit::Replace(RULE_LINE, "    rule: null\n"), RejectsValid),
    mutation("long_form.rule", "empty-value null", Policy, Edit::Replace(RULE_LINE, "    rule:\n"), RejectsValid),
    mutation("long_form.rule", "wrong type", Policy, Edit::Replace(RULE_LINE, "    rule: 5\n"), PrepareFails),
    mutation("long_form.rule", "wrong type bool", Policy, Edit::Replace(RULE_LINE, "    rule: true\n"), PrepareFails),
    mutation(
        "long_form.rule",
        "one wrong element",
        Policy,
        Edit::Replace(RULE_LINE, "    rule: [short_form, 5]\n"),
        PrepareFails,
    ),
    mutation("long_form.rule", "every element wrong", Policy, Edit::Replace(RULE_LINE, "    rule: [1, 2]\n"), PrepareFails),
    mutation("long_form.rule", "empty string", Policy, Edit::Replace(RULE_LINE, "    rule: ''\n"), PrepareFails),
    mutation("long_form.rule", "empty list", Policy, Edit::Replace(RULE_LINE, "    rule: []\n"), PrepareFails),
    mutation("long_form.rule", "empty object", Policy, Edit::Replace(RULE_LINE, "    rule: {}\n"), RejectsValid),
    mutation(
        "long_form.rule",
        "duplicate key",
        Policy,
        Edit::Replace(RULE_LINE, "    rule: short_form(required)\n    rule: short_form(required)\n"),
        PrepareFails,
    ),
    mutation(
        "long_form.rule",
        "malformed reference",
        Policy,
        Edit::Replace(RULE_LINE, "    rule: short_form(required\n"),
        PrepareFails,
    ),
    mutation(
        "long_form.rule",
        "misspelled local name",
        Policy,
        Edit::Replace(RULE_LINE, "    rule: short_fom(required)\n"),
        PrepareFails,
    ),
    probed(
        "long_form.rule",
        "required removed",
        Policy,
        Edit::Replace(RULE_LINE, "    rule: short_form\n"),
        Control,
        ("content_policy:\n  - {action: archive}\n", Verdict::Accept),
    ),
    // ── policy `long_form.action` ──
    mutation("long_form.action", "control", Policy, Edit::Append(""), Control),
    mutation("long_form.action", "absent", Policy, Edit::Replace(ACTION_LINE, ""), RejectsValid),
    mutation("long_form.action", "null", Policy, Edit::Replace(ACTION_LINE, "    action: null\n"), RejectsValid),
    mutation("long_form.action", "empty-value null", Policy, Edit::Replace(ACTION_LINE, "    action:\n"), RejectsValid),
    mutation("long_form.action", "wrong type", Policy, Edit::Replace(ACTION_LINE, "    action: 5\n"), PrepareFails),
    mutation(
        "long_form.action",
        "wrong type bool",
        Policy,
        Edit::Replace(ACTION_LINE, "    action: true\n"),
        PrepareFails,
    ),
    mutation(
        "long_form.action",
        "one wrong element",
        Policy,
        Edit::Replace(ACTION_LINE, "    action: [string, 5]\n"),
        PrepareFails,
    ),
    mutation(
        "long_form.action",
        "every element wrong",
        Policy,
        Edit::Replace(ACTION_LINE, "    action: [1, 2]\n"),
        PrepareFails,
    ),
    mutation("long_form.action", "empty string", Policy, Edit::Replace(ACTION_LINE, "    action: ''\n"), PrepareFails),
    mutation("long_form.action", "empty list", Policy, Edit::Replace(ACTION_LINE, "    action: []\n"), PrepareFails),
    mutation("long_form.action", "empty object", Policy, Edit::Replace(ACTION_LINE, "    action: {}\n"), RejectsValid),
    mutation(
        "long_form.action",
        "duplicate key",
        Policy,
        Edit::Replace(ACTION_LINE, concat!(
            "    action: \"enum(refresh, archive, remove; required)\"\n",
            "    action: \"enum(refresh, archive, remove; required)\"\n",
        )),
        PrepareFails,
    ),
    mutation(
        "long_form.action",
        "malformed constraint",
        Policy,
        Edit::Replace(ACTION_LINE, "    action: \"enum(refresh, archive, remove; required\"\n"),
        PrepareFails,
    ),
    probed(
        "long_form.action",
        "required removed",
        Policy,
        Edit::Replace(ACTION_LINE, "    action: \"enum(refresh, archive, remove)\"\n"),
        Control,
        ("content_policy:\n  - {rule: TimeSensitive}\n", Verdict::Accept),
    ),
];
