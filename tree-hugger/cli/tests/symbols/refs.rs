//! `hug refs`: selecting one declaration (exact smart-case names, qualified
//! and file-prefixed names, `--id`, locals), the ambiguity error, empty
//! reports, the three directions, the versioned `--json` envelope, the text
//! report, rejected flags and formats, the file-local boundary, and Rust
//! pattern names (no invented self-calls or declarations), parameter
//! defaults and types (references, never parameters), and unreported
//! declaration tokens (never suggestions).

use std::process::Output;

use serde_json::Value;

use crate::{Scratch, json_of, stdout_of};

/// A TypeScript file with direct calls (two on one line), a usage, an
/// arrow-function owner, file-level code, two same-named methods called on
/// an untyped receiver, and loop/catch bindings without records.
const APP_TS: &str = "\
class A { run() { return 1; } }
class B { run() { return 2; } }
export function parse(): number { return 0; }
function go(x: A) { x.run(); parse(); parse(); }
const f = () => parse();
const handler = parse;
parse();
function quiet() { return 1; }
function apply(xs: Array<() => void>) {
  for (const g of xs) { g(); }
  try { xs[0](); } catch (e) { e(); }
  parse();
  missing();
}
";

/// Another file calling `parse` by the same name: never a resolved caller
/// of `app.ts`'s `parse`.
const OTHER_TS: &str = "\
import { parse } from './app';
function main() { parse(); }
";

/// A Rust file with a bound import, an unresolved call and a parameter.
const TOOLS_RS: &str = "\
use std::io::read_to_string as slurp;
fn tool(n: u32) -> u32 {
    slurp(n);
    nothing();
    n + n
}
";

const BOUNDARY: &str = "References are resolved within one file";

fn scratch() -> Scratch {
    Scratch::new(&[("app.ts", APP_TS), ("other.ts", OTHER_TS), ("tools.rs", TOOLS_RS)])
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn refs_json(scratch: &Scratch, args: &[&str]) -> Value {
    let mut full = vec!["--json", "refs"];
    full.extend_from_slice(args);
    json_of(&scratch.stdout(&full))
}

/// The listed rows of the report's one file as `(kind, written target,
/// line:column, confidence)`.
fn rows(json: &Value, key: &str) -> Vec<(String, String, String, String)> {
    let files = json["files"].as_array().expect("files");
    assert_eq!(files.len(), 1, "one file: {json}");
    files[0][key]
        .as_array()
        .unwrap_or_else(|| panic!("{key}"))
        .iter()
        .map(|row| {
            let target = &row["target"];
            let written = target["qualified_name"].as_str().or(target["name"].as_str()).expect("name");
            (
                row["kind"].as_str().expect("kind").to_string(),
                written.to_string(),
                format!("{}:{}", row["span"]["start"]["line"], row["span"]["start"]["column"]),
                row["confidence"].as_str().expect("confidence").to_string(),
            )
        })
        .collect()
}

fn row(kind: &str, written: &str, at: &str, confidence: &str) -> (String, String, String, String) {
    (kind.into(), written.into(), at.into(), confidence.into())
}

fn declaration_id(json: &Value) -> String {
    json["declaration"]["id"].as_str().expect("declaration id").to_string()
}

/// The default lists established calls and usages from every owner of the
/// declaration's file, each call site kept (two calls on one line are two
/// rows), with owners prefixed for presentation and the anonymous owner
/// region distinguishable.
#[test]
fn references_list_every_established_call_and_usage_in_the_declarations_file() {
    let scratch = scratch();
    let json = refs_json(&scratch, &["parse", "app.ts"]);
    assert_eq!(json["declaration"]["identity"]["qualified_name"], "app.ts#parse");
    assert_eq!(json["direction"], "references");
    assert_eq!(json["selector"], serde_json::json!({ "name": "parse" }));
    assert_eq!(
        rows(&json, "occurrences"),
        vec![
            row("Call", "parse", "4:30", "Scoped"),
            row("Call", "parse", "4:39", "Scoped"),
            row("Call", "parse", "5:17", "Scoped"),
            row("Usage", "parse", "6:17", "Scoped"),
            row("Call", "parse", "7:1", "Scoped"),
            row("Call", "parse", "12:3", "Scoped"),
        ]
    );
    let id = declaration_id(&json);
    let file = &json["files"][0];
    for occurrence in file["occurrences"].as_array().expect("occurrences") {
        assert_eq!(occurrence["target"]["id"], id.as_str(), "every row is bound to the selected ID");
        assert!(occurrence.get("absent_id").is_none());
    }
    let owners: Vec<Option<&str>> = file["occurrences"]
        .as_array()
        .expect("occurrences")
        .iter()
        .map(|occurrence| occurrence["owner"]["qualified_name"].as_str())
        .collect();
    assert_eq!(
        owners,
        vec![Some("app.ts#go"), Some("app.ts#go"), Some("app.ts#f"), None, None, Some("app.ts#apply")],
        "owners carry the displayed prefix; file-level code has none"
    );
    assert_eq!(file["file"], "app.ts");
    assert!(file["name_only_suggestions"].as_array().expect("suggestions").is_empty());
    assert!(
        file["completed_capabilities"]
            .as_array()
            .expect("capabilities")
            .iter()
            .any(|capability| capability == "occurrences")
    );
    // Every listed row's owning region is reported.
    let regions: Vec<&Value> = file["coverage"].as_array().expect("coverage").iter().map(|region| &region["span"]).collect();
    for occurrence in file["occurrences"].as_array().expect("occurrences") {
        assert!(regions.contains(&&occurrence["owner_span"]), "coverage for {occurrence}");
    }
}

/// `--callers` keeps only incoming calls: the usage `const handler = parse`
/// is gone and the repeated calls stay.
#[test]
fn callers_exclude_usages_and_keep_repeated_calls() {
    let scratch = scratch();
    let json = refs_json(&scratch, &["parse", "--callers", "app.ts"]);
    assert_eq!(json["direction"], "callers");
    let listed = rows(&json, "occurrences");
    assert!(listed.iter().all(|(kind, ..)| kind == "Call"), "{listed:?}");
    assert_eq!(listed.len(), 5);
    assert_eq!(listed[0].2, "4:30");
    assert_eq!(listed[1].2, "4:39");
}

/// `--callees` lists the calls the declaration's own body makes, whatever
/// their confidence, and explains every absent target ID; usages and calls
/// owned by other code are excluded.
#[test]
fn callees_list_owned_calls_and_explain_absent_ids() {
    let scratch = scratch();
    let json = refs_json(&scratch, &["apply", "--callees", "app.ts"]);
    assert_eq!(json["direction"], "callees");
    let listed = rows(&json, "occurrences");
    assert_eq!(
        listed,
        vec![
            row("Call", "g", "10:25", "Scoped"),
            row("Call", "e", "11:32", "Scoped"),
            row("Call", "parse", "12:3", "Scoped"),
            row("Call", "missing", "13:3", "Unresolved"),
        ],
        "the dynamic `xs[0]()` has no call target"
    );
    let absent: Vec<Option<&str>> = json["files"][0]["occurrences"]
        .as_array()
        .expect("occurrences")
        .iter()
        .map(|row| row["absent_id"].as_str())
        .collect();
    assert_eq!(
        absent,
        vec![Some("binding_without_record"), Some("binding_without_record"), None, Some("unresolved")]
    );
    let coverage = json["files"][0]["coverage"].as_array().expect("coverage");
    assert!(
        coverage.iter().any(|region| region["owner"]["id"] == json["declaration"]["id"]),
        "the declaration's own region is reported: {coverage:?}"
    );

    let rust = refs_json(&scratch, &["tool", "--callees", "tools.rs"]);
    let absent: Vec<(String, Option<String>)> = rust["files"][0]["occurrences"]
        .as_array()
        .expect("occurrences")
        .iter()
        .map(|row| {
            (
                row["target"]["name"].as_str().expect("name").to_string(),
                row["absent_id"].as_str().map(String::from),
            )
        })
        .collect();
    assert_eq!(
        absent,
        vec![("slurp".to_string(), Some("import".to_string())), ("nothing".to_string(), Some("unresolved".to_string()))],
        "the parameter usages `n` are not calls"
    );
}

/// A `NameOnly` member call is a suggestion, in its own group, only for a
/// declaration among its candidates; it never becomes an established row,
/// and `--id` does not upgrade it.
#[test]
fn name_only_suggestions_stay_separate_and_an_id_cannot_upgrade_them() {
    let scratch = scratch();
    let json = refs_json(&scratch, &["app.ts#A.run", "app.ts"]);
    assert!(rows(&json, "occurrences").is_empty());
    assert_eq!(rows(&json, "name_only_suggestions"), vec![row("Call", "x.run", "4:23", "NameOnly")]);
    let suggestion = &json["files"][0]["name_only_suggestions"][0];
    assert_eq!(suggestion["absent_id"], "name_only");
    assert!(suggestion["target"].get("id").is_none());
    let candidates: Vec<&str> = suggestion["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .map(|candidate| candidate["qualified_name"].as_str().expect("candidate name"))
        .collect();
    assert_eq!(candidates, vec!["app.ts#A.run", "app.ts#B.run"]);

    let id = declaration_id(&json);
    let by_id = refs_json(&scratch, &["--id", &id, "app.ts"]);
    assert_eq!(by_id["selector"], serde_json::json!({ "id": id }));
    assert_eq!(by_id["files"], json["files"], "an ID selector reports the same evidence");

    // `parse` is no member, so the member call is no suggestion for it.
    let parse = refs_json(&scratch, &["parse", "app.ts"]);
    assert!(rows(&parse, "name_only_suggestions").is_empty());
}

/// The same name declared in two scanned files is ambiguous: a nonzero exit,
/// nothing on stdout, and every match's location, kind, name and ID on
/// stderr. A file prefix, `--id` or a narrower scan selects one.
#[test]
fn a_name_declared_in_two_files_is_an_error_listing_both() {
    let scratch = Scratch::new(&[
        ("one.ts", "export function shared() { shared(); }\n"),
        ("two.ts", "export function shared() { return 2; }\n"),
    ]);
    let output = scratch.hug(&["--json", "refs", "shared", "--no-cache"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "no partial output: {}", String::from_utf8_lossy(&output.stdout));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("`shared` names 2 declarations"), "{stderr}");
    assert!(stderr.contains("one.ts:1:8"), "{stderr}");
    assert!(stderr.contains("two.ts:1:8"), "{stderr}");
    let ids: Vec<&str> = stderr
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|word| word.len() == 64 && word.chars().all(|c| c.is_ascii_hexdigit()))
        .collect();
    assert_eq!(ids.len(), 2, "{stderr}");
    assert_ne!(ids[0], ids[1]);

    let text = scratch.hug(&["refs", "shared"]);
    assert_eq!(text.status.code(), Some(1));
    assert!(text.stdout.is_empty());

    let prefixed = refs_json(&scratch, &["one.ts#shared"]);
    assert_eq!(rows(&prefixed, "occurrences"), vec![row("Call", "shared", "1:28", "Scoped")]);
    let by_id = refs_json(&scratch, &["--id", ids[1]]);
    assert_eq!(by_id["files"][0]["file"], "two.ts");
    let narrowed = refs_json(&scratch, &["shared", "--exclude-files", "two.ts"]);
    assert_eq!(narrowed["files"][0]["file"], "one.ts");
}

/// Names are exact under smart case: never a substring, an all-lowercase
/// name ignores case, any uppercase letter makes it case-sensitive. IDs are
/// case-sensitive. Locals are selectable.
#[test]
fn names_are_exact_smart_case_and_ids_case_sensitive() {
    let scratch = Scratch::new(&[(
        "names.ts",
        "function widget() {}\nclass Widget {}\nfunction widgetize() { widget(); }\n",
    )]);
    let ambiguous = scratch.hug(&["refs", "widget"]);
    assert_eq!(ambiguous.status.code(), Some(1), "`widget` matches `widget` and `Widget`, not `widgetize`");
    let stderr = stderr_of(&ambiguous);
    assert!(stderr.contains("names 2 declarations"), "{stderr}");
    assert!(!stderr.contains("widgetize"), "{stderr}");

    let class = refs_json(&scratch, &["Widget"]);
    assert_eq!(class["declaration"]["kind"], "Class");
    let none = refs_json(&scratch, &["WIDGET"]);
    assert!(none["declaration"].is_null());

    // An all-lowercase qualified name is case-insensitive too; the listing's
    // ID selects the function.
    let function_id = stderr
        .lines()
        .find(|line| line.contains(" function "))
        .and_then(|line| line.split_whitespace().last())
        .expect("function row");
    let function = refs_json(&scratch, &["--id", function_id]);
    let id = declaration_id(&function);
    assert_eq!(id, function_id);
    assert_eq!(rows(&function, "occurrences"), vec![row("Call", "widget", "3:24", "Scoped")]);
    let upper = refs_json(&scratch, &["--id", &id.to_uppercase()]);
    assert!(upper["declaration"].is_null(), "an ID is compared case-sensitively");

    let tools = Scratch::new(&[("tools.rs", TOOLS_RS)]);
    let local = refs_json(&tools, &["n"]);
    assert_eq!(local["declaration"]["scope"], "Local");
    assert_eq!(
        rows(&local, "occurrences"),
        vec![
            row("Usage", "n", "3:11", "Scoped"),
            row("Usage", "n", "5:5", "Scoped"),
            row("Usage", "n", "5:9", "Scoped"),
        ]
    );
}

/// An unknown name or ID is a successful empty report that still states the
/// boundary; so is a declaration nothing refers to.
#[test]
fn empty_reports_succeed_and_state_the_boundary() {
    let scratch = scratch();
    for args in [vec!["nowhere"], vec!["--id", "0000"]] {
        let mut full = vec!["--json", "refs"];
        full.extend(args.iter().copied());
        let json = json_of(&scratch.stdout(&full));
        assert!(json["declaration"].is_null());
        assert_eq!(json["files"], serde_json::json!([]));
        assert!(json["boundary"].as_str().is_some_and(|text| text.starts_with(BOUNDARY)));
        assert_eq!(json["errors"], serde_json::json!([]));

        let mut text_args = vec!["refs"];
        text_args.extend(args.iter().copied());
        let text = scratch.stdout(&text_args);
        assert!(text.starts_with("No declaration "), "{text}");
        assert!(text.contains(BOUNDARY), "{text}");
    }

    let quiet = refs_json(&scratch, &["quiet", "app.ts"]);
    assert_eq!(quiet["declaration"]["identity"]["name"], "quiet");
    assert!(rows(&quiet, "occurrences").is_empty());
    let text = scratch.stdout(&["refs", "quiet", "app.ts"]);
    assert!(text.contains("(no references)"), "{text}");
    assert!(text.contains(BOUNDARY), "{text}");
}

/// A same-named call in another file is never attached: `other.ts` imports
/// and calls `parse`, and none of its occurrences appears in the report.
#[test]
fn a_cross_file_same_name_call_is_never_a_resolved_reference() {
    let scratch = scratch();
    let json = refs_json(&scratch, &["parse"]);
    let files: Vec<&str> = json["files"].as_array().expect("files").iter().map(|file| file["file"].as_str().expect("file")).collect();
    assert_eq!(files, vec!["app.ts"], "the import in other.ts is no declaration and its call is not listed");
    assert!(
        json["files"][0]["occurrences"]
            .as_array()
            .expect("occurrences")
            .iter()
            .all(|row| row["owner"]["qualified_name"].as_str().is_none_or(|owner| owner.starts_with("app.ts#")))
    );
    let callers = scratch.stdout(&["refs", "parse", "--callers"]);
    assert!(!callers.contains("other.ts"), "{callers}");
}

/// The envelope is versioned like the declaration reports' and ends with
/// `errors`; an unreadable file is reported there with exit status 1.
#[test]
fn the_json_envelope_is_versioned_and_ends_with_errors() {
    let scratch = scratch();
    let stdout = scratch.stdout(&["--json", "refs", "parse", "app.ts"]);
    assert!(
        stdout.starts_with(r#"{"report_schema_version":1,"schema_version":{"major":5,"minor":0},"root_dir":"#),
        "{}",
        &stdout[..120.min(stdout.len())]
    );
    assert!(stdout.ends_with(",\"errors\":[]}\n"), "`errors` closes the report");
    let json = json_of(&stdout);
    let keys: Vec<&str> = json.as_object().expect("object").keys().map(String::as_str).collect();
    let mut expected = vec![
        "boundary",
        "declaration",
        "direction",
        "errors",
        "files",
        "report_schema_version",
        "root_dir",
        "schema_version",
        "selector",
    ];
    expected.sort_unstable();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, expected);
    // The selected record is trimmed and relative like `--json` records.
    assert_eq!(json["declaration"]["source"]["file_path"], "app.ts");
    assert!(json["declaration"]["source"].get("declaration_text").is_none());

    let broken = Scratch::new(&[("ok.ts", "function fine() { fine(); }\n"), ("bad.ts", "")]);
    std::fs::write(broken.repo.path().join("bad.ts"), [0x66u8, 0xff, 0xfe, 0x0a]).expect("write");
    let output = broken.hug(&["--json", "refs", "fine"]);
    assert_eq!(output.status.code(), Some(1));
    let json = json_of(&String::from_utf8(output.stdout).expect("utf-8"));
    assert_eq!(json["errors"][0]["file"], "bad.ts");
    assert_eq!(json["declaration"]["identity"]["name"], "fine");
}

/// The text report: heading, boundary, established rows grouped by owner
/// with file-level code labeled by its span, and suggestions in their own
/// group.
#[test]
fn the_text_report_groups_rows_by_owner_and_labels_ownerless_code() {
    let scratch = scratch();
    let text = scratch.stdout(&["refs", "parse", "app.ts"]);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("References to "), "{text}");
    assert!(lines[0].contains("parse"), "{text}");
    assert!(lines[0].ends_with("app.ts:3:8"), "{text}");
    assert!(lines[1].starts_with(BOUNDARY), "{text}");
    let go = lines.iter().position(|line| line.trim() == "app.ts#go 4:1").expect("go heading");
    assert_eq!(lines[go + 1].trim(), "- Call parse 4:30");
    assert_eq!(lines[go + 2].trim(), "- Call parse 4:39");
    assert!(lines.iter().any(|line| line.trim() == "file-level code [1:1–15:1]"), "{text}");
    assert!(!text.contains("Name-only suggestions"), "{text}");

    let member = scratch.stdout(&["refs", "app.ts#A.run", "app.ts"]);
    assert!(member.contains("Name-only suggestions"), "{member}");
    assert!(!member.contains("Established"), "{member}");
    assert!(member.contains("- Call x.run 4:23 — NameOnly, candidates: app.ts#A.run, app.ts#B.run"), "{member}");

    let callees = scratch.stdout(&["refs", "tool", "--callees", "tools.rs"]);
    assert!(callees.contains("Callees of "), "{callees}");
    assert!(callees.contains("Scoped to an import"), "{callees}");
    assert!(callees.contains("Unresolved"), "{callees}");
}

