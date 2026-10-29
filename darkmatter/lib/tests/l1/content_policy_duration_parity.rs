//! Lexical parity between Content Policy's rule parser and the editor
//! patterns it ships in `content_policy::EDITOR_SCHEMA`.
//!
//! The parser is the authority; the editor patterns repeat its spellings as
//! regexes so DMLS can flag a malformed rule while the author types. They are
//! maintained by hand, so one [`ROSTER`] states, for every rule and every
//! duration unit in each `ValidFor` baseline form, what the parser decides and
//! what the patterns decide, and three consumers are held to it: the parser
//! (`Policy::from_declaration`), the pattern fragments compiled on their own,
//! and the shipped schema validated through Darkmatter's embedded baseline.
//!
//! Rows marked [`Expect::LexicalOnly`] are intentional differences: the
//! spelling is well formed, so the patterns accept it, and only the parser's
//! semantic check rejects it. A row where the patterns reject what the parser
//! accepts would be a defect in the patterns, and no such row is allowed.

use std::collections::{BTreeMap, BTreeSet};

use content_policy::{Baseline, Deadline, DurationUnit, Policy, Rule};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde_json::Value;

use crate::request_support::context_at;

/// Every duration unit the parser knows, with its written suffix.
const UNITS: [(DurationUnit, &str); 4] = [
    (DurationUnit::Days, "d"),
    (DurationUnit::Weeks, "wk"),
    (DurationUnit::Months, "mo"),
    (DurationUnit::Years, "yr"),
];

/// Fails to compile when Content Policy adds a unit, so [`UNITS`] (and the
/// roster built on it) cannot silently miss one.
fn unit_is_rostered(unit: DurationUnit) {
    match unit {
        DurationUnit::Days | DurationUnit::Weeks | DurationUnit::Months | DurationUnit::Years => {}
    }
}

/// One spelling family of a compact rule; each editor pattern fragment
/// covers exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Form {
    Evergreen,
    TimeSensitive,
    /// `ValidFor(3mo)`: the caller's default date property is the baseline.
    ValidForDefault,
    /// `ValidFor(3mo, @reviewed)`.
    ValidForProperty,
    /// `ValidFor(3mo, 2026-09-28)`.
    ValidForInline,
    ValidUntilDate,
    ValidUntilProperty,
    FileChanged,
}

impl Form {
    const ALL: [Self; 8] = [
        Self::Evergreen,
        Self::TimeSensitive,
        Self::ValidForDefault,
        Self::ValidForProperty,
        Self::ValidForInline,
        Self::ValidUntilDate,
        Self::ValidUntilProperty,
        Self::FileChanged,
    ];

    fn takes_duration(self) -> bool {
        matches!(self, Self::ValidForDefault | Self::ValidForProperty | Self::ValidForInline)
    }

    /// The form a parsed rule was written in. Exhaustive over [`Rule`], so a
    /// new rule fails to compile until it has a form here.
    fn of(rule: &Rule) -> Self {
        match rule {
            Rule::Evergreen => Self::Evergreen,
            Rule::TimeSensitive => Self::TimeSensitive,
            Rule::ValidFor { baseline: Baseline::Defaulted, .. } => Self::ValidForDefault,
            Rule::ValidFor { baseline: Baseline::Reference(_), .. } => Self::ValidForProperty,
            Rule::ValidFor { baseline: Baseline::Inline(_), .. } => Self::ValidForInline,
            Rule::ValidUntil { deadline: Deadline::Inline(_) } => Self::ValidUntilDate,
            Rule::ValidUntil { deadline: Deadline::Reference(_) } => Self::ValidUntilProperty,
            Rule::FileChanged { .. } => Self::FileChanged,
        }
    }
}

/// The editor schema's `short_form` fragments, in declaration order, by the
/// description each carries after `->`.
const FRAGMENTS: [(&str, Form); 8] = [
    ("Never triggers", Form::Evergreen),
    ("Always triggers", Form::TimeSensitive),
    ("Interval from the default date property", Form::ValidForDefault),
    ("Interval from a property", Form::ValidForProperty),
    ("Interval from an inline date", Form::ValidForInline),
    ("Fixed deadline", Form::ValidUntilDate),
    ("Deadline held in a property", Form::ValidUntilProperty),
    ("Content change of a file, fingerprint held in a property", Form::FileChanged),
];

