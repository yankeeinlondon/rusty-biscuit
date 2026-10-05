//! The record selectors on the one filter path: kind, scope, visibility,
//! container, test, docs, deprecation and signature-shape filters,
//! smart-case and exact name matching, `--file`/`--name` against positional
//! classification, `classes` containers versus members, `imports --name`,
//! `--json-full` rejections, and selectors staying out of the cache.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::{Scratch, TYPESCRIPT_FIXTURE, json_of, stdout_of};

const APP_PY: &str = r#""""Module docs."""
from os.path import join as pjoin


def grow():
    """Documented."""
    return 1


def Grow_more(size: int, step: int) -> int:
    return size + step


def ünï():
    pass


def Ünï_big():
    pass


def parse_rs():
    pass


async def fetch(url: str) -> str:
    return url


class Box:
    """A box."""

    def open(self):
        pass

    async def load(self, path: str) -> bytes:
        return b""

    @staticmethod
    def make(size: int, depth: int, label: str):
        return None


class Empty:
    pass


def outer():
    def inner():
        pass
    return inner
"#;

const WIDGETS_RS: &str = "use std::collections::HashMap as Map;

/// Documented.
pub fn documented() {}

pub fn undocumented_fn(count: u32) -> Option<u32> {
    let _ = Map::<u32, u32>::new();
    Some(count)
}

#[deprecated]
pub fn old_api() {}

pub struct Widget {
    pub size: u32,
}

impl Widget {
    pub fn grow(&mut self, amount: u32) {
        self.size += amount;
    }

    pub async fn load(&self) -> Result<(), String> {
        Ok(())
    }
}

pub type Alias = Widget;

mod inner {
    pub struct Widget;