/// A warm run answers from the cache with the same report.
#[test]
fn a_warm_run_hits_the_cache_with_the_same_report() {
    let scratch = scratch();
    let cold = scratch.hug(&["--json", "--verbose", "refs", "parse", "app.ts"]);
    let warm = scratch.hug(&["--json", "--verbose", "refs", "parse", "app.ts"]);
    assert_eq!(stdout_of(warm.clone(), &[]), stdout_of(cold, &[]));
    let stderr = stderr_of(&warm);
    assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses"), "{stderr}");
    assert!(stderr.contains("parsed 0 files"), "{stderr}");
}

/// A selector is required, `--callers` and `--callees` conflict, and every
/// format but text and `--json`, and every declaration selector, is
/// rejected before any file is read.
#[test]
fn missing_selectors_conflicts_and_unsupported_formats_are_rejected() {
    let scratch = scratch();
    let cases: [(&[&str], &str); 8] = [
        (&["refs"], "needs a declaration"),
        (&["refs", "parse", "--callers", "--callees"], "cannot be used with"),
        (&["--json-full", "refs", "parse"], "`--json-full`"),
        (&["refs", "parse", "--jsonl"], "--jsonl"),
        (&["refs", "parse", "--format", "table"], "--format"),
        (&["refs", "parse", "--count"], "--count"),
        (&["refs", "parse", "--kind", "function"], "--kind"),
        (&["refs", "parse", "--exclude-symbols", "x"], "--exclude-symbols"),
    ];
    for (args, expected) in cases {
        let output = scratch.hug(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {}", stderr_of(&output));
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(stderr_of(&output).contains(expected), "{args:?}: {}", stderr_of(&output));
    }
}

/// Lexical lookup stops where no binding is established: a call inside a
/// `with` body is only a suggestion and its function's recursion stays
/// unknown; an inline module's call never reaches its parent's function;
/// an `if let` binding is not the `else` branch's.
#[test]
fn boundaries_that_establish_no_binding_list_no_established_rows() {
    let scratch = Scratch::new(&[
        ("rec.js", "function f(obj) { with (obj) { f(obj); } }\n"),
        ("module.rs", "fn f() {}\nmod m { pub fn g() { f(); } }\n"),
        ("cond.rs", "fn f() {}\nfn g(x: Option<fn()>) {\n    if let Some(f) = x { f(); } else { f(); }\n    f();\n}\n"),
    ]);

    let with = refs_json(&scratch, &["f", "--callers", "rec.js"]);
    assert!(rows(&with, "occurrences").is_empty(), "{with}");
    assert_eq!(rows(&with, "name_only_suggestions"), vec![row("Call", "f", "1:32", "NameOnly")]);
    let full = json_of(&scratch.stdout(&["--json-full", "symbols", "rec.js"]));
    let recursion: Vec<&Value> = full["indexes"][0]["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .filter(|record| record["identity"]["name"] == "f")
        .map(|record| &record["semantics"]["is_recursive"])
        .collect();
    assert_eq!(recursion, vec![&Value::from("Unknown")], "a `with` body may intercept the self-call");

    let module = refs_json(&scratch, &["module.rs#f", "--callers", "module.rs"]);
    assert!(rows(&module, "occurrences").is_empty(), "{module}");
    assert_eq!(rows(&module, "name_only_suggestions"), vec![row("Call", "f", "2:22", "NameOnly")]);

    let conditional = refs_json(&scratch, &["cond.rs#f", "--callers", "cond.rs"]);
    assert_eq!(
        rows(&conditional, "occurrences"),
        vec![row("Call", "f", "3:40", "Scoped"), row("Call", "f", "4:5", "Scoped")],
        "the `else` call and the call after the `if`, never the consequence's"
    );
}

/// A Rust pattern name binds or reads a constant by lookup, never by its
/// spelling: a capitalized pattern named like the function binds a new
/// local, so the function has no caller and is not recursive, and a
/// lowercase constant's pattern and body names are both references to it.
#[test]
fn rust_pattern_names_never_invent_self_calls_or_hide_constant_reads() {
    let scratch = Scratch::new(&[
        (
            "upper.rs",
            "fn F(x: fn()) {\n    if let F = x { F(); }\n    while let F = x { F(); }\n    match x { F => F() }\n    \
             match x { mut F => F() }\n    match x { ref F => F() }\n}\n",
        ),
        (
            "lower.rs",
            "const f: u8 = 1;\npub fn g(x: u8) {\n    match x { f => { f; }, _ => {} }\n    if let f = x { f; }\n    \
             while let f = x { f; }\n    match x { f if f == 1 => {}, _ => {} }\n}\n",
        ),
    ]);

    let callers = refs_json(&scratch, &["F", "--callers", "upper.rs"]);
    assert!(rows(&callers, "occurrences").is_empty(), "{callers}");
    assert!(rows(&callers, "name_only_suggestions").is_empty(), "{callers}");
    let full = json_of(&scratch.stdout(&["--json-full", "symbols", "upper.rs"]));
    let recursion: Vec<&Value> = full["indexes"][0]["symbols"]
        .as_array()
        .expect("symbols")
        .iter()
        .filter(|record| record["identity"]["name"] == "F")
        .map(|record| &record["semantics"]["is_recursive"])
        .collect();
    assert_eq!(recursion, vec![&Value::from("Unknown")], "each call goes through a pattern local");

    let constant = refs_json(&scratch, &["f", "lower.rs"]);
    assert_eq!(
        rows(&constant, "occurrences"),
        vec![
            row("Usage", "f", "3:15", "Scoped"),
            row("Usage", "f", "3:22", "Scoped"),
            row("Usage", "f", "4:12", "Scoped"),
            row("Usage", "f", "4:20", "Scoped"),
            row("Usage", "f", "5:15", "Scoped"),
            row("Usage", "f", "5:23", "Scoped"),
            row("Usage", "f", "6:15", "Scoped"),
            row("Usage", "f", "6:20", "Scoped"),
        ],
        "every pattern and body `f` reads the constant: {constant}"
    );
}

/// `hug symbols` lists no declaration for a pattern name that binds
/// nothing (the rule `refs` follows): `Some(C)` reading a constant in
/// `if let`, `while let` and `match`, a struct shorthand reading one, a
/// refutable name an import may supply, and `Some` itself; `Some(x)` and
/// `Some(C)` with no constant in view are locals, `Some(mut C)` binds but
/// the locals query records no moded name.
#[test]
fn symbols_list_no_declaration_for_pattern_names_that_bind_nothing() {
    let scratch = Scratch::new(&[
        (
            "reads.rs",
            "const C: u8 = 1;\npub struct S {\n    pub C: u8,\n}\nmod m {\n    pub const D: u8 = 2;\n}\nuse m::D;\n\
             pub fn g(x: Option<u8>, s: S) {\n    if let Some(C) = x {}\n    while let Some(C) = x {\n        break;\n    }\n    \
             match x {\n        Some(C) => {}\n        Some(D) => {}\n        _ => {}\n    }\n    if let S { C } = s {}\n}\n",
        ),
        (
            "binds.rs",
            "pub fn h(x: Option<u8>) {\n    if let Some(y) = x {\n        y;\n    }\n    if let Some(C) = x {\n        C;\n    }\n    \
             if let Some(mut M) = x {\n        M += 1;\n        M;\n    }\n}\n",
        ),
    ]);

    let json = json_of(&scratch.stdout(&["--json", "symbols", "--all-scopes", "reads.rs", "binds.rs"]));
    let mut listed: Vec<(String, String, u64)> = json["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols").clone())
        .filter(|record| record["scope"] == "Local")
        .map(|record| {
            let file = record["source"]["file_path"].as_str().expect("file").to_string();
            let name = record["identity"]["name"].as_str().expect("name").to_string();
            (file, name, record["source"]["name_span"]["start"]["line"].as_u64().expect("line"))
        })
        .collect();
    listed.sort();
    let expected: Vec<(String, String, u64)> = [
        ("binds.rs", "C", 5),
        ("binds.rs", "x", 1),
        ("binds.rs", "y", 2),
        ("reads.rs", "s", 9),
        ("reads.rs", "x", 9),
    ]
    .into_iter()
    .map(|(file, name, line)| (file.to_string(), name.to_string(), line))
    .collect();
    assert_eq!(listed, expected, "{json}");
}

/// A named function or generator expression's own name is visible only
/// inside the expression: its inner call is never a caller of the outer
/// declaration of that name, and only the final outer call is established.
#[test]
fn named_function_expressions_never_call_the_outer_declaration() {
    let source = |star: &str| format!("function f() {{}}\nconst g = function{star} f() {{\n  f();\n}};\nf();\n");
    let (function, generator) = (source(""), source("*"));
    let scratch = Scratch::new(&[
        ("function.ts", function.as_str()),
        ("function.js", function.as_str()),
        ("generator.ts", generator.as_str()),
        ("generator.js", generator.as_str()),
    ]);

    for file in ["function.ts", "function.js", "generator.ts", "generator.js"] {
        let callers = refs_json(&scratch, &["f", "--callers", file]);
        assert_eq!(rows(&callers, "occurrences"), vec![row("Call", "f", "5:1", "Scoped")], "{file}: {callers}");
        assert!(rows(&callers, "name_only_suggestions").is_empty(), "{file}: {callers}");
    }
}

/// A parameter default is evaluated before the body and cannot see the
/// body's `var`: in every callable form, the default's call is a caller of
/// the outer `f` and a callee of the callable bound to that `f`, while the
/// body's call binds the body's own `f`.
#[test]
fn parameter_defaults_call_the_outer_declaration_not_a_body_var() {
    // (file stem, header, closing, the callable's selector); a function or
    // generator expression is an anonymous owner and a generator
    // declaration is not extracted, so neither has a callees report.
    let forms: &[(&str, &str, &str, Option<&str>)] = &[
        ("declaration", "function g(x = f()) {", "}", Some("g")),
        ("arrow", "const g = (x = f()) => {", "};", Some("g")),
        ("expression", "const g = function (x = f()) {", "};", None),
        ("generator", "function* g(x = f()) {", "}", None),
        ("generator_expression", "const g = function* (x = f()) {", "};", None),
        ("method", "class C { g(x = f()) {", "} }", Some("C.g")),
        ("constructor", "class C { constructor(x = f()) {", "} }", Some("C.constructor")),
    ];
    let sources: Vec<(String, String)> = forms
        .iter()
        .flat_map(|(stem, header, closing, _)| {
            let source = format!("function f() {{}}\n{header}\n  f();\n  var f = () => {{}};\n{closing}\nf();\n");
            ["ts", "js"].map(|extension| (format!("{stem}.{extension}"), source.clone()))
        })
        .collect();
    let files: Vec<(&str, &str)> = sources.iter().map(|(name, source)| (name.as_str(), source.as_str())).collect();
    let scratch = Scratch::new(&files);

    for (stem, header, _, callable) in forms {
        let column = header.find("f()").expect("default call") + 1;
        for extension in ["ts", "js"] {
            let file = format!("{stem}.{extension}");
            // `f` names the outer function and the body `var` (an anonymous
            // callable gives its locals no prefix): select the outer one by ID.
            let listing = scratch.hug(&["refs", "f", &file]);
            assert_eq!(listing.status.code(), Some(1), "{file}: two declarations named `f`");
            let listing = stderr_of(&listing);
            let outer = listing
                .lines()
                .find(|line| line.trim_start().starts_with(&format!("{file}:1:1 ")))
                .and_then(|line| line.split_whitespace().last())
                .unwrap_or_else(|| panic!("{file}: outer `f` in {listing}"))
                .to_string();
            let callers = refs_json(&scratch, &["--id", &outer, "--callers", &file]);
            assert_eq!(
                rows(&callers, "occurrences"),
                vec![row("Call", "f", &format!("2:{column}"), "Scoped"), row("Call", "f", "6:1", "Scoped")],
                "{file}: {callers}"
            );
            assert!(rows(&callers, "name_only_suggestions").is_empty(), "{file}: {callers}");

            let Some(callable) = callable else { continue };
            let callees = refs_json(&scratch, &[&format!("{file}#{callable}"), "--callees", &file]);
            let calls = callees["files"][0]["occurrences"].as_array().unwrap_or_else(|| panic!("{file}: {callees}"));
            let targets: Vec<(String, Option<&str>)> = calls
                .iter()
                .map(|call| {
                    let line = &call["span"]["start"]["line"];
                    (format!("{line}:{}", call["confidence"]), call["target"]["id"].as_str())
                })
                .collect();
            assert_eq!(targets.len(), 2, "{file}: {callees}");
            assert_eq!(targets[0], ("2:\"Scoped\"".to_string(), Some(outer.as_str())), "{file}: {callees}");
            assert_eq!(targets[1].0, "3:\"Scoped\"", "{file}: {callees}");
            assert!(targets[1].1.is_some_and(|id| id != outer), "the body call binds the body `var`: {callees}");
        }
    }
}

/// A call through transparent wrappers is a call in every report direction:
/// established in a resolved language, a name-only suggestion in a limited
/// one. A wrapped name passed as an argument stays a usage.
#[test]
fn wrapped_and_command_calls_are_listed_as_calls() {
    let scratch = Scratch::new(&[
        (
            "wrapped.ts",
            "function f(n: number) {\n  if (n) ((f))(n - 1);\n  (f as () => void)();\n  f!();\n  g((f));\n}\nfunction h() { (f)(); }\n",
        ),
        ("wrapped.py", "def f(n):\n    (f)(n)\n    g((f))\n\ndef h(x):\n    ((f))(x)\n"),
        ("tool.sh", "f() {\n  X=1 f arg\n}\ng() {\n  echo f\n  f\n}\n"),
    ]);

    let callers = refs_json(&scratch, &["f", "--callers", "wrapped.ts"]);
    assert_eq!(
        rows(&callers, "occurrences"),
        vec![
            row("Call", "f", "2:12", "Scoped"),
            row("Call", "f", "3:4", "Scoped"),
            row("Call", "f", "4:3", "Scoped"),
            row("Call", "f", "7:17", "Scoped"),
        ],
        "the argument `g((f))` is a usage: {callers}"
    );
    let callees = refs_json(&scratch, &["h", "--callees", "wrapped.ts"]);
    assert_eq!(rows(&callees, "occurrences"), vec![row("Call", "f", "7:17", "Scoped")]);

    let python = refs_json(&scratch, &["f", "--callers", "wrapped.py"]);
    assert!(rows(&python, "occurrences").is_empty(), "{python}");
    assert_eq!(
        rows(&python, "name_only_suggestions"),
        vec![row("Call", "f", "2:6", "NameOnly"), row("Call", "f", "6:7", "NameOnly")]
    );

    let shell = refs_json(&scratch, &["f", "--callers", "tool.sh"]);
    assert_eq!(
        rows(&shell, "name_only_suggestions"),
        vec![row("Call", "f", "2:7", "NameOnly"), row("Call", "f", "6:3", "NameOnly")],
        "`echo f` passes `f` as a word: {shell}"
    );
    let callees = refs_json(&scratch, &["g", "--callees", "tool.sh"]);
    assert_eq!(
        rows(&callees, "occurrences"),
        vec![row("Call", "echo", "5:3", "Unresolved"), row("Call", "f", "6:3", "NameOnly")]
    );
}

/// A TypeScript instantiation expression (`f<number>` written apart from
/// its call) is a call of its operand in both directions, behind every
/// transparent wrapper; a member instantiated that way is a name-only
/// suggestion, and an instantiation that is only a value or an argument is
/// no call.
#[test]
fn instantiation_expression_calls_are_listed_as_calls() {
    let scratch = Scratch::new(&[(
        "generic.ts",
        "function f<T>() { (f<number>)(); }\nclass W { m<T>() {} }\nclass C<T> {}\nfunction g(obj: W) {\n  ((f<number>))();\n  (f<number>)!();\n  (f<number> as () => void)();\n  (f<number> satisfies () => void)();\n  (<() => void>f<number>)();\n  new (C<number>)();\n  (obj.m<number>)();\n  const x = f<number>;\n  consume(f<number>);\n}\n",
    )]);

    let callers = refs_json(&scratch, &["f", "--callers", "generic.ts"]);
    assert_eq!(
        rows(&callers, "occurrences"),
        vec![
            row("Call", "f", "1:20", "Scoped"),
            row("Call", "f", "5:5", "Scoped"),
            row("Call", "f", "6:4", "Scoped"),
            row("Call", "f", "7:4", "Scoped"),
            row("Call", "f", "8:4", "Scoped"),
            row("Call", "f", "9:16", "Scoped"),
        ],
        "the value and argument instantiations are usages: {callers}"
    );

    let callees = refs_json(&scratch, &["g", "--callees", "generic.ts"]);
    assert_eq!(
        rows(&callees, "occurrences"),
        vec![
            row("Call", "f", "5:5", "Scoped"),
            row("Call", "f", "6:4", "Scoped"),
            row("Call", "f", "7:4", "Scoped"),
            row("Call", "f", "8:4", "Scoped"),
            row("Call", "f", "9:16", "Scoped"),
            row("Call", "C", "10:8", "Scoped"),
            row("Call", "obj.m", "11:8", "NameOnly"),
            row("Call", "consume", "13:3", "Unresolved"),
        ]
    );

    let member = refs_json(&scratch, &["generic.ts#W.m", "generic.ts"]);
    assert!(rows(&member, "occurrences").is_empty(), "{member}");
    assert_eq!(rows(&member, "name_only_suggestions"), vec![row("Call", "obj.m", "11:8", "NameOnly")]);
}

/// Calling a Rust value (`Self::C()` with `const C: fn() = Self::f`) is
/// listed as a call of the value, never of the function its initializer
/// names, and the caller's own region is limited by `indirect_dispatch`, in
/// JSON and text.
#[test]
fn value_calls_are_listed_against_the_value_with_limited_coverage() {
    let scratch = Scratch::new(&[(
        "values.rs",
        "struct S;\nimpl S {\n    const C: fn() = Self::f;\n    fn f() { Self::C(); }\n}\n",
    )]);
    let value_id = declaration_id(&refs_json(&scratch, &["values.rs#S::C", "values.rs"]));
    let id = declaration_id(&refs_json(&scratch, &["values.rs#S::f", "values.rs"]));

    let callees = refs_json(&scratch, &["--id", &id, "--callees", "values.rs"]);
    assert_eq!(rows(&callees, "occurrences"), vec![row("Call", "Self::C", "4:20", "Scoped")]);
    assert_eq!(callees["files"][0]["occurrences"][0]["target"]["id"], value_id.as_str());
    let coverage = callees["files"][0]["coverage"].as_array().expect("coverage");
    let own: Vec<&Value> = coverage.iter().filter(|region| region["owner"]["id"] == id.as_str()).collect();
    assert_eq!(own.len(), 1, "{coverage:?}");
    assert_eq!(own[0]["state"], "Limited", "{coverage:?}");
    assert_eq!(own[0]["reasons"], serde_json::json!(["indirect_dispatch"]), "{coverage:?}");

    let text = scratch.stdout(&["refs", "--id", &id, "--callees", "values.rs"]);
    assert!(text.contains("limited (indirect_dispatch)"), "{text}");
}

/// Only a direct `eval` may add a `var` to its function: after an
/// optional (`eval?.(…)`) or tagged (``eval`…` ``) call, the nested
/// self-call is an established caller row of the inner `f` (fresh and on
/// a warm cache), `f` is recursive and `D` extends `Base`'s ID; after a
/// direct `eval` there is no established row.
#[test]
fn optional_and_tagged_eval_keep_established_callers() {
    let source = |statement: &str| {
        format!(
            "class Base {{}}\nfunction outer() {{\n    function f() {{\n        {statement}\n        class D extends Base {{}}\n        f();\n    }}\n}}\n"
        )
    };
    let files = [
        ("optional.js", source("eval?.(\"var f = 1\");"), true),
        ("tagged.js", source("eval`var f = 1`;"), true),
        ("optional.ts", source("(eval as any)?.(\"var f = 1\");"), true),
        ("tagged.ts", source("(eval)`var f = 1`;"), true),
        ("direct.js", source("eval(\"var f = 1\");"), false),
        ("direct.ts", source("((eval))(\"var f = 1\");"), false),
    ];
    let scratch = Scratch::new(&files.iter().map(|(name, text, _)| (*name, text.as_str())).collect::<Vec<_>>());

    for (file, _, binds_nothing) in &files {
        let id = declaration_id(&refs_json(&scratch, &[&format!("{file}#outer.f"), file]));
        for run in ["fresh", "warm"] {
            let callers = refs_json(&scratch, &["--id", &id, "--callers", file]);
            if *binds_nothing {
                assert_eq!(rows(&callers, "occurrences"), vec![row("Call", "f", "6:9", "Scoped")], "{file} {run}: {callers}");
                assert_eq!(callers["files"][0]["occurrences"][0]["target"]["id"], id.as_str(), "{file} {run}");
                assert_eq!(callers["files"][0]["occurrences"][0]["owner"]["id"], id.as_str(), "{file} {run}");
            } else {
                assert!(rows(&callers, "occurrences").is_empty(), "{file} {run}: {callers}");
            }
            assert!(rows(&callers, "name_only_suggestions").is_empty(), "{file} {run}: {callers}");
        }

        let full = json_of(&scratch.stdout(&["--json-full", "symbols", file]));
        let symbols = full["indexes"][0]["symbols"].as_array().expect("symbols");
        let named = |name: &str| symbols.iter().find(|record| record["identity"]["name"] == name).expect("record");
        let expected = if *binds_nothing { "Yes" } else { "Unknown" };
        assert_eq!(named("f")["semantics"]["is_recursive"], expected, "{file}");
        let base = &named("D")["relations"]["extends"][0];
        if *binds_nothing {
            assert_eq!(base["id"], named("Base")["id"], "{file}: {base}");
        } else {
            assert!(base["id"].is_null(), "{file}: {base}");
        }
    }
}

/// A cached entry whose written path for `Self::h()` was changed to
/// `Other::h` is refused and recomputed, so the callees report shows the
/// path the source writes, in text and JSON; the unedited entry is a hit
/// with the same report.
#[test]
fn a_stored_written_path_the_source_does_not_write_is_recomputed() {
    let scratch = Scratch::new(&[("paths.rs", "struct S;\nimpl S {\n    fn h() {}\n    fn f() { Self::h(); }\n}\n")]);
    let id = declaration_id(&refs_json(&scratch, &["paths.rs#S::f", "paths.rs"]));
    let entry = crate::cache_upgrade::only_entry(scratch.temp.path());
    let written = crate::cache_upgrade::read_json(&entry);
    let mut edited = written.clone();
    let occurrence = edited["occurrences"]
        .as_array_mut()
        .expect("occurrences")
        .iter_mut()
        .find(|occurrence| occurrence["target"]["name"] == "h")
        .expect("the `h` call");
    assert_eq!(occurrence["target"]["qualified_name"], "Self::h");
    occurrence["target"]["qualified_name"] = Value::String("Other::h".into());
    let edited = serde_json::to_string(&edited).expect("serialize");

    for format in [None, Some("--json")] {
        std::fs::write(&entry, &edited).expect("write the edited entry");
        let mut args: Vec<&str> = format.into_iter().collect();
        args.extend(["--verbose", "refs", "--id", &id, "--callees", "paths.rs"]);
        let output = scratch.hug(&args);
        let stderr = stderr_of(&output);
        assert!(
            stderr.contains("inconsistent entry: an occurrence whose written path is not the source text ending at its name"),
            "{format:?}: {stderr}"
        );
        assert!(stderr.contains("cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{format:?}: {stderr}");
        let stdout = stdout_of(output, &args);
        assert!(!stdout.contains("Other::h"), "{format:?}: {stdout}");
        if format.is_some() {
            assert_eq!(rows(&json_of(&stdout), "occurrences"), vec![row("Call", "Self::h", "4:20", "Scoped")]);
        } else {
            assert!(stdout.lines().any(|line| line.trim_end().ends_with("Call Self::h 4:20")), "{stdout}");
        }

        // The recomputed entry is served unedited.
        let warm = scratch.hug(&args);
        assert!(stderr_of(&warm).contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"));
        assert_eq!(stdout_of(warm, &args), stdout);
    }
}

/// A cached entry whose written path for `other::external()` was set to
/// `null` (its stored start kept) is refused and recomputed, so the callees
/// report keeps the path in text and JSON, never the bare `external`; the
/// recomputed entry is then a hit with the same report.
#[test]
fn a_deleted_written_path_is_recomputed_for_callees() {
    let scratch = Scratch::new(&[("lost.rs", "fn g() { other::external(); }\n")]);
    refs_json(&scratch, &["g", "lost.rs"]);
    let entry = crate::cache_upgrade::only_entry(scratch.temp.path());
    let mut edited = crate::cache_upgrade::read_json(&entry);
    let occurrence = edited["occurrences"]
        .as_array_mut()
        .expect("occurrences")
        .iter_mut()
        .find(|occurrence| occurrence["target"]["name"] == "external")
        .expect("the `external` call");
    assert_eq!(occurrence["target"]["qualified_name"], "other::external");
    occurrence["target"]["qualified_name"] = Value::Null;
    let edited = serde_json::to_string(&edited).expect("serialize");

    for format in [None, Some("--json")] {
        std::fs::write(&entry, &edited).expect("write the edited entry");
        let mut args: Vec<&str> = format.into_iter().collect();
        args.extend(["--verbose", "refs", "g", "--callees", "lost.rs"]);
        let output = scratch.hug(&args);
        let stderr = stderr_of(&output);
        assert!(
            stderr.contains("inconsistent entry: an occurrence whose written path disagrees with its stored start"),
            "{format:?}: {stderr}"
        );
        assert!(stderr.contains("cache: 0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{format:?}: {stderr}");
        let stdout = stdout_of(output, &args);
        if format.is_some() {
            assert_eq!(rows(&json_of(&stdout), "occurrences"), vec![row("Call", "other::external", "1:17", "Unresolved")]);
        } else {
            assert!(stdout.lines().any(|line| line.contains("Call other::external 1:17")), "{stdout}");
            assert!(!stdout.lines().any(|line| line.contains("Call external ")), "{stdout}");
        }

        let warm = scratch.hug(&args);
        assert!(stderr_of(&warm).contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"));
        assert_eq!(stdout_of(warm, &args), stdout);
    }
}

/// Callees and callers show the full written target (`R4-F1`): roots,
/// right-nested and generic C++ paths, C# conditional receivers, PHP
/// nullsafe receivers and namespaces, Perl package and receiver paths, in
/// JSON and text. A rooted Rust self-call (`::f()`) is a name-only
/// suggestion of the root function, never an established row, and the
/// shadowing `let f` beside a rooted call has no row at all.
#[test]
fn callees_and_callers_show_the_full_written_target() {
    let scratch = Scratch::new(&[
        ("rooted.rs", "fn f() {\n    ::f();\n}\nfn g() {\n    let f = || {};\n    ::f();\n}\n"),
        (
            "paths.cpp",
            "void g() {\n    ::f();\n    A::B::f();\n    r4::f<int>();\n    obj.template m<int>();\n    N::C<int>::member();\n}\n",
        ),
        (
            "paths.cs",
            "class W { public void F() {} public System.Action A; }\nclass G { void g(W obj) {\n    obj?.F();\n    obj?.A?.Invoke();\n    global::N.C.M();\n} }\n",
        ),
        ("paths.php", "<?php\nfunction g($o) {\n    $o?->m();\n    \\Ns\\f();\n}\n"),
        ("paths.pl", "sub g {\n    R4::r4f();\n    R4->r4f();\n    $obj->r4f();\n}\n"),
    ]);

    let callees = |file: &str, name: &str| refs_json(&scratch, &[name, "--callees", file]);
    type Row = (String, String, String, String);
    // `(file, selector, callee rows)`.
    let expected: &[(&str, &str, Vec<Row>)] = &[
        ("rooted.rs", "rooted.rs#f", vec![row("Call", "::f", "2:7", "NameOnly")]),
        ("rooted.rs", "rooted.rs#g", vec![row("Call", "::f", "6:7", "NameOnly")]),
        (
            "paths.cpp",
            "g",
            vec![
                row("Call", "::f", "2:7", "Unresolved"),
                row("Call", "A::B::f", "3:11", "Unresolved"),
                row("Call", "r4::f", "4:9", "Unresolved"),
                row("Call", "obj.template m", "5:18", "Unresolved"),
                row("Call", "N::C<int>::member", "6:16", "Unresolved"),
            ],
        ),
        (
            "paths.cs",
            "paths.cs#G.g",
            vec![
                row("Call", "obj?.F", "3:10", "NameOnly"),
                row("Call", "obj?.A?.Invoke", "4:13", "Unresolved"),
                row("Call", "global::N.C.M", "5:17", "Unresolved"),
            ],
        ),
        (
            "paths.php",
            "g",
            vec![row("Call", "$o?->m", "3:10", "Unresolved"), row("Call", "\\Ns\\f", "4:9", "Unresolved")],
        ),
        (
            "paths.pl",
            "g",
            vec![
                row("Call", "R4::r4f", "2:9", "Unresolved"),
                row("Call", "R4->r4f", "3:9", "Unresolved"),
                row("Call", "$obj->r4f", "4:11", "Unresolved"),
            ],
        ),
    ];
    for (file, name, rows_expected) in expected {
        let json = callees(file, name);
        assert_eq!(&rows(&json, "occurrences"), rows_expected, "{file} {name} fresh: {json}");
        let args = ["--verbose", "--json", "refs", name, "--callees", file];
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
        let json = json_of(&stdout_of(warm, &args));
        assert_eq!(&rows(&json, "occurrences"), rows_expected, "{file} {name} warm: {json}");
        let text = scratch.stdout(&["refs", name, "--callees", file]);
        for (kind, written, at, _) in rows_expected {
            let listed = format!("{kind} {written} {at}");
            assert!(
                text.lines().any(|line| {
                    let line = line.trim_end();
                    line.ends_with(&listed) || line.contains(&format!("{listed} —"))
                }),
                "{file} {name}: `{listed}` in {text}"
            );
        }
    }

    // The rooted calls suggest the root function only; nothing is
    // established, and the local `let f` is never a target or candidate.
    let callers = refs_json(&scratch, &["rooted.rs#f", "--callers", "rooted.rs"]);
    assert!(rows(&callers, "occurrences").is_empty(), "{callers}");
    assert_eq!(
        rows(&callers, "name_only_suggestions"),
        vec![row("Call", "::f", "2:7", "NameOnly"), row("Call", "::f", "6:7", "NameOnly")]
    );
    let local = refs_json(&scratch, &["rooted.rs#g::f", "rooted.rs"]);
    assert!(rows(&local, "occurrences").is_empty(), "{local}");
    assert!(rows(&local, "name_only_suggestions").is_empty(), "{local}");
    let text = scratch.stdout(&["refs", "rooted.rs#g::f", "rooted.rs"]);
    assert!(!text.contains("::f 6:7"), "{text}");

    let full = json_of(&scratch.stdout(&["--json-full", "symbols", "rooted.rs"]));
    let symbols = full["indexes"][0]["symbols"].as_array().expect("symbols");
    let recursion = |name: &str| {
        symbols
            .iter()
            .find(|record| record["identity"]["name"] == name && record["kind"] == "Function")
            .map(|record| record["semantics"]["is_recursive"].clone())
    };
    assert_eq!(recursion("f"), Some(Value::String("Unknown".into())), "never a false `No`");
    assert_eq!(recursion("g"), Some(Value::String("No".into())));
}

/// Invocation kinds in both directions (`R4-F2`): rooted wrapped calls,
/// C# conditional member calls, PHP variable calls and dynamic
/// construction, and Perl code-reference calls are listed as calls; PHP
/// callable creation (`f(...)`) is no call, so it is absent from callees
/// and callers and stays a usage in the default report. Cold and warm
/// JSON, and text.
#[test]
fn invocation_kinds_decide_callees_callers_and_the_default_report() {
    let scratch = Scratch::new(&[
        ("rooted.rs", "fn f() {}\nfn g() {\n    (::f)();\n    ((::f))();\n}\n"),
        ("rooted.cpp", "void f() {}\nvoid g() {\n    (::f)();\n    ((::f))();\n}\n"),
        (
            "calls.cs",
            "class W { public void F() {} public void M<T>() {} public System.Action A; }\nclass G { void g(W obj) {\n    obj?.F();\n    obj?.M<int>();\n    obj?.A();\n    obj?.A?.Invoke();\n} }\n",
        ),
        (
            "calls.php",
            "<?php\nfunction f() {}\nfunction g($cb, $o, $args) {\n    f(...);\n    consume(f(...));\n    $o->m(...);\n    $cb(...);\n    $cb();\n    ($cb)();\n    new $C();\n    f(...$args);\n}\n",
        ),
        ("calls.pl", "sub g {\n    $cb->();\n    &$cb();\n    $cb->m();\n}\n"),
    ]);
    type Row = (String, String, String, String);
    // `(file, selector, direction, key, rows)`.
    let expected: Vec<(&str, &str, &str, &str, Vec<Row>)> = vec![
        (
            "rooted.rs",
            "g",
            "--callees",
            "occurrences",
            vec![row("Call", "::f", "3:8", "NameOnly"), row("Call", "::f", "4:9", "NameOnly")],
        ),
        (
            "rooted.rs",
            "f",
            "--callers",
            "name_only_suggestions",
            vec![row("Call", "::f", "3:8", "NameOnly"), row("Call", "::f", "4:9", "NameOnly")],
        ),
        (
            "rooted.cpp",
            "g",
            "--callees",
            "occurrences",
            vec![row("Call", "::f", "3:8", "NameOnly"), row("Call", "::f", "4:9", "NameOnly")],
        ),
        (
            "calls.cs",
            "calls.cs#G.g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "obj?.F", "3:10", "NameOnly"),
                row("Call", "obj?.M", "4:10", "NameOnly"),
                row("Call", "obj?.A", "5:10", "NameOnly"),
                row("Call", "obj?.A?.Invoke", "6:13", "Unresolved"),
            ],
        ),
        (
            "calls.cs",
            "calls.cs#W.F",
            "--callers",
            "name_only_suggestions",
            vec![row("Call", "obj?.F", "3:10", "NameOnly")],
        ),
        (
            "calls.cs",
            "calls.cs#W.A",
            "--all",
            "name_only_suggestions",
            vec![row("Call", "obj?.A", "5:10", "NameOnly"), row("Usage", "obj?.A", "6:10", "NameOnly")],
        ),
        (
            "calls.php",
            "g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "consume", "5:5", "Unresolved"),
                row("Call", "cb", "8:6", "NameOnly"),
                row("Call", "cb", "9:7", "NameOnly"),
                row("Call", "C", "10:10", "Unresolved"),
                row("Call", "f", "11:5", "NameOnly"),
            ],
        ),
        (
            "calls.php",
            "f",
            "--callers",
            "name_only_suggestions",
            vec![row("Call", "f", "11:5", "NameOnly")],
        ),
        (
            "calls.php",
            "f",
            "--all",
            "name_only_suggestions",
            vec![
                row("Usage", "f", "4:5", "NameOnly"),
                row("Usage", "f", "5:13", "NameOnly"),
                row("Call", "f", "11:5", "NameOnly"),
            ],
        ),
        (
            "calls.pl",
            "g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "$cb", "2:5", "Unresolved"),
                row("Call", "$cb", "3:6", "Unresolved"),
                row("Call", "$cb->m", "4:10", "Unresolved"),
            ],
        ),
    ];
    for (file, name, direction, key, rows_expected) in &expected {
        let mut args = vec!["--verbose", "--json", "refs", name];
        if *direction != "--all" {
            args.push(direction);
        }
        args.push(file);
        let cold = json_of(&scratch.stdout(&args));
        assert_eq!(&rows(&cold, key), rows_expected, "{file} {name} {direction} cold: {cold}");
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
        let warm = json_of(&stdout_of(warm, &args));
        assert_eq!(&rows(&warm, key), rows_expected, "{file} {name} {direction} warm: {warm}");

        let text_args: Vec<&str> = args.iter().copied().filter(|arg| !matches!(*arg, "--verbose" | "--json")).collect();
        let text = scratch.stdout(&text_args);
        for (kind, written, at, _) in rows_expected {
            let listed = format!("{kind} {written} {at}");
            assert!(
                text.lines().any(|line| line.trim_end().ends_with(&listed) || line.contains(&format!("{listed} —"))),
                "{file} {name} {direction}: `{listed}` in {text}"
            );
        }
        // PHP callable creation (lines 4–7) is never a call row.
        if *file == "calls.php" && *direction != "--all" {
            for at in ["4:5", "5:13", "6:9", "7:6"] {
                assert!(!text.contains(&format!(" {at}")), "{name} {direction}: `{at}` listed in {text}");
            }
        }
    }
}

/// Swift force-unwrapped and C# null-forgiving invocations (`R6-F1`) at
/// several grouping depths, bare, through a member and after a conditional
/// access, are callees of their owner and calls among the invoked
/// declaration's suggestions, with their written paths; an unwrapped value
/// that is not invoked is a usage, listed only in the default report.
/// ID-selected, cold and warm JSON, and text.
#[test]
fn null_assertion_invocations_are_callees_and_callers() {
    let scratch = Scratch::new(&[
        (
            "unwrap.swift",
            "class X { var cb: (() -> Void)? }\nfunc g(cb: (() -> Void)??, x: X) {\n    (cb!)!()\n    ((cb!)!)()\n    x.cb!()\n    (x.cb!)?()\n    _ = x.cb!\n}\n",
        ),
        (
            "forgiving.cs",
            "class G {\n    System.Action F;\n    void g(System.Action cb, G obj) {\n        cb!();\n        ((cb!)!)();\n        ((obj.F)!)();\n        obj?.F!();\n        obj.F!!();\n        var v = obj.F!;\n    }\n}\n",
        ),
    ]);
    type Row = (String, String, String, String);
    // `(file, selector resolved to an ID, direction, key, rows)`.
    let expected: Vec<(&str, &str, &str, &str, Vec<Row>)> = vec![
        (
            "unwrap.swift",
            "unwrap.swift#g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "cb", "3:6", "NameOnly"),
                row("Call", "cb", "4:7", "NameOnly"),
                row("Call", "x.cb", "5:7", "NameOnly"),
                row("Call", "x.cb", "6:8", "NameOnly"),
            ],
        ),
        (
            "unwrap.swift",
            "unwrap.swift#X.cb",
            "--callers",
            "name_only_suggestions",
            vec![
                row("Call", "cb", "3:6", "NameOnly"),
                row("Call", "cb", "4:7", "NameOnly"),
                row("Call", "x.cb", "5:7", "NameOnly"),
                row("Call", "x.cb", "6:8", "NameOnly"),
            ],
        ),
        (
            "unwrap.swift",
            "unwrap.swift#X.cb",
            "--all",
            "name_only_suggestions",
            vec![
                row("Call", "cb", "3:6", "NameOnly"),
                row("Call", "cb", "4:7", "NameOnly"),
                row("Call", "x.cb", "5:7", "NameOnly"),
                row("Call", "x.cb", "6:8", "NameOnly"),
                row("Usage", "x.cb", "7:11", "NameOnly"),
            ],
        ),
        (
            "forgiving.cs",
            "forgiving.cs#G.g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "cb", "4:9", "NameOnly"),
                row("Call", "cb", "5:11", "NameOnly"),
                row("Call", "obj.F", "6:15", "NameOnly"),
                row("Call", "obj?.F", "7:14", "NameOnly"),
                row("Call", "obj.F", "8:13", "NameOnly"),
            ],
        ),
        (
            "forgiving.cs",
            "forgiving.cs#G.F",
            "--callers",
            "name_only_suggestions",
            vec![
                row("Call", "obj.F", "6:15", "NameOnly"),
                row("Call", "obj?.F", "7:14", "NameOnly"),
                row("Call", "obj.F", "8:13", "NameOnly"),
            ],
        ),
        (
            "forgiving.cs",
            "forgiving.cs#G.F",
            "--all",
            "name_only_suggestions",
            vec![
                row("Call", "obj.F", "6:15", "NameOnly"),
                row("Call", "obj?.F", "7:14", "NameOnly"),
                row("Call", "obj.F", "8:13", "NameOnly"),
                row("Usage", "obj.F", "9:21", "NameOnly"),
            ],
        ),
    ];
    for (file, selector, direction, key, rows_expected) in &expected {
        let id = declaration_id(&refs_json(&scratch, &[selector, file]));
        let mut args = vec!["--verbose", "--json", "refs", "--id", &id];
        if *direction != "--all" {
            args.push(direction);
        }
        args.push(file);
        // The selector lookup above wrote the entries; remove them so this
        // run is cold.
        for entry in crate::cache_entries(scratch.temp.path()) {
            std::fs::remove_file(entry).expect("remove an entry");
        }
        let cold = scratch.hug(&args);
        assert!(stderr_of(&cold).contains("0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{file}: {}", stderr_of(&cold));
        let cold = json_of(&stdout_of(cold, &args));
        assert_eq!(&rows(&cold, key), rows_expected, "{file} {selector} {direction} cold: {cold}");
        let other = if *key == "occurrences" { "name_only_suggestions" } else { "occurrences" };
        assert!(rows(&cold, other).is_empty(), "{file} {selector} {direction}: {cold}");
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
        let warm = json_of(&stdout_of(warm, &args));
        assert_eq!(warm, cold, "{file} {selector} {direction} warm");

        let text_args: Vec<&str> = args.iter().copied().filter(|arg| !matches!(*arg, "--verbose" | "--json")).collect();
        let text = scratch.stdout(&text_args);
        for (kind, written, at, _) in rows_expected {
            let listed = format!("{kind} {written} {at}");
            assert!(
                text.lines().any(|line| line.trim_end().ends_with(&listed) || line.contains(&format!("{listed} —"))),
                "{file} {selector} {direction}: `{listed}` in {text}"
            );
        }
    }
}

/// Construction and implicit-member invocations (`R5-F1`): Java and Scala
/// generic construction, Swift specialized construction and implicit
/// members, C++ template allocation and braced construction are callees of
/// their owner and calls among the constructed type's suggestions; type
/// arguments, qualifiers, receivers, arguments, annotations, an implicit
/// member value and array creation are never call rows. Cold and warm
/// JSON, and text.
#[test]
fn constructed_types_and_implicit_members_are_callees_and_callers() {
    let scratch = Scratch::new(&[
        (
            "construct.java",
            "class C<T> {}\nclass G {\n    void g(Object y) {\n        new C<String>(y);\n        new a.b.C<>();\n        outer.new C<String>() {};\n        C<String> x = null;\n        Object w = new C<String>[2];\n    }\n}\n",
        ),
        (
            "construct.scala",
            "class C[T]\nobject G {\n  def g(y: Int): Unit = {\n    new C[Int](y)\n    new a.b.C[Int] {}\n    new C[Int] with T\n    val x: C[Int] = null\n  }\n}\n",
        ),
        (
            "construct.swift",
            "class C<T> {\n    static func f() -> C<T> { fatalError() }\n}\nfunc g(x: Int) {\n    _ = C<Int>()\n    _ = N.C<Int>()\n    let _: C<Int> = .f()\n    let _: C<Int> = .f\n    let _: E = .some(x)\n}\n",
        ),
        (
            "construct.cpp",
            "template <typename T> struct C {};\nstruct Plain {};\nvoid g(int x) {\n    new C<int>();\n    new N::C<int>;\n    Plain{x};\n    N::C<int>{};\n    C<int> v{};\n    new Plain[2];\n}\n",
        ),
    ]);
    type Row = (String, String, String, String);
    // `(file, selector, direction, key, rows)`.
    let expected: Vec<(&str, &str, &str, &str, Vec<Row>)> = vec![
        (
            "construct.java",
            "construct.java#G.g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "C", "4:13", "NameOnly"),
                row("Call", "a.b.C", "5:17", "NameOnly"),
                row("Call", "outer.new C", "6:19", "NameOnly"),
            ],
        ),
        (
            "construct.java",
            "construct.java#C",
            "--callers",
            "name_only_suggestions",
            vec![
                row("Call", "C", "4:13", "NameOnly"),
                row("Call", "a.b.C", "5:17", "NameOnly"),
                row("Call", "outer.new C", "6:19", "NameOnly"),
            ],
        ),
        (
            "construct.java",
            "construct.java#C",
            "--all",
            "name_only_suggestions",
            vec![
                row("Call", "C", "4:13", "NameOnly"),
                row("Call", "a.b.C", "5:17", "NameOnly"),
                row("Call", "outer.new C", "6:19", "NameOnly"),
                row("Usage", "C", "7:9", "NameOnly"),
                row("Usage", "C", "8:24", "NameOnly"),
            ],
        ),
        (
            "construct.scala",
            "construct.scala#G.g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "C", "4:9", "NameOnly"),
                row("Call", "a.b.C", "5:13", "NameOnly"),
                row("Call", "C", "6:9", "NameOnly"),
            ],
        ),
        (
            "construct.scala",
            "construct.scala#C",
            "--all",
            "name_only_suggestions",
            vec![
                row("Call", "C", "4:9", "NameOnly"),
                row("Call", "a.b.C", "5:13", "NameOnly"),
                row("Call", "C", "6:9", "NameOnly"),
                row("Usage", "C", "7:12", "NameOnly"),
            ],
        ),
        (
            "construct.swift",
            "construct.swift#g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "C", "5:9", "NameOnly"),
                row("Call", "N.C", "6:11", "NameOnly"),
                row("Call", "f", "7:22", "NameOnly"),
                row("Call", "some", "9:17", "Unresolved"),
            ],
        ),
        (
            "construct.swift",
            "construct.swift#C.f",
            "--all",
            "name_only_suggestions",
            vec![row("Call", "f", "7:22", "NameOnly"), row("Usage", "f", "8:22", "NameOnly")],
        ),
        (
            "construct.cpp",
            "construct.cpp#g",
            "--callees",
            "occurrences",
            vec![
                row("Call", "C", "4:9", "NameOnly"),
                row("Call", "N::C", "5:12", "NameOnly"),
                row("Call", "Plain", "6:5", "NameOnly"),
                row("Call", "N::C", "7:8", "NameOnly"),
            ],
        ),
        (
            "construct.cpp",
            "construct.cpp#Plain",
            "--all",
            "name_only_suggestions",
            vec![row("Call", "Plain", "6:5", "NameOnly"), row("Usage", "Plain", "9:9", "NameOnly")],
        ),
    ];
    for (file, name, direction, key, rows_expected) in &expected {
        let mut args = vec!["--verbose", "--json", "refs", name];
        if *direction != "--all" {
            args.push(direction);
        }
        args.push(file);
        let cold = json_of(&scratch.stdout(&args));
        assert_eq!(&rows(&cold, key), rows_expected, "{file} {name} {direction} cold: {cold}");
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
        let warm = json_of(&stdout_of(warm, &args));
        assert_eq!(&rows(&warm, key), rows_expected, "{file} {name} {direction} warm: {warm}");

        let text_args: Vec<&str> = args.iter().copied().filter(|arg| !matches!(*arg, "--verbose" | "--json")).collect();
        let text = scratch.stdout(&text_args);
        for (kind, written, at, _) in rows_expected {
            let listed = format!("{kind} {written} {at}");
            assert!(
                text.lines().any(|line| line.trim_end().ends_with(&listed) || line.contains(&format!("{listed} —"))),
                "{file} {name} {direction}: `{listed}` in {text}"
            );
        }
    }
}

