//! DMLS reads guard-narrowed types through Darkmatter's one type scope
//! (`2026-10-16-type-narrowing`, acceptance 7 and 13).
//!
//! Each hover test computes the same site's type with the library directly
//! (`TypeScope::assume` / `BodyGuards` and `TypeScope::type_of`) and asserts
//! the editor shows exactly that type. Diagnostics are compared with the
//! library's own static analysis. Every request runs in an in-process session,
//! and none evaluates a guard.

use crate::common;

// DMLS reads the rows; the matrix-only parts of the file go unused here.
#[allow(dead_code)]
#[path = "../../../lib/tests/common/type_scope_cases.rs"]
mod cases;

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::directive_targets::{TargetNullability, analyze_directive_targets};
use darkmatter::markdown::compose::expression::{
    DeclaredTypes, Polarity, ScopedType, Subject, TypeScope, parse, parse_condition, parse_spanned,
    static_variable_reads,
};
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::compose::page_blocks::BodyGuards;
use darkmatter::markdown::schemas::{DarkmatterSchemas, SimplifiedSchema, parse_yaml_schema};
use dmls::overlay::expressions::type_text;
use serde_json::{Value, json};

/// `target: string | number | null` and a required enum, as in the spec's
/// guard example.
const SCHEMA: &str = "target: [string, number, \"null\"]\nmode: \"enum(release, debug, test; required)\"\n";

fn initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "capabilities": {
            "general": { "positionEncodings": ["utf-8", "utf-16"] },
            "textDocument": { "foldingRange": { "lineFoldingOnly": true } }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
    })
}

/// A document whose frontmatter declares [`SCHEMA`] (indented under
/// `$schema`) plus `extra` frontmatter lines, followed by `body`.
fn document(extra: &str, body: &str) -> String {
    let schema: String = SCHEMA.lines().map(|line| format!("  {line}\n")).collect();
    format!("---\n$schema:\n{schema}{extra}---\n\n{body}")
}

fn schema() -> SimplifiedSchema {
    parse_yaml_schema(&serde_yaml_ng::from_str(SCHEMA).unwrap()).unwrap()
}

/// An open in-process session on one document.
struct Session<'w> {
    fixture: LspFixture<'w>,
    uri: String,
    text: String,
    diagnostics: Vec<Value>,
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
        let diagnostics = fixture.wait_for_diagnostics(&uri);
        Self {
            fixture,
            uri,
            text: text.to_string(),
            diagnostics,
        }
    }

    /// The hover Markdown at byte `offset` of the document.
    fn hover(&mut self, offset: usize) -> String {
        let (line, character) = position(&self.text, offset);
        let response = self.fixture.request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": self.uri }, "position": { "line": line, "character": character } }),
        );
        response.result.expect("hover result")["contents"]["value"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    /// `(line, character)` ranges of every diagnostic with `code`, sorted.
    fn ranges(&self, code: &str) -> Vec<(u64, u64)> {
        let mut ranges: Vec<(u64, u64)> = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["code"] == json!(code))
            .map(|diagnostic| {
                let start = &diagnostic["range"]["start"];
                (start["line"].as_u64().unwrap(), start["character"].as_u64().unwrap())
            })
            .collect();
        ranges.sort();
        ranges
    }

    fn shutdown(self) {
        self.fixture.shutdown();
    }
}

/// `(line, character)` of an ASCII byte offset.
fn position(text: &str, offset: usize) -> (u32, u32) {
    let line = text[..offset].matches('\n').count();
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    (line as u32, (offset - line_start) as u32)
}

/// Byte offset of the `nth` (0-based) occurrence of `needle` after the first
/// occurrence of `after`.
fn offset_of(text: &str, after: &str, needle: &str, nth: usize) -> usize {
    let base = text.find(after).unwrap_or_else(|| panic!("no `{after}`"));
    text[base..]
        .match_indices(needle)
        .nth(nth)
        .map(|(index, _)| base + index)
        .unwrap_or_else(|| panic!("no occurrence {nth} of `{needle}` after `{after}`"))
}

