//! Composition reports an undeclared member (`dm.expression.undeclared_member`)
//! exactly where the passive check found one and the evaluator reached it,
//! once per authored site.
//!
//! Every test composes a real document in its own temporary directory through
//! the public compose API, and starts from a control spelled `name` that
//! composes with no member warning, so each edit is load-bearing.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeFailure, ComposeOptions, ComposeReport, ComposeWarning};
use tempfile::TempDir;

const CODE: &str = "dm.expression.undeclared_member";

const SERVER: &str = "$schema:\n  server: \"{ name: string, port: number }\"\nserver: { name: web, port: 80 }\n";

/// Composes `content` as `doc.md` in `dir`.
fn compose_in(dir: &Path, content: &str) -> (Markdown, ComposeReport) {
    try_compose_in(dir, content, |options| options).unwrap_or_else(|failure| panic!("{content:?} failed: {}", failure.error))
}

/// The member warnings of composing `content`, as `(line, column, message)`
/// of their shared diagnostic.
#[track_caller]
fn member_warnings(content: &str) -> Vec<(Option<usize>, Option<usize>, String)> {
    let dir = TempDir::new().unwrap();
    let (_, report) = compose_in(dir.path(), content);
    members(&report.warnings)
}

fn members(warnings: &[ComposeWarning]) -> Vec<(Option<usize>, Option<usize>, String)> {
    warnings
        .iter()
        .filter(|warning| warning.code.as_deref() == Some(CODE))
        .map(|warning| {
            let diagnostic = warning.diagnostic.as_ref().expect("a member warning carries its diagnostic");
            let source = diagnostic.source.as_ref().expect("located");
            (source.line, source.column, diagnostic.message.clone())
        })
        .collect()
}

fn message(member: &str, subject: &str) -> String {
    format!("Member '{member}' is not declared by the known object type of '{subject}'.")
}

/// `body` after the `SERVER` frontmatter plus `extra` frontmatter.
fn document(extra: &str, body: &str) -> String {
    format!("---\n{SERVER}{extra}---\n{body}")
}

#[test]
fn body_interpolation_reports_a_reached_member() {
    assert!(member_warnings(&document("", "{{ server.name }}\n")).is_empty());
    assert_eq!(
        member_warnings(&document("", "x {{ server.nmae }}\n")),
        [(Some(6), Some(13), message("nmae", "server"))]
    );
}

#[test]
fn an_unchosen_branch_reports_nothing() {
    assert!(member_warnings(&document("", "{{ server.port > 0 ? server.name : server.nmae }}\n")).is_empty());
    assert_eq!(
        member_warnings(&document("", "{{ server.port > 0 ? server.nmae : server.name }}\n")).len(),
        1
    );
}

#[test]
fn frontmatter_interpolation_reports_a_reached_member() {
    assert!(member_warnings(&document("title: \"{{ server.name }}\"\n", "body\n")).is_empty());
    assert_eq!(
        member_warnings(&document("title: \"{{ server.nmae }}\"\n", "body\n")),
        [(Some(5), Some(19), message("nmae", "server"))]
    );
    assert_eq!(
        member_warnings(&document("title: \"x {{ server.nmae }}\"\n", "body\n")),
        [(Some(5), Some(21), message("nmae", "server"))]
    );
}

#[test]
fn a_block_condition_reports_a_reached_member() {
    assert!(member_warnings(&document("", "::block when=\"server.name\"\nin\n::end-block\n")).is_empty());
    assert_eq!(
        member_warnings(&document("", "::block when=\"server.nmae\"\nin\n::end-block\n")),
        [(Some(6), Some(22), message("nmae", "server"))]
    );
}

#[test]
fn a_transclusion_condition_reports_a_reached_member() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("child.md"), "child\n").unwrap();
    let (_, report) = compose_in(dir.path(), &document("", "::file ./child.md when=\"server.name\"\n"));
    assert!(members(&report.warnings).is_empty());
    let (_, report) = compose_in(dir.path(), &document("", "::file ./child.md when=\"server.nmae\"\n"));
    assert_eq!(members(&report.warnings), [(Some(6), Some(32), message("nmae", "server"))]);
}

