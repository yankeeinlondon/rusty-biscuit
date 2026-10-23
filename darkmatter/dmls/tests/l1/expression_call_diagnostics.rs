//! Argument-type call diagnostics through a real LSP session: the worked
//! example's rows under both `expressions.strict` settings, from
//! `.dmls.toml` and from editor settings, reloaded without a restart, plus
//! ranging, dead branches, numeric recovery, narrowing, and the surfaces that
//! are not executable.
//!
//! Every category comes from Darkmatter's call check; these tests pin what the
//! editor shows for it.

use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const INCOMPATIBLE: &str = "dm.expression.incompatible_call";
const UNCERTAIN: &str = "dm.expression.uncertain_call";
const ERROR: u64 = 1;
const WARNING: u64 = 2;

/// The worked example's schema with every variable, including the
/// undeclared `untyped_var`, set in frontmatter.
const FRONTMATTER: &str = "---
$schema:
  count: string
  maybe: number
  size: number(required)
  flag: boolean(required)
  foo:
    - string(required)
    - number(required)
count: \"3\"
maybe: 3
size: 3
flag: true
foo: 4
untyped_var: 1
---

";

/// Rows a–e, one per line.
const ROWS: &str = "a {{ min(size, 5) }}
b {{ min(maybe, 5) }}
c {{ min(count, 5) }}
d {{ min(flag, 5) }}
e {{ min(untyped_var, 5) }}
";

fn initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": {
            "general": { "positionEncodings": ["utf-8"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "calls" } ]
    })
}

/// A call diagnostic: `(line, start, end, severity, code)`, characters in
/// UTF-8 bytes (the fixtures are ASCII).
type Finding = (u64, u64, u64, u64, String);

struct Session<'w> {
    fixture: LspFixture<'w>,
    uri: String,
}

impl<'w> Session<'w> {
    fn open(workspace: &'w LspWorkspace, text: &str) -> Self {
        let path = workspace.path().join("doc.md");
        std::fs::write(&path, text).unwrap();
        let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
        let mut fixture = LspFixture::start(workspace);
        fixture.initialize(initialize_params(workspace.path()));
        fixture.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
        );
        Self { fixture, uri }
    }

    /// The latest published diagnostics, after the server handled everything
    /// sent so far.
    fn diagnostics(&mut self) -> Vec<Value> {
        self.fixture.flush_server();
        let mut latest = self.fixture.wait_for_diagnostics_params(&self.uri);
        while let Some(params) = self.fixture.take_buffered_diagnostics(&self.uri) {
            latest = params;
        }
        latest["diagnostics"].as_array().cloned().unwrap_or_default()
    }

    /// The call diagnostics, sorted.
    fn calls(&mut self) -> Vec<Finding> {
        calls(&self.diagnostics())
    }

    fn settings(&self, settings: Value) {
        self.fixture.notify("workspace/didChangeConfiguration", json!({ "settings": settings }));
    }

    fn config_file_changed(&self, workspace: &LspWorkspace) {
        let uri = url::Url::from_file_path(workspace.path().join(".dmls.toml")).unwrap();
        self.fixture.notify(
            "workspace/didChangeWatchedFiles",
            json!({ "changes": [ { "uri": uri.as_str(), "type": 2 } ] }),
        );
    }

    fn shutdown(self) {
        self.fixture.shutdown();
    }
}