fn subject(path: &str) -> Subject {
    Subject::document_path(path).unwrap()
}

/// Asserts `hover` names exactly the library's narrowed type.
fn assert_shows(hover: &str, library: &ScopedType, expected_text: &str) {
    assert!(library.is_refined() || library.guard_presence().is_some(), "{library:?}");
    assert_eq!(type_text(library), expected_text, "{library:?}");
    assert!(hover.contains(&format!("Type here: `{expected_text}`")), "{hover}");
}

/// Acceptance 7, body sites: the hover inside a `::block when` shows the
/// type the library's `BodyGuards` scope gives the same offset, for a prose
/// interpolation, a `::shell` hole, and a `::shell-block` hole; outside the
/// block nothing is narrowed.
#[test]
fn hover_inside_a_body_block_guard_shows_the_library_type() {
    let body = concat!(
        "::block when=\"is_string(target)\"\n",
        "Branch {{ target }}.\n",
        "::shell git checkout {{ target }}\n",
        "::end-block\n",
        "::block when=\"!is_string(target) && target\"\n",
        "::shell-block\n",
        "gh pr checkout {{ target }}\n",
        "::end-block\n",
        "::end-block\n",
        "::block when=\"mode != 'release'\"\n",
        "Mode {{ mode }}.\n",
        "::end-block\n",
        "Plain {{ target }}.\n",
    );
    let text = document("", body);
    let schema = schema();
    let declared = DeclaredTypes::new(Some(&schema));
    let guards = BodyGuards::scan(&text);
    let library = |offset: usize, path: &str| {
        guards
            .scope_at(TypeScope::unguarded(&declared), offset)
            .type_of(&subject(path))
    };

    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);

    let prose = offset_of(&text, "Branch {{", "target", 0);
    assert_shows(&session.hover(prose), &library(prose, "target"), "string");

    let shell = offset_of(&text, "::shell git checkout", "target", 0);
    assert_shows(&session.hover(shell), &library(shell, "target"), "string");

    let shell_block = offset_of(&text, "gh pr checkout", "target", 0);
    assert_shows(&session.hover(shell_block), &library(shell_block, "target"), "number");

    let mode = offset_of(&text, "Mode {{", "mode", 0);
    let mode_type = library(mode, "mode");
    let mode_hover = session.hover(mode);
    assert_shows(&mode_hover, &mode_type, &type_text(&mode_type));
    assert!(!type_text(&mode_type).contains("release"), "{mode_type:?}");

    let plain = offset_of(&text, "Plain {{", "target", 0);
    assert!(!library(plain, "target").is_refined());
    assert!(!session.hover(plain).contains("Type here"));
    session.shutdown();
}