    impl Widget {
        pub fn grow(&self) {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn checks() {}
}
";

const RUN_SH: &str = "#!/usr/bin/env bash\ngreet() {\n    echo hi\n}\n";

const TEST_APP_PY: &str = "def test_grows():\n    assert True\n";

const GEO_CPP: &str = "namespace geometry {\nint dot(int a, int b) {\n    return a * b;\n}\n}\n\nint top_level() {\n    return 0;\n}\n";

fn scratch() -> Scratch {
    Scratch::new(&[
        ("app.py", APP_PY),
        ("widgets.rs", WIDGETS_RS),
        ("run.sh", RUN_SH),
        ("test_app.py", TEST_APP_PY),
        ("geo.cpp", GEO_CPP),
        ("nested/deep.py", "def deep_one():\n    pass\n"),
        ("fixtures/hidden.py", "def hidden_one():\n    pass\n"),
        (".gitignore", "ignored.py\n"),
        ("ignored.py", "def ignored_one():\n    pass\n"),
    ])
}

/// Every record of a `--json` report.
fn records(json: &Value) -> Vec<Value> {
    json["files"]
        .as_array()
        .expect("files")
        .iter()
        .flat_map(|file| file["symbols"].as_array().expect("symbols").clone())
        .collect()
}

/// The sorted names of `hug --json <args>`.
fn names(scratch: &Scratch, args: &[&str]) -> Vec<String> {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let mut names: Vec<String> = records(&json_of(&scratch.stdout(&all)))
        .iter()
        .map(|record| record["identity"]["name"].as_str().expect("name").to_string())
        .collect();
    names.sort();
    names
}

/// Asserts `hug <args>` is an argument error (exit 2) whose message
/// contains `needle`.
fn assert_argument_error(scratch: &Scratch, args: &[&str], needle: &str) {
    let output = scratch.hug(args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "hug {args:?} is an argument error: {stderr}");
    assert!(stderr.contains(needle), "hug {args:?}: `{needle}` in {stderr}");
    assert!(output.stdout.is_empty(), "hug {args:?} prints nothing on stdout");
}

#[test]
fn symbols_hides_locals_until_scopes_say_otherwise() {
    let scratch = scratch();
    let json = json_of(&scratch.stdout(&["--json", "symbols", "app.py"]));
    let default = records(&json);
    assert!(default.iter().all(|record| record["scope"] != "Local"), "no local by default");
    let listed = |records: &[Value], name: &str| records.iter().any(|record| record["identity"]["name"] == name);
    assert!(listed(&default, "outer") && !listed(&default, "inner"), "the nested function is a local");
    assert!(listed(&default, "open"), "members are listed");

    let all = records(&json_of(&scratch.stdout(&["--json", "symbols", "--all-scopes", "app.py"])));
    assert!(listed(&all, "inner") && listed(&all, "size"), "`--all-scopes` restores locals and parameters");
    assert_eq!(
        all.len(),
        default.len() + all.iter().filter(|record| record["scope"] == "Local").count(),
        "`--all-scopes` adds exactly the locals"
    );

    let locals = records(&json_of(&scratch.stdout(&["--json", "symbols", "--scope", "local", "app.py"])));
    assert!(!locals.is_empty() && locals.iter().all(|record| record["scope"] == "Local"));
    let explicit = records(&json_of(&scratch.stdout(&[
        "--json", "symbols", "--scope", "item", "--scope", "member", "--scope", "local", "app.py",
    ])));
    assert_eq!(explicit, all, "listing every scope equals `--all-scopes`");

    // The other commands start with every scope.
    assert!(names(&scratch, &["functions", "app.py"]).contains(&"inner".to_string()));
    assert!(!names(&scratch, &["functions", "--scope", "item", "app.py"]).contains(&"inner".to_string()));
    assert_argument_error(&scratch, &["symbols", "--scope", "item", "--all-scopes"], "cannot be used with");
}

#[test]
fn filters_intersect_and_values_within_a_list_are_alternatives() {
    let scratch = scratch();
    assert_eq!(
        names(&scratch, &["functions", "--file", "app.py", "--kind", "method", "--name", "load", "--name", "open"]),
        ["load", "open"]
    );
    assert_eq!(
        names(&scratch, &["functions", "--file", "app.py", "--kind", "method", "--name", "load", "--name", "open", "--async"]),
        ["load"]
    );
    assert_eq!(
        names(&scratch, &["functions", "--file", "app.py", "--file", "widgets.rs", "--name", "grow!"]),
        ["grow", "grow", "grow"],
        "files and names intersect; `grow!` matches `grow` in any case, not `Grow_more`"
    );
    assert_eq!(
        names(&scratch, &["functions", "app.py", "grow", "--exclude-symbols", "Grow_more"]),
        ["grow"],
        "an exclusion wins"
    );
    assert_eq!(
        names(&scratch, &["functions", "app.py", "--returns", "int", "--param-type", "int", "--min-params", "2"]),
        ["Grow_more"]
    );
    assert_eq!(names(&scratch, &["functions", "widgets.rs", "--returns", "Result<*"]), ["load"]);
    assert_argument_error(&scratch, &["functions", "--exclude-tests", "--tests-only"], "cannot be used with");
}

#[test]
fn names_match_with_smart_case_and_exact_matching_including_non_ascii() {
    let scratch = scratch();
    let functions = |patterns: &[&str]| {
        let mut args = vec!["functions", "--file", "app.py"];
        args.extend_from_slice(patterns);
        names(&scratch, &args)
    };
    assert_eq!(functions(&["grow"]), ["Grow_more", "grow"], "lowercase ignores case");
    assert_eq!(functions(&["Grow"]), ["Grow_more"], "an uppercase letter makes the match case-sensitive");
    assert_eq!(functions(&["grow!"]), ["grow"], "trailing `!`: the whole name");
    assert_eq!(functions(&["--exact", "--name", "grow"]), ["grow"]);
    assert_eq!(functions(&["--exact", "--name", "GROW"]), Vec::<String>::new(), "exact keeps smart case");
    assert_eq!(functions(&["--exact", "--name", "gr*"]), Vec::<String>::new(), "`*` is literal under `--exact`");
    assert_eq!(functions(&["gr*"]), ["Grow_more", "grow"], "`*` matches the whole name");
    assert_eq!(functions(&["ünï"]), ["Ünï_big", "ünï"], "non-ASCII folds whole characters");
    assert_eq!(functions(&["Ünï"]), ["Ünï_big"]);
    assert_eq!(functions(&["--exact", "--name", "ünï"]), ["ünï"]);
    assert_eq!(functions(&["--exclude-symbols", "ünï", "--name", "*ï*"]), Vec::<String>::new(), "smart-case exclusion");
}

#[test]
fn file_and_name_flags_bypass_positional_classification() {
    let scratch = scratch();
    // Positionally, a token with `/` or a source extension is a file
    // filter; `--name` takes it as a name.
    let positional = scratch.hug(&["--json", "functions", "*/deep.py#*"]);
    assert!(!positional.status.success(), "positionally a file glob that matches no file");
    assert_eq!(names(&scratch, &["functions", "--name", "*/deep.py#*"]), ["deep_one"]);
    let by_extension = names(&scratch, &["functions", "*.rs"]);
    assert!(by_extension.contains(&"undocumented_fn".to_string()) && !by_extension.contains(&"parse_rs".to_string()));
    assert_eq!(names(&scratch, &["functions", "--name", "*_rs"]), ["parse_rs"]);

    // A named file is scanned whatever `.gitignore` or the fixture
    // exclusions say; a pathless scan skips both.
    let pathless = names(&scratch, &["functions"]);
    assert!(!pathless.contains(&"ignored_one".to_string()) && !pathless.contains(&"hidden_one".to_string()));
    assert_eq!(names(&scratch, &["functions", "--file", "ignored.py"]), ["ignored_one"]);
    assert_eq!(names(&scratch, &["functions", "--file", "fixtures/hidden.py"]), ["hidden_one"]);
    assert_eq!(names(&scratch, &["functions", "--file", "./nested/deep.py"]), ["deep_one"]);
    assert_eq!(names(&scratch, &["functions", "--file", "nested/*.py"]), ["deep_one"], "a glob");
    assert_eq!(
        names(&scratch, &["functions", "--file", "nested/deep.py", "--file", "ignored.py"]),
        ["deep_one", "ignored_one"],
        "file values are alternatives"
    );
    // An excluded file stays excluded; no file left is the usual
    // "no source files" failure, as for positional filters.
    let excluded = scratch.hug(&["--json", "functions", "--file", "ignored.py", "--exclude-files", "ignored.py"]);
    assert_eq!(excluded.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&excluded.stderr).contains("NoSourceFiles"));
    assert_argument_error(&scratch, &["functions", "--file", "~someone/x.rs"], "--file ~someone/x.rs");
}

#[test]
fn qualified_name_patterns_anchor_at_the_displayed_name() {
    let scratch = scratch();
    let app_functions = names(&scratch, &["functions", "app.py"]);
    assert_eq!(names(&scratch, &["functions", "--name", "app.py#*"]), app_functions);
    assert_eq!(names(&scratch, &["functions", "--name", "deep.py#*"]), Vec::<String>::new(), "below the scan root");
    assert_eq!(names(&scratch, &["functions", "--name", "*/deep.py#*"]), ["deep_one"]);
    assert_eq!(names(&scratch, &["functions", "--name", "Widget::*"]), Vec::<String>::new());
    assert_eq!(names(&scratch, &["functions", "--name", "*Widget::*"]), ["grow", "grow", "load"]);
    assert_eq!(names(&scratch, &["functions", "--name", "*inner::Widget::*"]), ["grow"]);
    // A C++ namespace roots the name: no file prefix.
    assert_eq!(names(&scratch, &["functions", "--name", "geometry::*"]), ["dot"]);
    assert_eq!(names(&scratch, &["functions", "--name", "box.*"]), Vec::<String>::new(), "anchored at the file prefix");
    assert_eq!(names(&scratch, &["functions", "--name", "*box.*"]), ["load", "make", "open"], "smart case");

    assert_eq!(names(&scratch, &["functions", "--in", "Widget"]), ["grow", "grow", "load"]);
    assert_eq!(names(&scratch, &["functions", "--in", "*inner::Widget"]), ["grow"]);
    assert_eq!(names(&scratch, &["functions", "--in", "box"]), ["load", "make", "open"]);
    assert_eq!(names(&scratch, &["functions", "--exact", "--in", "bo"]), Vec::<String>::new());
}

/// `classes`: identity predicates and `--in` select containers, shape
/// predicates select displayed members, and a selected container may keep
/// no member.
#[test]
fn classes_select_containers_and_members_separately() {
    let scratch = scratch();
    let report = |args: &[&str]| -> BTreeMap<String, Vec<String>> {
        let mut all = vec!["--json", "classes", "app.py"];
        all.extend_from_slice(args);
        let json = json_of(&scratch.stdout(&all));
        let mut containers = BTreeMap::new();
        for file in json["files"].as_array().expect("files") {
            for container in file["symbols"].as_array().expect("symbols") {
                let members = file["members"][container["id"].as_str().expect("id")]
                    .as_array()
                    .expect("a members list per container")
                    .iter()
                    .map(|member| member["identity"]["name"].as_str().expect("name").to_string())
                    .collect();
                containers.insert(container["identity"]["name"].as_str().expect("name").to_string(), members);
            }
        }
        containers
    };
    let expect = |pairs: &[(&str, &[&str])]| -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(name, members)| (name.to_string(), members.iter().map(ToString::to_string).collect()))
            .collect()
    };

    assert_eq!(report(&[]), expect(&[("Box", &["open", "load", "make"]), ("Empty", &[])]));
    assert_eq!(report(&["--name", "box", "--async"]), expect(&[("Box", &["load"])]));
    assert_eq!(
        report(&["--min-params", "3"]),
        expect(&[("Box", &["make"]), ("Empty", &[])]),
        "a member filter never drops a container"
    );
    assert_eq!(report(&["--in", "Box"]), expect(&[("Box", &["open", "load", "make"])]), "`--in` names the container");
    assert_eq!(report(&["--undocumented"]), expect(&[("Empty", &[])]));
    assert_eq!(report(&["--kind", "class"]), report(&[]));
    assert_eq!(report(&["--kind", "function"]), expect(&[]), "a member kind selects no container");

    let plain = scratch.stdout(&["--plain", "classes", "app.py", "--async"]);
    assert!(plain.contains("class Empty"), "a container without matching members is listed:\n{plain}");
    assert!(!plain.contains("Fields"), "fields have no signature to match:\n{plain}");
}