/// Java receivers written before `new` (`R5-F3`): every report row shows
/// the full written target (`outer.new C`, `(outer).new C`, `this.new C`,
/// `factory().new C`, `holder.outer.new C`), type arguments and an anonymous
/// body excluded, in the default report, `--callers` and `--callees`, cold
/// and warm JSON and text; the receiver call `factory()` stays its own row
/// and the constructed type stays a `NameOnly` suggestion.
#[test]
fn receiver_qualified_constructions_report_their_written_target() {
    let scratch = Scratch::new(&[(
        "receivers.java",
        "class G {\n    class C {}\n    void g(G outer) {\n        outer.new C();\n        (outer).new C();\n        this.new C();\n        factory().new C();\n        holder.outer.new C();\n        outer.new C<String>();\n        outer.new C() {};\n    }\n    G factory() { return this; }\n}\n",
    )]);
    let constructed = vec![
        row("Call", "outer.new C", "4:19", "NameOnly"),
        row("Call", "(outer).new C", "5:21", "NameOnly"),
        row("Call", "this.new C", "6:18", "NameOnly"),
        row("Call", "factory().new C", "7:23", "NameOnly"),
        row("Call", "holder.outer.new C", "8:26", "NameOnly"),
        row("Call", "outer.new C", "9:19", "NameOnly"),
        row("Call", "outer.new C", "10:19", "NameOnly"),
    ];
    let mut callees = constructed.clone();
    callees.insert(3, row("Call", "factory", "7:9", "NameOnly"));
    type Row = (String, String, String, String);
    // `(selector, direction, key, rows)`.
    let expected: [(&str, &str, &str, Vec<Row>); 3] = [
        ("receivers.java#G.C", "--all", "name_only_suggestions", constructed.clone()),
        ("receivers.java#G.C", "--callers", "name_only_suggestions", constructed),
        ("receivers.java#G.g", "--callees", "occurrences", callees),
    ];
    for (name, direction, key, rows_expected) in &expected {
        let mut args = vec!["--verbose", "--json", "refs", name];
        if *direction != "--all" {
            args.push(direction);
        }
        args.push("receivers.java");
        let cold = json_of(&scratch.stdout(&args));
        assert_eq!(&rows(&cold, key), rows_expected, "{name} {direction} cold: {cold}");
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{name}: {stderr}");
        let warm = json_of(&stdout_of(warm, &args));
        assert_eq!(&rows(&warm, key), rows_expected, "{name} {direction} warm: {warm}");

        let text_args: Vec<&str> = args.iter().copied().filter(|arg| !matches!(*arg, "--verbose" | "--json")).collect();
        let text = scratch.stdout(&text_args);
        for (kind, written, at, _) in rows_expected {
            let listed = format!("{kind} {written} {at}");
            assert!(
                text.lines().any(|line| line.trim_end().ends_with(&listed) || line.contains(&format!("{listed} —"))),
                "{name} {direction}: `{listed}` in {text}"
            );
        }
    }
}

