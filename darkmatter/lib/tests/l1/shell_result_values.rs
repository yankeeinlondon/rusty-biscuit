//! Frontmatter shell result suffixes (`::ok`, `::exit-code`, `::result`), read
//! through the public composition result.
//!
//! Commands are portable: `git` in a private repository supplies exit statuses
//! and streams, and this test binary re-executed in a helper mode supplies a
//! command that never exits, one that counts its runs, and one that ends by a
//! signal (a console interrupt status on Windows).

use std::path::{Path, PathBuf};
use std::process::Command;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::ComposeOptions;
use serde_json::{Value, json};

/// A positional argument: libtest reads it as a name filter, which under
/// `--exact` matches nothing, so the harness ignores it.
const HELPER_ARG: &str = "dm-shell-result-helper=";

/// What a re-executed copy of this binary does instead of testing.
#[test]
fn helper_process_entrypoint() {
    let Some(mode) = std::env::args().find_map(|arg| arg.strip_prefix(HELPER_ARG).map(str::to_owned))
    else {
        return;
    };
    let code = match mode.split_once(':') {
        None if mode == "sleep" => loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
        },
        None if mode == "interrupted" => interrupted(),
        // `count:<code>:<path>`: record one run, then exit with `code`.
        Some(("count", rest)) => {
            let (code, path) = rest.split_once(':').expect("count:<code>:<path>");
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("open run counter");
            file.write_all(b".").expect("record run");
            // Long enough that identical concurrent requests overlap.
            std::thread::sleep(std::time::Duration::from_millis(200));
            code.parse().expect("count code")
        }
        _ => panic!("unknown helper mode `{mode}`"),
    };
    std::process::exit(code);
}

#[cfg(unix)]
fn interrupted() -> i32 {
    // SAFETY: signalling our own process; nothing is shared with the parent.
    unsafe { libc::kill(libc::getpid(), libc::SIGKILL) };
    unreachable!("SIGKILL ends the process")
}

#[cfg(windows)]
fn interrupted() -> i32 {
    // How Windows reports a console Ctrl+C.
    0xC000_013A_u32 as i32
}

/// A `$( … )` body that runs this binary in `mode`, quoted for the shell
/// tokenizer and for a YAML single-quoted scalar.
fn helper(mode: &str) -> String {
    let exe = std::env::current_exe().expect("current test executable");
    let module = module_path!()
        .split_once("::")
        .map(|(_, rest)| rest)
        .expect("module path has a crate segment");
    format!(
        "'{}' --exact {module}::helper_process_entrypoint --nocapture {HELPER_ARG}{mode}",
        exe.display()
    )
}

/// A committed, clean Git repository to run commands in.
struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        for args in [
            &["init", "-q"][..],
            &["config", "user.email", "t@example.com"],
            &["config", "user.name", "t"],
            &["config", "commit.gpgsign", "false"],
        ] {
            git(dir.path(), args);
        }
        std::fs::write(dir.path().join("tracked.txt"), "one\n").expect("write tracked file");
        git(dir.path(), &["add", "tracked.txt"]);
        git(dir.path(), &["commit", "-q", "-m", "init"]);
        Self { dir }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn counter(&self, name: &str) -> PathBuf {
        self.path().join(name)
    }

    fn runs(&self, name: &str) -> usize {
        std::fs::read_to_string(self.counter(name)).map_or(0, |text| text.len())
    }

    /// Composes `frontmatter` (YAML lines) with pre-approval computed by
    /// discovery, as a caller that audits before composing does.
    fn compose(&self, frontmatter: &str, allow_timeout: bool) -> Result<Value, String> {
        self.compose_within(frontmatter, allow_timeout, std::time::Duration::from_secs(60))
    }

    fn compose_within(
        &self,
        frontmatter: &str,
        allow_timeout: bool,
        timeout: std::time::Duration,
    ) -> Result<Value, String> {
        let path = self.path().join("doc.md");
        std::fs::write(&path, format!("---\n{frontmatter}\n---\nBody\n")).expect("write doc");
        let document = Markdown::try_from(path.as_path()).map_err(|error| error.to_string())?;
        let options = ComposeOptions::new()
            .with_source_file(&path)
            .with_shell_policy_root(self.path())
            .with_shell_timeout(timeout)
            .with_allow_shell_timeout(allow_timeout);
        let approved = document
            .compose_preflight(&crate::request_support::request(options.clone()))
            .map_err(|error| error.to_string())?
            .approval_set()
            .into_iter()
            .collect();
        let (composed, _report) = document
            .compose_with(&crate::request_support::request(options.with_pre_approved_commands(approved)))
            .map_err(|error: MarkdownError| error.to_string())?;
        Ok(Value::Object(
            composed
                .frontmatter()
                .as_map()
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ))
    }

    fn value(&self, frontmatter: &str) -> Value {
        self.compose(frontmatter, false)
            .unwrap_or_else(|error| panic!("{frontmatter}: {error}"))["v"]
            .clone()
    }
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

