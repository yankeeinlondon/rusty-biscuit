//! Template-mode execution at the library boundary: the shared filler and its
//! typed refusals, the late-value barrier, direct-target resolution, and the
//! approval handler's shape choice during composition.
//!
//! Argument vectors as a native program receives them are proven through
//! `md compose` in `darkmatter-cli`'s `compose_shell_template`; these tests
//! need no program to run. A template-mode outcome is told from a
//! resolved-byte one by its error: a program missing from `PATH` is a
//! `TemplateTarget` refusal in template mode and `CommandNotFound` as bytes.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::expression::ExpressionError;
use darkmatter::markdown::compose::shell_expansion::store::load_ruleset;
use darkmatter::markdown::compose::shell_expansion::template::{
    ArgPart, CommandTemplate, HoleError, HoleTiming, RejectReason, TemplateShape, TemplateTargetError,
    fill_template, parse_authored, resolve_direct_target, resolve_late_values,
};
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellCommandOrigin, ShellExpansionError,
    ShellRuleEntry,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::schemas::simplified::grammar::parse_type_expr;
use darkmatter::markdown::schemas::{PropertyDef, SchemaShape, SimplifiedSchema};
use biscuit_terminal::errors::SourceContext;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// A program no `PATH` holds.
const MISSING: &str = "dm-template-missing-program";

fn typed(command: &str, declarations: &[(&str, &str)]) -> CommandTemplate {
    let mut shape = SchemaShape::new();
    for (name, ty) in declarations {
        shape
            .properties
            .insert((*name).to_string(), PropertyDef::Single(parse_type_expr(name, ty).unwrap()));
    }
    let mut template = parse_authored(command).unwrap();
    template.project_types(Some(&SimplifiedSchema::Single(shape)), &[]);
    template
}

fn scope(values: Value) -> impl Fn(&str) -> Result<Value, ExpressionError> {
    move |expression: &str| Ok(values.get(expression).cloned().unwrap_or(Value::Null))
}

/// Display, every source, and the Debug form of `error`, joined.
fn full_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut text = format!("{error} {error:?}");
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(&format!(" {cause} {cause:?}"));
        source = cause.source();
    }
    text
}

fn authored_ctx() -> SourceContext {
    SourceContext::new(PathBuf::from("/doc.md"), PathBuf::from("doc.md"), "::shell tool {{ id }}\n")
}

// ── Filling and the run-time check (AC7) ─────────────────────────────────

/// The original value is validated against the declaration, constraints
/// included and nothing coerced; a refusal names the placeholder and the
/// declared type, and no part of its error chain holds the value.
#[test]
fn a_value_its_declaration_refuses_is_rejected_without_showing_it() {
    let template = typed("tool {{ id }}", &[("id", "alphanumeric(min(3))")]);
    let cases = [
        (json!(98765), RejectReason::DeclaredType, "98765"),
        (json!("zq"), RejectReason::DeclaredType, "zq"),
        (json!(["sekrit-array"]), RejectReason::NotScalar, "sekrit-array"),
        (json!({ "sekrit-key": 1 }), RejectReason::NotScalar, "sekrit-key"),
        (json!("sekrit\u{0}nul"), RejectReason::EmbeddedNul, "sekrit"),
    ];
    for (value, expected, marker) in cases {
        let error = fill_template(&template, &scope(json!({ "id": value })), None).unwrap_err();
        match &error {
            HoleError::Rejected {
                expression,
                declared_type,
                reason,
            } => {
                assert_eq!(expression, "id");
                assert_eq!(declared_type, "alphanumeric(min(3))");
                assert_eq!(*reason, expected, "{value}");
            }
            other => panic!("{value}: expected a rejection, got {other:?}"),
        }
        let shell = error.into_shell_error(&authored_ctx(), "tool {{ id }}", &ShellCommandOrigin::Body { line: 1 });
        assert!(matches!(shell, ShellExpansionError::TemplateValueRejected { .. }), "{shell:?}");
        let chain = full_chain(&MarkdownError::ShellExpansion(Box::new(shell)));
        assert!(!chain.contains(marker), "{value} leaked: {chain}");
        assert!(chain.contains("'id'") && chain.contains("alphanumeric(min(3))"), "{chain}");
    }

    let filled = fill_template(&template, &scope(json!({ "id": "abc" })), None).unwrap();
    assert_eq!(filled.segments[0].args, ["abc"]);
}