/// The parser check a lexically well-formed spelling can still fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Semantic {
    /// The count does not fit the parser's `u32`.
    CountRange,
    /// The date has the `YYYY-MM-DD` shape but is not a calendar date.
    CalendarDate,
    /// Baseline plus duration falls outside the supported calendar range.
    DateRange,
    /// A `{{` after a path's first character, which a group-free pattern
    /// cannot exclude.
    PathTemplate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// Parser and patterns both accept.
    Accept,
    /// Parser and patterns both reject.
    Reject,
    /// Intentional difference: patterns accept, the parser rejects.
    LexicalOnly(Semantic),
}

impl Expect {
    fn parser_accepts(self) -> bool {
        self == Self::Accept
    }

    fn lexically_accepts(self) -> bool {
        self != Self::Reject
    }
}

/// One roster row. A `{u}` in `text` expands to one row per [`UNITS`] entry.
#[derive(Debug, Clone, Copy)]
struct Row {
    text: &'static str,
    form: Form,
    expect: Expect,
}

const fn row(text: &'static str, form: Form, expect: Expect) -> Row {
    Row { text, form, expect }
}

use Expect::{Accept, LexicalOnly, Reject};
use Form::{
    Evergreen, FileChanged, TimeSensitive, ValidForDefault, ValidForInline, ValidForProperty,
    ValidUntilDate, ValidUntilProperty,
};
use Semantic::{CalendarDate, CountRange, DateRange, PathTemplate};

