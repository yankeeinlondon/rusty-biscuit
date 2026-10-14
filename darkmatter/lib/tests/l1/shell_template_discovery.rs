//! Discovery provenance and approval transport for typed shell templates:
//! the missing-value refusal at preflight, one candidate per command site
//! with its owner's types, host-declared late roots, structured
//! pre-approvals and their membership proof, and the `md` approval
//! lifecycle's per-site choices (acceptance criteria 1, 12, 15, and 16 of
//! `2026-10-23-shell-template-approval`, at the library boundary).

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::store::{append_whitelist_template, load_ruleset};
use darkmatter::markdown::compose::shell_expansion::template::{
    ApprovalType, DeferredShellRoots, HoleTiming, ModeRequirement, ShellCandidate, ShellPreApprovals,
    TemplateShape,
};
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellCommandOrigin, ShellExpansionError,
    ShellPolicyPaths, ShellRuleEntry,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOperation, ComposeOptions};
use darkmatter::markdown::MarkdownError;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// Options for composing `path`, anchored in its own temp dir.
fn options_for(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_shell_policy_root(dir)
}

fn preflight(dir: &Path, path: &Path, configure: impl FnOnce(ComposeOptions) -> ComposeOptions) -> Result<
    darkmatter::markdown::compose::preflight::ComposePreflightReport,
    MarkdownError,
> {
    let options = configure(options_for(dir, path));
    Markdown::try_from(path)
        .unwrap()
        .compose_preflight(&crate::request_support::request(options))
}

fn missing_value(error: &MarkdownError) -> Option<(&str, &str, &ShellCommandOrigin)> {
    match error {
        MarkdownError::ShellExpansion(inner) => match &**inner {
            ShellExpansionError::MissingValue {
                command,
                expression,
                origin,
                ..
            } => Some((command.as_str(), expression.as_str(), origin)),
            _ => None,
        },
        _ => None,
    }
}

fn hole_types(candidate: &ShellCandidate) -> Vec<String> {
    candidate
        .template
        .as_ref()
        .expect("valid template")
        .placeholders()
        .map(|placeholder| placeholder.typing.display_type())
        .collect()
}

// ── AC1: a missing value fails preflight on every Darkmatter surface ──────

#[test]
fn a_null_or_absent_value_fails_preflight_on_every_shell_surface() {
    let cases = [
        (
            "body, null",
            "---\ntarget: null\n---\n::shell ls {{ target }}\n",
            "ls {{ target }}",
            "target",
            ShellCommandOrigin::Body { line: 4 },
        ),
        (
            "body, absent",
            "# Doc\n::shell ls {{ target }}\n",
            "ls {{ target }}",
            "target",
            ShellCommandOrigin::Body { line: 2 },
        ),
        (
            "body, fm namespace",
            "---\ntarget: null\n---\n::shell ls {{ fm.target }}\n",
            "ls {{ fm.target }}",
            "fm.target",
            ShellCommandOrigin::Body { line: 4 },
        ),
        (
            "shell block, null",
            "---\ntarget: null\n---\n::shell-block\nls {{ target }}\n::end-block\n",
            "ls {{ target }}",
            "target",
            ShellCommandOrigin::ShellBlock {
                start_line: 4,
                command_line: 5,
            },
        ),
        (
            "frontmatter, null",
            "---\ntarget: null\nlisting: $(ls {{ target }})\n---\nbody\n",
            "ls {{ target }}",
            "target",
            ShellCommandOrigin::Frontmatter {
                key: "listing".to_string(),
                line: Some(3),
            },
        ),
        (
            "frontmatter, absent",
            "---\nlisting: $(ls {{ target }})\n---\nbody\n",
            "ls {{ target }}",
            "target",
            ShellCommandOrigin::Frontmatter {
                key: "listing".to_string(),
                line: Some(2),
            },
        ),
        (
            "invalid template grammar still refuses a missing value",
            "# Doc\n::shell ls {{ target }} a&b\n",
            "ls {{ target }} a&b",
            "target",
            ShellCommandOrigin::Body { line: 2 },
        ),
    ];
    for (row, content, command, expression, origin) in cases {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "doc.md", content);
        let error = preflight(dir.path(), &path, |options| options).expect_err(row);
        let (got_command, got_expression, got_origin) =
            missing_value(&error).unwrap_or_else(|| panic!("{row}: expected MissingValue, got {error:?}"));
        assert_eq!(got_command, command, "{row}");
        assert_eq!(got_expression, expression, "{row}");
        assert_eq!(got_origin, &origin, "{row}");
        let message = error.to_string();
        assert!(message.contains(command) && message.contains("missing"), "{row}: {message}");
    }
}

