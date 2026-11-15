//! Typed shell-template policy primitives: placeholder types and the covers
//! relation, `template` rules in `permissions.shell-commands` and their
//! robustness matrix, and allow/exclusion matching (acceptance criteria 3,
//! 4, 5, 13, and 17 of `2026-10-23-shell-template-approval`, at the public
//! library boundary).

use darkmatter::markdown::Markdown;
use darkmatter::config::load::ConfigError;
use darkmatter::config::permissions::persist::{PersistentApproval, save_user_approval};
use darkmatter::markdown::compose::shell_expansion::template::{
    ApprovalType, FilledChain, FilledSegment, HoleType, HoleValidator, TemplateShape, TypeProvenance,
    parse_authored,
};
use darkmatter::markdown::compose::expression::{DeclaredTypes, TypeScope};
use darkmatter::markdown::compose::shell_expansion::types::{
    ChainOperator, ShellRuleEntry, ShellRuleSet,
};
use darkmatter::markdown::compose::shell_expansion::{
    check_filled_chain, check_template_blacklist, check_template_exclusion, check_user_blacklist,
    filled_template_needs_approval, normalize_command,
};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use simplified_schema::SchemaRoot;
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

/// The `SchemaRoot` projection of a document whose `$schema` body is
/// `schema` (YAML lines indented two spaces).
fn schema(schema: &str) -> Option<SchemaRoot> {
    let source: Markdown = format!("---\n$schema:\n{schema}---\nbody\n").into();
    DarkmatterSchemas::new(crate::request_support::cwd_context())
        .effective_for(&source)
        .expect("schema loads")
        .expect("effective schema")
        .simplified
}

/// `expression` typed at an unguarded site of a document whose projection
/// is `schema` and whose host declares `host_globals`.
fn project_hole(expression: &str, schema: Option<&SchemaRoot>, host_globals: &[&str]) -> HoleType {
    let declared = DeclaredTypes::new(schema).with_host_globals(host_globals);
    darkmatter::markdown::compose::shell_expansion::template::project_hole(expression, &TypeScope::unguarded(&declared))
}

/// The shape of `command` typed against `schema`.
fn typed_shape(command: &str, schema: Option<&SchemaRoot>) -> TemplateShape {
    let mut template = parse_authored(command).unwrap_or_else(|error| panic!("{command}: {error}"));
    template.project_types(&TypeScope::unguarded(&DeclaredTypes::new(schema)));
    TemplateShape::from_template(&template)
}