/// The parity roster. `{u}` rows run for every unit.
const ROSTER: &[Row] = &[
    // ── Evergreen / TimeSensitive ──
    row("Evergreen", Evergreen, Accept),
    row("TimeSensitive", TimeSensitive, Accept),
    row("evergreen", Evergreen, Reject),
    row(" Evergreen", Evergreen, Reject),
    row("Evergreen ", Evergreen, Reject),
    row("Evergreen()", Evergreen, Reject),
    row("Evergreen(3mo)", Evergreen, Reject),
    row("TimeSensitive(@x)", TimeSensitive, Reject),
    row("Time Sensitive", TimeSensitive, Reject),
    // ── ValidFor, default baseline ──
    row("ValidFor(3{u})", ValidForDefault, Accept),
    row("ValidFor(1{u})", ValidForDefault, Accept),
    row("ValidFor(10{u})", ValidForDefault, Accept),
    row("ValidFor(4294967295{u})", ValidForDefault, Accept),
    row("ValidFor( 3{u})", ValidForDefault, Accept),
    row("ValidFor(3{u} )", ValidForDefault, Accept),
    row("ValidFor(   3{u}   )", ValidForDefault, Accept),
    row("ValidFor(\t3{u})", ValidForDefault, Reject),
    row("ValidFor(3{u}\t)", ValidForDefault, Reject),
    row("ValidFor (3{u})", ValidForDefault, Reject),
    row(" ValidFor(3{u})", ValidForDefault, Reject),
    row("ValidFor(3{u}) ", ValidForDefault, Reject),
    row("ValidFor(3 {u})", ValidForDefault, Reject),
    row("ValidFor({u})", ValidForDefault, Reject),
    row("ValidFor(0{u})", ValidForDefault, Reject),
    row("ValidFor(03{u})", ValidForDefault, Reject),
    row("ValidFor(-3{u})", ValidForDefault, Reject),
    row("ValidFor(+3{u})", ValidForDefault, Reject),
    row("ValidFor(1.5{u})", ValidForDefault, Reject),
    row("ValidFor(3{u}3{u})", ValidForDefault, Reject),
    row("ValidFor(3{u} 2{u})", ValidForDefault, Reject),
    row("ValidFor(3{u}x)", ValidForDefault, Reject),
    row("ValidFor(3{u})x", ValidForDefault, Reject),
    row("ValidFor(3{u}))", ValidForDefault, Reject),
    row("ValidFor((3{u}))", ValidForDefault, Reject),
    row("ValidFor(3{u},)", ValidForDefault, Reject),
    row("validfor(3{u})", ValidForDefault, Reject),
    row("ValidFor(4294967296{u})", ValidForDefault, LexicalOnly(CountRange)),
    row("ValidFor(3)", ValidForDefault, Reject),
    row("ValidFor()", ValidForDefault, Reject),
    row("ValidFor", ValidForDefault, Reject),
    row("ValidFor(3w)", ValidForDefault, Reject),
    row("ValidFor(3m)", ValidForDefault, Reject),
    row("ValidFor(3y)", ValidForDefault, Reject),
    row("ValidFor(3h)", ValidForDefault, Reject),
    row("ValidFor(3D)", ValidForDefault, Reject),
    row("ValidFor(3MO)", ValidForDefault, Reject),
    row("ValidFor(3days)", ValidForDefault, Reject),
    row("ValidFor(3mos)", ValidForDefault, Reject),
    row("ValidFor(\u{663}mo)", ValidForDefault, Reject),
    // ── ValidFor, property baseline ──
    row("ValidFor(3{u}, @last_updated)", ValidForProperty, Accept),
    row("ValidFor(3{u},@last_updated)", ValidForProperty, Accept),
    row("ValidFor( 3{u} ,  @reviewed-on )", ValidForProperty, Accept),
    row("ValidFor(3{u}, @_x)", ValidForProperty, Accept),
    row("ValidFor(3{u}, @A9)", ValidForProperty, Accept),
    row("ValidFor(3{u}, @)", ValidForProperty, Reject),
    row("ValidFor(3{u}, @1x)", ValidForProperty, Reject),
    row("ValidFor(3{u}, @-x)", ValidForProperty, Reject),
    row("ValidFor(3{u}, @a.b)", ValidForProperty, Reject),
    row("ValidFor(3{u}, @@x)", ValidForProperty, Reject),
    row("ValidFor(3{u}, @x y)", ValidForProperty, Reject),
    row("ValidFor(3{u}, last_updated)", ValidForProperty, Reject),
    row("ValidFor(3{u}, @x, @y)", ValidForProperty, Reject),
    row("ValidFor(3{u}, @x,)", ValidForProperty, Reject),
    row("ValidFor(3{u},\t@x)", ValidForProperty, Reject),
    row("ValidFor(3{u} @x)", ValidForProperty, Reject),
    row("ValidFor(, @x)", ValidForProperty, Reject),
    row("ValidFor({u}, @x)", ValidForProperty, Reject),
    row("ValidFor(0{u}, @x)", ValidForProperty, Reject),
    row("ValidFor(3w, @x)", ValidForProperty, Reject),
    row("ValidFor(4294967296{u}, @x)", ValidForProperty, LexicalOnly(CountRange)),
    // ── ValidFor, inline date ──
    row("ValidFor(3{u}, 2026-09-28)", ValidForInline, Accept),
    row("ValidFor(3{u},2026-09-28)", ValidForInline, Accept),
    row("ValidFor( 3{u} ,  2026-09-28  )", ValidForInline, Accept),
    row("ValidFor(1{u}, 2028-02-29)", ValidForInline, Accept),
    row("ValidFor(3{u}, 2026-9-28)", ValidForInline, Reject),
    row("ValidFor(3{u}, 2026/09/28)", ValidForInline, Reject),
    row("ValidFor(3{u}, 20260928)", ValidForInline, Reject),
    row("ValidFor(3{u}, 2026-09-28T10:00:00Z)", ValidForInline, Reject),
    row("ValidFor(3{u}, 2026-09-28, @x)", ValidForInline, Reject),
    row("ValidFor(3{u}, 2026-09-28 @x)", ValidForInline, Reject),
    row("ValidFor(0{u}, 2026-09-28)", ValidForInline, Reject),
    row("ValidFor({u}, 2026-09-28)", ValidForInline, Reject),
    row("ValidFor(3w, 2026-09-28)", ValidForInline, Reject),
    row("ValidFor(3{u}, 2026-02-30)", ValidForInline, LexicalOnly(CalendarDate)),
    row("ValidFor(3{u}, 2027-02-29)", ValidForInline, LexicalOnly(CalendarDate)),
    row("ValidFor(3{u}, 2026-13-01)", ValidForInline, LexicalOnly(CalendarDate)),
    row("ValidFor(3{u}, 2026-00-10)", ValidForInline, LexicalOnly(CalendarDate)),
    row("ValidFor(4294967295{u}, 2026-01-01)", ValidForInline, LexicalOnly(DateRange)),
    row("ValidFor(4294967296{u}, 2026-01-01)", ValidForInline, LexicalOnly(CountRange)),
    // ── ValidUntil ──
    row("ValidUntil(2027-01-01)", ValidUntilDate, Accept),
    row("ValidUntil( 2027-01-01 )", ValidUntilDate, Accept),
    row("ValidUntil(2028-02-29)", ValidUntilDate, Accept),
    row("ValidUntil(2027-1-1)", ValidUntilDate, Reject),
    row("ValidUntil(2027-01-01T00:00:00Z)", ValidUntilDate, Reject),
    row("ValidUntil(\t2027-01-01)", ValidUntilDate, Reject),
    row("ValidUntil(2027-01-01, @x)", ValidUntilDate, Reject),
    row("ValidUntil(3mo)", ValidUntilDate, Reject),
    row("ValidUntil()", ValidUntilDate, Reject),
    row("ValidUntil(2026-02-30)", ValidUntilDate, LexicalOnly(CalendarDate)),
    row("ValidUntil(2027-02-29)", ValidUntilDate, LexicalOnly(CalendarDate)),
    row("ValidUntil(@retire_on)", ValidUntilProperty, Accept),
    row("ValidUntil(  @retire-on )", ValidUntilProperty, Accept),
    row("ValidUntil(@a.b)", ValidUntilProperty, Reject),
    row("ValidUntil(@)", ValidUntilProperty, Reject),
    row("ValidUntil(retire_on)", ValidUntilProperty, Reject),
    // ── FileChanged ──
    row("FileChanged(src/config.rs, @config_fingerprint)", FileChanged, Accept),
    row("FileChanged(a,@fp)", FileChanged, Accept),
    row("FileChanged(src/a.rs,   @fp  )", FileChanged, Accept),
    row("FileChanged(src/config.rs)", FileChanged, Reject),
    row("FileChanged(src/config.rs, fp)", FileChanged, Reject),
    row("FileChanged(src/config.rs, @a.b)", FileChanged, Reject),
    row("FileChanged( src/a.rs, @fp)", FileChanged, Reject),
    row("FileChanged(/etc/hosts, @fp)", FileChanged, Reject),
    row("FileChanged(docs/{{HOME}}.md, @fp)", FileChanged, LexicalOnly(PathTemplate)),
];