const HOSTS: &str = "hosts: \"{ name: string }[]\"\n";

/// A document whose schema also types `hosts`, with `count` items.
fn hosts_document(count: usize, body: &str) -> String {
    let items: Vec<String> = (0..count).map(|index| format!("{{ name: h{index} }}")).collect();
    format!(
        "---\n$schema:\n  server: \"{{ name: string, port: number }}\"\n  {HOSTS}server: {{ name: web, port: 80 }}\nhosts: [{}]\n---\n{body}",
        items.join(", ")
    )
}

#[test]
fn collection_gates_and_callbacks_report_once_per_site() {
    let control = "::map hosts \"h => h.name\"\n{{ h }}\n::end-map\n";
    assert!(member_warnings(&hosts_document(3, control)).is_empty());
    let mapped = "::map hosts \"h => h.nmae\"\n{{ h }}\n::end-map\n";
    assert_eq!(member_warnings(&hosts_document(3, mapped)), [(Some(8), Some(21), message("nmae", "h"))]);
    assert!(member_warnings(&hosts_document(0, mapped)).is_empty(), "an empty source reaches no callback");
    let filtered = "::filter hosts \"h => h.nmae\"\n{{ h.name }}\n::end-filter\n";
    assert_eq!(member_warnings(&hosts_document(2, filtered)), [(Some(8), Some(24), message("nmae", "h"))]);
    let gated = "::map hosts when=\"server.nmae\"\n{{ i.name }}\n::end-map\n";
    assert_eq!(member_warnings(&hosts_document(2, gated)), [(Some(8), Some(26), message("nmae", "server"))]);
}

#[test]
fn a_region_body_reports_once_across_its_copies() {
    let body = "::map hosts\n{{ i.nmae }}\n::end-map\n";
    assert_eq!(member_warnings(&hosts_document(3, body)), [(Some(9), Some(6), message("nmae", "i"))]);
    assert!(member_warnings(&hosts_document(0, body)).is_empty());
}

/// Approves every shell command.
struct AllowAll;

impl darkmatter::markdown::compose::shell_expansion::types::ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: darkmatter::markdown::compose::shell_expansion::types::ShellApprovalRequest,
    ) -> Result<
        darkmatter::markdown::compose::shell_expansion::types::ShellApprovalDecision,
        darkmatter::markdown::compose::ShellExpansionError,
    > {
        Ok(darkmatter::markdown::compose::shell_expansion::types::ShellApprovalDecision::AllowOnce)
    }
}

/// The member warnings of composing `content` with every shell command
/// approved.
fn shell_member_warnings(content: &str) -> Vec<(Option<usize>, Option<usize>, String)> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, content).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .with_source_file(path.clone())
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(dir.path())
        .with_shell_approval_handler(std::sync::Arc::new(AllowAll));
    let (_, report) = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request_at(dir.path(), options))
        .unwrap_or_else(|error| panic!("{content:?} failed: {error}"));
    members(&report.warnings)
}

#[test]
fn a_frontmatter_shell_ternary_reports_its_condition_and_selected_value_branch() {
    let shell = |value: &str| shell_member_warnings(&document(&format!("pick: \"{value}\"\n"), "{{ pick }}\n"));
    assert!(shell("$( server.name ? echo yes : '' )").is_empty());
    assert_eq!(shell("$( server.nmae ? echo yes : '' )"), [(Some(5), Some(18), message("nmae", "server"))]);
    assert_eq!(
        shell("$( server.name ? fm.server.nmae : echo no )"),
        [(Some(5), Some(35), message("nmae", "fm.server"))]
    );
    assert!(shell("$( server.name ? echo no : fm.server.nmae )").is_empty(), "the unselected branch is not read");
}

#[test]
fn a_placeholder_in_a_frontmatter_shell_value_is_a_frontmatter_interpolation_site() {
    let shell = |value: &str| shell_member_warnings(&document(&format!("pick: \"{value}\"\n"), "{{ pick }}\n"));
    assert!(shell("$( echo {{ server.name }} )").is_empty());
    assert_eq!(shell("$( echo {{ server.nmae }} )"), [(Some(5), Some(26), message("nmae", "server"))]);
}