#[test]
fn empty_zero_false_and_fallback_values_are_present() {
    for (row, frontmatter, hole) in [
        ("empty string", "target: \"\"", "{{ target }}"),
        ("zero", "target: 0", "{{ target }}"),
        ("false", "target: false", "{{ target }}"),
        ("fallback", "target: null", "{{ target || 'src' }}"),
        ("absent with fallback", "other: 1", "{{ target || 'src' }}"),
    ] {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "doc.md", &format!("---\n{frontmatter}\n---\n::shell ls x{hole}\n"));
        let report = preflight(dir.path(), &path, |options| options).unwrap_or_else(|error| panic!("{row}: {error}"));
        assert_eq!(report.candidates.len(), 1, "{row}");
        assert_eq!(report.entries.len(), 1, "{row}");
    }
}

#[test]
fn a_missing_value_is_unsuppressible_and_a_non_shell_null_still_renders_empty() {
    let dir = TempDir::new().unwrap();
    // `--when-error` handles a failed command, never a missing value.
    let path = write(
        dir.path(),
        "doc.md",
        "---\ntarget: null\n---\nValue: [{{ target }}]\n::shell --when-error 'fallback' ls {{ target }}\n",
    );
    let error = preflight(dir.path(), &path, |options| options).expect_err("missing value");
    assert!(missing_value(&error).is_some(), "{error:?}");
    assert!(error.shell_input_violation().is_some());

    // The strict-mode contract for null elsewhere is unchanged.
    let path = write(dir.path(), "text.md", "---\ntarget: null\n---\nValue: [{{ target }}]\n");
    let (composed, _) = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with(&crate::request_support::request(options_for(dir.path(), &path)))
        .unwrap();
    assert_eq!(composed.content().trim_end(), "Value: []");
}

#[test]
fn compose_fails_with_the_missing_value_before_any_command_runs() {
    let dir = TempDir::new().unwrap();
    let marker = dir.path().join("ran");
    let path = write(
        dir.path(),
        "doc.md",
        "---\ntarget: null\nfirst: $(git init --quiet ran)\n---\n::shell ls {{ target }}\n",
    );
    write(dir.path(), ".darkmatter-shell-whitelist", "prefix git\nprefix ls\n");
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let request = crate::request_support::request(options_for(dir.path(), &path));
    let error = markdown
        .compose_preflight_approvals(&request, None)
        .expect_err("missing value");
    assert!(
        matches!(error, ShellExpansionError::MissingValue { ref expression, .. } if expression == "target"),
        "{error:?}"
    );
    assert!(!marker.exists(), "no command may run before the missing value is reported");
}

// ── Candidates: one per site, typed by the owning document ────────────────