/// One concrete roster entry after `{u}` expansion.
#[derive(Debug, Clone)]
struct Case {
    text: String,
    form: Form,
    unit: Option<DurationUnit>,
    expect: Expect,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for row in ROSTER {
        if row.text.contains("{u}") {
            for (unit, suffix) in UNITS {
                cases.push(Case {
                    text: row.text.replace("{u}", suffix),
                    form: row.form,
                    unit: Some(unit),
                    expect: row.expect,
                });
            }
        } else {
            cases.push(Case {
                text: row.text.to_string(),
                form: row.form,
                unit: None,
                expect: row.expect,
            });
        }
    }
    cases
}

fn parse(text: &str) -> Result<Rule, content_policy::Invalid> {
    let policy = Policy::from_declaration(&Value::Array(vec![Value::String(text.to_string())]))?;
    Ok(policy.entries()[0].rule.clone())
}

/// The editor schema's `short_form` fragments as `(description, pattern)`.
fn shipped_fragments() -> Vec<(String, String)> {
    let schema: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(content_policy::EDITOR_SCHEMA).expect("the editor schema is YAML");
    schema["types"]["short_form"]
        .as_sequence()
        .expect("`short_form` is a union list")
        .iter()
        .map(|arm| {
            let arm = arm.as_str().expect("each `short_form` arm is a string");
            let (_, description) = arm.split_once(" -> ").expect("each arm has a description");
            // Patterns spell `(` and `)` as `\x28`/`\x29`, so the first `)`
            // after `pattern(` closes the argument.
            let start = arm.find("pattern(").expect("each arm has a pattern") + "pattern(".len();
            let end = start + arm[start..].find(')').expect("the pattern argument closes");
            (description.to_string(), arm[start..end].to_string())
        })
        .collect()
}

fn compiled_fragments() -> Vec<(Form, regex::Regex)> {
    let shipped = shipped_fragments();
    let descriptions: Vec<_> = shipped.iter().map(|(description, _)| description.as_str()).collect();
    let expected: Vec<_> = FRAGMENTS.iter().map(|(description, _)| *description).collect();
    assert_eq!(descriptions, expected, "the editor schema's `short_form` fragments changed");
    shipped
        .into_iter()
        .zip(FRAGMENTS)
        .map(|((_, pattern), (_, form))| {
            (form, regex::Regex::new(&pattern).unwrap_or_else(|error| panic!("{pattern}: {error}")))
        })
        .collect()
}

/// The roster names every rule form and unit, has a positive control for
/// every form (per unit where the form takes a duration), and keeps its
/// lexical-only rows to semantic checks the patterns cannot express.
#[test]
fn roster_covers_every_rule_form_and_unit() {
    for (unit, suffix) in UNITS {
        unit_is_rostered(unit);
        assert_eq!(unit.suffix(), suffix);
    }
    let cases = cases();
    for form in Form::ALL {
        let controls: BTreeSet<_> = cases
            .iter()
            .filter(|case| case.form == form && case.expect == Accept)
            .map(|case| case.unit.map(DurationUnit::suffix))
            .collect();
        if form.takes_duration() {
            let expected: BTreeSet<_> = UNITS.iter().map(|(_, suffix)| Some(*suffix)).collect();
            assert!(controls.is_superset(&expected), "{form:?} lacks a control per unit: {controls:?}");
        } else {
            assert!(!controls.is_empty(), "{form:?} has no positive control");
        }
        assert!(
            cases.iter().any(|case| case.form == form && case.expect == Reject),
            "{form:?} has no malformed spelling"
        );
    }
    let texts: BTreeSet<_> = cases.iter().map(|case| case.text.as_str()).collect();
    assert_eq!(texts.len(), cases.len(), "roster texts are unique");
}