/// Strict equality at body sites: inside `target != null` the hover drops
/// `null`; `target != ''` proves nothing because a missing value is `null`
/// and `null != ''`; `target == null` leaves only `null`; and equality with
/// an array literal keeps only the array arm of a `string[] | string` union.
/// Each expected text is hard-coded as well as read from the library, so the
/// two cannot agree on a wrong answer.
#[test]
fn hover_inside_strict_equality_guards_shows_the_library_type() {
    const TAGS: &str = "tags: [\"string[]\", string]\n";
    let body = concat!(
        "::block when=\"target != null\"\n",
        "Present {{ target }}.\n",
        "::end-block\n",
        "::block when=\"target != ''\"\n",
        "Nonempty {{ target }}.\n",
        "::end-block\n",
        "::block when=\"target == null\"\n",
        "Null {{ target }}.\n",
        "::end-block\n",
        "::block when=\"tags == []\"\n",
        "Tags {{ tags }}.\n",
        "::end-block\n",
    );
    let text = document(&format!("  {TAGS}"), body);
    let schema = parse_yaml_schema(&serde_yaml_ng::from_str(&format!("{SCHEMA}{TAGS}")).unwrap()).unwrap();
    let declared = DeclaredTypes::new(Some(&schema));
    let guards = BodyGuards::scan(&text);
    let library = |offset: usize, path: &str| {
        guards
            .scope_at(TypeScope::unguarded(&declared), offset)
            .type_of(&subject(path))
    };

    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);

    let present = offset_of(&text, "Present {{", "target", 0);
    assert_shows(&session.hover(present), &library(present, "target"), "number | string");

    let nonempty = offset_of(&text, "Nonempty {{", "target", 0);
    let nonempty_type = library(nonempty, "target");
    assert!(nonempty_type.guard_presence().is_none(), "{nonempty_type:?}");
    assert!(!nonempty_type.is_refined(), "{nonempty_type:?}");
    assert_eq!(type_text(&nonempty_type), "number | string | null");
    let nonempty_hover = session.hover(nonempty);
    assert!(!nonempty_hover.contains("Type here"), "{nonempty_hover}");

    let null = offset_of(&text, "Null {{", "target", 0);
    assert_shows(&session.hover(null), &library(null, "target"), "null");

    let tags = offset_of(&text, "Tags {{", "tags", 0);
    assert_shows(&session.hover(tags), &library(tags, "tags"), "string[]");
    session.shutdown();
}

/// Acceptance 7, expression-internal sites: a ternary branch and a later `&&`
/// operand show the library's `TypeScope::assume` type for that side, in a
/// body interpolation and in an Expression-typed frontmatter value (whose
/// spans go through the YAML scalar projection). Both ternary branches are
/// analyzed, each with its own side.
#[test]
fn hover_inside_ternary_and_conjunction_shows_the_library_type() {
    let text = document(
        "  when: expression\nwhen: \"is_string(target) && target\"\n",
        concat!(
            "True {{ is_string(target) ? target : 'none' }}.\n",
            "False {{ is_string(target) ? 'some' : target }}.\n",
            "And {{ is_number(target) && target }}.\n",
        ),
    );
    let schema = schema();
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let side = |guard: &str, polarity| {
        site.assume(&parse_condition(guard).unwrap(), polarity)
            .type_of(&subject("target"))
    };

    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);

    let then_branch = offset_of(&text, "True {{", "target", 1);
    assert_shows(&session.hover(then_branch), &side("is_string(target)", Polarity::True), "string");

    let else_branch = offset_of(&text, "False {{", "target", 1);
    assert_shows(&session.hover(else_branch), &side("is_string(target)", Polarity::False), "number | null");

    let conjunction = offset_of(&text, "And {{", "target", 1);
    assert_shows(&session.hover(conjunction), &side("is_number(target)", Polarity::True), "number");

    // The condition operand itself is not narrowed by its own guard.
    let condition = offset_of(&text, "True {{", "target", 0);
    assert!(!session.hover(condition).contains("Type here"));

    let frontmatter = offset_of(&text, "when: \"is_string", "target", 1);
    assert_shows(&session.hover(frontmatter), &side("is_string(target)", Polarity::True), "string");
    session.shutdown();
}