#[test]
fn each_site_keeps_its_authored_template_and_its_owners_types() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "child.md",
        "---\n$schema:\n  dir: string\n---\n::shell just build --out {{ dir }}/build\n",
    );
    let path = write(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  dir: alphanumeric(allow-slashes)\ndir: dist\n---\n\
         ::shell just build --out {{ dir }}/build\n::file ./child.md\n",
    );
    let report = preflight(dir.path(), &path, |options| options).unwrap();

    // Both sites resolve to the same bytes; discovery keeps both.
    assert_eq!(report.entries.len(), 1, "{:?}", report.entries);
    assert_eq!(report.entries[0].normalized, "just build --out dist/build");
    assert_eq!(report.candidates.len(), 2);
    let [root, child] = [&report.candidates[0], &report.candidates[1]];
    assert_eq!(root.authored(), Some("just build --out {{ dir }}/build"));
    assert_eq!(hole_types(root), ["alphanumeric(allow-slashes)"]);
    assert_eq!(hole_types(child), ["string"]);
    assert_ne!(root.shape(), child.shape());
    for candidate in [root, child] {
        assert_eq!(candidate.requirement, ModeRequirement::AnyMode);
        assert_eq!(candidate.resolved.as_deref(), Some(&["just build --out dist/build".to_string()][..]));
        assert!(candidate.offered().exact && candidate.offered().shape && candidate.offered().prefix);
    }
    assert!(root.site.owner.as_deref().is_some_and(|owner| owner.ends_with("doc.md")));
    assert!(child.site.owner.as_deref().is_some_and(|owner| owner.ends_with("child.md")));
    assert_ne!(root.site.source_file, child.site.source_file);
}

#[test]
fn directive_options_are_not_template_arguments_and_holes_stay_whole() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\nname: a b\n---\n::shell --no-cache echo \"from {{ name || 'x y' }}\" --when-error fallback\n",
    );
    let report = preflight(dir.path(), &path, |options| options).unwrap();
    let candidate = &report.candidates[0];
    let shape = candidate.shape().expect("shape");
    assert_eq!(shape.canonical_text(), "echo \"from \"{{ string }}");
    assert_eq!(
        candidate.approval_view().unwrap().display,
        "echo \"from \"{{ name || 'x y': string }}"
    );
}

#[test]
fn a_site_without_placeholders_offers_no_shape() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "::shell git status\n");
    let report = preflight(dir.path(), &path, |options| options).unwrap();
    let candidate = &report.candidates[0];
    assert!(candidate.shape().is_none());
    let offered = candidate.offered();
    assert!(offered.exact && offered.prefix && !offered.shape);
}

// ── Host-declared late roots ──────────────────────────────────────────────

#[test]
fn a_late_root_is_not_evaluated_and_requires_a_template() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\nlatest: \"{{ last(outputs) }}\"\nout: dist\n---\n\
         ::shell just test {{ last(outputs) }} --package {{ out }}\n\
         ::shell just clean {{ latest }}\n\
         ::shell just build --out {{ out }}\n",
    );
    let report = preflight(dir.path(), &path, |options| {
        options.with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]))
    })
    .unwrap();

    let [direct, derived, known] = [&report.candidates[0], &report.candidates[1], &report.candidates[2]];
    for candidate in [direct, derived] {
        assert_eq!(candidate.requirement, ModeRequirement::TemplateRequired);
        assert!(candidate.resolved.is_none());
        let offered = candidate.offered();
        assert!(!offered.exact && offered.shape && offered.prefix);
    }
    let timings: Vec<HoleTiming> = direct
        .template
        .as_ref()
        .unwrap()
        .placeholders()
        .map(|placeholder| placeholder.timing.clone())
        .collect();
    assert_eq!(
        timings,
        [
            HoleTiming::Deferred {
                roots: vec!["outputs".to_string()]
            },
            HoleTiming::ApprovalTime
        ]
    );
    let derived_timing = &derived.template.as_ref().unwrap().placeholders().next().unwrap().timing;
    assert_eq!(
        derived_timing,
        &HoleTiming::Deferred {
            roots: vec!["latest".to_string()]
        }
    );
    assert_eq!(known.requirement, ModeRequirement::AnyMode);

    // Only the resolved-byte site contributes bytes to approve.
    let normalized: Vec<&str> = report.entries.iter().map(|entry| entry.normalized.as_str()).collect();
    assert_eq!(normalized, ["just build --out dist"]);
}

