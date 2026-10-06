//! `hug refs`: selecting one declaration (exact smart-case names, qualified
//! and file-prefixed names, `--id`, locals), the ambiguity error, empty
//! reports, the three directions, the versioned `--json` envelope, the text
//! report, rejected flags and formats, and the file-local boundary.

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