#[test]
fn kind_selection_narrows_the_command_set_and_rejects_unknown_kinds() {
    let scratch = scratch();
    let aliases = records(&json_of(&scratch.stdout(&["--json", "symbols", "widgets.rs", "--kind", "type_alias"])));
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0]["kind"], "TypeAlias");
    assert_eq!(aliases[0]["identity"]["name"], "Alias");
    assert_eq!(
        names(&scratch, &["types", "widgets.rs", "--kind", "type_alias", "--kind", "function"]),
        ["Alias"],
        "`--kind` never widens the command's kinds"
    );
    let json = json_of(&scratch.stdout(&["--json", "functions", "--kind", "type_alias"]));
    assert_eq!(json["files"], serde_json::json!([]));
    assert_argument_error(&scratch, &["symbols", "--kind", "type-alias"], "invalid value 'type-alias'");
    assert_argument_error(&scratch, &["symbols", "--kind", "struct"], "possible values");
    assert_argument_error(&scratch, &["symbols", "--visibility", "exported"], "possible values");
}

#[test]
fn imports_match_an_alias_or_an_original_name_and_keep_positional_files() {
    let scratch = scratch();
    let imports = |args: &[&str]| -> Vec<String> {
        let mut all = vec!["--json", "imports", "widgets.rs", "app.py"];
        all.extend_from_slice(args);
        let json = json_of(&scratch.stdout(&all));
        let mut names: Vec<String> = json["files"]
            .as_array()
            .expect("files")
            .iter()
            .flat_map(|file| file["imports"].as_array().expect("imports").clone())
            .map(|import| import["name"].as_str().expect("name").to_string())
            .collect();
        names.sort();
        names
    };
    assert_eq!(imports(&[]), ["Map", "pjoin"]);
    assert_eq!(imports(&["--name", "HashMap"]), ["Map"], "the original name");
    assert_eq!(imports(&["--name", "Map"]), ["Map"], "the alias");
    assert_eq!(imports(&["--name", "HashMap!"]), ["Map"], "the original's last segment, exactly");
    assert_eq!(imports(&["--name", "join"]), ["pjoin"]);
    assert_eq!(imports(&["--name", "std::collections::*"]), ["Map"], "a qualified pattern: the whole path");
    assert_eq!(imports(&["--name", "map", "--exclude-symbols", "hashmap"]), Vec::<String>::new());
    assert_eq!(imports(&["--name", "missing"]), Vec::<String>::new());

    // A positional token stays a file filter for `imports`.
    let output = scratch.hug(&["--plain", "imports", "Map"]);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("HashMap"), "`Map` is not an import name");
    assert_argument_error(&scratch, &["imports", "--kind", "function"], "unexpected argument '--kind'");
    assert_argument_error(&scratch, &["imports", "--exported"], "unexpected argument '--exported'");
}