fn rule(text: &str) -> TemplateShape {
    TemplateShape::parse_rule(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn ruleset(rules: &[&str]) -> ShellRuleSet {
    ShellRuleSet {
        entries: rules.iter().map(|text| ShellRuleEntry::Template(rule(text))).collect(),
    }
}

fn filled(segments: &[(ChainOperator, &str, &[&str])]) -> FilledChain {
    FilledChain {
        segments: segments
            .iter()
            .map(|(operator, program, args)| FilledSegment {
                operator: *operator,
                program: program.to_string(),
                args: args.iter().map(|arg| arg.to_string()).collect(),
                redirection: Default::default(),
            })
            .collect(),
    }
}

// ── AC5: every row of the covers table ───────────────────────────────────

#[test]
fn the_covers_table_holds_for_a_declared_placeholder() {
    let narrow = rule("build {{ alphanumeric(allow-slashes) }}");
    let broad = rule("build {{ string }}");
    for (declared, covered) in [
        (Some("alphanumeric(allow-slashes)"), true),
        (Some("alphanumeric"), true),
        (Some("alphanumeric(allow-slashes; min(3))"), true),
        (Some("alphanumeric(allow-slashes, block-leading-slash)"), true),
        (Some("alphanumeric(allow-slashes, allow-periods)"), false),
        (Some("string"), false),
        (None, false),
        (Some("email"), false),
    ] {
        let projection = match declared {
            Some(ty) => schema(&format!("  dir: \"{ty}\"\n")),
            None => schema("  other: string\n"),
        };
        let candidate = typed_shape("build {{ dir }}", projection.as_ref());
        assert_eq!(narrow.covers(&candidate), covered, "{declared:?}");
        assert!(broad.covers(&candidate), "a `string` rule covers {declared:?}");
    }
}

#[test]
fn a_broad_string_declaration_is_never_covered_by_a_narrower_rule() {
    let numeric = schema("  dir: \"number\"\n");
    let candidate = typed_shape("build {{ dir }}", numeric.as_ref());
    assert!(!rule("build {{ alphanumeric(allow-slashes) }}").covers(&candidate));
    assert!(rule("build {{ string }}").covers(&candidate));
}

#[test]
fn unsupported_rule_types_never_load_even_when_identical_to_the_declaration() {
    for ty in ["number", "boolean", "unknown", "string[]", "alphanumeric[]", "[string, number]"] {
        assert!(TemplateShape::parse_rule(&format!("build {{{{ {ty} }}}}")).is_err(), "{ty}");
    }
}

// ── AC17: schema provenance ──────────────────────────────────────────────

#[test]
fn a_bare_property_reads_its_resolved_named_type() {
    let projection = schema("  slug: \"alphanumeric(allow-dashes)\"\n  pkg: slug\n");
    let hole = project_hole("pkg", projection.as_ref(), &[]);
    assert_eq!(hole.approval.text(), "alphanumeric(allow-dashes)");
    assert_eq!(hole.provenance, TypeProvenance::BareProperty { name: "pkg".into() });
    let candidate = typed_shape("just test --package {{ pkg }}", projection.as_ref());
    assert!(rule("just test --package {{ alphanumeric(allow-dashes) }}").covers(&candidate));
    assert!(!rule("just test --package {{ alphanumeric }}").covers(&candidate));
}

#[test]
fn only_a_bare_top_level_property_reads_a_declaration() {
    let projection = schema("  dir: \"alphanumeric(allow-slashes)\"\n");
    let projected = |expression: &str| project_hole(expression, projection.as_ref(), &["loop", "outputs"]);
    assert_eq!(projected("fm.dir"), projected("dir"));
    assert!(matches!(projected("dir").approval, ApprovalType::StringLike(_)));
    for (expression, provenance) in [
        ("dir + '/x'", TypeProvenance::NotBareProperty),
        ("last(outputs)", TypeProvenance::NotBareProperty),
        ("missing", TypeProvenance::Undeclared),
        ("ctx.today", TypeProvenance::ReservedRoot),
        ("env.HOME", TypeProvenance::ReservedRoot),
        ("loop.count", TypeProvenance::ReservedRoot),
        ("outputs", TypeProvenance::ReservedRoot),
    ] {
        let hole = projected(expression);
        assert_eq!(hole.provenance, provenance, "{expression}");
        assert_eq!(hole.approval, ApprovalType::BroadString, "{expression}");
        assert_eq!(hole.validator, HoleValidator::FallbackScalar, "{expression}");
    }
    let untyped = project_hole("dir", None, &[]);
    assert_eq!(untyped.provenance, TypeProvenance::NoProjection);
    assert_eq!(untyped.approval, ApprovalType::BroadString);
}

#[test]
fn a_local_child_schema_types_its_own_placeholders() {
    let parent = schema("  dir: \"alphanumeric(allow-slashes)\"\n");
    let child = schema("  dir: string\n");
    let narrow = rule("build {{ alphanumeric(allow-slashes) }}");
    assert!(narrow.covers(&typed_shape("build {{ dir }}", parent.as_ref())));
    assert!(!narrow.covers(&typed_shape("build {{ dir }}", child.as_ref())));
}

#[test]
fn numeric_and_union_declarations_disclose_a_string_hole_and_keep_their_validator() {
    let projection = schema("  count: \"number(min(1))\"\n  either: [alphanumeric, number]\n");
    for (name, validated_as) in [("count", "number(min(1))"), ("either", "[alphanumeric, number]")] {
        let hole = project_hole(name, projection.as_ref(), &[]);
        assert_eq!(hole.approval, ApprovalType::BroadString, "{name}");
        assert!(matches!(hole.validator, HoleValidator::Declared { .. }), "{name}");
        assert_eq!(hole.display_type(), format!("string (validated as {validated_as})"));
    }
    let shape = typed_shape("seq {{ count }}", projection.as_ref());
    assert_eq!(shape.canonical_text(), "seq {{ string }}");
    let count = project_hole("count", projection.as_ref(), &[]);
    assert!(count.validator.accepts(&json!(2)));
    assert!(!count.validator.accepts(&json!(0)), "the declared min(1) still applies");
    assert!(!count.validator.accepts(&json!("2")), "a string is not converted to a number");
}

#[test]
fn values_are_validated_as_declared_without_conversion() {
    let projection = schema("  dir: \"alphanumeric(allow-slashes; min(3))\"\n");
    let hole = project_hole("dir", projection.as_ref(), &[]);
    assert_eq!(hole.display_type(), "alphanumeric(allow-slashes)");
    assert!(hole.validator.accepts(&json!("src/app")));
    assert!(!hole.validator.accepts(&json!(12345)), "a number cannot satisfy alphanumeric by conversion");
    assert!(!hole.validator.accepts(&json!("ab")), "min(3) is enforced");
    assert!(!hole.validator.accepts(&json!("a b c")));

    let fallback = project_hole("dir + 'x'", projection.as_ref(), &[]).validator;
    for scalar in [json!("x"), json!(""), json!(0), json!(false)] {
        assert!(fallback.accepts(&scalar), "{scalar}");
    }
    for rejected in [json!(null), json!([1]), json!({"a": 1})] {
        assert!(!fallback.accepts(&rejected), "{rejected}");
    }
}

// ── AC3: rule round trip through the user configuration ─────────────────

/// The user layer's `shell-commands` allows of the configuration whose home
/// is `home`.
fn user_allows(home: &Path) -> Vec<ShellRuleEntry> {
    let snapshot = crate::request_support::shell_config(home, &[]);
    snapshot.policy().expect("valid permissions").user().shell_allows().entries.clone()
}

/// The rules of `entries` (`shell-commands` text, `!` included) as the
/// configuration loader reads them, or the loader's refusal.
fn load_entries(home: &Path, entries: &[String]) -> Result<ShellRuleSet, ConfigError> {
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    let snapshot = crate::request_support::shell_config(home, &entries);
    snapshot.policy().map(|policy| {
        let user = policy.user();
        let mut rules = user.shell_allows().clone();
        rules.entries.extend(user.shell_exclusions().entries.iter().cloned());
        rules
    })
}

#[test]
fn exact_prefix_and_template_rules_survive_save_read_save_read() {
    let dir = TempDir::new().expect("tempdir");
    let shape = typed_shape("just clean --out {{ out }}/build", None);
    let rules = [
        ShellRuleEntry::Exact("echo hello".into()),
        ShellRuleEntry::Prefix("ls".into()),
        ShellRuleEntry::Template(shape.clone()),
    ];
    let save = || {
        for rule in &rules {
            let snapshot = crate::request_support::shell_config(dir.path(), &[]);
            let policy = snapshot.policy().expect("valid permissions");
            save_user_approval(policy, &PersistentApproval::ShellRule { rule: rule.clone(), command: None })
                .expect("saved");
        }
    };
    save();
    let first = user_allows(dir.path());
    save();
    let second = user_allows(dir.path());
    assert_eq!(first, rules);
    assert_eq!(second, rules);
    let text = std::fs::read_to_string(dir.path().join(".darkmatter.yaml")).expect("configuration");
    assert_eq!(
        text.lines().filter(|line| line.contains("template ")).map(str::trim).collect::<Vec<_>>(),
        ["- template just clean --out {{ string }}/build"],
        "the template is written once and holds its type, never a value"
    );
}

#[test]
fn a_constraint_or_program_word_hole_entry_invalidates_the_permissions() {
    let dir = TempDir::new().expect("tempdir");
    for invalid in [
        "template build {{ alphanumeric(allow-slashes; min(3)) }}",
        "template build {{ alphanumeric(min(3)) }}",
        "template {{ string }} run",
        "template build {{ alphanumeric(allow-everything) }}",
        "template build {{ slug }}",
    ] {
        let entries = ["exact echo hello".to_string(), invalid.to_string(), "prefix ls".to_string()];
        let error = load_entries(dir.path(), &entries).expect_err(invalid);
        assert!(error.to_string().contains("shell-commands"), "{invalid}: {error}");
    }
    let valid = load_entries(dir.path(), &["exact echo hello".to_string(), "prefix ls".to_string()]).expect("valid");
    assert_eq!(valid.entries, [ShellRuleEntry::Exact("echo hello".into()), ShellRuleEntry::Prefix("ls".into())]);
}

// ── Input robustness matrix: one `shell-commands` entry ─────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Decision {
    /// The control allow approves (or exclusion matches) the target.
    Applies,
    /// The edited entry contributes no decision.
    None,
}

struct Cell {
    name: &'static str,
    /// Rewrites the saved control entry; a newline splits it into entries.
    edit: fn(&str) -> String,
    decision: Decision,
    /// Whether the loader refuses the block: a malformed configured rule is
    /// never skipped, so nothing in the block applies.
    rejected: bool,
}

const SENTINELS: [&str; 2] = ["exact echo sentinel", "prefix sentinel-tool"];

fn payload(line: &str) -> &str {
    line.strip_prefix("template ").expect("control is a template entry")
}

fn cells() -> Vec<Cell> {
    use Decision::{Applies, None as Nothing};
    vec![
        Cell { name: "control", edit: |line| line.to_string(), decision: Applies, rejected: false },
        Cell { name: "keyword absent", edit: |line| payload(line).to_string(), decision: Nothing, rejected: true },
        Cell { name: "payload absent", edit: |_| "template".into(), decision: Nothing, rejected: true },
        Cell { name: "keyword null", edit: |line| format!("null {}", payload(line)), decision: Nothing, rejected: true },
        Cell { name: "payload null", edit: |_| "template null".into(), decision: Nothing, rejected: false },
        Cell { name: "keyword collection", edit: |line| format!("[template] {}", payload(line)), decision: Nothing, rejected: true },
        Cell { name: "payload collection", edit: |_| r#"template {"program": "just"}"#.into(), decision: Nothing, rejected: true },
        Cell { name: "keyword wrong type", edit: |line| format!("123 {}", payload(line)), decision: Nothing, rejected: true },
        Cell {
            name: "one hole type wrong",
            edit: |line| line.replacen("{{ string }}", "{{ 123 }}", 1),
            decision: Nothing,
            rejected: true,
        },
        Cell {
            name: "every hole type wrong",
            edit: |line| line.replace("{{ string }}", "{{ 123 }}").replace("{{ alphanumeric(allow-dashes) }}", "{{ 123 }}"),
            decision: Nothing,
            rejected: true,
        },
        Cell { name: "payload empty", edit: |_| "template ".into(), decision: Nothing, rejected: true },
        Cell { name: "duplicate entry", edit: |line| format!("{line}\n{line}"), decision: Applies, rejected: false },
        Cell { name: "keyword trailing", edit: |line| format!("templatex {}", payload(line)), decision: Nothing, rejected: true },
        Cell { name: "payload valid extra argument", edit: |line| format!("{line} extra"), decision: Nothing, rejected: false },
        Cell { name: "payload invalid trailing", edit: |line| format!("{line} \"open"), decision: Nothing, rejected: true },
        Cell {
            name: "malformed trailing entry",
            edit: |line| format!("{line}\ntemplate {{{{ string }}}} run"),
            decision: Nothing,
            rejected: true,
        },
    ]
}

/// Walks every cell through the configuration loader. `prefix` is `""` for
/// an allow and `"!"` for an exclusion; `decides` observes the decision.
fn walk_matrix(prefix: &str, decides: impl Fn(&ShellRuleSet) -> bool) {
    let dir = TempDir::new().expect("tempdir");
    let profile = schema("  profile: \"alphanumeric(allow-dashes)\"\n");
    let control = typed_shape("just clean --out {{ out }}/build {{ profile }}", profile.as_ref());
    let control_line = format!("template {}", control.canonical_text());
    assert_eq!(control_line, "template just clean --out {{ string }}/build {{ alphanumeric(allow-dashes) }}");

    for cell in cells() {
        let mut entries: Vec<String> = SENTINELS.iter().map(|sentinel| format!("{prefix}{sentinel}")).collect();
        entries.extend((cell.edit)(&control_line).lines().map(|line| format!("{prefix}{line}")));
        match load_entries(dir.path(), &entries) {
            Err(error) => {
                assert!(cell.rejected, "{}: {error}", cell.name);
                assert!(error.to_string().contains("shell-commands"), "{}: {error}", cell.name);
            }
            Ok(rules) => {
                assert!(!cell.rejected, "{}: a malformed entry was accepted", cell.name);
                let decision = if decides(&rules) { Decision::Applies } else { Decision::None };
                assert_eq!(decision, cell.decision, "{}", cell.name);
                assert!(rules.matches_exact("echo sentinel"), "{}: exact sentinel lost", cell.name);
                assert!(rules.matches_prefix("sentinel-tool"), "{}: prefix sentinel lost", cell.name);
            }
        }
    }
}

#[test]
fn the_allow_robustness_matrix() {
    let profile = schema("  profile: \"alphanumeric(allow-dashes)\"\n");
    let target = typed_shape("just clean --out {{ out }}/build {{ profile }}", profile.as_ref());
    walk_matrix("", |rules| rules.template_covers(&target));
}

#[test]
fn the_exclusion_robustness_matrix() {
    let profile = schema("  profile: \"alphanumeric(allow-dashes)\"\n");
    let target = typed_shape("just clean --out {{ out }}/build {{ profile }}", profile.as_ref());
    let filled = filled(&[(ChainOperator::None, "just", &["clean", "--out", "dist/build", "release"])]);
    walk_matrix("!", |rules| {
        let preflight = check_template_exclusion(&target, rules);
        let runtime = rules.blocks_filled(&filled);
        assert_eq!(preflight, runtime, "preflight and runtime exclusions disagree");
        preflight
    });
}

// ── AC4: argument-list matching ──────────────────────────────────────────

#[test]
fn whitelist_templates_match_on_the_argument_list() {
    let projection = schema("  dir: \"alphanumeric(allow-slashes)\"\n");
    let shape = |command: &str| typed_shape(command, projection.as_ref());
    let rule = rule("tool --out {{ alphanumeric(allow-slashes) }}/build && done > /dev/null 2>&1");
    for (command, matches) in [
        ("tool --out {{ dir }}/build && done > /dev/null 2>&1", true),
        ("tool   '--out'  \"{{ dir }}/build\" &&   done >/dev/null 2>&1", true),
        ("tool --out {{ dir }}'/build' && \"done\" > /dev/null 2>&1", true),
        ("tool --output {{ dir }}/build && done > /dev/null 2>&1", false),
        ("tool --out {{ dir }}/build extra && done > /dev/null 2>&1", false),
        ("tool {{ dir }} --out/build && done > /dev/null 2>&1", false),
        ("tool --out {{ dir }}/build || done > /dev/null 2>&1", false),
        ("tool --out {{ dir }}/build && done 2>&1 > /dev/null", false),
        ("tool --out {{ dir }}/build && done", false),
        ("tool --out {{ dir }}{{ dir }}/build && done > /dev/null 2>&1", false),
    ] {
        assert_eq!(rule.covers(&shape(command)), matches, "{command}");
    }
}

#[test]
fn an_exclusion_template_matches_on_shape_whatever_the_hole_type() {
    let projection = schema("  dir: \"alphanumeric\"\n");
    let candidate = typed_shape("deploy {{ dir }} --now", projection.as_ref());
    let exclusions = ruleset(&["deploy {{ string }} --now"]);
    assert!(check_template_exclusion(&candidate, &exclusions));
    assert!(ruleset(&["deploy {{ email }} --now"]).template_blocks_shape(&candidate));
    // A single-segment rule matches any segment of a chain.
    let chained = typed_shape("echo start && deploy {{ dir }} --now", projection.as_ref());
    assert!(check_template_exclusion(&chained, &exclusions));
    assert!(!check_template_exclusion(&typed_shape("deploy {{ dir }}", projection.as_ref()), &exclusions));
}

// ── AC13 primitives: runtime policy on filled commands ───────────────────

#[test]
fn a_string_template_cannot_authorize_a_dangerous_filled_value() {
    let candidate = typed_shape("git {{ action }}", None);
    assert!(ruleset(&["git {{ string }}"]).template_covers(&candidate));
    assert!(check_template_blacklist(&candidate).is_none());
    let reset = filled(&[(ChainOperator::None, "git", &["reset"])]);
    let reason = check_filled_chain(&reset).expect("git reset is refused");
    assert!(reason.contains("git reset"), "{reason}");
    assert!(check_filled_chain(&filled(&[(ChainOperator::None, "git", &["status"])])).is_none());
}

#[test]
fn preflight_keeps_hole_positions_when_checking_known_words() {
    // `reset` is the second argument, not the subcommand.
    assert!(check_template_blacklist(&typed_shape("git {{ x }} reset", None)).is_none());
    assert!(check_template_blacklist(&typed_shape("git reset {{ x }}", None)).is_some());
    assert!(check_template_blacklist(&typed_shape("psql -c DROP{{ x }}", None)).is_some());
    assert!(check_template_blacklist(&typed_shape("rm {{ x }}", None)).is_some());
}

#[test]
fn exclusions_match_filled_commands_with_and_without_authored_holes() {
    let exact = ShellRuleSet { entries: vec![ShellRuleEntry::Exact("git status".into())] };
    let status = filled(&[(ChainOperator::None, "git", &["status"])]);
    assert!(exact.blocks_filled(&status), "an exact exclusion matches a filled template");

    let template = ruleset(&["deploy {{ string }}"]);
    let prod = filled(&[(ChainOperator::None, "deploy", &["prod"])]);
    assert!(template.blocks_filled(&prod));
    // The resolved-byte path: a command with no authored holes.
    let args = vec!["prod".to_string()];
    assert!(check_user_blacklist(&template, "deploy", &args, &normalize_command("deploy", &args)));
    assert!(!check_user_blacklist(&template, "deploy", &[], "deploy"));

    let partial = ruleset(&["tool pre-{{ string }}-post"]);
    for (value, matched) in [("pre--post", true), ("pre-x-post", true), ("pre-x", false), ("x-post", false)] {
        let chain = filled(&[(ChainOperator::None, "tool", &[value])]);
        assert_eq!(partial.blocks_filled(&chain), matched, "{value}");
    }
}

#[test]
fn a_filled_value_an_exclusion_matches_needs_approval_unless_the_shape_was_excluded() {
    let shape = typed_shape("git add {{ path }}", None);
    let secrets = filled(&[(ChainOperator::None, "git", &["add", "secrets"])]);
    let readme = filled(&[(ChainOperator::None, "git", &["add", "README.md"])]);
    let values = ShellRuleSet { entries: vec![ShellRuleEntry::Exact("git add secrets".into())] };
    assert!(filled_template_needs_approval(&secrets, &shape, &values));
    assert!(!filled_template_needs_approval(&readme, &shape, &values));
    // A shape the exclusion already matched was approved explicitly.
    let program = ShellRuleSet { entries: vec![ShellRuleEntry::Prefix("git".into())] };
    assert!(!filled_template_needs_approval(&secrets, &shape, &program));
}

#[test]
fn a_refused_later_segment_refuses_the_whole_chain() {
    let chain = filled(&[(ChainOperator::None, "echo", &["start"]), (ChainOperator::Or, "git", &["reset", "--hard"])]);
    assert!(check_filled_chain(&chain).is_some());
    let user = ruleset(&["deploy {{ string }}"]);
    let chain = filled(&[(ChainOperator::None, "echo", &["start"]), (ChainOperator::And, "deploy", &["prod"])]);
    assert!(user.blocks_filled(&chain));
}