type Located = (Option<usize>, Option<usize>, String);

/// Composes `content` as `doc.md` in `dir` with `adjust` applied to the
/// options, keeping the warnings reported before a failure.
fn try_compose_in(
    dir: &Path,
    content: &str,
    adjust: impl FnOnce(ComposeOptions) -> ComposeOptions,
) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    let path = dir.join("doc.md");
    std::fs::write(&path, content).unwrap();
    let options = adjust(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.clone()),
    );
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with_warnings(&crate::request_support::request_at(dir, options))
}

/// The one-based `(line, column)`, in characters, of byte `offset` in
/// `content`: an oracle computed from the authored text, independent of how
/// composition rewrites it.
fn position(content: &str, offset: usize) -> (Option<usize>, Option<usize>) {
    let preceding = &content[..offset];
    let line_start = preceding.rfind('\n').map_or(0, |at| at + 1);
    (Some(preceding.matches('\n').count() + 1), Some(preceding[line_start..].chars().count() + 1))
}

/// The position right after the first occurrence of `before` in `content`.
#[track_caller]
fn after(content: &str, before: &str) -> (Option<usize>, Option<usize>) {
    let at = content.find(before).unwrap_or_else(|| panic!("{before:?} not in {content:?}"));
    position(content, at + before.len())
}

/// One expected member warning: `member` of `subject`, located right after
/// the first `before` in `content`.
#[track_caller]
fn expect(content: &str, before: &str, member: &str, subject: &str) -> Located {
    let (line, column) = after(content, before);
    (line, column, message(member, subject))
}

/// [`expect`] after the last occurrence of `before`.
#[track_caller]
fn expect_last(content: &str, before: &str, member: &str, subject: &str) -> Located {
    let at = content.rfind(before).unwrap_or_else(|| panic!("{before:?} not in {content:?}"));
    let (line, column) = position(content, at + before.len());
    (line, column, message(member, subject))
}

/// Writes `files` (relative path, text) under `dir`.
fn write_files(dir: &Path, files: &[(&str, &str)]) {
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
}

const HOST_ROWS: &str = "- { name: a, port: 1 }\n- { name: b, port: 2 }\n- { name: c, port: 3 }\n";

#[test]
fn ac1_rows_and_empty() {
    let typo = "---\ntitle: t\n---\n{{ map(load_data(\"hosts.yaml\", \"{ name: string, port: number }[]\"), h => h.nmae) }}\n";
    let control = typo.replace("h.nmae", "h.name");

    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("hosts.yaml", HOST_ROWS)]);
    let (_, report) = compose_in(dir.path(), &control);
    assert!(members(&report.warnings).is_empty(), "control: {:?}", report.warnings);
    let (_, report) = compose_in(dir.path(), typo);
    assert_eq!(members(&report.warnings), [expect(typo, "h => h.", "nmae", "h")], "three rows, one site");

    let empty = TempDir::new().unwrap();
    write_files(empty.path(), &[("hosts.yaml", "[]\n")]);
    let (_, report) = compose_in(empty.path(), typo);
    assert!(members(&report.warnings).is_empty(), "an empty input reaches no callback");

    // A contract read from a variable is not a literal: nothing is typed, so
    // nothing is reported and no member name is inferred.
    let computed = "---\ncontract: \"{ name: string, port: number }[]\"\n---\n{{ map(load_data(\"hosts.yaml\", contract), h => h.nmae) }}\n";
    let (_, report) = compose_in(dir.path(), computed);
    assert!(members(&report.warnings).is_empty(), "{:?}", report.warnings);
    assert!(
        !report.warnings.iter().any(|warning| warning.message.contains("name")),
        "no inferred member names: {:?}",
        report.warnings
    );
}