/// `dm.transclusion.nullable_target` follows the library's predicate
/// registry: aliases, `!isNull`, `!is_empty`, and equality with a non-null
/// literal each prove the target present, as `file_exists` always did.
/// Equality is strict and a missing value is `null`, so `x != null` and
/// `!(x == null)` prove `x` present, while `x != ''` (`null != ''` holds) and
/// `x == null` do not.
#[test]
fn nullable_target_is_suppressed_by_registry_predicate_guards() {
    let text = concat!(
        "---\n$schema:\n",
        "  optional: file\n  a: file\n  b: file\n  c: file\n  d: file\n  e: file\n",
        "  f: file\n  g: file\n  h: file\n  i: file\n",
        "---\n\n",
        "::file {{optional}}\n\n",
        "::block when=\"fileExists(a)\"\n::file {{a}}\n::end-block\n",
        "::block when=\"isString(b)\"\n::file {{b}}\n::end-block\n",
        "::block when=\"!isNull(c)\"\n::file {{c}}\n::end-block\n",
        "::block when=\"!is_empty(d)\"\n::file {{d}}\n::end-block\n",
        "::block when=\"e == 'main.md'\"\n::file {{e}}\n::end-block\n",
        "::block when=\"is_null(a)\"\n::file {{a}}\n::end-block\n",
        "::block when=\"f != null\"\n::file {{f}}\n::end-block\n",
        "::block when=\"!(g == null)\"\n::file {{g}}\n::end-block\n",
        "::block when=\"h != ''\"\n::file {{h}}\n::end-block\n",
        "::block when=\"i == null\"\n::file {{i}}\n::end-block\n",
    );
    let workspace = LspWorkspace::new();
    let session = Session::open(&workspace, text);

    let unguarded = position(text, text.find("{{optional}}").unwrap());
    let false_side = position(text, text.rfind("{{a}}").unwrap());
    let not_empty_string = position(text, text.find("{{h}}").unwrap());
    let equals_null = position(text, text.find("{{i}}").unwrap());
    let expected: Vec<(u64, u64)> = [unguarded, false_side, not_empty_string, equals_null]
        .into_iter()
        .map(|(line, character)| (u64::from(line), u64::from(character)))
        .collect();
    assert_eq!(session.ranges("dm.transclusion.nullable_target"), expected, "{:#?}", session.diagnostics);
    session.shutdown();
}

/// `dm.expression.undeclared_property` reports exactly the reads the
/// library's static walk leaves unhandled: a direct `&&` operand is handled
/// (`and` declares `unknown | null`), a later operand
/// is covered by the earlier one's facts, a false ternary branch is not
/// covered by its condition, a fact about `bar.a` does not cover its sibling
/// `bar.b`, and under strict equality `baz != null` covers `baz.name` while
/// `qux != ''` does not cover `qux.name` (a missing `qux` is `null`, and
/// `null != ''`). Each read sits in a comparison, an ordinary operand, so
/// only facts cover it.
#[test]
fn undeclared_property_findings_match_the_library_branch_facts() {
    let text = concat!(
        "---\ntitle: T\n---\n\n",
        "A {{ foo && foo == 'n' }}\n",
        "B {{ foo ? 'x' : foo }}\n",
        "C {{ bar.a && bar.b == 'n' }}\n",
        "D {{ baz != null && baz.name == 'n' }}\n",
        "E {{ qux != '' && qux.name == 'n' }}\n",
    );
    let workspace = LspWorkspace::new();
    let session = Session::open(&workspace, text);

    let mut library: Vec<(u64, u64)> = Vec::new();
    for (prefix, inner) in [
        ("A {{ ", "foo && foo == 'n'"),
        ("B {{ ", "foo ? 'x' : foo"),
        ("C {{ ", "bar.a && bar.b == 'n'"),
        ("D {{ ", "baz != null && baz.name == 'n'"),
        ("E {{ ", "qux != '' && qux.name == 'n'"),
    ] {
        let base = text.find(prefix).unwrap() + prefix.len();
        let expr = parse_spanned(inner).unwrap();
        // `null` parses as a read of the reserved root, never a property.
        for read in static_variable_reads(&expr)
            .into_iter()
            .filter(|read| !read.handles_absence && read.root() != "null")
        {
            let (line, character) = position(text, base + read.span.start);
            library.push((u64::from(line), u64::from(character)));
        }
    }
    library.sort();

    let expected: Vec<(u64, u64)> = [
        offset_of(text, "B {{", "foo", 1),
        offset_of(text, "C {{", "bar.b", 0),
        offset_of(text, "D {{", "baz", 0),
        offset_of(text, "E {{", "qux", 0),
        offset_of(text, "E {{", "qux.name", 0),
    ]
    .into_iter()
    .map(|offset| {
        let (line, character) = position(text, offset);
        (u64::from(line), u64::from(character))
    })
    .collect();
    assert_eq!(library, expected);
    assert_eq!(session.ranges("dm.expression.undeclared_property"), library, "{:#?}", session.diagnostics);
    session.shutdown();
}