/// A parameter's default value and type are references, never parameters:
/// selecting the real `external` or `W` is unambiguous, their reads are
/// listed as suggestions against them alone, the parameter lists name only
/// the real parameters, and two real declarations are still ambiguous.
#[test]
fn parameter_defaults_and_types_are_references_not_declarations() {
    let scratch = Scratch::new(&[
        ("defaults.py", "def external(): pass\ndef g(x=external, y=external()): pass\n"),
        ("types.cs", "class W {}\nclass G { void g(W obj) {} }\n"),
    ]);
    let candidate_names = |json: &Value| -> Vec<Vec<String>> {
        json["files"][0]["name_only_suggestions"]
            .as_array()
            .expect("suggestions")
            .iter()
            .map(|row| {
                row["candidates"]
                    .as_array()
                    .expect("candidates")
                    .iter()
                    .map(|candidate| candidate["qualified_name"].as_str().expect("candidate").to_string())
                    .collect()
            })
            .collect()
    };

    let external = refs_json(&scratch, &["external", "--no-cache"]);
    assert_eq!(external["declaration"]["kind"], "Function", "{external}");
    assert_eq!(
        rows(&external, "name_only_suggestions"),
        vec![row("Usage", "external", "2:9", "NameOnly"), row("Call", "external", "2:21", "NameOnly")]
    );
    assert_eq!(candidate_names(&external), vec![vec!["defaults.py#external"]; 2]);

    let w = refs_json(&scratch, &["W", "--no-cache"]);
    assert_eq!(w["declaration"]["kind"], "Class", "{w}");
    assert_eq!(rows(&w, "name_only_suggestions"), vec![row("Usage", "W", "2:18", "NameOnly")]);
    assert_eq!(candidate_names(&w), vec![vec!["types.cs#W"]]);

    let json = json_of(&scratch.stdout(&["--json", "symbols", "--all-scopes", "--no-cache"]));
    let mut parameters: Vec<(String, String)> = json["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols").clone())
        .filter(|record| record["kind"] == "Parameter")
        .map(|record| {
            let file = record["source"]["file_path"].as_str().expect("file").to_string();
            (file, record["identity"]["name"].as_str().expect("name").to_string())
        })
        .collect();
    parameters.sort();
    let expected: Vec<(String, String)> = [("defaults.py", "x"), ("defaults.py", "y"), ("types.cs", "obj")]
        .into_iter()
        .map(|(file, name)| (file.to_string(), name.to_string()))
        .collect();
    assert_eq!(parameters, expected, "{json}");

    let listing = scratch.stdout(&["--plain", "functions", "types.cs", "--no-cache"]);
    assert!(listing.contains("g(obj: W)"), "{listing}");
    assert!(!listing.contains("W: W"), "{listing}");

    let scratch = Scratch::new(&[
        ("defaults.py", "def external(): pass\ndef g(x=external): pass\n"),
        ("other.py", "def external(): pass\n"),
    ]);
    let output = scratch.hug(&["--json", "refs", "external", "--no-cache"]);
    assert_eq!(output.status.code(), Some(1), "two real declarations stay ambiguous");
    assert!(stderr_of(&output).contains("`external` names 2 declarations"), "{}", stderr_of(&output));
    let narrowed = refs_json(&scratch, &["external", "defaults.py", "--no-cache"]);
    assert_eq!(rows(&narrowed, "name_only_suggestions"), vec![row("Usage", "external", "2:9", "NameOnly")]);
}