#[test]
fn a_late_read_with_a_missing_approval_time_value_still_fails() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "::shell just test {{ last(outputs) }} --package {{ pkg }}\n");
    let error = preflight(dir.path(), &path, |options| {
        options.with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]))
    })
    .expect_err("pkg is missing");
    assert_eq!(missing_value(&error).map(|(_, expression, _)| expression), Some("pkg"));
}

#[test]
fn a_late_command_that_is_not_a_valid_template_fails_preflight() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "::shell cat {{ last(outputs) }} a&b\n");
    let error = preflight(dir.path(), &path, |options| {
        options.with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]))
    })
    .expect_err("template grammar");
    match &error {
        MarkdownError::ShellExpansion(inner) => assert!(
            matches!(inner.as_ref(), ShellExpansionError::TemplateGrammar { command, .. } if command == "cat {{ last(outputs) }} a&b"),
            "{inner:?}"
        ),
        other => panic!("expected TemplateGrammar, got {other:?}"),
    }
    assert!(error.shell_input_violation().is_some());
}

#[test]
fn without_declared_late_roots_the_same_read_is_an_approval_time_value() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "::shell just test {{ last(outputs) }}\n");
    // `outputs` is an absent document property here: `last` of it is null.
    let error = preflight(dir.path(), &path, |options| options).expect_err("missing value");
    assert_eq!(
        missing_value(&error).map(|(_, expression, _)| expression),
        Some("last(outputs)")
    );
}

// ── Structured pre-approvals and the execution gate ───────────────────────

fn execute(
    dir: &Path,
    path: &Path,
    approvals: ShellPreApprovals,
    deferred: &[&str],
) -> Result<String, MarkdownError> {
    let options = options_for(dir, path)
        .only(&[
            ComposeOperation::FrontmatterInterpolation,
            ComposeOperation::Interpolation,
            ComposeOperation::ShellExpansion,
        ])
        .with_shell_timeout(std::time::Duration::from_secs(60))
        .with_deferred_shell_roots(DeferredShellRoots::new(deferred.iter().copied()))
        .with_pre_approved_commands(approvals);
    Markdown::try_from(path)
        .unwrap()
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed.content().to_string())
}

fn not_pre_approved(error: &MarkdownError) -> bool {
    error.pre_approval_violation().is_some()
}

#[test]
fn exact_and_template_approvals_never_authorize_each_other() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\nword: hello\n---\n::shell git --version {{ word }}\n");
    let report = preflight(dir.path(), &path, |options| options).unwrap();
    let shape = report.candidates[0].shape().unwrap();

    // The shape alone does not admit the resolved bytes; the bytes alone do
    // not admit the shape.
    let mut by_shape = ShellPreApprovals::new();
    by_shape.insert_template(shape.clone());
    assert!(!by_shape.contains_exact("git --version hello"));
    let mut by_bytes = ShellPreApprovals::new();
    by_bytes.insert_exact("git --version hello");
    assert!(!by_bytes.contains_template(&shape));
    assert!(report.candidates[0].authorized_by(&by_bytes));
    assert!(report.candidates[0].authorized_by(&by_shape));

    // A template-required site is never authorized by its bytes.
    let late = write(dir.path(), "late.md", "::shell git --version {{ last(outputs) }}\n");
    let late_report = preflight(dir.path(), &late, |options| {
        options.with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]))
    })
    .unwrap();
    let late_candidate = &late_report.candidates[0];
    let mut every_byte = ShellPreApprovals::new();
    every_byte.insert_exact("git --version");
    every_byte.insert_exact("git --version {{ last(outputs) }}");
    assert!(!late_candidate.authorized_by(&every_byte));
    let error = execute(dir.path(), &late, every_byte, &["outputs"]).expect_err("not pre-approved");
    assert!(not_pre_approved(&error), "{error:?}");
}