#[test]
fn test_docs_and_deprecation_filters_use_the_computed_flags() {
    let scratch = scratch();
    let undocumented = names(&scratch, &["functions", "widgets.rs", "run.sh", "--undocumented"]);
    assert!(undocumented.contains(&"undocumented_fn".to_string()));
    assert!(!undocumented.contains(&"documented".to_string()));
    assert!(!undocumented.contains(&"greet".to_string()), "Bash has no doc detection: never undocumented");
    assert_eq!(names(&scratch, &["functions", "run.sh", "--undocumented"]), Vec::<String>::new());
    assert_eq!(names(&scratch, &["functions", "run.sh"]), ["greet"]);

    assert_eq!(names(&scratch, &["functions", "--tests-only"]), ["checks", "test_grows"]);
    let without_tests = names(&scratch, &["functions", "--exclude-tests"]);
    assert!(!without_tests.contains(&"checks".to_string()) && !without_tests.contains(&"test_grows".to_string()));
    assert!(without_tests.contains(&"grow".to_string()));
    assert_eq!(names(&scratch, &["functions", "--deprecated"]), ["old_api"]);

    assert_eq!(names(&scratch, &["functions", "run.sh", "--visibility", "unknown"]), ["greet"]);
    assert_eq!(
        names(&scratch, &["symbols", "widgets.rs", "--visibility", "unknown"]),
        Vec::<String>::new(),
        "`unknown` matches only Unknown"
    );
    let exported = names(&scratch, &["functions", "widgets.rs", "--exported"]);
    let public = names(&scratch, &["functions", "widgets.rs", "--visibility", "public"]);
    assert!(public.contains(&"grow".to_string()), "a public method");
    assert!(!exported.contains(&"grow".to_string()), "is not a module export: {exported:?}");
    assert!(exported.contains(&"documented".to_string()));
}

