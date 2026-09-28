//! Inserted text is data, never an instruction (R1, R1.2, R1.4).
//!
//! Every authored span is scanned once; text an expression, a file read, a
//! shell command, a literal escape, or a data override produced is never
//! scanned again: not for `{{ … }}`, not for `{{{ … }}}`, not for a whole-value
//! `$( … )`, and not for a body directive. Each test composes a real document
//! through the public pipeline and records every shell approval it asks for.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOptions, OverrideLayer, ShellExpansionError, collect_shell_commands,
};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// The four payload shapes the spec names, each a different instruction form.
const PAYLOADS: [&str; 4] = ["{{ area }}", "{{…}}", "{{{ area }}}", "$(echo X)"];

/// Records every approval request and answers with a fixed decision.
struct Recorder {
    decision: ShellApprovalDecision,
    commands: Mutex<Vec<String>>,
}

impl Recorder {
    fn allowing() -> Arc<Self> {
        Arc::new(Self { decision: ShellApprovalDecision::AllowOnce, commands: Mutex::default() })
    }

    fn denying() -> Arc<Self> {
        Arc::new(Self { decision: ShellApprovalDecision::Deny, commands: Mutex::default() })
    }

    fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }
}

impl ShellApprovalHandler for Recorder {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.commands.lock().unwrap().push(request.normalized_exact);
        Ok(self.decision.clone())
    }
}

/// Compose options for a document in `dir`, with no repository capture and
/// `approvals` answering every shell request.
fn options(dir: &Path, file: &str, approvals: Arc<Recorder>) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(dir.join(file))
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_approval_handler(approvals)
}

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

fn compose(dir: &Path, file: &str, options: ComposeOptions) -> Result<Markdown, String> {
    let markdown = Markdown::try_from(dir.join(file).as_path()).unwrap();
    markdown
        .compose_with(options)
        .map(|(composed, _)| composed)
        .map_err(|error| error.to_string())
}

fn frontmatter_string(markdown: &Markdown, key: &str) -> Value {
    markdown.frontmatter().as_map().get(key).cloned().unwrap_or(Value::Null)
}

// ── R1: a data override is inert in frontmatter, body, and transclusion ──

/// Each payload, injected with data origin, composes to its exact text in
/// frontmatter, in the body, and in a transcluded child, beside an authored
/// span that still evaluates. No payload is evaluated, fails to parse, or asks
/// for shell approval.
#[test]
fn data_overrides_stay_exact_in_frontmatter_body_and_transclusion() {
    for payload in PAYLOADS {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "child.md", "Child: {{ note }}\n");
        write(
            dir.path(),
            "doc.md",
            "---\narea: claudine\ncopy: \"pre {{ note }}\"\n---\nBody: {{ area }} {{ note }}\n\n::file ./child.md\n",
        );
        let approvals = Recorder::denying();
        let options = options(dir.path(), "doc.md", approvals.clone())
            .with_data_overrides(json!({ "note": payload }));

        let composed = compose(dir.path(), "doc.md", options)
            .unwrap_or_else(|error| panic!("{payload}: {error}"));

        assert_eq!(frontmatter_string(&composed, "note"), json!(payload), "{payload}");
        assert_eq!(frontmatter_string(&composed, "copy"), json!(format!("pre {payload}")), "{payload}");
        let body = composed.content();
        assert!(body.contains(&format!("Body: claudine {payload}")), "{payload}: {body}");
        assert!(body.contains(&format!("Child: {payload}")), "{payload}: {body}");
        assert!(approvals.commands().is_empty(), "{payload}: {:?}", approvals.commands());
    }
}