/// DMLS takes its branch types from `TypeScope` alone, so a guard call the
/// passive call check rejects narrows nothing in the editor either: the
/// conjunction's later `foo` keeps `unknown` (no "Type here" line) after
/// `probe_pred(foo, {})`, and reads `file` after the valid call. The probe is
/// a thread-local test function, so this calls `local_type` directly.
#[cfg(feature = "effects-instrumentation")]
#[test]
fn local_type_takes_no_fact_from_a_rejected_guard_call() {
    use std::sync::Arc;

    use darkmatter::markdown::compose::expression::SpannedExprKind;
    use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
    use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
    use dmls::overlay::expressions::local_type;

    let signature = "function(parameters(value: unknown, count: number); returns(boolean); narrows(value: file; true-implies))";
    let registration = FunctionRegistration::builder("probe_pred", signature).build().expect("registers");
    let _probe = hooks::register(registration, Arc::new(|_| Ok(json!(true))));
    let schema = parse_yaml_schema(&serde_yaml_ng::from_str("foo: unknown").unwrap()).unwrap();
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);
    let later_foo = |source: &str| {
        let expr = parse_spanned(source).unwrap();
        let SpannedExprKind::FunctionCall { args, .. } = &expr.kind else {
            panic!("`&&` lowers to `and(...)`");
        };
        local_type(&expr, &args[1], &site).map(|ty| type_text(&ty))
    };
    assert_eq!(later_foo("probe_pred(foo, 1) && foo").as_deref(), Some("file"));
    assert_eq!(later_foo("probe_pred(foo, {}) && foo"), None);
    assert_eq!(later_foo("probe_pred(foo) && foo"), None);
    assert_eq!(hooks::handler_entries("probe_pred"), 0);
}