fn yaml(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

const SUCCEEDS: &str = "git rev-parse --is-inside-work-tree";
const FAILS: &str = "git rev-parse --verify nope";

/// The value a suffix reads, or `Err(fragment)` of the composition error.
type Expected = Result<Value, &'static str>;

#[test]
fn each_suffix_reads_each_outcome_as_a_typed_value() {
    let repo = Repo::new();
    let sleep = helper("sleep");
    struct Row {
        command: String,
        allow_timeout: bool,
        /// A short deadline for the never-exiting helper; a generous one
        /// otherwise, so a loaded host cannot time out `git`.
        timeout: std::time::Duration,
        none: Expected,
        ok: Expected,
        exit_code: Expected,
        result: Expected,
    }
    let fatal = |value: &Value| value["stderr"].as_str().is_some_and(|s| s.starts_with("fatal:"));
    let rows = [
        Row {
            command: SUCCEEDS.to_string(),
            allow_timeout: false,
            timeout: std::time::Duration::from_secs(60),
            none: Ok(json!("true")),
            ok: Ok(json!(true)),
            exit_code: Ok(json!(0)),
            result: Ok(json!({"ok": true, "code": 0, "stdout": "true", "stderr": ""})),
        },
        Row {
            command: FAILS.to_string(),
            allow_timeout: false,
            timeout: std::time::Duration::from_secs(60),
            none: Err("exit 128"),
            ok: Ok(json!(false)),
            exit_code: Ok(json!(128)),
            result: Ok(json!({"ok": false, "code": 128, "stdout": ""})),
        },
        Row {
            command: sleep.clone(),
            allow_timeout: false,
            timeout: std::time::Duration::from_millis(200),
            none: Err("timed out"),
            ok: Err("timed out"),
            exit_code: Err("timed out"),
            result: Err("timed out"),
        },
        Row {
            command: sleep,
            allow_timeout: true,
            timeout: std::time::Duration::from_millis(200),
            none: Ok(json!("")),
            ok: Ok(json!(false)),
            exit_code: Ok(Value::Null),
            result: Ok(json!({"ok": false, "code": null, "stdout": "", "stderr": ""})),
        },
    ];

    for row in &rows {
        let cells = [
            ("", "none", &row.none),
            ("::ok", "ok", &row.ok),
            ("::exit-code", "exit_code", &row.exit_code),
            ("::result", "result", &row.result),
        ];
        let label = |suffix: &str| format!("{}{suffix} allow={}", row.command, row.allow_timeout);
        // Every value expected to succeed composes in one document, so they
        // run concurrently; each expected failure composes alone.
        let succeeding: Vec<_> = cells.iter().filter(|(_, _, expected)| expected.is_ok()).collect();
        let frontmatter = succeeding
            .iter()
            .map(|(suffix, key, _)| format!("{key}: {}", yaml(&format!("$({}){suffix}", row.command))))
            .collect::<Vec<_>>()
            .join("\n");
        let values = repo
            .compose_within(&frontmatter, row.allow_timeout, row.timeout)
            .unwrap_or_else(|error| panic!("{}: {error}", label("")));
        for (suffix, key, expected) in &succeeding {
            let mut actual = values[*key].clone();
            if *suffix == "::result" && row.command == FAILS {
                assert!(fatal(&actual), "{}: {actual}", label(suffix));
                actual.as_object_mut().unwrap().remove("stderr");
            }
            assert_eq!(Ok(&actual), expected.as_ref(), "{}", label(suffix));
        }
        for (suffix, key, expected) in cells.iter().filter(|(_, _, expected)| expected.is_err()) {
            let frontmatter = format!("{key}: {}", yaml(&format!("$({}){suffix}", row.command)));
            let error = repo
                .compose_within(&frontmatter, row.allow_timeout, row.timeout)
                .expect_err(&label(suffix));
            let fragment = expected.as_ref().unwrap_err();
            assert!(error.contains(fragment), "{}: {error}", label(suffix));
        }
    }
}

#[test]
fn a_signal_is_never_a_value() {
    let repo = Repo::new();
    for suffix in ["", "::ok", "::exit-code", "::result"] {
        let frontmatter = format!("v: {}", yaml(&format!("$({}){suffix}", helper("interrupted"))));
        let error = repo
            .compose(&frontmatter, true)
            .expect_err("a signal-terminated command stays a failure");
        assert!(error.contains("Command failed"), "{suffix}: {error}");
    }
}

#[test]
fn a_result_suffix_applies_to_every_shape() {
    let repo = Repo::new();
    let rows: [(&str, &str, Value); 7] = [
        ("&& chain stops at the failure", &format!("$({SUCCEEDS} && {FAILS})::result"), json!({"ok": false, "code": 128, "stdout": "true"})),
        ("|| fallback runs", &format!("$({FAILS} || {SUCCEEDS})::result"), json!({"ok": true, "code": 0, "stdout": "true"})),
        ("|| fallback is skipped", &format!("$({SUCCEEDS} || {FAILS})::result"), json!({"ok": true, "code": 0, "stdout": "true", "stderr": ""})),
        ("ternary selects a command", &format!("$( flag ? {FAILS} : {SUCCEEDS} )::exit-code"), json!(128)),
        ("ternary selects a chain", &format!("$( flag ? {FAILS} || {SUCCEEDS} : 'x' )::ok"), json!(true)),
        ("ternary selects a literal", &format!("$( !flag ? {FAILS} : 'skipped' )::result"), json!({"ok": true, "code": 0, "stdout": "skipped", "stderr": ""})),
        ("unsuffixed fallback joins as today", &format!("$({FAILS} || {SUCCEEDS})"), json!("true")),
    ];
    for (name, value, expected) in rows {
        let values = repo
            .compose(&format!("flag: true\nv: {}", yaml(value)), false)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let mut actual = values["v"].clone();
        if let (Some(object), Some(expected)) = (actual.as_object_mut(), expected.as_object())
            && !expected.contains_key("stderr")
        {
            let stderr = object.remove("stderr").expect("stderr member");
            assert!(stderr.as_str().unwrap().starts_with("fatal:"), "{name}: {stderr}");
        }
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn a_result_suffix_combines_with_timeout_and_no_cache_in_either_order() {
    let repo = Repo::new();
    for suffixes in ["::ok::timeout:30::no-cache", "::no-cache::timeout:30::ok", "::timeout:30::ok"] {
        let value = repo.value(&format!("v: {}", yaml(&format!("$({SUCCEEDS}){suffixes}"))));
        assert_eq!(value, json!(true), "{suffixes}");
    }
    // Both orders in one document, so the two deadlines elapse concurrently.
    let sleep = helper("sleep");
    let frontmatter = format!(
        "a: {}\nb: {}",
        yaml(&format!("$({sleep})::ok::timeout:1")),
        yaml(&format!("$({sleep})::timeout:1::ok")),
    );
    let values = repo.compose(&frontmatter, true).unwrap();
    assert_eq!((&values["a"], &values["b"]), (&json!(false), &json!(false)), "the suffix's own deadline applies");
}

#[test]
fn two_result_suffixes_fail_to_parse_naming_both() {
    let repo = Repo::new();
    let error = repo
        .compose(&format!("v: {}", yaml(&format!("$({SUCCEEDS})::ok::result"))), false)
        .expect_err("two result suffixes");
    assert!(error.contains("`::ok`") && error.contains("`::result`"), "{error}");
}

#[test]
fn one_command_runs_once_for_every_reader_in_a_document() {
    let repo = Repo::new();
    let counted = |code: i32, name: &str| {
        helper(&format!("count:{code}:{}", repo.counter(name).display()))
    };

    // Unsuffixed and `::result` readers, plus two more concurrent readers.
    let command = counted(0, "shared");
    let frontmatter = [
        format!("a: {}", yaml(&format!("$({command})"))),
        format!("b: {}", yaml(&format!("$({command})::result"))),
        format!("c: {}", yaml(&format!("$({command})::ok"))),
        format!("d: {}", yaml(&format!("$({command})::exit-code"))),
    ]
    .join("\n");
    let values = repo.compose(&frontmatter, false).unwrap();
    assert_eq!(repo.runs("shared"), 1, "{values}");
    assert_eq!(values["c"], json!(true));
    assert_eq!(values["d"], json!(0));

    // `::no-cache` neither reads nor populates the cache.
    let command = counted(0, "no-cache");
    let frontmatter = [
        format!("a: {}", yaml(&format!("$({command})::no-cache"))),
        format!("b: {}", yaml(&format!("$({command})::ok"))),
        format!("c: {}", yaml(&format!("$({command})::exit-code"))),
    ]
    .join("\n");
    repo.compose(&frontmatter, false).unwrap();
    assert_eq!(repo.runs("no-cache"), 2, "`b` and `c` share one run; `a` runs alone");

    // A different deadline is a different execution.
    let command = counted(0, "deadline");
    let frontmatter = [
        format!("a: {}", yaml(&format!("$({command})::ok::timeout:30"))),
        format!("b: {}", yaml(&format!("$({command})::ok::timeout:31"))),
    ]
    .join("\n");
    repo.compose(&frontmatter, false).unwrap();
    assert_eq!(repo.runs("deadline"), 2);

    // An unsuffixed reader of a cached non-zero outcome still fails.
    let command = counted(3, "failing");
    let frontmatter = [
        format!("a: {}", yaml(&format!("$({command})::exit-code"))),
        format!("b: {}", yaml(&format!("$({command})"))),
    ]
    .join("\n");
    let error = repo.compose(&frontmatter, false).expect_err("`b` reads exit 3");
    assert!(error.contains("exit 3"), "{error}");
    assert_eq!(repo.runs("failing"), 1);
}

#[test]
fn a_transcluded_document_reads_a_result_and_shares_the_cache() {
    let repo = Repo::new();
    let command = helper(&format!("count:0:{}", repo.counter("included").display()));
    std::fs::write(
        repo.path().join("part.md"),
        format!(
            "---\nclean: {}\nsame: {}\n---\nclean={{{{ clean }}}}\n",
            yaml(&format!("$({SUCCEEDS})::ok")),
            yaml(&format!("$({command})::ok")),
        ),
    )
    .expect("write partial");
    let path = repo.path().join("root.md");
    std::fs::write(
        &path,
        format!("---\nfirst: {}\n---\n::file ./part.md\n", yaml(&format!("$({command})"))),
    )
    .expect("write root");
    let document = Markdown::try_from(path.as_path()).unwrap();
    let options = ComposeOptions::new()
        .with_source_file(&path)
        .with_shell_policy_root(repo.path())
        .with_shell_timeout(std::time::Duration::from_secs(60));
    let approved = document.compose_preflight(&crate::request_support::request(options.clone())).unwrap().approval_set().into_iter().collect();
    let (composed, _) = document.compose_with(&crate::request_support::request(options.with_pre_approved_commands(approved))).unwrap();
    assert!(composed.content().contains("clean=true"), "{}", composed.content());
    assert_eq!(repo.runs("included"), 1, "the root and the partial share one run");
}

#[test]
fn schema_validates_the_expanded_value() {
    let repo = Repo::new();
    let values = repo
        .compose(
            &format!("$schema:\n  v: boolean\n  c: number\nv: {}\nc: {}",
                yaml(&format!("$({SUCCEEDS})::ok")),
                yaml(&format!("$({FAILS})::exit-code"))),
            false,
        )
        .unwrap();
    assert_eq!(values["v"], json!(true));
    assert_eq!(values["c"], json!(128));

    let error = repo
        .compose(
            &format!("$schema:\n  v: number\nv: {}", yaml(&format!("$({SUCCEEDS})::ok"))),
            false,
        )
        .expect_err("a boolean is not a number");
    assert!(error.contains("schema"), "{error}");
}

/// The input robustness matrix for the top-level frontmatter reader: one
/// fixture, one edit per cell, asserted through the composed result.
#[test]
fn frontmatter_shell_value_reader_robustness_matrix() {
    let repo = Repo::new();
    let control = "$(git diff --quiet)::ok";
    enum Cell {
        Value(Value),
        Absent,
        Error(&'static [&'static str]),
    }
    const FIVE: &[&str] = &["`::ok`", "`::exit-code`", "`::result`", "`::timeout:<seconds>`", "`::no-cache`"];
    let cells: Vec<(&str, String, Cell)> = vec![
        ("control", format!("v: {}", yaml(control)), Cell::Value(json!(true))),
        ("absent suffix", format!("v: {}", yaml("$(git diff --quiet)")), Cell::Value(json!(""))),
        ("::exit-code", format!("v: {}", yaml("$(git diff --quiet)::exit-code")), Cell::Value(json!(0))),
        ("::result", format!("v: {}", yaml("$(git diff --quiet)::result")), Cell::Value(json!({"ok": true, "code": 0, "stdout": "", "stderr": ""}))),
        ("explicit null", "v: null".to_string(), Cell::Value(Value::Null)),
        ("empty value is null", "v:".to_string(), Cell::Value(Value::Null)),
        ("key absent", "w: 1".to_string(), Cell::Absent),
        ("wrong type, whole field", "v: 123".to_string(), Cell::Value(json!(123))),
        ("wrong type, one element", format!("v: {}", yaml("$(git diff --quiet)::ok::bogus")), Cell::Error(FIVE)),
        ("wrong type, every element", format!("v: {}", yaml("$(git diff --quiet)::bogus")), Cell::Error(FIVE)),
        ("empty suffix", format!("v: {}", yaml("$(git diff --quiet)::")), Cell::Error(&["Empty suffix"])),
        ("empty command", format!("v: {}", yaml("$()::ok")), Cell::Error(&["no shell command"])),
        ("duplicate result suffix", format!("v: {}", yaml("$(git diff --quiet)::ok::result")), Cell::Error(&["`::ok`", "`::result`"])),
        ("duplicate timeout", format!("v: {}", yaml("$(git diff --quiet)::timeout:5::timeout:9")), Cell::Error(&["`::timeout:5`", "`::timeout:9`"])),
        ("duplicate YAML key", format!("v: {}\nv: {}", yaml(control), yaml(control)), Cell::Error(&["duplicate"])),
        ("trailing content", format!("v: {}", yaml("$(git diff --quiet)::ok trailing")), Cell::Error(FIVE)),
    ];
    for (name, frontmatter, cell) in cells {
        let result = repo.compose(&frontmatter, false);
        match cell {
            Cell::Value(expected) => {
                let values = result.unwrap_or_else(|error| panic!("{name}: {error}"));
                assert_eq!(values.get("v"), Some(&expected), "{name}");
            }
            Cell::Absent => {
                let values = result.unwrap_or_else(|error| panic!("{name}: {error}"));
                assert_eq!(values.get("v"), None, "{name}");
            }
            Cell::Error(fragments) => {
                let error = result.expect_err(name);
                for fragment in fragments {
                    assert!(error.contains(fragment), "{name}: {error}");
                }
            }
        }
    }
}

mod resolved_values {
    use super::*;
    use darkmatter::markdown::compose::expression::ResolutionContext;
    use darkmatter::markdown::compose::{
        ComposeContext, ResolvedShellValue, check_frontmatter_shell_value,
        execute_resolved_shell_values,
    };
    use std::collections::{HashMap, HashSet};

    fn state(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect()
    }

    fn resolve(repo: &Repo, authored: &str, resolved: &str, at: &[(&str, Value)]) -> ResolvedShellValue {
        ResolvedShellValue::resolve(
            "v",
            authored,
            resolved,
            state(at),
            &ComposeContext::capture_for_content(repo.path(), ""),
            ResolutionContext::new(repo.path().to_path_buf()),
        )
        .unwrap_or_else(|error| panic!("{authored}: {error}"))
    }

    fn options(repo: &Repo, approved: &[&ResolvedShellValue]) -> ComposeOptions {
        let approved: HashSet<String> = approved.iter().flat_map(|value| value.commands()).collect();
        ComposeOptions::new()
            .with_source_file(repo.path().join("doc.md"))
            .with_shell_timeout(std::time::Duration::from_secs(60))
            .with_pre_approved_commands(approved)
    }

    #[test]
    fn command_bytes_are_fixed_when_resolved() {
        let repo = Repo::new();
        let value = resolve(
            &repo,
            "$(git rev-parse --verify {{ target }})::ok",
            "$(git rev-parse --verify HEAD)::ok",
            &[("target", json!("HEAD"))],
        );
        assert_eq!(value.commands(), ["git rev-parse --verify HEAD"]);
        // A later write to `target` reaches neither the bytes nor approval.
        let results = execute_resolved_shell_values(
            &[&value],
            state(&[("target", json!("nope"))]),
            &crate::request_support::request(options(&repo, &[&value])),
        )
        .unwrap();
        assert_eq!(results, [("v".to_string(), json!(true))]);
    }

    #[test]
    fn both_ternary_branches_are_approved_and_the_condition_reads_execution_state() {
        let repo = Repo::new();
        let authored = format!("$( flag ? {FAILS} : {SUCCEEDS} )::exit-code");
        let value = resolve(&repo, &authored, &authored, &[("flag", json!(true))]);
        assert_eq!(
            value.commands(),
            ["git rev-parse --verify nope", "git rev-parse --is-inside-work-tree"]
        );
        let options = options(&repo, &[&value]);
        for (flag, code) in [(true, 128), (false, 0)] {
            let results =
                execute_resolved_shell_values(&[&value], state(&[("flag", json!(flag))]), &crate::request_support::request(options.clone()))
                    .unwrap();
            assert_eq!(results[0].1, json!(code), "flag={flag}");
        }
    }

    #[test]
    fn each_execution_has_its_own_result_cache() {
        let repo = Repo::new();
        let authored = format!("$({})::ok", helper(&format!("count:0:{}", repo.counter("runs").display())));
        let first = resolve(&repo, &authored, &authored, &[]);
        let second = resolve(&repo, &authored, &authored, &[]);
        let options = options(&repo, &[&first]);

        execute_resolved_shell_values(&[&first, &second], HashMap::new(), &crate::request_support::request(options.clone())).unwrap();
        assert_eq!(repo.runs("runs"), 1, "values in one execution share a run");
        execute_resolved_shell_values(&[&first], HashMap::new(), &crate::request_support::request(options.clone())).unwrap();
        assert_eq!(repo.runs("runs"), 2, "a later execution runs the command again");
    }

    #[test]
    fn an_unapproved_command_is_refused() {
        let repo = Repo::new();
        let value = resolve(&repo, &format!("$({SUCCEEDS})::ok"), &format!("$({SUCCEEDS})::ok"), &[]);
        let options = ComposeOptions::new()
            .with_source_file(repo.path().join("doc.md"))
            .with_pre_approved_commands(HashSet::new());
        let error = execute_resolved_shell_values(&[&value], HashMap::new(), &crate::request_support::request(options.clone())).unwrap_err();
        assert!(error.to_string().contains("not pre-approved"), "{error}");
    }

    #[test]
    fn checking_an_authored_value_applies_the_frontmatter_grammar() {
        assert!(check_frontmatter_shell_value("v", "$(git rev-parse {{ ref }})::result").is_ok());
        for (authored, fragment) in [
            ("$(git status)::ok::result", "`::result`"),
            ("$(git status)::bogus", "`::no-cache`"),
            ("$()", "no shell command"),
            ("$({{ cmd }} status)", "may not come from interpolation"),
            ("$(git status) trailing", "trailing content"),
            ("plain text", "not a whole-value"),
        ] {
            let error = check_frontmatter_shell_value("v", authored).unwrap_err().to_string();
            assert!(error.contains(fragment), "{authored}: {error}");
        }
    }
}