/// Control row: the same payloads as authored overrides are templates, so the
/// data edits above are what made them inert (ruling N1).
#[test]
fn authored_overrides_of_the_same_payloads_stay_templates() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "doc.md", "---\narea: claudine\n---\nBody: {{ note }}\n");

    let fills = options(dir.path(), "doc.md", Recorder::denying())
        .with_set_overrides(json!({ "note": "{{ area }}" }));
    assert!(compose(dir.path(), "doc.md", fills).unwrap().content().contains("Body: claudine"));

    let literal = options(dir.path(), "doc.md", Recorder::denying())
        .with_set_overrides(json!({ "note": "{{{ area }}}" }));
    assert!(compose(dir.path(), "doc.md", literal).unwrap().content().contains("Body: {{ area }}"));

    let malformed = options(dir.path(), "doc.md", Recorder::denying())
        .with_set_overrides(json!({ "note": "{{…}}" }));
    assert!(compose(dir.path(), "doc.md", malformed).is_err());

    let approvals = Recorder::denying();
    let shell = options(dir.path(), "doc.md", approvals.clone())
        .with_set_overrides(json!({ "note": "$(echo X)" }));
    assert!(compose(dir.path(), "doc.md", shell).is_err());
    assert_eq!(approvals.commands(), vec!["echo X"]);
}

/// A key takes the origin of the layer that supplied it, whatever the order.
#[test]
fn override_layers_take_the_origin_of_the_winning_layer() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "doc.md", "---\narea: claudine\n---\nX={{ x }} Y={{ y }}\n");
    let template = json!({ "x": "{{ area }}", "y": "{{ area }}" });

    let data_wins = options(dir.path(), "doc.md", Recorder::denying()).with_override_layers([
        OverrideLayer::authored(template.clone()),
        OverrideLayer::data(json!({ "y": "{{ area }}" })),
    ]);
    assert!(compose(dir.path(), "doc.md", data_wins).unwrap().content().contains("X=claudine Y={{ area }}"));

    let authored_wins = options(dir.path(), "doc.md", Recorder::denying()).with_override_layers([
        OverrideLayer::data(json!({ "y": "{{ area }}" })),
        OverrideLayer::authored(template),
    ]);
    assert!(compose(dir.path(), "doc.md", authored_wins).unwrap().content().contains("X=claudine Y=claudine"));
}

// ── Single pass: results are data ───────────────────────────────────────

/// The spec reproduction: an escape's output is not re-read by the body.
#[test]
fn escaped_frontmatter_value_stays_literal_in_the_body() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "esc.md", "---\narea: claudine\nnote: \"fixed {{{ area }}}\"\n---\nBody: {{ note }}\n");
    let composed = compose(dir.path(), "esc.md", options(dir.path(), "esc.md", Recorder::denying())).unwrap();
    assert_eq!(composed.content().trim(), "Body: fixed {{ area }}");
}

/// Frontmatter pass 2 scans only the keys it deferred, from their authored
/// text: shell output and pass-1 results that spell `{{ … }}` stay data.
#[test]
fn frontmatter_pass_two_scans_only_deferred_authored_keys() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\nflag: true\nc: C\nt: \"{{ flag ? 'x {{ c }}' : '' }}\"\ns: \"$(printf '%s' 'x {{{ c }}}')\"\nd: \"pre {{ s }}\"\n---\nbody\n",
    );
    let approvals = Recorder::allowing();
    let composed = compose(dir.path(), "doc.md", options(dir.path(), "doc.md", approvals.clone())).unwrap();

    assert_eq!(frontmatter_string(&composed, "t"), json!("x {{ c }}"));
    assert_eq!(frontmatter_string(&composed, "s"), json!("x {{ c }}"));
    assert_eq!(frontmatter_string(&composed, "d"), json!("pre x {{ c }}"));
    assert_eq!(approvals.commands().len(), 1, "{:?}", approvals.commands());
}

/// A file read is data in the reader's scope (spike S1 E7, E8): its `{{ … }}`
/// is not evaluated and its `{{{ … }}}` is not converted.
#[test]
fn a_file_read_is_not_rescanned_or_converted() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "other.md", "---\nm: \"see {{ b }} here\"\nk: \"a {{{ b }}} c\"\n---\n");
    write(
        dir.path(),
        "doc.md",
        "---\nb: SECRET\n---\nM={{ frontmatter('./other.md', 'm') }}\nK={{ frontmatter('./other.md', 'k') }}\n",
    );
    let composed = compose(dir.path(), "doc.md", options(dir.path(), "doc.md", Recorder::denying())).unwrap();
    assert!(composed.content().contains("M=see {{ b }} here"), "{}", composed.content());
    assert!(composed.content().contains("K=a {{{ b }}} c"), "{}", composed.content());
}