#[test]
fn empty_selections_succeed_and_analysis_errors_still_fail() {
    let scratch = scratch();
    let json = json_of(&scratch.stdout(&["--json", "symbols", "--name", "nothing_matches_this"]));
    assert_eq!(json["files"], serde_json::json!([]));
    assert_eq!(json["errors"], serde_json::json!([]));
    let plain = scratch.stdout(&["--plain", "functions", "app.py", "--name", "nothing_matches_this"]);
    assert_eq!(plain, "(no symbols)\n");

    std::fs::write(scratch.repo.path().join("bad.rs"), b"fn ok() {}\n\xff\xfe\n").expect("write bad file");
    let output = scratch.hug(&["--json", "functions", "--name", "grow"]);
    assert_eq!(output.status.code(), Some(1), "a failed file still fails a filtered run");
    assert!(String::from_utf8_lossy(&output.stderr).contains("bad.rs"));
    let json = json_of(&String::from_utf8(output.stdout).expect("utf-8"));
    assert_eq!(json["errors"][0]["file"], "bad.rs");
    assert!(!records(&json).is_empty(), "the good files are still listed");
}

#[test]
fn json_full_rejects_every_record_selector() {
    let scratch = scratch();
    for (args, flag) in [
        (&["--kind", "function"][..], "--kind"),
        (&["--scope", "item"], "--scope"),
        (&["--all-scopes"], "--all-scopes"),
        (&["--visibility", "public"], "--visibility"),
        (&["--in", "Box"], "--in"),
        (&["--exclude-tests"], "--exclude-tests"),
        (&["--tests-only"], "--tests-only"),
        (&["--undocumented"], "--undocumented"),
        (&["--deprecated"], "--deprecated"),
        (&["--async"], "--async"),
        (&["--returns", "*"], "--returns"),
        (&["--param-type", "*"], "--param-type"),
        (&["--min-params", "1"], "--min-params"),
        (&["--name", "grow"], "--name"),
    ] {
        let mut all = vec!["--json-full", "symbols"];
        all.extend_from_slice(args);
        assert_argument_error(&scratch, &all, flag);
    }
    let json = json_of(&scratch.stdout(&["--json-full", "symbols", "--file", "app.py", "--exact"]));
    assert_eq!(json["indexes"].as_array().expect("indexes").len(), 1, "file selectors are accepted");
}