/// Acceptance 13: a `file_exists` guard narrows statically. A target that is
/// a remote URL or a path that does not exist is still a present string
/// inside the guard (nothing asked whether it exists), and analysis attempts
/// no network access and creates no file.
#[test]
fn a_file_exists_guard_narrows_without_touching_the_filesystem_or_network() {
    #[cfg(feature = "effects-instrumentation")]
    let before = darkmatter::effects::network_attempt_count();

    let body = concat!(
        "::block when=\"file_exists(remote)\"\n",
        "Remote {{ remote }}.\n",
        "::file {{remote}}\n",
        "::end-block\n",
        "::block when=\"file_exists(nearby)\"\n",
        "Nearby {{ nearby }}.\n",
        "::file {{nearby}}\n",
        "::end-block\n",
    );
    let text = format!(
        "---\n$schema:\n  remote: file\n  nearby: file\n---\n\n{body}"
    );
    let workspace = LspWorkspace::new();
    let listing = || {
        let mut names: Vec<String> = std::fs::read_dir(workspace.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    let mut session = Session::open(&workspace, &text);
    let files_before = listing();

    for (prefix, name) in [("Remote {{", "remote"), ("Nearby {{", "nearby")] {
        let hover = session.hover(offset_of(&text, prefix, name, 0));
        assert!(hover.contains("Type here: `file`"), "{name}: {hover}");
    }
    assert!(session.ranges("dm.transclusion.nullable_target").is_empty(), "{:#?}", session.diagnostics);
    session.shutdown();

    assert_eq!(listing(), files_before, "analysis created or removed files");
    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(darkmatter::effects::network_attempt_count(), before, "a file_exists guard attempted network access");
}

// ── the conformance matrix's shared guard rows ──────────────────────────

/// Declares [`DmlsSite`] and [`DmlsSite::ALL`] from one list.
macro_rules! dmls_sites {
    ($($variant:ident),+ $(,)?) => {
        /// An editor surface that could show a guard-narrowed type.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum DmlsSite {
            $($variant),+
        }

        impl DmlsSite {
            const ALL: &'static [DmlsSite] = &[$(DmlsSite::$variant),+];
        }
    };
}

dmls_sites! {
    BodyBlockHover,
    ExpressionBranchHover,
    FrontmatterExpressionHover,
    TransclusionTargetDiagnostic,
    LifecycleBranchDiagnostic,
    LifecycleBranchHover,
    TaskBody,
}

/// `Ok(test)` for a site DMLS supports, with the test proving it agrees with
/// the library; `Err(reason)` for one it does not support yet.
fn dmls_state(site: DmlsSite) -> Result<&'static str, &'static str> {
    const UNGUARDED_HOVER: &str = "planned: hover inside a lifecycle `then`/`else` body, and every site of a sequence \
         task body, shows the declared type, not the type a `when` narrows (docs/hover.md)";
    match site {
        DmlsSite::BodyBlockHover | DmlsSite::TransclusionTargetDiagnostic => {
            Ok("body_guard_rows_agree_with_the_library_in_hover_and_nullable_targets")
        }
        DmlsSite::ExpressionBranchHover | DmlsSite::FrontmatterExpressionHover => {
            Ok("expression_branch_rows_agree_with_the_library")
        }
        DmlsSite::LifecycleBranchDiagnostic => Ok("lifecycle_host_profile::claudine_events_narrow_until_a_write_with_stdout"),
        DmlsSite::LifecycleBranchHover | DmlsSite::TaskBody => Err(UNGUARDED_HOVER),
    }
}

/// Every editor surface has an explicit state: supported ones name their
/// agreement test, and lifecycle `then`/`else` hover and task bodies are
/// planned, not claimed.
#[test]
fn every_dmls_site_has_an_explicit_state() {
    for &site in DmlsSite::ALL {
        match dmls_state(site) {
            Ok(test) => assert!(!test.is_empty(), "{site:?}"),
            Err(reason) => {
                assert!(reason.starts_with("planned:"), "{site:?}: {reason}");
                assert!(matches!(site, DmlsSite::LifecycleBranchHover | DmlsSite::TaskBody), "{site:?}");
            }
        }
    }
}

fn cases_schema() -> SimplifiedSchema {
    parse_yaml_schema(&serde_yaml_ng::from_str(cases::SCHEMA).unwrap()).unwrap()
}

/// Asserts the hover at `offset` shows the library's local type exactly when
/// the library has one, and that the library agrees with the shared row.
fn assert_agrees(session: &mut Session<'_>, offset: usize, library: &ScopedType, row: &cases::BlockCase, surface: &str) {
    assert_eq!(library.guard_presence().is_some(), row.proven, "{surface} {row:?}: the library disagrees with the row");
    let hover = session.hover(offset);
    if library.is_refined() || library.guard_presence().is_some() {
        let expected = format!("Type here: `{}`", type_text(library));
        assert!(hover.contains(&expected), "{surface} {row:?}: expected {expected} in {hover}");
    } else {
        assert!(!hover.contains("Type here"), "{surface} {row:?}: {hover}");
    }
}

/// Acceptance 7 over the matrix's shared rows: inside each row's
/// `::block when=` guards, the hover on a prose interpolation shows exactly
/// the library's `BodyGuards` type, and `dm.transclusion.nullable_target`
/// fires exactly where the library's target analysis finds a nullable,
/// unnarrowed target.
#[test]
fn body_guard_rows_agree_with_the_library_in_hover_and_nullable_targets() {
    let _ = include_str!("../../../lib/tests/common/type_scope_cases.rs");
    let mut body = String::new();
    for (index, row) in cases::BLOCK_CASES.iter().enumerate() {
        let inner = format!("R{index} {{{{ {} }}}}.\n::file {{{{{}}}}}\n", row.read, row.read);
        body.push_str(&cases::wrap(row, &inner));
    }
    let text = format!("{}\n{body}", cases::frontmatter(""));
    let schema = cases_schema();
    let declared = DeclaredTypes::new(Some(&schema));
    let guards = BodyGuards::scan(&text);

    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);
    for (index, row) in cases::BLOCK_CASES.iter().enumerate() {
        let start = offset_of(&text, &format!("R{index} {{{{"), row.read, 0);
        let library = guards
            .scope_at(TypeScope::unguarded(&declared), start)
            .type_of_expr(&parse(row.read).unwrap())
            .expect("a stable subject");
        assert_agrees(&mut session, start + row.read.len() - 1, &library, row, "body block");
    }

    let context = build_resolution_context(
        &RequestSnapshot::new(workspace.path()).with_home(Some(workspace.path().join("home"))),
    )
    .expect("library context");
    let effective = DarkmatterSchemas::new(context)
        .effective_for(&Markdown::from(text.as_str()))
        .expect("schema resolves")
        .expect("effective schema");
    let analyses = analyze_directive_targets(&text, Some(&effective), &json!({})).unwrap();
    assert_eq!(analyses.len(), cases::BLOCK_CASES.len());
    let mut library: Vec<(u64, u64)> = analyses
        .iter()
        .filter(|analysis| matches!(analysis.nullability, TargetNullability::Nullable { .. }) && !analysis.is_narrowed())
        .map(|analysis| {
            let (line, character) = position(&text, analysis.expression_span.start);
            (u64::from(line), u64::from(character))
        })
        .collect();
    library.sort();
    assert!(!library.is_empty(), "non-vacuity: some row leaves a nullable target unnarrowed");
    assert!(analyses.iter().any(|analysis| analysis.is_narrowed()), "non-vacuity: some row narrows its target");
    assert_eq!(session.ranges("dm.transclusion.nullable_target"), library, "{:#?}", session.diagnostics);
    session.shutdown();
}

