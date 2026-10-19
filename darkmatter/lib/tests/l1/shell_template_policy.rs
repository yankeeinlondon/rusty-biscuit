//! Typed shell-template policy primitives: placeholder types and the covers
//! relation, the `template` rule store and its robustness matrix, and
//! whitelist/blacklist matching (acceptance criteria 3, 4, 5, 13, and 17 of
//! `2026-10-23-shell-template-approval`, at the public library boundary).

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::store::{
    append_blacklist_template, append_whitelist_exact, append_whitelist_prefix, append_whitelist_template,
    load_ruleset, load_ruleset_with_warnings,
};
use darkmatter::markdown::compose::shell_expansion::template::{
    ApprovalType, FilledChain, FilledSegment, HoleValidator, TemplateShape, TypeProvenance, parse_authored,
    project_hole,
};
use darkmatter::markdown::compose::shell_expansion::types::{
    ChainOperator, ShellExpansionError, ShellPolicyPaths, ShellRuleEntry, ShellRuleSet,
};
use darkmatter::markdown::compose::shell_expansion::{
    check_filled_chain, check_template_blacklist, check_user_blacklist, normalize_command,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, SimplifiedSchema};
use serde_json::json;
use std::path::Path;
use tempfile::TempDir;

/// The `SimplifiedSchema` projection of a document whose `$schema` body is
/// `schema` (YAML lines indented two spaces).
fn schema(schema: &str) -> Option<SimplifiedSchema> {
    let source: Markdown = format!("---\n$schema:\n{schema}---\nbody\n").into();
    DarkmatterSchemas::new(crate::request_support::cwd_context())
        .effective_for(&source)
        .expect("schema loads")
        .expect("effective schema")
        .simplified
}

/// The shape of `command` typed against `schema`.
fn typed_shape(command: &str, schema: Option<&SimplifiedSchema>) -> TemplateShape {
    let mut template = parse_authored(command).unwrap_or_else(|error| panic!("{command}: {error}"));
    template.project_types(schema, &[]);
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
        assert!(matches!(hole.validator, HoleValidator::Declared(_)), "{name}");
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

// ── AC3: store round trip ────────────────────────────────────────────────

fn policy_paths(dir: &Path) -> ShellPolicyPaths {
    ShellPolicyPaths {
        whitelist: dir.join(".darkmatter-shell-whitelist"),
        blacklist: dir.join(".darkmatter-shell-blacklist"),
    }
}

#[test]
fn exact_prefix_and_template_rules_survive_write_read_write_read() {
    let dir = TempDir::new().expect("tempdir");
    let paths = policy_paths(dir.path());
    let shape = typed_shape("just clean --out {{ out }}/build", None);
    let write = || {
        append_whitelist_exact(&paths, "echo hello").expect("exact");
        append_whitelist_prefix(&paths, "ls").expect("prefix");
        append_whitelist_template(&paths, &shape).expect("template");
    };
    write();
    let first = load_ruleset(&paths.whitelist).expect("first read");
    write();
    let second = load_ruleset(&paths.whitelist).expect("second read");
    let expected = vec![
        ShellRuleEntry::Exact("echo hello".into()),
        ShellRuleEntry::Prefix("ls".into()),
        ShellRuleEntry::Template(shape.clone()),
    ];
    assert_eq!(first.entries, expected);
    assert_eq!(second.entries, expected);
    let text = std::fs::read_to_string(&paths.whitelist).expect("whitelist");
    assert_eq!(
        text.lines().filter(|line| line.starts_with("template ")).collect::<Vec<_>>(),
        ["template just clean --out {{ string }}/build"],
        "the template is written once and holds its type, never a value"
    );
}

#[test]
fn a_constraint_or_program_word_hole_line_is_skipped_with_its_file_and_line() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(
        &path,
        "exact echo hello\n\
         template build {{ alphanumeric(allow-slashes; min(3)) }}\n\
         template build {{ alphanumeric(min(3)) }}\n\
         template {{ string }} run\n\
         template build {{ alphanumeric(allow-everything) }}\n\
         template build {{ slug }}\n\
         prefix ls\n",
    )
    .expect("write");
    let (rules, warnings) = load_ruleset_with_warnings(&path).expect("loads");
    assert_eq!(
        rules.entries,
        [ShellRuleEntry::Exact("echo hello".into()), ShellRuleEntry::Prefix("ls".into())]
    );
    assert_eq!(warnings.iter().map(|warning| warning.line).collect::<Vec<_>>(), [2, 3, 4, 5, 6]);
    for warning in &warnings {
        assert_eq!(warning.path, path);
        let rendered = warning.to_string();
        assert!(rendered.contains(&format!("{}:{}", path.display(), warning.line)), "{rendered}");
    }
}

