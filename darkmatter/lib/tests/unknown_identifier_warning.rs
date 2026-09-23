//! Requirement 4 of the dasherized-identifiers spec
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`): a well-formed
//! identifier that resolves to nothing warns at
//! `dm.expression.unknown_identifier`, composition still succeeds and the value
//! still renders empty — unless the author handled the absence explicitly or
//! the root is known to the effective state, a caller input layer, or the
//! effective schema.
//!
//! Every test composes a real document through `compose_with` and asserts the
//! rendered output together with the warning set.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{
    CallerInputRecord, CallerInputRecords, ComposeContext, ComposeOptions, ComposeReport,
    ComposeWarning,
};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

/// Approves every `$()` command, so a ternary fixture can run `echo`.
struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    path
}

/// A context capturing only the date/time group, so no test observes the host
/// repository or environment.
fn options(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_shell(ShellExpansionOptions {
            policy_root: Some(dir.to_path_buf()),
            ..Default::default()
        })
        .with_shell_approval_handler(Arc::new(AllowAll))
}

fn compose(path: &Path, options: ComposeOptions) -> (String, ComposeReport) {
    let (composed, report) = Markdown::try_from(path)
        .expect("document loads")
        .compose_with(options)
        .expect("an unknown identifier never fails composition");
    (composed.content().trim_end().to_string(), report)
}

/// Composes `content` as `doc.md` in a fresh directory with default options.
fn compose_doc(content: &str) -> (String, ComposeReport) {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", content);
    compose(&path, options(dir.path(), &path))
}

fn unknown_identifiers(report: &ComposeReport) -> Vec<&ComposeWarning> {
    report
        .warnings
        .iter()
        .filter(|w| w.code.as_deref() == Some(ComposeWarning::UNKNOWN_IDENTIFIER_CODE))
        .collect()
}

/// The roots named by the report's unknown-identifier warnings, in order.
fn warned_roots(report: &ComposeReport) -> Vec<String> {
    unknown_identifiers(report)
        .iter()
        .map(|w| {
            // Messages are Prose markup, so `_` in a root arrives escaped.
            let rest = w.message.strip_prefix("unknown identifier '").expect(&w.message);
            rest[..rest.find('\'').unwrap()].replace('\\', "")
        })
        .collect()
}

// ── Worked examples (spec table) ─────────────────────────────────────────────

#[test]
fn an_undeclared_kebab_root_warns_renders_empty_and_composes() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\ntitle: T\n---\n[{{ iteration-1 }}]\n");

    let (content, report) = compose(&path, options(dir.path(), &path));

    assert_eq!(content, "[]");
    let warnings = unknown_identifiers(&report);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    let warning = warnings[0];
    assert_eq!(warning.source.as_deref(), Some(ComposeWarning::EXPRESSION_SOURCE));
    assert_eq!(warning.path.as_deref(), Some(path.as_path()));
    assert_eq!(warning.line_number, Some(4));
    assert!(warning.message.starts_with("unknown identifier 'iteration-1' at "), "{}", warning.message);
    assert!(warning.message.contains("doc.md:4"), "{}", warning.message);
    assert_eq!(report.warnings.len(), 1, "no other diagnostic: {:?}", report.warnings);
}

#[test]
fn a_handled_absence_is_silent_and_renders_the_fallback() {
    let (content, report) = compose_doc("---\ntitle: T\n---\n[{{ iteration-1 || \"fallback\" }}]\n");

    assert_eq!(content, "[fallback]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn a_declared_optional_root_is_silent_and_renders_empty() {
    let (content, report) =
        compose_doc("---\n$schema:\n  iteration-1: number\n---\n[{{ iteration-1 }}]\n");

    assert_eq!(content, "[]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

/// Schema validation owns an unset required property: the failure is the
/// schema's own, and it fires before any candidate could be reconciled.
#[test]
fn a_declared_required_root_fails_with_the_schema_message_only() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  iteration-1: number(required)\n---\n[{{ iteration-1 }}]\n",
    );

    let error = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(options(dir.path(), &path))
        .expect_err("an unset required property fails schema validation");

    let MarkdownError::SchemaValidationFailed { problems, .. } = &error else {
        panic!("expected the schema's own failure, got {error:?}");
    };
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].property.as_deref(), Some("iteration-1"), "{problems:?}");
    assert!(!error.to_string().contains("unknown identifier"), "{error}");
}

// ── Suppression table (spec) ─────────────────────────────────────────────────

/// Each row: expression, frontmatter, expected render, expected warned roots.
/// `set` is truthy and `blank` is known but falsy, so each row can choose which
/// branch or fallback operand evaluation reaches.
#[test]
fn every_suppression_row_warns_exactly_where_evaluation_reads_an_unhandled_root() {
    let rows: &[(&str, &str, &[&str])] = &[
        ("{{ x }}", "", &["x"]),
        ("{{ x || \"d\" }}", "d", &[]),
        // The right-hand side warns only when the fallback evaluates it.
        ("{{ blank || x }}", "", &["x"]),
        ("{{ set || x }}", "yes", &[]),
        ("{{ x ? \"a\" : \"b\" }}", "b", &[]),
        ("{{ x ? x : \"b\" }}", "b", &[]),
        // A guarded root is silent even in the branch evaluation reaches.
        ("{{ x ? \"a\" : x }}", "", &[]),
        // A different root in a guarded ternary's branch still warns.
        ("{{ x ? \"a\" : y }}", "", &["y"]),
        ("{{ set ? x : \"b\" }}", "", &["x"]),
        ("{{ blank ? x : \"b\" }}", "b", &[]),
        ("{{ is_null(x) }}", "true", &[]),
        ("{{ is_empty(x) }}", "true", &[]),
        // Registered aliases resolve through the same function catalog.
        ("{{ isnull(x) }}", "true", &[]),
        ("{{ isEmpty(x) }}", "true", &[]),
        // Nested: the predicate no longer directly guards the read. The spec
        // writes `is_empty(trim(x))`, but `trim` is not a catalog function.
        ("{{ is_empty(lower(x)) }}", "true", &["x"]),
        ("{{ length(x) }}", "0", &["x"]),
        // A chain that is itself a primary handles every operand.
        ("{{ x || y || \"d\" }}", "d", &[]),
        // "Direct" only: a comparison in the condition is an ordinary operand.
        ("{{ x == \"a\" ? \"1\" : \"2\" }}", "2", &["x"]),
        ("{{ !x ? \"1\" : \"2\" }}", "1", &["x"]),
        // Path access on a handled value is still that value's absence check.
        ("{{ x.deep || \"d\" }}", "d", &[]),
        ("{{ x[0] || \"d\" }}", "d", &[]),
        ("{{ x.deep }}", "", &["x"]),
        // A known root with a missing member is not an unknown root.
        ("{{ set.missing }}", "", &[]),
        // The grammar has no `null` literal; authors use the identifier as one.
        ("{{ set ? \"a\" : null }}", "a", &[]),
        ("{{ blank ? \"a\" : null }}", "", &[]),
    ];

    for (expression, rendered, warned) in rows {
        let (content, report) = compose_doc(&format!(
            "---\nset: \"yes\"\nblank: \"\"\n---\n[{expression}]\n"
        ));
        assert_eq!(content, format!("[{rendered}]"), "render of {expression}");
        assert_eq!(warned_roots(&report), *warned, "warnings for {expression}: {:?}", report.warnings);
    }
}

#[test]
fn a_bare_unknown_page_block_gate_warns_at_its_line() {
    let (content, report) =
        compose_doc("---\ntitle: T\n---\nbefore\n\n::block when=\"gate\"\nhidden\n::end-block\n\nafter\n");

    assert!(!content.contains("hidden"), "{content}");
    let warnings = unknown_identifiers(&report);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    assert!(warnings[0].message.starts_with("unknown identifier 'gate'"), "{}", warnings[0].message);
    assert_eq!(warnings[0].line_number, Some(6));
}

// ── Known roots: missing is not the same as falsy ────────────────────────────

#[test]
fn explicit_null_and_empty_frontmatter_values_are_known() {
    let (content, report) =
        compose_doc("---\nnothing: null\nempty: \"\"\n---\n[{{ nothing }}/{{ empty }}]\n");

    assert_eq!(content, "[/]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn set_overrides_are_known_even_when_null_or_empty() {
    for value in [json!(null), json!(""), json!("v")] {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "doc.md", "---\ntitle: T\n---\n[{{ supplied }}]\n");
        let options = options(dir.path(), &path).with_set_overrides(json!({ "supplied": value }));

        let (_, report) = compose(&path, options);

        assert!(report.warnings.is_empty(), "--set {value}: {:?}", report.warnings);
    }
}

#[test]
fn external_state_is_known() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "[{{ inherited }}]\n");
    let options = options(dir.path(), &path).with_external_state(json!({ "inherited": null }));

    let (content, report) = compose(&path, options);

    assert_eq!(content, "[]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

/// Inherited (external) state and `--set` values are merged into the
/// frontmatter before pass 1, so a frontmatter value reading one is silent
/// from the first pass on.
#[test]
fn a_frontmatter_read_of_inherited_state_is_silent() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---
label: \"x {{ inherited }}\"\n---\n{{ label }}\n");
    let options = options(dir.path(), &path).with_external_state(json!({ "inherited": "P" }));

    let (_, report) = compose(&path, options);

    assert!(unknown_identifiers(&report).is_empty(), "{:?}", report.warnings);
}

/// Ruling R-9 left open whether a caller input record can name a root that
/// never reaches the effective state. A record alone is a caller input layer,
/// so its root is known either way.
#[test]
fn a_caller_input_record_is_known_even_without_an_override() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "[{{ recorded }}]\n");
    let mut records = CallerInputRecords::new();
    records.insert(
        "recorded".to_string(),
        CallerInputRecord::new(json!("x"), biscuit_file::FileResolutionContext::new(dir.path())),
    );

    let (_, report) = compose(&path, options(dir.path(), &path).with_caller_input_records(records));

    assert!(unknown_identifiers(&report).is_empty(), "{:?}", report.warnings);
}

#[test]
fn a_baseline_schema_property_is_known() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\ntitle: T\n---\n[{{ from-baseline }}]\n");
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str("from-baseline: string\n").unwrap();
    let baseline = darkmatter::markdown::schemas::simplified::parse_yaml_schema(&yaml).unwrap();

    let (content, report) = compose(&path, options(dir.path(), &path).with_baseline_schema(baseline));

    assert_eq!(content, "[]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

/// A matched trigger schema's payload is part of the effective schema; the
/// same document without trigger discovery warns, so the trigger is what
/// silences it.
#[test]
fn a_matched_trigger_schema_property_is_known() {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    write(
        repo.path(),
        "schemas/stable.trigger.yaml",
        "kind: trigger-schema\nmatch:\n  gate: literal(stable; required)\n$schema: payload.yaml\n",
    );
    write(repo.path(), "schemas/payload.yaml", "$schema:\n  trigger_marker: string\n");
    let path = write(repo.path(), "prompts/doc.md", "---\ngate: stable\n---\n[{{ trigger_marker }}]\n");

    for (triggers, expected) in [(true, Vec::<String>::new()), (false, vec!["trigger_marker".to_string()])] {
        let context = biscuit_file::FileResolutionContext::new(repo.path())
            .with_repository_root(repo.path())
            .with_source_path(&path);
        let options = options(repo.path(), &path)
            .with_file_resolution_context(context)
            .with_trigger_schemas(triggers);

        let (content, report) = compose(&path, options);

        assert_eq!(content, "[]");
        assert_eq!(warned_roots(&report), expected, "triggers={triggers}: {:?}", report.warnings);
    }
}

/// A transcluded child inherits its parent's state, so a parent key is known
/// there; the child's own unknown root warns against the child document.
#[test]
fn a_transcluded_child_knows_inherited_state_and_warns_for_itself() {
    let dir = TempDir::new().unwrap();
    let child = write(dir.path(), "child.md", "child {{ parent_key }} {{ child_typo }}\n");
    let root = write(dir.path(), "root.md", "---\nparent_key: P\n---\n::file ./child.md\n");

    let (content, report) = compose(&root, options(dir.path(), &root));

    assert!(content.contains("child P"), "{content}");
    let warnings = unknown_identifiers(&report);
    assert_eq!(warned_roots(&report), ["child_typo"], "{:?}", report.warnings);
    // A transcluded child is loaded by its canonical path.
    assert_eq!(warnings[0].path, Some(child.canonicalize().unwrap()));
    assert_eq!(warnings[0].line_number, Some(1));
}

// ── Surfaces: every runtime evaluation site uses the shared policy ───────────

#[test]
fn mixed_and_whole_value_frontmatter_warn_at_their_key() {
    let (content, report) = compose_doc(
        "---\ntitle: T\nlabel: \"x {{ fm_mixed }} y\"\nwhole: \"{{ fm_whole }}\"\n---\n{{ label }}\n",
    );

    assert_eq!(content, "x  y");
    let warnings = unknown_identifiers(&report);
    assert_eq!(warned_roots(&report), ["fm_mixed", "fm_whole"], "{:?}", report.warnings);
    assert_eq!(warnings[0].line_number, Some(3));
    assert!(warnings[0].message.contains("(frontmatter key 'label')"), "{}", warnings[0].message);
    assert_eq!(warnings[1].line_number, Some(4));
    assert!(warnings[1].message.contains("(frontmatter key 'whole')"), "{}", warnings[1].message);
}

/// Pass 1 runs before schema validation, which materializes a declared
/// optional document property as `null`, and before a baseline is consulted.
/// Its candidates are reconciled afterwards, so neither root warns.
#[test]
fn pass_one_candidates_known_by_the_final_state_or_schema_are_dropped() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  declared: string\nlabel: \"{{ declared }}{{ baseline_only }}\"\n---\n[{{ label }}]\n",
    );
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str("baseline_only: string\n").unwrap();
    let baseline = darkmatter::markdown::schemas::simplified::parse_yaml_schema(&yaml).unwrap();

    let (content, report) = compose(&path, options(dir.path(), &path).with_baseline_schema(baseline));

    assert_eq!(content, "[]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn a_transclusion_gate_warns_at_its_authored_line() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "CHILD\n");
    // The page block above the directive removes three lines before the
    // transclusion stage parses the body; the warning still names line 10.
    let path = write(
        dir.path(),
        "doc.md",
        "---\ntitle: T\n---\nintro\n\n::block when=\"false\"\nhidden\n::end-block\n\n::file ./child.md when=\"include_child\"\n",
    );

    let (content, report) = compose(&path, options(dir.path(), &path));

    assert!(!content.contains("CHILD"), "{content}");
    let warnings = unknown_identifiers(&report);
    assert_eq!(warned_roots(&report), ["include_child"], "{:?}", report.warnings);
    assert_eq!(warnings[0].line_number, Some(10));
}

#[test]
fn a_shell_ternary_warns_for_its_gate_and_its_selected_branch_only() {
    let (content, report) = compose_doc(
        "---\ngated: \"$(shell_gate ? echo a : echo b)\"\npicked: \"$(true ? chosen_branch : echo b)\"\nunpicked: \"$(false ? skipped_branch : echo c)\"\n---\n{{ gated }}/{{ picked }}/{{ unpicked }}\n",
    );

    assert_eq!(content, "b//c", "the gate is falsy and the chosen value branch is null");
    assert_eq!(warned_roots(&report), ["shell_gate", "chosen_branch"], "{:?}", report.warnings);
    for warning in unknown_identifiers(&report) {
        assert_eq!(warning.stage, "frontmatter-shell-ternary");
    }
}

// ── One warning per root per source document (Requirement 5) ─────────────────

#[test]
fn one_root_read_by_every_surface_warns_once_at_its_first_read() {
    let (_, report) = compose_doc(
        "---\nlabel: \"{{ everywhere }}\"\n---\n::block when=\"everywhere\"\nx\n::end-block\n{{ everywhere }}\n{{ everywhere }}\n",
    );

    let warnings = unknown_identifiers(&report);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    assert!(warnings[0].message.contains("(frontmatter key 'label')"), "{}", warnings[0].message);
}

#[test]
fn ten_body_reads_warn_once_at_the_first_line() {
    let body: String = (0..10).map(|_| "- {{ repeated }}\n").collect();
    let (content, report) = compose_doc(&format!("---\ntitle: T\n---\n{body}"));

    assert_eq!(content.lines().count(), 10, "every reference still renders: {content}");
    let warnings = unknown_identifiers(&report);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    // Ten identical spans cannot be told apart by text, but the first span is
    // the untouched body prefix, so its line is provable.
    assert_eq!(warnings[0].line_number, Some(4));
}

#[test]
fn one_root_in_two_transcluded_documents_warns_once_per_document() {
    let dir = TempDir::new().unwrap();
    let a = write(dir.path(), "a.md", "a {{ shared_typo }}\n");
    let b = write(dir.path(), "b.md", "b {{ shared_typo }}\n");
    let root = write(dir.path(), "root.md", "::file ./a.md\n\n::file ./b.md\n\n::file ./a.md\n");

    let (_, report) = compose(&root, options(dir.path(), &root));

    let paths: Vec<_> = unknown_identifiers(&report).iter().map(|w| w.path.clone()).collect();
    let expected = [Some(a.canonicalize().unwrap()), Some(b.canonicalize().unwrap())];
    assert_eq!(paths, expected, "{:?}", report.warnings);
}

/// Pre-approval runs a condition-blind discovery compose (no page blocks)
/// first. A root read only inside a false block is never reached by the real
/// run, so discovery's reads must never become warnings.
#[test]
fn a_root_read_only_inside_a_false_block_never_warns_even_with_preflight_discovery() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\nstamp: \"$(echo ok)\"\n---\n{{ stamp }}\n\n::block when=\"false\"\n{{ ghost }}\n::end-block\n",
    );
    let options = options(dir.path(), &path)
        .with_pre_approved_commands(std::collections::HashSet::from(["echo ok".to_string()]));

    let (content, report) = compose(&path, options);

    assert_eq!(content, "ok");
    assert!(unknown_identifiers(&report).is_empty(), "{:?}", report.warnings);
}