#[test]
fn the_gate_accepts_a_shape_approval_and_never_adds_its_bytes() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\nword: hello\n---\n::shell git --version {{ word }}\n");
    let report = preflight(dir.path(), &path, |options| options).unwrap();
    let mut approvals = ShellPreApprovals::new();
    approvals.insert_template(report.candidates[0].shape().unwrap());

    // The gate admits the site by its shape. Execution then needs template
    // mode, which resolved-byte execution never fakes by adding the bytes it
    // resolved: the command is refused, not run.
    let error = execute(dir.path(), &path, approvals.clone(), &[]).expect_err("template mode not run as bytes");
    assert!(not_pre_approved(&error), "{error:?}");
    assert!(!approvals.contains_exact("git --version hello"));

    // A changed declared type is a different shape and is not a member.
    let typed = write(
        dir.path(),
        "typed.md",
        "---\n$schema:\n  word: alphanumeric\nword: hello\n---\n::shell git --version {{ word }}\n",
    );
    let typed_report = preflight(dir.path(), &typed, |options| options).unwrap();
    assert!(!typed_report.candidates[0].authorized_by(&approvals));
}

#[test]
fn exact_pre_approvals_still_run_resolved_bytes() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\nflag: --version\n---\n::shell git {{ flag }}\n");
    let approvals: ShellPreApprovals = ["git --version".to_string()].into_iter().collect();
    let output = execute(dir.path(), &path, approvals, &[]).unwrap();
    assert!(output.contains("git version"), "{output}");
}

// ── The `md` approval lifecycle, per site ─────────────────────────────────

struct Scripted {
    decision: ShellApprovalDecision,
    requests: Mutex<Vec<ShellApprovalRequest>>,
}

impl Scripted {
    fn new(decision: ShellApprovalDecision) -> Arc<Self> {
        Arc::new(Self {
            decision,
            requests: Mutex::new(Vec::new()),
        })
    }
}

impl ShellApprovalHandler for Scripted {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.requests.lock().unwrap().push(request);
        Ok(self.decision.clone())
    }
}

fn approvals_for(
    dir: &Path,
    path: &Path,
    handler: Option<Arc<Scripted>>,
    deferred: &[&str],
) -> Result<darkmatter::markdown::compose::preflight::ComposePreflightApprovals, ShellExpansionError> {
    let options =
        options_for(dir, path).with_deferred_shell_roots(DeferredShellRoots::new(deferred.iter().copied()));
    Markdown::try_from(path).unwrap().compose_preflight_approvals(
        &crate::request_support::request(options),
        handler.map(|handler| handler as Arc<dyn ShellApprovalHandler>),
    )
}

fn policy(dir: &Path) -> ShellPolicyPaths {
    ShellPolicyPaths {
        whitelist: dir.join(".darkmatter-shell-whitelist"),
        blacklist: dir.join(".darkmatter-shell-blacklist"),
    }
}

#[test]
fn a_resolved_site_prompt_offers_exact_and_shape_and_each_choice_persists_its_row() {
    for (decision, persisted) in [
        (ShellApprovalDecision::AllowExactPersist, Some("exact git --version hello")),
        (ShellApprovalDecision::AllowTemplatePersist, Some("template git --version {{ string }}")),
        (ShellApprovalDecision::AllowCommandPersist, Some("prefix git")),
        (ShellApprovalDecision::AllowOnce, None),
    ] {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "doc.md", "---\nword: hello\n---\n::shell git --version {{ word }}\n");
        let handler = Scripted::new(decision.clone());
        let approvals = approvals_for(dir.path(), &path, Some(handler.clone()), &[]).unwrap();

        let requests = handler.requests.lock().unwrap();
        assert_eq!(requests.len(), 1, "{decision:?}");
        let request = &requests[0];
        assert!(request.offered.exact && request.offered.shape && request.offered.prefix);
        assert_eq!(request.normalized_exact, "git --version hello");
        assert_eq!(
            request.template.as_ref().map(|view| view.display.as_str()),
            Some("git --version {{ word: string }}")
        );

        let whitelist = std::fs::read_to_string(dir.path().join(".darkmatter-shell-whitelist")).unwrap_or_default();
        assert_eq!(whitelist.lines().next(), persisted, "{decision:?}");
        let by_template = decision == ShellApprovalDecision::AllowTemplatePersist;
        assert_eq!(approvals.pre_approved_commands.contains_exact("git --version hello"), !by_template);
        assert_eq!(
            approvals
                .pre_approved_commands
                .contains_template(&request.template.as_ref().unwrap().shape),
            by_template
        );
    }
}