#[test]
fn ac2_forms() {
    for (expression, before, subject) in [
        ("server.nmae", "{{ server.", "server"),
        ("(server).nmae", "{{ (server).", "(server)"),
        ("fm.server.nmae", "{{ fm.server.", "fm.server"),
        ("server[\"nmae\"]", "{{ server[", "server"),
        ("{ name: \"x\" }.nmae", "{{ { name: \"x\" }.", "{ name: \"x\" }"),
    ] {
        let content = document("", &format!("{{{{ {expression} }}}}\n"));
        assert_eq!(member_warnings(&content), [expect(&content, before, "nmae", subject)], "{expression}");
        let control = content.replace("nmae", "name");
        assert!(member_warnings(&control).is_empty(), "{control}");
    }

    // A typed function return: `load_data` with an object contract.
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("server.yaml", "name: web\n")]);
    let typed = "{{ load_data(\"server.yaml\", \"{ name: string }\").nmae }}\n";
    let (_, report) = compose_in(dir.path(), typed);
    assert_eq!(
        members(&report.warnings),
        [expect(typed, "}\").", "nmae", "load_data(\"server.yaml\", \"{ name: string }\")")]
    );
    let (_, report) = compose_in(dir.path(), &typed.replace("nmae", "name"));
    assert!(members(&report.warnings).is_empty());

    // A declared optional member is silent even when it is unset.
    let unset = "---\n$schema:\n  server: \"{ name: string, port: number }\"\nserver: { port: 80 }\n---\n[{{ server.name }}]\n";
    assert!(member_warnings(unset).is_empty());
}

/// A document whose schema has a pending (pure literal) enum slot, with a
/// frontmatter site reading an undeclared member of `server`.
const PENDING: &str = "---\n$schema:\n  server: \"{ name: string, port: number }\"\n  env: \"enum({{ [\\\"web\\\", \\\"db\\\"] }})\"\nserver: { name: web, port: 80 }\nenv: web\ntitle: \"{{ server.nmae }}\"\n---\nbody\n";

#[test]
fn ac3_pending_template() {
    use darkmatter::markdown::compose::DataRuntime;
    use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
    use darkmatter::markdown::schemas::slots::{SessionMode, SlotSession};

    let expected = [expect(PENDING, "{{ server.", "nmae", "server")];

    let dir = TempDir::new().unwrap();
    let resolving = std::sync::Arc::new(SlotSession::new(DataRuntime::new()));
    let (_, report) = try_compose_in(dir.path(), PENDING, |options| options.with_slot_session(resolving))
        .unwrap_or_else(|failure| panic!("resolves and composes: {}", failure.error));
    assert_eq!(members(&report.warnings), expected, "after resolution");

    let passive = std::sync::Arc::new(SlotSession::new(DataRuntime::new()).with_mode(SessionMode::Passive));
    let failure = try_compose_in(dir.path(), PENDING, |options| options.with_slot_session(passive))
        .expect_err("a passive session leaves the template pending");
    let codes: Vec<String> = error_diagnostics(&failure.error).into_iter().map(|diagnostic| diagnostic.code).collect();
    assert_eq!(codes, ["dm.schema.slot_pending"], "{}", failure.error);
    // The caller-file projection needs the effective schema before the call
    // check or any frontmatter expression runs, so a template left pending
    // stops the document before the site is reached: composition reports
    // only selectors it evaluated, and reports this one after resolution.
    assert!(members(&failure.warnings).is_empty(), "{:?}", failure.warnings);
}

