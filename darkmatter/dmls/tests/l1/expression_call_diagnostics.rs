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

    /// Rows f and i: a call argument has its callee's declared return type,
    /// and `|| 0` removes `null`, so neither outer `min` is shown under
    /// either setting; the inner `length(count)` keeps its own category 2,
    /// which only strict mode shows.
    #[test]
    fn rows_f_and_i_are_silent_under_both_settings() {
        let text = format!("{FRONTMATTER}f {{{{ min(length(count), 5) }}}}\ni {{{{ min(maybe || 0, 5) }}}}\n");
        let workspace = LspWorkspace::new();
        let mut session = Session::open(&workspace, &text);
        assert!(session.calls().is_empty(), "default settings show nothing");
        session.shutdown();

        let workspace = LspWorkspace::new();
        std::fs::write(workspace.path().join(".dmls.toml"), "[expressions]\nstrict = true\n").unwrap();
        let mut session = Session::open(&workspace, &text);
        let found = session.calls();
        let row_f = body_line();
        let (start, end) = columns(&text, row_f, "count");
        assert_eq!(found, [(row_f, start, end, WARNING, UNCERTAIN.to_string())], "only the inner call warns");
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

/// `numberlike` and `boolish` keep text as sent: a `boolish` passed to a
/// numeric or `length` parameter may bind (`"1"`, `"yes"`), so the editor
/// warns (category 3) and never errs.
mod preserved_text {
    use super::*;

    const DECLARED: &str = "---
$schema:
  flag: boolish(required)
  amount: numberlike(required)
flag: \"1\"
amount: \"4\"
---

";

    /// `(line, start, end, severity, code)` of every call diagnostic of
    /// `body` under [`DECLARED`], lines relative to the body.
    fn body_calls(body: &str) -> Vec<Finding> {
        let workspace = LspWorkspace::new();
        let text = format!("{DECLARED}{body}");
        let mut session = Session::open(&workspace, &text);
        let offset = DECLARED.matches('\n').count() as u64;
        let found = session.calls().into_iter().map(|(line, start, end, severity, code)| (line - offset, start, end, severity, code)).collect();
        session.shutdown();
        found
    }

    #[test]
    fn a_boolish_to_a_numeric_or_length_parameter_warns() {
        let body = "{{ min(flag, 5) }}\n{{ max(flag, 5) }}\n{{ abs(flag) }}\n{{ length(flag) }}\n";
        let expected: Vec<Finding> = body
            .lines()
            .enumerate()
            .map(|(line, text)| {
                let start = text.find("flag").unwrap() as u64;
                (line as u64, start, start + 4, WARNING, UNCERTAIN.to_string())
            })
            .collect();
        assert_eq!(body_calls(body), expected);
    }

    #[test]
    fn numberlike_recovered_and_literal_controls_are_silent() {
        let body = "{{ min(amount, 5) }}\n{{ number(flag, 7) }}\n{{ min(\"1\", 5) }}\n";
        assert_eq!(body_calls(body), vec![]);
    }
}

/// Flow-sensitive narrowing in the editor: the same library scope
/// composition builds, so guarded calls are silent exactly where the
/// library types them category 1, and a lifecycle `when` narrows only until
/// a write or the end of its event.
mod narrowing {
    use super::*;
    use darkmatter::markdown::compose::expression::{
        ArgumentCategory, DeclaredTypes, TypeScope, check_expression_calls, parse_spanned,
    };
    use darkmatter::markdown::compose::page_blocks::BodyGuards;
    use darkmatter::markdown::schemas::parse_yaml_schema;

    const SCHEMA: &str = "x: number\nready: boolean\nflag: boolean(required)\nfoo:\n  - string(required)\n  - number(required)\n";

    /// One guard form per line; `::block` lines hold no call.
    const BODY: &str = "{{ x ? min(x, 5) : 0 }}
{{ x && min(x, 5) }}
::block when=\"x\"
{{ min(x, 5) }}
::end-block
{{ min(x, 5) }}
{{ x ? 0 : min(x, 5) }}
{{ !x ? 0 : min(x, 5) }}
{{ is_number(foo) ? min(foo, 5) : 0 }}
{{ is_string(foo) ? 0 : min(foo, 5) }}
{{ is_number(foo) || ready ? min(foo, 5) : 0 }}
{{ is_number(trim(foo)) ? min(foo, 5) : 0 }}
{{ is_number(flag) ? min(flag, 5) : 0 }}
";

    fn strict_session(workspace: &LspWorkspace, text: &str) -> Vec<Finding> {
        std::fs::write(workspace.path().join(".dmls.toml"), "[expressions]\nstrict = true\n").unwrap();
        let mut session = Session::open(workspace, text);
        let found = session.calls();
        session.shutdown();
        found
    }

    /// Criteria 9, 10, and 33, and the parity checkpoint: for every body
    /// line, the editor shows a `min` finding exactly when the library's
    /// call check, at the site composition types (its `::block` guards
    /// through `BodyGuards`), is not category 1, at the strict severity of
    /// that category.
    #[test]
    fn the_editor_shows_exactly_the_library_categories_at_each_guarded_site() {
        let schema = parse_yaml_schema(&serde_yaml_ng::from_str(SCHEMA).unwrap()).unwrap();
        let declared = DeclaredTypes::new(Some(&schema));
        let guards = BodyGuards::scan(BODY);
        let mut expected = Vec::new();
        let mut offset = 0;
        for (index, line) in BODY.lines().enumerate() {
            if let (Some(start), Some(end)) = (line.find("{{"), line.find("}}")) {
                let site = guards.scope_at(TypeScope::unguarded(&declared), offset + start);
                let checks = check_expression_calls(&parse_spanned(&line[start + 2..end]).unwrap(), &site);
                let min = checks.iter().find(|check| check.function == "min").expect("min is checked");
                match min.category() {
                    ArgumentCategory::Compatible => {}
                    ArgumentCategory::Incompatible => expected.push((index, ERROR)),
                    _ => expected.push((index, WARNING)),
                }
            }
            offset += line.len() + 1;
        }
        // The library verdicts this fixture pins: the guarded forms are
        // silent, and the unguarded, false-branch, non-narrowing, and
        // impossible ones are not.
        assert_eq!(expected, [(5, WARNING), (6, WARNING), (10, WARNING), (11, WARNING), (12, ERROR)]);

        let schema_yaml: String = SCHEMA.lines().map(|line| format!("  {line}\n")).collect();
        let head = format!("---\n$schema:\n{schema_yaml}flag: true\nfoo: 1\n---\n");
        let workspace = LspWorkspace::new();
        let first = head.matches('\n').count() as u64;
        let shown: Vec<(usize, u64)> = strict_session(&workspace, &format!("{head}{BODY}"))
            .into_iter()
            .map(|(line, _, _, severity, _)| ((line - first) as usize, severity))
            .collect();
        assert_eq!(shown, expected);
    }

    /// Criterion 34, editor side: a `when` guard narrows its `then` body
    /// until a `set` writes the subject, and never reaches another event.
    #[test]
    fn a_lifecycle_guard_ends_at_a_write_and_its_event() {
        let text = "---
$schema:
  count: number
count: 1
composed:
  - when: count
    then:
      - stdout: \"{{ min(count, 5) }}\"
      - set:
          count: 7
      - stdout: \"{{ min(count, 6) }}\"
finalize:
  - stdout: \"{{ min(count, 7) }}\"
---
Body
";
        let line_of = |needle: &str| text.lines().position(|line| line.contains(needle)).unwrap() as u64;
        let workspace = LspWorkspace::new();
        let found: Vec<(u64, u64, u64, u64)> = strict_session(&workspace, text)
            .into_iter()
            .map(|(line, start, end, severity, _)| (line, start, end, severity))
            .collect();
        let at = |needle: &str| {
            let line = line_of(needle);
            let (start, end) = columns(text, line, "count, ");
            (line, start, end - 2, WARNING)
        };
        assert_eq!(found, [at("min(count, 6)"), at("min(count, 7)")]);
    }
}

mod file_predicates {
    use super::*;

    const ALWAYS_FALSE: &str = "dm.expression.always_false_call";

    /// `(line, start, end, severity)` of every always-false diagnostic.
    fn always_false(diagnostics: &[Value]) -> Vec<(u64, u64, u64, u64)> {
        let mut found: Vec<_> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["code"] == json!(ALWAYS_FALSE))
            .map(|diagnostic| {
                let range = &diagnostic["range"];
                (
                    range["start"]["line"].as_u64().unwrap(),
                    range["start"]["character"].as_u64().unwrap(),
                    range["end"]["character"].as_u64().unwrap(),
                    diagnostic["severity"].as_u64().unwrap(),
                )
            })
            .collect();
        found.sort();
        found
    }

    const TEXT: &str = "---
$schema:
  path: string(required)
path: ./a.md
composed:
  - stdout: \"{{ file_exists('https://') }}\"
---
{{ file_exists(\"https://\") }}
{{ fileexists(\"\") }}
{{ file_exists(path) }}
{{ file_exists(\"./a.md\") }}
::block when=\"file_exists('vault:')\"
inside
::end-block
";

    /// Criteria 15 and 35, editor side: each malformed literal argument,
    /// in the body, a block condition, and a lifecycle action, and through
    /// the `fileexists` alias, is one warning ranged at the literal under
    /// both settings, with Darkmatter's code; a `string`-typed variable and a
    /// well-formed literal get none.
    #[test]
    fn a_malformed_literal_reference_warns_and_a_string_variable_does_not() {
        let line_of = |needle: &str| TEXT.lines().position(|line| line.contains(needle)).unwrap() as u64;
        let at = |needle: &str, literal: &str| {
            let line = line_of(needle);
            let (start, end) = columns(TEXT, line, literal);
            (line, start, end, WARNING)
        };
        let mut expected = vec![
            at("stdout:", "'https://'"),
            at("{{ file_exists(\"https://\") }}", "\"https://\""),
            at("fileexists", "\"\""),
            at("::block", "'vault:'"),
        ];
        expected.sort();
        for strict in [false, true] {
            let workspace = LspWorkspace::new();
            std::fs::write(workspace.path().join(".dmls.toml"), format!("[expressions]\nstrict = {strict}\n")).unwrap();
            let mut session = Session::open(&workspace, TEXT);
            let diagnostics = session.diagnostics();
            session.shutdown();
            assert_eq!(always_false(&diagnostics), expected, "strict = {strict}: {diagnostics:#?}");
            assert_eq!(calls(&diagnostics), [], "strict = {strict}: no category finding");
        }
    }

    /// Criterion 16, editor side: with `source: unknown`, only the unguarded
    /// `frontmatter` call warns under `expressions.strict`, and neither does
    /// by default.
    #[test]
    fn under_strict_only_the_unguarded_file_argument_warns() {
        let text = "---
$schema:
  source: unknown
source: ./a.md
---
{{ frontmatter(source, 'title') }}
{{ is_file_type(source) && frontmatter(source, 'title') }}
";
        let unguarded = text.lines().position(|line| line.starts_with("{{ frontmatter")).unwrap() as u64;
        let (start, end) = columns(text, unguarded, "source");
        for (strict, expected) in [
            (false, vec![]),
            (true, vec![(unguarded, start, end, WARNING, UNCERTAIN.to_string())]),
        ] {
            let workspace = LspWorkspace::new();
            std::fs::write(workspace.path().join(".dmls.toml"), format!("[expressions]\nstrict = {strict}\n")).unwrap();
            let mut session = Session::open(&workspace, text);
            assert_eq!(session.calls(), expected, "strict = {strict}");
            session.shutdown();
        }
    }

    /// Criterion 18: the configuration model has no `file_exists`-specific
    /// setting; `expressions` holds `strict` alone.
    #[test]
    fn the_configuration_has_no_file_exists_setting() {
        fn keys(value: &Value, prefix: &str, out: &mut Vec<String>) {
            if let Value::Object(map) = value {
                for (key, child) in map {
                    let path = format!("{prefix}{key}");
                    keys(child, &format!("{path}."), out);
                    out.push(path);
                }
            }
        }
        let config = serde_json::to_value(dmls::config::DmlsConfig::default()).unwrap();
        let mut all = Vec::new();
        keys(&config, "", &mut all);
        let named: Vec<_> = all.iter().filter(|key| key.contains("file_exists") || key.contains("fileexists")).collect();
        assert!(named.is_empty(), "{named:?}");
        assert_eq!(config["expressions"], json!({ "strict": false }));
    }
}

/// Criterion 19 (composition and editor legs) and the shared record behind
/// it: the same call reports the same code, severity, message, authored
/// position, and expected/actual types from composition (the rejection's
/// [`call_diagnostic`](darkmatter::markdown::MarkdownError::call_diagnostic),
/// a warning's `diagnostic`) and from DMLS, whose types travel in `data`.
mod shared_fields {
    use super::*;

    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::expression::{Diagnostic, DiagnosticSource, Severity};
    use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, warning_diagnostic};
    use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};

    /// The editor's diagnostic as the shared record, positions made
    /// one-based (UTF-8 columns; the fixtures are ASCII).
    fn editor_record(diagnostic: &Value, file: &std::path::Path) -> Diagnostic {
        let start = &diagnostic["range"]["start"];
        Diagnostic {
            code: diagnostic["code"].as_str().unwrap().to_string(),
            severity: match diagnostic["severity"].as_u64().unwrap() {
                ERROR => Severity::Error,
                WARNING => Severity::Warning,
                other => panic!("unexpected severity {other}"),
            },
            message: diagnostic["message"].as_str().unwrap().to_string(),
            source: Some(DiagnosticSource {
                file: Some(file.to_path_buf()),
                line: start["line"].as_u64().unwrap() as usize + 1,
                column: start["character"].as_u64().unwrap() as usize + 1,
            }),
            expected_type: diagnostic["data"]["expected_type"].as_str().map(str::to_string),
            actual_type: diagnostic["data"]["actual_type"].as_str().map(str::to_string),
        }
    }

    fn editor_calls(session: &mut Session<'_>, file: &std::path::Path) -> Vec<Diagnostic> {
        session
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic["code"] == json!(INCOMPATIBLE) || diagnostic["code"] == json!(UNCERTAIN))
            .map(|diagnostic| editor_record(diagnostic, file))
            .collect()
    }

    /// A request anchored at the test's own workspace, never the checkout.
    fn request(dir: &std::path::Path, path: &std::path::Path) -> ComposeRequest {
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
            .with_source_file(path.to_path_buf());
        let snapshot = darkmatter::markdown::compose::RequestSnapshot::new(dir).with_env(options.context().env().clone());
        ComposeRequest::prepare(options, &snapshot).expect("test request")
    }

    #[test]
    fn a_rejected_call_reports_the_editor_fields() {
        let text = "---\n$schema:\n  flag: boolean(required)\nflag: true\ntitle: \"{{ min(flag, 2) }}\"\n---\n\nBody {{ min(flag, 5) }}\n";
        for (label, document) in [
            ("body", text.replace("title: \"{{ min(flag, 2) }}\"\n", "")),
            ("frontmatter", text.replace("Body {{ min(flag, 5) }}", "Body")),
        ] {
            let workspace = LspWorkspace::new();
            let mut session = Session::open(&workspace, &document);
            let path = workspace.path().join("doc.md");
            let editor = editor_calls(&mut session, &path);
            session.shutdown();

            let error = Markdown::try_from(path.as_path())
                .unwrap()
                .compose_with(&request(workspace.path(), &path))
                .expect_err("a category 5 call rejects the document");
            let composed = error.call_diagnostic().unwrap_or_else(|| panic!("{label}: no call diagnostic: {error}"));
            assert_eq!(editor, vec![composed.clone()], "{label}");
            // `md compose --output report-json` writes these records.
            assert_eq!(error_diagnostics(&error), editor, "{label}: the report's records");
            assert_eq!(composed.severity, Severity::Error, "{label}");
            assert_eq!(composed.expected_type.as_deref(), Some("number"), "{label}");
            assert_eq!(composed.actual_type.as_deref(), Some("boolean"), "{label}");
        }
    }

    /// Clean controls: a page-block or transclusion condition's rejection
    /// already reports the editor's record.
    #[test]
    fn a_rejected_condition_reports_the_editor_fields() {
        for (label, document) in [
            ("page block", "---\ntitle: t\n---\n::block when=\"min(true, 5) > 1\"\nshown\n::end-block\n"),
            ("transclusion", "---\ntitle: t\n---\n::file ./other.md when=\"min(true, 5) > 1\"\n"),
        ] {
            let workspace = LspWorkspace::new();
            std::fs::write(workspace.path().join("other.md"), "other\n").unwrap();
            let mut session = Session::open(&workspace, document);
            let path = workspace.path().join("doc.md");
            let editor = editor_calls(&mut session, &path);
            session.shutdown();

            let error = Markdown::try_from(path.as_path())
                .unwrap()
                .compose_with(&request(workspace.path(), &path))
                .expect_err("a category 5 condition rejects the document");
            assert_eq!(error_diagnostics(&error), editor, "{label}: the report's records");
            let [record] = editor.as_slice() else { panic!("{label}: one record: {editor:#?}") };
            assert_eq!(record.code, INCOMPATIBLE, "{label}");
            assert_eq!(record.source.as_ref().map(|source| source.line), Some(4), "{label}");
            assert_eq!(record.expected_type.as_deref(), Some("number"), "{label}");
            assert_eq!(record.actual_type.as_deref(), Some("literal true"), "{label}");
        }
    }

    /// The lifecycle rejection of `document`, written to `doc.md` in a
    /// fresh workspace, beside the editor's call records for the same bytes:
    /// `(report records, editor records, workspace)`.
    fn lifecycle_rejection(document: &str) -> (Vec<Diagnostic>, Vec<Diagnostic>, LspWorkspace) {
        use darkmatter::lifecycle::md_host;

        let workspace = LspWorkspace::new();
        let mut session = Session::open(&workspace, document);
        let path = workspace.path().join("doc.md");
        let editor = editor_calls(&mut session, &path);
        session.shutdown();

        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let frontmatter: Value =
            markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
        let config = md_host::parse_lifecycle(&frontmatter, &path, None).expect("the lifecycle parses");
        let error = darkmatter::lifecycle::reject_incompatible_calls(
            &frontmatter,
            &config,
            &path,
            markdown.loaded_text(),
            md_host::registry(),
            None,
        )
        .expect_err("a category 5 lifecycle call rejects the document");
        (error_diagnostics(&error), editor, workspace)
    }

    /// One test per lifecycle surface: the rejection's report records equal
    /// the editor's for the same bytes, all six fields, at `(line, column)`.
    macro_rules! lifecycle_rejection_parity {
        ($($name:ident: $item:expr, ($line:expr, $column:expr);)+) => {$(
            #[test]
            fn $name() {
                let (reported, editor, workspace) = lifecycle_rejection(&$item);
                assert_eq!(reported, editor, "the report's records are the editor's");
                let [record] = reported.as_slice() else { panic!("one record: {reported:#?}") };
                assert_eq!(record.code, INCOMPATIBLE);
                assert_eq!(record.severity, Severity::Error);
                assert_eq!(
                    record.source,
                    Some(DiagnosticSource { file: Some(workspace.path().join("doc.md")), line: $line, column: $column })
                );
                assert_eq!(record.expected_type.as_deref(), Some("number"));
                assert_eq!(record.actual_type.as_deref(), Some("literal true"));
                // Rejection is passive: no lifecycle effect ran.
                assert!(!workspace.path().join("marker").exists());
            }
        )+};
    }

    fn action(event: &str) -> String {
        format!("---\n{event}:\n  - stdout: \"{{{{ min(true, 5) }}}}\"\n---\nBody\n")
    }

    fn condition(event: &str) -> String {
        format!("---\n{event}:\n  - when: min(true, 5)\n    then:\n      - stdout: hi\n---\nBody\n")
    }

    fn composed(item: &str) -> String {
        format!("---\ncomposed:\n  - {item}\n---\nBody\n")
    }

    lifecycle_rejection_parity! {
        initialize_action_rejection_reports_the_editor_fields: action("initialize"), (3, 21);
        composed_action_rejection_reports_the_editor_fields: action("composed"), (3, 21);
        failed_action_rejection_reports_the_editor_fields: action("failed"), (3, 21);
        cancelled_action_rejection_reports_the_editor_fields: action("cancelled"), (3, 21);
        finalize_action_rejection_reports_the_editor_fields: action("finalize"), (3, 21);
        initialize_condition_rejection_reports_the_editor_fields: condition("initialize"), (3, 15);
        composed_condition_rejection_reports_the_editor_fields: condition("composed"), (3, 15);
        failed_condition_rejection_reports_the_editor_fields: condition("failed"), (3, 15);
        cancelled_condition_rejection_reports_the_editor_fields: condition("cancelled"), (3, 15);
        finalize_condition_rejection_reports_the_editor_fields: condition("finalize"), (3, 15);
        shell_rejection_reports_the_editor_fields: composed("shell: \"{{ min(true,5) }}\""), (3, 20);
        ensure_file_rejection_reports_the_editor_fields: composed("ensure_file: [\"marker\", \"{{ min(true,5) }}\"]"), (3, 37);
        set_rejection_reports_the_editor_fields: composed("set: {count: \"{{ min(true,5) }}\"}"), (3, 26);
        error_reason_rejection_reports_the_editor_fields: composed("error: \"{{ min(true,5) }}\""), (3, 20);
    }

    #[test]
    fn a_category_3_warning_reports_the_editor_fields() {
        let document = "---\n$schema:\n  label: string\nlabel: \"7\"\ntitle: \"{{ min(label, 9) }}\"\n---\n\nBody {{ min(label, 5) }}\n";
        let workspace = LspWorkspace::new();
        let mut session = Session::open(&workspace, document);
        let path = workspace.path().join("doc.md");
        let editor = editor_calls(&mut session, &path);
        session.shutdown();

        let (_, report) = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&request(workspace.path(), &path))
            .unwrap_or_else(|error| panic!("a category 3 call composes: {error}"));
        let composed: Vec<Diagnostic> = report.warnings.iter().filter_map(|warning| warning.diagnostic.as_deref().cloned()).collect();
        assert_eq!(composed.len(), 2, "{:#?}", report.warnings);
        assert_eq!(editor, composed);
        let reported: Vec<Diagnostic> = report.warnings.iter().filter(|warning| warning.diagnostic.is_some()).map(warning_diagnostic).collect();
        assert_eq!(reported, editor, "the report's records");
        assert!(composed.iter().all(|diagnostic| diagnostic.severity == Severity::Warning && diagnostic.code == UNCERTAIN));
    }
}