/// `null` is missing, even under a broad `string` hole; empty, zero, and
/// false are present and convert through the scalar engine.
#[test]
fn null_is_missing_and_other_scalars_convert() {
    let template = parse_authored("tool {{ v }}").unwrap();
    let error = fill_template(&template, &scope(json!({ "v": null })), None).unwrap_err();
    assert!(matches!(&error, HoleError::Missing { expression } if expression == "v"), "{error:?}");
    let shell = error.into_shell_error(&authored_ctx(), "tool {{ v }}", &ShellCommandOrigin::Body { line: 1 });
    assert!(matches!(shell, ShellExpansionError::MissingValue { .. }), "{shell:?}");

    for (value, text) in [(json!(""), ""), (json!(0), "0"), (json!(false), "false"), (json!(2.5), "2.5")] {
        let filled = fill_template(&template, &scope(json!({ "v": value })), None).unwrap();
        assert_eq!(filled.segments[0].args, [text]);
    }
}

/// Every hole of every segment is filled before anything can run, even a
/// segment `&&` would skip, and each placeholder is evaluated exactly once.
#[test]
fn every_segment_is_filled_once_before_the_chain_runs() {
    let template = parse_authored("first {{ a }} && second {{ b }}{{ a }}").unwrap();
    let calls = RefCell::new(Vec::new());
    let evaluate = |expression: &str| -> Result<Value, ExpressionError> {
        calls.borrow_mut().push(expression.to_string());
        Ok(json!({ "a": "x", "b": ["y"] })[expression].clone())
    };
    let error = fill_template(&template, &evaluate, None).unwrap_err();
    assert!(matches!(&error, HoleError::Rejected { expression, .. } if expression == "b"), "{error:?}");
    assert_eq!(*calls.borrow(), ["a", "b"]);

    calls.borrow_mut().clear();
    let evaluate = |expression: &str| -> Result<Value, ExpressionError> {
        calls.borrow_mut().push(expression.to_string());
        Ok(json!(expression))
    };
    let filled = fill_template(&template, &evaluate, None).unwrap();
    assert_eq!(filled.segments[1].args, ["ba"]);
    assert_eq!(*calls.borrow(), ["a", "b", "a"]);
}

// ── The late-value barrier ───────────────────────────────────────────────

fn with_late_first_hole(command: &str) -> CommandTemplate {
    let mut template = parse_authored(command).unwrap();
    if let ArgPart::Hole(placeholder) = &mut template.segments[0].args[0].parts[0] {
        placeholder.timing = HoleTiming::Deferred {
            roots: vec!["outputs".to_string()],
        };
    }
    template
}

/// The barrier validates only late holes, and a fill that receives its
/// values never evaluates them again; other holes are read at the command.
#[test]
fn late_values_are_validated_once_at_the_barrier() {
    let template = with_late_first_hole("tool {{ last(outputs) }} {{ name }}");
    let late = resolve_late_values([("step-2/shell", &template)], &scope(json!({ "last(outputs)": "a b" }))).unwrap();
    assert_eq!(late.site("step-2/shell").and_then(|site| site.get(0)), Some("a b"));
    assert_eq!(late.site("step-2/shell").and_then(|site| site.get(1)), None);

    let at_command = |expression: &str| -> Result<Value, ExpressionError> {
        assert_ne!(expression, "last(outputs)", "a late value is never evaluated twice");
        Ok(json!("now"))
    };
    let filled = fill_template(&template, &at_command, late.site("step-2/shell")).unwrap();
    assert_eq!(filled.segments[0].args, ["a b", "now"]);
}