fn calls(diagnostics: &[Value]) -> Vec<Finding> {
    let mut found: Vec<Finding> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!(INCOMPATIBLE) || diagnostic["code"] == json!(UNCERTAIN))
        .map(|diagnostic| {
            let range = &diagnostic["range"];
            (
                range["start"]["line"].as_u64().unwrap(),
                range["start"]["character"].as_u64().unwrap(),
                range["end"]["character"].as_u64().unwrap(),
                diagnostic["severity"].as_u64().unwrap(),
                diagnostic["code"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

/// The first line of the body.
fn body_line() -> u64 {
    FRONTMATTER.matches('\n').count() as u64
}

/// `(row letter, severity)` of every call diagnostic on a row line.
fn rows(found: &[Finding]) -> Vec<(char, u64)> {
    found
        .iter()
        .map(|(line, _, _, severity, _)| {
            let row = ROWS.lines().nth((line - body_line()) as usize).expect("a row line");
            (row.chars().next().unwrap(), *severity)
        })
        .collect()
}

const DEFAULT_ROWS: [(char, u64); 2] = [('c', WARNING), ('d', ERROR)];
const STRICT_ROWS: [(char, u64); 4] = [('b', WARNING), ('c', WARNING), ('d', ERROR), ('e', WARNING)];

fn worked_example() -> String {
    format!("{FRONTMATTER}{ROWS}")
}

/// The `(start, end)` characters of `needle` in body line `row`.
fn columns(text: &str, line: u64, needle: &str) -> (u64, u64) {
    let line_text = text.lines().nth(line as usize).unwrap();
    let start = line_text.find(needle).unwrap_or_else(|| panic!("{needle} in {line_text}")) as u64;
    (start, start + needle.len() as u64)
}

mod settings {
    use super::*;

    /// Criterion 40, default settings: only row c warns and only row d errs;
    /// no unknown-identifier warning fires because every variable is set.
    #[test]
    fn rows_a_to_e_by_default() {
        let workspace = LspWorkspace::new();
        let mut session = Session::open(&workspace, &worked_example());
        let diagnostics = session.diagnostics();
        assert!(
            !diagnostics.iter().any(|diagnostic| diagnostic["code"] == json!("dm.expression.undeclared_property")),
            "{diagnostics:#?}"
        );
        assert_eq!(rows(&calls(&diagnostics)), DEFAULT_ROWS);
        session.shutdown();
    }

    /// Criterion 40 from `.dmls.toml`, reloaded when the file changes.
    #[test]
    fn rows_a_to_e_from_the_config_file_and_its_reload() {
        let workspace = LspWorkspace::new();
        let config = workspace.path().join(".dmls.toml");
        std::fs::write(&config, "[expressions]\nstrict = true\n").unwrap();
        let mut session = Session::open(&workspace, &worked_example());
        assert_eq!(rows(&session.calls()), STRICT_ROWS);

        std::fs::write(&config, "[expressions]\nstrict = false\n").unwrap();
        session.config_file_changed(&workspace);
        assert_eq!(rows(&session.calls()), DEFAULT_ROWS);

        // A malformed file contributes nothing (default: not strict), and its
        // repair applies again.
        std::fs::write(&config, "[expressions]\nstrict = \"yes\"\n").unwrap();
        session.config_file_changed(&workspace);
        assert_eq!(rows(&session.calls()), DEFAULT_ROWS);
        std::fs::write(&config, "[expressions]\nstrict = true\n").unwrap();
        session.config_file_changed(&workspace);
        assert_eq!(rows(&session.calls()), STRICT_ROWS);
        session.shutdown();
    }

    /// Criterion 40 from editor settings, flipped at runtime, over a file
    /// that says otherwise; invalid settings fall back to the file.
    #[test]
    fn rows_a_to_e_from_editor_settings_flipped_at_runtime() {
        let workspace = LspWorkspace::new();
        std::fs::write(workspace.path().join(".dmls.toml"), "[expressions]\nstrict = false\n").unwrap();
        let mut session = Session::open(&workspace, &worked_example());
        assert_eq!(rows(&session.calls()), DEFAULT_ROWS);

        session.settings(json!({ "dmls": { "expressions": { "strict": true } } }));
        assert_eq!(rows(&session.calls()), STRICT_ROWS);
        session.settings(json!({ "dmls": { "expressions": { "strict": false } } }));
        assert_eq!(rows(&session.calls()), DEFAULT_ROWS);
        session.settings(json!({ "dmls": { "expressions": { "strict": true } } }));
        assert_eq!(rows(&session.calls()), STRICT_ROWS);

        // Invalid: the file layer applies, not the previous `true`.
        session.settings(json!({ "dmls": { "expressions": { "strict": null } } }));
        assert_eq!(rows(&session.calls()), DEFAULT_ROWS);
        // Recovered.
        session.settings(json!({ "dmls": { "expressions": { "strict": true } } }));
        assert_eq!(rows(&session.calls()), STRICT_ROWS);
        // A whole-settings `null` clears the overlay.
        session.settings(Value::Null);
        assert_eq!(rows(&session.calls()), DEFAULT_ROWS);
        session.shutdown();
    }
}

mod ranges {
    use super::*;

    /// Criterion 41: row d's error ranges exactly `flag`, and an arity error
    /// ranges the whole call.
    #[test]
    fn an_argument_error_ranges_the_argument_and_an_arity_error_the_call() {
        let workspace = LspWorkspace::new();
        let text = format!("{FRONTMATTER}d {{{{ min(flag, 5) }}}}\nx {{{{ min(1) }}}}\n");
        let mut session = Session::open(&workspace, &text);
        let line = body_line();
        let (flag_start, flag_end) = columns(&text, line, "flag");
        let (call_start, call_end) = columns(&text, line + 1, "min(1)");
        let diagnostics = session.diagnostics();
        assert_eq!(
            calls(&diagnostics),
            vec![
                (line, flag_start, flag_end, ERROR, INCOMPATIBLE.to_string()),
                (line + 1, call_start, call_end, ERROR, INCOMPATIBLE.to_string()),
            ]
        );
        let message = diagnostics
            .iter()
            .find(|diagnostic| diagnostic["code"] == json!(INCOMPATIBLE))
            .and_then(|diagnostic| diagnostic["message"].as_str())
            .unwrap();
        for part in ["min", "boolean", "number"] {
            assert!(message.contains(part), "{message} names {part}");
        }
        session.shutdown();
    }

    /// A frontmatter interpolation, a `when` condition, and a call in a
    /// `when` condition's own text are checked and ranged at the argument.
    #[test]
    fn frontmatter_and_condition_calls_range_the_argument() {
        let workspace = LspWorkspace::new();
        let frontmatter = FRONTMATTER.replace("untyped_var: 1\n", "untyped_var: 1\ntitle: \"{{ min(flag, 5) }}\"\n");
        let text = format!("{frontmatter}::block when=\"min(flag, 5) > 1\"\nshown\n::end-block\n");
        let mut session = Session::open(&workspace, &text);
        let title_line = text.lines().position(|line| line.starts_with("title:")).unwrap() as u64;
        let block_line = text.lines().position(|line| line.starts_with("::block")).unwrap() as u64;
        let (title_start, title_end) = columns(&text, title_line, "flag");
        let (block_start, block_end) = columns(&text, block_line, "flag");
        assert_eq!(
            session.calls(),
            vec![
                (title_line, title_start, title_end, ERROR, INCOMPATIBLE.to_string()),
                (block_line, block_start, block_end, ERROR, INCOMPATIBLE.to_string()),
            ]
        );
        session.shutdown();
    }
}

mod verdicts {
    use super::*;

    /// One call diagnostic per body line, as `(line index, severity)`, for a
    /// body of one expression per line.
    fn body_verdicts(body: &str) -> Vec<(usize, u64)> {
        let workspace = LspWorkspace::new();
        let mut session = Session::open(&workspace, &format!("{FRONTMATTER}{body}"));
        let found = session
            .calls()
            .into_iter()
            .map(|(line, _, _, severity, _)| ((line - body_line()) as usize, severity))
            .collect();
        session.shutdown();
        found
    }

    /// Criterion 42: `file_exists(42)` binds like any other call.
    #[test]
    fn file_exists_takes_a_number_silently() {
        assert_eq!(body_verdicts("{{ file_exists(42) }}\n"), vec![]);
    }

    /// Criterion 49: a call in a branch that never runs is still an error on
    /// `flag`.
    #[test]
    fn branches_that_do_not_run_are_checked() {
        let body = "{{ true ? 'ok' : min(flag, 5) }}\n{{ size || min(flag, 5) }}\n::block when=\"false\"\n{{ min(flag, 5) }}\n::end-block\n";
        let workspace = LspWorkspace::new();
        let text = format!("{FRONTMATTER}{body}");
        let mut session = Session::open(&workspace, &text);
        let expected: Vec<Finding> = [0, 1, 3]
            .into_iter()
            .map(|offset| {
                let line = body_line() + offset;
                let (start, end) = columns(&text, line, "flag");
                (line, start, end, ERROR, INCOMPATIBLE.to_string())
            })
            .collect();
        assert_eq!(session.calls(), expected);
        session.shutdown();
    }

    /// Criterion 51: a valid fallback recovers the first argument; a fallback
    /// that cannot bind, a missing fallback, and a missing required argument
    /// are errors.
    #[test]
    fn numeric_recovery() {
        let body = concat!(
            "{{ number(\"pear\", 7) }}\n",
            "{{ number(true, 7) }}\n",
            "{{ number(null, 7) }}\n",
            "{{ round(\"pear\", 2.7) }}\n",
            "{{ number(flag, 7) }}\n",
            "{{ number(\"4\", \"x\") }}\n",
            "{{ number(\"pear\") }}\n",
            "{{ number() }}\n",
        );
        assert_eq!(body_verdicts(body), vec![(5, ERROR), (6, ERROR), (7, ERROR)]);
    }

    /// Rows g and h narrow `foo` through the guard.
    #[test]
    fn guards_narrow_rows_g_and_h() {
        assert_eq!(
            body_verdicts("{{ is_number(foo) ? min(foo, 5) : 0 }}\n{{ is_string(foo) ? 0 : min(foo, 5) }}\n"),
            vec![]
        );
    }

    /// Fenced code and `{{{ … }}}` literals are not executable.
    #[test]
    fn fenced_code_and_literals_are_not_checked() {
        assert_eq!(
            body_verdicts("```md\n{{ min(flag, 5) }}\n```\n\n{{{ min(flag, 5) }}}\n\n{{ min(flag, 5) }}\n"),
            vec![(6, ERROR)]
        );
    }
}
