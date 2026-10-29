//! Every value-argument expression function binds through the one coercion
//! engine before its handler runs (spec criterion 10), and no handler keeps
//! its own arity, type, or conversion code (criterion 11).
//!
//! One row per canonical function holds an accepted, a rejected, and a
//! converted-input fixture, each run through normal dispatch
//! (`evaluate(parse(..))`) with a handler-entry counter: a rejected fixture is
//! a binding error and never enters the handler; an accepted or converted
//! fixture enters it exactly once. Where the behavior ledger marks a cell
//! impossible, the row says why and its evidence shows the input arriving
//! unchanged (ruling R1); conversion itself is proven by the test-registered
//! functions in `function_binding_conversions.rs`.
//!
//! Every context function reads a resolution context anchored in a private
//! temporary directory that is an empty Git repository with no remote, an
//! empty home, and an empty environment. Git, provider, network, repository,
//! and composition functions therefore prove acceptance by entering their
//! handler and failing with a domain error (no remote, a capture that was
//! never taken, a deny-all authority); nothing reaches the network, ICMP, or
//! a login shell.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use darkmatter::markdown::compose::expression::call_binding::CallErrorKind;
use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
use darkmatter::markdown::compose::expression::functions::dispatchable_canonical_names;
use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ExpressionError, ResolutionContext, evaluate, parse,
};
use serde_json::{Value, json};
use tempfile::TempDir;