#[test]
fn a_template_required_site_is_offered_no_exact_choice() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "::shell git --version {{ last(outputs) }}\n");
    let handler = Scripted::new(ShellApprovalDecision::AllowTemplatePersist);
    let approvals = approvals_for(dir.path(), &path, Some(handler.clone()), &["outputs"]).unwrap();
    let requests = handler.requests.lock().unwrap();
    let request = &requests[0];
    assert!(!request.offered.exact && request.offered.shape && request.offered.prefix);
    assert_eq!(request.normalized_exact, "", "an unknown value is not encoded as empty bytes");
    assert_eq!(
        request.template.as_ref().unwrap().display,
        "git --version {{ last(outputs): string }}"
    );
    assert_eq!(approvals.pre_approved_commands.exact().count(), 0);
    assert_eq!(approvals.pre_approved_commands.templates().count(), 1);

    // An exact decision was not offered, so it authorizes nothing.
    let refused = TempDir::new().unwrap();
    let path = write(refused.path(), "doc.md", "::shell git --version {{ last(outputs) }}\n");
    let error = approvals_for(
        refused.path(),
        &path,
        Some(Scripted::new(ShellApprovalDecision::AllowExactPersist)),
        &["outputs"],
    )
    .expect_err("exact is not offered");
    assert!(matches!(error, ShellExpansionError::Denied { .. }), "{error:?}");
    assert!(!refused.path().join(".darkmatter-shell-whitelist").exists());
}

#[test]
fn rules_are_preferred_exact_then_template_then_prefix() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "---\nword: hello\n---\n::shell git --version {{ word }}\n");
    let shape: TemplateShape = TemplateShape::parse_rule("git --version {{ string }}").unwrap();
    for (rules, exact, template) in [
        ("exact git --version hello\ntemplate git --version {{ string }}\nprefix git\n", true, false),
        ("template git --version {{ string }}\nprefix git\n", false, true),
        ("prefix git\n", true, false),
    ] {
        write(dir.path(), ".darkmatter-shell-whitelist", rules);
        let approvals = approvals_for(dir.path(), &path, None, &[]).unwrap();
        assert_eq!(approvals.pre_approved_commands.contains_exact("git --version hello"), exact, "{rules}");
        assert_eq!(approvals.pre_approved_commands.contains_template(&shape), template, "{rules}");
        assert_eq!(approvals.stats.already_whitelisted, 1, "{rules}");
    }
}

#[test]
fn a_persisted_template_rule_covers_a_late_site_without_prompting() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "::shell git --version {{ last(outputs) }}\n");
    append_whitelist_template(&policy(dir.path()), &TemplateShape::parse_rule("git --version {{ string }}").unwrap())
        .unwrap();
    let approvals = approvals_for(dir.path(), &path, None, &["outputs"]).unwrap();
    assert_eq!(approvals.pre_approved_commands.templates().count(), 1);

    // An exact rule for some fill of it does not.
    let other = TempDir::new().unwrap();
    let path = write(other.path(), "doc.md", "::shell git --version {{ last(outputs) }}\n");
    write(other.path(), ".darkmatter-shell-whitelist", "exact git --version\nexact git --version x\n");
    let error = approvals_for(other.path(), &path, None, &["outputs"]).expect_err("approval required");
    assert!(matches!(error, ShellExpansionError::ApprovalRequired { .. }), "{error:?}");
}