/// A generic parameter's declaration token is no reference to a same-named
/// class: the default report for `T` lists only the two type reads (bytes
/// 24 and 28) as suggestions, never the declaration `<T>` (byte 19), and the
/// call-only direction lists nothing.
#[test]
fn unreported_declaration_tokens_are_never_suggestions() {
    let source = "class T {} class G<T> { T f(T x){ return x; } }\n";
    let scratch = Scratch::new(&[("Generic.java", source)]);

    let json = refs_json(&scratch, &["T", "--no-cache"]);
    assert_eq!(json["declaration"]["kind"], "Class", "{json}");
    assert_eq!(
        rows(&json, "name_only_suggestions"),
        vec![row("Usage", "T", "1:25", "NameOnly"), row("Usage", "T", "1:29", "NameOnly")]
    );
    let bytes: Vec<u64> = json["files"][0]["name_only_suggestions"]
        .as_array()
        .expect("suggestions")
        .iter()
        .map(|row| row["span"]["start_byte"].as_u64().expect("start byte"))
        .collect();
    assert_eq!(bytes, vec![24, 28], "{json}");
    assert_eq!(&source[19..20], "T", "byte 19 is the declaration token");
    assert_eq!(rows(&json, "occurrences"), Vec::new(), "{json}");

    let callers = refs_json(&scratch, &["T", "--callers", "--no-cache"]);
    assert_eq!(rows(&callers, "name_only_suggestions"), Vec::new(), "{callers}");
}