/// What one fixture must produce.
enum Expect {
    /// Success with exactly this value.
    Is(Value),
    /// Success with text that starts with this prefix.
    TextStartingWith(&'static str),
    /// Success with text that ends with this suffix.
    TextEndingWith(&'static str),
    /// Success with a list of this many items.
    ListOf(usize),
    /// The handler was entered and failed with its own (non-binding) error.
    DomainError,
    /// Binding failed with this kind; the handler was never entered.
    Binding(CallErrorKind),
}

struct Case {
    expression: &'static str,
    expect: Expect,
}

enum Rejected {
    Fixture(Case),
    /// The ledger marks the cell impossible; the evidence shows why.
    Impossible { reason: &'static str, evidence: Vec<Case> },
}

enum Converted {
    Fixture(Case),
    /// The ledger marks the cell impossible; the evidence shows each
    /// candidate input arriving unchanged (or, for a refined target that no
    /// converted text can satisfy, converting and then being rejected).
    Impossible { reason: &'static str, evidence: Vec<Case> },
}

struct Row {
    accepted: Case,
    rejected: Rejected,
    converted: Converted,
}

fn ok(expression: &'static str, value: Value) -> Case {
    Case { expression, expect: Expect::Is(value) }
}

fn starts(expression: &'static str, prefix: &'static str) -> Case {
    Case { expression, expect: Expect::TextStartingWith(prefix) }
}

fn ends(expression: &'static str, suffix: &'static str) -> Case {
    Case { expression, expect: Expect::TextEndingWith(suffix) }
}

fn list_of(expression: &'static str, len: usize) -> Case {
    Case { expression, expect: Expect::ListOf(len) }
}

fn domain(expression: &'static str) -> Case {
    Case { expression, expect: Expect::DomainError }
}

fn type_error(expression: &'static str) -> Case {
    Case { expression, expect: Expect::Binding(CallErrorKind::Type) }
}

fn rejects_type(expression: &'static str) -> Rejected {
    Rejected::Fixture(type_error(expression))
}

fn rejects_arity(expression: &'static str) -> Rejected {
    Rejected::Fixture(Case { expression, expect: Expect::Binding(CallErrorKind::Arity) })
}

fn converts(case: Case) -> Converted {
    Converted::Fixture(case)
}

fn unchanged(reason: &'static str, evidence: Vec<Case>) -> Converted {
    Converted::Impossible { reason, evidence }
}

const PRESERVATION: &str = "the only parameter is preservation-only `unknown | null`, so the engine keeps every input";
const ANY_LIST: &str = "`any[]` never wraps a scalar or parses text, and its unconstrained elements are kept";
const PLAIN_OBJECT: &str = "`object | null` never parses text or wraps a value, and a plain object's contents are kept";
const NO_DATE_TEXT: &str =
    "no number or boolean text is an ISO date or a duration, so a converted input always fails the refined type";

/// Builds a row whose rejected cell is arity only and whose converted cell is
/// impossible because the parameter preserves every value.
fn preservation(accepted: Case, arity: &'static str, evidence: Vec<Case>) -> Row {
    Row {
        accepted,
        rejected: rejects_arity(arity),
        converted: unchanged(PRESERVATION, evidence),
    }
}

/// A row for a predicate over dates: arity is the only rejection, and a
/// number is inspected as itself, never as its text.
fn date_predicate(accepted: Case, arity: &'static str, unconverted: [&'static str; 2]) -> Row {
    preservation(accepted, arity, unconverted.into_iter().map(|expression| ok(expression, json!(false))).collect())
}

/// A row for a list renderer, whose `any[]` parameter converts nothing.
fn list_renderer(accepted: Case, rejected: &'static str, evidence: Vec<Case>) -> Row {
    Row {
        accepted,
        rejected: rejects_type(rejected),
        converted: unchanged(ANY_LIST, evidence),
    }
}

fn row(accepted: Case, rejected: Rejected, converted: Converted) -> Row {
    Row { accepted, rejected, converted }
}

/// Variables plus a resolution context anchored at the fixture directory.
struct Lookup {
    context: ResolutionContext,
}

impl EvaluationLookup for Lookup {
    fn get(&self, _path: &str) -> Option<Value> {
        None
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        Some(&self.context)
    }
}

const FIXTURE: &str = "---\ntitle: Fixture Title\n---\n# Fixture Title\n\nBody.\n";
const NUMBERED: &str = "---\ntitle: Numbered\n---\n# Numbered\n\nBody.\n";

/// A private fixture tree: `fixture.md`, a file named `2026` (so a number
/// converted to text names a real file), an index family, `sub/note.md`, an
/// empty home, and an empty Git repository with no remote, so nothing
/// discovers a repository above it.
fn fixture() -> (TempDir, Lookup) {
    let dir = TempDir::new().expect("tempdir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical tempdir");
    git2::Repository::init(&root).expect("git init");
    let write = |relative: &str, content: &str| {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        std::fs::write(path, content).expect("write fixture");
    };
    write("fixture.md", FIXTURE);
    write("2026", NUMBERED);
    write("review-1.md", "# Review\n");
    write("review-2.md", "# Review\n");
    write("sub/note.md", "# Note\n");
    std::fs::create_dir_all(root.join("home")).expect("home");
    let lookup = lookup_at(&root);
    (dir, lookup)
}

fn lookup_at(root: &Path) -> Lookup {
    let file_context =
        biscuit_file::FileResolutionContext::from_snapshot(root, Some(root.join("home")), HashMap::new());
    Lookup { context: ResolutionContext::new(file_context) }
}

/// `and` and `or` are evaluated lazily by the evaluator, which binds each
/// operand as it goes; they have no handler to enter.
fn expected_entries(function: &str) -> usize {
    if matches!(function, "and" | "or") { 0 } else { 1 }
}

fn run_case(function: &str, cell: &str, case: &Case, lookup: &Lookup) {
    let expression = case.expression;
    hooks::reset_handler_entries();
    let result = evaluate(&parse(expression).unwrap_or_else(|error| panic!("{expression}: {error}")), lookup);
    let entries = hooks::handler_entries(function);
    let context = format!("{function} {cell} fixture `{expression}`");
    match (&case.expect, &result) {
        (Expect::Binding(kind), Err(ExpressionError::Call(error))) => {
            assert_eq!(error.kind(), *kind, "{context}: {error}");
            assert_eq!(error.function(), function, "{context}: the error names the function");
            assert_eq!(entries, 0, "{context}: a rejected call never enters the handler");
            return;
        }
        (Expect::Binding(kind), other) => panic!("{context}: expected a {kind:?} binding error, got {other:?}"),
        (Expect::DomainError, Err(ExpressionError::Call(error))) => {
            panic!("{context}: expected a domain error after entry, got a binding error: {error}")
        }
        (Expect::DomainError, Err(_)) => {}
        (Expect::DomainError, Ok(value)) => panic!("{context}: expected a domain error, got {value}"),
        (_, Err(error)) => panic!("{context}: {error}"),
        (Expect::Is(expected), Ok(value)) => assert_eq!(value, expected, "{context}"),
        (Expect::TextStartingWith(prefix), Ok(value)) => {
            assert!(value.as_str().is_some_and(|text| text.starts_with(prefix)), "{context}: {value}")
        }
        (Expect::TextEndingWith(suffix), Ok(value)) => {
            assert!(value.as_str().is_some_and(|text| text.ends_with(suffix)), "{context}: {value}")
        }
        (Expect::ListOf(len), Ok(value)) => {
            assert_eq!(value.as_array().map(Vec::len), Some(*len), "{context}: {value}")
        }
    }
    assert_eq!(entries, expected_entries(function), "{context}: handler entries");
}

fn check(function: &str, row: Row) {
    let (_dir, lookup) = fixture();
    run_case(function, "accepted", &row.accepted, &lookup);
    match &row.rejected {
        Rejected::Fixture(case) => {
            assert!(matches!(case.expect, Expect::Binding(_)), "{function}: a rejected fixture must fail binding");
            run_case(function, "rejected", case, &lookup);
        }
        Rejected::Impossible { reason, evidence } => {
            assert!(!reason.is_empty() && !evidence.is_empty(), "{function}: an impossible cell needs evidence");
            for case in evidence {
                run_case(function, "rejected-impossible", case, &lookup);
            }
        }
    }
    match &row.converted {
        Converted::Fixture(case) => {
            assert!(!matches!(case.expect, Expect::Binding(_)), "{function}: a converted fixture must bind");
            run_case(function, "converted", case, &lookup);
        }
        Converted::Impossible { reason, evidence } => {
            assert!(!reason.is_empty() && !evidence.is_empty(), "{function}: an impossible cell needs evidence");
            for case in evidence {
                run_case(function, "converted-impossible", case, &lookup);
            }
        }
    }
}

/// One `#[test]` per registered canonical function, so the rows run in
/// parallel, plus the list of names the guard compares with the registry.
macro_rules! registry_rows {
    ($($name:ident => $row:expr;)+) => {
        const ROW_NAMES: &[&str] = &[$(stringify!($name)),+];

        mod rows {
            use super::*;

            $(
                #[test]
                fn $name() {
                    check(stringify!($name), $row);
                }
            )+
        }
    };
}

registry_rows! {
    // ── Type predicates ────────────────────────────────────────────────────
    is_null => preservation(ok("is_null(null)", json!(true)), "is_null()", vec![
        ok("is_null(4)", json!(false)), ok(r#"is_null("4")"#, json!(false)), ok("is_null(true)", json!(false)),
    ]);
    is_empty => preservation(ok(r#"is_empty("")"#, json!(true)), "is_empty()", vec![
        ok("is_empty(0)", json!(false)), ok("is_empty(false)", json!(false)),
    ]);
    is_string => preservation(ok(r#"is_string("x")"#, json!(true)), "is_string()", vec![
        ok("is_string(4)", json!(false)), ok("is_string(true)", json!(false)), ok(r#"is_string("4")"#, json!(true)),
    ]);
    is_number => preservation(ok("is_number(4)", json!(true)), "is_number()", vec![
        ok(r#"is_number("4")"#, json!(false)), ok("is_number(true)", json!(false)),
    ]);
    is_array => preservation(ok("is_array([1, 2, 3])", json!(true)), "is_array()", vec![
        ok(r#"is_array("[1]")"#, json!(false)), ok("is_array(4)", json!(false)),
    ]);
    is_object => preservation(ok("is_object({a: 1})", json!(true)), "is_object()", vec![
        ok(r#"is_object("{}")"#, json!(false)), ok("is_object([])", json!(false)),
    ]);
    is_integer => preservation(ok("is_integer(7)", json!(true)), "is_integer()", vec![
        ok(r#"is_integer("7")"#, json!(false)), ok("is_integer(true)", json!(false)),
    ]);
    is_positive => row(ok("is_positive(5)", json!(true)), rejects_type("is_positive(true)"),
        converts(ok(r#"is_positive("4")"#, json!(true))));
    is_negative => row(ok("is_negative(-3)", json!(true)), rejects_type(r#"is_negative("pear")"#),
        converts(ok(r#"is_negative(" -4 ")"#, json!(true))));

    // ── Math and type conversion ───────────────────────────────────────────
    abs => row(ok("abs(-3)", json!(3)), rejects_type("abs(null)"), converts(ok(r#"abs("-3")"#, json!(3))));
    max => row(ok("max(2, 5)", json!(5)), rejects_type("max(true, 5)"), converts(ok(r#"max("7", 5)"#, json!(7))));
    min => row(ok("min(2, 5)", json!(2)), rejects_type(r#"min("pear", 5)"#), converts(ok(r#"min("4", 5)"#, json!(4))));
    // `x` is `numberlike` and delivered as sent; the fallback converts.
    round => row(ok("round(3.7)", json!(4)), rejects_type(r#"round("pear")"#),
        converts(ok(r#"round("pear", "2.7")"#, json!(3))));
    number => row(ok(r#"number("42")"#, json!(42)), rejects_type(r#"number("pear")"#),
        converts(ok(r#"number("pear", "7")"#, json!(7))));

    // ── Collection ─────────────────────────────────────────────────────────
    contains => row(ok(r#"contains("hello", "ell")"#, json!(true)), rejects_arity(r#"contains("a")"#),
        converts(ok("contains(1234, 23)", json!(true))));
    first => list_renderer(ok("first([1, 2, 3])", json!(1)), r#"first("abc")"#, vec![
        ok(r#"first(["4", true])"#, json!("4")), ok("first([4])", json!(4)),
    ]);
    last => list_renderer(ok("last([1, 2, 3])", json!(3)), r#"last("abc")"#, vec![
        ok(r#"last([true, "4"])"#, json!("4")), ok("last([4])", json!(4)),
    ]);
    has_key => row(ok(r#"has_key({a: 1}, "a")"#, json!(true)), rejects_type(r#"has_key("x", "a")"#),
        converts(ok(r#"has_key({"1": true}, 1)"#, json!(true))));
    keys => row(ok("keys({b: 1, a: 2})", json!(["b", "a"])), rejects_type("keys([1])"), unchanged(PLAIN_OBJECT, vec![
        ok(r#"keys({"1": "2"})"#, json!(["1"])), ok("keys(null)", json!([])),
    ]));
    values => row(ok("values({b: 1, a: 2})", json!([1, 2])), rejects_type(r#"values("x")"#), unchanged(PLAIN_OBJECT, vec![
        ok(r#"values({a: "4", b: true})"#, json!(["4", true])), ok("values(null)", json!([])),
    ]));
    length => row(ok(r#"length("hello")"#, json!(5)), rejects_type("length(true)"), converts(ok("length(2.5)", json!(3))));

    // ── String predicates and mutations ────────────────────────────────────
    starts_with => row(ok(r#"starts_with("hello", "he")"#, json!(true)), rejects_type(r#"starts_with(["a"], "a")"#),
        converts(ok("starts_with(2026, 20)", json!(true))));
    ends_with => row(ok(r#"ends_with("hello", "lo")"#, json!(true)), rejects_type(r#"ends_with("a", {})"#),
        converts(ok("ends_with(2026, 26)", json!(true))));
    lower => row(ok(r#"lower("HELLO")"#, json!("hello")), rejects_type(r#"lower(["A"])"#),
        converts(ok("lower(true)", json!("true"))));
    upper => row(ok(r#"upper("hello")"#, json!("HELLO")), rejects_type("upper([1])"),
        converts(ok("upper(2026)", json!("2026"))));
    capitalize => row(ok(r#"capitalize("hello")"#, json!("Hello")), rejects_type("capitalize({})"),
        converts(ok("capitalize(true)", json!("True"))));
    kebab_case => row(ok(r#"kebab_case("Hello World")"#, json!("hello-world")), rejects_type("kebab_case([])"),
        converts(ok("kebab_case(2026)", json!("2026"))));
    snake_case => row(ok(r#"snake_case("Hello World")"#, json!("hello_world")), rejects_type("snake_case([])"),
        converts(ok("snake_case(2026)", json!("2026"))));
    camel_case => row(ok(r#"camel_case("hello world")"#, json!("helloWorld")), rejects_type("camel_case([])"),
        converts(ok("camel_case(2026)", json!("2026"))));
    pascal_case => row(ok(r#"pascal_case("hello world")"#, json!("HelloWorld")), rejects_type("pascal_case([])"),
        converts(ok("pascal_case(2026)", json!("2026"))));
    title_case => row(ok(r#"title_case("hello world")"#, json!("Hello World")), rejects_type("title_case([])"),
        converts(ok("title_case(2026)", json!("2026"))));
    trim => row(ok(r#"trim("  a  ")"#, json!("a")), rejects_type("trim({})"), converts(ok("trim(4)", json!("4"))));
    without_date => row(ok(r#"without_date("Note 2024-06-15")"#, json!("Note ")), rejects_type("without_date([])"),
        converts(ok("without_date(42)", json!("42"))));
    ensure_leading => row(ok(r#"ensure_leading("world", "hello ")"#, json!("hello world")),
        rejects_type(r#"ensure_leading([], "a")"#), converts(ok(r#"ensure_leading(true, "x")"#, json!("xtrue"))));
    ensure_trailing => row(ok(r#"ensure_trailing("hello", " world")"#, json!("hello world")),
        rejects_type(r#"ensure_trailing({}, "a")"#), converts(ok(r#"ensure_trailing(true, "!")"#, json!("true!"))));
    replace => row(ok(r#"replace("a.b.c", ".", "/")"#, json!("a/b/c")), rejects_type(r#"replace([], ".", "/")"#),
        converts(ok("replace(2026, 0, 1)", json!("2126"))));
    replace_first => row(ok(r#"replace_first("a.b.c", ".", "/")"#, json!("a/b.c")),
        rejects_type(r#"replace_first("a", [], "/")"#), converts(ok("replace_first(1001, 1, 7)", json!("7001"))));
    replace_last => row(ok(r#"replace_last("a.b.c", ".", "/")"#, json!("a.b/c")),
        rejects_type(r#"replace_last("a", ".", {})"#), converts(ok("replace_last(1001, 1, 7)", json!("1007"))));
    terminal => row(ok(r#"terminal("hello")"#, json!("hello")), rejects_type("terminal([1])"),
        converts(ok("terminal(42)", json!("42"))));

    // ── Dates ──────────────────────────────────────────────────────────────
    date => row(ok(r#"date("2024-06-15", "long")"#, json!("Sat, June 15th, 2024")), rejects_type(r#"date("pear", "long")"#),
        unchanged(NO_DATE_TEXT, vec![type_error(r#"date(20240615, "long")"#), type_error(r#"date("2024-06-15", 1)"#)]));
    is_date => date_predicate(ok(r#"is_date("2024-06-15")"#, json!(true)), "is_date()", ["is_date(20240615)", "is_date(true)"]);
    is_date_utc => date_predicate(ok(r#"is_date_utc("2024-06-15")"#, json!(true)), "is_date_utc()",
        ["is_date_utc(20240615)", "is_date_utc(true)"]);
    is_date_time => date_predicate(ok(r#"is_date_time("2024-06-15T12:30:00")"#, json!(true)), "is_date_time()",
        ["is_date_time(20240615)", "is_date_time(true)"]);
    is_date_time_utc => date_predicate(ok(r#"is_date_time_utc("2024-06-15T12:30:00Z")"#, json!(true)),
        "is_date_time_utc()", ["is_date_time_utc(20240615)", "is_date_time_utc(true)"]);
    is_today => date_predicate(ok(r#"is_today("1999-01-01")"#, json!(false)), "is_today()", ["is_today(4)", "is_today(true)"]);
    is_today_utc => date_predicate(ok(r#"is_today_utc("1999-01-01")"#, json!(false)), "is_today_utc()",
        ["is_today_utc(4)", "is_today_utc(true)"]);
    is_yesterday => date_predicate(ok(r#"is_yesterday("1999-01-01")"#, json!(false)), "is_yesterday()",
        ["is_yesterday(4)", "is_yesterday(true)"]);
    is_yesterday_utc => date_predicate(ok(r#"is_yesterday_utc("1999-01-01")"#, json!(false)), "is_yesterday_utc()",
        ["is_yesterday_utc(4)", "is_yesterday_utc(true)"]);
    is_tomorrow => date_predicate(ok(r#"is_tomorrow("1999-01-01")"#, json!(false)), "is_tomorrow()",
        ["is_tomorrow(4)", "is_tomorrow(true)"]);
    is_tomorrow_utc => date_predicate(ok(r#"is_tomorrow_utc("1999-01-01")"#, json!(false)), "is_tomorrow_utc()",
        ["is_tomorrow_utc(4)", "is_tomorrow_utc(true)"]);
    is_this_month => date_predicate(ok(r#"is_this_month("1999-01-01")"#, json!(false)), "is_this_month()",
        ["is_this_month(4)", "is_this_month(true)"]);
    is_this_month_utc => date_predicate(ok(r#"is_this_month_utc("1999-01-01")"#, json!(false)), "is_this_month_utc()",
        ["is_this_month_utc(4)", "is_this_month_utc(true)"]);
    is_this_year => date_predicate(ok(r#"is_this_year("1999-01-01")"#, json!(false)), "is_this_year()",
        ["is_this_year(4)", "is_this_year(true)"]);
    is_this_year_utc => date_predicate(ok(r#"is_this_year_utc("1999-01-01")"#, json!(false)), "is_this_year_utc()",
        ["is_this_year_utc(4)", "is_this_year_utc(true)"]);
    date_delta => row(ok(r#"date_delta("2024-06-01", "2024-06-20", "14d")"#, json!(true)),
        rejects_type(r#"date_delta("pear", "2024-06-20", "14d")"#),
        unchanged(NO_DATE_TEXT, vec![type_error(r#"date_delta("2024-06-01", "2024-06-20", 14)"#)]));
    newer_than => row(ok(r#"newer_than("2024-06-20", "2024-06-01", "14d")"#, json!(true)),
        rejects_type(r#"newer_than("2024-06-20", null, "14d")"#),
        unchanged(NO_DATE_TEXT, vec![type_error(r#"newer_than(20240620, "2024-06-01", "14d")"#)]));
    older_than => row(ok(r#"older_than("2024-06-01", "2024-06-20", "14d")"#, json!(true)),
        rejects_type(r#"older_than("2024-06-01", "2024-06-20", "fortnight")"#),
        unchanged(NO_DATE_TEXT, vec![type_error(r#"older_than("2024-06-01", "2024-06-20", true)"#)]));

    // ── Logical (lazy: no handler; the evaluator binds each operand) ───────
    and => row(ok(r#"and(true, "x")"#, json!(true)),
        Rejected::Impossible {
            reason: "every value binds and every arity, including zero, is eligible",
            evidence: vec![ok("and()", json!(true)), ok("and(null, [1], {})", json!(false))],
        },
        unchanged("preservation-only `unknown | null` variadic: truthiness reads the original value", vec![
            ok(r#"and("0")"#, json!(true)), ok("and(0)", json!(false)),
        ]));
    or => row(ok(r#"or(false, "x")"#, json!(true)),
        Rejected::Impossible {
            reason: "every value binds and every arity, including zero, is eligible",
            evidence: vec![ok("or()", json!(false)), ok("or(null, [], {})", json!(false))],
        },
        unchanged("preservation-only `unknown | null` variadic: truthiness reads the original value", vec![
            ok(r#"or("false")"#, json!(true)), ok("or(0)", json!(false)),
        ]));

    // ── Git (an empty repository with no remote, deny-all remote policy) ───
    // With no remote there is nothing to ask: no branch exists on one, and
    // the vendor is the empty member of its enum; a named remote (here the
    // converted text `"1"`) is not configured.
    predict_conflicts => row(domain(r#"predict_conflicts("main")"#), rejects_type(r#"predict_conflicts(["main"])"#),
        converts(domain("predict_conflicts(123)")));
    branch_exists_on_remote => row(ok("branch_exists_on_remote()", json!(false)), rejects_type("branch_exists_on_remote([])"),
        converts(ok("branch_exists_on_remote(123)", json!(false))));
    remote_vendor => row(ok("remote_vendor()", json!("")), rejects_type("remote_vendor({})"),
        converts(domain("remote_vendor(1)")));
    // No captured repository root: a bound count reads no history.
    recent_commits => row(ok("recent_commits(3)", json!([])), rejects_type("recent_commits(0)"),
        converts(ok(r#"recent_commits("3")"#, json!([]))));

    // ── Providers (no remote: the handler fails before any request) ────────
    // `0` fails the number option but converts to the text `"0"`, which
    // the `string(min(1))` option admits, so `pr(0)` binds and the handler
    // rejects it; a list reaches no option.
    pr => row(domain("pr(123)"), rejects_type("pr([1])"), converts(domain("pr(true)")));
    pr_list => row(domain("pr_list(5)"), rejects_type("pr_list(0)"), converts(domain(r#"pr_list("3")"#)));
    cicd => row(domain("cicd(456)"), rejects_type(r#"cicd("")"#), converts(domain("cicd(true)")));
    cicd_list => row(domain("cicd_list(5)"), rejects_type("cicd_list(101)"), converts(domain(r#"cicd_list("3")"#)));

    // ── Repository and network (captures never taken; ICMP denied) ─────────
    package => row(domain(r#"package("sub/note.md")"#), rejects_type("package([])"), converts(domain("package(2026)")));
    package_area => row(domain(r#"package_area("sub/note.md")"#), rejects_type("package_area(null)"),
        converts(domain("package_area(2026)")));
    ipv4 => row(domain("ipv4()"), rejects_type("ipv4([])"), converts(domain("ipv4(10)")));
    ipv6 => row(domain("ipv6()"), rejects_type("ipv6(null)"), converts(domain("ipv6(1)")));
    // The default ICMP authority grants nothing, so no packet is sent and
    // the answer is `null`.
    ping => row(ok(r#"ping("127.0.0.1")"#, Value::Null), rejects_type(r#"ping("example.com")"#),
        converts(ok(r#"ping("127.0.0.1", "500")"#, Value::Null)));
    ping_under => row(ok(r#"ping_under("127.0.0.1", 50)"#, Value::Null), rejects_arity(r#"ping_under("127.0.0.1")"#),
        converts(ok(r#"ping_under("127.0.0.1", "50", "2")"#, Value::Null)));

    // ── Filesystem ─────────────────────────────────────────────────────────
    absolute => row(ends(r#"absolute("fixture.md")"#, "fixture.md"), rejects_type("absolute([])"),
        converts(ends("absolute(2026)", "2026")));
    relative => row(ok(r#"relative("fixture.md")"#, json!("fixture.md")), rejects_type("relative({})"),
        converts(ok("relative(2026)", json!("2026"))));
    basename => row(ok(r#"basename("sub/note.md")"#, json!("note.md")), rejects_type("basename([])"),
        converts(ok("basename(2026)", json!("2026"))));
    basename_without_index => row(ok(r#"basename_without_index("review-1.md")"#, json!("review.md")),
        rejects_type("basename_without_index([])"), converts(ok("basename_without_index(2026)", json!("2026"))));
    dirname => row(ok(r#"dirname("sub/note.md")"#, json!("sub")), rejects_type("dirname([])"),
        converts(ok("dirname(2026)", json!(""))));
    ext => row(ok(r#"ext("sub/note.md")"#, json!("md")), rejects_type("ext([])"), converts(ok("ext(2026.5)", json!("5"))));
    parent_dir => row(ok(r#"parent_dir("sub/note.md")"#, json!("sub")), rejects_type("parent_dir([])"),
        converts(ok("parent_dir(2026)", json!(""))));
    file_trailing => row(ok(r#"file_trailing("sub/note.md")"#, json!("sub/note.md")), rejects_type("file_trailing([])"),
        converts(ok("file_trailing(2026)", json!("2026"))));
    dir_leading => row(ok(r#"dir_leading("sub/note.md")"#, json!("")), rejects_type("dir_leading([])"),
        converts(ok("dir_leading(2026)", json!(""))));
    file_index => row(ok(r#"file_index("review-1.md")"#, json!(1)), rejects_type("file_index([])"),
        converts(ok("file_index(2026)", json!(-1))));
    increment_file_index => row(ok(r#"increment_file_index("review-1.md")"#, json!("review-2.md")),
        rejects_type("increment_file_index([])"), converts(ok("increment_file_index(2026)", json!("2026-2"))));
    decrement_file_index => row(ok(r#"decrement_file_index("review-2.md")"#, json!("review-1.md")),
        rejects_type("decrement_file_index([])"), converts(ok("decrement_file_index(2026)", json!("2026-0"))));
    find_first_index => row(ok(r#"find_first_index("review-2.md")"#, json!("review-1.md")),
        rejects_type("find_first_index([])"), converts(ok("find_first_index(2026)", json!("2026"))));
    find_last_index => row(ok(r#"find_last_index("review-1.md")"#, json!("review-2.md")),
        rejects_type("find_last_index([])"), converts(ok("find_last_index(2026)", json!("2026"))));
    is_file_type => preservation(ok(r#"is_file_type("./docs/spec.md")"#, json!(true)), "is_file_type()", vec![
        ok("is_file_type(42)", json!(false)), ok(r#"is_file_type("42")"#, json!(true)), ok("is_file_type(null)", json!(false)),
        ok(r#"is_file_type("https://")"#, json!(false)),
    ]);
    is_indexed_file => row(ok(r#"is_indexed_file("review-1.md")"#, json!(true)), rejects_type("is_indexed_file([])"),
        converts(ok("is_indexed_file(2026)", json!(false))));
    join => row(ok(r#"join("sub", "note.md")"#, json!("sub/note.md")), rejects_type(r#"join([], "a")"#),
        converts(ok("join(1, 2)", json!("1/2"))));
    link => row(starts(r#"link("fixture.md")"#, "[fixture.md]("), rejects_type("link([])"),
        converts(starts(r#"link("fixture.md", 2026)"#, "[2026](")));
    code_link => row(starts(r#"code_link("fixture.md")"#, "[`fixture.md`]("), rejects_type("code_link([])"),
        converts(starts(r#"code_link("fixture.md", 2026)"#, "[`2026`](")));
    // A file named `2026` exists, so `false` for the number proves it was
    // inspected, not converted to the text that names the file.
    file_exists => row(ok(r#"file_exists("fixture.md")"#, json!(true)), rejects_arity("file_exists()"),
        unchanged("preservation-only `unknown | null`: the argument is inspected, not converted (criterion 42)", vec![
            ok("file_exists(2026)", json!(false)), ok(r#"file_exists("2026")"#, json!(true)),
        ]));
    find_files => row(list_of(r#"find_files("**/note.md")"#, 1), rejects_type("find_files([])"),
        converts(list_of("find_files(2026)", 1)));
    frontmatter => row(ok(r#"frontmatter("fixture.md")"#, json!({"title": "Fixture Title"})),
        rejects_type(r#"frontmatter("fixture.md", [])"#), converts(ok("frontmatter(2026)", json!({"title": "Numbered"}))));
    try_frontmatter => row(
        ok(r#"try_frontmatter("fixture.md")"#, json!({"ok": true, "value": {"title": "Fixture Title"}, "error": null})),
        rejects_type("try_frontmatter([])"),
        converts(ok("try_frontmatter(2026)", json!({"ok": true, "value": {"title": "Numbered"}, "error": null}))));
    markdown_body_empty => row(ok(r#"markdown_body_empty("fixture.md")"#, json!(false)),
        rejects_type("markdown_body_empty([])"), converts(ok("markdown_body_empty(2026)", json!(false))));
    markdown_title => row(ok(r#"markdown_title("fixture.md")"#, json!("Fixture Title")),
        rejects_type("markdown_title({})"), converts(ok("markdown_title(2026)", json!("Numbered"))));
    validate_schema => row(ok(r#"validate_schema("fixture.md")"#, json!(true)),
        rejects_type(r#"validate_schema("fixture.md", "x")"#), converts(ok("validate_schema(2026)", json!(true))));
    has_command => row(ok(r#"has_command("dm-no-such-command-4c1d")"#, json!(false)), rejects_type(r#"has_command(["x"])"#),
        converts(ok("has_command(987654321)", json!(false))));
    has_binary => row(ok(r#"has_binary("dm-no-such-binary-4c1d")"#, json!(false)), rejects_type("has_binary({})"),
        converts(ok("has_binary(987654321)", json!(false))));

    // ── Context, shell, and agentic CLIs ───────────────────────────────────
    has_skill => row(ok(r#"has_skill("darkmatter")"#, json!(false)), rejects_type("has_skill([])"),
        converts(ok("has_skill(1)", json!(false))));
    has_local_skill => row(ok(r#"has_local_skill("darkmatter")"#, json!(false)), rejects_type("has_local_skill({})"),
        converts(ok("has_local_skill(1)", json!(false))));
    // The default shell authority never launches a login shell.
    has_alias => row(ok(r#"has_alias("ll")"#, json!(false)), rejects_type("has_alias([])"),
        converts(ok("has_alias(1)", json!(false))));
    has_builtin_function => row(ok(r#"has_builtin_function("cd")"#, json!(false)),
        rejects_type("has_builtin_function([])"), converts(ok("has_builtin_function(true)", json!(false))));
    has_user_function => row(ok(r#"has_user_function("mkcd")"#, json!(false)), rejects_type("has_user_function({})"),
        converts(ok("has_user_function(1)", json!(false))));
    can_execute => row(ok(r#"can_execute("dm-no-such-command-4c1d")"#, json!(false)), rejects_type("can_execute([])"),
        converts(ok("can_execute(987654321)", json!(false))));
    // The `Agent` group was never captured: a roster name binds, then the
    // handler reports the missing capture.
    has_agentic_cli => row(domain(r#"has_agentic_cli("claude")"#), rejects_type(r#"has_agentic_cli("aider")"#),
        Converted::Impossible {
            reason: "no number or boolean text is a roster name, so a converted input always fails the enum",
            evidence: vec![type_error("has_agentic_cli(1)"), type_error("has_agentic_cli(true)")],
        });

    // ── List formatting ────────────────────────────────────────────────────
    as_csv => list_renderer(ok("as_csv([1, 2, 3])", json!("1, 2, 3")), r#"as_csv("a, b")"#, vec![
        ok(r#"as_csv(["4", true])"#, json!("4, true")),
    ]);
    as_json => list_renderer(ok(r#"as_json(["a", 1])"#, json!(r#"["a",1]"#)), r#"as_json("[1]")"#, vec![
        ok(r#"as_json(["4", 4, true])"#, json!(r#"["4",4,true]"#)),
    ]);
    as_json5 => list_renderer(ok(r#"as_json5(["a", 1])"#, json!("['a', 1]")), "as_json5(4)", vec![
        ok(r#"as_json5(["4", 4])"#, json!("['4', 4]")),
    ]);
    as_line_separated => list_renderer(ok(r#"as_line_separated(["a", "b"])"#, json!("a\nb")), "as_line_separated(true)", vec![
        ok(r#"as_line_separated(["4", 4])"#, json!("4\n4")),
    ]);
    as_space_separated => list_renderer(ok("as_space_separated([1, 2, 3])", json!("1 2 3")),
        r#"as_space_separated("a b")"#, vec![ok(r#"as_space_separated(["4", false])"#, json!("4 false"))]);
    as_tsv => list_renderer(ok(r#"as_tsv(["a", "b"])"#, json!("a\tb")), "as_tsv({})", vec![
        ok(r#"as_tsv(["4", 4])"#, json!("4\t4")),
    ]);
    as_unordered_list => list_renderer(ok(r#"as_unordered_list(["a", "b"])"#, json!("- a\n- b")),
        r#"as_unordered_list("a")"#, vec![ok(r#"as_unordered_list(["4", true])"#, json!("- 4\n- true"))]);
    as_ordered_list => list_renderer(ok(r#"as_ordered_list(["a", "b"])"#, json!("1. a\n2. b")),
        r#"as_ordered_list("a")"#, vec![ok(r#"as_ordered_list(["4", true])"#, json!("1. 4\n2. true"))]);

    // ── Composition (no compose request: the handler reports it) ──────────
    as_markdown => row(domain(r##"as_markdown("# Title")"##), rejects_type("as_markdown([])"),
        converts(domain("as_markdown(42)")));
}

/// Value-handler rows and inline callback contracts cover the registry exactly.
/// A callback has no value handler to count; its call is checked passively.
#[test]
fn every_registered_function_has_a_row() {
    let mut rows: BTreeSet<&str> = ROW_NAMES.iter().copied().collect();
    assert_eq!(rows.len(), ROW_NAMES.len(), "a function has two rows");
    let declared = darkmatter::markdown::compose::expression::DeclaredTypes::new(None);
    let scope = darkmatter::markdown::compose::expression::TypeScope::unguarded(&declared);
    for (name, source) in [("map", "map([1], x => x)"), ("filter", "filter([1], x -> x)")] {
        assert!(rows.insert(name), "a callback also has a value-handler row");
        let expression = darkmatter::markdown::compose::expression::parse_spanned(source).unwrap();
        let checks = darkmatter::markdown::compose::expression::check_expression_calls(&expression, &scope);
        assert_eq!(checks[0].category(), darkmatter::markdown::compose::expression::ArgumentCategory::Compatible);
    }
    let registered: BTreeSet<&str> = dispatchable_canonical_names().into_iter().collect();
    let missing: Vec<_> = registered.difference(&rows).collect();
    let extra: Vec<_> = rows.difference(&registered).collect();
    assert!(missing.is_empty() && extra.is_empty(), "registered without a row: {missing:?}; rows without a function: {extra:?}");
}

/// Criterion 11: the per-handler arity, type-check, and conversion helpers are
/// deleted from every file of the function registry. The second check keeps
/// the scanned list complete as files are added.
#[test]
fn no_handler_local_coercion_helpers_remain() {
    let sources: &[(&str, &str)] = &[
        ("agentic_cli.rs", include_str!("../../src/markdown/compose/expression/functions/agentic_cli.rs")),
        (
            "agentic_cli_generated.rs",
            include_str!("../../src/markdown/compose/expression/functions/agentic_cli_generated.rs"),
        ),
        ("cicd.rs", include_str!("../../src/markdown/compose/expression/functions/cicd.rs")),
        ("collections.rs", include_str!("../../src/markdown/compose/expression/functions/collections.rs")),
        ("composition.rs", include_str!("../../src/markdown/compose/expression/functions/composition.rs")),
        ("dates.rs", include_str!("../../src/markdown/compose/expression/functions/dates.rs")),
        ("escape.rs", include_str!("../../src/markdown/compose/expression/functions/escape.rs")),
        ("fs.rs", include_str!("../../src/markdown/compose/expression/functions/fs.rs")),
        ("fs_tests.rs", include_str!("../../src/markdown/compose/expression/functions/fs_tests.rs")),
        ("git.rs", include_str!("../../src/markdown/compose/expression/functions/git.rs")),
        ("markdown_docs.rs", include_str!("../../src/markdown/compose/expression/functions/markdown_docs.rs")),
        ("mod.rs", include_str!("../../src/markdown/compose/expression/functions/mod.rs")),
        ("network.rs", include_str!("../../src/markdown/compose/expression/functions/network.rs")),
        ("paths.rs", include_str!("../../src/markdown/compose/expression/functions/paths.rs")),
        ("predicates.rs", include_str!("../../src/markdown/compose/expression/functions/predicates.rs")),
        ("provider.rs", include_str!("../../src/markdown/compose/expression/functions/provider.rs")),
        ("pull_requests.rs", include_str!("../../src/markdown/compose/expression/functions/pull_requests.rs")),
        ("repository.rs", include_str!("../../src/markdown/compose/expression/functions/repository.rs")),
        ("shell.rs", include_str!("../../src/markdown/compose/expression/functions/shell.rs")),
        ("skills.rs", include_str!("../../src/markdown/compose/expression/functions/skills.rs")),
        ("strings.rs", include_str!("../../src/markdown/compose/expression/functions/strings.rs")),
        ("terminal.rs", include_str!("../../src/markdown/compose/expression/functions/terminal.rs")),
        ("test_hooks.rs", include_str!("../../src/markdown/compose/expression/functions/test_hooks.rs")),
    ];
    let mut found = Vec::new();
    for (file, source) in sources {
        for retired in [
            "require_args",
            "require_number(",
            "require_string(",
            "require_array(",
            "any_null(",
            "to_number_coerce",
            "ExpressionError::ArgType",
            "ExpressionError::Arity",
        ] {
            if source.contains(retired) {
                found.push(format!("{file}: {retired}"));
            }
        }
    }
    assert!(found.is_empty(), "handler-local coercion code remains:\n{}", found.join("\n"));

    let directory = biscuit_test_harness::manifest_dir!().join("src/markdown/compose/expression/functions");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&directory)
        .expect("read the functions directory")
        .map(|entry| entry.expect("directory entry").path())
        .inspect(|path| assert!(!path.is_dir(), "unscanned subdirectory {}", path.display()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| path.file_name().expect("file name").to_string_lossy().into_owned())
        .collect();
    let scanned: BTreeSet<String> = sources.iter().map(|(file, _)| (*file).to_string()).collect();
    assert_eq!(scanned, on_disk, "the scan must include every file of the function registry");
}