/// Acceptance 7 over the shared single-guard rows at the expression-internal
/// sites: the then-branch of `(G) ? read : …` and the right operand of
/// `(G) && read`, in a body interpolation and in an Expression-typed
/// frontmatter value, each hover the library's `TypeScope::assume` type.
macro_rules! expression_branch_groups {
    ($($name:ident: $start:literal..$end:literal),+ $(,)?) => {
        const EXPRESSION_BRANCH_GROUPS: &[(usize, usize)] = &[$(($start, $end)),+];

        $(
            #[test]
            fn $name() {
                let _ = include_str!("../../../lib/tests/common/type_scope_cases.rs");
                let rows: Vec<&cases::BlockCase> = cases::BLOCK_CASES
                    .iter()
                    .filter(|row| row.guards.len() == 1)
                    .collect();
                expression_branch_rows(&rows[$start..$end]);
            }
        )+
    };
}

expression_branch_groups! {
    expression_branch_rows_agree_with_the_library: 0..7,
    expression_branch_rows_agree_with_the_library_second_group: 7..14,
    expression_branch_rows_agree_with_the_library_third_group: 14..21,
    expression_branch_rows_agree_with_the_library_fourth_group: 21..28,
}

#[test]
fn every_single_guard_row_is_scheduled_once_in_a_bounded_expression_group() {
    let _ = include_str!("../../../lib/tests/common/type_scope_cases.rs");
    let count = cases::BLOCK_CASES.iter().filter(|row| row.guards.len() == 1).count();
    let mut next = 0;
    for &(start, end) in EXPRESSION_BRANCH_GROUPS {
        assert_eq!(start, next, "groups must cover the single-guard rows without gaps or overlap");
        assert!((1..=7).contains(&(end - start)), "each group must contain one to seven rows");
        next = end;
    }
    assert_eq!(next, count, "schedule every single-guard row when adding cases");
}