/// Content Policy's public parser accepts exactly the rows the roster says
/// it accepts, reading each in the form and unit the row names.
#[test]
fn parser_meets_the_roster() {
    let mut failures = Vec::new();
    for case in cases() {
        match (parse(&case.text), case.expect.parser_accepts()) {
            (Ok(rule), true) => {
                let unit = match &rule {
                    Rule::ValidFor { duration, .. } => Some(duration.unit()),
                    _ => None,
                };
                if Form::of(&rule) != case.form || unit != case.unit {
                    failures.push(format!("{:?}: parsed as {rule:?}", case.text));
                }
            }
            (Err(_), false) => {}
            (Ok(rule), false) => failures.push(format!("{:?}: accepted as {rule:?}", case.text)),
            (Err(invalid), true) => failures.push(format!("{:?}: rejected: {invalid:?}", case.text)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Each shipped pattern fragment, compiled alone, matches exactly the
/// lexically valid rows of its own form; a duration fragment has one branch
/// per parser unit.
#[test]
fn editor_fragments_meet_the_roster() {
    let fragments = compiled_fragments();
    for (form, regex) in &fragments {
        if !form.takes_duration() {
            continue;
        }
        let units: BTreeSet<_> = regex
            .as_str()
            .split('|')
            .map(|branch| {
                let (_, after) = branch.split_once("[1-9][0-9]*").expect("a branch spells a count");
                after.split('\\').next().expect("a unit precedes the next escape").to_string()
            })
            .collect();
        let expected: BTreeSet<_> = UNITS.iter().map(|(_, suffix)| (*suffix).to_string()).collect();
        assert_eq!(units, expected, "{form:?} branches one per unit");
    }

    let mut failures = Vec::new();
    for case in cases() {
        let matched: Vec<_> = fragments
            .iter()
            .filter(|(_, regex)| regex.is_match(&case.text))
            .map(|(form, _)| *form)
            .collect();
        let expected = if case.expect.lexically_accepts() { vec![case.form] } else { Vec::new() };
        if matched != expected {
            failures.push(format!("{:?}: expected {expected:?}, matched {matched:?}", case.text));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The shipped schema, as Darkmatter's embedded baseline types
/// `content_policy`, accepts exactly the lexically valid rows, both as a
/// compact item and as a long-form `rule:`. One document holds every row, so
/// the schema prepares once.
#[test]
fn shipped_schema_through_darkmatter_meets_the_roster() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let api = DarkmatterSchemas::new(context_at(&root))
        .with_darkmatter_baseline_json_schema()
        .expect("the embedded baseline attaches");

    let cases = cases();
    let mut frontmatter = String::from("content_policy:\n");
    for case in &cases {
        // A JSON string is a YAML double-quoted scalar, so tabs and
        // non-ASCII digits survive exactly.
        let quoted = serde_json::to_string(&case.text).expect("a string serializes");
        frontmatter.push_str(&format!("  - {quoted}\n  - {{rule: {quoted}, action: refresh}}\n"));
    }
    let markdown: Markdown = format!("---\n{frontmatter}---\nBody\n").as_str().into();
    let report = api.validate(&markdown).expect("the schema prepares");

    let mut rejected: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for problem in &report.problems {
        let index = problem
            .path
            .strip_prefix("/content_policy/")
            .and_then(|rest| rest.split('/').next())
            .and_then(|index| index.parse::<usize>().ok())
            .unwrap_or_else(|| panic!("a problem outside a list item: {problem:?}"));
        rejected.entry(index).or_default().push(problem.path.clone());
    }
    let mut failures = Vec::new();
    for (position, case) in cases.iter().enumerate() {
        for (offset, shape) in [(0, "compact"), (1, "long-form")] {
            let accepted = !rejected.contains_key(&(position * 2 + offset));
            if accepted != case.expect.lexically_accepts() {
                failures.push(format!("{:?} ({shape}): schema accepted = {accepted}", case.text));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