#[test]
fn ac5_paths() {
    let nested_schema = "$schema:\n  server: \"{ name: string, inner: { name: string } }\"\nserver: { name: web, inner: { name: x } }\n";
    let nested = format!("---\n{nested_schema}---\n{{{{ server.inner.nmae }}}}\n");
    assert_eq!(member_warnings(&nested), [expect(&nested, "inner.", "nmae", "server.inner")]);
    assert!(member_warnings(&nested.replace("nmae", "name")).is_empty());

    // A bracket key is one key, dot included.
    let dotted = document("", "{{ server[\"a.b\"] }}\n");
    assert_eq!(member_warnings(&dotted), [expect(&dotted, "{{ server[", "a.b", "server")]);

    // A computed key is not checked; its undeclared root is reported by its
    // own family, not as a member.
    let dir = TempDir::new().unwrap();
    let computed = document("", "[{{ server[key] }}]\n");
    let (_, report) = compose_in(dir.path(), &computed);
    assert!(members(&report.warnings).is_empty(), "{:?}", report.warnings);
    assert!(
        report.warnings.iter().any(|warning| {
            warning.code.as_deref() == Some("dm.expression.undeclared_property") && warning.message.contains("`key`")
        }),
        "the root warning: {:?}",
        report.warnings
    );

    let element = hosts_document(2, "{{ hosts[0].nmae }}\n");
    assert_eq!(member_warnings(&element), [expect(&element, "hosts[0].", "nmae", "hosts[0]")]);
    assert!(member_warnings(&element.replace("nmae", "name")).is_empty());

    // Only the first missing step reports.
    let cascade = document("", "{{ server.nmae.deeper }}\n");
    assert_eq!(member_warnings(&cascade), [expect(&cascade, "{{ server.", "nmae", "server")]);
}