fn expression_branch_rows(rows: &[&cases::BlockCase]) {
    let mut schema_lines: String = cases::SCHEMA.lines().map(|line| format!("  {line}\n")).collect();
    let mut values = String::new();
    let mut body = String::new();
    for (index, row) in rows.iter().enumerate() {
        let guard = row.guards[0];
        schema_lines.push_str(&format!("  fe{index}: expression\n"));
        values.push_str(&format!("fe{index}: \"({guard}) ? {} : 'none'\"\n", row.read));
        body.push_str(&format!("T{index} {{{{ ({guard}) ? {} : 'none' }}}}.\n", row.read));
        body.push_str(&format!("A{index} {{{{ ({guard}) && {} }}}}.\n", row.read));
    }
    let text = format!("---\n$schema:\n{schema_lines}{values}---\n\n{body}");
    let schema = cases_schema();
    let declared = DeclaredTypes::new(Some(&schema));
    let site = TypeScope::unguarded(&declared);

    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);
    for (index, row) in rows.iter().enumerate() {
        let guard = row.guards[0];
        let library = site
            .assume(&parse_condition(guard).unwrap(), Polarity::True)
            .type_of_expr(&parse(row.read).unwrap())
            .expect("a stable subject");
        for (prefix, operator) in [
            (format!("T{index} {{{{ "), " ? "),
            (format!("A{index} {{{{ "), " && "),
            (format!("\nfe{index}: \""), " ? "),
        ] {
            let start = text.find(&prefix).unwrap() + prefix.len() + guard.len() + 2 + operator.len();
            assert_eq!(&text[start..start + row.read.len()], row.read, "{prefix}");
            assert_agrees(&mut session, start + row.read.len() - 1, &library, row, prefix.trim());
        }
    }
    session.shutdown();
}

/// Criterion 21, editor side: `dm.expression.undeclared_property` follows the
/// library's type rule. A direct argument to `is_null`, `is_empty`,
/// `file_exists` (each `unknown | null`), `trim`, or `lower` (each
/// `string | null`, whatever they return) is handled; `min(x, 5)`'s `number`
/// parameter and `is_empty(trim(x))`'s nested read are not, so they warn as
/// composition does.
#[test]
fn undeclared_property_findings_follow_the_parameter_types() {
    let text = concat!(
        "---\ntitle: T\n---\n\n",
        "A {{ is_null(a) }} {{ is_empty(b) }} {{ file_exists(c) }}\n",
        "B {{ min(d, 5) }} {{ is_empty(trim(e)) }}\n",
        "C {{ trim(f) }} {{ lower(g) }}\n",
    );
    let workspace = LspWorkspace::new();
    let session = Session::open(&workspace, text);
    let expected: Vec<(u64, u64)> = [offset_of(text, "B {{", "d", 0), offset_of(text, "B {{", "trim(e", 0) + "trim(".len()]
        .into_iter()
        .map(|offset| {
            let (line, character) = position(text, offset);
            (u64::from(line), u64::from(character))
        })
        .collect();
    assert_eq!(session.ranges("dm.expression.undeclared_property"), expected, "{:#?}", session.diagnostics);
    session.shutdown();
}

/// The same rule with a document schema declaring another property: only
/// `dm.expression.undeclared_property` is compared, since strict call
/// warnings on the same reads are a separate diagnostic.
#[test]
fn a_direct_nullable_argument_is_handled_under_a_document_schema() {
    let text = concat!(
        "---\n$schema:\n  other: string\nother: o\n---\n\n",
        "A {{ trim(x) }} {{ lower(y) }} {{ is_null(z) }}\n",
        "B {{ is_empty(trim(w)) }}\n",
    );
    let workspace = LspWorkspace::new();
    let session = Session::open(&workspace, text);
    let (line, character) = position(text, offset_of(text, "B {{", "trim(w", 0) + "trim(".len());
    assert_eq!(
        session.ranges("dm.expression.undeclared_property"),
        [(u64::from(line), u64::from(character))],
        "{:#?}",
        session.diagnostics
    );
    session.shutdown();
}