/// A missing or invalid late value fails at the barrier, naming its site.
#[test]
fn a_missing_late_value_fails_at_the_barrier_naming_its_site() {
    let template = with_late_first_hole("tool {{ last(outputs) }}");
    let error = resolve_late_values([("step-2/shell", &template)], &scope(json!({}))).unwrap_err();
    assert_eq!(error.site, "step-2/shell");
    assert!(matches!(&error.error, HoleError::Missing { expression } if expression == "last(outputs)"));
}

// ── Direct targets ───────────────────────────────────────────────────────

/// A program that is not on the search path is refused, never handed to a
/// shell.
#[test]
fn a_program_missing_from_the_search_path_is_not_found() {
    let dir = TempDir::new().unwrap();
    let error = resolve_direct_target(MISSING, Some(dir.path().as_os_str()), dir.path()).unwrap_err();
    assert_eq!(error, TemplateTargetError::NotFound { program: MISSING.to_string() });
}

#[cfg(unix)]
#[test]
fn a_script_without_a_shebang_is_refused_and_a_shebang_script_resolves() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = TempDir::new().unwrap();
    for (name, content) in [("plain", "echo hi\n"), ("bang", "#!/bin/sh\necho hi\n")] {
        let path = dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let search = Some(dir.path().as_os_str());
    assert_eq!(
        resolve_direct_target("plain", search, dir.path()).unwrap_err(),
        TemplateTargetError::NotNative { path: dir.path().join("plain") }
    );
    assert_eq!(resolve_direct_target("bang", search, dir.path()).unwrap(), dir.path().join("bang"));
}

#[cfg(windows)]
#[test]
fn a_batch_file_is_refused_as_a_direct_target() {
    let dir = TempDir::new().unwrap();
    for extension in ["cmd", "bat"] {
        std::fs::write(dir.path().join(format!("tool.{extension}")), "@echo hi\r\n").unwrap();
        let error = resolve_direct_target(&format!("tool.{extension}"), Some(dir.path().as_os_str()), dir.path())
            .unwrap_err();
        assert!(
            matches!(&error, TemplateTargetError::Interpreter { extension: found, .. } if found == extension),
            "{error:?}"
        );
    }
}

// ── The approval handler during composition ──────────────────────────────

/// Answers every request with one decision and records what it was asked.
struct Scripted {
    decision: ShellApprovalDecision,
    requests: Mutex<Vec<ShellApprovalRequest>>,
}

impl Scripted {
    fn new(decision: ShellApprovalDecision) -> Arc<Self> {
        Arc::new(Self {
            decision,
            requests: Mutex::default(),
        })
    }

    fn requests(&self) -> Vec<ShellApprovalRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl ShellApprovalHandler for Scripted {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.requests.lock().unwrap().push(request);
        Ok(self.decision.clone())
    }
}

fn compose_with(dir: &Path, body: &str, handler: Arc<Scripted>) -> Result<Markdown, MarkdownError> {
    let path = dir.join("doc.md");
    std::fs::write(&path, body).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.clone())
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(handler);
    Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed)
}

fn shell_error(error: &MarkdownError) -> &ShellExpansionError {
    match error {
        MarkdownError::ShellExpansion(inner) => inner,
        other => panic!("expected a shell error, got {other}"),
    }
}

fn is_template_target(error: &MarkdownError) -> bool {
    matches!(
        shell_error(error),
        ShellExpansionError::TemplateTarget { reason, .. } if matches!(**reason, TemplateTargetError::NotFound { .. })
    )
}

fn rules(path: PathBuf) -> Vec<ShellRuleEntry> {
    load_ruleset(&path).unwrap().entries
}

fn shape(command: &str) -> TemplateShape {
    TemplateShape::from_template(&parse_authored(command).unwrap())
}