#[test]
fn ac6_identity() {
    // Two authored callbacks over three items: one warning each.
    let two = hosts_document(3, "{{ map(hosts, h => h.nmae) }}\n{{ map(hosts, h => h.nmae) }}\n");
    assert_eq!(
        member_warnings(&two),
        [expect(&two, "h => h.", "nmae", "h"), expect_last(&two, "h => h.", "nmae", "h")]
    );

    // `local.h` is the callback parameter.
    let local = hosts_document(3, "{{ map(hosts, h => local.h.nmae) }}\n");
    assert_eq!(member_warnings(&local), [expect(&local, "local.h.", "nmae", "local.h")]);
    assert!(member_warnings(&local.replace("nmae", "name")).is_empty());

    // A parameter named like a frontmatter key shadows it: `server.port` is
    // declared by the document's `server` but not by a host item.
    let shadowed = hosts_document(3, "{{ map(hosts, server => server.port) }}\n");
    assert_eq!(member_warnings(&shadowed), [expect(&shadowed, "server => server.", "port", "server")]);
    // ...while `fm.server` still reaches the document's value.
    assert!(member_warnings(&hosts_document(3, "{{ map(hosts, server => fm.server.port) }}\n")).is_empty());

    // The same child included twice is one site; two children are two, each
    // attributed to its own document.
    let child = format!("---\n{SERVER}---\n{{{{ server.nmae }}}}\n");
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("a.md", &child), ("b.md", &child)]);
    let (_, report) = compose_in(dir.path(), "::file ./a.md\n\n::file ./a.md\n");
    assert_eq!(members(&report.warnings), [expect(&child, "{{ server.", "nmae", "server")], "one child, twice");
    let (_, report) = compose_in(dir.path(), "::file ./a.md\n\n::file ./b.md\n");
    assert_eq!(members(&report.warnings).len(), 2, "{:?}", report.warnings);
    let files: Vec<_> = report
        .warnings
        .iter()
        .filter(|warning| warning.code.as_deref() == Some(CODE))
        .map(|warning| {
            let file = warning.diagnostic.as_ref().and_then(|diagnostic| diagnostic.source.as_ref()?.file.clone());
            file.map(|file| file.file_name().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    assert_eq!(files, [Some("a.md".to_string()), Some("b.md".to_string())]);
}

#[test]
fn ac7_surfaces_frontmatter_interpolation() {
    let reached = document("title: \"{{ server.port > 0 ? server.nmae : server.name }}\"\n", "body\n");
    assert_eq!(member_warnings(&reached), [expect(&reached, "? server.", "nmae", "server")]);
    let unreached = document("title: \"{{ server.port > 0 ? server.name : server.nmae }}\"\n", "body\n");
    assert!(member_warnings(&unreached).is_empty());
}

#[test]
fn ac7_surfaces_body_interpolation_and_short_circuits() {
    let reached = document("", "{{ server.port > 0 && server.nmae == 'x' }}\n");
    assert_eq!(member_warnings(&reached), [expect(&reached, "&& server.", "nmae", "server")]);
    for unreached in ["{{ server.name || server.nmae == 'x' }}\n", "{{ server.port < 0 && server.nmae == 'x' }}\n"] {
        assert!(member_warnings(&document("", unreached)).is_empty(), "{unreached}");
    }
}

#[test]
fn ac7_surfaces_block_condition() {
    let reached = document("", "::block when=\"server.port > 0 && server.nmae == 'x'\"\nin\n::end-block\n");
    assert_eq!(member_warnings(&reached), [expect(&reached, "&& server.", "nmae", "server")]);
    let unreached = document("", "::block when=\"server.name || server.nmae == 'x'\"\nin\n::end-block\n");
    assert!(member_warnings(&unreached).is_empty());
}

#[test]
fn ac7_surfaces_transclusion_condition() {
    let dir = TempDir::new().unwrap();
    write_files(dir.path(), &[("child.md", "child\n")]);
    let reached = document("", "::file ./child.md when=\"server.port > 0 && server.nmae == 'x'\"\n");
    let (_, report) = compose_in(dir.path(), &reached);
    assert_eq!(members(&report.warnings), [expect(&reached, "&& server.", "nmae", "server")]);
    let (_, report) = compose_in(dir.path(), &document("", "::file ./child.md when=\"server.name || server.nmae == 'x'\"\n"));
    assert!(members(&report.warnings).is_empty());
}

#[test]
fn ac7_surfaces_collections() {
    let gate = "::map hosts when=\"server.name || server.nmae == 'x'\"\n{{ i.name }}\n::end-map\n";
    assert!(member_warnings(&hosts_document(2, gate)).is_empty(), "short-circuited gate operand");
    let gated = hosts_document(2, "::map hosts when=\"server.port > 0 && server.nmae == 'x'\"\n{{ i.name }}\n::end-map\n");
    assert_eq!(member_warnings(&gated), [expect(&gated, "&& server.", "nmae", "server")]);

    let filtered = "::filter hosts \"h => h.nmae\"\n{{ h.name }}\n::end-filter\n";
    assert!(member_warnings(&hosts_document(0, filtered)).is_empty(), "an empty source runs no callback");
    let filtered = hosts_document(3, filtered);
    assert_eq!(member_warnings(&filtered), [expect(&filtered, "h => h.", "nmae", "h")]);

    let unchosen = hosts_document(3, "::map hosts \"h => h.name ? h.name : h.nmae\"\n{{ h }}\n::end-map\n");
    assert!(member_warnings(&unchosen).is_empty(), "an unchosen callback branch");
}

#[test]
fn ac7_surfaces_region_body() {
    let body = "::map hosts\n{{ i.name ? i.name : i.nmae }}\n::end-map\n";
    assert!(member_warnings(&hosts_document(3, body)).is_empty(), "an unchosen branch in every copy");
    let body = hosts_document(3, "::map hosts\n{{ i.name ? i.nmae : i.name }}\n::end-map\n");
    assert_eq!(member_warnings(&body), [expect(&body, "? i.", "nmae", "i")]);
}

#[test]
fn ac7_surfaces_shell_condition_and_branch() {
    let shell = |value: &str| {
        let content = document(&format!("pick: \"{value}\"\n"), "{{ pick }}\n");
        (shell_member_warnings(&content), content)
    };
    let (found, content) = shell("$( server.port > 0 && server.nmae == 'x' ? echo yes : '' )");
    assert_eq!(found, [expect(&content, "&& server.", "nmae", "server")]);
    let (found, _) = shell("$( server.name || server.nmae == 'x' ? echo yes : '' )");
    assert!(found.is_empty(), "short-circuited condition operand");
    let (found, content) = shell("$( server.name ? echo {{ server.nmae }} : echo no )");
    assert_eq!(found, [expect(&content, "{{ server.", "nmae", "server")], "a hole in the selected branch");
    // A hole is a frontmatter interpolation site, interpolated before the
    // command is chosen, so the unselected branch's hole is reached too.
    let (found, content) = shell("$( server.name ? echo no : echo {{ server.nmae }} )");
    assert_eq!(found, [expect(&content, "{{ server.", "nmae", "server")], "a hole in the unselected branch");
}

#[test]
fn ac7_surfaces_authored_positions() {
    // A removed block and a longer insertion before the site move the
    // composed text; the warning stays at the authored line and column.
    let edited = document(
        "",
        "::block when=\"false\"\none\ntwo\nthree\n::end-block\n{{ \"a much longer inserted text\" }} {{ server.nmae }}\n",
    );
    let (line, column) = after(&edited, "}} {{ server.");
    assert_eq!(member_warnings(&edited), [(line, column, message("nmae", "server"))]);
    assert_eq!((line, column), (Some(11), Some(47)), "the oracle itself");

    // A block scalar spanning several lines, the member on a later one.
    let multiline = document("title: |\n  first line\n  second {{ server.nmae }}\n", "body\n");
    assert_eq!(member_warnings(&multiline), [(Some(7), Some(20), message("nmae", "server"))]);

    // Columns count characters: `é` is two bytes and one column.
    assert_eq!(
        member_warnings(&document("", "é {{ server.nmae }}\n")),
        [(Some(6), Some(13), message("nmae", "server"))]
    );
    assert_eq!(
        member_warnings(&document("title: \"é {{ server.nmae }}\"\n", "body\n")),
        [(Some(5), Some(21), message("nmae", "server"))]
    );
}

#[test]
fn ac7_try() {
    let content = "{{ map(try(load_data(\"rows.yaml\", \"{ a: string }[]\"), []), h => h.nmae) }}\n";
    let dir = TempDir::new().unwrap();
    let (_, report) = compose_in(dir.path(), content);
    assert!(members(&report.warnings).is_empty(), "the fallback `[]` reaches no callback");
    write_files(dir.path(), &[("rows.yaml", "- a: x\n- a: y\n")]);
    let (_, report) = compose_in(dir.path(), content);
    assert_eq!(members(&report.warnings), [expect(content, "h => h.", "nmae", "h")]);
}

/// A member warning reported by an earlier stage (frontmatter
/// interpolation, a `::block` condition) survives a later body expression
/// whose read fails the composition.
#[test]
fn ac8_later_failure() {
    let failing = "{{ load_data(\"absent.yaml\", \"string[]\") }}\n";
    for (content, before) in [
        (document("title: \"{{ server.nmae }}\"\n", failing), "{{ server."),
        (document("", &format!("::block when=\"server.nmae == 'x'\"\nin\n::end-block\n\n{failing}")), "when=\"server."),
    ] {
        let dir = TempDir::new().unwrap();
        let failure = try_compose_in(dir.path(), &content, |options| options).expect_err("the read fails");
        assert!(failure.error.to_string().contains("absent.yaml"), "{}", failure.error);
        assert_eq!(members(&failure.warnings), [expect(&content, before, "nmae", "server")], "{content}");
    }
}

#[test]
fn discovery_pass_reports_none() {
    let content = document("title: \"{{ server.nmae }}\"\n", "{{ server.nmae }}\n");
    let dir = TempDir::new().unwrap();
    let (_, report) = try_compose_in(dir.path(), &content, |options| options)
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(members(&report.warnings).len(), 2, "control: an ordinary composition reports both");
    let (_, report) = try_compose_in(dir.path(), &content, ComposeOptions::for_reference_check)
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    assert!(members(&report.warnings).is_empty(), "{:?}", report.warnings);
}

#[test]
fn pass_repetition() {
    // `title` reads a `$( … )` key, so the first frontmatter pass leaves it
    // for the post-shell pass; the member before that read is reported once.
    let content = document("cmd: \"$( echo hi )\"\ntitle: \"{{ server.nmae }} {{ cmd }}\"\n", "{{ title }}\n");
    assert_eq!(shell_member_warnings(&content), [expect(&content, "{{ server.", "nmae", "server")]);
}