// ── R1.2: the command shape comes from authored source ──────────────────

/// An authored whole-value `$( … )` is found by preflight and handled by the
/// approval policy; the same text as a data override is neither.
#[test]
fn only_an_authored_frontmatter_command_is_collected_and_approved() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "doc.md", "---\ns: \"$(echo hi)\"\n---\n{{ s }}\n");
    let authored = options(dir.path(), "doc.md", Recorder::denying());
    let markdown = Markdown::try_from(dir.path().join("doc.md").as_path()).unwrap();
    let entries = collect_shell_commands(&markdown, &authored).unwrap();
    assert_eq!(entries.iter().map(|e| e.normalized.as_str()).collect::<Vec<_>>(), ["echo hi"]);
    let approvals = Recorder::denying();
    assert!(compose(dir.path(), "doc.md", options(dir.path(), "doc.md", approvals.clone())).is_err());
    assert_eq!(approvals.commands(), vec!["echo hi"]);

    write(dir.path(), "data.md", "---\ntitle: t\n---\n{{ s }}\n");
    let data = options(dir.path(), "data.md", Recorder::denying())
        .with_data_overrides(json!({ "s": "$(echo hi)" }));
    let markdown = Markdown::try_from(dir.path().join("data.md").as_path()).unwrap();
    assert!(collect_shell_commands(&markdown, &data).unwrap().is_empty());
}

/// Interpolation that produces a whole-value `$( … )` produces text, and
/// shell output that looks like a command is kept as data rather than
/// rejected as a leak or executed.
#[test]
fn produced_command_shapes_are_data() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\ncmd: \"{{ '$(' + 'echo X)' }}\"\nout: \"$(printf '%s' '$(echo Y)')\"\n---\nC={{ cmd }} O={{ out }}\n",
    );
    let approvals = Recorder::allowing();
    let composed = compose(dir.path(), "doc.md", options(dir.path(), "doc.md", approvals.clone())).unwrap();
    assert_eq!(frontmatter_string(&composed, "cmd"), json!("$(echo X)"));
    assert_eq!(frontmatter_string(&composed, "out"), json!("$(echo Y)"));
    assert!(composed.content().contains("C=$(echo X) O=$(echo Y)"));
    assert_eq!(approvals.commands(), vec!["printf %s \"$(echo Y)\""]);
}

/// A body directive whose executable an expression supplies is rejected
/// (spike S1 E5c); an argument an expression supplies still runs.
#[test]
fn a_body_executable_from_interpolation_is_rejected() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "exe.md", "---\ny: echo\n---\n::shell {{ y }} EXEC\n");
    let approvals = Recorder::allowing();
    let error = compose(dir.path(), "exe.md", options(dir.path(), "exe.md", approvals.clone())).unwrap_err();
    assert!(error.contains("supplies the executable `echo`"), "{error}");
    assert!(approvals.commands().is_empty());

    write(dir.path(), "arg.md", "---\ny: ARG\n---\n::shell echo {{ y }}\n");
    let approvals = Recorder::allowing();
    let composed = compose(dir.path(), "arg.md", options(dir.path(), "arg.md", approvals.clone())).unwrap();
    assert!(composed.content().contains("ARG"));
    assert_eq!(approvals.commands(), vec!["echo ARG"]);
}

/// A data operator cannot extend an authored command's pipeline.
#[test]
fn a_body_pipeline_operator_from_interpolation_is_rejected() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "op.md", "---\ny: \"a && rm -rf x\"\n---\n::shell echo {{ y }}\n");
    let approvals = Recorder::allowing();
    let error = compose(dir.path(), "op.md", options(dir.path(), "op.md", approvals.clone())).unwrap_err();
    assert!(error.contains("changes the command from 1 action(s) to 2"), "{error}");
    assert!(approvals.commands().is_empty());
}