#[test]
fn a_shape_blacklist_refuses_a_late_site_and_blacklisting_persists_the_shape() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "::shell git --version {{ last(outputs) }}\n");
    write(dir.path(), ".darkmatter-shell-blacklist", "template git --version {{ string }}\n");
    write(dir.path(), ".darkmatter-shell-whitelist", "template git --version {{ string }}\n");
    let error = approvals_for(dir.path(), &path, None, &["outputs"]).expect_err("blacklisted");
    assert!(matches!(error, ShellExpansionError::Blacklisted { .. }), "{error:?}");

    let fresh = TempDir::new().unwrap();
    let path = write(fresh.path(), "doc.md", "::shell git --version {{ last(outputs) }}\n");
    let error = approvals_for(
        fresh.path(),
        &path,
        Some(Scripted::new(ShellApprovalDecision::BlacklistPersist)),
        &["outputs"],
    )
    .expect_err("blacklisted");
    assert!(matches!(error, ShellExpansionError::Blacklisted { .. }), "{error:?}");
    let blacklist = load_ruleset(&policy(fresh.path()).blacklist).unwrap();
    assert_eq!(
        blacklist.entries,
        [ShellRuleEntry::Template(TemplateShape::parse_rule("git --version {{ string }}").unwrap())]
    );
}

/// Replaces the whitelist file with a directory when asked to approve, so
/// the policy loaded before the prompt but the append after it fails.
struct BreaksTheWhitelist {
    whitelist: PathBuf,
}

impl ShellApprovalHandler for BreaksTheWhitelist {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        let _ = std::fs::remove_file(&self.whitelist);
        std::fs::create_dir(&self.whitelist).unwrap();
        Ok(ShellApprovalDecision::AllowTemplatePersist)
    }
}

#[test]
fn a_failed_template_write_is_a_policy_failure_and_approves_nothing() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "doc.md", "::shell git --version {{ last(outputs) }}\n");
    write(dir.path(), ".darkmatter-shell-whitelist", "prefix ls\n");
    let options = options_for(dir.path(), &path).with_deferred_shell_roots(DeferredShellRoots::new(["outputs"]));
    let handler = Arc::new(BreaksTheWhitelist {
        whitelist: policy(dir.path()).whitelist,
    });
    let error = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_preflight_approvals(&crate::request_support::request(options), Some(handler))
        .expect_err("policy write fails");
    assert!(matches!(error, ShellExpansionError::PolicyIo { .. }), "{error:?}");
}

#[test]
fn a_hole_type_follows_the_owner_schema_into_the_persisted_rule() {
    let dir = TempDir::new().unwrap();
    let path = write(
        dir.path(),
        "doc.md",
        "---\n$schema:\n  pkg: alphanumeric(allow-dashes)\n  count: number\npkg: core\ncount: 2\n---\n\
         ::shell just test {{ last(outputs) }} --package {{ pkg }} --jobs {{ count }}\n",
    );
    let handler = Scripted::new(ShellApprovalDecision::AllowTemplatePersist);
    approvals_for(dir.path(), &path, Some(handler.clone()), &["outputs"]).unwrap();
    let request = &handler.requests.lock().unwrap()[0];
    assert_eq!(
        request.template.as_ref().unwrap().display,
        "just test {{ last(outputs): string }} --package {{ pkg: alphanumeric(allow-dashes) }} \
         --jobs {{ count: string (validated as number) }}"
    );
    let whitelist = std::fs::read_to_string(dir.path().join(".darkmatter-shell-whitelist")).unwrap();
    assert_eq!(
        whitelist.trim_end(),
        "template just test {{ string }} --package {{ alphanumeric(allow-dashes) }} --jobs {{ string }}"
    );
    let shape = &request.template.as_ref().unwrap().shape;
    let holes: Vec<&ApprovalType> = shape
        .segments
        .iter()
        .flat_map(|segment| &segment.args)
        .flat_map(|arg| &arg.parts)
        .filter_map(|part| match part {
            darkmatter::markdown::compose::shell_expansion::template::ShapePart::Hole(ty) => Some(ty),
            _ => None,
        })
        .collect();
    assert_eq!(holes.len(), 3);
}
