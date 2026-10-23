//! Catalog identity survives the migration (spec criterion 57): every
//! registered alias still dispatches to its canonical function, in any letter
//! case, and the catalog's descriptors list each canonical overload once and
//! never under an alias. A semantically duplicate overload failing to load is
//! `catalog::tests::semantically_duplicate_overloads_fail_catalog_load`
//! (catalog loader) and `schema_grammar_descriptors.rs` (criterion 6); a
//! wrong-returning handler is `function_dispatch.rs`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
use darkmatter::markdown::compose::expression::functions::dispatchable_canonical_names;
use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ExpressionError, ResolutionContext, evaluate, expression_function_descriptors,
    function_descriptors, parse,
};
use serde_json::Value;
use tempfile::TempDir;

/// The 75 aliases the registry held before the migration (the Phase 1
/// registry inventory), each with its canonical function and arguments that
/// bind to it.
const ALIASES: &[(&str, &str, &str)] = &[
    ("isnull", "is_null", "(null)"),
    ("isempty", "is_empty", r#"("")"#),
    ("isstring", "is_string", r#"("x")"#),
    ("isnumber", "is_number", "(4)"),
    ("isarray", "is_array", "([])"),
    ("isobject", "is_object", "({})"),
    ("ispositive", "is_positive", "(1)"),
    ("isnegative", "is_negative", "(-1)"),
    ("isinteger", "is_integer", "(1)"),
    ("haskey", "has_key", r#"({a: 1}, "a")"#),
    ("aslineseparated", "as_line_separated", "([1])"),
    ("ascsv", "as_csv", "([1])"),
    ("astsv", "as_tsv", "([1])"),
    ("asspaceseparated", "as_space_separated", "([1])"),
    ("asunorderedlist", "as_unordered_list", "([1])"),
    ("asorderedlist", "as_ordered_list", "([1])"),
    ("asjson", "as_json", "([1])"),
    ("asjson5", "as_json5", "([1])"),
    ("startswith", "starts_with", r#"("ab", "a")"#),
    ("endswith", "ends_with", r#"("ab", "b")"#),
    ("kebabcase", "kebab_case", r#"("a b")"#),
    ("snakecase", "snake_case", r#"("a b")"#),
    ("camelcase", "camel_case", r#"("a b")"#),
    ("pascalcase", "pascal_case", r#"("a b")"#),
    ("titlecase", "title_case", r#"("a b")"#),
    ("withoutdate", "without_date", r#"("a 2024-06-15")"#),
    ("ensureleading", "ensure_leading", r#"("b", "a")"#),
    ("ensuretrailing", "ensure_trailing", r#"("a", "b")"#),
    ("replacefirst", "replace_first", r#"("a", "a", "b")"#),
    ("replacelast", "replace_last", r#"("a", "a", "b")"#),
    ("isdate", "is_date", r#"("2024-06-15")"#),
    ("isdateutc", "is_date_utc", r#"("2024-06-15")"#),
    ("isdatetime", "is_date_time", r#"("2024-06-15T12:30:00")"#),
    ("is_datetime", "is_date_time", r#"("2024-06-15T12:30:00")"#),
    ("isdatetimeutc", "is_date_time_utc", r#"("2024-06-15T12:30:00Z")"#),
    ("is_datetime_utc", "is_date_time_utc", r#"("2024-06-15T12:30:00Z")"#),
    ("istoday", "is_today", r#"("1999-01-01")"#),
    ("istodayutc", "is_today_utc", r#"("1999-01-01")"#),
    ("isyesterday", "is_yesterday", r#"("1999-01-01")"#),
    ("isyesterdayutc", "is_yesterday_utc", r#"("1999-01-01")"#),
    ("istomorrow", "is_tomorrow", r#"("1999-01-01")"#),
    ("istomorrowutc", "is_tomorrow_utc", r#"("1999-01-01")"#),
    ("isthismonth", "is_this_month", r#"("1999-01-01")"#),
    ("isthismonthutc", "is_this_month_utc", r#"("1999-01-01")"#),
    ("isthisyear", "is_this_year", r#"("1999-01-01")"#),
    ("isthisyearutc", "is_this_year_utc", r#"("1999-01-01")"#),
    ("datedelta", "date_delta", r#"("2024-06-01", "2024-06-20", "14d")"#),
    ("olderthan", "older_than", r#"("2024-06-01", "2024-06-20", "14d")"#),
    ("newerthan", "newer_than", r#"("2024-06-20", "2024-06-01", "14d")"#),
    ("predictconflicts", "predict_conflicts", r#"("main")"#),
    ("branchexistsonremote", "branch_exists_on_remote", "()"),
    ("remotevendor", "remote_vendor", "()"),
    ("prlist", "pr_list", "(5)"),
    ("cicdlist", "cicd_list", "(5)"),
    ("fileexists", "file_exists", r#"("page.md")"#),
    ("findfiles", "find_files", r#"("*.md")"#),
    ("hascommand", "has_command", r#"("dm-no-such-command-4c1d")"#),
    ("hasbinary", "has_binary", r#"("dm-no-such-command-4c1d")"#),
    ("isindexedfile", "is_indexed_file", r#"("review-1.md")"#),
    ("fileindex", "file_index", r#"("review-1.md")"#),
    ("incrementfileindex", "increment_file_index", r#"("review-1.md")"#),
    ("decrementfileindex", "decrement_file_index", r#"("review-1.md")"#),
    ("findfirstindex", "find_first_index", r#"("review-1.md")"#),
    ("findlastindex", "find_last_index", r#"("review-1.md")"#),
    ("basenamewithoutindex", "basename_without_index", r#"("review-1.md")"#),
    ("parentdir", "parent_dir", r#"("sub/page.md")"#),
    ("filetrailing", "file_trailing", r#"("sub/page.md")"#),
    ("dirleading", "dir_leading", r#"("sub/page.md")"#),
    ("codelink", "code_link", r#"("page.md")"#),
    ("hasskill", "has_skill", r#"("x")"#),
    ("haslocalskill", "has_local_skill", r#"("x")"#),
    ("tryfrontmatter", "try_frontmatter", r#"("page.md")"#),
    ("markdownbodyempty", "markdown_body_empty", r#"("page.md")"#),
    ("markdowntitle", "markdown_title", r#"("page.md")"#),
    ("validateschema", "validate_schema", r#"("page.md")"#),
];

/// A resolution context anchored in a private temporary directory: an empty
/// Git repository with no remote holding `page.md` and `review-1.md`.
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

fn fixture() -> (TempDir, Lookup) {
    let dir = TempDir::new().expect("tempdir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical tempdir");
    git2::Repository::init(&root).expect("git init");
    std::fs::write(root.join("page.md"), "---\ntitle: Page\n---\n# Page\n").expect("write page");
    std::fs::write(root.join("review-1.md"), "# Review\n").expect("write review");
    std::fs::create_dir_all(root.join("home")).expect("home");
    let file_context =
        biscuit_file::FileResolutionContext::from_snapshot(&root, Some(root.join("home")), HashMap::new());
    (dir, Lookup { context: ResolutionContext::new(file_context) })
}

/// The registry's aliases, read from the catalog API that consumers use.
fn registered_aliases() -> BTreeMap<&'static str, &'static str> {
    let mut aliases = BTreeMap::new();
    for descriptor in expression_function_descriptors() {
        for alias in descriptor.aliases {
            if let Some(previous) = aliases.insert(*alias, descriptor.name) {
                assert_eq!(previous, descriptor.name, "alias `{alias}` names two functions");
            }
        }
    }
    aliases
}

#[test]
fn the_registered_aliases_are_the_inventoried_ones() {
    let inventoried: BTreeMap<&str, &str> =
        ALIASES.iter().map(|(alias, canonical, _)| (*alias, *canonical)).collect();
    assert_eq!(inventoried.len(), 75, "the inventory lists 75 aliases");
    assert_eq!(registered_aliases(), inventoried);
}

/// Each alias, in lowercase and uppercase, enters its canonical function's
/// handler exactly once. A domain error after entry (no remote, a missing
/// capture) still proves the alias bound to the canonical.
#[test]
fn aliases_resolve_as_before() {
    let (_dir, lookup) = fixture();
    for (alias, canonical, arguments) in ALIASES {
        for spelling in [alias.to_string(), alias.to_ascii_uppercase()] {
            let expression = format!("{spelling}{arguments}");
            hooks::reset_handler_entries();
            let result = evaluate(&parse(&expression).expect("the expression parses"), &lookup);
            if let Err(ExpressionError::Call(error)) = &result {
                panic!("{expression}: did not bind: {error}");
            }
            assert_eq!(hooks::handler_entries(canonical), 1, "{expression} must enter `{canonical}`: {result:?}");
        }
    }
}

/// Canonical names dispatch in any letter case too.
#[test]
fn canonical_names_resolve_case_insensitively() {
    let (_dir, lookup) = fixture();
    for expression in ["IS_NULL(null)", "Upper(\"a\")", "Has_Key({a: 1}, \"a\")"] {
        hooks::reset_handler_entries();
        evaluate(&parse(expression).expect("the expression parses"), &lookup).expect(expression);
        let canonical = expression.split('(').next().expect("name").to_ascii_lowercase();
        assert_eq!(hooks::handler_entries(&canonical), 1, "{expression}");
    }
}

/// The documentation source lists every canonical overload once, never an
/// alias, and an alias looks up its canonical's overloads.
#[test]
fn documentation_lists_canonicals_once() {
    let descriptors = expression_function_descriptors();
    let aliases = registered_aliases();
    let mut overloads: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for descriptor in descriptors {
        assert!(!aliases.contains_key(descriptor.name), "`{}` is listed under an alias", descriptor.name);
        overloads.entry(descriptor.name).or_default().push(descriptor.overload);
    }
    for (name, indices) in &overloads {
        let expected: Vec<usize> = (0..indices.len()).collect();
        assert_eq!(indices, &expected, "`{name}` lists each overload once, in order");
    }
    let listed: BTreeSet<&str> = overloads.keys().copied().collect();
    let registered: BTreeSet<&str> = dispatchable_canonical_names().into_iter().collect();
    assert_eq!(listed, registered);

    for (alias, canonical) in &aliases {
        let by_alias: Vec<_> = function_descriptors(alias).iter().map(|descriptor| descriptor.signature).collect();
        let by_name: Vec<_> = function_descriptors(canonical).iter().map(|descriptor| descriptor.signature).collect();
        assert!(!by_name.is_empty(), "`{canonical}` has descriptors");
        assert_eq!(by_alias, by_name, "`{alias}` looks up `{canonical}`");
    }
}