/// Data in a `::shell-block` cannot add a command or close the block
/// (spike S1 E5, E5b).
#[test]
fn data_cannot_split_or_close_a_shell_block() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "split.md",
        "---\nx: \"hi\\necho SECOND\"\n---\n::shell-block\necho {{ x }}\n::end-block\n",
    );
    let approvals = Recorder::allowing();
    let error = compose(dir.path(), "split.md", options(dir.path(), "split.md", approvals.clone())).unwrap_err();
    assert!(error.contains("splits or joins the block's commands"), "{error}");
    assert!(approvals.commands().is_empty());

    write(
        dir.path(),
        "close.md",
        "---\nx: \"\\n::end-block\\nafter\"\n---\n::shell-block\necho {{ x }}\n::end-block\n",
    );
    let approvals = Recorder::allowing();
    assert!(compose(dir.path(), "close.md", options(dir.path(), "close.md", approvals.clone())).is_err());
    assert!(approvals.commands().is_empty());
}

/// Directives inside data are text (spike S1 E1–E4): none runs, transcludes,
/// or asks for approval, and the authored equivalent still runs.
#[test]
fn directives_inside_data_are_text() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "secret.md", "SECRET\n");
    write(
        dir.path(),
        "doc.md",
        "---\nshell: \"::shell echo INJECTED\"\nfile: \"::file ./secret.md\"\ncode: \"::code ./secret.md\"\nblock: \"::shell-block\\necho BLOCK\\n::end-block\"\n---\n{{ shell }}\n\n{{ file }}\n\n{{ code }}\n\n{{ block }}\n\n::shell echo AUTHORED\n",
    );
    let approvals = Recorder::allowing();
    let composed = compose(dir.path(), "doc.md", options(dir.path(), "doc.md", approvals.clone())).unwrap();
    let body = composed.content();
    for literal in ["::shell echo INJECTED", "::file ./secret.md", "::code ./secret.md", "echo BLOCK"] {
        assert!(body.contains(literal), "{literal} stays text: {body}");
    }
    assert!(!body.contains("SECRET\n"), "{body}");
    assert!(body.contains("AUTHORED"), "{body}");
    assert_eq!(approvals.commands(), vec!["echo AUTHORED"]);
}

/// A code fence in data neither hides an authored directive nor protects a
/// data directive (spike S1 E6).
#[test]
fn a_data_fence_does_not_hide_an_authored_directive() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "doc.md", "---\nx: \"```\"\n---\n{{ x }}\n\n::shell echo AUTHORED\n");
    let approvals = Recorder::allowing();
    let composed = compose(dir.path(), "doc.md", options(dir.path(), "doc.md", approvals.clone())).unwrap();
    assert!(composed.content().contains("AUTHORED"));
    assert_eq!(approvals.commands(), vec!["echo AUTHORED"]);
}

/// An authored `replace:` value still writes authored text, so a replacement
/// can still expand to a directive (the macro use), while an interpolated
/// replacement value is data.
#[test]
fn replacement_values_keep_the_origin_of_their_leaf() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\ncmd: \"echo DATA\"\nreplace:\n  AUTHORED: \"::shell echo MACRO\"\n  PRODUCED: \"::shell {{ cmd }}\"\n---\nAUTHORED\n\nPRODUCED\n",
    );
    let approvals = Recorder::allowing();
    let composed = compose(dir.path(), "doc.md", options(dir.path(), "doc.md", approvals.clone())).unwrap();
    assert!(composed.content().contains("MACRO"), "{}", composed.content());
    assert!(composed.content().contains("::shell echo DATA"), "{}", composed.content());
    assert_eq!(approvals.commands(), vec!["echo MACRO"]);
}

// ── Transclusion: the parent's composed values are data in the child ─────

/// A parent value holding shell-looking text is not a command in the child,
/// and a data `set.NAME=` value is not rescanned there (I2 B4, B5).
#[test]
fn a_child_receives_parent_values_as_data() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "child.md", "---\nactor: default\n---\nNote={{ note }} Actor={{ actor }}\n");
    write(
        dir.path(),
        "parent.md",
        "---\nnote: \"{{ '$(' + 'echo X)' }}\"\nx: \"{{ '{{' + ' area }}' }}\"\n---\n::file ./child.md set.actor=\"{{ x }}\"\n",
    );
    let approvals = Recorder::allowing();
    let composed = compose(dir.path(), "parent.md", options(dir.path(), "parent.md", approvals.clone())).unwrap();
    assert!(composed.content().contains("Note=$(echo X) Actor={{ area }}"), "{}", composed.content());
    assert!(approvals.commands().is_empty());
}