/// Selectors and the displayed prefix are applied after the cache: a
/// filtered run is a hit on the unfiltered run's entry, and from another
/// scan root a file-qualified pattern names the file by that root.
#[test]
fn selectors_never_reach_the_cache_entry() {
    let scratch = Scratch::new(&[("sub/mod.py", APP_PY)]);
    let run = |dir: &Path, args: &[&str]| -> (String, Value) {
        let mut all = vec!["--verbose", "--json", "symbols"];
        all.extend_from_slice(args);
        let output = scratch.process(&all).current_dir(dir).output().expect("run hug");
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        (stderr, json_of(&stdout_of(output, &all)))
    };
    let root = scratch.repo.path();
    let (cold, _) = run(root, &["sub/mod.py"]);
    assert!(cold.starts_with("cache: 0 hits, 0 partial hits, 1 misses"), "{cold}");

    let (filtered, from_root) = run(root, &["sub/mod.py", "--name", "sub/mod.py#Box.*", "--kind", "method"]);
    assert!(filtered.starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{filtered}");
    let (from_sub_stats, from_sub) = run(&root.join("sub"), &["mod.py", "--name", "mod.py#Box.*", "--kind", "method"]);
    assert!(from_sub_stats.starts_with("cache: 1 hits, 0 partial hits, 0 misses; parsed 0 files"), "{from_sub_stats}");

    let facts = |json: &Value| -> Vec<(Value, String)> {
        records(json)
            .iter()
            .map(|record| (record["id"].clone(), record["identity"]["qualified_name"].as_str().expect("name").to_string()))
            .collect()
    };
    let (root_facts, sub_facts) = (facts(&from_root), facts(&from_sub));
    assert_eq!(root_facts.len(), 3, "open, load and make: {root_facts:?}");
    for ((root_id, root_name), (sub_id, sub_name)) in root_facts.iter().zip(&sub_facts) {
        assert_eq!(root_id, sub_id, "same ID for {root_name}");
        assert_eq!(root_name.strip_prefix("sub/"), Some(sub_name.as_str()), "only the prefix differs");
    }
    assert_eq!(crate::cache_entries(scratch.temp.path()).len(), 1, "one entry");
}

/// Parameters bound by a pattern (Rust tuple and `_`, TypeScript and
/// JavaScript object and array patterns) or spread over curried Scala lists
/// are counted by `--min-params` and typed for `--param-type`: each
/// selection keeps `patterned`.
#[test]
fn signature_filters_select_patterned_and_curried_parameters() {
    let scratch = Scratch::new(&[
        ("patterned.rs", "pub fn patterned((a, b): (u32, u32), _: String) {}\n"),
        ("patterned.ts", "export function patterned({x}: {x:number}, [a,b]: number[]) {}\n"),
        ("patterned.js", "export function patterned({x}, [a,b]) {}\n"),
        ("patterned.scala", "object O {\n  def patterned(a: Int)(b: String): Int = a\n}\n"),
    ]);
    let cases: &[(&str, &[&str])] = &[
        ("patterned.rs", &["--min-params", "2"]),
        ("patterned.rs", &["--param-type", "(u32, u32)"]),
        ("patterned.rs", &["--param-type", "String"]),
        ("patterned.ts", &["--min-params", "2"]),
        ("patterned.ts", &["--param-type", "number[]"]),
        ("patterned.js", &["--min-params", "2"]),
        ("patterned.scala", &["--min-params", "2"]),
        ("patterned.scala", &["--param-type", "String"]),
    ];
    for (file, selector) in cases {
        let mut args = vec!["functions", "--file", file, "--name", "patterned"];
        args.extend_from_slice(selector);
        args.extend_from_slice(&["--count", "--no-cache"]);
        assert_eq!(scratch.stdout(&args), "1\n", "hug {args:?}");
    }
}

/// `--deprecated` and `--tests-only` select by syntax-read markers: a marker
/// word inside a Swift `message:` string selects nothing, while a `cfg(test)`
/// with a comment in it and a Go block doc whose `Deprecated:` paragraph
/// follows a blank line are selected.
#[test]
fn marker_filters_follow_source_boundaries() {
    let scratch = Scratch::new(&[
        (
            "markers.swift",
            "@available(*, message: \"hello, deprecated, world\")\nfunc decoy() {}\n",
        ),
        ("markers.rs", "#[cfg(/* rationale */ test)]\nmod checks {\n    pub fn helper() {}\n}\n"),
        (
            "probe.go",
            "package p\n\n/* OldBlock documents OldBlock.\n\nDeprecated: use New.\n*/\nfunc OldBlock() {}\n",
        ),
    ]);
    let cases: [(&[&str], &str); 3] = [
        (&["functions", "--file", "markers.swift", "--deprecated", "--name", "decoy"], "0"),
        (&["symbols", "--file", "markers.rs", "--tests-only", "--name", "checks"], "1"),
        (&["functions", "--file", "probe.go", "--deprecated", "--name", "OldBlock"], "1"),
    ];
    for (selection, expected) in cases {
        let args: Vec<&str> = selection.iter().copied().chain(["--count", "--no-cache"]).collect();
        assert_eq!(scratch.stdout(&args).trim(), expected, "hug {}", args.join(" "));
    }
}

/// TypeScript doc text appended to the primary fixture: in `fence_decoy`
/// the `@deprecated` line sits inside a four-backtick fence that a
/// three-backtick line does not close; in `fence_real` it follows the actual
/// close.
pub(crate) const FENCED_DEPRECATION_TS: &str = concat!(
    "\n/**\n * ````text\n * ```\n * @deprecated example text\n * ````\n */\nexport function fence_decoy() {}\n",
    "\n/**\n * ````text\n * ```\n * example text\n * ````\n * @deprecated use another function\n */\nexport function fence_real() {}\n",
);

/// `--deprecated` honors fence boundaries: a tag inside an example is not a
/// tag, and a tag after the example's real closing fence is.
#[test]
fn deprecated_filter_follows_fence_boundaries() {
    let source = format!("{TYPESCRIPT_FIXTURE}{FENCED_DEPRECATION_TS}");
    let scratch = Scratch::new(&[("typescript.ts", source.as_str())]);
    for (name, expected) in [("fence_decoy", "0"), ("fence_real", "1")] {
        let args = [
            "functions", "--file", "typescript.ts", "--name", name, "--deprecated", "--count", "--no-cache",
        ];
        assert_eq!(scratch.stdout(&args).trim(), expected, "hug {}", args.join(" "));
    }
}

/// A Go file whose `ReviewLiteral` doc is a starred `Deprecated:` (text, not
/// a marker), `ReviewMarked` a real marker, and `ReviewBlockStar` and
/// `ReviewLineStar` star-only docs.
pub(crate) const GO_DOC_STARS: &str = concat!(
    "package stars\n\n",
    "/* *Deprecated: this is literal starred text. */\nfunc ReviewLiteral() {}\n\n",
    "/* Deprecated: use New. */\nfunc ReviewMarked() {}\n\n",
    "/* * */\nfunc ReviewBlockStar() {}\n\n",
    "// *\nfunc ReviewLineStar() {}\n",
);

/// Go comments carry no decoration: a `*` after `/*` or `//` is doc text, so
/// a starred `Deprecated:` is no marker and a star-only comment still
/// documents its function, for `--deprecated`, `--undocumented` and the doc
/// coverage of `--stats`.
#[test]
fn go_doc_stars_are_text_for_deprecated_undocumented_and_stats() {
    let scratch = Scratch::new(&[("stars.go", GO_DOC_STARS)]);
    let count = |args: &[&str]| scratch.stdout(args).trim().to_string();
    for (name, expected) in [("ReviewLiteral", "0"), ("ReviewMarked", "1")] {
        let args = ["functions", "--file", "stars.go", "--name", name, "--deprecated", "--count", "--no-cache"];
        assert_eq!(count(&args), expected, "hug {}", args.join(" "));
    }
    let star_only = ["functions", "--file", "stars.go", "--name", "*Star", "--undocumented", "--count", "--no-cache"];
    assert_eq!(count(&star_only), "0", "hug {}", star_only.join(" "));

    let stats = json_of(&scratch.stdout(&[
        "functions", "--file", "stars.go", "--name", "*Star", "--stats", "--json", "--no-cache",
    ]));
    let coverage = &stats["stats"]["doc_coverage"];
    assert_eq!(coverage["documented"], 2, "{coverage}");
    assert_eq!(coverage["total"], 2, "{coverage}");
    assert_eq!(coverage["ratio"].as_f64(), Some(1.0), "{coverage}");
}

/// Only a nonempty `str` constant is a Python docstring: a formatted or a
/// bytes literal leaves its function undocumented, while an adjacent
/// concatenation of ordinary literals documents it, both for
/// `--undocumented` and for the doc coverage of `--stats`.
#[test]
fn python_docstrings_are_string_constants_for_undocumented_and_stats() {
    let scratch = Scratch::new(&[(
        "docs.py",
        concat!(
            "def plain():\n    \"docs\"\n",
            "def formatted():\n    f\"docs\"\n",
            "def bytestring():\n    b\"docs\"\n",
            "def concatenated():\n    \"doc\" \"umented\"\n",
            "def escaped():\n    \"\\u0041\"\n",
        ),
    )]);
    assert_eq!(
        names(&scratch, &["functions", "--file", "docs.py", "--undocumented", "--no-cache"]),
        ["bytestring", "formatted"]
    );
    let stats = json_of(&scratch.stdout(&["functions", "--file", "docs.py", "--stats", "--json", "--no-cache"]));
    let coverage = &stats["stats"]["doc_coverage"];
    assert_eq!(coverage["documented"], 3, "{coverage}");
    assert_eq!(coverage["total"], 5, "{coverage}");
    assert_eq!(coverage["ratio"].as_f64(), Some(0.6), "{coverage}");
}