/// A site whose bytes may run is offered both; choosing the shape persists
/// `template <shape>` and runs the command as a template, while "once" runs
/// its resolved bytes.
#[test]
fn the_shape_choice_persists_the_template_and_runs_in_template_mode() {
    let dir = TempDir::new().unwrap();
    let body = format!("---\nv: a b\n---\n::shell {MISSING} {{{{ v }}}}\n");

    let handler = Scripted::new(ShellApprovalDecision::AllowTemplatePersist);
    let error = compose_with(dir.path(), &body, handler.clone()).unwrap_err();
    assert!(is_template_target(&error), "{error:?}");
    let requests = handler.requests();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].offered.exact && requests[0].offered.shape && requests[0].offered.prefix);
    assert_eq!(requests[0].normalized_exact, format!("{MISSING} a b"));
    assert_eq!(
        requests[0].template.as_ref().map(|view| view.display.as_str()),
        Some(format!("{MISSING} {{{{ v: string }}}}").as_str())
    );
    let template_rule = ShellRuleEntry::Template(shape(&format!("{MISSING} {{{{ v }}}}")));
    assert_eq!(rules(dir.path().join(".darkmatter-shell-whitelist")), vec![template_rule]);

    // The persisted rule now answers without a prompt.
    let handler = Scripted::new(ShellApprovalDecision::Deny);
    let error = compose_with(dir.path(), &body, handler.clone()).unwrap_err();
    assert!(is_template_target(&error), "{error:?}");
    assert!(handler.requests().is_empty());

    let other = TempDir::new().unwrap();
    let handler = Scripted::new(ShellApprovalDecision::AllowOnce);
    let error = compose_with(other.path(), &body, handler).unwrap_err();
    assert!(
        matches!(shell_error(&error), ShellExpansionError::CommandNotFound { .. }),
        "once runs the resolved bytes: {error:?}"
    );
}

/// A site whose value keeps its bytes from parsing is offered only its shape;
/// "once" approves that shape for the session, so a second site of the same
/// shape is not asked again.
#[test]
fn a_template_only_site_offers_no_bytes_and_once_covers_its_shape() {
    let dir = TempDir::new().unwrap();
    let body = "---\nv: \"a;b\"\nw: \"c;d\"\n---\n::shell git --version {{ v }}\n\n::shell git --version {{ w }}\n";
    let handler = Scripted::new(ShellApprovalDecision::AllowOnce);

    let composed = compose_with(dir.path(), body, handler.clone()).unwrap();

    assert_eq!(composed.content().matches("git version").count(), 2, "{}", composed.content());
    let requests = handler.requests();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].offered.exact && requests[0].offered.shape);
    assert_eq!(requests[0].normalized_exact, "");
    assert!(!dir.path().join(".darkmatter-shell-whitelist").exists());
}

/// An unoffered exact choice denies; blacklisting persists the shape.
#[test]
fn a_template_only_site_refuses_an_exact_choice_and_blacklists_its_shape() {
    let body = format!("---\nv: \"a;b\"\n---\n::shell {MISSING} {{{{ v }}}}\n");

    let dir = TempDir::new().unwrap();
    let error = compose_with(dir.path(), &body, Scripted::new(ShellApprovalDecision::AllowExactPersist)).unwrap_err();
    assert!(matches!(shell_error(&error), ShellExpansionError::Denied { .. }), "{error:?}");
    assert!(!dir.path().join(".darkmatter-shell-whitelist").exists());

    let dir = TempDir::new().unwrap();
    let error = compose_with(dir.path(), &body, Scripted::new(ShellApprovalDecision::BlacklistPersist)).unwrap_err();
    assert!(matches!(shell_error(&error), ShellExpansionError::Blacklisted { .. }), "{error:?}");
    assert_eq!(
        rules(dir.path().join(".darkmatter-shell-blacklist")),
        vec![ShellRuleEntry::Template(shape(&format!("{MISSING} {{{{ v }}}}")))]
    );
}

/// An exact rule answers before a template rule, which answers before the
/// prompt.
#[test]
fn an_exact_rule_wins_over_a_template_rule() {
    let body = format!("---\nv: a b\n---\n::shell {MISSING} {{{{ v }}}}\n");
    let template_rule = format!("template {}\n", shape(&format!("{MISSING} {{{{ v }}}}")).canonical_text());

    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join(".darkmatter-shell-whitelist"),
        format!("{template_rule}exact {MISSING} a b\n"),
    )
    .unwrap();
    let error = compose_with(dir.path(), &body, Scripted::new(ShellApprovalDecision::Deny)).unwrap_err();
    assert!(matches!(shell_error(&error), ShellExpansionError::CommandNotFound { .. }), "{error:?}");

    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(".darkmatter-shell-whitelist"), template_rule).unwrap();
    let error = compose_with(dir.path(), &body, Scripted::new(ShellApprovalDecision::Deny)).unwrap_err();
    assert!(is_template_target(&error), "{error:?}");
}