#[test]
fn unreadable_and_non_utf8_policy_files_are_policy_io_failures() {
    let dir = TempDir::new().expect("tempdir");
    let binary = dir.path().join("binary");
    std::fs::write(&binary, [b't', 0xff, 0xfe, b'\n']).expect("write");
    assert!(matches!(load_ruleset(&binary), Err(ShellExpansionError::PolicyIo { .. })));
    assert!(matches!(load_ruleset(dir.path()), Err(ShellExpansionError::PolicyIo { .. })));
    assert!(load_ruleset(&dir.path().join("missing")).expect("missing").entries.is_empty());
}

// ── Input robustness matrix: the line-based policy format ────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Decision {
    /// The control whitelist approves (or blacklist denies) the target.
    Applies,
    /// The edited line contributes no decision.
    None,
}

struct Cell {
    name: &'static str,
    /// Rewrites the persisted control line.
    edit: fn(&str) -> String,
    decision: Decision,
    /// Whether the loader skips the edited line with a warning.
    warns: bool,
}

const SENTINELS: &str = "exact echo sentinel\nprefix sentinel-tool\n";

fn payload(line: &str) -> &str {
    line.strip_prefix("template ").expect("control is a template line")
}

fn cells() -> Vec<Cell> {
    use Decision::{Applies, None as Nothing};
    vec![
        Cell { name: "control", edit: |line| line.to_string(), decision: Applies, warns: false },
        Cell { name: "keyword absent", edit: |line| payload(line).to_string(), decision: Nothing, warns: true },
        Cell { name: "payload absent", edit: |_| "template".into(), decision: Nothing, warns: true },
        Cell { name: "keyword null", edit: |line| format!("null {}", payload(line)), decision: Nothing, warns: true },
        Cell { name: "payload null", edit: |_| "template null".into(), decision: Nothing, warns: false },
        Cell { name: "keyword collection", edit: |line| format!("[template] {}", payload(line)), decision: Nothing, warns: true },
        Cell { name: "payload collection", edit: |_| r#"template {"program": "just"}"#.into(), decision: Nothing, warns: true },
        Cell { name: "keyword wrong type", edit: |line| format!("123 {}", payload(line)), decision: Nothing, warns: true },
        Cell {
            name: "one hole type wrong",
            edit: |line| line.replacen("{{ string }}", "{{ 123 }}", 1),
            decision: Nothing,
            warns: true,
        },
        Cell {
            name: "every hole type wrong",
            edit: |line| line.replace("{{ string }}", "{{ 123 }}").replace("{{ alphanumeric(allow-dashes) }}", "{{ 123 }}"),
            decision: Nothing,
            warns: true,
        },
        Cell { name: "keyword empty", edit: |line| format!(" {}", payload(line)), decision: Nothing, warns: true },
        Cell { name: "payload empty", edit: |_| "template ".into(), decision: Nothing, warns: true },
        Cell { name: "duplicate line", edit: |line| format!("{line}\n{line}"), decision: Applies, warns: false },
        Cell { name: "keyword trailing", edit: |line| format!("templatex {}", payload(line)), decision: Nothing, warns: true },
        Cell { name: "payload valid extra argument", edit: |line| format!("{line} extra"), decision: Nothing, warns: false },
        Cell { name: "payload invalid trailing", edit: |line| format!("{line} \"open"), decision: Nothing, warns: true },
        Cell {
            name: "malformed trailing line",
            edit: |line| format!("{line}\ntemplate {{{{ string }}}} run"),
            decision: Applies,
            warns: true,
        },
    ]
}

/// Walks every cell for one policy file. `persist` writes the control rule
/// through the real persistence API; `decides` observes the public decision.
fn walk_matrix(
    persist: fn(&ShellPolicyPaths, &TemplateShape) -> Result<(), ShellExpansionError>,
    path_of: fn(&ShellPolicyPaths) -> &Path,
    decides: impl Fn(&ShellRuleSet) -> bool,
) {
    let dir = TempDir::new().expect("tempdir");
    let paths = policy_paths(dir.path());
    let profile = schema("  profile: \"alphanumeric(allow-dashes)\"\n");
    let control = typed_shape("just clean --out {{ out }}/build {{ profile }}", profile.as_ref());
    persist(&paths, &control).expect("persist the control rule");
    let path = path_of(&paths);
    let persisted = std::fs::read_to_string(path).expect("persisted file");
    let control_line = persisted.lines().next().expect("one line").to_string();
    assert_eq!(control_line, "template just clean --out {{ string }}/build {{ alphanumeric(allow-dashes) }}");

    for cell in cells() {
        std::fs::write(path, format!("{SENTINELS}{}\n", (cell.edit)(&control_line))).expect("write cell");
        let (rules, warnings) = load_ruleset_with_warnings(path).expect(cell.name);
        let decision = if decides(&rules) { Decision::Applies } else { Decision::None };
        assert_eq!(decision, cell.decision, "{}", cell.name);
        assert_eq!(!warnings.is_empty(), cell.warns, "{}: {warnings:?}", cell.name);
        for warning in &warnings {
            assert_eq!(warning.path, path, "{}", cell.name);
            assert!(warning.line > 2, "{}: a sentinel line was reported", cell.name);
        }
        assert!(rules.matches_exact("echo sentinel"), "{}: exact sentinel lost", cell.name);
        assert!(rules.matches_prefix("sentinel-tool"), "{}: prefix sentinel lost", cell.name);
        let templates = rules.entries.iter().filter(|entry| matches!(entry, ShellRuleEntry::Template(_))).count();
        assert!(templates <= 1, "{}: duplicate rules load once", cell.name);
    }

    std::fs::write(path, "").expect("blank document");
    let (rules, warnings) = load_ruleset_with_warnings(path).expect("blank");
    assert!(rules.entries.is_empty() && warnings.is_empty());
    assert!(!decides(&rules));
}

#[test]
fn the_whitelist_robustness_matrix() {
    let profile = schema("  profile: \"alphanumeric(allow-dashes)\"\n");
    let target = typed_shape("just clean --out {{ out }}/build {{ profile }}", profile.as_ref());
    walk_matrix(append_whitelist_template, |paths| &paths.whitelist, |rules| rules.template_covers(&target));
}

#[test]
fn the_blacklist_robustness_matrix() {
    let profile = schema("  profile: \"alphanumeric(allow-dashes)\"\n");
    let target = typed_shape("just clean --out {{ out }}/build {{ profile }}", profile.as_ref());
    let filled = filled(&[(ChainOperator::None, "just", &["clean", "--out", "dist/build", "release"])]);
    walk_matrix(append_blacklist_template, |paths| &paths.blacklist, |rules| {
        let preflight = check_template_blacklist(&target, rules).is_some();
        let runtime = check_filled_chain(&filled, rules).is_some();
        assert_eq!(preflight, runtime, "preflight and runtime blacklists disagree");
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
fn a_blacklist_template_matches_on_shape_whatever_the_hole_type() {
    let projection = schema("  dir: \"alphanumeric\"\n");
    let candidate = typed_shape("deploy {{ dir }} --now", projection.as_ref());
    let blacklist = ruleset(&["deploy {{ string }} --now"]);
    assert!(check_template_blacklist(&candidate, &blacklist).is_some());
    assert!(ruleset(&["deploy {{ email }} --now"]).template_blocks_shape(&candidate));
    // A single-segment rule matches any segment of a chain.
    let chained = typed_shape("echo start && deploy {{ dir }} --now", projection.as_ref());
    assert!(check_template_blacklist(&chained, &blacklist).is_some());
    assert!(check_template_blacklist(&typed_shape("deploy {{ dir }}", projection.as_ref()), &blacklist).is_none());
}

// ── AC13 primitives: runtime policy on filled commands ───────────────────

#[test]
fn a_string_template_cannot_authorize_a_dangerous_filled_value() {
    let candidate = typed_shape("git {{ action }}", None);
    assert!(ruleset(&["git {{ string }}"]).template_covers(&candidate));
    assert!(check_template_blacklist(&candidate, &ShellRuleSet::default()).is_none());
    let reset = filled(&[(ChainOperator::None, "git", &["reset"])]);
    let reason = check_filled_chain(&reset, &ShellRuleSet::default()).expect("git reset is refused");
    assert!(reason.contains("git reset"), "{reason}");
    assert!(check_filled_chain(&filled(&[(ChainOperator::None, "git", &["status"])]), &ShellRuleSet::default()).is_none());
}

#[test]
fn preflight_keeps_hole_positions_when_checking_known_words() {
    let none = ShellRuleSet::default();
    // `reset` is the second argument, not the subcommand.
    assert!(check_template_blacklist(&typed_shape("git {{ x }} reset", None), &none).is_none());
    assert!(check_template_blacklist(&typed_shape("git reset {{ x }}", None), &none).is_some());
    assert!(check_template_blacklist(&typed_shape("psql -c DROP{{ x }}", None), &none).is_some());
    assert!(check_template_blacklist(&typed_shape("rm {{ x }}", None), &none).is_some());
}

#[test]
fn user_blacklists_block_filled_commands_with_and_without_authored_holes() {
    let exact = ShellRuleSet { entries: vec![ShellRuleEntry::Exact("git status".into())] };
    let status = filled(&[(ChainOperator::None, "git", &["status"])]);
    assert!(check_filled_chain(&status, &exact).is_some(), "an exact blacklist blocks a filled template");

    let template = ruleset(&["deploy {{ string }}"]);
    let prod = filled(&[(ChainOperator::None, "deploy", &["prod"])]);
    assert!(check_filled_chain(&prod, &template).is_some());
    // The resolved-byte path: a command with no authored holes.
    let args = vec!["prod".to_string()];
    assert!(check_user_blacklist(&template, "deploy", &args, &normalize_command("deploy", &args)));
    assert!(!check_user_blacklist(&template, "deploy", &[], "deploy"));

    let partial = ruleset(&["tool pre-{{ string }}-post"]);
    for (value, blocked) in [("pre--post", true), ("pre-x-post", true), ("pre-x", false), ("x-post", false)] {
        let chain = filled(&[(ChainOperator::None, "tool", &[value])]);
        assert_eq!(check_filled_chain(&chain, &partial).is_some(), blocked, "{value}");
    }
}

#[test]
fn a_blocked_later_segment_blocks_the_whole_chain() {
    let chain = filled(&[(ChainOperator::None, "echo", &["start"]), (ChainOperator::Or, "git", &["reset", "--hard"])]);
    assert!(check_filled_chain(&chain, &ShellRuleSet::default()).is_some());
    let user = ruleset(&["deploy {{ string }}"]);
    let chain = filled(&[(ChainOperator::None, "echo", &["start"]), (ChainOperator::And, "deploy", &["prod"])]);
    assert!(check_filled_chain(&chain, &user).is_some());
}