/// A declaration token no record reports, or one component of a declared
/// path, is no reference (`R7-F2.C5`): the ID-selected default report for
/// the outer Java class `A` lists only the annotation read `@A` (2:59) as a
/// suggestion, never the nested annotation type's own name (2:22), and the
/// one for the TypeScript class `A` lists only the type read (1:58) as a
/// `Scoped` reference, never the namespace prefix `A.B` (1:22); cold and on
/// a verified warm hit, in JSON and text.
#[test]
fn unreported_and_compound_declaration_names_are_never_references() {
    let scratch = Scratch::new(&[
        ("Annotation.java", "class A {}\nclass G { @interface A { int value() default external; } @A void g() {} }\n"),
        ("namespace.ts", "class A {} namespace A.B { export function f(){} } let x:A;\n"),
    ]);
    // `(file, the incoming rows, the suggestions, the declaration positions never listed)`.
    let expected = [
        ("Annotation.java", Vec::new(), vec![row("Usage", "A", "2:59", "NameOnly")], "2:22"),
        ("namespace.ts", vec![row("Usage", "A", "1:58", "Scoped")], Vec::new(), "1:22"),
    ];
    for (file, incoming, suggestions, declared) in &expected {
        let id = declaration_id(&refs_json(&scratch, &["A", file, "--no-cache"]));
        let args = ["--verbose", "--json", "refs", "--id", id.as_str(), file];
        for entry in crate::cache_entries(scratch.temp.path()) {
            std::fs::remove_file(entry).expect("remove an entry");
        }
        let cold = scratch.hug(&args);
        assert!(stderr_of(&cold).contains("0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{file}: {}", stderr_of(&cold));
        let cold = json_of(&stdout_of(cold, &args));
        assert_eq!(&rows(&cold, "occurrences"), incoming, "{file} cold: {cold}");
        assert_eq!(&rows(&cold, "name_only_suggestions"), suggestions, "{file} cold: {cold}");
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
        let warm = json_of(&stdout_of(warm, &args));
        assert_eq!(warm, cold, "{file} warm");

        let text = scratch.stdout(&["--plain", "refs", "--id", id.as_str(), file]);
        for (kind, written, at, _) in incoming {
            let listed = format!("- {kind} {written} {at}\n");
            assert!(text.contains(&listed), "{file}: `{listed}` in {text}");
        }
        for (kind, written, at, _) in suggestions {
            let listed = format!("{kind} {written} {at} —");
            assert!(text.contains(&listed), "{file}: `{listed}` in {text}");
        }
        assert!(!text.contains(&format!(" {declared}")), "{file}: declaration {declared} listed in {text}");
    }
}

/// A Swift selective import is listed by the name it binds with its path
/// as the source (`R7-F3.S7`); a whole-module import keeps its path.
#[test]
fn swift_selective_imports_are_listed_by_their_bound_name() {
    let scratch = Scratch::new(&[("I.swift", "import Foo.Sub\nimport struct Foo.Bar\nimport func Foo.baz\n")]);
    let json = json_of(&scratch.stdout(&["--json", "imports", "I.swift"]));
    let listed: Vec<(String, Option<String>)> = json["files"][0]["imports"]
        .as_array()
        .expect("imports")
        .iter()
        .map(|import| (import["name"].as_str().expect("name").to_string(), import["source"].as_str().map(str::to_string)))
        .collect();
    let entry = |name: &str, source: &str| (name.to_string(), Some(source.to_string()));
    assert_eq!(listed, [entry("Foo.Sub", "Foo.Sub"), entry("Bar", "Foo.Bar"), entry("baz", "Foo.baz")], "{json}");
    let plain = scratch.stdout(&["--plain", "imports", "I.swift"]);
    for line in ["import Foo.Sub [1:8]", "import Foo.Bar [2:19]", "import Foo.baz [3:17]"] {
        assert!(plain.contains(line), "`{line}` in {plain}");
    }
}

/// A contextual syntax marker is no reference to a same-named declaration
/// (`R6-F3.C4`, `R7-F1.C5`): the ID-selected default report lists only the
/// real reads as suggestions, never the Lua attribute, Java's inferred-type
/// `var` (also beside a static import of a member named `var`, which is no
/// legacy type), PHP's class-relative `static`, a Swift precedence
/// group's associativity value, a named-argument label (`R7-F1.S5`) or a
/// built-in Swift attribute name (`R7-F1.S6`), cold and on a verified warm
/// hit, in JSON and text.
#[test]
fn contextual_syntax_markers_are_never_suggestions() {
    let scratch = Scratch::new(&[
        ("attributes.lua", "local const=1; local close=2\nlocal x <const> = const\nlocal y <close> = close\n"),
        ("Inferred.java", "record R() {\n    void g() { var x=factory(); int var=1; use(var); }\n}\n"),
        (
            "relative.php",
            "<?php\nclass C {\n    function static() {}\n    function f(): static { return new static(); }\n    function g() { return $this->static(); }\n}\n",
        ),
        ("StaticImport.java", "import static p.Tools.var;\nrecord G() {\n    void g() { var x = 1; int var = 2; use(var); }\n}\n"),
        ("precedence.swift", "precedencegroup A {\n    associativity: left\n}\nfunc g() {\n    let left = 1\n    use(left)\n}\n"),
        ("keywords.py", "x = 1\nf(x=x)\n"),
        ("labels.swift", "func g() {\n    let x = 1\n    _ = f(x: x)\n}\n"),
        ("attributes.swift", "@objc func h() {}\nfunc g() {\n    let objc = 1\n    use(objc)\n}\n"),
    ]);
    // `(file, name, the suggestions, the marker positions never listed)`.
    let expected = [
        ("attributes.lua", "const", vec![row("Usage", "const", "2:19", "NameOnly")], vec!["2:10"]),
        ("attributes.lua", "close", vec![row("Usage", "close", "3:19", "NameOnly")], vec!["3:10"]),
        ("Inferred.java", "var", vec![row("Usage", "var", "2:48", "NameOnly")], vec!["2:16"]),
        ("relative.php", "static", vec![row("Call", "$this->static", "5:34", "NameOnly")], vec!["4:19", "4:39"]),
        ("StaticImport.java", "var", vec![row("Usage", "var", "3:44", "NameOnly")], vec!["3:16"]),
        ("precedence.swift", "left", vec![row("Usage", "left", "6:9", "NameOnly")], vec!["2:20"]),
        ("keywords.py", "x", vec![row("Usage", "x", "2:5", "NameOnly")], vec!["2:3"]),
        ("labels.swift", "x", vec![row("Usage", "x", "3:14", "NameOnly")], vec!["3:11"]),
        ("attributes.swift", "objc", vec![row("Usage", "objc", "4:9", "NameOnly")], vec!["1:2"]),
    ];
    for (file, name, suggestions, markers) in &expected {
        let id = declaration_id(&refs_json(&scratch, &[name, file, "--no-cache"]));
        let args = ["--verbose", "--json", "refs", "--id", id.as_str(), file];
        for entry in crate::cache_entries(scratch.temp.path()) {
            std::fs::remove_file(entry).expect("remove an entry");
        }
        let cold = scratch.hug(&args);
        assert!(stderr_of(&cold).contains("0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{file}: {}", stderr_of(&cold));
        let cold = json_of(&stdout_of(cold, &args));
        assert_eq!(&rows(&cold, "name_only_suggestions"), suggestions, "{file} `{name}` cold: {cold}");
        assert_eq!(rows(&cold, "occurrences"), Vec::new(), "{file} `{name}`: {cold}");
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
        let warm = json_of(&stdout_of(warm, &args));
        assert_eq!(warm, cold, "{file} `{name}` warm");

        let text_args = ["--plain", "refs", "--id", id.as_str(), file];
        let text = scratch.stdout(&text_args);
        for (kind, written, at, _) in suggestions {
            let listed = format!("{kind} {written} {at} —");
            assert!(text.contains(&listed), "{file} `{name}`: `{listed}` in {text}");
        }
        for marker in markers {
            assert!(!text.contains(&format!(" {marker} ")), "{file} `{name}`: marker {marker} listed in {text}");
        }
    }
}

/// A qualified Java annotation or module service is a read, never an
/// import (`R7-F3.C6`): the ID-selected default report for the class lists
/// it as a suggestion with its written path beside the unqualified control,
/// cold and on a verified warm hit, in JSON and text; it is no callee or
/// caller; and `hug imports` lists only the genuine import's final segment.
#[test]
fn qualified_java_annotations_and_services_are_suggestions_not_imports() {
    let scratch = Scratch::new(&[
        ("G.java", "class A {}\n@p.q.A class G {}\n@A class H {}\n"),
        ("module-info.java", "module m {\n    uses a.b.C;\n    provides a.b.C with d.e.D;\n}\nclass C {}\n"),
        ("K.java", "import a.b.Z;\n@p.q.A class K {}\n"),
    ]);
    // `(file, name, the suggestions)`.
    let expected = [
        (
            "G.java",
            "A",
            vec![row("Usage", "p.q.A", "2:6", "NameOnly"), row("Usage", "A", "3:2", "NameOnly")],
        ),
        (
            "module-info.java",
            "C",
            vec![row("Usage", "a.b.C", "2:14", "NameOnly"), row("Usage", "a.b.C", "3:18", "NameOnly")],
        ),
    ];
    for (file, name, suggestions) in &expected {
        let id = declaration_id(&refs_json(&scratch, &[name, file, "--no-cache"]));
        let args = ["--verbose", "--json", "refs", "--id", id.as_str(), file];
        for entry in crate::cache_entries(scratch.temp.path()) {
            std::fs::remove_file(entry).expect("remove an entry");
        }
        let cold = scratch.hug(&args);
        assert!(stderr_of(&cold).contains("0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{file}: {}", stderr_of(&cold));
        let cold = json_of(&stdout_of(cold, &args));
        assert_eq!(&rows(&cold, "name_only_suggestions"), suggestions, "{file} `{name}` cold: {cold}");
        assert_eq!(rows(&cold, "occurrences"), Vec::new(), "{file} `{name}`: {cold}");
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
        let warm = json_of(&stdout_of(warm, &args));
        assert_eq!(warm, cold, "{file} `{name}` warm");

        let text = scratch.stdout(&["--plain", "refs", "--id", id.as_str(), file]);
        for (kind, written, at, _) in suggestions {
            let listed = format!("{kind} {written} {at} —");
            assert!(text.contains(&listed), "{file} `{name}`: `{listed}` in {text}");
        }
        let callers = refs_json(&scratch, &["--id", id.as_str(), "--callers", file]);
        assert_eq!(rows(&callers, "name_only_suggestions"), Vec::new(), "{file}: {callers}");
    }

    let imports = json_of(&scratch.stdout(&["--json", "imports", "G.java", "module-info.java", "K.java"]));
    let listed: Vec<(String, String)> = imports["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| {
            let path = std::path::Path::new(file["file"].as_str().expect("file"))
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            file["imports"]
                .as_array()
                .expect("imports")
                .iter()
                .map(move |import| (path.clone(), import["name"].as_str().expect("name").to_string()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(listed, [("K.java".to_string(), "Z".to_string())], "{imports}");
    let plain = scratch.stdout(&["--plain", "imports", "G.java", "module-info.java", "K.java"]);
    assert_eq!(plain.matches("import ").count(), 1, "{plain}");
}

/// Every C# `using` directive is an import (`R7-F3.S1`): `hug imports`
/// lists the namespace, `static` and alias directives, with the alias and
/// its target, and nothing else.
#[test]
fn csharp_using_directives_are_listed_by_imports() {
    let scratch = Scratch::new(&[(
        "U.cs",
        "using System;\nusing A.B;\nusing static C.D;\nusing E = F.G;\nclass Z { E e; }\n",
    )]);
    let json = json_of(&scratch.stdout(&["--json", "imports", "U.cs"]));
    let files = json["files"].as_array().expect("files");
    assert_eq!(files.len(), 1, "{json}");
    let listed: Vec<(String, Option<String>, Option<String>)> = files[0]["imports"]
        .as_array()
        .expect("imports")
        .iter()
        .map(|import| {
            (
                import["name"].as_str().expect("name").to_string(),
                import["alias"].as_str().map(str::to_string),
                import["source"].as_str().map(str::to_string),
            )
        })
        .collect();
    let entry = |name: &str, alias: Option<&str>, source: &str| (name.to_string(), alias.map(str::to_string), Some(source.to_string()));
    assert_eq!(
        listed,
        [entry("System", None, "System"), entry("B", None, "A.B"), entry("D", None, "C.D"), entry("E", Some("E"), "F.G")],
        "{json}"
    );
    let plain = scratch.stdout(&["--plain", "imports", "U.cs"]);
    for line in ["using System [1:7]", "using A.B [2:9]", "using C.D [3:16]", "using E = F.G [4:7]"] {
        assert!(plain.contains(line), "`{line}` in {plain}");
    }
    assert_eq!(plain.matches("using ").count(), 4, "{plain}");
}

/// A bare-identifier initializer is a read (`R6-F3.X1`) and a zsh
/// declaration-builtin operand is a declaration token (`R6-F3.X2`): the
/// ID-selected report for the Java field `xs` lists its field, local and
/// inferred-local initializer reads, and the one for the zsh function `x`
/// lists only the expansion and the call, never `local x`/`export x`, cold
/// and on a verified warm hit, in JSON and text.
#[test]
fn bare_initializers_are_suggestions_and_shell_declaration_operands_are_not() {
    let scratch = Scratch::new(&[
        ("Initializer.java", "class A {\n    int xs = 1; int ys = xs;\n    void g() { int k = xs; var v = xs; }\n}\n"),
        ("declare.zsh", "x() { :; }\nf() { local x; export x; echo $x; x; }\n"),
    ]);
    // `(file, name, the suggestions, the positions never listed)`.
    let expected = [
        (
            "Initializer.java",
            "xs",
            vec![
                row("Usage", "xs", "2:26", "NameOnly"),
                row("Usage", "xs", "3:24", "NameOnly"),
                row("Usage", "xs", "3:36", "NameOnly"),
            ],
            vec!["2:9"],
        ),
        ("declare.zsh", "x", vec![row("Usage", "x", "2:32", "NameOnly"), row("Call", "x", "2:35", "NameOnly")], vec!["2:13", "2:23"]),
    ];
    for (file, name, suggestions, absent) in &expected {
        let id = declaration_id(&refs_json(&scratch, &[name, file, "--no-cache"]));
        let args = ["--verbose", "--json", "refs", "--id", id.as_str(), file];
        for entry in crate::cache_entries(scratch.temp.path()) {
            std::fs::remove_file(entry).expect("remove an entry");
        }
        let cold = scratch.hug(&args);
        assert!(stderr_of(&cold).contains("0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{file}: {}", stderr_of(&cold));
        let cold = json_of(&stdout_of(cold, &args));
        assert_eq!(&rows(&cold, "name_only_suggestions"), suggestions, "{file} `{name}` cold: {cold}");
        assert_eq!(rows(&cold, "occurrences"), Vec::new(), "{file} `{name}`: {cold}");
        let warm = scratch.hug(&args);
        let stderr = stderr_of(&warm);
        assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
        let warm = json_of(&stdout_of(warm, &args));
        assert_eq!(warm, cold, "{file} `{name}` warm");

        let text_args = ["--plain", "refs", "--id", id.as_str(), file];
        let text = scratch.stdout(&text_args);
        for (kind, written, at, _) in suggestions {
            let listed = format!("{kind} {written} {at} —");
            assert!(text.contains(&listed), "{file} `{name}`: `{listed}` in {text}");
        }
        for position in absent {
            assert!(!text.contains(&format!(" {position} ")), "{file} `{name}`: {position} listed in {text}");
        }
    }
}

/// A comment before, after or on both sides of a default's `=` neither
/// erases the default nor becomes it (`R6-F2`): the ID-selected text report
/// heads every C#, PHP and Scala callable form with its real defaults,
/// cold and warm.
#[test]
fn commented_defaults_show_their_real_values_in_the_reference_header() {
    let scratch = Scratch::new(&[
        (
            "defaults.cs",
            "class K {\n    void g(int x /* b */ = 1, int y = /* a */ 2, int z /* b */ = /* a */ 3) {}\n    \
             K(int x /* b */ = 1, int y = /* a */ 2) {}\n    \
             void Outer() { void Local(int x /* b */ = 1, int y = /* a */ 2) {} }\n}\n",
        ),
        (
            "defaults.php",
            "<?php\nfunction g($x /* b */ = 1, $y = /* a */ 2, $z /* b */ = /* a */ 3) {}\nclass K {\n    \
             function __construct($x /* b */ = 1, $y = /* a */ 2) {}\n    \
             function method(int $x /* b */ = 1, int $y = /* a */ 2) {}\n}\n",
        ),
        (
            "defaults.scala",
            "object O {\n  def g(x: Int /* b */ = 1, y: Int = /* a */ 2, z: Int /* b */ = /* a */ 3): Int = x\n}\n\
             class K {\n  def method(x: Int /* b */ = 1, y: Int = /* a */ 2): Int = x\n}\n",
        ),
    ]);
    let listing = json_of(&scratch.stdout(&["--json", "functions", "--no-cache"]));
    let id_of = |file: &str, name: &str| -> String {
        let ids: Vec<&str> = listing["files"]
            .as_array()
            .expect("files")
            .iter()
            .flat_map(|entry| entry["symbols"].as_array().expect("symbols"))
            .filter(|record| record["source"]["file_path"] == file && record["identity"]["name"] == name)
            .map(|record| record["id"].as_str().expect("id"))
            .collect();
        let [id] = ids.as_slice() else { panic!("{file} `{name}`: {ids:?} in {listing}") };
        id.to_string()
    };
    // `(file, callable, the header's signature)`.
    let expected = [
        ("defaults.cs", "g", "g(x: int = 1, y: int = 2, z: int = 3)"),
        ("defaults.cs", "K", "K(x: int = 1, y: int = 2)"),
        ("defaults.cs", "Local", "Local(x: int = 1, y: int = 2)"),
        ("defaults.php", "g", "g(x = 1, y = 2, z = 3)"),
        ("defaults.php", "__construct", "__construct(x = 1, y = 2)"),
        ("defaults.php", "method", "method(x: int = 1, y: int = 2)"),
        ("defaults.scala", "g", "g(x: Int = 1, y: Int = 2, z: Int = 3)"),
        ("defaults.scala", "method", "method(x: Int = 1, y: Int = 2)"),
    ];
    for (file, name, signature) in expected {
        let id = id_of(file, name);
        let args = ["--verbose", "--plain", "refs", "--id", id.as_str(), file];
        // An earlier callable's run wrote this file's entry; remove it so
        // this run is cold.
        for entry in crate::cache_entries(scratch.temp.path()) {
            std::fs::remove_file(entry).expect("remove an entry");
        }
        for (run, served) in [("cold", "0 hits, 0 partial hits, 1 misses"), ("warm", "1 hits, 0 partial hits, 0 misses")] {
            let output = scratch.hug(&args);
            assert!(stderr_of(&output).contains(served), "{file} `{name}` {run}: {}", stderr_of(&output));
            let text = stdout_of(output, &args);
            let header = text.lines().next().unwrap_or_default();
            assert!(header.starts_with("References to ") && header.contains(signature), "{file} `{name}` {run}: {text}");
            assert!(!header.contains("/*"), "{file} `{name}` {run}: no comment in {header}");
        }
    }
}

/// `(file, [(name, alias, source)], plain lines)`.
type ListedImports = (&'static str, Vec<(&'static str, Option<&'static str>, Option<&'static str>)>, Vec<&'static str>);

/// Scala, Python, Rust, PHP and TypeScript imports list only the names
/// they bind (`R7-F3.S3`-`.S6` and the TS/JS sweep): never a Scala or
/// Python qualifier or an aliased original, always a braced selector, a
/// one-segment or grouped PHP clause, a Rust `use a;`, glob or nested list
/// entry, and a default import, in JSON and plain text.
#[test]
fn imports_list_only_the_names_each_import_binds() {
    let scratch = Scratch::new(&[
        ("I.scala", "import a.b.C\nimport a.{D, E => F}\nimport c.d.*, e.given\nobject O\n"),
        ("i.py", "import a.b\nimport c.d as e, f\nfrom g.h import i as j, k\nfrom l import *\nfrom __future__ import annotations\n"),
        ("i.rs", "use a;\nuse b::*;\nuse c::{g::h, d, e as f};\nextern crate y as z;\nuse k::{self, l};\n"),
        ("i.php", "<?php\nuse A;\nuse B\\C as D;\nuse E\\{F, G as H};\nuse function I\\j;\n"),
        ("i.ts", "import d, { a as b } from \"m\";\nimport q = require(\"m\");\n"),
    ]);
    let expected: [ListedImports; 5] = [
        (
            "I.scala",
            vec![
                ("C", None, Some("a.b.C")),
                ("D", None, Some("a.D")),
                ("F", Some("F"), Some("a.E")),
                ("d", Some("d"), Some("c.d")),
                ("e", Some("e"), Some("e")),
            ],
            vec!["import a.b.C [1:12]", "import a.D, a.E as F [2:11, 19]", "import c.d.*, e.given [3:10, 15]"],
        ),
        (
            "i.py",
            vec![
                ("a", None, Some("a.b")),
                ("e", Some("e"), Some("c.d")),
                ("f", None, Some("f")),
                ("j", Some("j"), Some("g.h")),
                ("k", None, Some("g.h")),
                ("l", Some("l"), Some("l")),
                ("annotations", None, Some("__future__")),
            ],
            vec![
                "import a.b [1:8]",
                "import c.d as e, f [2:15, 18]",
                "from g.h import i as j, k [3:22, 25]",
                "from l import * [4:6]",
                "from __future__ import annotations [5:24]",
            ],
        ),
        (
            "i.rs",
            vec![
                ("a", None, None),
                ("b", Some("b"), Some("b")),
                ("h", None, Some("c::g")),
                ("d", None, Some("c")),
                ("f", Some("f"), Some("c")),
                ("z", Some("z"), None),
                ("k", None, None),
                ("l", None, Some("k")),
            ],
            vec!["use a [1:5]", "use b::* [2:5]", "use {c::g::h, c::d, c::e as f} [3:12, 15, 23]", "use y as z [4:19]", "use {k, k::l} [5:5, 15]"],
        ),
        (
            "i.php",
            vec![
                ("A", None, Some("A")),
                ("D", Some("D"), Some("B\\C")),
                ("F", None, Some("E\\F")),
                ("H", Some("H"), Some("E\\G")),
                ("j", None, Some("I\\j")),
            ],
            vec!["use A [2:5]", "use B\\C as D [3:12]", "use E\\F, E\\G as H [4:8, 16]", "use I\\j [5:16]"],
        ),
        (
            "i.ts",
            vec![("d", None, Some("m")), ("b", Some("b"), Some("m")), ("q", None, Some("m"))],
            vec!["import { d, a as b } from \"m\" [1:8, 18]", "import { q } from \"m\" [2:8]"],
        ),
    ];
    for (file, entries, lines) in &expected {
        let json = json_of(&scratch.stdout(&["--json", "imports", file]));
        let listed: Vec<(String, Option<String>, Option<String>)> = json["files"][0]["imports"]
            .as_array()
            .expect("imports")
            .iter()
            .map(|import| {
                (
                    import["name"].as_str().expect("name").to_string(),
                    import["alias"].as_str().map(str::to_string),
                    import["source"].as_str().map(str::to_string),
                )
            })
            .collect();
        let entries: Vec<(String, Option<String>, Option<String>)> = entries
            .iter()
            .map(|(name, alias, source)| (name.to_string(), alias.map(str::to_string), source.map(str::to_string)))
            .collect();
        assert_eq!(listed, entries, "{file}: {json}");
        let plain = scratch.stdout(&["--plain", "imports", file]);
        for line in lines {
            assert!(plain.contains(&format!("- {line}\n")), "`{line}` in {plain}");
        }
        assert_eq!(plain.matches("\n  - ").count(), lines.len(), "{file}: {plain}");
    }
}

/// TS/JS export clauses (`R7-F3.X2`): the ID-selected report for the
/// function `k` lists only its local export `export { k as g }` and the
/// call in `g`, never the names of a re-export from `"m"` or of
/// `export * as k`, and the one for `const j` lists nothing for the
/// exported alias `j`, cold and on a verified warm hit, in JSON and text.
#[test]
fn export_clause_names_are_never_references() {
    let source = "function k() {}\nconst j = 1;\nexport { k as j } from \"m\";\nexport { k as g };\nexport * as k from \"m\";\nfunction g() { k(); }\n";
    let scratch = Scratch::new(&[("exports.ts", source), ("exports.js", source)]);
    // `(name, the established rows, the positions never listed)`.
    let expected = [
        ("k", vec![row("Usage", "k", "4:10", "Scoped"), row("Call", "k", "6:16", "Scoped")], vec!["3:10", "5:13"]),
        ("j", vec![], vec!["3:15", "4:15"]),
    ];
    for file in ["exports.ts", "exports.js"] {
        for (name, established, absent) in &expected {
            let id = declaration_id(&refs_json(&scratch, &[name, file, "--no-cache"]));
            let args = ["--verbose", "--json", "refs", "--id", id.as_str(), file];
            for entry in crate::cache_entries(scratch.temp.path()) {
                std::fs::remove_file(entry).expect("remove an entry");
            }
            let cold = scratch.hug(&args);
            assert!(stderr_of(&cold).contains("0 hits, 0 partial hits, 1 misses; parsed 1 files"), "{file}: {}", stderr_of(&cold));
            let cold = json_of(&stdout_of(cold, &args));
            assert_eq!(&rows(&cold, "occurrences"), established, "{file} `{name}` cold: {cold}");
            assert_eq!(rows(&cold, "name_only_suggestions"), Vec::new(), "{file} `{name}`: {cold}");
            let warm = scratch.hug(&args);
            let stderr = stderr_of(&warm);
            assert!(stderr.contains("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{file}: {stderr}");
            let warm = json_of(&stdout_of(warm, &args));
            assert_eq!(warm, cold, "{file} `{name}` warm");

            let text = scratch.stdout(&["--plain", "refs", "--id", id.as_str(), file]);
            for (kind, written, at, _) in established {
                let listed = format!("- {kind} {written} {at}\n");
                assert!(text.contains(&listed), "{file} `{name}`: `{listed}` in {text}");
            }
            for position in absent {
                assert!(!text.contains(&format!(" {position}\n")), "{file} `{name}`: {position} listed in {text}");
            }
        }
    }
}

/// A Go raw-string import path is listed by `hug imports` exactly like
/// an interpreted one (`R7-F3.X3`), its source without quotes.
#[test]
fn go_raw_string_imports_are_listed_by_imports() {
    let scratch = Scratch::new(&[("r.go", "package p\nimport `c/d`\nimport x `e/f`\nimport \"m/n\"\n")]);
    let json = json_of(&scratch.stdout(&["--json", "imports", "r.go"]));
    let listed: Vec<(String, Option<String>, Option<String>)> = json["files"][0]["imports"]
        .as_array()
        .expect("imports")
        .iter()
        .map(|import| {
            (
                import["name"].as_str().expect("name").to_string(),
                import["alias"].as_str().map(str::to_string),
                import["source"].as_str().map(str::to_string),
            )
        })
        .collect();
    let entry = |name: &str, alias: Option<&str>, source: &str| (name.to_string(), alias.map(str::to_string), Some(source.to_string()));
    assert_eq!(listed, [entry("d", None, "c/d"), entry("x", Some("x"), "e/f"), entry("n", None, "m/n")], "{json}");
    let plain = scratch.stdout(&["--plain", "imports", "r.go"]);
    for line in ["- import \"c/d\" [2:8]\n", "- import x \"e/f\" [3:8]\n", "- import \"m/n\" [4:8]\n"] {
        assert!(plain.contains(line), "`{line}` in {plain}");
    }
}