#[test]
fn a_document_late_value_snapshot_survives_changes_before_dispatch() {
    use darkmatter::markdown::compose::shell_expansion::template::{DeferredShellRoots, DocumentShellValues, ShellPreApprovals};
    let directory = TempDir::new().unwrap();
    let document = directory.path().join("prompt.md");
    std::fs::write(&document, format!("::shell {MISSING} {{{{ last(outputs) }}}}\n")).unwrap();
    let markdown = Markdown::try_from(document.as_path()).unwrap();
    let snapshot = DocumentShellValues::default();
    let context = ComposeContext::capture_for_content(directory.path(), "");
    let base = ComposeOptions::new()
        .with_context(context)
        .with_source_file(&document)
        .with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]))
        .with_document_shell_values(snapshot);
    let preflight = markdown.compose_preflight(&crate::request_support::request(base.clone()
        .with_data_overrides(json!({"outputs": ["checked"]}))
        .with_deferred_shell_value_validation(true))).unwrap();
    let mut approved = ShellPreApprovals::new();
    approved.insert_template(preflight.candidates[0].shape().unwrap());
    let error = markdown.compose_with(&crate::request_support::request(base
        .with_data_overrides(json!({"outputs": [null]}))
        .with_pre_approved_commands(approved))).unwrap_err();
    assert!(is_template_target(&error), "the already validated value must be used: {error:?}");
}

#[test]
fn a_document_site_changed_after_the_barrier_cannot_read_a_new_late_value() {
    use darkmatter::markdown::compose::shell_expansion::template::{DeferredShellRoots, DocumentShellValues, ShellPreApprovals};
    let directory = TempDir::new().unwrap();
    let document = directory.path().join("prompt.md");
    std::fs::write(&document, format!("::shell {MISSING} {{{{ last(outputs) }}}}\n")).unwrap();
    let markdown = Markdown::try_from(document.as_path()).unwrap();
    let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(directory.path(), ""))
        .with_source_file(&document)
        .with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]))
        .with_document_shell_values(DocumentShellValues::default())
        .with_data_overrides(json!({"outputs": ["checked"]}));
    let preflight = markdown.compose_preflight(&crate::request_support::request(base.clone()
        .with_deferred_shell_value_validation(true))).unwrap();
    let mut approved = ShellPreApprovals::new();
    approved.insert_template(preflight.candidates[0].shape().unwrap());
    std::fs::write(&document, format!("::shell {MISSING} {{{{ outputs[0] }}}}\n")).unwrap();
    let changed = Markdown::try_from(document.as_path()).unwrap();
    let error = changed.compose_with(&crate::request_support::request(base
        .with_pre_approved_commands(approved))).unwrap_err();
    assert!(matches!(shell_error(&error), ShellExpansionError::NotPreApproved { .. }), "{error:?}");
}


#[test]
fn persisted_policy_matrix_decides_public_composition() {
    let directory = TempDir::new().unwrap();
    let document = directory.path().join("doc.md");
    std::fs::write(&document, format!("---\nv: safe\n---\n::shell {MISSING} {{{{ v }}}}\n")).unwrap();
    let markdown = Markdown::try_from(document.as_path()).unwrap();
    let context = ComposeContext::capture_for_content(directory.path(), "");
    let control = format!("template {MISSING} {{{{ string }}}}");
    let cases = [
        ("control", control.clone(), true),
        ("absent keyword", format!("{MISSING} {{{{ string }}}}"), false),
        ("absent payload", "template".into(), false),
        ("null keyword", format!("null {MISSING} {{{{ string }}}}"), false),
        ("null payload", "template null".into(), false),
        ("wrong keyword type", format!("123 {MISSING} {{{{ string }}}}"), false),
        ("collection payload", "template [123]".into(), false),
        ("wrong hole type", format!("template {MISSING} {{{{ 123 }}}}"), false),
        ("empty keyword", format!(" {MISSING} {{{{ string }}}}"), false),
        ("empty payload", "template ".into(), false),
        ("empty file", "".into(), false),
        ("duplicate", format!("{control}\n{control}"), true),
        ("invalid trailing", format!("{control} \"open"), false),
        ("valid extra argument", format!("{control} extra"), false),
        ("bad second row", format!("{control}\ntemplate {{{{ string }}}} run"), true),
        ("constraint", format!("template {MISSING} {{{{ alphanumeric(min(3)) }}}}"), false),
        ("constraint semicolon", format!("template {MISSING} {{{{ alphanumeric; min(3) }}}}"), false),
        ("malformed constraint", format!("template {MISSING} {{{{ alphanumeric(min() }}}}"), false),
        ("malformed constraint semicolon", format!("template {MISSING} {{{{ alphanumeric; min( }}}}"), false),
        ("program hole", "template {{ string }} safe".into(), false),
        ("literal mismatch", format!("template {MISSING} other"), false),
    ];
    for (name, line, applies) in cases {
        for blacklist in [false, true] {
            let whitelist = directory.path().join(".darkmatter-shell-whitelist");
            let denylist = directory.path().join(".darkmatter-shell-blacklist");
            std::fs::write(&whitelist, if blacklist { &control } else { &line }).unwrap();
            std::fs::write(&denylist, if blacklist { &line } else { "" }).unwrap();
            let handler = Scripted::new(ShellApprovalDecision::Deny);
            let options = ComposeOptions::new_with_context(context.clone())
                .with_source_file(&document)
                .with_shell_policy_root(directory.path())
                .with_shell_working_directory(directory.path())
                .with_shell_approval_handler(handler.clone());
            let error = markdown.compose_with(&crate::request_support::request(options)).unwrap_err();
            if blacklist && applies {
                assert!(matches!(shell_error(&error), ShellExpansionError::Blacklisted { .. }), "{name}: {error:?}");
            } else if blacklist || applies {
                assert!(is_template_target(&error), "{name}: {error:?}");
            } else {
                assert!(matches!(shell_error(&error), ShellExpansionError::Denied { .. }), "{name}: {error:?}");
                assert_eq!(handler.requests().len(), 1, "{name}");
            }
            if blacklist || applies {
                assert!(handler.requests().is_empty(), "{name}: policy must answer without prompting");
            }
        }
    }
}


#[test]
fn once_cannot_approve_invalid_late_template_grammar() {
    use darkmatter::markdown::compose::shell_expansion::template::DeferredShellRoots;
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("doc.md");
    let context = ComposeContext::capture_for_content(directory.path(), "");
    for command in [
        "tool {{ outputs[0] }} | tool second",
        "tool {{ outputs[0] }}; tool second",
        "{{ outputs[0] }} first",
        "tool arg > {{ outputs[0] }}",
        "tool {{ outputs[0] }} >> trace.txt",
        "tool {{ outputs[0] }} < trace.txt",
        "tool {{ outputs[0] }} &",
    ] {
        std::fs::write(&path, format!("::shell {command}\n")).unwrap();
        let markdown = Markdown::try_from(path.as_path()).unwrap();
        let handler = Scripted::new(ShellApprovalDecision::AllowOnce);
        let options = ComposeOptions::new_with_context(context.clone())
            .with_source_file(&path)
            .with_shell_policy_root(directory.path())
            .with_shell_working_directory(directory.path())
            .with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]))
            .with_data_overrides(json!({"outputs": ["safe"]}))
            .with_shell_approval_handler(handler.clone());
        let error = markdown.compose_with(&crate::request_support::request(options)).unwrap_err();
        assert!(!matches!(shell_error(&error), ShellExpansionError::CommandNotFound { .. } | ShellExpansionError::TemplateTarget { .. }), "{command}: {error:?}");
        assert!(handler.requests().is_empty(), "{command}: invalid grammar must be refused before once is offered");
    }
}
